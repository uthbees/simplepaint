pub mod canvas_panel;
mod left_panel;
mod right_panel;
mod scrollbar;
pub mod ui_config;

use canvas_panel::CanvasPanel;
use egui::Ui;
use left_panel::LeftPanel;
use right_panel::RightPanel;
use ui_config::UiConfig;

pub struct UiManager {
    pub canvas_panel: CanvasPanel,
    pub config: UiConfig,
}

impl UiManager {
    pub fn new(egui_ctx: egui::Context) -> Self {
        Self {
            canvas_panel: CanvasPanel::new(egui_ctx),
            config: UiConfig::new(),
        }
    }

    pub fn draw(&mut self, ui: &mut Ui) {
        LeftPanel::draw(ui, &mut self.config);
        RightPanel::draw(ui, &mut self.config);
        self.canvas_panel.draw(ui, &self.config);
    }
}
