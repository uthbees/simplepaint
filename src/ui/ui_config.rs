use egui::Rgba;

pub struct UiConfig {
    pub selected_color: Rgba,
}

impl UiConfig {
    pub fn new() -> UiConfig {
        UiConfig {
            selected_color: Rgba::BLACK,
        }
    }
}
