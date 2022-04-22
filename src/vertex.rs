use wgpu::VertexBufferLayout;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    color: [f32; 2],
}
impl Vertex {
    pub fn desc<'a>() -> VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}
// pub const VERTICES: &[Vertex] = &[
//     Vertex {
//         position: [0.0, 0.5, 0.0],
//         color: [1.0, 0.0, 0.0],
//     },
//     Vertex {
//         position: [-0.5, -0.5, 0.0],
//         color: [0.0, 1.0, 0.0],
//     },
//     Vertex {
//         position: [0.5, -0.5, 0.0],
//         color: [0.0, 0.0, 1.0],
//     },
// ];
pub fn vertex(pos: [i8; 3], tc: [i8; 2], x_offset: f32, y_offset: f32) -> Vertex {
    Vertex {
        position: [pos[0] as f32, pos[1] as f32, pos[2] as f32],
        color: [(tc[0] as f32 + x_offset) / 16.0, (tc[1] as f32 + y_offset) / 16.0 ],
    }
}
pub fn north(x_offset: f32, y_offset: f32) -> [Vertex; 4] {
    [
        vertex([-1, -1, 1], [0, 0], x_offset, y_offset),
        vertex([1, -1, 1], [1, 0], x_offset, y_offset),
        vertex([1, 1, 1], [1, 1], x_offset, y_offset),
        vertex([-1, 1, 1], [0, 1], x_offset, y_offset),
    ]
}
pub fn south(x_offset: f32, y_offset: f32) -> [Vertex; 4] {
    [
        vertex([-1, 1, -1], [1, 0], x_offset, y_offset),
        vertex([1, 1, -1], [0, 0], x_offset, y_offset),
        vertex([1, -1, -1], [0, 1], x_offset, y_offset),
        vertex([-1, -1, -1], [1, 1], x_offset, y_offset),
    ]
}
pub fn west(x_offset: f32, y_offset: f32) -> [Vertex; 4] {
    [
        vertex([-1, -1, 1], [1, 0], x_offset, y_offset),
        vertex([-1, 1, 1], [0, 0], x_offset, y_offset),
        vertex([-1, 1, -1], [0, 1], x_offset, y_offset),
        vertex([-1, -1, -1], [1, 1], x_offset, y_offset),
    ]
}
pub fn east(x_offset: f32, y_offset: f32) -> [Vertex; 4] {
    [
        vertex([1, -1, -1], [0, 0], x_offset, y_offset),
        vertex([1, 1, -1], [1, 0], x_offset, y_offset),
        vertex([1, 1, 1], [1, 1], x_offset, y_offset),
        vertex([1, -1, 1], [0, 1], x_offset, y_offset),
    ]
}
pub fn bottom(x_offset: f32, y_offset: f32) -> [Vertex; 4] {
    [
        vertex([1, -1, 1], [0, 0], x_offset, y_offset),
        vertex([-1, -1, 1], [1, 0], x_offset, y_offset),
        vertex([-1, -1, -1], [1, 1], x_offset, y_offset),
        vertex([1, -1, -1], [0, 1], x_offset, y_offset),
    ]
}

pub fn top(x_offset: f32, y_offset: f32) -> [Vertex; 4] {
    [
        vertex([1, 1, -1], [1, 0], x_offset, y_offset),
        vertex([-1, 1, -1], [0, 0], x_offset, y_offset),
        vertex([-1, 1, 1], [0, 1], x_offset, y_offset),
        vertex([1, 1, 1], [1, 1], x_offset, y_offset),
    ]
}
