# Renderer selection record (stage 2.4)

Status: **signed.** The gap check, the minimal measurement set and the Fable evaluation were written on 16 November
2026, and the owner signed the same day with W-J option A. Owner decision:
[close the renderer line](../../wiki/decisions/owner-decisions-2026-11-16-renderer-close.md). Signed records:
[ADR-R01](../../architecture/1.0/renderer/ADR-R01-renderer-selection.md) (accepted), the
[renderer freeze record](../../wiki/decisions/renderer-freeze.md) and the [H-10 handoff](renderer-handoff-H-10.md).

This is the public summary of packet [E-2.4-05](packets/renderer/E-2.4-05-selection.md), filled for the three
candidates that passed the day-7 go/no-go. The filled packet is private. Raw captures, traces and run records stay
private; only the judge's summaries are published.

## 1. Gap check against E-2.4-05

### Evidence

All files are on `main`. Digests are `tools/repo_digest.py` digests, checked on 16 November 2026. B1, G1 and Q1 match
the values the [day-7 ruling](../../wiki/decisions/day7-go-no-go-ruling.md) and the frozen feature matrix (RM.28)
record; B2 to B6 are not cited there and were checked against the files.

| Ref | File | SHA-256 | Content |
|---|---|---|---|
| B1 | `work/experiments/renderer-sb/results/w3-20261010T164807115Z-summary.json` | `5d3752b3543b443ad29339ae16771bd56d8044876f503c65ae7d6650a39c2cac` | S-B, three cold W3 runs, build `d41020f6…`, 10 October 2026 |
| B2 | `work/experiments/renderer-wj/results/w3-20261010T194430557Z-summary.json` | `3cc627af916c9e564851d57b2c853ef389e24ddf5b634629908ce6e0cfa7c868` | S-B with the per-piece pose path, three cold W3 runs, build `74b22fec…` (W-J item 6) |
| B3 | `work/experiments/renderer-wj/results/wj-S4-20261010T184006692Z-summary.json` | `54b4bf3b1ad1cabc6b6bd589288ddfbe6f2e2c7972c7b7d38cdfdf7eb105bff1` | W-J fixture S4, three cold runs, build `74b22fec…` |
| B4 | `work/experiments/renderer-wj/results/wj-I_a-20261010T191829654Z-summary.json` | `e622ec1a3b328074d1a584f7efbb359a786369d7f6207ca83ec61a4e53c1d3f7` | W-J fixture I_a, same build |
| B5 | `work/experiments/renderer-wj/results/wj-I_b-20261010T193029102Z-summary.json` | `3ce21f3f2b5d70644bc5c013f5517e9c20fd904b3cce5f85ba6d4b34c6857330` | W-J fixture I_b, same build |
| B6 | `work/experiments/renderer-sb/results/handoff-20261003T063841Z-summary.json` | `f906f0686421f7bb0b193acfb4b3700e81e2a87eaf714183f66b37b9e50414ed` | S-B fenced resource handoff, 3 October 2026 |
| G1 | `work/experiments/renderer-l2/results/sa2-w3-20261010T175043422Z-summary.json` | `339f629cc1197f5cd35a17337a3166f69f6c5769523e073f8af718bec07d43ed` | S-A2 (Godot 4.7.2 .NET), three cold W3 runs, build `06d14019…`, 10 October 2026 |
| Q1 | `work/experiments/renderer-l2/results/sd-w3-20261010T172910231Z-summary.json` | `e990333279f6fe70ecc122059d247036945933b5f3f8ce7b98c6c921909ce52e` | S-D (Qt 6.10.3), three cold W3 runs, build `befc441f…`, 10 October 2026 |

Result cards: [S-B](../../../work/experiments/renderer-sb/RESULT.md), [W-J](../../../work/experiments/renderer-wj/RESULT.md),
[level 2](../../../work/experiments/renderer-l2/RESULT.md), [S-A2](../../../work/experiments/renderer-sa2/RESULT.md) and
[S-D](../../../work/experiments/renderer-sd/RESULT.md). Full build identities:

- S-B: `d41020f63b69ddaa480286c81ac9cbe505df9a5ea7d7f9622a93421774aaf804`; with the pose path
  `74b22fec1a969f8d595c2a5a6b3158cd632a12b8556e02ffbe65e83cd43e9daa`.
- S-A2: `06d14019c937ed55e90ad3a78e28e9988139a3fdd946e3c9f20495e9ee034106`.
- S-D: `befc441f2b5715c3f0c60d173700fc2eef22ad3eac18c6f91beac23f58f088c5`.

All three draw the 3D view with the same S-B method. S-A2 and S-D load one shared native library. It reuses the S-B
probe's readers, checks and unmodified shaders and ports its baseline GPU path. The probe's own device, window, swap
chain, presents, cost-scene features and MSAA variants are left out of it
([native library README](../../../work/experiments/renderer-sa2/native/README.md), "Reuse and record choices"). Godot
and Qt only show its texture. Every W3 run above is the same opaque baseline (no H-06 feature). The W-J build
`74b22fec…` is a copy of the S-B probe with the per-piece pose path; the shared library has no pose path.

### Acceptance items and section 4 fields

| Item | S-B | S-A2 (Godot 4.7.2 .NET) | S-D (Qt 6.10.3) |
|---|---|---|---|
| Day-7 ruling | go, 10 October 2026 | go, 10 October 2026 | go, 10 October 2026 |
| W5, three cold runs | **missing**: no preset (H-01 to H-04) while the design line is on hold; W3 is used | **missing**, same reason | **missing**, same reason |
| W3, three cold runs of one build, each run and pooled | **met** (B1): 791.42, 783.74, 789.31 fps, pooled 788.16; p99 1.492, 1.509, 1.473 ms, pooled 1.493 ms. Also met on `74b22fec…` (B2): pooled 610.28 fps, p99 1.861 ms | **met** (G1): 671.28, 671.05, 669.74 fps, pooled 670.69; p99 1.895, 1.906, 1.916 ms, pooled 1.907 ms; max 12.328 ms in run 1 | **met** (Q1): 60.00 fps in each run; p99 17.412, 17.431, 17.475 ms, pooled 17.440 ms. A vertical-blank cap, not headroom (D7-001) |
| Exact label check on every run | **met**: the judge accepts a label scene only with a passing label check (`tools/perf/renderer_gate.py`); the card gives 1,010 revisions per run. W-J runs: the exact pose check on every drawn revision | **met**, same rule; 1,010 revisions per run on the level 2 card | **met**, same rule; 1,010 revisions per run on the level 2 card |
| W-J item 5, three cold runs per fixture | **met** on `74b22fec…`: S4 614.96 fps, p99 1.926 ms (B3); I_a 613.28 fps, p99 1.911 ms (B4); I_b 611.92 fps, p99 1.920 ms (B5). W-J item 7 is unmeasured, so the W-J card does not accept W-J as a whole | **missing**: the shared library has no pose path | **missing**, same reason |
| Peak VRAM (budget about 7 GB) | 81.7 MB (B1); at most 86.85 MB on `74b22fec…` | 217.2 MB | 229.6 MB |
| Feature cost table (H-06) | **missing**; the cost scene `w3f` (`f554af5`) and the table tool (`1c95357`) are committed | **missing**; the shared library has none of the cost-scene features | **missing**, same as S-A2 |
| Renderer constraints (H-09) | no framework; the probe's limits are in its SPEC | delivered 10 October 2026 (S-A2 card). Its pacing row quotes the 4 October runs on build `709a24eb…`, not G1. The plan's and the protocol's H-09 rows still say open | delivered 10 October 2026 (S-D card). Its pacing row likewise quotes the 4 October runs on build `64f08369…` |
| Build that ran | the probe executable | the official Godot 4.7.2 .NET binary, an editor build, running the project; no release export template was built (S-A2 harness README; S-A2 card, "Not claimed") | a Qt 6.10.3 deployment of the level 2 app |
| Interop | fenced handoff on the same device, to a second device in a child process and to a D3D11 consumer, with resize; every frame verified (B6, 3 October) | level 1, 3 October: zero copy for textures Godot creates; in the editor build an imported texture only through a copy (R6); after a device removal the process survives, recovery untested | level 1, 3 October: zero copy through a texture node; route A recovers from a device removal by itself, route B does not; display after recovery untested |
| Resize with a scene loaded | the probe's handoff passed resizes (B6) | the shared library needs a drain, a scene reload and a ring rebuild; level 2 refuses a resize | same as S-A2 |
| Presented-frame report (RM.26, ONE.41; required, RPI.66) | **not implemented**; the probe owns its swap chain | **not implemented**; Godot owns the swap chain, and Godot 4.7.2's public API offers no handle to it or to its frame statistics (`RenderingDevice.DriverResource` and `DisplayServer.HandleType` in the installed .NET API reference) | **not implemented**; Qt owns the swap chain; `QQuickWindow::swapChain()` returns Qt's swap chain object (FRAMEWORK-FACTS, Q4), and native access to the DXGI swap chain is not recorded |
| Picking and product recovery (F18, F22) | not implemented | not implemented | not implemented |
| Attribution scenes W1, W2, W4 (not gate scenes) | **not run** | run 4 October on the pre-anchor build `709a24eb…` | run 4 October on the pre-anchor build `64f08369…` |
| Build binding | build identity hashed from the executable and its four DXIL blobs (seven in the W-J build) | the 10 October runs ran under the loaded-file guard, within its stated limits; its attended experiments are unrun (D7-003) | same as S-A2 |
| Discrete GPU (gate conditions) | every summary records the RTX 4070 Laptop GPU driving the display; the operator's discrete (MUX) declaration is in B1 | the presenting adapter is recorded; **no** manual declaration in the summary | same as S-A2 |
| RPI.80: run-time stage compile path and stopped-frame skip | **not stated**: S-B's HLSL is compiled offline by DXC to DXIL; no run-time compiler | **not stated**, same shaders | **not stated**, same shaders |
| Framework licence | none | MIT ([framework paper comparison](framework-paper-comparison.md), section 3) | Qt Quick: commercial, LGPLv3 or GPLv2; `qsb` is GPLv3-only for open-source users; a licence review is an owner item (same section) |
| Product dependencies (owner approval) | none beyond the Windows runtime | Godot 4.7.2 .NET and the .NET 8 runtime (`net8.0`, `Godot.NET.Sdk/4.7.2`). .NET 8's support ended on 10 November 2026 ([.NET blog](https://devblogs.microsoft.com/dotnet/dotnet-8-9-end-of-support/)), so a product build needs a supported .NET release | Qt 6.10.3 as tested |
| Work the vertical slice still needs | section 3 | section 3 | section 3 |

## 2. Minimal measurement set

A measurement is kept only if the gate or E-2.4-05's acceptance cannot be judged without it (owner decision of
16 November 2026).

| Item | Decision | Reason |
|---|---|---|
| W-J item 5 on the selected candidate | **needed** | Owner, 22 October 2026 (Q3). Met for S-B on `74b22fec…`. For S-A2 or S-D the pose path must first be ported into the shared library. That changes shared paths, so three cold W3 runs of the new build follow (E-2.4-0J item 6), and the new build gets its own identity. |
| D7-001: uncapped Qt (new identity, three attended cold W3 runs) | deferred | The gate is pass or fail, and S-D passes at its cap; an uncapped figure adds headroom detail only. Needed only if S-D is to be selected. |
| D7-002: S-B's W1, W2 and W4 | deferred | Attribution scenes, not gate scenes; S-B's gate scene, W3, is met. |
| D7-002: H-06 cost table | deferred | It serves the Look Lab cost meter and the 2.5 feature list; no feature list exists to cost while the design line is on hold, and the three candidates share one drawing core. |
| D7-002: H-09 | nothing to run | Delivered for S-A2 and S-D on 10 October 2026; S-B hosts no framework. |
| D7-003: the guard's attended experiments | deferred | They test the level 2 harness's file guard, not a candidate; the 10 October runs ran under the guard within its stated limits. |
| W-J item 7: overlay costs | deferred | E-2.4-05 asks for item 5 only; the overlays' look and costs are design-line work. |
| Peak VRAM | nothing to run | Recorded for all three, far below the budget. |
| W5 | not available | No preset arrives while the design line is on hold; W3 is used and recorded. |
| RPI.80 statements | **needed**, no GPU | A design statement for the selected candidate, with the owner's compiler decision kept open ([ADR-R01](../../architecture/1.0/renderer/ADR-R01-renderer-selection.md)). |

**Result: no GPU measurement is needed to rank the candidates.** If S-B were selected, no GPU evening would be needed,
but S-B has no shell and needs the most vertical-slice work (section 3). If S-A2 or S-D is selected, W-J item 5 and its
W3 re-acceptance run on a pose-path build of the shared library: either in one attended evening before the signature,
or as LP3 acceptance runs on the frozen candidate. The owner chose the LP3 acceptance runs (option A) at the signature.

Risks that come with the deferrals:

- **No look is measured.** Every result is the opaque baseline. E1 (the gate in the shipped look) stays open for the
  vertical slice and for LP3's run on the selected renderer (RPI.79). H-06's per-feature costs are unknown on every
  candidate.
- **S-D's margin is quantized.** Qt delivers each frame after the vertical blank (fact Q10). At 60 Hz two refresh
  intervals are 33.33 ms, above the 33.3 ms limit, so S-D would fail the p99 if more than 1 % of frames missed a
  vertical blank. How far a heavier load can go before that is not measured (D7-001).
- **Binding.** S-A2's and S-D's 10 October identities rest on the loaded-file guard within its stated limits (D7-003).
- **Overlays.** W-J's overlay drawings exist, but their costs are unmeasured (item 7).
- **Conditions record.** The level 2 and W-J summaries lack the operator's discrete-GPU declaration; the presenting
  adapter is recorded in each.

## 3. Evaluation

Fable 5.2 (`fable-solver`, effort `high`) evaluated the selection on 16 November 2026, in place of E-2.4-05's Astra
review (owner decision). Its seven-part packet held the evidence and the criteria and named no preferred candidate. The
packet and the full answer are private. Its points are leads; the integrator checked each against the files cited.

### Ranking

Fable ranked **S-A2 first, S-D second and S-B third**. It gave about 65 % confidence for S-A2 first and about 55 % for
S-D above S-B. All three pass the gate on W3 with one drawing core, so the gate does not rank them. The ranking rests on
E-2.4-05's rule that the vertical slice should need the least extra work, on the framework constraints and on Qt's
frame pacing.

| Criterion | S-A2 (Godot) | S-D (Qt) | S-B (bare) |
|---|---|---|---|
| Gate on W3 | met uncapped: pooled 670.69 fps, p99 1.907 ms | met at the 60 Hz cap: pooled p99 17.440 ms. A frame that misses one vertical blank lasts 33.33 ms, so the usable budget for a heavier look is about 16.7 ms (inference) | met: pooled 788.16 fps, p99 1.493 ms |
| W-J on the candidate | missing | missing | met on `74b22fec…` |
| Peak VRAM | 217.2 MB | 229.6 MB | 81.7 MB |
| Least extra work for the vertical slice | The Look Lab is a Godot .NET app whose net8.0 core owns the parameter, preset, colour, turn, layout, flow and cost rules; it grows into the product shell if S-A2 is selected ([protocol](stage-2-experiment-protocol.md), section 4.2) | The Look Lab's shell and its .NET core are rebuilt for Qt Quick | No shell: "The probe is not a product renderer" (S-B card); a UI stack is chosen and built |
| A feature that alone breaks the gate | not applicable: no H-06 cost is measured | same | same |
| RPI.80, RPI.79, RPI.17 | no run-time compiler and no stopped-frame skip yet; shared core, so the same on all three; no proven conflict | same | same |
| Licence and platform | MIT; a cross-platform shell | LGPLv3 obligations and a `qsb` licence review (owner items); a cross-platform shell; better documented UI, per-monitor DPI and accessibility ([framework paper comparison](framework-paper-comparison.md), section 2) | no framework; a later macOS or Linux shell is built from scratch |

The framework paper comparison calls UI the weightiest dimension and documents Qt Quick as stronger there (section 2).
That is the main argument against S-A2 first. Fable weighed it against the Look Lab's switching cost, Godot's simpler
licence and Qt's paced delivery.

### Dispositions

| Lead | Reply | Check |
|---|---|---|
| Gate figures, VRAM and digests | adopt | The summaries and `repo_digest.py` digests match section 1. |
| S-D's quantized frame budget | adopt, as an inference | FRAMEWORK-FACTS Q10; 33.33 ms is above 33.3 ms. |
| W-J met only on the S-B probe copy | adopt | W-J card; the shared library has no pose path. |
| The Look Lab grows into the product shell only under S-A2 | adopt | Protocol section 4.2; the plan's H-10 row; framework paper comparison, section 8. The same protocol paragraph says the Look Lab's use is not evidence for the selection; that bars gate evidence, not the work estimate. |
| RPI.80 unmet by all three, in the shared core | adopt | Native library README ("there is no runtime shader compiler"); chapter 10, RPI.80 and its open table. |
| Licence facts | adopt | Framework paper comparison, section 3, documented on 2 October 2026 for Godot 4.7 and Qt 6.12. |
| A pose-path port needs W3 re-acceptance and a new identity | adopt | E-2.4-0J item 6. |
| S-A2's gate evidence ran on Godot's editor binary | adopt | S-A2 harness README ("no release template build"); S-A2 card, "Not claimed". |
| The presented-frame report could change the ranking | adopt, strengthened | RM.26, ONE.41 and RPI.66 make it required, and no candidate implements it. The integrator checked Godot 4.7.2's installed .NET API reference: `RenderingDevice.DriverResource` reaches the device, queue and resource objects only, and `DisplayServer.HandleType` the window handles only. On S-A2 a confirmed present therefore needs a change to the Godot engine or another confirmation route. S-D's route through Qt's swap chain object is unrecorded. S-B owns its swap chain. |
| Godot and the .NET runtime need the owner's approval as product dependencies | adopt | Briefing, 15 November 2026 entry. |
| The lockfile pins Godot "4.7.x" | reject_with_evidence | The `godot` entry of `tools/toolchain.lock.json` pins version `4.7.2` (`GodotEngine.GodotEngine.Mono`); "4.7.x" is its purpose text. |
| The .NET runtime version is unverified | adopt, now verified | .NET 8: the S-A2 project targets `net8.0` with `Godot.NET.Sdk/4.7.2`. The integrator adds that .NET 8's support ended on 10 November 2026 (risks below). |
| Gap-check errors 1 to 12 | adopt (8 in part) | Corrected in sections 1 and 2. S-B's identity covers four DXIL blobs, as written; the W-J build's covers seven. |
| Verdicts on the deferred items | adopt | No deferred measurement is needed for a sound selection. W-J item 5 stays needed on the selected candidate. |

### Work the vertical slice still needs

Common to all three:

- The look in the core's HLSL. The Look Lab draws its 3D view with Godot's own renderer, its costs unmeasured
  ([Look Lab README](../../../tools/looklab/README.md)), so no look carries over as gate evidence.
- The H-06 features in the product core, with their costs; transparency, since the core is opaque only.
- The RPI.80 statement and the owner's run-time compiler decision.
- The presented-frame report (RM.26), picking (F18) and device recovery (F22).
- A screen-position interface for UI anchored to the 3D view (no ABI 2 function returns one).
- The link from the Rust engine process to the shell.
- The gate in the shipped look (charter E1; RPI.79).

S-A2 only:

- The pose path in the shared library, with its exact pose check, then W-J item 5 and the W3 re-acceptance.
- A release export template build; the gate evidence ran on the editor binary.
- A confirmed present without a public swap chain handle: a change to the Godot engine or another route.
- Per-monitor DPI awareness (Godot 4.7.2 is system DPI aware only, FRAMEWORK-FACTS G10).
- Resize with a scene loaded; device-loss recovery; the imported-texture refusal if imported textures are needed (R6).
- Replacing the Look Lab's Godot-drawn viewport with the shared library's view.
- Moving the project and the Look Lab's core from .NET 8, out of support since 10 November 2026, to a supported .NET
  release, with a new build identity.

S-D only:

- The pose path and W-J, as for S-A2.
- A pacing decision: stay capped, or go uncapped on a new build identity (D7-001).
- Rebuilding the Look Lab's shell and its .NET core for Qt Quick.
- The licence review; route B device recovery; high-DPI scaling, which was off in the tests (FRAMEWORK-FACTS Q8).

S-B only:

- A whole shell: windowing, UI, text, input and accessibility, with a toolkit that has no evidence yet.
- Composition of the 3D view with that UI. Nothing for W-J.

### Risks for S-A2

| Risk | Evidence | Mark |
|---|---|---|
| W-J item 5 and the W3 re-acceptance are unrun on S-A2's pose-path build | Section 2; E-2.4-0J items 5 and 6 | condition of the freeze: the owner signed with option A, so they are LP3 acceptance runs at C9 |
| No confirmed-present route; Godot's public API has no swap chain handle | RM.26, ONE.41, RPI.66; the installed Godot .NET API reference | record; it adds engine work to S-A2, and LP3 tests it (CT-RM-06, CT-ONE-26) |
| The gate evidence ran on the editor binary; export templates are untested | S-A2 harness README; S-A2 card | record; RPI.79 re-runs the gate on the product build |
| No look is measured; E1 is open | Section 2 | record |
| RPI.80: no run-time compiler; shipping DXC is an owner item | Chapter 10, RPI.80 and its open table | record; the statement is in ADR-R01 |
| Device-loss recovery untested; resize with a scene unsupported; system DPI awareness only | S-A2 card (H-09 and level 1); FRAMEWORK-FACTS G10 | record |
| Build binding rests on the guard, whose attended experiments are unrun | Level 2 card; day-7 ruling, D7-003 | record |
| Godot 4.7.2 .NET and .NET 8 become product dependencies | Briefing, 15 November 2026 entry | approved at the signature as Godot 4.7.2 .NET and a supported .NET release (ADR-R01.5) |
| The tested runtime, .NET 8, left Microsoft's support on 10 November 2026. A supported release such as .NET 10 is a new build identity, and whether Godot 4.7.2 accepts it is not verified here | S-A2 project (`net8.0`); [.NET blog](https://devblogs.microsoft.com/dotnet/dotnet-8-9-end-of-support/) | record; RPI.79's run uses the product build |
| One 12.328 ms frame in G1's run 1 | G1 | record |
| The G1 summary lacks the operator's discrete-GPU declaration | Section 2 | record |

What would change the ranking, in Fable's answer:

- a failed W-J or export-template run on S-A2;
- a strict owner reading of the protocol's sentence that the Look Lab's use is not evidence for the selection
  (section 4.2), which would take away the work estimate behind S-A2's lead;
- a confirmed-present route on Godot that proves impractical;
- the owner's licence or dependency decision;
- a D7-001 result;
- a shell-language or design-system decision that excludes Godot's C# UI.
