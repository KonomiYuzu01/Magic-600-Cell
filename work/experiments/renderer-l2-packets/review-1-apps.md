# Review packet, shard A: the level 2 modes of the Godot and Qt apps

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: one full review of the two apps' level 2 modes in the finished level 2 candidate on branch `claude/renderer-l2`.
  - `sa2` (Godot 4.7.2 .NET): `work/experiments/renderer-sa2/project/` (the level 2 files and their changes to the level 1 files), `prepare_l2.py`, `check_project.py` and `README.md`. Codex wrote them in implement call 20261004T063553Z-b6017bfa, and Claude reviewed and integrated them.
  - `sd` (Qt 6.10.3): `work/experiments/renderer-sd/` (`app/src/l2.cpp`, `l2.h`, the level 2 changes to `main.cpp`, `native_loader.*`, `smoke.*`, `app/CMakeLists.txt`, `prepare_l2.py`, `check_project.py`, `README.md`). Codex wrote them in implement call 20261004T055858Z-69aff5bb. Claude reviewed them and removed GPU-based validation.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`.
  - Report only `blocker` or `major` findings, each with a concrete counterexample: a start-up, a run, a window event or a sequence of calls, and the wrong outcome.
  - The verdict is `pass` if there is no such finding.
- Questions:
  1. Does each app implement `HARNESS.md` sections 2 to 6 for its framework?
     - Window: full screen on the primary monitor, topmost, one `SetForegroundWindow`, no overlay of its own.
     - Presentation: swap interval 0, and one produce per present.
     - Sizes and scaling.
     - Options and exit codes.
     - The call order of section 4: produce, ready, shown, trace begin and end, drain, `write_run` only for a completed trace, then unload, unregister, release and detach.
     - Condition sampling every 100 ms in the trace.
     - The `harness.json` shape that `work/experiments/renderer-l2/finalize_run.py` reads.
  2. Can an app record a fact it did not observe? Examples: an adapter, a size, the vsync state, a debug-layer state, a `presents` count that differs from the DLL's traced frames, or a sample count.
  3. Can an app write `harness.json` or a DLL output after a failed or interrupted trace in a way the finalizer would accept? Can a failure path exit 0?
  4. Are the framework facts the apps rely on correct at the pinned versions (`FRAMEWORK-FACTS.md` G1 to G8 and Q1 to Q8)? Do the apps use them as the table's consequence column says?
  5. Does each `prepare_l2.py` write the `launch.json` that `run_scene.ps1` reads (section 9 and the runner)? Check:
     - the executable is the presenting process, not a console wrapper;
     - the argument lists and the separator;
     - Qt keeps `QT_ENABLE_HIGHDPI_SCALING=0`;
     - Godot passes `--render-thread`, `--disable-vsync` and the D3D12 driver, and validation adds `--gpu-validation`.
  6. Is the level 1 smoke mode of each app unchanged apart from the ABI 2 binding?
- Out of scope:
  - the finalizer, the runner and the DLL (shard B reviews them);
  - `PLAN.md`;
  - performance claims;
  - findings below `major`.

## 2. Actual problem and reproduction
No failure is known. Claude's review findings are in section 5. The DLL's exit 3 after an unconfirmed drain writes no `harness.json`; `HARNESS.md` section 4 now says so, and shard B reviews that amendment.

## 3. Environment and versions
- Branch `claude/renderer-l2`, at the commit that adds this packet.
- Windows 11, RTX 4070 Laptop GPU, 2560 x 1600 at 144 DPI (150 %).
- Godot 4.7.2-stable .NET (official build `ed1daf0bf`), Qt 6.10.3 (MSVC 2022 x64).
- Evidence available: source and fixtures only.
  - `python work/experiments/renderer-sa2/check_project.py` passes.
  - `python work/experiments/renderer-sd/check_project.py` passes.
  - Neither app has been built or run in level 2 mode.
  - The framework facts are source-checked at the pinned tags (`FRAMEWORK-FACTS.md`), not run.

## 4. Necessary source and evidence
- Contract: `work/experiments/renderer-l2-packets/HARNESS.md`, sections 2 to 6 and 9; `FRAMEWORK-FACTS.md`; packets `L2-G-godot.md` and `L2-Q-qt.md`.
- The finalizer's input reader: `work/experiments/renderer-l2/finalize_run.py` (`read_harness`, `read_native`, `build_identity`, `check_sizes`, `check_scaling`, `check_conditions`).
- DLL interface: `work/experiments/renderer-sa2/native/include/sa2_interop.h` (ABI 2).
- S-B, whose rules the apps port: `work/experiments/renderer-sb/probe/src/gpu.cpp` (window, conditions, power).

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | L2-Q as written | implement call 20261004T055858Z-69aff5bb | wrapper acceptance check | valid |
| 2 | L2-Q enabled GPU-based validation, which the contract does not ask for; it would slow W3 and W4 past their turns | the debug layer only; `check_project.py` refuses `SetEnableGPUBasedValidation` | `check_project.py` | fixed |
| 3 | L2-G as written | implement call 20261004T063553Z-b6017bfa | wrapper acceptance check | valid |
| 4 | The Godot README's run commands lacked `-Overlays`, which the runner requires | added, set once beside the declarations | — | fixed |

## 6. Constraints and owned files
- Read-only review. No file changes.
- Invariants:
  - synthetic labels only;
  - claims only for the measured build identity;
  - the apps record facts and the finalizer judges them.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
