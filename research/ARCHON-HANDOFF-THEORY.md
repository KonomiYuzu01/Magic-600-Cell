# Handoff to the Lean formalization: base theory of the Full 600-cell (draft)

Draft of 19 November 2026, for the Lean agent Archon. Formalization has not started: whether and when it starts, and
what it covers, wait for the owner's decision. This page is the index that work starts from, for the base mathematical
theory only. The jumbling and restoration theory has its own page
([`jumbling/restoration/12-low-height/ARCHON-HANDOFF.md`](jumbling/restoration/12-low-height/ARCHON-HANDOFF.md)).

The statements, hypotheses and proofs are in [`Full_600cell_Puzzle_Theory.tex`](Full_600cell_Puzzle_Theory.tex) (the
"supplement"), section "Mathematical development". This page adds what a formalization needs for its plan: the objects,
the 21 results in an order in which each comes after the results it uses, the hypotheses that are finite computations
and what decides them, and the results of the literature that the proofs rely on. It was extracted from the supplement
at commit b322cae9 and checked against the file's headings; line numbers refer to that file at that commit. What the scripts
decide was read from `audit/` ([README](audit/README.md)). Ids that the supplement does not give (FC, DS,
S1 to S3, O1 to O18, HR to HE, N1 to N13, E1 to E10) are labels of this page and are marked "(inferred)" where they
first occur. Where a use between results is not written in the proof, the table says so.

## 1. Layers

- **Geometry over Z[φ]** (5 results, layer "geometry"): L1 (the 120 vertices, the 600-cell), L2 (radial topology of the
  boundary), L3 (faces, links, duality), FC (the finite checker and its scope, label inferred), L5 (cell-pole
  normalization). Exact arithmetic in Z[φ] with φ² = φ + 1 and quaternions over it. L2 is the one place with real
  topology (radial homeomorphism to S³, integral homology, van Kampen).
- **Geometric symmetry** (2 results, layer "symmetry"): L4 (|K⁺| = 7200, |K| = 14400, the Coxeter presentation of
  W(H₄)) and DS (the group-role table and the covering SO(4); label inferred, heading "Distinct symmetry and motion
  groups"). They need the geometry layer.
- **The retained model** (4 results, layer "model"): L6 (cap actions, signatures, piece equivariance), L7 (transported
  frames and the ambient wreath product), L8 (necessary invariants and the upper bound), L12 (the geometric symmetries
  normalize the legal group). They are statements about the puzzle once the hypotheses R, F, I and E (section 4) are
  taken as given; none follows from the geometry layer alone.
- **Finite permutation groups and graphs** (8 results, layer "group"; 1 more, "group + computed"): S1 to S3 (scope
  tests, labels inferred), L9 (pure three-cycles give alternating groups), L10 (balanced-color lower bound), L13
  (conditions for sound symmetry reduction), L14 (complete endpoint transport), L15 (guarded setup graphs and legal
  inverses), and L11 (word-ball and total-variation bounds, which also needs two exact integer inequalities).
  Statements about abstract permutation groups, with hypothesis C as an input.
- **The combined theorem** (1 result, layer "package"): T, "Full 600-cell theory package", seven parts.
- **Computed facts**: finite checks listed in section 4 (counts, ranks, census sums, exact big-integer inequalities).
  Results with a computed ingredient: FC, L3 (counts), L4 (lower count), L11 (inequalities), L8 (the constant
  2⁴³·5²). A formalization either re-derives them by `decide` or an exact-arithmetic computation, or states them as
  computational facts and marks them.
- **Not for formalization**: the implementation, rendering, performance and provenance sections (section 6).

## 2. Objects

Defined in the supplement; line numbers are those of the defining text. "Decided by" names the script that computes the
object (all scripts are in `research/audit/`).

| Object | Name | Section, line | Decided by |
| --- | --- | --- | --- |
| O1 (inferred) | φ, τ = φ⁻¹, the ring Z[φ], the pair arithmetic | Definitions, 274; checker, 619 | verify_regular_geometry.py: add, mul, positive, dot |
| O2 (inferred) | V: the 120 vertices (three families, radius 2) | Definitions, 304; Lemma 1, 344 | verify_regular_geometry.py: verify (set construction, norms) |
| O3 (inferred) | I*: the binary icosahedral group, quaternion product | Definitions, 284 | verify_regular_geometry.py: qmul, closure loop over V × V |
| O4 (inferred) | P = conv(V), tetrahedra, edges, links, the facet set | Lemma 1, 381; Lemma 3, 438 | verify_regular_geometry.py: verify (edges, triangles, 600 tetrahedra) |
| O5 (inferred) | L_{p,q}, J, K⁺, K, W(H₄) presentation | Lemma 4, 525 and 535 | verify_regular_geometry.py: rotation_maps, reflection_maps (7200 + 7200 distinct) |
| O6 (inferred) | pole convention n₀, scale 4/φ, similarity | Lemma 5, 696 | — |
| O7 (inferred) | caps U_c, α = 121/125, 300 antipodal axes, layers | Retained model, 739 | — |
| O8 (inferred) | chambers, surface pieces, M(A), Host(A), sticker slot (M(A), h) | Retained model, 753 to 771 | — |
| O9 (inferred) | the 35-orbit census table (N_o, k_o, H_o) | Retained model, 786 | verify_extended_derivations.py: rows; verify_color_bound.py: census |
| O10 (inferred) | sticker set S, ρ: S → pieces, A₄ at a cap, H_c, T_c, G | Lemma 6, 889; Lemma 7, 956 | — |
| O11 (inferred) | transported frames, H_o^N ⋊ S_N, product law (p,a)(r,b) | Lemma 7, 958 to 964 | — |
| O12 (inferred) | the invariants Φ_o, positional sign, H_o^ab | Lemma 8, 1022 | verify_extended_derivations.py: commutator images of D₅, A₅ |
| O13 (inferred) | the pure star t_C = (A B C), relocation, setup S | Hypothesis C, 851; Lemma 9, 1095 | verify_color_bound.py: audit |
| O14 (inferred) | alphabet A (1800 letters), q = 1800, word metric | Lemma 11, 1191 | — |
| O15 (inferred) | B, Ω (reachable fixed-frame color states) | Lemma 10, 1138 to 1146 | verify_extended_derivations.py: lower (exact) |
| O16 (inferred) | the allowed state graph Γ, goal set, quotient by L | Lemma 13, 1391 | — |
| O17 (inferred) | complete nonfixed map E_W of a permutation | Lemma 14, 1435 | — |
| O18 (inferred) | guarded setup graph on ordered target frames, E₊, E± | Lemma 15, 1486 | — |

## 3. Results in dependency order

"Uses" lists the results of this page that the proof uses, "Used by" the reverse. External results are in section 5,
hypotheses in section 4. Statuses are those the supplement states: "proved" for a proof in the text, "conditional on"
for a proof that uses the named hypothesis of section 4, "computed" for a finite check. Where the text proves a fact
inline that a later result also names (S3 in L9), the note under the table says so.

| # | Id | Result | Status as stated | Layer | Uses | Used by | Section, line |
| ---: | --- | --- | --- | --- | --- | --- | --- |
| 1 | L1 | The 120 elements of V have norm 2; V/2 = I*; conv V is a regular 600-cell | proved; uses E1, E2 | geometry | — | L2, L3, FC, L4, L5, T | Lemma 1, 339 |
| 2 | L2 | ∂P ≅ S³; homology Z, 0, 0, Z; trivial π₁ | proved | geometry | L1 (a) | L4, T | Lemma 2, 384 |
| 3 | L3 | Circumradius 2, edge 2/φ, f = (120,720,1200,600), icosahedral links, {3,3,5}, dual {5,3,3} | proved; counts computed | geometry | L1 | L4, L5, T | Lemma 3, 433 |
| 4 | FC | Supporting-facet test (N·v_i = 4 + 6φ, ‖N‖² = 16 + 24φ), exact algorithm and completeness of the facet list | proved; run computed | geometry | L1 (a) | L5, T | Finite checker, 615 |
| 5 | L4 | K⁺ ≅ (I*×I*)/⟨(−1,−1)⟩ of order 7200; K of order 14400; K ≅ W(H₄) with the chain presentation | proved; uses E5 | symmetry | L1, L2, L3 | DS, L12, T | Lemma 4, 520 |
| 6 | L5 | Inradius φ²/√2; the similarity of scale 4/φ puts a pole at n₀ = (φ³,1,1,1) | proved | geometry | L1, L3, FC | L12, T | Lemma 5, 691 |
| 7 | L6 | H_c, T_c generate A₄; ρ is equivariant; signatures address stickers; r(A), k(A) | proved under R, F | model | — | L7, L12, L15, T | Lemma 6, 883 |
| 8 | L7 | G embeds in ∏_o (H_o^{N_o} ⋊ S_{N_o}) with the chronological product law | proved under R | model | L6 (b) | L8, T | Lemma 7, 950 |
| 9 | L8 | |G| ≤ ∏ N_o! \|H_o\|^{N_o} / (2⁴³·5²) | proved under R, I | model | L7 | T | Lemma 8, 1006 |
| 10 | S3 | Permutations with disjoint full supports commute | proved | group | — | L9 (c), T | Scope tests, 1548 |
| 11 | L14 | The nonfixed map of S⁻¹WS is the transport of E_W under S | proved | group | — | L9 (c), T | Lemma 14, 1428 |
| 12 | L9 | Pure stars generate A₃₆₀₀ × A₂₄₀₀ × A₇₂₀₀⁷ inside G | proved under R, C | group | S3 (c), L14 (c) | L10, T | Lemma 9, 1082 |
| 13 | L10 | The balanced-color count B is a lower bound for \|Ω\| | proved under R, C | group | L9 | L11, DS, T | Lemma 10, 1133 |
| 14 | L11 | Some state has distance ≥ 46648; TV distance from uniform after 1000 letters > 1 − 10⁻¹⁴⁸⁵⁹⁵ | proved under R, C; inequalities computed | group + computed | L10 | L13 (d), DS, T | Lemma 11, 1186 |
| 15 | DS | Group roles; SO(4) is covered by Sp(1)×Sp(1); B > 10^151851 separates G from K | proved (covering); remark | symmetry | L4, L10 (e) | T | Distinct groups, 1318 |
| 16 | L12 | K normalizes G; G ∩ K = 1; ⟨G,K⟩ ≅ G ⋊ K | proved under R, F, E | model | L4, L5, L6 | L13, T | Lemma 12, 1267 |
| 17 | L13 | Normalization does not preserve the word metric; when a quotient preserves distances | proved | group | L12 (d), L11 (d) | T | Lemma 13, 1362 |
| 18 | L15 | Adding legal inverses cannot raise a setup length on the guarded frame graph | proved | group | L6 (b) | T | Lemma 15, 1470 |
| 19 | S1 | Equal restrictions to a target orbit, different full actions | proved (example) | group | — | T | Scope tests, 1528 |
| 20 | S2 | Disjoint visible supports, noncommuting full actions | proved (example) | group | — | T | Scope tests, 1539 |
| 21 | T | Full 600-cell theory package, parts 1 to 7 | proved; each part under its stated hypotheses | package | L1 to L15, FC, DS, S1 to S3 | — | Theorem, 1557 |

Notes on "Uses":

- (a) The proofs of L2 (the ball inside P, which uses the vertices ±2e_i) and of FC (the three vertex families) use the
  coordinates of V from L1 without citing it (implicit).
- (b) L7 uses that legal moves preserve each piece orbit, which is the content of L6 (implicit). L15 uses the cap
  signatures M(A), M(B) and the orders of H_c and T_c from L6 and F (implicit).
- (c) L9 proves inline that the nine supported subgroups commute and that S⁻¹t_CS = t_X; these are S3 and the formula of
  L14 for a pure three-cycle. The supplement refers to them only as "the complete-support hypotheses in Lemmas 9 and 14"
  (line 1555); the dependency is implicit.
- (d) L13's statement is about the 1800-letter alphabet of L11 and the normalization of L12; the proof uses them as
  context and does not use their conclusions (inferred).
- (e) DS has two parts: the covering of SO(4) (uses L4's kernel calculation, stated at line 1355) and the remark
  B > 10^151851 (line 1359, uses L10).
- T's proof (1646) names, per part: part 1 uses L1 to L3 and FC; part 2 L4; part 3 L5 and L6; part 4 L7 to L10; part 5
  L11; part 6 L12 and L13; part 7 L14, L15 and the scope tests. DS is used for the distinction of the groups in part 2
  (inferred).
- Hypotheses used: R for L6 to L12, F for L6, L12, L15 (implicit), I for L8, C for L9 to L11, E for L12.

## 4. Hypotheses that are finite computations, and numeric facts

Hypotheses of the supplement (lines 776 to 877) and what decides them. R, F and E describe the retained subdivision,
which the supplement states it does not reconstruct (line 880: no independent derivation of the 433 regions per
tetrahedron is claimed).

| Id | Hypothesis or fact | Decided by |
| --- | --- | --- |
| HR (inferred), 776 | Census: 35 orbits, ΣN_o = 176520, 600 + Σk_oN_o = 259800, 177120 pieces, 259800 slots; faithful frame groups H_o | Census and group shape: verify_extended_derivations.py (rows) and verify_color_bound.py (census, stored local groups), reading `assets/census.json`. The arithmetic sums are also plain `decide` (checked by hand for this page). The subdivision itself, "no hidden identification" and legal moves carrying pieces to pieces: no script |
| HF (inferred), 839 | Every cap admits the full A₄ on the same subdivision | No script in `audit/` |
| HI (inferred), 844 | Every chosen primitive has even positional permutation in every orbit and zero orientation image | Not decided by any script in `audit/`: the README states that neither checker reruns the all-generator transported-frame audit; the supplement calls it "retained evidence" |
| HC (inferred), 851 | Nine pure-controller certificates for orbits {1,4,5,7,18,19,27,31,33}: a word that is exactly a three-cycle, a relocation, and a buffer-fixing setup to every other position | verify_color_bound.py (full-word replay on all 259800 labels, relocation, positive-move reachability), results in `audit/results.json`; needs the separate archive `Full_600cell_Curated_Reference_and_Replay.zip` and `assets/model.npz` |
| HE (inferred), 866 | K acts geometrically on the whole subdivision, centers at the poles, faithful | No script in `audit/` |
| N1 (inferred) | 120 vertices; 720 edges; 1200 triangles; 600 tetrahedra; 5 per edge, 20 per vertex; links (12,30,20) | Computed by verify_regular_geometry.py (asserts); proved in L1 and L3 |
| N2 (inferred) | |K⁺| = 7200, |K| = 14400 | Proved (L4, the upper bound needs L3's link). The script only counts 7200 + 7200 distinct maps (the lower bound) |
| N3 (inferred) | Mod-2 boundary ranks (119, 601, 599), Betti numbers (1,0,0,1); vertex and dual graph distance histograms (diameters 5 and 15) | Computed by verify_regular_geometry.py. Not used by any proof (line 1578 says so for the ranks); the distance histograms are not in the supplement (inferred) |
| N4 (inferred) | 35 orbits: 7 orbits with C₂, 1 with D₅, 2 with C₅, 1 with A₅, 24 trivial; hence 35 + 7 + 1 = 43 exponent of 2 and 5² | Counted by hand for this page from the table at 786; verify_extended_derivations.py gives the divisor 2⁴³·5² and the per-orbit orientation order |
| N5 (inferred) | [D₅,D₅] has 5 elements; [A₅,A₅] = A₅ (60 elements) | Computed by verify_extended_derivations.py; proved in L8 |
| N6 (inferred) | Three-cycle identities (A Y X) and (x y z) on domains of size 4 to 9 | Computed by verify_extended_derivations.py; proved in general in L9 |
| N7 (inferred) | B > 10^151851; log₁₀ B ≈ 151851.164 | Exact integers, no logarithm: verify_extended_derivations.py (`assert lower > 10**151851`) and verify_color_bound.py (`exact_integer_comparison`). Statement at 1359 |
| N8 (inferred) | Equation 1218: 1800^46648 − 1 < 1799·B; cross-check 1228: 1800^46649 − 1 ≥ 1799·B | Exact: verify_extended_derivations.py (bisection, depth 46647 below and 46648 reaching B). The text says "confirmed by the accompanying exact-integer audit" |
| N9 (inferred) | Equation 1220: 1800^1000 · 10^148595 < B | Exact: verify_extended_derivations.py (assert) |
| N10 (inferred) | The ambient upper count and its constrained form (logarithms ≈ 598075.04 and 598060.70) | Computed (verify_extended_derivations.py, verify_color_bound.py). Not stated as a number in the supplement; only the formula at 1011 |
| N11 (inferred) | 600 distinct home symbols; 6 copies in O01, 4 in O33, 12 in each of the seven other selected orbits | Computed (verify_color_bound.py, uniform multiplicities); the copies per orbit also follow from N_o/600 |
| N12 (inferred) | Left unproved by the supplement: the exact order of G, sufficiency of the invariants, reachability | Open; nothing to formalize (lines 246, 1079, 1615) |
| N13 (inferred) | |Ω/L| ≥ |Ω|/|L| for a finite group L acting on Ω | Stated without proof at 1262 (orbit counting); easy |

What each script decides:

- `verify_regular_geometry.py`: builds the 120 vertices exactly in Z[φ], edges as pairs with inner product 2φ, triangles,
  tetrahedra, supporting-facet inequalities, incidence counts, links, F₂ boundary ranks, graph distance histograms,
  quaternion closure on V and the 7200 + 7200 symmetry maps. It does not decide the upper bound |K| ≤ 14400, the
  topology or the Coxeter presentation.
- `verify_color_bound.py`: for the nine selected orbits, independently rebuilds primitive permutations and replays the
  complete raw words on all 259800 labels; checks one pure three-cycle, the relocation, positive-move reachability with
  both buffers fixed, uniform color multiplicities, stored local groups, and the exact comparisons B > 10^151851 and
  B > U · 10^86155 for a 4⁷ comparison bound. It does not regenerate the cuts or check every generator invariant.
- `verify_extended_derivations.py`: per-orbit census agreement, seed identity and tree depth structure, the invariant
  divisor, the exact word-ball bisection (46648), the total-variation inequality, small-domain cycle identities and the
  D₅ and A₅ commutator images. It does not rerun the nine-controller audit or the all-orbit transition checks.

## 5. Results of the literature and earlier work the proofs rely on

| External result | Where cited in the supplement | Used by |
| --- | --- | --- |
| E1 (inferred): coordinates of the binary lift of the icosahedral rotation group (Baez, "From the Icosahedron to E8", arXiv 1712.06436v2, the Icosians; unnumbered assertion) | Definitions, 304 to 319 | L1 |
| E2 (inferred): the binary icosahedral subgroup is an orbit of the reflection group of type H₄ and its 120 elements are the vertices of a regular 600-cell (Choi and Lee, Symmetry 10 (2018) 326, Theorem 2) | Definitions, 321 to 334 | L1, L3 (regularity), L4, L5 (same inradius at each facet) |
| E3 (inferred): standard topology: CW structure of S³ and its homology; van Kampen; a continuous bijection from a compact space to a Hausdorff space is a homeomorphism | L2 proof, 422 to 427 | L2, T part 1 |
| E4 (inferred): standard polytope facts: regular means flag-transitive; every ridge of a convex 4-polytope lies in two facets; the facet adjacency graph is connected; polarity reverses face incidences | L3 proof, 489 to 517; FC, 663 to 668 | L3, FC |
| E5 (inferred): the Coxeter group with the chain presentation (3,3,5) is W(H₄); closed galleries in the dual cell complex of S³ are generated by backtracking and the rank-two polygon boundaries | L4 proof, 598 to 613 | L4 |
| E6 (inferred): SO(4) is covered by Sp(1)×Sp(1) via p, q ↦ (x ↦ pxq⁻¹); every rotation of the imaginary quaternions is conjugation by a unit quaternion | Distinct groups, 1350 to 1355 | DS |
| E7 (inferred): standard permutation group facts: every even permutation is a product of three-cycles; three-cycles are conjugate in A₅; stabilizer-orbit counting for colorings | L8 proof, 1053 to 1057; L9 proof, 1113 to 1122; L10 proof, 1157 to 1166 | L8, L9, L10 |
| E8: the 35-orbit algorithms paper and the curated replay archive (words, trees, census): the retained evidence for R, C and I | Hypotheses, 873 to 878; Reproduction, 2025 to 2028 | HR, HC, HI and hence L6 to L11, T |
| E9: Hypercubing community pages (grip theory, piece invariants, God's number, cut depths) and Vogan's lecture on Coxeter groups | Purpose, 186 to 203; Reading guide, 2065 to 2102 | Background only; no proof depends on them (E9 is not formalized) |
| E10: the Rethlas review (an informal AI proof check with verdict "correct", no gaps) | Purpose, 181 to 184; Reproduction, 2044 to 2059 | Provenance only. It is not a proof and not a source for Archon |

The supplement cites no Jordan-type theorem and no theorem on wreath products beyond the product law it proves in L7.
Every permutation group argument is proved in the text (E7 is standard background only, not cited to a source).

## 6. Not for formalization

- Lines 1 to 262: front matter, evidence levels, the table "Objects that must remain distinct" (a role table, no result).
- Lines 1694 to 2122 (everything from "From theory to implementation"): normalization remark (repeats L5), state
  representation and incremental updates, certified net maps and caches, search graph sizes
  (V_o = (N_o − 2)|H_o|, cost O(V_o b k)), rendering and instancing, filtering and picking, transactions, performance
  evidence (the 0.2.4 timings, the Amdahl model, f < 0.0736, the 30 fps target), reproduction and Rethlas provenance,
  reading guide. Marked in the text as engineering analysis and proposals, not implemented results (line 172).
- The workload shares at lines 1825 to 1832 (about 40.34 percent, 3.8 percent) and the web mesh counts at 1855 to 1861.
- What the supplement says it does not establish: the exact order of G, a presentation of G, reachability of all
  invariant-satisfying states, optimal solving, a derivation of the 433 regions per tetrahedron, a lower bound for each
  scramble (lines 246, 880, 1079, 1256, 1615).
- The extra bounds that appear only in `audit/` (log₁₀ U of the 4⁷ comparison, the workload bound 405945 stars and
  385797152 primitives): the scripts state them as derived from retained reports. The supplement does not state them
  (inferred), so they are not Archon targets unless the owner adds them.
- The hypotheses R, F, I, C and E themselves: Archon takes them as hypotheses of a structure; their truth for the real
  puzzle is evidence (section 4), not theory.

## 7. Suggested formalization order

Six milestones, each in the order of section 3.

1. **Permutation core (no hypotheses beyond C).** S1, S2, S3, L14, then L9 (as a theorem about an abstract permutation
   group containing pure three-cycles with buffer-fixing setups), L10, L11, and N5 and N6 as decidable checks. The
   computation: N7 to N9 are exact natural-number inequalities on B, 1800^46648 and 1800^1000 (numbers of about 150000
   digits); first test whether `decide` or `norm_num` with big-number kernel arithmetic handles them, otherwise state
   them with a recorded certificate (inferred; not tried).
2. **Z[φ] and the 120 vertices.** O1, O2, L1 (counts, norms, the three families; E1 and E2 as stated hypotheses), FC and
   the combinatorial part of L3 (N1). Decidable: the sets are finite; the script is the reference computation.
3. **Symmetry group.** O3, O5, L4 (orders 7200 and 14400, the kernel computation, the stabilizer bound from the
   icosahedral link), then DS part 1. Computation: the closure of V under quaternion product (the loop in
   `verify_regular_geometry.py`); the upper bound is a proof, not a check.
4. **Topology and the presentation.** L2, the Coxeter presentation of L4 (E3 and E5), L5 (an identity in Z[φ]). This is
   the heaviest milestone; the orders and the presentation may be postponed, and L5 does not depend on L2.
5. **The retained model as a structure.** A Lean structure with the census table (O9), the frame groups and the
   hypotheses R, F, I, C, E as fields; L6, L7, L8 (N4 and N5 as decidable checks), the model-level uses of L9 to L11
   (the nine orbits, N11). The census data are inputs read from `assets/census.json` through a checked table, not
   re-derived.
6. **Normalization and the package.** L12, L13, L15, then T as the conjunction of its seven parts, each stated under its
   own hypotheses. Part 1 and part 2 come from milestones 2 to 4, parts 4 and 5 from milestones 1 and 5.
