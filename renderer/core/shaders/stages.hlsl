// C9 U5: the device passes of a scene frame with a user stage in its chain (sa2_stage_compile, sa2_scene_stages).
// The core composes this file at run time after the stage text, geometry.hlsl and draw.hlsl (whose #include line it
// blanks) and compiles it through DXC with the frozen profile. A vertex compile defines M600_STAGE_VERTEX, a fragment
// compile M600_STAGE_FRAGMENT; the core's own compile defines neither.
//
// A stage text that names m600_vertex_arrays or m600_fragment_arrays declares that buffer itself: the core then
// defines M600_VERTEX_ARRAYS_DECLARED or M600_FRAGMENT_ARRAYS_DECLARED after the stage text (a stage text may also
// define the guard itself), and this file declares a buffer only when its guard is undefined.
#ifndef M600_VERTEX_ARRAYS_DECLARED
#define M600_VERTEX_ARRAYS_DECLARED 1
StructuredBuffer<float> m600_vertex_arrays : register(t20);
#endif
#ifndef M600_FRAGMENT_ARRAYS_DECLARED
#define M600_FRAGMENT_ARRAYS_DECLARED 1
StructuredBuffer<float> m600_fragment_arrays : register(t21);
#endif

// The frame's root constants (eight 32-bit values).
cbuffer M600StageFrame : register(b1) {
    uint m600Width, m600Height, m600AllowanceLow, m600AllowanceHigh;
    float m600Time;
    uint m600Pad0, m600Pad1, m600Pad2;
};
// The charged buffer of evaluated positions: one record per (cell, vertex) of the instanced list, cell-major. A defined
// vertex stores its clip position; an undefined one stores (0, 0, 0, NaN) with its coverage code in the NaN's payload.
StructuredBuffer<uint4> m600Eval : register(t22);
RWStructuredBuffer<uint4> m600EvalOut : register(u2);
// The frame row: U (bytes 0..7, two 32-bit words), the predicate (8..15: 1 runs the user chain, 0 stops it) and the
// report (16..31: U, the allowance; 32: stopped).
RWByteAddressBuffer m600FrameOut : register(u3);

static const uint M600_VERTICES = 30480u;
static const uint M600_TRIANGLES = 10160u;

// camera3 and clip: projectWorld's lines after the 4D projection, for a stage's 3D position.
float4 m600_clip(float3 p) {
    float z = 5 - p.z;
    return float4(p.x*zoom/aspect, p.y*zoom, 100.0/(100.0-0.05)*(z-0.05), z);
}
bool m600_defined(float4 clip) {
    return all(isfinite(clip));
}
void m600_store(uint index, uint code, float4 clip) {
    if (code == 0u && !m600_defined(clip)) code = 2u;
    if (code == 0u) m600EvalOut[index] = asuint(clip);
    else m600EvalOut[index] = uint4(0u, 0u, 0u, 0x7FC00000u | min(code, 0x003FFFFFu));
}

// eval with the built-in vertex stage: the built-in projection.
[numthreads(64, 1, 1)]
void m600_eval_builtin(uint3 id : SV_DispatchThreadID) {
    if (id.x >= M600_VERTICES) return;
    Projection p = projectVertex(id.y, id.x);
    m600_store(id.y * M600_VERTICES + id.x, 0u, p.clip);
}

#ifdef M600_STAGE_VERTEX
// eval with a user vertex stage: projectVertex's chain up to Q * world, the stage, then camera3 and clip.
[numthreads(64, 1, 1)]
void m600_eval_stage(uint3 id : SV_DispatchThreadID) {
    if (id.x >= M600_VERTICES) return;
    uint cell = id.y, vi = id.x;
    uint local = stickerIds[vi];
    uint slot = cell * 433 + local;
    float4 world = posedPoint(slot, homePoint(cell, centers[local], vertices[vi]));
    if (theta != 0 && animated[slot] != 0) {
        float x = dot(world, turnU), y = dot(world, turnV);
        float c = cos(theta), s = sin(theta);
        world += ((c-1)*x-s*y)*turnU + (s*x+(c-1)*y)*turnV;
    }
    world = mul(Q, world);
    float3 position = float3(0, 0, 0);
    uint code = m600_vertex(world.x, world.y, world.z, world.w, m600Time, position);
    m600_store(cell * M600_VERTICES + vi, code, code == 0u ? m600_clip(position) : float4(0, 0, 0, 0));
}
#endif

// bound: RPI.72 step 3 over the evaluated positions, exactly as rust/c600-display/src/raster.rs counts a triangle the
// adapter placed or classified. Classification (the adapter's convention, C9): a triangle with a nonfinite component
// is undefined (0); one with all three vertices beyond one clip plane is outside (0); one with a vertex in front of the
// near plane (z < 0) or with w not above 0 crosses the near plane (the whole surface); any other is placed on the
// surface with D3D12's viewport transform.
float2 m600_pixel(float4 clip) {
    precise float nx = clip.x / clip.w;
    precise float ny = clip.y / clip.w;
    precise float px = (nx * 0.5f + 0.5f) * (float)m600Width;
    precise float py = (0.5f - ny * 0.5f) * (float)m600Height;
    return float2(px, py);
}
uint m600_extent(float low, float high, uint size) {
    int l = (int)clamp(floor(low), 0.0f, (float)size);
    int h = (int)clamp(ceil(high), 0.0f, (float)size);
    if (l >= h) return 0u;
    l = l & ~1;
    h = (h + 1) & ~1;
    return (uint)(h - l);
}
uint m600_count(float4 a, float4 b, float4 c) {
    if (!m600_defined(a) || !m600_defined(b) || !m600_defined(c)) return 0u;
    if ((a.x > a.w && b.x > b.w && c.x > c.w) || (a.x < -a.w && b.x < -b.w && c.x < -c.w)
        || (a.y > a.w && b.y > b.w && c.y > c.w) || (a.y < -a.w && b.y < -b.w && c.y < -c.w)
        || (a.z < 0 && b.z < 0 && c.z < 0) || (a.z > a.w && b.z > b.w && c.z > c.w)) return 0u;
    if (a.z < 0 || b.z < 0 || c.z < 0 || !(a.w > 0) || !(b.w > 0) || !(c.w > 0)) return m600Width * m600Height;
    float2 p0 = m600_pixel(a), p1 = m600_pixel(b), p2 = m600_pixel(c);
    if (!all(isfinite(float4(p0, p1))) || !all(isfinite(p2))) return 0u;
    uint ex = m600_extent(min(min(p0.x, p1.x), p2.x), max(max(p0.x, p1.x), p2.x), m600Width);
    uint ey = m600_extent(min(min(p0.y, p1.y), p2.y), max(max(p0.y, p1.y), p2.y), m600Height);
    return ex * ey;
}
// One thread per triangle; the 64-bit sum is exact: each wave sums the 16-bit halves of its counts, and its first lane
// adds the wave's total to U with two 32-bit atomics and the carry.
[numthreads(64, 1, 1)]
void m600_bound(uint3 id : SV_DispatchThreadID) {
    uint count = 0u;
    if (id.x < M600_TRIANGLES) {
        uint base = id.y * M600_VERTICES + 3u * id.x;
        count = m600_count(asfloat(m600Eval[base]), asfloat(m600Eval[base + 1u]), asfloat(m600Eval[base + 2u]));
    }
    uint low = WaveActiveSum(count & 0xFFFFu);
    uint high = WaveActiveSum(count >> 16);
    if (WaveIsFirstLane()) {
        uint add = low + ((high & 0xFFFFu) << 16);
        uint carry = add < low ? 1u : 0u;
        uint before = 0u;
        m600FrameOut.InterlockedAdd(0u, add, before);
        if (before + add < before) carry += 1u;
        uint upper = (high >> 16) + carry;
        uint ignored = 0u;
        if (upper != 0u) m600FrameOut.InterlockedAdd(4u, upper, ignored);
    }
}
// decide: RPI.72 step 4. The frame stops when U exceeds the allowance; at equality it runs.
[numthreads(1, 1, 1)]
void m600_decide(uint3 id : SV_DispatchThreadID) {
    uint2 u = m600FrameOut.Load2(0u);
    bool stopped = u.y > m600AllowanceHigh || (u.y == m600AllowanceHigh && u.x > m600AllowanceLow);
    m600FrameOut.Store2(8u, uint2(stopped ? 0u : 1u, 0u));
    m600FrameOut.Store4(16u, uint4(u.x, u.y, m600AllowanceLow, m600AllowanceHigh));
    m600FrameOut.Store4(32u, uint4(stopped ? 1u : 0u, 0u, 0u, 0u));
}

// The user draw's vertex stage: it passes eval's clip positions through. A triangle with an undefined vertex goes
// wholly in front of the near plane, so it draws nothing. The fragment inputs follow vsMain; the projected position
// is recovered from the clip position (equal to the stage's position within rounding).
VertexOut m600_pass_vs(uint vi : SV_VertexID, uint cell : SV_InstanceID) {
    uint first = cell * M600_VERTICES + vi - vi % 3u;
    float4 clip = asfloat(m600Eval[cell * M600_VERTICES + vi]);
    bool undefined = !m600_defined(asfloat(m600Eval[first])) || !m600_defined(asfloat(m600Eval[first + 1u]))
        || !m600_defined(asfloat(m600Eval[first + 2u]));
    uint slot = cell * 433 + stickerIds[vi];
    VertexOut result;
    if (undefined) {
        result.position = float4(0, 0, -1, 1);
        result.projected = float3(0, 0, 0);
    } else {
        result.position = clip;
        result.projected = float3(clip.x * aspect / zoom, clip.y / zoom, 5 - clip.w);
    }
    result.colourClass = labels[slot] / 433;
    result.marked = overlayMode == 3 ? straddles[slotPieces[slot]] : 0;
    return result;
}

#ifdef M600_STAGE_FRAGMENT
// The user draw's pixel stage with a user fragment stage: psMain's shade, then the stage's opaque colour.
float4 m600_fragment_ps(VertexOut input) : SV_Target {
    float3 n = normalize(cross(ddx(input.projected), ddy(input.projected)));
    float shade = 0.5 + 0.5*abs(dot(n, normalize(float3(0.3,0.5,1))));
    float3 rgb = m600_fragment(float(input.colourClass), shade, input.projected.x, input.projected.y,
                               input.projected.z, m600Time);
    return float4(rgb, 1);
}
#endif
