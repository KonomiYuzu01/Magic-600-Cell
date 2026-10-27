---
id: owner-decisions-2026-10-26-tail-then-freeze
type: decision
status: verified
visibility: public
summary: Owner decision of 26 October 2026 - once stage 17 is merged, the next card of the jumbling theorem programme is the tail completion; when that card ends, the jumbling tutorial and the jumbling research are frozen.
related: [owner-decisions-2026-10-25-jams-first, owner-decisions-2026-10-24-theorem-program, owner-decisions-2026-10-23-depth, owner-decisions-2026-10-22-tutorial-first]
supersedes: []
claims:
  - {id: tail-card-next, evidence_kind: decision, checked_at: 2026-10-26}
  - {id: freeze-after-tail, evidence_kind: decision, checked_at: 2026-10-26}
---

# Owner decision, 26 October 2026: the tail completion, then a freeze

The owner gave this decision in a chat message in Chinese; this page records it in English.

## Evidence put to the owner

The integrator's status report (chat, 26 October 2026), headless on Linux:

- The tutorial's text and figures are written and merged (pull requests #112 and #129). None of the 21 acceptance
  scrambles has run: under the [22 October decision](owner-decisions-2026-10-22-tutorial-first.md) the development runs
  at depths 10, 20, 40 and 100 must reach the solved state first.
- The development runs (`research/jumbling/restoration/13-tutorial-runner/runs/dev/`): depths 10 and 20 solved and
  checked; depth 40 (seed 7101), run 4, stopped `stuck` in the tail with 158 pieces off, all at height 2, where no
  tail rule and no fallback word applies, and its check passes; depth 100 (seed 7102) still in the bulk after 511
  steps, from 115,566 to 14,528 pieces off (91 at height 4); the six depth 3000 runs stopped in the height phase.
- The theorem programme for height 4 (cards 6 to 8, restoration stages 14 to 16, and stage 17 in pull request #137):
  at the six D2 height ends the depth Φ is down from 1,468 to 3,910 to between 386 and 1,158 (65% to 86% fewer pieces
  at height 4), and zero at none; where P*** stops, releases abound but no tail of its forms descends. The tail
  completion and the bulk cards had not started.
- The integrator recommended opening the tail completion card next, in the recorded order, because the depth 40 run
  is stuck in the tail and the first acceptance batches need it, and keeping the height descent (wider tails) as a
  lead.

## Decision

Once stage 17 is merged, the tail completion card opens first. When it ends, the tutorial and the jumbling research
are frozen.

## Integrator readings (the owner may change them)

- The tail completion card is the programme's next card ("Tail completion: shared cells, vacancies and locks"),
  worked like cards 6 to 8: the card with the facts at its stops, sealed analyses by Astra and Claude Opus, a
  cross-examination, exact experiments and a stage record. Its first evidence is the depth 40 tail stop.
- The card ends when its record (stage README, card status, indexes, wiki log and progress) is merged.
- The freeze covers the tutorial (its text, runner, checker, acceptance test and development runs) and the jumbling
  research (the restoration stages and theory cards, including the height descent; the bulk card does not open). After
  it no research round, run, card or development run starts, runs still in flight stop at their last checkpoint, and
  the records stay as they are. The tutorial stays unaccepted and its acceptance test does not run.
- The freeze does not cover the jumbling engineering line (EN3, EN4 and the Jumble feature's path to the 2.5 freeze,
  option B), which its own decisions govern, or the design-table topics, which are on hold under the
  [22 October decision](owner-decisions-2026-10-22-dt-m1.md).

## Effect

- The order of the programme becomes: card 6, card 7, the six jams (card 8, with stage 17), the tail completion. The
  bulk card is not opened.
- The joint-attack protocol of [24 October](owner-decisions-2026-10-24-theorem-program.md) is unchanged for the tail
  card; the work uses subscription quota only.
