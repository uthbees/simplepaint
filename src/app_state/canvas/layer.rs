use crate::utils::dimensions::DimensionsPxNonzero;
use std::hash::{Hash, Hasher};
use tiny_skia::{Color, Pixmap};

/// Stores and manages the state of a canvas layer. Data is stored in a CPU-side 2D pixel buffer.
#[derive(Debug)]
pub struct Layer {
    /// Row-major layout: pixel `(x, y)` is at index `y * width + x`.
    pub(super) pixmap: Pixmap,
    /// Whether the buffer has been modified since the last GPU upload.
    pub dirty: bool,

    id: i32,
    pub name: String,
    pub enabled: bool,
}

impl Layer {
    pub fn new(
        dimensions: DimensionsPxNonzero,
        name: String,
        id: i32,
        initial_color: &InitialColor,
    ) -> Self {
        let mut pixmap = Pixmap::new(dimensions.width(), dimensions.height())
            .expect("Pixmap row size should fit in an i32 and width/height should not be zero");
        pixmap.fill(match initial_color {
            InitialColor::White => Color::WHITE,
            InitialColor::Transparent => Color::TRANSPARENT,
        });

        Self {
            pixmap,
            dirty: false,
            id,
            name,
            enabled: true,
        }
    }

    pub fn new_from_pixmap(name: String, id: i32, pixmap: Pixmap) -> Self {
        Self {
            pixmap,
            dirty: false,
            id,
            name,
            enabled: true,
        }
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

pub enum InitialColor {
    White,
    Transparent,
}
