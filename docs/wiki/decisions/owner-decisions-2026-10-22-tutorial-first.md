---
id: owner-decisions-2026-10-22-tutorial-first
type: decision
status: verified
visibility: public
summary: Owner decision of 22 October 2026 - write the jumbling tutorial and carry out its acceptance test first, to the standard of a finished product; this restarts, for that scope, the work the research stop of 20 October halted.
related: [owner-decisions-2026-10-20-research-stop, owner-decisions-2026-10-20-tutorial-acceptance, owner-decisions-2026-10-18-machine-tutorial, owner-decisions-2026-10-17-jumbling-pr]
supersedes: []
claims:
  - {id: tutorial-first, evidence_kind: decision, checked_at: 2026-10-22}
  - {id: tutorial-first-readings, evidence_kind: decision, checked_at: 2026-10-22}
---

# Owner decision, 22 October 2026: the tutorial and its acceptance first

The owner gave this decision in a chat message in Chinese; this page records it in English.

## Decision

Write the tutorial and carry out its acceptance first, to the standard of a finished product.

This answers the question left open by the [research stop](owner-decisions-2026-10-20-research-stop.md): the work
restarts, for the tutorial and its acceptance test.

## Integrator readings (the owner may change them)

1. **Scope.** The restart covers what the tutorial and its [acceptance test](owner-decisions-2026-10-20-tutorial-acceptance.md)
   need: integrating packets P1 to P5 into the tutorial runner (part I1 of the runner-run/3 plan, with the open `major`
   finding P3-falsepass-01 fixed), the development runs, the acceptance batches, the tutorial's text, its macros and
   its illustrations, and the research a batch needs in order to pass. Work the tutorial and the test do not need,
   such as the coverage proofs of B′(2) for their own sake and the Archon formal verification, stays stopped.
2. **Finished standard.** The tutorial states every rule the runner follows, in the runner's order, exactly enough that
   a program reading only the current state can follow it; it meets the content decision of 20 October 2026
   (detailed illustrations; every macro and formula given in full and expandable to single twists; each step a tree of
   named parts with their lengths); every number in it is checked against a record or a script; and every claim keeps
   its evidence status (computed, proved by model review only, run on one instance). The acceptance test runs as
   decided on 20 October 2026, with no relaxation, and the owner signs it off.
3. **Order of work.** I1 first, then the development runs of the runner-run/3 plan (seeds 7001, 7106 and 7101 and the
   3000-twist stops of seeds 7302 to 7307); batch 1 (depths 10, 20, 40 and 100) once the development runs at those
   depths reach the solved state. The text, the macro appendices and the illustrations are written alongside.
4. **Illustrations.** Drawn as SVG by scripts in the repository from exact simulator states of development seeds
   (never a seed of the test set), each with a manifest entry giving the state digest, the word digest, the tool
   versions and the command, and a check that draws it again and compares. Their colours are provisional and decide
   nothing for the design track; G1 to G6 stay with the owner, and Taste Lab's files stay unchanged.

## Not decided

The integrator expects, from the six height-4 stops where the R22D-25 kernel with D19 found no step, that depth 100
and the deeper batches need new rules for height 4 to height 2. If the development runs show this, the integrator
brings the evidence and the options to the owner; limiting the depth of the test would change the owner's decision
of 17 October that scrambles are not restricted, so it is the owner's.

## Effect

- The research stop of 20 October no longer holds for the scope above.
- Draft pull request #112 stays a draft until its merge conditions hold
  ([record](owner-decisions-2026-10-17-jumbling-pr.md)).
