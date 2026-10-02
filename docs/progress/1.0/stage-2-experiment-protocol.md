# Stage 2 experiment protocol (draft)

Status: **draft, not accepted**, except the owner decisions recorded in section 4.1 and the working-day schedule in section 5. It proposes how stage 2 (2.0 to 2.5) runs experiments so that trial and error stays cheap and every attempt leaves a usable result. It changes no rule, no ADR and no gate. The renderer selection gate stays as recorded in [renderer-candidates](../../wiki/decisions/renderer-candidates.md), and UX sign-off stays with the owner (`AGENTS.md` "Ask the owner").

## 1. Principles

1. **Every experiment answers one question.** Write the question and the result that would change a decision before starting. An experiment that cannot change a decision is not run.
2. **Kill criteria before effort.** Each experiment states its time box and the result that stops it. Hitting the time box is a result, not a failure.
3. **Cheapest artifact that answers the question.** Paper, a static render or a 10-second clip comes before a prototype, and a prototype comes before production code.
4. **Judgement and correctness are separate.** Tests, oracles and reviews decide correctness. Only the owner decides taste. Model agreement never decides either.
5. **Results are recorded as they happen.** Each experiment gets a card in the experiment ledger notebook (`HUMAN_GUIDE.md` section 7), including negative results.

## 2. Experiment card

Every experiment in stage 2 uses this card. Keep it in the experiment ledger.

| Field | Content |
|---|---|
| ID | `E-<stage>-<n>`, for example `E-2.4-03`; UI vision rounds use `UV-<n>` |
| Question | One sentence |
| Decision it feeds | The disposition, ADR or gate that changes with the answer |
| Hypothesis | The expected result and why |
| Method | The artifact, tool, script and fixture |
| Time box | Hours or days |
| Kill criteria | The result that stops the experiment early |
| Evidence class | source/fixture, synthetic geometry, actual Windows/DirectX, performance, or owner judgement |
| Result | What happened, with evidence links |
| Decision | Adopted, rejected or open, and who decided |

## 3. Correctness oracle for rebuilt components

When 1.0 rebuilds a component that has a 0.4 counterpart, a differential oracle checks it. The 0.4 engine as it stands after 0.4.1 step 1 (`core.py` and the reference maps) is the reference.

- Run the same scripted input (generators, macros, fixture sessions) through the 0.4 reference and the new component.
- Compare full labelled-state hashes after every step, not only at the end.
- The first mismatching step is the bug report. A mismatch is never accepted as "close enough".
- The shared inputs are fixture sessions built from synthetic data, using the fixture design of [exporter-spec-draft](../0.4.1/exporter-spec-draft.md) section 5. There is no 0.4.1 release (owner decision, 1 October 2026).

This keeps AI-written code cheap to verify: a change either matches the reference or points at a step.

## 4. Design track

Goal: fuse a layout that makes every kept function easy to use with the chosen art style into one 1.0 GUI, and freeze it with the renderer features it needs before UI engineering starts. Function and look are designed together, not one after the other: the core flows and the command table say what the layout must do, and the Look Lab is where layout and style are combined and judged. The owner is the art director and does the judging, tuning and sketching; agents build the instruments, produce variants and implement. The engineering track (renderer and backend) runs in parallel; the two tracks meet through the handoffs in section 4.6, and the 2.4 experiment dates are the test of whether the combination works on a real renderer.

### 4.1 Starting point (owner decisions, 30 September 2026)

- **0.4 is not a UX baseline.** In practical use 0.4 is an unusable program. 1.0 designs its experience from scratch. From 0.4 it keeps only the mathematical contract, the protection rules, the model identity and the function inventory of stage 2.1. Its screens, layouts and interaction patterns are not carried forward or incrementally polished. 0.4 pain points are used only as anti-goal evidence.
- **Chosen route:** four methods, driven through one instrument, the Look Lab:
  - **M1. Mathematics-first aesthetic.** Visual structure comes from the 600-cell itself: symmetry-orbit colourings, Hopf fibrations, projections and camera paths along symmetry axes.
  - **M2. Guiding metaphor.** One metaphor (for example an observatory instrument, a museum exhibit, a precision watch or Japanese stationery) and a one-page manifesto that every later decision is checked against.
  - **M3. Anti-goals.** A list of what the application must never look or feel like, written before the positive direction.
  - **M4. Preference search.** The owner rates generated variants quickly. A preference model moves the Look Lab parameters toward the owner's taste.
- **Not chosen for now:** incremental evolution of 0.4, design from observed 0.4 usage, a commissioned human designer, and community polls. They can be reopened by the owner. A commissioned designer and any public poll need owner approval (cost and publishing).

### 4.2 The Look Lab (instrument)

A local development tool in Godot 4.7 (.NET), the S-A2 candidate's framework, that shows the real 600-cell geometry (synthetic, from the model; never a personal session) next to live parameter controls and greybox layouts. It is the prototype of the 1.0 shell: if S-A2 is selected it grows into the product shell; if another candidate is selected, the framework-neutral outputs below are rebuilt there. Its use is not evidence for the stage 2.4 renderer selection; its cost readings are estimates until the engineering track measures them.

| Group | Live parameters |
|---|---|
| Structure | projection (stereographic, perspective, orthographic), fibration or orbit highlighting, cell and slice visibility |
| Colour | sticker palette, saturation, background, accent colours, colour-vision-deficiency preview |
| Material | matte, gloss or glass; sticker gaps; edge and outline treatment |
| Light and depth | key and fill light, fog, depth of field, ambient occlusion |
| Motion | turn duration, easing curve (editable), inertia, settle, camera damping and field of view |
| Frame | panel density, typography sample, layout grid overlay |
| Layout | greybox structure (for example one central stage with contextual instruments, or a docked multi-panel workbench), viewport share, panel placement per context, overlay or docked panels |

Functions:
- Save and load **presets** as JSON (the raw material for design tokens). Presets are diffable and versioned in the repository; screenshots and clips stay under `work/`.
- **Capture** a still or a clip of at most 10 seconds from any preset, straight into the workbench gallery.
- **Swipe mode** for M4: show one generated variant at a time; the owner answers better or worse than the current best, about 1 second per decision.
- **Compare mode:** two to four presets side by side, the same camera and the same scripted turn.
- **Greybox mode:** a layout without style (grey panels, system font, real commands from the command table) so a layout is judged on use before it gets a look. A preset applied to a greybox gives the styled version of the same layout.
- **Flow scripts:** each core flow (section 4.3) runs as a script on any layout and reports its steps, pointer travel and time. This is the measurable side of "easy to use"; taste stays with the owner.
- **Cost meter:** every visual feature and preset shows its estimated frame-time cost at full detail, from the engineering track's cost table (handoff H-06), so a look is tuned against the 30 fps gate while it is chosen.

Framework-neutral outputs (they survive any renderer selection): design tokens (JSON presets), the layout specification (grid, regions, contexts, command placement), the motion table, the visual feature list and the core flow storyboards.

Build acceptance: runs on the owner's machine; every parameter changes the view live; a preset round-trips byte for byte; capture, swipe, greybox mode and flow scripts work; no mechanical state is changed or relabelled.

### 4.3 Work plan and goals

**Preparation (days 1 and 2)**
- The owner keeps a taste diary: one sentence per thing seen and liked or disliked, with a neutral source description. Images stay private.
- The owner drafts the anti-goals list (M3).

**2.0 Charter: design charter**
- Anti-goals list finalized, at least 10 items, each with one sentence of reasoning.
- Two or three candidate metaphors, each with a one-paragraph manifesto draft.
- Quality bar kept, changed or dropped by the owner: signature moments and the engineering, mathematics, design and art bars ([charter-2.0-draft](charter-2.0-draft.md) section 5).
- Exit: the owner signs the charter.

**2.1 and 2.2 Inventory and dispositions (design side)**
- **Core flows.** The owner storyboards five to seven core flows, six to ten frames each, on paper or in draw.io: for example open a session and read its state; find a piece, plan a step, preview its full effect and commit; protect solved orbits, save a checkpoint and undo; use and manage macros; the solved moment.
- Agents attach every inventory item to the flows it serves. An item no flow uses is a delete or automate candidate in 2.2.
- Every 0.4 function gets keep, redesign, delete or automate, judged by its purpose, not by its 0.4 presentation.
- The owner annotates 0.4 pain points (private screenshots). Each pain point becomes an anti-goal or a UX criterion.
- The Look Lab is built in parallel (Codex-first). It needs only Godot and the model geometry.

**2.3 Design exploration (main owner phase)**
- **G1. Structure catalogue (M1) and greybox layouts.** Agents implement at least six structure-derived visual modes and two or three greybox layout structures in the Look Lab. The flow scripts run on each layout. The owner picks at least two visual modes and one layout structure. Exit: the chosen modes as presets, the chosen layout as the first layout specification, and the first visual feature list (H-01) and layout specification (H-04) sent to the engineering track.
- **G2. Metaphor decision (M2).** For each candidate metaphor, agents produce one hero still and one 10-second clip in the Look Lab, all using the same scene. The metaphor also sets the character of the layout (for example an observatory instrument suggests a central stage with surrounding gauges). The owner picks one and finalizes the manifesto. Exit: the manifesto is signed.
- **G3. Preference search (M4).** Swipe sessions of about 20 minutes, at most three, over the parameters the metaphor leaves open. Stop when two consecutive sessions converge on the same parameter region, or when the owner calls it. Each candidate shows its cost-meter reading. Exit: one to three finalist presets, sent to the engineering track (H-02).
- **G4. Sticker palette.** Candidate palettes must pass measurable checks: minimum perceptual distance between adjacent cells (OKLab), a colour-vision-deficiency simulation, and legibility at full detail. The owner picks among the palettes that pass. Exit: palette preset frozen.
- **G5. Key moments.** The owner tunes the motion of turn, undo, invalid action and solved in the Look Lab curve editor. Exit: a motion parameter table (H-03).
- **G6. Styled layouts.** The finalist preset is applied to the chosen greybox layout. The three most important core flows are built to high fidelity in the Look Lab, with the keyboard model taken from the command table; the other flows stay at greybox plus tokens. The owner sketches corrections; agents apply them. Exit: owner-approved styled layouts for the three flows, and the final layout specification.

**2.4 Vertical slice (inside the experiment window)**
- One complete core flow is built at full quality on the leading renderer candidate, inside the chosen layout, with the finalist preset, palette and motion. This is where the combination of function, layout, style and renderer is tested together.
- It must pass the renderer selection gate. A look that cannot hold full detail at 30 fps does not pass. In that case, return to G3 with the conflicting parameters and their measured cost.
- **Living test:** the owner uses the slice for about 15 minutes a day for five days and writes one sentence a day. Exit: still satisfied after the five days, with a correction list.

**2.5 Freeze**
- A design-system ADR: tokens (colour, spacing, type scale, motion durations and curves), the palette, components, the layout specification, the core flows, the manifesto and the anti-goals. The command table and the renderer feature list are frozen with the architecture.
- Tokens carry several themes on one design language that users can tune (owner decision, 2 October 2026), and layouts allow other languages and longer strings (subtitles are undecided).
- The owner accepts the ADR. After the freeze, UI engineering implements the system and does not search for the feel again. A later change goes through the Look Lab and an ADR update.

### 4.4 Owner time and tools

| Phase | Owner time (estimate) | Owner does |
|---|---|---|
| Preparation | a few minutes a day | taste diary, anti-goals draft |
| 2.0 | half a day | charter sign-off |
| 2.1–2.2 | two to three hours | core flow storyboards, pain-point annotation |
| 2.3 | about one hour a day on days 4 to 12 | layout and style picks, swipe sessions, tuning, sketches |
| 2.4 | 15 minutes a day for five days | living test |
| 2.5 | one hour | ADR acceptance |

Tools: Godot (Look Lab), Blender (hero stills where Godot is not enough), marimo (preference model and palette checks), draw.io (wireframes), Typst (type specimens). Adding a tool outside the allowlist (for example Krita for sketching) follows the normal allowlist process.

### 4.5 Acceptance for the track

- Signed charter with anti-goals, and a signed manifesto.
- Look Lab build acceptance met.
- Every design decision traces to a preset, a swipe session record or an owner pick.
- Every kept function is reachable in the layout specification through the command table, and each core flow has a flow-script result.
- Every handoff in section 4.6 is recorded as delivered, or as dropped with the owner's reason.
- The vertical slice passes the renderer selection gate and the living test.
- The design-system ADR is proposed and accepted by the owner.

### 4.6 Cross-track record (design side)

The design track and the engineering track run in parallel and meet through numbered handoffs. Each track keeps its own copy of this record with the same IDs: this one, and section 6 of [renderer-experiment-plan](renderer-experiment-plan.md). When a handoff happens, both copies are updated with the date and where the item lives. A late handoff is not waited for: the receiving track uses the last delivered version and records that.

**Design track gives**

| ID | Item | First version | Final | Used by the engineering track for | Status |
|---|---|---|---|---|---|
| H-01 | Visual feature list (effects the look needs: highlighting, sticker gaps, outlines, materials, fog, depth of field, transparency) | day 6 (G1) | day 10 (G3) | workload W5 and the renderer feature list | open |
| H-02 | Finalist presets as JSON tokens | day 10 | day 12 | formal runs on days 10–13, the vertical slice | open |
| H-03 | Motion table (turn, undo, invalid action, solved) | day 11 | day 12 | W3 turn duration and easing in the slice | open |
| H-04 | Layout specification (viewport share, overlay or docked panels, contexts) | day 6 | day 12 | viewport size, UI-over-3D compositing, input routing in S-A2 and S-D | open |
| H-05 | Pointing needs from the core flows (sticker, piece, cell, grip, orbit) | day 4 | day 8 | picking design and the `pick` probe | open |

**Design track needs**

| ID | Item | Needed by | From | Used for | Status |
|---|---|---|---|---|---|
| H-06 | Cost table: measured frame-time cost per visual feature at full detail | day 5 | S-B results | the Look Lab cost meter | open |
| H-07 | Day-7 go/no-go result and the remaining candidates | day 9 | Astra ruling | Look Lab framework risk; where G6 is built | open |
| H-08 | Command table and the layer boundaries | day 4 | 2.1 and 2.2 | greybox layouts, keyboard model, flow scripts | first version 1 October: [command-table](command-table.md); final after the 2.2 dispositions |
| H-09 | Renderer constraints (overlay layers, text in the 3D view, transparency and sorting limits) | day 8 | S-A2 and S-D | which features stay in H-01 | open |
| H-10 | Selected renderer | day 14 | selection | vertical slice and the framework-neutral rebuild if not Godot | open |

## 5. Schedule: about 20 working days (owner decisions, 1 and 2 October 2026)

Preparation and all of stage 2 are planned as about 20 working days. Day numbers count the owner's working days, not calendar days: a day counts when the owner works on the project, and the owner can correct the count ([owner-decisions-2026-10-02](../../wiki/decisions/owner-decisions-2026-10-02.md)). Day 1 was 1 October 2026; 2 October was not a working day. Tracks run in parallel; the owner's time goes to the design track and to the gate decisions. The experiment dates are also the compatibility test of the two tracks: each candidate renderer is tried with the design track's features, layout and presets as they are handed over (section 4.6).

| Days | Engineering track | Design track (owner) | Exit |
|---|---|---|---|
| 1–2 | Merge the 0.4.1 step 1 and B4-12 harness branches. Choose the migration path. Codex starts the Look Lab build. | Taste diary and Taste Lab ratings start. Anti-goals draft, two or three candidate metaphors. | **2.0** charter signed (end of day 2), including the migration path. |
| 2–4 | **2.1** two independent inventories as concurrent Codex shards; **2.2** dispositions; command table and layer boundaries (H-08). | Core flow storyboards; inventory attached to the flows; pain-point annotation; pointing needs (H-05). | Dispositions signed (day 4). |
| 3–14 | **2.4** renderer window, 12 days: S-B bare Direct3D 12 probe on days 3–4 with the cost table (H-06), then S-A2 and S-D with the design's features, layout and presets as they arrive (H-01 to H-04). Go/no-go on window day 7 (day 9), Astra ruling. Final selection on day 14. | Uses H-06, H-07, H-09 and H-10 as they arrive. | Renderer selected, or the failure recorded. |
| 4–12 | Look Lab ready by day 4 with greybox mode, flow scripts and the cost meter; agents produce layout and style variants. | **2.3** G1 (days 4–6), G2 (days 6–8), G3 and G4 (days 8–10), G5 and G6 (days 10–12). | Manifesto, layout specification, finalist preset, palette, motion table and styled layouts for three core flows approved. |
| 13–15 | **2.4** vertical slice: one complete core flow in the chosen layout and style on the leading candidate; gate measurement. | Review of the slice. | Slice passes the selection gate. |
| 15–19 | Correction fixes only. | Living test, five days. | Living test exit. |
| 19–20 | **2.5** architecture freeze with the command table, layer interfaces and renderer feature list (Astra gate ruling), migration format freeze, design-system ADR. | ADR acceptance. | Stage 2 closed. |

Rules for the schedule:
- Exit days are targets. When a task is likely to miss its target day, the integrator says so once with a short re-plan (move the target, cut the task down or drop it), and the owner chooses. Nothing is cut or extended automatically.
- Time never relaxes a gate: the selection gate, the Astra rulings, the oracle, the review rules and the owner sign-offs are unchanged.
- Agent work that needs no owner input may run on any day; it does not advance the day count. Work that needs the owner, the owner's computer or a sign-off waits for a working day.
- If no renderer candidate passes the gate by its target day (day 14), the vertical slice uses the best candidate for the design checks only, the failure is recorded, and the owner decides the next step.
- No 0.4 performance work and no 0.4.1 release work runs in stage 2.

## 6. Out of scope

- Choosing the renderer. The design track supplies features, layout and presets to the 2.4 experiments and uses the selected candidate; it does not select it.
- Mechanics, model identity and protection rules. They are unchanged by any UI decision.
- Publishing any reference image, personal session content or screenshot of the owner's machine.
