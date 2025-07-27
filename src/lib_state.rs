use crate::camera;
use crate::instance::{Instance, InstanceRaw};
use crate::model;
use crate::model_line;
use crate::model_pipe;
use crate::model_point;
use crate::model_polygon;
use crate::lib_pipeline;
use crate::lib_render::{CameraUniform, LightUniform};
use crate::RenderMode;
use crate::model::Vertex; // Import Vertex trait for desc() method
use cgmath::prelude::*;
use wgpu::util::DeviceExt;
use winit::window::Window;

/// State struct for the application
#[allow(dead_code)]
pub struct State<'a> {
    pub window: &'a Window,
    pub surface: wgpu::Surface<'a>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub render_pipeline: wgpu::RenderPipeline,
    pub point_pipeline: Option<wgpu::RenderPipeline>,
    pub line_pipeline: Option<wgpu::RenderPipeline>,
    pub pipe_pipeline: Option<wgpu::RenderPipeline>,
    pub polygon_pipeline: Option<wgpu::RenderPipeline>,
    pub obj_model: model::Model,
    pub additional_mesh_models: Vec<model::Model>,
    pub point_model: Option<model::PointModel>,
    pub quad_point_model: Option<model_point::QuadPointModel>,
    pub line_model: Option<model::LineModel>,
    pub pipe_model: Option<model_pipe::PipeModel>,
    pub polygon_model: Option<model_polygon::PolygonModel>,
    pub render_mode: RenderMode,
    pub camera: camera::Camera,
    pub projection: camera::Projection,
    pub camera_controller: camera::CameraController,
    pub camera_uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
    pub camera_bind_group: wgpu::BindGroup,
    pub instances: Vec<Instance>,
    #[allow(dead_code)]
    pub instance_buffer: wgpu::Buffer,
    pub depth_texture_view: wgpu::TextureView,
    pub size: winit::dpi::PhysicalSize<u32>,
    pub light_uniform: LightUniform,
    pub light_buffer: wgpu::Buffer,
    pub light_bind_group: wgpu::BindGroup,
    pub light_render_pipeline: wgpu::RenderPipeline,
    pub mouse_pressed: bool,
}

impl<'a> State<'a> {
    /// Create a new State instance with full GPU initialization
    pub async fn new(window: &'a Window) -> Result<State<'a>, Box<dyn std::error::Error>> {
        let size = window.inner_size();

        // Initialize GPU context
        let (instance, surface, adapter, device, queue, config) = 
            init_gpu_context(window, size).await?;

        // Configure the surface with the device - this was missing and causing the macOS crash
        surface.configure(&device, &config);

        // Initialize camera system
        let (camera, projection, camera_controller, camera_uniform, camera_buffer, camera_bind_group, camera_bind_group_layout) = 
            init_camera_system(&device, &config);

        // Initialize lighting system
        let (light_uniform, light_buffer, light_bind_group, light_bind_group_layout) = 
            init_lighting_system(&device);

        // Create depth texture
        let depth_texture_view = create_depth_texture(&device, &config);
        
        // Initialize all rendering pipelines
        let (render_pipeline, point_pipeline, line_pipeline, pipe_pipeline, polygon_pipeline, light_render_pipeline) = 
            init_pipelines(&device, &config, &camera_bind_group_layout, &light_bind_group_layout).await;

        // Load default models and create instances
        let (obj_model, instances, instance_buffer) = 
            init_models_and_instances(&device, &queue).await;
        
        // Create grid lines for visualization
        let line_model = Some(crate::geometry_generator::create_grid_lines(&device));

        Ok(State {
            window,
            surface,
            device,
            queue,
            config,
            render_pipeline,
            point_pipeline,
            line_pipeline,
            pipe_pipeline,
            polygon_pipeline,
            obj_model,
            additional_mesh_models: Vec::new(),
            point_model: None,
            quad_point_model: None,
            line_model,
            pipe_model: None,
            polygon_model: None,
            render_mode: RenderMode::default(),
            camera,
            projection,
            camera_controller,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            instances,
            instance_buffer,
            depth_texture_view,
            size,
            light_uniform,
            light_buffer,
            light_bind_group,
            light_render_pipeline,
            mouse_pressed: false,
        })
    }
}

/// Initialize GPU context (instance, surface, adapter, device, queue, config)
async fn init_gpu_context(
    window: &Window, 
    size: winit::dpi::PhysicalSize<u32>
) -> Result<(wgpu::Instance, wgpu::Surface, wgpu::Adapter, wgpu::Device, wgpu::Queue, wgpu::SurfaceConfiguration), Box<dyn std::error::Error>> {
    // The instance is a handle to our GPU
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        #[cfg(not(target_arch = "wasm32"))]
        backends: wgpu::Backends::PRIMARY,
        #[cfg(target_arch = "wasm32")]
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..Default::default()
    });

    let surface = instance.create_surface(window)
        .map_err(|e| {
            #[cfg(target_arch = "wasm32")]
            {
                web_sys::console::error_1(&format!("Failed to create WebGPU surface: {}. This browser may not support WebGPU yet. Try Chrome/Chromium for the best WebGPU experience.", e).into());
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                eprintln!("Failed to create surface: {}", e);
            }
            e
        })?;

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
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits())
                } else {
                    wgpu::Limits::default()
                },
                memory_hints: Default::default(),
            },
            None,
        )
        .await
        .unwrap();

    let surface_caps = surface.get_capabilities(&adapter);
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

    Ok((instance, surface, adapter, device, queue, config))
}

/// Initialize camera system (camera, projection, controller, uniform, buffer, bind group, layout)
fn init_camera_system(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
) -> (camera::Camera, camera::Projection, camera::CameraController, CameraUniform, wgpu::Buffer, wgpu::BindGroup, wgpu::BindGroupLayout) {
    // Initialize arcball camera
    let camera_target = cgmath::Point3::new(0.0, 0.0, 0.0);
    let camera_position = cgmath::Point3::new(0.0, 10.0, 10.0);
    let mut camera = camera::Camera::new(camera_position, camera_target);
    camera.update_position();

    let projection = camera::Projection::new(config.width, config.height, cgmath::Deg(45.0), 0.1, 100.0);
    let camera_controller = camera::CameraController::new(4.0, 0.4);

    let mut camera_uniform = CameraUniform::new();
    camera_uniform.update_view_proj(&camera, &projection);
    camera_uniform.update_aspect_ratio(config.width as f32, config.height as f32);

    let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Camera Buffer"),
        contents: bytemuck::cast_slice(&[camera_uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

    (camera, projection, camera_controller, camera_uniform, camera_buffer, camera_bind_group, camera_bind_group_layout)
}

/// Initialize lighting system (uniform, buffer, bind group, layout)
fn init_lighting_system(device: &wgpu::Device) -> (LightUniform, wgpu::Buffer, wgpu::BindGroup, wgpu::BindGroupLayout) {
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

    let light_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

    (light_uniform, light_buffer, light_bind_group, light_bind_group_layout)
}

/// Create depth texture
fn create_depth_texture(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> wgpu::TextureView {
    let depth_size = wgpu::Extent3d {
        width: config.width.max(1),
        height: config.height.max(1),
        depth_or_array_layers: 1,
    };

    let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("depth_texture"),
        size: depth_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[wgpu::TextureFormat::Depth32Float],
    });

    depth_texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// Initialize all rendering pipelines
async fn init_pipelines(
    device: &wgpu::Device,
    config: &wgpu::SurfaceConfiguration,
    camera_bind_group_layout: &wgpu::BindGroupLayout,
    light_bind_group_layout: &wgpu::BindGroupLayout,
) -> (
    wgpu::RenderPipeline,
    Option<wgpu::RenderPipeline>,
    Option<wgpu::RenderPipeline>,
    Option<wgpu::RenderPipeline>,
    Option<wgpu::RenderPipeline>,
    wgpu::RenderPipeline,
) {
    const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    // Create empty texture bind group layout
    let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[],
        label: Some("texture_bind_group_layout"),
    });

    // Main render pipeline
    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[camera_bind_group_layout, light_bind_group_layout],
        push_constant_ranges: &[],
    });

    let render_pipeline = {
        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("Normal Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shader.wgsl").into()),
        };
        lib_pipeline::create_render_pipeline(
            device,
            &render_pipeline_layout,
            config.format,
            Some(DEPTH_FORMAT),
            &[model::ModelVertex::desc(), InstanceRaw::desc()],
            shader,
        )
    };

    // Point pipeline
    let point_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Point Pipeline Layout"),
        bind_group_layouts: &[camera_bind_group_layout],
        push_constant_ranges: &[],
    });

    let point_pipeline = Some({
        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("Point Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/point.wgsl").into()),
        };
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

    // Line pipeline
    let line_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Line Pipeline Layout"),
        bind_group_layouts: &[camera_bind_group_layout],
        push_constant_ranges: &[],
    });

    let line_pipeline = Some({
        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("Line Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/line.wgsl").into()),
        };
        let shader_module = device.create_shader_module(shader);
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Line Render Pipeline"),
            layout: Some(&line_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                buffers: &[model_line::LineVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
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

    // Pipe pipeline
    let pipe_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Pipe Pipeline Layout"),
        bind_group_layouts: &[camera_bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipe_pipeline = Some({
        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("Pipe Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/pipe.wgsl").into()),
        };
        let shader_module = device.create_shader_module(shader);
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Pipe Render Pipeline"),
            layout: Some(&pipe_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                buffers: &[model_pipe::PipeVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
            cache: None,
        })
    });

    // Polygon pipeline
    let polygon_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Polygon Pipeline Layout"),
        bind_group_layouts: &[camera_bind_group_layout, light_bind_group_layout],
        push_constant_ranges: &[],
    });

    let polygon_pipeline = Some({
        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("Polygon Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/polygon.wgsl").into()),
        };
        let shader_module = device.create_shader_module(shader);
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Polygon Render Pipeline"),
            layout: Some(&polygon_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader_module,
                entry_point: Some("vs_main"),
                buffers: &[model_polygon::PolygonVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader_module,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
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
            cache: None,
        })
    });

    // Light render pipeline
    let light_render_pipeline = {
        let shader = wgpu::ShaderModuleDescriptor {
            label: Some("Light Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/light.wgsl").into()),
        };
        lib_pipeline::create_render_pipeline(
            device,
            &render_pipeline_layout,
            config.format,
            Some(DEPTH_FORMAT),
            &[model::ModelVertex::desc(), InstanceRaw::desc()],
            shader,
        )
    };

    (render_pipeline, point_pipeline, line_pipeline, pipe_pipeline, polygon_pipeline, light_render_pipeline)
}

/// Initialize models and instances
async fn init_models_and_instances(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> (model::Model, Vec<Instance>, wgpu::Buffer) {
    // Create empty texture bind group layout for model loading
    let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        entries: &[],
        label: Some("texture_bind_group_layout"),
    });
    
    // Load default cube model
    let obj_model = crate::resources::load_model("cube.obj", device, queue, &texture_bind_group_layout)
        .await
        .expect("Failed to load cube model");

    // Create single instance at origin
    let instances = vec![Instance {
        position: cgmath::Vector3::new(0.0, 0.0, 0.0),
        rotation: cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0)),
    }];

    let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
    let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Instance Buffer"),
        contents: bytemuck::cast_slice(&instance_data),
        usage: wgpu::BufferUsages::VERTEX,
    });

    (obj_model, instances, instance_buffer)
}
