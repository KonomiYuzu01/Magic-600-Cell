---
id: owner-decisions-2026-11-26-adr-d02-build
type: decision
status: verified
visibility: public
summary: Owner answers of 26 November 2026 on ADR-D02's build framework - the build, the owner's view and the acceptance are in the product shell, with Look Lab dropped and Jumble drawing built in this build; J0 and J1 gain one lean researcher subagent on the subscription; the core draws two views, Global and Local, in this build; the drawing is built and measured as candidates before R5 admits it; the watching window never sits on top and never takes focus; the build waits for the owner's go before B1; and, later that day, ADR-D01's whole frame enters the product shell on sample data in this build (part of ADR-D03's scope), after which B5 and B6 run without a stop for the owner's check; the product window turns, opens on the owner's view with smaller stickers, and Local shows only the turning cells; checkpoints do not stop the build; the drawing runs as a second track; and the finished build gets one Fable refinement round, then two Astra rounds, then the owner's freeze; the product window zooms with the wheel; and the 0.4 Piece Filter enters the default view as a Focus layer, with "piece filter" as a palette alias.
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
  - {id: rotation-opening-view-stickers, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: local-turning-cells-only, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: no-checkpoint-stops, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: fast-drawing-track-astra-freeze, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: fable-refinement-before-astra, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: wheel-zoom, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: focus-layer-piece-filter, evidence_kind: decision, checked_at: 2026-11-26}
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

## 8. Rotation, the opening view, smaller stickers and Local's shape

Said in the design-system ADR line on 26 November 2026 while the owner watched the product window, and relayed to the
ADR-D02 session. The owner's words, translated: "Also, rotating the puzzle does nothing, the default view is not the one
I set, and the stickers must be smaller." Then: "The Local view must also show only the turning cells, as I set it."
Then, on bringing Local forward: "Bring it forward if you can; finish it before B5."

The integrator's reading:
- The product window turns with the mouse as Look Lab's does, from Look Lab's kept camera (`q` and `spin`), with
  ADR-D01.40's linked rotation once Local is drawn. A drag never goes to the window's focus hand-back.
- The product opens on the owner's opening view of 22 November 2026 (`tools/looklab/data/opening.json`): the Hopf-ring
  layout, field of view 0.85, cell gap 0.3 and sticker shrink 0.55. Gap and shrink are tunable values, never constants.
- Local takes `tools/looklab/data/local-view.json` as it stands: orthographic, only the turning cells with no
  neighbours, pulled apart so the inner ones show, centred at a fixed size, and turning with the linked rotation.
- B5 had already landed when the owner asked, so "before B5" no longer applies. Local comes as early as the drawing
  track allows (section 10).
- The Hopf-ring layout and Local's shape are renderer features. Under section 5 they are built as candidates behind
  `--d02` and measured, and the watching window opens on them there. They become the product's default only through
  R5, after gate sitting 2, with the owner's signature.

## 9. No stops at the checkpoints

The owner's words, translated: "Do not stop at the checkpoints either; do everything in one go."

The integrator's reading:
- C1 to C6 still get Fable's read, and blocking and major findings are fixed before the next block. Nothing waits for
  the owner.
- Each checkpoint's "owner is shown" line becomes one line in the session's report, saying what to look at in the
  watching window.
- Only what truly needs the owner is batched: the two gate sittings, B19 and B24, which need an attended start and a
  quiet machine, are prepared and handed to the top integrator for an owner-machine sitting it arranges; the
  certificate glyphs' acceptance goes on one card with whatever else is open at the end. The build continues past B19
  and B24 wherever a step does not need their numbers.

## 10. As fast as possible, with the drawing as a second track

The owner's words, translated: "Do it as soon as possible. D02 and D03 will each get two rounds of Astra review then,
and once they pass they freeze." Asked whether the drawing could start now: "Can the drawing be done now?"

The integrator's reading:
- The build runs straight through. Independent steps that touch different files may run in parallel subagents, each
  in its own worktree; the ADR-D02 session stays the one integrator and reviews each diff before it merges it. Two
  writers never touch the same file at once, and the build stays one pull request.
- The drawing runs as a second track beside the solve side: the rotation, a per-view shape input (gap and shrink)
  brought forward from B14, the rest of B14, B18a to B18c with Local's shape, the Hopf-ring preset (B23) before the
  ring sheet (B22), then B15 to B17 as their solve steps land, B18, B20 and B21.
- The two Astra rounds are Codex calls the owner asked for, which the Codex minimum allows. They run through the review
  wrapper with the model, effort and speed passed explicitly, on a seven-part packet prepared at the end of the build.
  The integrator's reading of "two rounds" is one full review of the finished candidate and one scoped re-check of the
  fixes; the owner may correct it. The model is chosen when the reviews run.

## 11. One Fable refinement round before the Astra rounds

Asked in the design-system ADR line whether, as with ADR-D01, Fable should scan the finished work for concrete
refinements and one more round be built before the reviews. The integrator recommended one capped round. The owner's
answer, translated: "In this order."

The order, once B1 to B33 are built:
1. One Fable refinement scan on the subscription, from the owner's point of view: the product window as it is
   (watching-window frames and the scripted runs' screen dumps), against ADR-D02's 56 items, ADR-D01 revision 6 and
   the owner's own requirements (open research freedom as high as the framework allows, the opening view of section 8,
   Local's shape, the window rule and the human-solve boundary). It gives one ranked list; each point carries its item,
   its evidence, the change and its cost.
2. One card to the owner, recommended option first, with an "all as recommended" choice.
3. One refinement build round from the card.
4. Astra round 1, a full review of the finished candidate.
5. Astra round 2, a scoped re-check of the fixes.
6. The owner's freeze, on a decision page.

One scan and one build round, with no iteration. Points outside ADR-D02's scope go to a follow-up list, and costly
points that are not severe are deferred to after the freeze.

## 12. Wheel zoom

Said while the owner watched the product window, relayed by the design-system ADR line. The owner's words, translated:
"That is fine, carry on. I cannot zoom in now; change this."

The integrator's reading:
- Look Lab's own viewport has no wheel zoom: it takes only a left drag, and its zoom is the preset's. So the product
  adds one in Look Lab's terms. The wheel scales the zoom of the view under the pointer. Wheel up zooms in, by 1.1 per
  notch, from 0.5 to 4 times the view's opening zoom.
- The wheel takes no keyboard focus and never reaches the window's focus hand-back (section 6).
- Local's own shape zooms the same way once it is drawn.

## 13. The Piece Filter in the default view, as a Focus layer

The owner's words, translated: "When can the Local view go in? Also, the Piece Filter should appear somewhere in
the default view; add it too." On the name, asked whether to keep the 0.4 word against ADR-D01's frozen vocabulary:
"Just give Focus layer an alias."

The integrator's reading:
- This pulls base-puzzle scope into ADR-D02's build on the owner's word. The refinement round's scope cap (section 11)
  does not apply to it.
- ADR-D01 revision 4 (.46, G21) makes the 0.4 Boolean filters focus layers, and its vocabulary turns "filter" into
  "Focus layer". The word on screen stays "Focus layer", so nothing frozen is overruled. "piece filter" (in any case)
  and "filter" are palette aliases of the eight rows (`filter.open`, `filter.preview`, `filter.apply`, `filter.edit`,
  `filter.preset`, `filter.save`, `filter.load`, `structure.filter.compose`), whose palette labels read "Focus layer".
- Placement: a "Focus layer =" entry on Global's border, in the default Solve view, after the focus-layer keys. It
  opens the editor in the right dock.
- The editor follows dispositions U6-022, U5-048, U5-050, U5-051 and U2-009:
  - the whole Boolean expression is validated before anything applies, and rejected text stays as typed;
  - replace, intersect, union and subtract;
  - live query, fixed pieces and watched positions are kept apart and never composed silently;
  - the exact count shows before apply, and a preview from another revision or text is refused at apply;
  - presets, and save and load (save keeps exactly the applied layer; load stages for review);
  - every control is reachable by keyboard.
- The applied layer is a focus layer with its count, on the cycling key. A dimmed non-member is a reference and is
  never picked; a hidden one is neither drawn nor picked.
- The filter never relabels a piece, rebinds a key, or narrows the analysis, the protection or the collateral of the
  259,800-label model.
- Sample mode: counts are exact on the bound sample state's pose rows (domain, height, lattice). The engine's filter
  (`c600-command`'s `filter.preview` and `filter.apply`) is not reached through the host yet; the editor says so.
