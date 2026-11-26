---
id: owner-decisions-2026-11-26-research-goal
type: decision
status: verified
visibility: public
summary: Owner decisions of 26 November 2026 on the research line and Lean - the jumbling research line's final goal is a proof that the restoration method covers every configuration that can arise, and no scramble test is part of it, so the 21-scramble acceptance of 20 October 2026 is no longer its final criterion; ADR-D02's base target 12c is a product target for users and has nothing to do with the research line; the research stays stopped; a narrow Lean line, for the proofs the 1.0 product's correctness relies on, starts together with LP6.
related: [owner-decisions-2026-10-15-jumbling-restored, owner-decisions-2026-10-17-jumbling-tutorial, owner-decisions-2026-10-20-tutorial-acceptance, owner-decisions-2026-10-27-freeze, owner-decisions-2026-10-27-jumbling-handoff, owner-decisions-2026-11-26-adr-d02-a4, owner-decisions-2026-11-26-log-flow]
supersedes: []
claims:
  - {id: research-goal-coverage-proof, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: research-12c-unrelated, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: lean-with-lp6, evidence_kind: decision, checked_at: 2026-11-26}
---

# Owner decisions, 26 November 2026: the research line's goal and the Lean line

**How the decisions came about.**
- The owner asked whether the integrator still recommends restarting the jumbling research and the Lean work.
- The integrator recommended:
  - a narrow Lean line, for the proofs that the 1.0 product's correctness relies on, run in Claude Code and started
    together with LP6;
  - no separate restart of the research.
- The owner answered on Lean: "Together with LP6".
- The owner then asked why the research line's goal, a general method for any scramble of depth 3000, is so hard. The
  mathematician read the research records and the integrator summarised them.
- The integrator asked whether ADR-D02's base target 12c
  ([26 November 2026](owner-decisions-2026-11-26-adr-d02-a4.md)) is an interim bar while the 21-scramble acceptance of
  [20 October 2026](owner-decisions-2026-10-20-tutorial-acceptance.md) stays the final goal.
- The owner answered in chat (in Chinese), translated: "12c is only D02's target; that test is for the product's users.
  The final goal I set for the research line should never need a scramble test: it is a proof that shows every
  situation that can arise is covered. 12c has nothing to do with the research line."

## Decisions

1. **The research line's final goal is a proof of coverage.**
   - The method must restore every configuration of the jumbled puzzle that can arise, at every depth, depth 3000
     included. "Restored" keeps its meaning of [15 October 2026](owner-decisions-2026-10-15-jumbling-restored.md).
   - The goal is met by a proof that the method covers every such configuration. No scramble test is part of the
     final goal.
   - The 21-scramble acceptance of 20 October 2026 is therefore no longer the research line's final criterion. Scramble
     runs remain experiments: they can guide the work, but they cannot establish the goal.
   - This answer changes how the goal is established, not the form of the method. The tutorial decision of
     [17 October 2026](owner-decisions-2026-10-17-jumbling-tutorial.md) is not revisited here.
2. **12c belongs to ADR-D02 alone.**
   - ADR-D02's base targets, 12c included, are product targets for the program's users.
   - They neither measure nor substitute for the research line's goal.
   - ADR-D02's Track J tests the research surfaces of the product, and it is not a restart of the research line.
3. **The research stays stopped.** The stop of [20 October 2026](owner-decisions-2026-10-20-research-stop.md) and the
   freeze of [27 October 2026](owner-decisions-2026-10-27-freeze.md) stand. Nothing restarts until the owner says so.
4. **A narrow Lean line starts together with LP6.**
   - LP6 starts when the ADR-D03 and K2 lines start ([decision](owner-decisions-2026-11-26-log-flow.md)).
   - Scope: the proofs that the 1.0 product's correctness relies on, following the owner's scope rule of 2 October 2026
     ("proofs only where its correctness relies on them"). Examples:
     - the floor N_c ≥ |In_c − Out_c| that LQ2's base tier uses;
     - the facts behind the repair list of 17 October 2026;
     - base-tier entries that need the mathematician's ruling.
   - It runs in Claude Code, as the Milestone 1 formalization did, and not through Archon, whose harness uses Codex
     (`AGENTS.md`, "Codex minimum").
   - Status today:
     - the base theory's six milestones are complete with no `sorry`;
     - the `Jumbling` library holds two batches of round-22 results (`research/lean/full600-theory/`).

## What the goal asks of the theory (the mathematician's reading, not a ruling)

- **Existence is not enough.** The freeze record of 27 October 2026 already shows that every scramble can be restored.
  Reversal needs the scramble's history. Theorem M\* needs an exhaustive word search with no length bound. Neither is a
  method whose coverage a proof establishes for the state alone.
- **The proof has to say which states can arise.** Suppose a method of finitely many rules, each lowering a computable
  measure, is proved to restore every configuration that can arise. Running it would then decide whether a
  configuration can arise. So a proof of coverage carries one of two things:
  - a description of which off-lattice configurations can arise;
  - a method proved to work on a checkable class that contains all of them.
  The lattice phase was completed through such a description (G = 𝒦). Off the lattice, no abelian invariant can give
  one. This is the reason earlier rule rounds proved termination and never coverage. It does not show that the goal
  cannot be reached.
- **The open items on that path**, as the research data record them:
  - M1, a bounded construction from the state alone;
  - a rule that takes height 4 to height 2;
  - coverage of the tail;
  - coverage at height 2, which includes 112 single-piece cases, MultiCover and MixedCoreBound;
  - stage 19's four remaining obligations.

## What this does not do

- It restarts no research and starts no Lean work.
- It moves no product bar.
- It changes neither ADR-D02 nor its base targets.
