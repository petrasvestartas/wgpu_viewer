use crate::model::LineModel;
use crate::model_pipe::PipeModel;
use crate::model_point::PointModel;
use crate::model_polygon::PolygonModel;
use crate::geometry_generator;

/// Struct to hold all the demo geometries
pub struct DemoGeometries {
    pub line_grid: Option<LineModel>,
    pub line_axes: Option<LineModel>,
    pub line_helix: Option<LineModel>,
    pub pipe_model: Option<PipeModel>,
    pub point_cloud: Option<PointModel>,
    pub polygon_model: Option<PolygonModel>,
}

impl DemoGeometries {
    pub fn new(device: &wgpu::Device) -> Self {
        // Create a 10x10 grid of lines with 1 unit spacing
        let line_grid = Some(geometry_generator::create_grid_lines(device));
        
        // Create coordinate axes
        let line_axes = Some(geometry_generator::create_axes(
            device, 
            3.0,  // size 
            [0.0, 0.0, 0.0],  // origin 
            [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]  // RGB colors
        ));
        
        // Create a helix
        let line_helix = Some(geometry_generator::create_helix_polyline(device));
        
        // Other models will be created later or in the main application
        Self {
            line_grid,
            line_axes,
            line_helix,
            pipe_model: None,
            point_cloud: None,
            polygon_model: None,
        }
    }
}
