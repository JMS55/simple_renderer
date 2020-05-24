#version 450

layout(set = 0, binding = 0) uniform texture2D t;
layout(set = 0, binding = 1) uniform sampler texture_sampler;
layout(location = 0) in vec2 texture_coordinate;
layout(location = 0) out vec4 color;

void main() {
   color = texture(sampler2D(t, texture_sampler), texture_coordinate);
}
