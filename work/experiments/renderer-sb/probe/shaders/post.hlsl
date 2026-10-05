Texture2D<float4> sceneColour : register(t0);
Texture2D<float> sceneDepth : register(t1);
Texture2D<float> auxiliary : register(t2);
cbuffer PostConstants : register(b0) {
    float width, height, aspect, zoom;
    float focusDepth, maxRadius;
    float2 blurDirection;
};
float4 fullscreenVS(uint id : SV_VertexID) : SV_Position {
    float2 p = float2((id<<1)&2, id&2);
    return float4(p.x*2-1, 1-p.y*2, 0, 1);
}
int2 pixelAt(float2 p) { return clamp(int2(p), int2(0,0), int2(width,height)-1); }
float viewDepth(float d) { return 0.05 / (1-d/(100.0/99.95)); }
float depthAt(float2 p) { return viewDepth(sceneDepth.Load(int3(pixelAt(p),0))); }
float coc(float z) { return maxRadius * saturate(4*abs(z-focusDepth)/z); }
float4 dofPS(float4 position : SV_Position) : SV_Target {
    float2 p = position.xy;
    float centreRadius = coc(depthAt(p));
    float3 colour = sceneColour.Load(int3(pixelAt(p),0)).rgb;
    float weightSum = 1;
    // Full-resolution 32-tap disc. Nearby blurred samples can spread into the
    // centre, while in-focus samples cannot supply an unrelated blur radius.
    for (uint i=0; i<32; ++i) {
        float r = maxRadius*sqrt((i+0.5)/32.0), angle = i*2.39996323;
        float2 samplePixel = p + r*float2(cos(angle),sin(angle));
        float radius = max(centreRadius,coc(depthAt(samplePixel)));
        float weight = saturate(radius-r+1);
        colour += sceneColour.Load(int3(pixelAt(samplePixel),0)).rgb*weight;
        weightSum += weight;
    }
    return float4(colour/weightSum,1);
}
float3 viewPosition(float2 pixel) {
    float z = depthAt(pixel);
    float2 ndc = float2(2*pixel.x/width-1, 1-2*pixel.y/height);
    return float3(ndc.x*aspect*z/zoom, ndc.y*z/zoom, z);
}
float aoPS(float4 position : SV_Position) : SV_Target {
    float2 pixel = position.xy;
    float3 p = viewPosition(pixel);
    float3 normal = normalize(cross(ddx(p),ddy(p)));
    if (dot(normal,-p)<0) normal = -normal;
    float3 tangent = normalize(cross(normal,abs(normal.z)<0.9 ? float3(0,0,1) : float3(0,1,0)));
    float3 bitangent = cross(normal,tangent);
    float occlusion = 0;
    const float radius = 0.22;
    for (uint i=0; i<16; ++i) {
        float h = (i+0.5)/16.0, phi = i*2.39996323;
        float xy = sqrt(1-h*h);
        float3 direction = tangent*(xy*cos(phi)) + bitangent*(xy*sin(phi)) + normal*h;
        float3 wanted = p + normal*0.007 + direction*radius*(0.25+0.75*h*h);
        float2 samplePixel = float2((wanted.x*zoom/aspect/wanted.z+1)*width*0.5,
                                   (1-wanted.y*zoom/wanted.z)*height*0.5);
        samplePixel = float2(pixelAt(samplePixel))+0.5;
        float3 surface = viewPosition(samplePixel), delta = surface-p;
        float rangeWeight = saturate(1-length(delta)/radius);
        occlusion += (surface.z<wanted.z-0.007 && dot(normal,delta)>0.007) ? rangeWeight : 0;
    }
    // Every pixel executes the 16 samples, including the clear background.
    return p.z>=99.9 ? 1 : saturate(1-2*occlusion/16);
}
float aoBlurPS(float4 position : SV_Position) : SV_Target {
    float2 pixel = position.xy;
    float z = depthAt(pixel), sum = 0, weights = 0;
    for (int i=-4; i<=4; ++i) {
        float2 samplePixel = pixel + blurDirection*i;
        float weight = exp(-float(i*i)/8) * exp(-abs(depthAt(samplePixel)-z)*12);
        sum += auxiliary.Load(int3(pixelAt(samplePixel),0))*weight;
        weights += weight;
    }
    return sum/weights;
}
float4 aoCompositePS(float4 position : SV_Position) : SV_Target {
    int2 pixel = pixelAt(position.xy);
    return float4(sceneColour.Load(int3(pixel,0)).rgb * auxiliary.Load(int3(pixel,0)),1);
}
