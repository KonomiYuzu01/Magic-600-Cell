# E-2.4-03 S-D result card

This is the experiment card ([protocol section 2](../../../docs/progress/1.0/stage-2-experiment-protocol.md)) for packet [E-2.4-03](../../../docs/progress/1.0/packets/renderer/E-2.4-03-sd-qt.md). It is filled in as results arrive: an item that has not run is listed as open, never estimated. Raw logs and run records stay private; only sanitised summaries are published.

| Field | Content |
|---|---|
| ID | E-2.4-03 |
| Question | Can a Qt Quick application host the S-B drawing method in a `QQuickRhiItem` on Qt's QRhi Direct3D 12 backend inside the selection gate, and what does it constrain (H-09)? |
| Decision it feeds | The day-7 go/no-go (E-2.4-04), the selection (E-2.4-05), and PR #43 section 7 (the framework paper's interop questions). |
| Hypothesis | Level 1: a texture written by our own D3D12 code on the device and queue Qt Quick submits to displays correctly when it is handed over in the state Qt's tracker holds, on Qt's own device (route A), on a QRhi created with our device and queue and adopted through `QQuickGraphicsDevice::fromRhi` (route B), and with our device only through `fromDeviceAndContext` (route C). |
| Method | Level 1 smoke test: the SA2 producer DLL ([../renderer-sa2/native/](../renderer-sa2/native/README.md)), loaded by a C++ Qt Quick application (packet [SD-Q](../renderer-sd-packets/SD-Q-qt.md)), run matrix Q0 to Q15 in [README.md](README.md). |
| Qt version | 6.10.3 (`msvc2022_64`), the newest release whose repository layout the pinned aqtinstall 3.3.0 reads. PR #43 asks about Qt 6.12; nothing here holds for another Qt version. |
| Time box | Window days 3–6 (stage days 5–8), shared with E-2.4-02. |
| Kill criteria | Level 1 not passed by the end of window day 6. |
| Evidence class | Actual Windows/DirectX (the Qt smoke test, below), valid only for the source and build stated. Statements marked "source" come from Qt 6.10.3's source. No performance evidence. |
| Result | Level 1 passed on the owner's GPU for the build stated below. The producer's sequence-numbered texture was shown correctly through Qt Quick's Direct3D 12 backend on all three routes, on Qt's queue and on the producer's own queue, through resizes and teardown, with 0 validation errors. Imported textures need no RENDER_TARGET handover (question 1 below). Qt Quick keeps an application's QRhi, device and queue handed over through `fromRhi`, and the device-only route works as a fallback (question 4). |
| Decision | Open: level 2 (geometry port) may start; the day-7 go/no-go is an Astra gate ruling across all candidates. |

## Qt smoke test (level 1), 3 October 2026

Harness at commit `aaf01b3` (source digest `24d32d95...`, matching HEAD), producer DLL `1743d93c...`, application `686faa1f...`; Qt 6.10.3 (`msvc2022_64`) on its Direct3D 12 backend, which uses legacy barriers; threaded render loop except Q8 (basic); RTX 4070 Laptop GPU, NVIDIA driver 616.92. Public summary: [sd-smoke-summary.json](results/sd-smoke-summary.json). All 17 runs were as expected. The run matrix and the judging rules are in [README.md](README.md).

Routes: A is Qt's own device and queue. B is a QRhi the application creates with its own `ID3D12Device` and `ID3D12CommandQueue` (`QRhiD3D12NativeHandles`) and hands to Qt Quick through `QQuickGraphicsDevice::fromRhi`. C is the application's device only, through `QQuickGraphicsDevice::fromDeviceAndContext`; Qt creates its own queue on it.

| Run | What it tests | Result |
|---|---|---|
| Q0 | producer self-test before the matrix | pass |
| Q0b | C++ code-layout fixture (no Qt) | pass |
| Q1 | route A, Qt uploads the expected image itself (baseline, the producer writes nothing) | pass |
| Q2, Q3 | route A, import and copy: Qt copies from textures the producer created, on Qt's queue or the producer's own | pass |
| Q4 | route A, export and copy: the producer writes into textures Qt created | pass |
| Q5 | route A, import direct: Qt Quick shows the producer's texture through a texture node, without a copy | pass |
| Q6, Q7 | route B, import and copy, on Qt's queue or the producer's own | pass |
| Q8 | route B, import direct, basic render loop | pass |
| Q9 | route C, import and copy | pass |
| Q10 | handover in RENDER_TARGET, declared to Qt with `setNativeLayout` after each production | recorded: 0 validation errors; all 300 frames decoded |
| Q11 | route B on the producer's queue, without the debug layer, 3,000 frames | pass |
| Q12 | the producer on enhanced barriers while Qt uses legacy barriers | recorded: 0 validation errors; all 1,200 frames decoded |
| Q13 | an unconfirmed drain | exit 3 with the exact one-line report and a pending result |
| Q14, Q15 | `RemoveDevice` at run frame 200, routes A and B | recorded: see question 3 below |

- Each passing run with 1,200 run frames decoded the produced sequence number from Qt's composited swap-chain image on all 1,200 run frames; Q11 decoded 3,000 of 3,000. Every 50 run frames the harness compared every texel of the producer's slot (all producer runs) and of the item's color buffer (the copy runs and the baseline), with no mismatch.
- With the debug layer on, the passing runs had 0 errors and 0 corruption messages. Their only warning ID is 820 (a render-target clear whose colour differs from the texture's optimised clear value), about 5.7 per produced frame, from the producer's clears. Q1, where the producer writes nothing, had no warning.
- Each passing run resized the window five times, cycling through four sizes; the item's color buffer and the ring were rebuilt each time. Teardown confirmed the drain and detached. Each import run released its 18 imported textures (3 slots x 6 ring generations) with refcount 0. On route B the application's device and queue, and on route C its device, reached refcount 0 when the application released them after deleting the window.
- Private shakedown runs before the record run found one harness defect (SDQ-R-01). Qt allocates render targets without zeroing, and the harness read the item's new color buffer and new export slots before writing them (debug-layer error 1422). Each ring creation now clears them once with the optimised clear values Qt declares. Sol review `20261003T192207Z-5ead16e7` of the fixed candidate had no finding.

### Answers to PR #43 section 7 (Qt)

These are smoke-test results for Qt 6.10.3 and the build above. PR #43 asks about the pinned Qt 6.12; nothing here was tested on 6.12. Questions 1 and 3 are asked about Godot; the Qt counterparts are given because S-B needs them for both stacks. Statements marked "source" come from Qt's source and were not tested separately.

1. **Import state (Qt counterpart).** An imported texture does not have to arrive in RENDER_TARGET. Qt's tracker starts at the state the host states when it wraps the texture (`QRhiTexture::createFrom`, or `QSGD3D12Texture::fromNative` for a texture node) and follows it with legacy barriers from then on.
   - Passed without a warm-up: the producer hands each slot back in the state it was wrapped in, COPY_SOURCE on the copy routes and PIXEL_SHADER_RESOURCE on the direct routes (Q2, Q3, Q5 to Q9, Q11).
   - Recorded: a handover in RENDER_TARGET, declared to Qt with `setNativeLayout(RENDER_TARGET)` after each production, gave 0 validation errors and decoded all 300 frames (Q10). This is an observation, not a correctness claim.
   - The producer may use enhanced barriers while Qt uses legacy barriers: 0 validation errors (Q12).
   - Textures Qt creates (export route, Q4) start in COMMON (source). Qt allocates them without zeroing, so the host clears each new one once and warms it with one copy before the producer writes; the producer then hands it back in COPY_SOURCE.
2. **Wait and Signal placement.** In the render step of frame f, on the thread Qt renders on and before Qt records frame f: `QQuickRhiItemRenderer::render()` on the item routes, a `QQuickWindow::beforeRendering` handler on the direct routes.
   1. Signal `free = f - 1` on Qt's queue.
   2. The producer writes slot f mod 3, on Qt's queue, or on its own queue after waiting until Qt has finished the frame that last showed the slot.
   3. The producer signals `ready = f`, and Qt's queue waits for `ready = f`.
   4. Qt records the copy, or draws the slot through the texture node, in frame f. It submits frame f in `endFrame`, after the wait in queue order (source).

   With this order every run frame showed the produced sequence number: on Qt's queue (Q2, Q4 to Q6, Q9), on the producer's queue (Q3, Q7, Q11) and in the basic render loop (Q8). Nothing was timed, so the smoke test says nothing about stalls.
3. **Device, queue, zero copy, resize and device loss (Qt counterpart).**
   - On route A the host took Qt's device and command queue from `QSGRendererInterface::getResource` in `sceneGraphInitialized`, on the render thread, before the first frame. They matched the QRhi's native handles (the device by COM identity, the queue by pointer), and the producer confirmed a direct queue of that device on the expected adapter.
   - Zero copy works: Qt Quick showed the producer's own texture through `QSGD3D12Texture::fromNative` and a texture node (Q5 on route A, Q8 on route B).
   - Each resize recreated the item's color buffer, and the harness rebuilt its ring at the new size. No Qt or producer call failed.
   - Device loss on route A (Q14): Qt detected the loss in `Present()`, released its scene graph and QRhi, created a new QRhi and resumed rendering: 584 more render steps in the remaining observation time.
   - Device loss on route B (Q15): Qt Quick cannot replace a QRhi the application supplied. In the 10-second observation it re-initialized its scene graph 1,828 times, and each attempt failed to create a swap chain on the removed device. Source: recovery on this route is the application's job.
   - On both routes all 199 eligible composites before the removal decoded. The copy readback of the frame of the removal returned wrong contents (914,944 of 921,600 texels differ; an earlier mismatch would have stopped the run), and that frame's composite was not checked. The producer's drain reported the removal (`SA2_E_DEVICE_REMOVED`, reason `DXGI_ERROR_DEVICE_REMOVED`), teardown completed, and the imported textures and, on route B, our device and queue reached refcount 0.
   - The harness never reattaches the producer after a loss, so display after recovery is untested.
4. **`createFrom`, `QQuickRhiItem`, `fromRhi` and `fromDeviceAndContext`.**
   - Route A: on Qt's own device and queue, `createFrom` and `QQuickRhiItem` behaved correctly through five resizes and teardown (Q2, Q3).
   - Route B: a QRhi created with our `ID3D12Device` and `ID3D12CommandQueue` (`QRhiD3D12NativeHandles`) and handed to Qt Quick through `QQuickGraphicsDevice::fromRhi` kept the native identities in every route-B run (Q6 to Q8, Q11, Q13, Q15). Qt Quick rendered with that QRhi (same pointer), our device (same COM identity) and our queue (same pointer), and Qt logged "Using imported device".
   - On route B, `QQuickRhiItem` rendering, resize and teardown worked (Q6, Q7, Q11: every run frame decoded, five resizes each). At shutdown the host closed the window and called `releaseResources()`. Qt destroyed the item's renderer on the render thread while the QRhi survived. After the window was deleted, the host deleted the QRhi and released our queue and device, which reached refcount 0. Every imported texture was released with refcount 0.
   - The device-only fallback works (Q9). With `fromDeviceAndContext`, Qt created its own QRhi and a direct queue on our device and logged "Using existing native D3D12 device". The producer confirmed that the queue belongs to our device, and the run passed. Qt takes no application queue on this route, so the producer works on Qt's queue or on its own.
5. Not tested: PIX and RenderDoc captures.
6. **Package size (preliminary).** 40.8 MB (40,766,408 bytes) in 33 files: the test application and what `windeployqt` adds for it (release, no translations, no software OpenGL, no system D3D compiler, no QML imports). The producer DLL and the Visual C++ runtime are not included. This is not a release package.
7. and 8. Not tested: screen readers, high-DPI math rendering. Every run used a device pixel ratio of exactly 1.
9. Not tested: the smoke test has no timing. The gate with interop overhead needs the geometry port (level 2) and gate runs.

## Renderer constraints (H-09), 10 October 2026

What hosting the S-B drawing method in Qt Quick constrains, for the design track (due on day 8). Our DLL draws the 3D view with Direct3D 12 on Qt's device into a texture, and Qt shows that texture through a texture node; Qt never sees the DLL's geometry, depth buffer or shaders. "Source" means Qt 6.10.3's source ([FRAMEWORK-FACTS.md](../renderer-l2-packets/FRAMEWORK-FACTS.md), ID given), the DLL or the [level 2 plan](../renderer-l2-packets/PLAN.md). "Measured" means the owner's level 1 runs of 3 October or the level 2 runs of 4 October for build `64f08369…`, whose loaded-build binding is still open (L2-V-002, [level 2 card](../renderer-l2/RESULT.md)). This list adds no measurement and says nothing about W5 or the final look.

| Area | Constraint | Evidence |
|---|---|---|
| Overlay layers | The 3D view is one flat layer: Qt Quick items go above or below it, never between stickers. | source (plan section 2) |
| | With nothing over it, the composite showed the producer's texture correctly on every checked frame. | measured (level 1) |
| | Level 2 drew nothing over the 3D view, so the cost and correctness of UI layers over it are not known. | not verified |
| Text in the 3D view | The hosted method draws stickers only; it has no text pass. Qt text sits above the 3D view, so stickers cannot hide it. | source ([S-B SPEC](../renderer-sb/SPEC.md) section 3) |
| | Text tied to a sticker needs that sticker's screen position every frame; no ABI 2 function returns one (`sa2_interop.h`). | source |
| | Level 2 runs with `QT_ENABLE_HIGHDPI_SCALING=0`, so the device pixel ratio is 1 (Q8). Scaled UI text beside a native-pixel 3D view was not tried. | source; not verified |
| Transparency and sorting | Opaque only: depth test on, blending off, no MSAA. The DLL clears to an opaque background, so the 3D view hides anything below it. | source (S-B SPEC section 3, [DLL README](../renderer-sa2/native/README.md) "Reuse and record choices") |
| | Per-sticker transparency (alpha 0.6, all 259,800 centres sorted back to front on the GPU every frame) exists only in the S-B probe; the hosted path omits features. Its cost in Qt (H-06) is not measured. | source; not verified |
| Shaders | S-B's unchanged HLSL, compiled offline by DXC to DXIL; no runtime compiler; the blob digests enter the build identity. Stated exception: no Qt shader tools, because Qt never sees these shaders and one port serves both candidates. The Astra plan check found the exception admissible. | source (plan section 2) |
| Frame pacing | Qt delivers each frame request after the display's vertical blank, whatever the swap interval (Q10). The level 2 W3 runs measured 60.00 fps with p99 17.380 ms at 60 Hz: a cap, not headroom. An uncapped run needs `QT_D3D_NO_VBLANK_THREAD=1` and `QT_QPA_UPDATE_IDLE_TIME=0`, a new build identity; it has not been run. | source; measured (level 2) |
| Resize | A size change needs a drain, a scene reload and a ring rebuild in the DLL; level 2 refuses one. Level 1 passed resizes without a scene. | source; measured (level 1) |
| Full screen | `showFullScreen()` gives a borderless window on the monitor rectangle (Q1). | source |
| Colour format | RGBA8 UNORM. | source (DLL README) |
| Zero copy | A native D3D12 texture is shown through a texture node without a copy (level 1 run Q5). | measured (level 1) |
| Measured setup | 2560 x 1600 at 60 Hz, the 3D view filling the window; peak VRAM 229.6 MB against a budget of about 7 GB. | measured (level 2) |

The design track can rely on: the full-detail opaque baseline at native resolution meeting the W3 gate (capped at the refresh rate, with the L2-V-002 qualification); UI and text as layers above or below the 3D view, not inside it; one set of DXIL shaders shared with Godot; VRAM far below the budget.

Not known yet: the cost and correctness of UI layers, translucent panels and text over the 3D view; sticker-anchored or depth-correct text; transparency with sorting and every other H-06 feature; Qt's uncapped frame rate; a 3D view smaller than the window, resize with a scene, DPI-scaled UI; HDR or 10-bit output; picking (H-05).

## Acceptance items (packet section 1)

| # | Item | Status |
|---|---|---|
| 1 | Interop smoke test | Done for the build stated: Q1 to Q9 and Q11 passed on the owner's GPU. |
| 2 | Geometry port | Done: the level 2 app hosts the S-B method through the shared DLL ([level 2 card](../renderer-l2/RESULT.md)). |
| 3 | Three cold W3 runs | Measured on 4 October for build `64f08369…`: pooled 60.00 fps, p99 17.380 ms, verdict `met`, a vertical-blank cap; loaded-build binding open (L2-V-002). |
| 4 | Layout specification and feature list | Not started. |
| 5 | Renderer constraints (H-09) | Delivered on 10 October (above). |

## Not claimed

- No timing was measured: nothing here says whether the interop stalls or what it costs.
- Nothing here holds for Qt 6.12 or another Qt version, another build, driver or machine.
- Display after a device-loss recovery is untested, and on route B Qt Quick did not recover by itself.
- The Qt-created initial state on the export route (COMMON) is from Qt's source; the harness records it without reading it back.
- The package size is a smoke-test deployment, not a release package.
