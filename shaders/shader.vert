#version 450

layout(location = 0) in vec2 position;
layout(location = 1) in vec3 vcolor;

layout(location = 0) out vec4 fcolor;


void main() {
    fcolor = vec4(vcolor, 1.0);
    gl_Position = vec4(position, 0.0, 1.0);
}