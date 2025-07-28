//! # Model Module
//! 
//! This module serves as the main facade for all 3D model functionality.
//! It re-exports types and traits from specialized modules:
//! 
//! - `model_mesh`: Mesh-based 3D models (polygonal geometry)
//! - `model_point`: Point cloud models (collections of 3D points)
//! - `model_line`: Line models (collections of 3D line segments)
//! - `model_pipe`: Pipe models (cylindrical geometry from lines)
//! - `model_polygon`: Polygon models (closed polyline geometry)
//!
//! OpenModel Integration:
//! This module provides unified access to OpenModel geometry kernel functionality,
//! enabling seamless conversion between OpenModel geometry types and GPU-ready
//! vertex structures for rendering.
//!
//! This organization follows the Single Responsibility Principle by
//! separating different model types into dedicated modules.

#[path = "model_mesh.rs"]
pub mod model_mesh;
#[path = "model_point.rs"]
pub mod model_point;
#[path = "model_line.rs"]
pub mod model_line;
#[path = "model_pipe.rs"]
pub mod model_pipe;
#[path = "model_polygon.rs"]
pub mod model_polygon;

// Re-export all model types and traits
pub use model_mesh::{ModelVertex, Mesh, Model, DrawModel, DrawLight, Vertex};
pub use model_point::{PointModel};
pub use model_line::{LineVertex, LineModel};
pub use model_pipe::{PipeModel};
pub use model_polygon::{PolygonModel};

// OpenModel imports for unified geometry handling
use openmodel::geometry::{
    Point as OpenModelPoint,
    Line as OpenModelLine, 
    Mesh as OpenModelMesh,
    Pline as OpenModelPline,
    PointCloud as OpenModelPointCloud,
};

/// Unified OpenModel geometry collection for mixed model types
/// This enum allows handling different OpenModel geometry types in a unified way
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum OpenModelGeometry {
    Point(OpenModelPoint),
    PointCloud(OpenModelPointCloud),
    Line(OpenModelLine),
    Mesh(OpenModelMesh),
    Pline(OpenModelPline),
}

/// Unified model creation from OpenModel geometries
/// This provides a high-level interface for creating GPU models from OpenModel data
#[allow(dead_code)]
pub struct UnifiedModelFactory;

impl UnifiedModelFactory {
    /// Create appropriate GPU models from a collection of mixed OpenModel geometries
    /// Returns separate model collections for each geometry type
    #[allow(dead_code)]
    pub fn create_models_from_openmodel_geometries(
        device: &wgpu::Device,
        name_prefix: &str,
        geometries: &[OpenModelGeometry],
    ) -> UnifiedModelCollection {
        let mut point_models = Vec::new();
        let mut line_models = Vec::new();
        let mut mesh_models = Vec::new();
        let pipe_models = Vec::new();
        let mut polygon_models = Vec::new();
        
        for (i, geometry) in geometries.iter().enumerate() {
            let model_name = format!("{}_{}_{}", name_prefix, i, geometry.type_name());
            
            match geometry {
                OpenModelGeometry::Point(point) => {
                    let model = PointModel::from_openmodel_points(device, &model_name, &[point.clone()]);
                    point_models.push(model);
                },
                OpenModelGeometry::PointCloud(pointcloud) => {
                    let model = PointModel::from_openmodel_pointcloud(device, &model_name, pointcloud);
                    point_models.push(model);
                },
                OpenModelGeometry::Line(line) => {
                    let model = LineModel::from_openmodel_line(device, &model_name, line);
                    line_models.push(model);
                },
                OpenModelGeometry::Mesh(mesh) => {
                    let model = Model::from_openmodel_mesh(device, &model_name, mesh);
                    mesh_models.push(model);
                },
                OpenModelGeometry::Pline(pline) => {
                    let model = PolygonModel::from_openmodel_pline(device, &model_name, pline);
                    polygon_models.push(model);
                },
            }
        }
        
        UnifiedModelCollection {
            point_models,
            line_models,
            mesh_models,
            pipe_models,
            polygon_models,
        }
    }
    
    /// Create pipe models from OpenModel lines with automatic mesh generation
    #[allow(dead_code)]
    pub fn create_pipe_models_from_openmodel_lines(
        device: &wgpu::Device,
        name: &str,
        lines: &[OpenModelLine],
    ) -> Vec<PipeModel> {
        vec![PipeModel::from_openmodel_lines(device, name, lines)]
    }
}

/// Collection of all model types created from OpenModel geometries
#[allow(dead_code)]
pub struct UnifiedModelCollection {
    pub point_models: Vec<PointModel>,
    pub line_models: Vec<LineModel>,
    pub mesh_models: Vec<Model>,
    pub pipe_models: Vec<PipeModel>,
    pub polygon_models: Vec<PolygonModel>,
}

impl OpenModelGeometry {
    /// Get the type name as a string for naming purposes
    #[allow(dead_code)]
    pub fn type_name(&self) -> &'static str {
        match self {
            OpenModelGeometry::Point(_) => "point",
            OpenModelGeometry::PointCloud(_) => "pointcloud",
            OpenModelGeometry::Line(_) => "line",
            OpenModelGeometry::Mesh(_) => "mesh",
            OpenModelGeometry::Pline(_) => "pline",
        }
    }
    
    /// Check if the geometry has color information
    #[allow(dead_code)]
    pub fn has_color(&self) -> bool {
        match self {
            OpenModelGeometry::Point(point) => point.data.has_color(),
            OpenModelGeometry::PointCloud(pointcloud) => pointcloud.data.has_color(),
            OpenModelGeometry::Line(line) => line.data.has_color(),
            OpenModelGeometry::Mesh(mesh) => mesh.data.has_color(),
            OpenModelGeometry::Pline(pline) => pline.data.has_color(),
        }
    }
    
    /// Get the color if available
    #[allow(dead_code)]
    pub fn get_color(&self) -> Option<[u8; 3]> {
        if self.has_color() {
            match self {
                OpenModelGeometry::Point(point) => Some(point.data.get_color()),
                OpenModelGeometry::PointCloud(pointcloud) => Some(pointcloud.data.get_color()),
                OpenModelGeometry::Line(line) => Some(line.data.get_color()),
                OpenModelGeometry::Mesh(mesh) => Some(mesh.data.get_color()),
                OpenModelGeometry::Pline(pline) => Some(pline.data.get_color()),
            }
        } else {
            None
        }
    }
}
