//! # Model Module
//! 
//! This module serves as the main facade for all 3D model functionality.
//! It re-exports types and traits from specialized modules:
//! 
//! - `model_mesh`: Mesh-based 3D models (polygonal geometry)
//! - `model_point`: Point cloud models (collections of 3D points)
//! - `model_line`: Line models (collections of 3D line segments)
//!
//! This organization follows the Single Responsibility Principle by
//! separating different model types into dedicated modules.
#[path = "model_mesh.rs"]
pub mod model_mesh;
#[path = "model_point.rs"]
pub mod model_point;
#[path = "model_line.rs"]
pub mod model_line;

pub use model_mesh::{ModelVertex, Mesh, Model, DrawModel, DrawLight, Vertex};
pub use model_point::{PointModel};
pub use model_line::{LineVertex, LineModel};
