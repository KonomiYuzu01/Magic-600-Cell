# Stage 2.0 charter (draft for owner signature)

Status: **draft, not signed**. Exit of stage 2.0 is the owner's signature at the end of day 2 (2 October 2026). Sections marked **Owner** are written or chosen by the owner; agents only collect inputs for them. Schedule and limits: [stage-2-experiment-protocol](stage-2-experiment-protocol.md) section 5.

## 1. Purpose

Stage 2 removes the named uncertainties that block building 1.0, in this order: what 0.4 does (2.1), what 1.0 keeps (2.2), how 1.0 looks and works (2.3), which renderer can carry it (2.4), and the frozen architecture and design system (2.5). It ends on day 20 whatever the result, with every open item recorded.

## 2. Binding inputs (not reopened in stage 2)

- Mathematical contract: the full `600-cell-Full` profile, all 259,800 labelled sticker slots, all 1,200 legal generators, finite legal witnesses with full collateral effects, chronological source-to-destination permutations (`AGENTS.md` "Mechanics and model").
- Model identity: `assets/manifest.json` is immutable. Geometry, cuts, IDs, seeds or frames change only with a new model identity and migration.
- Protection rules and the protected-orbit checks before commit.
- Rendering never changes mechanical state or relabels pieces.
- Renderer selection gate ([renderer-candidates](../../wiki/decisions/renderer-candidates.md)): full detail, average at least 30 fps and p99 frame time at most 33.3 ms on the RTX 4070 Laptop GPU, peak VRAM at most about 7 GB.
- 0.4 is not a UX baseline; 1.0 designs its experience from scratch ([owner-decisions-2026-09-30](../../wiki/decisions/owner-decisions-2026-09-30.md)).
- No 0.4.1 release; stage 2 within 20 days ([owner-decisions-2026-10-01](../../wiki/decisions/owner-decisions-2026-10-01.md)).
- Themes and subtitles are undecided; layouts and tokens must allow both.

## 3. Authority

| Decision | Who decides | Record |
|---|---|---|
| Scope, taste, UX, dispositions of DELETE and AUTO, release | Owner | wiki decision page |
| Charter, manifesto, design-system ADR acceptance | Owner | signature line in the document |
| Day-7 go/no-go, migration format freeze, architecture freeze | Codex Astra gate ruling (`--gate`), owner informed | review record and wiki log |
| Correctness of rebuilt components | Differential oracle against the 0.4 engine (protocol section 3) | test output |
| Everything else with a conventional default | Integrator (Claude), recorded | wiki or the stage document |

Model agreement never decides taste or correctness. A task that would pass its exit day is cut or dropped by the owner, not extended.

## 4. Scope of 1.0 (to confirm)

In scope:
- A new application with its own runtime; it does not inherit the 0.4 runtime or process split.
- A dedicated renderer on Direct3D 12 (candidates S-A2 Godot 4.7 .NET, S-D Qt Quick on QRhi; the bare D3D12 probe S-B first).
- The mechanical engine re-implemented or wrapped behind a new boundary, proven equal to 0.4 by the differential oracle.
- One-way migration of 0.4 user data (section 6).
- The development workbench as the project monitor.

Out of scope for stage 2: shipping code, release packaging, NVIDIA-only features on the main path, public data uploads.

## 5. Design charter (**Owner**)

- **Anti-goals** (at least 10, one sentence of reasoning each): _owner draft_.
- **Candidate metaphors** (two or three, one paragraph each): _owner draft_. Examples on the table: observatory instrument, museum exhibit, precision watch, Japanese stationery.
- **UX success criteria** in plain words. Starting proposals for the owner to keep, change or drop:
  1. A first-time user makes a legal turn within 30 seconds of opening.
  2. Every turn shows its full collateral effect before commit, and the commit takes one action.
  3. A turn at full detail responds within 100 ms (the B4-12 M1 interval) on the target machine.
  4. No action loses work: undo is always available, and a crash returns to the last commit.
  5. The solved moment feels earned (owner judgement during the living test).

## 6. Migration path (decision needed by day 2)

Options and the recommendation are in [migration-options](migration-options.md). The charter records the choice; the migration format is frozen at the 2.5 gate.

Chosen path: _owner choice_ (recommended: option B, a 1.0 importer reading a locked copy of the 0.4 database).

## 7. Inputs already available

- 0.4 screening findings as 1.0 requirements: [requirements-from-screening](requirements-from-screening.md).
- Renderer experiment plan and the S-B packet: [renderer-experiment-plan](renderer-experiment-plan.md).
- Inventory packets for 2.1: five concurrent Codex shards `packets/inventory-bottom-up-s1.md` to `-s5.md` (from the code), and an independent top-down inventory by Claude from the user documents (`USAGE.md`, `docs/RELEASE_0_4.md`, `docs/STRUCTURE_EXPLORER.md`).
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
