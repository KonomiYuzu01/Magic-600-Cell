#include "geometry.hlsl"
RWStructuredBuffer<uint4> latticeResults : register(u1);
float largest(float4 x) {return max(max(x.x,x.y),max(x.z,x.w));}
[numthreads(64,1,1)]
void latticeMain(uint3 thread : SV_DispatchThreadID) {
    uint slot = thread.x;
    if (slot >= 259800) return;
    uint dest = destinations[slot];
    uint cell = slot / 433, local = slot % 433;
    uint dcell = dest / 433, dlocal = dest % 433;
    uint pid = poseIndex[slotPieces[slot]];
    float4 leftCenter = posedPoint(slot, homePoint(cell, centers[local], centers[local]));
    float4 rightCenter = homePoint(dcell, centers[dlocal], centers[dlocal]);
    uint labelError = labels[dest] != slot;
    uint centerError = largest(abs(leftCenter - rightCenter)) > 2e-6;
    uint geometryError = 0;
    if (pid != 0) {
        float4 lmin = 1e30, lmax = -1e30, rmin = 1e30, rmax = -1e30;
        for (uint vi = offsets[local]; vi < offsets[local+1]; ++vi) {
            float4 world = posedPoint(slot, homePoint(cell, centers[local], vertices[vi]));
            lmin = min(lmin, world); lmax = max(lmax, world);
        }
        for (uint vi = offsets[dlocal]; vi < offsets[dlocal+1]; ++vi) {
            float4 world = homePoint(dcell, centers[dlocal], vertices[vi]);
            rmin = min(rmin, world); rmax = max(rmax, world);
        }
        geometryError = max(largest(abs(lmin-rmin)), largest(abs(lmax-rmax))) > 2e-6;
    } else geometryError = dest != slot;
    latticeResults[slot] = uint4(labelError, centerError, geometryError, poseLattice[pid] < 0);
}
