//! # Geometry Loader Module
//! 
//! This module provides functionality to load geometry data from JSON files.
//! It supports loading meshes, lines, points, pipes, and polygons from a
//! standardized JSON format.

use std::fs::File;
use std::io::BufReader;
use serde::{Deserialize, Serialize};
use wgpu::util::DeviceExt;
use cfg_if::cfg_if;

use crate::model::{Mesh, Model, ModelVertex};

use crate::model_point::{PointVertex, QuadPointModel};
use crate::model_pipe::{PipeSegment, PipeModel};
use crate::model_polygon::{PolygonVertex, PolygonModel};
// Texture module no longer used

// Helper functions for tangent space calculation

/// Calculate a default tangent vector perpendicular to the given normal
fn calculate_default_tangent(normal: &[f32; 3]) -> [f32; 3] {
    // Choose an arbitrary axis to cross with the normal
    // If normal is close to Y-axis, use Z-axis instead
    let up = if normal[1] > 0.99 || normal[1] < -0.99 {
        [0.0, 0.0, 1.0]  // Use Z-axis if normal is close to Y-axis
    } else {
        [0.0, 1.0, 0.0]  // Otherwise use Y-axis
    };
    
    // Cross product to find perpendicular vector
    let tangent = cross_product(&up, normal);
    normalize(&tangent)
}

/// Calculate a default bitangent from normal and tangent
fn calculate_default_bitangent(normal: &[f32; 3], tangent: &[f32; 3]) -> [f32; 3] {
    // Bitangent is perpendicular to both normal and tangent
    let bitangent = cross_product(normal, tangent);
    normalize(&bitangent)
}

/// Simple cross product implementation
fn cross_product(a: &[f32; 3], b: &[f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Normalize a vector to unit length
fn normalize(v: &[f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length > 0.0001 {
        [
            v[0] / length,
            v[1] / length,
            v[2] / length,
        ]
    } else {
        [0.0, 0.0, 0.0] // Avoid division by zero
    }
}

// Main structure that contains all geometry data from JSON
#[derive(Serialize, Deserialize, Debug)]
pub struct GeometryData {
    pub metadata: Metadata,
    pub meshes: Option<Vec<MeshData>>,
    pub points: Option<Vec<PointData>>,
    pub lines: Option<Vec<LineData>>,
    pub pipes: Option<Vec<PipeData>>,
    pub polygons: Option<Vec<PolygonData>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Metadata {
    pub version: String,
    pub description: String,
    pub created: String,
}

// Mesh Data Structures
#[derive(Serialize, Deserialize, Debug)]
pub struct MeshData {
    pub name: String,
    pub vertices: Vec<MeshVertexData>,
    pub indices: Vec<u32>,
    pub material: Option<MaterialData>,
    pub face_colors: Option<Vec<[f32; 3]>>, // Add optional face colors array
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MeshVertexData {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
    pub tangent: Option<[f32; 3]>,    // Made optional
    pub bitangent: Option<[f32; 3]>,  // Made optional
    pub color: Option<[f32; 3]>, // Add optional per-vertex color
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MaterialData {
    pub name: String,
    pub diffuse_texture: String,
    pub normal_texture: String,
}

// Point Data Structures
#[derive(Serialize, Deserialize, Debug)]
pub struct PointData {
    pub name: String,
    pub vertices: Vec<PointVertexData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PointVertexData {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub size: f32,
}

// Line Data Structures
#[derive(Serialize, Deserialize, Debug)]
pub struct LineData {
    pub name: String,
    pub vertices: Vec<LineVertexData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LineVertexData {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

// Pipe Data Structures
#[derive(Serialize, Deserialize, Debug)]
pub struct PipeData {
    pub name: String,
    pub segments: Vec<PipeSegmentData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PipeSegmentData {
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub color: [f32; 3],
    pub radius: f32,
}

// Polygon Data Structures
#[derive(Serialize, Deserialize, Debug)]
pub struct PolygonData {
    pub name: String,
    pub polygons: Vec<PolygonMeshData>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PolygonMeshData {
    pub vertices: Vec<PolygonVertexData>,
    pub indices: Vec<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PolygonVertexData {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

#[cfg(target_arch = "wasm32")]
fn format_url(file_name: &str) -> reqwest::Url {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let origin = location.origin().unwrap();
    // Use the assets folder for geometry files in WebAssembly
    let base = reqwest::Url::parse(&format!("{}/assets/", origin,)).unwrap();
    base.join(file_name).unwrap()
}

/// Load geometry data from a JSON file
pub async fn load_geometry_file(path: &str) -> Result<GeometryData, Box<dyn std::error::Error>> {
    cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            // For WASM, extract just the filename from the path
            let file_name = if path.starts_with("assets/") {
                &path[7..] // Remove "assets/" prefix
            } else {
                path
            };
            let url = format_url(file_name);
            let json_text = reqwest::get(url)
                .await?
                .text()
                .await?;
            let geometry_data: GeometryData = serde_json::from_str(&json_text)?;
        } else {
            // For native, use the full path as-is
            let file_path = std::path::Path::new(path);
            let file = File::open(file_path)?;
            let reader = BufReader::new(file);
            let geometry_data: GeometryData = serde_json::from_reader(reader)?;
        }
    }
    Ok(geometry_data)
}

/// Convert JSON mesh data to a Model
pub fn create_model_from_mesh_data(
    device: &wgpu::Device, 
    _queue: &wgpu::Queue,  // Kept for compatibility but unused
    mesh_data: &MeshData,
    _texture_bind_group_layout: &wgpu::BindGroupLayout  // Kept for compatibility but unused
) -> Result<Model, Box<dyn std::error::Error>> {
    let mut meshes = Vec::new();
    // Materials removed - not needed for texture-free pipeline
    
    // Convert vertices with color handling
    let mut vertices: Vec<ModelVertex> = mesh_data.vertices.iter()
        .map(|v| {
            // Default tangent space vectors based on normal
            // These are arbitrary but consistent given a normal
            let default_tangent = calculate_default_tangent(&v.normal);
            let default_bitangent = calculate_default_bitangent(&v.normal, &default_tangent);
            
            ModelVertex {
                position: v.position,
                tex_coords: v.tex_coords,
                normal: v.normal,
                tangent: v.tangent.unwrap_or(default_tangent),  // Use default if not provided
                bitangent: v.bitangent.unwrap_or(default_bitangent),  // Use default if not provided
                color: v.color.unwrap_or([0.7, 0.7, 0.7]), // Default color if not provided
            }
        })
        .collect();
    
    // Handle per-face colors if provided
    if let Some(face_colors) = &mesh_data.face_colors {
        if face_colors.len() * 3 <= mesh_data.indices.len() / 3 {
            // Apply face colors to vertices
            for (face_idx, color) in face_colors.iter().enumerate() {
                let idx_base = face_idx * 3; // Each face has 3 vertices
                if idx_base + 2 < mesh_data.indices.len() {
                    // Get the three vertex indices for this face
                    let v1_idx = mesh_data.indices[idx_base] as usize;
                    let v2_idx = mesh_data.indices[idx_base + 1] as usize;
                    let v3_idx = mesh_data.indices[idx_base + 2] as usize;
                    
                    // Apply the face color to all three vertices
                    if v1_idx < vertices.len() { vertices[v1_idx].color = *color; }
                    if v2_idx < vertices.len() { vertices[v2_idx].color = *color; }
                    if v3_idx < vertices.len() { vertices[v3_idx].color = *color; }
                }
            }
        }
    }
    
    // Create vertex buffer
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("{} Vertex Buffer", mesh_data.name)),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    
    // Create index buffer
    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(&format!("{} Index Buffer", mesh_data.name)),
        contents: bytemuck::cast_slice(&mesh_data.indices),
        usage: wgpu::BufferUsages::INDEX,
    });
    
    let mesh = Mesh {
        _name: mesh_data.name.clone(),
        vertex_buffer,
        index_buffer,
        num_elements: mesh_data.indices.len() as u32,
        // material field removed - not needed for texture-free pipeline
    };
    
    meshes.push(mesh);
    
    Ok(Model { meshes })
}

/// Convert JSON point data to a QuadPointModel
pub fn create_quad_point_model_from_point_data(
    device: &wgpu::Device,
    point_data: &PointData
) -> QuadPointModel {
    // Convert point data to PointVertex format
    let points: Vec<PointVertex> = point_data.vertices.iter()
        .map(|v| PointVertex {
            position: v.position,
            color: v.color,
            size: v.size,
        })
        .collect();
    
    // Create QuadPointModel
    QuadPointModel::new(device, &point_data.name, &points)
}



/// Convert JSON pipe data to a PipeModel
pub fn create_pipe_model_from_pipe_data(
    device: &wgpu::Device,
    pipe_data: &PipeData
) -> PipeModel {
    println!("DEBUG: Converting {} pipe segments from JSON", pipe_data.segments.len());
    // Convert pipe segment data to PipeSegment format
    let segments: Vec<PipeSegment> = pipe_data.segments.iter()
        .map(|s| PipeSegment {
            start: s.start,
            end: s.end,
            color: s.color,
            radius: s.radius,
        })
        .collect();
    
    // Create PipeModel
    PipeModel::new(device, &pipe_data.name, &segments)
}

/// Convert JSON polygon data to a PolygonModel
pub fn create_polygon_model_from_polygon_data(
    device: &wgpu::Device,
    polygon_data: &PolygonData
) -> PolygonModel {
    // Convert all polygons to flat lists of vertices and indices
    let mut all_vertices = Vec::new();
    let mut all_indices = Vec::new();
    let mut vertex_offset = 0;
    
    for polygon in &polygon_data.polygons {
        // Convert polygon vertex data to PolygonVertex format
        let vertices: Vec<PolygonVertex> = polygon.vertices.iter()
            .map(|v| PolygonVertex {
                position: v.position,
                color: v.color,
            })
            .collect();
        
        // Add vertices to global list
        all_vertices.extend(vertices);
        
        // Adjust indices to account for the offset
        for &index in &polygon.indices {
            all_indices.push(index + vertex_offset);
        }
        
        vertex_offset += polygon.vertices.len() as u32;
    }
    
    // Create PolygonModel
    PolygonModel::new(device, &polygon_data.name, &all_vertices, &all_indices)
}
