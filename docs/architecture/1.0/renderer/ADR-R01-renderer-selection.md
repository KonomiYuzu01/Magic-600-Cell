# ADR-R01: the 1.0 renderer

Status: **Accepted** (owner, 16 November 2026, with W-J option A; signature in the
[freeze record](../../../wiki/decisions/renderer-freeze.md)). Owner decision:
[close the renderer line](../../../wiki/decisions/owner-decisions-2026-11-16-renderer-close.md). Selection record:
[renderer-selection](../../../progress/1.0/renderer-selection.md). Handoff:
[H-10](../../../progress/1.0/renderer-handoff-H-10.md).

The format follows the [language records](../language/adr/README.md): Context, Decision, What this record does not
decide, Why, Alternatives considered, Consequences, Evidence, and What the owner accepts.

## Context

- Stage 2.4 tested renderer candidates against the selection gate of
  [renderer-candidates](../../../wiki/decisions/renderer-candidates.md): all 259,800 sticker slots at full detail with
  the most complex turn animation and camera workload, an average of at least 30 fps and a p99 frame time of at most
  33.3 ms over the full 180 s interval (PresentMon, native resolution, no frame generation) on the owner's RTX 4070
  Laptop GPU with 8 GB, and peak VRAM at most about 7 GB.
- The [day-7 go/no-go](../../../wiki/decisions/day7-go-no-go-ruling.md) of 10 October 2026 passed three candidates on
  their W3 runs: S-B (a bare Direct3D 12 probe), S-A2 (Godot 4.7.2 .NET) and S-D (Qt 6.10.3 Qt Quick). It selected
  none. The frozen language architecture records the same state: the renderer remains unselected (chapter 10,
  RPI.60; feature matrix RM.29).
- All three draw the 3D view with one method, the S-B probe's Direct3D 12 source. S-A2 and S-D load one shared native
  library compiled from it, and Godot and Qt only show its texture, as one flat layer (the H-09 tables of the
  [S-A2](../../../../work/experiments/renderer-sa2/RESULT.md) and [S-D](../../../../work/experiments/renderer-sd/RESULT.md)
  cards).
- The design line is on hold until LP2 to LP5 are done (owner, 22 October and 6 November 2026), so W5's presets
  (H-01 to H-04) have not arrived. Packet [E-2.4-05](../../../progress/1.0/packets/renderer/E-2.4-05-selection.md)
  then uses W3 and records that.
- LP3's on-screen half (C9) waits for the renderer freeze (chapter 12, PHS.24).
- On 16 November 2026 the owner decided to cut the remaining measurements to the minimum, to have Fable evaluate the
  selection in place of E-2.4-05's Astra review, and then to sign the freeze, which ends the renderer line.

## Decision

- **ADR-R01.1 The renderer.** The 1.0 renderer is S-A2: the S-B Direct3D 12 drawing core, built as the shared native
  library, with Godot 4.7.2 .NET as its host. The choice rests on the three cold W3 runs of build `06d14019…` (G1).
  W3 stands in for W5, as E-2.4-05 provides while no preset has arrived.
- **ADR-R01.2 Core and host.** The core draws the gated workload, all 259,800 sticker slots, with S-B's method and its
  portable HLSL. It stays a native library behind its C interface. Godot shows the core's texture and hosts the shell:
  window, UI and input. Godot's own renderer draws no part of the gated workload. The Look Lab grows into the product
  shell ([protocol](../../../progress/1.0/stage-2-experiment-protocol.md), section 4.2), and its 3D view moves to the
  core.
- **ADR-R01.3 W-J on the selected candidate.** The core gains W-J's per-piece pose path with its exact pose check.
  On that build, three cold runs each of fixtures S4, I_a and I_b (E-2.4-0J item 5) and three cold W3 runs (item 6)
  then run with unchanged bars.
  - *Option A, chosen by the owner at the signature:* they are LP3 acceptance runs at C9, done together with LP3's
    other owner-machine runs (owner, 16 November 2026). A failing run reopens this selection, and the owner decides.
  - *Option B, not chosen:* they would have run before the signature, in one attended session.
- **ADR-R01.4 RPI.80 statement.**
  - *Run-time compile path:* a checked stage becomes HLSL (RPI.78). DXC compiles it to DXIL off the frame path, under
    a budget, with the built-in chain as the fallback, and the core loads the result as a pipeline state. DXC is not
    on the toolchain allowlist, so shipping it is the owner's decision; this record leaves it open. Until then, and
    until RPI.79's run passes, no user stage is advertised (RM.12).
  - *Stopped-frame skip:* the core records a frame's user-stage draws under Direct3D 12 predication, keyed on a value
    that the raster-bound pass writes on the GPU. A frame whose bound exceeds the allowance then runs no user-stage
    rasterization or fragment work (RPI.72).
  - Both are design statements, unproven on a device. The skip is in the core's own command lists, so it does not
    depend on Godot. CT-RPI-31 tests it at C9.
- **ADR-R01.5 Product dependencies.** Godot 4.7.2 .NET (MIT) and a supported .NET release become product
  dependencies. The tests ran on .NET 8, whose support ended on 10 November 2026
  ([.NET blog](https://devblogs.microsoft.com/dotnet/dotnet-8-9-end-of-support/)). The product build therefore targets
  a supported release such as .NET 10. That is a new build identity, and RPI.79's run covers it.

## What this record does not decide

- **The look.** The design-system ADR, the presets H-01 to H-04, the layout and the motion table belong to the design
  line. Charter E1, the gate in the shipped look, stays open.
- **The feature list.** H-06's per-feature costs are unmeasured. A feature that alone breaks the gate goes back to the
  design track with its measured cost (E-2.4-05).
- **User stages.** The stage and view ceilings stay unset and no support is advertised until RPI.79's run of the
  unchanged gate passes (RM.12). Shipping a run-time compiler is an owner item (RPI.80).
- **Frozen wording.** The clauses that say no renderer is selected (RPI.60, RM.29 and chapter 10's open table) are read
  together with the signed freeze record; their wording follows in a later versioned change (PHS.52).
- **NVIDIA features and DLSS.** Optional, off by default and never on the correctness path (renderer-candidates).
- **Other platforms.** macOS and Linux versions come after 1.0 (owner, 2 October 2026).
- **The vertical slice and the living test** of stage 2.4, which stay open with the design line.

## Why

Fable 5.2 evaluated the selection in place of an Astra review (owner, 16 November 2026), and the integrator checked
each of its points ([selection record](../../../progress/1.0/renderer-selection.md), section 3). Fable ranked S-A2
first, S-D second and S-B third.

1. **The gate does not rank them.** All three pass on one drawing core. S-A2 passes uncapped: pooled 670.69 fps, p99
   1.907 ms, peak VRAM 217.2 MB (G1). S-D passes at its 60 Hz cap. Qt paces frames to the vertical blank, so one missed
   blank gives a 33.33 ms frame, above the limit (FRAMEWORK-FACTS Q10; inference). S-B is fastest, at a pooled
   788.16 fps, but it is a probe without a shell.
2. **Least extra work for the vertical slice** (E-2.4-05). The Look Lab is a Godot .NET app. Its framework-neutral
   core already owns the parameter, preset, colour, turn, layout, flow and cost rules. It grows into the product shell
   only if S-A2 is selected (protocol, section 4.2; handoff H-10). Under S-D the shell and that core are rebuilt for
   Qt Quick. Under S-B a UI stack is chosen and built from scratch.
3. **Licence and later platforms.** Godot is MIT. Qt Quick brings LGPLv3 obligations or a commercial licence, and a
   licence review of `qsb`; all of these are owner items
   ([framework paper comparison](../../../progress/1.0/framework-paper-comparison.md), section 3). Godot and Qt both
   keep a later macOS or Linux shell cheap; S-B does not.

The cost of the choice: Qt Quick is better documented for UI, per-monitor DPI and accessibility, and the framework
paper comparison counts UI as its weightiest dimension (section 2). Fable weighed that against the Look Lab's switching
cost and gave about 65 % confidence for S-A2 first.

The same protocol section says the Look Lab's use is not evidence for the renderer selection. The integrator reads
that as barring gate evidence, not the work estimate. If the owner reads it strictly, reason 2 falls away and the
choice between S-A2 and S-D becomes closer.

## Alternatives considered

- **S-D, Qt 6.10.3 Qt Quick (second).**
  - For: it passes at the refresh-rate cap, and its UI is better documented.
  - Against: the Look Lab's shell and core are rebuilt; LGPLv3 or a commercial licence and the `qsb` review come with
    it; the vertical blank cuts the usable frame budget to about 16.7 ms unless an uncapped build (D7-001) shows
    otherwise; on route B, Qt Quick does not recover from a device removal by itself.
  - It would move first if the owner's licence or dependency decision excludes Godot, if no confirmed-present route
    works on Godot while one works on Qt, or if a shell decision excludes Godot's C# UI.
- **S-B, the bare Direct3D 12 probe (third).**
  - For: it is the fastest, has the smallest VRAM and is the only candidate with W-J met. It owns its swap chain, so a
    confirmed present needs no framework change.
  - Against: "The probe is not a product renderer" ([S-B card](../../../../work/experiments/renderer-sb/RESULT.md)). It
    has no shell, so a UI toolkit with no evidence yet would be chosen and built, and a later macOS or Linux shell would
    start from nothing.
  - It stays the recorded fallback ([renderer experiment plan](../../../progress/1.0/renderer-experiment-plan.md),
    section 3: "continue S-B as the dedicated renderer with a minimal shell").
- **No selection yet, or more measurements first.** The owner closed the line on 16 November 2026. No deferred
  measurement can change a gate result, and the evaluation found none needed apart from W-J on the selected candidate
  (ADR-R01.3).

## Consequences

- **LP3 may start its on-screen half** (C9; PHS.24). Its owner-machine runs, done together, include:
  - the W-J runs under option A;
  - RPI.79's gate run on the product build with the advertised combination;
  - the windows tests of the [LP3 plan](../../../progress/1.0/language-lp3-plan.md) at C9, among them CT-RM-05,
    CT-RM-06, CT-ONE-26 and CT-RPI-31.
- **The evidence binds build `06d14019…` on Godot's editor binary.** The product build is a new build identity: a
  release export template, a supported .NET release, the pose path and the core's features. It passes the unchanged
  gate before any stage or view support is advertised (RPI.79, RM.12).
- **Work this choice adds** ([selection record](../../../progress/1.0/renderer-selection.md), section 3):
  - A confirmed-present route (RM.26), since Godot 4.7.2's public API has no swap chain handle. An engine change would
    need a Godot source build, which is outside the allowlist and an owner item.
  - A release export template build.
  - Per-monitor DPI awareness.
  - Resize with a scene loaded, and device-loss recovery.
  - The move to a supported .NET release.
- **Frozen wording.** RPI.60, RM.29 and chapter 10's open table say no renderer is selected. They are read with the
  signed [freeze record](../../../wiki/decisions/renderer-freeze.md), and their wording follows in a versioned change
  (PHS.52). Every later change to the frozen renderer (its host, the core's interface or its features) follows PHS.52:
  the recorded revision and review, and the owner's acceptance.
- **The renderer line ends.** No further stage 2.4 renderer experiment runs. D7-001 to D7-003, H-06's cost table,
  W-J item 7 and W5 are deferred; the freeze record gives each one's reason and risk.
- **The design track receives [H-10](../../../progress/1.0/renderer-handoff-H-10.md).** The vertical slice and the
  living test of stage 2.4 stay open with the design line.

## Evidence

All summaries are on `main`, judged by `tools/perf/renderer_gate.py` (format `magic600-renderer-gate-v1`); digests
are `tools/repo_digest.py` digests, listed in the [selection record](../../../progress/1.0/renderer-selection.md),
section 1.

| Ref | Summary | What it shows |
|---|---|---|
| B1 | [S-B W3](../../../../work/experiments/renderer-sb/results/w3-20261010T164807115Z-summary.json) | S-B, build `d41020f6…`: three cold W3 runs met |
| B2 | [S-B W3 with poses](../../../../work/experiments/renderer-wj/results/w3-20261010T194430557Z-summary.json) | S-B with the per-piece pose path, build `74b22fec…`: three cold W3 runs met (W-J item 6) |
| B3 to B5 | [S4](../../../../work/experiments/renderer-wj/results/wj-S4-20261010T184006692Z-summary.json), [I_a](../../../../work/experiments/renderer-wj/results/wj-I_a-20261010T191829654Z-summary.json), [I_b](../../../../work/experiments/renderer-wj/results/wj-I_b-20261010T193029102Z-summary.json) | W-J item 5 on build `74b22fec…`: three cold runs per fixture met |
| B6 | [S-B handoff](../../../../work/experiments/renderer-sb/results/handoff-20261003T063841Z-summary.json) | S-B's fenced resource handoff, with resize |
| G1 | [S-A2 W3](../../../../work/experiments/renderer-l2/results/sa2-w3-20261010T175043422Z-summary.json) | S-A2, build `06d14019…`: three cold W3 runs met |
| Q1 | [S-D W3](../../../../work/experiments/renderer-l2/results/sd-w3-20261010T172910231Z-summary.json) | S-D, build `befc441f…`: three cold W3 runs met at the refresh-rate cap |

These are actual Windows/DirectX and performance results for the build identities named, on the owner's machine.
They are the opaque baseline: no H-06 feature and no look.

## What the owner accepted

The owner signed on 16 November 2026, in chat in Chinese. The owner's words, translated: "Sign; choose A." By
signing, the owner:

1. accepted ADR-R01.1 to ADR-R01.5;
2. recorded the renderer freeze as the [freeze record](../../../wiki/decisions/renderer-freeze.md) states it, with its
   conditions, risks and deferred items;
3. approved Godot 4.7.2 .NET and a supported .NET release as product dependencies;
4. chose option A for the timing of W-J: LP3 acceptance runs at C9 (ADR-R01.3);
5. sent [H-10](../../../progress/1.0/renderer-handoff-H-10.md) to the design track.

The signature does not decide shipping DXC (RPI.80), a Godot source build, the design-system ADR, the feature list
(H-06), the wording change of the frozen clauses (PHS.52) or any stage or view ceiling (RPI.79).
