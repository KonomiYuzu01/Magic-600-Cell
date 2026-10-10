#include "geometry.hlsl"
struct Out {float4 position : SV_Position; float4 colour : COLOR0;};
Out overlayVS(uint vi : SV_VertexID, uint instance : SV_InstanceID) {
    const float2 corners[6] = {float2(-1,-1),float2(1,-1),float2(-1,1),
                               float2(-1,1),float2(1,-1),float2(1,1)};
    float2 corner = corners[vi]; Out result;
    if (overlayMode == 4) {
        // A radial preview gauge spans the fixture's full twist angle, with its
        // current smoothstep angle marked in yellow. No legality is inferred.
        float phi = float(instance) / 63 * maxAngle;
        float2 direction = float2(cos(phi),sin(phi));
        float2 p = float2(.78,-.76) + .14*direction + corner*.002;
        result.position = float4(p,0,1);
        result.colour = phi <= previewAngle ? float4(1,.85,.15,1) : float4(.35,.35,.35,1);
    } else {
        OverlayPoint marker = overlayPoints[instance];
        result.position = projectWorld(marker.world).clip;
        result.position.xy += corner * float2(.003/aspect,.003) * result.position.w;
        result.colour = marker.colour;
    }
    return result;
}
float4 overlayPS(Out input) : SV_Target {return input.colour;}
