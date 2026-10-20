---
id: owner-decisions-2026-10-20-tutorial-acceptance
type: decision
status: verified
visibility: public
summary: Owner decision of 20 October 2026 - the machine-followable jumbling tutorial is tested on 21 scrambles that development never used, three fresh seeds at each of seven depths from 10 to 3000 twists, fixed in advance and run in batches from shallow to deep; only the tutorial runner plays, every word twist by twist, and an independent checker replays every record.
related: [owner-decisions-2026-10-18-machine-tutorial, owner-decisions-2026-10-17-jumbling-pr, owner-decisions-2026-10-15-jumbling-restored]
supersedes: []
claims:
  - {id: tutorial-acceptance-test-set, evidence_kind: decision, checked_at: 2026-10-20}
  - {id: tutorial-acceptance-batches, evidence_kind: decision, checked_at: 2026-10-20}
  - {id: tutorial-acceptance-pass, evidence_kind: decision, checked_at: 2026-10-20}
---

# Owner decision, 20 October 2026: the acceptance test of the machine-followable tutorial

The owner gave this decision in a chat message in Chinese, accepting the integrator's proposal; this page records it
in English. It says how the target of [18 October](owner-decisions-2026-10-18-machine-tutorial.md) is tested: the
tutorial is accepted when a machine that follows it step by step, using only its rules and playing only literal words,
takes a scramble to the solved state.

## Context

The owner asked whether height 4 to height 2 is the only link left, and how the acceptance would be done. The
integrator's answer:
- Height 4 to height 2 is the front of the theory, not the only gap. The tutorial runner
  ([stage 13](../../../research/jumbling/restoration/13-tutorial-runner/README.md)) has taken two scrambles to the
  solved state, depth 10 (seed 7001) and depth 20 (seed 7106), with the lattice phase checked by composition rather
  than played. The depth 40 scramble of seed 7101 reached the solved state by literal words, but with the units and
  the kinds of step chosen by hand (stage 12, R22B-34).
- The gaps: no rule that always takes height 4 to height 2 (line (d)); a bulk phase that is slow at depth (the
  3000-twist scrambles of seeds 7304, 7306 and 7307 kept 95,569 to 108,823 pieces off after census descents with a
  budget of 5.25 hours, [stage 11](../../../research/jumbling/restoration/11-scramble-census/README.md)); rules the
  runner does not call yet (the height-6 rules R22D-75 and R22D-81, the R22D-25 kernel with the height-4 rule D19, a
  rule for preparations like the three letters before E2's b06 step); certificate steps tested at two states only;
  coverage at height 2, whose round 9 is held; and a lattice phase that the runner does not play twist by twist (the
  literal player exists and is not called).
- A finite test cannot show that every scramble is restored, and no uniform bound on a restoration exists
  ([stage 10](../../../research/jumbling/restoration/10-x1-unbounded/README.md)). The test is therefore a set of
  scrambles fixed in advance that development never used, together with a statement of what the theory covers.

## Decision

1. **Test set.** 21 scrambles: three fresh seeds at each of the depths 10, 20, 40, 100, 300, 1000 and 3000 twists,
   made as the census makes them (`restore.scramble` of `theory/restore.py` with the menu I_a; `runner.py --scramble
   <k> <seed>`). The seeds are 9000 + 10·i + j, with i = 1 to 7 for the depths in that order and j = 1, 2, 3: 9011 to
   9013 at depth 10, 9021 to 9023 at depth 20, 9031 to 9033 at depth 40, 9041 to 9043 at depth 100, 9051 to 9053 at
   depth 300, 9061 to 9063 at depth 1000 and 9071 to 9073 at depth 3000. The census and the research so far used the
   seeds 7001 to 7109 and 7301 to 7307. No one runs a seed of the test set before its batch.
2. **Batches, shallow to deep.** Depths 10, 20, 40 and 100 first, then 300, then 1000, then 3000.
3. **The run.** Only the tutorial runner plays: it reads only the current state and applies only the tutorial's
   rules, from the scramble to the solved state. Every word is played literally, twist by twist, the lattice phase
   included. A step known only by a certificate counts as stuck, and a step of the exhaustive fallback counts as a
   failure of coverage.
4. **Record and check.** Every step records the rule that fired, the word, its length and the digests before and
   after. An independent checker that shares no code with the runner replays every record twist by twist.
5. **Pass.** A batch passes when every one of its scrambles reaches the solved state and the checker passes every
   record. The results, with every step's length, go into stage 13, and the owner signs the acceptance off.

Defaults the integrator applies, which the owner may change:
- A scramble that fails becomes a development input; the next attempt at its batch replaces it with the next seed of
  its depth (j = 4, 5 and so on), and the failed run stays on record.
- The tutorial is accepted only when every batch has passed with the same runner and rules; a batch passed with an
  earlier version is run again with the final one.
- No time limit is set; every run's time is recorded.

## Effect

- The order of work before the first batch: connect the existing rules to the runner and run it on the development
  inputs (seed 7101 and the 3000-twist scrambles of seeds 7302 to 7307) until it stops; line (d) round 7 on height 4
  to height 2; the speed of the bulk phase at depth.
- The checker plays the lattice phase twist by twist; today `check_run.py` checks it by composition through
  `verify_late.py` (stage 13, section 6).
- None of the 21 scrambles has been run.
