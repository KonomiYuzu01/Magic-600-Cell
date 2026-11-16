---
id: owner-decisions-2026-11-16-time-saving
type: decision
status: verified
visibility: public
summary: Owner decision of 16 November 2026 - five measures save the owner's time in the language's development - LQ13 is decided directly from a one-page comparison of the three LX-8 candidates without a human test, so CT-RTE-18 stays not run; LP4's non-author tests run at minimal scale; LP3's owner-machine runs are done together in one or two evenings; at most one question card per milestone; one page of summary per phase exit.
related: [owner-decisions-2026-11-06, owner-decisions-2026-11-14-language-route, owner-decisions-2026-11-15-freeze, owner-decisions-2026-11-16-lq13]
supersedes: []
claims:
  - {id: time-saving-measures, evidence_kind: decision, checked_at: 2026-11-16}
---

# Owner decision, 16 November 2026: five ways to save the owner's time

The owner gave this decision in chat in Chinese on 16 November 2026. This page records it in English.

## Decision

The integrator estimated the rest of the language's development and separated the agents' working days from the
owner's own hours. The owner answered that they do not have that much time. The integrator proposed five ways to save
it, and the owner chose them all ("go with the time-saving measures").

1. **LQ13: the owner decides the syntax directly.** The integrator writes a one-page side-by-side comparison of the
   three LX-8 candidates. The owner reads it, in about 30 to 60 minutes, and decides; nobody else is tested. The stated
   cost: CT-RTE-18 is recorded as not run, never as passed, and there is no human evidence on which syntax is easier to
   learn. This changes the owner's LQ13 procedure of 6 November 2026 ("decide after LX-8, before LP2 builds the
   parser") and is recorded as a new owner decision.
2. **LP4's non-author tests run at minimal scale,** for example 2 to 3 people at about 30 minutes each. The integrator
   drafts the protocol and the success criteria; the owner edits and approves them.
3. **The owner-machine runs are merged.** LP3's charter E2 run, LX-2's and LX-4's decisive runs and the renderer gate
   already have to hold in the same runs, so they are done together in one or two evenings. The owner only connects
   mains power, closes other programs, starts the run and clicks when a window must be in the foreground.
4. **Questions are batched:** at most one question card per milestone, each with a recommended option.
5. **Explanations are compressed:** one page of summary in Chinese per phase exit, with no walkthroughs in several
   batches like those of November.

Not chosen: moving the user shader language (LQ7), or LP5's research library and research-archive checkers, to after
1.0; and more quota, which would be a new cost. The big-integer question for M2 stays open.

## What each point changes

**Point 1 changes the owner's own procedure, which the owner was told; it lowers no bar.**

- CT-RTE-18, LP1's one human requirement, stays visible as not run. An unrun check is open, never a pass (PHS.2), and
  source-only material never stands in for a human requirement (CT-PHS-06).
- LG1's semantic, recovery and edition obligations are unchanged (RTE.63), and the choice still comes before LP2's
  parser.
- The choice is the owner's, so it is not a syntax decision by agreement of authors or models (RTE.62, RTE.63). If the
  owner later wants the human test, RTE.63's protocol and CT-RTE-18 still govern it.
- Within its scope it supersedes the frozen text that restates the 6 November procedure: chapter 04's LQ13 row and
  EV5.12's LX-8 sentence; chapter 11's LX-8 card under RTE.62 and its LQ13 row; chapter 12's LP1 decisions row
  (PHS.13), PHS.14's LX-8 sentence and PHS.56's LP1 owner action; and ADR-L12's waiting LQ13 question. This page edits
  no frozen text; the wording follows in a later versioned change.
- The LX-8 protocol question in the owner's inbox is superseded.
- The owner answered LQ13 the same day from that comparison: [candidate A, braces](owner-decisions-2026-11-16-lq13.md).

**Points 2 to 5 change how the work is run; no bar moves.**

- Point 2: no frozen clause sets a participant count, a session length or a pass rate. EV5.12 and PHS.27 leave the
  form and the success criteria to the owner, and PHS.6 asks for people and the owner's form. Still in force: the
  mathematician checks the form's mathematical content (EV5.12); correctness, transfer, help and time are recorded
  separately (PHS.27); no pass rate is invented (EV5.12); and an unset protocol keeps its acceptance item open
  (PHS.29). The point covers CT-EV5-03, CT-EV5-05 and CT-LB6-04.
- Point 3: the bars stay unchanged (RTE.21 and the LP3 acceptance table). Chapter 11 already requires LX-2 and LX-4 to
  run with the renderer gate and E2 in the same run. The runs use one frozen LP3 candidate (PHS.4), a quiet machine
  and the three cold startups of CT-RTE-11.
- Point 4 changes when questions reach the owner, not which decisions need the owner: `AGENTS.md` "Ask the owner" and
  PHS.57 are unchanged.
- Point 5: PHS.5, PHS.56 and PHS.57 stay. The mathematician and the engineer advisor explain each phase's concepts,
  the page covers the phase's PHS.56 row, and the owner's understanding confirmation stays a recorded response. The
  page reaches the owner in chat or in the workbench, never in the repository.

## Integrator readings (the owner may change them)

- "The renderer gate" in point 3 is LP3's run of the unchanged gate for the advertised combination on the selected
  renderer (RPI.79). The renderer selection run of stage 2.4 comes earlier, because LP3's on-screen half depends on the
  renderer freeze (PHS.24).
- The comparison for point 1 reaches the owner in chat. Any recommendation in it is labelled as the integrator's
  judgement.

## Where it is recorded

- This page. Points 4 and 5 are not in `CLAUDE.md`.
- `AGENTS.md` and the briefing are unchanged; the migration format packet pins both. The briefing's two LQ13 phrases
  change together with the owner's syntax choice, in one FP.1 pin refresh.
- No progress item is ticked for LX-8 or CT-RTE-18.
