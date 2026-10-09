# Jumbling study of the 600-cell-Full puzzle

Status: **exploratory computation with one senior plan check and one scoped re-check (9 October 2026). All findings adopted; the Q5 revision awaits the second and last scoped re-check.** No model identity, product scope or rule changes here.

Owner decision, 9 October 2026: **no fudged variant is pursued.** The study covers rigid, unfudged jumbling of the retained cut geometry only. `state-contract.md` defines that model.

## Question

The owner asked whether the facet-turning (cell-cap) 600-cell can be a jumbling puzzle, in the sense of the Hypercubing wiki's [formal grip theory](https://hypercubing.xyz/theory/grip-theory/formal/#jumbling) and HactarCE's [Jambler](https://github.com/HactarCE/Jambler). Jumbling here means extra twists of a cap by rotations about the cell axis that are not cap symmetries. This page records what the retained model (cut depth α = 121/125, `assets/model.npz`) says about that question. The external definitions are summarized from the linked pages; no primary-source text is captured in this repository.

Terms follow `research/theory/RETHLAS_BLUEPRINT.md`:
- pole n_c, cap U_c, complete cap signature M(A), the cap's A4 rotation group, K⁺ and K;
- a **jumble twist** about pole c is a rotation in SO(3)_c, the rotations of four-space fixing n_c, that is not in A4.

## Reproduce

```text
python research/jumbling/jumble_study.py
```

The script needs the standard library and NumPy only, takes about 80 s, reads `assets/model.npz` read-only and writes `research/jumbling/results.json`. `h4.py` builds the 600-cell from the 120 unit quaternions of the binary icosahedral group, independently of the model. The script aligns the two coordinate systems by an orthogonal map and checks that all 600 poles match.

## Computed results

Evidence kind: synthetic geometry and model data only. Nothing here is Windows, rendering or performance evidence.

1. **Interacting poles.**
   - The signatures of the 3,097 pieces in one cap use exactly 57 poles: c and 56 others in five shells at 15.522°, 25.243°, 36°, 41.410° and 44.478° from n_c (4, 12, 24, 12 and 4 poles). These are the cells that share at least one vertex with c.
   - For every depth strictly between 3/(2φ) ≈ 0.92705098 and 1 (fraction of the facet distance), a vertex-hull calculation over every pole shows that the same 56 poles interact.
   - At 3/(2φ) and below, the 24 poles at 49.118° also interact. Deeper shells can mix thresholds; at 60° they range from 0.88197 to 0.89443.
2. **Discrete jumble twists.**
   - The script searches for rotations in SO(3)_c outside A4 that carry at least two independent interacting poles exactly onto poles of the same shell. It found 1,224, with the first pole restricted to the six A4-orbit representatives, in 33 classes up to A4 × A4.
   - The strongest realigns 24 poles (90°, all of the 36° shell) or 20 poles (44.48°).
   - Every class leaves some interacting pole between 16° and 48° from the nearest pole of its shell, against a nearest-pole spacing of 15.52°.
3. **Snapping of one representative per class.**
   - With nearest candidates taken up to geometric ties, no representative has a perfect matching onto the 57 interacting poles; the largest matching has 49.
   - Onto all 600 poles, one class (63.43°) admits 57 distinct nearest targets, but they are not the original 57.
   - The argmin signature counts in `results.json` depend on floating-point tie-breaking and describe this implementation only.
4. **Fixed-slot automorphisms.**
   - An exhaustive individualisation-refinement search finds exactly 24 permutations of the 57 poles that fix c and map the set of 3,097 cap signatures onto itself.
   - They are exactly the 24 geometric symmetries of the cell's neighbourhood: the 12 rotations of A4 and 12 reflections.
5. **Sampled near-symmetry profile.**
   - For 200,000 random rotations (seed 20261009), the smallest sampled worst-case same-shell landing error at distance at least d from A4 is 3.1° (d = 5°), 6.5° (10°), 9.4° (15°) and 18.7° (30°).
   - These are sampled upper bounds on the optimum, not lower bounds. They use same-shell targets, so they are not directly comparable with the Sun Cube baseline (9.7° error over F, E and V grips at 35.3° spacing).

## Claims after the plan check

- **Fixed-slot theorem (narrowed from the first draft).** Suppose the cap's set of signatures must be preserved, that is, no piece may take a signature outside the retained set. Then every fudged jumble twist about a cell is one of the 24 symmetries in result 4.
  - This does not prove that no fudging exists in general. In an abstract grip-set extension, a permutation applied only to the cap's signatures is invertible and keeps signatures distinct while leaving the retained set (finding C4-Q1).
  - The 12 reflections are not rigid twists.
  - The owner's decision above makes the broader fudging question out of scope.
- **Unfudged jumbling is infinite.**
  - K⁺ = (I* × I*)/⟨(−1, −1)⟩ is a maximal finite subgroup of SO(4). Any jumble twist therefore generates an infinite group with K⁺, and the spanning set of poles has an infinite orbit. This is a statement about grip closure, not about reachable configurations.
  - Rotations that realign one pole pair form a circle in SO(3)_c. These circles are sufficient for unblocking that grip, but not the whole unblocking locus: a cut that misses the moved region entirely gives open sets of admissible rotations (finding C5-Q2).

## Plan-check dispositions (call 20261009T161321Z-7c78d2bf)

| Finding | Severity | Reply | Change |
| --- | --- | --- | --- |
| C1 | minor | adopt | Exact threshold 3/(2φ), every shell member evaluated, open interval stated |
| C3 | minor | adopt | Tie-aware matching added; argmin counts labelled as implementation-specific |
| C4-Q1 | major | adopt | Claim narrowed to the fixed-slot theorem; general no-fudging claim withdrawn; fudging out of scope by owner decision |
| C5-Q2 | minor | adopt | Circles stated as sufficient, not complete; grip closure separated from reachability |
| C6 | minor | adopt | Profile labelled as sampled upper bounds with same-shell targets |
| Q3 | minor | adopt | `state-contract.md` section 4: reachable configurations and first-order legality strata |
| Q4 | major | adopt | `state-contract.md` section 5: handoff only from a witnessed retained-state checkpoint |
| Q5 | major | adopt | Plan reordered as below; contract first, one minimal witness, impact assessment, then stop |

Scoped re-check 20261009T163617Z-bc992d63 found C4-Q1 and Q4 resolved. It also found that Q5 remained open, because the contract's tie band accepted shallow crossings of up to 1e-9.

| Finding | Severity | Reply | Change |
| --- | --- | --- | --- |
| Q5 (re-check) | major | adopt | `state-contract.md` section 3: exact one-sided containment with no tolerance; exact Q(√5) certificates (complete exact vertex sets for inside and outside, two exact witness points for straddling); uncertain means rejected. Section 6: an exact witness rotation and E0 controls for shallow crossings and exact contact |

## Plan

1. State contract (`state-contract.md`). Drafted; awaiting the scoped re-check.
2. Minimal geometric witness E0–E4 from the contract: controls, exact piece regions for two neighbouring caps, an exact jumble twist of about 10° fixing both poles, a turn at the neighbour, certified blocked and unblocked grips, and the reverse sequence.
3. Initial impact assessment on stages 2.3 to 2.5 for the owner.
4. Stop. A simulator against the contract, one realignment-circle atlas, rendering of certified witnesses and blocking reasons, and the four-dimensional Jambler port remain optional and need a separate go-ahead.
