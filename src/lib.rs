use chunk::{Block, Chunk};
use mobile_entry_point::mobile_entry_point;
use noise::{Fbm, NoiseFn};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Fullscreen, Window, WindowBuilder},
};
mod camera;
mod chunk;
mod gui;
mod model;
mod renderer;
mod texture;
mod vertex;

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

fn run(event_loop: EventLoop<renderer::Event>, window: Window) {
    let mut chunk = Chunk::new();
    let fbm = Fbm::new();

    // println!("{}",val);
    for i in 0..chunk::WIDTH {
        // for j in 0..((val.abs()*(chunk::HEIGHT as f64)) as i64) {
        for k in 0..chunk::LENGTH {
            let val = (fbm.get([i as f64 / 256.0, k as f64 / 256.0, 1.0]) * 6.0 + 5.0).abs();
            println!("{}", val);

            for j in 0..(val as i64) {
                chunk.set_block(Block {
                    x: i,
                    y: j,
                    z: k,
                    kind: chunk::BlockKind::Stone,
                });
            }
        }
        // }
    }

    let mut renderer =
        renderer::TropicRenderer::new(&window, event_loop.create_proxy(), &mut chunk);

    event_loop.run(move |event, _, control_flow| {
        renderer.egui_platform.handle_event(&event);
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent { event, .. } = &event {
            renderer.camera_controller.process_events(&event);
        }
        match event {
            Event::WindowEvent {
                window_id: _,
                event: WindowEvent::Resized(size),
            } => {
                renderer.resize(size);
                window.request_redraw();
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::KeyboardInput { input, .. } => match input {
                    KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::F11),
                        state: ElementState::Released,
                        ..
                    } => match window.fullscreen() {
                        Some(_) => window.set_fullscreen(None),
                        None => window.set_fullscreen(Some(Fullscreen::Borderless(None))),
                    },
                    KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::F2),
                        state: ElementState::Released,
                        ..
                    } => {
                        println!("TODO: screenshot")
                    }
                    _ => {}
                },
                WindowEvent::CloseRequested => {
                    println!("Close requested, exiting.");
                    *control_flow = ControlFlow::Exit
                }
                _ => {}
            },
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            // Event::RedrawEventsCleared | Event::MainEventsCleared | Event::NewEvents(_) => {}
            Event::UserEvent(renderer::Event::RequestRedraw) => {
                window.request_redraw();
                println!("User event redraw")
            }
            Event::Resumed => {
                let size = window.inner_size();
                renderer.resize(size);
                renderer.resume(&window);
                window.request_redraw();
            }

            Event::Suspended => {
                renderer.suspend();
            }
            Event::RedrawRequested(_) => {
                #[cfg(not(target_os = "android"))]
                renderer.prepare_surface(&window);
                renderer.render(&window);
                // for smooth fps
                #[cfg(target_arch = "wasm32")]
                window.request_redraw();

                // window.request_redraw();
            }

            _ => {}
        }
    });
}
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn main_proxy() {
    init_logging();
    let event_loop = EventLoop::with_user_event();

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
            // .and_then(|doc| doc.get_elements_by_tag_name(""))
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
    let event_loop = EventLoop::with_user_event();

    let window = WindowBuilder::new()
        .with_title("A fantastic window!")
        // .with_fullscreen(Some(Fullscreen::Borderless(None)))
        .build(&event_loop)
        .unwrap();

    run(event_loop, window);
    println!("Event loop stopped running")
}
