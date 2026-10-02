# Orbit-first block building: the solving workflow behind 1.0

Status: **draft, compiled by Claude from repository sources on 2 October 2026; checked by Codex against the sources the same day (all findings adopted); owner correction pending.** The owner stated that the only practical method for a complete 600-cell solve is orbit-based block building, and that the 1.0 toolset and workflow are built backward from it ([charter](charter-2.0-draft.md) section 5). This page describes that method as the repository records it, so that stage 2.2 can test every inventory item against a step of the method. It is not a new rule and adds no mathematical claim. The archived 0.4 contracts are used as a record of the owner's solving workflow, not as current instructions; the owner's 2 October decisions override them where they differ.

Sources: the [technical report](../../../research/Full_600cell_Technical_Report.tex) ("Seeds, collateral, and two execution modes", "Star operations and guarded setups", "Placement and orientation", "Preserving completed work", "Stage correctness and the role of collateral"), and the archived 0.4 design contracts: [product brief](../0.4/snapshot/design/01_PRODUCT_BRIEF_ZH.md), [abstract view spec](../0.4/snapshot/design/02_ABSTRACT_VIEW_SPEC_ZH.md), [architecture 0.4-R2](../0.4/snapshot/design/03_ARCHITECTURE.md) and the [integration and endgame contract](../0.4/snapshot/design/08_INTEGRATION_AND_ENDGAME.md).

## 1. The method in one paragraph

The puzzle is solved **one orbit at a time**, in an order that follows the collateral dependencies of the macros, so that every completed orbit stays solved at every macro boundary. Inside an orbit the solver fixes two buffer positions A and B and places pieces with **star** macros: the orbit's seed word, which cycles three pieces of the orbit, relocated to A, B and a reference position and conjugated by a setup that brings a target X into place without moving the buffers. The seed's complete action is kept, including any collateral on other orbits. Targets are grouped into blocks of nearby destinations (cell-local destination batching), which keeps the context and reduces navigation. When every non-buffer position is right, orientation is fixed with star pairs, then buffer A, then buffer B, whose residual is limited by the orbit's invariants and then corrected. Then the orbit is protected and the next orbit begins.

## 2. Across orbits: seeds, stage order and protection

- **Raw seeds and pure controllers.** Each orbit has a legal seed word whose effect on that orbit is a three-piece cycle. Twelve of the 35 raw seeds have no collateral; the other 23 also move pieces of other orbits, and a filtered view of the target cycle cannot tell which case applies. A **pure controller** is a raw seed followed by corrections of all its collateral through previously built pure controllers; purity concerns the net effect only.
- **The practical baseline uses raw seeds** in triangular order: each orbit is solved before the orbits its seed disturbs, so all collateral falls on unfinished orbits. Executing only the visible three-cycle while discarding the other effects would be a different, uncertified operation.
- **Stage order.** A directed edge o → j means the seed for o moves pieces of orbit j at its net end. Solving every o before every j with o → j keeps each completed orbit solved at macro boundaries (technical report, "Triangular stage invariant"). The certified order is printed in the report's census appendix; it is not the order of orbit IDs or ranks. Custom macros carry their own collateral and need their own analysis.
- **Boundaries, not intermediate steps.** Pieces of completed orbits move and return inside a long word. Protection therefore checks the net effect at the end of the complete operation (Net), or every primitive when the solver asks for it (Strict); an unchecked prefix is shown as unchecked, never as safe.
- An orbit becomes protected automatically only when it changes from unfinished to exactly solved during the solve; new and reset sessions are not blanket-locked, and a manual unlock is not undone on unchanged state.

## 3. Inside one orbit

### 3.1 Buffers and stars

- Two buffer **positions** A and B are fixed for the orbit. Their **occupants** change whenever a star runs; a buffer is a place, not a piece.
- A star for target X has the target action (A B X); its inverse acts as (A X B). It is setup⁻¹ · relocation⁻¹ · seed · relocation · setup, where the setup uses only caps that touch neither A nor B.
- Cost example from the report: the O33 seed is 46 primitives, 56 with its relocation; a target setup of depth d gives 56 + 2d moves. The report's guarded setup tables cover every oriented non-buffer state of every orbit (for O34, 118 × 60 = 7,080 states). Under the owner's boundary the program may propose such setups for a target the solver names (section 7).

### 3.2 Placement

For a wrong non-buffer destination X, let Y be where the piece that belongs at X is now:
- Y = A: use the inverse star for X;
- Y = B: use the star for X;
- otherwise: star for Y inverted, then star for X; it places the piece at X and keeps all completed non-buffer positions.

When all non-buffer positions are right, a lone swap of A and B cannot remain: each orbit's positional permutation is even.

### 3.3 Blocks and partial protection

- A block is an explicit achievement: members, intended positions or relations, a reference, progress conditions and a preservation policy chosen by the solver. Two kinds are recorded:
  - a **Home block**: identity and Home-position requirements with an exact slot and orientation condition, not matching face colours;
  - a **referenced block**: identity, position and frame relations in one verified common reference, describing a useful partial structure away from Home. It is not a claim that the structure can be carried around rigidly; that needs a separate proof.
- Building a block may disturb unprotected parts of the active orbit. Gains, losses and retained members are shown before execution. "Completed", "protected" and "selected" are separate facts.
- **Protection is finer than whole orbits:** a position lock keeps occupants and slots, an identity lock keeps the chosen pieces' location and orientation, a completed-block lock keeps the block's conditions, and a relative-block lock keeps its relations. For a protected orbit that is not yet solved, "preserve" means its captured state does not change. Example from the archived O33 walk-through: after inserting X into a two-member Home block, the solver protects the achieved X requirement before working on Y.
- After a successful insertion, the next target is the solver's locked Next if set; otherwise a candidate inside the current orbit that continues the current context, lies on the current block's boundary, matches one of the solver's existing macros, and needs no change of protection. The candidate is shown with its reasons; it is never executed automatically.

### 3.4 Orientation and the buffers (the endgame)

Suggested stages, not a forced sequence; returning is allowed, and new position errors change the displayed stage at once:
1. **Position finish:** remaining cycles, buffer occupants, Home.
2. **Non-buffer orientation:** correct pieces in place with orientation transfers P_X(q) = K_{X,q} K_X⁻¹, which fix positions and move the inverse orientation change to a buffer; both ends are shown.
3. **Buffer A:** full occupant and orientation, with its collateral on B checked.
4. **Buffer B:** only the final buffer can keep a residual, and the orbit's orientation group limits it: none for the trivial, C2 and C5 groups; inside the rotation subgroup for D5; for A5 a finite certified table covers the required residuals.
   - A **permitted nonidentity residual** (D5 rotation, A5 element) is not solved. The solver inspects the exact residual and its inverse, selects a correction macro, for example the report's final-buffer construction, the commutator [P_X(q), P_Y(r)] of two orientation transfers, which expands to eight stars and changes only that buffer's orientation on the target orbit, reviews its complete effect against every protected object, and executes it.
   - A **missing applicable macro or certificate** is shown as a gap, never as an automatic unlock or a generated solution.
   - An **impossible residual** (a lone A/B swap, a lone C2 flip or C5 twist) is diagnosed as a provenance, mapping or certificate problem; another orbit cannot compensate for it.
5. **Exact orbit completion,** confirmed by a completion check of every identity, position, orientation and sticker of the orbit, then protection. The whole puzzle is complete only when every label of every moving orbit and the required fixed structures is exact; empty graphs, missing candidates or matching colours are not evidence.

The current state is described per orbit by its residuals: non-buffer pieces not at Home, wrong buffer occupants, wrongly oriented pieces at Home and in buffers, objects whose frame is unknown, and exact solved status.

## 4. The solver's loop for one piece

Recorded in the 0.4 product brief as the core loop:

**choose target → look at the piece and the buffers → enter, record or choose a macro → bind roles and compose the operation → check star, frame, orientation and the complete effect → adjust one's own preparation → preview the whole operation → execute explicitly → check the result and the next work.**

| Step | What the solver needs to see | What the solver does | Tool support |
|---|---|---|---|
| Choose target | Residuals of the orbit, current block, locked Next, candidates with reasons | Picks a target or confirms the candidate | Residuals, explained candidates |
| Find piece and buffers | Where the target's piece is, A and B occupants, the cells involved, Home vs current structure | Looks; switches between following the piece and watching a fixed position | Piece-focused tracking, Local view |
| Relate the two places | Where the piece and the target are on the whole polytope, shared cells, distance | Orients themself | Global view (adjacency routes are navigation, not move sequences) |
| Choose or enter a macro | Macros of this orbit by use (star, pure cycle, orientation), with their effects | Picks, records or types a macro | Macro base, keyboard |
| Bind roles and compose | Piece, target, A/B roles, reference frame; Prepare, Macro and Cleanup phases | Fills the roles, composes the three phases (Cleanup may be the inverse of Prepare), inverts a selected segment | Work sheet, phase editor |
| Prepare (setup) | Proposed setups that keep the buffers fixed; grip, cells, legal twist directions, the key for each | Takes, edits or enters a setup, as a draft phase or as live turns | Setup search, Grip and Twist keys, onscreen keyboard |
| Check | Star or other cycle, its direction, frame changes, orientation, collateral, protection of the complete operation | Reads; edits preparation | Complete-effect analysis on all labels, protection check (Net or Strict) |
| Execute and review | Before and after residuals, protected damage, buffer change | Commits; undoes if needed | Preview, commit, undo, residual change |
| Save and reuse | The operation as a work sheet or a new macro | Saves it; later reuses it with a new target or reference | Work sheets, Macro base; every reuse gets a fresh review |
| Next | Next target, possibly another orbit with its saved context; or the endgame | Continues (section 4.2) | Per-orbit context, locked Next |

### 4.1 Rules of the loop

- **Draft and live input are separate.** In a draft, grips and twists append steps to the chosen phase and update a predicted view; the committed state does not change. A live twist is an immediate complete operation under the active protection. Temporary motion of protected pieces is allowed only inside a reviewed complete draft; a live turn cannot borrow the cleanup of a future draft.
- **The complete operation is what counts.** A macro body may conflict with protection while the complete Prepare / Macro / Cleanup preserves it, and the reverse; analysis and review always cover the complete operation, on all 259,800 labels, including filtered-out regions.
- **Permission is not progress.** A legal, protection-preserving operation may execute without completing the chosen goal; the goal is then reported as unmet. Preparation is never blocked for not improving the goal.
- **Reuse never reuses a review.** A saved work sheet keeps its roles, phases, reference and macro versions; reused with another target it is analysed again, and a role or target mismatch or a protection conflict is shown, never silently retargeted.

### 4.2 Branches

- **Candidates exhausted, residual left:** stay in the orbit and open the endgame (section 3.4).
- **Orbit finished, no Next:** show later-orbit suggestions and wait for the solver's choice.
- **Locked Next in another orbit:** it has priority; the current orbit's draft and settings are saved.
- **Return or resume:** drafts and choices are kept, the identities' current positions are resolved again, execution permission is invalidated, and the current global protection stays in force; an old orbit context never rolls protection back.
- **Interruption:** save, close, restore, undo and redo keep identity, state and history consistent.

### 4.3 Keyboard, filters and views

Recorded needs; the 1.0 layouts are redesigned (charter section 5), so these are requirements, not a layout.
- **Keyboard:** every program operation has a keyboard route; the physical turn editor has only Grip and Twist. The 0.4 design kept editable key sets per orbit for buffer A, buffer B, insertion and block extension, macro composition, and the endgame, plus a separate Functions set, with explicit switching. Hosting cells are an ergonomic start, not the full set of affecting caps (a two-cell piece can have 18 affecting caps), so every one of the 1,200 generators stays reachable. A key never changes meaning silently when the camera, a filter or the piece moves.
- **Filters and work sets:** three scopes stay separate: the full mechanical analysis, the solver's work and interaction filter, and noninteractive reference display. A saved set is a live query, frozen identities (members follow the pieces) or frozen positions (occupants change). A hidden buffer can remain visible as a reference for frame comparison without being pickable or twistable; making it interactive is a separate action.
- **Separate actions:** inspecting an object, assigning the target, assigning a grip and moving the camera are different actions; hovering only inspects.

## 5. Where the effort goes

The 0.4 brief names the costliest part as **buffer preparation and protection checks before each macro**: finding positions again, identifying several cells, resetting keys, and re-checking completed orbits for every piece. In the owner's own use, 0.4's functions could not be found or understood (charter section 5). No timing data exists yet; the 1.0 slice measures preparation-cycle time, mistakes, context switches and recovery after interruption instead of assuming them.

## 6. What this means for 1.0

- **Piece-focused context:** target, piece, buffers, block and protection stay set across every view and tool, so nothing is typed twice.
- **Keyboard first:** grip and twist keys around the current piece, with keys that never change meaning silently; every other command in the command table and the palette.
- **Local and Global, redesigned** (owner, 2 October 2026): Local answers "does this piece match its target here", Global answers "how do these two places relate". The 0.4 views are not the baseline.
- **Macro base by orbit and use,** with work sheets, and the complete effect drawn as graphics: cycles, sockets, occupants, orientation.
- **Protection always visible,** at the scope the solver chose; a conflict is never hidden by a filter.
- **Residual state per orbit** drives the stage display, candidates and completion.
- **Learning path:** the steps above, in this order, are the levels of the skill (charter section 5).

## 7. Boundary

The owner's human-solve boundary (charter section 5) applies to every row of section 4: tools may track, analyse, check, protect and reuse the solver's own macros and templates, and aid the view; the program never chooses or outputs a solving macro by itself, never executes without the solver's action and never solves automatically. **Setup search is allowed** (owner, 2 October 2026): for a target the solver names, the program may propose setups that keep the buffers fixed, such as those of the certified guarded setup tables in the technical report; the solver chooses the macro, may edit the setup and executes. Since setup preparation is the costliest repeated step (section 5), this is expected to save much of the preparation effort; how much is measured in the vertical slice, not assumed.

## 8. Open items

- Owner correction from actual solving practice, especially sections 4 and 5.
- Measured preparation-cycle costs from the vertical slice.
