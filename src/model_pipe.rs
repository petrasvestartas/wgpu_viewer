//! # Pipe Model Module
//! 
//! This module provides functionality for rendering 3D lines as cylindrical pipes using OpenModel geometry.
//! It defines data structures and traits for storing and rendering collections
//! of 3D pipe segments with position, color and radius attributes.
//!
//! Key components:
//! - `PipeVertex`: GPU vertex structure for pipes with position and color
//! - `PipeSegment`: Definition of a pipe segment with start, end, color and radius
//! - `PipeModel`: A collection of pipe segments rendered as 3D cylinders
//! - `DrawPipes` trait: Rendering abstraction for pipe collections
//! - OpenModel integration: Uses OpenModel's create_pipe method for accurate pipe generation

use wgpu::util::DeviceExt;
use openmodel::geometry::{Line as OpenModelLine, Point as OpenModelPoint, Mesh as OpenModelMesh};
use openmodel::primitives::Color as OpenModelColor;

// Configuration constants
pub const PIPE_RADIUS: f32 = 0.05;  // Default pipe radius/thickness
#[allow(dead_code)]
pub const PIPE_COLOR: [f32; 3] = [1.0, 0.0, 0.0];  // Bright red for debugging

// Pipe segment definition
#[derive(Debug, Clone)]
pub struct PipeSegment {
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub color: [f32; 3],
    pub radius: f32,
}

impl PipeSegment {
    /// Create a new PipeSegment
    #[allow(dead_code)]
    pub fn new(start: [f32; 3], end: [f32; 3], color: [f32; 3], radius: f32) -> Self {
        Self { start, end, color, radius }
    }

    /// Create a PipeSegment from an OpenModel Line
    pub fn from_openmodel_line(line: &OpenModelLine) -> Self {
        let color = if line.data.has_color() {
            let color_data = line.data.get_color();
            [color_data[0] as f32 / 255.0, color_data[1] as f32 / 255.0, color_data[2] as f32 / 255.0]
        } else {
            [1.0, 1.0, 1.0] // Default white color
        };

        let radius = line.data.get_thickness() as f32;
        let radius = if radius > 0.0 { radius } else { PIPE_RADIUS }; // Use default if no thickness

        Self {
            start: [line.x0 as f32, line.y0 as f32, line.z0 as f32],
            end: [line.x1 as f32, line.y1 as f32, line.z1 as f32],
            color,
            radius,
        }
    }

    /// Create a PipeSegment from an OpenModel Line with specified color and radius
    #[allow(dead_code)]
    pub fn from_openmodel_line_with_params(line: &OpenModelLine, color: &OpenModelColor, radius: f32) -> Self {
        let color_array = [color.r as f32 / 255.0, color.g as f32 / 255.0, color.b as f32 / 255.0];
        
        Self {
            start: [line.x0 as f32, line.y0 as f32, line.z0 as f32],
            end: [line.x1 as f32, line.y1 as f32, line.z1 as f32],
            color: color_array,
            radius,
        }
    }
}

// Vertex structure for cylinders - simplified for flat color shader
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PipeVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl PipeVertex {
    #[allow(dead_code)]
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<PipeVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // color
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct PipeModel {
    #[allow(dead_code)]
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

impl PipeModel {
    pub fn new(
        device: &wgpu::Device, 
        name: &str, 
        pipe_segments: &[PipeSegment],
    ) -> Self {
        // Generate vertices and indices for all pipe segments using OpenModel
        let mut all_vertices = Vec::new();
        let mut all_indices = Vec::new();
        let mut vertex_offset = 0u32;
        
        for segment in pipe_segments {
            // Convert to OpenModel types
            let start = OpenModelPoint::new(segment.start[0] as f64, segment.start[1] as f64, segment.start[2] as f64);
            let end = OpenModelPoint::new(segment.end[0] as f64, segment.end[1] as f64, segment.end[2] as f64);
            let radius = segment.radius as f64;
            
            // Use OpenModel's create_pipe method
            let openmodel_mesh = OpenModelMesh::create_pipe(start, end, radius);
            
            // Convert OpenModel mesh to GPU format
            let mut vertex_map = std::collections::HashMap::new();
            let mut next_local_index = 0u32;
            
            for (_face_key, face_vertices) in openmodel_mesh.get_face_data() {
                if face_vertices.len() >= 3 {
                    // Triangulate the face (fan triangulation)
                    for i in 1..face_vertices.len() - 1 {
                        let triangle_vertices = [face_vertices[0], face_vertices[i], face_vertices[i + 1]];
                        
                        for &vertex_key in &triangle_vertices {
                            if let Some(&existing_local_index) = vertex_map.get(&vertex_key) {
                                all_indices.push(vertex_offset + existing_local_index);
                            } else {
                                if let Some(position) = openmodel_mesh.vertex_position(vertex_key) {
                                    let pipe_vertex = PipeVertex {
                                        position: [position.x as f32, position.y as f32, position.z as f32],
                                        color: segment.color,
                                    };
                                    
                                    all_vertices.push(pipe_vertex);
                                    vertex_map.insert(vertex_key, next_local_index);
                                    all_indices.push(vertex_offset + next_local_index);
                                    next_local_index += 1;
                                }
                            }
                        }
                    }
                }
            }
            
            vertex_offset += next_local_index;
        }
        
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} Vertex Buffer", name)),
            contents: bytemuck::cast_slice(&all_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} Index Buffer", name)),
            contents: bytemuck::cast_slice(&all_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        Self {
            name: String::from(name),
            vertex_buffer,
            index_buffer,
            num_indices: all_indices.len() as u32,
        }
    }

    /// Create a PipeModel from an OpenModel Line
    #[allow(dead_code)]
    pub fn from_openmodel_line(device: &wgpu::Device, name: &str, line: &OpenModelLine) -> Self {
        let pipe_segment = PipeSegment::from_openmodel_line(line);
        Self::new(device, name, &[pipe_segment])
    }

    /// Create a PipeModel from a collection of OpenModel Lines
    pub fn from_openmodel_lines(device: &wgpu::Device, name: &str, lines: &[OpenModelLine]) -> Self {
        let pipe_segments: Vec<PipeSegment> = lines.iter()
            .map(|line| PipeSegment::from_openmodel_line(line))
            .collect();
        Self::new(device, name, &pipe_segments)
    }

    /// Create a PipeModel from an OpenModel Line with specified color and radius
    #[allow(dead_code)]
    pub fn from_openmodel_line_with_params(device: &wgpu::Device, name: &str, line: &OpenModelLine, color: &OpenModelColor, radius: f32) -> Self {
        let pipe_segment = PipeSegment::from_openmodel_line_with_params(line, color, radius);
        Self::new(device, name, &[pipe_segment])
    }
}

#[allow(dead_code)]
pub trait DrawPipes<'a> {
    fn draw_pipes(
        &mut self,
        pipe_model: &'a PipeModel,
        camera_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b: 'a> DrawPipes<'a> for wgpu::RenderPass<'b> {
    fn draw_pipes(
        &mut self,
        pipe_model: &'a PipeModel,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, pipe_model.vertex_buffer.slice(..));
        self.set_index_buffer(pipe_model.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, camera_bind_group, &[]);
        self.draw_indexed(0..pipe_model.num_indices, 0, 0..1);
    }
}
