# LX-7: generated charts

| Field | Frozen experiment card |
| --- | --- |
| LX card | LX-7, chapter 11 RTE.59 to RTE.61 |
| Experiment ID | E-2.4-L7, candidate lx7-r1 |
| Owner allowance | `docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2; packet source/synthetic implementation |
| Time box | what can be done now; one wrapper call of at most two hours |
| Question | Does a generated projection stage agree with its declared reference precision and meet the renderer selection gate? |
| Hypothesis | Per-frame vertex work requires GPU evaluation; this experiment supplies no generated-stage timing |
| Method now | Freeze subset, three charts, independent built-in perspective formula, exact rational vertices/poses, exclusions and float semantics; check CPU/reference/generation; compile HLSL and prepare named-device readback |
| Inputs | 24 seeded base-vertex indices plus axis extrema, lifted exactly from binary32 and scaled by rational 1/8; synthetic pole/boundary/subnormal/overflow extremes; identity and central inversion poses; exact d=59/50 |
| Frozen identities | `manifest.json`: per-file and aggregate SHA-256 for candidate, inputs, synthetic outputs and upstream sources; device input binary digest |
| Precision | float32 per operation, ties-to-even; `subset.md`; raw absolute, relative and CPU ULP distributions, no admission threshold |
| Singular exclusions | Inclusive denominator radius 1/4096, declared per chart in `subset.md`; boundary disagreement reported, not hidden |
| Kill criteria | Digest, admission, CPU oracle, deterministic generation or compiler failure invalidates source acceptance; malformed/misbound device output invalidates comparison; RPI.36 must be settled before a precision verdict; each failed cold gate run fails that measured candidate; time-box exhaustion is no pass |
| Evidence now | source and synthetic; actual named-device output separately windows, and sandbox device output only a lead |
| Later | Integrator's named-device comparison; renderer integration and three cold W3 runs for each stage |
| Cannot show | Arbitrary-chart correctness, an exact chart property from float agreement, renderer selection, or a performance pass from a no-window comparison |
| Decision | open |
| Decider | owner |

RTE.35, RTE.37 and CT-RTE-05 apply. No route, engine language, syntax, renderer or
LQ is selected. No product code, state, IDs, model identity or labels change.
`assets/mesh.json` counts 30,480 base vertices and 600 cells: 18,288,000 evaluations
per frame. The small sample tests translation, not that workload's throughput.

The rational asset lift is an exact input to this experiment, not recovery of an
exact pre-rounded mesh. Both poses are exact orthogonal matrices with determinant
+1; -I is central inversion of the centrally symmetric 600-cell. No approximate
camera is rounded into a pose. The sample is not a region-property test.

Freeze production command (before any device run, never automatic during checks):
`python work/experiments/language-lx/lx7-charts/freeze.py --freeze`.
Changed bytes require a new identified candidate before further evidence is taken.
Canonical records exclude host paths, time and duration; device records retain
adapter/driver/tool versions and build digests. No timing claim is produced.
