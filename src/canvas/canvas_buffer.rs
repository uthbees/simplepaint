use crate::canvas::PixelColor;
use crate::canvas::canvas_pos::CanvasPos;
use tiny_skia::{Color, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};

/// Stores and manages the state of the canvas. Data is stored in a CPU-side 2D pixel buffer.
pub struct CanvasBuffer {
    width: u32,
    height: u32,
    /// Row-major layout: pixel `(x, y)` is at index `y * width + x`.
    pixmap: Pixmap,
    /// Whether the buffer has been modified since the last GPU upload.
    pub dirty: bool,
}

impl CanvasBuffer {
    /// Creates a new canvas filled with a solid white background.
    pub fn new(width: u32, height: u32) -> Self {
        let mut buffer = Self {
            width,
            height,
            pixmap: Pixmap::new(width, height)
                .expect("Pixmap row size should fit in an i32 and width/height should not be zero"),
            dirty: false,
        };
        buffer.pixmap.fill(Color::WHITE);
        buffer
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Exports the entire buffer as a flat premultiplied RGBA byte slice.
    ///
    /// The slice has length `width * height * 4`.
    pub fn as_bytes(&self) -> &[u8] {
        self.pixmap.data()
    }

    /// Draws a line from `pos1` to `pos2` with the specified thickness and color.
    #[allow(clippy::cast_sign_loss)]
    pub fn draw_line(&mut self, pos1: CanvasPos, pos2: CanvasPos, thickness: f32, color: PixelColor) {
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

        let mut paint = Paint::default();
        paint.set_color(Color::from_rgba8(color.r, color.g, color.b, color.a));
        paint.anti_alias = true;

        self.pixmap
            .stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        self.dirty = true;
    }
}
