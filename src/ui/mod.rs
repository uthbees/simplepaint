pub mod canvas_panel;
pub mod side_panel;

use crate::ui::canvas_panel::CanvasPanel;
use crate::ui::side_panel::draw_side_panel;

pub struct Ui {
    pub canvas_panel: CanvasPanel,
}

impl Ui {
    pub fn new(egui_ctx: egui::Context) -> Self {
        Self {
            canvas_panel: CanvasPanel::new(egui_ctx),
        }
    }

    pub fn draw(&mut self, egui_ui: &mut egui::Ui, canvas_panel_texture_id: egui::TextureId) {
        draw_side_panel(egui_ui, egui::Color32::BLACK);
        self.canvas_panel.draw(egui_ui, canvas_panel_texture_id);
    }
}
