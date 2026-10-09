# Jumbling puzzle and Jumble feature: program plan

Status: **draft for the Astra plan check (9 October 2026).** It implements the [owner decisions of 9 October 2026](../../wiki/decisions/owner-decisions-2026-10-09-jumbling.md): rigid jumbling enters 1.0 as an independent puzzle and a Jumble feature. Nothing here changes 600-cell-Full, its model identity or its contract.

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

- **State.** Every one of the 177,120 pieces has a pose: an exact 4 × 4 rotation over Q(√5), and a lattice flag (state contract section 2).
  - Lattice pieces are classified by their transported signature.
  - Off-lattice pieces are classified by their exact region (contract section 3).
- **Regions.** Each region is computed exactly once per K⁺-orbit representative by double description, and transported to the other pieces of the orbit by exact K⁺ rotations.
  - K⁺-orbits are unions of the 35 + 1 G-orbits, because every retained twist lies in K⁺.
  - Regions are built lazily and cached, since only off-lattice pieces need them.
- **Twists.**
  - Retained twists: the 1,200 generators and every A4 element.
  - Jumble twists about pole c:
    - the plane rotations of the witness (fixing n_c and a second pole);
    - general rotations of SO(3)_c by the Cayley map, with parameter ω ∈ Q(√5)³ in the exact frame {n_c i, n_c j, n_c k} of n_c^⊥.
  - Every rotation is exact. The Cayley map covers every rotation except half-turns. A requested axis and angle maps to the nearest parameter with a bounded denominator, and the realised angle is reported.
- **Admissibility.** Exactly section 3 of the contract: certificates, with uncertain treated as rejected, and rejection without any state change.
- **Survey.** Gives the status of all 600 grips with certificates. It prunes with exact pose-group supersets, as `classify_grouped` does.
- **Journal.** Records exact twist records, replay, undo by exact inverse and the checkpoint test of contract section 5. Handoff to the retained solver happens only from a witnessed retained-state checkpoint.
- **Acceptance:** the "later simulator" criteria of contract section 6, for all 1,200 generators:
  1. labelled sticker and frame agreement with `rotperms`, `move_src` and `move_dst`;
  2. independently certified blocked and unblocked cases, at least the witness E2 and E3 sets;
  3. conservative handling of uncertain contacts;
  4. rejection without state change;
  5. exact round trips of random admissible sequences that include jumble twists.

### J2 Rendering and observation prototype

Code: `research/jumbling/viewer/`. This is a research prototype, not a 1.0 renderer candidate. It exists to find out what a player must see, and to feed the design track and the renderer packet.

- **Mesh export.** A sticker is the 3-polytope where a piece region meets one of its host facets (contract section 2). The export turns exact regions into float meshes, groups them by piece, and adds the per-piece pose.
- **Viewer.** A WebGL page in two views, Global and Local, visible together and linked (owner design decision of 2 October 2026). It shows:
  - per-piece four-dimensional transforms, projected to three dimensions;
  - twist animation along the exact one-parameter family;
  - admissible and blocked grips, with the straddling piece and its two certificate points;
  - off-lattice pieces;
  - realignment cues.
- **Data.** Sequences come from J1 (the witness first, then scripted scrambles). Any live in-browser preview is labelled as an uncertified float preview. Only J1 decides legality.
- **Review.** The owner reviews it as a private page. Look, colour and motion values are provisional and belong to the design track (G1–G6 stay with the owner).

### J3 Solving theory (Astra)

Problem statement: `research/jumbling/theory-problem.md`. It is produced and verified by Astra, in the same way as `research/theory/` (generation and verification as separate invocations, run by the owner).

- **Scope.**
  - The state space and its groupoid of admissible twists.
  - Lattice states and handoff.
  - Invariants and obstructions to returning to a lattice state.
  - Unjumbling methods a human can carry out.
  - How orbit-first block building extends to jumbled scrambles.
  - Protection (Net and Strict) under jumbling.
  - Whether a finite twist menu gives a finite reachable set.
- **Evidence.** J1 and J4 supply exact computations, as leads for Astra, not as proofs.
- **Human-solve boundary.** It is unchanged: the theory may describe methods, but the program never chooses or executes them.

### J4 Four-dimensional grip-orbit explorer

Code: `research/jumbling/explorer/`. It is written here from the published formal definition of grip theory. Jambler has no licence file, so none of its code is copied.

- **Orbits.** It computes grip orbits on S³ under jumble twists of the 600 cell caps, for a chosen twist menu:
  - BFS depth;
  - growth curve;
  - near-coincidences, with an exact check where the menu is exact.
- **View.** Stereographic projection to three dimensions in a WebGL page, coloured by depth, with the 600 poles as reference.
- **Questions it answers:**
  - Which menus have finite grip orbits?
  - Where do the orbits become dense?
  - Which realignment classes from the study keep some grips usable?

It feeds the J3 finiteness question and the owner's twist-menu decision.

## 2. Order and mid-stage

1. **Start.** J1 core, J2 mesh export with the witness sequence, and J4 run in parallel; the J3 problem statement is written. The Astra plan check of this plan and the contract amendments (section 4) runs on the owner's machine.
2. **Mid-stage.** It is reached when all of the following hold:
   - J1 passes its acceptance (items 1–5);
   - J2 shows certified sequences from J1, with blocked grips and certificates;
   - J4 has orbit data for the candidate twist menus;
   - J3 has one Astra generation and verification pass.

   Then the integrator writes the renderer packet (section 3) and gives it to the owner.
3. **After mid-stage.** The owner signs off the twist menu, which defines the puzzle. The design track adds the jumbling states (H-04 layout, H-05 pointing, the command table). Engine and command-layer interfaces enter the 2.5 freeze.

A likely miss of the stage 2 schedule is reported once, with a re-plan, under the schedule rules. No duration is estimated here.

## 3. Renderer packet (outline; written at mid-stage)

File: `packets/renderer/E-2.4-0J-jumbling.md`, in the seven-part format. Planned content:

- **What does not change.** Piece and sticker geometry are those of 600-cell-Full: the 259,800 slots, the 433 sticker shapes per cell and the base meshes. Lattice states draw exactly as today.
- **What changes for the drawing method.** Off-lattice pieces leave their slots permanently, so the label-buffer method (fixed slots, colour by label) is not enough for them.
  - The renderer needs a per-piece transform buffer: up to 177,120 rigid four-dimensional transforms, with the 0.4 projection and shrink applied after the transform.
  - It also needs a per-piece lattice flag.
  - The outer shape is no longer the polytope.
- **New overlays.** Admissible and blocked grips; the straddling piece with its two certificate points; the twist-angle input during preview.
- **Workload W-J.** Full detail with the largest off-lattice piece count the menu reaches in a scripted scramble, plus a continuous jumble-twist animation. Gate thresholds are those of renderer-candidates.
- **Re-acceptance.**
  - Gate evidence for lattice states stays valid.
  - The jumbling mode needs its own W-J gate runs, because the drawing method changes.
  - A change to geometry (cut depth, piece or sticker meshes) re-runs the whole acceptance, as the owner said.
- **Data contract.** Pose format (4 × 4 float or a unit-quaternion pair), its update and adoption rule, and the engine side as the source of truth. Rendering never decides legality.

## 4. Contract amendments for the plan check

Proposed changes to `research/jumbling/state-contract.md`:

| ID | Change | Why |
| --- | --- | --- |
| A1 | Admit Cayley-parametrised rotations of SO(3)_c with ω ∈ Q(√5)³, alongside the plane rotations of section 6. Specify the exact frame of n_c^⊥ and the input mapping from axis and angle. | A general simulator needs twists about any axis in the cap, not only planes through a second pole. |
| A2 | Allow a filtered sign test: a float evaluation of h_A at an exact vertex, with a proven forward error bound, decides the sign when the absolute value exceeds the bound; otherwise exact Q(√5) evaluation decides. The bound and its derivation are part of the certificate. | Survey and preview cost. The decision stays exact; this is the standard exact-geometric-computation filter. |
| A3 | Define labelled stickers and frames for jumbled states: a sticker is a (piece, host facet) pair carried by the piece pose, and on lattice states it must agree with the retained slot permutation and frames. | J1 acceptance item 1 and the renderer data contract. |
| A4 | Define the puzzle J(Λ) by a twist menu Λ per cap, conjugated by K⁺. Λ = A4 gives 600-cell-Full. Candidate menus are the realignment classes of the study, a finite angle set, and all exact rotations. The menu is chosen by the owner at the freeze. | The independent puzzle needs a definition before its model identity. |

## 5. Owned files and boundaries

- **New files:**
  - `research/jumbling/sim/`, `viewer/`, `explorer/`, `theory-problem.md`;
  - this plan;
  - `packets/renderer/E-2.4-0J-jumbling.md` (at mid-stage);
  - tests under `tests/test_jumbling_*.py`.
- **Read-only:**
  - `assets/model.npz`, `assets/manifest.json` and every critical path (`core.py` and the others);
  - `tools/tastelab/` and `docs/progress/1.0/taste-lab-plan.md`.
- **No new costs.** Subscription only, with no API team. Codex calls run through the wrapper on the owner's machine.
- **Process.**
  - The integrator writes and integrates; one review per finished candidate.
  - The contract amendments get the Astra plan check. The J1 code, as behaviour against a contract, gets a Sol review of its finished candidate.
  - The theory follows the `research/theory/` generation and verification route.
