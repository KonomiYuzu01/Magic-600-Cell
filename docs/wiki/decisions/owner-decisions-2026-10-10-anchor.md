---
id: owner-decisions-2026-10-10-anchor
type: decision
status: verified
visibility: public
summary: Owner decision of 10 October 2026 - the sticker shrink anchor becomes the area-weighted centroid of each base sticker's triangles, computed from the unchanged mesh; no asset or model identity changes; the S-B specification, the jumbling render contract and the references change, and W3 is re-accepted.
related: [owner-decisions-2026-10-09-jumbling, renderer-candidates, s-b-probe-results]
supersedes: []
claims:
  - {id: area-centroid-anchor, evidence_kind: decision, checked_at: 2026-10-10}
  - {id: no-asset-or-identity-change, evidence_kind: decision, checked_at: 2026-10-10}
  - {id: w3-re-acceptance, evidence_kind: decision, checked_at: 2026-10-10}
---

# Owner decision, 10 October 2026: sticker shrink anchor

The owner gave this decision in a chat message in Chinese, in answer to the question the W-J lattice check raised. This page records it in English.

## Background

- The S-B pipeline shrinks each sticker towards an anchor point: `work/experiments/renderer-sb/SPEC.md` section 3, steps 1 and 2.
- The anchor was `assets/mesh_centers.f32`, the retained toolkit's numbering centres. These do not move with the sticker under the puzzle's symmetries.
- Two defects followed, both measured from the unchanged assets in the engine environment on 10 October 2026:
  - **W-J lattice check:** J1 generator 1 moves 4,605 stickers. For 4,125 of them, the shrunk geometry disagreed with the destination slot's by up to 0.0144 world units. This is acceptance item 2 of E-2.4-0J, which failed.
  - **S-B W3:** after each full turn, the shrunk geometry of 4,120 of the 4,600 moving slots jumped by up to 0.0118 world units.
- The raw sticker meshes themselves are congruent under these moves (to 1.07e-7). Only the anchor was at fault.

## Decision

1. **Option A.** The anchor of base sticker `local` becomes the area-weighted centroid of its triangles in `assets/mesh_vertices.f32`, computed directly from the existing mesh.
2. **Nothing else changes in the model.** No asset and no model identity changes.
3. **Accepted cost:**
   - the S-B specification, section 3 step 1, and the jumbling render contract change;
   - the references are generated again;
   - W3 is re-accepted, under E-2.4-0J acceptance item 6: three cold W3 runs of the changed build.

With the new anchor, the lattice disagreement is at most 7.73e-8 and the W3 turn-end jump at most 6.61e-8. No slot is above 2e-6 in either.

## Consequences

- The S-B probe, and through it the level 2 native library of the Godot (S-A2) and Qt (S-D) scenes, uses the new anchor from the change on. Their builds get new identities.
- The S-B W3 records and the level 2 records of 4 October keep their build identities and their results. They describe the old anchor.
- The GPU re-acceptance runs need the owner's attended session.
