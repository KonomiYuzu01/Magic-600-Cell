import Jumbling.Group.Certified
import Jumbling.Group.Cluster
import Jumbling.Classical.JordanPrimeCycle

set_option linter.style.header false in
/-!
# R22B-15: a local test for Alt

`PROOFS-B.md` section 3, R22B-15 (layer "group" of `ARCHON-HANDOFF.md`).

* `Alphabet.inducedPerm` and `Alphabet.inducedGroup`: the permutation `π_w|Δ` that a word induces
  on an invariant set `Δ`, and the group `G|Δ` that `G` induces on `Δ`.
* `Alphabet.alternatingGroup_le_inducedGroup`: if `Δ` is an orbit and `G|Δ` contains a cycle of
  prime length `p` with `d/2 < p ≤ d - 3`, then `G|Δ` contains `Alt(Δ)`: `G|Δ` is transitive, the
  long-prime-cycle block argument of Theorem A makes it primitive
  (`Classical.isPreprimitive_of_isCycle_mem`), and Jordan's theorem
  (`Classical.alternatingGroup_le_of_isPreprimitive_of_isCycle_mem`) gives `Alt(Δ)`.
* `Alphabet.localAlt` (R22B-15, the local test): if some word acts on `Δ` with a `p`-cycle, `G|Δ`
  contains `Alt(Δ)`; nothing is asked of the other orbits. `Alphabet.localAlt_words` states it with
  words, and `eight_le_of_prime` shows that `d ≥ 8` follows from `d/2 < p ≤ d - 3`.
* `Alphabet.vacate_iff_short` (R22B-15, without Alt): the exact condition for a vacating word is
  `π_V⁻¹(T) ⊆ H` (`clusterExactTest`, R22B-03), and breadth-first search over the joint images of
  the target decides it: if some word vacates `T`, one of at most `∏_Δ C(|Δ|, |T ∩ Δ|) - 1` letters
  does. It is the argument of R22B-02 (`Alphabet.vacateShortest`) on target images.
* `cyclicCounterexample` (R22B-15, abstract counterexample): with `r = (0 1 2 3 4 5)` and letters
  `r², r⁴, r³` the group is cyclic of order 6, imprimitive and without `A₆`; the holes `{0, 3}`
  cannot cover `T = {0, 1}` although N2 holds, yet they cover `{1, 4}`.

The remark on `5 ≤ d ≤ 7` (an exact closure computation), the remark on local and global Alt (no
formal content beyond R22B-01's diagonal counterexample, `diagonalCounterexample`) and the open
claim that the SAFE geometry forces either condition at every state of `𝒞₂` are not formalized.
-/

namespace Jumbling

open Equiv Equiv.Perm Finset MulAction
open scoped Pointwise

/-- The hypotheses `d/2 < p ≤ d - 3` of R22B-15 force `d ≥ 8`: no prime lies in `(d/2, d - 3]`
when `d ≤ 7`. -/
theorem eight_le_of_prime {d p : ℕ} (hp : p.Prime) (hlong : d < 2 * p) (hshort : p + 3 ≤ d) :
    8 ≤ d := by
  by_contra h
  have h2 := hp.two_le
  have h4 : p ≠ 4 := by
    rintro rfl
    exact (by decide : ¬Nat.Prime 4) hp
  omega

namespace Alphabet

variable {L Ω : Type*} (A : Alphabet L Ω)

/-! ### The group induced on an invariant set -/

/-- The permutation `π_w|Δ` that a word `w` induces on an invariant set `Δ`. -/
def inducedPerm {Δ : Finset Ω} (hΔ : A.IsInvariant Δ) (w : List L) : Perm Δ :=
  (A.wordPerm w).subtypePerm fun _ => hΔ.wordPerm_mem_iff w

@[simp]
theorem inducedPerm_apply {Δ : Finset Ω} (hΔ : A.IsInvariant Δ) (w : List L) (x : Δ) :
    (A.inducedPerm hΔ w x : Ω) = A.wordPerm w x :=
  rfl

theorem inducedPerm_append {Δ : Finset Ω} (hΔ : A.IsInvariant Δ) (u v : List L) :
    A.inducedPerm hΔ (u ++ v) = A.inducedPerm hΔ v * A.inducedPerm hΔ u := by
  ext x
  simp

theorem inducedPerm_invWord {Δ : Finset Ω} (hΔ : A.IsInvariant Δ) (w : List L) :
    A.inducedPerm hΔ (A.invWord w) = (A.inducedPerm hΔ w)⁻¹ := by
  rw [eq_inv_iff_mul_eq_one, ← inducedPerm_append]
  ext x
  simp

/-- The group `G|Δ` that `G` induces on an invariant set `Δ`: the permutations of `Δ` induced by
words. -/
def inducedGroup {Δ : Finset Ω} (hΔ : A.IsInvariant Δ) : Subgroup (Perm Δ) where
  carrier := Set.range (A.inducedPerm hΔ)
  mul_mem' := by
    rintro _ _ ⟨u, rfl⟩ ⟨v, rfl⟩
    exact ⟨v ++ u, A.inducedPerm_append hΔ v u⟩
  one_mem' := ⟨[], by ext; rfl⟩
  inv_mem' := by
    rintro _ ⟨u, rfl⟩
    exact ⟨A.invWord u, A.inducedPerm_invWord hΔ u⟩

theorem mem_inducedGroup {Δ : Finset Ω} (hΔ : A.IsInvariant Δ) {σ : Perm Δ} :
    σ ∈ A.inducedGroup hΔ ↔ ∃ w, A.inducedPerm hΔ w = σ :=
  Iff.rfl

/-- `G|Δ` is transitive on an orbit `Δ`. -/
theorem isPretransitive_inducedGroup {Δ : Finset Ω} (hΔ : A.IsOrbit Δ) :
    IsPretransitive (A.inducedGroup hΔ.invariant) Δ := by
  refine ⟨fun x y => ?_⟩
  obtain ⟨w, hw⟩ := hΔ.connected x x.2 y y.2
  exact ⟨⟨A.inducedPerm hΔ.invariant w, w, rfl⟩, Subtype.ext hw⟩

variable [DecidableEq Ω]

/-- Theorem A, steps 3 and 4, on an orbit (used by R22B-08, step 2, and R22B-15): if `G|Δ`
contains a cycle of prime length `p` with `d/2 < p ≤ d - 3`, it contains `Alt(Δ)`. -/
theorem alternatingGroup_le_inducedGroup {Δ : Finset Ω} (hΔ : A.IsOrbit Δ) {σ : Perm Δ}
    (hσ : σ ∈ A.inducedGroup hΔ.invariant) (hcyc : σ.IsCycle) (hp : σ.support.card.Prime)
    (hlong : Δ.card < 2 * σ.support.card) (hshort : σ.support.card + 3 ≤ Δ.card) :
    alternatingGroup Δ ≤ A.inducedGroup hΔ.invariant := by
  have := A.isPretransitive_inducedGroup hΔ
  exact Classical.alternatingGroup_le_of_isPretransitive_of_isCycle_mem hcyc hp hσ
    (by rwa [Fintype.card_coe]) (by rwa [Fintype.card_coe])

/-! ### R22B-15, the local test -/

/-- **R22B-15, the local test.** Let `Δ` be an orbit with `d` chambers and `p` a prime with
`d/2 < p ≤ d - 3` (so `d ≥ 8`, `eight_le_of_prime`). If some word `w` acts on `Δ` with a `p`-cycle
(`p` is a cycle length of `π_w|Δ`), the group `G|Δ` that `G` induces on `Δ` contains `Alt(Δ)`.
Nothing is asked of the other orbits. The other cycles of `w` on `Δ` have fewer than `p` points, so
the power of `w` by the least common multiple of their lengths acts on `Δ` as one `p`-cycle; `G|Δ`
is transitive, primitive by the long-prime-cycle block argument, and contains `Alt(Δ)` by Jordan's
theorem. -/
theorem localAlt {Δ : Finset Ω} (hΔ : A.IsOrbit Δ) {w : List L} {p : ℕ} (hp : p.Prime)
    (hlong : Δ.card < 2 * p) (hshort : p + 3 ≤ Δ.card)
    (hw : p ∈ (A.inducedPerm hΔ.invariant w).cycleType) :
    alternatingGroup Δ ≤ A.inducedGroup hΔ.invariant := by
  set τ := A.inducedPerm hΔ.invariant w with hτ
  rw [cycleType_def, Multiset.mem_map] at hw
  obtain ⟨c, hc, hcp⟩ := hw
  have hcp' : c.support.card = p := hcp
  -- Every other cycle of `w` on `Δ` has fewer than `p` points.
  have hother : ∀ c' ∈ τ.cycleFactorsFinset, c' ≠ c → ¬c.support.card ∣ c'.support.card := by
    intro c' hc' hne hdvd
    have hdisj : _root_.Disjoint c'.support c.support :=
      (cycleFactorsFinset_pairwise_disjoint τ hc' hc hne).disjoint_support
    have h1 : c'.support.card + c.support.card ≤ Δ.card := by
      rw [← card_union_of_disjoint hdisj, ← Fintype.card_coe Δ]
      exact card_le_univ _
    have h2 : 2 ≤ c'.support.card := (mem_cycleFactorsFinset_iff.1 hc').1.two_le_card_support
    have := Nat.le_of_dvd (by omega) hdvd
    omega
  obtain ⟨hpow, hcyc, hsupp⟩ := Classical.pow_orderOf_mul_inv hc (hcp' ▸ hp) hother
  refine A.alternatingGroup_le_inducedGroup hΔ (σ := c ^ orderOf (τ * c⁻¹)) ?_ hcyc ?_ ?_ ?_
  · rw [← hpow]
    exact pow_mem (show τ ∈ A.inducedGroup hΔ.invariant from ⟨w, rfl⟩) _
  · rwa [hsupp, hcp']
  · rwa [hsupp, hcp']
  · rwa [hsupp, hcp']

/-- R22B-15, the local test, with words: under the hypotheses of `localAlt`, every even permutation
of `Δ` is induced on `Δ` by a word. -/
theorem localAlt_words {Δ : Finset Ω} (hΔ : A.IsOrbit Δ) {w : List L} {p : ℕ} (hp : p.Prime)
    (hlong : Δ.card < 2 * p) (hshort : p + 3 ≤ Δ.card)
    (hw : p ∈ (A.inducedPerm hΔ.invariant w).cycleType) (σ : Perm Δ)
    (hσ : σ ∈ alternatingGroup Δ) : ∃ u : List L, ∀ x : Δ, A.wordPerm u x = σ x := by
  obtain ⟨u, hu⟩ := A.localAlt hΔ hp hlong hshort hw hσ
  exact ⟨u, fun x => by rw [← hu]; rfl⟩

/-! ### R22B-15, without Alt -/

variable [Finite Ω]

/-- **R22B-15, without Alt.** A word `V` vacates `T` exactly when `π_V⁻¹(T) ⊆ H` (R22B-03,
`clusterExactTest`). Breadth-first search over the joint images `T·u` of the target decides it:
every image meets each orbit `Δ_i` in `|T ∩ Δ_i|` chambers, so there are at most
`∏_i C(|Δ_i|, |T ∩ Δ_i|)` of them, an image inside `H` gives `V` as the inverse of its path word,
and a shortest path repeats no vertex. So if some word vacates `T`, one of at most that number minus
one letters does, and an exhausted search shows that none exists. The orbits `Δ_i` only need to be
invariant and to cover `Ω`. -/
theorem vacate_iff_short {H T : Finset Ω} {r : ℕ} {Δ : Fin r → Finset Ω}
    (hinv : ∀ i, A.IsInvariant (Δ i)) (hcover : ∀ x, ∃ i, x ∈ Δ i) :
    (∃ V : List L, T ⊆ H.image (A.wordPerm V)) ↔
      ∃ V : List L, T ⊆ H.image (A.wordPerm V) ∧
        V.length ≤ (∏ i, (Δ i).card.choose (T ∩ Δ i).card) - 1 := by
  refine ⟨fun ⟨V₀, hV₀⟩ => ?_, fun ⟨V, hV, _⟩ => ⟨V, hV⟩⟩
  have := Fintype.ofFinite Ω
  -- The vertices: sets meeting each orbit in as many chambers as `T` does.
  let Y : Finset (Finset Ω) := univ.filter fun S => ∀ i, (S ∩ Δ i).card = (T ∩ Δ i).card
  have hTY : T ∈ Y := by simp [Y]
  have hY : ∀ l ∈ (Set.univ : Set L), ∀ S ∈ Y, S.image (A.act l) ∈ Y := by
    intro l _ S hS
    simp only [Y, mem_filter, mem_univ, true_and] at hS ⊢
    intro i
    rw [← hS i, ← A.card_image_inter (hinv i) S [l], wordPerm_singleton]
  have hYcard : Y.card ≤ ∏ i, (Δ i).card.choose (T ∩ Δ i).card := by
    have hinj : Set.InjOn (fun S : Finset Ω => fun i => S ∩ Δ i) Y := by
      intro S _ S' _ hSS'
      ext x
      obtain ⟨i, hi⟩ := hcover x
      have := congrArg (fun f => x ∈ f i) hSS'
      simpa [hi] using this
    have hmaps : ∀ S ∈ Y, (fun i => S ∩ Δ i) ∈
        Fintype.piFinset fun i => (Δ i).powersetCard (T ∩ Δ i).card := by
      intro S hS
      simp only [Y, mem_filter, mem_univ, true_and] at hS
      simp [Fintype.mem_piFinset, mem_powersetCard, hS]
    have := card_le_card_of_injOn _ hmaps hinj
    simpa [Fintype.card_piFinset, card_powersetCard] using this
  -- Shorten the path word `V₀⁻¹` of the image `T·V₀⁻¹ ⊆ H`.
  have hu₀ : T.image (A.wordPerm (A.invWord V₀)) ⊆ H := by
    rw [wordPerm_invWord]
    exact (clusterExactTest _ H T).1 hV₀
  obtain ⟨u, -, hlen, hend⟩ := exists_short_walk (fun l (S : Finset Ω) => S.image (A.act l))
    (Gen := Set.univ) hY hTY (A.invWord V₀) (fun _ _ => Set.mem_univ _)
  rw [walk_image, walk_image] at hend
  refine ⟨A.invWord u, ?_, ?_⟩
  · rw [clusterExactTest, wordPerm_invWord, inv_inv, hend]
    exact hu₀
  · rw [length_invWord]
    omega

end Alphabet

/-! ### The abstract counterexample -/

/-- The rotation `r = (0 1 2 3 4 5)` of the counterexample of R22B-15. -/
def rot6 : Perm (Fin 6) where
  toFun i := i + 1
  invFun i := i - 1
  left_inv i := by simp
  right_inv i := by simp

/-- The alphabet of the counterexample of R22B-15: letters `r²`, `r⁴`, `r³`, with `r²` and `r⁴`
inverse to each other and `r³` its own inverse. -/
def cyclicAlphabet : Alphabet (Fin 3) (Fin 6) where
  act := ![rot6 ^ 2, rot6 ^ 4, rot6 ^ 3]
  inv := ![1, 0, 2]
  act_inv := by decide

/-- Every word of the counterexample acts as a power `r ^ k`, `k < 6`. -/
theorem cyclicAlphabet_wordPerm (w : List (Fin 3)) :
    ∃ k : Fin 6, cyclicAlphabet.wordPerm w = rot6 ^ (k : ℕ) := by
  induction w with
  | nil => exact ⟨0, rfl⟩
  | cons l w ih =>
    obtain ⟨k, hk⟩ := ih
    rw [Alphabet.wordPerm_cons, hk]
    have key : ∀ k : Fin 6, ∀ l : Fin 3, ∃ k' : Fin 6,
        rot6 ^ (k : ℕ) * cyclicAlphabet.act l = rot6 ^ (k' : ℕ) := by decide
    exact key k l

/-- The holes `{0, 3}` of the counterexample. -/
def cyclicHoles : Finset (Fin 6) := {0, 3}

/-- The target `{0, 1}` of the counterexample. -/
def cyclicTargets : Finset (Fin 6) := {0, 1}

/-- **R22B-15, counterexample (abstract, not a state of the puzzle).** With `r = (0 1 2 3 4 5)` and
letters `r²`, `r⁴`, `r³`:
1. the group is the cyclic group of order 6 generated by `r`;
2. it is imprimitive (`{0, 3}` is a nontrivial block) and does not contain `A₆`;
3. N2 holds for the holes `{0, 3}` and the target `T = {0, 1}`, yet no word vacates `T`;
4. a word vacates `{1, 4}`.
So the lack of Alt does not by itself forbid a vacating word. -/
theorem cyclicCounterexample :
    cyclicAlphabet.group = Subgroup.zpowers rot6 ∧ orderOf rot6 = 6 ∧
    ¬IsPreprimitive cyclicAlphabet.group (Fin 6) ∧
    ¬alternatingGroup (Fin 6) ≤ cyclicAlphabet.group ∧
    cyclicAlphabet.N2 cyclicHoles cyclicTargets ∧
    (∀ V : List (Fin 3), ¬cyclicTargets ⊆ cyclicHoles.image (cyclicAlphabet.wordPerm V)) ∧
    ({1, 4} : Finset (Fin 6)) ⊆ cyclicHoles.image (cyclicAlphabet.wordPerm [1, 2]) := by
  have hmem : ∀ g ∈ cyclicAlphabet.group, ∃ k : Fin 6, g = rot6 ^ (k : ℕ) := by
    intro g hg
    obtain ⟨w, rfl⟩ := cyclicAlphabet.mem_group_iff.1 hg
    exact cyclicAlphabet_wordPerm w
  have hrot : rot6 ∈ cyclicAlphabet.group := by
    rw [Alphabet.mem_group_iff]
    exact ⟨[1, 2], by decide⟩
  refine ⟨?_, ?_, ?_, ?_, ?_, ?_, by decide⟩
  · refine le_antisymm ?_ ((Subgroup.zpowers_le).2 hrot)
    intro g hg
    obtain ⟨k, rfl⟩ := hmem g hg
    exact Subgroup.npow_mem_zpowers _ _
  · rw [orderOf_eq_iff (by norm_num)]
    decide
  · intro hprim
    have hblock : IsBlock cyclicAlphabet.group ((cyclicHoles : Finset (Fin 6)) : Set (Fin 6)) := by
      rw [isBlock_iff_smul_eq_or_disjoint]
      intro g
      obtain ⟨k, hk⟩ := hmem g g.2
      have hset : g • ((cyclicHoles : Finset (Fin 6)) : Set (Fin 6)) =
          ((cyclicHoles.image (rot6 ^ (k : ℕ)) : Finset (Fin 6)) : Set (Fin 6)) := by
        ext x
        simp only [Set.mem_smul_set, Finset.coe_image, Set.mem_image, Finset.mem_coe,
          Subgroup.smul_def, Perm.smul_def, hk]
      have key : ∀ k : Fin 6, cyclicHoles.image (rot6 ^ (k : ℕ)) = cyclicHoles ∨
          _root_.Disjoint (cyclicHoles.image (rot6 ^ (k : ℕ))) cyclicHoles := by decide
      rw [hset, Finset.coe_inj, Finset.disjoint_coe]
      exact key k
    rcases hprim.isTrivialBlock_of_isBlock hblock with h | h
    · have := h (by simp [cyclicHoles] : (0 : Fin 6) ∈ ((cyclicHoles : Finset (Fin 6)) : Set _))
        (by simp [cyclicHoles] : (3 : Fin 6) ∈ ((cyclicHoles : Finset (Fin 6)) : Set _))
      exact absurd this (by decide)
    · have : (1 : Fin 6) ∈ ((cyclicHoles : Finset (Fin 6)) : Set (Fin 6)) := by
        rw [h]
        exact Set.mem_univ _
      simp [cyclicHoles] at this
  · intro hle
    have h3 : (swap (0 : Fin 6) 1 * swap 0 2) ∈ alternatingGroup (Fin 6) :=
      (isThreeCycle_swap_mul_swap_same (a := (0 : Fin 6)) (b := 1) (c := 2) (by decide)
        (by decide) (by decide)).mem_alternatingGroup
    obtain ⟨k, hk⟩ := hmem _ (hle h3)
    have key : ∀ k : Fin 6, swap (0 : Fin 6) 1 * swap 0 2 ≠ rot6 ^ (k : ℕ) := by decide
    exact key k hk
  · intro Δ hΔ
    -- The only orbit is everything.
    have hall : Δ = univ := by
      obtain ⟨x, hx⟩ := hΔ.nonempty
      refine eq_univ_of_forall fun y => ?_
      have key : ∀ x y : Fin 6, y = (rot6 ^ ((y - x : Fin 6) : ℕ)) x := by decide
      obtain ⟨k, rfl⟩ : ∃ k : Fin 6, y = (rot6 ^ (k : ℕ)) x := ⟨y - x, key x y⟩
      have hstep : ∀ z ∈ Δ, rot6 z ∈ Δ := fun z hz => by
        have := hΔ.invariant.wordPerm_mem [1, 2] hz
        rwa [show cyclicAlphabet.wordPerm [1, 2] = rot6 by decide] at this
      have : ∀ n : ℕ, (rot6 ^ n) x ∈ Δ := by
        intro n
        induction n with
        | zero => exact hx
        | succ n ih => rw [pow_succ', Perm.mul_apply]; exact hstep _ ih
      exact this k
    subst hall
    decide
  · intro V hV
    obtain ⟨k, hk⟩ := cyclicAlphabet_wordPerm V
    rw [hk] at hV
    have key : ∀ k : Fin 6, ¬cyclicTargets ⊆ cyclicHoles.image (rot6 ^ (k : ℕ)) := by decide
    exact key k hV

end Jumbling
