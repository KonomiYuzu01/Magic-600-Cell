import Jumbling.Classical.AlternatingAut
import Jumbling.Group.Words
import Mathlib.GroupTheory.GroupAction.SubMulAction.Combination
import Mathlib.GroupTheory.SpecificGroups.Alternating.Simple

set_option linter.style.header false in
/-!
# R22B-19: local Alt and separating words give a product of alternating groups

`PROOFS-B.md` section 3, round 4 of line (b), R22B-19 (layer "group" of `ARCHON-HANDOFF.md`).

Setting: a permutation group `G` of the chambers `Ω` (for the SAFE-ALL letters, `A.group`), finite
sets `Δ i` that `G` maps into themselves (`IsStable`; orbits in the source), and the group `G|Δ i`
that `G` induces on `Δ i` (`restrict`).

* `productAlt` (R22B-19, statement): H1 (every `G|Δ i` contains `Alt(Δ i)`), H2 (for `i ≠ j` with
  `|Δ i| = |Δ j|` some element has different cycle types on `Δ i` and `Δ j`) and `|Δ i| ≥ 7` give,
  for every `i` and every even permutation `σ` of `Δ i`, an element of `G` that acts as `σ` on
  `Δ i` and as the identity on every other `Δ j`; `productAlt_pi`: one element acts as any given
  even permutation on every `Δ i` at once, so the group induced on the union contains
  `∏ Alt(Δ i)`. The element need not fix chambers outside the union.
* `Alphabet.vacancy_of_productAlt` (consequence for vacancy): if every orbit of at least two
  chambers that meets the target is one of these orbits, N2 (hence N1) gives a vacating word.
* `CyclicFour.counterexample`: the size condition cannot be dropped.

Proof of `productAlt`. The source passes to `D = [H, H]` and the structure of subdirect products of
nonabelian simple groups. The formal proof is an induction on the set `J` of indices on which the
elements must act trivially (`alt_le_map_trivialOn`): the elements of `G` that act trivially on
every `Δ j`, `j ∈ J`, still induce a group containing `Alt(Δ i)` on each `Δ i`, `i ∉ J`. The step
is `alt_le_or_coupled`, for a normal subgroup `K` that induces groups containing `Alt(X)` and
`Alt(Y)` through homomorphisms `f` and `g`: with `M = K ∩ ker g`, the trace of `f(M)` in the simple
group `Alt(X)` is all of it (the step succeeds) or trivial; then `f(M)` centralizes `Alt(X)`, so
`f` kills `M` (trivial centralizer), and `f` induces a homomorphism `Alt(Y) → Sym(X)` that is an
isomorphism onto `Alt(X)` (simplicity on both sides). By `Aut(Alt(d)) = Sym(d)` for `d ≥ 7`
(`Jumbling.AlternatingAut.exists_equiv_of_mulEquiv`) it is conjugation by a bijection `τ`, and the
trivial centralizer extends `f = τ·g·τ⁻¹` to all of `G`; equal cycle types then contradict H2.

H1 (in the source, R22B-15's local test or a global certificate) and H2 are hypotheses; R22B-15 is
not used. The size bound is `|Δ i| ≥ 7`, weaker than the source's `d ≥ 8`; the Δ i need not be
orbits or disjoint for `productAlt`.
-/

open Equiv Equiv.Perm Subgroup
open scoped Nat

namespace Jumbling

namespace ProductAlt

/-- A square is even. -/
theorem sq_mem_alternatingGroup {Z : Type*} [Fintype Z] [DecidableEq Z] (σ : Perm Z) :
    σ ^ 2 ∈ alternatingGroup Z := by
  rw [mem_alternatingGroup, map_pow, Int.units_sq]

/-- `Alt(Z)` has at least three elements when `5 ≤ |Z|`. -/
theorem three_le_nat_card_alternatingGroup {Z : Type*} [Fintype Z] [DecidableEq Z]
    (h : 5 ≤ Fintype.card Z) : 3 ≤ Nat.card (alternatingGroup Z) := by
  have : Nontrivial Z := Fintype.one_lt_card_iff_nontrivial.1 (by omega)
  have h1 : 2 * Nat.card (alternatingGroup Z) = (Fintype.card Z)! := by
    rw [two_mul_nat_card_alternatingGroup, Nat.card_perm, Nat.card_eq_fintype_card]
  have h2 : 5 ! ≤ (Fintype.card Z)! := Nat.factorial_le h
  have h3 : (5 : ℕ)! = 120 := by decide
  omega

section Key

variable {G : Type*} [Group G] {X Y : Type*} [Fintype X] [DecidableEq X] [Fintype Y]
  [DecidableEq Y]

/-- R22B-19, the two-coordinate step of the proof. Let `f : G → Sym(X)` and `g : G → Sym(Y)` be
homomorphisms, `7 ≤ |X|`, `5 ≤ |Y|`, and `K` a normal subgroup of `G` with `Alt(X) ≤ f(K)` and
`Alt(Y) ≤ g(K)`. Then either the elements of `K` that act trivially through `g` still give
`Alt(X) ≤ f(K ∩ ker g)`, or the two actions are coupled: `f x = τ·(g x)·τ⁻¹` for a bijection
`τ : Y ≃ X` and every `x ∈ G` (a diagonal). -/
theorem alt_le_or_coupled (hX : 7 ≤ Fintype.card X) (hY : 5 ≤ Fintype.card Y)
    (f : G →* Perm X) (g : G →* Perm Y) (K : Subgroup G) [K.Normal]
    (hf : alternatingGroup X ≤ K.map f) (hg : alternatingGroup Y ≤ K.map g) :
    alternatingGroup X ≤ (K ⊓ g.ker).map f ∨ ∃ τ : Y ≃ X, ∀ x, f x = τ.permCongr (g x) := by
  have hX5 : 5 ≤ Nat.card X := by rw [Nat.card_eq_fintype_card]; omega
  have hY5 : 5 ≤ Nat.card Y := by rw [Nat.card_eq_fintype_card]; omega
  have hAn : (alternatingGroup X).Normal := inferInstance
  set M := K ⊓ g.ker with hM
  have hMn : M.Normal := Subgroup.normal_inf_normal _ _
  let N' : Subgroup (alternatingGroup X) := (M.map f).subgroupOf (alternatingGroup X)
  have hN' : N'.Normal := ⟨fun n hn a => by
    rw [Subgroup.mem_subgroupOf] at hn ⊢
    obtain ⟨m, hm, hmn⟩ := Subgroup.mem_map.1 hn
    obtain ⟨k, hk, hka⟩ := Subgroup.mem_map.1 (hf a.2)
    refine Subgroup.mem_map.2 ⟨k * m * k⁻¹, hMn.conj_mem m hm k, ?_⟩
    simp [hmn, hka]⟩
  rcases alternatingGroup.normal_subgroup_eq_bot_or_eq_top hX5 (N := N') with hbot | htop
  swap
  · left
    intro a ha
    have : (⟨a, ha⟩ : alternatingGroup X) ∈ N' := by rw [htop]; trivial
    exact Subgroup.mem_subgroupOf.1 this
  right
  -- Step 2: `f` kills `M`.
  have hMf : ∀ m ∈ M, f m = 1 := by
    intro m hm
    have hcen : f m ∈ centralizer (alternatingGroup X : Set (Perm X)) := by
      rw [mem_centralizer_iff]
      intro a ha
      obtain ⟨k, hk, hka⟩ := Subgroup.mem_map.1 (hf ha)
      have hc : f m * a * (f m)⁻¹ * a⁻¹ ∈ alternatingGroup X :=
        mul_mem (hAn.conj_mem a ha (f m)) (inv_mem ha)
      have hcM : f m * a * (f m)⁻¹ * a⁻¹ ∈ M.map f := by
        refine Subgroup.mem_map.2 ⟨m * (k * m⁻¹ * k⁻¹),
          mul_mem hm (hMn.conj_mem _ (inv_mem hm) k), ?_⟩
        simp [hka, mul_assoc]
      have hN : (⟨_, hc⟩ : alternatingGroup X) ∈ N' := Subgroup.mem_subgroupOf.2 hcM
      rw [hbot, Subgroup.mem_bot] at hN
      have h1 : f m * a * (f m)⁻¹ * a⁻¹ = 1 := congrArg Subtype.val hN
      calc a * f m = (f m * a * (f m)⁻¹ * a⁻¹)⁻¹ * (f m * a) := by group
        _ = f m * a := by rw [h1, inv_one, one_mul]
    rwa [AlternatingAut.centralizer_alternatingGroup_eq_bot (by omega), Subgroup.mem_bot] at hcen
  -- Step 3: the induced map `θ : Alt(Y) → Sym(X)`.
  set K' := K ⊓ (alternatingGroup Y).comap g with hK'
  have hK'n : K'.Normal := Subgroup.normal_inf_normal _ _
  let gK : K' →* alternatingGroup Y :=
    (g.comp K'.subtype).codRestrict _ (fun k => (Subgroup.mem_inf.1 k.2).2)
  let fK : K' →* Perm X := f.comp K'.subtype
  have hsurj : Function.Surjective gK := by
    intro b
    obtain ⟨k, hk, hkb⟩ := Subgroup.mem_map.1 (hg b.2)
    exact ⟨⟨k, Subgroup.mem_inf.2 ⟨hk, by rw [Subgroup.mem_comap, hkb]; exact b.2⟩⟩,
      Subtype.ext hkb⟩
  have hker : gK.ker ≤ fK.ker := by
    intro k hk
    rw [MonoidHom.mem_ker] at hk ⊢
    have : g k = 1 := congrArg Subtype.val hk
    exact hMf k (Subgroup.mem_inf.2 ⟨(Subgroup.mem_inf.1 k.2).1, this⟩)
  let θ : alternatingGroup Y →* Perm X :=
    gK.liftOfRightInverse (Function.surjInv hsurj) (Function.rightInverse_surjInv hsurj)
      ⟨fK, hker⟩
  have hθ : ∀ k : K', θ (gK k) = f k := fun k =>
    gK.liftOfRightInverse_comp_apply _ _ ⟨fK, hker⟩ k
  -- Step 4: `θ` is injective.
  have hθinj : Function.Injective θ := by
    rcases alternatingGroup.normal_subgroup_eq_bot_or_eq_top hY5 (N := θ.ker) with h | h
    · exact (MonoidHom.ker_eq_bot_iff θ).1 h
    · exfalso
      have hsq : ∀ a ∈ alternatingGroup X, a ^ 2 = 1 := by
        intro a ha
        obtain ⟨k, hk, hka⟩ := Subgroup.mem_map.1 (hf ha)
        have hk2 : k ^ 2 ∈ K' := Subgroup.mem_inf.2 ⟨pow_mem hk 2, by
          rw [Subgroup.mem_comap, map_pow]; exact sq_mem_alternatingGroup _⟩
        have h1 : θ (gK ⟨k ^ 2, hk2⟩) = 1 := by
          rw [← MonoidHom.mem_ker, h]
          trivial
        rw [hθ] at h1
        rw [← hka, ← map_pow]
        exact h1
      obtain ⟨x, -⟩ := AlternatingAut.exists_not_mem (s := (∅ : Finset X)) (by simp; omega)
      obtain ⟨y, hy⟩ := AlternatingAut.exists_not_mem (s := {x}) (by simp; omega)
      obtain ⟨z, hz⟩ := AlternatingAut.exists_not_mem (s := {x, y})
        (lt_of_le_of_lt Finset.card_le_two (by omega))
      simp only [Finset.mem_insert, Finset.mem_singleton, not_or] at hy hz
      have hxy : x ≠ y := Ne.symm hy
      have h3 := AlternatingAut.star_isThreeCycle hxy (Ne.symm hz.1) (Ne.symm hz.2)
      have h2 := hsq _ (AlternatingAut.star_mem hxy (Ne.symm hz.2))
      have := orderOf_dvd_of_pow_eq_one h2
      rw [h3.orderOf] at this
      omega
  -- Step 5: `θ` lands in `Alt(X)`.
  have hθalt : ∀ b, θ b ∈ alternatingGroup X := by
    rcases alternatingGroup.normal_subgroup_eq_bot_or_eq_top hY5 (N := (sign.comp θ).ker) with
      h | h
    · exfalso
      have hinj := (MonoidHom.ker_eq_bot_iff _).1 h
      have h1 := Nat.card_le_card_of_injective _ hinj
      have h2 : Nat.card ℤˣ = 2 := by rw [Nat.card_eq_fintype_card, Fintype.card_units_int]
      have h3 := three_le_nat_card_alternatingGroup hY
      omega
    · intro b
      have : b ∈ (sign.comp θ).ker := h ▸ Subgroup.mem_top b
      exact this
  -- Step 6: `θ` maps onto `Alt(X)`.
  let R : Subgroup (alternatingGroup X) := θ.range.subgroupOf (alternatingGroup X)
  have hRn : R.Normal := ⟨fun r hr a => by
    rw [Subgroup.mem_subgroupOf] at hr ⊢
    obtain ⟨b, hb⟩ := hr
    obtain ⟨k', rfl⟩ := hsurj b
    obtain ⟨k, hk, hka⟩ := Subgroup.mem_map.1 (hf a.2)
    refine ⟨gK ⟨k * k' * k⁻¹, hK'n.conj_mem _ k'.2 k⟩, ?_⟩
    rw [hθ]
    rw [hθ] at hb
    simp [← hb, hka]⟩
  have hRtop : R = ⊤ := by
    rcases alternatingGroup.normal_subgroup_eq_bot_or_eq_top hX5 (N := R) with h | h
    · exfalso
      have : Nontrivial (alternatingGroup Y) :=
        alternatingGroup.nontrivial_of_three_le_card (by omega)
      obtain ⟨b, hb⟩ := exists_ne (1 : alternatingGroup Y)
      have hR : (⟨θ b, hθalt b⟩ : alternatingGroup X) ∈ R := Subgroup.mem_subgroupOf.2 ⟨b, rfl⟩
      rw [h, Subgroup.mem_bot] at hR
      exact hb (hθinj ((congrArg Subtype.val hR).trans (map_one θ).symm))
    · exact h
  let θc : alternatingGroup Y →* alternatingGroup X := θ.codRestrict _ hθalt
  have hbij : Function.Bijective θc := by
    refine ⟨fun b b' h => hθinj (congrArg Subtype.val h), fun a => ?_⟩
    have : a ∈ R := hRtop ▸ Subgroup.mem_top a
    obtain ⟨b, hb⟩ := Subgroup.mem_subgroupOf.1 this
    exact ⟨b, Subtype.ext hb⟩
  -- Step 7: `θ` is conjugation by a bijection `τ`.
  obtain ⟨τ, hτ⟩ := AlternatingAut.exists_equiv_of_mulEquiv hX (MulEquiv.ofBijective θc hbij)
  have hθτ : ∀ b, θ b = τ.permCongr b := fun b => hτ b
  -- Step 8: `f = τ g τ⁻¹` on all of `G`.
  refine ⟨τ, fun x => ?_⟩
  set P := τ.permCongr (g x) with hP
  have hcen : P⁻¹ * f x ∈ centralizer (alternatingGroup X : Set (Perm X)) := by
    rw [mem_centralizer_iff]
    intro a ha
    have hb : τ.symm.permCongr a ∈ alternatingGroup Y := by
      rw [mem_alternatingGroup, sign_permCongr]
      exact ha
    obtain ⟨k', hk'⟩ := hsurj ⟨_, hb⟩
    have hgk' : g k' = τ.symm.permCongr a := congrArg Subtype.val hk'
    have hfk' : f k' = a := by
      rw [← hθ, hk', hθτ]
      ext t
      simp [Equiv.permCongr_apply]
    have hxk : x * k' * x⁻¹ ∈ K' := hK'n.conj_mem _ k'.2 x
    have h1 := hθ ⟨x * k' * x⁻¹, hxk⟩
    rw [hθτ] at h1
    have h2 : f x * a * (f x)⁻¹ = P * a * P⁻¹ := by
      calc f x * a * (f x)⁻¹ = f (x * k' * x⁻¹) := by simp [hfk']
        _ = τ.permCongr (g (x * k' * x⁻¹)) := h1.symm
        _ = P * a * P⁻¹ := by
          rw [show g (x * k' * x⁻¹) = g x * g k' * (g x)⁻¹ by simp, hgk']
          ext t
          simp only [P, Perm.coe_mul, Function.comp_apply, Equiv.permCongr_apply]
          rfl
    calc a * (P⁻¹ * f x) = P⁻¹ * (P * a * P⁻¹) * f x := by group
      _ = P⁻¹ * (f x * a * (f x)⁻¹) * f x := by rw [h2]
      _ = P⁻¹ * f x * a := by group
  rw [AlternatingAut.centralizer_alternatingGroup_eq_bot (by omega), Subgroup.mem_bot] at hcen
  exact (inv_mul_eq_one.1 hcen).symm

end Key


/-! ### The group induced on an invariant set -/

section Restrict

variable {Ω : Type*}

/-- A finite set of chambers that every element of the group `G` maps into itself, such as an
orbit or a union of orbits. -/
def IsStable (G : Subgroup (Perm Ω)) (Δ : Finset Ω) : Prop :=
  ∀ g ∈ G, ∀ x ∈ Δ, g x ∈ Δ

/-- A stable set is mapped onto itself. -/
theorem IsStable.apply_mem_iff {G : Subgroup (Perm Ω)} {Δ : Finset Ω} (h : IsStable G Δ)
    {g : Perm Ω} (hg : g ∈ G) (x : Ω) : g x ∈ Δ ↔ x ∈ Δ :=
  ⟨fun hx => by simpa using h g⁻¹ (inv_mem hg) _ hx, h g hg x⟩

/-- The permutation group `G|Δ` that `G` induces on a stable set `Δ`, as a homomorphism. -/
def restrict {G : Subgroup (Perm Ω)} {Δ : Finset Ω} (h : IsStable G Δ) : G →* Perm Δ where
  toFun g := (g : Perm Ω).subtypePerm (h.apply_mem_iff g.2)
  map_one' := by ext; rfl
  map_mul' g g' := by ext; rfl

/-- `restrict` acts as the permutation itself. -/
@[simp]
theorem restrict_apply {G : Subgroup (Perm Ω)} {Δ : Finset Ω} (h : IsStable G Δ) (g : G) (x : Δ) :
    (restrict h g x : Ω) = (g : Perm Ω) x :=
  rfl

/-- An element induces the identity on `Δ` exactly when it fixes `Δ` pointwise. -/
theorem restrict_eq_one_iff {G : Subgroup (Perm Ω)} {Δ : Finset Ω} (h : IsStable G Δ) (g : G) :
    restrict h g = 1 ↔ ∀ x ∈ Δ, (g : Perm Ω) x = x := by
  constructor
  · intro hg x hx
    have := congrArg (fun π : Perm Δ => (π ⟨x, hx⟩ : Ω)) hg
    simpa using this
  · intro hg
    ext x
    simp [hg x x.2]

/-- R22B-19 and R22B-25: independent factors combine. If for each `i ∈ s` some element of `G` acts
as `π i` on `Δ i` and fixes every other `Δ j` pointwise, then one element acts as `π i` on every
`Δ i`, `i ∈ s`, and fixes the `Δ j`, `j ∉ s`, pointwise. -/
theorem exists_forall_of_factors {ι : Type*} {G : Subgroup (Perm Ω)}
    {Δ : ι → Finset Ω} (π : ∀ i, Perm (Δ i)) (s : Finset ι)
    (h : ∀ i ∈ s, ∃ g ∈ G, (∀ x : Δ i, g x = π i x) ∧ ∀ j ≠ i, ∀ x ∈ Δ j, g x = x) :
    ∃ g ∈ G, (∀ i ∈ s, ∀ x : Δ i, g x = π i x) ∧ ∀ j ∉ s, ∀ x ∈ Δ j, g x = x := by
  classical
  induction s using Finset.induction_on with
  | empty => exact ⟨1, one_mem G, by simp, fun _ _ _ _ => rfl⟩
  | insert i s hi ih =>
    obtain ⟨g, hg, hgs, hgo⟩ := ih fun k hk => h k (Finset.mem_insert_of_mem hk)
    obtain ⟨g', hg', hg'i, hg'o⟩ := h i (Finset.mem_insert_self i s)
    refine ⟨g * g', mul_mem hg hg', fun k hk x => ?_, fun j hj x hx => ?_⟩
    · rcases Finset.mem_insert.1 hk with rfl | hk
      · rw [Perm.mul_apply, hg'i x, hgo k hi _ (π k x).2]
      · have hki : k ≠ i := fun e => hi (e ▸ hk)
        rw [Perm.mul_apply, hg'o k hki x x.2, hgs k hk x]
    · rw [Finset.mem_insert, not_or] at hj
      rw [Perm.mul_apply, hg'o j hj.1 x hx, hgo j hj.2 x hx]

end Restrict

/-! ### R22B-19 -/

section ProductAlt

variable {Ω : Type*} {G : Subgroup (Perm Ω)} {ι : Type*} {Δ : ι → Finset Ω}

/-- The elements of `G` that act trivially on `Δ j` for every `j ∈ J`. -/
def trivialOn (hΔ : ∀ i, IsStable G (Δ i)) (J : Finset ι) : Subgroup G where
  carrier := {g | ∀ j ∈ J, restrict (hΔ j) g = 1}
  mul_mem' {a b} ha hb j hj := by
    simp only [Set.mem_ofPred_eq] at ha hb
    rw [map_mul, ha j hj, hb j hj, one_mul]
  one_mem' j _ := map_one _
  inv_mem' {a} ha j hj := by
    simp only [Set.mem_ofPred_eq] at ha
    rw [map_inv, ha j hj, inv_one]

/-- Membership in `trivialOn`. -/
theorem mem_trivialOn {hΔ : ∀ i, IsStable G (Δ i)} {J : Finset ι} {g : G} :
    g ∈ trivialOn hΔ J ↔ ∀ j ∈ J, restrict (hΔ j) g = 1 :=
  Iff.rfl

/-- The elements that act trivially on given stable sets form a normal subgroup. -/
theorem trivialOn_normal (hΔ : ∀ i, IsStable G (Δ i)) (J : Finset ι) :
    (trivialOn hΔ J).Normal :=
  ⟨fun n hn g j hj => by
    rw [map_mul, map_mul, map_inv, mem_trivialOn.1 hn j hj, mul_one, mul_inv_cancel]⟩

/-- No condition: all of `G`. -/
theorem trivialOn_empty (hΔ : ∀ i, IsStable G (Δ i)) : trivialOn hΔ ∅ = ⊤ := by
  ext g
  simp [mem_trivialOn]

/-- One more index adds one kernel. -/
theorem trivialOn_insert [DecidableEq ι] (hΔ : ∀ i, IsStable G (Δ i)) (j : ι) (J : Finset ι) :
    trivialOn hΔ (insert j J) = trivialOn hΔ J ⊓ (restrict (hΔ j)).ker := by
  ext g
  simp only [mem_trivialOn, Finset.mem_insert, Subgroup.mem_inf, MonoidHom.mem_ker]
  constructor
  · intro h
    exact ⟨fun k hk => h k (Or.inr hk), h j (Or.inl rfl)⟩
  · rintro ⟨h, hj⟩ k (rfl | hk)
    · exact hj
    · exact h k hk

/-- R22B-19, the induction of the proof: for every finite set `J` of indices and every `i ∉ J`, the
elements of `G` that act trivially on the `Δ j`, `j ∈ J`, still induce a group containing
`Alt(Δ i)` on `Δ i`. -/
theorem alt_le_map_trivialOn [DecidableEq Ω] (hΔ : ∀ i, IsStable G (Δ i)) (hd : ∀ i, 7 ≤ (Δ i).card)
    (H1 : ∀ i, alternatingGroup (Δ i) ≤ (restrict (hΔ i)).range)
    (H2 : ∀ i j, i ≠ j → (Δ i).card = (Δ j).card →
      ∃ g : G, (restrict (hΔ i) g).cycleType ≠ (restrict (hΔ j) g).cycleType)
    (J : Finset ι) : ∀ i ∉ J, alternatingGroup (Δ i) ≤ (trivialOn hΔ J).map (restrict (hΔ i)) := by
  classical
  have hcard : ∀ k, 7 ≤ Fintype.card (Δ k) := fun k => by
    rw [Fintype.card_coe]
    exact hd k
  induction J using Finset.induction_on with
  | empty =>
    intro i _
    rw [trivialOn_empty, ← MonoidHom.range_eq_map]
    exact H1 i
  | insert j J hj ih =>
    intro i hi
    rw [Finset.mem_insert, not_or] at hi
    have := trivialOn_normal hΔ J
    rcases alt_le_or_coupled (hcard i) (by have := hcard j; omega) (restrict (hΔ i))
      (restrict (hΔ j)) (trivialOn hΔ J) (ih i hi.2) (ih j hj) with h | ⟨τ, hτ⟩
    · rwa [trivialOn_insert]
    · exfalso
      have hc : (Δ i).card = (Δ j).card := by
        rw [← Fintype.card_coe, ← Fintype.card_coe (Δ j)]
        exact (Fintype.card_congr τ).symm
      obtain ⟨g, hg⟩ := H2 i j hi.1 hc
      exact hg (by rw [hτ g, AlternatingAut.cycleType_permCongr])

/-- **R22B-19.** Let `Δ i` be stable sets of the permutation group `G` with `|Δ i| ≥ 7` (the
source asks `d_i ≥ 8`). H1: each `G|Δ i` contains `Alt(Δ i)`. H2: for `i ≠ j` with
`|Δ i| = |Δ j|`, some element of `G` has different cycle types on `Δ i` and `Δ j`. Then every even
permutation of `Δ i`, extended by the identity on the other `Δ j`, is induced on their union by an
element of `G`. The element need not fix chambers outside the union. -/
theorem productAlt [DecidableEq Ω] [Finite ι] (hΔ : ∀ i, IsStable G (Δ i))
    (hd : ∀ i, 7 ≤ (Δ i).card)
    (H1 : ∀ i, alternatingGroup (Δ i) ≤ (restrict (hΔ i)).range)
    (H2 : ∀ i j, i ≠ j → (Δ i).card = (Δ j).card →
      ∃ g : G, (restrict (hΔ i) g).cycleType ≠ (restrict (hΔ j) g).cycleType)
    (i : ι) {σ : Perm (Δ i)} (hσ : σ ∈ alternatingGroup (Δ i)) :
    ∃ g ∈ G, (∀ x : Δ i, g x = σ x) ∧ ∀ j ≠ i, ∀ x ∈ Δ j, g x = x := by
  classical
  have := Fintype.ofFinite ι
  have := alt_le_map_trivialOn hΔ hd H1 H2 (Finset.univ.erase i) i (by simp) hσ
  obtain ⟨g, hg, hgσ⟩ := Subgroup.mem_map.1 this
  refine ⟨g, g.2, fun x => by rw [← hgσ]; rfl, fun j hj x hx => ?_⟩
  exact (restrict_eq_one_iff (hΔ j) g).1 (mem_trivialOn.1 hg j (by simp [hj])) x hx

/-- **R22B-19**, product form: `H ⊇ ∏ Alt(Δ i)`. Under the hypotheses of `productAlt`, for every
family of even permutations `σ i` of the `Δ i` one element of `G` acts as `σ i` on every `Δ i`. -/
theorem productAlt_pi [DecidableEq Ω] [Finite ι] (hΔ : ∀ i, IsStable G (Δ i))
    (hd : ∀ i, 7 ≤ (Δ i).card)
    (H1 : ∀ i, alternatingGroup (Δ i) ≤ (restrict (hΔ i)).range)
    (H2 : ∀ i j, i ≠ j → (Δ i).card = (Δ j).card →
      ∃ g : G, (restrict (hΔ i) g).cycleType ≠ (restrict (hΔ j) g).cycleType)
    (σ : ∀ i, Perm (Δ i)) (hσ : ∀ i, σ i ∈ alternatingGroup (Δ i)) :
    ∃ g ∈ G, ∀ i, ∀ x : Δ i, g x = σ i x := by
  have := Fintype.ofFinite ι
  obtain ⟨g, hg, h, -⟩ := exists_forall_of_factors σ Finset.univ
    fun i _ => productAlt hΔ hd H1 H2 i (hσ i)
  exact ⟨g, hg, fun i => h i (Finset.mem_univ i)⟩

end ProductAlt

/-! ### Even permutations that move a subset onto another -/

open scoped Pointwise in
/-- R22B-19 and R22B-25: for `3 ≤ |B|`, `S, T ⊆ B` and `|T| ≤ |S|`, some even permutation of `B`
maps a subset of `S` onto `T` (the parity is corrected inside the chosen subset or its complement;
Mathlib's `Set.powersetCard.isPretransitive_alternatingGroup`). -/
theorem exists_alt_cover {Ω : Type*} [DecidableEq Ω] {B : Finset Ω} (hB : 3 ≤ B.card)
    {S T : Finset Ω} (hS : S ⊆ B) (hT : T ⊆ B) (hc : T.card ≤ S.card) :
    ∃ σ ∈ alternatingGroup B, ∀ t ∈ T, ∃ s : B, (s : Ω) ∈ S ∧ (σ s : Ω) = t := by
  obtain ⟨S', hS'S, hS'c⟩ := Finset.exists_subset_card_eq hc
  have hS'B : S' ⊆ B := hS'S.trans hS
  let S'' : Finset B := S'.subtype (· ∈ B)
  let T'' : Finset B := T.subtype (· ∈ B)
  have hS'' : S''.card = T.card := by
    rw [Finset.card_subtype, Finset.filter_true_of_mem fun x hx => hS'B hx, hS'c]
  have hT'' : T''.card = T.card := by
    rw [Finset.card_subtype, Finset.filter_true_of_mem fun x hx => hT hx]
  have hpre := Set.powersetCard.isPretransitive_alternatingGroup (α := B) (n := T.card)
    (by rw [Nat.card_eq_fintype_card, Fintype.card_coe]; exact hB)
  obtain ⟨σ, hσ⟩ := hpre.exists_smul_eq ⟨S'', hS''⟩ ⟨T'', hT''⟩
  refine ⟨σ, σ.2, fun t ht => ?_⟩
  have h2 : σ • S'' = T'' := congrArg Subtype.val hσ
  have hmem : (⟨t, hT ht⟩ : B) ∈ σ • S'' := by
    rw [h2]
    simp [T'', ht]
  rw [Finset.mem_smul_finset] at hmem
  obtain ⟨s, hs, hst⟩ := hmem
  refine ⟨s, hS'S (by simpa [S''] using hs), ?_⟩
  have := congrArg Subtype.val hst
  simpa [Subgroup.smul_def, Perm.smul_def] using this

open scoped Pointwise in
/-- R22B-25: for `S, T ⊆ B` and `|T| ≤ |S|`, some permutation of `B` maps a subset of `S` onto
`T` (Mathlib's `Set.powersetCard.isPretransitive`). -/
theorem exists_perm_cover {Ω : Type*} {B : Finset Ω} {S T : Finset Ω}
    (hS : S ⊆ B) (hT : T ⊆ B) (hc : T.card ≤ S.card) :
    ∃ σ : Perm B, ∀ t ∈ T, ∃ s : B, (s : Ω) ∈ S ∧ (σ s : Ω) = t := by
  classical
  obtain ⟨S', hS'S, hS'c⟩ := Finset.exists_subset_card_eq hc
  have hS'B : S' ⊆ B := hS'S.trans hS
  let S'' : Finset B := S'.subtype (· ∈ B)
  let T'' : Finset B := T.subtype (· ∈ B)
  have hS'' : S''.card = T.card := by
    rw [Finset.card_subtype, Finset.filter_true_of_mem fun x hx => hS'B hx, hS'c]
  have hT'' : T''.card = T.card := by
    rw [Finset.card_subtype, Finset.filter_true_of_mem fun x hx => hT hx]
  have hpre := Set.powersetCard.isPretransitive (α := B) (n := T.card)
  obtain ⟨σ, hσ⟩ := hpre.exists_smul_eq ⟨S'', hS''⟩ ⟨T'', hT''⟩
  refine ⟨σ, fun t ht => ?_⟩
  have h2 : σ • S'' = T'' := congrArg Subtype.val hσ
  have hmem : (⟨t, hT ht⟩ : B) ∈ σ • S'' := by
    rw [h2]
    simp [T'', ht]
  rw [Finset.mem_smul_finset] at hmem
  obtain ⟨s, hs, hst⟩ := hmem
  refine ⟨s, hS'S (by simpa [S''] using hs), ?_⟩
  have := congrArg Subtype.val hst
  simpa [Perm.smul_def] using this

end ProductAlt

/-! ### Consequence for vacancy (in part one's setting) -/

namespace Alphabet

open ProductAlt

variable {L Ω : Type*} [DecidableEq Ω] (A : Alphabet L Ω)

omit [DecidableEq Ω] in
/-- A set that every letter maps into itself is stable under the generated group. -/
theorem isStable_group {Δ : Finset Ω} (h : A.IsInvariant Δ) : IsStable A.group Δ := by
  intro g hg x hx
  obtain ⟨w, rfl⟩ := A.mem_group_iff.1 hg
  exact h.wordPerm_mem w hx

/-- **R22B-19, consequence for vacancy.** If every orbit of at least two chambers that meets the
target `T` is one of the orbits `Δ i` of at least 7 chambers (the source: 8), and H1 and H2 of
R22B-19 hold on them, then N2 (which implies N1) gives a vacating word: a word `V` with
`T ⊆ holes(Z·V)`. The word may move other orbits. -/
theorem vacancy_of_productAlt [Finite Ω] {ι : Type*} [Finite ι]
    {Δ : ι → Finset Ω} (horbit : ∀ i, A.IsOrbit (Δ i)) (hd : ∀ i, 7 ≤ (Δ i).card)
    (H1 : ∀ i, alternatingGroup (Δ i) ≤
      (restrict (A.isStable_group (horbit i).invariant)).range)
    (H2 : ∀ i j, i ≠ j → (Δ i).card = (Δ j).card → ∃ w : List L,
      (restrict (A.isStable_group (horbit i).invariant)
        ⟨A.wordPerm w, A.wordPerm_mem_group w⟩).cycleType ≠
      (restrict (A.isStable_group (horbit j).invariant)
        ⟨A.wordPerm w, A.wordPerm_mem_group w⟩).cycleType)
    {H T : Finset Ω} (hN2 : A.N2 H T)
    (hcover : ∀ Δ', A.IsOrbit Δ' → 2 ≤ Δ'.card → (T ∩ Δ').Nonempty → ∃ i, Δ' = Δ i) :
    ∃ V : List L, T ⊆ H.image (A.wordPerm V) := by
  have := Fintype.ofFinite Ω
  have hΔ : ∀ i, IsStable A.group (Δ i) := fun i => A.isStable_group (horbit i).invariant
  have H2' : ∀ i j, i ≠ j → (Δ i).card = (Δ j).card →
      ∃ g : A.group, (restrict (hΔ i) g).cycleType ≠ (restrict (hΔ j) g).cycleType := by
    intro i j hij hc
    obtain ⟨w, hw⟩ := H2 i j hij hc
    exact ⟨⟨A.wordPerm w, A.wordPerm_mem_group w⟩, hw⟩
  have hσ : ∀ i, ∃ σ ∈ alternatingGroup (Δ i), ∀ t ∈ T ∩ Δ i, ∃ s : Δ i, (s : Ω) ∈ H ∧
      (σ s : Ω) = t := by
    intro i
    obtain ⟨σ, hσ, h⟩ := exists_alt_cover (B := Δ i) (by have := hd i; omega)
      Finset.inter_subset_right Finset.inter_subset_right (hN2 (Δ i) (horbit i))
    exact ⟨σ, hσ, fun t ht => by
      obtain ⟨s, hs, hst⟩ := h t ht
      exact ⟨s, (Finset.mem_inter.1 hs).1, hst⟩⟩
  choose σ hσalt hσT using hσ
  obtain ⟨g, hg, hgσ⟩ := productAlt_pi hΔ hd H1 H2' σ hσalt
  obtain ⟨V, rfl⟩ := A.mem_group_iff.1 hg
  refine ⟨V, fun t ht => ?_⟩
  by_cases hfix : A.IsFixed t
  · exact A.mem_image_of_fixed (hfix.wordPerm_apply V) (hN2.n1 t ht hfix)
  · obtain ⟨i, hi⟩ := hcover (A.orbitOf t) (A.isOrbit_orbitOf t) (A.two_le_card_orbitOf hfix)
      ⟨t, Finset.mem_inter.2 ⟨ht, A.mem_orbitOf_self t⟩⟩
    have htΔ : t ∈ Δ i := hi ▸ A.mem_orbitOf_self t
    obtain ⟨s, hsH, hst⟩ := hσT i t (Finset.mem_inter.2 ⟨ht, htΔ⟩)
    exact Finset.mem_image.2 ⟨s, hsH, by rw [hgσ i s, hst]⟩

end Alphabet

/-! ### The size condition cannot be dropped -/

namespace ProductAlt.CyclicFour

/-- The four-cycle `(0 1 2 3)`. -/
def c : Perm (Fin 4) :=
  ⟨![1, 2, 3, 0], ![3, 0, 1, 2], by decide, by decide⟩

/-- The alphabet of the four-cycle and its inverse. -/
def alphabet : Jumbling.Alphabet Bool (Fin 4) where
  act b := if b then c else c⁻¹
  inv b := !b
  act_inv b := by cases b <;> simp

/-- The holes `{0, 2}`. -/
def holes : Finset (Fin 4) := {0, 2}

/-- The target `{0, 1}`. -/
def targets : Finset (Fin 4) := {0, 1}

/-- The group of the alphabet is `{1, c, c², c³}`. -/
theorem wordPerm_mem (w : List Bool) :
    alphabet.wordPerm w ∈ ({1, c, c * c, c * c * c} : Finset (Perm (Fin 4))) := by
  have key : ∀ g ∈ ({1, c, c * c, c * c * c} : Finset (Perm (Fin 4))), ∀ b : Bool,
      g * alphabet.act b ∈ ({1, c, c * c, c * c * c} : Finset (Perm (Fin 4))) := by
    simp only [alphabet]
    decide
  induction w with
  | nil => simp
  | cons b w ih =>
    rw [Alphabet.wordPerm_cons]
    exact key _ ih b

/-- **R22B-19**, counterexample to dropping the size condition (abstract, not a state of the
puzzle): the cyclic group of `(0 1 2 3)` with holes `{0, 2}` and target `{0, 1}` satisfies N1 and
N2, and no element maps a set of holes onto the target. -/
theorem counterexample :
    alphabet.N1 holes targets ∧ alphabet.N2 holes targets ∧
      ¬∃ V : List Bool, targets ⊆ holes.image (alphabet.wordPerm V) := by
  have hc : ∀ t : Fin 4, c t ≠ t := by decide
  have hcyc : ∀ x y : Fin 4, y = x ∨ y = c x ∨ y = c (c x) ∨ y = c (c (c x)) := by decide
  refine ⟨fun t _ hfix => absurd (hfix true) (hc t), fun Δ hΔ => ?_, ?_⟩
  · obtain ⟨x, hx⟩ := hΔ.nonempty
    have hinv : ∀ y ∈ Δ, c y ∈ Δ := fun y hy => hΔ.invariant true y hy
    have huniv : Δ = Finset.univ := by
      ext y
      simp only [Finset.mem_univ, iff_true]
      rcases hcyc x y with rfl | rfl | rfl | rfl
      · exact hx
      · exact hinv _ hx
      · exact hinv _ (hinv _ hx)
      · exact hinv _ (hinv _ (hinv _ hx))
    subst huniv
    decide
  · rintro ⟨V, hV⟩
    have hm := wordPerm_mem V
    generalize alphabet.wordPerm V = g at hV hm
    revert g
    decide

end ProductAlt.CyclicFour

end Jumbling
