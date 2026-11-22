---
id: owner-decisions-2026-11-22-adr-d01-overrules
type: decision
status: verified
visibility: public
summary: Owner decision of 22 November 2026 on the design-system ADR (ADR-D01, revision 3, content approved) - LP3 freezes as planned without waiting for it; where ADR-D01 conflicts with a frozen clause, ADR-D01 wins without a versioned-change review; its view and workspace command-table rows are ordinary edits after the freeze; its commit rows (layout history, multi-step rollback, partial undo, one-key preview-and-commit, the settle interval) are separate work after the freeze, each with KN5 boundary attack tests, and any change to the frozen migration format goes to the owner.
related: [owner-decisions-2026-11-22-lx-bars-freeze, owner-decisions-2026-11-15-freeze, owner-decisions-2026-11-17-c5-evidence, owner-decisions-2026-10-02-design]
supersedes: []
claims:
  - {id: lp3-does-not-wait-for-adr-d01, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: adr-d01-overrules-frozen-clauses, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: adr-d01-view-workspace-rows-ordinary, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: adr-d01-commit-rows-after-freeze, evidence_kind: decision, checked_at: 2026-11-22}
---

# Owner decision, 22 November 2026: ADR-D01 and the frozen clauses

The design-system ADR line reached revision 3 of ADR-D01 on 22 November 2026. The owner approved its content and its
dependency list in the ADR line's session. The owner also said that A5 to A7 follow the owner's picks and that, for this
project, where those picks conflict with frozen rules, the conflict is set aside without an approval step. The owner
then gave the integrator, in chat, the decision recorded here. ADR-D01 itself stays a local draft until A7:
- A5 is the Look Lab build;
- A6 is one Astra review;
- A7 is the owner's item-by-item freeze.

This page records the decision in English. It accepts no ADR, freezes nothing and edits no frozen text.

## Decisions

1. **LP3 does not wait for ADR-D01.** LP3 freezes as planned. Its exit (C10) is judged on `013900ff`
   ([owner-decisions-2026-11-22-lx-bars-freeze](owner-decisions-2026-11-22-lx-bars-freeze.md), item 6).
2. **ADR-D01 wins over the frozen clauses it conflicts with, without a versioned-change review.** PHS.52 does not
   apply to these changes. Revision 3 names these clauses. Each is replaced for the 1.0 product as stated, and every
   other part of the frozen architecture holds.

   | Clause | Frozen text, short | Replaced by |
   | --- | --- | --- |
   | LG3.6, LG3.7 | A preview is prepared only inside a command the solver invokes; views have no preview effect | A view may drive a live preview (caret, live parameters, `=` bindings on preview parameters); ADR-D01.9, .23, .43 |
   | LG3.1, LG3.5 | Programs cannot invoke view-kind rows; workspace changes happen only as requested changes inside an invoked command | An invoked entry point may run view and workspace rows; views still never write the workspace, and live bindings are derived values; ADR-D01.2, .23 |
   | KN5.12 [O] | Whether a program may replace a shown preview is open | Answered: replacement is allowed, with a new ID, a visible mark and a settle interval; ADR-D01.9 |
   | KN5.14 | The commit token comes from a key-down on the preview shown on screen | Also the one-key preview-and-commit on the solver's own macros and twists; ADR-D01.12 |
   | KN5.19 | One undo key and one durable order across journal and workspace | Two histories with their own keys, moves and layout; ADR-D01.14 |
   | KN5.20 | `session.undo` and `session.redo` are the only entries, one step each | Multi-step rollback by one action, and partial undo inside an entry as its own recorded action; ADR-D01.15 |
   | ONE.74, KN4.6 | A branch comes only from a commit after undo | Also a mid-history insert, with the old line's later moves offered as a preview; ADR-D01.16 |
   | EV4.16, BND.84, BND.15 | No solver-tool solving preview before the joint setting; program-produced words refused at preview in a solving session | The solver's own programs prepare solving previews; packages from others stay refused there; ADR-D01.8 |
   | DET.37 | Capabilities are granted per profile and per run | Consent at install, revocable; ADR-D01.32 |
   | ADR-L11.5 | Display primitives are a stated inventory | The inventory gains the canvas and any building block of ADR-D01.29 it lacks; ADR-D01.31 |

   These stay binding:
   - no program commits (LG3.4, DET.35);
   - every commit rechecks the preview revision, the full-state hash and the protected orbits;
   - recordings never commit and stop where EV4.18 says;
   - the human-solve boundary of 2 October 2026 for built-in tools, with the repair-list exception of 17 October 2026.
3. **View and workspace rows for ADR-D01 are ordinary edits after the freeze.** They are not PHS.52 changes. They
   land on main after the exit candidate `013900ff` and are not part of it. The rows are data only, with no new gate
   and no commit row. Each gives its kind, lane, preview, undo, gates and job, with the source "ADR-D01 (owner,
   22 Nov 2026)". The command table's checks pass. The names below are proposals and follow the table's naming. A row
   that is unclear is left out.
   - Workspace kind:
     - themes: select, tune (seed knobs), token override set and reset;
     - presets: density, type scale, motion, reduced motion;
     - layout: apply a stored arrangement, panel mount and unmount, radial menu edit;
     - palette pin and alias;
     - explore on and off, which may only pause commits.
   - View kind: preview view open and pin, A/B wipe compare, view back and forward, history dock mode.
4. **The commit rows come after the freeze, as separate work.** Each needs its own design and tests:
   - layout-history undo and redo, a second history beside `session.undo` and `session.redo`;
   - multi-step rollback, one action over several steps;
   - partial undo inside a journal entry, appended as its own action;
   - one-key preview-and-commit on the solver's own macros and twists, which rechecks the gates as
     `operation.execute` does;
   - the settle interval after a preview replacement.

   Before it lands, each gets KN5 boundary attack tests in the style of the attack of 17 November 2026. If a new
   journal action kind touches the frozen migration format, that part goes to the owner.

## What follows

- The integrator marks each clause in item 2 in its chapter with a note that cites this page. Where a pinned chapter
  changes, the format packet's pin refresh (FP.1) follows. This comes after LP3's chapter 11 amendment lands, so that
  one pull request at a time changes the pins. Done on 22 November 2026, after pull request 409:
  - Thirteen clauses carry a note:
    - chapter 02: LG3.1, LG3.5, LG3.6 and LG3.7;
    - chapter 01: KN4.6, KN5.12, KN5.14, KN5.19 and KN5.20;
    - chapter 04: EV4.16;
    - chapter 08: ONE.74;
    - chapter 09: DET.37;
    - ADR-L11.5.
  - BND.15 and BND.84 carry none. Chapter 07 is a file of the frozen format candidate `en4-data-core-draft-1`, and a
    byte change there would change the frozen format's identity, which needs the owner. This page is their record.
  - Chapter 01 is an input of the format packet, so its pin is refreshed under FP.1:
    - `docs/architecture/1.0/language/architecture/01-kernel.md`, SHA-256
      `fdd5348f4d251a756a1890343a4a80adf1cd89522e802face6e0d42af8919a68` to
      `28d221846f2e7223167abc9e198e73239f38a48459c669b5df8dc3f1711b859a`;
    - `input_commit`, `b7cb2bd625ec93fddf3ee1acc9376d407e59d777` to `f58e1242f1ad2654c7450e8d89b67f8f38901b2c`, the
      commit that adds the notes.

    The packet cites none of the noted clauses. The candidate files, `candidate_sha256` and every other pin are
    unchanged.
- A session adds item 3's rows after `013900ff`. Item 4's work starts after the owner signs C10.
- ADR-D01's accepted text enters the repository at A7.

## What this does not do

- It does not accept or freeze ADR-D01. The design-system ADR stays open until A7.
- It does not change the frozen migration format. Any such change goes to the owner (item 4).
- It moves no gate or bar.
- It changes neither exit candidate.
