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
3. **Pure-twist step** (only when the driver finds no step at Z). For each unit, largest first, `cellpure3.py`
   builds the class-cell model of the unit's class (the 120 caps of one of the five classes of grips; a cell is a
   sign vector over those caps) and searches the pure twist: commutator powers [x, y]^k of free class-P letters
   around a cycle of three or four edge cells through the carrier's cell X, exponents solved over Z5 so that the
   carrier's cell alone turns to label 0 (model check on every content, then exact replay). Its W moves only the
   carrier and the lattice pieces of X (Bad(W) = X). The units with a word are then closed, the unit with the fewest
   lattice pieces in X first, by `tclose2.py`; on 'no V', by `tclose2m3.py` (the same closure with a targeted
   depth-two peel for V). The first clean step in that order is taken.
4. **Stop.** If neither kind of step restores a unit, the run stops at Z and records, per unit, why the pure twist
   failed (precondition of the cell model, no cycle of edge cells, model check, ghost premises, no V) and the
   driver's best candidates.

Every step is written as a chain step (word with digests, unit, free set). `verify_u.py` replays the chain from the
start table with the simulator only: every twist admitted, both digests, unit restored, collateral 0, every moved
off piece inside the unit or the free set, off count falling.

### Phase 2: lattice phase

When no piece is off the lattice, `restore_lattice_late.py` computes the invariant nu (F2^43 + F5^2 coordinates)
from the configuration, chooses the correction combination C from the library of 36 macros (rank 34 over F2 and 2
over C5, which spans A_J), solves the labelled configuration with the retained-group cascade R, and applies C last.
`verify_late.py --chain` rebuilds the start of this phase from the chain's start table through every step word
(each twist an I_a menu element and admissible, every digest checked), evaluates R with its own code, builds the
target T = solved . C^-1 in the simulator, checks that T has the labels R produces, then applies C and requires the
solved configuration (identity labels on all 259,800 stickers).

- Completeness: **Theorem R** (author's proof, unreviewed): when the library spans A_J, the restorer solves every
  configuration with all pieces on the lattice. The library's rank (34 over F2, 2 over F5) is a **certificate**; it
  equals A_J = C2^34 x C5^2, whose lower bound is a certificate (40 words) and whose upper bound is proved
  (Theorem A and B5, unreviewed).
- Lengths: R is about 1.5 to 2.2 x 10^8 twists, C about 10^4. These are machine-scale (section 5, gap M2).


## 3. What the method rests on, and the status of each part

| Part | Statement (short) | Status |
| --- | --- | --- |
| Basic properties | Legal twists keep the state valid and are invertible; a lattice configuration has exactly one piece per chamber; the centre pieces never leave their poles. | proved (Codex senior review) |
| Lemma U | A clean step lowers the off count by at least the unit's size; so every run of clean steps terminates. | proved |
| Closure V W V^-1 | Every step of every chain here is a clean step. | certificate per step (`verify_u.py` replay) |
| Theorem B, Proposition N, Theorem A | Vacancy for Bad(W): necessary counts N1/N2; a peeling certificate gives V, its thresholds and a length bound. | proved (Codex math review) |
| Lemma D | Per-type count: \|Bad_t(W)\| is at most the number of holes of type t; off counts by type only fall along a run. | proved |
| Proposition GH, Lemma CM | Glued holes move with their carrier; a carrier moved by free K+ letters keeps the step clean. | proved |
| Lemma C (class-cell pure twist) | Commutator powers around a cycle of three or four edge cells of one class twist the carrier's cell alone. | measured on 4 states, each word replayed exactly; no written proof |
| Lemma TT (stage 06) | A K+-transported word moves a congruent block exactly as the original word moves its block. | proved (stage 06); whether the transported step is clean depends on the target state: measured on 2 carriers |
| Theorem S at stop62@245 | Every block-family word has \|Bad\| >= 333 > 245 holes, so no family closure works there. | certificate |
| G = K | The retained group is the whole kernel of the invariants (35 position and 11 frame certificates). | certificate (independent checker) |
| A_J = C2^34 x C5^2 | The lattice-phase invariant group. | lower bound certificate (40 words); upper bound proved |
| Theorem R | With a library that spans A_J, the lattice restorer solves every lattice configuration. | proved |
| Termination of the whole method | Phase 1 by Lemma U, phase 2 by construction. | proved |
| Completeness of phase 1 | Every reachable state with an off piece has a clean step that the method finds. | **open** (gap M1) |

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
- **M1. Step existence (the core gap; open).** No theorem says that every reachable state with an off piece has a
  clean step that the method finds from the state. It contains two narrower questions:
  - *V existence.* N1/N2 are necessary only and a peeling certificate is sufficient only; no theorem guarantees V,
    and the searches do fail (unit 1040 first at stop62@233: no V).
  - *Scope of the pure twist.* It needs one carrier in one clean class cell with enough holes of its types. It does
    not cover multi-frame blocks, frozen blocks, carriers across cells, or a small carrier without slack (the 61
    carrier alone, s61c). Lemma C has no written proof; it is measured on 4 states.
  The family form of the Block lemma was one candidate route to M1; Theorem S rules it out at 245. The question
  itself is unchanged.
- **M2. Lattice-phase length against "a person can replicate it".** The cascade is about 1.5 to 2.2 x 10^8 twists;
  at one twist per second that is 5 to 7 years. Program assistance does not close this. Theorem R is unreviewed.

Two evidence gaps:
- **E1. Coverage.** The method has been run on the inputs of section 6 only. Deep scrambles (the I_a@40 scale,
  about 94,000 off pieces, multi-frame and frozen blocks) were not run with it.
- **E2. Proof text and review.** Lemma C has no written proof. Lemma CM, Proposition GH, Theorem R and the Card 5
  upper bound have no independent mathematical review.


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

`gdrive.py` started on 15 October 2026 at 06:49 UTC on two inputs, with no hand steering. Status at 08:12 UTC:

| Input | Off at start | Steps so far | Off now | Status |
| --- | ---: | --- | ---: | --- |
| stop62 | 860 | 4 driver steps (10, 22, 404 and 982 twists) | 620 | running |
| e2run24: the end of the 24-twist E2 run of stage 05, not the state E2 of stage 01 | 690 | 1 driver step (44 twists) | 642 | running |

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

Check the lattice phase of a run that reached the lattice, rebuilding its start from the chain:

    python3 -I -B scripts/lattice/checker/verify_late.py <repo> <research/jumbling> data/library-merged.json \
        runs/<name>/lattice report.json --chain runs/<name>/chain/start-table.json runs/<name>/chain/step*-word.json

(decompress `runs/<name>/lattice/solutions.json.gz` first).

## 8. Checks not run

- Windows, DirectX, the native host (`tests/native/NativeHostRegression.cs`), input, long-session and performance
  checks: none run; nothing here is evidence for them. Timings are cloud timings of research code.
- A written proof of Lemma C, and an independent mathematical review of the new proofs (Lemma CM, Proposition GH,
  Theorem R, the Card 5 upper bound): not done.
- The method on deep scrambles (the I_a@40 scale): not run.
