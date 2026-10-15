# Jumbling the 600-cell

In the retained puzzle every turn of a cell's cap is a symmetry of the cut arrangement, so every piece always lands
in a slot of the same lattice. A *jumble twist* turns a cap by a rotation about its cell's axis that is not such a
symmetry. The pieces it moves leave the lattice: they no longer fill a slot, they may stick across other cuts, and
the caps they straddle can no longer turn until they move away. The puzzle stays rigid (pieces never overlap or
change shape), but its group becomes infinite and its configurations become hard to read.

This directory is the jumbling research of the project. It asks whether the 600-cell can jumble, how to simulate
and draw it exactly, and how to restore a jumbled configuration to the solved state.

## Decisions

- **9 October 2026:** no fudged variant. Rigid jumbling of the retained geometry enters 1.0 as an independent puzzle
  with its own model identity, plus a Jumble feature; 600-cell-Full is unchanged
  ([decision](../../docs/wiki/decisions/owner-decisions-2026-10-09-jumbling.md)).
- **10 October 2026:** the restoration method must restore every reachable configuration completely
  ([decision](../../docs/wiki/decisions/owner-decisions-2026-10-10-jumbling-complete.md)).
- **13 and 14 October 2026:** the twist menu is I_a: at every grip, the 12 retained rotations and 48 jumble twists of
  one icosahedral group ([decision](../../docs/wiki/decisions/owner-decisions-2026-10-13-jumbling-menu.md)).
- **15 October 2026:** a configuration counts as restored when every sticker slot carries its home label; macros may
  be long if a person can replicate them with the program's help; a general method for the machine comes first
  ([decision](../../docs/wiki/decisions/owner-decisions-2026-10-15-jumbling-restored.md)).

## Where it stands (15 October 2026)

- **Exact legality.** The simulator decides whether a twist is admissible in exact arithmetic over Q(√5), with no
  tolerance. A ten-degree jumble twist of one cap was certified legal and reversible on the retained geometry
  ([first study](study-2026-10-09.md)).
- **End-to-end restorations.** A 40-twist scramble that takes 94,147 of the 177,120 pieces off the lattice is
  restored to the solved state, independently replayed: a 62-twist seam descent to 860 off pieces, 14 clean steps
  (14,098 twists, no collateral) to 0, then the lattice phase. Three other reachable inputs (X, I1 and I2) are restored too. The order
  of the 14 steps was chosen by hand; a driver that reads only the state is running
  ([restoration](restoration/README.md)).
- **Structure.** The retained group is exactly the kernel of its invariants (G = 𝒦, an order of 598,061 digits),
  and the corrections a jumbled history can leave on the lattice form A_J = C2^34 × C5^2.
- **Open.** No theorem yet says that every reachable state has a clean step that the method finds (gap M1). The
  lattice phase is about 1.5 to 2.2 × 10^8 twists, far beyond what a person can replicate (gap M2). Deep scrambles
  were not run with the general method (gap E1). The new proofs have no independent review, and the pure twist
  (Lemma C) has no written proof yet (gap E2).
- **Rendering.** The research viewer draws all 259,800 stickers of a jumbled state. On the owner's RTX 4070 Laptop
  GPU, the jumbling pose probe ran the I_a fixture at 613 fps (p99 1.91 ms); overlay costs are not measured yet
  ([evidence](../../docs/wiki/evidence/w-j-probe-results.md)).

## Map

| Path | What it is |
| --- | --- |
| [restoration/](restoration/README.md) | Restoration research of 12 to 15 October 2026, in nine stages: closures, families, the lattice group, the general method and the end-to-end results |
| [theory/](theory/README.md) | Theory and scripts: tree model, rim peel, seam reading, buffer transport, the problem cards for an external prover; [open problems](theory/open-problems.md) |
| [sim/](sim/README.md) | J1, the exact reference simulator for rigid jumbling; tests in `tests/test_jumbling_sim.py` |
| [viewer/](viewer/README.md) | J2, the WebGL research viewer, including the full-state page |
| [explorer/](explorer/README.md) | J4, the four-dimensional grip-orbit explorer |
| [fixtures/](fixtures/README.md) | W-J fixtures: scripted scrambles for the S4, I_a and I_b menus, replayed by J1 |
| [state-contract.md](state-contract.md) | What a valid jumbled state is: exact one-sided containment, certificates, reachable configurations |
| [render-contract.md](render-contract.md) | The data contract between the engine side and every drawing of the jumbling puzzle |
| [study-2026-10-09.md](study-2026-10-09.md) | The first study: interacting poles, discrete jumble twists, the fixed-slot theorem, the exact witness E0 to E4, plan check and code review |
| [impact-assessment.md](impact-assessment.md) | The initial assessment of what jumbling means for the 1.0 program |
| [theory-problem.md](theory-problem.md) and `*-packet.md` | The theory problem statement and the review packets of 9 October |
| `jumble_study.py`, `h4.py`, `exact.py`, `witness.py` | Scripts of the first study; `results.json` and `witness-results.json` are their outputs |

## Run

From the repository root, with Python 3 and NumPy:

    python tests/test_jumbling_sim.py                   # the simulator's headless tests
    python research/jumbling/fixtures/wj.py check       # replay every W-J fixture and compare its references
    python research/jumbling/witness.py                 # the exact witness E0 to E4 (about 3 minutes)

Each stage of [restoration/](restoration/README.md) gives its own replay commands.

## Evidence

Everything here is source, fixture or exact synthetic-geometry evidence from headless runs, except the GPU probe,
which ran on the owner's machine. Nothing here is evidence for Windows input, long sessions or the performance of a
1.0 build.
