# Packet: plan check of the jumbling study (senior reviewer, plan kind)

Run read-only as a plan check by the senior Codex reviewer (Astra), effort `max`, speed tier `fast`. Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: decide whether the computed claims in `research/jumbling/README.md` are correct and sufficient, and whether the planned next steps (section 5, "Planned next steps") are the right ones, before any jumbling theory, simulator or renderer work starts. The owner's question: can the facet-turning 600-cell (the retained 600-cell-Full model) jumble, and if so, what are all its jumbling states, what puzzle theory and solving theory (extending orbit-first block building) apply, how should a jumbling puzzle be rendered and inspected in four dimensions, and what would adding jumbling do to the 1.0 program.
- Acceptance: a schema-valid result in which every claim C1 to C6 below and every question Q1 to Q5 has a finding or an explicit statement that it holds, with evidence (`path:line`) or a counterexample and an adjudicating experiment where the reviewer disagrees.
- Non-goals: an exact order of the legal group; a new human-solve claim; any performance, Windows or DirectX claim; product scope or model-identity decisions (owner only).

## 2. Actual problem and reproduction
- Reproduce every number: `python research/jumbling/jumble_study.py` (NumPy only, about 80 s, reads `assets/model.npz`, writes `research/jumbling/results.json`).
- Claims:
  - **C1, interacting poles.** At α = 121/125 the 3,097 cap signatures of one cap use exactly 57 poles: c and 56 others in shells at 15.522°, 25.243°, 36°, 41.410°, 44.478° (4, 12, 24, 12, 4). The same 56 interact for every depth in (0.92705, 1) (fraction of the facet distance), by the vertex-hull calculation in `shells_section`.
  - **C2, discrete jumble twists.** Rotations in SO(3)_c outside A4 that carry at least two independent interacting poles exactly onto poles of the same shell: 33 classes up to A4 × A4 (`realignment_section`). The best realigns 24 poles (90°) or 20 (44.48°); every class leaves a pole 16° to 48° from the nearest pole of its shell (spacing 15.52°).
  - **C3, snapping.** No class gives a bijective nearest-pole snapping; at most 299 of 3,097 cap signatures map onto cap signatures (`snapping_section`).
  - **C4, automorphisms (main claim).** Exactly 24 permutations of the 57 poles fix c and map the set of cap signatures onto itself, and they equal the 24 geometric symmetries of the cell's neighbourhood: the 12 A4 rotations and 12 reflections (`automorphism_section`). Inference: if a fudged jumble twist is a pole permutation σ fixing c and applied to the cap's pieces through their signatures, then σ must preserve the cap's signature set, so it is a symmetry. The retained puzzle has no nontrivial fudged jumbling about cell axes.
  - **C5, unfudged jumbling is infinite.** K⁺ is a maximal finite subgroup of SO(4) because I* is maximal finite in Sp(1); a jumble twist outside K⁺ generates an infinite group with K⁺, and the spanning set of 600 poles has an infinite orbit. Realigning one pole pair fixes the rotation only up to a circle in SO(3)_c, so the jumble twists that unblock a given cap form one-parameter families.
  - **C6, no near-symmetries.** The smallest worst-case landing error grows almost linearly with distance from A4 (3.1° at 5°, 9.4° at 15°, 18.7° at 30°; 200,000 samples, `approx_section`). Sun Cube baseline: 9.7° error at a 35.3° grip spacing.

## 3. Environment and versions
- Branch `claude/jumbling-study` from main 7057cb8. Linux cloud session, CPython 3, NumPy 2.5.3. Evidence kind: source/fixture (`assets/model.npz`) and synthetic geometry only.

## 4. Necessary source and evidence
- `research/jumbling/jumble_study.py`, `research/jumbling/h4.py`, `research/jumbling/results.json`, `research/jumbling/README.md`.
- Theory baseline: `research/theory/RETHLAS_BLUEPRINT.md` (retained-model definitions and hypotheses R, F, I, C, E; Lemma 6 on cap signatures, including its caution that a local twist need not act on all signatures by one common permutation; Lemma 7 wreath-product embedding; Lemma 12 normalization; the table of distinct groups).
- Solving workflow: `docs/progress/1.0/solving-workflow.md` (orbit-first block building, stars, buffers, protection) and the human-solve boundary in its section 7.
- Model data: `assets/model.npz` keys `normals`, `mask_offsets`, `mask_values` (complete cap signatures M(A)), `orbit_id`.
- External definitions: Hypercubing wiki formal grip theory, jumbling extension (infinite grip sets, blocked grips); Jambler (grip orbits under jumble twists, applied to all grips, with nearest-grip fudging judged visually).

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | Some discrete jumble twist about a cell axis is a usable fudging | Enumerate two-pair realignments, classify, snap | `realignment_section`, `snapping_section` | 33 classes, none bijective |
| 2 | A fudging need not be a realignment; search all of SO(3)_c | Random sampling with a distance-from-A4 profile | `approx_section` | Error grows with distance; no isolated near-symmetry seen |
| 3 | Fudged twists must be signature-preserving pole permutations | Exhaustive automorphism search of the cap incidence structure | `automorphism_section` | Exactly the 24 geometric symmetries |

Planned next steps, pending this check:
1. Exact geometric blocking after a jumble twist: which caps can turn, using the piece regions rather than signatures alone (the moved cap region R(U_c ∩ P) is not U_c ∩ P because P is not round).
2. An unfudged jumbling simulator on the model data: per-piece four-dimensional rotations instead of slot permutations, with legality decided by blocking; round-trip and equivariance tests against `rotperms`, `move_src` and `move_dst` for the 1,200 retained generators.
3. Puzzle theory: replace the group G by the groupoid of configurations reachable by legal twists; define what replaces orbits, frames and invariants; state how orbit-first block building extends, for example an "unjumble to the lattice, then solve" phase, within the human-solve boundary.
4. Rendering and inspection requirements: per-piece transforms, morphing for any fudged variant, display of blocked and unblocked caps, cap-local focus, grip-orbit overlay. A separate four-dimensional Jambler port is used as an exploration tool.
5. Alternative designs that admit fudging (other cuts, other grip sets, or a smaller subset of caps). Any such puzzle is a new model identity, so it needs an owner decision.
6. An impact assessment on stages 2.3 to 2.5 for the owner.

## 6. Constraints and owned files
- Invariants: `assets/manifest.json` and the model are immutable. Nothing here changes model identity, legal generators or the 259,800 labelled slots. Jumbling is studied as a possible separate puzzle, not a change to 600-cell-Full.
- Read-only review. No files may change.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Use one finding per claim or question where there is anything to say. Questions:
  - **Q1.** Is the C4 inference sound and complete? In particular: is "a fudged twist is a signature-preserving pole permutation" the right definition, given Lemma 6's caution and the grip-theory definition (pieces = sets of grips, twists act on active grips)? Do reflections count as fudged twists? Could a fudged puzzle legitimately add pieces with new signatures, and would that still be the same model?
  - **Q2.** Is C5 correct, including the maximality argument and the one-parameter families?
  - **Q3.** What does "all jumbling states" mean precisely for this model, and which finite object should be classified instead: first-order jumble twist classes, blocking patterns, or something else?
  - **Q4.** Which parts of the theory in `RETHLAS_BLUEPRINT.md` survive unfudged jumbling (signatures, Lemma 7 frames, Lemma 12 normalization, invariants), and what replaces orbit-first block building?
  - **Q5.** Are the planned next steps 1 to 6 right, in the right order, and is anything missing or unnecessary?
- Review only; do not perform follow-up work.
