//! # Polygon Model Module
//! 
//! This module provides functionality for rendering polygons from vertex lists.
//! It defines data structures and traits for storing and rendering collections
//! of polygon vertices with position and color attributes.

use wgpu::util::DeviceExt;
// Only import what we need from cgmath
// No cgmath imports needed in this module

// Configuration constants
#[allow(dead_code)]
pub const POLYGON_COLOR: [f32; 3] = [0.5, 0.5, 0.5];  // Default gray color

// Polygon vertex definition
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PolygonVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl PolygonVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<PolygonVertex>() as wgpu::BufferAddress,
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

pub struct PolygonModel {
    #[allow(dead_code)]
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

impl PolygonModel {
    pub fn new(
        device: &wgpu::Device, 
        name: &str, 
        vertices: &[PolygonVertex],
        indices: &[u32],
    ) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} Vertex Buffer", name)),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{} Index Buffer", name)),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        Self {
            name: String::from(name),
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
        }
    }
    
    // Convenience method to create a polygon from a simple list of positions and a color
    #[allow(dead_code)]
    pub fn from_positions(
        device: &wgpu::Device,
        name: &str,
        positions: &[[f32; 3]],
        color: [f32; 3],
    ) -> Self {
        let vertices: Vec<PolygonVertex> = positions.iter()
            .map(|&pos| PolygonVertex { position: pos, color })
            .collect();
        
        // For simple polygons, create a triangle fan
        // Assumes the polygon is convex and the vertices are in order
        let mut indices = Vec::new();
        if positions.len() >= 3 {
            for i in 1..(positions.len() as u32 - 1) {
                indices.push(0);  // Center vertex
                indices.push(i);  // Current vertex
                indices.push(i + 1); // Next vertex
            }
        }
        
        Self::new(device, name, &vertices, &indices)
    }
    
    // Create a model for multiple polygons
    #[allow(dead_code)]
    pub fn from_polygon_list(
        device: &wgpu::Device,
        name: &str,
        polygons: &[Vec<[f32; 3]>],
        colors: &[[f32; 3]],
    ) -> Self {
        let mut all_vertices = Vec::new();
        let mut all_indices = Vec::new();
        let mut vertex_offset = 0;
        
        for (i, polygon) in polygons.iter().enumerate() {
            let color = if i < colors.len() { colors[i] } else { POLYGON_COLOR };
            
            // Add vertices for this polygon
            for &pos in polygon {
                all_vertices.push(PolygonVertex { position: pos, color });
            }
            
            // Add indices for triangulation (simple triangle fan)
            if polygon.len() >= 3 {
                for j in 1..(polygon.len() - 1) {
                    all_indices.push(vertex_offset); // First vertex as center
                    all_indices.push(vertex_offset + j as u32); // Current vertex
                    all_indices.push(vertex_offset + j as u32 + 1); // Next vertex
                }
            }
            
            vertex_offset += polygon.len() as u32;
        }
        
        Self::new(device, name, &all_vertices, &all_indices)
    }
}

pub trait DrawPolygons<'a> {
    fn draw_polygons(
        &mut self,
        polygon_model: &'a PolygonModel,
        camera_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b: 'a> DrawPolygons<'a> for wgpu::RenderPass<'b> {
    fn draw_polygons(
        &mut self,
        polygon_model: &'a PolygonModel,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, polygon_model.vertex_buffer.slice(..));
        self.set_index_buffer(polygon_model.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, camera_bind_group, &[]);
        self.draw_indexed(0..polygon_model.num_indices, 0, 0..1);
    }
}
