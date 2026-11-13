mod canvas_panel;
mod left_panel;
pub mod menu_bar_panel;
mod right_panel;
mod scrollbar;
mod top_panel;

use crate::ActionRequestFlags;
use crate::app_state::AppState;
use crate::gpu::CanvasGpu;
use canvas_panel::CanvasPanel;
use egui::Ui;
use left_panel::LeftPanel;
use menu_bar_panel::MenuBarPanel;
use right_panel::RightPanel;
use top_panel::TopPanel;

#[allow(clippy::struct_field_names)]
pub struct Display {
    top_panel: TopPanel,
    right_panel: RightPanel,
    canvas_panel: CanvasPanel,
}

impl Display {
    pub fn new(egui_ctx: egui::Context, app_state: &AppState) -> Self {
        Self {
            top_panel: TopPanel::new(app_state),
            right_panel: RightPanel::new(),
            canvas_panel: CanvasPanel::new(egui_ctx),
        }
    }

    pub fn draw(
        &mut self,
        ui: &mut Ui,
        app_state: &mut AppState,
        canvas_gpu: &mut CanvasGpu,
        action_request_flags: &mut ActionRequestFlags,
    ) {
        MenuBarPanel::draw(ui, app_state, canvas_gpu, action_request_flags);
        self.top_panel.draw(ui, app_state, canvas_gpu);
        LeftPanel::draw(ui, app_state);
        self.right_panel.draw(ui, app_state, canvas_gpu);
        self.canvas_panel.draw(ui, app_state);
    }

    pub fn register_canvas_view_texture_id(&mut self, texture_id: egui::TextureId) {
        self.canvas_panel.set_canvas_view_texture_id(texture_id);
    }
}
