# Jumbling cost leads (EN2 slice 3)

Status: the cost leads of packet ENG-0J revision 2, acceptance item 5
(`docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md`): DT-J1's
experiments X1 to X5, DT-M1's experiment XM1, the entry heights and
denominators, the cost of exact classification against the filtered test of
amendment A2, and the size of the exposed-face meshes. Results:
[`results/summary.md`](results/summary.md).

**Evidence kind.** Every number here comes from the J1 reference engine
(`research/jumbling/sim/`) and the research restorer
(`research/jumbling/theory/`), run headless in a Linux cloud container. A time
is a cloud timing of single-threaded Python research code: a lead for the
engine-language question, never performance evidence for a product engine, a
renderer or Windows/DirectX.

**Boundary.** These are measurements, not solver aids. A job answers about the
cap the caller names. Where a script walks caps in an order (X1) or reproduces
a research choice (X4), the order or the choice is a measurement input taken
from the census record, kept in the measurement script, and never offered as a
feature (the human-solve boundary of `docs/development-guide/AGENT_BRIEFING.md`
section 2).

## Census states

`census.py` rebuilds the records of the scramble census
(`research/jumbling/restoration/11-scramble-census/runs/`) step by step, as the
census replay check does: the scramble from (k, seed), checked against the
recorded history, then each step's twists, with N and the off count checked
before and after every step and J1's end digest checked against the census
replay check. `walk(seed)` yields each step with the restorer at the state
before it; the consumer must hand it back unchanged, and the walk checks that.
`state_at(seed, n)` returns one state. The research centroid cache goes to a
temporary directory, so nothing is written in the repository.

## Leads

Every command runs from the repository root with `python -B`, takes `--out DIR`
and refuses a directory inside the repository before it writes anything. Each
summary states the CPU count, the Python and NumPy versions and the commit
(`null` outside a Git checkout).

### Repair lists and X1

`repair_list.py` is the job of ENG-0J section 4 at a cap c the caller names:
for each non-identity item x in pinned order and each admissible misaligned cap
d that meets c, in cap order, after (c, x), y is d's item that leaves the
lowest N_d and z is c's item that leaves the lowest N_c, each tie going to the
smaller pinned index. A pair is formed only when neither d nor c is blocked or
closed and y strictly lowers N_d; it is listed when the three twists lower N,
as (x, d, y, z, ΔN), none marked best. ΔN is the sum of the three own-cut
changes (restoration M2), so z is never applied. The job applies each x once
and reuses the restorer's caches across d. `repair_list(S, c, max_pairs=None,
cancel=None)` returns a `RepairListResult` with status `completed`,
`cancelled`, `cut_off` or `error`; only a completed result has rows, and
`completed_rows` refuses the others. Coverage (pairs tried, items done, the
meeting caps) and timings are reported for every status.

`x1_walk.py` times the job at the 201 (step, cap) requests of the seed 7101
record's measurement walk (DT-J1 E13: caps by decreasing N_c through the
recorded repair cap at each of the 63 repair steps; 128,502 pairs). It checks
the request and pair counts, that the lists before each repair cap are
complete and empty, that the record's own pair is row 1 at the repair cap,
and every outcome against `density.py`'s recorded rows where they exist.

```sh
python -B research/jumbling/differential/leads/x1_walk.py --out DIR [--steps N ...] [--jobs K] [--rest PATH]
```

`--rest` takes the density records of the 48 repair steps that are not on main
(see "Inputs outside this repository"); without it those comparisons are
reported as unchecked. Workers split steps and replay with `census.walk`.
Writes `x1-requests.jsonl` and `x1-summary.json`.

### X5: release checks

`x5_release.py` checks one previewed twist (c, x) at the current revision:
N_c now, N_c after x only (never the other 59 items), the floor |In_c − Out_c|,
ΔN, and the counts of pieces by left tree height (n₀, n₂, n₄, n₆, above 6)
before and after, from the exact hypothetical poses of the inside pieces. The
left height is `trees.pose_tree(M)[0]`. The research `scores` oracle checks
each result afterwards, outside the timing. It runs at the first twist of the
63 repair steps of seed 7101 and at every misaligned admissible cap of the
`wj-I_a` end state, item 1.

```sh
python -B research/jumbling/differential/leads/x5_release.py --out DIR
```

Writes `x5-checks.jsonl` and `x5-summary.json`.

### Covers and X4

`covers.py` answers DT-J1 M4 for a named cap c: for each of the 80 caps d that
meet c (n_c · n_d ≥ 7 + 3√5 exactly, the same caps as `restore.NEAR`), whether
d covers c and c covers d, with the counts on c's face and every face piece
that straddles d. Inside sets are full certified sets, also for a blocked cap.
The answer is about the named pair only: no ordering and no next cap.
`covers_query(S, c)` returns a `CoversResult`; a blocked, closed or not
misaligned cap is an error with its reason.

`x4_covers.py` compares the exact relation with the research float centroid
relation at every cover step of the census records and traces each
disagreement (`blocked_d`, `rounding` or `unexplained`). At the 63 repair
steps of seed 7101 it times one covers query and compares the two recorded
flags of `runs/findability/ia-k40-s7101-steps.jsonl`. It also records where
the research rule's choice would change under the exact relation; that choice
is reproduced for comparison only.

```sh
python -B research/jumbling/differential/leads/x4_covers.py --out DIR [--seeds SEED ...] [--jobs K]
```

Writes `x4-pairs.jsonl` and `x4-summary.json`.

### XM1: face-match seats

`xm1_seats.py` (DT-M1) looks at every face-match step (`rule == 'seam'`) of
the census records: the cut's two sides, their lattice flags and domains, the
closing set Z (every non-identity item whose N_c after reaches the floor), and
whether Z lies in one seat. The seats are the right cosets A4₀·s of the
retained items A4₀ (items 0 to 11), computed by exact composition and checked
against the pinned seats {12, 16, 20, 24–32}, {13, 17, 19, 34–42},
{14, 21, 23, 43–51} and {15, 18, 22, 33, 52–59}; left cosets are recorded
too.

```sh
python -B research/jumbling/differential/leads/xm1_seats.py --out DIR [--seeds SEED ...]
```

Writes `xm1-steps.jsonl` and `xm1-summary.json`.

### Straddler sets and X2

`../straddlers.py` gives the full certified straddler set of a named cap (the
straddler-set rule of ENG-0J item 2) and an independent exact check of each
certificate; the harness section "Named-cap straddler sets" in
[`../README.md`](../README.md) describes both. `x2_straddlers.py` compares, at
all 600 caps of one state, the early-exit classification that
`State.survey` makes per grip with the full set: time, blocked flag, the
early-exit piece, u (the number of straddlers), and agreement. Certificate
checks run outside the full-set time.

```sh
python -B research/jumbling/differential/leads/x2_straddlers.py --out DIR (--scramble K SEED | --record PATH [--letters PATH] [--expect DIGEST]) [--caps C ...]
```

`--record` rebuilds a census-format record to its end with `census.walk` and
then applies the letters (a JSON list of `[grip, item]`); `--expect` checks the
measured state's digest. Writes `x2-caps.jsonl` and `x2-summary.json`.

### Exact against filtered classification

`classify_cost.py` classifies each cap twice on the same state, exact first
(`state.filtered = False`) and then with amendment A2's filtered sign test,
with `need_inside=True`. Status, inside set and blocked flag must agree; a
difference is recorded as a failure, never averaged away.

```sh
python -B research/jumbling/differential/leads/classify_cost.py --out DIR [--fixture I_a|S4|I_b] [--census SEED] [--scramble K SEED] [--caps C ...]
```

The default is the `wj-I_a` end state. Writes `classify-caps.jsonl` and
`classify-summary.json`.

### Entry heights

`heights.py` replays an input and records, after every `every`-th step, the
largest entry height and denominator as `wj.height` defines them (the largest
|a|, |b| or d over the non-K⁺ poses of the live pose table), the off count and
the pose-table size. The first crossings of 2³¹, 2⁵³, 2⁶³ and 2⁶⁴ are found at
every step. With `--widths` it also records the bit lengths of the two
operands of the exact sign test (`sim.kernel.sign5`), through a wrapper that
exists only during the run. The fixtures' stage values, the negative control's
digests and cap-0 heights and the words' end digests are checked.

```sh
python -B research/jumbling/differential/leads/heights.py --out DIR [--fixture NAME ...] [--journals] [--h6030] [--correction35] [--word PATH START ...] [--then LETTERS] [--start-digest D] [--every N] [--widths]
```

A start is `solved`, `census SEED` or `record PATH`. The default is every input
on main. Writes `heights-steps.jsonl` and `heights-summary.json`.

### Exposed-face meshes

`meshes.py` finds, for each of the 36 K⁺-orbit representative regions, the
3-faces on the cut hyperplanes and on the facet hyperplanes (the stickers), in
exact integer arithmetic, with an Euler check per face, and scales the counts
by the orbit sizes. It builds the base mesh of the exposed cut faces in memory
and compares the exact sticker faces with the retained float32 sticker asset
(`assets/mesh.json`, `assets/mesh_vertices.f32`), sticker by sticker.

```sh
python -B research/jumbling/differential/leads/meshes.py --out DIR [--write-mesh]
```

Writes `meshes-orbits.jsonl` and `meshes-summary.json`; `--write-mesh` also
writes the base mesh (`exposed-mesh-vertices.f32`,
`exposed-mesh-offsets.json`), which is a measurement, not a product asset.

### X3: whole-word previews

`x3_previews.py` replays a stored word from its start, one letter at a time,
each letter applied exactly once. The first refused letter ends the preview at
that prefix; a word whose `input_digest` differs from the start is refused
before any letter. It reports per-letter times and exact-evaluation counts,
and the effect at the end: pieces landed, lattice pieces lifted off, and other
off pieces at unchanged or changed exact poses.

```sh
python -B research/jumbling/differential/leads/x3_previews.py --out DIR [--u1] [--correction35] [--h6030] [--word PATH --start START ... [--then LETTERS] [--start-digest D] [--label NAME]] [--sample-every N]
```

The default is the three words on main: u1 (624 letters from the end of census
seed 7101), correction 35 (5,629 letters) and the X1 witness H6030 (3,156
letters). Writes `x3-letters.jsonl` and `x3-summary.json`.

## Inputs outside this repository

The full runs also used words and records that exist only on draft #112's
branch (commit `ec0f0ac`), extracted with `git show` into a scratch directory
outside the repository and never committed. Tests never read them.
[`results/summary.md`](results/summary.md) lists each with its path at
`ec0f0ac` and its SHA-256.

## Tests

`tests/test_jumbling_leads_repair.py`, `tests/test_jumbling_leads_covers.py`,
`tests/test_jumbling_straddlers.py` and `tests/test_jumbling_leads_scale.py`
run bounded smoke versions of each lead with NumPy, in one isolated worker
each under a shared unittest command (`tests/isolated_numpy.py`). Each writes
only in temporary directories outside the repository, and an audit hook fails
any other write. The Windows CI job runs them with the other headless checks;
a pass there is not Windows/DirectX evidence.
