---
id: owner-decisions-2026-11-25-icon-tiers
type: decision
status: verified
visibility: public
summary: Owner decision of 25 November 2026 on the two icon tiers - tier 2 content glyphs are coloured from the scene's encoding tokens, high quality and animated only on a state or view change; tier 1 command icons stay monochrome; the icon values come in a separate icon ADR in freeze round 3; three.js may serve as an authoring and preview tool, never at runtime and not for K2's product line. It matches the frozen ADR-D01.53 to ADR-D01.55 and edits no frozen text.
related: [owner-decisions-2026-11-23-a7-freeze, owner-decisions-2026-11-22-design-art-path, owner-decisions-2026-11-22-adr-d01-overrules]
supersedes: []
claims:
  - {id: tier2-glyphs-coloured, evidence_kind: decision, checked_at: 2026-11-25}
  - {id: tier1-icons-monochrome, evidence_kind: decision, checked_at: 2026-11-25}
  - {id: icon-adr-round3, evidence_kind: decision, checked_at: 2026-11-25}
  - {id: threejs-authoring-only, evidence_kind: decision, checked_at: 2026-11-25}
---

# Owner decision, 25 November 2026: the two icon tiers

**How the decision came about.**
- In the ADR line's session, the owner asked whether three.js suited K2, and then whether it suited work outside the
  renderer, for example coloured, animated, high-quality icons.
- The ADR line answered with a recommendation for each of ADR-D01's two icon tiers.
- The owner answered: "tier 2 as recommended, tier 1 stays monochrome".

**What this page does not do.**
- It edits no frozen text. The decision matches ADR-D01.53 to ADR-D01.55 as frozen at A7
  ([A7 freeze](owner-decisions-2026-11-23-a7-freeze.md)), so it is no overrule.
- It moves no gate.
- It accepts no ADR.

## Decisions

1. **Tier 2 content glyphs follow the recommendation** (ADR-D01.54, ADR-D01.55). They cover cells, grips, axes,
   macros, previews and proof obligations, and the Jumble and research glyphs of ADR-D02.
   - Colour comes from the scene's encoding tokens, after the 16 px legibility test.
   - They are drawn to a high standard from the project's own geometry. The mathematician checks that each glyph is
     true to the puzzle.
   - A glyph moves only when state or view changes, and every motion has a meaning in the motion table (H-03). Reduced
     motion turns it off.
   - At rest a glyph is static. The renderer gate is met with the full chrome, so animated glyphs count against it.
2. **Tier 1 command icons stay monochrome** (ADR-D01.53, unchanged).
   - Fills stay bound to theme roles.
   - The final set is drawn from the project's geometry.
   - A small motion on a press or a state change is allowed only with an H-03 meaning.
   - Reason: colour already carries meaning in the scene (state roles, palettes). Coloured command icons would compete
     with that meaning and would have to hold up in every theme, contrast and colour vision mode.
3. **The icon values come in a separate icon ADR in freeze round 3**
   ([design and art path](owner-decisions-2026-11-22-design-art-path.md)).
   - The interaction artist leads it, and the mathematician checks the glyphs.
   - Stage 2 keeps labelled placeholder icons.
4. **three.js is a tool, not part of the product.**
   - It may serve as a parametric authoring and preview tool: glyphs generated from the model's geometry, and
     throwaway pages that show the owner options. Its output enters the product as assets that Godot draws with theme
     tokens.
   - It is never a runtime dependency.
   - It is not used for K2's product line, for three reasons:
     - the greybox runs in the product shell;
     - the renderer is selected: the S-B Direct3D 12 drawing core hosted by Godot 4.7.2 .NET;
     - shaders are written once in portable HLSL.
   - A local install needs an allowlist entry and the owner's approval.
   - Signature moments and promotion material are rendered with Blender and FFmpeg. A promotional web demo may use
     three.js.
