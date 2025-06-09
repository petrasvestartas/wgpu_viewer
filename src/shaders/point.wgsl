// Vertex shader for points with proper sizing support - HARDCODED VALUES TEST

struct CameraUniform {
    view_position: vec4<f32>,
    view_proj: mat4x4<f32>,
    aspect_ratio: vec4<f32>, // Only using x component
};

// Global configuration parameters
struct ConfigUniform {
    // First vec4: x=point_size, yzw=padding
    point_size_and_padding: vec4<f32>,
    // Second vec4: reserved for future parameters
    reserved: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> config: ConfigUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) corner: vec2<f32>,  // Corner offset [-1,-1] to [1,1]
    @location(3) size: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,  // Normalized coordinates for the fragment shader
};

@vertex
fn vs_main(vertex: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Get center position in world space
    let world_position = vec4<f32>(vertex.position, 1.0);
    
    // Transform to clip space
    let clip_pos = camera.view_proj * world_position;
    
    // Get point size from config uniform (now in the x component of the point_size_and_padding vec4)
    let point_size = config.point_size_and_padding.x;
    
    // Use dynamic aspect ratio from camera uniform
    let dynamic_aspect_ratio = camera.aspect_ratio.x;
    
    // Apply the corner offset in clip space with configurable point size and aspect ratio correction
    out.clip_position = vec4<f32>(
        clip_pos.x + vertex.corner.x * point_size * clip_pos.w,
        clip_pos.y + vertex.corner.y * point_size * dynamic_aspect_ratio * clip_pos.w,
        clip_pos.z,
        clip_pos.w
    );
    
    // Pass color to fragment shader
    out.color = vertex.color;
    
    // Create texture coordinates from corner ([-1,-1] to [1,1]) to ([0,0] to [1,1])
    out.tex_coords = vertex.corner * 0.5 + 0.5;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Calculate distance from center for circle shape
    // Convert tex_coords from [0,1] back to [-1,1] for distance calculation
    let point_coord = (in.tex_coords - 0.5) * 2.0;
    let distance_from_center = length(point_coord);
    
    // Discard fragments outside the circle
    if (distance_from_center > 1.0) {
        discard;
    }
    
    // Smooth edge of the circle
    let alpha = 1.0 - smoothstep(0.8, 1.0, distance_from_center);
    
    // Return color with calculated alpha
    return vec4<f32>(in.color, alpha);
}
