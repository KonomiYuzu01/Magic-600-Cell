# Plan check packet: binding the level 2 identity to the loaded files (L2-V-002)

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: check the plan `work/experiments/renderer-l2-packets/PLAN-L2-V-002.md` (uncommitted, in the working tree) before any code is written. It answers Astra's escalation ruling `20261009T155501Z-a2cb4438` (findings L2-V-002 and L2-V-002-02 to -05).
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. The verdict is `pass` if the plan, carried out as written, closes every one of the five findings under the stated threat model and has no `blocker` or `major` design defect. Each finding needs a concrete counterexample.
- Questions:
  1. Does a guard acquired before launch (section 2) meet the binding rule of the ruling for every identity part: Godot exe, project assembly, `project.godot`, `Main.tscn`, `Smoke.cs`, native DLL, shaders, Qt exe and every loaded Qt module? Is an existing write handle or writable mapping refused at acquisition?
  2. Is the argument in section 2 sound? It says that a held file with no `FILE_SHARE_DELETE`, plus held ancestor directories, plus a final-path check against the requested path, means that a recorded path equal to a guarded path and with the same file ID names the guarded file for the whole run. Name any path or alias form it misses.
  3. Are the finalizer checks of section 3 sufficient and fail-closed? The checks are: guard alive with matching creation time, write probe, app creation after `ready`, path and file-ID match, same-handle digest, MVID from the same bytes, and the Qt event union. Is folding binding failures into the existing `identity` refusal acceptable?
  4. Does the Qt observer of section 5 give the module-lifecycle coverage that L2-V-002-04 requires? Is the residual pre-`main` case in section 9 acceptable, given that its bytes are protected?
  5. The plan places same-handle hashing, protection lifetime and fail-closed binding in `check_guard.py` and `check_l2.py`, not in the app `check_project.py` suites, because the apps no longer hash. Does this meet the acceptance tests of the ruling? Are the test lists and the owner-machine experiments of section 7 sufficient?
  6. Is the packet split of section 8 sound? G1 to G3 run in parallel, then G4, then G5.
- Out of scope:
  - code (none is written yet);
  - the 4 October records and their wording;
  - S-B's runner and `renderer_gate.py`;
  - performance claims;
  - findings below `major`.

## 2. Actual problem and reproduction
The finalizer hashes every recorded framework file after the run without binding the hash to the bytes the app loaded. The shaders are hashed in a separate read after pipeline creation. Godot reads `project.godot` and `Main.tscn` before any app code runs. The Qt module set is a snapshot taken at the end of the run. The five findings and their counterexamples are in `work/reviews/20261009T155501Z-a2cb4438/review.json`.

## 3. Environment and versions
- Branch `claude/renderer-l2-followup`, from `main` at `7057cb8`.
- Windows 11, Python 3.14, Godot 4.7.2 .NET, Qt 6.10.3, PresentMon 2.6.0.

## 4. Necessary source and evidence
- The plan: `work/experiments/renderer-l2-packets/PLAN-L2-V-002.md`.
- The ruling: `work/reviews/20261009T155501Z-a2cb4438/review.json`; the escalation packet: `work/experiments/renderer-l2-packets/escalation-L2-V-002.md`.
- `work/experiments/renderer-l2/finalize_run.py`: `build_identity` (line 246).
- `work/experiments/renderer-l2/run_scene.ps1`: `Start-App` (line 120) and the per-run sequence (lines 158-245).
- `work/experiments/renderer-l2/check_l2.py`: `fixture`, `main`.
- `work/experiments/renderer-sa2/project/Level2.cs` (lines 91-94 and `ProjectAssemblyPath`, line 141).
- `work/experiments/renderer-sd/app/src/l2.cpp`: `files()` (line 407).
- `work/experiments/renderer-l2-packets/HARNESS.md`, sections 5 to 7.
- `tools/perf/renderer_gate.py`: `validate_run` (line 71).

## 5. Attempts so far
- Claude's first proposal (escalation packet section 5) had three parts:
  - locks taken at app start;
  - mapped-name comparison;
  - an MVID check.

  Astra ruled it insufficient (findings -03 and -05).
- This plan is the second design. It follows the ruling's "guard acquired before process launch".

## 6. Constraints and owned files
- Owned: the plan file and this packet. No code changes in this call.
- Not a critical path. It is a contract change to the level 2 harness, so it gets an Astra plan check (fast tier, as the owner set for every plan check).

## 7. Required return format
One JSON object matching `schemas/review-result.schema.json`, with ID, severity, evidence (`path:line` with file digest), counterexample, suggested experiment and verification status for each finding.
