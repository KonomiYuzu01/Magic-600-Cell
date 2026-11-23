import Jumbling.Model.CleanPowers
import Mathlib.Data.Set.Card

set_option linter.style.header false in
/-!
# Transport without endpoint collateral (R22A-01, R22A-02, R22A-03, R22A-05, R22A-07)

Source: `research/jumbling/restoration/12-low-height/PROOFS-B.md`, section 2.

All results are about words over the free class-P letters of a state `Z`, under LC-03
(`ClassCellModel.LC03`), in the class-cell model of `Jumbling.Model.ClassCell`.

Main results:

* R22A-01: `LC03.ghost_never_straddles` and `LC03.Bad_eq` (item 1),
  `LC03.collateral_eq` and `LC03.collateral_eq_empty_iff` (item 2),
  `ClassCellModel.exists_home_label_ne`, `LC03.exists_home_chambersOf_subset_Bad` and
  `LC03.le_encard_Bad` (item 3, with the cell sizes as a hypothesis).
* R22A-02: `LC03.keeps_content_iff`, `LC03.lattice_stays_iff`,
  `LC03.lattice_content_stays_iff` and `LC03.foreign_apply`.
* R22A-03: `LC03.transport`.
* R22A-05: `ClassCellModel.pointPerm_injective`, `ClassCellModel.cellMap_injective` (item 1),
  `ClassCellModel.capacity` and `LC03.capacity` (item 2), `ClassCellModel.vacant_le` and
  `ClassCellModel.le_vacant` (item 3), `ClassCellModel.pointPerm_ne_of_fixed` (the consequence).
* R22A-07: `LC03.glued`.

Readings and omissions:

* "W lands a content at `p`" is read in the class-cell model as: `W` sends `p` to a label-0 point
  of a home cell. `LC03.apply_mem_K_iff` shows that the label part is exactly the landing of the
  pieces; that the landed cell is a home cell is taken as a hypothesis where it is used.
* R22A-01, item 3: the numbers 111 and 87 are geometric cell sizes; here every home cell of the
  kind has at least `m` chambers, a hypothesis. The remark (M1-03) is not formalized.
* R22A-02: the counterexample to the point rule (LC-12) is a computed geometric fact, and "need
  not be fixed" is a remark; neither is formalized.
* R22A-03: the owned set is any finite set `O` that contains the label-0 points of the cells of
  the lattice pieces and the fibre of the cell of every other class-P off piece (the source's
  `O_U`, which also contains the excluded cells, is one). The proof uses neither `ℓ ≠ 0`, nor
  that `U` is the whole content, nor H3, nor that the target cell is a home cell of `U`'s kind
  with a free fibre, nor step 5 of the source's proof (LC-03 covers the whole word), so the
  theorem is stated without them.
* R22A-07: the SAFE remedy (Theorem C, LC-12, Proposition N) and the computed instance are not
  formalized. The proof does not use that the lattice content `h` is nonempty.
-/

namespace Jumbling

namespace ClassCellModel

variable {Γ : Type*} [Group Γ] {R : Type*} [MulAction Γ R] {Grip Piece : Type*}
variable {M : PieceModel Γ R Grip Piece} {G : Subgroup Γ} {Cell : Type*} [MulAction G Cell]
  {Kind : Type*}

section Model

variable (Q : ClassCellModel M G Cell Kind)

/-- R22A-05, item 1: under a class-P word distinct points stay distinct; two contents that start
at different points never end at one point. -/
theorem pointPerm_injective (w : List Q.Letter) : Function.Injective (Q.pointPerm w) :=
  (Q.pointPerm w).injective

/-- R22A-05, item 1: under a class-P word distinct cells stay distinct. -/
theorem cellMap_injective (w : List Q.Letter) :
    Function.Injective fun C => Q.cellAcc w C • C := by
  intro C D h
  have hC := congrArg Prod.fst (Q.pointMap_wordInv w (C, M.label0 G))
  have hD := congrArg Prod.fst (Q.pointMap_wordInv w (D, M.label0 G))
  simp only [pointMap] at hC hD
  simp only at h
  rw [← hC, ← hD, h]

/-- R22A-05, consequence: if a class-P word keeps the label-0 point of `C` in place, no other
point is sent there. -/
theorem pointPerm_ne_of_fixed (w : List Q.Letter) {C : Cell}
    (hC : Q.pointPerm w (C, M.label0 G) = (C, M.label0 G)) {q : Cell × M.Label G}
    (hq : q ≠ (C, M.label0 G)) : Q.pointPerm w q ≠ (C, M.label0 G) := by
  rw [← hC]
  exact fun h => hq (Q.pointPerm_injective w h)

/-- The label-0 points of the home cells of kind `κ`. -/
def homeZero (κ : Kind) : Set (Cell × M.Label G) :=
  {q | Q.IsHome q.1 ∧ Q.kind q.1 = κ ∧ q.2 = M.label0 G}

/-- A class-P word maps the label-0 point of a home cell whose label-0 it keeps to a label-0
point of a home cell of the same kind. -/
lemma pointPerm_mem_homeZero (w : List Q.Letter) {C : Cell} (hC : Q.IsHome C)
    (h0 : (Q.pointPerm w (C, M.label0 G)).2 = M.label0 G) :
    Q.pointPerm w (C, M.label0 G) ∈ Q.homeZero (Q.kind C) := by
  have hK : (Q.cellAcc w C : Γ) ∈ M.K := (M.smul_label0_eq_iff _).1 h0
  exact ⟨hC.smul hK, Q.kind_smul _ _, h0⟩

/-- R22A-01, item 3, the class-cell part: if a class-P word sends a point `(X, ℓ)` with `ℓ ≠ 0`
to a label-0 point of a home cell, some home cell `C` of the kind of `X` has its label-0 point
sent to a nonzero label. -/
theorem exists_home_label_ne [Finite Cell] (w : List Q.Letter) {X : Cell} {ℓ : M.Label G}
    (hℓ : ℓ ≠ M.label0 G) (hland : (Q.pointPerm w (X, ℓ)).2 = M.label0 G)
    (hhome : Q.IsHome (Q.pointPerm w (X, ℓ)).1) :
    ∃ C, Q.IsHome C ∧ Q.kind C = Q.kind X ∧
      (Q.pointPerm w (C, M.label0 G)).2 ≠ M.label0 G := by
  by_contra hcon
  simp only [not_exists, not_and, not_not] at hcon
  set S := Q.homeZero (Q.kind X)
  have hfin : S.Finite :=
    (Set.finite_range fun C : Cell => (C, M.label0 G)).subset fun q hq =>
      ⟨q.1, Prod.ext rfl hq.2.2.symm⟩
  have hmaps : Set.MapsTo (Q.pointPerm w) S S := by
    rintro ⟨C, l⟩ ⟨hC, hk, hl⟩
    simp only at hC hk hl
    subst hl
    have := Q.pointPerm_mem_homeZero w hC (hcon C hC hk)
    rwa [hk] at this
  have hbij := (hfin.injOn_iff_bijOn_of_mapsTo hmaps).1 (Q.pointPerm_injective w).injOn
  have himage : Q.pointPerm w (X, ℓ) ∈ S :=
    ⟨hhome, by simp [Q.kind_smul], hland⟩
  obtain ⟨s, hs, hseq⟩ := hbij.surjOn himage
  have := Q.pointPerm_injective w hseq
  subst this
  exact hℓ hs.2.2

/-- R22A-05, item 2 (capacity), in the class-cell model. Let `Occ` be home cells of kind `κ` (those
that hold lattice pieces) whose label-0 points the word keeps at label 0 (their lattice contents
stay on the lattice), and `Pts` points of kind `κ` with nonzero labels (initially distinct off
contents) that the word lands, at label-0 points of home cells. Then at most
`v_κ = |home cells of kind κ \ Occ|` of them land. -/
theorem capacity [Finite Cell] (w : List Q.Letter) (κ : Kind) {Occ : Set Cell}
    (hOcc : Occ ⊆ {C | Q.IsHome C ∧ Q.kind C = κ})
    (hstay : ∀ C ∈ Occ, (Q.pointPerm w (C, M.label0 G)).2 = M.label0 G)
    {Pts : Set (Cell × M.Label G)}
    (hPts : ∀ q ∈ Pts, Q.kind q.1 = κ ∧ q.2 ≠ M.label0 G ∧
      (Q.pointPerm w q).2 = M.label0 G ∧ Q.IsHome (Q.pointPerm w q).1) :
    Pts.ncard ≤ ({C | Q.IsHome C ∧ Q.kind C = κ} \ Occ).ncard := by
  set S := {C | Q.IsHome C ∧ Q.kind C = κ}
  set Occ0 : Set (Cell × M.Label G) := (fun C => (C, M.label0 G)) '' Occ
  have hS0fin : (Q.homeZero κ).Finite :=
    (Set.finite_range fun C : Cell => (C, M.label0 G)).subset fun q hq =>
      ⟨q.1, Prod.ext rfl hq.2.2.symm⟩
  have hS0card : (Q.homeZero κ).ncard = S.ncard := by
    have : Q.homeZero κ = (fun C => (C, M.label0 G)) '' S := by
      ext ⟨C, l⟩
      simp only [homeZero, Set.mem_image, Set.mem_ofPred_eq, Prod.mk.injEq, S]
      constructor
      · rintro ⟨h1, h2, rfl⟩
        exact ⟨C, ⟨h1, h2⟩, rfl, rfl⟩
      · rintro ⟨D, ⟨h1, h2⟩, rfl, rfl⟩
        exact ⟨h1, h2, rfl⟩
    rw [this, Set.ncard_image_of_injective _ fun C D h => congrArg Prod.fst h]
  have hmaps : ∀ q ∈ Occ0 ∪ Pts, Q.pointPerm w q ∈ Q.homeZero κ := by
    rintro q (⟨C, hC, rfl⟩ | hq)
    · have := Q.pointPerm_mem_homeZero w (hOcc hC).1 (hstay C hC)
      rwa [(hOcc hC).2] at this
    · obtain ⟨hk, -, h0, hh⟩ := hPts q hq
      exact ⟨hh, by simp [Q.kind_smul, hk], h0⟩
  have hle : (Occ0 ∪ Pts).ncard ≤ (Q.homeZero κ).ncard :=
    Set.ncard_le_ncard_of_injOn _ hmaps (Q.pointPerm_injective w).injOn hS0fin
  have hfin : (Occ0 ∪ Pts).Finite :=
    Set.Finite.of_finite_image (hS0fin.subset (Set.image_subset_iff.2 hmaps))
      (Q.pointPerm_injective w).injOn
  have hdisj : Disjoint Occ0 Pts := by
    rw [Set.disjoint_left]
    rintro _ ⟨C, -, rfl⟩ hq
    exact (hPts _ hq).2.1 rfl
  have hOccfin : Occ.Finite := Set.toFinite Occ
  rw [Set.ncard_union_eq hdisj (hfin.subset Set.subset_union_left)
    (hfin.subset Set.subset_union_right),
    Set.ncard_image_of_injective _ fun C D h => congrArg Prod.fst h, hS0card] at hle
  have := Set.ncard_sdiff_add_ncard_of_subset hOcc (Set.toFinite S)
  omega

end Model

section TypeCounts

variable (Q : ClassCellModel M G Cell Kind) [Finite Cell] {T : Type*} [Fintype T] (type : R → T)

omit [Finite Cell] in
lemma chambersOf_subset_chamber (C : Cell) : Q.chambersOf C ⊆ M.chamber := fun _ hx => hx.1

omit [Finite Cell] in
/-- A finite set of chambers counted through a finset of all chambers. -/
private lemma ncard_eq_card_filter {Ωf : Finset R} (hΩf : ∀ x, x ∈ Ωf ↔ x ∈ M.chamber)
    {s : Set R} (hs : s ⊆ M.chamber) [DecidablePred (· ∈ s)] :
    s.ncard = (Ωf.filter (· ∈ s)).card := by
  rw [← Set.ncard_coe_finset]
  congr 1
  ext x
  simp only [Finset.coe_filter, Set.mem_ofPred_eq, hΩf]
  exact ⟨fun hx => ⟨hs hx, hx⟩, fun hx => hx.2⟩

omit [Fintype T] in
/-- R22A-05, item 3, the upper bound: if every home cell of kind `κ` has `b_t` chambers of type
`t`, the number `v_κ` of home cells of kind `κ` whose chambers are all holes is at most
`⌊h_t / b_t⌋` for every type `t` with `b_t > 0`, `h_t` the number of holes of type `t`. -/
theorem vacant_le (hΩ : M.chamber.Finite) (Z : Piece → Γ) (κ : Kind) (b : T → ℕ)
    (hb : ∀ C, Q.IsHome C → Q.kind C = κ → ∀ t, (Q.chambersOf C ∩ {x | type x = t}).ncard = b t)
    (t : T) (ht : 0 < b t) :
    {C | (Q.IsHome C ∧ Q.kind C = κ) ∧ Q.chambersOf C ⊆ M.holes Z}.ncard ≤
      (M.holes Z ∩ {x | type x = t}).ncard / b t := by
  classical
  have := Fintype.ofFinite Cell
  set Ωf := hΩ.toFinset
  have hΩf : ∀ x, x ∈ Ωf ↔ x ∈ M.chamber := fun x => hΩ.mem_toFinset
  set vac := {C | (Q.IsHome C ∧ Q.kind C = κ) ∧ Q.chambersOf C ⊆ M.holes Z}
  set V := Finset.univ.filter (· ∈ vac)
  have hV : vac.ncard = V.card := by
    rw [← Set.ncard_coe_finset]
    congr 1
    ext C
    simp [V]
  set A := Ωf.filter fun x => Q.chamberCell x ∈ V ∧ type x = t
  have hA : A.card = V.card * b t := by
    rw [Finset.card_eq_sum_card_fiberwise (f := Q.chamberCell) (t := V)
      (fun x hx => (Finset.mem_filter.1 hx).2.1)]
    refine Finset.sum_const_nat fun C hC => ?_
    have hCv : C ∈ vac := (Finset.mem_filter.1 hC).2
    rw [← hb C hCv.1.1 hCv.1.2 t,
      ncard_eq_card_filter hΩf (Set.inter_subset_left.trans (Q.chambersOf_subset_chamber C))]
    congr 1
    ext x
    simp only [A, Finset.mem_filter, chambersOf, Set.mem_inter_iff, Set.mem_ofPred_eq, hΩf]
    constructor
    · rintro ⟨⟨hx, -, htx⟩, rfl⟩
      exact ⟨hx, ⟨hx, rfl⟩, htx⟩
    · rintro ⟨hx, ⟨-, rfl⟩, htx⟩
      exact ⟨⟨hx, hC, htx⟩, rfl⟩
  have hsub : A ⊆ Ωf.filter (· ∈ M.holes Z ∩ {x | type x = t}) := by
    intro x hx
    simp only [A, Finset.mem_filter] at hx
    obtain ⟨hxΩ, hxV, htx⟩ := hx
    have hCv : Q.chamberCell x ∈ vac := (Finset.mem_filter.1 hxV).2
    exact Finset.mem_filter.2 ⟨hxΩ, hCv.2 ⟨(hΩf x).1 hxΩ, rfl⟩, htx⟩
  rw [Nat.le_div_iff_mul_le ht, hV, ← hA,
    ncard_eq_card_filter hΩf (Set.inter_subset_left.trans (M.holes_subset_chamber Z))]
  exact Finset.card_le_card hsub

/-- R22A-05, item 3, the lower bound:
`max(0, M_κ − Σ_t min(M_κ·b_t, N_t − h_t)) ≤ v_κ` (natural subtraction is the `max(0, ·)`), where
`M_κ` is the number of home cells of kind `κ`, `N_t` the number of chambers and `h_t` the number
of holes of type `t`. -/
theorem le_vacant (hΩ : M.chamber.Finite) (Z : Piece → Γ) (κ : Kind) (b : T → ℕ)
    (hb : ∀ C, Q.IsHome C → Q.kind C = κ → ∀ t, (Q.chambersOf C ∩ {x | type x = t}).ncard = b t) :
    {C | Q.IsHome C ∧ Q.kind C = κ}.ncard -
        ∑ t, min ({C | Q.IsHome C ∧ Q.kind C = κ}.ncard * b t)
          ((M.chamber ∩ {x | type x = t}).ncard - (M.holes Z ∩ {x | type x = t}).ncard) ≤
      {C | (Q.IsHome C ∧ Q.kind C = κ) ∧ Q.chambersOf C ⊆ M.holes Z}.ncard := by
  classical
  have := Fintype.ofFinite Cell
  set Ωf := hΩ.toFinset
  have hΩf : ∀ x, x ∈ Ωf ↔ x ∈ M.chamber := fun x => hΩ.mem_toFinset
  set Hs := {C | Q.IsHome C ∧ Q.kind C = κ}
  set vac := {C | (Q.IsHome C ∧ Q.kind C = κ) ∧ Q.chambersOf C ⊆ M.holes Z}
  have hvac : vac ⊆ Hs := fun C hC => hC.1
  set Oκ := {x | x ∈ M.chamber ∧ x ∉ M.holes Z ∧ Q.chamberCell x ∈ Hs}
  have hOκ : Oκ ⊆ M.chamber := fun x hx => hx.1
  -- the nonvacant cells inject into the occupied chambers of their kind
  have hN : (Hs \ vac).ncard ≤ Oκ.ncard := by
    have hsub : Hs \ vac ⊆ Q.chamberCell '' Oκ := by
      rintro C ⟨hC, hCv⟩
      have : ¬ Q.chambersOf C ⊆ M.holes Z := fun h => hCv ⟨hC, h⟩
      obtain ⟨x, ⟨hx, rfl⟩, hxh⟩ := Set.not_subset.1 this
      exact ⟨x, ⟨hx, hxh, hC⟩, rfl⟩
    exact (Set.ncard_le_ncard hsub).trans (Set.ncard_image_le (hΩ.subset hOκ))
  -- the occupied chambers of kind `κ`, type by type
  set HsF := Finset.univ.filter (· ∈ Hs)
  have hHs : Hs.ncard = HsF.card := by
    rw [← Set.ncard_coe_finset]
    congr 1
    ext C
    simp [HsF]
  have hOsum : Oκ.ncard = ∑ t, (Ωf.filter (· ∈ Oκ ∩ {x | type x = t})).card := by
    rw [ncard_eq_card_filter hΩf hOκ, Finset.card_eq_sum_card_fiberwise (f := type)
      (t := Finset.univ) (fun _ _ => Finset.mem_coe.2 (Finset.mem_univ _))]
    refine Finset.sum_congr rfl fun t _ => ?_
    congr 1
    ext x
    simp [Set.mem_inter_iff, and_assoc]
  have hterm : ∀ t, (Ωf.filter (· ∈ Oκ ∩ {x | type x = t})).card ≤
      min (Hs.ncard * b t)
        ((M.chamber ∩ {x | type x = t}).ncard - (M.holes Z ∩ {x | type x = t}).ncard) := by
    intro t
    refine le_min ?_ ?_
    · rw [Finset.card_eq_sum_card_fiberwise (f := Q.chamberCell) (t := HsF)
        (fun x hx => by
          simp only [Finset.coe_filter, Set.mem_ofPred_eq] at hx
          simpa [HsF] using hx.2.1.2.2), hHs, ← smul_eq_mul, ← Finset.sum_const]
      refine Finset.sum_le_sum fun C hC => ?_
      have hCs : C ∈ Hs := (Finset.mem_filter.1 hC).2
      rw [← hb C hCs.1 hCs.2 t,
        ncard_eq_card_filter hΩf (Set.inter_subset_left.trans (Q.chambersOf_subset_chamber C))]
      refine Finset.card_le_card fun x hx => ?_
      simp only [Finset.mem_filter] at hx ⊢
      exact ⟨hx.1.1, ⟨hx.1.2.1.1, hx.2⟩, hx.1.2.2⟩
    · have hhc : M.holes Z ∩ {x | type x = t} ⊆ M.chamber ∩ {x | type x = t} :=
        Set.inter_subset_inter_left _ (M.holes_subset_chamber Z)
      have hsub : Oκ ∩ {x | type x = t} ⊆
          (M.chamber ∩ {x | type x = t}) \ (M.holes Z ∩ {x | type x = t}) :=
        fun x hx => ⟨⟨hx.1.1, hx.2⟩, fun h => hx.1.2.1 h.1⟩
      have hfin : (M.chamber ∩ {x | type x = t}).Finite := hΩ.subset Set.inter_subset_left
      rw [← ncard_eq_card_filter hΩf (Set.inter_subset_left.trans hOκ),
        ← Set.ncard_sdiff hhc (hfin.subset hhc)]
      exact Set.ncard_le_ncard hsub (hfin.subset Set.sdiff_subset)
  have hsplit := Set.ncard_sdiff_add_ncard_of_subset hvac (Set.toFinite Hs)
  have hsum := Finset.sum_le_sum fun t (_ : t ∈ Finset.univ) => hterm t
  rw [← hOsum] at hsum
  omega

end TypeCounts

variable {Q : ClassCellModel M G Cell Kind} {Z : Piece → Γ}

namespace LC03

variable (B : Q.LC03 Z) {w : List Q.Letter}

/-- A ghost of a home chamber ends on the lattice exactly when its cell's label-0 point is sent to
label 0. -/
lemma ghost_mem_K_iff (hw : B.FreeWord w) {x : R} (hx : x ∈ M.chamber) :
    M.acc (Q.lift w) x ∈ M.K ↔
      (Q.pointPerm w (Q.chamberCell x, M.label0 G)).2 = M.label0 G := by
  rw [B.ghost_acc w hw x hx, pointPerm_apply, M.smul_label0_eq_iff]

/-- R22A-01, item 1: no ghost of a home chamber straddles a cut of `P` (O8, the field
`LC03.ghost_clear`). -/
theorem ghost_never_straddles (hw : B.FreeWord w) : ∀ x ∈ M.chamber, M.Clear (Q.lift w) x :=
  B.ghost_clear w hw

/-- R22A-01, item 1: `Bad(W)` is the union of `Ω_C` over the home cells `C` whose label-0 point
`W` sends to a point of nonzero label. (That no ghost of a home chamber straddles a cut of `P` is
`LC03.ghost_never_straddles`.) -/
theorem Bad_eq (hw : B.FreeWord w) :
    M.Bad (Q.lift w) = ⋃ C ∈ {C | Q.IsHome C ∧
      (Q.pointPerm w (C, M.label0 G)).2 ≠ M.label0 G}, Q.chambersOf C := by
  ext x
  simp only [PieceModel.Bad, Set.mem_ofPred_eq, Set.mem_iUnion, chambersOf, exists_prop]
  constructor
  · rintro ⟨hx, hbad⟩
    have hbad' : M.acc (Q.lift w) x ∉ M.K := hbad.resolve_left (not_not.2 (B.ghost_clear w hw x hx))
    exact ⟨Q.chamberCell x, ⟨⟨x, hx, rfl⟩, fun h => hbad' ((B.ghost_mem_K_iff hw hx).2 h)⟩,
      hx, rfl⟩
  · rintro ⟨C, ⟨-, hC⟩, hx, rfl⟩
    exact ⟨hx, Or.inr fun h => hC ((B.ghost_mem_K_iff hw hx).1 h)⟩

/-- R22A-01, item 2: `C_Z(W)` is the set of lattice pieces of `Z` at chambers of `Bad(W)`. -/
theorem collateral_eq (hw : B.FreeWord w) :
    M.collateral Z (Q.lift w) = {p | Z p ∈ M.K ∧ M.region Z p ∈ M.Bad (Q.lift w)} :=
  M.collateral_eq (B.admissible w hw)

/-- R22A-01, item 2: `C_Z(W) = ∅` exactly when `Bad(W) ⊆ holes(Z)`. -/
theorem collateral_eq_empty_iff (hw : B.FreeWord w) :
    M.collateral Z (Q.lift w) = ∅ ↔ M.Bad (Q.lift w) ⊆ M.holes Z :=
  M.collateral_eq_empty_iff (B.admissible w hw)

/-- If `W` sends the label-0 point of a cell to a nonzero label, all its home chambers are Bad. -/
lemma chambersOf_subset_Bad (hw : B.FreeWord w) {C : Cell}
    (hC : (Q.pointPerm w (C, M.label0 G)).2 ≠ M.label0 G) :
    Q.chambersOf C ⊆ M.Bad (Q.lift w) := by
  rintro x ⟨hx, rfl⟩
  exact ⟨hx, Or.inr fun h => hC ((B.ghost_mem_K_iff hw hx).1 h)⟩

/-- R22A-01, item 3: if `W` lands the content at `(X, ℓ)`, `ℓ ≠ 0` (it sends that point to a
label-0 point of a home cell), then `Bad(W)` contains `Ω_C` for a home cell `C` of the kind of
`X`. -/
theorem exists_home_chambersOf_subset_Bad [Finite Cell] (hw : B.FreeWord w) {X : Cell}
    {ℓ : M.Label G} (hℓ : ℓ ≠ M.label0 G) (hland : (Q.pointPerm w (X, ℓ)).2 = M.label0 G)
    (hhome : Q.IsHome (Q.pointPerm w (X, ℓ)).1) :
    ∃ C, Q.IsHome C ∧ Q.kind C = Q.kind X ∧ Q.chambersOf C ⊆ M.Bad (Q.lift w) := by
  obtain ⟨C, hC, hk, hne⟩ := Q.exists_home_label_ne w hℓ hland hhome
  exact ⟨C, hC, hk, B.chambersOf_subset_Bad hw hne⟩

/-- R22A-01, item 3, with the cell sizes as a hypothesis: if every home cell of the kind of `X`
has at least `m` chambers (111 for a triangle cell and 87 for a tetrahedron cell in the source),
`Bad(W)` has at least `m` chambers, even when they are all holes. -/
theorem le_encard_Bad [Finite Cell] (hw : B.FreeWord w) {X : Cell} {ℓ : M.Label G}
    (hℓ : ℓ ≠ M.label0 G) (hland : (Q.pointPerm w (X, ℓ)).2 = M.label0 G)
    (hhome : Q.IsHome (Q.pointPerm w (X, ℓ)).1) (m : ℕ∞)
    (hm : ∀ C, Q.IsHome C → Q.kind C = Q.kind X → m ≤ (Q.chambersOf C).encard) :
    m ≤ (M.Bad (Q.lift w)).encard := by
  obtain ⟨C, hC, hk, hsub⟩ := B.exists_home_chambersOf_subset_Bad hw hℓ hland hhome
  exact (hm C hC hk).trans (Set.encard_le_encard hsub)

/-- The piece `p` of `Z` sits at the point `(C, ℓ)`: it is a class-P off piece of the cell `C`
with pose `h·k`, `h ∈ G_P` of label `ℓ` and `k ∈ K⁺` (O7). -/
def AtPoint (p : Piece) (q : Cell × M.Label G) : Prop :=
  B.Own p q.1 ∧ ∃ h : G, ∃ k ∈ M.K, Z p = (h : Γ) * k ∧ (h : M.Label G) = q.2

/-- A piece at `(C, ℓ)` lands under `W` exactly when `W` sends `(C, ℓ)` to label 0. -/
lemma apply_mem_K_iff (hw : B.FreeWord w) {p : Piece} {q : Cell × M.Label G}
    (hp : B.AtPoint p q) : M.apply (Q.lift w) Z p ∈ M.K ↔ (Q.pointPerm w q).2 = M.label0 G := by
  obtain ⟨hown, h, k, hk, hZ, hℓ⟩ := hp
  rw [B.own_apply w hw p q.1 hown, hZ, ← mul_assoc, M.K.mul_mem_cancel_right hk,
    pointPerm_apply, ← hℓ, MulAction.Quotient.smul_coe, smul_eq_mul, M.coe_eq_label0_iff,
    Subgroup.coe_mul]

/-- R22A-02: `W` keeps the exact pose of every piece of a nonempty class-P off content `Qc` of
the cell `C` if and only if it fixes all five points `(C, 0), …, (C, 4)`. -/
theorem keeps_content_iff (hw : B.FreeWord w) {C : Cell} {Qc : Set Piece} (hQ : Qc.Nonempty)
    (hown : ∀ p ∈ Qc, B.Own p C) :
    (∀ p ∈ Qc, M.apply (Q.lift w) Z p = Z p) ↔
      ∀ ℓ : M.Label G, Q.pointPerm w (C, ℓ) = (C, ℓ) := by
  constructor
  · intro h
    obtain ⟨p, hp⟩ := hQ
    have h1 := h p hp
    rw [B.own_apply w hw p C (hown p hp)] at h1
    have h2 : (Q.cellAcc w C : Γ) = 1 := mul_right_cancel (h1.trans (one_mul _).symm)
    have h3 : Q.cellAcc w C = 1 := OneMemClass.coe_eq_one.1 h2
    intro ℓ
    simp [h3]
  · intro h p hp
    rw [B.own_apply w hw p C (hown p hp), B.cellAcc_eq_one w h]
    simp

/-- R22A-02: a lattice piece of `Z` in the home cell `C` stays on the lattice if and only if `W`
sends `(C, 0)` to a point of label 0. -/
theorem lattice_stays_iff (hw : B.FreeWord w) {p : Piece} (hp : Z p ∈ M.K) :
    M.apply (Q.lift w) Z p ∈ M.K ↔
      (Q.pointPerm w (Q.chamberCell (M.region Z p), M.label0 G)).2 = M.label0 G := by
  rw [M.apply_mem_K_iff hp, B.ghost_mem_K_iff hw (M.region_mem_chamber hp)]

/-- R22A-02: a nonempty lattice content of a home cell `C` stays on the lattice if and only if
`W` sends `(C, 0)` to a point of label 0. -/
theorem lattice_content_stays_iff (hw : B.FreeWord w) {C : Cell}
    (hne : ∃ p, Z p ∈ M.K ∧ Q.chamberCell (M.region Z p) = C) :
    (∀ p, Z p ∈ M.K → Q.chamberCell (M.region Z p) = C → M.apply (Q.lift w) Z p ∈ M.K) ↔
      (Q.pointPerm w (C, M.label0 G)).2 = M.label0 G := by
  constructor
  · intro h
    obtain ⟨p, hp, rfl⟩ := hne
    exact (B.lattice_stays_iff hw hp).1 (h p hp rfl)
  · rintro h p hp rfl
    exact (B.lattice_stays_iff hw hp).2 h

/-- R22A-03, the transport lemma. Let `O` contain the label-0 points of the cells of the lattice
pieces of `Z` and every point of the cell of every class-P off piece outside `U`, let the pieces
of `U` sit at `(X, ℓ)`, and let `W = A_0^{L_0} ⋯ A_{m−1}^{L_{m−1}}` be a path of clean powers of
commutators at free grips that sends `(X, ℓ)` to a label-0 point `(H, 0)`. Then `W` has
`4·Σ L_i` letters, it is admissible at `Z`, it is a strict step for `U`, and `C_Z(W) = ∅`. -/
theorem transport {U : Set Piece} {X : Cell} {ℓ : M.Label G} (O : Finset (Cell × M.Label G))
    (hU : ∀ p ∈ U, B.AtPoint p (X, ℓ))
    (hOlat : ∀ p, Z p ∈ M.K → (Q.chamberCell (M.region Z p), M.label0 G) ∈ O)
    (hOown : ∀ p ∉ U, Z p ∉ M.K → ∀ C, B.Own p C → ∀ ℓ' : M.Label G, (C, ℓ') ∈ O)
    (path : List (Q.Letter × Q.Letter))
    (hfree : ∀ xy ∈ path, xy.1.grip ∈ B.free ∧ xy.2.grip ∈ B.free)
    {H : Cell} (htarget : Q.pointPerm (Q.pathWord O path) (X, ℓ) = (H, M.label0 G)) :
    (Q.lift (Q.pathWord O path)).length = (path.map fun xy =>
      4 * CleanPowers.cleanExp O (Q.pointPerm (Q.comm xy.1 xy.2))).sum ∧
    M.IsStrict Z U (Q.lift (Q.pathWord O path)) ∧
    M.collateral Z (Q.lift (Q.pathWord O path)) = ∅ := by
  set W := Q.pathWord O path
  have hw : B.FreeWord W := by
    intro a ha
    obtain ⟨xy, hxy, hg⟩ := Q.mem_pathWord O ha
    rcases hg with hg | hg <;> rw [hg]
    · exact (hfree xy hxy).1
    · exact (hfree xy hxy).2
  have hfix : ∀ q ∈ O, Q.pointPerm W q = q := Q.pathWord_fixes O path
  have hlat : ∀ p, Z p ∈ M.K → M.apply (Q.lift W) Z p ∈ M.K := by
    intro p hp
    rw [B.lattice_stays_iff hw hp, hfix _ (hOlat p hp)]
  refine ⟨by rw [length_lift, length_pathWord], ⟨B.admissible W hw, fun p hp => ?_, hlat,
    fun p hp hoff => ?_⟩, ?_⟩
  · rw [B.apply_mem_K_iff hw (hU p hp), htarget]
  · rcases B.off_cases p hoff with hf | ⟨C, hC⟩
    · exact B.foreign_apply hw hf
    · rw [B.own_apply W hw p C hC,
        B.cellAcc_eq_one W fun ℓ' => hfix _ (hOown p hp hoff C hC ℓ')]
      simp
  · ext p
    simp only [PieceModel.collateral, Set.mem_ofPred_eq, Set.mem_empty_iff_false, iff_false,
      not_and, not_not]
    exact hlat p

/-- R22A-05, item 2 (capacity), for pieces: if `W` keeps every lattice piece of `Z` on the
lattice, at most `v_κ` initially distinct off contents of kind `κ` land, where `v_κ` counts the
home cells of kind `κ` that hold no lattice piece of `Z`. -/
theorem capacity [Finite Cell] (hw : B.FreeWord w) (κ : Kind)
    (hlat : ∀ p, Z p ∈ M.K → M.apply (Q.lift w) Z p ∈ M.K)
    {Pts : Set (Cell × M.Label G)}
    (hPts : ∀ q ∈ Pts, Q.kind q.1 = κ ∧ q.2 ≠ M.label0 G ∧
      (Q.pointPerm w q).2 = M.label0 G ∧ Q.IsHome (Q.pointPerm w q).1) :
    Pts.ncard ≤ ({C | Q.IsHome C ∧ Q.kind C = κ} \
      {C | ∃ p, Z p ∈ M.K ∧ Q.chamberCell (M.region Z p) = C}).ncard := by
  have := Q.capacity w κ (Occ := {C | Q.IsHome C ∧ Q.kind C = κ} ∩
      {C | ∃ p, Z p ∈ M.K ∧ Q.chamberCell (M.region Z p) = C}) Set.inter_subset_left
    (by
      rintro C ⟨-, p, hp, rfl⟩
      exact (B.lattice_stays_iff hw hp).1 (hlat p hp)) hPts
  rwa [Set.sdiff_self_inter] at this

/-- R22A-07, glued contents: if a class-P off piece `u` and the lattice pieces of a cell `X` lie
in the same cell at `Z`, every free class-P word that lands `u` takes every lattice piece of `X`
off the lattice, and `Ω_X ⊆ Bad(W)`. -/
theorem glued (hw : B.FreeWord w) {X : Cell} {u : Piece} (hu : B.Own u X) (hoff : Z u ∉ M.K)
    (hland : M.apply (Q.lift w) Z u ∈ M.K) :
    (∀ p, Z p ∈ M.K → Q.chamberCell (M.region Z p) = X → M.apply (Q.lift w) Z p ∉ M.K) ∧
      Q.chambersOf X ⊆ M.Bad (Q.lift w) := by
  rw [B.own_apply w hw u X hu] at hland
  have hnot : (Q.cellAcc w X : Γ) ∉ M.K := by
    intro ha
    exact hoff ((M.K.mul_mem_cancel_left ha).1 hland)
  refine ⟨fun p hp hpX => ?_, fun x hx => ⟨hx.1, Or.inr ?_⟩⟩
  · rw [M.apply_mem_K_iff hp, B.ghost_acc w hw _ (M.region_mem_chamber hp), hpX]
    exact hnot
  · rw [B.ghost_acc w hw _ hx.1, hx.2]
    exact hnot

end LC03

end ClassCellModel

end Jumbling
