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
    float previewAngle, maxAngle;
    uint viewFlags; // ABI 9: bit 0 orthographic, bit 1 the focus test, bit 2 the spread; 0 draws as ABI 8; ABI 10: bit 3 the Hopf rings, bit 4 laid flat as the ring sheet (B22)
    float focusSpread;
    float4 focusPole; // the focus pole in the world, before Q
    float focusCos, pad2, pad3, pad4;
};
#ifdef SA2_LAYOUT
// ABI 10: the Hopf table (product passes only; product.hlsl defines SA2_LAYOUT). See product.h's HopfTable.
StructuredBuffer<float4> hopfRows : register(t18);
#endif
struct Projection {
    float4 clip;
    float3 projected;
    uint slot;
};
// ADR-D02 B20: `factor` scales the sticker shrink (the slice's inside layer); 1 is the shape's own.
float4 homePointShrunk(uint cell, float4 center, float4 vertex, float factor) {
    float4 v = (N0 + cellShrink * (center - N0)
                + cellShrink * (stickerShrink * factor) * (vertex - center)) / radius;
    return frames[cell*4] * v.x + frames[cell*4+1] * v.y
                 + frames[cell*4+2] * v.z + frames[cell*4+3] * v.w;
}
float4 homePoint(uint cell, float4 center, float4 vertex) { return homePointShrunk(cell, center, vertex, 1.0); }
float4 posedPoint(uint slot, float4 world) {
    if (poseEnabled == 0) return world;
    uint pid = poseIndex[slotPieces[slot]];
    return float4(dot(poseRows[pid*4], world), dot(poseRows[pid*4+1], world),
                  dot(poseRows[pid*4+2], world), dot(poseRows[pid*4+3], world));
}
Projection projectWorld(float4 world) {
    world = mul(Q, world);
    float3 p = d4 * world.xyz / (d4 - world.w);
    if ((viewFlags & 1u) != 0) p = world.xyz; // ABI 9: orthographic
    float z = 5 - p.z;
    Projection result;
    result.clip = float4(p.x*zoom/aspect, p.y*zoom, 100.0/(100.0-0.05)*(z-0.05), z);
    result.projected = p;
    result.slot = 0;
    return result;
}
// ABI 9: a cell-frame point (already divided by the radius) in the world, before the pose.
float4 framePoint(uint cell, float4 v) {
    return frames[cell*4] * v.x + frames[cell*4+1] * v.y + frames[cell*4+2] * v.z + frames[cell*4+3] * v.w;
}
// ABI 9's focus (sa2_view_focus_set): a sticker whose posed, unshrunk centre lies outside the focus cap is placed
// outside the clip volume; the spread moves the rest by (spread - 1) times their posed cell centre's projection.
// ADR-D02 B20: without `capTest` (the slice's radius shell replaces it) only the spread applies.
Projection focusVertex(Projection result, uint cell, uint slot, uint local, bool capTest) {
    if (capTest && (viewFlags & 2u) != 0) {
        float4 centre = posedPoint(slot, framePoint(cell, centers[local] / radius));
        if (dot(centre, focusPole) < focusCos) {
            result.clip = float4(0, 0, -1, 1);
            return result;
        }
    }
    if ((viewFlags & 4u) != 0) {
        float4 c = mul(Q, posedPoint(slot, framePoint(cell, N0 / radius)));
        float factor = (viewFlags & 1u) != 0 ? 1.0 : d4 / (d4 - c.w);
        float3 p = result.projected + c.xyz * factor * (focusSpread - 1);
        float z = 5 - p.z;
        result.clip = float4(p.x*zoom/aspect, p.y*zoom, 100.0/(100.0-0.05)*(z-0.05), z);
        result.projected = p;
    }
    return result;
}
#ifdef SA2_LAYOUT
float4 qmul(float4 a, float4 b) {
    return float4(a.x*b.x - a.y*b.y - a.z*b.z - a.w*b.w, a.x*b.y + a.y*b.x + a.z*b.w - a.w*b.z,
                  a.x*b.z - a.y*b.w + a.z*b.x + a.w*b.y, a.x*b.w + a.y*b.z - a.z*b.y + a.w*b.x);
}
// ABI 10 (viewFlags bit 3): the Hopf rings. The chart is the cell nearest the posed centre p of the sticker's own cell
// in Hopf coordinates: the ring whose base point is nearest Im(p J p*), then the place whose phase along that ring is
// nearest. The chart places the posed point; the camera's upper 3 x 3 turns it (the linked spin). With bit 4 (B22) the
// chart is the cell's ring-sheet chart, from row 2611 on, and the camera does not turn it.
Projection hopfVertex(float4 world, uint cell, uint slot) {
    float4 p = normalize(posedPoint(slot, framePoint(cell, N0 / radius)));
    float4 j = hopfRows[2460];
    float3 b = qmul(qmul(p, j), float4(p.x, -p.y, -p.z, -p.w)).yzw;
    uint ring = 0;
    float best = -2;
    for (uint r = 0; r < 20; ++r) {
        float d = dot(b, hopfRows[2440 + r].xyz);
        if (d > best) { best = d; ring = r; }
    }
    float phase = atan2(dot(p, hopfRows[2420 + ring]), dot(p, hopfRows[2400 + ring])) * (30.0 / 3.14159265);
    uint parity = asuint(hopfRows[2440 + ring].w);
    int place = int(round((phase - float(parity)) * 0.5));
    place = ((place % 30) + 30) % 30;
    uint e = ring * 30 + uint(place);
    // B22: t18 is a root SRV, which nothing bounds; a chart index read from anything but the table must not send the
    // next reads past it (a sheet once drew without its table: DXGI_ERROR_DEVICE_HUNG).
    uint chart = min(asuint(hopfRows[2461 + e / 4][e % 4]), 599u);
    bool sheet = (viewFlags & 16u) != 0;
    uint first = (sheet ? 2611u : 0u) + chart * 4;
    float3 placed = float3(dot(hopfRows[first], world), dot(hopfRows[first+1], world), dot(hopfRows[first+2], world))
                  + hopfRows[first+3].xyz;
    float3 q = sheet ? placed : mul(Q, float4(placed, 0)).xyz;
    float z = 5 - q.z;
    Projection result;
    result.clip = float4(q.x*zoom/aspect, q.y*zoom, 100.0/(100.0-0.05)*(z-0.05), z);
    result.projected = q;
    result.slot = slot;
    return result;
}
#endif
Projection projectVertexShrunk(uint cell, uint vi, float factor, bool capTest) {
    uint local = stickerIds[vi];
    uint slot = cell * 433 + local;
    float4 world = posedPoint(slot, homePointShrunk(cell, centers[local], vertices[vi], factor));
#ifdef SA2_LAYOUT
    // ADR-D02 B18: in the product passes a turning frame binds the piece style at t4, whose bit 16 marks a moving piece.
    const bool turns = theta != 0 && (animated[slotPieces[slot]] & 0x10000u) != 0;
#else
    const bool turns = theta != 0 && animated[slot] != 0;
#endif
    if (turns) {
        float x = dot(world, turnU), y = dot(world, turnV);
        float c = cos(theta), s = sin(theta);
        world += ((c-1)*x-s*y)*turnU + (s*x+(c-1)*y)*turnV;
    }
#ifdef SA2_LAYOUT
    if ((viewFlags & 8u) != 0) return hopfVertex(world, cell, slot);
#endif
    Projection result = projectWorld(world);
    if (viewFlags > 1u) result = focusVertex(result, cell, slot, local, capTest);
    result.slot = slot;
    return result;
}
Projection projectVertex(uint cell, uint vi) { return projectVertexShrunk(cell, vi, 1.0, true); }
