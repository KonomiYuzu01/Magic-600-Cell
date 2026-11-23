---
id: owner-decisions-2026-11-22-adr-line-evening
type: decision
status: verified
visibility: public
summary: Owner decisions of the evening of 22 November 2026 on the design-system ADR line - ADR-D01 revision 4 (the solving surface) on every recommended option, A5's stage 10 scope, ADR-D02 and a new ADR-D03 as shells with K2's greybox stage unchanged, ADR-D03 accepted by a learning-curve solve test with no human sessions, A5's API budget past the USD 150 ceiling, one pull request for A5, the 1.0 log file to LP4 with signature tabs as its form, and A6 with computer use.
related: [owner-decisions-2026-11-22-adr-d01-overrules, owner-decisions-2026-11-22-design-art-path, owner-decisions-2026-11-22-c10-lp2-exit, owner-decisions-2026-11-22-lx-bars-freeze]
supersedes: []
claims:
  - {id: rev4-recommended-options, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: a5-stage-10-scope, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: adr-d02-d03-shells, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: adr-d03-solve-test, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: a5-budget-past-150, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: one-pr-for-a5, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: log-file-to-lp4, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: a6-computer-use, evidence_kind: decision, checked_at: 2026-11-22}
---

# Owner decisions, 22 November 2026 (evening): the design-system ADR line

The owner gave these answers in the ADR line's session on the evening of 22 November 2026. This page records them in
English.

What this page does not do:
- It accepts no ADR and freezes nothing.
- It edits no frozen text and moves no gate or bar.
- It does not publish ADR-D01. ADR-D01 stays a private draft until A7, where the owner freezes it item by item
  (UI/UX freeze round 1: [design and art path](owner-decisions-2026-11-22-design-art-path.md)).

## Decisions

1. **ADR-D01 revision 4, the solving surface: every recommended option.**
   - Where it comes from: the owner's atlas of every 0.4 function left 38 functions without a surface in revision 3.
     One round of the design table (designer, artist, engineer advisor, mathematician) then wrote revision 4.
   - The owner took all seven recommended options:
     - **Six cuts are signed.** The frame gives no surface to:
       - a suggestion of the following orbit;
       - candidate lists with reasons;
       - likely-use scoring;
       - the extra word set ("Word A");
       - a completion acknowledgement;
       - continuation planning.
     - **Provenance mark.** It is shown by author class.
     - **Rail order.** The orbit rail follows the seed stage order, labelled as one admissible order.
     - **Encoding grammar.** It gains the rule "length carries quantity".
     - **Words.** Target, Wanted, Draft, Direct, Core, Template, Layouts, Learn and Library.
     - **Glance test.** The bar is 90%.
     - **Look-cost envelope.** Three additions come after A7.
   - Pages these cuts overrule: two of the cuts (the following-orbit suggestion and candidate lists with reasons)
     overrule text in the public [solving workflow](../../progress/1.0/solving-workflow.md) and the glossary. Until
     those pages are edited, this page is the record.

2. **A5's stage 10.**
   - What A5 is: Fable (`claude-fable-5-2`) building ADR-D01 in Look Lab on the owner's API.
   - Stage 10 built the points the owner chose, plus three extra ones (the palette's row menu, the inspector tiers'
     value ladder, and the preview view's before and after).
   - Point 12 moves to ADR-D03.
   - Point 13, the tuning panel (A/B slots, saved presets, advanced overrides), is built at the end of ADR-D02.

3. **ADR-D02 and a new ADR-D03 build shells.**
   - **ADR-D02** builds the Jumble and research surfaces in the product shell.
   - **ADR-D03** builds the complete canvas and UI/UX in the product shell, on sample data, without the backend or
     servers.
   - **K2's greybox stage is unchanged.** The owner corrected a first reading that ADR-D03 replaced it. The greybox
     stage still wires the five flows to the backend end to end. The stage 2 exit is unchanged.
   - **A light A1 to A4 for each of ADR-D02 and ADR-D03.** The owner wants one: a gap list, reuse of the tool
     analyses, one design-table round, and one detailed page where the owner picks. Nothing of it starts until the
     owner says so.

4. **ADR-D03 is accepted by a learning-curve solve test.**
   - **What it is.** Agents play solvers at different points of the learning curve. They really solve, and they
     score convenience and experience.
   - **When it runs.**
     - A reduced run inside ADR-D03, before its decision page is written.
     - A full run after development.
     - Real restoration needs K2's greybox wiring, so K2 wires the solve flow first.
   - **Re-runs and cost.** Small parts must be easy to re-run. Runs must spend few tokens without weakening the result.
   - **No human sessions.** All results are synthetic, and the test's validity rests on planted UX defects.
   - **What it does not decide.**
     - Agreement between agents is never a pass criterion.
     - UX sign-off stays the owner's.
     - The owner's five-day living test at round 2 is separate.

5. **A5's API budget goes past the USD 150 ceiling.**
   - The owner added USD 50 to A5's budget, taking it to USD 130. With A1 to A3's spend, the cumulative paid API
     amount then passes the USD 150 ceiling of `AGENTS.md` "Records and budgets".
   - The owner approved this and set the Console's monthly limit to USD 200 as the hard stop.
   - The next ceiling step in `AGENTS.md`, USD 300, is not set.
   - Exact amounts are in the private ledger.

6. **One pull request for A5.** It carries A5's work and this record, and it is merged once its checks pass.

7. **The 1.0 log file goes to LP4.**
   - **What exists today.** 0.4 exports, checks and imports C600 and MPUlt logs. 1.0 has only the command-table rows
     `file.log.export`, `file.log.check` and `file.log.import` (disposition "redesign").
   - **What LP4 builds.**
     - The 1.0 log format and its compiler.
     - Check, import and export: C600, and MPUlt as interchange for 600-cell-Full only.
     - The document services: new, open, save, save as, compile and close.
   - **What a log records.** The whole program's state, including everything at exit and the embedded scripts.
   - **Scripts.** A script can also be saved and compiled on its own, as `.6cs`.
   - **Documents.** The program can create, compile and save documents at any time.
   - **The form in ADR-D01.** The owner left the choice to the ADR line, which chose signature tabs:
     - Each document is a tab beside the main pair.
     - A log's tab carries a 35-tick orbit signature.
     - A script's tab carries its compile lamp.
     - Opening a log shows a free view bound to its saved state.
     - Making a log current is the explicit `file.log.import` commit.
     - A spine shelf is kept on record as an alternative.
   - The details freeze with ADR-D01 at A7.

8. **A6 runs with computer use.** The A6 review is written in a session of its own. The owner requires that the
   reviewer, Codex Astra, can use computer use extensively for a comprehensive review. This is the line's one Codex
   call under the Codex minimum.

## What follows

- After this pull request, a session of its own writes A6's packet and the computer-use route. It asks the owner once,
  with the cost, before the call runs.
- Then come A7 and its single owner card, ADR-D02, and K2 beside LP4 and LP5.
