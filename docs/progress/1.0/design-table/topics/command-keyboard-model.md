# DT-M1: Command model and keyboard model

Status: **open.** Opened on 22 October 2026 by the main session under the [round protocol](../README.md), as the second design-table topic of the [re-plan](../../replan-2026-10-22.md) (section 4). The card limits apply from this topic on: at most six cards per role in round 1, and at most four in round 2, each answering other cards by ID (owner, 22 October 2026: [decision](../../../../wiki/decisions/owner-decisions-2026-10-22.md)).

## Question

How does a solver reach and run every function of 1.0, and what do the keys do? The round settles the design side of H-08 final:
- the **command model**: which commands exist, how coarse they are, what their arguments name, where they are available, how they commit and undo, and how the command palette offers them;
- the **keyboard model**: how keys bind to commands, how a grip and a twist are typed, how keys open level by level, which keys may repeat, and how a solver changes the keys.

These are the 2.3 items `command-model` and `keyboard-model`. After the round, EN4 builds the final command table through `build_command_table.py`, and G6 takes the keyboard model into the styled layouts. The final letters on the keys are G6's, with the owner.

For each item below, the round settles one of three answers:
- allowed as stated;
- allowed under a stated condition;
- out.

Anything the records do not settle becomes an owner question.

## Inputs at a glance

- **The command table** is the H-08 draft of 1 October: [command-table.md](../../command-table.md) and `command-table.json`.
  - It has 86 commands: 32 workspace, 27 view, 12 query, 11 commit, 2 file and 2 app.
  - It was built from the 72-row top-down inventory, which was written from documents only (`inventory/top-down.json`).
  - Every row is still `pending-2.2`.
- **The 2.2 dispositions** were signed on 3 October ([dispositions.md](../../dispositions/dispositions.md)).
  - 503 units carry 556 decisions: keep 52, redesign 486, delete 12, automate 6.
  - 377 of the keep and redesign decisions belong to units (331 of them) that had a 0.4 user entry point: a key, a button, a menu, a command ID, a mouse action or a control.
  - Keep and redesign decisions by flow: F-NAVIGATE 165, F-COMPOSE 137, F-CHECK 132, F-SESSION 124, F-FIND 118, F-REUSE 70, F-MACRO 64, F-LEARN 63, F-RELATE 62, F-EXECUTE 56, F-ENDGAME 54, F-TARGET 48, F-PLATFORM 45, F-PROTECT 37, F-MIGRATE 12. A decision can serve several flows ([flows.json](../../dispositions/flows.json)).
- **No join exists between the two.** Command rows cite top-down rows (T-xx), and dispositions cite units (U-xxx). The table announced two updates: the code-only functions on day 3, and a disposition for each command on day 4 (command-table.md, status line). Neither was made, so no command has a disposition yet. The join is engineering work for EN4; this round sets the rules it follows (item C1).
- **0.4's keys.**
  - The units hold 1,689 key entry points.
  - 0.4 grouped its keys into sets that the solver switched explicitly: editable sets per orbit for buffer A, buffer B, insertion and block extension, macro composition and the endgame, plus a Functions set and sets such as Workspace, Operation, Macro, Views, Keyboard, Cycles, Session and Filter ([solving-workflow.md](../../solving-workflow.md), section 4.3; the inventories' key entry points). A set chooser and a command index sat on F2 and F1.
  - The charter records the result: "the functions under Command All could not be found or understood, and even a simple operation was hard" ([charter](../../charter-2.0-draft.md), line 49).
- **Grips and twists.**
  - 600-cell-Full has 600 caps, one per cell. Each cap turns by the 11 rotations of its tetrahedral group: three half-turns and four axes of third-turns, each third-turn in two directions. All of them are words in the cap's two generators of the 1,200 (`grips.py`, lines 1 and 9; [glossary](../../glossary/glossary.md): Cap, Generator, Grip, Twist).
  - A two-cell piece can have 18 affecting caps (solving-workflow.md, section 4.3).
  - With Jumble on under I_a, each grip has 59 menu twists besides the identity: its 11 retained twists and 48 jumble twists (`research/jumbling/fixtures/wj-I_a.json`, menu base of 60 items). *Corrected after round 1 (cards A6, E6, M4 and M5); the record first said 59 jumble twists.*
- **Jumble rows.**
  - DT-J1 settled that each admitted aid is one row whose arguments name the solver's object, and that both puzzles share one macro base and one key set ([DT-J1](jumble-solver-aids.md), Outcome: "E1" and "Command rows").
  - ENG-0J revision 2 proposes the rows ([ENG-0J](../../packets/engineering/ENG-0J-jumbling-backend.md), lines 145 to 149).

## Items

**C. The command model**
- **C1. The command set and its join to 2.2.** Proposed rules for H-08 final:
  - one command per purpose, whose arguments name the object (a cap, a piece, an orbit, a macro, a phase), rather than one command per 0.4 control or per key of a set;
  - every keep or redesign unit with a user entry point maps to a command or to a recorded "not a command" (engine internal, view-model refresh, storage policy);
  - automate and delete units get no command;
  - a command takes the dispositions of its units.

  Are these the right rules, and at which grain? For example, is selecting a grip one command with a cap argument?
- **C2. Contexts and focus.** The draft has eight contexts: `any`, `puzzle`, `solve`, `operation`, `keyboard`, `filter`, `macro_base` and `editor`. 1.0 has one workspace and two linked views that are always visible (owner, 2 October: sections 6 and 7). What decides which keys are live:
  - the panel that has keyboard focus;
  - a mode, such as draft input against live input, or Jumble on against Jumble off;
  - or both?

  A key in `editor` context never reaches a puzzle command (command-table.md, section 4).
- **C3. The command palette (E5).** The palette always offers every command (owner, 2 October, section 5). Open points:
  - whether it lists a command that the engine says is unavailable, with the reason, or hides it;
  - what it searches: labels, glossary terms, command IDs;
  - whether each entry shows its key, so that the palette teaches the keys;
  - whether it takes arguments, such as a cap ID;
  - whether a commit run from the palette commits at once or opens its preview.
- **C4. The commit path.** "The commit takes one action" (S4), from a fresh preview. A live twist is an immediate complete operation under the active protection, and draft input appends to a phase without changing the committed state (solving-workflow.md, section 4.1). Open points:
  - how the solver switches between draft and live input, and how the screen shows which one is active;
  - which key commits a preview, and when that key is live;
  - whether destructive commits (new solve, reset, checkpoint restore, log import) need a confirming key beyond their `confirm` gate.
- **C5. Undo and jobs.**
  - Undo: the draft has two undo classes, `journal` for the puzzle and `workspace` for drafts, protection, macros and keys. Options: one key that undoes the latest action of either class; two keys; or a key whose class follows focus.
  - Jobs: checks, previews, the buffer analyser, Jumble's covers and repair lists are cancellable jobs, with one `job.stop`. Open points: which key stops a job; what happens to a running job when the revision changes; and what one key such as Escape means when a job runs, a panel is open and a selection is set.
- **C6. Jumble rows.** One key set serves both puzzles (DT-J1, E1). Open points:
  - whether Jumble on and Jumble off are plain commands or need a confirming key;
  - whether Jumble on is a mode under which the grip and twist keys type jumble twists, or the jumble twists get keys of their own;
  - how the 59 menu twists of a grip are typed, beside the "axis and angle" input of ENG-0J;
  - where the query rows of DT-J1 sit: the release check, the word check, ν, the correction-set check, the straddler set, covers and the repair list.
- **C7. The draft table's open points and 0.4's key-set commands.** Points 2 to 4 of command-table.md section 5 (point 1 is part of EN4's join, and point 5 is out of scope):
  - whether the timer writes to the journal;
  - whether the structure commands from 0.3 documents stay;
  - whether `twist.apply` into a draft becomes its own command.

  Also, whether `bank.select`, `bank.functions.toggle`, `grip.capture` and `input.destination.set`, which serve 0.4's key sets, survive the keyboard model of part K.

**K. The keyboard model**
- **K1. The model.** The charter's reference is the Hyperspeedcube keybind workflow, at the efficiency of an office application used from the keyboard alone (line 49). Candidate families, which can be combined:
  1. a fixed global map with modifier shortcuts, plus the palette;
  2. a grip-and-twist block for puzzle input, with other commands on modifiers, sequences or the palette;
  3. switched key sets, as in 0.4;
  4. prefix sequences: one key opens a group, and a second key picks the command.
- **K2. Typing a grip and a twist.** Every one of the 1,200 generators stays reachable, and a key never changes meaning silently when the camera, a filter or the piece moves (solving-workflow.md, section 4.3). Open points:
  - whether grip keys are relative to the current piece, re-mapped when the piece changes with the change shown and applied by the solver (the owner's 0.4 brief, below), or absolute, with a cap typed by ID or picked by pointer;
  - how the 11 twists of a cap map to keys (seven axes, each forward and inverse, as 0.4 drew them; `grips.py`, line 22);
  - how the screen shows what each grip and twist key will turn.
- **K3. Disclosure levels.** Keys open level by level, "open" meaning shown in the key map and hints. The draft levels are:
  - L0 grip, twist and undo;
  - L1 inspect and isolate;
  - L2 macro entry and star;
  - L3 setup search and work sheets;
  - L4 blocks and locks;
  - L5 endgame;
  - L6 custom key maps.

  Sources: [design-options-2026-10.md](../../design-options-2026-10.md), line 210, chosen with 5C by the owner. Open points: check the levels against the steps of the solving workflow and against the Jumble rows; name the scale apart from the reward scale; and settle how "show all keys" and "open all" work.
- **K4. Repeat, hold and text input.** Key repeat never repeats a state-changing command (R-18). The owner's 0.4 brief also asks the program to handle text input, IME, key repeat, a grip held while the window changes, a lost key-up, the busy state and key conflicts: a letter typed in a macro text box never twists, and Enter in Global never commits the main preview. Open points:
  - which keys may repeat: camera, list movement, preview scrubbing;
  - whether a grip is held while its key is down, as in Hyperspeedcube, or toggled;
  - what happens to held keys when focus is lost.
- **K5. Physical keys and layouts.** 0.4 bound physical key codes such as `KeyY`, `Digit7` and `Backquote` (the inventories' key entry points). Hyperspeedcube 2 tells physical from semantic keys ([reference audit](../../../0.4/snapshot/design/06_REFERENCE_AUDIT.md), line 31). Open points:
  - whether keys bind by position, so that spatial blocks survive other layouts and the labels follow the user's layout, or by character, so that mnemonics follow the layout;
  - what the on-screen keyboard is for;
  - how keys reach a solver with no full keyboard.
- **K6. Key maps as the solver's own.** Key maps are workspace items (`keymap.edit`, `file.keymap.apply`, `file.keymap.check`, `file.keymap.export`), and themes never change keys (owner, 2 October, section 3). Open points:
  - one default map with the solver's edits, or one map per orbit as in 0.4;
  - the conflict check;
  - import and export;
  - whether 0.4 key maps migrate (F-MIGRATE).
- **K7. Pointer and keys together.** The H-05 pick targets are `object.inspect`, `grip.select`, `twist.apply`, `camera.rotate` and `camera.center` (command-table.md, section 4). Inspecting an object, assigning the target, assigning a grip and moving the camera are separate actions, and hovering only inspects (solving-workflow.md, section 4.3). The owner's 0.4 brief kept 3D and 4D drag, Ctrl gestures, multi-click twists and exact picking. Open points:
  - which of these stay;
  - the modifier conventions for pointer actions;
  - how a twist by pointer meets the twist keys.

## Owner input (binding)

On `main`:
- The [charter](../../charter-2.0-draft.md), signed on 3 October:
  - line 49, the product thesis: keyboard-alone efficiency, the Hyperspeedcube reference and the failure of 0.4;
  - line 83, S4: the commit takes one action;
  - line 92, E5: every action is reachable from the keyboard through the command table, including a command palette;
  - section 4 and [command-table.md](../../command-table.md) section 1: the six layers and their rules;
  - anti-goals 2 (screens full of text) and 7 (a game HUD).
- The owner's [design decisions](../../../../wiki/decisions/owner-decisions-2026-10-02-design.md) of 2 October:
  - section 3: commands and keys are fixed across theme families;
  - section 5: soft disclosure; the palette always offers every command; an expert can open all keys at once; key-disclosure levels and reward levels are different scales with distinct names;
  - section 6: two linked views with one context;
  - section 7: one workspace, with "show all keys" always available;
  - section 8: the role priority;
  - section 9: key hints are not free text.
- The human-solve boundary ([owner-decisions-2026-10-02](../../../../wiki/decisions/owner-decisions-2026-10-02.md), line 59), its one exception ([owner-decisions-2026-10-17-repair-pairs](../../../../wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md)) and the 3 October ruling on selected constructions ([owner-decisions-2026-10-03](../../../../wiki/decisions/owner-decisions-2026-10-03.md), lines 57 to 59).
- DT-J1's outcome and the owner's answers of 22 October ([DT-J1](jumble-solver-aids.md)): O9 (a), number keys copy a repair row into the draft; O10 (b), one commit of a whole repair after a preview the solver can step through.
- [requirements-from-screening.md](../../requirements-from-screening.md):
  - R-05: one permission path for every command that changes protection;
  - R-07: no tool commits outside the normal commit path;
  - R-14: no synchronous engine call on the UI thread;
  - R-16: atomic job admission;
  - R-18: key repeat never repeats a state-changing command, and key routing is defined in the keyboard model.
- [solving-workflow.md](../../solving-workflow.md):
  - section 4.1: draft and live input;
  - section 4.3: keyboard, filters and views;
  - section 6: keyboard first.

## Handoff and evidence

- [command-table.md](../../command-table.md), `command-table.json` and [build_command_table.py](../../build_command_table.py).
- `inventory/top-down.json`, `dispositions/units.json`, `dispositions/dispositions-s1.json` to `-s6.json` and [flows.json](../../dispositions/flows.json).
- [jumbling-handoff/design-table.md](../../jumbling-handoff/design-table.md), DH-06: the rows the views need. [jumbling-handoff/look-lab.md](../../jumbling-handoff/look-lab.md), LH-19: a Look Lab flow step must name a real command.
- The owner's 0.4 product brief, section 4, "Keybind convenience is the absolute core" (`docs/progress/0.4/snapshot/design/01_PRODUCT_BRIEF_ZH.md`, lines 50 to 65, in Chinese). The brief is historical and was superseded on 14 September 2026, but section 4 states the owner's own key requirements; solving-workflow.md section 4.3 carries them.
- `grips.py`: the 11 rotations of a cap and their seven axes.

## Out of scope

- The layout and look of panels, the palette and the key map (H-04; G1, G6), and motion (G5).
- The letters on the keys: G6, with the owner. This round settles the model, its rules and the options.
- The availability protocol between the command layer and the engine (command-table.md, section 5, point 5), which needs the Astra plan check of the architecture before 2.5.
- Writing the final table: EN4.
- The layout of Jumble's views and of the repair list (DT-J2, DT-J4).
- Performance claims. A cost is given as measured, with its evidence, or as unmeasured.

## Round 1

Run on 22 October 2026. The four roles answered independently on the record above, at most six cards each. Their cards are appended unchanged; only the layout differs, with one heading per card and one list item per field.

### Front-end designer

#### D1

- on: items K1, C1, C7, K5, K6
- kind: proposal
- claim: Owner, for item K1: (a) family 2, a grip-and-twist block bound by key position (labels follow the user's layout, and the clickable on-screen keyboard mirrors it), plus family 1 as one fixed map of chords for the loop's frequent commands, plus family 4 for the rest as prefix groups whose second keys appear as tips, all under the palette, which I recommend; (b) families 2 and 1 with the palette alone, so rare commands cost a search; (c) either of these with 0.4's switched sets (family 3). Under (a) or (b) a key calls one command whose object argument (a cap, piece, orbit, macro or phase) comes from the shared context, so one default map plus the solver's edits replaces 0.4's 175 orbit banks and its Functions set: `bank.select` gives way to the anchor of card D2, and `bank.functions.toggle` gets no row, which is item C1's rule applied by purpose rather than by 0.4 control.
- evidence: docs/progress/1.0/charter-2.0-draft.md:49; docs/progress/0.4/snapshot/design/03_ARCHITECTURE.md:35 (Functions set), :432 (175 bank records), :450 and :486-487 (picker opened with F2 and typed as "33-I", or Alt+1 to Alt+5); docs/progress/0.4/snapshot/design/01_PRODUCT_BRIEF_ZH.md:56-57 (in English: "the piece edit workbench has only two actions, Grip and Twist"; software-level commands stay reachable "through menus, the command index and the existing shortcut system" and are not put into the Grip/Twist editor), :63 (the on-screen keyboard "shows the real current mapping and pressed state and can be operated"); docs/progress/0.4/snapshot/design/06_REFERENCE_AUDIT.md:31 (physical and semantic keys); docs/wiki/decisions/owner-decisions-2026-10-02-design.md:50; docs/progress/1.0/dispositions/dispositions-s5.json:138; docs/progress/1.0/dispositions/dispositions-s6.json:158; docs/progress/1.0/command-table.json:1367, :1405. Sketch, lock a finished block and then set the next target: in 0.4 the solver first switched bank (F2, "33-I", Enter: six presses, or Alt+n if the solver recalls n) and then pressed the bank's key. Under (a) it is <prefix: blocks and locks> <lock block> (two presses), then <set target> on the inspected position (one press), with no active set to remember.
- needs_owner: yes

#### D2

- on: items K2, K4, K7, C7
- kind: proposal
- claim: Grip keys are one block of 20 slots holding the affecting caps of an anchor position, hosting cells first in a fixed model order. The slots never change by themselves: when the target changes, the new cap IDs show beside the old ones until the solver re-anchors with one key (on the target, buffer A, buffer B, the inspected position or the previous anchor), so `grip.capture` becomes this command and the orbit and task sets with previous-set recall that 2.2 kept survive as anchors. A slot whose cap moves A or B carries a status pattern, because a star's setup must avoid those caps, and any of the 600 caps can also be gripped by a pointer pick (H-05) or by its ID in the palette. Twists are seven axis keys (three half-turns and four third-turn axes) plus one inverse modifier, in the cap's fixed frame, shown with each slot's cap ID on the on-screen keyboard and on the gripped cap in both views; a grip holds while its key is down (latch as an option), focus loss releases it, and no grip or twist key repeats.
- evidence: Measurement on assets/model.npz (arrays mask_offsets, mask_values, face_offsets and face_values, read with the Python standard library; model files only, no session). Every moving position has 2 to 20 affecting caps, and the count is the same within each orbit. Only the 120 positions of O34 have 20; O33 has 19 and O32 has 18. At all 176,520 moving positions the hosting cells are among the affecting caps. The sampled orbits agree with docs/progress/0.4/snapshot/experiment/ID_AUDIT.md:58-61 and docs/progress/0.4/snapshot/design/03_ARCHITECTURE.md:683. So 20 slots hold every cap that moves any one position, with no paging. Owner's brief, docs/progress/0.4/snapshot/design/01_PRODUCT_BRIEF_ZH.md (in English): :59 "show the current piece's hosting cells and organise grips for these cells first, while telling apart the caps that really affect the piece"; :61 "the binding table shows which C ID, frame and twist a key now points to; after the screen turns, the filter changes or the piece moves, a key must not silently change meaning; when the mapping must be rebuilt from the current piece, show the change and let the user apply it explicitly"; :63 "a 20-cell piece must not force the user to hunt for keys through layers of pages". Further sources: docs/progress/1.0/solving-workflow.md:24, :96; core.py:152 (the buffer guard set); grips.py:22; docs/progress/1.0/dispositions/dispositions-s5.json:354; docs/progress/1.0/dispositions/dispositions-s6.json:158; docs/progress/1.0/command-table.json:1386; docs/progress/1.0/command-table.md:70. Sketch, a two-twist setup into Prepare with the anchor on the target: hold <slot 4> and tap <T2>; then hold <slot 11>, hold <inverse> and tap <T1>. That is five presses, and the keys are the same after any camera move.
- needs_owner: no

#### D3

- on: items C2, K4
- kind: proposal
- claim: Both, in one fixed order: a key-capture field first, then a text field, which takes every character key (so a letter typed in a macro never twists), then a focused list or view, which takes only navigation keys (arrows, Enter to inspect, Tab, and the row-pick number keys of DT-J1's O9), and then one global map for every other key, wherever the focus sits. The eight draft contexts thus become three routing classes (text, list or view, global), and the engine still decides availability per revision. The input destination is the only mode that changes what a key does (Jumble only adds keys, card D6), and it is always shown beside the grip state; the camera, a filter, hover and the focused view never change a key's command or argument, and only camera, list-movement and preview-scrubbing keys repeat.
- evidence: docs/progress/0.4/snapshot/design/01_PRODUCT_BRIEF_ZH.md:65 (in English: "typing a letter in a macro text box never twists, and pressing Enter in Global never commits the main preview"); docs/progress/1.0/command-table.md:23, :69; docs/progress/1.0/build_command_table.py:18; docs/progress/0.4/snapshot/design/03_ARCHITECTURE.md:479 (0.4's order also had an "active workbank" layer); docs/wiki/decisions/owner-decisions-2026-10-02-design.md:66-67, :72; docs/progress/1.0/solving-workflow.md:96, :98; docs/progress/1.0/requirements-from-screening.md:35; docs/progress/1.0/design-table/topics/jumble-solver-aids.md:1022. Sketch: with a fresh preview pending, the solver arrows through Global to a cell and presses Enter. The cell is inspected in both views and nothing commits. <set target> then assigns it, the preview goes stale, and <commit> is refused as stale until <preview> runs again.
- needs_owner: no

#### D4

- on: items C4, C5, C7
- kind: proposal
- claim: Twist keys type into one shown destination, either live or the selected draft phase, which changes only through the destination and phase keys (never through a commit, undo, focus or the camera). The commit is one non-repeating chord, not Enter, that works from any non-text focus while the preview is fresh, where 0.4 first needed focus on its execution controls. Escape closes only the innermost transient layer (palette, text edit, row focus, inspection) and never stops a job, cancels a preview or drops a draft; job stop and preview cancel keep their own keys. Owner, for undo: (a) one key for the solver's latest action of either class, with the key map naming what it will undo, which I recommend because the solver then holds nothing in mind; it needs a workspace-undo row, and `twist.apply` split from a draft-append row so that each row has one undo class (item C7); (b) two keys, one per class; (c) the class follows focus.
- evidence: docs/progress/1.0/charter-2.0-draft.md:83 (S4), :91 (E4); docs/progress/1.0/solving-workflow.md:63, :80; docs/progress/1.0/dispositions/dispositions-s5.json:273 (the destination is unambiguous before turn input and kept as work context); docs/progress/0.4/snapshot/design/03_ARCHITECTURE.md:479 (Escape), :501 (0.4 committed "only when its execution controls have focus"); docs/progress/1.0/command-table.md:44, :77; docs/progress/1.0/command-table.json:163 (the only undo row, journal class; the workspace class has no row), :553; docs/wiki/decisions/owner-decisions-2026-10-02-design.md:35. Sketch: <Prepare>, two grip-and-twist chords, <add macro> for the orbit's star, <Cleanup from Prepare>, <preview>, <commit>. A wrong twist is undone with <undo> under the hint "draft step 2"; a wrong commit is undone with <undo> under the hint "commit".
- needs_owner: yes

#### D5

- on: items C3, K3
- kind: proposal
- claim: The palette lists every command, showing an unavailable one dimmed beside the engine's reason rather than hiding it. It searches labels, glossary terms, command IDs and key names, shows each entry's key even before its level opens, takes an object argument that defaults to the context, and runs a commit through the same gates, so a commit without a fresh preview shows that reason and offers the preview instead of opening a second path. For item K3, L2 must also open the target, buffer roles, draft destination, preview and commit, because its goal is the first reward occasion (a checked insertion with the solver's own macro). Under card D1 (a) each level opens one prefix group, "show all keys" only shows the whole map, and "open all" is a saved choice the solver can undo.
- evidence: docs/progress/1.0/charter-2.0-draft.md:49, :52, :92, :111; docs/wiki/decisions/owner-decisions-2026-10-02-design.md:62, :73; docs/progress/1.0/command-table.md:23; docs/progress/1.0/requirements-from-screening.md:15, :17; docs/progress/1.0/design-options-2026-10.md:210; docs/progress/1.0/solving-workflow.md:63, :112. Sketch: <palette>, then type "repair". The entry reads "Repair list · cap C150 (named) · <its key>", and Enter runs it for the named cap. Typing a key's name lists what that key does here.
- needs_owner: no

#### D6

- on: item C6
- kind: proposal
- claim: Jumble needs no key mode. Each cap's I_a menu is a group of 60 rotations that contains the cap's 12 retained rotations (identity included), so the retained twist keys keep their meaning with Jumble on, and the other 48 menu twists come from four seat keys (a working name), one per remaining coset of the retained group, each pressed alone or held with a twist key. The manual's at most 12 face-match candidates are then one seat (or no seat) held with the twist keys, once the mathematician pins each seat's base twist and the coset side together with the I_a order. Owner: (a) seat keys beside ENG-0J's pointer drag, which I recommend; (b) a menu index typed in digits; (c) the pointer drag and the palette only. In each option, Jumble on and Jumble off are plain commands whose refusals the engine states, DT-J1's query rows take the named cap from the context, and no key that previews a menu twist repeats, so every check stays a twist the solver released (DT-J1 fix-first 6).
- evidence: research/jumbling/theory/groups.py:57-72 (I_a is built from A4_0 and a 72-degree rotation; 60 elements; closure and containment of A4_0 are checked exactly); research/jumbling/sim/twists.py:433-439; research/jumbling/manual/02-descent.md:44 ("at most 12 twists of the menu do that, and they differ by the 12 retained twists of the cap"); docs/progress/1.0/design-table/topics/command-keyboard-model.md:38; docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:130, :145; docs/progress/1.0/design-table/topics/jumble-solver-aids.md:163 (a median of 71 checks per face match), :959, :974; docs/wiki/decisions/owner-decisions-2026-10-22.md:43; research/jumbling/state-contract.md:183-184. Sketch, a face match at the named cap: <grip named cap> latches the grip; hold <seat k> and tap twist keys, one check per release, at most 12 presses per seat; then <commit>. Option (b) costs up to three presses per twist (two digits and a confirm), and the index has no spatial meaning.
- needs_owner: yes

### Interaction artist

#### A1

- on: item K2
- kind: proposal
- claim: A cap has 11 distinct twists, not 14. Each of its three half-turns is its own inverse, so 0.4's inverse control on those axes repeated the same rotation; whatever letters G6 picks, a half-turn gets one key and an undirected glyph, and each of the four third-turn axes gets a mirrored pair of directed glyphs. Each twist hint sits where its rotation acts, on the gripped cell's own tetrahedron (a third-turn pair at the vertex its axis passes through, a half-turn across the two opposite edges its axis joins), comes from the same resolver as the physical key and in draft input also carries the target phase's glyph, so its on-screen sense follows the camera exactly as the turn will while the key's meaning never changes.
- evidence: grips.py:14, :22, :27-28 (an order-2 rotation is paired with itself as its inverse); tests/test_v02.py:32 (seven axes, 11 rotations); native/NativeHost.cs:464-465 (0.4: left click forward, right click inverse, on all seven axis buttons); tests/test_turn_probe.py:490-492 (an order-two item's inverse key is itself); research/jumbling/fixtures/README.md:36 ("half-turns have no swept direction"); docs/progress/1.0/design-options-2026-10-round2.md:50 (glyph and orientation-glyph channels); docs/progress/1.0/dispositions/dispositions.md:262 (U3-011: meanings from the same binding resolver as physical input), :441 (U4-030: no silent remap on camera, filter or piece moves); docs/progress/0.4/snapshot/design/01_PRODUCT_BRIEF_ZH.md:61
- needs_owner: no

#### A2

- on: items K2, K4, K1
- kind: proposal
- claim: No piece has more than 20 affecting caps (all 177,120 positions: one-cell pieces have 1 to 19, two-cell 2 to 18, five-cell 5, 10 or 15, and the 120 twenty-cell pieces exactly 20). So at most 20 grip slots relative to the current piece, hosting cells first, reach every cap that acts on it with no paging and no switched key set, each hinted by a small key-cap glyph in the user's own layout at its cap's pole inside the focus set, never by a hotbar, a game-style button prompt or a text legend (anti-goals 2 and 7). When the piece changes, each slot whose cap would change keeps its old meaning under a "remap pending" status pattern until the solver applies the new map with one key, and the gripped cap always carries a grip outline that drops in the same frame a held grip is released or focus is lost, so no twist lands on a cap the screen does not show as gripped.
- evidence: assets/model.npz `face_offsets`, `face_values`, `mask_offsets`, `mask_values`, read as core.py:98-99 defines hosting cells and caps. I counted all 177,120 positions read-only with the standard library: the maximum is 20, and the hosting cells are among the affecting caps at every position. NumPy is not installed in this session, so core.py itself was not run. Further sources: docs/progress/1.0/solving-workflow.md:96, :107; docs/progress/0.4/snapshot/design/01_PRODUCT_BRIEF_ZH.md:59, :61, :63 (no hunting for keys through pages for a 20-cell piece), :65; docs/progress/1.0/charter-2.0-draft.md:49, :64, :69; docs/wiki/decisions/owner-decisions-2026-10-02-design.md:42, :85; docs/progress/1.0/dispositions/dispositions.md:441
- needs_owner: no

#### A3

- on: items C4, C5, K2
- kind: proposal
- claim: Frequent commands answer by channel, never by colour (no colour is assigned here):
  - `grip.select`: a grip outline and no puzzle motion;
  - a live twist or redo: the H-03 turn on the committed puzzle;
  - a draft twist: a step glyph in its phase, a forecast status pattern on any view of the predicted state, and a motion class of its own, so it never reads as a commit;
  - undo: the H-03 undo motion, with its hint showing beforehand the glyph of what it would take back (a twist, a draft step, a lock), under any undo model of item C5;
  - a preview: effect glyphs under a "current" pattern that turns "stale" on any edit, a pattern distinct from "applied, display out of date";
  - `operation.execute`: a net-effect motion, since a macro's net effect is a permutation, not one rotation;
  - hover: an inspection outline only.

  H-03 lists only turn, undo, invalid action and solved, so under charter bar A1 it needs rows for the draft step, the preview scrub, the operation commit and a key remap or opening (meanings settled here, timing in G5); otherwise those changes must not move.
- evidence: docs/wiki/decisions/owner-decisions-2026-10-02-design.md:32, :35, :41, :68; docs/progress/1.0/charter-2.0-draft.md:81 (S2), :83 (S4), :115 (bar A1); docs/progress/1.0/stage-2-experiment-protocol.md:152 (H-03 rows); docs/progress/1.0/solving-workflow.md:80, :98; docs/progress/1.0/command-table.md:26, :29; docs/progress/1.0/glossary/glossary.md:50 (a grip changes nothing in the state)
- needs_owner: no

#### A4

- on: items C3, C4, C5
- kind: proposal
- claim: The palette lists every command with its key as a key-cap glyph, so that it teaches the keys. An unavailable command stays listed under an "unavailable" status pattern, with its typed reason as a category glyph (busy, stale, protection, input and the rest), because hiding it breaks "always offers every command" and dimming alone leaves status to a tone. A refused key press answers in place, never modally: what refuses it breaks through with the H-03 invalid-action motion (the protected pieces a twist would move, the running job with its stop key, a commit hint marked stale), and a stopped or cut-off job keeps its own pattern. The owner picks whether the error message that charter bar E6 requires shows (a) only on request, or (b) at once for the first refusals of each category, then on request (preferred: learners get the reason while it is new, experts get the glyph only).
- evidence: docs/wiki/decisions/owner-decisions-2026-10-02-design.md:41, :42, :62, :85; docs/progress/1.0/command-table.md:23, :25, :31 (ten typed error categories); docs/progress/1.0/charter-2.0-draft.md:64, :93 (bar E6), :120 (bar A6); docs/progress/1.0/dispositions/dispositions.md:252 (U3-001); docs/progress/1.0/design-table/topics/jumble-solver-aids.md:955 (typed job results)
- needs_owner: yes

#### A5

- on: item K3
- kind: proposal
- claim: "L2" already means three things: "orbits and protection" in the glossary, "target, piece, buffers" on the first reward ladder, and "macro entry and star" on the draft key scale; "L" also reads as Local. So the owner picks a name for the key scale without L numbers:
  - (a) steps named by the workflow step they open, each with its concept glyph and in the glossary's teaching order, so a key opens together with its object's term card, while rewards keep their occasion names (preferred);
  - (b) a numbered scale under a word no other scale uses, such as "key tier 0 to 6";
  - (c) keys keep L0 to L6, and the other two scales are renamed.

  Whatever the name, a step opens in place, never modally or during input. Its key hints carry a "new" status pattern until first used, with a quiet motion unlike any reward motion, because opening keys is learning, not progress. "Show all keys" changes no disclosure state, and "open all" is a saved choice that can be reversed.
- evidence: docs/progress/1.0/glossary/glossary.md:7-14, :130, :202 (star is taught at glossary L3 but its keys open at key L2; protection is taught at L2 but locks open at key L4); docs/progress/1.0/design-options-2026-10.md:85, :210; docs/progress/1.0/design-options-2026-10-round2.md:116; docs/wiki/decisions/owner-decisions-2026-10-02-design.md:33, :36, :62, :73; docs/progress/1.0/dispositions/dispositions.md:266 (U3-015); docs/progress/1.0/charter-2.0-draft.md:102 (bar M6), :120 (bar A6)
- needs_owner: yes

#### A6

- on: items C6, K3
- kind: proposal
- claim: Under I_a the twist keys need not change. The pinned menu's items 1 to 11 are exactly a cap's 11 rotations (items 0 to 11 are the cell's tetrahedral stabiliser), so Jumble on extends the twist block rather than re-mapping it, and only 48 items per grip need the axis-and-angle or menu input: 12 half-turns, 12 third-turns and 24 fifth-turns of one or two steps (72° or 144°), on 24 further axes. These take the same glyph grammar by order. The drag shows its realised element as that glyph, with no snap or detent that favours alignment. A blocked grip answers a twist key with the cap map's blocked pattern, never with a new meaning. The jumble-only input opens as its own key step at the first Jumble on, outside the orbit-first steps.
- evidence: research/jumbling/theory/walk.py:21 (the walk draws only items outside the cell's stabiliser). research/jumbling/fixtures/wj-I_a.json (one line), measured read-only:
  - `menu.contains_a4` is true, and the base lists I_a[0] to I_a[59] in order;
  - the 841 journal records use exactly items 12 to 59, twelve each at 72°, 120°, 144° and 180°;
  - the traces of items 1 to 11 are 0 at items 1, 10 and 11 and 1 at items 2 to 9, that is, three half-turns and eight third-turns;
  - the icosahedral rotation group has 6 five-fold, 10 three-fold and 15 two-fold axes, of which the tetrahedral subgroup uses 4 three-fold and 3 two-fold.

  Further sources: docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:127 (pinned order, index 0 is the identity), :130 (the drag maps to the nearest menu element and shows the realised element), :145 (rows); docs/progress/1.0/design-table/topics/jumble-solver-aids.md:948, :970, :974 (one key set, E1); docs/progress/1.0/solving-workflow.md:96
- needs_owner: no

### Mathematician

#### M1

- on: K2, K6 (record lines 36 and 92; grips.py:22)
- kind: objection
- claim: "Seven axes, each forward and inverse" gives 14 inputs for only 11 twists. The 3 half-turns are their own inverses (−(2c+1) and 2c+1 are the same turn), and only the 4 vertex axes have two senses each, where a sense is one of A4_c's two conjugacy classes of third-turns and every symmetry in K⁺ preserves it; so the least input is one chord per twist, for example 7 axis keys plus an inverse modifier that changes only the 4 vertex-axis twists and is shown as having no effect on a half-turn. Keys must name a sense by its class, never by a screen sense such as "clockwise", because one camera move reverses the drawn sense of the same rotation; 0.4's labels break the class (T1, T2 and T4 are in T1's class, but T3 = [1,−2] is in T1⁻¹'s), so 1.0's names must follow the class, and a 0.4 key map migrates by the word each key typed, never by its label.
- evidence:
  - grips.py:14 (order 2 or 3); :27-28 (a half-turn's inverse is itself, j = i).
  - research/Full_600cell_Puzzle_Theory.tex, hypothesis F (lines 839-842) and the proof of Lemma 6 (lines 903-907: the three half-turns form the Klein four group, and T_c cycles them).
  - Exact read-only check, standard library only, on assets/model.npz (rotperms, frameperms):
    - grips.py's breadth-first labels at cap 0 are H1=[1], T1=[2], T1'=[−2], T2=[1,2], T3=[1,−2], T3'=[2,1], T2'=[−2,1], T4=[1,2,1], T4'=[1,−2,1], H2=[2,1,−2], H3=[−2,1,2].
    - H_c = F_c·H_0·F_c⁻¹ and T_c = F_c·T_0·F_c⁻¹ at all 600 caps, so the same pattern holds at every cap.
    - Conjugation inside A4_c at caps 0, 1, 137 and 599 puts T1, T2, T4 and T3' in one class and T1', T2', T4' and T3 in the other.
  - Camera: research/Full_600cell_Puzzle_Theory.tex:1840-1848 (camera Q in SO(4); perspective Π₄(x,y,z,w) = d(x,y,z)/(d−w)). Sketch of the argument:
    - a vertex axis seen from its vertex and seen from the opposite face shows opposite senses;
    - Π₄ mirrors the image of every cell that faces away from the eye (0,0,0,d), so turning cell c away flips the drawn sense of all its twists.
  - 0.4 twist keys typed exact words: docs/progress/0.4/snapshot/experiment/NATIVE_REVIEW_GUIDE.md:75.
- needs_owner: no

#### M2

- on: K2, K7 (record lines 37 and 91)
- kind: proposal
- claim: A grip key is well defined only as a stored map from keys to cap IDs that no camera, filter or piece motion changes. "Relative to the current piece" is then an explicit rebind command: it proposes a new map from the piece's affecting caps M(A), shows the difference and waits for the solver. One page of 20 grip slots is exactly enough for every piece's affecting caps: O34 has 20, O33 has 19 with one host cell, and O32 has 18 with two. The rebind rule must state its own order, because the piece does not fix one: the stabiliser of the position in K⁺ permutes M(A) (O34: order 60, one orbit of 20 caps; O33: order 3, the host cap plus six triples that its single sticker cannot tell apart). So the tie-break is cell IDs or a reference the solver set, never the screen.
- evidence:
  - research/Full_600cell_Puzzle_Theory.tex, Lemma 6:
    - lines 888-897: r(A) = |M(A)| − 1, and complete signatures identify positions;
    - lines 929-933: M(kA) = kM(A) for k in K, so the stabiliser of the position equals the set stabiliser of M(A).
  - assets/census.json:1240, :1293, :1331 (orbits 32 to 34: rank, mask, faces). Over all 35 orbits, the largest mask has 20 caps, and every faces set lies inside its mask.
  - Standard library on assets/model.npz:
    - K⁺ = {F_c·s : s in A4_0} gives 7,200 pole permutations;
    - the set stabiliser of O34's mask has order 60 and one orbit of 20 caps;
    - the set stabiliser of O33's mask has order 3, with orbit sizes 1 + 6 × 3.
  - Owner rule, docs/progress/0.4/snapshot/design/01_PRODUCT_BRIEF_ZH.md:
    - :59: hosting cells come first, but affecting caps are kept apart from them;
    - :61: each key shows its C ID, frame and twist, and a re-map to the current piece is shown and applied by the solver;
    - :63: a 20-cell piece must not force paging.
  - docs/progress/1.0/solving-workflow.md:96.
- needs_owner: no

#### M3

- on: C1, C3, C6
- kind: proposal
- claim: Every command argument names an absolute object: a cap (which shares its cell's index), a position or slot, a piece identity (never inferred from a position or the other way round), an orbit, a rotation by its exact matrix or K⁺ index, or a generator word. The journal, macros, drafts, receipts and palette store that resolved value, because relative names (the current grip, a key, a twist label, a menu index, a list row) depend on state that moves. A relative name kept anywhere must cite what fixes it, and is refused once that has changed:
  - a menu index means F_c·Λ₀[i]·F_c⁻¹ under the order digest 5a0429d2…;
  - a repair-row number key holds only at its list's revision and cap;
  - I_a numbers the 11 retained twists in a different order from 0.4 (I_a[4] = T3', I_a[10] = H3, I_a[11] = H2), so one key set for both puzzles binds keys to rotations, never to menu indices.
- evidence:
  - research/jumbling/sim/twists.py:482-483 (the menu identity is computed over sorted matrices); :491-505 (a cap's items are the base items transported by F_c, in record order).
  - docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:127 (the order digest); :141 (stored (grip, index) letters cite it).
  - research/jumbling/fixtures/wj-I_a.json, journal records: the exact matrix sits beside the label (for example grip 57 with I_a[17]).
  - docs/progress/1.0/design-table/topics/jumble-solver-aids.md:1006 (O9 (a): a number key copies a row into the draft); :187 (the list is discarded at the next commit).
  - Exact standard-library comparison of the menu record with grips.py's labels at cap 0: I_a[1] to I_a[11] are H1, T1, T2, T3', T1', T4', T4, T3, T2', H3, H2.
  - docs/progress/0.4/snapshot/experiment/NATIVE_REVIEW_GUIDE.md:63 (piece(n) follows the identity; position(n) stays where it is).
  - research/Full_600cell_Puzzle_Theory.tex, Lemma 7 (lines 997-998: another frame choice conjugates the coordinates).
  - A twist's word is canonical only by rule: [2,1,2] = [1,−2,1] at caps 0, 1, 137 and 599. grips.py:18-20 and research/jumbling/sim/kplus.py:175-193 apply the same breadth-first rule.
- needs_owner: no

#### M4

- on: C6 (ENG-0J line 145, "jumble twist with axis and angle input"; record line 75)
- kind: objection
- claim: The A1 axis-and-angle input cannot type 48 of a grip's 59 I_a twists. A1 searches only rational Cayley parameters and rational half-turn axes, and menu mode never approximates; an exact check of the wj-I_a menu finds rational parameters only for the 11 A4 items, while the other 12 half-turns, 12 third-turns and 24 fifth-turns have parameters in Q(√5) outside Q. Proposal: a menu-mode input that names an axis and then an angle over I_a's 31 axes (15 two-fold axes with one angle, 10 three-fold with two, 6 five-fold with four: 15 + 20 + 24 = 59). Its 7 retained axes carry exactly the 11 retained twists, so the retained twist keys are the Jumble-off part of one key set, and a non-retained axis is refused with a typed error while Jumble is off.
- evidence:
  - research/jumbling/state-contract.md:126-128 (A1 searches ω in Q³, and half-turn axes with the same bounds); :131 (menu elements are never approximated; other requests are refused in menu mode).
  - research/jumbling/sim/twists.py:421 (parameters are built as Q5(x, 0, q), so they are rational); :424-425 (the menu refusal).
  - research/jumbling/sim/README.md:60.
  - Exact standard-library computation on the `menu` record of research/jumbling/fixtures/wj-I_a.json, in the cap frame of twists.py:39-41:
    - ω = axial(R − Rᵀ)/(1 + tr R), or the half-turn axis, is rational exactly for I_a[1] to I_a[11];
    - grouping the 59 items by axis gives 31 axes holding 1, 2 or 4 items;
    - the 7 axes of I_a[1] to I_a[11] hold no other item.
  - research/jumbling/theory/groups.py:59 (I_a is A4₀ plus a 72° turn about (0, 1, φ)).
  - docs/progress/1.0/design-table/topics/jumble-solver-aids.md:783-790 (E14: J1 has no map from a drag to a menu item); :974 (E1: a jumble letter is refused while Jumble is off).
- needs_owner: no

#### M5

- on: C6, K2, K3 (glossary terms)
- kind: proposal
- claim: One term per concept fails exactly where this topic labels keys:
  - the glossary's Twist ("one of the 1,200 generators or its inverse") is false for 8 of a cap's 11 twists; for example H2 = [2c+2, 2c+1, −(2c+2)] is not a generator;
  - "jumble twist" means the 59 twists of a grip in this record, the 48 outside A4 in DT-J1 M12, and an A1 rotation in ENG-0J line 145;
  - Grip is a cap in the glossary but "cap and frame" in the 0.4 brief and in the command table.

  Proposal (the names are preference):
  - Twist = a turn of one grip by a non-identity element of the active menu, with two species: retained twist (11) and jumble twist (48);
  - Generator stays the 1,200, and every count names its alphabet (keys, twists or generators);
  - Grip stays the cap, and the frame moves into M2's rebind rule;
  - Key map = a stored map from (key, context) to a command ID with absolute arguments.
- evidence:
  - docs/progress/1.0/glossary/glossary.md:40, :50, :90 (sources: docs/progress/1.0/glossary/terms.json:80, :88).
  - docs/progress/1.0/design-table/topics/command-keyboard-model.md:36, :38.
  - docs/progress/1.0/design-table/topics/jumble-solver-aids.md:387-393 (M12).
  - docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:145 ("jumble twist with axis and angle input" beside "menu twist").
  - docs/progress/1.0/command-table.json:1286 ("Grip (cap and frame)").
  - docs/progress/0.4/snapshot/design/01_PRODUCT_BRIEF_ZH.md:56.
  - docs/progress/0.4/snapshot/experiment/NATIVE_REVIEW_GUIDE.md:77 (cell-only captures use the retained cap basis).
  - research/jumbling/state-contract.md:156-161 (three alphabets).
  - Word lengths from M1's computation: 3 twists are one generator long, 4 are two long and 4 are three long.
- needs_owner: no

#### M6

- on: K3
- kind: question
- claim: The draft key levels L0 to L6 use the same letter as the glossary's learning levels L1 to L6, but with different contents. L1 is "inspect and isolate" against "turning the puzzle", and L2 is "macro entry and star" against "orbits and protection"; stars open at L2 but are taught at L3, and work sheets open at L3 but are taught at L4. So one symbol names two scales, and the key scale places neither target and buffer keys nor Jumble's rows. Question for the owner:
  - (a) one scale: each key opens at the learning level of its concept (stars with target and buffers at L3), and Jumble's keys open when Jumble is first turned on;
  - (b) a separate key scale with its own letter, for example K0 to K6 (preference), with a stated map to the learning levels;
  - (c) keep L for keys and rename the learning levels.
- evidence:
  - docs/progress/1.0/design-options-2026-10.md:210 (the key levels).
  - docs/progress/1.0/glossary/glossary.md:7-14 (learning levels); :162, :192, :202, :212 (Buffer, Setup, Star and Target at L3); :264 (Work sheet at L4); :3 (the levels are provisional until the owner decides the learning path).
  - docs/wiki/decisions/owner-decisions-2026-10-02-design.md:40 (one concept table gives the level where each concept is first taught); :62 (key-disclosure and reward scales have distinct names).
- needs_owner: yes

### Engineer's advisor

#### E1

- on: items C1, C7, K6
- kind: proposal
- claim: Only the 0.4 command IDs can join units to commands mechanically. 268 of the 377 keep or redesign decisions with a user entry point carry one, so about 205 reviewed alias rows (270 IDs under nine argument patterns; for example `grip:0` to `grip:6` become one row with a cap argument) join them, while the other 109 need a mapping each (17 of them have a candidate from a linked unit). EN4 should join per decision (a unit or a split part) after the disposition filter, and `build_command_table.py` should check coverage over these 377 decisions instead of the 72 T-xx rows; under that rule the structure commands of item C7 stay, because their 0.4 source units are redesign decisions, and the same alias table translates the command bindings in 0.4 key-map files (item K6).
- evidence: Counted read-only (python -I, script kept outside the repository) over docs/progress/1.0/dispositions/units.json, dispositions-s1.json to -s6.json and the twelve sealed inventory files. A split part takes its own rows' entry points. User entry kinds are key, button, menu, command, mouse and control. This reproduces the record's 377 decisions in 331 units. 110 of the 268 carry several IDs, and 94 still span several families after the patterns, so the join is many-to-many. The nine patterns: drop `NativeWorkbench.KeyAction:`; `:N`; `bank-<set>`; `global|local-<direction>`; `goal-*`; `edit|inspect|insert|phase-<phase>`; `endgame-q|r|x|y`; `review-reason-N`; `solve-locate-a|b|target`. No other join key exists. No T-xx string occurs in units.json, dispositions-s*.json, inventory/merged.json or inventory/correspondence.json (grep count 0). The top-down evidence (15 files) shares no file with the inventories' evidence (116 files). 173 of the 281 distinct key strings occur in more than one context, and top-down's "Ctrl+Enter" (docs/progress/1.0/inventory/top-down.json:534) appears as "Control+Enter" in the units. The disposition filter is needed because 9 IDs (eight `cycles-*` and `macro-check-library`) occur in both an automate and a redesign decision (U6-037's split; U3-085 against U6-031). Structure: U4-065, U4-066, U4-069/1 to /3, U4-070, U4-072 and U4-074 (cell views, centre, find and compare colour, layers) and U4-037 and U5-051 (saved sets) are all redesign, which contradicts docs/progress/1.0/command-table.md:76 ("0.3 documents only"). Key maps: the 0.4 schema names 0.4 command and bank IDs (docs/progress/0.4/snapshot/experiment/evidence/keymap-files-20260917/README.md:5); bank-scoped bindings have no target if the banks go.
- needs_owner: no

#### E2

- on: item K2; item C7 (`grip.capture`)
- kind: support
- claim: Relative grip keys need no engine call. A position's affecting caps are static model data (2 to 20 per moving piece, more than 10 for 50.5% of the 176,520 moving pieces), so a re-map is a lookup in `mask_offsets` and `mask_values` plus cap rotations derived once from the model, never 0.4's per-request `grips()` closure, which E-02 timed at 64.8 s behind a preview. I support item K2's relative keys on four conditions: the block holds 20 grip slots on one page beside the 7 axis keys (27 of the 47 character keys of a US ANSI main block); the binding changes only when the solver applies a shown re-map (0.4's `grip.capture` becomes that action); caps are ordered by a model rule, never by the camera; and a headless test shows that camera, filter and twist events leave the binding digest unchanged.
- evidence: assets/census.json, orbits: affecting caps = rank + 1 (docs/progress/0.4/snapshot/experiment/ID_AUDIT.md:52). This reproduces O32's 18 caps at docs/progress/0.4/snapshot/design/03_ARCHITECTURE.md:683. Counted read-only with python -I: from 2 caps (O00) to 20 (O34, 120 pieces); 18 to 20 caps for 6,120 pieces (O32 to O34); more than 10 for 89,160 pieces. core.py:45-47: mask offsets and values; rank = caps − 1; 1,857,600 position-cap pairs in all. grips.py:9-21: a closure with a least-squares fit and an SVD per call, served per request at server.py:199 and work/experiments/magic600-04/adapter.py:461. docs/progress/0.4.1/screening-findings.md:65: E-02, 6 ms idle and 64.8 s behind a 2.9M-primitive preview; this is a 0.4 headless timing, not a 1.0 measurement. docs/progress/0.4/snapshot/design/01_PRODUCT_BRIEF_ZH.md:61 (show the change, the solver applies it) and :63 (a 20-cell piece must not force paging); docs/progress/1.0/solving-workflow.md:96 and :107. Experiment: the key-hint overlay that marks what each grip key turns (up to 20 cap regions) enters the H-06 cost table and the H-09 overlay limits (docs/progress/1.0/renderer-experiment-plan.md:159, :162). No cost is claimed before then.
- needs_owner: no

#### E3

- on: items K4, C2, C7
- kind: objection
- claim: Fix first: the builder enforces R-18 only on `commit` rows. A copy in which `macro.add` (F-02's own failure: a held Enter appends a macro) and the `operation.preview` job may repeat still exits 0, so the builder should allow repeat only on `view` rows and should check a binding table in which no `editor`-context key resolves to a puzzle command. For item C7, split `twist.apply` so that the draft append drops the live twist's protection gate and journal undo (a draft may move protected pieces temporarily), and make the timer rows non-journal rows with no undo, as 0.4 kept one total in its `meta` table; this also removes the builder's timer exception.
- evidence: Counterexample: a copy of docs/progress/1.0/build_command_table.py and inventory/top-down.json, run outside the repository with `repeat=True` added to the `macro.add` and `operation.preview` rows, prints "86 Counter(...)" and exits 0. docs/progress/1.0/build_command_table.py:154 (repeat is checked for `commit` only), :155 (timer exception), :108 (`twist.apply`: gates protection and not_busy, undo journal, "draft destination edits the phase"). docs/progress/1.0/command-table.md:46 (R-18 narrowed to commit), :68 (the editor rule, which nothing checks, since the builder holds no bindings), :75. docs/progress/1.0/requirements-from-screening.md:35 (R-18 comes from F-02 and F-03); docs/progress/0.4.1/screening-findings.md:69-70; docs/progress/1.0/solving-workflow.md:80. session.py:16, :36 and enhanced.py:9, :17: the timer total lives in `meta`, never in `events`.
- needs_owner: no

#### E4

- on: items C4, C5, C3
- kind: question
- claim: Every commit row must carry `not_busy`, `twist.apply` and `session.undo` included, and 0.4 refused every submitted operation while any job ran. Unless 1.0 redefines busy, a live twist or an undo is therefore refused for as long as a check runs: 64.8 s behind one 0.4 preview, and the research repair list took 3 to 5 minutes per cap. Owner, what does a commit do to a running job: (a) the commit proceeds at once and cancels the job, whose old-revision result could never pass `fresh_preview` (the engineer's preference); (b) the commit proceeds and the job finishes, with its result shown as stale; (c) the commit is refused as busy until the job ends or is stopped, as in 0.4.
- evidence: docs/progress/1.0/build_command_table.py:155; work/experiments/magic600-04/adapter.py:2164-2166 (409, "An operation is busy; input was not queued"); server.py:136 (one worker), :343; docs/architecture/1.0/10_V1_ARCHITECTURE.md:513 (busy means wait or cancel); docs/progress/0.4.1/screening-findings.md:65 (0.4 headless timing); docs/progress/1.0/jumbling-observation.md:176 (research timing). Sketch that holds for every option (R-14, R-16):
  - A key resolves against the shell's binding table and one availability table per revision with a typed reason per command. The palette can then list an unavailable command with its reason (item C3) without an engine call, and a palette entry runs the same row and gates as its key (R-05, R-07).
  - `invoke` returns a receipt at once.
  - Admission is one compare-and-set per lane, keyed by the durable command ID. Release checks get a latest-wins lane (docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:133), so they never queue behind a repair-list job (:147).
  - Escape never reaches `job.stop`, which keeps its own key (0.4: Shift+Escape, build_command_table.py:63).

  Experiments: B-004's instrumented admission fixture (screening-findings.md:67) with concurrent invocations and a held key, run headless; commit latency during a long preview under (a) and (b), measured with the candidate engine on the owner's machine.
- needs_owner: yes

#### E5

- on: item K5; item K4 (IME, held keys)
- kind: proposal
- claim: Bind the grip-and-twist block and O9's number keys by physical key code, as 0.4 did for 1,109 of its 1,689 key entry points, and draw each label from the active layout. Binding by character breaks both: on French AZERTY the unshifted top row types & é " ' ( - è _ ç à, so the character 1 needs Shift, and A and Q, Z and W swap places. Owner, for the other commands: (a) all by position; (b) mnemonic modifier shortcuts by character, which makes the conflict check depend on the layout, so it must run against every layout the solver uses; (c) all by character, which I advise against.
- evidence: Counted read-only over the key entry points of docs/progress/1.0/dispositions/units.json, by notation: 1,109 physical codes (KeyQ, Digit7, Backquote), 426 named keys (F1, Enter, arrows), 35 characters (Ctrl+Z, B, 1) and 114 other. docs/progress/0.4/snapshot/design/06_REFERENCE_AUDIT.md:31 (physical bindings with clear display); docs/progress/0.4/snapshot/design/01_PRODUCT_BRIEF_ZH.md:65 (text input, IME, repeat, a held grip, a lost key-up); docs/progress/1.0/build_command_table.py:140 (T-59 is a routing rule); docs/progress/1.0/design-table/topics/jumble-solver-aids.md:1006, :1022 (O9 (a)); docs/development-guide/AGENT_BRIEFING.md:28 (platform code behind narrow interfaces). The portable layers store codes only; the code-to-label lookup and IME handling sit in the platform layer. The router's half (an `editor`-context key never fires a binding; losing focus releases held grips) is a headless test. The IME half needs a fixture on the owner's Windows machine. Nothing about Windows is claimed here.
- needs_owner: yes

#### E6

- on: item C6
- kind: question
- claim: Under I_a, a grip's 59 non-identity items lie on 31 axes (15 half-turn, 10 third-turn, 6 fifth-turn) and include the 11 retained twists on the 7 retained axes. So the retained block keeps its exact rotations with Jumble on, only 48 jumble twists on 24 further axes (1, 2 or 4 angles each) need new input, and Jumble on and off need no confirming key, since every refusal leaves the state unchanged and Jumble off keeps the exit poses for resume. Owner, for the 48: (a) a Jumble-on layer of axis keys plus an angle key, independent of the menu's item order; (b) the menu index typed after the grip, which ties typed input to the pinned order digest; (c) pointer and drag only, with the palette taking (grip, index) as the keyboard route.
- evidence: Classified read-only with exact fractions over the 60 matrices of the `menu` record in research/jumbling/fixtures/wj-I_a.json, by trace (2 + 2cos θ for a rotation that fixes the pole): 1 identity, 15 of order 2, 20 of order 3 and 24 of order 5 (12 at 72°, 12 at 144°). The record has `contains_a4: true`, and DT-J1 counts 48 jumble twists (docs/progress/1.0/design-table/topics/jumble-solver-aids.md:391). Line 38 of this topic's record ("59 jumble twists") should therefore read "59 non-identity items, 48 of them jumble twists". grips.py:22 (seven retained axes). Jumble on and off: docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:51 (Jumble on gives a witnessed retained checkpoint), :56 (each refusal is a typed error with no state change), :53 and :55 (exit poses kept; resume). Order dependence: jumble-solver-aids.md:954 (fix-first 1). Query rows: covers and the repair list are jobs (ENG-0J:147) that run in the exclusive lane and stop with `job.stop`; the release check is latest-wins (:133); each row takes the cap the solver names (jumble-solver-aids.md:979).
- needs_owner: yes
