use std::{f32::consts::PI, iter};

/// Specifies what type of geometry to render
#[derive(Debug, Copy, Clone, PartialEq)]
pub enum RenderMode {
    All = 0,
    Points = 1,
    Lines = 2,
    Meshes = 3,
}

impl Default for RenderMode {
    fn default() -> Self {
        RenderMode::All
    }
}

mod camera;
mod config;
mod instance;
mod model;
mod model_point;
mod pipeline;
mod renderer;
mod resources;
mod texture;

use cgmath::prelude::*;
use wgpu::util::DeviceExt;
use crate::model::{DrawModel, DrawLight, DrawLines, Vertex};
use crate::model_point::{DrawQuadPoints, PointVertex as MPPointVertex, QuadPointModel};
use winit::{
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

// Renderer will be used in future upgrades
// use crate::renderer::Renderer;

// Re-exports for use in other modules
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

use instance::Instance;
use instance::InstanceRaw;
use renderer::CameraUniform;
use renderer::LightUniform;

const NUM_INSTANCES_PER_ROW: u32 = 10;

#[allow(dead_code)]
struct State<'a> {
    window: &'a Window,
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    point_pipeline: Option<wgpu::RenderPipeline>, // Pipeline for points
    line_pipeline: Option<wgpu::RenderPipeline>,  // Pipeline for lines
    obj_model: model::Model,
    point_model: Option<model::PointModel>,      // Optional point cloud model
    quad_point_model: Option<model_point::QuadPointModel>, // Optional quad-based point model for billboard rendering
    line_model: Option<model::LineModel>,        // Optional line model
    render_mode: RenderMode,                     // Current rendering mode
    camera: camera::Camera,                      // UPDATED!
    projection: camera::Projection,              // NEW!
    camera_controller: camera::CameraController, // UPDATED!
    camera_uniform: CameraUniform,
    config_uniform: config::ConfigUniform,       // Global configuration settings
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    instances: Vec<Instance>,
    #[allow(dead_code)]
    instance_buffer: wgpu::Buffer,
    depth_texture: texture::Texture,
    size: winit::dpi::PhysicalSize<u32>,
    light_uniform: LightUniform,
    light_buffer: wgpu::Buffer,
    light_bind_group: wgpu::BindGroup,
    light_render_pipeline: wgpu::RenderPipeline,
    #[allow(dead_code)]
    debug_material: model::Material,
    // NEW!
    mouse_pressed: bool,
}

// create_render_pipeline function has been moved to pipeline.rs module

impl<'a> State<'a> {
    async fn new(window: &'a Window) -> State<'a> {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    // WebGL doesn't support all of wgpu's features, so if
                    // we're building for the web we'll have to disable some.
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    memory_hints: Default::default(),
                },
                None, // Trace path
            )
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an Srgb surface texture. Using a different
        // one will result all the colors comming out darker. If you want to support non
        // Srgb surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    // normal map
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });

        // UPDATED!
        let camera = camera::Camera::new((0.0, 5.0, 10.0), cgmath::Deg(-90.0), cgmath::Deg(-20.0));
        let projection =
            camera::Projection::new(config.width, config.height, cgmath::Deg(45.0), 0.1, 100.0);
        let camera_controller = camera::CameraController::new(4.0, 0.4);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera, &projection);
        
        // Initialize with correct aspect ratio
        let initial_width = config.width as f32;
        let initial_height = config.height as f32;
        camera_uniform.update_aspect_ratio(initial_width, initial_height);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        const SPACE_BETWEEN: f32 = 3.0;
        // Print the constants for debugging
        println!("DEBUG: NUM_INSTANCES_PER_ROW = {}", NUM_INSTANCES_PER_ROW);
        println!("DEBUG: SPACE_BETWEEN = {}", SPACE_BETWEEN);
        
        let instances = (0..NUM_INSTANCES_PER_ROW)
            .flat_map(|z| {
                (0..NUM_INSTANCES_PER_ROW).map(move |x| {
                    let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
                    let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

                    let position = cgmath::Vector3 { x, y: 0.0, z };
                    
                    // Print all mesh box positions
                    println!("MESH BOX: grid[{}][{}] = position({:.1}, {:.1}, {:.1})", z, x, position.x, position.y, position.z);

                    let rotation = if position.is_zero() {
                        cgmath::Quaternion::from_axis_angle(
                            cgmath::Vector3::unit_z(),
                            cgmath::Deg(0.0),
                        )
                    } else {
                        cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
                    };

                    Instance { position, rotation }
                })
            })
            .collect::<Vec<_>>();

        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&instance_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });
        
        // Initialize the global configuration uniform
        let config_uniform = config::ConfigUniform::new(&device);

        // Load standard mesh model
        let obj_model =
            resources::load_model("cube.obj", &device, &queue, &texture_bind_group_layout)
                .await
                .unwrap();
        
        // Generate point cloud vertices using our utility function
        let point_vertices = model::model_point::generate_point_cloud(&instances);
        
        let axis_length = 5.0;

        // Create a point cloud model for the cube corners
        let point_model = Some(model::PointModel::new(
            &device,
            "point_cloud",
            &point_vertices,
        ));
        
        // Create a quad-based point model for better visual appearance
        // Convert model::PointVertex to model_point::PointVertex
        let mp_point_vertices: Vec<MPPointVertex> = point_vertices.iter().map(|v| MPPointVertex {
            position: v.position,
            color: v.color,
            size: v.size,
        }).collect();
        
        let quad_point_model = Some(QuadPointModel::new(
            &device,
            "quad_point_cloud",
            &mp_point_vertices,
        ));
        
        // Create line vertices collection
        let mut line_vertices = Vec::new();
        
        // Print debug info about lengths
        println!("DEBUG: Creating vertical lines for {} instances", instances.len());
        
        // Create vertical lines at each mesh box position with the same rotation as the boxes
        for instance in &instances {
            let pos = instance.position;
            let rotation = instance.rotation;
            
            // Convert the quaternion rotation to a 4x4 matrix
            let rotation_matrix = cgmath::Matrix4::from(rotation);
            
            // Define the start and end points in local space
            let start_local = cgmath::Point3::new(0.0, -0.5, 0.0);
            let end_local = cgmath::Point3::new(0.0, 1.5, 0.0);
            
            // Transform the points using the rotation matrix and then translate
            let start_rotated = rotation_matrix * cgmath::Vector4::new(start_local.x, start_local.y, start_local.z, 1.0);
            let end_rotated = rotation_matrix * cgmath::Vector4::new(end_local.x, end_local.y, end_local.z, 1.0);
            
            // Get the final world positions
            let start_world = cgmath::Point3::new(
                start_rotated.x + pos.x,
                start_rotated.y + pos.y,
                start_rotated.z + pos.z
            );
            
            let end_world = cgmath::Point3::new(
                end_rotated.x + pos.x,
                end_rotated.y + pos.y,
                end_rotated.z + pos.z
            );
            
            // Add the transformed line vertices
            line_vertices.push(model::LineVertex {
                position: [start_world.x, start_world.y, start_world.z],
                color: [1.0, 0.0, 0.0], // Red for high visibility
            });
            
            line_vertices.push(model::LineVertex {
                position: [end_world.x, end_world.y, end_world.z],
                color: [1.0, 0.0, 0.0],
            });
            
            // Debug info for center position
            if pos.x.abs() < 0.001 && pos.z.abs() < 0.001 {
                println!("DEBUG: Created line at center: start({:.2}, {:.2}, {:.2}) end({:.2}, {:.2}, {:.2})", 
                         start_world.x, start_world.y, start_world.z,
                         end_world.x, end_world.y, end_world.z);
            }
        }
        
        // Now add coordinate axes to the line vertices
        line_vertices.push(model::LineVertex {
            position: [0.0, 0.0, 0.0],
            color: [1.0, 0.0, 0.0],
        });
        line_vertices.push(model::LineVertex {
            position: [axis_length, 0.0, 0.0],
            color: [1.0, 0.0, 0.0],
        });
        
        // Y axis (green)
        line_vertices.push(model::LineVertex {
            position: [0.0, 0.0, 0.0],
            color: [0.0, 1.0, 0.0],
        });
        line_vertices.push(model::LineVertex {
            position: [0.0, axis_length, 0.0],
            color: [0.0, 1.0, 0.0],
        });
        
        // Z axis (blue)
        line_vertices.push(model::LineVertex {
            position: [0.0, 0.0, 0.0],
            color: [0.0, 0.0, 1.0],
        });
        line_vertices.push(model::LineVertex {
            position: [0.0, 0.0, axis_length],
            color: [0.0, 0.0, 1.0],
        });
        
        // Create the line model from the vertices
        let line_model = Some(model::LineModel::new(
            &device,
            "grid",
            &line_vertices,
        ));

        let light_uniform = LightUniform {
            position: [2.0, 2.0, 2.0],
            _padding: 0,
            color: [1.0, 1.0, 1.0],
            _padding2: 0,
        };

        let light_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Light VB"),
            contents: bytemuck::cast_slice(&[light_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let light_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: None,
            });

        let light_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &light_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: light_buffer.as_entire_binding(),
            }],
            label: None,
        });

        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_bind_group_layout,
                    &camera_bind_group_layout,
                    &light_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let render_pipeline = {
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Normal Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
            };
            pipeline::create_render_pipeline(
                &device,
                &render_pipeline_layout,
                config.format,
                Some(texture::Texture::DEPTH_FORMAT),
                &[model::ModelVertex::desc(), InstanceRaw::desc()],
                shader,
            )
        };

        // Create point pipeline with improved point size support
        let point_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Point Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout, &config_uniform.bind_group_layout],
            push_constant_ranges: &[],
        });
        
        // Get device features to check for necessary point size features
        let supports_point_size = device.features().contains(wgpu::Features::SHADER_PRIMITIVE_INDEX);
        println!("DEBUG: Device supports SHADER_PRIMITIVE_INDEX feature: {}", supports_point_size);
        
        let point_pipeline = Some({
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Point Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/point.wgsl").into()),
            };
            
            // Create a pipeline specific for point primitives with enhanced size support
            let shader_module = device.create_shader_module(shader);
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Point Render Pipeline"),
                layout: Some(&point_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: Some("vs_main"),
                    buffers: &[model_point::QuadPointVertex::desc()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        // Use alpha blending for smoother points
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::SrcAlpha,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor: wgpu::BlendFactor::One,
                                dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                                operation: wgpu::BlendOperation::Add,
                            },
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None, // Don't cull our billboard quads
                    // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                    polygon_mode: wgpu::PolygonMode::Fill,
                    // Requires Features::DEPTH_CLIP_CONTROL
                    unclipped_depth: false,
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: texture::Texture::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            })
        });
        
        // Create line pipeline
        let line_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Line Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let line_pipeline = Some({
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Line Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/line.wgsl").into()),
            };
            
            // Create a pipeline specific for line primitives
            let shader_module = device.create_shader_module(shader);
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Line Render Pipeline"),
                layout: Some(&line_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader_module,
                    entry_point: Some("vs_main"),
                    buffers: &[model::LineVertex::desc()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader_module,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::LineList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: texture::Texture::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview: None,
                cache: None,
            })
        });

        let light_render_pipeline = {
            let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Light Pipeline Layout"),
                bind_group_layouts: &[&camera_bind_group_layout, &light_bind_group_layout],
                push_constant_ranges: &[],
            });
            let shader = wgpu::ShaderModuleDescriptor {
                label: Some("Light Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("shaders/light.wgsl").into()),
            };
            pipeline::create_render_pipeline(
                &device,
                &layout,
                config.format,
                Some(texture::Texture::DEPTH_FORMAT),
                &[model::ModelVertex::desc()],
                shader,
            )
        };

        let debug_material = {
            let diffuse_bytes = include_bytes!("../res/cobble-diffuse.png");
            let normal_bytes = include_bytes!("../res/cobble-normal.png");

            let diffuse_texture = texture::Texture::from_bytes(
                &device,
                &queue,
                diffuse_bytes,
                "res/alt-diffuse.png",
                false,
            )
            .unwrap();
            let normal_texture = texture::Texture::from_bytes(
                &device,
                &queue,
                normal_bytes,
                "res/alt-normal.png",
                true,
            )
            .unwrap();

            model::Material::new(
                &device,
                "alt-material",
                diffuse_texture,
                normal_texture,
                &texture_bind_group_layout,
            )
        };

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            point_pipeline,
            line_pipeline,
            obj_model,
            point_model, // Assigned point model
            quad_point_model, // Assigned quad-based point model
            line_model,  // Assigned line model
            render_mode: RenderMode::default(),  // Default to rendering everything
            camera,
            projection,
            camera_controller,
            camera_uniform,
            config_uniform,
            camera_buffer,
            camera_bind_group,
            instances,
            instance_buffer,
            depth_texture,
            light_uniform,
            light_buffer,
            light_bind_group,
            light_render_pipeline,
            #[allow(dead_code)]
            debug_material,
            mouse_pressed: false,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        // UPDATED!
        if new_size.width > 0 && new_size.height > 0 {
            self.projection.resize(new_size.width, new_size.height);
            
            // Update aspect ratio in camera uniform
            self.camera_uniform.update_aspect_ratio(new_size.width as f32, new_size.height as f32);
            
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture =
                texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
        }
    }

    // UPDATED!
    fn input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => {
                // Handle number keys for render mode selection
                match key {
                    KeyCode::Digit0 => {
                        self.render_mode = RenderMode::All;
                        println!("Render mode: All (0)");
                        true
                    }
                    KeyCode::Digit1 => {
                        self.render_mode = RenderMode::Points;
                        println!("Render mode: Points (1)");
                        true
                    }
                    KeyCode::Digit2 => {
                        self.render_mode = RenderMode::Lines;
                        println!("Render mode: Lines (2)");
                        true
                    }
                    KeyCode::Digit3 => {
                        self.render_mode = RenderMode::Meshes;
                        println!("Render mode: Meshes (3)");
                        true
                    }
                    // Add '[' and ']' keys to decrease/increase point size
                    KeyCode::BracketLeft => {
                        // Decrease point size with a minimum of 0.001
                        let current_size = config::get_point_size();
                        let new_size = (current_size - 0.005).max(0.001);
                        config::set_point_size(new_size);
                        
                        // Update the uniform buffer
                        self.config_uniform.update(&self.queue);
                        println!("Point size decreased to: {:.3}", new_size);
                        true
                    }
                    KeyCode::BracketRight => {
                        // Increase point size with a maximum of 0.1
                        let current_size = config::get_point_size();
                        let new_size = (current_size + 0.005).min(0.1);
                        config::set_point_size(new_size);
                        
                        // Update the uniform buffer
                        self.config_uniform.update(&self.queue);
                        println!("Point size increased to: {:.3}", new_size);
                        true
                    }
                    _ => self.camera_controller.process_keyboard(*key, ElementState::Pressed),
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(key),
                        state,
                        ..
                    },
                ..
            } => self.camera_controller.process_keyboard(*key, *state),
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_controller.process_scroll(delta);
                true
            }
            WindowEvent::MouseInput {
                button: MouseButton::Left,
                state,
                ..
            } => {
                self.mouse_pressed = *state == ElementState::Pressed;
                true
            }
            _ => false,
        }
    }

    fn update(&mut self, dt: std::time::Duration) {
        // UPDATED!
        self.camera_controller.update_camera(&mut self.camera, dt);
        self.camera_uniform
            .update_view_proj(&self.camera, &self.projection);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );

        // Update the light
        let old_position: cgmath::Vector3<_> = self.light_uniform.position.into();
        self.light_uniform.position = (cgmath::Quaternion::from_axis_angle(
            (0.0, 1.0, 0.0).into(),
            cgmath::Deg(PI * dt.as_secs_f32()),
        ) * old_position)
            .into();
        self.queue.write_buffer(
            &self.light_buffer,
            0,
            bytemuck::cast_slice(&[self.light_uniform]),
        );
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
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
                    view: &self.depth_texture.view,
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
            match self.render_mode {
                RenderMode::All => {
                    // Render the light model
                    render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
                    render_pass.set_pipeline(&self.light_render_pipeline);
                    render_pass.draw_light_model(
                        &self.obj_model,
                        &self.camera_bind_group,
                        &self.light_bind_group,
                    );
                    
                    // Render the mesh model
                    render_pass.set_pipeline(&self.render_pipeline);
                    render_pass.draw_model_instanced(
                        &self.obj_model,
                        0..self.instances.len() as u32,
                        &self.camera_bind_group,
                        &self.light_bind_group,
                    );

                    // Render points if available - use the quad-based point model for better visuals
                    if let (Some(pipeline), Some(model)) = (&self.point_pipeline, &self.quad_point_model) {
                        render_pass.set_pipeline(pipeline);
                        render_pass.draw_quad_points(model, &self.camera_bind_group, &self.config_uniform.bind_group);
                    }

                    // Render lines if available
                    if let (Some(pipeline), Some(model)) = (&self.line_pipeline, &self.line_model) {
                        render_pass.set_pipeline(pipeline);
                        render_pass.draw_lines(model, &self.camera_bind_group);
                    }
                },
                RenderMode::Points => {
                    // Render only points using quad-based rendering for better visuals
                    if let (Some(pipeline), Some(model)) = (&self.point_pipeline, &self.quad_point_model) {
                        render_pass.set_pipeline(pipeline);
                        render_pass.draw_quad_points(model, &self.camera_bind_group, &self.config_uniform.bind_group);
                    }
                },
                RenderMode::Lines => {
                    // Render only lines
                    if let (Some(pipeline), Some(model)) = (&self.line_pipeline, &self.line_model) {
                        render_pass.set_pipeline(pipeline);
                        render_pass.draw_lines(model, &self.camera_bind_group);
                    }
                },
                RenderMode::Meshes => {
                    // Render the light and mesh models
                    render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
                    render_pass.set_pipeline(&self.light_render_pipeline);
                    render_pass.draw_light_model(
                        &self.obj_model,
                        &self.camera_bind_group,
                        &self.light_bind_group,
                    );
                    
                    render_pass.set_pipeline(&self.render_pipeline);
                    render_pass.draw_model_instanced(
                        &self.obj_model,
                        0..self.instances.len() as u32,
                        &self.camera_bind_group,
                        &self.light_bind_group,
                    );
                },
            }
        }
        self.queue.submit(iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log::Level::Info).expect("Could't initialize logger");
        } else {
            env_logger::init();
        }
    }

    let event_loop = EventLoop::new().unwrap();
    let title = env!("CARGO_PKG_NAME");
    let window = winit::window::WindowBuilder::new()
        .with_title(title)
        .build(&event_loop)
        .unwrap();

    #[cfg(target_arch = "wasm32")]
    {
        // Winit prevents sizing with CSS, so we have to set
        // the size manually when on web.
        use winit::dpi::PhysicalSize;
        let _ = window.request_inner_size(PhysicalSize::new(450, 400));

        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("wasm-example")?;
                let canvas = web_sys::Element::from(window.canvas()?);
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }

    let mut state = State::new(&window).await; // NEW!
    let mut last_render_time = instant::Instant::now();
    event_loop.run(move |event, control_flow| {
        match event {
            // NEW!
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion{ delta, },
                .. // We're not using device_id currently
            } => if state.mouse_pressed {
                state.camera_controller.process_mouse(delta.0, delta.1)
            }
            // UPDATED!
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() && !state.input(event) => {
                match event {
                    #[cfg(not(target_arch="wasm32"))]
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                state: ElementState::Pressed,
                                physical_key: PhysicalKey::Code(KeyCode::Escape),
                                ..
                            },
                        ..
                    } => control_flow.exit(),
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    // UPDATED!
                    WindowEvent::RedrawRequested => {
                        state.window().request_redraw();
                        let now = instant::Instant::now();
                        let dt = now - last_render_time;
                        last_render_time = now;
                        state.update(dt);
                        match state.render() {
                            Ok(_) => {}
                            // Reconfigure the surface if it's lost or outdated
                            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => state.resize(state.size),
                            // The system is out of memory, we should probably quit
                            Err(wgpu::SurfaceError::OutOfMemory | wgpu::SurfaceError::Other) => control_flow.exit(),
                            // We're ignoring timeouts
                            Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }).unwrap();
}
