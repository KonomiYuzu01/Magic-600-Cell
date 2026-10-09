# Review packet: scoped verification of the level 2 review fixes

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: a scoped verification round for the level 2 candidate on branch `claude/renderer-l2`. Claude wrote the fixes in three commits: `git diff c08f41d 6f83d0b`.
  1. `ddee7de`, Godot app (`work/experiments/renderer-sa2/project/Level2.cs`, `check_project.py`):
     - L2-A-001;
     - the compile fix found by the first offline build (L2G-R3): `System.Environment` twice and `GetTree().AutoAcceptQuit`.
  2. `092463b`, Qt app (`work/experiments/renderer-sd/app/src/{l2.h,l2.cpp,main.cpp,smoke.cpp}`, `check_project.py`, `FRAMEWORK-FACTS.md` Q9): L2-A-002.
  3. `6f83d0b`, finalizer (`work/experiments/renderer-l2/finalize_run.py`, `check_l2.py`): L2-B-01 and L2-B-02.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`.
  - For each finding of section 2, say whether it is fixed.
  - Report a new `blocker` or `major` only if one of these fixes introduces it, with a concrete counterexample: an input, a run or a sequence of calls and the wrong outcome.
  - The verdict is `pass` if every finding is fixed and nothing new blocks.
- Questions:
  1. L2-A-001: after a complete trace, can a removal reported by the first teardown drain still end in exit 0 or 2? Does the fix keep the removal-safe teardown and the first failure's exit code and reason?
  2. L2-A-002: after a failed `endFrame` (Q9), can the Qt app still:
     - mark that frame shown;
     - count it as presented;
     - end the trace on it;
     - or exit 0?
     Is the handler installed before Qt starts a thread? Does the previous handler still receive every message?
  3. L2-B-01: does the finalizer accept a geometry record whose `build_identity` is not the hash the DLL writes (`scene.cpp` 420-422 and 427-429) for the verified identity? Does it refuse one the DLL writes for a run that meets the contract?
  4. L2-B-02:
     - Can a geometry or label summary that contradicts its counters still become a record?
     - Can a geometry record miss a reference camera or pose?
     - Do the new rules refuse a record the DLL writes for a run that meets the contract? Check them against `checkLabels` (`work/experiments/renderer-sb/probe/src/probe.cpp` 151-178), the DLL's label records and trace (`renderer-sa2/native/src/scene.cpp` 297-360) and the geometry writer (`scene.cpp` 361-432).
  5. Does the compile fix change behaviour?
- Out of scope:
  - everything else in the level 2 candidate (the full review ran in two shards; section 2);
  - performance claims;
  - findings below `major`.

## 2. Actual problem and reproduction
The full review ran in two shards on the fast tier: 20261004T072727Z-2bfa2ace (apps) and 20261004T072727Z-b44bd97c (finalizer, runner, contract and DLL fixes). It found four verified `major` findings:
- L2-A-001: Godot exits 0 when the final drain reports device removal. A normal run completes; the first teardown `sa2_drain` returns `SA2_E_DEVICE_REMOVED`; the cleanup calls succeed. The app skipped `write_run` but wrote `harness.json` with exit code 0 and reason null.
- L2-A-002: Qt can complete a trace after a failed present. On the final trace frame, Qt's `Present` fails other than by device loss, and Qt still emits `afterFrameEnd`. The handler marked the frame shown, ended the trace and set `traceCompleted`, so teardown wrote the outputs and the app exited 0.
- L2-B-01: geometry evidence was not checked against the measured DLL identity. `geometry.json`'s `build_identity` only had to be a non-empty string.
- L2-B-02: contradictory check summaries could become passing evidence:
  - label `status` was not checked against its counters;
  - geometry, result and per-cell statuses were not checked against their counts;
  - an incomplete camera and pose set could pass.

## 3. Environment and versions
- Branch `claude/renderer-l2`, at the commit that adds this packet. Godot 4.7.2 .NET; Qt 6.10.3; Python 3.14.
- Evidence available (source and fixtures, plus offline builds):
  - `python -B work/experiments/renderer-l2/check_l2.py`: 168 finalizer cases, 27 of them new;
  - `python work/experiments/renderer-sa2/check_project.py`: PASS;
  - `python work/experiments/renderer-sd/check_project.py`: ok;
  - both apps' `prepare_l2.py` built offline at `6f83d0b`.
  - No app has run in level 2 mode, and no failed `Present` has been produced (it needs a fault inside Qt).

## 4. Necessary source and evidence
- Qt facts: `FRAMEWORK-FACTS.md` Q9 (sources at tag `v6.10.3`):
  - `qtdeclarative/src/quick/scenegraph/qsgthreadedrenderloop.cpp` 788-799 and 815-818;
  - `qsgrenderloop.cpp` 709-722;
  - `qtbase/src/gui/rhi/qrhid3d12.cpp` 1824-1876;
  - `qtbase/src/corelib/global/qlogging.cpp` 2374-2381.
- Contract: `work/experiments/renderer-l2-packets/HARNESS.md` sections 4 (exit codes) and 7 (refusals).
- S-B reference set: `work/experiments/renderer-sb/reference/index.json` (cameras c0 to c2; poses start, mid and end; 9066 samples; 600 cells of 30480).
- The DLL's identity serialization: `work/experiments/renderer-sb/probe/src/json.h` (`dump`) and `renderer-sa2/native/src/scene_record.cpp` `scene_identity()`.

## 5. Attempts so far
| # | Finding | Change | Verification | Result |
|---|---|---|---|---|
| 1 | L2-A-001 | `Fail(1, "sa2_drain")` when the first teardown drain returns `SA2_E_DEVICE_REMOVED`; teardown continues | `check_project.py` pins the line in the teardown order; planted removal caught | fixed (source) |
| 2 | L2G-R3 | `System.Environment` twice and `GetTree().AutoAcceptQuit = false` | offline C# build | fixed |
| 3 | L2-A-002 | message handler (Q9) flags `Failed to end frame`; `afterFrameEnd` calls `fail("present")` before `sa2_mark_shown` | `check_project.py`, four planted defects; offline build | fixed (source) |
| 4 | L2-B-01 | `build_identity == sha256(canonical(identity))` in geometry mode; hex-64 format | two refusal cases | fixed (fixtures) |
| 5 | L2-B-02 | geometry: reference set and summary consistency; labels: counters, `copies == revisions ==` the trace's last revision on a clean check | 23 refusal cases; two consistent cases (a failed geometry record with counts; a W3 trace with no revision) | fixed (fixtures) |

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
