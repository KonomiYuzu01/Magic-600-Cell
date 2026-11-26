// ABI 7 (ADR-D02 build step B14, a candidate): the styled product pass. Its vertex stage is product.hlsl's productVs
// with the piece's style and the corner of its triangle (the draw is a non-indexed triangle list, so the corner is
// SV_VertexID % 3); its fragment stage is draw.hlsl's psMain, then the tint, the outline, the dim and the output
// colour matrix of the frame's style slot. The core draws with it only when the style changes something, so frames
// without a style keep the unchanged product pipeline. The DXIL is embedded in sa2_interop.dll at build time.
// ADR-D02 B17 (ADR-D01.46, ABI 10): dimMode 3 and 4 hide the pieces that 1 and 2 dim (styledVs places their stickers
// outside the clip volume), and styledIdPs is the ID pass of a styled frame: a dimmed piece answers no instance.
#include "product.hlsl"
StructuredBuffer<uint> pieceStyle : register(t17); // per piece: the fill byte, then the outline byte << 8
cbuffer ViewStyle : register(b1) {
    float4 palette[16];
    float4 matrixRows[3]; // xyz: a row of the output colour matrix
    uint tintMode, dimMode;
    float dimAmount, outlineWidth;
    uint matrixOn;
};
struct StyledOut {
    float4 position : SV_Position;
    float3 projected : TEXCOORD0;
    nointerpolation uint colourClass : TEXCOORD1;
    nointerpolation uint marked : TEXCOORD2;
    nointerpolation uint style : TEXCOORD3;
    float3 corner : TEXCOORD4;
    nointerpolation uint instance : TEXCOORD5; // ADR-D02 B17: the ID pass's value
};
StyledOut styledVs(uint vi : SV_VertexID, uint cell : SV_InstanceID) {
    ProductOut p = productVs(vi, cell);
    uint slot = cell * 433 + stickerIds[vi];
    uint k = vi % 3;
    StyledOut result;
    result.position = p.position;
    result.projected = p.projected;
    result.colourClass = p.colourClass;
    result.marked = p.marked;
    result.style = pieceStyle[slotPieces[slot]];
    result.corner = float3(k == 0 ? 1 : 0, k == 1 ? 1 : 0, k == 2 ? 1 : 0);
    result.instance = p.instance;
    // ADR-D02 B17: a hidden piece's stickers fall outside the clip volume, in the colour pass and the ID pass alike.
    bool dimSet = (result.style & 0x80) != 0;
    if ((dimMode == 3 && dimSet) || (dimMode == 4 && !dimSet)) result.position = float4(0, 0, -1, 1);
    return result;
}
float4 styledPs(StyledOut input) : SV_Target {
    VertexOut v;
    v.position = input.position;
    v.projected = input.projected;
    v.colourClass = input.colourClass;
    v.marked = input.marked;
    float4 colour = psMain(v);
    float3 n = normalize(cross(ddx(input.projected), ddy(input.projected)));
    float shade = 0.5 + 0.5*abs(dot(n, normalize(float3(0.3,0.5,1))));
    uint fill = input.style & 0xFF, outline = (input.style >> 8) & 0xFF;
    if ((tintMode & 1) != 0 && (fill & 15) != 0) {
        float4 tint = palette[fill & 15];
        colour.rgb = lerp(colour.rgb, tint.rgb * shade, tint.a);
    }
    float edge = min(input.corner.x, min(input.corner.y, input.corner.z));
    if ((tintMode & 2) != 0 && (outline & 15) != 0 && edge < outlineWidth) {
        float4 stroke = palette[outline & 15];
        colour.rgb = lerp(colour.rgb, stroke.rgb, stroke.a);
    }
    bool dimSet = (fill & 0x80) != 0;
    if ((dimMode == 1 && dimSet) || (dimMode == 2 && !dimSet)) colour.rgb *= 1 - dimAmount;
    if (matrixOn != 0)
        colour.rgb = float3(dot(matrixRows[0].xyz, colour.rgb), dot(matrixRows[1].xyz, colour.rgb), dot(matrixRows[2].xyz, colour.rgb));
    return colour;
}
// ADR-D02 B17 (ADR-D01.46): picking follows visibility. A piece the slot dims answers no instance; a hidden one is not
// drawn at all (styledVs); a tint, an outline or the matrix leaves the instance as the unstyled ID pass gives it.
uint styledIdPs(StyledOut input) : SV_Target {
    bool dimSet = (input.style & 0x80) != 0;
    bool dimmed = dimAmount > 0 && ((dimMode == 1 && dimSet) || (dimMode == 2 && !dimSet));
    return dimmed ? 0 : input.instance;
}
