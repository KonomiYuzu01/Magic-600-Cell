# Packet: review of the exact jumbling witness code (routine reviewer, review kind)

Run read-only as a review with the default reviewer model, effort `max`, speed tier `fast`. Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: check that the code on branch `claude/jumbling-study` implements the plan-checked state contract (`research/jumbling/state-contract.md`, sections 3 and 6) correctly, and that the results in `research/jumbling/README.md` ("Exact witness") follow from it.
- Acceptance: a schema-valid result. Every `blocker` or `major` finding must name the line and give a counterexample or a failing input.
- Out of scope:
  - the mathematics already accepted in the plan checks (the contract itself);
  - the fudging question, which is out of scope by owner decision;
  - `jumble_study.py` beyond regressions;
  - performance, and Windows or DirectX behaviour.

## 2. Actual problem and reproduction
- Reproduce with `python research/jumbling/witness.py`. It takes about 3 minutes on four cores, uses NumPy and the standard library only, and writes `research/jumbling/witness-results.json`.
- Then run `python research/jumbling/witness_float_check.py`, an independent float consistency check of the two blocked-grip sets.
- Claims to check: all items under "Exact witness" in `research/jumbling/README.md`.

## 3. Environment and versions
- Branch `claude/jumbling-study`, current head. Linux, CPython 3, NumPy 2.5.3. Evidence kind: source/fixture and synthetic geometry.

## 4. Necessary source and evidence
- `research/jumbling/exact.py`. Points to check:
  - Q(√5) arithmetic, especially `Q5.sign` and `Q5.inv`;
  - exact Gaussian elimination;
  - `double_description`, an exact cone double description with the combinatorial adjacency test. Is it complete for these inputs, and does it fail loudly on unbounded or degenerate input?
- `research/jumbling/witness.py`. Points to check:
  - `cap_region` and `implied`: is the superset argument sound, given that every constraint is checked at the vertices?
  - `build_region`: the constraints from complete signatures;
  - `classify_grouped`: may a whole pose group be skipped when its posed cap region is on one side?
  - `Config.classify` and `Config.apply`: exact rule; rejection leaves the state unchanged; lattice pieces are classified by transported signature;
  - `witness_rotation`: exact orthogonality, fixed poles, not in A4;
  - the choice of T_d, the E0–E4 flow and the generator agreement check.
- `research/jumbling/witness_float_check.py`.
- `research/jumbling/state-contract.md` sections 3 and 6, which define the required behaviour.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | First witness run | T_d = first third-turn of d | Float cross-check | That T_d fixes c and commutes with g, so E3 was degenerate; replaced by a third-turn that moves c |
| 2 | Corrected witness | T_d with T_d(c) ≠ c | Exact run and float cross-check | 54, then 65 blocked grips; reverse exact; sets agree |

## 6. Constraints and owned files
- `assets/model.npz` and `assets/manifest.json` are read-only inputs. Nothing may change the model or the retained generators. Read-only review; no files may change.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
