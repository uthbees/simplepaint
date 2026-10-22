use crate::canvas::canvas_pos::CanvasPos;
use crate::dimensions::{DimensionsPx, DimensionsPxNonzero};
use egui::Rgba;
use tiny_skia::{Color, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};

/// Stores and manages the state of the canvas. Data is stored in a CPU-side 2D pixel buffer.
pub struct CanvasBuffer {
    dimensions: DimensionsPxNonzero,
    /// Row-major layout: pixel `(x, y)` is at index `y * width + x`.
    pixmap: Pixmap,
    /// Whether the buffer has been modified since the last GPU upload.
    pub dirty: bool,
}

impl CanvasBuffer {
    /// Creates a new canvas filled with a solid white background.
    pub fn new(dimensions: DimensionsPx) -> Self {
        let mut buffer = Self {
            dimensions: dimensions.to_nonzero(),
            pixmap: Pixmap::new(dimensions.width, dimensions.height)
                .expect("Pixmap row size should fit in an i32 and width/height should not be zero"),
            dirty: false,
        };
        buffer.pixmap.fill(Color::WHITE);
        buffer
    }

    pub fn dimensions(&self) -> DimensionsPxNonzero {
        self.dimensions
    }

    /// Exports the entire buffer as a flat premultiplied RGBA byte slice.
    ///
    /// The slice has length `width * height * 4`.
    pub fn as_bytes(&self) -> &[u8] {
        self.pixmap.data()
    }

    /// Draws a line from `pos1` to `pos2` with the specified thickness and color.
    #[allow(clippy::cast_sign_loss)]
    pub fn draw_line(&mut self, pos1: CanvasPos, pos2: CanvasPos, thickness: f32, color: Rgba) {
        // Build a path for the line segment (or a zero-length segment for a dot).
        let x1 = pos1.x as f32 + 0.5;
        let y1 = pos1.y as f32 + 0.5;
        let x2 = pos2.x as f32 + 0.5;
        let y2 = pos2.y as f32 + 0.5;

        let mut pb = PathBuilder::new();
        pb.move_to(x1, y1);
        pb.line_to(x2, y2);
        let Some(path) = pb.finish() else {
            return;
        };

        let stroke = Stroke {
            width: thickness,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        };

        let converted_color: Color = Color::from_rgba(
            color[0].clamp(0.0, 1.0),
            color[1].clamp(0.0, 1.0),
            color[2].clamp(0.0, 1.0),
            color[3].clamp(0.0, 1.0),
        )
        .expect("Color values should be clamped properly");
        let mut paint = Paint::default();
        paint.set_color(converted_color);
        paint.anti_alias = true;

        self.pixmap
            .stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        self.dirty = true;
    }
}
