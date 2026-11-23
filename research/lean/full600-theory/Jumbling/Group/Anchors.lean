import Jumbling.Group.Schreier
import Jumbling.Group.Certified

set_option linter.style.header false in
/-!
# R22B-06: moving the anchors, a decision procedure and no guarantee

`PROOFS-B.md` section 3, R22B-06 (layer "group" of `ARCHON-HANDOFF.md`).

* `Alphabet.isSeed_conj`, `Alphabet.isSeed_conj_inv` (convention): with chronological words
  `g⁻¹·C·g` acts on `Δ` as the three-cycle `(a·g, b·g, c·g)`, and `g·C·g⁻¹` has the anchors
  `a·g⁻¹, b·g⁻¹, c·g⁻¹`.
* `Alphabet.selectionTest` (selection test): if at the pair `(a·g, b·g)` the graph on `Δ \ {a·g}`
  of the letters that fix `a·g` and the graph on `Δ \ {a·g, b·g}` of the letters that fix both are
  connected, `g⁻¹·C·g` with third anchor `c·g` is a seed with single-letter tables
  (`e ≤ d - 2`, `f ≤ d - 3`); the second test does not mention the third anchor.
* `Alphabet.exists_short_pair_word`, `Alphabet.selection_decides`: the pair orbit has at most
  `d(d - 1)` pairs, so a passing pair is reached by a word of at most `d(d - 1) - 1` letters, and
  exhausting the pair orbit decides whether any conjugate of `C` passes (both conventions,
  `Alphabet.exists_inv_anchor_iff`).
* `NinePoint.anchorCounterexample` (negative statement): on nine points, with
  `r = (0 1 2 3 4 5 6 7 8)`, `t = (0 1 2)`, `b = r·t` and the letters `r, r⁻¹, b, b⁻¹`, the word
  `r⁻¹·b = t` is a seed, `t·r⁻²·t·r² = (0 1 3 4 2)` is a Jordan word (`9/2 < 5 ≤ 9 - 3`), the
  letters generate `A₉` of order 181,440, and yet no anchor pair passes the selection test;
  conjugates of single letters are again derangements (`Alphabet.conj_derangement`), and the word
  tables of R22B-05 still exist (`NinePoint.schreierTables_nine`).

The letters of the nine-point alphabet are `0 = r`, `1 = r⁻¹`, `2 = b`, `3 = b⁻¹`. The kernel
checks of that example are small `decide +kernel` evaluations on `Fin 9`.
-/

namespace Jumbling

open Equiv Finset

namespace Alphabet

variable {L Ω : Type*} [DecidableEq Ω] (A : Alphabet L Ω)

/-! ### The conjugation convention -/

/-- R22B-06, convention: with chronological words, `g⁻¹·C·g` acts on `Δ` as the three-cycle
`(a·g, b·g, c·g)`, still fixes `E` pointwise, and has `|C| + 2|g|` letters. -/
theorem isSeed_conj {Δ E : Finset Ω} (hΔ : A.IsInvariant Δ) (hE : A.IsInvariant E) {a b c : Ω}
    {C : List L} (hC : A.IsSeed Δ E a b c C) (g : List L) :
    A.IsSeed Δ E (A.wordPerm g a) (A.wordPerm g b) (A.wordPerm g c) (A.invWord g ++ C ++ g) ∧
      (A.invWord g ++ C ++ g).length = C.length + 2 * g.length := by
  have hinj := (A.wordPerm g).injective
  refine ⟨⟨hΔ.wordPerm_mem g hC.ha, hΔ.wordPerm_mem g hC.hb, hΔ.wordPerm_mem g hC.hc,
    fun h => hC.hab (hinj h), fun h => hC.hac (hinj h), fun h => hC.hbc (hinj h),
    fun x hx => ?_, fun x hx => ?_⟩, ?_⟩
  · rw [wordPerm_conj_apply, hC.acts _ (hΔ.inv_mem g hx), ← Perm.mul_apply, ← Perm.mul_apply,
      conj_cycle3]
  · rw [wordPerm_conj_apply, hC.fixes _ (hE.inv_mem g hx), perm_apply_inv_self]
  · simp only [List.length_append, length_invWord]
    ring

/-- R22B-06, convention: `g·C·g⁻¹` has the anchors `a·g⁻¹, b·g⁻¹, c·g⁻¹`. -/
theorem isSeed_conj_inv {Δ E : Finset Ω} (hΔ : A.IsInvariant Δ) (hE : A.IsInvariant E)
    {a b c : Ω} {C : List L} (hC : A.IsSeed Δ E a b c C) (g : List L) :
    A.IsSeed Δ E ((A.wordPerm g)⁻¹ a) ((A.wordPerm g)⁻¹ b) ((A.wordPerm g)⁻¹ c)
      (g ++ C ++ A.invWord g) ∧ (g ++ C ++ A.invWord g).length = C.length + 2 * g.length := by
  have hinj := (A.wordPerm g)⁻¹.injective
  have happ : ∀ x, A.wordPerm (g ++ C ++ A.invWord g) x =
      (A.wordPerm g)⁻¹ (A.wordPerm C (A.wordPerm g x)) := fun x => by
    rw [wordPerm_append_apply, wordPerm_append_apply, wordPerm_invWord]
  refine ⟨⟨hΔ.inv_mem g hC.ha, hΔ.inv_mem g hC.hb, hΔ.inv_mem g hC.hc,
    fun h => hC.hab (hinj h), fun h => hC.hac (hinj h), fun h => hC.hbc (hinj h),
    fun x hx => ?_, fun x hx => ?_⟩, ?_⟩
  · have h := congrArg (fun σ : Perm Ω => σ x) (conj_cycle3 (A.wordPerm g)⁻¹ a b c)
    simp only [inv_inv, Perm.mul_apply] at h
    rw [← h, happ, hC.acts _ (hΔ.wordPerm_mem g hx)]
  · rw [happ, hC.fixes _ (hE.wordPerm_mem g hx), perm_inv_apply_self]
  · simp only [List.length_append, length_invWord]
    ring

/-! ### The selection test -/

/-- R22B-06, selection test: if at the pair `(a·g, b·g)` the graph on `Δ \ {a·g}` of the letters
that fix `a·g` is connected, and the graph on `Δ \ {a·g, b·g}` of the letters that fix `a·g` and
`b·g` is connected (a test that does not mention the third anchor), then `g⁻¹·C·g` with third anchor
`c·g` is a seed of `|C| + 2|g|` letters and has single-letter tables with `e ≤ d - 2` and
`f ≤ d - 3`. -/
theorem selectionTest {Δ E : Finset Ω} (hΔ : A.IsInvariant Δ) (hE : A.IsInvariant E) {a b c : Ω}
    {C : List L} (hC : A.IsSeed Δ E a b c C) (g : List L)
    (h₁ : A.StabConnected Δ {A.wordPerm g a})
    (h₂ : A.StabConnected Δ {A.wordPerm g a, A.wordPerm g b}) :
    A.IsSeed Δ E (A.wordPerm g a) (A.wordPerm g b) (A.wordPerm g c) (A.invWord g ++ C ++ g) ∧
      (A.invWord g ++ C ++ g).length = C.length + 2 * g.length ∧
      ∃ P Q : Ω → List L,
        A.IsTables Δ (A.wordPerm g a) (A.wordPerm g b) (A.wordPerm g c) P Q
          (Δ.card - 2) (Δ.card - 3) ∧
        (∀ x, ∀ l ∈ P x, A.act l (A.wordPerm g a) = A.wordPerm g a) ∧
        (∀ z, ∀ l ∈ Q z, A.act l (A.wordPerm g a) = A.wordPerm g a ∧
          A.act l (A.wordPerm g b) = A.wordPerm g b) := by
  obtain ⟨hC', hlen⟩ := A.isSeed_conj hΔ hE hC g
  exact ⟨hC', hlen, A.tables_of_connected hΔ hC'.ha hC'.hb hC'.hc hC'.hab hC'.hac hC'.hbc h₁ h₂⟩

omit [DecidableEq Ω] in
theorem walk_pair (w : List L) (p q : Ω) :
    walk (fun l (u : Ω × Ω) => (A.act l u.1, A.act l u.2)) (p, q) w =
      (A.wordPerm w p, A.wordPerm w q) := by
  induction w generalizing p q with
  | nil => rfl
  | cons l w ih => simp only [walk_cons, ih, wordPerm_cons, Perm.mul_apply]

omit [DecidableEq Ω] in
/-- R22B-06: the orbit of the ordered pair `(a, b)` of distinct chambers of `Δ` has at most
`d(d - 1)` pairs, so breadth-first search reaches every pair `(a·g, b·g)` by a word of at most
`d(d - 1) - 1` letters. -/
theorem exists_short_pair_word {Δ : Finset Ω} (hΔ : A.IsInvariant Δ) {a b : Ω} (ha : a ∈ Δ)
    (hb : b ∈ Δ) (hab : a ≠ b) (g : List L) :
    ∃ g' : List L, A.wordPerm g' a = A.wordPerm g a ∧ A.wordPerm g' b = A.wordPerm g b ∧
      g'.length + 1 ≤ Δ.card * (Δ.card - 1) := by
  classical
  obtain ⟨g', -, hlen, hend⟩ := exists_short_walk
    (fun l (u : Ω × Ω) => (A.act l u.1, A.act l u.2)) (Gen := Set.univ) (Y := distinctPairs Δ)
    (by
      rintro l - ⟨p, q⟩ hpq
      obtain ⟨hp, hq, hpq⟩ := mem_distinctPairs.1 hpq
      exact mem_distinctPairs.2 ⟨hΔ l p hp, hΔ l q hq, fun h => hpq ((A.act l).injective h)⟩)
    (mem_distinctPairs.2 ⟨ha, hb, hab⟩) g (fun _ _ => Set.mem_univ _)
  rw [walk_pair, walk_pair, Prod.mk.injEq] at hend
  exact ⟨g', hend.1, hend.2, hlen.trans (card_distinctPairs_le Δ)⟩

omit [DecidableEq Ω] in
/-- R22B-06: exhausting the pair orbit decides the selection test. For any test `Pass` on anchor
pairs (for example the two graph tests of `selectionTest`), some conjugate `g⁻¹·C·g` passes if and
only if one with a conjugating word of at most `d(d - 1) - 1` letters does. -/
theorem selection_decides {Δ : Finset Ω} (hΔ : A.IsInvariant Δ) {a b : Ω} (ha : a ∈ Δ)
    (hb : b ∈ Δ) (hab : a ≠ b) (Pass : Ω → Ω → Prop) :
    (∃ g : List L, Pass (A.wordPerm g a) (A.wordPerm g b)) ↔
      ∃ g : List L, g.length + 1 ≤ Δ.card * (Δ.card - 1) ∧
        Pass (A.wordPerm g a) (A.wordPerm g b) := by
  constructor
  · rintro ⟨g, hg⟩
    obtain ⟨g', h1, h2, hlen⟩ := A.exists_short_pair_word hΔ ha hb hab g
    exact ⟨g', hlen, by rwa [h1, h2]⟩
  · rintro ⟨g, -, hg⟩
    exact ⟨g, hg⟩

omit [DecidableEq Ω] in
/-- R22B-06: the anchor pairs of the conjugates `g·C·g⁻¹` (anchors `a·g⁻¹, b·g⁻¹`) are the same
as those of the conjugates `g⁻¹·C·g` (anchors `a·g, b·g`), so the pair orbit covers both
conventions. -/
theorem exists_inv_anchor_iff (a b : Ω) (Pass : Ω → Ω → Prop) :
    (∃ g : List L, Pass ((A.wordPerm g)⁻¹ a) ((A.wordPerm g)⁻¹ b)) ↔
      ∃ g : List L, Pass (A.wordPerm g a) (A.wordPerm g b) := by
  constructor
  · rintro ⟨g, hg⟩
    exact ⟨A.invWord g, by rwa [wordPerm_invWord]⟩
  · rintro ⟨g, hg⟩
    exact ⟨A.invWord g, by rwa [wordPerm_invWord, inv_inv]⟩

/-! ### Why the test can fail everywhere -/

/-- If some point of `F` is moved by every letter, the stabilizer graph on `Δ \ F` has no edges, so
it is not connected as soon as `Δ \ F` has two chambers. -/
theorem not_stabConnected {Δ F : Finset Ω} {p : Ω} (hp : p ∈ F) (hmove : ∀ l, A.act l p ≠ p)
    (hcard : 1 < (Δ \ F).card) : ¬A.StabConnected Δ F := by
  intro hconn
  obtain ⟨x, hx, y, hy, hxy⟩ := Finset.one_lt_card.1 hcard
  obtain ⟨w, hwF, hw⟩ := hconn x hx y hy
  cases w with
  | nil => exact hxy hw
  | cons l w => exact hmove l (hwF l List.mem_cons_self p hp)

omit [DecidableEq Ω] in
/-- R22B-06: a conjugate of a letter that moves every chamber again moves every chamber (in either
convention, since `σ` is any permutation), so longer conjugators around one letter do not add edges
to a stabilizer graph. -/
theorem conj_derangement {l : L} (hl : ∀ x, A.act l x ≠ x) (σ : Perm Ω) (x : Ω) :
    (σ * A.act l * σ⁻¹) x ≠ x := by
  intro h
  rw [Perm.mul_apply, Perm.mul_apply] at h
  apply hl (σ⁻¹ x)
  conv_rhs => rw [← h]
  rw [perm_inv_apply_self]

end Alphabet

/-! ### The nine-point counterexample -/

namespace NinePoint

/-- `r = (0 1 2 3 4 5 6 7 8)`. -/
def r : Perm (Fin 9) := List.formPerm [0, 1, 2, 3, 4, 5, 6, 7, 8]

/-- `t = (0 1 2)`. -/
def t : Perm (Fin 9) := List.formPerm [0, 1, 2]

/-- `b = r·t` in chronological order: first `r`, then `t`. -/
def b : Perm (Fin 9) := t * r

/-- The letters `0 = r`, `1 = r⁻¹`, `2 = b`, `3 = b⁻¹`. -/
def act : Fin 4 → Perm (Fin 9)
  | 0 => r
  | 1 => r⁻¹
  | 2 => b
  | 3 => b⁻¹

/-- The inverse letter. -/
def inv : Fin 4 → Fin 4
  | 0 => 1
  | 1 => 0
  | 2 => 3
  | 3 => 2

/-- The alphabet `S = {r, r⁻¹, b, b⁻¹}` on nine points. -/
def alphabet : Alphabet (Fin 4) (Fin 9) where
  act := act
  inv := inv
  act_inv l := by fin_cases l <;> simp [act, inv]

/-- The word `r⁻¹·b`. -/
def seedWord : List (Fin 4) := [1, 2]

/-- The word `t·r⁻²·t·r²`, with `t = r⁻¹·b`. -/
def jordanWord : List (Fin 4) := seedWord ++ [1, 1] ++ seedWord ++ [0, 0]

/-- The word `r⁻ᵏ·t·rᵏ`, which acts as the three-cycle `(k, k + 1, k + 2)`. -/
def consecutiveWord (k : ℕ) : List (Fin 4) :=
  List.replicate k 1 ++ seedWord ++ List.replicate k 0

/-- A word that fixes `0` and `1` and sends `2` to `X` (for `X ≠ 0, 1`), made of the consecutive
three-cycles `(k, k + 1, k + 2)` with `2 ≤ k ≤ 6`. -/
def coverWord : Fin 9 → List (Fin 4)
  | 3 => consecutiveWord 2
  | 4 => consecutiveWord 2 ++ consecutiveWord 3
  | 5 => consecutiveWord 2 ++ consecutiveWord 3 ++ consecutiveWord 4
  | 6 => consecutiveWord 2 ++ consecutiveWord 3 ++ consecutiveWord 4 ++ consecutiveWord 5
  | 7 => consecutiveWord 2 ++ consecutiveWord 3 ++ consecutiveWord 4 ++ consecutiveWord 5 ++
      consecutiveWord 6
  | 8 => consecutiveWord 2 ++ consecutiveWord 3 ++ consecutiveWord 4 ++ consecutiveWord 5 ++
      consecutiveWord 6 ++ consecutiveWord 6
  | _ => []

/-- R22B-06 counterexample: `r` is the nine-cycle `(0 1 2 3 4 5 6 7 8)` and `b` the nine-cycle
`(0 2 3 4 5 6 7 8 1)`. -/
theorem r_b_nineCycles : r.IsCycle ∧ r.support = Finset.univ ∧
    b = List.formPerm [0, 2, 3, 4, 5, 6, 7, 8, 1] ∧ b.IsCycle ∧ b.support = Finset.univ := by
  have hb : b = List.formPerm [0, 2, 3, 4, 5, 6, 7, 8, 1] := Equiv.ext (by decide +kernel)
  refine ⟨List.isCycle_formPerm (by decide) (by decide), ?_, hb, ?_, ?_⟩
  · rw [r, List.support_formPerm_of_nodup _ (by decide) (by simp)]
    decide
  · rw [hb]
    exact List.isCycle_formPerm (by decide) (by decide)
  · rw [hb, List.support_formPerm_of_nodup _ (by decide) (by simp)]
    decide

/-- R22B-06 counterexample: every letter moves every point. -/
theorem act_ne : ∀ (l : Fin 4) (x : Fin 9), alphabet.act l x ≠ x := by
  decide +kernel

/-- R22B-06 counterexample: for every choice of anchors, both single-letter stabilizer graphs have
no edges, so neither is connected and no anchor pair passes the selection test. -/
theorem not_stabConnected (a' b' : Fin 9) :
    ¬alphabet.StabConnected Finset.univ {a'} ∧ ¬alphabet.StabConnected Finset.univ {a', b'} := by
  have hcard : ∀ F : Finset (Fin 9), F.card ≤ 2 → 1 < (Finset.univ \ F).card := fun F hF => by
    have := Finset.card_le_card_sdiff_add_card (s := Finset.univ) (t := F)
    rw [Finset.card_univ, Fintype.card_fin] at this
    omega
  refine ⟨alphabet.not_stabConnected (Finset.mem_singleton_self a') (act_ne · a')
    (hcard _ (by simp)), alphabet.not_stabConnected (Finset.mem_insert_self a' {b'})
    (act_ne · a') (hcard _ ((Finset.card_insert_le _ _).trans (by simp)))⟩

/-- R22B-06 counterexample: `r⁻¹·b = t`, and `t` is the three-cycle `(0 1 2)`. -/
theorem seedWord_eq : alphabet.wordPerm seedWord = t ∧ t = cycle3 0 1 2 :=
  ⟨Equiv.ext (by decide +kernel), Equiv.ext (by decide +kernel)⟩

/-- R22B-06 counterexample: the conjugates of `t` by powers of `r` are the three-cycles of
consecutive points. -/
theorem consecutiveWord_eq (k : Fin 9) :
    alphabet.wordPerm (consecutiveWord k.val) = cycle3 k (k + 1) (k + 2) := by
  revert k
  exact fun k => Equiv.ext (by revert k; decide +kernel)

/-- R22B-06 counterexample, coverage: `coverWord X` fixes `0` and `1` and sends `2` to `X`. -/
theorem coverWord_spec : ∀ X : Fin 9, X ≠ 0 → X ≠ 1 →
    alphabet.wordPerm (coverWord X) 0 = 0 ∧ alphabet.wordPerm (coverWord X) 1 = 1 ∧
      alphabet.wordPerm (coverWord X) 2 = X := by
  decide +kernel

/-- R22B-06 counterexample: all nine points form one orbit. -/
theorem isOrbit_univ : alphabet.IsOrbit Finset.univ where
  nonempty := Finset.univ_nonempty
  invariant _ _ _ := Finset.mem_univ _
  connected x _ y _ := ⟨List.replicate (y - x).val 0, by revert x y; decide +kernel⟩

/-- The relocated-star certificate of `ArchonFull600Theory` for the nine points: seed `t = (0 1 2)`,
no relocation, buffers `0, 1`, anchor `2`, and `coverWord` for the coverage. -/
def certificate :
    ArchonFull600Theory.PureControllerCertificate alphabet.group (Set.univ : Set (Fin 9)) where
  seedA := ⟨0, Set.mem_univ _⟩
  seedB := ⟨1, Set.mem_univ _⟩
  seedC := ⟨2, Set.mem_univ _⟩
  bufferA := ⟨0, Set.mem_univ _⟩
  bufferB := ⟨1, Set.mem_univ _⟩
  anchor := ⟨2, Set.mem_univ _⟩
  seedDistinct := by decide
  relocatedDistinct := by decide
  seed := ⟨t, seedWord_eq.1 ▸ alphabet.wordPerm_mem_group seedWord⟩
  seed_eq := rfl
  relocation := 1
  relocation_eq := ⟨rfl, rfl, rfl⟩
  coverage X hA hB :=
    ⟨⟨_, alphabet.wordPerm_mem_group (coverWord X.val)⟩,
      coverWord_spec X.val (fun h => hA (Subtype.ext h)) (fun h => hB (Subtype.ext h))⟩

/-- R22B-06 counterexample: the nine-cycle `r` is even. -/
theorem r_mem_alternatingGroup : r ∈ alternatingGroup (Fin 9) := by
  rw [Perm.mem_alternatingGroup, r_b_nineCycles.1.sign, r_b_nineCycles.2.1, Finset.card_univ,
    Fintype.card_fin]
  decide

/-- R22B-06 counterexample: `S` generates `A₉`. -/
theorem group_eq : alphabet.group = alternatingGroup (Fin 9) := by
  apply le_antisymm
  · refine (Subgroup.closure_le _).2 ?_
    rintro _ ⟨l, rfl⟩
    have hr := r_mem_alternatingGroup
    have ht : t ∈ alternatingGroup (Fin 9) := by
      rw [seedWord_eq.2]
      exact cycle3_mem_alternatingGroup (by decide) (by decide)
    have hb : b ∈ alternatingGroup (Fin 9) := mul_mem ht hr
    fin_cases l
    · exact hr
    · exact inv_mem hr
    · exact hb
    · exact inv_mem hb
  · rw [← Perm.closure_three_cycles_eq_alternating, Subgroup.closure_le]
    intro τ hτ
    obtain ⟨x, y, z, hxy, hxz, hyz, rfl⟩ := ArchonFull600Theory.isThreeCycle_eq_swap_mul_swap hτ
    exact certificate.three_cycle_mem (Set.mem_univ x) (Set.mem_univ y) (Set.mem_univ z) hxy hxz
      hyz

/-- R22B-06 counterexample: the group `S` generates has order 181,440. -/
theorem card_group : Nat.card alphabet.group = 181440 := by
  rw [group_eq, nat_card_alternatingGroup, Nat.card_eq_fintype_card, Fintype.card_fin]
  decide

/-- R22B-06 counterexample: `t·r⁻²·t·r² = (0 1 3 4 2)` is a cycle of prime length `5` with
`9/2 < 5 ≤ 9 - 3` (a Jordan word). -/
theorem jordanWord_spec :
    alphabet.wordPerm jordanWord = List.formPerm [0, 1, 3, 4, 2] ∧
      (alphabet.wordPerm jordanWord).IsCycle ∧ (alphabet.wordPerm jordanWord).support.card = 5 ∧
      Nat.Prime 5 ∧ 9 < 2 * 5 ∧ 5 ≤ 9 - 3 := by
  have h : alphabet.wordPerm jordanWord = List.formPerm [0, 1, 3, 4, 2] :=
    Equiv.ext (by decide +kernel)
  refine ⟨h, ?_, ?_, Nat.prime_five, by norm_num, by norm_num⟩
  · rw [h]
    exact List.isCycle_formPerm (by decide) (by decide)
  · rw [h, List.support_formPerm_of_nodup _ (by decide) (by simp)]
    decide

/-- R22B-06 counterexample, the word tables of R22B-05 still exist: for the anchors `0, 1, 2`,
first-table words of at most `(9 - 2)(2·9 - 1) = 119` letters and second-table words of at most
`(9 - 3)(2·9 - 3)(2·9 - 1) = 1530` letters. -/
theorem schreierTables_nine :
    (∀ x : Fin 9, x ≠ 0 → ∃ P : List (Fin 4), alphabet.wordPerm P 0 = 0 ∧
        alphabet.wordPerm P x = 1 ∧ P.length ≤ 119) ∧
    (∀ z : Fin 9, z ≠ 0 → z ≠ 1 → ∃ Q : List (Fin 4), alphabet.wordPerm Q 0 = 0 ∧
        alphabet.wordPerm Q 1 = 1 ∧ alphabet.wordPerm Q 2 = z ∧ Q.length ≤ 1530) := by
  have hcard : (Finset.univ : Finset (Fin 9)).card = 9 := by simp
  have H1 : ∀ σ : Perm (Finset.univ : Finset (Fin 9)), σ ∈ alternatingGroup _ →
      ∃ g ∈ alphabet.group, ∀ x : (Finset.univ : Finset (Fin 9)), g x = σ x := by
    intro σ hσ
    refine ⟨Perm.ofSubtype σ, ?_, fun x => Perm.ofSubtype_apply_of_mem σ x.2⟩
    rw [group_eq, Perm.mem_alternatingGroup, Perm.sign_ofSubtype]
    exact hσ
  obtain ⟨h1, h2⟩ := alphabet.schreierTables isOrbit_univ (by rw [hcard]; norm_num)
    (a := 0) (b := 1) (c := 2) (Finset.mem_univ _) (Finset.mem_univ _) (Finset.mem_univ _)
    (by decide) (by decide) (by decide) H1
  rw [hcard] at h1 h2
  exact ⟨fun x hx => h1 x (Finset.mem_univ x) hx, fun z hz0 hz1 => h2 z (Finset.mem_univ z) hz0 hz1⟩

/-- R22B-06, negative statement: neither a seed nor a Jordan word guarantees that some anchor pair
passes the selection test. On the nine points (one orbit, `d = 9`, no earlier orbits) `r⁻¹·b` is a
seed, `t·r⁻²·t·r²` is a Jordan word, `S` generates `A₉`, and for every pair of anchors neither
single-letter stabilizer graph is connected. -/
theorem anchorCounterexample :
    alphabet.IsOrbit Finset.univ ∧ 5 ≤ (Finset.univ : Finset (Fin 9)).card ∧
      alphabet.IsSeed Finset.univ ∅ 0 1 2 seedWord ∧
      alphabet.group = alternatingGroup (Fin 9) ∧
      ((alphabet.wordPerm jordanWord).IsCycle ∧ (alphabet.wordPerm jordanWord).support.card = 5 ∧
        Nat.Prime 5 ∧ 9 < 2 * 5 ∧ 5 ≤ 9 - 3) ∧
      ∀ a' b' : Fin 9, ¬alphabet.StabConnected Finset.univ {a'} ∧
        ¬alphabet.StabConnected Finset.univ {a', b'} := by
  refine ⟨isOrbit_univ, by simp, ?_, group_eq, jordanWord_spec.2, not_stabConnected⟩
  refine ⟨Finset.mem_univ _, Finset.mem_univ _, Finset.mem_univ _, by decide, by decide,
    by decide, fun x _ => ?_, fun x hx => absurd hx (Finset.notMem_empty x)⟩
  rw [seedWord_eq.1, seedWord_eq.2]

end NinePoint

end Jumbling
