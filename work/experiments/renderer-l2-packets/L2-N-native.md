# Packet L2-N: ABI 2 of the producer DLL, drawing the S-B scenes (E-2.4-02 and E-2.4-03, level 2)

Run from the `claude/renderer-l2` checkout, with no H-06 run, gate capture or other implement call active:
`python tools/agents/codex_review.py --kind implement --model gpt-6.1-sol --effort max --timeout 7200 --packet work/experiments/renderer-l2-packets/L2-N-native.md`

Plan: `work/experiments/renderer-l2-packets/PLAN.md` (sections 2 to 4). This packet owns `work/experiments/renderer-sa2/native/`, including the header. The Godot harness (L2-G) and the Qt harness (L2-Q) come after it and build against its header and DLL. Section 6 is the ABI 2 contract, and it is binding. Where it leaves a choice, make the choice and name it in the final message.

## 1. Goal and acceptance
- Goal: ABI version 2 of `sa2_interop.dll`. A framework host (Godot or Qt) gets the S-B drawing method on the framework's own D3D12 device. The DLL draws W1 to W4 into the textures the framework displays, using level 1's slots, queue modes and fence protocol, and it produces S-B's checks and run-record parts:
  - the geometry check;
  - the exact label check;
  - `trace.jsonl`;
  - peak VRAM;
  - target and viewport sizes;
  - the DLL's identity.
- Acceptance check: `python work/experiments/renderer-sa2/native/check_native.py` exits 0. It builds out of tree, including the shaders, and runs `sa2_selftest.exe --cpu`.
- Done when:
  - the header is ABI 2 as section 6 specifies;
  - every ABI 1 function keeps its documented behaviour;
  - every ABI 2 function is implemented;
  - the self-test has the CPU checks and the GPU checks of section 6;
  - `README.md` gives the owner's commands and the host call order.
- Non-goals:
  - the Godot and Qt harnesses;
  - PresentMon;
  - windows, swap chains and presents;
  - the H-06 features and the `w3f` scene;
  - MSAA above 1;
  - NVIDIA-specific features;
  - the Agility SDK.

## 2. Actual problem and reproduction
- S-B met the W3 gate as a bare D3D12 probe (`work/experiments/renderer-sb/RESULT.md`). Level 1 showed that our own D3D12 code can write a texture on Godot's and Qt's own devices, which the framework then shows (`work/experiments/renderer-sa2/RESULT.md`, `work/experiments/renderer-sd/RESULT.md`), with the ABI 1 DLL in this folder.
- Level 2 must run S-B's W1 to W4 inside each framework with S-B's drawing method and S-B's geometry and label checks. One DLL serves both frameworks, because S-D loads it from this folder.
- The day-7 gate trusts the run record, so the DLL must report what each harness checks (plan section 3):
  - the real target and viewport sizes;
  - a complete identity (the DLL and every shader blob it loads);
  - the framework process's peak VRAM.

## 3. Environment and versions
- Base: branch `claude/renderer-l2` (this worktree's HEAD).
- Owner's machine:
  - Windows 11 (build 26200);
  - NVIDIA GeForce RTX 4070 Laptop GPU (driver 616.92) and an integrated AMD Radeon 610M;
  - Visual Studio Community 2026 (MSVC 14.51) and Windows SDK 10.0.26100.0;
  - CMake 4.4.3 and Ninja 1.13.2 in `tools/.venv/renderer-spike/Scripts/` (not present in this worktree).
- C++20, Windows SDK only, static CRT (`/MT`). Shaders are compiled with DXC the way `work/experiments/renderer-sb/probe/CMakeLists.txt` does it.
- Evidence kind in the sandbox: source/fixture only. Every GPU check runs on the owner's machine afterwards.

## 4. Necessary source and evidence
- ABI 1, as it stands: `include/sa2_interop.h`, `src/interop.cpp`, `src/selftest.cpp`, `src/gpu_test.cpp`, `src/pure.*`, `build.cmd`, `CMakeLists.txt` and `check_native.py`, all in this folder. The level 1 packet `work/experiments/renderer-sa2-packets/SA2-N-native.md` gives the build, error, naming, barrier, drain and callback rules. They all stay in force.
- S-B, read-only:
  - `work/experiments/renderer-sb/SPEC.md`: sections 2 to 7 (assets, geometry pipeline, turn animation and trace, run directory, label check, geometry outputs);
  - `work/experiments/renderer-sb/probe/src/` (`probe.h`, `probe.cpp`, `edges.cpp`, `json.h`, `gpu.cpp`);
  - `work/experiments/renderer-sb/probe/shaders/`;
  - `work/experiments/renderer-sb/probe/CMakeLists.txt` and `build.cmd`.
- The gate's run contract: `tools/perf/renderer_gate.py` (read-only).

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| — | none for level 2 | — | — | — |

## 6. Constraints and owned files

Owned files, all under `work/experiments/renderer-sa2/native/`:
- `include/sa2_interop.h`
- `CMakeLists.txt`, `build.cmd`, `check_native.py`, `README.md`
- `src/*`
- `shaders/*`: use this only if an S-B shader cannot be used unchanged, and say why.

Change nothing else. In particular:
- `../code_layout.json` and every file under `work/experiments/renderer-sb/` stay byte-identical;
- nothing under the repository's root `native/`, `assets/` or `tools/` changes.

### Reuse of S-B
- **Readers and checks.** Compile S-B's `probe/src/probe.cpp` and `probe/src/edges.cpp` unchanged into the DLL, with the definitions S-B's build gives them (for example `M600_REPO_ROOT`). They provide:
  - `sb::Assets`, `frameState`, `turnAt`, `turnTicks` and `rotate`;
  - `checkLabels`, `injectLabels` and `edgeMasks`;
  - `sha256` and `Json`.
- **Unused symbols.** If those files reference symbols the DLL does not use (for example `sb::runGpu`), define them in the DLL's own sources so that they fail cleanly when called.
- **`exeDir()`.** Never use `sb::exeDir()` or `sb::buildIdentity()` in the DLL. Inside a framework they point at the framework's executable.
- **Drawing path.** Port the drawing path of `gpu.cpp` into the DLL's own sources, with these parts the same as S-B:
  - the passes and the resources;
  - the root signatures and the bindings;
  - the camera and the turn state per frame;
  - the label upload, use and copy protocol.

  Drop the window, the swap chain, the present, the condition sampling and the features. The drawing method must stay S-B's: the S-B `none` feature at MSAA 1.
- **Shaders.** Compile S-B's HLSL files from `probe/shaders/` unchanged, with S-B's entry points, profiles and flags, into the build directory next to the DLL. The DLL loads the blobs from its own directory, found with `GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS, …)`. It never compiles shaders at run time.

### ABI 2 contract (binding)

**Header.**
- `SA2_ABI_VERSION` becomes `2u`, and `sa2_abi_version()` returns 2.
- Every ABI 1 declaration, constant, struct layout and documented behaviour stays as it is. ABI 2 only adds.
- Comment each new function in the header with its semantics, preconditions and statuses, as ABI 1 does. The header comments are the specification L2-G and L2-Q work from.

**New statuses.**
- `SA2_E_CHECK_FAILED 8`: a geometry or label check ran and failed. Its output is still written.
- `SA2_E_IO 9`: a file could not be read or written, or an output file already exists.

**`sa2_scene_config`**, `sizeof == 32` on x64, with a `static_assert`:
- `uint32_t struct_size`;
- `uint32_t scene`: 1 to 4 for W1 to W4;
- `double turn_ms`: W3 only, in (0, the gate's `TURN_MS_MAX`]; S-B's default is 190;
- `const char* inject`: NULL, or one of the fault names `sb::injectLabels` accepts; W3 and W4 only;
- `uint32_t flags`: bit 0 is `SA2_SCENE_NO_VRAM`, a debug switch that turns VRAM sampling off;
- `uint32_t reserved`: must be 0.

**Functions.** Each is called from the context's thread. Each validates `struct_size` and its arguments first (`SA2_E_INVALID_ARGUMENT`), and sets the last error on failure as ABI 1 does.
1. **`int32_t sa2_scene_load(sa2_context* ctx, const sa2_scene_config* config)`**
   - Loads the S-B workload with S-B's readers and the shader blobs, then creates every GPU resource of the scene on the attached device.
   - One loaded scene per context. A second load returns `SA2_E_WRONG_STATE`.
   - A missing asset or shader blob returns `SA2_E_IO`, with the file named in the last error.
2. **`int32_t sa2_scene_produce(sa2_context* ctx, uint32_t slot, uint64_t frame)`**
   - The same preconditions, fail-closed checks, queue modes, fences and handover states as `sa2_produce`. Instead of the code image, it draws scene frame `frame` into the slot, with the viewport covering the slot's full size.
   - The camera, turn state and label protocol per frame follow S-B. W3's turn timing uses the trace start the way S-B uses its own trace start.
   - Between trace begin and trace end, each call:
     - appends one trace record in S-B's format (`SPEC.md` section 5), with `qpc` read at the call;
     - samples `QueryVideoMemoryInfo` (node 0, local segment, `CurrentUsage`) for the device's adapter, unless `SA2_SCENE_NO_VRAM` is set.
3. **`int32_t sa2_scene_trace_begin(sa2_context* ctx)`** and **`int32_t sa2_scene_trace_end(sa2_context* ctx)`**
   - Record `trace_start_qpc` and `trace_stop_qpc` at the call.
   - Once each per load, in that order; otherwise `SA2_E_WRONG_STATE`.
4. **`int32_t sa2_scene_write_run(sa2_context* ctx, const char* directory_utf8)`**
   - Only after trace end and a confirmed `sa2_drain` that covers every scene command list; otherwise `SA2_E_WRONG_STATE`.
   - Runs S-B's label check for W3 and W4.
   - Writes two files into the existing directory, and never overwrites one (`SA2_E_IO`):
     - `trace.jsonl`, exactly in S-B's format;
     - `native.json`, described below.
   - Returns `SA2_OK`, or `SA2_E_CHECK_FAILED` when the label check fails.
5. **`int32_t sa2_scene_geometry_check(sa2_context* ctx, const char* directory_utf8)`**
   - S-B's geometry check (`SPEC.md` section 7) on the attached device and queue, with the DLL's own offscreen targets, never a slot.
   - Allowed with a loaded scene, outside the trace window. It waits for its own work with a bounded CPU fence wait (`SA2_E_TIMEOUT`).
   - Writes `geometry.json` in S-B's format. Returns `SA2_OK` on a pass, `SA2_E_CHECK_FAILED` on a mismatch.
6. **`int32_t sa2_identity(char* buffer, uint32_t buffer_size)`**
   - No context. Uses the buffer and truncation convention of `sa2_debug_messages`.
   - Writes one JSON object: `{"dll": {"file", "sha256"}, "shaders": [{"file", "sha256"}, …]}`.
     - `dll` is the DLL file this module was loaded from.
     - `shaders` covers every shader blob the DLL loads, sorted by file name.
     - Each `sha256` is over the exact bytes on disk.
7. **`int32_t sa2_scene_unload(sa2_context* ctx)`**
   - Releases the scene's resources.
   - Needs a confirmed drain that covers the scene's work, except after device removal, like `sa2_unregister_slot`.
   - `sa2_detach` with a loaded scene returns `SA2_E_WRONG_STATE`.

**`native.json`** (format `magic600-sa2-scene-native-v1`):
- `format` and `scene` (`"w1"` … `"w4"`);
- `qpc_frequency`;
- `markers` `{trace_start_qpc, trace_stop_qpc}`;
- `frames`: the number of trace records;
- `turn_ms`: W3 only;
- `label_check`: S-B's object, W3 and W4 only;
- `injection_applied`;
- `vram_peak_mb` and `vram_samples`: `vram_peak_mb` is a number in MiB, or `null` when `SA2_SCENE_NO_VRAM` was set;
- `target` `{width, height}`: the slot texture;
- `viewport` `{width, height}`: what the draws used;
- `identity`: the `sa2_identity` object;
- `queue_mode` and `barrier_api`.

The harnesses merge this into `run.json`. The DLL never writes `run.json`.

### Self-test (`sa2_selftest.exe`)
Keep every ABI 1 check, with the expected ABI version raised to 2 and the export list extended.

**CPU checks** (`--cpu`, under 60 s):
- every new export is found by name;
- `static_assert` checks on the struct sizes;
- each new function's argument and state checks return the documented status:
  - a null context;
  - a wrong `struct_size`;
  - scene 0 or 5;
  - W3 with `turn_ms` of 0 or less;
  - an unknown `inject` name;
  - a nonzero `reserved`;
  - `write_run` before trace end;
  - an existing output file;
- `sb::Assets` loads through the DLL build and has S-B's counts;
- the `native.json` writer, tested as a pure function on synthetic data:
  - every field is present;
  - `vram_peak_mb` is `null` with `SA2_SCENE_NO_VRAM`;
- `sa2_identity` names the DLL and every shader blob. `check_native.py` recomputes each SHA-256 in Python and compares.

**GPU checks** (`--warp`, `--hardware`, optional `--debug`; the stand-in host and device rules of level 1):
- For each scene W1 to W4, both queue modes (`same`, `own`), and legacy barriers (enhanced too when OPTIONS12 supports them):
  - load;
  - produce on a 3-slot ring, in level 1's host order;
  - trace begin and end inside the run;
  - drain;
  - write the run into a fresh temp directory;
  - unload.
- Sizes and frames:
  - WARP: 640 × 360, 40 frames;
  - hardware: 2560 × 1600, 600 frames.
- Every run must have:
  - one trace record per traced frame, with consecutive frame numbers;
  - `target == viewport ==` the slot size;
  - `vram_peak_mb > 0`;
  - a passing label check for W3 and W4.
- The geometry check passes once per device.
- A W3 run with an injected fault returns `SA2_E_CHECK_FAILED` with label status `fail`.
- A run with `SA2_SCENE_NO_VRAM` writes `vram_peak_mb: null`.
- With `--debug`: error and corruption counts are 0.
- Reference counts return to their values before attach.

### `check_native.py`
Keep the level 1 rules:
- a plain `mkdir` temp directory;
- `M600_SA2_CPU_CHECK=1`;
- timeouts that kill only the build's own process tree;
- a CRLF-only `build.cmd`;
- nothing left inside the repository.

Add:
- the shader blobs in the build;
- the identity recomputation;
- `--gpu` (not part of the acceptance): runs `--warp --debug`, then `--hardware --debug`.

### `README.md`
- the owner's build and run commands;
- what each GPU check proves;
- the host's call order per frame and per run:
  - per frame: `signal_godot_free(f - 1)`, `sa2_scene_produce(f % 3, f)`, `godot_wait_ready(f)`, the framework draws, `mark_shown`;
  - per run: trace begin after the preroll, trace end after the duration, drain, `write_run`, unload.

```implement-contract
{"allowed_files": ["work/experiments/renderer-sa2/native/include/sa2_interop.h", "work/experiments/renderer-sa2/native/CMakeLists.txt", "work/experiments/renderer-sa2/native/build.cmd", "work/experiments/renderer-sa2/native/check_native.py", "work/experiments/renderer-sa2/native/README.md", "work/experiments/renderer-sa2/native/src/*", "work/experiments/renderer-sa2/native/shaders/*"], "acceptance_check": ["python", "work/experiments/renderer-sa2/native/check_native.py"], "stop_condition": "sa2_interop.dll (ABI 2) and sa2_selftest.exe build with build.cmd including the S-B shader blobs, check_native.py passes (sa2_selftest --cpu and the identity recomputation), every ABI 1 behaviour is kept, every ABI 2 function in section 6 is implemented and documented in the header, the --warp/--hardware scene checks are implemented, and README.md gives the owner's commands and the host call order"}
```

## 7. Required return format
- Implementation: changes only in the assigned worktree. The final message lists:
  - the changed files;
  - the acceptance result;
  - what was not verified in the sandbox (every GPU check);
  - each choice section 6 left open, and the choice made;
  - every S-B part that could not be reused unchanged, and why;
  - open points.
