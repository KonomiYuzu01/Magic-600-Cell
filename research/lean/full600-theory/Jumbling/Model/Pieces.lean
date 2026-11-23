import Mathlib.Algebra.Group.Subgroup.Basic
import Mathlib.Algebra.Group.Action.Basic
import Mathlib.Data.List.Basic

set_option linter.style.header false in
/-!
# Pieces, letters, words and steps (objects O1 to O6)

Source: `research/jumbling/restoration/12-low-height/PROOFS-B.md`, section 1 (objects O1 to O6).

The puzzle is replaced by an abstract model whose fields are the structural facts the proofs of
round 22 use. `PieceModel Γ R Grip Piece` has

* a group `Γ` of poses with the subgroup `K` (the rotation group `K⁺` of the 600-cell);
* a type `R` of regions on which `Γ` acts, and grips with `side c r`, which says whether the
  region `r` lies inside the cap of the grip `c`, outside it, or straddles its cut;
* a menu `menu c` at each grip, closed under inverses, whose elements keep the side of every
  region (a menu element maps its cap onto itself);
* the home chambers `Ω = chamber`: no chamber straddles a cut, and `K` maps chambers to chambers
  (so a lattice piece never straddles a cut, O2);
* the pieces, each with a home chamber.

A state assigns a pose to every piece; the region of a piece is its pose applied to its home
chamber. A letter `(c, e)` multiplies on the left the pose of every piece whose region lies inside
the cap of `c`, and it is admissible when no piece straddles the cut of `c` (O2). Words are
chronological lists: the first letter acts first.

Every piece moves on its own: its trajectory depends only on its region and its pose. So the
model is written region by region. `acc w r` is the accumulated left factor that a word `w`
applies to an object starting in region `r`, `endRegion w r` its final region and `Clear w r`
says that it straddles no cut of a letter of `w` at the prefix where that letter acts. A ghost of
the chamber `x` (O6) is the object at `x` with pose `1`: it ends at pose `acc w x`.

Main definitions:

* `PieceModel.apply`, `PieceModel.Admissible` (every prefix admissible, `admissible_cons`).
* `PieceModel.wordInv`: the inverse word, reversed with inverse letters; `apply_wordInv_apply`
  shows that its pose map is the inverse of the word's.
* `PieceModel.Bad`, `PieceModel.holes`, `PieceModel.occupied`, `PieceModel.collateral` (O1, O4,
  O6) and `PieceModel.IsStrictPrime`, `PieceModel.IsStrict` (O4).
* `PieceModel.SafeWord` (O5): letters with poses in `K` at grips whose cut no piece straddles and
  whose cap holds no off piece.

Main results:

* `PieceModel.collateral_eq` and `PieceModel.collateral_eq_empty_iff`: R22A-01, item 2, for every
  admissible word (the class-P version is in `Jumbling.Model.Transport`).
-/

namespace Jumbling

/-- Where a region lies relative to the cut of a grip (O2). -/
inductive Side
  | inside
  | outside
  | straddle
  deriving DecidableEq

/-- The piece model (O1, O2). `Γ` is the group of poses, `R` the type of regions, `Grip` the
grips and `Piece` the pieces. -/
structure PieceModel (Γ : Type*) [Group Γ] (R : Type*) [MulAction Γ R] (Grip Piece : Type*) where
  /-- The rotation group `K⁺` of the 600-cell: a piece is on the lattice when its pose is in `K`. -/
  K : Subgroup Γ
  /-- Whether a region lies inside the cap of a grip, outside it, or straddles its cut. -/
  side : Grip → R → Side
  /-- The menu at a grip. -/
  menu : Grip → Set Γ
  /-- A menu element maps its cap onto itself: it keeps the side of every region. -/
  menu_side : ∀ c, ∀ e ∈ menu c, ∀ r, side c (e • r) = side c r
  /-- Menus are closed under inverses. -/
  menu_inv : ∀ c, ∀ e ∈ menu c, e⁻¹ ∈ menu c
  /-- The home chambers `Ω`. -/
  chamber : Set R
  /-- No chamber straddles a cut. -/
  chamber_side : ∀ x ∈ chamber, ∀ c, side c x ≠ .straddle
  /-- `K` maps chambers to chambers. -/
  smul_chamber : ∀ k ∈ K, ∀ x ∈ chamber, k • x ∈ chamber
  /-- The home chamber of a piece. -/
  home : Piece → R
  home_mem : ∀ p, home p ∈ chamber

namespace PieceModel

variable {Γ : Type*} [Group Γ] {R : Type*} [MulAction Γ R] {Grip Piece : Type*}
variable (M : PieceModel Γ R Grip Piece)

/-- A letter: a grip and an element of its menu (O2). -/
structure Letter where
  /-- The grip. -/
  grip : Grip
  /-- The menu element. -/
  elem : Γ
  mem : elem ∈ M.menu grip

variable {M}

/-- The inverse letter: the same grip with the inverse menu element. -/
def Letter.inv (a : M.Letter) : M.Letter :=
  ⟨a.grip, a.elem⁻¹, M.menu_inv _ _ a.mem⟩

@[simp] lemma Letter.inv_grip (a : M.Letter) : a.inv.grip = a.grip := rfl

@[simp] lemma Letter.inv_elem (a : M.Letter) : a.inv.elem = a.elem⁻¹ := rfl

@[simp] lemma Letter.inv_inv (a : M.Letter) : a.inv.inv = a := by
  cases a
  simp [Letter.inv]

variable (M)

/-- The left factor that the letter `a` applies to an object in region `r`. -/
def mult (a : M.Letter) (r : R) : Γ :=
  if M.side a.grip r = .inside then a.elem else 1

/-- The region of an object in region `r` after the letter `a`. -/
def step (a : M.Letter) (r : R) : R :=
  M.mult a r • r

/-- The accumulated left factor that a chronological word applies to an object starting in
region `r`. -/
def acc : List M.Letter → R → Γ
  | [], _ => 1
  | a :: w, r => acc w (M.step a r) * M.mult a r

/-- The final region of an object starting in region `r`. -/
def endRegion (w : List M.Letter) (r : R) : R :=
  M.acc w r • r

/-- An object starting in region `r` straddles no cut of a letter of `w` at the prefix where that
letter acts. -/
def Clear : List M.Letter → R → Prop
  | [], _ => True
  | a :: w, r => M.side a.grip r ≠ .straddle ∧ Clear w (M.step a r)

/-- The inverse word: reversed, with inverse letters. -/
def wordInv (w : List M.Letter) : List M.Letter :=
  (w.map Letter.inv).reverse

/-- The region of the piece `p` at the state `s`. -/
def region (s : Piece → Γ) (p : Piece) : R :=
  s p • M.home p

/-- The state after the word `w`. -/
def apply (w : List M.Letter) (s : Piece → Γ) : Piece → Γ :=
  fun p => M.acc w (M.region s p) * s p

/-- A letter is admissible at `s` when no piece straddles the cut of its grip (O2). -/
def LetterAdmissible (s : Piece → Γ) (a : M.Letter) : Prop :=
  ∀ p, M.side a.grip (M.region s p) ≠ .straddle

/-- A word is admissible at `s` when every prefix is: no piece straddles the cut of a letter
when that letter acts (`admissible_cons`). -/
def Admissible (s : Piece → Γ) (w : List M.Letter) : Prop :=
  ∀ p, M.Clear w (M.region s p)

section Basic

variable {a : M.Letter} {u v w : List M.Letter} {r x : R} {s : Piece → Γ}

@[simp] lemma acc_nil (r : R) : M.acc [] r = 1 := rfl

@[simp] lemma acc_cons (a : M.Letter) (w : List M.Letter) (r : R) :
    M.acc (a :: w) r = M.acc w (M.step a r) * M.mult a r := rfl

@[simp] lemma clear_nil (r : R) : M.Clear [] r := trivial

@[simp] lemma clear_cons (a : M.Letter) (w : List M.Letter) (r : R) :
    M.Clear (a :: w) r ↔ M.side a.grip r ≠ .straddle ∧ M.Clear w (M.step a r) := Iff.rfl

@[simp] lemma endRegion_nil (r : R) : M.endRegion [] r = r := by
  simp [endRegion]

lemma endRegion_cons (a : M.Letter) (w : List M.Letter) (r : R) :
    M.endRegion (a :: w) r = M.endRegion w (M.step a r) := by
  simp [endRegion, step, mul_smul]

lemma endRegion_singleton (a : M.Letter) (r : R) : M.endRegion [a] r = M.step a r := by
  simp [endRegion_cons]

lemma acc_append (u v : List M.Letter) (r : R) :
    M.acc (u ++ v) r = M.acc v (M.endRegion u r) * M.acc u r := by
  induction u generalizing r with
  | nil => simp
  | cons a u ih => simp [ih, endRegion_cons, mul_assoc]

lemma endRegion_append (u v : List M.Letter) (r : R) :
    M.endRegion (u ++ v) r = M.endRegion v (M.endRegion u r) := by
  simp only [endRegion, acc_append, mul_smul]

lemma clear_append (u v : List M.Letter) (r : R) :
    M.Clear (u ++ v) r ↔ M.Clear u r ∧ M.Clear v (M.endRegion u r) := by
  induction u generalizing r with
  | nil => simp
  | cons a u ih => simp [ih, endRegion_cons, and_assoc]

lemma region_apply (w : List M.Letter) (s : Piece → Γ) (p : Piece) :
    M.region (M.apply w s) p = M.endRegion w (M.region s p) := by
  simp [region, apply, endRegion, mul_smul]

@[simp] lemma apply_nil (s : Piece → Γ) : M.apply [] s = s := by
  funext p
  simp [apply]

lemma apply_append (u v : List M.Letter) (s : Piece → Γ) :
    M.apply (u ++ v) s = M.apply v (M.apply u s) := by
  funext p
  simp only [apply, acc_append, region_apply, mul_assoc]

lemma admissible_append (s : Piece → Γ) (u v : List M.Letter) :
    M.Admissible s (u ++ v) ↔ M.Admissible s u ∧ M.Admissible (M.apply u s) v := by
  simp only [Admissible, clear_append, region_apply, forall_and]

/-- The admissibility of a word is the admissibility of every prefix's next letter (O2). -/
lemma admissible_cons (s : Piece → Γ) (a : M.Letter) (w : List M.Letter) :
    M.Admissible s (a :: w) ↔ M.LetterAdmissible s a ∧ M.Admissible (M.apply [a] s) w := by
  simp only [Admissible, LetterAdmissible, clear_cons, region_apply, endRegion_singleton,
    forall_and]

lemma admissible_singleton (s : Piece → Γ) (a : M.Letter) :
    M.Admissible s [a] ↔ M.LetterAdmissible s a := by
  rw [admissible_cons]
  exact and_iff_left fun _ => trivial

/-- A letter keeps the side of every region relative to its own cut. -/
lemma side_step (a : M.Letter) (r : R) : M.side a.grip (M.step a r) = M.side a.grip r := by
  unfold step mult
  split_ifs
  · exact M.menu_side _ _ a.mem r
  · rw [one_smul]

lemma step_inv_step (a : M.Letter) (r : R) : M.step a.inv (M.step a r) = r := by
  by_cases h : M.side a.grip r = .inside
  · have h' : M.side a.grip (a.elem • r) = .inside := by rw [M.menu_side _ _ a.mem, h]
    simp [step, mult, h, h', smul_smul]
  · simp [step, mult, h]

lemma mult_inv_step (a : M.Letter) (r : R) : M.mult a.inv (M.step a r) = (M.mult a r)⁻¹ := by
  by_cases h : M.side a.grip r = .inside
  · have h' : M.side a.grip (a.elem • r) = .inside := by rw [M.menu_side _ _ a.mem, h]
    simp [step, mult, h, h']
  · simp [step, mult, h]

@[simp] lemma wordInv_nil : M.wordInv [] = [] := rfl

lemma wordInv_cons (a : M.Letter) (w : List M.Letter) :
    M.wordInv (a :: w) = M.wordInv w ++ [a.inv] := by
  simp [wordInv]

lemma wordInv_append (u v : List M.Letter) : M.wordInv (u ++ v) = M.wordInv v ++ M.wordInv u := by
  simp [wordInv]

@[simp] lemma wordInv_wordInv (w : List M.Letter) : M.wordInv (M.wordInv w) = w := by
  simp [wordInv, List.map_reverse, Function.comp_def]

@[simp] lemma length_wordInv (w : List M.Letter) : (M.wordInv w).length = w.length := by
  simp [wordInv]

lemma mem_wordInv {b : M.Letter} : b ∈ M.wordInv w ↔ ∃ a ∈ w, a.inv = b := by
  simp [wordInv]

/-- The inverse word takes the final region back to the start. -/
lemma endRegion_wordInv (w : List M.Letter) (r : R) :
    M.endRegion (M.wordInv w) (M.endRegion w r) = r := by
  induction w generalizing r with
  | nil => simp
  | cons a w ih =>
    rw [wordInv_cons, endRegion_append, M.endRegion_cons a w r, ih, endRegion_singleton,
      step_inv_step]

/-- The inverse word undoes the accumulated factor. -/
lemma acc_wordInv (w : List M.Letter) (r : R) :
    M.acc (M.wordInv w) (M.endRegion w r) = (M.acc w r)⁻¹ := by
  induction w generalizing r with
  | nil => simp
  | cons a w ih =>
    rw [wordInv_cons, acc_append, M.endRegion_cons a w r, ih, endRegion_wordInv]
    simp [mult_inv_step, mul_inv_rev]

/-- An object that straddles no cut along `w` straddles none along the inverse word from its
final region. -/
lemma clear_wordInv (h : M.Clear w r) : M.Clear (M.wordInv w) (M.endRegion w r) := by
  induction w generalizing r with
  | nil => trivial
  | cons a w ih =>
    rw [clear_cons] at h
    rw [wordInv_cons, clear_append, endRegion_cons]
    refine ⟨ih h.2, ?_⟩
    rw [endRegion_wordInv]
    simpa [side_step] using h.1

/-- R22D-01, item 1 of the proof: the pose map of the inverse word is the inverse of the pose map
of the word. -/
lemma apply_wordInv_apply (w : List M.Letter) (s : Piece → Γ) :
    M.apply (M.wordInv w) (M.apply w s) = s := by
  funext p
  simp only [apply, region_apply, acc_wordInv, inv_mul_cancel_left]

lemma apply_apply_wordInv (w : List M.Letter) (s : Piece → Γ) :
    M.apply w (M.apply (M.wordInv w) s) = s := by
  simpa using M.apply_wordInv_apply (M.wordInv w) s

/-- The final pose of a piece depends only on its pose at the start. -/
lemma apply_eq_apply_of_eq {s s' : Piece → Γ} {p : Piece} (h : s p = s' p)
    (w : List M.Letter) : M.apply w s p = M.apply w s' p := by
  simp [apply, region, h]

/-- The pose of a single piece under the inverse word determines its pose before it. -/
lemma apply_wordInv_eq_iff (w : List M.Letter) (s t : Piece → Γ) (p : Piece) :
    M.apply (M.wordInv w) t p = s p ↔ t p = M.apply w s p := by
  constructor
  · intro h
    have := M.apply_eq_apply_of_eq (s := M.apply (M.wordInv w) t) (s' := s) h w
    rw [← this, apply_apply_wordInv]
  · intro h
    have := M.apply_eq_apply_of_eq (s := t) (s' := M.apply w s) h (M.wordInv w)
    rw [this, apply_wordInv_apply]

lemma acc_mem {S : Subgroup Γ} (hw : ∀ a ∈ w, a.elem ∈ S) (r : R) : M.acc w r ∈ S := by
  induction w generalizing r with
  | nil => exact S.one_mem
  | cons a w ih =>
    refine S.mul_mem (ih (fun b hb => hw b (List.mem_cons_of_mem a hb)) _) ?_
    unfold mult
    split_ifs
    · exact hw a List.mem_cons_self
    · exact S.one_mem

lemma region_mem_chamber {p : Piece} (h : s p ∈ M.K) : M.region s p ∈ M.chamber :=
  M.smul_chamber _ h _ (M.home_mem p)

/-- A word of letters with poses in `K` keeps chambers among chambers. -/
lemma endRegion_mem_chamber (hw : ∀ a ∈ w, a.elem ∈ M.K) (hx : x ∈ M.chamber) :
    M.endRegion w x ∈ M.chamber :=
  M.smul_chamber _ (M.acc_mem hw x) _ hx

/-- An object in a chamber never straddles a cut along a word of letters with poses in `K`
(O2: a lattice piece never straddles a cut). -/
lemma clear_of_mem_chamber (hw : ∀ a ∈ w, a.elem ∈ M.K) (hx : x ∈ M.chamber) :
    M.Clear w x := by
  induction w generalizing x with
  | nil => trivial
  | cons a w ih =>
    refine ⟨M.chamber_side x hx a.grip, ih (fun b hb => hw b (List.mem_cons_of_mem a hb)) ?_⟩
    simpa [endRegion_singleton] using
      M.endRegion_mem_chamber (w := [a]) (by simpa using hw a List.mem_cons_self) hx

/-- An object outside the cap of every letter of `w` never moves. -/
lemma acc_eq_one_of_forall_outside (h : ∀ a ∈ w, M.side a.grip r = .outside) :
    M.Clear w r ∧ M.acc w r = 1 := by
  induction w with
  | nil => simp
  | cons a w ih =>
    have ha := h a List.mem_cons_self
    have hstep : M.step a r = r := by simp [step, mult, ha]
    obtain ⟨h1, h2⟩ := ih (fun b hb => h b (List.mem_cons_of_mem a hb))
    refine ⟨⟨by simp [ha], by rwa [hstep]⟩, ?_⟩
    simp [hstep, h2, mult, ha]

end Basic

/-! ### Ghosts, holes and steps (O1, O4, O6) -/

/-- O6: `Bad w` is the set of home chambers whose ghost (an object at the chamber with pose `1`)
straddles a cut at some prefix of `w` or ends off the lattice. -/
def Bad (w : List M.Letter) : Set R :=
  {x | x ∈ M.chamber ∧ (¬ M.Clear w x ∨ M.acc w x ∉ M.K)}

/-- O1: the holes of `s`, the home chambers that no lattice piece occupies. -/
def holes (s : Piece → Γ) : Set R :=
  {x | x ∈ M.chamber ∧ ∀ p, s p ∈ M.K → M.region s p ≠ x}

/-- The chambers occupied by lattice pieces at `s`. -/
def occupied (s : Piece → Γ) : Set R :=
  {x | ∃ p, s p ∈ M.K ∧ M.region s p = x}

/-- O4: the endpoint collateral `C_Z(w)`, the lattice pieces of `Z` that are off after `w`. -/
def collateral (s : Piece → Γ) (w : List M.Letter) : Set Piece :=
  {p | s p ∈ M.K ∧ M.apply w s p ∉ M.K}

/-- O4: `w` is a strict′ step for the unit `U` at `s`: it is admissible, and after it every piece
of `U` and every lattice piece of `s` is on the lattice and every other off piece of `s` is on the
lattice or at its exact pose of `s`. -/
structure IsStrictPrime (s : Piece → Γ) (U : Set Piece) (w : List M.Letter) : Prop where
  admissible : M.Admissible s w
  lands : ∀ p ∈ U, M.apply w s p ∈ M.K
  keepsLattice : ∀ p, s p ∈ M.K → M.apply w s p ∈ M.K
  others : ∀ p ∉ U, s p ∉ M.K → M.apply w s p ∈ M.K ∨ M.apply w s p = s p

/-- O4: `w` is a strict step for `U` at `s`: a strict′ step after which every other off piece of
`s` keeps its exact pose. -/
structure IsStrict (s : Piece → Γ) (U : Set Piece) (w : List M.Letter) : Prop where
  admissible : M.Admissible s w
  lands : ∀ p ∈ U, M.apply w s p ∈ M.K
  keepsLattice : ∀ p, s p ∈ M.K → M.apply w s p ∈ M.K
  others : ∀ p ∉ U, s p ∉ M.K → M.apply w s p = s p

lemma IsStrict.isStrictPrime {M : PieceModel Γ R Grip Piece} {s : Piece → Γ} {U : Set Piece}
    {w : List M.Letter} (h : M.IsStrict s U w) : M.IsStrictPrime s U w :=
  ⟨h.admissible, h.lands, h.keepsLattice, fun p hp hoff => Or.inr (h.others p hp hoff)⟩

section Holes

variable {s : Piece → Γ} {w : List M.Letter}

lemma occupied_subset_chamber (s : Piece → Γ) : M.occupied s ⊆ M.chamber := by
  rintro x ⟨p, hp, rfl⟩
  exact M.region_mem_chamber hp

/-- The chambers that are not holes are the occupied chambers (the set `L` of R22A2-02). -/
lemma chamber_diff_holes (s : Piece → Γ) : M.chamber \ M.holes s = M.occupied s := by
  ext x
  simp only [holes, occupied, Set.mem_sdiff, Set.mem_ofPred_eq, not_and, not_forall,
    not_not]
  constructor
  · rintro ⟨hx, h⟩
    obtain ⟨p, hp, rfl⟩ := h hx
    exact ⟨p, hp, rfl⟩
  · rintro ⟨p, hp, rfl⟩
    exact ⟨M.region_mem_chamber hp, fun _ => ⟨p, hp, rfl⟩⟩

lemma subset_holes_iff {T : Set R} (hT : T ⊆ M.chamber) :
    T ⊆ M.holes s ↔ Disjoint T (M.occupied s) := by
  rw [Set.disjoint_right]
  constructor
  · rintro h x ⟨p, hp, rfl⟩ hx
    exact (h hx).2 p hp rfl
  · intro h x hx
    exact ⟨hT hx, fun p hp hpx => h ⟨p, hp, hpx⟩ (hpx ▸ hx)⟩

lemma Bad_subset_chamber (w : List M.Letter) : M.Bad w ⊆ M.chamber := fun _ hx => hx.1

lemma holes_subset_chamber (s : Piece → Γ) : M.holes s ⊆ M.chamber := fun _ hx => hx.1

/-- A lattice piece follows the ghost of its chamber: it ends on the lattice exactly when the
ghost's pose does. -/
lemma apply_mem_K_iff {p : Piece} (hp : s p ∈ M.K) :
    M.apply w s p ∈ M.K ↔ M.acc w (M.region s p) ∈ M.K :=
  M.K.mul_mem_cancel_right hp

/-- R22A-01, item 2, for every admissible word: `C_Z(w)` is the set of lattice pieces of `Z` at
chambers of `Bad(w)`. -/
theorem collateral_eq (hw : M.Admissible s w) :
    M.collateral s w = {p | s p ∈ M.K ∧ M.region s p ∈ M.Bad w} := by
  ext p
  simp only [collateral, Bad, Set.mem_ofPred_eq]
  constructor
  · rintro ⟨hp, hoff⟩
    exact ⟨hp, M.region_mem_chamber hp, Or.inr fun h => hoff ((M.apply_mem_K_iff hp).2 h)⟩
  · rintro ⟨hp, -, hbad⟩
    refine ⟨hp, fun h => ?_⟩
    rcases hbad with hbad | hbad
    · exact hbad (hw p)
    · exact hbad ((M.apply_mem_K_iff hp).1 h)

/-- R22A-01, item 2, for every admissible word: `C_Z(w) = ∅` exactly when
`Bad(w) ⊆ holes(Z)`. -/
theorem collateral_eq_empty_iff (hw : M.Admissible s w) :
    M.collateral s w = ∅ ↔ M.Bad w ⊆ M.holes s := by
  rw [M.collateral_eq hw, M.subset_holes_iff (M.Bad_subset_chamber w), Set.disjoint_right,
    Set.eq_empty_iff_forall_notMem]
  constructor
  · rintro h x ⟨p, hp, rfl⟩ hx
    exact h p ⟨hp, hx⟩
  · rintro h p ⟨hp, hx⟩
    exact h ⟨p, hp, rfl⟩ hx

end Holes

/-! ### SAFE-ALL words (O5) -/

/-- O5: a grip of `A(s)`: no piece straddles its cut and its cap holds no off piece. -/
def SafeGrip (s : Piece → Γ) (c : Grip) : Prop :=
  (∀ p, M.side c (M.region s p) ≠ .straddle) ∧
    (∀ p, s p ∉ M.K → M.side c (M.region s p) ≠ .inside)

/-- O5: a SAFE-ALL word at `s`: every letter has its pose in `K` and its grip in `A(s)`. -/
def SafeWord (s : Piece → Γ) (w : List M.Letter) : Prop :=
  ∀ a ∈ w, a.elem ∈ M.K ∧ M.SafeGrip s a.grip

section Safe

variable {M} {s t : Piece → Γ} {w : List M.Letter}

lemma SafeWord.wordInv (h : M.SafeWord s w) : M.SafeWord s (M.wordInv w) := by
  intro b hb
  obtain ⟨a, ha, rfl⟩ := M.mem_wordInv.1 hb
  exact ⟨M.K.inv_mem (h a ha).1, (h a ha).2⟩

/-- A SAFE-ALL word stays SAFE-ALL at a state whose off pieces are off pieces of `s` at their
poses of `s`. -/
lemma SafeWord.mono (h : M.SafeWord s w) (hst : ∀ p, t p ∉ M.K → t p = s p) :
    M.SafeWord t w := by
  intro a ha
  refine ⟨(h a ha).1, fun p => ?_, fun p hp => ?_⟩
  · by_cases hp : t p ∈ M.K
    · exact M.chamber_side _ (M.region_mem_chamber hp) _
    · have : M.region t p = M.region s p := by simp [region, hst p hp]
      rw [this]
      exact (h a ha).2.1 p
  · have hsp : s p ∉ M.K := by rwa [← hst p hp]
    have : M.region t p = M.region s p := by simp [region, hst p hp]
    rw [this]
    exact (h a ha).2.2 p hsp

lemma SafeWord.retained (h : M.SafeWord s w) : ∀ a ∈ w, a.elem ∈ M.K := fun a ha => (h a ha).1

/-- O5: a SAFE-ALL word fixes every off piece. -/
lemma SafeWord.apply_off (h : M.SafeWord s w) {p : Piece} (hp : s p ∉ M.K) :
    M.Clear w (M.region s p) ∧ M.apply w s p = s p := by
  have hout : ∀ a ∈ w, M.side a.grip (M.region s p) = .outside := by
    intro a ha
    have h1 := (h a ha).2.1 p
    have h2 := (h a ha).2.2 p hp
    cases hside : M.side a.grip (M.region s p) <;> simp_all
  obtain ⟨hc, ha⟩ := M.acc_eq_one_of_forall_outside hout
  exact ⟨hc, by simp [apply, ha]⟩

/-- O5: a SAFE-ALL word keeps every lattice piece on the lattice. -/
lemma SafeWord.apply_lattice (h : M.SafeWord s w) {p : Piece} (hp : s p ∈ M.K) :
    M.Clear w (M.region s p) ∧ M.apply w s p ∈ M.K :=
  ⟨M.clear_of_mem_chamber h.retained (M.region_mem_chamber hp),
    M.K.mul_mem (M.acc_mem h.retained _) hp⟩

/-- O5: a SAFE-ALL word is admissible at every prefix. -/
lemma SafeWord.admissible (h : M.SafeWord s w) : M.Admissible s w := by
  intro p
  by_cases hp : s p ∈ M.K
  · exact (h.apply_lattice hp).1
  · exact (h.apply_off hp).1

/-- A word of letters with poses in `K` keeps lattice membership, both ways. -/
lemma apply_mem_K_iff_of_retained (hw : ∀ a ∈ w, a.elem ∈ M.K) (s : Piece → Γ) (p : Piece) :
    M.apply w s p ∈ M.K ↔ s p ∈ M.K :=
  M.K.mul_mem_cancel_left (M.acc_mem hw _)

end Safe

end PieceModel

end Jumbling
