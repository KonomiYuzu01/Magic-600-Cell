import ArchonFull600Theory.Topology

set_option linter.style.header false in
/-!
# Caps, cut chambers and complete cap signatures

Source: `research/Full_600cell_Puzzle_Theory.tex`, the retained-model definitions (lines 736 to
769) and the geometric part of Lemma 6 (lines 914 to 935).

The 600 caps `𝒞` are the facets of `P`, indexed by the four-cliques of the edge graph
(`Facets`). `K` permutes them through its action on the vertices (`capVertexEquiv`). With the
pole `n_c = N / 4` of Lemma 5 and the depth `α = 121/125`, the cap region is
`U_c = {x ∈ P : n_c · x > α ‖n_c‖²}` and the cut hyperplane of `c` is `n_c · x = α ‖n_c‖²`.
The source works in the cell-pole model `P̃ = (4/φ) Q P` of Lemma 5; the similarity multiplies
every cut function by `(4/φ)²` (`cutValue_cellPoleMap`), so the side choices, chambers and
signatures are the same in both scales, and this file uses `P`.

A full-dimensional cut chamber is a choice of strict side of every cut hyperplane intersected with
`int P` (`chamber S`, where `S` is the set of caps on whose cap side the chamber lies). Its points
are the points of `cutComplement`, the part of `int P` off every cut hyperplane, with complete
signature `M(x) = {c : x ∈ U_c}` equal to `S`.

Lemma 6, geometric part:
* `convex_chamber`: a chamber is convex, so it has at most one connected component;
* `completeSignature_eq_iff`: two points off the cuts have the same complete signature exactly
  when they lie in the same connected component of `cutComplement`, so complete signatures
  identify the cut-chamber positions;
* `completeSignature_smul`: a symmetry `k` permutes the cap labels, `M(kx) = k M(x)`;
* `isPreconnected_patch`: the intersection of a chamber closure with a closed facet is convex,
  which is the source's reason for at most one patch per hosting facet (the patch itself, with
  its lower-dimensional boundary removed, is not stated).
-/

namespace ArchonFull600Theory

open Finset
open scoped goldenRatio Quaternion RealInnerProductSpace

/-! ### Caps and the action of `K` on them -/

/-- **𝒞.** The 600 caps, one for each facet of `P`, indexed by the facet's four vertices (the
four-cliques of the edge graph). -/
abbrev Cap := ↥(edgeGraph.cliqueFinset 4)

theorem card_cap : Fintype.card Cap = 600 := by
  rw [Fintype.card_coe]
  exact faceCounts.2.2.2.2

theorem card_capVertices (c : Cap) : Fintype.card ↥(c : Finset (Fin 120)) = 4 := by
  rw [Fintype.card_coe]
  exact (SimpleGraph.mem_cliqueFinset_iff.1 c.prop).card_eq

theorem image_smul_mem_cliqueFinset (A : symmetryGroup) {s : Finset (Fin 120)}
    (hs : s ∈ edgeGraph.cliqueFinset 4) : s.image (A • ·) ∈ edgeGraph.cliqueFinset 4 := by
  rw [SimpleGraph.mem_cliqueFinset_iff] at hs ⊢
  refine ⟨?_, ?_⟩
  · intro x hx y hy hxy
    simp only [coe_image, Set.mem_image, mem_coe] at hx hy
    obtain ⟨a, ha, rfl⟩ := hx
    obtain ⟨b, hb, rfl⟩ := hy
    rw [adj_smul_iff]
    exact hs.isClique (mem_coe.2 ha) (mem_coe.2 hb) fun h => hxy (h ▸ rfl)
  · rw [card_image_of_injective _ (MulAction.injective A)]
    exact hs.card_eq

/-- `K` permutes the caps through its action on the vertices. -/
noncomputable instance : MulAction symmetryGroup Cap where
  smul A c := ⟨(c : Finset (Fin 120)).image (A • ·), image_smul_mem_cliqueFinset A c.prop⟩
  one_smul c := Subtype.ext (by
    change (c : Finset (Fin 120)).image ((1 : symmetryGroup) • ·) = c
    simp)
  mul_smul A B c := Subtype.ext (by
    change (c : Finset (Fin 120)).image ((A * B) • ·) =
      ((c : Finset (Fin 120)).image (B • ·)).image (A • ·)
    rw [image_image]
    congr 1
    funext x
    exact mul_smul A B x)

theorem coe_smul_cap (A : symmetryGroup) (c : Cap) :
    ((A • c : Cap) : Finset (Fin 120)) = (c : Finset (Fin 120)).image (A • ·) := rfl

/-- A symmetry maps the four vertices of cap `c` onto those of cap `A • c`. -/
noncomputable def capVertexEquiv (A : symmetryGroup) (c : Cap) :
    ↥(c : Finset (Fin 120)) ≃ ↥((A • c : Cap) : Finset (Fin 120)) where
  toFun x := ⟨A • x.val, by rw [coe_smul_cap]; exact mem_image_of_mem _ x.prop⟩
  invFun y := ⟨A⁻¹ • y.val, by
    have hy : y.val ∈ (c : Finset (Fin 120)).image (A • ·) := y.prop
    obtain ⟨x, hx, hxy⟩ := mem_image.1 hy
    rw [← hxy, inv_smul_smul]
    exact hx⟩
  left_inv x := Subtype.ext (inv_smul_smul A x.val)
  right_inv y := Subtype.ext (smul_inv_smul A y.val)

/-! ### Cap regions and cut hyperplanes -/

/-- The cut depth `α = 121/125`. -/
noncomputable def capDepth : ℝ := 121 / 125

/-- The pole `n_c = N / 4` of cap `c` (Lemma 5). -/
noncomputable def capPole (c : Cap) : ℍ := facetPole c

/-- The cut function `n_c · x - α ‖n_c‖²` of cap `c`: positive exactly on the cap side of the
cut hyperplane of `c`. -/
noncomputable def cutValue (c : Cap) (x : ℍ) : ℝ := ⟪capPole c, x⟫ - capDepth * ‖capPole c‖ ^ 2

/-- **The cap region** `U_c = {x ∈ P : n_c · x > α ‖n_c‖²}`. -/
def capRegion (c : Cap) : Set ℍ := {x | x ∈ polytope ∧ 0 < cutValue c x}

theorem mem_capRegion_iff (c : Cap) (x : ℍ) :
    x ∈ capRegion c ↔ x ∈ polytope ∧ capDepth * ‖capPole c‖ ^ 2 < ⟪capPole c, x⟫ := by
  rw [capRegion, Set.mem_ofPred_eq, cutValue, sub_pos]

/-- The complement of the cut arrangement in `int P`: the union of the full-dimensional cut
chambers. -/
def cutComplement : Set ℍ := {x | x ∈ interior polytope ∧ ∀ c, cutValue c x ≠ 0}

/-- **A full-dimensional cut chamber**: the points of `int P` on the cap side of the cut of every
`c ∈ S` and on the other side of the cut of every `c ∉ S`. -/
def chamber (S : Finset Cap) : Set ℍ :=
  {x | x ∈ interior polytope ∧ ∀ c, (c ∈ S → 0 < cutValue c x) ∧ (c ∉ S → cutValue c x < 0)}

open Classical in
/-- **The complete cap signature** `M(x) = {c : x ∈ U_c}` of a point of `P`. -/
noncomputable def completeSignature (x : ℍ) : Finset Cap := univ.filter fun c => 0 < cutValue c x

theorem mem_completeSignature (x : ℍ) (c : Cap) :
    c ∈ completeSignature x ↔ 0 < cutValue c x := by
  classical
  rw [completeSignature, mem_filter]
  simp

theorem mem_completeSignature_iff {x : ℍ} (hx : x ∈ polytope) (c : Cap) :
    c ∈ completeSignature x ↔ x ∈ capRegion c := by
  rw [mem_completeSignature, capRegion, Set.mem_ofPred_eq, and_iff_right hx]

theorem continuous_cutValue (c : Cap) : Continuous (cutValue c) :=
  (continuous_const.inner continuous_id).sub continuous_const

theorem cutValue_combo (c : Cap) (x y : ℍ) {a b : ℝ} (hab : a + b = 1) :
    cutValue c (a • x + b • y) = a * cutValue c x + b * cutValue c y := by
  simp only [cutValue, inner_add_right, real_inner_smul_right]
  linear_combination (capDepth * ‖capPole c‖ ^ 2) * hab

theorem combo_pos {a b u v : ℝ} (ha : 0 ≤ a) (hb : 0 ≤ b) (hab : a + b = 1) (hu : 0 < u)
    (hv : 0 < v) : 0 < a * u + b * v := by
  rcases ha.lt_or_eq with h | h
  · exact add_pos_of_pos_of_nonneg (mul_pos h hu) (mul_nonneg hb hv.le)
  · rw [← h, zero_mul, zero_add, show b = 1 by linarith, one_mul]
    exact hv

/-! ### Chambers -/

theorem chamber_subset_cutComplement (S : Finset Cap) : chamber S ⊆ cutComplement := by
  intro x hx
  refine ⟨hx.1, fun c => ?_⟩
  by_cases hc : c ∈ S
  · exact ((hx.2 c).1 hc).ne'
  · exact ((hx.2 c).2 hc).ne

/-- **Lemma 6.** A chamber is convex, hence has at most one connected component. -/
theorem convex_chamber (S : Finset Cap) : Convex ℝ (chamber S) := by
  intro x hx y hy a b ha hb hab
  refine ⟨convex_polytope.interior hx.1 hy.1 ha hb hab, fun c => ⟨fun hc => ?_, fun hc => ?_⟩⟩
  · rw [cutValue_combo c x y hab]
    exact combo_pos ha hb hab ((hx.2 c).1 hc) ((hy.2 c).1 hc)
  · rw [cutValue_combo c x y hab]
    have h := combo_pos ha hb hab (neg_pos.2 ((hx.2 c).2 hc)) (neg_pos.2 ((hy.2 c).2 hc))
    linarith

theorem isPreconnected_chamber (S : Finset Cap) : IsPreconnected (chamber S) :=
  (convex_chamber S).isPreconnected

theorem completeSignature_of_mem_chamber {S : Finset Cap} {x : ℍ} (hx : x ∈ chamber S) :
    completeSignature x = S := by
  ext c
  rw [mem_completeSignature]
  by_cases hc : c ∈ S
  · exact iff_of_true ((hx.2 c).1 hc) hc
  · exact iff_of_false (not_lt.2 ((hx.2 c).2 hc).le) hc

theorem mem_chamber_completeSignature {x : ℍ} (hx : x ∈ cutComplement) :
    x ∈ chamber (completeSignature x) := by
  refine ⟨hx.1, fun c => ⟨fun hc => (mem_completeSignature x c).1 hc, fun hc => ?_⟩⟩
  rw [mem_completeSignature, not_lt] at hc
  exact lt_of_le_of_ne hc (hx.2 c)

/-- The sign of every cut function is constant on a preconnected set off the cuts. -/
theorem cutValue_pos_of_isPreconnected {C : Set ℍ} (hC : IsPreconnected C)
    (hsub : C ⊆ cutComplement) {x y : ℍ} (hx : x ∈ C) (hy : y ∈ C) (c : Cap)
    (h : 0 < cutValue c x) : 0 < cutValue c y := by
  by_contra hneg
  have hy' : cutValue c y < 0 := lt_of_le_of_ne (not_lt.1 hneg) ((hsub hy).2 c)
  obtain ⟨z, hz, hz0⟩ := hC.intermediate_value hy hx (continuous_cutValue c).continuousOn
    ⟨hy'.le, h.le⟩
  exact (hsub hz).2 c hz0

/-- **Lemma 6: complete cap signatures identify the cut-chamber positions.** Two points off the
cuts have the same complete signature exactly when they lie in the same connected component of
the cut complement. -/
theorem completeSignature_eq_iff {x y : ℍ} (hx : x ∈ cutComplement) (hy : y ∈ cutComplement) :
    completeSignature x = completeSignature y ↔ y ∈ connectedComponentIn cutComplement x := by
  constructor
  · intro h
    have hxS := mem_chamber_completeSignature hx
    have hyS := mem_chamber_completeSignature hy
    rw [← h] at hyS
    exact (isPreconnected_chamber _).subset_connectedComponentIn hxS
      (chamber_subset_cutComplement _) hyS
  · intro h
    have hC := isPreconnected_connectedComponentIn (x := x) (F := cutComplement)
    have hsub := connectedComponentIn_subset cutComplement x
    have hxC := mem_connectedComponentIn hx
    ext c
    rw [mem_completeSignature, mem_completeSignature]
    exact ⟨cutValue_pos_of_isPreconnected hC hsub hxC h c,
      cutValue_pos_of_isPreconnected hC hsub h hxC c⟩

/-! ### Symmetries and the similarity of Lemma 5 -/

theorem capPole_smul (A : symmetryGroup) (c : Cap) :
    capPole (A • c) = (A : ℍ ≃ₗᵢ[ℝ] ℍ) (capPole c) := by
  rw [capPole, capPole, coe_smul_cap, facetPole, facetPole, facetNormal, facetNormal,
    sum_image fun x _ y _ h => MulAction.injective A h, map_smul, map_sum]
  simp only [vertexR_smul]

/-- A symmetry carries the cut function of `c` to that of `k • c`. -/
theorem cutValue_smul (A : symmetryGroup) (c : Cap) (x : ℍ) :
    cutValue (A • c) ((A : ℍ ≃ₗᵢ[ℝ] ℍ) x) = cutValue c x := by
  rw [cutValue, cutValue, capPole_smul, LinearIsometryEquiv.inner_map_map,
    LinearIsometryEquiv.norm_map]

/-- **Lemma 6: `M(kA) = k M(A)`.** A symmetry permutes the cap labels of complete signatures. -/
theorem completeSignature_smul (A : symmetryGroup) (x : ℍ) :
    completeSignature ((A : ℍ ≃ₗᵢ[ℝ] ℍ) x) = (completeSignature x).image (A • ·) := by
  ext c
  rw [mem_image, mem_completeSignature]
  constructor
  · intro h
    refine ⟨A⁻¹ • c, ?_, smul_inv_smul A c⟩
    rw [mem_completeSignature, ← cutValue_smul A, smul_inv_smul]
    exact h
  · rintro ⟨d, hd, rfl⟩
    rw [cutValue_smul]
    exact (mem_completeSignature x d).1 hd

/-- Symmetries map the cut complement onto itself. -/
theorem apply_mem_cutComplement (A : symmetryGroup) {x : ℍ} (hx : x ∈ cutComplement) :
    (A : ℍ ≃ₗᵢ[ℝ] ℍ) x ∈ cutComplement := by
  refine ⟨?_, fun c => ?_⟩
  · have h : (A : ℍ ≃ₗᵢ[ℝ] ℍ).toHomeomorph x ∈
        (A : ℍ ≃ₗᵢ[ℝ] ℍ).toHomeomorph '' interior polytope :=
      Set.mem_image_of_mem _ hx.1
    rwa [Homeomorph.image_interior, LinearIsometryEquiv.coe_toHomeomorph,
      show (A : ℍ ≃ₗᵢ[ℝ] ℍ) '' polytope = polytope from A.prop] at h
  · rw [← smul_inv_smul A c, cutValue_smul]
    exact hx.2 _

/-- The similarity `σ x = (4 / φ) Q x` of Lemma 5 multiplies every cut function by `(4 / φ)²`:
the cuts of `P̃` are the images of the cuts of `P`, with the same sides. -/
theorem cutValue_cellPoleMap (Q : ℍ ≃ₗᵢ[ℝ] ℍ) (c : Cap) (x : ℍ) :
    ⟪cellPoleMap Q (capPole c), cellPoleMap Q x⟫ - capDepth * ‖cellPoleMap Q (capPole c)‖ ^ 2 =
      (4 / φ) ^ 2 * cutValue c x := by
  rw [inner_cellPoleMap, norm_cellPoleMap, cutValue]
  ring

/-- **Lemma 6, the convexity behind one patch per hosting facet.** The intersection of a
chamber closure with a closed facet is convex, hence connected when nonempty. The source's patch,
this set with its lower-dimensional boundary removed, is not stated here. -/
theorem isPreconnected_patch (S : Finset Cap) (h : Cap) :
    IsPreconnected (closure (chamber S) ∩ cliqueTetra h) :=
  ((convex_chamber S).closure.inter (convex_convexHull ℝ _)).isPreconnected

end ArchonFull600Theory
