# Rigid jumbling of the cell-cap 600-cell: theory draft

Status: **integrator's draft for workstream J3 (9 October 2026), verified once by the senior reviewer** (review `20261009T221306Z-4784adf6`): Propositions 1.1, 1.2, 2.2, 2.3 and 3.1–3.3 were found sound, and Proposition 1.3 and Theorem 2.1 sound once restricted to valid lattice configurations. Its five findings (J3V-1 to J3V-5) are adopted below. Scoped round 1 (`20261009T225939Z-05de315d`) found them fixed and raised two new majors, J3W-1 (cached centre replay) and J3W-2 (finiteness summaries), both adopted; scoped round 2 (`20261009T232508Z-5e6b0f1d`) found both fixed, Proposition 3.5 sound and no new blocker or major. It answers parts of the problem statement [`theory-problem.md`](../theory-problem.md) and marks the rest as open. Item numbers follow the problem statement. Scripts and results are in this folder (`*.py`, `results/`); every script runs from the repository root with Python 3 and NumPy, on top of the J1 reference (`research/jumbling/sim/`).

Labels used below:
- **Proved:** a proof is given here, from the definitions of the state contract and unconditional geometry.
- **Computed:** an exact computation in Q(√5) by the scripts named, with its algorithm stated. Exact means no floating-point decision.
- **Lead:** a sampled or randomised computation. It suggests, and proves nothing.
- **Open:** not settled.

Notation: P̃ is the 600-cell with facets n_e · x ≤ ‖n‖² (all poles have the same norm ‖n‖² = 12 + 4√5), κ = α‖n‖² with α = 121/125, H_c = {n_c · x = κ}, H_c⁺ = {n_c · x ≥ κ} and H_c⁻ = {n_c · x ≤ κ}. A chamber is the closure of a retained piece's home region Ā₀. The cap polytope is K_c = P̃ ∩ H_c⁺.

## Item 1. Groupoid structure and validity

**Definition.** A configuration is valid when the posed regions g_A(Ā₀) have pairwise disjoint interiors.

**Proposition 1.1 (proved).** An admissible twist maps a valid configuration to a valid one.
- Let (c, g) be admissible, with inside set I. Every region of a piece in I lies in H_c⁺, and every other region in H_c⁻.
- A full-dimensional convex set in a closed half-space has its interior in the open half-space. So an inside region and an outside region have disjoint interiors, before and after the twist.
- g fixes n_c, so it maps H_c⁺ and H_c⁻ onto themselves. An inside region stays in H_c⁺ after the twist.
- Two inside regions are moved by the same bijection g, and two outside regions are not moved. Either way their interiors stay disjoint.

**Proposition 1.2 (proved).** If (c, g) is admissible from X with inside set I, then (c, g⁻¹) is admissible from (c, g)X with the same inside set I, and it returns X.
- After the twist, the pieces of I lie in H_c⁺, because g preserves H_c⁺. The other pieces are unchanged and lie in H_c⁻.
- So no piece straddles H_c, and the inside set of (c, g⁻¹) is exactly I: a full-dimensional region cannot be both inside and outside.
- Applying g⁻¹ to I restores every pose.
- Admissible twists therefore form a groupoid on configurations. J1 asserts exactly this in `State.undo`.

**Lemma 1.2a (proved): chamber occupancy.** A valid lattice configuration occupies every chamber exactly once. Every configuration reachable from solved is valid.
- K⁺ permutes the poles, hence the cut hyperplanes and the facet hyperplanes, hence the chambers: k(C) is a chamber for every k ∈ K⁺ and every chamber C. In a lattice configuration every posed region g_A(Ā₀), g_A ∈ K⁺, is therefore a chamber.
- In a valid configuration two pieces have disjoint interiors, so they never occupy the same chamber: the map from pieces to occupied chambers is injective. There are as many pieces as chambers (each piece is one chamber at home), so the map is a bijection.
- Solved is valid, and admissible twists keep validity (Proposition 1.1), so every reachable configuration is valid.
- Validity is needed. The contract defines a lattice configuration by K⁺ poses alone: giving the 3,097 pieces of cap 0 the pose −I ∈ K⁺ and every other piece the identity is a lattice configuration that leaves cap 0 empty and stacks pieces elsewhere (review finding J3V-1, reproduced read-only with J1, which rejects its sticker projection as not bijective).

**Proposition 1.3 (proved).** On valid lattice configurations, a retained twist (c, a), a ∈ A4_c, acts on labels as the retained generator does.
- A valid lattice configuration occupies exactly the retained chambers, each by one piece (Lemma 1.2a). Each chamber lies on one side of H_c, so (c, a) is admissible, and its inside set is the set of pieces in chambers whose signature contains c.
- a ∈ K⁺ maps the chamber of signature M onto the chamber of signature a(M), and each host facet patch onto the patch on the image facet. Through the labelled projection π (contract A3), this is the retained generator's slot permutation.
- **Computed:** J1 checks this for all 1,200 generators against `primitives.npz`, `move_src` and `move_dst` (`sim/accept.py`, section 1).

## Item 2. Locality and blocking

**Theorem 2.1 (proved).** Let X be any valid lattice configuration (for example any lattice configuration reachable from solved) and (c, g) any twist with g ∈ SO(3)_c.
1. (c, g) is admissible from X. It moves exactly the pieces in the chambers that make up K_c.
2. After (c, g), a grip d ≠ c is blocked if and only if
   - w = g⁻¹ n_d is not a pole, and
   - the hyperplane Π = {w · x = κ} meets the interior of K_c.
3. Equivalently, d is blocked if and only if w is not a pole and min over the vertices v of K_c of w · v < κ < max of the same.
4. The blocked set depends only on (c, g), not on the valid lattice configuration X.

Proof.
- (1) Every chamber lies on one side of every cut hyperplane, and the chambers with c in their signature tile K_c.
- (2) The unmoved pieces are chambers, so they never straddle H_d. A moved piece occupies g(C) for a chamber C ⊆ K_c. Because g is orthogonal, n_d · g(y) = w · y, so g(C) straddles H_d exactly when C straddles Π.
  - If w = n_{d′} is a pole, Π = H_{d′} is a cut hyperplane, and no chamber straddles it. These are the realigned grips, the fixed poles included.
  - If w is not a pole, suppose first that Π meets int K_c. If Π met no chamber interior, the relatively open set Π ∩ int K_c would lie in the union of the chamber boundaries. Those lie in the finitely many arrangement hyperplanes: the 600 cut hyperplanes H_e and the 600 facet hyperplanes {n_e · x = ‖n‖²}. A non-empty relatively open subset of Π inside a finite union of hyperplanes forces Π to equal one of them (a finite union of nowhere dense closed sets is nowhere dense in Π).
    - Π = H_e would give w = n_e, because κ ≠ 0. That contradicts the assumption.
    - Π = {n_e · x = ‖n‖²} would give w = n_e / α, but ‖w‖ = ‖n_d‖ = ‖n_e‖ and α ≠ 1.
    - So Π meets the interior of some chamber C ⊆ K_c. A hyperplane through an interior point of a convex body leaves points on both sides, so C straddles Π, and d is blocked.
  - Conversely, if Π misses int K_c, every chamber C ⊆ K_c has its interior on one side of Π, hence C itself. No moved piece straddles H_d.
- (3) K_c is a full-dimensional polytope. A non-constant linear function takes values on both sides of κ over K_c exactly when its zero set meets int K_c, and its extremes over K_c are attained at vertices.
- (4) The argument uses only the fact that X occupies every chamber exactly once (Lemma 1.2a). Without validity the statement fails: in the configuration of Lemma 1.2a's last bullet, a quarter-turn of cap 0 moves nothing and leaves every grip admissible, where the theorem would give 16 blocked grips.

**Computed (exact).** `blocking.py`, results in `results/blocking.json`:
- K₀ has 40 vertices. They were computed by exact double description from the facets near n₀ and the cut H₀⁺, and each was checked against all 600 facet constraints, so the superset equals K₀.
- The criterion of Theorem 2.1 agrees with the J1 survey (independent region classification with certificates) on all 600 grips for eight twists of cap 0 from solved:

| Twist of cap 0 | Angle | Realigned poles | Blocked grips (J1) | Agreement |
| --- | ---: | ---: | ---: | ---: |
| plane rotation fixing poles 0 and 13 (witness E2) | 9.99987° | 12 | 54 | 600/600 |
| quarter-turn of S4₀ (Cayley ω = (1, 0, 0)) | 90° | 216 | 16 | 600/600 |
| fifth-turn of the icosahedral group I_a ⊃ A4₀ | 72° | 120 | 25 | 600/600 |
| fifth-turn of the icosahedral group I_b ⊃ A4₀ | 72° | 120 | 25 | 600/600 |
| four Cayley rotations with random rational ω | 144°–168° | 2 | 36–48 | 600/600 |

- This explains the witness fact of 54 blocked grips after the 9.99987° twist.

**Proposition 2.2 (proved): same-cap twists stay admissible.** After any admissible (c, g), every (c, g′) with g′ ∈ SO(3)_c is admissible and moves the same inside set (Proposition 1.2 applied to g′g⁻¹). Consecutive twists of one cap compose: (c, g′) after (c, g) equals (c, g′g).

**Proposition 2.3 (proved): shared fixed poles.** Let grip c be admissible in a configuration Y, and let h fix both n_c and n_d. Then after an admissible (d, h), grip c is still admissible.
- Every piece of Y lies in H_c⁺ or in H_c⁻. h maps both half-spaces onto themselves, so every piece still lies in one of them after (d, h).

**Explanation of the second witness fact.** In witness E3, g fixes n₀ and n₁₃, and T is a third-turn in A4₁₃.
- After (0, g), (13, T), cap 0 is blocked. T does not fix n₀, so it carries g-rotated pieces of K₀ ∩ K₁₃ across H₀ (Theorem 2.1 with the moved set T g(K₀ ∩ K₁₃)).
- (13, g ∘ T⁻¹) is a twist of grip 13, because g fixes n₁₃. By Proposition 2.2 it composes with (13, T) to the net twist (13, g). The configuration is therefore (0, g) followed by (13, g).
- By Proposition 2.3, cap 0 is admissible there.
- The configuration is not the earlier one: the pieces of K₁₃ now carry g or g².

**Proposition 2.4 (proved): blocking in any configuration.** In any configuration, grip d is blocked if and only if some pose class P (pieces with a common pose g_P, home chambers S_P) has w = g_P⁻¹ n_d not a pole and the hyperplane {w · x = κ} meeting the interior of a chamber of S_P.
- A piece with pose g and home chamber C straddles H_d exactly when C straddles {w · x = κ}, because g is orthogonal (n_d · g(y) = w · y).
- If w is a pole, that hyperplane is a cut hyperplane, and no chamber straddles it.
- Otherwise a convex body straddles a hyperplane exactly when the hyperplane meets its interior.
- The union over the pieces of P, and then over the pose classes, gives the criterion. It is a test per chamber, so S_P need not be convex. J1's anchor-cap superset test implements a sound version of it.

## Item 3. Finiteness of R(Λ)

**Proposition 3.1 (proved): same cap.** The configurations reached by twists of one cap c from solved are in bijection with ⟨Λ_c⟩ (Proposition 2.2, and distinct rotations move the non-empty cap differently). This set is finite exactly when ⟨Λ₀⟩ is a finite subgroup of SO(3). Finite rotation groups are cyclic, dihedral, tetrahedral, octahedral or icosahedral. Those containing the tetrahedral group A4₀ are:
- A4₀ itself;
- S4₀ = N(A4₀), the unique octahedral group containing it;
- the two icosahedral groups containing it.

**Computed (exact).** In the cap frame of amendment A1, A4₀'s three half-turns are the diagonal sign matrices, so S4₀ and both icosahedral groups I_a, I_b have entries in {0, ±1, ±1/2, ±φ/2, ±1/(2φ)} ⊂ Q(√5). All 24, 60 and 60 elements were lifted to exact four-dimensional rotations fixing n₀ (`groups.py`). The group closure itself is a float search; every element is then identified with an exact value from the list above and checked exactly to be a rotation fixing n₀, and each set is checked exactly to be closed under multiplication and to contain A4₀.

**Proposition 3.2 (proved): exact negative control.** A same-cap menu of finite-order elements can give an infinite R.
- g = 2P₀,₁₃ − I is a half-turn fixing n₀ and n₁₃, and g ∉ A4₀.
- With H₀ the retained half-turn of cap 0 (generator 0), the four-dimensional trace is tr(H₀ g) = 4/3 (computed exactly).
- A rotation of finite order has a trace that is a sum of roots of unity, hence an algebraic integer. 4/3 is not one, so H₀g has infinite order.
- By Proposition 3.1, the menu generated by A4₀ and g, closed under inverses and A4 conjugation, gives infinitely many configurations at cap 0 alone.

**Proposition 3.3 (proved): the generated group is infinite.** For any menu not contained in A4₀, the group generated by all the transported menus is infinite.
- **Computed:** the 600 groups A4_c generate all 7,200 elements of K⁺ (an exact closure in J1's K⁺ table).
- So the group strictly contains K⁺. K⁺ is a maximal finite subgroup of SO(4): a finite overgroup lifts to a finite subgroup of SU(2) × SU(2) whose two projections contain the binary icosahedral group 2I, which is maximal finite in SU(2).
- This alone does not make R(Λ) infinite, because admissibility restricts which products occur.

**Corollary 3.4 (proved): a menu contained in none of the three groups is infinite.** If the base-cap menu Λ₀ is not contained in S4₀, I_a or I_b, then R(Λ) is infinite already at cap 0. ⟨Λ₀⟩ contains A4₀, so by Proposition 3.1 it is finite only when it is one of A4₀ ⊂ S4₀, S4₀, I_a or I_b, and ⟨Λ₀⟩ ⊆ G exactly when Λ₀ ⊆ G.
- **Computed (exact), the E2 witness** (`invariants.py`, `results/invariants.json`): the plane rotation g of witness E2 (fixing n₀ and n₁₃, 9.99987°) has tr g = (3426525189964 − 4987013400√5)/860378847541, whose field trace to Q, 6853050379928/860378847541, is not an integer. So tr g is not an algebraic integer, g has infinite order, and every menu containing g gives infinite R at cap 0 (review finding J3V-3).
- **Computed (exact), the explorer menus** (`research/jumbling/explorer/`, field `j1_relation`, exact matrix sets in J1's frame): class-00 ⊂ S4₀, class-03 ⊂ I_a and class-04 ⊂ I_b. Every other exact realignment menu (class-01, class-02 and class-05 to class-32) and the exact plane menus (plane-10, plane-36, plane-72) lie in none of the three groups, so each of them gives infinite R.
**Proposition 3.5 (proved): a menu reaches what its generated group reaches.** For every menu Λ, R(Λ) = R(Λ̄), where Λ̄ is the menu whose base-cap set is the group ⟨Λ₀⟩.
- Λ₀ ⊆ ⟨Λ₀⟩, so R(Λ) ⊆ R(Λ̄).
- Conversely, let (c, h) with h ∈ ⟨Λ_c⟩ be admissible from X. Admissibility depends only on the cut: (c, g) is admissible when every piece is inside or outside H_c (state contract, line 47), whatever g is. Λ_c is closed under inverses, so h = g_k ⋯ g_1 with every g_i ∈ Λ_c. (c, g_1) is admissible from X; by Proposition 2.2 each further (c, g_i) is admissible and the sequence composes to (c, h). So every step of a Λ̄ sequence is reproduced by Λ twists, and R(Λ̄) ⊆ R(Λ).
- The transported menus agree: K⁺-conjugation is a group homomorphism, so the transport of ⟨Λ₀⟩ to cap c is ⟨Λ_c⟩.
- **Which groups occur.** A4₀ has index 2 in S4₀, so no group lies strictly between them. A4 is a maximal subgroup of the icosahedral rotation group (A5), so every group strictly between A4₀ and I_a is I_a, and likewise for I_b. S4₀ ∩ I_a = S4₀ ∩ I_b = I_a ∩ I_b = A4₀, because each intersection is a group containing A4₀ and contained in two different groups of the list. Hence, for A4₀ ⊊ Λ₀ ⊆ G with G one of S4₀, I_a, I_b, ⟨Λ₀⟩ = G and R(Λ) = R(G).
- **Classification (review finding J3W-2).** Every menu containing A4₀ falls in exactly one case:
  - Λ₀ = A4₀: finite. Every reachable pose stays in K⁺, so there are at most 7200^177120 configurations.
  - ⟨Λ₀⟩ = S4₀, I_a or I_b: R(Λ) = R(S4₀), R(I_a) or R(I_b). For the explorer's menus, class-00 gives R(S4₀), class-03 gives R(I_a) and class-04 gives R(I_b). Finiteness is open for these three.
  - Λ₀ contained in none of S4₀, I_a and I_b: infinite (Corollary 3.4).
- The explorer's ball-model counts are consistent with this: the lattice closures of class-00, class-03 and class-04 agree with those of S4₀, I_a and I_b through depth 5. They are leads, not part of the proof.

**Leads (randomised, exact states).** Exact random walks of J(S4₀), J(I_a) and J(I_b), with uniformly random caps and jumble elements (`walk.py`, seeds 1–3, logged every five applied twists; summaries in `results/walk-*.json`, and the journals of the three longest runs are the W-J fixtures in `research/jumbling/fixtures/`):
- In the long runs (seeds 2 and 3), 93–96% of attempted jumble twists are blocked.
- After 620–885 applied twists:
  - 110,000–128,000 of the 177,120 pieces are off the lattice (at most 129,941 at a logged step);
  - the pose table holds 1,225–1,591 distinct poses;
  - the largest logged coefficient or denominator of a non-K⁺ pose is 16 for I_a, 32 for S4₀ in both long runs, and 32 at one logged step of one I_b run.
- Slowly growing heights would be consistent with infinite R. Bounded heights would imply finitely many poses. The data decide neither.

**Open (item 3).** Finiteness of R(S4₀), R(I_a) and R(I_b). By Proposition 3.5 these three decide every menu containing A4₀ that Corollary 3.4 does not settle. A multi-cap criterion for them is also open.

## Item 4. Lattice states reached through jumbling

**Lead (exact states, exhaustive over one word family).** Words W = (0, q)(d, a)(0, q⁻¹), with q the quarter-turn of S4₀ (Cayley ω = (1, 0, 0)) or a fifth-turn of I_a, over every grip d ≠ 0 and every non-identity a ∈ A4_d (6,589 words per menu; `conj.py`, `conj2.py`, results in `results/conj-*.json` and `results/conj2.json`):

| Menu | (d, a) blocked | W ends off the lattice | W ends on the lattice, state changed | Equal to one retained twist | Other lattice states |
| --- | ---: | ---: | ---: | ---: | ---: |
| S4₀ | 176 | 432 | 5,981 | 5,973 | 8 |
| I_a | 275 | 333 | 5,981 | 5,973 | 8 |

- (0, q⁻¹) was never blocked in these words.
- The 8 other lattice states (d ∈ {24, 42, 74, 108}, two third-turns each, the same for both menus) move 2,808 pieces, which no single retained twist does: a retained twist moves 3,097.
- They keep every centre in its own chamber, as Proposition 4.1 requires of every reachable state, and the positional permutation of every K⁺-orbit is even (`conj.py`, exact on the K⁺ poses). The retained orientation invariants were not computed.
- Without a witnessed retained word, J1 correctly refuses them as checkpoints.
- **Open:** whether they lie in G·solved. Settling it needs the orientation invariants of the blueprint (a necessary test) and, for a positive answer, a witness word.

**Proposition 4.1 (proved): centre points are fixed.** In every configuration reachable from solved, the centre piece of each cap c (the piece with signature {c}) has a pose g_c with g_c n_c = n_c. Centres can rotate in place about their own pole, but never move off it (review finding J3V-2).
- The closed home region of the centre of c contains the point p_c = αn_c: n_c · p_c = κ, and n_e · p_c = α n_e · n_c ≤ κ and ≤ ‖n‖² for every pole e (computed exactly for all 600 centres, `invariants.py`).
- The posed point x = g_c(p_c) has norm α‖n‖. Suppose an admissible twist (d, h) has the centre of c in its inside set. Then the posed region lies in H_d⁺, so n_d · x ≥ κ = α‖n‖². Cauchy–Schwarz gives n_d · x ≤ ‖n_d‖ ‖x‖ = α‖n‖². So equality holds, x = αn_d, and h fixes x because h fixes n_d.
- A twist leaves every piece outside its inside set unchanged. By induction from solved, x = p_c after every twist, so g_c(αn_c) = αn_c.
- **Consequence.** A configuration rotated globally by k ∈ K⁺, k ≠ 1, is not reachable: the 600 poles span four-space, so k n_c = n_c for every c forces k = 1. In a reachable lattice configuration every centre's pose lies in K⁺ ∩ Stab(n_c) = A4_c.
- **Computed (exact, evidence, not part of the proof):** J1 replays of the three W-J journals check g_c n_c = n_c for every centre after every applied twist (`invariants.py --replay`, `results/invariants.json`). All 2,366 twists pass (640 for S4₀, 841 for I_a, 885 for I_b). The replay keeps no cache: after every twist it checks every centre whose pose is not the identity, 53,795, 56,659 and 65,632 checks, over 509, 743 and 769 distinct exact (centre, pose) pairs. At most 99, 79 and 86 centres are rotated in place at once; at the end, 97, 79 and 86, none off its pole.

**Proved remark.**
- The retained invariants (positional parity, orientation abelianization, the 2⁴³ · 5² quotient) are necessary conditions only. Membership of a lattice state in G·solved needs a witnessed word (contract section 5), not an invariant check.

## Item 5. Defect and return to the lattice

**Proved.** Reversing the journal always returns to solved (Proposition 1.2).

**Definitions proposed.** The defect δ(X) is the set of pose classes {(g_P, S_P)} with g_P ∉ K⁺, and |δ| is the number of off-lattice pieces. δ(X) = ∅ exactly when X ∈ L.

**Open.** A bound on the return distance in terms of a visible defect, and a procedure that uses visible information only. The leads above show that a scramble reaches about two-thirds of the pieces off the lattice within a few hundred twists.

## Item 6. Solving theory

**Proved consequences for the workflow.**
- "Return to lattice, then block-build" is correct only from a witnessed retained checkpoint (contract section 5). Reaching L is not enough (item 4).
- Protection: an admissible twist (c, g) changes the pose of a piece exactly when the piece is in its inside class and g ≠ I (review finding J3V-5: the identity is in every menu and changes nothing). A protected block (Net or Strict, on poses) is kept by every admissible twist whose cut leaves the whole block in H_c⁻. Proposition 2.3 adds that rotations fixing several poles keep those grips usable.

**Open.** Block preservation through jumble twists, and per-phase certificates beyond those of the contract.

## Item 7. Scope

- Not settled: the finiteness and size of R(S4₀), R(I_a) and R(I_b), which decide every remaining menu (Proposition 3.5: A4₀ is finite, a menu contained in none of the three groups is infinite, every other menu reaches what one of the three groups reaches), worst-case distances, the membership question of item 4 for any menu, and items 5 and 6 beyond the remarks above.
- No performance, rendering or Windows claims.
