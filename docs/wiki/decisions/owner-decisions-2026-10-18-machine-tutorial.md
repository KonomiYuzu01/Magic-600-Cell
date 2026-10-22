---
id: owner-decisions-2026-10-18-machine-tutorial
type: decision
status: verified
visibility: public
summary: Owner decision of 18 October 2026 - the jumbling tutorial is accepted when a machine that follows it step by step takes any scramble to the solved state, playing only literal words; the owner then makes it workable for a person, with a person's complexity limits in view. The general form of gap M1 is not needed for this target. Coverage round 9 is held, the senior reviewer looks for literal words for the nine certificate steps of E2, and height 4 comes before coverage.
related: [owner-decisions-2026-10-17-jumbling-tutorial, owner-decisions-2026-10-18-bprime-goal, owner-decisions-2026-10-17-jumbling-m2, owner-decisions-2026-10-17-jumbling-pr, owner-decisions-2026-10-10-jumbling-complete]
supersedes: []
claims:
  - {id: machine-followable-tutorial, evidence_kind: decision, checked_at: 2026-10-18}
  - {id: person-version-by-owner, evidence_kind: decision, checked_at: 2026-10-18}
  - {id: literal-steps-only, evidence_kind: decision, checked_at: 2026-10-18}
  - {id: work-order-2026-10-18, evidence_kind: decision, checked_at: 2026-10-18}
---

# Owner decision, 18 October 2026: a tutorial that a machine can follow to the end

The owner gave this decision in chat messages in Chinese; this page records it in English.

## Context

The owner asked whether a tutorial through which a machine restores every scramble needs the general form of gap M1,
and if not, which research route reaches that goal fastest. The integrator's answer:
- The general form of M1, a bound computable from the state for one clean step at every reachable state, exists
  exactly when reachability is decidable (Theorem D, round 21,
  [SENIOR-REVIEW](../../../research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md) section 26). A procedure
  that restores every reachable configuration within a bound computable in advance would decide reachability as
  well. No route to either is known.
- A machine that always succeeds exists already in principle: heuristic steps, the fair exhaustive fallback and the
  lattice phase form a complete method that reads only the state (Theorem M\*,
  [METHOD](../../../research/jumbling/restoration/09-general-method/METHOD.md)), but the fallback is practical only for
  very short words. What a tutorial needs is coverage (gap E1): at every state the machine actually meets, the
  tutorial's catalogue has a step, so the fallback never fires.
- Where the work stands: the bulk descent of the scramble census takes the 10- and 20-twist scrambles and one of two
  40-twist scrambles to the lattice, and runs into a slow tail on the other 40-twist scramble and on every deeper one;
  experiment E2 took the depth 40 stop from 1,193 off pieces to 0, with the unit, the kind of step and the order chosen
  by hand; the lattice phase has a bound of 20,866,844 twists for every input (proved, model review) and runs as its
  own program.

In the same exchange the owner noted that lengths like 10^500 are far beyond what a person can carry out, and asked
what happens above left height 4. The integrator's answers:
- Of the 21 steps of E2, twelve are literal words (8,804 letters in all); the other nine are known only by
  certificates of their length (147 to 164 digits). A certificate proves that a step exists; it is not a word that a
  machine or a person can play.
- Random scrambles of up to 1000 twists reached at most left height 6, and the bulk descent cleared height 6; the
  residues lie at heights 2 and 4 ([stage 11](../../../research/jumbling/restoration/11-scramble-census/README.md)).
  Every census stop from depth 100 on keeps pieces at height 4 (111 to 607 of them).
- Constructed states reach every height: the left heights of reachable states are unbounded
  ([stage 10](../../../research/jumbling/restoration/10-x1-unbounded/README.md)). One twist changes a left height by at
  most 2, so a state of height h needs at least h/2 twists, and no uniform bound on a restoration or on one clean step
  exists. The endpoints of the stage 10 family restore in at most h/2 + 141 letters, by recognizing the family
  (R21-09).

## Decision

1. **Target.** The tutorial is accepted when a machine that follows it step by step, using only the rules it teaches,
   takes a scramble to the solved state.
2. **People afterwards.** Once a machine can follow the tutorial to the end, the owner works out how a person can
   carry it out, keeping in view the limits of what a person can do (the complexity bounds of each step).
3. **Literal steps only.** The machine plays only literal words, twist by twist. A step that exists only as a
   certificate counts as stuck, and every step's length is recorded as it is.
4. **Order of work.** Round 9 of the coverage of B′(2) is held. The senior reviewer looks for short literal words for
   the nine certificate steps of E2, from short seeds (the commutator-power search of R22B-18) and the structure of
   the steps. Height 4 (line (d)) comes before coverage.

## Effect

- The route the integrator proposed, which the owner's replies left unchanged apart from items 3 and 4:
  - a tutorial runner: one program that reads only the state and applies only the tutorial's rules, in order the bulk
    descent of the scramble census, the tail steps of E2 with the choices made there by hand turned into rules read
    from the state, and the lattice phase; it keeps the exhaustive fallback and records whether it fires;
  - for every step the runner records the rule that fired, the word and its length, the candidates checked before
    the step was found and the state before and after, which is what judging a person's complexity needs;
  - runs on the census stops and on fresh seeds at every depth, each run replayed exactly; the theory follows the
    states where the runner stops, and conjecture B′(h) is aimed at those states.
- Coverage of B′(2) (the C line) leaves the critical path; its round 9 waits.
- The search of R22B-18, deferred because short seeds only shrink the bound of V, is no longer deferred: short seeds
  are what turn a certificate step into a word.
- The general form of M1 is not pursued, as decided for option D on
  [17 October](owner-decisions-2026-10-17-jumbling-tutorial.md).
- **Unchanged.** Scrambles are not restricted ([17 October](owner-decisions-2026-10-17-jumbling-tutorial.md)); the
  goal of B′(h) is existence with explicit bounds ([18 October](owner-decisions-2026-10-18-bprime-goal.md)); the
  human-solve boundary of the product, with the [repair-list exception](owner-decisions-2026-10-17-repair-pairs.md);
  the work collects in draft pull request #112 ([record](owner-decisions-2026-10-17-jumbling-pr.md)); headless Linux
  checks only, and subscription quota only.
