# ADR-D01 revision 5 draft: documents and the log file

Status: **Accepted** with revision 3a (owner, 23 November 2026, at A7; [freeze record](../../../../wiki/decisions/owner-decisions-2026-11-23-a7-freeze.md), [README](README.md)). It was not part of A5: Fable did not build it.

## The owner's words (22 November 2026, about 22:30 UTC, translated from Chinese)

> The log file compiler belongs in LP4. Here is the ADR's form, said ahead of time: a log file records the whole program's state, including everything at exit and the embedded scripts (a script can of course also be saved and compiled on its own). The program should be able to create, compile and save documents at any time, perhaps as tabs (or in a more artistically sophisticated form; you decide).

Reading:
- **Who builds it.** LP4 builds the log compiler and the document services: the log format, compile, check, import and export. LP4's editor services are built in stage 2.
- **What ADR-D01 fixes.** ADR-D01 fixes only the form of these documents in the frame.
- **The log file.** It records the whole program's state, including everything at exit and the embedded scripts.
- **Scripts.** A script can also be saved and compiled on its own, as `.6cs`.
- **Documents.** The program can create, compile and save documents at any time.
- **The form.** Tabs, or a more artistic form; the owner left the choice to adr-line. The choice is signature tabs (ADR-D01.83). The spine shelf is kept on record.

## Items

- **ADR-D01.79 Documents.** There are two kinds of document: a log, and a script (`.6cs`).
  - LP4 names the log's extension.
  - Documents open as tabs in the centre pane group, beside the main pair. Any number can be open.
  - The live session is the first tab, and it is always there. It is the log that is being written. At exit it is saved without a prompt.
  - These actions are command rows with default keys: New (log or script), Open, Save, Save as, Compile, Close.

- **ADR-D01.80 What a log holds.**
  - The model identity.
  - The start state.
  - Both histories (moves and layout), with abandoned branches and partial undos (ADR-D01.15 and .16).
  - The state at save or exit, with its full-state hash.
  - The view, the layout, the workspace and the open documents.
  - The solver's macros, Library entries and scripts that the log uses, embedded with their provenance marks (author class).
  - The preferences, recorded for reference. Import never applies them: it keeps the current ones, as the `file.log.import` row says.

- **ADR-D01.81 Compile.**
  - **A script.** Compiling parses it, checks it (exactness rules, capability separation) and builds the IR.
  - **A log.** Compiling checks:
    - the format and the model identity (gate `model_match`);
    - every move, replayed with its legal witness;
    - the final full-state hash;
    - its embedded scripts, compiled as above.
  - **Where results appear.** Problems go to the Problems tab. The result shows on the document's tab.
  - **What a failed log does.** It opens read-only, with the reason.
  - **What compile never does.** It never changes the live state, and it never runs a script.

- **ADR-D01.82 Opening and importing a log.**
  - **Opening.** A log opens as a free view bound to its saved state, with the binding chip "saved". Its history scrubs as a preview (ADR-D01.10).
  - **Making it current.** Making a log the current session is the explicit import commit (`file.log.import`). That row has:
    - a file check gate and a confirm gate;
    - a recovery checkpoint;
    - an undo journal;
    - current preferences kept.
  - **Embedded scripts.** They never run when a log opens. A script by another author class needs the install-time consent of ADR-D01 (the DET.37 overrule) before it can run.
  - **Still binding.** A recording never commits, and no program commits.

- **ADR-D01.83 Signature tabs (the form).**
  - **On every document tab:** its name, a dirty dot when unsaved, a pin, and a signature.
  - **A log's signature.** 35 ticks in the rail's order. Each tick's length is that orbit's solved share ("length carries quantity"). A row of log tabs then reads as a shelf of progress at a glance.
    - Hover shows the move count, the time and the model identity.
  - **A script's signature.** Its compile lamp, in the lamp grammar of the top-bar readouts:
    - an empty ring when not compiled;
    - solid when compiled clean;
    - solid with an outer ring when it has problems.
  - **The session tab.** The live signature and a recording mark.
  - **Overflow.** Tabs that do not fit go to a searchable list, also reachable from the palette.
  - **Comparing two logs.** Drag a tab into a split to compare two logs side by side.
  - **On record, not chosen:**
    - *Spine shelf:* documents as book spines along the left edge, each with its signature drawn vertically. A candidate for round 4, signature moments and polish.
    - *Plain filename tabs:* rejected. In the same space they hide what the signature shows.

- **ADR-D01.84 Ownership and rows.**
  - **LP4** builds:
    - the 1.0 log format and its compiler;
    - check, import and export: C600, and MPUlt as interchange for 600-cell-Full only, with the cap LP4 sets;
    - the document services.
  - **ADR-D03** builds the tabs on sample data.
  - **K2's greybox stage** wires the rows to the backend.
  - **Rows to add at the command-table freeze:** `file.document.new`, `.open`, `.save`, `.save_as`, `.compile`, `.close`. The `file.log.export`, `file.log.check` and `file.log.import` rows (disposition "redesign") follow these items.
  - **Look-cost.** A signature is 35 short marks per tab.

## Slot names for A7

`document.tab`, `document.tab.signature`, `document.tab.lamp`, `document.tab.dirty`, `document.tab.pin`, `document.overflow`.
