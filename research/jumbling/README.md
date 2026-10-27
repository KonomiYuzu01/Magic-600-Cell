# Jumbling the 600-cell

In the retained puzzle every turn of a cell's cap is a symmetry of the cut arrangement, so every piece always lands
in a slot of the same lattice. A *jumble twist* turns a cap by a rotation about its cell's axis that is not such a
symmetry. The pieces it moves leave the lattice: they no longer fill a slot, they may stick across other cuts, and
the caps they straddle can no longer turn until they move away. The puzzle stays rigid (pieces never overlap or
change shape), but the rotations of the caps now generate an infinite group, and its configurations become hard to
read. The twist menu I_a reaches infinitely many configurations (proved on 17 October 2026).

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
- **17 October 2026:** the most important output of the restoration research is a tutorial that people can read and
  carry out, from unrestricted random scrambles. After the first scramble experiments the owner chose, provisionally,
  a measured coverage census with a manual of techniques and a completeness theorem for low heights
  ([decision](../../docs/wiki/decisions/owner-decisions-2026-10-17-jumbling-tutorial.md),
  [census](restoration/11-scramble-census/README.md)).
- **26 and 27 October 2026:** the tutorial and the jumbling research are **frozen**. The tutorial is shelved
  unaccepted; card 9 (restoration stage 18) is the last card, and its last twist-by-twist replays were stopped; the
  development runs stay as recorded. Reopening any of it is an owner decision
  ([26 October](../../docs/wiki/decisions/owner-decisions-2026-10-26-tail-then-freeze.md),
  [27 October](../../docs/wiki/decisions/owner-decisions-2026-10-27-freeze.md)).
- **27 October 2026:** the research is handed to the engineering line as data only
  ([data page](../../docs/progress/1.0/jumbling-handoff/research-data.md)), and the jumbling line is designed as a
  puzzle to be solved, in which a methodology that restores the complete puzzle from any scramble of depth 3000 is
  open; the answers already given stay, and what is not yet decided assists the remaining research
  ([decision](../../docs/wiki/decisions/owner-decisions-2026-10-27-jumbling-handoff.md)).

## Where it stands (17 October 2026)

- **Exact legality.** The simulator decides whether a twist is admissible in exact arithmetic over Q(√5), with no
  tolerance. A ten-degree jumble twist of one cap was certified legal and reversible on the retained geometry
  ([first study](study-2026-10-09.md)).
- **End-to-end restorations.** A 40-twist scramble that takes 94,147 of the 177,120 pieces off the lattice is
  restored to the solved state: a 62-twist seam descent to 860 off pieces, 14 clean steps (14,098 twists, no new
  off-lattice pieces at the end of any step) to 0, then the lattice phase. It was checked independently: the
  off-lattice words and the corrections were replayed on the exact simulator, and the retained cascade was evaluated
  as an exact permutation program on all sticker labels. Three other reachable inputs (X, I1 and I2) are restored
  too. The order of the 14 steps was chosen by hand. The state-only driver v4 took the same start from 860 to 269
  off pieces and stopped there for lack of a fallback ([restoration](restoration/README.md)).
- **Structure.** The retained group is exactly the kernel of its invariants (G = 𝒦, an order of 598,061 digits),
  and the corrections a jumbled history can leave on the lattice form A_J = C2^34 × C5^2.
- **Senior review (15 October 2026).** A Codex senior review proved that a clean step always exists and that an
  exhaustive search reading only the state finds one, with no bound on its length. With that search as a fallback
  after the fast steps, and the lattice phase at the end, the method restores every reachable configuration from the
  current state alone (Theorem M\*), and the driver now implements it; the runs so far used v4, which had no
  fallback. It proved the pure twist (Lemma C) with a bound of 720 twists and extended it to every kind of cell (in
  place for vertex cells, by transport for triangle and tetrahedron cells), corrected the statements of Lemma CM and
  Theorem R, and checked the other proofs. It also proved that a bound on the restoration length for every reachable
  configuration exists if and only if there are finitely many reachable configurations (there are not; see below),
  and bounded the lattice phase: a construction of its cascade needs at most 356,565,166 twists for every input
  (4,840,993 actions, counting single twists and applications of 70 stored macros), the corrections at most 25,852
  more, and with retained words only some input needs at least 224,601 twists
  ([record](restoration/09-general-method/SENIOR-REVIEW.md)). It is a model review, not a human one.
- **Shorter seeds for the lattice phase.** Commutators whose supports meet in one piece give pure three-cycles,
  which move three pieces of one orbit and nothing else, for 33 of the 35 orbits. The lattice cascade needs no fibre
  alignment from its seeds (proved, model review), so 31 of its 35 seeds are replaced by shorter elements moved onto
  its buffers. It then restores the two synthetic inputs in 4.5 × 10^7 twists instead of 2.3 × 10^8 (checked by the
  independent checker of stage 03), and its cascade needs at most 71,516,526 twists for every input. A construction
  that applies the seeds fewer times needs at most 20,866,844 (proved, model review); implemented
  on 16 October as a runner separate from the driver, it restores them in 1.58 × 10^7 twists (checked by the same
  checker).
- **X1 settled (17 October 2026).** The left heights of reachable poses are unbounded, so R(I_a) is infinite: an
  explicit family of words, admissible from the solved state, alternates jumbles at two grips whose caps both hold
  seven pieces whole and raises them by 2 with every letter, without end. An unattended run on the owner's API
  credit found it, and the integrator checked it independently; no human has reviewed it
  ([stage 10](restoration/10-x1-unbounded/README.md)).
- **Open.** A construction that works from the state alone with a bound on its length, which must depend on the
  state (gap M1). Round 21 showed that a computable bound in the number of off pieces and the largest height, valid
  at every reachable state, exists exactly when reachability is decidable, and proved bounds only for classes given
  by checkable hypotheses ([record](restoration/09-general-method/SENIOR-REVIEW.md) section 26). Deep scrambles were
  not run with the general method (gap E1). A census of the state-only descent on unrestricted random scrambles
  began on 17 October: scrambles of 10 and 20 twists reach the lattice, and so does one of two 40-twist scrambles;
  the others descend fast and then slowly, and no run got stuck
  ([stage 11](restoration/11-scramble-census/README.md)). Option B, a completeness conjecture for left heights 2 and
  4, has its plan and four experiments ([stage 12](restoration/12-low-height/README.md)): the partner template
  restores the s61c state from the state alone with catalogue cores (2,374 letters against 4,402 from its history),
  and at the depth 60 stop one retained letter makes a unit of every blocking group ready, with checked strict′
  steps of at most 506 letters for 20 of the 52 units left there. A first bounded driver run from the depth 40 stop
  made no step; most units there are triangle and tetrahedron contents, which need a transport closure (LC-13). A
  transport search, tried on three of them, finds for each a single commutator power of 12 or 24 letters that lands
  it, and the tetrahedron content among the three closes into a checked strict′ step of 624 letters; for the two
  triangle contents no vacating word was found yet. For height 4, an exhaustive search for four-letter commutators that move one high unit
  and nothing else, pruned by a proved lemma, finds 232 that lower the unit at the positive control X and none for any
  unit at the depth 100 stop or the fixture end. No human has reviewed the new proofs (gap E2); once M1 is solved, a Lean formalization
  is to check them.
- **Closed by the owner (17 October 2026).** Gap M2, whether a lattice phase of 1.58 × 10^7 twists, or a few
  million actions (single twists and stored-macro applications), is within reach of a person: the bounds stand,
  and help for a person moves to the engineering line
  ([decision](../../docs/wiki/decisions/owner-decisions-2026-10-17-jumbling-m2.md)).
- **Rendering.** The research viewer draws all 259,800 stickers of a jumbled state. On the owner's RTX 4070 Laptop
  GPU, the jumbling pose probe ran the I_a fixture at 613 fps (p99 1.91 ms); overlay costs are not measured yet
  ([evidence](../../docs/wiki/evidence/w-j-probe-results.md)).

## Map

| Path | What it is |
| --- | --- |
| [manual/](manual/README.md) | The manual of the restoration techniques for a person (draft, option A of 17 October): what the solver sees, the descent with worked examples from the census, an outline of the tail, and the lattice phase (the invariant ν, the corrections and the retained method) |
| [restoration/](restoration/README.md) | Restoration research of 12 to 17 October 2026, in twelve stages: closures, families, the lattice group, the general method, the end-to-end results, the answer to X1, the scramble census and the low-height work |
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
