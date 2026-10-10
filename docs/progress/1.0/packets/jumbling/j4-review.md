# Packet: review of the J4 grip-orbit explorer (senior reviewer, review kind)

Run read-only as a review by the senior reviewer, with effort `max` and speed tier `fast`. Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: the first independent review of workstream J4 (`research/jumbling/explorer/`). The explorer was written by a Claude agent (`434c8fd`) and extended by a Codex implementation (`574ba44`: group menus S4₀, I_a and I_b, J1 menu identities and relations, filtered preset merge). The senior reviewer reviews it so that neither author reviews its own code. Check:
  1. **Exactness.** Every statement the README labels exact is decided in Q(√5): the exact realignment generators (two aligned poles, four-dimensional cross product), the group menus and their checks (rotation, fixed n₀, inverses, closure, inclusion of A4), the J1 frame map (pole match by a positive exact scale, K⁺ membership, transport of A4) and the J1 identity and relation of each exact menu.
  2. **Reduction.** The lattice closure skips A4 twists and the all-grips closure counts Σ 7200/|Stab|. Is each reduction valid for the stated closure, and does `--selftest` compare it with a direct search that does not share the reduced code path?
  3. **Labels.** The README and the page call the closure counts leads, not proofs, and name the ball of 46.8° as a stand-in for the cap geometry. Is anything stated as established that the code only samples or estimates?
  4. **Merge.** `--preset --menus` keeps other run records byte for byte, refuses different search parameters and unknown menus, and never offers a menu without runs.
  5. **Page.** `explorer.js` and `check_page.mjs` show and check the identity and relation fields from `explorer-results.json` only; no host outside the allowlist is requested.
- Acceptance: a schema-valid result. Each `blocker` or `major` finding gives a counterexample or a failing input and `path:line` evidence. The summary lists what was checked and found sound.
- Out of scope:
  - J1 (`research/jumbling/sim/`, accepted after its own review) and the theory draft (`research/jumbling/theory/`, reviewed separately);
  - the result numbers of the earlier preset as such (they are leads); only whether the code computes what the README says;
  - look and wording, `minor` and `nit` issues;
  - performance.

## 2. Actual problem and reproduction
- From the repository root: `python research/jumbling/explorer/explore.py --selftest` (all lines `ok`), `--list-menus` (identities and relations), `--menu class-05 --depth 3`.
- Computed identities: class-01 and class-02 give the same J1 identity (`19a4199f…`), so their closures must agree; the README records the earlier sampled difference for that pair as needing a recheck. class-00 ⊂ S4₀, class-03 ⊂ I_a, class-04 ⊂ I_b.

## 3. Environment and versions
- Branch `claude/jumbling-1-0`, current head. Python 3 with NumPy, Node 22 for the page check. Evidence kind: synthetic geometry.

## 4. Necessary source and evidence
- `research/jumbling/explorer/explore.py`, `explorer.js`, `index.html`, `check_page.mjs`, `README.md`.
- `research/jumbling/h4.py`, `exact.py`, `witness.py` (shared helpers), `research/jumbling/state-contract.md` sections 1–3.
- `research/jumbling/theory/groups.py` (`build`, `lift`) and `research/jumbling/sim/` (`get_context`, `TwistMenu`) as called by `J1Menus`.
- `docs/progress/1.0/packets/jumbling/j4-group-menus.md` (the extension's contract).

## 5. Attempts so far
| # | Step | Result |
|---|---|---|
| 1 | Explorer and preset (`434c8fd`) | published; not reviewed |
| 2 | Group menus, J1 identities, merge (`574ba44`) | integrator read the patch; self-test all `ok` |

## 6. Constraints and owned files
- Read-only review; no files may change.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`. Finding IDs use the prefix `J4R`.
- Review only; do not perform follow-up work.
