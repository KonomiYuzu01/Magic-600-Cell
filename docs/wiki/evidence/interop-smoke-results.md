---
id: interop-smoke-results
type: evidence
status: verified
visibility: public
summary: Level-1 interop smoke tests of the framework candidates on the owner's RTX 4070 Laptop GPU. Godot 4.7.2 (.NET, editor build) shows a texture written by our own Direct3D 12 code on Godot's device and queue, through resizes and teardown, with 0 validation errors; imported textures need the layout Godot's tracker holds, not RENDER_TARGET. Qt 6.10.3 shows it on Qt's own device and queue, on a QRhi created with our device and queue (fromRhi) and with our device only (fromDeviceAndContext), with 0 validation errors; imported textures need no RENDER_TARGET handover. Answers to PR #43 section 7 for both.
related: [renderer-candidates, s-b-probe-results]
supersedes: []
claims:
  - {id: godot-level-1-passed, evidence_kind: actual_windows_directx, path: work/experiments/renderer-sa2/results/sa2-smoke-summary.json, sha256: 6d432bfbd5f84da8f0b0db8ed9c61fd8c37238dc629fe4d67fc9c8238154ef8b, checked_at: 2026-10-03}
  - {id: godot-finalize-line-is-godot, evidence_kind: actual_windows_directx, path: work/experiments/renderer-sa2/results/sa2-blank-control-summary.json, sha256: 99cafdd19a9144d8ae78ed025cb09f0fa86d83bfcc9c1a30e2a804f9fa6770f1, checked_at: 2026-10-03}
  - {id: godot-pr43-answers, evidence_kind: actual_windows_directx, path: work/experiments/renderer-sa2/RESULT.md, sha256: 1171a131ca3bff1e3132720bcff593ba4625ea5c08d5a55fbb39848e52281c3f, checked_at: 2026-10-03}
  - {id: godot-tracker-source-facts, evidence_kind: source, path: work/experiments/renderer-sa2/README.md, sha256: a06bae8b6504b893fd8f876d5c8e11e1eed294e6c3a1708701b7f27fe048d22c, checked_at: 2026-10-10}
  - {id: qt-level-1-passed, evidence_kind: actual_windows_directx, path: work/experiments/renderer-sd/results/sd-smoke-summary.json, sha256: 56d5040f4b6c7b5142e56dd8017cfd8017991f44662baefa7288dcb9ba4c3f88, checked_at: 2026-10-03}
  - {id: qt-pr43-answers, evidence_kind: actual_windows_directx, path: work/experiments/renderer-sd/RESULT.md, sha256: 0e06faab5792171b8ed7526ab93457cd73e09bd91c83c013f27e8acb16f852f7, checked_at: 2026-10-03}
  - {id: qt-source-facts, evidence_kind: source, path: work/experiments/renderer-sd/README.md, sha256: 13917a821f4eb3d0077cb58bf86de9200653e3c774cf4578b68489c3a233e39f, checked_at: 2026-10-05}
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

## Qt 6.10.3, 3 October 2026

Result card: [E-2.4-03](../../../work/experiments/renderer-sd/RESULT.md). Build: harness commit `aaf01b3`, producer DLL `1743d93c...`, application `686faa1f...`; Qt 6.10.3 (`msvc2022_64`) on its Direct3D 12 backend, which uses legacy barriers; RTX 4070 Laptop GPU, NVIDIA driver 616.92. PR #43 asks about Qt 6.12; nothing here was tested on 6.12.

Routes: A is Qt's own device and queue. B is a QRhi created with our `ID3D12Device` and `ID3D12CommandQueue` and handed to Qt Quick through `QQuickGraphicsDevice::fromRhi`. C is our device only, through `fromDeviceAndContext`.

- All 17 runs of the [run matrix](../../../work/experiments/renderer-sd/README.md) were as expected.
- The passing runs (Q1 to Q9, Q11) showed the produced code through Qt Quick's composited swap-chain image on every run frame: 1,200 of 1,200 in each 1,200-frame run and 3,000 of 3,000 in Q11. Every texel was compared every 50 frames, with no mismatch.
  - Route A: Qt copied from textures the producer created, on Qt's queue and on the producer's queue; the producer wrote into textures Qt created; and Qt Quick showed the producer's texture through a texture node, without a copy.
  - Route B: copies on both queues, and the texture node in the basic render loop.
  - Route C: copies on Qt's queue.
- With the debug layer on, the passing runs had 0 errors and 0 corruption messages. Their only warning ID, 820, comes from the producer's clears, whose colours differ from the textures' optimised clear values; Q1, where the producer writes nothing, had none.
- Each passing run resized the window five times and rebuilt its ring each time. Teardown confirmed the drain. Imported textures, and on routes B and C our device and queue, were released with refcount 0.
- Private shakedown runs found that Qt allocates render targets without zeroing, so the harness read new ones before writing them. The harness now clears each new one once (SDQ-R-01); the fix was reviewed before the record run.
- Recorded as designed:
  - a handover in RENDER_TARGET, declared to Qt with `setNativeLayout` (Q10): 0 validation errors, all frames decoded;
  - the producer on enhanced barriers while Qt uses legacy barriers (Q12): 0 validation errors;
  - a drain that cannot be confirmed: exit code 3 with a pending result (Q13);
  - `RemoveDevice`: on route A Qt released its scene graph and QRhi, created a new one and resumed rendering (Q14); on route B, where the application owns the QRhi, Qt re-initialized its scene graph 1,828 times in 10 seconds without recovering (Q15).

### Answers to PR #43 section 7 (Qt)

1. **Import state** (asked about Godot; the Qt counterpart). An imported texture does not need to arrive in RENDER_TARGET. Qt's tracker starts at the state the host states when it wraps the texture (`createFrom`, `QSGD3D12Texture::fromNative`), and handovers in that state passed without a warm-up. A RENDER_TARGET handover declared with `setNativeLayout` gave 0 errors (Q10, an observation). Qt uses legacy barriers; a producer on enhanced barriers gave 0 errors (Q12).
2. **Wait and Signal placement.** In the render step of frame f (`QQuickRhiItemRenderer::render()`, or a `beforeRendering` handler for a texture node), before Qt records frame f: Qt's queue signals `free = f - 1`; the producer writes slot f mod 3, on Qt's queue, or on its own queue after waiting until Qt has finished the frame that last showed the slot; the producer signals `ready = f`, and Qt's queue waits for it. This order showed the right code on both queues and in the basic render loop. It was not timed.
3. **Device, queue, zero copy, resize and device loss** (asked about Godot; the Qt counterpart). On route A the host took Qt's device and queue from `QSGRendererInterface::getResource` in `sceneGraphInitialized`, before the first frame. Zero copy works through `QSGD3D12Texture::fromNative` (Q5, Q8). The resizes recreated the item's color buffer without a failure. After `RemoveDevice`, Qt recovered on route A and did not recover on route B; display after recovery is untested.
4. **Qt.** `createFrom` and `QQuickRhiItem` work on Qt's own device and queue through resize and teardown (Q2, Q3). A QRhi created with our device and queue and handed over through `fromRhi` keeps the QRhi, device and queue identities, and `QQuickRhiItem` rendering, resize and teardown with `releaseResources()` work on it (Q6, Q7, Q11); our device and queue reached refcount 0 after release. The device-only route `fromDeviceAndContext` works as a fallback (Q9): Qt creates its own direct queue on our device.
5. Not tested: PIX and RenderDoc captures.
6. **Package size** (preliminary): 40.8 MB in 33 files, the test application and what `windeployqt` adds for it, without the producer DLL and the Visual C++ runtime. This is not a release package.
7. and 8. Not tested: screen readers, high-DPI math rendering.
9. Not tested: the smoke test has no timing; the gate with interop overhead needs the geometry port and gate runs.

What it changes:
- The S-D level-1 acceptance item is met for this build, so its geometry port (level 2) may start.
- The day-7 go/no-go is not decided here: it is an Astra gate ruling across all candidates.
