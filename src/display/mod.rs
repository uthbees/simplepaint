pub mod canvas_panel;
mod left_panel;
mod right_panel;
mod scrollbar;

use crate::app_state::AppState;
use canvas_panel::CanvasPanel;
use egui::Ui;
use left_panel::LeftPanel;
use right_panel::RightPanel;

pub struct Display {
    canvas_panel: CanvasPanel,
}

impl Display {
    pub fn new(egui_ctx: egui::Context) -> Self {
        Self {
            canvas_panel: CanvasPanel::new(egui_ctx),
        }
    }

    pub fn draw(&mut self, ui: &mut Ui, app_state: &mut AppState) {
        LeftPanel::draw(ui, app_state);
        RightPanel::draw(ui, app_state);
        self.canvas_panel.draw(ui, app_state);
    }

    pub fn register_canvas_view_texture_id(&mut self, texture_id: egui::TextureId) {
        self.canvas_panel.set_canvas_view_texture_id(texture_id);
    }
}
