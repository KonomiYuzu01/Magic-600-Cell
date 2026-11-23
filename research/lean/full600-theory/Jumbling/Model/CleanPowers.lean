import Jumbling.Model.ClassCell
import Mathlib.Algebra.GCDMonoid.Finset
import Mathlib.Algebra.GCDMonoid.Nat
import Mathlib.Dynamics.PeriodicPts.Lemmas
import Mathlib.GroupTheory.OrderOfElement
import Mathlib.Logic.Relation
import Mathlib.SetTheory.Cardinal.Finite

set_option linter.style.header false in
/-!
# Clean powers: bounds and completeness (R22A-04)

Source: `research/jumbling/restoration/12-low-height/PROOFS-B.md`, R22A-03 (step 1 of its proof)
and R22A-04 (section 2).

For a permutation `σ` of the points and a finite owned set `O`, `CleanPowers.cleanExp O σ` is
`L`, the least common multiple of the lengths of the cycles of `σ` that meet `O` (`1` when none
does). The clean power of a commutator `A` is `A^{L_A}`.

Generic results, for permutations of any type of points:

* `CleanPowers.pow_fixes_iff`: a power `σ^e` fixes every point of `O` exactly when `L ∣ e`
  (R22A-03, step 1, and the completeness of R22A-04); `CleanPowers.pow_eq_cleanPow_pow`: such a
  power is a power of `σ^L`.
* `CleanPowers.cleanExp_dvd_orderOf` and `CleanPowers.orderOf_dvd_lcmUpTo`: `L` divides the order
  of `σ`, which divides `E = lcm(1, …, N)` for `N` points.
* `CleanPowers.exists_pos_pow_eq_inv` and `CleanPowers.eqvGen_iff_reflTransGen`: every step has
  finite order, so its inverse is a positive power of it and the undirected components of the
  graph are its reachability classes.
* `CleanPowers.exists_walk_length_lt`: a point reachable from `p` is reached by a path of at most
  `f − 1` steps, where `f` is the number of points reachable from `p`.

For the class-cell model: `ClassCellModel.cleanPower`, `ClassCellModel.pathWord` and the
statements `ClassCellModel.cleanExp_dvd_orderOf_dvd`, `ClassCellModel.cleanPower_complete`,
`ClassCellModel.cleanPower_inv` and `ClassCellModel.exists_cleanPath` (R22A-04).

Not formalized: the integrator's unreviewed remark on Landau's function, the count of
commutators `m ≤ (120·59)²`, the breadth-first search cost and the instance numbers.
-/

namespace Jumbling

namespace CleanPowers

open Function

variable {α : Type*}

/-- `L`: the least common multiple of the lengths of the cycles of `σ` that meet `O`. -/
noncomputable def cleanExp (O : Finset α) (σ : Equiv.Perm α) : ℕ :=
  O.lcm fun q => minimalPeriod σ q

/-- A point on a cycle of length `λ` is fixed by `σ^e` exactly when `λ ∣ e`. -/
lemma pow_apply_eq_iff (σ : Equiv.Perm α) (q : α) (e : ℕ) :
    (σ ^ e) q = q ↔ minimalPeriod σ q ∣ e := by
  rw [← isPeriodicPt_iff_minimalPeriod_dvd, IsPeriodicPt, IsFixedPt, Equiv.Perm.iterate_eq_pow]

/-- R22A-03, step 1, and R22A-04, completeness: a power of `σ` fixes every point of `O` exactly
when its exponent is a multiple of `L`. -/
theorem pow_fixes_iff (O : Finset α) (σ : Equiv.Perm α) (e : ℕ) :
    (∀ q ∈ O, (σ ^ e) q = q) ↔ cleanExp O σ ∣ e := by
  simp only [pow_apply_eq_iff, cleanExp, Finset.lcm_dvd_iff]

/-- The clean power `σ^L` fixes every point of `O`. -/
lemma cleanPow_fixes (O : Finset α) (σ : Equiv.Perm α) : ∀ q ∈ O, (σ ^ cleanExp O σ) q = q :=
  (pow_fixes_iff O σ _).2 dvd_rfl

/-- R22A-04, completeness: every power of `σ` that fixes `O` is a power of the clean power. -/
theorem pow_eq_cleanPow_pow {O : Finset α} {σ : Equiv.Perm α} {e : ℕ}
    (h : ∀ q ∈ O, (σ ^ e) q = q) : σ ^ e = (σ ^ cleanExp O σ) ^ (e / cleanExp O σ) := by
  rw [← pow_mul, Nat.mul_div_cancel' ((pow_fixes_iff O σ e).1 h)]

/-- `E = lcm(1, …, N)`. -/
def lcmUpTo (N : ℕ) : ℕ :=
  (Finset.Icc 1 N).lcm id

lemma lcmUpTo_pos (N : ℕ) : 0 < lcmUpTo N := by
  rw [Nat.pos_iff_ne_zero, lcmUpTo, Ne, Finset.lcm_eq_zero_iff]
  simp

lemma minimalPeriod_dvd_orderOf (σ : Equiv.Perm α) (q : α) : minimalPeriod σ q ∣ orderOf σ := by
  rw [← pow_apply_eq_iff, pow_orderOf_eq_one]
  rfl

/-- R22A-04: `L` divides the order of `σ`. -/
theorem cleanExp_dvd_orderOf (O : Finset α) (σ : Equiv.Perm α) : cleanExp O σ ∣ orderOf σ :=
  Finset.lcm_dvd fun q _ => minimalPeriod_dvd_orderOf σ q

section Finite

variable [Finite α]

/-- R22A-04: the order of a permutation of `N` points divides `E = lcm(1, …, N)`. -/
theorem orderOf_dvd_lcmUpTo (σ : Equiv.Perm α) : orderOf σ ∣ lcmUpTo (Nat.card α) := by
  apply orderOf_dvd_of_pow_eq_one
  ext q
  rw [Equiv.Perm.one_apply, pow_apply_eq_iff]
  have hfix : IsPeriodicPt σ (orderOf σ) q := by
    rw [IsPeriodicPt, IsFixedPt, Equiv.Perm.iterate_eq_pow, pow_orderOf_eq_one]
    rfl
  have hmem : minimalPeriod σ q ∈ Finset.Icc 1 (Nat.card α) := by
    have := Fintype.ofFinite α
    rw [Finset.mem_Icc, Nat.card_eq_fintype_card]
    exact ⟨hfix.minimalPeriod_pos (orderOf_pos σ), minimalPeriod_le_card⟩
  exact Finset.dvd_lcm (f := id) hmem

/-- R22A-04: `L ≤ E`. -/
lemma cleanExp_le_lcmUpTo (O : Finset α) (σ : Equiv.Perm α) :
    cleanExp O σ ≤ lcmUpTo (Nat.card α) :=
  Nat.le_of_dvd (lcmUpTo_pos _) ((cleanExp_dvd_orderOf O σ).trans (orderOf_dvd_lcmUpTo σ))

/-- R22A-04: a permutation of finitely many points has finite order, so its inverse is a positive
power of it. -/
theorem exists_pos_pow_eq_inv (σ : Equiv.Perm α) : ∃ n, 0 < n ∧ σ⁻¹ = σ ^ n := by
  have ho := orderOf_pos σ
  refine ⟨2 * orderOf σ - 1, by omega, (eq_inv_of_mul_eq_one_left ?_).symm⟩
  rw [← pow_succ, Nat.sub_add_cancel (by omega), pow_mul', pow_orderOf_eq_one, one_pow]

end Finite

/-! ### The graph of clean powers -/

section Graph

variable {ι : Type*} (step : ι → Equiv.Perm α)

/-- The edges of the graph: `q → step i q`. -/
def Edge (q q' : α) : Prop :=
  ∃ i, step i q = q'

/-- The end point of a path of steps from `p`, the first step acting first. -/
def walk (l : List ι) (p : α) : α :=
  l.foldl (fun x i => step i x) p

@[simp] lemma walk_nil (p : α) : walk step [] p = p := rfl

@[simp] lemma walk_cons (i : ι) (l : List ι) (p : α) :
    walk step (i :: l) p = walk step l (step i p) := rfl

lemma walk_append_singleton (l : List ι) (i : ι) (p : α) :
    walk step (l ++ [i]) p = step i (walk step l p) := by
  simp [walk, List.foldl_append]

lemma walk_replicate (n : ℕ) (i : ι) (p : α) :
    walk step (List.replicate n i) p = (step i ^ n) p := by
  induction n generalizing p with
  | zero => rfl
  | succ n ih => rw [List.replicate_succ, walk_cons, ih, pow_succ, Equiv.Perm.mul_apply]

lemma walk_map {κ : Type*} (g : κ → ι) (l : List κ) (p : α) :
    walk (fun k => step (g k)) l p = walk step (l.map g) p := by
  induction l generalizing p with
  | nil => rfl
  | cons k l ih => simp [ih]

lemma reflTransGen_walk (l : List ι) (p : α) :
    Relation.ReflTransGen (Edge step) p (walk step l p) := by
  induction l generalizing p with
  | nil => exact Relation.ReflTransGen.refl
  | cons i l ih => exact Relation.ReflTransGen.head ⟨i, rfl⟩ (ih (step i p))

lemma exists_walk_of_reflTransGen {p q : α} (h : Relation.ReflTransGen (Edge step) p q) :
    ∃ l, walk step l p = q := by
  induction h with
  | refl => exact ⟨[], rfl⟩
  | tail _ hbc ih =>
    obtain ⟨i, rfl⟩ := hbc
    obtain ⟨l, rfl⟩ := ih
    exact ⟨l ++ [i], walk_append_singleton step l i p⟩

section Finite

variable [Finite α]

/-- Every edge can be walked back along positive steps. -/
lemma reflTransGen_of_edge {q q' : α} (h : Edge step q q') :
    Relation.ReflTransGen (Edge step) q' q := by
  obtain ⟨i, rfl⟩ := h
  have := reflTransGen_walk step (List.replicate (orderOf (step i) - 1) i) (step i q)
  rwa [walk_replicate, ← Equiv.Perm.mul_apply, ← pow_succ,
    Nat.sub_add_cancel (orderOf_pos (step i)), pow_orderOf_eq_one, Equiv.Perm.one_apply] at this

lemma reflTransGen_symm {p q : α} (h : Relation.ReflTransGen (Edge step) p q) :
    Relation.ReflTransGen (Edge step) q p := by
  induction h with
  | refl => exact Relation.ReflTransGen.refl
  | tail _ hbc ih => exact (reflTransGen_of_edge step hbc).trans ih

/-- R22A-04: the undirected components of the graph are its reachability classes. -/
theorem eqvGen_iff_reflTransGen {p q : α} :
    Relation.EqvGen (Edge step) p q ↔ Relation.ReflTransGen (Edge step) p q := by
  constructor
  · intro h
    induction h with
    | rel x y hxy => exact Relation.ReflTransGen.single hxy
    | refl => exact Relation.ReflTransGen.refl
    | symm x y _ ih => exact reflTransGen_symm step ih
    | trans x y z _ _ ih₁ ih₂ => exact ih₁.trans ih₂
  · intro h
    induction h with
    | refl => exact Relation.EqvGen.refl _
    | tail _ hbc ih => exact Relation.EqvGen.trans _ _ _ ih (Relation.EqvGen.rel _ _ hbc)

/-- R22A-04: if `f` points are reachable from `p`, every one of them is reached by a path of at
most `f − 1` steps (a breadth-first ball argument). -/
theorem exists_walk_length_lt {p q : α} (h : Relation.ReflTransGen (Edge step) p q) :
    ∃ l : List ι, l.length < Nat.card {x // Relation.ReflTransGen (Edge step) p x} ∧
      walk step l p = q := by
  classical
  have := Fintype.ofFinite α
  set reach : Finset α := Finset.univ.filter (Relation.ReflTransGen (Edge step) p) with hreach
  have hcard : Nat.card {x // Relation.ReflTransGen (Edge step) p x} = reach.card := by
    rw [Nat.card_eq_fintype_card, Fintype.card_subtype]
  let ball : ℕ → Finset α := fun n =>
    Finset.univ.filter fun x => ∃ l : List ι, l.length ≤ n ∧ walk step l p = x
  have hmem : ∀ n x, x ∈ ball n ↔ ∃ l : List ι, l.length ≤ n ∧ walk step l p = x := by
    intro n x
    simp [ball]
  have hsub : ∀ n, ball n ⊆ reach := by
    intro n x hx
    obtain ⟨l, -, rfl⟩ := (hmem n x).1 hx
    simpa [hreach] using reflTransGen_walk step l p
  have hmono : ∀ n, ball n ⊆ ball (n + 1) := by
    intro n x hx
    obtain ⟨l, hl, rfl⟩ := (hmem n x).1 hx
    exact (hmem _ _).2 ⟨l, by omega, rfl⟩
  have hclosed : ∀ n, ball (n + 1) ⊆ ball n → reach ⊆ ball n := by
    intro n hs x hx
    have hx' : Relation.ReflTransGen (Edge step) p x := by simpa [hreach] using hx
    induction hx' with
    | refl => exact (hmem n p).2 ⟨[], by simp, rfl⟩
    | tail _ hbc ih =>
      obtain ⟨i, rfl⟩ := hbc
      obtain ⟨l, hl, hw⟩ := (hmem n _).1 (ih (by simpa [hreach]))
      refine hs ((hmem _ _).2 ⟨l ++ [i], by simp; omega, ?_⟩)
      rw [walk_append_singleton, hw]
  have key : ∀ n, ball n = reach ∨ n + 1 ≤ (ball n).card := by
    intro n
    induction n with
    | zero =>
      right
      exact Finset.card_pos.2 ⟨p, (hmem 0 p).2 ⟨[], le_rfl, rfl⟩⟩
    | succ n ih =>
      rcases ih with h | h
      · left
        exact subset_antisymm (hsub _) (h ▸ hmono n)
      · by_cases heq : ball (n + 1) = reach
        · exact Or.inl heq
        · right
          have hne : ¬ ball (n + 1) ⊆ ball n := fun hs =>
            heq (subset_antisymm (hsub _) ((hclosed n hs).trans (hmono n)))
          have hlt : (ball n).card < (ball (n + 1)).card :=
            Finset.card_lt_card ⟨hmono n, hne⟩
          omega
  have hp : p ∈ reach := by
    rw [hreach, Finset.mem_filter]
    exact ⟨Finset.mem_univ _, .refl⟩
  have hpos : 0 < reach.card := Finset.card_pos.2 ⟨p, hp⟩
  have hfin : ball (reach.card - 1) = reach := by
    rcases key (reach.card - 1) with h | h
    · exact h
    · exact Finset.eq_of_subset_of_card_le (hsub _) (by omega)
  have hq : q ∈ ball (reach.card - 1) := by
    rw [hfin]
    simpa [hreach] using h
  obtain ⟨l, hl, hw⟩ := (hmem _ _).1 hq
  exact ⟨l, by omega, hw⟩

end Finite

end Graph

end CleanPowers

namespace ClassCellModel

variable {Γ : Type*} [Group Γ] {R : Type*} [MulAction Γ R] {Grip Piece : Type*}
variable {M : PieceModel Γ R Grip Piece} {G : Subgroup Γ} {Cell : Type*} [MulAction G Cell]
  {Kind : Type*}
variable (Q : ClassCellModel M G Cell Kind)

/-- The clean power `A^{L_A}` of the commutator `A = [x, y]` for the owned set `O` (R22A-03). -/
noncomputable def cleanPower (O : Finset (Cell × M.Label G)) (xy : Q.Letter × Q.Letter) :
    List Q.Letter :=
  Q.wpow (Q.comm xy.1 xy.2) (CleanPowers.cleanExp O (Q.pointPerm (Q.comm xy.1 xy.2)))

/-- The word `A_0^{L_0} ⋯ A_{m−1}^{L_{m−1}}` of a path of clean powers. -/
noncomputable def pathWord (O : Finset (Cell × M.Label G)) :
    List (Q.Letter × Q.Letter) → List Q.Letter
  | [] => []
  | xy :: l => Q.cleanPower O xy ++ pathWord O l

variable (O : Finset (Cell × M.Label G))

lemma pointPerm_cleanPower (xy : Q.Letter × Q.Letter) :
    Q.pointPerm (Q.cleanPower O xy) =
      Q.pointPerm (Q.comm xy.1 xy.2) ^
        CleanPowers.cleanExp O (Q.pointPerm (Q.comm xy.1 xy.2)) :=
  Q.pointPerm_wpow _ _

lemma pointPerm_pathWord (l : List (Q.Letter × Q.Letter)) (q : Cell × M.Label G) :
    Q.pointPerm (Q.pathWord O l) q =
      CleanPowers.walk (fun xy => Q.pointPerm (Q.cleanPower O xy)) l q := by
  induction l generalizing q with
  | nil => simp [pathWord]
  | cons xy l ih =>
    rw [pathWord, pointPerm_append, Equiv.Perm.mul_apply, ih, CleanPowers.walk_cons]

/-- R22A-03, step 1: a clean power fixes every point of the owned set. -/
lemma cleanPower_fixes (xy : Q.Letter × Q.Letter) :
    ∀ q ∈ O, Q.pointPerm (Q.cleanPower O xy) q = q := by
  rw [pointPerm_cleanPower]
  exact CleanPowers.cleanPow_fixes O _

/-- A path of clean powers fixes every point of the owned set. -/
lemma pathWord_fixes (l : List (Q.Letter × Q.Letter)) :
    ∀ q ∈ O, Q.pointPerm (Q.pathWord O l) q = q := by
  intro q hq
  induction l with
  | nil => simp [pathWord]
  | cons xy l ih =>
    rw [pathWord, pointPerm_append, Equiv.Perm.mul_apply, Q.cleanPower_fixes O xy q hq, ih]

lemma length_cleanPower (xy : Q.Letter × Q.Letter) :
    (Q.cleanPower O xy).length =
      4 * CleanPowers.cleanExp O (Q.pointPerm (Q.comm xy.1 xy.2)) := by
  simp [cleanPower, Nat.mul_comm]

lemma length_pathWord (l : List (Q.Letter × Q.Letter)) :
    (Q.pathWord O l).length =
      (l.map fun xy => 4 * CleanPowers.cleanExp O (Q.pointPerm (Q.comm xy.1 xy.2))).sum := by
  induction l with
  | nil => rfl
  | cons xy l ih => simp [pathWord, ih, length_cleanPower]

lemma mem_pathWord {l : List (Q.Letter × Q.Letter)} {a : Q.Letter} (h : a ∈ Q.pathWord O l) :
    ∃ xy ∈ l, a.grip = xy.1.grip ∨ a.grip = xy.2.grip := by
  induction l with
  | nil => simp [pathWord] at h
  | cons xy l ih =>
    rw [pathWord, List.mem_append] at h
    rcases h with h | h
    · exact ⟨xy, List.mem_cons_self, Q.mem_comm (Q.mem_wpow h)⟩
    · obtain ⟨xy', hxy', h'⟩ := ih h
      exact ⟨xy', List.mem_cons_of_mem _ hxy', h'⟩

/-- R22A-04, completeness: every power `A^e` that fixes the owned set has `L_A ∣ e`, and it is a
power of the clean power `A^{L_A}`. -/
theorem cleanPower_complete (x y : Q.Letter) (e : ℕ)
    (h : ∀ q ∈ O, (Q.pointPerm (Q.comm x y) ^ e) q = q) :
    CleanPowers.cleanExp O (Q.pointPerm (Q.comm x y)) ∣ e ∧
      Q.pointPerm (Q.comm x y) ^ e = Q.pointPerm (Q.cleanPower O (x, y)) ^
        (e / CleanPowers.cleanExp O (Q.pointPerm (Q.comm x y))) := by
  refine ⟨(CleanPowers.pow_fixes_iff O _ e).1 h, ?_⟩
  rw [pointPerm_cleanPower]
  exact CleanPowers.pow_eq_cleanPow_pow h

variable [Finite (Cell × M.Label G)]

/-- R22A-04: for every commutator `A = [x, y]`, `L_A` divides the order of `A` on the points,
which divides `E = lcm(1, …, N_P)`, `N_P` the number of points. -/
theorem cleanExp_dvd_orderOf_dvd (x y : Q.Letter) :
    CleanPowers.cleanExp O (Q.pointPerm (Q.comm x y)) ∣ orderOf (Q.pointPerm (Q.comm x y)) ∧
      orderOf (Q.pointPerm (Q.comm x y)) ∣
        CleanPowers.lcmUpTo (Nat.card (Cell × M.Label G)) :=
  ⟨CleanPowers.cleanExp_dvd_orderOf O _, CleanPowers.orderOf_dvd_lcmUpTo _⟩

/-- R22A-04: each clean power has finite order, so its inverse is a positive power of it. -/
theorem cleanPower_inv (xy : Q.Letter × Q.Letter) :
    ∃ n, 0 < n ∧ (Q.pointPerm (Q.cleanPower O xy))⁻¹ = Q.pointPerm (Q.cleanPower O xy) ^ n :=
  CleanPowers.exists_pos_pow_eq_inv _

/-- R22A-04: over a set `S` of commutators (pairs of letters), the undirected components of the
graph of clean powers are its reachability classes. -/
theorem cleanGraph_eqvGen_iff (S : Set (Q.Letter × Q.Letter)) {p q : Cell × M.Label G} :
    Relation.EqvGen (CleanPowers.Edge fun xy : S => Q.pointPerm (Q.cleanPower O xy)) p q ↔
      Relation.ReflTransGen (CleanPowers.Edge fun xy : S => Q.pointPerm (Q.cleanPower O xy))
        p q :=
  CleanPowers.eqvGen_iff_reflTransGen _

/-- R22A-04: if `f` points are reachable from `p` in the graph of clean powers of the commutators
in `S`, a reachable target `t` has a path of at most `f − 1` clean powers, whose word has at most
`4E(f − 1) ≤ 4E(N_P − 1)` letters, `E = lcm(1, …, N_P)`. -/
theorem exists_cleanPath (S : Set (Q.Letter × Q.Letter)) {p t : Cell × M.Label G}
    (h : Relation.ReflTransGen
      (CleanPowers.Edge fun xy : S => Q.pointPerm (Q.cleanPower O xy)) p t) :
    ∃ l : List (Q.Letter × Q.Letter), (∀ xy ∈ l, xy ∈ S) ∧
      l.length + 1 ≤ Nat.card {x // Relation.ReflTransGen
        (CleanPowers.Edge fun xy : S => Q.pointPerm (Q.cleanPower O xy)) p x} ∧
      Q.pointPerm (Q.pathWord O l) p = t ∧
      (Q.pathWord O l).length ≤ 4 * CleanPowers.lcmUpTo (Nat.card (Cell × M.Label G)) *
        (Nat.card {x // Relation.ReflTransGen
          (CleanPowers.Edge fun xy : S => Q.pointPerm (Q.cleanPower O xy)) p x} - 1) ∧
      4 * CleanPowers.lcmUpTo (Nat.card (Cell × M.Label G)) *
        (Nat.card {x // Relation.ReflTransGen
          (CleanPowers.Edge fun xy : S => Q.pointPerm (Q.cleanPower O xy)) p x} - 1) ≤
        4 * CleanPowers.lcmUpTo (Nat.card (Cell × M.Label G)) *
          (Nat.card (Cell × M.Label G) - 1) := by
  set E := CleanPowers.lcmUpTo (Nat.card (Cell × M.Label G))
  obtain ⟨l, hl, hw⟩ := CleanPowers.exists_walk_length_lt _ h
  have hwalk := CleanPowers.walk_map (fun xy => Q.pointPerm (Q.cleanPower O xy))
    (Subtype.val : S → Q.Letter × Q.Letter) l p
  have hlen : (Q.pathWord O (l.map Subtype.val)).length ≤ 4 * E * l.length := by
    rw [length_pathWord, List.map_map]
    have : ∀ xy ∈ l.map (fun xy : S => 4 * CleanPowers.cleanExp O
        (Q.pointPerm (Q.comm xy.1.1 xy.1.2))), xy ≤ 4 * E := by
      intro n hn
      obtain ⟨xy, -, rfl⟩ := List.mem_map.1 hn
      exact Nat.mul_le_mul_left 4 (CleanPowers.cleanExp_le_lcmUpTo O _)
    have hsum := List.sum_le_length_nsmul _ _ this
    simpa [Function.comp_def, Nat.mul_comm, Nat.mul_left_comm, Nat.mul_assoc] using hsum
  have hcard : Nat.card {x // Relation.ReflTransGen
      (CleanPowers.Edge fun xy : S => Q.pointPerm (Q.cleanPower O xy)) p x} ≤
      Nat.card (Cell × M.Label G) := Finite.card_subtype_le _
  refine ⟨l.map Subtype.val, ?_, by simpa using hl, ?_, ?_, ?_⟩
  · intro xy hxy
    obtain ⟨⟨xy', hS⟩, -, rfl⟩ := List.mem_map.1 hxy
    exact hS
  · rw [pointPerm_pathWord, ← hwalk, hw]
  · refine hlen.trans (Nat.mul_le_mul_left _ ?_)
    omega
  · exact Nat.mul_le_mul_left _ (Nat.sub_le_sub_right hcard 1)

end ClassCellModel

end Jumbling
