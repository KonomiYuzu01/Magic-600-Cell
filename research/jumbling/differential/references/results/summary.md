# Jumbling reference values: results (EN2 slice 4)

Status: measured on 22 October 2026 for packet ENG-0J revision 2,
acceptance items 3 (its remaining refusals and shown cases) and 8, with
amendment A5 (`docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md`).
The modules and their commands are in [`../README.md`](../README.md).
Acceptance item 7, the optional-state round trip, is covered by the bounded
tests of `tests/test_jumbling_session.py`; it has no full-mode run.

**Evidence kind.** Source and fixture replay with the J1 reference engine
(`research/jumbling/sim/`), the research theory scripts
(`research/jumbling/theory/`) and the restoration records. Every run was
headless, in a Linux cloud container with four CPUs, CPython 3.11.15 and
NumPy 2.3.5 (`tools/python/engine.txt`). A time is a cloud timing of
single-threaded research code while up to four measurement processes shared
the four CPUs. It is a lead, never performance evidence for a product engine,
a renderer or Windows/DirectX.

**Boundary.** Every value answers about a state, cap, word or correction set
that a record names. No run chooses, ranks or proposes a twist, a cap, a
correction or a word.

## At a glance

Each line answers an acceptance item against the records; the sections below
give the numbers, the checks and the caveats.

- **ν (EH-10, A5).** On the three research lattice states E3, I1 and I2, ν
  equals TABLE-NU's row and the restore summary's row. All 40 words of
  CERT-LB-V2 are admissible from solved, end on the lattice and give the
  certificate's ν.
- **Corrections (EH-11).** All 36 corrections add their row to ν from solved
  and, by literal replay, at E3, I1 and I2. In J1 the literal replay on those
  states costs about 0.2 s a letter, about twelve times the replay from
  solved.
- **Observations (EH-05, EH-26).** At 20 states (the 18 W-J fixture prefixes,
  the I_a@40 stop and the deep scramble) the two rim routes agree and every
  recorded value is equal. At the I_a@40 stop five statements of the prose on
  off-grip seams agree; its "20 caps that hold off-grip seams" does not
  (39 caps hold an off-grip cell, 25 a whole group).
- **Height-4 states.** The six height-4 states of stage 12, rebuilt from
  their census records with every recorded digest checked, equal the stage 12
  README in n₂, n₄, n₆, the off count and the blocked caps.
- **Word checks (EH-16, EH-25; item 3).** u1, b20 and b06-14 (672,417
  letters) equal their records in every recorded field. b14 after b12 and b03
  at 737 off are shown with a false step flag and 17 and 43 lattice pieces
  lifted off, as item 3 expects. R22B-34's negative controls are not stored
  as words, so that case is unavailable.

## Results

### ν on the research lattice states (EH-10)

`nu_refs.py --cert-lb` rebuilt the three research lattice tables and replayed
the 40 words of CERT-LB-V2 from solved.

| Table | State digest | ν against TABLE-NU's row | ν against the restore summary's row |
|---|---|---|---|
| E3 (`01-closure-x-to-e3/e2-b-prime/D3/E3-table.json.gz`) | `fb199d99…` | equal | equal |
| I1 (`02-three-block-family/runs/I1/end-table.json.gz`) | `d4c74f8e…` | equal | equal |
| I2 (`02-three-block-family/runs/I2/end-table.json.gz`) | `67bafbc0…` | equal | equal |

Each rebuilt table's digest equals the digest that the table records, and
TABLE-NU's row was matched by the SHA-256 of the decompressed table.

CERT-LB-V2 (`04-card5-lattice-invariants/certs/cert-lb-v2.json`): all 40
words are admissible at every prefix from solved, end on the lattice and give
the certificate's ν, which is non-zero for every one of them. The nine fixed
coordinates are 0 at every end (`check_fixed`). The words have 5 to 5,629
letters, 30,567 in all; a word's replay took 1.1 s at the median and 93 s at
most (MB2-5, 5,629 letters), 441 s in all.

### Corrections from solved and from the lattice inputs (EH-11)

`corrections.py` checks, for each of the 36 corrections of the library
(SHA-256 of the decompressed library `8a13eb6c…`), that ν after the
correction equals ν before plus the correction's row.

| Start | Path | Corrections passed | Letters | Wall time |
|---|---|---|---|---|
| solved | transported action (its witness is the literal replay from solved) | 36 of 36 | 23,932 | 363 s |
| E3 | literal: every twist replayed on the state | 36 of 36 | 23,932 | 4,618 s |
| I1 | literal | 36 of 36 | 23,932 | 4,799 s |
| I2 | literal | 36 of 36 | 23,932 | 5,119 s |

At E3, I1 and I2, ν before the corrections is not zero (16 or 17 non-zero
coordinates), and every row adds exactly. From solved a row's time follows
the correction's length: 0.2 to 0.9 s for the corrections of 5 and 9 letters,
94 s for correction 35 (5,629 letters), 17 ms a letter at the median. The
literal replay on the three lattice inputs took 0.20 to 0.22 s a letter at
the median (945 to 1,025 s for correction 35), about twelve times the replay
from solved, where every piece shares one pose. This is the cost that the
transported action avoids in `play`; it is a lead about J1 only.

### Observations (EH-05, EH-26)

`obs_refs.py` rebuilt 20 states and observed each one by both rim routes
(`rim.py` and `seams.balance`), which agree at every state, with the domain
keys partitioning the pieces exactly as `rim.Domains` does.

- **W-J fixtures.** The 18 prefixes of `research/jumbling/theory/results/trees-fixtures.json`
  (S4₀, I_a and I_b at 10, 20, 60, 150 and 300 records and at their ends, 640,
  841 and 885): both height histograms (pieces and pose classes by left and
  right height) equal the recorded ones.
- **The I_a@40 stop** (I_a@40 and the 62 recorded restorer twists): digest
  `9e70c4b9…`, N 1,192, grip N 1,057 and 15 misaligned caps, as
  `seams-I_a-40-stop.json` records; 860 pieces off.
- **The deep scramble**: digest `3061502b…` and 165,529 pieces off, as
  `deep-scramble-I_a.json` records. Its N and grip N are both 150,914 (no
  recorded value to compare).

In the run on the reviewed code (`obs-final`), an observation took 18 s at
the median per route (14 to 61 s, the deep scramble the slowest). Rebuilding
the states took 952 s of the run's 2,323 s, the deep scramble 170 s and the
ends of the fixtures 147 to 226 s. A first run on `11615e0`, before the review
fixes, gave the same output files apart from their times.

**The off-grip seams at the I_a@40 stop** against the prose of
`research/jumbling/theory/open-problems.md` (R1 in detail):

| Prose | Observed | Result |
|---|---|---|
| 135 misaligned cells on off-grip seams | 135 | equal |
| four groups by domain, of 59, 37, 25 and 14 cells | 59, 37, 25, 14 | equal |
| the group of 59 lies in caps 30, 34, 145, 31, 73 and 89 | the caps that hold the whole group: 30, 31, 34, 73, 89, 145 | equal |
| blocked holders, among them 31, 73, 96, 152, 153, 295 and 302 | all seven hold an off-grip cell and are blocked | equal |
| free aligned holders, among them 64, 89, 183, 283, 290, 301 and 319 | all seven hold one, are admissible and have N_c = 0 | equal |
| "the 20 caps that hold off-grip seams (aligned caps allowed)" | 39 caps hold an off-grip cell (23 admissible, 16 blocked); 25 hold a whole group (19 admissible, 6 blocked) | **differs** |

The reference reading: a cell is occupied on one side only, its plane is no
grip's cut, and a cap holds it when it contains every vertex of the closed
cell. No count of this reading is 20, by holding a cell or a whole group,
with or without the blocked caps. The prose's experiment script was not kept
(it says so), so its reading of "hold" cannot be recovered. This is recorded
as a prose discrepancy: it does not change a recorded value of a results file,
and the other values above agree.

### The height-4 states of stage 12

`obs_refs.py --height4` rebuilt the six height-4 states that stage 12's
`r22d25_d19_stops.py` describes. Each is the end of a census record (its
scramble, then its descent) followed by 6 to 25 committed letters; the run
checked the record's stop digest and off count, the digests recorded after
letters 7 and 12 where there are any, and the state's own digest. At every
state the two rim routes agree, the domain keys partition the pieces, and
n₂, n₄, n₆, the off count and the blocked caps equal the state's row in
`research/jumbling/restoration/12-low-height/README.md` (on #112's branch):

| State | Start record | n₂ | n₄ | n₆ | Off | Blocked caps | N | Grip N | Result |
|---|---|---|---|---|---|---|---|---|---|
| `bbef1c8e…` | stage 11, seed 7302 | 102,960 | 936 | 0 | 103,896 | 562 | 132,930 | 33,143 | equal |
| `2430f4cc…` | stage 11, seed 7302 | 102,750 | 734 | 0 | 103,484 | 563 | 133,818 | 33,174 | equal |
| `ac127dc3…` | stage 12, seed 7304 | 95,674 | 797 | 0 | 96,471 | 559 | 117,736 | 29,958 | equal |
| `cc5ef7ec…` | stage 12, seed 7305 | 108,453 | 1,419 | 0 | 109,872 | 578 | 134,346 | 19,284 | equal |
| `16c4aefe…` | stage 12, seed 7306 | 108,641 | 2,591 | 0 | 111,232 | 574 | 140,018 | 21,720 | equal |
| `895b45de…` | stage 12, seed 7307 | 98,984 | 1,585 | 0 | 100,569 | 562 | 115,782 | 27,217 | equal |

The README records no N or grip N, so those two columns are observed only.
Its counts of height-4 and height-2 units are kept in each output file but
not compared: the acceptance item names the heights, the off counts and the
blocked caps. Every off piece is at height 2 or 4 in its left tree and at
height 0 in its right tree.

`bbef1c8e` is also the state that the renderer line exported in the render
format (`work/experiments/renderer-wj/deep/bbef1c8e-summary.json`). That
export's digest, N, off count, left heights and pose counts (610 on the
lattice, 2,188 off) equal this run's.

A state took 1,136 to 1,347 s, of which replaying its record and letters was
1,005 to 1,205 s; the two rim routes took 28 to 38 s each, and the off-grip
seams 68 to 98 s. The whole run took 7,335 s.

### Word checks (EH-16, EH-25; acceptance item 3)

`word_check.py` checked each recorded word from its recorded start, on a copy
of the state, and compared every field that its record holds (the mapping is
in each output row). In the stage 12 records a unit's "collateral" is the
number of lattice pieces the word lifts off, and "strict prime" is the step
flag.

| Case | Start (digest, pieces off) | Letters | Reference | Fields compared | Result | Time |
|---|---|---|---|---|---|---|
| u1 | census 7101's end (`7e128820…`, 1,193) | 624 | `12-low-height/runs/lc13-k40/replay-b01.json`, row 1, and the output digest of `steps/u1.json` (both on `main`) | 12 | equal | 38 s |
| b20 | b16's end (`fde9e0fd…`, 507) | 40,500 | `03-b20-stepchk.json` | 12 | equal | 343 s |
| b14 | b12's end, reached by u12 from census 7101's end (`4a0f4271…`, 1,152) | 60 | acceptance item 3 (EH-15): 17 lattice pieces lifted off, step flag false | 2 | equal | 56 s |
| b03 | the 737-off table (`2e3b8f99…`, 737) | 160 | acceptance item 3 (EH-24): step flag false | 1 | equal | 4 s |
| b06-14 | b15's end (`31139216…`, 71) | 672,417 | the last line of `b06-14-apply.log` | 11 (the log records no letter count) | equal | 10,728 s |
| R22B-34 | | | its negative controls are described, not stored as words | | unavailable | |

The twelve fields are the start and end digests, the letters, the unit's size
and how many of its pieces end on the lattice, the lattice pieces lifted off,
the other off pieces and how many keep their exact pose or land, the off
counts before and after, and the step flag. A case's time includes rebuilding
its start state (u1 replays census 7101's walk, b20 and b03 load a pose
table).

- **u1** lands its whole unit of 72 pieces, lifts nothing and keeps the other
  1,121 off pieces at their exact poses: step flag true, ΔN = +214.
- **b20** lands its unit of 60 pieces and keeps the other 447 exact: step flag
  true, ΔN = −118.
- **b14** after b12's step is admissible at every prefix, lands its unit of 78
  pieces and lifts 17 lattice pieces off (off 1,152 to 1,091): step flag
  false, ΔN = −40. Its time includes replaying u12's 1,158 letters to reach
  the start, whose digest is checked against `b12-replay.json`.
- **b03** at 737 off is admissible at every prefix, lands its unit of 63
  pieces and lifts 43 lattice pieces off (737 to 717): step flag false,
  ΔN = +34.
- **b06-14** lands its unit of 10 pieces and the other 61 off pieces too, so
  it ends with no piece off the lattice (end digest `02a27279…`), with at
  most 3,097 off along the way: step flag true, ΔN = −272. Its time is almost
  all the replay of its 672,417 letters, 16 ms a letter.

Both shown cases are results, not refusals: the word check reports the false
step flag and the lifted pieces, and whether a commit is allowed follows from
admissibility and protection alone (DT-J1 A7). The commit side is in the
bounded tests: an admissible word that lifts lattice pieces off is shown and
then committed from its preview
(`test_admissible_collateral_is_shown_and_committed_from_preview` in
`tests/test_jumbling_word_check.py`).

## Runs

Every run ran this package's code from the repository checkout, with
`python -B`, outputs outside the repository and NumPy 2.3.5 from
`tools/python/engine.txt` in a virtual environment outside it. At most four
measurement processes ran at once on the four CPUs; the one-minute load
average stayed at about 4 while four ran. `R` is
`research/jumbling/differential/references` and `INPUTS` the scratch
directory of the inputs from draft #112's branch.

| Output directory | Command | Started (UTC) | Code at start | Wall time |
|---|---|---|---|---|
| `corrections` | `R/corrections.py --from solved` | 13:47:59 | `a0caf51` | 363 s |
| `corrections` | `R/corrections.py --from E3 --literal` | 13:47:59 | `a0caf51` | 4,618 s |
| `corrections` | `R/corrections.py --from I1 --literal` | 13:47:59 | `a0caf51` | 4,799 s |
| `corrections` | `R/corrections.py --from I2 --literal` | 13:47:59 | `a0caf51` | 5,119 s |
| `nu` | `R/nu_refs.py --cert-lb` | 13:54:40 | `11615e0` | 451 s |
| `words` | `R/word_check.py --extract INPUTS --cases u1 b20 b14 b03 r22b34` | 14:41:10 | `6867ab4` | 442 s |
| `words-b06-14` | `R/word_check.py --extract INPUTS --cases b06-14` | 15:05:02 | `c85154f` | 10,728 s |
| `obs-final` | `R/obs_refs.py` | 15:08:03 | `c85154f` | 2,323 s |
| `height4-final` | `R/obs_refs.py --states --extract INPUTS --height4` | 16:15:34 | `e3ddf92` | 7,335 s |

"Code at start" is the commit checked out when the run started; the `commit`
field in a run's summary is the commit when the summary was written, which is
later for a long run. `word_check.py` exits with status 1 when a requested
case is unavailable, as R22B-34 is; its four compared cases passed. Two
first runs are not kept. A first observation run (`R/obs_refs.py`, 14:02:15
on `11615e0`, 2,327 s) was repeated by `obs-final` on the reviewed code. A
first height-4 run (14:48:36 on `6867ab4`, 4,883 s) reconstructed four of
the six states, with the values that `height4-final` gives, and could not
start the other two (next section); `height4-final` reran all six.

## Provenance of the runs

The review fixes (`f4d0a07`, `6867ab4`) landed while the first runs went on.
What they change in the code those runs used:

- `corrections.py`, `nu_refs.py` and `nu.py` are the same at `a0caf51`,
  `11615e0` and the final commit, so the `corrections` and `nu` runs ran the
  final code.
- `observations.py` changed: `compare` refuses an incomplete reference and
  compares the pieces per pose class, and `observe` records them. `obs-final`
  ran the reviewed code on all 20 states. Its output files equal the first
  run's apart from the times, and both rim routes still agree at every state
  under the stricter comparison.
- The word checks started on `6867ab4`, the reviewed code, and on `c85154f`,
  which changed only this summary's draft and the README; `word_check.py` is
  the same there and at the final commit.
- The full-mode height-4 run found a defect that the tests had not: two of
  the six states start from stage 11's `ia-k3000-s7302.json`, which, like the
  stage 12 records, is not on `main`, and `obs_refs.py` read it from the
  checkout (`FileNotFoundError`). `e3ddf92` reads both kinds of start record
  from the extraction and adds a test; its own review passed with no
  finding. `height4-final` ran the six states on `e3ddf92`.

## Inputs outside this repository

The word checks of b20, b06-14, b14 and b03 and the height-4 states read files
that exist only on draft #112's branch (commit `ec0f0ac`). They were extracted
with `git show` into a scratch directory outside the repository, and every
extracted file equals its committed blob byte for byte (1,523 files of the
stage 12 tree and stage 11's `ia-k3000-s7302.json`, of which the runs read
those below). Tests never read them.

Word checks (paths relative to
`research/jumbling/restoration/12-low-height/runs/round22/`):

| File at `ec0f0ac` | SHA-256 | Case |
|---|---|---|
| `line-a/literal/run1/z-02-b16.json.gz` | `b659492c5495131cb7eff0073097677d3bf5763d3e66dc2f7fe58a9ac08941e5` | b20 |
| `line-a/literal/run1/03-b20-program.json` | `57c27076e4bfef65c15a77e9282dbe3e09ed58a6da7adcd3b5884774e24df91a` | b20 |
| `line-a/literal/run1/03-b20-V-word.json.gz` | `1ae91e59bc4be23a02fa58fa4b4f24d165b47f188c8f63785d331775c9c8bb9a` | b20 |
| `line-a/replay2/inputs/cores/b20-core.json` | `b5abdc4c23cef1969cd149a70bf284107ce570406b7604c8f1856b1282b5dcef` | b20 |
| `line-a/literal/run1/03-b20-stepchk.json` | `9ba81a4c729cd3cdfbf5a972b25b44a10e82015df1ade96791552b3b49376873` | b20 |
| `line-a/units/eb5-k40-k00-b20.json` | `eeabb5e67fc49e920211b04bb745a5eec00175051edc69cf6bc48e7abb23315c` | b20 |
| `line-a/literal/run2b/z-04-b15.json.gz` | `f13df51db228299a579cbd2f7b3445773e528db00c315371e9d643fe36986edc` | b06-14 |
| `line-a/literal/run3/t1/b06-14-program.json` | `69a4b2a2b9fe3de985ef09756f893f387e8d2a8c8803ae5191dca255817f5678` | b06-14 |
| `line-a/literal/run3/prep2/x-word.json` | `e4cdf39f19f1bade77904fcfcac05101b968088f80450e86fd8607dbbaf26ea6` | b06-14 |
| `line-a/literal/run3/t1/b06-14-W.json` | `d5dd3155912c7218445e44e16c819f01594caefd5a8a040c0a3060293386b290` | b06-14 |
| `line-a/literal/run3/t1/b06-14-V-word.json.gz` | `9b4d864ceb88d8b62ec20c8fde07b20c6ac49b00f71be384310599b692fd418c` | b06-14 |
| `line-a/literal/run3/t1/b06-14-winv.json` | `132ef1019379e624eb7680a2a84f4d923fe72c3bc81679520bad3aeb1e7c3110` | b06-14 |
| `line-a/literal/run3/t1/b06-14-apply.log` | `2119e6d075bed397ddf4b23ec5e159c5eb943349ba29141611791ac45b98d164` | b06-14 |
| `line-a/units/eb5-k40-k00-b06.json` | `0f933cae3c89216094d481356ff8a0d22b34c105e8296d64a3ce642b717d872f` | b06-14 |
| `t1prime-k40/steps/u12.json` | `66f735c65e1c6756b7b74ce666a74b3c9e7eb70c662843e44cb38b4c16909cc4` | b14 |
| `line-c/b14-after-b12/b12-replay.json` | `167f7305aed7a6632c959856df51caae73e6fade176d51f21b3bd8534c5c5887` | b14 |
| `line-a/census-class3.json` | `b3255c78b2c3a27049ba35bf9f9758f88d93070ffdee4c93284edfd4e6f539d6` | b14 |
| `line-a/units/eb5-k40-k00-b14.json` | `6539253d5bf12b99f73768ab2981800acd733c02cf7d539cb2ae18a809e30965` | b14 |
| `line-a/replay/tables/737.json.gz` | `8e67b4429df2c3dcc462db2a003b4330167703c94d33d7cc7d7a2f952ca0f8c2` | b03 |
| `t1prime-k40/steps/u3.json` | `f32869f5224e85bdd8696ceeefa8551c0dff8855afb753f5c546c39dfc33f24c` | b03 |
| `line-a/units/eb5-k40-k00-b03.json` | `779a15c7d5c263dea5704d6cec2dd9178cf9bce9a9e3b5ab3e3a08fa871b2b7e` | b03 |

u1 reads only files on `main`: census 7101's record, and `runs/lc13-k40/steps/u1.json`
and `runs/lc13-k40/replay-b01.json` under `research/jumbling/restoration/12-low-height/`.

Height-4 states (paths relative to `research/jumbling/restoration/`):

| File at `ec0f0ac` | SHA-256 | What it gives |
|---|---|---|
| `12-low-height/scripts/round22/r22d25_d19_stops.py` | `4752297cf889bd63c839b80942b019b9a3ce3da10ed92b468f2469f0e039dce5` | the six states' descriptors (start record, stop digest, prefix, checkpoint and end digests), read as literals without running the script |
| `12-low-height/README.md` | `d968ca799acd56371e0dd815dc01e17cdee18e80c8aec8fa4f21a4423fbb1fac` | the recorded counts of the six states |
| `11-scramble-census/runs/heights/ia-k3000-s7302.json` | `29b2c1b6f2e40a271a03c8eefa1ea3e3166c69d360c0ecf6683d096811d07c86` | the start of `bbef1c8e` and `2430f4cc` |
| `12-low-height/runs/round22/cloud/census-k3000-s7304/ia-k3000-s7304.json` | `dd98f72245e3bb640523608f59f614da1452d5d4e86ab2682c4710c835301716` | the start of `ac127dc3` |
| `12-low-height/runs/round22/cloud/census-k3000-s7305/ia-k3000-s7305.json` | `c8a27051f90c7596a6271ed10bfe50ead90c0d8066cbf88d43c054e9ef43caa2` | the start of `cc5ef7ec` |
| `12-low-height/runs/round22/cloud/census-k3000-s7306/ia-k3000-s7306.json` | `2a87e9462ea8319aee6da041fc15e414f43360ba8fe390ce0af8b6c0721517f4` | the start of `16c4aefe` |
| `12-low-height/runs/round22/cloud/census-k3000-s7307/ia-k3000-s7307.json` | `b974b60ae4cfdf877d1b41752fa8ad2c059fa53718c3f9e97a860432ebcf262c` | the start of `895b45de` |

## Output files

Each run's directory holds the files its script writes, with private paths
replaced: `OUT` for the scratch results directory, `INPUTS` for the extracted
inputs, `SCRATCH` for other scratch paths and `.` for the checkout. No JSON
Lines file is over 200,000 bytes, so none is compressed. The runs' standard
output and error were empty, apart from one progress line that the research
census code printed while the `words` run rebuilt census 7101's end; they are
not kept, and neither are the first `obs` and `height4` runs.

| Directory | Files (bytes) |
|---|---|
| `corrections` | `corrections-solved.jsonl` (9,361), `corrections-solved-summary.json` (429), `corrections-E3-literal.jsonl` (18,951), `corrections-E3-literal-summary.json` (424), `corrections-I1-literal.jsonl` (19,630), `corrections-I1-literal-summary.json` (424), `corrections-I2-literal.jsonl` (18,951), `corrections-I2-literal-summary.json` (424) |
| `nu` | `table-nu.json` (5,904), `cert-lb.jsonl` (10,303), `cert-lb-summary.json` (321) |
| `words` | `word-checks.jsonl` (9,521), `word-summary.json` (92) |
| `words-b06-14` | `word-checks.jsonl` (2,003), `word-summary.json` (75) |
| `obs-final` | `summary.json` (6,054) and one `observation-<state>.json` for each of the 20 states (115,825 to 292,344 each, 3,205,480 in all) |
| `height4-final` | `summary.json` (2,321) and one `observation-height4-<state>.json` for each of the six states (217,007 to 245,199 each, 1,405,958 in all) |

In all, 4,716,626 bytes in 43 files and six directories.
`words/word-summary.json` reads `"passed": false` only because R22B-34 is
unavailable (see Runs).

## Checks not run

- No Windows/DirectX, input, long-session or performance check: these are
  headless Linux runs of research code, and no time here is evidence for a
  product engine, a renderer or Windows.
- No time was measured on an idle machine.
- Acceptance item 7, case 5 (exit at the solved end with a non-solved pose
  digest): no such state exists on `main` (see the README's gaps).
- R22B-34's negative controls: not stored as words, so the word check reports
  the case unavailable.
- No product engine was measured. Every comparison here runs against the
  reference values; the candidate engine's comparisons wait for EN3 and EN4.
