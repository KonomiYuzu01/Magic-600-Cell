# State contract for rigid jumbling of the 600-cell-Full geometry

Status: **draft for the senior reviewer's scoped re-check.** This contract defines what the first geometric witness and any later simulator must compute. It answers plan-check findings Q3, Q4 and Q5. It is not accepted theory and changes nothing in the retained model.

Scope: the rigid, unfudged jumbling extension of the retained cut geometry (α = 121/125). Owner decision, 9 October 2026: no fudged variant is pursued. The extension is studied as a separate puzzle concept. 600-cell-Full, its 1,200 legal generators, its 259,800 labelled slots and `assets/manifest.json` are unchanged.

Terms follow `research/theory/RETHLAS_BLUEPRINT.md`: pole n_c, cap U_c, polytope P̃, complete cap signature M(A), the cap rotation group A4_c, K⁺ and the retained legal group G.

## 1. Grips and twists

- **Grips are world-fixed.** The grips are the 600 retained poles n_c in a fixed world frame, each with the cut hyperplane n_c · x = α‖n_c‖². Cut planes carried by moved pieces are not grips. They matter only through the moved piece regions.
- **Twist.** A twist is a pair (c, g) with g ∈ SO(3)_c, the rotations of four-space that fix n_c. A retained twist has g ∈ A4_c; a jumble twist has g ∉ A4_c.
- **Twist alphabets.**
  - The continuous model allows every g ∈ SO(3)_c and is used for theory only.
  - Computations use explicit finite menus. Every result names its menu.

## 2. Pieces and their regions

- **Pieces.** The pieces are the 177,120 retained pieces, the 600 cell centres included, each with its stickers attached rigidly.
- **Home region.** The home region of piece A is Ā₀ = cl(int P̃ ∩ ⋂_{c∈M(A)} U_c ∩ ⋂_{c∉M(A)} {n_c · x < α‖n_c‖²}). It is computed as a convex H-polytope from the signature in `assets/model.npz`, followed by vertex enumeration.
  - Lemma 6 makes the signature determine the chamber within the retained model.
  - The model files do not contain piece regions; they are reconstructed here.
  - The reconstruction is accepted only where its facet patches agree with the retained sticker data. Section 6 states that check.
- **Pose.** A configuration assigns every piece a pose g_A ∈ SO(4). The posed region is g_A(Ā₀). The solved configuration has every pose equal to the identity.
- **Equality.** Two configurations are equal when every labelled piece has the same pose. No symmetry quotient is taken.
- **Lattice configuration.** A configuration is on the lattice when every g_A ∈ K⁺ and every g_A(Ā₀) is a retained chamber closure (a slot position).

## 3. Admissibility

For a twist (c, g) in configuration X, let h_A^min and h_A^max be the minimum and maximum of (n_c · x)/‖n_c‖² − α over the vertices of g_A(Ā₀). This is the signed offset from the cut as a fraction of the facet distance. Each piece is classified with a numerical tie band τ = 1e-9 and a certified separation margin δ = 1e-6:

| Class | Condition |
| --- | --- |
| inside | h_A^min ≥ −τ |
| outside | h_A^max ≤ τ |
| straddling | h_A^min ≤ −δ and h_A^max ≥ δ |
| uncertain | anything else |

- **Outcome.**
  - The twist is admissible when every piece is inside or outside.
  - It is certified blocked when some piece straddles.
  - It is rejected as uncertain otherwise.
  - A blocked or uncertain twist leaves the configuration unchanged.
- **Effect.** An admissible twist replaces g_A by g ∘ g_A for every inside piece and leaves outside pieces unchanged.
- **No collisions.** g fixes n_c, so it maps both closed half-spaces onto themselves. Inside pieces stay in one closed half-space and outside pieces in the other, so interiors stay disjoint. No other contact rule is needed.
- **Shape changes.** The outer shape of the puzzle is no longer P̃ after a jumble twist. This is allowed.
- **Cell centres.** In the retained puzzle the centres are fixed by G. Under a jumble twist the centre of c is rotated off the lattice like any other inside piece.

Neither condition below is a definition of admissibility. Admissibility is always decided by the region classification above.
- A grip d whose pole is fixed by g, or with g(n_{d′}) = n_d for some pole d′, stays admissible after (c, g) from a lattice configuration. These are the realignment circles.
- A grip whose hyperplane misses every moved region stays admissible too. This gives open sets of admissible rotations (plan-check finding C5-Q2).

## 4. What "all jumbling states" means here

- **Reachable configurations.** These are the endpoints of finite admissible twist sequences from the solved configuration.
  - With the continuous alphabet they form an uncountable set, even modulo K.
  - With a finite menu the set is at most countable.
  - The infinite global grip closure (C5) is a separate fact about poles and does not by itself describe reachable configurations.
- **Finite object to classify instead (Q3).** The first-order legality strata of a stated finite grip list at a stated configuration. For each twisting grip c, SO(3)_c is partitioned by which grips are admissible afterwards, and each stratum records:
  - its witnesses;
  - the moved piece identities;
  - the contact events at its boundary.
- **Limits of the 33 classes and of bitmaps.** The 33 two-pair realignment classes are one subset of these strata. A blocked-grip bitmap alone is not a sound quotient for later moves.

## 5. Handoff to the retained solving workflow (Q4)

- **Checkpoint.** Orbit-first block building and every retained certificate (R, F, I, C, E; parity and orientation sums; controller and setup tables) apply only from a retained-state checkpoint.
  - A retained-state checkpoint is a lattice configuration with a witnessed legal word in the 1,200 retained generators from solved.
  - Equivalently, it is a lattice configuration certified equal to a state reached by such a word.
- **Returning to the lattice is not enough.** A lattice configuration reached through jumble twists is not assumed to lie in the retained state orbit. Reachability sufficiency is open in the blueprint, and passing the necessary invariants is not a certificate.
- **Audit before transferring invariants.** Before any invariant is transferred, short admissible off-lattice excursions that return to the lattice are audited for new lattice actions. Any net slot permutation that violates a retained invariant would show that jumbling reaches lattice states outside G.
- **Human-solve boundary.** The boundary is unchanged:
  - tools may track, check and protect;
  - blocks, full collateral and Net/Strict protection are evaluated on actual poses and admissible paths;
  - the program never chooses or executes an unjumbling or solving sequence by itself.

## 6. First acceptance experiment: a minimal witness (Q5)

Stop after this experiment and the initial impact assessment. A general simulator and rendering wait for a separate go-ahead.

1. **E1 Regions.** Build Ā₀ for every piece whose signature contains c or d, where d is a face-neighbour pole of c.
   - Check that each region is non-empty and full-dimensional.
   - Check that its patches on the facets in Host(A) contain the retained slot centres `slot_centers` of exactly its stickers.
   - Check that no slot centre lies in two regions.
2. **E2 Jumble twist.** Let g be the rotation by 10° that fixes span(n_c, n_d) pointwise.
   - Verify g ∉ A4_c.
   - Apply (c, g) from solved, with every piece classified with margins.
3. **E3 Turn at the neighbour.** Apply (d, T_d), with T_d a third-turn in A4_d. It must be certified admissible.
   - Record, with certificates, which of c and its 56 interacting poles are admissible, blocked or uncertain after (c, g) alone and after (c, g), (d, T_d).
   - At least one certified blocked grip is required as a negative control. Applying it must be rejected with the configuration unchanged.
4. **E4 Reverse.** Apply (d, T_d⁻¹) and then (c, g⁻¹). Both must be certified admissible, and every pose must return to the identity within 1e-12.

Round trips alone are not sufficient acceptance (Q5). E1's agreement with retained data and E3's certified blocked and unblocked cases carry the evidence.

Acceptance for a later simulator, not for this experiment:
- full labelled sticker and frame agreement with `rotperms`, `move_src` and `move_dst` for all 1,200 retained generators;
- independently certified blocked and unblocked cases;
- conservative handling of uncertain contacts;
- rejection without any state change.
