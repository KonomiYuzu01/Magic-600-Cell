# LX-9: input-map determinism (E-2.4-L9)

Experiment only. Decision: **open**; decider: owner. No route, engine language,
syntax, LQ answer or state-contract amendment is selected.

Authority: `docs/wiki/decisions/owner-decisions-2026-11-05.md` and the LX-9
implementation packet. Time box: **what can be done now; one wrapper call of at
most two hours**. A reached limit is no pass. Evidence class: **synthetic**.

## Frozen card (chapter 11, RTE.64 to RTE.66)

| Field | Card |
| --- | --- |
| Question | Do fresh A1 input requests and menu-mode drag choices select the same exact rotation across the reference and candidate runtimes? |
| Decision it feeds | INV-10 conformance; EN1's Astra check and owner decision on any state-contract amendment |
| Hypothesis | R2-E2 found non-correctly-rounded trigonometric targets on its synthetic corpus; it did not establish different chosen rotations |
| Method | Freeze requests and exact bisectors by digest; compare target bits on Windows CPython, Linux CPython and the LX-6 engine candidate, then exact chosen-rotation keys near the bisectors |
| Time box | Owner sets it before the run |
| Kill criteria | Any different chosen rotation stops an unqualified cross-runtime claim and triggers the amendment packet; target-bit differences alone continue to step 2; timeout leaves untested requests open |
| Evidence class | synthetic, on both platforms and the named candidate; no native-input or performance result |
| Cannot show | Equality for every request from a finite sample, input latency, or an approved change to state contract A1 |
| Product left behind | Frozen target-bit and bisector fixtures, exact-choice comparisons and, if triggered, the proposed amendment packet |

The packet supplies the time box above. A differing rotation is an experimental
result, not a failed harness. RTE.66 distinguishes fresh requests from replay of
an already recorded exact rotation. This experiment performs no state replay.

## Construction fixed before measuring step 1

R2-E2 (`docs/progress/1.0/design-table/topics/language-ecosystem.md:581`) states
3,600 angles every 0.1 degree and 10,000 random angles, but supplies **no seed,
angle endpoints, random generator, axes, raw input bytes or digest**. Therefore
the original 13,600 bytes cannot be reconstructed from that record. This is a
new frozen corpus of the stated shape, not a claimed replay of those exact
historical requests or of its 484/436 incorrectly-rounded counts.

Step 1 uses `i / 10` for `i=0..3599`, then `Random(0x4c5839).random() * 360`
10,000 times. Axes cycle through `(1,0,0)`, `(0,1,0)`, `(0,0,1)`, `(1,2,3)`,
`(0.3,-0.2,0.9)`, `(1,1,1)`, `(-1,2,-3)`, `(1,-1,0.25)` across all requests.
All requests record binary64 bits, N=1 and D=4. These are explicit small A1
bounds, not the product defaults N=16, D=1000. All 13,600 receive exact choices
as well as target comparisons, allowing later Linux differences to be retained.

Step 2 starts with 1,216 distinct certified near-bisector requests:

* 1,024 half-turn requests: 128 rational values `z=(2*i-127)/1024`, four signed
  coordinate permutations, and the two binary64 numbers bracketing
  `t=sqrt(2)-1`. The ideal axis is `(1,t,z)`, angle 180 degrees, N=1, D=4.
  The half-turn candidates `(1,0,0)` and `(1,1,0)` tie exactly:
  `(1+t)^2/2=1`. `t` is the positive root of `t^2+2*t-1=0`.
  Fraction arithmetic in Q(sqrt(2)) checks **every** numerator triple and
  proves these are global maxima for each z. Cayley candidates have the same
  dot numerator and a strictly larger squared norm at the ideal half-turn.
  The perturbed axis component lies within one ulp of this exact bisector;
  this is a request-coordinate statement, not a claim about rounded targets.
* 192 axial Cayley requests: N=1, D=16, all 16 adjacent pairs along a positive
  coordinate axis, three coordinate axes, two signs, and two enclosing degree
  values. The ordered rotations are identity, then `2*atan(1/d)` for d=16..1.
  Their angular bisector is `atan(1/d)+atan(1/e)`, with the identity term zero.
  Its tangent-half-angle quadratic follows from equality of squared quaternion
  dot products; integer coefficients are retained. Rational alternating-series
  bounds for atan and Machin's identity for pi enclose the degree value.
  Fraction checks prove that adjacent binary64 values bracket the enclosure,
  hence both are within one degree-coordinate ulp of the exact boundary.
  The exactly 45-degree boundary uses 45 itself and its next greater float.
  Off-axis numerator components only increase squared norm, so adjacent axial
  candidates are globally nearest. No floating inverse trig constructs a bound.

After step 1, **every** request whose target bits or exact rotation differ
between any primary Windows pair is appended unchanged, ordered by ID and
deduplicated. Such carried requests are not claimed to be near a bisector.
`MANIFEST.json` freezes both stages and the inclusion IDs; acceptance regenerates
and checks the bytes. A later Linux leg extends the inclusion list by the same
rule and uses the already computed Windows step-1 choices for those requests.

## Legs, controls and limits

The reference imports the unchanged `_input_quaternion` and `nearest_parameter`
from `research/jumbling/sim/twists.py`, including its 1e-12 screening and its
within-one-degree half-turn branch. It does not initialize a puzzle or session.
The pure port replaces screening with exhaustive exact integer scoring and
keeps the reference's exact tie order. Its target norm uses explicit sequential
FMA, controlled bit-for-bit against this NumPy build on **every** request in
the same interpreter. The standalone Windows pure leg then isolates interpreter
effects. A separate unfused Python control quantifies ordinary loop summation.

The Rust candidate uses standard-library f64 trigonometry, explicit ordinary
multiply/add norms, and an independent arbitrary-width unsigned-limb scorer.
Its exact keys are checked against Python integer scoring on the Rust target
bits. A Rust FMA-norm control separates summation from the rest of its float
path. This is a standalone LX-9 float candidate using the packet's LX-6 Rust
toolchain; it is not a claim that an unavailable LX-6 implementation was run.

Keys are primitive integer quaternions modulo common scaling and sign, uniquely
identifying the exact rotation in the common cap frame. Raw parameter ties are
also compared, so equivalent encodings cannot hide a tie-order error.

This packet covers cap-frame A1 requests. The proposed ENG-0J menu-mode drag
chooser (60 I_a candidates, fixed metric, pinned index ties), world-frame input,
product-default bounds, Linux execution and native input are not established by
these runs. `twists.py` implements exact menu membership, not that proposed drag
chooser. No menu comparison or contract amendment is silently substituted.

The implementation packet fixes scope and acceptance. Nested review-wrapper
calls would require network access and writes outside this folder and are not
run; integrator review remains outside this implementation call. No commit is
made. `check.py` removes its build folder in `finally`, on success or failure.
