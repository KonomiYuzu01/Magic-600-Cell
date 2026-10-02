# Design options, round 2: open points in the owner's picks (October 2026)

Status: **options for the owner to choose from, not decisions.** Companion to [design-options-2026-10](design-options-2026-10.md). After the owner rules on the points below, the picks of both rounds are recorded together in a dated owner-decision page and the charter draft.

## Picks already settled by the owner (2 October 2026)

These are listed so that the open points are read against them. They are recorded in the wiki only after the open points below are ruled, as one decision page.

| Gap | Owner's pick | Meaning |
|---|---|---|
| 1 | 1B, milestones compressed to a small scale | see R1 |
| 2 | 2A combined with 2B; the mathematical language must be described | see R2 |
| 3 | 3B with 3C: authored theme families, each with scene looks | count open, see R3 |
| 4 | 4A | annotated pairs and nexus cards; no embedding-first discovery, so no Taste Lab phase 1b approval is needed for this pick |
| 5 | 5A with 5C | competence checks on fixture tasks plus local in-situ metrics on real solving; soft key disclosure |
| 6 | Global and Local only, redesigned; solve "the two problems" inside them | see R4 |
| 7 | 7D, one entry serving both roles | see R5 |
| 8 | 8B, with one rule above it | an engineer's evaluation that finds something wrong which affects the work or correctness has top priority; otherwise 8B's order |
| 9 | 9B | budgets by context (about 20, 50, 80 words); see R6 |
| 10 | 10B | audition in G5, minimal set in the slice; see R6 |

## How this was made

As in round 1: Claude wrote its answer first and recorded its SHA-256 digest (`2228949a…6c6d`, 22:00 UTC); Codex (senior model, read-only wrapper call `20261002T220104Z-5cbd3fe9`) answered the same packet without access to it. The owner then settled gaps 3, 4, 5 and 8 while the analyses ran; options for those are dropped here.

## R1. "Milestones compressed to a small scale" (gap 1)

**R1a. Fewer unlock occasions.** *(recommended, with R1b's presentation)*
- Approach: structural feedback (1A) at every step as before. Real content unlocks at only three occasions: the first checked insertion with the solver's own macro, the first completed block, the first exactly completed orbit; plus one quiet whole-puzzle moment. Each unlock is one small item: a diagram, a theory note, or the solver's macro saved as a template. No theme or preset is a reward; comfortable themes are always available.
- Pros: keeps 1B's earned content with little content to write.
- Costs: few surprises between the first block and the first orbit.
- Engineering and renderer: events from committed, checked results only; no achievement database beyond a small event log.
- Conflicts: trims the round-1 ladder's unlocks, not its truth conditions.

**R1b. Same ladder, every reward small and local.**
- Approach: keep L0–L11, but every reward is one small item shown in place (on the block, on the orbit's entry), never full-screen or modal; the content waits as a marker the user opens.
- Pros: richest learning feedback.
- Costs: hardly reduces the work: twelve rewards still need content and event rules.

**R1c. Short-distance milestones.**
- Approach: milestones are small achievements: a new skill shown on a fresh task, a small declared block, one endgame condition cleared. Each offers one explanatory fragment; at most one pending offer.
- Pros: encouragement before the first orbit, which may take days.
- Costs: more event definitions; risk of trivial blocks declared only to collect rewards.

Recommendation: **R1a**, presented as R1b (small, in place, never modal). Claude and Codex differ: Claude recommended R1b's presentation alone; Codex recommended R1a and noted that R1b does not reduce scope.

## R2. 2A with 2B, and describing the mathematical language (gap 2)

**R2a. One concept table; focus changes visibility, never meaning.** *(recommended)*
- Approach: one table lists, for every concept, its term, meaning, notation, exact ID, glyph, example and the level where it is first taught. Channels as in 2A: colour = class from a proper colouring of the rings (written as a map c: rings → {1..k}); position and fixed anchors = identity and relations; glyph = role in the current operation; orientation glyph = an element of the tetrahedral rotation group, drawn by its action on a marked tetrahedron; pattern = status predicate; motion = change, with a static equivalent. 2B: inside the focus set all channels are drawn; outside it only class colour, but protection conflicts and complete-operation effects always break through. On inspection the exact mathematical address (cell, ring, orbit, orientation element) appears in standard notation next to the glyph; the term card gives the definition; the theory book gives proofs or references and each visual's basis. Distinguish explicitly: buffer position versus its changing occupant, position versus orientation, permission versus completion.
- Pros: one language across both views and every theme; the user learns real notation.
- Costs: mathematician review (M2); tests of simultaneous role and status combinations.
- Engineering and renderer: shared semantic metadata drives cards and overlays; geometry and IDs never change.
- Conflicts: none.

**R2b. A relationship strip beside the geometry.**
- Approach: as R2a, but most exact language sits in a compact strip inside Global and Local (destination, current occupant, wanted piece, buffers, frame, residual), linked to geometric markers.
- Pros: fewer small labels in the scene; the relations read as a grammar.
- Costs: eye travel; the strip can become a text panel.

**R2c. Three levels of detail by distance.**
- Approach: as R2a, with a middle ring: focus = all channels; same orbit or block = shape and colour; elsewhere = colour class only.
- Costs: a third level to tune; adopt only if the G4 test shows users lose orientation with two levels.

Recommendation: **R2a**; R2c only if G4 asks for it. Claude and Codex agree on R2a in substance.

## R3. How many theme families at 1.0 (gap 3)

- **R3a. Two families, three scene looks each** (solving, inspecting, celebrating) *(recommended)*: the smallest number that proves the family mechanism; six scene presets to verify.
- **R3b. Three families**: more taste coverage; nine presets, more owner review and asset work, possibly more stage-2 time.
- **R3c. One family plus a variant**: least work; may not satisfy "several themes".

Fixed across families (both analyses): commands, keys, layout anchors, role glyphs, status meanings, comparison conventions, accessibility alternatives, motion meanings; a theme never changes projection or interaction scope. Families may vary surfaces, materials, approved typography, palette character and density within one region topology. Users tune within validated ranges. Every family's most expensive scene must pass the renderer gate.

## R4. Global and Local only: which "two problems" (gap 6)

First, which two problems did you mean?
- **Reading 1 (Codex):** the two view questions of D8: Local, "does this piece match its target here?"; Global, "how do these two places relate?".
- **Reading 2 (Claude):** the two problems the rejected Orbit and Effect views were meant to solve: "what is left in this orbit?" (residual) and "what will this complete operation change and preserve?" (effect, collateral, protection).

Options (each covers both readings):

**R4a. Linked Global and Local, analysis in place.** *(recommended, with R4b's split)*
- Approach: both views visible and linked. Local compares current and target/Home frames with fixed anchors for target, piece and buffers; Global locates them in the actual structure. A residual strip and an operation summary are embedded in the views; detail expands in place. Following R4b: the orbit residual expands in Global (zoomed out it shows the 35 orbits in stage order on the real structure), the operation's net effect on target, occupant and buffers expands in Local during preview, with a collateral line naming every other orbit touched and every protection result.
- Pros: no context rebuilding; each problem shows where its question is asked.
- Costs: screen area; two viewports to composite and measure.
- Engineering and renderer: one context and revision; any edit of role, macro or protection invalidates the review; collateral shows even under display filters.
- Conflicts: none.

**R4b. Each view absorbs one problem.** As above, but with no shared strip: residual lives only in Global, effect only in Local.

**R4c. One display area, explicit switch.** One viewport switches between Global and Local; a shared context strip keeps target, buffers, residual summary and review status. Pros: more room, lower rendering cost. Costs: more switching; the solver must remember the other view.

Recommendation: **R4a with R4b's split.** Please also say which reading of "two problems" you meant.

## R5. One entry for both roles (gap 7)

**R5a. One workspace, task-led optional coaching, a practice copy.** *(recommended)*
- Approach: everyone enters the same workspace. First launch: one cap or orbit emphasised on the full model, an optional first-turn hint (S1 within 30 seconds). Coaching follows the task the user chooses (inspect structure, compare Home, find buffers, compose, check, execute); explorers can stay with structural questions, solvers continue along the same path. Returning users resume their context directly. "Show all keys" is always available. A practice copy (a fresh, clearly labelled full-model session) lets anyone scramble and play without touching their own solve.
- Pros: no role choice, no separate doors.
- Costs: coaching triggers need care.
- Conflicts: none, if learning stays a path, not a forced wizard.

**R5b. A short common introduction, then a visible skill index.**
- Approach: every first-time user gets the same brief introduction (turn and undo, isolate structure, compare a piece with Home), skippable at once; afterwards a compact skill index shows the same ordered path to both audiences.
- Pros: predictable; fewer trigger rules.
- Costs: experts may find the introduction irrelevant; explorers may find later chapters solving-centred.

Recommendation: **R5a.** Both analyses agree.

## R6. Details to confirm (from the picks together)

- **Undo and rewards:** undo at once withdraws a current achievement and any pending celebration, but keeps explanations already unlocked and macros the user saved. (Round 1 said only "revoked silently on undo".)
- **9B numbers:** about 20, 50 and 80 words are trial budgets for the whole screen in that context (solving, explicitly opened inspection, explicitly opened learning), not allowances per panel. Error, protection and stale messages and the theory book stay exempt. The numbers are set by the G6 test.
- **10B:** after the G5 audition, sound is adopted or rejected as a separate owner decision; if adopted, one minimal cue set shared by all themes, each cue with a visual equivalent and mute.
- **Numbering:** the reward ladder's levels and the key-disclosure levels are two different scales; they get distinct names.
- **Gap 8 rule:** "an engineer's evaluation finds something wrong that affects the work or correctness" means a measured or demonstrated failure (a test, a measurement, a counterexample); an unmeasured concern becomes an experiment first, not a veto.

Recommendation: confirm all five.

## Next step

The owner replies, for example "R1a R2a R3a R4a(reading 2) R5a R6 ok". Then the picks of both rounds go into one owner-decision page, the charter draft, the briefing and the wiki index and log, and the tests named in both pages are listed as stage 2.3 work.
