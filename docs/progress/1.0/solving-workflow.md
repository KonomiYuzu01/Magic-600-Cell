# Orbit-first block building: the solving workflow behind 1.0

Status: **draft, compiled by Claude from repository sources on 2 October 2026; Codex verification against the sources and owner correction pending.** The owner stated that the only practical method for a complete 600-cell solve is orbit-based block building, and that the 1.0 toolset and workflow are built backward from it ([charter](charter-2.0-draft.md) section 5). This page describes that method as the repository records it, so that stage 2.2 can test every inventory item against a step of the method. It is not a new rule and adds no mathematical claim.

Sources: the [technical report](../../../research/Full_600cell_Technical_Report.tex) ("Placement and orientation", "Preserving completed work", "Stage correctness and the role of collateral"), and the archived 0.4 design contracts: [product brief](../0.4/snapshot/design/01_PRODUCT_BRIEF_ZH.md), [architecture 0.4-R2](../0.4/snapshot/design/03_ARCHITECTURE.md) and the [integration and endgame contract](../0.4/snapshot/design/08_INTEGRATION_AND_ENDGAME.md).

## 1. The method in one paragraph

The puzzle is solved **one orbit at a time**, in an order that follows the collateral dependencies of the macros, so that every completed orbit stays solved at every macro boundary. Inside an orbit the solver fixes two buffer positions A and B and places pieces with **star** macros: a pure seed that cycles A, B and a target X, conjugated by a setup that brings X into place without moving the buffers. Targets are grouped into blocks of nearby destinations (cell-local destination batching), which keeps the context and reduces navigation. When every non-buffer position is right, orientation is fixed with star pairs, then buffer A, then buffer B, whose residual is limited by the orbit's invariants. Then the orbit is protected and the next orbit begins.

## 2. Across orbits: stage order and protection

- Each orbit's seed macro has collateral on some other orbits. A directed edge o → j means the seed for o moves pieces of orbit j at its net end.
- If orbits are solved in an order that puts every o before every j with o → j, each completed orbit stays solved at macro boundaries (technical report, "Triangular stage invariant"). The certified stage order is printed in the report's census appendix; it is not the order of orbit IDs or ranks.
- Preservation holds **at macro boundaries**, not during a macro: pieces of completed orbits move and return inside a long word. Protection therefore checks the net effect (Net mode), or every primitive when asked (Strict mode).
- Custom macros carry their own collateral and need their own analysis; the certified order covers the reference seeds.
- An orbit becomes protected when it changes from unfinished to exactly solved during the solve.

## 3. Inside one orbit

### 3.1 Buffers and stars

- Two buffer **positions** A and B are fixed for the orbit. Their **occupants** change whenever a star runs; a buffer is a place, not a piece.
- A star for target X has the target action (A B X); its inverse acts as (A X B). It is built as setup⁻¹ · seed · setup, where the setup moves only caps that do not touch A or B.
- Cost example from the report: the O33 seed is 46 primitives, 56 with its relocation; a target setup of depth d gives 56 + 2d moves. The report's guarded setup tables cover every oriented non-buffer state of every orbit (for O34, 118 × 60 = 7,080 states).

### 3.2 Placement

For a wrong non-buffer destination X, let Y be where the piece that belongs at X is now:
- Y = A: use the inverse star for X;
- Y = B: use the star for X;
- otherwise: star for Y inverted, then star for X; it places the piece at X and keeps all completed non-buffer positions.

When all non-buffer positions are right, a lone swap of A and B cannot remain: each orbit's positional permutation is even.

### 3.3 Blocks

- A block is a set of target positions in one orbit, with their Home and frame conditions and a checkable completion condition. It is a working unit, not a rigid piece that can be carried around; that would need a separate proof.
- After a successful insertion, the next target is the solver's locked Next if set; otherwise a candidate inside the current orbit that continues the current context, lies on the current block's boundary, matches one of the solver's existing macros, and needs no change of protection. The candidate is shown with its reasons; it is never executed automatically.

### 3.4 Orientation and the buffers (the endgame)

Suggested stages, not a forced sequence:
1. **Position finish:** remaining cycles, buffer occupants, Home.
2. **Non-buffer orientation:** correct pieces in place by comparing a star with a star of another frame; the change moves to a buffer.
3. **Buffer A:** full occupant and orientation.
4. **Buffer B:** only the final buffer can keep a residual, and the orbit's orientation group limits it: none for trivial, C2 and C5 groups; inside the rotation subgroup for D5; for A5 a finite certified table is needed.
5. **Exact orbit completion,** then protection.

The current state is described per orbit by its residuals: non-buffer pieces not at Home, wrong buffer occupants, wrongly oriented pieces at Home and in buffers, objects whose frame is unknown, and exact solved status.

## 4. The solver's loop for one piece

Recorded in the 0.4 product brief as the core loop:

**choose target → look at the piece and the buffers → enter, record or choose a macro → check star, frame, orientation and the complete effect → adjust one's own preparation → preview the whole operation → execute explicitly → check the result and the next work.**

| Step | What the solver needs to see | What the solver does | Tool support |
|---|---|---|---|
| Choose target | Residuals of the orbit, current block, locked Next, candidates with reasons | Picks a target or confirms the candidate | Residuals, explained candidates |
| Find piece and buffers | Where the target's piece is, A and B occupants, the cells involved, Home vs current structure | Looks; switches between following the piece and watching a fixed position | Piece-focused tracking, Local view |
| Relate the two places | Where the piece and the target are on the whole polytope, shared cells, distance | Orients themself | Global view |
| Choose or enter a macro | Macros of this orbit by use (star, pure cycle, orientation), with their effects | Picks, records or types a macro | Macro base, keyboard |
| Prepare (setup) | Proposed setups that keep the buffers fixed; grip, cells, legal twist directions, the key for each | Takes, edits or enters a setup | Setup search, Grip and Twist keys, onscreen keyboard |
| Check | Star or other cycle, its direction, frame changes, orientation, collateral, protection | Reads; edits preparation | Complete-effect analysis, protection check |
| Execute and review | Before and after residuals, protected damage, buffer change | Commits; undoes if needed | Preview, commit, undo, residual change |
| Next | Next target, possibly another orbit with its saved context | Continues | Per-orbit context, locked Next |

## 5. Where the effort goes

The 0.4 brief names the costliest part as **buffer preparation and protection checks before each macro**: finding positions again, identifying several cells, resetting keys, and re-checking completed orbits for every piece. In the owner's own use, 0.4's functions could not be found or understood (charter section 5). No timing data exists yet; the 1.0 slice measures preparation-cycle time, mistakes, context switches and recovery after interruption instead of assuming them.

## 6. What this means for 1.0

- **Piece-focused context:** target, piece, buffers, block and protection stay set across every view and tool, so nothing is typed twice.
- **Keyboard first:** grip and twist keys around the current piece, with keys that never change meaning silently; every other command in the command table and the palette.
- **Local and Global, redesigned** (owner, 2 October 2026): Local answers "does this piece match its target here", Global answers "how do these two places relate". The 0.4 views are not the baseline.
- **Macro base by orbit and use,** with the complete effect drawn as graphics: cycles, sockets, occupants, orientation.
- **Protection always visible;** a conflict never hidden by a filter.
- **Residual state per orbit** drives the stage display, candidates and completion.
- **Learning path:** the steps above, in this order, are the levels of the skill (charter section 5).

## 7. Boundary

The owner's human-solve boundary (charter section 5) applies to every row of section 4: tools may track, analyse, check, protect and reuse the solver's own macros and templates, and aid the view; the program never chooses or outputs a solving macro by itself, never executes without the solver's action and never solves automatically. **Setup search is allowed** (owner, 2 October 2026): for a target the solver names, the program may propose setups that keep the buffers fixed, such as those of the certified guarded setup tables in the technical report; the solver chooses the macro, may edit the setup and executes. This removes most of the preparation work that section 5 names as the costliest step.

## 8. Open items

- Codex check of this page against the cited sources (Codex is not signed in in the current cloud session).
- Owner correction from actual solving practice, especially section 4 and section 5.
- Measured preparation-cycle costs from the vertical slice.
