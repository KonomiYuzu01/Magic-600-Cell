---
id: owner-decisions-2026-10-17-jumbling-tutorial
type: decision
status: verified
visibility: public
summary: Owner decision of 17 October 2026 - the most important output of the jumbling restoration research is a tutorial that people can read and carry out; its scrambles are not restricted; scramble experiments come first, and the owner chooses the research option from their results. After the first experiments the owner chose, provisionally, a measured coverage census with a technique manual and a completeness theorem for low heights.
related: [owner-decisions-2026-10-17-jumbling-m2, owner-decisions-2026-10-10-jumbling-complete, owner-decisions-2026-10-15-jumbling-restored]
supersedes: []
claims:
  - {id: tutorial-first, evidence_kind: decision, checked_at: 2026-10-17}
  - {id: no-restricted-scrambles, evidence_kind: decision, checked_at: 2026-10-17}
  - {id: experiments-before-options, evidence_kind: decision, checked_at: 2026-10-17}
  - {id: options-a-and-b, evidence_kind: decision, checked_at: 2026-10-17}
---

# Owner decision, 17 October 2026: a tutorial first, from unrestricted scrambles

The owner gave this decision in chat messages in Chinese; this page records it in English.

## Decisions

1. **A tutorial is the most important output.** The restoration research is to produce a tutorial that people can
   read and carry out. The integrator assessed the research directions against that goal and proposed options: a
   measured coverage census of the existing method and a manual of recognizable techniques; a completeness theorem
   for states of low height; scrambles restricted to what the method covers; and further work on the strong form of
   gap M1.
2. **Scrambles are not restricted.** The owner rejected the option of limiting the scrambles, for example a
   tutorial mode whose scrambles stay inside a class that the method is known to cover. The tutorial has to work
   from the product's full random scrambles, in line with the decision of 10 October 2026 that restoration must
   cover every reachable configuration ([record](owner-decisions-2026-10-10-jumbling-complete.md)).
3. **Experiments first.** The integrator first runs scramble experiments: the existing state-only procedure on
   fresh random scrambles of increasing depth, recording how far it gets, where it stops and what is left. The
   owner then chooses the research option from the results.
4. **Options A and B, provisionally.** After the first experiments the owner chose, for now and open to change:
   - option A, the measured coverage census of the method on random scrambles and a manual of the techniques it
     uses;
   - option B, a completeness theorem for low heights, aimed at what the census leaves behind: pieces at left
     height 2 and 4, in blocks that hold one another up.

   Further work on the strong form of gap M1 is not pursued for now. The integrator's assessment and the first
   results are in the [scramble census](../../../research/jumbling/restoration/11-scramble-census/README.md).

## Effect

- **Round 21.** The joint attack on gap M1 is recorded as it stands
  ([SENIOR-REVIEW](../../../research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md) section 26). Its
  Theorem D, under which a computable bound on one clean step valid at every reachable state exists exactly when
  reachability is decidable, is part of the assessment.
- **Lean handoff.** Pending the owner's choice, the plan of 17 October 2026 for the Lean formalization (after M1 is
  solved) is unchanged ([decision](owner-decisions-2026-10-17-jumbling-m2.md)).
- **Unchanged.** The definition of restored and the research priority of 15 October 2026, the product's human-solve
  boundary of 2 October 2026, and the rule that project work runs on subscription quota.
