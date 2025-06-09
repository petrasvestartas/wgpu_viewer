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

/// Generates point cloud vertices for a series of cube instances
#[allow(dead_code)]
pub fn generate_point_cloud(instances: &[Instance]) -> Vec<PointVertex> {
    println!("DEBUG: Creating point clouds for {} cube instances", instances.len());
    
    let mut point_vertices = Vec::new();
    
    // Define a small local grid for each instance
    let local_grid_size = 10; // Points along each axis per cube
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
                        size: 1.5, // Small size for dense appearance
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
