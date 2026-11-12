use crate::app_state::{AppState, MAX_ZOOM, MIN_ZOOM};
use egui::{DragValue, Frame, Panel, Ui, Vec2, vec2};
use egui_material_icons::icons;

pub struct TopPanel {}

impl TopPanel {
    pub fn draw(ui: &mut Ui, app_state: &mut AppState) {
        Panel::top("top_panel")
            .frame(Frame::new().inner_margin(10.0))
            .resizable(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    Self::draw_zoom_controls(ui, app_state);
                });
            });
    }

    fn draw_zoom_controls(ui: &mut Ui, app_state: &mut AppState) {
        ui.spacing_mut().item_spacing = Vec2 { x: 4.0, y: 0.0 };
        ui.label("Zoom:");

        let mut zoom_log = app_state.canvas_view.zoom().log2();
        let old_zoom_log = zoom_log;
        ui.add(
            DragValue::new(&mut zoom_log)
                .min_size(vec2(60.0, 0.0))
                .clip_text(true)
                .range(MIN_ZOOM.log2()..=MAX_ZOOM.log2())
                .speed(0.015)
                .update_while_editing(false)
                .custom_formatter(|log_val, _| {
                    format!("{:.0}%", 2_f32.powf(log_val as f32) * 100.0)
                })
                .custom_parser(|str_val| {
                    str_val
                        .trim_end_matches('%')
                        .parse::<f64>()
                        .ok()
                        .map(|pct_val| (pct_val / 100.0).log2())
                }),
        );
        #[allow(clippy::float_cmp)]
        if zoom_log != old_zoom_log {
            app_state
                .canvas_view
                .set_zoom(2_f32.powf(zoom_log), &app_state.canvas);
        }

        ui.spacing_mut().button_padding = Vec2 { x: 4.0, y: 0.0 };
        let response = ui.button(icons::ICON_ZOOM_IN);
        if response.clicked() {
            Self::zoom_to_next_step(app_state, &ZoomDirection::In);
        }
        response.on_hover_text("Zoom in");

        let response = ui.button(icons::ICON_ZOOM_OUT);
        if response.clicked() {
            Self::zoom_to_next_step(app_state, &ZoomDirection::Out);
        }
        response.on_hover_text("Zoom out");
    }

    fn zoom_to_next_step(app_state: &mut AppState, zoom_direction: &ZoomDirection) {
        let zoom_log = app_state.canvas_view.zoom().log2();

        let new_zoom_log = if (zoom_log - zoom_log.round()).abs() < 0.01 {
            let zoom_log_diff = match zoom_direction {
                ZoomDirection::In => 1.0,
                ZoomDirection::Out => -1.0,
            };
            zoom_log.round() + zoom_log_diff
        } else {
            match zoom_direction {
                ZoomDirection::In => zoom_log.ceil(),
                ZoomDirection::Out => zoom_log.floor(),
            }
        };

        app_state
            .canvas_view
            .set_zoom(2_f32.powf(new_zoom_log), &app_state.canvas);
    }
}

enum ZoomDirection {
    In,
    Out,
}
