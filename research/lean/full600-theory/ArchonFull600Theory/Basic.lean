import Mathlib.Basic.Real.Basic
import Mathlib.Data.Finset.Powerset
import Mathlib.Data.Set.Card
import Mathlib.Geometry.Convex.ConvexSpace.Defs
import Mathlib.GroupTheory.Commutator.Basic
import Mathlib.GroupTheory.Perm.List
import Mathlib.GroupTheory.SpecificGroups.Alternating
import Mathlib.GroupTheory.SpecificGroups.Dihedral
import Mathlib.Order.Interval.Finset.Nat

set_option linter.style.header false in
/-!
# Permutation core

Full sticker actions, supported alternating actions, and bounds on reachable colorings
for the Full 600-cell puzzle.
-/

def hello := "world"

namespace ArchonFull600Theory

open scoped BigOperators

/-- The one-based sticker labels used in the finite cycle examples. -/
abbrev StickerLabels (n : ℕ) := ↥(Finset.Icc 1 n)

theorem scopeSameRestriction :
    let one : StickerLabels 6 := ⟨1, by decide⟩
    let two : StickerLabels 6 := ⟨2, by decide⟩
    let three : StickerLabels 6 := ⟨3, by decide⟩
    let four : StickerLabels 6 := ⟨4, by decide⟩
    let five : StickerLabels 6 := ⟨5, by decide⟩
    let six : StickerLabels 6 := ⟨6, by decide⟩
    let u := [one, two, three].formPerm
    let v := [four, five, six].formPerm * u
    u ∈ alternatingGroup (StickerLabels 6) ∧ v ∈ alternatingGroup (StickerLabels 6) ∧
      Set.EqOn u v {one, two, three} ∧ u ≠ v := by
  sorry

theorem scopeVisibleDisjoint :
    let one : StickerLabels 7 := ⟨1, by decide⟩
    let two : StickerLabels 7 := ⟨2, by decide⟩
    let three : StickerLabels 7 := ⟨3, by decide⟩
    let four : StickerLabels 7 := ⟨4, by decide⟩
    let five : StickerLabels 7 := ⟨5, by decide⟩
    let six : StickerLabels 7 := ⟨6, by decide⟩
    let seven : StickerLabels 7 := ⟨7, by decide⟩
    let a := Equiv.swap one two * Equiv.swap five six
    let b := Equiv.swap three four * Equiv.swap six seven
    let displayed : Finset (StickerLabels 7) := {one, two, three, four}
    a.support ∩ displayed = {one, two} ∧ b.support ∩ displayed = {three, four} ∧
      Disjoint (a.support ∩ displayed) (b.support ∩ displayed) ∧
      ¬ Commute a b ∧ b (a five) = seven ∧ a (b five) = six := by
  sorry

theorem disjointFullSupportsCommute {Slot : Type*} [Fintype Slot] [DecidableEq Slot]
    (P Q : Equiv.Perm Slot) (h : Disjoint P.support Q.support) :
    ⇑Q ∘ ⇑P = ⇑P ∘ ⇑Q := by
  sorry

/-- The graph of a permutation on its moved slots. -/
def nonfixedMap {Slot : Type*} (W : Equiv.Perm Slot) : Set (Slot × Slot) :=
  {uv | uv.2 = W uv.1 ∧ W uv.1 ≠ uv.1}

/-- The chronological word `S⁻¹ W S` acts at `x` by `S (W (S⁻¹ x))`. -/
theorem transportNonfixedMap {Slot : Type*} (G : Subgroup (Equiv.Perm Slot)) (W S : G) :
    nonfixedMap (S * W * S⁻¹).val =
      (fun uv : Slot × Slot => (S.val uv.1, S.val uv.2)) '' nonfixedMap W.val := by
  sorry

/-- A legal seed three-cycle, a relocation to the buffer pair and anchor, and setups
covering every nonbuffer destination. -/
structure PureControllerCertificate {Slot : Type*} [DecidableEq Slot]
    (G : Subgroup (Equiv.Perm Slot)) (O : Set Slot) where
  seedA : O
  seedB : O
  seedC : O
  bufferA : O
  bufferB : O
  anchor : O
  seedDistinct : [seedA, seedB, seedC].Nodup
  relocatedDistinct : [bufferA, bufferB, anchor].Nodup
  seed : G
  seed_eq : seed.val = [seedA.val, seedB.val, seedC.val].formPerm
  relocation : G
  relocation_eq :
    relocation.val seedA.val = bufferA.val ∧ relocation.val seedB.val = bufferB.val ∧
      relocation.val seedC.val = anchor.val
  coverage : ∀ X : O, X ≠ bufferA → X ≠ bufferB →
    ∃ S : G, S.val bufferA.val = bufferA.val ∧ S.val bufferB.val = bufferB.val ∧
      S.val anchor.val = X.val

/-- Orbit numbers with pure-controller certificates in the balanced-color bound. -/
def selectedOrbitIds : Finset ℕ := {1, 4, 5, 7, 18, 19, 27, 31, 33}

section

variable {Slot : Type*} [Fintype Slot] [DecidableEq Slot]
variable (G : Subgroup (Equiv.Perm Slot)) (O : ↥selectedOrbitIds → Finset Slot)
variable (hDisjoint : Pairwise fun i j => Disjoint (O i) (O j))
  (hPreserved : ∀ i (g : G) (s : Slot), g.val s ∈ O i ↔ s ∈ O i)
  (hDegree : ∀ i, (O i).card = if i.val = 1 then 3600 else if i.val = 33 then 2400 else 7200)
  (certificates : ∀ i, PureControllerCertificate G (O i : Set Slot))

include hDisjoint hPreserved hDegree certificates in
/-- The product acts as each prescribed alternating permutation on its orbit and fixes
every slot outside the selected orbits. -/
theorem supportedAlternatingActions :
    ∃ ρ : (∀ i, alternatingGroup ↥(O i)) →* G,
      Function.Injective ρ ∧
      (∀ π i (s : O i), (ρ π).val s.val = ((π i).val s).val) ∧
      (∀ π, ((ρ π).val.support : Set Slot) ⊆ ⋃ i, (O i : Set Slot)) := by
  sorry

variable {Color : Type*} (c₀ : Slot → Color)

/-- The fixed-frame colorings obtained from the solved coloring by the legal action. -/
def reachableColorings : Set (Slot → Color) :=
  Set.range fun g : G => c₀ ∘ g.val.symm

/-- The product of the nine balanced-color multinomial counts. -/
def balancedColorCount : ℕ :=
  (Nat.factorial 3600 / Nat.factorial 6 ^ 600) *
    (Nat.factorial 2400 / Nat.factorial 4 ^ 600) *
    (Nat.factorial 7200 / Nat.factorial 12 ^ 600) ^ 7

variable [Fintype Color] [DecidableEq Color]
variable (hPalette : Fintype.card Color = 600)
  (hCopies : ∀ i (color : Color), ((O i).filter (fun s => c₀ s = color)).card =
    if i.val = 1 then 6 else if i.val = 33 then 4 else 12)

set_option linter.unusedFintypeInType false in
include hDisjoint hPreserved hDegree certificates hPalette hCopies in
/-- Each supported witness preserves the solved coloring off the selected orbits at
its macro boundary. -/
theorem balancedColorLowerBound :
    let supportedColorings : Set (Slot → Color) :=
      {x | ∃ g : G, (∀ s, (∀ i, s ∉ O i) → g.val s = s) ∧ x = c₀ ∘ g.val.symm}
    balancedColorCount ≤ supportedColorings.ncard ∧
      balancedColorCount ≤ (reachableColorings G c₀).ncard := by
  sorry

variable {Letter : Type*}

/-- Starting from the identity, each successive letter acts after the earlier letters. -/
def wordPermutation {legal : Subgroup (Equiv.Perm Slot)} (moves : Letter → legal)
    (w : List Letter) : legal :=
  w.foldl (fun g a => moves a * g) 1

/-- The reachable coloring at the end of a word, with the word read from left to right. -/
def wordColoring (moves : Letter → G) (w : List Letter) : reachableColorings G c₀ :=
  let g := wordPermutation moves w
  ⟨c₀ ∘ g.val.symm, ⟨g, rfl⟩⟩

/-- The least word length attaining a reachable coloring from solved. -/
noncomputable def wordDistance (moves : Letter → G)
    (hRealizes : Function.Surjective (wordPermutation moves))
    (x : reachableColorings G c₀) : ℕ := by
  classical
  refine Nat.find (p := fun n => ∃ w : List Letter, w.length = n ∧ wordColoring G c₀ moves w = x) ?_
  obtain ⟨g, hg⟩ := x.property
  obtain ⟨w, hw⟩ := hRealizes g
  refine ⟨w.length, w, rfl, Subtype.ext ?_⟩
  change c₀ ∘ (wordPermutation moves w).val.symm = x.val
  simpa only [hw] using hg

set_option linter.unusedDecidableInType false in
set_option linter.unusedFintypeInType false in
theorem wordBallCount [Fintype Letter] (moves : Letter → G)
    (hRealizes : Function.Surjective (wordPermutation moves))
    (hq : 2 ≤ Fintype.card Letter) (r : ℕ) :
    let q := Fintype.card Letter
    ({x : reachableColorings G c₀ | wordDistance G c₀ moves hRealizes x ≤ r}).ncard ≤
      (∑ j ∈ Finset.range (r + 1), q ^ j) ∧
      (∑ j ∈ Finset.range (r + 1), q ^ j) = (q ^ (r + 1) - 1) / (q - 1) := by
  sorry

theorem balancedColorCountGtPower : 10 ^ 151851 < balancedColorCount := by
  sorry

theorem radius46647Comparison : 1800 ^ 46648 - 1 < 1799 * balancedColorCount := by
  sorry

theorem radius46648CrossCheck : 1799 * balancedColorCount ≤ 1800 ^ 46649 - 1 := by
  sorry

theorem thousandLetterSupportComparison : 1800 ^ 1000 * 10 ^ 148595 < balancedColorCount := by
  sorry

/-- Each of the 600 caps supplies three unit-cost labels for `H`, `T`, and `T⁻¹`. -/
def capLetterPermutation {legal : Subgroup (Equiv.Perm Slot)}
    (H T : Fin 600 → legal) (letter : Fin 600 × Fin 3) : legal :=
  if letter.2 = 0 then H letter.1 else if letter.2 = 1 then T letter.1 else (T letter.1)⁻¹

variable (H T : Fin 600 → G)
variable (hRealizes : Function.Surjective (wordPermutation (capLetterPermutation H T)))

set_option linter.unusedFintypeInType false in
include hDisjoint hPreserved hDegree certificates hPalette hCopies in
theorem wordDistanceLowerBound :
    ∃ x : reachableColorings G c₀,
      46648 ≤ wordDistance G c₀ (capLetterPermutation H T) hRealizes x := by
  sorry

/-- The maximum absolute difference of the two probability laws on a finite event. -/
noncomputable def totalVariation {X : Type*} [Finite X]
    (μ ν : Convexity.StdSimplex ℝ X) : ℝ := by
  classical
  letI := Fintype.ofFinite X
  let events := (Finset.univ : Finset X).powerset
  exact events.sup' (Finset.powerset_nonempty _) fun E =>
    |(∑ x ∈ E, μ.weights x) - (∑ x ∈ E, ν.weights x)|

set_option linter.unusedFintypeInType false in
include hDisjoint hPreserved hDegree certificates hPalette hCopies hRealizes in
theorem totalVariationLowerBound (μ ν : Convexity.StdSimplex ℝ (reachableColorings G c₀))
    (hSupport : ∀ x ∈ μ.weights.support, ∃ w : List (Fin 600 × Fin 3),
      w.length = 1000 ∧ wordColoring G c₀ (capLetterPermutation H T) w = x)
    (hUniform : ∀ x, ν.weights x = 1 / ((reachableColorings G c₀).ncard : ℝ)) :
    1 - ((10 ^ 148595 : ℕ) : ℝ)⁻¹ < totalVariation μ ν := by
  sorry

end

theorem dihedralFiveCommutatorCard :
    commutator (DihedralGroup 5) = Subgroup.zpowers (DihedralGroup.r (1 : ZMod 5)) ∧
      Nat.card (commutator (DihedralGroup 5)) = 5 := by
  sorry

theorem alternatingFivePerfect :
    commutator (alternatingGroup (Fin 5)) = ⊤ ∧
      Nat.card (commutator (alternatingGroup (Fin 5))) = 60 := by
  sorry

/-- In each product the right factor acts first, as in Mathlib's permutation group. -/
theorem threeCycleIdentityChecks (n : ℕ) (hnLow : 4 ≤ n) (hnHigh : n ≤ 9) :
    (∀ A B X Y : StickerLabels n, [A, B, X, Y].Nodup →
      [A, B, X].formPerm * [A, B, Y].formPerm⁻¹ = [A, Y, X].formPerm) ∧
    (∀ A x y z : StickerLabels n, [A, x, y, z].Nodup →
      [A, z, x].formPerm * [A, x, y].formPerm = [x, y, z].formPerm) := by
  sorry

end ArchonFull600Theory
