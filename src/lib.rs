pub mod game;
pub mod player;

use game::Game;
use pyxl::input::Input;
use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub async fn run() {
    env_logger::init();
    //std::env::set_var("RUST_BACKTRACE", "1");

    let event_loop = EventLoop::new();
    let scale = 2;
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(scale * 427, scale * 240))
        .with_min_inner_size(PhysicalSize::new(427, 240))
        .build(&event_loop)
        .unwrap();

    let mut game = Game::new(&window).await.unwrap();
    let mut inp = Input::default();
    let mut previous_time = std::time::Instant::now();
    let mut delta = 1.0 / 60.0;

    let mut gilrs = gilrs::Gilrs::new().unwrap();
    let mut keyboard_inputs = vec![];

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::KeyboardInput { input, .. } => keyboard_inputs.push(*input),
            WindowEvent::Resized(physical_size) => {
                game.renderer.resize(*physical_size);
            }
            WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                game.renderer.resize(**new_inner_size);
            }
            _ => {}
        },
        Event::RedrawRequested(window_id) if window_id == window.id() => game.draw(),
        Event::MainEventsCleared => {
            let current_time = std::time::Instant::now();
            delta = (current_time - previous_time).as_nanos() as f32 / 1_000_000_000.0;
            let mut controller_inputs: Vec<gilrs::Event> = vec![];
            while let Some(event) = gilrs.next_event() {
                controller_inputs.push(event);
            }
            inp.update(&mut controller_inputs, &mut keyboard_inputs);
            keyboard_inputs.clear();
            game.update(&inp, delta);
            window.request_redraw();
            previous_time = std::time::Instant::now();
        }
        _ => {}
    });
}
