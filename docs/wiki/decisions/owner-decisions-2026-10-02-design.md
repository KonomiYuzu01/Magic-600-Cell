---
id: owner-decisions-2026-10-02-design
type: decision
status: verified
visibility: public
summary: Owner design decisions of 2 October 2026 on ten design gaps - small content rewards, one encoding grammar with defined and proved mathematical language, two theme families with scene looks, annotated references, competence checks with local metrics, two linked views, one entry, role priority, text budgets and sound audition.
related: [owner-decisions-2026-10-02, renderer-candidates, owner-decisions-2026-10-02-scope]
supersedes: []
claims:
  - {id: rewards-small, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: encoding-and-math-language, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: theme-families, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: interests-annotated, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: learning-measures, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: two-views, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: one-entry, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: role-priority, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: text-budgets, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: sound-audition, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: options-page, evidence_kind: source, path: docs/progress/1.0/design-options-2026-10.md, checked_at: 2026-10-02}
  - {id: options-round2-page, evidence_kind: source, path: docs/progress/1.0/design-options-2026-10-round2.md, checked_at: 2026-10-02}
---

# Owner design decisions, 2 October 2026

The owner chose among the options for ten design gaps in two rounds: [design-options-2026-10](../../progress/1.0/design-options-2026-10.md) and [design-options-2026-10-round2](../../progress/1.0/design-options-2026-10-round2.md). Each option there was merged from two sealed analyses (Claude and Codex). The owner's replies were in Chinese; this page records them in English. Option codes refer to those pages. Every number below that is marked "trial" is set by the named stage 2.3 test, not by this page.

These decisions add to the charter draft section 5 ([charter-2.0-draft](../../progress/1.0/charter-2.0-draft.md)), which carries the details. They do not change any binding input: rendering never changes mechanical state, all 259,800 sticker slots and 1,200 generators stay, the [renderer gate](renderer-candidates.md) holds in the shipped look, and the human-solve boundary of [2 October](owner-decisions-2026-10-02.md) holds.

## 1. Rewards (1B, with fewer unlocks: R1a)

- The structure is always the reward: after an exact progress check on the committed state, the puzzle's own drawing changes (a seated piece settles, a finished block's outline consolidates, a completed orbit sweeps once).
- Real content unlocks at only three occasions: the first checked insertion with the solver's own macro, the first completed block, the first exactly completed orbit; plus one quiet whole-puzzle moment. Each unlock is one small item (a diagram, a theory note, or the solver's macro saved as a template), shown in place, never modal.
- No theme or preset is a reward; comfortable themes are always available. Unlocks open explanations, never functions (E5).
- Undo at once withdraws a current achievement and any pending celebration, but keeps explanations already unlocked and macros the user saved.
- Learning attainment, legal execution, current progress and exact completion are four different things and are never merged.

## 2. Encoding and the mathematical language (2A with 2B: R2b)

- One concept table lists, for every concept, its term, meaning, notation, exact ID, glyph, example and the level where it is first taught.
- Channels: colour carries a class from a proper colouring of the rings, never identity; position and fixed anchors carry identity and relations; glyphs carry the role in the current operation; patterns carry status; motion carries change, always with a static equivalent; exact IDs on focus resolve repeated colours.
- Focus: inside the focus set every channel is drawn; outside it only class colour. Protection conflicts and complete-operation effects always break through.
- Most exact language sits in a compact relationship strip inside Global and Local (destination, current occupant, wanted piece, buffers, frame, residual), linked to the geometric markers it names.
- The owner's conditions: the strip and the encoding must be **intuitive** (read at a glance, picture first), and the **mathematical grammar itself gets definitions and proofs**: every term, symbol and relation the strip or the encoding uses is defined in the theory book, with proofs or cited proofs of the facts it relies on (for example why a proper colouring needs only a few classes, why a buffer position differs from its occupant). This extends M4 to the UI's own grammar.
- Amended the same day ([owner-decisions-2026-10-02-scope](owner-decisions-2026-10-02-scope.md)): in 1.0 the theory book covers only what the UI shows, with proofs only for facts its correctness relies on.

## 3. Themes (3B with 3C: R3a)

- 1.0 ships **two authored theme families**, each with three scene looks (solving, inspecting, celebrating): six presets.
- Fixed across families: commands, keys, layout anchors, role glyphs, status meanings, comparison conventions, accessibility alternatives and motion meanings. A theme never changes projection or interaction scope.
- Families may vary surfaces, materials, approved typography, palette character and density within one region topology. Users tune within validated ranges. Each family's most expensive scene must pass the renderer gate.

## 4. Interests and nexus points (4A)

- Annotated liked and disliked reference pairs and nexus cards (interest, bridge to the project's structure or mathematics, product place). Agents extract transferable attributes only, never assets.
- Embedding-first discovery (4C) was not chosen. Taste Lab phase 1b keeps its own separate approval; this decision does not request or grant it.

## 5. Learning curve and keys (5A with 5C)

- Competence checks on fixture tasks matching the learning path, on fresh isolated data: independent completion, critical mistakes, help use, transfer to a new target; time reported separately from correctness.
- Plus local in-situ metrics on real solving (for example preparation time per star, context switches, undo rate), stored locally only, never uploaded.
- Keys open level by level as soft disclosure: "open" means shown in the key map and hints; the command palette always offers every command (E5); an expert can open all at once. Key-disclosure levels and reward levels are two different scales with distinct names.

## 6. Views (Global and Local only: R4a, reading 1)

- No third view. The two problems are the two view questions (D8): **Local**, does this piece match its target here; **Global**, how do these two places relate.
- Both views are visible and linked, sharing one context and revision. Local compares current and target frames with fixed anchors for target, piece and buffers; Global locates them in the actual structure.
- A residual summary and an operation summary are embedded in the views and expand in place; any edit of role, macro or protection invalidates the review; collateral shows even under display filters.

## 7. Entry (7D: R5a)

- One workspace and one path for everyone; no separate doors or profiles.
- Coaching follows the task the user chooses; explorers may stay with structural questions, solvers continue along the same path. Returning users resume their context directly; "show all keys" is always available.
- A practice copy (a fresh, clearly labelled full-model session) lets anyone scramble and play without touching their own solve.

## 8. Role priority (8B with one rule above it)

- First, the filter: an option that breaks the mathematical contract, protection, the human-solve boundary, the renderer gate, data safety or an owner design requirement is out.
- **Top priority:** when the engineer's evaluation finds something wrong that affects the work or correctness, it is fixed first. "Finds" means a demonstrated failure (a test, a measurement or a counterexample); an unmeasured concern becomes an experiment first, not a veto.
- Otherwise 8B's order among admissible options: core solver workflow (designer), then legibility and delight (artist), then cost beyond the gates (engineer), then notation preference beyond correctness (mathematician). The charter line "the core users' workflow decides trade-offs" stands.

## 9. Text (9B)

- Budgets by context, as trial values set by the G6 test: about 20 words of free sentences while solving, 50 in explicitly opened inspection, 80 in an explicitly opened learning panel. Each is a budget for the whole screen in that context, not per panel.
- Labels, numbers, exact IDs, notation, glossary terms and key hints are not free text. Error, protection and stale-state messages (E6) and the theory book are exempt. Every glossary term has a term card with a diagram rendered from the real geometry (M4, M6).

## 10. Sound (10B)

- The motion table (H-03) gets optional sound semantics. In G5 the owner compares short clips, silent against sounded. Whether sound is adopted is a separate owner decision after that audition.
- If adopted, one minimal cue set shared by all themes (for example "solved" and "invalid action") goes into the vertical slice, each cue with a visual equivalent and a mute; the full set follows after the slice. Sound never drives mechanics and never signals success for a rejected or stale result.
- Amended the same day ([owner-decisions-2026-10-02-scope](owner-decisions-2026-10-02-scope.md)): the audition is one short comparison against a two-cue set; if adopted, 1.0 has at most two cues and the full set moves to 1.x.

## Follow-up

- The tests named in both option pages (G1 to G6, the living test) are stage 2.3 work; they set the trial numbers above.
- The charter draft section 5 carries these decisions; the charter as a whole is still signed by the owner at the 2.0 exit.
