import Jumbling.Group.Schreier
import Jumbling.Group.Certified

set_option linter.style.header false in
/-!
# R22B-17: the bound that short seeds would give

`PROOFS-B.md` section 3, R22B-17 (layer "group" of `ARCHON-HANDOFF.md`). The statement is
conditional: it assumes the hypotheses of R22B-01, Alt on each target orbit (H1 of R22B-05, so that
R22B-05 gives the tables), and a seed bound `s_i ≤ S(d_i)`.

* `tableE`, `tableF`: `E(d) = (d - 2)(2d - 1)` and `F(d) = (d - 3)(2d - 3)(2d - 1)`, both monotone.
* `Alphabet.shortSeedBound`: `|V| ≤ 2·Σ_i k_i·[S(d_i) + 2E(d_i) + 2F(d_i)]`.
* `Alphabet.shortSeedBound_max`: if every `d_i ≤ D` and `S*(D)` is the largest `S(d)` with
  `5 ≤ d ≤ D` (`seedMax`), then `|V| ≤ 2t·[S*(D) + 2E(D) + 2F(D)]` with `t = Σ_i k_i`.
* `tableE_7200`, `tableF_7200`, `tables_7200`: `E(7200) = 103,644,002`,
  `F(7200) = 1,491,955,394,391` and `2E(7200) + 2F(7200) = 2,984,118,076,786`;
  `Alphabet.shortSeedBound_7200`: with every orbit of at most 7,200 chambers (an explicit
  hypothesis; in the geometry every SAFE letter keeps the `K⁺` chamber type) and `t ≤ n` by N1
  and N2, `|V| ≤ 2n·[S*(7200) + 2,984,118,076,786]`.

The instance constants for b00 and b19 use their exact orbit sizes, which are computed data; they
are not formalized here.
-/

namespace Jumbling

open Equiv Finset

/-- `E(d) = (d - 2)(2d - 1)`, the bound of R22B-05 on the first table. -/
def tableE (d : ℕ) : ℕ := (d - 2) * (2 * d - 1)

/-- `F(d) = (d - 3)(2d - 3)(2d - 1)`, the bound of R22B-05 on the second table. -/
def tableF (d : ℕ) : ℕ := (d - 3) * (2 * d - 3) * (2 * d - 1)

/-- R22B-17: `E` increases. -/
theorem tableE_mono : Monotone tableE := fun _ _ h =>
  Nat.mul_le_mul (Nat.sub_le_sub_right h 2) (Nat.sub_le_sub_right (Nat.mul_le_mul_left 2 h) 1)

/-- R22B-17: `F` increases. -/
theorem tableF_mono : Monotone tableF := fun _ _ h =>
  Nat.mul_le_mul (Nat.mul_le_mul (Nat.sub_le_sub_right h 3)
    (Nat.sub_le_sub_right (Nat.mul_le_mul_left 2 h) 3))
    (Nat.sub_le_sub_right (Nat.mul_le_mul_left 2 h) 1)

/-- R22B-17: `E(7200) = 103,644,002`. -/
theorem tableE_7200 : tableE 7200 = 103644002 := by
  norm_num [tableE]

/-- R22B-17: `F(7200) = 1,491,955,394,391`. -/
theorem tableF_7200 : tableF 7200 = 1491955394391 := by
  norm_num [tableF]

/-- R22B-17: `2E(7200) + 2F(7200) = 2,984,118,076,786`. -/
theorem tables_7200 : 2 * tableE 7200 + 2 * tableF 7200 = 2984118076786 := by
  rw [tableE_7200, tableF_7200]

/-- `S*(D)`: the largest `S(d)` with `5 ≤ d ≤ D` (0 if there is none). -/
def seedMax (S : ℕ → ℕ) (D : ℕ) : ℕ := (Finset.Icc 5 D).sup S

theorem le_seedMax (S : ℕ → ℕ) {d D : ℕ} (h5 : 5 ≤ d) (hD : d ≤ D) : S d ≤ seedMax S D :=
  Finset.le_sup (f := S) (Finset.mem_Icc.2 ⟨h5, hD⟩)

namespace Alphabet

variable {L Ω : Type*} [DecidableEq Ω] [Finite Ω] (A : Alphabet L Ω)

/-- R22B-17, the bound that short seeds would give. Under the hypotheses of R22B-01 (N1, N2, the
orbits `Δ_i` of size at least 2 that meet `T`, `d_i ≥ 5`, a seed `C_i` for each orbit that fixes
the earlier orbits), Alt on each target orbit (`hAlt`, the hypothesis H1 of R22B-05, so that R22B-05
gives tables with `e_i ≤ E(d_i)` and `f_i ≤ F(d_i)`) and seeds of at most `S(d_i)` letters,
`|V| ≤ 2·Σ_i k_i·[S(d_i) + 2E(d_i) + 2F(d_i)]` with `k_i = |T ∩ Δ_i|`. -/
theorem shortSeedBound {H T : Finset Ω} (hN1 : A.N1 H T) (hN2 : A.N2 H T) {r : ℕ}
    {Δ : ℕ → Finset Ω} (horbit : ∀ i < r, A.IsOrbit (Δ i)) (hd : ∀ i < r, 5 ≤ (Δ i).card)
    (hcover : ∀ Δ', A.IsOrbit Δ' → 2 ≤ Δ'.card → (T ∩ Δ').Nonempty → ∃ i < r, Δ' = Δ i)
    {a b c : ℕ → Ω} {C : ℕ → List L} (S : ℕ → ℕ)
    (hseed : ∀ i < r, A.IsSeed (Δ i) ((Finset.range i).biUnion Δ) (a i) (b i) (c i) (C i))
    (hs : ∀ i < r, (C i).length ≤ S (Δ i).card)
    (hAlt : ∀ i < r, ∀ σ : Perm (Δ i), σ ∈ alternatingGroup (Δ i) →
      ∃ g ∈ A.group, ∀ x : Δ i, g x = σ x) :
    ∃ V : List L, T ⊆ H.image (A.wordPerm V) ∧
      V.length ≤ 2 * ∑ i ∈ Finset.range r,
        (T ∩ Δ i).card * (S (Δ i).card + 2 * tableE (Δ i).card + 2 * tableF (Δ i).card) := by
  have hP : ∀ i x, ∃ w : List L, i < r → x ∈ Δ i → x ≠ a i →
      A.wordPerm w (a i) = a i ∧ A.wordPerm w x = b i ∧ w.length ≤ tableE (Δ i).card := by
    intro i x
    by_cases h : i < r ∧ x ∈ Δ i ∧ x ≠ a i
    · obtain ⟨w, hw⟩ := (A.schreierTables (horbit i h.1) (hd i h.1) (hseed i h.1).ha
        (hseed i h.1).hb (hseed i h.1).hc (hseed i h.1).hab (hseed i h.1).hac (hseed i h.1).hbc
        (hAlt i h.1)).1 x h.2.1 h.2.2
      exact ⟨w, fun _ _ _ => hw⟩
    · exact ⟨[], fun hi hx hxa => absurd ⟨hi, hx, hxa⟩ h⟩
  have hQ : ∀ i z, ∃ w : List L, i < r → z ∈ Δ i → z ≠ a i → z ≠ b i →
      A.wordPerm w (a i) = a i ∧ A.wordPerm w (b i) = b i ∧ A.wordPerm w (c i) = z ∧
        w.length ≤ tableF (Δ i).card := by
    intro i z
    by_cases h : i < r ∧ z ∈ Δ i ∧ z ≠ a i ∧ z ≠ b i
    · obtain ⟨w, hw⟩ := (A.schreierTables (horbit i h.1) (hd i h.1) (hseed i h.1).ha
        (hseed i h.1).hb (hseed i h.1).hc (hseed i h.1).hab (hseed i h.1).hac (hseed i h.1).hbc
        (hAlt i h.1)).2 z h.2.1 h.2.2.1 h.2.2.2
      exact ⟨w, fun _ _ _ _ => hw⟩
    · exact ⟨[], fun hi hz hza hzb => absurd ⟨hi, hz, hza, hzb⟩ h⟩
  choose P hP using hP
  choose Q hQ using hQ
  exact A.certifiedThreeCycles hN1 hN2 horbit hd hcover (s := fun i => S (Δ i).card)
    (e := fun i => tableE (Δ i).card) (f := fun i => tableF (Δ i).card) hseed hs
    (fun i hi => ⟨fun x hx hxa => hP i x hi hx hxa, fun z hz hza hzb => hQ i z hi hz hza hzb⟩)

/-- R22B-17 with a size bound: if moreover every `d_i ≤ D`, then, since `E` and `F` increase,
`|V| ≤ 2t·[S*(D) + 2E(D) + 2F(D)]` with `t = Σ_i k_i` and `S*(D)` the largest `S(d)` with
`5 ≤ d ≤ D`. -/
theorem shortSeedBound_max {H T : Finset Ω} (hN1 : A.N1 H T) (hN2 : A.N2 H T) {r : ℕ}
    {Δ : ℕ → Finset Ω} (horbit : ∀ i < r, A.IsOrbit (Δ i)) (hd : ∀ i < r, 5 ≤ (Δ i).card)
    (hcover : ∀ Δ', A.IsOrbit Δ' → 2 ≤ Δ'.card → (T ∩ Δ').Nonempty → ∃ i < r, Δ' = Δ i)
    {a b c : ℕ → Ω} {C : ℕ → List L} (S : ℕ → ℕ)
    (hseed : ∀ i < r, A.IsSeed (Δ i) ((Finset.range i).biUnion Δ) (a i) (b i) (c i) (C i))
    (hs : ∀ i < r, (C i).length ≤ S (Δ i).card)
    (hAlt : ∀ i < r, ∀ σ : Perm (Δ i), σ ∈ alternatingGroup (Δ i) →
      ∃ g ∈ A.group, ∀ x : Δ i, g x = σ x)
    {D : ℕ} (hD : ∀ i < r, (Δ i).card ≤ D) :
    ∃ V : List L, T ⊆ H.image (A.wordPerm V) ∧
      V.length ≤ 2 * (∑ i ∈ Finset.range r, (T ∩ Δ i).card) *
        (seedMax S D + 2 * tableE D + 2 * tableF D) := by
  obtain ⟨V, hV, hlen⟩ :=
    A.shortSeedBound hN1 hN2 horbit hd hcover S hseed hs hAlt
  refine ⟨V, hV, hlen.trans ?_⟩
  rw [mul_assoc, Finset.sum_mul]
  refine Nat.mul_le_mul_left 2 (Finset.sum_le_sum fun i hi => Nat.mul_le_mul_left _ ?_)
  have hi' := Finset.mem_range.1 hi
  have h1 := le_seedMax S (hd i hi') (hD i hi')
  have h2 := tableE_mono (hD i hi')
  have h3 := tableF_mono (hD i hi')
  omega

/-- R22B-17 for the geometry: if every target orbit has at most 7,200 chambers (an explicit
hypothesis; there every SAFE letter keeps the `K⁺` chamber type, of at most 7,200 chambers), then
`t ≤ |T| ≤ n` by N1 and N2, and `|V| ≤ 2n·[S*(7200) + 2,984,118,076,786]` with `n = |H|`. -/
theorem shortSeedBound_7200 {H T : Finset Ω} (hN1 : A.N1 H T) (hN2 : A.N2 H T) {r : ℕ}
    {Δ : ℕ → Finset Ω} (horbit : ∀ i < r, A.IsOrbit (Δ i)) (hd : ∀ i < r, 5 ≤ (Δ i).card)
    (hcover : ∀ Δ', A.IsOrbit Δ' → 2 ≤ Δ'.card → (T ∩ Δ').Nonempty → ∃ i < r, Δ' = Δ i)
    {a b c : ℕ → Ω} {C : ℕ → List L} (S : ℕ → ℕ)
    (hseed : ∀ i < r, A.IsSeed (Δ i) ((Finset.range i).biUnion Δ) (a i) (b i) (c i) (C i))
    (hs : ∀ i < r, (C i).length ≤ S (Δ i).card)
    (hAlt : ∀ i < r, ∀ σ : Perm (Δ i), σ ∈ alternatingGroup (Δ i) →
      ∃ g ∈ A.group, ∀ x : Δ i, g x = σ x)
    (hD : ∀ i < r, (Δ i).card ≤ 7200) :
    ∃ V : List L, T ⊆ H.image (A.wordPerm V) ∧
      V.length ≤ 2 * H.card * (seedMax S 7200 + 2984118076786) := by
  obtain ⟨V, hV, hlen⟩ :=
    A.shortSeedBound_max hN1 hN2 horbit hd hcover S hseed hs hAlt hD
  refine ⟨V, hV, hlen.trans ?_⟩
  have ht : ∑ i ∈ Finset.range r, (T ∩ Δ i).card ≤ H.card :=
    (sum_card_inter_le fun i hi j hj hij => A.disjoint_of_seeds horbit hseed hi hj hij).trans
      hN2.card_le
  rw [add_assoc, tables_7200]
  exact Nat.mul_le_mul_right _ (Nat.mul_le_mul_left 2 ht)

end Alphabet

end Jumbling
