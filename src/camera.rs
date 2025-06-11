use cgmath::*;
use std::f32::consts::FRAC_PI_2;
use std::time::Duration;
use winit::dpi::PhysicalPosition;
use winit::event::*;
use winit::keyboard::KeyCode;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

// New arcball camera implementation
#[derive(Debug)]
pub struct Camera {
    // Eye position in 3D space
    pub position: Point3<f32>,
    // Center/target point that the camera looks at
    pub target: Point3<f32>,
    // Up direction, typically (0, 1, 0)
    pub up: Vector3<f32>,
    // Distance from target (used for zoom)
    pub distance: f32,
    // Horizontal rotation angle (in radians)
    pub yaw: Rad<f32>,
    // Vertical rotation angle (in radians)
    pub pitch: Rad<f32>,
}

impl Camera {
    pub fn new<V: Into<Point3<f32>>>(position: V, target: Point3<f32>) -> Self {
        let position = position.into();
        let offset = position - target;
        let distance = offset.magnitude();
        
        // Calculate initial yaw and pitch from position
        let yaw = Rad(offset.x.atan2(offset.z));
        let pitch = Rad((offset.y / distance).asin().min(SAFE_FRAC_PI_2).max(-SAFE_FRAC_PI_2));

        
        Self {
            position,
            target,
            up: Vector3::unit_y(),
            distance,
            yaw,
            pitch,
        }
    }

    // Update the camera position based on yaw, pitch, and distance
    pub fn update_position(&mut self) {
        // Convert spherical coordinates to cartesian
        let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();
        
        self.position = Point3::new(
            self.target.x + self.distance * cos_pitch * sin_yaw,
            self.target.y + self.distance * sin_pitch,
            self.target.z + self.distance * cos_pitch * cos_yaw,
        );
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        Matrix4::look_at_rh(self.position, self.target, self.up)
    }
    
    // Pan the camera by moving both position and target
    pub fn pan(&mut self, right_amount: f32, up_amount: f32) {
        // Calculate right and up vectors in world space
        let forward = (self.target - self.position).normalize();
        let right = forward.cross(self.up).normalize();
        let up = right.cross(forward).normalize();
        
        // Scale by distance for more intuitive panning
        let pan_speed = self.distance * 0.01;
        let pan_right = right * right_amount * pan_speed;
        let pan_up = up * up_amount * pan_speed;
        
        // Apply panning to both position and target to maintain orientation
        self.position = self.position + pan_right + pan_up;
        self.target = self.target + pan_right + pan_up;
    }
}

pub struct Projection {
    aspect: f32,
    fovy: Rad<f32>,
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new<F: Into<Rad<f32>>>(width: u32, height: u32, fovy: F, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

#[derive(Debug)]
pub struct CameraController {
    // Panning
    amount_left: f32,
    amount_right: f32,
    amount_up: f32,
    amount_down: f32,
    
    // Mouse panning
    mouse_pan_x: f32,
    mouse_pan_y: f32,
    is_panning: bool,      // Track if user is currently panning (middle button pressed)
    
    // Mouse drag rotation
    rotate_horizontal: f32,
    rotate_vertical: f32,
    is_rotating: bool,     // Track if user is currently rotating (right button pressed)
    
    // Zoom
    scroll: f32,
    
    // Control parameters
    speed: f32,            // General movement speed
    sensitivity: f32,      // Mouse sensitivity
    zoom_speed: f32,       // Zoom speed factor
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            mouse_pan_x: 0.0,
            mouse_pan_y: 0.0,
            is_panning: false,
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            is_rotating: false,
            scroll: 0.0,
            speed,
            sensitivity,
            zoom_speed: 0.05, // Reduced for softer zoom
        }
    }

    pub fn process_keyboard(&mut self, key: KeyCode, state: ElementState) -> bool {
        let amount = if state == ElementState::Pressed { 1.0 } else { 0.0 };
        match key {
            // Pan left/right/up/down with arrows or WASD
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.amount_up = amount;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.amount_down = amount;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.amount_left = amount;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.amount_right = amount;
                true
            }
            _ => false,
        }
    }
    
    // Process mouse movement for both rotation and panning based on which mouse button is pressed
    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        if self.is_rotating {
            // Right-click drag rotates the camera
            self.rotate_horizontal = mouse_dx as f32;
            self.rotate_vertical = mouse_dy as f32;
        }
        
        if self.is_panning {
            // Middle-click drag pans the camera
            self.mouse_pan_x = mouse_dx as f32;
            self.mouse_pan_y = mouse_dy as f32;
        }
    }
    
    // Process mouse button presses
    pub fn process_mouse_button(&mut self, state: ElementState, button: MouseButton) -> bool {
        match button {
            // Right mouse button controls rotation
            MouseButton::Right => {
                self.is_rotating = state == ElementState::Pressed;
                if !self.is_rotating {
                    // Reset rotation values when released
                    self.rotate_horizontal = 0.0;
                    self.rotate_vertical = 0.0;
                }
                return true;
            },
            // Middle mouse button controls panning
            MouseButton::Middle => {
                self.is_panning = state == ElementState::Pressed;
                if !self.is_panning {
                    // Reset pan values when released
                    self.mouse_pan_x = 0.0;
                    self.mouse_pan_y = 0.0;
                }
                return true;
            },
            _ => false,
        }
    }

    // Process scroll wheel for zoom
    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = match delta {
            // Reduce scroll multiplier for softer zoom
            MouseScrollDelta::LineDelta(_, scroll) => -*scroll * 1.0,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => -*scroll as f32 * 0.005,
        };
    }

    // Update the arcball camera
    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
        let dt = dt.as_secs_f32();
        
        // Handle keyboard panning (WASD/arrow keys)
        let key_pan_right = (self.amount_right - self.amount_left) * self.speed * dt;
        let key_pan_up = (self.amount_up - self.amount_down) * self.speed * dt;
        if key_pan_right != 0.0 || key_pan_up != 0.0 {
            camera.pan(key_pan_right, key_pan_up);
        }
        
        // Handle mouse panning (middle button drag)
        if self.is_panning && (self.mouse_pan_x != 0.0 || self.mouse_pan_y != 0.0) {
            // Apply pan with a sensitivity factor - increased by 10x
            let mouse_pan_speed = self.speed * self.sensitivity * 0.1;
            
            // Invert both X and Y to get the correct panning direction
            // When moving mouse right, the scene should move right
            let mouse_pan_right = -self.mouse_pan_x * mouse_pan_speed;
            let mouse_pan_up = self.mouse_pan_y * mouse_pan_speed;
            
            camera.pan(mouse_pan_right, mouse_pan_up);
            
            // Don't reset pan values as they should continue while middle button is held
            // They'll be reset when the button is released
        }
        
        // Handle rotation from mouse drag (right button)
        if self.is_rotating && (self.rotate_horizontal != 0.0 || self.rotate_vertical != 0.0) {
            camera.yaw += Rad(self.rotate_horizontal * self.sensitivity * dt);
            camera.pitch += Rad(-self.rotate_vertical * self.sensitivity * dt);
            
            // Keep pitch within safe limits to prevent gimbal lock
            if camera.pitch < -Rad(SAFE_FRAC_PI_2) {
                camera.pitch = -Rad(SAFE_FRAC_PI_2);
            } else if camera.pitch > Rad(SAFE_FRAC_PI_2) {
                camera.pitch = Rad(SAFE_FRAC_PI_2);
            }
            
            // Update camera position after rotation
            camera.update_position();
            
            // Don't reset rotation values as they should continue while right button is held
            // They'll be reset when the button is released
        }
        
        // Handle zooming with scroll wheel
        if self.scroll != 0.0 {
            // Adjust distance with scroll (zoom in/out) with softer effect
            camera.distance *= 1.0 + self.scroll * self.zoom_speed;
            
            // Ensure camera doesn't get too close or too far
            camera.distance = camera.distance.max(0.5).min(100.0);
            
            // Reset scroll and update position
            self.scroll = 0.0;
            camera.update_position();
        }
    }
}
