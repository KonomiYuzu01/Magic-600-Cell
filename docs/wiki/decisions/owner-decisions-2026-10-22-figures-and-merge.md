---
id: owner-decisions-2026-10-22-figures-and-merge
type: decision
status: verified
visibility: public
summary: Owner decisions of 22 October 2026 - the tutorial's illustrations cover every rule, every macro, one worked example per phase and the reading concepts (several dozen to more than a hundred figures); the main figures are pictures of the actual puzzle; and the integrator merges pull request 112 itself once the tutorial and its acceptance are written.
related: [owner-decisions-2026-10-22-tutorial-first, owner-decisions-2026-10-20-tutorial-acceptance, owner-decisions-2026-10-17-jumbling-pr]
supersedes: []
claims:
  - {id: figure-coverage, evidence_kind: decision, checked_at: 2026-10-22}
  - {id: figure-puzzle-pictures, evidence_kind: decision, checked_at: 2026-10-22}
  - {id: figure-shrink, evidence_kind: decision, checked_at: 2026-10-22}
  - {id: merge-112-by-integrator, evidence_kind: decision, checked_at: 2026-10-22}
  - {id: figures-and-merge-readings, evidence_kind: decision, checked_at: 2026-10-22}
---

# Owner decisions, 22 October 2026: figure coverage and the merge of pull request 112

The owner gave both decisions in chat messages in Chinese, after the decision to put the
[tutorial and its acceptance first](owner-decisions-2026-10-22-tutorial-first.md); this page records them in English.

## Decision 1: what the illustrations cover

To the standard of a finished product the illustrations cover four layers:

1. **Every rule** has at least one before-and-after pair of figures: in the bulk phase seam, facematch, cover and conj;
   in the height phase r22d75 T0, T1 and T2, r22d81 and each tier of h4; in the high phase F1 to F3 and D19; in the
   tail zero_bad, pure, close and t1prime; in the lattice phase the orbit steps and the corrections.
2. **Every macro** in the macro appendices has a step-tree figure (its named parts such as x, y and z, or V·W·V⁻¹)
   and a figure of the region it acts on.
3. **Every phase** has one complete worked example drawn as a sequence, one figure per step or per segment, from the
   development seeds 7001, 7106 and 7101 and the stops of the 3000-twist scrambles.
4. **Reading and concept figures**: the cuts, height, the cover graph, invisible rotations, cells, and the difference
   between labels and poses, among others.

The owner expects this to come to several dozen to more than a hundred figures, not a dozen or so.

## Decision 3: the main figures show the actual puzzle

After seeing four abstract drawing styles (cap nets, ring sheets, projections of centroids, diagrams), the owner said
that most figures should be pictures of the actual puzzle. The integrator proposed pictures of the full-state page
(`research/jumbling/viewer/full.html` in its shot mode, all 259,800 stickers of the exact state), and the owner
confirmed the second prototype:

- The main figure of a step is the stereographic local view centred on the step's cap. The pieces the step moves are
  drawn opaque, and the stickers near the cap are faint ghosts.
- Whole-puzzle overviews use the perspective view.
- Abstract drawings stay only where nothing geometric is shown: step trees, rule tables, the cover graph and the
  height trees.

## Decision 4: smaller cells and stickers in the puzzle pictures

The owner asked for smaller cells and stickers in the tutorial's pictures. The integrator showed three settings of the
full-state page's two shrink controls, drawn on the same two states (a local stereographic view and a global
perspective view): A, the page's default, cell (facet) shrink 0.76 and sticker shrink 0.82; B, 0.62 and 0.72; C, 0.50
and 0.62. The owner chose C. Every puzzle picture of the tutorial is drawn with cell shrink 0.50 and sticker shrink
0.62 (`cs` and `ss` in `research/jumbling/tutorial/figures/figures.json`). The page's own default stays as it is.

## Decision 2: the integrator merges pull request 112

When the tutorial and its acceptance are written, the integrator merges pull request 112 itself.

## Integrator readings (the owner may change them)

1. **Figures.** Every figure follows reading 4 of the [tutorial-first decision](owner-decisions-2026-10-22-tutorial-first.md):
   drawn by repository scripts from exact simulator states of development seeds, never a seed of the test set, each
   with a manifest entry and a check that draws it again and compares. The puzzle pictures of decision 3 are headless
   Chromium captures of the research page (`research/jumbling/viewer/capture_tutorial.mjs`), not pictures of the
   Direct3D 12 renderer and not Windows evidence; the abstract drawings are SVG; colours are provisional and decide nothing
   for the design track, and G1 to G6 stay with the owner. A rule that has not fired in any development run is drawn
   from the first development state where it fires; until one exists the catalogue lists the figure as missing and
   says why, and no figure stands in for it. The drawing style is a matter of taste, so the integrator shows the owner
   prototypes and the owner picks one before the full set is drawn.
2. **When to merge.** "Written" means the tutorial (text, macro appendices and figures) meets the finished standard,
   and the acceptance test has been run and recorded as far as it can go without a further owner decision. The merge
   records the results; it is not the owner's sign-off of the acceptance test, which stays the owner's. The conditions
   of `AGENTS.md` "Merging" hold: the checks for the changed areas pass on the final head and are listed in the pull
   request's description, the `headless` CI check passes on that head, the review rules are met, the branch merges
   cleanly, and the method is "merge". After the merge the integrator gives the owner a short summary with the link.
3. **Earlier merge conditions.** The instruction sets the point at which pull request 112 merges. Of the conditions in
   the [record of 17 October](owner-decisions-2026-10-17-jumbling-pr.md), the accepted tutorial becomes reading 2 and
   the handoff and observation logs are done; the A and B theory and the Archon formal verification, which the
   [tutorial-first decision](owner-decisions-2026-10-22-tutorial-first.md) leaves stopped, no longer hold the merge
   back.

## Effect

- The tutorial's figure catalogue is planned against the four layers above, with puzzle pictures as the main figures,
  drawn with cell shrink 0.50 and sticker shrink 0.62.
- Pull request 112 no longer waits for the owner to merge it; the integrator does so once reading 2 holds. This
  updates the last item of the tutorial-first decision's "Effect" and the merge conditions of 17 October (reading 3).
