---
id: owner-decisions-2026-10-10-jumbling-deferred
type: decision
status: verified
visibility: public
summary: Owner decision of 10 October 2026 - jumbling theory problems the integrator judges out of reach for now are not pursued (state count and worst-case distance, a finiteness proof, a proved bound on lowering words, a restoration rule that never needs a trial twist); a working restoration method has top priority.
related: [owner-decisions-2026-10-09-jumbling]
supersedes: []
claims:
  - {id: deferred-out-of-reach, evidence_kind: decision, checked_at: 2026-10-10}
  - {id: restoration-method-first, evidence_kind: decision, checked_at: 2026-10-10}
---

# Owner decision, 10 October 2026: deferred jumbling problems

The owner gave this decision in a chat message in Chinese; this page records it in English. The problems are those of the [theory draft](../../../research/jumbling/theory/theory-draft.md) (items 3 to 7) and of [restoration](../../../research/jumbling/theory/restoration.md) (section 5).

## Decision

1. **Problems judged out of reach are not pursued for now.** The integrator listed the open jumbling problems with a judgement on each; the owner accepted that those judged out of reach are set aside and recorded here.
2. **A working restoration method has top priority.** The owner needs a restoration method the integrator believes works on deep jumbles: human-executable, reading only the visible configuration, and complete.

## Deferred problems

Each entry gives the reason and what would reopen it.

| Problem | Source | Why it is out of reach now | Reopens when |
| --- | --- | --- | --- |
| The number of reachable configurations of R(S4₀), R(I_a), R(I_b), and worst-case distances (a "God's number") | theory draft item 7 | Not known even for the retained 600-cell puzzle; jumbling only enlarges the state space. | A method for the retained puzzle appears. |
| A proof that R(S4₀), R(I_a) or R(I_b) is finite | theory draft item 3 | By the tree model (restoration T2 to T5) finiteness amounts to the tree heights staying bounded on reachable configurations; a proof needs a global argument that blocking caps the heights across all 600 caps, and none is in sight. The data show small heights only (largest (2, 4) and (3, 1) under S4₀ after 640 twists). | A concrete mechanism that bounds heights is found. |
| A proved general bound on the length of the shortest N-lowering word | restoration section 5, open problem 1 | Along the true scramble paths the shortest lowering prefix reaches 36 (an upper bound) under S4₀ at 640 twists; no structure that bounds it is known. Theorem U already makes phase U complete without the bound. | Never needed for completeness; only for a worst-case time statement. |
| A restoration rule that reads the configuration and never needs a trial twist | restoration section 3 | A configuration does not determine the scramble that made it, and symmetric twists leave seams that a later twist did not disturb: under I_a at 40 twists one cap's seam passes every visible top-twist test and is not the top of the scramble. A local visible rule is fallible, so the method keeps trial twists and a return to the solver's own bookmark. | A visible test is proved exact. |

## Still pursued

- The restoration method (top priority): reading the last twist from its seam, with trial twists and bookmarks where the reading is ambiguous.
- At lower priority, when spare compute allows: a search for a repeatable admissible word that raises tree heights, which would prove a menu infinite; statement (b) of restoration section 4 and the eight extra lattice states of theory draft item 4; block preservation through jumble twists (item 6).
- Statement (a) of restoration section 4 (the retained group equals the configurations with the Lemma 8 invariants) is large but not out of reach; it is a separate task for the retained puzzle.
