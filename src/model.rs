use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, Buffer, BufferUsages, CommandEncoder, Device, RenderPass, VertexBufferLayout,
};

use crate::vertex::{self, Vertex};

pub struct Faces {
    pub north: bool,
    pub south: bool,
    pub top: bool,
    pub bottom: bool,
    pub east: bool,
    pub west: bool,
}
pub struct Model {
    vertex_buffer: Buffer,
    instance_buffer: Buffer,
    vert_count: u32,
    index_buffer: Buffer,
    indexes: u32,
    position: cgmath::Vector3<f32>,
}
impl Model {
    fn get_vertices(f: &Faces) -> (Vec<Vertex>, Vec<u16>) {
        let mut v = vec![];
        let mut i: Vec<u16> = vec![];
        let mut faces_added = 0;
        // #[rustfmt::skip]
        let mut indexes = || {
            let ind = [0, 1, 2, 2, 3, 0].map(|j| j + faces_added * 4);
            faces_added += 1;
            ind
        };
        if f.top {
            v.extend(vertex::top_face());
            i.extend(indexes());
        }
        if f.bottom {
            v.extend(vertex::bottom());
            i.extend(indexes());
            println!("{:?}", i);
        }
        if f.north {
            v.extend(vertex::north());
            i.extend(indexes());
        }
        if f.south {
            v.extend(vertex::south());
            i.extend(indexes());
        }
        if f.east {
            v.extend(vertex::east());
            i.extend(indexes());
        }
        if f.west {
            v.extend(vertex::west());
            i.extend(indexes());
        }

        (v, i)
    }
    pub fn new(device: &Device, f: &Faces, x: f32) -> Self {
        let (verts, indexes) = Self::get_vertices(f);
        let position = cgmath::vec3(x, 1.0, 1.0);
        let q = cgmath::Matrix4::from_translation(position);
        let p: &[[f32; 4]; 4] = &q.into();
        // let p: &[f32; 3] = &position.into();
        // let ir = InstanceRaw{
        //     model: position.into()
        // };
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&verts),
            usage: BufferUsages::VERTEX,
        });
        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Index buffer"),
            contents: bytemuck::cast_slice::<u16, _>(&indexes),
            usage: BufferUsages::INDEX,
        });
        let instance_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(p),
            usage: BufferUsages::VERTEX,
        });
        println!("{:#?}", p);
        Self {
            vertex_buffer,
            vert_count: verts.len() as u32,
            index_buffer,
            indexes: indexes.len() as u32,
            position,
            instance_buffer
        }
    }
    // pub   fn render<'a> (&self, render_pass: &'a mut RenderPass<'a>, camera_bind_group: &'a mut BindGroup) {
    //     render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
    //     render_pass.set_bind_group(0, camera_bind_group, &[]);
    //     render_pass.draw(0..3, 0..1);

    // }
}
pub trait RenderModel<'r> {
    fn render_model(&mut self, m: &'r Model);
}
impl<'a, 'b> RenderModel<'b> for RenderPass<'a>
where
    'b: 'a,
{
    fn render_model(&mut self, m: &'b Model) {
        self.set_vertex_buffer(0, m.vertex_buffer.slice(..));
        self.set_vertex_buffer(1, m.instance_buffer.slice(..));
        self.set_index_buffer(m.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

        // self.draw(0..m.vert_count, 0..1);
        self.draw_indexed(0..m.indexes, 0, 0..1);
    }
}
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceRaw {
    model: [[f32; 4]; 4]
}
pub fn get_instance_buffer_layout<'a>() -> VertexBufferLayout<'a> {
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Instance,
        attributes: &[
            wgpu::VertexAttribute {
                offset: 0,
                // While our vertex shader only uses locations 0, and 1 now, in later tutorials we'll
                // be using 2, 3, and 4, for Vertex. We'll start at slot 5 not conflict with them later
                shader_location: 2,
                format: wgpu::VertexFormat::Float32x4,
            },
            // A mat4 takes up 4 vertex slots as it is technically 4 vec4s. We need to define a slot
            // for each vec4. We'll have to reassemble the mat4 in
            // the shader.
            wgpu::VertexAttribute {
                offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                shader_location: 3,
                format: wgpu::VertexFormat::Float32x4,
            },
            wgpu::VertexAttribute {
                offset: std::mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                shader_location: 4,
                format: wgpu::VertexFormat::Float32x4,
            },
            wgpu::VertexAttribute {
                offset: std::mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                shader_location: 5,
                format: wgpu::VertexFormat::Float32x4,
            },
            // wgpu::VertexAttribute {
            //     offset: 0,
            //     shader_location: 2,
            //     format: wgpu::VertexFormat::Float32x3,
            // },
            // wgpu::VertexAttribute {
            //     offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
            //     shader_location: 1,
            //     format: wgpu::VertexFormat::Float32x3,
            // },
        ],
    }
}