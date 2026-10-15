pub mod canvas_panel;
mod scrollbar;
pub mod side_panel;
mod ui_config;

use canvas_panel::CanvasPanel;
use side_panel::draw_side_panel;
use ui_config::UiConfig;

pub struct Ui {
    pub canvas_panel: CanvasPanel,
    ui_config: UiConfig,
}

impl Ui {
    pub fn new(egui_ctx: egui::Context) -> Self {
        Self {
            canvas_panel: CanvasPanel::new(egui_ctx),
            ui_config: UiConfig::new(),
        }
    }

    pub fn draw(&mut self, egui_ui: &mut egui::Ui) {
        draw_side_panel(egui_ui, &mut self.ui_config);
        self.canvas_panel.draw(egui_ui, &self.ui_config);
    }
}
