import ArchonFull600Theory.SelectedOrbits
import Mathlib.GroupTheory.SemidirectProduct

set_option linter.style.header false in
/-!
# Lemma 12: geometric normalization of the legal group

Source: `research/Full_600cell_Puzzle_Theory.tex`, Lemma 12 (lines 1267 to 1316) and the remark
closing "Distinct symmetry and motion groups" (lines 1357 to 1360).

Under R, F and E, write `κ = geom : K →* S_𝒮` for the faithful geometric action of hypothesis E.

* `HypothesisE.conj_legalGroup`: `κ(k) G κ(k)⁻¹ = G` for every `k ∈ K`. Hypothesis E's field
  `geom_capAction` is the source's `k R_{c,a} k⁻¹ = R_{kc, kak⁻¹}`, and by F and Lemma 6 every cap
  rotation is legal.
* `HypothesisE.legalGroup_inf_range`: `G ∩ κ(K) = 1`. A legal element fixes every centre; a
  geometric `κ(k)` that fixes every centre fixes every cap (`geom_center`), and a symmetry of `P`
  that fixes every facet is the identity (`eq_one_of_forall_smul_cap`). The source argues through
  the poles spanning `ℝ⁴`; here the facet `{0, 30, 31, 32}` and four facets that meet it in one
  vertex each pin down its four vertices, which form a basis of `ℍ` (Milestone 3).
* `HypothesisE.semidirectEquiv`: `⟨G, κ(K)⟩ ≅ G ⋊ K`, for the action of `K` on `G` by
  conjugation, through `(g, k) ↦ g κ(k)`; `HypothesisE.existsUnique_mul` is the unique form
  `g κ(k)`.
* `HypothesisC.card_symmetryGroup_lt`: under R and C, the balanced-colour bound `B` alone already
  separates the legal group from the geometric group, `|K| < |G|`.
-/

namespace ArchonFull600Theory

open Equiv Finset

/-! ### A symmetry fixing every cap is the identity -/

/-- If a symmetry fixes two caps that meet in the single vertex `v`, it fixes `v`. -/
theorem smul_eq_of_inter_eq {A : symmetryGroup} {c d : Cap} (hc : A • c = c) (hd : A • d = d)
    {v : Fin 120} (h : (c : Finset (Fin 120)) ∩ d = {v}) : A • v = v := by
  have hv : v ∈ (c : Finset (Fin 120)) ∩ d := h ▸ mem_singleton_self v
  have h1 : A • v ∈ ((A • c : Cap) : Finset (Fin 120)) := by
    rw [coe_smul_cap]
    exact mem_image_of_mem _ (mem_inter.1 hv).1
  have h2 : A • v ∈ ((A • d : Cap) : Finset (Fin 120)) := by
    rw [coe_smul_cap]
    exact mem_image_of_mem _ (mem_inter.1 hv).2
  rw [hc] at h1
  rw [hd] at h2
  have h3 := mem_inter.2 ⟨h1, h2⟩
  rw [h] at h3
  exact mem_singleton.1 h3

/-- The cap with vertex set `s`. -/
def capOf (s : Finset (Fin 120)) (hs : edgeGraph.IsNClique 4 s) : Cap :=
  ⟨s, SimpleGraph.mem_cliqueFinset_iff.2 hs⟩

theorem isNClique_witnesses :
    edgeGraph.IsNClique 4 ({0, 42, 79, 92} : Finset (Fin 120)) ∧
      edgeGraph.IsNClique 4 ({8, 27, 30, 33} : Finset (Fin 120)) ∧
      edgeGraph.IsNClique 4 ({8, 28, 31, 35} : Finset (Fin 120)) ∧
      edgeGraph.IsNClique 4 ({8, 27, 32, 34} : Finset (Fin 120)) := by
  decide +kernel

/-- **A symmetry of `P` that fixes every facet is the identity.** -/
theorem eq_one_of_forall_smul_cap {A : symmetryGroup} (h : ∀ c : Cap, A • c = c) : A = 1 := by
  have key : ∀ (t : Finset (Fin 120)) (ht : edgeGraph.IsNClique 4 t) (v : Fin 120),
      facet0 ∩ t = {v} → A • v = v := fun t ht v hv =>
    smul_eq_of_inter_eq (c := capOf facet0 isNClique_facet0) (d := capOf t ht) (h _) (h _) hv
  obtain ⟨h0, h30, h31, h32⟩ := isNClique_witnesses
  have e0 := key _ h0 0 (by decide)
  have e30 := key _ h30 30 (by decide)
  have e31 := key _ h31 31 (by decide)
  have e32 := key _ h32 32 (by decide)
  refine Subtype.ext (eq_of_eqOn_clique facet0_mem fun k hk => ?_)
  rw [← vertexR_smul]
  simp only [facet0, mem_insert, mem_singleton] at hk
  rcases hk with rfl | rfl | rfl | rfl
  · rw [e0]
    rfl
  · rw [e30]
    rfl
  · rw [e31]
    rfl
  · rw [e32]
    rfl

/-! ### Lemma 12 -/

variable {Sticker Piece : Type*} [Fintype Sticker] [Fintype Piece] [DecidableEq Piece]
  {M : RetainedModel Sticker Piece} {R : HypothesisR M} {F : HypothesisF M}

namespace HypothesisE

variable (E : HypothesisE R F)

/-- Conjugation by a geometric symmetry maps the legal group into itself: each primitive is a cap
rotation (F), its conjugate is the rotation of the image cap (E), and every cap rotation is legal
(Lemma 6). -/
theorem conj_mem (k : symmetryGroup) {g : Perm Sticker} (hg : g ∈ M.legalGroup) :
    E.geom k * g * (E.geom k)⁻¹ ∈ M.legalGroup := by
  have hle : M.legalGroup ≤ M.legalGroup.comap (MulAut.conj (E.geom k)).toMonoidHom := by
    refine M.legalGroup_le (fun c => ?_) (fun c => ?_)
    · rw [Subgroup.mem_comap, MulEquiv.coe_toMonoidHom, MulAut.conj_apply, F.H_eq,
        E.geom_capAction]
      exact F.capAction_mem _ _
    · rw [Subgroup.mem_comap, MulEquiv.coe_toMonoidHom, MulAut.conj_apply, F.T_eq,
        E.geom_capAction]
      exact F.capAction_mem _ _
  exact hle hg

/-- **Lemma 12, normalization.** The geometric action of `K` normalizes `G`. -/
theorem range_le_normalizer :
    E.geom.range ≤ Subgroup.normalizer (M.legalGroup : Set (Perm Sticker)) := by
  rintro _ ⟨k, rfl⟩
  refine Subgroup.mem_normalizer_iff.2 fun g => ⟨E.conj_mem k, fun hg => ?_⟩
  have h := E.conj_mem k⁻¹ hg
  simpa only [map_inv, inv_inv, mul_assoc, inv_mul_cancel_left, inv_mul_cancel, mul_one] using h

/-- **Lemma 12, normalization:** `κ(k) G κ(k)⁻¹ = G`. -/
theorem conj_legalGroup (k : symmetryGroup) :
    M.legalGroup.map (MulAut.conj (E.geom k)).toMonoidHom = M.legalGroup := by
  ext g
  constructor
  · rintro ⟨h, hh, rfl⟩
    exact E.conj_mem k hh
  · intro hg
    refine ⟨(E.geom k)⁻¹ * g * E.geom k, ?_, ?_⟩
    · have h := E.conj_mem k⁻¹ hg
      rwa [map_inv, inv_inv] at h
    · rw [MulEquiv.coe_toMonoidHom, MulAut.conj_apply]
      group

/-- A geometric symmetry whose action fixes every centre fixes every cap. -/
theorem smul_cap_eq_of_fixes_centers {k : symmetryGroup}
    (hk : ∀ s, R.orbit (M.piece s) = none → E.geom k s = s) (c : Cap) : k • c = c := by
  have h := hk _ (E.center c).prop
  rw [E.geom_center] at h
  exact E.center.injective (Subtype.ext h)

/-- **Lemma 12, trivial intersection:** `G ∩ κ(K) = 1`. -/
theorem legalGroup_inf_range : M.legalGroup ⊓ E.geom.range = ⊥ := by
  refine (Subgroup.eq_bot_iff_forall _).2 fun x hx => ?_
  obtain ⟨hG, k, rfl⟩ := hx
  have hfix := R.legalGroup_le_centerFixing hG
  rw [eq_one_of_forall_smul_cap (E.smul_cap_eq_of_fixes_centers hfix), map_one]

theorem geom_mem_normalizer (k : symmetryGroup) :
    E.geom k ∈ Subgroup.normalizer (M.legalGroup : Set (Perm Sticker)) :=
  E.range_le_normalizer ⟨k, rfl⟩

/-- The action of `K` on `G` by conjugation through `κ`. -/
noncomputable def conjAction : symmetryGroup →* MulAut M.legalGroup :=
  M.legalGroup.normalizerMonoidHom.comp (E.geom.codRestrict _ E.geom_mem_normalizer)

theorem coe_conjAction_apply (k : symmetryGroup) (g : M.legalGroup) :
    (E.conjAction k g : Perm Sticker) = E.geom k * g * (E.geom k)⁻¹ := rfl

/-- The homomorphism `(g, k) ↦ g κ(k)` from `G ⋊ K` to the sticker permutations. -/
noncomputable def semidirectHom : M.legalGroup ⋊[E.conjAction] symmetryGroup →* Perm Sticker :=
  SemidirectProduct.lift M.legalGroup.subtype E.geom fun k => by
    ext g s
    rfl

theorem semidirectHom_apply (x : M.legalGroup ⋊[E.conjAction] symmetryGroup) :
    E.semidirectHom x = (x.left : Perm Sticker) * E.geom x.right := rfl

theorem semidirectHom_injective : Function.Injective E.semidirectHom := by
  refine (injective_iff_map_eq_one _).2 fun x hx => ?_
  rw [semidirectHom_apply, mul_eq_one_iff_eq_inv] at hx
  have hg : E.geom x.right ∈ M.legalGroup := by
    rw [← inv_inv (E.geom x.right), ← hx]
    exact M.legalGroup.inv_mem x.left.prop
  have hmem : E.geom x.right ∈ M.legalGroup ⊓ E.geom.range := ⟨hg, x.right, rfl⟩
  rw [E.legalGroup_inf_range, Subgroup.mem_bot] at hmem
  have hr : x.right = 1 := E.geom_injective (hmem.trans (map_one _).symm)
  have hl : x.left = 1 := by
    refine Subtype.ext ?_
    rw [hx, hmem, inv_one]
    rfl
  exact SemidirectProduct.ext hl hr

theorem range_semidirectHom : E.semidirectHom.range = M.legalGroup ⊔ E.geom.range := by
  refine le_antisymm ?_ (sup_le ?_ ?_)
  · rintro _ ⟨x, rfl⟩
    exact Subgroup.mul_mem _ (Subgroup.mem_sup_left x.left.prop)
      (Subgroup.mem_sup_right ⟨x.right, rfl⟩)
  · intro g hg
    exact ⟨SemidirectProduct.inl ⟨g, hg⟩, by
      rw [semidirectHom_apply, SemidirectProduct.left_inl, SemidirectProduct.right_inl, map_one,
        mul_one]⟩
  · rintro _ ⟨k, rfl⟩
    exact ⟨SemidirectProduct.inr k, by
      rw [semidirectHom_apply, SemidirectProduct.left_inr, SemidirectProduct.right_inr,
        Subgroup.coe_one, one_mul]⟩

/-- **Lemma 12, the semidirect product:** `⟨G, κ(K)⟩ ≅ G ⋊ K`, through `(g, k) ↦ g κ(k)`. -/
noncomputable def semidirectEquiv :
    M.legalGroup ⋊[E.conjAction] symmetryGroup ≃* ↥(M.legalGroup ⊔ E.geom.range) :=
  (MonoidHom.ofInjective E.semidirectHom_injective).trans
    (MulEquiv.subgroupCongr E.range_semidirectHom)

theorem semidirectEquiv_apply (x : M.legalGroup ⋊[E.conjAction] symmetryGroup) :
    (E.semidirectEquiv x : Perm Sticker) = (x.left : Perm Sticker) * E.geom x.right := rfl

/-- **Lemma 12, the unique form.** Every element of `⟨G, κ(K)⟩` is `g κ(k)` for exactly one
`g ∈ G` and `k ∈ K`. -/
theorem existsUnique_mul {x : Perm Sticker} (hx : x ∈ M.legalGroup ⊔ E.geom.range) :
    ∃! p : M.legalGroup × symmetryGroup, (p.1 : Perm Sticker) * E.geom p.2 = x := by
  obtain ⟨y, hy⟩ := E.semidirectEquiv.surjective ⟨x, hx⟩
  refine ⟨(y.left, y.right), congrArg Subtype.val hy, fun p hp => ?_⟩
  have h : E.semidirectHom ⟨p.1, p.2⟩ = E.semidirectHom y :=
    hp.trans (congrArg Subtype.val hy).symm
  have := E.semidirectHom_injective h
  rw [← this]

/-- **Lemma 12.** Under R, F and E, the geometric action of `K` normalizes `G` and meets it
trivially, `(g, k) ↦ g κ(k)` is an isomorphism `G ⋊ K ≅ ⟨G, κ(K)⟩`, and every element of
`⟨G, κ(K)⟩` has a unique form `g κ(k)`. -/
theorem normalization :
    (∀ k, M.legalGroup.map (MulAut.conj (E.geom k)).toMonoidHom = M.legalGroup) ∧
      M.legalGroup ⊓ E.geom.range = ⊥ ∧
      (∃ e : M.legalGroup ⋊[E.conjAction] symmetryGroup ≃* ↥(M.legalGroup ⊔ E.geom.range),
        ∀ x, (e x : Perm Sticker) = (x.left : Perm Sticker) * E.geom x.right) ∧
      ∀ x ∈ M.legalGroup ⊔ E.geom.range,
        ∃! p : M.legalGroup × symmetryGroup, (p.1 : Perm Sticker) * E.geom p.2 = x :=
  ⟨E.conj_legalGroup, E.legalGroup_inf_range, ⟨E.semidirectEquiv, E.semidirectEquiv_apply⟩,
    fun _ hx => E.existsUnique_mul hx⟩

end HypothesisE

namespace HypothesisC

variable [DecidableEq Sticker] {Color : Type*} [Fintype Color] [DecidableEq Color]
  {c₀ : Sticker → Color}

set_option linter.unusedFintypeInType false in
omit [Fintype Piece] [DecidableEq Piece] [DecidableEq Sticker] [Fintype Color]
  [DecidableEq Color] in
theorem ncard_reachableColorings_le :
    (reachableColorings M.legalGroup c₀).ncard ≤ Nat.card M.legalGroup := by
  rw [reachableColorings, ← Set.image_univ, ← Set.ncard_univ]
  exact Set.ncard_image_le

/-- **The separation remark** (lines 1357 to 1360). Under R and C, the balanced-colour bound
alone separates the legal group from the geometric group:
`|K| = 14400 < 10^151851 < B ≤ |Ω| ≤ |G|`. -/
theorem card_symmetryGroup_lt (C : HypothesisC R c₀) :
    Nat.card symmetryGroup < Nat.card M.legalGroup := by
  rw [card_symmetryGroup]
  calc 14400 < 10 ^ 151851 :=
        lt_of_lt_of_le (by norm_num : 14400 < 10 ^ 5) (Nat.pow_le_pow_right (by norm_num)
          (by norm_num))
    _ < balancedColorCount := balancedColorCountGtPower
    _ ≤ (reachableColorings M.legalGroup c₀).ncard := C.balancedColorBound.2
    _ ≤ Nat.card M.legalGroup := ncard_reachableColorings_le

end HypothesisC

end ArchonFull600Theory
