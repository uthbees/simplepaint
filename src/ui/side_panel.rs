use egui::{Color32, Panel, RichText, Ui};

/// Draws the left-hand side panel containing basic tool/color indicators.
pub fn draw_side_panel(egui_ui: &mut Ui, selected_color: Color32) {
    Panel::left("tool_panel")
        .default_size(160.0)
        .resizable(false)
        .show(egui_ui, |egui_ui| {
            draw_tools(egui_ui, selected_color);
        });
}

fn draw_tools(egui_ui: &mut Ui, selected_color: Color32) {
    egui_ui.add_space(8.0);
    egui_ui.heading("SimplePaint");
    egui_ui.separator();

    egui_ui.add_space(8.0);
    egui_ui.label(RichText::new("Tool").strong());
    egui_ui.label("Pencil");

    egui_ui.add_space(12.0);
    egui_ui.label(RichText::new("Color").strong());

    // Color swatch showing the currently active drawing color.
    let swatch_size = egui::Vec2::splat(32.0);
    let (rect, _) = egui_ui.allocate_exact_size(swatch_size, egui::Sense::hover());
    egui_ui.painter().rect_filled(rect, 4.0, selected_color);
    egui_ui.label(format!(
        "#{:02X}{:02X}{:02X}",
        selected_color.r(),
        selected_color.g(),
        selected_color.b()
    ));
}
