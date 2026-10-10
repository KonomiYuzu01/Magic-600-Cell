# Packet: verification of the J3 theory draft (senior reviewer, review kind)

Run read-only as a review by the senior reviewer, with effort `max` and speed tier `fast`. Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: the one Astra verification that mid-stage row J3 of `docs/progress/1.0/jumbling-plan.md` section 2 requires. The draft `research/jumbling/theory/theory-draft.md` labels each statement as proved, computed, lead or open. Check:
  1. Is every **proved** statement correct and its proof complete from the definitions of the state contract (`research/jumbling/state-contract.md`) and unconditional geometry? In particular Propositions 1.1–1.3, Theorem 2.1 (all four parts), Propositions 2.2, 2.3, 3.1, 3.2 and 3.3, and the proved remarks of items 4–6.
  2. Does every **computed** statement follow from the named script by the stated algorithm, with every decision exact? The scripts are `research/jumbling/theory/blocking.py`, `groups.py`, `conj.py` and `conj2.py`, with results in `research/jumbling/theory/results/`.
  3. Is every **lead** labelled as a lead, and does no proved or computed statement depend on a lead?
  4. Is anything labelled open actually settled by the draft or the J1 acceptance, or anything labelled proved actually open?
- Acceptance: a schema-valid result. Each finding names the statement, the label it should have, and `path:line` evidence. A `blocker` or `major` finding gives a counterexample or the missing step. Report also which statements you checked and found sound, in the summary.
- Out of scope:
  - the J1 code (`research/jumbling/sim/`, accepted after its own review);
  - presentation, wording and `minor` or `nit` issues, except a wrong label;
  - owner decisions, including the menu choice;
  - performance.

## 2. Actual problem and reproduction
- The draft was written by the integrator with exact J1 computations, as the plan's second J3 route allows. Its claims feed the menu decision, the renderer packet (`E-2.4-0J`) and the engineering packet (`ENG-0J`).
- Reproduce the computations from the repository root:
  - `python research/jumbling/theory/groups.py` (exact closure checks);
  - `python research/jumbling/theory/blocking.py <out.json>` (eight twists, all 600 grips; minutes);
  - `python research/jumbling/theory/conj.py S4 <out.json>` and `I_a` (about 20 minutes each), then `python research/jumbling/theory/conj2.py <dir>`.

## 3. Environment and versions
- Branch `claude/jumbling-1-0`, current head. Python 3 with NumPy. Contract revision `state-contract 2026-10-09 A1-A4`. Evidence kind: source and synthetic geometry.

## 4. Necessary source and evidence
- `research/jumbling/theory/theory-draft.md` (the candidate) and `research/jumbling/theory-problem.md` (the questions).
- `research/jumbling/state-contract.md`, sections 1–5 and 7.
- `research/jumbling/theory/*.py` and `research/jumbling/theory/results/*.json`.
- `research/jumbling/sim/README.md` for the J1 operations the scripts call (`survey`, `classify`, `apply`, `digest`, `checkpoint`, `kplus`, `regions`).
- `research/jumbling/sim/acceptance.json`, section `negative_control` (the exact trace of Proposition 3.2).

## 5. Attempts so far
| # | Step | Result |
|---|---|---|
| 1 | Problem statement revised after the plan check and two re-checks | passed |
| 2 | Draft and exact scripts (`a2167ff`); `conj.py` re-run after its centre check was made exact | this candidate |

## 6. Constraints and owned files
- Read-only review; no files may change.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`. Finding IDs use the prefix `J3V`.
- Review only; do not perform follow-up work.
