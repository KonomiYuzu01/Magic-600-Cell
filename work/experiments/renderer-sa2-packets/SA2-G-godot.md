# Packet SA2-G: the Godot 4.7.2 .NET interop smoke test (E-2.4-02 level 1)

Run from the `claude/renderer-sb` checkout:
`python tools/agents/codex_review.py --kind implement --model gpt-6.1-sol --effort max --timeout 7200 --packet work/experiments/renderer-sa2-packets/SA2-G-godot.md`

The parallel packet SA2-N (`SA2-N-native.md`) owns `native/`: the producer DLL `sa2_interop.dll`, its self-test and `native/build.cmd`. This packet owns the Godot project, the run script and the project checks. Both implement the same committed interface, `work/experiments/renderer-sa2/native/include/sa2_interop.h` (ABI version 1), and the code layout `work/experiments/renderer-sa2/code_layout.json`. Neither file may change, and the DLL does not exist in this worktree: write against the header.

## 1. Goal and acceptance
- Goal: a Godot 4.7.2 .NET project and a run script that show, on the owner's machine, whether a changing, sequence-numbered texture produced by Direct3D 12 work is displayed correctly through Godot's own D3D12 backend. The test also covers device ownership, synchronisation, window resize and teardown (E-2.4-02 level 1).
- It must also answer PR #43, section 7, for Godot: does an imported `ID3D12Resource` need to arrive in RENDER_TARGET state? Answers are smoke-test results, kept apart from documentation claims.
- Acceptance check: `python work/experiments/renderer-sa2/check_project.py` exits 0 (static checks and Python fixtures only; no Godot, no GPU, no .NET build).
- Done when:
  - the project, the C# harness, `run_smoke.py` with the run matrix in section 6, `check_project.py` and `README.md` are written;
  - the harness implements all four routes, both queue modes, both handovers, resize, teardown and the result file;
  - the run steps for the owner's machine are written down.
- Non-goals and deviations from E-2.4-02:
  - Level 2 is out of scope: no geometry port, asset digests or reference outputs.
  - Also out of scope: PresentMon, timing claims, NVIDIA features, export templates and product UI.

## 2. Actual problem and reproduction

Godot 4.7.2 source facts, read from tag `4.7.2-stable`. The smoke test exists to confirm them at runtime.
- **S1, import.**
  - `RenderingDevice.texture_create_from_extension` keeps the raw `ID3D12Resource*` without AddRef, and freeing its RID never releases the resource.
  - The D3D12 driver's legacy-barrier tracking starts the imported texture at RENDER_TARGET.
  - The RID is "mutable": it gets a render-graph tracker at usage NONE.
- **S2, barriers.**
  - Godot uses enhanced barriers whenever `D3D12_OPTIONS12.EnhancedBarriersSupported` is true, with the generic layouts (`SHADER_RESOURCE`, `COPY_SOURCE`, …).
  - The tracker maps usage NONE to layout UNDEFINED, so the first Godot use of an imported texture discards its contents.
  - The tracked usage persists across frames: after a texture has been sampled, Godot issues no further barrier for sampling, and after a copy-from, none for the next copy-from.
- **S3, Texture2DRD.** `Texture2DRD` always makes a shared view of the RD texture. Under `DEBUG_ENABLED`, which includes the editor binary, the D3D12 driver refuses a shared view of a texture it did not allocate. So in the editor binary, `Texture2DRD` cannot show an imported texture; release templates skip that check. When the view is refused, `TextureStorage::texture_rd_initialize` still creates the RenderingServer texture, but `RenderingServer.texture_get_rd_texture` of it returns an invalid RID.
- **S4, Godot-created textures.**
  - A texture made with `TEXTURE_USAGE_COLOR_ATTACHMENT_BIT` has format `R8G8B8A8_TYPELESS`, `ALLOW_RENDER_TARGET` and a black optimized clear value. It starts at layout UNDEFINED (enhanced barriers) or `COPY_DEST` (legacy).
  - Without initial data, a texture with COLOR_ATTACHMENT or STORAGE usage is cleared at its first Godot use (`pending_clear`).
  - `get_driver_resource(DRIVER_RESOURCE_TEXTURE, rid, 0)` returns its `ID3D12Resource*`. Making a `Texture2DRD` of it does not recreate the resource.
- **S5, validation.** With `--gpu-validation`, Godot enables the D3D12 debug layer and registers its own info-queue callback (`IGNORE_FILTERS`). Its errors and warnings appear in Godot's output.
- **S6, threads.**
  - In the default thread model (`rendering/driver/threads/thread_model = 1`, "safe"), `RenderingServer.call_on_render_thread` runs the callable at once on the main thread. With `--render-thread separate`, it queues the callable, first in first out, with the `RenderingServer.draw()` calls.
  - `frame_pre_draw` is emitted on the main thread synchronously in `draw()`, before the frame is queued or drawn. `frame_post_draw` is emitted late (deferred) with a separate render thread. Use `frame_pre_draw` to know that iteration f's draw follows the render step of frame f.
  - `Texture2DRD.texture_rd_rid` defers its work to the render thread.
- **S7, viewport readback.**
  - The root viewport's render target is `R8G8B8A8_UNORM` with `CAN_COPY_FROM` (`hdr_2d` is off by default).
  - `RenderingDevice.texture_get_data_async(rid, 0, callback)` must run on the render thread. The callback receives a tightly packed `PackedByteArray` a few frames later, on the render thread.
  - A readback requested in the render step of frame f, which is before frame f's draw, captures frame f − 1's composite.
  - A resize recreates the viewport's render target, so the RD texture behind the viewport gets a new RID.
- **S8, submission and synchronous readback.**
  - Godot submits the work it has recorded only in `swap_buffers`, at the end of a drawn frame. A frame that is not drawn (every window minimised) leaves its recorded work pending.
  - `RenderingDevice.texture_get_data` records a copy, so the texture's tracked usage becomes copy-from (layout `COPY_SOURCE`). It then calls `_flush_and_stall_for_all_frames`, which:
    - waits for all frames, running their pending `texture_get_data_async` callbacks on the calling thread;
    - ends and submits the work recorded so far, and waits for it;
    - disposes of textures freed with `free_rid` in the current frame, through the new frame start.

Therefore, for the import and export routes, the handover state the producer leaves a texture in must be the state Godot's tracker holds:
- after a warm-up use: `SHADER_RESOURCE` for a sampled texture, `COPY_SOURCE` for a copied one;
- RENDER_TARGET only as Godot's legacy assumption at import time.

## 3. Environment and versions
- Base: branch `claude/renderer-sb` (this worktree's HEAD).
- Owner's machine:
  - Windows 11 (build 26200), NVIDIA GeForce RTX 4070 Laptop GPU plus an integrated AMD Radeon 610M, Visual Studio 2026 C++ tools.
  - Godot 4.7.2 .NET, version string `4.7.2.stable.mono.official.ed1daf0bf`, installed by `bootstrap.py install godot`. Path pattern: `%LOCALAPPDATA%\Microsoft\WinGet\Packages\GodotEngine.GodotEngine.Mono_*\Godot_v4.7.2-stable_mono_win64\Godot_v4.7.2-stable_mono_win64_console.exe`. Godot's NuGet packages are in `GodotSharp\Tools\nupkgs\` next to it (`Godot.NET.Sdk`, `Godot.SourceGenerators`, `GodotSharp`, `GodotSharpEditor`, all 4.7.2).
  - No Agility SDK folder ships with it, so Godot uses the OS D3D12 runtime.
  - .NET SDK 10.0.401 with the 8.0 targeting pack installed.
- Network rule: the build is offline. No nuget.org and no other new source domain; a new source domain is an owner decision. The integrator verified an offline build on this machine with:
  - a `nuget.config` that has `<clear />` and one source `%M600_GODOT_NUPKGS%`; NuGet expands the environment variable, and the MSBuild SDK resolver uses the same config;
  - `NUGET_PACKAGES` set to a private directory, and `DOTNET_CLI_TELEMETRY_OPTOUT=1`, `DOTNET_NOLOGO=1`, `DOTNET_SKIP_FIRST_TIME_EXPERIENCE=1` and `DOTNET_CLI_WORKLOAD_UPDATE_NOTIFY_DISABLE=1`;
  - `dotnet build <csproj> -c Debug`, with `<Project Sdk="Godot.NET.Sdk/4.7.2">`, net8.0 and `NuGetAudit` false.

  It restored only the four Godot packages and wrote the assembly to `.godot/mono/temp/bin/Debug/`.
- Godot command-line options checked against `--help` of this binary: `--path`, `--rendering-driver d3d12`, `--gpu-validation`, `--render-thread safe|separate`, `--disable-vsync`, `--max-fps`, `--resolution WxH`, `--position X,Y`, `--windowed`, `--log-file <file>`, and `--` before user arguments (`OS.get_cmdline_user_args()`).
- Godot API (4.7.2 class reference; C# uses the PascalCase names):
  - `RenderingServer`:
    - `get_rendering_device()`, `call_on_render_thread(callable)`, `get_current_rendering_driver_name()`, `get_video_adapter_name()`, `get_video_adapter_api_version()`;
    - `texture_get_rd_texture(rid, srgb=false)`;
    - signals `frame_pre_draw` and `frame_post_draw`.
  - `RenderingDevice`, textures:
    - `get_driver_resource(resource, rid, index)`;
    - `texture_create(format, view, data=[])`;
    - `texture_create_from_extension(type, format, samples, usage_flags, image, width, height, depth, layers, mipmaps=1)`;
    - `texture_copy(from, to, from_pos, to_pos, size, src_mipmap, dst_mipmap, src_layer, dst_layer)`;
    - `texture_get_data(texture, layer)`, `texture_get_data_async(texture, layer, callback)`, `texture_get_format(texture)`;
    - `set_resource_name(id, name)`, `free_rid(rid)`, `get_frame_delay()`.
  - `RenderingDevice`, compute:
    - `shader_compile_spirv_from_source(source, allow_cache=true)`, `shader_create_from_spirv(spirv, name="")`;
    - `compute_pipeline_create(shader, constants=[])`, `uniform_set_create(uniforms, shader, set)`;
    - `compute_list_begin()`, `compute_list_bind_compute_pipeline`, `compute_list_bind_uniform_set`, `compute_list_set_push_constant(list, bytes, size)`, `compute_list_dispatch`, `compute_list_end()`.
  - Enum values:
    - `DriverResource`: LOGICAL_DEVICE 0, PHYSICAL_DEVICE 1, COMMAND_QUEUE 3, TEXTURE 5;
    - `DATA_FORMAT_R8G8B8A8_UNORM` 36, `TEXTURE_TYPE_2D` 1, `TEXTURE_SAMPLES_1` 0;
    - usage bits: SAMPLING 1, COLOR_ATTACHMENT 2, STORAGE 8, CAN_COPY_FROM 128, CAN_COPY_TO 256.
  - Other classes: `Texture2DRD.texture_rd_rid`; `OS.get_cmdline_user_args()`, `OS.has_feature("editor")`; `Engine.get_version_info()`; `Window.size`; `SceneTree.quit(exit_code)`; `TextureRect.stretch_mode` (KEEP is 2) and `expand_mode` (IGNORE_SIZE is 1); `CanvasItem.texture_filter` (NEAREST is 1).
- Evidence kind in the sandbox: source/fixture only.

## 4. Necessary source and evidence
- `work/experiments/renderer-sa2/native/include/sa2_interop.h`: the native API, its call order, fences, states and teardown rules. Read all of it.
- `work/experiments/renderer-sa2/code_layout.json`: the code image, the decode rule and test vectors.
- `docs/progress/1.0/packets/renderer/E-2.4-02-sa2-godot.md` (level 1) and `docs/progress/1.0/renderer-experiment-plan.md`, section 3.
- `docs/progress/1.0/requirements-from-screening.md`, requirements R-04 (bounded, ordered shutdown), R-14 (no unbounded waits on the UI thread) and R-17 (device loss: recover or report).
- `work/experiments/renderer-sb/handoff/README.md` and `RESULT.md`: the S-B fence protocol and drain rule this test carries into Godot.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | Godot's C# project restores offline from Godot's own packages | integrator built a minimal `Godot.NET.Sdk/4.7.2` project with the nuget.config and environment in section 3 | `dotnet build -c Debug` | pass: only the four Godot packages were restored |

## 6. Constraints and owned files

Owned files (all under `work/experiments/renderer-sa2/`):
- `project/**`;
- `run_smoke.py`, `smoke_summary.py`, `sa2_reference.py`, `check_project.py`;
- `README.md`.

Change nothing else. Do not leave build output in the worktree (`project/.godot/`, `bin/`, `obj/`).

Optional: compile the C# code in the sandbox.
- Copy `project/` to `work/experiments/renderer-sa2/project/.sandbox-build/`.
- Point `M600_GODOT_NUPKGS` at the NuGet folder from section 3, and `NUGET_PACKAGES` and `DOTNET_CLI_HOME` into `.sandbox-build/`.
- Build with the `dotnet build` command from `run_smoke.py` step 4.
- Delete `.sandbox-build/` before you finish, and report whether the build succeeded.

Synthetic content only: the test writes code images, never labels from a personal session, and it never reads a user database.

### Project (`project/`)
- **`project.godot`.**
  - Application: name "SA2 Smoke", main scene `res://Main.tscn`.
  - Window: 1280 × 720, resizable, `display/window/vsync/vsync_mode=0`.
  - .NET: `dotnet/project/assembly_name="SA2Smoke"`.
  - Rendering:
    - `rendering/rendering_device/driver.windows="d3d12"`;
    - `rendering/rendering_device/fallback_to_vulkan=false` and `fallback_to_opengl3=false`;
    - `rendering/renderer/rendering_method="forward_plus"`.
  - Nothing persistent in `user://`: `debug/file_logging/enable_file_logging.pc=false`, `rendering/shader_compiler/shader_cache/enabled=false` and `rendering/rendering_device/pipeline_cache/enable=false`.
- **`SA2Smoke.csproj`.** `<Project Sdk="Godot.NET.Sdk/4.7.2">`, `net8.0`, `EnableDynamicLoading` true, `AllowUnsafeBlocks` true, `NuGetAudit` false, and no `PackageReference`.
- **`nuget.config`.** Exactly `<clear />` and one source, `%M600_GODOT_NUPKGS%`.
- **`Main.tscn`.** A root `Control` (full rect) with the script `res://Smoke.cs`, referenced by path without a uid. It has one child `TextureRect`: full rect, stretch KEEP, expand IGNORE_SIZE, filter NEAREST, at position (0, 0).
- **C# files** (`Smoke.cs`, `Native.cs`, `CodeLayout.cs`, and more if useful):
  - `Native.cs` loads the DLL from the absolute path given by `--sa2-dll` with `NativeLibrary.Load`, and binds every header function with `NativeLibrary.GetExport` and `delegate* unmanaged<…>` function pointers.
  - It checks `sa2_abi_version() == 1` and the struct sizes (`Sa2DeviceInfo` 48, `Sa2Config` 28, `Sa2DebugCounts` 128 with `fixed int Ids[16]`) before any other call.
  - Constants equal the header's.

### Harness arguments (after `--`; any unknown or malformed argument: one stderr line and exit 2)
- `--sa2-out <abs json>` (required).
- `--sa2-dll <abs path>`: required for every route.
- `--sa2-route rd-compute|export|import-copy|import-texture2drd`
- `--sa2-queue same|own`
- `--sa2-handover tracked|render-target`
- `--sa2-barriers match|legacy|enhanced` (the DLL's barrier API)
- `--sa2-frames N` (run frames, default 1200)
- `--sa2-warmup N` (default 3; 0 for `render-target` and `rd-compute`)
- `--sa2-resize-every N` (0 = off)
- `--sa2-verify-every N` (default 50)
- `--sa2-device-loss-at N` (0 = off)
- `--sa2-timeout-ms N` (default 5000; used for the DLL's waits and its drain)

### Routes
- **rd-compute.** A ring of 3 RD textures (`R8G8B8A8_UNORM`; STORAGE | SAMPLING | CAN_COPY_FROM) written by a GLSL compute shader through RenderingDevice. The shader is an inline string compiled with `shader_compile_spirv_from_source`. Push constants (32 bytes): the code's low and high words, the packed fill colour, the width and the height. Each slot is shown with `Texture2DRD`.
  - The DLL is attached only to count debug messages: `SA2_QUEUE_SAME`, no slots.
  - This is the pure-Godot baseline, including Godot's own validation messages.
- **export** (zero copy).
  - A ring of 3 textures that Godot creates (SAMPLING | COLOR_ATTACHMENT | CAN_COPY_FROM, named "SA2 export slot k" with `set_resource_name`).
  - Each is registered with `sa2_register_slot(…, godot_owned 1)`, using the pointer from `get_driver_resource(TEXTURE, rid, 0)`, and shown with `Texture2DRD`.
- **import-copy.**
  - A ring of 3 textures made by `sa2_create_texture(…, initial_state SA2_STATE_RENDER_TARGET)`. They are imported with `texture_create_from_extension(TEXTURE_TYPE_2D, R8G8B8A8_UNORM, SAMPLES_1, SAMPLING | COLOR_ATTACHMENT | CAN_COPY_FROM, ptr, w, h, 1, 1, 1)`, named "SA2 imported slot k", and registered with `godot_owned 0`.
  - Each frame Godot copies the produced slot (`texture_copy`) into one Godot display texture (SAMPLING | CAN_COPY_TO | CAN_COPY_FROM), which is shown with `Texture2DRD`.
- **import-texture2drd** (probe for S3). As import-copy, but each imported RID is wrapped in a `Texture2DRD` directly.
  - If `RenderingServer.texture_get_rd_texture(texture2drd.GetRid())` is not valid after the wrap has been processed on the render thread, the run ends with status `unsupported` and reason `texture2drd-refused` (full teardown).
  - Otherwise it runs like export, with status `recorded`.

Handover states (the `sa2_config` fields):
- **`tracked`.**
  - `state_before_write = state_after_write = SA2_STATE_ALL_SHADER_RESOURCE` for export and import-texture2drd, and `SA2_STATE_COPY_SOURCE` for import-copy.
  - After each (re)build of the ring, a warm-up of one frame per slot, in which Godot shows or copies the slot without a write. Godot's tracker then holds that state.
- **`render-target`** (probe for the PR #43 question): `before = after = SA2_STATE_RENDER_TARGET`, no warm-up.
- `queue_mode` comes from `--sa2-queue`, `barrier_api` from `--sa2-barriers` and `wait_timeout_ms` from `--sa2-timeout-ms`; `debug_callback` is 1.

### Frame protocol
The main thread drives a state machine: init → wrap → warm-up → run → (rebuild → wrap → warm-up → run)* → teardown.
- All `RenderingDevice` and `sa2_*` calls run on the render thread, inside `RenderingServer.CallOnRenderThread`. The step for frame f is queued from `_Process` of frame f.
- Scene-tree work stays on the main thread: TextureRect changes, creating and `Dispose()` of `Texture2DRD`, window size and quit.
- State shared between the threads, including readback callbacks, is guarded by a lock. Results travel from the render thread to the main thread through a mailbox.
- The harness must work with both `--render-thread safe` and `separate`.
- Never hold the lock while calling Godot or the DLL: `texture_get_data` runs pending readback callbacks on the calling thread (S8).
- **Godot flush:** a synchronous `texture_get_data` of a small texture the harness creates at init for this purpose: 4 × 4, `R8G8B8A8_UNORM`, SAMPLING | CAN_COPY_FROM, with initial data (S8).
- Frame numbers:
  - f counts `_Process` calls, and the render step queued in iteration f is step f;
  - slot k of the ring is used in the iterations with f % 3 = k, in warm-up and run alike;
  - in the export and import-texture2drd routes, iteration f sets the TextureRect to slot f % 3's `Texture2DRD`.

The render step of a run frame f, in this order:
1. `sa2_signal_godot_free(f - 1)`. Every step after `sa2_attach` calls it, warm-up and rebuild steps included.
2. `sa2_produce(slot f % 3, f, sequence f, generation)`.
3. `sa2_godot_wait_ready(f)`.
4. `sa2_mark_shown(slot, f)`.
5. import-copy only: `texture_copy` into the display texture.
6. Every `verify_every` run frames:
   - `sa2_verify_slot` (native routes);
   - `RenderingDevice.texture_get_data` of the texture Godot shows (the slot, or the display texture), compared texel by texel with the expected image.
7. If frame f − 1 is eligible: request `texture_get_data_async` of the root viewport's RD texture, re-read each step with `texture_get_rd_texture`. The callback decodes both corners at the ring texture's position and size and compares them with the code of frame f − 1.

More rules for the steps:
- **Warm-up steps** call `sa2_mark_shown` for the slot Godot uses in that frame, and never `sa2_godot_wait_ready`.
- **rd-compute** replaces steps 1 to 4 with the compute dispatch.
- **Frames not drawn.** Godot draws every iteration unless every window is minimised. A frame that is not drawn breaks two assumptions:
  - its recorded work stays unsubmitted (S8), so `sa2_signal_godot_free` would claim work that has not run;
  - after an RD verify of an export slot, Godot's tracked state for that slot stays `COPY_SOURCE`.

  So the first iteration without `frame_pre_draw` ends the run: status `fail`, reason `not-drawn`. No further produce, signal or readback follows, and teardown starts.
- **Eligible frames.** Frame f − 1's readback is requested at step f only when all of these hold:
  - frame f − 1 is a run frame of the current ring generation;
  - the viewport's RD texture at step f is the same RID as at step f − 1. After a resize, the new texture never held frame f − 1;
  - that texture is at least as large as the ring.

  Every other frame is counted as skipped, with its reason (`warmup` or `transition`).
- **Export pointer check.** After each wrap, a step re-reads `get_driver_resource(TEXTURE, rid, 0)` for every export slot. A pointer that differs from the registered one fails the run (`resource-changed`).
- **Resize.**
  - Every `resize_every` run frames the main thread sets `Window.Size` to the next size in 1280×720, 1024×640, 1440×810, 800×600 (cycling).
  - Whenever the viewport size differs from the ring size, rebuild:
    1. main thread: clear the TextureRect and `Dispose()` the `Texture2DRD`s;
    2. render step, in order:
       - a Godot flush;
       - `free_rid` of the ring, display and dependent RIDs;
       - a second Godot flush, after which Godot has disposed of them;
       - `sa2_drain`;
       - `sa2_unregister_slot` for each slot;
       - `sa2_release_texture` for each imported texture (record `refcount_after`);
    3. increase the generation (mod 4096) and create the new ring at the viewport size;
    4. main thread, on the next frames: wrap the new ring, then warm up.
  - Record the frames each transition took.
- **Device loss** (`--sa2-device-loss-at N`, a probe for R-17).
  - At run frame N, the step calls `sa2_remove_device`.
  - Afterwards, record what the DLL and Godot report: statuses, `sa2_device_removed_reason`, whether frames keep being drawn.
  - Then tear down. The status is `recorded` whatever happens after the removal, and the reasons record what was seen, including `not-drawn`.
- **Teardown** (after the run frames, or early on an `unsupported` or failed run):
  1. main thread: clear the TextureRect and `Dispose()` the `Texture2DRD`s;
  2. render steps:
     - a Godot flush, which also completes every requested readback;
     - `free_rid` of all RIDs except the flush texture;
     - a second Godot flush, then `free_rid` of the flush texture;
     - write the result file with `teardown.phase = "pending"`;
     - `sa2_drain`; `sa2_unregister_slot` for each slot; `sa2_release_texture` (record `refcount_after`);
     - `sa2_debug_counts` and `sa2_debug_messages`; `sa2_detach`;
     - write the final result;
  3. main thread: `GetTree().Quit(code)`.

  An unconfirmed drain ends the process with exit 3 inside the DLL, so the pending result is all that remains.
- **Exit codes.** 0 for status `pass`, `recorded`, or `unsupported` with `texture2drd-refused`; 1 for `fail`; 2 for argument errors. Exit 3 comes only from the DLL.

### Result file (`--sa2-out`, format `magic600-sa2-smoke-run-v1`, written atomically: a temp file, then a rename)
- `status`: `pass`|`fail`|`recorded`|`unsupported`.
- `reasons`: short codes from a fixed set, for example:
  - `decode-mismatch`, `readback-missing`, `verify-mismatch`;
  - `debug-errors`, `refcount-nonzero`;
  - `sa2-call-failed`, `driver-not-d3d12`, `engine-version`;
  - `texture2drd-refused`, `device-removed`;
  - `not-drawn`, `resource-changed`.
- `config`: the arguments, plus whether `--gpu-validation` and `--render-thread separate` were on the command line.
- `engine`: version string, `editor_build` (`OS.has_feature("editor")`), `debug_build` (`OS.is_debug_build()`), driver name, rendering method, adapter name, adapter API version.
- `device`: from `sa2_probe`.
- `dll`: ABI version, resolved barrier API, handover and initial states.
- `frames`: produced, drawn, not drawn, eligible, verified, mismatched, skipped by reason, and the first mismatch (frame, expected code, decoded code or `null`). Also readbacks requested and completed.
- `verify`: native and RD checks run, and mismatched texels.
- `resize`: rebuilds, sizes, the longest transition.
- `texture2drd_probe`, `device_loss`.
- `debug`: all `sa2_debug_counts` fields and the masked messages.
- `teardown`: phase, drain result, `refcount_after` per imported texture, slots unregistered, detached.
- `sa2_failures`: function, status, last-error text.

A run passes when:
- it was not a probe;
- every eligible frame verified, and eligible frames make up at least 90% of the run frames outside warm-up and transitions;
- all readbacks completed, and no verify reported a mismatch;
- no `sa2_*` call failed;
- with validation on, `error` and `corruption` are 0;
- teardown completed with the drain confirmed, every `refcount_after` is 0, and the context detached.

### `run_smoke.py`
Python 3 standard library only.

Steps:
1. Locate Godot: `--godot <exe>`, else exactly one match of the path pattern in section 3. Require the exact version string from `--version`. The NuGet folder is relative to the exe.
2. Record the source identity:
   - `git rev-parse HEAD`;
   - a sha256 source digest over sorted relative paths and file digests under `work/experiments/renderer-sa2/`, excluding `results/`, `project/.godot/`, `bin/`, `obj/` and `__pycache__/`;
   - whether those files match HEAD. `work/` is ignored by Git, so compare the walked files with `git ls-files` and `git diff --quiet HEAD --` for that directory.
3. Build the DLL with `cmd.exe /d /c call native\build.cmd <private>\native-build`. It produces `sa2_interop.dll` and `sa2_selftest.exe` in that directory's root.
4. Copy `project/` to `<private>\project\`, without `.godot/`, `bin/` and `obj/`. Build that copy offline with the environment from section 3: `dotnet build SA2Smoke.csproj -c Debug -nologo -nodeReuse:false -p:UseSharedCompilation=false`, with `NUGET_PACKAGES` and `DOTNET_CLI_HOME` under the private directory. Afterwards, check that the package cache holds only the four Godot packages. The source tree never gets build output.
5. Record the sha256 of the DLL and of `SA2Smoke.dll`.

Run matrix: one Godot process per run, run in order, on the private copy. Common options: `--path <private>\project --rendering-driver d3d12 --windowed --resolution 1280x720 --position 40,40 --disable-vsync --log-file <private log>`. There is no editor or import step. stdout and stderr are captured.

| Run | Route | Queue | Handover | DLL barriers | Validation | Render thread | Frames | Resize every | Expected |
|---|---|---|---|---|---|---|---|---|---|
| R0 | `sa2_selftest.exe --hardware --debug` | | | | | | | | pass; on failure, stop the matrix |
| R1 | rd-compute | same | – | match | on | safe | 1200 | 200 | pass |
| R2 | export | same | tracked | match | on | safe | 1200 | 200 | pass |
| R3 | export | own | tracked | match | on | safe | 1200 | 200 | pass |
| R4 | import-copy | same | tracked | match | on | safe | 1200 | 200 | pass |
| R5 | import-copy | own | tracked | match | on | safe | 1200 | 200 | pass |
| R6 | import-texture2drd | same | tracked | match | on | safe | 300 | 0 | unsupported `texture2drd-refused` (editor binary) |
| R7 | export | same | render-target | match | on | safe | 300 | 0 | recorded |
| R8 | import-copy | same | render-target | match | on | safe | 300 | 0 | recorded |
| R9 | export | own | tracked | match | off | safe | 3000 | 500 | pass |
| R10 | export | own | tracked | match | on | separate | 1200 | 200 | pass |
| R11 | export | own | tracked | legacy | on | safe | 1200 | 0 | recorded |
| R12 | export | own | tracked | match | on | safe | 120 | 0 | exit 3, the DLL's stderr line, and a pending result (`M600_SA2_INJECT_UNCONFIRMED_DRAIN=1` for this run only) |
| R13 | export | own | tracked | match | on | safe | 400 | 0 | recorded; only with `--device-loss`, removal at run frame 200. As expected whenever the process ends within its timeout, whatever the exit code: Godot may abort after the device is removed. Record the exit code, whether a result file exists and what it says |

- Judging: each run gets `as-expected` or `unexpected`, against its row and against `--expect-adapter` (default "NVIDIA GeForce RTX 4070 Laptop GPU").
- Timeouts: 300 s per run, 120 s for R13. On a timeout, kill only that run's process tree (`taskkill /PID <pid> /T /F`) and record `timeout`.
- Godot output: count lines with `ERROR:` and with `WARNING:`.
- Options: `--only R2,R5`, `--device-loss`, `--skip-build` (reuse the newest private build), `--dry-run` (no build and no processes; prints the matrix and the command lines with placeholders) and `--write-summary`.

Private outputs go under `work/loop-memory/perf/renderer/sa2-smoke/<UTC stamp>/` of the repository root: builds, logs, per-run results and a private summary.

With `--write-summary`, `smoke_summary.py` writes `work/experiments/renderer-sa2/results/sa2-smoke-summary.json` (format `magic600-sa2-smoke-summary-v1`):
- It contains:
  - the source identity, the DLL and assembly digests, the Godot version string and editor flag;
  - the adapter name and the UMD version formatted as a driver version;
  - `enhanced_barriers`;
  - per run: the row, `status`, judgement, reasons, frame counts, verify counts, debug counts and distinct IDs, the teardown fields and the exit code.
- It is built from an allowlist of fields. Free text never enters the summary: no messages, last errors, paths, user names, LUIDs, vendor or device IDs, or command lines.
- A final scan refuses to write if any string matches a drive path, `\Users\`, `/Users/`, `AppData`, a `%VAR%` pattern or the current user name.

### `check_project.py` (acceptance; runs in the sandbox)
- **Files and settings.**
  - Required files exist.
  - The `project.godot` settings, csproj properties and `nuget.config` are as specified, with no `http` source and no `PackageReference`.
- **C# against the header.** Every `SA2_API` function in the header has a binding in `Native.cs` with the same name and number of parameters. The status, queue, barrier and state constants, the struct sizes and the code-layout constants in the C# files equal the header and `code_layout.json`.
- **Python reference** (`sa2_reference.py`):
  - the CRC and code test vectors from `code_layout.json`;
  - expected-image round trips at 128 × 128 and 160 × 144, where both corners decode and a flipped block is rejected;
  - the readback decode at a viewport larger than the ring.
- **Run script.** `run_smoke.py --dry-run` exits 0 and lists R0 to R12, and R13 only with `--device-loss`.
- **Summary writer.**
  - A synthetic run result with a planted drive path and a planted user name is refused or stripped.
  - A clean synthetic result is written to a temp directory, never to `results/`, and passes the scan.
- **Owned files.** No drive path with a `Users` component, no `/Users/` and no `AppData\` literal; `.godot/`, `bin/`, `obj/` and `.sandbox-build/` are ignored. Temp directories are made with plain `mkdir` under `tempfile.gettempdir()` and removed afterwards (never `mkdtemp` or `TemporaryDirectory`).

### `README.md`
- Prerequisites and the owner's commands. The full matrix, `--only`, `--device-loss` and `--write-summary`.
- Run only on an idle machine: never during PresentMon captures, builds or Codex implementation calls.
- What each run proves.
- Where private outputs go, and what may be committed.
- The Godot API notes:
  - S1 to S8, each marked "source-verified against 4.7.2-stable" or "runtime-confirmed by run Rn". Runtime confirmation is pending until the owner's runs.
  - What is not verified: release export templates; Godot's legacy-barrier path when the device supports enhanced barriers.
- How the results answer the PR #43 question and the level-1 items: device ownership, synchronisation, resize, teardown, sequence numbers.

```implement-contract
{"allowed_files": ["work/experiments/renderer-sa2/project/*", "work/experiments/renderer-sa2/run_smoke.py", "work/experiments/renderer-sa2/smoke_summary.py", "work/experiments/renderer-sa2/sa2_reference.py", "work/experiments/renderer-sa2/check_project.py", "work/experiments/renderer-sa2/README.md"], "acceptance_check": ["python", "work/experiments/renderer-sa2/check_project.py"], "stop_condition": "the Godot project, the C# harness with all four routes, both queue modes, both handovers, resize, device-loss probe and teardown, run_smoke.py with the R0-R13 matrix and the sanitised summary writer, check_project.py and README.md are written, and check_project.py passes"}
```

## 7. Required return format
- Implementation: changes only in the assigned worktree. The final message lists:
  - the changed files;
  - the acceptance result;
  - what was not verified in the sandbox: every Godot, GPU and .NET build step;
  - any Godot API use not listed in section 3, with the reason it is believed to exist in 4.7.2;
  - open points.
