use crate::dimensions::DimensionsPxNonzero;
use tiny_skia::{Color, Pixmap};

/// Stores and manages the state of a canvas layer. Data is stored in a CPU-side 2D pixel buffer.
pub struct Layer {
    /// Row-major layout: pixel `(x, y)` is at index `y * width + x`.
    pub pixmap: Pixmap,
    /// Whether the buffer has been modified since the last GPU upload.
    pub dirty: bool,
}

impl Layer {
    /// Creates a new layer filled with a solid white background.
    pub fn new(dimensions: DimensionsPxNonzero) -> Self {
        let mut buffer = Self {
            pixmap: Pixmap::new(dimensions.width(), dimensions.height())
                .expect("Pixmap row size should fit in an i32 and width/height should not be zero"),
            dirty: false,
        };
        buffer.pixmap.fill(Color::TRANSPARENT);
        buffer
    }

    /// Exports the entire buffer as a flat premultiplied RGBA byte slice.
    ///
    /// The slice has length `width * height * 4`.
    pub fn as_bytes(&self) -> &[u8] {
        self.pixmap.data()
    }
}
