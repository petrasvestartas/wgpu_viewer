# WebGPU Camera Example

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
cd code/intermediate/tutorial12-camera
wasm-pack build --target web

# Serve
python3 -m http.server 8002
# Open http://localhost:8002
```

Controls: WASD/arrows (move), Space/Shift (up/down), Mouse drag (rotate), Mouse wheel (zoom)
