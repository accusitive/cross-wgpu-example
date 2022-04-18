use log::{debug, info};
use mobile_entry_point::mobile_entry_point;
use wgpu::{Backends, RequestAdapterOptions, TextureFormat, Texture};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

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
#[mobile_entry_point]
fn main() {
    init_logging();
    let event_loop = EventLoop::new();

    let window = WindowBuilder::new()
        .with_title("A fantastic window!")
        // .with_inner_size(winit::dpi::LogicalSize::new(128.0, 128.0))
        .build(&event_loop)
        .unwrap();
    pollster::block_on(async {
        let instance = wgpu::Instance::new(Backends::PRIMARY);
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                force_fallback_adapter: false,
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: None,
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
        let shader = device.create_shader_module(&wgpu::include_wgsl!("./shader.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        // let swapchain_format = TextureFormat::Rgba8UnormSrgb;
        let swapchain_format = TextureFormat::Bgra8Unorm;

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
                targets: &[swapchain_format.into()],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
        let size = window.inner_size();
        let mut config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: swapchain_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Mailbox,
        };
        let mut surface = None;

        info!("Got device and queue");
        // println!("{:#?}", device);   
        // println!("{:#?}", queue);
        let mut has_resumed = false;
   
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            println!("Event called: {:?}", event);

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    window_id,
                } if window_id == window.id() => *control_flow = ControlFlow::Poll,
                Event::Resumed => {
                    println!("RESUMED");
                    let size = window.inner_size();
                    config.width = size.width;
                    config.height = size.height;
                    surface = Some(unsafe { instance.create_surface(&window) });
                    surface.as_ref().unwrap().configure(&device, &config);
                    println!("Created {:?}", surface);
                    window.request_redraw();
                }
                Event::Suspended => {
                    println!("SUSPENDED");
                    surface.take();
                }
                Event::RedrawRequested(_) | Event::RedrawEventsCleared => {
                    println!("RedrawRequested");
                    #[cfg(target_os = "linux")]
                    {
                        if surface.is_none() {
                            surface = Some(unsafe { instance.create_surface(&window) });
                            surface.as_ref().unwrap().configure(&device, &config);
                        }
                       
                    }
                    
                    match &surface {
                        Some(surface) => {
                            println!("Has surface");
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
                                                load: wgpu::LoadOp::Clear(wgpu::Color::GREEN),
                                                store: true,
                                            },
                                        }],
                                        depth_stencil_attachment: None,
                                    });
                                rpass.set_pipeline(&render_pipeline);
                                rpass.draw(0..3, 0..1);
                            }

                            queue.submit(Some(encoder.finish()));
                            frame.present();
                        }
                        None => println!("No surface"),
                    }
                }
                Event::MainEventsCleared => {
                    println!("MainEventsCleared");

                    window.request_redraw();
                }

                _ => (),
            }
        });
    });
    println!("Event loop stopped running")
}
