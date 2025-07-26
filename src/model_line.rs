//! # Line Model Module
//! 
//! This module provides functionality for handling 3D line models.
//! It defines data structures and traits for storing and rendering
//! collections of 3D lines with position and color attributes.
//!
//! Key components:
//! - `LineVertex`: Vertex structure for lines with position and color
//! - `LineModel`: A collection of lines with rendering properties
//! - `DrawLines` trait: Rendering abstraction for line collections

use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
#[allow(dead_code)]
pub struct LineVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl LineVertex {
    #[allow(dead_code)]
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<LineVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct LineModel {
    pub _name: String, // Using underscore to indicate unused field
    pub vertex_buffer: wgpu::Buffer,
    pub num_vertices: u32,
}

impl LineModel {
    #[allow(dead_code)]
    pub fn new(device: &wgpu::Device, name: &str, vertices: &[LineVertex]) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} Vertex Buffer", name)),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        LineModel {
            _name: name.to_string(),
            vertex_buffer,
            num_vertices: vertices.len() as u32,
        }
    }
}

#[allow(dead_code)]
pub trait DrawLines<'a> {
    fn draw_lines(
        &mut self,
        line_model: &'a LineModel,
        camera_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawLines<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_lines(
        &mut self,
        line_model: &'b LineModel,
        camera_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, line_model.vertex_buffer.slice(..));
        self.set_bind_group(0, camera_bind_group, &[]);
        self.draw(0..line_model.num_vertices, 0..1);
    }
}
