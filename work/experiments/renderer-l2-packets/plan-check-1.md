# Plan check packet: level 2 for S-A2 (Godot) and S-D (Qt)

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: check the level 2 plan in `work/experiments/renderer-l2-packets/PLAN.md` before any packet or interface is written. Level 2 must run W1 to W4 inside each framework with the S-B drawing method, pass the same geometry and label checks as S-B, and lead to three cold W3 runs per candidate for the day-7 go/no-go.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`.
  - Report only `blocker` or `major` findings.
  - Each finding needs a concrete counterexample: what the plan would produce, and why that is wrong for the packets, the gate or the day-7 ruling.
  - The verdict is `pass` if there is no such finding.
- Check these questions:
  1. **Route.** The plan proposes the native route: our own Direct3D 12 drawing on the framework's device, inside the shared producer DLL, drawing into the texture the framework displays.
     - Does a W3 result from this route answer packets E-2.4-02 and E-2.4-03 ("W1 to W4 inside Godot/Qt using the S-B drawing method")?
     - Is it a valid result under the selection gate for candidates `sa2` and `sd`, so that the day-7 ruling can rely on it?
     - Or do the packets, the plan or the gate require the framework's own rendering path (RenderingDevice or QRhi)?
  2. **Measurement.** With this route, does a PresentMon capture of the framework process measure the frame the gate means? What must each harness provide so that `tools/perf/renderer_gate.py` can judge `sa2` and `sd` runs exactly as it judges S-B runs? This covers the trace, the label check, peak VRAM, cold runs and the declared controls.
  3. **Split and order.** Can the work run as L2-H (Claude's ABI 2 header), then L2-N (native port), then L2-G and L2-Q in parallel, with disjoint files inside each packet's owned-file rule? Look for hidden dependencies: S-D loads the DLL from `renderer-sa2/`, and S-B's readers are reused without changing S-B files.
  4. **H-09.** Is the stated shader exception (portable HLSL to DXIL; the framework never sees the shaders) acceptable under the packets' wording? What must each H-09 list contain so that the selection (E-2.4-05) can use it?
  5. **Timeline.** Is any step missing that would keep a candidate from having valid W3 runs by the day-7 ruling (stage day 9)?
- Out of scope:
  - S-B itself and its results;
  - the H-06 series;
  - the H-04 and H-01 prototypes;
  - the design of the alternative route, beyond whether the native route is admissible.

## 2. Actual problem and reproduction
- S-B met the W3 gate on three owner-attended cold runs of build `2b5bf5e6`: pooled average 778.37 fps, pooled p99 1.546 ms, peak VRAM 81.7 MB.
- Both framework candidates passed level 1. Each showed a sequence-numbered texture, written by our D3D12 code on the framework's device, through the framework's own D3D12 backend:
  - Godot 4.7.2 .NET: `texture_create_from_extension`, on Godot's queue and on the producer's queue;
  - Qt 6.10.3: routes A, B and C, on Qt's queue and on the producer's queue.
  Both ran through resizes and teardown with 0 validation errors.
- Level 2 has not started. The day-7 go/no-go needs valid W3 runs of each framework candidate.

## 3. Environment and versions
- Branch `claude/renderer-l2`, the commit that adds this packet. It is the `claude/renderer-sb` branch plus the plan.
- The owner's machine: Windows 11 and an RTX 4070 Laptop GPU with 8 GB. Godot 4.7.2 .NET and Qt 6.10.3 run there only. A Codex sandbox cannot run Godot, Qt or the GPU.

## 4. Necessary source and evidence
- The plan: `work/experiments/renderer-l2-packets/PLAN.md`.
- Packets:
  - `docs/progress/1.0/packets/renderer/E-2.4-02-sa2-godot.md`
  - `docs/progress/1.0/packets/renderer/E-2.4-03-sd-qt.md`
  - `docs/progress/1.0/packets/renderer/E-2.4-04-day7-gate.md`
  - `docs/progress/1.0/packets/renderer/E-2.4-05-selection.md`
- Plan and gate:
  - `docs/progress/1.0/renderer-experiment-plan.md` (sections 2 and 3);
  - `docs/wiki/decisions/renderer-candidates.md`;
  - `tools/perf/renderer_gate.py`.
- S-B: `work/experiments/renderer-sb/SPEC.md`, `RESULT.md`, `probe/src/` and `probe/shaders/`.
- Level 1:
  - `work/experiments/renderer-sa2/RESULT.md` and `work/experiments/renderer-sa2/native/include/sa2_interop.h` (ABI 1);
  - `work/experiments/renderer-sd/RESULT.md` and `work/experiments/renderer-sd/README.md`.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| — | none for level 2 | — | — | — |

## 6. Constraints and owned files
- Read-only check. No file changes.
- The owned-file rules of E-2.4-02 and E-2.4-03 apply to the later packets. Nothing under the repository's root `native/`, `assets/` or `tools/` changes.
- Synthetic geometry and labels only. NVIDIA-specific features stay off.
- W3 gate runs are owner-attended, with operator confirmation (Astra ruling 20261003T033021Z-274f20af).

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
