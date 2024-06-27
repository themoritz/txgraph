#version 300 es
precision mediump float;

out vec4 fragColor;

uniform sampler2D u_rects;
uniform int u_numRects;
uniform float u_repulsionRadius;
uniform float u_scaleSquared;

float kernel(float radius, float distance) {
    float value = 1.0 - distance / radius * distance / radius;
    return value * value;
}

float clearSpacing(vec4 a, vec4 b) {
    float x = abs(a.x - b.x) - (a.z + b.z) / 2.0;
    float y = abs(a.y - b.y) - (a.w + b.w) / 2.0;
    return max(max(x, y), 2.0);
}

void main() {
    int i = int(gl_FragCoord.x);

    vec4 rect_i = texelFetch(u_rects, ivec2(i, 0), 0);

    vec2 force = vec2(0.0);
    for (int j = 0; j < u_numRects; j++) {
        if (i == j) continue;

        vec4 rect_j = texelFetch(u_rects, ivec2(j, 0), 0);
        float spacing = clearSpacing(rect_i, rect_j);

        if (spacing > u_repulsionRadius) continue;

        vec2 pos_i = rect_i.xy;
        vec2 pos_j = rect_j.xy;
        vec2 delta = normalize(pos_i - pos_j);
        force += u_scaleSquared / spacing * kernel(u_repulsionRadius, spacing) * delta;
    }

    fragColor = vec4(force, 0.0, 0.0);
}
