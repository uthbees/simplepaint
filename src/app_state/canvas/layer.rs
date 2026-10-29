use crate::utils::dimensions::DimensionsPxNonzero;
use std::hash::{Hash, Hasher};
use tiny_skia::{Color, Pixmap};

/// Stores and manages the state of a canvas layer. Data is stored in a CPU-side 2D pixel buffer.
#[derive(Debug)]
pub struct Layer {
    /// Row-major layout: pixel `(x, y)` is at index `y * width + x`.
    pub pixmap: Pixmap,
    /// Whether the buffer has been modified since the last GPU upload.
    pub dirty: bool,

    id: i32,
    pub name: String,
    pub enabled: bool,
}

impl Layer {
    /// Creates a new layer filled with a solid background, either transparent or white.
    pub fn new(dimensions: DimensionsPxNonzero, name: String, id: i32, fill_white: bool) -> Self {
        let mut buffer = Self {
            pixmap: Pixmap::new(dimensions.width(), dimensions.height())
                .expect("Pixmap row size should fit in an i32 and width/height should not be zero"),
            dirty: false,
            id,
            name,
            enabled: true,
        };
        let fill_color = if fill_white {
            Color::WHITE
        } else {
            Color::TRANSPARENT
        };

        buffer.pixmap.fill(fill_color);
        buffer
    }

    pub fn id(&self) -> &i32 {
        &self.id
    }

    /// Exports the entire buffer as a flat premultiplied RGBA byte slice.
    ///
    /// The slice has length `width * height * 4`.
    pub fn buffer_as_bytes(&self) -> &[u8] {
        self.pixmap.data()
    }
}

impl Hash for Layer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
