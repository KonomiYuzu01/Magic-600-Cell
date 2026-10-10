# Packet: scoped verification of the J3 theory fixes (senior reviewer, review kind)

Round 2 (the last allowed) checks only the two findings of round 1 (`20261009T225939Z-05de315d`) and whether their fixes add a new `blocker` or `major`:
- **J3W-1** (the replay cached centre checks by pose id, which J1 renumbers after every twist): `invariants.py --replay` no longer caches; it checks every rotated centre after every applied twist, and counts distinct exact pose keys separately. The evidence in `results/invariants.json` and the numbers in Proposition 4.1 come from this rerun.
- **J3W-2** (the finiteness summaries dropped Corollary 3.4's containment hypothesis): the summaries now follow the new **Proposition 3.5** (a menu reaches what its generated group reaches, because admissibility depends only on the cut and same-cap twists compose by Proposition 2.2; no group lies strictly between A4₀ and S4₀, I_a or I_b). The classification is: A4₀ finite; a menu generating S4₀, I_a or I_b reaches what that group reaches (open); a menu contained in none of the three groups is infinite. Check that proof and every summary of it (theory draft items 3 and 7, `docs/progress/1.0/jumbling-midstage.md`, `research/jumbling/explorer/README.md`).

Run read-only as a review by the senior reviewer, with effort `max` and speed tier `fast`. Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: the scoped verification round after the verification `20261009T221306Z-4784adf6` of the J3 theory draft (`research/jumbling/theory/theory-draft.md`). Check only:
  1. whether each blocking finding is fixed, and whether each new or relabelled statement is correct with a complete proof:
     - **J3V-1:** Lemma 1.2a (chamber occupancy, and reachable implies valid) and the restriction of Proposition 1.3 and Theorem 2.1 to valid lattice configurations;
     - **J3V-2:** Proposition 4.1 (centre points are fixed) and its consequence that no nonidentity global K⁺ rotation is reachable and that every centre pose of a reachable lattice configuration lies in A4_c;
     - **J3V-3:** Corollary 3.4 (every menu containing A4₀ outside S4₀, I_a and I_b gives infinite R), the exact E2 trace certificate, and the use of the explorer's exact relations for the realignment and plane menus;
  2. whether the minor fixes J3V-4 (Proposition 2.4) and J3V-5 (g ≠ I in item 6) carry a correct label;
  3. whether the fixes introduce a new `blocker` or `major`, in particular a proved or computed statement that now depends on a lead.
- Acceptance: a schema-valid result. A `blocker` or `major` gives a counterexample or the missing step, with `path:line` evidence.
- Out of scope: statements the first verification found sound and did not flag, presentation, `minor` or `nit` issues other than a wrong label, owner decisions, performance.

## 2. Actual problem and reproduction
- `python research/jumbling/theory/invariants.py <out.json>` checks exactly that each of the 600 caps has one centre piece (signature {c}) whose closed home region contains αn_c, and computes the E2 trace: tr g = (3426525189964 − 4987013400√5)/860378847541, field trace 6853050379928/860378847541, not an algebraic integer.
- `--replay` also replays the three W-J journals with J1 and checks g_c n_c = n_c for every centre after every applied twist (`results/invariants.json`, `results/invariants.log`). This is evidence for Proposition 4.1, not part of its proof.
- The explorer's exact relations: `python research/jumbling/explorer/explore.py --list-menus` prints each exact menu's J1 identity and its relation to S4₀, I_a and I_b (`j1_relation` in `research/jumbling/explorer/explorer-results.json`).

## 3. Environment and versions
- Branch `claude/jumbling-1-0`, current head (the fixes are in `3be8ffb`, the replay evidence in the commit after it). Python 3 with NumPy. Contract revision `state-contract 2026-10-09 A1-A4`. Evidence kind: source and synthetic geometry.

## 4. Necessary source and evidence
- `work/reviews/20261009T221306Z-4784adf6/review.json` (the findings).
- `research/jumbling/theory/theory-draft.md` (the candidate), `research/jumbling/state-contract.md` sections 1–5 and 7.
- `research/jumbling/theory/invariants.py` and `results/invariants.json`.
- `research/jumbling/explorer/explore.py` (`J1Menus.identify`, `group_menus`) for the relations Corollary 3.4 cites.

## 5. Attempts so far
| # | Step | Result |
|---|---|---|
| 1 | Verification `20261009T221306Z-4784adf6` | three majors and two minors, all adopted |
| 2 | Fixes above and `invariants.py` | scoped round 1 `20261009T225939Z-05de315d`: J3V-1 to J3V-5 fixed; new majors J3W-1 and J3W-2, adopted |
| 3 | Uncached replay and Proposition 3.5 | this candidate |

## 6. Constraints and owned files
- Read-only review; no files may change.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`. Finding IDs use the prefix `J3W2`.
- Review only; do not perform follow-up work.
