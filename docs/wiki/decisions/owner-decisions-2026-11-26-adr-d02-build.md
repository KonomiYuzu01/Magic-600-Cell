---
id: owner-decisions-2026-11-26-adr-d02-build
type: decision
status: verified
visibility: public
summary: Owner answers of 26 November 2026 on ADR-D02's build framework - the build, the owner's view and the acceptance are in the product shell, with Look Lab dropped and Jumble drawing built in this build; J0 and J1 gain one lean researcher subagent on the subscription; the core draws two views, Global and Local, in this build; the drawing is built and measured as candidates before R5 admits it; the watching window never sits on top and never takes focus; the build waits for the owner's go before B1; and, later that day, ADR-D01's whole frame enters the product shell on sample data in this build (part of ADR-D03's scope), after which B5 and B6 run without a stop for the owner's check.
related: [owner-decisions-2026-11-26-adr-d02-a4, owner-decisions-2026-11-25, owner-decisions-2026-11-22-adr-line-evening, owner-decisions-2026-11-22-design-art-path]
supersedes: []
claims:
  - {id: build-in-product-shell, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: lean-researcher-subagent, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: build-waits-for-go, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: two-drawn-views, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: measure-before-r5, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: watching-window-never-on-top, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: d01-frame-on-sample-data, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: b5-b6-without-stop, evidence_kind: decision, checked_at: 2026-11-26}
---

# Owner decisions on ADR-D02's build framework

**How the decisions came about.**
- On 26 November 2026, on the owner's go, Fable wrote the build framework for ADR-D02's shell as a subscription
  subagent ([25 November 2026](owner-decisions-2026-11-25.md), item 5: Fable frames, Opus builds). Its first revision
  built in Look Lab's chrome library with a live Look Lab window and left Jumble drawing waiting for Look Lab's L1.
- The ADR-D02 session put three questions to the owner on one card, with the recommended option first. The owner
  answered in the session's chat on the same day.
- Fable then wrote revision 2 of [the build framework](../../progress/1.0/adr-d02-build-framework.md) to the answers.
  It raised two more questions, which went to the owner on a second card the same day with the recommended option
  first (sections 4 and 5). The owner answered on that card.

**What this page does not do.**
- It accepts no ADR, edits no frozen text and moves no gate. ADR-D02 stays proposed; its framework items and slot
  names freeze in round 2 with K2.
- It admits no renderer feature. The drawing is built as candidates behind a switch and measured on gate rows; R5, a
  revision of the frozen render chapters, admits them only with the owner's signature.

## 1. Build, watch and accept in the product shell

The owner's words, translated: "This time build directly in the product framework and leave Look Lab alone; show me
the building process and the acceptance in the product framework too."

The integrator's reading:
- The product shell (`renderer/shell/`, with the host library and the core as needed) is the build surface, the
  owner's view and the place of every acceptance check. Look Lab is no part of acceptance.
- The chrome library stays where it is, staged into the shell, so there is one copy.
- Jumble drawing is built in this build, in the product core. Look Lab's L1 probe is retired; the drawing's cost is
  measured on gate rows on the owner's machine, with the full chrome.

## 2. One lean researcher subagent for J0 and J1

The owner's words, translated: "Add a subagent, but keep its work lean."

The integrator's reading: the scripted driver stays the base of the reduced Track J run. One subscription subagent on a
smaller model adds a reach test and a defect-finding pass, with no API and no source access, under a fixed cap
(framework section 7).

## 3. The build waits for the owner's go

The owner's words, translated: "Wait for now."

The integrator's reading: Fable revises the framework; no build step starts until the owner says so. The owner said
"wait" again on the second card, after revision 2.

## 4. Two drawn views in this build

The question: should the core draw one view at a time (a key switches between Global and Local), or Global and Local
at once? The recommendation was one view, leaving the pair to ADR-D03 and K2. The owner chose two views.

The integrator's reading:
- The core draws Global and Local at once in this build.
- The cut slice's held split and the raw view's side-by-side antipode chart are built as ADR-D02.17 and .18 describe
  them, with no single-view stand-ins.
- Fable's framework gains the steps for the second view (B18a to B18c) and three gate rows in the two existing
  sittings: about 27 more agent hours, about 210 in all, by Fable's estimate. These are the first measured runs of more
  than one full-detail view; a failing row lowers the pane ceiling and never moves the gate.

## 5. Build and measure the drawing before R5

The question: may the drawing candidates be built and measured behind the `--d02` switch before R5 admits them? The
owner took the recommended option: yes. After the gate sittings, R5 goes to the owner once, with the measured numbers.
R5 is a revision of the frozen render chapters; the top integrator writes it and the owner signs it. Nothing ships
before then.

## 6. The watching window never sits on top and never takes focus

Said in the design-system ADR line the same day and relayed to the ADR-D02 session. The owner's words, translated: "I
have told the top integrator when B1 starts. One more requirement: the live-build script must not put the window on
top. I may be doing other things, and I will look at the build now and then."

The integrator's reading:
- Every window the build starts for the owner to watch is a normal window. It is never topmost and never takes the
  foreground, neither at launch nor on a reload, rebuild or restart. The owner switches to it to look.
- Reloading in place is preferred to restarting, so no fresh window appears at each step.
- Gate measurement runs that need the foreground by design keep their own foreground rules. They run in quiet windows
  arranged with the owner.
- The framework's section 1, B1, B4 and C1 carry the rule.

## 7. ADR-D01's frame in the product on sample data, then B5 and B6 without a stop

Asked in the design-system ADR line on the afternoon of 26 November 2026, after B4 landed, and relayed to the ADR-D02
session. The owner's question, translated: "Why is D01's Look Lab frame not connected now? And the build just now was
not shown either?" The integrator listed the work on one message: fix the dock bug, mount ADR-D01's whole frame in the
product on sample data, then B5 and B6. The owner's answer, translated: "Do all of it, then go on to B5 and B6; no need
to stop and wait for me to check."

The integrator's reading:
- A new step, B4a, mounts ADR-D01's whole frame in the product shell on sample data behind a launch switch
  (`launch.py --sample`), with a visible "sample" chip: the relationship strip and the orbit rail, the draft line with
  its Goal and Net/Strict chip, the Library and its signatures, the inspector's tiers, Problems, Console and Tasks, the
  editor, the Scripts dock and the script panel slots, the key map drawn as a keyboard, the pick and focus layer keys,
  and the readouts where data exists. Sample facts never pass for the real session's: the session's History dock, its
  readouts and its undo and redo stay its own.
- This pulls part of ADR-D03's scope, the frame in the product on sample data, into ADR-D02's pull request. One copy:
  the shell stages Look Lab's sample wiring and shares it, never duplicates it. Global anchors need the core's
  projection and are left out; Local stays with B18a to B18c, and the look and palette with B14.
- B5 and B6 follow at once. C1, the checkpoint after B6, runs the integrator's own checks and records them; it does not
  stop for the owner. The integrator reports what landed when B6 is done.
- The watching window stays open from the session's process through this work, and every new build goes into it, under
  section 6's rule.
