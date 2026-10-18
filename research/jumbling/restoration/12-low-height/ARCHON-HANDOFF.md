# Handoff to the Lean formalization (draft)

Draft of 18 October 2026, for the Lean agent Archon. Formalization has not started: when it starts and what it covers
wait for the owner's decision ([E2 and the Lean plan](../../../../docs/wiki/decisions/owner-decisions-2026-10-17-jumbling-m2.md);
[B′(h)](../../../../docs/wiki/decisions/owner-decisions-2026-10-18-bprime-goal.md)). A passed formal verification is
one of the conditions for merging draft pull request #112
([owner decision, 17 October 2026](../../../../docs/wiki/decisions/owner-decisions-2026-10-17-jumbling-pr.md)); this page
is the index that work starts from.

The statements, hypotheses, proofs and bounds are in [`PROOFS-B.md`](PROOFS-B.md), written hypothesis by hypothesis for
this purpose. This page adds what a formalization needs for its plan: the objects, the 117 results of theory round 22 in
an order in which each comes after the results it uses, the hypotheses that are finite computations and what decides
them, and the results of earlier stages and of the literature that the proofs rely on. It was extracted from
`PROOFS-B.md` at commit 2e94c53, extended by hand with the results recorded later (R22B-19 to R22B-29, R22C3-01 to
R22C3-06 and COV-01 to COV-43), and checked against the file's headings; line numbers are those of the file in the
commit that last updated this page.

## 1. Four layers

- **Group theory** (13 results, layer "group"): finite permutation groups and words, with no puzzle data, for example
  R22B-01 (certified three-cycles give a polynomial bound), R22B-05 (Schreier tables), R22B-12 (when a power of a
  word is a seed), R22B-19 (local Alt and separating words give a product of alternating groups) and R22B-25 (a
  finite classifier of the induced action). They come first and need Jordan's theorem on primitive groups with a
  prime cycle, Schreier's lemma, the normal subgroups of Alt(d) and Sym(d), and the simplicity and automorphisms of
  Alt(d) (section 5).
- **The model** (46 results, layer "model"; 25 more, "model + computed", also have computed hypotheses): statements
  about the puzzle's state model, defined once through the objects O1 to O11 (section 2): poses in K⁺ over Q(√5),
  letters, heights, steps, the SAFE-ALL group, ghosts and Bad sets, the class-cell model, blocks and types. The vacancy
  theorem R22B-29 (a finite equivalence for vacancy at a middle, without Alt) and the coverage results COV-03, COV-06,
  COV-08, COV-10, COV-12, COV-15, COV-21, COV-32, COV-34, COV-38, COV-39, COV-40 and COV-42 belong here.
- **Computed facts** (23 results, layer "computed", and the computed hypotheses of the mixed ones): finite checks on
  stated states, decided by the scripts named in section 4. A formalization does not re-derive them by hand: either a
  checker whose soundness is proved reads the recorded data (R22B-16 states the soundness of `check_r22b10.py` for the
  certificates of R22B-10), or they stay stated computational facts, marked as such.
- **Specifications** (10): R22B-10 (implemented as E22-1b), R22B-11 (an audit), R22B-18 (not implemented), R22C3-05 (an
  inventory of proposed routes), COV-11 (an audit of a heuristic), COV-16, COV-22, COV-28, COV-35 and COV-43 (experiment
  design); nothing to prove.

## 2. Objects

Defined in `PROOFS-B.md` section 1 (O1 to O8) and in the preamble of section 7 (O9 to O11). "Decided by" names the
code that computes the object at a state.

| Object | Name | Section, line | Decided by |
| --- | --- | --- | --- |
| O1 | pieces and the lattice | 1, 46 | — |
| O2 | letters | 1, 50 | restore.element |
| O3 | heights | 1, 55 | trees.pose_tree |
| O4 | steps | 1, 58 | — |
| O5 | SAFE-ALL | 1, 62 | buffer.condition and buffer.check_jordan (research/jumbling/theory/buffer.py) |
| O6 | ghosts and Bad | 1, 68 | fastghost.Ghost, ghostlib.ghost |
| O7 | the class-cell model (LC-01) | 1, 72 | e3_class.Model and classes.py (research/jumbling/theory/) |
| O8 | LC-03 (used throughout) | 1, 81 | — |
| O9 | blocks, locks and shadows | 7 (preamble), 1683 | blocks.Blocks |
| O10 | types | 7 (preamble), 1690 | regions.orbit_of |
| O11 | creators and partners | 7 (preamble), 1693 | — |

## 3. Results in dependency order

"Uses" lists the results of this file that a proof uses, "Used by" the reverse; results of earlier stages and of the
literature are in section 5. Statuses as in `PROOFS-B.md`. A formalization can follow the numbering.

| # | Id | Result | Status | Layer | Uses (this file) | Used by | Section, line |
| ---: | --- | --- | --- | --- | --- | --- | --- |
| 1 | R22A-01 | Zero Bad means zero endpoint collateral | proved | model | — | R22A-03, R22C-V | 2, 88 |
| 2 | R22A-02 | Whole-cell ownership | proved | model | — | R22A-03, R22A-06, R22A-09 | 2, 114 |
| 3 | R22A-03 | The transport lemma | proved | model | R22A-01, R22A-02 | R22A-04, R22A-06 | 2, 130 |
| 4 | R22A-04 | Bounds and completeness for clean powers | proved | group | R22A-03 | R22A-06 | 2, 161 |
| 5 | R22A-05 | No merging; target capacity | proved | model | — | R22A2-02, R22C-V | 2, 177 |
| 6 | R22A-06 | Entry and closure | proved; conditional on the hypotheses | model | R22A-02, R22A-03, R22A-04 | R22A2-01, R22A2-04, R22A2-05, R22A2-06, R22B-23, R22C3-01 | 2, 204 |
| 7 | R22A-07 | Glued contents | proved; instance computed | model | — | R22A-08, R22A2-06 | 2, 234 |
| 8 | R22A-08 | Vacant home cells at the depth 40 stop | computed | computed | R22A-07 | — | 2, 250 |
| 9 | R22A-09 | The five disputed units | computed | model + computed | R22A-02 | — | 2, 259 |
| 10 | R22B-01 | Certified three-cycles give a polynomial bound | proved; conditional on the certificate | group | — | R22B-05, R22A2-01, R22B-02, R22A2-06, R22B-09, R22B-10, R22B-11, R22B-16, R22B-17 | 3, 553 |
| 11 | R22B-05 | Tables of Schreier words always exist | proved | group | R22B-01 | R22A2-01, R22B-07, R22B-09, R22B-10, R22B-17 | 3, 642 |
| 12 | R22B-03 | Cluster certificates and separated commutators | proved; instance computed | group | — | R22B-08, R22B-14, R22B-15, R22B-22 | 3, 606 |
| 13 | R22B-08 | A seed from a Jordan word and a letter of small support | proved | group | R22B-03 | R22A2-01, R22B-09, R22B-10, R22B-14 | 3, 721 |
| 14 | R22A2-01 | b19's vacating word by construction | computed hypotheses; proved consequence | model + computed | R22A-06, R22B-01, R22B-05, R22B-08 | R22A2-04 | 2, 282 |
| 15 | R22A2-02 | When a second strict′ step survives the first | proved | model | R22A-05 | R22A2-03, R22A2-05 | 2, 307 |
| 16 | R22A2-03 | The thirteen supplied steps compose at most six deep | computed | computed | R22A2-02 | — | 2, 329 |
| 17 | R22A2-04 | Fresh closures land fourteen units in a checked order | proved; computed hypotheses | model + computed | R22A-06, R22A2-01 | R22A2-05, R22C3-02 | 2, 352 |
| 18 | R22B-02 | A bound in n alone whenever V exists | proved | group | R22B-01 | R22A2-05, R22A2-06 | 3, 593 |
| 19 | R22A2-05 | A shared target: a fresh closure, not the old preparation | proved; instance computed and built | model | R22A-06, R22A2-02, R22A2-04, R22B-02 | — | 2, 387 |
| 20 | R22A2-06 | Closure bounds and the two failure modes at the stop | proved; instances computed | model | R22A-06, R22A-07, R22B-01, R22B-02 | — | 2, 412 |
| 21 | R22B-04 | b00's original target can be vacated | computed hypotheses; proved consequence | model + computed | — | R22B-07, R22B-09, R22B-20, R22B-23 | 3, 626 |
| 22 | R22B-06 | Moving the anchors: a decision procedure, and no guarantee | proved; counterexample checked | group | — | R22B-10 | 3, 675 |
| 23 | R22B-07 | Seeds: an exact criterion and a fallback that always works | proved | group | R22B-05, R22B-04 | R22B-10 | 3, 695 |
| 24 | R22B-09 | b00: all hypotheses hold; a construction bound | computed hypotheses; proved consequence | model + computed | R22B-01, R22B-04, R22B-05, R22B-08 | — | 3, 752 |
| 25 | R22B-10 | A checker for certificates with word-valued tables | specification; implemented, E22-1b | specification | R22B-01, R22B-05, R22B-06, R22B-07, R22B-08 | R22B-16, R22B-18 | 3, 768 |
| 26 | R22B-11 | E22-1's seeds protected fewer orbits | checked in the source | specification | R22B-01 | — | 3, 804 |
| 27 | R22B-12 | When a power of a word is a seed | proved | group | — | R22B-13, R22B-17, R22B-18 | 3, 815 |
| 28 | R22B-13 | Two short seeds for b00 | computed | computed | R22B-12 | R22B-18 | 3, 836 |
| 29 | R22B-14 | Powers of one letter cannot shrink a support | proved; computed | model + computed | R22B-08, R22B-03 | — | 3, 850 |
| 30 | R22B-15 | A local test for Alt, and what stays open | proved; the universal claim open | group | R22B-03 | R22B-19, R22B-21, R22B-22, R22B-25 | 3, 860 |
| 31 | R22B-16 | The checker of E22-1b is sound, and the recorded numbers hold | checked in the source; computed | model + computed | R22B-01, R22B-10 | — | 3, 883 |
| 32 | R22B-17 | The bound that short seeds would give | proved; conditional | group | R22B-01, R22B-05, R22B-12 | — | 3, 898 |
| 33 | R22B-18 | An extended commutator-power search | specification; not implemented | specification | R22B-12, R22B-13, R22B-10 | — | 3, 919 |
| 34 | R22B-19 | Local Alt and separating words give a product of alternating groups | proved | group | R22B-15 | R22B-20, R22B-22, R22B-23, R22B-25, R22B-28, R22B-29, COV-10 | 3, 953 |
| 35 | R22B-20 | At b00 no letter separates two pairs; one word separates all | computed | computed | R22B-19, R22B-04 | — | 3, 978 |
| 36 | R22B-21 | An orbit of twelve chambers with a group of order twelve | computed; counterexample | computed | R22B-15 | R22B-24 | 3, 990 |
| 37 | R22B-22 | Small orbits need a joint test; they occur at the depth 60 stop | proved; computed | model + computed | R22B-15, R22B-03, R22B-19 | R22B-23, R22B-24, R22B-25, R22B-26, R22B-27, R22B-28, R22B-29 | 3, 1008 |
| 38 | R22C-G | Lemma G: one rotation per cell | proved; R22CF-01, X22C-01 | model | — | R22C-V, R22C-T1′, R22C-T, R22C-M, R22C2-01, R22C2-05, COV-01, COV-17, COV-20 | 7, 1696 |
| 39 | R22C-V | Type vectors and counts | proved; computed; R22C-04, R22CF-03, X22C-03, X22C-04 | model + computed | R22A-01, R22A-05, R22C-G | R22C-T1′, R22C2-05, COV-03, COV-04 | 7, 1798 |
| 40 | R22C-T1′ | Theorem T1′: land, vacate the landing chambers, unwind | proved; computed; R22CF-04, X22C-05 | model | R22C-G, R22C-V | R22B-23, R22C-T2′, R22C2-01, R22C2-03, R22C2-05, R22C-K, R22C3-02, R22C3-03, R22C3-04, COV-06 | 7, 1832 |
| 41 | R22B-23 | What the closure bounds give without Alt | proved; conditional | model | R22B-04, R22B-19, R22B-22, R22A-06, R22C-T1′ | R22B-29 | 3, 1029 |
| 42 | R22D-01 | Retained conjugation | proved | model | — | R22D-08, R22C-05, R22C-K, R22C3-02, R22C3-03 | 5, 1287 |
| 43 | R22D-02 | Six-letter commutators: the mask lemma | proved; computed | model + computed | — | — | 5, 1315 |
| 44 | R22D-03 | The endpoint contract of a lowering conjugate | proved | model | — | R22D-08 | 5, 1335 |
| 45 | R22D-04 | Free grips after lowering at the depth 100 stop | computed | model + computed | — | — | 5, 1357 |
| 46 | R22D-05 | Nested-set vacancy for a two-level step | proved | model | — | — | 5, 1371 |
| 47 | R22D-06 | Fixed occupied chambers rule out both whole-cap middles | computed; proved | model + computed | — | — | 5, 1387 |
| 48 | R22D-07 | Unlocking | proved; Lemmas B and C conditional | model | — | — | 5, 1401 |
| 49 | R22D-08 | A finite bound for a stated family | proved | model | R22D-01, R22D-03 | — | 5, 1417 |
| 50 | R22C-T | Locked chambers and Lemma T | proved as scoped; R22CF-02, R22C-01, X22C-02 | model | R22C-G | COV-37 | 7, 1719 |
| 51 | R22C-S | The Shadow lemma | proved; R22CF-02, X22C-02 | model | — | — | 7, 1757 |
| 52 | R22C-M | Partners do not repair N1 | proved; R22CF-03, X22C-03 | model | R22C-G | — | 7, 1785 |
| 53 | R22C-T2′ | One partner and T1′ | proved; conditional on its certificates; R22CF-03, X22C-03 | model | R22C-T1′ | R22C-K, R22C3-02 | 7, 1868 |
| 54 | R22C2-01 | The intrinsic Q-shadow | proved | model | R22C-G, R22C-T1′ | COV-01, COV-03, COV-04, COV-14 | 7, 1884 |
| 55 | R22C2-03 | One action for the edge-cycle family | proved for a stated family | model | R22C-T1′ | — | 7, 1904 |
| 56 | R22C2-05 | b06: the twelve landing rotations | computed; proved parts | model + computed | R22C-V, R22C-G, R22C-T1′ | — | 7, 1920 |
| 57 | R22C-FC | Theorem FC: no self-frozen set | proved; R22CF-05, X22C-07 | model | — | COV-03, COV-13, COV-15, COV-26, COV-30, COV-32, COV-34, COV-38 | 7, 1936 |
| 58 | R22C-R1 | Lemma R1: the one-letter readiness test | proved; R22CF-05, X22C-07 | model | — | — | 7, 1952 |
| 59 | R22C-CH | Lemma CH: co-held cycles | conditional; R22CF-05, X22C-07 | model | — | — | 7, 1963 |
| 60 | R22C-05 | One-letter unlocking with vacancy | proved | model | R22D-01 | COV-01 | 7, 1973 |
| 61 | R22C-06 | A rigid two-domain cell cannot be unlocked by retained conjugation | proved | model | — | — | 7, 1986 |
| 62 | R22C-K | The catalogue, backward certification and coverage | conditional; R22C-07, R22CF-06, X22C-08, R22C2-06 | model + computed | R22C-T1′, R22C-T2′, R22D-01 | R22C3-06, COV-15, COV-29 | 7, 1998 |
| 63 | R22C3-01 | Strict′ steps keep SAFE-ALL, not the hole inventories | proved | model | R22A-06 | R22C3-02, R22C3-04, R22B-28, COV-08 | 7, 2077 |
| 64 | R22C3-02 | Mixed orders need resource contracts as well as a blocker order | proved | model | R22C-T1′, R22C-T2′, R22D-01, R22A2-04, R22C3-01 | — | 7, 2098 |
| 65 | R22C3-03 | Experiment D passes an independent exact audit | computed | computed | R22C-T1′, R22D-01 | R22B-27 | 7, 2127 |
| 66 | R22C3-04 | 142, then 151: a proved prefix of 130's predecessors | proved from computed hypotheses | model + computed | R22C-T1′, R22C3-01 | R22B-27, COV-05 | 7, 2144 |
| 67 | R22C3-05 | The catalogue of unresolved units stays conditional | inventory | specification | — | — | 7, 2171 |
| 68 | R22C3-06 | A uniform core bound; coverage stays open | proved; conditional | model | R22C-K | COV-03, COV-15, COV-21, COV-24, COV-29, COV-34, COV-36, COV-39 | 7, 2191 |
| 69 | R22B-24 | A non-Alt orbit meets a catalogue target that has a hole; vacancy still holds | computed | computed | R22B-21, R22B-22 | — | 3, 1065 |
| 70 | R22B-25 | A finite classifier of the induced action, with a residual case | proved | group | R22B-15, R22B-19, R22B-22 | — | 3, 1084 |
| 71 | R22B-26 | The depth 60 home targets miss every small orbit | computed | computed | R22B-22 | — | 3, 1104 |
| 72 | R22B-27 | The T1′ middles of 151 and 180 miss the small orbits too | computed | computed | R22B-22, R22C3-03, R22C3-04 | — | 3, 1115 |
| 73 | R22B-28 | What a strict′ step keeps: subgroup certificates, not inventories | proved | model | R22C3-01, R22B-19, R22B-22 | — | 3, 1125 |
| 74 | R22B-29 | The vacancy theorem: a finite equivalence without Alt | proved | model | R22B-19, R22B-22, R22B-23 | COV-03, COV-08, COV-10, COV-12 | 3, 1143 |
| 75 | COV-01 | An isolated triangle with an intrinsic Q-shadow | computed; proved; reachability open | model + computed | R22C2-01, R22C-G, R22C-05 | COV-14, COV-19, COV-20, COV-21, COV-22, COV-26, COV-27, COV-28 | 8, 2248 |
| 76 | COV-02 | Seven geometric off-point cases | computed | computed | — | COV-23 | 8, 2275 |
| 77 | COV-03 | A local certificate, and what coverage still needs | proved; conditional | model | R22C2-01, R22C-V, R22C-FC, R22C3-06, R22B-29 | COV-15 | 8, 2290 |
| 78 | COV-04 | At 737, b05 and b15 are blocked by other units, b06 by two types | computed | computed | R22C-V, R22C2-01 | COV-33 | 8, 2321 |
| 79 | COV-05 | Unit 130 at depth 60 is blocked by seven units | from the record | computed | R22C3-04 | — | 8, 2343 |
| 80 | COV-06 | Coverage needs the holes and the off poses, not the lattice orientations | proved | model | R22C-T1′ | COV-08, COV-09, COV-10, COV-12, COV-36, COV-38, COV-43 | 8, 2352 |
| 81 | COV-07 | The stored T1′ words form no order at 737 | computed | computed | — | — | 8, 2371 |
| 82 | COV-08 | T1′ resources pull back through the good-ghost map | proved | model | COV-06, R22C3-01, R22B-29 | COV-09, COV-10 | 8, 2414 |
| 83 | COV-09 | A strict′ predecessor turns N2 at a whole-cap middle from true to false | computed | computed | COV-06, COV-08 | COV-16, COV-38 | 8, 2436 |
| 84 | COV-10 | SAFE preparation: an exact finite test and a flow criterion | proved | model | COV-06, COV-08, R22B-19, R22B-29 | COV-11, COV-12, COV-15, COV-24, COV-32, COV-33 | 8, 2456 |
| 85 | COV-11 | prep2 is a heuristic, not a preparation test | audit | specification | COV-10 | — | 8, 2481 |
| 86 | COV-12 | Whole-cap cores: exact eligibility and a three-segment form | proved | model | COV-06, COV-10, R22B-29 | COV-13, COV-14, COV-15, COV-21, COV-24, COV-29, COV-32 | 8, 2494 |
| 87 | COV-13 | A reachable height-2 state with no all-content-landing cap | computed | computed | R22C-FC, COV-12 | COV-16, COV-32, COV-33, COV-38, COV-42 | 8, 2515 |
| 88 | COV-14 | The isolated triangle has whole-cap landers but no prepared whole-cap vacancy | computed; proved | model + computed | COV-01, COV-12, R22C2-01 | COV-20, COV-21, COV-23, COV-25, COV-27 | 8, 2530 |
| 89 | COV-15 | Coverage reduces to one finite per-state predicate | proved; conditional | model | COV-03, COV-10, COV-12, R22C3-06, R22C-K, R22C-FC | COV-16, COV-21, COV-24, COV-29, COV-34 | 8, 2550 |
| 90 | COV-16 | The adjudicating experiments are done; E5's inputs were wrong | experiment design | specification | COV-09, COV-13, COV-15 | — | 8, 2576 |
| 91 | COV-17 | A legal 24-letter word reaches exactly one off piece | computed | computed | R22C-G | COV-20, COV-22, COV-33, COV-41 | 8, 2606 |
| 92 | COV-18 | Singleton off contents exist in all four kinds of cell | proved from computed certificates | model + computed | — | COV-20, COV-21, COV-24, COV-29 | 8, 2622 |
| 93 | COV-19 | No homomorphism to an abelian group tells X apart | computed; proved | model + computed | COV-01 | — | 8, 2652 |
| 94 | COV-20 | X is outside the SAFE sandwich family of J = (0, 12); its reachability stays open | computed | computed | COV-01, COV-14, COV-17, COV-18, R22C-G | COV-22 | 8, 2666 |
| 95 | COV-21 | The constructed singletons are covered by the catalogue as it stands | proved | model | COV-01, COV-12, COV-14, COV-15, COV-18, R22C3-06 | COV-27 | 8, 2686 |
| 96 | COV-22 | One bounded mixed-class experiment for X's family | experiment design | specification | COV-01, COV-17, COV-20 | COV-28 | 8, 2704 |
| 97 | COV-23 | 500 classes of height-2 off placements: 353 rim, 131 deep, 16 without a landing letter | computed | computed | COV-02, COV-14 | COV-24, COV-25, COV-26, COV-27, COV-30, COV-31, COV-36 | 8, 2808 |
| 98 | COV-24 | Whole-cap coverage of singletons: an exact hole-access criterion | proved from computed certificates | model + computed | COV-10, COV-12, COV-15, COV-18, COV-23, R22C3-06 | COV-27, COV-29, COV-34 | 8, 2832 |
| 99 | COV-25 | No retained middle that returns X's locking block reaches X's family | proved from computed certificates | model + computed | COV-14, COV-23 | COV-27, COV-28, COV-30, COV-31 | 8, 2871 |
| 100 | COV-26 | X is not the only deep singleton; outer placements need their own treatment | computed | computed | COV-01, COV-23, R22C-FC | COV-30 | 8, 2893 |
| 101 | COV-27 | The last move of X's piece: two branches, neither excluded for unrestricted histories | computed | computed | COV-01, COV-14, COV-21, COV-24, COV-25 | COV-31 | 8, 2910 |
| 102 | COV-28 | A mixed-class experiment that changes X's cohort | experiment design | specification | COV-01, COV-22, COV-25 | COV-35 | 8, 2934 |
| 103 | COV-29 | The 24 exceptional holes: four cores through the antipodal cap | proved from computed certificates | model + computed | COV-12, COV-15, COV-18, COV-24, R22C-K, R22C3-06 | COV-30, COV-34 | 8, 2971 |
| 104 | COV-30 | Validity leaves 67 deep and two outer classes: 457,200 class–hole pairs | computed | computed | COV-23, COV-25, COV-26, COV-29, R22C-FC | COV-34, COV-35, COV-36, COV-43 | 8, 3000 |
| 105 | COV-31 | Vacant locks: a retained-word invariant without COV-25's exact return | proved from computed certificates | model + computed | COV-23, COV-25, COV-27 | COV-37 | 8, 3030 |
| 106 | COV-32 | Several off pieces: an exact joint test for a whole-cap step, and lifting past separated contents | proved | model | COV-10, COV-12, COV-13, R22C-FC | COV-33, COV-34, COV-38, COV-42 | 8, 3055 |
| 107 | COV-33 | Exact whole-cap counts: five caps at 737, none at COV-13's state, two at COV-17's | computed | computed | COV-04, COV-10, COV-13, COV-17, COV-32 | COV-34, COV-37, COV-38 | 8, 3088 |
| 108 | COV-34 | What B′(2) still needs, and the bounds | proved | model | COV-15, COV-24, COV-29, COV-30, COV-32, COV-33, R22C3-06, R22C-FC | COV-39, COV-40, COV-42 | 8, 3116 |
| 109 | COV-35 | An experiment on eight valid deep classes | experiment design | specification | COV-28, COV-30 | — | 8, 3143 |
| 110 | COV-36 | The singleton residual is 118 coverage cases | computed; the reduction proved | model + computed | COV-06, COV-23, COV-30, R22C3-06 | COV-42, COV-43 | 8, 3178 |
| 111 | COV-37 | The vacant-lock condition fails at the reachable 737 state | computed | computed | R22C-T, COV-31, COV-33 | — | 8, 3205 |
| 112 | COV-38 | Separated contents restore one at a time while their hole resources survive | proved | model | R22C-FC, COV-06, COV-09, COV-13, COV-32, COV-33 | COV-42 | 8, 3224 |
| 113 | COV-39 | R2 gives B′(2) with an explicit exhaustive catalogue | proved | model | R22C3-06, COV-34 | COV-40, COV-42 | 8, 3250 |
| 114 | COV-40 | HRH is R2 together with a lattice bridge | proved; the puzzle lemmas open | model | COV-34, COV-39 | COV-41, COV-42 | 8, 3270 |
| 115 | COV-41 | Two-letter excursions above height 2: classified, and collapsed at one cap | proved; computed | model + computed | COV-17, COV-40 | COV-42 | 8, 3293 |
| 116 | COV-42 | The covered subclass and the obligations left | proved | model | COV-13, COV-32, COV-34, COV-36, COV-38, COV-39, COV-40, COV-41 | — | 8, 3329 |
| 117 | COV-43 | An experiment on all 118 cases | experiment design | specification | COV-06, COV-30, COV-36 | — | 8, 3351 |

## 4. Hypotheses that are finite computations

Each row is a hypothesis that a result takes from a computation, with the script or function that decides it (paths
under `scripts/` are in this stage, the others under `research/jumbling/theory/` or
`research/jumbling/restoration/09-general-method/scripts/`). Hypotheses with no decider named are proof obligations of
the result itself.

| Id | Hypothesis | Decided by |
| --- | --- | --- |
| R22A-01 | H1: every letter of W is at a grip of class P that is free at Z (O7) | e3_class.Model and classes.py, per O7 |
| R22A-02 | H1: every letter of W is at a grip of P that is free at Z ('as in R22A-01'; O7) | e3_class.Model and classes.py, per O7 |
| R22A-02 | H2: Q is a nonempty class-P off content of cell C (O7) | e3_class.Model and classes.py, per O7 |
| R22A-03 | H1: U is a whole class-P off content at p = (X, ℓ) with ℓ ≠ 0 | zpath4.py |
| R22A-03 | H2: every grip used is free at Z | zpath4.py |
| R22A-03 | H3: no point of the fibre {(X, 0), ..., (X, 4)} lies in the owned set O_U (U is not glued to another content of its cell) | zpath4.py |
| R22A-03 | H4: a path p = q_0, ..., q_m with q_{i+1} = A_i^{L_i}(q_i), A_i = [x_i, y_i] the commutator of two letters at free grips, L_i the lcm of the lengths of the cycles of A_i that meet O_U (1 when none does), and q_m = (H, 0) for a home cell H of U's kind with no point of its fibre in O_U | zpath4.py |
| R22A-06 | entry: H1 to H3 of R22A-03 | zpath4.py, per R22A-03 |
| R22A-08 | state: the depth 40 stop, digest 7e128820…, 1,193 off pieces | scripts/round22/r22a08_vacancy.py |
| R22A-09 | census at the depth 40 stop with whole cells as owners | scripts/round22/transport_census.py; records runs/round22/line-a/census-class3.json and census-class24.json |
| R22B-05 | H1: G(Z) induces on Δ a group that contains Alt(Δ) | a Jordan certificate of Δ, buffer.check_jordan |
| R22B-08 | H1: a global Jordan certificate (w, p) for Δ: \|w\| = ℓ, p prime with d/2 < p ≤ d − 3, π_w has exactly one cycle of length divisible by p, a p-cycle inside Δ | buffer.check_jordan, on all of Ω |
| R22A2-01 | H1: N1 and N2 hold for T = Bad(W) (198 chambers, 15 occupied; closures/b19-target.json) | scripts/round22/r22a2_04_sequence.py; rechecked by scripts/round22/jordan_target.py |
| R22A2-01 | H2: each of the 20 orbits Δ of G meeting T (orbits 16 to 35 of buffer.Alphabet, sizes 120 to 7,200) has a global Jordan certificate (w, p) with \|w\| = 120 (H1 of R22B-08); the 120-chamber orbit has p = 73 | buffer.check_jordan on all 177,120 chambers; r22a2_04_sequence.py; jordan_target.py |
| R22A2-01 | H3: on every Δ the SAFE letter (0, 1) moves r chambers with 2 ≤ r and 2r − 1 ≤ d (H2 of R22B-08); r runs from 4 to 204 | r22a2_04_sequence.py |
| R22A2-03 | the thirteen supplied strict′ steps at the depth 40 stop: the ten census words and the complete closures V·W·V⁻¹ of b08, b09 and b10 | scripts/round22/r22a2_04_sequence.py, MATRIX and FIXED_WORDS_MAX rows |
| R22A2-03 | each word fixes the original occupied chambers, and every passing transition keeps the earlier landing sets (both checked exactly) | r22a2_04_sequence.py |
| R22A2-04 | H2: φ_i maps every chamber of its good domain into the same G0-orbit | r22a2_04_sequence.py |
| R22A2-04 | H3: before step i, h_Δ − Σ_{j earlier} \|D_j ∩ Δ\| ≥ \|B_i ∩ Δ\| on every G0-orbit Δ, singletons included (N1 is the singleton case) | r22a2_04_sequence.py, RESOURCE_STEP rows |
| R22B-04 | at the depth 40 stop, Bad(W) of b00's transport word (runs/lc13-k40/transport-b00-word.json; 111 chambers, all occupied, digest 391e9473…) satisfies N1 and N2 | scripts/round22/r22b04_jordan.py |
| R22B-04 | each of its 19 target orbits (sizes 1,440 to 7,200, 1 to 15 targets each) has a Jordan certificate checked on all 177,120 chambers (words of 120 letters, primes 1,009 to 5,783) | r22b04_jordan.py; buffer.check_jordan per O5 |
| R22B-09 | at the depth 40 stop (state 7e128820…, target digest 391e9473…, 111 chambers, all occupied) N1 and N2 hold | scripts/round22/r22b09_bound.py |
| R22B-09 | each of b00's 19 target orbits has the Jordan certificate of R22B-04 (H1 of R22B-08), rechecked on all 177,120 chambers | r22b09_bound.py |
| R22B-09 | each orbit has a single SAFE letter whose support r on the orbit satisfies 36 ≤ r ≤ 204 and 2r − 1 ≤ d (H2 of R22B-08) | r22b09_bound.py |
| R22B-13 | Z the depth 40 stop (digest 7e128820…); b00's 19 target orbits in the full decreasing-size order | scripts/round22/r22b13_seeds.py |
| R22B-16 | a pass of check_r22b10.py on a certificate at a stated state | scripts/check_r22b10.py; node lengths and bounds re-audited by scripts/round22/r22b13_seeds.py |
| R22B-19 | H1: for each i, G\|Δ_i contains Alt(Δ_i) | R22B-15's local test or a global Jordan certificate (scripts/round22/r22b20_separation.py, scripts/round22/jordan_target.py) |
| R22B-19 | H2: for each pair i ≠ j with d_i = d_j, some SAFE word has different cycle types on Δ_i and Δ_j | scripts/round22/r22b20_separation.py |
| R22B-20 | the depth 40 stop; b00's 19 target orbits and their 40 equal-size pairs | scripts/round22/r22b20_separation.py |
| R22B-21 | the state reached from the solved state by the eight menu twists listed; its SAFE alphabet and the orbit of 12 chambers | scripts/round22/r22b21_regular12.py |
| R22B-22 | the orbit counts and small groups at the depth 40 and 60 stops | scripts/round22/r22b22_small_orbits.py |
| R22C-G | W a word over the free class-P letters of Z ('H1 of R22A-01') | e3_class.Model and classes.py, per O7 |
| R22C-V | item 1: none (exact computation over the fixed geometry) | scripts/round22/r22c04_vectors.py, r22c04_orbits.py |
| R22D-04 | state: the depth 100 stop, digest 4ca44008…; all 124 high pieces lie in clean cap 61 | scripts/round22/r22d04_free_grips.py |
| R22D-06 | the depth 100 stop, prepared states after (61, 57) and after (61, 59) | scripts/round22/r22d06_fixed_chambers.py |
| R22C-T | H1: x ∈ Ω_C is occupied at Z | blocks.Blocks |
| R22C-T | H2: x is locked to the rigid block B ⊇ U (O9) | blocks.Blocks |
| R22C-S | B a rigid block (O9) ; x a home chamber | blocks.Blocks |
| R22C2-01 | H1: a chamber x occupied by a piece of g0·U such that every retained letter moving x is at a grip whose cap contains, or whose cut is straddled by, a piece of g0·Q0 (U and Q0 rotated by g0, all else as at Y) | r22c2_02_blockers.py, r22c2_05_vertex.py at the instances |
| R22C2-05 | the depth 40 stop; b06 the content of the vertex cell {91} | scripts/round22/r22c2_05_vertex.py |
| R22C3-02 | (iii) complete ghost Bad sets | fastghost.Ghost, ghostlib.ghost |
| R22C3-02 | (iv) vacancy at the right state by N1, N2 and independent Jordan factors or a joint search | scripts/round22/jordan_target.py, buffer.condition |
| R22C3-03 | the records of experiment D at the depth 60 stop and the certificate of V′ | scripts/round22/r22c3_audit_d.py |
| R22C3-04 | the geometry of 142 at the depth 60 stop: whole content, vacant cell, free poles and common neighbours, one landing rotation | scripts/round22/r22c3_geometry.py |
| R22C3-04 | 151's pure endpoint, N1, N2 and the 18 Jordan certificates | scripts/round22/r22c3_u151.py |
| R22B-24 | the state reached from the solved state by the 20 letters listed; its SAFE alphabet, the regular orbit of 12 chambers with one hole, the target (111 chambers) and the vacating letter (6, 1) | scripts/round22/r22b24_regular_hole.py |
| R22B-26 | the depth 60 stop (digest ef45a117…); its small SAFE orbits and the home-cell targets of the 184 units and of all triangle and tetrahedron cells | scripts/round22/r22b26_small60.py |
| R22B-27 | the T1′ middles of 151 (digest d76309e0…) and 180 (digest 534aaa7a…), their targets and small orbits | scripts/round22/r22b27_small_t1prime.py |
| R22B-29 | the joint target-image test Γ at a stated middle (N1, N2 and an accepting tuple) | a breadth-first search over the joint action, as in scripts/round22/r22b22_small_orbits.py; N1 and N2 by scripts/round22/jordan_target.py |
| COV-01 | the state X (digest 6580ef02…): non-overlap, type and freeness checks, and the moving-cap certificate of its two witness chambers | scripts/round22/cov01_isolated_triangle.py |
| COV-02 | the off-point slots of class 0 and their orbits under K⁺ ∩ Sym⁺(class 0) | scripts/round22/cov02_point_orbits.py |
| COV-04 | the replay state of 737 (digest 2e3b8f99…): the T1′ middles of b05 and b15, their fixed landing chambers and blockers, and b06's type inventory | scripts/round22/cov04_middles737.py |
| COV-07 | the replay state of 737 and the four stored T1′ words of b03, b04, b12 and b18 | scripts/round22/cov07_word_order.py (COV_FIRST = 3, 4, 12, 18) |
| COV-09 | the recorded 96 state of run 2 prepared by X = (178,7)(170,1)(75,1), the stored b15 program and the whole-cap letter (91,14): the strict′ contract of the predecessor and N1, N2 at both middles | scripts/round22/cov09_resource_persistence.py |
| COV-13 | the state of 120 class-0 letters from the solved state (digest afc43efd…): its heights and frame domains and the classification of all 600 grips | scripts/round22/cov13_mixed_frames.py |
| COV-14 | COV-01's state X: its 36 whole-cap landing letters, and the middle of (0,12) with its fixed occupied landing chamber and deficit | scripts/round22/cov14_isolated_wholecap.py |
| COV-17 | the state reached from the solved state by the 24-letter word J·[[[A, B], B], D]·J⁻¹ (digest 16d61ac2…): legality of every prefix, its one off piece and one hole, and the inverse word back to the solved digest | scripts/round22/cov17_single_off.py |
| COV-18 | Y = solved·J for J = (0, 12): its SAFE group, and for the four rows (vertex, edge, triangle, tetrahedron) the global Jordan certificates of the orbits of h, a and b | scripts/round22/cov18_singleton_kinds.py |
| COV-19 | the 60 I_a elements at grip 0: faithful action on the 120 class-0 poles and every element a commutator; every grip's frame in K⁺ | scripts/round22/cov19_perfect_menu.py |
| COV-20 | J·R in K⁺ maps chamber 1474 to 2149; at Y, 2149 is a hole in a singleton SAFE orbit; the type-18 holes of cap 0 at Y by cell and orbit kind | scripts/round22/cov18_singleton_kinds.py (X_TARGET_FIXED, TYPE18_SINGLE_J) |
| COV-23 | the height-2 off placements R·λ_y: the 500 orbits of H on the 177,120 chambers, no two identified by the exact vertex sets under the 7,200 rotations of K⁺, and the rim, deep and outer counts by class-0 cell | scripts/round22/cov23_singleton_classes.py (CLASSIFICATION) |
| COV-24 | at Y, the global Jordan certificates of the 29 SAFE orbits that contain rim chambers; for every rim class and landing letter, the hole-access criterion with the exact normalizations E_i = t·J·u, and its 24 exceptional holes; the reference check with the full SAFE alphabet at R·λ_2335 and R·λ_3805 | scripts/round22/cov23_singleton_classes.py (JORDAN, COVERAGE); scripts/round22/cov24_reference.py |
| COV-25 | at Y, the 29 rigid blocks and their 445 locked chambers in cap 0; X's locking block (first piece 404, its I_B and S_B, its rigidity certificate); the normalized landing chambers of X's 36 landing letters | scripts/round22/cov23_singleton_classes.py (LOCKS, LAST_MOVE) |
| COV-26 | piece 2249 at R with every other piece at identity: type, height, class-0 cell, separation from the lattice interiors and its landing chambers; the outer placements of pieces 1345 and 1437 and their mover caps; the 16 outer classes split by Theorem FC | scripts/round22/cov26_exterior_singletons.py; scripts/round22/cov23_singleton_classes.py (OUTER, LAST_MOVE) |
| COV-27 | the nine caps that contain X's piece and the 531 inverse-letter cases of its last move (36 landing, 99 retained and 396 jumble off-to-off) | scripts/round22/cov23_singleton_classes.py (LAST_MOVE) |
| COV-29 | for each of the 24 exceptional pairs: D·J applied exactly (its holes and 6,193 off pieces), the resource chamber in cap 596 in both SAFE orbits, and the global Jordan certificates of the orbit of y at the first middle and of the orbit of h under G(solved·D) | scripts/round22/cov29_two_cap.py |
| COV-30 | for the 131 deep and the seven outer representatives with movers: each home region rebuilt from its active cuts and body facets (vertices checked), rotated by R and intersected with the lattice body by exact double description, and the overlap certificates | scripts/round22/cov30_validity.py |
| COV-31 | at Y, the rigid blocks, their locked chambers and the deep landing chambers, as for COV-25 | scripts/round22/cov23_singleton_classes.py (LOCKS, LAST_MOVE) |
| COV-33 | the replay state of 737: the classification of its 600 caps, the actual middle of each landing item of each eligible cap with its Jordan certificates, the optimistic middle groups of the failed caps, and cap 91's flow and certificates; COV-13's state: its 600 caps; COV-17's singleton: its eligible caps and landing items | scripts/round22/cov33_whole_cap_737.py; scripts/round22/cov13_mixed_frames.py; scripts/round22/cov33_singleton_caps.py |
| COV-36 | for the 69 valid classes of COV-30: the orbits of the chambers of each class's type under the retained letters of the caps that leave its placement wholly outside, and the K⁺ stabilizer of the placement acting on them | scripts/round22/cov36_hole_orbits.py |
| COV-37 | the replay state of 737: its intrinsic rigid off blocks, their I_B and S_B, and the occupied chambers locked to each | scripts/round22/cov37_vl_737.py |
| COV-41 | at COV-17's singleton: the caps of other classes that contain its off piece, the returning caps and lowering items after the item 12, the two-cap witnesses and the multipliers at the first endpoint, the three-letter return at cap 8, and the same-cap counts | scripts/round22/cov41_excursions.py, scripts/round22/cov41_endpoint.py |

## 5. Results of earlier stages and of the literature

| External result | Where | Used by |
| --- | --- | --- |
| Proposition N | buffer-transport.md sections 2 and 3 (research/jumbling/theory/buffer-transport.md; named in O5) | R22A-07, R22A2-06, R22B-01, R22C-M |
| Theorem A | buffer-transport.md sections 2 and 3 (research/jumbling/theory/buffer-transport.md; named in O5) | R22B-01, R22B-04, R22B-07, R22B-08, R22B-15, R22B-19, R22B-23, R22C3-01, R22B-29 |
| Corollary A1 | buffer-transport.md sections 2 and 3 (research/jumbling/theory/buffer-transport.md; named in O5) | R22A2-04, R22A2-06, R22B-04, R22C3-04, COV-10 |
| Jordan certificate (global; Jordan word and prime) | buffer-transport.md sections 2 and 3 (research/jumbling/theory/buffer-transport.md; named in O5) | R22A2-01, R22A2-04, R22A2-05, R22B-01, R22B-04, R22B-05, R22B-06, R22B-07, R22B-08, R22B-09, R22B-10, COV-18, COV-24, COV-29, COV-33 |
| Lemma M (items (M1), (M2)) | not named in PROOFS-B | R22A2-04, R22C-M, R22C-T1′, R22C-T2′, R22C3-01, R22B-28, COV-06, COV-12 |
| Theorem B (peeling certificate) | not named in PROOFS-B | R22C-T1′ |
| Theorem C | not named in PROOFS-B | R22A-07, R22C-T1′, R22C-05, R22C3-04, R22C3-06, COV-04 |
| Theorem D (F2) | not named in PROOFS-B | R22C-FC |
| Proposition F | PROOFS-U.md (named in R22D-07; no path given) | R22D-07 |
| Lemma E6 | not named in PROOFS-B | R22D-01 |
| T1 | not named in PROOFS-B | R22C-K |
| lock lemma | buffer-transport.md section 5 (research/jumbling/theory/buffer-transport.md; named in O9) | R22C-T, COV-25, COV-31 |
| LC-03 | not named in PROOFS-B | R22A-01 |
| LC-12 | not named in PROOFS-B | R22A-02, R22A-07, R22C-G, R22C-T1′ |
| LC-13 | not named in PROOFS-B | R22C-T1′ |
| M1-03 | not named in PROOFS-B | R22A-01 |
| M1-17 (the target-orbit test) | not named in PROOFS-B | R22A-06, R22A2-06, R22C-T1′, R22C-05 |
| M1-18 | not named in PROOFS-B | R22C-R1 |
| AU-03 | SENIOR-REVIEW.md section 9 (research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md; named in O6) | R22D-03, R22D-04, R22D-08 |
| R21F-03 | not named in PROOFS-B | R22D-07 |
| R21F-12 | not named in PROOFS-B | R22D-03 |
| Jordan's theorem | not named in PROOFS-B | R22B-08, R22B-15 |
| Schreier's lemma | not named in PROOFS-B | R22B-05 |
| the point stabilizer of Alt(d) is Alt(d − 1) | not named in PROOFS-B | R22B-05 |
| the normal subgroups of Alt(d) and Sym(d), d ≥ 5 | not named in PROOFS-B | R22B-07 |
| Chinese remainder theorem | not named in PROOFS-B | R22A-06 |
| toolkit bounds b_κ (720, 104, 240, 192) | not named in PROOFS-B | R22D-03 |
| cellpure4.py / cellpure5.py word families (pair-tool catalogue with expo; transport family; cycle words, source bound 3,200) | not named in PROOFS-B | R22C2-03, R22C2-05, R22C-K |
| R22C2-06 | SENIOR-REVIEW.md section 27 (research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md; the review record of round 22, named in the file preamble) | R22C3-06 |
| the simplicity of Alt(d) and Aut(Alt(d)) = Sym(d), d ≥ 7 | not named in PROOFS-B | R22B-19 |
| the tree model of the poses (two 5-regular trees; heights as tree distances) | trees.py (research/jumbling/theory/trees.py; named in O3) | COV-39, COV-41 |
| the lattice phase (solved sticker labels) | METHOD.md (research/jumbling/restoration/09-general-method/METHOD.md; named in COV-40) | COV-40 |

The results of earlier stages (Proposition N, Theorems A to D, Lemma M, the LC- and M1- results, AU-03, R21F-03 and
R21F-12) are proved in their own files (`research/jumbling/theory/buffer-transport.md`, the senior review record
`research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md` and the stage files it cites); a formalization
either covers them or states them as assumptions, and says which.

## 6. Open, not for formalization yet

- R22B-15, R22B-21, R22B-24, R22B-25 and R22B-29: the local test for Alt is proved; the claim that every large orbit
  at every state of 𝒞₂ carries Alt is false (R22B-21, an orbit without holes); an orbit without Alt does meet a
  catalogue target that has a hole (R22B-24); and vacancy at a stated middle is decided without Alt by the finite
  test of R22B-29. Open: whether N1 and N2 alone suffice at every target of 𝒞₂, and the residual primitive case of
  R22B-25.
- R22C-K and R22C3-06: coverage of 𝒞₂ by the catalogue (backward certification) is conditional; R22C-CH needs a
  persistent co-holding hypothesis; R22D-07's Lemmas B and C are conditional. Experiment E2 (`PROOFS-B.md` sections 2
  and 8; computed) is evidence along one trajectory only: 21 strict′ steps from the depth 40 stop to the lattice, each
  an instance of R22A-03's transports, R22A-06's closures, Theorem C or R22C-T1′ (the last once after a SAFE-ALL
  preparation), with every finite hypothesis checked at the state in front of the step. Round 2 (COV-08 to COV-16)
  reduces coverage to one finite per-state predicate, Cover(Z) (COV-15). Round 3 (COV-17 to COV-22) shows reachable
  states with exactly one off piece in every kind of cell, covered by the catalogue as it stands (COV-17, COV-18,
  COV-21), rules out every abelian invariant (COV-19) and one family of reaching words (COV-20), and starts a bounded
  experiment (COV-22), which found no reaching word; the integrator's checks after COV-20 extend it to every landing
  letter and, for two of them, to a second level. The integrator's two subsections of section 8 before round 4 (those
  checks, and the depth table, by which every one-letter state is, up to K⁺, solved·(0, 12)) have no rows; round 4
  (COV-23 to COV-28) takes them up. It classifies the height-2 off placements up to K⁺ (COV-23), covers every singleton
  with a rim placement except at 24 exceptional holes (COV-24), excludes every retained middle that returns X's locking
  block exactly (COV-25); its bounded experiment (COV-28) found no reaching word. Round 5 (COV-29 to COV-35) covers the
  24 exceptional holes (COV-29), so every valid singleton with a rim placement has a step; cuts the other singletons by
  validity to 457,200 class–hole pairs (COV-30); and gives an exact joint test for a whole-cap step with several off
  pieces and a separation lemma (COV-32); its bounded experiment (COV-35) found no reaching word. Round 6 (COV-36 to
  COV-43) reduces the singleton residual exactly to 118 cases (COV-36), shows that the vacant-lock condition is no
  invariant of reachable states (COV-37), and proves that restoration inside height 2 (R2) gives B′(2) with an explicit
  catalogue (COV-39) and that HRH is R2 together with a lattice bridge (COV-40); its bounded experiment (COV-43) is
  running. Open, as COV-42 states them: SingletonResidual (the 118 cases of COV-36, in 67 deep classes, X's and COV-26's
  among them, and the outer classes 1345 and 1437), MultiCover (every state of 𝒞₂ with two or more off pieces has a
  step) and MixedCoreBound (a bound for any further native cores the proof needs), or R2 in place of all three; and
  whether COV-01's state is reachable (COV-14, COV-20, COV-25, COV-31, COV-37).
- Results recorded after this page's last update are not in the tables; they get rows when they are recorded.
