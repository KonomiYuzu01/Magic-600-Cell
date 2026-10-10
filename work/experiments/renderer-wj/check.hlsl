#include "geometry.hlsl"
RWStructuredBuffer<float3> projectedSamples : register(u1);
[numthreads(64,1,1)]
void csMain(uint3 thread : SV_DispatchThreadID) {
    if (thread.x >= sampleCount) return;
    uint g = samples[thread.x];
    Projection p = projectVertex(g/30480, g%30480);
    projectedSamples[thread.x] = float3(p.clip.x/p.clip.w, p.clip.y/p.clip.w, p.clip.w);
}
