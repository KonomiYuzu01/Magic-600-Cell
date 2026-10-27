---
id: owner-decisions-2026-10-27-freeze
type: decision
status: verified
visibility: public
summary: Owner decisions of 27 October 2026 - the jumbling tutorial is shelved and the freeze of the tutorial and the jumbling research holds at once; card 9's last twist-by-twist replays are stopped, and only its record and the freeze record are merged.
related: [owner-decisions-2026-10-26-tail-then-freeze, owner-decisions-2026-10-22-tutorial-first, owner-decisions-2026-10-23-depth]
supersedes: []
claims:
  - {id: tutorial-shelved, evidence_kind: decision, checked_at: 2026-10-27}
  - {id: freeze-now, evidence_kind: decision, checked_at: 2026-10-27}
  - {id: depth-3000-restorable, evidence_kind: source, path: research/jumbling/state-contract.md, checked_at: 2026-10-27}
---

# Owner decisions, 27 October 2026: the tutorial shelved, the freeze at once

The owner gave these decisions in three chat messages in Chinese; this page records them in English.

## Evidence put to the owner

The owner asked what was still missing before a complete tutorial could be written. The integrator's answer (chat,
27 October 2026), headless on Linux:

- A complete tutorial needs the runner to restore all 21 acceptance scrambles, and none has run. Depths 10 and 20 are
  solved and checked.
- Depth 40 (seed 7101) lacks engineering: card 9 gave a state-only whole-cap plan rule that completes the tail stop of
  the development run in three literal steps (688,686 letters), after which the lattice phase solves; the rule is not
  in the runner and was not run from the run's tail entry, where whole caps alone stick (proved) and a mixed rule's
  completion is conditional.
- Depth 100 (D3, seed 7102) lacks a bulk rule that lowers the off count, a research question whose card never opened:
  after 530 steps the run stood at 14,528 pieces off, unchanged since 25 October, with N at 19,560 falling by about 1
  to 20 an hour.
- Depths 300 to 3000 lack complete rules for the height and high phases, the hardest research questions: the six
  depth 3000 runs stopped in the height phase and four of them in the high phase as well; cards 6 to 8 and stage 17
  lowered Φ at the six ends by 65% to 86%, to zero at none.
- The text (chapters 1 to 7 and Appendix A) covers the existing rules; new rules would rewrite chapters 3 to 5.
- The integrator recommended the freeze as planned and named one option: limit the tutorial to depth 40 and unfreeze
  one engineering item (the plan rule in the runner, run from the tail entry, then chapter 5 and the acceptance
  scope), which would need the owner to change the freeze and the acceptance scope.

## Decisions

1. The tutorial is not written further for now; it is shelved.
2. Later the same morning the owner found card 9's last twist-by-twist replays still running and said the freeze
   was meant to hold already. The integrator stopped them. The owner then allowed bookkeeping only: the card's record
   and the freeze record are written from what exists and merged, with no further computation.

## The owner's question before the stop

Before the stop the owner asked whether a depth 3000 scramble can certainly be restored. The answer (proved):

- Yes. A scramble is a finite sequence of admissible twists from the solved state. An admissible twist (c, g) moves
  only the pieces inside the cut of grip c, and g fixes the grip's pole, so afterwards every piece is still inside or
  outside that cut and (c, g⁻¹) is admissible and undoes it exactly
  ([state contract](../../../research/jumbling/state-contract.md) section 3); the menu I_a contains every inverse. The
  inverse twists in reverse order restore the solved state.
- From the current state alone, without the scramble: Theorem M\*
  ([restoration](../../../research/jumbling/restoration/README.md), proved in the senior model review) restores every
  reachable configuration, with an exhaustive search over words as its fallback; that search has no bound on its
  length. The practical rules do not restore a depth 3000 scramble: all six depth 3000 development runs stopped in
  the height phase. Whether they or any rule with a useful bound always finish is not established, and the freeze
  leaves it so.

## Integrator readings (the owner may change them)

- The freeze of [26 October](owner-decisions-2026-10-26-tail-then-freeze.md) holds at once for the tutorial and the
  jumbling research: no tutorial text, runner rule, development run, acceptance batch, research round or card starts,
  and the tutorial stays unaccepted. The depth 40 option was not taken; it stays on record.
- Card 9 ends with its record as it stood: T6's steps 1 to 3 passed all three gates of stage 18; T6's step 4 and the
  rule's three stages pass the length and composed gates, and their twist-by-twist replays stopped after 5 of 81
  chunks ([stage 18, section 3.3](../../../research/jumbling/restoration/18-tail-completion/README.md)).
- The development run at depth 100 (D3, seed 7102) stopped at its last checkpoint, 530 bulk steps; its record is
  merged as an interrupted record, not checked. The depth 40 line had already ended (its run 4 stopped `stuck` at the
  tail stop, and its check passes). The scheduled check-ins of both lines are switched off.

## Effect

- The tutorial (`research/jumbling/tutorial/`), the runner and checker
  (`research/jumbling/restoration/13-tutorial-runner/`), the restoration stages and the theory cards stay as merged.
  Reopening any of them is an owner decision.
- The freeze does not cover the jumbling engineering line or the design table, as on 26 October.
