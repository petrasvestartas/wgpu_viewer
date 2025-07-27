//! # Mesh Model Module
//! 
//! This module provides functionality for handling 3D mesh models using OpenModel geometry.
//! It defines data structures and traits for loading, storing, and rendering
//! polygonal 3D models with materials, textures, and lighting support.
//!
//! Key components:
//! - `ModelVertex`: GPU vertex structure for 3D meshes with positions, normals, etc.
//! - `Material`: Represents surface properties with texture maps
//! - `Mesh`: A single mesh with vertices and indices
//! - `Model`: A collection of meshes with materials
//! - `DrawModel` & `DrawLight` traits: Rendering abstractions for meshes
//! - OpenModel integration: Bridge between OpenModel Mesh and GPU structures

use wgpu::util::DeviceExt;
use openmodel::geometry::Mesh as OpenModelMesh;
use openmodel::primitives::Color as OpenModelColor;

// Texture module no longer used

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
    pub tangent: [f32; 3],
    pub bitangent: [f32; 3],
    pub color: [f32; 3],
}

impl ModelVertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as wgpu::BufferAddress,
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
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 3,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 11]>() as wgpu::BufferAddress,
                    shader_location: 4,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 14]>() as wgpu::BufferAddress,
                    shader_location: 12,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

/// A common trait for all vertex types that can be used with WGPU rendering.
/// 
/// This trait provides a single method `desc()` that returns the vertex buffer layout
/// required by the GPU pipeline to interpret the vertex data correctly.
pub trait Vertex {
    /// Returns the buffer layout description for this vertex type
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

// Material struct removed - not needed for texture-free pipeline

pub struct Mesh {
    pub _name: String, // Using underscore to indicate unused field
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    // material field removed - not needed for texture-free pipeline
}

pub struct Model {
    pub meshes: Vec<Mesh>,
    // materials field removed - not needed for texture-free pipeline
}

impl Mesh {
    /// Create a new Mesh from vertices and indices
    pub fn new(device: &wgpu::Device, name: &str, vertices: &[ModelVertex], indices: &[u32]) -> Self {
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
            _name: name.to_string(),
            vertex_buffer,
            index_buffer,
            num_elements: indices.len() as u32,
        }
    }

    /// Create a Mesh from an OpenModel Mesh
    pub fn from_openmodel_mesh(device: &wgpu::Device, name: &str, openmodel_mesh: &OpenModelMesh) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut vertex_map = std::collections::HashMap::new();
        let mut next_index = 0u32;

        // Get vertex normals from OpenModel mesh
        let vertex_normals = openmodel_mesh.vertex_normals();

        // Convert OpenModel mesh to GPU format
        for (face_key, face_vertices) in openmodel_mesh.get_face_data() {
            // Triangulate the face (assuming it's a polygon)
            if face_vertices.len() >= 3 {
                // For each triangle in the face (fan triangulation)
                for i in 1..face_vertices.len() - 1 {
                    let triangle_vertices = [face_vertices[0], face_vertices[i], face_vertices[i + 1]];
                    
                    for &vertex_key in &triangle_vertices {
                        if let Some(&existing_index) = vertex_map.get(&vertex_key) {
                            indices.push(existing_index);
                        } else {
                            // Get vertex position
                            if let Some(position) = openmodel_mesh.vertex_position(vertex_key) {
                                // Get vertex normal (default to up if not available)
                                let normal = vertex_normals.get(&vertex_key)
                                    .map(|n| [n.x as f32, n.y as f32, n.z as f32])
                                    .unwrap_or([0.0, 0.0, 1.0]);

                                // Get color from mesh data (default to white)
                                let color = if openmodel_mesh.data.has_color() {
                                    let color_data = openmodel_mesh.data.get_color();
                                    [color_data[0] as f32 / 255.0, color_data[1] as f32 / 255.0, color_data[2] as f32 / 255.0]
                                } else {
                                    [1.0, 1.0, 1.0] // Default white
                                };

                                let model_vertex = ModelVertex {
                                    position: [position.x as f32, position.y as f32, position.z as f32],
                                    tex_coords: [0.0, 0.0], // Default texture coordinates
                                    normal,
                                    tangent: [1.0, 0.0, 0.0], // Default tangent
                                    bitangent: [0.0, 1.0, 0.0], // Default bitangent
                                    color,
                                };

                                vertices.push(model_vertex);
                                vertex_map.insert(vertex_key, next_index);
                                indices.push(next_index);
                                next_index += 1;
                            }
                        }
                    }
                }
            }
        }

        Self::new(device, name, &vertices, &indices)
    }
}

impl Model {
    /// Create a new Model from a collection of meshes
    pub fn new(meshes: Vec<Mesh>) -> Self {
        Self { meshes }
    }

    /// Create a Model from an OpenModel Mesh (single mesh)
    pub fn from_openmodel_mesh(device: &wgpu::Device, name: &str, openmodel_mesh: &OpenModelMesh) -> Self {
        let mesh = Mesh::from_openmodel_mesh(device, name, openmodel_mesh);
        Self::new(vec![mesh])
    }

    /// Create a Model from multiple OpenModel Meshes
    pub fn from_openmodel_meshes(device: &wgpu::Device, openmodel_meshes: &[(String, OpenModelMesh)]) -> Self {
        let meshes: Vec<Mesh> = openmodel_meshes.iter()
            .map(|(name, mesh)| Mesh::from_openmodel_mesh(device, name, mesh))
            .collect();
        Self::new(meshes)
    }
}

#[allow(dead_code)]
pub trait DrawModel<'a> {
    fn draw_mesh(
        &mut self,
        mesh: &'a Mesh,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        instances: std::ops::Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );

    fn draw_model(
        &mut self,
        model: &'a Model,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_model_instanced(
        &mut self,
        model: &'a Model,
        instances: std::ops::Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(mesh, 0..1, camera_bind_group, light_bind_group);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        instances: std::ops::Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        // No material bind group - removed for texture-free pipeline
        self.set_bind_group(0, camera_bind_group, &[]);  // Camera at group 0
        self.set_bind_group(1, light_bind_group, &[]);   // Light at group 1
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_model(
        &mut self,
        model: &'b Model,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        self.draw_model_instanced(model, 0..1, camera_bind_group, light_bind_group);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: std::ops::Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'b wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            // Material access removed - not needed for texture-free pipeline
            self.draw_mesh_instanced(
                mesh,
                instances.clone(),
                camera_bind_group,
                light_bind_group,
            );
        }
    }


}

#[allow(dead_code)]
pub trait DrawLight<'a> {
    fn draw_light_mesh(
        &mut self,
        mesh: &'a Mesh, 
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    
    fn draw_light_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        instances: std::ops::Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    
    fn draw_light_model(
        &mut self,
        model: &'a Model,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    
    fn draw_light_model_instanced(
        &mut self,
        model: &'a Model,
        instances: std::ops::Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawLight<'a> for wgpu::RenderPass<'b>
where
    'b: 'a,
{
    fn draw_light_mesh(
        &mut self,
        mesh: &'a Mesh,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, camera_bind_group, &[]);
        self.set_bind_group(1, light_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, 0..1);
    }

    fn draw_light_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        instances: std::ops::Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, camera_bind_group, &[]);
        self.set_bind_group(1, light_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_light_model(
        &mut self,
        model: &'a Model,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, model.meshes[0].vertex_buffer.slice(..));
        self.set_index_buffer(
            model.meshes[0].index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        self.set_bind_group(0, camera_bind_group, &[]);
        self.set_bind_group(1, light_bind_group, &[]);
        self.draw_indexed(0..model.meshes[0].num_elements, 0, 0..1);
    }

    fn draw_light_model_instanced(
        &mut self,
        model: &'a Model,
        instances: std::ops::Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, model.meshes[0].vertex_buffer.slice(..));
        self.set_index_buffer(
            model.meshes[0].index_buffer.slice(..),
            wgpu::IndexFormat::Uint32,
        );
        self.set_bind_group(0, camera_bind_group, &[]);
        self.set_bind_group(1, light_bind_group, &[]);
        self.draw_indexed(0..model.meshes[0].num_elements, 0, instances);
    }
}


