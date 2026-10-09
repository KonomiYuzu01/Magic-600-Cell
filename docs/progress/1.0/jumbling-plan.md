# Jumbling puzzle and Jumble feature: program plan

Status: **revised after the Astra plan check of 9 October 2026 (call `20261009T201801Z-28fb9085`; all eight findings adopted, section 6). Its scoped re-check (`20261009T202958Z-6200127b`) found all eight resolved and raised one new finding, JR1, adopted in section 6; the owner then added the engineering packet (section 3b). The second scoped re-check (`20261009T203802Z-cb46d0be`) passed with no findings, and amendments A1–A4 are now in the state contract (section 7).** An earlier run of the same packet on the owner's machine (`20261009T185506Z-1a2654f2`) raised the same areas, but its text was not transcribed. This plan implements the [owner decisions of 9 October 2026](../../wiki/decisions/owner-decisions-2026-10-09-jumbling.md): rigid jumbling enters 1.0 as an independent puzzle and a Jumble feature. Nothing here changes 600-cell-Full, its model identity or its contract.

Inputs:
- the jumbling study (`research/jumbling/README.md`);
- the plan-checked state contract (`research/jumbling/state-contract.md`);
- the exact witness (`research/jumbling/witness.py`, `witness-results.json`);
- the retained theory (`research/theory/RETHLAS_BLUEPRINT.md`);
- the solving workflow (`solving-workflow.md`);
- the renderer plan (`renderer-experiment-plan.md`) and its packets (`packets/renderer/`).

Evidence kind for everything this plan produces in a cloud session: source, fixture and synthetic geometry. No result here is Windows, Direct3D 12, input or performance evidence.

## 1. Workstreams

### J1 General simulator (reference engine)

Code: `research/jumbling/sim/`. It is the exact reference for the jumbling puzzle, the role the differential oracle plays for 600-cell-Full. Python and NumPy only.

- **State.** Every one of the 177,120 pieces has a pose: an exact 4 × 4 rotation over Q(√5) (contract section 2). State equality is exact pose equality.
- **Regions.** Each region is computed exactly once per K⁺-orbit representative by double description, and certified against all 1,200 constraints. It is transported to the other pieces of the orbit by exact K⁺ rotations, lazily and cached.
- **Twists.**
  - Retained twists, and the exact domain and input map of amendment A1 (section 4).
  - Menus follow amendment A4: finite, inverse-closed and A4-conjugation-invariant, transported by K⁺ conjugation.
- **Admissibility.** Exactly section 3 of the contract, with the optional filtered sign test of amendment A2.
- **Survey.** The status of all 600 grips, with certificates.
- **Journal.** Exact twist records, replay, undo and the checkpoint test of contract section 5. Handoff to the retained solver only from a witnessed retained-state checkpoint.
- **Labelled projection.** The projection of amendment A3 from lattice pose states to retained labelled states.
- **Acceptance** (contract section 6, extended per finding J-Q5):
  1. (signature, host facet) transport and coherent-frame agreement with `rotperms`, `move_src`, `move_dst` and the retained primitives for all 1,200 generators, unchanged stickers included. The projection of amendment A3 must be a bijection onto the 259,800 labels and must agree with the retained primitive action, centres included.
  2. Independently certified blocked and unblocked cases: the witness E2 and E3 sets, and an independent recomputation of every grip status by fresh double description.
  3. The E0 controls (shallow crossings and exact contact), and conservative handling of uncertain contacts.
  4. Rejection without state change.
  5. Filtered and grouped classification compared with the unfiltered per-piece exact classifier.
  6. Journal replay from records, inverse round trips and undo, each tested separately on seeded random sequences that mix retained and jumble twists.
  7. Checkpoint controls:
     - a same-cap excursion (c, g), then (c, a·g⁻¹), which returns to a non-identity retained pose and is a checkpoint;
     - a configuration rotated globally by a K⁺ element outside G, which is a lattice configuration but not a retained checkpoint, because its centres are moved.
  8. Menu controls: inverse closure and all 12 A4 conjugations checked exactly, and transport by t and by t·a compared.
  9. The finite-order negative control of finding J-Q7-SCOPE: the half-turn g = 2P₀,₁₃ − I fixing n₀ and n₁₃ is not in A4₀, and tr(H₀ g) = 4/3 is not an algebraic integer. So H₀g has infinite order and repeated admissible twists of cap 0 reach infinitely many configurations.
- **Review.** J1 changes behaviour against a contract. Its finished candidate gets an Astra review (AGENTS.md "Team protocol"), not a routine one.

### J2 Rendering and observation prototype

Code: `research/jumbling/viewer/`. This is a research prototype, not a 1.0 renderer candidate. It exists to find out what a player must see, and to feed the design track and the renderer packet.

- **Mesh export.** A sticker is the 3-polytope where a piece region meets one of its host facets. The export turns exact regions into float meshes, groups them by piece, and adds the per-piece pose.
- **Viewer.** A WebGL page in two views, Global and Local, visible together and linked (owner design decision of 2 October 2026). It shows:
  - per-piece four-dimensional transforms, projected to three dimensions;
  - twist animation along the exact one-parameter family;
  - admissible and blocked grips, with the straddling piece and its two certificate points;
  - off-lattice pieces;
  - realignment cues.
- **Data.** Sequences come from J1, with menu identity and contract revision recorded: the witness first, then scripted scrambles. Any live preview is labelled as an uncertified float preview. Only J1 decides legality.
- **Review.** The owner reviews it as a private page. Look, colour and motion values are provisional and belong to the design track (G1–G6 stay with the owner).

### J3 Solving theory (Astra)

Problem statement: `research/jumbling/theory-problem.md`, revised per findings J-Q3, J-Q4, J-Q7 and J-Q7-SCOPE. Two routes:
- **Owner's machine.** Astra generates and verifies the theory, as for `research/theory/`.
- **This session.** The integrator drafts the theory with exact J1 computations, and Astra reviews the draft through the wrapper. The two are separate invocations.

- **Scope.**
  - The lattice set L is defined by occupation of the retained surface chambers, so the solved configuration belongs to L.
  - The groupoid of admissible twists for a menu as in amendment A4.
  - The finiteness question, starting from the exact negative control.
  - Membership in the retained group through the labelled projection of amendment A3. This is distinct from exact pose restoration.
  - A constructive or complexity result for return to the lattice, beyond reversing the scramble.
  - Orbit-first block building at witnessed checkpoints.
  - Protection by poses.
- **Evidence.** J1 and J4 supply exact computations, as leads for Astra, not as proofs.
- **Human-solve boundary.** It is unchanged: the theory may describe methods, but the program never chooses or executes them.

### J4 Four-dimensional grip-orbit explorer

Code: `research/jumbling/explorer/`. It is written here from the published formal definition of grip theory. Jambler has no licence file, so none of its code is copied.

- **Orbits.** Grip orbits on S³ for exact menus that are inverse-closed and A4-invariant, with BFS depth, a growth curve and exact checks of sampled coincidences. Every result records its menu identity.
- **View.** Stereographic projection to three dimensions in a WebGL page.
- **Status.** Its interaction ball is an approximation. Its results are leads for J3 and for the owner's menu decision, not proofs.

## 2. Order and mid-stage

1. **Start.** J1, J2 and J4 run in parallel, as the owner authorized; J3's problem statement is revised. Acceptance runs and J3 work that depend on amendments A1–A4 use the contract revision that passes the re-check.
2. **Mid-stage.** It is reached when an acceptance table, audited at the milestone, links each deliverable to:
   - its contract revision;
   - its exact menu identity;
   - its replayable fixture;
   - its result.

   The table's rows:
   - **J1:** acceptance items 1–9 pass, and its Astra review is valid.
   - **J2:** shows certified J1 sequences, with blocked grips and certificates.
   - **J4:** orbit data for the candidate menus.
   - **J3:** lists which conclusions are verified, which are conjectural and which are open, after one Astra verification. Running an invocation is not by itself an acceptance result.

   The table is [jumbling-midstage.md](jumbling-midstage.md).

   Then the integrator writes the two mid-stage packets and gives them to the owner (owner, 9 October 2026: the jumbling work affects both the renderer line and the engineering line):
   - the renderer packet (section 3);
   - the engineering packet for the backend boundaries (section 3b).
3. **After mid-stage.** The owner signs off the twist menu, which defines the puzzle. The design track adds the jumbling states (H-04 layout, H-05 pointing, the command table). Engine and command-layer interfaces enter the 2.5 freeze.

A likely miss of the stage 2 schedule is reported once, with a re-plan, under the schedule rules. No duration is estimated here.

## 3. Renderer packet (outline; written at mid-stage)

File: `packets/renderer/E-2.4-0J-jumbling.md`, in the seven-part format. Planned content:

- **Geometry.** Piece and sticker geometry are those of 600-cell-Full: the 259,800 slots, the 433 sticker shapes per cell and the base meshes.
- **What changes for the drawing method.**
  - Off-lattice pieces leave their slots, so the label-buffer method alone is not enough.
  - The renderer needs a per-piece transform buffer of up to 177,120 rigid four-dimensional transforms, with projection and shrink anchored in each piece's home frame.
  - It also needs a per-piece lattice flag.
  - The outer shape is no longer the polytope.
  - Retained turns also pass through off-slot poses while they animate.
- **New overlays.**
  - Admissible and blocked grips.
  - The straddling pieces with their two certificate points. Certificate points can lie on no sticker, so they come from the engine's certificate data, not from the meshes.
  - The twist-angle input during preview.
- **Correctness evidence for W-J.**
  - First-use verification that the bound pose, label and lattice-flag revisions match the engine's revision, with atomic adoption.
  - Negative tests with stale or mismatched pose data, and delayed-adoption injections. A label-only readback cannot detect a stale transform.
  - Replayable fixtures for each menu, with start, midpoint and end geometry references from an independent reference.
- **Workload W-J.** Full detail with the off-lattice piece count observed in a scripted, replayable scramble of the chosen menu, plus a continuous jumble-twist animation. This is an observed workload description, not a proved worst case. Gate thresholds are those of renderer-candidates.
- **Re-acceptance.**
  - Earlier gate evidence holds for its matching build only.
  - When shared rendering paths change (shaders, uploads, bindings, projection, shrink), the affected lattice workloads are run again.
  - The jumbling mode needs its own W-J gate runs.
  - A change to geometry (cut depth, piece or sticker meshes) re-runs the whole acceptance, as the owner said.
- **Data contract.** Pose format (4 × 4 float or a unit-quaternion pair) with revision numbers, its update and adoption rule, and the engine side as the source of truth. Rendering never decides legality.

## 3b. Engineering packet (outline; written at mid-stage)

File: `packets/engineering/ENG-0J-jumbling-backend.md`, in the seven-part format. It covers the backend boundaries of charter section 4 (engine, session store, command layer, view model) for the jumbling puzzle. It feeds the engine-language question (`docs/wiki/questions/engine-language-rust.md`) and the 2.5 interface freeze; it starts no implementation. Planned content:

- **Model identity.** The jumbling puzzle has its own model identity per signed menu, separate from 600-cell-Full. It shares the retained geometry (cut depth α, pieces, regions, stickers, the 1,200 generators), read from `assets/` without change. A geometry change gives a new identity for both puzzles and re-runs every acceptance.
- **Engine.**
  - State: an exact pose per piece over Q(√5), a pose table, lattice flags, and canonical digests (contract section 2).
  - Twists: the exact domain and input map of A1 and the signed menu of A4. A non-representable rotation is rejected, never approximated.
  - Admissibility: exact classification with certificates (contract section 3), and the filtered sign test of A2 as an optional accelerator whose result never differs from the exact one.
  - Regions: one exact region per K⁺-orbit, transported by K⁺.
  - Survey, preview of the swept motion, undo, replay and checkpoints (contract sections 4 and 5).
  - J1 is the differential oracle: the product engine must match it on replayable fixtures for each menu, as the 0.4 oracle does for 600-cell-Full.
- **Exact arithmetic.** The observed entry heights and denominators of the poses in scripted scrambles, and the cost of exact evaluation against the filtered test. These are inputs to the engine-language question, measured in Linux cloud sessions with the Python reference. They are not performance evidence for the product engine.
- **Session store.**
  - A versioned format for pose states, the pose table and the jumbling journal, with exact numbers stored losslessly.
  - Transactional writes and recovery as for 600-cell-Full.
  - Jumbling sessions are separate from 600-cell-Full sessions. Path B migration (the 0.4 importer) is unchanged, because 0.4 has no jumbling state.
- **Command layer.**
  - New command-table rows: jumble twist with angle and axis input, grip survey, blocked-grip and certificate inspection, preview scrubbing, checkpoint and handoff to the retained solver.
  - The human-solve boundary applies unchanged: the program never chooses or executes a twist by itself, and a handoff needs a witnessed checkpoint.
  - Pointing targets for H-05: grips, pieces and certificate points.
- **View model.** Per-piece transforms and lattice flags with revision numbers, the admissible and blocked grip sets, and the certificate points, as the source of truth for the renderer data contract of section 3.
- **Correctness evidence.** Differential runs against J1 on each fixture; negative tests for stale revisions, non-representable input, menu violations and the exact negative control of J1 acceptance item 9; replay determinism; and round trips through the session store.
- **Re-acceptance.** A change to geometry, a menu or the contract re-runs the differential evidence for every affected fixture.
- **Platform.** The engine, session store, command layer and view model stay free of Windows-only code (briefing section 2); their checks run headless.

## 4. Contract amendments (written into `research/jumbling/state-contract.md` section 7)

### A1 Exact rotation domain and input map
- **Domain.**
  - Exact rotations of SO(3)_c with entries in Q(√5). Every non-half-turn is a Cayley rotation R(ω) with ω ∈ Q(√5)³, in the frame u₁ = i·n_c, u₂ = j·n_c, u₃ = k·n_c (left quaternion multiplication, coordinates (−n₁, n₀, −n₃, n₂), (−n₂, n₃, n₀, −n₁), (−n₃, −n₂, n₁, n₀)). Its Gram matrix is ‖n_c‖² I, so no square root is needed.
  - Half-turns use an exact branch, H_u = 2(P_c + P_u) − I, with P_u = uuᵀ/‖u‖² for an exact axis u ⊥ n_c.
- **Not representable.** Rotations whose trace is not in Q(√5), such as 2π/7, are not representable and are rejected.
- **Input map.** A requested float axis and angle maps to the exact rotation that minimises the Frobenius distance to the requested rotation. The search ranges over:
  - Cayley parameters ω ∈ Q³ with |numerators| ≤ N and common denominator ≤ D;
  - half-turn axes with the same bounds, for angles within 1° of π.

  Ties go to the smaller denominator, then the lexicographically smaller numerators. The realised axis, angle and distance are reported. N, D and the distance are recorded in the journal, and replay uses the recorded exact rotation.
- **Menu elements.** They are exact and are never approximated. A request that is not a menu element is refused in menu mode.

### A2 Filtered sign test
- **Rule.** A float evaluation of h_A at an exact vertex, or at an exact superset vertex, decides the sign only when its absolute value exceeds a proven forward error bound. Otherwise exact Q(√5) evaluation decides. Non-finite values fall back to exact.
- **The bound** covers conversion of the exact inputs, pose evaluation, cancellation and rounding.
- **The certificate records:**
  - the exact inputs (pose and vertex identities);
  - the state and model identity;
  - the coverage proof (complete vertex set or a superset);
  - the arithmetic and error-bound version;
  - the computed enclosure;
  - the accepted sign or the exact fallback result.

### A3 Labelled projection
- **Sticker identity.** A sticker's identity is (A, f): the piece and its home host facet.
- **Projection.** In a configuration where every pose lies in K⁺, the sticker (A, f) with pose k = g_A sits in slot(k(A), k(f)), where k transports both the complete signature and the host facet. Its orientation is the A4₀ element F_{k(f)}⁻¹ k F_f given by the coherent transported frames.
- **π.** This defines the projection π from lattice pose states to retained labelled states.
- **π is not injective on poses.** A centre piece, or a piece with a non-trivial stabiliser, can change pose with equal labels; for example, generator 1 rotates centre piece 0 in place. Contract equality and checkpoints use exact pose equality; label equality is weaker. Protection is pose-based.

### A4 Menus and the puzzle J(Λ)
- **Menu.** A menu is a finite list Λ₀ ⊂ SO(3)₀ of exact rotations with:
  - A4₀ ⊆ Λ₀;
  - Λ₀ = Λ₀⁻¹;
  - aΛ₀a⁻¹ = Λ₀ for every a ∈ A4₀.
- **Transport.** Λ_c = tΛ₀t⁻¹ for any t ∈ K⁺ with t(n₀) = n_c. A4-invariance makes this independent of t.
- **Alphabets.** Three are distinguished:
  - a finite menu, which defines the puzzle J(Λ);
  - the infinite field-valued alphabet, the simulator's free mode under A1;
  - the continuous theory alphabet SO(3)_c.

  A finite angle set with free axes is not a finite menu.
- **Candidate menus for the owner** (each identified exactly, with its inverse and conjugacy closure):
  - A4₀, the control, which gives 600-cell-Full;
  - S4₀ = N(A4₀), which adds the 90° rotations and edge half-turns, if it is exactly representable;
  - the icosahedral groups containing A4₀, if they are exactly representable;
  - closures of the exact witness rotation and of reconstructed realignment rotations.
- **Same-cap finiteness.** For one cap, ⟨A4₀, Λ₀⟩ is finite exactly when it lies in a finite subgroup of SO(3) containing A4₀: A4, S4 or A5. Multi-cap reachability is the J3 question.

## 5. Owned files and boundaries

- **New files:**
  - `research/jumbling/sim/`, `viewer/`, `explorer/`, `theory/`, `fixtures/`, `theory-problem.md`;
  - `jumbling-midstage.md` and the packets under `packets/jumbling/`;
  - this plan;
  - `packets/renderer/E-2.4-0J-jumbling.md` and `packets/engineering/ENG-0J-jumbling-backend.md` (at mid-stage);
  - tests under `tests/test_jumbling_*.py`.
- **Read-only:**
  - `assets/model.npz`, `assets/manifest.json` and every critical path (`core.py` and the others);
  - `tools/tastelab/` and `docs/progress/1.0/taste-lab-plan.md`.
- **No new costs.** Subscription only, with no API team. Codex runs through the wrapper, on the owner's machine or in this session.
- **Reviews.**
  - The contract amendments get the Astra plan check and its scoped re-checks.
  - J1, which is behaviour against a contract, gets an Astra review of its finished candidate.
  - J2 and J4, research prototypes, get a routine review.
  - Fixes written by one Codex model are verified by the other.

## 6. Plan-check dispositions (call `20261009T201801Z-28fb9085`)

Finding Q2 holds: a filtered sign test with exact fallback keeps the containment rule. Its certificate fields are in A2.

| Finding | Severity | Reply | Change |
| --- | --- | --- | --- |
| J-Q1 | major | adopt | A1: exact domain with a half-turn branch; deterministic input map over bounded numerators and denominators; realised axis reported; menu elements never approximated; non-representable rotations rejected |
| J-Q4 | major | adopt | A4: menus inverse-closed and A4-conjugation-invariant, transported by conjugation; three alphabets distinguished; J1 acceptance item 8 |
| J-Q3 | major | adopt | A3: labelled projection π with coherent frames; pose equality kept distinct from label equality; J3 item 4 uses π |
| J-Q5 | major | adopt | J1 acceptance items 1 and 3–7: host-facet transport, frames and controls |
| J-Q6 | major | adopt | Section 3: conditional evidence reuse; pose and flag revision checks; stale-pose negative tests; replayable fixtures; W-J as an observed workload |
| J-Q7 | major | adopt | J3 and the contract: L defined by occupation of retained surface chambers; solved ∈ L as a control |
| J-Q7-SCOPE | minor | adopt | J1 acceptance item 9 and J3 item 3 use the exact negative control; J3 items 4 and 5 sharpened |
| J-Q8 | major | adopt | J1 review assigned to Astra; mid-stage defined by an audited acceptance table |

Scoped re-check `20261009T202958Z-6200127b`: the eight findings above are resolved. Second scoped re-check `20261009T203802Z-cb46d0be`: JR1 resolved; section 3b raises no finding.

| Finding | Severity | Reply | Change |
| --- | --- | --- | --- |
| JR1 | major | adopt | Theory problem item 3, grip closure: the A4 control stays within the 600 poles; the explorer's growth is reported per tested menu and depth only, not as unbounded; unbounded growth needs a separate argument |
