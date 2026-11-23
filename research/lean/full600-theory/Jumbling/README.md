# Jumbling: round-22 restoration results in Lean

The second library of this Lake project. It formalizes two first batches of proved results from the jumbling
restoration research, indexed in
[`research/jumbling/restoration/12-low-height/ARCHON-HANDOFF.md`](../../../jumbling/restoration/12-low-height/ARCHON-HANDOFF.md)
with statements and proofs in
[`PROOFS-B.md`](../../../jumbling/restoration/12-low-height/PROOFS-B.md). It checks existing results and draws no new
conclusions.

The library shares the project's pinned toolchain (Lean v4.36.0) and Mathlib (`lake-manifest.json`), and imports the
permutation lemmas of `ArchonFull600Theory.Basic`. `lake build` builds both libraries. No file has a `sorry`, an
`admit`, a `native_decide` or a new axiom; the results depend on `propext`, `Classical.choice` and `Quot.sound` at
most. Earlier-stage results are hypotheses, never axioms (see each scope below).

## Scope A: the model layer

### Modules

| Module | Content |
| --- | --- |
| `Jumbling.Model.Pieces` | The piece model (objects O1 to O6): poses, regions, grips and menus, letters, chronological words and their inverses, admissibility, lattice and off pieces, holes, ghosts and `Bad`, endpoint collateral, strict and strict′ steps, SAFE-ALL words. |
| `Jumbling.Model.Steps` | R22A2-02, R22D-01, R22D-03, R22D-05. |
| `Jumbling.Model.ClassCell` | The class-cell model (O7): labels `G_P ⧸ H_P`, class letters, the accumulated rotation `cellAcc` (a_C), the point permutation `pointPerm`, commutators and powers, and the hypothesis structure `LC03` (O8). |
| `Jumbling.Model.CleanPowers` | R22A-04, for any permutation of a finite set of points and in the class-cell model. |
| `Jumbling.Model.Transport` | R22A-01, R22A-02, R22A-03, R22A-05, R22A-07. |

### Results

`PM.` is `Jumbling.PieceModel.`, `CCM.` is `Jumbling.ClassCellModel.` and `CP.` is `Jumbling.CleanPowers.`.

| Handoff | Lean | Content |
| --- | --- | --- |
| R22A-01, item 1 | `CCM.LC03.ghost_never_straddles`, `CCM.LC03.Bad_eq` | No ghost of a home chamber straddles a cut of P, and `Bad(W)` is the union of `Ω_C` over the home cells whose label-0 point goes to a nonzero label. |
| R22A-01, item 2 | `PM.collateral_eq`, `PM.collateral_eq_empty_iff`, `CCM.LC03.collateral_eq`, `CCM.LC03.collateral_eq_empty_iff` | `C_Z(W)` is the set of lattice pieces at chambers of `Bad(W)`, so it is empty exactly when `Bad(W) ⊆ holes(Z)`. The piece-model form holds for every admissible word. |
| R22A-01, item 3 | `CCM.exists_home_label_ne`, `CCM.LC03.exists_home_chambersOf_subset_Bad`, `CCM.LC03.le_encard_Bad` | A word that lands an off content of kind κ puts `Ω_C` into `Bad(W)` for a home cell `C` of kind κ, so `|Bad(W)| ≥ m` when every home cell of kind κ has at least `m` chambers. |
| R22A-02 | `CCM.LC03.keeps_content_iff`, `CCM.LC03.lattice_stays_iff`, `CCM.LC03.lattice_content_stays_iff`, `CCM.LC03.foreign_apply` | `W` keeps the exact pose of every piece of a nonempty content exactly when it fixes the five points of its cell; a lattice content stays on the lattice exactly when `(C, 0)` goes to a label-0 point; foreign pieces never move. |
| R22A-03 | `CCM.LC03.transport` (with `CCM.cleanPower`, `CCM.pathWord`, `CCM.pathWord_fixes`) | A path of clean powers of commutators at free grips from the content's point to a label-0 point is a word of `4·Σ L_i` letters, admissible at `Z`, a strict step for the content, with `C_Z(W) = ∅`. |
| R22A-04 | `CP.cleanExp_dvd_orderOf`, `CP.orderOf_dvd_lcmUpTo`, `CP.pow_fixes_iff`, `CP.pow_eq_cleanPow_pow`, `CP.exists_pos_pow_eq_inv`, `CP.eqvGen_iff_reflTransGen`, `CP.exists_walk_length_lt`, `CCM.cleanExp_dvd_orderOf_dvd`, `CCM.cleanPower_complete`, `CCM.cleanPower_inv`, `CCM.cleanGraph_eqvGen_iff`, `CCM.exists_cleanPath` | `L_A` divides the order of `A` on the points, which divides `lcm(1, …, N_P)`; a reachable target has a path of at most `f − 1` clean powers and a word of at most `4E(f − 1) ≤ 4E(N_P − 1)` letters; every power of `A` that fixes `O` is a power of `A^{L_A}`; inverses are positive powers, so undirected components are reachability classes. |
| R22A-05 | `CCM.pointPerm_injective`, `CCM.cellMap_injective`, `CCM.capacity`, `CCM.LC03.capacity`, `CCM.vacant_le`, `CCM.le_vacant`, `CCM.pointPerm_ne_of_fixed` | Points and cells never merge; at most `v_κ` initially distinct off contents of kind κ land; the type-count bounds on `v_κ` (with `max(0, ·)` as natural-number subtraction); a partly filled cell cannot receive another content. |
| R22A-07 | `CCM.LC03.glued` | A class-P word that lands an off content glued to a lattice content of its cell takes every piece of that lattice content off. |
| R22A2-02 | `PM.isStrictPrime_after_iff`, `PM.occupied_apply`, `PM.isStrictPrime_after_iff_disjoint` | A second strict′ step `w2` stays strict′ after `w1` exactly when `Bad(w2) ⊆ holes(Z·w1)`, and the test at `Z` alone through the good-ghost map of `w1` and the chambers it lands. |
| R22D-01 | `PM.isStrictPrime_conj_iff`, `PM.admissible_wordInv_of_isStrictPrime`, `PM.apply_conj_eq_iff`, `PM.forall_apply_conj_eq_iff` | For a retained word `A`, `C` is a strict′ landing step at `Z·A` exactly when `A·C·A⁻¹` is one at `Z`, with `A⁻¹` admissible automatically; `A·C·A⁻¹` returns every piece outside `U` exactly when `C` does. |
| R22D-03 | `PM.isStrictPrime_lowering_iff`, `PM.side_outside_of_lowering`, `PM.isStrictPrime_lowering_of_returns` | The endpoint contract of `a·C·a⁻¹` (conditions (1) to (3)); if `C` lands `U` and `D ∉ K⁺`, `U` ends `C` outside the cap; landing `U` outside the cap and returning every other piece is sufficient. |
| R22D-05 | `PM.nestedVacancy` | `a·q⁻¹·b·q·a⁻¹` is admissible, lands `U` and returns every other piece to its pose of `Z`. |

### Hypotheses and readings

The piece model `PieceModel` has a group of poses with the subgroup `K` (K⁺), regions acted on by the poses, and for
each grip a side function (inside, outside, straddle) and a menu. Its fields are the facts the proofs use:

- `menu_side`: a menu element keeps the side of every region with respect to its own grip (a letter maps its cap onto
  itself). The source uses this without stating it.
- `menu_inv`: menus are closed under inverses.
- `chamber_side`: no home chamber straddles a cut (a lattice piece never straddles, O2).
- `smul_chamber`: `K` maps home chambers to home chambers; `home_mem`: each piece's home is a chamber.

A word is admissible when no piece straddles the cut of a letter when that letter acts (`admissible_cons` shows this
is the prefix condition of O2). SAFE-ALL grips are fixed at the state where the word starts (O5).

The class-cell model `ClassCellModel` (O7) takes `G_P` as a subgroup of the poses, `H_P = G_P ∩ K⁺`, labels
`G_P ⧸ H_P` with label 0 the coset of 1, cells acted on by `G_P`, and the fields `inCap_smul` (a class menu element
keeps its cap), `kind_smul` (`G_P` keeps kinds) and `chamberCell_smul` (the cell map on chambers is
`H_P`-equivariant, which is how "a pose in K⁺ keeps a lattice ghost on the body" enters R22A-01 item 3 and the
capacity).

**LC-03 (object O8) is an earlier-stage result and is not proved here.** It is the hypothesis structure `LC03` over a
state `Z`, with these fields:

- `admissible`: every prefix of a word of free class-P letters is admissible;
- `own_apply`: every class-P off piece of a cell `C` ends at `a_C(W)·γ` from its pose `γ`;
- `ghost_clear` and `ghost_acc`: no ghost of a home chamber straddles a cut of P, and the ghosts of the chambers of
  `C` receive `a_C(W)`;
- `foreign_outside`: pieces of a foreign frame lie outside every free cap;
- `rigid`: a rotation of `G_P` that fixes a cell lying in at least one cap of P and all its labels is the identity
  (the outer cell is never moved, which is proved);
- `off_cases` (a reading of O7's freeness): every off piece is foreign or a class-P off piece in some cell.

Readings in the statements:

- "`W` lands the content at `(X, ℓ)`" means that `W` sends that point to a label-0 point of a home cell; the label part
  is proved equivalent to the pieces landing (`CCM.LC03.apply_mem_K_iff`), and the home-cell part is a hypothesis of
  R22A-01 item 3 and R22A-05 item 2.
- R22A-01 item 3: the cell sizes 111 (triangle) and 87 (tetrahedron) are geometric facts; the theorem takes "every
  home cell of the kind has at least `m` chambers" as a hypothesis.
- R22A-05 item 3: the chamber counts `b_{κ,t}` per type are a hypothesis.
- R22D-03: "`D ∉ K⁺`" is a hypothesis of `side_outside_of_lowering`; the model has no heights, so it cannot derive it
  from "`D` lowers `U`".

Some statements need fewer hypotheses than the source states, so the Lean theorems are stronger:

- R22A-03: `ℓ ≠ 0`, the wholeness of `U`, H3 and "the target is a home cell of `U`'s kind with a free fibre" are not
  needed, and neither is step 5 of the proof, because LC-03 covers the whole word. The owned set is any finite set `O`
  that contains the lattice label-0 points and the fibres of the other contents; the source's `O_U` with its excluded
  cells is one such set.
- R22A2-02: the two units need not be disjoint.
- R22A-07: the lattice content need not be nonempty.
- R22D-03: "`U` whole in `c` at height 4" and "`D` lowers to height 2" are not needed for the equivalence.
- R22D-05: "lowering" and "`b` is a jumble letter" are not needed.

### Not formalized in scope A

- Computed instances: R22A-05's and R22A-07's instance (R22A-08), the b07 and b14 instance of R22A2-02, R22D-01's
  counts.
- Parts that use other earlier-stage results: the remark of R22A-01 (M1-03), the counterexample of R22A-02 (LC-12),
  the SAFE remedy of R22A-07 (Theorem C, LC-12, Proposition N), the consequences of R22D-03 that use AU-03 or R21F-12.
- R22A-04: the integrator's unreviewed remark (Landau's function), the count `m ≤ (120·59)²` and the search cost.
- R22A2-02: the capacity case, which needs the chambers of the pieces `w1` lands to lie in their new cell `H₁`; LC-03
  as stated does not say where a landed piece's chamber lies. Also the remark on disjoint caps.
- R22D-01: the exact one-letter update of the cap classification (the model has no action of `K⁺` on grips).
- R22D-03: the remark that returning only the pieces inside the cap is not enough.
- R22D-05: the existence bound and the necessary orbit conditions.
- R22A-06 is the next model result in the dependency order; it needs M1-17 and AU-03 as further hypotheses.

## Scope B: the group-theory layer

### Modules

| Module | Content |
| --- | --- |
| `Jumbling.Group.Words` | The common setting: an alphabet of letters, each with a permutation of a finite set of chambers and an inverse letter; chronological words (Full600's `wordPermutation` convention: the first letter acts first) and their inverses; the generated group and its orbits; N1 and N2; three-cycles on an orbit; pairs and triples. |
| `Jumbling.Group.Schreier` | R22B-05. |
| `Jumbling.Group.Cluster` | R22B-03. |
| `Jumbling.Group.Certified` | R22B-01 and R22B-02. |
| `Jumbling.Group.PowerSeed` | R22B-12. |
| `Jumbling.Group.Anchors` | R22B-06, with the nine-point counterexample. |
| `Jumbling.Group.ShortSeeds` | R22B-17. |

### Results

`A.` is `Jumbling.Alphabet.`; other names are in the namespace `Jumbling`.

| Handoff | Lean | Content |
| --- | --- | --- |
| R22B-01 (given the certificate) | `A.certifiedThreeCycles`, `A.certifiedThreeCycles_uniform`, `A.tables_of_connected`, `A.certifiedThreeCycles_shortLetters`, `A.certifiedThreeCycles_alt`, `diagonalCounterexample` | Given N1, N2, seeds that act on each target orbit as a three-cycle and fix the earlier orbits pointwise, and the two tables, there is a word `V` with `T ⊆ holes(Z·V)` and `|V| ≤ 2·Σ k_i(s_i + 2e_i + 2f_i)`, so `|V| ≤ 10·|T|·L`; connected single-letter stabilizer graphs give tables with `e ≤ d − 2` and `f ≤ d − 3`; without tables, under the conclusion of Theorem A as a hypothesis, `|V| ≤ Σ k_i[s_i + 2(d_i(d_i − 1)(d_i − 2) − 1)]`; the diagonal `A₅` counterexample. |
| R22B-02 (given the certificate) | `A.vacateShortest`, `A.vacateShortest_bounded`, `A.vacateShortest_certified` | If some word vacates `T`, one of at most `C(N, n) − 1 ≤ N^n − 1` letters does; with the certificate of R22B-01, `|V| ≤ 10·n·L`. |
| R22B-03 | `clusterLemma`, `clusterExactTest`, `support_chronoComm_subset`, `separatedOverlap` | The cluster lemma and its exact test `p⁻¹(P ∪ Q) ⊆ H`; the support of `c = a·b·a⁻¹·b⁻¹` lies in `I ∪ a⁻¹(I) ∪ b⁻¹(I)`, and under separation `c` is the product of the disjoint three-cycles `(r, b⁻¹(r), a⁻¹(r))`. |
| R22B-05 | `A.schreierTables` | Under H1 (the group induces a group containing `Alt(Δ)` on `Δ`) and `d ≥ 5`, tables `P_x` and `Q_z` exist with `e ≤ (d − 2)(2d − 1)` and `f ≤ (d − 3)(2d − 3)(2d − 1)`. |
| R22B-06 | `A.isSeed_conj`, `A.isSeed_conj_inv`, `A.selectionTest`, `A.exists_short_pair_word`, `A.selection_decides`, `A.exists_inv_anchor_iff`, `A.not_stabConnected`, `A.conj_derangement`, `NinePoint.anchorCounterexample`, `NinePoint.r_b_nineCycles`, `NinePoint.act_ne`, `NinePoint.not_stabConnected`, `NinePoint.seedWord_eq`, `NinePoint.consecutiveWord_eq`, `NinePoint.jordanWord_spec`, `NinePoint.group_eq`, `NinePoint.card_group`, `NinePoint.schreierTables_nine` | The conjugation convention; the selection test over the at most `d(d − 1)` anchor pairs, with a conjugator of at most `d(d − 1) − 1` letters, decides whether a conjugate of the seed repairs the single-letter tables; derangement letters give stabilizer graphs with no edges. The nine-point counterexample: `r` and `b = r·t` are nine-cycles, `r⁻¹·b = t` is a seed, `t·r⁻²·t·r² = (0 1 3 4 2)` with `9/2 < 5 ≤ 9 − 3`, the letters generate `A₉` (order 181,440; proved through Full600's `PureControllerCertificate`, without Jordan's theorem), and R22B-05's tables exist there. |
| R22B-12 | `powerSeed_iff`, `criterion_of_seed`, `powerSeed_of_criterion`, `otherLcm_dvd_of_seed`, `A.powerSeed_word`, `lcm_le_pow`, `powerSeed_bound`, `A.powerSeed_commutator`, `powerSeed_counterexample` | A positive power of `q` is a seed exactly when `q` has on `E ∪ Δ` exactly one cycle of length divisible by 3, of length 3 and inside `Δ`; then `M` is prime to 3, `q^M` is a seed of `|q|·M` letters and every admissible exponent is a multiple of `M`; `M ≤ D^c` and `s ≤ 8·D^c` for the commutator forms; a 3-cycle of `q` on an earlier orbit rules out every exponent. |
| R22B-17 (conditional) | `tableE`, `tableF`, `tableE_mono`, `tableF_mono`, `A.shortSeedBound`, `seedMax`, `le_seedMax`, `A.shortSeedBound_max`, `tableE_7200`, `tableF_7200`, `tables_7200`, `A.shortSeedBound_7200` | With R22B-05's tables and seeds of at most `S(d_i)` letters, `|V| ≤ 2·Σ k_i[S(d_i) + 2E(d_i) + 2F(d_i)]`; with every `d_i ≤ D`, `|V| ≤ 2t[S*(D) + 2E(D) + 2F(D)]`; `E(7200) = 103,644,002`, `F(7200) = 1,491,955,394,391` and `2E(7200) + 2F(7200) = 2,984,118,076,786`. |

### Hypotheses and readings

- N1 and N2 are hypotheses, as in the source (`A.N1`, `A.N2`). Proposition N is not proved. N1 follows from N2,
  whose singleton orbits are N1's case (`N2.n1`).
- R22B-05's H1, also used by R22B-17: every even permutation of `Δ` is induced on `Δ` by an element of the group.
  The Jordan certificate that decides it in the source is not formalized.
- **Theorem A is an earlier-stage result and is not proved here.** R22B-01's "without tables" remark takes its
  conclusion as the hypothesis `hAlt`: the permutation that acts as `σ` on `Δ_i` and fixes everything else lies in the
  group, for every `σ ∈ Alt(Δ_i)`.
- R22B-01: the target orbits are a list in the chosen order, and `hcover` says that every orbit of size at least 2
  that meets `T` is one of them (the reading of "the orbits of size at least 2 that meet T"). The earlier orbits of
  orbit `i` are those before it in the list. The certificate is the structure `IsTables` with the seeds.
- R22B-12 makes explicit that `q` maps `Δ` into itself, that `Δ` and `E` are disjoint and the chamber set is finite;
  its bound takes every point of `E ∪ Δ` to lie in a `q`-invariant set of at most `D` chambers, `1 ≤ D`, and at most
  `c` primes, with the commutator words of at most 8 letters.
- R22B-06: `Δ` and `E` are invariant. In the nine-point example the letters are `r`, `r⁻¹`, `b`, `b⁻¹`, `Δ` is all
  nine points and `E` is empty; the "Jordan word" is formalized as the facts the source cites (a 5-cycle, 5 prime,
  `9 < 2·5`, `5 ≤ 9 − 3`), not as a Jordan certificate.
- R22B-17: the seed bounds `s_i ≤ S(d_i)` and `d_i ≤ D` are hypotheses; in `A.shortSeedBound_7200`, "every orbit has
  at most 7,200 chambers" (the source's K⁺ chamber types) is a hypothesis.

Where the formal statement is sharper: R22B-01 derives the disjointness of the orbits from the seeds and needs only
`d ≥ 3` for its construction (`d ≥ 5` is kept as stated); R22B-05 derives transitivity on `Δ \ {a}` from H1 through a
three-cycle instead of citing the point stabilizer of `Alt(d)`; the counterexample of R22B-12 needs no 3-cycle on `Δ`.

### Not formalized in scope B

- Computed instances: R22B-02's count at the depth 40 stop, R22B-03's b00 commutators, R22B-17's b00 and b19
  constants (they rest on computed orbit sizes).
- R22B-03's greedy-run remark, R22B-05's sharper instance bounds and its remark without H1.

Out of scope for this batch: R22B-08 and R22B-15 (they need Jordan's theorem for a prime cycle, which the pinned
Mathlib does not have), R22B-19 and R22B-25 (they need `Aut(Alt(d)) = Sym(d)`, also absent), and R22B-07.

## How it was produced

Claude Code (Claude Opus 5.5), with no Archon run and no Codex call; the two scopes were written in parallel by two
Claude subagents and integrated in the main session.
