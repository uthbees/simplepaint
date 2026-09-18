use bytemuck::{Pod, Zeroable};

/// A single RGBA pixel stored as four `u8` bytes.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct PixelColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl PixelColor {
    pub const WHITE: Self = Self {
        r: 255,
        g: 255,
        b: 255,
        a: 255,
    };
    pub const BLACK: Self = Self {
        r: 0,
        g: 0,
        b: 0,
        a: 255,
    };
}

/// A CPU-side 2D pixel buffer backed by a contiguous `Vec<PixelColor>`.
///
/// Row-major layout: pixel `(x, y)` is at index `y * width + x`.
pub struct Canvas {
    width: u32,
    height: u32,
    pixels: Vec<PixelColor>,
}

impl Canvas {
    /// Create a new canvas filled with a solid white background.
    pub fn new(width: u32, height: u32) -> Self {
        let pixel_count = (width * height) as usize;
        Self {
            width,
            height,
            pixels: vec![PixelColor::WHITE; pixel_count],
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Set the pixel at `(x, y)` to the given color.
    ///
    /// Out-of-bounds coordinates are silently ignored.
    pub fn set_pixel(&mut self, x: u32, y: u32, color: PixelColor) {
        if x >= self.width || y >= self.height {
            return;
        }
        self.pixels[(y * self.width + x) as usize] = color;
    }

    /// Export the entire buffer as a flat RGBA byte slice.
    ///
    /// The slice has length `width * height * 4`.
    pub fn as_bytes(&self) -> &[u8] {
        bytemuck::cast_slice(&self.pixels)
    }
}
