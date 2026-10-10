# Scoped plan re-check: WJ-P1 and WJ-P2

Review only; do not perform follow-up work. Read only.

## 1. Goal and acceptance
- Goal: check only whether plan-check findings WJ-P1 and WJ-P2 (call `20261010T022650Z-3c3bcd36`, `work/reviews/20261010T022650Z-3c3bcd36/review.json`) are fixed in `work/experiments/renderer-wj-packets/judge-wj.md` and `docs/progress/1.0/packets/renderer/E-2.4-0J-jumbling.md`, and whether the fixes introduce a new `blocker` or `major`.
- Acceptance: a result per `schemas/review-result.schema.json`; one finding per unfixed item; no full review.
- Out of scope: everything the first check did not raise, wording, `minor` and `nit` items.

## 2. Actual problem and reproduction
- WJ-P1: `pose_check` counts were not reconciled with the trace. Fix: `pose_check.revisions` lists every checked revision, and the judge requires it to equal the set of trace revisions in [`trace_start_qpc`, `trace_stop_qpc`], with tests for under-reported, over-reported and missing entries (judge-wj.md section 6; E-2.4-0J section 4, "Gate and judge").
- WJ-P2: the single replay must keep the exact J1 digest check of every stage. Fix: E-2.4-0J section 6, the `check_wj.py` item: each stage's digest and array hashes are captured before the next record and compared by one function, which is also run against in-memory fixture copies with one altered digest per stage and one altered array hash, each required to fail, with no extra replay.

## 3. Environment and versions
- As in `work/experiments/renderer-wj-packets/plan-check.md`.

## 4. Necessary source and evidence
- `work/experiments/renderer-wj-packets/judge-wj.md`, `docs/progress/1.0/packets/renderer/E-2.4-0J-jumbling.md` (diff against `HEAD`), `tools/perf/renderer_gate.py` (`read_trace`, `trace_reasons`), `research/jumbling/fixtures/wj.py`.

## 5. Attempts so far
- Plan check `20261010T022650Z-3c3bcd36`: WJ-P1 and WJ-P2 (major), both adopted.

## 6. Constraints and owned files
- Read only. The selection gate is fixed.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`, findings citing `path:line`. Review only; do not perform follow-up work.
