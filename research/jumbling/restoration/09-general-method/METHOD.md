# A state-only general method for menu I_a (v4), 15 October 2026

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
on the length of a fallback step. The driver v4 used here is a partial implementation: it has no fallback, it
stops after `--max-steps`, and it skips the lattice phase when the last permitted step reaches the lattice.

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
   section 3), which v4 does not use yet. The units with a word are then closed, the unit with the fewest lattice
   pieces in X first, by `tclose2.py`; on 'no V', by `tclose2m3.py` (the same closure with a targeted depth-two
   peel for V). The first clean step in that order is taken.
4. **Stop.** If neither kind of step restores a unit, the run stops at Z and records, per unit, why the pure twist
   failed (precondition of the cell model, no cycle of edge cells, model check, ghost premises, no V) and the
   driver's best candidates.

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
  These are machine-scale (section 5, gap M2).


## 3. What the method rests on, and the status of each part

| Part | Statement (short) | Status |
| --- | --- | --- |
| Basic properties | Legal twists keep the state valid and are invertible; a lattice configuration has exactly one piece per chamber; the centre pieces never leave their poles. | proved (Codex senior review) |
| Lemma U | A clean step lowers the off count by at least the unit's size; so every run of clean steps terminates. | proved (Codex senior review, 15 October 2026) |
| Closure V W V^-1 | Every step of every chain here is a clean step. | certificate per step (`verify_u.py` replay) |
| Theorem B, Proposition N, Theorem A | Vacancy for Bad(W): necessary counts N1/N2; a peeling certificate gives V, its thresholds and a length bound. One certificate must cover all targets at once: separate certificates per orbit do not prove it. | proved (Codex math review; the simultaneity condition from the Codex senior review, `SENIOR-REVIEW.md` section 9) |
| Lemma D | Per-type count: \|Bad_t(W)\| is at most the number of holes of type t; off counts by type only fall along a run. | proved (Codex senior review, 15 October 2026; the remark after Corollary D'' needs f_t = 0, fixed) |
| Proposition GH, Lemma CM | Glued holes move with their carrier; a carrier moved by free K+ letters keeps the step clean, with the same free set. | proved (Codex senior review, 15 October 2026: GH as written; the statement of CM corrected to keep the same free set, which the constructions use) |
| Lemma C (class-cell pure twist) | If the two poles of the carrier's edge cell X and one common neighbour are free, a word of at most 720 letters at those three grips twists X alone and leaves every other piece at its exact pose. Closed by a peeling certificate V that vacates X, V W V^-1 is a clean step of at most 2 L_cert + 720 letters. | proved (Codex senior review, 15 October 2026; no human review): an exact certificate on one triangle of edge cells, transported by symmetry, re-run by the integrator. The code's cheaper powers may also permute lattice pieces outside X, which a clean step allows; the claim that they return every other content to its point was false. Applied on 4 states, each word replayed exactly; at s86a Bad(W) equals X exactly (checked). `SENIOR-REVIEW.md` sections 4 and 5 |
| Other cell kinds | A real carrier in a vertex cell lands in place (at most 104 letters); one in a triangle or tetrahedron cell cannot, but a pure transport word lands it in another cell of the same kind (at most 240 or 192 letters), every other piece keeping its exact pose. A rigid block of one frame lies in one cell. | proved (Codex senior review, 15 October 2026; no human review), with exact certificates re-run by the integrator; assumes the class's grips are free. Not used by v4, whose pure twist covers edge cells only. `SENIOR-REVIEW.md` section 6 |
| Lemma TT (stage 06) | A K+-transported word moves a congruent block exactly as the original word moves its block. | proved (stage 06); whether the transported step is clean depends on the target state: measured on 2 carriers |
| Theorem S at stop62@245 | Every block-family word has \|Bad\| >= 333 > 245 holes, so no family closure works there. | certificate |
| G = K | The retained group is the whole kernel of the invariants (35 position and 11 frame certificates). | certificate (independent checker) |
| A_J = C2^34 x C5^2 | The lattice-phase invariant group. | lower bound certificate (40 words); upper bound proved (Codex senior review, 15 October 2026) |
| Theorem R | With a library that spans A_J, the lattice restorer solves every reachable lattice configuration (stickers). | proved (Codex senior review, 15 October 2026; restricted to reachable configurations) |
| Termination of the whole method | Phase 1 by Lemma U, phase 2 by construction. | proved |
| Existence of a clean step | At every reachable state with an off piece, every unit has a clean step, and an exhaustive search over words in order of length finds one from the state alone and stops. | proved (Codex senior review, 15 October 2026; no human review); no bound on length or run time |
| Small states need excursions | With 0 < h < 3,097 off pieces, every clean step has at least two jumble letters, and its first jumble letter takes at least 3,097 - h lattice pieces off. | proved (same review) |
| Theorem M\* (a general method) | Heuristic steps, a fair exhaustive search when they find nothing, then the lattice phase with the corrections last: restores every reachable configuration from the current state alone, in at most \|off\| clean steps. | proved (Codex senior review, 15 October 2026; no human review), `SENIOR-REVIEW.md` section 9; no bound on the length of a fallback step |
| Toolkit theorem | For a unit whose frame belongs to class P, a class-P landing word W that passes finite checks at the state (free grips, exact landing action, every other off piece lands or keeps its exact pose, Bad(W) the union of its cells, vacancy for all targets at once) closes to a clean step of at most 2(Q_safe - 1) + b letters, b = 720, 104, 240 or 192 by cell kind. | proved (same review), given the certificates of `SENIOR-REVIEW.md` sections 5 and 6. Units with pieces of height four lie outside it; 150 twists of the I_a fixture make 1,411 such pieces (re-run) |
| Lattice-phase length | At most 1,324,686,034 twists for every input, with the committed cascade and library. | proved (same review; the integer bound re-run by the integrator) |
| Completeness of the driver v4 | v4 finds a clean step at every reachable state with an off piece. | **open** (gap M1): v4 is a partial implementation of M\*. The pure twist is tried on at most 8 units, the run stops after 80 steps (and skips the lattice phase if the last permitted step reaches the lattice), and there is no exhaustive fallback |

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

Two mathematical gaps:
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
  - *A bound for s61c.* A word of 4,402 letters restores s61c (re-run by the integrator), so a search in order of
    length from that state alone stops by length 4,402. The word re-creates two partner carriers from lattice
    pieces, lands the 61-piece carrier together with one of them, and pure-twists the other. Its partner creation
    comes from the history, so it is a bound for this state, not a construction.
  - *Uniform bounds.* A uniform bound on the restoration length, or on one clean step at every reachable state,
    exists if and only if R(I_a) is finite (Card 5 Q1.1, open). A search that bounds every piece's height is finite,
    and retained preparation for a given middle word has an explicit finite bound; both bounds are astronomically
    large.
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

  What stays open is a construction from the state alone with an explicit bound on the word length, and a
  coverage argument for the driver v4 itself. As written, v4 tries the pure twist on at most 8 units, stops after
  80 steps and has no exhaustive fallback. A complete method needs either that fallback or a coverage theorem. M1
  contains two narrower questions:
  - *V existence.* N1/N2 are necessary only and a peeling certificate is sufficient only; no theorem guarantees V,
    and the searches do fail (unit 1040 first at stop62@233: no V). A breadth-first search over whole hole sets
    decides vacancy from the state, with at most Q_safe hole sets; searching orbit by orbit does not.
  - *Scope of the pure twist.* The word exists whenever three grips are free (Theorem C, `SENIOR-REVIEW.md`). The
    closure still needs the cell's 86 chambers vacated, which needs enough holes in the right orbits. The step
    does not cover:
    - a unit with pieces of height four or more: no word of one class lands them (`SENIOR-REVIEW.md` section 9);
    - a foreign frame part in the block (the class-P alphabet cannot move it: LC-04);
    - frozen blocks (LC-05);
    - carriers in triangle or tetrahedron cells: they cannot be twisted in place, but pure transport words land
      them in another cell of the same kind (LC-13; at stop62 10 of the 15 units are such carriers); v4 does not
      use these words yet;
    - a small carrier without slack: s61c, 61 off pieces, where no whole-edge-cell closure can be clean because
      the cell has 86 chambers and there are only 61 holes (LC-07); it needs created partners (M1-13).
  The family form of the Block lemma was one candidate route to M1; Theorem S rules it out at 245. The question
  itself is unchanged.
- **M2. Lattice-phase length against "a person can replicate it".** The cascade took 1.5 to 2.3 x 10^8 twists on
  the runs so far, 5 to 7 years at one twist per second; the bound for every input is 1,324,686,034 twists, about
  42 years. Program assistance does not close this. The gap is the length:
  Theorem R itself is proved, with a model review (section 3).

Two evidence gaps:
- **E1. Coverage.** The method has been run on the inputs of section 6 only. Deep scrambles (the I_a@40 scale,
  about 94,000 off pieces, multi-frame and frozen blocks) were not run with it.
- **E2. Proof text and review.** On 15 October 2026 a Codex senior review checked the proofs of stage 08 (Fact 1',
  Lemmas U, D and CM, Propositions F and GH, Theorem D1) and of the lattice phase (Theorem R, Theorems A and B5,
  the span of the library). It proved Lemma C as Theorem C with a length bound, and proved the M1 results above. It
  corrected the statements of Lemma CM and Theorem R and fixed three smaller points (`SENIOR-REVIEW.md`). A
  milestone audit then proved Theorem M\*, the toolkit theorem and the lattice-phase bound (section 9 there). That is a
  model review: no human has reviewed these proofs.


## 6. Results

### 6.1 End-to-end restorations verified so far (15 October 2026, hand-steered order)

These runs were steered by hand: at each state the researcher chose which unit to restore and which tool to run.
Every step is a clean step found by the tools of this directory and the earlier stages, and every result below was
checked by an independent replay. The records are in `earlier/`.

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

### 6.2 The state-only driver v4 (running)

`gdrive.py` started on 15 October 2026 at 06:49 UTC on two inputs, with no hand steering. Status at 10:19 UTC:

| Input | Off at start | Steps so far | Off now | Status |
| --- | ---: | --- | ---: | --- |
| stop62 | 860 | 7 driver steps (10, 22, 404, 982, 1,072, 1,296 and 1,436 twists) | 355 | the driver found no step at 355; the pure-twist stage is running (below) |
| e2run24: the end of the 24-twist E2 run of stage 05, not the state E2 of stage 01 | 690 | 2 driver steps (44 and 150 twists) | 613 | running |

At stop62@355 the pure-twist stage found words of 288 twists for three of the seven units, all in edge cells (two
86-piece carriers and a 12-piece unit), and is closing them. It skipped the other four:
- three units of one frame (78, 61 and 15 pieces) lie in an edge, a triangle and a tetrahedron cell, and
  `cellpure3.py` needs the whole frame in one cell (`SENIOR-REVIEW.md` section 9: containment would suffice);
- the 17-piece unit does not lie in an edge cell (at the start it lay in a tetrahedron cell, which needs a
  transport word); the script stopped with an error (`KeyError`) instead of reporting that.

Fresh scrambles of 2, 3, 4 and 6 records (6,194 to 18,247 off pieces, `gen_scramble.py`) were started and stopped:
scoring one state with 58 units took about an hour, so a run would take days. They are **not run** (gap E1). The
I_a@40 fixture state itself (94,147 off pieces) was not run with v4.


## 7. Reproduction

Prerequisites: the research tree `research/jumbling` on `main` (PR #79, merged on 15 October 2026 as `9fc2419`; the
runs used commit `27a001c` of its branch, and up to `9fc2419` only two `.md` files under `research/jumbling` changed),
the repository root (read only), Python 3.13 with NumPy 2.5 on Linux, `ulimit -v 7000000` per process. Decompress `data/library-merged.json.gz` before the lattice phase.

Run the method on a pose table (this is `scripts/run/run_e2.sh` with neutral paths):

    python3 -I scripts/gm/gdrive.py <research/jumbling> scripts/echain <start table> <name> <work dir> --par 2 \
        --lattice scripts/lattice/scripts <repo> data/library-merged.json scripts/lattice/checker

The stop62 run added `--seed <dir>` for the word searches of the earlier stop62 runs (section 6). Fresh scrambles:

    python3 -I scripts/gm/gen_scramble.py <research/jumbling>/theory <out dir> 2:9101 3:9102 4:9103 6:9104

Check a chain of the off-lattice phase independently (simulator, start table and recorded words only):

    python3 -I scripts/gm/verify_u.py <research/jumbling> scripts/echain runs/<name>/chain/chain.json out.jsonl

Accept a chain only when the last line of `out.jsonl` has `"all_ok": true` and no row has `"ok": false`. The script's
exit status does not reflect the verdict.

Check that a stored pure twist has Bad(W) equal to its cell (Theorem C; for chain step 14 of the stop62 chain, W is
the 288 letters after the 543 letters of V):

    python3 -I scripts/gm/omega_check.py <research/jumbling>/theory scripts/echain \
        earlier/stop62-860-to-0/chain/step014-word.json 543 288 1 out.json 5 11

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
