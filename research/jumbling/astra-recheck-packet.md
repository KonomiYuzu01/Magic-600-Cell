# Packet: scoped re-check of the jumbling study plan (senior reviewer, plan kind)

Run read-only as a scoped plan re-check by the senior Codex reviewer (Astra), effort `max`, speed tier `fast`. Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: confirm that the three blocking findings of plan check 20261009T161321Z-7c78d2bf (C4-Q1, Q4, Q5) are resolved by the changes, and that the changes introduce no new `blocker` or `major`.
- Acceptance: a schema-valid result with one finding or an explicit "resolved" statement for each of C4-Q1, Q4 and Q5. Any new finding must be `blocker` or `major`; report nothing else.
- Out of scope:
  - a second full review;
  - minor findings and nits;
  - the external grip-theory definitions;
  - any fudged variant, which is out of scope by owner decision of 9 October 2026;
  - implementation of the witness itself.

## 2. Actual problem and reproduction
- The earlier plan check returned three blocking findings:
  - **C4-Q1:** the no-fudging conclusion relied on an unstated fixed-slot closure.
  - **Q4:** "unjumble to the lattice, then solve" lacked a handoff condition.
  - **Q5:** the plan built a simulator before defining the state contract and a minimal witness.
- All eight findings were adopted. Reproduce the corrected numbers with `python research/jumbling/jumble_study.py`.

## 3. Environment and versions
- Branch `claude/jumbling-study`; the reviewed candidate is its current head. Evidence kind: source/fixture and synthetic geometry.

## 4. Necessary source and evidence
- `research/jumbling/README.md`, sections "Claims after the plan check", "Plan-check dispositions" and "Plan". C4-Q1 is answered by the narrowed fixed-slot theorem plus the owner decision.
- `research/jumbling/state-contract.md`:
  - sections 1–3 answer Q5's request for definitions: world-fixed grips, piece regions from complete signatures, poses, admissibility with margins, rejection without state change;
  - section 4 answers Q3;
  - section 5 answers Q4;
  - section 6 is the minimal witness E1–E4 and the stop point.
- `research/jumbling/jumble_study.py`: `shells_section` (C1) and `snapping_section` with `max_matching` (C3), for context only.
- The previous findings are summarized in the README dispositions table.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | Plan as first submitted | — | Plan check 20261009T161321Z-7c78d2bf | 8 findings, 3 blocking |
| 2 | Narrow C4, define the contract and handoff, reorder the plan | README and `state-contract.md`; script fixes for C1 and C3 | This re-check | pending |

## 6. Constraints and owned files
- The model, `assets/manifest.json`, the retained generators and the labelled slots stay unchanged. Read-only review; no files may change.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Scoped verification only: is each of C4-Q1, Q4 and Q5 resolved, and does the change introduce a new `blocker` or `major`? Check in particular that the admissibility rule in `state-contract.md` section 3 is sound, including the no-collision argument, and that witness E1–E4 tests what Q5 asked for.
- Review only; do not perform follow-up work.
