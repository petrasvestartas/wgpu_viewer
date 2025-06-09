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
//! - `generate_point_cloud`: Utility function to generate point clouds from instances

use wgpu::util::DeviceExt;
use crate::instance::Instance;
// use cgmath::prelude::*;  // Not currently used

// Configuration constants
pub const POINT_SIZE: f32 = 0.02;  // Default point size

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PointVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub size: f32,
}

/// Billboard vertex for rendering points as camera-facing quads
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadPointVertex {
    pub position: [f32; 3],      // Center position of the point
    pub color: [f32; 3],        // Color of the point
    pub corner: [f32; 2],       // Corner offset (-1,-1 to 1,1)
    pub size: f32,              // Size of the point
}

#[allow(dead_code)]
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

#[allow(dead_code)]
impl QuadPointVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<QuadPointVertex>() as wgpu::BufferAddress,
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
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32,
                },
            ],
        }
    }
    
    /// Converts a single point into 4 quad vertices (for a billboarded point)
    pub fn from_point(point: &PointVertex) -> [Self; 4] {
        // Create the 4 corners of a quad
        let corners = [
            [-1.0f32, -1.0f32], // Bottom-left
            [ 1.0f32, -1.0f32], // Bottom-right
            [-1.0f32,  1.0f32], // Top-left
            [ 1.0f32,  1.0f32], // Top-right
        ];
        
        // Map each corner to a QuadPointVertex
        corners.map(|corner| Self {
            position: point.position,
            color: point.color,
            corner,
            size: point.size,
        })
    }
    
    /// Converts a slice of points into quad vertices
    pub fn points_to_quads(points: &[PointVertex]) -> Vec<Self> {
        let mut quad_vertices = Vec::with_capacity(points.len() * 4);
        
        for point in points {
            let quad_verts = Self::from_point(point);
            quad_vertices.extend_from_slice(&quad_verts);
        }
        
        quad_vertices
    }
}

pub struct PointModel {
    pub _name: String, // Using underscore to indicate unused field
    pub vertex_buffer: wgpu::Buffer,
    pub num_vertices: u32,
}

pub struct QuadPointModel {
    pub _name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub num_vertices: u32,
    pub indices: Option<wgpu::Buffer>,
    pub num_indices: u32,
}

#[allow(dead_code)]
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

    /// Convert this point model into a QuadPointModel for billboard rendering
    pub fn to_quad_model(self, device: &wgpu::Device) -> QuadPointModel {
        // Extract point vertices from buffer - this is inefficient but works for demonstration
        // In production code, you would keep the original vertices around
        // This is just to show the concept
        let point_count = self.num_vertices as usize;
        let placeholder_points = vec![PointVertex {
            position: [0.0, 0.0, 0.0],
            color: [1.0, 1.0, 1.0],
            size: 5.0,
        }; point_count];
        
        // Convert points to quad vertices
        let quad_vertices = QuadPointVertex::points_to_quads(&placeholder_points);
        
        // Create indices for the quads (2 triangles per quad)
        let mut indices = Vec::with_capacity(point_count * 6);
        
        // For each point, create 2 triangles (6 indices)
        for i in 0..point_count {
            let base = (i * 4) as u16;
            // First triangle (bottom-left, bottom-right, top-left)
            indices.push(base + 0);
            indices.push(base + 1);
            indices.push(base + 2);
            // Second triangle (bottom-right, top-right, top-left)
            indices.push(base + 1);
            indices.push(base + 3);
            indices.push(base + 2);
        }
        
        // Create buffers
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad Point Vertex Buffer"),
            contents: bytemuck::cast_slice(&quad_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Quad Point Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        QuadPointModel {
            _name: self._name,
            vertex_buffer,
            num_vertices: quad_vertices.len() as u32,
            indices: Some(index_buffer),
            num_indices: indices.len() as u32,
        }
    }
}

#[allow(dead_code)]
impl QuadPointModel {
    pub fn new(device: &wgpu::Device, name: &str, points: &[PointVertex]) -> Self {
        // Convert points to quad vertices
        let quad_vertices = QuadPointVertex::points_to_quads(points);
        
        // Create indices for the quads (2 triangles per quad)
        let mut indices: Vec<u32> = Vec::with_capacity(points.len() * 6);
        
        // For each point, create 2 triangles (6 indices)
        for i in 0..points.len() {
            let base = (i * 4) as u32; // Use u32 instead of u16 to support more vertices
            // First triangle (bottom-left, bottom-right, top-left)
            indices.push(base + 0);
            indices.push(base + 1);
            indices.push(base + 2);
            // Second triangle (bottom-right, top-right, top-left)
            indices.push(base + 1);
            indices.push(base + 3);
            indices.push(base + 2);
        }
        
        // Create buffers
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} Quad Vertex Buffer", name)),
            contents: bytemuck::cast_slice(&quad_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} Quad Index Buffer", name)),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        Self {
            _name: String::from(name),
            vertex_buffer,
            num_vertices: quad_vertices.len() as u32,
            indices: Some(index_buffer),
            num_indices: indices.len() as u32,
        }
    }
}

#[allow(dead_code)]
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

/// A trait for drawing billboard-based points (rendered as quads)
#[allow(dead_code)]
pub trait DrawQuadPoints<'a, 'b>
where
    'b: 'a,
{
    fn draw_quad_points(
        &mut self,
        quad_model: &'b QuadPointModel,
        camera_bind_group: &'b wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawQuadPoints<'a, 'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_quad_points(
        &mut self,
        quad_model: &'b QuadPointModel,
        camera_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, quad_model.vertex_buffer.slice(..));
        self.set_bind_group(0, camera_bind_group, &[]);
        
        if let Some(index_buffer) = &quad_model.indices {
            self.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            self.draw_indexed(0..quad_model.num_indices, 0, 0..1);
        } else {
            self.draw(0..quad_model.num_vertices, 0..1);
        }
    }
}

/// Generates point cloud vertices for a series of cube instances
#[allow(dead_code)]
pub fn generate_point_cloud(instances: &[Instance]) -> Vec<PointVertex> {
    println!("DEBUG: Creating point clouds for {} cube instances", instances.len());
    
    let mut point_vertices = Vec::new();
    
    // Define a small local grid for each instance
    let local_grid_size = 22; // Points along each axis per cube (22^3 * 10^2 ≈ 10.6 million points)
    let local_grid_extent = 1.01; // Size of cube is 1.0 (-0.5 to +0.5)
    let step = (2.0 * local_grid_extent) / (local_grid_size as f32 - 1.0);
    
    // For each cube instance, create a small grid of points with the appropriate transformation
    for instance in instances {
        let pos = instance.position;
        let rotation = instance.rotation;
        
        // Convert the quaternion rotation to a 4x4 matrix
        let rotation_matrix = cgmath::Matrix4::from(rotation);
        
        // Create a grid of points for this instance
        for i in 0..local_grid_size {
            for j in 0..local_grid_size {
                for k in 0..local_grid_size {
                    // Calculate local position within the cube (-0.5 to 0.5)
                    let local_x = -local_grid_extent + (i as f32) * step;
                    let local_y = -local_grid_extent + (j as f32) * step;
                    let local_z = -local_grid_extent + (k as f32) * step;
                    
                    // Transform the point using the rotation matrix
                    let point_local = cgmath::Vector4::new(local_x, local_y, local_z, 1.0);
                    let point_rotated = rotation_matrix * point_local;
                    
                    // Final world position
                    let world_x = point_rotated.x + pos.x;
                    let world_y = point_rotated.y + pos.y;
                    let world_z = point_rotated.z + pos.z;
                    
                    // Color based on local position within the cube
                    let color_r = 0.0;
                    let color_g = ((local_y + 0.5) * 0.8).min(0.8); // Gradient from bottom to top
                    let color_b = 1.0;
                    
                    point_vertices.push(PointVertex {
                        position: [world_x, world_y, world_z],
                        color: [color_r, color_g, color_b],
                        size: POINT_SIZE, // Use the configurable point size
                    });
                }
            }
        }
        
        // Debug info for center cube
        if pos.x.abs() < 0.001 && pos.z.abs() < 0.001 {
            println!("DEBUG: Created point cloud grid for center cube at ({:.2}, {:.2}, {:.2})", 
                    pos.x, pos.y, pos.z);
        }
    }
    
    println!("DEBUG: Generated {} points across all cubes", point_vertices.len());
    
    point_vertices
}
