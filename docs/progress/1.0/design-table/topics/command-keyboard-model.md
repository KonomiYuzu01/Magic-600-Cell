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
  - With Jumble on under I_a, each grip has 59 jumble twists besides the identity (`research/jumbling/fixtures/wj-I_a.json`, menu base of 60 items).
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
