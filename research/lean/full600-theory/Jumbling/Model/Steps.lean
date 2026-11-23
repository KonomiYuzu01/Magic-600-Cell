import Jumbling.Model.Pieces

set_option linter.style.header false in
/-!
# Composing and conjugating steps (R22A2-02, R22D-01, R22D-03, R22D-05)

Source: `research/jumbling/restoration/12-low-height/PROOFS-B.md`, R22A2-02 (section 2) and
R22D-01, R22D-03 and R22D-05 (section 5). All four are statements about the piece model of
`Jumbling.Model.Pieces`; they use only that each piece moves on its own and that a menu element
keeps its cap.

Main results:

* `PieceModel.isStrictPrime_after_iff` (R22A2-02): a second strict′ step `w₂` survives the first
  step `w₁` exactly when `Bad(w₂) ⊆ holes(Z·w₁)`; `PieceModel.occupied_apply` and
  `PieceModel.isStrictPrime_after_iff_disjoint` give the test at `Z` alone.
* `PieceModel.isStrictPrime_conj_iff` (R22D-01): for a word `A` of retained letters admissible at
  `Z`, `C` is a strict′ step at `Z·A` exactly when `A·C·A⁻¹` is one at `Z`;
  `PieceModel.admissible_wordInv_of_isStrictPrime` (the suffix `A⁻¹` is admissible automatically)
  and `PieceModel.apply_conj_eq_iff` (exact return, piece by piece).
* `PieceModel.isStrictPrime_lowering_iff` (R22D-03): the endpoint contract of `a·C·a⁻¹`, with
  the consequences `PieceModel.side_outside_of_lowering` and
  `PieceModel.isStrictPrime_lowering_of_returns`.
* `PieceModel.nestedVacancy` (R22D-05): `a·q⁻¹·b·q·a⁻¹` lands the unit and returns every other
  piece exactly.

Not formalized: in R22A2-02 the remarks on disjoint caps and distinct destination cells (the
latter is a computed instance) and the capacity case; in R22D-01 the exact update of the
classifications for one letter (it needs the action of `K` on the grips, which the model does
not have) and the counts; in R22D-03 the consequences that use AU-03 or R21F-12 and the remark
that returning only the pieces inside `c` is not enough; in R22D-05 the existence bound and the
necessary conditions on the orbits.
-/

namespace Jumbling

namespace PieceModel

variable {Γ : Type*} [Group Γ] {R : Type*} [MulAction Γ R] {Grip Piece : Type*}
variable (M : PieceModel Γ R Grip Piece)

lemma apply_singleton (a : M.Letter) (s : Piece → Γ) (p : Piece) :
    M.apply [a] s p = M.mult a (M.region s p) * s p := by
  simp [apply]

lemma wordInv_singleton (a : M.Letter) : M.wordInv [a] = [a.inv] := rfl

lemma endRegion_endRegion_wordInv (w : List M.Letter) (r : R) :
    M.endRegion w (M.endRegion (M.wordInv w) r) = r := by
  simpa using M.endRegion_wordInv (M.wordInv w) r

lemma retained_wordInv {w : List M.Letter} (hw : ∀ a ∈ w, a.elem ∈ M.K) :
    ∀ b ∈ M.wordInv w, b.elem ∈ M.K := by
  intro b hb
  obtain ⟨a, ha, rfl⟩ := M.mem_wordInv.1 hb
  exact M.K.inv_mem (hw a ha)

/-! ### R22A2-02 -/

section Survival

variable {Z : Piece → Γ} {U₁ U₂ : Set Piece} {w₁ w₂ : List M.Letter}

/-- After a strict′ step, every off piece was off before, with the same pose, and is not in the
unit of the step. -/
lemma IsStrictPrime.off_after (h : M.IsStrictPrime Z U₁ w₁) {p : Piece}
    (hp : M.apply w₁ Z p ∉ M.K) : Z p ∉ M.K ∧ M.apply w₁ Z p = Z p ∧ p ∉ U₁ := by
  have hZ : Z p ∉ M.K := fun h' => hp (h.keepsLattice p h')
  have hU : p ∉ U₁ := fun h' => hp (h.lands p h')
  exact ⟨hZ, (h.others p hU hZ).resolve_left hp, hU⟩

/-- A lattice piece at a state whose holes contain `Bad(w)` follows a good ghost. -/
lemma clear_and_mem_of_Bad_subset {s : Piece → Γ} {w : List M.Letter}
    (hB : M.Bad w ⊆ M.holes s) {p : Piece} (hp : s p ∈ M.K) :
    M.Clear w (M.region s p) ∧ M.acc w (M.region s p) ∈ M.K := by
  by_contra hcon
  have hbad : M.region s p ∈ M.Bad w := by
    refine ⟨M.region_mem_chamber hp, ?_⟩
    by_cases hc : M.Clear w (M.region s p)
    · exact Or.inr fun hk => hcon ⟨hc, hk⟩
    · exact Or.inl hc
  exact (hB hbad).2 p hp rfl

/-- R22A2-02. Let `w₁` and `w₂` be strict′ steps at `Z` for the units `U₁` and `U₂`, and
`Z₁ = Z·w₁`. Then `w₂` is a strict′ step for `U₂` at `Z₁` if and only if
`Bad(w₂) ⊆ holes(Z₁)`. (The source asks for disjoint units; the proof does not use it.) -/
theorem isStrictPrime_after_iff (h₁ : M.IsStrictPrime Z U₁ w₁) (h₂ : M.IsStrictPrime Z U₂ w₂) :
    M.IsStrictPrime (M.apply w₁ Z) U₂ w₂ ↔ M.Bad w₂ ⊆ M.holes (M.apply w₁ Z) := by
  constructor
  · intro h x hx
    refine ⟨hx.1, fun p hp hpx => ?_⟩
    subst hpx
    rcases hx.2 with hx | hx
    · exact hx (h.admissible p)
    · exact hx ((M.apply_mem_K_iff hp).1 (h.keepsLattice p hp))
  · intro hB
    have hlat : ∀ p, M.apply w₁ Z p ∈ M.K →
        M.Clear w₂ (M.region (M.apply w₁ Z) p) ∧ M.apply w₂ (M.apply w₁ Z) p ∈ M.K :=
      fun p hp => ⟨(M.clear_and_mem_of_Bad_subset hB hp).1,
        (M.apply_mem_K_iff hp).2 (M.clear_and_mem_of_Bad_subset hB hp).2⟩
    have hoff : ∀ p, M.apply w₁ Z p ∉ M.K → Z p ∉ M.K ∧ M.apply w₁ Z p = Z p ∧
        M.region (M.apply w₁ Z) p = M.region Z p ∧
        M.apply w₂ (M.apply w₁ Z) p = M.apply w₂ Z p := by
      intro p hp
      obtain ⟨hZ, heq, -⟩ := h₁.off_after M hp
      exact ⟨hZ, heq, by simp [region, heq], M.apply_eq_apply_of_eq heq w₂⟩
    refine ⟨fun p => ?_, fun p hp => ?_, fun p hp => (hlat p hp).2, fun p hp hpK => ?_⟩
    · by_cases hp : M.apply w₁ Z p ∈ M.K
      · exact (hlat p hp).1
      · rw [(hoff p hp).2.2.1]
        exact h₂.admissible p
    · by_cases hK : M.apply w₁ Z p ∈ M.K
      · exact (hlat p hK).2
      · rw [(hoff p hK).2.2.2]
        exact h₂.lands p hp
    · obtain ⟨hZ, heq, -, happ⟩ := hoff p hpK
      rw [happ, heq]
      exact h₂.others p hp hZ

/-- R22A2-02, a test at `Z` alone: the occupied chambers of `Z₁ = Z·w₁` are `φ₁(L) ∪ D₁`, where
`L` is the set of occupied chambers of `Z`, `φ₁` maps a chamber to the end chamber of its ghost
under `w₁`, and `D₁` is the set of end chambers of the pieces that `w₁` lands. -/
lemma occupied_apply (h₁ : M.IsStrictPrime Z U₁ w₁) :
    M.occupied (M.apply w₁ Z) = M.endRegion w₁ '' M.occupied Z ∪
      {x | ∃ p, Z p ∉ M.K ∧ M.apply w₁ Z p ∈ M.K ∧ M.region (M.apply w₁ Z) p = x} := by
  ext x
  constructor
  · rintro ⟨p, hp, rfl⟩
    by_cases hZ : Z p ∈ M.K
    · exact Or.inl ⟨M.region Z p, ⟨p, hZ, rfl⟩, (M.region_apply w₁ Z p).symm⟩
    · exact Or.inr ⟨p, hZ, hp, rfl⟩
  · rintro (⟨y, ⟨p, hp, rfl⟩, rfl⟩ | ⟨p, -, hp, rfl⟩)
    · exact ⟨p, h₁.keepsLattice p hp, M.region_apply w₁ Z p⟩
    · exact ⟨p, hp, rfl⟩

/-- R22A2-02, a test at `Z` alone: `w₂` is a strict′ step at `Z₁` exactly when `Bad(w₂)` misses
both `φ₁(L)` and `D₁`. -/
theorem isStrictPrime_after_iff_disjoint (h₁ : M.IsStrictPrime Z U₁ w₁)
    (h₂ : M.IsStrictPrime Z U₂ w₂) :
    M.IsStrictPrime (M.apply w₁ Z) U₂ w₂ ↔
      Disjoint (M.Bad w₂) (M.endRegion w₁ '' M.occupied Z) ∧
        Disjoint (M.Bad w₂)
          {x | ∃ p, Z p ∉ M.K ∧ M.apply w₁ Z p ∈ M.K ∧ M.region (M.apply w₁ Z) p = x} := by
  rw [M.isStrictPrime_after_iff h₁ h₂, M.subset_holes_iff (M.Bad_subset_chamber w₂),
    M.occupied_apply h₁, Set.disjoint_union_right]

end Survival

/-! ### R22D-01 -/

section Conjugation

variable {Z : Piece → Γ} {U : Set Piece} {A C : List M.Letter}

/-- R22D-01, the forward direction's admissibility: if `C` is a strict′ step at `Y = Z·A`, the
suffix `A⁻¹` is admissible after `C` automatically. -/
theorem admissible_wordInv_of_isStrictPrime (hA : ∀ a ∈ A, a.elem ∈ M.K)
    (hAadm : M.Admissible Z A) (h : M.IsStrictPrime (M.apply A Z) U C) :
    M.Admissible (M.apply C (M.apply A Z)) (M.wordInv A) := by
  intro p
  by_cases hT : M.apply C (M.apply A Z) p ∈ M.K
  · exact M.clear_of_mem_chamber (M.retained_wordInv hA) (M.region_mem_chamber hT)
  · have hY : M.apply A Z p ∉ M.K := fun h' => hT (h.keepsLattice p h')
    have hU : p ∉ U := fun h' => hT (h.lands p h')
    have heq : M.apply C (M.apply A Z) p = M.apply A Z p := (h.others p hU hY).resolve_left hT
    have hreg : M.region (M.apply C (M.apply A Z)) p = M.endRegion A (M.region Z p) := by
      rw [← M.region_apply A Z p]
      simp [region, heq]
    rw [hreg]
    exact M.clear_wordInv (hAadm p)

/-- R22D-01. For a word `A` of retained letters admissible at `Z` and `Y = Z·A`, a word `C` is a
strict′ landing step at `Y` for the pieces of `U` if and only if `A·C·A⁻¹` is a strict′ landing
step for `U` at `Z`. -/
theorem isStrictPrime_conj_iff (hA : ∀ a ∈ A, a.elem ∈ M.K) (hAadm : M.Admissible Z A) :
    M.IsStrictPrime (M.apply A Z) U C ↔ M.IsStrictPrime Z U (A ++ C ++ M.wordInv A) := by
  have hAi := M.retained_wordInv hA
  have happ : M.apply (A ++ C ++ M.wordInv A) Z =
      M.apply (M.wordInv A) (M.apply C (M.apply A Z)) := by
    rw [apply_append, apply_append]
  set Y := M.apply A Z with hYdef
  set T := M.apply C Y with hTdef
  have hYK : ∀ p, Y p ∈ M.K ↔ Z p ∈ M.K := M.apply_mem_K_iff_of_retained hA Z
  have hEK : ∀ p, M.apply (M.wordInv A) T p ∈ M.K ↔ T p ∈ M.K :=
    M.apply_mem_K_iff_of_retained hAi T
  have hret : ∀ p, M.apply (M.wordInv A) T p = Z p ↔ T p = Y p :=
    fun p => M.apply_wordInv_eq_iff A Z T p
  constructor
  · intro h
    refine ⟨?_, fun p hp => ?_, fun p hp => ?_, fun p hp hZ => ?_⟩
    · rw [M.admissible_append, M.admissible_append, apply_append]
      exact ⟨⟨hAadm, h.admissible⟩, M.admissible_wordInv_of_isStrictPrime hA hAadm h⟩
    · rw [happ, hEK]
      exact h.lands p hp
    · rw [happ, hEK]
      exact h.keepsLattice p ((hYK p).2 hp)
    · rw [happ, hEK, hret]
      exact h.others p hp (fun h' => hZ ((hYK p).1 h'))
  · intro h
    refine ⟨?_, fun p hp => ?_, fun p hp => ?_, fun p hp hY => ?_⟩
    · have := h.admissible
      rw [M.admissible_append, M.admissible_append] at this
      exact this.1.2
    · rw [← hEK, ← happ]
      exact h.lands p hp
    · rw [← hEK, ← happ]
      exact h.keepsLattice p ((hYK p).1 hp)
    · rw [← hEK, ← hret, ← happ]
      exact h.others p hp (fun h' => hY ((hYK p).2 h'))

/-- R22D-01. `A·C·A⁻¹` returns a piece exactly to its pose of `Z` if and only if `C` returns it
exactly to its pose of `Y = Z·A`. This holds for every word `A`. -/
theorem apply_conj_eq_iff (A C : List M.Letter) (Z : Piece → Γ) (p : Piece) :
    M.apply (A ++ C ++ M.wordInv A) Z p = Z p ↔ M.apply C (M.apply A Z) p = M.apply A Z p := by
  rw [apply_append, apply_append]
  exact M.apply_wordInv_eq_iff A Z _ p

/-- R22D-01. `A·C·A⁻¹` returns every piece outside `U` exactly if and only if `C` does so at
`Y = Z·A`. -/
theorem forall_apply_conj_eq_iff (A C : List M.Letter) (Z : Piece → Γ) (U : Set Piece) :
    (∀ p ∉ U, M.apply (A ++ C ++ M.wordInv A) Z p = Z p) ↔
      ∀ p ∉ U, M.apply C (M.apply A Z) p = M.apply A Z p := by
  simp only [M.apply_conj_eq_iff]

end Conjugation

/-! ### R22D-03 -/

section Lowering

variable {Z T : Piece → Γ} {U : Set Piece} {C : List M.Letter} {a : M.Letter}

/-- R22D-03. Let `a = (c, D)` be admissible at `Z`, `Y = Z·a`, `C` admissible at `Y` and
`T = Y·C`; `D^(−ε_p)` is `D⁻¹` when the piece `p` lies inside `c` at `T` and `1` otherwise. Then
`a·C·a⁻¹` is admissible and strict′ for `U` if and only if (1) `c` is clean at `T`;
(2) `D^(−ε_p)·T_p ∈ K` for every `p` in `U` or on the lattice at `Z`; (3) `D^(−ε_p)·T_p` is the
pose of `p` at `Z` or lies in `K` for every other off piece of `Z`. (The source's hypotheses that
`U` lies whole in `c` at height 4 and that `D` lowers it to height 2 are not used.) -/
theorem isStrictPrime_lowering_iff (ha : M.LetterAdmissible Z a)
    (hC : M.Admissible (M.apply [a] Z) C) (hT : M.apply C (M.apply [a] Z) = T) :
    M.IsStrictPrime Z U ([a] ++ C ++ [a.inv]) ↔
      (∀ p, M.side a.grip (M.region T p) ≠ .straddle) ∧
      (∀ p, (p ∈ U ∨ Z p ∈ M.K) →
        (if M.side a.grip (M.region T p) = .inside then a.elem⁻¹ else 1) * T p ∈ M.K) ∧
      (∀ p ∉ U, Z p ∉ M.K →
        (if M.side a.grip (M.region T p) = .inside then a.elem⁻¹ else 1) * T p = Z p ∨
        (if M.side a.grip (M.region T p) = .inside then a.elem⁻¹ else 1) * T p ∈ M.K) := by
  have happ : ∀ p, M.apply ([a] ++ C ++ [a.inv]) Z p =
      (if M.side a.grip (M.region T p) = .inside then a.elem⁻¹ else 1) * T p := by
    intro p
    rw [apply_append, apply_append, hT, apply_singleton]
    rfl
  have hadm : M.Admissible Z ([a] ++ C ++ [a.inv]) ↔
      ∀ p, M.side a.grip (M.region T p) ≠ .straddle := by
    rw [M.admissible_append, M.admissible_append, M.admissible_singleton,
      M.admissible_singleton, apply_append, hT]
    exact ⟨fun h => h.2, fun h => ⟨⟨ha, hC⟩, h⟩⟩
  constructor
  · intro h
    refine ⟨hadm.1 h.admissible, fun p hp => ?_, fun p hp hZ => ?_⟩
    · rw [← happ]
      rcases hp with hp | hp
      · exact h.lands p hp
      · exact h.keepsLattice p hp
    · rw [← happ]
      exact (h.others p hp hZ).symm
  · rintro ⟨h1, h2, h3⟩
    refine ⟨hadm.2 h1, fun p hp => ?_, fun p hp => ?_, fun p hp hZ => ?_⟩
    · rw [happ]
      exact h2 p (Or.inl hp)
    · rw [happ]
      exact h2 p (Or.inr hp)
    · rw [happ]
      exact (h3 p hp hZ).symm

/-- R22D-03, first consequence: if `D ∉ K` (the source's `D` lowers `U`, so it is not in `K⁺`),
`a·C·a⁻¹` is strict′ for `U` and `C` lands `U`, then `U` ends `C` outside `c`. -/
theorem side_outside_of_lowering (ha : M.LetterAdmissible Z a)
    (hC : M.Admissible (M.apply [a] Z) C) (hT : M.apply C (M.apply [a] Z) = T)
    (hD : a.elem ∉ M.K) (h : M.IsStrictPrime Z U ([a] ++ C ++ [a.inv]))
    (hland : ∀ p ∈ U, T p ∈ M.K) :
    ∀ p ∈ U, M.side a.grip (M.region T p) = .outside := by
  obtain ⟨h1, h2, -⟩ := (M.isStrictPrime_lowering_iff ha hC hT).1 h
  intro p hp
  have h2p := h2 p (Or.inl hp)
  cases hs : M.side a.grip (M.region T p) with
  | outside => rfl
  | straddle => exact absurd hs (h1 p)
  | inside =>
    simp only [hs, ↓reduceIte, M.K.mul_mem_cancel_right (hland p hp)] at h2p
    exact absurd (M.K.inv_mem_iff.1 h2p) hD

/-- R22D-03, the sufficient condition: if `C` lands `U` outside `c` and returns every other piece
exactly to its pose of `Y = Z·a`, then `a·C·a⁻¹` is admissible and strict′ for `U`, and every
piece outside `U` retraces `a` to its pose of `Z`. -/
theorem isStrictPrime_lowering_of_returns (ha : M.LetterAdmissible Z a)
    (hC : M.Admissible (M.apply [a] Z) C) (hT : M.apply C (M.apply [a] Z) = T)
    (hU : ∀ p ∈ U, T p ∈ M.K ∧ M.side a.grip (M.region T p) = .outside)
    (hret : ∀ p ∉ U, T p = M.apply [a] Z p) :
    M.IsStrictPrime Z U ([a] ++ C ++ [a.inv]) ∧
      ∀ p ∉ U, M.apply ([a] ++ C ++ [a.inv]) Z p = Z p := by
  have happ : M.apply ([a] ++ C ++ [a.inv]) Z = M.apply [a.inv] T := by
    rw [apply_append, apply_append, hT]
  have hout : ∀ p ∉ U, M.apply ([a] ++ C ++ [a.inv]) Z p = Z p := by
    intro p hp
    rw [happ, ← wordInv_singleton, M.apply_wordInv_eq_iff]
    exact hret p hp
  have hin : ∀ p ∈ U, M.apply ([a] ++ C ++ [a.inv]) Z p = T p := by
    intro p hp
    rw [happ, apply_singleton]
    simp [mult, (hU p hp).2]
  refine ⟨⟨?_, fun p hp => ?_, fun p hp => ?_, fun p hp _ => Or.inr (hout p hp)⟩, hout⟩
  · rw [M.admissible_append, M.admissible_append, M.admissible_singleton,
      M.admissible_singleton, apply_append, hT]
    refine ⟨⟨ha, hC⟩, fun p => ?_⟩
    by_cases hp : p ∈ U
    · simp [(hU p hp).2]
    · have hreg : M.region T p = M.step a (M.region Z p) := by
        rw [← M.endRegion_singleton, ← M.region_apply]
        simp [region, hret p hp]
      simpa [hreg, side_step] using ha p
  · rw [hin p hp]
    exact (hU p hp).1
  · by_cases hpU : p ∈ U
    · rw [hin p hpU]
      exact (hU p hpU).1
    · rw [hout p hpU]
      exact hp

end Lowering

/-! ### R22D-05 -/

section NestedVacancy

/-- R22D-05. Let `a = (c, D)` be admissible at `Z` and `Y = Z·a`; let `d` be the grip of a
letter `b` that is clean at `Y` and whose cap holds exactly the unit `U` among the off pieces of
`Y`, and suppose that `b` lands `U`. Let `T` be the home chambers inside `d`, `L` the chambers
where `b` lands `U` and `Ω_c` the home chambers inside `c`. If a SAFE-ALL word `q` at `Y` has
`π_q(T) ⊆ holes(Y)` and `π_q(L) ∩ Ω_c = ∅`, then `a·q⁻¹·b·q·a⁻¹` is admissible at `Z`, lands
`U` and returns every other piece exactly to its pose of `Z`. (The source's hypotheses that `a`
lowers a height-4 unit to height 2 and that `b` is a jumble letter are not used.) -/
theorem nestedVacancy {Z Y : Piece → Γ} {U : Set Piece} (a b : M.Letter) (q : List M.Letter)
    (ha : M.LetterAdmissible Z a) (hY : M.apply [a] Z = Y)
    (hb : M.LetterAdmissible Y b) (hUoff : ∀ p ∈ U, Y p ∉ M.K)
    (hholds : ∀ p, Y p ∉ M.K → (M.side b.grip (M.region Y p) = .inside ↔ p ∈ U))
    (hlands : ∀ p ∈ U, b.elem * Y p ∈ M.K) (hq : M.SafeWord Y q)
    (hT : M.endRegion q '' {x | x ∈ M.chamber ∧ M.side b.grip x = .inside} ⊆ M.holes Y)
    (hL : Disjoint (M.endRegion q '' ((fun p => b.elem • M.region Y p) '' U))
      {x | x ∈ M.chamber ∧ M.side a.grip x = .inside}) :
    M.Admissible Z ([a] ++ M.wordInv q ++ [b] ++ q ++ [a.inv]) ∧
      (∀ p ∈ U, M.apply ([a] ++ M.wordInv q ++ [b] ++ q ++ [a.inv]) Z p ∈ M.K) ∧
      ∀ p ∉ U, M.apply ([a] ++ M.wordInv q ++ [b] ++ q ++ [a.inv]) Z p = Z p := by
  set s₁ := M.apply (M.wordInv q) Y with hs₁
  set s₂ := M.apply [b] s₁ with hs₂
  set s₃ := M.apply q s₂ with hs₃
  have hW : M.apply ([a] ++ M.wordInv q ++ [b] ++ q ++ [a.inv]) Z = M.apply [a.inv] s₃ := by
    rw [apply_append, apply_append, apply_append, apply_append, hY]
  have hq' := hq.wordInv
  -- the stage `s₁ = Y·q⁻¹`
  have h1off : ∀ p, Y p ∉ M.K → s₁ p = Y p := fun p hp => (hq'.apply_off hp).2
  have h1lat : ∀ p, Y p ∈ M.K → s₁ p ∈ M.K := fun p hp => (hq'.apply_lattice hp).2
  have h1notT : ∀ p, Y p ∈ M.K → M.side b.grip (M.region s₁ p) ≠ .inside := by
    intro p hp hin
    have hmem : M.region s₁ p ∈ {x | x ∈ M.chamber ∧ M.side b.grip x = .inside} :=
      ⟨M.region_mem_chamber (h1lat p hp), hin⟩
    have := hT ⟨_, hmem, rfl⟩
    rw [hs₁, region_apply, endRegion_endRegion_wordInv] at this
    exact this.2 p hp rfl
  -- the stage `s₂ = s₁·b`
  have h2U : ∀ p ∈ U, s₂ p = b.elem * Y p ∧ M.region s₂ p = b.elem • M.region Y p := by
    intro p hp
    have hoff := hUoff p hp
    have hin : M.side b.grip (M.region Y p) = .inside := (hholds p hoff).2 hp
    have hreg : M.region s₁ p = M.region Y p := by simp [region, h1off p hoff]
    refine ⟨?_, ?_⟩
    · rw [hs₂, apply_singleton, hreg, h1off p hoff]
      simp [mult, hin]
    · rw [hs₂, region_apply, endRegion_singleton, hreg]
      simp [step, mult, hin]
  have h2off : ∀ p ∉ U, Y p ∉ M.K → s₂ p = Y p := by
    intro p hp hoff
    have hnin : M.side b.grip (M.region Y p) ≠ .inside := fun h => hp ((hholds p hoff).1 h)
    have hreg : M.region s₁ p = M.region Y p := by simp [region, h1off p hoff]
    rw [hs₂, apply_singleton, hreg, h1off p hoff]
    simp [mult, hnin]
  have h2lat : ∀ p, Y p ∈ M.K → s₂ p = s₁ p := by
    intro p hp
    rw [hs₂, apply_singleton]
    simp [mult, h1notT p hp]
  have hsafe₂ : M.SafeWord s₂ q := by
    refine hq.mono fun p hp => ?_
    by_cases hpY : Y p ∈ M.K
    · exact absurd (by rw [h2lat p hpY]; exact h1lat p hpY) hp
    · by_cases hpU : p ∈ U
      · exact absurd (by rw [(h2U p hpU).1]; exact hlands p hpU) hp
      · exact h2off p hpU hpY
  -- the stage `s₃ = s₂·q`
  have h3ret : ∀ p ∉ U, s₃ p = Y p := by
    intro p hp
    by_cases hpY : Y p ∈ M.K
    · rw [hs₃, M.apply_eq_apply_of_eq (h2lat p hpY), hs₁, apply_apply_wordInv]
    · have h2 := h2off p hp hpY
      rw [hs₃, (hsafe₂.apply_off (by rw [h2]; exact hpY)).2, h2]
  have h3U : ∀ p ∈ U, s₃ p ∈ M.K ∧ M.side a.grip (M.region s₃ p) = .outside := by
    intro p hp
    have hK₂ : s₂ p ∈ M.K := by rw [(h2U p hp).1]; exact hlands p hp
    have hK₃ : s₃ p ∈ M.K := (hsafe₂.apply_lattice hK₂).2
    refine ⟨hK₃, ?_⟩
    have hch := M.region_mem_chamber hK₃
    have hnot : M.side a.grip (M.region s₃ p) ≠ .inside := by
      intro hin
      have hmem : M.region s₃ p ∈ M.endRegion q '' ((fun p => b.elem • M.region Y p) '' U) :=
        ⟨M.region s₂ p, ⟨p, hp, (h2U p hp).2.symm⟩, (M.region_apply q s₂ p).symm⟩
      exact Set.disjoint_left.1 hL hmem ⟨hch, hin⟩
    have hns := M.chamber_side _ hch a.grip
    cases hs : M.side a.grip (M.region s₃ p) <;> simp_all
  -- the last letter `a⁻¹`
  have hfinal : ∀ p ∉ U, M.apply [a.inv] s₃ p = Z p := by
    intro p hp
    rw [← wordInv_singleton, M.apply_wordInv_eq_iff, hY]
    exact h3ret p hp
  refine ⟨?_, fun p hp => ?_, fun p hp => by rw [hW]; exact hfinal p hp⟩
  · rw [M.admissible_append, M.admissible_append, M.admissible_append, M.admissible_append,
      M.admissible_singleton, M.admissible_singleton, M.admissible_singleton]
    simp only [apply_append, hY]
    refine ⟨⟨⟨⟨ha, hq'.admissible⟩, fun p => ?_⟩, hsafe₂.admissible⟩, fun p => ?_⟩
    · by_cases hpY : Y p ∈ M.K
      · exact M.chamber_side _ (M.region_mem_chamber (h1lat p hpY)) _
      · have hreg : M.region s₁ p = M.region Y p := by simp [region, h1off p hpY]
        rw [hreg]
        exact hb p
    · change M.side a.grip (M.region s₃ p) ≠ .straddle
      by_cases hpU : p ∈ U
      · simp [(h3U p hpU).2]
      · have hreg : M.region s₃ p = M.step a (M.region Z p) := by
          rw [← M.endRegion_singleton, ← M.region_apply, hY]
          simp [region, h3ret p hpU]
        simpa [hreg, side_step] using ha p
  · rw [hW, apply_singleton]
    simp [mult, (h3U p hp).2, (h3U p hp).1]

end NestedVacancy

end PieceModel

end Jumbling
