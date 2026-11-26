---
id: owner-decisions-2026-11-26-release-order
type: decision
status: verified
visibility: public
summary: Owner decisions of 26 November 2026 on the order of work to the 1.0 release - 1.0 is released before 2027, preferably by 20 December 2026; ADR-D02 first, then the IR line and the remaining tests, then ADR-D03 and LP6 together, and K2 only when both are done; the owner makes a reduced Taste Lab selection first; ADR-D03 gets a full A1 to A4 and is finished after the reduced solver-agent run; after LP6 the language product freezes and K2 wires it into the program; every K2 line gets a full A1 to A4, Nexus cards enter the product in K2, and K2's art line does the fine work that rounds 3 and 4 held for after stage 2; then the greybox wires the backend and brings it online, a full-power solver-agent run gives constructive findings, refinement continues, and the promotion line is planned with its tools installed before stage 2 freezes; stage 3 runs only the full-volume acceptance and bug fixes; stage 4 makes the complete promotional film, then packages, overhauls the GitHub home page and releases; December's API budget may be used.
related: [owner-decisions-2026-11-22-design-art-path, owner-decisions-2026-11-22-adr-line-evening, owner-decisions-2026-11-23-a7-freeze, owner-decisions-2026-11-26-adr-d02-a4, owner-decisions-2026-11-26-log-flow, owner-decisions-2026-11-26-research-goal, owner-decisions-2026-11-03]
supersedes: []
claims:
  - {id: release-before-2027, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: order-to-release, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: adr-d03-full-a1-a4, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: language-product-freeze-after-lp6, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: k2-content-and-order, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: stage-3-acceptance-only, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: stage-4-film-then-release, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: december-api-budget, evidence_kind: decision, checked_at: 2026-11-26}
---

# Owner decisions, 26 November 2026: the order of work to the 1.0 release

**How the decisions came about.** The owner set the order of the remaining work in four chat messages (in Chinese),
translated:
1. "When D02 is done, finish the IR work and the remaining tests, then start the K2 line, D03 and LP6. The K2 line has
   the most work. When the other two are done, think out the promotion line, then make the program's first version; at
   that point the whole of stage 2 freezes. Stages 3 and 4 are complete acceptance and refinement, then packaging and
   release."
2. "After LP6 the language product freezes, and K2 wires it into the program."
3. The integrator gave an estimated schedule and asked whether to record it. The owner answered: "D03's A1 to A4 are not
   the light kind: it needs the complete materials, and it is finished after the light solver-agent run. Every line of
   K2 needs a full A1 to A4; before that, a reduced Taste Lab selection. Nexus starts to enter the product here. The
   fine work of stage 3 is also done here, in this art line. Only then does the greybox start wiring the backend and
   going online. This stage runs the solver agent at full power and full volume, and it is also constructive. Then
   refinement and adjustment continue, and last comes planning the promotion line and installing the many tools it
   will need. Stage 3 runs only the full-volume acceptance and bug fixes, and touches stage 2's work only for things
   that affect the program. Stage 4 first makes the complete promotional film, then packages, overhauls the GitHub
   home page, and finally releases."
4. The integrator gave a revised estimate, with the release around May 2027. The owner answered: "Nonsense. K2 can wait
   until the other two lines are done; I will finish the Taste Lab selection first, and next month's API budget can
   still be used. The times you give are always much, much longer than the real ones. My goal is to release before
   2027, preferably before 20 December."

## Decisions

1. **The target.** 1.0 is released before 2027, preferably by 20 December 2026.
2. **The order.**
   - ADR-D02 is finished first: its build, its checkpoints, two Astra review rounds and its freeze.
   - Then the IR line and the remaining tests are finished.
   - Then the ADR-D03 and LP6 lines start together.
   - K2 starts only when ADR-D03 and LP6 are both done.
   - The owner makes the reduced Taste Lab selection first, while this work runs.
3. **ADR-D03 gets a full A1 to A4.**
   - Its A1 to A4 carry the complete materials, not the light set of
     [22 November 2026](owner-decisions-2026-11-22-adr-line-evening.md), decision 3.
   - ADR-D03 is finished only after the reduced run of its learning-curve solve test, which runs inside ADR-D03 as that
     page's decision 4 says.
4. **The language product freezes after LP6, and K2 wires it into the program.**
5. **K2's content and order.**
   - Every K2 line gets a full A1 to A4, after the owner's reduced Taste Lab selection.
   - Nexus cards ([2 October 2026](owner-decisions-2026-10-02-design.md)) start to enter the product in K2.
   - K2's art line does the fine work that the [design and art path](owner-decisions-2026-11-22-design-art-path.md)
     placed after stage 2, in its stages S3 and S4.
   - Only then does the greybox stage wire the backend and bring the program online.
   - Stage 2 runs the solver agent at full power and full volume, and that run is constructive: it feeds findings back
     into the work.
   - Refinement and adjustment continue after it.
   - Last, the promotion line is planned and the tools it needs are installed.
   - Stage 2 then freezes with the program's first version.
6. **Stage 3 runs only the full-volume acceptance and bug fixes.** It touches stage 2's work only for things that
   affect the program.
7. **Stage 4 makes the complete promotional film first, then packages, overhauls the GitHub home page and releases.**
   Publishing still needs the owner's authorization.
8. **December's API budget may be used.**
9. **Plans use the measured pace.** The owner found the integrator's estimates far too long. Schedules are planned
   from the pace the lines have measured, not from the agent-day estimates in the plans.

## Superseded in part

- The [3 November 2026](owner-decisions-2026-11-03.md) setting-aside of target days, for the release: decision 1 sets
  one. Gates and bars are unchanged.
- The [log flow decision](owner-decisions-2026-11-26-log-flow.md), decision 2: LP6 starts with ADR-D03. K2 starts only
  when both are done.
- The [design and art path](owner-decisions-2026-11-22-design-art-path.md), decision 2:
  - its rounds 3 and 4 no longer come after stage 2;
  - their fine work moves into stage 2's K2 art line;
  - the promotional film, music and Blender work move into stage 4, before packaging;
  - stage 2's usable-framework acceptance stays a minimum, and its five-day living test is unchanged.
- The [22 November 2026 evening](owner-decisions-2026-11-22-adr-line-evening.md) decision 3, for ADR-D03 only: its A1 to
  A4 are full, not light. ADR-D02's A1 to A4 have run.
- The same page's decision 4, for the full run only: the full run after development becomes two runs:
  - stage 2 has a constructive full-power run;
  - stage 3 has the full-volume acceptance run.

## Readings and open points (the integrator's, not rulings)

- **K2's lines.** They are named at K2's start, on the card that opens their A1 to A4. A likely split:
  - interface structure and flows;
  - art (values, icons, motion and sound);
  - Nexus;
  - the greybox wiring.
- **The freeze rounds.** Rounds 3 and 4 now fall inside stage 2. How the four rounds map onto stage 2's steps is proposed
  at K2's start.
- **"Online".** The greybox brings the product shell up on the real backend and local services. Nothing is published:
  publishing stays an owner authorization.
- **The API ceiling.** December's spend stays within the Console's monthly limit. The cumulative ceiling's next step in
  `AGENTS.md`, USD 300, is not set, and setting it is the owner's (`AGENTS.md`, "Records and budgets"). The design and
  art line's no-API rule of 22 November 2026 stands until the owner lifts it for a named use.
- **Tools.** Installs follow the reviewed allowlist. New entries, such as Blender and Audacity, are pinned, and the owner
  approves them in one batch.
- **What the target cannot move.** Release-build evidence (the renderer gate and Windows/DirectX runs), code signing and
  publishing need the owner's machine or authorization. A test that needs people other than the authors, such as
  CT-EV5-03, cannot be done by agents.

## The briefing pin

The briefing records these decisions. It is an input of the format packet, so its pin is refreshed under FP.1:
- `docs/development-guide/AGENT_BRIEFING.md`, SHA-256
  `46e4e61c033b6a2cb3afb07cd9d133ab7d65767183af3a4d26e0b96c084f857c` to
  `3eef53ce444431ba6095468eafdcf8099c38757d3bd67e91745e514a09a733fc`;
- `input_commit`, `3bdc30c095b12ffd0133534d005e1e1eaf12a012` to `73270a39b9589504268777200fdeb3b729c05769`, the last
  commit that edits the briefing.

The packet cites none of the edited lines. The candidate files, `candidate_sha256` and every other pin are unchanged.

## What this does not do

- It moves no bar and edits no frozen text.
- It accepts no ADR.
