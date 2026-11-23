# ADR-D01 revision 4: the solving surface (amendments to revision 3a)

Status: **Accepted** with revision 3a (owner, 23 November 2026, at A7; [freeze record](../../../../wiki/decisions/owner-decisions-2026-11-23-a7-freeze.md), [README](README.md)). Written by adr-line on 22 November 2026. The owner took every recommended option of section 7 on 22 November 2026 ("all as recommended"), so no item of this file is open.

These amendments are a separate file on purpose. The A5 build reads `ADR-D01-design-system.md` (revision 3a) as its spec while it runs, so that file is unchanged. The amendments are merged into it at A5's stop or in K2, whichever the owner chooses.

Where they come from: the owner's atlas of every 0.4 function (544 records), a coverage check against revision 3a (132 gap records, which are 38 functions), and one round of the design table with all four roles on 22 November 2026. The round record, with every card and the outcome, stays with the ADR line's working files.

Tags are those of revision 3a: (framework), (default), (tuned), (open).

## 1. New section 15: the solving surface

- **ADR-D01.61** The solving surface is part of the frame, not a set of panels. Every step of the core loop that recurs for every piece (name the Target, read where the Wanted piece is, set up, add the solver's macro, check, commit, go on) has a host key and a host surface and never waits on a script. A function lives in one of four places: a host element of this record; a built-in tool written in 600 Cell Script (LB6.4) and mounted through ADR-D01.30; a kernel read that a tool presents; or the solver's own script. A tool fills a host slot asynchronously. The slot shows one of six states: empty, pending, ready, stale, refused, budget-stopped. A commit that changes what a tool reads marks its slot stale (ADR-D01.26). (framework)
- **ADR-D01.62** A relationship strip sits inside the locked pair and carries the owner's six items (2 October 2026): the Target with its Occupant; the Wanted piece with where it sits now (the Target, Buffer A, Buffer B, or elsewhere with the position shown); Buffers A and B with their Occupants; the Target frame (Home or Captured); and the working orbit's Residual, which expands in place. Each item is linked to an anchor in Local and a marker in Global. Position anchors (Target, A, B) stay on positions; piece anchors (Wanted, Next) follow pieces. Target, A and B are distinct positions of the working orbit. Under Home the Wanted piece is derived from the Target and is drawn as derived; under Captured the capture names its revision. An ill-formed tuple is shown as invalid and is never corrected silently. The strip states facts ("Wanted is in A") and never names the macro or the star that follows from them (ADR-D01.4 item 6). (framework for the items, the anchors, the well-formedness rules and the boundary; the arrangement is default and is settled by the glance test of section 6)
- **ADR-D01.63** Next is a bookmarked piece, kept apart from the relation: a quiet mark at the strip's end and a glyph on its orbit's rail cell. One key each sets, clears and activates it. It may lie in another orbit; activating it saves the present orbit's draft and settings. (default)
- **ADR-D01.64** An orbit rail sits on the body of the locked pair, never in a dock: one cell for each of the 35 orbits. A cell carries the orbit id (O00 to O34), the orientation-group glyph, a residual level, a solved mark, a lock with three values (none, partial, whole orbit), the working mark and the Next glyph. A click selects the working orbit; so does a held orbit key followed by two digits. A cell expands in place into the orbit's exact residual classes (positions not at Home, wrong buffer occupants, orientation errors, unknown frames, exactly solved; in the endgame the last buffer as identity, permitted or impossible) and the list of unfinished positions of ADR-D01.67. Numerals are exact, and "unknown" is drawn apart from zero. The order of the cells never depends on the state. No cell gains emphasis from being the first unfinished one, nothing on the rail moves at idle, and a completion moment never travels toward another cell. On hover a cell shows the orbits its raw seed touches and the orbits whose seeds touch it; these static facts are never lit from the state. Solved, protected and working are three separate marks. The default order is the seed stage order, labelled on the rail as one admissible order, and orbit-id order is a setting. The residual level is drawn as a length (the grammar's rule that length carries quantity), and the exact numerals are in the expansion. (framework for the state-independent order, the three-valued lock, the exact classes and the no-emphasis rules; grouping and orientation default; default order and residual level: owner, 22 November 2026)
- **ADR-D01.65** Protection is always visible. Stickers carry status patterns with five meanings: protected, conflict (which breaks through, ADR-D01.4 item 8), unchecked prefix, preview, stale. The rail carries the locks. A check-mode chip on the commit bar shows Net or Strict together with passed, failed or unchecked, and never one "safe" lamp, because a word can pass Net and fail Strict. Protection is set by a pick and a key, or on a rail cell's lock; its kinds (whole orbit, position, identity, block) are distinct marks. (framework for the meanings; patterns tuned; how many statuses get their own scene pattern is measured in gate batch 1)
- **ADR-D01.66** The commit bar is also the draft line. It shows the operation draft in three segments, Prepare, Core and Cleanup, with the active phase marked; the Draft/Direct switch on its left cap; a Goal slot; the check-mode chip on its right cap; and a settling lamp while the settle interval of ADR-D01.9 runs. In Draft, grip and twist keys append to the active phase and the preview rebuilds live; in Direct a twist is one complete operation (ADR-D01.13). The switch also sets the pattern of the grip marker in Local, and switching clears held keys. The Goal is a predicate the solver declares; a read-side check evaluates it over the head and the staged effect; its result (unset, met, unmet, unknown) has its own slot, apart from the commit status, and never gates a commit: permission is not progress. The host draws the draft from the `operation.phase.*` rows; the staged `.6cs` statement of ADR-D01.11 is its projection. (framework for "the Goal never gates", for the route being always visible and for the complete operation being what is checked; bar layout default)
- **ADR-D01.67** The solver names the Target by a pick and a key, by the radial or context menu, or from the keyboard alone by stepping through the working orbit's unfinished positions and through the members of the solver's own block. The list of unfinished positions is in position-id order. It carries no reasons, no ranking and no macro match, and an empty list is never evidence of completion. (framework for the neutral order; keys default)
- **ADR-D01.68** A setup slot sits under the Target cell and is filled by the built-in setup search, the one solving-side exception of LB1.5. The solver names the target (one key may name the Wanted piece's present place); the alphabet excludes the caps that move the buffers; rows are in a neutral order with their length shown, and a length is called minimal only within the guarded alphabet; the solver picks a row and the host submits that word to the preview; a row fills Prepare and Cleanup around the macro the solver placed, or previews the setup alone, and never a whole placement; an empty answer names the setup family that was examined. The tool ships only with its BoundaryClass and its PrincipleRecord. (framework)
- **ADR-D01.69** A Library dock on the left holds the solver's records in one tree: macros, Templates, sets, blocks and saved states, with a Built-in branch for the shipped tools. A row leads with glyphs and the effect signature; an input box filters in place. Enter adds the selected macro to the active phase; Shift+Enter previews it alone. A Template is a macro with typed parameters; parameters typed as roles (Target, Buffer A, Buffer B, Target frame) bind to the strip by default, so nothing is typed twice. A Template reused with another target is analysed again and never reuses a review. Sets are of three kinds with three glyphs: Query set, Piece set, Position set. A block is a Library record drawn in the views as an outline (Home or referenced) with separate marks for attained, protected and selected; adding the present piece to a block and removing one are host keys. (default; re-analysis on reuse framework)
- **ADR-D01.70** One effect signature carries a macro's facts everywhere: 35 slots in the rail's order with one glyph per affected orbit, taken from the complete effect with its evidence status. It appears in Library rows, in the comparison tool, in the preview view and in History entries. No use is scored or guessed; without a certificate an effect reads as unverified. (default; the meaning framework)
- **ADR-D01.71** The top bar has a Timer cell (time and a running lamp, one key to start or pause; it certifies nothing and does not sit beside a completion) and a Session cell that shows lamps until it is expanded in place into counts, the log head, the revision and state hash, and the session, model and build identity. A practice copy is marked on every view border. (framework for the practice mark; cells default)
- **ADR-D01.72** A completion is stated only after the exact completion check of every label of the orbit. It is written as a completion entry in the History dock, which also holds the slot for the first-completed-orbit moment; the bookkeeping needs no acknowledgement. (framework for the claim; drawing default)
- **ADR-D01.73** Help is a card at the cursor: F1 opens the term under the cursor with a real geometry figure, within the inspection text budget, and links to the Learn layout. The Solve layout has no standing help panel. (default)
- **ADR-D01.74** The shortcut sheet of ADR-D01.17 is the key map: drawn as a keyboard, shown as an overlay while its key is held and pinnable, it shows what each key does in the hovered view's context and which key set is active. A key never changes meaning silently. Keycaps and the Grip frame share one set of ordered marks. Key sets per orbit are keymap presets, not a mode. (framework for "never silently"; drawing default)
- **ADR-D01.75** The selected grip carries Grip frame marks: the ordered, proper corners of the gripped cap's cell, which decide what each twist key means. The radial menu changes the frame. (framework)
- **ADR-D01.76** The shipped built-in tools, with the names the solver sees: Residuals, Protection check, Setup search, Macro facts, Compare macros, Cycles, Structure. The long names of LB6.5 stay as module and record titles and as palette synonyms. Each mounts through ADR-D01.30 and fills a host slot or a panel in the Analyse layout. Cycles draws a kernel read; its states are Before, Operation and After. Orientation parameters are typed as elements of the orbit's group, a tool evaluates the solver's own values and never solves for them, and parameters are named by role. No tool ships before its PrincipleRecord exists. (framework for the boundary; the list default)
- **ADR-D01.77** The frame is a canvas in two layers. Surfaces are for the design and art line to paint: the shell, the grounds of the strip, the rail, the top bar and the docks, the view border material, the key map's body, the figure plates of help cards and the Learn layout, empty states, and the completion moment. Instruments keep a meaning fixed by name in every theme: strip cells and anchors, rail cells, status patterns, locks, lamps, chips, the Draft/Direct mark, the residual level, Grip frame marks, the commit status, numerals, arrows, orientation glyphs, set-kind glyphs and block marks. An arrow's tail is the source and its head the destination; state arrows and operation arrows differ; there is one orientation glyph kind per group type (trivial, C2, C5, D5, A5). The number of values and the meaning of an instrument are framework; its drawing is the art line's. New elements live on the frame's body (the pair's shared edge, the view borders, the top bar) and add no dock. (framework)
- **ADR-D01.78** The frame gives no surface to these 0.4 functions: a suggestion of the following orbit, candidate lists with reasons, likely-use scoring of macros, the Word A operand, continuation planning after a commit, and acknowledging the completion summary. (framework; the owner signed the six cuts on 22 November 2026)

## 2. Amended items of revision 3a

- **ADR-D01.4 item 4** becomes: "Every script-drawn area carries a provenance mark that the host draws from the run's recorded author class; a script can neither declare nor hide it. A built-in tool whose module digest matches the shipped lockfile carries the mark alone. The solver's own scripts, forks of built-in tools included, carry the mark and a thin border. Packages written by others, and modules of unknown origin, carry the mark and a patterned border. One key opens the source of any script-drawn item. Commit controls are drawn only by the host." (framework; owner, 22 November 2026, option (c) of the round)
- **ADR-D01.4 item 7** gains the instruments of ADR-D01.77 next to "the relationship-strip classes". The encoding grammar gains one rule: length carries quantity. Its one use at A7 is the rail's residual level (ADR-D01.64); the theory book defines it with the rest of the grammar. (owner, 22 November 2026)
- **ADR-D01.8**, second and third sentences, become: "Sources: the solver's typed twists and own macros, a .6cs preview program, a library macro, and a built-in tool. A built-in tool computes only functions of input the solver supplied: the inverse, a power, the concatenation and the commutator of the solver's words; the conjugate by a word the solver supplies; the transport by a geometric symmetry the solver names (a rotation or a mirror, fixed by an ordered corner frame at the source cell and at the destination cell), rewritten letter by letter into legal generators and checked afresh against protection; a part the solver marks; or a single twist the solver specified. It never chooses a solving word. There are two exceptions (LB1.5), each a declared BoundaryClass: the setup search for a target the solver names (ADR-D01.68) and, in the jumbled puzzle, the repair list at a cap the solver names (ADR-D02). A scramble is a seeded word with recorded provenance that the solver stages and commits as a session action; it is never offered as a solving step." The rest of the item is unchanged.
- **ADR-D01.11** refers to ADR-D01.66 for the draft line, the Draft/Direct switch, the Goal slot and the check-mode chip; "settling" joins the statuses.
- **ADR-D01.13** refers to ADR-D01.75 for the Grip frame marks and uses "Direct" for the twist route throughout.
- **ADR-D01.15** gains completion entries (ADR-D01.72) and the effect signature on entry headers (ADR-D01.70).
- **ADR-D01.19** refers to ADR-D01.74 for the key map and gains the typed orbit jump (ADR-D01.64).
- **ADR-D01.22** Selection facts name their content by selection kind; for an orbit they are the orbit catalogue (definition, orientation group, seed witness with its collateral), opened from the rail cell. The View section names projection, shrink and frame visibility.
- **ADR-D01.24** pinned cards include the Tracked piece: a card that follows a piece.
- **ADR-D01.28** lists the transforms as ADR-D01.8 does.
- **ADR-D01.30** a mounted tool that fills a host slot uses the six slot states of ADR-D01.61.
- **ADR-D01.31** the canvas primitive gains: scene-anchored marks drawn by the core (a glyph at a slot or piece, an arrow between slots, an outline of a slot set) with instance ceilings; picking from a mark back to its `.6cs` line; encoding roles by name (role glyphs, status patterns, class colours, motion meanings) beside design tokens; a host graph layout; a declared basis for every drawing (projection, schematic or table), shown on hover; the exact-numeral widget; and text through type role tokens only. A built-in tool passes the same token lint as product code; other scripts may use free values under the non-standard mark of ADR-D01.51.
- **ADR-D01.36** in the Solve layout the left dock is the Library, the right dock is History with the inspector on demand, and the bottom dock is collapsed; the editor column belongs to the Script layout, and Solve keeps the one-line command bar.
- **ADR-D01.36 to ADR-D01.39** the named arrangements are called layouts, not workspaces: "workspace" already means the solver's working context in the command table and the solver's module store in LB7. The shipped layouts are Solve, Script, Analyse and Learn. (owner, 22 November 2026)
- **ADR-D01.46** a focus layer is a predicate (a built-in class, a set, or an `=` expression) with a display state; its count shows beside it. The 0.4 Boolean filters are focus layers.
- **Throughout:** "frame" keeps its mathematical meanings (Frame, Target frame, Grip frame). The chrome says shell, view border and script mark.

## 3. Vocabulary

One term per concept. Every term below is in the glossary or enters it before it reaches the screen (anti-goal 3). Words marked (owner) are the owner's picks of 22 November 2026.

| Was | Now |
| --- | --- |
| Destination | Target (owner) |
| Current (piece) | Wanted (owner) |
| (none) | Occupant |
| Current (state) | Before; with Operation and After |
| Frame (strip); Home, captured | Target frame: Home or Captured |
| corner marks | Grip frame marks |
| policy chip | Check mode chip: Net or Strict |
| draft and live input | Draft and Direct (owner) |
| Prepare, Macro, Cleanup | Prepare, Core, Cleanup (owner) |
| work sheet, method | Template (owner) |
| filter | Focus layer |
| named set | Query set, Piece set, Position set |
| tracked identity | Tracked piece |
| layer shells | Shells |
| endgame family | Orientation transfer; Final-buffer commutator |
| workspaces; Reference | Layouts; Learn (owner) |
| help and reference panel; reference states | Help card; Saved states |
| Library | Library (owner; Collection and Repertoire were proposed) |
| completion receipt | Completion entry |
| keyboard sheet | Key map |
| Buffer A, Buffer B, Next, Goal, Cycle, Residual, Net, Strict | unchanged |
| candidate, Word A | leave the vocabulary |

New glossary entries: Relationship strip, Working orbit, Wanted, Next, Target frame, Captured, Grip frame, Goal, Operation and its phases, Template, Draft and Direct, the three set kinds, Tracked piece, Focus layer, Shell, Cycle, Exactly solved, Final-buffer commutator, Practice copy, Completion entry, Key map, Layout.

## 4. Slot names for A7

Round 1 freezes names, not values. Not fixed at A7: glyph drawings, shapes, colours, sizes, the rail's orientation and grouping, the strip's arrangement, the number of scene patterns, transparent overlays and text tied to stickers.

- `strip.ground`; `strip.cell.{target, occupant, wanted, buffer_a, buffer_b, target_frame, residual}` with states unset, set, derived, match, mismatch, stale, invalid, selected; `strip.link`; `strip.next` with states unset, set, other_orbit.
- `anchor.{target, buffer_a, buffer_b}` (on positions); `anchor.{wanted, next}` (on pieces).
- `rail.ground`; `rail.cell.{id, group, residual_level, solved, lock, working, next}`; `rail.lock.{none, partial, whole}`; `rail.status.{open, solved, permitted_residual, unknown_frame}`; `rail.moment.complete`.
- `pattern.{protected, conflict, unchecked, preview, stale}`; `protection.kind.{orbit, position, identity, block}`.
- `chip.check_mode.{net, strict}` with results passed, failed, unchecked.
- `mark.route.{draft, direct}`; `draft.phase.{prepare, core, cleanup}`; `commit.status.{ready, impossible, stale, refused, settling}`; `commit.goal.{unset, met, unmet, unknown}`.
- `mark.practice` on `view.border`; `cell.timer`, `lamp.running`; `cell.session`, `lamp.revision`.
- `history.entry.completion`; `signature.slot`; `library.row.{kind, signature}`; `set.kind.{query, piece, position}`.
- `block.outline.{home, referenced}`; `block.mark.{attained, protected, selected}`.
- `arrow.{state, operation}`; `orientation.glyph.{trivial, c2, c5, d5, a5}`; `grip.frame_mark.n`.
- `keymap.key.{bound, unbound, held, out_of_context, twist}`; `card.figure`.
- `provenance.{built_in, own, other}`; `slot.tool.{empty, pending, ready, stale, refused, budget_stopped}`.
- Motion rows (H-03) and cue ids (ADR-D01.56) for: a strip change, an orbit completion, a Next activation, a lock set and released, a Draft/Direct switch.

## 5. Where each 0.4 function lives

| Group | Function in 0.4 | Now | Item |
| --- | --- | --- | --- |
| G01 | Current piece and destination | host | .62 |
| G02 | Next bookmark | host | .63 |
| G03 | Buffers A and B | host | .62 |
| G04 | Target frame | host | .62 |
| G05 | Goal declaration | host + tool | .66 |
| G06a | Working-orbit selection | host | .64 |
| G06b | Orbit residuals and completion | host + tool (Residuals) | .64 |
| G06c | Orbit catalogue | merged into the inspector | .22 |
| G07 | Following-orbit suggestion | cut | .78 |
| G08 | Explained candidate lists | cut; the position-id list stays | .67, .78 |
| G09 | Protected sets and Net/Strict | host | .65 |
| G10 | Block requirements | host + tool (Protection check) | .69 |
| G11 | Operation draft in three phases | host + tool | .66 |
| G12 | Draft or live input | host | .66 |
| G13 | Work sheets | merged into Templates | .69 |
| G14 | Macro library | host | .69 |
| G14i | Effect of the selected macro | tool (Macro facts) | .69, .70 |
| G15 | Macro comparison | tool (Compare macros) | .70, .76 |
| G16 | Likely-use scoring | cut | .70, .78 |
| G17 | Endgame families and parameters | merged into Templates and help | .76 |
| G18 | Cycle inspection | kernel read + tool (Cycles) | .76 |
| G19 | Entry and sticker correspondence | host | .62 |
| G20 | Buffer and setup analysis | host + tool (Setup search) | .68 |
| G21 | Boolean filters | merged into focus layers | .46 |
| G22 | Named sets | host | .69 |
| G23 | Tracked identity | merged into pinned cards | .24 |
| G24 | Word A operand | cut | .78 |
| G25 | Timer | host | .71 |
| G26 | Session readouts | host | .71 |
| G27 | Completion record | host | .72 |
| G27b | Acknowledging the completion summary | cut | .72, .78 |
| G28 | Continuation planning | cut | .63, .78 |
| G29 | Help and terms | host | .73 |
| G30 | Structure explorer | tool (Structure) | .76 |
| G31 | On-screen keyboard | host | .74 |
| G32 | Key banks per orbit | merged into keymap presets | .74 |
| G33 | Ordered frame on the grip | host | .75 |
| G34 | Grip review layout | merged | .74, .75 |

Host 16, host + tool 5, tool 3, kernel read + tool 1, merged 7, cut 6.

## 6. Experiments before A7

1. **Glance test in Look Lab.** Frames from fresh data, shown briefly, at compact density, in both families, under colour-vision-deficiency simulation and in high contrast. It compares strip arrangements (cells or a relation figure; on the seam or one per view), rail groupings and protection encodings. Questions: the working orbit; the locked orbits; where the Wanted piece is; Home or Captured; Draft or Direct; and "which orbit does the screen suggest next?", whose required answer is "none". Pass bar (owner, 22 November 2026): at least 90% correct on one two-minute card.
2. **Truth round trip, headless.** Render the strip and the rail on fixture states (mid-solve and two endgames), decode them by the grammar, compare with the kernel's residuals. Any mismatch fails.
3. **Residual latency.** `orbit_residuals` for all 35 orbits through the runtime child after a 20-move and a 1,000-move scramble. The rail is current within one frame of the commit display and stays off the key path. A failure adds a kernel read under PHS.52 and keeps the tool as the viewable reference.
4. **Gate batch 1 additions.** The protection pattern alone; the anchors during the fastest camera rotation; four full-detail views; the full chrome; a pick test with the strip and the anchors overlaid.
5. **Anchored arrows.** Cycle arrows for a 7,200-piece orbit during a twist, anchored against screen-space.
6. **Living test.** Count commits refused inside the settle interval during fast solving.

## 7. Owner decisions (22 November 2026)

The owner was given seven items with a recommended option each and answered "all as recommended": every recommended option. The options not taken stay on record here.

1. **The six cuts of ADR-D01.78: signed.** Two of them (the following-orbit suggestion and candidate lists with reasons) overrule `docs/progress/1.0/solving-workflow.md` (section 3.3's candidate "shown with its reasons" and the "later-orbit suggestions" at the orbit's exit) and the glossary's UI lines for Target and Stage order. The integrator corrects those pages.
2. **The provenance mark rule of ADR-D01.4 item 4: the text of section 2.** Not taken: built-in tools carry no mark at all; every script-drawn area keeps one identical frame.
3. **The rail's default order: the seed stage order,** labelled as one admissible order, with orbit-id order as a setting. Not taken: orbit-id order by default.
4. **The encoding grammar gains "length carries quantity"** for the residual level, with exact numerals in the expansion. Not taken: exact numerals only.
5. **The words of section 3:** Target, Wanted, Draft and Direct, Core, Template, Layouts and Learn, Library.
6. **The glance test's pass bar:** at least 90% correct on one two-minute card.
7. **Three additions join the one look-cost envelope revision that follows A7:** a per-slot status channel with the protection pattern in the built-in chain; screen positions or core-drawn glyphs for the Local anchors; and an envelope that counts up to four full-detail views, not three states.

These decisions accept no ADR and freeze nothing: ADR-D01 freezes at A7, item by item.

## 8. Leads for other lines

- **Command and engine line.** Rows with no server yet that the surface needs first: `target.current.set`, `target.next.*`, `buffer.assign`, `orbit.switch`, `input.destination.set`, `intent.set`, `operation.phase.*`, `session.timer.*`. A kernel read for cycles. A `residual_counts` read if experiment 3 fails. A PrincipleRecord for each built-in tool; a BoundaryClass for Setup search. Build order: host rows, residual and completion, protection and block, macro facts and comparison, setup search, cycles and structure.
- **Command ids that read wrong against the vocabulary** (not renamed here): `input.destination.set`, `target.current.set`, `grip.anchor.set`, `intent.set`, `orbit.switch`, `protection.policy.set`, `view.residuals` (its kind), `view.cycle.mode`, `filter.*`, `endgame.*`. Rows left without a surface: `view.recommendations`, `macro.operand.set`, `worksheet.*`.
- **Glossary.** The UI lines of Target and Stage order; the Work sheet entry; one form for orbit ids; the new entries of section 3.
- **Renderer.** Scene-anchored marks and the three envelope additions; any chrome node inside a view lets picks through.
- **A5.** The running build is not interrupted. At its stop, the strip, the rail, the draft line and the key map are the first elements to build, because the glance test needs them.
