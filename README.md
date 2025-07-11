# WebGPU Viewer

![Screenshot from 2025-06-01 19-20-28](https://github.com/user-attachments/assets/5c328e87-3e67-4330-a69c-7c460a8cb2c8)


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

- [ ] Update the draw method when json file is changed.

