use crate::ui::ui_config::UiConfig;
use egui::color_picker::{Alpha, color_edit_button_rgba};
use egui::{Panel, RichText, Ui};

/// Draws the left-hand side panel containing basic tool/color indicators.
pub fn draw_side_panel(egui_ui: &mut Ui, ui_config: &mut UiConfig) {
    Panel::left("left_panel")
        .exact_size(200.0)
        .resizable(false)
        .show(egui_ui, |egui_ui| {
            Panel::bottom("color_panel")
                .exact_size(200.0)
                .resizable(false)
                .show(egui_ui, |egui_ui| {
                    color_edit_button_rgba(
                        egui_ui,
                        &mut ui_config.selected_color,
                        Alpha::OnlyBlend,
                    );
                });
            Panel::top("tools_panel")
                .resizable(false)
                .show(egui_ui, |egui_ui| {
                    draw_tools(egui_ui);
                });
        });
}

fn draw_tools(egui_ui: &mut Ui) {
    egui_ui.add_space(8.0);
    egui_ui.heading("SimplePaint");
    egui_ui.separator();

    egui_ui.add_space(8.0);
    egui_ui.label(RichText::new("Tool").strong());
    egui_ui.label("Pencil");
}
