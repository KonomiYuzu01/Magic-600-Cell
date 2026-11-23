import Jumbling.Group.Words
import Mathlib.Data.Nat.Choose.Bounds
import Mathlib.GroupTheory.GroupAction.MultipleTransitivity

set_option linter.style.header false in
/-!
# R22B-01 and R22B-02: certified three-cycles give a polynomial bound

`PROOFS-B.md` section 3, R22B-01 and R22B-02 (layer "group" of `ARCHON-HANDOFF.md`).

Setting: an alphabet `A` (the SAFE-ALL letters of O5) on the chambers `Ω`, holes `H`, a target `T`,
and the orbits `Δ 0, …, Δ (r - 1)` of size at least 2 that meet `T`, in a fixed order (`hcover`
says that every such orbit is one of them). The union of the orbits before `Δ i` is
`(Finset.range i).biUnion Δ`.

* `IsSeed` is H2 of R22B-01 and `IsTables` is H3 and H4.
* `vacate_of_three_cycles`: the hole-moving procedure (steps 3 and 4) for any supply of three-cycle
  words that fix the earlier orbits, with a per-orbit cost `K i`.
* `certifiedThreeCycles` (R22B-01, statement): a word `V` with `T ⊆ holes(Z·V)` and
  `|V| ≤ 2·Σ_i k_i·(s_i + 2e_i + 2f_i)`; `certifiedThreeCycles_uniform`: `|V| ≤ 10·|T|·L` and
  `|T| ≤ n(Z)`.
* `tables_of_connected`, `certifiedThreeCycles_shortLetters` (tables from short letters):
  `e ≤ d - 2`, `f ≤ d - 3` and `|V| ≤ 2·Σ_i k_i·(s_i + 4d_i - 10)`.
* `certifiedThreeCycles_alt` (without tables): from Theorem A's conclusion, taken as the hypothesis
  `hAlt`, `|V| ≤ Σ_i k_i·[s_i + 2(d_i(d_i - 1)(d_i - 2) - 1)]`.
* `diagonalCounterexample`: the abstract diagonal `A₅` counterexample.
* `vacateShortest` and `vacateShortest_certified` (R22B-02).

Theorem A and Proposition N are not proved here; where a statement needs Theorem A's conclusion it
is an explicit hypothesis.
-/

namespace Jumbling

namespace Alphabet

open Equiv Finset

variable {L Ω : Type*} [DecidableEq Ω] (A : Alphabet L Ω)

/-- H2 of R22B-01: a seed for the orbit `Δ` relative to the union `E` of the earlier orbits, a word
`C` that acts on `Δ` as the three-cycle `(a b c)` of distinct anchors `a, b, c ∈ Δ` and fixes `E`
pointwise. -/
structure IsSeed (Δ E : Finset Ω) (a b c : Ω) (C : List L) : Prop where
  ha : a ∈ Δ
  hb : b ∈ Δ
  hc : c ∈ Δ
  hab : a ≠ b
  hac : a ≠ c
  hbc : b ≠ c
  acts : ∀ x ∈ Δ, A.wordPerm C x = cycle3 a b c x
  fixes : ∀ x ∈ E, A.wordPerm C x = x

/-- H3 and H4 of R22B-01: the first table (for `x ∈ Δ \ {a}`, a word `P x` of at most `e` letters
that fixes `a` and sends `x` to `b`) and the second table (for `z ∈ Δ \ {a, b}`, a word `Q z` of at
most `f` letters that fixes `a` and `b` and sends `c` to `z`). -/
structure IsTables (Δ : Finset Ω) (a b c : Ω) (P Q : Ω → List L) (e f : ℕ) : Prop where
  first : ∀ x ∈ Δ, x ≠ a → A.wordPerm (P x) a = a ∧ A.wordPerm (P x) x = b ∧ (P x).length ≤ e
  second : ∀ z ∈ Δ, z ≠ a → z ≠ b →
    A.wordPerm (Q z) a = a ∧ A.wordPerm (Q z) b = b ∧ A.wordPerm (Q z) c = z ∧ (Q z).length ≤ f

/-! ### Steps 1 and 2: every three-cycle of `Δ` -/

/-- R22B-01, step 1: for distinct `x, y ∈ Δ \ {a}`, with `z = π_{P_x}(y)` and `U = P_x·Q_z⁻¹`, the
word `U·C·U⁻¹` induces the three-cycle `(a x y)` on `Δ`, fixes the earlier orbits pointwise and has
at most `s + 2e + 2f` letters. -/
theorem exists_cycle_through_anchor {Δ E : Finset Ω} (hΔ : A.IsInvariant Δ) (hE : A.IsInvariant E)
    {a b c : Ω} {C : List L} (hC : A.IsSeed Δ E a b c C) {P Q : Ω → List L} {e f : ℕ}
    (hT : A.IsTables Δ a b c P Q e f) {x y : Ω} (hx : x ∈ Δ) (hy : y ∈ Δ) (hxa : x ≠ a)
    (hya : y ≠ a) (hxy : x ≠ y) :
    ∃ W : List L, (∀ p ∈ Δ, A.wordPerm W p = cycle3 a x y p) ∧ (∀ p ∈ E, A.wordPerm W p = p) ∧
      W.length ≤ C.length + 2 * e + 2 * f := by
  obtain ⟨hPa, hPx, hPlen⟩ := hT.first x hx hxa
  set z := A.wordPerm (P x) y with hz_def
  have hzΔ : z ∈ Δ := hΔ.wordPerm_mem _ hy
  have hza : z ≠ a := fun h => hya ((A.wordPerm (P x)).injective (h.trans hPa.symm))
  have hzb : z ≠ b := fun h => hxy ((A.wordPerm (P x)).injective (hPx.trans h.symm))
  obtain ⟨hQa, hQb, hQc, hQlen⟩ := hT.second z hzΔ hza hzb
  set U := P x ++ A.invWord (Q z) with hU_def
  have hUa : A.wordPerm U a = a := by
    rw [hU_def, wordPerm_append_apply, hPa]
    exact A.wordPerm_invWord_apply_of_apply hQa
  have hUx : A.wordPerm U x = b := by
    rw [hU_def, wordPerm_append_apply, hPx]
    exact A.wordPerm_invWord_apply_of_apply hQb
  have hUy : A.wordPerm U y = c := by
    rw [hU_def, wordPerm_append_apply, ← hz_def]
    exact A.wordPerm_invWord_apply_of_apply hQc
  refine ⟨U ++ C ++ A.invWord U, ?_, ?_, ?_⟩
  · intro p hp
    rw [wordPerm_append_apply, wordPerm_append_apply, hC.acts _ (hΔ.wordPerm_mem U hp)]
    by_cases hpa : p = a
    · subst hpa
      rw [hUa, cycle3_apply_left hC.hab hC.hac, cycle3_apply_left (Ne.symm hxa) (Ne.symm hya)]
      exact A.wordPerm_invWord_apply_of_apply hUx
    by_cases hpx : p = x
    · subst hpx
      rw [hUx, cycle3_apply_mid hC.hac hC.hbc, cycle3_apply_mid (Ne.symm hya) hxy]
      exact A.wordPerm_invWord_apply_of_apply hUy
    by_cases hpy : p = y
    · subst hpy
      rw [hUy, cycle3_apply_right, cycle3_apply_right]
      exact A.wordPerm_invWord_apply_of_apply hUa
    have hU := (A.wordPerm U).injective
    rw [cycle3_apply_of_ne (fun h => hpa (hU (h.trans hUa.symm)))
      (fun h => hpx (hU (h.trans hUx.symm))) (fun h => hpy (hU (h.trans hUy.symm))),
      wordPerm_invWord_apply_self, cycle3_apply_of_ne hpa hpx hpy]
  · intro p hp
    rw [wordPerm_append_apply, wordPerm_append_apply, hC.fixes _ (hE.wordPerm_mem U hp),
      wordPerm_invWord_apply_self]
  · simp only [List.length_append, length_invWord, hU_def]
    omega

/-- R22B-01, step 2: every three-cycle `(x y z)` of `Δ` is induced by a word that fixes the earlier
orbits pointwise and has at most `2(s + 2e + 2f)` letters; `(a x y)` followed by `(a z x)` is
`(x y z)`. -/
theorem exists_three_cycle_word {Δ E : Finset Ω} (hΔ : A.IsInvariant Δ) (hE : A.IsInvariant E)
    {a b c : Ω} {C : List L} (hC : A.IsSeed Δ E a b c C) {P Q : Ω → List L} {e f : ℕ}
    (hT : A.IsTables Δ a b c P Q e f) {x y z : Ω} (hx : x ∈ Δ) (hy : y ∈ Δ) (hz : z ∈ Δ)
    (hxy : x ≠ y) (hxz : x ≠ z) (hyz : y ≠ z) :
    ∃ W : List L, (∀ p ∈ Δ, A.wordPerm W p = cycle3 x y z p) ∧ (∀ p ∈ E, A.wordPerm W p = p) ∧
      W.length ≤ 2 * (C.length + 2 * e + 2 * f) := by
  have hle : C.length + 2 * e + 2 * f ≤ 2 * (C.length + 2 * e + 2 * f) := by omega
  by_cases hxa : x = a
  · subst hxa
    obtain ⟨W, h1, h2, h3⟩ := A.exists_cycle_through_anchor hΔ hE hC hT hy hz (Ne.symm hxy)
      (Ne.symm hxz) hyz
    exact ⟨W, h1, h2, h3.trans hle⟩
  by_cases hya : y = a
  · subst hya
    obtain ⟨W, h1, h2, h3⟩ := A.exists_cycle_through_anchor hΔ hE hC hT hz hx (Ne.symm hyz)
      hxa (Ne.symm hxz)
    refine ⟨W, fun p hp => ?_, h2, h3.trans hle⟩
    rw [h1 p hp, cycle3_rotate hxy hxz hyz]
  by_cases hza : z = a
  · subst hza
    obtain ⟨W, h1, h2, h3⟩ := A.exists_cycle_through_anchor hΔ hE hC hT hx hy hxa hya hxy
    refine ⟨W, fun p hp => ?_, h2, h3.trans hle⟩
    rw [h1 p hp, cycle3_rotate hxy hxz hyz, cycle3_rotate hyz (Ne.symm hxy) (Ne.symm hxz)]
  obtain ⟨W₁, h11, h12, h13⟩ := A.exists_cycle_through_anchor hΔ hE hC hT hx hy hxa hya hxy
  obtain ⟨W₂, h21, h22, h23⟩ := A.exists_cycle_through_anchor hΔ hE hC hT hz hx hza hxa
    (Ne.symm hxz)
  refine ⟨W₁ ++ W₂, fun p hp => ?_, fun p hp => ?_, ?_⟩
  · rw [wordPerm_append_apply, h21 _ (hΔ.wordPerm_mem W₁ hp), h11 p hp, ← Perm.mul_apply,
      cycle3_mul_cycle3 (Ne.symm hxa) (Ne.symm hya) (Ne.symm hza) hxy hxz hyz]
  · rw [wordPerm_append_apply, h12 p hp, h22 p hp]
  · rw [List.length_append]
    omega

/-! ### Steps 3 and 4: moving the holes -/

/-- If a word acts on `Δ` as `σ`, a chamber of `Δ` is a hole after the word exactly when its
preimage under `σ` was a hole. -/
theorem mem_image_iff_of_acts {Δ : Finset Ω} (hΔ : A.IsInvariant Δ) {W : List L} {σ : Perm Ω}
    (hW : ∀ p ∈ Δ, A.wordPerm W p = σ p) (H : Finset Ω) {p : Ω} (hp : p ∈ Δ) :
    p ∈ H.image (A.wordPerm W) ↔ σ⁻¹ p ∈ H := by
  rw [mem_image_perm]
  have hq : (A.wordPerm W)⁻¹ p ∈ Δ := hΔ.inv_mem W hp
  have : σ ((A.wordPerm W)⁻¹ p) = p := by rw [← hW _ hq, perm_apply_inv_self]
  rw [(Perm.inv_eq_iff_eq).2 this.symm]

/-- R22B-01, step 3: the three-cycle `(z r t)` when `r` is a hole and `(z t r)` otherwise turns the
target chamber `t` into a hole and keeps the status of every other target chamber. -/
theorem hole_move {H T : Finset Ω} {z t r : Ω} (hzH : z ∈ H) (hzT : z ∉ T) (htH : t ∉ H)
    (hrz : r ≠ z) (hrt : r ≠ t) {σ : Perm Ω}
    (hσ : (r ∈ H ∧ σ = cycle3 z r t) ∨ (r ∉ H ∧ σ = cycle3 z t r)) {p : Ω} (hp : p ∈ T) :
    σ⁻¹ p ∈ H ↔ p ∈ H ∨ p = t := by
  have hzt : z ≠ t := fun h => htH (h ▸ hzH)
  have hpz : p ≠ z := fun h => hzT (h ▸ hp)
  rcases hσ with ⟨hrH, rfl⟩ | ⟨hrH, rfl⟩
  · rw [cycle3_inv (Ne.symm hrz) hzt hrt]
    by_cases hpt : p = t
    · subst hpt
      rw [cycle3_apply_mid (Ne.symm hrz) (Ne.symm hrt)]
      exact ⟨fun _ => Or.inr rfl, fun _ => hrH⟩
    by_cases hpr : p = r
    · subst hpr
      rw [cycle3_apply_right]
      exact ⟨fun _ => Or.inl hrH, fun _ => hzH⟩
    rw [cycle3_apply_of_ne hpz hpt hpr]
    exact ⟨Or.inl, fun h => h.resolve_right hpt⟩
  · rw [cycle3_inv hzt (Ne.symm hrz) (Ne.symm hrt)]
    by_cases hpt : p = t
    · subst hpt
      rw [cycle3_apply_right]
      exact ⟨fun _ => Or.inr rfl, fun _ => hzH⟩
    by_cases hpr : p = r
    · subst hpr
      rw [cycle3_apply_mid hzt hrt]
      exact ⟨fun h => absurd h htH, fun h => absurd (h.resolve_right hpt) hrH⟩
    rw [cycle3_apply_of_ne hpz hpr hpt]
    exact ⟨Or.inl, fun h => h.resolve_right hpt⟩

/-- R22B-01, step 3 for one orbit: if every three-cycle of `Δ` is induced by a word of at most `K`
letters that fixes `E` pointwise and N2 holds on `Δ`, a word of at most `|(T ∩ Δ) \ H|·K` letters
fixes `E` pointwise and leaves every target chamber of `Δ` a hole. -/
theorem clear_orbit {Δ E T : Finset Ω} (hΔ : A.IsOrbit Δ) (hd : 3 ≤ Δ.card) {K : ℕ}
    (hK : ∀ x ∈ Δ, ∀ y ∈ Δ, ∀ z ∈ Δ, x ≠ y → x ≠ z → y ≠ z → ∃ W : List L,
      (∀ p ∈ Δ, A.wordPerm W p = cycle3 x y z p) ∧ (∀ p ∈ E, A.wordPerm W p = p) ∧
        W.length ≤ K)
    (H : Finset Ω) (hN2 : (T ∩ Δ).card ≤ (H ∩ Δ).card) :
    ∃ W : List L, (∀ p ∈ E, A.wordPerm W p = p) ∧ T ∩ Δ ⊆ H.image (A.wordPerm W) ∧
      W.length ≤ ((T ∩ Δ) \ H).card * K := by
  suffices key : ∀ m, ∀ H : Finset Ω, ((T ∩ Δ) \ H).card = m → (T ∩ Δ).card ≤ (H ∩ Δ).card →
      ∃ W : List L, (∀ p ∈ E, A.wordPerm W p = p) ∧ T ∩ Δ ⊆ H.image (A.wordPerm W) ∧
        W.length ≤ m * K from key _ H rfl hN2
  intro m
  induction m with
  | zero =>
    intro H hm _
    refine ⟨[], fun p _ => rfl, ?_, by simp⟩
    rw [Finset.card_eq_zero, Finset.sdiff_eq_empty_iff_subset] at hm
    simpa using hm
  | succ m ih =>
    intro H hm hN2
    obtain ⟨t, ht⟩ : ((T ∩ Δ) \ H).Nonempty := Finset.card_pos.1 (by omega)
    obtain ⟨htTΔ, htH⟩ := Finset.mem_sdiff.1 ht
    obtain ⟨htT, htΔ⟩ := Finset.mem_inter.1 htTΔ
    -- A hole of `Δ` outside the target.
    obtain ⟨z, hz⟩ : ((H ∩ Δ) \ T).Nonempty := by
      by_contra hne
      rw [Finset.not_nonempty_iff_eq_empty, Finset.sdiff_eq_empty_iff_subset] at hne
      have hsub : H ∩ Δ ⊆ (T ∩ Δ).erase t := by
        intro y hy
        refine Finset.mem_erase.2 ⟨fun h => htH (h ▸ (Finset.mem_inter.1 hy).1), ?_⟩
        exact Finset.mem_inter.2 ⟨hne hy, (Finset.mem_inter.1 hy).2⟩
      have := Finset.card_le_card hsub
      rw [Finset.card_erase_of_mem htTΔ] at this
      have hpos : 0 < (T ∩ Δ).card := Finset.card_pos.2 ⟨t, htTΔ⟩
      omega
    obtain ⟨hzHΔ, hzT⟩ := Finset.mem_sdiff.1 hz
    obtain ⟨hzH, hzΔ⟩ := Finset.mem_inter.1 hzHΔ
    have hzt : z ≠ t := fun h => htH (h ▸ hzH)
    -- A third chamber of `Δ`.
    obtain ⟨r, hrΔ, hr⟩ := Finset.exists_mem_notMem_of_card_lt_card
      (s := {z, t}) (t := Δ) (by rw [Finset.card_pair hzt]; omega)
    simp only [Finset.mem_insert, Finset.mem_singleton, not_or] at hr
    obtain ⟨hrz, hrt⟩ := hr
    -- The three-cycle word.
    obtain ⟨σ, hσ, W₁, hW₁, hW₁E, hW₁len⟩ : ∃ σ : Perm Ω,
        ((r ∈ H ∧ σ = cycle3 z r t) ∨ (r ∉ H ∧ σ = cycle3 z t r)) ∧ ∃ W : List L,
          (∀ p ∈ Δ, A.wordPerm W p = σ p) ∧ (∀ p ∈ E, A.wordPerm W p = p) ∧ W.length ≤ K := by
      by_cases hrH : r ∈ H
      · exact ⟨_, Or.inl ⟨hrH, rfl⟩, hK z hzΔ r hrΔ t htΔ (Ne.symm hrz) hzt hrt⟩
      · exact ⟨_, Or.inr ⟨hrH, rfl⟩, hK z hzΔ t htΔ r hrΔ hzt (Ne.symm hrz) (Ne.symm hrt)⟩
    set H' := H.image (A.wordPerm W₁) with hH'
    have hstep : (T ∩ Δ) \ H' = ((T ∩ Δ) \ H).erase t := by
      ext p
      simp only [Finset.mem_sdiff, Finset.mem_erase, Finset.mem_inter]
      constructor
      · rintro ⟨⟨hpT, hpΔ⟩, hpH'⟩
        rw [hH', A.mem_image_iff_of_acts hΔ.invariant hW₁ H hpΔ,
          hole_move hzH hzT htH hrz hrt hσ hpT, not_or] at hpH'
        exact ⟨hpH'.2, ⟨hpT, hpΔ⟩, hpH'.1⟩
      · rintro ⟨hpt, ⟨hpT, hpΔ⟩, hpH⟩
        refine ⟨⟨hpT, hpΔ⟩, ?_⟩
        rw [hH', A.mem_image_iff_of_acts hΔ.invariant hW₁ H hpΔ,
          hole_move hzH hzT htH hrz hrt hσ hpT, not_or]
        exact ⟨hpH, hpt⟩
    have hm' : ((T ∩ Δ) \ H').card = m := by
      rw [hstep, Finset.card_erase_of_mem ht, hm, Nat.add_sub_cancel]
    have hN2' : (T ∩ Δ).card ≤ (H' ∩ Δ).card := by
      rw [hH', A.card_image_inter hΔ.invariant]
      exact hN2
    obtain ⟨W₂, hW₂E, hW₂T, hW₂len⟩ := ih H' hm' hN2'
    refine ⟨W₁ ++ W₂, fun p hp => ?_, ?_, ?_⟩
    · rw [wordPerm_append_apply, hW₁E p hp, hW₂E p hp]
    · rw [image_wordPerm_append]
      exact hW₂T
    · rw [List.length_append, Nat.succ_mul]
      omega

variable [Finite Ω]

/-- R22B-01, step 4: the induction over the orbits. If every three-cycle of `Δ i` is induced by a
word of at most `K i` letters that fixes the earlier orbits pointwise, then under N1 and N2 a word
`V` with at most `Σ_i k_i·K i` letters leaves every target chamber a hole. Targets in singleton
orbits are holes by N1 and never move. -/
theorem vacate_of_three_cycles {H T : Finset Ω} (hN1 : A.N1 H T) (hN2 : A.N2 H T) {r : ℕ}
    {Δ : ℕ → Finset Ω} (horbit : ∀ i < r, A.IsOrbit (Δ i)) (hd : ∀ i < r, 3 ≤ (Δ i).card)
    (hcover : ∀ Δ', A.IsOrbit Δ' → 2 ≤ Δ'.card → (T ∩ Δ').Nonempty → ∃ i < r, Δ' = Δ i)
    (K : ℕ → ℕ)
    (hK : ∀ i < r, ∀ x ∈ Δ i, ∀ y ∈ Δ i, ∀ z ∈ Δ i, x ≠ y → x ≠ z → y ≠ z → ∃ W : List L,
      (∀ p ∈ Δ i, A.wordPerm W p = cycle3 x y z p) ∧
        (∀ p ∈ (Finset.range i).biUnion Δ, A.wordPerm W p = p) ∧ W.length ≤ K i) :
    ∃ V : List L, T ⊆ H.image (A.wordPerm V) ∧
      V.length ≤ ∑ i ∈ Finset.range r, (T ∩ Δ i).card * K i := by
  let _ : Fintype Ω := Fintype.ofFinite Ω
  have key : ∀ n ≤ r, ∃ V : List L, (∀ i < n, T ∩ Δ i ⊆ H.image (A.wordPerm V)) ∧
      V.length ≤ ∑ i ∈ Finset.range n, (T ∩ Δ i).card * K i := by
    intro n
    induction n with
    | zero => exact fun _ => ⟨[], fun i hi => absurd hi (Nat.not_lt_zero i), by simp⟩
    | succ n ih =>
      intro hn
      obtain ⟨V, hV, hVlen⟩ := ih (Nat.le_of_succ_le hn)
      have hnr : n < r := hn
      set H' := H.image (A.wordPerm V)
      have hN2' : (T ∩ Δ n).card ≤ (H' ∩ Δ n).card := (hN2.image V) (Δ n) (horbit n hnr)
      obtain ⟨W, hWE, hWT, hWlen⟩ := A.clear_orbit (E := (Finset.range n).biUnion Δ)
        (horbit n hnr) (hd n hnr) (hK n hnr) H' hN2'
      refine ⟨V ++ W, fun i hi => ?_, ?_⟩
      · rw [image_wordPerm_append]
        rcases Nat.lt_succ_iff_lt_or_eq.1 hi with hi | rfl
        · intro p hp
          have hpE : p ∈ (Finset.range n).biUnion Δ :=
            Finset.mem_biUnion.2 ⟨i, Finset.mem_range.2 hi, (Finset.mem_inter.1 hp).2⟩
          exact A.mem_image_of_fixed (hWE p hpE) (hV i hi hp)
        · exact hWT
      · rw [Finset.sum_range_succ, List.length_append]
        have : ((T ∩ Δ n) \ H').card * K n ≤ (T ∩ Δ n).card * K n :=
          Nat.mul_le_mul_right _ (Finset.card_le_card Finset.sdiff_subset)
        omega
  obtain ⟨V, hV, hVlen⟩ := key r le_rfl
  refine ⟨V, fun t ht => ?_, hVlen⟩
  by_cases hfix : A.IsFixed t
  · exact A.mem_image_of_fixed (hfix.wordPerm_apply V) (hN1 t ht hfix)
  · obtain ⟨i, hi, hΔi⟩ := hcover (A.orbitOf t) (A.isOrbit_orbitOf t) (A.two_le_card_orbitOf hfix)
      ⟨t, Finset.mem_inter.2 ⟨ht, A.mem_orbitOf_self t⟩⟩
    exact hV i hi (Finset.mem_inter.2 ⟨ht, hΔi ▸ A.mem_orbitOf_self t⟩)

omit [Finite Ω] in
/-- The union of the orbits before `Δ i` is invariant. -/
theorem isInvariant_earlier {r : ℕ} {Δ : ℕ → Finset Ω} (horbit : ∀ i < r, A.IsOrbit (Δ i))
    {i : ℕ} (hi : i < r) : A.IsInvariant ((Finset.range i).biUnion Δ) :=
  IsInvariant.biUnion fun j hj => (horbit j (by simp at hj; omega)).invariant

omit [Finite Ω] in
/-- Under the seeds of R22B-01 the listed orbits are pairwise distinct, so pairwise disjoint: a
seed for `Δ j` acts on it as a three-cycle and fixes every earlier orbit pointwise. -/
theorem disjoint_of_seeds {r : ℕ} {Δ : ℕ → Finset Ω} (horbit : ∀ i < r, A.IsOrbit (Δ i))
    {a b c : ℕ → Ω} {C : ℕ → List L}
    (hseed : ∀ i < r, A.IsSeed (Δ i) ((Finset.range i).biUnion Δ) (a i) (b i) (c i) (C i))
    {i j : ℕ} (hi : i < r) (hj : j < r) (hij : i ≠ j) : Disjoint (Δ i) (Δ j) := by
  refine (horbit i hi).disjoint (horbit j hj) fun heq => ?_
  rcases lt_or_gt_of_ne hij with hlt | hlt
  · have hs := hseed j hj
    have ha : a j ∈ (Finset.range j).biUnion Δ :=
      Finset.mem_biUnion.2 ⟨i, Finset.mem_range.2 hlt, heq ▸ hs.ha⟩
    have h1 := hs.fixes _ ha
    rw [hs.acts _ hs.ha, cycle3_apply_left hs.hab hs.hac] at h1
    exact hs.hab h1.symm
  · have hs := hseed i hi
    have ha : a i ∈ (Finset.range i).biUnion Δ :=
      Finset.mem_biUnion.2 ⟨j, Finset.mem_range.2 hlt, heq.symm ▸ hs.ha⟩
    have h1 := hs.fixes _ ha
    rw [hs.acts _ hs.ha, cycle3_apply_left hs.hab hs.hac] at h1
    exact hs.hab h1.symm

omit [Finite Ω] in
/-- `Σ_i k_i ≤ |T|` for pairwise disjoint orbits. -/
theorem sum_card_inter_le {T : Finset Ω} {r : ℕ} {Δ : ℕ → Finset Ω}
    (hdisj : ∀ i < r, ∀ j < r, i ≠ j → Disjoint (Δ i) (Δ j)) :
    ∑ i ∈ Finset.range r, (T ∩ Δ i).card ≤ T.card := by
  rw [← Finset.card_biUnion]
  · exact Finset.card_le_card (Finset.biUnion_subset.2 fun i _ => Finset.inter_subset_left)
  · intro i hi j hj hij
    simp only [Finset.coe_range, Set.mem_Iio] at hi hj
    exact Finset.disjoint_of_subset_left Finset.inter_subset_right
      (Finset.disjoint_of_subset_right Finset.inter_subset_right (hdisj i hi j hj hij))

/-- R22B-01, certified three-cycles give a polynomial bound. Under N1 and N2 for `T`, with every
`d_i ≥ 5`, a seed `C_i` of at most `s_i` letters for each orbit (H2), and tables `P_x`, `Q_z` of at
most `e_i` and `f_i` letters (H3, H4), there is a SAFE-ALL word `V` with `T ⊆ holes(Z·V)` and
`|V| ≤ 2·Σ_i k_i·(s_i + 2e_i + 2f_i)`, where `k_i = |T ∩ Δ_i|`. -/
theorem certifiedThreeCycles {H T : Finset Ω} (hN1 : A.N1 H T) (hN2 : A.N2 H T) {r : ℕ}
    {Δ : ℕ → Finset Ω} (horbit : ∀ i < r, A.IsOrbit (Δ i)) (hd : ∀ i < r, 5 ≤ (Δ i).card)
    (hcover : ∀ Δ', A.IsOrbit Δ' → 2 ≤ Δ'.card → (T ∩ Δ').Nonempty → ∃ i < r, Δ' = Δ i)
    {a b c : ℕ → Ω} {C : ℕ → List L} {P Q : ℕ → Ω → List L} {s e f : ℕ → ℕ}
    (hseed : ∀ i < r, A.IsSeed (Δ i) ((Finset.range i).biUnion Δ) (a i) (b i) (c i) (C i))
    (hs : ∀ i < r, (C i).length ≤ s i)
    (htab : ∀ i < r, A.IsTables (Δ i) (a i) (b i) (c i) (P i) (Q i) (e i) (f i)) :
    ∃ V : List L, T ⊆ H.image (A.wordPerm V) ∧
      V.length ≤ 2 * ∑ i ∈ Finset.range r, (T ∩ Δ i).card * (s i + 2 * e i + 2 * f i) := by
  obtain ⟨V, hV, hVlen⟩ := A.vacate_of_three_cycles hN1 hN2 horbit (fun i hi => by
    have := hd i hi; omega) hcover (fun i => 2 * (s i + 2 * e i + 2 * f i)) (by
    intro i hi x hx y hy z hz hxy hxz hyz
    obtain ⟨W, h1, h2, h3⟩ := A.exists_three_cycle_word (horbit i hi).invariant
      (A.isInvariant_earlier horbit hi) (hseed i hi) (htab i hi) hx hy hz hxy hxz hyz
    exact ⟨W, h1, h2, h3.trans (by have := hs i hi; omega)⟩)
  refine ⟨V, hV, hVlen.trans (le_of_eq ?_)⟩
  rw [Finset.mul_sum]
  exact Finset.sum_congr rfl fun i _ => by ring

/-- R22B-01, the uniform bound: if every `s_i`, `e_i`, `f_i` is at most `L`, then
`|V| ≤ 10·|T|·L`, and `|T| ≤ n(Z)` by N1 and N2. -/
theorem certifiedThreeCycles_uniform {H T : Finset Ω} (hN1 : A.N1 H T) (hN2 : A.N2 H T) {r : ℕ}
    {Δ : ℕ → Finset Ω} (horbit : ∀ i < r, A.IsOrbit (Δ i)) (hd : ∀ i < r, 5 ≤ (Δ i).card)
    (hcover : ∀ Δ', A.IsOrbit Δ' → 2 ≤ Δ'.card → (T ∩ Δ').Nonempty → ∃ i < r, Δ' = Δ i)
    {a b c : ℕ → Ω} {C : ℕ → List L} {P Q : ℕ → Ω → List L} {s e f : ℕ → ℕ}
    (hseed : ∀ i < r, A.IsSeed (Δ i) ((Finset.range i).biUnion Δ) (a i) (b i) (c i) (C i))
    (hs : ∀ i < r, (C i).length ≤ s i)
    (htab : ∀ i < r, A.IsTables (Δ i) (a i) (b i) (c i) (P i) (Q i) (e i) (f i))
    {Lb : ℕ} (hsL : ∀ i < r, s i ≤ Lb) (heL : ∀ i < r, e i ≤ Lb) (hfL : ∀ i < r, f i ≤ Lb) :
    ∃ V : List L, T ⊆ H.image (A.wordPerm V) ∧ V.length ≤ 10 * T.card * Lb ∧
      T.card ≤ H.card := by
  obtain ⟨V, hV, hVlen⟩ :=
    A.certifiedThreeCycles hN1 hN2 horbit hd hcover hseed hs htab
  refine ⟨V, hV, hVlen.trans ?_, hN2.card_le⟩
  have hsum : ∑ i ∈ Finset.range r, (T ∩ Δ i).card ≤ T.card :=
    sum_card_inter_le fun i hi j hj hij => A.disjoint_of_seeds horbit hseed hi hj hij
  calc 2 * ∑ i ∈ Finset.range r, (T ∩ Δ i).card * (s i + 2 * e i + 2 * f i)
      ≤ 2 * ∑ i ∈ Finset.range r, (T ∩ Δ i).card * (5 * Lb) := by
        refine Nat.mul_le_mul_left 2 (Finset.sum_le_sum fun i hi => ?_)
        have hi' := Finset.mem_range.1 hi
        have := hsL i hi'
        have := heL i hi'
        have := hfL i hi'
        exact Nat.mul_le_mul_left _ (by omega)
    _ = 10 * Lb * ∑ i ∈ Finset.range r, (T ∩ Δ i).card := by
        rw [← Finset.sum_mul]
        ring
    _ ≤ 10 * Lb * T.card := Nat.mul_le_mul_left _ hsum
    _ = 10 * T.card * Lb := by ring

/-! ### Tables from short letters -/

/-- The graph on `Δ \ F` whose edges are the letters that fix `F` pointwise is connected: any two
of its chambers are joined by a word of such letters (the alphabet is closed under inverses). -/
def StabConnected (Δ F : Finset Ω) : Prop :=
  ∀ x ∈ Δ \ F, ∀ y ∈ Δ \ F, ∃ w : List L, (∀ l ∈ w, ∀ p ∈ F, A.act l p = p) ∧ A.wordPerm w x = y

omit [Finite Ω] in
/-- A breadth-first tree in a connected stabilizer graph on `Δ \ F` gives paths of at most
`|Δ \ F| - 1` letters. -/
theorem exists_short_stab_word {Δ F : Finset Ω} (hΔ : A.IsInvariant Δ) (hconn : A.StabConnected Δ F)
    {x y : Ω} (hx : x ∈ Δ \ F) (hy : y ∈ Δ \ F) :
    ∃ w : List L, (∀ l ∈ w, ∀ p ∈ F, A.act l p = p) ∧ A.wordPerm w x = y ∧
      w.length + 1 ≤ (Δ \ F).card := by
  obtain ⟨w, hwF, hwxy⟩ := hconn x hx y hy
  obtain ⟨w', hw'F, hlen, hend⟩ := exists_short_walk (fun l p => A.act l p)
    (Gen := {l | ∀ p ∈ F, A.act l p = p}) (Y := Δ \ F)
    (fun l hl p hp => by
      have hl' : ∀ q ∈ F, A.act l q = q := hl
      have h1 : A.wordPerm [l] p ∈ Δ \ F :=
        hΔ.wordPerm_mem_sdiff (fun q hq => by simpa using hl' q hq) hp
      simpa using h1) hx w hwF
  refine ⟨w', hw'F, ?_, hlen⟩
  rw [wordPerm_apply_eq_walk, hend, ← wordPerm_apply_eq_walk, hwxy]

omit [Finite Ω] in
/-- R22B-01, tables from short letters: if the graph on `Δ \ {a}` of the letters that fix `a` and
the graph on `Δ \ {a, b}` of the letters that fix `a` and `b` are connected, breadth-first trees
give tables with `e ≤ d - 2` and `f ≤ d - 3`, each word made of single letters that fix the
anchors. -/
theorem tables_of_connected {Δ : Finset Ω} (hΔ : A.IsInvariant Δ) {a b c : Ω} (ha : a ∈ Δ)
    (hb : b ∈ Δ) (hc : c ∈ Δ) (hab : a ≠ b) (hac : a ≠ c) (hbc : b ≠ c)
    (h₁ : A.StabConnected Δ {a}) (h₂ : A.StabConnected Δ {a, b}) :
    ∃ P Q : Ω → List L, A.IsTables Δ a b c P Q (Δ.card - 2) (Δ.card - 3) ∧
      (∀ x, ∀ l ∈ P x, A.act l a = a) ∧ (∀ z, ∀ l ∈ Q z, A.act l a = a ∧ A.act l b = b) := by
  have hcardA : (Δ \ {a}).card = Δ.card - 1 := by
    rw [Finset.card_sdiff_of_subset (Finset.singleton_subset_iff.2 ha), Finset.card_singleton]
  have hcardAB : (Δ \ {a, b}).card = Δ.card - 2 := by
    rw [Finset.card_sdiff_of_subset (Finset.insert_subset ha (Finset.singleton_subset_iff.2 hb)),
      Finset.card_pair hab]
  have hbA : b ∈ Δ \ {a} := by simp [hb, Ne.symm hab]
  have hcAB : c ∈ Δ \ {a, b} := by simp [hc, Ne.symm hac, Ne.symm hbc]
  have hP : ∀ x, ∃ w : List L, (∀ l ∈ w, A.act l a = a) ∧ (x ∈ Δ → x ≠ a →
      A.wordPerm w x = b ∧ w.length ≤ Δ.card - 2) := by
    intro x
    by_cases hx : x ∈ Δ ∧ x ≠ a
    · obtain ⟨w, hwF, hwx, hlen⟩ := A.exists_short_stab_word hΔ h₁ (x := x) (y := b)
        (by simp [hx.1, hx.2]) hbA
      exact ⟨w, fun l hl => hwF l hl a (Finset.mem_singleton_self a),
        fun _ _ => ⟨hwx, by omega⟩⟩
    · exact ⟨[], by simp, fun h1 h2 => absurd ⟨h1, h2⟩ hx⟩
  have hQ : ∀ z, ∃ w : List L, (∀ l ∈ w, A.act l a = a ∧ A.act l b = b) ∧ (z ∈ Δ → z ≠ a →
      z ≠ b → A.wordPerm w c = z ∧ w.length ≤ Δ.card - 3) := by
    intro z
    by_cases hz : z ∈ Δ ∧ z ≠ a ∧ z ≠ b
    · obtain ⟨w, hwF, hwz, hlen⟩ := A.exists_short_stab_word hΔ h₂ (x := c) (y := z) hcAB
        (by simp [hz.1, hz.2.1, hz.2.2])
      exact ⟨w, fun l hl => ⟨hwF l hl a (by simp), hwF l hl b (by simp)⟩,
        fun _ _ _ => ⟨hwz, by omega⟩⟩
    · exact ⟨[], by simp, fun h1 h2 h3 => absurd ⟨h1, h2, h3⟩ hz⟩
  choose P hPa hPx using hP
  choose Q hQab hQz using hQ
  refine ⟨P, Q, ⟨fun x hx hxa => ?_, fun z hz hza hzb => ?_⟩, hPa, hQab⟩
  · exact ⟨A.wordPerm_apply_of_forall (hPa x), (hPx x hx hxa).1, (hPx x hx hxa).2⟩
  · exact ⟨A.wordPerm_apply_of_forall fun l hl => (hQab z l hl).1,
      A.wordPerm_apply_of_forall fun l hl => (hQab z l hl).2, (hQz z hz hza hzb).1,
      (hQz z hz hza hzb).2⟩

/-- R22B-01 with tables from short letters: `|V| ≤ 2·Σ_i k_i·(s_i + 4d_i - 10)`. -/
theorem certifiedThreeCycles_shortLetters {H T : Finset Ω} (hN1 : A.N1 H T) (hN2 : A.N2 H T)
    {r : ℕ} {Δ : ℕ → Finset Ω} (horbit : ∀ i < r, A.IsOrbit (Δ i)) (hd : ∀ i < r, 5 ≤ (Δ i).card)
    (hcover : ∀ Δ', A.IsOrbit Δ' → 2 ≤ Δ'.card → (T ∩ Δ').Nonempty → ∃ i < r, Δ' = Δ i)
    {a b c : ℕ → Ω} {C : ℕ → List L} {s : ℕ → ℕ}
    (hseed : ∀ i < r, A.IsSeed (Δ i) ((Finset.range i).biUnion Δ) (a i) (b i) (c i) (C i))
    (hs : ∀ i < r, (C i).length ≤ s i)
    (hconn : ∀ i < r, A.StabConnected (Δ i) {a i} ∧ A.StabConnected (Δ i) {a i, b i}) :
    ∃ V : List L, T ⊆ H.image (A.wordPerm V) ∧
      V.length ≤ 2 * ∑ i ∈ Finset.range r, (T ∩ Δ i).card * (s i + 4 * (Δ i).card - 10) := by
  have htab : ∀ i, ∃ P Q : Ω → List L, i < r →
      A.IsTables (Δ i) (a i) (b i) (c i) P Q ((Δ i).card - 2) ((Δ i).card - 3) := by
    intro i
    by_cases hi : i < r
    · obtain ⟨P, Q, hPQ, -⟩ := A.tables_of_connected (horbit i hi).invariant (hseed i hi).ha
        (hseed i hi).hb (hseed i hi).hc (hseed i hi).hab (hseed i hi).hac (hseed i hi).hbc
        (hconn i hi).1 (hconn i hi).2
      exact ⟨P, Q, fun _ => hPQ⟩
    · exact ⟨fun _ => [], fun _ => [], fun h => absurd h hi⟩
  choose P Q hPQ using htab
  obtain ⟨V, hV, hVlen⟩ := A.certifiedThreeCycles hN1 hN2 horbit hd hcover hseed hs
    (fun i hi => hPQ i hi)
  refine ⟨V, hV, hVlen.trans (le_of_eq ?_)⟩
  congr 1
  refine Finset.sum_congr rfl fun i hi => ?_
  have := hd i (Finset.mem_range.1 hi)
  congr 1
  omega

/-! ### Without tables -/

omit [DecidableEq Ω] [Finite Ω] in
theorem walk_triple (w : List L) (p q t : Ω) :
    walk (fun l (u : Ω × Ω × Ω) => (A.act l u.1, A.act l u.2.1, A.act l u.2.2)) (p, q, t) w =
      (A.wordPerm w p, A.wordPerm w q, A.wordPerm w t) := by
  induction w generalizing p q t with
  | nil => rfl
  | cons l w ih => simp only [walk_cons, ih, wordPerm_cons, Perm.mul_apply]

omit [Finite Ω] in
/-- The alternating group of a set of at least five points carries any ordered triple of distinct
points to any other. -/
theorem exists_alt_triple {Δ : Finset Ω} (hd : 5 ≤ Δ.card) {a b c x y z : Δ} (hab : a ≠ b)
    (hac : a ≠ c) (hbc : b ≠ c) (hxy : x ≠ y) (hxz : x ≠ z) (hyz : y ≠ z) :
    ∃ σ ∈ alternatingGroup Δ, σ a = x ∧ σ b = y ∧ σ c = z := by
  have hcard : Nat.card Δ = Δ.card := by rw [Nat.card_eq_fintype_card, Fintype.card_coe]
  have := alternatingGroup.isMultiplyPretransitive Δ
  have : MulAction.IsMultiplyPretransitive (alternatingGroup Δ) Δ 3 :=
    MulAction.isMultiplyPretransitive_of_le (n := Nat.card Δ - 2) (by omega) (by omega)
  let e₁ : Fin 3 ↪ Δ := ⟨![a, b, c], by
    intro i j h
    fin_cases i <;> fin_cases j <;> simp_all [eq_comm]⟩
  let e₂ : Fin 3 ↪ Δ := ⟨![x, y, z], by
    intro i j h
    fin_cases i <;> fin_cases j <;> simp_all [eq_comm]⟩
  obtain ⟨g, hg⟩ := MulAction.exists_smul_eq (alternatingGroup Δ) e₁ e₂
  have hgi : ∀ i, (g : Perm Δ) (e₁ i) = e₂ i := fun i => by
    rw [← hg]
    rfl
  exact ⟨g, g.2, hgi 0, hgi 1, hgi 2⟩

omit [Finite Ω] in
/-- R22B-01, without tables, for one orbit: if `G` contains `Alt(Δ)` acting on `Δ` alone
(Theorem A's conclusion, `hAlt`) and `d ≥ 5`, a breadth-first search over the ordered triples finds
a word `u` of at most `d(d - 1)(d - 2) - 1` letters carrying the seed's triple to any wanted one,
and `u⁻¹·C·u` is the wanted three-cycle. -/
theorem exists_three_cycle_word_alt {Δ E : Finset Ω} (hΔ : A.IsInvariant Δ)
    (hE : A.IsInvariant E) (hd : 5 ≤ Δ.card) {a b c : Ω} {C : List L}
    (hC : A.IsSeed Δ E a b c C)
    (hAlt : ∀ σ : Perm Δ, σ ∈ alternatingGroup Δ → Perm.ofSubtype σ ∈ A.group)
    {x y z : Ω} (hx : x ∈ Δ) (hy : y ∈ Δ) (hz : z ∈ Δ) (hxy : x ≠ y) (hxz : x ≠ z)
    (hyz : y ≠ z) :
    ∃ W : List L, (∀ p ∈ Δ, A.wordPerm W p = cycle3 x y z p) ∧ (∀ p ∈ E, A.wordPerm W p = p) ∧
      W.length ≤ C.length + 2 * (Δ.card * (Δ.card - 1) * (Δ.card - 2) - 1) := by
  have hne : ∀ {p q : Ω} (hp : p ∈ Δ) (hq : q ∈ Δ), p ≠ q →
      (⟨p, hp⟩ : Δ) ≠ ⟨q, hq⟩ := fun _ _ h e => h (congrArg Subtype.val e)
  obtain ⟨σ, hσ, hσa, hσb, hσc⟩ := exists_alt_triple hd (hne hC.ha hC.hb hC.hab)
    (hne hC.ha hC.hc hC.hac) (hne hC.hb hC.hc hC.hbc) (hne hx hy hxy) (hne hx hz hxz)
    (hne hy hz hyz)
  obtain ⟨w, hw⟩ := (A.mem_group_iff).1 (hAlt σ hσ)
  have hwa : A.wordPerm w a = x := by
    rw [hw, Perm.ofSubtype_apply_of_mem σ hC.ha, hσa]
  have hwb : A.wordPerm w b = y := by
    rw [hw, Perm.ofSubtype_apply_of_mem σ hC.hb, hσb]
  have hwc : A.wordPerm w c = z := by
    rw [hw, Perm.ofSubtype_apply_of_mem σ hC.hc, hσc]
  -- Shorten `w` in the graph of ordered triples.
  have habc : (a, b, c) ∈ distinctTriples Δ :=
    mem_distinctTriples.2 ⟨hC.ha, hC.hb, hC.hc, hC.hab, hC.hac, hC.hbc⟩
  obtain ⟨u, -, hlen, hend⟩ := exists_short_walk
    (fun l (t : Ω × Ω × Ω) => (A.act l t.1, A.act l t.2.1, A.act l t.2.2))
    (Gen := Set.univ) (Y := distinctTriples Δ) (by
      rintro l - ⟨p, q, t⟩ hpqt
      obtain ⟨hp, hq, ht, hpq, hpt, hqt⟩ := mem_distinctTriples.1 hpqt
      have hinj := (A.act l).injective
      exact mem_distinctTriples.2 ⟨hΔ l p hp, hΔ l q hq, hΔ l t ht, fun h => hpq (hinj h),
        fun h => hpt (hinj h), fun h => hqt (hinj h)⟩) habc w (fun _ _ => Set.mem_univ _)
  rw [walk_triple, walk_triple, hwa, hwb, hwc, Prod.mk.injEq, Prod.mk.injEq] at hend
  obtain ⟨hua, hub, huc⟩ := hend
  refine ⟨A.invWord u ++ C ++ u, fun p hp => ?_, fun p hp => ?_, ?_⟩
  · rw [wordPerm_conj_apply, hC.acts _ (hΔ.inv_mem u hp), ← Perm.mul_apply,
      ← Perm.mul_apply, conj_cycle3, hua, hub, huc]
  · rw [wordPerm_conj_apply, hC.fixes _ (hE.inv_mem u hp), perm_apply_inv_self]
  · have := card_distinctTriples_le Δ
    simp only [List.length_append, length_invWord]
    omega

/-- R22B-01, without tables: if every orbit has a seed and `G` contains `Alt(Δ_i)` acting on `Δ_i`
alone (Theorem A's conclusion, the hypothesis `hAlt`; it rests on a Jordan certificate, which is not
formalized here), then `|V| ≤ Σ_i k_i·[s_i + 2(d_i(d_i - 1)(d_i - 2) - 1)]`. -/
theorem certifiedThreeCycles_alt {H T : Finset Ω} (hN1 : A.N1 H T) (hN2 : A.N2 H T) {r : ℕ}
    {Δ : ℕ → Finset Ω} (horbit : ∀ i < r, A.IsOrbit (Δ i)) (hd : ∀ i < r, 5 ≤ (Δ i).card)
    (hcover : ∀ Δ', A.IsOrbit Δ' → 2 ≤ Δ'.card → (T ∩ Δ').Nonempty → ∃ i < r, Δ' = Δ i)
    {a b c : ℕ → Ω} {C : ℕ → List L} {s : ℕ → ℕ}
    (hseed : ∀ i < r, A.IsSeed (Δ i) ((Finset.range i).biUnion Δ) (a i) (b i) (c i) (C i))
    (hs : ∀ i < r, (C i).length ≤ s i)
    (hAlt : ∀ i < r, ∀ σ : Perm (Δ i), σ ∈ alternatingGroup (Δ i) → Perm.ofSubtype σ ∈ A.group) :
    ∃ V : List L, T ⊆ H.image (A.wordPerm V) ∧
      V.length ≤ ∑ i ∈ Finset.range r, (T ∩ Δ i).card *
        (s i + 2 * ((Δ i).card * ((Δ i).card - 1) * ((Δ i).card - 2) - 1)) :=
  A.vacate_of_three_cycles hN1 hN2 horbit (fun i hi => by have := hd i hi; omega) hcover _
    (fun i hi x hx y hy z hz hxy hxz hyz => by
      obtain ⟨W, h1, h2, h3⟩ := A.exists_three_cycle_word_alt (horbit i hi).invariant
        (A.isInvariant_earlier horbit hi) (hd i hi) (hseed i hi) (hAlt i hi) hx hy hz hxy hxz hyz
      exact ⟨W, h1, h2, h3.trans (by have := hs i hi; omega)⟩)

/-! ### R22B-02 -/

omit [Finite Ω] in
theorem walk_image (w : List L) (S : Finset Ω) :
    walk (fun l (S : Finset Ω) => S.image (A.act l)) S w = S.image (A.wordPerm w) := by
  induction w generalizing S with
  | nil => simp
  | cons l w ih =>
    rw [walk_cons, ih, wordPerm_cons, Perm.coe_mul, Finset.image_image]

omit [Finite Ω] in
/-- R22B-02, a bound in `n` alone whenever `V` exists: if some SAFE-ALL word vacates `T`, a
shortest path in the graph of hole sets (at most `C(N, n)` vertices, `N = |Ω|`, `n = |H|`) repeats
no vertex, so some word of at most `C(N, n) - 1 ≤ N^n - 1` letters vacates `T`. -/
theorem vacateShortest [Fintype Ω] (H T : Finset Ω)
    (hV : ∃ V : List L, T ⊆ H.image (A.wordPerm V)) :
    ∃ V : List L, T ⊆ H.image (A.wordPerm V) ∧
      V.length ≤ (Fintype.card Ω).choose H.card - 1 ∧
      (Fintype.card Ω).choose H.card - 1 ≤ Fintype.card Ω ^ H.card - 1 := by
  obtain ⟨V, hV⟩ := hV
  obtain ⟨V', -, hlen, hend⟩ := exists_short_walk (fun l (S : Finset Ω) => S.image (A.act l))
    (Gen := Set.univ) (Y := Finset.powersetCard H.card Finset.univ) (by
      intro l _ S hS
      rw [Finset.mem_powersetCard] at hS ⊢
      exact ⟨Finset.subset_univ _, by
        rw [Finset.card_image_of_injective _ (A.act l).injective, hS.2]⟩)
    (Finset.mem_powersetCard.2 ⟨Finset.subset_univ _, rfl⟩) V (fun _ _ => Set.mem_univ _)
  rw [walk_image, walk_image] at hend
  rw [Finset.card_powersetCard, Finset.card_univ] at hlen
  refine ⟨V', hend ▸ hV, by omega, Nat.sub_le_sub_right (Nat.choose_le_pow _ _) 1⟩

omit [Finite Ω] in
/-- R22B-02, consequence: for states with at most `n₀` holes, shortest vacating words have at most
`N^{n₀} - 1` letters, so no family of states with bounded `n` has shortest vacating words of
unbounded length. -/
theorem vacateShortest_bounded [Fintype Ω] [Nonempty Ω] (H T : Finset Ω) {n₀ : ℕ} (hn : H.card ≤ n₀)
    (hV : ∃ V : List L, T ⊆ H.image (A.wordPerm V)) :
    ∃ V : List L, T ⊆ H.image (A.wordPerm V) ∧ V.length ≤ Fintype.card Ω ^ n₀ - 1 := by
  obtain ⟨V, hV, hlen, hpow⟩ := A.vacateShortest H T hV
  exact ⟨V, hV, hlen.trans (hpow.trans (Nat.sub_le_sub_right
    (Nat.pow_le_pow_right Fintype.card_pos hn) 1))⟩

/-- R22B-02, with a certificate as in R22B-01 whose lengths are all at most `L`: `|V| ≤ 10·n·L`,
`n = |H|`. -/
theorem vacateShortest_certified {H T : Finset Ω} (hN1 : A.N1 H T) (hN2 : A.N2 H T) {r : ℕ}
    {Δ : ℕ → Finset Ω} (horbit : ∀ i < r, A.IsOrbit (Δ i)) (hd : ∀ i < r, 5 ≤ (Δ i).card)
    (hcover : ∀ Δ', A.IsOrbit Δ' → 2 ≤ Δ'.card → (T ∩ Δ').Nonempty → ∃ i < r, Δ' = Δ i)
    {a b c : ℕ → Ω} {C : ℕ → List L} {P Q : ℕ → Ω → List L} {s e f : ℕ → ℕ}
    (hseed : ∀ i < r, A.IsSeed (Δ i) ((Finset.range i).biUnion Δ) (a i) (b i) (c i) (C i))
    (hs : ∀ i < r, (C i).length ≤ s i)
    (htab : ∀ i < r, A.IsTables (Δ i) (a i) (b i) (c i) (P i) (Q i) (e i) (f i))
    {Lb : ℕ} (hsL : ∀ i < r, s i ≤ Lb) (heL : ∀ i < r, e i ≤ Lb) (hfL : ∀ i < r, f i ≤ Lb) :
    ∃ V : List L, T ⊆ H.image (A.wordPerm V) ∧ V.length ≤ 10 * H.card * Lb := by
  obtain ⟨V, hV, hlen, hTH⟩ := A.certifiedThreeCycles_uniform hN1 hN2 horbit hd hcover hseed hs
    htab hsL heL hfL
  exact ⟨V, hV, hlen.trans (Nat.mul_le_mul_right _ (Nat.mul_le_mul_left _ hTH))⟩

end Alphabet

/-! ### The diagonal counterexample -/

/-- The holes `{1, 2′}` of the diagonal counterexample: two copies of five points. -/
def diagonalHoles : Finset (Fin 5 ⊕ Fin 5) := {Sum.inl 1, Sum.inr 2}

/-- The targets `{1, 1′}` of the diagonal counterexample. -/
def diagonalTargets : Finset (Fin 5 ⊕ Fin 5) := {Sum.inl 1, Sum.inr 1}

open Equiv in
/-- R22B-01, counterexample (abstract): without the condition on earlier orbits the construction is
unsound. The diagonal `A₅` on two copies of five points (`σ ↦ sumCongr σ σ`) induces every
three-cycle on each copy, but no element of it that fixes the first copy pointwise moves anything,
so no seed for the second copy fixes the first; each copy holds one hole and one target (N2 holds);
and the holes `{1, 2′}` cannot cover the targets `{1, 1′}`, since one permutation would have to send
both `1` and `2` to `1`. -/
theorem diagonalCounterexample :
    (∀ τ : Perm (Fin 5), τ.IsThreeCycle → τ ∈ alternatingGroup (Fin 5) ∧
      (∀ i, Perm.sumCongr τ τ (Sum.inl i) = Sum.inl (τ i)) ∧
      (∀ i, Perm.sumCongr τ τ (Sum.inr i) = Sum.inr (τ i))) ∧
    (∀ σ : Perm (Fin 5), (∀ i, Perm.sumCongr σ σ (Sum.inl i) = Sum.inl i) →
      ∀ x, Perm.sumCongr σ σ x = x) ∧
    ((diagonalHoles.filter fun x => x.isLeft = true).card = 1 ∧
      (diagonalTargets.filter fun x => x.isLeft = true).card = 1 ∧
      (diagonalHoles.filter fun x => x.isRight = true).card = 1 ∧
      (diagonalTargets.filter fun x => x.isRight = true).card = 1) ∧
    (∀ σ : Perm (Fin 5), ¬diagonalTargets ⊆ diagonalHoles.image (Perm.sumCongr σ σ)) := by
  refine ⟨fun τ hτ => ⟨hτ.mem_alternatingGroup, fun i => rfl, fun i => rfl⟩, ?_, by decide, ?_⟩
  · intro σ hσ x
    have h1 : ∀ i, σ i = i := fun i => Sum.inl_injective (hσ i)
    rcases x with i | i
    · exact hσ i
    · change Sum.inr (σ i) = Sum.inr i
      rw [h1 i]
  · intro σ hsub
    simp only [diagonalTargets, diagonalHoles] at hsub
    have h1 := hsub (Finset.mem_insert_self _ _)
    have h2 := hsub (Finset.mem_insert_of_mem (Finset.mem_singleton_self _))
    simp only [Finset.mem_image, Finset.mem_insert, Finset.mem_singleton] at h1 h2
    obtain ⟨x, hx, hx1⟩ := h1
    obtain ⟨y, hy, hy1⟩ := h2
    rcases hx with rfl | rfl <;> rcases hy with rfl | rfl <;>
      simp only [Perm.sumCongr_apply, Sum.map_inl, Sum.map_inr, reduceCtorEq,
        Sum.inl.injEq, Sum.inr.injEq] at hx1 hy1
    have : (1 : Fin 5) = 2 := σ.injective (hx1.trans hy1.symm)
    exact absurd this (by decide)

end Jumbling
