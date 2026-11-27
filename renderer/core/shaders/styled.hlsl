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
    uint matrixOn, sliceMode; // ADR-D02 B20: the cut slice (SA2_SLICE_*), its shrink, its shell as cosines, its ghost
    float sliceShrink, shellMinCos;
    float shellMaxCos, slicePad0, slicePad1, slicePad2;
    float4 ghost;
};
static const uint SliceInside = 0x20000u; // the piece style word's bit 17: the slice's inside layer
static const uint Moving = 0x10000u; // bit 16: a moving piece of the transform (B18)
// ADR-D02 B20: a slice draws each sticker with the radius shell in place of the view's focus test, the inside layer
// at the slice's smaller shrink, and only the layers its mode names.
ProductOut sliceVs(uint vi, uint cell, uint style) {
    uint local = stickerIds[vi];
    uint slot = cell * 433 + local;
    bool inside = (style & SliceInside) != 0;
    Projection v = projectVertexShrunk(cell, vi, inside ? sliceShrink : 1.0, false);
    float d = dot(posedPoint(slot, framePoint(cell, centers[local] / radius)), focusPole);
    bool shell = d >= shellMaxCos - 1e-5 && d <= shellMinCos + 1e-5;
    bool layer = inside ? sliceMode != 2 : sliceMode != 3;
    ProductOut result;
    result.position = slotVisible[slot] != 0 && shell && layer ? v.clip : float4(0, 0, -1, 1);
    result.projected = v.projected;
    result.colourClass = labels[slot] / 433;
    result.marked = overlayMode == 3 ? straddles[slotPieces[slot]] : 0;
    result.instance = instanceIds[slot];
    return result;
}
// The band an inside piece keeps: its outline, at least SA2_SLICE_BAND wide.
bool sliceHole(uint style, float3 corner) {
    if (sliceMode == 0 || (style & SliceInside) == 0) return false;
    return min(corner.x, min(corner.y, corner.z)) >= max(outlineWidth, 0.08);
}
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
    uint slot = cell * 433 + stickerIds[vi];
    uint style = pieceStyle[slotPieces[slot]];
    ProductOut p;
    if (sliceMode != 0) p = sliceVs(vi, cell, style); else p = productVs(vi, cell);
    uint k = vi % 3;
    StyledOut result;
    result.position = p.position;
    result.projected = p.projected;
    result.colourClass = p.colourClass;
    result.marked = p.marked;
    result.style = style;
    result.corner = float3(k == 0 ? 1 : 0, k == 1 ? 1 : 0, k == 2 ? 1 : 0);
    result.instance = p.instance;
    // ADR-D02 B17: a hidden piece's stickers fall outside the clip volume, in the colour pass and the ID pass alike.
    bool dimSet = (result.style & 0x80) != 0;
    if ((dimMode == 3 && dimSet) || (dimMode == 4 && !dimSet)) result.position = float4(0, 0, -1, 1);
    return result;
}
float4 styledPs(StyledOut input) : SV_Target {
    if (sliceHole(input.style, input.corner)) discard; // ADR-D02 B20
    VertexOut v;
    v.position = input.position;
    v.projected = input.projected;
    v.colourClass = input.colourClass;
    v.marked = input.marked;
    float shade = stickerShade(v.projected);
    float4 colour = stickerColour(v, shade);
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
    // ADR-D02 B20: the turn preview's ghost on the moving inside pieces.
    if (sliceMode != 0 && ghost.a > 0 && theta != 0 && (input.style & (SliceInside | Moving)) == (SliceInside | Moving))
        colour.rgb = lerp(colour.rgb, ghost.rgb, ghost.a);
    bool dimSet = (fill & 0x80) != 0;
    if ((dimMode == 1 && dimSet) || (dimMode == 2 && !dimSet)) colour.rgb *= 1 - dimAmount;
    // ADR-D01.25: the hatch repeats about every 6 pixels across the diagonal; a regular band keeps its own colour.
    if ((tintMode & 2) != 0 && (outline & 0x10u) != 0 && palette[0].a > 0) {
        bool hatch = frac((input.position.x + input.position.y) / 8.0) < 0.5;
        if (edge < outlineWidth ? (outline & 15) == 0 : hatch)
            colour.rgb = lerp(colour.rgb, palette[0].rgb, palette[0].a);
    }
    if (matrixOn != 0)
        colour.rgb = float3(dot(matrixRows[0].xyz, colour.rgb), dot(matrixRows[1].xyz, colour.rgb), dot(matrixRows[2].xyz, colour.rgb));
    return colour;
}
// ADR-D02 B17 (ADR-D01.46): picking follows visibility. A piece the slot dims answers no instance; a hidden one is not
// drawn at all (styledVs); a tint, an outline or the matrix leaves the instance as the unstyled ID pass gives it.
uint styledIdPs(StyledOut input) : SV_Target {
    if (sliceHole(input.style, input.corner)) discard; // ADR-D02 B20: an inside piece answers only on its band
    bool dimSet = (input.style & 0x80) != 0;
    bool dimmed = dimAmount > 0 && ((dimMode == 1 && dimSet) || (dimMode == 2 && !dimSet));
    return dimmed ? 0 : input.instance;
}
