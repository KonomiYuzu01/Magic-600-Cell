"""Deterministic HLSL from admitted chart definitions; no host path in output."""
import sys
sys.dont_write_bytecode = True
from pathlib import Path
from subset import admit, bits

HERE = Path(__file__).resolve().parent
CHARTS = ("perspective", "stereographic", "gnomonic")
STAGES = ("builtin_perspective",) + CHARTS


def function(name, source):
    program = admit(source)  # Admission always precedes emitting any shader bytes.

    def operand(value):
        if not isinstance(value, str):
            return f"asfloat(0x{bits(value):08x}u)"
        return "p." + value if value in ("x", "y", "z", "w") else value

    lines = [f"uint4 chart_{name}(float4 p, float d) {{",
             "    if (!all(isfinite(p)) || !isfinite(d)) return missing(2u);"]
    for dest, op, args in program.instructions:
        values = [operand(arg) for arg in args]
        a = values[0]
        if op == "exclude":
            lines.append(f"    if (abs({a}) <= {values[1]}) return missing(1u);")
            continue
        if op in ("neg", "abs"):
            expression = f"(-{a})" if op == "neg" else f"abs({a})"
        else:
            symbol = {"add": "+", "sub": "-", "mul": "*", "div": "/"}[op]
            if op == "div":
                lines.append(f"    if ({values[1]} == 0.0f) return missing(2u);")
            expression = f"{a} {symbol} {values[1]}"
        lines.extend((f"    precise float {dest} = {expression};",
                      f"    if (!isfinite({dest})) return missing(2u);"))
    out = ", ".join(f"asuint({operand(arg)})" for arg in program.outputs)
    lines.extend((f"    return uint4(0u, {out});", "}"))
    return "\n".join(lines) + "\n"


def generate():
    definitions = [function(name, (HERE / "charts" / (name + ".chart")).read_text(encoding="utf-8"))
                   for name in CHARTS]
    header = """// LX-7 experiment, generated deterministically by generate_hlsl.py.
// SM 6.2, strict IEEE operations, denormals preserved; no vendor features.
struct Input { float4 vertex; float4 q0; float4 q1; float4 q2; float4 q3;
               float d; uint stage; uint pad0; uint pad1; };
StructuredBuffer<Input> inputs : register(t0);
RWStructuredBuffer<uint4> outputs : register(u0);
cbuffer Count : register(b0) { uint count; };
uint4 missing(uint reason) { return uint4(reason, 0u, 0u, 0u); }
float row(float4 q, float4 v) {
    precise float a = q.x * v.x;
    precise float b = q.y * v.y;
    precise float c = q.z * v.z;
    precise float d = q.w * v.w;
    precise float ab = a + b;
    precise float abc = ab + c;
    precise float abcd = abc + d;
    return abcd;
}
"""
    dispatch = """
[numthreads(64, 1, 1)]
void main(uint3 tid : SV_DispatchThreadID) {
    if (tid.x >= count) return;
    Input v = inputs[tid.x];
    precise float4 p = float4(row(v.q0, v.vertex), row(v.q1, v.vertex),
                            row(v.q2, v.vertex), row(v.q3, v.vertex));
    uint4 result = missing(3u); // Invalid stage is a harness error, never a point.
    switch (v.stage) {
    case 0u: result = builtin_perspective(p, v.d); break;
    case 1u: result = chart_perspective(p, v.d); break;
    case 2u: result = chart_stereographic(p, v.d); break;
    case 3u: result = chart_gnomonic(p, v.d); break;
    }
    outputs[tid.x] = result;
}
"""
    return (header + (HERE / "builtin.hlsl").read_text(encoding="utf-8") +
            "\n" + "\n".join(definitions) + dispatch).encode("utf-8")


if __name__ == "__main__":
    if len(sys.argv) != 1:
        raise SystemExit("Usage: python generate_hlsl.py (writes generated.hlsl)")
    (HERE / "generated.hlsl").write_bytes(generate())
