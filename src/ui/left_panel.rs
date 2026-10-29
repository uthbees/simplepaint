use crate::ui::ui_state::UiState;
use egui::color_picker::{Alpha, color_picker_hsva_2d, show_color_at};
use egui::ecolor::Hsva;
use egui::{
    Color32, Id, Panel, Popup, PopupCloseBehavior, Rect, Rgba, RichText, Sense, SetOpenCommand,
    StrokeKind, Ui,
};

const LARGE_SWATCH_SIZE: f32 = 28.0;
const SMALL_SWATCH_SIZE: f32 = 16.0;
const BACKGROUND_COLOR_SWATCH_OFFSET: f32 = 15.0;
const COLOR_SLIDER_WIDTH: f32 = 275.0;

#[derive(Copy, Clone)]
enum MainColorPicker {
    Foreground,
    Background,
}

pub struct LeftPanel {}

impl LeftPanel {
    /// Draws the left-hand side panel containing basic tool/color indicators.
    pub fn draw(ui: &mut Ui, ui_state: &mut UiState) {
        Panel::left("left_panel")
            .exact_size(230.0)
            .resizable(false)
            .show(ui, |ui| {
                Panel::bottom("color_panel")
                    .exact_size(200.0)
                    .resizable(false)
                    .show(ui, |ui| {
                        Self::draw_colors(ui, ui_state);
                    });
                Panel::top("tools_panel").resizable(false).show(ui, |ui| {
                    Self::draw_tools(ui, ui_state);
                });
            });
    }

    #[allow(unused_variables)]
    fn draw_tools(ui: &mut Ui, ui_state: &mut UiState) {
        ui.add_space(8.0);
        ui.heading("SimplePaint");
        ui.separator();

        ui.add_space(8.0);
        ui.label(RichText::new("Tool").strong());
        ui.label("Paintbrush");
    }

    fn draw_colors(ui: &mut Ui, ui_state: &mut UiState) {
        ui.add_space(4.0);
        Self::draw_fg_bg_colors(ui, ui_state);

        ui.add_space(8.0);
        ui.separator();
        Self::draw_recent_colors(ui, ui_state);

        ui.add_space(4.0);
        ui.separator();
        Self::draw_preset_colors(ui, ui_state);
    }

    /// Draws two overlapping color swatch boxes: foreground (top-left, in front)
    /// and background (bottom-right, behind). Clicking either opens a color picker
    /// popup; releasing the mouse with a color picker open or closing the popup saves
    /// the color to the list of 10 most recently used colors.
    fn draw_fg_bg_colors(ui: &mut Ui, ui_state: &mut UiState) {
        let box_size = egui::vec2(LARGE_SWATCH_SIZE, LARGE_SWATCH_SIZE);
        let total_size = egui::vec2(
            LARGE_SWATCH_SIZE + BACKGROUND_COLOR_SWATCH_OFFSET,
            LARGE_SWATCH_SIZE + BACKGROUND_COLOR_SWATCH_OFFSET,
        );

        let (total_rect, _) = ui.allocate_exact_size(total_size, Sense::hover());

        // Background color box - offset to bottom-right.
        let bg_rect = Rect::from_min_size(
            total_rect.min
                + egui::vec2(
                    BACKGROUND_COLOR_SWATCH_OFFSET,
                    BACKGROUND_COLOR_SWATCH_OFFSET,
                ),
            box_size,
        );
        let bg_id = ui.make_persistent_id("bg_color_btn");

        // Foreground color box - at top-left origin, overlapping the background box.
        let fg_rect = Rect::from_min_size(total_rect.min, box_size);
        let fg_id = ui.make_persistent_id("fg_color_btn");

        // Register interaction: background first (lower priority), foreground second
        // (higher priority in the overlap region).
        let bg_response = ui.interact(bg_rect, bg_id, Sense::click());
        let fg_response = ui.interact(fg_rect, fg_id, Sense::click());

        // Draw background swatch (behind).
        Self::draw_large_swatch(ui, bg_rect, ui_state.background_color, &bg_response);
        // Draw foreground swatch (in front).
        Self::draw_large_swatch(ui, fg_rect, ui_state.foreground_color, &fg_response);

        // Popup for the foreground color picker.
        let fg_popup_id = ui.make_persistent_id("fg_color_popup");
        Self::draw_main_color_picker_popup(
            ui,
            &fg_response,
            fg_popup_id,
            MainColorPicker::Foreground,
            ui_state,
        );

        // Popup for the background color picker.
        let bg_popup_id = ui.make_persistent_id("bg_color_popup");
        Self::draw_main_color_picker_popup(
            ui,
            &bg_response,
            bg_popup_id,
            MainColorPicker::Background,
            ui_state,
        );
    }

    fn draw_large_swatch(ui: &Ui, rect: Rect, color: Rgba, response: &egui::Response) {
        if !ui.is_rect_visible(rect) {
            return;
        }

        let visuals = ui.style().interact(response);
        let draw_rect = rect.expand(visuals.expansion);
        show_color_at(ui.painter(), Color32::from(color), draw_rect.shrink(1.0));
        ui.painter()
            .rect_stroke(draw_rect, 2.0, (1.0, visuals.bg_fill), StrokeKind::Inside);
    }

    /// Draws a color picker popup that opens when `trigger_response` is clicked.
    /// Saves the chosen color to `recent_colors` whenever mouse is released with the picker open,
    /// or when the popup closes.
    fn draw_main_color_picker_popup(
        ui: &mut Ui,
        trigger_response: &egui::Response,
        popup_id: Id,
        color_picker: MainColorPicker,
        ui_state: &mut UiState,
    ) {
        let color = match color_picker {
            MainColorPicker::Foreground => &mut ui_state.foreground_color,

            MainColorPicker::Background => &mut ui_state.background_color,
        };

        let was_open_before = Popup::is_id_open(ui.ctx(), popup_id);

        // Use a stable cache id to preserve hue when editing grays.
        let cache_id = popup_id.with("hsva_cache");
        let mut hsva: Hsva = ui
            .data(|d| d.get_temp::<Hsva>(cache_id))
            .unwrap_or_else(|| Hsva::from(*color));

        // Sync hsva from rgba when the popup is closed (picks up external color changes).
        if !was_open_before {
            hsva = Hsva::from(*color);
        }

        // Toggle popup open/closed on click.
        Popup::from_response(trigger_response)
            .id(popup_id)
            .open_memory(trigger_response.clicked().then_some(SetOpenCommand::Toggle))
            .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.spacing_mut().slider_width = COLOR_SLIDER_WIDTH;
                color_picker_hsva_2d(ui, &mut hsva, Alpha::OnlyBlend);
                *color = Rgba::from(hsva);
            });

        let is_open_after = Popup::is_id_open(ui.ctx(), popup_id);

        // Whenever the mouse is released with the color picker open, or when the popup closes,
        // save the color to the list of 10 most recently used colors.
        let mouse_released = ui.input(|i| i.pointer.any_released());
        let color = *color;
        if was_open_before && (mouse_released || !is_open_after) {
            ui_state.push_recent_color(color);
        }

        // Persist the hsva so hue is stable across frames.
        ui.data_mut(|d| d.insert_temp(cache_id, hsva));
    }

    fn draw_recent_colors(ui: &mut Ui, ui_state: &mut UiState) {
        ui.add_space(4.0);
        ui.label(RichText::new("Recent").strong());

        Self::draw_small_swatch_row(
            ui,
            ui_state.get_recent_colors(),
            |_, response, (_, color)| {
                if response.clicked() {
                    ui_state.foreground_color = color;
                } else if response.secondary_clicked() {
                    ui_state.background_color = color;
                }
                response.on_hover_text(
                    "Left click to set as foreground, right click to set as background.",
                );
            },
        );
    }

    fn draw_preset_colors(ui: &mut Ui, ui_state: &mut UiState) {
        ui.add_space(4.0);
        ui.label(RichText::new("Presets").strong());

        let palette_colors = ui_state.palette_colors;
        Self::draw_small_swatch_row(
            ui,
            palette_colors.iter().map(|&color| Some(color)),
            |ui, response, (index, color)| {
                let popup_id = ui.make_persistent_id(("preset_edit", index));
                if response.double_clicked() {
                    Popup::open_id(ui.ctx(), popup_id);
                } else if response.secondary_clicked() {
                    ui_state.background_color = color;
                    ui_state.push_recent_color(color);
                } else if response.clicked() {
                    ui_state.foreground_color = color;
                    ui_state.push_recent_color(color);
                }

                let open = Popup::is_id_open(ui.ctx(), popup_id);
                if open {
                    let cache_id = popup_id.with("preset_hsva");
                    let mut hsva: Hsva = ui
                        .data(|d| d.get_temp::<Hsva>(cache_id))
                        .unwrap_or_else(|| Hsva::from(color));

                    Popup::from_response(&response)
                        .id(popup_id)
                        .open_memory(None)
                        .close_behavior(PopupCloseBehavior::CloseOnClickOutside)
                        .show(|ui| {
                            ui.spacing_mut().slider_width = COLOR_SLIDER_WIDTH;
                            color_picker_hsva_2d(ui, &mut hsva, Alpha::OnlyBlend);
                            ui_state.palette_colors[index] = Rgba::from(hsva);
                        });

                    ui.data_mut(|d| d.insert_temp(cache_id, hsva));

                    if ui.input(|i| i.pointer.any_released()) {
                        ui_state.foreground_color = color;
                        ui_state.push_recent_color(color);
                    }
                }

                let tooltip = "Left click to set as foreground, right click to set as background, double click to edit.";
                response.on_hover_text(tooltip);
            },
        );
    }

    fn draw_small_swatch_row<ColorsIter>(
        ui: &mut Ui,
        colors: ColorsIter,
        mut on_draw_swatch: impl FnMut(&mut Ui, egui::Response, (usize, Rgba)),
    ) where
        ColorsIter: IntoIterator<Item = Option<Rgba>>,
    {
        let swatch_size = egui::vec2(SMALL_SWATCH_SIZE, SMALL_SWATCH_SIZE);
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);

        ui.horizontal_wrapped(|ui| {
            for (index, color) in colors.into_iter().enumerate() {
                if let Some(color) = color {
                    let (rect, response) = ui.allocate_exact_size(swatch_size, Sense::click());
                    if ui.is_rect_visible(rect) {
                        ui.painter().rect_filled(rect, 2.0, Color32::from(color));
                        ui.painter().rect_stroke(
                            rect,
                            2.0,
                            egui::Stroke::new(
                                1.0,
                                if response.hovered() {
                                    Color32::WHITE
                                } else {
                                    Color32::GRAY
                                },
                            ),
                            StrokeKind::Inside,
                        );
                    }

                    on_draw_swatch(ui, response, (index, color));
                } else {
                    let (rect, _) = ui.allocate_exact_size(swatch_size, Sense::hover());
                    if ui.is_rect_visible(rect) {
                        ui.painter().rect_stroke(
                            rect,
                            2.0,
                            egui::Stroke::new(1.0, Color32::from_gray(60)),
                            StrokeKind::Inside,
                        );
                    }
                }
            }
        });
    }
}
