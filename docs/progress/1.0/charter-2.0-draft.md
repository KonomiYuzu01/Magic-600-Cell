# Stage 2.0 charter (draft for owner signature)

Status: **draft, not signed**. Exit of stage 2.0 is the owner's signature, targeted for working day 2 (the owner's next working day after 1 October 2026). Sections marked **Owner** are written or chosen by the owner; agents only collect inputs for them. Schedule and limits: [stage-2-experiment-protocol](stage-2-experiment-protocol.md) section 5.

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
- Themes and subtitles are undecided; layouts and tokens must allow both.

## 3. Authority

| Decision | Who decides | Record |
|---|---|---|
| Scope, taste, UX, dispositions of DELETE and AUTO, release | Owner | wiki decision page |
| Charter, manifesto, design-system ADR acceptance | Owner | signature line in the document |
| Day-7 go/no-go, migration format freeze, architecture freeze | Codex Astra gate ruling (`--gate`), owner informed | review record and wiki log |
| Correctness of rebuilt components | Differential oracle against the 0.4 engine (protocol section 3) | test output |
| Everything else with a conventional default | Integrator (Claude), recorded | wiki or the stage document |

Model agreement never decides taste or correctness. A task likely to miss its target exit day is reported once with a re-plan, and the owner moves the target, cuts the task or drops it.

## 4. Scope of 1.0 (to confirm)

In scope:
- A new application with its own runtime; it does not inherit the 0.4 runtime or process split.
- A dedicated renderer on Direct3D 12 (candidates S-A2 Godot 4.7 .NET, S-D Qt Quick on QRhi; the bare D3D12 probe S-B first).
- The mechanical engine re-implemented or wrapped behind a new boundary, proven equal to 0.4 by the differential oracle.
- One-way migration of 0.4 user data (section 6).
- The development workbench as the project monitor.
- One GUI in which the layout that makes the kept functions easy to use and the chosen art style are designed together (protocol section 4), on six layers with frozen boundaries: engine (mathematics, proven by the oracle), session store (journal, checkpoints, migration), command layer (one command table: ID, permission, preview, undo, contexts; every button, key and menu calls it), view model, shell (layout, panels, motion) and renderer (Direct3D 12, reads labels and view state only). The command table is where function and layout meet.

Out of scope for stage 2: shipping code, release packaging, NVIDIA-only features on the main path, public data uploads.

## 5. Design charter (**Owner**)

- **Anti-goals** (at least 10, one sentence of reasoning each): _owner draft_.
- **Candidate metaphors** (two or three, one paragraph each): _owner draft_. Examples on the table: observatory instrument, museum exhibit, precision watch, Japanese stationery.
- **Quality bar** (proposal for the owner to keep, change or drop). Intent: 1.0 should read as the work of a senior software engineer, a mathematician, a front-end designer and an interaction artist working together. Every bar below can be checked by a measurement, a review against a written list, or an owner judgement in the living test. The bars apply to the vertical slice's scope on day 15 and to the whole product at release. They never relax a binding input in section 2. Numbers marked _proposal_ are set in 2.3 and frozen at 2.5.

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
  - M4 The theory book has definitions, proofs or cited proofs, a notation table, and figures rendered from the real geometry with local tools, reproducibly.
  - M5 A mathematics review (Fable or Codex Astra; a human mathematician if the owner approves the cost) checks terminology and visualizations against the theory before the vertical slice and before release.

  **Design.**
  - D1 Design tokens cover colour, type scale, spacing grid, radius, elevation and motion; product code refers to token names only, checked by lint.
  - D2 Every view has designed empty, loading, error, stale ("applied, display stale") and busy states.
  - D3 A custom icon set drawn from the project's geometry, on one grid and one stroke weight.
  - D4 The 600-cell palette is designed in OKLab and passes the adjacent-cell ΔE threshold and colour-vision-deficiency simulation set in G4.
  - D5 Dense panel layouts keep a clear hierarchy at the target resolution and under high-DPI scaling.
  - D6 Typography includes a face that sets mathematical notation correctly.

  **Interaction and art.**
  - A1 Every motion has a meaning recorded in the motion table (H-03); no motion is decoration only.
  - A2 Turn interpolation follows the actual four-dimensional rotation; the owner picks the easing in Look Lab.
  - A3 Materials, light and transparency serve the legibility of the structure; an effect that hides structure is dropped, whatever it costs to build.
  - A4 Sound, if adopted, is designed together with the motion table and can be switched off.
  - A5 The owner signs each signature moment in the living test against its benchmark.

  **Review lenses.** Candidate reviews keep the engineering lens (Codex). Proposed additions: a mathematics lens (M1 to M5), a design lens (Look Lab or slice screenshots checked against D1 to D6) and an art lens (the owner as art director, A1 to A5). Role cards for the new lenses are a follow-up process change and are not part of signing this charter.

## 6. Migration path (decision needed by day 2)

Options and the recommendation are in [migration-options](migration-options.md). The charter records the choice; the migration format is frozen at the 2.5 gate.

Chosen path: _owner choice_ (recommended: option B, a 1.0 importer reading a locked copy of the 0.4 database).

## 7. Inputs already available

- 0.4 screening findings as 1.0 requirements: [requirements-from-screening](requirements-from-screening.md).
- Renderer experiment plan and the S-B packet: [renderer-experiment-plan](renderer-experiment-plan.md).
- Inventory packets for 2.1: five concurrent Codex shards `packets/inventory-bottom-up-s1.md` to `-s5.md` (from the code), and an independent top-down inventory by Claude from the user and design documents ([inventory/top-down.md](inventory/top-down.md), 72 functions, 7 unclear items).
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

Signed by the owner: _name, date_ — sections 5 and 6 filled.
