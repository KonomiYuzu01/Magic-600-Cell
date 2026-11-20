import ArchonFull600Theory.RetainedModel
import Mathlib.Algebra.Group.MinimalAxioms

set_option linter.style.header false in
/-!
# Lemma 7: transported frames and the ambient wreath product

Source: `research/Full_600cell_Puzzle_Theory.tex`, Lemma 7 (lines 950 to 1004).

`FrameWreath H X` is the wreath product `H^X ⋊ S_X`: a position permutation `p` and a frame
increment `a_i ∈ H` at each source position `i`. Mathlib composes permutations right to left
(`y * x` is `x` followed by `y`), and `FrameWreath` follows it, so that the coordinate map is a
homomorphism of Mathlib groups:

  `(y * x).perm = y.perm * x.perm`, `(y * x).frame i = y.frame (x.perm i) * x.frame i`.

This is the source's chronological law `(p, a)(r, b) = (r ∘ p, c)`, `c_i = a_i b_{p(i)}`, for
`x = (p, a)` applied first and `y = (r, b)` second: the source multiplies frame increments
chronologically (its right-action convention), and `y.frame (x.perm i) * x.frame i` is the
functional form of `a_i` followed by `b_{p(i)}`.

A coherent frame of the moving orbit `O_o` (`CoherentFrames`) labels its stickers by a position
and a reference sticker index, `(i, f) ↦ frame o (i, f)`, so that every legal element sends
`frame o (i, f)` to `frame o (j, a f)` for one `a ∈ H_o`. `HypothesisR.coherentFrames` builds one
from a reference piece and legal transporters, as in the source's proof; the frame groups of
hypothesis R are the return maps on the reference piece. `CoherentFrames.coord` is then the
injective homomorphism `G ↪ ∏_o (H_o^{N_o} ⋊ S_{N_o})` with
`g (frame o (i, f)) = frame o (p(i), a_i f)` (`CoherentFrames.coord_apply`), and
`natCard_ambient` counts the ambient product, `∏_o N_o! |H_o|^{N_o}`.
-/

namespace ArchonFull600Theory

open Equiv Finset

/-! ### The wreath product -/

/-- **The ambient wreath product** `H^X ⋊ S_X`: a position permutation and a frame increment at
each source position. -/
@[ext]
structure FrameWreath (H X : Type*) where
  perm : Perm X
  frame : X → H

namespace FrameWreath

variable {H X : Type*} [Group H]

instance : Mul (FrameWreath H X) :=
  ⟨fun y x => ⟨y.perm * x.perm, fun i => y.frame (x.perm i) * x.frame i⟩⟩

instance : One (FrameWreath H X) := ⟨⟨1, fun _ => 1⟩⟩

instance : Inv (FrameWreath H X) := ⟨fun x => ⟨x.perm⁻¹, fun i => (x.frame (x.perm⁻¹ i))⁻¹⟩⟩

theorem mul_perm (y x : FrameWreath H X) : (y * x).perm = y.perm * x.perm := rfl

theorem mul_frame (y x : FrameWreath H X) (i : X) :
    (y * x).frame i = y.frame (x.perm i) * x.frame i := rfl

theorem one_perm : (1 : FrameWreath H X).perm = 1 := rfl

theorem one_frame (i : X) : (1 : FrameWreath H X).frame i = 1 := rfl

instance : Group (FrameWreath H X) :=
  Group.ofLeftAxioms
    (fun a b c => FrameWreath.ext (mul_assoc a.perm b.perm c.perm)
      (funext fun i => mul_assoc (a.frame (b.perm (c.perm i))) (b.frame (c.perm i)) (c.frame i)))
    (fun a => FrameWreath.ext (one_mul a.perm) (funext fun i => one_mul (a.frame i)))
    (fun a => FrameWreath.ext (inv_mul_cancel a.perm) (funext fun i => by
      change (a.frame (a.perm⁻¹ (a.perm i)))⁻¹ * a.frame i = 1
      rw [perm_inv_apply, inv_mul_cancel]))

/-- The position permutation as a homomorphism. -/
def permHom : FrameWreath H X →* Perm X where
  toFun := perm
  map_one' := rfl
  map_mul' _ _ := rfl

/-- `FrameWreath H X` is `Perm X × (X → H)` as a type. -/
def equivProd : FrameWreath H X ≃ Perm X × (X → H) where
  toFun x := (x.perm, x.frame)
  invFun p := ⟨p.1, p.2⟩
  left_inv _ := rfl
  right_inv _ := rfl

omit [Group H] in
/-- `|H^X ⋊ S_X| = |X|! |H|^|X|`. -/
theorem natCard [Finite X] :
    Nat.card (FrameWreath H X) = (Nat.card X).factorial * Nat.card H ^ Nat.card X := by
  rw [Nat.card_congr equivProd, Nat.card_prod, Nat.card_perm, Nat.card_fun]

end FrameWreath

/-! ### Coherent frames -/

theorem stickersPerPiece_pos (o : Fin 35) : 0 < (census o).stickersPerPiece := by
  revert o
  decide

variable {Sticker Piece : Type*} [Fintype Sticker] [Fintype Piece] [DecidableEq Piece]
  {M : RetainedModel Sticker Piece}

namespace HypothesisR

variable (R : HypothesisR M)

/-- The positions of the moving orbit `O_o`: its pieces. -/
abbrev Position (o : Fin 35) := {A : Piece // R.orbit A = some o}

/-- The stickers of the moving orbit `O_o`. -/
abbrev OrbitSticker (o : Fin 35) := {s : Sticker // R.orbit (M.piece s) = some o}

theorem natCard_position (o : Fin 35) : Nat.card (R.Position o) = (census o).pieces := by
  rw [Nat.card_eq_fintype_card, Fintype.card_subtype, R.card_orbit]

/-- The position permutation of a legal element on `O_o`. -/
noncomputable def posPerm (g : M.legalGroup) (o : Fin 35) : Perm (R.Position o) :=
  (R.pieceAction g).subtypePerm fun A => by rw [R.orbit_pieceAction]

theorem posPerm_val (g : M.legalGroup) (o : Fin 35) (i : R.Position o) :
    (R.posPerm g o i).val = R.pieceAction g i.val := rfl

theorem posPerm_mul (g h : M.legalGroup) (o : Fin 35) :
    R.posPerm (g * h) o = R.posPerm g o * R.posPerm h o :=
  Equiv.ext fun i => Subtype.ext (by
    rw [Perm.mul_apply, posPerm_val, posPerm_val, posPerm_val, map_mul, Perm.mul_apply])

/-- **Coherent transported frames.** For each moving orbit, a labelling of its stickers by a
position and a reference sticker index such that every legal element moves each piece's frame
by one element of the frame group `H_o`. -/
structure CoherentFrames where
  frame : (o : Fin 35) → R.Position o × Fin (census o).stickersPerPiece ≃ R.OrbitSticker o
  piece_frame : ∀ o i f, M.piece (frame o (i, f)).val = i.val
  coherent : ∀ g ∈ M.legalGroup, ∀ o (i : R.Position o), ∃ (j : R.Position o),
    ∃ a ∈ R.frameGroup o, ∀ f, g (frame o (i, f)).val = (frame o (j, a f)).val

/-! #### The construction from transporters -/

theorem refFrame_surjective (o : Fin 35) (s : Sticker) (hs : M.piece s = R.refPiece o) :
    ∃ f, R.refFrame o f = s := by
  classical
  have hsub : univ.image (R.refFrame o) ⊆ univ.filter fun s => M.piece s = R.refPiece o := by
    intro x hx
    obtain ⟨f, -, rfl⟩ := mem_image.1 hx
    exact mem_filter.2 ⟨mem_univ _, R.refFrame_piece o f⟩
  have hcard : (univ.filter fun s => M.piece s = R.refPiece o).card ≤
      (univ.image (R.refFrame o)).card := by
    rw [card_image_of_injective _ (R.refFrame_injective o), card_univ, Fintype.card_fin,
      R.card_stickers _ o (R.refPiece_orbit o)]
  have hs' : s ∈ univ.image (R.refFrame o) :=
    (eq_of_subset_of_card_le hsub hcard) ▸ mem_filter.2 ⟨mem_univ _, hs⟩
  obtain ⟨f, -, hf⟩ := mem_image.1 hs'
  exact ⟨f, hf⟩

/-- A legal transporter from the reference piece of `O_o` to position `i`. -/
noncomputable def transporter (o : Fin 35) (i : R.Position o) : M.legalGroup :=
  ⟨(R.transitive o (R.refPiece o) i.val (R.refPiece_orbit o) i.prop).choose,
    (R.transitive o (R.refPiece o) i.val (R.refPiece_orbit o) i.prop).choose_spec.1⟩

theorem transporter_piece (o : Fin 35) (i : R.Position o) (s : Sticker)
    (hs : M.piece s = R.refPiece o) : M.piece ((R.transporter o i).val s) = i.val :=
  (R.transitive o (R.refPiece o) i.val (R.refPiece_orbit o) i.prop).choose_spec.2 s hs

theorem pieceAction_transporter (o : Fin 35) (i : R.Position o) :
    R.pieceAction (R.transporter o i) (R.refPiece o) = i.val := by
  obtain ⟨s, hs⟩ := R.piece_surjective (R.refPiece o)
  rw [← hs, ← R.piece_legal, R.transporter_piece o i s hs]

theorem transporter_inv_piece (o : Fin 35) (i : R.Position o) (s : Sticker)
    (hs : M.piece s = i.val) : M.piece ((R.transporter o i)⁻¹.val s) = R.refPiece o := by
  apply (R.pieceAction (R.transporter o i)).injective
  rw [← R.piece_legal, Subgroup.coe_inv, perm_apply_inv, hs, R.pieceAction_transporter]

/-- The transported frame: `(i, f) ↦ t_i (refFrame f)`. -/
noncomputable def frameFun (o : Fin 35) (p : R.Position o × Fin (census o).stickersPerPiece) :
    R.OrbitSticker o :=
  ⟨(R.transporter o p.1).val (R.refFrame o p.2), by
    rw [R.transporter_piece o p.1 _ (R.refFrame_piece o p.2)]
    exact p.1.prop⟩

theorem frameFun_piece (o : Fin 35) (p : R.Position o × Fin (census o).stickersPerPiece) :
    M.piece (R.frameFun o p).val = p.1.val :=
  R.transporter_piece o p.1 _ (R.refFrame_piece o p.2)

theorem frameFun_bijective (o : Fin 35) : Function.Bijective (R.frameFun o) := by
  refine ⟨fun p q h => ?_, fun s => ?_⟩
  · have hpos : p.1 = q.1 := Subtype.ext (by
      rw [← R.frameFun_piece o p, ← R.frameFun_piece o q, h])
    obtain ⟨i, f⟩ := p
    obtain ⟨j, f'⟩ := q
    simp only at hpos
    subst hpos
    have h' : (R.transporter o i).val (R.refFrame o f) =
        (R.transporter o i).val (R.refFrame o f') := congrArg Subtype.val h
    rw [R.refFrame_injective o ((R.transporter o i).val.injective h')]
  · let i : R.Position o := ⟨M.piece s.val, s.prop⟩
    obtain ⟨f, hf⟩ := R.refFrame_surjective o _ (R.transporter_inv_piece o i s.val rfl)
    refine ⟨(i, f), Subtype.ext ?_⟩
    change (R.transporter o i).val (R.refFrame o f) = s.val
    rw [hf, Subgroup.coe_inv, perm_apply_inv]

/-- **Lemma 7, coherent coordinates.** The transported frames are coherent: for a legal `g`
taking position `i` to `j`, the return map `t_j⁻¹ g t_i` fixes the reference piece, so by
hypothesis R it acts by an element of `H_o`. -/
noncomputable def coherentFrames : CoherentFrames R where
  frame o := Equiv.ofBijective _ (R.frameFun_bijective o)
  piece_frame o i f := R.frameFun_piece o (i, f)
  coherent := by
    intro g hg o i
    let j : R.Position o := ⟨R.pieceAction ⟨g, hg⟩ i.val, by rw [R.orbit_pieceAction]; exact i.prop⟩
    let u : M.legalGroup := (R.transporter o j)⁻¹ * ⟨g, hg⟩ * R.transporter o i
    have hu : ∀ f, M.piece (u.val (R.refFrame o f)) = R.refPiece o := by
      intro f
      apply R.transporter_inv_piece o j
      change M.piece (g ((R.transporter o i).val (R.refFrame o f))) = _
      rw [show g = (⟨g, hg⟩ : M.legalGroup).val from rfl, R.piece_legal,
        R.transporter_piece o i _ (R.refFrame_piece o f)]
    obtain ⟨a, ha, hua⟩ := R.stabilizer_mem o u.val u.prop hu
    refine ⟨j, a, ha, fun f => ?_⟩
    change g ((R.transporter o i).val (R.refFrame o f)) =
      (R.transporter o j).val (R.refFrame o (a f))
    rw [← hua f]
    change g _ = (R.transporter o j).val
      (((R.transporter o j).val)⁻¹ (g ((R.transporter o i).val (R.refFrame o f))))
    rw [perm_apply_inv]

end HypothesisR

/-! ### The coordinate homomorphism -/

namespace HypothesisR.CoherentFrames

variable {R : HypothesisR M} (CF : R.CoherentFrames)

theorem frame_injective (o : Fin 35) {p q} (h : (CF.frame o p).val = (CF.frame o q).val) :
    p = q :=
  (CF.frame o).injective (Subtype.ext h)

theorem increment_exists (g : M.legalGroup) (o : Fin 35) (i : R.Position o) :
    ∃ a ∈ R.frameGroup o, ∀ f,
      g.val (CF.frame o (i, f)).val = (CF.frame o (R.posPerm g o i, a f)).val := by
  obtain ⟨j, a, ha, h⟩ := CF.coherent g.val g.prop o i
  refine ⟨a, ha, ?_⟩
  have hj : j = R.posPerm g o i := by
    let f₀ : Fin (census o).stickersPerPiece := ⟨0, stickersPerPiece_pos o⟩
    apply Subtype.ext
    rw [R.posPerm_val, ← CF.piece_frame o j (a f₀), ← h f₀, R.piece_legal, CF.piece_frame]
  rw [← hj]
  exact h

/-- The frame increment `a_i ∈ H_o` of a legal element at the source position `i`. -/
noncomputable def increment (g : M.legalGroup) (o : Fin 35) (i : R.Position o) :
    R.frameGroup o :=
  ⟨(CF.increment_exists g o i).choose, (CF.increment_exists g o i).choose_spec.1⟩

theorem apply_frame (g : M.legalGroup) (o : Fin 35) (i : R.Position o) (f) :
    g.val (CF.frame o (i, f)).val =
      (CF.frame o (R.posPerm g o i, (CF.increment g o i : Perm _) f)).val :=
  (CF.increment_exists g o i).choose_spec.2 f

theorem increment_eq (g : M.legalGroup) (o : Fin 35) (i : R.Position o) (a : R.frameGroup o)
    (h : ∀ f, g.val (CF.frame o (i, f)).val = (CF.frame o (R.posPerm g o i, (a : Perm _) f)).val) :
    CF.increment g o i = a :=
  Subtype.ext (Equiv.ext fun f =>
    congrArg Prod.snd (CF.frame_injective o ((CF.apply_frame g o i f).symm.trans (h f))))

/-- **Lemma 7, the coordinates.** The position permutation and the frame increments of a legal
element in each moving orbit. -/
noncomputable def coordFun (g : M.legalGroup) :
    ∀ o : Fin 35, FrameWreath (R.frameGroup o) (R.Position o) :=
  fun o => ⟨R.posPerm g o, CF.increment g o⟩

/-- **Lemma 7.** The coordinates form a homomorphism into the ambient product. -/
noncomputable def coord :
    M.legalGroup →* ∀ o : Fin 35, FrameWreath (R.frameGroup o) (R.Position o) :=
  MonoidHom.mk' CF.coordFun fun g h => funext fun o => FrameWreath.ext (R.posPerm_mul g h o)
    (funext fun i => by
      change CF.increment (g * h) o i = CF.increment g o (R.posPerm h o i) * CF.increment h o i
      apply CF.increment_eq
      intro f
      rw [Subgroup.coe_mul, Perm.mul_apply, CF.apply_frame h, CF.apply_frame g, R.posPerm_mul,
        Subgroup.coe_mul, Perm.mul_apply, Perm.mul_apply])

/-- **Lemma 7, the defining equation:** `g (frame o (i, f)) = frame o (p(i), a_i f)`. -/
theorem coord_apply (g : M.legalGroup) (o : Fin 35) (i : R.Position o) (f) :
    g.val (CF.frame o (i, f)).val =
      (CF.frame o ((CF.coord g o).perm i, ((CF.coord g o).frame i : Perm _) f)).val :=
  CF.apply_frame g o i f

/-- **Lemma 7, injectivity.** Trivial coordinates fix every moving sticker, and hypothesis R
fixes the centres. -/
theorem coord_injective : Function.Injective CF.coord := by
  rw [injective_iff_map_eq_one]
  intro g hg
  apply Subtype.ext
  apply Equiv.ext
  intro s
  change g.val s = s
  cases ho : R.orbit (M.piece s) with
  | none => exact R.fixes_center g s ho
  | some o =>
    obtain ⟨⟨i, f⟩, hif⟩ := (CF.frame o).surjective ⟨s, ho⟩
    have hs : s = (CF.frame o (i, f)).val := by rw [hif]
    have hgo : CF.coord g o = 1 := congrFun hg o
    rw [hs, CF.coord_apply, hgo]
    rfl

end HypothesisR.CoherentFrames

namespace HypothesisR

variable (R : HypothesisR M)

theorem natCard_frameGroup (o : Fin 35) : Nat.card (R.frameGroup o) = (census o).frame.order := by
  obtain ⟨e⟩ := R.frameGroup_equiv o
  rw [Nat.card_congr e.toEquiv, FrameGroupType.card_carrier]

/-- **The ambient product** `∏_o (H_o^{N_o} ⋊ S_{N_o})` has order `∏_o N_o! |H_o|^{N_o}`. -/
theorem natCard_ambient :
    Nat.card (∀ o : Fin 35, FrameWreath (R.frameGroup o) (R.Position o)) =
      ∏ o, (census o).pieces.factorial * (census o).frame.order ^ (census o).pieces := by
  rw [Nat.card_pi]
  refine prod_congr rfl fun o _ => ?_
  rw [FrameWreath.natCard, R.natCard_position, R.natCard_frameGroup]

/-- **Lemma 7.** Under R, coherent transported frames give an injective homomorphism
`G ↪ ∏_o (H_o^{N_o} ⋊ S_{N_o})` with `g (frame o (i, f)) = frame o (p(i), a_i f)`; the
multiplication of the ambient product is `FrameWreath.mul_perm` and `FrameWreath.mul_frame`. -/
theorem transportedFrames :
    ∃ CF : R.CoherentFrames,
      ∃ φ : M.legalGroup →* ∀ o : Fin 35, FrameWreath (R.frameGroup o) (R.Position o),
        Function.Injective φ ∧
          ∀ g o i f, g.val (CF.frame o (i, f)).val =
            (CF.frame o ((φ g o).perm i, ((φ g o).frame i : Perm _) f)).val :=
  ⟨R.coherentFrames, R.coherentFrames.coord, R.coherentFrames.coord_injective,
    R.coherentFrames.coord_apply⟩

end HypothesisR

end ArchonFull600Theory
