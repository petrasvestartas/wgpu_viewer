use std::{f32::consts::PI, iter};

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
mod pipeline;
mod renderer;
mod resources;
mod texture;
pub mod geometry_generator;
pub mod demo_geometries;

use cgmath::prelude::*;
use wgpu::util::DeviceExt;
use crate::model::{DrawModel, DrawLight, Vertex};
// No need to import DrawLines since we're not using the trait directly
use crate::model_point::{DrawQuadPoints, QuadPointModel, PointVertex as MPPointVertex};
use crate::model_pipe::{DrawPipes, PipeVertex};
use crate::model_polygon::{DrawPolygons, PolygonVertex};
use crate::instance::Instance;
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
    pipe_pipeline: Option<wgpu::RenderPipeline>,  // Pipeline for 3D pipe lines
    polygon_pipeline: Option<wgpu::RenderPipeline>,  // Pipeline for polygons
    obj_model: model::Model,
    point_model: Option<model::PointModel>,      // Optional point cloud model
    quad_point_model: Option<model_point::QuadPointModel>, // Optional quad-based point model for billboard rendering
    line_model: Option<model::LineModel>,        // Optional line model
    pipe_model: Option<model_pipe::PipeModel>, // 3D pipe model with thickness
    polygon_model: Option<model_polygon::PolygonModel>, // Polygon model for flat surfaces
    render_mode: RenderMode,                     // Current rendering mode
    camera: camera::Camera,                      // UPDATED!
    projection: camera::Projection,              // NEW!
    camera_controller: camera::CameraController, // UPDATED!
    camera_uniform: CameraUniform,
    // Config is now hardcoded in the shader
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

        // Initialize arcball camera
        let camera_target = cgmath::Point3::new(0.0, 0.0, 0.0); // Target the origin
        let camera_position = cgmath::Point3::new(0.0, 10.0, 10.0); // Position from above and behind
        let mut camera = camera::Camera::new(camera_position, camera_target);
        
        // Set explicit camera orientation (looking from above and behind)
        camera.pitch = cgmath::Rad(-std::f32::consts::FRAC_PI_4); // -45 degrees (looking down at an angle)
        camera.yaw = cgmath::Rad(0.0); // 0 degrees rotation (flipped 180 degrees from previous setting)
        camera.update_position(); // Update position after changing orientation
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
        // Point size is now hardcoded in the shader

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
        
        // Create the grid line model using our geometry generator
        println!("Creating 10x10 grid of lines with 1 unit spacing");
        let line_model = Some(geometry_generator::create_grid_lines(&device));

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

        const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;
        
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
                Some(DEPTH_FORMAT),
                &[model::ModelVertex::desc(), InstanceRaw::desc()],
                shader,
            )
        };

        // Create point pipeline with improved point size support
        let point_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Point Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
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
                    format: DEPTH_FORMAT,
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
                    format: DEPTH_FORMAT,
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

        // Create the 3D pipeline
        let pipe_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipe_pipeline_layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipe_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("pipe_pipeline"),
            layout: Some(&pipe_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("pipe_shader"),
                    source: wgpu::ShaderSource::Wgsl(include_str!("shaders/pipe.wgsl").into()),
                }),
                entry_point: Some("vs_main"),
                buffers: &[PipeVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("pipe_shader"),
                    source: wgpu::ShaderSource::Wgsl(include_str!("shaders/pipe.wgsl").into()),
                }),
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
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
        });

        // Create polygon pipeline
        let polygon_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("polygon_pipeline_layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });
        
        let polygon_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            cache: None,
            label: Some("polygon_pipeline"),
            layout: Some(&polygon_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("polygon_shader"),
                    source: wgpu::ShaderSource::Wgsl(include_str!("shaders/polygon.wgsl").into()),
                }),
                entry_point: Some("vs_main"),
                buffers: &[PolygonVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &device.create_shader_module(wgpu::ShaderModuleDescriptor {
                    label: Some("polygon_shader"),
                    source: wgpu::ShaderSource::Wgsl(include_str!("shaders/polygon.wgsl").into()),
                }),
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None, // No culling to start with, easier for debugging
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
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
                Some(DEPTH_FORMAT),
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
            pipe_pipeline: Some(pipe_pipeline),
            polygon_pipeline: Some(polygon_pipeline),
            obj_model,
            point_model, // Assigned point model
            quad_point_model, // Assigned quad-based point model
            line_model,  // Assigned line model
            pipe_model: None, // Will be generated from line model when needed
            polygon_model: None, // Will be created separately when needed
            render_mode: RenderMode::default(),  // Default to rendering everything
            camera,
            projection,
            camera_controller,
            camera_uniform,
            // Config values now hardcoded in the shader
            camera_buffer,
            camera_bind_group,
            instances,
            #[allow(dead_code)]
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
        self.window
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
                        // Force creation of pipe lines when switching to Lines mode
                        self.create_pipes_from_lines();
                        true
                    }
                    KeyCode::Digit3 => {
                        self.render_mode = RenderMode::RegularLines;
                        println!("Render mode: Regular Lines (3)");
                        true
                    }
                    KeyCode::Digit4 => {
                        self.render_mode = RenderMode::Meshes;
                        println!("Render mode: Meshes (4)");
                        true
                    }
                    KeyCode::Digit5 => {
                        self.render_mode = RenderMode::Polygons;
                        println!("Render mode: Polygons (5)");
                        // Create sample polygon when switching to polygon mode
                        self.create_sample_polygon();
                        true
                    }
                    // Point size is now hardcoded directly in the shader
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
                button,
                state,
                ..
            } => {
                // For arcball camera, pass all mouse buttons to the camera controller
                if self.camera_controller.process_mouse_button(*state, *button) {
                    return true;
                }
                // Still maintain the mouse_pressed state for other functionality
                if *button == MouseButton::Left {
                    self.mouse_pressed = *state == ElementState::Pressed;
                    return true;
                }
                false
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
    
    // The extract_line_vertices_from_buffer function has been removed as it was unused
    
    /// Create a grid of polygons matching other geometries
    fn create_sample_polygon(&mut self) {
        const SCALE_FACTOR: f32 = 0.25; // Size factor for polygon

        // Collect all polygon vertex data and indices
        let mut all_vertices = Vec::new();
        let mut all_indices = Vec::new();
        let mut vertex_count: u32 = 0;
        
        // Use the same instances stored in self.instances
        // This guarantees the same positions and rotations as other geometry
        println!("Creating polygon grid with {} instances", self.instances.len());
        
        // Create polygons at each instance position with the same rotation as other geometries
        for instance in &self.instances {
            let pos = instance.position;
            let rotation = instance.rotation;
            
            // Create a single color for the entire polygon based on its position
            // Use position to generate consistent colors
            let x_normalized = (pos.x + 15.0) / 30.0;  // Normalize x in [-15,15] to [0,1]
            let z_normalized = (pos.z + 15.0) / 30.0;  // Normalize z in [-15,15] to [0,1]
            let color = [
                x_normalized, 
                (1.0 - x_normalized) * z_normalized,
                1.0 - z_normalized,
            ];
            
            // Convert the quaternion rotation to a 4x4 matrix - EXACTLY like in line code
            let rotation_matrix = cgmath::Matrix4::from(rotation);
            
            // Define the same start/end points as the lines to ensure exact alignment
            // Lines use these exact coordinates
            let start_local = cgmath::Point3::new(0.0, -0.5, 0.0);
            let end_local = cgmath::Point3::new(0.0, 1.5, 0.0);
            
            // Create polygon vertices around the same vertical line
            let vertex_positions = [
                // Top vertex at the same position as the line end
                cgmath::Point3::new(0.0, end_local.y, 0.0),
                
                // Create points in a circle around the line at middle height
                cgmath::Point3::new(SCALE_FACTOR, 0.5, 0.0),
                cgmath::Point3::new(0.0, 0.5, SCALE_FACTOR),
                cgmath::Point3::new(-SCALE_FACTOR, 0.5, 0.0),
                cgmath::Point3::new(0.0, 0.5, -SCALE_FACTOR),
                
                // Bottom vertex at the same position as the line start
                cgmath::Point3::new(0.0, start_local.y, 0.0),
            ];
            
            // Add vertices for this polygon instance exactly the same way as lines
            for local_pos in &vertex_positions {
                // Transform using the rotation matrix and then translate - SAME as lines
                let rotated_pos = rotation_matrix * cgmath::Vector4::new(
                    local_pos.x, local_pos.y, local_pos.z, 1.0
                );
                
                // Apply position offset
                let world_pos = cgmath::Point3::new(
                    rotated_pos.x + pos.x,
                    rotated_pos.y + pos.y,
                    rotated_pos.z + pos.z
                );
                
                // Add transformed vertex with instance color
                all_vertices.push(model_polygon::PolygonVertex {
                    position: [world_pos.x, world_pos.y, world_pos.z],
                    color,
                });
            }
            
            // Create triangles for a pyramid-like shape connecting vertices
            // Top to middle points
            all_indices.extend_from_slice(&[
                // Top triangles (connect top to side points)
                vertex_count, vertex_count + 1, vertex_count + 2,
                vertex_count, vertex_count + 2, vertex_count + 3,
                vertex_count, vertex_count + 3, vertex_count + 4,
                vertex_count, vertex_count + 4, vertex_count + 1,
                
                // Bottom triangles (connect bottom to side points)
                vertex_count + 5, vertex_count + 2, vertex_count + 1,
                vertex_count + 5, vertex_count + 3, vertex_count + 2,
                vertex_count + 5, vertex_count + 4, vertex_count + 3,
                vertex_count + 5, vertex_count + 1, vertex_count + 4,
            ]);
            
            // Update the vertex count for the next polygon
            vertex_count += 6;
        }

        println!("Created {} polygons with {} vertices total", self.instances.len(), all_vertices.len());
        
        // Create the polygon model from our vertices and indices
        self.polygon_model = Some(model_polygon::PolygonModel::new(
            &self.device,
            "polygon_grid",
            &all_vertices,
            &all_indices,
        ));
    }

    /// Convert regular lines from line_model into 3D pipe lines
    fn create_pipes_from_lines(&mut self) {
        // Lazily create a pipe model from the line model if needed
        if self.line_model.is_some() && self.pipe_model.is_none() {
            println!("Creating pipe model from line model...");
            // Instead of extracting vertices from the buffer, let's create the same source vertices
            // that we used to create the line model in the first place
            
            // Define the WGPU instances configuration from original code
            const NUM_INSTANCES_PER_ROW: u32 = 10;
            const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(
                NUM_INSTANCES_PER_ROW as f32 * 0.5, 
                0.0, 
                NUM_INSTANCES_PER_ROW as f32 * 0.5
            );
            
            let mut instances = Vec::new();
            
            // Generate the same instance data as the original code
            for z in 0..NUM_INSTANCES_PER_ROW {
                for x in 0..NUM_INSTANCES_PER_ROW {
                    let position = cgmath::Vector3 {
                        x: x as f32 * 3.0,
                        y: 0.0,
                        z: z as f32 * 3.0,
                    } - INSTANCE_DISPLACEMENT;
                    
                    // Create instance with position and rotation from original code
                    let rotation = if position.is_zero() {
                        cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
                    } else {
                        cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
                    };
                    
                    instances.push(Instance {
                        position,
                        rotation,
                    });
                }
            }
            
            println!("Creating vertical lines for {} instances", instances.len());
            
            // Create line segments for the pipe model
            let mut line_segments = Vec::new();
            
            // Extract vertices and create pipe segments that match line segments exactly
            if let Some(_line_model) = &self.line_model {
                println!("Creating pipe model from line model...");
                
                // Use the instances directly without an extra reference
                let instances = &self.instances;
                println!("Adding vertical lines for {} instances", instances.len());
                
                // Create vertical lines at each mesh box position with the same rotation as the boxes
                for instance in instances {
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
                    
                    // Add the segment directly (without going through LineVertex intermediate)
                    line_segments.push(model_pipe::PipeSegment {
                        start: [start_world.x, start_world.y, start_world.z],
                        end: [end_world.x, end_world.y, end_world.z],
                        color: model_pipe::PIPE_COLOR,
                        radius: model_pipe::PIPE_RADIUS,
                    });
                }
            }
            
            // Add coordinate axes to the pipe segments
            let axis_length = 5.0;
            
            // X axis (red)
            line_segments.push(model_pipe::PipeSegment {
                start: [0.0, 0.0, 0.0],
                end: [axis_length, 0.0, 0.0],
                color: [1.0, 0.0, 0.0], // Red for X axis
                radius: model_pipe::PIPE_RADIUS,
            });
            
            // Y axis (green)
            line_segments.push(model_pipe::PipeSegment {
                start: [0.0, 0.0, 0.0],
                end: [0.0, axis_length, 0.0],
                color: [0.0, 1.0, 0.0], // Green for Y axis
                radius: model_pipe::PIPE_RADIUS,
            });
            
            // Z axis (blue) - Make sure it's highly visible
            line_segments.push(model_pipe::PipeSegment {
                start: [0.0, 0.0, 0.0],
                end: [0.0, 0.0, axis_length],
                color: [0.0, 0.0, 1.0], // Blue for Z axis
                radius: model_pipe::PIPE_RADIUS * 1.2, // Slightly larger radius for better visibility
            });
            
            // Extra blue line for testing
            line_segments.push(model_pipe::PipeSegment {
                start: [0.0, 0.0, 0.0],
                end: [0.0, 0.0, -axis_length], // Negative z direction
                color: [0.0, 0.2, 1.0], // Light blue for negative Z
                radius: model_pipe::PIPE_RADIUS,
            });
            
            println!("Creating 3D pipes from {} line segments", line_segments.len());
            
            // Create the pipe model with the line segments
            self.pipe_model = Some(model_pipe::PipeModel::new(
                &self.device,
                "pipe_model",
                &line_segments,
                model_pipe::PIPE_RESOLUTION,
            ));
        }
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
                        render_pass.draw_quad_points(model, &self.camera_bind_group);
                    }

                    // Create pipe lines from line data if needed
                    if self.pipe_model.is_none() && self.line_model.is_some() {
                        // Lazily create pipe lines from the line model
                        drop(render_pass); // Release the render pass to modify state
                        self.create_pipes_from_lines();
                        // Re-acquire render pass
                        render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Render Pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                                view: &self.depth_texture.view,
                                depth_ops: Some(wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                }),
                                stencil_ops: None,
                            }),
                            occlusion_query_set: None,
                            timestamp_writes: None,
                        });
                        
                        // Need to reset these since we dropped the render pass
                        if self.render_mode == RenderMode::All {
                            // Reset the pipeline and instance buffer
                            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
                        }
                    }
                    
                    // Render 3D pipe lines instead of regular lines
                    if let (Some(pipeline), Some(model)) = (&self.pipe_pipeline, &self.pipe_model) {
                        render_pass.set_pipeline(pipeline);
                        render_pass.draw_pipes(model, &self.camera_bind_group);
                    }
                    
                    // Render polygons if available
                    if let (Some(pipeline), Some(model)) = (&self.polygon_pipeline, &self.polygon_model) {
                        render_pass.set_pipeline(pipeline);
                        render_pass.draw_polygons(model, &self.camera_bind_group);
                    }
                    
                    // Commented out regular line rendering in favor of 3D pipe lines
                    /*
                    if let (Some(pipeline), Some(model)) = (&self.line_pipeline, &self.line_model) {
                        render_pass.set_pipeline(pipeline);
                        render_pass.draw_lines(model, &self.camera_bind_group);
                    }
                    */
                },
                RenderMode::Points => {
                    // Render only points using quad-based rendering for better visuals
                    if let (Some(pipeline), Some(model)) = (&self.point_pipeline, &self.quad_point_model) {
                        render_pass.set_pipeline(pipeline);
                        render_pass.draw_quad_points(model, &self.camera_bind_group);
                    }
                },
                RenderMode::Lines => {
                    // Create pipe lines from line data if needed
                    if self.pipe_model.is_none() && self.line_model.is_some() {
                        // Lazily create pipe lines from the line model
                        drop(render_pass); // Release the render pass to modify state
                        self.create_pipes_from_lines();
                        // Re-acquire render pass
                        render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Render Pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                                view: &self.depth_texture.view,
                                depth_ops: Some(wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                }),
                                stencil_ops: None,
                            }),
                            occlusion_query_set: None,
                            timestamp_writes: None,
                        });
                    }
                    
                    // Render 3D pipe lines instead of regular lines
                    if let (Some(pipeline), Some(model)) = (&self.pipe_pipeline, &self.pipe_model) {
                        render_pass.set_pipeline(pipeline);
                        render_pass.draw_pipes(model, &self.camera_bind_group);
                    }
                    // Commented out regular line rendering in favor of 3D pipe lines
                    /*
                    if let (Some(pipeline), Some(model)) = (&self.line_pipeline, &self.line_model) {
                        render_pass.set_pipeline(pipeline);
                        render_pass.draw_lines(model, &self.camera_bind_group);
                    }
                    */
                },
                RenderMode::RegularLines => {
                    // Render regular lines without 3D pipes
                    if let (Some(pipeline), Some(model)) = (&self.line_pipeline, &self.line_model) {
                        render_pass.set_pipeline(pipeline);
                        
                        // Use the correct type - model_line::LineModel is expected by draw_lines
                        // Draw the model without using draw_lines trait which has type mismatch
                        render_pass.set_vertex_buffer(0, model.vertex_buffer.slice(..));
                        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
                        render_pass.draw(0..model.num_vertices, 0..1);
                    }
                },
                RenderMode::Polygons => {
                    // Create sample polygon if it doesn't exist
                    if self.polygon_model.is_none() {
                        // Release the render pass to modify state
                        drop(render_pass);
                        self.create_sample_polygon();
                        // Re-acquire render pass
                        render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                            label: Some("Render Pass"),
                            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                                view: &view,
                                resolve_target: None,
                                ops: wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                },
                            })],
                            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                                view: &self.depth_texture.view,
                                depth_ops: Some(wgpu::Operations {
                                    load: wgpu::LoadOp::Load,
                                    store: wgpu::StoreOp::Store,
                                }),
                                stencil_ops: None,
                            }),
                            occlusion_query_set: None,
                            timestamp_writes: None,
                        });
                    }
                    
                    // Render the polygon model
                    if let (Some(pipeline), Some(model)) = (&self.polygon_pipeline, &self.polygon_model) {
                        render_pass.set_pipeline(pipeline);
                        render_pass.draw_polygons(model, &self.camera_bind_group);
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
        .with_inner_size(winit::dpi::PhysicalSize::new(1920, 1080))
        .build(&event_loop)
        .unwrap();

    #[cfg(not(target_arch = "wasm32"))]
    {
        window.focus_window();
    }

    #[cfg(target_arch = "wasm32")]
    {
        // For web, we want to use the full browser window size
        use winit::dpi::PhysicalSize;
        
        // Get the browser window dimensions
        let browser_window = web_sys::window().expect("Unable to get browser window");
        let width = browser_window.inner_width().unwrap().as_f64().unwrap() as u32;
        let height = browser_window.inner_height().unwrap().as_f64().unwrap() as u32;
        
        // Set canvas to browser window size
        let _ = window.request_inner_size(PhysicalSize::new(width, height));
        
        use winit::platform::web::WindowExtWebSys;
        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                // Add CSS to make canvas fullscreen
                let style = doc.create_element("style").unwrap();
                style.set_text_content(Some("
                    html, body {
                        margin: 0 !important;
                        padding: 0 !important;
                        width: 100% !important;
                        height: 100% !important;
                        overflow: hidden !important;
                    }
                    canvas {
                        display: block !important;
                        width: 100% !important;
                        height: 100% !important;
                    }
                "));
                
                // Append style to document
                doc.body().unwrap().append_child(&style).ok();
                
                // Append canvas to document body or container
                let canvas = web_sys::Element::from(window.canvas()?);
                canvas.set_id("wgpu-canvas");
                
                // Try to find the target element, fall back to body if not found
                let dst = doc.get_element_by_id("wasm-example")
                    .unwrap_or_else(|| doc.body().unwrap().into());
                
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
            
        // In WASM, we don't need to manually request a redraw on resize
        // as the browser will handle this automatically
        let resize_closure = Closure::wrap(Box::new(move |_: web_sys::Event| {
            // We don't need to do anything here, canvas CSS handles resizing
        }) as Box<dyn FnMut(_)>);
        
        web_sys::window()
            .unwrap()
            .add_event_listener_with_callback("resize", resize_closure.as_ref().unchecked_ref())
            .expect("Failed to add resize event listener");
            
        // Prevent closure from being garbage collected
        resize_closure.forget();
    }

    let mut state = State::new(&window).await; // NEW!
    let mut last_render_time = instant::Instant::now();
    event_loop.run(move |event, control_flow| {
        match event {
            // NEW!
            Event::DeviceEvent {
                event: DeviceEvent::MouseMotion{ delta, },
                .. // We're not using device_id currently
            } => {
                // Let the camera controller handle mouse movements directly
                // It will determine whether to rotate based on if is_rotating is true
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
