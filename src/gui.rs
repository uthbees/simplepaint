use egui::{CentralPanel, Color32, Panel, RichText, Ui};

/// Width of the left tool panel in logical points.
pub const SIDE_PANEL_WIDTH: f32 = 160.0;

/// Draws the left-hand side panel containing basic tool/color indicators.
pub fn draw_side_panel(ui: &mut Ui, selected_color: Color32) {
    Panel::left("tool_panel")
        .default_size(SIDE_PANEL_WIDTH)
        .resizable(false)
        .show(ui, |ui| {
            draw_tool_panel_contents(ui, selected_color);
        });
}

fn draw_tool_panel_contents(ui: &mut Ui, selected_color: Color32) {
    ui.add_space(8.0);
    ui.heading("SimplePaint");
    ui.separator();

    ui.add_space(8.0);
    ui.label(RichText::new("Tool").strong());
    ui.label("Pencil");

    ui.add_space(12.0);
    ui.label(RichText::new("Color").strong());

    // Color swatch showing the currently active drawing color.
    let swatch_size = egui::Vec2::splat(32.0);
    let (rect, _) = ui.allocate_exact_size(swatch_size, egui::Sense::hover());
    ui.painter().rect_filled(rect, 4.0, selected_color);
    ui.label(format!(
        "#{:02X}{:02X}{:02X}",
        selected_color.r(),
        selected_color.g(),
        selected_color.b()
    ));
}

/// Draws the central canvas panel and returns the `egui::Response` for the
/// image widget, along with the top-left origin of the image in logical points.
///
/// Returns `None` when no texture has been registered yet.
pub fn draw_canvas_panel(
    ui: &mut Ui,
    texture_id: Option<egui::TextureId>,
    canvas_width: u32,
    canvas_height: u32,
) -> Option<egui::Response> {
    let mut canvas_response = None;

    CentralPanel::default().show(ui, |ui| {
        if let Some(tid) = texture_id {
            let available = ui.available_size();

            #[allow(clippy::cast_precision_loss)]
            let canvas_aspect_ratio = canvas_width as f32 / canvas_height as f32;
            let panel_aspect_ratio = available.x / available.y;

            // Fit the canvas inside the available panel area while preserving the pixel-exact aspect ratio.
            let display_size = if canvas_aspect_ratio > panel_aspect_ratio {
                egui::Vec2::new(available.x, available.x / canvas_aspect_ratio)
            } else {
                egui::Vec2::new(available.y * canvas_aspect_ratio, available.y)
            };

            let sized_texture = egui::load::SizedTexture::new(tid, display_size);

            let response = ui
                .add(egui::Image::from_texture(sized_texture).sense(egui::Sense::DRAG));
            canvas_response = Some(response);
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Initializing canvas...");
            });
        }
    });

    canvas_response
}
