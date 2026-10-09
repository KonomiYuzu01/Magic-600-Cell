# Initial impact assessment: adding rigid jumbling to the 1.0 program

Status: **initial assessment for the owner, plan step 3 (`README.md`), written after the exact witness.** This page proposes nothing binding. Adding jumbling to 1.0 in any form is an owner decision on scope and model identity (`AGENTS.md` "Ask the owner").

Inputs:
- the plan-checked state contract (`state-contract.md`);
- the exact witness (`witness-results.json`);
- the stage 2 charter (`docs/progress/1.0/charter-2.0-draft.md`);
- the schedule (`docs/progress/1.0/stage-2-experiment-protocol.md` section 5);
- the solving workflow (`docs/progress/1.0/solving-workflow.md`).

What the witness established:
- the retained geometry admits genuine rigid jumbling with certified legality;
- after a 10° jumble twist of cap 0, its face neighbour stays turnable while 54 other grips are blocked;
- after the neighbour's third-turn, cap 0 itself is blocked until the neighbour is undone.

The exact computation over 4,375 piece regions took about 3 minutes on four cloud cores. That shows exact certification is practical for research. It is not evidence about interactive performance.

No performance, Windows or DirectX evidence exists for any jumbling feature. Any statement about cost below is a structural observation, not a measurement.

## 1. What jumbling changes, layer by layer

| Area | 1.0 as chartered | With rigid jumbling |
| --- | --- | --- |
| Mathematical contract (charter section 2, binding in stage 2) | 600-cell-Full; 259,800 labelled slots; 1,200 legal generators; slot permutations with full collateral | A second puzzle: each piece has its own SO(4) pose, twists include rotations outside A4, and legality is decided by exact region containment (contract section 3). The retained contract stays valid only for lattice states reached from retained-state checkpoints (contract section 5). |
| Model identity | `assets/manifest.json`, immutable | A new model identity, with its own definition of states, moves and journal records. 600-cell-Full itself is unchanged. |
| Engine | Re-implemented behind a new boundary; the differential oracle proves it equal to 0.4 | Needs a second state representation (poses, not permutations). The oracle has no 0.4 counterpart for jumbled states; correctness would rest on exact certificates such as the witness. |
| Session store and journal | Moves are generator indices | Moves carry exact rotations, for example the Q(√5) parameter of the witness rotation, plus legality certificates. Migration of 0.4 data is unaffected. |
| Command layer and command table (H-08) | 1,200 generators; preview, undo, protection | New commands: a jumble twist with a parameter or a finite menu, display of blocking reasons, and a return-to-checkpoint path. Protection (Net/Strict) must be evaluated on poses and admissible paths. |
| Solving workflow | Orbit-first block building on the retained group | Unchanged at retained-state checkpoints. Off the lattice there are no orbits, stars or certificates (contract section 5; plan-check finding Q4). The human-solve boundary is unchanged: no automatic unjumbling. |
| Renderer (gate: 259,800 slots, ≥ 30 fps average, p99 ≤ 33.3 ms) | Labels in fixed slots; per-cell frames | Per-piece 4D transforms for up to 177,120 pieces. Pieces leave the polytope surface, so the outer shape changes. Blocked and admissible caps and their certificates must be displayable. Not measured; the selection gate would have to cover the jumbling mode too. |
| Design track (2.3) | Layout, presets and motion for the retained workflow | Extra states to design: a jumbled shape, blocked grips and a twist-angle input. Look Lab variants would need them. |

## 2. Effect on the stage 2 schedule

- **Charter conflict.** Stage 2.0 closed with the mathematical contract as a binding input, "not reopened in stage 2" (charter section 2). Adding jumbling to 1.0 reopens it. That needs an owner decision, and a model-identity change is an "Ask the owner" item.
- **Freeze gates.** The 2.5 architecture freeze (command table, layer interfaces, renderer feature list; Astra gate ruling) would have to include the pose-based state and its commands. The migration format freeze would not change.
- **Renderer window.** The 2.4 renderer window and its gate measurements would need a jumbling scenario that is not defined today.
- **Duration.** The work above is not estimated. Per the schedule rules, the integrator reports a likely miss once with a re-plan, and the owner moves, cuts or drops the work.

## 3. Options

| Option | What it means | Cost to 1.0 |
| --- | --- | --- |
| **A. Research track only (recommended)** | 1.0 scope stays as chartered. Jumbling continues on this branch: theory, exact witnesses and an optional pose-based simulator and renderer prototype, decided after the 1.0 architecture freeze. | Low. Two inputs to the 2.5 freeze, if the owner wants them: the renderer accepts a per-piece transform buffer, and the engine's state interface does not assume permutations only. Both are recorded as candidates, not authorized. |
| B. Jumbling mode in 1.0 | A second puzzle in the same application, with a new model identity, engine state, commands, UX and gate scenario | High. Reopens the charter's binding inputs and all three freeze inputs, and adds design work in 2.3. |
| C. Separate exploration tool | A four-dimensional Jambler-style viewer for grip orbits and certified witnesses, outside the product | Low. Not part of 1.0; it shares no runtime. |

Recommendation: option A. Option C can be added alongside if the owner wants a visual exploration tool.
