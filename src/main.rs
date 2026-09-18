mod app;
mod canvas;
mod gui;
pub mod gpu;

use winit::event_loop::EventLoop;

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut application = app::App::new();
    event_loop
        .run_app(&mut application)
        .expect("Event loop terminated with an error");
}
