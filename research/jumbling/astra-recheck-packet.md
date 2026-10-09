# Packet: second scoped re-check of the jumbling study plan (senior reviewer, plan kind)

Run read-only as a scoped plan re-check by the senior Codex reviewer (Astra), effort `max`, speed tier `fast`. Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: this is the second and last scoped verification round. Confirm that finding Q5 of re-check 20261009T163617Z-bc992d63 is resolved, and that the change introduces no new `blocker` or `major`. That re-check already found C4-Q1 and Q4 resolved.
- Acceptance: a schema-valid result that either states Q5 resolved or gives a finding for it. Any new finding must be `blocker` or `major`; report nothing else.
- Out of scope:
  - a second full review;
  - minor findings and nits;
  - the external grip-theory definitions;
  - any fudged variant, which is out of scope by owner decision of 9 October 2026;
  - implementation of the witness itself.

## 2. Actual problem and reproduction
- Re-check 20261009T163617Z-bc992d63, Q5 (major): the inside and outside predicates had a tie band τ = 1e-9, so shallow crossings were accepted and the no-collision argument did not hold.
- The fix is in `research/jumbling/state-contract.md`:
  - section 3 now defines the classes by exact one-sided containment, and certifies them exactly in Q(√5): complete exact vertex sets of a proven superset for inside and outside, two exact witness points for straddling, rejection when uncertain;
  - section 6 now uses an exact Q(√5) witness rotation and adds E0 controls: shallow-crossing negatives and an exact-contact positive.

## 3. Environment and versions
- Branch `claude/jumbling-study`; the reviewed candidate is its current head. Evidence kind: source/fixture and synthetic geometry.

## 4. Necessary source and evidence
- `research/jumbling/README.md`, sections "Claims after the plan check", "Plan-check dispositions" and "Plan". C4-Q1 is answered by the narrowed fixed-slot theorem plus the owner decision.
- `research/jumbling/state-contract.md`:
  - sections 1–3 answer Q5's request for definitions: world-fixed grips, piece regions from complete signatures, poses, exact admissibility with certificates, rejection without state change;
  - section 4 answers Q3;
  - section 5 answers Q4;
  - section 6 is the minimal witness E0–E4 and the stop point.
- `research/jumbling/jumble_study.py`: `shells_section` (C1) and `snapping_section` with `max_matching` (C3), for context only.
- The previous findings are summarized in the README dispositions table.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | Plan as first submitted | — | Plan check 20261009T161321Z-7c78d2bf | 8 findings, 3 blocking |
| 2 | Narrow C4, define the contract and handoff, reorder the plan | README and `state-contract.md`; script fixes for C1 and C3 | Re-check 20261009T163617Z-bc992d63 | C4-Q1, Q4 resolved; Q5 open (tie band) |
| 3 | Exact containment and exact certificates | `state-contract.md` sections 3 and 6 | This re-check | pending |

## 6. Constraints and owned files
- The model, `assets/manifest.json`, the retained generators and the labelled slots stay unchanged. Read-only review; no files may change.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Scoped verification only: is Q5 resolved, and does the change introduce a new `blocker` or `major`? Check in particular:
  - the soundness of the exact certificates in section 3, including the superset argument and the straddling witness points;
  - that the Q(√5) witness rotation in section 6 is exactly orthogonal, fixes both poles and lies outside A4_c;
  - that E0 covers the counterexamples of the re-check.
- Review only; do not perform follow-up work.
