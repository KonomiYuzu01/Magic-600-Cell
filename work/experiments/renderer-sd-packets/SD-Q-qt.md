# Packet SD-Q: the Qt 6.10.3 Qt Quick interop smoke test (E-2.4-03 level 1)

Run from the `claude/renderer-sb` checkout, after the SA2 native DLL is committed:
`python tools/agents/codex_review.py --kind implement --model gpt-6.1-sol --effort max --timeout 7200 --packet work/experiments/renderer-sd-packets/SD-Q-qt.md`

This packet owns the S-D smoke test under `work/experiments/renderer-sd/`:
- a C++ Qt Quick application (no QML);
- its build script and run script;
- the project checks.

**Shared producer.** The test reuses the SA2 producer DLL. Its interface is the committed `work/experiments/renderer-sa2/native/include/sa2_interop.h` (ABI version 1), and its code image is `work/experiments/renderer-sa2/code_layout.json`.
- The run script builds the DLL from `work/experiments/renderer-sa2/native/` with that directory's own `build.cmd`.
- Nothing under `work/experiments/renderer-sa2/` may change.
- The header's comments name Godot as the consumer. Here Qt Quick is the consumer, so read "Godot's queue" as the queue Qt Quick submits to, and "Godot's render thread" as Qt Quick's render thread.

**Qt is not in this worktree.** Write against the Qt 6.10.3 facts in section 2 and the API list in section 3.

## 1. Goal and acceptance
- **Goal.** A Qt 6.10.3 Qt Quick application and a run script that show, on the owner's machine, whether a changing, sequence-numbered texture produced by Direct3D 12 work is displayed correctly through Qt Quick's own Direct3D 12 backend. The test also covers device ownership, synchronisation, window resize and teardown (E-2.4-03 level 1).
- **Questions to answer.** The run must answer the Qt questions of PR #43, section 7. Answers are smoke-test results, kept apart from documentation claims. The questions:
  - **Route A, Qt's own device.** Do `QRhiTexture::createFrom` and `QQuickRhiItem` behave correctly on Qt's own device and queue, through resize and teardown?
  - **Route B, `fromRhi`.** We create the QRhi with our `ID3D12Device` and `ID3D12CommandQueue` (`QRhiD3D12NativeHandles`) and hand it to Qt Quick through `QQuickGraphicsDevice::fromRhi`:
    - do the native device and queue identities hold?
    - do `QQuickRhiItem` rendering, resize and releasing resources work?
  - **Route C, device only.** Does `QQuickGraphicsDevice::fromDeviceAndContext(device, nullptr)` work as a fallback?
  - **Queue signals.** Where can our `Signal` and `Wait` go on the queue Qt exposes (section 7, question 2)?
  - **Package size.** What is the size of a minimal deployed Windows package (section 7, question 6, preliminary)?
- **Acceptance check.** `python work/experiments/renderer-sd/check_project.py` exits 0. It runs static checks and Python fixtures only: no Qt, no GPU and no C++ build.
- **Done when:**
  - the application, `build.cmd`, `run_smoke.py` with the run matrix in section 6, `smoke_summary.py`, `sd_reference.py`, `check_project.py` and `README.md` are written;
  - the application implements all three device routes, all four display routes, both queue modes, both handovers, resize, the device-loss probe, teardown and the result file;
  - the run steps for the owner's machine are written down.
- **Non-goals, and deviations from E-2.4-03:**
  - Levels 2 to 5 are out of scope: no geometry port, asset digests, reference outputs or gate runs. So this packet has its own contract instead of E-2.4-03's.
  - Also out of scope: PresentMon, timing claims, NVIDIA features, QML, custom shaders and `qsb`, PIX and RenderDoc, screen readers, and product UI.

## 2. Actual problem and reproduction

Qt 6.10.3 source facts, read from the `v6.10.3` tags of qtbase and qtdeclarative. The smoke test exists to confirm them at runtime.

- **Q1, graphics devices.** `QQuickGraphicsDevice` offers:
  - `fromAdapter(luidLow, luidHigh, featureLevel)`, `fromDeviceAndContext(void *device, void *context)`, `fromRhi(QRhi *)` (since 6.6) and `fromRhiAdapter(QRhiAdapter *)` (since 6.10);
  - no factory that takes a Direct3D 12 device together with a queue.

  For Direct3D 12, the documentation of `fromDeviceAndContext` says `context` is unused and may be null, and `device` is an `ID3D12Device*`. The caller keeps the device alive.
- **Q2, adoption** (`qsgrhisupport.cpp`, `QSGRhiSupport::createRhi`).
  - A `fromRhi` device returns the application's QRhi with `own = false`.
  - A Direct3D 12 `fromDeviceAndContext` sets only `QRhiD3D12NativeHandles::dev`, so Qt creates its own DIRECT queue at normal priority.
  - `QSGRendererInterface::getResource(window, DeviceResource)` and `getResource(window, CommandQueueResource)` return the `dev` and `commandQueue` of `QRhi::nativeHandles()`.
- **Q3, device import** (`qrhid3d12.cpp`, the `QRhiD3D12` constructor and `destroy()`).
  - An imported device is queried for `ID3D12Device2`, and the extra reference is released, so Qt keeps no net reference.
  - If `ID3D12Device2` is missing, Qt prints `ID3D12Device2 not supported, cannot import device` and silently creates its own device.
  - An imported command queue is stored without AddRef.
  - `destroy()` releases neither an imported device nor an imported queue.
  - `QRhiD3D12InitParams::enableDebugLayer` acts only when Qt creates the device. With an imported device, the application enables the debug layer before `D3D12CreateDevice`.
- **Q4, wrapping a native texture.**
  - `QD3D12Texture::createFrom({object, layout})` adds the resource without ownership, so Qt never releases it. Qt tracks the legacy state from `layout` (a `D3D12_RESOURCE_STATES` value).
  - `nativeTexture()` returns the resource and the state Qt currently tracks. `setNativeLayout(state)` overwrites the tracked state.
  - `destroy()` defers the release of Qt's bookkeeping and the SRV by the frames in flight; the resource itself is never released.
- **Q5, barriers.**
  - Qt's Direct3D 12 backend issues only legacy `ResourceBarrier` transitions, never enhanced barriers.
  - A texture sampled in the fragment stage is moved to `PIXEL_SHADER_RESOURCE`.
  - A texture Qt creates with `QRhiTexture::RenderTarget` gets `ALLOW_RENDER_TARGET` and an optimised clear value.
- **Q6, native commands.**
  - `QRhiCommandBuffer::nativeHandles()` gives `QRhiD3D12CommandBufferNativeHandles::commandList`, an `ID3D12GraphicsCommandList1*`.
  - `beginExternal()` does nothing. `endExternal()` resets the per-pass state and rebinds Qt's descriptor heaps and render target.
  - A transition recorded outside Qt must be reported with `setNativeLayout`.
- **Q7, frames.**
  - Qt keeps 2 frames in flight.
  - `beginFrame` waits, without a time limit, for the frame-slot fence of every swap chain, then waits up to 1000 ms on the frame-latency object.
  - `endFrame`, in order: `Close`; `ExecuteCommandLists` with Qt's single command list for the frame; `Present`; then `Signal` of the frame fence on the queue.
  - A `Present` that returns `DXGI_ERROR_DEVICE_REMOVED` or `_RESET` sets `deviceLost`, which `isDeviceLost()` returns.
  - `QRhi::finish()` may be called inside a frame outside a render pass. It submits the partial command list, waits for the GPU without a time limit, restarts the list, runs deferred releases and completes pending readbacks.
- **Q8, device loss in the threaded render loop.**
  - On loss, the render thread prints `Graphics device lost, cleaning up scenegraph and releasing RHI` and tears down the scene graph and swap chain. It destroys the QRhi only if it owns it.
  - The next frame calls `createRhi` again:
    - with `fromRhi`, the same lost QRhi comes back;
    - with `fromDeviceAndContext`, the same removed device is imported again;
    - with Qt's own device, Qt creates a new one.
  - So under routes B and C, recovery is the application's job.
- **Q9, `QQuickRhiItem` colour buffer** (`QQuickRhiItemNode::sync`).
  - The colour texture is `RGBA8` with `RenderTarget | UsedAsTransferSource`.
  - On resize, the same `QRhiTexture` object is resized and created again, which gives a new native resource, and the renderer's `initialize()` runs again.
- **Q10, teardown.**
  - `QQuickRhiItem::releaseResources()` (GUI thread) and `invalidateSceneGraph()` (render thread) only drop the node pointer.
  - The scene graph deletes the node and its renderer on the render thread while the QRhi is still alive. The renderer's destructor is therefore the place to release native objects.
- **Q11, setting the device.** `QQuickWindow::setGraphicsDevice` has an effect only before the scene graph is initialised.
- **Q12, zero-copy display.** `QNativeInterface::QSGD3D12Texture::fromNative(void *texture, int resourceState, QQuickWindow *, QSize, options)` wraps an `ID3D12Resource` as a `QSGTexture`, which a `QSGSimpleTextureNode` can show without a copy.
- **Q13, when `QQuickRhiItem` renders.** `QQuickRhiItemNode::render` is connected to `QQuickWindow::beforeRendering` with a direct connection. It runs on the render thread before Qt Quick records its main pass, and only after the item called `update()`.
- **Q14, threaded-loop order per frame.**
  1. swap-chain resize;
  2. `beforeFrameBegin`;
  3. `beginFrame`;
  4. sync, if requested, with the GUI thread blocked;
  5. `renderSceneGraph`, which emits `beforeRendering`, records the main pass and emits `afterRendering`;
  6. `endFrame`;
  7. `afterFrameEnd`.

Therefore:
- **Queue order.** Work our producer submits on Qt's queue during `beforeRendering` of frame f runs before Qt's command list for frame f, which is submitted in `endFrame`.
- **Free signal.** A `Signal` enqueued at `beforeRendering` of frame f comes after Qt's frame f − 1 submission, so it marks frame f − 1 as finished.
- **Wait.** A `Wait` enqueued before `endFrame` holds back Qt's frame f.
- **Handover state.** The state the producer leaves a texture in must be the state Qt tracks:
  - `COPY_SOURCE` for a texture Qt copies from;
  - `PIXEL_SHADER_RESOURCE` for a texture the scene graph samples;
  - or a state the harness declares with `setNativeLayout` before Qt's next use.

## 3. Environment and versions
- Base: branch `claude/renderer-sb` (this worktree's HEAD), with the SA2 native DLL sources committed under `work/experiments/renderer-sa2/native/`.
- Owner's machine:
  - Windows 11 (build 26200), NVIDIA GeForce RTX 4070 Laptop GPU plus an integrated AMD Radeon 610M.
  - Visual Studio Community 2026 (MSVC 14.51) and Windows SDK 10.0.26100.0.
  - CMake 4.4.3 and Ninja 1.13.2 in `tools/.venv/renderer-spike/Scripts/`.
  - Qt 6.10.3 `msvc2022_64` at `tools/qt/6.10.3/msvc2022_64/` (modules qtbase, qtdeclarative, qtsvg and qtshadertools), installed by `bootstrap.py install qt-6-10-3` once the owner approves the entry. None of this is in the sandbox.
  - The Qt binaries were built with MSVC 2022. Microsoft keeps the v14x toolsets binary compatible; the build on the owner's machine confirms it for MSVC 14.51.
- Network rule: build and run offline; nothing is downloaded.
- Qt API (6.10.3) the application may use. Mark any other Qt call in the final message with the reason it is believed to exist in 6.10.3.
  - **Application and window.**
    - `QGuiApplication`, `QQuickWindow` with items created in C++ (`contentItem()`), `QQuickWindow::setGraphicsApi(QSGRendererInterface::Direct3D12)`;
    - `QQuickWindow::setGraphicsDevice`, `QQuickGraphicsDevice::fromRhi`, `QQuickGraphicsDevice::fromDeviceAndContext`;
    - `QQuickWindow::rhi()`, `QQuickWindow::swapChain()`, `QQuickWindow::rendererInterface()`, `QSGRendererInterface::getResource(QQuickWindow *, Resource)`, `QSGRendererInterface::graphicsApi()`;
    - `QQuickWindow` signals `sceneGraphInitialized`, `sceneGraphInvalidated`, `sceneGraphError`, `beforeFrameBegin`, `beforeRendering`, `afterRendering`, `afterFrameEnd` and `frameSwapped`;
    - `QQuickWindow::effectiveDevicePixelRatio()` and `qVersion()`.
  - **Items.**
    - `QQuickRhiItem` with `createRenderer()`, and `QQuickRhiItemRenderer` with `initialize(QRhiCommandBuffer *)`, `synchronize(QQuickRhiItem *)`, `render(QRhiCommandBuffer *)`, `colorTexture()`, `rhi()` and `update()`;
    - `QQuickItem::updatePaintNode`, `QSGSimpleTextureNode` (with `setFiltering(QSGTexture::Nearest)`), `QNativeInterface::QSGD3D12Texture::fromNative` (`<QtQuick/qsgtexture_platform.h>`).
  - **QRhi** (`<rhi/qrhi.h>`, linked through `Qt6::Gui`).
    - `QRhi::create(QRhi::D3D12, &QRhiD3D12InitParams, QRhi::Flags, &QRhiD3D12NativeHandles)`, `QRhi::nativeHandles()`, `QRhi::finish()`, `QRhi::nextResourceUpdateBatch()`;
    - `QRhi::newTexture(QRhiTexture::RGBA8, QSize, 1, flags)` with `create()`, `createFrom(QRhiTexture::NativeTexture{object, layout})`, `nativeTexture()` and `setNativeLayout(int)`;
    - `QRhiResourceUpdateBatch::copyTexture`, `uploadTexture` and `readBackTexture`, `QRhiReadbackDescription` (empty for the current swap-chain back buffer), `QRhiReadbackResult` with its `completed` callback, `format` and `pixelSize`;
    - `QRhiCommandBuffer::resourceUpdate`.
  - **Environment variables.**
    - `QSG_RHI_DEBUG_LAYER=1` enables the debug layer when Qt creates the device.
    - `QT_LOGGING_RULES=qt.rhi.general=true;qt.scenegraph.general=true` prints `Using imported device` and `Using existing native D3D12 device`.
    - `QSG_RENDER_LOOP=threaded|basic` selects the render loop.
    - `QT_ENABLE_HIGHDPI_SCALING=0` makes logical pixels equal device pixels.
- Evidence kind in the sandbox: source/fixture only.

## 4. Necessary source and evidence
- `work/experiments/renderer-sa2/native/include/sa2_interop.h`: the producer API, its call order, fences, states and teardown rules. Read all of it.
- `work/experiments/renderer-sa2/native/README.md` and `src/`: how the DLL behaves after a device removal.
- `work/experiments/renderer-sa2/code_layout.json`: the code image, the decode rule and the test vectors.
- `work/experiments/renderer-sa2-packets/SA2-G-godot.md`: the parallel Godot packet. Mirror its result-file rules, pass rule, summary allowlist, privacy scan, timeouts and temp-directory rules where this packet does not say otherwise.
- `docs/progress/1.0/packets/renderer/E-2.4-03-sd-qt.md` (level 1) and `docs/progress/1.0/renderer-experiment-plan.md`, section 3.
- `docs/progress/1.0/requirements-from-screening.md`: R-04 (bounded, ordered shutdown), R-14 (no unbounded waits on the UI thread) and R-17 (device loss: recover or report).
- `work/experiments/renderer-sb/handoff/build.cmd` and `README.md`: the build-script pattern and the S-B fence and drain protocol.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| — | none for Qt; the Godot smoke test (SA2-G) is the parallel design | — | — | — |
| 1 | the SA2-G runner can build the DLL under its private output root | SA2-G run on the owner's machine, 2026-10-03 | native build log | failed: cl.exe fatal error C1083 on a try-compile object path over 260 characters; builds moved to a short folder under `work/`, as section 6 now requires here |

## 6. Constraints and owned files

Owned files (all under `work/experiments/renderer-sd/`):
- `app/**`;
- `build.cmd`;
- `run_smoke.py`, `smoke_summary.py`, `sd_reference.py`, `check_project.py`;
- `README.md`.

Change nothing else, and leave no build output in the worktree.

Synthetic content only: the test writes code images. It never uses labels from a personal session and never reads a user database.

### Application (`app/`)
- **`CMakeLists.txt`.**
  - CMake 3.30 or later, C++20.
  - `find_package(Qt6 REQUIRED COMPONENTS Core Gui Quick)`, then configuration fails unless `Qt6_VERSION` is exactly 6.10.3.
  - `qt_standard_project_setup()`.
  - Executable `sd_smoke`, console subsystem so stdout and stderr can be captured, linking `Qt6::Core`, `Qt6::Gui`, `Qt6::Quick`, `d3d12`, `dxgi` and `dxguid`.
  - A second executable, `sd_code_layout_test`, without Qt: it checks the C++ code-layout functions against the test vectors.
  - The SA2 header comes from `../../renderer-sa2/native/include`, and the code uses only its types and constants. The DLL is loaded at run time with `LoadLibraryW` from the absolute path in `--sd-dll`, and every function is bound with `GetProcAddress`; `decltype(&sa2_x)` gives the pointer types.
  - Before any other call, the application checks `sa2_abi_version() == 1` and the struct sizes 48, 28 and 128.
- **Code layout** (`app/src/code_layout.{h,cpp}`): CRC, code value, expected image and decode, exactly as in `code_layout.json`.
- **Window.**
  - A `QQuickWindow` of 1280 × 720 at position (40, 40), with `setGraphicsApi(Direct3D12)` before the window is created. Its single content item is at (0, 0) and its size follows the window.
  - No QML engine.
  - The effective device pixel ratio must be 1, which the run script ensures with `QT_ENABLE_HIGHDPI_SCALING=0`. Any other value fails the run with reason `dpr-not-1`.
  - The backend must be Direct3D 12 (`rendererInterface()->graphicsApi()`); otherwise the run fails with `backend-not-d3d12`.

### Harness arguments (any unknown or malformed argument: one stderr line and exit 2)
- `--sd-out <abs json>` (required).
- `--sd-dll <abs path>` (required).
- `--sd-device qt|from-rhi|from-device`
- `--sd-route rhi-upload|import-copy|export-copy|import-direct`
- `--sd-queue same|own`
- `--sd-handover tracked|declared`
- `--sd-barriers legacy|match` (the DLL's barrier API; Qt's own barriers are always legacy)
- `--sd-debug-layer`
- `--sd-frames N` (default 1200)
- `--sd-resize-every N` (0 = off)
- `--sd-verify-every N` (default 50)
- `--sd-device-loss-at N` (0 = off)
- `--sd-timeout-ms N` (default 5000; used for the DLL's waits and its drain)

### Device routes
- **`qt` (route A).** Qt creates the device and queue.
  - On the render thread after `sceneGraphInitialized`, read `DeviceResource` and `CommandQueueResource` through `QSGRendererInterface`, and compare them with `rhi()->nativeHandles()`.
  - With `--sd-debug-layer`, the run script sets `QSG_RHI_DEBUG_LAYER=1`.
- **`from-rhi` (route B).** On the GUI thread, before the window is shown:
  1. Pick the adapter with `IDXGIFactory6::EnumAdapterByGpuPreference(0, HIGH_PERFORMANCE)`.
  2. With `--sd-debug-layer`, call `D3D12GetDebugInterface` and `EnableDebugLayer`.
  3. `D3D12CreateDevice` at feature level 11_0. Create a DIRECT queue named "SD harness queue".
  4. `QRhi::create(QRhi::D3D12, {enableDebugLayer}, {}, {dev, commandQueue})`.
  5. Check `rhi->nativeHandles()`:
     - the `dev` has the same COM identity as our device (`QueryInterface(IID_IUnknown)` on both);
     - the `commandQueue` is our queue.

     A failed check, which includes Qt's silent fallback (Q3), fails the run with `identity-mismatch`.
  6. `setGraphicsDevice(QQuickGraphicsDevice::fromRhi(rhi))`, then show the window.

  On the render thread after `sceneGraphInitialized`, check that `window->rhi() == rhi` and that both `QSGRendererInterface` resources equal ours.
- **`from-device` (route C).**
  - Create our device as in route B, without a queue, then `setGraphicsDevice(QQuickGraphicsDevice::fromDeviceAndContext(dev, nullptr))`.
  - After initialisation, check the device's COM identity. The queue is Qt's: read it through `QSGRendererInterface`, and record `queue_device_matches` from `sa2_probe`.

In every route, the DLL is attached on the render thread to the device and the queue Qt Quick submits to. `sa2_probe` uses the adapter that matches the device's LUID; the LUID is never written out.

### Display routes
- **`rhi-upload`** (pure Qt baseline).
  - The `QQuickRhiItem` renderer uploads the expected code image of frame f into `colorTexture()` with `uploadTexture`.
  - The DLL is attached only for its debug counters: `SA2_QUEUE_SAME`, no slots.
- **`import-copy`.**
  - A ring of 3 textures from `sa2_create_texture`, with the handover state as initial state. Each is wrapped in a `QRhiTexture` (`RGBA8`, `UsedAsTransferSource`) with `createFrom({ptr, state})` and registered with `godot_owned 0`.
  - Each frame the renderer copies the produced slot into `colorTexture()` (`copyTexture`).
- **`export-copy`.**
  - A ring of 3 textures that Qt creates (`RGBA8`, `RenderTarget | UsedAsTransferSource`). Each is registered with `godot_owned 1`, using `nativeTexture().object`.
  - Each frame the renderer copies the produced slot into `colorTexture()`.
- **`import-direct`** (zero copy, no `QQuickRhiItem`).
  - The ring textures from `sa2_create_texture` are wrapped with `QSGD3D12Texture::fromNative(ptr, D3D12_RESOURCE_STATE_PIXEL_SHADER_RESOURCE, window, size)`.
  - A custom `QQuickItem` shows them through a `QSGSimpleTextureNode` with nearest filtering. Its `updatePaintNode` selects slot f % 3.

Handover states (the `sa2_config` fields):
- **`tracked`.**
  - `import-copy` and `export-copy`: `before = after = SA2_STATE_COPY_SOURCE`.
  - `import-direct`: `before = after = SA2_STATE_PIXEL_SHADER_RESOURCE`.
  - `export-copy` only: after each (re)build of the ring, a warm-up of one frame per slot, in which Qt copies the slot without a write. Qt's tracker then holds `COPY_SOURCE`. The imported routes need no warm-up, because `createFrom` starts Qt's tracker at the handover state.
- **`declared`** (probe for Q4, `import-copy` only).
  - `before = SA2_STATE_COPY_SOURCE`, `after = SA2_STATE_RENDER_TARGET`, and the initial state is `RENDER_TARGET`.
  - After each produce, the renderer calls `setNativeLayout(D3D12_RESOURCE_STATE_RENDER_TARGET)` on the slot's `QRhiTexture` before recording the copy.
- `queue_mode` comes from `--sd-queue`, `barrier_api` from `--sd-barriers` (`legacy` is `SA2_BARRIERS_LEGACY`, `match` is `SA2_BARRIERS_MATCH_GODOT`) and `wait_timeout_ms` from `--sd-timeout-ms`; `debug_callback` is 1.

### Frame protocol
- **Threads and counting.**
  - All `sa2_*` calls and all native Direct3D 12 calls after window creation run on Qt Quick's render thread. With `QSG_RENDER_LOOP=basic`, that is the GUI thread.
  - Run frame f counts render-thread frames:
    - `QQuickRhiItemRenderer::render` calls for the three `QQuickRhiItem` routes;
    - `beforeRendering` emissions, through a direct connection, for `import-direct`.
  - Slot k is used in the frames with f % 3 = k.
  - The GUI thread requests every frame: it calls `update()` on the item from a queued handler of `frameSwapped`.
- **The render step of a run frame f, in this order, inside `render()` or the `beforeRendering` handler:**
  1. `sa2_signal_godot_free(f - 1)`. Every step after `sa2_attach` calls it, warm-up and rebuild steps included.
  2. `sa2_produce(slot f % 3, f, sequence f, generation)`.
  3. `sa2_godot_wait_ready(f)`.
  4. `sa2_mark_shown(slot, f)`.
  5. Copy routes: record `copyTexture` into `colorTexture()`. `rhi-upload`: record the upload instead of steps 1 to 4.
  6. Every `verify_every` run frames:
     - `sa2_verify_slot` (native routes);
     - in the copy routes and `rhi-upload`, a `readBackTexture` of `colorTexture()` after the copy, compared texel by texel with the expected image when its `completed` callback runs.

     Never read back a ring slot through Qt: Qt would then track it as `COPY_SOURCE` while the producer assumes another state.
  7. If frame f is eligible: in `afterRendering` of frame f, a `readBackTexture` of the current swap-chain back buffer (empty description). The callback decodes both corners at (0, 0) and the ring size, using the reported format (RGBA8 or BGRA8), and compares them with frame f's code.
- **Locks.** Readback callbacks run on the render thread. State shared with the GUI thread is guarded by a lock, which is never held while calling Qt or the DLL.
- **Eligible frames.** Frame f's composite readback is requested only when all of these hold:
  - f is a run frame of the current ring generation, outside warm-up;
  - the back buffer is at least as large as the ring;
  - no resize is pending.

  Every other frame is counted as skipped, with its reason (`warmup` or `transition`).
- **Export pointer check.** At every frame, `nativeTexture().object` of each export slot must equal the registered pointer; otherwise the run fails with `resource-changed`.
- **Resize.**
  - Every `resize_every` run frames the GUI thread resizes the window to the next size in 1280×720, 1024×640, 1440×810, 800×600 (cycling).
  - When the render step finds the colour buffer (or, for `import-direct`, the item) at a size other than the ring's, it rebuilds inside the frame:
    1. `rhi()->finish()` (Q7);
    2. destroy the ring's `QRhiTexture` or `QSGTexture` wrappers;
    3. `sa2_drain`;
    4. `sa2_unregister_slot` for each slot;
    5. `sa2_release_texture` for each imported texture (record `refcount_after`);
    6. increase the generation (mod 4096), then create, wrap and register the new ring at the new size, and warm up where the route needs it.
  - Record the frames each transition took.
- **Device loss** (`--sd-device-loss-at N`, a probe for R-17).
  - At run frame N, the render step calls `sa2_remove_device`.
  - Afterwards, for up to 10 s, record:
    - what the DLL reports (statuses, `sa2_device_removed_reason`);
    - what Qt reports: `sceneGraphInvalidated`, `sceneGraphError`, a second `sceneGraphInitialized`, and whether render steps resume on a new QRhi;
    - Qt's stderr lines (counted by the run script).
  - Then tear down. The status is `recorded` whatever happens, and the reasons record what was seen.
  - The DLL is not attached again after a loss.
- **Teardown** (after the run frames, or early on a failed run):
  1. The GUI thread closes the window.
  2. On the render thread, in the renderer's destructor or on `sceneGraphInvalidated` (Q10):
     - `rhi()->finish()`;
     - write the result file with `teardown.phase = "pending"`;
     - `sa2_drain`; `sa2_unregister_slot` for each slot; `sa2_release_texture` (record `refcount_after`);
     - `sa2_debug_counts` and `sa2_debug_messages`; `sa2_detach`.
  3. The GUI thread deletes the window.
  4. Route B: `delete` the QRhi after the window and its render thread are gone, then release our queue and our device and record the counts `Release` returns. Route C: release our device and record the count.
  5. Write the final result, then `QGuiApplication::exit(code)`.

  An unconfirmed drain ends the process with exit 3 inside the DLL, so the pending result is all that remains.
- **Exit codes.** 0 for status `pass` or `recorded`; 1 for `fail`; 2 for argument errors. Exit 3 comes only from the DLL.

### Result file (`--sd-out`, format `magic600-sd-smoke-run-v1`, written atomically: a temp file, then a rename)
- `status`, `reasons` and `sa2_failures`: as in SA2-G, plus the reasons `identity-mismatch`, `backend-not-d3d12`, `dpr-not-1`, `qt-version` and `rhi-create-failed`.
- `config`: the arguments, plus the render loop and the debug-layer setting.
- `qt`: `qVersion()`, the graphics API, the render loop, the effective device pixel ratio, and the back-buffer format seen by the readback.
- `device`:
  - the route and `sa2_probe`'s fields;
  - `identity`: `device_matches`, `queue_matches` (route B), `rhi_matches` (route B), `qt_queue_device_matches` (route C), and `fallback_detected`.
- `dll`: ABI version, resolved barrier API, handover and initial states.
- `frames`, `verify`, `resize`, `device_loss` and `debug`: as in SA2-G, with composite readbacks in place of Godot's viewport readbacks.
- `teardown`:
  - phase, drain result, `refcount_after` per imported texture, slots unregistered, detached;
  - routes B and C: `device_refcount_after`, and `queue_refcount_after` for route B.

A run passes when:
- it was not a probe;
- every eligible frame verified, and eligible frames make up at least 90% of the run frames outside warm-up and transitions;
- all readbacks completed, and no verify reported a mismatch;
- no `sa2_*` call failed;
- with the debug layer on, `error` and `corruption` are 0;
- the identity checks held;
- teardown completed with the drain confirmed, every `refcount_after` is 0, and the context detached;
- route B ends with `queue_refcount_after` and `device_refcount_after` at 0, and route C with `device_refcount_after` at 0.

### `build.cmd [build-dir]`
- Same pattern as `work/experiments/renderer-sb/handoff/build.cmd`: `vswhere`, `vcvars64.bat`, and the pinned CMake and Ninja from `tools\.venv\renderer-spike\Scripts\` when present.
- It configures `app/` with Ninja, `-DCMAKE_BUILD_TYPE=Release` and `-DCMAKE_PREFIX_PATH=<repository>\tools\qt\6.10.3\msvc2022_64`. It fails with one line when that directory is missing.
- The file is CRLF only.

### `run_smoke.py`
Python 3 standard library only.

Steps:
1. Require `tools/qt/6.10.3/msvc2022_64/bin/qmake.exe -query QT_VERSION` to print exactly `6.10.3`.
2. Record the source identity:
   - `git rev-parse HEAD`;
   - a sha256 source digest over sorted relative paths and file digests under `work/experiments/renderer-sd/` and `work/experiments/renderer-sa2/native/`, excluding `results/`, build output and `__pycache__/`;
   - whether those files match HEAD, compared as in SA2-G.
3. Build the SA2 DLL with `cmd.exe /d /c call work\experiments\renderer-sa2\native\build.cmd <build>\native`.
   - `<build>` is `work/sdb/<UTC stamp>/` of the repository root, in the ignored work tree, with the same stamp as the private output folder. cl.exe is not long-path aware: CMake's try-compile objects under the private output root pass 260 characters in this checkout's path (observed in the SA2 runner on 2026-10-03, fatal error C1083).
   - Refuse to build, with one line, when `<build>\native` is longer than 150 characters.
   - Builds and the deployment go under `<build>`; logs, per-run results and the private summary stay under the private output root.
4. Build the application with `cmd.exe /d /c call work\experiments\renderer-sd\build.cmd <build>\app`.
5. Deploy:
   - copy `sd_smoke.exe` into `<build>\deploy\` and run Qt's `windeployqt --release --no-translations --no-system-d3d-compiler --no-opengl-sw --no-quick-import --dir <build>\deploy <build>\deploy\sd_smoke.exe`;
   - record the deployed size as total bytes and file count, without the SA2 DLL;
   - all runs start from the deployed copy.
6. Record the sha256 of the DLL and of `sd_smoke.exe`.

Run matrix: one process per run, run in order. Common environment:
- `QT_ENABLE_HIGHDPI_SCALING=0`;
- `QT_LOGGING_RULES=qt.rhi.general=true;qt.scenegraph.general=true`;
- `QSG_RENDER_LOOP` from the row;
- `QSG_RHI_DEBUG_LAYER=1` only for route A rows with the debug layer on;
- `PATH` without any other Qt installation.

stdout and stderr are captured.

| Run | Device | Route | Queue | Handover | DLL barriers | Debug layer | Render loop | Frames | Resize every | Expected |
|---|---|---|---|---|---|---|---|---|---|---|
| Q0 | `sa2_selftest.exe --hardware --debug` | | | | | | | | | pass; on failure, stop the matrix |
| Q0b | `sd_code_layout_test.exe` | | | | | | | | | pass; on failure, stop the matrix |
| Q1 | qt | rhi-upload | same | – | legacy | on | threaded | 1200 | 200 | pass |
| Q2 | qt | import-copy | same | tracked | legacy | on | threaded | 1200 | 200 | pass |
| Q3 | qt | import-copy | own | tracked | legacy | on | threaded | 1200 | 200 | pass |
| Q4 | qt | export-copy | same | tracked | legacy | on | threaded | 1200 | 200 | pass |
| Q5 | qt | import-direct | same | tracked | legacy | on | threaded | 1200 | 200 | pass |
| Q6 | from-rhi | import-copy | same | tracked | legacy | on | threaded | 1200 | 200 | pass |
| Q7 | from-rhi | import-copy | own | tracked | legacy | on | threaded | 1200 | 200 | pass |
| Q8 | from-rhi | import-direct | same | tracked | legacy | on | basic | 1200 | 200 | pass |
| Q9 | from-device | import-copy | same | tracked | legacy | on | threaded | 1200 | 200 | pass |
| Q10 | qt | import-copy | same | declared | legacy | on | threaded | 300 | 0 | recorded |
| Q11 | from-rhi | import-copy | own | tracked | legacy | off | threaded | 3000 | 500 | pass |
| Q12 | qt | import-copy | own | tracked | match | on | threaded | 1200 | 0 | recorded |
| Q13 | from-rhi | import-copy | own | tracked | legacy | on | threaded | 120 | 0 | exit 3, the DLL's stderr line, and a pending result (`M600_SA2_INJECT_UNCONFIRMED_DRAIN=1` for this run only) |
| Q14 | qt | import-copy | own | tracked | legacy | on | threaded | 400 | 0 | recorded; only with `--device-loss`, removal at run frame 200 |
| Q15 | from-rhi | import-copy | own | tracked | legacy | on | threaded | 400 | 0 | recorded; only with `--device-loss`, removal at run frame 200 |

- **Judging.** Each run gets `as-expected` or `unexpected`, against its row and against `--expect-adapter` (default "NVIDIA GeForce RTX 4070 Laptop GPU"). For Q14 and Q15, the run is as expected whenever the process ends within its timeout, whatever the exit code; record the exit code, whether a result file exists and what it says.
- **Timeouts.** 300 s per run, 120 s for Q14 and Q15. On a timeout, kill only that run's process tree (`taskkill /PID <pid> /T /F`) and record `timeout`.
- **Qt output.** Count stderr lines in total, and the lines containing each of these phrases:
  - `Failed to`;
  - `Device loss detected`;
  - `Graphics device lost`;
  - `cannot import device`;
  - `Using imported device`;
  - `Using existing native D3D12 device`.
- **Options.**
  - `--only Q2,Q6`;
  - `--device-loss`;
  - `--skip-build`: reuse the newest build and deployment whose source identity and Qt version match, only from under `work/sdb/`, after checking the recorded digests;
  - `--dry-run`: no build and no processes; prints the matrix and the command lines with placeholders;
  - `--write-summary`.

Private outputs go under `work/loop-memory/perf/renderer/sd-smoke/<UTC stamp>/` of the repository root: logs, per-run results, the build identity and a private summary. Builds and the deployment go under `<build>` (step 3).

With `--write-summary`, `smoke_summary.py` writes `work/experiments/renderer-sd/results/sd-smoke-summary.json` (format `magic600-sd-smoke-summary-v1`):
- It contains:
  - the source identity, the DLL and executable digests, and the Qt version;
  - the adapter name, and the UMD version formatted as a driver version;
  - `enhanced_barriers`;
  - the deployed size;
  - per run: the row, `status`, judgement, reasons, identity fields, frame counts, verify counts, debug counts and distinct IDs, the teardown fields, the stderr phrase counts and the exit code.
- It is built from an allowlist of fields. Free text never enters the summary: no messages, last errors, paths, user names, LUIDs, pointer values, vendor or device IDs, or command lines.
- A final scan refuses to write if any string matches a drive path, `\Users\`, `/Users/`, `AppData`, a `%VAR%` pattern, a `0x` pointer-like value of 8 or more hex digits, or the current user name.

### `check_project.py` (acceptance; runs in the sandbox)
- **Files and settings.**
  - Required files exist.
  - `CMakeLists.txt` requires exactly Qt 6.10.3, links the listed libraries and builds both executables.
  - `build.cmd` is CRLF only and names no `http` source.
- **C++ against the header.**
  - Every `SA2_API` function in the header is bound by name in the application's loader.
  - The status, queue, barrier and state constants used in the C++ files come from the header, not from copies.
  - The code-layout constants in `code_layout.{h,cpp}` equal `code_layout.json`.
- **Python reference** (`sd_reference.py`):
  - the CRC and code test vectors from `code_layout.json`;
  - expected-image round trips at 128 × 128 and 160 × 144, where both corners decode and a flipped block is rejected;
  - the decode of a composite larger than the ring, in RGBA8 and BGRA8.
- **Run script.** `run_smoke.py --dry-run` exits 0 and lists Q0 to Q13, and Q14 and Q15 only with `--device-loss`.
- **Summary writer.**
  - A synthetic run result with a planted drive path, a planted user name and a planted pointer value is refused or stripped.
  - A clean synthetic result is written to a temp directory, never to `results/`, and passes the scan.
- **Owned files.** No drive path with a `Users` component, no `/Users/` and no `AppData\` literal. Temp directories are made with plain `mkdir` under `tempfile.gettempdir()` and removed afterwards (never `mkdtemp` or `TemporaryDirectory`).
- **No side effects.** The acceptance check writes nothing in the worktree outside its temp directories: every script sets `sys.dont_write_bytecode = True` before it imports a local module, and nothing writes `results/`, `work/loop-memory/` or `work/sdb/`. The wrapper rejects a run in which any file outside the owned files changed, ignored files included.

### `README.md`
- Prerequisites and the owner's commands: the full matrix, `--only`, `--device-loss` and `--write-summary`.
- Run only on an idle machine: never during PresentMon captures, builds or Codex implementation calls.
- What each run proves.
- Where private outputs go, and what may be committed.
- The Qt API notes:
  - Q1 to Q14, each marked "source-verified against 6.10.3" or "runtime-confirmed by run Qn". Runtime confirmation is pending until the owner's runs.
  - What is not verified: Qt 6.11 and later; the `basic` render loop beyond Q8; recovery after a device loss under routes B and C.
- How the results answer the PR #43 questions and the level-1 items: device ownership, synchronisation, resize, teardown, sequence numbers, package size.

```implement-contract
{"allowed_files": ["work/experiments/renderer-sd/*"], "acceptance_check": ["python", "work/experiments/renderer-sd/check_project.py"], "stop_condition": "the Qt Quick application with the three device routes, four display routes, both queue modes, both handovers, resize, device-loss probe and teardown, build.cmd, run_smoke.py with the Q0-Q15 matrix and the sanitised summary writer, sd_reference.py, check_project.py and README.md are written, and check_project.py passes"}
```

## 7. Required return format
- Implementation: changes only in the assigned worktree. The final message lists:
  - the changed files;
  - the acceptance result;
  - what was not verified in the sandbox: every Qt, GPU and C++ build step;
  - any Qt API use not listed in section 3, with the reason it is believed to exist in 6.10.3;
  - open points.
