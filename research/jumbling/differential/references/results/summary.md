# Jumbling reference values: results (EN2 slice 4)

Status: draft. Runs marked PENDING are still running; the other results were
measured on 22 October 2026 for packet ENG-0J revision 2, acceptance items 3
(its remaining refusals and shown cases) and 8, with amendment A5
(`docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md`). The
modules and their commands are in [`../README.md`](../README.md). Acceptance
item 7, the optional-state round trip, is covered by the bounded tests of
`tests/test_jumbling_session.py`; it has no full-mode run.

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

PENDING

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
| E3 | literal: every twist replayed on the state | PENDING | 23,932 | PENDING |
| I1 | literal | PENDING | 23,932 | PENDING |
| I2 | literal | PENDING | 23,932 | PENDING |

From solved a row's time follows the correction's length: 0.2 to 0.9 s for
the corrections of 5 and 9 letters, 94 s for correction 35 (5,629 letters).

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

An observation took 18 s at the median per route (14 to 61 s, the deep
scramble the slowest). Rebuilding the states took 960 s of the run's 2,327 s,
the deep scramble 172 s and the ends of the fixtures 147 to 228 s.

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

PENDING

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
| b06-14 | b15's end | 672,417 | the last line of `b06-14-apply.log` | 11 (the log records no letter count) | PENDING | PENDING |
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

Both shown cases are results, not refusals: the word check reports the false
step flag and the lifted pieces, and whether a commit is allowed follows from
admissibility and protection alone (DT-J1 A7). The commit side is in the
bounded tests: an admissible word that lifts lattice pieces off is shown and
then committed from its preview
(`test_admissible_collateral_is_shown_and_committed_from_preview` in
`tests/test_jumbling_word_check.py`).

## Runs

PENDING

## Provenance of the runs

PENDING

## Inputs outside this repository

PENDING

## Output files

PENDING

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
