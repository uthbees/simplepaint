use crate::app_state::AppState;
use crate::app_state::canvas::canvas_pos::CanvasPos;
use crate::display::scrollbar::{
    SCROLLBAR_TRACK_COLOR, SCROLLBAR_TRACK_SIZE_PT, Scrollbar, ScrollbarAxis,
};
use egui::{CentralPanel, Ui, Vec2};

pub struct CanvasPanel {
    egui_ctx: egui::Context,
    canvas_view_texture_id: Option<egui::TextureId>,
    scrollbars: CanvasPanelScrollbars,
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
            canvas_view_texture_id: None,
            scrollbars: CanvasPanelScrollbars {
                horizontal: Scrollbar::new(ScrollbarAxis::Horizontal),
                vertical: Scrollbar::new(ScrollbarAxis::Vertical),
            },
            last_drag_pos: None,
        }
    }

    pub fn set_canvas_view_texture_id(&mut self, texture_id: egui::TextureId) {
        self.canvas_view_texture_id = Some(texture_id);
    }

    pub fn draw(&mut self, ui: &mut Ui, app_state: &mut AppState) {
        CentralPanel::default().show(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            let panel_rect = ui.available_rect_before_wrap();

            let canvas_size = egui::vec2(
                (panel_rect.width() - SCROLLBAR_TRACK_SIZE_PT).max(1.0),
                (panel_rect.height() - SCROLLBAR_TRACK_SIZE_PT).max(1.0),
            );

            let (canvas_view_rect, response) =
                ui.allocate_exact_size(canvas_size, egui::Sense::DRAG);
            app_state.canvas_view.dimensions =
                (canvas_view_rect * self.egui_ctx.pixels_per_point()).into();

            if let Some(canvas_view_texture_id) = self.canvas_view_texture_id {
                ui.painter().image(
                    canvas_view_texture_id,
                    canvas_view_rect,
                    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                    egui::Color32::WHITE,
                );
            }
            self.handle_canvas_view_input(ui, app_state, &response);

            self.draw_scrollbars(ui, app_state, panel_rect, canvas_view_rect);
        });
    }

    #[allow(clippy::similar_names)]
    fn draw_scrollbars(
        &mut self,
        ui: &mut Ui,
        app_state: &mut AppState,
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

        let canvas_view_dim = app_state.canvas_view.dimensions;
        let canvas_dim = app_state.canvas.dimensions();

        let zoom = app_state.canvas_view.zoom();
        let pan_px = app_state.canvas_view.pan_px();

        // Calculate the viewport fractions, first just with respect to the canvas, then also
        // including the off-canvas scroll buffer.
        let canvas_x_viewport_fraction =
            canvas_view_dim.width() as f32 / (canvas_dim.width() as f32 * zoom);
        let full_x_viewport_fraction =
            canvas_x_viewport_fraction / (canvas_x_viewport_fraction + 1.0);
        let canvas_y_viewport_fraction =
            canvas_view_dim.height() as f32 / (canvas_dim.height() as f32 * zoom);
        let full_y_viewport_fraction =
            canvas_y_viewport_fraction / (canvas_y_viewport_fraction + 1.0);

        let updated_horizontal_pan = self.scrollbars.horizontal.draw(
            ui,
            horizontal_track,
            full_x_viewport_fraction,
            -pan_px.x / canvas_dim.width() as f32 + 0.5,
        );
        let updated_vertical_pan = self.scrollbars.vertical.draw(
            ui,
            vertical_track,
            full_y_viewport_fraction,
            -pan_px.y / canvas_dim.height() as f32 + 0.5,
        );

        if let Some(updated_horizontal_pan) = updated_horizontal_pan {
            app_state.canvas_view.set_pan_px_x(
                (0.5 - updated_horizontal_pan) * canvas_dim.width() as f32,
                &app_state.canvas,
            );
        }
        if let Some(updated_vertical_pan) = updated_vertical_pan {
            app_state.canvas_view.set_pan_px_y(
                (0.5 - updated_vertical_pan) * canvas_dim.height() as f32,
                &app_state.canvas,
            );
        }

        ui.painter().rect_filled(corner, 0.0, SCROLLBAR_TRACK_COLOR);
    }

    fn handle_canvas_view_input(
        &mut self,
        ui: &mut Ui,
        app_state: &mut AppState,
        response: &egui::Response,
    ) {
        if response.hovered() {
            self.handle_scrolled(ui, app_state, response);
        }

        if response.drag_stopped() {
            self.last_drag_pos = None;
        } else if response.dragged() {
            self.handle_dragged(ui, app_state, response);
        }
    }

    fn handle_scrolled(&mut self, ui: &Ui, app_state: &mut AppState, response: &egui::Response) {
        let scroll_delta = ui.input(egui::InputState::translation_delta);
        if scroll_delta != Vec2::ZERO {
            let pan_delta = scroll_delta / app_state.canvas_view.zoom();
            app_state.canvas_view.set_pan_px(
                app_state.canvas_view.pan_px() + pan_delta,
                &app_state.canvas,
            );
        }

        let zoom_delta = ui.input(egui::InputState::zoom_delta);
        #[allow(clippy::float_cmp)]
        if zoom_delta != 1.0 {
            let old_zoom = app_state.canvas_view.zoom();
            let new_zoom = (old_zoom * zoom_delta).clamp(0.1, 10.0);

            let px_per_pt = self.egui_ctx.pixels_per_point();
            let cursor_pos_px = (ui
                .input(|i| i.pointer.hover_pos())
                .unwrap_or_else(|| response.rect.center())
                - response.rect.min)
                * px_per_pt;

            let canvas_view_dim = app_state.canvas_view.dimensions;
            let view_center = egui::vec2(
                canvas_view_dim.width() as f32,
                canvas_view_dim.height() as f32,
            ) * 0.5;
            let cursor_offset = cursor_pos_px - view_center;

            let zoom_factor_delta = 1.0 / new_zoom - 1.0 / old_zoom;
            let pan_delta = cursor_offset * zoom_factor_delta;

            app_state.canvas_view.set_pan_and_zoom(
                app_state.canvas_view.pan_px() + pan_delta,
                new_zoom,
                &app_state.canvas,
            );
        }
    }

    fn handle_dragged(
        &mut self,
        ui: &mut Ui,
        app_state: &mut AppState,
        canvas_response: &egui::Response,
    ) {
        if let Some(cursor_pos) = ui.input(|i| i.pointer.interact_pos()) {
            let rect = canvas_response.rect;

            let relative_pos = cursor_pos - rect.min;
            let pos_px = CanvasPos::from_panel_pos(relative_pos, rect, app_state);

            app_state.canvas.draw_line(
                pos_px,
                self.last_drag_pos.unwrap_or(pos_px),
                1.0,
                app_state.foreground_color,
            );
            self.last_drag_pos = Some(pos_px);
        }
    }
}
