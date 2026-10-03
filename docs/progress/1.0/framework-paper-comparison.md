# Framework paper comparison: Godot 4.7 .NET (S-A2) and Qt Quick 6.12 (S-D)

Status: **input, not a selection.** This page collects documented facts for the stage 2.4 day-7 go/no-go and the framework selection ([renderer-candidates](../../wiki/decisions/renderer-candidates.md), [renderer-experiment-plan](renderer-experiment-plan.md)). It replaces no measurement. It makes no performance claim: whether either stack meets the gate is answered only by the S-A2 and S-D runs.

How to read it:
- Sources were accessed on 2 October 2026.
- **D** (documented) means an official source (documentation, release page, official blog or engine source at the named tag) states the fact directly.
- **U** (unverified) means the fact is inferred, community-reported or not found. Every U item is a question for the experiments, not a finding.
- Version facts:
  - Godot `4.7-stable` (`version.py` at that tag, D). Its release date is only reported second-hand (U).
  - Qt 6.12.0, which doc.qt.io now serves (D). The Qt wiki schedule lists the final release for 30 September 2026 (D, schedule; the actual date is not confirmed). LTS status for open-source users is stated inconsistently between the beta blog and the releases page (U).

## 1. Direct3D 12 interop

| Question | Godot 4.7, RenderingDevice | Qt Quick 6.12, QQuickRhiItem on QRhi D3D12 |
|---|---|---|
| Backend status | D3D12 is "pretty much on par with Vulkan"; new Windows projects default to D3D12 since 4.6 (D, [4.6 release page](https://godotengine.org/releases/4.6/)) | The D3D12 backend has existed since 6.6. D3D11 stays the Qt Quick default on Windows "for the foreseeable future"; D3D12 is selected with `QSG_RHI_BACKEND=d3d12` or `QQuickWindow::setGraphicsApi` (D, [Qt 6.6 D3D12 blog](https://www.qt.io/blog/direct3d-12-support-in-qt-6.6); the 6.12 scenegraph docs still name D3D11 as the Windows default) |
| Native device and queue | `RenderingDevice.get_driver_resource()` returns `ID3D12Device`, `IDXGIAdapter` and `ID3D12CommandQueue` (D, [class docs](https://docs.godotengine.org/en/stable/classes/class_renderingdevice.html)) | `QRhiD3D12NativeHandles` exposes `dev` and `commandQueue` (D, [qrhid3d12nativehandles](https://doc.qt.io/qt-6/qrhid3d12nativehandles.html)). Qt Quick can be pointed at our adapter by LUID with `QQuickGraphicsDevice::fromAdapter` (D, [qquickgraphicsdevice](https://doc.qt.io/qt-6/qquickgraphicsdevice.html)). With D3D12 selected, `QQuickGraphicsDevice::fromDeviceAndContext` adopts an existing `ID3D12Device` (the context argument is unused) (D). At the QRhi level, the `commandQueue` member of `QRhiD3D12NativeHandles` documents importing an existing `ID3D12CommandQueue` when the QRhi is created (D). Handing such a QRhi to Qt Quick through `QQuickGraphicsDevice::fromRhi` is the inferred route for sharing our device and queue with Qt Quick (U, to test) |
| Importing a texture we render | `texture_create_from_extension`. The docs name only `VkImage`; the D3D12 driver accepts an `ID3D12Resource*` as a non-owned resource (D, source `drivers/d3d12/rendering_device_driver_d3d12.cpp` at 4.7-stable). **The driver assumes the resource starts in `RENDER_TARGET` (or `DEPTH_WRITE`)**, and no public API declares another state (D source; U for the absence) | `QRhiTexture::createFrom(NativeTexture)` takes an `ID3D12Resource*` plus its `D3D12_RESOURCE_STATES`; `setNativeLayout()` updates the state after our rendering (D, [qrhitexture](https://doc.qt.io/qt-6/qrhitexture.html), source `qrhid3d12.cpp` at v6.12.0) |
| Recording into the framework's frame | Public barriers are deprecated no-ops ("Barriers are automatically inserted"); `submit()` and `sync()` exist only on local RenderingDevices (D) | `beginExternal()` and `endExternal()` allow native command recording; the D3D12 command list is valid only between `beginFrame` and `endFrame`, and Qt's command-buffer state is invalid after `endExternal()` (D, [qrhicommandbuffer](https://doc.qt.io/qt-6/qrhicommandbuffer.html)). The command-list handle needs `Qt6::GuiPrivate` (D) |
| Fences | No public fence or semaphore API (D, by absence). Cross-queue sync means `Wait`/`Signal` on the exposed queue (U) | No public fence API (U, none found). The backend uses an internal fence (D source). Cross-queue sync works the same way as for Godot (U) |
| Resize and teardown | No D3D12-specific guidance (U). Freeing an imported RID does not release our resource (D source) | The item's offscreen texture "is automatically recreated" on resize; `releaseResources()` handles cleanup (D, [qquickrhiitem](https://doc.qt.io/qt-6/qquickrhiitem.html)) |
| API stability | RenderingDevice is public API | QRhi has no source or binary compatibility guarantee across Qt versions (D, [qrhi](https://doc.qt.io/qt-6/qrhi.html)), so the Qt version must be pinned |

Reading: both expose what the S-B handoff needs when our work runs on the framework's own device and queue (device, queue, a non-owned imported texture). Qt documents importing our device and queue into a QRhi, and adopting our device into Qt Quick; carrying our queue into Qt Quick through `fromRhi` is still to be tested. Godot documents neither, so for Godot the S-B handoff works on the framework's device and queue. Qt documents the resource-state handover, resize and teardown explicitly. Godot's import path is documented only for Vulkan, works for D3D12 by source reading, and fixes the incoming state, so the S-B handoff must deliver resources in `RENDER_TARGET` state or the smoke test must show otherwise.

## 2. UI and text for the decided design

The decided design (owner decisions of [2 October](../../wiki/decisions/owner-decisions-2026-10-02-design.md) and the [scope page](../../wiki/decisions/owner-decisions-2026-10-02-scope.md)) is UI-heavy:
- a relationship strip inside both views;
- term cards with notation;
- the 20/50/80-word text budgets;
- two theme families with tunable ranges;
- two linked views;
- small rewards in place.

| Need | Godot 4.7 | Qt Quick 6.12 |
|---|---|---|
| Rich text | RichTextLabel with BBCode: tables, images, fonts, OpenType features, effects (D, [BBCode docs](https://docs.godotengine.org/en/stable/tutorials/ui/bbcode_in_richtextlabel.html)) | `Text` with RichText (HTML 4 subset) and MarkdownText (CommonMark plus tables); distance-field, native and, since 6.7, GPU curve rendering (D, [Text](https://doc.qt.io/qt-6/qml-qtquick-text.html)) |
| Math notation (term cards, strip) | Not documented; would need a third-party library or prerendered SVG (U) | Not documented; same options (U) |
| Accessibility (screen readers) | AccessKit since 4.5, "still in its experimental phase"; 4.7 adds landmark navigation (D, [4.5](https://godotengine.org/releases/4.5/), [4.7](https://godotengine.org/releases/4.7/)) | Built-in accessibility through the `Accessible` attached type (D). The Windows mechanism was not confirmed on the pages read (U) |
| High-DPI | hiDPI setting (U, no page cited) | Per-Monitor DPI Aware V2 by default, fractional scale factors (D, [highdpi](https://doc.qt.io/qt-6/highdpi.html)) |
| Layout and theming | Control containers and Theme resources (U, no page cited) | Qt Quick Layouts; Flexbox layout since 6.10 (D, releases page); Controls styling not checked (U) |

Reading: this is the weightiest dimension, because design is the core of 1.0. Qt Quick is a UI toolkit with documented rich text, high-DPI handling and mainstream accessibility. Godot's UI is capable, but its accessibility is experimental, and UI is not its centre. Neither documents math rendering, so the math path (likely prerendered SVG from the theory book's own toolchain) is an experiment for both. These are documented capabilities, not a judgement of how they feel in use; that belongs to the greybox and G-tests.

## 3. Licence and packaging

| Item | Godot 4.7 | Qt 6.12 |
|---|---|---|
| Licence | MIT; ship the licence text and COPYRIGHT.txt notices (D, [complying with licenses](https://docs.godotengine.org/en/stable/about/complying_with_licenses.html)) | Qt Quick: commercial, LGPLv3 or GPLv2 (D, [Qt Quick](https://doc.qt.io/qt-6/qtquick-index.html)). LGPL obligations: dynamic linking, user can relink a modified Qt, source or written offer, licence text, notice (D, [LGPL obligations](https://www.qt.io/licensing/open-source-lgpl-obligations)) |
| Build tools | Standard export templates | `qsb` (Qt Shader Tools) is GPLv3-only for open-source users (D, [Qt Shader Tools](https://doc.qt.io/qt-6/qtshadertools-index.html)); as a build-time tool this is probably acceptable (U, needs licence review, an owner item) |
| Runtime | .NET 8 or later; desktop C# export bundles the needed .NET parts (D) | Qt DLLs and plugins deployed with the application |
| Package size | Not documented (U) | Not documented (U) |

Reading: Godot's licence is simpler. Qt under LGPL is workable but adds obligations and a licence review of `qsb`. A commercial Qt licence would be a new cost, which is an "Ask the owner" item.

## 4. Later macOS and Linux versions

The architecture rule is portable HLSL written once and a small renderer backend interface (owner decision of [2 October](../../wiki/decisions/owner-decisions-2026-10-02.md)).

- **Godot.** RenderingDevice targets Vulkan, D3D12, Metal and WebGPU (D). Its shaders are GLSL 450 compiled to SPIR-V (D, [compute shaders](https://docs.godotengine.org/en/stable/tutorials/shaders/compute_shaders.html)). Our HLSL is not a native input. The paths are DXC to SPIR-V and then Godot's pipeline, or our own backend per platform (U).
- **Qt.** QQuickRhiItem runs the same code on Vulkan, Metal, D3D11/12 and OpenGL (D). Qt's shader input is Vulkan-style GLSL, translated to HLSL and MSL (D, [overview](https://doc.qt.io/qt-6/qtshadertools-overview.html)). `qsb -r` can inject our own HLSL for D3D (D, [qsb](https://doc.qt.io/qt-6/qtshadertools-qsb.html)). Metal and Vulkan would still need another shader source (U).
- **Both.** Native D3D12 code inside the framework frame is Windows-only by nature, so it belongs behind the renderer backend interface either way (U, design inference).

Reading: neither takes portable HLSL directly. Both keep the platform layer portable when our own renderer sits behind the backend interface, which the architecture already requires.

## 5. Debugging and tooling

- **Godot.** PIX event markers need a custom build with `use_pix=yes` (D, [compiling for Windows](https://docs.godotengine.org/en/stable/engine_details/development/compiling/compiling_for_windows.html)). No official statement was found on capturing official builds with PIX or RenderDoc (U).
- **Qt.** `QSG_RHI_DEBUG_LAYER=1` enables the D3D debug layer, and the D3D12 backend allows tools such as PIX (D, 6.6 blog). RenderDoc on the D3D12 path is not stated (U).

## 6. Risk against the interop requirements

| Requirement | Godot 4.7 | Qt Quick 6.12 |
|---|---|---|
| R-02: commit and refresh are reported separately | Framework-neutral: lives in the command layer and view model | Same |
| R-04: ordered shutdown within its grace period | Teardown of imported resources undocumented for D3D12 (U); must be shown in the smoke test | `releaseResources()` documented (D); must still be shown |
| R-14: no synchronous engine call on the UI thread; a failed or slow lookup shows an error in place and never reaches the application-exit path | Godot's main loop owns the UI thread; engine calls must be asynchronous by design, and lookup failures must be caught before any engine-level quit path (U) | QQuickRhiItem renders on the scenegraph render thread, separate from the GUI thread (D); engine calls must still be asynchronous, and failures must be shown in place, never through an exit path (U) |
| R-17: device loss during any input leaves no publication or input state held; the renderer recovers or reports | No D3D12 device-loss guidance found (U). The smoke test must show that a device loss during a drag or key sequence releases input capture and any publication guard, not only that the loss is reported | No device-loss guidance found on the pages read (U). Same check: released input and publication state, then recovery or a report |

## 7. Questions only the experiments answer

These go to the S-A2 and S-D smoke tests and runs ([E-2.4-02](packets/renderer/E-2.4-02-sa2-godot.md), [E-2.4-03](packets/renderer/E-2.4-03-sd-qt.md)):

1. Godot: does an imported `ID3D12Resource` that is not in `RENDER_TARGET` state get correct barriers, or must S-B always hand it over in that state?
2. Both: where can our `Wait`/`Signal` go on the exposed queue, so that our command lists finish before composition, without stalls or cross-frame hazards?
3. Godot: can a C# host get the device and queue before the first frame and share resources without a copy? Do device loss or a resize invalidate imported RIDs?
4. Qt: do `createFrom` and QQuickRhiItem behave correctly on Qt's own device and queue through resize and `releaseResources()`? On pinned Qt 6.12, does a QRhi created with our device and queue and handed to Qt Quick through `fromRhi` keep the native device and queue identities, and do QQuickRhiItem rendering, resize and teardown work on it? Does the device-only route (`fromDeviceAndContext`) work as a fallback?
5. Both: can PIX and RenderDoc capture each stack with our command lists visible and named?
6. Both: what is the minimal Windows package size?
7. Both: do NVDA and Narrator read the relationship strip and the term cards?
8. Both: which math rendering path (prerendered SVG or a library) stays sharp at high DPI?
9. Both: does the stack meet the renderer gate with the interop overhead included? No source answers this.

## 8. What this suggests for day 7 (not a selection)

- The plan's selection rule prefers, among passing candidates, the one the design track can use with the least extra work ([renderer-experiment-plan](renderer-experiment-plan.md) section 3). Godot is the design track's Look Lab framework: if S-A2 passes, the Look Lab grows into the product shell; choosing Qt means rebuilding that shell ([E-2.4-02](packets/renderer/E-2.4-02-sa2-godot.md)). This switching cost is real and must be weighed against section 2; it is not estimated here, because it depends on how much of the Look Lab exists by day 12.
- If both pass the gate, section 2 and the switching cost weigh most, because design is the core of 1.0. On paper Qt Quick is better documented for the UI-heavy design and for the D3D12 handover; Godot is simpler to license.
- A commercial Qt licence, a licence review of `qsb`, or any new cost goes to the owner.
- The selection is made only from measured S-A2 and S-D results under the gate in [renderer-candidates](../../wiki/decisions/renderer-candidates.md), with this page as context.
