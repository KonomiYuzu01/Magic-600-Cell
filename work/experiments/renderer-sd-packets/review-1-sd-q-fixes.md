# Review packet: Claude's fixes to the SD-Q Qt smoke-test harness

Review only; do not perform follow-up work.

`python tools/agents/codex_review.py --kind review --model gpt-6.1-sol --effort max --speed fast --packet work/experiments/renderer-sd-packets/review-1-sd-q-fixes.md`

## 1. Goal and acceptance
- Goal: review Claude-authored changes to the SD-Q Qt Quick smoke-test harness before it is committed and run for the record on the owner's machine.
  - Codex wrote the harness under packet `SD-Q-qt.md` (implement call `20261003T164526Z-c90c7e87`).
  - Two Claude pre-reviews of that output found SDQ-P-01 to P-08 (runner and summary) and SDQ-C-01 to C-10 (C++). A private shakedown run on the owner's machine then found SDQ-R-01. Section 2 lists them with Claude's disposition. Claude made the fixes.
  - This is non-critical tooling and probe code, so it gets this one Sol review.
- Acceptance: one JSON result matching `schemas/review-result.schema.json` that answers three questions:
  1. Does each fix do what section 2 says, without departing from the packet's pass and judging rules (`SD-Q-qt.md`, "A run passes when:" at line 317, and "Judging" at line 381), except for the one stated deviation (SDQ-C-08)?
  2. Are the four runtime fixes correct for Qt 6.10.3: SDQ-C-05 (the QRhi read through the renderer interface in `sceneGraphInitialized`), SDQ-C-06 (shutdown no longer waits for `renderDone`), SDQ-C-08 (imported slots start in the producer-before state) and SDQ-R-01 (new Qt render targets cleared once at each ring creation)?
  3. Is there a defect in the changed code that would make a judgement or the public summary wrong? That means a false pass, a false failure, a privacy leak in `results/`, or a run that cannot start or hangs. Give a concrete counterexample: a result JSON, an output line or a file state, then the wrong outcome.
- Verdict `pass` when nothing reaches `major`.

## 2. Actual problem and reproduction
The pre-reviews read an early snapshot of the implement output; the fixes were applied to the final output (`work/reviews/20261003T164526Z-c90c7e87/changes.patch`).

Runner and summary (`run_smoke.py`, `smoke_summary.py`, `check_project.py`):
- **SDQ-P-01 (blocker), adopt.** `build_command` returned a list; `Popen` joined it with `list2cmdline`, which escapes the inner quotes as `\"`, and cmd.exe then fails to find the script. Fix: the string form SA2 uses, `cmd.exe /d /c call "<script>" "<destination>"`, with a fixture that the string has no `\"`.
- **SDQ-P-02 (major), adopt.** A recorded row (Q10, Q12) was as expected on exit 0, status `recorded` and the adapter name alone, although a probe always reports `recorded` with exit 0, even after a setup failure with zero frames. Fix: a recorded row also needs `frames.run == frames`, a complete teardown (phase `complete`, drain confirmed, detached) and reasons only from `RECORDED_REASONS` (observations: `debug-errors`, `decode-mismatch`, `verify-mismatch`, `unverified-frames`, `eligible-coverage`, `readback-missing`). Six fixtures cover the rejected variants. This also covers SDQ-C-09.
- **SDQ-P-03, adopt.** The source identity's `git diff --quiet HEAD` did not exclude what the file walk excludes (`results/`, build output), and the walk did not exclude leftover `native/build-check-*` folders. Fix: SA2's `excluded()` and `diff_pathspecs()`, Git with `--no-optional-locks`, and a fixture in a temporary repository showing that changes under excluded folders are ignored and source changes are seen.
- **SDQ-P-04, adopt.** Q0 required every self-test check to be `pass` and did not check the format. Fix: SA2's `selftest_status` (every check `pass` or `unsupported` with a reason, at least one `pass`, the self-test format, no failing overall status).
- **SDQ-P-05, adopt.** The self-test JSON has no `status`, so Q0's published status was null. Fix: the summary derives it with `selftest_status`.
- **SDQ-P-06, adopt.** The check that the pending result is written before the drain could not fail. Fix: `0 <= pending < writeResult(false) < releaseRing();`, all searched from the pending phase. A planted move of `writeResult(false)` after `releaseRing();` now fails the check.
- **SDQ-P-07, adopt.** Without a console window Qt logs to OutputDebugString, so the stderr phrase counts would read 0. Fix: `QT_FORCE_STDERR_LOGGING=1` in the run environment.
- **SDQ-P-08, adopt.** An inherited `VCINSTALLDIR` makes windeployqt add the VC redistributable installer, inflating the recorded deployment size. Fix: `deploy_environment()` removes it for windeployqt only.

C++ application (`app/`):
- **SDQ-C-01 (blocker), adopt.** `<rhi/qrhi.h>` is installed only under `include/QtGui/6.10.3/QtGui/rhi/`, which only `Qt6::GuiPrivate` adds. Fix: CMake finds and links `Qt6::GuiPrivate`; `check_project.py` checks the new lines. The packet's "linked through Qt6::Gui" is wrong for 6.10.
- **SDQ-C-02 (blocker), adopt.** 6.10.3 has no `<rhi/qrhid3d12.h>`. Fix: `<rhi/qrhi_platform.h>`, which declares `QRhiD3D12InitParams` and `QRhiD3D12NativeHandles`.
- **SDQ-C-03 (blocker), adopt.** `QRhiD3D12NativeHandles::dev` is `void *`. Fix: `static_cast<IUnknown*>` before `QueryInterface` and `sameDevice`.
- **SDQ-C-04 (blocker), adopt.** A local variable named `interface` meets `combaseapi.h`'s `#define interface struct`. Fix: renamed to `rif`.
- **SDQ-C-05 (blocker), adopt.** In the threaded loop, `sceneGraphInitialized` is emitted inside the render context's `initialize()`, before the render thread stores the QRhi in the window, so `QQuickWindow::rhi()` is still null there (qtdeclarative 6.10.3 `qsgthreadedrenderloop.cpp`, the `sgrc->initialize()` call before `cd->rhi = rhi`). Every threaded row would have failed with `rhi-create-failed`. Fix: `rhi_` comes from `rendererInterface()->getResource(window, QSGRendererInterface::RhiResource)`, which reads the render context's QRhi, set before the signal. The renderer interface is taken first, so the loss path reads the same source.
- **SDQ-C-06 (blocker), adopt.** In the basic loop (Q8), `hide()` and `releaseResources()` release nothing; only window deletion invalidates the scene graph. The timer waited for `renderDone`, which therefore never came, and Q8 would hang to the runner's timeout. Fix: after `close()` and `releaseResources()`, the timer finalizes at once. Deleting the window runs any remaining teardown (the renderer's destructor or `sceneGraphInvalidated`) and joins the render thread, before the application's QRhi, queue and device are released. In the threaded loop, `close()` already ran the teardown synchronously.
- **SDQ-C-07 (major), reject_with_evidence.** Concern: route A lets Qt pick DXGI adapter index 0, which on a hybrid laptop may be the integrated GPU. Evidence: on the owner's machine `IDXGIFactory1::EnumAdapters1` lists index 0 = NVIDIA GeForce RTX 4070 Laptop GPU and index 1 = Microsoft Basic Render Driver; the integrated GPU is not enumerated in the current display mode. If the order changes, `--expect-adapter` judges every route-A row unexpected, which is a visible failure, not a false pass.
- **SDQ-C-08 (minor), adopt; a stated deviation from the packet.** The packet gave Q10 (declared handover) the initial state `RENDER_TARGET` with producer-before `COPY_SOURCE`, so the DLL's first produce on each slot records a `COPY_SOURCE -> RENDER_TARGET` barrier on a texture actually in `RENDER_TARGET` (`sa2_interop.h`: the produce barrier starts from `state_before_write`). That would put debug-layer errors of the producer's own making into the Q10 observation. Fix: every imported slot is created, and wrapped with `createFrom`, in `state_before_write`; the result's `dll.initial_state` reports the same. For the tracked rows `before == after`, so nothing changes there. The README states the deviation.
- **SDQ-C-09 (minor), adopt through SDQ-P-02.**
- **SDQ-C-10 (minor), adopt.** The raw pointer comparisons `device == handles->dev` and `device == suppliedDevice` were dropped; device identity is COM identity (`sameDevice`) only. Queue identity stays a pointer comparison of the same interface type.

Private shakedown run on the owner's machine (`run_smoke.py --device-loss`, no summary; section 4):
- **SDQ-R-01 (major), adopt.** Q1 to Q4, Q6, Q7 and Q9 failed on `debug-errors` alone, D3D12 debug-layer error 1422: a resource "with D3D12_HEAP_FLAG_CREATE_NOT_ZEROED flag with either render target or depth stencil flags must be initialized with a Discard/Clear/Copy operation", here "not initialized but is used in" `SetGraphicsRootDescriptorTable`. Qt allocates render targets without zeroing.
  - On a ring (re)build step, `renderStep` returned before writing the `QQuickRhiItem`'s new color buffer, and the scene graph sampled it on that step: one error per new color buffer, 6 per resizing row (the first ring and five resizes) and 1 in each recorded row. The direct routes (Q5, Q8) have no item color buffer and had 0 errors.
  - On the export route the warm-up copied Qt-created slot textures that nothing had written yet: 18 more errors in Q4 (3 slots × 6 rings).
  - Everything else in those rows passed: every composite, texture and native verification, identity, resize, teardown and refcounts. Q11 (Q7's route without the debug layer, 3,000 frames) passed.
  - Fix (`smoke.cpp`, `smoke.h`, `main.cpp`): the rebuild step, after creating the ring and before anything reads it, clears the item's new color buffer with one empty pass on `renderTarget()` and, on the export route, each new slot texture with one empty pass on its own `QRhiTextureRenderTarget` (created with the slot, destroyed in `destroyWrappers` before the texture). `renderStep` takes the item's render target (`nullptr` on the direct route).
  - The clears use the optimized clear values Qt 6.10.3 declares for render-target textures: color all zero (`qrhid3d12.cpp`, `QD3D12Texture::create`: a zero-initialized `D3D12_CLEAR_VALUE` passed when the texture is a render target) and depth 1, stencil 0. Shakedown 3 cleared with opaque black and Q1, where the producer writes nothing, then counted 6 clear-value warnings (ID 820), one per clear; the current files clear with `Qt::transparent`.
  - Unchanged: the judging rules and the frame accounting. The export state contract is also unchanged: the warm-up copy still leaves Qt's tracker at `COPY_SOURCE`, the producer-before state. The README states the clears.

## 3. Environment and versions
- Windows 11, CPython 3.14.7 64-bit.
- Qt 6.10.3 `msvc2022_64`, installed through the reviewed toolchain installer at `tools/qt/6.10.3/msvc2022_64`.
- MSVC 19.51 (x64), Windows SDK 10.0.26100.0, the pinned renderer-spike CMake and Ninja.
- RTX 4070 Laptop GPU, NVIDIA driver 616.92.
- The review sandbox is read-only, without GPU, Qt, compiler or network.

## 4. Necessary source and evidence
- Delta: `work/experiments/renderer-sd-packets/sd-q-fix-delta.patch`. It runs from the implement output to the current files:

  | File | Before | Now (sha256) |
  |---|---|---|
  | `run_smoke.py` | `faeb0177…` | `c05d88cc4a2aa2ec2f9d34be5469102fccf336878e92364c85606389aecd89e4` |
  | `smoke_summary.py` | `31e55fb7…` | `ae472bc13b617a2fc08c97ea679dcfd93a70993cb20a9ff6230bcd3d8e18a7ea` |
  | `check_project.py` | `3e1f5173…` | `7494e18b9be6e489a1b126bd2bd965ee2121e6a7405a7d1102f0d70bb380eff4` |
  | `README.md` | `9f5e7441…` | `0117bb9c259c54ab06aa308b8c9709df2e0f482118c595002d50e7b18d02b7ca` |
  | `app/CMakeLists.txt` | `3bb7ef0d…` | `c6b67562f1bd46fea6f3224c1811d540a2d0a4d6ef958397aaeb8e82039b3b62` |
  | `app/src/main.cpp` | `d704a329…` | `00e3cc25e18970a9bc0cadcec980b5ee033ac394a44ee03a82fe7a165dec8d2a` |
  | `app/src/smoke.h` | `6a05650a…` | `c5fc8ccba10fbfbdf2d69f6d10335ab93406e752cc7efdfde8fecd7faacb41f2` |
  | `app/src/smoke.cpp` | `24bb2adb…` | `daf88154849a7f5a19e2d24b517dc42ca200b4769ad51cfc6722709320bce592` |

- Every other harness file is byte-identical to the implement output.
- The files are under `work/experiments/renderer-sd/`. `work/` is ignored and these files are not committed yet, so read them directly; `git diff` does not show them. The packet is `work/experiments/renderer-sd-packets/SD-Q-qt.md`; the producer ABI is `work/experiments/renderer-sa2/native/include/sa2_interop.h`.
- Checks run on the owner's machine:
  - `python work/experiments/renderer-sd/check_project.py` passes ("source/static and Python fixtures").
  - The application builds against the installed Qt 6.10.3 with `build.cmd`. Before the fixes the builds stopped at C1083 for `<rhi/qrhi.h>`, then C1083 for `<rhi/qrhid3d12.h>`, then C2737/C2227 at `actual->dev` in `main.cpp` and syntax errors at `interface` in `smoke.cpp`.
  - Private shakedown runs (`run_smoke.py --device-loss`, no summary; results under the ignored `work/loop-memory/`, not available to the reviewer):
    1. Shakedown 1 built the native DLL, the application and the windeployqt deployment without errors and wrote the build identity. Q0 then could not start: Windows Smart App Control refused the freshly built, unsigned `sa2_selftest.exe` (WinError 4551; a per-hash cloud reputation verdict, not a harness defect). No row ran on the GPU. The owner then turned Smart App Control off.
    2. Shakedown 2 ran all 17 rows on the RTX 4070 Laptop GPU, with the code before the SDQ-R-01 fix.
       - Q0, Q0b, Q5, Q8 and Q11 passed. Q10 and Q12 to Q15 were as expected. Q1 to Q4, Q6, Q7 and Q9 failed on `debug-errors` alone (SDQ-R-01).
       - SDQ-C-05: every threaded row got its QRhi and attached.
       - SDQ-C-06: Q8 (basic loop) ended without a timeout, with a complete teardown.
       - SDQ-C-08: Q10 (declared handover) recorded only the one SDQ-R-01 error and no barrier error.
    3. Shakedown 3, with the SDQ-R-01 fix but clearing with opaque black: all 17 rows as expected.
       - Q1 to Q9 and Q11 passed, with 0 debug-layer errors and 0 corruption messages.
       - Q10 and Q12 recorded 0 errors. Q13 exited 3 with a pending result. Q14 and Q15 recorded the device loss.
       - The only warning ID was 820. Q1, where the producer writes nothing, had 6, from the black clears. The copy and direct rows had about 6,880 per 1,200 frames, from the producer's code-image clears: its textures have no optimized clear value. Q5, which has no harness clear, shows the same count.
    4. Shakedown 4 (`--only Q1,Q2,Q4`, fresh build), with the current files: Q0, Q0b, Q1, Q2 and Q4 as expected; Q1, Q2 and Q4 passed with 0 debug-layer errors. Q1 had no warning at all; Q2 and Q4 had only the producer's ID 820 warnings (6,875 and 6,882).

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | SD-Q as implemented by Codex | implement call `20261003T164526Z-c90c7e87` | its acceptance check; two Claude pre-reviews | check passed; SDQ-P-01 to P-08 and SDQ-C-01 to C-10 found |
| 2 | the SDQ-P and SDQ-C fixes in section 2 | Claude-authored | the checks in section 4, shakedowns 1 and 2 | shakedown 2 found SDQ-R-01 |
| 3 | the SDQ-R-01 fix | Claude-authored | shakedown 3 | see section 4; this review |

## 6. Constraints and owned files
- Read-only review. Do not build, start Qt or do GPU work, and run nothing from `run_smoke.py` except `--dry-run`.
- `check_project.py` creates a fixture directory and a temporary Git repository in the system temp folder. If the sandbox refuses that, say so and judge from the source.
- In scope: the delta above and its effect on judgements, on the result file and on the public summary.
- Out of scope:
  - the producer DLL under `work/experiments/renderer-sa2/native/`, reviewed separately (SA2-N);
  - Codex-authored code the delta does not touch, except where a fix depends on it;
  - `minor` and `nit` findings, and style.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`, with finding IDs `SD-Q-F-01` and up.
- Review only; do not perform follow-up work.
