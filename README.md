# WebGPU Viewer

<img width="2560" height="1600" alt="Screenshot from 2025-07-11 21-09-34" src="https://github.com/user-attachments/assets/1c1e0911-6593-4672-9425-35f32167ab0e" />

## Features

- **Pure WebGPU**: No WebGL fallback - uses modern WebGPU API for optimal performance
- **Cross-platform**: Runs natively on Windows, macOS, Linux, and in web browsers
- **Multiple geometry types**: Points, lines, pipes, polygons, and meshes
- **Interactive camera**: Arcball camera with mouse and keyboard controls
- **Hot reload**: Live geometry updates from JSON files (web version)
- **Render modes**: Switch between different geometry visualization modes
- **JSON geometry loading**: Load complex geometry data from JSON files
- **OpenModel integration**: Advanced pipe mesh generation using OpenModel geometry kernel

## Architecture Overview

### Code Organization

The project has been modularized from a large monolithic `lib.rs` file into focused modules:

#### **Core Modules (Original)**
- `camera.rs` - Camera system (arcball camera, projection)
- `instance.rs` - Instance management for rendering multiple objects
- `model.rs` - Base model loading and rendering traits
- `model_line.rs` - Line geometry implementation
- `model_pipe.rs` - 3D pipe geometry implementation
- `model_point.rs` - Point cloud rendering
- `model_polygon.rs` - Polygon mesh rendering
- `model_mesh.rs` - Mesh-based 3D models (polygonal geometry)
- `resources.rs` - Asset loading (OBJ files, textures)
- `geometry_loader.rs` - JSON geometry file parsing
- `geometry_generator.rs` - Procedural geometry generation (grid lines, axes)

#### **Extracted Modules (lib_* prefix)**
- `lib_state.rs` - GPU state initialization and management
- `lib_app.rs` - Application runner and event loop
- `lib_render.rs` - Rendering engine, draw calls, and GPU uniforms (CameraUniform, LightUniform)
- `lib_input.rs` - Input handling and camera controls
- `lib_geometry_manager.rs` - Geometry loading and management
- `lib_hot_reload.rs` - Hot reload functionality
- `lib_pipeline.rs` - GPU pipeline creation utilities

#### **Main Entry Point**
- `lib.rs` - Clean main entry point (~120 lines, delegates to modules)

### Recent Restructuring (2025-01)

The project underwent significant modularization and cleanup:

#### **✅ Completed Improvements**
- **Modularization**: Extracted large `lib.rs` file (~2000 lines) into focused modules with `lib_` prefix
- **File Cleanup**: Removed unused files (`demo_geometries.rs`)
- **Module Merge**: Consolidated `renderer.rs` into `lib_render.rs` for better separation of concerns
- **Consistent Naming**: All extracted modules follow `lib_` prefix convention
- **Pure WebGPU**: Verified no WebGL fallback code remains in codebase
- **Documentation**: Updated README with comprehensive architecture overview

#### **🎯 Benefits Achieved**
- **Maintainability**: Clear separation of concerns with focused modules
- **Readability**: Each module has a single responsibility
- **Consistency**: Unified naming convention for extracted modules
- **Performance**: Pure WebGPU implementation without WebGL fallback
- **Documentation**: Complete execution flow and platform split documentation

### Execution Flow

#### **1. Application Startup**
```
main() [main.rs]
  ↓
lib_app::run() [lib_app.rs]
  ↓
State::new() [lib_state.rs]
  ↓
init_gpu_context() → init_camera_system() → init_lighting_system() → init_pipelines()
```

#### **2. State Initialization Sequence**
```
State::new() [lib_state.rs]
├── init_gpu_context()
│   ├── Create wgpu::Instance
│   ├── Create wgpu::Surface (from window)
│   ├── Request wgpu::Adapter
│   ├── Request wgpu::Device & wgpu::Queue
│   └── Configure surface
├── init_camera_system()
│   ├── Create Camera (position, target)
│   ├── Create Projection (perspective)
│   ├── Create CameraController (input handling)
│   └── Create camera uniform buffer & bind group
├── init_lighting_system()
│   ├── Create LightUniform
│   └── Create light buffer & bind group
├── init_pipelines()
│   ├── Create render pipeline (meshes)
│   ├── Create point pipeline (point clouds)
│   ├── Create line pipeline (grid lines)
│   ├── Create pipe pipeline (3D pipes)
│   ├── Create polygon pipeline (polygons)
│   └── Create light render pipeline
├── init_models_and_instances()
│   ├── Load default cube.obj model
│   └── Create instance buffer
└── Create grid lines (geometry_generator::create_grid_lines)
```

#### **3. Geometry Loading Flow**
```
lib_geometry_manager::load_geometries_from_file() [lib_geometry_manager.rs]
├── Parse JSON geometry file
├── Process meshes → create GPU buffers
├── Process points → create quad point models
├── Process pipes → create 3D pipe meshes (OpenModel integration)
└── Process polygons → create polygon models
```

#### **4. Main Event Loop**
```
Event Loop [lib_app.rs]
├── Handle WindowEvent::Resized → state.resize()
├── Handle WindowEvent::RedrawRequested → state.render()
├── Handle DeviceEvent::MouseMotion → lib_input::handle_mouse_input()
├── Handle WindowEvent::KeyboardInput → lib_input::handle_keyboard_input()
└── Handle WindowEvent::CloseRequested → exit
```

#### **5. Rendering Pipeline**
```
state.render() [lib.rs] → lib_render::render() [lib_render.rs]
├── Create command encoder
├── Handle render mode-specific setup
│   ├── RenderMode::All → create pipes from lines if needed
│   └── RenderMode::Polygons → create sample polygon if needed
├── Begin render pass
├── Render based on mode:
│   ├── render_all_mode() → meshes + points + pipes + polygons + grid lines
│   ├── render_points_mode() → points only
│   ├── render_lines_mode() → pipes only
│   ├── render_regular_lines_mode() → grid lines only
│   ├── render_polygons_mode() → polygons only
│   └── render_meshes_mode() → meshes only
└── Submit commands to GPU queue
```

### Web vs Native Split

#### **Platform Detection**
The code splits between web and native using Rust's conditional compilation:

```rust
#[cfg(target_arch = "wasm32")]  // Web (WASM) code
#[cfg(not(target_arch = "wasm32"))]  // Native code
```

#### **Key Differences**

| Component | Native | Web (WASM) |
|-----------|--------|------------|
| **GPU Backend** | `wgpu::Backends::PRIMARY` | `wgpu::Backends::BROWSER_WEBGPU` |
| **Device Limits** | `wgpu::Limits::default()` | `wgpu::Limits::downlevel_defaults().using_resolution(adapter.limits())` |
| **Hot Reload** | File system watching (notify crate) | HTTP polling for JSON changes |
| **Canvas Setup** | Native window | HTML5 canvas with dynamic sizing |
| **Asset Loading** | File system (`std::fs`) | HTTP requests (`reqwest`) |
| **Error Handling** | `eprintln!()` | `web_sys::console::error_1()` |
| **Event Loop** | Native winit event loop | Browser event loop integration |

#### **Web-Specific Features [lib_app.rs]**
```rust
#[cfg(target_arch = "wasm32")]
{
    // Set up HTML canvas
    let canvas = web_sys::window()
        .and_then(|win| win.document())
        .and_then(|doc| doc.get_element_by_id("wasm-example"))
        .and_then(|div| {
            let canvas = doc.create_element("canvas").ok()?;
            canvas.set_attribute("id", "wgpu-canvas").ok()?;
            div.append_child(&canvas).ok()?;
            Some(canvas)
        })
        .and_then(|canvas| canvas.dyn_into::<web_sys::HtmlCanvasElement>().ok())?;
    
    // Dynamic canvas resizing
    let resize_closure = Closure::wrap(Box::new(move || {
        // Resize canvas based on window size
    }) as Box<dyn FnMut()>);
}
```

#### **Hot Reload Implementation**

**Native [lib_hot_reload.rs]:**
```rust
#[cfg(not(target_arch = "wasm32"))]
use notify::{Watcher, RecursiveMode, watcher};
// File system watching for geometry file changes
```

**Web [lib_hot_reload.rs + hot_reload_complete.js]:**
```rust
#[cfg(target_arch = "wasm32")]
// HTTP polling for JSON file changes
// JavaScript integration for live updates
```

## Build & Run

### Native
```bash
cargo run
```

### Web
```bash
# Build WASM
wasm-pack build --target web --out-dir pkg

# Serve
python3 -m http.server 8002
# Open http://localhost:8002
```

## Controls

- **WASD/Arrow keys**: Move camera forward/backward/left/right
- **Space/Shift**: Move camera up/down
- **Mouse**: Look around (arcball camera)
- **Number keys (0-5)**: Switch render modes
  - 0: All geometry
  - 1: Points only
  - 2: Lines/Pipes only
  - 3: Regular lines only
  - 4: Meshes only
  - 5: Polygons only

## Browser Support

- ✅ **Chrome/Chromium**: Full WebGPU support
- ✅ **Edge**: Full WebGPU support
- ⏳ **Firefox**: Experimental WebGPU (enable `dom.webgpu.enabled` in `about:config`)
- ⏳ **Safari**: Limited WebGPU support

## Geometry Format

The viewer loads geometry from `assets/sample_geometry.json` with support for:
- Meshes (vertices, indices, materials)
- Point clouds
- Line segments
- 3D pipes (generated using OpenModel)
- Polygons sample_geometry.json e.g. cube with faces composed from 4 face vertices instead of 3.
- [ ] model_mesh.rs, shader files and lib.rs change to use the geometry from (check if it needs to be published first): https://github.com/petrasvestartas/openmodel/tree/main/src/geometry
- [ ] Optional: Mesh backfaces with different color.
- [ ] Optional: Mesh normals
- [ ] Optional: Mesh windings