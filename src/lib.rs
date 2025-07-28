/// Specifies what type of geometry to render
#[derive(Debug, Copy, Clone, PartialEq, Default)]
pub enum RenderMode {
    #[default]
    All = 0,
    Points = 1,
    Lines = 2, // Now uses pipe lines by default
    RegularLines = 3, // Added option for regular lines without pipes
    Meshes = 4,
    Polygons = 5,
}

mod camera;
mod instance;
mod model_line;
mod model;
mod model_pipe;
mod model_point;
mod model_polygon;
mod lib_pipeline;
mod resources;
mod geometry_loader;
pub mod geometry_generator;
mod lib_hot_reload;
mod lib_input;
mod lib_geometry_manager;
mod lib_app;
mod lib_render;
mod lib_state;

use cgmath::prelude::*;
use winit::{
    event::*,
    window::Window,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// Re-export State from lib_state module
pub use lib_state::State;

// create_render_pipeline function has been moved to pipeline.rs module

impl<'a> State<'a> {
    pub fn window(&self) -> &Window {
        self.window
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.projection.resize(new_size.width, new_size.height);
            
            // Update aspect ratio in camera uniform
            self.camera_uniform.update_aspect_ratio(new_size.width as f32, new_size.height as f32);
            
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            
            // Create new depth texture directly without texture module
            let depth_size = wgpu::Extent3d {
                width: self.config.width.max(1),
                height: self.config.height.max(1),
                depth_or_array_layers: 1,
            };
            let depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("depth_texture"),
                size: depth_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[wgpu::TextureFormat::Depth32Float],
            });
            self.depth_texture_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

            // Recreate multisample textures with new size
            self.multisample_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("multisample_texture"),
                size: wgpu::Extent3d {
                    width: self.config.width,
                    height: self.config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 4, // 4x MSAA for web compatibility
                dimension: wgpu::TextureDimension::D2,
                format: self.config.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[self.config.format],
            });

            self.multisample_texture_view = self.multisample_texture.create_view(&wgpu::TextureViewDescriptor::default());

            // Recreate multisample depth texture
            self.multisample_depth_texture = self.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("multisample_depth_texture"),
                size: wgpu::Extent3d {
                    width: self.config.width,
                    height: self.config.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 4, // 4x MSAA for web compatibility
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Depth32Float,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[wgpu::TextureFormat::Depth32Float],
            });

            self.multisample_depth_texture_view = self.multisample_depth_texture.create_view(&wgpu::TextureViewDescriptor::default());
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        lib_input::handle_input(self, event)
    }

    fn update(&mut self, dt: std::time::Duration) {
        // UPDATED!
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera_uniform.update_view_proj(&self.camera, &self.projection);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        // Update the light
        let old_position: cgmath::Vector3<_> = self.light_uniform.position.into();
        self.light_uniform.position =
            (cgmath::Quaternion::from_axis_angle((0.0, 1.0, 0.0).into(), cgmath::Deg(1.0))
                * old_position)
                .into();
        self.queue.write_buffer(
            &self.light_buffer,
            0,
            bytemuck::cast_slice(&[self.light_uniform]),
        );
    }
    
    /// Load geometry data from a JSON file
    async fn load_geometries_from_file(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        lib_geometry_manager::load_geometries_from_file(self, path).await
    }

    /// Main rendering method - delegates to the rendering engine module
    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        lib_render::render(self)
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
/// Main application entry point - delegates to the application runner module
pub async fn run() {
    lib_app::run().await;
}
