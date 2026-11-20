import ArchonFull600Theory.Basic
import Lean.Data.Json.Parser
import Lean.Elab.Command
import Mathlib.GroupTheory.Abelianization.Defs
import Mathlib.GroupTheory.SpecificGroups.Alternating
import Mathlib.GroupTheory.SpecificGroups.Dihedral

set_option linter.style.header false in
/-!
# The census table of the retained model

Source: `research/Full_600cell_Puzzle_Theory.tex`, hypothesis R (lines 776 to 836) and Lemma 8
(lines 1006 to 1081); handoff `research/ARCHON-HANDOFF-THEORY.md`, O9, N4, N5 and N11.

The 35 rows of the table (rank, stickers per piece `k_o`, pieces `N_o`, frame group `H_o`) are an
input, not a derivation. They are typed in as `censusRows`, and the `run_cmd` below reads
`assets/census.json` from the repository whenever this module is compiled and stops the build
unless every row equals the file's `rank`, `colors`, `pieces`, `stickers = colors · pieces`,
`orientation_order` and `orientation_group`, and the file's totals (`pieces`, `stickers`,
`fixed_centers`, `moving_orbits`) equal the source's.

The frame group types name abstract groups: `C₂`, `C₅` (cyclic), `D₅ = DihedralGroup 5`,
`A₅ = alternatingGroup (Fin 5)` and the trivial group. Their orders and the orders of their
abelianizations (`C₂ → C₂`, `C₅ → C₅`, `D₅ → C₂`, `A₅ → 1`) are proved here, the last two through
N5 (`dihedralFiveCommutatorCard`, `alternatingFivePerfect`).
-/

namespace ArchonFull600Theory

open scoped BigOperators

/-! ### Frame group types -/

/-- The frame group types of the census table. -/
inductive FrameGroupType
  | trivial
  | c2
  | c5
  | d5
  | a5
  deriving DecidableEq, Repr

namespace FrameGroupType

/-- The abstract group of each type. -/
def Carrier : FrameGroupType → Type
  | trivial => Multiplicative (ZMod 1)
  | c2 => Multiplicative (ZMod 2)
  | c5 => Multiplicative (ZMod 5)
  | d5 => DihedralGroup 5
  | a5 => ↥(alternatingGroup (Fin 5))

instance instGroup : (t : FrameGroupType) → Group t.Carrier
  | trivial => inferInstanceAs (Group (Multiplicative (ZMod 1)))
  | c2 => inferInstanceAs (Group (Multiplicative (ZMod 2)))
  | c5 => inferInstanceAs (Group (Multiplicative (ZMod 5)))
  | d5 => inferInstanceAs (Group (DihedralGroup 5))
  | a5 => inferInstanceAs (Group ↥(alternatingGroup (Fin 5)))

instance instFinite : (t : FrameGroupType) → Finite t.Carrier
  | trivial => inferInstanceAs (Finite (Multiplicative (ZMod 1)))
  | c2 => inferInstanceAs (Finite (Multiplicative (ZMod 2)))
  | c5 => inferInstanceAs (Finite (Multiplicative (ZMod 5)))
  | d5 => inferInstanceAs (Finite (DihedralGroup 5))
  | a5 => inferInstanceAs (Finite ↥(alternatingGroup (Fin 5)))

/-- `|H|` for each type. -/
def order : FrameGroupType → ℕ
  | trivial => 1
  | c2 => 2
  | c5 => 5
  | d5 => 10
  | a5 => 60

/-- `|H^ab|` for each type. -/
def abOrder : FrameGroupType → ℕ
  | trivial => 1
  | c2 => 2
  | c5 => 5
  | d5 => 2
  | a5 => 1

/-- The type names used in the `orientation_group` field of `assets/census.json`. -/
def jsonName : FrameGroupType → String
  | trivial => "trivial"
  | c2 => "C2"
  | c5 => "C5"
  | d5 => "D5 (order 10)"
  | a5 => "A5 (order 60)"

theorem natCard_multiplicative_zmod (n : ℕ) [NeZero n] :
    Nat.card (Multiplicative (ZMod n)) = n :=
  (Nat.card_congr Multiplicative.toAdd).trans (Nat.card_zmod n)

theorem card_carrier (t : FrameGroupType) : Nat.card t.Carrier = t.order := by
  cases t
  · exact natCard_multiplicative_zmod 1
  · exact natCard_multiplicative_zmod 2
  · exact natCard_multiplicative_zmod 5
  · exact DihedralGroup.nat_card
  · change Nat.card ↥(alternatingGroup (Fin 5)) = 60
    rw [Nat.card_eq_fintype_card, card_alternatingGroup]
    decide

theorem natCard_abelianization_of_comm (H : Type*) [CommGroup H] :
    Nat.card (Group.Abelianization H) = Nat.card H :=
  (Nat.card_congr (Group.Abelianization.equivOfComm (H := H)).toEquiv).symm

theorem natCard_abelianization (H : Type*) [Group H] :
    Nat.card (Group.Abelianization H) = (commutator H).index := rfl

/-- **N5 at the level of `H^ab`.** `|C₂^ab| = 2`, `|C₅^ab| = 5`, `|D₅^ab| = 2`, `|A₅^ab| = 1`. -/
theorem card_abelianization (t : FrameGroupType) :
    Nat.card (Group.Abelianization t.Carrier) = t.abOrder := by
  cases t
  · exact (natCard_abelianization_of_comm (Multiplicative (ZMod 1))).trans
      (natCard_multiplicative_zmod 1)
  · exact (natCard_abelianization_of_comm (Multiplicative (ZMod 2))).trans
      (natCard_multiplicative_zmod 2)
  · exact (natCard_abelianization_of_comm (Multiplicative (ZMod 5))).trans
      (natCard_multiplicative_zmod 5)
  · change Nat.card (Group.Abelianization (DihedralGroup 5)) = 2
    have h := Subgroup.card_mul_index (commutator (DihedralGroup 5))
    rw [dihedralFiveCommutatorCard.2, DihedralGroup.nat_card] at h
    rw [natCard_abelianization]
    omega
  · change Nat.card (Group.Abelianization ↥(alternatingGroup (Fin 5))) = 1
    rw [natCard_abelianization, alternatingFivePerfect.1, Subgroup.index_top]

end FrameGroupType

/-! ### The table -/

/-- One row of the census table: application rank, stickers per piece `k_o`, number of pieces
`N_o`, frame group `H_o`. -/
structure CensusRow where
  rank : ℕ
  stickersPerPiece : ℕ
  pieces : ℕ
  frame : FrameGroupType
  deriving DecidableEq, Repr

/-- **O9.** The 35 moving orbits `O00` to `O34`, in order (source lines 790 to 826). -/
def censusRows : List CensusRow :=
  [⟨1, 2, 1200, .c2⟩, ⟨2, 1, 3600, .trivial⟩, ⟨3, 1, 2400, .trivial⟩, ⟨3, 2, 3600, .c2⟩,
    ⟨4, 1, 7200, .trivial⟩, ⟨4, 1, 7200, .trivial⟩, ⟨4, 5, 720, .d5⟩, ⟨5, 1, 7200, .trivial⟩,
    ⟨5, 2, 3600, .c2⟩, ⟨6, 2, 7200, .trivial⟩, ⟨7, 1, 7200, .trivial⟩, ⟨7, 2, 3600, .c2⟩,
    ⟨8, 1, 7200, .trivial⟩, ⟨8, 1, 7200, .trivial⟩, ⟨8, 2, 7200, .trivial⟩,
    ⟨9, 1, 2400, .trivial⟩, ⟨9, 2, 7200, .trivial⟩, ⟨9, 5, 1440, .c5⟩, ⟨10, 1, 7200, .trivial⟩,
    ⟨10, 1, 7200, .trivial⟩, ⟨10, 2, 7200, .trivial⟩, ⟨11, 1, 7200, .trivial⟩, ⟨11, 2, 3600, .c2⟩,
    ⟨12, 2, 7200, .trivial⟩, ⟨13, 1, 7200, .trivial⟩, ⟨13, 2, 3600, .c2⟩, ⟨14, 1, 7200, .trivial⟩,
    ⟨14, 1, 7200, .trivial⟩, ⟨14, 5, 1440, .c5⟩, ⟨15, 1, 2400, .trivial⟩, ⟨15, 2, 7200, .trivial⟩,
    ⟨16, 1, 7200, .trivial⟩, ⟨17, 2, 3600, .c2⟩, ⟨18, 1, 2400, .trivial⟩, ⟨19, 20, 120, .a5⟩]

theorem censusRows_length : censusRows.length = 35 := rfl

/-- The census row of orbit `o`. -/
def census (o : Fin 35) : CensusRow := censusRows.get (o.cast censusRows_length.symm)

/-! ### The check against `assets/census.json` -/

section JsonCheck

open Lean

/-- Compares one orbit entry of `assets/census.json` with a census row. -/
def checkCensusEntry (i : ℕ) (row : CensusRow) (j : Json) : Except String Unit := do
  let field (name : String) : Except String ℕ := j.getObjValAs? ℕ name
  let id ← field "id"
  unless id == i do throw s!"orbit {i}: id {id}"
  let rank ← field "rank"
  unless rank == row.rank do throw s!"orbit {i}: rank {rank} ≠ {row.rank}"
  let colors ← field "colors"
  unless colors == row.stickersPerPiece do
    throw s!"orbit {i}: colors {colors} ≠ {row.stickersPerPiece}"
  let pieces ← field "pieces"
  unless pieces == row.pieces do throw s!"orbit {i}: pieces {pieces} ≠ {row.pieces}"
  let stickers ← field "stickers"
  unless stickers == row.stickersPerPiece * row.pieces do
    throw s!"orbit {i}: stickers {stickers}"
  let order ← field "orientation_order"
  unless order == row.frame.order do throw s!"orbit {i}: orientation_order {order}"
  let group ← j.getObjValAs? String "orientation_group"
  unless group == row.frame.jsonName do throw s!"orbit {i}: orientation_group {group}"

/-- Compares `assets/census.json` with `censusRows` and with the source's totals. -/
def checkCensusJson (j : Json) : Except String Unit := do
  let total (name : String) (value : ℕ) : Except String Unit := do
    let v ← j.getObjValAs? ℕ name
    unless v == value do throw s!"{name}: {v} ≠ {value}"
  total "pieces" 177120
  total "stickers" 259800
  total "fixed_centers" 600
  total "moving_orbits" 35
  let orbits ← (← j.getObjVal? "orbits").getArr?
  unless orbits.size == censusRows.length do throw s!"{orbits.size} orbit entries"
  for (row, i) in censusRows.zipIdx do
    checkCensusEntry i row orbits[i]!

end JsonCheck

open Lean Elab Command in
run_cmd do
  let file := System.FilePath.mk (← getFileName)
  let some dir := file.parent | throwError "census check: no directory for {file}"
  let path := dir / ".." / ".." / ".." / ".." / "assets" / "census.json"
  let text ← IO.FS.readFile path
  match Json.parse text with
  | .error e => throwError "census check: {path}: {e}"
  | .ok j =>
    match checkCensusJson j with
    | .error e => throwError "census check: {path}: {e}"
    | .ok () => pure ()

/-! ### Decidable checks on the table -/

/-- **HR, the census sums.** `Σ N_o = 176520` and `600 + Σ k_o N_o = 259800`; with the 600
centers there are `177120` pieces. -/
theorem censusSums :
    ∑ o, (census o).pieces = 176520 ∧
      600 + ∑ o, (census o).stickersPerPiece * (census o).pieces = 259800 ∧
      600 + ∑ o, (census o).pieces = 177120 := by
  refine ⟨?_, ?_, ?_⟩ <;> decide +kernel

/-- **N4.** Seven orbits with `C₂`, one with `D₅`, two with `C₅`, one with `A₅`, 24 trivial. -/
theorem frameTypeCounts :
    (Finset.univ.filter fun o => (census o).frame = .c2).card = 7 ∧
      (Finset.univ.filter fun o => (census o).frame = .d5).card = 1 ∧
      (Finset.univ.filter fun o => (census o).frame = .c5).card = 2 ∧
      (Finset.univ.filter fun o => (census o).frame = .a5).card = 1 ∧
      (Finset.univ.filter fun o => (census o).frame = .trivial).card = 24 := by
  refine ⟨?_, ?_, ?_, ?_, ?_⟩ <;> decide +kernel

/-- **N4, the denominator.** `∏_o 2 · |H_o^ab| = 2³⁵ · 2⁷ · 2 · 5² = 2⁴³ · 5²`: thirty-five
positional signs, seven `C₂`, one `D₅` and two `C₅` orientation quotients. -/
theorem invariantOrder : ∏ o, 2 * (census o).frame.abOrder = 2 ^ 43 * 5 ^ 2 := by
  decide +kernel

/-- **N11.** The nine selected orbits `{1, 4, 5, 7, 18, 19, 27, 31, 33}` have one sticker per
piece and `N_o = 600 · m_o` pieces with `m_o = 6` for `O01`, `4` for `O33` and `12` otherwise. -/
theorem selectedOrbitCensus :
    ∀ o : Fin 35, o.val ∈ selectedOrbitIds →
      (census o).stickersPerPiece = 1 ∧
        (census o).pieces = (if o.val = 1 then 3600 else if o.val = 33 then 2400 else 7200) ∧
        (census o).pieces = 600 * (if o.val = 1 then 6 else if o.val = 33 then 4 else 12) := by
  decide +kernel

end ArchonFull600Theory
