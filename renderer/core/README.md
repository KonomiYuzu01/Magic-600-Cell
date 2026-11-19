# Renderer core (S-A2, ABI 5)

`renderer/core/` is the product copy of the selected S-A2 renderer core. It is a native Direct3D 12 producer DLL, `sa2_interop.dll`, that a Godot 4.7.2 .NET host loads. It draws into textures that Godot shares with it, on Godot's device and queue. W-J's per-piece pose path is added as ABI 3, so W-J item 5 (S4, I_a, I_b) and the W3 re-acceptance can run on the S-A2 host with unchanged bars. ABI 4 adds the product API that the Rust display host calls, one C call per RM.14 operation. ABI 5 adds route B: the core's own device presenting in a DirectComposition layer. Route B is accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks. `PROVENANCE.md` lists every file's source under `work/experiments/` and how it changed. The build compiles nothing from there.

## ABI 3

`include/sa2_interop.h` is the contract. `SA2_ABI_VERSION` is 5. The ABI 1 to ABI 3 declarations, layouts and behaviour are kept, and `sizeof(sa2_scene_config)` is still 32. ABI 3 added:

```c
typedef struct sa2_wj_config {   /* 48 bytes on x64 */
    uint32_t struct_size, flags; /* flags: SA2_SCENE_NO_VRAM only */
    const char* menu;            /* "S4", "I_a" or "I_b" */
    const char* data_utf8;       /* the directory prepare_wj.py wrote */
    double turn_ms;              /* 0 < turn_ms <= 10000 */
    const char* inject;          /* NULL or one of W-J's five faults */
    uint32_t trace_ms, reserved; /* 1..3600000 ms; reserved = 0 */
} sa2_wj_config;
SA2_API int32_t sa2_scene_load_wj(sa2_context* ctx, const sa2_wj_config* config);
SA2_API int32_t sa2_scene_lattice_check(sa2_context* ctx, const char* directory_utf8);
```

A W-J scene uses the ABI 2 calls (`sa2_scene_produce`, trace begin and end, `sa2_scene_write_run`, `sa2_scene_geometry_check`, `sa2_identity`, `sa2_scene_unload`) with W-J's revision protocol, its pose check (`pose_check.json`), its geometry references and its output formats. The header comment at the end of `sa2_interop.h` gives the details.

W1..W4 load with `sa2_scene_load` as before. They keep S-B's readers, label protocol, checks and output files. They draw through W-J's pose-path shaders with identity poses and the pose path disabled. The identity lists `lattice_cs.dxil` as well.

## ABI 4

ABI 4 adds one C call per RM.14 operation of the Rust display host (`rust/c600-display/src/host.rs`). Every new struct starts with `struct_size`, every call returns a status and sets the last error, and invalid arguments return `SA2_E_INVALID_ARGUMENT` before any state check. The new status `SA2_E_UNAVAILABLE` (10) means the core cannot perform the operation and did nothing.

| RM.14 operation | C call | Struct (bytes on x64) |
| --- | --- | --- |
| `describe_backend` | `sa2_describe` | SupportReport as JSON |
| `adopt_engine` | `sa2_engine_adopt` | `sa2_engine_image` (80) |
| `prepare` | `sa2_prepare` | `sa2_preparation` (112) |
| `poll` | `sa2_poll` | `sa2_poll_row` (32) |
| `activate` | `sa2_activate` | 32-byte stamp digest |
| `cancel` | `sa2_cancel` | reason and `superseded_by` |
| `retire` | `sa2_retire` | - |
| `submit_frame` | `sa2_frame_submit` | `sa2_frame_input` (264) |
| `read_frame_reports` | `sa2_frame_reports` | `sa2_frame_report` (104) |
| `read_presented` | `sa2_presented` | `sa2_presented_row` (56) |
| `request_pick` | `sa2_pick_request` | - |
| `poll_pick` | `sa2_pick_poll` | `sa2_pick_row` (112) |
| `recover` | `sa2_recover` | JSON refusal |
| host services | `sa2_resize`, `sa2_counters`, `sa2_device_status` | `sa2_counter_values` (64) |

What each call does:

- The product draws the loaded scene's model: every stateful call needs a scene from `sa2_scene_load` or `sa2_scene_load_wj` and refuses inside a trace. Product frames use their own label, pose, constant and query resources, so W1..W4 and W-J outputs are unchanged.
- `sa2_engine_adopt` validates and copies the image on the CPU, with no GPU work or wait, also during device loss (RM.16). The next frame naming that revision uploads it into the oldest of three GPU ring copies; reusing a copy first waits for the frame that last uploaded into it.
- `sa2_prepare` uploads the per-slot visibility and instance IDs on the producer queue with its own command list and signals the product fence; `sa2_poll` reports ready at that fence. A ticket cancelled while pending stays cancelled after its upload completes, and a cancelled or failed ticket keeps answering that state (RM.17). At most 256 tickets hold GPU memory (2 MiB each) until their sets are released.
- `sa2_frame_submit` checks the stamp, draws into the frame target through the same handover as `sa2_scene_produce`, and returns the ready fence value as the frame ID. One `ClearRenderTargetView` without rectangles clears the target.
- `sa2_frame_reports` returns, in frame order, the GPU time from timestamp queries and `view_vertex_evals` from the pipeline-statistics `VSInvocations` of the frame's draw (RM.39).
- On a `sa2_attach` context, `sa2_presented` and `sa2_recover` return `SA2_E_UNAVAILABLE`: the core owns no swap chain and no device there, so it cannot confirm a presentation (RM.26) or recover (RPI.46). `sa2_recover` still writes the refusal with the removal reason. ABI 5's `sa2_create` context has their positive paths (below).
- `sa2_pick_request` draws an ID pass with the frame's camera, engine copy and set into an `R32_UINT` target (the same vertex stage, `id_ps` writes the instance ID), copies one texel to a readback buffer and signals the product fence. A newer surface or view generation, a resize, a retired set or a replaced engine copy makes the reply stale (RPI.23 to RPI.25).
- `sa2_resize` recreates the depth and ID targets after a confirmed drain, once all three slots are registered at the new size.

Frame drawing goes through one internal seam, `sa2::FrameTarget` (`src/frame_target.h`). Its implementations are the Godot-texture route (`SlotTarget`, a registered ring slot) and, since ABI 5, route B's surface (`SurfaceTarget`, the composition swap chain's current back buffer). The device-free logic (validation, tickets, stamps, reports, picks, counters, device loss) is in `src/product.cpp` and is covered by the CPU self-test. The GPU side is in `src/scene.cpp` and `shaders/product.hlsl`.

The ABI 1 `clear_code` path still batches up to 32 rectangles per `ClearRenderTargetView`, as measured for S-A2; the ABI 4 passes clear one whole target per call.

## ABI 5 (route B, accepted for now by the owner on 18 Nov 2026, pending three owner-machine checks)

The record is `docs/wiki/decisions/owner-decisions-2026-11-18-route-b.md`. The route B parts are adapted from the C9 present probe `work/experiments/c9-present-probe/` (PR 245); `PROVENANCE.md` names the files and digests. ABI 5 adds no struct. It extends `sa2_presented_row`, which no ABI 4 core wrote, with `display_qpc`, the SyncQPCTime of the statistics row that confirmed the frame. The Godot-texture route (`sa2_attach`, the ring slots, W1..W4 and W-J) is unchanged.

```c
#define SA2_TARGET_SURFACE 0xFFFFFFFFu  /* sa2_frame_input.target */
#define SA2_SURFACE_SYNC_INTERVAL 1u    /* Present sync interval */
#define SA2_SURFACE_BUFFERS 3u          /* FLIP_DISCARD, latency waitable, maximum latency 2 */
#define SA2_SURFACE_MAX_PX 16384u
SA2_API int32_t sa2_create(uint64_t adapter_luid, const sa2_config* config, sa2_context** out);
SA2_API int32_t sa2_surface_attach(sa2_context* ctx, uint64_t hwnd, int32_t x, int32_t y, uint32_t width, uint32_t height);
SA2_API int32_t sa2_surface_move(sa2_context* ctx, int32_t x, int32_t y);
SA2_API int32_t sa2_surface_swap_chain(sa2_context* ctx, uint64_t* out);
SA2_API int32_t sa2_surface_detach(sa2_context* ctx);
```

- `sa2_create` makes the core's own device with the D3D12 device factory (not stored as the process singleton, so never Godot's device) on the adapter with that LUID (0: the first hardware adapter), with its own DIRECT queue and the context setup `sa2_attach` makes. An unknown LUID is `SA2_E_INVALID_ARGUMENT`; no hardware adapter or no device factory is `SA2_E_UNSUPPORTED` (there is no `D3D12CreateDevice` fallback).
- `sa2_surface_attach` makes a DirectComposition target for the window, one visual at (x, y), and a composition swap chain (B8G8R8A8_UNORM, three FLIP_DISCARD buffers, the frame-latency waitable, maximum latency 2) on the producer queue. `sa2_surface_move` moves and commits the visual, also during device loss. `sa2_surface_detach` needs the scene unloaded and a drain. `sa2_surface_swap_chain` (C9 diagnostics, added without an ABI change, so a shell treats it as optional) writes the current swap chain's address, which the shell records so that a PresentMon capture can tell the core's presents from Godot's; a resize or recovery makes a new chain. On a `sa2_create` context the ring-slot calls answer `SA2_E_WRONG_STATE`, and the scene loads at the surface's size.
- A frame with target `SA2_TARGET_SURFACE` waits for the back buffer's previous frame, then for the latency waitable, draws through a second product pipeline for the B8G8R8A8 format, presents with sync interval 1 and records `GetLastPresentCount`.
- `sa2_presented` confirms a frame only by the probe's exact rule: a `GetFrameStatistics` row of the frame's swap-chain generation whose `PresentCount` equals the frame's count and is not above `GetLastPresentCount`. A frame whose count a row passed, or whose generation ended, is never reported; a failed statistics call returns `SA2_OK` with no row unless the device is removed. The rule is the pure function `present_match` in `src/product.cpp`, with CPU self-tests.
- `sa2_recover` on a removed device builds a new device and queue on the same adapter, a new swap chain as the same visual's content (or a rebuilt target and visual on the same window and rectangle), and the scene again from its retained config, and only then releases the old objects. The adopted CPU engine image is kept, `device_epoch` increases by one, and it writes `{status: "recovered", reason, device_epoch, device_removed: "0x%08x"}`. Old tickets, sets, frames and picks answer `SA2_E_WRONG_STATE`, unread old reports are dropped, and numbers keep counting. A live device answers `SA2_E_WRONG_STATE` with `status: "not_removed"`.
- `sa2_resize` resizes the swap chain after a drain; `sa2_describe` reports interface version 5 and lists F22 (recover) and F23 (read_presented) as supported only on a `sa2_create` context with a surface; `sa2_remove_device` works on a `sa2_create` context.

## C9 U5: user shader stages (additive in ABI 5)

The U5 design names ABI 6; these calls are added to ABI 5 instead, `SA2_ABI_VERSION` stays 5 and no earlier call changes. The header comment before the declarations gives the contract.

```c
#define SA2_STAGE_VERTEX 1u
#define SA2_STAGE_FRAGMENT 2u
#define SA2_STAGE_RECORD_BYTES 65536u
#define SA2_STAGE_HLSL_MAX_BYTES 1048576u
#define SA2_STAGE_ARRAY_MAX 1048576u
SA2_API int32_t sa2_stage_compile(sa2_context* ctx, uint32_t kind, const char* hlsl_utf8, uint32_t hlsl_size, uint64_t* out_handle, char* record, uint32_t record_size, uint32_t* out_record_size);
SA2_API int32_t sa2_scene_stages(sa2_context* ctx, uint64_t vertex_stage, uint64_t fragment_stage, uint32_t overdraw, const uint32_t* vertex_arrays, uint32_t vertex_count, const uint32_t* fragment_arrays, uint32_t fragment_count);
SA2_API int32_t sa2_stage_report(sa2_context* ctx, sa2_stage_frame* out); /* sa2_stage_frame: 64 bytes on x64 */
SA2_API int32_t sa2_stage_release(sa2_context* ctx, uint64_t handle);
```

- A stage is HLSL text with one fixed entry: `uint m600_vertex(float x, float y, float z, float w, float t, out float3 position)` (x, y, z, w: Q*world; the result is the coverage code, 0 a point, 1 singular_neighbourhood, 2 nonfinite) or `float3 m600_fragment(float class_, float shade, float x, float y, float z, float t)` (x, y, z: the projected 3D position). Its arrays are `StructuredBuffer<float> m600_vertex_arrays : register(t20)` and `m600_fragment_arrays : register(t21)`; the core declares a buffer behind a `#define` guard only when the stage text does not name it.
- `sa2_stage_compile` composes the stage text with the core's `geometry.hlsl`, `draw.hlsl` and `shaders/stages.hlsl`, embedded in the DLL at build time by `embed.cmake`, and compiles every pass with DXC at run time with exactly `-T <vs|ps|cs>_6_2 -E <entry> -HV 2021 -Gis -denorm preserve -O3 -Ges -WX`. `dxcompiler.dll` and `dxil.dll` load from the DLL's own folder by absolute path, never the search path; the build does not copy them. The compile record holds the compiler and validator versions, the arguments, and the SHA-256 of the composed source and of each pass's DXIL. A failure records `EV3:input.stage_compile_failed` with DXC's error text and keeps the built-in chain usable.
- `sa2_scene_stages` attaches a chain to the loaded W1..W4 scene outside a trace. Every later `sa2_scene_produce` frame, traced or not, on a ring slot or route B's surface, runs RPI.72: eval (compute, the chain up to Q*world with a traced frame's W3 turn, the vertex stage, camera and clip), bound (compute, `rust/c600-display/src/raster.rs`'s raster bound with an exact 64-bit sum), decide (stopped when U exceeds width * height * overdraw), then the user draw under `D3D12_PREDICATION_OP_EQUAL_ZERO` and the built-in draw under `NOT_EQUAL_ZERO`. Product frames, W-J and frames without a chain draw exactly as before. C9 U5c changed U5b's rule, under which traced frames drew without the chain: the renderer gate measures traced frames, so they must run the stages.
- `sa2_stage_report` returns the oldest completed chain frame not yet returned (U, the allowance, the stop decision and the pixel-shader invocations of both draws), so a caller that reports after each frame and again while the frame number rises sees every chain frame once, in order; with none waiting it returns the last row again. At most 4096 rows wait (C9 U5c; U5b returned only the newest).
- A stage compile that cannot load DXC, compose a pass or build a pipeline returns its status with the compile record; no exception crosses the ABI (C9 U5c).

## Build and CPU check

These need Windows, MSVC x64, the Windows SDK's `dxc` and the pinned `renderer-spike` CMake and Ninja. A worktree without `tools/.venv/renderer-spike` can set `M600_RENDERER_SPIKE_SCRIPTS` to another checkout's `Scripts` folder.

```bat
renderer\core\build.cmd [build-directory]
```

The default build directory is `work\build\renderer-core`. Build only under the repository's ignored `work/` folder: Application Control refuses fresh unsigned binaries in `%TEMP%`.

```sh
python renderer/core/check_core.py
```

This builds Release in a fresh `work/build/renderer-core-<pid>` folder. It checks that no generated build file names `work/experiments` and runs `sa2_selftest.exe --cpu`, whose 35 checks must all pass (17 ABI 2 checks, 5 W-J checks, 8 product checks, 3 route B checks: route b arguments, present matching and recovery bookkeeping, and 2 C9 U5 checks: stage arguments and raster bound, the latter against vectors that include raster.rs's unit-test cases). It then recomputes the DLL and the eight shader SHA-256 values in the identity and removes the folder. It starts no D3D12 device.

`tests/test_renderer_core.py` checks the provenance digests, the file list, the CMake sources and the ABI pins. It needs only the Python standard library and runs on Linux.

```sh
python renderer/core/check_core.py --stages <folder with dxcompiler.dll and dxil.dll>
```

This also copies the two DXC libraries beside the built DLL and runs `sa2_selftest.exe --warp --stages --debug`, C9 U5's user-stage check, on WARP only (the self-test refuses `--stages` with any other mode). At 640 x 360 it compiles hand-written fixture stages through DXC at run time, checks their compile records and a broken stage's refusal, draws W3 frames with the user chain, compares each frame's device U with the C++ bound over the dumped eval buffer, forces a stop with overdraw 0 and checks with pipeline statistics that the user draw ran no pixel shader and the built-in draw did, and checks the releases, the trace rule (traced frames draw with the chain) and that the report returns every chain frame once in order. It does not cover route B's surface, which needs a window.

`sa2_selftest.exe --warp --measure-stages <folder> --size WxH --frames N --traced M --out <file>.jsonl` (C9 U5c, WARP only) measures U for a generated stage pair (`c600-stage-hlsl` output with an `arrays.json` beside it): it loads W3 at that size, attaches the pair with an unlimited allowance, produces N untraced frames (the camera turns 0.002 rad per frame, one period is 3142 frames) and M traced frames with W3's turns, and writes one JSON line per frame with U. It is a measurement, never a check; the acceptance harness's README gives the overdraw it sized.

## GPU checks (owner runs)

None of these has been run for this copy. Each starts a D3D12 device:

- `python renderer/core/check_core.py --gpu` runs the self-test's WARP and hardware modes with the debug layer. These are ABI 2's ring, produce, verify, drain and device-loss checks, and its W1..W4 scene device, geometry and injected-W3 checks, now drawn through the pose-path shaders. They do not cover a W-J scene.
- The W3 re-acceptance of W1..W4 on the S-A2 Godot host, with the ABI 2 bars unchanged. Their output through the pose-path shaders has not been compared with ABI 2's on a GPU.
- W-J item 5 on the S-A2 Godot host:
  - S4, I_a and I_b runs (`sa2_scene_load_wj`, produce, `sa2_scene_write_run`) with the label and pose checks;
  - `sa2_scene_geometry_check` against `work/experiments/renderer-wj/ref_<menu>_*`;
  - `sa2_scene_lattice_check` on the `lattice-start` and `lattice-retained` controls;
  - the five injections, each of which must fail its check.

  The data directory comes from W-J's owner-run `work/experiments/renderer-wj/prepare_wj.py --out <dir>`.
- The ABI 4 GPU paths: the engine and preparation uploads, a product frame with its timestamp and pipeline-statistics queries, the ID pass and its readback, `sa2_resize`, and device loss during product work. They compile, but no self-test mode exercises them yet.
- The ABI 5 route B paths: `sa2_create`, the surface calls in a real window, surface frames and their presentation, `sa2_presented` against real frame statistics, `sa2_resize` of the swap chain, and `sa2_recover` after `sa2_remove_device`. They compile, but no self-test mode exercises them; route B's three owner-machine checks are still open.

The S-A2 host project under `work/experiments/renderer-sa2/project/` pins ABI 2 in `Native.cs`. A host that loads this DLL must expect ABI 5 and bind the new functions; the acceptance harness `renderer/acceptance/` pins ABI 5 and uses only ABI 1 to 3 calls.
