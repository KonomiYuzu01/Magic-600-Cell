# Handoff to the Lean formalization (draft)

Draft of 18 October 2026, for the Lean agent Archon. Formalization has not started: when it starts and what it covers
wait for the owner's decision ([E2 and the Lean plan](../../../../docs/wiki/decisions/owner-decisions-2026-10-17-jumbling-m2.md);
[B′(h)](../../../../docs/wiki/decisions/owner-decisions-2026-10-18-bprime-goal.md)). A passed formal verification is
one of the conditions for merging draft pull request #112
([owner decision, 17 October 2026](../../../../docs/wiki/decisions/owner-decisions-2026-10-17-jumbling-pr.md)); this page
is the index that work starts from.

The statements, hypotheses, proofs and bounds are in [`PROOFS-B.md`](PROOFS-B.md), written hypothesis by hypothesis
for this purpose. This page adds what a formalization needs for its plan: the objects, the 90 results of theory round
22 in an order in which each comes after the results it uses, the hypotheses that are finite computations and what
decides them, and the results of earlier stages and of the literature that the proofs rely on. It was extracted from
`PROOFS-B.md` at commit 2e94c53, extended by hand with the results recorded later (R22B-19 to R22B-29, R22C3-01 to
R22C3-06 and COV-01 to COV-16), and checked against the file's headings; line numbers are those of the file in the commit that last
updated this page.

## 1. Four layers

- **Group theory** (13 results, layer "group"): finite permutation groups and words, with no puzzle data, for example
  R22B-01 (certified three-cycles give a polynomial bound), R22B-05 (Schreier tables), R22B-12 (when a power of a
  word is a seed), R22B-19 (local Alt and separating words give a product of alternating groups) and R22B-25 (a
  finite classifier of the induced action). They come first and need Jordan's theorem on primitive groups with a
  prime cycle, Schreier's lemma, the normal subgroups of Alt(d) and Sym(d), and the simplicity and automorphisms of
  Alt(d) (section 5).
- **The model** (39 results, layer "model"; 17 more, "model + computed", also have computed hypotheses): statements
  about the puzzle's state model, defined once through the objects O1 to O11 (section 2): poses in K⁺ over Q(√5),
  letters, heights, steps, the SAFE-ALL group, ghosts and Bad sets, the class-cell model, blocks and types. The
  vacancy theorem R22B-29 (a finite equivalence for vacancy at a middle, without Alt) and the coverage results
  COV-03, COV-06, COV-08, COV-10, COV-12 and COV-15 belong here.
- **Computed facts** (15 results, layer "computed", and the computed hypotheses of the mixed ones): finite checks on
  stated states, decided by the scripts named in section 4. A formalization does not re-derive them by hand: either a
  checker whose soundness is proved reads the recorded data (R22B-16 states the soundness of `check_r22b10.py` for the
  certificates of R22B-10), or they stay stated computational facts, marked as such.
- **Specifications** (6): R22B-10 (implemented as E22-1b), R22B-11 (an audit), R22B-18 (not implemented), R22C3-05
  (an inventory of proposed routes), COV-11 (an audit of a heuristic) and COV-16 (experiment design); nothing to prove.

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
| O9 | blocks, locks and shadows | 7 (preamble), 1529 | blocks.Blocks |
| O10 | types | 7 (preamble), 1536 | regions.orbit_of |
| O11 | creators and partners | 7 (preamble), 1539 | — |

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
| 38 | R22C-G | Lemma G: one rotation per cell | proved; R22CF-01, X22C-01 | model | — | R22C-V, R22C-T1′, R22C-T, R22C-M, R22C2-01, R22C2-05, COV-01 | 7, 1644 |
| 39 | R22C-V | Type vectors and counts | proved; computed; R22C-04, R22CF-03, X22C-03, X22C-04 | model + computed | R22A-01, R22A-05, R22C-G | R22C-T1′, R22C2-05, COV-03, COV-04 | 7, 1746 |
| 40 | R22C-T1′ | Theorem T1′: land, vacate the landing chambers, unwind | proved; computed; R22CF-04, X22C-05 | model | R22C-G, R22C-V | R22B-23, R22C-T2′, R22C2-01, R22C2-03, R22C2-05, R22C-K, R22C3-02, R22C3-03, R22C3-04, COV-06 | 7, 1780 |
| 41 | R22B-23 | What the closure bounds give without Alt | proved; conditional | model | R22B-04, R22B-19, R22B-22, R22A-06, R22C-T1′ | R22B-29 | 3, 1029 |
| 42 | R22D-01 | Retained conjugation | proved | model | — | R22D-08, R22C-05, R22C-K, R22C3-02, R22C3-03 | 5, 1287 |
| 43 | R22D-02 | Six-letter commutators: the mask lemma | proved; computed | model + computed | — | — | 5, 1315 |
| 44 | R22D-03 | The endpoint contract of a lowering conjugate | proved | model | — | R22D-08 | 5, 1335 |
| 45 | R22D-04 | Free grips after lowering at the depth 100 stop | computed | model + computed | — | — | 5, 1357 |
| 46 | R22D-05 | Nested-set vacancy for a two-level step | proved | model | — | — | 5, 1371 |
| 47 | R22D-06 | Fixed occupied chambers rule out both whole-cap middles | computed; proved | model + computed | — | — | 5, 1387 |
| 48 | R22D-07 | Unlocking | proved; Lemmas B and C conditional | model | — | — | 5, 1401 |
| 49 | R22D-08 | A finite bound for a stated family | proved | model | R22D-01, R22D-03 | — | 5, 1417 |
| 50 | R22C-T | Locked chambers and Lemma T | proved as scoped; R22CF-02, R22C-01, X22C-02 | model | R22C-G | — | 7, 1667 |
| 51 | R22C-S | The Shadow lemma | proved; R22CF-02, X22C-02 | model | — | — | 7, 1705 |
| 52 | R22C-M | Partners do not repair N1 | proved; R22CF-03, X22C-03 | model | R22C-G | — | 7, 1733 |
| 53 | R22C-T2′ | One partner and T1′ | proved; conditional on its certificates; R22CF-03, X22C-03 | model | R22C-T1′ | R22C-K, R22C3-02 | 7, 1816 |
| 54 | R22C2-01 | The intrinsic Q-shadow | proved | model | R22C-G, R22C-T1′ | COV-01, COV-03, COV-04, COV-14 | 7, 1832 |
| 55 | R22C2-03 | One action for the edge-cycle family | proved for a stated family | model | R22C-T1′ | — | 7, 1852 |
| 56 | R22C2-05 | b06: the twelve landing rotations | computed; proved parts | model + computed | R22C-V, R22C-G, R22C-T1′ | — | 7, 1868 |
| 57 | R22C-FC | Theorem FC: no self-frozen set | proved; R22CF-05, X22C-07 | model | — | COV-03, COV-13, COV-15 | 7, 1884 |
| 58 | R22C-R1 | Lemma R1: the one-letter readiness test | proved; R22CF-05, X22C-07 | model | — | — | 7, 1900 |
| 59 | R22C-CH | Lemma CH: co-held cycles | conditional; R22CF-05, X22C-07 | model | — | — | 7, 1911 |
| 60 | R22C-05 | One-letter unlocking with vacancy | proved | model | R22D-01 | COV-01 | 7, 1921 |
| 61 | R22C-06 | A rigid two-domain cell cannot be unlocked by retained conjugation | proved | model | — | — | 7, 1934 |
| 62 | R22C-K | The catalogue, backward certification and coverage | conditional; R22C-07, R22CF-06, X22C-08, R22C2-06 | model + computed | R22C-T1′, R22C-T2′, R22D-01 | R22C3-06, COV-15 | 7, 1946 |
| 63 | R22C3-01 | Strict′ steps keep SAFE-ALL, not the hole inventories | proved | model | R22A-06 | R22C3-02, R22C3-04, R22B-28, COV-08 | 7, 2025 |
| 64 | R22C3-02 | Mixed orders need resource contracts as well as a blocker order | proved | model | R22C-T1′, R22C-T2′, R22D-01, R22A2-04, R22C3-01 | — | 7, 2046 |
| 65 | R22C3-03 | Experiment D passes an independent exact audit | computed | computed | R22C-T1′, R22D-01 | R22B-27 | 7, 2075 |
| 66 | R22C3-04 | 142, then 151: a proved prefix of 130's predecessors | proved from computed hypotheses | model + computed | R22C-T1′, R22C3-01 | R22B-27, COV-05 | 7, 2092 |
| 67 | R22C3-05 | The catalogue of unresolved units stays conditional | inventory | specification | — | — | 7, 2119 |
| 68 | R22C3-06 | A uniform core bound; coverage stays open | proved; conditional | model | R22C-K | COV-03, COV-15 | 7, 2139 |
| 69 | R22B-24 | A non-Alt orbit meets a catalogue target that has a hole; vacancy still holds | computed | computed | R22B-21, R22B-22 | — | 3, 1065 |
| 70 | R22B-25 | A finite classifier of the induced action, with a residual case | proved | group | R22B-15, R22B-19, R22B-22 | — | 3, 1084 |
| 71 | R22B-26 | The depth 60 home targets miss every small orbit | computed | computed | R22B-22 | — | 3, 1104 |
| 72 | R22B-27 | The T1′ middles of 151 and 180 miss the small orbits too | computed | computed | R22B-22, R22C3-03, R22C3-04 | — | 3, 1115 |
| 73 | R22B-28 | What a strict′ step keeps: subgroup certificates, not inventories | proved | model | R22C3-01, R22B-19, R22B-22 | — | 3, 1125 |
| 74 | R22B-29 | The vacancy theorem: a finite equivalence without Alt | proved | model | R22B-19, R22B-22, R22B-23 | COV-03, COV-08, COV-10, COV-12 | 3, 1143 |
| 75 | COV-01 | An isolated triangle with an intrinsic Q-shadow | computed; proved; reachability open | model + computed | R22C2-01, R22C-G, R22C-05 | COV-14 | 8, 2183 |
| 76 | COV-02 | Seven geometric off-point cases | computed | computed | — | — | 8, 2210 |
| 77 | COV-03 | A local certificate, and what coverage still needs | proved; conditional | model | R22C2-01, R22C-V, R22C-FC, R22C3-06, R22B-29 | COV-15 | 8, 2225 |
| 78 | COV-04 | At 737, b05 and b15 are blocked by other units, b06 by two types | computed | computed | R22C-V, R22C2-01 | — | 8, 2256 |
| 79 | COV-05 | Unit 130 at depth 60 is blocked by seven units | from the record | computed | R22C3-04 | — | 8, 2278 |
| 80 | COV-06 | Coverage needs the holes and the off poses, not the lattice orientations | proved | model | R22C-T1′ | COV-08, COV-09, COV-10, COV-12 | 8, 2287 |
| 81 | COV-07 | The stored T1′ words form no order at 737 | computed | computed | — | — | 8, 2306 |
| 82 | COV-08 | T1′ resources pull back through the good-ghost map | proved | model | COV-06, R22C3-01, R22B-29 | COV-09, COV-10 | 8, 2349 |
| 83 | COV-09 | A strict′ predecessor turns N2 at a whole-cap middle from true to false | computed | computed | COV-06, COV-08 | COV-16 | 8, 2371 |
| 84 | COV-10 | SAFE preparation: an exact finite test and a flow criterion | proved | model | COV-06, COV-08, R22B-19, R22B-29 | COV-11, COV-12, COV-15 | 8, 2391 |
| 85 | COV-11 | prep2 is a heuristic, not a preparation test | audit | specification | COV-10 | — | 8, 2416 |
| 86 | COV-12 | Whole-cap cores: exact eligibility and a three-segment form | proved | model | COV-06, COV-10, R22B-29 | COV-13, COV-14, COV-15 | 8, 2429 |
| 87 | COV-13 | A reachable height-2 state with no all-content-landing cap | computed | computed | R22C-FC, COV-12 | COV-16 | 8, 2450 |
| 88 | COV-14 | The isolated triangle has whole-cap landers but no prepared whole-cap vacancy | computed; proved | model + computed | COV-01, COV-12, R22C2-01 | — | 8, 2465 |
| 89 | COV-15 | Coverage reduces to one finite per-state predicate | proved; conditional | model | COV-03, COV-10, COV-12, R22C3-06, R22C-K, R22C-FC | COV-16 | 8, 2485 |
| 90 | COV-16 | The adjudicating experiments are done; E5's inputs were wrong | experiment design | specification | COV-09, COV-13, COV-15 | — | 8, 2511 |

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

## 5. Results of earlier stages and of the literature

| External result | Where | Used by |
| --- | --- | --- |
| Proposition N | buffer-transport.md sections 2 and 3 (research/jumbling/theory/buffer-transport.md; named in O5) | R22A-07, R22A2-06, R22B-01, R22C-M |
| Theorem A | buffer-transport.md sections 2 and 3 (research/jumbling/theory/buffer-transport.md; named in O5) | R22B-01, R22B-04, R22B-07, R22B-08, R22B-15, R22B-19, R22B-23, R22C3-01, R22B-29 |
| Corollary A1 | buffer-transport.md sections 2 and 3 (research/jumbling/theory/buffer-transport.md; named in O5) | R22A2-04, R22A2-06, R22B-04, R22C3-04, COV-10 |
| Jordan certificate (global; Jordan word and prime) | buffer-transport.md sections 2 and 3 (research/jumbling/theory/buffer-transport.md; named in O5) | R22A2-01, R22A2-04, R22A2-05, R22B-01, R22B-04, R22B-05, R22B-06, R22B-07, R22B-08, R22B-09, R22B-10 |
| Lemma M (items (M1), (M2)) | not named in PROOFS-B | R22A2-04, R22C-M, R22C-T1′, R22C-T2′, R22C3-01, R22B-28, COV-06, COV-12 |
| Theorem B (peeling certificate) | not named in PROOFS-B | R22C-T1′ |
| Theorem C | not named in PROOFS-B | R22A-07, R22C-T1′, R22C-05, R22C3-04, R22C3-06, COV-04 |
| Theorem D (F2) | not named in PROOFS-B | R22C-FC |
| Proposition F | PROOFS-U.md (named in R22D-07; no path given) | R22D-07 |
| Lemma E6 | not named in PROOFS-B | R22D-01 |
| T1 | not named in PROOFS-B | R22C-K |
| lock lemma | buffer-transport.md section 5 (research/jumbling/theory/buffer-transport.md; named in O9) | R22C-T |
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
  reduces coverage to one finite per-state predicate, Cover(Z) (COV-15). Open: the lemma that every state of 𝒞₂ with
  off pieces satisfies it, and whether COV-01's state is reachable (COV-14).
- Results recorded after this page's last update are not in the tables; they get rows when they are recorded.
