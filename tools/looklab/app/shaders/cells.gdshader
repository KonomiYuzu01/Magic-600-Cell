shader_type spatial;
render_mode unshaded, cull_disabled, shadows_disabled, fog_disabled;

uniform sampler2D centers : filter_nearest, repeat_disable;
uniform sampler2D labels : filter_nearest, repeat_disable;
uniform sampler2D moving_slots : filter_nearest, repeat_disable;
uniform sampler2D cell_classes : filter_nearest, repeat_disable;
uniform sampler2D palette : filter_nearest, repeat_disable;
uniform sampler2D cell_rings : filter_nearest, repeat_disable;
uniform sampler2D slot_orbits : filter_nearest, repeat_disable;
uniform vec4 base_normal;
uniform float radius;
uniform float cs = 0.76;
uniform float ss = 0.82;
uniform vec4 plane_u;
uniform vec4 plane_v;
uniform float theta = 0.0;
uniform mat4 q;
uniform float d4 = 1.18;
uniform float zoom = 1.15;
uniform float aspect = 1.6;
uniform int projection_kind = 0;
uniform bool cellsVisible = true;
uniform bool sliceVisible = false;
uniform float sliceMin = -1.0;
uniform float sliceMax = 1.0;
uniform float edgeWeight = 0.0;
uniform float edgeBrightness = 0.0;
uniform float gloss = 0.0;
uniform float glow = 0.0;
uniform float fog = 0.0;
uniform float keyLight = 0.5;
uniform float fillLight = 0.5;
uniform int finish_kind = 0;
uniform int highlight_kind = 0;
uniform vec4 accent;
uniform vec4 background;
// LL5: linear-light CVD, off for normal vision and geometry probes.
uniform bool cvd_enabled = false;
uniform vec4 cvd_r0 = vec4(1.0, 0.0, 0.0, 0.0);
uniform vec4 cvd_r1 = vec4(0.0, 1.0, 0.0, 0.0);
uniform vec4 cvd_r2 = vec4(0.0, 0.0, 1.0, 0.0);
// Test modes use these same vertex and fragment functions in an RGBA32F
// framebuffer. Normal viewport draws keep geometry_check and transpose_q off.
uniform bool geometry_check = false;
uniform bool transpose_q = false;
uniform int geometry_width = 300;

varying vec3 projected;
varying float visible_w;
varying vec3 barycentric;
varying vec3 sticker_colour;
varying float moving;
varying vec3 geometry_value;

void vertex() {
    int local = int(UV.y);
    int slot = INSTANCE_ID * 433 + local;
    ivec2 slot_texel = ivec2(slot % 1024, slot / 1024);
    vec4 base = vec4(VERTEX, UV.x);
    vec4 center = texelFetch(centers, ivec2(local, 0), 0);
    vec4 shrunk = (base_normal + cs * (center - base_normal) + cs * ss * (base - center)) / radius;
    mat4 frame = mat4(vec4(MODEL_MATRIX[0].xyz, INSTANCE_CUSTOM.r),
                      vec4(MODEL_MATRIX[1].xyz, INSTANCE_CUSTOM.g),
                      vec4(MODEL_MATRIX[2].xyz, INSTANCE_CUSTOM.b),
                      vec4(MODEL_MATRIX[3].xyz, INSTANCE_CUSTOM.a));
    vec4 world = frame * shrunk;
    moving = texelFetch(moving_slots, slot_texel, 0).r;
    if (moving > 0.5 && theta != 0.0) {
        float x = dot(world, plane_u);
        float y = dot(world, plane_v);
        float c = cos(theta);
        float s = sin(theta);
        world += ((c - 1.0) * x - s * y) * plane_u + (s * x + (c - 1.0) * y) * plane_v;
    }
    visible_w = world.w;
    if (transpose_q) world = transpose(q) * world;
    else world = q * world;
    float factor = 1.0;
    if (projection_kind == 0) factor = d4 / (d4 - world.w);
    if (projection_kind == 1) factor = 1.0 / (1.0 - world.w);
    projected = world.xyz * factor;
    float w = 5.0 - projected.z;
    geometry_value = vec3(projected.x * zoom / aspect / w, projected.y * zoom / w, w);
    // Godot's Vulkan/D3D12 depth is reversed (near=1, far=0).
    POSITION = vec4(projected.x * zoom / aspect, projected.y * zoom, (0.05 / 99.95) * (100.0 - w), w);
    int label = int(texelFetch(labels, slot_texel, 0).r);
    int colour_class = int(texelFetch(cell_classes, ivec2(label / 433, 0), 0).r);
    sticker_colour = texelFetch(palette, ivec2(colour_class, 0), 0).rgb;
    if (highlight_kind == 1 && int(texelFetch(cell_rings, ivec2(INSTANCE_ID, 0), 0).r) == 0) sticker_colour = accent.rgb;
    if (highlight_kind == 2 && int(texelFetch(slot_orbits, slot_texel, 0).r) == 34) sticker_colour = accent.rgb;
    int corner = int(UV2.y);
    barycentric = vec3(corner == 0 ? 1.0 : 0.0, corner == 1 ? 1.0 : 0.0, corner == 2 ? 1.0 : 0.0);
    if (geometry_check) {
        // Three copies of each sampled vertex form a tiny triangle around one
        // pixel centre. Its payload is constant; neighbours cannot overlap it.
        vec2 offset = vec2(0.0, 0.4);
        if (corner == 0) offset = vec2(-0.4, -0.4);
        if (corner == 1) offset = vec2(0.4, -0.4);
        POSITION = vec4(2.0 * (UV2.x + 0.5 + offset.x) / float(geometry_width) - 1.0, offset.y * 2.0, 0.0, 1.0);
    }
}

void fragment() {
    if (geometry_check) {
        ALBEDO = geometry_value;
    } else {
    if (!cellsVisible || (sliceVisible && (visible_w < sliceMin || visible_w > sliceMax))) discard;
    vec3 n = normalize(cross(dFdx(projected), dFdy(projected)));
    vec3 eye = normalize(vec3(0.0, 0.0, 5.0) - projected);
    if (dot(n, eye) < 0.0) n = -n;
    vec3 light = normalize(vec3(0.4, 0.7, 0.6));
    float diffuse = fillLight * 0.7 + keyLight * 1.3 * max(dot(n, light), 0.0);
    float specular = gloss * pow(max(dot(n, normalize(light + eye)), 0.0), mix(8.0, 96.0, gloss));
    float rim = glow * pow(1.0 - max(dot(n, eye), 0.0), 2.0);
    vec3 colour = sticker_colour * diffuse + vec3(specular) + sticker_colour * rim * 1.5;
    if (finish_kind == 1) colour += sticker_colour * specular * 0.5;
    if (finish_kind == 2) colour += vec3(specular) * 0.5;
    if (edgeWeight > 0.0) {
        vec3 e = smoothstep(vec3(0.0), fwidth(barycentric) * edgeWeight, barycentric);
        float edge = 1.0 - min(min(e.x, e.y), e.z);
        colour = mix(colour, vec3(edgeBrightness), edge * 0.85);
    }
    float depth_fog = 1.0 - exp(-fog * 0.6 * max(5.0 - projected.z - 1.5, 0.0));
    ALBEDO = clamp(mix(colour, background.rgb, clamp(depth_fog, 0.0, 1.0)), vec3(0.0), vec3(1.0));
    if (cvd_enabled) {
        ALBEDO = clamp(vec3(dot(cvd_r0.xyz, ALBEDO), dot(cvd_r1.xyz, ALBEDO), dot(cvd_r2.xyz, ALBEDO)), vec3(0.0), vec3(1.0));
    }
    }
}
