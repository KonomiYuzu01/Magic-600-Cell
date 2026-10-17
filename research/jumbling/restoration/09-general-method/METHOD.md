# A state-only general method for menu I_a, 15 October 2026

Research package for the jumbling puzzle on the retained 600-cell-Full geometry, menu I_a (177,120 pieces, 259,800
sticker slots, K+ of order 7,200). Every computation is a headless Linux run on exact synthetic geometry. Nothing
here is Windows, DirectX, input, long-session or performance evidence.

Labels used for every claim:
- **proved**: a written proof (author's proof; no independent mathematical review unless the line says otherwise);
- **certificate**: an exact computation with a replayable record that an independent checker in this package or an
  earlier package accepts;
- **measured**: a result on a finite set of inputs;
- **open**: no result.

## 1. What counts as restored (owner decision, 15 October 2026)

1. **Sticker definition.** A configuration is restored when all 259,800 sticker slots carry their home labels. A
   rotation of a piece that maps the piece and each of its stickers onto themselves does not matter.
2. **Length.** A macro may be very long, as long as a person can replicate it; the program may assist the person.
3. **Priority.** Most important is that the machine has a general method.

The note is the wiki page [owner-decisions-2026-10-15-jumbling-restored](../../../../docs/wiki/decisions/owner-decisions-2026-10-15-jumbling-restored.md). It sets the research definition. The product's
human-solve boundary (owner decision, 2 October 2026) is not changed by it.

Consequence used below: while a piece is off the lattice, the lattice has a hole (the number of holes equals the
number of off pieces), so the configuration is not restored under the sticker definition either. The off-lattice
phase is therefore unchanged by the new standard. The standard removes only the exact pose at the end, the length
limits and reading the state by eye.

## 2. The method

Input: one pose table (the current state only). Output: a chain of steps, each a twist word with its input and
output state digests, ending at the solved configuration; or a stop with its reason.

**Completeness: Theorem M\* (proved; Codex senior review on 15 October 2026, no human review;
`SENIOR-REVIEW.md` section 9).** Add to phase 1 a fair fallback: when the steps below find nothing, enumerate all
menu words in order of length and take the first clean step. The method then restores every reachable
configuration from the current state alone, in at most as many clean steps as there are off pieces at the start,
followed by the lattice phase. It needs no history and no finiteness of the reachable set, and it gives no bound
on the length of a fallback step. The driver `gdrive.py` implements M\* (step 4 below; `mstar.py` and
`fallback.py`). The runs of section 6 used the earlier driver v4, a partial implementation: it had no fallback, it
stopped after `--max-steps`, and it skipped the lattice phase when the last permitted step reached the lattice.

### Phase 1: off-lattice phase (clean steps)

A *unit* is a rigid block of off pieces (no cut separates them), or one frame part of a multi-frame block
(`mkblocks2.py`). A *clean step* for a unit T at state Z is a word V W V^-1 such that every lattice piece of Z ends
on the lattice, every piece of T ends on the lattice, and every off piece outside T and the step's free set F keeps
its exact pose (collateral 0).

- **Lemma U (proved).** A clean step lowers the off count by at least |T|. So a run of clean steps ends after at most
  as many steps as there are off pieces at the start. Termination is not a gap.

At each state the driver `gdrive.py` does the following, reading only the pose table:

1. **Units.** `mkblocks2.py` lists the units.
2. **Driver step.** `cbrun3.py --max-steps 1`: candidate words W for every unit (the word library, the single
   letters at the block's free grips, a fresh D-type/C-type search `dfam2.py` for units without a usable word, one
   commutator refinement round `comm3.py`), scored at Z (`score4.py`: T lands on the lattice; Bad(W); the vacancy
   conditions N1 and N2), then closed with `tclose2.py`: V from the peeling certificate (Theorem B), a greedy
   search if the certificate fails, replay of V, W, V^-1 on the simulator with every twist checked admissible. The
   first candidate whose closure is a clean step is taken.
3. **Pure-twist step** (only when the driver finds no step at Z). For the largest units, at most 8 of them
   (`--pure-max`; the units left out are logged as `pure_dropped`), largest first, `cellpure3.py`
   builds the class-cell model of the unit's class (the 120 caps of one of the five classes of grips; a cell is a
   sign vector over those caps) and searches the pure twist: commutator powers [x, y]^k of free class-P letters
   around a cycle of three or four edge cells through the carrier's cell X, exponents solved over Z5 so that the
   carrier's cell alone turns to label 0 (model check on every content, then exact replay). Its W lands the carrier
   and takes the lattice pieces of X off (Bad(W) = X); the cheaper powers it uses may also permute lattice pieces
   outside X, which a clean step allows (`SENIOR-REVIEW.md`, LC-02 and LC-03). Theorem C there proves that a word of
   at most 720 letters exists whenever the two poles of X and one common neighbour are free; `cellpure3.py` searches
   for one and does not use that bound. The stage covers carriers in edge cells only; the same review proved
   words for the other cell kinds (in place for vertex cells, transport for triangle and tetrahedron cells;
   section 3), which the driver does not use yet (the fallback covers those units). The units with a word are then
   closed, the unit with the fewest lattice pieces in X first, by `tclose2.py`; on 'no V', by `tclose2m3.py` (the
   same closure with a targeted depth-two peel for V). The first clean step in that order is taken.
4. **Fallback.** If neither kind of step restores a unit, the driver records, per unit, why the pure twist failed
   (precondition of the cell model, no cycle of edge cells, model check, ghost premises, no V) and the driver's
   best candidates. Then `fallback.py` takes the step: for the largest unit U it searches the menu words in order
   of length and takes the first clean step, with free set off(Z) \ U. It leaves out identity letters and two
   consecutive letters at one cap (the menu is a group, so such a pair acts as one letter), which keeps the search
   complete. At a reachable state it always finds a step (`SENIOR-REVIEW.md`, M1-01), with no bound on its
   length. Length 1 has at most 35,400 letters (600 caps, 59 items). At the state of `test_fallback.py` a letter
   took about 0.02 s (measured: 826 letters at 20 caps in 18 s), so length 1 takes minutes and length 2, about
   1.25 x 10^9 words, about 300 days of one process. The fallback is the completeness device of M\*, not a
   practical search.

The outer loop (`mstar.py`) checks the off count before any step budget, so a run whose last permitted step
reaches the lattice still goes on to phase 2. Each heuristic subprocess is killed after `--heuristic-seconds`
(default 21,600), and a heuristic that fails counts as a miss, so the fallback is reached whenever the heuristic
steps find nothing. `--max-steps` and `--fallback-max-length` are experiment bounds outside M\*, off by default.

Every step is written as a chain step (word with digests, unit, free set). `verify_u.py` replays the chain from the
start table with the simulator only: every twist admitted, both digests, unit restored, collateral 0, every moved
off piece inside the unit or the free set, off count falling.

### Phase 2: lattice phase

When no piece is off the lattice, `restore_lattice_late.py` computes the invariant nu (F2^43 + F5^2 coordinates)
from the configuration, chooses the correction combination C from the library of 36 macros (rank 34 over F2 and 2
over F5, which spans A_J), solves the labelled configuration with the retained-group cascade R, and applies C last.
`verify_late.py --chain` rebuilds the start of this phase from the chain's start table through every step word
(each twist an I_a menu element and admissible, every digest checked), evaluates R with its own code, builds the
target T = solved . C^-1 in the simulator, checks that T has the labels R produces, then applies C and requires the
solved configuration (identity labels on all 259,800 stickers).

- Completeness: **Theorem R** (proved; Codex senior review on 15 October 2026, no human review): when the library
  spans A_J, the restorer solves every *reachable* configuration with all pieces on the lattice, in the sense of
  stickers. It does not hold for every lattice configuration: one with every piece rotated by the same order-three
  element of K+ has nu = 0 but displaced centres. With C applied last, choose C with nu(C) = -nu(Y), put
  T = solved . C^-1 and solve Y relative to T, whose nu is zero; then Y . R . C = solved.
- The library spans A_J: its rank (34 over F2, 2 over F5) is a **certificate**, and the two ranks suffice. In
  C2^34 x C5^2, multiplying by 5 keeps the binary part and multiplying by 6 keeps the C5 part, so a subgroup is the
  product of its two projections (Chinese remainder theorem). A_J = C2^34 x C5^2: the lower bound is a certificate
  (40 words), and the upper bound is proved (Theorems A and B5; the same review found them sound, with an inverse
  typo and a frame convention to state, both fixed).
- Lengths: on the runs so far R took 1.5 to 2.3 x 10^8 twists and C about 10^4. For every input, the committed
  cascade and library give at most 1,324,686,034 twists (proved, same review, `SENIOR-REVIEW.md` section 9).
  Round 6 (`SENIOR-REVIEW.md` section 11; proved, model review) sharpens this:
  - the frame passes of the 24 orbits with trivial H_o do nothing, so the committed cascade needs at most
    771,844,512 twists (M2-04);
  - a shorter frame recipe (M2-03) and the exact count of the position loop give a construction of at most
    356,565,166 twists for every input, and at most 231,334,003 on average over G (M2-04). It is not implemented;
  - some input needs at least 224,601 retained twists (M2-02).

  Round 7 (`SENIOR-REVIEW.md` section 12) replaces 31 of the 35 seeds by shorter elements relocated onto the
  buffers. The cascade needs no fibre alignment: a seed must move exactly the stickers of the buffers on its own
  orbit, as the position cycle, with collateral on orbits solved later; on D5 and A5 the pairs are re-indexed
  (proved, model review).
  - The cascade with the replaced seeds restores the two synthetic inputs in 45,009,802 and 44,940,982 twists
    instead of 229,763,734 and 230,484,810 (computed; checked by the stage 03 checker, which evaluates each solution
    with its own code).
  - For every input it then needs at most 71,516,526 twists as it runs (expected length at most 46,191,246 for a
    uniform input on G); the construction with the shorter single of M2-03 needs at most 71,514,544. The corrections
    add at most 25,852. This cascade runs in `restore_realigned.py`, a runner separate from the driver: the lattice
    phase of `gdrive.py` (`restore_lattice_late.py`) still runs the committed cascade, at most 356,666,844 twists by
    the exact count of R7-03.
  - Round 10 (`SENIOR-REVIEW.md` section 15) performs each three-cycle of positions with one application of the
    seed, conjugated by a word of at most e_o + d_o letters, instead of up to two. This construction needs at most
    50,560,184 twists for every input (expected length at most 32,201,480), or 4,675,165 actions (proved, model
    review). It is not implemented.
  - Round 11 (`SENIOR-REVIEW.md` section 16) also sets the frames inside the position loop with twisted cycles, uses
    the fewest three-cycles on the orbits without frames, and clears the remaining frames two at a time. A
    construction then needs at most 20,866,844 twists for every input (expected length at most 17,010,356), or
    4,655,642 actions (proved, model review). That is about 1.334 times the floor of T11-06 for cascades that solve
    one orbit at a time with conjugates of these seeds; the comparison does not cover a construction that uses
    collateral on purpose, or free cancellation (round 12, R12-01). It is implemented as a runner separate from
    the driver and restores the two synthetic inputs in 15,761,436 and 15,760,112 twists (measured; checked by the
    stage 03 checker; section 6.4).

  These are still machine-scale (section 5, gap M2).


## 3. What the method rests on, and the status of each part

| Part | Statement (short) | Status |
| --- | --- | --- |
| Basic properties | Legal twists keep the state valid and are invertible; a lattice configuration has exactly one piece per chamber; the centre pieces never leave their poles. | proved (Codex senior review) |
| Lemma U | A clean step lowers the off count by at least the unit's size; so every run of clean steps terminates. | proved (Codex senior review, 15 October 2026) |
| Closure V W V^-1 | Every step of every chain here is a clean step. | certificate per step (`verify_u.py` replay) |
| Theorem B, Proposition N, Theorem A | Vacancy for Bad(W): necessary counts N1/N2; a peeling certificate gives V, its thresholds and a length bound. One certificate must cover all targets at once: separate certificates per orbit do not prove it. | proved (Codex math review; the simultaneity condition from the Codex senior review, `SENIOR-REVIEW.md` section 9) |
| Lemma D | Per-type count: \|Bad_t(W)\| is at most the number of holes of type t; off counts by type only fall along a run. | proved (Codex senior review, 15 October 2026; the remark after Corollary D'' needs f_t = 0, fixed) |
| Proposition GH, Lemma CM | Glued holes move with their carrier; a carrier moved by free K+ letters keeps the step clean, with the same free set. | proved (Codex senior review, 15 October 2026: GH as written; the statement of CM corrected to keep the same free set, which the constructions use) |
| Lemma C (class-cell pure twist) | If the two poles of the carrier's edge cell X and one common neighbour are free, a word of at most 720 letters at those three grips twists X alone: every piece outside X keeps its exact pose (corrected in round 8, R8-04). Closed by a peeling certificate V that vacates X, V W V^-1 is a clean step of at most 2 L_cert + 720 letters. | proved (Codex senior review, 15 October 2026; no human review): an exact certificate on one triangle of edge cells, transported by symmetry, re-run by the integrator. The code's cheaper powers may also permute lattice pieces outside X, which a clean step allows; the claim that they return every other content to its point was false. Applied on 4 states, each word replayed exactly; at s86a Bad(W) equals X exactly (checked). `SENIOR-REVIEW.md` sections 4 and 5 |
| Other cell kinds | A real carrier in a vertex cell lands in place (at most 104 letters), and every piece outside that cell keeps its exact pose. One in a triangle or tetrahedron cell cannot, but a pure transport word lands it in another cell of the same kind (at most 240 or 192 letters). A transport word moves several whole cells of that kind (each seed cycles three) and fixes every other cell, so its closure needs the other chambers of those cells vacant (corrected in round 8, R8-04). A rigid block of one frame lies in one cell. | proved (Codex senior review, 15 October 2026; no human review), with exact certificates re-run by the integrator; assumes the class's grips are free. Not used by the driver's heuristic steps, whose pure twist covers edge cells only; the fallback covers those units. `SENIOR-REVIEW.md` section 6 |
| Lemma TT (stage 06) | A K+-transported word moves a congruent block exactly as the original word moves its block. | proved (stage 06); whether the transported step is clean depends on the target state: measured on 2 carriers |
| Theorem S at stop62@245 | Every block-family word has \|Bad\| >= 333 > 245 holes, so no family closure works there. | certificate |
| G = K | The retained group is the whole kernel of the invariants (35 position and 11 frame certificates). | certificate (independent checker) |
| A_J = C2^34 x C5^2 | The lattice-phase invariant group. | lower bound certificate (40 words); upper bound proved (Codex senior review, 15 October 2026) |
| Theorem R | With a library that spans A_J, the lattice restorer solves every reachable lattice configuration (stickers). | proved (Codex senior review, 15 October 2026; restricted to reachable configurations) |
| Termination of the whole method | Phase 1 by Lemma U, phase 2 by construction. | proved |
| Existence of a clean step | At every reachable state with an off piece, every unit has a clean step, and an exhaustive search over words in order of length finds one from the state alone and stops. | proved (Codex senior review, 15 October 2026; no human review); no bound on length or run time |
| Small states need excursions | With 0 < h < 3,097 off pieces, every clean step has at least two jumble letters, and its first jumble letter takes at least 3,097 - h lattice pieces off. | proved (same review) |
| Theorem M\* (a general method) | Heuristic steps, a fair exhaustive search when they find nothing, then the lattice phase with the corrections last: restores every reachable configuration from the current state alone, in at most \|off\| clean steps. | proved (Codex senior review, 15 October 2026; no human review), `SENIOR-REVIEW.md` section 9; no bound on the length of a fallback step |
| Toolkit theorem | For a unit whose frame belongs to class P, a class-P landing word W that passes finite checks at the state (free grips, exact landing action, every other off piece lands or keeps its exact pose, Bad(W) the union of its cells, vacancy for all targets at once) closes to a clean step of at most 2(Q_safe - 1) + \|W\| letters. When all of the class's grips are free, W can be chosen with \|W\| <= b, b = 720, 104, 240 or 192 by cell kind; with some grips blocked, b bounds no W and the length of W must be checked (corrected in round 8, R8-05). | proved (same review), given the certificates of `SENIOR-REVIEW.md` sections 5 and 6. Units with pieces of height four lie outside it; 150 twists of the I_a fixture make 1,411 such pieces (re-run) |
| Lattice-phase length | The lattice phase is the retained cascade R, then the corrections C, which need at most 25,852 twists for every input (AU-05). R needs at most 1,324,660,182 twists with the committed cascade (1,324,686,034 with C), 771,844,512 without its trivial frame passes, and 356,666,844 by the exact count of R7-03. A shorter frame recipe gives a construction of R of at most 356,565,166 twists for every input (expected length at most 231,334,003 for a uniform input on G), or at most 4,840,993 actions with a library of the 35 seeds and their inverses. With the replaced seeds of round 7, R needs at most 71,516,526 twists for every input as the cascade runs (71,542,378 with C), and the construction at most 71,514,544 (expected length at most 46,189,354). With direct three-cycles (round 10), a construction of R needs at most 50,560,184 twists for every input (expected length at most 32,201,480), or 4,675,165 actions. With the schedule of round 11, at most 20,866,844 twists for every input (expected length at most 17,010,356), or 4,655,642 actions. | proved (same review for AU-05; round 6, M2-04, round 7, R7-03, round 10, T10-02, and round 11, T11-05, for the rest; the integer bounds re-run by the integrator). The round 7 bounds use the certified lengths and rest on the replacement row below. The construction of round 11 is implemented, and so is the cascade with the replaced seeds, both as runners separate from the driver, whose lattice phase runs the committed cascade (sections 6.3 and 6.4); the other constructions are not implemented as such |
| Lower bounds for the lattice phase | Some input needs at least 224,601 retained twists (185,441 cap twists), and all but a fraction 10⁻⁶ of the inputs need at least 224,599. With a fixed library of m macros at 7,200 placements each, some input needs at least 145,909, 110,495 or 104,774 actions for m = 1, 35 or 70. A cascade that solves one orbit at a time with conjugates of the seeds, without using their collateral on purpose, needs at least 95,024 seed applications for some input, 15,643,520 twists charged for the seeds; this is not a bound on the length of a freely reduced word. | proved (Codex senior review, 15 October 2026, M2-02 and, for the seed applications, T11-06; no human review; the integer checks re-run by the integrator). Retained words only; the floor of T11-06 counts seed applications in a cascade that solves one orbit at a time (scope stated in round 12, R12-01) |
| Single-overlap lemma | If the supports of X and Y (every piece with a sticker moved) meet in one piece and both move that piece's position, [X, Y] is a pure three-cycle on its orbit. | proved (same review, M2-01) |
| Short pure three-cycles | Pure three-cycles for 33 of the 35 orbits; the shortest per orbit has 28 to 12,416 primitives, 21 of them at most 200. None found for O15 and O17. | computed: exact searches, every word re-checked by `check_cycles.py`. 17 of them replace seeds after relocation (round 7) |
| Replacing a seed (round 7) | The committed cascade works with any seed that moves exactly the stickers of its three buffers on its own orbit, as the position cycle A → B → C → A, and whose collateral lies on orbits solved later. Its fibres need no alignment; on D5 and A5 the pairs are re-indexed, at no extra length. A shorter element is relocated onto the buffers by a word g, at the cost 2\|g\|. 31 of the 35 seeds are replaced: 17 by pure three-cycles and 14 by spine prefixes, among them O15 (11,574 to 106 primitives) and O17 (1,546 to 24). | proved (Codex senior review, 15 October 2026, R7-01 and R7-04; no human review). Every replacement certified on all slots (`realign.py`); the cascade with them restores the two synthetic inputs, checked by the stage 03 checker (`SENIOR-REVIEW.md` section 12) |
| Frame elements | A single at C exists only for values in the derived group H_o′. From a seed of length ℓ, aligned or not (round 7, R7-01), a pair costs at most 2ℓ + 2h_x at B and a single at most 4ℓ + 2(h_x + h_y + h_xy). | proved (same review, M2-03; the constants re-run by the integrator) |
| Completeness of the driver | `gdrive.py` finds a clean step at every reachable state with an off piece, and goes on to the lattice phase. | **proved** for the method (Theorem M\*); the driver implements it: its fallback is the exhaustive search of M\*, with no step limit by default. The code had one Codex review at the fast tier (two major findings, both fixed; the verification passed) and passes `test_mstar.py` (16 tests) and `test_fallback.py` (two tests of the fallback path from one jumble twist: each reaches the lattice and passes `verify_u`, with the heuristic steps skipped or timed out; they do not run the lattice phase, R12-10). No bound on the length or run time of a fallback step. The runs of section 6 used v4, which had no fallback, stopped after 80 steps and could skip the lattice phase |

## 4. What the new standard removes from the earlier programme

1. **Exact pose at the end.** Rotations of a piece that fix all its stickers no longer count (for example the
   9,119 such rotations left at the end of the lattice phase from Y . R).
2. **Length and reading by eye.**
   - Shortest macros, length bounds and normal forms at every height (Card 5 Q1.3) are not needed.
   - Methods for reading the state by eye are not needed in the theory: seam reading (J3-R.5), the rim-peel rules
     of thumb (J3-R.2) and bookmarks and forced corrections for error recovery (J3-R.6). The program reads and
     checks the state; these can return as product features.
3. **Routes replaced by today's results, or not state-only.**
   - The family form of the Block lemma (do family parameters always exist): the method needs some clean step, not
     a fixed word family.
   - Commutator refinement [Q, k] to 0, Lemma O and landing-escape words, collateral recovery at 104: replaced by the
     pure twist.
   - Creating a partner carrier, restructuring, twin steps, history templates: replaced, or they read the history.
   - The special attack on the I_a@40 stop: no longer pursued on its own.
4. **Finiteness, state counts and worst-case distance** (deferred since 10 October): the method terminates by
   Lemma U and needs none of them.

## 5. Gaps

Two mathematical gaps; the owner closed M2 on 17 October 2026, so M1 is the one still open:
- **M1. A bounded step from the state alone (the core gap; open).** Bare existence is settled; a bound from the
  state's own features is not. Codex senior reviews on 15 October 2026 proved the following (`SENIOR-REVIEW.md`
  sections 1 to 3 and 9); there is no human review yet:
  - *Existence.* At every reachable state with an off piece, every unit U has a clean step with free set
    off(Z) \ U. Every letter has an admissible inverse, so the inverse of a history from the solved state is such a
    step. An exhaustive search over all words in order of length therefore finds one from the state alone and stops.
    It gives no bound on the length or the run time.
  - *Excursions.* With 0 < h < 3,097 off pieces, every clean step has at least two jumble letters, and its first
    jumble letter takes at least 3,097 - h lattice pieces off. At s61c (61 off) every clean step passes through at
    least 3,036 off pieces.
  - *One class is not enough.* At s61c every word that lands the carrier with the grips of one class, with retained
    letters before and after, takes all 86 lattice chambers of the carrier's cell off. A restoring word uses two
    classes between its first and its last jumble letter. From a single-unit state the whole word must make exactly
    the initial holes bad, type by type.
  - *A bound for s61c.* A word of 4,402 letters takes s61c to the lattice (re-run by the integrator), so a search in
    order of length from that state alone stops by length 4,402; the lattice phase then restores the stickers
    (corrected in round 8, R8-03). The word re-creates two partner carriers from lattice pieces, lands the 61-piece
    carrier together with one of them, and pure-twists the other. Its partner creation comes from the history, so
    it is a bound for this state, not a construction.
  - *Uniform bounds.* A uniform bound on the restoration length, or on one clean step at every reachable state,
    exists if and only if R(I_a) is finite (Card 5 Q1.1). R(I_a) is infinite (stage 10, 17 October 2026), so no
    uniform bound exists. A search that bounds every piece's height is finite, and retained preparation for a given
    middle word has an explicit finite bound; both bounds are astronomically large.
  - *Finiteness (round 9).* Open then (`SENIOR-REVIEW.md` section 14). A word repeated without end is periodic,
    with a period that depends on neither the word nor the state, so no repeated word raises heights without bound
    (T9-02). No single letter takes the deep state H6 of stage 04 to height 8 (exact, T9-04). For each m, the
    states whose heights are at most 2m have a finite maximum distance to the lattice, but no value of it is known
    (T9-05).
  - *Finiteness (rounds 13 to 20).* Open then (`SENIOR-REVIEW.md` sections 18 to 25). Every left height is at most
    6 if and only if P6 holds: at every reachable configuration, every cap of a grip of another branch that wholly
    contains a piece of height at least 6 is straddled (T13-03). P6 is false (round 15, T15-01): after the fixture
    records 1 to 723, four admissible letters take seven pieces of model orbits 19 to 24 to height 8. Grip 593
    departs at record 85 and stays out; grip 501, which is not out, raises the seven pieces, which lie in cap 593,
    to height 6; and 593, which is not among the 16 grips nearest 501, raises them to 8. Inside the inscribed ball
    the only blocking is that of the 16 grips nearest an out grip (T13b-02), and a cap that holds part of a high
    island is blocked by the island itself (T13b-05). So the trap lemma of round 14 (T14-03), under which an island
    raised by a grip that was not out stays 2 higher until that grip returns, applies only when every other cap
    that holds the whole island in one of the raiser's 48 non-retained orientations is among the 16 grips nearest
    the raiser; at 501 that fails. A high island can also enter the cap of a grip that is already out (T15-02).
    R(I_a) is finite if and only if the clean steps have a uniform length bound (T15-04; this does not need
    hypothesis H). Every cap decision behind Proposition HB is exact (REFX-EXACT), and Proposition HB is unchanged.
    Round 16 found no height 10: every continuation of that relay that was examined stops at a cap blocked by the
    16 grips nearest an out grip or straddled by a height-2 piece outside the inscribed ball (T16-01). An
    abstraction that tracks only the inner cohorts reaches height 10, so a bound would have to use the outer pieces
    (T16-02). A pattern moved by a symmetry cannot pump heights (T16-03), and the periodicity of repeated words
    cannot by itself prove finiteness (T16-04). Round 17 reached height 14 (E17-4, E17-6, E17-7). The word of
    T15-01 cut down to the causal past of the seven pieces, a return of grip 593 that keeps them at height 8, and a
    jumble at cap 582 give height 10 (147 letters). One more letter at 582 turns the pieces into a pose that a free
    cap of another branch holds whole, and a jumble there gives height 12 (149 letters). A letter at 576, one at 573
    and a jumble at 526 give height 14 (152 letters). At that raise the hypothesis TH of the trap lemma holds, so
    the pieces stay at height 14 until the raiser returns. Round 18: TH is decided by four orientations at the
    raiser and not by height (T18-01, T18-02); at most 120 grips are out at once, so a relay of raises at grips that
    were in needs returns to go on without bound (T18-04). Two lemmas of the integrator, not yet reviewed, explain
    the stop at 14 (E18-1 to E18-3): a raise followed only by letters at the raiser and its return is one retained
    letter at the raiser (excursion identity), so each return of 526 gives a state that one retained letter at 526
    gives before the raise; and TH holds wherever some vertex of the pieces lies deeper in the raiser's cap than a
    point of its norm can lie while it is also in a cap at 36 degrees or more from the raiser (lens criterion). The
    pieces lay that deep in cap 526, and in cap 486, where clearing gives a second raise to 14. Round 19 proved both
    lemmas (T19-01, T19-03), gave an exact form of the lens criterion and a sharper exact criterion that covers
    every trap of the census (T19-03, T19-04), and proved that traps cannot carry the height: while the pieces stay
    high, every rise beyond one comes from a raise at a grip that is already out or from a fresh raise where TH
    fails (T19-08). At most three out grips hold the pieces at once (T19-07). The integrator's frame lemma, not yet
    reviewed, explains the blocking: a jumble at a grip maps exactly the cuts of its branch onto cuts, so a piece
    that it moves off the lattice never straddles a cut of that branch, and the rim raise at 576 used pieces that
    the raiser 582 had returned to the lattice (E19-1 to E19-3). Round 20 proved the frame lemma, and its covering
    corollary inside the current occupied body (T20-01); it does not hold inside the 600-cell, because jumbled caps
    leave empty regions that a clean cut can cross next to the pieces (E20-3). Frames are decided by height: all
    cuts at height 0, the 120 cuts of the tree branch at height 2 and none from height 4 (T20-02, E20-1), so a clean
    cut is lined only by lattice pieces and pieces of height 2 in its branch (E20-2). The number of raises at grips
    that are already out, or fresh raises where TH fails, is not bounded while the pieces stay high, because one
    such raise and its inverse repeat without end (T20-03), and at the trap before (526, 12) letters at other grips
    free no other raiser (T20-05). Whether left heights are bounded at all stayed open after round 20.
  - *Finiteness (17 October 2026).* Settled: left heights are unbounded and R(I_a) is infinite
    ([stage 10](../10-x1-unbounded/README.md)). An explicit family of words, admissible from the solved state,
    alternates jumbles at two grips 36 degrees apart whose caps both hold seven pieces whole. Two finite Good sets
    keep the other pieces of each cap from blocking the other cut, and every letter raises the seven pieces by 2.
    A ball certificate of seven label-returning words keeps the family going without end. So bounds for M1 must
    depend on the state.
  - *Round 21 (17 October 2026; `SENIOR-REVIEW.md` section 26).* A joint attack: two sealed analyses, by the
    senior reviewer and by the solver, a cross-examination of each by the other, and the integrator's experiments.
    - Statements. A strict clean step lands U and leaves every other off piece at its exact pose; a free one only
      lands U without new off pieces. Strict steps bound phase 1 by n·B(n, h) for a monotone B; free steps by
      Σ B(n, H_i) with H_{i+1} = H_i + 2B(n, H_i), since a letter changes a height by at most 2 (R21-01). Whether
      strict steps always exist is open; a word-independent obstruction for them is proved and not yet tested
      (R21-02).
    - Theorem D (the integrator's, reviewed): a computable B(n, h) valid at every reachable state, or any computable
      bound on pose tables, exists exactly when reachability is decidable, and exactly when a restoring path needs to
      climb only a computable height above the state's own heights. So an explicit bound for M1 in its general form
      would decide reachability; Theorem D neither settles nor refutes M1.
    - Explicit bounds on classes given by checkable hypotheses: a unit that is the only off content of an edge cell
      with free poles and enough holes has a strict step of at most 2·min(C(N, 86), C(N, n)) + 718 letters (R21-07);
      a high unit that passes a ladder of edge-cell checks at every level falls by 2 per level through Theorem C's
      pure word, at most 360h letters plus the vacancy preparations (R21-03, R21-04); the X1 states are recognized
      from the state and restored in at most h/2 + 141 letters (R21-09). In X, σ and the end of the I_a fixture the
      ladder fails at its first level for every high unit, and at the fixture end 186 of the 305 high units have no
      admissible lowering letter (E21-3).
    - Moves: the twelve items at a grip of a unit's own branch that lower it form one coset of the retained items
      (R21F-03); a four-letter lens macro turns the common content of two clean caps and returns everything else
      when its cap tests pass (R21F-02). None of this covers ordinary scrambles.
  - *Option B (stage 12, from 17 October 2026).* A completeness conjecture for left heights 2 and 4, B′(h), with
    four template obligations, passed a plan check. Its first two experiments, at three height-2 states: every unit
    is the whole content of one class cell; besides grip blocking, the vacancy count N1 is the T1 check that fails
    most often; and the strict-step falsifier R21-02 holds everywhere, so it cannot refute strict steps there
    ([README](../12-low-height/README.md)).
  - *The whole method (round 4 audit).* Heuristic steps, a fair exhaustive fallback and the lattice phase form a
    complete method that reads only the state (Theorem M\*). Where the toolkit applies, its clean steps have an
    explicit bound, and the lattice phase has one for every input. So M1 is a question of bounds and coverage, not
    of existence or of a general method. The reviewer's next question: does every reachable depth-one state with
    no toolkit step pass the three-phase partner template?
  - *Partners from the state.* A landing word W whose Bad set has exactly the size of what it lands, type by type,
    can be inverted to create carriers from lattice pieces without the history, after two finite checks. With it,
    the three-phase template of the s61c certificate (create partners, land two carriers at once, pure-twist the
    rest) is decidable from the state, with an explicit but astronomical bound. s61c needs two created partners,
    and the creator first needs a preparation that fills six of its source chambers (not run). Whether every state
    with one deficient carrier passes such a template is open.
  - *Coverage at depth one (round 5).* Whether every reachable depth-one state without a toolkit step admits a
    partner template is still open; there is no proof and no reachable counterexample. The round proved three
    bounded extensions to test before new creators (`SENIOR-REVIEW.md` section 10):
    - retained conjugation: a legal retained word A before a toolkit closure and A^-1 after gives a clean step of
      at most 2r + 2B + b letters, with a finite setup search from the state (M1-18; first test r = 1, not run);
    - the template of M1-11 decides one catalogue of cores, not every creator and middle, and an edge middle needs
      u <= e (M1-16);
    - a finite exact test for free grips, with a positive case at I_a@115: grip 194 is free and its letter lands
      1,410 off pieces (M1-19, re-run by the integrator).

  What stays open is a construction from the state alone with an explicit bound on the word length. By Theorem D
  (round 21), a computable bound in (n, h) valid at every reachable state exists exactly when reachability is
  decidable. The driver now has the exhaustive fallback, so it is complete (Theorem M\*), but a fallback step has no
  length bound and is practical only for very short words; a coverage theorem for the heuristic steps would bound
  the run. (The runs of section 6 used v4, which tried the pure twist on at most 8 units, stopped after 80 steps and
  had no fallback.) M1 contains two narrower questions:
  - *V existence.* N1/N2 are necessary only and a peeling certificate is sufficient only; no theorem guarantees V,
    and the searches do fail (unit 1040 first at stop62@233: no V). A breadth-first search over whole hole sets
    decides vacancy from the state, with at most Q_safe hole sets; searching orbit by orbit does not. One search
    over target images decides it exactly, and a shortest vacating word has at most min(Q_T, Q_H) - 1 letters
    (M1-17).
  - *Scope of the pure twist.* The word exists whenever three grips are free (Theorem C, `SENIOR-REVIEW.md`). The
    closure still needs the cell's 86 chambers vacated, which needs enough holes in the right orbits. The step
    does not cover:
    - a unit with pieces of height four or more: no word of one class lands them (`SENIOR-REVIEW.md` section 9);
    - a foreign frame part in the block (the class-P alphabet cannot move it: LC-04);
    - frozen blocks (LC-05);
    - carriers in triangle or tetrahedron cells: they cannot be twisted in place, but pure transport words land
      them in another cell of the same kind (LC-13; at stop62 10 of the 15 units are such carriers); the driver's
      heuristic steps do not use these words yet;
    - a small carrier without slack: s61c, 61 off pieces, where no whole-edge-cell closure can be clean because
      the cell has 86 chambers and there are only 61 holes (LC-07); it needs created partners (M1-13).
  The family form of the Block lemma was one candidate route to M1; Theorem S rules it out at 245. The question
  itself is unchanged.
- **M2. Lattice-phase length against "a person can replicate it" (closed by owner decision, 17 October 2026;
  bounds proved).** Theorem R itself is proved, with a model review (section 3); the gap was the length. The owner
  closed it for the theory: the bounds below stand, and help that lets a person carry out the lattice phase with
  the program's assistance moves to the engineering line ([decision](../../../../docs/wiki/decisions/owner-decisions-2026-10-17-jumbling-m2.md)).
  Round 6 (`SENIOR-REVIEW.md` section 11) measures the length in two ways, which are the integrator's reading of
  the owner decision of 15 October (the owner has not fixed a measure): twists L, and actions A, where a person
  performs cap twists and applications of macros that the program stores and places (7,200 placements each), with
  the cost of creating the macros counted apart.
  - *Measured.* The committed cascade took 1.5 to 2.3 × 10^8 twists on the runs so far, 5 to 7 years at one twist
    per second. With the replaced seeds of round 7 it restores the two synthetic inputs in 4.5 × 10^7 twists, about
    1.4 years (section 6.3), and with the schedule of round 11 in 1.58 × 10^7 twists, about 6 months (section 6.4).
  - *Upper bounds (proved, model review).* A construction with the committed seeds and a shorter frame recipe needs
    at most 356,565,166 twists for every input, about 11 years at one per second, with expected length at most
    231,334,003 for a uniform input on G (M2-04). A person who replays the 35 relocated seeds and their inverses as
    70 stored macros needs at most 4,840,993 actions (single twists and macro applications), about 56 days at one
    per second (expected number at most 3,098,745); creating the macros costs 128,076 twists. The committed code's
    cascade has the bound 771,844,512, and 356,666,844 by the exact count of R7-03; it is the cascade that the
    driver's lattice phase runs. With the replaced seeds of round 7 the cascade as it runs needs at most 71,516,526
    twists for every input, about 2.3 years, and the construction at most 71,514,544 (expected length at most
    46,189,354). Creating the 70 macros then costs 14,048 twists, and the action bounds do not change (4,841,151 as
    the cascade runs). These bounds are for the cascade; the corrections add at most 25,852 twists (AU-05).
  - *Lower bounds (proved, model review).* Some input needs at least 224,601 retained twists, about 2.6 days, and
    all but a fraction 10⁻⁶ of the inputs need at least 224,599 (M2-02). With any fixed library of 70 macros, some
    input needs at least 104,774 actions. A library chosen for the input escapes any bound on actions alone. These
    bounds count retained words only; a solution that leaves the lattice through jumble twists is not covered
    (made explicit in round 8, R8-07).
  - *Shorter seeds (computed; round 7).* Commutators whose supports meet in one piece give pure three-cycles for
    33 of the 35 orbits (S1 and S2). Round 7 shows that the cascade needs no fibre alignment, only the position
    cycle on the buffers and legal collateral, with re-indexed pairs on D5 and A5 (proved, model review, R7-01). It
    relocates the candidates onto the buffers. 31 seeds are replaced: 17 by pure three-cycles and 14 by spine
    prefixes, among them O15 (11,574 to 106 primitives) and O17 (1,546 to 24), for which no pure three-cycle was
    found. O10, O24, O25 and O26 keep their seeds. No other choice among the recorded candidates, in any order,
    gives a smaller bound (round 8, R8-02).
  - *Direct three-cycles (proved, model review; round 10).* The position loop can perform each three-cycle (A X Y)
    as g c' g⁻¹: one application of the seed c', conjugated by a word g that fixes A and takes X and Y to the other
    two buffers, with at most e_o + d_o letters (e_o from 4 to 18, computed exactly). With the frame passes
    unchanged, a construction of R then needs at most 50,560,184 twists for every input, about 1.6 years at one per
    second (at most 32,201,480 on average), or 4,675,165 actions, about 54 days. The library and its creation cost
    do not change. This is about 225 times the lower bound, and it is not implemented. Setting the frames during
    the position loop instead (twisted cycles) fails as proposed: pieces that start in place are never visited,
    and one fibre map of the proposed cycle does not depend on its return (T10-03).
  - *Fewer seed applications (proved, model review; round 11).* Twisted direct cycles with two returns set the frame
    of each piece that the position loop places. The misframed pieces that start in place are cleared two at a time
    (C2 pairs and frame triples) or one at a time (direct pairs), with two seed applications each, and the orbits
    without frames use the fewest three-cycles, at most N_o/2. A construction of R then needs at most 20,866,844
    twists for every input, about 8 months at one per second (expected length at most 17,010,356), or 4,655,642
    actions, about 54 days, with the same library: about 93 times the lower bound. A cascade that solves one orbit
    at a time with conjugates of these seeds, without using their collateral on purpose, needs at least 95,024 seed
    applications for some input, 15,643,520 twists charged for the seeds (T11-06). That floor counts applications
    times seed length, not the length of a freely reduced word. The schedule is about 1.334 times the floor, so
    within that class a larger gain needs shorter seeds (round 12, R12-01). It is implemented (section 6.4): on the
    six inputs of its acceptance check, every orbit stays within its term of the bound (measured).
  - *Closed (17 October 2026).* The owner judged the question: M2 is closed for the theory, and the help a person
    needs moves to the engineering line. Shorter candidates for O10, O28, O03, O01 and O18 (now the largest terms),
    the exhaustive frame check of R7-02, a uniform sampler for the synthetic inputs (M2-05) and a cascade order that
    admits the spine prefixes dropped for their collateral (M2-06; over the recorded candidates the order gains
    nothing, R8-02) remain possible improvements, not open gaps.

Two evidence gaps:
- **E1. Coverage.** The method has been run on the inputs of section 6 only. Deep scrambles (the I_a@40 scale,
  about 94,000 off pieces, multi-frame and frozen blocks) were not run with it. Stage 11 (17 October 2026) began a
  census of phase U alone on unrestricted random scrambles ([README](../11-scramble-census/README.md)).
- **E2. Proof text and review.** On 15 October 2026 a Codex senior review checked the proofs of stage 08 (Fact 1',
  Lemmas U, D and CM, Propositions F and GH, Theorem D1) and of the lattice phase (Theorem R, Theorems A and B5,
  the span of the library). It proved Lemma C as Theorem C with a length bound, and proved the M1 results above. It
  corrected the statements of Lemma CM and Theorem R and fixed three smaller points (`SENIOR-REVIEW.md`). A
  milestone audit then proved Theorem M\*, the toolkit theorem and the lattice-phase bound (section 9 there). That is a
  model review: no human has reviewed these proofs. The owner's plan (17 October 2026): once M1 is solved, a
  complete handoff document of the theory goes to a strict Lean formalization by the owner's agent (Archon), which
  is to close E2.


## 6. Results

### 6.1 End-to-end restorations verified so far (15 October 2026, hand-steered order)

These runs were steered by hand: at each state the researcher chose which unit to restore and which tool to run.
Every step is a clean step found by the tools of this directory and the earlier stages. Every result below was
checked independently: the off-lattice words and the corrections were replayed on the exact simulator, and the
retained cascade was evaluated as an exact permutation program on all 259,800 sticker labels (round 12, R12-03). The
records are in `earlier/`.

| Input | Off pieces at start | Off-lattice phase | Lattice phase | Check |
| --- | ---: | --- | --- | --- |
| stop62: the stop of the 62-twist seam descent after the I_a@40 scramble | 860 | 14 clean steps, 14,098 twists, collateral 0 at every step | corrections 8,129 twists; retained cascade 151,011,430 twists | **certificate**: `verify_u` replay all steps ok, 0 off at the end; `verify_late` pass, final digest = solved, labels identity |
| X: a reachable state on E3's history (stage 01) | 107 | 3 clean segments, 2,401 twists (X to E1 to E2 to E3) | corrections 21,888 twists; retained cascade 223,378,194 twists | **certificate**: end-to-end check rebuilt from X through every segment word; pass, final digest = solved |
| I1 and I2: lattice states of stage 02 | 0 | none | corrections 21,660 and 21,888 twists; cascades 222,690,068 and 224,683,850 twists | **certificate**: `verify_late` pass, final digest = solved |

The stop62 state lies on one replayed word from the solved state: the first 40 records of the I_a@40 fixture (94,147
pieces off the lattice), then the 62-twist seam descent, which stopped at 860 off pieces (stage 05, section 5). So
that 40-twist scramble is restored to the solved state end to end. The three records join by state digest: the
history replay ends at `9e70c4b9…`, where the 14-step chain starts; the chain ends at `981c5bb2…`, the input digest
of the lattice run. The lattice run read that state from a table, not with `verify_late.py --chain`.

### 6.2 The state-only driver v4

`gdrive.py` started on 15 October 2026 at 06:49 UTC on two inputs, with no hand steering. These runs used the
driver v4 (`gdrive.py` as committed at `5c679b9`), before the M\* change: no fallback, at most 80 steps.

| Input | Off at start | Steps | Off | Result |
| --- | ---: | --- | ---: | --- |
| stop62 | 860 | 7 driver steps (10, 22, 404, 982, 1,072, 1,296 and 1,436 twists), then 1 pure-twist step (762 twists): 5,984 twists | 269 | **stopped** after 4 h 12 min: at 269 neither heuristic found a step, and v4 has no fallback. `verify_u.py` replays all 8 steps (`all_ok`, final off 269). Record: `runs/stop62-v4/` |
| e2run24: the end of the 24-twist E2 run of stage 05, not the state E2 of stage 01 | 690 | 7 driver steps (44, 150, 946, 1,026, 990, 1,362 and 1,446 twists) and 2 pure-twist steps (288 and 934 twists): 7,186 twists | 194 | **stopped** after 10 h 4 min: at 194 neither heuristic found a step, and v4 has no fallback. `verify_u.py` replays all 9 steps (`all_ok`, final off 194). Record: `runs/e2run24-v4/` |

At stop62@269 six units remain: 1040 (86 pieces), 18512 (78), 23955 (61), 9117 (17), 18517 (15) and 18445 (12).
- The driver step found no unit it could restore.
- The pure-twist stage found 288-letter words for the carrier 1040 (36 lattice pieces in its edge cell) and for the
  unit 18445 (all 86 chambers of its cell hold lattice pieces), but neither closure script found a vacating V.
- It skipped the others. Three units of one frame (78, 61 and 15 pieces) lie in an edge, a triangle and a
  tetrahedron cell, and `cellpure3.py` needs the whole frame in one cell (`SENIOR-REVIEW.md` section 9:
  containment would suffice). The 17-piece unit does not lie in an edge cell; the script stopped with an error
  (`KeyError`) instead of reporting that (fixed on 15 October 2026: it now reports that no cycle passes through
  the unit's cell).

The hand-steered run of section 6.1 took the same start to 0 in 14 clean steps. Under M\* the fallback would take
over at 269. By M1-03 every clean step there has at least two jumble letters, and the fallback costs about 300 days
for length 2 alone (section 2, step 4), so the run ends here; the next bounded steps to try are those of
`SENIOR-REVIEW.md` section 10 (retained conjugation first).

On e2run24 the off count went 690, 642, 613, 565, 548, 505, 502, 391, 280 and 194. At 613 and at 280 the driver
step found nothing, and the pure twist restored a unit: the 48-piece unit 8888 at 613 (V empty, 288 twists), and
the 86-piece unit 64184 at 280 (V of 287 letters around a 360-letter word, 934 twists). At 194 four units remain:
64213 (87 pieces), 16589 (50), 35795 (45) and 18445 (12).
- The driver step found no unit it could restore.
- The pure-twist stage found words for 35795 (504 letters) and 18445 (360 letters; all 86 chambers of its cell hold
  lattice pieces), but neither closure script found a vacating V.
- For 64213 and 16589 `cellpure3.py` stopped with a `KeyError`, as for the 17-piece unit at stop62: their cells, a
  tetrahedron cell and a triangle cell, are not edge cells, so no pair tool moves them. Re-run on the final state with
  the fix, it reports no cycle for both.

As at stop62, the fallback of M\* would take over at 194, and its cost ends the run there.

Fresh scrambles of 2, 3, 4 and 6 records (6,194 to 18,247 off pieces, `gen_scramble.py`) were started and stopped:
scoring one state with 58 units took about an hour, so a run would take days. They are **not run** (gap E1). The
I_a@40 fixture state itself (94,147 off pieces) was not run with v4.

### 6.3 The lattice phase with replaced seeds (round 7)

The stage 03 cascade with 31 of its 35 seeds replaced (`SENIOR-REVIEW.md` section 12), on the committed synthetic
inputs (random elements of G given as labelled stickers; not uniform on G, M2-05):

| Input | Pieces displaced | Committed cascade | With the replaced seeds | Check |
| --- | ---: | --- | --- | --- |
| uniform:1 | 176,499 | 229,763,734 twists, 190,925 factors | 45,009,802 twists, 191,143 factors | **certificate**: the stage 03 checker evaluates the solution with its own code and reaches the identity on all 259,800 stickers |
| uniform:2 | 176,492 | 230,484,810 twists, 191,093 factors | 44,940,982 twists, 191,089 factors | the same |

The stored input arrays hash to the committed digests, and the same runner with no seed replaced reproduces the
committed column exactly. Records: `runs/realigned-cascade/`; the replacements: `data/realign.json.gz`.

### 6.4 The lattice phase with the schedule of round 11 (16 October 2026)

The construction of T11-04 and T11-05 (`SENIOR-REVIEW.md` section 16), with the replaced seeds of round 7:
twisted direct three-cycles that set the frame of every position they finalize, then the misframed positions that
started in place (C2 pairs, direct pairs on O06 and O17, frame triples on O28 and O34), then the buffer cleanup of
T10-04; on the 24 orbits without frames, the cheaper of the direct loop and the fewest three-cycles for each
input. `scripts/schedule11/sched11.py` builds it; `restore_schedule11.py` runs it, separately from the driver.

| Input | Round 7 runner | Round 11 schedule | Largest orbit length / its bound | Check |
| --- | --- | --- | --- | --- |
| uniform:1 | 45,009,802 twists | 15,761,436 twists, 88,352 factors | 0.974 (O10) | **certificate**: the stage 03 checker evaluates the solution with its own code and reaches the identity on all 259,800 stickers |
| uniform:2 | 44,940,982 twists | 15,760,112 twists, 88,350 factors | 0.974 (O10) | the same |

The acceptance check `check_schedule11.py` was written before the implementation and fixed its interface. It
recomputes the bound of T11-05 orbit by orbit (sum 20,866,844) and runs the four inputs that T11-05 asks for: all
positions in place with wrong frames (15,198,318 twists; largest ratio 0.967, O00), three-cycles that close at the
buffer (1,704,164), paired transpositions (1,914,908) and the non-abelian cleanup on O06 and O34 (664,676 and
914,142). After every orbit it evaluates the straight-line program with the rules of the stage 03 checker and
checks that the orbits solved before stay solved; it requires every orbit within its term of the bound and the
identity at the end. On the two synthetic inputs it runs the runner and checks its written output the same way:
the solution is cut into the per-orbit slices that the run summary records and replayed slice by slice, every
orbit's length is recomputed, exactly the two requested runs must be present, and the unchanged stage 03 checker
must pass on the same output. A review of the check found three ways to pass a wrong output (lengths per orbit
taken from the runner's summary, an empty list of runs, and an empty product that the stage 03 checker rejects);
these clauses close them, and `mutants_check11.py` shows that the check rejects eight mutants of a valid output
that cover the three findings and accepts the output itself. The implementation did not change. The bound
itself is proved by model review, not by these runs (measured on six inputs). Records: `runs/schedule11/`.


## 7. Reproduction

Prerequisites: the research tree `research/jumbling` on `main` (PR #79, merged on 15 October 2026 as `9fc2419`; the
runs used commit `27a001c` of its branch, and up to `9fc2419` only two `.md` files under `research/jumbling` changed),
the repository root (read only), Python 3.13 with NumPy 2.5 on Linux, `ulimit -v 7000000` per process. Decompress `data/library-merged.json.gz` before the lattice phase.

Run the method on a pose table (this is `scripts/run/run_e2.sh` with neutral paths):

    python3 -I scripts/gm/gdrive.py <research/jumbling> scripts/echain <start table> <name> <work dir> --par 2 \
        --lattice scripts/lattice/scripts <repo> data/library-merged.json scripts/lattice/checker

The stop62 run added `--seed <dir>` for the word searches of the earlier stop62 runs (section 6). The runs of
section 6 used v4 (`gdrive.py` at commit `5c679b9`). The current driver adds the fallback and three options:
`--heuristic-seconds N` (default 21,600), and `--max-steps N` and `--fallback-max-length N`, experiment bounds
outside M\* that are off by default.

Test the outer loop and the word search on synthetic states (16 tests, no simulator), and the fallback path end to
end (one jumble twist at cap 0 of the solved state, then the driver with `--fallback-only` and again with its
heuristic steps killed after one second; about two minutes):

    python3 -I -B scripts/gm/test_mstar.py
    python3 -I -B scripts/gm/test_fallback.py

Fresh scrambles:

    python3 -I scripts/gm/gen_scramble.py <research/jumbling>/theory <out dir> 2:9101 3:9102 4:9103 6:9104

Check a chain of the off-lattice phase independently (simulator, start table and recorded words only):

    python3 -I scripts/gm/verify_u.py <research/jumbling> scripts/echain runs/<name>/chain/chain.json out.jsonl

Accept a chain only when the last line of `out.jsonl` has `"all_ok": true` and no row has `"ok": false`. The script's
exit status does not reflect the verdict.

Check that a stored pure twist has Bad(W) equal to its cell (Theorem C; for chain step 14 of the stop62 chain, W is
the 288 letters after the 543 letters of V):

    python3 -I scripts/gm/omega_check.py <research/jumbling>/theory scripts/echain \
        earlier/stop62-860-to-0/chain/step014-word.json 543 288 1 out.json 5 11

The v4 runs of section 6.2 check the same way, with `runs/stop62-v4/chain/chain.json` and
`runs/e2run24-v4/chain/chain.json` (decompress each start table first); the stored `verify_u.jsonl` next to each
chain is that check's output.

Short pure elements for the lattice phase (gap M2, `SENIOR-REVIEW.md` section 11). These scripts read the retained
model from `assets/`, so run them from the repository root; `<09>` is
`research/jumbling/restoration/09-general-method`:

    python3 -I -B <09>/scripts/short-cycles/kplus.py
    python3 -I -B <09>/scripts/short-cycles/short_cycles.py 12286 short-cycles.json
    python3 -I -B <09>/scripts/short-cycles/check_cycles.py <09>/data/short-cycles.json.gz
    python3 -I -B <09>/scripts/short-cycles/short_cycles2.py 12286 0,2,6,15,17,28,29 12 short-cycles-2.json
    python3 -I -B <09>/scripts/short-cycles/check_cycles.py <09>/data/short-cycles-2.json.gz
    python3 -I -B <09>/scripts/short-cycles/short_cycles2.py 12286 15,17 12 two-b.json 8

The first prints the order of K+ (7,200) and the number of primitives that do not act as their rotation (0). The
second writes the shortest verified element per orbit and kind (about 6 minutes); the committed result is
`data/short-cycles.json.gz`. The third re-checks every recorded word from the primitives alone; accept the result
only when it prints `"all_ok": true`. The fourth is the two-level search for the orbits the first leaves open (about
10 minutes; committed result `data/short-cycles-2.json.gz`, checked by the fifth). The sixth repeats it for O15 and
O17 with overlaps of up to eight pieces; it writes an empty result (`{}`).

The replaced seeds and the cascade with them (round 7, section 6.3; from the repository root, `<03>` is
`research/jumbling/restoration/03-retained-group-and-lattice-restorer`):

    python3 -I -B <09>/scripts/realign/realign.py realign.json
    python3 -I -B <09>/scripts/realign/restore_realigned.py realign.json run uniform:1 uniform:2
    python3 -I <03>/checker/verify_restore.py . <research/jumbling> <03>/restore/library-final.json run report.json

The first certifies the replacements and prints the construction bound (about 50 s); its committed output is
`data/realign.json.gz`. The second runs the cascade with them (about 35 s). The third replays the result with the
stage 03 checker; accept it only when `report.json` has `"pass": true` and both runs restored to the identity.
`runs/realigned-cascade/commands.txt` also gives the control run and the independent hash of the inputs, and
`SENIOR-REVIEW.md` section 12 the bounds for the cascade as it runs.

The schedule of round 11 (section 6.4; from the repository root):

    python3 -I -B <09>/scripts/schedule11/test_sched11.py
    python3 -I -B <09>/scripts/schedule11/check_schedule11.py
    python3 -I -B <09>/scripts/schedule11/restore_schedule11.py <09>/data/realign.json.gz run uniform:1 uniform:2
    python3 -I <03>/checker/verify_restore.py . <research/jumbling> <03>/restore/library-final.json run report.json
    python3 -I -B <09>/scripts/schedule11/mutants_check11.py run

The first runs five exact gadget tests (about 6 s). The second is the acceptance check (about 5 minutes; it
also runs the next two steps itself, in a temporary directory; it exits 0 and prints `PASS`). The third runs the schedule on
the synthetic inputs (about 110 s) and the fourth replays it with the stage 03 checker (about 40 s); accept it
only when `report.json` has `"pass": true` and both runs restored to the identity. The fifth applies the
acceptance check to eight mutants of `run` (about 3.5 minutes; it prints `PASS` when each is rejected and `run`
itself passes). `runs/schedule11/commands.txt` gives the SHA-256 of the files not stored.

Check the lattice phase of a run that reached the lattice, rebuilding its start from the chain:

    python3 -I -B scripts/lattice/checker/verify_late.py <repo> <research/jumbling> data/library-merged.json \
        runs/<name>/lattice report.json --chain runs/<name>/chain/start-table.json runs/<name>/chain/step*-word.json

(decompress `runs/<name>/lattice/solutions.json.gz` first).

## 8. Checks not run

- Windows, DirectX, the native host (`tests/native/NativeHostRegression.cs`), input, long-session and performance
  checks: none run; nothing here is evidence for them. Timings are cloud timings of research code.
- A human review of the proofs: not done. The senior review of 15 October 2026 (`SENIOR-REVIEW.md`) is a model
  review.
- The method on deep scrambles (the I_a@40 scale): not run.
