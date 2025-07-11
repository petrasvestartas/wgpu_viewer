#version 450

layout(location=0) in vec3 v_world_normal;
layout(location=1) in vec3 v_world_position;
layout(location=2) in vec3 v_flat_normal;
layout(location=3) in vec3 v_color;

layout(location=0) out vec4 f_color;

layout(set = 1, binding = 0) uniform Light {
    vec3 light_position;
    vec3 light_color;
};

void main() {
    // Use flat normal for flat shading
    vec3 normal = normalize(v_flat_normal);
    
    // Basic lighting calculation
    vec3 light_dir = normalize(light_position - v_world_position);
    float diffuse_strength = max(dot(normal, light_dir), 0.0);
    
    // Apply lighting to vertex color
    float ambient_strength = 0.3;
    vec3 ambient = ambient_strength * light_color;
    vec3 diffuse = diffuse_strength * light_color;
    
    vec3 result = (ambient + diffuse) * v_color;
    f_color = vec4(result, 1.0);
}