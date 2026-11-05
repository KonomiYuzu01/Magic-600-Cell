// Frozen built-in perspective formula from renderer-sb/probe/shaders/geometry.hlsl.
// Only its 4D-to-3D step; the experiment adds the same coverage envelope as charts.
uint4 builtin_perspective(float4 p, float d) {
    if (!all(isfinite(p)) || !isfinite(d)) return missing(2u);
    precise float den = d - p.w;
    if (!isfinite(den)) return missing(2u);
    if (abs(den) <= 0.000244140625f) return missing(1u);
    precise float nx = d * p.x;
    precise float ny = d * p.y;
    precise float nz = d * p.z;
    if (!isfinite(nx) || !isfinite(ny) || !isfinite(nz)) return missing(2u);
    precise float px = nx / den;
    precise float py = ny / den;
    precise float pz = nz / den;
    if (!isfinite(px) || !isfinite(py) || !isfinite(pz)) return missing(2u);
    return uint4(0u, asuint(px), asuint(py), asuint(pz));
}
