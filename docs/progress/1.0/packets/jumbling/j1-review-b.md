# Packet: review of the J1 simulator, shard B (senior reviewer, review kind)

Run read-only as a full review by the senior reviewer, with effort `max` and speed tier `fast`. It is one of two concurrent shards of one review. Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: the full review of the finished J1 candidate at commit `38652f0` against `research/jumbling/state-contract.md` (sections 1–5 and amendments A1–A4 in section 7) and J1 acceptance items 1–9 of `docs/progress/1.0/jumbling-plan.md` section 1. J1 is the exact reference engine for the jumbling puzzle; its behaviour is checked against a contract, so the plan assigns its review to the senior reviewer.
- Shard B, twists, menus and records: `twists.py` (Cayley, plane, half-turn, input map, menus), and in `state.py` the journal, replay, digest, snapshot, checkpoint, `export_retained`, `lattice_stickers`, `from_global_rotation`; `accept.py`; `tests/test_jumbling_sim.py`; `README.md`.
- Questions:
- Does the input map return the exact global minimum of amendment A1 over its stated space, with the stated tie order? Check the pruning argument in `nearest_parameter` and the README.
- Is every menu inverse-closed, A4-invariant and containing A4, and is transport independent of the frame (A4)? Can a twist outside the menu be applied in menu mode?
- Are journal records, replay and the identity fields exact and tamper-evident enough for contract section 5? Is the digest a faithful equality test of configurations?
- Is the checkpoint rule right: lattice, centres in their own chambers, a witness word replayed exactly? Can a non-checkpoint be exported?
- Is the labelled projection (A3) a bijection that agrees with the retained primitives, centres included?
- Do the acceptance flags test what their names claim? Name any flag that could pass on a wrong implementation.
- Acceptance: a schema-valid result. Every `blocker` or `major` finding needs a counterexample (a configuration, twist or input) or a failing experiment.
- Out of scope:
  - the other shard's files, except where a finding crosses the boundary;
  - performance;
  - the viewer, the explorer and `witness.py`;
  - `minor` and `nit` style issues;
  - the theory (J3) and owner decisions.

## 2. Actual problem and reproduction
- `python tests/test_jumbling_sim.py`: 32 tests pass in about 25 s.
- `python research/jumbling/sim/accept.py`: 31 acceptance flags pass in about 500 s with 4 workers; the result is `research/jumbling/sim/acceptance.json`.

## 3. Environment and versions
- Branch `claude/jumbling-1-0` at or after `38652f0`. Linux, Python 3.11, NumPy. Evidence kind: source and synthetic geometry.

## 4. Necessary source and evidence
- `research/jumbling/sim/` and `research/jumbling/sim/README.md`.
- `research/jumbling/exact.py` and `research/jumbling/witness.py`: the exact arithmetic and the region builder that J1 reuses.
- `research/jumbling/state-contract.md`.
- `research/jumbling/sim/acceptance.json`.
- `tests/test_jumbling_sim.py`.
- The extension commit `38652f0` was written through the pair-implementation wrapper (call `20261009T204330Z-ce6fc00f`) and read by the integrator. The first candidate (`58edec2`) has had no Codex review.

## 5. Attempts so far
| # | Step | Result |
|---|---|---|
| 1 | First candidate `58edec2` | 22 acceptance flags pass |
| 2 | Plan check and two scoped re-checks of the amended contract | passed |
| 3 | Extension `38652f0` | 31 flags pass; 32 tests pass |

## 6. Constraints and owned files
- Read-only review; no files may change.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`. Finding IDs use the prefix `J1B-`.
- Review only; do not perform follow-up work.
