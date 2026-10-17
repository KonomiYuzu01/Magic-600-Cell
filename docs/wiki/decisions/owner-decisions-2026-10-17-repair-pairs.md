---
id: owner-decisions-2026-10-17-repair-pairs
type: decision
status: verified
visibility: public
summary: Owner decision of 17 October 2026 - the human-solve boundary for restoring the jumbled puzzle is settled - at a cap, the program may list the pairs (x, d) whose three-twist repair lowers N, and the person picks one; measured, 1 to 11 such pairs per tail step.
related: [owner-decisions-2026-10-02, owner-decisions-2026-10-17-jumbling-tutorial, owner-decisions-2026-10-09-jumbling]
supersedes: []
claims:
  - {id: repair-pair-list, evidence_kind: decision, checked_at: 2026-10-17}
  - {id: person-picks, evidence_kind: decision, checked_at: 2026-10-17}
---

# Owner decision, 17 October 2026: the program may list the repairs that lower N at a cap

The owner gave this decision in a chat message in Chinese; this page records it in English.

## Decision

The human-solve boundary for restoring the jumbled puzzle is settled: the program may list the pairs (x, d) at a
cap that lower N. Measured, there are 1 to 11 of them per step, and the person picks one.

## Context

The descent of the [technique manual](../../../research/jumbling/manual/02-descent.md) has three kinds of step. The
third, the three-twist repair (manual section 2.3), opens a cap c with a twist x, makes the best twist y of a
misaligned neighbour d, and then the best twist z of c. The integrator measured how many proposals a person would
make at each step of the census record of seed 7101 (depth 40, 97 steps;
[census](../../../research/jumbling/restoration/11-scramble-census/README.md), "What a person would have to try"):
- from step 44 on, the census rule made a median of about 110 openings and considered about 800 pairs of opening
  and neighbour before a repair lowered N;
- at the cap where the repair was found, 1 to 11 of the 472 to 1,298 pairs lowered N, at the 15 tail steps measured;
- nothing the program showed marked those pairs.

Under the boundary of [2 October 2026](owner-decisions-2026-10-02.md), no view listed the twists that lower N
([observation requirements](../../progress/1.0/jumbling-observation.md), rule 6). The integrator put three options
to the owner:
- keep the boundary;
- a search on a target the solver names;
- research into visible signals without changing the boundary.

The owner chose the list of lowering pairs at a cap.

## How the integrator reads it

The owner may correct these readings; until then the requirements and the manual follow them.
- **The pairs** are those of the three-twist repair: an opening x of the cap c, the 59 twists of its menu other than
  the identity, and a misaligned cap d that meets c. A pair is listed when its repair, with y the best twist of d
  after x and z the best twist of c after x and y, lowers N. This is how the measurement counted them.
- **The cap is the solver's.** The solver names the cap; the program lists the pairs at that cap and does not
  choose the cap. An empty list says that no pair at that cap lowers N.
- **Each pair comes with its y and z,** since a pair lowers N only through them, and the solver picks one pair and
  makes its three twists.
- **No ranking.** The list is in a neutral order (by x in menu order, then by d) and marks no pair as best. The
  program never makes a twist without the solver's action.
- **Scope.** The decision covers the three-twist repair of the jumbled puzzle's descent. Elsewhere the human-solve
  boundary of 2 October 2026 is unchanged: for the face match and the cover step, and for the 600-cell-Full puzzle,
  no view lists helpful twists, ranks caps or marks a next move.

## Effect

- **Observation requirements.** Rule 6 of [jumbling-observation](../../progress/1.0/jumbling-observation.md) takes
  this exception, and the U phase lists the repair list among what the program shows.
- **Manual.** Section 2.3 gives the procedure with the list, and section 2.6 and the claims table state what was
  measured: naming caps by decreasing N_c, the cap with a lowering pair was the first or second named at 41 of the
  63 repair steps of the record, and at most the 28th.
- **Measurement.** The list was measured at 15 of the 63 repair steps of the record. Measuring it at all of them is
  the next item of option A.
- **Briefing.** The briefing's jumbling line still says that the human-solve boundary applies unchanged. Changing a
  rule file needs the owner's explicit authorization, which has been asked for.
