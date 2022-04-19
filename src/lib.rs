use futures::{
    executor::{LocalPool, LocalSpawner},
    task::SpawnExt,
};
use mobile_entry_point::mobile_entry_point;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use wgpu::{Backends, Limits};
use wgpu_glyph::{GlyphBrushBuilder, Section, Text};
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Window, WindowBuilder},
};
mod renderer;

#[cfg(target_os = "android")]
fn init_logging() {
    android_logger::init_once(
        android_logger::Config::default()
            .with_min_level(log::Level::Trace)
            .with_tag("vox3"),
    );
}
#[cfg(target_arch = "wasm32")]
fn init_logging() {
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    console_log::init_with_level(log::Level::Warn).expect("Could't initialize logger");
    println!("LOGGER SETUP");
}
#[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
fn init_logging() {
    // simple_logger::SimpleLogger::new().init().unwrap();
}

fn run(event_loop: EventLoop<()>, window: Window) {
    let mut renderer = renderer::Renderer::new(&window);

    // let mut frames = 0;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            Event::WindowEvent {
                window_id: _,
                event: WindowEvent::Resized(size),
            } => {
                renderer.resize(size);
                window.request_redraw();
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput { input, .. } => {
                    if let KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::F2),
                        state: ElementState::Released,
                        ..
                    } = input
                    {
                        println!("TODO: Screenshot");
                    }
                }
                _ => {}
            },
            Event::RedrawEventsCleared | Event::MainEventsCleared | Event::NewEvents(_) => {}
            Event::Resumed => {
                let size = window.inner_size();
                renderer.resize(size);
                window.request_redraw();
            }

            Event::Suspended => {
                renderer.suspend();
            }
            Event::RedrawRequested(_) => {
                renderer.prepare_surface(&window);
                renderer.render();
                window.request_redraw();
            }

            _ => {}
        }
    });
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn main_proxy() {
    init_logging();
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("A fantastic window!")
        .build(&event_loop)
        .unwrap();
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| doc.body())
            .and_then(|body| {
                body.append_child(&web_sys::Element::from(window.canvas()))
                    .ok()
            })
            .expect("Couldn't append canvas to document body.");
    }

    run(event_loop, window);
}
#[mobile_entry_point]
fn mobile_main() {
    init_logging();
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("A fantastic window!")
        .build(&event_loop)
        .unwrap();

    run(event_loop, window);
    println!("Event loop stopped running")
}
