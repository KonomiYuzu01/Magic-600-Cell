import ArchonFull600Theory.EdgeGraph
import Mathlib.Analysis.Convex.Combination
import Mathlib.Analysis.InnerProductSpace.GramMatrix
import Mathlib.LinearAlgebra.AffineSpace.FiniteDimensional

set_option linter.style.header false in
/-!
# The 600 facets

The finite checker's supporting-facet test and the completeness of its facet list. For each
four-clique of the edge graph, the sum `N` of its vertices satisfies `N · vᵢ = 4 + 6 φ` at its
four vertices and a strict inequality at the other 116, so its tetrahedron is the face of `P`
cut out by `N`; the Gram matrix of its vertices is positive definite, so the face has dimension 3.
Under the standard polytope facts E4 (a ridge lies in exactly two facets, and the facet adjacency
graph is connected), these 600 tetrahedra are all the facets of `P`.
-/

namespace ArchonFull600Theory

open scoped goldenRatio Quaternion RealInnerProductSpace
open Finset Vec4

/-! ### Faces of the convex hull of a finite set -/

section Face

variable {E : Type*} [AddCommGroup E] [Module ℝ E]

/-- A linear functional bounded by `M` on a finite set `t` is bounded by `M` on the convex hull
of `t`, and the points of the hull where it equals `M` are the hull of the points of `t` where it
equals `M`. -/
theorem face_convexHull (t : Finset E) (l : E →ₗ[ℝ] ℝ) (M : ℝ) (h : ∀ y ∈ t, l y ≤ M) :
    (∀ x ∈ convexHull ℝ (t : Set E), l x ≤ M) ∧
      {x ∈ convexHull ℝ (t : Set E) | l x = M} =
        convexHull ℝ ((t.filter fun y => l y = M : Finset E) : Set E) := by
  have hle : convexHull ℝ (t : Set E) ⊆ {x | l x ≤ M} :=
    convexHull_min (fun y hy => h y hy) (convex_halfSpace_le l.isLinear M)
  refine ⟨fun x hx => hle hx, ?_⟩
  ext x
  constructor
  · rintro ⟨hx, hlx⟩
    obtain ⟨w, hw0, hw1, hwx⟩ := Finset.mem_convexHull.mp hx
    rw [Finset.centerMass_eq_of_sum_1 t id hw1] at hwx
    have hlx' : ∑ y ∈ t, w y * l y = M := by
      rw [← hlx, ← hwx, map_sum]
      simp only [id, map_smul, smul_eq_mul]
    have hsum : ∑ y ∈ t, w y * (M - l y) = 0 := by
      have e : ∀ y ∈ t, w y * (M - l y) = w y * M - w y * l y := fun y _ => by ring
      rw [sum_congr rfl e, sum_sub_distrib, ← sum_mul, hw1, hlx']
      ring
    have hzero := (sum_eq_zero_iff_of_nonneg fun y hy =>
      mul_nonneg (hw0 y hy) (sub_nonneg.mpr (h y hy))).mp hsum
    have hw : ∀ y ∈ t, w y ≠ 0 → l y = M := by
      intro y hy hwy
      rcases mul_eq_zero.mp (hzero y hy) with h' | h'
      · exact absurd h' hwy
      · linarith
    have hsum1 : ∑ y ∈ t.filter (fun y => l y = M), w y = 1 := by
      rw [sum_filter_of_ne fun y hy hwy => hw y hy hwy, hw1]
    refine Finset.mem_convexHull.mpr ⟨w, fun y hy => hw0 y (mem_filter.mp hy).1, hsum1, ?_⟩
    rw [Finset.centerMass_eq_of_sum_1 _ id hsum1,
      sum_filter_of_ne fun y hy hne => hw y hy fun h0 => hne (by rw [h0, zero_smul])]
    exact hwx
  · intro hx
    refine ⟨convexHull_mono (coe_subset.mpr (filter_subset _ _)) hx, ?_⟩
    have hsub : convexHull ℝ ((t.filter fun y => l y = M : Finset E) : Set E) ⊆ {x | l x = M} :=
      convexHull_min (fun y hy => (mem_filter.mp hy).2) (convex_hyperplane l.isLinear M)
    exact hsub hx

/-- The face lemma for an indexed finite family `f` on `t`. -/
theorem face_convexHull_image {ι : Type*} (t : Finset ι) (f : ι → E) (l : E →ₗ[ℝ] ℝ) (M : ℝ)
    (h : ∀ i ∈ t, l (f i) ≤ M) :
    (∀ x ∈ convexHull ℝ (f '' t), l x ≤ M) ∧
      {x ∈ convexHull ℝ (f '' t) | l x = M} =
        convexHull ℝ (f '' (t.filter fun i => l (f i) = M : Finset ι)) := by
  classical
  have key := face_convexHull (t.image f) l M fun y hy => by
    obtain ⟨i, hi, rfl⟩ := mem_image.mp hy
    exact h i hi
  rw [coe_image] at key
  refine ⟨key.1, ?_⟩
  rw [key.2]
  congr 1
  ext y
  simp only [mem_coe, mem_filter, mem_image, Set.mem_image]
  constructor
  · rintro ⟨⟨i, hi, rfl⟩, hy⟩
    exact ⟨i, ⟨hi, hy⟩, rfl⟩
  · rintro ⟨i, ⟨hi, hy⟩, rfl⟩
    exact ⟨⟨i, hi, rfl⟩, hy⟩

end Face

/-! ### Uniform Gram matrices -/

section Gram

variable {F : Type*} [NormedAddCommGroup F] [InnerProductSpace ℝ F]
variable {ι : Type*} [DecidableEq ι] (x : ι → F)

/-- The inner products of the points of `s`: `r` on the diagonal and `m` off it. -/
def UniformGram (s : Finset ι) (r m : ℝ) : Prop :=
  ∀ i ∈ s, ∀ j ∈ s, ⟪x i, x j⟫ = if i = j then r else m

theorem inner_sum_mem {s : Finset ι} {r m : ℝ} (h : UniformGram x s r m) {k : ι} (hk : k ∈ s) :
    ⟪∑ i ∈ s, x i, x k⟫ = r + (#s - 1) * m := by
  rw [sum_inner]
  have : ∀ i ∈ s, ⟪x i, x k⟫ = if i = k then r else m := fun i hi => h i hi k hk
  rw [sum_congr rfl this, ← sum_erase_add _ _ hk, ite_eq_left rfl,
    sum_congr rfl (fun i hi => ite_eq_right (ne_of_mem_erase hi)), sum_const, card_erase_of_mem hk,
    nsmul_eq_mul, Nat.cast_sub (one_le_card.mpr ⟨k, hk⟩), Nat.cast_one]
  ring

theorem norm_sq_sum {s : Finset ι} {r m : ℝ} (h : UniformGram x s r m) :
    ‖∑ i ∈ s, x i‖ ^ 2 = #s * (r + (#s - 1) * m) := by
  rw [← real_inner_self_eq_norm_sq]
  conv_lhs => rw [inner_sum]
  rw [sum_congr rfl (fun k hk => inner_sum_mem x h hk), sum_const, nsmul_eq_mul]

omit [DecidableEq ι] in
theorem inner_sum_nonmem {s : Finset ι} {m : ℝ} {k : ι} (h : ∀ i ∈ s, ⟪x i, x k⟫ ≤ m) :
    ⟪∑ i ∈ s, x i, x k⟫ ≤ #s * m := by
  rw [sum_inner]
  calc ∑ i ∈ s, ⟪x i, x k⟫ ≤ ∑ _i ∈ s, m := sum_le_sum h
    _ = #s * m := by rw [sum_const, nsmul_eq_mul]

theorem norm_sq_combination {s : Finset ι} {r m : ℝ} (h : UniformGram x s r m) (c : ι → ℝ) :
    ‖∑ i ∈ s, c i • x i‖ ^ 2 = (r - m) * ∑ i ∈ s, c i ^ 2 + m * (∑ i ∈ s, c i) ^ 2 := by
  rw [← real_inner_self_eq_norm_sq, sum_inner]
  have hrow : ∀ i ∈ s, ⟪c i • x i, ∑ j ∈ s, c j • x j⟫ =
      c i * ((r - m) * c i + m * ∑ j ∈ s, c j) := by
    intro i hi
    rw [inner_sum]
    have : ∀ j ∈ s, ⟪c i • x i, c j • x j⟫ = c i * (c j * (if i = j then r else m)) := by
      intro j hj
      rw [real_inner_smul_left, real_inner_smul_right, h i hi j hj]
    rw [sum_congr rfl this, ← mul_sum]
    congr 1
    rw [← sum_erase_add _ _ hi, ite_eq_left rfl,
      sum_congr rfl (fun j hj => by rw [ite_eq_right (ne_of_mem_erase hj).symm]), ← sum_mul,
      ← sum_erase_add _ _ hi]
    ring
  rw [sum_congr rfl hrow]
  calc ∑ i ∈ s, c i * ((r - m) * c i + m * ∑ j ∈ s, c j)
      = ∑ i ∈ s, ((r - m) * c i ^ 2 + (m * ∑ j ∈ s, c j) * c i) :=
        sum_congr rfl fun i _ => by ring
    _ = (r - m) * ∑ i ∈ s, c i ^ 2 + (m * ∑ j ∈ s, c j) * ∑ i ∈ s, c i := by
        rw [sum_add_distrib, ← mul_sum, ← mul_sum]
    _ = _ := by ring

/-- With `r > m > 0`, a vanishing combination of points with a uniform Gram matrix is trivial. -/
theorem combination_eq_zero {s : Finset ι} {r m : ℝ} (h : UniformGram x s r m) (hm : 0 ≤ m)
    (hrm : m < r) (c : ι → ℝ) (hc : ∑ i ∈ s, c i • x i = 0) : ∀ i ∈ s, c i = 0 := by
  have hn := norm_sq_combination x h c
  rw [hc, norm_zero] at hn
  have hsq : ∑ i ∈ s, c i ^ 2 = 0 := by
    have h1 : 0 ≤ ∑ i ∈ s, c i ^ 2 := sum_nonneg fun i _ => sq_nonneg (c i)
    have h2 : 0 ≤ m * (∑ i ∈ s, c i) ^ 2 := mul_nonneg hm (sq_nonneg _)
    have h3 : (r - m) * ∑ i ∈ s, c i ^ 2 = 0 := by nlinarith
    rcases mul_eq_zero.mp h3 with h4 | h4
    · linarith
    · exact h4
  intro i hi
  exact pow_eq_zero_iff two_ne_zero |>.mp
    ((sum_eq_zero_iff_of_nonneg fun j _ => sq_nonneg (c j)).mp hsq i hi)

end Gram

/-! ### The supporting-facet test -/

/-- The convex hull `P` of `V`. -/
noncomputable def polytope : Set ℍ := convexHull ℝ vertexSet

theorem vertexSet_eq_image :
    vertexSet = vertexR '' ((univ : Finset (Fin 120)) : Set (Fin 120)) := by
  rw [coe_univ, Set.image_univ, range_vertexR]

/-- The facet normal of a four-clique: the sum of its vertices. -/
noncomputable def facetNormal (s : Finset (Fin 120)) : ℍ := ∑ i ∈ s, vertexR i

/-- The convex hull of the vertices of a clique: for a four-clique, its tetrahedron. -/
noncomputable def cliqueTetra (s : Finset (Fin 120)) : Set ℍ := convexHull ℝ (vertexR '' s)

theorem uniformGram_clique {s : Finset (Fin 120)} (hs : edgeGraph.IsClique (s : Set (Fin 120))) :
    UniformGram vertexR s 4 (2 * φ) := by
  intro i hi j hj
  by_cases hij : i = j
  · subst hij
    rw [ite_eq_left rfl, inner_vertexR_self]
  · rw [ite_eq_right hij]
    exact (edgeGraph_adj_iff i j).mp (hs hi hj hij)

theorem four_sub_two_mul_goldenRatio_pos : 0 < 4 - 2 * φ := by
  linarith [Real.goldenRatio_lt_two]

section Clique

variable {s : Finset (Fin 120)} (hs : s ∈ edgeGraph.cliqueFinset 4)
include hs

theorem inner_facetNormal_mem {k : Fin 120} (hk : k ∈ s) :
    ⟪facetNormal s, vertexR k⟫ = 4 + 6 * φ := by
  have hs' := SimpleGraph.mem_cliqueFinset_iff.mp hs
  rw [facetNormal, inner_sum_mem vertexR (uniformGram_clique hs'.1) hk, hs'.2]
  push_cast
  ring

theorem inner_facetNormal_lt {k : Fin 120} (hk : k ∉ s) :
    ⟪facetNormal s, vertexR k⟫ < 4 + 6 * φ := by
  have hs' := SimpleGraph.mem_cliqueFinset_iff.mp hs
  have h1 := inner_sum_nonmem vertexR (s := s) (m := 2 * φ) (k := k)
    fun i hi => inner_vertexR_le i k fun h => hk (h ▸ hi)
  rw [hs'.2] at h1
  push_cast at h1
  rw [facetNormal]
  linarith [Real.goldenRatio_lt_two]

theorem norm_sq_facetNormal : ‖facetNormal s‖ ^ 2 = 16 + 24 * φ := by
  have hs' := SimpleGraph.mem_cliqueFinset_iff.mp hs
  rw [facetNormal, norm_sq_sum vertexR (uniformGram_clique hs'.1), hs'.2]
  push_cast
  ring

theorem facet_filter :
    univ.filter (fun k => innerₛₗ ℝ (facetNormal s) (vertexR k) = 4 + 6 * φ) = s := by
  ext k
  rw [mem_filter_univ, innerₛₗ_apply_apply]
  constructor
  · intro hk
    by_contra hks
    exact ne_of_lt (inner_facetNormal_lt hs hks) hk
  · exact inner_facetNormal_mem hs

/-- **Supporting-facet test (FC).** The facet normal `N` of a four-clique satisfies
`N · vᵢ = 4 + 6 φ` at its vertices, `‖N‖² = 16 + 24 φ`, and `N · v < 4 + 6 φ` at every other
vertex; so `N · x ≤ 4 + 6 φ` on `P`, with equality exactly on the tetrahedron of the clique. -/
theorem supportingFacet :
    (∀ k ∈ s, ⟪facetNormal s, vertexR k⟫ = 4 + 6 * φ) ∧ ‖facetNormal s‖ ^ 2 = 16 + 24 * φ ∧
      (∀ k ∉ s, ⟪facetNormal s, vertexR k⟫ < 4 + 6 * φ) ∧
      (∀ x ∈ polytope, ⟪facetNormal s, x⟫ ≤ 4 + 6 * φ) ∧
      {x ∈ polytope | ⟪facetNormal s, x⟫ = 4 + 6 * φ} = cliqueTetra s := by
  have hb : ∀ k ∈ univ, innerₛₗ ℝ (facetNormal s) (vertexR k) ≤ 4 + 6 * φ := by
    intro k _
    rw [innerₛₗ_apply_apply]
    by_cases hk : k ∈ s
    · exact (inner_facetNormal_mem hs hk).le
    · exact (inner_facetNormal_lt hs hk).le
  obtain ⟨hle, heq⟩ := face_convexHull_image univ vertexR (innerₛₗ ℝ (facetNormal s))
    (4 + 6 * φ) hb
  rw [facet_filter hs] at heq
  simp only [innerₛₗ_apply_apply] at hle heq
  refine ⟨fun k hk => inner_facetNormal_mem hs hk, norm_sq_facetNormal hs,
    fun k hk => inner_facetNormal_lt hs hk, ?_, ?_⟩
  · rw [polytope, vertexSet_eq_image]
    exact hle
  · rw [polytope, vertexSet_eq_image, cliqueTetra]
    exact heq

/-- The vertices of a four-clique, as a family indexed by the clique. -/
theorem uniformGram_subtype :
    UniformGram (vertexR ∘ ((↑) : s → Fin 120)) univ 4 (2 * φ) := by
  have hs' := (SimpleGraph.mem_cliqueFinset_iff.mp hs).1
  intro a _ b _
  by_cases hab : a = b
  · subst hab
    rw [ite_eq_left rfl]
    exact inner_vertexR_self a
  · rw [ite_eq_right hab]
    exact (edgeGraph_adj_iff a b).mp (hs' a.2 b.2 fun h => hab (Subtype.ext h))

/-- **Gram matrix.** The Gram matrix of the four vertices of a clique has diagonal entries 4 and
off-diagonal entries `2 φ`, and it is positive definite. -/
theorem facetGram :
    UniformGram vertexR s 4 (2 * φ) ∧
      (Matrix.gram ℝ (vertexR ∘ ((↑) : s → Fin 120))).PosDef := by
  refine ⟨uniformGram_clique (SimpleGraph.mem_cliqueFinset_iff.mp hs).1,
    Matrix.posDef_gram_iff_linearIndependent.mpr ?_⟩
  rw [Fintype.linearIndependent_iff]
  intro g hg i
  exact combination_eq_zero _ (uniformGram_subtype hs) (by positivity)
    (by linarith [Real.goldenRatio_lt_two]) g hg i (mem_univ i)

theorem affineIndependent_clique : AffineIndependent ℝ (vertexR ∘ ((↑) : s → Fin 120)) := by
  intro u w hw0 hvs i hi
  rw [Finset.weightedVSub_eq_linear_combination u hw0] at hvs
  have hu : UniformGram (vertexR ∘ ((↑) : s → Fin 120)) u 4 (2 * φ) := fun a _ b _ =>
    uniformGram_subtype hs a (mem_univ a) b (mem_univ b)
  exact combination_eq_zero _ hu (by positivity) (by linarith [Real.goldenRatio_lt_two]) w hvs i hi

omit hs in
theorem range_vertexR_subtype :
    Set.range (vertexR ∘ ((↑) : s → Fin 120)) = vertexR '' (s : Set (Fin 120)) := by
  ext q
  simp

/-- The tetrahedron of a four-clique spans an affine space of dimension 3. -/
theorem finrank_cliqueTetra : Module.finrank ℝ (vectorSpan ℝ (cliqueTetra s)) = 3 := by
  have hcard : Fintype.card s = 3 + 1 := by
    rw [Fintype.card_coe, (SimpleGraph.mem_cliqueFinset_iff.mp hs).2]
  have h := (affineIndependent_clique hs).finrank_vectorSpan hcard
  rw [range_vertexR_subtype] at h
  rw [cliqueTetra, ← direction_affineSpan, affineSpan_convexHull, direction_affineSpan]
  exact h

theorem vertexR_mem_cliqueTetra_iff (k : Fin 120) : vertexR k ∈ cliqueTetra s ↔ k ∈ s := by
  constructor
  · intro hk
    rw [← (supportingFacet hs).2.2.2.2] at hk
    by_contra hks
    exact absurd hk.2 (ne_of_lt (inner_facetNormal_lt hs hks))
  · intro hks
    exact subset_convexHull ℝ _ (Set.mem_image_of_mem vertexR hks)

omit hs in
theorem cliqueTetra_subset_polytope : cliqueTetra s ⊆ polytope := by
  refine convexHull_mono ?_
  rw [← range_vertexR]
  exact Set.image_subset_range _ _

end Clique

/-- **Cliques span exposed faces.** For an `n`-clique `s` with vertex sum `N`, the functional
`N · x` is at most `4 + 2 (n - 1) φ` on `P`, with equality exactly on the convex hull of the
clique. So the edges, triangles and four-cliques of the edge graph span exposed faces of `P`;
for an edge `{v, w}` this is the source's statement that `v + w` exposes the segment. -/
theorem cliqueFace {n : ℕ} {s : Finset (Fin 120)} (hs : s ∈ edgeGraph.cliqueFinset n) :
    (∀ x ∈ polytope, ⟪facetNormal s, x⟫ ≤ 4 + 2 * (n - 1) * φ) ∧
      {x ∈ polytope | ⟪facetNormal s, x⟫ = 4 + 2 * (n - 1) * φ} = cliqueTetra s := by
  have hs' := SimpleGraph.mem_cliqueFinset_iff.mp hs
  have hn : #s = n := hs'.2
  have hmem : ∀ k ∈ s, ⟪facetNormal s, vertexR k⟫ = 4 + 2 * (n - 1) * φ := by
    intro k hk
    rw [facetNormal, inner_sum_mem vertexR (uniformGram_clique hs'.1) hk, hn]
    ring
  have hlt : ∀ k ∉ s, ⟪facetNormal s, vertexR k⟫ < 4 + 2 * (n - 1) * φ := by
    intro k hk
    have h1 := inner_sum_nonmem vertexR (s := s) (m := 2 * φ) (k := k)
      fun i hi => inner_vertexR_le i k fun h => hk (h ▸ hi)
    rw [hn] at h1
    rw [facetNormal]
    nlinarith [Real.goldenRatio_lt_two]
  have hb : ∀ k ∈ univ, innerₛₗ ℝ (facetNormal s) (vertexR k) ≤ 4 + 2 * (n - 1) * φ := by
    intro k _
    rw [innerₛₗ_apply_apply]
    by_cases hk : k ∈ s
    · exact (hmem k hk).le
    · exact (hlt k hk).le
  have hfilter :
      univ.filter (fun k => innerₛₗ ℝ (facetNormal s) (vertexR k) = 4 + 2 * (n - 1) * φ) = s := by
    ext k
    rw [mem_filter_univ, innerₛₗ_apply_apply]
    constructor
    · intro hk
      by_contra hks
      exact ne_of_lt (hlt k hks) hk
    · exact hmem k
  obtain ⟨hle, heq⟩ := face_convexHull_image univ vertexR (innerₛₗ ℝ (facetNormal s))
    (4 + 2 * (n - 1) * φ) hb
  rw [hfilter] at heq
  simp only [innerₛₗ_apply_apply] at hle heq
  refine ⟨?_, ?_⟩
  · rw [polytope, vertexSet_eq_image]
    exact hle
  · rw [polytope, vertexSet_eq_image, cliqueTetra]
    exact heq

theorem cliqueTetra_injOn :
    Set.InjOn cliqueTetra (edgeGraph.cliqueFinset 4 : Set (Finset (Fin 120))) := by
  intro s hs s' hs' h
  ext k
  rw [← vertexR_mem_cliqueTetra_iff hs, ← vertexR_mem_cliqueTetra_iff hs', h]

/-! ### Completeness of the facet list -/

/-- A facet of a set `P ⊆ ℍ`: the part of `P` where a linear functional bounded by `M` on `P`
attains `M`, of affine dimension 3. This is the usual notion for a convex body with nonempty
interior in `ℍ`, such as the polytope `P`. -/
def IsFacet (P F : Set ℍ) : Prop :=
  ∃ l : ℍ →ₗ[ℝ] ℝ, ∃ M : ℝ, (∀ x ∈ P, l x ≤ M) ∧ F = {x ∈ P | l x = M} ∧
    Module.finrank ℝ (vectorSpan ℝ F) = 3

/-- Two facets are adjacent when they are distinct and meet in a set of affine dimension 2, a
ridge. -/
def FacetsAdjacent (F G : Set ℍ) : Prop :=
  F ≠ G ∧ Module.finrank ℝ (vectorSpan ℝ (F ∩ G)) = 2

/-- The standard polytope facts E4 that the completeness argument uses for `P`: a ridge lies in
exactly two facets, and the facet adjacency graph is connected. -/
structure FacetFacts (P : Set ℍ) : Prop where
  /-- Every ridge lies in no facet other than the two that meet in it. -/
  ridge_two : ∀ F G H, IsFacet P F → IsFacet P G → IsFacet P H → FacetsAdjacent F G →
    F ∩ G ⊆ H → H = F ∨ H = G
  /-- The facet adjacency graph is connected. -/
  connected : ∀ F G, IsFacet P F → IsFacet P G →
    Relation.ReflTransGen (fun A B => IsFacet P A ∧ IsFacet P B ∧ FacetsAdjacent A B) F G

/-- The tetrahedron of every four-clique is a facet of `P`. -/
theorem isFacet_cliqueTetra {s : Finset (Fin 120)} (hs : s ∈ edgeGraph.cliqueFinset 4) :
    IsFacet polytope (cliqueTetra s) := by
  refine ⟨innerₛₗ ℝ (facetNormal s), 4 + 6 * φ, fun x hx => ?_, ?_, finrank_cliqueTetra hs⟩
  · rw [innerₛₗ_apply_apply]
    exact (supportingFacet hs).2.2.2.1 x hx
  · rw [← (supportingFacet hs).2.2.2.2]
    simp only [innerₛₗ_apply_apply]

theorem finrank_vectorSpan_image_le (r : Finset (Fin 120)) :
    Module.finrank ℝ (vectorSpan ℝ (vertexR '' (r : Set (Fin 120)))) ≤ #r - 1 := by
  classical
  rcases r.eq_empty_or_nonempty with rfl | hr
  · simp
  · have h := finrank_vectorSpan_image_finset_le (k := ℝ) vertexR r (n := #r - 1)
      (by have := hr.card_pos; omega)
    rwa [coe_image] at h

/-- A facet adjacent to the tetrahedron of a four-clique is the tetrahedron of another
four-clique: the ridge is the triangle of a three-clique, which lies in exactly two
four-cliques. -/
theorem adjacent_isCliqueTetra (h : FacetFacts polytope) {s : Finset (Fin 120)}
    (hs : s ∈ edgeGraph.cliqueFinset 4) {G : Set ℍ} (hG : IsFacet polytope G)
    (hadj : FacetsAdjacent (cliqueTetra s) G) :
    ∃ s' ∈ edgeGraph.cliqueFinset 4, G = cliqueTetra s' := by
  obtain ⟨l, M, hl, hGeq, -⟩ := id hG
  have hs' := SimpleGraph.mem_cliqueFinset_iff.mp hs
  have hs4 : #s = 4 := hs'.2
  have hb : ∀ k ∈ s, l (vertexR k) ≤ M := fun k hk =>
    hl _ (cliqueTetra_subset_polytope ((vertexR_mem_cliqueTetra_iff hs k).mpr hk))
  obtain ⟨r, hr_def⟩ : ∃ r, r = s.filter fun k => l (vertexR k) = M := ⟨_, rfl⟩
  have hridge : cliqueTetra s ∩ G = cliqueTetra r := by
    have hface := (face_convexHull_image s vertexR l M hb).2
    rw [← hr_def] at hface
    rw [cliqueTetra, cliqueTetra, ← hface, hGeq]
    ext x
    constructor
    · rintro ⟨hx, -, hlx⟩
      exact ⟨hx, hlx⟩
    · rintro ⟨hx, hlx⟩
      exact ⟨hx, cliqueTetra_subset_polytope hx, hlx⟩
  have hdim : Module.finrank ℝ (vectorSpan ℝ (vertexR '' (r : Set (Fin 120)))) = 2 := by
    have := hadj.2
    rwa [hridge, cliqueTetra, ← direction_affineSpan, affineSpan_convexHull,
      direction_affineSpan] at this
  have hrs : r ⊆ s := hr_def ▸ filter_subset _ _
  have hr3 : #r = 3 := by
    have hle := finrank_vectorSpan_image_le r
    have hr4 : #r ≤ 4 := hs4 ▸ card_le_card hrs
    rcases (by omega : #r = 3 ∨ #r = 4) with h3 | h4
    · exact h3
    · exfalso
      have hrs' : r = s := eq_of_subset_of_card_le hrs (by omega)
      have := hadj.2
      rw [hridge, hrs', finrank_cliqueTetra hs] at this
      omega
  have hr : r ∈ edgeGraph.cliqueFinset 3 :=
    SimpleGraph.mem_cliqueFinset_iff.mpr ⟨hs'.1.subset (coe_subset.mpr hrs), hr3⟩
  have htwo := incidenceCounts.2.2 r hr
  obtain ⟨s', hs'mem, hne⟩ := exists_mem_ne (by omega :
    1 < #((edgeGraph.cliqueFinset 4).filter (r ⊆ ·))) s
  obtain ⟨hs'4, hrs'⟩ := mem_filter.mp hs'mem
  have hsub : cliqueTetra s ∩ G ⊆ cliqueTetra s' := by
    rw [hridge]
    exact convexHull_mono (Set.image_mono (coe_subset.mpr hrs'))
  rcases h.ridge_two _ _ _ (isFacet_cliqueTetra hs) hG (isFacet_cliqueTetra hs'4) hadj hsub with
    h1 | h1
  · exact absurd (cliqueTetra_injOn hs'4 hs h1) hne
  · exact ⟨s', hs'4, h1.symm⟩

/-- **Completeness of the facet list (FC).** Under the polytope facts E4, the facets of `P` are
exactly the tetrahedra of the four-cliques of the edge graph, and there are 600 of them. -/
theorem facetCompleteness (h : FacetFacts polytope) :
    {F | IsFacet polytope F} =
        cliqueTetra '' (edgeGraph.cliqueFinset 4 : Set (Finset (Fin 120))) ∧
      {F | IsFacet polytope F}.ncard = 600 := by
  have hlist : {F | IsFacet polytope F} =
      cliqueTetra '' (edgeGraph.cliqueFinset 4 : Set (Finset (Fin 120))) := by
    ext F
    constructor
    · intro hF
      obtain ⟨s₀, hs₀⟩ : (edgeGraph.cliqueFinset 4).Nonempty := by
        rw [← card_pos, faceCounts.2.2.2.2]
        norm_num
      have hpath := h.connected (cliqueTetra s₀) F (isFacet_cliqueTetra hs₀) hF
      clear hF
      induction hpath with
      | refl => exact ⟨s₀, hs₀, rfl⟩
      | tail _ hbc ih =>
        obtain ⟨s, hs, rfl⟩ := ih
        obtain ⟨-, hc, hadj⟩ := hbc
        obtain ⟨s', hs', rfl⟩ := adjacent_isCliqueTetra h hs hc hadj
        exact ⟨s', hs', rfl⟩
    · rintro ⟨s, hs, rfl⟩
      exact isFacet_cliqueTetra hs
  refine ⟨hlist, ?_⟩
  rw [hlist, cliqueTetra_injOn.ncard_image, Set.ncard_coe_finset,
    faceCounts.2.2.2.2]

end ArchonFull600Theory
