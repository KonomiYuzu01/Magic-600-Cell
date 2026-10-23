---
id: owner-decisions-2026-10-22-g1
type: decision
status: verified
visibility: public
summary: Owner answer of 22 October 2026 to the G1 pick - all seven structure modes and all four greybox layout structures stay in the Look Lab; nothing is dropped and no single layout is chosen yet; Look Lab work continues.
related: [owner-decisions-2026-10-03, owner-decisions-2026-10-10-looklab-views, owner-decisions-2026-10-22]
supersedes: []
claims:
  - {id: g1-all-modes-kept, evidence_kind: decision, checked_at: 2026-10-22}
  - {id: g1-all-layouts-kept, evidence_kind: decision, checked_at: 2026-10-22}
  - {id: looklab-continues, evidence_kind: decision, checked_at: 2026-10-22}
---

# Owner decision, 22 October 2026: G1 keeps every mode and layout

The owner gave this answer in a chat message in Chinese, replying to the G1 question: which structure modes go forward (at least two of the seven), and which layout structure. This page records it in English.

## Decision

1. **Every structure mode stays.** All seven G1 presets stay in the Look Lab as presets: perspective, stereographic, orthographic, two hemispheres, Hopf rings, ring sheet, and stereographic with the polar cap hidden (`tools/looklab/presets/g1-*.json`, merged with pull request #121). None is dropped.
2. **Every layout structure stays.** The four greybox layouts stay in the Look Lab: `central-stage`, `docked-workbench`, `split-views` and `focus-inset`. The owner chose no single layout.
3. **Look Lab work continues.**

## Effect

- **G1** ([protocol](../../progress/1.0/stage-2-experiment-protocol.md), section 4.3) asks for at least two modes kept as presets; keeping all seven meets that. Its exit also asks for a chosen layout as the first layout specification, and for the first visual feature list (H-01) and layout specification (H-04) to go to the engineering track. These stay open: with no single layout chosen, there is no first layout specification yet.
- **Progress.** The 2.3 item "G1 structure catalogue: visual modes kept as presets" is done with this page as evidence. A new item tracks the open part of the G1 exit.
- **Unchanged.** The owner alone takes G1 to G6 ([owner-decisions-2026-10-03](owner-decisions-2026-10-03.md)). The cap variant still works only at the starting orientation, and the plain stereographic mode still shows one cell from inside there (Look Lab handoff LO-33). Every layout stays a greybox for exploration, not an owner-approved layout ([owner-decisions-2026-10-10-looklab-views](owner-decisions-2026-10-10-looklab-views.md)).
