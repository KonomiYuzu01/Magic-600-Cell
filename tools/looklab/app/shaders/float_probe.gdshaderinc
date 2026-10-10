// Powers of two have exact single-precision representations.
vec4 looklab_float_probe(float offset) {
    return vec4(0.25, 0.5, 0.75, 1.0) + vec4(offset);
}
