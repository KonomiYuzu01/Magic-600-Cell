# DT-M1: Command model and keyboard model

Status: **settled with the owner, for now.** On 22 October 2026 the owner chose every recommended option and kept the other options of each question on record as alternatives ([owner answers](#owner-answers-22-october-2026), at the end). The round table and the design line are on hold after this topic. Opened on 22 October 2026 by the main session under the [round protocol](../README.md), as the second design-table topic of the [re-plan](../../replan-2026-10-22.md) (section 4). The card limits apply from this topic on: at most six cards per role in round 1, and at most four in round 2, each answering other cards by ID (owner, 22 October 2026: [decision](../../../../wiki/decisions/owner-decisions-2026-10-22.md)).

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

## Round 2

Run on 22 October 2026. Each role read round 1 and answered other cards by ID, at most four cards each, marking new evidence. Their cards are appended unchanged; only the layout differs, with one heading per card and one list item per field.

### Front-end designer

#### D7

- on: D1, D2, M1, A1, M2, A2, E2, M3, M5, E1, E5
- kind: proposal
- claim: I change D2 to the merged rule of M1, A1, M2, A2, E2, M3 and M5. Twenty slots hold the anchor's affecting caps, hosting cells first and then by cell ID, and they change only through a rebind that is shown and that the solver applies (M2's rebind, A2's "remap pending", E2's digest test). Seven axis keys and one inverse modifier type a cap's 11 twists in the cap's fixed frame; the modifier leaves a half-turn unchanged, and "forward" names M1's conjugacy class rather than a screen direction, so 0.4's T3 and T3' swap names and, where 0.4 key maps migrate, a twist key maps by the word it typed (M1) and a command maps through E1's alias table. Owner, two questions, recommended option first:
  - K1, the model (D1 unchanged; no card objects to it): (a) the position-bound grip-and-twist block, one fixed map of chords for the loop's frequent commands, and prefix groups whose second keys show as tips, all under the palette; (b) the block and the chords, with the palette alone for everything else; (c) either of these with 0.4's switched key sets.
  - Binding (E5): (a) every key by position, labelled from the active layout, except undo and redo, which keep the characters they have in office software and get a conflict check whenever the layout changes (E5 (b), narrowed); (b) every key by position (E5 (a)); (c) every mnemonic chord and prefix key by character (E5 (b)). E5 (c), every key by character, drops out of the batch because it breaks the block, as E5 shows.
- evidence: new evidence: docs/progress/1.0/dispositions/units.json:11385-11388 (U4-027: 0.4's undo was on Ctrl+Z in the main window, the workbench and the Edit menu); docs/progress/1.0/dispositions/dispositions.md:479 (U4-064, signed: text undo and clipboard keys stay with the focused editor and never reach puzzle undo). Further: docs/progress/0.4/snapshot/design/01_PRODUCT_BRIEF_ZH.md:57 (software-level commands keep "the existing shortcut system", undo included), :61, :63; docs/progress/0.4/snapshot/experiment/NATIVE_REVIEW_GUIDE.md:75 (0.4's G1 sample already had 20 grip slots on explicit caps, H1 to H3 and T1 to T4, and hold or latch); grips.py:27-28; docs/progress/1.0/design-table/topics/command-keyboard-model.md:228, :236, :298, :316, :337-340, :373-382, :415, :423, :453. Why I changed D2: a half-turn is its own inverse (M1, A1); a camera move reverses the drawn direction of the same rotation (M1); the position's stabiliser leaves the slot order to a tie-break (M2). Sketch: an AZERTY solver presses Ctrl with the key labelled Z. Under binding (a) it undoes, as in any office application; under (b) it fires whatever Ctrl chord sits at QWERTY's W position.
- needs_owner: yes

#### D8

- on: D6, M4, E6, A6, M3
- kind: proposal
- claim: I keep D6's seat keys and pin down what it left open. Seat k (k = 1 to 4) is the right coset A4_c·g^k for one pinned fifth-turn g, and a seat chord types k fifths of a turn about g's axis followed by the retained twist. So each of a grip's 59 menu twists is exactly one chord, and a cut's closing set, which the research writes as one coset A4_c·h⁻¹, is exactly the twelve chords of one seat. Under axis-then-angle input (M4, E6 (a)) the same twelve twists lie on twelve different axes behind a Jumble-on layer of 24 more axis keys. A typed menu index (D6 (b), E6 (b)) drops out of the batch: it has no spatial meaning and ties typing to the order digest (M3). Owner, for the 48 jumble twists:
  - (a) four seat keys, held like a modifier with the retained twist keys, each tapped alone for its own twist (recommended);
  - (b) axis then angle over the 24 further axes, as keys on a Jumble-on layer (M4, E6 (a));
  - (c) no Jumble keys: the drag is the pointer route and the palette the keyboard route, naming a twist by its rotation (E6 (c) under M3).
  - In every option:
    - Jumble on and Jumble off are plain commands with typed refusals and no confirming key (E6).
    - The 11 retained keys keep their rotations, and a jumble twist typed while Jumble is off is refused (A6, M4).
    - Keys bind to rotations, never to menu indices (M3).
    - ENG-0J's drag stays, and its A1 axis-and-angle row gives way to the chosen input (M4).
    - Nothing marks a seat, axis or chord according to the state.
    - Jumble's keys open at the first Jumble on (A6).
- evidence: new evidence:
  - research/jumbling/theory/restoration.md:79 (caps 389 and 36 "close with all twelve twists of one coset A4_c h⁻¹"), :100, :102 (at most 12 closing twists, one coset of A4_c, differing by elements of A4_c).
  - Exact computation with the Python standard library on the 60 `menu` matrices of research/jumbling/fixtures/wj-I_a.json (Q(√5) held as pairs of fractions; script piped on stdin, nothing written). A twist multiplies poses on the left (research/jumbling/state-contract.md:179), and the record agrees: cap 380's scramble twists 20 then 34 compose to item 38, which is the face match's undo, while the other order gives 53 (research/jumbling/manual/scripts/examples.out:2). Results:
    - The right cosets of items 0 to 11 are {12, 16, 20, 24–32}, {13, 17, 19, 34–42}, {14, 21, 23, 43–51} and {15, 18, 22, 33, 52–59}. Each holds 3 half-turns, 3 third-turns and 6 fifth-turns on 12 distinct axes.
    - Each meets every non-trivial left coset s·A4 in exactly 3 items. So with the opposite chord order, a closing set would split 3+3+3+3 over the four seats.
    - The powers of the fifth-turn g = I_a[12] are items 12, 33, 39 and 45, one in each seat. So the cosets A4·g^k (k = 0 to 4) are all five, and every item is exactly one r·g^k.
  - Key count: E2's 27 keys (docs/progress/1.0/design-table/topics/command-keyboard-model.md:423) plus four seat keys use 31 of the 47 main-block keys. Twenty slots, 31 axis keys and an angle key would need 52.
  - Boundary: docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:145, :149; docs/progress/1.0/design-table/topics/jumble-solver-aids.md:948, :970. Seats are fixed by the menu, as the retained keys are.
  - Scope and experiment: one seat holds the closing set when the outside of the cut is on the lattice, as at the isolated caps. When only the inside is on the lattice, the set is a left coset and splits 3+3+3+3. The experiment is to count both cases over the census face-match steps.
  - Sketch: at a named cap, the solver drags the inside until the realised twist looks right (ENG-0J line 130 shows it; here it would also show its seat). The solver then holds that seat, taps the twelve chords with one check per release, and commits the best. Under (b), the other eleven candidates sit on eleven other axes, and the solver must work out each one.
- needs_owner: yes

#### D9

- on: E3, E4, D4, D3, A3, A4
- kind: proposal
- claim: I support E3's fix-first, its timer rows and its split of `twist.apply`, which D4 (a) needs. D3's routing classes give the binding check: a key in a text field reaches no command, and a key that a focused list or view captures reaches only navigation, inspection or O9's row pick. I answer E4 by narrowing "busy" to one other commit in flight. A live twist or an undo then goes ahead at once and cancels only the jobs that read the revision it replaces, each shown as cancelled (A4). Keyed commits typed during a commit wait in order behind it; at the first refusal the rest are dropped, and their count is shown. Owner, recommended option first:
  - Undo (D4 unchanged): (a) one key undoes the solver's latest action of either class, its hint showing what it would take back (A3), while a text field keeps its own undo; (b) one key per class; (c) the class follows focus.
  - A commit while a job runs (E4): (a) as narrowed here (E4 (a)); (b) the commit goes ahead and those jobs finish, shown as stale, which needs ENG-0J line 150 and observation rule 3 changed (E4 (b)); (c) the commit is refused as busy until the job ends or is stopped, as in 0.4 (E4 (c)).
- evidence: new evidence: docs/progress/1.0/dispositions/dispositions.md:438 (U4-027, signed on 3 October: one command ID with the same busy and context gates for every key, pointer and menu invocation; editor undo stays separate from puzzle undo; a held committing key never repeats); :479 (U4-064: text edits, clipboard keys and text undo go to the focused editor, never to puzzle undo, redo or execution). E3's check and every D4 option keep both. docs/progress/1.0/jumbling-observation.md:35 (rule 3: after a commit a result is recomputed or discarded, never shown against a newer state). Further: docs/progress/1.0/build_command_table.py:36-37, :108, :154-155; docs/progress/1.0/command-table.json:163 (the only undo row, journal class); docs/progress/1.0/solving-workflow.md:80 (protected pieces may move inside a reviewed complete draft, so a draft append needs no per-twist protection gate); docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:133, :150 (a job result is read only when its digest matches the shown revision); work/experiments/magic600-04/adapter.py:2164-2166 (0.4 refused any submission while an earlier one ran: "input was not queued"); docs/progress/1.0/design-table/topics/command-keyboard-model.md:202, :244-253, :431, :439. Sketch: the solver types four live setup twists in quick succession. 0.4 refused those that arrived while the previous one ran. Here they commit in order; if protection refuses the second, the third and fourth are dropped and shown as dropped.
- needs_owner: yes

#### D10

- on: A4, A5, M6, D5
- kind: proposal
- claim: I accept A4's objection to D5: an unavailable palette entry keeps its row under the "unavailable" status pattern with its typed-reason glyph, instead of being dimmed as I proposed, and the focused entry shows the E6 message. I also replace D5's numbered levels with one placement rule that holds under any name: a key, like a term under charter bar M6, opens at the first learning step whose goal flow script needs it, so no goal, including the first checked insertion, meets a closed key. Owner, recommended option first:
  - Refusal messages outside the palette (A4): (a) shown at once for the first refusals of each category, in place and without taking focus or keys, then on request (A4 (b)); (b) only on request (A4 (a)).
  - Key scale (A5, M6): (a) no key numbers: keys open with the learning path's steps, each step named by its workflow step and opened with its concepts' term cards, and Jumble's keys open at the first Jumble on (A5 (a), M6 (a)); (b) a separate numbered key scale, K0 to K6, with a stated map to the learning levels (M6 (b), A5 (b)); (c) keys keep L0 to L6 and the other scales are renamed (A5 (c), M6 (c)).
- evidence: new evidence: docs/progress/1.0/glossary/terms.json holds 30 terms, and none of them is preview, commit, draft, phase, macro or undo. So M6 (a)'s rule alone cannot place the commit path's keys. The goal rule can, as a script over the flow scripts (docs/progress/1.0/command-table.md:69). "L" already names three scales: the reward ladder L0 to L7 (docs/progress/1.0/design-options-2026-10.md:83-90), the glossary levels L1 to L6 (docs/progress/1.0/glossary/glossary.md:7-14) and the draft key levels L0 to L6 (docs/progress/1.0/design-options-2026-10.md:210). Further: docs/progress/1.0/charter-2.0-draft.md:52, :93 (E6), :102 (M6: each term enters at the level where it is first needed), :111 (D7); docs/wiki/decisions/owner-decisions-2026-10-02-design.md:33, :62, :85 (error messages are exempt from the text budgets); docs/progress/1.0/dispositions/dispositions.md:438 (the same gates for every invocation, so a commit from the palette keeps its key's gates); docs/progress/1.0/design-table/topics/command-keyboard-model.md:210, :261, :269-274, :399-402. Sketch: the goal "first checked insertion with the solver's own macro" uses target, buffers A and B, macro entry, the draft destination, grip, twist, preview, commit and undo, so all of them open at its step under any name.
- needs_owner: yes

### Interaction artist

#### A7

- on: M1, D2, A1, D1, E5
- kind: support
- claim: I support M1 and D2's design of seven axis keys plus one inverse modifier, under one glyph rule. On every screen-space surface (on-screen keyboard, palette, journal, macro text), a vertex-axis twist shows an undirected class mark and a half-turn shows an undirected glyph that stays the same while the modifier is held, because only the four vertex keys switch class. Only the hint on the gripped cell draws a turning sense; it is built in the cell's own space, not as a screen sprite, so it reverses exactly when the turn's picture does (M1's camera and Π₄ argument). For K1 and K5, legibility leaves two options each, and I would not carry D1 (c) or E5 (c). Switched sets tie a key's meaning to a set chosen long before, which is the 0.4 design the charter records as failed, and binding by character breaks the block's spatial shape on other layouts (E5's AZERTY case).
  - Owner, K1 (D1): (a) D1 (a), with each prefix group's second keys shown as key-cap and concept glyphs, a pending prefix carrying a status pattern with no time-out, and Escape closing it (D4); preferred. (b) D1 (b), with the palette for every rare command.
  - Owner, K5 (E5): (a) every command bound by position, with labels from the active layout (E5 (a)). (b) The grip-and-twist block and the number keys by position, and the conventional office chords by character, checked against each layout the solver uses (E5 (b)); preferred. Under (a), a chord bound at QWERTY's Z reads Ctrl+W on AZERTY, which goes against the solver's habit from every other program.
- evidence: no new evidence. Relies on research/Full_600cell_Puzzle_Theory.tex:1840-1848 and grips.py:27-28 (M1); docs/progress/1.0/design-table/topics/command-keyboard-model.md:33-34 (0.4's switched sets and the charter's verdict), :186 (D2), :194 (D3), :228 (A1), :453 (E5); docs/progress/1.0/charter-2.0-draft.md:49; docs/progress/0.4/snapshot/design/01_PRODUCT_BRIEF_ZH.md:61, :63; docs/wiki/decisions/owner-decisions-2026-10-02-design.md:41, :85
- needs_owner: yes

#### A8

- on: M2, D2, E2, A2
- kind: proposal
- claim: When the next target shares a hosting cell with the current one, a rebind changes about half the keys even under the rule that changes the fewest. So it should be shown in place, not as D2's old and new cap IDs side by side:
  - each changed key's future key-cap glyph sits at its new cap under A2's "pending" pattern, while the live glyphs stay solid;
  - the hint shows one count of changed keys;
  - cap IDs stay in the binding table and on focus;
  - applying the map moves each changed glyph to its new cap (A3's key-remap row; timing is G5's).

  I change A2 and propose M2's "reference the solver set" as the order: a rebind keeps every cap that still affects the new anchor on its key, and only the freed keys get new caps, hosting cells first and then by cell ID, with hosting cells told apart by a role glyph. The rule reads only the applied map and model data, so E2's binding-digest test still holds.
  - Owner, for the rebind order: (a) the rule above, which changes the fewest keys; preferred. (b) Hosting cells on fixed first keys, with the other shared caps keeping theirs. (c) D2's order (hosting cells first, then cell ID), recomputed at each rebind.
- evidence: new evidence: a read-only count with the standard library, model files only, over assets/model.npz (orbit_id, mask_offsets, mask_values, face_offsets, face_values; hosting cells and caps as core.py:98-99). It covers all 3,388,320 ordered pairs of moving positions in the same orbit that share a hosting cell, which is the cell-local next target of docs/progress/1.0/solving-workflow.md:9 and :43. The maps in these pairs hold 10.75 caps on average. Keys whose cap changes per step:
  - (c): 9.01 on average, which is 82.7% of the keys; at least half the keys change in 92.6% of steps; 15.51, 17.78 and 19.76 in O32, O33 and O34.
  - (b): 6.76.
  - (a): 5.56, the minimum, which is 50.3% of the keys; at least half change in 70.0% of steps; 10.60, 14.00 and 15.00 in O32, O33 and O34.

  These counts cover every such pair in the model; they are not observed solver steps. Further sources: docs/progress/0.4/snapshot/design/01_PRODUCT_BRIEF_ZH.md:59 (hosting cells get priority; affecting caps are told apart), :61 (the binding table shows each key's cap ID); docs/wiki/decisions/owner-decisions-2026-10-02-design.md:41 (exact IDs on focus); docs/progress/1.0/design-table/topics/command-keyboard-model.md:253 (A3).
- needs_owner: yes

#### A9

- on: D6, M4, E6, A6, M3
- kind: support
- claim: I change A6 and support D6's seat keys, because a seat has a picture: I_a carries the cell's tetrahedron onto exactly five tetrahedra, all inscribed in one regular dodecahedron. The face match's candidates (at most 12) are the twists that bring one of these tetrahedra home: one held seat key, alone or with each of the cell's own eleven twist chords. Under M4's axis-then-angle input, the same 12 candidates lie on 12 different axes at four angles, and none of these is an axis of the cell. The seat glyph, and the outline drawn while a seat key is held, are fixed geometry and never come from the state, so nothing marks which seat passes the face match (DT-J1). Four seat keys also fit beside the 20 grip slots and 7 twist keys (31 of E2's 47 character keys). Adding 24 axis keys instead would need 51 keys, which forces E6 (a)'s Jumble-on layer, the kind of key mode that D3 excludes.
  - Owner, for the 48 jumble twists: (a) D6 (a), seat keys, preferred. Each key-cap glyph marks one of the five tetrahedra (the cell itself is the home seat), and each seat's base twist is pinned as a rotation, never as an index (M3). While a seat key is held, the twist keys show the order glyph of the twist they would make (A6), and the drag is available beside them. (b) M4's axis-then-angle input as a prefix group, with its 24 further axes picked on the same dodecahedron (E6 (a) without a layer). (c) The drag and the palette only, with the palette naming a twist by seat and twist rather than by index (D6 (c), E6 (c)).
  - I would not carry typing the menu index (D6 (b), E6 (b)): a numeral has no picture and is tied to the order digest (M3). In every option, Jumble on and Jumble off are plain commands (D6, E6), and one status pattern shows Jumble's state in both views. No colour is assigned here.
- evidence: new evidence: a read-only computation with the standard library. It applies the 60 exact items of the menu record in research/jumbling/fixtures/wj-I_a.json (Q(√5) entries, evaluated in floating point) to the four vertices of cell 0. The vertices were solved from the facet equations n_e · x = |n|² using the normals in assets/model.npz; the result is a regular tetrahedron with edge 3.0557.
  - The items carry the cell onto 5 distinct tetrahedra, 12 items each, and items 0 to 11 leave it in place.
  - The 20 vertices of the five tetrahedra have a regular dodecahedron's distance spectrum: 30, 60, 60, 30 and 10 pairs at ratios 1, φ, 2.288, φ² and 2.803.
  - For each other seat, the items that carry it onto the cell form one right coset of items 0 to 11: the retained twist acts after the seat, which settles D6's coset side. Each coset has three items each at 72°, 120°, 144° and 180°, on 12 distinct axes, none of them among the 7 retained axes.
  - The 31 axes pass through the dodecahedron's 15 edge-midpoint pairs, 10 vertex pairs and 6 face-centre pairs. None of the 24 non-retained axes meets a vertex or an edge midpoint of the cell.

  The fact the glyph relies on is the orbit-stabiliser count, 60/12 = 5 (charter M4). Further sources: research/jumbling/manual/02-descent.md:43-44; research/jumbling/theory/groups.py:59; docs/progress/1.0/design-table/topics/jumble-solver-aids.md:948, :970; docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:130; docs/progress/1.0/design-table/topics/command-keyboard-model.md:194 (D3), :337-340 (M3), :423 (E2).
- needs_owner: yes

#### A10

- on: D5, A4, E4, M6, A5, D4
- kind: proposal
- claim: I merge A4 into D5: an unavailable command keeps its place in the palette's match ranking, because a list sorted by availability reorders itself while the solver reads it whenever a job ends. Every unavailable row carries the "unavailable" pattern and a reason-category glyph, and only the focused row shows the full E6 message; E4's per-revision availability table supplies this without an engine call. I also merge A5 (a) into M6 (a): each key opens with its concept's term card, on one scale shared with the learning steps. The steps are named by workflow step with a glyph, never "L", because "L" already names the glossary levels, the reward ladder's levels and the Local view, and the owner wants the key and reward scales named apart.
  - Owner, for refused key presses (A4): (a) the first refusal of each category also shows its E6 message at once, in place; later ones show only the glyph, with the message on request; preferred. (b) The message only on request.
  - Owner, for the key scale (M6, A5): (a) one scale shared with the learning steps, as above, with Jumble's keys opening the first time Jumble is turned on (M6 (a), A5 (a)); preferred. (b) A separate key scale with its own word and numbers, and a stated map to the learning steps (M6 (b), A5 (b)). I would not carry M6 (c) or A5 (c).
  - Owner, for undo (D4): (a) one key whose hint shows the glyph of what it would take back (D4 (a) with A3); preferred. (b) Two keys, one per undo class (D4 (b)). I would not carry D4 (c): under D3 the focused view never changes a key's command, so undo would be the only key whose effect follows focus.
- evidence: new evidence: a sketch. While a check runs, the solver types "pre" in the palette and rests the cursor on the second row. The check ends and Preview becomes available. A list sorted by availability then lifts Preview above the cursor, so the Enter the solver is about to press runs a different command. Further sources: docs/progress/1.0/design-table/topics/command-keyboard-model.md:194 (D3), :210 (D5), :261 (A4), :269-274 (A5), :399-402 (M6), :441 (E4); docs/wiki/decisions/owner-decisions-2026-10-02-design.md:62, :85; docs/progress/1.0/glossary/glossary.md:7-14; docs/progress/1.0/design-options-2026-10.md:85, :210; docs/progress/1.0/charter-2.0-draft.md:64, :93
- needs_owner: yes

### Mathematician

#### M7

- on: D6, E6, A6, M4
- kind: proposal
- claim: D6's seats work only as right cosets A4_c·s, typed as "seat, then twist key" (the twist t·s). The face-match candidates for the twist h that opened a cut are the 12 twists of the right coset A4_c·h⁻¹, and that coset meets each of the four jumble left cosets in exactly 3 twists, so left-coset seats would spread the 12 candidates over all four seats. With right cosets, three things hold: each seat is named by the vertex axis it shares with the cell, which is fixed by geometry once the four axes are named and does not depend on the item order; a seat alone or with one of the 11 twist chords types each of the 48 jumble twists exactly once; and only each seat's base twist needs a reference beyond the seat itself (a second axis name or a recorded rule), because the third-turn pair that fixes the seat moves all 12 of its twists.
  Owner options for item C6, one set for D6 and E6:
  - (a) Seat keys as fixed here. While a seat is held, the twist keys show the rotation they will actually perform (D6 (a)).
  - (b) Axis and angle over the 24 jumble axes, with recorded axis names and a sense rule on the 12 third-turn and fifth-turn axes (E6 (a), M4).
  - (c) The menu index, typed after the grip or given to the palette, beside the pointer drag. The index is resolved at once to the rotation and cites the order digest (D6 (b) and (c), E6 (b) and (c), M3).
  My position is (a), as preference: seats add four names to the retained alphabet. This changes M4's proposal, which becomes option (b). M4's objection to the A1 input stands.
- evidence: new evidence: exact Q(√5) products of the 60 matrices in the `menu` record of research/jumbling/fixtures/wj-I_a.json, composed as research/jumbling/sim/state.py:403 does (new pose = twist·pose). Conjugation by F_c carries every result to each cap (research/jumbling/sim/twists.py:433-436). Results:
  - A4 (items 0 to 11) is not normal in the menu.
  - Each of the 4 right cosets meets each of the 4 left cosets in exactly 3 twists. For every jumble twist h, A4·h⁻¹ meets every left coset in 3.
  - For each right coset A4·g, A4 ∩ g⁻¹A4g is the identity plus one retained third-turn pair. This gives a bijection onto the 4 vertex axes.
  - Conjugation by that pair keeps the coset and moves all 12 of its twists.
  - Each right coset holds 3 twists of each angle. Both chord rules cover the 48 exactly once.
  Side: domains are left cosets γK⁺ (research/jumbling/theory/restoration.md:39), so a cut whose outside is the lattice closes under A4_c·h⁻¹ (restoration.md:79, :100-102; research/jumbling/manual/02-descent.md:43-44). Counterexample to left-coset seats: step 1 of the manual's example, (72, 18), has 12 closing twists (02-descent.md:67); left-coset seats would need three chords on each of the four seats. "Seat" is unused in the glossary, the theory and the manual, so it can enter as a new term (docs/progress/1.0/charter-2.0-draft.md:96, bar M1).
- needs_owner: yes

#### M8

- on: A6, E6, A1, D2, M1
- kind: support
- claim: I support A6 and E6 on the condition from M3 (keys bind to rotations, never to item numbers), and D2's inverse modifier and A1's glyph pairs on the conditions from M1. On every cap, the unmodified vertex-axis key types the twist in the class of generator 2c+2 (0.4's T3 and T3′ swap names; which class is "forward" is preference); the modifier is shown as having no effect on a half-turn; and a held seat (M7) relabels the twist keys, because a seat chord never turns about its key's own axis (the 7 retained axes carry no jumble twist, M4). M1's class rule has no jumble counterpart: A4_c moves all 24 jumble axes and conjugates every jumble twist to its inverse, so the cell's symmetries keep only a jumble twist's angle (72°, 120°, 144° or 180°), and its axis and sense need a named reference, such as the seat it brings home (M7); its glyph sense must therefore be drawn from the rotation actually performed, as A6 proposes.
- evidence: new evidence: exact Q(√5) computation on the 60 matrices of the `menu` record of research/jumbling/fixtures/wj-I_a.json, as in M7:
  - Items 0 to 11 form a group: the identity, 3 half-turns and 8 third-turns.
  - A4_c conjugation splits the 48 jumble twists into four orbits of 12, one per angle. So the 12 two-fold, 6 three-fold and 6 five-fold jumble axes each form one orbit.
  - Every jumble twist is conjugate to its inverse under A4_c; for each 72°, 120° and 144° twist a retained half-turn does it. No retained third-turn is conjugate to its inverse.
  Other sources: M1's class split from round 1 (docs/progress/1.0/design-table/topics/command-keyboard-model.md:303-305); grips.py:9 (generators 2c+1 and 2c+2); research/Full_600cell_Puzzle_Theory.tex:903-907 (proof of Lemma 6); research/jumbling/sim/twists.py:433-436 (menus transported by F_c); docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:82 (with Jumble on, every twist, retained ones included, passes the admissibility check).
- needs_owner: no

#### M9

- on: D2, A2, E2, M2
- kind: support
- claim: I support D2, A2 and E2: their counts are exact (at most 20 affecting caps, with the hosts always among them), so one page of 20 slots reaches every cap that acts on the anchor, and E2's binding-digest test is the right check. "Fixed model order" has two admissible definitions, and the choice between them is the designer's. One is the ascending cap ID the model stores, under which a slot has no stable role within an orbit. The other is Lemma 7's reference-and-transporter construction applied to M(A): slot i keeps one role at all 136,800 positions of the 19 orbits with N_o = 7,200, while at the other 39,720 positions (O32 to O34 among them) it does so only up to a position stabiliser that moves caps of M(A), so cap ID breaks the tie. The screen stays out (M2).
- evidence: new evidence: standard-library computation on assets/model.npz (rotperms, frameperms, mask_offsets, mask_values, face_offsets, face_values and orbit_id; model files only, no session):
  - K⁺ = {F_c·s : s in A4_0} has 7,200 pole permutations and contains all 1,200 generators.
  - Each position's cap list is stored in ascending ID order.
  - Caps per piece: 1 to 19 with one host, 2 to 18 with two, 5, 10 or 15 with five, and 20 with twenty, as A2 counts. The hosts lie among the caps at all 177,120 positions.
  - The set stabiliser of M(A) has order 7200/N_o, which matches research/Full_600cell_Puzzle_Theory.tex:795-829. It moves caps of M(A) in all 16 orbits with N_o < 7,200: in O32 it swaps nine pairs, the two hosts included; in O33 it fixes the host and permutes six triples; in O34 it has one orbit of 20.
  - Transporting one reference position's ascending order by all of K⁺ gives an ascending list 22 times at O04 and only once (the identity) at O31, O32 and O33.
  Lemma 6 (research/Full_600cell_Puzzle_Theory.tex:931: M(kA) = kM(A)); Lemma 7 (:969-972 for the construction, :977-978 on screen orderings). Both orders are static data per position, so E2's rebind stays a table lookup.
- needs_owner: no

#### M10

- on: D1, E5, A5, M6
- kind: proposal
- claim: No option of D1, E5, A5 or M6 is out, but three conditions bind. First, in each routing class the bound key sequences must form a prefix code (a group key is never also bound alone), and `file.keymap.check` must test this as well as duplicates. Second, a binding by character depends on the layout, so a key-map file with such bindings must record its layout and the conflict check must run once per layout. Third, under bar M6 no key may open before its concept's learning level; checking that needs a stated map between the two scales, because read index for index the draft opens stars and work sheets one level early. I merge M6 into A5's question and withdraw my "K0 to K6", because K already names items K1 to K7 in this record and the group K⁺ (preference).
  Owner options, one set per question:
  - Item K1 (D1):
    - (a) D1 (a), with the prefix-code check.
    - (b) D1 (b).
    - (c) Switched sets, only if the active set is always shown and is stored in each binding.
    Mathematics does not rank (a) against (b).
  - Item K5 (E5):
    - (a) All bindings by position. This is the only option whose key map does not depend on the layout; it is my position, on definability grounds only.
    - (b) Mixed, under the two layout conditions above.
    - (c) All bindings by character, under the same conditions.
  - Item K3, the key scale (A5, M6):
    - (a) A key opens at its concept's learning level, and Jumble keys open at the first Jumble on. This meets bar M6 by construction; it is my position.
    - (b) A key scale with its own name and a map that meets bar M6.
    - (c) L0 to L6 kept for keys, with the learning levels and the reward ladder renamed and the same map.
- evidence: new evidence:
  - Counterexample: Shift+KeyQ bound by position (for example an inverse-twist chord) and Shift+A bound by character are two different chords on US QWERTY but one chord on French AZERTY, where KeyQ types A (E5, docs/progress/1.0/design-table/topics/command-keyboard-model.md:453).
  - Sketch: if a group key P is also bound alone, the key stream P, Q reads either as two commands or as one group command, and only a timeout can decide. With each chord counted as one symbol, a prefix code reads every key stream at once.
  - Scales: docs/progress/1.0/charter-2.0-draft.md:102 (bar M6); docs/progress/1.0/glossary/glossary.md:7-14, :202, :264, against docs/progress/1.0/design-options-2026-10.md:210; docs/wiki/decisions/owner-decisions-2026-10-02-design.md:62.
  - The letter K: items K1 to K7 at docs/progress/1.0/design-table/topics/command-keyboard-model.md:84-120, and K⁺ at research/Full_600cell_Puzzle_Theory.tex:529.
- needs_owner: yes

### Engineer's advisor

#### E7

- on: D2, A2, M2, D1, E2, E5
- kind: support
- claim: D2's re-anchor command and A2's "remap pending" state are one mechanism, and it meets my E2 conditions. The applied binding (20 slots to cap IDs, hosting cells first, then M2's cell-ID order) is stored as workspace state, and the one pending proposal beside it is a static mask lookup: it needs from the engine at most the anchored piece's position in each snapshot (nothing for D2's position anchors), so it needs no job, no synchronous call (R-14) and no view-model copy of the state. I raise E2's count to 31 character keys (20 slots, 7 axis keys and D6's 4 seats if adopted), which fit without the digit row so that O9's number keys never shadow a grip key; engineering is neutral among D1's options, which all resolve through one binding table, and I merge my E5 options (b) and (c), since D1 and E5 already bind the block by position:
  - (a) every key by position, with its label drawn from the active layout;
  - (b) the block and O9's keys by position, and mnemonic chords and prefix second keys by character, with the conflict check run against each layout the solver uses.
- evidence: new evidence: docs/progress/1.0/command-table.md:24 (the view model keeps no copy of the puzzle state, so a moving anchor's position must come from the snapshot) and :22 (the workspace and its revision live in the session store). The applied cap list is stored, not re-derived from the anchor: after a restart, re-deriving it from a piece that has moved would apply a pending remap silently (docs/progress/1.0/design-table/topics/command-keyboard-model.md:90). Count: a US ANSI main block has 47 character keys, 37 without the ten digit keys, and the block needs 20 + 7 + 4 = 31; D2's inverse modifier is a modifier key. A grip slot on a digit would grip or copy a repair row depending on focus, which D3 forbids (docs/progress/1.0/design-table/topics/command-keyboard-model.md:194; docs/progress/1.0/design-table/topics/jumble-solver-aids.md:1006). Sources: core.py:45-47 (mask arrays); docs/progress/1.0/build_command_table.py:113 (`grip.capture`, workspace undo). Experiments, no cost claimed: a headless test that camera, filter, twist and focus events leave the slot-binding digest unchanged and only the re-anchor command changes it; the slot hints and A2's same-frame grip-outline drop go into H-06 and H-09 (docs/progress/1.0/renderer-experiment-plan.md:159, :162).
- needs_owner: yes

#### E8

- on: D3, D4, E3
- kind: support
- claim: I support D3's routing classes and D4's commit chord, Escape rule and `twist.apply` split, provided EN4's builder also checks the G6 binding table against them: repeat only on D3's camera, list-movement and preview-scrub rows; no text-class key reaching a command; list-class keys reaching only navigation, inspection and O9's row copy; and one undo class per row that holds for every destination. The repeat check is narrower than my E3 "view rows", which would let toggles such as `display.frame.toggle` flip on every repeat. Any workspace-undo row must run R-05's single permission path, because undoing `protection.position.protect` releases a position without the `confirm` that `protection.position.release` requires; D4's option (c) lets focus change a global key's command, which D3 forbids, so I narrow D4's owner question to:
  - (a) one undo key for the latest action of either class, with one durable action order across journal and workspace in the session store; each undo carries the action number its hint named and is refused as stale if another action landed first;
  - (b) two undo keys, one per class.
- evidence: new evidence: docs/progress/1.0/build_command_table.py:102 (`protection.position.protect`: no gate, workspace undo) and :103 (`protection.position.release`: `confirm`), against docs/progress/1.0/requirements-from-screening.md:15 (R-05: no second path skips the permission check, so its test must also enumerate the undo of each protection row). Count: 24 rows carry `undo="workspace"` and no row performs that undo (`grep -c 'undo="workspace"'` on docs/progress/1.0/build_command_table.py). Further lines in the same file: :123, :70 and :106 (view toggles that E3's rule would let repeat); :74 (`object.inspect` repeats, outside D3's list); :108 (`twist.apply`: one journal class for both destinations); :154 (repeat is checked on `commit` rows only). docs/progress/1.0/command-table.md:22 (one session store holds journal and workspace) and :68 (the editor rule, which nothing checks); docs/progress/1.0/design-table/topics/command-keyboard-model.md:194 (D3: the focused view never changes a key's command). E3's round 1 counterexample still holds. Unmeasured: the cost of one shared action order. A headless test that sends an undo naming a superseded action settles the stale refusal.
- needs_owner: yes

#### E9

- on: A4, D5, E4
- kind: proposal
- claim: I support A4 and D5 listing an unavailable command with its typed reason. Both read the engine's per-revision availability, one row per command with one of the ten categories, so the palette makes no engine call per keystroke; dimming versus a pattern with a category glyph, and A4's timing question, make no engineering difference (A4 (b) needs only a per-category count kept with the preferences). Fix first, whatever the owner picks: drop `not_busy` from `session.undo` and exempt undo from the builder's commit rule, because the draft refuses undo while any job runs, against charter E4's "undo is always available"; that and rule 3, which forbids showing an old-revision result after a commit, remove my E4 options (b) and (c) as stated, so the owner question becomes:
  - (a) a commit or undo proceeds at once and cancels the running jobs whose results belong to the old revision (previews, checks, the analyser, covers, repair lists); file jobs finish, and "busy" refuses only an action whose lane is held (R-16). This is my preference.
  - (b) a commit other than undo is refused as busy until the job ends or is stopped, as in 0.4, while undo always proceeds and cancels those jobs.
- evidence: new evidence: docs/progress/1.0/charter-2.0-draft.md:91 (E4: "undo is always available") against docs/progress/1.0/build_command_table.py:36 (`session.undo` is gated `not_busy`) and :155 (every commit row except the timer rows must carry `not_busy`), which repeats 0.4's rule (work/experiments/magic600-04/adapter.py:2164-2166). docs/progress/1.0/jumbling-observation.md:35 (rule 3: after a commit, a per-revision result is recomputed or discarded and never shown against a newer state) and :176 (the repair list took 3 to 5 minutes per cap, a research timing), so my old option (b) would hold the exclusive lane for a list that rule 3 discards. docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:150 (each jumbling job result carries its input digest, so the command layer knows which jobs a commit cancels); docs/progress/1.0/command-table.md:21 and :23 (the engine gives each command's availability, and the command layer exposes it), :31 (the ten categories). While a job runs, option (b) makes a commit two actions, which the settle step should read against S4 (docs/progress/1.0/charter-2.0-draft.md:83). Experiments, no cost claimed: B-004's admission fixture (docs/progress/0.4.1/screening-findings.md:67) with a commit and an undo during a running job, run headless; commit latency under (a), measured with the candidate engine on the owner's machine.
- needs_owner: yes

#### E10

- on: M4, D6, E6, M3, A6
- kind: proposal
- claim: Fix first for ENG-0J:145: in menu mode, M4's exact axis-and-angle lookup replaces the A1 input map. It is one table built once from the pinned menu, holding each of the 59 items with its axis, angle and inverse (which also gives A6's glyph by order), and every route, drag and seats included, stores the resolved item as a (grip, index) letter under the order digest or as its exact matrix, so that, as M3 requires, no key, seat, axis number or list row reaches the journal, a macro or a receipt. D6's seats are one key layout over that table; it works only when the seat representatives and the coset side are pinned together and match the side of the manual's face-match candidates, and a key per axis does not fit beside the grip block. So I replace my E6 options with the three below and drop the typed menu index (D6 (b), my E6 (b)), which ties typed input to the order digest and gives a global meaning to the digit keys that O9 uses:
  - (a) D6's four seat keys, held with the retained twist keys;
  - (b) M4's axis then angle, with the axis named through a prefix group or a pointer pick;
  - (c) ENG-0J's pointer drag and the palette only, with the palette taking (grip, axis, angle).
- evidence: new evidence: I computed exact products of the 60 `menu.base` matrices of research/jumbling/fixtures/wj-I_a.json (entries (a + b√5)/q as Python Fractions; python3 -I with the standard library; the script is kept outside the repository), with A = items 0 to 11. Results:
  - A is closed under products, and none of the 4 other left cosets g·A is a right coset.
  - With the smallest left-coset representatives (items 12, 24, 25 and 27), s·a types all 48 jumble twists but a·s types only 12 of them.
  - Each of the 44 chords that join a seat to a twist key gives a different item under the two sides.
  - Each right coset outside A meets all four left cosets, 3 items each. With the wrong side, the 12 candidates of one face match (research/jumbling/manual/02-descent.md:44, which does not state the side) would spread 3 + 3 + 3 + 3 over the four seats.
  - Every item's inverse is an item, which Cleanup = inverse of Prepare needs (docs/progress/1.0/build_command_table.py:55).
  Count: 20 slots + 31 axis keys = 51, more than the 47 character keys of a US ANSI main block. Sources: research/jumbling/sim/twists.py:421, :425; research/jumbling/state-contract.md:131; docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:130, :141, :145; docs/progress/1.0/design-table/topics/jumble-solver-aids.md:954 (fix-first 1: order dependence), :974 (E1: a jumble letter is refused while Jumble is off), :1006 (O9). The command layer then needs one menu-twist row whose argument is the resolved item, refused with a typed error for items 12 to 59 while Jumble is off. Each input needs one pinned convention: for M4, an axis order and an angle sense per axis; for D6, the representatives together with their side. Experiments, headless: each route reaches all 59 items of every grip, and one journal replayed under two key maps gives one state hash.
- needs_owner: yes

## Round 3

Run on 22 October 2026, on two points of round 2 that no other role had answered: A8's rebind counts with M9's two slot orders, put to the designer, and D9's commit queue against E9, put to the engineer. Each answered in one card. The cards are appended unchanged; only the layout differs, with one heading per card and one list item per field.

### Front-end designer

#### D11

- on: A8, M9, M5, D7
- kind: proposal
- claim: Of M9's two admissible orders I choose the transported one. I support A8's in-place display under every rule, but not A8's cap-sticky order: on a cell-local next target, every rule moves more than half the keys to new caps (5.56 of 10.75 under A8's rule, 9.75 under the transported order). A8's rule still changes the role of 83.5% of the keys and depends on the rebind history. The transported order changes no key's role in the 19 free orbits, whether the anchor moves to a next target or follows the piece through a twist, and the twisted cap keeps its key. Twist keys share that stability only if each slot also carries its transported frame (M5's "frame in the rebind rule"; the 0.4 brief's "C ID, frame and twist"), because D7's fixed cap frames keep a twist key's role for only 7.3% of keys. So option (a) replaces D7's fixed frame. Owner, recommended option first:
  - (a) Piece-relative keys: M9's transported order, hosts first and ties broken by cap ID, with each slot carrying its transported frame. In the 19 free orbits every grip and twist key then keeps one role relative to the piece, and the on-screen keyboard can teach one arrangement per orbit.
  - (b) A8's cap-sticky order with fixed frames (A8 (a)).
  - (c) D2's ascending cap ID with hosts first and fixed frames, recomputed at each rebind (A8 (c)).
  - A8 (b) leaves the batch: like (b), it depends on the rebind history, it changes more caps (6.76), and it keeps no roles.
- evidence: new evidence:
  - Method: a read-only computation with the Python standard library on assets/model.npz (frameperms, rotperms, orbit_id, mask_offsets, mask_values, face_offsets, face_values; model files only; script piped on stdin, nothing written).
    - K⁺ = {F_c·s} has 7,200 elements and contains all 1,200 generators. The signature (hosts, caps) identifies all 177,120 positions.
    - The transported order: each orbit's smallest position is the reference, ordered hosts first and then by cap ID. Every element of K⁺ transports it, and the lexicographically smallest transported list is kept.
    - Hosts stay first because Host(kA) = kHost(A) (research/Full_600cell_Puzzle_Theory.tex:931). This is Lemma 7's construction (:969-970), which "arbitrary independent screen orderings" lack (:977-978).
  - Cell-local next targets: the computation reproduces A8's figures (3,388,320 pairs, 10.75 caps, 5.56 and 9.01 changed caps).
    - The transported order changes 9.75 caps (O32 17.27, O33 17.21, O34 19.83).
    - In the 19 free orbits (2,800,800 pairs, 10.83 caps), roles change for 9.04 keys under A8's rule (starting from the transported map), 8.92 under D2's order and 0 under the transported order.
  - Following the piece: anchor X, every twist t of every cap of X, rebind at t(X); 446,600 steps over 200 sampled positions per free orbit, 11.87 caps per step.
    - Cap changes: 7.82 under A8's rule, 11.07 under D2's order, 10.38 under the transported order.
    - Role changes: 9.05, 9.88 and 0.
    - The twisted cap keeps its key at every step under both A8's rule and the transported order.
  - Frames: under the transported order with fixed cap frames, a twist key keeps its role across a cell-local step for 12,261 of 168,520 keys (7.3%; 40 sampled anchors per free orbit), because F_{k(c)}⁻¹·k·F_c is rarely the identity. A transported frame keeps it at every key by construction, and it keeps M1's sense class, which K⁺ preserves.
  - Both orders and both frames are static data per position, so E2's rebind stays a table lookup and its digest test holds.
  - Sources:
    - docs/progress/0.4/snapshot/design/01_PRODUCT_BRIEF_ZH.md:59 (hosts first), :61 (each key shows "which C ID, which frame, which twist");
    - docs/progress/0.4/snapshot/experiment/NATIVE_REVIEW_GUIDE.md:77 (0.4 planned ordered-frame capture; cell-only capture used the fixed cap basis);
    - docs/progress/1.0/solving-workflow.md:9 (cell-local batching), :68 (following the piece), :106 (piece-focused context);
    - docs/progress/1.0/charter-2.0-draft.md:49 ("once learned");
    - docs/progress/1.0/design-table/topics/command-keyboard-model.md:381 (M5).
  - Sketch: in a free orbit the solver learns once that one key grips the cap across the piece's first host and that a given twist key turns it toward the buffers. Every later target in that orbit, and every re-anchor during a setup, keeps both meanings. Under (b), the solver re-reads about half the glyphs after each rebind, and a role learned at one target does not carry over.
- needs_owner: yes

### Engineer's advisor

#### E11

- on: D9 (its commit queue), E9, E8
- kind: objection
- claim: An ordered queue fits R-16, because one queue in front of one atomic admission still keeps one commit in flight. It fits R-18 if key repeats are dropped before they are queued. D9's version, however, contradicts the redesign rules of the twist units it serves:
  - U5-011 says busy input "cannot queue extra turns", and U6-013/2 suppresses busy input.
  - U4-061 blocks a stale pre-state, so each queued twist is either refused or admitted against a state the solver has not seen.
  - Queuing only keyed commits breaks the single busy gate that U4-027 and U4-061 set for key, pointer and menu invocations.

  The queue handles a commit typed during another commit, so it should not sit inside E9 (a). E9 (a) is about query jobs, keeps E9's two options and combines with either answer here. I propose the queue as its own owner question, which reopens those units on the record; they are redesign decisions, not among the 18 the owner signed:
  - (i) a commit typed while another commit is in flight is refused as busy and shown in place, as the units say and as E9 (a) assumes. This is my preference until the experiment below shows bursts of typed twists outrunning a commit.
  - (ii) D9's queue, with the units reopened, under these conditions:
    - one queue for every channel;
    - only the 6 commit rows without `confirm` or `fresh_preview` may enter it;
    - arguments are resolved at key-down (M3);
    - all gates are rechecked at the head of the queue (AGENTS.md:10);
    - each entry's pre-state guard, and a queued undo's target, name the commit ahead by its durable command ID instead of E8's action number;
    - repeats are dropped before they are queued;
    - at the first refusal, protection included, the rest are dropped and counted, as D9 says.
- evidence: new evidence:
  - Twist and undo units:
    - docs/progress/1.0/dispositions/dispositions.md:519 (U5-011: "held-key repeat, busy, stale, text, modal and IME input cannot queue extra turns");
    - :602 (U6-013/2: twists "carry state/frame guards"; "Suppress repeat, busy, modal and editor input");
    - :476 (U4-061: the same permission, busy and context gates for every key, pointer and menu invocation; a recheck before commit; "Block ... stale pre-state");
    - :438 (U4-027: the same gates for undo);
    - :181 (U1-069: "busy availability shared by all entry paths").
  - Authority of these rules: dispositions.md:5 and docs/wiki/decisions/owner-decisions-2026-10-03.md:63 show the owner signed only the 18 delete and automate decisions. The redesign rules therefore bind through item C1 (docs/progress/1.0/design-table/topics/command-keyboard-model.md:50) and can be reopened on the record.
  - Count: 6 of the 11 commit rows carry neither `confirm` nor `fresh_preview` (docs/progress/1.0/build_command_table.py:31, :32, :36, :37, :38, :108). At the head of a queue, the other 5 (:29, :35, :39, :44, :60) would meet a different revision from the one their confirmation or preview showed.
  - docs/progress/1.0/command-table.md:26: the renderer never draws an uncommitted state, so queued twists land after what the solver saw.
  - command-table.md:30: the durable command identity is the only absolute name for a commit that has not landed, as M3 requires.
  - docs/progress/1.0/requirements-from-screening.md:33, :35 (R-16, R-18).
  - docs/progress/0.4.1/screening-findings.md:69 (F-02: a held key appended a macro repeatedly). Behind a queue, such repeats would keep landing after the key is released.
  - docs/architecture/1.0/10_V1_ARCHITECTURE.md:513 (a proposal: `busy` means waiting or an explicit cancellation, never blind command replay).
  - Experiments, nothing claimed:
    - Measure the commit latency of a live twist with the candidate engine on the owner's machine, against the gaps between key presses in typed twist bursts during the living test. If every commit ends within the shortest gap, (i) refuses no key in practice and the question closes without a queue.
    - Run B-004's admission fixture (docs/progress/0.4.1/screening-findings.md:67) headless, with concurrent key and pointer submissions and a held key, to check (ii)'s conditions.
- needs_owner: yes

## Outcome

Settled by the main session on 22 October 2026 from the cards above, in the order of the [round protocol](../README.md): the filter, demonstrated failures, the 8B order among admissible options, and experiments for unmeasured objections. Nothing here is an owner decision until the owner answers the batch at the end.

Round 2 produced new evidence, so the protocol allowed a third round. Most of it converged. Two points had no answer from another role: A8's rebind counts with M9's two slot orders, and D9's commit queue, which contradicted E9. The targeted third round put the first to the designer and the second to the engineer, one card each (D11, E11). Some claims of rounds 2 and 3 have not been answered by the other roles: D11's transported frames, E11's reading of the unit rules, M10's prefix-code condition and D10's goal rule. Each rests on a computation or a reading of the records; the outcome adopts them as stated or puts them to the owner.

### Out by the filter

- A twist key named by a screen direction, such as "clockwise". One camera move reverses the drawn sense of the same rotation, so the name would change meaning with the camera, which section 4.3 of the solving workflow forbids (M1, A7).
- A grip map derived again from the anchor at any time other than the solver's rebind, a restart included. If the anchored piece has moved, a pending remap would apply silently (E7), so the applied map is stored.
- A palette that hides an unavailable command. The owner's palette always offers every command (2 October, section 5; A4, D5, D10).
- Undo refused while a job runs (E4's option (c), applied to undo). Charter bar E4 says that undo is always available (E9).
- A commit that lets the old revision's jobs finish and then shows their results as stale (E4 (b), D9 (b)). Rule 3 of the requirements draft and R-19 forbid showing a per-revision result against a newer state (E9).

### Fix first

These are demonstrated failures that affect the work or correctness. They need no owner answer.

1. **ENG-0J's axis-and-angle row cannot type the jumble twists.** The row "jumble twist with axis and angle input (through the A1 input map)" (ENG-0J, line 145) searches only rational parameters, and 48 of a grip's 59 I_a twists have none (M4; the main session re-checked this exactly, and only items 1 to 11 are reachable). Fix: in menu mode, one exact table built once from the pinned menu gives each of the 59 items with its axis, angle and inverse (E10). Every input route, the drag included, stores the resolved item as a (grip, index) letter under the order digest or as its exact matrix (M3, E10). The row gives way to the input that the owner picks in Q7 (D8).
2. **The builder checks repeat on commit rows only.** `build_command_table.py` enforces R-18 at line 154 for `commit` rows alone. A table in which `macro.add` (F-02's own failure: a held Enter appends a macro) and the `operation.preview` job repeat still builds (E3's counterexample). Fix: repeat is allowed only on D3's camera, list-movement and preview-scrub rows (E8). E3's broader rule, repeat on every view row, would let toggles such as `display.frame.toggle` flip on every repeat (E8). Under the narrower rule, `object.inspect` (line 74) stops repeating. The builder also checks the G6 binding table against D3's routing classes (E3, E8):
   - no key of the text class reaches a command;
   - keys of the list class reach only navigation, inspection and O9's row copy;
   - each row has one undo class for every destination.
3. **Undo is not always available.** `session.undo` is gated `not_busy` (`build_command_table.py`, line 36), and the builder demands `not_busy` on every commit row except the timer rows (line 155). Undo is therefore refused while any job runs, against charter bar E4, "undo is always available" (E9; checked). Also, 24 rows declare workspace undo, and no row performs it (E8; checked). Fix:
   - undo carries no `not_busy` gate and is exempt from that builder rule;
   - workspace undo gets a command under Q4;
   - the undo of a protection row runs R-05's single permission path, because undoing `protection.position.protect` releases a position without the `confirm` that `protection.position.release` requires (E8).
4. **The structure commands are not "from 0.3 documents only".** Section 5 of command-table.md says so (line 76), but their 0.4 source units are redesign decisions (E1; checked): U4-065, U4-066, U4-069 (its parts), U4-070, U4-072 (its parts) and U4-074 for cell views, centring, finding and comparing colours and layers, and U4-037 and U5-051 for saved sets. Fix: the structure commands stay, and the builder checks coverage over the 377 keep and redesign decisions that have a user entry point, instead of the 72 top-down rows (E1).

### Settled without the owner

Each item is allowed as stated, or under the condition given.

**Command model**

- **C1, the command set and its join.** One command per purpose, with arguments that name the object (D1, M3). EN4 joins units to commands per decision (a unit or a split part), after the disposition filter (E1):
  - about 205 reviewed alias rows join the 268 decisions that carry a 0.4 command ID (270 IDs under nine argument patterns);
  - the other 109 decisions are mapped one by one;
  - automate and delete units get no command;
  - the join is many-to-many, so a command takes the dispositions of all its units.
- **Absolute arguments.** Every stored argument names an absolute object: a cap, a position or slot, a piece identity, an orbit, a rotation by its exact matrix or K⁺ index, or a word. This holds for the journal, macros, drafts, receipts and the palette. A relative name (the current grip, a key, a twist label, a menu index, a list row) is kept only together with what fixes it, and is refused once that has changed (M3, E10).
- **C2, routing.** D3's fixed order decides where a key goes:
  1. a key-capture field;
  2. a text field, which takes every character key;
  3. a focused list or view, which takes only navigation keys: the arrows, Enter to inspect, Tab and O9's row-pick number keys;
  4. one global map for every other key.

  The input destination is the only mode that changes what a key does. The camera, a filter, hover and the focused view never change a key's command or argument (D3). Text undo and the clipboard keys stay with the focused editor (U4-064; D7, D9).
- **C3, the palette.** It lists every command.
  - An unavailable command keeps its place in the match ranking (A10), under the "unavailable" status pattern with its typed reason as a category glyph. The focused entry shows the full E6 message (A4, D10).
  - It searches labels, glossary terms, command IDs and key names, and shows each entry's key even before that key opens (D5).
  - It takes an object argument that defaults to the context (D5).
  - A commit from the palette runs the same gates as its key. A commit without a fresh preview shows that reason and offers the preview (D5, U4-027).
  - It reads the engine's availability table per revision, so it makes no engine call per keystroke (E4, E9).
- **C4, the commit path.**
  - Twist keys type into one shown destination, live or the selected draft phase. It changes only through the destination and phase keys, never through a commit, undo, focus or the camera (D4). `input.destination.set` stays as that key.
  - The commit is one non-repeating chord, not Enter. It works from any focus outside a text field while the preview is fresh (D4).
  - Escape closes only the innermost transient layer (palette, text edit, row focus, inspection). It never stops a job, cancels a preview or drops a draft; `job.stop` and preview cancel keep their own keys (D4, E4).
  - Destructive commits keep their `confirm` gate. No card asked for a further key.
- **C6, Jumble.**
  - Jumble on and Jumble off are plain commands with typed refusals and no confirming key (D6, E6, D8, A9).
  - The 11 retained twist keys keep their rotations with Jumble on. A jumble twist typed while Jumble is off is refused with a typed error (A6, M4, E10).
  - One menu-twist row takes the resolved item (E10).
  - DT-J1's query rows take the named cap from the context (D6, E6).
  - No key that previews a menu twist repeats (D6).
  - Nothing marks a seat, an axis or a chord according to the state (A9, D8).
  - Jumble's keys open the first time Jumble is turned on (A6, D8).

  The input for the 48 jumble twists is Q7.
- **Seats, if Q7 picks them.**
  - A seat is a right coset A4_c·s of the cap's retained group, typed "seat, then twist key". The chord types the twist t·s, with s performed first, as poses compose (M7, D8, A9). With left cosets, the 12 candidates of one face match would spread 3 + 3 + 3 + 3 over the four seats (E10, M7, D8; the main session re-checked the coset facts exactly).
  - Each seat is named by geometry, never by an item number: by the vertex axis that it shares with the cell (M7), drawn as one of the five tetrahedra in the cell's dodecahedron (A9).
  - Each seat's base twist is pinned as a rotation by one recorded rule, such as D8's powers of one pinned fifth-turn (M3, M7, D8).
  - While a seat is held, the twist keys show the glyph of the twist that they would make (A6, A9, M8).
- **C7, the draft table's open points.**
  - The timer rows write no journal entry and have no undo; 0.4 kept one total in its `meta` table. This also removes the builder's timer exception (E3).
  - `twist.apply` splits. The draft append drops the live twist's protection gate and journal undo, since protected pieces may move inside a reviewed complete draft (E3, D4, D9; solving-workflow.md section 4.1).
  - `bank.select` gives way to the grip anchor, and `bank.functions.toggle` gets no row (D1). `grip.capture` becomes the rebind command (D2, E2, E7).

**Keyboard model**

- **K1 under any option.** A pending prefix carries a status pattern and has no time-out, and Escape closes it. Prefix second keys show as key-cap and concept glyphs (A7, D4). The model itself is Q1.
- **K2, twist keys.** Seven axis keys and one inverse modifier type a cap's 11 twists (M1, D7, A1, A7, M8). They act in the cap's fixed frame, or under Q3 (a) in the frame that the slot carries (D11).
  - The modifier is shown as having no effect on a half-turn.
  - "Forward" names a conjugacy class, never a screen direction, so 0.4's T3 and T3′ swap names. Which class is forward is preference (M1, M8).
  - On surfaces drawn in screen space (the on-screen keyboard, the palette, the journal, macro text), a third-turn shows an undirected class mark and a half-turn an undirected glyph. Only the hint on the gripped cell draws a turning sense. It is built in the cell's own space, so it reverses exactly when the picture of the turn does (A7, A1).
- **K2, grip slots.** Twenty slots hold the anchor's affecting caps, hosting cells first (D7, A2, E2, M2, M9). No piece has more than 20 affecting caps, and its hosting cells are always among them.
  - The applied map is workspace state. A rebind proposal beside it is a static lookup in model data, with no job and no synchronous engine call (E2, E7, D11).
  - A rebind proposal shows in place: each changed key's future glyph sits at its new cap under the "pending" pattern while the live glyphs stay solid, and the hint shows one count of changed keys. Cap IDs stay in the binding table and on focus (A8, D11).
  - No camera, filter, twist or focus event changes the map (E2's digest test).
  - A grip holds while its key is down, with latching as an option. Losing focus releases it, and the grip outline drops in the same frame (D2, A2).
  - Any of the 600 caps can also be gripped by a pointer pick, or by its ID in the palette (D2).
  - The 20 slots, the 7 axis keys and 4 seat keys use 31 of the 37 non-digit keys of a US main block, so no grip key shadows O9's number keys (E7).

  The order of the slots, and the frame that each slot carries, are Q3.
- **K3, opening keys.**
  - A key opens at the first learning step whose goal flow script needs it, so no goal meets a closed key (D10). A script over the flow scripts checks this, together with bar M6: no key opens before its concept's term (M10, D10).
  - A step opens in place, never modally and never during input. Its keys carry a "new" status pattern until first used, with a quiet motion unlike any reward motion (A5).
  - "Show all keys" changes no disclosure state. "Open all" is a saved choice that the solver can reverse (A5, D5).

  The name of the scale is Q9.
- **K4, repeat and hold.** Only camera, list-movement and preview-scrub keys repeat (D3, E8). No grip or twist key repeats (D2). Losing focus releases every held key (D2, E5). A key typed in a text field never reaches a command (D3, U4-064).
- **K5, physical keys.** The grip-and-twist block and O9's number keys bind by physical key code, with labels drawn from the active layout (E5, D1, A7, E7). The portable layers store codes only. The lookup from code to label and the IME handling sit in the platform layer (E5). The binding of the other commands is Q2.
- **K6, key maps.**
  - One default map with the solver's edits replaces 0.4's orbit sets (D1).
  - `file.keymap.check` tests duplicates and, in each routing class, that the bound key sequences form a prefix code, so that no group key is also bound alone. A key-map file with bindings by character records its layout, and the check runs once per layout (M10).
  - 0.4 key maps migrate: twist keys by the word each key typed, commands through E1's alias table (M1, D7, E1). Bindings scoped to a 0.4 set have no target once the sets go, and EN4 states how they are reported (E1).
- **K7, pointer and keys.** The round settled only what the pick targets mean:
  - a pointer pick grips any cap (D2);
  - the drag shows its realised element, with no snap or detent that favours alignment (A6; DT-J1);
  - hover only inspects (A3).

  No card took up the modifier conventions for pointer actions or 0.4's gestures. EN4 carries them with H-05 and brings any conflict back to the table.

**Motion and terms**

- **Motion meanings (A3).** Frequent commands answer by channel, never by colour:
  - `grip.select`: a grip outline, and no puzzle motion;
  - a live twist or redo: the H-03 turn;
  - a draft twist: a step glyph in its phase and a motion class of its own;
  - undo: the H-03 undo motion, with its hint showing beforehand what it would take back;
  - a preview: effect glyphs under a "current" pattern that turns "stale" on any edit;
  - `operation.execute`: a net-effect motion;
  - hover: an inspection outline only.

  H-03 lists only turn, undo, invalid action and solved. Under charter bar A1, a draft step, a preview scrub, an operation commit and a key opening or remap therefore do not move until H-03 gives them a row. Whether they get one is Q10.
- **Terms, for DH-09 (M5, M7).**
  - A twist is a turn of one grip by a non-identity element of the active menu. It has two species: retained twists (11) and jumble twists (48). The glossary's "one of the 1,200 generators or its inverse" is false for 8 of a cap's 11 twists (M5).
  - A generator stays one of the 1,200. Every count names its alphabet: keys, twists or generators.
  - A grip stays a cap, and its frame moves into the rebind rule.
  - A key map is a stored map from (key, context) to a command ID with absolute arguments.
  - "Seat" enters as a new term if Q7 picks seats.

### Experiments

None of these is performance evidence. A cost comes from the candidate engine on the owner's machine.

- **XM1** (D8, M7): over the census face-match steps, count the steps where the cut's outside is on the lattice and the steps where only its inside is. In the first case the closing set is one right coset, so one seat holds all 12 candidates. In the second it is a left coset, which splits 3 + 3 + 3 + 3 over the seats. Headless, on the research records, for EN2. If the second case is common, Q7's recommendation is revisited.
- **XM2** (E2, E7): a headless test that camera, filter, twist and focus events leave the digest of the slot map unchanged, and that only the rebind command changes it.
- **XM3** (E8): a headless test that an undo naming an action that another action has superseded is refused as stale, under Q4 (a).
- **XM4** (E4, E9, D9, E11): B-004's admission fixture (screening-findings.md, line 67), run headless, with a commit and an undo during a running job, concurrent key and pointer submissions, and a held key. On the owner's machine, with the candidate engine: the commit latency during a long preview, and the commit latency of a live twist against the gaps between key presses in bursts of typed twists during the living test. If every commit ends within the shortest gap, Q6 (a) refuses no key in practice and Q6 closes.
- **XM5** (E10): every input route reaches all 59 items of every grip, and one journal replayed under two key maps gives one state hash. Headless.
- **XM6** (E2, A2, E5): the key-hint overlay (up to 20 cap regions) and the grip outline that drops in the same frame join the H-06 cost table and the H-09 overlay limits. The IME half of E5 needs a fixture on the owner's Windows machine. No cost is claimed before then.

### Owner batch

Each question offers two or three options. "Recommended" marks the option that the roles converge on, or the 8B order's choice where they do not.

- **Q1. The keyboard model (K1).** (a) The grip-and-twist block bound by position, one fixed map of chords for the frequent commands of the solving loop, and prefix groups whose second keys show as tips, all under the palette (D1, D7, A7). (b) The block and the chords, with the palette alone for every other command (D1). (c) Either of these with 0.4's switched key sets, only if the active set is always shown and is stored in each binding (M10). Recommended: (a). In D1's sketch, locking a finished block and setting the next target needs a six-press set switch in 0.4 before the set's own key, and three presses under (a).
- **Q2. Binding the other commands (K5).** The block and the number keys bind by position in every option. (a) Every other key by position too, labelled from the active layout (E5, M10). (b) The same, except undo and redo, which keep the characters they have in office software, with a conflict check whenever the layout changes (D7). A7 would extend this to the other conventional office chords. (c) Every mnemonic chord and prefix second key by character, with the conflict check run against each layout the solver uses (E5, E7). Not offered: every key by character, which breaks the block on other layouts; on French AZERTY the digits need Shift, and A and Q swap places (E5, A7, D7). Recommended: (b), by the 8B order (D7). M10 prefers (a), on grounds of definability only.
- **Q3. The grip keys after a rebind (K2).** On a cell-local next target a map holds 10.75 caps on average, and under every rule more than half the keys get new caps (A8, D11). The options differ in what a key keeps:
  - (a) Keys relative to the piece. The slots follow M9's transported order (Lemma 7's construction), hosting cells first and ties broken by cap ID, and each slot carries its transported frame. In the 19 orbits where no position has a symmetry (136,800 positions), every grip key and twist key then keeps one role relative to the piece, both at the next target and when the anchor follows the piece through a twist. 9.75 keys change caps (D11).
  - (b) Keys that stick to their caps. Every cap that still affects the new anchor keeps its key; freed keys get new caps, hosting cells first, then by cap ID, in the caps' fixed frames. The fewest keys change caps, 5.56 (A8), but the map depends on the order of past rebinds, and 83.5% of the keys change role (D11).
  - (c) Hosting cells first, then ascending cap ID, recomputed at each rebind, in fixed frames. 9.01 keys change caps (D2, D7, A8).

  Not offered: hosting cells on fixed first keys with the other shared caps sticky (A8's (b)), which depends on past rebinds like (b), changes 6.76 caps and keeps no roles (D11). Recommended: (a), by the 8B order (D11); A8 prefers (b). The counts cover every such pair of positions in the model, not observed solver steps.
- **Q4. Undo (C5).** (a) One key undoes the solver's latest action of either class, and its hint names what it would take back (D4, D9, A10). The session store keeps one order of actions across the journal and the workspace; each undo carries the number of the action that its hint named, and is refused as stale if another action landed first (E8). A text field keeps its own undo. (b) Two keys, one per class (D4, E8). Not offered: a class that follows focus (D4 (c)), because then focus would change a global key's command, which the routing rule of C2 excludes (A10, E8). Recommended: (a).
- **Q5. A commit or undo while a job runs (C5).** (a) It goes ahead at once and cancels the running jobs that read the revision it replaces (previews, checks, the analyser, covers, repair lists), each shown as cancelled; file jobs finish (E9, D9). (b) A commit other than undo is refused as busy until the job ends or is stopped, as in 0.4, while undo always goes ahead and cancels those jobs (E9). Recommended: (a). While a job runs, (b) makes a commit two actions, which S4's "the commit takes one action" weighs against (E9).
- **Q6. A commit typed while another commit is in flight (C5).** (a) It is refused as busy and shown in place, as the redesign decisions U5-011, U6-013/2 and U4-061 say (E11). (b) It waits in one ordered queue behind the commit in flight, and at the first refusal the rest are dropped and counted (D9). This reopens those decisions on the record, and E11 sets its conditions:
  - one queue for every channel;
  - only the six commit rows without `confirm` or `fresh_preview` enter it;
  - arguments are resolved at key-down;
  - every gate is checked again at the head of the queue, and each entry's guard names the commit ahead of it by its durable command ID;
  - repeats are dropped before they are queued.

  Recommended: (a) for now (E11). Option (b) reopens recorded decisions, and XM4's measurement decides whether bursts of typed twists outrun a commit at all; if they do, (b) comes back with E11's conditions.
- **Q7. Typing the 48 jumble twists (C6).** (a) Four seat keys, held like a modifier with the retained twist keys and tapped alone for their own twist, as settled above (D6, D8, A9, M7). (b) Axis then angle over the 24 further axes, through a prefix group, with recorded axis names and a sense rule (M4, M7, A9, E10). (c) No Jumble keys: the drag is the pointer route, and the palette is the keyboard route, naming a twist by its rotation (D8, E6, E10). Not offered: a Jumble-on layer of axis keys, which would be a key mode that C2's routing excludes (A9); and a typed menu index, which has no spatial meaning, ties typing to the order digest and would give the number keys a second meaning beside O9 (D8, A9, E10). M7 would keep the index in the palette, resolved at once to its rotation. Recommended: (a), pending XM1. One face match's 12 candidates are then one seat's 12 chords, while under (b) they lie on 12 different axes (A9, D8).
- **Q8. Messages for refused key presses (C3).** (a) The first refusals of each category show their E6 message at once, in place, without taking focus or keys; later ones show only the glyph, with the message on request (A4, A10, D10). (b) The message only on request. Recommended: (a).
- **Q9. The key scale (K3).** (a) No key numbers: keys open with the learning path's steps, by the rule settled above. Each step is named by its workflow step, with a glyph, and opens with its concepts' term cards (A5, A10, D10, M10). (b) A separate numbered key scale under a word that no other scale uses, with a stated map that meets bar M6 (A5, M10, D10). Not "L" or "K": "L" already names the glossary levels, the reward ladder and the Local view, and "K" names this record's items and the group K⁺ (A5, D10, M10). (c) Keys keep L0 to L6, and the glossary levels and the reward ladder are renamed (A5, M6, D10). Recommended: (a), by the designer, the artist and the mathematician; no engineer's card took it up.
- **Q10. Motion for the new changes (A3).** (a) H-03 gains rows for a draft step, a preview scrub, an operation commit and a key opening or remap. Their meanings are settled above, and G5 tunes their timing. (b) These four changes do not move in 1.0. Recommended: (a) (A3); no card argued for (b).

### After the owner's answers

- The answers are recorded here and in `docs/wiki/decisions/`.
- EN4 builds the final command table with fix-first items 2 to 4, the join of C1 and the rules settled here, and turns XM2 to XM5 into headless tests.
- ENG-0J's line 145 changes with Q7 (fix-first item 1). EN3 names the menu-twist row and the Jumble keys.
- EN2 runs XM1 beside X1 to X5 of DT-J1.
- G6 takes the keyboard model and the key letters; G5 takes the motion rows if Q10 adds them.
- The terms go to DH-09.
- No handoff row (H-01 to H-10) changes unless Q10 adds rows to H-03; XM6 joins H-06 and H-09.

## Owner answers (22 October 2026)

The owner chose every recommended option, for now, and asked that the other options of every question stay on record as alternatives ([decision](../../../../wiki/decisions/owner-decisions-2026-10-22-dt-m1.md)). The owner may revisit any answer; until then the alternatives below are kept, not discarded. The options that the outcome lists as "not offered" stay out: the filter removed them, not the owner.

| Question | Chosen | Alternatives kept |
| --- | --- | --- |
| Q1, the keyboard model (K1) | (a) the grip-and-twist block bound by position, one fixed map of chords for the frequent commands, and prefix groups with tips, all under the palette | (b) the block and the chords, with the palette alone for every other command; (c) either of these with 0.4's switched key sets, the active set always shown and stored in each binding |
| Q2, binding the other commands (K5) | (b) by position, except undo and redo, which keep their office-software characters, with a conflict check whenever the layout changes | (a) every other key by position; (c) every mnemonic chord and prefix second key by character, with the conflict check against each layout |
| Q3, the grip keys after a rebind (K2) | (a) keys relative to the piece: M9's transported slot order, each slot with its transported frame | (b) keys that stick to their caps; (c) hosting cells first, then ascending cap ID, recomputed at each rebind |
| Q4, undo (C5) | (a) one key with a durable action order; an undo that another action has superseded is refused as stale | (b) two keys, one per class |
| Q5, a commit or undo while a job runs (C5) | (a) it goes ahead and cancels the jobs that read the replaced revision; file jobs finish | (b) a commit other than undo is refused as busy, while undo goes ahead |
| Q6, a commit typed while another is in flight (C5) | (a) refused as busy and shown in place | (b) one ordered queue under E11's conditions, which reopens U5-011, U6-013/2 and U4-061 |
| Q7, typing the 48 jumble twists (C6) | (a) four seat keys, pending XM1 | (b) axis then angle through a prefix group; (c) no Jumble keys: the drag and the palette |
| Q8, messages for refused key presses (C3) | (a) the first refusals of each category show their message in place | (b) the message only on request |
| Q9, the key scale (K3) | (a) no key numbers: keys open with the learning path's steps | (b) a separate numbered scale under a word of its own; (c) keys keep L0 to L6, and the glossary levels and the reward ladder are renamed |
| Q10, motion for the new changes (A3) | (a) H-03 gains rows for a draft step, a preview scrub, an operation commit and a key opening or remap | (b) these four changes do not move in 1.0 |

What follows from the answers:

- **Engineering continues.** EN4 carries fix-first items 2 to 4 and the rules settled here into the final command table. ENG-0J's line 145 changes with Q7 (a), and EN3 names the menu-twist row and the seat keys. EN2 runs XM1; if XM1 finds that the closing set is often a left coset, Q7 goes back to the owner.
- **The design side waits.** The round table and the design line are on hold after this topic (owner, 22 October 2026). The parts that belong to them wait until the owner resumes them: G6's key letters, G5's timing for the new H-03 rows, and the DH-09 terms.
