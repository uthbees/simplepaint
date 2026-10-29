use crate::ui::ui_config::UiConfig;
use egui::Panel;

pub struct RightPanel {}

impl RightPanel {
    /// Draws the right-hand side panel containing layer controls.
    pub fn draw(egui_ui: &mut egui::Ui, ui_config: &mut UiConfig) {
        Panel::right("right_panel")
            .exact_size(250.0)
            .resizable(false)
            .show(egui_ui, |egui_ui| {
                Self::draw_layer_controls(egui_ui, ui_config);
            });
    }

    fn draw_layer_controls(egui_ui: &mut egui::Ui, ui_config: &mut UiConfig) {}
}
