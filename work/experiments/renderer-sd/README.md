# SD-Q Qt Quick interop: levels 1 and 2

This is E-2.4-03 level 1: a synthetic sequence-numbered code image, displayed by
Qt Quick's Direct3D 12 backend. It implements packet SD-Q against the committed
SA2 producer ABI 2 and immutable `../renderer-sa2/code_layout.json`. It loads the
DLL by absolute path and binds all 27 functions declared by the header, checking
the ABI before any other call and asserting the four struct sizes. The level 1
defaults, synthetic image path, runner and result format are retained. It uses
C++ items, dynamic Qt linking and no QML, session or database. Level 2 selects
the DLL's S-B scenes with `--l2-mode`; its contract is
[`HARNESS.md`](../renderer-l2-packets/HARNESS.md).

Codex wrote the harness in a sandbox without Qt or a GPU; statements below
marked source-verified come from Qt 6.10.3's source. The owner-machine results
for the build named there, including the runtime checks listed below, are in
[RESULT.md](RESULT.md). No performance was measured.

## Owner-machine commands

Prerequisites: Windows 11, x64 MSVC C++ tools and Windows SDK; CMake 3.30 or newer
and Ninja (the build prefers the pinned renderer-spike tools); Qt **exactly
6.10.3**, `msvc2022_64`, already installed at
`tools/qt/6.10.3/msvc2022_64`. The Qt toolchain entry needs the owner's existing
installer approval before installation. This harness installs or downloads
nothing. The Windows Direct3D debug layer must already be available.
The owner's MSVC 14.51 build will check compatibility with Qt's MSVC 2022 binaries.
Qt is dynamically linked under the spike's LGPL choice; a deployed directory is
private measurement output, not a release package.

Run from the repository root, on an **idle machine**, never during PresentMon
captures, builds or Codex implementation calls:

```powershell
python work/experiments/renderer-sd/check_project.py
python work/experiments/renderer-sd/run_smoke.py --dry-run
python work/experiments/renderer-sd/run_smoke.py
python work/experiments/renderer-sd/run_smoke.py --only Q2,Q6
python work/experiments/renderer-sd/run_smoke.py --device-loss
python work/experiments/renderer-sd/run_smoke.py --skip-build --only Q2,Q6
python work/experiments/renderer-sd/run_smoke.py --write-summary
```

`--only` always includes Q0 and Q0b first. Q14 and Q15 require `--device-loss`,
including when requested through `--only`. `--expect-adapter` defaults to
`NVIDIA GeForce RTX 4070 Laptop GPU`; another value declares the expected adapter
for a separate smoke test. It does not change adapter selection. Qt's own-device
route uses Qt's adapter choice; imported-device routes select DXGI's first
high-performance adapter. The probe finds the adapter matching the actual device.
No LUID or native pointer is written to a result.

The runner checks `qmake -query QT_VERSION`, records HEAD, the sorted source-path
and file-digest aggregate, and whether the walked source files equal Git's tracked
files and have no diff against HEAD. Ignored, untracked source makes
`matches_head` false. Results, build directories and bytecode are excluded. It
checks source identity again after the build and before each run. Every reused
binary, self-test and deployed file must match the stored digest manifest.

Builds live in `work/sdb/<UTC stamp>/native`, `/app` and `/deploy`, using the same
stamp as the private output folder. The runner refuses a native build path over
150 characters. Use a short checkout path on the owner's machine: the nested
implementation worktree is intentionally unsuitable for MSVC try-compile paths.
The SA2 build uses its unchanged `native/build.cmd`. Qt deployment uses:

```text
windeployqt --release --no-translations --no-system-d3d-compiler --no-opengl-sw --no-quick-import --dir <build>/deploy <build>/deploy/sd_smoke.exe
```

The deployment contains no SA2 DLL; its byte total and file count measure the
minimal preliminary Qt package. All application runs start the deployed executable.
For a manual build, supply a short output directory:

```powershell
& work/experiments/renderer-sd/build.cmd work/sdb/manual/app
```

## Matrix and what its results mean

| Run | Device / display / queue | Other settings | Expected observation |
| --- | --- | --- | --- |
| Q0 | SA2 native hardware self-test | Debug layer | Pass; stop the matrix on failure |
| Q0b | C++ code-layout fixture, no Qt | CRC, code vectors, corrupt blocks, both formats | Pass; stop on failure |
| Q1 | Qt / RHI upload / same | Threaded, debug, 1200 frames, resize every 200 | Pure Qt baseline |
| Q2 | Qt / import-copy / same | Tracked, legacy, threaded, debug, 1200/200 | Qt device and queue, imported `createFrom` ring |
| Q3 | Qt / import-copy / own | As Q2 | Two queues with free and ready fences |
| Q4 | Qt / export-copy / same | As Q2 | Qt-created native resources, optimized clear value and pointer stability |
| Q5 | Qt / import-direct / same | As Q2 | Zero-copy `QSGD3D12Texture` display through a texture node |
| Q6 | fromRhi / import-copy / same | As Q2 | Application QRhi/device/queue retained by Qt Quick |
| Q7 | fromRhi / import-copy / own | As Q2 | Imported device plus separate producer queue |
| Q8 | fromRhi / import-direct / same | Basic loop, debug, 1200/200 | The basic-loop zero-copy case only |
| Q9 | Device only / import-copy / same | As Q2 | Device-only fallback; Qt creates its own queue on that device |
| Q10 | Qt / import-copy / same | Declared, legacy, debug, threaded, 300/0 | Recorded state-tracker probe |
| Q11 | fromRhi / import-copy / own | Tracked, legacy, debug off, threaded, 3000/500 | Longer resize and lifetime smoke test |
| Q12 | Qt / import-copy / own | Tracked, match, debug, threaded, 1200/0 | Recorded enhanced-producer / legacy-Qt probe |
| Q13 | fromRhi / import-copy / own | Tracked, legacy, debug, threaded, 120/0; injected drain | Exit 3, exact DLL stderr line, pending result |
| Q14 | Qt / import-copy / own | Legacy, debug, threaded, 400/0; remove at run frame 200 | Optional bounded device-loss observation |
| Q15 | fromRhi / import-copy / own | As Q14 | Optional lost application-QRhi observation |

Q1–Q9 and Q11 expect `pass`. Q10 and Q12 expect `recorded`, even when diagnostics
or mismatches are observed. Q13 sets `M600_SA2_INJECT_UNCONFIRMED_DRAIN=1` only in
that child; inherited injection and debug-layer environment are cleared first.
All Qt runs use DPI scaling off, the selected render loop and Qt scenegraph/RHI
logging. Only Qt-owned-device debug rows set `QSG_RHI_DEBUG_LAYER=1`; imported
devices enable the native debug layer before creation. Other Qt installations
are removed from the run PATH, and plugin/import path overrides are cleared.

Each run is judged `as-expected` or `unexpected`, including an exact adapter-name
check for application rows. Q14 and Q15 are observational exceptions: any exit
within 120 seconds is as expected, even an abort or missing result. The private
summary retains that exit code, result existence and any returned observations;
this judgement establishes neither successful recovery nor display correctness.
Other run timeouts are 300 seconds. The runner kills only that run's PID tree and
records a timeout. It captures stdout/stderr to private files and counts all
stderr lines plus the six phrases named by SD-Q.

## Rendering and lifetime protocol

The window starts at 1280×720, position (40,40). One C++ item follows its size at
(0,0). Initialization fails if the backend is not Direct3D 12, the effective
device pixel ratio is not exactly 1, a device/queue identity differs, or a
requested debug layer is unavailable. Route B checks COM device identity and
queue identity before showing the window, then QRhi and renderer-interface
identities again on the render thread. Route C checks the imported device's COM
identity and the probe's queue-device identity. It supplies no queue.

All producer and post-window native D3D12 calls run on the render thread, except
the explicitly required GUI-thread release of application-owned objects after
window deletion. State shared with the GUI is protected by a mutex; Qt rendering
and DLL calls occur outside it. A queued `frameSwapped` handler requests the next
item/window update. A GUI timer consumes resize/close requests without a GPU wait.

Render-step fence numbers start at 1, continue through warm-up and rebuilds, and
select slot `f % 3`. On native routes every step signals free `f-1` on Qt's queue.
Run steps produce `(sequence=f, slot=f%3, generation)`, wait ready on Qt's queue,
mark shown, and copy or display the slot. Queue order places producer submissions
before Qt's current frame submission. The early free signal covers the previous
Qt frame; the ready wait holds Qt's current frame. The baseline instead uploads
the oracle image and attaches the DLL only for debug counters, without slots.

Imported copy textures start tracked at `COPY_SOURCE`; direct textures start at
`PIXEL_SHADER_RESOURCE`. Qt allocates render targets without zeroing, so every
ring creation first clears the item's new color buffer, which the scenegraph
shows on that step, and on the export route each new slot texture, once. The
clears use the optimized clear values Qt declares for its render targets (zero
color, depth 1, stencil 0), so they add no clear-value warning (ID 820). Without
these clears the debug layer reports error 1422 (render target read before
initialization). Export textures are warmed with three copy-only steps,
one per slot, after every ring creation. Qt creates them in `COMMON` (source);
the result records that state without reading it back from Qt. The steady
producer state is `COPY_SOURCE`. Export pointers are
checked each step. Every imported slot starts in the producer-before state.
Q10 declares the handover: producer-before `COPY_SOURCE`, producer-after
`RENDER_TARGET`, and `setNativeLayout(RENDER_TARGET)` after each production.
The packet's initial `RENDER_TARGET` for Q10 was not kept: it would start every
slot's first write from a state mismatch the producer causes itself. The
declared-layout handover is an observation, not a correctness claim. Qt's
barriers remain legacy in every run;
Q12 resolves the producer's `match` API from adapter support.

Every 50 run frames, native routes verify the producer slot with the DLL.
Copy routes and the upload baseline read back the color texture and compare
every RGBA texel with the expected image. No Qt readback changes a ring slot's
tracked state. Every eligible frame also requests the *current* composite in
`afterRendering`, using the swap-chain command buffer. Callbacks decode both
ring corners with the reported RGBA8/BGRA8 format. They capture the frame, ring
size and generation at request time. Results remain alive through completion,
and completed ticket storage is discarded at the next render step.

Rebuild and warm-up steps do not increment the requested run-frame count.
Eligibility requires a run frame in the current generation, a sufficiently
large back buffer and no pending resize. Every other rendered step is counted
as warm-up or transition. Resize cycles through 1280×720, 1024×640, 1440×810 and
800×600. On a size change, the render step finishes Qt work, removes the direct
texture node if present, destroys wrappers, drains the producer, unregisters
slots and releases imported resources. It increments generation modulo 4096,
creates the new ring, clears the new render targets and performs export
warm-up. Each requested resize records
the render-step count through synchronization, rebuild and warm-up.

On device removal the harness records DLL statuses/removal reason and Qt
invalidation, errors, reinitialization and resumed rendering for up to 10 seconds.
It never reattaches the DLL. `new_rhi_seen` is a conservative pointer-change
observation; an allocator reusing the same address cannot establish a new QRhi
from that field. Qt stderr is independent evidence. The probe then closes.

Shutdown closes the window on the GUI thread, with persistent scenegraph and
graphics resources disabled, and requests resource release. The RHI renderer's
destructor (or direct route's `sceneGraphInvalidated` handler) finishes Qt work,
destroys wrappers and atomically writes a **pending** result before draining.
It unregisters slots, releases imports, collects debug counters/messages and
detaches. After that callback, the GUI deletes the window, then the supplied
QRhi, queue and device, recording the counts returned by `Release`. The final
result is written atomically with `QSaveFile`'s temporary-file/rename commit.
An unconfirmed live-device drain exits 3 inside the DLL and leaves the pending
file. A removed device permits safe release but never reports a confirmed drain.

A normal pass requires the requested frame count, all eligible composites
verified, at least 90% eligibility outside warm-up/transitions, every readback
completed, zero mismatches, zero failed ABI calls, matching identities, and
complete, confirmed teardown with zero imported/application-owned reference
counts. With the debug layer enabled, error/corruption counts must be zero.
Warnings and distinct IDs are reported. The exit is 0 for pass/recorded, 1 for
failure, 2 for malformed arguments; only the DLL emits exit 3.

Qt's `finish()` and frame-slot waits can themselves wait without a time limit.
This experiment reports that limitation: the runner's process timeout bounds the
whole run, not Qt's wait internally. The threaded cases put GPU waits off the UI
thread; Q8's explicit basic-loop probe shares the GUI thread. This is evidence
for R-04, R-14 and R-17 evaluation, not a claim that Qt provides bounded recovery.

## Qt 6.10.3 source facts and runtime checks

These labels refer to the v6.10.3 source facts supplied in packet SD-Q. None has
yet been relabelled runtime-confirmed; the owner must run the listed rows.

| Source fact | Verification label and supplied fact | Runtime exercise, pending |
| --- | --- | --- |
| Q1 | **source-verified against 6.10.3**: graphics-device factories, no D3D12 device-plus-queue factory | Q6–Q9 |
| Q2 | **source-verified against 6.10.3**: fromRhi borrows; device-only creates a DIRECT queue; renderer-interface resources expose native handles | Q2, Q6–Q9 identities |
| Q3 | **source-verified against 6.10.3**: device2 query, silent fallback, imported device/queue borrowed, imported-device debug-layer ownership | Q6–Q9 identities and final reference counts |
| Q4 | **source-verified against 6.10.3**: createFrom borrows native resource, tracks layout, defers bookkeeping, setNativeLayout overrides tracking | Q2/Q3/Q6/Q7, Q10, import reference counts |
| Q5 | **source-verified against 6.10.3**: legacy barriers, fragment shader-resource state, RenderTarget flag and clear value | Q4 clear-value messages, Q12 probe |
| Q6 | **source-verified against 6.10.3**: native command-list handles, external-command behavior, external state must be declared | Q10 declares state; direct command-list injection is not used |
| Q7 | **source-verified against 6.10.3**: two frames in flight; endFrame submit/present/signal order; finish submits, waits and completes readbacks | Q2/Q3/Q7 ordering, resize and teardown |
| Q8 | **source-verified against 6.10.3**: threaded device-loss cleanup and application-owned recovery responsibility | Optional Q14/Q15 observations |
| Q9 | **source-verified against 6.10.3**: QQuickRhiItem RGBA8 transfer-source color buffer, recreated native resource on resize | Q1–Q4/Q6/Q7/Q9/Q11 resize |
| Q10 | **source-verified against 6.10.3**: renderer destruction on render thread while QRhi survives | All teardown rows, especially Q13 pending file |
| Q11 | **source-verified against 6.10.3**: setGraphicsDevice must precede scenegraph initialization | Q6–Q9 set before show |
| Q12 | **source-verified against 6.10.3**: native D3D12 texture wrapping for texture nodes | Q5/Q8 zero copy |
| Q13 | **source-verified against 6.10.3**: RHI item rendering is update-driven in beforeRendering | Q1–Q4/Q6/Q7/Q9/Q11 |
| Q14 | **source-verified against 6.10.3**: resize/begin/sync/render/end/afterFrameEnd order in threaded loop | Q2/Q3/Q7 readbacks and queue protocol |

Additional Qt APIs beyond packet section 3 are listed here for the integrator's
first Qt build. They are believed present in 6.10.3 because they are established
Qt 6 public APIs or the supporting QRhi operations for APIs expressly authorized
by the packet; their availability was not checked against installed headers here:

| Additional API use | Why expected in 6.10.3 |
| --- | --- |
| QObject::connect, direct/queued connections; QTimer timeout/start; QGuiApplication exec/exit/setQuitOnLastWindowClosed | Standard Qt Core/Gui application/event-loop API |
| QQuickWindow resize, position, width/height and change signals, show, update, close, releaseResources, setPersistentSceneGraph/setPersistentGraphics | Established QWindow geometry and QQuickWindow resource-lifetime APIs |
| QQuickItem setPosition/setSize, width/height, update, ItemHasContents/setFlag | Established C++ scenegraph item API needed to create the specified single item |
| QSGSimpleTextureNode setRect/setTexture/setOwnsTexture; QSGNode appendChildNode/removeChildNode/markDirty/DirtyMaterial | Existing texture-node and scenegraph ownership APIs; a container avoids null textures during rebuild |
| QRhiTexture::pixelSize; QRhiSwapChain::currentPixelSize/currentFrameCommandBuffer | QRhi resource-size and current-frame accessors needed for resize and afterRendering readback |
| QRhiTextureUploadDescription/Entry/SubresourceUploadDescription; QRhiReadbackDescription(texture); QRhiReadbackResult::data | Supporting descriptors and data access for packet-listed upload/readback APIs |
| QString conversions, QByteArray, QSize/QPointF/QSizeF, qRound, qEnvironmentVariable, QFileInfo::isAbsolute, QJsonObject/Array/Document/Value, QSaveFile and QIODevice::WriteOnly | Longstanding Core value, path-validation, JSON and atomic file APIs |

The integrator's first build against the installed 6.10.3 headers needed these
changes: QRhi's headers are semi-public, so CMake finds and links
`Qt6::GuiPrivate`; the Direct3D 12 native handles and init parameters are in
`<rhi/qrhi_platform.h>` (6.10 has no `qrhid3d12.h`); `QRhiD3D12NativeHandles::dev`
is `void *`; and `interface` is a `combaseapi.h` macro. From Qt source reading,
the threaded loop emits `sceneGraphInitialized` before `QQuickWindow::rhi()` is
set, so the harness reads the QRhi through the renderer interface; and the basic
loop releases nothing on hide, so shutdown deletes the window without waiting
for the renderer's teardown.

Not verified: Qt 6.11 or later; basic-loop behavior beyond Q8; recovery after
device loss with fromRhi or fromDeviceAndContext; native command-list injection;
geometry/label correctness, input/accessibility, long-session reliability,
performance or E-2.4-03 levels 2–5.

## Private output, public summary and acceptance

Private logs, per-run JSON, build identity and `private_summary.json` go to
`work/loop-memory/perf/renderer/sd-smoke/<UTC stamp>/`. Build/deployment output
stays under `work/sdb/`. Keep those private; they can contain absolute output/DLL
paths, native diagnostics and command context. Never commit them or the binaries.

Only `--write-summary` writes
`results/sd-smoke-summary.json` (`magic600-sd-smoke-summary-v1`), suitable for
review before committing. Its typed allowlist includes source/build identities,
Qt version, adapter name, formatted driver version, enhanced-barrier support,
deployed size and per-row judgements, frame/verification/identity/debug/teardown
counts, distinct IDs, stderr phrase counts and exits. Messages, error descriptions,
paths, user names, native addresses and vendor/device IDs are excluded. A final
privacy scan rejects suspicious strings before any public file is written.

`check_project.py` checks required source files, CMake and CRLF settings, every
ABI binding and header constant, immutable code vectors, both-corner decode and
corruption rejection, RGBA8/BGRA8 composites, dry-run matrices, probe judgements,
build-reuse integrity and planted private-text refusal/projection. Its fixtures
create a unique directory with plain `mkdir` under the system temp directory,
then remove it. All Python entry points disable bytecode before local imports.
Acceptance builds nothing, starts no Qt/GPU process and writes nothing in the
worktree, including ignored output trees or `results/`.

For PR #43 section 7: Q2–Q5 answer route A and createFrom/RHI item behavior;
Q6–Q8 answer route B identities and resource lifetime; Q9 answers device-only
route C; Q2/Q3/Q7 exercise where free Signal and ready Wait can be enqueued;
the deployment totals answer the preliminary package-size question. Exact sequence
and generation readbacks, resize transition counts and teardown reference counts
supply the corresponding level-1 evidence. Source facts alone answer none of
these runtime questions. The integrator reviews this candidate before merging.

## Level 2 mode (L2-Q)

`--l2-mode run` displays one of W1 to W4, produced by the ABI 2 DLL on Qt's
Direct3D 12 device. The defaults are Q5: device `qt`, route `import-direct`,
queue `same`, handover `tracked`, barriers `legacy`, render loop `threaded`.
The app requests the primary monitor, borderless full screen, topmost, a blank
cursor and swap interval 0. A matching high-performance adapter is independently
checked through DXGI after Qt has created its device. The render thread must be
per-monitor-v2 DPI aware. No UI or debug layer is drawn over the texture.

The synchronized item and the display, client and swap-chain sizes must match
before the three-slot ring is created. The ring is then fixed through teardown.
Each frame signals free, produces one scene slot and waits ready; `afterFrameEnd`
marks the slot shown after Qt submits and presents. The export route retains its
three copy-only warm-up frames before preroll and trace. During trace, each
produced frame increments `window.presents`. There are no composite readbacks,
synthetic code images or per-frame application logs in this mode.

Preroll and trace use QPC. After preroll the app allows up to 5 seconds for the
window to become foreground. During trace it samples visibility, cloaking,
five-point coverage, foreground, mains/battery, effective power mode and all five
sizes every 100 ms, as S-B does. Enforce mode stops at the first failed visibility
or foreground sample with exit 3 and no `write_run`. Record mode retains the
counts and continues. Size changes are recorded; the app keeps the ring and the
finalizer refuses the resulting evidence. Copy routes limit a copy to the source
and destination extents after a resize, so this never requires a ring rebuild.

At the first presented frame boundary at or after the requested trace length,
the app ends the trace, stops requesting frames and closes. On the render thread
it finishes Qt's submitted work, drains, writes the DLL outputs, unloads,
unregisters, destroys wrappers, releases imports and detaches. GPU-validation
counters and messages are read after the drain. The supplied QRhi/device/queue
are released only after window deletion joins the render thread. `geometry` mode
loads W1 when no scene was supplied, runs the DLL's offscreen geometry check,
drains and tears down without a trace or condition sampling.

All handled exits write `harness.json` next to the DLL outputs when `--l2-out`
is usable. It uses exclusive `harness.json.tmp` creation, flush, close and a
Windows rename with `MOVEFILE_WRITE_THROUGH` and no replacement flag. Existing
outputs are preserved. DLL-enforced `TerminateProcess` on an unconfirmed drain,
external termination and framework aborts cannot execute the final writer; the
finalizer refuses a missing harness. No successful evidence is claimed for
those exits. Exit 0 means outputs written, 1 means usage/setup/DLL/framework/I/O
failure, 2 means an output-bearing label or geometry failure, and 3 means an
enforced foreground/visibility failure. `reason` is null only on success.

The options implement HARNESS section 3. Required are `--l2-mode`, `--l2-out`
(existing directory), `--l2-run-id`, `--l2-dll` (absolute existing
`sa2_interop.dll`) and, in run mode, `--l2-scene w1|w2|w3|w4`.
Defaults/ranges: trace 192000 ms (1000 to 3600000), preroll 4000 ms (0 to 60000),
W3 turn 190 ms (finite, greater than 0, at most 10000), GPU validation 0 (0 or 1),
conditions `enforce` (`enforce` or `record`). `--l2-inject` accepts the four named
faults only for W3/W4. Repeated `--l2-declare key=value` converts `true`/`false`
to booleans; `overlays` and repeated declaration keys are refused.
`--l2-no-vram` and `--l2-debug-half-target` are flags. Half target rounds down.
Unknown, repeated, malformed and inappropriate options exit 1 before window
work. Logs and `presentmon.csv` already in the output directory are accepted;
only `harness.json`, its temporary name, `native.json`, `trace.jsonl` and
`geometry.json` block a fresh run.

Framework device/route/queue/handover/barrier options keep their `--sd-` names.
`--sd-timeout-ms` accepts only 5000, the contract's fixed wait bound. The CPU
`rhi-upload` baseline has no scene meaning and is refused in level 2, along with
frame counts, resize, verification, loss and smoke debug-layer options and an
inherited drain injection. GPU validation uses `--l2-gpu-validation 1`, enables
the D3D12 debug layer (`EnableDebugLayer` and `QQuickGraphicsConfiguration::setDebugLayer`),
and attaches with `debug_callback=1`. It does not enable GPU-based validation:
level 1 and the DLL self-test use the debug layer only, and GPU-based validation
would slow frames past the W3 and W4 turns. The render loop remains an environment
setting (`threaded` or `basic`).

Choices where the contract leaves representation open: Qt scaling uses
`device_pixel_ratio`, `item_width`, `item_height` (logical pixels) and
`texture_stretch="none"`. Configuration is flat and includes all effective
`QSG_*`/`QT_*` environment names and values except `QSG_RHI_DEBUG_LAYER`, plus
the framework options, API, loop, version and swap interval. Validation options
are excluded from configuration. The file map uses `qt:exe` and
`qt:<basename>` for each loaded module under the deployment directory; duplicate
basenames with different paths fail. Effective power mode is `unknown` if its
notification is unavailable. Incomplete AC samples give `unknown`, matching
S-B; all mains gives `mains`, all battery gives `battery`, complete mixed samples
give `changed`. Foreground is requested once, without a foreground-lock workaround.

## Level 2 owner-machine commands

Run in a short checkout on an idle machine, with no build, implementation call,
H-06 or capture active. Preparation reuses `run_smoke.py`'s Qt version check,
offline DLL/app/deployment commands, 150-character native build path bound and
artifact digest manifest. It starts no Qt app. A new build goes to
`work/sdb/<UTC stamp>/`; `--skip-build` selects only the newest build whose source,
Qt version and complete recorded artifact manifest still match. Build logs stay
in that build directory. The prepare command prints the build directory.

```powershell
python work/experiments/renderer-sd/check_project.py
python work/experiments/renderer-sd/prepare_l2.py
# Or, after an unchanged successful preparation:
python work/experiments/renderer-sd/prepare_l2.py --skip-build
$build = 'work/sdb/<printed stamp>'
```

`launch.json` starts the deployed `sd_smoke.exe` itself, with the built native
DLL's absolute path, deployment working directory and no argument separator.
The three argument lists and validation environment are empty: Q5 defaults and
the runner's `--l2-` options select the mode. Its environment prepends this
deployment to the cleaned PATH, removes other Qt installations and plugin/import
overrides, retains level 1's DPI setting and startup-only diagnostics, and sets
the threaded loop. Inherited debug-layer, drain, timing, profiling, visualization
and renderer-debug switches are removed with null overrides. The app also clears
inherited visualization/frame logging and the debug-layer environment switch.

After L2-F is integrated, follow HARNESS section 11 in this order:

```powershell
& work/experiments/renderer-l2/run_scene.ps1 -Candidate sd -Build $build -Validation
& work/experiments/renderer-l2/run_scene.ps1 -Candidate sd -Build $build -Geometry
# The commands below require administrator PowerShell, mains power and an idle machine.
& work/experiments/renderer-l2/run_scene.ps1 -Candidate sd -Build $build -Short -Scene w3
& work/experiments/renderer-l2/run_scene.ps1 -Candidate sd -Build $build -Short -Scene w3 -DebugHalfTarget
& work/experiments/renderer-l2/run_scene.ps1 -Candidate sd -Build $build -Short -Scene w3 -NoVram
& work/experiments/renderer-l2/run_scene.ps1 -Candidate sd -Build $build -Scene w1 -Overlays '<none running, or the overlays running>' -Declare @('frame_generation=false','upscaling=false')
& work/experiments/renderer-l2/run_scene.ps1 -Candidate sd -Build $build -Scene w2 -Overlays '<none running, or the overlays running>' -Declare @('frame_generation=false','upscaling=false')
& work/experiments/renderer-l2/run_scene.ps1 -Candidate sd -Build $build -Scene w4 -Overlays '<none running, or the overlays running>' -Declare @('frame_generation=false','upscaling=false')
# Three owner-attended cold W3 runs, with the runner's operator confirmation:
& work/experiments/renderer-l2/run_scene.ps1 -Candidate sd -Build $build -Scene w3 -Runs 3 -Overlays '<none running, or the overlays running>' -Declare @('frame_generation=false','upscaling=false')
```

Require four passing validation records, a passing geometry record and a passing
short check. The deliberate half target must give only `size-mismatch`; no VRAM
must give only `vram-missing`. W1/W2/W4 are preliminary without owner attendance.
The shared finalizer alone writes run/short/geometry/validation/refusal records;
the app writes only `harness.json` and the DLL writes its own outputs. Level 1
should also be exercised with the ABI 2 build by the unchanged `run_smoke.py`.

## Additional Qt 6.10.3 assumptions for level 2

The implementing sandbox had no Qt headers or source, compiler or GPU, so it
assumed every fact in this table. The integrator then checked the window,
swap-interval, swap-chain size, adapter, device-name, debug-layer and DPI facts
against tag `v6.10.3` (`work/experiments/renderer-l2-packets/FRAMEWORK-FACTS.md`,
Q1 to Q8) and found no contradiction. That is source evidence only; the other
rows stay assumed until the owner's build and runs. The DLL README says to end within the planned trace length; the
binding harness instead requires the first frame boundary at or after it, which
this app follows (the DLL reserves two additional label copies).

| Assumed fact | Check that catches an incorrect assumption |
| --- | --- |
| `showFullScreen` on the primary screen and frameless/topmost hints produce a borderless native D3D12 window; Win32 topmost request persists | Startup compares display/client/backbuffer/displayed sizes and topmost; condition samples, `size-mismatch` and `conditions` in the short capture |
| `QSurfaceFormat` swap interval 0 reaches Qt's D3D12 swap chain and uses the no-vsync present flags | Short PresentMon capture: `sync-interval`; the finalizer derives tearing from `AllowsTearing` rather than assuming present flags |
| `QQuickWindow::swapChain()->currentPixelSize()` is available and legal in render callbacks | Owner compilation, startup size comparison, trace `size-mismatch` |
| `QT_D3D_ADAPTER_INDEX` selects the `EnumAdapters1` index on Qt's D3D12 device | Owner compilation/source check; the render-thread device LUID must match DXGI's first high-performance adapter or startup fails |
| `QRhi::driverInfo().deviceName` is the adapter name | Owner compilation; recorded adapter/presenting-adapter compared with DXGI and the expected GPU in validation/short capture and gate conditions |
| `QQuickGraphicsConfiguration::setDebugLayer` works before expose and leaves graphics-device import behavior intact | Owner compilation; probe `debug_layer`, validation refusal `validation`, debug counts and imported-device identities |
| Qt defaults to per-monitor-v2 awareness even with level 1's DPI scaling disabled | Render-thread awareness check fails startup otherwise; five native sizes and scaling check |
| Sync, `beforeRendering`, RHI-item render, submit/present and `afterFrameEnd` retain level 1's order, with one render per present | Short capture `trace-steps`, native frame count versus `window.presents`; validation catches handover hazards |
| Effective DPR times item logical size equals its physical rectangle; texture node nearest filtering fills it without another scaling transform | `scaling`, `size-mismatch` and half-target refusal in short captures |
| `QQuickRhiItem::setFixedColorBufferWidth/Height` fixes its physical color-buffer size; explicit copy extents are supported | Owner compilation, copy-route startup color-buffer comparison and half-target `size-mismatch` |
| `releaseResources`, window deletion, renderer destruction and `QRhi::finish` retain level 1's render-thread teardown/join behavior | Validation/geometry/short run completion, native teardown statuses and import reference counts; runner timeout bounds Qt waits |
| Qt Core JSON, path, environment, `QFile::NewOnly`/flush and Qt screen/format/configuration APIs used here retain their established behavior | Owner compilation; usage/output-preservation checks and complete `harness.json` in every handled run; finalizer `harness` and `identity` checks |
| The retained scenegraph/RHI general logging rules log startup only | Inspect validation/short logs; application frame callbacks contain no log statements |

The acceptance check runs only Python. It retains the level 1 matrices, reference
images, privacy checks and reuse fixtures, updates the four header-derived ABI
layouts, tests the new launch writer and checks sections 2 to 6 statically with
planted defects. Pathspec exclusion scenarios now run through a Python matcher
in a plain fixture; actual Git pathspec execution is not evidence from this
sandbox. C++ parsing, Qt rendering, Windows window behavior, DLL/GPU execution,
deployment and every owner command above remain unverified, as do all performance
claims. The static pass is source/fixture evidence only.
