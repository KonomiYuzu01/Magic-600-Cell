import Jumbling.Group.LocalAlt
import Jumbling.Group.PowerSeed

set_option linter.style.header false in
/-!
# R22B-08: a seed from a Jordan word and a letter of small support

`PROOFS-B.md` section 3, R22B-08 (layer "group" of `ARCHON-HANDOFF.md`).

Setting: an alphabet `A` (the SAFE-ALL letters of O5) on the chambers `Ω` and an orbit `Δ` with `d`
chambers; the orbits before it are arbitrary.

* `IsJordanCertificate` is H1: a global Jordan certificate `(w, p)` for `Δ`, `|w| = ℓ`. H2 is a
  hypothesis of the statements: a word `h` of `q` letters whose moved support `R = movedOn h Δ` on
  `Δ` has `r` chambers, `2 ≤ r` and `2r - 1 ≤ d`.
* Step 1 (`jordanPower`): with `M = jordanExp w p`, the least common multiple of the cycle lengths
  of `π_w` other than the `p`-cycle, the word `A = w^M` of `ℓM` letters acts as one `p`-cycle
  inside `Δ` and fixes every other chamber.
* Step 2 (`exists_short_word_from`, `genAlphabet`, `alternatingGroup_le_genAlphabet`,
  `inducedGroup_genAlphabet_eq`): breadth-first search from `supp(A)` gives words `u_x` of at most
  `d - p` letters; the conjugates `u_x⁻¹·A·u_x` and their inverses, of at most `J = ℓM + 2(d - p)`
  letters, fix every chamber outside `Δ` and induce exactly `Alt(Δ)` on `Δ` (transitive, primitive
  by the long-prime-cycle block argument, `Alt(Δ)` by Jordan's theorem, and the generators are
  even). The remark that at most `d - p + 1` conjugates are needed is not formalized; all of them
  are used, which changes no bound.
* Steps 3 and 4 and the bound (`jordanSeed`): `R′` with `|R′| = r` and `R ∩ R′ = {z}`; a word `U` of
  at most `J·(C(d, r) - 1)` letters with `R·U = R′` that fixes every chamber outside `Δ`; and
  `C = h·h′·h⁻¹·h′⁻¹` with `h′ = U⁻¹·h·U` (`seedWord`) is the three-cycle `(z, z·h′⁻¹, z·h⁻¹)` on
  `Ω` (by R22B-03, `separatedOverlap`), with `|C| ≤ 4q + 4·[ℓM + 2(d - p)]·[C(d, r) - 1]`.
* `jordanSeed_isSeed`: `C` is a seed (`IsSeed` of R22B-01) for `Δ` relative to every set of chambers
  outside `Δ`, so for every order of the orbits.

Theorem A's steps that R22B-08 uses (the isolating power, the block argument and Jordan's theorem)
are proved in `Jumbling.Classical.JordanPrimeCycle`, not assumed.
-/

namespace Jumbling

namespace Alphabet

open Equiv Equiv.Perm Finset MulAction

variable {L Ω : Type*} (A : Alphabet L Ω)

/-! ### Repeated words and conjugates -/

theorem wordPerm_replicate_letter (n : ℕ) (l : L) :
    A.wordPerm (List.replicate n l) = A.act l ^ n := by
  induction n with
  | zero => rfl
  | succ n ih => rw [List.replicate_succ, wordPerm_cons, ih, pow_succ]

theorem wordPerm_replicate_flatten (n : ℕ) (w : List L) :
    A.wordPerm (List.replicate n w).flatten = A.wordPerm w ^ n := by
  induction n with
  | zero => rfl
  | succ n ih => rw [List.replicate_succ, List.flatten_cons, wordPerm_append, ih, pow_succ]

/-- The chronological conjugate `u⁻¹·a·u`: `u⁻¹`, then `a`, then `u`. -/
def conjWord (a u : List L) : List L :=
  A.invWord u ++ a ++ u

theorem wordPerm_conjWord (a u : List L) :
    A.wordPerm (A.conjWord a u) = A.wordPerm u * A.wordPerm a * (A.wordPerm u)⁻¹ := by
  simp [conjWord, mul_assoc]

theorem length_conjWord (a u : List L) :
    (A.conjWord a u).length = a.length + 2 * u.length := by
  simp only [conjWord, List.length_append, length_invWord]
  ring

/-- The seed word of R22B-08, step 4: `C = h·h′·h⁻¹·h′⁻¹` with `h′ = U⁻¹·h·U`. -/
def seedWord (h U : List L) : List L :=
  h ++ A.conjWord h U ++ A.invWord h ++ A.invWord (A.conjWord h U)

theorem length_seedWord (h U : List L) :
    (A.seedWord h U).length = 4 * h.length + 4 * U.length := by
  simp only [seedWord, List.length_append, length_invWord, length_conjWord]
  ring

/-! ### The alphabet of conjugates -/

/-- The words of the letters of `genAlphabet`: `(x, true)` is the conjugate `u_x⁻¹·a·u_x` and
`(x, false)` its inverse. -/
def genWord (a : List L) (u : Ω → List L) : Ω × Bool → List L
  | (x, true) => A.conjWord a (u x)
  | (x, false) => A.invWord (A.conjWord a (u x))

/-- R22B-08, step 2: the alphabet whose letters are the conjugates `u_x⁻¹·a·u_x` and their
inverses. -/
def genAlphabet (a : List L) (u : Ω → List L) : Alphabet (Ω × Bool) Ω where
  act l := A.wordPerm (A.genWord a u l)
  inv l := (l.1, !l.2)
  act_inv := by
    rintro ⟨x, _ | _⟩ <;> simp [genWord]

theorem genAlphabet_wordPerm (a : List L) (u : Ω → List L) (v : List (Ω × Bool)) :
    (A.genAlphabet a u).wordPerm v = A.wordPerm (v.map (A.genWord a u)).flatten := by
  induction v with
  | nil => rfl
  | cons l v ih =>
    rw [wordPerm_cons, ih, List.map_cons, List.flatten_cons, wordPerm_append]
    rfl

theorem length_genWord_le (a : List L) (u : Ω → List L) {J : ℕ}
    (hJ : ∀ x, a.length + 2 * (u x).length ≤ J) (l : Ω × Bool) : (A.genWord a u l).length ≤ J := by
  obtain ⟨x, _ | _⟩ := l <;> simp only [genWord, length_invWord, length_conjWord] <;> exact hJ x

/-! ### Step 2: breadth-first search from a set -/

variable [DecidableEq Ω]

/-- R22B-08, step 2: breadth-first search in the single-letter graph of an orbit `Δ` from a nonempty
set `S ⊆ Δ`. For each `x ∈ Δ`, some word of at most `|Δ \ S|` letters carries a point of `S` to `x`:
a shortest path from the set visits at most the points outside it. -/
theorem exists_short_word_from {Δ S : Finset Ω} (hΔ : A.IsOrbit Δ) (hS : S ⊆ Δ)
    (hSne : S.Nonempty) {x : Ω} (hx : x ∈ Δ) :
    ∃ s ∈ S, ∃ u : List L, A.wordPerm u s = x ∧ u.length ≤ (Δ \ S).card := by
  by_cases hxS : x ∈ S
  · exact ⟨x, hxS, [], rfl, Nat.zero_le _⟩
  obtain ⟨s₀, hs₀⟩ := hSne
  -- The single-letter graph with `S` collapsed to `s₀`, which it never leaves.
  let mv : L → Ω → Ω := fun l y => if y ∈ S then s₀ else if A.act l y ∈ S then s₀ else A.act l y
  have hstay : ∀ ws, walk mv s₀ ws = s₀ := by
    intro ws
    induction ws with
    | nil => rfl
    | cons l ws ih =>
      rw [walk_cons]
      simp only [mv, hs₀, ↓reduceIte]
      exact ih
  -- A walk that meets `S` ends at `s₀` in the collapsed graph.
  have hhit : ∀ ws y, y ∉ S → (∃ k, walk (fun l y => A.act l y) y (ws.take k) ∈ S) →
      walk mv y ws = s₀ := by
    intro ws
    induction ws with
    | nil =>
      intro y hy ⟨k, hk⟩
      rw [List.take_nil, walk_nil] at hk
      exact absurd hk hy
    | cons l ws ih =>
      intro y hy ⟨k, hk⟩
      rcases k with _ | k
      · rw [List.take_zero, walk_nil] at hk
        exact absurd hk hy
      rw [List.take_succ_cons, walk_cons] at hk
      rw [walk_cons]
      by_cases hl : A.act l y ∈ S
      · simp only [mv, hy, hl, ↓reduceIte]
        exact hstay ws
      · simp only [mv, hy, hl, ↓reduceIte]
        exact ih _ hl ⟨k, hk⟩
  -- A walk that ends at `s₀` in the collapsed graph meets `S` within its length.
  have hback : ∀ ws y, walk mv y ws = s₀ →
      ∃ k ≤ ws.length, walk (fun l y => A.act l y) y (ws.take k) ∈ S := by
    intro ws
    induction ws with
    | nil =>
      intro y hy
      rw [walk_nil] at hy
      exact ⟨0, le_rfl, by rw [List.take_zero, walk_nil, hy]; exact hs₀⟩
    | cons l ws ih =>
      intro y hy
      by_cases hyS : y ∈ S
      · exact ⟨0, Nat.zero_le _, by rw [List.take_zero, walk_nil]; exact hyS⟩
      by_cases hl : A.act l y ∈ S
      · refine ⟨1, by simp, ?_⟩
        rw [List.take_succ_cons, List.take_zero, walk_cons, walk_nil]
        exact hl
      rw [walk_cons] at hy
      simp only [mv, hyS, hl, ↓reduceIte] at hy
      obtain ⟨k, hk, hkS⟩ := ih _ hy
      refine ⟨k + 1, by rw [List.length_cons]; omega, ?_⟩
      rw [List.take_succ_cons, walk_cons]
      exact hkS
  -- The collapsed graph keeps `Y = {s₀} ∪ (Δ \ S)`.
  set Y := insert s₀ (Δ \ S) with hYdef
  have hY : ∀ l ∈ (Set.univ : Set L), ∀ y ∈ Y, mv l y ∈ Y := by
    intro l _ y hy
    by_cases hyS : y ∈ S
    · simp only [mv, hyS, ↓reduceIte]
      exact mem_insert_self _ _
    have hyΔ : y ∈ Δ := by
      rcases mem_insert.1 hy with rfl | h
      · exact absurd hs₀ hyS
      · exact (mem_sdiff.1 h).1
    by_cases hl : A.act l y ∈ S
    · simp only [mv, hyS, hl, ↓reduceIte]
      exact mem_insert_self _ _
    · simp only [mv, hyS, hl, ↓reduceIte]
      exact mem_insert_of_mem (mem_sdiff.2 ⟨hΔ.invariant l y hyΔ, hl⟩)
  have hxY : x ∈ Y := mem_insert_of_mem (mem_sdiff.2 ⟨hx, hxS⟩)
  have hYcard : Y.card = (Δ \ S).card + 1 :=
    card_insert_of_notMem fun h => (mem_sdiff.1 h).2 hs₀
  -- Some word carries `x` into `S`; shorten it in the collapsed graph.
  obtain ⟨w₀, hw₀⟩ := hΔ.connected x hx s₀ (hS hs₀)
  have hend : walk mv x w₀ = s₀ := hhit w₀ x hxS ⟨w₀.length, by
    rw [List.take_length, ← wordPerm_apply_eq_walk, hw₀]
    exact hs₀⟩
  obtain ⟨ws, -, hlen, hws⟩ :=
    exists_short_walk mv (Gen := Set.univ) hY hxY w₀ (fun _ _ => Set.mem_univ _)
  rw [hend] at hws
  obtain ⟨k, hk, hkS⟩ := hback ws x hws
  rw [← wordPerm_apply_eq_walk] at hkS
  refine ⟨_, hkS, A.invWord (ws.take k), wordPerm_invWord_apply_self _ _ _, ?_⟩
  rw [length_invWord, List.length_take]
  omega

/-! ### H1 and step 1 -/

variable [Fintype Ω]

/-- H1 of R22B-08: a global Jordan certificate `(w, p)` for `Δ` (`buffer-transport.md` section 3,
checked by `buffer.check_jordan` on all of `Ω`): `p` is a prime with `d/2 < p ≤ d - 3`, and `π_w`
has exactly one cycle of length divisible by `p`, a `p`-cycle inside `Δ`. -/
structure IsJordanCertificate (Δ : Finset Ω) (w : List L) (p : ℕ) : Prop where
  prime : p.Prime
  long : Δ.card < 2 * p
  short : p + 3 ≤ Δ.card
  cycle : ∃ c ∈ (A.wordPerm w).cycleFactorsFinset, c.support.card = p ∧ c.support ⊆ Δ ∧
    ∀ c' ∈ (A.wordPerm w).cycleFactorsFinset, p ∣ c'.support.card → c' = c

/-- The exponent `M` of R22B-08, step 1: the least common multiple of the cycle lengths of `π_w`
other than the `p`-cycle. -/
def jordanExp (w : List L) (p : ℕ) : ℕ :=
  ((A.wordPerm w).cycleType.erase p).lcm

/-- The moved support `R` on `Δ` of a word `h` (H2 of R22B-08). -/
def movedOn (h : List L) (Δ : Finset Ω) : Finset Ω :=
  (A.wordPerm h).support ∩ Δ

/-- R22B-08, step 1: with `M = jordanExp w p`, `p` does not divide `M`, and `A = w^M`, a word of
`ℓM` letters, acts as one `p`-cycle inside `Δ` and fixes every other chamber. -/
theorem jordanPower {Δ : Finset Ω} {w : List L} {p : ℕ} (hJ : A.IsJordanCertificate Δ w p) :
    (A.wordPerm (List.replicate (A.jordanExp w p) w).flatten).IsCycle ∧
      (A.wordPerm (List.replicate (A.jordanExp w p) w).flatten).support.card = p ∧
      (A.wordPerm (List.replicate (A.jordanExp w p) w).flatten).support ⊆ Δ ∧
      (List.replicate (A.jordanExp w p) w).flatten.length = w.length * A.jordanExp w p := by
  obtain ⟨c, hc, hcp, hcΔ, huniq⟩ := hJ.cycle
  have hother : ∀ c' ∈ (A.wordPerm w).cycleFactorsFinset, c' ≠ c →
      ¬c.support.card ∣ c'.support.card :=
    fun c' hc' hne hdvd => hne (huniq c' hc' (hcp ▸ hdvd))
  obtain ⟨hpow, hcyc, hsupp⟩ :=
    Classical.pow_orderOf_mul_inv hc (by rw [hcp]; exact hJ.prime) hother
  have hM : orderOf (A.wordPerm w * c⁻¹) = A.jordanExp w p := by
    rw [Classical.orderOf_mul_inv_eq_lcm hc, hcp, jordanExp]
  rw [hM] at hpow hcyc hsupp
  rw [wordPerm_replicate_flatten, hpow]
  exact ⟨hcyc, by rw [hsupp, hcp], hsupp ▸ hcΔ, length_replicate_flatten w _⟩

/-! ### Step 2: the conjugates generate `Alt(Δ)` -/

/-- A permutation whose support lies in `Δ` maps `Δ` into itself. -/
theorem apply_mem_of_support_subset {f : Perm Ω} {Δ : Finset Ω}
    (hf : f.support ⊆ Δ) {y : Ω} (hy : y ∈ Δ) : f y ∈ Δ := by
  by_cases h : f y = y
  · rwa [h]
  · exact hf (Perm.apply_mem_support.2 (Perm.mem_support.2 h))

/-- A cycle whose support lies in `Δ` restricts to a cycle of `Δ` with as many points. -/
theorem isCycle_subtypePerm {f : Perm Ω} (hf : f.IsCycle) {Δ : Finset Ω}
    (h : ∀ x, f x ∈ Δ ↔ x ∈ Δ) (hsub : f.support ⊆ Δ) :
    (f.subtypePerm h : Perm Δ).IsCycle ∧
      (f.subtypePerm h : Perm Δ).support.card = f.support.card := by
  refine ⟨?_, ?_⟩
  · obtain ⟨x₀, hx₀, hcyc⟩ := hf
    have hx₀Δ : x₀ ∈ Δ := hsub (Perm.mem_support.2 hx₀)
    refine ⟨⟨x₀, hx₀Δ⟩, fun h' => hx₀ (congrArg Subtype.val h'), fun y hy => ?_⟩
    rw [sameCycle_subtypePerm]
    exact hcyc fun h' => hy (Subtype.ext h')
  · have hmap : (f.subtypePerm h : Perm Δ).support.map (Function.Embedding.subtype _) =
        f.support := by
      ext y
      simp only [mem_map, Perm.mem_support, Function.Embedding.subtype_apply]
      constructor
      · rintro ⟨x, hx, rfl⟩
        exact fun h' => hx (Subtype.ext h')
      · intro hy
        exact ⟨⟨y, hsub (Perm.mem_support.2 hy)⟩, fun h' => hy (congrArg Subtype.val h'), rfl⟩
    rw [← hmap, card_map]

/-- R22B-08, step 2. Let the word `a` act as one `p`-cycle with support `S ⊆ Δ`, `d/2 < p ≤ d - 3`,
and let `u_x` carry a point of `S` to `x` for every `x ∈ Δ`. The letters of `genAlphabet a u` (the
conjugates `u_x⁻¹·a·u_x` and their inverses) fix every chamber outside `Δ`; they are `p`-cycles
inside `Δ` whose supports cover `Δ` and meet pairwise (`2p > d`), so they make `Δ` an orbit, and the
group they induce on `Δ` contains `Alt(Δ)`: it is primitive by the long-prime-cycle block argument
and contains `Alt(Δ)` by Jordan's theorem. -/
theorem alternatingGroup_le_genAlphabet {Δ : Finset Ω} (hΔ : A.IsInvariant Δ) {a : List L}
    {p : ℕ} (hcyc : (A.wordPerm a).IsCycle) (hcard : (A.wordPerm a).support.card = p)
    (hsub : (A.wordPerm a).support ⊆ Δ) (hp : p.Prime) (hlong : Δ.card < 2 * p)
    (hshort : p + 3 ≤ Δ.card) {u : Ω → List L}
    (hu : ∀ x ∈ Δ, ∃ s ∈ (A.wordPerm a).support, A.wordPerm (u x) s = x) :
    (∀ l y, y ∉ Δ → (A.genAlphabet a u).act l y = y) ∧
      ∃ hB : (A.genAlphabet a u).IsOrbit Δ,
        alternatingGroup Δ ≤ (A.genAlphabet a u).inducedGroup hB.invariant := by
  set B := A.genAlphabet a u with hBdef
  let g : Ω → Perm Ω := fun x => A.wordPerm (A.conjWord a (u x))
  have hg : ∀ x, g x = A.wordPerm (u x) * A.wordPerm a * (A.wordPerm (u x))⁻¹ :=
    fun x => A.wordPerm_conjWord a (u x)
  have hgcyc : ∀ x, (g x).IsCycle := fun x => by rw [hg]; exact hcyc.conj
  have hgcard : ∀ x, (g x).support.card = p := fun x => by rw [hg, card_support_conj, hcard]
  have hgsub : ∀ x, (g x).support ⊆ Δ := by
    intro x y hy
    rw [hg, support_conj, mem_map] at hy
    obtain ⟨s, hs, rfl⟩ := hy
    exact hΔ.wordPerm_mem _ (hsub hs)
  have hgx : ∀ x ∈ Δ, x ∈ (g x).support := by
    intro x hx
    obtain ⟨s, hs, hsx⟩ := hu x hx
    rw [hg, support_conj, mem_map]
    exact ⟨s, hs, hsx⟩
  have hact : ∀ x, B.act (x, true) = g x ∧ B.act (x, false) = (g x)⁻¹ := fun x =>
    ⟨rfl, by simp [hBdef, genAlphabet, genWord, g]⟩
  have hsupp : ∀ l, (B.act l).support ⊆ Δ := by
    rintro ⟨x, _ | _⟩
    · rw [(hact x).2, support_inv]
      exact hgsub x
    · rw [(hact x).1]
      exact hgsub x
  have hfix : ∀ l y, y ∉ Δ → B.act l y = y := fun l y hy =>
    Perm.notMem_support.1 fun h => hy (hsupp l h)
  have hinv : B.IsInvariant Δ := fun l y hy => apply_mem_of_support_subset (hsupp l) hy
  have hΔne : Δ.Nonempty := card_pos.1 (by omega)
  -- The supports of two conjugates meet, since `2p > d`.
  have hmeet : ∀ x y, ∃ z, z ∈ (g x).support ∧ z ∈ (g y).support := by
    intro x y
    have hnd : ¬_root_.Disjoint (g x).support (g y).support := by
      intro hdisj
      have := card_le_card (union_subset (hgsub x) (hgsub y))
      rw [card_union_of_disjoint hdisj, hgcard, hgcard] at this
      omega
    obtain ⟨z, hz, hz'⟩ := not_disjoint_iff.1 hnd
    exact ⟨z, hz, hz'⟩
  have hconn : ∀ x ∈ Δ, ∀ y ∈ Δ, ∃ v, B.wordPerm v x = y := by
    intro x hx y hy
    obtain ⟨z, hzx, hzy⟩ := hmeet x y
    obtain ⟨i, hi⟩ := (hgcyc x).exists_pow_eq (Perm.mem_support.1 (hgx x hx))
      (Perm.mem_support.1 hzx)
    obtain ⟨j, hj⟩ := (hgcyc y).exists_pow_eq (Perm.mem_support.1 hzy)
      (Perm.mem_support.1 (hgx y hy))
    refine ⟨List.replicate i (x, true) ++ List.replicate j (y, true), ?_⟩
    rw [wordPerm_append_apply, wordPerm_replicate_letter, wordPerm_replicate_letter, (hact x).1,
      (hact y).1, hi, hj]
  have hB : B.IsOrbit Δ := ⟨hΔne, hinv, hconn⟩
  refine ⟨hfix, hB, ?_⟩
  -- A `p`-cycle in the induced group.
  obtain ⟨x₀, hx₀⟩ := hΔne
  have hf : B.wordPerm [(x₀, true)] = g x₀ := by rw [wordPerm_singleton]; rfl
  have hres := isCycle_subtypePerm (f := B.wordPerm [(x₀, true)]) (by rw [hf]; exact hgcyc x₀)
    (fun _ => hinv.wordPerm_mem_iff [(x₀, true)]) (by rw [hf]; exact hgsub x₀)
  have hcardσ : (B.inducedPerm hB.invariant [(x₀, true)]).support.card = p := by
    rw [← hgcard x₀, ← hf]
    exact hres.2
  refine B.alternatingGroup_le_inducedGroup hB (σ := B.inducedPerm hB.invariant [(x₀, true)])
    ⟨_, rfl⟩ hres.1 ?_ ?_ ?_ <;> rw [hcardσ]
  · exact hp
  · exact hlong
  · exact hshort

/-- Every letter of `genAlphabet a u` acts as a `p`-cycle inside `Δ` when `a` does. -/
theorem genAlphabet_act {Δ : Finset Ω} (hΔ : A.IsInvariant Δ) {a : List L} {p : ℕ}
    (hcyc : (A.wordPerm a).IsCycle) (hcard : (A.wordPerm a).support.card = p)
    (hsub : (A.wordPerm a).support ⊆ Δ) (u : Ω → List L) (l : Ω × Bool) :
    ((A.genAlphabet a u).act l).IsCycle ∧ ((A.genAlphabet a u).act l).support.card = p ∧
      ((A.genAlphabet a u).act l).support ⊆ Δ := by
  have hconj : ∀ x, (A.wordPerm (A.conjWord a (u x))).IsCycle ∧
      (A.wordPerm (A.conjWord a (u x))).support.card = p ∧
      (A.wordPerm (A.conjWord a (u x))).support ⊆ Δ := by
    intro x
    rw [wordPerm_conjWord]
    refine ⟨hcyc.conj, by rw [card_support_conj, hcard], fun y hy => ?_⟩
    rw [support_conj, mem_map] at hy
    obtain ⟨s, hs, rfl⟩ := hy
    exact hΔ.wordPerm_mem _ (hsub hs)
  obtain ⟨x, _ | _⟩ := l
  · change (A.wordPerm (A.invWord (A.conjWord a (u x)))).IsCycle ∧
      (A.wordPerm (A.invWord (A.conjWord a (u x)))).support.card = p ∧
      (A.wordPerm (A.invWord (A.conjWord a (u x)))).support ⊆ Δ
    rw [wordPerm_invWord, support_inv]
    exact ⟨(hconj x).1.inv, (hconj x).2⟩
  · exact hconj x

/-- R22B-08, step 2, end: the conjugates are `p`-cycles with `p` odd, so they are even, and the
group they induce on `Δ` is exactly `Alt(Δ)`. -/
theorem inducedGroup_genAlphabet_eq {Δ : Finset Ω} (hΔ : A.IsInvariant Δ) {a : List L}
    {p : ℕ} (hcyc : (A.wordPerm a).IsCycle) (hcard : (A.wordPerm a).support.card = p)
    (hsub : (A.wordPerm a).support ⊆ Δ) (hp : p.Prime) (hlong : Δ.card < 2 * p)
    (hshort : p + 3 ≤ Δ.card) {u : Ω → List L}
    (hu : ∀ x ∈ Δ, ∃ s ∈ (A.wordPerm a).support, A.wordPerm (u x) s = x) :
    ∃ hB : (A.genAlphabet a u).IsOrbit Δ,
      (A.genAlphabet a u).inducedGroup hB.invariant = alternatingGroup Δ := by
  obtain ⟨-, hB, hAlt⟩ :=
    A.alternatingGroup_le_genAlphabet hΔ hcyc hcard hsub hp hlong hshort hu
  refine ⟨hB, le_antisymm ?_ hAlt⟩
  set B := A.genAlphabet a u with hBdef
  have hodd : Odd p := hp.odd_of_ne_two (by omega)
  have hletter : ∀ l, B.inducedPerm hB.invariant [l] ∈ alternatingGroup Δ := by
    intro l
    obtain ⟨hlc, hlcard, hlsub⟩ := A.genAlphabet_act hΔ hcyc hcard hsub u l
    have hf : B.wordPerm [l] = B.act l := wordPerm_singleton B l
    have hres := isCycle_subtypePerm (f := B.wordPerm [l]) (by rw [hf]; exact hlc)
      (fun _ => hB.invariant.wordPerm_mem_iff [l]) (by rw [hf]; exact hlsub)
    rw [mem_alternatingGroup]
    change sign ((B.wordPerm [l]).subtypePerm _) = 1
    rw [hres.1.sign, hres.2, hf, hlcard, hodd.neg_one_pow, neg_neg]
  rintro σ ⟨v, rfl⟩
  induction v with
  | nil =>
    have : B.inducedPerm hB.invariant [] = 1 := by
      ext
      rfl
    rw [this]
    exact one_mem _
  | cons l v ih =>
    rw [show l :: v = [l] ++ v from rfl, inducedPerm_append]
    exact mul_mem ih (hletter l)

/-! ### Steps 3 and 4 -/

/-- A permutation `f` that maps `Δ` onto itself, restricted to `Δ` and extended by the identity. -/
def restrictTo (Δ : Finset Ω) (f : Perm Ω) (hf : ∀ x, f x ∈ Δ ↔ x ∈ Δ) : Perm Ω :=
  Perm.ofSubtype (f.subtypePerm hf)

omit [Fintype Ω] in
theorem restrictTo_apply_of_mem {Δ : Finset Ω} {f : Perm Ω} (hf : ∀ x, f x ∈ Δ ↔ x ∈ Δ) {x : Ω}
    (hx : x ∈ Δ) : restrictTo Δ f hf x = f x := by
  rw [restrictTo, ofSubtype_apply_of_mem _ hx, subtypePerm_apply]

omit [Fintype Ω] in
theorem restrictTo_apply_of_notMem {Δ : Finset Ω} {f : Perm Ω} (hf : ∀ x, f x ∈ Δ ↔ x ∈ Δ)
    {x : Ω} (hx : x ∉ Δ) : restrictTo Δ f hf x = x :=
  ofSubtype_apply_of_not_mem _ hx

omit [Fintype Ω] in
theorem restrictTo_inv_apply_of_mem {Δ : Finset Ω} {f : Perm Ω} (hf : ∀ x, f x ∈ Δ ↔ x ∈ Δ)
    {x : Ω} (hx : x ∈ Δ) : (restrictTo Δ f hf)⁻¹ x = f⁻¹ x := by
  have hx' : f⁻¹ x ∈ Δ := (hf _).1 (by rwa [perm_apply_inv_self])
  rw [Perm.inv_eq_iff_eq, restrictTo_apply_of_mem hf hx', perm_apply_inv_self]

omit [Fintype Ω] in
theorem restrictTo_inv_apply_of_notMem {Δ : Finset Ω} {f : Perm Ω} (hf : ∀ x, f x ∈ Δ ↔ x ∈ Δ)
    {x : Ω} (hx : x ∉ Δ) : (restrictTo Δ f hf)⁻¹ x = x := by
  rw [Perm.inv_eq_iff_eq, restrictTo_apply_of_notMem hf hx]

theorem mem_support_restrictTo {Δ : Finset Ω} {f : Perm Ω} (hf : ∀ x, f x ∈ Δ ↔ x ∈ Δ) {x : Ω} :
    x ∈ (restrictTo Δ f hf).support ↔ x ∈ f.support ∩ Δ := by
  rw [Perm.mem_support, mem_inter, Perm.mem_support]
  by_cases hx : x ∈ Δ
  · rw [restrictTo_apply_of_mem hf hx]
    tauto
  · rw [restrictTo_apply_of_notMem hf hx]
    tauto

/-- **R22B-08**, steps 3 and 4 and the bound. Let `(w, p)` be a global Jordan certificate for the
orbit `Δ` (H1, `|w| = ℓ`, `M = jordanExp w p`) and `h` a word of `q` letters whose moved support
`R = movedOn h Δ` on `Δ` has `r` chambers, `2 ≤ r` and `2r - 1 ≤ d` (H2); `h` may move the earlier
orbits. Then there are a word `U` of at most `J·(C(d, r) - 1)` letters, `J = ℓM + 2(d - p)`, that
fixes every chamber outside `Δ`, and a chamber `z ∈ R` with `R·U ∩ R = {z}` (step 3), such that
`C = h·h′·h⁻¹·h′⁻¹` with `h′ = U⁻¹·h·U` (`seedWord h U`) acts on all of `Ω` as the three-cycle
`(z, z·h′⁻¹, z·h⁻¹)` of three distinct chambers of `Δ` (step 4, by R22B-03), and
`|C| ≤ 4q + 4·[ℓM + 2(d - p)]·[C(d, r) - 1]`. -/
theorem jordanSeed {Δ : Finset Ω} (hΔ : A.IsOrbit Δ) {w : List L} {p : ℕ}
    (hJ : A.IsJordanCertificate Δ w p) {h : List L} (hr2 : 2 ≤ (A.movedOn h Δ).card)
    (hr : 2 * (A.movedOn h Δ).card ≤ Δ.card + 1) :
    ∃ U : List L, (∀ x ∉ Δ, A.wordPerm U x = x) ∧
      U.length ≤ (w.length * A.jordanExp w p + 2 * (Δ.card - p)) *
        (Δ.card.choose (A.movedOn h Δ).card - 1) ∧
      ∃ z ∈ A.movedOn h Δ, (A.movedOn h Δ).image (A.wordPerm U) ∩ A.movedOn h Δ = {z} ∧
        A.wordPerm (A.seedWord h U) =
          cycle3 z ((A.wordPerm (A.conjWord h U))⁻¹ z) ((A.wordPerm h)⁻¹ z) ∧
        (A.wordPerm (A.conjWord h U))⁻¹ z ∈ Δ ∧ (A.wordPerm h)⁻¹ z ∈ Δ ∧
        z ≠ (A.wordPerm (A.conjWord h U))⁻¹ z ∧ z ≠ (A.wordPerm h)⁻¹ z ∧
        (A.wordPerm (A.conjWord h U))⁻¹ z ≠ (A.wordPerm h)⁻¹ z ∧
        (A.seedWord h U).length ≤ 4 * h.length +
          4 * ((w.length * A.jordanExp w p + 2 * (Δ.card - p)) *
            (Δ.card.choose (A.movedOn h Δ).card - 1)) := by
  set d := Δ.card with hd
  set R := A.movedOn h Δ with hRdef
  set r := R.card with hrdef
  set M := A.jordanExp w p with hM
  set J := w.length * M + 2 * (d - p) with hJdef
  have hd8 := eight_le_of_prime hJ.prime hJ.long hJ.short
  have hRΔ : R ⊆ Δ := inter_subset_right
  -- Step 1: `a = w^M` is one `p`-cycle inside `Δ`.
  obtain ⟨hacyc, hacard, hasub, halen⟩ := A.jordanPower hJ
  set a := (List.replicate M w).flatten with hadef
  -- Step 2: the words `u_x` and the alphabet of conjugates.
  have hu : ∀ x, ∃ u : List L, u.length ≤ d - p ∧
      (x ∈ Δ → ∃ s ∈ (A.wordPerm a).support, A.wordPerm u s = x) := by
    intro x
    by_cases hx : x ∈ Δ
    · obtain ⟨s, hs, u, hsu, hlen⟩ :=
        A.exists_short_word_from hΔ hasub hacyc.nonempty_support hx
      rw [card_sdiff_of_subset hasub, hacard] at hlen
      exact ⟨u, hlen, fun _ => ⟨s, hs, hsu⟩⟩
    · exact ⟨[], Nat.zero_le _, fun h => absurd h hx⟩
  choose u hulen hu using hu
  obtain ⟨hBfix, hB, hAlt⟩ := A.alternatingGroup_le_genAlphabet hΔ.invariant hacyc hacard hasub
    hJ.prime hJ.long hJ.short hu
  set B := A.genAlphabet a u with hBdef
  have halen' : a.length = w.length * M := halen
  have hglen : ∀ l, (A.genWord a u l).length ≤ J :=
    A.length_genWord_le a u fun x => by
      have := hulen x
      omega
  -- Step 3: `R′` with `|R′| = r` and `R′ ∩ R = {z}`.
  obtain ⟨z, hzR⟩ : R.Nonempty := card_pos.1 (by omega)
  have hzΔ : z ∈ Δ := hRΔ hzR
  obtain ⟨T', hT'sub, hT'card⟩ := exists_subset_card_eq (s := Δ \ R) (n := r - 1)
    (by rw [card_sdiff_of_subset hRΔ]; omega)
  have hzT' : z ∉ T' := fun hz => (mem_sdiff.1 (hT'sub hz)).2 hzR
  set R' := insert z T' with hR'def
  have hR'card : R'.card = r := by
    rw [card_insert_of_notMem hzT', hT'card]
    omega
  have hR'Δ : R' ⊆ Δ := insert_subset hzΔ (hT'sub.trans sdiff_subset)
  have hR'R : R' ∩ R = {z} := by
    ext y
    simp only [mem_inter, mem_insert, mem_singleton, R']
    constructor
    · rintro ⟨rfl | hy, hyR⟩
      · rfl
      · exact absurd hyR (mem_sdiff.1 (hT'sub hy)).2
    · rintro rfl
      exact ⟨Or.inl rfl, hzR⟩
  -- `Alt(Δ)` carries `R` onto `R′`, since it is `(d - 2)`-fold transitive and `r ≤ d - 2`.
  have hrle : r ≤ Nat.card Δ - 2 := by
    rw [Nat.card_eq_fintype_card, Fintype.card_coe]
    omega
  have := alternatingGroup.isMultiplyPretransitive Δ
  have : IsMultiplyPretransitive (alternatingGroup Δ) Δ r :=
    isMultiplyPretransitive_of_le (n := Nat.card Δ - 2) hrle (Nat.sub_le _ _)
  let eR : Fin r ↪ Δ := ⟨fun i => ⟨R.equivFin.symm i, hRΔ (R.equivFin.symm i).2⟩,
    fun i j hij => by
      simp only [Subtype.mk.injEq] at hij
      exact R.equivFin.symm.injective (Subtype.ext hij)⟩
  let eR' : Fin r ↪ Δ := ⟨fun i => ⟨(R'.equivFinOfCardEq hR'card).symm i,
      hR'Δ ((R'.equivFinOfCardEq hR'card).symm i).2⟩,
    fun i j hij => by
      simp only [Subtype.mk.injEq] at hij
      exact (R'.equivFinOfCardEq hR'card).symm.injective (Subtype.ext hij)⟩
  obtain ⟨σ, hσ⟩ := MulAction.exists_smul_eq (alternatingGroup Δ) eR eR'
  have hσR : ∀ x (hx : x ∈ R), ((σ : Perm Δ) ⟨x, hRΔ hx⟩ : Ω) ∈ R' := by
    intro x hx
    have h1 : (σ : Perm Δ) (eR (R.equivFin ⟨x, hx⟩)) = eR' (R.equivFin ⟨x, hx⟩) := by
      rw [← hσ]
      rfl
    have h2 : eR (R.equivFin ⟨x, hx⟩) = ⟨x, hRΔ hx⟩ := by
      apply Subtype.ext
      change ((R.equivFin.symm (R.equivFin ⟨x, hx⟩) : R) : Ω) = x
      rw [Equiv.symm_apply_apply]
    rw [h2] at h1
    rw [h1]
    exact ((R'.equivFinOfCardEq hR'card).symm _).2
  -- A word of conjugates that induces `σ` on `Δ`, shortened by a search over `r`-subsets.
  obtain ⟨v, hv⟩ := hAlt σ.2
  have hvx : ∀ x (hx : x ∈ Δ), B.wordPerm v x = ((σ : Perm Δ) ⟨x, hx⟩ : Ω) := fun x hx => by
    rw [← hv]
    rfl
  have hRv : R.image (B.wordPerm v) = R' := by
    refine eq_of_subset_of_card_le ?_ ?_
    · intro y hy
      obtain ⟨x, hx, rfl⟩ := mem_image.1 hy
      rw [hvx x (hRΔ hx)]
      exact hσR x hx
    · rw [hR'card, card_image_of_injective _ (B.wordPerm v).injective]
  obtain ⟨v', -, hv'len, hv'end⟩ := exists_short_walk (fun l (S : Finset Ω) => S.image (B.act l))
    (Gen := Set.univ) (Y := Δ.powersetCard r) (by
      intro l _ S hS
      rw [mem_powersetCard] at hS ⊢
      refine ⟨fun y hy => ?_, by rw [card_image_of_injective _ (B.act l).injective, hS.2]⟩
      obtain ⟨y', hy', rfl⟩ := mem_image.1 hy
      exact hB.invariant l y' (hS.1 hy'))
    (mem_powersetCard.2 ⟨hRΔ, rfl⟩) v (fun _ _ => Set.mem_univ _)
  rw [B.walk_image, B.walk_image, hRv] at hv'end
  rw [card_powersetCard] at hv'len
  have hv'len' : v'.length + 1 ≤ d.choose r := hv'len
  -- The word `U` with `R·U = R′`.
  set U := (v'.map (A.genWord a u)).flatten with hUdef
  have hUperm : A.wordPerm U = B.wordPerm v' := (A.genAlphabet_wordPerm a u v').symm
  have hUfix : ∀ x ∉ Δ, A.wordPerm U x = x := fun x hx => by
    rw [hUperm]
    exact B.wordPerm_apply_of_forall fun l _ => hBfix l x hx
  have hUlen : U.length ≤ J * (d.choose r - 1) := by
    have := length_flatten_le (v'.map (A.genWord a u)) J (by
      intro w' hw'
      obtain ⟨l, -, rfl⟩ := List.mem_map.1 hw'
      exact hglen l)
    rw [List.length_map] at this
    calc U.length ≤ v'.length * J := this
      _ ≤ (d.choose r - 1) * J := Nat.mul_le_mul_right _ (by omega)
      _ = J * (d.choose r - 1) := mul_comm _ _
  have hRU : R.image (A.wordPerm U) = R' := by
    rw [hUperm]
    exact hv'end
  -- Step 4: `h′ = U⁻¹·h·U` acts as `h` outside `Δ`.
  have hπh' : A.wordPerm (A.conjWord h U) = A.wordPerm U * A.wordPerm h * (A.wordPerm U)⁻¹ :=
    A.wordPerm_conjWord h U
  have hinvh : ∀ x, A.wordPerm h x ∈ Δ ↔ x ∈ Δ := fun x => hΔ.invariant.wordPerm_mem_iff h
  have hinvh' : ∀ x, A.wordPerm (A.conjWord h U) x ∈ Δ ↔ x ∈ Δ := fun x =>
    hΔ.invariant.wordPerm_mem_iff _
  have hUinv : ∀ x ∉ Δ, (A.wordPerm U)⁻¹ x = x := fun x hx => by
    rw [Perm.inv_eq_iff_eq, hUfix x hx]
  have hout : ∀ x ∉ Δ, A.wordPerm (A.conjWord h U) x = A.wordPerm h x := by
    intro x hx
    have hhx : A.wordPerm h x ∉ Δ := fun h' => hx ((hinvh x).1 h')
    rw [hπh', Perm.mul_apply, Perm.mul_apply, hUinv x hx, hUfix _ hhx]
  -- The restrictions of `h` and `h′` to `Δ`; their supports are `R` and `R′`.
  set a' := restrictTo Δ (A.wordPerm h) hinvh with ha'def
  set b' := restrictTo Δ (A.wordPerm (A.conjWord h U)) hinvh' with hb'def
  have ha'supp : a'.support = R := by
    ext x
    rw [mem_support_restrictTo]
    rfl
  have hb'supp : b'.support = R' := by
    ext x
    rw [mem_support_restrictTo, ← hRU, mem_image_perm, hRdef, movedOn, mem_inter, mem_inter,
      Perm.mem_support, Perm.mem_support]
    constructor
    · rintro ⟨hx1, hx2⟩
      refine ⟨fun heq => hx1 ?_, hΔ.invariant.inv_mem U hx2⟩
      rw [hπh', Perm.mul_apply, Perm.mul_apply, heq, perm_apply_inv_self]
    · rintro ⟨hx1, hx2⟩
      have hxΔ : x ∈ Δ := by
        have := hΔ.invariant.wordPerm_mem U hx2
        rwa [perm_apply_inv_self] at this
      refine ⟨fun heq => hx1 ?_, hxΔ⟩
      rw [hπh', Perm.mul_apply, Perm.mul_apply] at heq
      exact Perm.eq_inv_iff_eq.2 heq
  -- The supports meet exactly in `z`, and the overlap is separated (R22B-03).
  have hI : a'.support ∩ b'.support = {z} := by
    rw [ha'supp, hb'supp, inter_comm, hR'R]
  have ha'z : a' z ≠ z := Perm.mem_support.1 (ha'supp ▸ hzR)
  have hb'z : b' z ≠ z := Perm.mem_support.1 (hb'supp ▸ mem_insert_self z T')
  have hsepa : _root_.Disjoint ((a'.support ∩ b'.support).image a') (a'.support ∩ b'.support) := by
    rw [hI, image_singleton, disjoint_singleton]
    exact ha'z
  have hsepb : _root_.Disjoint ((a'.support ∩ b'.support).image b') (a'.support ∩ b'.support) := by
    rw [hI, image_singleton, disjoint_singleton]
    exact hb'z
  obtain ⟨⟨hd1, hd2, hd3⟩, -, -, hprod⟩ := separatedOverlap a' b' hsepa hsepb
  rw [hI] at hd1 hd2 hd3 hprod
  simp only [toList_singleton, List.map_cons, List.map_nil, List.prod_cons, List.prod_nil,
    mul_one, image_singleton, disjoint_singleton] at hprod hd1 hd2 hd3
  have ha'inv : a'⁻¹ z = (A.wordPerm h)⁻¹ z := restrictTo_inv_apply_of_mem hinvh hzΔ
  have hb'inv : b'⁻¹ z = (A.wordPerm (A.conjWord h U))⁻¹ z := restrictTo_inv_apply_of_mem hinvh' hzΔ
  rw [ha'inv, hb'inv] at hprod hd3
  rw [ha'inv] at hd1
  rw [hb'inv] at hd2
  -- `C` agrees with the commutator of the restrictions.
  have hagree : chronoComm (A.wordPerm h) (A.wordPerm (A.conjWord h U)) = chronoComm a' b' := by
    ext x
    rw [chronoComm_apply, chronoComm_apply]
    by_cases hx : x ∈ Δ
    · have h1 : A.wordPerm h x ∈ Δ := (hinvh x).2 hx
      have h2 : A.wordPerm (A.conjWord h U) (A.wordPerm h x) ∈ Δ := (hinvh' _).2 h1
      have h3 : (A.wordPerm h)⁻¹ (A.wordPerm (A.conjWord h U) (A.wordPerm h x)) ∈ Δ :=
        hΔ.invariant.inv_mem h h2
      rw [restrictTo_apply_of_mem hinvh hx, restrictTo_apply_of_mem hinvh' h1,
        restrictTo_inv_apply_of_mem hinvh h2, restrictTo_inv_apply_of_mem hinvh' h3]
    · have h1 : A.wordPerm h x ∉ Δ := fun h' => hx ((hinvh x).1 h')
      rw [restrictTo_apply_of_notMem hinvh hx, restrictTo_apply_of_notMem hinvh' hx,
        restrictTo_inv_apply_of_notMem hinvh hx, restrictTo_inv_apply_of_notMem hinvh' hx,
        hout _ h1, perm_inv_apply_self, Perm.inv_eq_iff_eq, hout x hx]
  have hC : A.wordPerm (A.seedWord h U) =
      cycle3 z ((A.wordPerm (A.conjWord h U))⁻¹ z) ((A.wordPerm h)⁻¹ z) := by
    rw [show A.seedWord h U = h ++ A.conjWord h U ++ A.invWord h ++ A.invWord (A.conjWord h U)
      from rfl, A.wordPerm_commutator, hagree, hprod]
  refine ⟨U, hUfix, hUlen, z, hzR, ?_, hC, hΔ.invariant.inv_mem _ hzΔ,
    hΔ.invariant.inv_mem _ hzΔ, hd2, hd1, fun h' => hd3 h'.symm, ?_⟩
  · rw [hRU, hR'R]
  · rw [length_seedWord]
    exact Nat.add_le_add_left (Nat.mul_le_mul_left 4 hUlen) _

/-- **R22B-08**, the seed: the word `C` of `jordanSeed` acts on `Δ` as a three-cycle and fixes
every chamber outside `Δ`, so it is a seed (H2 of R22B-01) for `Δ` relative to every set `E` of
chambers outside `Δ`, such as the union of the orbits before `Δ` in any order, with
`|C| ≤ 4q + 4·[ℓM + 2(d - p)]·[C(d, r) - 1]`. -/
theorem jordanSeed_isSeed {Δ : Finset Ω} (hΔ : A.IsOrbit Δ) {w : List L} {p : ℕ}
    (hJ : A.IsJordanCertificate Δ w p) {h : List L} (hr2 : 2 ≤ (A.movedOn h Δ).card)
    (hr : 2 * (A.movedOn h Δ).card ≤ Δ.card + 1) :
    ∃ (C : List L) (x y z : Ω), (∀ E : Finset Ω, _root_.Disjoint E Δ → A.IsSeed Δ E x y z C) ∧
      C.length ≤ 4 * h.length + 4 * ((w.length * A.jordanExp w p + 2 * (Δ.card - p)) *
        (Δ.card.choose (A.movedOn h Δ).card - 1)) := by
  obtain ⟨U, -, -, z, hzR, -, hC, hy, hx, hzy, hzx, hyx, hlen⟩ := A.jordanSeed hΔ hJ hr2 hr
  have hzΔ : z ∈ Δ := (mem_inter.1 hzR).2
  refine ⟨A.seedWord h U, z, _, _, fun E hE => ⟨hzΔ, hy, hx, hzy, hzx, hyx,
    fun q _ => by rw [hC], fun q hq => ?_⟩, hlen⟩
  have hqΔ : q ∉ Δ := disjoint_left.1 hE hq
  rw [hC]
  exact cycle3_apply_of_ne (fun h' => hqΔ (h' ▸ hzΔ)) (fun h' => hqΔ (h' ▸ hy))
    (fun h' => hqΔ (h' ▸ hx))

end Alphabet

end Jumbling
