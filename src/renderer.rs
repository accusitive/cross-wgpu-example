use std::sync::{Arc, Mutex};

use egui::FullOutput;
use egui_demo_lib::WrapApp;
use egui_wgpu_backend::ScreenDescriptor;
use egui_winit_platform::{Platform, PlatformDescriptor};
use epi::App;
use futures::{
    executor::{LocalPool, LocalSpawner},
    task::SpawnExt,
};
use image::Primitive;
// use imgui::FontSource;
use instant::Instant;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt, StagingBelt},
    Adapter, Backends, BindGroup, BindGroupLayout, Buffer, BufferUsages, Device, Face, Features,
    Instance, Limits, PrimitiveState, Queue, RenderPipeline, ShaderModule, Surface,
    SurfaceConfiguration,
};
use wgpu_glyph::{GlyphBrush, GlyphBrushBuilder, Section, Text};
use winit::{dpi::PhysicalSize, event_loop::EventLoopProxy, window::Window};

use crate::{
    camera::{Camera, CameraController},
    gui::{self, TropicGui},
    model::{Faces, Model, RenderModel, self},
    vertex::{Vertex, VERTICES},
};

#[cfg(target_os = "android")]
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
#[cfg(target_arch = "wasm32")]
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Bgra8UnormSrgb;

impl TropicRenderer {
    fn create_initial_surface(window: &Window, instance: &Instance) -> Option<Surface> {
        #[cfg(target_os = "android")]
        return None;

        #[cfg(not(target_os = "android"))]
        Some(unsafe { instance.create_surface(&window) })
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
    fn create_render_pipeline(
        device: &Device,
        shader: &ShaderModule,
        bind_groups_layouts: Vec<&BindGroupLayout>,
        primitive: Option<PrimitiveState>,
    ) -> RenderPipeline {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &bind_groups_layouts,
            push_constant_ranges: &[],
        });
        // let mut primitive = wgpu::PrimitiveState::default();

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), model::get_instance_buffer_layout()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[FORMAT.into()],
            }),
            primitive: primitive.unwrap_or_default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        })
    }
    fn get_features() -> Features {
        #[cfg(target_arch = "wasm32")]
        return Features::empty();

        Features::POLYGON_MODE_LINE
    }
    pub fn new(window: &Window, event_loop_proxy: EventLoopProxy<Event>) -> Self {
        let instance = wgpu::Instance::new(Backends::all());

        let surface = Self::create_initial_surface(window, &instance);
        let adapter = Self::get_default_adapter(&instance, &surface);

        let (device, queue) = futures::executor::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("Main device"),
                features: Self::get_features(),
                limits: Self::get_device_limits(&adapter),
            },
            None,
        ))
        .expect("Failed to create device");

        let staging_belt = wgpu::util::StagingBelt::new(1024);

        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
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

        let camera = create_camera(&config);
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });
        let camera_controller = CameraController::new(0.2);
        let render_pipeline =
            Self::create_render_pipeline(&device, &shader, vec![&camera_bind_group_layout], None);
        #[cfg(target_arch = "wasm32")]
        let wire_frame_render_pipeline = None;

        let wireframe_primitive = PrimitiveState {
            polygon_mode: wgpu::PolygonMode::Line,
            ..Default::default()
        };
        #[cfg(not(target_arch = "wasm32"))]
        let wire_frame_render_pipeline = Some(Self::create_render_pipeline(
            &device,
            &shader,
            vec![&camera_bind_group_layout],
            Some(wireframe_primitive),
        ));

        let font_brush = Self::setup_fonts(&device);
        let local_pool = futures::executor::LocalPool::new();
        let local_spawner = local_pool.spawner();

        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Index buffer"),
            contents: bytemuck::cast_slice::<u16, _>(&[0, 1, 2, 2, 3, 0]),
            usage: BufferUsages::INDEX,
        });
        let platform = Self::setup_egui(window, &size);
        let egui_rpass = egui_wgpu_backend::RenderPass::new(&device, FORMAT, 1);
        // let demo_app = egui_demo_lib::WrapApp::default();
        let demo_app = gui::TropicGui { wireframe: false };

        let start_time = Instant::now();
        let previous_frame_time = None;

        {

            // let render_pipeline_layout =
            //     device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            //         label: Some("Render Pipeline Layout"),
            //         bind_group_layouts: &[&texture_bind_group_layout, &camera_bind_group_layout],
            //         push_constant_ranges: &[],
            //     });
        }
        Self {
            instance,
            device,
            surface,
            config,
            render_pipeline,
            wire_frame_render_pipeline,
            font_brush,
            size,
            staging_belt,
            queue,
            local_pool,
            local_spawner,
            fps_smoothing: 0.9,
            fps_measurement: 0.0,
            vertex_buffer,
            index_buffer,
            egui_rpass,
            tropic_gui: demo_app,
            start_time,
            egui_platform: platform,
            scale_factor: window.scale_factor(),
            previous_frame_time,
            repaint_signal: Arc::new(ExampleRepaintSignal(Mutex::new(event_loop_proxy))),
            camera,
            camera_uniform,
            camera_bind_group,
            camera_buffer,
            camera_controller,
        }
    }
    pub fn resume(&mut self, window: &Window) {
        self.prepare_surface(window);
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
    pub fn prepare_surface(&mut self, window: &Window) {
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
    pub fn setup_egui(window: &Window, size: &PhysicalSize<u32>) -> Platform {
        Platform::new(PlatformDescriptor {
            physical_width: size.width as u32,
            physical_height: size.height as u32,
            scale_factor: window.scale_factor(),
            font_definitions: egui::FontDefinitions::default(),
            style: Default::default(),
        })
    }
    fn draw_hud(&mut self) {
        self.draw_text(
            &format!("FPS {}", self.fps_measurement * 1000.0),
            1.0,
            1.0,
            0xff00ffff,
        );
    }
    fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }
    pub fn render(&mut self, window: &Window) {
        self.update();
        self.egui_platform
            .update_time(self.start_time.elapsed().as_secs_f64());
        let m = Model::new(
            &self.device,
            &Faces {
                north: true,
                south: true,
                top: true,
                bottom: true,
                east: false,
                west: true,
            },
            0.0
        );
        let m2 = Model::new(
            &self.device,
            &Faces {
                north: true,
                south: true,
                top: true,
                bottom: true,
                east: true,
                west: false,
            },
            1.0
        );
        match &self.surface {
            Some(surface) => {
                // let start_of_frame = std::time::Instant::now();
                let start_of_frame = Instant::now();
                let frame = surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture");
                let view = frame
                    .texture
                    .create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = self
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                //    if self.tropic_gui.wireframe {
                //      self.render_pipeline = Self::create_render_pipeline(self.device, self.shader, bind_groups_layouts, primitive)
                //     }
                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
                    if self.tropic_gui.wireframe {
                        render_pass
                            .set_pipeline(&self.wire_frame_render_pipeline.as_ref().unwrap());
                    } else {
                        render_pass.set_pipeline(&self.render_pipeline);
                    }
                    // render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                    // render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

                    // // render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    // // render_pass.draw_indexed(0..6, 0, 0..1);
                    // render_pass.draw(0..3, 0..1);

                    render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                    render_pass.render_model(&m);
                    render_pass.render_model(&m2);

                    // m.render(&mut render_pass, &mut self.camera_bind_group);
                }
                self.draw_hud();

                // Draw fonts
                {
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
                // Egui
                // #[cfg(not(target_arch = "wasm32"))]
                {
                    let egui_start = Instant::now();
                    self.egui_platform.begin_frame();
                    let app_output = epi::backend::AppOutput::default();
                    let mut frame = epi::Frame::new(epi::backend::FrameData {
                        info: epi::IntegrationInfo {
                            name: "egui_example",
                            web_info: None,
                            cpu_usage: self.previous_frame_time,
                            native_pixels_per_point: Some(self.scale_factor as _),
                            prefer_dark_mode: None,
                        },
                        output: app_output,
                        repaint_signal: self.repaint_signal.clone(),
                    });
                    self.tropic_gui
                        .update(&self.egui_platform.context(), &mut frame);

                    // let (_output, paint_commands) = self.egui_platform.end_frame(Some(&window));
                    let FullOutput {
                        platform_output: _output,
                        shapes: paint_commands,
                        textures_delta,
                        ..
                    } = self.egui_platform.end_frame(Some(&window));

                    let paint_jobs = self.egui_platform.context().tessellate(paint_commands);
                    let frame_time = (Instant::now() - egui_start).as_secs_f64() as f32;
                    self.previous_frame_time = Some(frame_time);

                    let screen_descriptor = ScreenDescriptor {
                        physical_width: self.config.width,
                        physical_height: self.config.height,
                        scale_factor: window.scale_factor() as f32,
                    };
                    // self.egui_rpass.
                    self.egui_rpass
                        .add_textures(&self.device, &self.queue, &textures_delta)
                        .expect("Couldn't add textures to EGUI");
                    // self.egui_rpass.update_user_textures(&self.device, &self.queue);
                    self.egui_rpass.update_buffers(
                        &self.device,
                        &self.queue,
                        &paint_jobs,
                        &screen_descriptor,
                    );

                    self.egui_rpass
                        .execute(&mut encoder, &view, &paint_jobs, &screen_descriptor, None)
                        .unwrap();
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
}

fn create_camera(config: &SurfaceConfiguration) -> Camera {
    let camera = Camera {
        // position the camera one unit up and 2 units back
        // +z is out of the screen
        eye: (0.0, 1.0, 2.0).into(),
        // have it look at the origin
        target: (0.0, 0.0, 0.0).into(),
        // which way is "up"
        up: cgmath::Vector3::unit_y(),
        aspect: config.width as f32 / config.height as f32,
        fovy: 45.0,
        znear: 0.1,
        zfar: 100.0,
    };
    camera
}

pub struct TropicRenderer {
    instance: Instance,
    device: Device,
    surface: Option<Surface>,
    config: SurfaceConfiguration,
    render_pipeline: RenderPipeline,
    wire_frame_render_pipeline: Option<RenderPipeline>,

    font_brush: GlyphBrush<()>,
    size: PhysicalSize<u32>,
    staging_belt: StagingBelt,
    local_pool: LocalPool,
    local_spawner: LocalSpawner,
    queue: Queue,
    fps_smoothing: f32,
    fps_measurement: f32,
    vertex_buffer: Buffer,
    index_buffer: Buffer,
    egui_rpass: egui_wgpu_backend::RenderPass,
    tropic_gui: TropicGui,
    start_time: Instant,
    pub egui_platform: Platform,
    scale_factor: f64,
    previous_frame_time: Option<f32>,
    repaint_signal: Arc<ExampleRepaintSignal>,
    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: Buffer,
    camera_bind_group: BindGroup,
    pub camera_controller: CameraController,
}
#[derive(Debug, Clone, Copy)]
pub enum Event {
    RequestRedraw,
}
struct ExampleRepaintSignal(std::sync::Mutex<winit::event_loop::EventLoopProxy<Event>>);

impl epi::backend::RepaintSignal for ExampleRepaintSignal {
    fn request_repaint(&self) {
        self.0.lock().unwrap().send_event(Event::RequestRedraw).ok();
    }
}

unsafe impl Sync for ExampleRepaintSignal {}
unsafe impl Send for ExampleRepaintSignal {}

#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    // We can't use cgmath with bytemuck directly so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}
