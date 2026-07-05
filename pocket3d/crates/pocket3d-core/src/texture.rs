//! CPU-side texture payload shared by the WAD/BSP decoders, asset pipeline,
//! and renderer upload path.

/// An 8-bit RGBA texture in row-major order (top-left origin).
#[derive(Clone, Debug, Default)]
pub struct TextureData {
    pub width: u32,
    pub height: u32,
    /// `width * height * 4` bytes, RGBA8.
    pub rgba: Vec<u8>,
}

impl TextureData {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            rgba: vec![0; (width as usize) * (height as usize) * 4],
        }
    }

    pub fn from_rgba(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        debug_assert_eq!(rgba.len(), (width * height * 4) as usize);
        Self {
            width,
            height,
            rgba,
        }
    }

    /// A solid-color 1×1 texture, handy as a fallback for missing WAD textures.
    pub fn solid(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            width: 1,
            height: 1,
            rgba: vec![r, g, b, a],
        }
    }

    /// A magenta/black checker used to make missing textures obvious.
    pub fn missing() -> Self {
        let (w, h) = (16u32, 16u32);
        let mut rgba = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let checker = ((x / 4) + (y / 4)) % 2 == 0;
                let i = ((y * w + x) * 4) as usize;
                let (r, g, b) = if checker { (255, 0, 255) } else { (0, 0, 0) };
                rgba[i] = r;
                rgba[i + 1] = g;
                rgba[i + 2] = b;
                rgba[i + 3] = 255;
            }
        }
        Self {
            width: w,
            height: h,
            rgba,
        }
    }

    pub fn pixel_count(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }
}
