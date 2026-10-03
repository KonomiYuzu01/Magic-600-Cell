---
id: interop-smoke-results
type: evidence
status: verified
visibility: public
summary: Level-1 interop smoke tests of the framework candidates. Godot 4.7.2 (.NET, editor build) shows a texture written by our own Direct3D 12 code on Godot's device and queue, through resizes and teardown, with 0 validation errors on the owner's RTX 4070 Laptop GPU; imported textures need the layout Godot's tracker holds, not RENDER_TARGET; answers to PR #43 section 7 for Godot. Qt has not run yet.
related: [renderer-candidates, s-b-probe-results]
supersedes: []
claims:
  - {id: godot-level-1-passed, evidence_kind: actual_windows_directx, path: work/experiments/renderer-sa2/results/sa2-smoke-summary.json, sha256: 6d432bfbd5f84da8f0b0db8ed9c61fd8c37238dc629fe4d67fc9c8238154ef8b, checked_at: 2026-10-03}
  - {id: godot-finalize-line-is-godot, evidence_kind: actual_windows_directx, path: work/experiments/renderer-sa2/results/sa2-blank-control-summary.json, sha256: 99cafdd19a9144d8ae78ed025cb09f0fa86d83bfcc9c1a30e2a804f9fa6770f1, checked_at: 2026-10-03}
  - {id: godot-pr43-answers, evidence_kind: actual_windows_directx, path: work/experiments/renderer-sa2/RESULT.md, sha256: 1171a131ca3bff1e3132720bcff593ba4625ea5c08d5a55fbb39848e52281c3f, checked_at: 2026-10-03}
  - {id: godot-tracker-source-facts, evidence_kind: source, path: work/experiments/renderer-sa2/README.md, sha256: d5665db479d18f7f1cf64be6e6d913d48346cc4ff2e320f3758264f1934af87c, checked_at: 2026-10-03}
---

# Interop smoke test results

A level-1 smoke test checks that a framework candidate can show a texture written by our own Direct3D 12 code, before any geometry is ported. Each test loads the SA2 producer DLL, which writes a sequence-numbered code image into a three-slot ring with a `ready` and a `free` fence, the protocol of the [S-B handoff](s-b-probe-results.md). The host reads the framework's composited output back and decodes the code on every eligible frame.

These are smoke-test results for the stated build on the owner's machine. They are kept apart from the documentation claims of the framework paper (PR #43), and statements marked "source" were read from the framework's source, not tested. No timing was measured, so nothing here says what the interop costs.

## Godot 4.7.2, 3 October 2026

Result card: [E-2.4-02](../../../work/experiments/renderer-sa2/RESULT.md). Build: harness commit `113a1b5`, producer DLL `77637425...`; Godot `4.7.2.stable.mono.official` editor build on its D3D12 driver with enhanced barriers; RTX 4070 Laptop GPU, NVIDIA driver 616.92.

- All 14 runs of the [run matrix](../../../work/experiments/renderer-sa2/README.md) were as expected.
- The passing runs (R1 to R5, R9, R10) showed the produced code through Godot's composited viewport on every eligible frame: 1,195 of 1,195 in each 1,200-frame run and 2,995 of 2,995 in R9. Every texel was compared every 50 frames, with no mismatch.
  - Export: the producer wrote into textures Godot created (zero copy), on Godot's queue and on its own queue.
  - Import: Godot copied from textures the producer created, on both queues.
  - R10 ran with Godot's separate render thread.
- With the debug layer on, the passing runs had 0 errors and 0 corruption messages. Their two warning IDs, 820 and 1356, also occur in R1, where the producer writes nothing.
- Each passing run resized the window five times and rebuilt its ring each time. Teardown confirmed the drain, and imported textures were released with refcount 0.
- R10 printed one `ERROR:` line at shutdown: `finalize` called off the render thread. A blank Godot project without the harness or the DLL printed the same line in 3 of 3 separate-thread runs and in none of 3 runs with the default (safe) thread model, so the line is Godot's own.
- Recorded as designed:
  - handovers in RENDER_TARGET without the warm-up: 312 validation errors and 3 wrongly decoded frames for textures Godot created (R7), 303 errors for imported textures (R8);
  - legacy barriers in the producer while Godot uses enhanced barriers: 50 errors (R11);
  - a drain that cannot be confirmed: exit code 3 with a one-line report (R12);
  - the editor build refuses to show an imported texture through `Texture2DRD` (R6).

### Answers to PR #43 section 7 (Godot)

1. **Import state.** An imported `ID3D12Resource` does not need to arrive in RENDER_TARGET state. It must arrive in the layout Godot's tracker holds for it: after one warm-up use per slot, every handover in that layout passed (R4, R5), and RENDER_TARGET handovers gave validation errors (R7, R8). The producer must also use Godot's barrier API, enhanced barriers on this GPU (R11). Source: Godot starts an imported texture at layout UNDEFINED, so its first use discards the contents; the warm-up comes before the producer writes.
2. **Wait and Signal placement.** In Godot's render-thread step for frame f, before Godot draws it: Godot's queue signals `free = f - 1`; the producer writes slot f mod 3, on Godot's queue, or on its own queue after waiting until Godot has finished the frame that last showed the slot; the producer signals `ready = f`, and Godot's queue waits for it. This order showed the right code on both queues and with a separate render thread. It was not timed.
3. **Device, queue, zero copy, resize and device loss.** The host took Godot's device and command queue from `RenderingDevice.GetDriverResource` on the render thread, and the producer confirmed a direct queue of that device on the reported adapter. Zero copy works for textures Godot creates (R2, R3, R9, R10); in the editor build an imported texture can be shown only through a copy (R6; source: release export templates skip the refusing check, not tested). The resizes invalidated no RID. After `RemoveDevice` (R13), Godot kept the process running, its next buffer creations failed with `DXGI_ERROR_DEVICE_REMOVED`, the producer's drain reported the removal and teardown completed; recovery is untested.
4. **Qt**: see below.
5. to 8. Not tested: PIX and RenderDoc captures, package size, screen readers, high-DPI math rendering.
9. Not tested: the smoke test has no timing; the gate with interop overhead needs the geometry port and gate runs.

What it changes:
- The S-A2 level-1 acceptance item is met for this build, so its geometry port (level 2) may start.
- The day-7 go/no-go is not decided here: it is an Astra gate ruling across all candidates.

## Qt 6.10.3

Not run yet: Qt waits for the owner's approval of the reviewed installer entry. PR #43 asks about Qt 6.12; the S-D results will hold only for 6.10.3.
