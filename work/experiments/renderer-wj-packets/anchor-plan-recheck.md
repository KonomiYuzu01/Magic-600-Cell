# Scoped plan re-check: area-centroid shrink anchors

Review only; do not perform follow-up work. Read only.

## 1. Goal and acceptance
- **Goal.** Check only whether the two findings of plan check `20261010T053301Z-f2cab142` are resolved by the plan changes in commit `a42845e`, and whether those changes introduce a new `blocker` or `major`.
- **Acceptance.** A result per `schemas/review-result.schema.json`:
  - `pass` when both findings are resolved and nothing new is blocking;
  - otherwise one finding per unresolved item.
- **Out of scope.** Everything the first plan check covered and did not flag; the implementation (two implement calls are running and are reviewed later); wording.

## 2. Actual problem and reproduction
- **ANCHOR-PLAN-1 (major).** Step 6 of the plan did not schedule W3 captures of the changed W-J executable.
  - Fix: `work/experiments/renderer-wj-packets/anchor-plan.md` section 6, step 6, W-J bullet. It now schedules three cold W3 runs of the changed W-J executable (`run_scene.ps1 -Scene w3 -Runs 3`), with the unchanged judge and the same executable and shader identity as its W-J runs.
- **ANCHOR-PLAN-2 (minor).** The continuity check covered only the 4,600 `move_src`/`move_dst` pairs.
  - Fix: `work/experiments/renderer-wj-packets/anchor-sb.md` section 6. The check now covers all 4,605 animated slots: the pairs, plus `(s, s)` for the five `moving_slots` not in `move_src`, derived from `turn.json`. The stop condition now names the 4,605 animated slots.

## 3. Environment and versions
- Windows 11, branch `claude/wj-area-centroid`, HEAD `a42845e`.

## 4. Necessary source and evidence
- `work/experiments/renderer-wj-packets/anchor-plan.md` section 6.
- `work/experiments/renderer-wj-packets/anchor-sb.md` sections 6 and the implement contract.
- `work/experiments/renderer-wj/README.md` (the W3 rerun rule near line 78).
- `work/reviews/20261010T053301Z-f2cab142/review.json`, if readable; its findings are quoted above.

## 5. Attempts so far
- The first plan check found these two items; both were adopted (`dispositions.json` of that call).

## 6. Constraints and owned files
- Read only. Do not open or change the running implement worktrees under `work/worktrees/`.

## 7. Required return format
- Findings per `schemas/review-result.schema.json`, or `pass` with no findings.
