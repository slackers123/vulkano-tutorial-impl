#version 450

layout(location = 0) out vec4 f_color;
layout(location = 0) in vec4 fcolor;

void main() {
    f_color = fcolor;
}