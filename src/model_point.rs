//! # Point Cloud Model Module
//! 
//! This module provides functionality for handling 3D point cloud models.
//! It defines data structures and traits for storing and rendering
//! collections of 3D points with position, color, and size attributes.
//!
//! Key components:
//! - `PointVertex`: Vertex structure for point clouds with position, color, and size
//! - `PointModel`: A collection of points with rendering properties
//! - `DrawPoints` trait: Rendering abstraction for point clouds

use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub size: f32,
}

impl PointVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<PointVertex>() as wgpu::BufferAddress,
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
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
}

pub struct PointModel {
    pub _name: String, // Using underscore to indicate unused field
    pub vertex_buffer: wgpu::Buffer,
    pub num_vertices: u32,
}

impl PointModel {
    pub fn new(device: &wgpu::Device, name: &str, vertices: &[PointVertex]) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} Vertex Buffer", name)),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            _name: String::from(name),
            vertex_buffer,
            num_vertices: vertices.len() as u32,
        }
    }
}

pub trait DrawPoints<'a> {
    fn draw_points(
        &mut self,
        point_model: &'a PointModel,
        camera_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawPoints<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_points(
        &mut self,
        point_model: &'b PointModel,
        camera_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, point_model.vertex_buffer.slice(..));
        self.set_bind_group(0, camera_bind_group, &[]);
        self.draw(0..point_model.num_vertices, 0..1);
    }
}
