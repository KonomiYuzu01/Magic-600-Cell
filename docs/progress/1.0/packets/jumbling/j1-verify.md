# Packet: scoped verification of the J1 review fixes (senior reviewer, review kind)

Run read-only as a scoped verification review by the senior reviewer, with effort `max` and speed tier `fast`. The fixes were implemented by the routine reviewer model, so the senior reviewer checks them. Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: the first scoped verification round under `AGENTS.md` "Review rounds" for the J1 candidate. Check only two things:
  - Are the six blocking findings of the review (shards `20261009T210942Z-04ccee5c` and `20261009T210943Z-d2ec15dd`) fixed at the current head?
  - Does the fix commit introduce a new `blocker` or `major`?
- The findings:
  - **J1A-01 and J1B-001 (major).** An unchecked K⁺ index let a twist execute a different rotation than its matrix. Counterexamples: `Twist(ctx, 0, ctx.kplus.matrix(0), kidx=int(ctx.kplus.gen_idx[26]))`; `kidx=3` in an A4 menu state.
  - **J1A-02 (major).** Display floats overflowed in a straddle certificate after 26–30 repeated 10° plane twists of cap 0 fixing pole 13.
  - **J1B-004 (major).** Display-angle overflow stopped a valid twist from applying: Cayley ω = ((10¹⁶⁰ + 1)/10¹⁶⁰, 0, 0).
  - **J1B-002 (major).** Tolerance ties violated A1's global minimum: `nearest_parameter([1, 0, 0], 45.00000000002, max_den=1, max_num=1)` chose the identity.
  - **J1B-003 (major).** Replay coerced corrupted exact JSON numbers: matrix entry `[1, 1, 4.25]`.
- Acceptance: a schema-valid result. For each finding ID, say fixed or not fixed, with `path:line` evidence. Report any new `blocker` or `major` with a counterexample.
- Out of scope:
  - everything outside the fix commit's diff to `research/jumbling/sim/` and `tests/test_jumbling_sim.py`;
  - `minor` and `nit` issues.

## 2. Actual problem and reproduction
- The fix commit is the head commit whose message begins "J1: fix the review findings". `git diff HEAD~1 HEAD -- research/jumbling/sim tests/test_jumbling_sim.py` shows the change; if this packet's own commit sits on top, use `HEAD~2 HEAD~1`.
- `python tests/test_jumbling_sim.py`: 38 tests pass, including `test_review_F1` to `test_review_F4`.
- `python research/jumbling/sim/accept.py`: 32 flags pass, including `review_fixes_F1_to_F4`.

## 3. Environment and versions
- Branch `claude/jumbling-1-0`. Linux, Python 3.11, NumPy.

## 4. Necessary source and evidence
- `research/jumbling/sim/twists.py`: `Twist.__init__`, `nearest_parameter`, `INPUT_SEARCH_MARGIN`.
- `research/jumbling/sim/kernel.py`: `q5_float`.
- `research/jumbling/sim/kplus.py`: `q5_from_json`.
- `research/jumbling/sim/state.py`: `_straddle_certificate`.
- `research/jumbling/sim/README.md`: the margin argument for the input map.
- `tests/test_jumbling_sim.py`: the new tests.

## 5. Attempts so far
| # | Step | Result |
|---|---|---|
| 1 | Review of `38652f0`, two shards | six major findings (five defects), all adopted |
| 2 | Implementation `20261009T212441Z-70a9a7ed`, integrated | 38 tests and 32 flags pass |

## 6. Constraints and owned files
- Read-only review; no files may change.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`. Reuse the finding IDs; new findings use the prefix `J1V-`.
- Review only; do not perform follow-up work.
