# Stage 2.0 charter

Status: **signed by the owner on 3 October 2026, as written** ([owner-decisions-2026-10-03](../../wiki/decisions/owner-decisions-2026-10-03.md)). This signature is the exit of stage 2.0. The file keeps its name `charter-2.0-draft.md` so that existing links stay valid. Sections marked **Owner** are written or chosen by the owner; agents only collect inputs for them. Schedule and limits: [stage-2-experiment-protocol](stage-2-experiment-protocol.md) section 5.

## 1. Purpose

Stage 2 removes the named uncertainties that block building 1.0, in this order: what 0.4 does (2.1), what 1.0 keeps (2.2), how 1.0 looks and works (2.3), which renderer can carry it (2.4), and the frozen architecture and design system (2.5). It is planned as about 20 working days; exit days are targets the owner may move, and every open item is recorded at its end.

## 2. Binding inputs (not reopened in stage 2)

- Mathematical contract: the full `600-cell-Full` profile, all 259,800 labelled sticker slots, all 1,200 legal generators, finite legal witnesses with full collateral effects, chronological source-to-destination permutations (`AGENTS.md` "Mechanics and model").
- Model identity: `assets/manifest.json` is immutable. Geometry, cuts, IDs, seeds or frames change only with a new model identity and migration.
- Protection rules and the protected-orbit checks before commit.
- Rendering never changes mechanical state or relabels pieces.
- Renderer selection gate ([renderer-candidates](../../wiki/decisions/renderer-candidates.md)): full detail, average at least 30 fps and p99 frame time at most 33.3 ms on the RTX 4070 Laptop GPU, peak VRAM at most about 7 GB.
- 0.4 is not a UX baseline; 1.0 designs its experience from scratch ([owner-decisions-2026-09-30](../../wiki/decisions/owner-decisions-2026-09-30.md)).
- No 0.4.1 release ([owner-decisions-2026-10-01](../../wiki/decisions/owner-decisions-2026-10-01.md)); stage 2 planned as about 20 owner working days with target exit days ([owner-decisions-2026-10-02](../../wiki/decisions/owner-decisions-2026-10-02.md)).
- Windows first; later macOS and Linux versions stay cheap: platform code behind narrow interfaces, one portable HLSL shader source, a small renderer backend interface ([owner-decisions-2026-10-02](../../wiki/decisions/owner-decisions-2026-10-02.md)).
- Several visual themes on one shared design language, which users can tune ([owner-decisions-2026-10-02](../../wiki/decisions/owner-decisions-2026-10-02.md)). Subtitles are undecided; layouts and tokens must allow them.

## 3. Authority

| Decision | Who decides | Record |
|---|---|---|
| Scope, taste, UX, dispositions of DELETE and AUTO, release | Owner | wiki decision page |
| Charter, manifesto, design-system ADR acceptance | Owner | signature line in the document |
| Day-7 go/no-go, migration format freeze, architecture freeze | Codex Astra gate ruling (`--gate`), owner informed | review record and wiki log |
| Correctness of rebuilt components | Differential oracle against the 0.4 engine (protocol section 3) | test output |
| Everything else with a conventional default | Integrator (Claude), recorded | wiki or the stage document |

Model agreement never decides taste or correctness. A task likely to miss its target exit day is reported once with a re-plan, and the owner moves the target, cuts the task or drops it.

## 4. Scope of 1.0 (confirmed at signing)

In scope:
- A new application with its own runtime; it does not inherit the 0.4 runtime or process split.
- A dedicated renderer on Direct3D 12 (candidates S-A2 Godot 4.7 .NET, S-D Qt Quick on QRhi; the bare D3D12 probe S-B first).
- The mechanical engine re-implemented or wrapped behind a new boundary, proven equal to 0.4 by the differential oracle.
- One-way migration of 0.4 user data (section 6).
- The development workbench as the project monitor.
- One GUI in which the layout that makes the kept functions easy to use and the chosen art style are designed together (protocol section 4), on six layers with frozen boundaries: engine (mathematics, proven by the oracle), session store (journal, checkpoints, migration), command layer (one command table: ID, permission, preview, undo, contexts; every button, key and menu calls it), view model, shell (layout, panels, motion) and renderer (Direct3D 12, reads labels and view state only). The command table is where function and layout meet.

Out of scope for stage 2: shipping code, release packaging, NVIDIA-only features on the main path, public data uploads.

## 5. Design charter (**Owner**)

Owner input of 2 October 2026, translated from the owner's notes and clarifications by Claude. The owner confirmed the wording as written at signing on 3 October 2026; the original notes stay private.

- **Product thesis.** A complete solve of the 600-cell is harder in practice than its mathematical description suggests. 1.0 gives the solver tools that compensate for that complexity, so that the result still counts as a human solve with appropriate tools. Once learned, the tool is as efficient as an office application used from the keyboard alone (the reference is the Hyperspeedcube keybind workflow); learning it feels like progressing through a game with a complex but learnable mechanic. The toolset and workflow are built backward from the only practical full-solve method, orbit-first block building ([solving-workflow](solving-workflow.md)). In 0.4 this failed: the functions under Command All could not be found or understood, and even a simple operation was hard.
- **Target users.** Core: people who seriously intend to solve the complete 600-cell (the owner and, as far as the owner knows, two or three others in the community). Secondary: people who know hypercube puzzles and want to understand the 600-cell and explore the program. The core users' workflow decides trade-offs; the secondary users get exploration and onboarding (S1, S3, S6) that never slows the core workflow.
- **Human-solve boundary.** Allowed: tools outside the puzzle itself: entering, recording and editing macros, macro analysis (effect, cycles, frames, collateral, protection), templates and reuse of the solver's own macros, tracking, protection and view aids. A macro is entered by the solver and the tool outputs its analysis. **Setup search is allowed** (owner, 2 October 2026, replacing the 0.4 ban): for a target the solver names, the program may propose setups that keep the buffers fixed, for example from the certified guarded setup tables; the solver chooses the macro, may edit the setup, and executes. Not allowed: the program choosing or outputting a solving macro by itself, executing without the solver's explicit action, or solving automatically.
- **Learning and reward.** The skill is learned level by level, as in a game with generous rewards for real progress (visual, optionally sound); the steps of the solving workflow are the levels. The mathematical language of the tool is part of what the user learns, introduced step by step. The interface itself does not look like a game (anti-goal 7). The structure is always the reward; real content (a diagram, a theory note, the solver's macro saved as a template) unlocks at only three occasions (first checked insertion with the solver's own macro, first completed block, first exactly completed orbit) plus one quiet whole-puzzle moment, always small, in place and never modal; undo withdraws a current achievement but keeps unlocked explanations and saved macros. Learning is measured by competence checks on fixture tasks and by local in-situ metrics on real solving; keys open by soft disclosure and the command palette always offers every command (owner design decisions of 2 October 2026: [owner-decisions-2026-10-02-design](../../wiki/decisions/owner-decisions-2026-10-02-design.md)).
- **Owner design requirements.**
  - The Global and Local views are redesigned from scratch; the 0.4 views are not a baseline. There is no third view: both are visible and linked, and residual and operation summaries are embedded in them (D8).
  - High-quality graphics replace most text. Text that remains is short, uses glossary terms and is reviewed by a person. Trial budgets of free sentences per whole screen: about 20 words while solving, 50 in opened inspection, 80 in an opened learning panel (set by G6; errors and the theory book exempt).
  - Several themes share one design language of high quality that is efficient in use and compatible with the engineering: renderer budget, picking and legibility. Users can tune their own comfortable experience; Taste Lab and Look Lab data can inform the presets. A scene (solving, inspecting, celebrating) may have its own look. 1.0 ships two authored theme families, each with the three scene looks; commands, keys, layout anchors, glyph and status meanings and motion meanings are fixed across families.
  - The owner's own interests and nexus points belong in the design; the metaphor and the mathematics hold them together. They are collected as annotated reference pairs and nexus cards (interest, bridge to the project, product place).
  - One encoding grammar: colour carries a class, never identity; position and anchors carry identity and relations; glyphs carry roles; patterns carry status; motion carries change. Full encoding inside the focus set, class colour outside it, with protection conflicts and operation effects always breaking through. Exact language sits in a relationship strip inside Global and Local. It must read intuitively, picture first, and its mathematical grammar is defined, with proofs or cited proofs, in the theory book (M4), at the reduced scale of [owner-decisions-2026-10-02-scope](../../wiki/decisions/owner-decisions-2026-10-02-scope.md): only what the 1.0 UI shows, with proofs only for the facts its correctness relies on.
  - One entry for everyone: one workspace and path, task-led optional coaching, direct resume for returning users, and a clearly labelled practice copy of the full model.
  - Role priority: an engineer's evaluation that demonstrates something wrong affecting the work or correctness is fixed first; otherwise, among admissible options, core workflow, then legibility and delight, then cost beyond the gates, then notation preference beyond correctness.
  - Sound is reduced ([owner-decisions-2026-10-02-scope](../../wiki/decisions/owner-decisions-2026-10-02-scope.md)): one short G5 comparison (silent against a two-cue set), then the owner adopts or rejects it; if adopted, at most two cues in 1.0 ("completed", "invalid action"), shared by all themes, each with a visual equivalent and mute; motion-table sound semantics and the full set move to 1.x.
- **Anti-goals** (owner draft, more to come; split into single checkable items by Claude; the reasons are Claude's reading of the owner's notes, confirmed at signing on 3 October 2026; more anti-goals may be added by the owner):
  1. **Outdated stack.** Never build on an outdated UI stack such as WinForms on the .NET Framework with Managed DirectX, the 0.4 host: effective new work cannot be built on a stack Windows has moved past.
  2. **Screens full of text.** Never use text where a graphic can carry the meaning: text is slow to read in a dense, repetitive workflow.
  3. **Unreviewed wording and stray jargon.** Never ship UI text that no person has reviewed, or terms that are not in the glossary: 0.4's front end was cluttered with generated wording nobody could follow.
  4. **One flat tone.** Never a single tone with flat animation and uniform images: that is what makes a product read as low-effort.
  5. **Office-software look.** Never look like generic office software, with plain lines, plain colours, thin panels and clashing graphics: 1.0 borrows office software's efficiency, not its look.
  6. **Style against engineering.** Never a style the renderer, picking or legibility cannot support, or tools whose looks do not belong together: style and function are designed as one.
  7. **Game-HUD interface.** Never a game-like interface with low information density: experts need dense, precise information. Learning may feel like a game; the screen does not.
  8. **Effects over geometry.** Never particle or other effects that hide geometry or get in the way of operation.
  9. **Performance spent on decoration.** Never spend frame time where it does not help while turns or picking miss their budgets.
  10. **One style only.** Never only one aesthetic, such as only minimal or only sci-fi, as if the product were a single art project.
  11. **Frame apart from the work.** Never let the interface frame stand apart from the puzzle view, or let scaling break the picture or operation.
  12. **One fixed look.** Never offer only one tone: users need the freedom to tune a comfortable experience.
  13. **Only mainstream taste.** Never only generic mainstream aesthetics: the owner's own interests and nexus points make the product distinct.
- **Candidate metaphors** (two or three, one paragraph each): _owner draft_. Examples on the table: observatory instrument, museum exhibit, precision watch, Japanese stationery.
- **Quality bar** (kept by the owner at signing; numbers marked _proposal_ stay proposals as stated below). Intent: 1.0 should read as the work of a senior software engineer, a mathematician, a front-end designer and an interaction artist working together. The owner's definitions of the four roles: the engineer builds the high-performance graphics and renderer and the low-latency backend, integrates, accepts and packages, and implements the other three roles' designs in high-quality code; the mathematician builds the tool's own mathematical language (the existing IDs are only part of it), keeps the remaining mathematical rigour and continues the research reports; the front-end designer designs the whole skill, the layout and the mechanisms, from convenience and the product thesis, together with the artist, for an art-integrated front end; the interaction artist colours the project from the trained Taste Lab, keeps the skill enjoyable, and understands the engineering and the mathematics well enough to do so. Every bar below can be checked by a measurement, a review against a written list, or an owner judgement in the living test. The bars apply to the vertical slice's scope on day 15 and to the whole product at release. They never relax a binding input in section 2. Numbers marked _proposal_ are set in 2.3 and frozen at 2.5.

  **Signature moments.** Four to six moments get many times the polish of ordinary features; everything else must be clean and consistent, not extraordinary. For each one the owner names a benchmark work, and the living test compares the two side by side.
  - S1 First launch: the user reaches a first legal turn within 30 seconds, and the structure is introduced from one cap and one orbit, not from all 259,800 stickers at once.
  - S2 One cap turn: motion, easing and (if adopted) sound make the four-dimensional rotation read as a rotation; the new labels are adopted on the frame where the turn ends.
  - S3 Structure view: any cap, orbit, frame group or Hopf fibre can be isolated and highlighted.
  - S4 Preview and witness: before commit, a turn or macro shows its full collateral effect, its witness sequence, its effect on every orbit and on protection; the commit takes one action.
  - S5 The solved moment, for one orbit and for the whole puzzle, feels earned.
  - S6 The opening chapter of the theory book.

  **Engineering.**
  - E1 The renderer selection gate (section 2) holds in the shipped look (W5), not only in a bare scene.
  - E2 A turn at full detail responds within 100 ms (the B4-12 M1 interval) on the target machine.
  - E3 Camera input latency and startup-to-interactive time have budgets set from S-B and vertical-slice measurements (_proposal_: interactive within 3 seconds with an existing session).
  - E4 No action loses work: undo is always available, a crash returns to the last commit, and requirements R-01 to R-19 each have a passing test.
  - E5 Every action is reachable from the keyboard through the command table, including a command palette.
  - E6 Every error message says what happened, why, and what the user can do.
  - E7 Install, update and uninstall leave nothing outside the declared directories, and every build is traceable to its build identity.

  **Mathematics.**
  - M1 Every term in the UI comes from one glossary tied to `research/PUZZLE_THEORY.md`: one term per concept, checked by script.
  - M2 Every visualization states its mathematical basis (projection, fibration, colouring rule) in the theory book; no geometry that misrepresents the structure.
  - M3 Shown numbers are exact or state their precision; evidence status (verified, recorded, unknown, unchecked) is never merged with a score or a permission.
  - M4 The theory book has definitions, proofs or cited proofs, a notation table, and figures rendered from the real geometry with local tools, reproducibly. This includes the UI's own grammar: every term, symbol and relation used by the encoding and the relationship strip. Scope in 1.0 (reduced, [owner-decisions-2026-10-02-scope](../../wiki/decisions/owner-decisions-2026-10-02-scope.md)): only what the 1.0 UI shows; proofs only for facts the UI's correctness relies on, short or cited; background mathematics moves to 1.x.
  - M5 A mathematics review (Fable or Codex Astra; a human mathematician if the owner approves the cost) checks terminology and visualizations against the theory before the vertical slice and before release; in 1.0 it covers the reduced set of M4.
  - M6 The tool's mathematical language (names for orbits, blocks, buffers, stars, residuals and frames) is designed to be learned: each term enters at the level where it is first needed, with a graphic.

  **Design.**
  - D1 Design tokens cover colour, type scale, spacing grid, radius, elevation and motion; product code refers to token names only, checked by lint.
  - D2 Every view has designed empty, loading, error, stale ("applied, display stale") and busy states.
  - D3 A custom icon set drawn from the project's geometry, on one grid and one stroke weight.
  - D4 The 600-cell palette is designed in OKLab and passes the adjacent-cell ΔE threshold and colour-vision-deficiency simulation set in G4.
  - D5 Dense panel layouts keep a clear hierarchy at the target resolution and under high-DPI scaling.
  - D6 Typography includes a face that sets mathematical notation correctly.
  - D7 A learning path takes a user from the first turn to a full orbit-first solve, level by level, with each level tied to a step of the [solving workflow](solving-workflow.md).
  - D8 Global and Local are redesigned and each answers one question: Local, does this piece match its target here; Global, how do these two places relate. They are the only two views, visible together and linked to one context and revision.

  **Interaction and art.**
  - A1 Every motion has a meaning recorded in the motion table (H-03); no motion is decoration only.
  - A2 Turn interpolation follows the actual four-dimensional rotation; the owner picks the easing in Look Lab.
  - A3 Materials, light and transparency serve the legibility of the structure; an effect that hides structure is dropped, whatever it costs to build.
  - A4 Sound, if adopted, is at most two cues in 1.0 and can be switched off; its design with the motion table moves to 1.x ([owner-decisions-2026-10-02-scope](../../wiki/decisions/owner-decisions-2026-10-02-scope.md)).
  - A5 The owner signs each signature moment in the living test against its benchmark.
  - A6 Rewards come from the structure itself and only from real progress (a block built, an orbit solved), never interrupt input, can be switched off, and quieten for experts.

  **Review lenses.** Candidate reviews keep the engineering lens (Codex). Proposed additions: a mathematics lens (M1 to M6), a design lens (Look Lab or slice screenshots checked against D1 to D8) and an art lens (the owner as art director, A1 to A6). Role cards for the new lenses are a follow-up process change and are not part of signing this charter.

## 6. Migration path (decision needed by day 2)

Options and the recommendation are in [migration-options](migration-options.md). The charter records the choice; the migration format is frozen at the 2.5 gate.

Chosen path: **option B**, a 1.0 importer reading a locked copy of the 0.4 database (owner, 2 October 2026). Design: [migration-exporter-proposal](migration-exporter-proposal.md). The Astra plan check of path B passed after four adopted findings (2 October 2026). Remaining prerequisite before the importer is built: the Windows probes P1 to P3 on synthetic sessions, run on the owner's machine. If P1 finds no connection strategy that leaves the source unchanged, the owner decides between the conflicting invariants.

## 7. Inputs already available

- 0.4 screening findings as 1.0 requirements: [requirements-from-screening](requirements-from-screening.md).
- Renderer experiment plan and the S-B packet: [renderer-experiment-plan](renderer-experiment-plan.md).
- Stage 2.1 inventories: two independent sealed inventories from source (Codex and Claude, six shards each), a source census, screening-finding attachments and the comparison ([inventory/README.md](inventory/README.md), [inventory/merged.md](inventory/merged.md)); the document-based [top-down list](inventory/top-down.md) is kept as a third reference.
- Glossary for M6: [glossary/glossary.md](glossary/glossary.md) (30 terms with diagrams; levels provisional).
- Differential oracle specification and tools: [oracle/README.md](oracle/README.md).
- Stage 2.4 renderer packets: [packets/renderer/README.md](packets/renderer/README.md).
- Correctness oracle: protocol section 3 and `docs/architecture/1.0/10_V1_ARCHITECTURE.md` V10 (`C600-COMPARE-FIXTURE-v1`).

## 8. Exit criteria per sub-stage

| Stage | Exit | Day |
|---|---|---|
| 2.0 | This charter signed, with sections 5 and 6 filled | 2 |
| 2.1 | Two independent inventories merged into one list; each disagreement listed | 3 |
| 2.2 | Every inventory item has keep, redesign, delete or automate, with a reason; owner signs DELETE and AUTO items | 4 |
| 2.3 | Manifesto, finalist preset, palette, motion table and hardest-screen wireframes approved | 12 |
| 2.4 | Renderer selected or failure recorded (day 14); vertical slice passes the gate (day 15); living test exit (day 19) | 14–19 |
| 2.5 | Architecture freeze and migration format freeze gate rulings; design-system ADR accepted | 20 |

## 9. Signature

Signed by the owner on 3 October 2026, as written, sections 5 and 6 filled. The owner approved in a chat message; the record is [owner-decisions-2026-10-03](../../wiki/decisions/owner-decisions-2026-10-03.md). Items this signature does not settle: the manifesto (stage 2.3), the review-lens role cards (section 5, a later process change) and every number marked _proposal_.
