---
id: owner-decisions-2026-10-09-jumbling
type: decision
status: verified
visibility: public
summary: Owner decisions of 9 October 2026 - no fudged variant; rigid jumbling enters 1.0 as an independent puzzle and a Jumble feature; simulator, rendering and observation, solving theory and a four-dimensional grip-orbit explorer start now; a renderer packet and an engineering packet follow at mid-stage; 1.0 implements four dimensions only, with interfaces that do not fix the dimension, and the renderer packet carries the rendering acceptance; jumbling is an optional state of a puzzle session.
related: [owner-decisions-2026-10-02-scope, renderer-candidates]
supersedes: []
claims:
  - {id: no-fudged-variant, evidence_kind: decision, checked_at: 2026-10-09}
  - {id: jumbling-in-1-0, evidence_kind: decision, checked_at: 2026-10-09}
  - {id: renderer-packet-at-mid-stage, evidence_kind: decision, checked_at: 2026-10-09}
  - {id: engineering-packet-at-mid-stage, evidence_kind: decision, checked_at: 2026-10-09}
  - {id: dimension-open-interfaces, evidence_kind: decision, checked_at: 2026-10-09}
  - {id: jumbling-optional-state, evidence_kind: decision, checked_at: 2026-10-09}
---

# Owner decisions, 9 October 2026: jumbling

The owner gave these decisions in chat messages in Chinese. This page records them in English. The study they rest on is [research/jumbling](../../../research/jumbling/README.md); its options were set out in the [impact assessment](../../../research/jumbling/impact-assessment.md).

## Decisions

1. **No fudged variant.** The jumbling work covers rigid, unfudged jumbling of the retained cut geometry (α = 121/125) only.
2. **Jumbling enters 1.0.** It comes as an independent puzzle and a Jumble feature. This is option B of the impact assessment; the recommended option A (research track only) was not chosen.
3. **Four workstreams start now:**
   - a general simulator;
   - rendering and the observation experience;
   - a complete solving theory for the jumbling puzzle;
   - a four-dimensional Jambler-style grip-orbit explorer.
4. **Renderer packet at mid-stage.** When the workstreams reach mid-stage, the integrator gives the owner a packet to submit to the renderer route. Only changes such as geometry may need the renderer acceptance to be run again.
5. **Engineering packet at mid-stage.** The jumbling work affects both the renderer line and the engineering line, so the integrator also gives the owner a mid-stage packet for the engineering line: the backend boundaries (engine, session store, command layer, view model).
6. **Four dimensions now, interfaces open to more.** Asked whether "4D+" means a dimension-general framework in 1.0, the owner chose the second option offered: 1.0 implements and accepts the four-dimensional puzzle only, and the data contracts and interfaces do not fix the dimension (a pose is a d × d matrix, every header names d, and the projection chain is a sequence of stages). The rendering acceptance is left to the renderer packet. The integrator builds the rendering framework: the shared data contract and a full-state research viewer.
7. **Jumbling is an optional puzzle state.** The engineering packet treats jumbling as an optional state of a puzzle session (Jumble on or off), not as a separate kind of session. How a session enters and leaves that state under the contract (enter by replaying the retained word, leave only at a witnessed retained-state checkpoint, off by default) is the packet's proposal for the 2.5 freeze, not part of this decision.

## Scope and limits

- **600-cell-Full is unchanged.** Its model identity, its 259,800 labelled slots, its 1,200 legal generators and its mathematical contract stay binding. The jumbling puzzle gets its own model identity. Its exact definition, in particular the menu of jumble twists, is still an owner sign-off at the 2.5 architecture freeze.
- **Charter.** The stage 2 charter made the mathematical contract a binding input that is not reopened in stage 2. Decision 2 reopens it for the jumbling puzzle only.
- **Freeze inputs.** The 2.5 architecture freeze now covers:
  - the pose-based jumbling state;
  - its commands;
  - its renderer features.
- **Human-solve boundary.** The boundary of 2 October 2026 applies unchanged. The program never unjumbles, chooses a solving twist or executes without the solver's action.
- **Theory.** Puzzle theory goes through Astra, as the retained puzzle's theory did (`research/theory/`).
- **Licensing.** HactarCE's Jambler has no licence file, so its code is not copied. The four-dimensional explorer is written in this repository from the published formal definition.

## Where it is applied

- Program plan: [jumbling-plan](../../progress/1.0/jumbling-plan.md).
- Engineering packet: `docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md` (decision 7).
- `docs/development-guide/AGENT_BRIEFING.md` section 2, with the owner's explicit authorization in chat (9 October 2026).
