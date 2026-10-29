use simplepaint::App;
use winit::event_loop::EventLoop;

fn main() {
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    let mut app_runner = App::default();
    event_loop
        .run_app(&mut app_runner)
        .expect("Event loop terminated with an error");
}
