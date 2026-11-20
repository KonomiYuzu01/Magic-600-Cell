import ArchonFull600Theory.Poles
import Mathlib.Algebra.Category.ModuleCat.Abelian
import Mathlib.Algebra.Category.ModuleCat.Colimits
import Mathlib.AlgebraicTopology.FundamentalGroupoid.SimplyConnected
import Mathlib.AlgebraicTopology.SingularHomology.Basic
import Mathlib.Analysis.Convex.GaugeRescale

set_option linter.style.header false in
/-!
# Lemma 2: the radial topology of the boundary

Source: `research/Full_600cell_Puzzle_Theory.tex`, Lemma 2 (lines 384 to 435).

`P` is convex and compact, and it contains the closed ball of radius `φ² / √2` about `0`
(`closedBall_inradius_subset`, Lemma 5), so `0` is an interior point. The radial map
`x ↦ x / ‖x‖` is then a homeomorphism from the boundary `∂P` onto the unit sphere `S³ ⊆ ℍ`
(`boundaryHomeomorph`); it is the restriction of Mathlib's gauge rescaling homeomorphism
`gaugeRescaleHomeomorph` from `P` to the closed unit ball, which maps the frontier of `P` onto the
unit sphere (`image_polytopeRescale_frontier`).

The homology and the fundamental group of `S³` are the standard topology E3 of the handoff. They
enter as the hypothesis `SphereTopology`, and `radialTopology` transfers them to `∂P` along the
homeomorphism. The third part of E3 (a continuous bijection from a compact space to a Hausdorff
space is a homeomorphism) is not needed as a hypothesis: the homeomorphism is proved.
-/

namespace ArchonFull600Theory

open scoped goldenRatio Quaternion RealInnerProductSpace Topology
open CategoryTheory Limits

/-! ### `P` as a convex body -/

theorem convex_polytope : Convex ℝ polytope := convex_convexHull ℝ _

theorem isCompact_polytope : IsCompact polytope := by
  rw [polytope, ← range_vertexR]
  exact (Set.finite_range vertexR).isCompact_convexHull (𝕜 := ℝ)

theorem isClosed_polytope : IsClosed polytope := isCompact_polytope.isClosed

theorem polytope_mem_nhds_zero : polytope ∈ 𝓝 (0 : ℍ) :=
  Filter.mem_of_superset (Metric.closedBall_mem_nhds 0 inradius_pos) closedBall_inradius_subset

theorem zero_mem_interior_polytope : (0 : ℍ) ∈ interior polytope :=
  mem_interior_iff_mem_nhds.mpr polytope_mem_nhds_zero

theorem isVonNBounded_polytope : Bornology.IsVonNBounded ℝ polytope :=
  NormedSpace.isVonNBounded_of_isBounded ℝ
    (Metric.isBounded_closedBall.subset polytope_subset_closedBall)

/-! ### The radial homeomorphism -/

/-- The gauge rescaling homeomorphism of `ℍ` that sends `P` onto the closed unit ball. -/
noncomputable def polytopeRescale : ℍ ≃ₜ ℍ :=
  gaugeRescaleHomeomorph polytope (Metric.closedBall 0 1) convex_polytope polytope_mem_nhds_zero
    isVonNBounded_polytope (convex_closedBall 0 1) (Metric.closedBall_mem_nhds 0 one_pos)
    (NormedSpace.isVonNBounded_closedBall ℝ ℍ 1)

theorem image_polytopeRescale_frontier :
    polytopeRescale '' frontier polytope = Metric.sphere 0 1 := by
  rw [← frontier_closedBall (0 : ℍ) one_ne_zero, ← closure_sdiff_interior,
    ← closure_sdiff_interior, Set.image_sdiff polytopeRescale.injective]
  rw [polytopeRescale, image_gaugeRescaleHomeomorph_interior,
    image_gaugeRescaleHomeomorph_closure]

theorem polytopeRescale_apply_of_mem_frontier {x : ℍ} (hx : x ∈ frontier polytope) :
    polytopeRescale x = ‖x‖⁻¹ • x := by
  change gaugeRescale polytope (Metric.closedBall 0 1) x = _
  rw [gaugeRescale_def, (gauge_eq_one_iff_mem_frontier convex_polytope
    polytope_mem_nhds_zero).mpr hx, gauge_closedBall zero_le_one, div_one, one_div]

/-- **Radial homeomorphism.** The map `x ↦ x / ‖x‖` is a homeomorphism from `∂P` onto `S³`. -/
noncomputable def boundaryHomeomorph : frontier polytope ≃ₜ Metric.sphere (0 : ℍ) 1 :=
  (polytopeRescale.image (frontier polytope)).trans
    (Homeomorph.setCongr image_polytopeRescale_frontier)

theorem boundaryHomeomorph_apply (x : frontier polytope) :
    (boundaryHomeomorph x : ℍ) = ‖(x : ℍ)‖⁻¹ • (x : ℍ) :=
  polytopeRescale_apply_of_mem_frontier x.2

/-! ### The topology of `S³` (E3) and Lemma 2 -/

/-- Integral singular homology `H_n(X; ℤ)`. -/
noncomputable abbrev intHomology (n : ℕ) (X : TopCat.{0}) : ModuleCat.{0} ℤ :=
  ((AlgebraicTopology.singularHomologyFunctor (ModuleCat.{0} ℤ) n).obj (ModuleCat.of ℤ ℤ)).obj X

/-- **E3** (the standard topology of `S³` that Lemma 2 uses), as a hypothesis: the unit sphere
`S³ ⊆ ℍ` is simply connected (the source uses van Kampen), and its integral singular homology is
`ℤ` in degrees 0 and 3 and zero in every other degree (the source uses the CW structure with one
0-cell and one 3-cell). -/
structure SphereTopology : Prop where
  simplyConnected : SimplyConnectedSpace (Metric.sphere (0 : ℍ) 1)
  homology_eq : ∀ n, n = 0 ∨ n = 3 →
    Nonempty (intHomology n (TopCat.of (Metric.sphere (0 : ℍ) 1)) ≅ ModuleCat.of ℤ ℤ)
  homology_isZero : ∀ n, n ≠ 0 → n ≠ 3 →
    IsZero (intHomology n (TopCat.of (Metric.sphere (0 : ℍ) 1)))

theorem intHomology_iso (n : ℕ) :
    Nonempty (intHomology n (TopCat.of (frontier polytope)) ≅
      intHomology n (TopCat.of (Metric.sphere (0 : ℍ) 1))) :=
  ⟨((AlgebraicTopology.singularHomologyFunctor (ModuleCat.{0} ℤ) n).obj
    (ModuleCat.of ℤ ℤ)).mapIso (TopCat.isoOfHomeo boundaryHomeomorph)⟩

/-- **Lemma 2.** `0` is an interior point of `P`, `P` is compact and lies in the ball of
radius 2, and `x ↦ x / ‖x‖` is a homeomorphism from `∂P` onto `S³`. Under E3
(`SphereTopology`), `H_j(∂P; ℤ)` is `ℤ` for `j = 0, 3` and zero otherwise, and `∂P` is simply
connected, so `π₁(∂P) = 1`. -/
theorem radialTopology (h : SphereTopology) :
    (0 : ℍ) ∈ interior polytope ∧ IsCompact polytope ∧ polytope ⊆ Metric.closedBall 0 2 ∧
      (∃ e : frontier polytope ≃ₜ Metric.sphere (0 : ℍ) 1,
        ∀ x, (e x : ℍ) = ‖(x : ℍ)‖⁻¹ • (x : ℍ)) ∧
      (∀ n, n = 0 ∨ n = 3 →
        Nonempty (intHomology n (TopCat.of (frontier polytope)) ≅ ModuleCat.of ℤ ℤ)) ∧
      (∀ n, n ≠ 0 → n ≠ 3 → IsZero (intHomology n (TopCat.of (frontier polytope)))) ∧
      SimplyConnectedSpace (frontier polytope) ∧
      ∀ x : frontier polytope, Subsingleton (FundamentalGroup (frontier polytope) x) := by
  have hsc : SimplyConnectedSpace (frontier polytope) :=
    have := h.simplyConnected
    boundaryHomeomorph.toHomotopyEquiv.simplyConnectedSpace
  refine ⟨zero_mem_interior_polytope, isCompact_polytope, polytope_subset_closedBall,
    ⟨boundaryHomeomorph, boundaryHomeomorph_apply⟩, fun n hn => ?_, fun n h0 h3 => ?_, hsc,
    fun x => inferInstance⟩
  · obtain ⟨e⟩ := intHomology_iso n
    obtain ⟨f⟩ := h.homology_eq n hn
    exact ⟨e ≪≫ f⟩
  · obtain ⟨e⟩ := intHomology_iso n
    exact (h.homology_isZero n h0 h3).of_iso e

end ArchonFull600Theory
