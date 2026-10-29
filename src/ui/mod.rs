pub mod canvas_panel;
mod left_panel;
mod right_panel;
mod scrollbar;
pub mod ui_state;

use canvas_panel::CanvasPanel;
use egui::Ui;
use left_panel::LeftPanel;
use right_panel::RightPanel;
use ui_state::UiState;

pub struct UiManager {
    pub canvas_panel: CanvasPanel,
    pub state: UiState,
}

impl UiManager {
    pub fn new(egui_ctx: egui::Context) -> Self {
        Self {
            canvas_panel: CanvasPanel::new(egui_ctx),
            state: UiState::new(),
        }
    }

    pub fn draw(&mut self, ui: &mut Ui) {
        LeftPanel::draw(ui, &mut self.state);
        RightPanel::draw(ui, &mut self.state);
        self.canvas_panel.draw(ui, &self.state);
    }
}
