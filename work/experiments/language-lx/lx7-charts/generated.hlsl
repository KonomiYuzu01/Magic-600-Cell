// LX-7 experiment, generated deterministically by generate_hlsl.py.
// SM 6.2, strict IEEE operations, denormals preserved; no vendor features.
struct Input { float4 vertex; float4 q0; float4 q1; float4 q2; float4 q3;
               float d; uint stage; uint pad0; uint pad1; };
StructuredBuffer<Input> inputs : register(t0);
RWStructuredBuffer<uint4> outputs : register(u0);
cbuffer Count : register(b0) { uint count; };
uint4 missing(uint reason) { return uint4(reason, 0u, 0u, 0u); }
float row(float4 q, float4 v) {
    precise float a = q.x * v.x;
    precise float b = q.y * v.y;
    precise float c = q.z * v.z;
    precise float d = q.w * v.w;
    precise float ab = a + b;
    precise float abc = ab + c;
    precise float abcd = abc + d;
    return abcd;
}
// Frozen built-in perspective formula from renderer-sb/probe/shaders/geometry.hlsl.
// Only its 4D-to-3D step; the experiment adds the same coverage envelope as charts.
uint4 builtin_perspective(float4 p, float d) {
    if (!all(isfinite(p)) || !isfinite(d)) return missing(2u);
    precise float den = d - p.w;
    if (!isfinite(den)) return missing(2u);
    if (abs(den) <= 0.000244140625f) return missing(1u);
    precise float nx = d * p.x;
    precise float ny = d * p.y;
    precise float nz = d * p.z;
    if (!isfinite(nx) || !isfinite(ny) || !isfinite(nz)) return missing(2u);
    precise float px = nx / den;
    precise float py = ny / den;
    precise float pz = nz / den;
    if (!isfinite(px) || !isfinite(py) || !isfinite(pz)) return missing(2u);
    return uint4(0u, asuint(px), asuint(py), asuint(pz));
}

uint4 chart_perspective(float4 p, float d) {
    if (!all(isfinite(p)) || !isfinite(d)) return missing(2u);
    precise float t0 = d - p.w;
    if (!isfinite(t0)) return missing(2u);
    if (abs(t0) <= asfloat(0x39800000u)) return missing(1u);
    precise float t2 = d * p.x;
    if (!isfinite(t2)) return missing(2u);
    if (t0 == 0.0f) return missing(2u);
    precise float t3 = t2 / t0;
    if (!isfinite(t3)) return missing(2u);
    precise float t4 = d * p.y;
    if (!isfinite(t4)) return missing(2u);
    if (t0 == 0.0f) return missing(2u);
    precise float t5 = t4 / t0;
    if (!isfinite(t5)) return missing(2u);
    precise float t6 = d * p.z;
    if (!isfinite(t6)) return missing(2u);
    if (t0 == 0.0f) return missing(2u);
    precise float t7 = t6 / t0;
    if (!isfinite(t7)) return missing(2u);
    return uint4(0u, asuint(t3), asuint(t5), asuint(t7));
}

uint4 chart_stereographic(float4 p, float d) {
    if (!all(isfinite(p)) || !isfinite(d)) return missing(2u);
    precise float t0 = asfloat(0x3f800000u) - p.w;
    if (!isfinite(t0)) return missing(2u);
    if (abs(t0) <= asfloat(0x39800000u)) return missing(1u);
    if (t0 == 0.0f) return missing(2u);
    precise float t2 = p.x / t0;
    if (!isfinite(t2)) return missing(2u);
    if (t0 == 0.0f) return missing(2u);
    precise float t3 = p.y / t0;
    if (!isfinite(t3)) return missing(2u);
    if (t0 == 0.0f) return missing(2u);
    precise float t4 = p.z / t0;
    if (!isfinite(t4)) return missing(2u);
    return uint4(0u, asuint(t2), asuint(t3), asuint(t4));
}

uint4 chart_gnomonic(float4 p, float d) {
    if (!all(isfinite(p)) || !isfinite(d)) return missing(2u);
    if (abs(p.w) <= asfloat(0x39800000u)) return missing(1u);
    if (p.w == 0.0f) return missing(2u);
    precise float t1 = p.x / p.w;
    if (!isfinite(t1)) return missing(2u);
    if (p.w == 0.0f) return missing(2u);
    precise float t2 = p.y / p.w;
    if (!isfinite(t2)) return missing(2u);
    if (p.w == 0.0f) return missing(2u);
    precise float t3 = p.z / p.w;
    if (!isfinite(t3)) return missing(2u);
    return uint4(0u, asuint(t1), asuint(t2), asuint(t3));
}

[numthreads(64, 1, 1)]
void main(uint3 tid : SV_DispatchThreadID) {
    if (tid.x >= count) return;
    Input v = inputs[tid.x];
    precise float4 p = float4(row(v.q0, v.vertex), row(v.q1, v.vertex),
                            row(v.q2, v.vertex), row(v.q3, v.vertex));
    uint4 result = missing(3u); // Invalid stage is a harness error, never a point.
    switch (v.stage) {
    case 0u: result = builtin_perspective(p, v.d); break;
    case 1u: result = chart_perspective(p, v.d); break;
    case 2u: result = chart_stereographic(p, v.d); break;
    case 3u: result = chart_gnomonic(p, v.d); break;
    }
    outputs[tid.x] = result;
}
