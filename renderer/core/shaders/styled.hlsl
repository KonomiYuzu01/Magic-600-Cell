// ABI 7 (ADR-D02 build step B14, a candidate): the styled product pass. Its vertex stage is product.hlsl's productVs
// with the piece's style and the corner of its triangle (the draw is a non-indexed triangle list, so the corner is
// SV_VertexID % 3); its fragment stage is draw.hlsl's psMain, then the tint, the outline, the dim and the output
// colour matrix of the frame's style slot. The core draws with it only when the style changes something, so frames
// without a style keep the unchanged product pipeline. The DXIL is embedded in sa2_interop.dll at build time.
// ADR-D02 B17 (ADR-D01.46, ABI 10): dimMode 3 and 4 hide the pieces that 1 and 2 dim (styledVs places their stickers
// outside the clip volume), and styledIdPs is the ID pass of a styled frame: a dimmed piece answers no instance.
#include "product.hlsl"
StructuredBuffer<uint> pieceStyle : register(t17); // fill, outline, moving/slice, then the status byte << 24
RWByteAddressBuffer statusDrawn : register(u0); // optional product-script capture
struct StatusPattern {
    uint shape;
    float spacing, width, angle;
    float4 colour;
};
cbuffer ViewStyle : register(b1) {
    float4 palette[16];
    float4 matrixRows[3]; // xyz: a row of the output colour matrix
    uint tintMode, dimMode;
    float dimAmount, outlineWidth;
    uint matrixOn, sliceMode; // ADR-D02 B20: the cut slice (SA2_SLICE_*), its shrink, its shell as cosines, its ghost
    float sliceShrink, shellMinCos;
    float shellMaxCos;
    uint statusOn, slicePad1, slicePad2;
    float4 ghost;
    StatusPattern patterns[8];
    StatusPattern marks[8];
};
static const uint SliceInside = 0x20000u; // the piece style word's bit 17: the slice's inside layer
static const uint Moving = 0x10000u; // bit 16: a moving piece of the transform (B18)
static const uint ReferenceLine = 0x40000u;
static const uint AlwaysOutline = 0x80000u;
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
    nointerpolation uint piece : TEXCOORD6;
    float cutDistance : TEXCOORD7;
};
bool statusConflict(uint style) {
    return statusOn != 0 && ((style >> 24) & 7u) == 2u;
}
bool hides(uint style) {
    bool outside = (style & 0x80u) != 0;
    return !statusConflict(style) && ((dimMode == 3 && outside) || (dimMode == 4 && !outside));
}

bool jumbleOptions(float4 entry) {
    return entry.z == 0.5 && asuint(entry.a) == 0x80000000u;
}

// The shell tags one unused palette entry: class 1 during height tint, height 2 otherwise.
// It carries the cap id (+1, in units of 1/1024) and which pane's focus bit to read. Ordinary style slots stay untagged.
uint jumbleStyle(inout uint style, out float capCode) {
    bool heightMode = jumbleOptions(palette[3]);
    float4 options = heightMode ? palette[3] : palette[10];
    capCode = 0;
    if (!jumbleOptions(options)) return 0;
    capCode = options.x;
    uint fill = style & 15u, outline = (style >> 8) & 15u;
    uint ink = heightMode ? outline >> 2 : fill & 3u;
    bool outside = (style & (options.y == 0 ? 0x80u : 8u)) != 0;
    uint height = fill & 7u;
    fill = heightMode && height != 0 ? height + 9u : 0;
    if (heightMode) outline = (outline & 3u) == 3u ? 15u : outline & 3u;
    style = (style & ~0x0f8fu) | (outline << 8) | fill | (outside ? 0x80u : 0);
    if (ink >= 2) style |= ReferenceLine;
    if (ink == 2) style |= AlwaysOutline;
    return ink;
}

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
    float capCode;
    uint ink = jumbleStyle(style, capCode);
    result.style = style;
    result.corner = float3(k == 0 ? 1 : 0, k == 1 ? 1 : 0, k == 2 ? 1 : 0);
    result.instance = p.instance;
    result.piece = slotPieces[slot];
    result.cutDistance = 1;
    if (capCode >= 1.0 / 1024 && capCode <= 600.0 / 1024 && ink != 0) {
        uint cap = (uint)(capCode * 1024) - 1;
        float4 pole = normalize(framePoint(cap, N0));
        // Measure against the unshrunk sticker face; draw the stripe on that face's rendered triangle.
        float4 world = posedPoint(slot, framePoint(cell, vertices[vi] / radius));
        result.cutDistance = dot(world, pole) - 121.0 / 125.0;
    }
    if (hides(style) && (style & ReferenceLine) == 0) result.position = float4(0, 0, -1, 1);
    return result;
}
float2 patternRotate(float2 p, float angle) {
    float s, c;
    sincos(angle, s, c);
    return float2(p.x * c - p.y * s, p.x * s + p.y * c);
}
bool patternInk(StatusPattern entry, float2 pixel, float edge) {
    if (entry.shape == 0) return false;
    float2 grid = frac(patternRotate(pixel, entry.angle) / entry.spacing) - 0.5;
    float2 distance = abs(grid);
    float w = entry.width * 0.5;
    if (entry.shape == 1) return distance.x < w;                           // hatch
    if (entry.shape == 2) return min(distance.x, distance.y) < w;               // cross-hatch
    if (entry.shape == 3) return length(grid) < w;                     // dot screen
    if (entry.shape == 4) return edge < 0.16 && distance.x < w;             // dashed edge
    return distance.x < w || abs(distance.x - 0.28) < w * 0.3;                // paired stripes
}
bool markInk(StatusPattern entry, float3 corner) {
    if (entry.shape == 0) return false;
    // An equilateral triangle in barycentric coordinates, centred at its centroid.
    float2 p = patternRotate(float2(corner.y - corner.z, (3 * corner.x - 1) * 0.577350269), entry.angle);
    float r = entry.spacing, w = r * entry.width;
    if (entry.shape == 1) return abs(length(p) - r) < w;               // ring
    if (entry.shape == 2) return length(p) < r;                        // dot
    if (entry.shape == 3) return max(abs(p.x), abs(p.y)) < r && min(abs(p.x), abs(p.y)) < w;
    // Three short ticks toward the corners, separated from the triangle edges.
    if (length(p) < r * 0.65 || length(p) >= r * 1.5) return false;
    return (abs(p.x) < w && p.y > 0) || abs(abs(p.x) + 1.732050808 * p.y) < w;
}
float4 styledPs(StyledOut input) : SV_Target {
    float edge = min(input.corner.x, min(input.corner.y, input.corner.z));
    float cutWidth = min(fwidth(input.cutDistance), 0.002);
    bool reference = (input.style & ReferenceLine) != 0;
    bool referenceOnly = reference && hides(input.style);
    if (referenceOnly && edge >= outlineWidth) discard;
    uint status = statusOn != 0 ? (input.style >> 24) & 0x3fu : 0;
    uint markIndex = status >> 3;
    bool marked = markIndex != 0 && markInk(marks[markIndex], input.corner);
    if (sliceHole(input.style, input.corner) && !marked) discard; // a protection mark can occupy the centre of a cut band
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
    if ((tintMode & 2) != 0 && (outline & 15) != 0 && edge < outlineWidth) {
        float4 stroke = palette[outline & 15];
        colour.rgb = lerp(colour.rgb, stroke.rgb, stroke.a);
    }
    // Keep the tint in the outer half of the band. The overlay has a separate inner stroke.
    bool drawLine = (input.style & AlwaysOutline) != 0 || referenceOnly;
    if (drawLine && edge < outlineWidth && ((outline & 15) == 0 || edge >= outlineWidth * 0.5)) {
        colour.rgb = referenceOnly ? palette[15].rgb : lerp(colour.rgb, palette[15].rgb, palette[15].a);
    }
    if (!referenceOnly && cutWidth > 0 && abs(input.cutDistance) <= cutWidth)
        colour.rgb = lerp(colour.rgb, palette[15].rgb, palette[15].a);
    // ADR-D02 B20: the turn preview's ghost on the moving inside pieces.
    if (sliceMode != 0 && ghost.a > 0 && theta != 0 && (input.style & (SliceInside | Moving)) == (SliceInside | Moving))
        colour.rgb = lerp(colour.rgb, ghost.rgb, ghost.a);
    bool dimSet = (fill & 0x80) != 0;
    if (!reference && !statusConflict(input.style) && ((dimMode == 1 && dimSet) || (dimMode == 2 && !dimSet))) colour.rgb *= 1 - dimAmount;
    // ADR-D01.25: the hatch repeats about every 6 pixels across the diagonal; a regular band keeps its own colour.
    if ((tintMode & 2) != 0 && (outline & 0x10u) != 0 && palette[0].a > 0) {
        bool hatch = frac((input.position.x + input.position.y) / 8.0) < 0.5;
        if (edge < outlineWidth ? (outline & 15) == 0 : hatch)
            colour.rgb = lerp(colour.rgb, palette[0].rgb, palette[0].a);
    }
    // ADR-D01.65: host statuses after the user stages, before the matrix; a conflict is never dimmed.
    if (status != 0) {
        if ((statusOn & 2u) != 0) statusDrawn.InterlockedOr((input.piece >> 5) * 4, 1u << (input.piece & 31u));
        StatusPattern pattern = patterns[status & 7u];
        if (patternInk(pattern, input.position.xy, edge)) colour.rgb = lerp(colour.rgb, pattern.colour.rgb, pattern.colour.a);
        if (marked) {
            float4 markColour = marks[markIndex].colour;
            colour.rgb = lerp(colour.rgb, markColour.rgb, markColour.a);
        }
    }
    if (matrixOn != 0)
        colour.rgb = float3(dot(matrixRows[0].xyz, colour.rgb), dot(matrixRows[1].xyz, colour.rgb), dot(matrixRows[2].xyz, colour.rgb));
    return colour;
}
// ADR-D02 B17 (ADR-D01.46): picking follows visibility. A piece the slot dims answers no instance; a hidden one is not
// drawn at all (styledVs); a tint, an outline or the matrix leaves the instance as the unstyled ID pass gives it.
uint styledIdPs(StyledOut input) : SV_Target {
    if (hides(input.style)) discard; // a kept reference outline never answers a pick
    if (sliceHole(input.style, input.corner)) discard; // ADR-D02 B20: an inside piece answers only on its band
    bool dimSet = (input.style & 0x80) != 0;
    bool dimmed = dimAmount > 0 && ((dimMode == 1 && dimSet) || (dimMode == 2 && !dimSet));
    return dimmed ? 0 : input.instance;
}
