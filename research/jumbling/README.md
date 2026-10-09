# Jumbling study of the 600-cell-Full puzzle

Status: **exploratory computation, not reviewed.** The results below are leads for the senior reviewer's plan check (`astra-plan-packet.md`), not accepted theory. No owner decision, model identity or product scope changes here.

## Question

The owner asked whether the facet-turning (cell-cap) 600-cell can be made a jumbling puzzle in the sense of the Hypercubing wiki's [formal grip theory](https://hypercubing.xyz/theory/grip-theory/formal/#jumbling) and HactarCE's [Jambler](https://github.com/HactarCE/Jambler): extra twists of a cap by rotations about the cell axis that are not cap symmetries, made finite by fudging, which snaps the moved grips back onto grips. This page records what the retained model (cut depth α = 121/125, `assets/model.npz`) says about that question.

Terms follow `research/theory/RETHLAS_BLUEPRINT.md`: pole n_c, cap U_c, complete cap signature M(A), the cap's A4 rotation group, K⁺ and K. A *jumble twist* about pole c is a rotation in SO(3)_c, the rotations of four-space fixing n_c, that is not in A4. A *fudged* jumble twist is a permutation σ of the poles that fixes c and is applied to the signatures of the cap's pieces.

## Reproduce

```text
python research/jumbling/jumble_study.py
```

Standard library plus NumPy. Reads `assets/model.npz` read-only and writes `research/jumbling/results.json` (about 80 s). `h4.py` builds the 600-cell from the 120 unit quaternions of the binary icosahedral group, independently of the model; the script aligns the two coordinate systems by an orthogonal map and checks that all 600 poles match.

## Computed results (synthetic geometry and model data; no Windows or rendering evidence)

1. **Interacting poles.** The signatures of the 3,097 pieces in one cap use exactly 57 poles: c and 56 others in five shells at 15.522°, 25.243°, 36°, 41.410° and 44.478° from n_c (4, 12, 24, 12 and 4 poles). These are the cells that share at least one vertex with c. A vertex-hull calculation shows that the same 56 poles interact for every cut depth between 0.92705 and 1 (depth as a fraction of the facet distance); the next shell (49.118°, 24 poles) joins below 0.92705.
2. **Discrete jumble twists.** Rotations in SO(3)_c outside A4 that carry at least two independent interacting poles exactly onto poles: 1,224 found (first pole restricted to the six A4-orbit representatives), in 33 classes up to A4 × A4. The strongest realigns 24 poles (90°, all of the 36° shell) or 20 poles (44.48°). Every class leaves some interacting pole between 16° and 48° from the nearest pole of its shell, against a nearest-pole spacing of 15.52°.
3. **Snapping.** For none of the 33 classes is nearest-pole snapping a bijection of the 57 poles. At most 299 of the 3,097 cap signatures are mapped onto cap signatures.
4. **Combinatorial automorphisms.** An exhaustive individualisation-refinement search finds exactly 24 permutations of the 57 poles that fix c and map the set of 3,097 cap signatures onto itself. They are exactly the 24 geometric symmetries of the cell's neighbourhood: the 12 rotations of A4 and 12 reflections.
5. **No near-symmetries.** Over 200,000 random rotations (seed 20261009), the smallest worst-case landing error grows almost linearly with the distance from A4: 3.1° at 5°, 6.5° at 10°, 9.4° at 15°, 18.7° at 30°. As a baseline, the Sun Cube 45° face twist lands its F, E and V grips within 9.7° of a grip, with a nearest grip spacing of 35.3°.

## Claims for review (not yet accepted)

- **C4 ⇒ no fudged jumbling.** If a fudged jumble twist must map the cap's existing piece signatures onto themselves, then result 4 says every fudged jumble twist about a cell is already a cap symmetry or a reflection. The retained puzzle would then admit no nontrivial fudged jumbling about cell axes at α = 121/125. Other depths are not checked; their signatures are not in the repository.
- **Unfudged jumbling is infinite.** K⁺ = (I* × I*)/⟨(−1, −1)⟩ is a maximal finite subgroup of SO(4), because I* is a maximal finite subgroup of the unit quaternions. Any jumble twist therefore generates an infinite group with K⁺, and the 600 poles, which span four-space, have an infinite orbit. Realigning a single pole pair fixes a rotation only up to a circle in SO(3)_c, so unrestricted jumble twists come in continuous families.

## Not covered

Exact geometric blocking after a jumble twist (which caps can turn), rendering, the solving theory and any alternative cut that might admit fudging. They wait for the plan check. The four-dimensional Jambler port is a separate local experiment outside this repository.
