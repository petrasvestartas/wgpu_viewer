# 🔧 wgpu_viewer Code Flow & Refactoring Plan

## 🎯 **Understanding How wgpu_viewer Works**

This document explains the **step-by-step execution flow** of the wgpu_viewer application and provides a refactoring plan to improve its architecture.

---

## 🚀 **Step-by-Step Code Execution Flow**

### **Phase 1: Application Startup** 📱

#### **1.1 Entry Point - `src/lib.rs`**
```rust
// Entry point for WASM
#[wasm_bindgen]
pub async fn run() {
    lib_app::run().await;
}
```

**What happens:**
1. **WASM Module Loads**: Browser loads `wgpu_viewer_bg.wasm` and `wgpu_viewer.js`
2. **JavaScript Calls**: `index.html` calls `init()` then `run()`
3. **Rust Code Starts**: `lib_app::run()` begins execution

#### **1.2 Application Runner - `src/lib_app.rs`**
```rust
pub async fn run() {
    // 1. Set up logging and panic hooks
    // 2. Create event loop
    // 3. Create window
    // 4. Initialize State
    // 5. Start event loop
}
```

**Step-by-step execution:**
1. **Logging Setup**: Configure console logging for WASM, env_logger for native
2. **Event Loop Creation**: `EventLoop::new()` - handles all window events
3. **Window Creation**: `WindowBuilder::new()` - creates the display window
4. **WASM Canvas Setup**: For web, attach canvas to DOM and configure CSS
5. **State Initialization**: `State::new(&window).await` - the core setup

---

### **Phase 2: State Initialization** 🏗️

#### **2.1 State Creation - `src/lib_state.rs`**
```rust
pub async fn new(window: &'a Window) -> Result<State<'a>, Box<dyn std::error::Error>> {
    // 1. Initialize GPU context
    // 2. Set up camera system
    // 3. Create rendering pipelines
    // 4. Load initial geometry
}
```

**Detailed GPU Setup Process:**
1. **GPU Instance Creation**: `wgpu::Instance::new()` - connects to GPU driver
2. **Surface Creation**: `instance.create_surface(window)` - creates drawing surface
3. **Adapter Selection**: `instance.request_adapter()` - chooses GPU (discrete/integrated)
4. **Device Creation**: `adapter.request_device()` - creates GPU command queue
5. **Surface Configuration**: Set up pixel format, size, and presentation mode

#### **2.2 Camera System Setup**
```rust
fn init_camera_system(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
    // 1. Create camera with position and target
    // 2. Create projection matrix
    // 3. Create camera controller
    // 4. Create uniform buffer for camera data
    // 5. Create bind group for GPU access
}
```

**Camera Components:**
- **Camera**: Position, target, up vector
- **Projection**: Field of view, aspect ratio, near/far planes
- **Controller**: Handles mouse/keyboard input for camera movement
- **Uniform Buffer**: GPU buffer containing camera matrices
- **Bind Group**: GPU binding for camera data access

#### **2.3 Rendering Pipeline Creation**
```rust
async fn init_pipelines(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) {
    // 1. Create main mesh pipeline
    // 2. Create point rendering pipeline
    // 3. Create line rendering pipeline
    // 4. Create pipe rendering pipeline
    // 5. Create polygon rendering pipeline
    // 6. Create lighting pipeline
}
```

**Pipeline Components:**
- **Vertex Shader**: Processes vertex positions and attributes
- **Fragment Shader**: Processes pixel colors and lighting
- **Vertex Buffer Layout**: Defines vertex data structure
- **Bind Group Layouts**: Define GPU resource bindings
- **Render Pipeline**: Combines shaders and configuration

---

### **Phase 3: Geometry Loading** 📐

#### **3.1 Geometry Loading - `src/geometry_loader.rs`**
```rust
pub async fn load_geometries_from_file(&mut self, file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Read JSON file
    // 2. Parse geometry data
    // 3. Create different model types
    // 4. Store in State
}
```

**Geometry Processing Flow:**
1. **File Reading**: `std::fs::read_to_string()` - reads JSON file
2. **JSON Parsing**: `serde_json::from_str()` - converts JSON to Rust structs
3. **Model Creation**: Creates different geometry types:
   - **Mesh Models**: 3D objects with vertices and faces
   - **Point Models**: Individual points in 3D space
   - **Line Models**: Lines connecting points
   - **Pipe Models**: Cylindrical pipes (using OpenModel)
   - **Polygon Models**: 2D polygons in 3D space

#### **3.2 OpenModel Integration**
```rust
// In src/model_pipe.rs
pub fn from_openmodel_line(line: &openmodel::Line, radius: f64) -> Self {
    let mesh = openmodel::Mesh::create_pipe(line.start, line.end, radius);
    // Convert OpenModel mesh to wgpu buffers
}
```

**OpenModel Process:**
1. **Line Input**: Start and end points from JSON
2. **Pipe Creation**: `Mesh::create_pipe()` - generates cylinder geometry
3. **Mesh Conversion**: Convert to wgpu vertex/index buffers
4. **Rendering Setup**: Prepare for GPU rendering

---

### **Phase 4: Event Loop & Input Handling** ⌨️

#### **4.1 Main Event Loop - `src/lib_app.rs`**
```rust
event_loop.run(move |event, control_flow| {
    match event {
        Event::WindowEvent { event, window_id } => {
            // Handle window events (resize, close, etc.)
        }
        Event::DeviceEvent { event, .. } => {
            // Handle input events (mouse, keyboard)
        }
        Event::RedrawRequested => {
            // Render frame
        }
    }
});
```

**Event Types:**
- **Window Events**: Resize, close, focus, etc.
- **Device Events**: Mouse movement, keyboard input
- **Redraw Events**: Triggered when screen needs updating

#### **4.2 Input Processing - `src/lib_state.rs`**
```rust
pub fn input(&mut self, event: &WindowEvent) -> bool {
    match event {
        WindowEvent::KeyboardInput { event, .. } => {
            // Handle keyboard input
        }
        WindowEvent::MouseInput { button, state, .. } => {
            // Handle mouse clicks
        }
        WindowEvent::CursorMoved { position, .. } => {
            // Handle mouse movement
        }
    }
}
```

**Input Flow:**
1. **Event Capture**: Window system captures user input
2. **Event Dispatch**: Event loop sends events to State
3. **Camera Control**: Camera controller processes mouse/keyboard
4. **Render Mode Switching**: Keyboard shortcuts change display modes
5. **State Update**: Camera position, rotation updated

---

### **Phase 5: Rendering Pipeline** 🎨

#### **5.1 Render Method - `src/lib_state.rs`**
```rust
pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
    // 1. Get next frame
    // 2. Create command encoder
    // 3. Create render pass
    // 4. Set pipeline
    // 5. Draw geometry
    // 6. Submit commands
}
```

**Detailed Rendering Process:**

**Step 1: Frame Acquisition**
```rust
let frame = self.surface.get_current_texture()?;
let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
```
- **Surface**: Gets the next frame buffer from GPU
- **Texture View**: Creates a view for rendering into

**Step 2: Command Encoder**
```rust
let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
    label: Some("Render Encoder"),
});
```
- **Command Encoder**: Records GPU commands for execution
- **GPU Commands**: Draw calls, buffer updates, pipeline switches

**Step 3: Render Pass**
```rust
let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
        view: &view,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
            store: true,
        },
    })],
    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
        view: &self.depth_texture_view,
        depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(1.0),
            store: true,
        }),
        stencil_ops: None,
    }),
});
```
- **Color Attachment**: Where pixels are drawn
- **Depth Attachment**: Handles 3D depth testing
- **Clear Operations**: Clear screen and depth buffer

**Step 4: Pipeline Selection & Drawing**
```rust
// Draw different geometry types based on render mode
match self.render_mode {
    RenderMode::Mesh => {
        render_pass.set_pipeline(&self.render_pipeline);
        render_pass.draw_model_instanced(&self.obj_model, 0..self.instances.len() as u32, &self.camera_bind_group, &self.light_bind_group);
    }
    RenderMode::Points => {
        render_pass.set_pipeline(&self.point_pipeline.as_ref().unwrap());
        render_pass.draw_point_model(&self.point_model.as_ref().unwrap(), &self.camera_bind_group);
    }
    // ... other modes
}
```

**Drawing Process:**
1. **Pipeline Binding**: Select appropriate shader pipeline
2. **Bind Group Binding**: Bind camera and lighting data
3. **Vertex Buffer Binding**: Bind geometry data
4. **Index Buffer Binding**: Bind triangle indices
5. **Draw Call**: Issue GPU draw command

**Step 5: Command Submission**
```rust
self.queue.submit(std::iter::once(encoder.finish()));
frame.present();
```
- **Command Submission**: Send commands to GPU for execution
- **Frame Presentation**: Display the rendered frame

---

### **Phase 6: Hot Reload System** 🔄

#### **6.1 WASM Hot Reload - `src/lib_hot_reload.rs`**
```rust
#[cfg(target_arch = "wasm32")]
pub fn check_reload_flag(state: &mut State) {
    // 1. Check if reload flag is set
    // 2. Fetch new geometry data
    // 3. Parse JSON and update models
    // 4. Clear reload flag
}
```

**WASM Hot Reload Flow:**
1. **JavaScript Polling**: `hot_reload_complete.js` polls for file changes
2. **Flag Setting**: JavaScript sets reload flag in WASM memory
3. **Flag Detection**: Rust checks flag on each frame
4. **Data Fetching**: `fetch_and_reload_geometry()` gets new JSON
5. **Model Update**: Parse JSON and recreate geometry models
6. **Visual Update**: New geometry appears immediately

#### **6.2 Native Hot Reload**
```rust
#[cfg(not(target_arch = "wasm32"))]
pub fn check_reload_flag(state: &mut State) {
    // 1. Check file modification time
    // 2. If changed, reload geometry
    // 3. Update models
}
```

**Native Hot Reload Flow:**
1. **File Watching**: `notify` crate monitors file system
2. **Change Detection**: Compare file modification times
3. **File Reading**: Read updated JSON file
4. **Model Recreation**: Parse and create new geometry
5. **Immediate Update**: New geometry renders next frame

---

## 🏗️ **Current Architecture Issues**

### **Problems with Monolithic Structure:**

1. **Single Responsibility Violation**
   - `lib_state.rs` handles rendering, geometry, input, and hot reload
   - 2000+ lines in one file makes navigation difficult

2. **Tight Coupling**
   - All systems depend on the State struct
   - Changes in one area affect multiple systems

3. **Testing Difficulty**
   - Cannot test individual components in isolation
   - Requires full application setup for any test

4. **Maintenance Burden**
   - Understanding changes requires reading entire file
   - Multiple developers cannot work on different features simultaneously

---

## 🔧 **Proposed Refactoring Plan**

### **Phase 1: Hot Reload Extraction** 🔄
**Target**: `src/hot_reload.rs`
**Rationale**: Least coupled, easiest to isolate
**Scope**: All WASM and native hot reload functionality

### **Phase 2: Input Handling Extraction** ⌨️
**Target**: `src/input.rs`
**Rationale**: Well-defined interface with State
**Scope**: Event processing and camera control

### **Phase 3: Geometry Management Extraction** 📐
**Target**: `src/geometry_manager.rs`
**Rationale**: Self-contained geometry operations
**Scope**: All geometry loading and creation methods

### **Phase 4: Application Runner Extraction** 🚀
**Target**: `src/app.rs`
**Rationale**: High-level orchestration with clear boundaries
**Scope**: Main event loop and window management

### **Phase 5: Rendering Engine Extraction** 🎨
**Target**: `src/render.rs`
**Rationale**: Most complex but State will be smaller by then
**Scope**: All rendering logic and pipeline management

### **Phase 6: State Finalization** 🏗️
**Target**: `src/state.rs`
**Rationale**: Clean up remaining State struct
**Scope**: Core state management and initialization

---

## 📊 **Expected Benefits**

### **Maintainability**
- **Single Responsibility**: Each module has one clear purpose
- **Smaller Files**: Easier to understand and navigate
- **Clear Boundaries**: Reduced cognitive load when making changes

### **Testability**
- **Unit Testing**: Individual modules can be tested in isolation
- **Mock Dependencies**: Easier to create test doubles
- **Focused Tests**: Test specific functionality without setup overhead

### **Developer Experience**
- **Faster Navigation**: Jump to relevant code quickly
- **Reduced Merge Conflicts**: Multiple developers can work on different modules
- **Clearer Git History**: Changes are scoped to relevant modules

---

## 📋 **Implementation Timeline**

**Estimated Duration**: 2-3 days
- **Phase 1-2**: 4-6 hours (Hot reload + Input)
- **Phase 3-4**: 6-8 hours (Geometry + App runner)
- **Phase 5-6**: 8-10 hours (Rendering + State cleanup)
- **Testing & Documentation**: 2-4 hours

---

**Status**: 📋 **Planning Phase**  
**Next Step**: Begin Phase 1 (Hot Reload Extraction)  
**Last Updated**: 2025-07-28
