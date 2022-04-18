use log::{debug, info};
use mobile_entry_point::mobile_entry_point;
use wgpu::{Backends, RequestAdapterOptions, Texture, TextureFormat};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{UserAttentionType, WindowBuilder, Window},
};
#[cfg(target_os = "android")]
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
#[cfg(not(target_os = "android"))]
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;
#[cfg(target_os = "android")]
fn init_logging() {
    android_logger::init_once(
        android_logger::Config::default()
            .with_min_level(log::Level::Trace)
            .with_tag("vox3"),
    );
}

#[cfg(not(target_os = "android"))]
fn init_logging() {
    // simple_logger::SimpleLogger::new().init().unwrap();
}
async fn run(event_loop: EventLoop<()>, window: Window) {
    let instance = wgpu::Instance::new(Backends::all());
    let mut surface = None;

    #[cfg(not(target_os = "android"))]
    {
        surface = Some(unsafe { instance.create_surface(&window) });
    }
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            force_fallback_adapter: false,
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: surface.as_ref(),
        })
        .await
        .expect("Failed to find an appropriate adapter");
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits()),
            },
            None,
        )
        .await
        .expect("Failed to create device");
    let size = window.inner_size();
    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: FORMAT,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Mailbox,
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

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            Event::RedrawEventsCleared | Event::MainEventsCleared | Event::NewEvents(_) => {}
            Event::Resumed => {
                let size = window.inner_size();
                config.width = size.width;
                config.height = size.height;
                surface = Some(unsafe { instance.create_surface(&window) });
                surface.as_ref().unwrap().configure(&device, &config);
                window.request_redraw();
            }
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
                                            load: wgpu::LoadOp::Clear(wgpu::Color{
                                                r: 0x78 as f64 / 255.0,
                                                g: 0xa7 as f64 / 255.0,
                                                b: 0xff as f64 / 255.0,
                                                a: 1.0
                                            }),
                                            store: true,
                                        },
                                    }],
                                    depth_stencil_attachment: None,
                                });
                            rpass.set_pipeline(&render_pipeline);
                            rpass.draw(0..10, 0..1);
                        }

                        queue.submit(Some(encoder.finish()));
                        frame.present();
                    }
                    None => {}
                }
            }

            _ => {
            },
        }
    });

}
#[mobile_entry_point]
fn main() {
    init_logging();
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("A fantastic window!")
        .build(&event_loop)
        .unwrap();
    pollster::block_on(run(event_loop, window));
    println!("Event loop stopped running")
}
