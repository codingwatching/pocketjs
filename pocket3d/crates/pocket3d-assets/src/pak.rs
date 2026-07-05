//! The `.p3dpak` archive format (DESIGN.md §9 "Asset Pipeline").
//!
//! A `.p3dpak` is a *simple, hash-validated* container: it concatenates named
//! asset blobs into one file with a trailing table-of-contents, and carries a
//! CRC-32 per entry so a shipped pak can be integrity-checked cheaply. Per the
//! design, **hash-based cache invalidation matters more than compression**, so
//! blobs are stored verbatim (no compression).
//!
//! ## On-disk layout
//!
//! ```text
//! ┌────────────────────────────────────────────┐
//! │ magic  b"P3DPAK\0"            (7 bytes)      │
//! │ version  u32 LE                              │
//! ├────────────────────────────────────────────┤
//! │ blob region: every asset's raw bytes,        │
//! │              back to back                    │
//! ├────────────────────────────────────────────┤
//! │ TOC @ toc_offset:                            │
//! │   entry_count  u32 LE                        │
//! │   for each entry:                            │
//! │     name_len   u32 LE                        │
//! │     name       utf-8 bytes                   │
//! │     kind        u8                           │
//! │     offset     u64 LE  (absolute in file)    │
//! │     length     u64 LE                        │
//! │     crc32       u32 LE                        │
//! ├────────────────────────────────────────────┤
//! │ footer: toc_offset  u64 LE   (last 8 bytes)  │
//! └────────────────────────────────────────────┘
//! ```
//!
//! Readers locate the TOC by reading the final 8 bytes (the footer), so the
//! blob region can be any size and the header stays fixed.

use anyhow::{bail, ensure, Context, Result};
use std::path::Path;

/// Archive magic: the first 7 bytes of every `.p3dpak`.
pub const MAGIC: &[u8] = b"P3DPAK\0";
/// Current archive format version.
pub const VERSION: u32 = 1;

/// The category of a packed asset, which maps 1:1 to a canonical file
/// extension (DESIGN.md §9).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AssetKind {
    /// A compiled BSP world (`.p3dworld`).
    World,
    /// A static/skinned mesh payload (`.p3dmesh`).
    Mesh,
    /// A skeleton + skin binding (`.p3dskin`).
    Skin,
    /// An animation clip (`.p3danim`).
    Anim,
    /// A material description (`.p3dmat`).
    Material,
    /// A texture payload (`.p3dtex`).
    Texture,
    /// Opaque bytes with no engine-defined meaning (`.p3draw`).
    Raw,
}

impl AssetKind {
    /// The canonical file extension for this kind (DESIGN.md §9).
    pub fn extension(self) -> &'static str {
        match self {
            AssetKind::World => ".p3dworld",
            AssetKind::Mesh => ".p3dmesh",
            AssetKind::Skin => ".p3dskin",
            AssetKind::Anim => ".p3danim",
            AssetKind::Material => ".p3dmat",
            AssetKind::Texture => ".p3dtex",
            AssetKind::Raw => ".p3draw",
        }
    }

    /// Stable wire tag stored in the TOC.
    fn to_u8(self) -> u8 {
        match self {
            AssetKind::World => 0,
            AssetKind::Mesh => 1,
            AssetKind::Skin => 2,
            AssetKind::Anim => 3,
            AssetKind::Material => 4,
            AssetKind::Texture => 5,
            AssetKind::Raw => 6,
        }
    }

    fn from_u8(tag: u8) -> Result<Self> {
        Ok(match tag {
            0 => AssetKind::World,
            1 => AssetKind::Mesh,
            2 => AssetKind::Skin,
            3 => AssetKind::Anim,
            4 => AssetKind::Material,
            5 => AssetKind::Texture,
            6 => AssetKind::Raw,
            other => bail!("unknown p3dpak AssetKind tag {other}"),
        })
    }
}

/// One entry in a pak's table of contents (DESIGN.md §9).
#[derive(Clone, Debug)]
pub struct PakEntry {
    /// Logical asset name (a lookup key; need not be a filesystem path).
    pub name: String,
    /// What kind of asset the blob holds.
    pub kind: AssetKind,
    /// Absolute byte offset of the blob within the archive file.
    pub offset: u64,
    /// Blob length in bytes.
    pub length: u64,
    /// CRC-32 (IEEE) of the blob bytes, for integrity validation.
    pub crc32: u32,
}

/// Compute the IEEE CRC-32 of `data` with a tiny, dependency-free bitwise
/// implementation (polynomial `0xEDB88320`). Used for per-entry hash
/// validation (DESIGN.md §9: "hash-based cache invalidation").
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            // Branchless: `mask` is all-ones when the low bit is set.
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

// ---------------------------------------------------------------------------
// Writer
// ---------------------------------------------------------------------------

/// Accumulates named blobs and serializes them into a `.p3dpak` (DESIGN.md §9).
#[derive(Default)]
pub struct PakWriter {
    blobs: Vec<(String, AssetKind, Vec<u8>)>,
}

impl PakWriter {
    /// Create an empty writer.
    pub fn new() -> Self {
        Self { blobs: Vec::new() }
    }

    /// Stage a named blob of the given `kind`. The bytes are copied.
    pub fn add(&mut self, name: &str, kind: AssetKind, bytes: &[u8]) {
        self.blobs.push((name.to_string(), kind, bytes.to_vec()));
    }

    /// Serialize every staged blob into a fresh `.p3dpak` byte vector.
    pub fn write_to_vec(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());

        // Blob region: record each entry's absolute offset as we append.
        let mut entries: Vec<PakEntry> = Vec::with_capacity(self.blobs.len());
        for (name, kind, bytes) in &self.blobs {
            let offset = out.len() as u64;
            out.extend_from_slice(bytes);
            entries.push(PakEntry {
                name: name.clone(),
                kind: *kind,
                offset,
                length: bytes.len() as u64,
                crc32: crc32(bytes),
            });
        }

        // TOC.
        let toc_offset = out.len() as u64;
        out.extend_from_slice(&(entries.len() as u32).to_le_bytes());
        for e in &entries {
            let name_bytes = e.name.as_bytes();
            out.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
            out.extend_from_slice(name_bytes);
            out.push(e.kind.to_u8());
            out.extend_from_slice(&e.offset.to_le_bytes());
            out.extend_from_slice(&e.length.to_le_bytes());
            out.extend_from_slice(&e.crc32.to_le_bytes());
        }

        // Footer: the TOC offset, so the reader can find the TOC from the tail.
        out.extend_from_slice(&toc_offset.to_le_bytes());
        out
    }

    /// Serialize and write the archive to `path`.
    pub fn write_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let bytes = self.write_to_vec();
        std::fs::write(path.as_ref(), &bytes)
            .with_context(|| format!("writing p3dpak to {}", path.as_ref().display()))?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Reader
// ---------------------------------------------------------------------------

/// A parsed, in-memory `.p3dpak`. Owns the archive bytes so [`Pak::get`] can
/// hand back borrowed, CRC-verified slices (DESIGN.md §9).
pub struct Pak {
    bytes: Vec<u8>,
    entries: Vec<PakEntry>,
}

/// Little-endian scalar readers over an owned buffer.
fn read_u32(bytes: &[u8], at: usize) -> Result<u32> {
    let end = at + 4;
    ensure!(end <= bytes.len(), "p3dpak: truncated u32 at {at}");
    Ok(u32::from_le_bytes(bytes[at..end].try_into().unwrap()))
}

fn read_u64(bytes: &[u8], at: usize) -> Result<u64> {
    let end = at + 8;
    ensure!(end <= bytes.len(), "p3dpak: truncated u64 at {at}");
    Ok(u64::from_le_bytes(bytes[at..end].try_into().unwrap()))
}

impl Pak {
    /// Open and parse a `.p3dpak` from disk.
    pub fn open(path: impl AsRef<Path>) -> Result<Pak> {
        let bytes = std::fs::read(path.as_ref())
            .with_context(|| format!("reading p3dpak {}", path.as_ref().display()))?;
        Pak::parse(bytes)
    }

    /// Parse a `.p3dpak` from an owned or borrowed byte buffer.
    pub fn parse(bytes: impl Into<Vec<u8>>) -> Result<Pak> {
        let bytes = bytes.into();
        ensure!(
            bytes.len() >= MAGIC.len() + 4 + 8,
            "p3dpak: buffer too small ({} bytes)",
            bytes.len()
        );
        ensure!(&bytes[..MAGIC.len()] == MAGIC, "p3dpak: bad magic");
        let version = read_u32(&bytes, MAGIC.len())?;
        ensure!(version == VERSION, "p3dpak: unsupported version {version}");

        // Footer (last 8 bytes) points at the TOC.
        let toc_offset = read_u64(&bytes, bytes.len() - 8)? as usize;
        ensure!(toc_offset <= bytes.len(), "p3dpak: TOC offset out of range");

        let mut cursor = toc_offset;
        let count = read_u32(&bytes, cursor)? as usize;
        cursor += 4;

        let mut entries = Vec::with_capacity(count);
        for _ in 0..count {
            let name_len = read_u32(&bytes, cursor)? as usize;
            cursor += 4;
            let name_end = cursor + name_len;
            ensure!(name_end <= bytes.len(), "p3dpak: truncated entry name");
            let name = std::str::from_utf8(&bytes[cursor..name_end])
                .context("p3dpak: entry name is not utf-8")?
                .to_string();
            cursor = name_end;

            ensure!(cursor < bytes.len(), "p3dpak: truncated entry kind");
            let kind = AssetKind::from_u8(bytes[cursor])?;
            cursor += 1;

            let offset = read_u64(&bytes, cursor)?;
            cursor += 8;
            let length = read_u64(&bytes, cursor)?;
            cursor += 8;
            let crc = read_u32(&bytes, cursor)?;
            cursor += 4;

            // Validate the blob range now so `get`/`entries` are safe later.
            let end = offset
                .checked_add(length)
                .context("p3dpak: entry length overflow")?;
            ensure!(
                end <= bytes.len() as u64,
                "p3dpak: entry '{name}' blob range out of bounds"
            );

            entries.push(PakEntry {
                name,
                kind,
                offset,
                length,
                crc32: crc,
            });
        }

        Ok(Pak { bytes, entries })
    }

    /// The table of contents.
    pub fn entries(&self) -> &[PakEntry] {
        &self.entries
    }

    /// Whether an entry with `name` exists (ignores CRC).
    pub fn contains(&self, name: &str) -> bool {
        self.entries.iter().any(|e| e.name == name)
    }

    fn entry(&self, name: &str) -> Option<&PakEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    fn blob(&self, e: &PakEntry) -> &[u8] {
        let start = e.offset as usize;
        let end = start + e.length as usize;
        &self.bytes[start..end]
    }

    /// Fetch a blob by name, **verifying its CRC-32** first. Returns `None` if
    /// the entry is missing *or* the stored bytes fail integrity validation
    /// (DESIGN.md §9). Use [`Pak::get_checked`] to distinguish the two cases.
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        let e = self.entry(name)?;
        let blob = self.blob(e);
        if crc32(blob) == e.crc32 {
            Some(blob)
        } else {
            None
        }
    }

    /// Like [`Pak::get`] but reports *why* a lookup failed: `Ok(None)` for a
    /// missing entry, `Err` for a CRC mismatch (corruption/tamper).
    pub fn get_checked(&self, name: &str) -> Result<Option<&[u8]>> {
        let Some(e) = self.entry(name) else {
            return Ok(None);
        };
        let blob = self.blob(e);
        let actual = crc32(blob);
        ensure!(
            actual == e.crc32,
            "p3dpak: CRC mismatch for '{name}' (stored {:#010x}, got {:#010x})",
            e.crc32,
            actual
        );
        Ok(Some(blob))
    }

    /// Verify every entry's CRC-32, returning the name of the first corrupt
    /// entry, or `Ok(())` if all pass.
    pub fn verify(&self) -> Result<()> {
        for e in &self.entries {
            let actual = crc32(self.blob(e));
            ensure!(
                actual == e.crc32,
                "p3dpak: CRC mismatch for '{}' (stored {:#010x}, got {:#010x})",
                e.name,
                e.crc32,
                actual
            );
        }
        Ok(())
    }
}
