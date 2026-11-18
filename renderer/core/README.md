# Renderer core (S-A2, ABI 4)

`renderer/core/` is the product copy of the selected S-A2 renderer core. It is a native Direct3D 12 producer DLL, `sa2_interop.dll`, that a Godot 4.7.2 .NET host loads. It draws into textures that Godot shares with it, on Godot's device and queue. W-J's per-piece pose path is added as ABI 3, so W-J item 5 (S4, I_a, I_b) and the W3 re-acceptance can run on the S-A2 host with unchanged bars. ABI 4 adds the product API that the Rust display host calls, one C call per RM.14 operation. `PROVENANCE.md` lists every file's source under `work/experiments/` and how it changed. The build compiles nothing from there.

## ABI 3

`include/sa2_interop.h` is the contract. `SA2_ABI_VERSION` is 4. The ABI 1 to ABI 3 declarations, layouts and behaviour are kept, and `sizeof(sa2_scene_config)` is still 32. ABI 3 added:

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
| `read_presented` | `sa2_presented` | `sa2_presented_row` (48) |
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
- `sa2_presented` and `sa2_recover` return `SA2_E_UNAVAILABLE`: the core owns no swap chain and no device, so it cannot confirm a presentation (RM.26) or recover (RPI.46). `sa2_recover` still writes the refusal with the removal reason.
- `sa2_pick_request` draws an ID pass with the frame's camera, engine copy and set into an `R32_UINT` target (the same vertex stage, `id_ps` writes the instance ID), copies one texel to a readback buffer and signals the product fence. A newer surface or view generation, a resize, a retired set or a replaced engine copy makes the reply stale (RPI.23 to RPI.25).
- `sa2_resize` recreates the depth and ID targets after a confirmed drain, once all three slots are registered at the new size.

Frame drawing goes through one internal seam, `sa2::FrameTarget` (`src/frame_target.h`). Its only implementation is the Godot-texture route (a registered ring slot); a swap-chain or DirectComposition target would be a second implementation. The device-free logic (validation, tickets, stamps, reports, picks, counters, device loss) is in `src/product.cpp` and is covered by the CPU self-test. The GPU side is in `src/scene.cpp` and `shaders/product.hlsl`.

The ABI 1 `clear_code` path still batches up to 32 rectangles per `ClearRenderTargetView`, as measured for S-A2; the ABI 4 passes clear one whole target per call.

## Build and CPU check

These need Windows, MSVC x64, the Windows SDK's `dxc` and the pinned `renderer-spike` CMake and Ninja. A worktree without `tools/.venv/renderer-spike` can set `M600_RENDERER_SPIKE_SCRIPTS` to another checkout's `Scripts` folder.

```bat
renderer\core\build.cmd [build-directory]
```

The default build directory is `work\build\renderer-core`. Build only under the repository's ignored `work/` folder: Application Control refuses fresh unsigned binaries in `%TEMP%`.

```sh
python renderer/core/check_core.py
```

This builds Release in a fresh `work/build/renderer-core-<pid>` folder. It checks that no generated build file names `work/experiments` and runs `sa2_selftest.exe --cpu`, whose 30 checks must all pass (17 ABI 2 checks, 5 W-J checks and 8 product checks). It then recomputes the DLL and the eight shader SHA-256 values in the identity and removes the folder. It starts no D3D12 device.

`tests/test_renderer_core.py` checks the provenance digests, the file list, the CMake sources and the ABI pins. It needs only the Python standard library and runs on Linux.

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

The S-A2 host project under `work/experiments/renderer-sa2/project/` pins ABI 2 in `Native.cs`. A host that loads this DLL must expect ABI 4 and bind the new functions.
