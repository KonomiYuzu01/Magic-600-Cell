# Scoped verification packet: level 2 harness contract, after plan re-check 20261004T054448Z-d64bab68

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: a scoped verification round of `work/experiments/renderer-l2-packets/HARNESS.md` after the four `major` findings of plan re-check 20261004T054448Z-d64bab68. The integrator adopted all four.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`.
  - Report only `blocker` or `major` findings, each with a concrete counterexample.
  - The verdict is `pass` if there is no such finding.
- Check only these questions:
  1. **L2-HARNESS-001.** Is every finalizer mode mapped to the app mode it expects? Section 7 now says `run`, `short` and `validation` expect app mode `run`, and `geometry` expects `geometry`; the `harness` refusal uses that mapping.
  2. **L2-HARNESS-002.** Are geometry inputs and checks separate from the trace modes? Section 7 now lists the DLL outputs per mode, and the `native` and `identity` checks have a geometry case: `geometry.json` only, with the identity checked against `dll_identity` and the part files.
  3. **L2-HARNESS-003.** Are contradictory condition samples caught? Section 6 now spells out S-B's derivation of `power_source` from `power_samples`, and section 7 has a new `conditions` refusal for contradictory counts and for visibility or foreground failures in a finished run. A consistent battery or changed record passes the finalizer and fails the gate's `condition_reasons`. Section 10 adds the fixtures.
  4. **L2-HARNESS-004.** Is the output-directory conflict gone? `--l2-out` now refuses only the app's and the DLL's output files. The runner creates each run directory and checks it is empty before it starts the app, so the framework's log and PresentMon's CSV may land there first.
  5. Does any of these changes add a new `blocker` or `major`?
- Out of scope: every other part of `HARNESS.md`, `PLAN.md` and the packets; findings below `major`.

## 2. Actual problem and reproduction
The four findings, their evidence and counterexamples are in the re-check result. Questions 1 to 4 restate them.

## 3. Environment and versions
- Branch `claude/renderer-l2`: the commit that adds this packet, or this worktree's files if it is not yet committed.

## 4. Necessary source and evidence
- `work/experiments/renderer-l2-packets/HARNESS.md`: sections 3 (`--l2-out`), 6 (`environment`), 7 (mode mapping, inputs, the refusal table), 9 ("Run directories") and 10 (L2-F fixtures; L2-G and L2-Q static checks).
- `tools/perf/renderer_gate.py` lines 103 to 122 (`condition_reasons`).
- `work/experiments/renderer-sb/probe/src/gpu.cpp` lines 455 to 478 (`sampleConditions`, `environment()`).
- `work/experiments/renderer-sa2/native/include/sa2_interop.h` (ABI 2 section): which call writes which file.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | `HARNESS.md` as first written | — | plan re-check 20261004T054448Z-d64bab68 | four `major` findings |
| 2 | The four contract fixes close them | sections 3, 6, 7, 9 and 10 | this verification round | — |

## 6. Constraints and owned files
- Read-only check. No file changes.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
