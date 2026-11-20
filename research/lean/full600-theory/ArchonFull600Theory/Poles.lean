import ArchonFull600Theory.Symmetry

set_option linter.style.header false in
/-!
# Lemma 5: the inradius and the cell-pole normalization

Source: `research/Full_600cell_Puzzle_Theory.tex`, Lemma 5 (lines 691 to 735).

The pole of a facet is the foot of the perpendicular from the centre `0` to its plane, `N / 4`
for the facet normal `N` of `Facets`. Every pole has norm `φ² / √2` (`norm_facetPole`), and this
number is the inradius of `P`: the largest closed ball about `0` inside `P` has this radius
(`isGreatest_inradius`). The proof that the ball lies in `P` shows first that `P` is the
intersection of its facet half-spaces (`polytope_eq_halfSpaces`), by a facet-cone argument: a
facet whose normal `N` maximizes `N · x` has `x` in the cone over its four vertices
(`exists_facetCone`).

`cellPoleNormalization` is Lemma 5: the cell pole `n₀ = (φ³, 1, 1, 1)` has
`‖n₀‖² = φ⁶ + 3 = 8 φ²` and `‖n₀‖ / r = 4 / φ`, and for each chosen facet there is a rotation
`Q ∈ SO(4)` such that the similarity `x ↦ (4 / φ) Q x` sends its pole to `n₀` and every facet
plane to a plane `ñ · y = ‖ñ‖²`.

Here a facet is one of the 600 four-cliques of the edge graph, as in `Facets`. These cut out `P`
(`polytope_eq_halfSpaces`), and under E4 they are all the facets of `P` (`facetCompleteness`).
-/

namespace ArchonFull600Theory

open scoped goldenRatio Quaternion RealInnerProductSpace
open Finset

/-! ### Identities in `ℤ[φ]` -/

theorem goldenRatio_pow_four : φ ^ 4 = 3 * φ + 2 := by
  linear_combination (φ ^ 2 + φ + 2) * Real.goldenRatio_sq

theorem goldenRatio_pow_six : φ ^ 6 = 8 * φ + 5 := by
  linear_combination (φ ^ 4 + φ ^ 3 + 2 * φ ^ 2 + 3 * φ + 5) * Real.goldenRatio_sq

/-! ### Poles and the inradius -/

/-- The pole of the facet of a four-clique: `N / 4`, the foot of the perpendicular from `0` to
the facet plane. -/
noncomputable def facetPole (s : Finset (Fin 120)) : ℍ := (4 : ℝ)⁻¹ • facetNormal s

/-- The inradius `φ² / √2` of the radius-two model. -/
noncomputable def inradius : ℝ := φ ^ 2 / Real.sqrt 2

/-- The cell pole `n₀ = (φ³, 1, 1, 1)`. -/
noncomputable def cellPole : ℍ := ⟨φ ^ 3, 1, 1, 1⟩

theorem inradius_pos : 0 < inradius :=
  div_pos (pow_pos Real.goldenRatio_pos 2) (Real.sqrt_pos.mpr two_pos)

theorem inradius_sq : inradius ^ 2 = φ ^ 4 / 2 := by
  rw [inradius, div_pow, Real.sq_sqrt zero_le_two, ← pow_mul]

theorem four_add_six_mul_goldenRatio_pos : 0 < 4 + 6 * φ := by
  linarith [Real.goldenRatio_pos]

section Clique

variable {s : Finset (Fin 120)} (hs : s ∈ edgeGraph.cliqueFinset 4)
include hs

theorem norm_sq_facetPole : ‖facetPole s‖ ^ 2 = (4 + 6 * φ) / 4 := by
  rw [facetPole, norm_smul, mul_pow, norm_sq_facetNormal hs, Real.norm_eq_abs,
    abs_of_pos (by norm_num : (0 : ℝ) < 4⁻¹)]
  ring

theorem norm_facetPole : ‖facetPole s‖ = inradius := by
  refine (pow_left_inj₀ (norm_nonneg _) inradius_pos.le two_ne_zero).mp ?_
  rw [norm_sq_facetPole hs, inradius_sq, goldenRatio_pow_four]
  ring

theorem norm_facetNormal : ‖facetNormal s‖ = 4 * inradius := by
  have h : facetNormal s = (4 : ℝ) • facetPole s := by
    rw [facetPole, smul_smul]
    norm_num
  rw [h, norm_smul, norm_facetPole hs, Real.norm_eq_abs, abs_of_pos (by norm_num : (0 : ℝ) < 4)]

omit hs in
theorem inner_facetPole (x : ℍ) : ⟪facetPole s, x⟫ = 4⁻¹ * ⟪facetNormal s, x⟫ := by
  rw [facetPole, real_inner_smul_left]

/-- The facet plane `N · x = 4 + 6 φ` is the plane `n · x = ‖n‖²` of the pole `n`. -/
theorem facetPlane_iff (x : ℍ) :
    ⟪facetNormal s, x⟫ = 4 + 6 * φ ↔ ⟪facetPole s, x⟫ = ‖facetPole s‖ ^ 2 := by
  rw [inner_facetPole, norm_sq_facetPole hs]
  constructor <;> intro h <;> linarith

theorem facetHalfSpace_iff (x : ℍ) :
    ⟪facetNormal s, x⟫ ≤ 4 + 6 * φ ↔ ⟪facetPole s, x⟫ ≤ ‖facetPole s‖ ^ 2 := by
  rw [inner_facetPole, norm_sq_facetPole hs]
  constructor <;> intro h <;> linarith

end Clique

/-! ### The facet-cone argument -/

theorem vertexR_mem_polytope (k : Fin 120) : vertexR k ∈ polytope :=
  subset_convexHull ℝ _ (by rw [← range_vertexR]; exact Set.mem_range_self k)

theorem zero_mem_polytope : (0 : ℍ) ∈ polytope := by
  have hneg : vertexR 4 = -vertexR 0 := by
    rw [vertexR_eq_smul, vertexR_eq_smul, unitVertex_four, unitVertex_zero, smul_neg]
  have h := convex_convexHull ℝ vertexSet (vertexR_mem_polytope 0) (vertexR_mem_polytope 4)
    (by norm_num : (0 : ℝ) ≤ 2⁻¹) (by norm_num : (0 : ℝ) ≤ 2⁻¹) (by norm_num)
  rwa [hneg, smul_neg, add_neg_cancel] at h

/-- The four vertices of a four-clique are linearly independent and span `ℍ`. -/
theorem linearIndependent_clique {s : Finset (Fin 120)} (hs : s ∈ edgeGraph.cliqueFinset 4) :
    LinearIndependent ℝ (vertexR ∘ ((↑) : s → Fin 120)) :=
  Matrix.posDef_gram_iff_linearIndependent.mp (facetGram hs).2

theorem card_clique_eq_finrank {s : Finset (Fin 120)} (hs : s ∈ edgeGraph.cliqueFinset 4) :
    Fintype.card s = Module.finrank ℝ ℍ := by
  rw [Fintype.card_coe, (SimpleGraph.mem_cliqueFinset_iff.mp hs).2, Quaternion.finrank_eq_four]

/-- Every point is a linear combination of the vertices of a four-clique. -/
theorem exists_eq_sum_clique {s : Finset (Fin 120)} (hs : s ∈ edgeGraph.cliqueFinset 4)
    (x : ℍ) : ∃ c : s → ℝ, x = ∑ k, c k • vertexR k := by
  have : Nonempty s :=
    Fintype.card_pos_iff.mp
      (by rw [card_clique_eq_finrank hs, Quaternion.finrank_eq_four]; norm_num)
  set b := basisOfLinearIndependentOfCardEqFinrank (linearIndependent_clique hs)
    (card_clique_eq_finrank hs)
  refine ⟨b.repr x, ?_⟩
  conv_lhs => rw [← b.sum_repr x]
  refine Finset.sum_congr rfl fun k _ => ?_
  rw [coe_basisOfLinearIndependentOfCardEqFinrank]
  rfl

/-- The neighbouring facet across the face opposite `j`: for `j` in a four-clique `s` there is a
vertex `j' ∉ s` such that `insert j' (s.erase j)` is a four-clique. -/
theorem exists_flip {s : Finset (Fin 120)} (hs : s ∈ edgeGraph.cliqueFinset 4) {j : Fin 120}
    (hj : j ∈ s) :
    ∃ j', j' ∉ s ∧ insert j' (s.erase j) ∈ edgeGraph.cliqueFinset 4 ∧
      ∀ k ∈ s.erase j, edgeGraph.Adj j' k := by
  have hs' := SimpleGraph.mem_cliqueFinset_iff.mp hs
  have hcl : edgeGraph.IsClique ((s.erase j : Finset (Fin 120)) : Set (Fin 120)) :=
    hs'.1.subset (by rw [coe_erase]; exact Set.sdiff_subset)
  have hct : #(s.erase j) = 3 := by rw [card_erase_of_mem hj, hs'.2]
  have ht : s.erase j ∈ edgeGraph.cliqueFinset 3 :=
    SimpleGraph.mem_cliqueFinset_iff.mpr ⟨hcl, hct⟩
  have hcard : 1 < #(commonNbrs edgeGraph (s.erase j)) := by
    rw [card_commonNbrs_triangle _ ht]
    norm_num
  obtain ⟨j', hj'c, hj'j⟩ := exists_mem_ne hcard j
  have hadj : ∀ k ∈ s.erase j, edgeGraph.Adj j' k := by
    simpa only [commonNbrs, mem_filter, mem_univ, true_and] using hj'c
  have hj't : j' ∉ s.erase j :=
    Finset.disjoint_right.mp (disjoint_commonNbrs edgeGraph (s.erase j)) hj'c
  have hj's : j' ∉ s := by
    intro h
    exact hj't (mem_erase.mpr ⟨hj'j, h⟩)
  refine ⟨j', hj's, SimpleGraph.mem_cliqueFinset_iff.mpr ⟨?_, ?_⟩, hadj⟩
  · rw [coe_insert, SimpleGraph.isClique_insert]
    exact ⟨hcl, fun k hk _ => hadj k hk⟩
  · rw [card_insert_of_notMem hj't, hct]

/-- **Facet cone.** For every `x ∈ ℍ`, a four-clique `s` whose facet normal maximizes `N · x` has
`x` in the cone over its four vertices. -/
theorem exists_facetCone (x : ℍ) :
    ∃ s ∈ edgeGraph.cliqueFinset 4, ∃ c : s → ℝ, (∀ k, 0 ≤ c k) ∧ x = ∑ k, c k • vertexR k ∧
      ∀ s' ∈ edgeGraph.cliqueFinset 4, ⟪facetNormal s', x⟫ ≤ ⟪facetNormal s, x⟫ := by
  obtain ⟨s, hs, hmax⟩ := exists_max_image (edgeGraph.cliqueFinset 4)
    (fun s => ⟪facetNormal s, x⟫) ⟨facet0, facet0_mem⟩
  obtain ⟨c, hx⟩ := exists_eq_sum_clique hs x
  refine ⟨s, hs, c, fun j => not_lt.mp fun hneg => ?_, hx, hmax⟩
  have hs' := SimpleGraph.mem_cliqueFinset_iff.mp hs
  obtain ⟨j', hj's, hs'', hadj⟩ := exists_flip hs j.2
  have hsum : facetNormal (insert j' (s.erase j)) - facetNormal s =
      vertexR j' - vertexR j := by
    rw [facetNormal, facetNormal, sum_insert (fun h => hj's (mem_of_mem_erase h)),
      ← add_sum_erase s vertexR j.2]
    abel
  have hk : ∀ k : s, k ≠ j → ⟪vertexR j' - vertexR j, vertexR k⟫ = 0 := by
    intro k hkj
    have hkj' : (k : Fin 120) ≠ j := fun h => hkj (Subtype.ext h)
    rw [inner_sub_left, (edgeGraph_adj_iff j' k).mp (hadj k (mem_erase.mpr ⟨hkj', k.2⟩)),
      (edgeGraph_adj_iff j k).mp (hs'.1 j.2 k.2 (Ne.symm hkj')), sub_self]
  have hval : ⟪vertexR j' - vertexR j, x⟫ = c j * (⟪vertexR j', vertexR j⟫ - 4) := by
    rw [hx, inner_sum, Finset.sum_eq_single j (fun k _ hkj => by
      rw [real_inner_smul_right, hk k hkj, mul_zero]) (fun h => absurd (mem_univ j) h),
      real_inner_smul_right, inner_sub_left, inner_vertexR_self]
  have hle : ⟪vertexR j', vertexR j⟫ ≤ 2 * φ :=
    inner_vertexR_le j' j fun h => hj's (h ▸ j.2)
  have hpos : 0 < ⟪vertexR j' - vertexR j, x⟫ := by
    rw [hval]
    exact mul_pos_of_neg_of_neg hneg (by linarith [Real.goldenRatio_lt_two])
  have hm := hmax _ hs''
  rw [← sub_nonpos, ← inner_sub_left, hsum] at hm
  linarith

/-- **H-representation.** `P` is the intersection of the half-spaces `N · x ≤ 4 + 6 φ` of its
facets (the four-cliques). -/
theorem polytope_eq_halfSpaces :
    polytope = {x | ∀ s ∈ edgeGraph.cliqueFinset 4, ⟪facetNormal s, x⟫ ≤ 4 + 6 * φ} := by
  ext x
  constructor
  · intro hx s hs
    exact (supportingFacet hs).2.2.2.1 x hx
  · intro hx
    obtain ⟨s, hs, c, hc, rfl, -⟩ := exists_facetCone x
    have hN : ⟪facetNormal s, ∑ k, c k • vertexR k⟫ = (4 + 6 * φ) * ∑ k, c k := by
      rw [inner_sum, mul_sum]
      refine Finset.sum_congr rfl fun k _ => ?_
      rw [real_inner_smul_right, inner_facetNormal_mem hs k.2, mul_comm]
    have hsum : ∑ k, c k ≤ 1 := by
      have h := hx s hs
      rw [hN] at h
      refine not_lt.mp fun hlt => ?_
      nlinarith [four_add_six_mul_goldenRatio_pos]
    have hconv := convex_convexHull ℝ vertexSet
    rcases (sum_nonneg fun k (_ : k ∈ univ) => hc k).eq_or_lt with h0 | hpos
    · have hz : ∀ k ∈ (univ : Finset s), c k = 0 := (sum_eq_zero_iff_of_nonneg
        fun k _ => hc k).mp h0.symm
      rw [sum_eq_zero fun k hk => by rw [hz k hk, zero_smul]]
      exact zero_mem_polytope
    · have hy : ∑ k, ((∑ i, c i)⁻¹ * c k) • vertexR k ∈ polytope :=
        hconv.sum_mem (fun k _ => mul_nonneg (inv_nonneg.mpr hpos.le) (hc k))
          (by rw [← mul_sum, inv_mul_cancel₀ hpos.ne']) fun k _ => vertexR_mem_polytope k
      have h := hconv.smul_mem_of_zero_mem zero_mem_polytope hy ⟨hpos.le, hsum⟩
      rwa [smul_sum, sum_congr rfl fun k _ => by
        rw [smul_smul, ← mul_assoc, mul_inv_cancel₀ hpos.ne', one_mul]] at h

/-- `P` is the intersection of the half-spaces `n · x ≤ ‖n‖²` of its facet poles. -/
theorem polytope_eq_poleHalfSpaces :
    polytope = {x | ∀ s ∈ edgeGraph.cliqueFinset 4, ⟪facetPole s, x⟫ ≤ ‖facetPole s‖ ^ 2} := by
  rw [polytope_eq_halfSpaces]
  ext x
  exact forall₂_congr fun s hs => facetHalfSpace_iff hs x

/-! ### The inradius -/

theorem closedBall_inradius_subset : Metric.closedBall (0 : ℍ) inradius ⊆ polytope := by
  intro x hx
  rw [Metric.mem_closedBall, dist_zero_right] at hx
  rw [polytope_eq_halfSpaces]
  intro s hs
  calc ⟪facetNormal s, x⟫ ≤ ‖facetNormal s‖ * ‖x‖ := real_inner_le_norm _ _
    _ ≤ ‖facetNormal s‖ * inradius := mul_le_mul_of_nonneg_left hx (norm_nonneg _)
    _ = 4 * inradius ^ 2 := by rw [norm_facetNormal hs]; ring
    _ = 4 + 6 * φ := by rw [inradius_sq, goldenRatio_pow_four]; ring

theorem inner_facetNormal_facetPole {s : Finset (Fin 120)} (hs : s ∈ edgeGraph.cliqueFinset 4) :
    ⟪facetNormal s, facetPole s⟫ = 4 + 6 * φ :=
  (facetPlane_iff hs _).mpr (real_inner_self_eq_norm_sq _)

/-- **Inradius.** `φ² / √2` is the largest radius of a closed ball about `0` inside `P`. -/
theorem isGreatest_inradius :
    IsGreatest {r | Metric.closedBall (0 : ℍ) r ⊆ polytope} inradius := by
  refine ⟨closedBall_inradius_subset, fun r hr => not_lt.mp fun hlt => ?_⟩
  have hr0 : 0 < r := inradius_pos.trans hlt
  set y := (r / inradius) • facetPole facet0
  have hy : y ∈ Metric.closedBall (0 : ℍ) r := by
    rw [Metric.mem_closedBall, dist_zero_right, norm_smul, norm_facetPole facet0_mem,
      Real.norm_eq_abs, abs_of_pos (div_pos hr0 inradius_pos),
      div_mul_cancel₀ _ inradius_pos.ne']
  have h := (supportingFacet facet0_mem).2.2.2.1 y (hr hy)
  rw [real_inner_smul_right, inner_facetNormal_facetPole facet0_mem] at h
  have h1 : 1 < r / inradius := (one_lt_div inradius_pos).mpr hlt
  nlinarith [four_add_six_mul_goldenRatio_pos]

/-! ### The cell pole -/

theorem norm_sq_cellPole : ‖cellPole‖ ^ 2 = φ ^ 6 + 3 := by
  rw [← real_inner_self_eq_norm_sq, Quaternion.inner_def]
  simp only [cellPole, Quaternion.re_mul, Quaternion.re_star, Quaternion.imI_star,
    Quaternion.imJ_star, Quaternion.imK_star]
  ring

theorem goldenRatio_pow_six_add_three : φ ^ 6 + 3 = 8 * φ ^ 2 := by
  rw [goldenRatio_pow_six, Real.goldenRatio_sq]
  ring

theorem norm_cellPole : ‖cellPole‖ = 4 / φ * inradius := by
  refine (pow_left_inj₀ (norm_nonneg _) (mul_nonneg (div_nonneg (by norm_num)
    Real.goldenRatio_pos.le) inradius_pos.le) two_ne_zero).mp ?_
  rw [norm_sq_cellPole, goldenRatio_pow_six_add_three, mul_pow, inradius_sq, div_pow]
  field_simp
  ring

theorem norm_cellPole_div_inradius : ‖cellPole‖ / inradius = 4 / φ := by
  rw [norm_cellPole, mul_div_assoc, div_self inradius_pos.ne', mul_one]

/-! ### A rotation between two directions -/

theorem smul_mem_unitary {x : ℍ} (hx : x ≠ 0) : ‖x‖⁻¹ • x ∈ unitary ℍ := by
  have hn : ‖‖x‖⁻¹ • x‖ = 1 := by
    rw [norm_smul, norm_inv, norm_norm, inv_mul_cancel₀ (norm_ne_zero_iff.mpr hx)]
  have h : Quaternion.normSq (‖x‖⁻¹ • x) = 1 := by
    rw [Quaternion.normSq_eq_norm_mul_self, hn, one_mul]
  rw [Unitary.mem_iff, Quaternion.star_mul_self, Quaternion.self_mul_star, h]
  exact ⟨by norm_num, by norm_num⟩

/-- For nonzero `x` and `y` there is a rotation `Q ∈ SO(4)` with `Q x = (‖x‖ / ‖y‖) y`. -/
theorem exists_rotation_eq (x y : ℍ) (hx : x ≠ 0) (hy : y ≠ 0) :
    ∃ Q ∈ specialOrthogonal, Q x = (‖x‖ / ‖y‖) • y := by
  set a : unitary ℍ := ⟨‖x‖⁻¹ • x, smul_mem_unitary hx⟩
  set b : unitary ℍ := ⟨‖y‖⁻¹ • y, smul_mem_unitary hy⟩
  refine ⟨lpq b a, mem_specialOrthogonal.mpr (orthDet_lpq b a), ?_⟩
  have hxa : x = ‖x‖ • (a : ℍ) := by
    change x = ‖x‖ • ‖x‖⁻¹ • x
    rw [smul_smul, mul_inv_cancel₀ (norm_ne_zero_iff.mpr hx), one_smul]
  have haa : (a : ℍ) * star (a : ℍ) = 1 := Unitary.mul_star_self_of_mem a.2
  rw [lpq_apply]
  conv_lhs => rw [hxa]
  rw [mul_smul_comm, smul_mul_assoc, mul_assoc, haa, mul_one]
  change ‖x‖ • ‖y‖⁻¹ • y = _
  rw [smul_smul, div_eq_mul_inv]

/-! ### Lemma 5 -/

/-- The similarity `x ↦ (4 / φ) Q x` of Lemma 5, for a rotation `Q`. -/
noncomputable def cellPoleMap (Q : ℍ ≃ₗᵢ[ℝ] ℍ) (x : ℍ) : ℍ := (4 / φ) • Q x

theorem four_div_goldenRatio_pos : 0 < 4 / φ := div_pos (by norm_num) Real.goldenRatio_pos

theorem inner_cellPoleMap (Q : ℍ ≃ₗᵢ[ℝ] ℍ) (x y : ℍ) :
    ⟪cellPoleMap Q x, cellPoleMap Q y⟫ = (4 / φ) ^ 2 * ⟪x, y⟫ := by
  rw [cellPoleMap, cellPoleMap, real_inner_smul_left, real_inner_smul_right,
    LinearIsometryEquiv.inner_map_map]
  ring

theorem norm_cellPoleMap (Q : ℍ ≃ₗᵢ[ℝ] ℍ) (x : ℍ) : ‖cellPoleMap Q x‖ = 4 / φ * ‖x‖ := by
  rw [cellPoleMap, norm_smul, LinearIsometryEquiv.norm_map, Real.norm_eq_abs,
    abs_of_pos four_div_goldenRatio_pos]

theorem cellPoleMap_bijective (Q : ℍ ≃ₗᵢ[ℝ] ℍ) : Function.Bijective (cellPoleMap Q) := by
  refine ⟨fun x y h => Q.injective (smul_right_injective ℍ four_div_goldenRatio_pos.ne' h),
    fun y => ⟨Q.symm ((4 / φ)⁻¹ • y), ?_⟩⟩
  rw [cellPoleMap, LinearIsometryEquiv.apply_symm_apply, smul_smul,
    mul_inv_cancel₀ four_div_goldenRatio_pos.ne', one_smul]

/-- The plane of a pole and its image: `n · x = ‖n‖²` iff `ñ · σ x = ‖ñ‖²` for `ñ = σ n`. -/
theorem cellPoleMap_plane_iff (Q : ℍ ≃ₗᵢ[ℝ] ℍ) (n x : ℍ) :
    ⟪n, x⟫ = ‖n‖ ^ 2 ↔ ⟪cellPoleMap Q n, cellPoleMap Q x⟫ = ‖cellPoleMap Q n‖ ^ 2 := by
  rw [inner_cellPoleMap, norm_cellPoleMap, mul_pow]
  exact (mul_right_inj' (pow_ne_zero 2 four_div_goldenRatio_pos.ne')).symm

theorem cellPoleMap_halfSpace_iff (Q : ℍ ≃ₗᵢ[ℝ] ℍ) (n x : ℍ) :
    ⟪n, x⟫ ≤ ‖n‖ ^ 2 ↔ ⟪cellPoleMap Q n, cellPoleMap Q x⟫ ≤ ‖cellPoleMap Q n‖ ^ 2 := by
  rw [inner_cellPoleMap, norm_cellPoleMap, mul_pow]
  exact (mul_le_mul_iff_of_pos_left (pow_pos four_div_goldenRatio_pos 2)).symm

/-- **Lemma 5 (cell-pole normalization).** Every facet pole `n = N / 4` has norm `φ² / √2`, with
`‖n‖² = (4 + 6 φ) / 4 = φ⁴ / 2`, and the facet plane is `n · x = ‖n‖²`; this norm is the inradius
of `P`, the largest radius of a closed ball about `0` inside `P`. The cell pole
`n₀ = (φ³, 1, 1, 1)` has `‖n₀‖² = φ⁶ + 3 = 8 φ²` and `‖n₀‖ / r = 4 / φ`. For each chosen facet
`s₀` there is a rotation `Q ∈ SO(4)` such that the similarity `σ x = (4 / φ) Q x` sends the pole
of `s₀` to `n₀` and every pole `n` to a pole `ñ = σ n` of norm `‖n₀‖`; it sends each
facet plane `n · x = ‖n‖²` to the plane `ñ · y = ‖ñ‖²` and each facet to the face of `σ P` on that
plane; and `σ P` is the intersection of the half-spaces `ñ · y ≤ ‖ñ‖²`. A facet is one of the
600 four-cliques of the edge graph (see the module docstring). -/
theorem cellPoleNormalization :
    (∀ s ∈ edgeGraph.cliqueFinset 4,
        ‖facetPole s‖ = inradius ∧ ‖facetPole s‖ ^ 2 = (4 + 6 * φ) / 4 ∧
          (4 + 6 * φ) / 4 = φ ^ 4 / 2 ∧
          ∀ x, ⟪facetNormal s, x⟫ = 4 + 6 * φ ↔ ⟪facetPole s, x⟫ = ‖facetPole s‖ ^ 2) ∧
      IsGreatest {r | Metric.closedBall (0 : ℍ) r ⊆ polytope} inradius ∧
      ‖cellPole‖ ^ 2 = φ ^ 6 + 3 ∧ φ ^ 6 + 3 = 8 * φ ^ 2 ∧ ‖cellPole‖ / inradius = 4 / φ ∧
      ∀ s₀ ∈ edgeGraph.cliqueFinset 4,
        ∃ Q ∈ specialOrthogonal, cellPoleMap Q (facetPole s₀) = cellPole ∧
          (∀ s ∈ edgeGraph.cliqueFinset 4,
            ‖cellPoleMap Q (facetPole s)‖ = ‖cellPole‖ ∧
              (∀ x, ⟪facetPole s, x⟫ = ‖facetPole s‖ ^ 2 ↔
                ⟪cellPoleMap Q (facetPole s), cellPoleMap Q x⟫ =
                  ‖cellPoleMap Q (facetPole s)‖ ^ 2) ∧
              cellPoleMap Q '' cliqueTetra s =
                {y ∈ cellPoleMap Q '' polytope |
                  ⟪cellPoleMap Q (facetPole s), y⟫ = ‖cellPoleMap Q (facetPole s)‖ ^ 2}) ∧
          cellPoleMap Q '' polytope =
            {y | ∀ s ∈ edgeGraph.cliqueFinset 4,
              ⟪cellPoleMap Q (facetPole s), y⟫ ≤ ‖cellPoleMap Q (facetPole s)‖ ^ 2} := by
  have hcell : cellPole ≠ 0 := by
    intro h
    have := norm_cellPole
    rw [h, norm_zero] at this
    exact (mul_pos four_div_goldenRatio_pos inradius_pos).ne this
  refine ⟨fun s hs => ⟨norm_facetPole hs, norm_sq_facetPole hs, ?_, facetPlane_iff hs⟩,
    isGreatest_inradius, norm_sq_cellPole, goldenRatio_pow_six_add_three,
    norm_cellPole_div_inradius, fun s₀ hs₀ => ?_⟩
  · rw [goldenRatio_pow_four]
    ring
  have hpole0 : facetPole s₀ ≠ 0 := by
    intro h
    have := norm_facetPole hs₀
    rw [h, norm_zero] at this
    exact inradius_pos.ne this
  obtain ⟨Q, hQ, hQn⟩ := exists_rotation_eq _ _ hpole0 hcell
  refine ⟨Q, hQ, ?_, fun s hs => ⟨?_, fun x => cellPoleMap_plane_iff Q _ x, ?_⟩, ?_⟩
  · rw [cellPoleMap, hQn, smul_smul, norm_facetPole hs₀, norm_cellPole]
    rw [show 4 / φ * (inradius / (4 / φ * inradius)) = 1 by
      field_simp [Real.goldenRatio_pos.ne', inradius_pos.ne'], one_smul]
  · rw [norm_cellPoleMap, norm_facetPole hs, norm_cellPole]
  · rw [← (supportingFacet hs).2.2.2.2]
    ext y
    constructor
    · rintro ⟨x, ⟨hxP, hx⟩, rfl⟩
      exact ⟨⟨x, hxP, rfl⟩, (cellPoleMap_plane_iff Q _ x).mp ((facetPlane_iff hs x).mp hx)⟩
    · rintro ⟨⟨x, hxP, rfl⟩, hy⟩
      exact ⟨x, ⟨hxP, (facetPlane_iff hs x).mpr ((cellPoleMap_plane_iff Q _ x).mpr hy)⟩, rfl⟩
  · ext y
    obtain ⟨x, rfl⟩ := (cellPoleMap_bijective Q).2 y
    rw [(cellPoleMap_bijective Q).1.mem_set_image, polytope_eq_poleHalfSpaces]
    exact forall₂_congr fun s _ => cellPoleMap_halfSpace_iff Q _ x

end ArchonFull600Theory
