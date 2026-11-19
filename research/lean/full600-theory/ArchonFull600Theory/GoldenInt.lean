import Mathlib.NumberTheory.Real.GoldenRatio
import Mathlib.Tactic.LinearCombination
import Mathlib.Tactic.Linarith
import Mathlib.Tactic.NormNum
import Mathlib.Tactic.Positivity
import Mathlib.Tactic.Ring

set_option linter.style.header false in
/-!
# The ring `ℤ[φ]`

The golden ratio `φ`, its inverse `τ = φ⁻¹`, and the exact pair arithmetic of the finite
checker: an element `a + b φ` is stored as the integer pair `(a, b)`, and its sign is decided
by comparing integers after writing `2 (a + b φ) = (2 a + b) + b √5`.
-/

namespace ArchonFull600Theory

open scoped goldenRatio

/-- The golden ratio identities of the definitions: `φ² = φ + 1`, `τ = φ⁻¹ = φ - 1` and
`τ² = 2 - φ`. -/
theorem goldenRatioIdentities :
    φ ^ 2 = φ + 1 ∧ φ⁻¹ = φ - 1 ∧ φ⁻¹ ^ 2 = 2 - φ := by
  have hinv : φ⁻¹ = φ - 1 := by
    rw [Real.inv_goldenRatio]
    linear_combination -Real.goldenRatio_add_goldenConj
  refine ⟨Real.goldenRatio_sq, hinv, ?_⟩
  rw [hinv]
  linear_combination Real.goldenRatio_sq

/-- An element `a + b φ` of `ℤ[φ]`, stored as the integer pair `(a, b)`. -/
structure ZPhi where
  /-- The rational part. -/
  a : ℤ
  /-- The coefficient of `φ`. -/
  b : ℤ
deriving DecidableEq

namespace ZPhi

/-- Addition of pairs. -/
def add (x y : ZPhi) : ZPhi := ⟨x.a + y.a, x.b + y.b⟩

/-- The checker's product `(a + b φ)(c + d φ) = (a c + b d) + (a d + b c + b d) φ`. -/
def mul (x y : ZPhi) : ZPhi := ⟨x.a * y.a + x.b * y.b, x.a * y.b + x.b * y.a + x.b * y.b⟩

/-- Negation of pairs. -/
def neg (x : ZPhi) : ZPhi := ⟨-x.a, -x.b⟩

/-- Subtraction of pairs. -/
def sub (x y : ZPhi) : ZPhi := add x (neg y)

/-- The pair `(0, 1)` representing `φ`. -/
def phi : ZPhi := ⟨0, 1⟩

/-- The pair `(-1, 1)` representing `τ = φ - 1`. -/
def tau : ZPhi := ⟨-1, 1⟩

/-- The real number `a + b φ`. -/
noncomputable def toReal (x : ZPhi) : ℝ := x.a + x.b * φ

theorem toReal_mk (a b : ℤ) : toReal ⟨a, b⟩ = a + b * φ := rfl

theorem toReal_add (x y : ZPhi) : toReal (add x y) = toReal x + toReal y := by
  simp only [toReal, add, Int.cast_add]
  ring

theorem toReal_neg (x : ZPhi) : toReal (neg x) = -toReal x := by
  simp only [toReal, neg, Int.cast_neg]
  ring

theorem toReal_sub (x y : ZPhi) : toReal (sub x y) = toReal x - toReal y := by
  rw [sub, toReal_add, toReal_neg, sub_eq_add_neg]

theorem toReal_mul (x y : ZPhi) : toReal (mul x y) = toReal x * toReal y := by
  simp only [toReal, mul, Int.cast_add, Int.cast_mul]
  linear_combination (-(x.b * y.b : ℝ)) * Real.goldenRatio_sq

theorem toReal_phi : toReal phi = φ := by simp [toReal, phi]

theorem toReal_tau : toReal tau = φ⁻¹ := by
  rw [goldenRatioIdentities.2.1]
  simp [toReal, tau]
  ring

/-- Distinct pairs are distinct real numbers, because `φ` is irrational. -/
theorem toReal_injective : Function.Injective toReal := by
  intro x y h
  simp only [toReal] at h
  have hb : x.b = y.b := by
    by_contra hne
    have hne' : ((x.b - y.b : ℤ) : ℝ) ≠ 0 := by exact_mod_cast sub_ne_zero.mpr hne
    apply Real.goldenRatio_irrational.ne_rational (y.a - x.a) (x.b - y.b)
    rw [eq_div_iff hne']
    push_cast
    linear_combination h
  have ha : x.a = y.a := by
    rw [hb] at h
    exact_mod_cast add_right_cancel h
  cases x
  cases y
  simp_all

/-- The pair arithmetic is the arithmetic of `ℤ[φ]`: `φ² = φ + 1` and `τ² = 2 - φ` as pairs. -/
theorem pairArithmetic :
    mul phi phi = add phi ⟨1, 0⟩ ∧ mul tau tau = sub ⟨2, 0⟩ phi ∧ mul phi tau = ⟨1, 0⟩ := by
  decide

/-- Sign of `A + B √5` for integers `A` and `B`, with the sign cases handled before squaring. -/
def signTest (A B : ℤ) : Bool :=
  if B = 0 then decide (0 < A)
  else if 0 < B then decide (0 ≤ A) || decide (A * A < 5 * B * B)
  else decide (0 < A) && decide (5 * B * B < A * A)

/-- The checker's sign test, through `2 (a + b φ) = (2 a + b) + b √5`. -/
def positive (x : ZPhi) : Bool := signTest (2 * x.a + x.b) x.b

theorem two_mul_toReal (x : ZPhi) :
    2 * toReal x = ((2 * x.a + x.b : ℤ) : ℝ) + x.b * Real.sqrt 5 := by
  simp only [toReal, Real.goldenRatio]
  push_cast
  ring

theorem signTest_iff (A B : ℤ) : signTest A B = true ↔ 0 < (A : ℝ) + B * Real.sqrt 5 := by
  have hs : Real.sqrt 5 ^ 2 = 5 := Real.sq_sqrt (by norm_num)
  have hs0 : 0 < Real.sqrt 5 := by positivity
  have e : ((B : ℝ) * Real.sqrt 5) ^ 2 = 5 * B ^ 2 := by rw [mul_pow, hs]; ring
  rcases lt_trichotomy B 0 with hB | hB | hB
  · have hB' : (B : ℝ) < 0 := by exact_mod_cast hB
    have hBs : (B : ℝ) * Real.sqrt 5 < 0 := mul_neg_of_neg_of_pos hB' hs0
    rw [signTest, ite_eq_right hB.ne, ite_eq_right (not_lt.mpr hB.le)]
    simp only [Bool.and_eq_true, decide_eq_true_eq]
    constructor
    · rintro ⟨hA, hsq⟩
      have hA' : (0 : ℝ) < A := by exact_mod_cast hA
      have hsq' : (5 : ℝ) * B * B < A * A := by exact_mod_cast hsq
      by_contra hle
      push Not at hle
      nlinarith [mul_nonneg (show (0 : ℝ) ≤ -(A + B * Real.sqrt 5) by linarith)
        (show (0 : ℝ) ≤ A - B * Real.sqrt 5 by linarith)]
    · intro h
      have hA' : (0 : ℝ) < A := by linarith
      refine ⟨by exact_mod_cast hA', ?_⟩
      have : (5 : ℝ) * B * B < A * A := by
        nlinarith [mul_pos h (show (0 : ℝ) < A - B * Real.sqrt 5 by linarith)]
      exact_mod_cast this
  · subst hB
    simp [signTest]
  · have hB' : (0 : ℝ) < B := by exact_mod_cast hB
    have hBs : 0 < (B : ℝ) * Real.sqrt 5 := mul_pos hB' hs0
    rw [signTest, ite_eq_right hB.ne', ite_eq_left hB]
    simp only [Bool.or_eq_true, decide_eq_true_eq]
    constructor
    · rintro (hA | hsq)
      · have hA' : (0 : ℝ) ≤ A := by exact_mod_cast hA
        linarith
      · have hsq' : (A : ℝ) * A < 5 * B * B := by exact_mod_cast hsq
        by_contra hle
        push Not at hle
        nlinarith [mul_nonneg (show (0 : ℝ) ≤ -A - B * Real.sqrt 5 by linarith)
          (show (0 : ℝ) ≤ -A + B * Real.sqrt 5 by linarith)]
    · intro h
      by_cases hA : 0 ≤ A
      · exact Or.inl hA
      · right
        have hA' : (A : ℝ) < 0 := by exact_mod_cast not_le.mp hA
        have : (A : ℝ) * A < 5 * B * B := by
          nlinarith [mul_pos h (show (0 : ℝ) < B * Real.sqrt 5 - A by linarith)]
        exact_mod_cast this

/-- The checker's sign test is exact: it accepts `a + b φ` exactly when this real number is
positive. -/
theorem positive_iff (x : ZPhi) : positive x = true ↔ 0 < toReal x := by
  rw [positive, signTest_iff, ← two_mul_toReal]
  constructor <;> intro h <;> linarith

end ZPhi

end ArchonFull600Theory
