use egui::Rgba;
use std::collections::VecDeque;

pub struct EnabledLayersConfig {
    pub states: Vec<bool>,
    // Indicates whether there have been any changes here since the last render.
    pub dirty: bool,
}

pub struct UiState {
    pub foreground_color: Rgba,
    pub background_color: Rgba,

    recent_colors: VecDeque<Option<Rgba>>,
    pub palette_colors: [Rgba; 10],
    pub selected_layer_idx: Option<usize>,
    pub enabled_layers: EnabledLayersConfig,
}

impl UiState {
    pub fn new() -> UiState {
        let mut recent_colors = VecDeque::with_capacity(10);
        recent_colors.resize(10, None);

        UiState {
            foreground_color: Rgba::BLACK,
            background_color: Rgba::WHITE,
            recent_colors,
            palette_colors: [
                Rgba::BLACK,
                Rgba::WHITE,
                Rgba::from_rgb(1.0, 0.0, 0.0),
                Rgba::from_rgb(0.0, 1.0, 0.0),
                Rgba::from_rgb(0.0, 0.0, 1.0),
                Rgba::from_rgb(1.0, 1.0, 0.0),
                Rgba::from_rgb(1.0, 0.0, 1.0),
                Rgba::from_rgb(0.0, 1.0, 1.0),
                Rgba::from_rgb(1.0, 0.5, 0.0),
                Rgba::from_rgb(0.5, 0.0, 0.5),
            ],
            selected_layer_idx: Some(0),
            enabled_layers: EnabledLayersConfig {
                states: vec![true],
                dirty: false,
            },
        }
    }

    /// Adds a color to the front of the recent colors list, deduplicating and
    /// capping at 10.
    pub fn push_recent_color(&mut self, new_color: Rgba) {
        // Remove any existing recent color that is within epsilon of the new color.
        let epsilon = 0.00001;
        self.recent_colors.retain(|recent| {
            match recent {
                Some(existing) => {
                    let a = existing.to_array();
                    let b = new_color.to_array();

                    // Check if all RGBA channels are within epsilon of each other
                    let is_close = a
                        .iter()
                        .zip(b.iter())
                        .all(|(x, y)| (x - y).abs() <= epsilon);

                    // Keep items that are NOT close (i.e., remove duplicates)
                    !is_close
                }
                None => true,
            }
        });

        self.recent_colors.insert(0, Some(new_color));
        self.recent_colors.resize(10, None);
    }

    pub fn get_recent_colors(&self) -> Vec<Option<Rgba>> {
        self.recent_colors.iter().copied().collect::<Vec<_>>()
    }
}
