use crate::lib_state::State;
use crate::RenderMode;
use crate::model::{DrawModel, DrawLight};
use crate::model_point::DrawQuadPoints;
use crate::model_pipe::DrawPipes;
use crate::model_polygon::DrawPolygons;
use crate::lib_geometry_manager::create_pipes_from_lines;
use crate::camera;
use cgmath::prelude::*;
use std::iter;

// GPU Uniform Structs (moved from renderer.rs)

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_position: [f32; 4],
    view_proj: [[f32; 4]; 4],
    aspect_ratio: [f32; 4], // Using vec4 for alignment (only first value used)
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_position: [0.0; 4],
            view_proj: cgmath::Matrix4::identity().into(),
            aspect_ratio: [1.0, 0.0, 0.0, 0.0], // Default to 1.0 aspect ratio
        }
    }

    pub fn update_view_proj(&mut self, camera: &camera::Camera, projection: &camera::Projection) {
        self.view_position = camera.position.to_homogeneous().into();
        self.view_proj = (projection.calc_matrix() * camera.calc_matrix()).into();
    }
    
    pub fn update_aspect_ratio(&mut self, width: f32, height: f32) {
        self.aspect_ratio[0] = width / height;
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub position: [f32; 3],
    pub _padding: u32,
    pub color: [f32; 3],
    pub _padding2: u32,
}

/// Main rendering function that handles all GPU drawing operations
pub fn render(state: &mut State) -> Result<(), wgpu::SurfaceError> {
    let output = state.surface.get_current_texture()?;
    let view = output
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    let mut encoder = state
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

    // Handle render modes that need to modify state before rendering
    match state.render_mode {
        RenderMode::All | RenderMode::Lines => {
            // Create pipe lines from line data if needed
            if state.pipe_model.is_none() && state.line_model.is_some() {
                create_pipes_from_lines(state);
            }
        },
        RenderMode::Polygons => {
            // Create sample polygon if it doesn't exist
            if state.polygon_model.is_none() {
                crate::lib_geometry_manager::create_sample_polygon(state);
            }
        },
        _ => {}
    }

    {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &state.multisample_texture_view, // Render to multisample texture
                resolve_target: Some(&view), // Resolve to final texture
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.9,
                        g: 0.9,
                        b: 0.9,
                        a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &state.multisample_depth_texture_view, // Use multisample depth texture
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        // Render based on the selected render mode
        match state.render_mode {
            RenderMode::All => {
                render_all_mode(state, &mut render_pass);
            },
            RenderMode::Points => {
                render_points_mode(state, &mut render_pass);
            },
            RenderMode::Lines => {
                render_lines_mode(state, &mut render_pass);
            },
            RenderMode::RegularLines => {
                render_regular_lines_mode(state, &mut render_pass);
            },
            RenderMode::Polygons => {
                render_polygons_mode(state, &mut render_pass);
            },
            RenderMode::Meshes => {
                render_meshes_mode(state, &mut render_pass);
            },
        }
    }
    state.queue.submit(iter::once(encoder.finish()));
    output.present();

    Ok(())
}

/// Render all geometry types (meshes, points, lines, polygons)
fn render_all_mode<'a>(
    state: &'a mut State,
    render_pass: &mut wgpu::RenderPass<'a>,
) {





    // Render the light model
    render_pass.set_vertex_buffer(1, state.instance_buffer.slice(..));
    render_pass.set_pipeline(&state.light_render_pipeline);
    render_pass.draw_light_model(
        &state.obj_model,
        &state.camera_bind_group,
        &state.light_bind_group,
    );
    
    // Render the mesh model
    render_pass.set_pipeline(&state.render_pipeline);
    // Draw main mesh model with edge visualization
    render_pass.draw_model_with_edges_instanced(
        &state.obj_model,
        0..state.instances.len() as u32,
        &state.camera_bind_group,
        &state.light_bind_group,
    );
    
    // Draw all additional mesh models with edge visualization
    for model in &state.additional_mesh_models {
        render_pass.draw_model_with_edges_instanced(
            model,
            0..1, // Only draw one instance for additional models
            &state.camera_bind_group,
            &state.light_bind_group,
        );
    }

    // Render points if available - use the quad-based point model for better visuals
    if let (Some(pipeline), Some(model)) = (&state.point_pipeline, &state.quad_point_model) {
        render_pass.set_pipeline(pipeline);
        render_pass.draw_quad_points(model, &state.camera_bind_group);
    }
    
    // Render 3D pipe lines instead of regular lines
    if let (Some(pipeline), Some(model)) = (&state.pipe_pipeline, &state.pipe_model) {
        render_pass.set_pipeline(pipeline);
        render_pass.draw_pipes(model, &state.camera_bind_group);
    }
    
    // Regular line rendering for grid lines to be visible by default
    if let (Some(pipeline), Some(model)) = (&state.line_pipeline, &state.line_model) {
        render_pass.set_pipeline(pipeline);
        
        // Use direct drawing approach to avoid trait issues
        render_pass.set_vertex_buffer(0, model.vertex_buffer.slice(..));
        render_pass.set_bind_group(0, &state.camera_bind_group, &[]);
        render_pass.draw(0..model.num_vertices, 0..1);
    }
    
    // Render polygons loaded from JSON
    if let (Some(pipeline), Some(model)) = (&state.polygon_pipeline, &state.polygon_model) {
        render_pass.set_pipeline(pipeline);
        render_pass.draw_polygons(model, &state.camera_bind_group, &state.light_bind_group);
    }
}

/// Render only points using quad-based rendering
fn render_points_mode<'a>(
    state: &'a mut State,
    render_pass: &mut wgpu::RenderPass<'a>,
) {


    // Render only points using quad-based rendering for better visuals
    if let (Some(pipeline), Some(model)) = (&state.point_pipeline, &state.quad_point_model) {
        render_pass.set_pipeline(pipeline);
        render_pass.draw_quad_points(model, &state.camera_bind_group);
    }
}

/// Render lines as 3D pipes
fn render_lines_mode(
    state: &mut State,
    render_pass: &mut wgpu::RenderPass,
) {

    
    // Render 3D pipe lines instead of regular lines
    if let (Some(pipeline), Some(model)) = (&state.pipe_pipeline, &state.pipe_model) {
        render_pass.set_pipeline(pipeline);
        render_pass.draw_pipes(model, &state.camera_bind_group);
    }
    // Regular line rendering for grid lines to be visible by default
    if let (Some(pipeline), Some(model)) = (&state.line_pipeline, &state.line_model) {
        render_pass.set_pipeline(pipeline);
        
        // Use direct drawing approach to avoid trait issues
        render_pass.set_vertex_buffer(0, model.vertex_buffer.slice(..));
        render_pass.set_bind_group(0, &state.camera_bind_group, &[]);
        render_pass.draw(0..model.num_vertices, 0..1);
    }
}

/// Render regular lines without 3D pipes
fn render_regular_lines_mode(
    state: &mut State,
    render_pass: &mut wgpu::RenderPass,
) {
    // Render regular lines without 3D pipes
    if let (Some(pipeline), Some(model)) = (&state.line_pipeline, &state.line_model) {
        render_pass.set_pipeline(pipeline);
        
        // Use the correct type - model_line::LineModel is expected by draw_lines
        // Draw the model without using draw_lines trait which has type mismatch
        render_pass.set_vertex_buffer(0, model.vertex_buffer.slice(..));
        render_pass.set_bind_group(0, &state.camera_bind_group, &[]);
        render_pass.draw(0..model.num_vertices, 0..1);
    }
}

/// Render only polygons
fn render_polygons_mode(
    state: &mut State,
    render_pass: &mut wgpu::RenderPass,
) {

    
    // Render the polygon model
    if let (Some(pipeline), Some(model)) = (&state.polygon_pipeline, &state.polygon_model) {
        render_pass.set_pipeline(pipeline);
        render_pass.draw_polygons(model, &state.camera_bind_group, &state.light_bind_group);
    }
}

/// Render only meshes with lighting
fn render_meshes_mode<'a>(
    state: &'a mut State,
    render_pass: &mut wgpu::RenderPass<'a>,
) {


    // Render the light and mesh models
    render_pass.set_vertex_buffer(1, state.instance_buffer.slice(..));
    render_pass.set_pipeline(&state.light_render_pipeline);
    
    // Draw the main mesh model light
    render_pass.draw_light_model(
        &state.obj_model,
        &state.camera_bind_group,
        &state.light_bind_group,
    );
    
    // Draw the main mesh model with edge visualization
    render_pass.set_pipeline(&state.render_pipeline);
    render_pass.draw_model_with_edges_instanced(
        &state.obj_model,
        0..state.instances.len() as u32,
        &state.camera_bind_group,
        &state.light_bind_group,
    );
    
    // Draw all additional mesh models with edge visualization
    for mesh_model in &state.additional_mesh_models {
        // Draw each mesh model with instancing and edge visualization
        render_pass.draw_model_with_edges_instanced(
            mesh_model,
            0..state.instances.len() as u32,
            &state.camera_bind_group,
            &state.light_bind_group,
        );
    }
}
