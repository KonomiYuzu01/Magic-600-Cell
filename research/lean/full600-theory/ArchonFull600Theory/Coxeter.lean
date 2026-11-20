import ArchonFull600Theory.Poles
import Mathlib.GroupTheory.Coxeter.Basic

set_option linter.style.header false in
/-!
# Lemma 4: the Coxeter presentation of `K`

Source: `research/Full_600cell_Puzzle_Theory.tex`, Lemma 4 (statement lines 533 to 543; the
Coxeter part of the proof, lines 586 to 612).

The base flag of `∂P` is `{v₀} ⊂ {v₀, v₃₀} ⊂ {v₀, v₃₀, v₃₁} ⊂ {v₀, v₃₀, v₃₁, v₃₂}`
(`flagFace`). For each rank `i`, `wallRefl i` is the reflection in the hyperplane orthogonal to
`v_a - v_b`, where `(a, b)` is `(0, 30)`, `(30, 31)`, `(31, 32)` or `(32, 42)`. Kernel-checked
tables in `ℤ[φ]` give its action on the 120 vertices (`wallRefl_vertexR`). So `wallRefl i` lies
in `K`, fixes the faces of the flag of the other three ranks, moves the face of rank `i`, and is
the reflection in the span of the barycenters of those three faces (`chamberWalls`).

The four reflections satisfy the relations of the chain presentation `(3, 3, 5)`, which is
Mathlib's `CoxeterMatrix.H₄`, with exactly these orders. They generate `K`: explicit words in them
equal `L_{v₈/2, 1}`, `L_{v₃₀/2, 1}`, `L_{1, v₈/2}`, `L_{1, v₃₀/2}` and `J`, and these maps
generate `K` (`mem_symmetryGroup_iff_lpq` and the word tree of `BinaryIcosahedral`). Hence the
Coxeter group of type `H₄` maps onto `K` with `sᵢ ↦ wallRefl i` (`coxeterPresentation`).

The source proves that the presentation is complete through closed galleries in the dual cell
complex of `S³` (the second part of E5, together with Lemma 2). This file reaches the same
conclusion by another route. E5's first part, that the chain presentation defines `W(H₄)`, enters
as the hypothesis `CoxeterSources` in its standard quantitative form `|W(H₄)| = 14400`. A
surjection between groups of the same finite order is an isomorphism, so `W(H₄) ≅ K`. The gallery
argument is not formalized.
-/

namespace ArchonFull600Theory

open scoped Quaternion RealInnerProductSpace
open ZPhi Vec4 Finset

namespace Vec4

/-- A vector times an element of `ℤ[φ]`, in exact coordinates. -/
def smulZ (c : ZPhi) (v : Vec4) : Vec4 := ⟨mul c v.x0, mul c v.x1, mul c v.x2, mul c v.x3⟩

/-- The difference of two vectors, in exact coordinates. -/
def vsub (v w : Vec4) : Vec4 := ⟨sub v.x0 w.x0, sub v.x1 w.x1, sub v.x2 w.x2, sub v.x3 w.x3⟩

/-- `(u · u) v - 2 (v · u) u`: the reflection of `v` in the hyperplane orthogonal to `u`, times
`u · u`. -/
def reflScaled (u v : Vec4) : Vec4 := vsub (smulZ (dot u u) v) (smulZ (add (dot v u) (dot v u)) u)

theorem toQuat_smulZ (c : ZPhi) (v : Vec4) : toQuat (smulZ c v) = toReal c • toQuat v := by
  ext <;> simp only [toQuat, smulZ, toReal_mul, Quaternion.re_smul, Quaternion.imI_smul,
    Quaternion.imJ_smul, Quaternion.imK_smul, smul_eq_mul]

theorem toQuat_vsub (v w : Vec4) : toQuat (vsub v w) = toQuat v - toQuat w := by
  ext <;> simp only [toQuat, vsub, toReal_sub, Quaternion.re_sub, Quaternion.imI_sub,
    Quaternion.imJ_sub, Quaternion.imK_sub]

theorem toQuat_reflScaled (u v : Vec4) :
    toQuat (reflScaled u v) = toReal (dot u u) • toQuat v - (2 * toReal (dot v u)) • toQuat u := by
  rw [reflScaled, toQuat_vsub, toQuat_smulZ, toQuat_smulZ, toReal_add, two_mul]

end Vec4

/-! ### The wall reflections -/

/-- The first vertex of wall `i`. -/
def wallA : Fin 4 → Fin 120 := ![0, 30, 31, 32]

/-- The second vertex of wall `i`: the reflection in wall `i` exchanges `v_{wallA i}` and
`v_{wallB i}`. -/
def wallB : Fin 4 → Fin 120 := ![30, 31, 32, 42]

/-- The wall reflection of rank 0 on vertex indices: it exchanges `v0` and `v30`. -/
def wallTable0 : List ℕ := [
  30, 1, 63, 57, 114, 5, 75, 93, 43, 9, 34, 24, 91, 13, 82, 72, 60, 70, 18, 55, 108, 118, 22, 103,
  11, 83, 53, 78, 28, 80, 0, 31, 32, 90, 10, 40, 58, 37, 65, 39, 35, 92, 42, 8, 44, 105, 48, 85, 46,
  107, 50, 81, 52, 26, 117, 19, 101, 3, 36, 64, 16, 61, 62, 2, 59, 38, 111, 67, 113, 87, 17, 109,
  15, 73, 74, 6, 95, 98, 27, 79, 29, 51, 14, 25, 106, 47, 86, 69, 88, 110, 33, 12, 41, 7, 96, 76,
  94, 97, 77, 99, 119, 56, 102, 23, 104, 45, 84, 49, 20, 71, 89, 66, 112, 68, 4, 115, 116, 54, 21,
  100]

/-- The wall reflection of rank 1 on vertex indices: it exchanges `v30` and `v31`. -/
def wallTable1 : List ℕ := [
  0, 38, 36, 37, 4, 98, 96, 97, 8, 41, 39, 52, 40, 51, 53, 15, 16, 89, 87, 100, 88, 99, 101, 23, 24,
  26, 25, 28, 27, 29, 31, 30, 32, 35, 34, 33, 2, 3, 1, 10, 12, 9, 42, 44, 43, 47, 46, 45, 86, 85,
  84, 13, 11, 14, 55, 54, 56, 57, 59, 58, 61, 60, 62, 64, 63, 65, 66, 68, 67, 69, 71, 70, 73, 72,
  74, 76, 75, 77, 78, 80, 79, 81, 83, 82, 50, 49, 48, 18, 20, 17, 91, 90, 92, 93, 95, 94, 6, 7, 5,
  21, 19, 22, 102, 104, 103, 107, 106, 105, 108, 110, 109, 112, 111, 113, 115, 114, 116, 119, 118,
  117]

/-- The wall reflection of rank 2 on vertex indices: it exchanges `v31` and `v32`. -/
def wallTable2 : List ℕ := [
  0, 37, 38, 36, 4, 97, 98, 96, 8, 39, 40, 53, 41, 52, 51, 15, 16, 87, 88, 101, 89, 100, 99, 23, 26,
  25, 24, 29, 28, 27, 30, 32, 31, 33, 35, 34, 3, 1, 2, 9, 10, 12, 44, 43, 42, 45, 47, 46, 85, 84,
  86, 14, 13, 11, 54, 56, 55, 58, 57, 59, 60, 62, 61, 65, 64, 63, 68, 67, 66, 70, 69, 71, 72, 74,
  73, 77, 76, 75, 80, 79, 78, 82, 81, 83, 49, 48, 50, 17, 18, 20, 90, 92, 91, 94, 93, 95, 7, 5, 6,
  22, 21, 19, 104, 103, 102, 105, 107, 106, 110, 109, 108, 113, 112, 111, 114, 116, 115, 117, 119,
  118]

/-- The wall reflection of rank 3 on vertex indices: it exchanges `v32` and `v42`. -/
def wallTable3 : List ℕ := [
  0, 73, 26, 84, 4, 61, 110, 48, 29, 28, 51, 11, 12, 39, 64, 65, 77, 76, 99, 19, 20, 87, 112, 113,
  24, 36, 2, 41, 9, 8, 30, 31, 42, 33, 81, 35, 25, 50, 72, 13, 40, 27, 32, 80, 79, 45, 93, 47, 7,
  49, 37, 10, 52, 63, 54, 55, 66, 106, 83, 82, 98, 5, 74, 53, 14, 15, 56, 104, 103, 118, 95, 94, 38,
  1, 62, 89, 17, 16, 92, 44, 43, 34, 59, 58, 3, 85, 97, 21, 88, 75, 90, 91, 78, 46, 71, 70, 109, 86,
  60, 18, 100, 111, 116, 68, 67, 105, 57, 107, 108, 96, 6, 101, 22, 23, 114, 115, 102, 117, 69, 119]

/-- A word for `L_{v₈/2, 1}`. -/
def wordLeft8 : List (Fin 4) := [0, 1, 2, 3, 2, 3, 1, 2, 3, 0, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1]

/-- A word for `L_{v₃₀/2, 1}`. -/
def wordLeft30 : List (Fin 4) := [0, 1, 2, 3, 2, 3, 1, 2, 3, 1, 2, 1]

/-- A word for `L_{1, v₈/2}`. -/
def wordRight8 : List (Fin 4) := [3, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 0, 1, 2, 3, 2, 0, 1, 0]

/-- A word for `L_{1, v₃₀/2}`. -/
def wordRight30 : List (Fin 4) := [1, 2, 3, 2, 3, 1, 2, 3, 1, 2, 1, 0]

/-- A word for `J`, quaternion conjugation. -/
def wordConj : List (Fin 4) := [3, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 3, 1, 2, 1]

/-- The table of wall `i`. -/
def wallTable : Fin 4 → List ℕ := ![wallTable0, wallTable1, wallTable2, wallTable3]

/-- The normal `u = v_a - v_b` of wall `i`, in exact coordinates. -/
def wallVecZ (i : Fin 4) : Vec4 := vsub (vertexN (wallA i)) (vertexN (wallB i))

/-- A wall table checked in exact coordinates: with `u = v_a - v_b`,
`(u · u) v_{t[k]} = (u · u) v_k - 2 (v_k · u) u` for every vertex `k`. -/
def reflCheck (a b : ℕ) (t : List ℕ) : Bool :=
  (List.range 120).all fun k =>
    decide (smulZ (dot (vsub (vertexN a) (vertexN b)) (vsub (vertexN a) (vertexN b)))
      (vertexN (tableAt t k)) = reflScaled (vsub (vertexN a) (vertexN b)) (vertexN k))

theorem reflCheck_0 : reflCheck 0 30 wallTable0 = true := by decide +kernel

theorem reflCheck_1 : reflCheck 30 31 wallTable1 = true := by decide +kernel

theorem reflCheck_2 : reflCheck 31 32 wallTable2 = true := by decide +kernel

theorem reflCheck_3 : reflCheck 32 42 wallTable3 = true := by decide +kernel

theorem reflCheck_wall (i : Fin 4) : reflCheck (wallA i) (wallB i) (wallTable i) = true := by
  fin_cases i
  exacts [reflCheck_0, reflCheck_1, reflCheck_2, reflCheck_3]

/-- The reflection in wall `i` as a map on vertex indices. -/
def wallPerm (i : Fin 4) : Fin 120 → Fin 120 := tableFin (wallTable i)

/-- The normal `u_i = v_a - v_b` of wall `i`. -/
noncomputable def wallVec (i : Fin 4) : ℍ := vertexR (wallA i) - vertexR (wallB i)

/-- **The wall reflection `sᵢ`**: the reflection in the hyperplane orthogonal to `u_i`. -/
noncomputable def wallRefl (i : Fin 4) : ℍ ≃ₗᵢ[ℝ] ℍ := (ℝ ∙ wallVec i)ᗮ.reflection

theorem wallVec_ne_zero (i : Fin 4) : wallVec i ≠ 0 :=
  sub_ne_zero.mpr (vertexR_injective.ne (by fin_cases i <;> decide))

theorem wallRefl_apply (i : Fin 4) (x : ℍ) :
    wallRefl i x = x - (2 * ⟪wallVec i, x⟫ / ‖wallVec i‖ ^ 2) • wallVec i := by
  rw [wallRefl, Submodule.reflection_orthogonal_apply, Submodule.reflection_singleton_apply]
  simp only [RCLike.ofReal_real_eq_id, id]
  module

theorem wallRefl_vertexR (i : Fin 4) (k : Fin 120) :
    wallRefl i (vertexR k) = vertexR (wallPerm i k) := by
  have hU : toQuat (wallVecZ i) = wallVec i := toQuat_vsub _ _
  have hc : toReal (dot (wallVecZ i) (wallVecZ i)) = ‖wallVec i‖ ^ 2 := by
    rw [← inner_toQuat, hU, real_inner_self_eq_norm_sq]
  have hd : toReal (dot (vertexN k) (wallVecZ i)) = ⟪wallVec i, vertexR k⟫ := by
    rw [← inner_toQuat, hU, real_inner_comm]
    rfl
  have h : smulZ (dot (wallVecZ i) (wallVecZ i)) (vertexN (tableAt (wallTable i) k)) =
      reflScaled (wallVecZ i) (vertexN k) :=
    of_decide_eq_true (all_range_iff.mp (reflCheck_wall i) k k.2)
  have h' := congrArg toQuat h
  rw [toQuat_smulZ, toQuat_reflScaled, hU, hc, hd] at h'
  change ‖wallVec i‖ ^ 2 • vertexR (wallPerm i k) =
    ‖wallVec i‖ ^ 2 • vertexR k - (2 * ⟪wallVec i, vertexR k⟫) • wallVec i at h'
  have hc0 : ‖wallVec i‖ ^ 2 ≠ 0 := pow_ne_zero 2 (norm_ne_zero_iff.mpr (wallVec_ne_zero i))
  rw [wallRefl_apply]
  refine smul_right_injective ℍ hc0 ?_
  dsimp only
  rw [h', smul_sub, smul_smul, mul_div_cancel₀ _ hc0]

theorem wallRefl_mem (i : Fin 4) : wallRefl i ∈ symmetryGroup :=
  mem_symmetryGroup_of_forall fun k => ⟨_, wallRefl_vertexR i k⟩

theorem wallPerm_wallPerm (i : Fin 4) (k : Fin 120) : wallPerm i (wallPerm i k) = k :=
  vertexR_injective (by
    rw [← wallRefl_vertexR, ← wallRefl_vertexR, wallRefl]
    exact Submodule.reflection_reflection _ _)

theorem wallPerm_injective (i : Fin 4) : Function.Injective (wallPerm i) :=
  Function.LeftInverse.injective (wallPerm_wallPerm i)

/-! ### The base chamber -/

/-- The faces of the base flag, by rank: the vertex `v₀`, the edge `v₀ v₃₀`, the triangle
`v₀ v₃₀ v₃₁` and the facet `v₀ v₃₀ v₃₁ v₃₂`. -/
def flagFace : Fin 4 → Finset (Fin 120) := ![{0}, {0, 30}, {0, 30, 31}, facet0]

theorem flagFace_isNClique : ∀ j : Fin 4, edgeGraph.IsNClique ((j : ℕ) + 1) (flagFace j) := by
  decide +kernel

theorem flagFace_mono :
    flagFace 0 ⊆ flagFace 1 ∧ flagFace 1 ⊆ flagFace 2 ∧ flagFace 2 ⊆ flagFace 3 := by
  decide +kernel

/-- The barycenter of the face of rank `j` of the base flag. -/
noncomputable def flagBarycenter (j : Fin 4) : ℍ :=
  ((#(flagFace j) : ℕ) : ℝ)⁻¹ • ∑ k ∈ flagFace j, vertexR k

theorem flagBarycenter_zero : flagBarycenter 0 = vertexR 0 := by
  rw [flagBarycenter, show flagFace 0 = {0} from rfl, sum_singleton, card_singleton]
  simp

theorem flagBarycenter_one : flagBarycenter 1 = (2 : ℝ)⁻¹ • (vertexR 0 + vertexR 30) := by
  rw [flagBarycenter, show flagFace 1 = {0, 30} from rfl, sum_pair (by decide),
    card_pair (by decide)]
  norm_num

theorem flagBarycenter_two :
    flagBarycenter 2 = (3 : ℝ)⁻¹ • (vertexR 0 + vertexR 30 + vertexR 31) := by
  rw [flagBarycenter, show flagFace 2 = {0, 30, 31} from rfl, sum_insert (by decide),
    sum_pair (by decide), show #({0, 30, 31} : Finset (Fin 120)) = 3 by decide, add_assoc]
  norm_num

theorem flagBarycenter_three :
    flagBarycenter 3 = (4 : ℝ)⁻¹ • (vertexR 0 + vertexR 30 + vertexR 31 + vertexR 32) := by
  rw [flagBarycenter, show flagFace 3 = {0, 30, 31, 32} from rfl, sum_insert (by decide),
    sum_insert (by decide), sum_pair (by decide),
    show #({0, 30, 31, 32} : Finset (Fin 120)) = 4 by decide, add_assoc, add_assoc]
  norm_num

/-- The four barycenters of the base flag span `ℍ`. -/
theorem span_flagBarycenter : Submodule.span ℝ (Set.range flagBarycenter) = ⊤ := by
  have : Nonempty facet0 := ⟨⟨0, by decide⟩⟩
  rw [eq_top_iff, ← (linearIndependent_clique facet0_mem).span_eq_top_of_card_eq_finrank
    (card_clique_eq_finrank facet0_mem), Submodule.span_le]
  have hb : ∀ j, flagBarycenter j ∈ Submodule.span ℝ (Set.range flagBarycenter) :=
    fun j => Submodule.subset_span ⟨j, rfl⟩
  rintro _ ⟨⟨k, hk⟩, rfl⟩
  simp only [facet0, mem_insert, mem_singleton] at hk
  simp only [Function.comp_apply, SetLike.mem_coe]
  rcases hk with rfl | rfl | rfl | rfl
  · simpa only [flagBarycenter_zero] using hb 0
  · have h : vertexR 30 = (2 : ℝ) • flagBarycenter 1 - flagBarycenter 0 := by
      rw [flagBarycenter_one, flagBarycenter_zero, smul_smul]
      norm_num
    rw [h]
    exact sub_mem (Submodule.smul_mem _ _ (hb 1)) (hb 0)
  · have h : vertexR 31 = (3 : ℝ) • flagBarycenter 2 - (2 : ℝ) • flagBarycenter 1 := by
      rw [flagBarycenter_two, flagBarycenter_one, smul_smul, smul_smul]
      norm_num
    rw [h]
    exact sub_mem (Submodule.smul_mem _ _ (hb 2)) (Submodule.smul_mem _ _ (hb 1))
  · have h : vertexR 32 = (4 : ℝ) • flagBarycenter 3 - (3 : ℝ) • flagBarycenter 2 := by
      rw [flagBarycenter_three, flagBarycenter_two, smul_smul, smul_smul]
      norm_num
    rw [h]
    exact sub_mem (Submodule.smul_mem _ _ (hb 3)) (Submodule.smul_mem _ _ (hb 2))

/-- A linear isometry that fixes the four barycenters of a flag is the identity. -/
theorem eq_one_of_fix_flagBarycenter {A : ℍ ≃ₗᵢ[ℝ] ℍ}
    (h : ∀ j, A (flagBarycenter j) = flagBarycenter j) : A = 1 := by
  have hspan : ∀ x ∈ Submodule.span ℝ (Set.range flagBarycenter), A x = x := by
    intro x hx
    induction hx using Submodule.span_induction with
    | mem x hx =>
      obtain ⟨j, rfl⟩ := hx
      exact h j
    | zero => exact map_zero A
    | add x y _ _ hx hy => rw [map_add, hx, hy]
    | smul a x _ hx => rw [map_smul, hx]
  refine LinearIsometryEquiv.ext fun x => ?_
  exact hspan x (by rw [span_flagBarycenter]; trivial)

/-- The reflection in wall `i` fixes the faces of the base flag of the other ranks and moves the
face of rank `i`. -/
theorem image_flagFace_wallPerm :
    ∀ i j : Fin 4, j ≠ i → (flagFace j).image (wallPerm i) = flagFace j := by
  decide +kernel

theorem image_flagFace_wallPerm_self :
    ∀ i : Fin 4, (flagFace i).image (wallPerm i) ≠ flagFace i := by
  decide +kernel

theorem wallRefl_flagBarycenter {i j : Fin 4} (h : j ≠ i) :
    wallRefl i (flagBarycenter j) = flagBarycenter j := by
  rw [flagBarycenter, map_smul, map_sum]
  simp_rw [wallRefl_vertexR]
  rw [← sum_image (wallPerm_injective i).injOn, image_flagFace_wallPerm i j h]

theorem reflection_congr {K L : Submodule ℝ ℍ} [K.HasOrthogonalProjection]
    [L.HasOrthogonalProjection] (h : K = L) : K.reflection = L.reflection := by
  subst h
  rfl

/-- The mirror of wall `i` is the span of the barycenters of the faces of the other three ranks. -/
theorem span_flagBarycenter_compl (i : Fin 4) :
    Submodule.span ℝ (flagBarycenter '' {i}ᶜ) = (ℝ ∙ wallVec i)ᗮ := by
  have hle : Submodule.span ℝ (flagBarycenter '' {i}ᶜ) ≤ (ℝ ∙ wallVec i)ᗮ := by
    rw [Submodule.span_le]
    rintro _ ⟨j, hj, rfl⟩
    exact (Submodule.reflection_eq_self_iff _).mp (wallRefl_flagBarycenter hj)
  refine Submodule.eq_of_le_of_finrank_le hle ?_
  have : Fact (Module.finrank ℝ ℍ = 3 + 1) := ⟨Quaternion.finrank_eq_four⟩
  rw [Submodule.finrank_orthogonal_span_singleton (n := 3) (wallVec_ne_zero i)]
  have hins : Set.range flagBarycenter = insert (flagBarycenter i) (flagBarycenter '' {i}ᶜ) := by
    ext x
    constructor
    · rintro ⟨j, rfl⟩
      by_cases hj : j = i
      · exact Or.inl (by rw [hj])
      · exact Or.inr ⟨j, hj, rfl⟩
    · rintro (rfl | ⟨j, -, rfl⟩) <;> exact ⟨_, rfl⟩
  have h := Submodule.finrank_add_le_finrank_add_finrank (ℝ ∙ flagBarycenter i)
    (Submodule.span ℝ (flagBarycenter '' {i}ᶜ))
  rw [← Submodule.span_insert, ← hins, span_flagBarycenter, finrank_top,
    Quaternion.finrank_eq_four] at h
  have h1 : Module.finrank ℝ (ℝ ∙ flagBarycenter i) ≤ 1 := by
    simpa using finrank_span_le_card ({flagBarycenter i} : Set ℍ)
  have h3 : 3 ≤ Module.finrank ℝ (Submodule.span ℝ (flagBarycenter '' {i}ᶜ)) := by omega
  exact h3

/-- **Chambers and walls.** The base flag is a chain of cliques of the edge graph (a vertex, an
edge, a triangle and a facet). Its four barycenters span `ℍ`, so a linear isometry that fixes
them is the identity. For each rank `i`, the wall reflection `sᵢ` is a symmetry of `P`; it fixes
the faces of the flag of the other ranks (so it fixes their barycenters) and moves the face of
rank `i`; and it is the reflection in the span of the other three barycenters. The chambers
themselves (the radially projected simplices of the barycentric subdivision) are not defined. -/
theorem chamberWalls :
    (∀ j : Fin 4, edgeGraph.IsNClique ((j : ℕ) + 1) (flagFace j)) ∧
      (flagFace 0 ⊆ flagFace 1 ∧ flagFace 1 ⊆ flagFace 2 ∧ flagFace 2 ⊆ flagFace 3) ∧
      Submodule.span ℝ (Set.range flagBarycenter) = ⊤ ∧
      (∀ A : ℍ ≃ₗᵢ[ℝ] ℍ, (∀ j, A (flagBarycenter j) = flagBarycenter j) → A = 1) ∧
      ∀ i : Fin 4, wallRefl i ∈ symmetryGroup ∧
        (∀ k, wallRefl i (vertexR k) = vertexR (wallPerm i k)) ∧
        (∀ j, j ≠ i → (flagFace j).image (wallPerm i) = flagFace j) ∧
        (flagFace i).image (wallPerm i) ≠ flagFace i ∧
        (∀ j, j ≠ i → wallRefl i (flagBarycenter j) = flagBarycenter j) ∧
        wallRefl i = (Submodule.span ℝ (flagBarycenter '' {i}ᶜ)).reflection :=
  ⟨flagFace_isNClique, flagFace_mono, span_flagBarycenter,
    fun _ h => eq_one_of_fix_flagBarycenter h, fun i =>
    ⟨wallRefl_mem i, wallRefl_vertexR i, image_flagFace_wallPerm i,
      image_flagFace_wallPerm_self i, fun _ h => wallRefl_flagBarycenter h,
      reflection_congr (span_flagBarycenter_compl i).symm⟩⟩

/-! ### The Coxeter relations -/

/-- Mathlib's `CoxeterMatrix.H₄` is the chain presentation `(3, 3, 5)` of the source:
`m_ii = 1`, `m_01 = m_12 = 3`, `m_23 = 5`, and `m_ij = 2` when `|i - j| > 1`. -/
theorem H₄_chain :
    (∀ i, CoxeterMatrix.H₄ i i = 1) ∧ CoxeterMatrix.H₄ 0 1 = 3 ∧ CoxeterMatrix.H₄ 1 2 = 3 ∧
      CoxeterMatrix.H₄ 2 3 = 5 ∧ ∀ i j : Fin 4, (i : ℕ) + 1 < j → CoxeterMatrix.H₄ i j = 2 := by
  refine ⟨by decide +kernel, by decide +kernel, by decide +kernel, by decide +kernel, ?_⟩
  intro i j
  fin_cases i <;> fin_cases j <;> decide +kernel

theorem wallRefl_mul_apply (i j : Fin 4) (k : Fin 120) :
    (wallRefl i * wallRefl j) (vertexR k) = vertexR ((wallPerm i ∘ wallPerm j) k) := by
  rw [LinearIsometryEquiv.coe_mul, Function.comp_apply, wallRefl_vertexR, wallRefl_vertexR]
  rfl

theorem pow_apply_vertexR {A : ℍ ≃ₗᵢ[ℝ] ℍ} {σ : Fin 120 → Fin 120}
    (h : ∀ k, A (vertexR k) = vertexR (σ k)) (m : ℕ) (k : Fin 120) :
    (A ^ m) (vertexR k) = vertexR (σ^[m] k) := by
  induction m generalizing k with
  | zero =>
    rw [pow_zero]
    rfl
  | succ m ih =>
    rw [pow_succ, LinearIsometryEquiv.coe_mul, Function.comp_apply, h, ih,
      Function.iterate_succ_apply]

/-- The rank-two checks: `(sᵢ sⱼ)^{m_ij}` fixes the base facet, and no smaller positive power
does. -/
theorem pairCheck (i j : Fin 4) :
    (∀ k ∈ facet0, (wallPerm i ∘ wallPerm j)^[CoxeterMatrix.H₄ i j] k = k) ∧
      ∀ m < CoxeterMatrix.H₄ i j, 0 < m → ∃ k ∈ facet0, (wallPerm i ∘ wallPerm j)^[m] k ≠ k := by
  fin_cases i <;> fin_cases j <;> decide +kernel

theorem wallRefl_mul_pow (i j : Fin 4) : (wallRefl i * wallRefl j) ^ CoxeterMatrix.H₄ i j = 1 := by
  refine eq_of_eqOn_clique facet0_mem fun k hk => ?_
  rw [pow_apply_vertexR (wallRefl_mul_apply i j), (pairCheck i j).1 k hk]
  rfl

theorem orderOf_wallRefl_mul (i j : Fin 4) :
    orderOf (wallRefl i * wallRefl j) = CoxeterMatrix.H₄ i j := by
  have hpos : 0 < CoxeterMatrix.H₄ i j := by
    fin_cases i <;> fin_cases j <;> decide +kernel
  rw [orderOf_eq_iff hpos]
  refine ⟨wallRefl_mul_pow i j, fun m hm hm0 h1 => ?_⟩
  obtain ⟨k, -, hk⟩ := (pairCheck i j).2 m hm hm0
  apply hk
  apply vertexR_injective
  rw [← pow_apply_vertexR (wallRefl_mul_apply i j), h1]
  rfl

/-! ### Generation -/

/-- The product `s_{w₀} s_{w₁} ⋯` of the wall reflections along a word. -/
noncomputable def wordRefl (w : List (Fin 4)) : ℍ ≃ₗᵢ[ℝ] ℍ := (w.map wallRefl).prod

/-- The same product as a map on vertex indices. -/
def wordPerm (w : List (Fin 4)) (k : Fin 120) : Fin 120 := w.foldr wallPerm k

theorem wordRefl_vertexR (w : List (Fin 4)) (k : Fin 120) :
    wordRefl w (vertexR k) = vertexR (wordPerm w k) := by
  induction w with
  | nil => rfl
  | cons i w ih =>
    rw [wordRefl, List.map_cons, List.prod_cons, LinearIsometryEquiv.coe_mul, Function.comp_apply,
      ← wordRefl, ih, wallRefl_vertexR]
    rfl

theorem wordRefl_mem (w : List (Fin 4)) : wordRefl w ∈ Subgroup.closure (Set.range wallRefl) := by
  refine list_prod_mem fun x hx => ?_
  obtain ⟨i, -, rfl⟩ := List.mem_map.mp hx
  exact Subgroup.subset_closure ⟨i, rfl⟩

/-- The words for `L_{v₈/2, 1}`, `L_{v₃₀/2, 1}`, `L_{1, v₈/2}`, `L_{1, v₃₀/2}` and `J`, checked on
the base facet. -/
theorem wordPermCheck :
    (∀ k ∈ facet0, wordPerm wordLeft8 k = tableFin leftTable8 k) ∧
      (∀ k ∈ facet0, wordPerm wordLeft30 k = tableFin leftTable30 k) ∧
      (∀ k ∈ facet0, wordPerm wordRight8 k =
        tableFin conjTable (tableFin leftTable8 (tableFin conjTable k))) ∧
      (∀ k ∈ facet0, wordPerm wordRight30 k =
        tableFin conjTable (tableFin leftTable30 (tableFin conjTable k))) ∧
      ∀ k ∈ facet0, wordPerm wordConj k = tableFin conjTable k := by
  decide +kernel

/-- The unit quaternion `vᵢ / 2` as an element of `Sp(1)`. -/
noncomputable def icosianU (i : Fin 120) : unitary ℍ := ⟨unitVertex i, unitVertex_mem_unitary i⟩

theorem lpq_left_vertexR {a : Fin 120} {τ : Fin 120 → Fin 120}
    (h : ∀ k, unitVertex a * unitVertex k = unitVertex (τ k)) (k : Fin 120) :
    lpq (icosianU a) 1 (vertexR k) = vertexR (τ k) := by
  rw [lpq_apply, OneMemClass.coe_one, star_one, mul_one, vertexR_eq_smul, vertexR_eq_smul,
    mul_smul_comm]
  exact congrArg _ (h k)

theorem lpq_right_vertexR {a : Fin 120} {τ : Fin 120 → Fin 120}
    (h : ∀ k, unitVertex a * unitVertex k = unitVertex (τ k)) (k : Fin 120) :
    lpq 1 (icosianU a) (vertexR k) = vertexR (tableFin conjTable (τ (tableFin conjTable k))) := by
  rw [lpq_apply, OneMemClass.coe_one, one_mul, vertexR_eq_smul, vertexR_eq_smul, smul_mul_assoc]
  refine congrArg _ ?_
  change unitVertex k * star (unitVertex a) = _
  rw [← star_unitVertex, ← h, star_mul, ← star_unitVertex, star_star]

theorem conjIso_vertexR (k : Fin 120) : conjIso (vertexR k) = vertexR (tableFin conjTable k) := by
  rw [conjIso_apply, vertexR_eq_smul, vertexR_eq_smul, Quaternion.star_smul, star_unitVertex]

theorem mul_unitVertex_8 (k : Fin 120) :
    unitVertex 8 * unitVertex k = unitVertex (tableFin leftTable8 k) :=
  unitVertex_mul_of_qmul (qmul_vertex_8 k)

theorem mul_unitVertex_30 (k : Fin 120) :
    unitVertex 30 * unitVertex k = unitVertex (tableFin leftTable30 k) :=
  unitVertex_mul_of_qmul (qmul_vertex_30 k)

/-- The word tree of `BinaryIcosahedral`, with the generator named. -/
theorem word_step' (i : Fin 120) (hi : i ≠ 0) :
    ∃ g p : Fin 120, (g = 8 ∨ g = 30) ∧ unitVertex g * unitVertex p = unitVertex i ∧
      wordLength.getD p 0 < wordLength.getD i 0 := by
  have h := all_range_iff.mp wordCheck_eq_true i i.2
  have hi' : ((i : ℕ) == 0) = false := by
    simpa using fun h0 => hi (Fin.ext h0)
  rw [hi', Bool.false_or, Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at h
  obtain ⟨h1, h2⟩ := h
  rw [wordTable] at h1
  cases hb : wordLetter.getD (i : ℕ) false
  · rw [hb] at h1
    simp only [Bool.false_eq_true, ↓reduceIte] at h1
    refine ⟨8, tableFin wordParent i, Or.inl rfl, ?_, h2⟩
    have h3 := mul_unitVertex_8 (tableFin wordParent i)
    rwa [show tableFin leftTable8 (tableFin wordParent i) = i from Fin.ext h1] at h3
  · rw [hb] at h1
    simp only [↓reduceIte] at h1
    refine ⟨30, tableFin wordParent i, Or.inr rfl, ?_, h2⟩
    have h3 := mul_unitVertex_30 (tableFin wordParent i)
    rwa [show tableFin leftTable30 (tableFin wordParent i) = i from Fin.ext h1] at h3

/-- Induction over `I*` along the word tree in the generators `v₈ / 2` and `v₃₀ / 2`. -/
theorem icosian_induction {P : Fin 120 → Prop} (h0 : P 0) (h8 : P 8) (h30 : P 30)
    (hmul : ∀ g p i, unitVertex g * unitVertex p = unitVertex i → P g → P p → P i)
    (i : Fin 120) : P i := by
  suffices h : ∀ n, ∀ i : Fin 120, wordLength.getD i 0 ≤ n → P i from h _ i le_rfl
  intro n
  induction n with
  | zero =>
    intro i hi
    by_cases hz : i = 0
    · rw [hz]
      exact h0
    · obtain ⟨g, p, -, -, hlt⟩ := word_step' i hz
      omega
  | succ n ih =>
    intro i hi
    by_cases hz : i = 0
    · rw [hz]
      exact h0
    · obtain ⟨g, p, hg, hgp, hlt⟩ := word_step' i hz
      refine hmul g p i hgp ?_ (ih p (by omega))
      rcases hg with rfl | rfl
      exacts [h8, h30]

theorem icosianU_zero : icosianU 0 = 1 := Subtype.ext unitVertex_zero

theorem icosianU_mul {g p i : Fin 120} (h : unitVertex g * unitVertex p = unitVertex i) :
    icosianU i = icosianU g * icosianU p :=
  Subtype.ext h.symm

theorem lpq_left_mem_closure (i : Fin 120) :
    lpq (icosianU i) 1 ∈ Subgroup.closure (Set.range wallRefl) := by
  refine icosian_induction
    (P := fun i => lpq (icosianU i) 1 ∈ Subgroup.closure (Set.range wallRefl))
    ?_ ?_ ?_ ?_ i
  · rw [icosianU_zero, lpq_one]
    exact one_mem _
  · rw [show lpq (icosianU 8) 1 = wordRefl wordLeft8 from
      eq_of_eqOn_clique facet0_mem fun k hk => by
        rw [lpq_left_vertexR mul_unitVertex_8, wordRefl_vertexR, wordPermCheck.1 k hk]]
    exact wordRefl_mem _
  · rw [show lpq (icosianU 30) 1 = wordRefl wordLeft30 from
      eq_of_eqOn_clique facet0_mem fun k hk => by
        rw [lpq_left_vertexR mul_unitVertex_30, wordRefl_vertexR, wordPermCheck.2.1 k hk]]
    exact wordRefl_mem _
  · intro g p i hgp hg hp
    have h := lpq_mul (icosianU g) 1 (icosianU p) 1
    rw [one_mul] at h
    rw [icosianU_mul hgp, ← h]
    exact mul_mem hg hp

theorem lpq_right_mem_closure (i : Fin 120) :
    lpq 1 (icosianU i) ∈ Subgroup.closure (Set.range wallRefl) := by
  refine icosian_induction
    (P := fun i => lpq 1 (icosianU i) ∈ Subgroup.closure (Set.range wallRefl))
    ?_ ?_ ?_ ?_ i
  · rw [icosianU_zero, lpq_one]
    exact one_mem _
  · rw [show lpq 1 (icosianU 8) = wordRefl wordRight8 from
      eq_of_eqOn_clique facet0_mem fun k hk => by
        rw [lpq_right_vertexR mul_unitVertex_8, wordRefl_vertexR, wordPermCheck.2.2.1 k hk]]
    exact wordRefl_mem _
  · rw [show lpq 1 (icosianU 30) = wordRefl wordRight30 from
      eq_of_eqOn_clique facet0_mem fun k hk => by
        rw [lpq_right_vertexR mul_unitVertex_30, wordRefl_vertexR, wordPermCheck.2.2.2.1 k hk]]
    exact wordRefl_mem _
  · intro g p i hgp hg hp
    have h := lpq_mul 1 (icosianU g) 1 (icosianU p)
    rw [one_mul] at h
    rw [icosianU_mul hgp, ← h]
    exact mul_mem hg hp

theorem conjIso_mem_closure : conjIso ∈ Subgroup.closure (Set.range wallRefl) := by
  rw [show conjIso = wordRefl wordConj from
    eq_of_eqOn_clique facet0_mem fun k hk => by
      rw [conjIso_vertexR, wordRefl_vertexR, wordPermCheck.2.2.2.2 k hk]]
  exact wordRefl_mem _

/-- **Generation.** The four wall reflections generate `K`. -/
theorem closure_wallRefl : Subgroup.closure (Set.range wallRefl) = symmetryGroup := by
  refine le_antisymm ((Subgroup.closure_le _).mpr ?_) ?_
  · rintro _ ⟨i, rfl⟩
    exact wallRefl_mem i
  · intro A hA
    obtain ⟨p, hp, q, hq, hA'⟩ := mem_symmetryGroup_iff_lpq.mp hA
    obtain ⟨a, ha⟩ : (p : ℍ) ∈ Set.range unitVertex := hp
    obtain ⟨b, hb⟩ : (q : ℍ) ∈ Set.range unitVertex := hq
    have hpq : lpq p q ∈ Subgroup.closure (Set.range wallRefl) := by
      have h := lpq_mul p 1 1 q
      rw [mul_one, one_mul] at h
      rw [← h, show p = icosianU a from Subtype.ext ha.symm,
        show q = icosianU b from Subtype.ext hb.symm]
      exact mul_mem (lpq_left_mem_closure a) (lpq_right_mem_closure b)
    rcases hA' with rfl | rfl
    · exact hpq
    · exact mul_mem hpq conjIso_mem_closure

/-! ### The presentation -/

/-- The wall reflection `sᵢ` as an element of `K`. -/
noncomputable def wallSym (i : Fin 4) : symmetryGroup := ⟨wallRefl i, wallRefl_mem i⟩

theorem isLiftable_wallSym : CoxeterMatrix.H₄.IsLiftable wallSym := fun i j =>
  Subtype.ext (by
    rw [Subgroup.coe_pow, Subgroup.coe_mul]
    exact wallRefl_mul_pow i j)

/-- The homomorphism from the Coxeter group of type `H₄` to `K` with `sᵢ ↦ wallRefl i`. -/
noncomputable def coxeterHom : CoxeterMatrix.H₄.Group →* symmetryGroup :=
  CoxeterMatrix.H₄.toCoxeterSystem.lift ⟨wallSym, isLiftable_wallSym⟩

theorem coxeterHom_simple (i : Fin 4) : coxeterHom (CoxeterMatrix.H₄.simple i) = wallSym i :=
  CoxeterMatrix.H₄.toCoxeterSystem.lift_apply_simple isLiftable_wallSym i

theorem exists_coxeterHom_eq {A : ℍ ≃ₗᵢ[ℝ] ℍ} (hA : A ∈ Subgroup.closure (Set.range wallRefl)) :
    ∃ w, (coxeterHom w : ℍ ≃ₗᵢ[ℝ] ℍ) = A := by
  induction hA using Subgroup.closure_induction with
  | mem x hx =>
    obtain ⟨i, rfl⟩ := hx
    exact ⟨_, congrArg Subtype.val (coxeterHom_simple i)⟩
  | one => exact ⟨1, by rw [map_one]; rfl⟩
  | mul x y _ _ hx hy =>
    obtain ⟨w₁, h₁⟩ := hx
    obtain ⟨w₂, h₂⟩ := hy
    exact ⟨w₁ * w₂, by rw [map_mul, Subgroup.coe_mul, h₁, h₂]⟩
  | inv x _ hx =>
    obtain ⟨w, h⟩ := hx
    exact ⟨w⁻¹, by rw [map_inv, Subgroup.coe_inv, h]⟩

theorem coxeterHom_surjective : Function.Surjective coxeterHom := by
  rintro ⟨A, hA⟩
  obtain ⟨w, hw⟩ := exists_coxeterHom_eq (closure_wallRefl.ge hA)
  exact ⟨w, Subtype.ext hw⟩

/-- **E5** (first part), as a hypothesis: the Coxeter group with the chain presentation
`(3, 3, 5)`, Mathlib's `CoxeterMatrix.H₄.Group`, is `W(H₄)`, in the standard quantitative form
`|W(H₄)| = 14400`. The second part of E5 (closed galleries in the dual cell complex of `S³` are
generated by backtracking and the rank-two polygon boundaries) is not used: here the completeness
of the presentation follows from the two orders. -/
structure CoxeterSources : Prop where
  /-- `|W(H₄)| = 14400`. -/
  card_group : Nat.card CoxeterMatrix.H₄.Group = 14400

theorem coxeterHom_bijective (h : CoxeterSources) : Function.Bijective coxeterHom := by
  have : Finite CoxeterMatrix.H₄.Group :=
    Nat.finite_of_card_ne_zero (by rw [h.card_group]; norm_num)
  exact coxeterHom_surjective.bijective_of_nat_card_le (by rw [h.card_group, card_symmetryGroup])

/-- **Lemma 4 (the Coxeter presentation).** `CoxeterMatrix.H₄` is the chain presentation
`(3, 3, 5)`. The wall reflections satisfy its relations `(sᵢ sⱼ)^{m_ij} = 1`, and `m_ij` is the
exact order of `sᵢ sⱼ` (so `sᵢ² = 1`, `(s₀s₁)³ = (s₁s₂)³ = (s₂s₃)⁵ = 1`, and `(sᵢsⱼ)² = 1` when
`|i - j| > 1`). They generate `K`, so the Coxeter group of type `H₄` maps onto `K` with
`sᵢ ↦ wallRefl i`. Under E5 (`CoxeterSources`), this map is an isomorphism `W(H₄) ≅ K`. -/
theorem coxeterPresentation :
    ((∀ i, CoxeterMatrix.H₄ i i = 1) ∧ CoxeterMatrix.H₄ 0 1 = 3 ∧ CoxeterMatrix.H₄ 1 2 = 3 ∧
        CoxeterMatrix.H₄ 2 3 = 5 ∧ ∀ i j : Fin 4, (i : ℕ) + 1 < j → CoxeterMatrix.H₄ i j = 2) ∧
      (∀ i j, (wallRefl i * wallRefl j) ^ CoxeterMatrix.H₄ i j = 1) ∧
      (∀ i j, orderOf (wallRefl i * wallRefl j) = CoxeterMatrix.H₄ i j) ∧
      Subgroup.closure (Set.range wallRefl) = symmetryGroup ∧
      (∃ φ : CoxeterMatrix.H₄.Group →* symmetryGroup, Function.Surjective φ ∧
        ∀ i, (φ (CoxeterMatrix.H₄.simple i) : ℍ ≃ₗᵢ[ℝ] ℍ) = wallRefl i) ∧
      (CoxeterSources → ∃ e : CoxeterMatrix.H₄.Group ≃* symmetryGroup,
        ∀ i, (e (CoxeterMatrix.H₄.simple i) : ℍ ≃ₗᵢ[ℝ] ℍ) = wallRefl i) :=
  ⟨H₄_chain, wallRefl_mul_pow, orderOf_wallRefl_mul, closure_wallRefl,
    ⟨coxeterHom, coxeterHom_surjective, fun i => congrArg Subtype.val (coxeterHom_simple i)⟩,
    fun h => ⟨MulEquiv.ofBijective coxeterHom (coxeterHom_bijective h),
      fun i => congrArg Subtype.val (coxeterHom_simple i)⟩⟩

end ArchonFull600Theory
