use cgmath::*;
use std::f32::consts::{FRAC_PI_2, PI};
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

const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.1;

// Default camera settings
const DEFAULT_YAW: f32 = 0.0;
const DEFAULT_PITCH: f32 = 0.0;
const DEFAULT_ROLL: f32 = 0.0;
const DEFAULT_DISTANCE: f32 = 10.0;
const DEFAULT_SENSITIVITY: f32 = 1.0;
const DEFAULT_SPEED: f32 = 2.0;
const MIN_ZOOM_DISTANCE: f32 = 0.5;
const MAX_ZOOM_DISTANCE: f32 = 100.0;
const TURNTABLE_MODE: bool = true; // Keep world up direction by default

// Professional 3D orbit camera implementation
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
    // Horizontal rotation angle around world Y-axis (in radians)
    pub yaw: Rad<f32>,
    // Vertical rotation angle (in radians)
    pub pitch: Rad<f32>,
    // Whether to maintain world up vector (turntable/orbit mode) or allow free rotation
    pub turntable_mode: bool,
    // The world up direction (typically Z in 3D modeling software)
    pub world_up: Vector3<f32>,
}

impl Camera {
    pub fn new<V: Into<Point3<f32>>>(position: V, target: Point3<f32>) -> Self {
        let position = position.into();
        let offset = position - target;
        let distance = offset.magnitude();
        
        // In most 3D modeling software, Z is up
        let world_up = Vector3::unit_z();
        
        // Calculate initial yaw (around Z) and pitch in a Z-up world
        // In Z-up world, the default view is looking down the Y axis
        let xz_dist = (offset.x * offset.x + offset.y * offset.y).sqrt();
        let yaw = Rad(offset.y.atan2(offset.x));
        let pitch = Rad((-offset.z / distance).asin().min(SAFE_FRAC_PI_2).max(-SAFE_FRAC_PI_2));
        
        let mut cam = Self {
            position,
            target,
            up: world_up,
            distance,
            yaw,
            pitch,
            turntable_mode: TURNTABLE_MODE,
            world_up,
        };
        
        cam.update_position();
        cam
    }

    // Update the camera position based on yaw, pitch, and distance - orbit style
    pub fn update_position(&mut self) {
        if self.turntable_mode {
            // In 3D modeling software with Z-up, standard turntable camera:
            // Yaw rotates around the z-axis (world up)
            // Pitch rotates around the local x-axis
            
            let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();
            let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
            
            // First calculate position as if looking down Z axis
            // Start with the camera at (0,distance,0) looking at origin
            let offset = Vector3::new(
                0.0,
                0.0,
                self.distance,
            );
            
            // Apply pitch rotation around X axis
            let pitch_rotation = Matrix3::from_axis_angle(Vector3::unit_x(), self.pitch);
            let pitched_offset = pitch_rotation * offset;
            
            // Apply yaw rotation around Z axis (world up in 3D modeling)
            let yaw_rotation = Matrix3::from_axis_angle(self.world_up, self.yaw);
            let final_offset = yaw_rotation * pitched_offset;
            
            // Apply to target to get final position
            self.position = self.target + final_offset;
            
            // Compute view up vector to maintain proper orientation
            // In turntable mode, the up vector always stays aligned with world up
            // but we need to handle the case when looking directly down the up axis
            let view_dir = (self.target - self.position).normalize();
            let dot = self.world_up.dot(view_dir);
            
            // If we're looking nearly parallel to the up vector, adjust the up vector
            if dot.abs() > 0.99 {
                // Use the yaw to create a stable up vector
                self.up = yaw_rotation * Vector3::unit_y();
            } else {
                self.up = self.world_up;
            }
        } else {
            // Free orbit mode - classic FPS camera
            let pitch_rotation = Matrix3::from_axis_angle(Vector3::unit_x(), self.pitch);
            let yaw_rotation = Matrix3::from_axis_angle(Vector3::unit_y(), self.yaw);
            let forward = yaw_rotation * pitch_rotation * -Vector3::unit_z();
            self.position = self.target + (forward * self.distance);
        }
    }

    pub fn calc_matrix(&self) -> Matrix4<f32> {
        // In professional 3D software, the camera view matrix is simply
        // looking from the position to the target with a consistent up vector
        Matrix4::look_at_rh(self.position, self.target, self.up)
    }
    
    // Pan camera in view plane (right and up vectors)
    pub fn pan(&mut self, right_amount: f32, up_amount: f32) {
        // For Z-up coordinate system (3D modeling software style)
        // Calculate view-aligned right and up vectors for panning
        let forward = (self.target - self.position).normalize();
        
        // In Z-up world, the right vector is perpendicular to forward and world_up
        let right = forward.cross(self.world_up).normalize();
        
        // The true up vector follows the orbit-style in Z-up world
        // This ensures panning is always aligned with view orientation
        let up = right.cross(forward).normalize();
        
        // Scale pan amount based on distance (pan faster when zoomed out)
        let pan_speed = self.distance * 0.01;
        
        // Compute pan offsets
        let pan_right = right * right_amount * pan_speed;
        let pan_up = up * up_amount * pan_speed;
        
        // Apply panning to both position and target to maintain relative position
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
    // Keyboard panning
    amount_left: f32,
    amount_right: f32,
    amount_up: f32,
    amount_down: f32,
    
    // Mouse panning
    mouse_pan_x: f32,
    mouse_pan_y: f32,
    is_panning: bool,      // Track if user is currently panning (middle button pressed)
    
    // Mouse orbital rotation
    mouse_delta_x: f32,
    mouse_delta_y: f32,
    is_orbiting: bool,     // Track if user is currently orbiting (right button pressed)
    
    // Orbit mode control
    alt_pressed: bool,     // Common in 3D software: Alt key for orbit mode
    
    // Zoom
    scroll: f32,
    
    // Control parameters
    speed: f32,            // General movement speed
    sensitivity: f32,      // Mouse sensitivity
    orbit_speed: f32,      // Speed multiplier for orbit rotation
    zoom_speed: f32,       // Zoom speed factor
    orbit_invert_y: bool,  // Whether to invert Y axis for orbiting (common option in 3D software)
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
            mouse_delta_x: 0.0,
            mouse_delta_y: 0.0,
            is_orbiting: false,
            alt_pressed: false,
            scroll: 0.0,
            speed,
            sensitivity,
            orbit_speed: 1.5,    // Increased orbit speed for responsive control
            zoom_speed: 0.05,    // Reduced for softer zoom
            orbit_invert_y: false, // Standard behavior in most 3D software
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
            // Alt key for orbit mode (common in 3D software)
            KeyCode::AltLeft | KeyCode::AltRight => {
                self.alt_pressed = state == ElementState::Pressed;
                true
            }
            _ => false,
        }
    }
    
    // Process mouse movement for orbit and panning based on which mouse button is pressed
    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        if self.is_orbiting {
            // Standard 3D modeling software orbit behavior with right mouse button
            self.mouse_delta_x = mouse_dx as f32;
            // Apply Y inversion if enabled (common option in 3D software)
            self.mouse_delta_y = if self.orbit_invert_y {
                -mouse_dy as f32
            } else {
                mouse_dy as f32
            };
        }
        
        if self.is_panning {
            // Middle-click drag pans the camera (standard in 3D modeling software)
            self.mouse_pan_x = mouse_dx as f32;
            self.mouse_pan_y = mouse_dy as f32;
        }
    }
    
    // Process mouse button presses
    pub fn process_mouse_button(&mut self, state: ElementState, button: MouseButton) -> bool {
        match button {
            // Right mouse button controls orbit rotation (standard in 3D modeling software)
            MouseButton::Right => {
                self.is_orbiting = state == ElementState::Pressed;
                if !self.is_orbiting {
                    // Reset orbit values when released
                    self.mouse_delta_x = 0.0;
                    self.mouse_delta_y = 0.0;
                }
                return true;
            },
            // Middle mouse button controls panning (standard in 3D modeling software)
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

    // Update the professional orbit camera - Z-up turntable style (Blender/Maya)
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
            // Apply pan with a sensitivity factor
            let mouse_pan_speed = self.speed * self.sensitivity * 0.1;
            
            // In Z-up world, panning should move in view-aligned XY plane
            let mouse_pan_right = -self.mouse_pan_x * mouse_pan_speed;
            let mouse_pan_up = self.mouse_pan_y * mouse_pan_speed;
            
            camera.pan(mouse_pan_right, mouse_pan_up);
        }
        
        // Handle orbit rotation (right button drag) - Z-up turntable style
        if self.is_orbiting && (self.mouse_delta_x != 0.0 || self.mouse_delta_y != 0.0) {
            // In Z-up turntable mode (like Blender/Maya):
            // X mouse movement -> rotate around Z world axis (yaw)
            // Y mouse movement -> rotate around horizontal axis (pitch)
            
            // Apply orbit with configured sensitivity
            let orbit_multiplier = self.orbit_speed * self.sensitivity * dt;
            
            // Update yaw based on horizontal mouse movement (around world Z axis)
            camera.yaw += Rad(self.mouse_delta_x * orbit_multiplier);
            
            // Update pitch based on vertical mouse movement
            // Invert if configured in preferences
            let pitch_delta = if self.orbit_invert_y {
                self.mouse_delta_y * orbit_multiplier
            } else {
                -self.mouse_delta_y * orbit_multiplier
            };
            
            camera.pitch += Rad(pitch_delta);
            
            // Keep pitch within safe limits to prevent gimbal lock
            // This is standard in 3D modeling software
            if camera.pitch < -Rad(SAFE_FRAC_PI_2) {
                camera.pitch = -Rad(SAFE_FRAC_PI_2);
            } else if camera.pitch > Rad(SAFE_FRAC_PI_2) {
                camera.pitch = Rad(SAFE_FRAC_PI_2);
            }
            
            // Update camera position after rotation
            camera.update_position();
        }
        
        // Handle zooming with scroll wheel (standard in all 3D software)
        if self.scroll != 0.0 {
            // Adjust distance with scroll (zoom in/out) with softer effect
            camera.distance *= 1.0 + self.scroll * self.zoom_speed;
            
            // Ensure camera doesn't get too close or too far
            camera.distance = camera.distance.max(MIN_ZOOM_DISTANCE).min(MAX_ZOOM_DISTANCE);
            
            // Reset scroll and update position
            self.scroll = 0.0;
            camera.update_position();
        }
    }
}
