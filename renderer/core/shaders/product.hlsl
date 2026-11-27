// ABI 4's product passes. The frame and its ID pass share one vertex stage
// (RPI.24, RPI.75): draw.hlsl's vsMain, then the preparation's per-slot
// visibility. A slot that is not drawn is placed outside the clip volume, so
// it enters neither pass (F02). The fragment stages have no discard and change
// no depth, stencil or coverage.
#define SA2_LAYOUT 1 // ABI 10: the product passes read the Hopf table (t18)
#include "draw.hlsl"
StructuredBuffer<uint> slotVisible : register(t15); // 1 = drawn
StructuredBuffer<uint> instanceIds : register(t16); // the ID pass value; 0 = no instance
StructuredBuffer<float4> stickerColours : register(t19); // a = 0 uses the built-in hue
struct ProductOut {
    float4 position : SV_Position;
    float3 projected : TEXCOORD0;
    nointerpolation uint colourClass : TEXCOORD1;
    nointerpolation uint marked : TEXCOORD2;
    nointerpolation uint instance : TEXCOORD3;
};
ProductOut productVs(uint vi : SV_VertexID, uint cell : SV_InstanceID) {
    VertexOut v = vsMain(vi, cell);
    uint slot = cell * 433 + stickerIds[vi];
    ProductOut result;
    result.position = slotVisible[slot] != 0 ? v.position : float4(0, 0, -1, 1);
    result.projected = v.projected;
    result.colourClass = v.colourClass;
    result.marked = v.marked;
    result.instance = instanceIds[slot];
    return result;
}
float stickerShade(float3 projected) {
    float3 n = normalize(cross(ddx(projected), ddy(projected)));
    return 0.5 + 0.5*abs(dot(n, normalize(float3(0.3,0.5,1))));
}
float4 stickerColour(VertexOut input, float shade) {
    float4 colour = psMain(input);
    if (input.marked == 0) {
        float4 entry = stickerColours[input.colourClass];
        if (entry.a > 0) colour.rgb = entry.rgb * shade;
    }
    return colour;
}
float4 productPs(ProductOut input) : SV_Target {
    VertexOut v;
    v.position = input.position;
    v.projected = input.projected;
    v.colourClass = input.colourClass;
    v.marked = input.marked;
    return stickerColour(v, stickerShade(v.projected));
}
uint idPs(ProductOut input) : SV_Target {
    return input.instance;
}
