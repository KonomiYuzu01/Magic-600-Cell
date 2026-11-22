---
id: owner-decisions-2026-11-22-design-art-path
type: decision
status: verified
visibility: public
summary: Owner decision of 22 November 2026 on the design, art and promotion path - stage 2's goal is a usable version of the whole program framework; UI/UX freezes in four rounds (round 1 at A7, round 2 ends stage 2, rounds 3 and 4 after it); the Jumbling ADR (ADR-D02) is drafted after A6 and K2 opens after A7 and ADR-D02, beside LP4 and LP5, which brings DT-J2 to DT-J4 and the design line's interface work forward; the look-cost envelope enters the core as one PHS.52 revision; LP4's editor services are built in stage 2; a Fable subagent runs a reduced M5 review; three gate sittings; sound, motion and icons are not cut, so the two-cue sound cap is lifted; the line uses no API until the owner says so.
related: [owner-decisions-2026-11-22-adr-d01-overrules, owner-decisions-2026-11-22-c10-lp2-exit, owner-decisions-2026-11-06, owner-decisions-2026-10-02-scope, owner-decisions-2026-10-22-dt-m1, renderer-freeze]
supersedes: []
claims:
  - {id: stage-2-usable-framework, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: four-freeze-rounds, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: k2-after-a7-and-adr-d02, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: decisions-c-to-f, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: sound-cap-lifted, evidence_kind: decision, checked_at: 2026-11-22}
  - {id: no-api-on-design-art-line, evidence_kind: decision, checked_at: 2026-11-22}
---

# Owner decision, 22 November 2026: the design and art path in four freeze rounds

On 22 November 2026 the design-system ADR line proposed a detailed path for the design, art and promotion line. Before
the proposal, a round table at the owner's request ran two rounds with four roles: the front-end designer, the
interaction artist, the engineer advisor and the mathematician. All four were Claude subagents within the subscription,
with no API use, and none of them read the owner's ratings, picks or chat answers.

The owner approved the proposal in the ADR line's session. The owner chose four freeze rounds, opened K2 after A7 and
the Jumbling ADR, and agreed to every other item as recommended. This page records the decisions in English. It accepts
no ADR, freezes nothing, edits no frozen text and moves no gate or bar. Day counts in the proposal are estimates in the
owner's working days; no target day is set ([3 November 2026](owner-decisions-2026-11-03.md)).

## Decisions

1. **Stage 2's goal is a usable version of the whole program framework.** Fine work comes after stage 2, and none of it
   is cut: two theme families with three scene looks each, the full icon and glyph set, the motion and sound tables and
   the signature moments.
2. **UI/UX freezes in four rounds, one per stage.**

   | Stage | Freeze round | When | Content |
   | --- | --- | --- | --- |
   | S1 Frame | Round 1, at A7 | In stage 2 | A5 to A7 of the design-system ADR (ADR-D01). A7 freezes its framework items and slot names, not values, and only ids that the glossary defines. Also: glossary rows, the look-cost envelope (decision 5), gate batch 1 and H-05 |
   | S2 Usable framework | Round 2, which ends stage 2 | In stage 2 | K2: ADR-D02, H-04's arrangement structure, the flows' steps and key contexts, the main views' content and status forms, the entry and learning-path skeleton, text budgets and H-01. A greybox build of five flows in the product shell. Direction work beside it, off the critical path. Then a reduced M5 review, gate batch 2 and a five-day living test |
   | S3 Values | Round 3 | After stage 2 | Two families of three scene looks (G3, G4), the full icon and glyph set, H-03 and the full sound table (G5), both families' formed interface and LP4's editor look, the cinematic look; sittings C1 and C2 |
   | S4 Signature moments and release polish | Round 4 | After stage 2 | S1 to S5 signed in use against benchmark works, rewards, learning content and competence checks, text review, gate batch 3 on the release build, the freeze package |

   The promotional film, music, Blender work and publishing come after round 4. Publishing needs the owner's
   authorization.

   Stage 2 counts as usable when these hold in the product shell (`renderer/shell`), with fresh data:
   - the five flows run end to end, each control invoking a command-table row:
     - one cap twist with preview and commit;
     - building one block orbit-first under protection;
     - writing and running a `.6cs` macro;
     - the Jumble repair flow;
     - the research output panel;
   - the linked Global and Local views, with the preview view, the commit bar, the History dock, a command palette that
     reaches every command, and a keyboard-only path;
   - the views' D2 states, recovery and the practice copy;
   - the skeleton of the learning path and key disclosure;
   - the renderer gate still met with the full chrome;
   - legibility: one G4-checked sticker palette per family, state roles with shape cues, and labelled placeholder icons.

   A flow that runs only in Look Lab does not count. The art line fills values only. A needed change to a framework
   item goes to the owner as a proposal on the stage's direction card.
3. **The Jumbling ADR (ADR-D02) covers the Jumble and research surfaces.**
   - It is drafted after A6, mostly beside A7.
   - Its main inputs are item J of the A6 review (can ADR-D01's framework host those surfaces) and DT-J1's outcome.
   - It gets one read by a fresh Claude subagent under the Codex minimum.
   - Its framework items and slot names freeze in round 2 with K2, so it adds no freeze round.
4. **K2 opens after A7 and ADR-D02, beside LP4 and LP5.**
   - DT-J2 to DT-J4 resume while ADR-D02 is written.
   - The front-end designer and the interaction artist return for K2.
   - Direction work starts once K2's structure is nearly frozen.

   This brings forward the hold of [6 November 2026](owner-decisions-2026-11-06.md), decision 7, under which the design
   line waited until LP2 to LP5 were done apart from LP4's editor look.
5. **The look-cost envelope enters the core as one PHS.52 revision right after A7.** The envelope is H-06's candidate
   features:
   - gaps, outline, and per-sticker transparency with sorting;
   - fog, depth of field and ambient occlusion;
   - MSAA 4x and translucent chrome;
   - the three full-detail states (the preview view and the A/B wipe).

   The same revision adds the stickers' screen positions to the core ABI.
   - Gate batch 1 measures each feature alone, then each family's most expensive combination with the full chrome.
   - A feature that breaks the gate alone goes back to the design line with its measured cost (E-2.4-05).
   - Later tuning stays inside the envelope; a feature outside it needs a new measurement and the owner's acceptance.
   - The renderer gate is unchanged ([renderer freeze](renderer-freeze.md)).
6. **LP4's editor services are built in stage 2.** Diagnostics, completion and run belong to LP4's plan, so the
   `.6cs` flow stays in round 2.
7. **A Fable subagent runs a reduced M5 mathematical review within the subscription, before the living test.** This is a
   proactive Fable task that the owner approved (`AGENTS.md`, Roles).
8. **Three gate sittings on the owner's machine:** batch 1 in S1, batch 2 in S2 and batch 3 in S4. Each sitting has
   about 0.5 hours attended; the unattended part may take hours. If S1's per-monitor DPI test fails, a Godot source build
   follows, which is an owner item.
9. **Sound, motion and icons are not cut.** The sound reduction of [2 October 2026](owner-decisions-2026-10-02-scope.md)
   is lifted:
   - one G5 comparison, then the full sound table in round 3, with no cap on the number of cues;
   - each cue has a visual equivalent and mute;
   - no cue, and no motion, differs with whether a check or repair result is good or bad;
   - adopting sound at all still follows the G5 audition and the owner's decision.

   This supersedes the scope page's two-cue limit and the charter's A4 and its sound line
   (`docs/progress/1.0/charter-2.0-draft.md`, lines 62 and 120). The reduction of the mathematical-language work
   stays.
10. **No API on the design and art line until the owner says so.** Its paid spend is zero. The integrator and Claude
    subagents do the work within the subscription, with local tools. The owner's answers on the Taste Lab batches stay
    local and never go to any API. A5, the ADR line's Look Lab build, keeps the API budget the owner already approved for
    it.
11. **Tools.** FFmpeg 9.1 (winget `Gyan.FFmpeg`, user scope, x64) is pinned in `tools/toolchain.lock.json`; the owner
    ran the pin and approved that revision on 22 November 2026. Blender and Audacity have no user-scope winget
    installer for x64, so they are pinned later as hashed archive entries: Audacity before round 3 and Blender after
    round 4. They are batched with the next approval-input change.

## What follows

- A5 starts on the owner's own start. Its condition, the frozen LP2 and LP3, is met by the
  [exits of 22 November 2026](owner-decisions-2026-11-22-c10-lp2-exit.md).
- After A5, the A6 review covers ADR-D01 with item J. Then ADR-D02 is drafted, and A7 follows on one card. The owner's
  ADR-D02 picks go on the same card where they can.
- ADR-D01's accepted text enters the repository at A7. ADR-D01's overrules of frozen clauses are recorded separately
  ([owner-decisions-2026-11-22-adr-d01-overrules](owner-decisions-2026-11-22-adr-d01-overrules.md)).
- The human-solve boundary is unchanged. No look, motion, sound or focus suggests where to twist next.
- The briefing records these decisions. It is an input of the format packet, so its pin is refreshed under FP.1:
  - `docs/development-guide/AGENT_BRIEFING.md`, SHA-256
    `455a465758b7842a42d07a9d963faf0082b87211ccc0152ef71d26faabceb11a` to
    `2d0b042513f9657491ad980142e53daa6a08bf88fbb1cf84d7d051cdb6a271bf`;
  - `input_commit`, `f58e1242f1ad2654c7450e8d89b67f8f38901b2c` to `57ea15b6364502f4640da722274b4988ed0cd2d0`, the
    commit that edits the briefing.

  The packet cites none of the edited lines. The candidate files, `candidate_sha256` and every other pin are
  unchanged.
