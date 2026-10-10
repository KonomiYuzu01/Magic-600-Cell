# Packet: extend the J1 simulator to the amended contract (implementation)

Run as an implementation with the default reviewer model, effort `max` and speed tier `fast`, in the worktree the wrapper creates. Change only the allowed files.

## 1. Goal and acceptance
- Goal: bring the J1 simulator (`research/jumbling/sim/`) up to J1 acceptance items 1–9 of `docs/progress/1.0/jumbling-plan.md` section 1 and to amendments A1–A4 of `research/jumbling/state-contract.md` section 7. The current acceptance run passes its 22 flags; the gaps below are what the amended contract adds.
- **G1 — item 1, labelled projection π (A3).**
  - For the solved state and after each of the 1,200 retained generators, π must be a bijection onto the 259,800 labelled slots: every slot receives exactly one sticker (A, f) and every sticker exactly one slot. Every orientation must lie in A4₀.
  - π must equal the retained primitive action (`assets/primitives.npz`), with the stickers of centre pieces counted explicitly. A centre piece changes pose under its own cap's generator while its labels stay; report how many centre stickers were checked.
  - Record the result as a new acceptance flag.
- **G2 — item 5, grouped against per-piece classification.**
  - Add a reference classifier that evaluates every off-lattice piece individually by exact Q(√5) arithmetic, with no group superset, no anchor caps and no filter.
  - On the E2 and E3 states, compare its status and inside set with `State.classify` for all 600 grips. Do the same on two seeded random states mixing retained and jumble twists, for at least 60 grips each, chosen by a fixed seed.
  - Every status and every inside set of an admissible grip must agree.
- **G3 — item 7, global K⁺ rotation control.**
  - Add a way to build the configuration in which every piece has the same pose k ∈ K⁺, for example `State.from_global_rotation(ctx, kidx)`. Use it for two elements: one in A4₀ other than the identity, and one that moves pole 0.
  - Each must be a lattice configuration whose digest differs from solved. It must not be a retained checkpoint (its centres are moved), `export_retained` must refuse it, and a supplied retained word must not make it a checkpoint.
  - Add the existing same-cap excursion (c, g), (c, a·g⁻¹) to the item 7 results under that name. It returns to a non-identity retained pose and is a checkpoint.
- **G4 — item 8, menu controls (A4).**
  - `TwistMenu` must check inverse closure exactly. With `close=True` it adds the missing inverses and the A4₀ elements; with `close=False` (as in `from_record`) it refuses a list that is not inverse-closed, not A4₀-conjugation-invariant, or does not contain A4₀.
  - The menu record states `inverse_closed`, `a4_invariant` and `contains_a4`, plus a SHA-256 menu identity over the canonical exact matrices.
  - Acceptance: for every cap c and every a ∈ A4₀, transport by t = F_c and by t·a gives the same twist set. Check this exactly for the A4 menu, an S4₀ menu and the negative-control menu of G5, over all 600 caps.
  - S4₀ is A4₀ with the 90° Cayley rotations ω = (±1, 0, 0), (0, ±1, 0), (0, 0, ±1) and their products: 24 elements, all exact in Q(√5) in the cap frame of A1.
- **G5 — item 9 and A1.**
  - **Half-turn branch.** Add exact half-turns H_u = 2(P_c + P_u) − I for an exact axis u ⊥ n_c, given in cap-frame coordinates over Q(√5). Use a new journal family `half_turn` with its parameters. It is its own inverse, and replay and `from_record` must rebuild it exactly.
  - **Input map.** Rewrite the float axis-and-angle input map to amendment A1:
    - It minimises the Frobenius distance to the requested rotation over Cayley parameters ω ∈ Q³ with |numerators| ≤ N and a common denominator ≤ D, and, for requested angles within 1° of 180°, over half-turn axes with the same bounds.
    - Ties go to the smaller denominator, then to the lexicographically smaller numerators.
    - The result is the exact minimum over that whole search space. A pruning argument that proves no other candidate can be closer is allowed; explain it in the README.
    - Float distances are allowed, because the input map is not a legality decision; candidates within 1e-12 of the best are tied.
    - Report the realised axis, angle and distance. The journal records N, D and the distance, and replay uses the recorded exact rotation.
    - A non-representable rotation is rejected, and a menu element is never approximated.
  - **Negative control (item 9).**
    - Let g = 2P₀,₁₃ − I, the half-turn fixing n₀ and n₁₃. Build it through the half-turn branch, with u the component of n₁₃ orthogonal to n₀.
    - Check exactly: g ∉ A4₀, g² = I, and the four-dimensional trace tr(H₀ g) = 4/3, where H₀ is retained generator 0. A rotation of finite order has an algebraic-integer trace, so H₀g has infinite order.
    - Menu NC is A4₀ ∪ {g}, closed as in G4. From solved, apply (0, H₀) and (0, g) alternately for at least 12 rounds. Every twist must be admissible, all digests must be pairwise distinct, and the maximum entry height of the cap-0 pose (largest |a|, |b| or d of (a + b√5)/d) must grow. Record these as item 9.
- **G6 — A2 certificate fields.** In filtered mode, `classify(..., record=True)` returns, for every float-decided evaluation:
  - the exact inputs: pose key and piece or anchor, vertex identities;
  - the state digest and model identity;
  - the coverage: complete vertex set, or a superset with its constraint subset;
  - the arithmetic and error-bound version;
  - the enclosure (lo, hi);
  - the accepted sign.

  It records exact fallbacks as such. Acceptance: on one E2 grip and one E3 grip, every recorded float decision has an enclosure that excludes zero and agrees with the exact sign. Filtering stays off by default.
- **G7 — identity.** The journal document also carries the model identity (the SHA-256 of `assets/model.npz` and `assets/primitives.npz` as read), the menu identity when a menu is set, and the contract revision string `state-contract 2026-10-09 A1-A4`. Replay refuses a journal whose identities differ. Journals without the field still replay.
- **Acceptance.**
  - The acceptance check below passes inside the sandbox.
  - `python research/jumbling/sim/accept.py` writes `acceptance.json`. Every earlier flag still passes, and there are new flags for G1–G7 named after the plan's items (`1_projection_bijection_and_centres`, `5_grouped_equals_per_piece`, `7_global_rotation_control`, `7_same_cap_excursion`, `8_menu_controls`, `9_negative_control`, `A1_input_map_and_half_turns`, `A2_certificate_fields`, `identity_in_journal`). The integrator runs this outside the sandbox if it does not finish inside it.
  - Tests for G1–G7 are added to `tests/test_jumbling_sim.py`. The whole file must stay under about 60 s.
- Out of scope:
  - the viewer, the explorer and `witness.py`;
  - the theory (J3);
  - performance;
  - any change to `assets/`, to `exact.py` or to a critical path.

## 2. Actual problem and reproduction
- `python research/jumbling/sim/accept.py` (about 5 minutes with 4 workers) passes its 22 flags, but the plan's items 1, 5, 7, 8 and 9 and amendments A1 and A2 are not fully covered:
  - `TwistMenu` does not check inverse closure;
  - there is no half-turn branch, and `cayley_axis_angle` rounds per denominator instead of minimising over the stated space;
  - item 7 has no global rotation control;
  - item 9 is missing;
  - the filtered certificate fields of A2 are not recorded.

## 3. Environment and versions
- Branch `claude/jumbling-1-0`, committed head. Linux, Python 3.11 or later, NumPy. No network is needed.

## 4. Necessary source and evidence
- `research/jumbling/state-contract.md`, sections 2, 3 and 5 and amendments A1–A4 in section 7.
- `docs/progress/1.0/jumbling-plan.md` section 1, J1 acceptance items 1–9.
- `research/jumbling/sim/`:
  - `state.py`: `classify`, `apply`, `checkpoint`, `export_retained`, `lattice_stickers`, journal and replay;
  - `twists.py`: `cayley_matrix`, `nearest_parameter`, `cayley_axis_angle`, `Twist.record` and `from_record`, `TwistMenu`;
  - `kernel.py`: `Evaluator` and `filtered_signs`;
  - `accept.py`: its sections and `summarise`;
  - `README.md`.
- `tests/test_jumbling_sim.py`.
- Exact fact checked by the integrator: with H₀ = retained generator 0, the four-dimensional trace of H₀·(2P₀,₁₃ − I) is 4/3.

## 5. Attempts so far
| # | Step | Result |
|---|---|---|
| 1 | J1 first candidate (merged as `58edec2`) | 22 acceptance flags pass |
| 2 | Plan check and two scoped re-checks of the amended plan | passed; amendments A1–A4 written into the contract (`2f23e22`) |

## 6. Constraints and owned files
- Change only the allowed files below. Do not commit; the wrapper collects the patch. Keep exactness: no floating-point decision may affect admissibility, equality or checkpoints.

```implement-contract
{"allowed_files": ["research/jumbling/sim/__init__.py", "research/jumbling/sim/state.py", "research/jumbling/sim/twists.py", "research/jumbling/sim/kernel.py", "research/jumbling/sim/kplus.py", "research/jumbling/sim/regions.py", "research/jumbling/sim/model.py", "research/jumbling/sim/accept.py", "research/jumbling/sim/acceptance.json", "research/jumbling/sim/README.md", "tests/test_jumbling_sim.py"], "acceptance_check": ["python", "tests/test_jumbling_sim.py"], "stop_condition": "G1-G7 are implemented in the allowed files, the new tests pass and the acceptance check passes"}
```

## 7. Required return format
- Changes only in the assigned worktree. The final message lists:
  - the changed files;
  - per gap G1–G7, what changed and which test or acceptance flag covers it;
  - the acceptance result, and whether `accept.py` was run inside the sandbox;
  - open points.
