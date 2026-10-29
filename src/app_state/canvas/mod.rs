use crate::app_state::canvas::canvas_pos::CanvasPos;
use crate::utils::dimensions::DimensionsPxNonzero;
use egui::Rgba;
use layer::Layer;
use tiny_skia::{Color, LineCap, LineJoin, Paint, PathBuilder, Stroke, Transform};

pub mod canvas_pos;
pub mod layer;

pub struct Canvas {
    dimensions: DimensionsPxNonzero,
    pub layers: Vec<Layer>,

    selected_layer_idx: Option<usize>,
    enabled_layers: EnabledLayersConfig,
}

pub struct EnabledLayersConfig {
    states: Vec<bool>,
    // Indicates whether there have been any changes here since the last render.
    pub dirty: bool,
}

impl Canvas {
    pub fn new(dimensions: DimensionsPxNonzero) -> Canvas {
        Canvas {
            dimensions,
            layers: vec![Layer::new(dimensions, true)],
            selected_layer_idx: Some(0),
            enabled_layers: EnabledLayersConfig {
                states: vec![true],
                dirty: false,
            },
        }
    }

    pub fn dimensions(&self) -> DimensionsPxNonzero {
        self.dimensions
    }

    pub fn enabled_layers(&mut self) -> &mut EnabledLayersConfig {
        &mut self.enabled_layers
    }

    /// Draws a line from `pos1` to `pos2` with the specified thickness and color.
    #[allow(clippy::cast_sign_loss)]
    pub fn draw_line(&mut self, pos1: CanvasPos, pos2: CanvasPos, thickness: f32, raw_color: Rgba) {
        let Some(active_layer) = self.get_active_layer() else {
            return;
        };

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
            raw_color[0].clamp(0.0, 1.0),
            raw_color[1].clamp(0.0, 1.0),
            raw_color[2].clamp(0.0, 1.0),
            raw_color[3].clamp(0.0, 1.0),
        )
        .expect("Color values should be clamped properly");
        let mut paint = Paint::default();
        paint.set_color(converted_color);
        paint.anti_alias = true;

        active_layer
            .pixmap
            .stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        active_layer.dirty = true;
    }

    fn get_active_layer(&mut self) -> Option<&mut Layer> {
        let selected_layer_idx = self.selected_layer_idx?;
        assert!(selected_layer_idx < self.layers.len());
        Some(&mut self.layers[selected_layer_idx])
    }
}

impl EnabledLayersConfig {
    pub fn states(&self) -> &[bool] {
        &self.states
    }
}
