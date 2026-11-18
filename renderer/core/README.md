# Renderer core (S-A2, ABI 3)

`renderer/core/` is the product copy of the selected S-A2 renderer core. It is a native Direct3D 12 producer DLL, `sa2_interop.dll`, that a Godot 4.7.2 .NET host loads. It draws into textures that Godot shares with it, on Godot's device and queue. W-J's per-piece pose path is added as ABI 3, so W-J item 5 (S4, I_a, I_b) and the W3 re-acceptance can run on the S-A2 host with unchanged bars. `PROVENANCE.md` lists every file's source under `work/experiments/` and how it changed. The build compiles nothing from there.

## ABI 3

`include/sa2_interop.h` is the contract. `SA2_ABI_VERSION` is 3. The ABI 1 and ABI 2 declarations, layouts and behaviour are kept, and `sizeof(sa2_scene_config)` is still 32. ABI 3 adds:

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

W1..W4 load with `sa2_scene_load` as before. They keep S-B's readers, label protocol, checks and output files. They draw through W-J's pose-path shaders with identity poses and the pose path disabled. The identity now lists `lattice_cs.dxil` as well.

## Build and CPU check

These need Windows, MSVC x64, the Windows SDK's `dxc` and the pinned `renderer-spike` CMake and Ninja. A worktree without `tools/.venv/renderer-spike` can set `M600_RENDERER_SPIKE_SCRIPTS` to another checkout's `Scripts` folder.

```bat
renderer\core\build.cmd [build-directory]
```

The default build directory is `work\build\renderer-core`. Build only under the repository's ignored `work/` folder: Application Control refuses fresh unsigned binaries in `%TEMP%`.

```sh
python renderer/core/check_core.py
```

This builds Release in a fresh `work/build/renderer-core-<pid>` folder. It checks that no generated build file names `work/experiments` and runs `sa2_selftest.exe --cpu`, whose 22 checks must all pass (17 ABI 2 checks and 5 W-J checks). It then recomputes the DLL and shader SHA-256 values in the identity and removes the folder. It starts no D3D12 device.

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

The S-A2 host project under `work/experiments/renderer-sa2/project/` pins ABI 2 in `Native.cs`. A host that loads this DLL must expect ABI 3 and bind the two new functions.
