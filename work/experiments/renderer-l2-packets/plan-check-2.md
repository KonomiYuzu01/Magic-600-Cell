# Scoped plan re-check packet: level 2 plan, revision of stage day 4

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: a scoped re-check of the revised level 2 plan, `work/experiments/renderer-l2-packets/PLAN.md`. The change is `git diff 8489136 -- work/experiments/renderer-l2-packets/PLAN.md`.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`.
  - Report only `blocker` or `major` findings, each with a concrete counterexample.
  - The verdict is `pass` if there is no such finding.
- Check only these questions:
  1. Does the new section 3 resolve **L2-PLAN-001**, together with the changed L2-G and L2-Q rows? That finding was that no check would catch a producer texture rendered below native resolution and scaled up by the framework. Its counterexample: a 1280×800 producer target stretched across a 2560×1600 window passes the gate.
  2. Does the new section 3 resolve **L2-PLAN-002**? That finding was that the plan assigned no complete run record and no composite build identity, and that a run without peak VRAM could still pass. Its counterexample: three runs that share a harness identity but use different DLL builds pool as one build, with `vram_peak_mb` null.
  3. Does the revision add a new `blocker` or `major`? Check in particular two changes:
     - the ABI 2 header text moves from Claude (L2-H) into the Codex packet L2-N, while Claude keeps the contract and checks the header against it before L2-G and L2-Q start;
     - the note that L2-Q builds against the integrated DLL without changing `renderer-sa2/`.
- Out of scope:
  - every unchanged part of the plan, already checked;
  - the H-06 line in section 6;
  - the design of the packets themselves.

## 2. Actual problem and reproduction
- The Astra plan check of the first revision (call 20261003T213525Z-b263caa3) found the native route and the stated DXIL exception admissible, and the split feasible.
- It raised two `major` findings: L2-PLAN-001 and L2-PLAN-002. Question 1 and question 2 summarise them.
- The integrator adopted both. This revision is the change made for them.

## 3. Environment and versions
- Branch `claude/renderer-l2`, the commit that adds this packet.
- `tools/perf/renderer_gate.py` is unchanged and must stay so, because nothing under the root `tools/` may change in these packets.

## 4. Necessary source and evidence
- `work/experiments/renderer-l2-packets/PLAN.md`: section 3, the work-split table and its notes, and the order table.
- `tools/perf/renderer_gate.py`: lines 60 to 105 for the run contract, and lines 330 to 362 for pooling and identity.
- `work/experiments/renderer-sb/probe/src/gpu.cpp` line 437: S-B's VRAM sampling.
- Packets: `docs/progress/1.0/packets/renderer/E-2.4-02-sa2-godot.md`, `docs/progress/1.0/packets/renderer/E-2.4-03-sd-qt.md` and `docs/progress/1.0/packets/renderer/E-2.4-04-day7-gate.md`.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | The first plan revision is complete | — | Astra plan check 20261003T213525Z-b263caa3 | two `major` findings |
| 2 | Harness-side refusals close both gaps without changing the gate | section 3 and the changed rows | this re-check | — |

## 6. Constraints and owned files
- Read-only check. No file changes.
- Synthetic geometry and labels only.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
