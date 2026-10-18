---
id: owner-decisions-2026-10-18-bprime-goal
type: decision
status: verified
visibility: public
summary: Owner decision of 18 October 2026 - a single step of a few hundred twists is executable by a person, so the goal of conjecture B′(h) becomes a proof of existence with explicit bounds that need not be small, not the shortest words; the theory research starts from the stuck units of the zero-Bad census and run 2, on four parallel lines with regular senior-review rounds.
related: [owner-decisions-2026-10-17-jumbling-tutorial, owner-decisions-2026-10-17-jumbling-pr, owner-decisions-2026-10-17-jumbling-m2, owner-decisions-2026-10-17-repair-pairs, owner-decisions-2026-10-15-jumbling-restored, owner-decisions-2026-10-13-jumbling-menu]
supersedes: []
claims:
  - {id: bprime-existence-goal, evidence_kind: decision, checked_at: 2026-10-18}
  - {id: bprime-research-lines, evidence_kind: decision, checked_at: 2026-10-18}
---

# Owner decision, 18 October 2026: the goal of B′(h) is existence with explicit bounds

The owner gave this decision in a chat message in Chinese; this page records it in English. Conjecture B′(h) is
stated in [stage 12](../../../research/jumbling/restoration/12-low-height/README.md), section 1.

## Decision

1. **Goal.** A single step of a few hundred twists is executable by a person. The goal of B′(h) is therefore to prove
   that the catalogue and the bounds exist and to state the bounds L_h(n) and S_h(n) explicitly. The bounds need not
   be small, and the shortest words are not sought.
2. **Starting point.** The research first sorts the units that stay stuck, by kind and by where they stick, from the
   zero-Bad transport census and the second bounded driver run (E-B5 run 2), and only then starts the theory. It does
   not wait for run 2 to finish.
3. **Research lines**, worked in parallel:
   - (a) a zero-Bad transport lemma: under which conditions on vacant points a block reaches a wholly vacant home cell
     by clean powers of commutators, and what to do when pieces of other blocks partly occupy that cell;
   - (b) the existence of the vacating word V with an explicit length bound, written as a certificate from the
     regularities of the words the greedy search finds;
   - (c) whether the T2 partner catalogue suffices, and how the units that T3 blocks are unlocked (straddlers,
     foreign-frame occupants, M1-19), as a joint attack of the senior reviewer and the solver: two sealed analyses,
     exchanged, cross-examined, and decided by an experiment;
   - (d) T4, height 4: retained conjugation, six-letter commutators and two protected lowering segments.
4. **Senior-review rounds.** After each batch of experimental results, a research or review round of the senior
   reviewer runs, one shard per line, several lines at once. Plan checks and reviews use the fast tier; joint attacks
   and escalations use the standard tier at the highest effort. A recurring check in the integrator's session looks
   every few hours for new material for the senior reviewer and starts a round when there is some. Only subscription
   quota is used: no API-team run and no new cost.
5. **Records.** Each round's conclusions go into stage 12 and the [senior review
   record](../../../research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md); proofs are written so that they
   can later be handed to the Lean agent Archon ([record](owner-decisions-2026-10-17-jumbling-m2.md)). Everything is
   pushed to the branch of draft pull request #112, which stays a draft; no new pull request is opened
   ([record](owner-decisions-2026-10-17-jumbling-pr.md)).

## Effect

- **B′(h) itself is unchanged.** Its statement already asks for an explicit catalogue and explicit bounds; the decision
  sets what counts as success: an existence proof with explicit bounds that need not be small. The proof does not need
  the shortest words, and words found by search are not shortened for their own sake; the size criterion of
  [14 October](owner-decisions-2026-10-13-jumbling-menu.md), lengths that are not astronomical, still applies.
- **Unchanged.** Headless Linux checks only; the Taste Lab files and `taste-lab-plan.md` are not touched;
  `status.json` is not edited by hand and critical-path files are not changed; the human-solve boundary, with the
  [repair-list exception](owner-decisions-2026-10-17-repair-pairs.md), is unchanged; E2 and the Lean formalization
  wait for the owner's decision.

## Where it is applied

- [Stage 12](../../../research/jumbling/restoration/12-low-height/README.md): section 1 (the goal) and the sections of
  the research lines as they are written.
