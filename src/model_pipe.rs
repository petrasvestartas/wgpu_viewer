//! # Pipe Model Module
//! 
//! This module provides functionality for rendering 3D lines as cylindrical pipes.
//! It defines data structures and traits for storing and rendering collections
//! of 3D pipe segments with position, color and radius attributes.
//!
//! Key components:
//! - `PipeVertex`: Vertex structure for pipes with position and color
//! - `PipeSegment`: Definition of a pipe segment with start, end, color and radius
//! - `PipeModel`: A collection of pipe segments rendered as 3D cylinders
//! - `DrawPipes` trait: Rendering abstraction for pipe collections

use wgpu::util::DeviceExt;
use cgmath::*;

// Configuration constants
pub const PIPE_RADIUS: f32 = 0.05;  // Default pipe radius/thickness
pub const PIPE_COLOR: [f32; 3] = [0.0, 0.0, 0.0];  // Bright red for debugging
pub const PIPE_RESOLUTION: u32 = 12;  // Number of sides for cylinder approximation

// Pipe segment definition
#[derive(Debug, Clone)]
pub struct PipeSegment {
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub color: [f32; 3],
    pub radius: f32,
}

// Vertex structure for cylinders - simplified for flat color shader
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PipeVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl PipeVertex {
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
        resolution: u32, // Number of sides for each cylinder
    ) -> Self {
        // Generate vertices and indices for all pipe segments
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut index_offset = 0;
        
        for segment in pipe_segments {
            let (segment_vertices, mut segment_indices) = 
                create_cylinder_for_pipe(segment, resolution);
            
            // Adjust indices to account for the offset
            for index in &mut segment_indices {
                *index += index_offset;
            }
            
            index_offset = vertices.len() as u32;
            vertices.extend(segment_vertices);
            indices.extend(segment_indices);
        }
        
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} Vertex Buffer", name)),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} Index Buffer", name)),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        Self {
            name: String::from(name),
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
        }
    }
}

// Create a cylinder mesh along a pipe segment with consistent counter-clockwise winding
fn create_cylinder_for_pipe(
    segment: &PipeSegment,
    sides: u32,
) -> (Vec<PipeVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    
    // Direction vector from start to end
    let start = Vector3::from(segment.start);
    let end = Vector3::from(segment.end);
    let direction = end - start;
    
    // Skip if the pipe has zero length
    if direction.magnitude() < 1e-6 {
        return (vertices, indices);
    }
    
    // Create an orthonormal basis for the cylinder
    let axis = direction.normalize();
    
    // Find perpendicular vectors for the cylinder cross section
    // Using a straightforward and stable approach to pick the perpendicular vector
    let perpendicular = if axis.z.abs() < 0.9 {
        // If not nearly parallel with Z, use Z-axis for cross product
        Vector3::new(0.0, 0.0, 1.0).cross(axis).normalize()
    } else {
        // Otherwise use X-axis for cross product
        Vector3::new(1.0, 0.0, 0.0).cross(axis).normalize()
    };
    
    // Now get the third basis vector with another cross product
    let binormal = axis.cross(perpendicular).normalize();
    
    // --- CONSISTENT APPROACH: ALWAYS USE CCW FOR ALL VERTICES ---
    
    // Add bottom cap center vertex (#0 in our vertex list)
    vertices.push(PipeVertex {
        position: [start.x, start.y, start.z],
        color: segment.color,
    });
    
    // Add bottom cap rim vertices (#1 to #sides in our vertex list)
    for i in 0..sides {
        let angle = i as f32 * 2.0 * std::f32::consts::PI / sides as f32;
        let x = angle.cos();
        let y = angle.sin();
        
        let point = start + segment.radius * (x * perpendicular + y * binormal);
        
        vertices.push(PipeVertex {
            position: [point.x, point.y, point.z],
            color: segment.color,
        });
    }
    
    // Add top cap center vertex (#sides+1 in our vertex list)
    vertices.push(PipeVertex {
        position: [end.x, end.y, end.z],
        color: segment.color,
    });
    
    // Add top cap rim vertices (#sides+2 to #sides*2+1 in our vertex list)
    for i in 0..sides {
        let angle = i as f32 * 2.0 * std::f32::consts::PI / sides as f32;
        let x = angle.cos();
        let y = angle.sin();
        
        let point = end + segment.radius * (x * perpendicular + y * binormal);
        
        vertices.push(PipeVertex {
            position: [point.x, point.y, point.z],
            color: segment.color,
        });
    }
    
    // Bottom cap triangles (CCW when viewed from outside, which is below)
    for i in 0..sides {
        let idx1 = 1 + i;
        let idx2 = 1 + (i + 1) % sides;
        
        indices.push(0);       // Center
        indices.push(idx2);    // Next rim point
        indices.push(idx1);    // Current rim point
    }
    
    // Top cap triangles (CCW when viewed from outside, which is above)
    let top_center_idx = 1 + sides;
    for i in 0..sides {
        let idx1 = top_center_idx + 1 + i;
        let idx2 = top_center_idx + 1 + (i + 1) % sides;
        
        indices.push(top_center_idx); // Center
        indices.push(idx1);           // Current rim point
        indices.push(idx2);           // Next rim point
    }
    
    // Side triangles - using the rim vertices we already created
    for i in 0..sides {
        let bottom_curr = 1 + i;
        let bottom_next = 1 + (i + 1) % sides;
        let top_curr = top_center_idx + 1 + i;
        let top_next = top_center_idx + 1 + (i + 1) % sides;
        
        // Each rectangular side consists of two triangles with FIXED winding order
        // Triangle 1: Counter-clockwise when viewed from outside
        indices.push(bottom_curr);
        indices.push(bottom_next);
        indices.push(top_curr);
        
        // Triangle 2: Counter-clockwise when viewed from outside
        indices.push(top_curr);
        indices.push(bottom_next);
        indices.push(top_next);
    }
    
    (vertices, indices)
}

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
