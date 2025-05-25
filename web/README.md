# WGPU WebAssembly Demo - Build Instructions

This document provides instructions on how to rebuild and run the WGPU application in a web browser.

## Prerequisites

Make sure you have the following installed:
- Rust and Cargo
- wasm-bindgen-cli
- A local web server (Python's http.server or similar)

## Rebuilding After Code Changes

1. **Build the project targeting WebAssembly**:
   ```bash
   cargo build --bin wgpu_viewer --target wasm32-unknown-unknown --release
   ```

2. **Generate JavaScript bindings**:
   ```bash
   wasm-bindgen --out-dir web --target web target/wasm32-unknown-unknown/release/wgpu_viewer.wasm
   ```

3. **Start a local web server**:
   ```bash
   cd web
   python3 -m http.server 8080
   ```

4. **View in browser**:
   Open a web browser and navigate to http://localhost:8080

## Troubleshooting

- If the triangle doesn't appear, check the browser console (F12) for errors
- Make sure your browser supports WebGL or WebGPU
- If you make changes to shader files (.wgsl, .frag, .vert), you need to rebuild

## Notes About Shader Files

- This project uses both WGSL shaders and GLSL shaders:
  - `shader.wgsl`: Used for WebAssembly/browser builds
  - `shader.frag`/`shader.vert`: Used for native builds

To change the appearance of the triangle, modify the appropriate shader file based on your target platform.
