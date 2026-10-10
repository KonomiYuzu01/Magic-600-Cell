#include "geometry.hlsl"
#ifdef COUNT_INVOCATIONS
RWStructuredBuffer<uint> invocationCount : register(u0);
#endif
struct VertexOut {
    float4 position : SV_Position;
    float3 projected : TEXCOORD0;
    nointerpolation uint colourClass : TEXCOORD1;
    nointerpolation uint marked : TEXCOORD2;
};
VertexOut vsMain(uint vi : SV_VertexID, uint cell : SV_InstanceID) {
#ifdef COUNT_INVOCATIONS
    InterlockedAdd(invocationCount[cell], 1);
#endif
    Projection p = projectVertex(cell, vi);
    VertexOut result;
    result.position = p.clip;
    result.projected = p.projected;
    result.colourClass = labels[p.slot] / 433;
    result.marked = overlayMode == 3 ? straddles[slotPieces[p.slot]] : 0;
    return result;
}
float3 hsv(float h, float s, float v) {
    float3 p = abs(frac(h.xxx + float3(0, 2.0/3.0, 1.0/3.0))*6-3);
    return v * lerp(float3(1,1,1), saturate(p-1), s);
}
float4 psMain(VertexOut input) : SV_Target {
    float3 n = normalize(cross(ddx(input.projected), ddy(input.projected)));
    float shade = 0.5 + 0.5*abs(dot(n, normalize(float3(0.3,0.5,1))));
    if (input.marked != 0) return float4(float3(1, .85, .15)*shade, 1);
    return float4(hsv(frac(float(input.colourClass)*0.61803398875), 0.61, 0.90)*shade, 1);
}
