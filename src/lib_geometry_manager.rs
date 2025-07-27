use crate::{State, geometry_loader};
use crate::model_polygon::PolygonVertex;
use crate::model_pipe::PipeVertex;
use cgmath::prelude::*;
use wgpu::util::DeviceExt;

/// Load geometry data from a JSON file
pub async fn load_geometries_from_file(state: &mut State<'_>, path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("Loading geometries from file: {}", path);
    
    // Load geometry data from file
    let geometry_data = geometry_loader::load_geometry_file(path).await?;
    
    // Process mesh data if available
    if let Some(meshes) = &geometry_data.meshes {
        if !meshes.is_empty() {
            // Create an empty texture bind group layout since we removed all texture dependencies
            let texture_bind_group_layout = 
                state.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[], // No entries needed anymore as we removed textures
                    label: Some("texture_bind_group_layout"),
                });
            
            // Store all mesh models in a Vec
            let mut mesh_models = Vec::new();
            
            // Load all meshes from the JSON file
            for mesh_data in meshes {
                println!("Loading mesh: {}", mesh_data.name);
                
                // Create the model from each mesh data
                let model = geometry_loader::create_model_from_mesh_data(
                    &state.device,
                    &state.queue,
                    mesh_data,
                    &texture_bind_group_layout
                )?;
                
                mesh_models.push(model);
            }
            
            // For backwards compatibility, set the first model as obj_model
            if !mesh_models.is_empty() {
                state.obj_model = mesh_models.remove(0);
            }
            
            // Store additional models in a new field
            state.additional_mesh_models = mesh_models;
        }
    }
    
    // Process point data if available
    if let Some(points) = &geometry_data.points {
        if !points.is_empty() {
            // Load the first point cloud
            let first_point_set = &points[0];
            println!("Loading point cloud: {}", first_point_set.name);
            
            // Create the quad point model directly
            let quad_point_model = geometry_loader::create_quad_point_model_from_point_data(
                &state.device,
                first_point_set
            );
            
            // Use the model directly
            state.quad_point_model = Some(quad_point_model);
        }
    }
    
    // We don't load lines from JSON files as requested by the user
    // Lines are created directly in State::new using geometry_generator::create_grid_lines
    // This preserves the original XYZ grid with grey lines
    
    // Process pipe data if available
    if let Some(pipes) = &geometry_data.pipes {
        if !pipes.is_empty() {
            // Load the first pipe set
            let first_pipe_set = &pipes[0];
            println!("Loading pipes: {}", first_pipe_set.name);
            
            // Create the pipe model
            // Get raw vertices and indices from the geometry_loader
            let pipe_model = geometry_loader::create_pipe_model_from_pipe_data(
                &state.device,
                first_pipe_set
            );
            
            // Use the PipeModel directly since it's already in the correct format with vertex_buffer, index_buffer, and num_indices
            state.pipe_model = Some(pipe_model);
        }
    }
    
    // Process polygon data if available
    if let Some(polygons) = &geometry_data.polygons {
        if !polygons.is_empty() {
            // Load the first polygon set
            let first_polygon_set = &polygons[0];
            println!("Loading polygons: {}", first_polygon_set.name);
            
            // Create the polygon model
            // Get raw vertices and indices from the geometry_loader
            let polygon_model = geometry_loader::create_polygon_model_from_polygon_data(
                &state.device,
                first_polygon_set
            );
            
            // Use the PolygonModel directly since it's already in the correct format with vertex_buffer, index_buffer, and num_indices
            state.polygon_model = Some(polygon_model);
        }
    }
    
    Ok(())
}

/// Create a grid of polygons matching other geometries
pub fn create_sample_polygon(state: &mut State) {
    const SCALE_FACTOR: f32 = 0.25; // Size factor for polygon

    // Collect all polygon vertex data and indices
    let mut all_vertices = Vec::new();
    let mut all_indices = Vec::new();
    let mut vertex_count: u32 = 0;
    
    // Use the same instances stored in state.instances
    // This guarantees the same positions and rotations as other geometry
    println!("Creating polygon grid with {} instances", state.instances.len());
    
    // Create polygons at each instance position with the same rotation as other geometries
    for instance in &state.instances {
        let pos = instance.position;
        let rotation = instance.rotation;
        
        // Create a single color for the entire polygon based on its position
        // Use position to generate consistent colors
        let x_normalized = (pos.x + 15.0) / 30.0;  // Normalize x in [-15,15] to [0,1]
        let z_normalized = (pos.z + 15.0) / 30.0;  // Normalize z in [-15,15] to [0,1]
        let color = [
            x_normalized, 
            (1.0 - x_normalized) * z_normalized,
            1.0 - z_normalized,
        ];
        
        // Convert the quaternion rotation to a 4x4 matrix - EXACTLY like in line code
        let rotation_matrix = cgmath::Matrix4::from(rotation);
        
        // Define the same start/end points as the lines to ensure exact alignment
        // Lines use these exact coordinates
        let start_local = cgmath::Point3::new(0.0, -0.5, 0.0);
        let end_local = cgmath::Point3::new(0.0, 1.5, 0.0);
        
        // Create polygon vertices around the same vertical line
        let vertex_positions = [
            // Top vertex at the same position as the line end
            cgmath::Point3::new(0.0, end_local.y, 0.0),
            
            // Create points in a circle around the line at middle height
            cgmath::Point3::new(SCALE_FACTOR, 0.5, 0.0),
            cgmath::Point3::new(0.0, 0.5, SCALE_FACTOR),
            cgmath::Point3::new(-SCALE_FACTOR, 0.5, 0.0),
            cgmath::Point3::new(0.0, 0.5, -SCALE_FACTOR),
            
            // Bottom vertex at the same position as the line start
            cgmath::Point3::new(0.0, start_local.y, 0.0),
        ];
        
        // Transform vertices by rotation and position
        for &vertex_pos in &vertex_positions {
            // Apply rotation first
            let rotated_pos = rotation_matrix.transform_point(vertex_pos);
            // Then translate to instance position
            let final_pos = rotated_pos + cgmath::Vector3::new(pos.x, pos.y, pos.z);
            
            all_vertices.push(PolygonVertex {
                position: [final_pos.x, final_pos.y, final_pos.z],
                color,
            });
        }
        
        // Create indices for the polygon (using triangle fan approach)
        let base_idx = vertex_count;
        
        // Top triangles (connecting top vertex to side vertices)
        all_indices.extend_from_slice(&[
            base_idx, base_idx + 1, base_idx + 2,  // Top to first two side vertices
            base_idx, base_idx + 2, base_idx + 3,  // Top to second and third side vertices
            base_idx, base_idx + 3, base_idx + 4,  // Top to third and fourth side vertices
            base_idx, base_idx + 4, base_idx + 1,  // Top to fourth and first side vertices (wrap around)
        ]);
        
        // Bottom triangles (connecting bottom vertex to side vertices)
        all_indices.extend_from_slice(&[
            base_idx + 5, base_idx + 2, base_idx + 1,  // Bottom to first two side vertices (reversed winding)
            base_idx + 5, base_idx + 3, base_idx + 2,  // Bottom to second and third side vertices
            base_idx + 5, base_idx + 4, base_idx + 3,  // Bottom to third and fourth side vertices
            base_idx + 5, base_idx + 1, base_idx + 4,  // Bottom to fourth and first side vertices (wrap around)
        ]);
        
        vertex_count += vertex_positions.len() as u32;
    }
    
    println!("Created {} polygon vertices and {} indices", all_vertices.len(), all_indices.len());
    
    // Create vertex buffer
    let vertex_buffer = state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Polygon Vertex Buffer"),
        contents: bytemuck::cast_slice(&all_vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });
    
    // Create index buffer
    let index_buffer = state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Polygon Index Buffer"),
        contents: bytemuck::cast_slice(&all_indices),
        usage: wgpu::BufferUsages::INDEX,
    });
    
    // Create the polygon model
    let polygon_model = crate::model_polygon::PolygonModel {
        name: "Sample Polygon Grid".to_string(),
        vertex_buffer,
        index_buffer,
        num_indices: all_indices.len() as u32,
    };
    
    state.polygon_model = Some(polygon_model);
    println!("Sample polygon grid created successfully!");
}

/// Convert regular lines from line_model into 3D pipe lines
pub fn create_pipes_from_lines(state: &mut State) {
    // Check if we have a line model to convert
    if let Some(ref line_model) = state.line_model {
        println!("Converting lines to 3D pipes from line model: {}", line_model._name);
        
        // We'll create pipes based on the same instances as the lines
        // This ensures the pipes are in the same positions as the original lines
        
        let mut all_vertices = Vec::new();
        let mut all_indices = Vec::new();
        let mut vertex_count: u32 = 0;
        
        const PIPE_RADIUS: f32 = 0.02; // Radius of the pipe
        const PIPE_SEGMENTS: u32 = 8;  // Number of segments around the pipe circumference
        
        // Use the same instances stored in state.instances
        println!("Creating pipes with {} instances", state.instances.len());
        
        for instance in &state.instances {
            let pos = instance.position;
            let rotation = instance.rotation;
            
            // Convert the quaternion rotation to a 4x4 matrix
            let rotation_matrix = cgmath::Matrix4::from(rotation);
            
            // Define the same start/end points as the lines
            let start_local = cgmath::Point3::new(0.0, -0.5, 0.0);
            let end_local = cgmath::Point3::new(0.0, 1.5, 0.0);
            
            // Apply rotation and translation to get world coordinates
            let start_world = rotation_matrix.transform_point(start_local) + cgmath::Vector3::new(pos.x, pos.y, pos.z);
            let end_world = rotation_matrix.transform_point(end_local) + cgmath::Vector3::new(pos.x, pos.y, pos.z);
            
            // Create pipe geometry between start and end points
            let pipe_direction: cgmath::Vector3<f32> = (end_world - start_world).normalize();
            
            // Create a coordinate system for the pipe
            let up = if pipe_direction.dot(cgmath::Vector3::unit_y()).abs() < 0.9 {
                cgmath::Vector3::unit_y()
            } else {
                cgmath::Vector3::unit_x()
            };
            let right = pipe_direction.cross(up).normalize();
            let forward = right.cross(pipe_direction).normalize();
            
            // Generate vertices for the pipe
            for segment in 0..PIPE_SEGMENTS {
                let angle = 2.0 * std::f32::consts::PI * segment as f32 / PIPE_SEGMENTS as f32;
                let cos_angle = angle.cos();
                let sin_angle = angle.sin();
                
                // Calculate the offset from the pipe center
                let offset = right * (cos_angle * PIPE_RADIUS) + forward * (sin_angle * PIPE_RADIUS);
                
                // Create vertices at both ends of the pipe
                let start_vertex = start_world + offset;
                let end_vertex = end_world + offset;
                
                // Use position-based coloring like other geometries
                let x_normalized = (pos.x + 15.0) / 30.0;
                let z_normalized = (pos.z + 15.0) / 30.0;
                let color = [
                    x_normalized, 
                    (1.0 - x_normalized) * z_normalized,
                    1.0 - z_normalized,
                ];
                
                all_vertices.push(PipeVertex {
                    position: [start_vertex.x, start_vertex.y, start_vertex.z],
                    color,
                });
                
                all_vertices.push(PipeVertex {
                    position: [end_vertex.x, end_vertex.y, end_vertex.z],
                    color,
                });
            }
            
            // Generate indices for the pipe
            let base_idx = vertex_count;
            
            for segment in 0..PIPE_SEGMENTS {
                let next_segment = (segment + 1) % PIPE_SEGMENTS;
                
                // Each segment creates a quad (2 triangles) on the pipe surface
                let start_current = base_idx + segment * 2;
                let end_current = base_idx + segment * 2 + 1;
                let start_next = base_idx + next_segment * 2;
                let end_next = base_idx + next_segment * 2 + 1;
                
                // First triangle
                all_indices.extend_from_slice(&[start_current, end_current, start_next]);
                // Second triangle
                all_indices.extend_from_slice(&[start_next, end_current, end_next]);
            }
            
            vertex_count += PIPE_SEGMENTS * 2;
        }
        
        println!("Created {} pipe vertices and {} indices", all_vertices.len(), all_indices.len());
        
        // Create vertex buffer
        let vertex_buffer = state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Pipe Vertex Buffer"),
            contents: bytemuck::cast_slice(&all_vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        
        // Create index buffer
        let index_buffer = state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Pipe Index Buffer"),
            contents: bytemuck::cast_slice(&all_indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        
        // Create the pipe model
        let pipe_model = crate::model_pipe::PipeModel {
            name: "Converted Pipe Lines".to_string(),
            vertex_buffer,
            index_buffer,
            num_indices: all_indices.len() as u32,
        };
        
        state.pipe_model = Some(pipe_model);
        println!("Line-to-pipe conversion completed successfully!");
    } else {
        println!("No line model available to convert to pipes");
    }
}
