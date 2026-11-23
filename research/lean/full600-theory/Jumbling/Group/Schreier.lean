import Jumbling.Group.Words

set_option linter.style.header false in
/-!
# R22B-05: tables of Schreier words

`PROOFS-B.md` section 3, R22B-05 (layer "group" of `ARCHON-HANDOFF.md`). For an orbit `Δ` of at
least five chambers on which the group induces a group containing `Alt(Δ)` (H1), and distinct
anchors `a, b, c ∈ Δ`, there are words `P_x` (fix `a`, send `x` to `b`) of at most
`(d - 2)(2d - 1)` letters and `Q_z` (fix `a` and `b`, send `c` to `z`) of at most
`(d - 3)(2d - 3)(2d - 1)` letters, `d = |Δ|`.

The proof is the source's: a breadth-first tree `R` on `Δ` from `a` (step 1), the Schreier
words `R_x·s·R_{x·s}⁻¹`, which generate the stabilizer of `a` by telescoping (step 2), a
breadth-first tree from `b` over them (step 3), and the same two steps one level down for the
stabilizer of `a` and `b` (step 4). The source cites Schreier's lemma and "the point stabilizer of
`Alt(d)` is `Alt(d - 1)`"; here the telescoping is proved directly (`wordPerm_schreierChain`), and
the transitivity that steps 3 and 4 need is taken from H1 with a three-cycle
(`exists_word_of_alt`).

H1 is stated as: every even permutation of `Δ` is induced by an element of `G`.
-/

namespace Jumbling

namespace Alphabet

open Equiv Finset

variable {L Ω : Type*} (A : Alphabet L Ω)

/-! ### Telescoping Schreier words -/

/-- The Schreier words along a list of steps: from a point `y`, each step `g` gives the word
`R_y·g·R_{y·g}⁻¹` (R22B-05, steps 2 and 4). -/
def schreierChain (R : Ω → List L) : Ω → List (List L) → List (List L)
  | _, [] => []
  | y, g :: gs => (R y ++ g ++ A.invWord (R (A.wordPerm g y))) ::
      schreierChain R (A.wordPerm g y) gs

/-- The telescoping of R22B-05, step 2: the product of the Schreier words along `g₁ ⋯ g_t` from `y`
is `R_y·g₁ ⋯ g_t·R_{y·g₁⋯g_t}⁻¹`. -/
theorem wordPerm_schreierChain (R : Ω → List L) (y : Ω) (gs : List (List L)) :
    A.wordPerm (A.schreierChain R y gs).flatten =
      (A.wordPerm (R (A.wordPerm gs.flatten y)))⁻¹ * A.wordPerm gs.flatten * A.wordPerm (R y) := by
  induction gs generalizing y with
  | nil => simp [schreierChain]
  | cons g gs ih =>
    simp only [schreierChain, List.flatten_cons, wordPerm_append, wordPerm_invWord, ih,
      Perm.mul_apply]
    group

variable [DecidableEq Ω]

/-- The Schreier generators: the words `R_z·g·R_{z·g}⁻¹` for `z ∈ Δ \ F` and `g ∈ Gen`. -/
def schreierGen (Δ F : Finset Ω) (Gen : Set (List L)) (R : Ω → List L) : Set (List L) :=
  {u | ∃ z ∈ Δ \ F, ∃ g ∈ Gen, u = R z ++ g ++ A.invWord (R (A.wordPerm g z))}

theorem schreierChain_mem {Δ F : Finset Ω} (hΔ : A.IsInvariant Δ) {Gen : Set (List L)}
    (hGenF : ∀ g ∈ Gen, ∀ p ∈ F, A.wordPerm g p = p) (R : Ω → List L) :
    ∀ (gs : List (List L)), (∀ g ∈ gs, g ∈ Gen) → ∀ y ∈ Δ \ F,
      ∀ u ∈ A.schreierChain R y gs, u ∈ A.schreierGen Δ F Gen R := by
  intro gs
  induction gs with
  | nil => intro _ y _ u hu; simp [schreierChain] at hu
  | cons g gs ih =>
    intro hgs y hy u hu
    have hg : g ∈ Gen := hgs g List.mem_cons_self
    simp only [schreierChain, List.mem_cons] at hu
    rcases hu with rfl | hu
    · exact ⟨y, hy, g, hg, rfl⟩
    · exact ih (fun g' hg' => hgs g' (List.mem_cons_of_mem g hg')) _
        (hΔ.wordPerm_mem_sdiff (hGenF g hg) hy) u hu

/-- One level of R22B-05 (step 2 with `F = ∅`, step 4 with `F = {a}`): if `R` is a tree of words
from `y₀` over the steps `Gen`, the Schreier generators fix `F` and `y₀`, have at most `2m + ℓ`
letters, and every product of steps that fixes `y₀` has the permutation of a product of Schreier
generators. -/
theorem schreier_level {Δ F : Finset Ω} (hΔ : A.IsInvariant Δ) {y₀ : Ω} (hy₀ : y₀ ∈ Δ \ F)
    {Gen : Set (List L)} {ℓ m : ℕ} (hGenF : ∀ g ∈ Gen, ∀ p ∈ F, A.wordPerm g p = p)
    (hGenℓ : ∀ g ∈ Gen, g.length ≤ ℓ) (R : Ω → List L) (hR₀ : R y₀ = [])
    (hR : ∀ z ∈ Δ \ F, A.wordPerm (R z) y₀ = z ∧ (∀ p ∈ F, A.wordPerm (R z) p = p) ∧
      (R z).length ≤ m) :
    (∀ u ∈ A.schreierGen Δ F Gen R,
      (∀ p ∈ insert y₀ F, A.wordPerm u p = p) ∧ u.length ≤ 2 * m + ℓ) ∧
    (∀ gs : List (List L), (∀ g ∈ gs, g ∈ Gen) → A.wordPerm gs.flatten y₀ = y₀ →
      ∃ us : List (List L), (∀ u ∈ us, u ∈ A.schreierGen Δ F Gen R) ∧
        A.wordPerm us.flatten = A.wordPerm gs.flatten) := by
  refine ⟨?_, ?_⟩
  · rintro u ⟨z, hz, g, hg, rfl⟩
    have hgz : A.wordPerm g z ∈ Δ \ F := hΔ.wordPerm_mem_sdiff (hGenF g hg) hz
    obtain ⟨hRz, hRzF, hRzm⟩ := hR z hz
    obtain ⟨hRgz, hRgzF, hRgzm⟩ := hR _ hgz
    refine ⟨?_, ?_⟩
    · intro p hp
      rw [wordPerm_append_apply, wordPerm_append_apply]
      rcases Finset.mem_insert.1 hp with rfl | hp
      · rw [hRz]
        exact A.wordPerm_invWord_apply_of_apply hRgz
      · rw [hRzF p hp, hGenF g hg p hp]
        exact A.wordPerm_invWord_apply_of_apply (hRgzF p hp)
    · simp only [List.length_append, length_invWord]
      have := hGenℓ g hg
      omega
  · intro gs hgs hfix
    refine ⟨A.schreierChain R y₀ gs, A.schreierChain_mem hΔ hGenF R gs hgs y₀ hy₀, ?_⟩
    rw [wordPerm_schreierChain, hfix, hR₀, wordPerm_nil, inv_one, one_mul, mul_one]

/-- The breadth-first tree over Schreier generators (R22B-05, steps 3 and 4): a point reached from
`u` by generators that fix `K` and keep `Δ` is reached by at most `|Δ \ K| - 1` of them. -/
theorem exists_short_chain {Δ K : Finset Ω} (hΔ : A.IsInvariant Δ) {Gen : Set (List L)} {ℓ : ℕ}
    (hGenK : ∀ g ∈ Gen, ∀ p ∈ K, A.wordPerm g p = p) (hGenℓ : ∀ g ∈ Gen, g.length ≤ ℓ)
    {u v : Ω} (hu : u ∈ Δ \ K) (gs : List (List L)) (hgs : ∀ g ∈ gs, g ∈ Gen)
    (hv : A.wordPerm gs.flatten u = v) :
    ∃ w : List L, (∀ p ∈ K, A.wordPerm w p = p) ∧ A.wordPerm w u = v ∧
      w.length ≤ ((Δ \ K).card - 1) * ℓ := by
  obtain ⟨gs', hgs', hlen, hend⟩ := exists_short_walk (fun g y => A.wordPerm g y) (Gen := Gen)
    (Y := Δ \ K) (fun g hg y hy => hΔ.wordPerm_mem_sdiff (hGenK g hg) hy) hu gs hgs
  refine ⟨gs'.flatten, fun p hp => A.wordPerm_flatten_apply_of_forall
    fun g hg => hGenK g (hgs' g hg) p hp, ?_, ?_⟩
  · rw [wordPerm_flatten_apply, hend, ← wordPerm_flatten_apply, hv]
  · calc gs'.flatten.length ≤ gs'.length * ℓ :=
          length_flatten_le gs' ℓ fun g hg => hGenℓ g (hgs' g hg)
      _ ≤ ((Δ \ K).card - 1) * ℓ := Nat.mul_le_mul_right ℓ (by omega)

/-- H1 of R22B-05 gives transitivity on `Δ \ K` by words that fix `K` pointwise, when `Δ` has at
least three chambers outside `K`: a three-cycle `(u v y)` with `y ∉ K ∪ {u, v}` is even. -/
theorem exists_word_of_alt {Δ K : Finset Ω} (hK : K ⊆ Δ) (hcard : K.card + 3 ≤ Δ.card)
    (H1 : ∀ σ : Perm Δ, σ ∈ alternatingGroup Δ → ∃ g ∈ A.group, ∀ x : Δ, g x = σ x)
    {u v : Ω} (hu : u ∈ Δ \ K) (hv : v ∈ Δ \ K) :
    ∃ w : List L, (∀ p ∈ K, A.wordPerm w p = p) ∧ A.wordPerm w u = v := by
  rw [Finset.mem_sdiff] at hu hv
  by_cases huv : u = v
  · exact ⟨[], fun p _ => rfl, huv⟩
  have hlt : (insert u (insert v K)).card < Δ.card := by
    have h1 := Finset.card_insert_le u (insert v K)
    have h2 := Finset.card_insert_le v K
    omega
  obtain ⟨y, hy, hyn⟩ := Finset.exists_mem_notMem_of_card_lt_card hlt
  simp only [Finset.mem_insert, not_or] at hyn
  obtain ⟨hyu, hyv, hyK⟩ := hyn
  have hne : ∀ {p q : Ω} (hp : p ∈ Δ) (hq : q ∈ Δ), p ≠ q →
      (⟨p, hp⟩ : Δ) ≠ ⟨q, hq⟩ := fun _ _ h e => h (congrArg Subtype.val e)
  let σ : Perm Δ := cycle3 ⟨u, hu.1⟩ ⟨v, hv.1⟩ ⟨y, hy⟩
  have hσ : σ ∈ alternatingGroup Δ :=
    cycle3_mem_alternatingGroup (hne _ _ huv) (hne _ _ (Ne.symm hyv))
  obtain ⟨g, hg, hgσ⟩ := H1 σ hσ
  obtain ⟨w, rfl⟩ := (A.mem_group_iff).1 hg
  refine ⟨w, fun p hp => ?_, ?_⟩
  · have hpu : p ≠ u := fun h => hu.2 (h ▸ hp)
    have hpv : p ≠ v := fun h => hv.2 (h ▸ hp)
    have hpy : p ≠ y := fun h => hyK (h ▸ hp)
    have := hgσ ⟨p, hK hp⟩
    rw [this, cycle3_apply_of_ne (hne _ _ hpu) (hne _ _ hpv) (hne _ _ hpy)]
  · have := hgσ ⟨u, hu.1⟩
    rw [this, cycle3_apply_left (hne _ _ huv) (hne _ _ (Ne.symm hyu))]

omit [DecidableEq Ω] in
/-- R22B-05, step 1: a breadth-first tree on an orbit `Δ` from `a` gives words `R_x` with
`a·R_x = x`, `R_a` empty and `|R_x| ≤ d - 1`. -/
theorem exists_tree_words {Δ : Finset Ω} (hΔ : A.IsOrbit Δ) {a : Ω} (ha : a ∈ Δ) :
    ∃ R : Ω → List L, R a = [] ∧ ∀ x ∈ Δ, A.wordPerm (R x) a = x ∧ (R x).length + 1 ≤ Δ.card := by
  have key : ∀ x, ∃ r : List L, (x = a → r = []) ∧
      (x ∈ Δ → A.wordPerm r a = x ∧ r.length + 1 ≤ Δ.card) := by
    intro x
    by_cases hxa : x = a
    · subst hxa
      exact ⟨[], fun _ => rfl, fun _ => ⟨rfl, Finset.card_pos.2 ⟨x, ha⟩⟩⟩
    by_cases hx : x ∈ Δ
    · obtain ⟨w, hw⟩ := hΔ.connected a ha x hx
      obtain ⟨w', -, hlen, hend⟩ := exists_short_walk (fun l y => A.act l y) (Gen := Set.univ)
        (Y := Δ) (fun l _ y hy => hΔ.invariant l y hy) ha w (fun _ _ => Set.mem_univ _)
      refine ⟨w', fun h => absurd h hxa, fun _ => ⟨?_, hlen⟩⟩
      rw [wordPerm_apply_eq_walk, hend, ← wordPerm_apply_eq_walk, hw]
    · exact ⟨[], fun _ => rfl, fun h => absurd h hx⟩
  choose R hR using key
  exact ⟨R, (hR a).1 rfl, fun x hx => (hR x).2 hx⟩

theorem flatten_map_singleton (w : List L) : (w.map fun l => [l]).flatten = w := by
  induction w with
  | nil => rfl
  | cons l w ih => simp [ih]

/-- R22B-05: tables of Schreier words always exist. Under H1 (the group induces on the orbit `Δ` a
group that contains `Alt(Δ)`), with `d = |Δ| ≥ 5` and distinct anchors `a, b, c ∈ Δ`: for every
`x ∈ Δ \ {a}` a word `P_x` of at most `(d - 2)(2d - 1)` letters fixes `a` and sends `x` to `b`, and
for every `z ∈ Δ \ {a, b}` a word `Q_z` of at most `(d - 3)(2d - 3)(2d - 1)` letters fixes `a` and
`b` and sends `c` to `z`. These are H3 and H4 of R22B-01. -/
theorem schreierTables {Δ : Finset Ω} (hΔ : A.IsOrbit Δ) (hd : 5 ≤ Δ.card) {a b c : Ω}
    (ha : a ∈ Δ) (hb : b ∈ Δ) (hc : c ∈ Δ) (hab : a ≠ b) (hac : a ≠ c) (hbc : b ≠ c)
    (H1 : ∀ σ : Perm Δ, σ ∈ alternatingGroup Δ → ∃ g ∈ A.group, ∀ x : Δ, g x = σ x) :
    (∀ x ∈ Δ, x ≠ a → ∃ P : List L, A.wordPerm P a = a ∧ A.wordPerm P x = b ∧
        P.length ≤ (Δ.card - 2) * (2 * Δ.card - 1)) ∧
    (∀ z ∈ Δ, z ≠ a → z ≠ b → ∃ Q : List L, A.wordPerm Q a = a ∧ A.wordPerm Q b = b ∧
        A.wordPerm Q c = z ∧ Q.length ≤ (Δ.card - 3) * (2 * Δ.card - 3) * (2 * Δ.card - 1)) := by
  set d := Δ.card with hd_def
  have hcardA : (Δ \ {a}).card = d - 1 := by
    rw [Finset.card_sdiff_of_subset (Finset.singleton_subset_iff.2 ha), Finset.card_singleton]
  have hcardAB : (Δ \ insert b {a}).card = d - 2 := by
    rw [Finset.card_sdiff_of_subset (Finset.insert_subset hb (Finset.singleton_subset_iff.2 ha)),
      Finset.card_pair (Ne.symm hab)]
  -- Step 1: the tree `R` from `a`.
  obtain ⟨R, hRa, hR⟩ := A.exists_tree_words hΔ ha
  -- Step 2: the Schreier generators of the stabilizer of `a`.
  let Gen₀ : Set (List L) := Set.range fun l : L => [l]
  have hsingle : ∀ w : List L, ∃ gs : List (List L), (∀ g ∈ gs, g ∈ Gen₀) ∧ gs.flatten = w :=
    fun w => ⟨w.map fun l => [l], by simp [Gen₀], flatten_map_singleton w⟩
  obtain ⟨hGen₁, hconv₁⟩ := A.schreier_level (F := ∅) (y₀ := a) (Gen := Gen₀) (ℓ := 1)
    (m := d - 1) hΔ.invariant (by simpa using ha) (by simp) (by rintro g ⟨l, rfl⟩; simp) R hRa
    (fun z hz => by
      have := hR z (Finset.mem_sdiff.1 hz).1
      exact ⟨this.1, by simp, by omega⟩)
  set Gen₁ := A.schreierGen Δ ∅ Gen₀ R
  have hGen₁a : ∀ g ∈ Gen₁, ∀ p ∈ ({a} : Finset Ω), A.wordPerm g p = p := by
    intro g hg p hp
    exact (hGen₁ g hg).1 p (by simpa using hp)
  have hGen₁ℓ : ∀ g ∈ Gen₁, g.length ≤ 2 * d - 1 := by
    intro g hg
    have := (hGen₁ g hg).2
    omega
  -- Every word that fixes `a` is a product of Schreier generators.
  have hconv₁' : ∀ w : List L, A.wordPerm w a = a →
      ∃ us : List (List L), (∀ u ∈ us, u ∈ Gen₁) ∧ A.wordPerm us.flatten = A.wordPerm w := by
    intro w hw
    obtain ⟨gs, hgs, rfl⟩ := hsingle w
    exact hconv₁ gs hgs hw
  -- Step 3: the tree `B` from `b` over the Schreier generators.
  have hBex : ∀ x, ∃ Bx : List L, (x = b → Bx = []) ∧ (x ∈ Δ \ {a} →
      A.wordPerm Bx b = x ∧ (∀ p ∈ ({a} : Finset Ω), A.wordPerm Bx p = p) ∧
        Bx.length ≤ (d - 2) * (2 * d - 1)) := by
    intro x
    by_cases hxb : x = b
    · subst hxb
      exact ⟨[], fun _ => rfl, fun _ => ⟨rfl, fun _ _ => rfl, Nat.zero_le _⟩⟩
    by_cases hx : x ∈ Δ \ {a}
    · have hbA : b ∈ Δ \ {a} := by simp [hb, Ne.symm hab]
      obtain ⟨w, hwa, hwx⟩ := A.exists_word_of_alt (K := {a}) (Finset.singleton_subset_iff.2 ha)
        (by rw [Finset.card_singleton]; omega) H1 hbA hx
      obtain ⟨us, hus, hperm⟩ := hconv₁' w (hwa a (Finset.mem_singleton_self a))
      obtain ⟨Bx, hBa, hBx, hlen⟩ := A.exists_short_chain hΔ.invariant hGen₁a hGen₁ℓ hbA us hus
        (by rw [hperm, hwx])
      refine ⟨Bx, fun h => absurd h hxb, fun _ => ⟨hBx, hBa, ?_⟩⟩
      rw [hcardA] at hlen
      calc Bx.length ≤ (d - 1 - 1) * (2 * d - 1) := hlen
        _ = (d - 2) * (2 * d - 1) := by congr 1
    · exact ⟨[], fun _ => rfl, fun h => absurd h hx⟩
  choose B hB using hBex
  have hBb : B b = [] := (hB b).1 rfl
  refine ⟨?_, ?_⟩
  · -- The table `P_x = B_x⁻¹`.
    intro x hx hxa
    have hxA : x ∈ Δ \ {a} := by simp [hx, hxa]
    obtain ⟨hBx, hBa, hlen⟩ := (hB x).2 hxA
    exact ⟨A.invWord (B x), A.wordPerm_invWord_apply_of_apply (hBa a (Finset.mem_singleton_self a)),
      A.wordPerm_invWord_apply_of_apply hBx, by rw [length_invWord]; exact hlen⟩
  · -- Step 4: the Schreier generators of the stabilizer of `a` and `b`, and the tree from `c`.
    intro z hz hza hzb
    have hbA : b ∈ Δ \ {a} := by simp [hb, Ne.symm hab]
    obtain ⟨hGen₂, hconv₂⟩ := A.schreier_level (F := {a}) (y₀ := b) (Gen := Gen₁)
      (ℓ := 2 * d - 1) (m := (d - 2) * (2 * d - 1)) hΔ.invariant hbA hGen₁a hGen₁ℓ B hBb
      (fun y hy => (hB y).2 hy)
    set Gen₂ := A.schreierGen Δ {a} Gen₁ B
    have hcAB : c ∈ Δ \ insert b {a} := by simp [hc, Ne.symm hac, Ne.symm hbc]
    by_cases hzc : z = c
    · subst hzc
      exact ⟨[], rfl, rfl, rfl, Nat.zero_le _⟩
    have hzAB : z ∈ Δ \ insert b {a} := by simp [hz, hza, hzb]
    obtain ⟨w, hwab, hwz⟩ := A.exists_word_of_alt (K := insert b {a})
      (Finset.insert_subset hb (Finset.singleton_subset_iff.2 ha))
      (by rw [Finset.card_pair (Ne.symm hab)]; omega) H1 hcAB hzAB
    have hwa : A.wordPerm w a = a := hwab a (by simp)
    have hwb : A.wordPerm w b = b := hwab b (by simp)
    obtain ⟨us, hus, hperm₁⟩ := hconv₁' w hwa
    obtain ⟨vs, hvs, hperm₂⟩ := hconv₂ us hus (by rw [hperm₁, hwb])
    obtain ⟨Q, hQab, hQc, hlen⟩ := A.exists_short_chain hΔ.invariant (K := insert b {a})
      (fun g hg p hp => (hGen₂ g hg).1 p hp) (fun g hg => (hGen₂ g hg).2) hcAB vs hvs
      (by rw [hperm₂, hperm₁, hwz])
    refine ⟨Q, hQab a (by simp), hQab b (by simp), hQc, ?_⟩
    rw [hcardAB] at hlen
    refine hlen.trans (le_of_eq ?_)
    obtain ⟨k, hk⟩ := Nat.exists_eq_add_of_le hd
    rw [hk, show 5 + k - 2 - 1 = k + 2 by omega, show 5 + k - 2 = k + 3 by omega,
      show 2 * (5 + k) - 1 = 2 * k + 9 by omega, show 5 + k - 3 = k + 2 by omega,
      show 2 * (5 + k) - 3 = 2 * k + 7 by omega]
    ring

end Alphabet

end Jumbling
