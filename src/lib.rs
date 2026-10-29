mod app_state;
mod display;
mod gpu;
pub mod utils;

use std::num::NonZeroU32;
use std::sync::Arc;

use app_state::AppState;
use display::Display;
use egui::ViewportId;
use egui_wgpu::winit::Painter;
use egui_wgpu::{RenderState, WgpuConfiguration};
use egui_winit::State as EguiWinitState;
use gpu::CanvasGpu;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

const WINDOW_TITLE: &str = "SimplePaint";

#[derive(Default)]
pub struct AppRunner {
    /// `None` until `resumed()` fires and we have a window.
    app: Option<App>,
}

struct App {
    window: Arc<Window>,
    painter: Painter,
    egui_state: EguiWinitState,
    render_state: RenderState,
    canvas_gpu: CanvasGpu,
    display: Display,
    app_state: AppState,
}

impl ApplicationHandler for AppRunner {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Initialize the window only once.
        if self.app.is_some() {
            return;
        }

        let app_state = AppState::new();

        let window_attrs = Window::default_attributes()
            .with_title(WINDOW_TITLE)
            .with_inner_size(LogicalSize::new(1200.0, 800.0))
            .with_min_inner_size(LogicalSize::new(800.0, 300.0))
            .with_visible(false);
        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("Failed to create window"),
        );

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

        // Upload the initial canvas to a wgpu texture.
        // This section will need updating if it becomes possible to start the app with
        // more than one layer.
        assert_eq!(app_state.canvas.layers().len(), 1);
        #[allow(clippy::cast_sign_loss)]
        let initial_layer = &app_state.canvas.layers()[0];
        let mut canvas_gpu = CanvasGpu::new(
            &render_state,
            app_state.canvas.dimensions(),
            app_state.canvas_view.dimensions,
            *initial_layer.id(),
            initial_layer.buffer_as_bytes(),
        );

        let mut display = Display::new(egui_ctx);
        display.register_canvas_view_texture_id(canvas_gpu.canvas_view().texture_id());

        self.app = Some(App {
            window,
            painter,
            egui_state,
            render_state,
            canvas_gpu,
            display,
            app_state,
        });
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(app) = self.app.as_mut() else {
            return;
        };

        app.window_event(event_loop, &event);
    }
}

impl App {
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

        let full_output = egui_ctx.run_ui(raw_input, |ui| {
            self.display.draw(ui, &mut self.app_state);
        });

        self.canvas_gpu
            .canvas_view()
            .update_size(&self.render_state, self.app_state.canvas_view.dimensions);
        self.canvas_gpu
            .camera()
            .update(&self.render_state.queue, &self.app_state);

        // Upload the CPU canvas to the GPU if it's changed.
        let canvas = &mut self.app_state.canvas;
        let canvas_dimensions = canvas.dimensions();
        canvas.layers_iter_mut().for_each(|layer| {
            if layer.dirty {
                self.canvas_gpu.layers().upload_layer(
                    &self.render_state.queue,
                    layer,
                    canvas_dimensions,
                );
                layer.dirty = false;
            }
        });

        // Render the canvas texture onto the canvas view texture with the pan/zoom shader
        // and layer configuration.
        self.canvas_gpu.render_canvas_view(
            &self.render_state.device,
            &self.render_state.queue,
            self.app_state.canvas.layers(),
        );

        // Handle egui platform output (cursor, clipboard, etc.).
        self.egui_state.handle_platform_output_with_event_loop(
            &self.window,
            event_loop,
            full_output.platform_output,
        );

        let pixels_per_point = egui_ctx.pixels_per_point();
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

        if egui_ctx.has_requested_repaint() {
            self.window.request_redraw();
        }

        // Make the window visible (if this is the first render).
        if !self.window.is_visible().unwrap_or(true) {
            self.window.set_visible(true);
        }
    }
}
