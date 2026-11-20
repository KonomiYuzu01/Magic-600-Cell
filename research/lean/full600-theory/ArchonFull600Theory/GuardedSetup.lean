import ArchonFull600Theory.SymmetryReduction

set_option linter.style.header false in
/-!
# Lemma 15: guarded setup graphs and legal inverses

Source: `research/Full_600cell_Puzzle_Theory.tex`, Lemma 15 (lines 1470 to 1521).

* `frameEdge`: the setup graph on ordered target frames `f = (s₁, …, s_k) : Fin k → Sticker`.
  A letter `a` with move `g = moves a` and cost `cost a` gives the edge
  `f ↦ (g s₁, …, g s_k)`.
* `pathDistance`: the least cost of a path, in a complete lattice of costs such as `ℕ∞` or
  `ℝ≥0∞`, with `⊤` for an unreachable frame. `setupLength` is the unit-cost case.
* `pathDistance_le_of_embedding`: enlarging the alphabet, with the old letters keeping their
  moves and costs, cannot increase any minimum cost. `setupLength_withInverses_le` (unit costs)
  and `pathDistance_withInverses_le` (any costs, the old letters keeping theirs) are the case of
  adding the inverse of each letter.
* `withInverses_fixes`: if every letter fixes the buffer stickers pointwise, so does every
  inverse letter.
* `reachable_withInverses_iff`: if every inverse transition has a path of positive letters, the
  inverse letters do not enlarge the reachable set.
* The retained model: `HypothesisF.H_inv` and `HypothesisF.T_inv` (`H_c⁻¹ = H_c` and
  `T_c⁻¹ = T_c²`), the positive cap alphabet `capLetters`, `capSetup_reachable_iff` (its inverse
  transitions have positive witnesses), and `capGuard_fixes`: under the support property
  `HypothesisR.CapSupport`, which R and F do not state, forbidding every cap of `M(A) ∪ M(B)`
  makes every allowed letter fix every sticker of the buffer pieces `A` and `B`.

Not formalized: that breadth-first search and Dijkstra's algorithm compute these distances.
-/

namespace ArchonFull600Theory

open Equiv

namespace PathCost

variable {X C C' : Type*} [AddMonoid C] [AddMonoid C']

/-- If every edge of `edge` is joined by some path of `edge'`, every path of `edge` is joined by
some path of `edge'`. -/
theorem exists_of_forall {edge : X → X → C → Prop} {edge' : X → X → C' → Prop}
    (h : ∀ x y c, edge x y c → ∃ c', PathCost edge' x y c') {x y : X} {c : C}
    (hp : PathCost edge x y c) : ∃ c', PathCost edge' x y c' := by
  induction hp with
  | refl x => exact ⟨0, .refl x⟩
  | cons he _ ih =>
    obtain ⟨c₁, h₁⟩ := h _ _ _ he
    obtain ⟨c₂, h₂⟩ := ih
    exact ⟨_, h₁.trans h₂⟩

theorem single {edge : X → X → C → Prop} {x y : X} {c : C} (h : edge x y c) :
    PathCost edge x y c := by
  simpa using PathCost.cons h (.refl y)

end PathCost

section Distance

variable {X C : Type*} [AddMonoid C] [CompleteLattice C]

/-- The least cost of a path from `x` to `y`, and `⊤` when there is none. -/
noncomputable def pathDistance (edge : X → X → C → Prop) (x y : X) : C :=
  sInf {c | PathCost edge x y c}

theorem pathDistance_le {edge : X → X → C → Prop} {x y : X} {c : C} (h : PathCost edge x y c) :
    pathDistance edge x y ≤ c :=
  sInf_le h

/-- More edges, with the old edges keeping their costs, cannot increase a minimum cost. -/
theorem pathDistance_mono {edge edge' : X → X → C → Prop}
    (h : ∀ x y c, edge x y c → edge' x y c) (x y : X) :
    pathDistance edge' x y ≤ pathDistance edge x y :=
  sInf_le_sInf fun _ hc => hc.mono h

end Distance

section Setup

variable {Sticker Letter Letter' C : Type*} {k : ℕ}

/-- The setup graph on ordered target frames: the letter `a` sends the frame `f` to
`moves a ∘ f` at cost `cost a`. -/
def frameEdge (moves : Letter → Perm Sticker) (cost : Letter → C) :
    (Fin k → Sticker) → (Fin k → Sticker) → C → Prop :=
  fun f f' c => ∃ a, f' = ⇑(moves a) ∘ f ∧ c = cost a

/-- The minimum unit-cost setup length, `⊤` for an unreachable frame. -/
noncomputable def setupLength (moves : Letter → Perm Sticker) (f₀ f : Fin k → Sticker) : ℕ∞ :=
  pathDistance (frameEdge moves fun _ => (1 : ℕ∞)) f₀ f

/-- The alphabet with the inverse of each letter added: `Sum.inl a` is `a` and `Sum.inr a` is
its inverse. -/
def withInverses (moves : Letter → Perm Sticker) : Letter ⊕ Letter → Perm Sticker :=
  Sum.elim moves fun a => (moves a)⁻¹

theorem frameEdge_of_embedding {moves : Letter → Perm Sticker} {cost : Letter → C}
    {moves' : Letter' → Perm Sticker} {cost' : Letter' → C} (ι : Letter → Letter')
    (hm : ∀ a, moves' (ι a) = moves a) (hc : ∀ a, cost' (ι a) = cost a)
    (f f' : Fin k → Sticker) (c : C) (h : frameEdge moves cost f f' c) :
    frameEdge moves' cost' f f' c := by
  obtain ⟨a, rfl, rfl⟩ := h
  exact ⟨ι a, by rw [hm], (hc a).symm⟩

/-- Enlarging the alphabet, with the old letters keeping their moves and costs, cannot increase
any minimum setup cost. -/
theorem pathDistance_le_of_embedding [AddMonoid C] [CompleteLattice C]
    {moves : Letter → Perm Sticker} {cost : Letter → C} {moves' : Letter' → Perm Sticker}
    {cost' : Letter' → C} (ι : Letter → Letter') (hm : ∀ a, moves' (ι a) = moves a)
    (hc : ∀ a, cost' (ι a) = cost a) (f₀ f : Fin k → Sticker) :
    pathDistance (frameEdge moves' cost') f₀ f ≤ pathDistance (frameEdge moves cost) f₀ f :=
  pathDistance_mono (frameEdge_of_embedding ι hm hc) f₀ f

/-- **Lemma 15.** Enlarging an alphabet by the inverses of its letters cannot increase any
minimum unit-cost setup length (`⊤` for an unreachable frame). This holds for every alphabet;
`capGuardedSetup` applies it to the guarded cap alphabet. -/
theorem setupLength_withInverses_le (moves : Letter → Perm Sticker) (f₀ f : Fin k → Sticker) :
    setupLength (withInverses moves) f₀ f ≤ setupLength moves f₀ f :=
  pathDistance_le_of_embedding Sum.inl (fun _ => rfl) (fun _ => rfl) f₀ f

/-- **Lemma 15, costs.** With edge costs, adding the inverse letters cannot increase a minimum
cost, provided the old letters keep their costs. -/
theorem pathDistance_withInverses_le [AddMonoid C] [CompleteLattice C]
    (moves : Letter → Perm Sticker) (cost : Letter ⊕ Letter → C) (f₀ f : Fin k → Sticker) :
    pathDistance (frameEdge (withInverses moves) cost) f₀ f ≤
      pathDistance (frameEdge moves (cost ∘ Sum.inl)) f₀ f :=
  pathDistance_le_of_embedding Sum.inl (fun _ => rfl) (fun _ => rfl) f₀ f

/-- If every letter fixes the buffer stickers pointwise, so does every inverse letter. -/
theorem withInverses_fixes {moves : Letter → Perm Sticker} {buffers : Set Sticker}
    (h : ∀ a, ∀ s ∈ buffers, moves a s = s) (b : Letter ⊕ Letter) :
    ∀ s ∈ buffers, withInverses moves b s = s := by
  rcases b with a | a
  · exact h a
  · intro s hs
    exact Perm.inv_eq_iff_eq.2 (h a s hs).symm

/-- If every inverse transition is joined by a path of positive letters, adding the inverse
letters does not enlarge the set of reachable frames. -/
theorem reachable_withInverses_iff [AddMonoid C] {moves : Letter → Perm Sticker}
    {cost : Letter → C} {cost' : Letter ⊕ Letter → C}
    (hwit : ∀ a (f : Fin k → Sticker), ∃ c, PathCost (frameEdge moves cost) f (⇑(moves a)⁻¹ ∘ f) c)
    (f₀ f : Fin k → Sticker) :
    (∃ c, PathCost (frameEdge (withInverses moves) cost') f₀ f c) ↔
      ∃ c, PathCost (frameEdge moves cost) f₀ f c := by
  constructor
  · rintro ⟨c, hp⟩
    refine hp.exists_of_forall ?_
    rintro x y c ⟨a | a, rfl, rfl⟩
    · exact ⟨_, .single ⟨a, rfl, rfl⟩⟩
    · exact hwit a x
  · rintro ⟨c, hp⟩
    refine hp.exists_of_forall ?_
    rintro x y c ⟨a, rfl, rfl⟩
    exact ⟨_, .single ⟨Sum.inl a, rfl, rfl⟩⟩

end Setup

namespace HypothesisF

variable {Sticker Piece : Type*} {M : RetainedModel Sticker Piece} (F : HypothesisF M)

include F in
/-- `H_c⁻¹ = H_c`. -/
theorem H_inv (c : Cap) : (M.H c)⁻¹ = M.H c := by
  have h2 : M.H c ^ 2 = 1 := by
    rw [F.H_eq, ← map_pow, ← F.orderOf_halfTurn c, pow_orderOf_eq_one, map_one]
  exact inv_eq_of_mul_eq_one_right ((pow_two _).symm.trans h2)

include F in
/-- `T_c⁻¹ = T_c²`. -/
theorem T_inv (c : Cap) : (M.T c)⁻¹ = M.T c ^ 2 := by
  have h3 : M.T c ^ 3 = 1 := by
    rw [F.T_eq, ← map_pow, ← F.orderOf_thirdTurn c, pow_orderOf_eq_one, map_one]
  rw [pow_two]
  exact inv_eq_of_mul_eq_one_right ((pow_three _).symm.trans h3)

end HypothesisF

section CapGuard

variable {Sticker Piece C : Type*} {k : ℕ}

/-- The positive guarded alphabet of the retained model: `H_c` (`Sum.inl c`) and `T_c`
(`Sum.inr c`) for each allowed cap `c`. -/
def capLetters (M : RetainedModel Sticker Piece) (allowed : Set Cap) :
    allowed ⊕ allowed → Perm Sticker :=
  Sum.elim (fun c => M.H c) fun c => M.T c

/-- Under F, the inverse transitions of the cap alphabet already have positive witnesses
(`H_c⁻¹ = H_c`, `T_c⁻¹ = T_c²`), so its inverse letters do not enlarge the reachable set of
frames. -/
theorem capSetup_reachable_iff [AddMonoid C] {M : RetainedModel Sticker Piece}
    (F : HypothesisF M) (allowed : Set Cap) {cost : allowed ⊕ allowed → C}
    {cost' : (allowed ⊕ allowed) ⊕ (allowed ⊕ allowed) → C} (f₀ f : Fin k → Sticker) :
    (∃ c, PathCost (frameEdge (withInverses (capLetters M allowed)) cost') f₀ f c) ↔
      ∃ c, PathCost (frameEdge (capLetters M allowed) cost) f₀ f c := by
  refine reachable_withInverses_iff ?_ f₀ f
  rintro (a | a) x
  · have hx : ⇑(capLetters M allowed (Sum.inl a))⁻¹ ∘ x =
        ⇑(capLetters M allowed (Sum.inl a)) ∘ x := by
      change ⇑(M.H a)⁻¹ ∘ x = ⇑(M.H a) ∘ x
      rw [F.H_inv]
    rw [hx]
    exact ⟨_, .single ⟨Sum.inl a, rfl, rfl⟩⟩
  · have h₁ : frameEdge (capLetters M allowed) cost x (⇑(M.T a) ∘ x) (cost (Sum.inr a)) :=
      ⟨Sum.inr a, rfl, rfl⟩
    have h₂ : frameEdge (capLetters M allowed) cost (⇑(M.T a) ∘ x)
        (⇑(M.T a) ∘ ⇑(M.T a) ∘ x) (cost (Sum.inr a)) := ⟨Sum.inr a, rfl, rfl⟩
    have hx : ⇑(capLetters M allowed (Sum.inr a))⁻¹ ∘ x = ⇑(M.T a) ∘ ⇑(M.T a) ∘ x := by
      change ⇑(M.T a)⁻¹ ∘ x = _
      rw [F.T_inv, pow_two, Perm.coe_mul]
      rfl
    rw [hx]
    exact ⟨_, (PathCost.single h₁).trans (.single h₂)⟩

variable [Fintype Sticker] [Fintype Piece] [DecidableEq Piece] {M : RetainedModel Sticker Piece}

/-- The support property of the retained cap twists that the cap guard uses: the twists of cap
`c` fix every sticker of a piece whose complete signature does not contain `c`. This is the
reading of the source's "it rotates the cap and fixes its complement" (proof of Lemma 12);
hypotheses R and F do not state it. -/
def HypothesisR.CapSupport (R : HypothesisR M) : Prop :=
  ∀ c s, c ∉ R.signature (M.piece s) → M.H c s = s ∧ M.T c s = s

/-- **Lemma 15, cap guard.** Under the support property, if no allowed cap lies in
`M(A) ∪ M(B)`, every letter of the positive cap alphabet, and so every inverse letter, fixes
every sticker of the buffer pieces `A` and `B`. -/
theorem capGuard_fixes {R : HypothesisR M} (hsupp : R.CapSupport) {allowed : Set Cap}
    {A B : Piece} (hguard : ∀ c ∈ allowed, c ∉ R.signature A ∪ R.signature B) :
    (∀ a, ∀ s ∈ {s | M.piece s = A ∨ M.piece s = B}, capLetters M allowed a s = s) ∧
      ∀ b, ∀ s ∈ {s | M.piece s = A ∨ M.piece s = B},
        withInverses (capLetters M allowed) b s = s := by
  have h : ∀ a, ∀ s ∈ {s | M.piece s = A ∨ M.piece s = B}, capLetters M allowed a s = s := by
    intro a s hs
    have hc : ∀ c ∈ allowed, c ∉ R.signature (M.piece s) := by
      intro c hc hmem
      apply hguard c hc
      rcases hs with hs | hs
      · exact Finset.mem_union_left _ (hs ▸ hmem)
      · exact Finset.mem_union_right _ (hs ▸ hmem)
    rcases a with ⟨c, hca⟩ | ⟨c, hca⟩
    · exact (hsupp c s (hc c hca)).1
    · exact (hsupp c s (hc c hca)).2
  exact ⟨h, withInverses_fixes h⟩

/-- **Lemma 15 on the retained model.** For the positive cap alphabet of the allowed caps, the
inverse letters are legal and adding them cannot increase a minimum unit-cost setup length.
Under the support property, if no allowed cap lies in `M(A) ∪ M(B)`, every letter of the
enlarged alphabet fixes every sticker of the buffer pieces `A` and `B`. -/
theorem capGuardedSetup {R : HypothesisR M} (hsupp : R.CapSupport) {allowed : Set Cap}
    {A B : Piece} (hguard : ∀ c ∈ allowed, c ∉ R.signature A ∪ R.signature B) :
    (∀ b, withInverses (capLetters M allowed) b ∈ M.legalGroup) ∧
      (∀ b, ∀ s ∈ {s | M.piece s = A ∨ M.piece s = B},
        withInverses (capLetters M allowed) b s = s) ∧
      ∀ {n : ℕ} (f₀ f : Fin n → Sticker),
        setupLength (withInverses (capLetters M allowed)) f₀ f ≤
          setupLength (capLetters M allowed) f₀ f := by
  refine ⟨?_, (capGuard_fixes hsupp hguard).2, fun f₀ f => setupLength_withInverses_le _ f₀ f⟩
  rintro ((c | c) | (c | c))
  · exact M.H_mem c
  · exact M.T_mem c
  · exact inv_mem (M.H_mem c)
  · exact inv_mem (M.T_mem c)

end CapGuard

end ArchonFull600Theory
