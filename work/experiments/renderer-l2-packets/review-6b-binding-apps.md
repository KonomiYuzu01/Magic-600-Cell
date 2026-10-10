# Review packet, shard B: what the level 2 apps record for the L2-V-002 binding

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: one full review of the app side of the L2-V-002 candidate on branch `claude/renderer-l2-followup`. The plan is `PLAN-L2-V-002.md` (Astra plan check passed; sections 4, 5 and 7 apply here). The parts:
  1. Qt (`work/experiments/renderer-sd/`):
     - `app/src/module_observer.cpp` and `.h`, new;
     - `app/src/main.cpp`, `l2.cpp`, `l2.h`, `native_loader.cpp` and `.h`;
     - `app/src/module_observer_test.cpp` and `module_probe.cpp`, the observer test program;
     - `build.cmd`, `app/CMakeLists.txt`, `check_project.py` and `README.md`.

     Codex wrote them in implement call 20261009T165242Z-e6ee0506 (G2). Claude reviewed them and added the fixes listed in section 5.
  2. Godot (`work/experiments/renderer-sa2/`):
     - `project/Level2.cs`, `project/Level2Result.cs` and `project/Native.cs`;
     - `check_project.py` and `README.md`.

     Codex wrote them in implement call 20261009T165242Z-6dbc9894 (G3), and Claude reviewed them (commit `6f9d485`).
  - Diff: `git diff 30b4fac HEAD -- work/experiments/renderer-sd work/experiments/renderer-sa2/project work/experiments/renderer-sa2/check_project.py work/experiments/renderer-sa2/README.md`. `30b4fac` is the plan commit. The merge commit `d0e028c` brought unrelated main work and is out of scope.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`.
  - Report only `blocker` or `major` findings, each with a concrete counterexample: a module load sequence, a thread interleaving, an input, or a planted defect that the suite does not catch.
  - The verdict is `pass` if there is no such finding.
- Questions:
  1. Qt observer, over the whole process lifetime: is every module load under the executable's directory either in the record's union or a failed run?
     - before `main`: the unload-history refusal;
     - registration as the first statement of `main`, then the history check and the snapshot;
     - after registration: the slot reservation, the `sealed` flag and the wait for complete slots, with sequentially consistent atomics;
     - overflow;
     - a load after the seal: exit 3.

     Name any interleaving in which an in-scope module is loaded and is missing from the union while the record is written as sealed.
  2. Is the notification callback safe under the loader lock? It must do no file I/O, no heap allocation and take no lock that another loader path can hold.
  3. Is the `qt:` key set exactly the union of loaded in-scope modules, excluding the exe and the DLL, with a refusal for two paths with one basename? Is "in scope" a directory boundary, so that a sibling folder whose name starts with the executable folder's name is not in scope?
  4. Do both apps take `files.dll` from `GetModuleFileNameW` on the handle the loader returned, never from the `--l2-dll` argument, and fail closed (`framework-modules`) when the call fails or truncates?
  5. Godot: is `assembly_mvid` the MVID of the assembly that holds `Smoke`, in `D` format?
  6. Does each planted defect of plan section 7 make `check_project.py` fail? Name a regression of questions 1 to 5 that the suite passes.
  7. Observer test program: does each of its seven cases test what its name claims? Can case 6 pass while the seal ordering is broken?
- Out of scope:
  - `file_guard.py`, `finalize_run.py`, `run_scene.ps1`, `check_guard.py`, `check_l2.py` and `HARNESS.md` (shard A reviews them);
  - the SA2 native DLL, the level 1 files and the 4 October records;
  - modules outside the executable's directory, which the plan does not bind (plan section 9);
  - performance claims;
  - findings below `major`.

## 2. Actual problem and reproduction
No failure is known. Astra's ruling `20261009T155501Z-a2cb4438` found that the level 2 identity named files that the running app might not have loaded (L2-V-002 and -02 to -05). This candidate implements the approved fix.

## 3. Environment and versions
- Branch `claude/renderer-l2-followup`, at the commit that adds this packet.
- Windows 11; MSVC 14.51 with Qt 6.10.3 (MSVC 2022 binaries); Godot 4.7.2 .NET (`4.7.2.stable.mono.official.ed1daf0bf`) with .NET SDK 10.0.401. The READMEs give the build recipes.
- Evidence available:
  - Source: `python -B work/experiments/renderer-sd/check_project.py` and `python -B work/experiments/renderer-sa2/check_project.py` pass.
  - Actual Windows, build `20261010T024533Z` of the Qt app, no GPU work:
    - `sd_module_observer_test.exe` passes 7 of 7 cases (207 child processes) in each of two runs. In case 6, 32 of 200 children exited 3 at the seal in the first run and 27 in the second (`guard_experiments.py`); the rest recorded the probe;
    - the G2 stop point: the deployed `sd_smoke.exe` with an invalid level 2 argument exits 1 with reason `usage`; its record shows the observer sealed, no failure, no overflow, 11 snapshot events and an empty unload history, so a normal Qt startup unloads no module before `main`.
  - No app run with a window, the GPU or PresentMon has used this candidate yet.

## 4. Necessary source and evidence
- Plan: `work/experiments/renderer-l2-packets/PLAN-L2-V-002.md`, sections 4, 5 and 7.
- Contract: `HARNESS.md` sections 6 to 8 (the record fields).
- Godot source facts: `FRAMEWORK-FACTS.md` G9 (the project assembly is loaded from memory).
- Earlier level 2 records: `work/experiments/renderer-sd/RESULT.md` and `work/experiments/renderer-sa2/RESULT.md`.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | G2 output as written | implement call 20261009T165242Z-e6ee0506 | wrapper acceptance check | valid |
| 2 | The observer took the loader lock before registering, so a load between the two could be missed | register the notification first, then take the loader lock for the history check and the snapshot | `check_project.py` pins the order | fixed |
| 3 | A test child that aborts exits 3 under MSVC, the same code as the seal, so a crash could pass case 5 or 6 | the child exits 2 on `SIGABRT` with the abort dialog off | test program | fixed |
| 4 | Case 6 sealed right after starting the loader thread, so the race always fell on one side | the seal is delayed by 0 to 3.98 ms in 20 µs steps; the case reports how many children exited 3 | build `20261010T024533Z`, two runs: 32 and 27 of 200 exited 3 | fixed |
| 5 | `constinit` on the 32 MB state exceeds MSVC's constant-evaluation limit (C2127) | dropped; the members keep constant initializers, and the callback is registered only from `main` | build | fixed |
| 6 | G3 output as written | implement call 20261009T165242Z-6dbc9894 | wrapper acceptance check; `check_project.py` | valid |

## 6. Constraints and owned files
- Read-only review. No file changes.
- Invariants:
  - synthetic labels only;
  - claims only for the measured build identity;
  - the finalizer alone writes records;
  - never commit raw PresentMon output or machine diagnostics.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
