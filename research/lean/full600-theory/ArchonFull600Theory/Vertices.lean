import ArchonFull600Theory.GoldenInt
import Mathlib.Analysis.Convex.Hull
import Mathlib.Analysis.Quaternion
import Mathlib.Data.Fintype.Perm
import Mathlib.Data.Fintype.Pi
import Mathlib.Data.Set.Card
import Mathlib.GroupTheory.Perm.Sign

set_option linter.style.header false in
/-!
# The 120 vertices

The vertex set `V` in quaternion coordinates, its three coordinate families, and Lemma 1
(coordinate identification). The two external geometric results E1 and E2 enter as hypotheses.

The exact computations use a table of the 120 vertices with coordinates in `ℤ[φ]`. The table is
checked by `decide +kernel` against the three families built from the even permutations of
`Fin 4`, so the kernel, not the table, certifies the vertex set.
-/

namespace ArchonFull600Theory

open scoped goldenRatio Quaternion RealInnerProductSpace Pointwise
open Finset ZPhi

/-! ### The vertex set in real coordinates -/

/-- `±1` from a sign bit; `true` is the negative sign. -/
def signOf (b : Bool) : ℝ := if b then -1 else 1

/-- The quaternion whose coordinate `k`, in the order `1, i, j, k`, is `c (σ k)` with the sign
`ε k`. -/
noncomputable def signedPermute (σ : Equiv.Perm (Fin 4)) (ε : Fin 4 → Bool) (c : Fin 4 → ℝ) :
    ℍ :=
  ⟨signOf (ε 0) * c (σ 0), signOf (ε 1) * c (σ 1), signOf (ε 2) * c (σ 2),
    signOf (ε 3) * c (σ 3)⟩

/-- The even coordinate permutations of `c`, with independent signs. -/
def evenSignedPerms (c : Fin 4 → ℝ) : Set ℍ :=
  {q | ∃ σ : Equiv.Perm (Fin 4), Equiv.Perm.sign σ = 1 ∧ ∃ ε : Fin 4 → Bool,
    q = signedPermute σ ε c}

/-- The vertex set `V`: `(±2, 0, 0, 0)`, `(±1, ±1, ±1, ±1)` and `(0, ±1, ±φ, ±τ)`, each with its
even coordinate permutations and independent signs. -/
noncomputable def vertexSet : Set ℍ :=
  evenSignedPerms ![2, 0, 0, 0] ∪ evenSignedPerms ![1, 1, 1, 1] ∪
    evenSignedPerms ![0, 1, φ, φ⁻¹]

/-- The coordinate set of the quaternion coordinate result E1: `(±1, 0, 0, 0)`,
`½ (±1, ±1, ±1, ±1)` and `½ (±φ, ±1, ±τ, 0)`, with even coordinate permutations and
independent signs. -/
noncomputable def quaternionCoordinateSet : Set ℍ :=
  evenSignedPerms ![1, 0, 0, 0] ∪ evenSignedPerms ![1 / 2, 1 / 2, 1 / 2, 1 / 2] ∪
    evenSignedPerms ![φ / 2, 1 / 2, φ⁻¹ / 2, 0]

theorem signedPermute_smul (a : ℝ) (σ : Equiv.Perm (Fin 4)) (ε : Fin 4 → Bool)
    (c : Fin 4 → ℝ) : signedPermute σ ε (a • c) = a • signedPermute σ ε c := by
  ext <;> simp [signedPermute] <;> ring

theorem evenSignedPerms_smul (a : ℝ) (c : Fin 4 → ℝ) :
    a • evenSignedPerms c = evenSignedPerms (a • c) := by
  ext q
  simp only [Set.mem_smul_set, evenSignedPerms, Set.mem_ofPred_eq]
  constructor
  · rintro ⟨_, ⟨σ, hσ, ε, rfl⟩, rfl⟩
    exact ⟨σ, hσ, ε, (signedPermute_smul a σ ε c).symm⟩
  · rintro ⟨σ, hσ, ε, rfl⟩
    exact ⟨_, ⟨σ, hσ, ε, rfl⟩, (signedPermute_smul a σ ε c).symm⟩

/-- Rearranging the coordinates of `c` by an even permutation gives the same set. -/
theorem evenSignedPerms_comp (π : Equiv.Perm (Fin 4)) (hπ : Equiv.Perm.sign π = 1)
    (c : Fin 4 → ℝ) : evenSignedPerms (c ∘ π) = evenSignedPerms c := by
  have key : ∀ σ ε, signedPermute σ ε (c ∘ π) = signedPermute (π * σ) ε c := fun σ ε => rfl
  ext q
  simp only [evenSignedPerms, Set.mem_ofPred_eq]
  constructor
  · rintro ⟨σ, hσ, ε, rfl⟩
    exact ⟨π * σ, by rw [Equiv.Perm.sign_mul, hπ, hσ, one_mul], ε, key σ ε⟩
  · rintro ⟨σ, hσ, ε, rfl⟩
    refine ⟨π⁻¹ * σ, by rw [Equiv.Perm.sign_mul, Equiv.Perm.sign_inv, hπ, hσ, one_mul], ε, ?_⟩
    rw [key, mul_inv_cancel_left]

/-! ### Exact coordinates -/

/-- A point of `ℝ⁴` with coordinates in `ℤ[φ]`, in the order `1, i, j, k`. -/
structure Vec4 where
  /-- The real coordinate. -/
  x0 : ZPhi
  /-- The `i` coordinate. -/
  x1 : ZPhi
  /-- The `j` coordinate. -/
  x2 : ZPhi
  /-- The `k` coordinate. -/
  x3 : ZPhi
deriving DecidableEq

namespace Vec4

/-- The checker's exact inner product. -/
def dot (v w : Vec4) : ZPhi :=
  add (add (mul v.x0 w.x0) (mul v.x1 w.x1)) (add (mul v.x2 w.x2) (mul v.x3 w.x3))

/-- The quaternion with these coordinates. -/
noncomputable def toQuat (v : Vec4) : ℍ := ⟨toReal v.x0, toReal v.x1, toReal v.x2, toReal v.x3⟩

theorem inner_toQuat (v w : Vec4) : ⟪toQuat v, toQuat w⟫ = toReal (dot v w) := by
  rw [Quaternion.inner_def]
  simp only [toQuat, dot, toReal_add, toReal_mul, Quaternion.re_mul, Quaternion.re_star,
    Quaternion.imI_star, Quaternion.imJ_star, Quaternion.imK_star]
  ring

theorem toQuat_injective : Function.Injective toQuat := by
  rintro ⟨a0, a1, a2, a3⟩ ⟨b0, b1, b2, b3⟩ h
  simp only [toQuat, QuaternionAlgebra.mk.injEq] at h
  obtain ⟨h0, h1, h2, h3⟩ := h
  rw [toReal_injective h0, toReal_injective h1, toReal_injective h2, toReal_injective h3]

theorem dot_comm (v w : Vec4) : dot v w = dot w v := by
  apply toReal_injective
  rw [← inner_toQuat, ← inner_toQuat, real_inner_comm]

end Vec4

open Vec4

/-- The sign `±` applied to an exact coordinate. -/
def sgnZ (b : Bool) (x : ZPhi) : ZPhi := if b then neg x else x

theorem toReal_sgnZ (b : Bool) (x : ZPhi) : toReal (sgnZ b x) = signOf b * toReal x := by
  cases b <;> simp [sgnZ, signOf, toReal_neg]

/-- The exact counterpart of `signedPermute`. -/
def signedPermuteZ (σ : Equiv.Perm (Fin 4)) (ε : Fin 4 → Bool) (c : Fin 4 → ZPhi) : Vec4 :=
  ⟨sgnZ (ε 0) (c (σ 0)), sgnZ (ε 1) (c (σ 1)), sgnZ (ε 2) (c (σ 2)), sgnZ (ε 3) (c (σ 3))⟩

/-- The exact counterpart of `evenSignedPerms`, as a finite set. -/
def evenSignedPermsZ (c : Fin 4 → ZPhi) : Finset Vec4 :=
  (((univ : Finset (Equiv.Perm (Fin 4))).filter fun σ => Equiv.Perm.sign σ = 1) ×ˢ
    (univ : Finset (Fin 4 → Bool))).image fun p => signedPermuteZ p.1 p.2 c

theorem toQuat_signedPermuteZ (σ : Equiv.Perm (Fin 4)) (ε : Fin 4 → Bool) (c : Fin 4 → ZPhi) :
    toQuat (signedPermuteZ σ ε c) = signedPermute σ ε (fun k => toReal (c k)) := by
  simp [toQuat, signedPermuteZ, signedPermute, toReal_sgnZ]

theorem toQuat_image_evenSignedPermsZ (c : Fin 4 → ZPhi) :
    toQuat '' (evenSignedPermsZ c : Set Vec4) = evenSignedPerms (fun k => toReal (c k)) := by
  ext q
  simp only [evenSignedPermsZ, coe_image, coe_product, coe_filter, coe_univ, Set.mem_image,
    Set.mem_prod, Set.mem_ofPred_eq, Set.mem_univ, and_true, Prod.exists, evenSignedPerms,
    mem_univ, true_and]
  constructor
  · rintro ⟨_, ⟨σ, ε, hσ, rfl⟩, rfl⟩
    exact ⟨σ, hσ, ε, toQuat_signedPermuteZ σ ε c⟩
  · rintro ⟨σ, hσ, ε, rfl⟩
    exact ⟨_, ⟨σ, ε, hσ, rfl⟩, toQuat_signedPermuteZ σ ε c⟩

/-- Exact coefficients of the first family, `(2, 0, 0, 0)`. -/
def coeffs1 : Fin 4 → ZPhi := ![⟨2, 0⟩, ⟨0, 0⟩, ⟨0, 0⟩, ⟨0, 0⟩]

/-- Exact coefficients of the second family, `(1, 1, 1, 1)`. -/
def coeffs2 : Fin 4 → ZPhi := ![⟨1, 0⟩, ⟨1, 0⟩, ⟨1, 0⟩, ⟨1, 0⟩]

/-- Exact coefficients of the third family, `(0, 1, φ, τ)` with `τ = φ - 1`. -/
def coeffs3 : Fin 4 → ZPhi := ![⟨0, 0⟩, ⟨1, 0⟩, ⟨0, 1⟩, ⟨-1, 1⟩]

theorem toReal_coeffs1 : (fun k => toReal (coeffs1 k)) = ![2, 0, 0, 0] := by
  funext k
  fin_cases k <;> simp [coeffs1, toReal]

theorem toReal_coeffs2 : (fun k => toReal (coeffs2 k)) = ![1, 1, 1, 1] := by
  funext k
  fin_cases k <;> simp [coeffs2, toReal]

theorem toReal_coeffs3 : (fun k => toReal (coeffs3 k)) = ![0, 1, φ, φ⁻¹] := by
  funext k
  fin_cases k
  · simp [coeffs3, toReal]
  · simp [coeffs3, toReal]
  · simp [coeffs3, toReal]
  · simpa [coeffs3, tau] using toReal_tau

/-! ### The vertex table -/

/-- The 120 vertices with coordinates in `ℤ[φ]`; `⟨a, b⟩` stands for `a + b φ`. -/
def vertexTable : List Vec4 := [
  ⟨⟨2, 0⟩, ⟨0, 0⟩, ⟨0, 0⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨2, 0⟩, ⟨0, 0⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨0, 0⟩, ⟨2, 0⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨0, 0⟩, ⟨0, 0⟩, ⟨2, 0⟩⟩,
  ⟨⟨-2, 0⟩, ⟨0, 0⟩, ⟨0, 0⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨-2, 0⟩, ⟨0, 0⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨0, 0⟩, ⟨-2, 0⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨0, 0⟩, ⟨0, 0⟩, ⟨-2, 0⟩⟩,
  ⟨⟨1, 0⟩, ⟨1, 0⟩, ⟨1, 0⟩, ⟨1, 0⟩⟩,
  ⟨⟨1, 0⟩, ⟨1, 0⟩, ⟨1, 0⟩, ⟨-1, 0⟩⟩,
  ⟨⟨1, 0⟩, ⟨1, 0⟩, ⟨-1, 0⟩, ⟨1, 0⟩⟩,
  ⟨⟨1, 0⟩, ⟨1, 0⟩, ⟨-1, 0⟩, ⟨-1, 0⟩⟩,
  ⟨⟨1, 0⟩, ⟨-1, 0⟩, ⟨1, 0⟩, ⟨1, 0⟩⟩,
  ⟨⟨1, 0⟩, ⟨-1, 0⟩, ⟨1, 0⟩, ⟨-1, 0⟩⟩,
  ⟨⟨1, 0⟩, ⟨-1, 0⟩, ⟨-1, 0⟩, ⟨1, 0⟩⟩,
  ⟨⟨1, 0⟩, ⟨-1, 0⟩, ⟨-1, 0⟩, ⟨-1, 0⟩⟩,
  ⟨⟨-1, 0⟩, ⟨1, 0⟩, ⟨1, 0⟩, ⟨1, 0⟩⟩,
  ⟨⟨-1, 0⟩, ⟨1, 0⟩, ⟨1, 0⟩, ⟨-1, 0⟩⟩,
  ⟨⟨-1, 0⟩, ⟨1, 0⟩, ⟨-1, 0⟩, ⟨1, 0⟩⟩,
  ⟨⟨-1, 0⟩, ⟨1, 0⟩, ⟨-1, 0⟩, ⟨-1, 0⟩⟩,
  ⟨⟨-1, 0⟩, ⟨-1, 0⟩, ⟨1, 0⟩, ⟨1, 0⟩⟩,
  ⟨⟨-1, 0⟩, ⟨-1, 0⟩, ⟨1, 0⟩, ⟨-1, 0⟩⟩,
  ⟨⟨-1, 0⟩, ⟨-1, 0⟩, ⟨-1, 0⟩, ⟨1, 0⟩⟩,
  ⟨⟨-1, 0⟩, ⟨-1, 0⟩, ⟨-1, 0⟩, ⟨-1, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨1, 0⟩, ⟨0, 1⟩, ⟨-1, 1⟩⟩,
  ⟨⟨0, 0⟩, ⟨0, 1⟩, ⟨-1, 1⟩, ⟨1, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨-1, 1⟩, ⟨1, 0⟩, ⟨0, 1⟩⟩,
  ⟨⟨1, 0⟩, ⟨0, 0⟩, ⟨-1, 1⟩, ⟨0, 1⟩⟩,
  ⟨⟨1, 0⟩, ⟨0, 1⟩, ⟨0, 0⟩, ⟨-1, 1⟩⟩,
  ⟨⟨1, 0⟩, ⟨-1, 1⟩, ⟨0, 1⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, 1⟩, ⟨0, 0⟩, ⟨1, 0⟩, ⟨-1, 1⟩⟩,
  ⟨⟨0, 1⟩, ⟨1, 0⟩, ⟨-1, 1⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, 1⟩, ⟨-1, 1⟩, ⟨0, 0⟩, ⟨1, 0⟩⟩,
  ⟨⟨-1, 1⟩, ⟨0, 0⟩, ⟨0, 1⟩, ⟨1, 0⟩⟩,
  ⟨⟨-1, 1⟩, ⟨1, 0⟩, ⟨0, 0⟩, ⟨0, 1⟩⟩,
  ⟨⟨-1, 1⟩, ⟨0, 1⟩, ⟨1, 0⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨1, 0⟩, ⟨0, 1⟩, ⟨1, -1⟩⟩,
  ⟨⟨0, 0⟩, ⟨0, 1⟩, ⟨1, -1⟩, ⟨1, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨1, -1⟩, ⟨1, 0⟩, ⟨0, 1⟩⟩,
  ⟨⟨1, 0⟩, ⟨0, 0⟩, ⟨1, -1⟩, ⟨0, 1⟩⟩,
  ⟨⟨1, 0⟩, ⟨0, 1⟩, ⟨0, 0⟩, ⟨1, -1⟩⟩,
  ⟨⟨1, 0⟩, ⟨1, -1⟩, ⟨0, 1⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, 1⟩, ⟨0, 0⟩, ⟨1, 0⟩, ⟨1, -1⟩⟩,
  ⟨⟨0, 1⟩, ⟨1, 0⟩, ⟨1, -1⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, 1⟩, ⟨1, -1⟩, ⟨0, 0⟩, ⟨1, 0⟩⟩,
  ⟨⟨1, -1⟩, ⟨0, 0⟩, ⟨0, 1⟩, ⟨1, 0⟩⟩,
  ⟨⟨1, -1⟩, ⟨1, 0⟩, ⟨0, 0⟩, ⟨0, 1⟩⟩,
  ⟨⟨1, -1⟩, ⟨0, 1⟩, ⟨1, 0⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨1, 0⟩, ⟨0, -1⟩, ⟨-1, 1⟩⟩,
  ⟨⟨0, 0⟩, ⟨0, -1⟩, ⟨-1, 1⟩, ⟨1, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨-1, 1⟩, ⟨1, 0⟩, ⟨0, -1⟩⟩,
  ⟨⟨1, 0⟩, ⟨0, 0⟩, ⟨-1, 1⟩, ⟨0, -1⟩⟩,
  ⟨⟨1, 0⟩, ⟨0, -1⟩, ⟨0, 0⟩, ⟨-1, 1⟩⟩,
  ⟨⟨1, 0⟩, ⟨-1, 1⟩, ⟨0, -1⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, -1⟩, ⟨0, 0⟩, ⟨1, 0⟩, ⟨-1, 1⟩⟩,
  ⟨⟨0, -1⟩, ⟨1, 0⟩, ⟨-1, 1⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, -1⟩, ⟨-1, 1⟩, ⟨0, 0⟩, ⟨1, 0⟩⟩,
  ⟨⟨-1, 1⟩, ⟨0, 0⟩, ⟨0, -1⟩, ⟨1, 0⟩⟩,
  ⟨⟨-1, 1⟩, ⟨1, 0⟩, ⟨0, 0⟩, ⟨0, -1⟩⟩,
  ⟨⟨-1, 1⟩, ⟨0, -1⟩, ⟨1, 0⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨1, 0⟩, ⟨0, -1⟩, ⟨1, -1⟩⟩,
  ⟨⟨0, 0⟩, ⟨0, -1⟩, ⟨1, -1⟩, ⟨1, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨1, -1⟩, ⟨1, 0⟩, ⟨0, -1⟩⟩,
  ⟨⟨1, 0⟩, ⟨0, 0⟩, ⟨1, -1⟩, ⟨0, -1⟩⟩,
  ⟨⟨1, 0⟩, ⟨0, -1⟩, ⟨0, 0⟩, ⟨1, -1⟩⟩,
  ⟨⟨1, 0⟩, ⟨1, -1⟩, ⟨0, -1⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, -1⟩, ⟨0, 0⟩, ⟨1, 0⟩, ⟨1, -1⟩⟩,
  ⟨⟨0, -1⟩, ⟨1, 0⟩, ⟨1, -1⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, -1⟩, ⟨1, -1⟩, ⟨0, 0⟩, ⟨1, 0⟩⟩,
  ⟨⟨1, -1⟩, ⟨0, 0⟩, ⟨0, -1⟩, ⟨1, 0⟩⟩,
  ⟨⟨1, -1⟩, ⟨1, 0⟩, ⟨0, 0⟩, ⟨0, -1⟩⟩,
  ⟨⟨1, -1⟩, ⟨0, -1⟩, ⟨1, 0⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨-1, 0⟩, ⟨0, 1⟩, ⟨-1, 1⟩⟩,
  ⟨⟨0, 0⟩, ⟨0, 1⟩, ⟨-1, 1⟩, ⟨-1, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨-1, 1⟩, ⟨-1, 0⟩, ⟨0, 1⟩⟩,
  ⟨⟨-1, 0⟩, ⟨0, 0⟩, ⟨-1, 1⟩, ⟨0, 1⟩⟩,
  ⟨⟨-1, 0⟩, ⟨0, 1⟩, ⟨0, 0⟩, ⟨-1, 1⟩⟩,
  ⟨⟨-1, 0⟩, ⟨-1, 1⟩, ⟨0, 1⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, 1⟩, ⟨0, 0⟩, ⟨-1, 0⟩, ⟨-1, 1⟩⟩,
  ⟨⟨0, 1⟩, ⟨-1, 0⟩, ⟨-1, 1⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, 1⟩, ⟨-1, 1⟩, ⟨0, 0⟩, ⟨-1, 0⟩⟩,
  ⟨⟨-1, 1⟩, ⟨0, 0⟩, ⟨0, 1⟩, ⟨-1, 0⟩⟩,
  ⟨⟨-1, 1⟩, ⟨-1, 0⟩, ⟨0, 0⟩, ⟨0, 1⟩⟩,
  ⟨⟨-1, 1⟩, ⟨0, 1⟩, ⟨-1, 0⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨-1, 0⟩, ⟨0, 1⟩, ⟨1, -1⟩⟩,
  ⟨⟨0, 0⟩, ⟨0, 1⟩, ⟨1, -1⟩, ⟨-1, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨1, -1⟩, ⟨-1, 0⟩, ⟨0, 1⟩⟩,
  ⟨⟨-1, 0⟩, ⟨0, 0⟩, ⟨1, -1⟩, ⟨0, 1⟩⟩,
  ⟨⟨-1, 0⟩, ⟨0, 1⟩, ⟨0, 0⟩, ⟨1, -1⟩⟩,
  ⟨⟨-1, 0⟩, ⟨1, -1⟩, ⟨0, 1⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, 1⟩, ⟨0, 0⟩, ⟨-1, 0⟩, ⟨1, -1⟩⟩,
  ⟨⟨0, 1⟩, ⟨-1, 0⟩, ⟨1, -1⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, 1⟩, ⟨1, -1⟩, ⟨0, 0⟩, ⟨-1, 0⟩⟩,
  ⟨⟨1, -1⟩, ⟨0, 0⟩, ⟨0, 1⟩, ⟨-1, 0⟩⟩,
  ⟨⟨1, -1⟩, ⟨-1, 0⟩, ⟨0, 0⟩, ⟨0, 1⟩⟩,
  ⟨⟨1, -1⟩, ⟨0, 1⟩, ⟨-1, 0⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨-1, 0⟩, ⟨0, -1⟩, ⟨-1, 1⟩⟩,
  ⟨⟨0, 0⟩, ⟨0, -1⟩, ⟨-1, 1⟩, ⟨-1, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨-1, 1⟩, ⟨-1, 0⟩, ⟨0, -1⟩⟩,
  ⟨⟨-1, 0⟩, ⟨0, 0⟩, ⟨-1, 1⟩, ⟨0, -1⟩⟩,
  ⟨⟨-1, 0⟩, ⟨0, -1⟩, ⟨0, 0⟩, ⟨-1, 1⟩⟩,
  ⟨⟨-1, 0⟩, ⟨-1, 1⟩, ⟨0, -1⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, -1⟩, ⟨0, 0⟩, ⟨-1, 0⟩, ⟨-1, 1⟩⟩,
  ⟨⟨0, -1⟩, ⟨-1, 0⟩, ⟨-1, 1⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, -1⟩, ⟨-1, 1⟩, ⟨0, 0⟩, ⟨-1, 0⟩⟩,
  ⟨⟨-1, 1⟩, ⟨0, 0⟩, ⟨0, -1⟩, ⟨-1, 0⟩⟩,
  ⟨⟨-1, 1⟩, ⟨-1, 0⟩, ⟨0, 0⟩, ⟨0, -1⟩⟩,
  ⟨⟨-1, 1⟩, ⟨0, -1⟩, ⟨-1, 0⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨-1, 0⟩, ⟨0, -1⟩, ⟨1, -1⟩⟩,
  ⟨⟨0, 0⟩, ⟨0, -1⟩, ⟨1, -1⟩, ⟨-1, 0⟩⟩,
  ⟨⟨0, 0⟩, ⟨1, -1⟩, ⟨-1, 0⟩, ⟨0, -1⟩⟩,
  ⟨⟨-1, 0⟩, ⟨0, 0⟩, ⟨1, -1⟩, ⟨0, -1⟩⟩,
  ⟨⟨-1, 0⟩, ⟨0, -1⟩, ⟨0, 0⟩, ⟨1, -1⟩⟩,
  ⟨⟨-1, 0⟩, ⟨1, -1⟩, ⟨0, -1⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, -1⟩, ⟨0, 0⟩, ⟨-1, 0⟩, ⟨1, -1⟩⟩,
  ⟨⟨0, -1⟩, ⟨-1, 0⟩, ⟨1, -1⟩, ⟨0, 0⟩⟩,
  ⟨⟨0, -1⟩, ⟨1, -1⟩, ⟨0, 0⟩, ⟨-1, 0⟩⟩,
  ⟨⟨1, -1⟩, ⟨0, 0⟩, ⟨0, -1⟩, ⟨-1, 0⟩⟩,
  ⟨⟨1, -1⟩, ⟨-1, 0⟩, ⟨0, 0⟩, ⟨0, -1⟩⟩,
  ⟨⟨1, -1⟩, ⟨0, -1⟩, ⟨-1, 0⟩, ⟨0, 0⟩⟩
]

theorem vertexTable_length : vertexTable.length = 120 := rfl

theorem vertexTable_nodup : vertexTable.Nodup := by decide +kernel

/-- The table is exactly the union of the three exact families. -/
theorem vertexTable_toFinset :
    vertexTable.toFinset =
      evenSignedPermsZ coeffs1 ∪ evenSignedPermsZ coeffs2 ∪ evenSignedPermsZ coeffs3 := by
  decide +kernel

/-- The three exact families have 8, 16 and 96 elements and are pairwise disjoint. -/
theorem familiesZ :
    #(evenSignedPermsZ coeffs1) = 8 ∧ #(evenSignedPermsZ coeffs2) = 16 ∧
      #(evenSignedPermsZ coeffs3) = 96 ∧
      Disjoint (evenSignedPermsZ coeffs1) (evenSignedPermsZ coeffs2) ∧
      Disjoint (evenSignedPermsZ coeffs1) (evenSignedPermsZ coeffs3) ∧
      Disjoint (evenSignedPermsZ coeffs2) (evenSignedPermsZ coeffs3) := by
  decide +kernel

theorem vertexTable_dot_self : ∀ v ∈ vertexTable, dot v v = ⟨4, 0⟩ := by decide +kernel

/-- Vertex number `n` of the table (the origin past the end). -/
def vertexN (n : ℕ) : Vec4 := vertexTable.getD n ⟨⟨0, 0⟩, ⟨0, 0⟩, ⟨0, 0⟩, ⟨0, 0⟩⟩

/-- The exact vertex with index `i`. -/
def vertex (i : Fin 120) : Vec4 := vertexN i

/-- The vertex with index `i`, as a quaternion. -/
noncomputable def vertexR (i : Fin 120) : ℍ := toQuat (vertex i)

theorem vertex_eq_getElem (i : Fin 120) : vertex i = vertexTable[i.1] := by
  simp only [vertex, vertexN, List.getD_eq_getElem?_getD]
  rw [List.getElem?_eq_getElem (by rw [vertexTable_length]; exact i.2)]
  rfl

theorem vertex_mem (i : Fin 120) : vertex i ∈ vertexTable := by
  rw [vertex_eq_getElem]
  exact List.getElem_mem _

theorem vertex_injective : Function.Injective vertex := by
  intro i j h
  rw [vertex_eq_getElem, vertex_eq_getElem] at h
  exact Fin.ext ((List.Nodup.getElem_inj_iff vertexTable_nodup).mp h)

theorem range_vertex : Set.range vertex = {v | v ∈ vertexTable} := by
  ext v
  simp only [Set.mem_range, Set.mem_ofPred_eq]
  constructor
  · rintro ⟨i, rfl⟩
    exact vertex_mem i
  · intro hv
    obtain ⟨n, hn, rfl⟩ := List.mem_iff_getElem.mp hv
    exact ⟨⟨n, hn⟩, vertex_eq_getElem ⟨n, hn⟩⟩

theorem vertexR_injective : Function.Injective vertexR :=
  toQuat_injective.comp vertex_injective

theorem range_vertexR : Set.range vertexR = vertexSet := by
  have h : {v | v ∈ vertexTable} =
      ((evenSignedPermsZ coeffs1 ∪ evenSignedPermsZ coeffs2 ∪ evenSignedPermsZ coeffs3 :
        Finset Vec4) : Set Vec4) := by
    rw [← vertexTable_toFinset]
    ext v
    simp
  rw [show vertexR = toQuat ∘ vertex from rfl, Set.range_comp, range_vertex, h, coe_union,
    coe_union, Set.image_union, Set.image_union, toQuat_image_evenSignedPermsZ,
    toQuat_image_evenSignedPermsZ, toQuat_image_evenSignedPermsZ, toReal_coeffs1,
    toReal_coeffs2, toReal_coeffs3, vertexSet]

theorem inner_vertexR (i j : Fin 120) :
    ⟪vertexR i, vertexR j⟫ = toReal (dot (vertex i) (vertex j)) :=
  inner_toQuat _ _

theorem inner_vertexR_self (i : Fin 120) : ⟪vertexR i, vertexR i⟫ = 4 := by
  rw [inner_vertexR, vertexTable_dot_self _ (vertex_mem i)]
  simp [toReal]

theorem norm_vertexR (i : Fin 120) : ‖vertexR i‖ = 2 := by
  have h : ‖vertexR i‖ ^ 2 = 2 ^ 2 := by
    rw [← real_inner_self_eq_norm_sq, inner_vertexR_self]
    norm_num
  exact (pow_left_inj₀ (norm_nonneg _) (by norm_num) two_ne_zero).mp h

/-! ### Lemma 1 -/

/-- The three coordinate families have 8, 16 and 96 elements and are pairwise disjoint. -/
theorem vertexFamilies :
    (evenSignedPerms ![2, 0, 0, 0]).ncard = 8 ∧ (evenSignedPerms ![1, 1, 1, 1]).ncard = 16 ∧
      (evenSignedPerms ![0, 1, φ, φ⁻¹]).ncard = 96 ∧
      Disjoint (evenSignedPerms ![2, 0, 0, 0]) (evenSignedPerms ![1, 1, 1, 1]) ∧
      Disjoint (evenSignedPerms ![2, 0, 0, 0]) (evenSignedPerms ![0, 1, φ, φ⁻¹]) ∧
      Disjoint (evenSignedPerms ![1, 1, 1, 1]) (evenSignedPerms ![0, 1, φ, φ⁻¹]) := by
  obtain ⟨h1, h2, h3, d12, d13, d23⟩ := familiesZ
  rw [← toReal_coeffs1, ← toReal_coeffs2, ← toReal_coeffs3, ← toQuat_image_evenSignedPermsZ,
    ← toQuat_image_evenSignedPermsZ, ← toQuat_image_evenSignedPermsZ,
    Set.ncard_image_of_injective _ toQuat_injective,
    Set.ncard_image_of_injective _ toQuat_injective,
    Set.ncard_image_of_injective _ toQuat_injective, Set.ncard_coe_finset,
    Set.ncard_coe_finset, Set.ncard_coe_finset, Set.disjoint_image_iff toQuat_injective,
    Set.disjoint_image_iff toQuat_injective, Set.disjoint_image_iff toQuat_injective,
    disjoint_coe, disjoint_coe, disjoint_coe]
  exact ⟨h1, h2, h3, d12, d13, d23⟩

/-- `V` has 120 elements. -/
theorem vertexCount : vertexSet.ncard = 120 := by
  rw [← range_vertexR, ← Set.image_univ, Set.ncard_image_of_injective _ vertexR_injective,
    Set.ncard_univ, Nat.card_eq_fintype_card, Fintype.card_fin]

/-- Every element of `V` has norm 2. -/
theorem vertexNorm : ∀ q ∈ vertexSet, ‖q‖ = 2 := by
  rw [← range_vertexR]
  rintro _ ⟨i, rfl⟩
  exact norm_vertexR i

/-- The even permutation of coordinate positions that turns `(0, 1, φ, τ)` into
`(φ, 1, τ, 0)`. -/
def thirdFamilyPerm : Equiv.Perm (Fin 4) where
  toFun := ![2, 1, 3, 0]
  invFun := ![3, 1, 0, 2]
  left_inv := by decide
  right_inv := by decide

theorem sign_thirdFamilyPerm : Equiv.Perm.sign thirdFamilyPerm = 1 := by decide

/-- `V / 2` is the coordinate set of the quaternion coordinate result, including its convention
of even coordinate permutations. -/
theorem halfVertexSet : (2 : ℝ)⁻¹ • vertexSet = quaternionCoordinateSet := by
  have e3 : ![φ / 2, 1 / 2, φ⁻¹ / 2, 0] = ((2 : ℝ)⁻¹ • ![0, 1, φ, φ⁻¹]) ∘ thirdFamilyPerm := by
    funext k
    fin_cases k <;> simp [thirdFamilyPerm] <;> ring
  have e1 : ![(1 : ℝ), 0, 0, 0] = (2 : ℝ)⁻¹ • ![2, 0, 0, 0] := by
    funext k
    fin_cases k <;> simp
  have e2 : ![(1 : ℝ) / 2, 1 / 2, 1 / 2, 1 / 2] = (2 : ℝ)⁻¹ • ![1, 1, 1, 1] := by
    funext k
    fin_cases k <;> simp
  rw [quaternionCoordinateSet, vertexSet, Set.smul_set_union, Set.smul_set_union,
    evenSignedPerms_smul, evenSignedPerms_smul, evenSignedPerms_smul, e1, e2, e3,
    evenSignedPerms_comp _ sign_thirdFamilyPerm]

/-- `V / 2` lies in `Sp(1)`. -/
theorem halfVertexSet_norm : ∀ q ∈ (2 : ℝ)⁻¹ • vertexSet, ‖q‖ = 1 := by
  rintro _ ⟨q, hq, rfl⟩
  rw [norm_smul, vertexNorm q hq]
  norm_num

/-- The external results used by Lemma 1, as hypotheses on two predicates: `IsBinaryIcosahedral S`
says that `S` is the inverse image in `Sp(1)` of the rotation group of a regular icosahedron, and
`IsRegular600Cell P` says that `P` is a regular 600-cell. -/
structure CoordinateSources (IsBinaryIcosahedral IsRegular600Cell : Set ℍ → Prop) : Prop where
  /-- E1, the quaternion coordinate result: in suitable orthonormal quaternion coordinates, the
  binary icosahedral group is the displayed coordinate set. -/
  quaternionCoordinates : IsBinaryIcosahedral quaternionCoordinateSet
  /-- E2, the binary-group polytope result: the elements of a binary icosahedral subgroup of
  unit quaternions are the vertices of a regular 600-cell. -/
  binaryGroupPolytope : ∀ S, IsBinaryIcosahedral S → IsRegular600Cell (convexHull ℝ S)
  /-- Multiplication by 2 preserves regularity. -/
  regular_two_smul : ∀ P, IsRegular600Cell P → IsRegular600Cell ((2 : ℝ) • P)

/-- **Lemma 1.** `V` has 120 distinct elements, each of norm 2; `V / 2` lies in `Sp(1)` and is a
binary icosahedral subgroup; and the convex hull of `V` is a regular 600-cell. -/
theorem coordinateIdentification {IsBinaryIcosahedral IsRegular600Cell : Set ℍ → Prop}
    (h : CoordinateSources IsBinaryIcosahedral IsRegular600Cell) :
    vertexSet.ncard = 120 ∧ (∀ q ∈ vertexSet, ‖q‖ = 2) ∧
      (∀ q ∈ (2 : ℝ)⁻¹ • vertexSet, ‖q‖ = 1) ∧
      IsBinaryIcosahedral ((2 : ℝ)⁻¹ • vertexSet) ∧ IsRegular600Cell (convexHull ℝ vertexSet) := by
  have hbin : IsBinaryIcosahedral ((2 : ℝ)⁻¹ • vertexSet) := by
    rw [halfVertexSet]
    exact h.quaternionCoordinates
  refine ⟨vertexCount, vertexNorm, halfVertexSet_norm, hbin, ?_⟩
  have hV : vertexSet = (2 : ℝ) • ((2 : ℝ)⁻¹ • vertexSet) := by
    rw [smul_smul]
    norm_num
  rw [hV, convexHull_smul]
  exact h.regular_two_smul _ (h.binaryGroupPolytope _ hbin)

end ArchonFull600Theory
