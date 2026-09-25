use crate::canvas::PixelColor;
use crate::canvas::canvas_buffer::CanvasBuffer;
use crate::canvas::canvas_pos::CanvasPos;
use egui::{CentralPanel, Vec2};

const DEFAULT_CANVAS_HEIGHT: u32 = 500;
const DEFAULT_CANVAS_WIDTH: u32 = 500;

pub struct CanvasPanel {
    egui_ctx: egui::Context,
    panel_rect_pts: egui::Rect,
    pub canvas: CanvasBuffer,
    /// The x/y pan offset of the canvas in canvas pixels. (0, 0) centers the canvas.
    pub pan_px: egui::Vec2,
    /// Canvas zoom. Larger numbers zoom in, smaller numbers zoom out.
    /// 1 is 1:1 screen px:canvas px, 2 is 2:1 screen px:canvas px, 0.5 is 1:2 screen px:canvas px.
    pub zoom: f32,
    pub last_drag_pos: Option<CanvasPos>,
}

impl CanvasPanel {
    pub fn new(egui_ctx: egui::Context) -> Self {
        Self {
            egui_ctx,
            panel_rect_pts: egui::Rect::from_min_max(egui::Pos2::ZERO, egui::Pos2::ZERO),
            canvas: CanvasBuffer::new(DEFAULT_CANVAS_WIDTH, DEFAULT_CANVAS_HEIGHT),
            pan_px: egui::Vec2::ZERO,
            zoom: 1.0,
            last_drag_pos: None,
        }
    }

    pub fn panel_rect_px(&self) -> egui::Rect {
        self.panel_rect_pts * self.egui_ctx.pixels_per_point()
    }

    pub fn draw(&mut self, egui_ui: &mut egui::Ui, panel_texture_id: egui::TextureId) {
        CentralPanel::default().show(egui_ui, |egui_ui| {
            let available = egui_ui.available_size();
            let sized_texture = egui::load::SizedTexture::new(panel_texture_id, available);

            // Use only the drag sense so that we get inputs as soon as they occur.
            let response =
                egui_ui.add(egui::Image::from_texture(sized_texture).sense(egui::Sense::DRAG));
            self.panel_rect_pts = response.rect;

            self.handle_input(egui_ui, &response);
        });
    }

    fn handle_input(&mut self, egui_ui: &mut egui::Ui, response: &egui::Response) {
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
        if scroll_delta != egui::Vec2::ZERO {
            let pan_delta = scroll_delta / self.zoom;
            self.pan_px += pan_delta;
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

            let panel_rect_px = self.panel_rect_px();
            let panel_center = egui::vec2(panel_rect_px.width(), panel_rect_px.height()) * 0.5;
            let cursor_offset = cursor_pos_px - panel_center;

            let zoom_factor_delta = 1.0 / new_zoom - 1.0 / old_zoom;
            let pan_delta = cursor_offset * zoom_factor_delta;

            self.pan_px += pan_delta;
            self.zoom = new_zoom;
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
                1,
                PixelColor::BLACK,
            );
            self.last_drag_pos = Some(pos_px);

            // testing lines
            // self.draw_line(
            //     PxCoords { x: 1, y: 1 },
            //     PxCoords { x: 1, y: 1 },
            //     1,
            //     PixelColor::BLACK,
            // );
            // self.draw_line(
            //     PxCoords { x: 10, y: 10 },
            //     PxCoords { x: 50, y: 50 },
            //     1,
            //     PixelColor::BLACK,
            // );
            // self.draw_line(
            //     PxCoords { x: 10, y: 10 },
            //     PxCoords { x: 25, y: 50 },
            //     1,
            //     PixelColor::BLACK,
            // );
            // self.draw_line(
            //     PxCoords { x: 10, y: 10 },
            //     PxCoords { x: 50, y: 25 },
            //     1,
            //     PixelColor::BLACK,
            // );
        }
    }

    /// Calculates the current normalization factors for the canvas panel. These factors can be
    /// multiplied with the starting uvs (numbers ranging linearly from 0 to 1 across the
    /// canvas panel coordinates) to normalize the points to a canvas with the proper aspect ratio
    /// and a 1:1 canvas pixel:screen pixel scale.
    /// Note that these factors do not account for zooming or panning.
    pub fn get_normalization_factors(&self) -> [f32; 2] {
        let panel_rect_px = self.panel_rect_pts * self.egui_ctx.pixels_per_point();

        [
            panel_rect_px.width() / self.canvas.width() as f32,
            panel_rect_px.height() / self.canvas.height() as f32,
        ]
    }
}
