//! WAD3 texture archive loading and 8-bit paletted miptex → RGBA decoding.
//!
//! GoldSrc textures are `miptex`: a 16-byte name, width/height, four mip
//! offsets, the four mip levels (8-bit palette indices), then a 256-entry RGB
//! palette. Textures whose name starts with `{` are color-keyed: palette index
//! 255 is transparent. Loading is lazy — `halflife.wad` is ~37 MB, so we keep
//! raw miptex lumps and decode only the textures a map actually references.

use crate::reader::Reader;
use anyhow::{bail, Context, Result};
use pocket3d_core::texture::TextureData;
use std::collections::HashMap;
use std::path::Path;

const WAD3_MAGIC: &[u8; 4] = b"WAD3";
/// Directory entry type for a miptex lump.
const TYP_MIPTEX: u8 = 0x43;

/// A loaded WAD3 archive: uppercased texture name -> raw miptex lump bytes.
#[derive(Default)]
pub struct Wad {
    textures: HashMap<String, Vec<u8>>,
}

impl Wad {
    pub fn is_empty(&self) -> bool {
        self.textures.is_empty()
    }

    pub fn len(&self) -> usize {
        self.textures.len()
    }

    /// Load and index a single `.wad` file.
    pub fn load_file(path: impl AsRef<Path>) -> Result<Wad> {
        let path = path.as_ref();
        let data =
            std::fs::read(path).with_context(|| format!("reading WAD {}", path.display()))?;
        Wad::parse(&data).with_context(|| format!("parsing WAD {}", path.display()))
    }

    pub fn parse(data: &[u8]) -> Result<Wad> {
        let mut r = Reader::new(data);
        let magic = r.bytes(4)?;
        if magic != WAD3_MAGIC {
            bail!(
                "not a WAD3 archive (magic {:?})",
                String::from_utf8_lossy(magic)
            );
        }
        let num_lumps = r.i32()? as usize;
        let dir_offset = r.u32()? as usize;

        let mut textures = HashMap::new();
        let mut dir = Reader::at(data, dir_offset);
        for _ in 0..num_lumps {
            let file_pos = dir.u32()? as usize;
            let _disk_size = dir.i32()?;
            let size = dir.i32()? as usize;
            let typ = dir.u8()?;
            let _compression = dir.u8()?;
            let _pad = dir.u16()?;
            let name = dir.fixed_str(16)?;
            if typ != TYP_MIPTEX {
                continue;
            }
            if file_pos + size > data.len() {
                continue; // skip malformed entry rather than fail the whole WAD
            }
            textures.insert(name.to_ascii_uppercase(), data[file_pos..file_pos + size].to_vec());
        }
        Ok(Wad { textures })
    }

    /// Merge another WAD's entries into this one (later WADs do not overwrite
    /// earlier ones, matching the map's `wad` search order).
    pub fn merge(&mut self, other: Wad) {
        for (k, v) in other.textures {
            self.textures.entry(k).or_insert(v);
        }
    }

    /// Look up raw miptex bytes by (case-insensitive) name.
    pub fn get(&self, name: &str) -> Option<&[u8]> {
        self.textures.get(&name.to_ascii_uppercase()).map(|v| v.as_slice())
    }

    /// Decode a named texture from this WAD to RGBA.
    pub fn decode(&self, name: &str) -> Option<TextureData> {
        let bytes = self.get(name)?;
        decode_miptex(bytes, name).ok()
    }
}

/// A parsed miptex header (also used for embedded BSP textures).
#[derive(Clone, Debug)]
pub struct MipTexHeader {
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// Byte offsets (relative to the miptex start) of each mip level, or 0 if
    /// the texture data lives in an external WAD.
    pub offsets: [u32; 4],
}

impl MipTexHeader {
    pub fn parse(bytes: &[u8]) -> Result<MipTexHeader> {
        let mut r = Reader::new(bytes);
        let name = r.fixed_str(16)?;
        let width = r.u32()?;
        let height = r.u32()?;
        let offsets = [r.u32()?, r.u32()?, r.u32()?, r.u32()?];
        Ok(MipTexHeader {
            name,
            width,
            height,
            offsets,
        })
    }

    /// Whether pixel data is embedded here (true) or lives in a WAD (false).
    pub fn has_embedded_pixels(&self) -> bool {
        self.offsets[0] != 0
    }
}

/// Decode a full miptex lump (header + mip0 + palette) into an RGBA texture.
/// `lookup_name` is the name used to decide color-keying (`{`-prefixed).
pub fn decode_miptex(bytes: &[u8], lookup_name: &str) -> Result<TextureData> {
    let hdr = MipTexHeader::parse(bytes)?;
    let (w, h) = (hdr.width, hdr.height);
    if w == 0 || h == 0 || w > 4096 || h > 4096 {
        bail!("miptex {} has invalid dimensions {}x{}", hdr.name, w, h);
    }
    if !hdr.has_embedded_pixels() {
        bail!("miptex {} has no embedded pixels", hdr.name);
    }

    let mip0_ofs = hdr.offsets[0] as usize;
    let n = (w * h) as usize;
    if mip0_ofs + n > bytes.len() {
        bail!("miptex {} mip0 out of range", hdr.name);
    }
    let indices = &bytes[mip0_ofs..mip0_ofs + n];

    // Palette sits after mip3: offsets[3] + (w/8 * h/8) bytes, then a u16 count
    // (usually 256) followed by count*3 palette bytes.
    let mip3_ofs = hdr.offsets[3] as usize;
    let mip3_len = ((w / 8).max(1) * (h / 8).max(1)) as usize;
    let pal_count_ofs = mip3_ofs + mip3_len;
    if pal_count_ofs + 2 > bytes.len() {
        bail!("miptex {} palette header out of range", hdr.name);
    }
    let pal_count = u16::from_le_bytes([bytes[pal_count_ofs], bytes[pal_count_ofs + 1]]) as usize;
    let pal_count = pal_count.min(256);
    let pal_ofs = pal_count_ofs + 2;
    if pal_ofs + pal_count * 3 > bytes.len() {
        bail!("miptex {} palette out of range", hdr.name);
    }
    let palette = &bytes[pal_ofs..pal_ofs + pal_count * 3];

    let color_keyed = lookup_name.starts_with('{');
    let mut rgba = vec![0u8; n * 4];
    for (i, &idx) in indices.iter().enumerate() {
        let pi = idx as usize;
        let (r, g, b, a) = if color_keyed && idx == 255 {
            // Transparent luxel: zero the color to avoid blue halos on edges.
            (0, 0, 0, 0)
        } else if pi * 3 + 2 < palette.len() {
            (palette[pi * 3], palette[pi * 3 + 1], palette[pi * 3 + 2], 255)
        } else {
            (255, 0, 255, 255)
        };
        let o = i * 4;
        rgba[o] = r;
        rgba[o + 1] = g;
        rgba[o + 2] = b;
        rgba[o + 3] = a;
    }

    Ok(TextureData::from_rgba(w, h, rgba))
}
