//! # Polygon Model Module
//! 
//! This module provides functionality for rendering polygons from vertex lists.
//! It defines data structures and traits for storing and rendering collections
//! of polygon vertices with position and color attributes.
//! 
//! OpenModel Integration:
//! - Integrates OpenModel Pline (polyline) geometry for polygon representation
//! - Converts OpenModel Point coordinates (f64) to GPU vertex format (f32)
//! - Handles color conversion from OpenModel Color (0-255) to GPU (0.0-1.0)

use wgpu::util::DeviceExt;
// Only import what we need from cgmath
// No cgmath imports needed in this module

// OpenModel imports for polygon geometry
use openmodel::geometry::Pline as OpenModelPline;
use openmodel::geometry::Point as OpenModelPoint;
use openmodel::primitives::Color as OpenModelColor;

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
    pub fn from_polygon_list(
        device: &wgpu::Device,
        name: &str,
        polygons: &[Vec<[f32; 3]>],
        colors: &[[f32; 3]],
    ) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut vertex_offset = 0u32;

        for (polygon, color) in polygons.iter().zip(colors.iter()) {
            // Add vertices for this polygon
            for position in polygon {
                vertices.push(PolygonVertex {
                    position: *position,
                    color: *color,
                });
            }

            // Triangulate the polygon using fan triangulation
            if polygon.len() >= 3 {
                for i in 1..polygon.len() - 1 {
                    indices.push(vertex_offset);
                    indices.push(vertex_offset + i as u32);
                    indices.push(vertex_offset + (i + 1) as u32);
                }
            }

            vertex_offset += polygon.len() as u32;
        }

        Self::new(device, name, &vertices, &indices)
    }

    /// Create a PolygonModel from an OpenModel Pline (polyline)
    /// Converts OpenModel Point coordinates (f64) to GPU vertex format (f32)
    pub fn from_openmodel_pline(device: &wgpu::Device, name: &str, pline: &OpenModelPline) -> Self {
        let color = if pline.data.has_color() {
            let color_data = pline.data.get_color();
            [color_data[0] as f32 / 255.0, color_data[1] as f32 / 255.0, color_data[2] as f32 / 255.0]
        } else {
            [1.0, 1.0, 1.0] // Default white
        };

        let positions: Vec<[f32; 3]> = pline.points.iter()
            .map(|point| [point.x as f32, point.y as f32, point.z as f32])
            .collect();

        Self::from_positions(device, name, &positions, color)
    }

    /// Create a PolygonModel from multiple OpenModel Plines
    pub fn from_openmodel_plines(device: &wgpu::Device, name: &str, plines: &[OpenModelPline]) -> Self {
        let mut polygons = Vec::new();
        let mut colors = Vec::new();

        for pline in plines {
            let color = if pline.data.has_color() {
                let color_data = pline.data.get_color();
                [color_data[0] as f32 / 255.0, color_data[1] as f32 / 255.0, color_data[2] as f32 / 255.0]
            } else {
                [1.0, 1.0, 1.0] // Default white
            };

            let positions: Vec<[f32; 3]> = pline.points.iter()
                .map(|point| [point.x as f32, point.y as f32, point.z as f32])
                .collect();

            polygons.push(positions);
            colors.push(color);
        }

        Self::from_polygon_list(device, name, &polygons, &colors)
    }

    /// Create a PolygonModel from OpenModel Pline with custom color override
    pub fn from_openmodel_pline_with_color(device: &wgpu::Device, name: &str, pline: &OpenModelPline, color: &OpenModelColor) -> Self {
        let (r, g, b, _a) = color.to_float();
        let gpu_color = [r, g, b];

        let positions: Vec<[f32; 3]> = pline.points.iter()
            .map(|point| [point.x as f32, point.y as f32, point.z as f32])
            .collect();

        Self::from_positions(device, name, &positions, gpu_color)
    }
}

pub trait DrawPolygons<'a> {
    fn draw_polygons(
        &mut self,
        polygon_model: &'a PolygonModel,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b: 'a> DrawPolygons<'a> for wgpu::RenderPass<'b> {
    fn draw_polygons(
        &mut self,
        polygon_model: &'a PolygonModel,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, polygon_model.vertex_buffer.slice(..));
        self.set_index_buffer(polygon_model.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, camera_bind_group, &[]);
        self.set_bind_group(1, light_bind_group, &[]);
        self.draw_indexed(0..polygon_model.num_indices, 0, 0..1);
    }
}
