import ArchonFull600Theory.FrameCoordinates

set_option linter.style.header false in
/-!
# Lemma 8: necessary invariants and the conditional upper bound

Source: `research/Full_600cell_Puzzle_Theory.tex`, hypothesis I (lines 844 to 849) and Lemma 8
(lines 1006 to 1081).

On the wreath product `H^X ⋊ S_X`, the positional sign `sgn p` and the orientation image
`Φ(p, a) = Σ_i χ(a_i)` in the abelianization `H^ab` are homomorphisms (`positionSign`,
`orientation`), and together they map onto `ℤˣ × H^ab` once `|X| ≥ 2` (`invariant_surjective`:
a transposition prescribes the sign, one frame coordinate prescribes the orientation value).
Over the 35 moving orbits the joint invariant `invariantPi` maps the ambient product onto a group
of order `∏_o 2 |H_o^ab| = 2^35 · 2^7 · 2 · 5^2 = 2^43 · 5^2` (`natCard_invariantTarget`, from
the census and the abelianizations of Milestone 1), so its kernel has exactly this index
(`index_invariantKernel`).

`HypothesisI` (I) says that, in some coherent transported coordinates, every chosen primitive has
even positional permutation and trivial orientation image in every moving orbit. Then every legal
word does, `G` embeds in the invariant kernel, and (`invariantBound`)

  `|G| · 2^43 · 5^2 ≤ ∏_o N_o! |H_o|^{N_o}`.

Equality is not asserted.
-/

namespace ArchonFull600Theory

open Equiv Finset

/-! ### The invariants of a wreath product -/

namespace FrameWreath

variable {H X : Type*} [Group H] [Fintype X] [DecidableEq X]

omit [Group H] [Fintype X] [DecidableEq X] in
instance [Finite H] [Finite X] : Finite (FrameWreath H X) := Finite.of_equiv _ equivProd.symm

/-- **The orientation image** `Φ(p, a) = Σ_i χ(a_i)` in `H^ab`, written multiplicatively. -/
def orientation : FrameWreath H X →* Group.Abelianization H where
  toFun x := ∏ i, Group.Abelianization.of (x.frame i)
  map_one' := by simp only [one_frame, map_one, prod_const_one]
  map_mul' y x := by
    simp only [mul_frame, map_mul, prod_mul_distrib]
    rw [Equiv.prod_comp x.perm fun i => Group.Abelianization.of (y.frame i)]

omit [DecidableEq X] in
/-- **The positional sign** `sgn p`. -/
def positionSign : FrameWreath H X →* ℤˣ := Perm.sign.comp permHom

/-- The joint invariant `(sgn p, Φ(p, a))`. -/
def invariant : FrameWreath H X →* ℤˣ × Group.Abelianization H := positionSign.prod orientation

theorem invariant_apply (x : FrameWreath H X) :
    invariant x = (Perm.sign x.perm, orientation x) := rfl

/-- **Lemma 8, independence in one orbit.** With at least two positions, the sign and the
orientation value can be prescribed independently. -/
theorem invariant_surjective (hX : 2 ≤ Fintype.card X) :
    Function.Surjective (invariant (H := H) (X := X)) := by
  rintro ⟨ε, z⟩
  obtain ⟨h, rfl⟩ := Group.Abelianization.surjective_of H z
  obtain ⟨a, b, hab⟩ := Fintype.exists_pair_of_one_lt_card hX
  refine ⟨⟨if ε = 1 then 1 else swap a b, Pi.mulSingle a h⟩, Prod.ext ?_ ?_⟩
  · change Perm.sign (if ε = 1 then 1 else swap a b) = ε
    rcases Int.units_eq_one_or ε with rfl | rfl
    · rw [ite_eq_left rfl, map_one]
    · rw [ite_eq_right (by decide), Perm.sign_swap hab]
  · change ∏ i, Group.Abelianization.of (Pi.mulSingle a h i) = Group.Abelianization.of h
    rw [Fintype.prod_eq_single a fun i hi => by rw [Pi.mulSingle_eq_of_ne hi, map_one],
      Pi.mulSingle_eq_same]

end FrameWreath

/-! ### The joint invariant over the moving orbits -/

section Pi

variable {ι : Type*} [Fintype ι] {H : ι → Type*} [∀ o, Group (H o)] {X : ι → Type*}
  [∀ o, Fintype (X o)] [∀ o, DecidableEq (X o)]

/-- The joint invariant of all orbits. -/
def invariantPi :
    (∀ o, FrameWreath (H o) (X o)) →* ∀ o, ℤˣ × Group.Abelianization (H o) :=
  MonoidHom.pi fun o => FrameWreath.invariant.comp (Pi.evalMonoidHom _ o)

omit [Fintype ι] in
theorem invariantPi_apply (x : ∀ o, FrameWreath (H o) (X o)) (o : ι) :
    invariantPi x o = FrameWreath.invariant (x o) := rfl

omit [Fintype ι] in
/-- **Lemma 8, independence across orbits.** Choices in different orbits are independent. -/
theorem invariantPi_surjective (hX : ∀ o, 2 ≤ Fintype.card (X o)) :
    Function.Surjective (invariantPi (H := H) (X := X)) := fun t =>
  ⟨fun o => (FrameWreath.invariant_surjective (hX o) (t o)).choose,
    funext fun o => (FrameWreath.invariant_surjective (hX o) (t o)).choose_spec⟩

omit [Fintype ι] in
/-- The index of the joint invariant kernel is the order of the joint invariant's target. -/
theorem index_ker_invariantPi (hX : ∀ o, 2 ≤ Fintype.card (X o)) :
    (invariantPi (H := H) (X := X)).ker.index =
      Nat.card (∀ o, ℤˣ × Group.Abelianization (H o)) := by
  rw [Subgroup.index_ker, MonoidHom.range_eq_top.2 (invariantPi_surjective hX),
    Subgroup.card_top]

end Pi

/-! ### The model -/

theorem two_le_pieces (o : Fin 35) : 2 ≤ (census o).pieces := by
  revert o
  decide

variable {Sticker Piece : Type*} [Fintype Sticker] [Fintype Piece] [DecidableEq Piece]
  {M : RetainedModel Sticker Piece}

namespace HypothesisR

variable (R : HypothesisR M)

theorem natCard_abelianization_frameGroup (o : Fin 35) :
    Nat.card (Group.Abelianization (R.frameGroup o)) = (census o).frame.abOrder := by
  obtain ⟨e⟩ := R.frameGroup_equiv o
  rw [Nat.card_congr (MulEquiv.abelianizationCongr e).toEquiv, FrameGroupType.card_abelianization]

/-- **Lemma 8, the order of the invariant group:** `∏_o 2 |H_o^ab| = 2^43 · 5^2`. -/
theorem natCard_invariantTarget :
    Nat.card (∀ o : Fin 35, ℤˣ × Group.Abelianization (R.frameGroup o)) = 2 ^ 43 * 5 ^ 2 := by
  rw [Nat.card_pi, ← invariantOrder]
  refine prod_congr rfl fun o _ => ?_
  rw [Nat.card_prod, Nat.card_eq_fintype_card (α := ℤˣ), Fintype.card_units_int,
    R.natCard_abelianization_frameGroup]

theorem two_le_card_position (o : Fin 35) : 2 ≤ Fintype.card (R.Position o) := by
  rw [← Nat.card_eq_fintype_card, R.natCard_position]
  exact two_le_pieces o

/-- **Lemma 8, the exact index.** The joint invariant kernel has index `2^43 · 5^2` in the
ambient product. -/
theorem index_invariantKernel :
    (invariantPi (H := fun o => ↥(R.frameGroup o)) (X := R.Position)).ker.index =
      2 ^ 43 * 5 ^ 2 := by
  rw [index_ker_invariantPi R.two_le_card_position, R.natCard_invariantTarget]

end HypothesisR

/-- **Hypothesis I: the retained generator-invariant certificate** (source lines 844 to 849). In
the transported coordinates of some coherent frames, each chosen primitive has even positional
permutation separately in every moving orbit and trivial total orientation image in each orbit's
abelianization. -/
structure HypothesisI {M : RetainedModel Sticker Piece} (R : HypothesisR M) where
  frames : R.CoherentFrames
  H_even : ∀ c o, Perm.sign (frames.coord ⟨M.H c, M.H_mem c⟩ o).perm = 1
  H_orientation : ∀ c o, FrameWreath.orientation (frames.coord ⟨M.H c, M.H_mem c⟩ o) = 1
  T_even : ∀ c o, Perm.sign (frames.coord ⟨M.T c, M.T_mem c⟩ o).perm = 1
  T_orientation : ∀ c o, FrameWreath.orientation (frames.coord ⟨M.T c, M.T_mem c⟩ o) = 1

namespace HypothesisI

variable {R : HypothesisR M} (I : HypothesisI R)

/-- Every legal word lies in the kernels of the invariants. -/
theorem invariantPi_coord (g : M.legalGroup) : invariantPi (I.frames.coord g) = 1 := by
  obtain ⟨g, hg⟩ := g
  have hg' : g ∈ Subgroup.closure (Set.range M.H ∪ Set.range M.T) := hg
  induction hg' using Subgroup.closure_induction with
  | mem x hx =>
    rcases hx with ⟨c, rfl⟩ | ⟨c, rfl⟩
    · exact funext fun o => Prod.ext (I.H_even c o) (I.H_orientation c o)
    · exact funext fun o => Prod.ext (I.T_even c o) (I.T_orientation c o)
  | one => exact map_one (invariantPi.comp I.frames.coord)
  | mul x y hx hy ihx ihy =>
    have hx' : x ∈ M.legalGroup := hx
    have hy' : y ∈ M.legalGroup := hy
    have h := map_mul (invariantPi.comp I.frames.coord) ⟨x, hx'⟩ ⟨y, hy'⟩
    rw [MonoidHom.comp_apply, MonoidHom.comp_apply, MonoidHom.comp_apply, ihx hx', ihy hy',
      one_mul] at h
    exact h
  | inv x hx ihx =>
    have hx' : x ∈ M.legalGroup := hx
    have h := map_inv (invariantPi.comp I.frames.coord) ⟨x, hx'⟩
    rw [MonoidHom.comp_apply, MonoidHom.comp_apply, ihx hx', inv_one] at h
    exact h

include I in
/-- **Lemma 8.** Under R and I,
`|G| · 2^43 · 5^2 ≤ ∏_o N_o! |H_o|^{N_o}`, so `|G| ≤ ∏_o N_o! |H_o|^{N_o} / (2^43 · 5^2)`;
the denominator is the exact index of the invariant kernel in the ambient product. -/
theorem invariantBound :
    Nat.card M.legalGroup * (2 ^ 43 * 5 ^ 2) ≤
        ∏ o, (census o).pieces.factorial * (census o).frame.order ^ (census o).pieces ∧
      Nat.card M.legalGroup ≤
        (∏ o, (census o).pieces.factorial * (census o).frame.order ^ (census o).pieces) /
          (2 ^ 43 * 5 ^ 2) ∧
      (invariantPi (H := fun o => ↥(R.frameGroup o)) (X := R.Position)).ker.index =
        2 ^ 43 * 5 ^ 2 := by
  let K := (invariantPi (H := fun o => ↥(R.frameGroup o)) (X := R.Position)).ker
  have hle : Nat.card M.legalGroup ≤ Nat.card K :=
    Nat.card_le_card_of_injective (fun g => ⟨I.frames.coord g, I.invariantPi_coord g⟩)
      fun g h hgh => I.frames.coord_injective (congrArg Subtype.val hgh)
  have hmul := K.card_mul_index
  rw [R.index_invariantKernel, R.natCard_ambient] at hmul
  have hbound : Nat.card M.legalGroup * (2 ^ 43 * 5 ^ 2) ≤
      ∏ o, (census o).pieces.factorial * (census o).frame.order ^ (census o).pieces :=
    hmul ▸ Nat.mul_le_mul_right _ hle
  exact ⟨hbound, (Nat.le_div_iff_mul_le (by norm_num)).2 hbound, R.index_invariantKernel⟩

end HypothesisI

end ArchonFull600Theory
