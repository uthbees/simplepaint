mod canvas;
mod gpu;
mod ui;

use std::num::NonZeroU32;
use std::sync::Arc;

use crate::gpu::CanvasGpu;
use crate::ui::Ui;
use egui::ViewportId;
use egui_wgpu::winit::Painter;
use egui_wgpu::{RenderState, WgpuConfiguration};
use egui_winit::State as EguiWinitState;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

const WINDOW_TITLE: &str = "SimplePaint";

#[derive(Default)]
pub struct App {
    /// `None` until `resumed()` fires and we have a window.
    inner: Option<AppState>,
}

struct AppState {
    window: Arc<Window>,
    painter: Painter,
    egui_state: EguiWinitState,
    render_state: RenderState,
    canvas_gpu: CanvasGpu,
    ui: Ui,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Initialize the window only once.
        if self.inner.is_some() {
            return;
        }

        let window_attrs = Window::default_attributes()
            .with_title(WINDOW_TITLE)
            .with_visible(false);
        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("Failed to create window"),
        );
        window.set_min_inner_size(Some(LogicalSize::new(300.0, 300.0)));

        // Build egui context and winit state.
        let viewport_id = ViewportId::ROOT;
        let egui_ctx = egui::Context::default();
        let egui_state = EguiWinitState::new(
            egui_ctx.clone(),
            viewport_id,
            event_loop,
            Some(window.scale_factor() as f32),
            window.theme(),
            None,
        );

        // Initialize the egui-wgpu Painter. (This creates a wgpu instance.)
        let mut painter = pollster::block_on(Painter::new(
            egui_ctx.clone(),
            WgpuConfiguration::default(),
            false,
            egui_wgpu::RendererOptions::default(),
        ));

        // Attach the window surface and initialize the device/queue.
        pollster::block_on(painter.set_window(viewport_id, Some(Arc::clone(&window))))
            .expect("Failed to set painter window");

        let render_state = painter
            .render_state()
            .expect("RenderState should be available after set_window");

        let ui = Ui::new(egui_ctx);

        // Upload the initial white canvas to a wgpu texture.
        let canvas = &ui.canvas_panel.canvas;
        let canvas_gpu = CanvasGpu::new(
            &render_state,
            canvas.width(),
            canvas.height(),
            canvas.as_bytes(),
        );

        self.inner = Some(AppState {
            window,
            painter,
            egui_state,
            render_state,
            canvas_gpu,
            ui,
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(app_state) = self.inner.as_mut() else {
            return;
        };

        app_state.window_event(event_loop, &event);
    }
}

impl AppState {
    fn window_event(&mut self, event_loop: &ActiveEventLoop, event: &WindowEvent) {
        match event {
            WindowEvent::RedrawRequested => {
                self.render_frame(event_loop);
            }
            WindowEvent::Resized(new_size) => {
                if let (Some(w), Some(h)) = (
                    NonZeroU32::new(new_size.width),
                    NonZeroU32::new(new_size.height),
                ) {
                    self.painter.on_window_resized(ViewportId::ROOT, w, h);
                    self.render_frame(event_loop);
                }
            }
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            _ => {
                // Forward all other events to egui.
                let egui_response = self.egui_state.on_window_event(&self.window, event);
                if egui_response.repaint {
                    self.window.request_redraw();
                }
            }
        }
    }

    pub fn render_frame(&mut self, event_loop: &ActiveEventLoop) {
        let raw_input = self.egui_state.take_egui_input(&self.window);
        let egui_ctx = self.egui_state.egui_ctx().clone();

        let full_output = egui_ctx.run_ui(raw_input, |egui_ui| {
            self.ui.draw(egui_ui, self.canvas_gpu.panel_texture_id);
        });

        let pixels_per_point = egui_ctx.pixels_per_point();

        let canvas_panel_rect_px = self.ui.canvas_panel.panel_rect_px();
        self.canvas_gpu.update_panel_size(
            &self.render_state,
            canvas_panel_rect_px.width(),
            canvas_panel_rect_px.height(),
        );

        // let inner_size = state.window.inner_size();
        // let physical_width =
        //     (inner_size.width as f32 - (SIDE_PANEL_WIDTH * pixels_per_point)).max(1.0);
        // let physical_height = (inner_size.height as f32).max(1.0);
        //
        // state.canvas_gpu.update_panel_size(
        //     &state.render_state,
        //     physical_width as u32,
        //     physical_height as u32,
        // );

        self.canvas_gpu
            .update_camera(&self.render_state.queue, &self.ui.canvas_panel);

        // Upload CPU canvas to GPU if it's changed.
        let canvas = &mut self.ui.canvas_panel.canvas;
        if canvas.dirty {
            self.canvas_gpu
                .upload_canvas_state(&self.render_state.queue, canvas);
            canvas.dirty = false;
        }

        // Render canvas texture with pan/zoom shader onto the canvas panel texture.
        self.canvas_gpu
            .render_canvas_panel_texture(&self.render_state.device, &self.render_state.queue);

        // Handle egui platform output (cursor, clipboard, etc.).
        self.egui_state.handle_platform_output_with_event_loop(
            &self.window,
            event_loop,
            full_output.platform_output,
        );

        let clipped_primitives = egui_ctx.tessellate(full_output.shapes, pixels_per_point);
        let mut textures_delta = full_output.textures_delta;

        self.painter.paint_and_update_textures(
            ViewportId::ROOT,
            pixels_per_point,
            [0.1, 0.1, 0.1, 1.0],
            &clipped_primitives,
            &mut textures_delta,
            vec![],
            &self.window,
        );

        // Make the window visible, if this is the first render.
        if !self.window.is_visible().unwrap_or(true) {
            self.window.set_visible(true);
        }
    }
}
