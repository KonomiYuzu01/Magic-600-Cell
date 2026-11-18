// Shared by the instanced vertex shader and the sampled compute check.
StructuredBuffer<float4> vertices : register(t0);
StructuredBuffer<uint> stickerIds : register(t1);
StructuredBuffer<float4> centers : register(t2);
StructuredBuffer<float4> frames : register(t3); // four columns per cell
StructuredBuffer<uint> animated : register(t4);
StructuredBuffer<uint> labels : register(t5);
StructuredBuffer<uint> samples : register(t6);
StructuredBuffer<uint> slotPieces : register(t7);
StructuredBuffer<int> poseIndex : register(t8);
StructuredBuffer<float4> poseRows : register(t9); // four rows per matrix, x' = M x
StructuredBuffer<int> poseLattice : register(t10);
StructuredBuffer<uint> destinations : register(t11);
struct OverlayPoint {float4 world; float4 colour;};
StructuredBuffer<OverlayPoint> overlayPoints : register(t12);
StructuredBuffer<uint> straddles : register(t13);
StructuredBuffer<uint> offsets : register(t14);
cbuffer Constants : register(b0) {
    column_major float4x4 Q;
    float4 N0;
    float radius, cellShrink, stickerShrink, d4;
    float zoom, aspect, theta;
    uint sampleCount;
    float4 turnU, turnV;
    uint poseEnabled, overlayMode, overlayCount, poseCount;
    float previewAngle, maxAngle, pad0, pad1;
};
struct Projection {
    float4 clip;
    float3 projected;
    uint slot;
};
float4 homePoint(uint cell, float4 center, float4 vertex) {
    float4 v = (N0 + cellShrink * (center - N0)
                + cellShrink * stickerShrink * (vertex - center)) / radius;
    return frames[cell*4] * v.x + frames[cell*4+1] * v.y
                 + frames[cell*4+2] * v.z + frames[cell*4+3] * v.w;
}
float4 posedPoint(uint slot, float4 world) {
    if (poseEnabled == 0) return world;
    uint pid = poseIndex[slotPieces[slot]];
    return float4(dot(poseRows[pid*4], world), dot(poseRows[pid*4+1], world),
                  dot(poseRows[pid*4+2], world), dot(poseRows[pid*4+3], world));
}
Projection projectWorld(float4 world) {
    world = mul(Q, world);
    float3 p = d4 * world.xyz / (d4 - world.w);
    float z = 5 - p.z;
    Projection result;
    result.clip = float4(p.x*zoom/aspect, p.y*zoom, 100.0/(100.0-0.05)*(z-0.05), z);
    result.projected = p;
    result.slot = 0;
    return result;
}
Projection projectVertex(uint cell, uint vi) {
    uint local = stickerIds[vi];
    uint slot = cell * 433 + local;
    float4 world = posedPoint(slot, homePoint(cell, centers[local], vertices[vi]));
    if (theta != 0 && animated[slot] != 0) {
        float x = dot(world, turnU), y = dot(world, turnV);
        float c = cos(theta), s = sin(theta);
        world += ((c-1)*x-s*y)*turnU + (s*x+(c-1)*y)*turnV;
    }
    Projection result = projectWorld(world);
    result.slot = slot;
    return result;
}
