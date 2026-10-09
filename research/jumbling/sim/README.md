# J1 general simulator (exact reference engine for rigid jumbling)

Status: **candidate for review (9 October 2026).** It implements J1 acceptance items 1–9 of [`docs/progress/1.0/jumbling-plan.md`](../../../docs/progress/1.0/jumbling-plan.md) against the plan-checked [state contract](../state-contract.md), revision `state-contract 2026-10-09 A1-A4`. A2 filtering is off by default. Nothing here changes 600-cell-Full, `assets/` or any root module.

Evidence kind: source and synthetic geometry from a headless cloud session. Nothing here is Windows, Direct3D, input or performance evidence.

## Run

From the repository root, with Python 3 and NumPy only:

```text
python tests/test_jumbling_sim.py        # focused headless tests; target under 60 s
python research/jumbling/sim/accept.py   # full acceptance, writes acceptance.json
```

The full run uses four workers. Its measured duration, model hashes, exact menu identities, replayable journals and all acceptance flags are written to `acceptance.json`.

```python
import sys; sys.path.insert(0, 'research/jumbling')
import sim
ctx = sim.get_context()
st = sim.State(ctx)                                   # solved; filtered=False (A2 off)
out = st.apply(sim.plane(ctx, 0, 13, degrees=10))     # witness E2 twist
grips = st.survey()                                   # status of all 600 grips, with certificates
st.undo()                                             # exact inverse
```

## Layout

| File | Content |
| --- | --- |
| `model.py` | Read-only model data and SHA-256 identities for the asset bytes as read; exact poles, signatures, hosts, centres and the (piece, facet) to slot map |
| `kplus.py` | K⁺ (order 7200) as pole permutations, with exact matrices on demand; composition, inverses, A4_c, generator words, exact membership test |
| `regions.py` | K⁺-orbits of pieces; exact regions of the 36 representatives (double description from `witness.build_region`); lazy, cached exact transport to any piece |
| `kernel.py` | Exact integer sign kernel over Q(√5); the optional filtered sign test (A2) with its error bound |
| `twists.py` | Retained, plane, Cayley and half-turn twists; globally minimising bounded input map (A1); validated exact menus (A4) |
| `state.py` | Configurations: poses, admissibility, apply, survey, journal, undo, replay, snapshots, checkpoints, labelled stickers and frames (A3) |
| `accept.py` | The acceptance run; `acceptance.json` is its output |

## Model and rules

- **State** (contract section 2). Each piece has an exact pose. Poses are interned in a pose table and each piece stores a pose id (`int32`). Id 0 is the identity, so no matrix is stored for a piece in its home pose. A pose in K⁺ is stored by its K⁺ index, any other pose by its exact 4 × 4 matrix over Q(√5). The lattice flag of a piece is "pose in K⁺". Equal configurations with equal journals serialise to equal bytes, because the pose table is kept canonical (unused poses dropped, entries ordered by exact key).
- **Regions.** The 177,120 pieces fall into 36 K⁺-orbits. Each orbit is exactly one retained orbit class (`orbit_id`); the 600 fixed centres form one K⁺-orbit. So K⁺-orbits are unions of G-orbits. One representative per orbit, in cap 0, gets an exact double description (5 to 24 vertices). Every other piece's region is the representative's region moved by the K⁺ element that maps signature to signature. Transported vertices are built only when needed and cached (LRU), so the 177,120 regions never exist at once.
- **Admissibility** (contract section 3), for a twist (c, g):
  - A lattice piece is classified by its transported signature. Its region is the retained chamber with that signature, which the constraint n_c · x ≥ κ (or ≤ κ) itself cuts out. This is the **signature certificate**.
  - Off-lattice pieces with one pose form a pose group. The group is split by anchor caps a, where a lies in the signature of every piece of the subgroup. The posed cap region g(Ū_a ∩ P̃) contains every posed region of the subgroup. An exact one-sided bound over its 40 exact vertices certifies the whole subgroup (**group superset**).
  - Otherwise each piece is bounded over the complete exact vertex set of its posed region (**piece vertices**).
  - `classify_per_piece` is the reference path: every off-lattice piece is evaluated individually over its complete vertices, with exact signs, no group bounds, no anchors and no filter. Lattice pieces keep their defining signature certificate. The reference defaults to evaluating every piece even after a straddle is found.
  - A straddling piece gives two exact vertices, one with h < 0 and one with h > 0. Each vertex is re-checked against all 600 cut and 600 facet constraints of the posed region before the certificate is issued.
  - A piece without a certificate makes the twist uncertain. That happens only for a rotation with no exact representation (`UnrepresentableTwist`) or an exhausted exact work budget. Blocked and uncertain twists change nothing. A commit is all-or-nothing.
- **Twists.**
  - Retained twists are the 1,200 generators and every A4_c element, with generator words.
  - Plane rotations use the witness construction for any s ∈ Q(√5).
  - Cayley rotations (A1) use R = I + 2(W + W²)/(1 + |ω|²) in the exact frame u₁ = (−n₁, n₀, −n₃, n₂), u₂ = (−n₂, n₃, n₀, −n₁), u₃ = (−n₃, −n₂, n₁, n₀). As quaternions these are i·n, j·n and k·n.
  - Half-turns use `half_turn(ctx, c, axis)`, with three exact cap-frame coordinates, and H_u = 2(P_c + P_u) − I. The axis must be nonzero. The result is its own inverse; journal family `half_turn` records its exact axis.
  - A supplied K⁺ index is checked against the exact twist matrix before it can select the retained composition path. Omitted indices are derived exactly; −1 is valid only for a matrix outside K⁺.
  - `cayley_axis_angle` maps a numerical axis and angle request to the global Frobenius minimum with numerator bound `max_num` (N, default 16) and common denominator bound `max_den` (D, default 1000). Within 1° of a half-turn it also searches exact half-turn axes. It reports the realised cap-frame axis, angle and distance. Only exactly equal distances are ties; they go to the smaller denominator, then the lexicographically smaller numerator triple. The identity may be the minimum for small requests. See the pruning and screening proof below.
  - A plane rotation with parameter s about pole d is exactly the Cayley rotation with ω = −s(uᵢ · n_d). The tests check this.
  - `TwistMenu` transports a base-cap menu by the retained frames, Λ_c = F_c Λ₀ F_c⁻¹. `close=True` adds all A4₀ elements, inverses and A4₀ conjugates. `close=False`, including record loading, refuses any missing requirement. Records carry `inverse_closed`, `a4_invariant`, `contains_a4` and a SHA-256 identity over the sorted canonical exact matrices, independent of labels and input order. `a4(ctx)` has 12 elements including identity; `s4(ctx)` has 24, generated exactly by A4₀ and the six Cayley parameters (±1,0,0), (0,±1,0), (0,0,±1) and their products.
  - A `State(menu=...)` rejects twists outside the exact menu as `invalid`. A mapped numerical request with nonzero approximation distance (above 10⁻¹² for float input comparisons) is also refused, even if its nearest candidate belongs to the menu. Passing `menu` to the input map refuses it at construction. Menu matrices themselves are never approximated.
- **Journal.** Records hold the grip, exact matrix (Q5 entries as `[a, b, d]`), family parameters and moved count. Every Q5 component must be a JSON integer; floats, booleans and strings are refused, including in menu matrices. Mapped requests also record N, D and the distance. Replay checks that retained, plane, Cayley and half-turn parameters rebuild the recorded matrix exactly; it never repeats the numerical search. The document carries the model identity (SHA-256 of both asset files as read), menu identity (or null in free mode) and contract revision. Replay requires these to match the supplied context and menu. Legacy documents without identity fields and bare record lists still replay. Undo applies the exact inverse after checking the recorded inside count.
- **Checkpoints** (contract section 5). `checkpoint()` requires a lattice configuration and a witness word in the 1,200 generators. Without a supplied word, it derives one from the journal by exact identities only:
  - consecutive twists of one grip merge, (c, g₁)(c, g₂) = (c, g₂g₁), since g₁ keeps every piece on its side of the cut;
  - identity twists drop.

  The word is always replayed from solved and compared pose by pose. `export_retained()` returns the labelled slot permutation (`labels[slot] = home slot`, the `core.Model.word_net` convention), and only at a checkpoint.
  `State.from_global_rotation(ctx, kidx)` builds a configuration with every piece in one K⁺ pose. Non-identity global rotations move centres to other chambers and cannot be retained checkpoints; a supplied word cannot bypass that exact obstruction. This constructor asserts no path from solved.
- **Labelled stickers and frames** (A3). A sticker is (piece, home host facet), labelled by its home slot. On a lattice configuration with pose k it sits in slot(k(A), k(f)). Its frame k·F_f equals F_{k(f)}·L with L ∈ A4₀, and the slot layout puts it at base-region index L(j). `lattice_stickers()` returns slots, labels, orientation indices and the frame-agreement flag, and refuses a non-bijective map or a frame outside A4₀. All 600 centre stickers are included: retained turns change the centre's pose while its label stays in place. Off the lattice, `sticker_frame(s)` is the exact pose times the cell frame.

## A1 input-search proof

Treat each component of the requested float quaternion t = (c,v) as the exact rational `Fraction(float)`. Its norm need not be exactly one after float normalisation. For a bounded integer numerator triple p and denominator d, the Cayley quaternion is (d,p)/√(d²+C), where B = v·p and C = p·p. The squared Frobenius distance to the normalised target is

```text
8 (1 − (c d + B)² / ((d² + C) |t|²)).
```

The input map enumerates every p ∈ [−N,N]³. For each p it maximises f(d) = (c d + B)²/(d²+C) on the integers 1…D. Its derivative is

```text
f'(d) = 2 (c d + B) (c C − B d) / (d² + C)².
```

The zero c d + B = 0 is a minimum of f, so cannot improve the rotation distance. The only possible interior maximum is d = c C/B. Consequently the endpoints and the floor/ceiling of that stationary point, clipped to 1…D, contain a minimum for every p. B = 0 and p = 0 give monotone or constant cases covered by the endpoints. The floor and ceiling are calculated with integer arithmetic using a common denominator for the rational target components. Thus float error cannot omit a denominator candidate. This eliminates denominators without eliminating any possible improvement; it does not round the desired Cayley parameter independently in each coordinate.

Float distances screen the resulting candidates with margin **10⁻¹²**, which is not a tie tolerance. For N,D ≤ 2⁵³, all candidate integers convert exactly, and their nonzero four-vector norms are in [1,2⁵⁴], so their squares and sums stay in the normal binary64 range. With u = 2⁻⁵³, normalising either four-vector contributes at most 12u in Euclidean norm (for the float target this bounds its deviation from its exactly normalised rational vector). Adding or subtracting them contributes at most 2u, and evaluating either resulting norm contributes at most 12u in absolute error. Each norm in √2‖q−t‖‖q+t‖ therefore has error at most 38u and magnitude at most 2 plus those errors. Product and √2 rounding leave total absolute distance error below **E = 256u + 2⁻⁵⁰⁰**. The absolute term conservatively covers underflow when a squared difference is tiny.

The float distance of any exact winner is at most 2E above the smallest computed distance: both the winner's distance and the computed minimum can err by E. Since 2E < 5.7·10⁻¹⁴ < 10⁻¹², no exact winner can fall outside the finalist margin. Outside N,D ≤ 2⁵³, screening is disabled and every pruned candidate is compared exactly. Finalists maximise the rational squared dot product above; only equality of those rational scores invokes (denominator, numerator triple) ordering. Constant cases include denominator 1, and adjacent denominators of an interior maximum are both present, so exact denominator ties are retained. An identical key across the two families uses Cayley first for determinism. Display distances are converted from the winner's exact squared distance.

For half-turns the quaternion is (0,p)/√C, so the exact score uses d = 0. Scaling an axis does not change its rotation, so every p/d has the same candidate at denominator 1; all nonzero bounded p are enumerated. These candidates are included only when the requested rotation angle is within 1° of π, modulo a full turn. The stable float distance √2‖q−q_target‖‖q+q_target‖ avoids cancellation near identical rotations. Independent small-box tests enumerate **every** denominator, compare 3×3 Frobenius distances with the selected minimum, and check tie order using rational quaternion scores. Requests just above and below 45° distinguish unequal near-ties.

This numerical input map chooses a nearby exact rotation. It does not claim that an arbitrary float matrix is field-valued. An exact request without a Q(√5) representation, such as a seventh turn, is represented by `UnrepresentableTwist` and rejected without changing state. Exact menu members are supplied as exact twists; a menu request is never rounded to another member.

## A2 recorded certificates

`State(ctx, filtered=True).classify(grip, record=True)` returns a `Classification` with `decisions`, one record per evaluated vertex. Each records the exact pose key, piece or anchor, vertex set and index, integer vertex and pulled-back normal forms, cut offset, state digest and model hashes, arithmetic/error-bound versions, an outward-rounded enclosure and the accepted sign. The enclosure measures ‖n‖² h, which has exactly the sign of h. A float decision is accepted only when the finite enclosure excludes zero.

Coverage is either the piece's complete exact vertex set (with its representative and transport identity), or the complete cap-superset vertex set, with the covered piece identities and the constraint subset: all outer facets and the anchor's inside cut. Every covered piece's signature contains that anchor. Uncertain or non-finite evaluations fall back to exact Q(√5) signs; those records say `exact_fallback`, include the accepted exact sign and the fallback reason, and use null for an unavailable float enclosure. Recording is opt-in and does not change legality.

## Acceptance results (`acceptance.json`)

The JSON carries the measured counts and pass flags for the current run. The original 31 flags remain, with a further flag for the J1 review fixes:

| Flag | Coverage |
| --- | --- |
| `1_projection_bijection_and_centres` | Solved and all 1,200 generators: 259,800 stickers form a bijection, orientations lie in A4₀ and labels equal the retained primitives. All 600 centre stickers are checked in each configuration (720,600 checks); each generator changes its own centre's pose with labels fixed |
| `5_grouped_equals_per_piece` | E2 and E3 over all 600 grips, plus two seeded mixed states over 60 grips each. The reference evaluates every off-lattice piece individually; statuses and every admissible inside set must match |
| `7_global_rotation_control` | Two global K⁺ rotations, one nonidentity A4₀ and one moving pole 0: lattice, distinct from solved, centres moved, checkpoint and export refused with and without a supplied word |
| `7_same_cap_excursion` | (c,g), (c,a·g⁻¹) goes off-lattice, then returns to the same nonidentity retained pose as a. Its journal witnesses a checkpoint and its export equals the primitive replay |
| `8_menu_controls` | A4 (12), S4 (24) and NC (16): exact inverse closure, A4 conjugacy invariance, containment and record round trips. Transport by F_c and F_c·a agrees for all 600 caps and all 12 a ∈ A4₀; production transport also agrees with independent integer conjugation |
| `9_negative_control` | g = 2P₀,₁₃ − I constructed as a half-turn; g ∉ A4₀, g² = I, tr(H₀g) = 4/3. Twelve rounds of alternating H₀ and g are admissible with 25 distinct digests including solved, growing cap-0 pose heights and exact replay |
| `A1_input_map_and_half_turns` | Independent exhaustive small-box minimum and tie-order checks, half-turn record reconstruction, recorded N/D/distance, explicit nonrepresentable rejection and refusal of approximated menu requests |
| `A2_certificate_fields` | Every recorded float decision on E2 grip 13 and E3 grip 0 has the required exact inputs, identities, coverage and versions; each finite enclosure excludes zero and agrees with the exact sign. Fallbacks are also recorded and checked |
| `identity_in_journal` | Asset byte hashes, menu hash and contract revision recorded; mismatches refused; matching and legacy journals replay |
| `review_fixes_F1_to_F4` | Mismatched K⁺ indices refused without change; 30 plane twists classified/surveyed with checked certificates and filtering off/on; huge Cayley parameter journal/replay/undo; exact near-ties and tie order in A1; non-integer exact JSON components refused in every reader |

The retained flags still cover the exact K⁺/region/frame build, all generator actions and inverses, E0–E4, fresh independent double descriptions and literal certificate checks, conservative rejection, 20 seeded mixed-sequence replay/inverse/undo trials, supplied checkpoint witnesses, and the unfiltered/filtered sign comparison. The `A1_cayley_twists` flag now requires half-turn acceptance, as amended by A1.

## Exact versus floating point

- **Exact (Q(√5) and integers)**:
  - poles, cut offsets and region vertices;
  - K⁺ and all twists, which are verified exactly orthogonal, of determinant 1, and fixing the grip pole;
  - every sign that decides inside, outside or straddling;
  - straddle certificates, poses and the journal;
  - checkpoint replays and the sticker, label and frame maps.
- **Floating point**:
  - the A2 filter when switched on (proven bound; undecided entries go exact);
  - the float cross-check in `accept.py`;
  - the axis/angle search, which only proposes a rational parameter whose exact rotation is then built and checked;
  - displayed angles and h values.

Display conversion keeps coefficient/denominator ratios as `Fraction` until the final binary64 conversion. Opposite-sign Q5 coefficients use (a²−5b²)/(d(a−b√5)) to avoid cancellation. Large coefficients alone never make display conversion fail; finite underflow returns zero, and a non-finite final value returns null. This helper is separate from the A2 filter conversion and never determines an exact sign or state change.

## Limits and open points

- **Signature certificate.** Lattice pieces are certified by their defining constraint, as in `witness.py`, rather than by a vertex bound. This is exact. The acceptance also compares it with vertex classification (0 disagreements). A reviewer should confirm that it meets section 3's wording.
- **Survey certificates.** Surveys report certificate counts and exact straddle points. Detailed A2 vertex decisions are available through the opt-in `classify(..., record=True)` path.
- **Undo and menus.** Undo is a journal operation and is not restricted by a twist menu.
- **No witness search.** Witness words come only from exact journal identities or from the caller. Nothing searches for a word, consistent with the human-solve boundary.
- **Excursion audit.** The audit of contract section 5, which asks whether off-lattice excursions create new lattice actions, is not part of J1 and is not implemented.
- **Non-exact rotations.** These are rejected, not handled by interval bounds; the contract allows either. All exact half-turn axes and Cayley parameters may use Q(√5), while the bounded numerical input map searches the rational subset specified by A1.
- **Timings.** Runtimes are cloud timings for this research code, not performance evidence for any product path.
