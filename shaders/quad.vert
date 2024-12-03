#version 440

layout(location = 0) in vec2 position;
layout(location = 1) in vec4 color;
layout(location = 0) out vec4 v_color;

layout(std140, set = 0, binding = 0) uniform InputUniform {
    mat4 matrix;
};

void main() {
    v_color = color;
    gl_Position = matrix * vec4(position, 0.0, 1.0);
}
