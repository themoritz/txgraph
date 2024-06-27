#version 300 es
precision mediump float;

const vec2 verts[6] = vec2[6](
    vec2(-1.0, -1.0),
    vec2( 1.0, -1.0),
    vec2(-1.0,  1.0),
    vec2(-1.0,  1.0),
    vec2( 1.0, -1.0),
    vec2( 1.0,  1.0)
);

void main() {
    gl_Position = vec4(verts[gl_VertexID], 0.0, 1.0);
}
