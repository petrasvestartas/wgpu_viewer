use crate::State;
use crate::geometry_loader;
use std::sync::mpsc;

#[cfg(not(target_arch = "wasm32"))]
use notify::EventKind;
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
type NotifyEvent = notify::Event;

// WASM hot reload communication
#[cfg(target_arch = "wasm32")]
use std::sync::{Arc, Mutex};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
static RELOAD_FLAG: std::sync::LazyLock<Arc<Mutex<bool>>> = std::sync::LazyLock::new(|| Arc::new(Mutex::new(false)));

#[cfg(target_arch = "wasm32")]
static RELOAD_DATA: std::sync::LazyLock<Arc<Mutex<Option<String>>>> = std::sync::LazyLock::new(|| Arc::new(Mutex::new(None)));

/// WASM-exposed function for triggering geometry hot reload
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn reload_geometry() {
    log::info!("Hot reload triggered from JavaScript - setting reload flag");
    
    // Set the reload flag so the main loop can pick it up
    if let Ok(mut flag) = RELOAD_FLAG.lock() {
        *flag = true;
        log::info!("Reload flag set - geometry will reload on next frame");
    } else {
        log::error!("Failed to set reload flag");
    }
}

/// Check and handle reload flag in the main loop (WASM)
#[cfg(target_arch = "wasm32")]
pub fn check_reload_flag(state: &mut State) {
    // Check if we have new geometry data to process
    if let Ok(mut data) = RELOAD_DATA.lock() {
        if let Some(json_string) = data.take() {
            log::info!("🔄 Processing fetched geometry data for in-place reload");
            
            // Parse and load the geometry data directly into State
            match process_geometry_reload(state, &json_string) {
                Ok(_) => {
                    log::info!("✅ Hot reload complete - geometry updated in-place! No page refresh needed!");
                    state.window().request_redraw();
                }
                Err(e) => {
                    log::error!("❌ Failed to process geometry reload: {}", e);
                }
            }
        }
    }
    
    // Check if we need to trigger a new fetch
    if let Ok(mut flag) = RELOAD_FLAG.lock() {
        if *flag {
            *flag = false; // Reset flag
            log::info!("Processing hot reload - fetching fresh geometry data");
            
            // Spawn async task to fetch geometry data
            wasm_bindgen_futures::spawn_local(async move {
                match fetch_and_reload_geometry().await {
                    Ok(_) => {
                        log::info!("📦 Fresh geometry data fetched and ready for processing");
                    }
                    Err(e) => {
                        log::error!("❌ Geometry fetch failed: {}", e);
                    }
                }
            });
        }
    }
}

/// Check for file changes and reload geometry if needed (native builds only)
#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
pub fn check_and_reload_geometry(state: &mut State, file_change_receiver: &mpsc::Receiver<notify::Result<NotifyEvent>>) {
    // Check for file change events without blocking
    while let Ok(event_result) = file_change_receiver.try_recv() {
        if let Ok(event) = event_result {
            match event.kind {
                EventKind::Modify(_) | EventKind::Create(_) => {
                    log::info!("JSON file changed, reloading geometry...");
                    // Reload geometry using pollster (already available in dependencies)
                    if let Err(e) = pollster::block_on(state.load_geometries_from_file("assets/sample_geometry.json")) {
                        log::error!("Failed to reload geometry: {}", e);
                    } else {
                        log::info!("Geometry reloaded successfully");
                    }
                }
                _ => {} // Ignore other events
            }
        }
    }
}

/// Process geometry reload by parsing JSON and updating State (WASM)
#[cfg(target_arch = "wasm32")]
fn process_geometry_reload(state: &mut State, json_string: &str) -> Result<(), String> {
    log::info!("🔍 Parsing {} bytes of geometry JSON", json_string.len());
    
    // Parse JSON into geometry data structures
    let geometry_data: geometry_loader::GeometryData = serde_json::from_str(json_string)
        .map_err(|e| format!("JSON parse error: {}", e))?;
    
    log::info!("🔄 Processing geometry data for hot reload");
    
    // Process mesh data if available
    if let Some(meshes) = &geometry_data.meshes {
        if !meshes.is_empty() {
            log::info!("🔹 Reloading {} meshes", meshes.len());
            
            // Create an empty texture bind group layout since we removed all texture dependencies
            let texture_bind_group_layout = 
                state.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    entries: &[], // No entries needed anymore as we removed textures
                    label: Some("texture_bind_group_layout"),
                });
            
            // Store all mesh models in a Vec
            let mut mesh_models = Vec::new();
            
            // Load all meshes from the JSON data
            for mesh_data in meshes {
                log::info!("Reloading mesh: {}", mesh_data.name);
                
                // Create the model from each mesh data
                let model = geometry_loader::create_model_from_mesh_data(
                    &state.device,
                    &state.queue,
                    mesh_data,
                    &texture_bind_group_layout
                ).map_err(|e| format!("Failed to create mesh model: {}", e))?;
                
                mesh_models.push(model);
            }
            
            // For backwards compatibility, set the first model as obj_model
            if !mesh_models.is_empty() {
                state.obj_model = mesh_models.remove(0);
            }
            
            // Store additional models
            state.additional_mesh_models = mesh_models;
        }
    }
    
    // Process point data if available
    if let Some(points) = &geometry_data.points {
        if !points.is_empty() {
            let first_point_set = &points[0];
            log::info!("🔵 Reloading point cloud: {}", first_point_set.name);
            
            let quad_point_model = geometry_loader::create_quad_point_model_from_point_data(
                &state.device,
                first_point_set
            );
            
            state.quad_point_model = Some(quad_point_model);
        }
    }
    
    // Process pipe data if available
    if let Some(pipes) = &geometry_data.pipes {
        if !pipes.is_empty() {
            let first_pipe_set = &pipes[0];
            log::info!("🔶 Reloading pipes: {}", first_pipe_set.name);
            
            let pipe_model = geometry_loader::create_pipe_model_from_pipe_data(
                &state.device,
                first_pipe_set
            );
            
            state.pipe_model = Some(pipe_model);
        }
    }
    
    // Process polygon data if available
    if let Some(polygons) = &geometry_data.polygons {
        if !polygons.is_empty() {
            let first_polygon_set = &polygons[0];
            log::info!("🔷 Reloading polygons: {}", first_polygon_set.name);
            
            let polygon_model = geometry_loader::create_polygon_model_from_polygon_data(
                &state.device,
                first_polygon_set
            );
            
            state.polygon_model = Some(polygon_model);
        }
    }
    
    log::info!("✅ Hot reload complete - all geometry updated in-place!");
    
    Ok(())
}

/// Fetch geometry JSON from server and reload it (WASM)
#[cfg(target_arch = "wasm32")]
async fn fetch_and_reload_geometry() -> Result<(), String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::{Request, RequestInit, Response};
    
    log::info!("🔄 Fetching fresh geometry data from server...");
    
    // Create request to fetch the geometry JSON with cache busting
    let opts = RequestInit::new();
    opts.set_method("GET");
    
    // Add timestamp to URL for cache busting
    let timestamp = js_sys::Date::now() as u64;
    let url = format!("assets/sample_geometry.json?t={}", timestamp);
    
    let request = Request::new_with_str_and_init(&url, &opts)
        .map_err(|e| format!("Failed to create request: {:?}", e))?;
    
    // Add cache-busting headers
    request.headers().set("Cache-Control", "no-cache, no-store, must-revalidate")
        .map_err(|e| format!("Failed to set header: {:?}", e))?;
    request.headers().set("Pragma", "no-cache")
        .map_err(|e| format!("Failed to set pragma header: {:?}", e))?;
    
    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await
        .map_err(|e| format!("Fetch failed: {:?}", e))?;
    let resp: Response = resp_value.dyn_into().unwrap();
    
    if !resp.ok() {
        return Err(format!("Failed to fetch geometry: HTTP {}", resp.status()));
    }
    
    let json_value = JsFuture::from(resp.text()
        .map_err(|e| format!("Failed to get response text: {:?}", e))?).await
        .map_err(|e| format!("Failed to read response: {:?}", e))?;
    let json_string = json_value.as_string().unwrap();
    
    log::info!("📄 Received {} bytes of geometry data", json_string.len());
    
    // Parse the JSON to validate it
    let _parsed: serde_json::Value = serde_json::from_str(&json_string)
        .map_err(|e| format!("JSON parse error: {}", e))?;
    
    log::info!("✅ JSON validation successful - geometry data is valid");
    
    // Store the fetched geometry data for the main thread to process
    if let Ok(mut data) = RELOAD_DATA.lock() {
        *data = Some(json_string);
        log::info!("📦 Geometry data stored for main thread processing");
    } else {
        return Err("Failed to store geometry data".to_string());
    }
    
    Ok(())
}
