# Full 600-cell base theory in Lean

A Lean 4 and Mathlib project for the base theory in [`research/ARCHON-HANDOFF-THEORY.md`](../../ARCHON-HANDOFF-THEORY.md). It holds Milestone 1, the permutation core, Milestone 2, the ring `ℤ[φ]` and the 120 vertices, and Milestone 3, the finite symmetry group, with every proof complete. Milestones 4 to 6 are not started.

No file has a `sorry`, an `admit`, a `native_decide` or a new axiom. `#print axioms` on every theorem named below lists only `propext`, `Classical.choice` and `Quot.sound`.

## Milestone 1: the permutation core

All 16 Milestone 1 theorems in `ArchonFull600Theory/Basic.lean` are proved.

| Handoff | Lean theorem | Content |
| --- | --- | --- |
| S1 | `scopeSameRestriction` | Two permutations of six labels agree on a three-label target orbit but differ as full actions. |
| S2 | `scopeVisibleDisjoint` | Two permutations of seven labels have disjoint visible supports but do not commute. |
| S3 | `disjointFullSupportsCommute` | Permutations with disjoint full supports commute. |
| L14 | `transportNonfixedMap` | The nonfixed map of `S⁻¹ W S` is the transport of the nonfixed map of `W` under `S`. |
| L9 | `supportedAlternatingActions` | For a permutation group `G` with nine pairwise-disjoint preserved orbits of sizes 3600, 2400 and seven times 7200, each with a pure-controller certificate, there is an injective homomorphism from the product of the nine alternating groups into `G` that acts as each factor on its orbit and fixes every other slot. |
| L10 | `balancedColorLowerBound` | With 600 colors and balanced solved copies (6, 4 and 12 per color), the balanced-color count `B` bounds the supported colorings and the reachable colorings from below. |
| L11 | `wordBallCount` | The ball of radius `r` in the word metric of a `q`-letter alphabet has at most `(q^(r+1) - 1)/(q - 1)` colorings. |
| L11 | `wordDistanceLowerBound` | For the 1800-letter alphabet of the 600 caps, some reachable coloring has word distance at least 46648. |
| L11 | `totalVariationLowerBound` | Any law supported on colorings reached by words of exactly 1000 letters has total variation distance greater than `1 - 10^(-148595)` from the uniform law. |
| N5 | `dihedralFiveCommutatorCard`, `alternatingFivePerfect` | `[D₅, D₅]` is the rotation subgroup with 5 elements; `[A₅, A₅] = A₅` with 60 elements. |
| N6 | `threeCycleIdentityChecks` | `t_Y⁻¹ t_X = (A Y X)` and `(A x y)(A z x) = (x y z)` in chronological order, on every domain of size 4 to 9. |
| N7 | `balancedColorCountGtPower` | `10^151851 < B`. |
| N8 | `radius46647Comparison`, `radius46648CrossCheck` | `1800^46648 - 1 < 1799 B` and `1799 B ≤ 1800^46649 - 1`. |
| N9 | `thousandLetterSupportComparison` | `1800^1000 · 10^148595 < B`. |

N7 to N9 depend on `propext` alone.

### How the Milestone 1 proofs work

- N7 to N9 are exact natural-number comparisons on numbers of about 150,000 digits. They are proved by `decide +kernel`, which the kernel checks with its arbitrary-precision arithmetic. `native_decide` is not used, so no compiler trust enters.
- L9 builds the homomorphism with `MonoidHom.noncommPiCoprod` from the nine permutations extended by the identity. Each orbit's three-cycles are produced from the certificate's controllers and buffers, and they generate the alternating group by `Equiv.Perm.closure_three_cycles_eq_alternating`.
- L10 counts each orbit's balanced recolorings by orbit-stabilizer for the alternating group (the stabilizer of a coloring with at least one repeated color contains an odd permutation), and multiplies the nine counts through L9.
- L11 compares the word ball at radius 46647 with `B` through N8, and bounds the support of the 1000-letter law by `1800^1000` through an injection into length-1000 vectors and N9.

### Changes to the round-19 statements

No theorem's proposition was changed. Each statement was checked against the blueprint chapter and the handoff; see the pull request that added the proofs.

- The starter definition `hello` and its blueprint section were removed.
- Eleven Mathlib modules were added to the imports the proofs need.
- `wordBallCount` gained an `omit` line that drops four instance arguments the statement never used, as the linter reported. The theorem is now slightly more general.
- Linter options (`set_option linter.… false in`) were added where the round-19 scaffold includes hypotheses that a proof does not use, for example `hPreserved` in L10 and `hRealizes` in the total-variation bound, and where `threeCycleIdentityChecks` holds for every `n`, so its bounds `4 ≤ n ≤ 9` go unused.
- In the blueprint chapter, each proved result's proof is marked `\leanok`.

## Milestone 2: the ring `ℤ[φ]` and the 120 vertices

The handoff's scope is O1, O2, Lemma 1 (counts, norms and the three families, with the external results E1 and E2 as stated hypotheses), the finite checker's supporting-facet test and completeness argument (FC), and the combinatorial part of Lemma 3 (N1). The statements and proofs were written in this session from `research/Full_600cell_Puzzle_Theory.tex` and the handoff; the blueprint chapter `blueprint/src/chapters/ArchonFull600Theory_Geometry.tex` quotes the source lines for each.

| Handoff | Lean theorem | Module | Content |
| --- | --- | --- | --- |
| O1 | `goldenRatioIdentities` | `GoldenInt` | `φ² = φ + 1`, `τ = φ⁻¹ = φ - 1`, `τ² = 2 - φ`. |
| O1 | `ZPhi.pairArithmetic`, `ZPhi.toReal_mul`, `ZPhi.toReal_injective` | `GoldenInt` | The checker's pair product `(a, b)(c, d) = (ac + bd, ad + bc + bd)` is the product of `ℤ[φ]`, and distinct pairs are distinct reals. |
| O1 | `ZPhi.positive_iff` | `GoldenInt` | The checker's sign test, through `2(a + bφ) = (2a + b) + b√5` with the sign cases handled before squaring, accepts a pair exactly when its value is positive. |
| O2 | `vertexSet`, `vertexTable`, `range_vertexR` | `Vertices` | `V` as the three families with even coordinate permutations and independent signs, and the same 120 points as a table over `ℤ[φ]`. |
| L1 | `vertexFamilies`, `vertexCount`, `vertexNorm` | `Vertices` | The families have 8, 16 and 96 elements and are pairwise disjoint; `V` has 120 elements, each of norm 2. |
| L1 | `coordinateIdentification` | `Vertices` | Under `CoordinateSources` (E1, E2 and scaling), `V/2` lies in `Sp(1)` and is binary icosahedral, and `conv V` is a regular 600-cell. |
| L3 | `exposedVertices` | `EdgeGraph` | Every vertex has norm 2, so all vertices lie at distance 2 from the centre; the exposed points and the extreme points of `P = conv V` are exactly the 120 points of `V`. |
| L3 | `edgeLength` | `EdgeGraph` | Distinct vertices are at distance at least `2/φ`, with equality exactly for the pairs with inner product `2φ`. |
| N1 | `faceCounts` | `EdgeGraph` | The edge graph has 720 edges, and 120, 720, 1200 and 600 cliques of sizes 1 to 4. |
| N1 | `vertexLinkCounts`, `incidenceCounts` | `EdgeGraph` | Each vertex has degree 12 and lies in 12 edges, 30 triangles and 20 four-cliques (link counts 12, 30, 20); each edge lies in 5 triangles and 5 four-cliques; each triangle in 2 four-cliques. |
| N1 | `eulerCharacteristic` | `EdgeGraph` | `120 - 720 + 1200 - 600 = 0`. |
| FC | `supportingFacet` | `Facets` | For a four-clique with vertex sum `N`: `N·vᵢ = 4 + 6φ`, `‖N‖² = 16 + 24φ`, `N·v < 4 + 6φ` at the other 116 vertices, and `N·x ≤ 4 + 6φ` on `P` with equality exactly on the tetrahedron of the clique. |
| L3 | `cliqueFace` | `Facets` | For an `n`-clique with vertex sum `N`, `N·x ≤ 4 + 2(n - 1)φ` on `P` with equality exactly on the hull of the clique: graph edges, triangles and four-cliques span exposed faces of `P`. |
| FC | `facetGram`, `finrank_cliqueTetra` | `Facets` | The Gram matrix of the four vertices has diagonal 4 and off-diagonal `2φ` and is positive definite; the tetrahedron has affine dimension 3. |
| FC | `isFacet_cliqueTetra`, `facetCompleteness` | `Facets` | Each such tetrahedron is a facet of `P`; under `FacetFacts` (E4), the facets of `P` are exactly these tetrahedra, and there are 600. |

### How the Milestone 2 proofs work

- Every finite fact is a computation over `ℤ[φ]` on the 120-row table, checked by `decide +kernel` and transferred to `ℍ` through the injective map `ZPhi → ℝ`. The adjacency of the 14,400 ordered pairs is stored as a 14,400-bit mask; a pair certificate, checked in eight blocks of 15 rows to stay within memory, shows that the mask records exactly the pairs with inner product `2φ` and that every other distinct pair has a smaller inner product by the exact sign test. A count certificate shows 12 common neighbours for every vertex, 5 for every edge and 2 for every triangle.
- The clique counts follow from those three numbers by double counting of nested cliques in an arbitrary finite graph (`cliqueCounts_of_commonNbrs`, `localCliqueCounts_of_commonNbrs`), so the kernel never enumerates the 1200 triangles or the 600 four-cliques.
- The face lemma `face_convexHull` shows that a functional bounded by `M` on a finite set cuts the convex hull in the hull of the points where it equals `M`. With the uniform Gram matrix it gives the supporting-facet test. A vanishing combination `∑ cᵢvᵢ` has squared norm `(4 - 2φ)∑cᵢ² + 2φ(∑cᵢ)²`, which gives positive definiteness and affine independence.
- Completeness follows the source: the ridge between a listed tetrahedron and an adjacent facet is the triangle of a three-clique, that triangle lies in exactly two four-cliques, so the adjacent facet is listed, and connectivity of the facet adjacency graph reaches every facet.

### Statements and hypotheses in Milestone 2

- E1 and E2 are not formalized: `CoordinateSources` takes two abstract predicates, "binary icosahedral" and "regular 600-cell", and states E1 (the binary icosahedral group is the displayed coordinate set) and E2 (a binary icosahedral group's convex hull is a regular 600-cell) as fields. A third field states that multiplication by 2 preserves regularity, which Lemma 1's proof asserts in words ("Multiplication by 2 preserves regularity and the face lattice") and which cannot be proved about an abstract predicate. Lemma 1's `V/2 ⊂ Sp(1)` and the identification of `V/2` with the coordinate set, including the even-permutation convention, are proved. The conclusions that `V/2` is binary icosahedral and that `conv V` is a regular 600-cell are carried entirely by these hypotheses, and the predicate "regular 600-cell" is not linked to the face results below. E1's words "in a suitable orthonormal quaternion coordinate system" are read, as Lemma 1's proof reads them, as the standard coordinates `1, i, j, k`.
- E4 is not formalized: `FacetFacts` states the two polytope facts the completeness argument uses, that a ridge lies in no facet other than the two that meet in it (the "at most two" half of "every ridge lies in two facets") and that the facet adjacency graph is connected. Both hold for every convex 4-polytope. A facet is defined as a face cut out by a bounded linear functional with affine dimension 3, and adjacency as meeting in affine dimension 2.
- `V` is defined from the three coordinate families that Lemma 1's proof names, `(±2, 0, 0, 0)`, `(±1, ±1, ±1, ±1)` and `(0, ±1, ±φ, ±τ)` with even permutations, which is the family order of `verify_regular_geometry.py`; the table lists the same points in the script's order.
- `f₁`, `f₂` and `f₃` are stated as the numbers of edges, triangles and four-cliques of the edge graph. `cliqueFace` shows that each of them spans an exposed face of `P`, and FC identifies the four-cliques with all the facets under E4. The proofs do not show that every 1-face and 2-face of `P` is a graph edge or triangle.
- Not in Milestone 2 and not formalized: regularity as flag-transitivity, the icosahedral geometry of the vertex links (only their counts 12, 30, 20), the Schläfli symbol `{3,3,5}`, the polar dual 120-cell and the ranks over `F₂` (step 6 of the checker). The quaternion closure and the symmetry counts 7200 and 14400 (step 7) are in Milestone 3.

## Milestone 3: the finite symmetry group

The handoff's scope is O3, O5, Lemma 4 without its Coxeter presentation (the orders 7200 and 14400, the kernel computation and the stabilizer bound from the icosahedral link) and the first part of DS, the double cover of `SO(4)` by `Sp(1) × Sp(1)`. The statements and proofs were written in this session from `research/Full_600cell_Puzzle_Theory.tex` and the handoff; the blueprint chapter `blueprint/src/chapters/ArchonFull600Theory_Symmetry.tex` quotes the source lines for each.

| Handoff | Lean theorem | Module | Content |
| --- | --- | --- | --- |
| O3 | `binaryIcosahedral`, `binaryIcosahedralGroup`, `halfVertexSet_closure` | `BinaryIcosahedral` | `V/2` contains `1` and `-1` and is closed under the quaternion product and conjugation, so it is a subgroup `I*` of `Sp(1)` with 120 elements. |
| O5 | `lpq`, `lpqHom`, `conjIso`, `conjIso_lpq_conjIso` | `QuaternionMaps` | `L_{p,q}(x) = p x q⁻¹` for unit quaternions `p, q` and `J(x) = x̄` are orthogonal maps of `ℍ`; `(p, q) ↦ L_{p,q}` is a homomorphism; `J L_{p,q} J = L_{q,p}`. |
| L4 | `lpq_eq_one_iff`, `ker_lpqHom` | `QuaternionMaps` | `L_{p,q}` is the identity exactly for `(p, q) = (1, 1)` and `(-1, -1)`. |
| L4 | `orthDet_lpq`, `orthDet_conjIso` | `QuaternionMaps` | `det L_{p,q} = 1` and `det J = -1`. |
| DS | `orthogonal_eq_lpq`, `doubleCoverSO4` | `QuaternionMaps` | Every orthogonal map of `ℍ` is `L_{p,q}` or `L_{p,q} J`; `(p, q) ↦ L_{p,q}` maps `Sp(1) × Sp(1)` onto `SO(4)` with kernel `{(1, 1), (-1, -1)}`. |
| O5 | `symmetryGroup`, `rotationGroup`, `mem_symmetryGroup_iff`, `isometry_mem_symmetryGroup` | `Symmetry` | `K` is the group of orthogonal maps with `A(P) = P`, equivalently `A(V) = V`, and `K⁺ = K ∩ SO(4)`; every isometry `f` of `ℍ` with `f(P) = P` is an element of `K`. |
| L4 | `card_stabilizer_le`, `card_symmetryGroup_le` | `Symmetry` | The stabilizer of a vertex has at most `20 · 6 = 120` elements; `K` is finite with at most `120 · 120 = 14400` elements. |
| L4 | `finiteSymmetryGroup`, `rotationGroupEquiv`, `mem_symmetryGroup_iff_lpq` | `Symmetry` | `K⁺ ≅ (I* × I*) / ⟨(-1, -1)⟩` through `(p, q) ↦ L_{p,q}`, `|K⁺| = 7200`, `J ∈ K` but `J ∉ K⁺`, `|K| = 14400`; every element of `K` is `L_{p,q}` or `L_{p,q} J` with `p, q ∈ I*`. |

### How the Milestone 3 proofs work

- Closure of `V/2`: the quaternion product of two table vertices is computed exactly over `ℤ[φ]`. Two kernel-checked tables show that left multiplication by vertices 8 and 30 maps `V/2` into itself. A tree of words in these two vertices (a parent table, a last-letter table and word lengths that decrease toward the root, all kernel-checked) shows that every element of `V/2` is such a word, so left multiplication by every element maps `V/2` into itself. A third table gives closure under conjugation. The kernel checks 240 products and 120 conjugations instead of the script's loop over all 14,400 products.
- Determinants are computed from the matrices in the basis `1, i, j, k`: left multiplication by `a` has determinant `|a|⁴`, `J` is `diag(1, -1, -1, -1)`, and `L_{p,q}(x) = p · conj(q · conj x)` has determinant `1`. The source argues `det L_{p,q} > 0` by continuity instead.
- The classification of orthogonal maps uses Mathlib's Cartan–Dieudonné theorem (`LinearIsometryEquiv.reflections_generate_dim`): each reflection is `x ↦ -u x̄ u = L_{u,-ū} J(x)` for a unit quaternion `u`, and the maps of the forms `L_{p,q}` and `L_{p,q} J` are closed under composition. This proves the `SO(4)` half of E6 instead of assuming it.
- The upper bound follows the source. `K` acts on the 120 vertices and preserves the edge graph. A symmetry fixing vertex `0` is determined by the images of vertices 30, 31 and 32, because the four vertices of the facet `{0, 30, 31, 32}` are linearly independent (Milestone 2's positive definite Gram matrix) and so form a basis of `ℍ`. The images form an ordered triangle among the twelve neighbours of vertex `0`, and a kernel count finds 120 such triangles. Orbit–stabilizer (`MulAction.index_stabilizer`, `Subgroup.card_mul_index`) with an orbit of at most 120 vertices gives that `K` is finite with `|K| ≤ 14400`.
- The lower bound: on `I* × I*` (14,400 elements) the kernel of `(p, q) ↦ L_{p,q}` has two elements, so its image in `K⁺` has 7200. The maps `A ↦ A` and `A ↦ A J` embed two disjoint copies of `K⁺` in `K` (determinants `1` and `-1`), so `2 |K⁺| ≤ |K| ≤ 14400`. Hence the image is all of `K⁺`, which gives the isomorphism, `|K⁺| = 7200` and `|K| = 14400`.

### Statements and hypotheses in Milestone 3

- `I*` is defined as the set `V/2` with the group structure of `Sp(1)`, and Lemma 4 is proved for this group. The source defines `I*` as the inverse image of the rotation group of a regular icosahedron under `Sp(1) → SO(3)`; that the two agree is E1, which stays a hypothesis (`CoordinateSources`, Milestone 2).
- `K` is defined as a group of orthogonal (linear isometric) maps of `ℍ`. `isometry_mem_symmetryGroup` shows that it contains every rigid symmetry of `P`: an isometry that maps `P` onto itself fixes the centre `0` and is linear by the Mazur–Ulam theorem. `SO(4)` is the group of orthogonal maps of `ℍ = ℝ⁴` (basis `1, i, j, k`) with determinant `1`.
- DS part 1 is formalized as a statement about groups: `(p, q) ↦ L_{p,q}` is a surjective homomorphism from `Sp(1) × Sp(1)` to `SO(4)` with kernel `{(1, 1), (-1, -1)}`. That it is a covering map of topological spaces, and the `SO(3)` half of E6 (every rotation of the imaginary quaternions is a conjugation), are not stated.
- Not in Milestone 3 and not formalized: the presentation `K ≅ W(H₄)` with its chain relations (Milestone 4, which uses E3 and E5), the transitivity of `K` on flags, and the second part of DS, the group-role table and the remark that `B > 10^151851` separates `G` from `K` (the inequality itself is N7, Milestone 1).

## Build

The toolchain is pinned in `lean-toolchain` (Lean v4.36.0), and `lakefile.toml` requires Mathlib at `v4.36.0`.

```
lake exe cache get
lake build
```

With Mathlib's build cache in place, `lake build` rebuilds only this library. `Basic.lean` takes under a minute; `EdgeGraph.lean`, the slowest module, takes about 50 seconds, most of it in the kernel checks of the pair and count certificates. The three Milestone 3 modules take 13 to 25 seconds each on an otherwise idle machine. The build passes with no errors and no linter warnings. Lake still notes, once for each import, that the imported Mathlib modules are designed for the module system; these files are not modules.

## How it was produced

- Milestone 1 statements: Archon 0.3.3, an automated Lean formalization loop run outside this repository, in iteration 19 of its `autoformalize` stage, with OpenAI GPT-6.2 Sol at effort `max` through the Codex CLI. The agent logs and Archon's state directory are not included.
- Milestone 1 proofs and all of Milestones 2 and 3: a Claude Code session (Claude Opus 5.5), without Codex calls.
