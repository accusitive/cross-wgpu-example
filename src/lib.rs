use futures::{
    executor::{LocalPool, LocalSpawner},
    task::SpawnExt,
};
use mobile_entry_point::mobile_entry_point;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
use wgpu::{
    util::StagingBelt, Backends, BufferDescriptor, Extent3d, ImageCopyBuffer, ImageCopyTexture,
    ImageDataLayout, Limits, RequestAdapterOptions, Texture, TextureFormat,
};
use wgpu_glyph::{GlyphBrushBuilder, Section, Text};
use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{UserAttentionType, Window, WindowBuilder},
};

#[cfg(target_os = "android")]
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
#[cfg(target_arch = "wasm32")]
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
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

fn run(
    event_loop: EventLoop<()>,
    window: Window,
    mut local_pool: LocalPool,
    spawner: LocalSpawner,
) {
    let instance = wgpu::Instance::new(Backends::all());

    #[cfg(target_os = "android")]
    let mut surface = None;

    #[cfg(not(target_os = "android"))]
    let mut surface = Some(unsafe { instance.create_surface(&window) });

    let adapter =
        futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            force_fallback_adapter: false,
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: surface.as_ref(),
        }))
        .expect("Failed to find an appropriate adapter");
    let (device, queue) = futures::executor::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: None,
            features: wgpu::Features::empty(),
            // limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
            limits: Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
        },
        None,
    ))
    .expect("Failed to create device");

    let mut staging_belt = wgpu::util::StagingBelt::new(1024);
    // let mut local_pool = futures::executor::LocalPool::new();
    let local_spawner = local_pool.spawner();

    let size = window.inner_size();
    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: FORMAT,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
    };
    #[cfg(not(target_os = "android"))]
    {
        surface.as_ref().unwrap().configure(&device, &config);
    }
    let shader = device.create_shader_module(&wgpu::include_wgsl!("./shader.wgsl"));

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[FORMAT.into()],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let inconsolata =
        wgpu_glyph::ab_glyph::FontArc::try_from_slice(include_bytes!("Inconsolata-Regular.ttf"))
            .unwrap();

    let mut glyph_brush = GlyphBrushBuilder::using_font(inconsolata).build(&device, FORMAT);
    let mut frames = 0;
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
                config.width = size.width;
                config.height = size.height;
                // If surface doesn't already exist resumed hasnt been called
                if surface.is_some() {
                    surface.as_ref().unwrap().configure(&device, &config);
                    window.request_redraw();
                }
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
                config.width = size.width;
                config.height = size.height;
                surface = Some(unsafe { instance.create_surface(&window) });
                surface.as_ref().unwrap().configure(&device, &config);
                window.request_redraw();
            }

            Event::Suspended => {
                surface.take();
            }
            Event::RedrawRequested(_) => {
                #[cfg(not(target_os = "android"))]
                {
                    if surface.is_none() {
                        surface = Some(unsafe { instance.create_surface(&window) });
                        surface.as_ref().unwrap().configure(&device, &config);
                    }
                }

                match &surface {
                    Some(surface) => {
                        let start_of_frame = std::time::Instant::now();
                        let frame = surface
                            .get_current_texture()
                            .expect("Failed to acquire next swap chain texture");
                        let view = frame
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default());

                        let mut encoder =
                            device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                                label: None,
                            });
                        {
                            let mut rpass =
                                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                                    label: None,
                                    color_attachments: &[wgpu::RenderPassColorAttachment {
                                        view: &view,
                                        resolve_target: None,
                                        ops: wgpu::Operations {
                                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                                r: 0x78 as f64 / 255.0,
                                                g: 0xa7 as f64 / 255.0,
                                                b: 0xff as f64 / 255.0,
                                                a: 1.0,
                                            }),
                                            store: true,
                                        },
                                    }],
                                    depth_stencil_attachment: None,
                                });
                            rpass.set_pipeline(&render_pipeline);
                            rpass.draw(0..10, 0..1);
                        }
                        let mut queue_text = |content: &str, x: f32, y: f32, color: u32| {
                            let r = (color & 0xFF000000) >> 24;
                            let g = (color & 0x00FF0000) >> 16;
                            let b = (color & 0x000000FF) >> 8;
                            let a = color & 0x000000FF;

                            glyph_brush.queue(Section {
                                screen_position: (x, y),
                                bounds: (size.width as f32, size.height as f32),
                                text: vec![Text::new(content)
                                    .with_color([
                                        r as f32 / 255.0,
                                        g as f32 / 255.0,
                                        b as f32 / 255.0,
                                        a as f32 / 255.0,
                                    ])
                                    .with_scale(20.0)],
                                ..Section::default()
                            });
                        };
                        {
                            queue_text("Welp", 1.0, 2.0, 0xffffffff);
                            queue_text(&format!("Frame #{}", frames), 1.0, 22.0, 0x00FF00FF);
                            // Draw the text!
                            glyph_brush
                                .draw_queued(
                                    &device,
                                    &mut staging_belt,
                                    &mut encoder,
                                    &view,
                                    size.width,
                                    size.height,
                                )
                                .expect("Draw queued");
                            staging_belt.finish();
                        }

                        queue.submit(Some(encoder.finish()));

                        frame.present();
                        local_spawner
                            .spawn(staging_belt.recall())
                            .expect("Recall staging belt");

                        local_pool.run_until_stalled();
                        println!("Presented frame, {}", start_of_frame.elapsed().as_millis());

                        frames+=1;
                        
                    }
                    None => {}
                }
                
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
    let pool = futures::executor::LocalPool::new();
    let spawner = pool.spawner();
    run(event_loop, window, pool, spawner);
}
#[mobile_entry_point]
fn mobile_main() {
    init_logging();
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("A fantastic window!")
        .build(&event_loop)
        .unwrap();
    let pool = futures::executor::LocalPool::new();
    let spawner = pool.spawner();
    run(event_loop, window, pool, spawner);
    println!("Event loop stopped running")
}
