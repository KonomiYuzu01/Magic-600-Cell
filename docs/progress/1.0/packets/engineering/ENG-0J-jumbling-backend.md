# Packet ENG-0J: backend boundaries for the jumbling puzzle

For the engineering line of stage 2 (2.3 redesign to the 2.5 interface freeze). It is a design hand-off, not an authorization to write product code: the 2.5 freeze and the engine-language question ([engine-language-rust](../../../../wiki/questions/engine-language-rust.md)) decide how it is built. The integrator runs it; separable parts (the differential harness and the store round trip) go to Codex through the wrapper with their own implement contracts, as `AGENTS.md` "Pair implementation" says.

Source: [jumbling plan](../../jumbling-plan.md) section 3b, made standalone at mid-stage (owner, 9 October 2026: the jumbling work affects both the renderer line and the engineering line; the renderer side is [E-2.4-0J](../renderer/E-2.4-0J-jumbling.md)). The plan, the [state contract](../../../../../research/jumbling/state-contract.md) and the [owner decision](../../../../wiki/decisions/owner-decisions-2026-10-09-jumbling.md) win where they differ from this packet.

## 1. Goal and acceptance
- Goal: give the jumbling puzzle a place in each backend layer of charter section 4 (engine, session store, command layer, view model) before the 2.5 freeze, so that the freeze covers both puzzles. 600-cell-Full and its contract are unchanged.
- Acceptance:
  1. **Interface proposal** (`docs/progress/1.0/jumbling-interfaces.md`): for each of the four layers, the data types, operations, revision rules and typed errors of the jumbling puzzle, in the vocabulary of the [command table](../../command-table.md) section 1. Every operation names the contract section or amendment it implements. Types and interfaces take the dimension as a parameter and do not fix it; the implementation and every acceptance cover d = 4 only (owner decision 6, 9 October 2026).
  2. **Differential harness** against J1: the product engine (or any candidate engine in the engine-language spike) replays each W-J fixture and every J1 acceptance journal, and after every step its canonical state digest, lattice flags, moved set and grip status equal J1's. Mismatches stop at the first differing step and name it.
  3. **Negative cases** in the harness, each refused without a state change: a stale revision, a non-representable rotation, a twist outside the signed menu, a menu record that is not inverse-closed or not A4-invariant, a checkpoint without a witness, an export off the lattice, and the exact negative control of J1 acceptance item 9, which must be admissible at every round with pairwise distinct digests and growing entry heights, as in J1.
  4. **Store round trip:** every fixture state and journal survives write, crash-recovery and read through the proposed session-store format with an equal digest, and the stored numbers are exact.
  5. **Cost leads** for the engine-language question: the entry heights and denominators observed in the fixtures, and the cost of exact classification against the filtered test of amendment A2, measured headless with the J1 reference. These are leads, never performance evidence for a product engine.
  6. **Command rows** proposed for the H-08 command table, with the human-solve boundary checked row by row.
- Non-goals: product code, the renderer (E-2.4-0J), the look of any overlay, the menu choice (an owner sign-off at the 2.5 freeze), the theory (J3), macOS and Linux builds.

## 2. Actual problem and reproduction
- 1.0 has two puzzles (owner decision, 9 October 2026): 600-cell-Full, whose state is a labelled slot permutation of 259,800 stickers, and the jumbling puzzle on the same geometry, whose state is an exact pose per piece. The six-layer architecture of charter section 4 was drawn for the first only:
  - the engine is "proven equal to 0.4 by the differential oracle", but 0.4 has no jumbling state;
  - the session store keeps a journal of retained moves and label checkpoints;
  - the command layer knows retained operations and macros;
  - the renderer reads labels only.
- What the jumbling puzzle needs instead (contract sections 1–5 and amendments A1–A4):
  - **State.** 177,120 pieces, each with an exact 4 × 4 rotation over Q(√5). Poses are interned in a pose table; a pose in K⁺ (7,200 elements) is stored by its index, any other by its exact matrix. Equality of states is exact pose equality, with a canonical digest.
  - **Twists.** A grip (one of 600 poles) and an exact rotation fixing it. The exact domain and the bounded input map of A1, and the signed menu of A4. A rotation without an exact representation is rejected, never approximated.
  - **Admissibility.** Exact inside/outside/straddle classification against the cut hyperplane, with certificates: a straddling piece is shown by two exact points on opposite sides. Uncertain means rejected. A commit is all-or-nothing.
  - **Checkpoints.** Handoff to the retained solving workflow only from a witnessed retained-state checkpoint (contract section 5). Reaching the lattice is not enough: J3 found eight lattice states, reached by three-twist words, that no single retained twist gives and that have no witness yet.
- Observed scale (J1 replays, `research/jumbling/fixtures/`), at the end of each scripted scramble:

  | Fixture (menu) | Twists | Pieces off the lattice | Pose-table entries (in K⁺) | Largest entry height | Blocked grips | Replay with references | End survey |
  |---|---:|---:|---:|---:|---:|---:|---:|
  | `wj-S4` (S4₀) | 640 | 117,498 of 177,120 | 1,591 (386) | 32 | 578 of 600 | 380 s | 61 s |
  | `wj-I_a` (I_a) | 841 | 121,184 | 1,333 (268) | 16 | 573 | 392 s | 40 s |
  | `wj-I_b` (I_b) | 885 | 128,013 | 1,248 (242) | 16 | 563 | 450 s | 34 s |

  Heights are the largest |numerator| or denominator of an entry of a non-K⁺ pose, in the form (a + b√5)/d. Times are cloud timings of the single-threaded Python reference while five other jobs shared a four-core container; they include building the references and are not performance evidence. In the random walks that produced these scripts, 93–96% of attempted twists were blocked once the state was scrambled. Whether heights stay bounded is open (J3 item 3); the leads do not decide it.
- Reproduce: `python research/jumbling/fixtures/wj.py check` replays every fixture with J1 and compares every reference; `python research/jumbling/sim/accept.py` reruns the J1 acceptance.

## 3. Environment and versions
- Base: `main` with `research/jumbling/` merged. The J1 reference needs Python 3 and NumPy only.
- J1 is accepted: an Astra review in two shards and its scoped verification (`research/jumbling/sim/README.md`). Contract revision `state-contract 2026-10-09 A1-A4`.
- Evidence kind: source and fixture. Headless runs on Linux are valid for the platform-independent layers (briefing section 2); nothing here is Windows, Direct3D or performance evidence.

## 4. Necessary source and evidence
- **Contract:** `research/jumbling/state-contract.md`, sections 1–5 and amendments A1–A4 (section 7).
- **Reference engine (J1):** `research/jumbling/sim/` and its README. The parts the product engine must match:
  - `state.py`: poses, the canonical pose table, `classify`, `apply`, `survey`, journal, `undo`, `replay`, `digest`, `checkpoint`, `export_retained`, `lattice_stickers` (A3);
  - `twists.py`: retained, plane, Cayley and half-turn twists; the input map of A1 with its proof; `TwistMenu` (A4) with its identity;
  - `kernel.py`: the exact integer sign kernel and the A2 filter with its error bound;
  - `regions.py`: one exact region per K⁺-orbit, transported by K⁺;
  - `model.py`: model identity (SHA-256 of the asset bytes as read) and `CONTRACT_REVISION`.
- **Fixtures:** `research/jumbling/fixtures/wj-<menu>.json` (menus S4, I_a, I_b): exact menu record, journal with identities, digests and array hashes at start, mid and end, the swept twist and the end survey with certificates. `research/jumbling/sim/acceptance.json` holds further replayable journals (the witness, seeded mixed sequences, the negative control).
- **Theory used by the engine:** `research/jumbling/theory/theory-draft.md` (draft; one Astra verification is pending):
  - Proposition 1.2: the inverse of an admissible twist is admissible with the same inside set, so undo is always legal;
  - Theorem 2.1: after one twist from a lattice configuration, the blocked grips follow from the cap polytope alone, which gives an exact fast path for that common case;
  - Proposition 2.2: consecutive twists of one grip compose, which the journal witness uses.
- **Layer rules:** `docs/progress/1.0/command-table.md` section 1 (layers, typed errors, evidence states); `docs/progress/1.0/requirements-from-screening.md` (R-numbers); the oracle specification `docs/progress/1.0/oracle/README.md` for the case and trace style.
- **What each layer gets** (from the plan, section 3b):
  - *Model identity.* One per signed menu, separate from 600-cell-Full. Both puzzles share the retained geometry (cut depth α = 121/125, pieces, regions, stickers, the 1,200 generators), read from `assets/` unchanged. A geometry change gives both puzzles new identities and re-runs every acceptance.
  - *Engine.* The state, twists, admissibility, regions, survey, swept-motion preview, undo, replay and checkpoints above. A2 is an optional accelerator whose result never differs from the exact one. J1 is the differential oracle.
  - *Session store.* A versioned format for pose states, the pose table and the jumbling journal, with exact numbers stored losslessly (integer triples, no floats), transactional writes and recovery as for 600-cell-Full. Jumbling sessions are separate from 600-cell-Full sessions. Path B migration (the 0.4 importer) is unchanged: 0.4 has no jumbling state.
  - *Command layer.* New rows: jumble twist with axis and angle input (through the A1 input map, showing the realised exact rotation), menu twist, grip survey, blocked-grip and certificate inspection, preview scrubbing, undo, checkpoint, and handoff to the retained solver. The human-solve boundary applies unchanged: the program never chooses or executes a twist by itself, and a handoff needs a witnessed checkpoint. Pointing targets for H-05: grips, pieces and certificate points.
  - *View model.* Per-piece pose index, pose table and lattice flags with revision numbers; the admissible and blocked grip sets; certificate points. It hands them to every drawing in the render data contract (`research/jumbling/render-contract.md`), which E-2.4-0J section 4 reads. The contract does not fix the dimension; 1.0 implements d = 4 only (owner decision 6, 9 October 2026).
- **Arithmetic for the engine-language spike.** J1 uses Python integers, which never overflow. A product engine must use checked fixed-width arithmetic with an exact big-integer fallback, or big integers throughout: no bound on entry heights is proved for the candidate menus, and under a menu that contains the negative control they are unbounded (its product has infinite order, and only finitely many matrices over Q(√5) have entries of bounded height).

## 5. Attempts so far
| # | Step | Result |
|---|---|---|
| 1 | Contract and witness (`research/jumbling/state-contract.md`, `witness.py`) | plan-checked; exact witness E0–E4 reviewed |
| 2 | J1 reference engine | accepted after an Astra review and scoped verification; 38 focused tests and 32 acceptance flags pass |
| 3 | J2 viewer and J4 explorer | research prototypes; they read J1 output and decide nothing |
| 4 | W-J fixtures (`research/jumbling/fixtures/wj.py`) | ENG_CHECK |
| — | Product engine, store or command rows | not started |

## 6. Constraints and owned files
- Read only: `assets/`, `research/jumbling/` (the reference), every critical path (`core.py`, `session.py`, `session_lock.py`, `log_io.py`, `engine_process.py`, `server.py`), `tools/tastelab/`.
- Owned files: `docs/progress/1.0/jumbling-interfaces.md`; the harness under `research/jumbling/differential/` (new), with tests under `tests/test_jumbling_*.py`. Command rows enter `docs/progress/1.0/command-table.json` only through `build_command_table.py` at the H-08 update.
- Platform rule (briefing section 2): the engine, session store, command layer and view model stay free of Windows-only code; their checks run headless on Linux.
- Fresh isolated data only; no personal session, no user database.
- Review: the interface proposal is a contract and design change, so it gets an Astra plan check and an Astra review of the finished candidate, run as at least two concurrent shards if it touches a critical path. The harness and store prototype get a routine review. The language choice stays with the owner.
- Re-acceptance: a change to geometry, a menu or the contract re-runs the differential evidence for every affected fixture.

## 7. Required return format
- `docs/progress/1.0/jumbling-interfaces.md` with the four layer sections, the proposed command rows and an open-questions list.
- The differential harness with its README, the result of each fixture and negative case, and the first differing step for any mismatch.
- The store round-trip result per fixture.
- The cost-lead table with how it was measured.
- Review findings on any candidate follow `schemas/review-result.schema.json`.
