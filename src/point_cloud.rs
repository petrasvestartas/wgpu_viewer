use crate::{model, instance::Instance};

/// Generates point cloud vertices for a series of cube instances
#[allow(dead_code)]
pub fn generate_point_cloud(instances: &[Instance]) -> Vec<model::PointVertex> {
    println!("DEBUG: Creating point clouds for {} cube instances", instances.len());
    
    let mut point_vertices = Vec::new();
    
    // Define a small local grid for each instance
    let local_grid_size = 10; // Points along each axis per cube
    let local_grid_extent = 0.5; // Size of cube is 1.0 (-0.5 to +0.5)
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
                    
                    point_vertices.push(model::PointVertex {
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