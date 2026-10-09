# J1 general simulator (exact reference engine for rigid jumbling)

Status: **candidate for review (9 October 2026).** It implements workstream J1 of [`docs/progress/1.0/jumbling-plan.md`](../../../docs/progress/1.0/jumbling-plan.md) against the plan-checked [state contract](../state-contract.md) (sections 2, 3, 5 and 6), with amendments A1 and A3, A2 behind a flag that is off by default, and A4 as a per-cap twist menu. Nothing here changes 600-cell-Full, `assets/` or any root module.

Evidence kind: source and synthetic geometry from a headless cloud session. Nothing here is Windows, Direct3D, input or performance evidence.

## Run

From the repository root, with Python 3 and NumPy only:

```text
python tests/test_jumbling_sim.py        # 22 unit tests, about 13 s
python research/jumbling/sim/accept.py   # full acceptance, writes acceptance.json
```

The acceptance run took 218 s and 293 s on four cloud cores. The second run shared the machine with another four-worker job (load average about 9). Building the shared context (exact poles, K⁺, regions, tables) takes about 4 s.

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
| `model.py` | Read-only model data: exact poles (via `witness.py`, which uses `exact.from_float`), signatures, hosts, cap membership, signature bitsets, the (piece, facet) to slot map |
| `kplus.py` | K⁺ (order 7200) as pole permutations, with exact matrices on demand; composition, inverses, A4_c, generator words, exact membership test |
| `regions.py` | K⁺-orbits of pieces; exact regions of the 36 representatives (double description from `witness.build_region`); lazy, cached exact transport to any piece |
| `kernel.py` | Exact integer sign kernel over Q(√5); the optional filtered sign test (A2) with its error bound |
| `twists.py` | Twists: retained generators and A4 elements, plane rotations (witness construction), Cayley rotations (A1), axis/angle mapping, `TwistMenu` (A4) |
| `state.py` | Configurations: poses, admissibility, apply, survey, journal, undo, replay, snapshots, checkpoints, labelled stickers and frames (A3) |
| `accept.py` | The acceptance run; `acceptance.json` is its output |

## Model and rules

- **State** (contract section 2). Each piece has an exact pose. Poses are interned in a pose table and each piece stores a pose id (`int32`). Id 0 is the identity, so no matrix is stored for a piece in its home pose. A pose in K⁺ is stored by its K⁺ index, any other pose by its exact 4 × 4 matrix over Q(√5). The lattice flag of a piece is "pose in K⁺". Equal configurations with equal journals serialise to equal bytes, because the pose table is kept canonical (unused poses dropped, entries ordered by exact key).
- **Regions.** The 177,120 pieces fall into 36 K⁺-orbits. Each orbit is exactly one retained orbit class (`orbit_id`); the 600 fixed centres form one K⁺-orbit. So K⁺-orbits are unions of G-orbits. One representative per orbit, in cap 0, gets an exact double description (5 to 24 vertices). Every other piece's region is the representative's region moved by the K⁺ element that maps signature to signature. Transported vertices are built only when needed and cached (LRU), so the 177,120 regions never exist at once.
- **Admissibility** (contract section 3), for a twist (c, g):
  - A lattice piece is classified by its transported signature. Its region is the retained chamber with that signature, which the constraint n_c · x ≥ κ (or ≤ κ) itself cuts out. This is the **signature certificate**.
  - Off-lattice pieces with one pose form a pose group. The group is split by anchor caps a, where a lies in the signature of every piece of the subgroup. The posed cap region g(Ū_a ∩ P̃) contains every posed region of the subgroup. An exact one-sided bound over its 40 exact vertices certifies the whole subgroup (**group superset**).
  - Otherwise each piece is bounded over the complete exact vertex set of its posed region (**piece vertices**).
  - A straddling piece gives two exact vertices, one with h < 0 and one with h > 0. Each vertex is re-checked against all 600 cut and 600 facet constraints of the posed region before the certificate is issued.
  - A piece without a certificate makes the twist uncertain. That happens only for a rotation with no exact representation (`UnrepresentableTwist`) or an exhausted exact work budget. Blocked and uncertain twists change nothing. A commit is all-or-nothing.
- **Twists.**
  - Retained twists are the 1,200 generators and every A4_c element, with generator words.
  - Plane rotations use the witness construction for any s ∈ Q(√5).
  - Cayley rotations (A1) use R = I + 2(W + W²)/(1 + |ω|²) in the exact frame u₁ = (−n₁, n₀, −n₃, n₂), u₂ = (−n₂, n₃, n₀, −n₁), u₃ = (−n₃, −n₂, n₁, n₀). As quaternions these are i·n, j·n and k·n.
  - `cayley_axis_angle` maps a requested axis and angle to the nearest parameter with common denominator ≤ `max_den` and reports the realised angle, the axis error and the rotation error. Half-turns have no Cayley parameter and are refused.
  - A plane rotation with parameter s about pole d is exactly the Cayley rotation with ω = −s(uᵢ · n_d). The tests check this.
  - `TwistMenu` (A4) transports a base-cap menu by the retained frames, Λ_c = F_c Λ₀ F_c⁻¹. With `close=True` it closes Λ₀ under A4₀ conjugation, which makes it K⁺-invariant. A `State(menu=...)` rejects twists outside the menu as `invalid`. The A4 menu gives 600-cell-Full.
- **Journal.** Records hold the grip, the exact matrix (Q5 entries as `[a, b, d]`), the family parameters and the moved count. Replay rebuilds twists from records; for retained, plane and Cayley records the parameters must reproduce the matrix exactly. Undo applies the exact inverse, after checking that it is admissible with the recorded inside set.
- **Checkpoints** (contract section 5). `checkpoint()` requires a lattice configuration and a witness word in the 1,200 generators. Without a supplied word, it derives one from the journal by exact identities only:
  - consecutive twists of one grip merge, (c, g₁)(c, g₂) = (c, g₂g₁), since g₁ keeps every piece on its side of the cut;
  - identity twists drop.

  The word is always replayed from solved and compared pose by pose. `export_retained()` returns the labelled slot permutation (`labels[slot] = home slot`, the `core.Model.word_net` convention), and only at a checkpoint.
- **Labelled stickers and frames** (A3). A sticker is (piece, host facet), labelled by its home slot. On a lattice configuration with pose k it sits in slot(k(A), k(f)). Its frame k·F_f equals F_{k(f)}·L with L ∈ A4₀, and the slot layout puts it at base-region index L(j). `lattice_stickers()` returns slots, labels, orientation indices and the frame-agreement flag. Off the lattice, `sticker_frame(s)` is the exact pose times the cell frame.

## Acceptance results (`acceptance.json`)

| Item | Result |
| --- | --- |
| Build | K⁺: 7200 exact rotations, all pole images exact. 36 K⁺-orbits. Representative regions: 36/36 full-dimensional, host patches equal the retained hosts, no vertex violates any of the 1,200 constraints, every vertex has a rank-4 tight set, and every representative touches every cut of its signature exactly. Transported signatures and hosts match for all 177,120 pieces. Frames are exact K⁺ elements, with a largest float deviation of 1.8·10⁻¹⁵, and the slot layout is frame-covariant for 259,800/259,800 slots |
| 1. Stickers and frames, all 1,200 generators | Combinatorial proof, with no failures. For each generator k: the exact matrix maps the poles as `rotperms[k]`; r_k = F_c r_t F_c⁻¹; the inside set equals `move_src[k]`; perm_k(signature(src)) = signature(dst) for all 3,097 moves; every sticker move equals `primitives.npz`; and the destination region index equals L(source index). Through the `State` API, 1200/1200 generators are admissible, move 3,097 pieces, match the primitive labels and frames, pass the checkpoint, undo to identical bytes, and their inverses match. 6600/6600 non-identity A4 twists are admissible from solved, and their generator words reproduce their matrices |
| 2. Certified blocked and unblocked cases | Witness reproduced exactly: g = witness rotation (s = 15/961). E2 has 54 blocked grips and E3 has 65, both sets equal to `witness-results.json`. The negative control is blocked with the witness certificate (piece 0, h = −0.01487 / +0.01344), with bytes unchanged. The alternative unblock (d, g·T_d⁻¹) frees c. E4 returns to solved bytes |
| 2. Independent checks | 54/54 and 65/65 blocked certificates verified literally from their JSON in plain Q5. All 600 statuses of the E2 and E3 states recomputed from fresh double descriptions of every off-lattice piece (no K⁺ transport, no integer kernel): 600/600 each. Float cross-check with a 10⁻⁹ band: 600/600 each. Signature against geometry on lattice states: 541,392 pieces, 0 disagreements |
| 3. Uncertain contacts | The E0 control simplices are classified correctly, exact and filtered (shallow crossings straddle; contact is inside or outside). A real 6.4·10⁻⁷° jumble twist blocks 54 of the first 120 grips with certificates, the smallest certified \|h\| being 1.5·10⁻¹⁷. An unrepresentable float rotation is rejected as uncertain. An exhausted budget is uncertain, and the same twist without a budget is admissible. Invalid matrices and tampered records are refused |
| 4. Rejection without change | Byte-identical snapshots after every rejection: the controls, the witness negative control, the menu rejections and 385 blocked attempts in the round trips |
| 5. Round trips | 20 seeded sequences (seed 20261009) of 8 to 13 admissible twists: 84 retained, 59 plane and 65 Cayley. 181 of the 208 twists needed geometric certificates, and up to 21,324 pieces were off the lattice. All 20 pass replay from JSON (equal bytes), the explicit inverse sequence (exact identity) and undo (solved bytes) |
| Checkpoints | A retained word of 40 moves: export equals the `primitives.npz` replay. A jumble excursion that merges into a retained twist is a checkpoint, and its export equals the replay of the derived word. A lattice state through non-cancelling jumble twists is not a checkpoint until a witness is supplied, a wrong witness is refused, and export is refused. Off-lattice states are refused |
| A2 filter | Off by default. Its surveys equal the exact surveys. 2,115,832 vertex signs were decided by the filter and 1,148 left to exact, with 0 disagreements |
| A1, A4, A3 | Plane equals Cayley in 20/20 cases. The A4 menu is K⁺-invariant and equals A4_c for 600/600 caps. A Cayley menu closes to 6 elements and recognises its transported members. Off-lattice sticker frames equal pose × cell frame |

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

## Limits and open points

- **Signature certificate.** Lattice pieces are certified by their defining constraint, as in `witness.py`, rather than by a vertex bound. This is exact. The acceptance also compares it with vertex classification (0 disagreements). A reviewer should confirm that it meets section 3's wording.
- **Survey certificates.** For admissible grips, the survey records the number of pieces certified by each kind, not the vertex lists. These are reproducible from the state. Blocked grips carry exact points.
- **Undo and menus.** Undo is a journal operation and is not restricted by a twist menu.
- **No witness search.** Witness words come only from exact journal identities or from the caller. Nothing searches for a word, consistent with the human-solve boundary.
- **Excursion audit.** The audit of contract section 5, which asks whether off-lattice excursions create new lattice actions, is not part of J1 and is not implemented.
- **Non-exact rotations.** These are rejected, not handled by interval bounds; the contract allows either. There is no constructor for half-turns outside A4, but `Twist` accepts any exact rotation matrix.
- **Timings.** Runtimes are cloud timings for this research code, not performance evidence for any product path.
