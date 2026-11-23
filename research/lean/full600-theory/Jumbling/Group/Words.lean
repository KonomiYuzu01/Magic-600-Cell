import ArchonFull600Theory.Basic

set_option linter.style.header false in
/-!
# Words over an alphabet closed under inverses

The shared setting of the group-theory layer of
`research/jumbling/restoration/12-low-height/ARCHON-HANDOFF.md` (section 1, layer "group"), with
the conventions of `PROOFS-B.md` section 0 and the object O5 (SAFE-ALL, N1, N2).

* A finite type `Ω` of chambers and an `Alphabet`: letters that act on `Ω` as permutations, each
  with an inverse letter. The letters stand for the SAFE-ALL letters `A(Z)` of O5, which are closed
  under inverses (O2). `Alphabet.group` is the group `G` they generate.
* Words are chronological: the first letter acts first, `uv` means `u`, then `v`, and the image
  `x·w` of a chamber `x` under a word `w` is `A.wordPerm w x`. In Mathlib's product the right
  factor acts first, so `A.wordPerm (u ++ v) = A.wordPerm v * A.wordPerm u`. This is
  `ArchonFull600Theory.wordPermutation` with the whole symmetric group as the legal subgroup
  (`Alphabet.wordPerm_eq_wordPermutation`). The length of a word is the length of its list; the
  inverse word `A.invWord w` reverses `w` and inverts each letter, and has the same length.
* The holes after a word `w` are the image `H.image (A.wordPerm w)` of the holes `H` (O5: a SAFE-ALL
  word maps the holes by `π_w`).
* Orbits of `G`, the fixed chambers `Fix(G)`, and the conditions N1 and N2 of O5.
* `exists_short_walk`: a walk whose steps keep a finite set `Y` can be replaced by one with the same
  end and at most `|Y| - 1` steps. This is the breadth-first-tree and shortest-path argument of
  R22B-01, R22B-02, R22B-05 and R22B-06.
-/

namespace Jumbling

open Equiv Finset

theorem perm_inv_apply_self {Ω : Type*} (f : Perm Ω) (x : Ω) : f⁻¹ (f x) = x := by
  simp

theorem perm_apply_inv_self {Ω : Type*} (f : Perm Ω) (x : Ω) : f (f⁻¹ x) = x := by
  simp

/-- An alphabet of letters acting on the chambers `Ω`, closed under inverses: each letter has an
inverse letter (O2, O5). -/
structure Alphabet (L Ω : Type*) where
  /-- The permutation of the chambers that a letter induces. -/
  act : L → Perm Ω
  /-- The inverse letter. -/
  inv : L → L
  /-- The inverse letter induces the inverse permutation. -/
  act_inv : ∀ l, act (inv l) = (act l)⁻¹

/-! ### Walks -/

/-- The end of a walk from `x` whose steps act in the order of the list. -/
def walk {X β : Type*} (move : β → X → X) (x : X) (ws : List β) : X :=
  ws.foldl (fun y g => move g y) x

section Walk

variable {X β : Type*} (move : β → X → X)

@[simp]
theorem walk_nil (x : X) : walk move x [] = x := rfl

@[simp]
theorem walk_cons (x : X) (g : β) (ws : List β) :
    walk move x (g :: ws) = walk move (move g x) ws := rfl

theorem walk_append (x : X) (u v : List β) :
    walk move x (u ++ v) = walk move (walk move x u) v :=
  List.foldl_append

/-- A walk whose steps keep `Y` stays in `Y`. -/
theorem walk_mem {Gen : Set β} {Y : Set X} (hY : ∀ g ∈ Gen, ∀ y ∈ Y, move g y ∈ Y)
    {ws : List β} (hws : ∀ g ∈ ws, g ∈ Gen) {x : X} (hx : x ∈ Y) : walk move x ws ∈ Y := by
  induction ws generalizing x with
  | nil => exact hx
  | cons g ws ih =>
    exact ih (fun g' hg' => hws g' (List.mem_cons_of_mem g hg'))
      (hY g (hws g List.mem_cons_self) x hx)

/-- Shortest walks: a walk from `x ∈ Y` whose steps lie in `Gen` and keep the finite set `Y` can be
replaced by a walk with the same steps allowed, the same end and at most `|Y| - 1` steps. A shortest
walk visits no point twice. -/
theorem exists_short_walk {Gen : Set β} {Y : Finset X} (hY : ∀ g ∈ Gen, ∀ y ∈ Y, move g y ∈ Y)
    {x : X} (hx : x ∈ Y) (ws : List β) (hws : ∀ g ∈ ws, g ∈ Gen) :
    ∃ ws' : List β, (∀ g ∈ ws', g ∈ Gen) ∧ ws'.length + 1 ≤ Y.card ∧
      walk move x ws' = walk move x ws := by
  induction hn : ws.length using Nat.strong_induction_on generalizing ws with
  | _ n ih =>
  let p : ℕ → X := fun k => walk move x (ws.take k)
  have hpY : ∀ k, p k ∈ Y := fun k =>
    walk_mem move (Y := (Y : Set X)) hY (fun g hg => hws g (List.mem_of_mem_take hg)) hx
  by_cases hinj : Set.InjOn p (Finset.range (n + 1))
  · refine ⟨ws, hws, ?_, rfl⟩
    have := Finset.card_le_card_of_injOn p (fun k _ => hpY k) hinj
    simpa [hn] using this
  · simp only [Set.InjOn, not_forall] at hinj
    obtain ⟨i, hi, j, hj, hij, hne⟩ := hinj
    simp only [coe_range, Set.mem_Iio] at hi hj
    -- Cut the loop between the two visits of the same point.
    have cut : ∀ i j, i < j → j ≤ n → p i = p j → ∃ ws' : List β, (∀ g ∈ ws', g ∈ Gen) ∧
        ws'.length + 1 ≤ Y.card ∧ walk move x ws' = walk move x ws := by
      intro i j hlt hjn hp
      let ws₁ := ws.take i ++ ws.drop j
      have hlen : ws₁.length < n := by
        simp only [ws₁, List.length_append, List.length_take, List.length_drop, hn]
        omega
      have hend : walk move x ws₁ = walk move x ws := by
        have hsplit : ws = ws.take j ++ ws.drop j := (List.take_append_drop j ws).symm
        rw [walk_append]
        conv_rhs => rw [hsplit, walk_append]
        exact congrArg (fun y => walk move y (ws.drop j)) hp
      have hws₁ : ∀ g ∈ ws₁, g ∈ Gen := by
        intro g hg
        rcases List.mem_append.1 hg with hg | hg
        · exact hws g (List.mem_of_mem_take hg)
        · exact hws g (List.mem_of_mem_drop hg)
      obtain ⟨ws', h1, h2, h3⟩ := ih ws₁.length hlen ws₁ hws₁ rfl
      exact ⟨ws', h1, h2, h3.trans hend⟩
    rcases lt_or_gt_of_ne hne with hlt | hlt
    · exact cut i j hlt (by omega) hij
    · exact cut j i hlt (by omega) hij.symm

end Walk

namespace Alphabet

variable {L Ω : Type*} (A : Alphabet L Ω)

/-! ### Words -/

/-- The permutation of a chronological word: the first letter acts first, and the image of `x`
under `w` is `A.wordPerm w x`. -/
def wordPerm (w : List L) : Perm Ω :=
  w.foldl (fun g l => A.act l * g) 1

theorem foldl_act_mul (w : List L) (g : Perm Ω) :
    w.foldl (fun g l => A.act l * g) g = A.wordPerm w * g := by
  induction w generalizing g with
  | nil => simp [wordPerm]
  | cons l w ih =>
    simp only [wordPerm, List.foldl_cons]
    rw [ih, ih, mul_one, mul_assoc]

@[simp]
theorem wordPerm_nil : A.wordPerm [] = 1 := rfl

@[simp]
theorem wordPerm_cons (l : L) (w : List L) : A.wordPerm (l :: w) = A.wordPerm w * A.act l := by
  rw [wordPerm, List.foldl_cons, foldl_act_mul, mul_one]

@[simp]
theorem wordPerm_append (u v : List L) : A.wordPerm (u ++ v) = A.wordPerm v * A.wordPerm u := by
  rw [wordPerm, List.foldl_append, foldl_act_mul, foldl_act_mul, mul_one]

theorem wordPerm_singleton (l : L) : A.wordPerm [l] = A.act l := by
  simp

theorem wordPerm_append_apply (u v : List L) (x : Ω) :
    A.wordPerm (u ++ v) x = A.wordPerm v (A.wordPerm u x) := by
  rw [wordPerm_append, Perm.mul_apply]

/-- The chronological convention is Full600's: `wordPerm` is `ArchonFull600Theory.wordPermutation`
with the whole symmetric group as the legal subgroup. -/
theorem wordPerm_eq_wordPermutation (w : List L) :
    (ArchonFull600Theory.wordPermutation (legal := ⊤)
      (fun l => (⟨A.act l, Subgroup.mem_top _⟩ : (⊤ : Subgroup (Perm Ω)))) w).val =
      A.wordPerm w := by
  have key : ∀ (w : List L) (g : (⊤ : Subgroup (Perm Ω))),
      (w.foldl (fun g l => (⟨A.act l, Subgroup.mem_top _⟩ : (⊤ : Subgroup (Perm Ω))) * g) g).val =
        w.foldl (fun g l => A.act l * g) g.val := by
    intro w
    induction w with
    | nil => intro g; rfl
    | cons l w ih => intro g; simp only [List.foldl_cons]; rw [ih]; rfl
  exact key w 1

/-- The inverse word: the letters in reverse order, each replaced by its inverse letter. -/
def invWord (w : List L) : List L :=
  (w.map A.inv).reverse

@[simp]
theorem length_invWord (w : List L) : (A.invWord w).length = w.length := by
  simp [invWord]

theorem invWord_cons (l : L) (w : List L) : A.invWord (l :: w) = A.invWord w ++ [A.inv l] := by
  simp [invWord]

@[simp]
theorem wordPerm_invWord (w : List L) : A.wordPerm (A.invWord w) = (A.wordPerm w)⁻¹ := by
  induction w with
  | nil => simp [invWord]
  | cons l w ih =>
    rw [invWord_cons, wordPerm_append, ih, wordPerm_singleton, A.act_inv, wordPerm_cons,
      mul_inv_rev]

theorem wordPerm_invWord_apply_self (w : List L) (x : Ω) :
    A.wordPerm (A.invWord w) (A.wordPerm w x) = x := by
  simp

theorem wordPerm_apply_invWord_self (w : List L) (x : Ω) :
    A.wordPerm w (A.wordPerm (A.invWord w) x) = x := by
  simp

/-- The chronological conjugate `g⁻¹·C·g` acts at `x` by `g (C (g⁻¹ x))`. -/
theorem wordPerm_conj_apply (g C : List L) (x : Ω) :
    A.wordPerm (A.invWord g ++ C ++ g) x = A.wordPerm g (A.wordPerm C ((A.wordPerm g)⁻¹ x)) := by
  simp

/-- A word's image of a point is the end of the walk of its letters. -/
theorem wordPerm_apply_eq_walk (w : List L) (x : Ω) :
    A.wordPerm w x = walk (fun l y => A.act l y) x w := by
  induction w generalizing x with
  | nil => rfl
  | cons l w ih => rw [wordPerm_cons, Perm.mul_apply, ih]; rfl

/-- The image of a point under a concatenation of words is the end of the walk of the words. -/
theorem wordPerm_flatten_apply (ws : List (List L)) (x : Ω) :
    A.wordPerm ws.flatten x = walk (fun u y => A.wordPerm u y) x ws := by
  induction ws generalizing x with
  | nil => rfl
  | cons u ws ih => rw [List.flatten_cons, wordPerm_append_apply, ih]; rfl

theorem length_flatten_le (ws : List (List L)) (ℓ : ℕ) (h : ∀ u ∈ ws, u.length ≤ ℓ) :
    ws.flatten.length ≤ ws.length * ℓ := by
  induction ws with
  | nil => simp
  | cons u ws ih =>
    rw [List.flatten_cons, List.length_append, List.length_cons, Nat.succ_mul]
    have h1 := h u List.mem_cons_self
    have h2 := ih (fun v hv => h v (List.mem_cons_of_mem u hv))
    omega

/-- A word fixes every point that each of its letters fixes. -/
theorem wordPerm_apply_of_forall {w : List L} {x : Ω} (h : ∀ l ∈ w, A.act l x = x) :
    A.wordPerm w x = x := by
  induction w with
  | nil => rfl
  | cons l w ih =>
    rw [wordPerm_cons, Perm.mul_apply, h l List.mem_cons_self]
    exact ih (fun l' hl' => h l' (List.mem_cons_of_mem l hl'))

/-- A concatenation of words fixes every point that each of the words fixes. -/
theorem wordPerm_flatten_apply_of_forall {ws : List (List L)} {x : Ω}
    (h : ∀ u ∈ ws, A.wordPerm u x = x) : A.wordPerm ws.flatten x = x := by
  induction ws with
  | nil => rfl
  | cons u ws ih =>
    rw [List.flatten_cons, wordPerm_append_apply, h u List.mem_cons_self]
    exact ih (fun v hv => h v (List.mem_cons_of_mem u hv))

theorem wordPerm_invWord_apply_of_apply {w : List L} {x y : Ω} (h : A.wordPerm w x = y) :
    A.wordPerm (A.invWord w) y = x := by
  rw [← h, wordPerm_invWord_apply_self]

/-! ### The group -/

/-- The group `G` generated by the letters (O5). -/
def group : Subgroup (Perm Ω) :=
  Subgroup.closure (Set.range A.act)

theorem wordPerm_mem_group (w : List L) : A.wordPerm w ∈ A.group := by
  induction w with
  | nil => exact A.group.one_mem
  | cons l w ih =>
    rw [wordPerm_cons]
    exact A.group.mul_mem ih (Subgroup.subset_closure ⟨l, rfl⟩)

/-- The elements of `G` are exactly the permutations of words, since the alphabet is closed under
inverses. -/
theorem mem_group_iff {g : Perm Ω} : g ∈ A.group ↔ ∃ w, A.wordPerm w = g := by
  refine ⟨fun hg => ?_, fun ⟨w, hw⟩ => hw ▸ A.wordPerm_mem_group w⟩
  induction hg using Subgroup.closure_induction with
  | mem x hx => obtain ⟨l, rfl⟩ := hx; exact ⟨[l], wordPerm_singleton A l⟩
  | one => exact ⟨[], rfl⟩
  | mul x y _ _ hx hy =>
    obtain ⟨u, rfl⟩ := hx
    obtain ⟨v, rfl⟩ := hy
    exact ⟨v ++ u, wordPerm_append A v u⟩
  | inv x _ hx =>
    obtain ⟨u, rfl⟩ := hx
    exact ⟨A.invWord u, wordPerm_invWord A u⟩

/-! ### Fixed chambers, invariant sets and orbits -/

/-- A chamber that every letter fixes, so that the whole group fixes it: `Fix(G)` of O5. -/
def IsFixed (x : Ω) : Prop :=
  ∀ l, A.act l x = x

variable {A} in
theorem IsFixed.wordPerm_apply {x : Ω} (hx : A.IsFixed x) (w : List L) : A.wordPerm w x = x :=
  A.wordPerm_apply_of_forall fun l _ => hx l

/-- A set of chambers that every letter maps into itself, such as a union of orbits. -/
def IsInvariant (E : Finset Ω) : Prop :=
  ∀ l, ∀ x ∈ E, A.act l x ∈ E

variable {A} in
theorem IsInvariant.wordPerm_mem {E : Finset Ω} (hE : A.IsInvariant E) (w : List L) {x : Ω}
    (hx : x ∈ E) : A.wordPerm w x ∈ E := by
  rw [wordPerm_apply_eq_walk]
  exact walk_mem _ (Gen := Set.univ) (Y := (E : Set Ω)) (fun l _ y hy => hE l y hy)
    (fun _ _ => Set.mem_univ _) hx

variable {A} in
theorem IsInvariant.wordPerm_mem_iff {E : Finset Ω} (hE : A.IsInvariant E) (w : List L) {x : Ω} :
    A.wordPerm w x ∈ E ↔ x ∈ E := by
  refine ⟨fun h => ?_, hE.wordPerm_mem w⟩
  have := hE.wordPerm_mem (A.invWord w) h
  rwa [wordPerm_invWord_apply_self] at this

variable {A} in
theorem IsInvariant.inv_mem {E : Finset Ω} (hE : A.IsInvariant E) (w : List L) {x : Ω}
    (hx : x ∈ E) : (A.wordPerm w)⁻¹ x ∈ E := by
  rw [← wordPerm_invWord]
  exact hE.wordPerm_mem _ hx

variable {A} in
theorem IsInvariant.union [DecidableEq Ω] {E F : Finset Ω} (hE : A.IsInvariant E)
    (hF : A.IsInvariant F) : A.IsInvariant (E ∪ F) := by
  intro l x hx
  rcases Finset.mem_union.1 hx with hx | hx
  · exact Finset.mem_union_left _ (hE l x hx)
  · exact Finset.mem_union_right _ (hF l x hx)

variable {A} in
theorem IsInvariant.biUnion [DecidableEq Ω] {ι : Type*} {s : Finset ι} {E : ι → Finset Ω}
    (hE : ∀ i ∈ s, A.IsInvariant (E i)) : A.IsInvariant (s.biUnion E) := by
  intro l x hx
  obtain ⟨i, hi, hx⟩ := Finset.mem_biUnion.1 hx
  exact Finset.mem_biUnion.2 ⟨i, hi, hE i hi l x hx⟩

variable {A} in
/-- A word that fixes a set `F` pointwise maps the rest of an invariant set `E` into itself. -/
theorem IsInvariant.wordPerm_mem_sdiff [DecidableEq Ω] {E F : Finset Ω} (hE : A.IsInvariant E)
    {w : List L} (hw : ∀ p ∈ F, A.wordPerm w p = p) {x : Ω} (hx : x ∈ E \ F) :
    A.wordPerm w x ∈ E \ F := by
  rw [Finset.mem_sdiff] at hx ⊢
  refine ⟨hE.wordPerm_mem w hx.1, fun hF => hx.2 ?_⟩
  have h := hw _ hF
  rw [(A.wordPerm w).injective h] at hF
  exact hF

/-- An orbit of the group: a nonempty set of chambers that every letter maps into itself and any
two of whose chambers are joined by a word. -/
structure IsOrbit (Δ : Finset Ω) : Prop where
  nonempty : Δ.Nonempty
  invariant : A.IsInvariant Δ
  connected : ∀ x ∈ Δ, ∀ y ∈ Δ, ∃ w, A.wordPerm w x = y

variable {A} in
/-- Two orbits that share a chamber are equal. -/
theorem IsOrbit.eq_of_mem {Δ Δ' : Finset Ω} (h : A.IsOrbit Δ) (h' : A.IsOrbit Δ') {x : Ω}
    (hx : x ∈ Δ) (hx' : x ∈ Δ') : Δ = Δ' := by
  ext y
  constructor
  · intro hy
    obtain ⟨w, rfl⟩ := h.connected x hx y hy
    exact h'.invariant.wordPerm_mem w hx'
  · intro hy
    obtain ⟨w, rfl⟩ := h'.connected x hx' y hy
    exact h.invariant.wordPerm_mem w hx

variable {A} in
/-- Distinct orbits are disjoint. -/
theorem IsOrbit.disjoint {Δ Δ' : Finset Ω} (h : A.IsOrbit Δ) (h' : A.IsOrbit Δ') (hne : Δ ≠ Δ') :
    Disjoint Δ Δ' :=
  Finset.disjoint_left.2 fun _ hx hx' => hne (h.eq_of_mem h' hx hx')

section Finite

variable [Fintype Ω]

/-- The orbit of a chamber. -/
noncomputable def orbitOf (x : Ω) : Finset Ω := by
  classical exact Finset.univ.filter fun y => ∃ w, A.wordPerm w x = y

theorem mem_orbitOf {x y : Ω} : y ∈ A.orbitOf x ↔ ∃ w, A.wordPerm w x = y := by
  simp [orbitOf]

theorem mem_orbitOf_self (x : Ω) : x ∈ A.orbitOf x :=
  (A.mem_orbitOf).2 ⟨[], rfl⟩

theorem isOrbit_orbitOf (x : Ω) : A.IsOrbit (A.orbitOf x) where
  nonempty := ⟨x, A.mem_orbitOf_self x⟩
  invariant := by
    intro l y hy
    obtain ⟨w, rfl⟩ := (A.mem_orbitOf).1 hy
    exact (A.mem_orbitOf).2 ⟨w ++ [l], by rw [wordPerm_append_apply, wordPerm_singleton]⟩
  connected := by
    intro y hy z hz
    obtain ⟨u, rfl⟩ := (A.mem_orbitOf).1 hy
    obtain ⟨v, rfl⟩ := (A.mem_orbitOf).1 hz
    exact ⟨A.invWord u ++ v, by rw [wordPerm_append_apply, wordPerm_invWord_apply_self]⟩

variable {A} in
theorem IsOrbit.eq_orbitOf {Δ : Finset Ω} (h : A.IsOrbit Δ) {x : Ω} (hx : x ∈ Δ) :
    Δ = A.orbitOf x :=
  h.eq_of_mem (A.isOrbit_orbitOf x) hx (A.mem_orbitOf_self x)

/-- A chamber that some letter moves has an orbit of at least two chambers. -/
theorem two_le_card_orbitOf {x : Ω} (hx : ¬A.IsFixed x) :
    2 ≤ (A.orbitOf x).card := by
  classical
  simp only [IsFixed, not_forall] at hx
  obtain ⟨l, hl⟩ := hx
  have hsub : ({x, A.act l x} : Finset Ω) ⊆ A.orbitOf x := by
    intro y hy
    rcases Finset.mem_insert.1 hy with rfl | hy
    · exact A.mem_orbitOf_self y
    · rw [Finset.mem_singleton.1 hy]
      exact (A.mem_orbitOf).2 ⟨[l], by rw [wordPerm_singleton]⟩
  have hcard : ({x, A.act l x} : Finset Ω).card = 2 := Finset.card_pair (Ne.symm hl)
  exact hcard ▸ Finset.card_le_card hsub

end Finite

/-! ### Holes and the conditions N1 and N2 -/

theorem mem_image_perm [DecidableEq Ω] (σ : Perm Ω) (H : Finset Ω) (y : Ω) :
    y ∈ H.image σ ↔ σ⁻¹ y ∈ H := by
  constructor
  · intro hy
    obtain ⟨x, hx, rfl⟩ := Finset.mem_image.1 hy
    simpa using hx
  · intro hy
    exact Finset.mem_image.2 ⟨σ⁻¹ y, hy, by simp⟩

/-- The holes after `u ++ v` are the holes after `u`, moved by `v`. -/
theorem image_wordPerm_append [DecidableEq Ω] (H : Finset Ω) (u v : List L) :
    H.image (A.wordPerm (u ++ v)) = (H.image (A.wordPerm u)).image (A.wordPerm v) := by
  rw [wordPerm_append, Perm.coe_mul, Finset.image_image]

/-- A chamber that a word fixes keeps its status. -/
theorem mem_image_of_fixed [DecidableEq Ω] {H : Finset Ω} {w : List L} {x : Ω}
    (hw : A.wordPerm w x = x) (hx : x ∈ H) : x ∈ H.image (A.wordPerm w) :=
  Finset.mem_image.2 ⟨x, hx, hw⟩

/-- Words keep the number of holes in every invariant set. -/
theorem card_image_inter [DecidableEq Ω] {E : Finset Ω} (hE : A.IsInvariant E) (H : Finset Ω)
    (w : List L) : (H.image (A.wordPerm w) ∩ E).card = (H ∩ E).card := by
  have heq : H.image (A.wordPerm w) ∩ E = (H ∩ E).image (A.wordPerm w) := by
    ext y
    simp only [Finset.mem_inter, Finset.mem_image]
    constructor
    · rintro ⟨⟨x, hx, rfl⟩, hy⟩
      exact ⟨x, ⟨hx, (hE.wordPerm_mem_iff w).1 hy⟩, rfl⟩
    · rintro ⟨x, ⟨hx, hxE⟩, rfl⟩
      exact ⟨⟨x, hx, rfl⟩, hE.wordPerm_mem w hxE⟩
  rw [heq, Finset.card_image_of_injective _ (A.wordPerm w).injective]

/-- N1 of O5: every target chamber in `Fix(G)` is a hole. -/
def N1 (H T : Finset Ω) : Prop :=
  ∀ t ∈ T, A.IsFixed t → t ∈ H

/-- N2 of O5: every orbit holds at least as many holes as targets. -/
def N2 [DecidableEq Ω] (H T : Finset Ω) : Prop :=
  ∀ Δ, A.IsOrbit Δ → (T ∩ Δ).card ≤ (H ∩ Δ).card

variable {A} in
/-- N1 is the case of N2 at the singleton orbits. -/
theorem N2.n1 [DecidableEq Ω] {H T : Finset Ω} (h : A.N2 H T) : A.N1 H T := by
  intro t ht hfix
  have horb : A.IsOrbit {t} :=
    { nonempty := Finset.singleton_nonempty t
      invariant := fun l x hx => by
        rw [Finset.mem_singleton] at hx ⊢
        rw [hx, hfix l]
      connected := fun x hx y hy => ⟨[], by
        rw [Finset.mem_singleton] at hx hy
        rw [hx, hy]; rfl⟩ }
  have hle := h {t} horb
  rw [Finset.inter_singleton_of_mem ht, Finset.card_singleton] at hle
  by_contra hH
  rw [Finset.inter_singleton_of_notMem hH, Finset.card_empty] at hle
  omega

variable {A} in
/-- A SAFE-ALL word keeps N1. -/
theorem N1.image [DecidableEq Ω] {H T : Finset Ω} (h : A.N1 H T) (w : List L) :
    A.N1 (H.image (A.wordPerm w)) T :=
  fun t ht hfix => A.mem_image_of_fixed (hfix.wordPerm_apply w) (h t ht hfix)

variable {A} in
/-- A SAFE-ALL word keeps N2: it keeps the number of holes in every orbit. -/
theorem N2.image [DecidableEq Ω] {H T : Finset Ω} (h : A.N2 H T) (w : List L) :
    A.N2 (H.image (A.wordPerm w)) T := by
  intro Δ hΔ
  rw [A.card_image_inter hΔ.invariant]
  exact h Δ hΔ

variable {A} in
/-- Under N2 there are at least as many holes as targets: `|T| ≤ n(Z)` (R22B-01). -/
theorem N2.card_le [Finite Ω] [DecidableEq Ω] {H T : Finset Ω} (h : A.N2 H T) :
    T.card ≤ H.card := by
  classical
  let _ : Fintype Ω := Fintype.ofFinite Ω
  let orbs : Finset (Finset Ω) := Finset.univ.image A.orbitOf
  have horbs : ∀ O ∈ orbs, A.IsOrbit O := by
    intro O hO
    obtain ⟨x, -, rfl⟩ := Finset.mem_image.1 hO
    exact A.isOrbit_orbitOf x
  have hdisj : ∀ (S : Finset Ω), (orbs : Set (Finset Ω)).PairwiseDisjoint (fun O => S ∩ O) := by
    intro S O hO O' hO' hne
    exact Finset.disjoint_of_subset_left Finset.inter_subset_right
      (Finset.disjoint_of_subset_right Finset.inter_subset_right
        ((horbs O hO).disjoint (horbs O' hO') hne))
  have hcover : ∀ (S : Finset Ω), S = orbs.biUnion (fun O => S ∩ O) := by
    intro S
    ext x
    simp only [Finset.mem_biUnion, Finset.mem_inter]
    exact ⟨fun hx => ⟨A.orbitOf x, Finset.mem_image_of_mem _ (Finset.mem_univ x), hx,
      A.mem_orbitOf_self x⟩, fun ⟨_, _, hx, _⟩ => hx⟩
  rw [hcover T, hcover H, Finset.card_biUnion (hdisj T), Finset.card_biUnion (hdisj H)]
  exact Finset.sum_le_sum fun O hO => h O (horbs O hO)

end Alphabet

/-! ### Three-cycles -/

section Cycles

variable {Ω : Type*} [DecidableEq Ω]

/-- The three-cycle `x ↦ y ↦ z ↦ x`, written as Full600 writes it, `swap x y * swap y z`. -/
def cycle3 (x y z : Ω) : Perm Ω :=
  swap x y * swap y z

theorem cycle3_apply_left {x y z : Ω} (hxy : x ≠ y) (hxz : x ≠ z) : cycle3 x y z x = y := by
  simp [cycle3, swap_apply_of_ne_of_ne hxy hxz]

theorem cycle3_apply_mid {x y z : Ω} (hxz : x ≠ z) (hyz : y ≠ z) : cycle3 x y z y = z := by
  simp [cycle3, swap_apply_of_ne_of_ne (Ne.symm hxz) (Ne.symm hyz)]

theorem cycle3_apply_right (x y z : Ω) : cycle3 x y z z = x := by
  simp [cycle3]

theorem cycle3_apply_of_ne {x y z p : Ω} (hx : p ≠ x) (hy : p ≠ y) (hz : p ≠ z) :
    cycle3 x y z p = p := by
  simp [cycle3, swap_apply_of_ne_of_ne hy hz, swap_apply_of_ne_of_ne hx hy]

theorem cycle3_inv {x y z : Ω} (hxy : x ≠ y) (hxz : x ≠ z) (hyz : y ≠ z) :
    (cycle3 x y z)⁻¹ = cycle3 x z y :=
  ArchonFull600Theory.cycle_inv hxy hxz hyz

theorem cycle3_rotate {x y z : Ω} (hxy : x ≠ y) (hxz : x ≠ z) (hyz : y ≠ z) :
    cycle3 x y z = cycle3 y z x :=
  ArchonFull600Theory.cycle_rotate hxy hxz hyz

/-- `(a x y)` followed by `(a z x)` is `(x y z)` (R22B-01, step 2). -/
theorem cycle3_mul_cycle3 {a x y z : Ω} (hax : a ≠ x) (hay : a ≠ y) (haz : a ≠ z) (hxy : x ≠ y)
    (hxz : x ≠ z) (hyz : y ≠ z) : cycle3 a z x * cycle3 a x y = cycle3 x y z :=
  ArchonFull600Theory.cycle_id_two hax hay haz hxy hxz hyz

/-- Conjugating a three-cycle moves its points. -/
theorem conj_cycle3 (g : Perm Ω) (x y z : Ω) : g * cycle3 x y z * g⁻¹ = cycle3 (g x) (g y) (g z) :=
  ArchonFull600Theory.PureControllerCertificate.conj_swap_mul_swap g x y z

/-- A three-cycle of three distinct points is even. -/
theorem cycle3_mem_alternatingGroup [Fintype Ω] {x y z : Ω} (hxy : x ≠ y) (hyz : y ≠ z) :
    cycle3 x y z ∈ alternatingGroup Ω := by
  rw [Equiv.Perm.mem_alternatingGroup, cycle3, map_mul, Equiv.Perm.sign_swap hxy,
    Equiv.Perm.sign_swap hyz]
  rfl

end Cycles

/-! ### Ordered pairs and triples of distinct chambers -/

section Tuples

variable {Ω : Type*} [DecidableEq Ω]

/-- The ordered pairs of distinct chambers of `Δ`. -/
def distinctPairs (Δ : Finset Ω) : Finset (Ω × Ω) :=
  Δ.biUnion fun p => (Δ.erase p).image fun q => (p, q)

theorem mem_distinctPairs {Δ : Finset Ω} {p q : Ω} :
    (p, q) ∈ distinctPairs Δ ↔ p ∈ Δ ∧ q ∈ Δ ∧ p ≠ q := by
  simp only [distinctPairs, Finset.mem_biUnion, Finset.mem_image, Finset.mem_erase, Prod.mk.injEq]
  constructor
  · rintro ⟨p', hp', q', ⟨hq'p, hq'⟩, rfl, rfl⟩
    exact ⟨hp', hq', Ne.symm hq'p⟩
  · rintro ⟨hp, hq, hpq⟩
    exact ⟨p, hp, q, ⟨Ne.symm hpq, hq⟩, rfl, rfl⟩

theorem card_distinctPairs_le (Δ : Finset Ω) :
    (distinctPairs Δ).card ≤ Δ.card * (Δ.card - 1) := by
  calc (distinctPairs Δ).card
      ≤ ∑ p ∈ Δ, ((Δ.erase p).image fun q => (p, q)).card := Finset.card_biUnion_le
    _ ≤ ∑ _p ∈ Δ, (Δ.card - 1) := Finset.sum_le_sum fun p hp =>
        Finset.card_image_le.trans (Finset.card_erase_of_mem hp).le
    _ = Δ.card * (Δ.card - 1) := by rw [Finset.sum_const, smul_eq_mul]

/-- The ordered triples of distinct chambers of `Δ`. -/
def distinctTriples (Δ : Finset Ω) : Finset (Ω × Ω × Ω) :=
  Δ.biUnion fun p => (Δ.erase p).biUnion fun q => ((Δ.erase p).erase q).image fun r => (p, q, r)

theorem mem_distinctTriples {Δ : Finset Ω} {p q r : Ω} :
    (p, q, r) ∈ distinctTriples Δ ↔ p ∈ Δ ∧ q ∈ Δ ∧ r ∈ Δ ∧ p ≠ q ∧ p ≠ r ∧ q ≠ r := by
  simp only [distinctTriples, Finset.mem_biUnion, Finset.mem_image, Finset.mem_erase,
    Prod.mk.injEq]
  constructor
  · rintro ⟨p', hp', q', ⟨hq'p, hq'⟩, r', ⟨hr'q, hr'p, hr'⟩, rfl, rfl, rfl⟩
    exact ⟨hp', hq', hr', Ne.symm hq'p, Ne.symm hr'p, Ne.symm hr'q⟩
  · rintro ⟨hp, hq, hr, hpq, hpr, hqr⟩
    exact ⟨p, hp, q, ⟨Ne.symm hpq, hq⟩, r, ⟨Ne.symm hqr, Ne.symm hpr, hr⟩, rfl, rfl, rfl⟩

theorem card_distinctTriples_le (Δ : Finset Ω) :
    (distinctTriples Δ).card ≤ Δ.card * (Δ.card - 1) * (Δ.card - 2) := by
  calc (distinctTriples Δ).card
      ≤ ∑ p ∈ Δ, ((Δ.erase p).biUnion fun q =>
          ((Δ.erase p).erase q).image fun r => (p, q, r)).card := Finset.card_biUnion_le
    _ ≤ ∑ _p ∈ Δ, (Δ.card - 1) * (Δ.card - 2) := by
        refine Finset.sum_le_sum fun p hp => ?_
        calc _ ≤ ∑ q ∈ Δ.erase p, (((Δ.erase p).erase q).image fun r => (p, q, r)).card :=
              Finset.card_biUnion_le
          _ ≤ ∑ _q ∈ Δ.erase p, (Δ.card - 2) := Finset.sum_le_sum fun q hq => by
              refine Finset.card_image_le.trans (le_of_eq ?_)
              rw [Finset.card_erase_of_mem hq, Finset.card_erase_of_mem hp]
              omega
          _ = (Δ.card - 1) * (Δ.card - 2) := by
              rw [Finset.sum_const, smul_eq_mul, Finset.card_erase_of_mem hp]
    _ = Δ.card * (Δ.card - 1) * (Δ.card - 2) := by
        rw [Finset.sum_const, smul_eq_mul, mul_assoc]

end Tuples

end Jumbling
