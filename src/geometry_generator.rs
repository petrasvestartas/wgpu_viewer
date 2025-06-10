use crate::model::{LineVertex, LineModel};

/// A simple line segment with start and end points and color
pub struct Line {
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub color: [f32; 3],
}

impl Line {
    /// Create a new line with start, end, and color
    pub fn new(start: [f32; 3], end: [f32; 3], color: [f32; 3]) -> Self {
        Self {
            start,
            end,
            color,
        }
    }
    
    /// Converts an array of Lines to a LineModel
    pub fn create_line_model(device: &wgpu::Device, lines: &[Line]) -> LineModel {
        let mut vertices = Vec::with_capacity(lines.len() * 2);
        
        // Process each line into vertices
        for line in lines.iter() {
            // Add start vertex
            vertices.push(LineVertex {
                position: line.start,
                color: line.color,
            });
            
            // Add end vertex
            vertices.push(LineVertex {
                position: line.end,
                color: line.color,
            });
        }
        
        // Create the line model
        LineModel::new(device, "line_model", &vertices)
    }
}

/// Creates a 10x10 grid of lines on the XZ plane with 1 unit spacing, centered at origin
pub fn create_grid_lines(device: &wgpu::Device) -> LineModel {
    let mut lines = Vec::new();
    
    // Define grid parameters
    let grid_size = 10; // 10x10 grid
    let grid_spacing = 1.0; // 1 unit spacing
    
    // Calculate grid start and end to center the grid
    let half_size = (grid_size as f32 * grid_spacing) / 2.0;
    let grid_start = -half_size;
    let grid_end = half_size;
    
    let grid_color = [0.7, 0.7, 0.7]; // Grey color for all grid lines
    let x_axis_color = [1.0, 0.0, 0.0]; // Red for X axis
    let y_axis_color = [0.0, 1.0, 0.0]; // Green for Y axis
    
    // A slight elevation to make the axes more visible
    let axis_elevation = 0.02;
    
    // Create grid lines along X and Z axes (all grey)
    for i in 0..=grid_size {
        let pos = grid_start + (i as f32 * grid_spacing);
        
        // X-axis line (varying Z)
        lines.push(Line::new(
            [pos, 0.0, grid_start], 
            [pos, 0.0, grid_end],
            grid_color
        ));
        
        // Z-axis line (varying X)
        lines.push(Line::new(
            [grid_start, 0.0, pos], 
            [grid_end, 0.0, pos],
            grid_color
        ));
    }
    
    // Add X axis (red) from origin to (5,0,0) - slightly elevated
    lines.push(Line::new(
        [0.0, axis_elevation, 0.0],  // start at origin, slightly elevated
        [5.0, axis_elevation, 0.0],  // extend 5 units along X axis
        x_axis_color
    ));
    
    // Add Y axis (green) from origin to (0,0,-5) - slightly elevated
    // In this viewer, Y axis is up, and Z axis is forward from origin
    lines.push(Line::new(
        [0.0, axis_elevation, 0.0],  // start at origin, slightly elevated
        [0.0, axis_elevation, -5.0], // extend 5 units in negative Z direction (which is negative Y in viewer coords)
        y_axis_color
    ));
    
    // Convert lines to a LineModel
    Line::create_line_model(device, &lines)
}

/// Creates coordinate system axes
pub fn create_axes(device: &wgpu::Device, size: f32, origin: [f32; 3], colors: [[f32; 3]; 3]) -> LineModel {
    let mut lines = Vec::new();
    
    // X axis - red
    lines.push(Line::new(
        origin,
        [origin[0] + size, origin[1], origin[2]],
        colors[0]
    ));
    
    // Y axis - green
    lines.push(Line::new(
        origin,
        [origin[0], origin[1] + size, origin[2]],
        colors[1]
    ));
    
    // Z axis - blue
    lines.push(Line::new(
        origin,
        [origin[0], origin[1], origin[2] + size],
        colors[2]
    ));
    
    Line::create_line_model(device, &lines)
}

/// Creates a 3D boundary box from min/max corners
pub fn create_boundary_box(device: &wgpu::Device, min: [f32; 3], max: [f32; 3], color: [f32; 3]) -> LineModel {
    let mut lines = Vec::new();
    
    // Bottom face edges
    lines.push(Line::new([min[0], min[1], min[2]], [max[0], min[1], min[2]], color));
    lines.push(Line::new([max[0], min[1], min[2]], [max[0], min[1], max[2]], color));
    lines.push(Line::new([max[0], min[1], max[2]], [min[0], min[1], max[2]], color));
    lines.push(Line::new([min[0], min[1], max[2]], [min[0], min[1], min[2]], color));
    
    // Top face edges
    lines.push(Line::new([min[0], max[1], min[2]], [max[0], max[1], min[2]], color));
    lines.push(Line::new([max[0], max[1], min[2]], [max[0], max[1], max[2]], color));
    lines.push(Line::new([max[0], max[1], max[2]], [min[0], max[1], max[2]], color));
    lines.push(Line::new([min[0], max[1], max[2]], [min[0], max[1], min[2]], color));
    
    // Vertical edges
    lines.push(Line::new([min[0], min[1], min[2]], [min[0], max[1], min[2]], color));
    lines.push(Line::new([max[0], min[1], min[2]], [max[0], max[1], min[2]], color));
    lines.push(Line::new([max[0], min[1], max[2]], [max[0], max[1], max[2]], color));
    lines.push(Line::new([min[0], min[1], max[2]], [min[0], max[1], max[2]], color));
    
    Line::create_line_model(device, &lines)
}

/// Creates lines approximating a parametric curve
pub fn create_parametric_curve(
    device: &wgpu::Device, 
    parametric_fn: fn(f32) -> [f32; 3],
    t_min: f32, 
    t_max: f32, 
    segments: usize,
    color: [f32; 3]
) -> LineModel {
    let mut lines = Vec::new();
    
    let step = (t_max - t_min) / segments as f32;
    
    for i in 0..segments {
        let t1 = t_min + i as f32 * step;
        let t2 = t_min + (i + 1) as f32 * step;
        
        let p1 = parametric_fn(t1);
        let p2 = parametric_fn(t2);
        
        lines.push(Line::new(p1, p2, color));
    }
    
    Line::create_line_model(device, &lines)
}

/// Creates a 3D helix curve
pub fn create_helix(device: &wgpu::Device, radius: f32, height: f32, turns: f32, segments_per_turn: usize) -> LineModel {
    let total_segments = (segments_per_turn as f32 * turns) as usize;
    let angle_step = turns * std::f32::consts::TAU / total_segments as f32;
    let height_step = height / total_segments as f32;
    
    // Define the helix parametric function
    let helix = |t: f32| {
        let angle = t * angle_step;
        [
            radius * angle.cos(),
            t * height_step,
            radius * angle.sin()
        ]
    };
    
    let mut lines = Vec::new();
    
    // Create lines connecting points along the helix
    for i in 0..total_segments {
        let p1 = helix(i as f32);
        let p2 = helix((i + 1) as f32);
        
        // Create a gradient color based on the position along the helix
        let h = i as f32 / total_segments as f32;
        let color = [h, 1.0 - h, 0.5];
        
        lines.push(Line::new(p1, p2, color));
    }
    
    Line::create_line_model(device, &lines)
}

/// Creates a 3D helix polyline with the lines array approach
pub fn create_helix_polyline(device: &wgpu::Device) -> LineModel {
    // Create a helix with specific parameters
    create_helix(device, 3.0, 10.0, 5.0, 20)
}
