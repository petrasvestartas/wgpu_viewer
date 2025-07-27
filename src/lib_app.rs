use crate::State;

#[cfg(target_arch = "wasm32")]
use crate::lib_hot_reload::check_reload_flag;
use winit::{
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

/// Main application runner that handles the event loop and window management
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

    // Create the initial state
    let mut state = match State::new(&window).await {
        Ok(state) => state,
        Err(e) => {
            #[cfg(target_arch = "wasm32")]
            {
                web_sys::console::error_1(&format!("Failed to initialize WebGPU: {}. This browser may not support WebGPU yet. Please try Chrome/Chromium for the best WebGPU experience.", e).into());
                panic!("WebGPU initialization failed");
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                panic!("Failed to initialize GPU: {}", e);
            }
        }
    };
    
    // Load geometries from the JSON file
    if let Err(err) = state.load_geometries_from_file("assets/sample_geometry.json").await {
        log::error!("Failed to load geometries from file: {}", err);
    } else {
        log::info!("Successfully loaded geometries from file");
    }
    
    // Only grid lines and JSON-loaded geometry should be displayed
    // Sample hardcoded geometry creation removed as per user request
    
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
                        
                        // Check for hot reload flag (WASM only)
                        #[cfg(target_arch = "wasm32")]
                        check_reload_flag(&mut state);
                        
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
