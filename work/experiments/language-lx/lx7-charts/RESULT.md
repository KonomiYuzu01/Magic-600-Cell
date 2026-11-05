# LX-7 source, synthetic and sandbox-lead result

> Integrator note, 5 November 2026. The named-device comparison ran outside the sandbox after this report was written. Read
> "Named-device run (integrator)" at the end. Lines below that say it was "not run here" describe the run that wrote this
> report.

| CT-RTE-05 field | Record |
| --- | --- |
| LX card | LX-7, RTE.59 to RTE.61 |
| Experiment ID | E-2.4-L7; frozen candidate lx7-r1 |
| Owner allowance | `docs/wiki/decisions/owner-decisions-2026-11-05.md`, decision 2; this packet implements the source/synthetic slice and named-device harness |
| Time box | what can be done now; one wrapper call of at most two hours |
| Frozen candidate digest | `07805a3bb3e3fd346e6e8740df83272d0cfb25a2cfd72aa732d2171a9674653f` |
| Frozen input digest | `bedf2f5d91e224ff0e9bd92cfde27788924960f1a768625ba53fb84986217756` |
| Manifest digest | `e7fa23382530b47718ccbbc2205eeabda7d9fdacee45357cc53abe08fc51b640` |
| Evidence class | source and synthetic accepted by the local check; actual Windows device readback retained only as a sandbox lead, not credited as the integrator's windows comparison; no performance evidence |
| Kill criteria | Digest/admission/reference/generation/compile or capture-binding failure invalidates the corresponding run. No precision judgement before RPI.36 is settled. Each future cold run must meet the complete gate. A reached time box is no pass. |
| Actual result | `check.py` exits 0, including actual DXC compilation. Named-device sandbox trial captured all 496 records. Integrator device comparison and W3: **not run here**. |
| Decision | **open** |
| Decider | **owner** |

This is experiment code only. No route, engine language, syntax, renderer, ADR or
LQ is selected. User charts are not enabled. A local acceptance pass checks the
experiment's implementation; it is not an LX-7 precision or renderer-gate pass.

## Frozen scope and reproducibility

`manifest.json` gives per-file SHA-256 values and sorted path/NUL/digest/LF aggregate
digests for candidate, inputs, synthetic outputs and upstream sources. It binds the
subset, all accepted/rejected sources, CPU evaluator, independent exact expectations,
HLSL generator, built-in stage, device harness and check. No timestamps or host paths
enter the identities. Result documents/captures are observations outside the frozen
candidate, and captures bind the manifest digest.

The generated HLSL SHA-256 is
`1c7f922e2f53e5a106accfb746c2337ce288e83c3226393fca8dbbd33ce4fc62`.
The exact device-input byte SHA-256 is
`6e717aaffe71d17c0d6ce7ff55588ab759b21a90dac4054a1a22dcea6814aebc`.
Fixtures were frozen before the successful device run: 62 vertices (24 seeded base
indices plus axis extrema, 31 asset points in total; 31 synthetic extremes), two
exact camera poses, and four stage selections, yielding 496 cases. Asset points
are rational lifts of stored float32 values scaled by exactly 1/8, **not** recovery
of pre-rounded exact mesh geometry. Synthetic inputs cover poles, inclusive
exclusion boundaries, adjacent exterior values, large coordinates, subnormals and
overflow. Poses are identity and central inversion. Exact d is 59/50.

The built-in and generated perspective use the same frozen 4D-to-3D formula.
Stereographic and gnomonic have separate declared poles. Exclusion radius 1/4096 is
an experiment parameter only; accepted product precision and singular/crossing
primitive policies remain open. `subset.md` defines evaluation order and coverage.
No shrink, animation, cell-frame transform, camera3 or rasterization is in this test.

Commands actually run from the repository root:

```text
python -B work/experiments/language-lx/lx7-charts/freeze.py --freeze
python work/experiments/language-lx/lx7-charts/check.py
python -B work/experiments/language-lx/lx7-charts/device/run.py --sandbox-lead
```

The freeze command is a deliberate pre-device production step, never invoked by
acceptance. Future changes require a new candidate identity before a device run.
`check-result.json` records the final acceptance invocation, tool version, shader
binary digest and cleanup. `cpu-records.json` and `cpu-summary.json` retain the small
synthetic observations; device records are in `device/sandbox-lead-capture.json`
and `device/sandbox-lead-summary.json`. Build products are removed on success and
failure; no executable, object, DXIL, binary input/output or Python cache is left.

Tools actually used: CPython **3.15.1**, standard library only; Windows SDK
**10.0.26100.0**; DXC/dxcompiler **1.8.2502.11 (239921522)**, dxil **1.8.2502.11**;
MSVC x64 **19.51.36260**. The direct MSVC build and DXC profile are in
`device/build.cmd`; DXC flags are `-T cs_6_2 -E main -HV 2021 -Gis -denorm preserve
-O3 -Ges -WX`. No install, network, third-party dependency or personal data was used.

## Local acceptance and raw observations

Admission accepted four fixture sources, including a bounded loop, and refused 18
counterexamples before generation. They include recursion, unbounded loops,
nonrepresentable literals, impure calls, allocation syntax, oversized loop expansion,
input mutation and invalid outputs. Nine integer-rounding golden cases cover ties,
subnormals, overflow and 1/3. Every CPU fixture operation agrees with an independent
Fraction/integer ties-to-even oracle. All exact rational expectations and frozen
CPU records reproduce. HLSL regenerates byte-identically and compiles with DXC.
Synthetic malformed/misbound captures are refused. These checks supply no GPU result
by themselves.

The separate sandbox device trial **did** produce a named hardware readback on
**NVIDIA GeForce RTX 4070 Laptop GPU**, vendor ID 4318, device ID 10336, driver
**32.0.16.1692**, dedicated memory 8,334,082,048 bytes, shader model 6.2.
It is a **lead only**: the integrator must repeat outside the sandbox. All 496
coverage codes matched CPU coverage. Finite results differed from CPU by at most
one binary32 ULP. The built-in and generated perspective records were bit-identical
for all 124 matched cases, including coverage. There was no window and no timing.

Raw maxima below are observations, **not tolerances or admission decisions**.
Absolute errors use projected-coordinate units; nonfinite/missing results are
excluded from distributions and counted separately. Large-coordinate and
singular-adjacent synthetic cases intentionally affect the maxima.

| Stage (124 cases each) | CPU max absolute error against exact | Sandbox device max absolute error against exact | Device max ULP distance to CPU | Exact/float coverage disagreements |
| --- | ---: | ---: | ---: | ---: |
| Built-in perspective | 1.7647058823529411 | 1.0320597833861 | 1 | 3 |
| Generated perspective | 1.7647058823529411 | 1.0320597833861 | 1 | 3 |
| Stereographic | 0.3333333333333333 | 0.3333333333333333 | 1 | 3 |
| Gnomonic | 0.000007628928898382667 | 0.000007628928898382667 | 1 | 2 |

Full min/p50/p95/p99/max absolute, nonzero-relative and CPU-ULP distributions,
sample counts and exact case IDs are retained in the summary JSON files. Quantiles
use nearest rank per coordinate. Relative error excludes exact zeros; ULP distance
collapses signed zero. Maximum device/CPU absolute differences were 2.0 for either
perspective, about 2.38419e-7 for stereographic and 7.62939453125e-6 for gnomonic.
No selected metric or acceptable bound is inferred from these values.

Negative observations are retained. For either perspective, one exact exterior
boundary input is classified excluded after float32 conversion/subtraction; two
finite exact overflow cases become missing coverage. Stereographic has two exterior
boundary inputs classified excluded and one overflow case. Gnomonic has two
overflow cases. Their exact formulas remain finite at those overflow inputs; typed
nonfinite coverage records the display-range loss. Relative error on deliberately
tiny values is substantial (up to 0.5 in stereographic), even when ULP distance is
small. These results cannot support an unqualified precision claim.

An initial harness attempt compiled but failed before process startup with
`FileNotFoundError`/WinError 2: the executable path was relative to the repository
root rather than the experiment directory. After tracing that failure, the launcher
was changed to use its computed executable path and the candidate was refrozen
before any successful device run. Failed-attempt products were removed. No failed
attempt is counted as device evidence.

## Remaining integrator work

The authoritative named-device comparison is **not run here**. Outside the sandbox,
from the repository root, run exactly:

```text
python work/experiments/language-lx/lx7-charts/check.py
python work/experiments/language-lx/lx7-charts/device/run.py --adapter "NVIDIA GeForce RTX 4070 Laptop GPU"
python work/experiments/language-lx/lx7-charts/device/compare_device.py work/experiments/language-lx/lx7-charts/device/device-capture.json
```

The runner writes a fresh capture and summary, validates their identity and removes
all temporary binaries. Do not run the checks and runner concurrently. The same
`check.py` command can exercise the source/reference leg from a Linux checkout;
that leg was **not run here**, and absent DXC is explicitly reported there.

W3 is **not run here**. `W3-PLAN.md` specifies the foreground-window conditions,
full gate and three cold runs per stage. Renderer integration and a generated-stage
selection interface remain unfinished by design. No exact executable W3 command for
that generated stage exists yet; it must be supplied by the integrator after the
handoff is implemented and reviewed. The existing built-in S-B command, only with
truthful overlay declaration and the required foreground/PresentMon conditions, is:

```powershell
powershell -NoProfile -File work/experiments/renderer-sb/probe/run_scene.ps1 -Scene w3 -Runs 3 -Duration 192 -Preroll 4 -Overlays 'none running'
```

Repeating that unchanged command cannot measure the generated chart. The owner must
still settle RPI.36's metric, tolerances, singular neighbourhoods and triangle
handling before judging precision, and LQ7/LQ20 remain open. Independent candidate
review and integration belong to the integrator; this packet made no commit.

**Cannot show:** arbitrary-chart correctness, an exact chart property from float
agreement, renderer selection, or a performance pass from a no-window comparison.
It also does not establish full-frame workload throughput, region/injectivity
proofs, exact retained pre-quantization vertices, native input latency or long-session
behaviour. No W3, W5, E2 or performance gate has run for this generated stage.

## Files added

All files are under `work/experiments/language-lx/lx7-charts/`:

| Files | Purpose |
| --- | --- |
| `CARD.md`, `subset.md`, `W3-PLAN.md`, `RESULT.md` | Frozen experiment scope, semantics, deferred gate plan and results |
| `subset.py`, `charts/perspective.chart`, `charts/stereographic.chart`, `charts/gnomonic.chart`, `charts/bounded.chart` | Admission and CPU interpreter; accepted chart/loop sources |
| `charts/rejected/recursion.chart`, `charts/rejected/unbounded.chart`, `charts/rejected/float64.chart`, `charts/rejected/impure.chart` | Frozen typed-refusal cases |
| `reference.py`, `make_fixtures.py`, `freeze.py`, `fixtures.json`, `expectations.json`, `manifest.json` | Rational oracle, seeded input generation and digest freeze |
| `builtin.hlsl`, `generate_hlsl.py`, `generated.hlsl` | Independent built-in formula and deterministic admitted-stage translation |
| `check.py`, `check-result.json`, `cpu-records.json`, `cpu-summary.json` | Acceptance and small source/synthetic evidence |
| `device/main.cpp`, `device/build.cmd`, `device/run.py`, `device/compare_device.py`, `device/README.md` | Named-adapter D3D12 compute/readback, cleanup and raw comparison |
| `device/sandbox-lead-capture.json`, `device/sandbox-lead-summary.json` | Actual sandbox readback and distributions, lead only |

## Named-device run (integrator)

Run on 5 November 2026 outside the sandbox, from a temporary local clone at the frozen commit `1e7d0074` with this folder laid
over it (the way `../verify_frozen.py` works), with `device/run.py --adapter "NVIDIA GeForce RTX 4070 Laptop GPU"` and then
`device/compare_device.py`. Evidence class: windows. No window was shown and nothing was timed.

| Field | Record |
| --- | --- |
| Device | NVIDIA GeForce RTX 4070 Laptop GPU, vendor 4318, device 10336, driver 32.0.16.1692, shader model 6.2, not a software adapter |
| Tools | Python 3.15.1; DXC 1.8.2502.11; MSVC 19.51.36260 |
| Records | 496, in `device/device-capture.json`; summary in `device/device-summary.json` |
| Capture digest | `cb12ac9e2b38094f9fc898206fdcadd16d05feba4dc21a31e49544915d4b7f5d` |
| Output digest | `43a789607f68a93ed50ec2f290e481da012247aecae022fdf5f5366f14db21dd`, equal to the sandbox lead |
| DXIL digest | `d5a65ae5d8ffa3c264d892fdb0b79961dc239c64aa54fcb64c07d57ae30a098d`, equal to the sandbox lead |
| Manifest, input, HLSL digests | Equal to the frozen values above |

The output bytes equal the sandbox lead's, so the table of maxima and the negative observations above now stand as windows
evidence for this device, driver and compiler. The stored comparison confirms them:

| Stage (124 cases each) | Points / singular / nonfinite | Coverage disagreements with CPU | Exact/float coverage disagreements | Max ULP to CPU |
| --- | ---: | ---: | ---: | ---: |
| Built-in perspective | 116 / 6 / 2 | 0 | 3 | 1 |
| Generated perspective | 116 / 6 / 2 | 0 | 3 | 1 |
| Stereographic | 116 / 7 / 1 | 0 | 3 | 1 |
| Gnomonic | 104 / 18 / 2 | 0 | 2 | 1 |

The built-in and generated perspective records are bit-identical across all 124 cases.

This run gives no precision verdict: RPI.36's metric, tolerances, singular neighbourhoods and triangle handling are not
accepted, and `precision_decision` stays open for the owner. It gives no performance result. W3 did not run. The decision on
LQ7 and LQ20 stays open. Agreement with the CPU float path says nothing exact about a chart.

The check is reproduced at the frozen commit: `python work/experiments/language-lx/verify_frozen.py lx7-charts`.
