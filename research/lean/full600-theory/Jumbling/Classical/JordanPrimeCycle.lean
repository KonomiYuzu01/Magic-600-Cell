import Mathlib.GroupTheory.GroupAction.Jordan
import Mathlib.GroupTheory.Sylow
import Mathlib.GroupTheory.Perm.Cycle.Type
import Mathlib.GroupTheory.SpecificGroups.Alternating

set_option linter.style.header false in
/-!
# Jordan's theorem for a cycle of prime length

Wielandt, *Finite permutation groups* (1964), Theorem 13.9, listed as a TODO in Mathlib's
`Mathlib/GroupTheory/GroupAction/Jordan.lean`: a primitive permutation group of degree `n` that
contains a cycle of prime length `p` with `p + 3 ≤ n` contains the alternating group. No bound
`p > n / 2` is assumed.

The proof. Let `Δ` be the support of the cycle `g` and `Γ` its complement, with `k = n - p ≥ 3`
points.

* (a) The pointwise stabilizer of `Γ` contains `g` and is transitive on `Δ` of prime degree, hence
  primitive there, so Jordan's criterion `MulAction.IsPreprimitive.isMultiplyPreprimitive` makes `G`
  `(k + 1)`-fold primitive, in particular `k`-fold transitive. So every permutation of `Γ` is
  induced on `Γ` by an element of `G` (`exists_mem_eqOn_compl_support`).
* (b) Frattini argument (`exists_conj_eq_zpow`): `⟨g⟩` is a Sylow `p`-subgroup of the pointwise
  stabilizer `K` of `Γ` (whose order divides `p!`), so an element of `G` that maps `Γ` onto itself
  can be corrected by an element of `K` into one that conjugates `g` to a power of `g`.
* (c) If `x₁, x₂` conjugate `g` to powers of `g`, then `(x₂ x₁)⁻¹ (x₁ x₂)` commutes with `g`. With
  `x₁` and `x₂` inducing the transpositions `(a b)` and `(a c)` on `Γ`, it acts on `Γ` as the
  three-cycle `((a b)(a c))²` and on `Δ` as a power of `g` (`exists_zpow_eq_on_support_of_commute`),
  so `G` contains a three-cycle, and
  `Equiv.Perm.alternatingGroup_le_of_isPreprimitive_of_isThreeCycle_mem` (Wielandt 13.3) finishes.

Two further classical results serve Theorem A of `research/jumbling/theory/buffer-transport.md`
(section 3) and the results R22B-08 and R22B-15 built on it:

* `isPreprimitive_of_isCycle_mem`, the long-prime-cycle block argument: a transitive group of degree
  `n` that contains a cycle of prime length `p > n / 2` is primitive (through Mathlib's
  `MulAction.IsPreprimitive.of_card_lt`); `alternatingGroup_le_of_isPretransitive_of_isCycle_mem`
  combines it with Jordan's theorem.
* `pow_orderOf_mul_inv`: if `c` is the only cycle of `σ` whose length is divisible by the prime `p`
  and `c` is a `p`-cycle, then for `M` the order of `σ c⁻¹` (the least common multiple of the other
  cycle lengths, `orderOf_mul_inv_eq_lcm`) the power `σ ^ M` is the cycle `c ^ M` with the support
  of `c`.
-/

namespace Jumbling.Classical

open Equiv Equiv.Perm MulAction

/-! ### Prime divisors of a least common multiple -/

/-- A prime that divides the least common multiple of a multiset of natural numbers divides one of
them. -/
theorem exists_mem_dvd_of_dvd_lcm {p : ℕ} (hp : p.Prime) {s : Multiset ℕ} (h : p ∣ s.lcm) :
    ∃ a ∈ s, p ∣ a := by
  induction s using Multiset.induction_on with
  | empty =>
    rw [Multiset.lcm_zero, Nat.dvd_one] at h
    exact absurd h hp.ne_one
  | cons a s ih =>
    rw [Multiset.lcm_cons, lcm_eq_nat_lcm] at h
    rcases (Nat.Prime.dvd_mul hp).1 (h.trans (Nat.lcm_dvd_mul a s.lcm)) with h | h
    · exact ⟨a, Multiset.mem_cons_self a s, h⟩
    · obtain ⟨b, hb, hpb⟩ := ih h
      exact ⟨b, Multiset.mem_cons_of_mem hb, hpb⟩

variable {α : Type*} [Fintype α] [DecidableEq α]

/-! ### The power that isolates a cycle of prime length -/

/-- The order of `σ c⁻¹` for a cycle `c` of `σ` is the least common multiple of the lengths of the
other cycles of `σ`. -/
theorem orderOf_mul_inv_eq_lcm {σ c : Perm α} (hc : c ∈ σ.cycleFactorsFinset) :
    orderOf (σ * c⁻¹) = (σ.cycleType.erase c.support.card).lcm := by
  rw [← lcm_cycleType, cycleType_mul_inv_mem_cycleFactorsFinset_eq_sub hc,
    (mem_cycleFactorsFinset_iff.1 hc).1.cycleType, Multiset.sub_singleton]

/-- Theorem A, step 2 (`buffer-transport.md` section 3) and R22B-08, step 1: let `c` be a cycle of
`σ` of prime length `p` such that no other cycle of `σ` has length divisible by `p`, and let `M` be
the order of `σ c⁻¹`, the least common multiple of the lengths of the other cycles
(`orderOf_mul_inv_eq_lcm`). Then `σ ^ M = c ^ M`, and `c ^ M` is a cycle with the support of `c`.
-/
theorem pow_orderOf_mul_inv {σ c : Perm α} (hc : c ∈ σ.cycleFactorsFinset)
    (hp : c.support.card.Prime)
    (hother : ∀ c' ∈ σ.cycleFactorsFinset, c' ≠ c → ¬c.support.card ∣ c'.support.card) :
    σ ^ orderOf (σ * c⁻¹) = c ^ orderOf (σ * c⁻¹) ∧ (c ^ orderOf (σ * c⁻¹)).IsCycle ∧
      (c ^ orderOf (σ * c⁻¹)).support = c.support := by
  set M := orderOf (σ * c⁻¹) with hM
  have hccyc : c.IsCycle := (mem_cycleFactorsFinset_iff.1 hc).1
  have hco : orderOf c = c.support.card := hccyc.orderOf
  -- `σ c⁻¹` commutes with `c`, and `σ = (σ c⁻¹) c`.
  have hcomm : Commute (σ * c⁻¹) c :=
    ((self_mem_cycle_factors_commute hc).mul_right (Commute.refl c).inv_right).symm
  have hpow : σ ^ M = c ^ M := by
    have hσ : σ = σ * c⁻¹ * c := by group
    rw [hσ, hcomm.mul_pow, hM, pow_orderOf_eq_one, one_mul]
  -- `p` does not divide `M`.
  have hndvd : ¬c.support.card ∣ M := by
    intro hdvd
    rw [hM, ← lcm_cycleType] at hdvd
    obtain ⟨n, hn, hpn⟩ := exists_mem_dvd_of_dvd_lcm hp hdvd
    rw [cycleType_def, Multiset.mem_map] at hn
    obtain ⟨c', hc', rfl⟩ := hn
    rw [Finset.mem_val, cycleFactorsFinset_mul_inv_mem_eq_sdiff hc, Finset.mem_sdiff,
      Finset.mem_singleton] at hc'
    exact hother c' hc'.1 hc'.2 hpn
  refine ⟨hpow, ?_, ?_⟩
  · rw [← pow_mod_orderOf, hco]
    refine hccyc.isCycle_pow_pos_of_lt_prime_order (hco ▸ hp) _ ?_ ?_
    · exact Nat.pos_of_ne_zero fun h => hndvd (Nat.dvd_of_mod_eq_zero h)
    · rw [hco]
      exact Nat.mod_lt _ hp.pos
  · refine support_pow_coprime ?_
    rw [hco]
    exact Nat.Coprime.symm ((Nat.Prime.coprime_iff_not_dvd hp).2 hndvd)

/-! ### The centralizer of a cycle -/

/-- A permutation that commutes with a cycle `g` agrees with a power of `g` on the support of `g`
(Wielandt 13.9, step (c) of the module docstring). -/
theorem exists_zpow_eq_on_support_of_commute {g c : Perm α} (hg : g.IsCycle) (hc : Commute c g) :
    ∃ j : ℤ, ∀ x ∈ g.support, c x = (g ^ j) x := by
  obtain ⟨x₀, hx₀⟩ := hg.nonempty_support
  have hx₀' : g x₀ ≠ x₀ := mem_support.1 hx₀
  have hcx₀ : g (c x₀) ≠ c x₀ := by
    intro h
    apply hx₀'
    apply c.injective
    rw [← Perm.mul_apply, hc.eq, Perm.mul_apply, h]
  obtain ⟨j, hj⟩ := hg.exists_zpow_eq hx₀' hcx₀
  refine ⟨j, fun x hx => ?_⟩
  obtain ⟨i, rfl⟩ := hg.exists_zpow_eq hx₀' (mem_support.1 hx)
  calc c ((g ^ i) x₀) = (c * g ^ i) x₀ := rfl
    _ = (g ^ i * c) x₀ := by rw [(hc.zpow_right i).eq]
    _ = (g ^ i) ((g ^ j) x₀) := by rw [Perm.mul_apply, hj]
    _ = (g ^ j) ((g ^ i) x₀) := by
      rw [← Perm.mul_apply, ← Perm.mul_apply, (Commute.zpow_zpow_self g i j).eq]

/-! ### Multiple transitivity -/

omit [Fintype α] [DecidableEq α] in
/-- A permutation that fixes a set pointwise maps its complement into itself. -/
theorem apply_notMem_of_forall_mem {s : Finset α} {σ : Perm α} (hσ : ∀ x ∈ s, σ x = x) {z : α}
    (hz : z ∉ s) : σ z ∉ s := fun h => hz (σ.injective (hσ _ h) ▸ h)

/-- The pointwise stabilizer in `G` of the complement of the support of a cycle `g ∈ G` of prime
length acts primitively on the support. -/
theorem isPreprimitive_ofFixingSubgroup_compl_support {G : Subgroup (Perm α)} {g : Perm α}
    (hgc : g.IsCycle) (hp : g.support.card.Prime) (hg : g ∈ G) :
    IsPreprimitive (fixingSubgroup G (g.support : Set α)ᶜ)
      (SubMulAction.ofFixingSubgroup G (g.support : Set α)ᶜ) := by
  classical
  have := isPretransitive_of_isCycle_mem hgc hg
  apply IsPreprimitive.of_prime_card
  convert hp
  rw [Nat.card_eq_fintype_card, Fintype.card_subtype]
  apply congr_arg
  ext x
  simp [SubMulAction.mem_ofFixingSubgroup_iff]

/-- Wielandt 13.9, step (a): by Jordan's criterion (Wielandt 13.2,
`MulAction.IsPreprimitive.isMultiplyPreprimitive`) a primitive group `G` that contains a cycle `g`
of prime length is `(n - p + 1)`-fold primitive, so for every permutation `σ` some element of `G`
agrees with `σ` on the complement of the support of `g`. -/
theorem exists_mem_eqOn_compl_support {G : Subgroup (Perm α)} (hG : IsPreprimitive G α)
    {g : Perm α} (hgc : g.IsCycle) (hp : g.support.card.Prime) (hg : g ∈ G) (σ : Perm α) :
    ∃ y ∈ G, ∀ z ∉ g.support, y z = σ z := by
  set Γ : Finset α := g.supportᶜ with hΓ
  have hΓcard : Γ.card + g.support.card = Fintype.card α := by
    rw [hΓ, Finset.card_compl]
    have := Finset.card_le_univ g.support
    omega
  rcases Nat.eq_zero_or_pos Γ.card with h0 | hpos
  · refine ⟨1, G.one_mem, fun z hz => absurd (Finset.mem_compl.2 hz) ?_⟩
    rw [← hΓ, Finset.card_eq_zero.1 h0]
    exact Finset.notMem_empty z
  obtain ⟨n, hn⟩ : ∃ n, Γ.card = n + 1 := ⟨Γ.card - 1, by omega⟩
  have hp2 := hp.two_le
  have hsn : ((g.support : Set α)ᶜ).ncard = n + 1 := by
    rw [← Finset.coe_compl, Set.ncard_coe_finset, ← hΓ, hn]
  have hsn' : n + 2 < Nat.card α := by
    rw [Nat.card_eq_fintype_card]
    omega
  have := hG.isMultiplyPreprimitive hsn hsn'
    (isPreprimitive_ofFixingSubgroup_compl_support hgc hp hg)
  have : IsMultiplyPretransitive G α (n + 1) :=
    isMultiplyPretransitive_of_le (n := n + 2) (Nat.le_succ _) hsn'.le
  -- Enumerate `Γ`, and move the enumeration by `σ`.
  let e : Fin (n + 1) ↪ α :=
    (Γ.equivFinOfCardEq hn).symm.toEmbedding.trans (Function.Embedding.subtype _)
  let e' : Fin (n + 1) ↪ α := e.trans σ.toEmbedding
  obtain ⟨y, hy⟩ := MulAction.exists_smul_eq G e e'
  refine ⟨y, y.2, fun z hz => ?_⟩
  have hzΓ : z ∈ Γ := Finset.mem_compl.2 hz
  set i := Γ.equivFinOfCardEq hn ⟨z, hzΓ⟩
  have hei : e i = z := by simp [e, i]
  have := congrArg (fun f : Fin (n + 1) ↪ α => f i) hy
  simp only [smul_apply, hei, e', Function.Embedding.trans_apply,
    Equiv.coe_toEmbedding] at this
  rwa [Subgroup.smul_def, Perm.smul_def] at this

/-! ### The Frattini argument -/

/-- Wielandt 13.9, step (b), the Frattini argument: let `g ∈ G` be a cycle of prime length `p`. If
`y ∈ G` maps the support of `g` onto itself, some `x ∈ G` agrees with `y` outside the support of `g`
and conjugates `g` to a power of `g`. The cyclic group `⟨g⟩` is a Sylow `p`-subgroup of the
pointwise stabilizer `K` of the complement of the support, since `|K|` divides `p!`, and the Sylow
subgroups `⟨g⟩` and `⟨y g y⁻¹⟩` of `K` are conjugate in `K`. -/
theorem exists_conj_eq_zpow {G : Subgroup (Perm α)} {g : Perm α} (hgc : g.IsCycle)
    (hp : g.support.card.Prime) (hg : g ∈ G) {y : Perm α} (hyG : y ∈ G)
    (hy : ∀ x, y x ∈ g.support ↔ x ∈ g.support) :
    ∃ x ∈ G, (∃ e : ℤ, x * g * x⁻¹ = g ^ e) ∧ ∀ z ∉ g.support, x z = y z := by
  classical
  set p := g.support.card with hpdef
  have : Fact p.Prime := ⟨hp⟩
  -- The elements of `G` that fix every point outside the support of `g`.
  let K : Subgroup (Perm α) := G ⊓ fixingSubgroup (Perm α) ((g.support : Set α)ᶜ)
  have memK : ∀ {σ : Perm α}, σ ∈ K ↔ σ ∈ G ∧ ∀ z ∉ g.support, σ z = z := by
    intro σ
    simp only [K, Subgroup.mem_inf, mem_fixingSubgroup_iff, Set.mem_compl_iff, Finset.mem_coe,
      Perm.smul_def]
  -- `|K|` divides `p!`: restriction to the support is injective on `K`.
  have hKdvd : Nat.card K ∣ p.factorial := by
    have hle : K ≤ (Perm.ofSubtype : Perm g.support →* Perm α).range := by
      intro σ hσ
      have h₂ : ∀ x, σ x ≠ x → x ∈ g.support := fun x hx => by
        by_contra h
        exact hx ((memK.1 hσ).2 x h)
      have h₁ : ∀ x, σ x ∈ g.support ↔ x ∈ g.support := fun x => by
        refine ⟨fun h => ?_, fun h => ?_⟩
        · by_contra hx
          exact hx (((memK.1 hσ).2 x hx) ▸ h)
        · by_contra hx
          have := σ.injective ((memK.1 hσ).2 _ hx)
          rw [this] at hx
          exact hx h
      exact ⟨σ.subtypePerm h₁, ofSubtype_subtypePerm h₁ h₂⟩
    have h := Subgroup.card_dvd_of_le hle
    rwa [← Nat.card_congr (MonoidHom.ofInjective ofSubtype_injective).toEquiv, Nat.card_perm,
      Nat.card_eq_finsetCard] at h
  -- A cyclic subgroup of `K` of order `p` is a Sylow subgroup.
  have hnot : ∀ k : K, orderOf k = p → ¬p ∣ (Subgroup.zpowers k).index := by
    intro k hk hdvd
    have h1 := (Subgroup.zpowers k).index_mul_card
    rw [Nat.card_zpowers, hk] at h1
    obtain ⟨m, hm⟩ := hdvd
    have hpp : p * p ∣ p.factorial :=
      dvd_trans ⟨m, by rw [← h1, hm]; ring⟩ hKdvd
    rw [← Nat.mul_factorial_pred hp.ne_zero] at hpp
    have := (Nat.Prime.dvd_factorial hp).1 (Nat.dvd_of_mul_dvd_mul_left hp.pos hpp)
    have := hp.two_le
    omega
  let S : ∀ k : K, orderOf k = p → Sylow p K := fun k hk =>
    (IsPGroup.of_card (n := 1) (by rw [Nat.card_zpowers, hk, pow_one])).toSylow (hnot k hk)
  have hg₀ : g ∈ K := memK.2 ⟨hg, fun z hz => notMem_support.1 hz⟩
  have hg₁ : y * g * y⁻¹ ∈ K := by
    refine memK.2 ⟨G.mul_mem (G.mul_mem hyG hg) (G.inv_mem hyG), fun z hz => ?_⟩
    have hz' : y⁻¹ z ∉ g.support := fun h => hz (by simpa using (hy (y⁻¹ z)).2 h)
    rw [Perm.mul_apply, Perm.mul_apply, notMem_support.1 hz']
    simp
  have ho₀ : orderOf (⟨g, hg₀⟩ : K) = p := by rw [Subgroup.orderOf_mk, hgc.orderOf]
  have ho₁ : orderOf (⟨y * g * y⁻¹, hg₁⟩ : K) = p := by
    rw [Subgroup.orderOf_mk, hgc.conj.orderOf, card_support_conj]
  obtain ⟨k, hk⟩ := MulAction.exists_smul_eq K (S _ ho₁) (S _ ho₀)
  have hmem : k * ⟨y * g * y⁻¹, hg₁⟩ * k⁻¹ ∈ Subgroup.zpowers (⟨g, hg₀⟩ : K) := by
    have h1 : (⟨y * g * y⁻¹, hg₁⟩ : K) ∈ (S _ ho₁ : Subgroup K) := Subgroup.mem_zpowers _
    have h2 := Subgroup.smul_mem_pointwise_smul _ (MulAut.conj k) _ h1
    rw [← Sylow.coe_subgroup_smul, hk] at h2
    have h3 : k * ⟨y * g * y⁻¹, hg₁⟩ * k⁻¹ ∈ (S _ ho₀ : Subgroup K) := by simpa using h2
    exact h3
  obtain ⟨e, he⟩ := Subgroup.mem_zpowers_iff.1 hmem
  refine ⟨(k : Perm α) * y, G.mul_mem (memK.1 k.2).1 hyG, ⟨e, ?_⟩, fun z hz => ?_⟩
  · have := congrArg Subtype.val he
    simp only [Subgroup.coe_zpow, Subgroup.coe_mul, Subgroup.coe_inv] at this
    rw [this]
    group
  · rw [Perm.mul_apply]
    exact (memK.1 k.2).2 _ fun h => hz ((hy z).1 h)

/-! ### Jordan's theorem -/

/-- **Jordan's theorem for a cycle of prime length** (Wielandt, *Finite permutation groups*,
Theorem 13.9; a TODO in Mathlib's `Mathlib/GroupTheory/GroupAction/Jordan.lean`): a primitive
subgroup `G` of `Perm α` that contains a cycle `g` whose support has prime cardinality `p` with
`p + 3 ≤ |α|` contains the alternating group. -/
theorem alternatingGroup_le_of_isPreprimitive_of_isCycle_mem {G : Subgroup (Perm α)}
    (hG : IsPreprimitive G α) {g : Perm α} (hgc : g.IsCycle) (hp : g.support.card.Prime)
    (hg : g ∈ G) (hcard : g.support.card + 3 ≤ Fintype.card α) : alternatingGroup α ≤ G := by
  -- Three distinct points outside the support of `g`.
  have hΓ : 2 < (g.supportᶜ).card := by
    rw [Finset.card_compl]
    omega
  obtain ⟨a, b, c, ha, hb, hc, hab, hac, hbc⟩ := Finset.two_lt_card_iff.1 hΓ
  rw [Finset.mem_compl] at ha hb hc
  have hfix : ∀ {u v : α}, u ∉ g.support → v ∉ g.support → ∀ x ∈ g.support, swap u v x = x :=
    fun hu hv x hx => swap_apply_of_ne_of_ne (fun h => hu (h ▸ hx)) (fun h => hv (h ▸ hx))
  -- An element of `G` inducing `σ` outside the support maps the support onto itself.
  have hpres : ∀ {y σ : Perm α}, (∀ x ∈ g.support, σ x = x) → (∀ z ∉ g.support, y z = σ z) →
      ∀ x, y x ∈ g.support ↔ x ∈ g.support := by
    intro y σ hσ hy x
    refine ⟨fun h => ?_, fun h => ?_⟩
    · by_contra hx
      exact apply_notMem_of_forall_mem hσ hx (hy x hx ▸ h)
    · by_contra hyx
      have hw : σ⁻¹ (y x) ∉ g.support :=
        apply_notMem_of_forall_mem (fun z hz => by rw [Perm.inv_eq_iff_eq, hσ z hz]) hyx
      have h1 : y (σ⁻¹ (y x)) = y x := by rw [hy _ hw]; simp
      rw [y.injective h1] at hw
      exact hw h
  obtain ⟨y₁, hy₁G, hy₁⟩ := exists_mem_eqOn_compl_support hG hgc hp hg (swap a b)
  obtain ⟨y₂, hy₂G, hy₂⟩ := exists_mem_eqOn_compl_support hG hgc hp hg (swap a c)
  obtain ⟨x₁, hx₁G, ⟨e₁, he₁⟩, hx₁⟩ :=
    exists_conj_eq_zpow hgc hp hg hy₁G (hpres (hfix ha hb) hy₁)
  obtain ⟨x₂, hx₂G, ⟨e₂, he₂⟩, hx₂⟩ :=
    exists_conj_eq_zpow hgc hp hg hy₂G (hpres (hfix ha hc) hy₂)
  -- `d₀ = (x₂ x₁)⁻¹ (x₁ x₂)` commutes with `g`.
  set u := x₁ * x₂ with hu
  set v := x₂ * x₁ with hv
  have huv : u * g * u⁻¹ = v * g * v⁻¹ := by
    have hu' : u * g * u⁻¹ = x₁ * (x₂ * g * x₂⁻¹) * x₁⁻¹ := by rw [hu]; group
    have hv' : v * g * v⁻¹ = x₂ * (x₁ * g * x₁⁻¹) * x₂⁻¹ := by rw [hv]; group
    rw [hu', hv', he₁, he₂, ← conj_zpow, ← conj_zpow, he₁, he₂, ← zpow_mul, ← zpow_mul, mul_comm]
  have hcomm : Commute (v⁻¹ * u) g := by
    change v⁻¹ * u * g = g * (v⁻¹ * u)
    calc v⁻¹ * u * g = v⁻¹ * (u * g * u⁻¹) * u := by group
      _ = v⁻¹ * (v * g * v⁻¹) * u := by rw [huv]
      _ = g * (v⁻¹ * u) := by group
  obtain ⟨j, hj⟩ := exists_zpow_eq_on_support_of_commute hgc hcomm
  -- `d = g⁻ʲ d₀` is the three-cycle `((a b)(a c))²`.
  set t := swap a b * swap a c with ht
  have hd : (g ^ j)⁻¹ * (v⁻¹ * u) = t * t := by
    ext z
    by_cases hz : z ∈ g.support
    · change (g ^ j)⁻¹ ((v⁻¹ * u) z) = t (t z)
      rw [hj z hz, ← Perm.mul_apply, inv_mul_cancel, Perm.one_apply, ht]
      simp only [Perm.mul_apply]
      rw [hfix ha hc z hz, hfix ha hb z hz, hfix ha hc z hz, hfix ha hb z hz]
    · have hnot : ∀ {w : α}, w ∉ g.support → swap a b w ∉ g.support ∧ swap a c w ∉ g.support :=
        fun hw => ⟨apply_notMem_of_forall_mem (hfix ha hb) hw,
          apply_notMem_of_forall_mem (hfix ha hc) hw⟩
      -- `u z = (a b) (a c) z`.
      have huz : u z = t z := by
        rw [hu, Perm.mul_apply, hx₂ z hz, hy₂ z hz, hx₁ _ (hnot hz).2, hy₁ _ (hnot hz).2, ht,
          Perm.mul_apply]
      -- `v⁻¹ w = (a b) (a c) w` for `w` outside the support.
      have hvinv : ∀ w ∉ g.support, v⁻¹ w = t w := by
        intro w hw
        rw [Perm.inv_eq_iff_eq, hv, Perm.mul_apply, ht, Perm.mul_apply]
        have h1 := (hnot hw).2
        have h2 := (hnot h1).1
        rw [hx₁ _ h2, hy₁ _ h2, hx₂ _ (hnot h2).1, hy₂ _ (hnot h2).1, swap_apply_self,
          swap_apply_self]
      have htz : t z ∉ g.support := (hnot (hnot hz).2).1
      have htz' : t (t z) ∉ g.support := (hnot (hnot htz).2).1
      change (g ^ j)⁻¹ (v⁻¹ (u z)) = t (t z)
      rw [huz, hvinv _ htz, Perm.inv_eq_iff_eq,
        zpow_apply_eq_self_of_apply_eq_self (notMem_support.1 htz')]
  have h3 : (t * t).IsThreeCycle := by
    have ht3 : t.IsThreeCycle := isThreeCycle_swap_mul_swap_same hab hac hbc
    have : t * t = t⁻¹ := by
      rw [eq_inv_iff_mul_eq_one, ← pow_three', ← ht3.orderOf, pow_orderOf_eq_one]
    rw [this]
    exact ht3.inv
  refine alternatingGroup_le_of_isPreprimitive_of_isThreeCycle_mem hG h3 ?_
  rw [← hd]
  exact G.mul_mem (G.inv_mem (G.zpow_mem hg j))
    (G.mul_mem (G.inv_mem (G.mul_mem hx₂G hx₁G)) (G.mul_mem hx₁G hx₂G))

/-! ### The long-prime-cycle block argument -/

/-- The long-prime-cycle block argument of Theorem A (`buffer-transport.md` section 3, step 3;
used by R22B-08, step 2, and R22B-15): a transitive permutation group of degree `n` that contains a
cycle of prime length `p` with `n < 2p` is primitive. A block with two points that meets the support
of the cycle is mapped onto itself by it (otherwise its `p` translates would be disjoint), so it has
more than `n / 2` points and is everything; here this goes through Mathlib's
`MulAction.IsPreprimitive.of_card_lt`, since the pointwise stabilizer of the complement of the
support is primitive on the support. -/
theorem isPreprimitive_of_isCycle_mem {G : Subgroup (Perm α)} [IsPretransitive G α] {g : Perm α}
    (hgc : g.IsCycle) (hp : g.support.card.Prime) (hg : g ∈ G)
    (hlong : Fintype.card α < 2 * g.support.card) : IsPreprimitive G α := by
  have := isPreprimitive_ofFixingSubgroup_compl_support hgc hp hg
  apply IsPreprimitive.of_card_lt
    (f := SubMulAction.ofFixingSubgroup_equivariantMap G (g.support : Set α)ᶜ)
  have hrange : Set.range (SubMulAction.ofFixingSubgroup_equivariantMap G (g.support : Set α)ᶜ) =
      (g.support : Set α) := by
    ext x
    constructor
    · rintro ⟨y, rfl⟩
      have hy : (y : α) ∉ (g.support : Set α)ᶜ := y.2
      change (y : α) ∈ (g.support : Set α)
      simpa using hy
    · intro hx
      have hx' : x ∉ (g.support : Set α)ᶜ := by simpa using hx
      exact ⟨⟨x, hx'⟩, rfl⟩
  rw [hrange, Set.ncard_coe_finset, Nat.card_eq_fintype_card]
  exact hlong

/-- Theorem A, steps 3 and 4 (`buffer-transport.md` section 3): a transitive permutation group of
degree `n` that contains a cycle of prime length `p` with `n / 2 < p ≤ n - 3` contains the
alternating group. -/
theorem alternatingGroup_le_of_isPretransitive_of_isCycle_mem {G : Subgroup (Perm α)}
    [IsPretransitive G α] {g : Perm α} (hgc : g.IsCycle) (hp : g.support.card.Prime) (hg : g ∈ G)
    (hlong : Fintype.card α < 2 * g.support.card)
    (hshort : g.support.card + 3 ≤ Fintype.card α) : alternatingGroup α ≤ G :=
  alternatingGroup_le_of_isPreprimitive_of_isCycle_mem
    (isPreprimitive_of_isCycle_mem hgc hp hg hlong) hgc hp hg hshort

end Jumbling.Classical
