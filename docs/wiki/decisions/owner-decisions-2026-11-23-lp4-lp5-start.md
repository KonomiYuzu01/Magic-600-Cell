---
id: owner-decisions-2026-11-23-lp4-lp5-start
type: decision
status: verified
visibility: public
summary: Owner decisions of 23 November 2026 that start LP4 and LP5, every recommended option on one milestone card - per-step rollback transactions stay (no format change); "j of k" becomes durable through the step ID; the command layer's other readings are recorded as overlays; LP4's seven items (LX-4's 22 November bars as the editor bars, the non-author form later, replay inspects recorded effects, explicit activation, first use is the manual, phase I as its own pull request, two overrules recorded); LP5's four items; the stutter re-run and reading (a)'s owner-machine halves in one sitting.
related: [owner-decisions-2026-11-22-c10-lp2-exit, owner-decisions-2026-11-22-lx-bars-freeze, owner-decisions-2026-11-22-adr-line-evening, owner-decisions-2026-11-22-design-art-path, owner-decisions-2026-11-23-a7-freeze]
supersedes: []
claims:
  - {id: lp4-lp5-start, evidence_kind: decision, checked_at: 2026-11-23}
  - {id: command-layer-readings, evidence_kind: decision, checked_at: 2026-11-23}
  - {id: lp4-items, evidence_kind: decision, checked_at: 2026-11-23}
  - {id: lp5-items, evidence_kind: decision, checked_at: 2026-11-23}
  - {id: one-owner-sitting, evidence_kind: decision, checked_at: 2026-11-23}
---

# Owner decisions, 23 November 2026: LP4 and LP5 start

The integrator put one milestone card to the owner in the top integrator's session on 23 November 2026, with a
recommended option for each item. The owner answered "every recommended option, start the work". This page records
the answers in English.

What this page does not do:
- It edits no frozen text. Item 2.3 names a PHS.52 revision; LP4 writes it as a reviewed change of chapters 05 and
  09, and this page only authorizes it.
- It moves no gate or bar.
- It accepts no ADR.

## Decisions

1. **LP4 and LP5 start now.**
   - LP3's exit and LP2's exit were signed on 22 November 2026
     ([C10 and LP2 exit](owner-decisions-2026-11-22-c10-lp2-exit.md)). A7 froze ADR-D01 on 23 November 2026
     ([A7 freeze](owner-decisions-2026-11-23-a7-freeze.md)). So LP4's functional work and LP5 may start under
     chapter 12 (PHS.41, PHS.32). LP4's editor look still waits for freeze round 3
     ([design and art path](owner-decisions-2026-11-22-design-art-path.md)).
   - The plans drafted on 22 November 2026 are the starting point: `docs/progress/1.0/language-lp4-plan.md` (about
     21 agent days) and `docs/progress/1.0/language-lp5-plan.md` (about 29.5 agent days). Each lands with its line's
     first pull request. The estimates bind nothing (PHS.1).
   - LP4's plan adds the 1.0 log file, which the owner moved to LP4 on 22 November 2026 with signature tabs as its
     form ([evening decisions](owner-decisions-2026-11-22-adr-line-evening.md); ADR-D01 revision 5's documents).
2. **LP4's seven items.**
   1. The editor bars of PHS.27 are LX-4's bars of 22 November 2026
      ([LX bars](owner-decisions-2026-11-22-lx-bars-freeze.md)). They run on LP4's frozen candidate in the next
      owner-machine batch.
   2. The non-author test form is set when the pieces it tests exist, from a one-page draft, at minimal scale
      ([16 November 2026](owner-decisions-2026-11-16-time-saving.md)).
   3. Replay (EV2.9) inspects recorded effects and never re-executes. This is a PHS.52 revision of chapters 05 and 09.
   4. Activation (EV1.13) is explicit in the stage 2 greybox.
   5. First use (EV5.14) is the manual.
   6. Pull requests: phase I, the `.6cs` authoring flow, lands as its own pull request before the greybox. The rest of
      LP4 is the line's one pull request. This is an exception to one pull request per line.
   7. Two overrules are recorded:
      - X1: ADR-D01.14 and ADR-D01.15 extend to EV4.15 and its tests.
      - X2: item 4 of the [design and art path](owner-decisions-2026-11-22-design-art-path.md) overrules chapter 12's
        hold on the design line (CT-PHS-03).

      Both follow the rule of [22 November 2026](owner-decisions-2026-11-22-adr-d01-overrules.md): where ADR-D01 or
      an owner pick conflicts with a frozen clause, the owner's pick wins.
3. **LP5's four items.**
   1. After the LP3 freeze, new research reads and overlay rows enter as ordinary additions (CM.19, CM.20).
   2. LP5 runs no Lean.
   3. If stage 18's start tables are not stored, they are rebuilt by exact replay as a new derivation with its own
      checker.
   4. LP5 writes the typed claim bindings as versioned data.

   The plan's readings A (KN6.1 as a family of sub-checkers) and B (brute-force tests as record checks on synthetic
   fixtures) are the integrator's readings. Its contradiction candidates C1 and C2 route through item 3.1.
4. **The command layer's open points** (from ADR-D01's commit rows, landed 22 November 2026):
   1. **Rollback stays per step.** A crash leaves a complete prefix of committed rollback steps, and recovery shows it
      as it is. All-or-nothing across a crash would need a format change (FP.31, FP.36). The owner chose to keep the
      per-step transactions.
   2. **"j of k" becomes durable (R2-5).** The step ID takes the form `{gesture}#{i}/{k}`, so a reopen can tell a cut
      gesture from a whole one. This needs no format change. It changes the step-ID form in the row's design notes.
   3. **Readings recorded as overlays.** No frozen text is edited.
      - A step whose fate is unknown makes the press answer `unknown_outcome`, even after earlier steps landed (R2-6).
      - A partly landed press answers with the last landed step's receipt (R2-10).
      - The overlays: chapter 13's `preview_shown` row, KN5.12, the draft codes `commits_paused` and
        `preview_settling`, and RM.26 (a frame comes before input).
5. **One owner sitting.** The stutter fix's device re-run and reading (a)'s owner-machine halves
   ([C10 and LP2 exit](owner-decisions-2026-11-22-c10-lp2-exit.md)) run together at the owner's next machine sitting,
   about half an hour attended.

## What follows

- LP4 and LP5 continue in the sessions that drafted their plans, each with up to three or four parallel sessions.
  Each line verifies its work once and queues it for the integrator, who merges on green.
- R2-5's step-ID change lands with LP4's line pull request.
- The integrator asks for the owner sitting (item 5) once, on the next milestone card. LP4-1's editor bars run at a
  later sitting, after LP4's candidate freezes.
