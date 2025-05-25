# WGPU Viewer

This project demonstrates a simple triangle renderer using WGPU. It can be run both natively and in a web browser via WebAssembly.

## Project Structure

- `src/main.rs`: Main entry point for the native application
- `src/challenge.rs`: Challenge version with additional features
- `src/lib.rs`: Core rendering functionality used by both binaries
- `src/shader.wgsl`: WGSL shader used for WebAssembly builds
- `src/shader.frag`, `src/shader.vert`: GLSL shaders used for native builds

## Running Natively

To run the main pipeline version:
```bash
cargo run --bin wgpu_viewer
```

To run the challenge version:
```bash
cargo run --bin tutorial3-challenge
```

## Running in the Browser (WebAssembly)

### 1. Build for WebAssembly

```bash
# Add the WebAssembly target (if not already added)
rustup target add wasm32-unknown-unknown

# Build the project targeting WebAssembly
cargo build --bin wgpu_viewer --target wasm32-unknown-unknown --release
```

### 2. Generate JavaScript Bindings

```bash
# Install wasm-bindgen-cli if not already installed
cargo install wasm-bindgen-cli

# Generate JavaScript bindings
wasm-bindgen --out-dir web --target web target/wasm32-unknown-unknown/release/wgpu_viewer.wasm
```

### 3. Start a Web Server

```bash
cd web
python3 -m http.server 8080
```

### 4. View in Browser

Open a web browser and navigate to http://localhost:8080

## Modifying the Project

If you modify shader files or other code, you'll need to rebuild:

1. For native builds:
   ```bash
   cargo build
   ```

2. For WebAssembly builds:
   ```bash
   cargo build --bin wgpu_viewer --target wasm32-unknown-unknown --release
   wasm-bindgen --out-dir web --target web target/wasm32-unknown-unknown/release/wgpu_viewer.wasm
   ```

## Troubleshooting

- If the triangle doesn't appear in the browser, check the console (F12) for errors
- Make sure your browser supports WebGL or WebGPU
- For the WebAssembly version, the project uses WebGL as a backend
