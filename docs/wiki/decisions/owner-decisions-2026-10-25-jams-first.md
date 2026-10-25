---
id: owner-decisions-2026-10-25-jams-first
type: decision
status: verified
visibility: public
summary: Owner decision of 25 October 2026 - the next card of the jumbling theorem programme attacks the six configurations where two-level P* stops for good (a descent outside its families, or a negative certificate with excursion for a stated alphabet), before the tail completion and the bulk.
related: [owner-decisions-2026-10-24-theorem-program]
supersedes: []
claims:
  - {id: jams-first, evidence_kind: decision, checked_at: 2026-10-25}
---

# Owner decision, 25 October 2026: the six jams before the tail

The owner gave this decision in a chat message in Chinese; this page records it in English.

## Evidence put to the owner

The integrator's report on card 7 (chat, 25 October 2026; pull request #135,
[stage 15](../../../research/jumbling/restoration/15-lowering-machine/README.md)), headless on Linux:

- A certified greedy procedure over five families of macros (two-level P*) lowers the depth Φ at all six chain ends of
  the D2 height phase and then stops for good at every one of them, at Φ 498, 936, 1172, 1572, 1490 and 1220. There no
  macro of its families and no single letter lowers Φ, so Question 4 of card 7 is answered negatively for those
  families.
- The programme's order was: the blocking structure (card 6), the lowering machine and placement (card 7), the tail
  completion, the bulk. The integrator recommended attacking the six jams first, because from them P* does not reach
  the tail at all: find a descent outside the families (longer retained releases, more helper letters, larger
  excursions), or prove a negative certificate with excursion (stage 15, Proposition BN) for a stated alphabet.

## Decision

The next card attacks the six jams, as recommended. The tail completion and the bulk follow.

## Effect

- The order of the programme becomes: card 6, card 7, the six jams (card 8), the tail completion, the bulk.
- The joint-attack protocol of [24 October](owner-decisions-2026-10-24-theorem-program.md) is unchanged: sealed
  analyses by Astra and Claude Opus, a cross-examination, then exact experiments; the work uses subscription quota only.
