use crate::canvas::PixelColor;
use crate::canvas::canvas_buffer::CanvasBuffer;
use crate::canvas::canvas_pos::CanvasPos;
use crate::ui::scrollbar::{
    SCROLLBAR_TRACK_COLOR, SCROLLBAR_TRACK_SIZE_PT, Scrollbar, ScrollbarAxis,
};
use egui::{CentralPanel, Vec2};

const DEFAULT_CANVAS_HEIGHT: u32 = 500;
const DEFAULT_CANVAS_WIDTH: u32 = 500;

pub struct CanvasPanel {
    egui_ctx: egui::Context,
    canvas_view_pts: egui::Rect,
    canvas_view_texture_id: Option<egui::TextureId>,
    scrollbars: CanvasPanelScrollbars,
    pub canvas: CanvasBuffer,
    /// The x/y pan offset of the center of the canvas from the center of the panel.
    /// Measured in canvas pixels. (0, 0) puts the center of the canvas in the center of the panel.
    pub pan_px: Vec2,
    /// Canvas zoom. Larger numbers zoom in, smaller numbers zoom out.
    /// 1 is 1:1 screen px:canvas px, 2 is 2:1 screen px:canvas px, 0.5 is 1:2 screen px:canvas px.
    pub zoom: f32,
    pub last_drag_pos: Option<CanvasPos>,
}

struct CanvasPanelScrollbars {
    horizontal: Scrollbar,
    vertical: Scrollbar,
}

impl CanvasPanel {
    pub fn new(egui_ctx: egui::Context) -> Self {
        Self {
            egui_ctx,
            canvas_view_pts: egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::ZERO),
            canvas_view_texture_id: None,
            scrollbars: CanvasPanelScrollbars {
                horizontal: Scrollbar::new(ScrollbarAxis::Horizontal),
                vertical: Scrollbar::new(ScrollbarAxis::Vertical),
            },
            canvas: CanvasBuffer::new(DEFAULT_CANVAS_WIDTH, DEFAULT_CANVAS_HEIGHT),
            pan_px: Vec2::ZERO,
            zoom: 1.0,
            last_drag_pos: None,
        }
    }

    pub fn set_canvas_view_texture_id(&mut self, texture_id: egui::TextureId) {
        self.canvas_view_texture_id = Some(texture_id);
    }

    pub fn canvas_view_px(&self) -> egui::Rect {
        self.canvas_view_pts * self.egui_ctx.pixels_per_point()
    }

    pub fn draw(&mut self, egui_ui: &mut egui::Ui) {
        CentralPanel::default().show(egui_ui, |egui_ui| {
            egui_ui.spacing_mut().item_spacing = Vec2::ZERO;
            let panel_rect = egui_ui.available_rect_before_wrap();

            let canvas_size = egui::vec2(
                (panel_rect.width() - SCROLLBAR_TRACK_SIZE_PT).max(1.0),
                (panel_rect.height() - SCROLLBAR_TRACK_SIZE_PT).max(1.0),
            );

            let (canvas_view_rect, response) =
                egui_ui.allocate_exact_size(canvas_size, egui::Sense::DRAG);
            self.canvas_view_pts = canvas_view_rect;

            if let Some(canvas_view_texture_id) = self.canvas_view_texture_id {
                egui_ui.painter().image(
                    canvas_view_texture_id,
                    canvas_view_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            self.handle_canvas_view_input(egui_ui, &response);

            self.draw_scrollbars(egui_ui, panel_rect, canvas_view_rect);
        });
    }

    #[allow(clippy::similar_names)]
    fn draw_scrollbars(
        &mut self,
        egui_ui: &mut egui::Ui,
        panel_rect: egui::Rect,
        canvas_view_rect: egui::Rect,
    ) {
        let horizontal_track = egui::Rect::from_min_max(
            egui::pos2(panel_rect.left(), canvas_view_rect.bottom()),
            egui::pos2(canvas_view_rect.right(), panel_rect.bottom()),
        );
        let vertical_track = egui::Rect::from_min_max(
            egui::pos2(canvas_view_rect.right(), panel_rect.top()),
            egui::pos2(panel_rect.right(), canvas_view_rect.bottom()),
        );
        let corner = egui::Rect::from_min_max(canvas_view_rect.max, panel_rect.max);

        let canvas_view_px = self.canvas_view_px();

        // Calculate the viewport fractions, first just with respect to the canvas, then also
        // including the off-canvas scroll buffer.
        let canvas_x_viewport_fraction =
            canvas_view_px.width() / (self.canvas.width() as f32 * self.zoom);
        let full_x_viewport_fraction =
            canvas_x_viewport_fraction / (canvas_x_viewport_fraction + 1.0);
        let canvas_y_viewport_fraction =
            canvas_view_px.height() / (self.canvas.height() as f32 * self.zoom);
        let full_y_viewport_fraction =
            canvas_y_viewport_fraction / (canvas_y_viewport_fraction + 1.0);

        let updated_horizontal_pan = self.scrollbars.horizontal.draw(
            egui_ui,
            horizontal_track,
            full_x_viewport_fraction,
            -self.pan_px.x / self.canvas.width() as f32 + 0.5,
        );
        let updated_vertical_pan = self.scrollbars.vertical.draw(
            egui_ui,
            vertical_track,
            full_y_viewport_fraction,
            -self.pan_px.y / self.canvas.height() as f32 + 0.5,
        );

        let mut pan_changed = false;
        if let Some(updated_horizontal_pan) = updated_horizontal_pan {
            pan_changed = true;
            self.pan_px.x = (0.5 - updated_horizontal_pan) * self.canvas.width() as f32;
        }
        if let Some(updated_vertical_pan) = updated_vertical_pan {
            pan_changed = true;
            self.pan_px.y = (0.5 - updated_vertical_pan) * self.canvas.height() as f32;
        }
        if pan_changed {
            self.clamp_pan();
        }

        egui_ui
            .painter()
            .rect_filled(corner, 0.0, SCROLLBAR_TRACK_COLOR);
    }

    fn handle_canvas_view_input(&mut self, egui_ui: &mut egui::Ui, response: &egui::Response) {
        if response.hovered() {
            self.handle_scrolled(egui_ui, response);
        }

        if response.drag_stopped() {
            self.last_drag_pos = None;
        } else if response.dragged() {
            self.handle_dragged(egui_ui, response);
        }
    }

    fn handle_scrolled(&mut self, egui_ui: &egui::Ui, response: &egui::Response) {
        let scroll_delta = egui_ui.input(egui::InputState::translation_delta);
        if scroll_delta != Vec2::ZERO {
            let pan_delta = scroll_delta / self.zoom;
            self.pan_px += pan_delta;
            self.clamp_pan();
        }

        let zoom_delta = egui_ui.input(egui::InputState::zoom_delta);
        #[allow(clippy::float_cmp)]
        if zoom_delta != 1.0 {
            let old_zoom = self.zoom;
            let new_zoom = (self.zoom * zoom_delta).clamp(0.1, 10.0);

            let px_per_pt = self.egui_ctx.pixels_per_point();
            let cursor_pos_px = (egui_ui
                .input(|i| i.pointer.hover_pos())
                .unwrap_or_else(|| response.rect.center())
                - response.rect.min)
                * px_per_pt;

            let canvas_view_px = self.canvas_view_px();
            let view_center = egui::vec2(canvas_view_px.width(), canvas_view_px.height()) * 0.5;
            let cursor_offset = cursor_pos_px - view_center;

            let zoom_factor_delta = 1.0 / new_zoom - 1.0 / old_zoom;
            let pan_delta = cursor_offset * zoom_factor_delta;

            self.pan_px += pan_delta;
            self.zoom = new_zoom;
            self.clamp_pan();
        }
    }

    fn handle_dragged(&mut self, egui_ui: &mut egui::Ui, canvas_response: &egui::Response) {
        if let Some(cursor_pos) = egui_ui.input(|i| i.pointer.interact_pos()) {
            let rect = canvas_response.rect;

            let relative_pos = cursor_pos - rect.min;
            let pos_px = CanvasPos::from_panel_pos(relative_pos, rect, self);

            self.canvas.draw_line(
                pos_px,
                self.last_drag_pos.unwrap_or(pos_px),
                1.0,
                PixelColor::BLACK,
            );
            self.last_drag_pos = Some(pos_px);
        }
    }

    /// Clamps the pan to prevent the edges of the canvas from going beyond the center of the panel.
    fn clamp_pan(&mut self) {
        let half_w = self.canvas.width() as f32 * 0.5;
        let half_h = self.canvas.height() as f32 * 0.5;
        self.pan_px.x = self.pan_px.x.clamp(-half_w, half_w);
        self.pan_px.y = self.pan_px.y.clamp(-half_h, half_h);
    }

    /// Calculates the current normalization factors for the canvas view. These factors can be
    /// multiplied with the starting uvs (numbers ranging linearly from 0 to 1 across the
    /// canvas panel coordinates) to normalize the points to a canvas with the proper aspect ratio
    /// and a 1:1 canvas pixel:screen pixel scale.
    /// Note that these factors do not account for zooming or panning.
    pub fn get_normalization_factors(&self) -> [f32; 2] {
        let canvas_view_px = self.canvas_view_px();

        [
            canvas_view_px.width() / self.canvas.width() as f32,
            canvas_view_px.height() / self.canvas.height() as f32,
        ]
    }
}
