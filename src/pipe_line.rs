use wgpu::util::DeviceExt;
use cgmath::*;

// Line segment definition
#[derive(Debug, Clone)]
pub struct LineSegment {
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub color: [f32; 3],
    pub radius: f32,
}

// Vertex structure for cylinders
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PipeVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
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
                // normal
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // color
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

pub struct PipeLineModel {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

impl PipeLineModel {
    pub fn new(
        device: &wgpu::Device, 
        name: &str, 
        line_segments: &[LineSegment],
        resolution: u32, // Number of sides for each cylinder
    ) -> Self {
        // Generate vertices and indices for all line segments
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut index_offset = 0;
        
        for segment in line_segments {
            let (mut segment_vertices, mut segment_indices) = 
                create_cylinder_for_line(segment, resolution);
            
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

// Create a cylinder mesh along a line segment with no-nonsense, hardcoded correct approach
fn create_cylinder_for_line(
    segment: &LineSegment,
    sides: u32,
) -> (Vec<PipeVertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    
    // Direction vector from start to end
    let start = Vector3::from(segment.start);
    let end = Vector3::from(segment.end);
    let direction = end - start;
    
    // Skip if the line has zero length
    if direction.magnitude() < 1e-6 {
        return (vertices, indices);
    }
    
    // Create an orthonormal basis for the cylinder
    let axis = direction.normalize();
    
    // Find perpendicular vectors for the cylinder cross section
    // This is the key to consistent orientation
    let perpendicular = if axis.x.abs() > 0.5 || axis.y.abs() > 0.5 {
        Vector3::new(axis.y, -axis.x, 0.0).normalize()
    } else {
        Vector3::new(0.0, axis.z, -axis.y).normalize()
    };
    let binormal = axis.cross(perpendicular).normalize();
    
    // --- CONSISTENT APPROACH: ALWAYS USE CCW FOR ALL VERTICES ---
    
    // Add bottom cap center vertex (#0 in our vertex list)
    vertices.push(PipeVertex {
        position: [start.x, start.y, start.z],
        normal: [-axis.x, -axis.y, -axis.z], // Bottom normal points down
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
            normal: [-axis.x, -axis.y, -axis.z], // Bottom normal points down
            color: segment.color,
        });
    }
    
    // Add top cap center vertex (#sides+1 in our vertex list)
    vertices.push(PipeVertex {
        position: [end.x, end.y, end.z],
        normal: [axis.x, axis.y, axis.z], // Top normal points up
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
            normal: [axis.x, axis.y, axis.z], // Top normal points up
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
        
        // Each quad of the cylinder side is made of two triangles
        
        // Triangle 1: bottom_curr, top_curr, top_next (CCW)
        indices.push(bottom_curr);
        indices.push(top_curr);
        indices.push(top_next);
        
        // Triangle 2: bottom_curr, top_next, bottom_next (CCW)
        indices.push(bottom_curr);
        indices.push(top_next);
        indices.push(bottom_next);
    }
    
    (vertices, indices)
}

pub trait DrawPipeLines<'a> {
    fn draw_pipe_lines(
        &mut self,
        pipe_line_model: &'a PipeLineModel,
        camera_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b: 'a> DrawPipeLines<'a> for wgpu::RenderPass<'b> {
    fn draw_pipe_lines(
        &mut self,
        pipe_line_model: &'a PipeLineModel,
        camera_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, pipe_line_model.vertex_buffer.slice(..));
        self.set_index_buffer(pipe_line_model.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, camera_bind_group, &[]);
        self.draw_indexed(0..pipe_line_model.num_indices, 0, 0..1);
    }
}
