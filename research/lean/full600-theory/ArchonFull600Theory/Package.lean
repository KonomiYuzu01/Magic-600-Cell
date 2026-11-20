import ArchonFull600Theory.Coxeter
import ArchonFull600Theory.GuardedSetup

set_option linter.style.header false in
/-!
# Theorem: the Full 600-cell theory package

Source: `research/Full_600cell_Puzzle_Theory.tex`, the Theorem (lines 1557 to 1693).

`fullTheoryPackage` is the conjunction of the seven parts `TheoryPart1` to `TheoryPart7`. Each
part is a proposition that states its assertions under its own hypotheses, and each is proved
from the lemmas of Milestones 1 to 6 (`theoryPart1` to `theoryPart7`). The hypotheses are the
external results E1 to E5 as Milestones 2 to 4 state them (`CoordinateSources`, `FacetFacts`,
`SphereTopology`, `CoxeterSources`) and the model hypotheses R, F, I, C and E of Milestone 5.

This module also proves the assertions of the package that no earlier lemma covers:

* `centralSymmetry`, `antipode` and `antipodalAxes`: `x ↦ -x` lies in `K`, so the 600 caps come
  in antipodal pairs `{c, -c}`, with `n_{-c} = -n_c`. There are 300 such axes, and the two cap
  regions of an axis, its outer layers, are disjoint.
* `infinite_specialOrthogonal`: `SO(4)` is infinite, so the finite group `K⁺` is not `SO(4)`.
* `card_capRotations`: the rotation group of a cap, the even permutations of its four vertices,
  has 12 elements.
* `HypothesisF.exists_inverse_word`: under F, every word in the 1800 letters `H_c`, `T_c`,
  `T_c⁻¹` has an inverse word of the same length.
-/

namespace ArchonFull600Theory

open Equiv Finset CategoryTheory Limits
open scoped goldenRatio Quaternion RealInnerProductSpace Pointwise

universe u v w

/-! ### Antipodal axes -/

/-- The central symmetry `x ↦ -x`, which is `L_{-1,1} ∈ K`. -/
noncomputable def centralSymmetry : symmetryGroup :=
  ⟨lpq (-1) 1, lpq_mem_symmetryGroup binaryIcosahedralGroup.2.2 (Subgroup.one_mem _)⟩

theorem centralSymmetry_apply (x : ℍ) : (centralSymmetry : ℍ ≃ₗᵢ[ℝ] ℍ) x = -x := by
  change lpq (-1) 1 x = -x
  rw [lpq_apply]
  simp [Unitary.coe_neg]

theorem centralSymmetry_mul_self : centralSymmetry * centralSymmetry = 1 :=
  Subtype.ext <| LinearIsometryEquiv.ext fun x => by
    change (centralSymmetry : ℍ ≃ₗᵢ[ℝ] ℍ) ((centralSymmetry : ℍ ≃ₗᵢ[ℝ] ℍ) x) = x
    rw [centralSymmetry_apply, centralSymmetry_apply, neg_neg]

/-- The antipodal cap `-c`. -/
noncomputable def antipode (c : Cap) : Cap := centralSymmetry • c

theorem antipode_antipode (c : Cap) : antipode (antipode c) = c := by
  rw [antipode, antipode, smul_smul, centralSymmetry_mul_self, one_smul]

theorem capPole_antipode (c : Cap) : capPole (antipode c) = -capPole c := by
  rw [antipode, capPole_smul, centralSymmetry_apply]

theorem antipode_ne (c : Cap) : antipode c ≠ c := by
  intro h
  have hc := SimpleGraph.mem_cliqueFinset_iff.1 c.2
  obtain ⟨i, hi⟩ : (c : Finset (Fin 120)).Nonempty := by
    rw [← Finset.card_pos, hc.card_eq]
    norm_num
  have hj : centralSymmetry • i ∈ (c : Finset (Fin 120)) := by
    have := Finset.mem_image_of_mem (centralSymmetry • ·) hi
    rwa [← coe_smul_cap, ← antipode, h] at this
  have hv : vertexR (centralSymmetry • i) = -vertexR i := by
    rw [vertexR_smul, centralSymmetry_apply]
  have hnorm : ‖vertexR i‖ = 2 := vertexNorm _ (range_vertexR ▸ ⟨i, rfl⟩)
  have hne : i ≠ centralSymmetry • i := by
    intro hij
    have h0 : vertexR i = -vertexR i := by rw [← hv, ← hij]
    have h2 : (2 : ℝ) • vertexR i = 0 := by
      rw [two_smul]
      nth_rw 1 [h0]
      exact neg_add_cancel _
    have h3 := congrArg norm h2
    rw [norm_smul, Real.norm_eq_abs, abs_two, hnorm, norm_zero] at h3
    norm_num at h3
  have hadj : edgeGraph.Adj i (centralSymmetry • i) := hc.isClique hi hj hne
  have hdist := ((edgeLength i _ hne).2).2 hadj
  rw [hv, dist_eq_norm, sub_neg_eq_add, ← two_smul ℝ, norm_smul, Real.norm_eq_abs, abs_two,
    hnorm] at hdist
  have hφ : (2 : ℝ) / φ < 2 := by
    rw [div_lt_iff₀ Real.goldenRatio_pos]
    nlinarith [Real.one_lt_goldenRatio]
  linarith

theorem antipode_disjoint (c : Cap) : Disjoint (capRegion c) (capRegion (antipode c)) := by
  rw [Set.disjoint_left]
  rintro x ⟨-, hx⟩ ⟨-, hx'⟩
  rw [cutValue, capPole_antipode, inner_neg_left, norm_neg] at hx'
  rw [cutValue] at hx
  have : 0 ≤ capDepth * ‖capPole c‖ ^ 2 := by unfold capDepth; positivity
  linarith

theorem antipodalPair_fiber (c₀ : Cap) :
    {x ∈ (univ : Finset Cap) | ({x, antipode x} : Finset Cap) = {c₀, antipode c₀}} =
      {c₀, antipode c₀} := by
  ext c
  simp only [Finset.mem_filter, Finset.mem_univ, true_and, Finset.mem_insert,
    Finset.mem_singleton]
  constructor
  · intro hc
    have : c ∈ ({c₀, antipode c₀} : Finset Cap) := hc ▸ Finset.mem_insert_self c _
    simpa using this
  · rintro (rfl | rfl)
    · rfl
    · rw [antipode_antipode, Finset.pair_comm]

theorem card_antipodalPair_fiber (c₀ : Cap) :
    #{x ∈ (univ : Finset Cap) | ({x, antipode x} : Finset Cap) = {c₀, antipode c₀}} = 2 := by
  rw [antipodalPair_fiber, Finset.card_pair_eq_two_iff]
  exact (antipode_ne c₀).symm

/-- The 600 caps form 300 antipodal pairs. The sum is rewritten with `Finset.sum_const_nat`:
through `Finset.sum_const`, the kernel would evaluate the card of the pairs to compare two
instances of `•`. -/
theorem card_antipodalAxes :
    #(univ.image fun c : Cap => ({c, antipode c} : Finset Cap)) = 300 := by
  have h1 : #(univ : Finset Cap) =
      ∑ b ∈ univ.image (fun c : Cap => ({c, antipode c} : Finset Cap)),
        #{x ∈ (univ : Finset Cap) | ({x, antipode x} : Finset Cap) = b} :=
    Finset.card_eq_sum_card_image _ univ
  have hfib : ∀ a ∈ univ.image fun c : Cap => ({c, antipode c} : Finset Cap),
      #{x ∈ (univ : Finset Cap) | ({x, antipode x} : Finset Cap) = a} = 2 := by
    intro a ha
    obtain ⟨c₀, -, rfl⟩ := Finset.mem_image.1 ha
    exact card_antipodalPair_fiber c₀
  rw [Finset.sum_const_nat hfib, Finset.card_univ, card_cap] at h1
  omega

/-- **Part 3, axes.** The antipodal map of the caps is an involution without fixed points, with
`n_{-c} = -n_c`, so the 600 caps form 300 antipodal axes `{c, -c}`; the two cap regions of an
axis, its outer layers, are disjoint. -/
theorem antipodalAxes :
    (∀ c, antipode (antipode c) = c ∧ antipode c ≠ c ∧ capPole (antipode c) = -capPole c ∧
      Disjoint (capRegion c) (capRegion (antipode c))) ∧
      #(univ.image fun c : Cap => ({c, antipode c} : Finset Cap)) = 300 :=
  ⟨fun c => ⟨antipode_antipode c, antipode_ne c, capPole_antipode c, antipode_disjoint c⟩,
    card_antipodalAxes⟩

/-! ### Distinct groups -/

theorem norm_circleQuaternion {t : ℝ} (ht : t ∈ Set.Icc (-1 : ℝ) 1) :
    ‖(⟨t, Real.sqrt (1 - t ^ 2), 0, 0⟩ : ℍ)‖ = 1 := by
  have h1 : 0 ≤ 1 - t ^ 2 := by nlinarith [ht.1, ht.2]
  have h : ‖(⟨t, Real.sqrt (1 - t ^ 2), 0, 0⟩ : ℍ)‖ *
      ‖(⟨t, Real.sqrt (1 - t ^ 2), 0, 0⟩ : ℍ)‖ = 1 := by
    rw [← Quaternion.normSq_eq_norm_mul_self, Quaternion.normSq_def']
    simp only [Real.sq_sqrt h1]
    ring
  exact (mul_self_eq_one_iff.1 h).resolve_right
    (by linarith [norm_nonneg (⟨t, Real.sqrt (1 - t ^ 2), 0, 0⟩ : ℍ)])

/-- The unit quaternions `t + √(1 - t²) i`, `-1 ≤ t ≤ 1`, are infinitely many. -/
theorem infinite_unitary : Infinite (unitary ℍ) := by
  have := (Set.Icc_infinite (by norm_num : (-1 : ℝ) < 1)).to_subtype
  refine Infinite.of_injective (fun t : Set.Icc (-1 : ℝ) 1 =>
    (⟨⟨t, Real.sqrt (1 - t ^ 2), 0, 0⟩, mem_unitary_of_norm_eq_one
      (norm_circleQuaternion t.2)⟩ : unitary ℍ)) fun s t h => ?_
  exact Subtype.ext (congrArg (fun q : unitary ℍ => (q : ℍ).re) h)

/-- **Part 2, distinct groups.** `SO(4)` is infinite, through `p ↦ L_{p,1}`. -/
theorem infinite_specialOrthogonal : Infinite specialOrthogonal := by
  have := infinite_unitary
  refine Infinite.of_injective (fun p : unitary ℍ =>
    (⟨lpq p 1, mem_specialOrthogonal.2 (orthDet_lpq p 1)⟩ : specialOrthogonal))
    fun p q h => ?_
  have h1 : lpq p 1 1 = lpq q 1 1 :=
    congrArg (fun A : specialOrthogonal => (A : ℍ ≃ₗᵢ[ℝ] ℍ) 1) h
  exact Subtype.ext (by simpa [lpq_apply] using h1)

/-- **Part 2, distinct groups.** The rotation group of a cap, the even permutations of its four
vertices, has 12 elements. -/
theorem card_capRotations (c : Cap) :
    Nat.card (alternatingGroup ↥(c : Finset (Fin 120))) = 12 := by
  have h4 : Nat.card ↥(c : Finset (Fin 120)) = 4 := by
    rw [Nat.card_eq_fintype_card, card_capVertices]
  have : Nontrivial ↥(c : Finset (Fin 120)) :=
    Fintype.one_lt_card_iff_nontrivial.1 (by rw [card_capVertices]; norm_num)
  have h := two_mul_nat_card_alternatingGroup (α := ↥(c : Finset (Fin 120)))
  have h24 : Nat.factorial 4 = 24 := rfl
  rw [Nat.card_perm, h4, h24] at h
  omega

/-! ### Inverse words -/

section InverseWords

variable {Slot : Type*} {legal : Subgroup (Perm Slot)} {Letter : Type*}

theorem wordPermutation_singleton (moves : Letter → legal) (a : Letter) :
    wordPermutation moves [a] = moves a :=
  mul_one _

/-- If each letter has an inverse letter, each word has an inverse word of the same length:
the reversed word of inverse letters. -/
theorem exists_inverse_word (moves : Letter → legal) (inv : Letter → Letter)
    (hinv : ∀ a, moves (inv a) = (moves a)⁻¹) (w : List Letter) :
    ∃ w' : List Letter, w'.length = w.length ∧
      wordPermutation moves w' = (wordPermutation moves w)⁻¹ := by
  induction w with
  | nil => exact ⟨[], rfl, inv_one.symm⟩
  | cons a w ih =>
    obtain ⟨w', hl, hw'⟩ := ih
    have ha : wordPermutation moves (a :: w) = wordPermutation moves w * moves a := by
      rw [← wordPermutation_singleton moves a, ← wordPermutation_append]
      rfl
    refine ⟨w' ++ [inv a], by simp [hl], ?_⟩
    rw [wordPermutation_append, hw', wordPermutation_singleton, hinv, ha, mul_inv_rev]

end InverseWords

/-- The inverse of a letter of the 1800-letter alphabet: `H_c ↦ H_c`, `T_c ↦ T_c⁻¹`,
`T_c⁻¹ ↦ T_c`. -/
def inverseCapLetter (a : Fin 600 × Fin 3) : Fin 600 × Fin 3 :=
  (a.1, if a.2 = 0 then 0 else if a.2 = 1 then 2 else 1)

/-- **Part 5, inverse words.** Under F, every word in the 1800 letters `H_c`, `T_c`, `T_c⁻¹`
has an inverse word of the same length. -/
theorem HypothesisF.exists_inverse_word {Sticker Piece : Type*} {M : RetainedModel Sticker Piece}
    (F : HypothesisF M) (e : Fin 600 ≃ Cap) (w : List (Fin 600 × Fin 3)) :
    ∃ w' : List (Fin 600 × Fin 3), w'.length = w.length ∧
      wordPermutation (capLetterPermutation (M.capH e) (M.capT e)) w' =
        (wordPermutation (capLetterPermutation (M.capH e) (M.capT e)) w)⁻¹ := by
  refine ArchonFull600Theory.exists_inverse_word _ inverseCapLetter ?_ w
  rintro ⟨j, i⟩
  fin_cases i
  · exact Subtype.ext (F.H_inv (e j)).symm
  · rfl
  · simp [capLetterPermutation, inverseCapLetter]

/-! ### The seven parts -/

/-- **Theorem, part 1: geometry and topology.** The vertices have norm 2, there are 120 of
them, and they are the exposed and the extreme points of `P`. Distinct vertices are at distance
at least `2/φ`, with equality exactly along the edge graph. The edge graph has 120, 720, 1200
and 600 cliques of sizes 1 to 4, and each clique spans an exposed face of `P`; each four-clique
spans a tetrahedral facet. Each vertex lies in 12 edges, 30 triangles and 20 four-cliques
(the counts of an icosahedral link), five four-cliques meet at each edge, and the radial map is
a homeomorphism `∂P ≅ S³`. Under E4 the facets are exactly the 600 tetrahedra. Under E3,
`H_j(∂P; ℤ)` is `ℤ` for `j = 0, 3` and zero otherwise, and `∂P` is simply connected. Under E1
and E2, `V/2` is binary icosahedral and `P` is a regular 600-cell. Not formalized: that the polar
of `P` is a regular 120-cell, flag regularity and the Schläfli symbol `{3,3,5}`; the vertex links
are stated through their counts. -/
def TheoryPart1 : Prop :=
  (vertexSet.ncard = 120 ∧ (∀ q ∈ vertexSet, ‖q‖ = 2) ∧
      (convexHull ℝ vertexSet).exposedPoints ℝ = vertexSet ∧
      (convexHull ℝ vertexSet).extremePoints ℝ = vertexSet) ∧
    (∀ i j : Fin 120, i ≠ j → 2 / φ ≤ dist (vertexR i) (vertexR j) ∧
      (dist (vertexR i) (vertexR j) = 2 / φ ↔ edgeGraph.Adj i j)) ∧
    (#(edgeGraph.cliqueFinset 1) = 120 ∧ #(edgeGraph.cliqueFinset 2) = 720 ∧
      #(edgeGraph.cliqueFinset 3) = 1200 ∧ #(edgeGraph.cliqueFinset 4) = 600) ∧
    (∀ n, ∀ s ∈ edgeGraph.cliqueFinset n,
      (∀ x ∈ polytope, ⟪facetNormal s, x⟫ ≤ 4 + 2 * (n - 1) * φ) ∧
        {x ∈ polytope | ⟪facetNormal s, x⟫ = 4 + 2 * (n - 1) * φ} = cliqueTetra s) ∧
    (∀ s ∈ edgeGraph.cliqueFinset 4, IsFacet polytope (cliqueTetra s)) ∧
    (∀ v, edgeGraph.degree v = 12 ∧ #((edgeGraph.cliqueFinset 2).filter (v ∈ ·)) = 12 ∧
      #((edgeGraph.cliqueFinset 3).filter (v ∈ ·)) = 30 ∧
      #((edgeGraph.cliqueFinset 4).filter (v ∈ ·)) = 20) ∧
    (∀ e ∈ edgeGraph.cliqueFinset 2, #((edgeGraph.cliqueFinset 4).filter (e ⊆ ·)) = 5) ∧
    (∃ e : frontier polytope ≃ₜ Metric.sphere (0 : ℍ) 1,
      ∀ x, (e x : ℍ) = ‖(x : ℍ)‖⁻¹ • (x : ℍ)) ∧
    (FacetFacts polytope →
      {F | IsFacet polytope F} =
          cliqueTetra '' (edgeGraph.cliqueFinset 4 : Set (Finset (Fin 120))) ∧
        {F | IsFacet polytope F}.ncard = 600) ∧
    (SphereTopology →
      (∀ n, n = 0 ∨ n = 3 →
        Nonempty (intHomology n (TopCat.of (frontier polytope)) ≅ ModuleCat.of ℤ ℤ)) ∧
      (∀ n, n ≠ 0 → n ≠ 3 → IsZero (intHomology n (TopCat.of (frontier polytope)))) ∧
      SimplyConnectedSpace (frontier polytope)) ∧
    ∀ {IsBinaryIcosahedral IsRegular600Cell : Set ℍ → Prop},
      CoordinateSources IsBinaryIcosahedral IsRegular600Cell →
        IsBinaryIcosahedral ((2 : ℝ)⁻¹ • vertexSet) ∧ IsRegular600Cell (convexHull ℝ vertexSet)

theorem theoryPart1 : TheoryPart1 := by
  obtain ⟨-, hexp, hext, -⟩ := exposedVertices
  obtain ⟨-, h1, h2, h3, h4⟩ := faceCounts
  refine ⟨⟨vertexCount, vertexNorm, hexp, hext⟩, edgeLength, ⟨h1, h2, h3, h4⟩,
    fun n s hs => cliqueFace hs, fun s hs => isFacet_cliqueTetra hs, vertexLinkCounts,
    fun e he => (incidenceCounts.1 e he).2, ⟨boundaryHomeomorph, boundaryHomeomorph_apply⟩,
    facetCompleteness, fun h => ?_, fun h => ?_⟩
  · obtain ⟨-, -, -, -, hH, hZ, hsc, -⟩ := radialTopology h
    exact ⟨hH, hZ, hsc⟩
  · obtain ⟨-, -, -, hI, hR⟩ := coordinateIdentification h
    exact ⟨hI, hR⟩

/-- **Theorem, part 2: geometric symmetry.** `V/2` is the group `I*` of 120 unit quaternions.
`(p, q) ↦ L_{p,q}` identifies `K⁺` with `(I* × I*)/⟨(-1, -1)⟩`, of order 7200; with
quaternion conjugation `J`, `K` has order 14400, and every element of `K` is `L_{p,q}` or
`L_{p,q} J` with `p, q ∈ I*`, where `J L_{p,q} J = L_{q,p}`. The wall reflections satisfy the
`H₄` chain relations and generate `K`, so `W(H₄)` maps onto `K`; under E5 this is an
isomorphism. `(p, q) ↦ L_{p,q}` maps `Sp(1) × Sp(1)` onto `SO(4)` with kernel
`{(1, 1), (-1, -1)}`. The finite groups are distinct from `SO(4)`, which is infinite, from the
rotation group of a cap, of order 12, and, under R and C, from the legal group, which is larger
than `K`. The source lists the last clause among the unconditional facts, and its remark on
distinct groups proves it under C; without a model there is no legal group to compare. -/
def TheoryPart2 : Prop :=
  (Nat.card binaryIcosahedral = 120 ∧
      (fun u : unitary ℍ => (u : ℍ)) '' binaryIcosahedral = (2⁻¹ : ℝ) • vertexSet) ∧
    (Nat.card rotationGroup = 7200 ∧ Nat.card symmetryGroup = 14400 ∧
      (∃ e : icosianPairs ⧸ Subgroup.zpowers negOnePair ≃* rotationGroup,
        ∀ pq : icosianPairs,
          (e (pq : icosianPairs ⧸ Subgroup.zpowers negOnePair) : ℍ ≃ₗᵢ[ℝ] ℍ) =
            lpq (pq : unitary ℍ × unitary ℍ).1 (pq : unitary ℍ × unitary ℍ).2) ∧
      conjIso ∈ symmetryGroup ∧ conjIso ∉ rotationGroup ∧
      (∀ A : ℍ ≃ₗᵢ[ℝ] ℍ, A ∈ symmetryGroup ↔ ∃ p ∈ binaryIcosahedral,
        ∃ q ∈ binaryIcosahedral, A = lpq p q ∨ A = lpq p q * conjIso) ∧
      ∀ p q : unitary ℍ, conjIso * lpq p q * conjIso = lpq q p) ∧
    (((∀ i, CoxeterMatrix.H₄ i i = 1) ∧ CoxeterMatrix.H₄ 0 1 = 3 ∧ CoxeterMatrix.H₄ 1 2 = 3 ∧
        CoxeterMatrix.H₄ 2 3 = 5 ∧ ∀ i j : Fin 4, (i : ℕ) + 1 < j → CoxeterMatrix.H₄ i j = 2) ∧
      (∀ i j, (wallRefl i * wallRefl j) ^ CoxeterMatrix.H₄ i j = 1) ∧
      (∀ i j, orderOf (wallRefl i * wallRefl j) = CoxeterMatrix.H₄ i j) ∧
      Subgroup.closure (Set.range wallRefl) = symmetryGroup ∧
      (∃ f : CoxeterMatrix.H₄.Group →* symmetryGroup, Function.Surjective f ∧
        ∀ i, (f (CoxeterMatrix.H₄.simple i) : ℍ ≃ₗᵢ[ℝ] ℍ) = wallRefl i) ∧
      (CoxeterSources → ∃ e : CoxeterMatrix.H₄.Group ≃* symmetryGroup,
        ∀ i, (e (CoxeterMatrix.H₄.simple i) : ℍ ≃ₗᵢ[ℝ] ℍ) = wallRefl i)) ∧
    (lpqHom.range = specialOrthogonal ∧ lpqHom.ker = Subgroup.zpowers (-1, -1)) ∧
    Infinite specialOrthogonal ∧
    (∀ c : Cap, Nat.card (alternatingGroup ↥(c : Finset (Fin 120))) = 12) ∧
    ∀ {Sticker : Type u} {Piece : Type v} [Fintype Sticker] [Fintype Piece] [DecidableEq Piece]
      [DecidableEq Sticker] {M : RetainedModel Sticker Piece} {R : HypothesisR M}
      {Color : Type w} [Fintype Color] [DecidableEq Color] {c₀ : Sticker → Color},
      HypothesisC R c₀ → Nat.card symmetryGroup < Nat.card M.legalGroup

theorem theoryPart2 : TheoryPart2.{u, v, w} := by
  obtain ⟨h1, h2, -⟩ := binaryIcosahedralGroup
  exact ⟨⟨h1, h2⟩, finiteSymmetryGroup, coxeterPresentation, doubleCoverSO4,
    infinite_specialOrthogonal, card_capRotations, fun C => C.card_symmetryGroup_lt⟩

/-- **Theorem, part 3: the retained cut model.** The 600 caps form 300 antipodal axes, and the
two outer layers of an axis, the cap regions at depth `α = 121/125`, are disjoint (the clause
`capDepth = 121 / 125` records the depth used in `capRegion`). That an outer-layer turn fixes
the middle layer is not formalized. Complete cap
signatures identify the components off the cuts, and `K` permutes them. The census rows add up
to `Σ N_o = 176520`, `600 + Σ k_o N_o = 259800` and `600 + Σ N_o = 177120`. Under R: 259800
sticker slots, 177120 pieces and 600 centres, which every legal element fixes, and `N_o` pieces
in each of the 35 moving orbits; the piece map is equivariant under legal elements; complete
signatures separate pieces, and with host facets they address stickers; the rank `|M(A)| - 1`
and the host count `|Host(A)|` are the census rank and `k_o`. Under F, `H_c` and `T_c` generate
the cap's full `A₄` action. -/
def TheoryPart3 : Prop :=
  ((∀ c, antipode (antipode c) = c ∧ antipode c ≠ c ∧ capPole (antipode c) = -capPole c ∧
      Disjoint (capRegion c) (capRegion (antipode c))) ∧
    #(univ.image fun c : Cap => ({c, antipode c} : Finset Cap)) = 300) ∧
    Fintype.card Cap = 600 ∧ capDepth = 121 / 125 ∧
    (∀ x y, x ∈ cutComplement → y ∈ cutComplement →
      (completeSignature x = completeSignature y ↔ y ∈ connectedComponentIn cutComplement x)) ∧
    (∀ (A : symmetryGroup) x,
      completeSignature ((A : ℍ ≃ₗᵢ[ℝ] ℍ) x) = (completeSignature x).image (A • ·)) ∧
    (∑ o, (census o).pieces = 176520 ∧
      600 + ∑ o, (census o).stickersPerPiece * (census o).pieces = 259800 ∧
      600 + ∑ o, (census o).pieces = 177120) ∧
    ∀ {Sticker : Type u} {Piece : Type v} [Fintype Sticker] [Fintype Piece] [DecidableEq Piece]
      {M : RetainedModel Sticker Piece} (R : HypothesisR M),
      Fintype.card Sticker = 259800 ∧ Fintype.card Piece = 177120 ∧
        #(univ.filter fun A => R.orbit A = none) = 600 ∧
        (∀ (g : M.legalGroup) s, R.orbit (M.piece s) = none → g.val s = s) ∧
        (∀ o, #(univ.filter fun A => R.orbit A = some o) = (census o).pieces) ∧
        (∀ (g : M.legalGroup) s, M.piece (g.val s) = R.pieceAction g (M.piece s)) ∧
        Function.Injective R.signature ∧
        Function.Injective (fun s => (R.signature (M.piece s), R.host s)) ∧
        (∀ A o, R.orbit A = some o →
          R.applicationRank A = (census o).rank ∧
            R.stickerMultiplicity A = (census o).stickersPerPiece ∧
            #(univ.filter fun s => M.piece s = A) = (census o).stickersPerPiece) ∧
        ∀ F : HypothesisF M, ∀ c, Subgroup.closure {M.H c, M.T c} = (F.capAction c).range

theorem theoryPart3 : TheoryPart3.{u, v} :=
  ⟨antipodalAxes, card_cap, rfl, fun _ _ hx hy => completeSignature_eq_iff hx hy,
    completeSignature_smul, censusSums, fun R => ⟨R.card_sticker, R.card_piece, R.card_centers,
      R.fixes_center, R.card_orbit, R.piece_legal, R.signature_injective, R.address_injective,
      R.rank_multiplicity, fun F => F.closure_primitives⟩⟩

/-- **Theorem, part 4: legal actions and conditional bounds.** The frame wreath product
multiplies chronologically: `(y * x).perm = y.perm * x.perm` and
`(y * x).frame i = y.frame (x.perm i) * x.frame i`. Under R, transported frames give an
injective homomorphism from `G` into `∏_o H_o^{N_o} ⋊ S_{N_o}`, of order
`∏_o N_o! |H_o|^{N_o}`, and the positional sign and orientation invariant has a kernel of index
`2^43 · 5^2`. Under R and I, `|G| ≤ ∏_o N_o! |H_o|^{N_o} / (2^43 · 5^2)`. Under R, the nine
selected orbits have 3600, 2400 and 7200 sticker slots, and under R and C, `G` contains
`A_3600 × A_2400 × A_7200^7` acting on them and fixing every other sticker, and there are at
least `B > 10^151851` reachable colourings. -/
def TheoryPart4 : Prop :=
  (∀ {H : Type u} {X : Type v} [Group H] (y x : FrameWreath H X) (i : X),
    (y * x).perm = y.perm * x.perm ∧ (y * x).frame i = y.frame (x.perm i) * x.frame i) ∧
    ∀ {Sticker : Type u} {Piece : Type v} [Fintype Sticker] [Fintype Piece] [DecidableEq Piece]
      {M : RetainedModel Sticker Piece} (R : HypothesisR M),
      (∃ CF : R.CoherentFrames,
        ∃ emb : M.legalGroup →* ∀ o : Fin 35, FrameWreath (R.frameGroup o) (R.Position o),
          Function.Injective emb ∧
            ∀ g o i f, g.val (CF.frame o (i, f)).val =
              (CF.frame o ((emb g o).perm i, ((emb g o).frame i : Perm _) f)).val) ∧
        Nat.card (∀ o : Fin 35, FrameWreath (R.frameGroup o) (R.Position o)) =
          ∏ o, (census o).pieces.factorial * (census o).frame.order ^ (census o).pieces ∧
        (invariantPi (H := fun o => ↥(R.frameGroup o)) (X := R.Position)).ker.index =
          2 ^ 43 * 5 ^ 2 ∧
        (HypothesisI R →
          Nat.card M.legalGroup * (2 ^ 43 * 5 ^ 2) ≤
              ∏ o, (census o).pieces.factorial * (census o).frame.order ^ (census o).pieces ∧
            Nat.card M.legalGroup ≤
              (∏ o, (census o).pieces.factorial * (census o).frame.order ^ (census o).pieces) /
                (2 ^ 43 * 5 ^ 2)) ∧
        (∀ i : ↥selectedOrbitIds, (R.orbitStickers (selectedOrbit i)).card =
          if i.val = 1 then 3600 else if i.val = 33 then 2400 else 7200) ∧
        ∀ [DecidableEq Sticker] {Color : Type w} [Fintype Color] [DecidableEq Color]
          {c₀ : Sticker → Color}, HypothesisC R c₀ →
          (∃ ρ : (∀ i, alternatingGroup ↥(R.orbitStickers (selectedOrbit i))) →* M.legalGroup,
            Function.Injective ρ ∧
              (∀ π i (s : R.orbitStickers (selectedOrbit i)), (ρ π).val s.val = ((π i).val s).val) ∧
              ∀ π, ((ρ π).val.support : Set Sticker) ⊆
                ⋃ i, (R.orbitStickers (selectedOrbit i) : Set Sticker)) ∧
            balancedColorCount ≤ {x : Sticker → Color | ∃ g : M.legalGroup,
              (∀ s, (∀ i, s ∉ R.orbitStickers (selectedOrbit i)) → g.val s = s) ∧
                x = c₀ ∘ g.val.symm}.ncard ∧
            balancedColorCount ≤ (reachableColorings M.legalGroup c₀).ncard ∧
            10 ^ 151851 < balancedColorCount

theorem theoryPart4 : TheoryPart4.{u, v, w} :=
  ⟨fun y x i => ⟨FrameWreath.mul_perm y x, FrameWreath.mul_frame y x i⟩, fun R =>
    ⟨R.transportedFrames, R.natCard_ambient, R.index_invariantKernel,
      fun I => ⟨I.invariantBound.1, I.invariantBound.2.1⟩, R.card_selectedOrbitStickers, fun C =>
        ⟨C.supportedAlternating, C.balancedColorBound.1, C.balancedColorBound.2,
          balancedColorCountGtPower⟩⟩⟩

/-- **Theorem, part 5: the word metric.** Under R and C, with the 1800 letters `H_c`, `T_c`,
`T_c⁻¹` (caps enumerated by any `e : Fin 600 ≃ Cap`), words realize every legal element, some
reachable colouring is at word distance at least 46648, and any distribution supported on the
colourings reached by 1000-letter words is at total-variation distance more than
`1 - 10^(-148595)` from uniform. Under F as well, each word has an inverse word of the same
length, so a recorded 1000-letter scramble is undone by 1000 letters. -/
def TheoryPart5 : Prop :=
  ∀ {Sticker : Type u} {Piece : Type v} [Fintype Sticker] [Fintype Piece] [DecidableEq Piece]
    [DecidableEq Sticker] {M : RetainedModel Sticker Piece} {R : HypothesisR M}
    {Color : Type w} [Fintype Color] [DecidableEq Color] {c₀ : Sticker → Color},
    HypothesisC R c₀ → ∀ e : Fin 600 ≃ Cap,
      Function.Surjective (wordPermutation (capLetterPermutation (M.capH e) (M.capT e))) ∧
      (∃ x : reachableColorings M.legalGroup c₀,
        46648 ≤ wordDistance M.legalGroup c₀ (capLetterPermutation (M.capH e) (M.capT e))
          (M.capWord_surjective e) x) ∧
      (∀ μ ν : Convexity.StdSimplex ℝ (reachableColorings M.legalGroup c₀),
        (∀ x ∈ μ.weights.support, ∃ w : List (Fin 600 × Fin 3),
          w.length = 1000 ∧
            wordColoring M.legalGroup c₀ (capLetterPermutation (M.capH e) (M.capT e)) w = x) →
        (∀ x, ν.weights x = 1 / ((reachableColorings M.legalGroup c₀).ncard : ℝ)) →
        1 - ((10 ^ 148595 : ℕ) : ℝ)⁻¹ < totalVariation μ ν) ∧
      (HypothesisF M → ∀ w : List (Fin 600 × Fin 3), ∃ w' : List (Fin 600 × Fin 3),
        w'.length = w.length ∧
          wordPermutation (capLetterPermutation (M.capH e) (M.capT e)) w' =
            (wordPermutation (capLetterPermutation (M.capH e) (M.capT e)) w)⁻¹)

theorem theoryPart5 : TheoryPart5.{u, v, w} := fun C e =>
  ⟨_root_.ArchonFull600Theory.RetainedModel.capWord_surjective _ e, C.wordDistanceBound e,
    fun μ ν hS hU => C.totalVariationBound e μ ν hS hU, fun F w => F.exists_inverse_word e w⟩

/-- **Theorem, part 6: the extended geometric action.** Under R, F and E, the geometric action
normalizes `G`, `G ∩ κ(K) = 1`, `(g, k) ↦ g κ(k)` is an isomorphism `G ⋊ K ≅ ⟨G, κ(K)⟩`, and
every element of `⟨G, κ(K)⟩` has a unique form `g κ(k)`. The source's illustration that
normalization need not preserve the word length of a chosen alphabet: in `A₄` with
`H = (1 2)(3 4)` and `T = (1 2 3)`, the conjugate `T H T⁻¹` is none of `H`, `T`, `T⁻¹` and
needs three letters. A symmetry group acting by
cost-preserving automorphisms of the state graph and preserving the goal set gives a quotient
with the same path costs to the goal. -/
def TheoryPart6 : Prop :=
  (∀ {Sticker : Type u} {Piece : Type v} [Fintype Sticker] [Fintype Piece] [DecidableEq Piece]
    {M : RetainedModel Sticker Piece} {R : HypothesisR M} {F : HypothesisF M}
    (E : HypothesisE R F),
    E.geom.range ≤ Subgroup.normalizer (M.legalGroup : Set (Perm Sticker)) ∧
      (∀ k, M.legalGroup.map (MulAut.conj (E.geom k)).toMonoidHom = M.legalGroup) ∧
      M.legalGroup ⊓ E.geom.range = ⊥ ∧
      (∃ e : M.legalGroup ⋊[E.conjAction] symmetryGroup ≃* ↥(M.legalGroup ⊔ E.geom.range),
        ∀ x, (e x : Perm Sticker) = (x.left : Perm Sticker) * E.geom x.right) ∧
      ∀ x ∈ M.legalGroup ⊔ E.geom.range,
        ∃! p : M.legalGroup × symmetryGroup, (p.1 : Perm Sticker) * E.geom p.2 = x) ∧
    (tetraThirdTurn * tetraHalfTurn * tetraThirdTurn⁻¹ ∈ alternatingGroup (StickerLabels 4) ∧
      orderOf (tetraThirdTurn * tetraHalfTurn * tetraThirdTurn⁻¹) = 2 ∧
      tetraThirdTurn * tetraHalfTurn * tetraThirdTurn⁻¹ ≠ tetraHalfTurn ∧
      tetraThirdTurn * tetraHalfTurn * tetraThirdTurn⁻¹ ∉
        Set.range (fun i => (tetraSymbols i : Perm _)) ∧
      (wordPermutation tetraSymbols [2, 0, 1] : Perm _) =
        tetraThirdTurn * tetraHalfTurn * tetraThirdTurn⁻¹ ∧
      (∀ w : List (Fin 3), (wordPermutation tetraSymbols w : Perm _) =
        tetraThirdTurn * tetraHalfTurn * tetraThirdTurn⁻¹ → 3 ≤ w.length) ∧
      (wordPermutation tetraSymbols [0] : Perm _) = tetraHalfTurn) ∧
    ∀ {X : Type u} {C : Type v} {L : Type w} [AddMonoid C] [Group L] [MulAction L X]
      (edge : X → X → C → Prop) (goal : Set X),
      (∀ (ℓ : L) u v c, edge u v c → edge (ℓ • u) (ℓ • v) c) →
      (∀ (ℓ : L) y, y ∈ goal → ℓ • y ∈ goal) → ∀ x c,
        ((∃ y ∈ goal, PathCost edge x y c) ↔
          ∃ q ∈ quotientGoal L goal, PathCost (quotientEdge L edge) (Quotient.mk _ x) q c)

theorem theoryPart6 : TheoryPart6.{u, v, w} :=
  ⟨fun E => ⟨E.range_le_normalizer, E.normalization⟩, tetrahedralConjugation,
    fun _ _ hedge hgoal x c => exists_goal_pathCost_iff hedge hgoal x c⟩

/-- **Theorem, part 7: sound optimization.** Transporting both endpoints computes the complete
nonfixed map of a conjugate (Lemma 14). Adding the inverse letters to a guarded alphabet cannot
increase a minimum unit-cost setup length, nor, with the old letters keeping their costs, a
minimum weighted cost; the inverse letters also fix the buffers. For the cap alphabet, the
inverse letters are legal and do not increase a setup length, and under the cap support
property the cap guard makes every letter fix the buffer pieces; under F the inverse letters
of the cap alphabet do not enlarge the reachable frames (Lemma 15). Equality on a target orbit
does not give equality of full actions (S1); disjoint visible supports do not give commutation
(S2); disjoint full supports do (S3). -/
def TheoryPart7 : Prop :=
  (∀ {Slot : Type u} (G : Subgroup (Perm Slot)) (W S : G),
    nonfixedMap (S * W * S⁻¹).val =
      (fun uv : Slot × Slot => (S.val uv.1, S.val uv.2)) '' nonfixedMap W.val) ∧
    (∀ {Sticker : Type u} {Letter : Type v} {k : ℕ} (moves : Letter → Perm Sticker)
      (f₀ f : Fin k → Sticker), setupLength (withInverses moves) f₀ f ≤ setupLength moves f₀ f) ∧
    (∀ {Sticker : Type u} {Letter : Type v} {C : Type w} [AddMonoid C] [CompleteLattice C]
      {k : ℕ} (moves : Letter → Perm Sticker) (cost : Letter ⊕ Letter → C)
      (f₀ f : Fin k → Sticker),
      pathDistance (frameEdge (withInverses moves) cost) f₀ f ≤
        pathDistance (frameEdge moves (cost ∘ Sum.inl)) f₀ f) ∧
    (∀ {Sticker : Type u} {Letter : Type v} (moves : Letter → Perm Sticker)
      (buffers : Set Sticker), (∀ a, ∀ s ∈ buffers, moves a s = s) →
        ∀ b, ∀ s ∈ buffers, withInverses moves b s = s) ∧
    (∀ {Sticker : Type u} {Piece : Type v} [Fintype Sticker] [Fintype Piece] [DecidableEq Piece]
      {M : RetainedModel Sticker Piece} {R : HypothesisR M}, R.CapSupport →
      ∀ {allowed : Set Cap} {A B : Piece}, (∀ c ∈ allowed, c ∉ R.signature A ∪ R.signature B) →
        (∀ b, withInverses (capLetters M allowed) b ∈ M.legalGroup) ∧
          (∀ b, ∀ s ∈ {s | M.piece s = A ∨ M.piece s = B},
            withInverses (capLetters M allowed) b s = s) ∧
          ∀ {n : ℕ} (f₀ f : Fin n → Sticker),
            setupLength (withInverses (capLetters M allowed)) f₀ f ≤
              setupLength (capLetters M allowed) f₀ f) ∧
    (∀ {Sticker : Type u} {Piece : Type v} {C : Type w} [AddMonoid C] {k : ℕ}
      {M : RetainedModel Sticker Piece}, HypothesisF M → ∀ (allowed : Set Cap)
      (cost : allowed ⊕ allowed → C) (cost' : (allowed ⊕ allowed) ⊕ (allowed ⊕ allowed) → C)
      (f₀ f : Fin k → Sticker),
      (∃ c, PathCost (frameEdge (withInverses (capLetters M allowed)) cost') f₀ f c) ↔
        ∃ c, PathCost (frameEdge (capLetters M allowed) cost) f₀ f c) ∧
    (let one : StickerLabels 6 := ⟨1, by decide⟩
      let two : StickerLabels 6 := ⟨2, by decide⟩
      let three : StickerLabels 6 := ⟨3, by decide⟩
      let four : StickerLabels 6 := ⟨4, by decide⟩
      let five : StickerLabels 6 := ⟨5, by decide⟩
      let six : StickerLabels 6 := ⟨6, by decide⟩
      let u := [one, two, three].formPerm
      let v := [four, five, six].formPerm * u
      u ∈ alternatingGroup (StickerLabels 6) ∧ v ∈ alternatingGroup (StickerLabels 6) ∧
        Set.EqOn u v {one, two, three} ∧ u ≠ v) ∧
    (let one : StickerLabels 7 := ⟨1, by decide⟩
      let two : StickerLabels 7 := ⟨2, by decide⟩
      let three : StickerLabels 7 := ⟨3, by decide⟩
      let four : StickerLabels 7 := ⟨4, by decide⟩
      let five : StickerLabels 7 := ⟨5, by decide⟩
      let six : StickerLabels 7 := ⟨6, by decide⟩
      let seven : StickerLabels 7 := ⟨7, by decide⟩
      let a := Equiv.swap one two * Equiv.swap five six
      let b := Equiv.swap three four * Equiv.swap six seven
      let displayed : Finset (StickerLabels 7) := {one, two, three, four}
      a.support ∩ displayed = {one, two} ∧ b.support ∩ displayed = {three, four} ∧
        Disjoint (a.support ∩ displayed) (b.support ∩ displayed) ∧
        ¬ Commute a b ∧ b (a five) = seven ∧ a (b five) = six) ∧
    ∀ {Slot : Type u} [Fintype Slot] [DecidableEq Slot] (P Q : Perm Slot),
      Disjoint P.support Q.support → ⇑Q ∘ ⇑P = ⇑P ∘ ⇑Q

theorem theoryPart7 : TheoryPart7.{u, v, w} :=
  ⟨transportNonfixedMap, setupLength_withInverses_le, pathDistance_withInverses_le,
    fun _ _ h => withInverses_fixes h, fun hsupp => fun hguard => capGuardedSetup hsupp hguard,
    fun F allowed _ _ f₀ f => capSetup_reachable_iff F allowed f₀ f, scopeSameRestriction,
    scopeVisibleDisjoint, disjointFullSupportsCommute⟩

/-- **Theorem: the Full 600-cell theory package.** The conjunction of the seven parts, each
under its own hypotheses. The clauses under R, F, I, C and E are implications: this package does
not construct a model that satisfies them. -/
theorem fullTheoryPackage :
    TheoryPart1 ∧ TheoryPart2.{u, v, w} ∧ TheoryPart3.{u, v} ∧ TheoryPart4.{u, v, w} ∧
      TheoryPart5.{u, v, w} ∧ TheoryPart6.{u, v, w} ∧ TheoryPart7.{u, v, w} :=
  ⟨theoryPart1, theoryPart2, theoryPart3, theoryPart4, theoryPart5, theoryPart6, theoryPart7⟩

end ArchonFull600Theory
