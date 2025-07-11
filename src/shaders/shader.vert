#version 450

layout(location=0) in vec3 a_position;
layout(location=1) in vec2 a_tex_coords;
layout(location=2) in vec3 a_normal;
layout(location=3) in vec3 a_tangent;
layout(location=4) in vec3 a_bitangent;
layout(location=12) in vec3 a_color;

layout(location=5) in vec4 model_matrix_0;
layout(location=6) in vec4 model_matrix_1;
layout(location=7) in vec4 model_matrix_2;
layout(location=8) in vec4 model_matrix_3;
layout(location=9) in vec3 normal_matrix_0;
layout(location=10) in vec3 normal_matrix_1;
layout(location=11) in vec3 normal_matrix_2;

layout(location=0) out vec3 v_world_normal;
layout(location=1) out vec3 v_world_position;
layout(location=2) out vec3 v_flat_normal;
layout(location=3) out vec3 v_color;

layout(set=0, binding=0) 
uniform Camera {
    vec4 view_pos;
    mat4 view_proj;
};

void main() {
    mat4 model_matrix = mat4(
        model_matrix_0,
        model_matrix_1,
        model_matrix_2,
        model_matrix_3
    );
    
    mat3 normal_matrix = mat3(
        normal_matrix_0,
        normal_matrix_1,
        normal_matrix_2
    );

    v_world_normal = normalize(normal_matrix * a_normal);
    
    vec4 world_position = model_matrix * vec4(a_position, 1.0);
    v_world_position = world_position.xyz;
    
    // Use vertex normal for flat shading (same as world_normal in this case)
    v_flat_normal = v_world_normal;
    
    // Pass through vertex color
    v_color = a_color;

    gl_Position = view_proj * world_position;
}