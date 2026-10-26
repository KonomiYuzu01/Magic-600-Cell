# Stage 18: the tail completion (card 9)

Status: **in progress** (26 October 2026). The record of [card 9](../../theory/cards/card-9-tail-completion.md),
opened by the [owner decision of 26 October 2026](../../../../docs/wiki/decisions/owner-decisions-2026-10-26-tail-then-freeze.md)
after stage 17. Everything here is an exact computation on the research simulator, headless on Linux (synthetic
geometry; no native-host, input or performance evidence).

## 1. The question

The tutorial runner's tail ([chapter 5](../../tutorial/05-tail.md)) lands the pieces still off the lattice one unit per
step, each step one literal strict′ word. The depth 40 development run D1 (seed 7101, run 4, acceptance mode;
[`13-tutorial-runner/runs/dev/d1-acc-s7101/`](../13-tutorial-runner/runs/dev/d1-acc-s7101/)) entered the tail with
1,193 pieces off and stopped `stuck` after 17 tail steps with 158 off, all at height 2, in five units (card 9, "The
stop"). Card 9 asks what completes the tail there and what keeps a tail from sticking.

## 2. Results so far (computed)

### 2.1 The whole-cap test at the stop (card 9, T5)

`scripts/wholecap.py` is stage 12's COV-33 computation (`12-low-height/scripts/round22/cov33_whole_cap_737.py`) at any
pose table, with every cap that part A leaves open resolved by the inside-mask-empty flow of COV-10 and COV-32. At the
stop (`results/wholecap/d1-stop-158.txt`) nine caps are eligible: 353 passes unprepared (item 22, for 81223); 91, 148,
217, 548 and 560 pass prepared; 12, 324 and 439 fail under every preparation. Every unit has a passing instance. The
program reproduces COV-33's record at E2's 737 state row for row (`results/wholecap/e2-737.txt`) and finds E2's last
step, cap 91 at item 14 with a flow of 71, at E2's 71 state (`results/wholecap/e2-71.txt`).

### 2.2 The tail completed from the stop (card 9, T6)

Four whole-cap steps X·(c, i)·V′·(c, i)⁻¹·V′⁻¹, each one literal word built at its state, put every piece of the stop
on the lattice. The order was chosen by hand: a passing unprepared instance if `wholecap.py` found one, else the
passing prepared instance that moves the fewest holes.

| Step | Off | Cap, item | Lands | X (holes moved out of the cap) | V′ | Letters | End digest |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 158 → 129 | 353, 22 | 81223 | none | 43,432 | 86,866 | `1007a601…` |
| 2 | 129 → 104 | 217, 15 | 112734 | none | 224,538 | 449,078 | `5e9f262c…` |
| 3 | 104 → 71 | 560, 13 | 60085 | 9,748 (23) | 46,740 | 103,230 | `b28448f3…` |
| 4 | 71 → 0 | 91, 14 | 9013, 16314 | 21,592 (19) | 424,994 | 871,582 | `c75c8dde…` |

1,510,756 letters in all, the longest step 871,582 (the literal limit is 10⁷). `scripts/wc_prep.py` builds X:
the prepared hole set J from the flow (outside the cap, the same number of holes in every source orbit of the SAFE-ALL
group, current holes kept where the flow allows), an R22B-10 certificate for J at the state (`r22b10_cert.py --seeds
atlas`, checked by `check_r22b10.py`, expanded by `expand_slp.py`), played from the state; after X the holes are
exactly J and every off piece keeps its pose. `scripts/wc_t1.py` builds the step (stage 12's `e2_b06_t1.py` with the
cap and the unit as arguments); `tclose2.py`'s peeling and greedy searches found no literal V′ at any of the four
middles, so every V′ is an expanded R22B-10 certificate. Each step took 13 to 38 minutes of one core. The whole-cap
test at 129 and 104 off is in `results/wholecap/d1-t6-129.txt` and `d1-t6-104.txt`: after step 1, cap 217 holds
112734 alone and passes unprepared.

`scripts/replay_tail.py` replays `results/d1-tail/` from the stop step by step with `slp_apply.py --unit`, which
checks the strict′ contract of each step (the unit on the lattice, no collateral, every other off piece at its pose),
and checks every word's SHA-256 and length, every digest and count, and the off count dropping by exactly the unit's
size against `results/d1-tail/steps.json`: passed. Its gate `scripts/test_replay_tail.py` plays eleven mutations of
the record (among them an empty unit, a lattice piece in a unit, wrong counts, a removed preparation and two commuting
letters of V′ swapped) and a wrong start table, each rejected, and the unmutated record, accepted: passed.

### 2.3 The lattice phase from the tail's end

From T6's end table (`c75c8dde…`, every piece on the lattice), the runner in development mode (`runner.py --table`,
restorer `schedule11`, [chapter 6](../../tutorial/06-lattice.md)) went straight to the lattice phase and ended
`solved`: 53 lattice steps, 13,231,266 twists, 2,844 s (`results/lattice-d1/`). `check_run.py` passes the record
(verdict `solved`, every step passed; the same summary form as the committed D0 runs).

So D1's scramble (depth 40, seed 7101) is restored to identity labels by the runner's 114 steps to the stop, T6's four
steps and the runner's lattice phase. This is not a runner run: T6's steps are not runner rules, and their order is the
integrator's.

## 3. Files

- `scripts/wholecap.py`, `wc_prep.py`, `wc_t1.py`, `replay_tail.py` and its gate `test_replay_tail.py` (research
  scripts; each docstring gives its use).
- `results/stop-units/`: the five units of the stop (`mkblocks2.py` on the stop table), named by key.
- `results/wholecap/`: the whole-cap outputs at the stop, at 129 and 104 off, and at E2's 737 and 71 states.
- `results/d1-tail/`: T6's four steps (`x.json.gz`, `w.json`, `v.json.gz`, `winv.json`, `unit.json` each) and
  `steps.json`.
- `results/lattice-d1/`: `run/`, the runner record of the lattice phase from T6's end (without the restorer's scratch
  `attempts/`, as in the committed development runs), `runner.out` and `check.jsonl` (`check_run.py`: verdict
  `solved`, no fail).

## 4. Reproduction

From the repository root, single-threaded (`OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1`), with
`Z=research/jumbling/restoration/13-tutorial-runner/runs/dev/d1-acc-s7101/out4/tables/00113.json` and `S=research/
jumbling/restoration/18-tail-completion`, every output outside the repository:

- T5: `python3 -I -B -u $S/scripts/wholecap.py $Z $S/results/stop-units/*.json` (about 2.5 CPU minutes);
- T6: `python3 -I -B $S/scripts/replay_tail.py $Z $S/results/d1-tail <new dir>` (about 10 CPU minutes), and its
  gate `python3 -I -B $S/scripts/test_replay_tail.py $Z $S/results/d1-tail <new dir> --full` (about 18 minutes);
- the lattice phase: `python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/runner.py --table <new
  dir>/04-91-14.json --out <new run dir>` (about 50 minutes), then `check_run.py <new run dir>`.
