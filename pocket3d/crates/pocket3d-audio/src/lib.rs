//! `pocket3d-audio` — the minimal audio layer for Pocket3D (DESIGN.md §22).
//!
//! §22 asks for something "minimal but present": a tiny abstraction that can
//! play the handful of sounds OpenStrike needs (gunshot, hit marker, bot death,
//! optional round cues and footsteps) without dictating a backend. The real
//! audio device is deliberately left as a *future* concern — v0 ships **no**
//! external audio dependency. Instead this crate provides:
//!
//! * [`AudioBackend`] — the trait every backend implements. Its three methods
//!   ([`play_2d`](AudioBackend::play_2d), [`play_3d`](AudioBackend::play_3d),
//!   [`set_listener`](AudioBackend::set_listener)) are **fire-and-forget**:
//!   they enqueue an intent and return immediately.
//! * [`NullAudio`] — the default silent backend for headless/CI runs.
//! * [`RecordingAudio`] — a test double that records every `play_*` call so
//!   headless sims can assert "the gunshot played" / "bot death played".
//! * [`SoundBank`] — maps logical sound *names* to stable [`SoundHandle`]s and
//!   can parse a RIFF/WAVE file into raw PCM (playback of that PCM is a v0
//!   no-op, but the asset path is proven end-to-end).
//!
//! ## Fire-and-forget contract (DESIGN.md §22)
//!
//! > "Audio must not block the main simulation loop."
//!
//! Every method on [`AudioBackend`] MUST return without blocking on I/O, device
//! locks, or decoding. A backend that needs to touch a real device is expected
//! to hand work to a mixer thread and return instantly. The backends in this
//! crate do trivially-cheap, non-blocking work (nothing, or a `Vec` push), so
//! they honour the contract by construction.

use std::collections::HashMap;
use std::path::Path;

use pocket3d_core::SoundHandle;

// Re-export `glam` so downstream users can spell `pocket3d_audio::glam::Vec3`
// without adding their own dependency; it is the same pinned build core uses.
pub use glam;
use glam::Vec3;

// ---------------------------------------------------------------------------
// AudioBackend trait
// ---------------------------------------------------------------------------

/// A pluggable audio sink (DESIGN.md §22).
///
/// All methods are **fire-and-forget**: they record/enqueue the request and
/// return immediately. Implementations **must not block** the calling
/// (simulation) thread — no waiting on device locks, disk, or decoders. A real
/// backend should post work to a mixer thread and return.
pub trait AudioBackend {
    /// Play a sound as a non-positional 2D cue (UI clicks, hit markers, music).
    ///
    /// `volume` is a linear gain, conventionally in `[0.0, 1.0]`. Fire-and-forget.
    fn play_2d(&mut self, sound: SoundHandle, volume: f32);

    /// Play a sound positioned in world space (`Z`-up, right-handed — see
    /// `pocket3d_core` §8). Spatialisation relative to the current listener is
    /// the backend's job. Fire-and-forget.
    fn play_3d(&mut self, sound: SoundHandle, pos: Vec3, volume: f32);

    /// Update the listener pose used to spatialise [`play_3d`] calls.
    ///
    /// `forward` and `up` describe the listener orientation (world space).
    /// Fire-and-forget.
    fn set_listener(&mut self, pos: Vec3, forward: Vec3, up: Vec3);
}

// ---------------------------------------------------------------------------
// NullAudio — the default silent backend
// ---------------------------------------------------------------------------

/// The default headless/silent backend: every call is a no-op.
///
/// This is what runs in CI, in the BSP/asset tools, and anywhere a real device
/// is unavailable or unwanted. Being a no-op, it is trivially non-blocking and
/// so satisfies the §22 fire-and-forget contract.
#[derive(Debug, Clone, Copy, Default)]
pub struct NullAudio;

impl NullAudio {
    /// Construct the silent backend.
    pub fn new() -> Self {
        Self
    }
}

impl AudioBackend for NullAudio {
    #[inline]
    fn play_2d(&mut self, _sound: SoundHandle, _volume: f32) {}

    #[inline]
    fn play_3d(&mut self, _sound: SoundHandle, _pos: Vec3, _volume: f32) {}

    #[inline]
    fn set_listener(&mut self, _pos: Vec3, _forward: Vec3, _up: Vec3) {}
}

// ---------------------------------------------------------------------------
// RecordingAudio — a test double
// ---------------------------------------------------------------------------

/// One recorded `play_*` request captured by [`RecordingAudio`].
///
/// `pos` is `Some` for [`play_3d`](AudioBackend::play_3d) and `None` for
/// [`play_2d`](AudioBackend::play_2d); `is_3d` mirrors that for ergonomic
/// assertions.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayedSound {
    /// The sound that was requested.
    pub sound: SoundHandle,
    /// Linear gain passed to the play call.
    pub volume: f32,
    /// World position for 3D plays; `None` for 2D plays.
    pub pos: Option<Vec3>,
    /// `true` if this came from [`play_3d`](AudioBackend::play_3d).
    pub is_3d: bool,
}

/// The last listener pose seen by [`RecordingAudio::set_listener`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ListenerPose {
    pub pos: Vec3,
    pub forward: Vec3,
    pub up: Vec3,
}

/// An [`AudioBackend`] that records every play call instead of making sound.
///
/// Headless tests and the deterministic sim use this to assert audio behaviour
/// ("gunshot played", "bot death played") without a device. Recording is a
/// single `Vec` push, so it stays fire-and-forget per §22.
#[derive(Debug, Clone, Default)]
pub struct RecordingAudio {
    /// Every `play_2d`/`play_3d` call, in the order it arrived.
    pub played: Vec<PlayedSound>,
    /// The most recent listener pose, if `set_listener` was ever called.
    pub last_listener: Option<ListenerPose>,
}

impl RecordingAudio {
    /// Construct an empty recorder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Number of recorded play calls.
    pub fn count(&self) -> usize {
        self.played.len()
    }

    /// Forget all recorded calls (keeps the last listener pose).
    pub fn clear(&mut self) {
        self.played.clear();
    }

    /// `true` if `sound` was played at least once (2D or 3D).
    pub fn played_sound(&self, sound: SoundHandle) -> bool {
        self.played.iter().any(|p| p.sound == sound)
    }
}

impl AudioBackend for RecordingAudio {
    fn play_2d(&mut self, sound: SoundHandle, volume: f32) {
        self.played.push(PlayedSound {
            sound,
            volume,
            pos: None,
            is_3d: false,
        });
    }

    fn play_3d(&mut self, sound: SoundHandle, pos: Vec3, volume: f32) {
        self.played.push(PlayedSound {
            sound,
            volume,
            pos: Some(pos),
            is_3d: true,
        });
    }

    fn set_listener(&mut self, pos: Vec3, forward: Vec3, up: Vec3) {
        self.last_listener = Some(ListenerPose { pos, forward, up });
    }
}

// ---------------------------------------------------------------------------
// WAV parsing
// ---------------------------------------------------------------------------

/// Decoded metadata + raw PCM payload from a RIFF/WAVE file.
///
/// v0 does not *play* this PCM — it exists to prove the asset pipeline can load
/// real sound files and hand a [`SoundHandle`] back. A future rodio-style
/// backend can consume [`WavData::pcm`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WavData {
    /// WAVE `fmt ` audio format tag (1 = uncompressed PCM).
    pub audio_format: u16,
    /// Channel count (1 = mono, 2 = stereo, ...).
    pub channels: u16,
    /// Samples per second, e.g. 44_100.
    pub sample_rate: u32,
    /// Bits per sample, e.g. 16.
    pub bits_per_sample: u16,
    /// Raw bytes of the `data` chunk (interleaved PCM frames).
    pub pcm: Vec<u8>,
}

/// Errors returned by [`parse_wav`] / [`SoundBank::load_wav`].
///
/// Note that a *missing file* surfaces as [`WavError::Io`] — loading never
/// panics (DESIGN.md §22: "Do not panic on missing files; return `Result`").
#[derive(Debug, thiserror::Error)]
pub enum WavError {
    /// The file could not be read (missing, permissions, ...).
    #[error("failed to read WAV file: {0}")]
    Io(#[from] std::io::Error),
    /// Buffer is too small to even contain the 12-byte RIFF/WAVE header.
    #[error("WAV data too short ({0} bytes; need at least 12)")]
    TooShort(usize),
    /// The first four bytes were not the ASCII magic `RIFF`.
    #[error("missing 'RIFF' magic")]
    BadRiffMagic,
    /// Bytes 8..12 were not the ASCII magic `WAVE`.
    #[error("missing 'WAVE' magic")]
    BadWaveMagic,
    /// A chunk header claimed a size that runs past the end of the buffer.
    #[error("truncated chunk: declared size runs past end of buffer")]
    TruncatedChunk,
    /// No `fmt ` chunk was present.
    #[error("missing 'fmt ' chunk")]
    MissingFmtChunk,
    /// The `fmt ` chunk was smaller than the 16-byte PCM header.
    #[error("'fmt ' chunk too small ({0} bytes; need at least 16)")]
    FmtChunkTooSmall(usize),
    /// No `data` chunk was present.
    #[error("missing 'data' chunk")]
    MissingDataChunk,
}

/// Read a little-endian `u16` at `off`, or `None` if out of bounds.
fn read_u16_le(bytes: &[u8], off: usize) -> Option<u16> {
    bytes
        .get(off..off + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
}

/// Read a little-endian `u32` at `off`, or `None` if out of bounds.
fn read_u32_le(bytes: &[u8], off: usize) -> Option<u32> {
    bytes
        .get(off..off + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// Parse an in-memory RIFF/WAVE buffer (DESIGN.md §22 asset path).
///
/// Validates the `RIFF`/`WAVE` magics, walks the chunk list to find `fmt ` and
/// `data`, reads sample rate / channels / bits-per-sample, and returns the raw
/// PCM. This is pure (no filesystem) so it is easy to unit-test with synthesised
/// bytes. It never panics — malformed input yields a [`WavError`].
pub fn parse_wav(bytes: &[u8]) -> Result<WavData, WavError> {
    // 12-byte container header: "RIFF" <u32 size> "WAVE".
    if bytes.len() < 12 {
        return Err(WavError::TooShort(bytes.len()));
    }
    if &bytes[0..4] != b"RIFF" {
        return Err(WavError::BadRiffMagic);
    }
    if &bytes[8..12] != b"WAVE" {
        return Err(WavError::BadWaveMagic);
    }

    let mut fmt: Option<(u16, u16, u32, u16)> = None; // (format, channels, rate, bits)
    let mut pcm: Option<Vec<u8>> = None;

    // Walk the sub-chunk list. Each sub-chunk is: 4-byte id, 4-byte LE size,
    // `size` bytes of body, then a pad byte if `size` is odd (word alignment).
    let mut off = 12usize;
    while off + 8 <= bytes.len() {
        let id = &bytes[off..off + 4];
        // Bounds already guaranteed by the loop condition.
        let size = read_u32_le(bytes, off + 4).unwrap() as usize;
        let body_start = off + 8;
        let body_end = body_start
            .checked_add(size)
            .ok_or(WavError::TruncatedChunk)?;
        if body_end > bytes.len() {
            return Err(WavError::TruncatedChunk);
        }
        let body = &bytes[body_start..body_end];

        match id {
            b"fmt " => {
                if body.len() < 16 {
                    return Err(WavError::FmtChunkTooSmall(body.len()));
                }
                let audio_format = read_u16_le(body, 0).unwrap();
                let channels = read_u16_le(body, 2).unwrap();
                let sample_rate = read_u32_le(body, 4).unwrap();
                // bytes 8..12 = byte rate, 12..14 = block align (not needed here).
                let bits_per_sample = read_u16_le(body, 14).unwrap();
                fmt = Some((audio_format, channels, sample_rate, bits_per_sample));
            }
            b"data" => {
                pcm = Some(body.to_vec());
            }
            // Ignore all other chunks (LIST/INFO, fact, cue, ...).
            _ => {}
        }

        // Advance past the body plus the odd-size pad byte.
        off = body_end + (size & 1);
    }

    let (audio_format, channels, sample_rate, bits_per_sample) =
        fmt.ok_or(WavError::MissingFmtChunk)?;
    let pcm = pcm.ok_or(WavError::MissingDataChunk)?;

    Ok(WavData {
        audio_format,
        channels,
        sample_rate,
        bits_per_sample,
        pcm,
    })
}

// ---------------------------------------------------------------------------
// SoundBank
// ---------------------------------------------------------------------------

/// One registered logical sound: its name and (optionally) its loaded PCM.
#[derive(Debug, Clone, Default)]
struct SoundEntry {
    name: String,
    wav: Option<WavData>,
}

/// Registry mapping logical sound *names* to stable [`SoundHandle`]s
/// (DESIGN.md §22).
///
/// Handles are dense indices assigned in registration order and are **stable**
/// for the lifetime of the bank: registering the same name twice returns the
/// same handle, and handles are never reused or invalidated. Game code can
/// resolve `"gunshot"` once at load and then fire the handle at the backend.
///
/// [`load_wav`](SoundBank::load_wav) parses a RIFF/WAVE file and attaches its
/// PCM to the name's handle. In v0 the PCM is never played (the backends above
/// are silent or recording), but loading it proves the asset path works and
/// gives a future real backend something to mix.
#[derive(Debug, Clone, Default)]
pub struct SoundBank {
    /// Entries indexed by `SoundHandle::index()`.
    entries: Vec<SoundEntry>,
    /// name -> handle, for `handle_of`.
    by_name: HashMap<String, SoundHandle>,
}

impl SoundBank {
    /// Construct an empty bank.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a logical sound name, returning its stable handle.
    ///
    /// Idempotent: calling with an already-registered name returns the existing
    /// handle rather than allocating a new one.
    pub fn register(&mut self, name: &str) -> SoundHandle {
        if let Some(&h) = self.by_name.get(name) {
            return h;
        }
        let handle = SoundHandle(self.entries.len() as u32);
        self.entries.push(SoundEntry {
            name: name.to_owned(),
            wav: None,
        });
        self.by_name.insert(name.to_owned(), handle);
        handle
    }

    /// Look up the handle previously assigned to `name`, if any.
    pub fn handle_of(&self, name: &str) -> Option<SoundHandle> {
        self.by_name.get(name).copied()
    }

    /// Number of registered sounds.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// `true` if no sounds are registered.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The logical name behind a handle, if it belongs to this bank.
    pub fn name_of(&self, handle: SoundHandle) -> Option<&str> {
        self.entries
            .get(handle.index())
            .map(|e| e.name.as_str())
    }

    /// Borrow the decoded [`WavData`] attached to a handle, if any was loaded.
    pub fn wav(&self, handle: SoundHandle) -> Option<&WavData> {
        self.entries.get(handle.index()).and_then(|e| e.wav.as_ref())
    }

    /// Borrow the raw PCM bytes attached to a handle, if any was loaded.
    pub fn pcm(&self, handle: SoundHandle) -> Option<&[u8]> {
        self.wav(handle).map(|w| w.pcm.as_slice())
    }

    /// Register `name` (if needed) and attach PCM parsed from a RIFF/WAVE file.
    ///
    /// Returns the (stable) handle for `name`. **Never panics on a missing or
    /// malformed file** — I/O and parse failures surface as [`WavError`]
    /// (DESIGN.md §22). Playback of the loaded PCM is a v0 no-op; this exists to
    /// prove the asset path.
    pub fn load_wav(&mut self, name: &str, path: impl AsRef<Path>) -> Result<SoundHandle, WavError> {
        let bytes = std::fs::read(path)?; // missing file -> Err, not panic.
        let wav = parse_wav(&bytes)?;
        let handle = self.register(name);
        // `register` guarantees the entry exists at this index.
        self.entries[handle.index()].wav = Some(wav);
        Ok(handle)
    }

    /// Attach already-parsed [`WavData`] to `name` (registering it if needed).
    ///
    /// Useful for embedded assets or tests that build PCM in memory instead of
    /// reading a file.
    pub fn insert_wav(&mut self, name: &str, wav: WavData) -> SoundHandle {
        let handle = self.register(name);
        self.entries[handle.index()].wav = Some(wav);
        handle
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Build the bytes of a minimal 16-bit mono PCM WAV in memory so the parser
    /// can be exercised without touching the filesystem.
    fn synth_wav(sample_rate: u32, channels: u16, bits: u16, pcm: &[u8]) -> Vec<u8> {
        let byte_rate = sample_rate * channels as u32 * (bits as u32 / 8);
        let block_align = channels * (bits / 8);
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        // RIFF chunk size = 4 ("WAVE") + (8 + 16 fmt) + (8 + data).
        let riff_size = 4 + (8 + 16) + (8 + pcm.len() as u32);
        out.extend_from_slice(&riff_size.to_le_bytes());
        out.extend_from_slice(b"WAVE");
        // fmt  chunk.
        out.extend_from_slice(b"fmt ");
        out.extend_from_slice(&16u32.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes()); // PCM
        out.extend_from_slice(&channels.to_le_bytes());
        out.extend_from_slice(&sample_rate.to_le_bytes());
        out.extend_from_slice(&byte_rate.to_le_bytes());
        out.extend_from_slice(&block_align.to_le_bytes());
        out.extend_from_slice(&bits.to_le_bytes());
        // data chunk.
        out.extend_from_slice(b"data");
        out.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
        out.extend_from_slice(pcm);
        out
    }

    #[test]
    fn recording_audio_records_2d_and_3d() {
        let mut bank = SoundBank::new();
        let gunshot = bank.register("gunshot");
        let death = bank.register("bot_death");

        let mut audio = RecordingAudio::new();
        // Fire-and-forget calls; the recorder just captures them.
        audio.set_listener(Vec3::ZERO, Vec3::Y, Vec3::Z);
        audio.play_2d(gunshot, 0.8);
        audio.play_3d(death, Vec3::new(1.0, 2.0, 3.0), 0.5);

        assert_eq!(audio.count(), 2);

        // First call: 2D gunshot, no position.
        let a = audio.played[0];
        assert_eq!(a.sound, gunshot);
        assert_eq!(a.volume, 0.8);
        assert_eq!(a.pos, None);
        assert!(!a.is_3d);

        // Second call: 3D bot death, with position.
        let b = audio.played[1];
        assert_eq!(b.sound, death);
        assert_eq!(b.volume, 0.5);
        assert_eq!(b.pos, Some(Vec3::new(1.0, 2.0, 3.0)));
        assert!(b.is_3d);

        // Listener pose was captured.
        assert_eq!(
            audio.last_listener,
            Some(ListenerPose {
                pos: Vec3::ZERO,
                forward: Vec3::Y,
                up: Vec3::Z
            })
        );

        // Convenience query.
        assert!(audio.played_sound(gunshot));
        assert!(audio.played_sound(death));
    }

    #[test]
    fn null_audio_is_a_silent_noop() {
        // Just proves the default backend implements the trait and never panics.
        let mut audio = NullAudio::new();
        let s = SoundHandle(0);
        audio.play_2d(s, 1.0);
        audio.play_3d(s, Vec3::new(0.0, 0.0, 1.0), 1.0);
        audio.set_listener(Vec3::ZERO, Vec3::Y, Vec3::Z);
    }

    #[test]
    fn sound_bank_hands_out_stable_handles() {
        let mut bank = SoundBank::new();
        let a = bank.register("gunshot");
        let b = bank.register("bot_death");
        let c = bank.register("gunshot"); // re-register same name

        // Distinct names get distinct, dense indices.
        assert_eq!(a, SoundHandle(0));
        assert_eq!(b, SoundHandle(1));
        // Re-registering returns the SAME (stable) handle.
        assert_eq!(a, c);
        assert_eq!(bank.len(), 2);

        // Lookups resolve.
        assert_eq!(bank.handle_of("gunshot"), Some(a));
        assert_eq!(bank.handle_of("bot_death"), Some(b));
        assert_eq!(bank.handle_of("missing"), None);
        assert_eq!(bank.name_of(a), Some("gunshot"));

        // No PCM attached until we load it.
        assert!(bank.pcm(a).is_none());
    }

    #[test]
    fn parses_a_synthesized_wav() {
        // 8 bytes = four 16-bit mono samples.
        let pcm = [0x01, 0x00, 0xFF, 0x7F, 0x00, 0x80, 0x34, 0x12];
        let bytes = synth_wav(44_100, 1, 16, &pcm);

        let wav = parse_wav(&bytes).expect("synthesized WAV must parse");
        assert_eq!(wav.audio_format, 1);
        assert_eq!(wav.channels, 1);
        assert_eq!(wav.sample_rate, 44_100);
        assert_eq!(wav.bits_per_sample, 16);
        assert_eq!(wav.pcm, pcm);
    }

    #[test]
    fn parse_rejects_bad_magic_without_panicking() {
        assert!(matches!(parse_wav(b"nope"), Err(WavError::TooShort(4))));
        let mut bytes = vec![0u8; 12];
        bytes[0..4].copy_from_slice(b"XXXX");
        assert!(matches!(parse_wav(&bytes), Err(WavError::BadRiffMagic)));
    }

    #[test]
    fn load_wav_roundtrips_through_a_file_and_missing_file_errs() {
        let pcm = [0xAA, 0xBB, 0xCC, 0xDD];
        let bytes = synth_wav(22_050, 2, 16, &pcm);

        // Write to a unique temp path.
        let mut path = std::env::temp_dir();
        path.push(format!("pocket3d_audio_test_{}.wav", std::process::id()));
        std::fs::write(&path, &bytes).unwrap();

        let mut bank = SoundBank::new();
        let h = bank.load_wav("footstep", &path).expect("should load");
        assert_eq!(bank.handle_of("footstep"), Some(h));
        assert_eq!(bank.pcm(h), Some(pcm.as_slice()));
        assert_eq!(bank.wav(h).unwrap().sample_rate, 22_050);

        let _ = std::fs::remove_file(&path);

        // Missing file must return Err, never panic (DESIGN.md §22).
        let err = bank.load_wav("nope", "/no/such/pocket3d/file.wav");
        assert!(matches!(err, Err(WavError::Io(_))));
    }
}
