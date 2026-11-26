---
id: owner-decisions-2026-11-26-adr-d02-build
type: decision
status: verified
visibility: public
summary: Owner answers of 26 November 2026 on ADR-D02's build framework - the build, the owner's view and the acceptance are in the product shell, with Look Lab dropped and Jumble drawing built in this build; J0 and J1 gain one lean researcher subagent on the subscription; and the build waits for the owner's go before B1.
related: [owner-decisions-2026-11-26-adr-d02-a4, owner-decisions-2026-11-25, owner-decisions-2026-11-22-adr-line-evening, owner-decisions-2026-11-22-design-art-path]
supersedes: []
claims:
  - {id: build-in-product-shell, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: lean-researcher-subagent, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: build-waits-for-go, evidence_kind: decision, checked_at: 2026-11-26}
---

# Owner decisions on ADR-D02's build framework

**How the decisions came about.**
- On 26 November 2026, on the owner's go, Fable wrote the build framework for ADR-D02's shell as a subscription
  subagent ([25 November 2026](owner-decisions-2026-11-25.md), item 5: Fable frames, Opus builds). Its first revision
  built in Look Lab's chrome library with a live Look Lab window and left Jumble drawing waiting for Look Lab's L1.
- The ADR-D02 session put three questions to the owner on one card, with the recommended option first. The owner
  answered in the session's chat on the same day.
- Fable then wrote revision 2 of [the build framework](../../progress/1.0/adr-d02-build-framework.md) to the answers.

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

The integrator's reading: Fable revises the framework; no build step starts until the owner says so.
