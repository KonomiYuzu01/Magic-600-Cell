# Stage 2 experiment protocol (draft)

Status: **draft, not accepted**, except the owner decisions recorded in section 4.1 and the 20-day limit in section 5. It proposes how stage 2 (2.0 to 2.5) runs experiments so that trial and error stays cheap and every attempt leaves a usable result. It changes no rule, no ADR and no gate. The renderer selection gate stays as recorded in [renderer-candidates](../../wiki/decisions/renderer-candidates.md), and UX sign-off stays with the owner (`AGENTS.md` "Ask the owner").

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

Goal: turn the owner's idea of an artistic, high-quality, satisfying application into a frozen design system before UI engineering starts. The owner is the art director and does the judging, tuning and sketching; agents build the instruments, produce variants and implement.

### 4.1 Starting point (owner decisions, 30 September 2026)

- **0.4 is not a UX baseline.** In practical use 0.4 is an unusable program. 1.0 designs its experience from scratch. From 0.4 it keeps only the mathematical contract, the protection rules, the model identity and the function inventory of stage 2.1. Its screens, layouts and interaction patterns are not carried forward or incrementally polished. 0.4 pain points are used only as anti-goal evidence.
- **Chosen route:** four methods, driven through one instrument, the Look Lab:
  - **M1. Mathematics-first aesthetic.** Visual structure comes from the 600-cell itself: symmetry-orbit colourings, Hopf fibrations, projections and camera paths along symmetry axes.
  - **M2. Guiding metaphor.** One metaphor (for example an observatory instrument, a museum exhibit, a precision watch or Japanese stationery) and a one-page manifesto that every later decision is checked against.
  - **M3. Anti-goals.** A list of what the application must never look or feel like, written before the positive direction.
  - **M4. Preference search.** The owner rates generated variants quickly. A preference model moves the Look Lab parameters toward the owner's taste.
- **Not chosen for now:** incremental evolution of 0.4, design from observed 0.4 usage, a commissioned human designer, and community polls. They can be reopened by the owner. A commissioned designer and any public poll need owner approval (cost and publishing).

### 4.2 The Look Lab (instrument)

A local Godot development tool that shows the real 600-cell geometry (synthetic, from the model; never a personal session) next to live parameter controls. It is a development tool, not the product. Its use is not evidence for the stage 2.4 renderer selection.

| Group | Live parameters |
|---|---|
| Structure | projection (stereographic, perspective, orthographic), fibration or orbit highlighting, cell and slice visibility |
| Colour | sticker palette, saturation, background, accent colours, colour-vision-deficiency preview |
| Material | matte, gloss or glass; sticker gaps; edge and outline treatment |
| Light and depth | key and fill light, fog, depth of field, ambient occlusion |
| Motion | turn duration, easing curve (editable), inertia, settle, camera damping and field of view |
| Frame | panel density, typography sample, layout grid overlay |

Functions:
- Save and load **presets** as JSON (the raw material for design tokens). Presets are diffable and versioned in the repository; screenshots and clips stay under `work/`.
- **Capture** a still or a clip of at most 10 seconds from any preset, straight into the workbench gallery.
- **Swipe mode** for M4: show one generated variant at a time; the owner answers better or worse than the current best, about 1 second per decision.
- **Compare mode:** two to four presets side by side, the same camera and the same scripted turn.

Build acceptance: runs on the owner's machine; every parameter changes the view live; a preset round-trips byte for byte; capture and swipe mode work; no mechanical state is changed or relabelled.

### 4.3 Work plan and goals

**Preparation (days 1 and 2)**
- The owner keeps a taste diary: one sentence per thing seen and liked or disliked, with a neutral source description. Images stay private.
- The owner drafts the anti-goals list (M3).

**2.0 Charter: design charter**
- Anti-goals list finalized, at least 10 items, each with one sentence of reasoning.
- Two or three candidate metaphors, each with a one-paragraph manifesto draft.
- UX success criteria in plain words, for example "a first-time user can make a turn within 30 seconds" or "the solved moment feels earned".
- Exit: the owner signs the charter.

**2.1 and 2.2 Inventory and dispositions (design side)**
- Every 0.4 function gets keep, redesign or delete, judged by its purpose, not by its 0.4 presentation.
- The owner annotates 0.4 pain points (private screenshots). Each pain point becomes an anti-goal or a UX criterion.
- The Look Lab is built in parallel (Codex-first). It needs only Godot and the model geometry.

**2.3 Design exploration (main owner phase)**
- **G1. Structure catalogue (M1).** Agents implement at least six structure-derived visual modes in the Look Lab. The owner picks at least two to keep. Exit: the chosen modes are recorded as presets.
- **G2. Metaphor decision (M2).** For each candidate metaphor, agents produce one hero still and one 10-second clip in the Look Lab, all using the same scene. The owner picks one and finalizes the manifesto. Exit: the manifesto is signed.
- **G3. Preference search (M4).** Swipe sessions of about 20 minutes, at most three, over the parameters the metaphor leaves open. Stop when two consecutive sessions converge on the same parameter region, or when the owner calls it. Exit: one to three finalist presets.
- **G4. Sticker palette.** Candidate palettes must pass measurable checks: minimum perceptual distance between adjacent cells (OKLab), a colour-vision-deficiency simulation, and legibility at full detail. The owner picks among the palettes that pass. Exit: palette preset frozen.
- **G5. Key moments.** The owner tunes the motion of turn, undo, invalid action and solved in the Look Lab curve editor. Exit: a motion parameter table.
- **G6. Hardest screens.** The command model, keyboard model and the three hardest screens are designed inside the chosen direction. The owner sketches; agents turn the sketches into draw.io wireframes. Exit: owner-approved wireframes.

**2.4 Vertical slice (inside the experiment window)**
- One scene is built at full quality on the leading renderer candidate: open, see the object, turn one layer, see the result, with the finalist preset, palette and motion.
- It must pass the renderer selection gate. A look that cannot hold full detail at 30 fps does not pass. In that case, return to G3 with the conflicting parameters and their measured cost.
- **Living test:** the owner uses the slice for about 15 minutes a day for five days and writes one sentence a day. Exit: still satisfied after the five days, with a correction list.

**2.5 Freeze**
- A design-system ADR: tokens (colour, spacing, type scale, motion durations and curves), the palette, components, the manifesto and the anti-goals.
- Tokens allow theme switching, and layouts allow other languages and longer strings (themes and subtitles are undecided).
- The owner accepts the ADR. After the freeze, UI engineering implements the system and does not search for the feel again. A later change goes through the Look Lab and an ADR update.

### 4.4 Owner time and tools

| Phase | Owner time (estimate) | Owner does |
|---|---|---|
| Preparation | a few minutes a day | taste diary, anti-goals draft |
| 2.0 | half a day | charter sign-off |
| 2.1–2.2 | one to two hours | pain-point annotation |
| 2.3 | about one hour a day on days 4 to 12 | picks, swipe sessions, tuning, sketches |
| 2.4 | 15 minutes a day for five days | living test |
| 2.5 | one hour | ADR acceptance |

Tools: Godot (Look Lab), Blender (hero stills where Godot is not enough), marimo (preference model and palette checks), draw.io (wireframes), Typst (type specimens). Adding a tool outside the allowlist (for example Krita for sketching) follows the normal allowlist process.

### 4.5 Acceptance for the track

- Signed charter with anti-goals, and a signed manifesto.
- Look Lab build acceptance met.
- Every design decision traces to a preset, a swipe session record or an owner pick.
- The vertical slice passes the renderer selection gate and the living test.
- The design-system ADR is proposed and accepted by the owner.

## 5. Schedule: 20 days (owner decision, 1 October 2026)

Preparation and all of stage 2 finish within 20 calendar days. Day 1 is 1 October 2026; day 20 is 20 October 2026. Tracks run in parallel; the owner's time goes to the design track and to the gate decisions.

| Days | Engineering track | Design track (owner) | Exit |
|---|---|---|---|
| 1–2 | Merge the 0.4.1 step 1 and B4-12 harness branches. Choose the migration path. Codex starts the Look Lab build. | Taste diary and Taste Lab ratings start. Anti-goals draft, two or three candidate metaphors. | **2.0** charter signed (end of day 2), including the migration path. |
| 2–4 | **2.1** two independent inventories as concurrent Codex shards; **2.2** dispositions. | Pain-point annotation. | Dispositions signed (day 4). |
| 3–14 | **2.4** renderer window, 12 days: S-B bare Direct3D 12 probe on days 3–4, then S-A2 and S-D. Go/no-go on window day 7 (day 9), Astra ruling. Final selection on day 14. | — | Renderer selected, or the failure recorded. |
| 4–12 | Look Lab ready by day 4; agents produce variants. | **2.3** G1 (days 4–6), G2 (days 6–8), G3 and G4 (days 8–10), G5 and G6 (days 10–12). | Manifesto, finalist preset, palette, motion table and wireframes approved. |
| 13–15 | **2.4** vertical slice on the leading candidate; gate measurement. | Review of the slice. | Slice passes the selection gate. |
| 15–19 | Correction fixes only. | Living test, five days. | Living test exit. |
| 19–20 | **2.5** architecture freeze (Astra gate ruling), migration format freeze, design-system ADR. | ADR acceptance. | Stage 2 closed. |

Rules for the limit:
- A task that would push past its exit day is cut down or dropped, not extended. The owner decides which.
- If no renderer candidate passes the gate by day 14, the vertical slice uses the best candidate for the design checks only. Stage 2 still ends on day 20 with the failure recorded, and the owner decides the next step.
- No 0.4 performance work and no 0.4.1 release work runs in these 20 days.

## 6. Out of scope

- Choosing the renderer. This track uses the stage 2.4 candidate and does not select it.
- Mechanics, model identity and protection rules. They are unchanged by any UI decision.
- Publishing any reference image, personal session content or screenshot of the owner's machine.
