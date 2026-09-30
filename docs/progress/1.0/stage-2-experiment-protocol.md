# Stage 2 experiment protocol (draft)

Status: **draft, not accepted**. It proposes how stage 2 (2.0 to 2.5) runs experiments so that trial and error stays cheap and every attempt leaves a usable result. It changes no rule, no ADR and no gate. The renderer selection gate stays as recorded in [renderer-candidates](../../wiki/decisions/renderer-candidates.md), and UX sign-off stays with the owner (`AGENTS.md` "Ask the owner").

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

When 1.0 rebuilds a component that has a 0.4 counterpart, a differential oracle checks it. The 0.4.1 engine (`core.py` and the reference maps) is the reference.

- Run the same scripted input (generators, macros, fixture sessions) through the 0.4.1 reference and the new component.
- Compare full labelled-state hashes after every step, not only at the end.
- The first mismatching step is the bug report. A mismatch is never accepted as "close enough".
- The 0.4.1 `.c600migrate` fixtures ([exporter-spec-draft](../0.4.1/exporter-spec-draft.md) section 5) are the shared inputs.

This keeps AI-written code cheap to verify: a change either matches the reference or points at a step.

## 4. UI vision track

Goal: turn the owner's idea of an artistic, high-quality, satisfying interface into a fixed design system before UI engineering starts. It runs in parallel with the renderer experiments and feeds stage 2.3 (the three hardest screens and design tokens) and stage 2.4 (the vertical slice).

Roles:
- **Owner:** supplies references, picks between variants and signs off. Owner time per round should be under 30 minutes.
- **Agents:** extract parameters, produce variants, build the slice and write the tokens.

### V0. Reference board (owner, about half a day)

- Collect 20 to 40 references that feel right and 5 to 10 that feel wrong. Any field works: games, scientific visualization, tools, motion design, architecture, painting.
- Write one sentence per reference that says what is right or wrong, for example "lots of empty space", "motion has weight", "colours are clean", "does not look like a web page".
- **Privacy and licence:** the images stay private under `work/loop-memory/ui-vision/`. Third-party images are not committed or published. Only the sentences, with a neutral source description, may enter the repository.

### V1. Feel brief (agents draft, owner edits)

Translate the sentences into adjustable parameters. Each parameter gets a proposed range, not a single value.

| Area | Parameters |
|---|---|
| Hierarchy | Is the 4D object or the tool panel the main subject; how much screen the object gets |
| Density | Controls per screen; panels hidden, on demand or always visible |
| Motion | Turn animation duration, easing curve, inertia and settle, camera damping |
| Colour and material | Dark or light base, saturation, sticker material (matte, gloss, glass), depth cues such as fog and depth of field |
| Typography | Typeface family, weights, tabular figures for counters and timers |
| Feedback | Hover and press states, error presentation, optional sound |

Output: `docs/progress/1.0/ui-feel-brief.md`, with each parameter linked to the reference sentences that support it.

### V2. A/B rounds (agents produce, owner picks; 1 to 2 days per round)

- Each round changes one or two parameters and shows 3 or 4 variants side by side. Everything else stays fixed.
- Artifacts are static renders or clips of at most 10 seconds (Blender for look development, a Godot scene, or a still image). Nothing is wired to the engine yet.
- The owner ranks the variants. A reason is optional.
- Record each round on a `UV-<n>` card: parameters, variants, pick, and the narrowed range.
- Stop a parameter when two rounds in a row pick the same region. Stop the track after about 5 rounds, or earlier if the owner calls it.
- Variants use synthetic geometry from the model. They are never screenshots of a personal session.

### V3. Vertical slice (inside the stage 2.4 window)

- Build one representative scene to full quality: open the app, see the object, drag to turn one layer, see the result. It uses the chosen motion, type, colour and lighting.
- Build it on the leading renderer candidate, so taste and performance are checked together. The selection gate still applies: a look that cannot hold full detail at 30 fps on the target machine does not pass.
- The owner reviews it on the target machine. Screenshots and clips are not a substitute.
- Kill criteria: if the slice cannot reach the chosen look within the gate, return to V2 with the conflicting parameters and a measured cost for each.

### V4. Design system freeze

- Record the accepted values as design tokens (colour, spacing, type scale, motion durations and curves) and a component list.
- Propose an ADR for the design system. The owner accepts it; accepting ADRs is an owner item.
- Themes and subtitles are undecided (owner decisions 2026-09-29). Tokens must allow theme switching, and layouts must allow text in other languages and longer strings. Do not assume English-only subtitles.
- After the freeze, UI engineering implements the tokens and components. It does not search for the feel again. A later change to the feel goes through a new `UV` round and an ADR update.

### Acceptance for the track

- The feel brief exists, and each parameter traces to reference sentences.
- Every `UV` round has a card with the owner's pick.
- The vertical slice passes the renderer selection gate and has owner sign-off.
- The design-system ADR has been proposed with tokens, components, and theme and language allowances.

## 5. Out of scope

- Choosing the renderer. This track uses the stage 2.4 candidate and does not select it.
- Mechanics, model identity and protection rules. They are unchanged by any UI decision.
- Publishing any reference image, personal session content or screenshot of the owner's machine.
