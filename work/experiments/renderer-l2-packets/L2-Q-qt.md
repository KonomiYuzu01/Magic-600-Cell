# Packet L2-Q: Qt Quick app, level 2 (E-2.4-03, level 2)

Run from the `claude/renderer-l2` checkout, with no H-06 run, gate capture or other implement call active, other than the parallel L2-F and L2-G calls:
`python tools/agents/codex_review.py --kind implement --model gpt-6.1-sol --effort max --timeout 7200 --packet work/experiments/renderer-l2-packets/L2-Q-qt.md`

Plan: `work/experiments/renderer-l2-packets/PLAN.md` (sections 3 and 4). Contract: `work/experiments/renderer-l2-packets/HARNESS.md`, which is binding. This packet implements its sections 2 to 6 for candidate `sd`, plus the Qt prepare step of section 9. L2-F (the shared finalizer and runner) and L2-G (the Godot app) are written in parallel against the same contract. Where the contract leaves a choice, make it and name it in the final message.

## 1. Goal and acceptance
- Goal: a level 2 mode of the Qt 6.10.3 Quick app.
  - It shows S-B's scenes W1 to W4, drawn by the ABI 2 DLL into the level 1 slots on Qt's own D3D12 device.
  - The window is full screen at native resolution, without vsync, with one produce per present.
  - It writes `harness.json` (`HARNESS.md` section 6) next to the DLL's outputs.
- Acceptance check: `python work/experiments/renderer-sd/check_project.py` exits 0 (`HARNESS.md` section 10, L2-Q part).
- Done when:
  - the level 2 mode implements sections 2 to 6, with the level 1 defaults of Q5;
  - the level 1 smoke mode still works, unchanged, with the ABI 2 DLL;
  - `prepare_l2.py` writes a prepared build with `launch.json` (section 9);
  - `check_project.py` passes, covering section 10 for L2-Q;
  - `README.md` describes the level 2 mode and the owner's commands of section 11 for this candidate.
- Non-goals:
  - the finalizer, the runner and every record other than `harness.json` (L2-F);
  - any change to the DLL or its header;
  - QML, a release package, static linking;
  - building or starting Qt, a C++ compiler or a GPU process in the sandbox;
  - performance claims.

## 2. Actual problem and reproduction
- Level 1 (packet SD-Q, results in `work/experiments/renderer-sd/RESULT.md`) displays a synthetic code texture in a 1280 x 720 window with resizes. Q5 (device `qt`, route `import-direct`, queue `same`, handover `tracked`, barriers `legacy`, render loop `threaded`) passed: zero-copy display through a texture node.
- Level 2 needs the S-B scenes on the framework's device at native resolution, with S-B's checks. The DLL now provides them. ABI 2 adds eight exports: `sa2_scene_load`, `sa2_scene_produce`, `sa2_scene_trace_begin`, `sa2_scene_trace_end`, `sa2_scene_write_run`, `sa2_scene_geometry_check`, `sa2_identity` and `sa2_scene_unload` (see the header).
- Reproduction: `python work/experiments/renderer-sd/check_project.py` fails today. It requires the loader to bind every export the header declares, and the header is now ABI 2 (27 exports, and a fourth struct, `sa2_scene_config`, 32 bytes).

## 3. Environment and versions
- Base: branch `claude/renderer-l2` (this worktree's HEAD).
- Owner's machine:
  - Windows 11 (build 26200);
  - NVIDIA GeForce RTX 4070 Laptop GPU (driver 616.92) and an integrated AMD Radeon 610M;
  - Qt **6.10.3** `msvc2022_64` at `tools/qt/6.10.3/msvc2022_64` (not present in this worktree);
  - Visual Studio Community 2026 (MSVC 14.51), Windows SDK 10.0.26100.0, CMake 4.4.3 and Ninja 1.13.2 from `tools/.venv/renderer-spike/Scripts/`.
- The sandbox has no Qt, no compiler run and no GPU. Python 3.14, standard library only.
- Evidence kind in the sandbox: source/fixture only. Every runtime fact is confirmed later on the owner's machine.

## 4. Necessary source and evidence
- `work/experiments/renderer-l2-packets/HARNESS.md`: binding. Read all of it. Sections 7 and 8 say how the finalizer judges what the app writes.
- The DLL, read-only:
  - `work/experiments/renderer-sa2/native/include/sa2_interop.h`;
  - `work/experiments/renderer-sa2/native/README.md`, "Scene host call order".
- Level 1, which this packet extends:
  - `work/experiments/renderer-sd/app/` (`main.cpp`, `smoke.cpp`, `smoke.h`, `native_loader.cpp`, `native_loader.h`, `CMakeLists.txt`);
  - `work/experiments/renderer-sd/build.cmd`;
  - `work/experiments/renderer-sd/run_smoke.py`: the build, deployment, digest manifest and run environment, which `prepare_l2.py` reuses by import;
  - `work/experiments/renderer-sd/check_project.py` and `work/experiments/renderer-sd/README.md`.
- S-B, read-only:
  - `work/experiments/renderer-sb/probe/src/gpu.cpp`: `sampleConditions`, `environment()`, `DisplayRequest`, the effective power mode, the five-point covered test.
- Qt 6.10.3 facts the code relies on. The sandbox has no Qt source or headers, so list each one as assumed in the final message, with the runtime check of `HARNESS.md` that catches it if it is wrong (`HARNESS.md` line 3). Level 1's README lists the Qt facts already verified at 6.10.3. Claude verifies the new ones against tag `v6.10.3` before the owner's runs:
  - that `showFullScreen()` gives a Direct3D 12 window no border on Windows, and how to keep it topmost;
  - how the D3D12 backend maps swap interval 0 to its swap chain, and with which present flags (PresentMon's `SyncInterval` must be 0);
  - `QQuickWindow::swapChain()->currentPixelSize()` and on which thread it may be read;
  - how Qt chooses the adapter for its own D3D12 device, and how to make it pick the high-performance adapter;
  - `QRhi::driverInfo().deviceName` and `QQuickGraphicsConfiguration::setDebugLayer`.

## 5. Attempts so far
None. This is the first call.

## 6. Constraints and owned files
Owned:
- `work/experiments/renderer-sd/app/` (any file in it);
- `work/experiments/renderer-sd/build.cmd`;
- `work/experiments/renderer-sd/prepare_l2.py` (new);
- `work/experiments/renderer-sd/check_project.py`;
- `work/experiments/renderer-sd/README.md`.

Change nothing else. In particular, these stay byte-identical:
- `run_smoke.py`, `smoke_summary.py`, `sd_reference.py`, `RESULT.md` and `results/`;
- everything under `work/experiments/renderer-sa2/`, including the DLL, its header and `code_layout.json`;
- `HARNESS.md`, `PLAN.md`, everything under `work/experiments/renderer-l2/`, `work/experiments/renderer-sb/` and `tools/`.

Rules:
- Level 1 stays as it is. `run_smoke.py` and the level 1 checks keep working with the ABI 2 DLL. Level 2 is a new mode of the same executable, selected by `--l2-mode` (section 3).
- Bind all 27 exports by exact name, with ABI-derived function types, in the level 1 binding style (`SD_BIND`), and keep the ABI check before any other call. Assert the four struct sizes, as level 1 does for three.
- The level 2 mode uses the DLL's scenes only. It draws nothing over the slot texture: no QML, text or debug display.
- Create the ring only after the window has its final full-screen size, and never rebuild it during the trace (section 4).
- The level 2 mode logs nothing per frame. Startup and shutdown lines are allowed.
- Write `harness.json` on every exit after the options are parsed, atomically (`QSaveFile` or a temporary name and a rename), never over an existing file.
- `prepare_l2.py`:
  - builds the DLL, the app and the deployment the way `run_smoke.py` does, into a new `work/sdb/<stamp>/` (the same path limit, Qt version check and digest manifest); it may reuse the newest build only when the manifest still matches, as `--skip-build` does;
  - writes `<build>/launch.json` (format `magic600-l2-launch-v1`, section 9). Its `executable` is the deployed `sd_smoke.exe`, `dll` is the built DLL, `separator` is `[]`, and `environment` applies level 1's run environment: other Qt installations removed from `PATH`, plugin and import path overrides removed, the same DPI setting. The debug-layer variable and per-frame logging stay out of it;
  - prints the build directory. It starts no Qt process and changes no installed tool.
- `check_project.py`:
  - keeps every level 1 check, updated from ABI 1 to ABI 2;
  - adds the static checks of section 10 for L2-Q, each with a planted-defect test where level 1 has one for the same kind of check;
  - tests `prepare_l2.py`'s `launch.json` writer on a fixture (no build);
  - keeps the level 1 rules: builds nothing, starts no process other than Python, writes nothing in the worktree, and uses a plain `mkdir` fixture folder under the system temp directory, removed on every exit.
- Every Python entry point disables bytecode before local imports. Keep the existing line endings of every file you change.

```implement-contract
{"allowed_files": ["work/experiments/renderer-sd/app/*", "work/experiments/renderer-sd/build.cmd", "work/experiments/renderer-sd/prepare_l2.py", "work/experiments/renderer-sd/check_project.py", "work/experiments/renderer-sd/README.md"], "acceptance_check": ["python", "work/experiments/renderer-sd/check_project.py"], "stop_condition": "the Qt app has a level 2 mode that implements HARNESS.md sections 2 to 6 with Q5's defaults, while the level 1 smoke mode keeps working; the loader binds all 27 ABI 2 exports; prepare_l2.py writes launch.json as section 9 says; check_project.py passes with the ABI 2 pins, the level 1 checks and the static level 2 checks of section 10; README.md documents the level 2 mode"}
```

## 7. Required return format
- Implementation: changes only in the assigned worktree. The final message lists:
  - the changed files;
  - the acceptance result;
  - each Qt 6.10.3 fact the code relies on (section 4 and any other), as assumed, with the runtime check that catches it;
  - what was not verified in the sandbox (every Qt, compiler, GPU and window step);
  - each choice the contract left open, and the choice made;
  - every place where the pinned version contradicts `HARNESS.md`, what you did, and how the check still holds;
  - open points.
