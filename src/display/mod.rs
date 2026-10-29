pub mod canvas_panel;
mod left_panel;
mod right_panel;
mod scrollbar;

use crate::app_state::AppState;
use crate::gpu::CanvasGpu;
use canvas_panel::CanvasPanel;
use egui::Ui;
use left_panel::LeftPanel;
use right_panel::RightPanel;

pub struct Display {
    canvas_panel: CanvasPanel,
    right_panel: RightPanel,
}

impl Display {
    pub fn new(egui_ctx: egui::Context) -> Self {
        Self {
            canvas_panel: CanvasPanel::new(egui_ctx),
            right_panel: RightPanel::new(),
        }
    }

    pub fn draw(&mut self, ui: &mut Ui, app_state: &mut AppState, canvas_gpu: &mut CanvasGpu) {
        LeftPanel::draw(ui, app_state);
        self.right_panel.draw(ui, app_state, canvas_gpu);
        self.canvas_panel.draw(ui, app_state);
    }

    pub fn register_canvas_view_texture_id(&mut self, texture_id: egui::TextureId) {
        self.canvas_panel.set_canvas_view_texture_id(texture_id);
    }
}
