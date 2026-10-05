# Review packet, shard B: level 2 finalizer and runner, contract amendments and the DLL fixes

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: one full review of these parts of the finished level 2 candidate on branch `claude/renderer-l2`:
  1. `work/experiments/renderer-l2/` (`finalize_run.py`, `run_scene.ps1`, `check_l2.py`, `README.md`). Codex wrote them in implement call 20261004T063551Z-b19f1ba3. Claude reviewed them and added the fixes listed in section 5.
  2. The Claude-authored fixes to the SA2 native DLL in commit `3ebf5c0` (`git diff 3ebf5c0~1 3ebf5c0 -- work/experiments/renderer-sa2/native`).
  3. The `HARNESS.md` amendments made after the plan check passed:
     - section 2: the G1 full-screen fact and the adapter sentence;
     - section 4: the DLL's exit 3 after an unconfirmed drain, which leaves no `harness.json`;
     - section 6: the Qt scaling keys and Godot's engine arguments from `GetCommandLineW`;
     - section 7: `--adapter`, the `adapter` refusal, the Qt scaling rule and `expected_adapter`;
     - section 9: `-Adapter`, the declaration requirement and the PresentMon idle refusal.
     `FRAMEWORK-FACTS.md` is their source record.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`.
  - Report only `blocker` or `major` findings, each with a concrete counterexample: an input, a run or a sequence of calls and the wrong outcome.
  - The verdict is `pass` if there is no such finding.
- Questions:
  1. Can the finalizer write a record (exit 0 or 2) for a run that a section 7 predicate should refuse? Can it refuse a run that meets the contract?
  2. Does `run.json` satisfy `renderer_gate.validate_run`? Does it carry S-B's fields with S-B's meaning (`work/experiments/renderer-sb/probe/src/probe.cpp`), so that the unchanged gate judges a level 2 run as it judges an S-B run?
  3. Does the composite identity of section 8 change for any change of a framework part, the DLL, a shader or the configuration? Does it stay the same across run options?
  4. Can the runner produce a record for a run the operator did not confirm? Can it capture a process other than the presenting one, finalize with a stale or foreign capture, or leave a trace session or process behind without naming it?
  5. DLL fixes in `3ebf5c0`:
     - Is the readback reservation correct: S-B's `ceil(trace_ms / turn_ms) + 2` copies at load, and a fail-closed refusal past it?
     - Is the COMMON initial state of the default-heap buffers correct?
     - Is WARP's narrow `unsupported` path correct, and can hardware never take it?
     - Is the build location correct?
     - Do the ABI 2 header, the README and the self-tests agree?
- Out of scope:
  - the apps (`renderer-sa2` outside `native/`, and `renderer-sd`; shard A reviews them);
  - `PLAN.md` and the level 1 files;
  - Codex's L2-N change in `4bc3fe1` (Claude reviewed it), except where a `3ebf5c0` fix depends on it;
  - performance claims;
  - findings below `major`.

## 2. Actual problem and reproduction
No failure is known. Claude's review of the L2-F output found three `major` defects, fixed before this review (section 5). The DLL fixes came from the owner-machine GPU self-test of the L2-N change.

## 3. Environment and versions
- Branch `claude/renderer-l2`, at the commit that adds this packet.
- Windows 11; the owner's machine has an RTX 4070 Laptop GPU and PresentMon 2.6.0. Intel's installer also runs `PresentMonService.exe` as an automatic service.
- Python 3.14. `tools/perf/renderer_gate.py` is unchanged.
- Evidence available:
  - Source and fixtures: `python -B work/experiments/renderer-l2/check_l2.py` passes (141 finalizer cases, the runner's guard inventory and the PowerShell parser).
  - Actual Windows/DirectX for the DLL only: `check_native.py --gpu` on the owner's RTX 4070, 29 of 29 hardware checks (see the `3ebf5c0` message).
  - No app, PresentMon or gate run has used the level 2 harness yet.

## 4. Necessary source and evidence
- Contract: `work/experiments/renderer-l2-packets/HARNESS.md`, sections 6 to 9; `FRAMEWORK-FACTS.md`.
- Gate: `tools/perf/renderer_gate.py`:
  - `condition_reasons` from line 103;
  - `interval_ticks`, `read_frames` and `trace_step_reasons` from line 125;
  - `validate_run`.
- S-B for comparison:
  - `work/experiments/renderer-sb/probe/run_scene.ps1`;
  - `work/experiments/renderer-sb/probe/src/probe.cpp` (run record and camera);
  - `work/experiments/renderer-sb/probe/src/gpu.cpp` (`environment()`, `sampleConditions`).
- DLL record formats:
  - `work/experiments/renderer-sa2/native/src/scene_record.cpp`, lines 80 and 115-121;
  - trace push in `src/scene.cpp`, about line 332;
  - `src/gpu_test.cpp`, lines 470-515.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | L2-F output as written | implement call 20261004T063551Z-b19f1ba3 | wrapper acceptance check | valid, 141 cases |
| 2 | `Assert-Idle` refused `PresentMon.*`, which matches the always-on `PresentMonService.exe`, so every run would stop before launch | refuse only `PresentMon.exe`, `PresentMonUI.exe` and `PresentMon-*.exe` | PowerShell test of the five names; `check_l2.py` pins it | fixed |
| 3 | The README's run commands declared no `frame_generation` or `upscaling`, so the gate would mark every run `conditions-missing` | the README declares both; the runner refuses a run without `-Inject` unless `-Declare` gives each once as `true` or `false` | `check_l2.py` pins it | fixed |
| 4 | The Qt README's run commands lacked `-Overlays`, which the runner requires | added | — | fixed |
| 5 | On an unconfirmed drain the DLL ends the process with exit 3 and no `harness.json`; section 4 promised a harness on every exit, and the runner's exit-3 message named only the window | section 4 names this exit; the runner's message names both causes. The runner already stopped on every exit 3, and the finalizer refuses a run without a harness | `check_l2.py` | fixed |

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
