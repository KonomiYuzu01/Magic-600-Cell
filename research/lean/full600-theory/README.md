# Full 600-cell base theory in Lean

A Lean 4 and Mathlib project for the base theory in [`research/ARCHON-HANDOFF-THEORY.md`](../../ARCHON-HANDOFF-THEORY.md). It holds Milestone 1, the permutation core, Milestone 2, the ring `ℤ[φ]` and the 120 vertices, Milestone 3, the finite symmetry group, Milestone 4, topology and the Coxeter presentation, Milestone 5, the retained model, and Milestone 6, normalization and the theory package, with every proof complete. All six milestones of the handoff are done.

No file has a `sorry`, an `admit`, a `native_decide` or a new axiom. `#print axioms` on every theorem named below lists only `propext`, `Classical.choice` and `Quot.sound`.

The project also holds a second library, `Jumbling`: Lean formalizations of round-22 results of the jumbling restoration research, which import `ArchonFull600Theory.Basic`. See [`Jumbling/README.md`](Jumbling/README.md). `lake build` builds both libraries.

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
- Not in Milestone 3 and not formalized: the presentation `K ≅ W(H₄)` with its chain relations (Milestone 4), the transitivity of `K` on flags, and the second part of DS, the group-role table and the remark that `B > 10^151851` separates `G` from `K` (the inequality itself is N7, Milestone 1).

## Milestone 4: topology and the Coxeter presentation

The handoff's scope is Lemma 2, the radial topology of `∂P`, with the standard topology E3; the Coxeter presentation of Lemma 4, with E5; and Lemma 5, the inradius and the cell-pole normalization. The statements and proofs were written in this session from `research/Full_600cell_Puzzle_Theory.tex` and the handoff. The blueprint chapter `blueprint/src/chapters/ArchonFull600Theory_Topology.tex` quotes the source lines for each.

| Handoff | Lean theorem | Module | Content |
| --- | --- | --- | --- |
| L5 | `norm_facetPole`, `norm_sq_facetPole`, `facetPlane_iff` | `Poles` | The pole of a facet is `n = N/4`. It has `‖n‖² = (4 + 6φ)/4 = φ⁴/2`, so `‖n‖ = φ²/√2`. The facet plane `N · x = 4 + 6φ` is the plane `n · x = ‖n‖²`. |
| L5 | `polytope_eq_halfSpaces`, `isGreatest_inradius` | `Poles` | `P` is the intersection of the 600 facet half-spaces. `φ²/√2` is the largest radius of a closed ball about `0` inside `P`. |
| L5 | `cellPoleNormalization` | `Poles` | Lemma 5. It contains the facts above, and `‖n₀‖² = φ⁶ + 3 = 8φ²` and `‖n₀‖/r = 4/φ` for `n₀ = (φ³, 1, 1, 1)`. For each chosen facet there is a rotation `Q ∈ SO(4)` such that `σx = (4/φ)Qx` does the following: it sends the facet's pole to `n₀`, every pole to a pole of norm `‖n₀‖`, every facet plane to a plane `ñ · y = ‖ñ‖²`, and every facet to the face of `σP` on that plane. `σP` is the intersection of the half-spaces `ñ · y ≤ ‖ñ‖²`. |
| L2 | `boundaryHomeomorph` | `Topology` | `x ↦ x/‖x‖` is a homeomorphism from `∂P` onto the unit sphere `S³ ⊆ ℍ`. |
| L2 | `radialTopology` | `Topology` | Lemma 2. `0` is an interior point of `P`, `P` is compact and lies in the ball of radius 2, and the radial map is a homeomorphism. Under `SphereTopology` (E3), `H_j(∂P; ℤ)` is `ℤ` for `j = 0, 3` and zero otherwise, and `∂P` is simply connected. |
| L4 | `chamberWalls` | `Coxeter` | The base flag `{v₀} ⊂ {v₀, v₃₀} ⊂ {v₀, v₃₀, v₃₁} ⊂ {v₀, v₃₀, v₃₁, v₃₂}` is a chain of cliques. Its four barycenters span `ℍ`, so a linear isometry that fixes them is the identity. Each wall reflection `sᵢ` lies in `K`, fixes the faces of the other ranks and moves the face of rank `i`. It is the reflection in the span of the other three barycenters. |
| L4 | `H₄_chain`, `wallRefl_mul_pow`, `orderOf_wallRefl_mul` | `Coxeter` | Mathlib's `CoxeterMatrix.H₄` is the chain `(3, 3, 5)`. `(sᵢsⱼ)^{mᵢⱼ} = 1`, and `sᵢsⱼ` has order exactly `mᵢⱼ`. |
| L4 | `closure_wallRefl` | `Coxeter` | `s₀, s₁, s₂, s₃` generate `K`. |
| L4 | `coxeterPresentation`, `coxeterHom_bijective` | `Coxeter` | Lemma 4's presentation. The Coxeter group of type `H₄` maps onto `K` with `sᵢ ↦` the wall reflections. Under `CoxeterSources` (E5, in the form `|W(H₄)| = 14400`) this map is an isomorphism `W(H₄) ≅ K`. |

### How the Milestone 4 proofs work

- Lemma 5, the numbers: `‖N‖² = 4(4 + 6φ)` comes from Milestone 2's supporting-facet computation. The identities `φ⁴ = 3φ + 2` and `φ⁶ = 8φ + 5` give `(4 + 6φ)/4 = φ⁴/2` and `φ⁶ + 3 = 8φ²`. The source says "regularity gives the same inradius at every facet"; here the norm of every pole is computed directly.
- Lemma 5, the half-space description of `P`: the ball of radius `φ²/√2` lies in `P` because `P` is the intersection of its facet half-spaces (`polytope_eq_halfSpaces`). That intersection is proved by a facet-cone argument.
  - Take a facet whose normal `N` maximizes `N · x`, and write `x` in the basis of its four vertices.
  - Suppose a coefficient were negative. The facet across the opposite ridge exists by Milestone 2's common-neighbour count, and it would have a larger `N · x`.
  - So `x` lies in the cone over the facet, and a point of that cone in the half-space lies in `P`.
- Lemma 5, the extremal ball and the rotation: no larger ball fits, because each pole lies on the boundary. The rotation is `L_{b,a}` with `a = n/‖n‖` and `b = n₀/‖n₀‖`, which lies in `SO(4)` by Milestone 3.
- Lemma 2: the source shows that `0` is interior through the cross-polytope `conv{±2eᵢ}`; here the inradius ball of Lemma 5 does this. The radial map is the restriction of Mathlib's gauge rescaling homeomorphism (`gaugeRescaleHomeomorph`) from `P` to the closed unit ball. It maps the frontier onto the unit sphere and agrees with `x ↦ x/‖x‖` there, because the gauge of `P` is `1` on its frontier. So the homeomorphism is proved directly, and the source's compact-to-Hausdorff step is not needed. Homology and simple connectivity move from `S³` to `∂P` along the homeomorphism, through Mathlib's singular homology functor and homotopy equivalence.
- Lemma 4, the walls: the reflections are in the hyperplanes orthogonal to `v_a - v_b` for the pairs `(0, 30)`, `(30, 31)`, `(31, 32)` and `(32, 42)`. Vertex 42 is the second common neighbour of the triangle `{0, 30, 31}`, so the four pairs move the faces of ranks 0 to 3 of the flag. Four kernel-checked tables over `ℤ[φ]` show that each reflection permutes the 120 vertices: `(u·u) v - 2(v·u) u` is `(u·u)` times a vertex. So each reflection lies in `K`. The same tables give the flag images and, through the barycenters, the mirror as the span of the other three barycenters.
- Lemma 4, the relations: by Milestone 3, an element of `K` is determined by the images of the four vertices of the base facet. So `(sᵢsⱼ)^m = 1` is a kernel check on those four vertices, and so is the minimality of `mᵢⱼ`.
- Lemma 4, generation: a breadth-first search over the group, run once outside Lean, gives explicit words in `s₀, …, s₃` for `L_{v₈/2,1}`, `L_{v₃₀/2,1}`, `L_{1,v₈/2}`, `L_{1,v₃₀/2}` and `J`. The kernel checks each word on the base facet. Milestone 3's word tree writes every element of `I*` as a word in `v₈/2` and `v₃₀/2`, and every element of `K` is `L_{p,q}` or `L_{p,q}J`. So the reflections generate `K`.
- Lemma 4, the presentation: Mathlib's `CoxeterSystem.lift` turns the relations into a homomorphism from `CoxeterMatrix.H₄.Group`. It is onto because the reflections generate `K`. Under `CoxeterSources`, both groups have 14,400 elements, so the surjection is a bijection.

### Statements and hypotheses in Milestone 4

- E3 is not formalized. `SphereTopology` states the three facts about the unit sphere `S³ ⊆ ℍ` that Lemma 2 uses: `S³` is simply connected, its integral singular homology is `ℤ` in degrees 0 and 3, and it is zero in every other degree. Mathlib v4.36.0 has neither the homology of spheres nor the simple connectivity of `S³`. The third part of E3, that a continuous bijection from a compact space to a Hausdorff space is a homeomorphism, is not needed, because the homeomorphism is proved.
- E5 is not formalized, and it enters by another route than the source's. The source proves that the presentation is complete through closed galleries in the dual cell complex of `S³`. That argument is E5's second part, and it uses Lemma 2's `π₁(∂P) = 1`. It then names the presented group `W(H₄)`, which is E5's first part.
  - Here `CoxeterSources` states the standard order of the Coxeter group of type `H₄`, `|W(H₄)| = 14400`, for Mathlib's group `CoxeterMatrix.H₄.Group` presented by the chain `(3, 3, 5)`.
  - Completeness then follows from the proved surjection and the proved order `|K| = 14400`.
  - So the gallery argument is not formalized, the order of `W(H₄)` is an external fact that the handoff's table of external results does not list, and Lemma 4's presentation does not depend on Lemma 2 here.
  - Without `CoxeterSources`, `coxeterPresentation` proves the relations with their exact orders, generation, and the surjection from `W(H₄)`.
- A facet in Lemma 5 is one of the 600 four-cliques of the edge graph, as in Milestone 2. These cut out `P` (`polytope_eq_halfSpaces`). Under E4 they are all the facets of `P` (`facetCompleteness`).
- Lemma 5's rotation is stated for each chosen facet. The source's remark that the symmetry group of `σP` is the conjugate of `K` under the similarity is not stated.
- Lemma 4's chambers are not defined as radially projected simplices. `chamberWalls` states what the proof uses about them: the flag, the barycenters, the triviality of the flag stabilizer, and the wall reflections. The simple transitivity of `K` on chambers and the connectivity of chamber adjacency are not stated. Generation is proved through explicit words instead.
- `∂P` is the frontier of `P` in `ℍ`, and `S³` is `Metric.sphere 0 1` in `ℍ` with the Euclidean norm. Homology is Mathlib's integral singular homology `AlgebraicTopology.singularHomologyFunctor`.

## Milestone 5: the retained model

The handoff's scope is the retained model as a structure: the census table O9, the frame groups and the hypotheses R, F, I, C and E as inputs; Lemmas 6, 7 and 8, with N4 and N5 as decidable checks; and the model-level uses of Lemmas 9 to 11 on the nine selected orbits, with N11. The statements and proofs were written in this session from `research/Full_600cell_Puzzle_Theory.tex` and the handoff. The blueprint chapter `blueprint/src/chapters/ArchonFull600Theory_RetainedModel.tex` quotes the source lines for each.

The hypotheses are inputs, not theorems. Their truth for the real puzzle is evidence (handoff section 4), and nothing here proves them. Each is a separate structure, so each lemma assumes exactly the hypotheses the source names for it.

| Handoff | Lean name | Module | Content |
| --- | --- | --- | --- |
| O9 | `censusRows`, `census` and the `run_cmd` check | `Census` | The 35 rows (rank, stickers per piece `k_o`, pieces `N_o`, frame group `H_o`) are typed in. Compiling the module reads `assets/census.json` and stops the build unless every row and the file's totals agree with it. |
| HR | `censusSums`, `HypothesisR.card_piece`, `HypothesisR.card_sticker` | `Census`, `RetainedModel` | `Σ N_o = 176520`, `600 + Σ k_o N_o = 259800` and `600 + Σ N_o = 177120`. Under R the model has 177120 pieces and 259800 sticker slots. |
| N4 | `frameTypeCounts`, `invariantOrder` | `Census` | Seven `C₂`, one `D₅`, two `C₅`, one `A₅` and 24 trivial frame groups, so `∏_o 2 |H_o^ab| = 2^43 · 5^2`. |
| N5 | `FrameGroupType.card_abelianization` | `Census` | `|D₅^ab| = 2` and `|A₅^ab| = 1`, through Milestone 1's `dihedralFiveCommutatorCard` and `alternatingFivePerfect`. |
| R, F, C, E | `RetainedModel`, `HypothesisR`, `HypothesisF`, `HypothesisC`, `HypothesisE` | `RetainedModel` | The data are the sticker slots, the pieces, the piece map and the primitives `H_c`, `T_c` of the 600 caps. `G` is the subgroup generated by the `H_c` and `T_c`. The four hypotheses are stated over these data. |
| L6 | `completeSignature_eq_iff`, `completeSignature_smul`, `convex_chamber`, `isPreconnected_patch` | `Chambers` | The geometric part. Two points off the cut hyperplanes have the same complete cap signature exactly when they lie in the same connected component. `M(kx) = k M(x)` for `k ∈ K`. Chambers are convex, and a chamber closure meets a closed facet in a convex set. |
| L6 | `HypothesisF.closure_primitives`, `HypothesisR.piece_legal`, `HypothesisR.signature_injective`, `HypothesisR.address_injective`, `HypothesisR.rank_multiplicity` | `RetainedModel` | Under F, `H_c` and `T_c` generate the cap's `A₄` action. Under R, the piece map is equivariant under every legal element, and distinct pieces have distinct complete signatures. `(M(A), h)` addresses the sticker slots, and `r(A) = |M(A)| - 1` and `k(A) = |Host(A)|` are the census rank and `k_o`. |
| L7 | `FrameWreath`, `FrameWreath.natCard`, `HypothesisR.coherentFrames`, `HypothesisR.CoherentFrames.coord`, `HypothesisR.transportedFrames`, `HypothesisR.natCard_ambient` | `FrameCoordinates` | Under R, transported frames exist. They give an injective homomorphism from `G` into `∏_o H_o^{N_o} ⋊ S_{N_o}`, a group of order `∏_o N_o! |H_o|^{N_o}`. |
| HI, L8 | `HypothesisI`, `FrameWreath.invariant_surjective`, `HypothesisR.index_invariantKernel`, `HypothesisI.invariantBound` | `InvariantBound` | The positional sign and the orientation image map each orbit's wreath product onto `{±1} × H_o^ab`, independently in each orbit. So their joint kernel has index exactly `2^43 · 5^2`. Under R and I, `|G| · 2^43 · 5^2 ≤ ∏_o N_o! |H_o|^{N_o}`. |
| N11 | `selectedOrbitCensus`, `HypothesisR.card_selectedOrbitStickers`, `HypothesisR.selected_single_sticker` | `Census`, `SelectedOrbits` | The nine selected orbits have one sticker per piece and 3600, 2400 and 7200 sticker slots, that is `600 m` with `m = 6, 4, 12`. |
| L9 | `HypothesisC.supportedAlternating` | `SelectedOrbits` | Under R and C, `G` contains `A_3600 × A_2400 × A_7200^7`, acting on the nine selected orbits and fixing every other sticker. |
| L10 | `HypothesisC.balancedColorBound` | `SelectedOrbits` | Under R and C, there are at least `B` reachable face-colour states, and `B` of them are attained with every unselected sticker solved. |
| L11 | `HypothesisC.wordDistanceBound`, `HypothesisC.totalVariationBound`, `RetainedModel.capWord_surjective` | `SelectedOrbits` | Under R and C, with the alphabet `{H_c, T_c, T_c⁻¹}` of 1800 letters, some state is at word distance at least 46648, and the total variation after 1000 letters is more than `1 - 10^(-148595)`. The alphabet's words realize every legal element. |

### How the Milestone 5 proofs work

- Census: the table is a Lean list, so every count is a kernel `decide`. The JSON check runs at compile time through `run_cmd` and Lean's JSON parser. It compares each row's fields and the file's totals, and a mismatch is a build error.
- Lemma 6, chambers:
  - The caps are Milestone 2's 600 four-cliques, with Lemma 5's poles. The cut function `n_c · x - α ‖n_c‖²` is affine, so a chamber is convex.
  - Two points with the same complete signature lie in one chamber, so in one component. Conversely, no cut function changes sign along a connected set off the cuts (intermediate value theorem), so the signature is constant on a component.
  - `K` acts on the caps through their vertices and preserves inner products, so it permutes the cut functions.
- Lemma 6, generation: a kernel check over `A₄` shows that any element of order 2 and any element of order 3 generate it. This is the source's Klein four-group argument. It is moved to each cap's vertex set by an equivalence.
- Lemma 6, signatures: R names a point of each piece's chamber and says that distinct pieces are distinct chambers ("no hidden identification"). Signature injectivity is derived from the geometric part, not assumed.
- Lemma 7 follows the source's proof:
  - Fix a reference piece and a reference sticker order for each orbit. R's transitivity gives a legal transporter to each position.
  - Labelling `(i, f)` by the transporter's image of reference sticker `f` gives a frame.
  - The increment of `g` at `i` is the return map `t_{g(i)}⁻¹ g t_i` on the reference piece, which R puts in `H_o`.
  - The coordinate map is a homomorphism by the wreath-product law. It is injective because the frames cover every moving sticker and the centres are fixed.
- Lemma 8: the sign and the abelianized product of frame increments are homomorphisms. For surjectivity, a transposition sets the sign and one frame coordinate sets the orientation value. The target has order `∏_o 2 |H_o^ab|`, which is `invariantOrder` through the abelianization orders of the census types. Under I, closure induction puts every legal element in the kernel, and `|kernel| · index = |ambient|` gives the bound.
- Lemmas 9 to 11 reuse Milestone 1's abstract theorems.
  - The nine orbits are R's orbit sticker sets. Their disjointness and invariance come from R, and their sizes come from the census through `|O_o| = k_o N_o`.
  - For Lemma 11, Milestone 1 assumed that the alphabet's words realize every legal element. Here this is proved: words compose by concatenation, the letters include every generator, and in the finite group `G` an inverse is a positive power.

### Statements and hypotheses in Milestone 5

- R, F, C and E are structures in `RetainedModel`. I is in `InvariantBound`, because it refers to the transported coordinates of Lemma 7. The lemmas use the hypotheses the source names:
  - Lemma 6 uses F for generation and R for the rest.
  - Lemmas 7 and 8 use R, and Lemma 8 also I.
  - Lemmas 9 to 11 use R and C.
  - E is stated but not used here; Lemma 12 (Milestone 6) uses it.
- R is stated in finite terms:
  - centres form the class `none`, and the counts are the census numbers;
  - each piece has a chamber point, and distinct pieces are distinct chambers;
  - each sticker has a host facet, and the host determines the sticker within its piece;
  - each piece has a rank, and each orbit a frame group `H_o ≤ S_{k_o}` isomorphic to the census type, with a reference piece and reference frame.
  - `stabilizer_mem` says that every legal element mapping the reference piece to itself acts on its stickers through `H_o`. This and `frameGroup_equiv` are the reading of "faithful frame groups after coherent transport".
  - The field `transitive` says that each class is one orbit of `G` on pieces. This is the reading of "moving orbit", and Lemma 7's transporters need it.
  - `host_injective` is the source's one patch per hosting facet, taken as part of "no hidden identification". The source's convexity reason for it is `isPreconnected_patch`, which is proved for the closed intersection. The open patch with its lower-dimensional boundary removed is not stated.
- F gives each cap a homomorphism from the even permutations of its four vertices to the permutations of the sticker slots. `H_c` and `T_c` are the images of an element of order 2 and an element of order 3. F does not require this action to be faithful.
- I asks for some coherent frames in which every `H_c` and `T_c` has even positional permutation and trivial orientation image in every orbit. The source states I in the frames its proof constructs. Allowing any coherent frames weakens the hypothesis, so Lemma 8 here is at least as strong. The source's remark that changing the frames conjugates the coordinates is not formalized.
- E has a field that the source's words imply but do not write out. `geom_capAction` says that `k ∈ K` conjugates each rotation of cap `c` into the rotation of cap `k • c` that permutes its vertices through `k`. This is the reading of "the subdivision is preserved", and Lemma 12 needs it. E does not tie the chamber points or host facets to the geometric action, so `M(kA) = k M(A)` is proved only for points (`completeSignature_smul`), not as a statement about pieces.
- The census check runs when `Census.lean` is compiled. Lake does not track `assets/census.json` as an input, so after the file changes, `Census.lean` must be rebuilt (for example from a clean build) for the check to run again.
- The selected orbits are numbered by Milestone 1's `selectedOrbitIds`. The alphabet of Lemma 11 is indexed through any enumeration `Fin 600 ≃ Cap` of the caps, and the theorems hold for every enumeration.

## Milestone 6: normalization and the package

The handoff's scope is Lemmas 12, 13 and 15, then Theorem T as the conjunction of its seven parts, each stated under its own hypotheses. Lemma 14 is Milestone 1's `transportNonfixedMap`. The statements and proofs were written in this session from `research/Full_600cell_Puzzle_Theory.tex` and the handoff. The blueprint chapter `blueprint/src/chapters/ArchonFull600Theory_Package.tex` quotes the source lines for each.

| Handoff | Lean name | Module | Content |
| --- | --- | --- | --- |
| L12 | `HypothesisE.conj_legalGroup`, `HypothesisE.range_le_normalizer`, `HypothesisE.normalization` | `Normalization` | Under R, F and E, `κ(k) G κ(k)⁻¹ = G` for every `k ∈ K`, so the geometric action normalizes `G`. |
| L12 | `HypothesisE.legalGroup_inf_range`, `eq_one_of_forall_smul_cap` | `Normalization` | `G ∩ κ(K) = 1`. A symmetry of `P` that fixes every facet is the identity. |
| L12 | `HypothesisE.semidirectEquiv`, `HypothesisE.existsUnique_mul`, `HypothesisE.normalization` | `Normalization` | `⟨G, κ(K)⟩ ≅ G ⋊ K` through `(g, k) ↦ g κ(k)`, and each element has a unique form `g κ(k)`. `normalization` states the three parts of Lemma 12 together. |
| L12 remark | `HypothesisC.card_symmetryGroup_lt` | `Normalization` | Under R and C, `|K| < |G|`, so the legal group is not the geometric group. |
| L13 | `tetrahedralConjugation` | `SymmetryReduction` | In `A₄` with `H = (1 2)(3 4)` and `T = (1 2 3)`, the conjugate `T H T⁻¹` is none of `H`, `T`, `T⁻¹` and needs three letters, while `H` needs one. |
| L13 | `PathCost`, `exists_goal_pathCost_iff`, `goalCosts_eq`, `isLeast_goalCosts_iff` | `SymmetryReduction` | If a group acts by cost-preserving automorphisms of the state graph and preserves the goal set, the costs of paths to the goal and to the goal orbits in the orbit graph are the same set, so the minimum costs agree. |
| L15 | `setupLength_withInverses_le`, `pathDistance_withInverses_le`, `pathDistance_le_of_embedding` | `GuardedSetup` | Adding the inverse letters cannot increase a minimum unit-cost setup length on ordered target frames, nor a minimum weighted cost when the old letters keep their costs. |
| L15 | `withInverses_fixes`, `capGuard_fixes`, `capGuardedSetup`, `reachable_withInverses_iff`, `capSetup_reachable_iff` | `GuardedSetup` | Inverse letters fix the buffers when the letters do. For the cap alphabet, the inverse letters are legal and do not increase a setup length, and under the support property `HypothesisR.CapSupport`, forbidding the caps of `M(A) ∪ M(B)` makes every letter fix the buffer pieces. Under F, `H_c⁻¹ = H_c` and `T_c⁻¹ = T_c²`, so the inverse letters do not enlarge the reachable frames. |
| T | `TheoryPart1` to `TheoryPart7`, `theoryPart1` to `theoryPart7`, `fullTheoryPackage` | `Package` | The seven parts as propositions, each under its own hypotheses, their proofs, and their conjunction. |
| T | `antipodalAxes`, `card_antipodalAxes`, `antipode_disjoint`, `infinite_specialOrthogonal`, `card_capRotations`, `HypothesisF.exists_inverse_word` | `Package` | The assertions of T that no earlier lemma covers: the 300 antipodal axes with disjoint outer layers, `SO(4)` infinite, the cap rotation group of order 12, and inverse words of the same length. |

### How the Milestone 6 proofs work

- Lemma 12:
  - Normalization: E's field `geom_capAction` says that `κ(k)` conjugates each rotation of cap `c` into a rotation of cap `k • c`. By F and Lemma 6 every cap rotation is legal, so conjugation maps the generators `H_c`, `T_c` into `G`. Applied to `k` and `k⁻¹`, it maps `G` onto `G`.
  - Trivial intersection: a legal element fixes every centre, and a geometric `κ(k)` that fixes every centre fixes every cap. The source argues that the poles span `ℝ⁴`. Here a symmetry that fixes every facet fixes the facet `{0, 30, 31, 32}` and four facets that meet it in one vertex each, so it fixes those four vertices, which form a basis of `ℍ` (Milestone 3).
  - The semidirect product is Mathlib's `SemidirectProduct` for conjugation, mapped by `(g, k) ↦ g κ(k)`. It is injective by the trivial intersection and onto the join because `κ(K)` normalizes `G`.
- Lemma 13:
  - The `A₄` example is decided by `decide` over the permutations of four labels, including every word of at most two letters.
  - Paths project to the orbit graph with the same cost. A path of the orbit graph lifts, from any state of its start orbit, to a labelled path of the same cost: the lift of an orbit edge represented by `u → v`, at the state `ℓ • u`, is the edge `ℓ • u → ℓ • v`. The goal set is invariant, so the lifted endpoint is a goal.
- Lemma 15: the enlarged alphabet contains the old letters with their moves and costs, so every old path is a new path of the same cost, and the least cost (`sInf` in a complete lattice, with `⊤` for unreachable) can only fall. The inverse transitions of the cap alphabet have positive witnesses because `H_c⁻¹ = H_c` and `T_c⁻¹ = T_c²` under F (F's cap actions are homomorphisms from `A₄`, and `H_c`, `T_c` are the images of an element of order 2 and one of order 3).
- Theorem T: each part is a `def` of a proposition, and its proof assembles the lemmas of Milestones 1 to 6. New in this module:
  - The central symmetry `x ↦ -x` is `L_{-1,1}`, and `-1 ∈ I*`, so it lies in `K`. It moves each cap to its antipode, with `n_{-c} = -n_c`. No cap is its own antipode: a vertex and its negative would be adjacent, but they are at distance 4, not `2/φ`. So the pairs `{c, -c}` have two elements each and there are 300. The two cap regions of an axis are disjoint because their cut functions cannot both be positive.
  - `SO(4)` is infinite: the unit quaternions `t + √(1 - t²) i` are infinitely many, and `p ↦ L_{p,1}` is injective into `SO(4)`.
  - The rotation group of a cap is the even permutations of its four vertices, of order `4!/2 = 12`.
  - Inverse words: a word's inverse is the reversed word of inverse letters, with `H_c ↦ H_c`, `T_c ↦ T_c⁻¹` and `T_c⁻¹ ↦ T_c`, and under F `H_c⁻¹ = H_c`.

### Statements and hypotheses in Milestone 6 and Theorem T

- Theorem T states its parts under the hypotheses the source names, with these readings:
  - Part 1 is unconditional for the coordinates, the edge length, the clique counts, the faces spanned by the cliques, the vertex link counts, the five facets at each edge and the radial homeomorphism `∂P ≅ S³`. That the facets are exactly the 600 tetrahedra uses E4 (`FacetFacts`), the homology and simple connectivity use E3 (`SphereTopology`), and the identification of `V/2` as binary icosahedral and of `P` as a regular 600-cell uses E1 and E2 (`CoordinateSources`), as in Milestones 2 and 4.
  - Part 2 is unconditional except for its last clause: the legal group is distinguished from `K` by `|K| < |G|`, which uses R and C. The source lists this clause among the unconditional facts; without a model there is no legal group to compare.
  - Part 3 takes the axis and cut facts unconditionally and the model counts under R. The 600 centres and the orbit sizes are fields of R; the source states them as properties of the retained model. Local `A₄` generation uses F.
  - Parts 4 to 7 use the hypotheses the source names: R for the embedding and the sizes 3600, 2400 and 7200 of the selected orbits, R and I for the upper bound, R and C for the lower bounds and the word metric, R, F and E for Lemma 12. Lemma 13's quotient statement and the scope tests are unconditional.
  - The clauses under R, F, I, C and E are implications. The package does not construct a model that satisfies them.
  - `capDepth = 121 / 125` in Part 3 records the depth that `capRegion` uses; it holds by definition.
- Lemma 12 needs the field `geom_capAction` of E described under Milestone 5. E's geometric action is faithful by hypothesis.
- Lemma 13's first statement is the source's own `A₄` example, which illustrates that normalization need not preserve a word metric. A statement that `κ(k)` itself fails to preserve the alphabet's word length for the retained model is not provable from F, which does not make the cap actions faithful. The source's remarks on colouring, protection and buffer conditions for a reduction are not formalized.
- Lemma 15's cap guard uses `HypothesisR.CapSupport`: a cap outside a piece's complete signature fixes the piece's stickers. This is the reading of the source's "it rotates the cap and fixes its complement" in the proof of Lemma 12. R and F do not state it, and the source's "is a sufficient condition" reads it as a property of the retained cut. It is an explicit hypothesis of `capGuard_fixes` and `capGuardedSetup` only.
- `setupLength_withInverses_le` and `pathDistance_withInverses_le` hold for every alphabet. `capGuardedSetup` applies the first to the cap alphabet of the allowed caps, together with the legality of the inverse letters and the buffer property.
- Not formalized:
  - Part 1: the polar 120-cell, flag regularity and the Schläfli symbol `{3,3,5}`, and the icosahedral vertex links beyond their counts (Milestones 2 and 4).
  - Part 3: that the middle layer of an axis is fixed during an outer-layer turn, and the 433 sticker regions per facet, which the source calls a retained-model input. `|M| - 1` being a membership rank is a remark on names.
  - Part 7 and Lemma 15: that breadth-first search and Dijkstra's algorithm compute these distances, and the absence of numerical speedups, which is not a mathematical statement.

## Build

The toolchain is pinned in `lean-toolchain` (Lean v4.36.0), and `lakefile.toml` requires Mathlib at `v4.36.0`.

```
lake exe cache get
lake build
```

With Mathlib's build cache in place, `lake build` rebuilds only this library. `Basic.lean` takes under a minute; `EdgeGraph.lean`, the slowest module, takes about 50 seconds, most of it in the kernel checks of the pair and count certificates. The three Milestone 3 modules, the three Milestone 4 modules, the six Milestone 5 modules and the four Milestone 6 modules take 10 to 25 seconds each on an otherwise idle machine; `Package.lean` takes about 11 seconds. The build passes with no errors and no linter warnings. Lake still notes, once for each import, that the imported Mathlib modules are designed for the module system; these files are not modules.

## How it was produced

- Milestone 1 statements: Archon 0.3.3, an automated Lean formalization loop run outside this repository, in iteration 19 of its `autoformalize` stage, with OpenAI GPT-6.2 Sol at effort `max` through the Codex CLI. The agent logs and Archon's state directory are not included.
- Milestone 1 proofs and all of Milestones 2 to 6: a Claude Code session (Claude Opus 5.5), without Codex calls.
