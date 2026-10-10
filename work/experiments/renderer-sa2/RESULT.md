# E-2.4-02 S-A2 result card

This is the experiment card ([protocol section 2](../../../docs/progress/1.0/stage-2-experiment-protocol.md)) for packet [E-2.4-02](../../../docs/progress/1.0/packets/renderer/E-2.4-02-sa2-godot.md). It is filled in as results arrive: an item that has not run is listed as open, never estimated. Raw logs and run records stay private; only sanitised summaries are published.

| Field | Content |
|---|---|
| ID | E-2.4-02 |
| Question | Can Godot 4.7 (.NET) host the S-B drawing method through its own Direct3D 12 RenderingDevice inside the selection gate, and what does it constrain (H-09)? |
| Decision it feeds | The day-7 go/no-go (E-2.4-04), the selection (E-2.4-05), and PR #43 section 7 (the framework paper's interop questions). |
| Hypothesis | Level 1: a texture written by our own D3D12 code on Godot's device, imported with `texture_create_from_extension`, displays correctly if it is handed back in the state Godot's tracker holds; RENDER_TARGET is required only where that tracker expects it. |
| Method | Level 1 smoke test: the producer DLL in [native/](native/README.md) (packet [SA2-N](../renderer-sa2-packets/SA2-N-native.md)), loaded by the Godot C# harness (packet [SA2-G](../renderer-sa2-packets/SA2-G-godot.md)). |
| Time box | Window days 3–6 (stage days 5–8), shared with E-2.4-03. |
| Kill criteria | Level 1 not passed by the end of window day 6. |
| Evidence class | Actual Windows/DirectX (the native producer self-test and the Godot smoke test, below), valid only for the source and build stated. No performance evidence. |
| Result | Level 1 passed on the owner's GPU for the build stated below: the producer's sequence-numbered texture was shown correctly through Godot's own D3D12 backend on Godot's device, on its queue and on the producer's own queue, through resizes and teardown, with 0 validation errors. Handing textures over in RENDER_TARGET state is wrong (question 1 below). |
| Decision | Open: level 2 (geometry port) may start; the day-7 go/no-go is an Astra gate ruling across all candidates. |

## Native producer self-test (3 October 2026)

The DLL writes a sequence-numbered code image (layout: [code_layout.json](code_layout.json)) into textures on a device and queue it is given, on that queue or on its own queue with fences, and hands each texture back in a stated state. `sa2_selftest.exe` stands in for Godot: it creates the device and a consumer queue, reads every produced frame back and decodes both code corners.

| Mode | Result |
|---|---|
| `--cpu` (no device) | pass, 11 of 11 checks: ABI, all 19 exports, invalid arguments, error-text truncation, CRC and code vectors, exact images at three sizes, decode rejection, text masking, state tables, JSON writer |
| `--warp --debug` | pass, 10 of 10 |
| `--hardware --debug` (RTX 4070 Laptop GPU) | pass, 10 of 10 |

- Each GPU mode runs all eight combinations of same or own queue, legacy or enhanced barriers, and created or external (Godot-like typeless, render-target, black clear value) textures. Each runs 300 frames on three slots with a rebuild from 256 × 192 to 320 × 240 before frame 150, decodes both corners on every consumer readback, verifies every texel every 50 frames, and detects one deliberate mismatch. With the debug layer, error and corruption counts are 0 and the only warnings are the expected mismatching-clear-value warnings of external textures.
- Two child probes per GPU mode: a drain that cannot be confirmed ends the child with exit code 3 and the exact one-line report, running no later code; `ID3D12Device5::RemoveDevice` makes the drain return `SA2_E_DEVICE_REMOVED` with a failing removed-reason HRESULT, after which releases and detach succeed.
- The first owner-machine run found three defects in the implement call's version (call `20261003T143409Z-24c43648`), fixed before these results:
  1. On the NVIDIA adapter, `ClearRenderTargetView` with more than 32 rectangles in one call ended the process with `0xC0000409` inside the call. Bisected with an instrumented copy: 33 rectangles fail at the first call, 32 run; WARP accepts 100. The producer now clears the code blocks in batches of at most 32 rectangles.
  2. With its own queue, the producer enqueued a GPU wait for fence value 0 for a slot never shown, which the debug layer warns about; the wait is now skipped.
  3. Application Control on this machine refuses unsigned executables in the system temp folder, so the build and its check moved under the repository's ignored `work/` tree.
- The Codex review of the fixed candidate (review `20261003T154728Z-36c0c10e`, finding SA2-N-001, major) found that one resource could be registered in two slots. Fence history is kept per slot, so with its own queue the producer could write the resource through one slot while Godot still read it through the other. Registration now refuses a resource already registered in another slot, and the self-test checks the refusal in every configuration.
- Build: MSVC 19.51.36260 (Visual Studio Community 2026), Windows SDK 10.0.26100.0, CMake 4.4.3, Ninja 1.13.2; Release, static CRT. GPU: RTX 4070 Laptop GPU, NVIDIA driver 616.92. Commit: recorded with the integration commit.

## Godot smoke test (level 1), 3 October 2026

Harness at commit `113a1b5` (source digest `a4bf57a9...`, matching HEAD), producer DLL `77637425...`, C# assembly `47193b84...`; Godot `4.7.2.stable.mono.official.ed1daf0bf` (editor build) on its D3D12 driver with enhanced barriers; RTX 4070 Laptop GPU, NVIDIA driver 616.92. Public summaries: [sa2-smoke-summary.json](results/sa2-smoke-summary.json) and [sa2-blank-control-summary.json](results/sa2-blank-control-summary.json). All 14 runs were as expected. The run matrix and the judging rules are in [README.md](README.md).

| Run | What it tests | Result |
|---|---|---|
| R0 | producer self-test before the matrix | pass |
| R1 | Godot compute writes the image (baseline, no producer writes) | pass |
| R2, R3 | export: the producer writes textures Godot created (zero copy), on Godot's queue or its own | pass |
| R4, R5 | import and copy: Godot copies from textures the producer created, on Godot's queue or its own | pass |
| R6 | an imported texture shown through `Texture2DRD` | unsupported, as expected: the editor build refuses the shared view |
| R7 | export handed over in RENDER_TARGET, without the warm-up | recorded: 312 validation errors (IDs 1332, 1334), 3 of 300 frames decoded wrong |
| R8 | import handed over in RENDER_TARGET, without the warm-up | recorded: 303 validation errors (ID 1334); all 300 frames decoded |
| R9 | export on the producer's queue, without validation, 3,000 frames | pass |
| R10 | export on the producer's queue, separate render thread | pass |
| R11 | the producer on legacy barriers while Godot uses enhanced barriers | recorded: 50 validation errors (ID 1350); all frames decoded |
| R12 | an unconfirmed drain | exit 3 with the exact one-line diagnostic and a pending result |
| R13 | `RemoveDevice` at run frame 200 | recorded: see question 3 below |

- Each passing run with 1,200 run frames decoded the produced sequence number from Godot's composited viewport on all 1,195 eligible frames; the other 5 run frames fell in resize transitions. R9 decoded 2,995 of 2,995. Every texel was compared every 50 frames, with no mismatch.
- With the debug layer on (`--gpu-validation`, confirmed active in each result), the passing runs had 0 errors and 0 corruption messages. Their warnings have two IDs, 820 (a render-target clear with a value other than the optimised clear value) and 1356 (non-optimal barrier batching); both also occur in R1, where the producer writes nothing.
- Each passing run resized the window five times, cycling through four sizes, and the harness rebuilt the texture ring each time. No Godot or producer call failed, the exported textures kept their native pointers while wrapped, and teardown confirmed the drain and detached. Each passing import run (R4, R5) released its 18 imported textures (3 slots x 6 ring generations) with refcount 0.
- R10 measured that the harness ran with a separate render thread. It printed one `ERROR:` line at shutdown, `This function (finalize) can only be called from the render thread.` The blank control printed the same line, with the same 1 error and 2 warning lines, in 3 of 3 separate-thread runs and in none of 3 safe runs, without the harness or the DLL. The line is Godot 4.7.2's own.

### Answers to PR #43 section 7 (Godot)

These are smoke-test results for the build above. Statements marked "source" come from Godot's source and were not tested separately.

1. **Import state.** An imported `ID3D12Resource` does not need to arrive in RENDER_TARGET state; it must arrive in the layout Godot's tracker holds for it.
   - Passed: one warm-up use per slot (Godot copies from it once, so its tracker holds the copy-source layout), then every handover in that layout (R4, R5: 0 validation errors).
   - Wrong: handing the same textures over in RENDER_TARGET gave 303 validation errors (R8); for textures Godot created, 312 errors and 3 wrongly decoded frames (R7).
   - The producer must also use Godot's barrier API. On this GPU that is enhanced barriers: legacy barriers in the producer gave 50 errors (R11).
   - Source: Godot starts an imported texture at layout UNDEFINED, so its first use discards the contents; the warm-up happens before the producer writes.
2. **Wait and Signal placement.** In the render step of frame f, on Godot's render thread (`RenderingServer.CallOnRenderThread`) and before Godot draws frame f:
   1. signal `free = f - 1` on Godot's queue;
   2. the producer writes slot f mod 3, on Godot's queue, or on its own queue after waiting until Godot has finished the frame that last showed the slot;
   3. it signals `ready = f`, and Godot's queue waits for `ready = f`.

   With this order every eligible frame showed the produced sequence number, on Godot's queue (R2, R4), on the producer's queue (R3, R5, R9) and with a separate render thread (R10). Nothing was timed, so the smoke test says nothing about stalls.
3. **Device, queue, zero copy, resize and device loss.**
   - The C# host got Godot's device and command queue through `RenderingDevice.GetDriverResource` in the render-thread step of the first `_Process` iteration, before Godot drew that frame. The producer checked that the queue is a direct queue of that device on the reported adapter, and worked on them in every run.
   - Zero copy works for textures Godot creates: the producer wrote into them and Godot showed them through `Texture2DRD` (R2, R3, R9, R10). In the editor build an imported texture can only be shown through a copy (R6). Source: release export templates skip the check that refuses it; not tested.
   - The resizes invalidated no RID: the harness frees and recreates its ring at each new size by design, and no call on the old ring failed before that.
   - Device loss (R13): after `RemoveDevice`, Godot kept the process running. Its next buffer creations failed with `DXGI_ERROR_DEVICE_REMOVED` (4 `ERROR:` lines), the harness recorded a failed Godot call, and the producer's drain returned `SA2_E_DEVICE_REMOVED`. Teardown completed and the process exited with code 0. 198 of the 199 eligible frames decoded; the readback of the last frame before the removal, requested in the same step just before `RemoveDevice`, returned an older frame's code. The harness stops after a loss, so recovery is untested.
4. Qt: see the S-D result card (`work/experiments/renderer-sd/RESULT.md`).
5. to 8. Not tested by this smoke test: PIX and RenderDoc captures, package size (no export template was built), screen readers, high-DPI math rendering.
9. Not tested: the smoke test has no timing; the gate with interop overhead needs the geometry port (level 2) and gate runs.

## Renderer constraints (H-09), 10 October 2026

What hosting the S-B drawing method in Godot constrains, for the design track (due on day 8). Our DLL draws the 3D view with Direct3D 12 on Godot's device into a texture Godot created, and Godot shows it in a full-window `TextureRect`; Godot never sees the DLL's geometry, depth buffer or shaders. "Source" means Godot 4.7.2's source ([FRAMEWORK-FACTS.md](../renderer-l2-packets/FRAMEWORK-FACTS.md), ID given), the DLL or the [level 2 plan](../renderer-l2-packets/PLAN.md). "Measured" means the owner's level 1 runs of 3 October or the level 2 runs of 4 October for build `709a24eb…`, whose loaded-build binding is still open (L2-V-002, [level 2 card](../renderer-l2/RESULT.md)). This list adds no measurement and says nothing about W5 or the final look.

| Area | Constraint | Evidence |
|---|---|---|
| Overlay layers | The 3D view is one flat layer: Godot controls and effects go above or below it, never between stickers. | source (plan section 2) |
| | With nothing over it, the composite showed the producer's texture correctly on every checked frame. | measured (level 1) |
| | Level 2 drew nothing over the 3D view, so the cost and correctness of UI layers over it are not known. | not verified |
| Text in the 3D view | The hosted method draws stickers only; it has no text pass. Godot text sits above the 3D view, so stickers cannot hide it. | source ([S-B SPEC](../renderer-sb/SPEC.md) section 3) |
| | Text tied to a sticker needs that sticker's screen position every frame; no ABI 2 function returns one (`sa2_interop.h`). | source |
| | Level 2 runs with content scaling disabled (the finalizer requires it). Scaled UI text beside a native-pixel 3D view was not tried. | source; not verified |
| Transparency and sorting | Opaque only: depth test on, blending off, no MSAA. The DLL clears to an opaque background, so the 3D view hides anything below it. | source (S-B SPEC section 3, [DLL README](native/README.md) "Reuse and record choices") |
| | Per-sticker transparency (alpha 0.6, all 259,800 centres sorted back to front on the GPU every frame) exists only in the S-B probe; the hosted path omits features. Its cost in Godot (H-06) is not measured. | source; not verified |
| Shaders | S-B's unchanged HLSL, compiled offline by DXC to DXIL; no runtime compiler; the blob digests enter the build identity. Stated exception: no SPIR-V for RenderingDevice, because Godot never sees these shaders and one port serves both candidates. The Astra plan check found the exception admissible. Level 1's GLSL compute baseline is a diagnostic exception only. | source (plan section 2; [README](README.md) "Frame, ownership and shutdown protocol") |
| Frame pacing | `--disable-vsync` presents with sync interval 0 and tearing allowed (G5). The level 2 W3 runs measured a pooled 670.71 fps with p99 1.887 ms, the opaque baseline without any H-06 feature. | source; measured (level 2) |
| Resize | A size change needs a drain, a scene reload and a ring rebuild in the DLL; level 2 refuses one. Level 1 passed resizes without a scene, and no RID was invalidated. | source; measured (level 1) |
| Full screen | Exclusive full screen gives the exact monitor rectangle; ordinary full screen overhangs it by 2 pixels (G1). | source |
| DPI | The process is system DPI aware, not per-monitor aware (G10): on a monitor whose DPI differs from the system DPI, Windows scales the window and the native-pixel check fails. | source |
| Colour format | RGBA8 UNORM, matching the textures Godot creates. | source (DLL README "States, references and fence protocol") |
| Zero copy | Only for textures Godot creates (level 1 runs R2 and R3). In the editor build an imported texture is refused and can be shown only through a copy (R6). | measured (level 1) |
| Measured setup | 2560 x 1600 at 60 Hz, the 3D view filling the window; peak VRAM 217.2 MB against a budget of about 7 GB. | measured (level 2) |

The design track can rely on: the full-detail opaque baseline at native resolution meeting the W3 gate with wide headroom (with the L2-V-002 qualification); UI and text as layers above or below the 3D view, not inside it; one set of DXIL shaders shared with Qt; VRAM far below the budget.

Not known yet: the cost and correctness of UI layers, translucent panels and text over the 3D view; sticker-anchored or depth-correct text; transparency with sorting and every other H-06 feature; a 3D view smaller than the window, resize with a scene, DPI-scaled UI or a per-monitor DPI change; HDR or 10-bit output; release export templates; picking (H-05).

## Acceptance items (packet section 1)

| # | Item | Status |
|---|---|---|
| 1 | Interop smoke test | Done for the build stated: R1 to R5, R9 and R10 passed on the owner's GPU. |
| 2 | Geometry port | Done: the level 2 app hosts the S-B method through the shared DLL ([level 2 card](../renderer-l2/RESULT.md)). |
| 3 | Three cold W3 runs | Measured on 4 October for build `709a24eb…`: pooled 670.71 fps, p99 1.887 ms, verdict `met`; loaded-build binding open (L2-V-002). |
| 4 | Layout specification and feature list | Not started. |
| 5 | Renderer constraints (H-09) | Delivered on 10 October (above). |

## Not claimed

- No timing was measured: nothing here says whether the interop stalls or what it costs.
- Nothing here holds for release export templates, another Godot version, a GPU without enhanced barriers, another build, driver or machine.
- Recovery after a device loss is untested.
