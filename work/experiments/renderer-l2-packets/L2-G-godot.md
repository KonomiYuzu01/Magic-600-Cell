# Packet L2-G: Godot app, level 2 (E-2.4-02, level 2)

Run from the `claude/renderer-l2` checkout, with no H-06 run, gate capture or other implement call active, other than the parallel L2-F and L2-Q calls:
`python tools/agents/codex_review.py --kind implement --model gpt-6.1-sol --effort max --timeout 7200 --packet work/experiments/renderer-l2-packets/L2-G-godot.md`

Plan: `work/experiments/renderer-l2-packets/PLAN.md` (sections 3 and 4). Contract: `work/experiments/renderer-l2-packets/HARNESS.md`, which is binding. This packet implements its sections 2 to 6 for candidate `sa2`, plus the Godot prepare step of section 9. L2-F (the shared finalizer and runner) and L2-Q (the Qt app) are written in parallel against the same contract. Where the contract leaves a choice, make it and name it in the final message.

## 1. Goal and acceptance
- Goal: a level 2 mode of the Godot 4.7.2 .NET app.
  - It shows S-B's scenes W1 to W4, drawn by the ABI 2 DLL into the level 1 slots on Godot's own D3D12 device.
  - The window is full screen at native resolution, without vsync, with one produce per present.
  - It writes `harness.json` (`HARNESS.md` section 6) next to the DLL's outputs.
- Acceptance check: `python work/experiments/renderer-sa2/check_project.py` exits 0 (`HARNESS.md` section 10, L2-G part).
- Done when:
  - the level 2 mode implements sections 2 to 6, with the level 1 defaults of R2;
  - the level 1 smoke mode still works, unchanged, with the ABI 2 DLL;
  - `prepare_l2.py` writes a prepared build with `launch.json` (section 9);
  - `check_project.py` passes, covering section 10 for L2-G;
  - `README.md` describes the level 2 mode and the owner's commands of section 11 for this candidate.
- Non-goals:
  - the finalizer, the runner and every record other than `harness.json` (L2-F);
  - any change to the DLL or its header;
  - export templates, a release build, an editor or import step;
  - starting Godot, .NET or a GPU process in the sandbox;
  - performance claims.

## 2. Actual problem and reproduction
- Level 1 (packet SA2-G, results in `work/experiments/renderer-sa2/RESULT.md`) displays a synthetic code texture in a resizable 1280 x 720 window. R2 (route `export`, queue `same`, handover `tracked`, barriers `match`) passed.
- Level 2 needs the S-B scenes on the framework's device at native resolution, with S-B's checks. The DLL now provides them. ABI 2 adds eight exports: `sa2_scene_load`, `sa2_scene_produce`, `sa2_scene_trace_begin`, `sa2_scene_trace_end`, `sa2_scene_write_run`, `sa2_scene_geometry_check`, `sa2_identity` and `sa2_scene_unload` (see the header).
- Reproduction: `python work/experiments/renderer-sa2/check_project.py` fails today, because it pins ABI 1 (19 exports and three struct sizes) and the header is now ABI 2 (27 exports and a fourth struct, `sa2_scene_config`, 32 bytes).
- The level 1 runner starts Godot's console executable (`..._console.exe`). That executable is a console wrapper that starts the real executable as a child process. PresentMon and the finalizer need the PID of the process that presents, so level 2 starts `Godot_v4.7.2-stable_mono_win64.exe` itself and logs with `--log-file`. Verify at the pinned version that the console executable is a wrapper.

## 3. Environment and versions
- Base: branch `claude/renderer-l2` (this worktree's HEAD).
- Owner's machine:
  - Windows 11 (build 26200);
  - NVIDIA GeForce RTX 4070 Laptop GPU (driver 616.92) and an integrated AMD Radeon 610M;
  - Godot **4.7.2 .NET**, exact version `4.7.2.stable.mono.official.ed1daf0bf`, from the WinGet package that `run_smoke.py` locates;
  - .NET SDK 10.0.401 with the .NET 8 targeting pack, and Godot's local NuGet folder (`work/experiments/renderer-sa2/README.md`);
  - Visual Studio Community 2026 (MSVC 14.51) for the DLL.
- The sandbox has no Godot, no .NET build and no GPU. Python 3.14, standard library only.
- Evidence kind in the sandbox: source/fixture only. Every runtime fact is confirmed later on the owner's machine.

## 4. Necessary source and evidence
- `work/experiments/renderer-l2-packets/HARNESS.md`: binding. Read all of it. Sections 7 and 8 say how the finalizer judges what the app writes.
- The DLL, read-only:
  - `work/experiments/renderer-sa2/native/include/sa2_interop.h`;
  - `work/experiments/renderer-sa2/native/README.md`, "Scene host call order".
- Level 1, which this packet extends:
  - `work/experiments/renderer-sa2/project/` (`Smoke.cs`, `Native.cs`, `Arguments.cs`, `RunResult.cs`, `CodeLayout.cs`, `Main.tscn`, `project.godot`);
  - `work/experiments/renderer-sa2/run_smoke.py`: the build commands, the Godot executable search and the isolated .NET environment, which `prepare_l2.py` reuses by import;
  - `work/experiments/renderer-sa2/check_project.py` and `work/experiments/renderer-sa2/README.md`.
- S-B, read-only:
  - `work/experiments/renderer-sb/probe/src/gpu.cpp`: `sampleConditions`, `environment()`, `DisplayRequest`, the effective power mode, the five-point covered test.
- Godot 4.7.2 facts the code relies on. The sandbox has no Godot source, so list each one as assumed in the final message, with the runtime check of `HARNESS.md` that catches it if it is wrong (`HARNESS.md` line 3). Claude verifies them against tag `4.7.2-stable` before the owner's runs:
  - which window mode covers the monitor without the 1-pixel border on Windows, and how to set it and topmost from C#;
  - `RenderingDevice.ScreenGetWidth/ScreenGetHeight` for the main window, and on which thread they may be called;
  - what `OS.GetCmdlineArgs()` returns, and that `--disable-vsync` turns vsync off for the D3D12 driver;
  - how the D3D12 driver chooses its adapter, and how to make it pick the high-performance adapter.

## 5. Attempts so far
None. This is the first call.

## 6. Constraints and owned files
Owned:
- `work/experiments/renderer-sa2/project/` (any file in it);
- `work/experiments/renderer-sa2/prepare_l2.py` (new);
- `work/experiments/renderer-sa2/check_project.py`;
- `work/experiments/renderer-sa2/README.md`.

Change nothing else. In particular, these stay byte-identical:
- `work/experiments/renderer-sa2/native/` (the DLL and its header);
- `run_smoke.py`, `smoke_summary.py`, `blank_control.py`, `sa2_reference.py`, `code_layout.json`, `RESULT.md` and `results/`;
- `HARNESS.md`, `PLAN.md`, everything under `work/experiments/renderer-sd/`, `work/experiments/renderer-l2/`, `work/experiments/renderer-sb/` and `tools/`.

Rules:
- Level 1 stays as it is. `run_smoke.py` and the level 1 checks keep working with the ABI 2 DLL. Level 2 is a new mode, selected by `--l2-mode` (section 3). `project.godot` keeps the settings the level 1 check pins; set the level 2 window state at run time.
- Bind all 27 exports by exact name and signature, in the level 1 binding style (`delegate* unmanaged[Cdecl]` and the export lookup that `check_project.py` parses). Guard the four struct sizes at run time, as level 1 does for three.
- The level 2 mode uses the DLL's scenes only. It draws nothing over the slot texture: no UI, text or debug display.
- Create the ring only after the window has its final full-screen size, and never rebuild it during the trace (section 4).
- Write `harness.json` on every exit after the options are parsed, atomically, with a temporary name in `--l2-out` and a rename, never over an existing file.
- `prepare_l2.py`:
  - builds the DLL and the .NET assembly the way `run_smoke.py` does, into a new `work/sa2b/<stamp>/` (the same path limit and isolated environment); it may reuse the newest build only when its source and binary digests still match, as `--skip-build` does;
  - writes `<build>/launch.json` (format `magic600-l2-launch-v1`, section 9). Its `executable` is the non-console Godot executable, `arguments` hold the engine arguments (`--path`, `--rendering-driver d3d12`, `--disable-vsync`, the render thread and the other level 1 engine flags), `run_arguments` hold `--log-file {out}\godot.log`, `validation_arguments` hold `--gpu-validation`, and `separator` is `["--"]`;
  - prints the build directory. It starts no Godot process and changes no installed tool.
- `check_project.py`:
  - keeps every level 1 check, updated from ABI 1 to ABI 2;
  - adds the static checks of section 10 for L2-G, each with a planted-defect test where level 1 has one for the same kind of check;
  - tests `prepare_l2.py`'s `launch.json` writer on a fixture (no build);
  - keeps the level 1 fixture folder rule (a plain `mkdir` under the system temp directory, removed on every exit) and starts no process other than Python.
- Every Python entry point disables bytecode before local imports. Keep the existing line endings of every file you change.

```implement-contract
{"allowed_files": ["work/experiments/renderer-sa2/project/*", "work/experiments/renderer-sa2/prepare_l2.py", "work/experiments/renderer-sa2/check_project.py", "work/experiments/renderer-sa2/README.md"], "acceptance_check": ["python", "work/experiments/renderer-sa2/check_project.py"], "stop_condition": "the Godot app has a level 2 mode that implements HARNESS.md sections 2 to 6 with R2's defaults, while the level 1 smoke mode keeps working; Native.cs binds all 27 ABI 2 exports; prepare_l2.py writes launch.json as section 9 says; check_project.py passes with the ABI 2 pins, the level 1 checks and the static level 2 checks of section 10; README.md documents the level 2 mode"}
```

## 7. Required return format
- Implementation: changes only in the assigned worktree. The final message lists:
  - the changed files;
  - the acceptance result;
  - each Godot 4.7.2 fact the code relies on (section 4 and any other), as assumed, with the runtime check that catches it;
  - what was not verified in the sandbox (every Godot, .NET, GPU and window step);
  - each choice the contract left open, and the choice made;
  - every place where the pinned version contradicts `HARNESS.md`, what you did, and how the check still holds;
  - open points.
