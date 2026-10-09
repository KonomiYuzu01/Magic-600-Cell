# Jumbling mid-stage acceptance table

Status: **mid-stage record of the [jumbling plan](jumbling-plan.md), 9 October 2026, for the milestone audit.** Plan section 2 defines the mid-stage: an acceptance table, audited at the milestone, links each deliverable to its contract revision, its exact menu identity, its replayable fixture and its result. Then the two mid-stage packets go to the owner.

Evidence kind for every row: source, fixture and synthetic geometry from a Linux cloud session. Nothing here is Windows, Direct3D 12, input or performance evidence.

## 1. Identities

- **Contract revision:** `state-contract 2026-10-09 A1-A4` ([state contract](../../../research/jumbling/state-contract.md), amendments in section 7). Every journal below records it.
- **Model identity:** the SHA-256 of `assets/model.npz` and `assets/primitives.npz` as read, recorded in every journal (J1 `model.py`). No asset changed.
- **Menu identities** (J1 `TwistMenu.identity`: SHA-256 over the sorted canonical exact matrices of the base-cap menu, independent of labels and order):

  | Menu | Elements | Identity | Built by |
  |---|---:|---|---|
  | A4₀ (control; 600-cell-Full) | 12 | `02d0b2c991c7f58f48b52210b16771cf94956770ac7e0e3276de1f1522e411f6` | `sim.TwistMenu.a4` |
  | S4₀ | 24 | `793fcfb284b0ccb08b84202f7f153615a88657b58731d1f4e489d7105394033b` | `sim.TwistMenu.s4` |
  | I_a | 60 | `fddc8532b9184bd1b413c1032b5a7a305dd9fd1f10385d87ba6ec4fd06eb958c` | `theory/groups.py` |
  | I_b | 60 | `d70c122f8fd517f6b5baf08442c4c58180ae4d06596eef17288834d0455b3fde` | `theory/groups.py` |
  | NC (negative control) | 16 | `b1f904f6c88d9f2562de4dbbb3cf1e766f176c884825da037117c581039843c4` | J1 acceptance item 9 |
  | free mode | — | `null` (every exact twist) | J1 default |

## 2. Table

| Row | Deliverable | Menu | Replayable fixture | Result | Review |
|---|---|---|---|---|---|
| J1 | Exact reference engine `research/jumbling/sim/` | free mode; controls A4₀, S4₀, NC | journals in `research/jumbling/sim/acceptance.json` (witness E2–E4, 20 seeded mixed sequences, negative control); rerun `python research/jumbling/sim/accept.py` | acceptance items 1–9 and A1–A4: 32 of 32 flags true; 38 focused tests pass (`tests/test_jumbling_sim.py`) | Astra full review in two shards (`20261009T210942Z-04ccee5c`, `20261009T210943Z-d2ec15dd`; six majors adopted and fixed in `0fb5494`), scoped verification passed (`20261009T213956Z-8426390f`) |
| J2 | Viewer `research/jumbling/viewer/` showing certified J1 sequences | S4₀ | `scene-s4.json` with its J1 journal (four twists and one rejected attempt); rerun `python research/jumbling/viewer/export_scene.py --source sim-s4` | 15 of 15 export checks true (every twist in S4₀ and certified, surveys complete, the rejected attempt certified blocked with the state unchanged, final digest equal to solved, vertices and certificates equal to J1 exactly); headless browser check 105 of 105 assertions for both scenes | viewer: routine review `20261009T195031Z-2a42a035`, fixes verified by Astra (`20261009T202151Z-5320d9c6`); the S4 scene (Codex-authored, `82a8ab2`) reviewed by the integrator |
| J4 | Grip-orbit explorer `research/jumbling/explorer/` | J4_MENUS | `explorer-results.json` and `points/`; rerun `python research/jumbling/explorer/explore.py --preset` | J4_RESULT | J4_REVIEW |
| J3 | Theory draft `research/jumbling/theory/theory-draft.md` | S4₀, I_a, I_b, NC | scripts and results in `research/jumbling/theory/`; the W-J fixtures | J3_RESULT | J3_REVIEW |
| W-J | Fixtures for both packets `research/jumbling/fixtures/` | S4₀, I_a, I_b | `wj-<menu>.json`; rerun `python research/jumbling/fixtures/wj.py check` | WJ_RESULT | integrator-written tool; replay is J1 itself, and `check` recomputes every reference |

## 3. Mid-stage packets

| Packet | Line | Status |
|---|---|---|
| [E-2.4-0J](packets/renderer/E-2.4-0J-jumbling.md) | renderer (stage 2.4) | written; `tools/perf/check_renderer_packets.py` passes; to the owner |
| [ENG-0J](packets/engineering/ENG-0J-jumbling-backend.md) | engineering (2.3 to the 2.5 freeze) | written; to the owner |

## 4. What mid-stage does not settle

- The twist menu: an owner sign-off at the 2.5 freeze (owner decision, 9 October 2026). The candidates with finite same-cap behaviour are A4₀, S4₀, I_a and I_b (theory draft, Proposition 3.1); any other base-cap group is infinite, and so is the set of configurations at that cap alone.
- Finiteness of R(Λ) for S4₀, I_a and I_b, item 4 (whether the eight extra lattice states lie in the retained group) and items 5 and 6 of the theory: open.
- Every Windows, Direct3D 12 and performance question: E-2.4-0J runs on the owner's machine.
