# Scoped plan re-check packet: level 2 plan, harness contract and three-packet split

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: a scoped re-check of one change to the level 2 plan.
  - Before: section 3's checks were made by each app's harness, L2-G (Godot) and L2-Q (Qt).
  - After: the checks are specified exactly in a new contract, `work/experiments/renderer-l2-packets/HARNESS.md`. One shared finalizer and runner (new packet L2-F) makes them for both candidates, and the apps only record facts.
  - The plan change is `git diff bf476d4 -- work/experiments/renderer-l2-packets/PLAN.md`. `HARNESS.md` is new.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`.
  - Report only `blocker` or `major` findings, each with a concrete counterexample.
  - The verdict is `pass` if there is no such finding.
- Check only these questions:
  1. Does `HARNESS.md` keep **L2-PLAN-001** closed, now that the apps record and the finalizer checks? The finding: no check would catch a producer texture rendered below native resolution and scaled up by the framework. Counterexample: a 1280×800 producer target stretched across a 2560×1600 window passes the gate. Look at sections 2, 6 and 7 (`size-mismatch`, `scaling`).
  2. Does `HARNESS.md` keep **L2-PLAN-002** closed? The finding: no complete run record and no composite build identity, and a run without peak VRAM could still pass. Counterexample: three runs that share a harness identity but use different DLL builds pool as one build, with `vram_peak_mb` null. Look at sections 6 to 8.
  3. Does the split add a new `blocker` or `major`? In particular:
     - an interface gap between the app side (sections 2 to 6) and the finalizer and runner (sections 7 to 9), so that the three packets, written in parallel, cannot fit together;
     - a fact the finalizer trusts that the app could misreport without any check noticing, where that would let an invalid run reach `renderer_gate.py`;
     - the presenting process: PresentMon and the finalizer must see the PID of the process that presents (section 9, "Launch");
     - the PresentMon checks of section 7 (`swap-chain`, `sync-interval`, `blind-seconds`, `trace-steps`) against S-B's runner;
     - the identity rule that run options never enter the identity (section 8), so validation, geometry, short and gate runs share one identity.
- Out of scope:
  - every unchanged part of the plan, already checked twice;
  - the integrated L2-N DLL;
  - the H-06 line in section 6;
  - the packets' wording beyond their owned files and acceptance checks.

## 2. Actual problem and reproduction
- The Astra plan check (20261003T213525Z-b263caa3) raised L2-PLAN-001 and L2-PLAN-002. The scoped re-check of their fix (`plan-check-2.md`) passed.
- Writing the packets showed that two harnesses would carry two copies of the same refusals, record formats and runner. A difference between the candidates could then come from the copies, not from the frameworks.
- The integrator moved the checks into one finalizer and runner (L2-F) and fixed the interfaces in `HARNESS.md`. This re-check covers that change only.

## 3. Environment and versions
- Branch `claude/renderer-l2`: the commit that adds this packet, or this worktree's files if it is not yet committed.
- `tools/perf/renderer_gate.py` is unchanged and must stay so: nothing under the root `tools/` may change in these packets.

## 4. Necessary source and evidence
- `work/experiments/renderer-l2-packets/HARNESS.md` (all of it) and `PLAN.md` sections 3 and 4.
- Packets, sections 1 and 6 only: `L2-F-finalizer.md`, `L2-G-godot.md`, `L2-Q-qt.md` in the same folder.
- `tools/perf/renderer_gate.py`: lines 60 to 105 for the run contract, and lines 330 to 362 for pooling and identity.
- S-B: `work/experiments/renderer-sb/probe/run_scene.ps1` (the runner ported by L2-F) and `work/experiments/renderer-sb/probe/src/gpu.cpp` (`sampleConditions`, `environment()`).
- The DLL's record writer: `work/experiments/renderer-sa2/native/src/scene_record.cpp` and the ABI 2 section of `work/experiments/renderer-sa2/native/include/sa2_interop.h`.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | The first plan revision is complete | — | Astra plan check 20261003T213525Z-b263caa3 | two `major` findings |
| 2 | Harness-side refusals close both gaps without changing the gate | section 3 and the changed rows | scoped re-check (`plan-check-2.md`) | pass |
| 3 | One shared finalizer and runner, with an exact contract, keeps both gaps closed and lets three packets run in parallel | `HARNESS.md`, section 4 rows and sequencing | this re-check | — |

## 6. Constraints and owned files
- Read-only check. No file changes.
- Synthetic geometry and labels only.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
