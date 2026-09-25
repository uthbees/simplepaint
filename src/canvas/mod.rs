use bytemuck::{Pod, Zeroable};

pub mod canvas_buffer;
pub mod canvas_pos;

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
