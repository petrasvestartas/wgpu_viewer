use wgpu::util::DeviceExt;
use std::sync::RwLock;
use lazy_static::lazy_static;

// Global configuration struct - this can be expanded with more parameters as needed
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct RenderConfig {
    pub point_size: f32,
    // Add padding for first vec4 (x=point_size, yzw=padding)
    pub _padding1: [f32; 3],
    // Add a second vec4 for future parameters
    pub _padding2: [f32; 4],
}

impl RenderConfig {
    pub fn new() -> Self {
        Self {
            point_size: 0.01, // Default point size
            _padding1: [0.0; 3],
            _padding2: [0.0; 4],
        }
    }
}

// Global instance that can be accessed from anywhere
lazy_static! {
    pub static ref RENDER_CONFIG: RwLock<RenderConfig> = RwLock::new(RenderConfig::new());
}

// Global helper functions for direct access from anywhere in the code

/// Set the global point size - one line access from anywhere
pub fn set_point_size(size: f32) {
    let mut config = RENDER_CONFIG.write().unwrap();
    config.point_size = size;
}

/// Get the current global point size
pub fn get_point_size() -> f32 {
    RENDER_CONFIG.read().unwrap().point_size
}

pub struct ConfigUniform {
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl ConfigUniform {
    pub fn new(device: &wgpu::Device) -> Self {
        // Get the current config values
        let config_data = *RENDER_CONFIG.read().unwrap();
        
        // Create the buffer
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Config Uniform Buffer"),
            contents: bytemuck::cast_slice(&[config_data]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        
        // Create the bind group layout
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("config_bind_group_layout"),
        });
        
        // Create the bind group
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }
            ],
            label: Some("config_bind_group"),
        });
        
        Self {
            buffer,
            bind_group,
            bind_group_layout,
        }
    }
    
    pub fn update(&self, queue: &wgpu::Queue) {
        // Get the latest config values
        let config_data = *RENDER_CONFIG.read().unwrap();
        
        // Update the GPU buffer
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[config_data]));
    }
    
    // Helper to change the point size and update the buffer
    pub fn set_point_size(size: f32, queue: &wgpu::Queue) {
        // Update the global config
        {
            let mut config = RENDER_CONFIG.write().unwrap();
            config.point_size = size;
        }
        
        // The caller is responsible for calling update() when convenient
    }
}
