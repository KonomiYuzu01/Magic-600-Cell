import Jumbling.Model.Pieces
import Mathlib.GroupTheory.GroupAction.Quotient
import Mathlib.Algebra.Group.End

set_option linter.style.header false in
/-!
# The class-cell model (O7) and LC-03 (O8)

Source: `research/jumbling/restoration/12-low-height/PROOFS-B.md`, section 1, objects O7 and O8,
and `research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md`, LC-01 and LC-03.

Fix a class `P` of grips. `G = G_P` is a subgroup of the pose group `Γ`, `H_P = G_P ∩ K⁺` is
`M.K.subgroupOf G`, and the labels are the left cosets `G ⧸ H_P` (`PieceModel.Label`), with
label `0` the coset of `1` (`PieceModel.label0`). `ClassCellModel M G Cell Kind` has

* the class `P`, and `inCap c C`, which says that the cell `C` lies inside the cap of `c`; a menu
  element of `G` at a grip of `P` keeps that relation (a letter maps its cap onto itself);
* the kind of a cell, preserved by `G` (the number of caps of `P` that contain it);
* `chamberCell`, the cell of a home chamber, equivariant under `H_P` (a reading: `H_P ⊆ K⁺` maps
  a home chamber of `C` to a home chamber of `h • C`).

A home cell is a cell that holds a home chamber. A class-P letter `(c, g)` sends the point
`(C, ℓ)` to `(g • C, g • ℓ)` when `C` lies in the cap of `c` and fixes it otherwise.
`cellAcc w C` is the accumulated rotation `a_C(w)`, and the point map of `w` is
`(C, ℓ) ↦ (a_C(w) • C, a_C(w) • ℓ)` (`pointMap`, a permutation `pointPerm`).

`LC03 Q Z` is LC-03 (O8), an earlier-stage result (handoff section 5), stated as a hypothesis and
not derived. For the words over the class-P letters at the free grips of `Z`:

* every prefix is admissible (`admissible`);
* every class-P off piece of `Z` in the cell `C`, and every ghost of a home chamber of `C`,
  receives the one rotation `a_C(w)` (`own_apply`, `ghost_acc`), and no ghost straddles a cut
  (`ghost_clear`);
* foreign pieces lie outside every free cap (`foreign_outside`);
* a rotation of `G` that fixes a nonempty cell (a cell inside at least one cap of `P`) and all its
  labels is `1` (`rigid`).

The field `off_cases` is a modelling reading of O7's freeness: every off piece of `Z` is a
class-P off piece in some cell or a foreign piece.
-/

namespace Jumbling

variable {Γ : Type*} [Group Γ] {R : Type*} [MulAction Γ R] {Grip Piece : Type*}

namespace PieceModel

/-- The five labels of O7: the left cosets `G_P ⧸ H_P`, with `H_P = G_P ∩ K⁺`. -/
abbrev Label (M : PieceModel Γ R Grip Piece) (G : Subgroup Γ) : Type _ :=
  G ⧸ M.K.subgroupOf G

/-- Label `0`, the coset `H_P`. -/
def label0 (M : PieceModel Γ R Grip Piece) (G : Subgroup Γ) : M.Label G :=
  ((1 : G) : M.Label G)

/-- An element of `G_P` keeps label `0` exactly when it lies in `K⁺`. -/
lemma smul_label0_eq_iff (M : PieceModel Γ R Grip Piece) {G : Subgroup Γ} (g : G) :
    g • M.label0 G = M.label0 G ↔ (g : Γ) ∈ M.K := by
  rw [label0, MulAction.Quotient.smul_coe, smul_eq_mul, mul_one, QuotientGroup.eq, mul_one,
    Subgroup.inv_mem_iff, Subgroup.mem_subgroupOf]

/-- The label of `g` is `0` exactly when `g` lies in `K⁺`. -/
lemma coe_eq_label0_iff (M : PieceModel Γ R Grip Piece) {G : Subgroup Γ} (g : G) :
    ((g : M.Label G) = M.label0 G) ↔ (g : Γ) ∈ M.K := by
  rw [← M.smul_label0_eq_iff g, label0, MulAction.Quotient.smul_coe, smul_eq_mul, mul_one]

end PieceModel

/-- O7, the class-cell model of a class `P` of grips (LC-01). -/
structure ClassCellModel (M : PieceModel Γ R Grip Piece) (G : Subgroup Γ) (Cell : Type*)
    [MulAction G Cell] (Kind : Type*) where
  /-- The class of grips. -/
  P : Set Grip
  /-- The cell `C` lies inside the cap of the grip `c`. -/
  inCap : Grip → Cell → Prop
  /-- A menu element of `G_P` at a grip of `P` maps the cap onto itself. -/
  inCap_smul : ∀ c ∈ P, ∀ g : G, (g : Γ) ∈ M.menu c → ∀ C, inCap c (g • C) ↔ inCap c C
  /-- The kind of a cell: the number of caps of `P` that contain it. -/
  kind : Cell → Kind
  /-- `G_P` permutes the caps of `P`, so it keeps kinds. -/
  kind_smul : ∀ (g : G) (C : Cell), kind (g • C) = kind C
  /-- The cell of a home chamber. -/
  chamberCell : R → Cell
  /-- `H_P` maps a home chamber of `C` to a home chamber of `h • C`. -/
  chamberCell_smul : ∀ g : G, (g : Γ) ∈ M.K → ∀ x ∈ M.chamber,
    chamberCell ((g : Γ) • x) = g • chamberCell x

namespace ClassCellModel

variable {M : PieceModel Γ R Grip Piece} {G : Subgroup Γ} {Cell : Type*} [MulAction G Cell]
  {Kind : Type*}
variable (Q : ClassCellModel M G Cell Kind)

/-- The home chambers `Ω_C` of a cell. -/
def chambersOf (C : Cell) : Set R :=
  {x | x ∈ M.chamber ∧ Q.chamberCell x = C}

/-- A home cell: a cell that holds a home chamber. -/
def IsHome (C : Cell) : Prop :=
  ∃ x ∈ M.chamber, Q.chamberCell x = C

/-- `H_P` maps home cells to home cells. -/
lemma IsHome.smul {Q : ClassCellModel M G Cell Kind} {C : Cell} (hC : Q.IsHome C) {g : G}
    (hg : (g : Γ) ∈ M.K) : Q.IsHome (g • C) := by
  obtain ⟨x, hx, rfl⟩ := hC
  exact ⟨(g : Γ) • x, M.smul_chamber _ hg _ hx, Q.chamberCell_smul g hg x hx⟩

/-- A class-P letter: a grip of `P` and an element of `G_P` in its menu. -/
structure Letter where
  /-- The grip. -/
  grip : Grip
  /-- The rotation. -/
  elem : G
  grip_mem : grip ∈ Q.P
  mem : (elem : Γ) ∈ M.menu grip

variable {Q}

/-- The inverse class-P letter. -/
def Letter.inv (a : Q.Letter) : Q.Letter :=
  ⟨a.grip, a.elem⁻¹, a.grip_mem, by simpa using M.menu_inv _ _ a.mem⟩

/-- The letter of the piece model that a class-P letter is. -/
def Letter.toLetter (a : Q.Letter) : M.Letter :=
  ⟨a.grip, a.elem, a.mem⟩

@[simp] lemma Letter.inv_grip (a : Q.Letter) : a.inv.grip = a.grip := rfl

@[simp] lemma Letter.inv_elem (a : Q.Letter) : a.inv.elem = a.elem⁻¹ := rfl

@[simp] lemma Letter.inv_inv (a : Q.Letter) : a.inv.inv = a := by
  cases a
  simp [Letter.inv]

lemma Letter.inCap_smul (a : Q.Letter) (C : Cell) :
    Q.inCap a.grip (a.elem • C) ↔ Q.inCap a.grip C :=
  Q.inCap_smul _ a.grip_mem _ a.mem C

variable (Q)

/-- A class-P word as a word of the piece model. -/
def lift (w : List Q.Letter) : List M.Letter :=
  w.map Letter.toLetter

@[simp] lemma lift_append (u v : List Q.Letter) : Q.lift (u ++ v) = Q.lift u ++ Q.lift v := by
  simp [lift]

@[simp] lemma length_lift (w : List Q.Letter) : (Q.lift w).length = w.length := by
  simp [lift]

/-- The inverse class-P word. -/
def wordInv (w : List Q.Letter) : List Q.Letter :=
  (w.map Letter.inv).reverse

open Classical in
/-- The rotation that a letter gives the cell `C`. -/
noncomputable def cellMult (a : Q.Letter) (C : Cell) : G :=
  if Q.inCap a.grip C then a.elem else 1

/-- The cell after one letter. -/
noncomputable def cellStep (a : Q.Letter) (C : Cell) : Cell :=
  Q.cellMult a C • C

/-- The accumulated rotation `a_C(w)` of the cell `C` under the chronological word `w`. -/
noncomputable def cellAcc : List Q.Letter → Cell → G
  | [], _ => 1
  | a :: w, C => cellAcc w (Q.cellStep a C) * Q.cellMult a C

/-- The point map of a class-P word: `(C, ℓ) ↦ (a_C(w) • C, a_C(w) • ℓ)`. -/
noncomputable def pointMap (w : List Q.Letter) (q : Cell × M.Label G) : Cell × M.Label G :=
  (Q.cellAcc w q.1 • q.1, Q.cellAcc w q.1 • q.2)

section Dynamics

@[simp] lemma cellAcc_nil (C : Cell) : Q.cellAcc [] C = 1 := rfl

@[simp] lemma cellAcc_cons (a : Q.Letter) (w : List Q.Letter) (C : Cell) :
    Q.cellAcc (a :: w) C = Q.cellAcc w (Q.cellStep a C) * Q.cellMult a C := rfl

lemma cellAcc_append (u v : List Q.Letter) (C : Cell) :
    Q.cellAcc (u ++ v) C = Q.cellAcc v (Q.cellAcc u C • C) * Q.cellAcc u C := by
  induction u generalizing C with
  | nil => simp
  | cons a u ih =>
    simp only [List.cons_append, cellAcc_cons, ih, cellStep, mul_smul, mul_assoc]

lemma pointMap_append (u v : List Q.Letter) (q : Cell × M.Label G) :
    Q.pointMap (u ++ v) q = Q.pointMap v (Q.pointMap u q) := by
  simp [pointMap, cellAcc_append, mul_smul]

@[simp] lemma pointMap_nil (q : Cell × M.Label G) : Q.pointMap [] q = q := by
  simp [pointMap]

lemma cellStep_inv (a : Q.Letter) (C : Cell) : Q.cellStep a.inv (Q.cellStep a C) = C := by
  by_cases h : Q.inCap a.grip C
  · have h' : Q.inCap a.grip (a.elem • C) := (a.inCap_smul C).2 h
    simp [cellStep, cellMult, h, h', smul_smul]
  · simp [cellStep, cellMult, h]

lemma cellMult_inv (a : Q.Letter) (C : Cell) :
    Q.cellMult a.inv (Q.cellStep a C) = (Q.cellMult a C)⁻¹ := by
  by_cases h : Q.inCap a.grip C
  · have h' : Q.inCap a.grip (a.elem • C) := (a.inCap_smul C).2 h
    simp [cellStep, cellMult, h, h']
  · simp [cellStep, cellMult, h]

lemma wordInv_cons (a : Q.Letter) (w : List Q.Letter) :
    Q.wordInv (a :: w) = Q.wordInv w ++ [a.inv] := by
  simp [wordInv]

@[simp] lemma wordInv_wordInv (w : List Q.Letter) : Q.wordInv (Q.wordInv w) = w := by
  simp [wordInv, List.map_reverse, Function.comp_def]

lemma pointMap_wordInv (w : List Q.Letter) (q : Cell × M.Label G) :
    Q.pointMap (Q.wordInv w) (Q.pointMap w q) = q := by
  induction w generalizing q with
  | nil => simp [wordInv]
  | cons a w ih =>
    have hsplit := Q.pointMap_append [a] w q
    simp only [List.singleton_append] at hsplit
    rw [wordInv_cons, pointMap_append, hsplit, ih]
    obtain ⟨C, ℓ⟩ := q
    simp only [pointMap, cellAcc_cons, cellAcc_nil, one_mul, Prod.mk.injEq]
    have h1 := Q.cellStep_inv a C
    have h2 := Q.cellMult_inv a C
    simp only [cellStep] at h1 h2
    refine ⟨h1, ?_⟩
    rw [h2, smul_smul, inv_mul_cancel, one_smul]

/-- The point map of a class-P word is a permutation of the points (R22A-05, item 1). -/
noncomputable def pointPerm (w : List Q.Letter) : Equiv.Perm (Cell × M.Label G) where
  toFun := Q.pointMap w
  invFun := Q.pointMap (Q.wordInv w)
  left_inv := Q.pointMap_wordInv w
  right_inv q := by simpa using Q.pointMap_wordInv (Q.wordInv w) q

@[simp] lemma pointPerm_apply (w : List Q.Letter) (q : Cell × M.Label G) :
    Q.pointPerm w q = (Q.cellAcc w q.1 • q.1, Q.cellAcc w q.1 • q.2) := rfl

lemma pointPerm_append (u v : List Q.Letter) :
    Q.pointPerm (u ++ v) = Q.pointPerm v * Q.pointPerm u := by
  ext q : 1
  exact Q.pointMap_append u v q

@[simp] lemma pointPerm_nil : Q.pointPerm [] = 1 := by
  ext q : 1
  simp

/-- A cell inside no cap of `P` (the outer cell) is never moved. -/
lemma cellAcc_eq_one_of_not_inCap {C : Cell} (hC : ∀ c ∈ Q.P, ¬ Q.inCap c C)
    (w : List Q.Letter) : Q.cellAcc w C = 1 := by
  induction w with
  | nil => rfl
  | cons a w ih =>
    have h := hC a.grip a.grip_mem
    simp [cellStep, cellMult, h, ih]

/-- A class-P word fixes the five points of a cell exactly when its accumulated rotation fixes
the cell and its labels. -/
lemma fixes_fibre_iff (w : List Q.Letter) (C : Cell) :
    (∀ ℓ : M.Label G, Q.pointPerm w (C, ℓ) = (C, ℓ)) ↔
      Q.cellAcc w C • C = C ∧ ∀ ℓ : M.Label G, Q.cellAcc w C • ℓ = ℓ := by
  simp only [pointPerm_apply, Prod.mk.injEq, forall_and]
  constructor
  · rintro ⟨h1, h2⟩
    exact ⟨h1 (M.label0 G), h2⟩
  · rintro ⟨h1, h2⟩
    exact ⟨fun _ => h1, h2⟩

/-- The commutator `[x, y] = x·y·x⁻¹·y⁻¹` of two class-P letters, chronologically. -/
def comm (x y : Q.Letter) : List Q.Letter :=
  [x, y, x.inv, y.inv]

@[simp] lemma length_comm (x y : Q.Letter) : (Q.comm x y).length = 4 := rfl

/-- The word `w` repeated `n` times. -/
def wpow (w : List Q.Letter) : ℕ → List Q.Letter
  | 0 => []
  | n + 1 => wpow w n ++ w

@[simp] lemma length_wpow (w : List Q.Letter) (n : ℕ) : (Q.wpow w n).length = n * w.length := by
  induction n with
  | zero => simp [wpow]
  | succ n ih => simp [wpow, ih, Nat.succ_mul]

lemma pointPerm_wpow (w : List Q.Letter) (n : ℕ) :
    Q.pointPerm (Q.wpow w n) = Q.pointPerm w ^ n := by
  induction n with
  | zero => simp [wpow]
  | succ n ih => rw [wpow, pointPerm_append, ih, pow_succ']

lemma mem_wpow {w : List Q.Letter} {n : ℕ} {a : Q.Letter} (h : a ∈ Q.wpow w n) : a ∈ w := by
  induction n with
  | zero => simp [wpow] at h
  | succ n ih =>
    rw [wpow, List.mem_append] at h
    exact h.elim ih id

lemma mem_comm {x y a : Q.Letter} (h : a ∈ Q.comm x y) :
    a.grip = x.grip ∨ a.grip = y.grip := by
  simp only [comm, List.mem_cons, List.not_mem_nil, or_false] at h
  rcases h with rfl | rfl | rfl | rfl <;> simp

end Dynamics

/-! ### LC-03 (O8) -/

/-- O8, LC-03, at the state `Z` (an earlier-stage result, stated as a hypothesis). The words are
the class-P words whose letters are at the free grips `free` of `Z`. -/
structure LC03 (Z : Piece → Γ) where
  /-- The free grips of `P` at `Z` (O7). -/
  free : Set Grip
  /-- `Own p C`: `p` is a class-P off piece of `Z` lying in the cell `C`. -/
  Own : Piece → Cell → Prop
  /-- The pieces of a foreign frame. -/
  Foreign : Piece → Prop
  /-- Modelling reading of O7: every off piece of `Z` is foreign or a class-P off piece in some
  cell. -/
  off_cases : ∀ p, Z p ∉ M.K → Foreign p ∨ ∃ C, Own p C
  /-- Foreign pieces lie outside every free cap. -/
  foreign_outside : ∀ p, Foreign p → ∀ c ∈ free, M.side c (M.region Z p) = .outside
  /-- Every prefix of a free class-P word is admissible. -/
  admissible : ∀ w : List Q.Letter, (∀ a ∈ w, a.grip ∈ free) → M.Admissible Z (Q.lift w)
  /-- Every class-P off piece of the cell `C` receives the rotation `a_C(w)`. -/
  own_apply : ∀ w : List Q.Letter, (∀ a ∈ w, a.grip ∈ free) → ∀ p C, Own p C →
    M.apply (Q.lift w) Z p = (Q.cellAcc w C : Γ) * Z p
  /-- No ghost of a home chamber straddles a cut of `P`. -/
  ghost_clear : ∀ w : List Q.Letter, (∀ a ∈ w, a.grip ∈ free) → ∀ x ∈ M.chamber,
    M.Clear (Q.lift w) x
  /-- The ghost of a home chamber of the cell `C` receives the rotation `a_C(w)`. -/
  ghost_acc : ∀ w : List Q.Letter, (∀ a ∈ w, a.grip ∈ free) → ∀ x ∈ M.chamber,
    M.acc (Q.lift w) x = Q.cellAcc w (Q.chamberCell x)
  /-- A rotation of `G_P` that fixes a nonempty cell and all its labels is the identity. -/
  rigid : ∀ C, (∃ c ∈ Q.P, Q.inCap c C) → ∀ g : G, g • C = C →
    (∀ ℓ : M.Label G, g • ℓ = ℓ) → g = 1

variable {Q} {Z : Piece → Γ}

/-- A free class-P word at `Z`. -/
def LC03.FreeWord (B : Q.LC03 Z) (w : List Q.Letter) : Prop :=
  ∀ a ∈ w, a.grip ∈ B.free

/-- R22A-02: pieces of a foreign frame never move under free class-P words (they lie outside
every free cap). -/
lemma LC03.foreign_apply (B : Q.LC03 Z) {w : List Q.Letter} (hw : B.FreeWord w) {p : Piece}
    (hp : B.Foreign p) : M.apply (Q.lift w) Z p = Z p := by
  have hout : ∀ a ∈ Q.lift w, M.side a.grip (M.region Z p) = .outside := by
    intro a ha
    obtain ⟨b, hb, rfl⟩ := List.mem_map.1 ha
    exact B.foreign_outside p hp _ (hw b hb)
  simp [PieceModel.apply, (M.acc_eq_one_of_forall_outside hout).2]

/-- A free class-P word that fixes the five points of a cell gives it the rotation `1` (by
`rigid` for a nonempty cell; an outer cell is never moved). -/
lemma LC03.cellAcc_eq_one (B : Q.LC03 Z) (w : List Q.Letter) {C : Cell}
    (h : ∀ ℓ : M.Label G, Q.pointPerm w (C, ℓ) = (C, ℓ)) : Q.cellAcc w C = 1 := by
  by_cases hC : ∃ c ∈ Q.P, Q.inCap c C
  · obtain ⟨h1, h2⟩ := (Q.fixes_fibre_iff w C).1 h
    exact B.rigid C hC _ h1 h2
  · simp only [not_exists, not_and] at hC
    exact Q.cellAcc_eq_one_of_not_inCap hC w

end ClassCellModel

end Jumbling
