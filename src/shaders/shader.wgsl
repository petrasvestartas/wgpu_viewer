// Vertex shader

struct Camera {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
}
@group(0) @binding(0)
var<uniform> camera: Camera;

struct Light {
    position: vec3<f32>,
    color: vec3<f32>,
}
@group(1) @binding(0)
var<uniform> light: Light;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
    @location(12) color: vec3<f32>,
}

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) normal_matrix_0: vec3<f32>,
    @location(10) normal_matrix_1: vec3<f32>,
    @location(11) normal_matrix_2: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_position: vec3<f32>,
    @location(2) @interpolate(flat) flat_normal: vec3<f32>, // Explicitly use flat interpolation
    @location(3) color: vec3<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_2,
    );

    let world_normal = normalize(normal_matrix * model.normal);
    let world_position = model_matrix * vec4<f32>(model.position, 1.0);

    var out: VertexOutput;
    out.clip_position = camera.view_proj * world_position;
    out.world_normal = world_normal;
    out.world_position = world_position.xyz;
    out.flat_normal = world_normal; // For flat shading - will be flat interpolated
    out.color = model.color;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Use the flat interpolated normal for consistent face shading
    let face_normal = normalize(in.flat_normal);
    
    // Use the vertex color passed from the vertex shader
    let object_color = vec4<f32>(in.color, 1.0);
    
    // Lighting calculation for more natural shading
    let light_dir = normalize(light.position - in.world_position);
    
    // Increased ambient for better visibility of non-directly lit faces
    let ambient = 0.35 * light.color;
    
    // Hemisphere lighting - adds subtle blue-ish light from below (sky) and warm light from above (ground)
    // This simulates environment indirect lighting
    let hemisphere_up = vec3<f32>(0.0, 1.0, 0.0);
    let hemisphere_factor = 0.5 * (dot(face_normal, hemisphere_up) + 1.0); // 0-1 range
    let sky_color = vec3<f32>(0.6, 0.7, 0.9); // Subtle blue for sky light
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
    
    // Edge detection using dpdx and dpdy (supported derivatives)
    // This detects sharp changes in position which indicate edges
    let pos_dx = dpdx(in.world_position);
    let pos_dy = dpdy(in.world_position);
    let edge_factor = length(cross(pos_dx, pos_dy));
    
    // Edge threshold - adjust as needed for edge thickness
    let edge_threshold = 0.15;
    
    if (edge_factor > edge_threshold) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0); // Black edge
    }
    
    return vec4<f32>(result, 1.0); // Enhanced color with more visible shading
}