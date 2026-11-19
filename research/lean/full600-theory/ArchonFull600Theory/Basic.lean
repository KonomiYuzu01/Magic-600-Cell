import Mathlib.Algebra.Order.Archimedean.Real.Basic
import Mathlib.Algebra.Ring.GeomSum
import Mathlib.Basic.Real.Basic
import Mathlib.Data.Finset.Powerset
import Mathlib.Data.Fintype.BigOperators
import Mathlib.Data.Set.Card
import Mathlib.Geometry.Convex.ConvexSpace.Defs
import Mathlib.GroupTheory.Commutator.Basic
import Mathlib.GroupTheory.Index
import Mathlib.GroupTheory.NoncommPiCoprod
import Mathlib.GroupTheory.Perm.Cycle.Concrete
import Mathlib.GroupTheory.Perm.DomMulAct
import Mathlib.GroupTheory.Perm.List
import Mathlib.GroupTheory.SpecificGroups.Alternating
import Mathlib.GroupTheory.SpecificGroups.Alternating.Centralizer
import Mathlib.GroupTheory.SpecificGroups.Dihedral
import Mathlib.Order.Interval.Finset.Nat
import Mathlib.Tactic.Linarith
import Mathlib.Tactic.Positivity
import Mathlib.Tactic.Ring

set_option linter.style.header false in
/-!
# Permutation core

Full sticker actions, supported alternating actions, and bounds on reachable colorings
for the Full 600-cell puzzle.
-/

namespace ArchonFull600Theory

open scoped BigOperators

/-- The one-based sticker labels used in the finite cycle examples. -/
abbrev StickerLabels (n : ℕ) := ↥(Finset.Icc 1 n)

theorem scopeSameRestriction :
    let one : StickerLabels 6 := ⟨1, by decide⟩
    let two : StickerLabels 6 := ⟨2, by decide⟩
    let three : StickerLabels 6 := ⟨3, by decide⟩
    let four : StickerLabels 6 := ⟨4, by decide⟩
    let five : StickerLabels 6 := ⟨5, by decide⟩
    let six : StickerLabels 6 := ⟨6, by decide⟩
    let u := [one, two, three].formPerm
    let v := [four, five, six].formPerm * u
    u ∈ alternatingGroup (StickerLabels 6) ∧ v ∈ alternatingGroup (StickerLabels 6) ∧
      Set.EqOn u v {one, two, three} ∧ u ≠ v := by
  intro one two three four five six u v
  refine ⟨by rw [Equiv.Perm.mem_alternatingGroup]; decide,
    by rw [Equiv.Perm.mem_alternatingGroup]; decide, ?_, by decide⟩
  intro x hx
  simp only [Set.mem_insert_iff, Set.mem_singleton_iff] at hx
  rcases hx with rfl | rfl | rfl <;> decide

theorem scopeVisibleDisjoint :
    let one : StickerLabels 7 := ⟨1, by decide⟩
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
      ¬ Commute a b ∧ b (a five) = seven ∧ a (b five) = six := by
  intro one two three four five six seven a b displayed
  refine ⟨by decide, by decide, by decide, ?_, by decide, by decide⟩
  rw [commute_iff_eq]
  decide

theorem disjointFullSupportsCommute {Slot : Type*} [Fintype Slot] [DecidableEq Slot]
    (P Q : Equiv.Perm Slot) (h : Disjoint P.support Q.support) :
    ⇑Q ∘ ⇑P = ⇑P ∘ ⇑Q := by
  have hc : Commute Q P := ((Equiv.Perm.disjoint_iff_disjoint_support).2 h).symm.commute
  have := congrArg (⇑) hc.eq
  simpa [Equiv.Perm.coe_mul] using this

/-- The graph of a permutation on its moved slots. -/
def nonfixedMap {Slot : Type*} (W : Equiv.Perm Slot) : Set (Slot × Slot) :=
  {uv | uv.2 = W uv.1 ∧ W uv.1 ≠ uv.1}

/-- The chronological word `S⁻¹ W S` acts at `x` by `S (W (S⁻¹ x))`. -/
theorem transportNonfixedMap {Slot : Type*} (G : Subgroup (Equiv.Perm Slot)) (W S : G) :
    nonfixedMap (S * W * S⁻¹).val =
      (fun uv : Slot × Slot => (S.val uv.1, S.val uv.2)) '' nonfixedMap W.val := by
  ext ⟨x, y⟩
  simp only [nonfixedMap, Subgroup.coe_mul, Subgroup.coe_inv, Equiv.Perm.coe_mul,
    Function.comp_apply, Set.mem_ofPred_eq, Set.mem_image, Prod.exists, Prod.mk.injEq]
  constructor
  · rintro ⟨hy, hne⟩
    refine ⟨(S.val)⁻¹ x, W.val ((S.val)⁻¹ x), ⟨rfl, ?_⟩, ?_, ?_⟩
    · intro h; apply hne; rw [h]; simp
    · simp
    · rw [hy]
  · rintro ⟨u, v, ⟨hv, hne⟩, rfl, rfl⟩
    subst hv
    refine ⟨by simp, ?_⟩
    simpa using hne

section Cycles

open Equiv

variable {α : Type*} [DecidableEq α]

theorem formPerm_three (x y z : α) : [x, y, z].formPerm = swap x y * swap y z := by
  simp [List.formPerm_cons_cons]

omit [DecidableEq α] in
theorem nodup_three {a b c : α} (h : [a, b, c].Nodup) : a ≠ b ∧ a ≠ c ∧ b ≠ c := by
  simp only [List.nodup_cons, List.mem_cons, not_or, List.not_mem_nil, List.nodup_nil,
    and_true] at h
  tauto

omit [DecidableEq α] in
theorem nodup_four {a b c d : α} (h : [a, b, c, d].Nodup) :
    a ≠ b ∧ a ≠ c ∧ a ≠ d ∧ b ≠ c ∧ b ≠ d ∧ c ≠ d := by
  simp only [List.nodup_cons, List.mem_cons, not_or, List.not_mem_nil, List.nodup_nil,
    and_true] at h
  tauto

/-- `(A B X)(A B Y)⁻¹ = (A Y X)`, with the right factor acting first. -/
theorem cycle_id_one {A B X Y : α} (hAB : A ≠ B) (hAX : A ≠ X) (hAY : A ≠ Y) (hBX : B ≠ X)
    (hBY : B ≠ Y) (hXY : X ≠ Y) :
    swap A B * swap B X * (swap A B * swap B Y)⁻¹ = swap A Y * swap Y X := by
  ext t
  simp only [mul_inv_rev, swap_inv, Perm.coe_mul, Function.comp_apply]
  by_cases h1 : t = A
  · subst h1; simp [swap_apply_of_ne_of_ne, *, Ne.symm]
  by_cases h2 : t = B
  · subst h2; simp [swap_apply_of_ne_of_ne, *, Ne.symm]
  by_cases h3 : t = X
  · subst h3; simp [swap_apply_of_ne_of_ne, *, Ne.symm]
  by_cases h4 : t = Y
  · subst h4; simp [swap_apply_of_ne_of_ne, *, Ne.symm]
  simp [swap_apply_of_ne_of_ne, *]

/-- `(A z x)(A x y) = (x y z)`, with the right factor acting first. -/
theorem cycle_id_two {A x y z : α} (hAx : A ≠ x) (hAy : A ≠ y) (hAz : A ≠ z) (hxy : x ≠ y)
    (hxz : x ≠ z) (hyz : y ≠ z) :
    swap A z * swap z x * (swap A x * swap x y) = swap x y * swap y z := by
  ext t
  simp only [Perm.coe_mul, Function.comp_apply]
  by_cases h1 : t = A
  · subst h1; simp [swap_apply_of_ne_of_ne, *]
  by_cases h2 : t = x
  · subst h2; simp [swap_apply_of_ne_of_ne, *, Ne.symm]
  by_cases h3 : t = y
  · subst h3; simp [swap_apply_of_ne_of_ne, *, Ne.symm]
  by_cases h4 : t = z
  · subst h4; simp [swap_apply_of_ne_of_ne, *, Ne.symm]
  simp [swap_apply_of_ne_of_ne, *]

theorem cycle_inv {x y z : α} (hxy : x ≠ y) (hxz : x ≠ z) (hyz : y ≠ z) :
    (swap x y * swap y z)⁻¹ = swap x z * swap z y := by
  ext t
  simp only [mul_inv_rev, swap_inv, Perm.coe_mul, Function.comp_apply]
  by_cases h1 : t = x
  · subst h1; simp [swap_apply_of_ne_of_ne, *]
  by_cases h2 : t = y
  · subst h2; simp [swap_apply_of_ne_of_ne, *, Ne.symm]
  by_cases h3 : t = z
  · subst h3; simp [swap_apply_of_ne_of_ne, *, Ne.symm]
  simp [swap_apply_of_ne_of_ne, *]

theorem cycle_rotate {x y z : α} (hxy : x ≠ y) (hxz : x ≠ z) (hyz : y ≠ z) :
    swap x y * swap y z = swap y z * swap z x := by
  ext t
  simp only [Perm.coe_mul, Function.comp_apply]
  by_cases h1 : t = x
  · subst h1; simp [swap_apply_of_ne_of_ne, *]
  by_cases h2 : t = y
  · subst h2; simp [swap_apply_of_ne_of_ne, *, Ne.symm]
  by_cases h3 : t = z
  · subst h3; simp [swap_apply_of_ne_of_ne, *, Ne.symm]
  simp [swap_apply_of_ne_of_ne, *]

/-- Every three-cycle is `swap a b * swap b c` for three distinct points. -/
theorem isThreeCycle_eq_swap_mul_swap [Fintype α] {τ : Perm α} (h : τ.IsThreeCycle) :
    ∃ a b c : α, a ≠ b ∧ a ≠ c ∧ b ≠ c ∧ τ = swap a b * swap b c := by
  obtain ⟨x, hx⟩ : τ.support.Nonempty := by
    rw [← Finset.card_pos, h.card_support]; norm_num
  have hxm : τ x ≠ x := Perm.mem_support.1 hx
  have hcyc : τ.cycleOf x = τ := h.isCycle.cycleOf_eq hxm
  have hlen : (τ.toList x).length = 3 := by
    rw [Perm.length_toList, hcyc, h.card_support]
  obtain ⟨a, b, c, hl⟩ := List.length_eq_three.1 hlen
  have hnd := Perm.nodup_toList τ x
  have hform := Perm.formPerm_toList τ x
  rw [hl] at hnd hform
  rw [hcyc, formPerm_three] at hform
  obtain ⟨h1, h2, h3⟩ := nodup_three hnd
  exact ⟨a, b, c, h1, h2, h3, hform.symm⟩

end Cycles

/-- A legal seed three-cycle, a relocation to the buffer pair and anchor, and setups
covering every nonbuffer destination. -/
structure PureControllerCertificate {Slot : Type*} [DecidableEq Slot]
    (G : Subgroup (Equiv.Perm Slot)) (O : Set Slot) where
  seedA : O
  seedB : O
  seedC : O
  bufferA : O
  bufferB : O
  anchor : O
  seedDistinct : [seedA, seedB, seedC].Nodup
  relocatedDistinct : [bufferA, bufferB, anchor].Nodup
  seed : G
  seed_eq : seed.val = [seedA.val, seedB.val, seedC.val].formPerm
  relocation : G
  relocation_eq :
    relocation.val seedA.val = bufferA.val ∧ relocation.val seedB.val = bufferB.val ∧
      relocation.val seedC.val = anchor.val
  coverage : ∀ X : O, X ≠ bufferA → X ≠ bufferB →
    ∃ S : G, S.val bufferA.val = bufferA.val ∧ S.val bufferB.val = bufferB.val ∧
      S.val anchor.val = X.val

section CertificateLemmas

open Equiv

namespace PureControllerCertificate

variable {Slot : Type*} [DecidableEq Slot] {G : Subgroup (Equiv.Perm Slot)} {O : Set Slot}

theorem conj_swap_mul_swap (g : Perm Slot) (a b c : Slot) :
    g * (swap a b * swap b c) * g⁻¹ = swap (g a) (g b) * swap (g b) (g c) := by
  rw [swap_apply_apply, swap_apply_apply]; group

/-- The relocated pure star `(A B X)` lies in `G` for every nonbuffer `X`. -/
theorem star_mem (cert : PureControllerCertificate G O) {X : Slot} (hX : X ∈ O)
    (hXA : X ≠ cert.bufferA.val) (hXB : X ≠ cert.bufferB.val) :
    swap cert.bufferA.val cert.bufferB.val * swap cert.bufferB.val X ∈ G := by
  obtain ⟨hA, hB, hC⟩ := cert.relocation_eq
  have htC : swap cert.bufferA.val cert.bufferB.val *
      swap cert.bufferB.val cert.anchor.val ∈ G := by
    have hmem := G.mul_mem (G.mul_mem cert.relocation.prop cert.seed.prop)
      (G.inv_mem cert.relocation.prop)
    rw [cert.seed_eq, formPerm_three, conj_swap_mul_swap, hA, hB, hC] at hmem
    exact hmem
  obtain ⟨S, hSA, hSB, hSC⟩ := cert.coverage ⟨X, hX⟩ (fun h => hXA (congrArg Subtype.val h))
    (fun h => hXB (congrArg Subtype.val h))
  have hmem := G.mul_mem (G.mul_mem S.prop htC) (G.inv_mem S.prop)
  rw [conj_swap_mul_swap, hSA, hSB, hSC] at hmem
  exact hmem

theorem buffers_ne (cert : PureControllerCertificate G O) :
    cert.bufferA.val ≠ cert.bufferB.val ∧ cert.bufferA.val ≠ cert.anchor.val ∧
      cert.bufferB.val ≠ cert.anchor.val := by
  obtain ⟨h1, h2, h3⟩ := nodup_three cert.relocatedDistinct
  exact ⟨fun e => h1 (Subtype.ext e), fun e => h2 (Subtype.ext e), fun e => h3 (Subtype.ext e)⟩

/-- Every three-cycle through the buffer `A` lies in `G`. -/
theorem cycle_through_buffer_mem (cert : PureControllerCertificate G O) {u v : Slot}
    (hu : u ∈ O) (hv : v ∈ O) (huA : u ≠ cert.bufferA.val) (hvA : v ≠ cert.bufferA.val)
    (huv : u ≠ v) :
    swap cert.bufferA.val u * swap u v ∈ G := by
  obtain ⟨hAB, -, -⟩ := cert.buffers_ne
  set A := cert.bufferA.val
  set B := cert.bufferB.val
  by_cases huB : u = B
  · subst huB
    exact cert.star_mem hv hvA (Ne.symm huv)
  by_cases hvB : v = B
  · subst hvB
    have h := G.inv_mem (cert.star_mem hu huA huB)
    rwa [cycle_inv hAB (Ne.symm huA) (Ne.symm huB)] at h
  have h := G.mul_mem (cert.star_mem hv hvA hvB) (G.inv_mem (cert.star_mem hu huA huB))
  rwa [cycle_id_one hAB (Ne.symm hvA) (Ne.symm huA) (Ne.symm hvB) (Ne.symm huB)
    (Ne.symm huv)] at h

/-- Every three-cycle on the orbit lies in `G`. -/
theorem three_cycle_mem (cert : PureControllerCertificate G O) {x y z : Slot}
    (hx : x ∈ O) (hy : y ∈ O) (hz : z ∈ O) (hxy : x ≠ y) (hxz : x ≠ z) (hyz : y ≠ z) :
    swap x y * swap y z ∈ G := by
  set A := cert.bufferA.val with hAdef
  have hA : A ∈ O := cert.bufferA.prop
  by_cases hxA : x = A
  · rw [hxA] at hxy hxz ⊢
    exact cert.cycle_through_buffer_mem hy hz (Ne.symm hxy) (Ne.symm hxz) hyz
  by_cases hyA : y = A
  · rw [cycle_rotate hxy hxz hyz, hyA]
    rw [hyA] at hyz hxy
    exact cert.cycle_through_buffer_mem hz hx (Ne.symm hyz) hxA (Ne.symm hxz)
  by_cases hzA : z = A
  · rw [cycle_rotate hxy hxz hyz, cycle_rotate hyz (Ne.symm hxy) (Ne.symm hxz), hzA]
    rw [hzA] at hyz hxz
    exact cert.cycle_through_buffer_mem hx hy hxA hyA hxy
  rw [← cycle_id_two (Ne.symm hxA) (Ne.symm hyA) (Ne.symm hzA) hxy hxz hyz]
  exact G.mul_mem (cert.cycle_through_buffer_mem hz hx hzA hxA (Ne.symm hxz))
    (cert.cycle_through_buffer_mem hx hy hxA hyA hxy)

/-- The even permutations of a certified finite orbit, extended by the identity, are legal. -/
theorem ofSubtype_mem {T : Finset Slot} (cert : PureControllerCertificate G (T : Set Slot))
    {σ : Perm T} (hσ : σ ∈ alternatingGroup T) : Perm.ofSubtype σ ∈ G := by
  have hle : alternatingGroup T ≤ G.comap (Perm.ofSubtype : Perm T →* Perm Slot) := by
    rw [← Perm.closure_three_cycles_eq_alternating, Subgroup.closure_le]
    intro τ hτ
    obtain ⟨a, b, c, hab, hac, hbc, rfl⟩ := isThreeCycle_eq_swap_mul_swap hτ
    simp only [SetLike.mem_coe, Subgroup.mem_comap, map_mul, Perm.ofSubtype_swap_eq]
    exact cert.three_cycle_mem (Finset.mem_coe.2 a.prop) (Finset.mem_coe.2 b.prop)
      (Finset.mem_coe.2 c.prop) (fun e => hab (Subtype.ext e)) (fun e => hac (Subtype.ext e))
      (fun e => hbc (Subtype.ext e))
  exact hle hσ

end PureControllerCertificate

section Supported
variable {Slot : Type*} [Fintype Slot] [DecidableEq Slot] {ι : Type*} [Fintype ι] [DecidableEq ι]

set_option linter.unusedDecidableInType false in
set_option linter.unusedFintypeInType false in
/-- Products of even permutations on pairwise-disjoint legal orbits, extended by the identity. -/
theorem exists_supported_alternating (G : Subgroup (Perm Slot)) (O : ι → Finset Slot)
    (hDisjoint : Pairwise fun i j => Disjoint (O i) (O j))
    (hmem : ∀ i (σ : Perm (O i)), σ ∈ alternatingGroup (O i) → Perm.ofSubtype σ ∈ G) :
    ∃ ρ : (∀ i, alternatingGroup ↥(O i)) →* G, Function.Injective ρ ∧
      (∀ π i (s : O i), (ρ π).val s.val = ((π i).val s).val) ∧
      (∀ π, ((ρ π).val.support : Set Slot) ⊆ ⋃ i, (O i : Set Slot)) := by
  let φ : ∀ i, alternatingGroup ↥(O i) →* Perm Slot := fun i =>
    (Perm.ofSubtype : Perm ↥(O i) →* Perm Slot).comp (alternatingGroup ↥(O i)).subtype
  have hφ_out : ∀ i (x : alternatingGroup ↥(O i)) (t : Slot), t ∉ O i → φ i x t = t :=
    fun i x t ht => Perm.ofSubtype_apply_of_not_mem x.val ht
  have hcomm : Pairwise fun i j => ∀ x y, Commute (φ i x) (φ j y) := by
    intro i j hij x y
    apply Perm.Disjoint.commute
    intro t
    by_cases hti : t ∈ O i
    · exact Or.inr (hφ_out j y t (Finset.disjoint_left.1 (hDisjoint hij) hti))
    · exact Or.inl (hφ_out i x t hti)
  let f := MonoidHom.noncommPiCoprod φ hcomm
  have hf : ∀ π, f π = Finset.univ.noncommProd (fun i => φ i (π i))
      (fun _ _ _ _ h => hcomm h _ _) := fun π => rfl
  -- every factor fixing `t` makes the product fix `t`
  have hfix : ∀ π (t : Slot), (∀ i, φ i (π i) t = t) → f π t = t := by
    intro π t ht
    have hmemStab : f π ∈ MulAction.stabilizer (Perm Slot) t := by
      rw [hf]
      exact Subgroup.noncommProd_mem _ _ fun i _ => by
        rw [MulAction.mem_stabilizer_iff, Perm.smul_def]; exact ht i
    rwa [MulAction.mem_stabilizer_iff, Perm.smul_def] at hmemStab
  have hout : ∀ π (t : Slot), (∀ i, t ∉ O i) → f π t = t :=
    fun π t ht => hfix π t fun i => hφ_out i _ t (ht i)
  have hact : ∀ π i (s : O i), f π s.val = ((π i).val s).val := by
    intro π i s
    have hsplit : π = Pi.mulSingle i (π i) * Function.update π i 1 := by
      funext j
      by_cases hj : j = i
      · subst hj; simp
      · simp [Pi.mulSingle_eq_of_ne hj, Function.update_of_ne hj]
    have hrest : f (Function.update π i 1) s.val = s.val := by
      apply hfix
      intro j
      by_cases hj : j = i
      · subst hj; simp
      · exact hφ_out j _ _ (Finset.disjoint_left.1 (hDisjoint (Ne.symm hj)) s.prop)
    rw [show f π = f (Pi.mulSingle i (π i)) * f (Function.update π i 1) by
      rw [← map_mul, ← hsplit], Perm.mul_apply, hrest, MonoidHom.noncommPiCoprod_mulSingle]
    exact Perm.ofSubtype_apply_coe _ s
  have hG : ∀ π, f π ∈ G := by
    intro π
    rw [hf]
    exact Subgroup.noncommProd_mem _ _ fun i _ => hmem i _ (π i).prop
  refine ⟨f.codRestrict G hG, ?_, fun π i s => hact π i s, ?_⟩
  · intro π π' h
    have h' : ∀ i (s : O i), ((π i).val s).val = ((π' i).val s).val := by
      intro i s
      rw [← hact, ← hact]
      exact congrArg (fun g : G => g.val s.val) h
    funext i
    exact Subtype.ext (Equiv.ext fun s => Subtype.ext (h' i s))
  · intro π t ht
    have ht' : f π t ≠ t := Perm.mem_support.1 (Finset.mem_coe.1 ht)
    by_contra hnot
    simp only [Set.mem_iUnion, Finset.mem_coe, not_exists] at hnot
    exact ht' (hout π t hnot)

end Supported

section Counting

set_option linter.unusedDecidableInType false in
/-- Orbit-stabilizer for recolouring by precomposition. -/
theorem ncard_range_comp_perm {α β : Type*} [Fintype α] [DecidableEq α] [Fintype β]
    [DecidableEq β] (c : α → β) :
    (Set.range fun σ : Perm α => c ∘ σ).ncard * ∏ b, (Fintype.card {a // c a = b}).factorial =
      (Fintype.card α).factorial := by
  have horb : (Set.range fun σ : Perm α => c ∘ σ) = MulAction.orbit (Perm α)ᵈᵐᵃ c := by
    ext x
    simp only [Set.mem_range, MulAction.mem_orbit_iff]
    constructor
    · rintro ⟨σ, rfl⟩; exact ⟨DomMulAct.mk σ, rfl⟩
    · rintro ⟨g, rfl⟩; exact ⟨DomMulAct.mk.symm g, rfl⟩
  have hstab : Nat.card (MulAction.stabilizer (Perm α)ᵈᵐᵃ c) =
      ∏ b, (Fintype.card {a // c a = b}).factorial := by
    rw [← DomMulAct.stabilizer_card c, ← Nat.card_eq_fintype_card]
    exact Nat.card_congr (Equiv.subtypeEquiv DomMulAct.mk.symm
      fun g => DomMulAct.mem_stabilizer_iff)
  rw [horb, ← MulAction.index_stabilizer, ← hstab, mul_comm,
    Subgroup.card_mul_index, Nat.card_congr (DomMulAct.mk (M := Perm α)).symm,
    Nat.card_eq_fintype_card, Fintype.card_perm]

theorem comp_swap_of_eq {α β : Type*} [DecidableEq α] (c : α → β) {a b : α}
    (h : c a = c b) : c ∘ swap a b = c := by
  funext x
  simp only [Function.comp_apply]
  by_cases hxa : x = a
  · subst hxa; rw [swap_apply_left, h]
  by_cases hxb : x = b
  · subst hxb; rw [swap_apply_right, h]
  rw [swap_apply_of_ne_of_ne hxa hxb]

/-- With a repeated colour, even permutations reach every recolouring. -/
theorem range_alternating_eq {α β : Type*} [Fintype α] [DecidableEq α] (c : α → β) {a b : α}
    (hab : a ≠ b) (h : c a = c b) :
    (Set.range fun σ : alternatingGroup α => c ∘ σ.val.symm) =
      Set.range fun σ : Perm α => c ∘ σ := by
  ext x
  constructor
  · rintro ⟨σ, rfl⟩; exact ⟨σ.val.symm, rfl⟩
  · rintro ⟨τ, rfl⟩
    rcases Int.units_eq_one_or (Perm.sign τ) with hτ | hτ
    · refine ⟨⟨τ⁻¹, by rw [Perm.mem_alternatingGroup, Perm.sign_inv, hτ]⟩, ?_⟩
      rfl
    · refine ⟨⟨(swap a b * τ)⁻¹, ?_⟩, ?_⟩
      · rw [Perm.mem_alternatingGroup, Perm.sign_inv, Perm.sign_mul, Perm.sign_swap hab, hτ]
        decide
      · change c ∘ ⇑(swap a b * τ)⁻¹.symm = c ∘ τ
        rw [show (swap a b * τ)⁻¹.symm = swap a b * τ from rfl, Perm.coe_mul,
          ← Function.comp_assoc, comp_swap_of_eq c h]

/-- The number of recolourings reached by the even permutations of a balanced orbit. -/
theorem ncard_range_alternating {α β : Type*} [Fintype α] [DecidableEq α] [Fintype β]
    [DecidableEq β] [Nonempty β] (c : α → β) (m : ℕ) (hm : 2 ≤ m)
    (hfib : ∀ b, Fintype.card {a // c a = b} = m) :
    (Set.range fun σ : alternatingGroup α => c ∘ σ.val.symm).ncard =
      (Fintype.card α).factorial / m.factorial ^ Fintype.card β := by
  obtain ⟨b₀⟩ := ‹Nonempty β›
  have h1 : 1 < Fintype.card {a // c a = b₀} := by rw [hfib]; omega
  obtain ⟨x, y, hxy⟩ := Fintype.exists_pair_of_one_lt_card h1
  rw [range_alternating_eq c (a := x.val) (b := y.val) (fun e => hxy (Subtype.ext e))
    (x.prop.trans y.prop.symm)]
  have key := ncard_range_comp_perm c
  simp only [hfib, Finset.prod_const, Finset.card_univ] at key
  rw [← key, Nat.mul_div_cancel _ (pow_pos (Nat.factorial_pos m) _)]

end Counting

section Assembly
variable {Slot Color ι : Type*} [Fintype Slot] [DecidableEq Slot] [Fintype Color]
  [DecidableEq Color] [Fintype ι] [DecidableEq ι]

omit [Fintype Slot] [DecidableEq Slot] [Fintype Color] in
theorem card_fiber_eq_filter (T : Finset Slot) (c₀ : Slot → Color) (b : Color) :
    Fintype.card {a : T // c₀ a.val = b} = (T.filter fun s => c₀ s = b).card := by
  rw [← Fintype.card_coe (T.filter fun s => c₀ s = b)]
  exact Fintype.card_congr
    { toFun := fun a => ⟨a.val.val, Finset.mem_filter.2 ⟨a.val.prop, a.prop⟩⟩
      invFun := fun x => ⟨⟨x.val, (Finset.mem_filter.1 x.prop).1⟩, (Finset.mem_filter.1 x.prop).2⟩
      left_inv := fun _ => rfl
      right_inv := fun _ => rfl }

set_option linter.unusedDecidableInType false in
set_option linter.unusedFintypeInType false in
/-- The supported colourings number at least the product of the per-orbit counts. -/
theorem supported_count_ge (G : Subgroup (Perm Slot)) (O : ι → Finset Slot)
    (hDisjoint : Pairwise fun i j => Disjoint (O i) (O j))
    (hmem : ∀ i (σ : Perm (O i)), σ ∈ alternatingGroup (O i) → Perm.ofSubtype σ ∈ G)
    (c₀ : Slot → Color) [Nonempty Color] (m : ι → ℕ) (hm : ∀ i, 2 ≤ m i)
    (hCopies : ∀ i color, ((O i).filter fun s => c₀ s = color).card = m i) :
    ∏ i, (O i).card.factorial / (m i).factorial ^ Fintype.card Color ≤
      {x : Slot → Color |
        ∃ g : G, (∀ s, (∀ i, s ∉ O i) → g.val s = s) ∧ x = c₀ ∘ g.val.symm}.ncard := by
  obtain ⟨ρ, -, hact, hsupp⟩ := exists_supported_alternating G O hDisjoint hmem
  let T : ∀ i, Set (O i → Color) := fun i =>
    Set.range fun σ : alternatingGroup (O i) => (fun s : O i => c₀ s.val) ∘ σ.val.symm
  have hT : ∀ i, (T i).ncard = (O i).card.factorial / (m i).factorial ^ Fintype.card Color := by
    intro i
    rw [ncard_range_alternating _ (m i) (hm i), Fintype.card_coe]
    intro b
    rw [← hCopies i b]
    exact card_fiber_eq_filter (O i) c₀ b
  have hinv : ∀ π i (s : O i), (ρ π).val.symm s.val = ((π i).val.symm s).val := by
    intro π i s
    have h := hact π⁻¹ i s
    rw [map_inv] at h
    exact h
  let Φ : (∀ i, alternatingGroup ↥(O i)) → Slot → Color := fun π => c₀ ∘ (ρ π).val.symm
  let R : (Slot → Color) → (∀ i, O i → Color) := fun x i s => x s.val
  have hRΦ : Set.range (R ∘ Φ) = Set.pi Set.univ T := by
    ext y
    simp only [Set.mem_range, Set.mem_pi, Set.mem_univ, true_implies, Function.comp_apply]
    constructor
    · rintro ⟨π, rfl⟩ i
      refine ⟨π i, ?_⟩
      funext s
      simp only [Function.comp_apply, R, Φ, hinv]
    · intro hy
      choose σ hσ using hy
      refine ⟨σ, ?_⟩
      funext i s
      rw [← hσ i]
      simp only [Function.comp_apply, R, Φ, hinv]
  have hfix : ∀ π, ∀ s, (∀ i, s ∉ O i) → (ρ π).val s = s := by
    intro π s hs
    by_contra h
    have hmem := hsupp π (Finset.mem_coe.2 (Perm.mem_support.2 h))
    simp only [Set.mem_iUnion, Finset.mem_coe] at hmem
    obtain ⟨i, hi⟩ := hmem
    exact hs i hi
  calc ∏ i, (O i).card.factorial / (m i).factorial ^ Fintype.card Color
      = ∏ i, (T i).ncard := by simp only [hT]
    _ = (Set.pi Set.univ T).ncard := by
        rw [← Nat.card_coe_set_eq, Nat.card_congr (Equiv.Set.univPi T), Nat.card_pi]
        simp only [Nat.card_coe_set_eq]
    _ = (R '' Set.range Φ).ncard := by rw [← Set.range_comp, hRΦ]
    _ ≤ (Set.range Φ).ncard := Set.ncard_image_le (Set.toFinite _)
    _ ≤ _ := by
        refine Set.ncard_le_ncard ?_ (Set.toFinite _)
        rintro x ⟨π, rfl⟩
        exact ⟨ρ π, hfix π, rfl⟩

end Assembly

end CertificateLemmas

/-- Orbit numbers with pure-controller certificates in the balanced-color bound. -/
def selectedOrbitIds : Finset ℕ := {1, 4, 5, 7, 18, 19, 27, 31, 33}

theorem prod_selectedOrbitIds (a b c : ℕ) :
    ∏ n ∈ selectedOrbitIds, (if n = 1 then a else if n = 33 then b else c) = a * b * c ^ 7 := by
  simp [selectedOrbitIds, Finset.prod_insert]
  ring

section

variable {Slot : Type*} [Fintype Slot] [DecidableEq Slot]
variable (G : Subgroup (Equiv.Perm Slot)) (O : ↥selectedOrbitIds → Finset Slot)
variable (hDisjoint : Pairwise fun i j => Disjoint (O i) (O j))
  (hPreserved : ∀ i (g : G) (s : Slot), g.val s ∈ O i ↔ s ∈ O i)
  (hDegree : ∀ i, (O i).card = if i.val = 1 then 3600 else if i.val = 33 then 2400 else 7200)
  (certificates : ∀ i, PureControllerCertificate G (O i : Set Slot))

set_option linter.unusedSectionVars false in
include hDisjoint hPreserved hDegree certificates in
/-- The product acts as each prescribed alternating permutation on its orbit and fixes
every slot outside the selected orbits. -/
theorem supportedAlternatingActions :
    ∃ ρ : (∀ i, alternatingGroup ↥(O i)) →* G,
      Function.Injective ρ ∧
      (∀ π i (s : O i), (ρ π).val s.val = ((π i).val s).val) ∧
      (∀ π, ((ρ π).val.support : Set Slot) ⊆ ⋃ i, (O i : Set Slot)) :=
  exists_supported_alternating G O hDisjoint fun _ _ hσ => (certificates _).ofSubtype_mem hσ

variable {Color : Type*} (c₀ : Slot → Color)

/-- The fixed-frame colorings obtained from the solved coloring by the legal action. -/
def reachableColorings : Set (Slot → Color) :=
  Set.range fun g : G => c₀ ∘ g.val.symm

/-- The product of the nine balanced-color multinomial counts. -/
def balancedColorCount : ℕ :=
  (Nat.factorial 3600 / Nat.factorial 6 ^ 600) *
    (Nat.factorial 2400 / Nat.factorial 4 ^ 600) *
    (Nat.factorial 7200 / Nat.factorial 12 ^ 600) ^ 7

variable [Fintype Color] [DecidableEq Color]
variable (hPalette : Fintype.card Color = 600)
  (hCopies : ∀ i (color : Color), ((O i).filter (fun s => c₀ s = color)).card =
    if i.val = 1 then 6 else if i.val = 33 then 4 else 12)

set_option linter.unusedSectionVars false in
set_option linter.unusedFintypeInType false in
include hDisjoint hPreserved hDegree certificates hPalette hCopies in
/-- Each supported witness preserves the solved coloring off the selected orbits at
its macro boundary. -/
theorem balancedColorLowerBound :
    let supportedColorings : Set (Slot → Color) :=
      {x | ∃ g : G, (∀ s, (∀ i, s ∉ O i) → g.val s = s) ∧ x = c₀ ∘ g.val.symm}
    balancedColorCount ≤ supportedColorings.ncard ∧
      balancedColorCount ≤ (reachableColorings G c₀).ncard := by
  intro supportedColorings
  have : Nonempty Color := Fintype.card_pos_iff.1 (by rw [hPalette]; norm_num)
  have hge := supported_count_ge G O hDisjoint
    (fun _ _ hσ => (certificates _).ofSubtype_mem hσ) c₀
    (fun i => if i.val = 1 then 6 else if i.val = 33 then 4 else 12)
    (fun i => by split_ifs <;> norm_num) hCopies
  have hterm : ∀ i : ↥selectedOrbitIds,
      (O i).card.factorial /
          ((fun i : ↥selectedOrbitIds => if i.val = 1 then 6 else if i.val = 33 then 4 else 12)
            i).factorial ^ Fintype.card Color =
        (fun n : ℕ => if n = 1 then Nat.factorial 3600 / Nat.factorial 6 ^ 600
          else if n = 33 then Nat.factorial 2400 / Nat.factorial 4 ^ 600
          else Nat.factorial 7200 / Nat.factorial 12 ^ 600) i.val := by
    intro i
    rw [hDegree i, hPalette]
    dsimp only
    split_ifs <;> rfl
  have hprod : balancedColorCount = ∏ i : ↥selectedOrbitIds, (O i).card.factorial /
      ((fun i : ↥selectedOrbitIds => if i.val = 1 then 6 else if i.val = 33 then 4 else 12)
        i).factorial ^ Fintype.card Color := by
    rw [Finset.prod_congr rfl fun i _ => hterm i]
    exact (prod_selectedOrbitIds (Nat.factorial 3600 / Nat.factorial 6 ^ 600)
      (Nat.factorial 2400 / Nat.factorial 4 ^ 600)
      (Nat.factorial 7200 / Nat.factorial 12 ^ 600)).symm.trans
      (Finset.prod_coe_sort selectedOrbitIds _).symm
  have hsub : supportedColorings ⊆ reachableColorings G c₀ := by
    rintro x ⟨g, -, rfl⟩
    exact ⟨g, rfl⟩
  have hfirst : balancedColorCount ≤ supportedColorings.ncard := hprod ▸ hge
  exact ⟨hfirst, hfirst.trans (Set.ncard_le_ncard hsub (Set.toFinite _))⟩

variable {Letter : Type*}

/-- Starting from the identity, each successive letter acts after the earlier letters. -/
def wordPermutation {legal : Subgroup (Equiv.Perm Slot)} (moves : Letter → legal)
    (w : List Letter) : legal :=
  w.foldl (fun g a => moves a * g) 1

/-- The reachable coloring at the end of a word, with the word read from left to right. -/
def wordColoring (moves : Letter → G) (w : List Letter) : reachableColorings G c₀ :=
  let g := wordPermutation moves w
  ⟨c₀ ∘ g.val.symm, ⟨g, rfl⟩⟩

/-- The least word length attaining a reachable coloring from solved. -/
noncomputable def wordDistance (moves : Letter → G)
    (hRealizes : Function.Surjective (wordPermutation moves))
    (x : reachableColorings G c₀) : ℕ := by
  classical
  refine Nat.find (p := fun n => ∃ w : List Letter, w.length = n ∧ wordColoring G c₀ moves w = x) ?_
  obtain ⟨g, hg⟩ := x.property
  obtain ⟨w, hw⟩ := hRealizes g
  refine ⟨w.length, w, rfl, Subtype.ext ?_⟩
  change c₀ ∘ (wordPermutation moves w).val.symm = x.val
  simpa only [hw] using hg

omit [Fintype Slot] [DecidableEq Slot] [Fintype Color] [DecidableEq Color] in
/-- A word of length `wordDistance x` reaches `x`. -/
theorem wordDistance_spec (moves : Letter → G)
    (hRealizes : Function.Surjective (wordPermutation moves)) (x : reachableColorings G c₀) :
    ∃ w : List Letter, w.length = wordDistance G c₀ moves hRealizes x ∧
      wordColoring G c₀ moves w = x := by
  classical
  unfold wordDistance
  exact @Nat.find_spec (fun n => ∃ w : List Letter, w.length = n ∧
    wordColoring G c₀ moves w = x) _ _

omit [Fintype Slot] [DecidableEq Slot] [Fintype Color] [DecidableEq Color] in
set_option linter.unusedDecidableInType false in
set_option linter.unusedFintypeInType false in
theorem wordBallCount [Fintype Letter] (moves : Letter → G)
    (hRealizes : Function.Surjective (wordPermutation moves))
    (hq : 2 ≤ Fintype.card Letter) (r : ℕ) :
    let q := Fintype.card Letter
    ({x : reachableColorings G c₀ | wordDistance G c₀ moves hRealizes x ≤ r}).ncard ≤
      (∑ j ∈ Finset.range (r + 1), q ^ j) ∧
      (∑ j ∈ Finset.range (r + 1), q ^ j) = (q ^ (r + 1) - 1) / (q - 1) := by
  intro q
  refine ⟨?_, Nat.geomSum_eq hq (r + 1)⟩
  let f : (Σ j : Fin (r + 1), List.Vector Letter j) → reachableColorings G c₀ :=
    fun w => wordColoring G c₀ moves w.2.toList
  have hsub : {x : reachableColorings G c₀ | wordDistance G c₀ moves hRealizes x ≤ r} ⊆
      Set.range f := by
    intro x hx
    obtain ⟨w, hlen, hw⟩ := wordDistance_spec G c₀ moves hRealizes x
    exact ⟨⟨⟨w.length, Nat.lt_succ_of_le (hlen ▸ hx)⟩, ⟨w, rfl⟩⟩, hw⟩
  calc _ ≤ (Set.range f).ncard := Set.ncard_le_ncard hsub (Set.toFinite _)
    _ ≤ Nat.card (Σ j : Fin (r + 1), List.Vector Letter j) := by
        rw [← Set.image_univ, ← Set.ncard_univ]
        exact Set.ncard_image_le (Set.toFinite _)
    _ = ∑ j ∈ Finset.range (r + 1), q ^ j := by
        rw [Nat.card_sigma, Fin.sum_univ_eq_sum_range (fun j => Nat.card (List.Vector Letter j))]
        simp only [Nat.card_eq_fintype_card, card_vector, q]

theorem balancedColorCountGtPower : 10 ^ 151851 < balancedColorCount := by
  decide +kernel

theorem radius46647Comparison : 1800 ^ 46648 - 1 < 1799 * balancedColorCount := by
  decide +kernel

theorem radius46648CrossCheck : 1799 * balancedColorCount ≤ 1800 ^ 46649 - 1 := by
  decide +kernel

theorem thousandLetterSupportComparison : 1800 ^ 1000 * 10 ^ 148595 < balancedColorCount := by
  decide +kernel

/-- Each of the 600 caps supplies three unit-cost labels for `H`, `T`, and `T⁻¹`. -/
def capLetterPermutation {legal : Subgroup (Equiv.Perm Slot)}
    (H T : Fin 600 → legal) (letter : Fin 600 × Fin 3) : legal :=
  if letter.2 = 0 then H letter.1 else if letter.2 = 1 then T letter.1 else (T letter.1)⁻¹

variable (H T : Fin 600 → G)
variable (hRealizes : Function.Surjective (wordPermutation (capLetterPermutation H T)))

set_option linter.unusedFintypeInType false in
include hDisjoint hPreserved hDegree certificates hPalette hCopies in
theorem wordDistanceLowerBound :
    ∃ x : reachableColorings G c₀,
      46648 ≤ wordDistance G c₀ (capLetterPermutation H T) hRealizes x := by
  by_contra hcon
  simp only [not_exists, not_le] at hcon
  have hq : Fintype.card (Fin 600 × Fin 3) = 1800 := by simp
  obtain ⟨hball, hsum⟩ :=
    wordBallCount G c₀ (capLetterPermutation H T) hRealizes (by rw [hq]; norm_num) 46647
  have hall : {x : reachableColorings G c₀ |
      wordDistance G c₀ (capLetterPermutation H T) hRealizes x ≤ 46647} = Set.univ :=
    Set.eq_univ_of_forall fun x => Nat.le_of_lt_succ (hcon x)
  rw [hall, Set.ncard_univ, hsum, hq, show (46647 : ℕ) + 1 = 46648 from rfl,
    show (1800 : ℕ) - 1 = 1799 from rfl] at hball
  have hB := (balancedColorLowerBound G O hDisjoint hPreserved hDegree certificates c₀ hPalette
    hCopies).2
  rw [← Nat.card_coe_set_eq] at hB
  have hlt : (1800 ^ 46648 - 1) / 1799 < balancedColorCount :=
    (Nat.div_lt_iff_lt_mul (by norm_num)).2 (by rw [mul_comm]; exact radius46647Comparison)
  exact absurd (hB.trans hball) (not_le.2 hlt)

/-- The maximum absolute difference of the two probability laws on a finite event. -/
noncomputable def totalVariation {X : Type*} [Finite X]
    (μ ν : Convexity.StdSimplex ℝ X) : ℝ := by
  classical
  letI := Fintype.ofFinite X
  let events := (Finset.univ : Finset X).powerset
  exact events.sup' (Finset.powerset_nonempty _) fun E =>
    |(∑ x ∈ E, μ.weights x) - (∑ x ∈ E, ν.weights x)|

theorem real_gap (e n K : ℕ) (hK : 0 < K) (h : e * K < n) :
    1 - (K : ℝ)⁻¹ < |1 - (e : ℝ) / n| := by
  have hn : (0 : ℝ) < n := by exact_mod_cast lt_of_le_of_lt (Nat.zero_le _) h
  have hK' : (0 : ℝ) < K := by exact_mod_cast hK
  have hK1 : (1 : ℝ) ≤ K := by exact_mod_cast hK
  have h' : (e : ℝ) * K < n := by exact_mod_cast h
  have hlt : (e : ℝ) / n < (K : ℝ)⁻¹ := by
    rw [div_lt_iff₀ hn, ← div_eq_inv_mul, lt_div_iff₀ hK']
    exact h'
  have hinv : (K : ℝ)⁻¹ ≤ 1 := inv_le_one_of_one_le₀ hK1
  rw [abs_of_nonneg (by linarith)]
  linarith

theorem le_totalVariation {X : Type*} [Finite X] (μ ν : Convexity.StdSimplex ℝ X)
    (E : Finset X) :
    |(∑ x ∈ E, μ.weights x) - (∑ x ∈ E, ν.weights x)| ≤ totalVariation μ ν := by
  classical
  let _ := Fintype.ofFinite X
  unfold totalVariation
  exact Finset.le_sup' (fun E : Finset X => |(∑ x ∈ E, μ.weights x) - (∑ x ∈ E, ν.weights x)|)
    (Finset.mem_powerset.2 (Finset.subset_univ E))

/-- A law carried by at most `e` points is far from the uniform law on `n` points. -/
theorem totalVariation_gt_of_small_support {X : Type*} [Finite X]
    (μ ν : Convexity.StdSimplex ℝ X) (n K : ℕ) (hK : 0 < K)
    (hUniform : ∀ x, ν.weights x = 1 / (n : ℝ)) (e : ℕ) (he : μ.weights.support.card ≤ e)
    (h : e * K < n) :
    1 - (K : ℝ)⁻¹ < totalVariation μ ν := by
  set E := μ.weights.support
  have hμ : ∑ x ∈ E, μ.weights x = 1 := μ.total
  have hν : ∑ x ∈ E, ν.weights x = (E.card : ℝ) / n := by
    simp only [hUniform, Finset.sum_const, nsmul_eq_mul]
    ring
  have hgap := real_gap E.card n K hK (lt_of_le_of_lt (Nat.mul_le_mul_right K he) h)
  calc 1 - (K : ℝ)⁻¹ < |1 - (E.card : ℝ) / n| := hgap
    _ = |(∑ x ∈ E, μ.weights x) - (∑ x ∈ E, ν.weights x)| := by rw [hμ, hν]
    _ ≤ totalVariation μ ν := le_totalVariation μ ν E

set_option linter.unusedSectionVars false in
set_option linter.unusedFintypeInType false in
include hDisjoint hPreserved hDegree certificates hPalette hCopies hRealizes in
theorem totalVariationLowerBound (μ ν : Convexity.StdSimplex ℝ (reachableColorings G c₀))
    (hSupport : ∀ x ∈ μ.weights.support, ∃ w : List (Fin 600 × Fin 3),
      w.length = 1000 ∧ wordColoring G c₀ (capLetterPermutation H T) w = x)
    (hUniform : ∀ x, ν.weights x = 1 / ((reachableColorings G c₀).ncard : ℝ)) :
    1 - ((10 ^ 148595 : ℕ) : ℝ)⁻¹ < totalVariation μ ν := by
  classical
  have hB := (balancedColorLowerBound G O hDisjoint hPreserved hDegree certificates c₀ hPalette
    hCopies).2
  have hE : μ.weights.support.card ≤ 1800 ^ 1000 := by
    have key : ∀ x : μ.weights.support, ∃ v : List.Vector (Fin 600 × Fin 3) 1000,
        wordColoring G c₀ (capLetterPermutation H T) v.toList = x := by
      intro x
      obtain ⟨w, hlen, hw⟩ := hSupport x x.2
      exact ⟨⟨w, hlen⟩, hw⟩
    choose g hg using key
    have hinj : Function.Injective g := fun a b hab =>
      Subtype.ext (by rw [← hg a, ← hg b, hab])
    have hcard := Fintype.card_le_of_injective g hinj
    rw [Fintype.card_coe, card_vector, Fintype.card_prod, Fintype.card_fin, Fintype.card_fin,
      show (600 : ℕ) * 3 = 1800 from rfl] at hcard
    exact hcard
  exact totalVariation_gt_of_small_support μ ν _ _ (by positivity) hUniform _ hE
    (lt_of_lt_of_le thousandLetterSupportComparison hB)

end

open scoped commutatorElement in
theorem dihedralFiveCommutatorCard :
    commutator (DihedralGroup 5) = Subgroup.zpowers (DihedralGroup.r (1 : ZMod 5)) ∧
      Nat.card (commutator (DihedralGroup 5)) = 5 := by
  have key : commutator (DihedralGroup 5) = Subgroup.zpowers (DihedralGroup.r (1 : ZMod 5)) := by
    apply le_antisymm
    · rw [commutator, Subgroup.commutator_le]
      intro g₁ _ g₂ _
      have hrot : ∀ a b : DihedralGroup 5, ∃ k : ZMod 5, ⁅a, b⁆ = DihedralGroup.r k := by
        decide
      obtain ⟨k, hk⟩ := hrot g₁ g₂
      rw [hk, ← ZMod.natCast_zmod_val k, ← DihedralGroup.r_one_pow]
      exact Subgroup.pow_mem _ (Subgroup.mem_zpowers _) _
    · rw [Subgroup.zpowers_le]
      have hgen : ∃ a b : DihedralGroup 5, ⁅a, b⁆ = DihedralGroup.r 1 := by decide
      obtain ⟨a, b, hab⟩ := hgen
      rw [← hab, commutator]
      exact Subgroup.commutator_mem_commutator (Subgroup.mem_top a) (Subgroup.mem_top b)
  refine ⟨key, ?_⟩
  rw [key, Nat.card_zpowers, DihedralGroup.orderOf_r_one]

theorem alternatingFivePerfect :
    commutator (alternatingGroup (Fin 5)) = ⊤ ∧
      Nat.card (commutator (alternatingGroup (Fin 5))) = 60 := by
  have key : commutator (alternatingGroup (Fin 5)) = ⊤ :=
    commutator_alternatingGroup_eq_top (by simp)
  refine ⟨key, ?_⟩
  rw [key, Subgroup.card_top, Nat.card_eq_fintype_card, card_alternatingGroup]
  decide

set_option linter.unusedVariables false in
/-- In each product the right factor acts first, as in Mathlib's permutation group. -/
theorem threeCycleIdentityChecks (n : ℕ) (hnLow : 4 ≤ n) (hnHigh : n ≤ 9) :
    (∀ A B X Y : StickerLabels n, [A, B, X, Y].Nodup →
      [A, B, X].formPerm * [A, B, Y].formPerm⁻¹ = [A, Y, X].formPerm) ∧
    (∀ A x y z : StickerLabels n, [A, x, y, z].Nodup →
      [A, z, x].formPerm * [A, x, y].formPerm = [x, y, z].formPerm) := by
  constructor
  · intro A B X Y h
    obtain ⟨hAB, hAX, hAY, hBX, hBY, hXY⟩ := nodup_four h
    simp only [formPerm_three]
    exact cycle_id_one hAB hAX hAY hBX hBY hXY
  · intro A x y z h
    obtain ⟨hAx, hAy, hAz, hxy, hxz, hyz⟩ := nodup_four h
    simp only [formPerm_three]
    exact cycle_id_two hAx hAy hAz hxy hxz hyz

end ArchonFull600Theory
