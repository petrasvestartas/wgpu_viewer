# WebGPU Viewer

<img width="2560" height="1600" alt="Screenshot from 2025-07-11 21-09-34" src="https://github.com/user-attachments/assets/1c1e0911-6593-4672-9425-35f32167ab0e" />



## Project Structure

```
/wgpu_viewer/
├── src/             # Source code files for the WebGPU application
├── res/             # 3D model and texture resources
├── pkg/             # WebAssembly build output
├── target/          # Rust build output
├── build.rs         # Build script that copies resources to output directory
├── Cargo.toml       # Project configuration and dependencies
├── index.html       # Web page for WebAssembly version
└── README.md        # This documentation file
```

### Key Source Files

- **src/main.rs**: Entry point for native application
- **src/lib.rs**: Main WebGPU implementation and WASM entry point
- **src/camera.rs**: Camera implementation with position and rotation
- **src/model.rs**: 3D model loading and rendering
- **src/resources.rs**: Resource loading for both native and web targets
- **src/texture.rs**: Texture loading and management
- **src/shader.wgsl**: Main shader (WebGPU Shading Language)
- **src/light.wgsl**: Light shader implementation

## Native

```bash
# Build
cargo build

# Run
cargo run
```

## Web

```bash
# Build WASM
wasm-pack build --target web

# Serve and close server with Ctrl+C
python3 -m http.server 8002
# Open http://localhost:8002
ps aux | grep "[p]ython3 -m http.server"
fuser -k 8002/tcp
```

## Controls

- **WASD/Arrow keys**: Move camera forward/backward/left/right
- **Space/Shift**: Move camera up/down
- **Mouse drag**: Rotate camera
- **Mouse wheel**: Zoom in/out

## TODO

- [ ] Separate geometry crate: to implement a geometry class for:
- [ ] a) point
- [ ] b) vector
- [ ] c) poincloud
- [ ] d) line
- [ ] e) frame
- [ ] f) transformation
- [ ] g) half-edge mesh
- [ ] g_a) earclipping
- [ ] g_b) edge line extraction
- [ ] g_c) create a mesh representation from polygons (lists of points), where duplicate points have to be removed.
- [ ] Create a polygon sample_geometry.json e.g. cube with faces composed from 4 face vertices instead of 3.
- [ ] model_mesh.rs, shader files and lib.rs change to use the geometry from (check if it needs to be published first): https://github.com/petrasvestartas/openmodel/tree/main/src/geometry
- [ ] Optional: Mesh backfaces with different color.
- [ ] Optional: Mesh normals
- [ ] Optional: Mesh windings