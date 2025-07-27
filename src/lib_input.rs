use crate::{State, RenderMode};
use winit::{
    event::*,
    keyboard::{KeyCode, PhysicalKey},
};

/// Handle input events and update state accordingly
pub fn handle_input(state: &mut State, event: &WindowEvent) -> bool {
    match event {
        WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(key),
                    state: ElementState::Pressed,
                    ..
                },
            ..
        } => {
            // Handle number keys for render mode selection
            match key {
                KeyCode::Digit0 => {
                    state.render_mode = RenderMode::All;
                    println!("Render mode: All (0)");
                    true
                }
                KeyCode::Digit1 => {
                    state.render_mode = RenderMode::Points;
                    println!("Render mode: Points (1)");
                    true
                }
                KeyCode::Digit2 => {
                    state.render_mode = RenderMode::Lines;
                    println!("Render mode: Lines (2)");
                    // Force creation of pipe lines when switching to Lines mode
                    crate::lib_geometry_manager::create_pipes_from_lines(state);
                    true
                }
                KeyCode::Digit3 => {
                    state.render_mode = RenderMode::RegularLines;
                    println!("Render mode: Regular Lines (3)");
                    true
                }
                KeyCode::Digit4 => {
                    state.render_mode = RenderMode::Meshes;
                    println!("Render mode: Meshes (4)");
                    true
                }
                KeyCode::Digit5 => {
                    state.render_mode = RenderMode::Polygons;
                    println!("Render mode: Polygons (5)");
                    // Create sample polygon when switching to polygon mode
                    crate::lib_geometry_manager::create_sample_polygon(state);
                    true
                }
                // Point size is now hardcoded directly in the shader
                _ => state.camera_controller.process_keyboard(*key, ElementState::Pressed),
            }
        }
        WindowEvent::KeyboardInput {
            event:
                KeyEvent {
                    physical_key: PhysicalKey::Code(key),
                    state: key_state,
                    ..
                },
            ..
        } => state.camera_controller.process_keyboard(*key, *key_state),
        WindowEvent::MouseWheel { delta, .. } => {
            state.camera_controller.process_scroll(delta);
            true
        }
        WindowEvent::MouseInput {
            button,
            state: button_state,
            ..
        } => {
            // For arcball camera, pass all mouse buttons to the camera controller
            if state.camera_controller.process_mouse_button(*button_state, *button) {
                return true;
            }
            // Still maintain the mouse_pressed state for other functionality
            if *button == MouseButton::Left {
                state.mouse_pressed = *button_state == ElementState::Pressed;
                return true;
            }
            false
        }
        _ => false,
    }
}
