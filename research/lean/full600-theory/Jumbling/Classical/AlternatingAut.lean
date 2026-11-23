import Mathlib.GroupTheory.SpecificGroups.Alternating.Centralizer
import ArchonFull600Theory.Basic

set_option linter.style.header false in
/-!
# `Aut(Alt(d)) = Sym(d)` for `d ≥ 7`

A classical theorem that the pinned Mathlib does not have, needed by R22B-19 and R22B-25 of
`research/jumbling/restoration/12-low-height/PROOFS-B.md`. It uses no puzzle notions.

For a finite type `α` with `7 ≤ |α|`:

* `autAlternatingGroup`: conjugation `MulAut.conjNormal : Perm α →* MulAut (alternatingGroup α)`
  is a group isomorphism `Perm α ≃* MulAut (alternatingGroup α)` (`conjNormal_bijective`);
* `existsUnique_perm_conj`: every automorphism of `Alt(α)` is conjugation by a unique permutation;
* `centralizer_alternatingGroup_eq_bot`: the centralizer of `Alt(α)` in `Sym(α)` is trivial (already
  for `4 ≤ |α|`), so conjugation is injective (`conjNormal_injective`);
* `exists_equiv_of_mulEquiv`: an isomorphism `Alt(β) ≃* Alt(α)` is conjugation by a bijection
  `β ≃ α`.

The range `d ≥ 7` is the one the handoff names. It does not reach `d = 6`, a true exception:
`Alt(6)` has an outer automorphism, which maps three-cycles to products of two disjoint
three-cycles. In step (a) below the two centralizer orders coincide exactly at `d = 6`.

Proof.

* (a) `isThreeCycle_map`: an automorphism keeps orders and centralizer orders. In `Alt(α)` the
  centralizer of a three-cycle has at least `3·(d - 3)!/2` elements and that of an element of cycle
  type `3^j`, `j ≥ 2`, at most `3^j·j!·(d - 3j)!` (Mathlib's `Equiv.Perm.nat_card_centralizer` and
  an index-two argument), which is smaller for `d ≥ 7` (`two_mul_lt`).
* (b) Fix `p ≠ q` and the stars `(p q k)`. The product of two stars is an involution, so their
  images, three-cycles by (a), share an oriented edge (`exists_common_edge`); the images of two
  fixed stars and any third one force one common edge `a ↦ b` (`common_edge_of_two`). Then
  `σ : p ↦ a, q ↦ b, k ↦ e(p q k)(b)` is a permutation with `e(p q k) = σ·(p q k)·σ⁻¹`.
* (c) The stars generate `Alt(α)` (`alternatingGroup_le_of_star_mem`, the argument of Full600's
  `PureControllerCertificate.three_cycle_mem`), so `e` is conjugation by `σ` (`exists_perm_conj`).
* (d) A permutation that commutes with every three-cycle is the identity
  (`centralizer_alternatingGroup_eq_bot`).
-/

open Equiv Equiv.Perm Subgroup
open scoped Nat

namespace Jumbling.AlternatingAut

/-- Classical, step (a): `2·3^(i+2)·(i+3)! < (3i+6)!`. -/
theorem aux_factorial (i : ℕ) : 2 * 3 ^ (i + 2) * (i + 3)! < (3 * i + 6)! := by
  induction i with
  | zero => decide
  | succ i ih =>
    have e1 : 3 * (i + 1) + 6 = 3 * i + 9 := by ring
    rw [e1]
    have h9 : (3 * i + 9)! = (3 * i + 9) * ((3 * i + 8) * ((3 * i + 7) * (3 * i + 6)!)) := by
      rw [show 3 * i + 9 = (3 * i + 8) + 1 by ring, Nat.factorial_succ,
        show 3 * i + 8 = (3 * i + 7) + 1 by ring, Nat.factorial_succ,
        show 3 * i + 7 = (3 * i + 6) + 1 by ring, Nat.factorial_succ]
    have h4 : (i + 1 + 3)! = (i + 4) * (i + 3)! := by
      rw [show i + 1 + 3 = (i + 3) + 1 by ring, Nat.factorial_succ]
    rw [h9, h4, pow_succ]
    have hp : 0 < (3 * i + 6)! := Nat.factorial_pos _
    have hc : 3 * (i + 4) ≤ (3 * i + 9) * ((3 * i + 8) * (3 * i + 7)) :=
      calc 3 * (i + 4) ≤ (3 * i + 8) * 2 := by omega
        _ ≤ (3 * i + 8) * (3 * i + 7) := Nat.mul_le_mul_left _ (by omega)
        _ ≤ _ := Nat.le_mul_of_pos_left _ (by omega)
    calc 2 * (3 ^ (i + 2) * 3) * ((i + 4) * (i + 3)!)
        = 3 * (i + 4) * (2 * 3 ^ (i + 2) * (i + 3)!) := by ring
      _ < 3 * (i + 4) * (3 * i + 6)! := Nat.mul_lt_mul_of_pos_left ih (by omega)
      _ ≤ (3 * i + 9) * ((3 * i + 8) * (3 * i + 7)) * (3 * i + 6)! :=
          Nat.mul_le_mul_right _ hc
      _ = _ := by ring

/-- Classical, step (a): for `d ≥ 7`, `j ≥ 2` and `3j ≤ d`, the centralizer bound of an element of
cycle type `3^j` is less than half that of a three-cycle; at `d = 6`, `j = 2` they are equal. -/
theorem two_mul_lt (n j : ℕ) (hn : 7 ≤ n) (hj : 2 ≤ j) (h3j : 3 * j ≤ n) :
    2 * ((n - 3 * j)! * 3 ^ j * j !) < (n - 3)! * 3 ^ 1 * 1 ! := by
  obtain ⟨i, rfl⟩ : ∃ i, j = i + 2 := ⟨j - 2, by omega⟩
  obtain ⟨m, rfl⟩ : ∃ m, n = m + 3 * i + 6 := ⟨n - 3 * i - 6, by omega⟩
  rw [show m + 3 * i + 6 - 3 * (i + 2) = m by omega,
    show m + 3 * i + 6 - 3 = m + (3 * i + 3) by omega]
  have hm : 0 < m ! := Nat.factorial_pos m
  rcases Nat.eq_zero_or_pos i with rfl | hi
  · have hm1 : 1 ≤ m := by omega
    have h3 : (m + (3 * 0 + 3))! = (m + 3) * ((m + 2) * ((m + 1) * m !)) := by
      rw [show m + (3 * 0 + 3) = (m + 2) + 1 by ring, Nat.factorial_succ,
        show m + 2 = (m + 1) + 1 by ring, Nat.factorial_succ, Nat.factorial_succ]
    rw [h3]
    have hc : 4 * (3 * 2) ≤ (m + 3) * ((m + 2) * (m + 1)) :=
      Nat.mul_le_mul (by omega) (Nat.mul_le_mul (by omega) (by omega))
    calc 2 * (m ! * 3 ^ (0 + 2) * (0 + 2)!) = 36 * m ! := by simp [Nat.factorial]; ring
      _ < 72 * m ! := by omega
      _ ≤ (m + 3) * ((m + 2) * (m + 1)) * 3 * m ! := by nlinarith
      _ = _ := by simp; ring
  · obtain ⟨k, rfl⟩ : ∃ k, i = k + 1 := ⟨i - 1, by omega⟩
    have hdvd := Nat.factorial_mul_factorial_dvd_factorial_add m (3 * k + 6)
    have hle := Nat.le_of_dvd (Nat.factorial_pos _) hdvd
    have ha := aux_factorial k
    rw [show 3 * (k + 1) + 3 = 3 * k + 6 by ring, show k + 1 + 2 = k + 3 by ring]
    have : 2 * (m ! * 3 ^ (k + 3) * (k + 3)!) < m ! * (3 * k + 6)! * 3 := by
      rw [pow_succ]
      nlinarith
    calc _ < m ! * (3 * k + 6)! * 3 := this
      _ ≤ (m + (3 * k + 6))! * 3 := by nlinarith
      _ = _ := by simp

variable {α : Type*} [Fintype α] [DecidableEq α]

/-- Classical, step (a): the centralizer in `Sym(α)` of a permutation of cycle type `3^j` has
`(d - 3j)!·3^j·j!` elements (Mathlib's `Equiv.Perm.nat_card_centralizer`). -/
theorem card_centralizer_replicate {g : Perm α} {j : ℕ}
    (hg : g.cycleType = Multiset.replicate j 3) :
    Nat.card (centralizer {g}) = (Fintype.card α - 3 * j)! * 3 ^ j * j ! := by
  rw [nat_card_centralizer, hg, Multiset.sum_replicate, Multiset.prod_replicate,
    Multiset.toFinset_replicate]
  rcases j with _ | j
  · simp
  · simp [Multiset.count_replicate_self, mul_comm]

/-- Classical, step (a): the centralizer of an even permutation in `Alt(α)`, read in `Sym(α)`. -/
theorem card_centralizer_alternatingGroup (g : alternatingGroup α) :
    Nat.card (centralizer {g}) =
      Nat.card (alternatingGroup α ⊓ centralizer {(g : Perm α)} : Subgroup (Perm α)) := by
  refine Nat.card_congr
    { toFun := fun x => ⟨x.1.1, Subgroup.mem_inf.2 ⟨x.1.2, ?_⟩⟩
      invFun := fun y => ⟨⟨y.1, (Subgroup.mem_inf.1 y.2).1⟩, ?_⟩
      left_inv := fun x => rfl
      right_inv := fun y => rfl }
  · have := mem_centralizer_singleton_iff.1 x.2
    exact mem_centralizer_singleton_iff.2 (congrArg Subtype.val this)
  · have := mem_centralizer_singleton_iff.1 (Subgroup.mem_inf.1 y.2).2
    exact mem_centralizer_singleton_iff.2 (Subtype.ext this)

/-- Classical, step (a): a centralizer in `Alt(α)` is at most the centralizer in `Sym(α)`. -/
theorem card_centralizer_alternatingGroup_le (g : alternatingGroup α) :
    Nat.card (centralizer {g}) ≤ Nat.card (centralizer {(g : Perm α)}) := by
  rw [card_centralizer_alternatingGroup]
  exact Subgroup.card_le_of_le inf_le_right

/-- Classical, step (a): a centralizer in `Alt(α)` has index at most two in the centralizer in
`Sym(α)`. -/
theorem card_centralizer_le_two_mul (h2 : 2 ≤ Fintype.card α) (g : alternatingGroup α) :
    Nat.card (centralizer {(g : Perm α)}) ≤ 2 * Nat.card (centralizer {g}) := by
  have : Nontrivial α := Fintype.one_lt_card_iff_nontrivial.1 (by omega)
  rw [card_centralizer_alternatingGroup, ← relIndex_mul_card (alternatingGroup α)]
  have hd := relIndex_dvd_index_of_normal (alternatingGroup α) (centralizer {(g : Perm α)})
  rw [alternatingGroup.index_eq_two] at hd
  have := Nat.le_of_dvd (by norm_num) hd
  exact Nat.mul_le_mul_right _ this

/-- Classical, step (a): automorphisms keep the orders of centralizers. -/
theorem card_centralizer_map (e : MulAut (alternatingGroup α)) (g : alternatingGroup α) :
    Nat.card (centralizer {e g}) = Nat.card (centralizer {g}) := by
  refine (Nat.card_congr (e.toEquiv.subtypeEquiv fun x => ?_)).symm
  simp only [mem_centralizer_singleton_iff, MulEquiv.toEquiv_eq_coe, EquivLike.coe_coe]
  exact ⟨fun h => h.map e, fun h => by simpa using h.map e.symm⟩

/-- Classical, step (a): an automorphism of `Alt(α)` maps three-cycles to three-cycles when
`7 ≤ |α|`. -/
theorem isThreeCycle_map (h7 : 7 ≤ Fintype.card α) (e : MulAut (alternatingGroup α))
    {g : alternatingGroup α} (hg : (g : Perm α).IsThreeCycle) :
    ((e g : alternatingGroup α) : Perm α).IsThreeCycle := by
  have hord : orderOf ((e g : alternatingGroup α) : Perm α) = 3 := by
    rw [Subgroup.orderOf_coe, MulEquiv.orderOf_eq, ← Subgroup.orderOf_coe, hg.orderOf]
  obtain ⟨k, hk⟩ := cycleType_prime_order (hord ▸ Nat.prime_three)
  rw [hord] at hk
  rcases k with _ | k
  · exact hk
  exfalso
  have hsum := sum_cycleType_le ((e g : alternatingGroup α) : Perm α)
  rw [hk, Multiset.sum_replicate, smul_eq_mul] at hsum
  have h1 := card_centralizer_replicate hk
  have h2 := card_centralizer_replicate (g := (g : Perm α)) (j := 1) hg
  have h3 := card_centralizer_alternatingGroup_le (e g)
  have h4 := card_centralizer_le_two_mul (by omega) g
  have h5 := card_centralizer_map e g
  have h6 := two_mul_lt (Fintype.card α) (k + 1 + 1) h7 (by omega) (by omega)
  rw [h2] at h4
  rw [h1, h5] at h3
  rw [mul_one] at h4
  omega

/-! ### Three-cycles -/

section ThreeCycle

variable {w : Perm α}

/-- A three-cycle has order three. -/
theorem three_apply (hw : w.IsThreeCycle) (x : α) : w (w (w x)) = x := by
  have h := pow_orderOf_eq_one w
  rw [hw.orderOf] at h
  have := congrArg (fun f : Perm α => f x) h
  simpa [pow_succ] using this

/-- A three-cycle has no point of period two. -/
theorem apply_apply_ne_self (hw : w.IsThreeCycle) {x : α} (hx : w x ≠ x) : w (w x) ≠ x := by
  intro h
  have h3 := three_apply hw x
  rw [h] at h3
  exact hx h3

omit [Fintype α] [DecidableEq α] in
/-- A moved point's image is moved. -/
theorem apply_apply_ne_apply {x : α} (hx : w x ≠ x) : w (w x) ≠ w x :=
  fun h => hx (w.injective h)

/-- The support of a three-cycle is `{x, w x, w (w x)}` for any moved point `x`. -/
theorem eq_of_apply_ne (hw : w.IsThreeCycle) {x y : α} (hx : w x ≠ x) (hy : w y ≠ y) :
    y = x ∨ y = w x ∨ y = w (w x) := by
  have hs := (hw.support_eq_iff_mem_support (a := x)).2 (mem_support.2 hx)
  have hy' : y ∈ w.support := mem_support.2 hy
  rw [hs] at hy'
  simpa using hy'

/-- Classical, step (b): two three-cycles whose product is an involution share an oriented edge
`y ↦ u y`, and their third points differ. -/
theorem exists_common_edge {u v : Perm α} (hu : u.IsThreeCycle) (hv : v.IsThreeCycle)
    (h2 : (u * v) ^ 2 = 1) (h1 : u * v ≠ 1) :
    ∃ y, u y ≠ y ∧ v y = u y ∧ v (u y) ≠ u (u y) := by
  have hA : ∀ x, u (v (u (v x))) = x := fun x => by
    have := congrArg (fun f : Perm α => f x) h2
    simpa [pow_two] using this
  by_cases hout : ∃ x, v x ≠ x ∧ u x = x
  · obtain ⟨x, hvx, hux⟩ := hout
    have hB : v (u (v x)) = x := u.injective (by rw [hA x, hux])
    have hC : u (v x) = v (v x) := by
      have : v (v (v (u (v x)))) = v (v x) := by rw [hB]
      rwa [three_apply hv] at this
    refine ⟨v x, ?_, hC.symm, ?_⟩
    · rw [hC]
      exact apply_apply_ne_apply hvx
    · intro h
      rw [hB] at h
      have h' := congrArg u h
      rw [three_apply hu, hux] at h'
      exact hvx h'.symm
  · push Not at hout
    have hsub : v.support ⊆ u.support := fun x hx => mem_support.2 (hout x (mem_support.1 hx))
    have heq : v.support = u.support :=
      Finset.eq_of_subset_of_card_le hsub (by rw [hu.card_support, hv.card_support])
    have hiff : ∀ z, v z ≠ z ↔ u z ≠ z := fun z => by
      rw [← mem_support, ← mem_support, heq]
    obtain ⟨x, hx⟩ : ∃ x, v x ≠ x := by
      obtain ⟨x, hx⟩ := Finset.card_pos.1 (by rw [hv.card_support]; norm_num : 0 < v.support.card)
      exact ⟨x, mem_support.1 hx⟩
    have hux := (hiff x).1 hx
    have hvv : v (v x) ≠ v (v (v x)) := by
      rw [three_apply hv]
      exact apply_apply_ne_self hv hx
    rcases eq_of_apply_ne hu hux ((hiff (v x)).1 (apply_apply_ne_apply hx)) with h | h | h
    · exact absurd h hx
    · -- `v = u`
      have h2' : v (v x) = u (u x) := by
        rcases eq_of_apply_ne hu hux ((hiff (v (v x))).1 (Ne.symm hvv)) with h' | h' | h'
        · exact absurd h' (apply_apply_ne_self hv hx)
        · exact absurd (h'.trans h.symm) (apply_apply_ne_apply hx)
        · exact h'
      have hvu : v = u := by
        ext z
        by_cases hz : u z = z
        · have : v z = z := by
            by_contra h'
            exact (hiff z).1 h' hz
          rw [this, hz]
        · rcases eq_of_apply_ne hu hux hz with hz' | hz' | hz' <;> rw [hz']
          · exact h
          · calc v (u x) = v (v x) := by rw [h]
              _ = u (u x) := h2'
          · calc v (u (u x)) = v (v (v x)) := by rw [h2']
              _ = x := three_apply hv x
              _ = u (u (u x)) := (three_apply hu x).symm
      have h4 := hA x
      rw [hvu, three_apply hu] at h4
      exact absurd h4 hux
    · -- `v = u⁻¹`
      exfalso
      apply h1
      ext z
      simp only [Perm.coe_mul, Function.comp_apply, Perm.coe_one, id_eq]
      by_cases hz : u z = z
      · have : v z = z := by
          by_contra h'
          exact (hiff z).1 h' hz
        rw [this, hz]
      · rcases eq_of_apply_ne hu hux hz with hz' | hz' | hz' <;> rw [hz']
        · rw [h, three_apply hu]
        · have hm : v (u x) ≠ u x := (hiff (u x)).2 (apply_apply_ne_apply hux)
          rcases eq_of_apply_ne hu hux ((hiff (v (u x))).1 (apply_apply_ne_apply hm)) with
            h' | h' | h'
          · rw [h']
          · exact absurd h' hm
          · exact absurd (v.injective (h'.trans h.symm)) hux
        · rcases eq_of_apply_ne hu hux ((hiff (v (v x))).1 (Ne.symm hvv)) with h' | h' | h'
          · exact absurd h' (apply_apply_ne_self hv hx)
          · rw [← h, h', h]
          · exact absurd (h'.trans h.symm) (apply_apply_ne_apply hx)

end ThreeCycle

/-! ### Stars `(p q k)` -/

section Star

/-- The star three-cycle `(p q k)`: `p ↦ q ↦ k ↦ p`. -/
def star (p q k : α) : Perm α :=
  swap p q * swap q k

/-- Classical, step (b): a star is even. -/
theorem star_mem {p q k : α} (hpq : p ≠ q) (hqk : q ≠ k) : star p q k ∈ alternatingGroup α := by
  rw [mem_alternatingGroup, star, map_mul, sign_swap hpq, sign_swap hqk]
  rfl

/-- Classical, step (b): a star is a three-cycle. -/
theorem star_isThreeCycle {p q k : α} (hpq : p ≠ q) (hpk : p ≠ k) (hqk : q ≠ k) :
    (star p q k).IsThreeCycle := by
  rw [star, swap_comm p q]
  exact isThreeCycle_swap_mul_swap_same (Ne.symm hpq) hqk hpk

omit [Fintype α] in
/-- Classical, step (b): the star `(p q k)` maps `q` to `k`. -/
theorem star_apply_right {p q k : α} (hpk : p ≠ k) (hqk : q ≠ k) : star p q k q = k := by
  simp [star, swap_apply_of_ne_of_ne (Ne.symm hpk) (Ne.symm hqk)]

omit [Fintype α] in
/-- Classical, step (b): `(p q k)·(p q l) = (p k)(q l)`. -/
theorem star_mul_star {p q k l : α} (hpq : p ≠ q) (hpk : p ≠ k) (hqk : q ≠ k) (hpl : p ≠ l)
    (hql : q ≠ l) (hkl : k ≠ l) : star p q k * star p q l = swap p k * swap q l := by
  ext t
  simp only [star, Perm.coe_mul, Function.comp_apply]
  by_cases h1 : t = p
  · subst h1; simp [swap_apply_of_ne_of_ne, *, Ne.symm]
  by_cases h2 : t = q
  · subst h2; simp [swap_apply_of_ne_of_ne, *, Ne.symm]
  by_cases h3 : t = k
  · subst h3; simp [swap_apply_of_ne_of_ne, *, Ne.symm]
  by_cases h4 : t = l
  · subst h4; simp [swap_apply_of_ne_of_ne, *, Ne.symm]
  simp [swap_apply_of_ne_of_ne, *]

omit [Fintype α] in
/-- Classical, step (b): the product of two stars is an involution. -/
theorem star_mul_star_sq {p q k l : α} (hpq : p ≠ q) (hpk : p ≠ k) (hqk : q ≠ k) (hpl : p ≠ l)
    (hql : q ≠ l) (hkl : k ≠ l) : (star p q k * star p q l) ^ 2 = 1 := by
  rw [star_mul_star hpq hpk hqk hpl hql hkl, pow_two]
  ext t
  simp only [Perm.coe_mul, Function.comp_apply, Perm.coe_one, id_eq]
  by_cases h1 : t = p
  · subst h1; simp [swap_apply_of_ne_of_ne, *, Ne.symm]
  by_cases h2 : t = q
  · subst h2; simp [swap_apply_of_ne_of_ne, *, Ne.symm]
  by_cases h3 : t = k
  · subst h3; simp [swap_apply_of_ne_of_ne, *, Ne.symm]
  by_cases h4 : t = l
  · subst h4; simp [swap_apply_of_ne_of_ne, *, Ne.symm]
  simp [swap_apply_of_ne_of_ne, *]

omit [Fintype α] in
/-- Classical, step (b): the product of two distinct stars is not the identity. -/
theorem star_mul_star_ne_one {p q k l : α} (hpq : p ≠ q) (hpk : p ≠ k) (hqk : q ≠ k)
    (hpl : p ≠ l) (hql : q ≠ l) (hkl : k ≠ l) : star p q k * star p q l ≠ 1 := by
  rw [star_mul_star hpq hpk hqk hpl hql hkl]
  intro h
  have := congrArg (fun f : Perm α => f p) h
  simp [swap_apply_of_ne_of_ne hpq hpl] at this
  exact hpk this.symm

/-- Classical, step (c): the stars `(p q k)` generate `Alt(α)` (the argument of Full600's
`PureControllerCertificate.three_cycle_mem`, with the buffers `p` and `q`). -/
theorem alternatingGroup_le_of_star_mem {G : Subgroup (Perm α)} {p q : α} (hpq : p ≠ q)
    (h : ∀ k, p ≠ k → q ≠ k → star p q k ∈ G) : alternatingGroup α ≤ G := by
  have hp : ∀ u v, u ≠ p → v ≠ p → u ≠ v → swap p u * swap u v ∈ G := by
    intro u v hup hvp huv
    by_cases huq : u = q
    · subst huq
      exact h v (Ne.symm hvp) huv
    by_cases hvq : v = q
    · subst hvq
      have := G.inv_mem (h u (Ne.symm hup) (Ne.symm huq))
      rwa [star, ArchonFull600Theory.cycle_inv hpq (Ne.symm hup) (Ne.symm huq)] at this
    have := G.mul_mem (h v (Ne.symm hvp) (Ne.symm hvq))
      (G.inv_mem (h u (Ne.symm hup) (Ne.symm huq)))
    rwa [star, star, ArchonFull600Theory.cycle_id_one hpq (Ne.symm hvp) (Ne.symm hup) (Ne.symm hvq)
      (Ne.symm huq) (Ne.symm huv)] at this
  rw [← closure_three_cycles_eq_alternating, closure_le]
  intro τ hτ
  obtain ⟨x, y, z, hxy, hxz, hyz, rfl⟩ := ArchonFull600Theory.isThreeCycle_eq_swap_mul_swap hτ
  by_cases hxp : x = p
  · rw [hxp] at hxy hxz ⊢
    exact hp y z (Ne.symm hxy) (Ne.symm hxz) hyz
  by_cases hyp : y = p
  · rw [ArchonFull600Theory.cycle_rotate hxy hxz hyz, hyp]
    rw [hyp] at hyz hxy
    exact hp z x (Ne.symm hyz) hxp (Ne.symm hxz)
  by_cases hzp : z = p
  · rw [ArchonFull600Theory.cycle_rotate hxy hxz hyz,
      ArchonFull600Theory.cycle_rotate hyz (Ne.symm hxy) (Ne.symm hxz), hzp]
    rw [hzp] at hyz hxz
    exact hp x y hxp hyp hxy
  rw [← ArchonFull600Theory.cycle_id_two (Ne.symm hxp) (Ne.symm hyp) (Ne.symm hzp) hxy hxz hyz]
  exact G.mul_mem (hp z x hzp hxp (Ne.symm hxz)) (hp x y hxp hyp hxy)

end Star

/-! ### Step (b): the images of the stars share an oriented edge -/

/-- Classical, step (b): if the three-cycles `u₁` and `u₂` share the edge `a ↦ u₁ a` with different
third points, and the three-cycle `w` shares an oriented edge with each of them, then `w` contains
the edge `a ↦ u₁ a`. -/
theorem common_edge_of_two {u₁ u₂ w : Perm α} (h₁ : u₁.IsThreeCycle) (h₂ : u₂.IsThreeCycle)
    (hw : w.IsThreeCycle) {a : α} (ha : u₁ a ≠ a) (hab : u₂ a = u₁ a)
    (hcd : u₂ (u₁ a) ≠ u₁ (u₁ a)) {y₁ y₂ : α} (hy₁ : u₁ y₁ ≠ y₁) (hw₁ : w y₁ = u₁ y₁)
    (hy₂ : u₂ y₂ ≠ y₂) (hw₂ : w y₂ = u₂ y₂) : w a = u₁ a := by
  have ha₂ : u₂ a ≠ a := hab ▸ ha
  have hbc : u₁ (u₁ a) ≠ u₁ a := apply_apply_ne_apply ha
  have hca : u₁ (u₁ a) ≠ a := apply_apply_ne_self h₁ ha
  have hbd : u₂ (u₁ a) ≠ u₁ a := by rw [← hab]; exact apply_apply_ne_apply ha₂
  have hda : u₂ (u₁ a) ≠ a := by rw [← hab]; exact apply_apply_ne_self h₂ ha₂
  have hd3 : u₂ (u₂ (u₁ a)) = a := by rw [← hab]; exact three_apply h₂ a
  rcases eq_of_apply_ne h₁ ha hy₁ with e₁ | e₁ | e₁ <;> rw [e₁] at hw₁
  · exact hw₁
  · rcases eq_of_apply_ne h₂ ha₂ hy₂ with e₂ | e₂ | e₂ <;> rw [e₂, hab] at hw₂
    · exact hw₂
    · exact absurd (hw₂.symm.trans hw₁) hcd
    · -- `w b = c` and `w d = a`
      rw [hd3] at hw₂
      have hwb : w (u₁ a) ≠ u₁ a := by rw [hw₁]; exact hbc
      have hwd : w (u₂ (u₁ a)) ≠ u₂ (u₁ a) := by rw [hw₂]; exact hda.symm
      rcases eq_of_apply_ne hw hwb hwd with f | f | f
      · exact absurd f hbd
      · exact absurd (f.trans hw₁) hcd
      · have := congrArg w f
        rw [three_apply hw, hw₂] at this
        exact absurd this.symm ha
  · rw [three_apply h₁] at hw₁
    rcases eq_of_apply_ne h₂ ha₂ hy₂ with e₂ | e₂ | e₂ <;> rw [e₂, hab] at hw₂
    · exact hw₂
    · -- `w c = a` and `w b = d`
      have hwb : w (u₁ a) ≠ u₁ a := by rw [hw₂]; exact hbd
      have hwc : w (u₁ (u₁ a)) ≠ u₁ (u₁ a) := by rw [hw₁]; exact hca.symm
      rcases eq_of_apply_ne hw hwb hwc with f | f | f
      · exact absurd f hbc
      · exact absurd (f.trans hw₂).symm hcd
      · have := congrArg w f
        rw [three_apply hw, hw₁] at this
        exact absurd this.symm ha
    · rw [hd3] at hw₂
      exact absurd (w.injective (hw₂.trans hw₁.symm)) hcd
/-! ### Step (b) and (c): every automorphism is a conjugation -/

section Construction

variable (e : MulAut (alternatingGroup α)) {p q : α} (hpq : p ≠ q)

/-- The image `e (p q k)` of a star as a permutation, or `1` when `k ∈ {p, q}`. -/
noncomputable def img (k : α) : Perm α :=
  if h : p ≠ k ∧ q ≠ k then (e ⟨star p q k, star_mem hpq h.2⟩ : Perm α) else 1

variable {e hpq}

/-- Classical, step (b): `img` at a point outside `{p, q}`. -/
theorem img_eq {k : α} (hpk : p ≠ k) (hqk : q ≠ k) :
    img e hpq k = (e ⟨star p q k, star_mem hpq hqk⟩ : Perm α) := by
  simp [img, hpk, hqk]

/-- Classical, step (b): the image of a star is a three-cycle, by step (a). -/
theorem img_isThreeCycle (h7 : 7 ≤ Fintype.card α) {k : α} (hpk : p ≠ k) (hqk : q ≠ k) :
    (img e hpq k).IsThreeCycle := by
  rw [img_eq hpk hqk]
  exact isThreeCycle_map h7 e (star_isThreeCycle hpq hpk hqk)

/-- Classical, step (b): the images of two distinct stars share an oriented edge. -/
theorem img_edge (h7 : 7 ≤ Fintype.card α) {k l : α} (hpk : p ≠ k) (hqk : q ≠ k) (hpl : p ≠ l)
    (hql : q ≠ l) (hkl : k ≠ l) :
    ∃ y, img e hpq k y ≠ y ∧ img e hpq l y = img e hpq k y ∧
      img e hpq l (img e hpq k y) ≠ img e hpq k (img e hpq k y) := by
  set tk : alternatingGroup α := ⟨star p q k, star_mem hpq hqk⟩
  set tl : alternatingGroup α := ⟨star p q l, star_mem hpq hql⟩
  have hsq : (tk * tl) ^ 2 = 1 := Subtype.ext (star_mul_star_sq hpq hpk hqk hpl hql hkl)
  have hne : tk * tl ≠ 1 := fun h =>
    star_mul_star_ne_one hpq hpk hqk hpl hql hkl (congrArg Subtype.val h)
  apply exists_common_edge (img_isThreeCycle h7 hpk hqk) (img_isThreeCycle h7 hpl hql)
  · rw [img_eq hpk hqk, img_eq hpl hql, ← Subgroup.coe_mul, ← map_mul, ← Subgroup.coe_pow,
      ← map_pow, hsq, map_one, Subgroup.coe_one]
  · rw [img_eq hpk hqk, img_eq hpl hql, ← Subgroup.coe_mul, ← map_mul]
    intro h
    exact hne ((map_eq_one_iff e e.injective).1 (Subtype.ext h))

/-- Classical, step (b): distinct stars have distinct images. -/
theorem img_injective {k l : α} (hpk : p ≠ k) (hqk : q ≠ k) (hpl : p ≠ l) (hql : q ≠ l)
    (h : img e hpq k = img e hpq l) : k = l := by
  rw [img_eq hpk hqk, img_eq hpl hql] at h
  have h' := e.injective (Subtype.ext h)
  have := congrArg (fun x : alternatingGroup α => (x : Perm α) q) h'
  simp only at this
  rwa [star_apply_right hpk hqk, star_apply_right hpl hql] at this

end Construction

omit [DecidableEq α] in
/-- A finite type has a point outside any smaller finite set. -/
theorem exists_not_mem {s : Finset α} (hs : s.card < Fintype.card α) : ∃ x, x ∉ s := by
  obtain ⟨x, -, hx⟩ := Finset.exists_mem_notMem_of_card_lt_card (s := s) (t := Finset.univ)
    (by rwa [Finset.card_univ])
  exact ⟨x, hx⟩

/-- Classical, steps (b) and (c): for `7 ≤ |α|` every automorphism of `Alt(α)` is conjugation by a
permutation of `α`. -/
theorem exists_perm_conj (h7 : 7 ≤ Fintype.card α) (e : MulAut (alternatingGroup α)) :
    ∃ σ : Perm α, ∀ g : alternatingGroup α, (e g : Perm α) = σ * g * σ⁻¹ := by
  obtain ⟨p, -⟩ := exists_not_mem (s := (∅ : Finset α)) (by simp; omega)
  obtain ⟨q, hq⟩ := exists_not_mem (s := {p}) (by simp; omega)
  have hpq : p ≠ q := fun h => hq (by simp [h])
  obtain ⟨k₁, hk₁⟩ := exists_not_mem (s := {p, q}) (lt_of_le_of_lt Finset.card_le_two (by omega))
  obtain ⟨k₂, hk₂⟩ := exists_not_mem (s := {p, q, k₁})
    (lt_of_le_of_lt Finset.card_le_three (by omega))
  simp only [Finset.mem_insert, Finset.mem_singleton, not_or] at hk₁ hk₂
  obtain ⟨hk₁p, hk₁q⟩ := hk₁
  obtain ⟨hk₂p, hk₂q, hk₂k₁⟩ := hk₂
  have h3 : ∀ k, p ≠ k → q ≠ k → (img e hpq k).IsThreeCycle :=
    fun k hpk hqk => img_isThreeCycle h7 hpk hqk
  obtain ⟨a, ha, hab, hcd⟩ := img_edge (e := e) (hpq := hpq) h7 (Ne.symm hk₁p) (Ne.symm hk₁q)
    (Ne.symm hk₂p) (Ne.symm hk₂q) (Ne.symm hk₂k₁)
  set s := img e hpq with hs_def
  set b := s k₁ a with hb_def
  have hall : ∀ k, p ≠ k → q ≠ k → s k a = b := by
    intro k hpk hqk
    by_cases hk1 : k = k₁
    · rw [hk1]
    by_cases hk2 : k = k₂
    · rw [hk2]
      exact hab
    obtain ⟨y₁, hy₁, hw₁, -⟩ := img_edge (e := e) (hpq := hpq) h7 (Ne.symm hk₁p) (Ne.symm hk₁q)
      hpk hqk (Ne.symm hk1)
    obtain ⟨y₂, hy₂, hw₂, -⟩ := img_edge (e := e) (hpq := hpq) h7 (Ne.symm hk₂p) (Ne.symm hk₂q)
      hpk hqk (Ne.symm hk2)
    exact common_edge_of_two (h3 _ (Ne.symm hk₁p) (Ne.symm hk₁q))
      (h3 _ (Ne.symm hk₂p) (Ne.symm hk₂q)) (h3 k hpk hqk) ha hab hcd hy₁ hw₁ hy₂ hw₂
  have hba : b ≠ a := ha
  have hform : ∀ k, p ≠ k → q ≠ k → s k = swap a b * swap b (s k b) := by
    intro k hpk hqk
    have hmem : a ∈ (s k).support := by
      rw [mem_support, hall k hpk hqk]
      exact hba
    have := ((h3 k hpk hqk).eq_swap_mul_swap_iff_mem_support (a := a)).2 hmem
    rwa [hall k hpk hqk] at this
  have hcne : ∀ k, p ≠ k → q ≠ k → s k b ≠ a ∧ s k b ≠ b := by
    intro k hpk hqk
    have hka : s k a ≠ a := by
      rw [hall k hpk hqk]
      exact hba
    rw [← hall k hpk hqk]
    exact ⟨apply_apply_ne_self (h3 k hpk hqk) hka, apply_apply_ne_apply hka⟩
  let σf : α → α := fun x => if x = p then a else if x = q then b else s x b
  have hσp : σf p = a := by simp [σf]
  have hσq : σf q = b := by simp [σf, Ne.symm hpq]
  have hσk : ∀ k, p ≠ k → q ≠ k → σf k = s k b := fun k hpk hqk => by
    simp [σf, Ne.symm hpk, Ne.symm hqk]
  have cls : ∀ z, z = p ∨ z = q ∨ (p ≠ z ∧ q ≠ z) := fun z => by
    by_cases h1 : z = p
    · exact Or.inl h1
    by_cases h2 : z = q
    · exact Or.inr (Or.inl h2)
    exact Or.inr (Or.inr ⟨Ne.symm h1, Ne.symm h2⟩)
  have hinj : Function.Injective σf := by
    intro x y hxy
    rcases cls x with rfl | rfl | ⟨hpx, hqx⟩ <;> rcases cls y with rfl | rfl | ⟨hpy, hqy⟩
    · rfl
    · rw [hσp, hσq] at hxy
      exact absurd hxy.symm hba
    · rw [hσp, hσk y hpy hqy] at hxy
      exact absurd hxy.symm (hcne y hpy hqy).1
    · rw [hσq, hσp] at hxy
      exact absurd hxy hba
    · rfl
    · rw [hσq, hσk y hpy hqy] at hxy
      exact absurd hxy.symm (hcne y hpy hqy).2
    · rw [hσk x hpx hqx, hσp] at hxy
      exact absurd hxy (hcne x hpx hqx).1
    · rw [hσk x hpx hqx, hσq] at hxy
      exact absurd hxy (hcne x hpx hqx).2
    · rw [hσk x hpx hqx, hσk y hpy hqy] at hxy
      have hst : s x = s y := by rw [hform x hpx hqx, hform y hpy hqy, hxy]
      exact img_injective hpx hqx hpy hqy hst
  let σ : Perm α := Equiv.ofBijective σf (Finite.injective_iff_bijective.1 hinj)
  have hconj : ∀ k (hpk : p ≠ k) (hqk : q ≠ k),
      (e ⟨star p q k, star_mem hpq hqk⟩ : Perm α) = σ * star p q k * σ⁻¹ := by
    intro k hpk hqk
    rw [← img_eq (e := e) (hpq := hpq) hpk hqk, star,
      ArchonFull600Theory.PureControllerCertificate.conj_swap_mul_swap]
    simp only [σ, Equiv.ofBijective_apply]
    rw [hσp, hσq, hσk k hpk hqk]
    exact hform k hpk hqk
  let E : Subgroup (alternatingGroup α) :=
    MonoidHom.eqLocus e.toMonoidHom (MulAut.conjNormal σ).toMonoidHom
  have hle : alternatingGroup α ≤ E.map (alternatingGroup α).subtype := by
    refine alternatingGroup_le_of_star_mem hpq fun k hpk hqk => ?_
    refine Subgroup.mem_map.2 ⟨⟨star p q k, star_mem hpq hqk⟩, ?_, rfl⟩
    change e _ = MulAut.conjNormal σ _
    exact Subtype.ext (by rw [MulAut.conjNormal_apply]; exact hconj k hpk hqk)
  refine ⟨σ, fun g => ?_⟩
  obtain ⟨y, hy, hyg⟩ := Subgroup.mem_map.1 (hle g.2)
  have hyg' : y = g := Subtype.ext hyg
  subst hyg'
  have hy' : e y = MulAut.conjNormal σ y := hy
  rw [hy', MulAut.conjNormal_apply]

/-! ### Step (d): the centralizer is trivial -/

/-- Classical, step (d): the centralizer of `Alt(α)` in `Sym(α)` is trivial when `4 ≤ |α|`. -/
theorem centralizer_alternatingGroup_eq_bot (h4 : 4 ≤ Fintype.card α) :
    centralizer (alternatingGroup α : Set (Perm α)) = ⊥ := by
  rw [eq_bot_iff]
  intro σ hσ
  rw [mem_bot]
  by_contra hne
  obtain ⟨x, hx⟩ : ∃ x, σ x ≠ x := by
    by_contra h
    push Not at h
    exact hne (Equiv.ext h)
  obtain ⟨z, hz⟩ := exists_not_mem (s := {x, σ x}) (lt_of_le_of_lt Finset.card_le_two (by omega))
  obtain ⟨w, hw⟩ := exists_not_mem (s := {x, σ x, z})
    (lt_of_le_of_lt Finset.card_le_three (by omega))
  simp only [Finset.mem_insert, Finset.mem_singleton, not_or] at hz hw
  obtain ⟨hzx, hzs⟩ := hz
  obtain ⟨hwx, hws, hwz⟩ := hw
  have hc : swap x z * swap z w ∈ alternatingGroup α := by
    rw [mem_alternatingGroup, map_mul, sign_swap (Ne.symm hzx), sign_swap (Ne.symm hwz)]
    rfl
  have hcomm := mem_centralizer_iff.1 hσ _ hc
  have := congrArg (fun f : Perm α => f x) hcomm
  simp only [Perm.coe_mul, Function.comp_apply] at this
  rw [swap_apply_of_ne_of_ne (Ne.symm hzx) (Ne.symm hwx), swap_apply_left,
    swap_apply_of_ne_of_ne (Ne.symm hzs) (Ne.symm hws),
    swap_apply_of_ne_of_ne hx (Ne.symm hzs)] at this
  exact hzx (σ.injective this).symm

/-- Classical, step (d): conjugation `Sym(α) → Aut(Alt(α))` is injective when `4 ≤ |α|`. -/
theorem conjNormal_injective (h4 : 4 ≤ Fintype.card α) :
    Function.Injective (MulAut.conjNormal : Perm α →* MulAut (alternatingGroup α)) := by
  rw [injective_iff_map_eq_one]
  intro σ hσ
  have hmem : σ ∈ centralizer (alternatingGroup α : Set (Perm α)) := by
    rw [mem_centralizer_iff]
    intro g hg
    have := congrArg (fun f : MulAut (alternatingGroup α) => ((f ⟨g, hg⟩ : alternatingGroup α) :
      Perm α)) hσ
    simp only [MulAut.conjNormal_apply, MulAut.one_apply] at this
    calc g * σ = σ * g * σ⁻¹ * σ := by rw [this]
      _ = σ * g := by group
  rwa [centralizer_alternatingGroup_eq_bot h4, mem_bot] at hmem

/-! ### The theorem -/

/-- Classical, step (c): conjugation `Sym(α) → Aut(Alt(α))` is surjective when `7 ≤ |α|`. -/
theorem conjNormal_surjective (h7 : 7 ≤ Fintype.card α) :
    Function.Surjective (MulAut.conjNormal : Perm α →* MulAut (alternatingGroup α)) := fun e => by
  obtain ⟨σ, hσ⟩ := exists_perm_conj h7 e
  exact ⟨σ, MulEquiv.ext fun g => Subtype.ext (by rw [MulAut.conjNormal_apply, hσ])⟩

/-- Classical: conjugation `Sym(α) → Aut(Alt(α))` is bijective when `7 ≤ |α|`. -/
theorem conjNormal_bijective (h7 : 7 ≤ Fintype.card α) :
    Function.Bijective (MulAut.conjNormal : Perm α →* MulAut (alternatingGroup α)) :=
  ⟨conjNormal_injective (by omega), conjNormal_surjective h7⟩

/-- Classical: **`Aut(Alt(d)) = Sym(d)` for `d ≥ 7`.** For a finite type `α` with `7 ≤ |α|`,
conjugation is an isomorphism `Perm α ≃* MulAut (alternatingGroup α)`. The bound is needed:
`d = 6` is a true exception (`Alt(6)` has an outer automorphism). -/
noncomputable def autAlternatingGroup (h7 : 7 ≤ Fintype.card α) :
    Perm α ≃* MulAut (alternatingGroup α) :=
  MulEquiv.ofBijective MulAut.conjNormal (conjNormal_bijective h7)

/-- Classical: `autAlternatingGroup` is conjugation. -/
theorem autAlternatingGroup_apply (h7 : 7 ≤ Fintype.card α) (σ : Perm α) (g : alternatingGroup α) :
    ((autAlternatingGroup h7 σ g : alternatingGroup α) : Perm α) = σ * g * σ⁻¹ :=
  MulAut.conjNormal_apply σ g

/-- Classical: for `7 ≤ |α|`, every automorphism of `Alt(α)` is conjugation by a unique
permutation of `α`. -/
theorem existsUnique_perm_conj (h7 : 7 ≤ Fintype.card α) (e : MulAut (alternatingGroup α)) :
    ∃! σ : Perm α, ∀ g : alternatingGroup α, (e g : Perm α) = σ * g * σ⁻¹ := by
  obtain ⟨σ, hσ⟩ := exists_perm_conj h7 e
  refine ⟨σ, hσ, fun τ hτ => conjNormal_injective (by omega) ?_⟩
  exact MulEquiv.ext fun g => Subtype.ext (by
    rw [MulAut.conjNormal_apply, MulAut.conjNormal_apply, ← hτ g, ← hσ g])

/-- Conjugation by a bijection between two types keeps the cycle type. -/
theorem cycleType_permCongr {β : Type*} [Fintype β] [DecidableEq β] (τ : α ≃ β) (σ : Perm α) :
    (τ.permCongr σ).cycleType = σ.cycleType := by
  have : τ.permCongr σ =
      σ.extendDomain (τ.trans (Equiv.subtypeUnivEquiv (p := fun _ : β => True)
        (fun _ => trivial)).symm) := by
    ext b
    rw [Perm.extendDomain_apply_subtype _ _ (show True from trivial)]
    simp [Equiv.permCongr_apply]
  rw [this, cycleType_extendDomain]

/-- Classical, two-type form: for `7 ≤ |α|`, an isomorphism `Alt(β) ≃* Alt(α)` is conjugation by
a bijection `β ≃ α` (so `|β| = |α|`). -/
theorem exists_equiv_of_mulEquiv {β : Type*} [Fintype β] [DecidableEq β]
    (h7 : 7 ≤ Fintype.card α) (θ : alternatingGroup β ≃* alternatingGroup α) :
    ∃ τ : β ≃ α, ∀ b : alternatingGroup β, (θ b : Perm α) = τ.permCongr b := by
  have hcard : Fintype.card β = Fintype.card α := by
    have : Nontrivial α := Fintype.one_lt_card_iff_nontrivial.1 (by omega)
    have h1 : 2 * Nat.card (alternatingGroup α) = (Fintype.card α)! := by
      rw [two_mul_nat_card_alternatingGroup, Nat.card_perm, Nat.card_eq_fintype_card]
    have h2 : Nat.card (alternatingGroup β) = Nat.card (alternatingGroup α) :=
      Nat.card_congr θ.toEquiv
    by_cases hβ : 2 ≤ Fintype.card β
    · have : Nontrivial β := Fintype.one_lt_card_iff_nontrivial.1 (by omega)
      have h3 : 2 * Nat.card (alternatingGroup β) = (Fintype.card β)! := by
        rw [two_mul_nat_card_alternatingGroup, Nat.card_perm, Nat.card_eq_fintype_card]
      exact (Nat.factorial_inj (by omega)).1 (by rw [← h3, ← h1, h2])
    · exfalso
      have h4 : Nat.card (alternatingGroup β) ≤ (Fintype.card β)! := by
        have := Subgroup.card_le_card_group (alternatingGroup β)
        rwa [Nat.card_perm, Nat.card_eq_fintype_card (α := β)] at this
      have h5 : (Fintype.card β)! ≤ 1 := by
        interval_cases h : Fintype.card β <;> simp
      have h6 : 7 ! ≤ (Fintype.card α)! := Nat.factorial_le h7
      have h7' : (7 : ℕ)! = 5040 := by decide
      omega
  obtain ⟨e₀⟩ : Nonempty (α ≃ β) := ⟨Fintype.equivOfCardEq hcard.symm⟩
  obtain ⟨σ, hσ⟩ := exists_perm_conj h7 ((e₀.altCongrHom).trans θ)
  refine ⟨e₀.symm.trans σ, fun b => ?_⟩
  have hb : e₀.altCongrHom (e₀.symm.altCongrHom b) = b := by
    ext x
    simp [Equiv.permCongr_apply]
  have := hσ (e₀.symm.altCongrHom b)
  rw [MulEquiv.trans_apply, hb] at this
  rw [this]
  ext x
  simp [Equiv.permCongr_apply]

end Jumbling.AlternatingAut
