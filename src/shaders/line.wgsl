// Vertex shader for lines

struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: Camera;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    // The mesh boxes are rotated by 45 degrees, but our lines don't have that rotation
    // Adding a 45-degree rotation around Y axis to match the box orientation
    let theta = 0.0; // Rotation in radians (0.0 means no rotation)
    let c = cos(theta);
    let s = sin(theta);
    
    // Create rotation matrix around Y axis
    let rot_y = mat4x4<f32>(
        vec4<f32>(c, 0.0, s, 0.0),
        vec4<f32>(0.0, 1.0, 0.0, 0.0),
        vec4<f32>(-s, 0.0, c, 0.0),
        vec4<f32>(0.0, 0.0, 0.0, 1.0)
    );
    
    // Apply rotation and then camera projection
    let world_position = rot_y * vec4<f32>(model.position, 1.0);
    out.clip_position = camera.view_proj * world_position;
    
    out.color = model.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}
