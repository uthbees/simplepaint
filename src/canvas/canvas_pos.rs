use crate::ui::canvas_panel::CanvasPanel;
use std::ops::Add;

/// A position in pixels relative to the canvas, with the top left corner being (0, 0).
///
/// Note that positions off the canvas are valid values. This includes negative coordinates,
/// even though all positions displayed on the canvas are positive.
#[derive(Copy, Clone, Eq, Debug)]
pub struct CanvasPos {
    pub x: i32,
    pub y: i32,
}

impl CanvasPos {
    /// Converts a position from logical points within the canvas panel to pixel coordinates relative
    /// to the canvas's pixel grid.
    ///
    /// This algorithm duplicates the algorithm in `canvas_panel.wgsl`. See the comments in that file
    /// for further explanation.
    #[must_use]
    pub fn from_panel_pos(
        panel_pos: egui::Vec2,
        canvas_rect: egui::Rect,
        canvas_panel_state: &CanvasPanel,
    ) -> CanvasPos {
        let norm_factors = canvas_panel_state.get_normalization_factors();
        let zoom = canvas_panel_state.zoom;
        let pan_px = canvas_panel_state.pan_px;
        let canvas = &canvas_panel_state.canvas;

        // Convert the pan to uv units before starting.
        let pan_uv_x = pan_px.x / canvas.width() as f32;
        let pan_uv_y = pan_px.y / canvas.height() as f32;

        let panel_uv_x = panel_pos.x / canvas_rect.width();
        let panel_uv_y = panel_pos.y / canvas_rect.height();

        let base_canvas_uv_x = (panel_uv_x - 0.5) * norm_factors[0];
        let base_canvas_uv_y = (panel_uv_y - 0.5) * norm_factors[1];

        let final_canvas_uv_x = (base_canvas_uv_x / zoom) - pan_uv_x + 0.5;
        let final_canvas_uv_y = (base_canvas_uv_y / zoom) - pan_uv_y + 0.5;

        let x_px = (final_canvas_uv_x * canvas.width() as f32) as i32;
        let y_px = (final_canvas_uv_y * canvas.height() as f32) as i32;

        CanvasPos { x: x_px, y: y_px }
    }
}

impl Add for CanvasPos {
    type Output = CanvasPos;

    fn add(self, rhs: Self) -> Self::Output {
        CanvasPos {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl PartialEq for CanvasPos {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}
