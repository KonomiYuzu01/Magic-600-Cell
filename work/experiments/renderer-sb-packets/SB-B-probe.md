# Packet SB-B: S-B bare Direct3D 12 probe (part of E-2.4-01)

## 1. Goal and acceptance
- Goal: a bare Direct3D 12 probe that draws all 600 x 433 = 259,800 stickers at full detail and runs the scenes W1 to W4, with the W3 trace, the exact label check and the run directory that `tools/perf/renderer_gate.py` judges. The gate comes first: W3 (all stickers, the turn animation, camera rotation, a fresh label upload after every turn) must be correct and fast before anything else.
- Acceptance check: `python work/experiments/renderer-sb/probe/check_probe.py` exits 0. It builds the probe out of tree (section 6) and runs `sb_probe.exe --selftest`, which needs no GPU.
- Measurements, GPU runs, PresentMon captures and the gate verdict come later from the owner's machine; the sandbox cannot produce them. Do not claim any frame rate.
- Non-goals: UI, input beyond the scripted camera, Look Lab presets, NVIDIA features (Reflex, DLSS, SER, NVAPI: none of them in the build), the H-06 feature table (a later packet; leave a `--feature` hook), shipping code quality.

## 2. Actual problem and reproduction
- No Direct3D 12 drawing of this model exists; 0.4 draws with Managed DirectX 9. 1.0 needs a D3D12 base (owner decision, 29 September 2026), and the day-7 go/no-go needs a number for W3 on the owner's RTX 4070 Laptop GPU.
- Expected: a probe whose W3 run on the owner's machine is judged by `renderer_gate.py`.

## 3. Environment and versions
- Base: branch `claude/renderer-sb` (this worktree's HEAD).
- Owner's machine (for the README, not for the sandbox): Windows 11, NVIDIA GeForce RTX 4070 Laptop GPU 8 GB (driver 616.92), hybrid graphics: the 2560 x 1600 60 Hz panel is driven by the integrated AMD Radeon 610M. Visual Studio Community 2026 18.10 (MSVC 14.51, cl 19.51), Windows SDK 10.0.26100.0 with `dxc.exe` and the D3D12 debug layer, CMake 4.4.3 and Ninja 1.13.2 in `tools/.venv/renderer-spike/Scripts/` (pinned; not present in this worktree), PresentMon 2.6.0.0 at `%ProgramFiles%\Intel\PresentMon\PresentMonConsoleApplication\PresentMon-2.6.0-x64.exe`, Python 3.14.
- Language: C++20 with the Windows SDK only (no third-party libraries, no Agility SDK, no vcpkg). Shaders in HLSL, shader model 6.0 or later, compiled at build time with the SDK's `dxc.exe`.
- Evidence kind in the sandbox: source/fixture only.

## 4. Necessary source and evidence
- `work/experiments/renderer-sb/SPEC.md` is the contract. Read all of it. Sections 2 (assets), 3 (pipeline and colour), 4 (turn, trace timing, label revisions, camera rotation), 5 (run directory), 6 (label check and fault injection), 7 (geometry-check outputs).
- `work/experiments/renderer-sb/workload/turn.json`, `work/experiments/renderer-sb/cameras.json`.
- `docs/progress/1.0/renderer-experiment-plan.md` sections 1 (W1-W4 and the trace contract) and 2 (measurement, the label check and its negative tests).
- `tools/perf/renderer_gate.py`: `validate_run`, `condition_reasons`, `read_frames`, `trace_reasons`. Your `run.json` and `trace.jsonl` must pass `validate_run` and must not be refused by `trace_reasons` when the run is correct.
- `web/renderer.js:21-48`: the vertex and fragment shader the pipeline ports.
- `work/experiments/renderer-readiness/minimal_d3d12.cpp`, `build.cmd`: a working borderless-window, flip-model D3D12 program and build script on this machine; reuse their structure.
- Reference outputs for the geometry check (`work/experiments/renderer-sb/reference/`) are written by a parallel packet and may be absent in your worktree; the geometry-check mode must report "reference missing" then, not fail the self-test.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | The machine builds and presents D3D12 | readiness program | 3 s run | 180 frames at 2560 x 1600 on the RTX 4070 (V-Sync on) |

## 6. Constraints and owned files

Owned: `work/experiments/renderer-sb/probe/**` only. Never change `assets/`, `SPEC.md`, `cameras.json`, `workload/`, `native/`, `core.py` or any other path.

Drawing method (start here; it is the expected pass):
- One instanced, non-indexed draw: `DrawInstanced(30480, 600, 0, 0)`; the vertex shader fetches the base vertex, its sticker id and centre, and the cell frame from structured buffers (SRVs), the label and the animated flag of the slot from buffers indexed by `cell*433 + local`, and the constants (Q, parameters, turn plane and `theta`, aspect) from a root constant buffer.
- Static data (vertices, sticker ids, centres, frames, the animated-slot flags) is uploaded once to default-heap buffers. Labels: several default-heap label buffers in a ring, each with a probe-assigned resource id, filled from an upload ring with `CopyBufferRegion` in the frame that adopts the revision (SPEC section 4).
- Depth buffer D32_FLOAT, render target R8G8B8A8_UNORM, no MSAA by default (`--msaa 4` option for later H-06 work). Flip-model swap chain, 3 back buffers, borderless window at the current display mode of the primary monitor (native resolution), backbuffer equal to the display size.
- Present: sync interval 0 with `DXGI_PRESENT_ALLOW_TEARING` when `DXGI_FEATURE_PRESENT_ALLOW_TEARING` is supported, else sync interval 0 without it; `--vsync` switches to sync interval 1. Record `vsync` and `tearing` in `run.json`.
- Frame pacing: at most 2 frames in flight, fence per frame; never wait on the GPU inside the measured path more than the in-flight limit requires.
- Adapter: `IDXGIFactory6::EnumAdapterByGpuPreference(HIGH_PERFORMANCE)`; abort with a clear message unless it is a hardware adapter (or `--warp` is given for debugging).
- If the method stays below the gate on the owner's machine, the integrator decides the next method; do not build a second method now.

Command line of `sb_probe.exe`:
- `--scene w1|w2|w3|w4`, `--duration <s>` (trace length after the trace start; default 192), `--preroll <s>` (render before the trace starts, default 4, no trace entries), `--out <run dir>` (created if absent), `--run-id <id>`, `--turn-ms <ms>` (default 190), `--vsync`, `--msaa 1|4`, `--inject corrupt-label|swap-same-colour|delay-adoption|stale-binding` (applied once at turn 20), `--declare key=value` (repeatable; copied into `environment.declared`, values `true`/`false` become booleans), `--geometry-check` (section below), `--selftest`, `--warp`, `--debug-layer`.
- W1: fixed camera (identity Q), solved labels, no trace turns. W2: W1 plus `rotate(0, 3, 0.002)` per frame. W3: SPEC section 4 in full, with W2 rotation. W4: the W3 label uploads, adoptions and readback copies every `turn-ms`, without animation and without camera rotation.
- On exit: wait for the GPU, run the label check (W3, W4), and write `run.json` and `trace.jsonl` (SPEC section 5). `presentmon.swap_chain` is written as `"FILL-FROM-CSV"`. `power_source` from `GetSystemPowerStatus`; `display` from `EnumDisplaySettingsW(ENUM_CURRENT_SETTINGS)` of the window's monitor; `refresh_hz`; `adapter` and `driver` (UMD version from `CheckInterfaceSupport(__uuidof(IDXGIDevice))`); `vram_peak_mb` sampled once per frame from `QueryVideoMemoryInfo(0, LOCAL)`.
- Print one line at the end with the probe's own timing from the trace (frames, mean fps and nearest-rank p99 of the QPC steps between frames in [T0 + 10 s, T0 + 190 s) or the whole trace if shorter), labelled `probe self-timing (not gate evidence)`.

Trace and labels: exactly SPEC sections 4 to 6. The readback copies of a run are kept in one readback-heap buffer sized for the run (`ceil(duration*1000/turn_ms) + 2` slots of 259,800 u32; about 1.1 GB for 192 s at 190 ms; fail at startup with a clear message if it cannot be allocated) and compared only after the trace ends. Trace entries are kept in memory and written at the end.

Geometry-check mode (`--geometry-check --out <dir>`): no window timing. For each camera of `cameras.json` and each pose of SPEC section 7, run a compute shader that shares the vertex shader's HLSL include for steps 1 to 7 on the sampled vertices (`reference/sample.u32`, or the sample rule of SPEC section 7 when the file is absent), read back `ndc_x`, `ndc_y`, `w`, and compare with `reference/<camera>_<pose>.f32` within the SPEC tolerance. Also count vertex-shader invocations per instance with a UAV counter in one W1 draw and require 30,480 for each of the 600 cells. Write `geometry_check.json` (per camera and pose: samples, max abs error, failures; the per-cell count result; overall `pass`/`fail`/`reference-missing`) and exit 0 only on `pass`.

Self-test (`--selftest`, CPU only, no device, under 60 s): load and digest-check the assets against `assets/manifest.json` (SHA-256 with `BCryptHash`); parse `turn.json` and `cameras.json`; rebuild both oracle label arrays and match the two SHA-256 digests in `turn.json`; run the label-check logic on synthetic copy records for a clean run and for each of the four injected faults (each must fail, the clean run must pass); check the turn, phase, theta and revision arithmetic of SPEC section 4 at turn boundaries; write a `run.json` and `trace.jsonl` for a synthetic 200 s W3 trace into memory and validate their shape; print `selftest: ok` and exit 0.

Files to write under `work/experiments/renderer-sb/probe/`:
- `CMakeLists.txt`, `build.cmd` (`build.cmd [<build dir>]`, default `build\`; finds Visual Studio with `vswhere` (`call "%VSWHERE%" ...` inside `for /f`, as `work/experiments/renderer-readiness/build.cmd`), runs `vcvars64.bat`, uses `tools\.venv\renderer-spike\Scripts\cmake.exe` and `ninja.exe` of the repository when present, else the CMake and Ninja on `PATH` after `vcvars64` (the Visual Studio copies), and compiles the HLSL with the SDK's `dxc.exe`; prints the CMake, Ninja, MSVC and DXC versions it used).
- `src/*.cpp`, `src/*.h`, `shaders/*.hlsl`.
- `check_probe.py`: builds into `Path(tempfile.gettempdir()) / "m600-sb-probe-<pid>"` created with plain `mkdir` (never `mkdtemp` or `TemporaryDirectory`: the sandbox's temp ACL breaks them), runs `sb_probe.exe --selftest`, prints both results, exits non-zero on any failure, and leaves nothing inside the repository.
- `run_scene.ps1`: for the owner's administrator PowerShell. `-Scene w1|w2|w3|w4 -Runs <n> [-Inject <fault>] [-Declare <k=v>...]`. Per run: a new run directory under `work/loop-memory/perf/renderer/sb/<timestamp>-<scene>-<n>/`, start `sb_probe.exe`, attach PresentMon 2.6.0.0 to its PID with `--v1_metrics --qpc_time --process_id <pid> --output_file <run>\presentmon.csv --timed <preroll + duration + 2> --terminate_after_timed`, wait, fill `presentmon.swap_chain` with the SwapChainAddress that has the most rows for the PID, pause 20 s between runs (cold runs: a new process each time), and finally run `python tools/perf/renderer_gate.py <runs> --out <dir>/summary.json` and print the scene verdict, fps, p99, `vram_peak_mb` and the invalid reasons. Refuse to start without administrator rights.
- `README.md`: build and run commands, the drawing method, the trace-log and run-directory format (pointing to SPEC), the label-check steps, the negative tests (one short W3 run per fault and what must fail), the geometry check, and what the sandbox could not verify.

```implement-contract
{"allowed_files": ["work/experiments/renderer-sb/probe/*"], "acceptance_check": ["python", "work/experiments/renderer-sb/probe/check_probe.py"], "stop_condition": "the probe source builds with build.cmd; sb_probe.exe --selftest passes (assets, oracle digests, label-check logic with the four faults, turn arithmetic, run.json and trace shape); W1-W4, the W3 trace, the label readback and check, the geometry-check mode and run_scene.ps1 are implemented; README.md gives the owner's build, run, gate and negative-test steps"}
```

## 7. Required return format
- Implementation: changes only in the assigned worktree; final message lists the changed files, the acceptance result (build and self-test output lines), what was not verified in the sandbox, and open points.
