//! # Line Model Module
//! 
//! This module provides functionality for handling 3D line models using OpenModel geometry.
//! It defines data structures and traits for storing and rendering
//! collections of 3D lines with position and color attributes.
//!
//! Key components:
//! - `LineVertex`: GPU vertex structure for lines with position and color
//! - `LineModel`: A collection of lines with rendering properties
//! - `DrawLines` trait: Rendering abstraction for line collections
//! - OpenModel integration: Bridge between OpenModel Line and GPU structures

use wgpu::util::DeviceExt;
use openmodel::geometry::Line as OpenModelLine;
use openmodel::primitives::Color as OpenModelColor;

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

    /// Create a LineVertex from position and color
    pub fn new(position: [f32; 3], color: [f32; 3]) -> Self {
        LineVertex { position, color }
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
            label: Some(&format!("{} Line Vertex Buffer", name)),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Self {
            _name: String::from(name),
            vertex_buffer,
            num_vertices: vertices.len() as u32,
        }
    }

    /// Create a LineModel from an OpenModel Line with default color
    pub fn from_openmodel_line(device: &wgpu::Device, name: &str, line: &OpenModelLine) -> Self {
        let color = if line.data.has_color() {
            let color_data = line.data.get_color();
            [color_data[0] as f32 / 255.0, color_data[1] as f32 / 255.0, color_data[2] as f32 / 255.0]
        } else {
            [1.0, 1.0, 1.0] // Default white color
        };

        let vertices = vec![
            LineVertex::new([line.x0 as f32, line.y0 as f32, line.z0 as f32], color),
            LineVertex::new([line.x1 as f32, line.y1 as f32, line.z1 as f32], color),
        ];

        Self::new(device, name, &vertices)
    }

    /// Create a LineModel from a collection of OpenModel Lines
    pub fn from_openmodel_lines(device: &wgpu::Device, name: &str, lines: &[OpenModelLine]) -> Self {
        let mut vertices = Vec::new();

        for line in lines {
            let color = if line.data.has_color() {
                let color_data = line.data.get_color();
                [color_data[0] as f32 / 255.0, color_data[1] as f32 / 255.0, color_data[2] as f32 / 255.0]
            } else {
                [1.0, 1.0, 1.0] // Default white color
            };

            vertices.push(LineVertex::new([line.x0 as f32, line.y0 as f32, line.z0 as f32], color));
            vertices.push(LineVertex::new([line.x1 as f32, line.y1 as f32, line.z1 as f32], color));
        }

        Self::new(device, name, &vertices)
    }

    /// Create a LineModel from an OpenModel Line with specified color
    pub fn from_openmodel_line_with_color(device: &wgpu::Device, name: &str, line: &OpenModelLine, color: &OpenModelColor) -> Self {
        let color_array = [color.r as f32 / 255.0, color.g as f32 / 255.0, color.b as f32 / 255.0];
        
        let vertices = vec![
            LineVertex::new([line.x0 as f32, line.y0 as f32, line.z0 as f32], color_array),
            LineVertex::new([line.x1 as f32, line.y1 as f32, line.z1 as f32], color_array),
        ];

        Self::new(device, name, &vertices)
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
