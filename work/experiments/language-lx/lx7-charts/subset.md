# LX-7 subset candidate r1

Experiment only; decision **open**, decider **owner**. This is an admission and
translation candidate for RPI.32/RPI.33, not a proposed product syntax or a security
sandbox for general Python. The source is parsed with `ast`; it is never executed.

Inputs are five binary32 scalars `x, y, z, w, d`. Statements are local assignment,
`exclude(expression, positive_literal)`, a final `output(x, y, z)`, and
`for _ in range(N)` with literal integer N from 1 through 8. Repetition is unrolled
at admission; `_` cannot be read. Local assignments can update earlier locals.
Expressions are names, literals, parentheses, unary minus, `abs`, and binary
`+ - * /`. All evaluation is left-to-right, postorder, without reassociation or
fused multiply-add. Inputs cannot be assigned. Output is allowed only at top level,
once, as the last statement. No functions, recursion, collections, allocation,
attributes, imports, file/engine calls, dynamic loops or big-integer chart values.
The Python host and independent rational oracle may allocate and use big integers;
the admitted chart has only its bounded scalar register set.

Admission limits: 4,096 UTF-8 source bytes, 512 AST nodes, expression depth 16,
loop depth 4, 256 expanded arithmetic/exclusion instructions. Every numeric literal
must denote an exact finite binary32 value, checked against its decimal rational
spelling. `0.5` and `16777216` fit; `0.1`, `16777217`, and `1e100` do not. There is
no implicit float64 literal conversion. `d = 59/50` is fixture metadata converted
once at the input boundary, not an admitted literal. All operations thereafter use
binary32. Loop bounds are checker metadata, never arbitrary-precision runtime data.

The CPU interpreter uses `struct.pack/unpack('<f')` after every arithmetic
operation; round-to-nearest, ties-to-even, gradual underflow, signed zero. The
binary64 host can exactly hold a binary32 product and supplies the intermediate
division before that rounding. The check independently evaluates fixture operations
with `fractions` and integer ties-to-even rounding; it also tests cancellation,
tie, subnormal and overflow cases. The camera conversion/rotation happens before
the chart, with separately rounded products and the sum `(((a+b)+c)+d)`.

Coverage is total: code 0 is a finite point; 1 is `singular_neighbourhood` when
`abs(denominator) <= radius`; 2 is `nonfinite` for nonfinite inputs, overflow or
division by zero. Evaluation stops at the first such event. Missing coverage
contains **no point**; the wire format reserves three zero words, which are never
drawn or compared as coordinates. Exact property, state and hit-test decisions
cannot use these floats. Mesh triangles crossing an exclusion are not handled in
this compute experiment; the product handling remains open with RPI.36.

The frozen experiment radius is 1/4096 in denominator-coordinate units, inclusive,
for all three maps. Perspective excludes `|d-w| <= 1/4096`, stereographic excludes
`|1-w| <= 1/4096`, and gnomonic excludes `|w| <= 1/4096`. These are experimental
neighbourhoods, not an accepted product exclusion policy. Exact classification
uses rationals, while executable classification uses rounded inputs/operations;
the report lists every disagreement. The underlying mathematical poles are d, 1
and 0 respectively. Charts report missing coverage on both sides of a pole.

The built-in stage freezes only the 4D-to-3D perspective formula from
`work/experiments/renderer-sb/probe/shaders/geometry.hlsl`, at exact d=59/50, plus
the experiment coverage envelope. It excludes shrink, frames, animation, camera3,
clip-space and rasterization. The stereographic formula is `xyz/(1-w)`; interpreting
it as stereographic on a sphere needs a sphere-domain proof not made here. Gnomonic
uses the projective affine coordinates `xyz/w`. No inverse, injectivity, section
membership, or region-property proof is claimed for any chart.

HLSL has scalar `precise` intermediates and no vendor features. Its frozen compiler
profile is `-T cs_6_2 -E main -HV 2021 -Gis -denorm preserve -O3 -Ges -WX`. Shader
model 6.2 is required for the declared denormal mode. This requests the float32
semantics; device comparison measures implementation differences. A lower shader
model or silent fast-math fallback is not the same candidate.

Refusal codes are experiment-local, not additions to chapter 13: `E_SYNTAX`,
`E_BOUNDS`, `E_RECURSION`, `E_UNBOUNDED_LOOP`, `E_LITERAL`, `E_NAME`, `E_OPERATION`,
`E_CALL`, `E_OUTPUT`. Rejected input produces no HLSL or device work. A passed
subset/admission check is not permission to enable a user chart: precision judgement
and LQ7 remain open. Absolute errors (projected units), relative errors excluding
exact zero, and binary32 ULP distances to the CPU are raw observations only.
