use futures::{
    executor::{LocalPool, LocalSpawner},
    task::{LocalSpawn, SpawnExt},
};
use wgpu::{
    util::StagingBelt, Adapter, Backends, Device, Instance, Limits, Queue, RenderPipeline, Surface,
    SurfaceConfiguration,
};
use wgpu_glyph::{GlyphBrush, GlyphBrushBuilder, Section, Text};
use winit::{
    dpi::{PhysicalPosition, PhysicalSize, Size},
    window::Window,
};

#[cfg(target_os = "android")]
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
#[cfg(target_arch = "wasm32")]
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

impl Renderer {
    fn create_initial_surface(window: &Window, instance: &Instance) -> Option<Surface> {
        #[cfg(target_os = "android")]
        let mut surface = None;

        #[cfg(not(target_os = "android"))]
        let mut surface = Some(unsafe { instance.create_surface(&window) });

        surface
    }
    fn get_default_adapter(instance: &Instance, surface: &Option<Surface>) -> Adapter {
        futures::executor::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            force_fallback_adapter: false,
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: surface.as_ref(),
        }))
        .expect("Failed to find an appropriate adapter")
    }
    fn get_device_limits(adapter: &Adapter) -> Limits {
        #[cfg(target_arch = "wasm32")]
        return Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits());
        Limits::downlevel_defaults().using_resolution(adapter.limits())
    }
    pub fn new(window: &Window) -> Self {
        let instance = wgpu::Instance::new(Backends::all());

        let surface = Self::create_initial_surface(window, &instance);
        let adapter = Self::get_default_adapter(&instance, &surface);

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

        let mut font_brush = Self::setup_fonts(&device);
        let local_pool = futures::executor::LocalPool::new();
        let local_spawner = local_pool.spawner();

        Self {
            instance,
            device,
            surface,
            config,
            render_pipeline,
            font_brush,
            size,
            staging_belt,
            queue,
            local_pool,
            local_spawner,
            fps_smoothing: 0.9,
            fps_measurement: 0.0,
        }
    }
    pub fn suspend(&mut self) {
        self.surface.take();
    }
    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.config.width = size.width;
        self.config.height = size.height;
        self.size = size;
        // If surface doesn't already exist resumed hasnt been called
        if self.surface.is_some() {
            self.surface
                .as_ref()
                .unwrap()
                .configure(&self.device, &self.config);
            // self.request_redraw();
        }
    }
    fn setup_fonts(device: &Device) -> GlyphBrush<()> {
        let inconsolata = wgpu_glyph::ab_glyph::FontArc::try_from_slice(include_bytes!(
            "Inconsolata-Regular.ttf"
        ))
        .unwrap();

        GlyphBrushBuilder::using_font(inconsolata).build(&device, FORMAT)
    }
    pub fn prepare_surface(&mut self, window: &Window) {
        #[cfg(not(target_os = "android"))]
        {
            if self.surface.is_none() {
                self.surface = Some(unsafe { self.instance.create_surface(&window) });
                self.surface
                    .as_ref()
                    .unwrap()
                    .configure(&self.device, &self.config);
            }
        }
    }
    pub fn calculate_fps(&mut self, frametime: f32) {
        let fps = 1.0 / frametime;
        self.fps_measurement =
            (self.fps_measurement * self.fps_smoothing) + (fps * (1.0 - self.fps_smoothing))
    }
    pub fn render(&mut self) {
        match &self.surface {
            Some(surface) => {
                let start_of_frame = std::time::Instant::now();
                let frame = surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture");
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = self
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                    rpass.set_pipeline(&self.render_pipeline);
                    rpass.draw(0..10, 0..1);
                }

                {
                    // self.draw_text("Welp", 1.0, 2.0, 0xffffffff);
                    self.draw_text(&format!("FPS {}", self.fps_measurement * 1000.0), 1.0, 22.0, 0x00FF00FF);
                    // Draw the text!
                    self.font_brush
                        .draw_queued(
                            &self.device,
                            &mut self.staging_belt,
                            &mut encoder,
                            &view,
                            self.size.width,
                            self.size.height,
                        )
                        .expect("Draw queued");
                    self.staging_belt.finish();
                }

                self.queue.submit(Some(encoder.finish()));

                frame.present();
                self.local_spawner
                    .spawn(self.staging_belt.recall())
                    .expect("Recall staging belt");

                self.local_pool.run_until_stalled();
                self.calculate_fps(start_of_frame.elapsed().as_millis() as f32);
                // println!("Presented frame, {}", );

                // frames += 1;
            }
            None => {}
        }
    }
    fn draw_text(&mut self, content: &str, x: f32, y: f32, color: u32) {
        let r = (color & 0xFF000000) >> 24;
        let g = (color & 0x00FF0000) >> 16;
        let b = (color & 0x000000FF) >> 8;
        let a = color & 0x000000FF;

        self.font_brush.queue(Section {
            screen_position: (x, y),
            bounds: (self.size.width as f32, self.size.height as f32),
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
    }
}

pub struct Renderer {
    instance: Instance,
    device: Device,
    surface: Option<Surface>,
    config: SurfaceConfiguration,
    render_pipeline: RenderPipeline,
    font_brush: GlyphBrush<()>,
    size: PhysicalSize<u32>,
    staging_belt: StagingBelt,
    local_pool: LocalPool,
    local_spawner: LocalSpawner,
    queue: Queue,
    fps_smoothing: f32,
    fps_measurement: f32,
}
