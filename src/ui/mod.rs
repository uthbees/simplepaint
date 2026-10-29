pub mod canvas_panel;
mod left_panel;
mod scrollbar;
pub mod ui_config;

use canvas_panel::CanvasPanel;
use left_panel::LeftPanel;
use ui_config::UiConfig;

pub struct Ui {
    pub canvas_panel: CanvasPanel,
    pub config: UiConfig,
}

impl Ui {
    pub fn new(egui_ctx: egui::Context) -> Self {
        Self {
            canvas_panel: CanvasPanel::new(egui_ctx),
            config: UiConfig::new(),
        }
    }

    pub fn draw(&mut self, egui_ui: &mut egui::Ui) {
        LeftPanel::draw(egui_ui, &mut self.config);
        self.canvas_panel.draw(egui_ui, &self.config);
    }
}
