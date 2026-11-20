import ArchonFull600Theory.Vertices
import Mathlib.Algebra.Star.Unitary

set_option linter.style.header false in
/-!
# The binary icosahedral group

Object O3: the binary icosahedral group `I* = V / 2`, a subgroup of the unit quaternions
`Sp(1) = unitary ℍ` with 120 elements. The closure of `V / 2` under the quaternion product is
certified by kernel-checked multiplication tables for two generators (vertices 8 and 30) together
with a tree of words in these generators that reaches every vertex. The inverse is quaternion
conjugation, checked by a third table.

That this group is the inverse image of the icosahedral rotation group in `Sp(1)` is the external
result E1, which stays a hypothesis (`CoordinateSources` in `Vertices`).
-/

namespace ArchonFull600Theory

open scoped Quaternion Pointwise
open ZPhi Vec4

namespace Vec4

/-- The quaternion product in exact coordinates. -/
def qmul (v w : Vec4) : Vec4 where
  x0 := sub (sub (sub (mul v.x0 w.x0) (mul v.x1 w.x1)) (mul v.x2 w.x2)) (mul v.x3 w.x3)
  x1 := add (add (mul v.x0 w.x1) (mul v.x1 w.x0)) (sub (mul v.x2 w.x3) (mul v.x3 w.x2))
  x2 := add (add (mul v.x0 w.x2) (mul v.x2 w.x0)) (sub (mul v.x3 w.x1) (mul v.x1 w.x3))
  x3 := add (add (mul v.x0 w.x3) (mul v.x3 w.x0)) (sub (mul v.x1 w.x2) (mul v.x2 w.x1))

/-- Twice a vector, in exact coordinates. -/
def dbl (v : Vec4) : Vec4 := ⟨add v.x0 v.x0, add v.x1 v.x1, add v.x2 v.x2, add v.x3 v.x3⟩

/-- Quaternion conjugation in exact coordinates. -/
def conj (v : Vec4) : Vec4 := ⟨v.x0, neg v.x1, neg v.x2, neg v.x3⟩

theorem toQuat_qmul (v w : Vec4) : toQuat (qmul v w) = toQuat v * toQuat w := by
  ext <;>
  simp only [toQuat, qmul, toReal_add, toReal_sub, toReal_mul, Quaternion.re_mul,
    Quaternion.imI_mul, Quaternion.imJ_mul, Quaternion.imK_mul] <;>
  ring

theorem toQuat_dbl (v : Vec4) : toQuat (dbl v) = (2 : ℝ) • toQuat v := by
  ext <;> simp only [toQuat, dbl, toReal_add, Quaternion.re_smul, Quaternion.imI_smul,
    Quaternion.imJ_smul, Quaternion.imK_smul, smul_eq_mul] <;> ring

theorem toQuat_conj (v : Vec4) : toQuat (conj v) = star (toQuat v) := by
  ext <;> simp [toQuat, conj, toReal_neg]

end Vec4

/-! ### Certificates -/

/-- Left multiplication by vertex 8: `v₈ vᵢ = 2 v_{leftTable8[i]}`. -/
def leftTable8 : List ℕ := [
  8, 17, 20, 18, 23, 14, 11, 13, 16, 2, 1, 9, 3, 12, 10, 0, 4, 21, 19, 7, 22, 5, 6, 15, 54, 55,
  56, 46, 47, 45, 26, 24, 25, 75, 76, 77, 89, 88, 87, 37, 36, 38, 33, 35, 34, 68, 67, 66, 85, 86,
  84, 41, 39, 40, 115, 116, 114, 83, 81, 82, 58, 57, 59, 42, 44, 43, 112, 111, 113, 60, 62, 61,
  94, 93, 95, 102, 104, 103, 28, 27, 29, 72, 74, 73, 49, 50, 48, 101, 99, 100, 31, 32, 30, 71, 69,
  70, 53, 52, 51, 97, 96, 98, 117, 119, 118, 80, 79, 78, 90, 91, 92, 106, 107, 105, 110, 108, 109,
  63, 64, 65]

/-- Left multiplication by vertex 30: `v₃₀ vᵢ = 2 v_{leftTable30[i]}`. -/
def leftTable30 : List ℕ := [
  30, 73, 89, 46, 114, 61, 53, 106, 24, 81, 28, 80, 38, 59, 39, 91, 55, 99, 95, 98, 68, 112, 69,
  108, 77, 47, 16, 26, 35, 2, 33, 29, 8, 45, 25, 36, 93, 1, 75, 34, 9, 72, 41, 31, 27, 54, 76, 17,
  83, 94, 62, 13, 82, 43, 4, 104, 67, 10, 51, 49, 11, 86, 97, 92, 52, 78, 116, 19, 102, 48, 7,
  100, 20, 50, 37, 56, 88, 66, 32, 12, 42, 84, 3, 40, 71, 58, 74, 18, 70, 103, 0, 44, 79, 21, 87,
  85, 57, 5, 63, 118, 22, 60, 101, 115, 111, 90, 64, 14, 65, 107, 15, 110, 119, 6, 117, 113, 23,
  105, 109, 96]

/-- The parent of each vertex in a breadth-first tree of words in vertices 8 and 30. -/
def wordParent : List ℕ := [
  0, 10, 29, 12, 16, 21, 113, 19, 0, 11, 14, 6, 79, 7, 107, 23, 8, 47, 3, 67, 2, 17, 20, 4, 8, 32,
  30, 44, 78, 31, 0, 90, 91, 30, 44, 43, 35, 39, 12, 14, 53, 42, 63, 65, 91, 33, 27, 25, 86, 59,
  73, 98, 64, 6, 24, 16, 26, 96, 60, 13, 101, 71, 50, 117, 118, 108, 77, 56, 45, 94, 88, 84, 41,
  83, 82, 33, 34, 24, 65, 92, 105, 58, 52, 57, 81, 95, 49, 38, 76, 2, 108, 15, 110, 36, 72, 74,
  119, 99, 19, 17, 89, 102, 75, 77, 55, 117, 111, 109, 23, 116, 114, 104, 66, 115, 4, 54, 55, 114,
  104, 103]

/-- The last letter of each word: `false` for vertex 8, `true` for vertex 30. -/
def wordLetter : List Bool := [
  false, false, true, false, false, false, true, false, false, false, false, false, true, false,
  true, false, false, true, false, true, false, false, false, false, true, false, false, true,
  false, true, true, false, false, true, false, false, true, false, true, true, false, true,
  false, false, true, true, false, true, false, true, true, false, true, true, false, true, false,
  true, false, true, true, false, true, false, false, true, true, true, false, false, true, true,
  true, false, false, false, false, true, true, true, false, false, true, false, true, true,
  false, false, true, true, false, true, false, true, false, false, true, false, true, true,
  false, true, false, false, true, true, false, true, true, false, false, true, false, true, true,
  false, false, true, false, false]

/-- The length of each word. -/
def wordLength : List ℕ := [
  0, 9, 9, 9, 3, 12, 6, 6, 1, 8, 8, 7, 8, 7, 7, 5, 2, 10, 10, 5, 10, 11, 11, 4, 2, 8, 2, 8, 8, 8,
  1, 7, 7, 2, 8, 8, 9, 9, 9, 8, 8, 8, 7, 7, 7, 3, 9, 9, 11, 9, 10, 7, 7, 7, 3, 3, 3, 7, 7, 8, 6,
  11, 11, 6, 6, 6, 4, 4, 4, 11, 11, 10, 9, 9, 9, 3, 9, 3, 7, 7, 7, 8, 8, 8, 9, 11, 10, 10, 10, 10,
  6, 6, 6, 10, 10, 10, 6, 12, 6, 11, 11, 5, 4, 4, 4, 6, 6, 6, 5, 5, 5, 5, 5, 5, 4, 4, 4, 5, 5, 5]

/-- Quaternion conjugation: `star vᵢ = v_{conjTable[i]}`. -/
def conjTable : List ℕ := [
  0, 5, 6, 7, 4, 1, 2, 3, 15, 14, 13, 12, 11, 10, 9, 8, 23, 22, 21, 20, 19, 18, 17, 16, 108, 109,
  110, 63, 64, 65, 90, 91, 92, 105, 106, 107, 96, 97, 98, 51, 52, 53, 78, 79, 80, 117, 118, 119,
  84, 85, 86, 39, 40, 41, 114, 115, 116, 81, 82, 83, 72, 73, 74, 27, 28, 29, 102, 103, 104, 93,
  94, 95, 60, 61, 62, 111, 112, 113, 42, 43, 44, 57, 58, 59, 48, 49, 50, 99, 100, 101, 30, 31, 32,
  69, 70, 71, 36, 37, 38, 87, 88, 89, 66, 67, 68, 33, 34, 35, 24, 25, 26, 75, 76, 77, 54, 55, 56,
  45, 46, 47]

/-- Entry `i` of a table, reduced modulo 120. -/
def tableAt (t : List ℕ) (i : ℕ) : ℕ := t.getD i 0 % 120

/-- A table as a map on vertex indices. -/
def tableFin (t : List ℕ) (i : Fin 120) : Fin 120 := ⟨tableAt t i, Nat.mod_lt _ (by norm_num)⟩

/-- The multiplication table of vertex `g`: `v_g vᵢ = 2 v_{t[i]}` for every `i`. -/
def leftCheck (g : ℕ) (t : List ℕ) : Bool :=
  (List.range 120).all fun i => decide (qmul (vertexN g) (vertexN i) = dbl (vertexN (tableAt t i)))

theorem leftCheck_8 : leftCheck 8 leftTable8 = true := by decide +kernel

theorem leftCheck_30 : leftCheck 30 leftTable30 = true := by decide +kernel

/-- The table of the last letter of the word of vertex `i`. -/
def wordTable (i : ℕ) : List ℕ := if wordLetter.getD i false then leftTable30 else leftTable8

/-- The word tree: every vertex other than `0` is a generator times its parent, and the parent's
word is shorter. -/
def wordCheck : Bool :=
  (List.range 120).all fun i =>
    i == 0 || (tableAt (wordTable i) (tableAt wordParent i) == i &&
      decide (wordLength.getD (tableAt wordParent i) 0 < wordLength.getD i 0))

theorem wordCheck_eq_true : wordCheck = true := by decide +kernel

/-- The conjugation table: `star vᵢ = v_{conjTable[i]}`. -/
def conjCheck : Bool :=
  (List.range 120).all fun i => decide (conj (vertexN i) = vertexN (tableAt conjTable i))

theorem conjCheck_eq_true : conjCheck = true := by decide +kernel

theorem vertex_zero : vertex 0 = ⟨⟨2, 0⟩, ⟨0, 0⟩, ⟨0, 0⟩, ⟨0, 0⟩⟩ := by decide +kernel

theorem vertex_four : vertex 4 = ⟨⟨-2, 0⟩, ⟨0, 0⟩, ⟨0, 0⟩, ⟨0, 0⟩⟩ := by decide +kernel

theorem all_range_iff {n : ℕ} {p : ℕ → Bool} :
    (List.range n).all p = true ↔ ∀ i < n, p i = true := by
  simp [List.all_eq_true, List.mem_range]

theorem qmul_vertex_of_check {g : ℕ} {t : List ℕ} (h : leftCheck g t = true) (i : Fin 120) :
    qmul (vertexN g) (vertex i) = dbl (vertex (tableFin t i)) :=
  of_decide_eq_true (all_range_iff.mp h i i.2)

theorem qmul_vertex_8 (i : Fin 120) :
    qmul (vertex 8) (vertex i) = dbl (vertex (tableFin leftTable8 i)) :=
  qmul_vertex_of_check leftCheck_8 i

theorem qmul_vertex_30 (i : Fin 120) :
    qmul (vertex 30) (vertex i) = dbl (vertex (tableFin leftTable30 i)) :=
  qmul_vertex_of_check leftCheck_30 i

/-! ### Unit vertices -/

/-- The unit quaternion `vᵢ / 2`. -/
noncomputable def unitVertex (i : Fin 120) : ℍ := (2⁻¹ : ℝ) • vertexR i

theorem range_unitVertex : Set.range unitVertex = (2⁻¹ : ℝ) • vertexSet := by
  rw [← range_vertexR, Set.smul_set_range]
  rfl

theorem unitVertex_injective : Function.Injective unitVertex := fun _ _ h =>
  vertexR_injective (smul_right_injective ℍ (inv_ne_zero two_ne_zero) h)

theorem norm_unitVertex (i : Fin 120) : ‖unitVertex i‖ = 1 := by
  rw [unitVertex, norm_smul, norm_vertexR]
  norm_num

theorem unitVertex_mul_of_qmul {a b c : Fin 120}
    (h : qmul (vertex a) (vertex b) = dbl (vertex c)) :
    unitVertex a * unitVertex b = unitVertex c := by
  have h' := congrArg toQuat h
  rw [toQuat_qmul, toQuat_dbl] at h'
  change vertexR a * vertexR b = (2 : ℝ) • vertexR c at h'
  rw [unitVertex, unitVertex, unitVertex, smul_mul_assoc, mul_smul_comm, smul_smul, h', smul_smul]
  norm_num

theorem unitVertex_zero : unitVertex 0 = 1 := by
  rw [unitVertex, vertexR, vertex_zero]
  ext <;> simp [toQuat, toReal]

theorem unitVertex_four : unitVertex 4 = -1 := by
  rw [unitVertex, vertexR, vertex_four]
  ext <;> simp [toQuat, toReal]

theorem star_unitVertex (i : Fin 120) :
    star (unitVertex i) = unitVertex (tableFin conjTable i) := by
  have h : conj (vertex i) = vertex (tableFin conjTable i) :=
    of_decide_eq_true (all_range_iff.mp conjCheck_eq_true i i.2)
  have h' := congrArg toQuat h
  rw [toQuat_conj] at h'
  rw [unitVertex, unitVertex, Quaternion.star_smul]
  exact congrArg _ h'

/-! ### Closure -/

/-- The quaternions whose left multiplication maps `V / 2` into itself. -/
def leftStable : Submonoid ℍ where
  carrier := {x | ∀ i, ∃ j, x * unitVertex i = unitVertex j}
  one_mem' := fun i => ⟨i, one_mul _⟩
  mul_mem' := by
    intro x y hx hy i
    obtain ⟨j, hj⟩ := hy i
    obtain ⟨k, hk⟩ := hx j
    exact ⟨k, by rw [mul_assoc, hj, hk]⟩

theorem unitVertex_8_mem : unitVertex 8 ∈ leftStable := fun i =>
  ⟨_, unitVertex_mul_of_qmul (qmul_vertex_8 i)⟩

theorem unitVertex_30_mem : unitVertex 30 ∈ leftStable := fun i =>
  ⟨_, unitVertex_mul_of_qmul (qmul_vertex_30 i)⟩

theorem word_step (i : Fin 120) (hi : i ≠ 0) :
    ∃ g p : Fin 120, unitVertex g ∈ leftStable ∧ unitVertex g * unitVertex p = unitVertex i ∧
      wordLength.getD p 0 < wordLength.getD i 0 := by
  have h := all_range_iff.mp wordCheck_eq_true i i.2
  have hi' : ((i : ℕ) == 0) = false := by
    simpa using fun h0 => hi (Fin.ext h0)
  rw [hi', Bool.false_or, Bool.and_eq_true, beq_iff_eq, decide_eq_true_eq] at h
  obtain ⟨h1, h2⟩ := h
  rw [wordTable] at h1
  cases hb : wordLetter.getD (i : ℕ) false
  · rw [hb] at h1
    simp only [Bool.false_eq_true, ↓reduceIte] at h1
    refine ⟨8, tableFin wordParent i, unitVertex_8_mem, ?_, h2⟩
    have h3 := unitVertex_mul_of_qmul (qmul_vertex_8 (tableFin wordParent i))
    rwa [show tableFin leftTable8 (tableFin wordParent i) = i from Fin.ext h1] at h3
  · rw [hb] at h1
    simp only [↓reduceIte] at h1
    refine ⟨30, tableFin wordParent i, unitVertex_30_mem, ?_, h2⟩
    have h3 := unitVertex_mul_of_qmul (qmul_vertex_30 (tableFin wordParent i))
    rwa [show tableFin leftTable30 (tableFin wordParent i) = i from Fin.ext h1] at h3

theorem unitVertex_mem_leftStable_of_le (n : ℕ) :
    ∀ i : Fin 120, wordLength.getD i 0 ≤ n → unitVertex i ∈ leftStable := by
  induction n with
  | zero =>
    intro i hi
    by_cases h0 : i = 0
    · rw [h0, unitVertex_zero]
      exact leftStable.one_mem
    · obtain ⟨g, p, hg, hgp, hlt⟩ := word_step i h0
      omega
  | succ n ih =>
    intro i hi
    by_cases h0 : i = 0
    · rw [h0, unitVertex_zero]
      exact leftStable.one_mem
    · obtain ⟨g, p, hg, hgp, hlt⟩ := word_step i h0
      rw [← hgp]
      exact leftStable.mul_mem hg (ih p (by omega))

theorem unitVertex_mem_leftStable (i : Fin 120) : unitVertex i ∈ leftStable :=
  unitVertex_mem_leftStable_of_le _ i le_rfl

/-- **Closure.** The product of two elements of `V / 2` lies in `V / 2`. -/
theorem unitVertex_mul (a b : Fin 120) : ∃ c, unitVertex a * unitVertex b = unitVertex c :=
  unitVertex_mem_leftStable a b

/-- **Closure of `V / 2`.** The set `V / 2` of 120 unit quaternions is closed under the quaternion
product and under conjugation, and contains `1` and `-1`. -/
theorem halfVertexSet_closure :
    (∀ x ∈ (2⁻¹ : ℝ) • vertexSet, ∀ y ∈ (2⁻¹ : ℝ) • vertexSet, x * y ∈ (2⁻¹ : ℝ) • vertexSet) ∧
      (∀ x ∈ (2⁻¹ : ℝ) • vertexSet, star x ∈ (2⁻¹ : ℝ) • vertexSet) ∧
      (1 : ℍ) ∈ (2⁻¹ : ℝ) • vertexSet ∧ (-1 : ℍ) ∈ (2⁻¹ : ℝ) • vertexSet := by
  rw [← range_unitVertex]
  refine ⟨?_, ?_, ⟨0, unitVertex_zero⟩, ⟨4, unitVertex_four⟩⟩
  · rintro _ ⟨a, rfl⟩ _ ⟨b, rfl⟩
    obtain ⟨c, hc⟩ := unitVertex_mul a b
    exact ⟨c, hc.symm⟩
  · rintro _ ⟨a, rfl⟩
    exact ⟨_, (star_unitVertex a).symm⟩

theorem unitVertex_mem_unitary (i : Fin 120) : unitVertex i ∈ unitary ℍ := by
  have h : Quaternion.normSq (unitVertex i) = 1 := by
    rw [Quaternion.normSq_eq_norm_mul_self, norm_unitVertex, one_mul]
  rw [Unitary.mem_iff, Quaternion.star_mul_self, Quaternion.self_mul_star, h]
  exact ⟨by norm_num, by norm_num⟩

/-! ### The group -/

/-- **O3.** The binary icosahedral group `I*`, defined here as the elements of `Sp(1)` that lie
in `V / 2`. That it is the inverse image of the icosahedral rotation group is E1, a hypothesis
(`CoordinateSources`). -/
def binaryIcosahedral : Subgroup (unitary ℍ) where
  carrier := {u | (u : ℍ) ∈ Set.range unitVertex}
  one_mem' := ⟨0, unitVertex_zero⟩
  mul_mem' := by
    rintro a b ⟨i, hi⟩ ⟨j, hj⟩
    obtain ⟨c, hc⟩ := unitVertex_mul i j
    refine ⟨c, ?_⟩
    change unitVertex c = (a : ℍ) * b
    rw [← hi, ← hj, hc]
  inv_mem' := by
    rintro a ⟨i, hi⟩
    refine ⟨tableFin conjTable i, ?_⟩
    rw [← Unitary.star_eq_inv, Unitary.coe_star, ← hi, star_unitVertex]

theorem mem_binaryIcosahedral {u : unitary ℍ} :
    u ∈ binaryIcosahedral ↔ (u : ℍ) ∈ (2⁻¹ : ℝ) • vertexSet := by
  rw [← range_unitVertex]
  rfl

/-- The element of `I*` with index `i`. -/
noncomputable def icosian (i : Fin 120) : binaryIcosahedral :=
  ⟨⟨unitVertex i, unitVertex_mem_unitary i⟩, ⟨i, rfl⟩⟩

theorem icosian_bijective : Function.Bijective icosian := by
  refine ⟨fun i j h => unitVertex_injective ?_, ?_⟩
  · exact congrArg (fun u : binaryIcosahedral => ((u : unitary ℍ) : ℍ)) h
  · rintro ⟨u, i, hi⟩
    exact ⟨i, Subtype.ext (Subtype.ext hi)⟩

theorem neg_one_mem_binaryIcosahedral : (-1 : unitary ℍ) ∈ binaryIcosahedral :=
  ⟨4, by rw [unitVertex_four, Unitary.coe_neg, OneMemClass.coe_one]⟩

/-- **O3.** `I*` is a subgroup of `Sp(1)` with 120 elements; as a set of quaternions it is `V / 2`,
and it contains `-1`. -/
theorem binaryIcosahedralGroup :
    Nat.card binaryIcosahedral = 120 ∧
      (fun u : unitary ℍ => (u : ℍ)) '' binaryIcosahedral = (2⁻¹ : ℝ) • vertexSet ∧
      (-1 : unitary ℍ) ∈ binaryIcosahedral := by
  refine ⟨?_, ?_, neg_one_mem_binaryIcosahedral⟩
  · rw [← Nat.card_eq_of_bijective icosian icosian_bijective, Nat.card_eq_fintype_card,
      Fintype.card_fin]
  · rw [← range_unitVertex]
    ext q
    constructor
    · rintro ⟨u, hu, rfl⟩
      exact hu
    · rintro ⟨i, rfl⟩
      exact ⟨⟨unitVertex i, unitVertex_mem_unitary i⟩, ⟨i, rfl⟩, rfl⟩

end ArchonFull600Theory
