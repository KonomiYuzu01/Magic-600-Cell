#include "draw.hlsl"
StructuredBuffer<uint> edgeMasks : register(t7);
struct OutlineOut {
    float4 position : SV_Position;
    float3 projected : TEXCOORD0;
    nointerpolation uint colourClass : TEXCOORD1;
    noperspective float3 barycentric : TEXCOORD2;
    nointerpolation uint mask : TEXCOORD3;
};
OutlineOut outlineVS(uint vi : SV_VertexID, uint cell : SV_InstanceID) {
    VertexOut base = vsMain(vi, cell);
    OutlineOut result;
    result.position = base.position;
    result.projected = base.projected;
    result.colourClass = base.colourClass;
    result.barycentric = float3(vi%3==0, vi%3==1, vi%3==2);
    result.mask = edgeMasks[vi/3];
    return result;
}
float4 outlinePS(OutlineOut input) : SV_Target {
    VertexOut base;
    base.position = input.position;
    base.projected = input.projected;
    base.colourClass = input.colourClass;
    float3 dx = ddx(input.barycentric), dy = ddy(input.barycentric);
    float3 distancePx = input.barycentric / max(sqrt(dx*dx+dy*dy), 1e-8);
    float nearest = 1e8;
    for (uint edge=0; edge<3; ++edge)
        if ((input.mask & (1u<<edge)) != 0) nearest = min(nearest, distancePx[edge]);
    float coverage = 1-smoothstep(1.0, 2.0, nearest);
    return float4(lerp(psMain(base).rgb, float3(0.025,0.028,0.032), coverage), 1);
}
