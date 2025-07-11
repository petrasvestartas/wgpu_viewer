// Vertex shader for polygon rendering with lighting support

struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> camera: CameraUniform;

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
};
@group(1) @binding(0)
var<uniform> light: Light;

// Vertex shader input
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

// Output from vertex to fragment shader
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) world_position: vec3<f32>,
};

@vertex
fn vs_main(
    vertex: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    let world_position = vec4<f32>(vertex.position, 1.0);
    out.clip_position = camera.view_proj * world_position;
    out.color = vertex.color;
    out.world_position = world_position.xyz;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Compute face normal using derivatives (same as mesh shader)
    let pos_dx = dpdx(in.world_position);
    let pos_dy = dpdy(in.world_position);
    let face_normal = normalize(cross(pos_dy, pos_dx)); // Note: order matters for winding
    
    // Use the vertex color passed from the vertex shader
    let object_color = vec4<f32>(in.color, 1.0);
    
    // Lighting calculation (same as mesh shader)
    let light_dir = normalize(light.position - in.world_position);
    
    // Increased ambient for better visibility of non-directly lit faces
    let ambient = 0.35 * light.color;
    
    // Hemisphere lighting - adds subtle blue-ish light from below (sky) and warm light from above (ground)
    let hemisphere_factor = 0.5 + 0.5 * dot(face_normal, vec3<f32>(0.0, 1.0, 0.0));
    let sky_color = vec3<f32>(0.1, 0.3, 0.6); // Blue-ish color for sky
    let ground_color = vec3<f32>(0.4, 0.3, 0.2); // Warm color for ground bounce
    let hemisphere = mix(ground_color, sky_color, hemisphere_factor) * 0.2;
    
    // Enhanced diffuse lighting with softer falloff
    let diff = max(dot(face_normal, light_dir), 0.0);
    // Use a modified diffuse term that has some light even at glancing angles
    let wrapped_diff = max(0.1 + 0.9 * diff, 0.0); 
    let diffuse = wrapped_diff * light.color;
    
    // Enhanced specular highlight
    let view_dir = normalize(camera.view_pos.xyz - in.world_position);
    let half_dir = normalize(light_dir + view_dir);
    let spec = pow(max(dot(face_normal, half_dir), 0.0), 32.0);
    let specular = 0.4 * spec * light.color;
    
    // Fresnel effect to brighten edges for more natural look
    let fresnel_factor = pow(1.0 - max(0.0, dot(view_dir, face_normal)), 2.0) * 0.2;
    
    // Combine all lighting components
    let result = (ambient + hemisphere + diffuse + specular + fresnel_factor) * object_color.xyz;
    
    return vec4<f32>(result, 1.0); // Enhanced color with lighting
}
