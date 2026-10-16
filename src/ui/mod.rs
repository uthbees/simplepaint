pub mod canvas_panel;
mod left_panel;
mod scrollbar;
mod ui_config;

use canvas_panel::CanvasPanel;
use left_panel::LeftPanel;
use ui_config::UiConfig;

pub struct Ui {
    pub canvas_panel: CanvasPanel,
    left_panel: LeftPanel,
    config: UiConfig,
}

impl Ui {
    pub fn new(egui_ctx: egui::Context) -> Self {
        Self {
            canvas_panel: CanvasPanel::new(egui_ctx),
            left_panel: LeftPanel::new(),
            config: UiConfig::new(),
        }
    }

    pub fn draw(&mut self, egui_ui: &mut egui::Ui) {
        self.left_panel.draw(egui_ui, &mut self.config);
        self.canvas_panel.draw(egui_ui, &self.config);
    }
}
