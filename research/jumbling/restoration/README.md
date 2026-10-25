# Restoring jumbled configurations (12 to 17 October 2026)

This directory holds the restoration research of 12 to 17 October 2026 for the jumbling puzzle on the retained
600-cell-Full geometry, with the twist menu I_a: at each of the 600 grips, the 12 rotations that keep the cut
arrangement and 48 jumble twists that do not. The model has 177,120 pieces and 259,800 sticker slots, and its
rotation group K⁺ has order 7,200.

A jumble twist can move a piece *off the lattice*: its pose is no longer in K⁺, so it no longer fills a chamber of
the retained puzzle, and it can straddle other cuts and block their grips. The question is how to get back to the
solved state from any reachable configuration, reading only the current state. Stage 10 answers open problem X1:
infinitely many configurations are reachable, so no bound on the restoration length holds for all of them.

## The standard

On 15 October 2026 the owner set what counts as restored
([decision](../../../docs/wiki/decisions/owner-decisions-2026-10-15-jumbling-restored.md)):
1. all 259,800 sticker slots carry their home labels; a rotation of a piece that maps its stickers onto themselves
   does not matter;
2. a macro may be very long, as long as a person can replicate it with the program's help;
3. most important is that the machine has a general method.

## Results

Every result below was checked independently. The off-lattice words and the corrections were replayed on the exact
simulator: every twist is a menu element and admissible, and every recorded state digest is reached. The retained
cascade was evaluated as an exact permutation program on all 259,800 sticker labels.

| Input | Off the lattice at the start | Off-lattice phase | Lattice phase | Status |
| --- | ---: | --- | --- | --- |
| **stop62**: a 40-twist scramble (94,147 pieces off), then a 62-twist seam descent | 860 | 14 clean steps, 14,098 twists, collateral 0 at every step | corrections 8,129 twists; retained cascade 151,011,430 twists | **certificate**: solved, identity labels on all 259,800 stickers |
| **X**: a reachable state with three glued blocks (stage 01) | 107 | 3 clean segments, 2,401 twists (stage 01) | corrections 21,888; cascade 223,378,194 | **certificate**: solved |
| **I1, I2**: reachable lattice states (stage 02) | 0 | none | corrections 21,660 and 21,888; cascades 222,690,068 and 224,683,850 | **certificate**: solved |

The stop62 state lies on one replayed word from the solved state (stage 05, section 5), so that 40-twist scramble is
restored to the solved state end to end. The three records (that history replay, the 14-step chain and the lattice
run) join by state digest. The order of its 14 steps was chosen by hand: at each state the researcher
picked the unit and the tool. The state-only driver v4, which makes these choices itself, took the same start from
860 to 269 off pieces in 8 steps and then stopped: it had no fallback (stage 09, section 6.2; replayed).

Along the way:
- **G = 𝒦** (stage 03, certificate): the retained puzzle's group is exactly the kernel of its invariants. The
  certificates are explicit words for 35 position and 11 frame certificates, checked by an independent checker. The
  group order has 598,061 decimal digits, and its index in the full wreath product is 2^43 · 5^2.
- **A_J = C2^34 × C5^2** (stage 04): the group of corrections that a jumbled history can leave on the lattice. The
  lower bound is a certificate (40 generator words); the upper bound is proved (Theorems A and B5).
- **The lattice restorer (Theorem R of stage 09, proved):** with a correction library that spans A_J, every
  reachable configuration with all pieces on the lattice is solved, in the sense of stickers. Stage 03 has the
  weaker form: the restorer solves a configuration or returns an exact failure certificate. The library has 36
  macros, of rank 34 over F2 and 2 over F5. (The senior review of 15 October 2026 restricted both forms to
  reachable configurations: an unreachable lattice configuration with displaced centres has nu = 0.)
- **Lemma U (stage 08, proved):** a clean step lowers the off count by at least its unit's size, so the off-lattice phase
  always terminates.
- **A clean step always exists (stage 09, proved in the senior review):** at every reachable state with an off
  piece, an exhaustive search over words that reads only the state finds a clean step. There is no bound on its
  length.
- **Theorem M\*, a general method (stage 09, proved in the senior review):** fast heuristic steps, that
  exhaustive search as a fallback when they find nothing, then the lattice phase with the corrections last. It
  restores every reachable configuration from the current state alone, in at most as many clean steps as there
  are off pieces at the start. The driver implements it; the runs of 15 October used the earlier driver v4, which
  had no fallback.
  Where the pure twist and transport words apply, a step has an explicit bound; units with pieces of height four
  lie outside them.
- **Theorem C, the pure twist (stage 09, proved in the senior review):** if the two poles of the carrier's edge
  cell and one common neighbour are free, a word of at most 720 twists turns that cell alone: every piece outside
  the cell keeps its exact pose. Closed by vacating the cell, it gives a clean step. Carriers in vertex cells are
  turned in place too (at most 104 twists); carriers in triangle and tetrahedron cells cannot be, but pure
  transport words land them in another cell of the same kind (at most 240 and 192 twists). A transport word moves
  several whole cells of that kind and fixes every other cell. These bounds assume the class's grips are free.

## The method, in two phases

1. **Off-lattice phase.** Split the off pieces into *units*: rigid blocks, or one frame part of a multi-frame block.
   A *clean step* for a unit is a word after which the unit and every piece that was on the lattice are on the
   lattice, and every off piece outside the unit and the step's declared *free set* keeps its exact pose. The fast
   steps have the form V·W·V⁻¹: W puts the unit on the lattice, V first empties the chambers W needs (from a
   peeling certificate), and V⁻¹ undoes it; a *pure twist* (commutator powers around a cycle of cells, exponents
   solved over Z5) turns one carrier's cell alone. When the fast steps find nothing, an exhaustive search over the
   menu words in order of length takes the step (Theorem M\*).
2. **Lattice phase.** With every piece on the lattice, compute the invariant ν (43 bits over F2 and 2 values over
   F5), choose the correction from the library, solve the rest with the retained puzzle's cascade, and apply the
   correction last.

[09-general-method/METHOD.md](09-general-method/METHOD.md) gives the method, the status of every part, what the
new standard removes from the earlier programme, and the reproduction commands.

## Gaps

| Gap | What is missing | Status |
| --- | --- | --- |
| M1 | A construction from the state alone with a bound on its length. Bare existence is proved (exhaustive search), and with it as a fallback the method is complete (Theorem M\*), which the driver implements. A bound for every state exists if and only if R(I_a) is finite, and R(I_a) is infinite (stage 10), so a bound must depend on the state. Round 21: a computable bound in (n, h) valid at every reachable state exists exactly when reachability is decidable (Theorem D); explicit bounds hold on classes given by checkable hypotheses ([record](09-general-method/SENIOR-REVIEW.md) section 26) | **open**; the core gap; option B at left heights 2 and 4 **in progress** ([stage 12](12-low-height/README.md)) |
| M2 | The committed lattice cascade took 1.5 to 2.3 × 10^8 twists on the runs so far, about 5 to 7 years at one twist per second. With 31 of its 35 seeds replaced by shorter relocated elements (round 7), it restores the two synthetic inputs in 4.5 × 10^7 twists, about 1.4 years, and its cascade needs at most 71,516,526 twists for every input (the corrections at most 25,852 more), or at most 4,841,151 actions (single twists and applications of 70 stored macros); with retained words only, some input needs at least 224,601 twists (proved, model review). A construction that applies the seeds fewer times needs at most 20,866,844 twists for every input (proved, model review), about 1.33 times the fewest twists that any cascade solving one orbit at a time with conjugates of these seeds spends on them; implemented on 16 October as a runner separate from the driver, it restores the two synthetic inputs in 1.58 × 10^7 twists, about 6 months at one twist per second. Against "a person can replicate it" | **closed** by owner decision ([17 October 2026](../../../docs/wiki/decisions/owner-decisions-2026-10-17-jumbling-m2.md)): help for a person moves to the engineering line; bounds **proved**; runs **certificate** |
| E1 | Coverage: deep scrambles at the I_a@40 scale (about 94,000 off pieces) were not run with the general method. Stage 11 (17 October) runs phase U alone on unrestricted random scrambles, one or two seeds per depth: 10 and 20 twists and one of two 40-twist scrambles reach the lattice; for the other 40-twist scramble and at 60, 100 and 1000 twists the clock stopped the descent after it had put back 98.7%, 86.2%, 72.8% and 35.1% of the off pieces, and no run got stuck. Most of the residue lies in rigid blocks of the three sizes catalogued in stage 06, but only 27% (60 twists) and 5% (100 twists) of it in groups of the catalogued shapes, and the stored template words alone restore 1.5% and 0.4% of it | **in progress** ([stage 11](11-scramble-census/README.md)); the general method from these stops **not run** |
| E2 | Review of the proofs. The senior model review of 15 October 2026 checked stages 08 and 09 and the A_J upper bound, proved Lemma C as Theorem C and corrected Lemma CM and Theorem R ([record](09-general-method/SENIOR-REVIEW.md)) | model review **done**; human review **not done**; a Lean formalization from a complete handoff document is planned once M1 is solved (owner, 17 October 2026) |

## Stages

Each stage directory has its own README with results, replay commands, files and limits. Result names are local to
a stage: Lemma D, Lemma U, Theorem R, Theorem G and several others name different results in different stages, so
this page gives the stage with each. The state E2 (3 off pieces, stage 01) is not e2run24 (690 off pieces, the end
of the 24-twist "E2 run" in the history of stage 05).

| Stage | Date | What it established | Main document |
| --- | --- | --- | --- |
| [01](01-closure-x-to-e3/README.md) closure X to E3 | 12 Oct | Three clean restorations from the current state: X (107 off) to E1 to E2 to E3 (0 off), collateral 0 | segment reports |
| [02](02-three-block-family/README.md) three-block family | 12 Oct | The finishing word as a family: iterated conjugation, a state-only certificate, about 9.3 × 10^10 hole configurations covered | [report](02-three-block-family/report/three-block-family.md) |
| [03](03-retained-group-and-lattice-restorer/README.md) retained group and lattice restorer | 13 Oct | G = 𝒦 without hypothesis; the lattice restorer and Theorem R | [report](03-retained-group-and-lattice-restorer/report.md) |
| [04](04-card5-lattice-invariants/README.md) card 5 | 14 Oct | A_J = C2^34 × C5^2; b5 = 0 by a potential; height bounds for the inner orbits | [PROOFS](04-card5-lattice-invariants/PROOFS.md) |
| [05](05-seed-table/README.md) seed table | 14 Oct | Single seeds fail inside glued blocks (Lemma G); the reachable history of X to E3 replayed from solved | [PROOFS-3E](05-seed-table/PROOFS-3E.md) |
| [06](06-block-family/README.md) block family | 14 Oct | A block-shape family carried by K⁺; first steps on stop62 (860 to 689) | [PROOFS-BF](06-block-family/PROOFS-BF.md) |
| [07](07-general-family/README.md) general family | 14 Oct | One step family for every block class; heap peeling removes 61% to 90% of the off pieces on deep states | [PROOFS-GF](07-general-family/PROOFS-GF.md) |
| [08](08-units-and-clean-steps/README.md) units and clean steps | 15 Oct | Lemma U; stop62 from 689 to 86 by clean steps and a carrier move; where the word family stops | [PROOFS-U](08-units-and-clean-steps/PROOFS-U.md) |
| [09](09-general-method/README.md) general method | 15 Oct | The two-phase method; stop62 and X to the solved state | [METHOD](09-general-method/METHOD.md) |
| [10](10-x1-unbounded/README.md) X1 | 17 Oct | R(I_a) is infinite: an explicit family of words raises seven pieces without bound (X1 closed) | [REPORT](10-x1-unbounded/REPORT.md) |
| [11](11-scramble-census/README.md) scramble census | 17 Oct | Phase U on unrestricted random scrambles of 10 to 1000 twists: a fast bulk of single-twist steps, then a slow tail of three-twist repairs, no run stuck; what is left at the stops | [README](11-scramble-census/README.md) |
| [12](12-low-height/README.md) low heights (option B) | 17 Oct | A completeness conjecture for left heights 2 and 4 and its plan; a structural screen and a strict-step falsifier at three height-2 states; the partner template from the state alone at s61c; one retained letter at the blocked units of the depth 60 stop (20 checked strict′ steps); a first bounded driver run from the depth 40 stop (no step); strict four-letter commutators for the height-4 units (232 lowering words at X, none at the depth 100 stop or the fixture end); LC-13 transport words of 12 and 24 letters at the depth 40 stop, one of them closed into a checked strict′ step of 624 letters | [README](12-low-height/README.md) |
| [13](13-tutorial-runner/README.md) tutorial runner | 19–20 Oct | The machine that follows the tutorial's rules from the current state, playing literal words, and an independent checker that replays its record twist by twist; a depth 10 and a depth 20 scramble solved and checked. Since 20 October the runner also builds a closure whose V is known only by a certificate, expands V into a literal word and records the step in format 2, and the checker reads both formats; this is tested at two states of E2's literal replay, not on a run from a scramble. A literal lattice player plays a lattice word's retained part and corrections twist by twist (R22D-57); the runner does not call it yet and it has not been run on a runner record, so in a run the lattice phase's retained part is still checked by composing permutations. The stage also holds the checker of the R22D-25 kernel's records (stage 12) | [README](13-tutorial-runner/README.md) |
| [14](14-blocking-structure/README.md) blocking structure | 24 Oct | Card 6 attacked jointly: why the grips a lowering needs are blocked at deep states; the precedence peel T3P (a tame round with finite tests at the state, at most 81 letters with clean helpers), certified at the D2 end of seed 7304; chained greedily it lowers Φ by 6% to 56% at the six D2 height ends and then has no round | [PROOFS](14-blocking-structure/PROOFS.md) |
| [15](15-lowering-machine/README.md) lowering machine | 24 Oct | Card 7 attacked jointly: a chronological certificate for any word (Theorem MC); retained releases and re-aims, which free 15 of the 131 own holders the precedence peel left; the lowering machine (a pump on two clean caps that share a high unit), proved, with no instance where tested; the certified procedure P* lowers Φ at all six chain ends and then stops for good at every one of them, where no macro of its families lowers Φ | [PROOFS](15-lowering-machine/PROOFS.md) |
| [16](16-six-jams/README.md) six jams | 25 Oct | Card 8 attacked jointly: every jam where two-level P* stopped has a certified tame descent, by transports to targets that are not own holders or by routing releases whose helpers move the obligated pieces inside brackets; with transports at its stops the procedure continues at all six jams and stops again, and where it stops depends on the route | [PROOFS](16-six-jams/PROOFS.md) |

The earlier theory (the tree model, rim peel, seam reading, buffer transport, cards 1 to 4) is in
[`../theory/`](../theory/README.md), and the open problems are in [`../theory/open-problems.md`](../theory/open-problems.md).
The questions put to an external prover, with the answers so far, are in [`../theory/cards/`](../theory/cards/README.md).

## How to verify

- Every script runs with `python3 -I` and NumPy, headless on Linux, and reads the research tree `research/jumbling`
  (the exact simulator `sim/` and the theory modules) without writing to it. Its code on main is unchanged since
  commit `27a001c`, which the runs used.
- Decompress the `.gz` files first, for example with `gunzip -k`. Each stage's `IMPORT-MANIFEST.json` lists the
  files stored gzipped and the SHA-256 of the original bytes.
- Paths in logs and commands are written as tokens: `<research/jumbling>`, `<echain>` (the five shared modules of
  stage 01), `<scratch>`, `<repo>`, `<tmp>`.
- Each result has a checker that does not import the construction code: `replay_check.py` (in `../theory/`),
  `verify_chain.py`, `verify_hp.py`, `verify_u.py`, `check_certs.py`, `verify_restore.py` and `verify_late.py`;
  stage 10 has three in `10-x1-unbounded/check/`, and stage 11 replays its census records with
  `11-scramble-census/scripts/check_records.py`.
- The runs used `ulimit -v 7000000` per process. Run times in the records are cloud timings, not performance
  evidence.

## Provenance

The stages were delivered as packages and are stored here without their Chinese delivery notes, checksum lists and
archives. Each `IMPORT-MANIFEST.json` names its source package:

| Stage | Source package |
| --- | --- |
| 01 | `Claude-E-closure-X1-20261012`, `Claude-E-closure-E1E2-20261012`, `Claude-E-closure-E2E3-20261012`, `Claude-E-closure-chain-X-E3-20261012` |
| 02 | `Claude-E-three-block-family-20261012` |
| 03 | `Claude-cards-items2-3-20261013` |
| 04 | `Claude-Card5-Ia-final2-20261014` |
| 05 | `Claude-3E-seed-table-20261014` |
| 06 | `Claude-3E-block-family-20261014` |
| 07 | `Claude-3E-general-family-20261014` |
| 08 | `Claude-3E-units-20261015` (version 3.1) |
| 09 | the working tree of the general method (driver v4 for the runs; M\* since the senior review's audit) |
| 10 | the output archive of the X1 run of 16 to 17 October 2026 |
| 11 | the integrator's census runs of 17 October 2026, with the scripts of this repository |

## Evidence and checks not run

Everything here is exact computation on synthetic geometry in headless Linux runs. Labels: **proved** (the author's
proof), **certificate** (an exact computation with a replayable record), **measured** (a result on finite inputs),
**open**. The senior model review of 15 October 2026 ([record](09-general-method/SENIOR-REVIEW.md)) checked the
proofs of stages 03, 04, 08 and 09. Stage 10 was checked by the run's two referee subagents and by the integrator.
No human has reviewed these proofs.

Not run: Windows, DirectX, the native host regression, input, long sessions and performance measurements.
