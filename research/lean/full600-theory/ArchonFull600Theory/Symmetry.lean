import ArchonFull600Theory.BinaryIcosahedral
import ArchonFull600Theory.Facets
import ArchonFull600Theory.QuaternionMaps
import Mathlib.Analysis.Normed.Affine.MazurUlam
import Mathlib.Data.ZMod.QuotientGroup

set_option linter.style.header false in
/-!
# The finite geometric symmetry group

Lemma 4 without its Coxeter presentation (the presentation `K ≅ W(H₄)` is Milestone 4).

`K` is the group of orthogonal maps of `ℍ` that map the 600-cell `P = conv V` onto itself, and
`K⁺ = K ∩ SO(4)`. `isometry_mem_symmetryGroup` shows that every isometry of `ℍ` that maps `P`
onto itself is one of these maps (in particular it fixes the centre `0`), so `K` is the full
rigid symmetry group of `P`.

The lower bound comes from the maps `L_{p,q}` with `p, q ∈ I*` and from `J`: `(p, q) ↦ L_{p,q}`
has kernel `{(1, 1), (-1, -1)}` on `I* × I*`, so it gives `120² / 2 = 7200` rotations, and
`det J = -1`. The upper bound is the proof of the source, not a computation: a symmetry fixing
vertex `0` is determined by the images of the three other vertices of the facet
`{0, 30, 31, 32}` (the four vertices are a basis of `ℍ`), and these images form an ordered
triangle of the icosahedral link of vertex `0`, of which there are `20 · 6 = 120`
(`card_linkTriangles`, a kernel computation over the twelve neighbours of vertex `0`). The orbit
of vertex `0` has at most 120 points, so `|K| ≤ 120 · 120 = 14400`.
-/

namespace ArchonFull600Theory

open scoped Quaternion RealInnerProductSpace
open Finset

/-! ### The groups `K` and `K⁺` -/

/-- **O5.** The symmetry group `K` of the 600-cell: the orthogonal maps of `ℍ` that map
`P = conv V` onto itself. -/
def symmetryGroup : Subgroup (ℍ ≃ₗᵢ[ℝ] ℍ) where
  carrier := {A | A '' polytope = polytope}
  one_mem' := by simp
  mul_mem' := by
    intro A B hA hB
    simp only [Set.mem_ofPred_eq] at hA hB ⊢
    rw [LinearIsometryEquiv.coe_mul, Set.image_comp, hB, hA]
  inv_mem' := by
    intro A hA
    simp only [Set.mem_ofPred_eq] at hA ⊢
    rw [LinearIsometryEquiv.coe_inv]
    conv_lhs => rw [← hA]
    rw [← Set.image_comp, A.symm_comp_self, Set.image_id]

/-- **O5.** `K⁺`: the symmetries of `P` of determinant `1`. -/
noncomputable def rotationGroup : Subgroup (ℍ ≃ₗᵢ[ℝ] ℍ) := symmetryGroup ⊓ specialOrthogonal

theorem mem_rotationGroup {A : ℍ ≃ₗᵢ[ℝ] ℍ} :
    A ∈ rotationGroup ↔ A ∈ symmetryGroup ∧ orthDet A = 1 := by
  rw [rotationGroup, Subgroup.mem_inf, mem_specialOrthogonal]

/-- An orthogonal map preserves `P` exactly when it preserves `V`: `V` is the set of extreme
points of `P`. -/
theorem mem_symmetryGroup_iff {A : ℍ ≃ₗᵢ[ℝ] ℍ} :
    A ∈ symmetryGroup ↔ A '' vertexSet = vertexSet := by
  have hP : polytope.extremePoints ℝ = vertexSet := exposedVertices.2.2.1
  constructor
  · intro h
    change A '' polytope = polytope at h
    rw [← hP, image_extremePoints, h]
  · intro h
    change A '' convexHull ℝ vertexSet = convexHull ℝ vertexSet
    have hL := (A.toLinearEquiv : ℍ →ₗ[ℝ] ℍ).image_convexHull vertexSet
    simp only [LinearEquiv.coe_coe, LinearIsometryEquiv.coe_toLinearEquiv] at hL
    rw [hL, h]

theorem vertexR_eq_smul (i : Fin 120) : vertexR i = (2 : ℝ) • unitVertex i := by
  rw [unitVertex, smul_smul]
  norm_num

theorem vertexSet_finite : vertexSet.Finite := range_vertexR ▸ Set.finite_range vertexR

/-- An orthogonal map that sends every vertex to a vertex is a symmetry of `P`. -/
theorem mem_symmetryGroup_of_forall {A : ℍ ≃ₗᵢ[ℝ] ℍ}
    (h : ∀ i, ∃ j, A (vertexR i) = vertexR j) : A ∈ symmetryGroup := by
  rw [mem_symmetryGroup_iff]
  have hsub : A '' vertexSet ⊆ vertexSet := by
    rintro _ ⟨x, hx, rfl⟩
    rw [← range_vertexR] at hx ⊢
    obtain ⟨i, rfl⟩ := hx
    obtain ⟨j, hj⟩ := h i
    exact ⟨j, hj.symm⟩
  exact Set.eq_of_subset_of_ncard_le hsub (Set.ncard_image_of_injective _ A.injective).ge
    vertexSet_finite

/-! ### The action of `K` on the vertices -/

theorem exists_vertexR_eq (A : symmetryGroup) (i : Fin 120) :
    ∃ j, (A : ℍ ≃ₗᵢ[ℝ] ℍ) (vertexR i) = vertexR j := by
  have hi : (A : ℍ ≃ₗᵢ[ℝ] ℍ) (vertexR i) ∈ vertexSet := by
    rw [← mem_symmetryGroup_iff.mp A.2]
    exact Set.mem_image_of_mem _ (by rw [← range_vertexR]; exact Set.mem_range_self i)
  rw [← range_vertexR] at hi
  obtain ⟨j, hj⟩ := hi
  exact ⟨j, hj.symm⟩

/-- The vertex that a symmetry sends vertex `i` to. -/
noncomputable def vertexImage (A : symmetryGroup) (i : Fin 120) : Fin 120 :=
  (exists_vertexR_eq A i).choose

theorem vertexR_vertexImage (A : symmetryGroup) (i : Fin 120) :
    vertexR (vertexImage A i) = (A : ℍ ≃ₗᵢ[ℝ] ℍ) (vertexR i) :=
  (exists_vertexR_eq A i).choose_spec.symm

/-- `K` acts on the 120 vertices. -/
noncomputable instance : MulAction symmetryGroup (Fin 120) where
  smul := vertexImage
  one_smul i := vertexR_injective (by
    change vertexR (vertexImage 1 i) = vertexR i
    rw [vertexR_vertexImage]
    rfl)
  mul_smul A B i := vertexR_injective (by
    change vertexR (vertexImage (A * B) i) = vertexR (vertexImage A (vertexImage B i))
    rw [vertexR_vertexImage, vertexR_vertexImage, vertexR_vertexImage, Subgroup.coe_mul,
      LinearIsometryEquiv.coe_mul, Function.comp_apply])

theorem vertexR_smul (A : symmetryGroup) (i : Fin 120) :
    vertexR (A • i) = (A : ℍ ≃ₗᵢ[ℝ] ℍ) (vertexR i) :=
  vertexR_vertexImage A i

/-- Symmetries preserve the edge graph. -/
theorem adj_smul_iff (A : symmetryGroup) (i j : Fin 120) :
    edgeGraph.Adj (A • i) (A • j) ↔ edgeGraph.Adj i j := by
  rw [edgeGraph_adj_iff, edgeGraph_adj_iff, vertexR_smul, vertexR_smul,
    LinearIsometryEquiv.inner_map_map]

/-! ### A symmetry is determined by one facet -/

/-- Two orthogonal maps that agree on the four vertices of a facet are equal: these four vertices
are linearly independent (their Gram matrix is positive definite), so they are a basis of `ℍ`. -/
theorem eq_of_eqOn_clique {s : Finset (Fin 120)} (hs : s ∈ edgeGraph.cliqueFinset 4)
    {A B : ℍ ≃ₗᵢ[ℝ] ℍ} (h : ∀ k ∈ s, A (vertexR k) = B (vertexR k)) : A = B := by
  have hli : LinearIndependent ℝ (vertexR ∘ ((↑) : s → Fin 120)) :=
    Matrix.posDef_gram_iff_linearIndependent.mp (facetGram hs).2
  have hcard : Fintype.card s = Module.finrank ℝ ℍ := by
    rw [Fintype.card_coe, (SimpleGraph.mem_cliqueFinset_iff.mp hs).2, Quaternion.finrank_eq_four]
  have : Nonempty s :=
    Fintype.card_pos_iff.mp (by rw [hcard, Quaternion.finrank_eq_four]; norm_num)
  have hAB : (A.toLinearEquiv : ℍ →ₗ[ℝ] ℍ) = (B.toLinearEquiv : ℍ →ₗ[ℝ] ℍ) :=
    (basisOfLinearIndependentOfCardEqFinrank hli hcard).ext fun k => by
      rw [coe_basisOfLinearIndependentOfCardEqFinrank]
      exact h k k.2
  exact LinearIsometryEquiv.ext fun x => LinearMap.congr_fun hAB x

/-- The facet `{0, 30, 31, 32}` at vertex `0`. -/
def facet0 : Finset (Fin 120) := {0, 30, 31, 32}

theorem isNClique_facet0 : edgeGraph.IsNClique 4 facet0 := by decide +kernel

theorem facet0_mem : facet0 ∈ edgeGraph.cliqueFinset 4 :=
  SimpleGraph.mem_cliqueFinset_iff.mpr isNClique_facet0

/-! ### The stabilizer bound from the icosahedral link -/

/-- The twelve neighbours of vertex `0`: the vertices of its icosahedral vertex figure. -/
def link0 : Finset (Fin 120) := {30, 31, 32, 42, 43, 44, 78, 79, 80, 90, 91, 92}

theorem link0_spec : ∀ x : Fin 120, edgeGraph.Adj 0 x → x ∈ link0 := by decide +kernel

/-- The ordered triangles of the link of vertex `0`: triples of pairwise adjacent neighbours. -/
def linkTriangles : Finset (Fin 120 × Fin 120 × Fin 120) :=
  (link0 ×ˢ link0 ×ˢ link0).filter fun t =>
    edgeGraph.Adj t.1 t.2.1 ∧ edgeGraph.Adj t.1 t.2.2 ∧ edgeGraph.Adj t.2.1 t.2.2

/-- The link of a vertex has `20 · 6 = 120` ordered triangles. -/
theorem card_linkTriangles : #linkTriangles = 120 := by decide +kernel

theorem adj_facet0 :
    edgeGraph.Adj 0 30 ∧ edgeGraph.Adj 0 31 ∧ edgeGraph.Adj 0 32 ∧ edgeGraph.Adj 30 31 ∧
      edgeGraph.Adj 30 32 ∧ edgeGraph.Adj 31 32 := by decide +kernel

theorem smul_mem_linkTriangles {A : symmetryGroup} (hA : A • (0 : Fin 120) = 0) :
    (A • (30 : Fin 120), A • (31 : Fin 120), A • (32 : Fin 120)) ∈ linkTriangles := by
  have hadj : ∀ i j : Fin 120, edgeGraph.Adj i j → edgeGraph.Adj (A • i) (A • j) :=
    fun i j h => (adj_smul_iff A i j).mpr h
  have h0 : ∀ i : Fin 120, edgeGraph.Adj 0 i → A • i ∈ link0 := fun i h => by
    have h' := hadj 0 i h
    rw [hA] at h'
    exact link0_spec _ h'
  obtain ⟨a1, a2, a3, a4, a5, a6⟩ := adj_facet0
  rw [linkTriangles, Finset.mem_filter, Finset.mem_product, Finset.mem_product]
  exact ⟨⟨h0 30 a1, h0 31 a2, h0 32 a3⟩, hadj 30 31 a4, hadj 30 32 a5, hadj 31 32 a6⟩

/-- The stabilizer of vertex `0` acts faithfully on its link: a symmetry fixing vertex `0` is
determined by the images of vertices `30`, `31` and `32`. -/
theorem stabilizer_injective :
    Function.Injective fun A : MulAction.stabilizer symmetryGroup (0 : Fin 120) =>
      ((A : symmetryGroup) • (30 : Fin 120), (A : symmetryGroup) • (31 : Fin 120),
        (A : symmetryGroup) • (32 : Fin 120)) := by
  intro A B h
  simp only [Prod.mk.injEq] at h
  obtain ⟨h30, h31, h32⟩ := h
  have hA : (A : symmetryGroup) • (0 : Fin 120) = 0 := MulAction.mem_stabilizer_iff.mp A.2
  have hB : (B : symmetryGroup) • (0 : Fin 120) = 0 := MulAction.mem_stabilizer_iff.mp B.2
  apply Subtype.ext
  apply Subtype.ext
  apply eq_of_eqOn_clique facet0_mem
  intro k hk
  rw [← vertexR_smul, ← vertexR_smul]
  simp only [facet0, Finset.mem_insert, Finset.mem_singleton] at hk
  rcases hk with rfl | rfl | rfl | rfl
  · rw [hA, hB]
  · rw [h30]
  · rw [h31]
  · rw [h32]

/-- **Stabilizer bound.** The stabilizer of vertex `0` has at most 120 elements, the number of
ordered triangles among its twelve neighbours (`card_linkTriangles`; `20 · 6` in the source).
-/
theorem card_stabilizer_le :
    Finite (MulAction.stabilizer symmetryGroup (0 : Fin 120)) ∧
      Nat.card (MulAction.stabilizer symmetryGroup (0 : Fin 120)) ≤ 120 := by
  let f : MulAction.stabilizer symmetryGroup (0 : Fin 120) → linkTriangles := fun A =>
    ⟨_, smul_mem_linkTriangles (MulAction.mem_stabilizer_iff.mp A.2)⟩
  have hf : Function.Injective f := fun A B h =>
    stabilizer_injective (congrArg Subtype.val h)
  refine ⟨Finite.of_injective f hf, ?_⟩
  have h := Nat.card_le_card_of_injective f hf
  rwa [Nat.card_eq_finsetCard, card_linkTriangles] at h

/-- **Upper bound.** `K` is finite with at most `120 · 120 = 14400` elements (orbit–stabilizer). -/
theorem card_symmetryGroup_le : Finite symmetryGroup ∧ Nat.card symmetryGroup ≤ 14400 := by
  obtain ⟨hfin, hstab⟩ := card_stabilizer_le
  have hc := Subgroup.card_mul_index (MulAction.stabilizer symmetryGroup (0 : Fin 120))
  rw [MulAction.index_stabilizer] at hc
  have horb : (MulAction.orbit symmetryGroup (0 : Fin 120)).ncard ≤ 120 := by
    calc _ ≤ (Set.univ : Set (Fin 120)).ncard := Set.ncard_le_ncard (Set.subset_univ _)
      _ = 120 := by rw [Set.ncard_univ, Nat.card_eq_fintype_card, Fintype.card_fin]
  have hpos1 : 0 < Nat.card (MulAction.stabilizer symmetryGroup (0 : Fin 120)) := Nat.card_pos
  have hpos2 : 0 < (MulAction.orbit symmetryGroup (0 : Fin 120)).ncard :=
    (Set.ncard_pos (Set.toFinite _)).mpr ⟨0, MulAction.mem_orbit_self 0⟩
  refine ⟨Nat.finite_of_card_ne_zero ?_, ?_⟩
  · rw [← hc]
    exact (Nat.mul_pos hpos1 hpos2).ne'
  · rw [← hc]
    calc _ ≤ 120 * 120 := Nat.mul_le_mul hstab horb
      _ = 14400 := by norm_num

/-! ### The lower bound: `L_{p,q}` and `J` -/

theorem lpq_mem_symmetryGroup {p q : unitary ℍ} (hp : p ∈ binaryIcosahedral)
    (hq : q ∈ binaryIcosahedral) : lpq p q ∈ symmetryGroup := by
  apply mem_symmetryGroup_of_forall
  intro i
  obtain ⟨a, ha⟩ : (p : ℍ) ∈ Set.range unitVertex := hp
  obtain ⟨b, hb⟩ : (q : ℍ) ∈ Set.range unitVertex := hq
  obtain ⟨c, hc⟩ := unitVertex_mul a i
  obtain ⟨d, hd⟩ := unitVertex_mul c (tableFin conjTable b)
  refine ⟨d, ?_⟩
  rw [lpq_apply, ← ha, ← hb, star_unitVertex, vertexR_eq_smul, vertexR_eq_smul, mul_smul_comm,
    smul_mul_assoc, hc, hd]

theorem conjIso_mem_symmetryGroup : conjIso ∈ symmetryGroup := by
  apply mem_symmetryGroup_of_forall
  intro i
  refine ⟨tableFin conjTable i, ?_⟩
  rw [conjIso_apply, vertexR_eq_smul, vertexR_eq_smul, Quaternion.star_smul, star_unitVertex]

/-- `I* × I*` as a subgroup of `Sp(1) × Sp(1)`. -/
def icosianPairs : Subgroup (unitary ℍ × unitary ℍ) := binaryIcosahedral.prod binaryIcosahedral

theorem card_icosianPairs : Nat.card icosianPairs = 14400 := by
  rw [icosianPairs, Nat.card_congr (Subgroup.prodEquiv _ _).toEquiv, Nat.card_prod,
    binaryIcosahedralGroup.1]

/-- `(-1, -1)` in `I* × I*`. -/
def negOnePair : icosianPairs :=
  ⟨(-1, -1), Subgroup.mem_prod.mpr ⟨neg_one_mem_binaryIcosahedral, neg_one_mem_binaryIcosahedral⟩⟩

theorem orderOf_negOnePair : orderOf negOnePair = 2 := by
  have h2 : negOnePair ^ 2 = 1 := by
    apply Subtype.ext
    rw [SubgroupClass.coe_pow]
    change ((-1 : unitary ℍ), (-1 : unitary ℍ)) ^ 2 = 1
    rw [Prod.pow_mk, neg_one_sq]
    rfl
  have h1 : negOnePair ≠ 1 := by
    intro h
    have h' := congrArg
      (fun x : icosianPairs => (((x : unitary ℍ × unitary ℍ).1 : unitary ℍ) : ℍ).re) h
    norm_num [negOnePair, Unitary.coe_neg] at h'
  exact orderOf_eq_prime h2 h1

/-- `(p, q) ↦ L_{p,q}` on `I* × I*`, with values in `K⁺`. -/
noncomputable def icosianRotation : icosianPairs →* rotationGroup :=
  (lpqHom.comp icosianPairs.subtype).codRestrict rotationGroup fun pq =>
    mem_rotationGroup.mpr
      ⟨lpq_mem_symmetryGroup (Subgroup.mem_prod.mp pq.2).1 (Subgroup.mem_prod.mp pq.2).2,
        orthDet_lpq _ _⟩

theorem coe_icosianRotation (pq : icosianPairs) :
    (icosianRotation pq : ℍ ≃ₗᵢ[ℝ] ℍ) =
      lpq (pq : unitary ℍ × unitary ℍ).1 (pq : unitary ℍ × unitary ℍ).2 := rfl

/-- **Kernel.** On `I* × I*` the kernel of `(p, q) ↦ L_{p,q}` is `{(1, 1), (-1, -1)}`. -/
theorem ker_icosianRotation : icosianRotation.ker = Subgroup.zpowers negOnePair := by
  apply le_antisymm
  · intro pq hpq
    rw [MonoidHom.mem_ker] at hpq
    have h1 : lpq (pq : unitary ℍ × unitary ℍ).1 (pq : unitary ℍ × unitary ℍ).2 = 1 := by
      rw [← coe_icosianRotation, hpq]
      rfl
    rcases (lpq_eq_one_iff _ _).mp h1 with ⟨ha, hb⟩ | ⟨ha, hb⟩
    · rw [show pq = 1 from Subtype.ext (Prod.ext ha hb)]
      exact Subgroup.one_mem _
    · rw [show pq = negOnePair from Subtype.ext (Prod.ext ha hb)]
      exact Subgroup.mem_zpowers _
  · rw [Subgroup.zpowers_le, MonoidHom.mem_ker]
    exact Subtype.ext ((lpq_eq_one_iff (-1) (-1)).mpr (Or.inr ⟨rfl, rfl⟩))

instance : (Subgroup.zpowers negOnePair).Normal := by
  rw [← ker_icosianRotation]
  infer_instance

theorem card_range_icosianRotation : Nat.card icosianRotation.range = 7200 := by
  have h := Subgroup.card_eq_card_quotient_mul_card_subgroup icosianRotation.ker
  rw [card_icosianPairs, ker_icosianRotation, Nat.card_zpowers, orderOf_negOnePair,
    Nat.card_congr (QuotientGroup.quotientMulEquivOfEq ker_icosianRotation.symm |>.trans
      (QuotientGroup.quotientKerEquivRange icosianRotation)).toEquiv] at h
  omega

/-! ### The orders of `K⁺` and `K` -/

/-- `K⁺` and `K⁺ J` are disjoint inside `K`, so `2 |K⁺| ≤ |K|`. -/
theorem two_mul_card_rotationGroup_le :
    Finite rotationGroup ∧ 2 * Nat.card rotationGroup ≤ Nat.card symmetryGroup := by
  have := card_symmetryGroup_le.1
  have hK : ∀ A : rotationGroup, (A : ℍ ≃ₗᵢ[ℝ] ℍ) ∈ symmetryGroup := fun A =>
    (mem_rotationGroup.mp A.2).1
  have hd : ∀ A : rotationGroup, orthDet (A : ℍ ≃ₗᵢ[ℝ] ℍ) = 1 := fun A =>
    (mem_rotationGroup.mp A.2).2
  let f : rotationGroup ⊕ rotationGroup → symmetryGroup := fun x =>
    match x with
    | .inl A => ⟨A, hK A⟩
    | .inr A => ⟨A * conjIso, mul_mem (hK A) conjIso_mem_symmetryGroup⟩
  have hf : Function.Injective f := by
    rintro (A | A) (B | B) h
    · have h' := congrArg Subtype.val h
      exact congrArg Sum.inl (Subtype.ext h')
    · have h' := congrArg (fun x : symmetryGroup => orthDet (x : ℍ ≃ₗᵢ[ℝ] ℍ)) h
      simp only [f, map_mul, orthDet_conjIso, hd] at h'
      norm_num at h'
    · have h' := congrArg (fun x : symmetryGroup => orthDet (x : ℍ ≃ₗᵢ[ℝ] ℍ)) h
      simp only [f, map_mul, orthDet_conjIso, hd] at h'
      norm_num at h'
    · have h' := congrArg Subtype.val h
      exact congrArg Sum.inr (Subtype.ext (mul_right_cancel h'))
  have : Finite rotationGroup := Finite.of_injective _ (hf.comp Sum.inl_injective)
  refine ⟨this, ?_⟩
  have h := Nat.card_le_card_of_injective f hf
  rwa [Nat.card_sum, ← two_mul] at h

theorem icosianRotation_surjective : Function.Surjective icosianRotation := by
  obtain ⟨hfin, h2⟩ := two_mul_card_rotationGroup_le
  have hle : Nat.card rotationGroup ≤ 7200 := by
    have := card_symmetryGroup_le.2
    omega
  have hrange : icosianRotation.range = ⊤ := by
    apply Subgroup.eq_top_of_card_eq
    have := Subgroup.card_le_card_group icosianRotation.range
    rw [card_range_icosianRotation] at this ⊢
    omega
  exact MonoidHom.range_eq_top.mp hrange

/-- **Lemma 4 (orders).** `K⁺ ≅ (I* × I*) / ⟨(-1, -1)⟩` through `(p, q) ↦ L_{p,q}`. -/
noncomputable def rotationGroupEquiv :
    icosianPairs ⧸ Subgroup.zpowers negOnePair ≃* rotationGroup :=
  (QuotientGroup.quotientMulEquivOfEq ker_icosianRotation.symm).trans
    (QuotientGroup.quotientKerEquivOfSurjective _ icosianRotation_surjective)

theorem rotationGroupEquiv_mk (pq : icosianPairs) :
    (rotationGroupEquiv (pq : icosianPairs ⧸ Subgroup.zpowers negOnePair) : ℍ ≃ₗᵢ[ℝ] ℍ) =
      lpq (pq : unitary ℍ × unitary ℍ).1 (pq : unitary ℍ × unitary ℍ).2 := rfl

theorem card_rotationGroup : Nat.card rotationGroup = 7200 := by
  rw [← card_range_icosianRotation, MonoidHom.range_eq_top.mpr icosianRotation_surjective]
  exact (Subgroup.card_top).symm

/-- Every symmetry of `P` is `L_{p,q}` or `L_{p,q} J` for some `p, q ∈ I*`. -/
theorem mem_symmetryGroup_iff_lpq {A : ℍ ≃ₗᵢ[ℝ] ℍ} :
    A ∈ symmetryGroup ↔ ∃ p ∈ binaryIcosahedral, ∃ q ∈ binaryIcosahedral,
      A = lpq p q ∨ A = lpq p q * conjIso := by
  constructor
  · intro hA
    have hrot : ∀ B : ℍ ≃ₗᵢ[ℝ] ℍ, B ∈ rotationGroup → ∃ p ∈ binaryIcosahedral,
        ∃ q ∈ binaryIcosahedral, B = lpq p q := fun B hB => by
      obtain ⟨pq, hpq⟩ := icosianRotation_surjective ⟨B, hB⟩
      refine ⟨_, (Subgroup.mem_prod.mp pq.2).1, _, (Subgroup.mem_prod.mp pq.2).2, ?_⟩
      rw [← coe_icosianRotation, hpq]
    rcases orthDet_eq_one_or A with h | h
    · obtain ⟨p, hp, q, hq, rfl⟩ := hrot A (mem_rotationGroup.mpr ⟨hA, h⟩)
      exact ⟨p, hp, q, hq, Or.inl rfl⟩
    · have hAJ : A * conjIso ∈ rotationGroup := mem_rotationGroup.mpr
        ⟨mul_mem hA conjIso_mem_symmetryGroup, by rw [orthDet_mul_conjIso, h, neg_neg]⟩
      obtain ⟨p, hp, q, hq, hpq⟩ := hrot _ hAJ
      refine ⟨p, hp, q, hq, Or.inr ?_⟩
      rw [← hpq, mul_assoc, conjIso_mul_self, mul_one]
  · rintro ⟨p, hp, q, hq, rfl | rfl⟩
    · exact lpq_mem_symmetryGroup hp hq
    · exact mul_mem (lpq_mem_symmetryGroup hp hq) conjIso_mem_symmetryGroup

theorem card_symmetryGroup : Nat.card symmetryGroup = 14400 := by
  have h1 := two_mul_card_rotationGroup_le.2
  have h2 := card_symmetryGroup_le.2
  rw [card_rotationGroup] at h1
  omega

/-- **Lemma 4 (without the Coxeter presentation).** `K⁺` has order 7200 and is isomorphic to
`(I* × I*) / ⟨(-1, -1)⟩` by an isomorphism that sends the class of `(p, q)` to `L_{p,q}`;
`J ∈ K \ K⁺`, so quaternion conjugation extends `K⁺` to `K`, of order 14400; every element of
`K` is `L_{p,q}` or `L_{p,q} J` with `p, q ∈ I*`; and `J L_{p,q} J = L_{q,p}`. -/
theorem finiteSymmetryGroup :
    Nat.card rotationGroup = 7200 ∧ Nat.card symmetryGroup = 14400 ∧
      (∃ e : icosianPairs ⧸ Subgroup.zpowers negOnePair ≃* rotationGroup,
        ∀ pq : icosianPairs,
          (e (pq : icosianPairs ⧸ Subgroup.zpowers negOnePair) : ℍ ≃ₗᵢ[ℝ] ℍ) =
            lpq (pq : unitary ℍ × unitary ℍ).1 (pq : unitary ℍ × unitary ℍ).2) ∧
      conjIso ∈ symmetryGroup ∧ conjIso ∉ rotationGroup ∧
      (∀ A : ℍ ≃ₗᵢ[ℝ] ℍ, A ∈ symmetryGroup ↔ ∃ p ∈ binaryIcosahedral,
        ∃ q ∈ binaryIcosahedral, A = lpq p q ∨ A = lpq p q * conjIso) ∧
      ∀ p q : unitary ℍ, conjIso * lpq p q * conjIso = lpq q p :=
  ⟨card_rotationGroup, card_symmetryGroup, ⟨rotationGroupEquiv, rotationGroupEquiv_mk⟩,
    conjIso_mem_symmetryGroup,
    fun h => conjIso_not_mem_specialOrthogonal (Subgroup.mem_inf.mp h).2,
    fun _ => mem_symmetryGroup_iff_lpq, conjIso_lpq_conjIso⟩

/-! ### Rigid symmetries -/

theorem polytope_subset_closedBall : polytope ⊆ Metric.closedBall 0 2 :=
  convexHull_min (fun q hq => by simp [vertexNorm q hq]) (convex_closedBall 0 2)

/-- **Rigid symmetries.** An isometry of `ℍ` that maps `P` onto itself is, as a map, an element
of `K`; in particular it fixes the centre `0`. The proof shows `f 0 = 0` first and then uses
the Mazur–Ulam theorem. -/
theorem isometry_mem_symmetryGroup (f : ℍ ≃ᵢ ℍ) (hf : f '' polytope = polytope) :
    ∃ A ∈ symmetryGroup, ⇑A = ⇑f := by
  set c := f 0
  have hball : ∀ x ∈ polytope, ‖x - c‖ ≤ 2 := by
    intro x hx
    rw [← hf] at hx
    obtain ⟨y, hy, rfl⟩ := hx
    rw [← dist_eq_norm, f.dist_eq, dist_zero_right]
    simpa using polytope_subset_closedBall hy
  have hv : vertexR 0 ∈ polytope :=
    subset_convexHull ℝ _ (by rw [← range_vertexR]; exact Set.mem_range_self 0)
  have hw : vertexR 4 ∈ polytope :=
    subset_convexHull ℝ _ (by rw [← range_vertexR]; exact Set.mem_range_self 4)
  have hneg : vertexR 4 = -vertexR 0 := by
    rw [vertexR_eq_smul, vertexR_eq_smul, unitVertex_four, unitVertex_zero, smul_neg]
  have h1 := hball _ hv
  have h2 := hball _ hw
  rw [hneg, show -vertexR 0 - c = -(vertexR 0 + c) by abel, norm_neg] at h2
  have hn : ‖vertexR 0‖ = 2 := norm_vertexR 0
  have e1 := norm_sub_sq_real (vertexR 0) c
  have e2 := norm_add_sq_real (vertexR 0) c
  have hs1 : ‖vertexR 0 - c‖ ^ 2 ≤ 4 := by
    nlinarith [norm_nonneg (vertexR 0 - c)]
  have hs2 : ‖vertexR 0 + c‖ ^ 2 ≤ 4 := by
    nlinarith [norm_nonneg (vertexR 0 + c)]
  have hc : ‖c‖ ^ 2 ≤ 0 := by nlinarith
  have h0 : f 0 = 0 := norm_eq_zero.mp (by nlinarith [norm_nonneg c])
  refine ⟨f.toRealLinearIsometryEquivOfMapZero h0, ?_, by simp⟩
  change _ '' polytope = polytope
  rw [IsometryEquiv.coe_toRealLinearIsometryEquivOfMapZero, hf]

end ArchonFull600Theory
