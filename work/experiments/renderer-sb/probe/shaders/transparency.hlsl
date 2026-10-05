#include "draw.hlsl"
VertexOut transparencyVS(uint g : SV_VertexID) {
    return vsMain(g%30480, g/30480);
}
float4 transparencyPS(VertexOut input) : SV_Target {
    return float4(psMain(input).rgb, 0.6);
}
