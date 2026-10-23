---
id: owner-decisions-2026-10-23-depth
type: decision
status: verified
visibility: public
summary: Owner decision of 23 October 2026 - the jumbling tutorial's acceptance test keeps all seven depths, 10 to 3000 twists; the research restarts on rules that take height 4 to height 2, which the deeper scrambles need (option B). Whether that research uses a paid API team is open until the owner confirms provider, models and budget.
related: [owner-decisions-2026-10-22-tutorial-first, owner-decisions-2026-10-22-figures-and-merge, owner-decisions-2026-10-20-tutorial-acceptance, owner-decisions-2026-10-20-research-stop, owner-decisions-2026-10-17-jumbling-tutorial]
supersedes: []
claims:
  - {id: depth-not-limited, evidence_kind: decision, checked_at: 2026-10-23}
  - {id: height4-research-restarts, evidence_kind: decision, checked_at: 2026-10-23}
---

# Owner decision, 23 October 2026: no depth limit; research the rules for height 4

The owner gave this decision in a chat message in Chinese; this page records it in English.

## Evidence put to the owner

The [tutorial-first decision](owner-decisions-2026-10-22-tutorial-first.md) left one question open: if the
development runs show that the deeper batches need new rules for height 4 to height 2, the integrator brings the
evidence and the options, and limiting the depth of the test is the owner's call. The evidence on 23 October 2026,
headless on Linux (records under `research/jumbling/restoration/13-tutorial-runner/runs/dev/`):

- D0, depths 10 and 20: both scrambles restored to the solved state; the checker passes both records.
- D2, the six 3000-twist runs: none reached the lattice phase. In each the height phase stuck with between 734 and
  1,955 pieces still at height 4, and at seed 7303 also 87 at height 6. The high phase then stuck with every kernel
  family exhausted at seeds 7304 to 7307, ran out of its clock at 7303 and was cut off by a lost container at 7302.
  The tail's unit listing ran past its time limit at 7304, 7305 and 7307 and was cut off by a lost container at 7306;
  the tail did not run at 7302 and 7303.
- D1, depth 40: in development mode the bulk ended stuck at 1,193 off, all at height 2, and the run was killed in the
  tail after one step; in acceptance mode, which sets no budgets, the bulk went from 90,441 off to 1,193 after 91 steps
  and then only lowered N for six steps (from 1,848 to 1,734), the last three taking 274, 598 and 1,579 seconds
  (checkpoint of 97 steps on branch `claude/dev-d1-acc-s7101`, commit `6fc22ab`).
- D3, depth 100: still in the bulk, from 115,566 off to 29,722 after 121 steps, then N only for six steps; 124 of the
  off pieces are at height 4 (checkpoint of 127 steps on branch `claude/dev-d3-s7102`, commit `922f70a`).

The options: (A) run the acceptance test to depth 100 only for now, (B) restart the research on rules that take
height 4 to height 2, of unknown length, (C) run all seven depths with the current rules and record the failures.
The integrator recommended A.

The D2 and D1 items above were checked against the run records after the decision (`RESULT.md` of each run). The
summary given in the chat said that every D2 tail stopped while listing units and that the depth 40 bulk in
development mode stopped by its budget; the records say otherwise, as given above. Neither point bears on the
choice.

## Decision

Option B. The acceptance test keeps its seven depths, 10 to 3000 twists, as decided on
[20 October](owner-decisions-2026-10-20-tutorial-acceptance.md); scrambles stay unrestricted. The research restarts on
rules that take height 4 to height 2.

## Open

The owner asked whether to do this research with the API. A paid API-team run starts only after the owner confirms the
provider, the model for each role and the budget (`AGENTS.md`, "Ask the owner" and "Records and budgets"); until then
the standing rule holds that the work uses subscription quota only and makes no new costs.

## Integrator readings (the owner may change them)

1. **Scope.** The restart covers the rules the acceptance batches need: a rule that reads only the current state and
   plays literal words, under the human-solve boundary, and takes every height-4 piece to height 2, first at the six
   states where the D2 height phases stopped (`2430f4cc…`, `d01dc91e…`, `ac127dc3…`, `cc5ef7ec…`, `ac4be82c…` and
   `895b45de…`, seeds 7302 to 7307 in that order) and then at the development states of the deeper depths, checked by
   replaying its words. At `d01dc91e…` 87 pieces are still at height 6, where the unlock rule at height 6
   (`r22d81`) found no step; a rule for them is in the same scope, because the batch needs it. It joins the scope of
   reading 1 of the tutorial-first decision ("the research a batch needs in order to pass"); the coverage proofs of
   B′(2) for their own sake and the Archon verification stay stopped.
2. **The bulk at depth 40.** That the acceptance-mode bulk keeps lowering N without landing pieces is runner work on
   the bulk's stopping point, not a new height rule; it needs no owner decision and goes through the normal review.
3. **Batches.** Batch 1 still runs only once the development runs at depths 10, 20, 40 and 100 reach the solved
   state; the batches at 300, 1000 and 3000 wait for the new rules.

## Effect

- The question left open by the tutorial-first decision ("Not decided") is answered: the depth is not limited.
