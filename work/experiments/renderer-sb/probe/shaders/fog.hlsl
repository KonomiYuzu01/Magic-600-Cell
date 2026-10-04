#include "draw.hlsl"
float4 fogMain(VertexOut input) : SV_Target {
    float4 colour = psMain(input);
    float transmittance = exp(-0.23 * max(5 - input.projected.z - 2.2, 0));
    return float4(lerp(float3(0.13, 0.145, 0.16), colour.rgb, transmittance), 1);
}
