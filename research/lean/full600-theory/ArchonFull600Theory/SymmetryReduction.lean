import ArchonFull600Theory.Normalization

set_option linter.style.header false in
/-!
# Lemma 13: conditions for sound symmetry reduction

Source: `research/Full_600cell_Puzzle_Theory.tex`, Lemma 13 (lines 1362 to 1427).

* `tetrahedralConjugation`: the source's example in one tetrahedral group. With
  `H = (1 2)(3 4)` and `T = (1 2 3)` in `A₄`, the conjugate `T H T⁻¹` is a half-turn other than
  `H` and none of the three local symbols `H`, `T`, `T⁻¹`. It is the three-letter word
  `T⁻¹, H, T` (read chronologically), and no word of fewer than three letters gives it, while
  `H` is a one-letter word. So conjugation inside a group, the simplest normalizing action,
  does not preserve the word length of a chosen generating alphabet.
* `PathCost`: paths in a graph with edge costs, by their total cost.
* `exists_goal_pathCost_iff`, `goalCosts_eq`, `isLeast_goalCosts_iff`: a group `L` acts on the
  states by cost-preserving graph automorphisms and the goal set is `L`-invariant. Then paths
  project to the orbit graph with the same cost (`PathCost.project`), and paths of the orbit
  graph lift to labelled paths of the same cost from any state of the start orbit
  (`PathCost.lift`). So the costs of paths to a goal and to a goal orbit agree, and with them
  the minimum costs.
-/

namespace ArchonFull600Theory

open Equiv

section Tetrahedral

/-- The half-turn `H = (1 2)(3 4)` of the source's tetrahedral example. -/
def tetraHalfTurn : Perm (StickerLabels 4) :=
  swap ⟨1, by decide⟩ ⟨2, by decide⟩ * swap ⟨3, by decide⟩ ⟨4, by decide⟩

/-- The third-turn `T = (1 2 3)` of the source's tetrahedral example. -/
def tetraThirdTurn : Perm (StickerLabels 4) :=
  [⟨1, by decide⟩, ⟨2, by decide⟩, ⟨3, by decide⟩].formPerm

theorem tetraHalfTurn_mem : tetraHalfTurn ∈ alternatingGroup (StickerLabels 4) := by
  rw [Perm.mem_alternatingGroup]; decide

theorem tetraThirdTurn_mem : tetraThirdTurn ∈ alternatingGroup (StickerLabels 4) := by
  rw [Perm.mem_alternatingGroup]; decide

/-- The three local symbols `H`, `T`, `T⁻¹` of one tetrahedral group, as elements of `A₄`. -/
def tetraSymbols : Fin 3 → alternatingGroup (StickerLabels 4) :=
  ![⟨tetraHalfTurn, tetraHalfTurn_mem⟩, ⟨tetraThirdTurn, tetraThirdTurn_mem⟩,
    ⟨tetraThirdTurn, tetraThirdTurn_mem⟩⁻¹]

/-- **Lemma 13, first statement.** Conjugating `H = (1 2)(3 4)` by `T = (1 2 3)` gives a
half-turn of `A₄` other than `H`, which is none of the three local symbols `H`, `T`, `T⁻¹`.
It is a word in them (first `T⁻¹`, then `H`, then `T`), and every word in them that gives it has
at least three letters, while `H` is a word of one letter. -/
theorem tetrahedralConjugation :
    let H := tetraHalfTurn
    let T := tetraThirdTurn
    T * H * T⁻¹ ∈ alternatingGroup (StickerLabels 4) ∧ orderOf (T * H * T⁻¹) = 2 ∧
      T * H * T⁻¹ ≠ H ∧ T * H * T⁻¹ ∉ Set.range (fun i => (tetraSymbols i : Perm _)) ∧
      (wordPermutation tetraSymbols [2, 0, 1] : Perm _) = T * H * T⁻¹ ∧
      (∀ w : List (Fin 3), (wordPermutation tetraSymbols w : Perm _) = T * H * T⁻¹ →
        3 ≤ w.length) ∧
      (wordPermutation tetraSymbols [0] : Perm _) = H := by
  intro H T
  refine ⟨by rw [Perm.mem_alternatingGroup]; decide, orderOf_eq_prime (by decide) (by decide),
    by decide, ?_, by decide, ?_, by decide⟩
  · rintro ⟨i, hi⟩
    fin_cases i <;> revert hi <;> decide
  · intro w hw
    by_contra hlt
    rcases w with _ | ⟨a, _ | ⟨b, _ | ⟨c, w⟩⟩⟩
    · revert hw; decide
    · fin_cases a <;> revert hw <;> decide
    · fin_cases a <;> fin_cases b <;> revert hw <;> decide
    · exact hlt (by simp)

end Tetrahedral

section Paths

variable {X C : Type*} [AddMonoid C]

/-- `PathCost edge x y c`: some path from `x` to `y` along `edge` has total cost `c`. An edge
`edge u v c` goes from `u` to `v` and costs `c`; parallel edges with different costs are
allowed. -/
inductive PathCost (edge : X → X → C → Prop) : X → X → C → Prop
  | refl (x : X) : PathCost edge x x 0
  | cons {x y z : X} {c d : C} : edge x y c → PathCost edge y z d → PathCost edge x z (c + d)

namespace PathCost

variable {edge edge' : X → X → C → Prop}

theorem trans {x y z : X} {c d : C} (h₁ : PathCost edge x y c) (h₂ : PathCost edge y z d) :
    PathCost edge x z (c + d) := by
  induction h₁ with
  | refl x => simpa using h₂
  | cons he _ ih => rw [add_assoc]; exact .cons he (ih h₂)

/-- A path along `edge` is a path along any larger edge relation, with the same cost. -/
theorem mono (h : ∀ x y c, edge x y c → edge' x y c) {x y : X} {c : C}
    (hp : PathCost edge x y c) : PathCost edge' x y c := by
  induction hp with
  | refl x => exact .refl x
  | cons he _ ih => exact .cons (h _ _ _ he) ih

end PathCost

section Quotient

variable (L : Type*) [Group L] [MulAction L X]

/-- The orbit graph: an edge of cost `c` between two orbits for each edge of cost `c` between
states in them. -/
def quotientEdge (edge : X → X → C → Prop) :
    MulAction.orbitRel.Quotient L X → MulAction.orbitRel.Quotient L X → C → Prop :=
  fun p q c => ∃ u v, edge u v c ∧ Quotient.mk _ u = p ∧ Quotient.mk _ v = q

/-- The goal orbits: the orbits of the goal states. -/
def quotientGoal (goal : Set X) : Set (MulAction.orbitRel.Quotient L X) :=
  Quotient.mk (MulAction.orbitRel L X) '' goal

variable {L} {edge : X → X → C → Prop}

/-- Projection sends every path to a path of the orbit graph with the same cost. -/
theorem PathCost.project {x y : X} {c : C} (h : PathCost edge x y c) :
    PathCost (quotientEdge L edge) (Quotient.mk _ x) (Quotient.mk _ y) c := by
  induction h with
  | refl x => exact .refl _
  | cons he _ ih => exact .cons ⟨_, _, he, rfl, rfl⟩ ih

/-- If `L` acts by cost-preserving graph automorphisms, a path of the orbit graph lifts, from
any state `x` of its start orbit, to a labelled path of the same cost. The lift of an orbit edge
represented by `u → v`, at the current state `x = ℓ • u`, is the edge `x → ℓ • v`. -/
theorem PathCost.lift (hedge : ∀ (ℓ : L) u v c, edge u v c → edge (ℓ • u) (ℓ • v) c)
    {p q : MulAction.orbitRel.Quotient L X} {c : C} (h : PathCost (quotientEdge L edge) p q c)
    (x : X) (hx : Quotient.mk _ x = p) :
    ∃ y, Quotient.mk _ y = q ∧ PathCost edge x y c := by
  induction h generalizing x with
  | refl p => exact ⟨x, hx, .refl x⟩
  | cons he _ ih =>
    obtain ⟨u, v, huv, rfl, rfl⟩ := he
    obtain ⟨ℓ, rfl⟩ : ∃ ℓ : L, ℓ • u = x :=
      MulAction.mem_orbit_iff.1 (MulAction.orbitRel_apply.1 (Quotient.exact hx))
    obtain ⟨y, hy, hp⟩ :=
      ih (ℓ • v) (Quotient.sound (MulAction.orbitRel_apply.2 (MulAction.mem_orbit v ℓ)))
    exact ⟨y, hy, .cons (hedge ℓ u v _ huv) hp⟩

/-- **Lemma 13, second statement.** If `L` acts by cost-preserving automorphisms of the state
graph and preserves the goal set, a state has a path of cost `c` to a goal exactly when its
orbit has a path of cost `c` to a goal orbit in the orbit graph. -/
theorem exists_goal_pathCost_iff (hedge : ∀ (ℓ : L) u v c, edge u v c → edge (ℓ • u) (ℓ • v) c)
    {goal : Set X} (hgoal : ∀ (ℓ : L) y, y ∈ goal → ℓ • y ∈ goal) (x : X) (c : C) :
    (∃ y ∈ goal, PathCost edge x y c) ↔
      ∃ q ∈ quotientGoal L goal, PathCost (quotientEdge L edge) (Quotient.mk _ x) q c := by
  constructor
  · rintro ⟨y, hy, hp⟩
    exact ⟨_, ⟨y, hy, rfl⟩, hp.project⟩
  · rintro ⟨q, ⟨y', hy', rfl⟩, hp⟩
    obtain ⟨y, hy, hpy⟩ := hp.lift hedge x rfl
    obtain ⟨ℓ, rfl⟩ : ∃ ℓ : L, ℓ • y' = y :=
      MulAction.mem_orbit_iff.1 (MulAction.orbitRel_apply.1 (Quotient.exact hy))
    exact ⟨_, hgoal ℓ y' hy', hpy⟩

/-- The costs of the paths from `x` to the goal set and from its orbit to the goal orbits are
the same set. -/
theorem goalCosts_eq (hedge : ∀ (ℓ : L) u v c, edge u v c → edge (ℓ • u) (ℓ • v) c)
    {goal : Set X} (hgoal : ∀ (ℓ : L) y, y ∈ goal → ℓ • y ∈ goal) (x : X) :
    {c | ∃ y ∈ goal, PathCost edge x y c} =
      {c | ∃ q ∈ quotientGoal L goal, PathCost (quotientEdge L edge) (Quotient.mk _ x) q c} :=
  Set.ext fun c => exists_goal_pathCost_iff hedge hgoal x c

/-- The quotient and the original minimum costs agree. -/
theorem isLeast_goalCosts_iff [Preorder C]
    (hedge : ∀ (ℓ : L) u v c, edge u v c → edge (ℓ • u) (ℓ • v) c)
    {goal : Set X} (hgoal : ∀ (ℓ : L) y, y ∈ goal → ℓ • y ∈ goal) (x : X) (c : C) :
    IsLeast {c | ∃ y ∈ goal, PathCost edge x y c} c ↔
      IsLeast {c | ∃ q ∈ quotientGoal L goal,
        PathCost (quotientEdge L edge) (Quotient.mk _ x) q c} c := by
  rw [goalCosts_eq hedge hgoal x]

end Quotient

end Paths

end ArchonFull600Theory
