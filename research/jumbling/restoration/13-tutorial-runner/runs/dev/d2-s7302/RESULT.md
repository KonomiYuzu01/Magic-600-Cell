# Development line d2-s7302: D2, the 3000-twist stop of seed 7302

Status: finished by the integrator's instruction (22 October 2026, 21:05 UTC), with a partial record. The component
run did not end. Its last checkpoint is the record reported here.

## Question

PLAN.md section 4, D2, seed 7302: what does the tutorial runner (`runner-run/3`) do from the census stop of seed 7302
(3000 twists, start `015d839e…`, 100,584 off, (n2, n4, n6) = (99,314, 1,085, 185)) in a component run with the bulk
skipped? Does it follow the first steps that stage 12 found with its separate scripts?

The full run (step 3 of the line) and the reproduction (step 4) were dropped by the integrator's line update of
22 October, 13:37 UTC. The component runs at the stops of 7304, 7305 and 7307 had shown the high phase's kernel
exhausted at the stop.

Conventions:
- Every command ran from the repository root with `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1`.
- `S=research/jumbling/restoration/13-tutorial-runner/scripts` and `L=research/jumbling/restoration/12-low-height/scripts`.
- `<dir>` is a scratch directory outside the repository.
- Times are cloud timings of research code on a shared headless Linux machine (4 cores), not performance evidence.

## Step 1: start table

On commit `35c3d4f`:

    python3 -I -B -u $L/export_table.py research/jumbling/restoration/11-scramble-census/runs/heights/ia-k3000-s7302.json <dir>/tables/s7302-stop.json

Output: `{'record': 'ia-k3000-s7302.json', 'run': 'ia-k3000-s7302', 'moves': 313}
015d839ecbb37bbdde74ae9391d5d7f274dea7fee4b4fdf0092d224eef907a1f 100584`.
- The digest and the off count match the expected values.
- Wall time was about 95 minutes (04:03 to 05:38 UTC), against the plan's estimate of about 1,650 s.
- The table is not committed. The run keeps it as `component-checkpoint/start-table.json`.

## Step 2: component run

### Attempts

1. **05:39 UTC, commit `35c3d4f`, `--out <dir>/out-c --cache <dir>/cache`.**
   - Lost to a container restart at about 09:03 UTC, which killed the runner with no signal handler run.
   - It had played the same first 17 steps as attempt 3 below (`run.json` at 07:39 and 06:20).
   - It was in the `two_letter_u` pass of step 17 (begun 06:25) when it died, after about 2 h 38 min of wall time.
   - Not committed.
2. **13:38 UTC, `--out <dir>/out-c2 --cache <dir>/cache2`.**
   - Stopped by PID with SIGTERM after about one minute, before its first step, when the integrator's update
     (resume and checkpoints) arrived.
3. **13:39 UTC, on the merge `40ac3fd` of `04057a4` (`claude/repair-pair-list`: runner `--resume`,
   `scripts/checkpoint.py`).** `config.source` is that commit, with `dirty` false. Commands:

       python3 -I -B -u $S/runner.py --table <dir>/tables/s7302-stop.json --out research/jumbling/restoration/13-tutorial-runner/runs/dev/d2-s7302/component --cache <dir>/cache3 --bulk-steps 0 --height-seconds 12000 --height-cpu 12000 --pair-shards 3 --kernel-seconds 3600
       python3 -I -B $S/checkpoint.py --run research/jumbling/restoration/13-tutorial-runner/runs/dev/d2-s7302/component --pid <runner pid> --trailer ... --trailer ...

   - The checkpoint helper pushed checkpoints through step 22. The last one is `d225cff`, at 17:25 UTC.
   - At 17:24 the runner started the high phase's kernel call (`logs/00022-high-kernel.log`, empty).
   - The container then stopped. It came back with a fresh boot at 21:02 UTC, with no runner process and no further
     write after 17:24.
   - The integrator stopped the line at 21:05 UTC. The run was not resumed. The uncommitted live directory was moved
     out of the repository unchanged, and `component-checkpoint/` is kept unchanged as the partial record.

### The partial record (`component-checkpoint/run.json`, step 22)

- **Record state:**
  - `result` `budget`: the value the record holds while the run is in progress, not a final result.
  - `acceptance.verdict` null; `config.mode` `development`; `config.component` `bulk_budget`; `resumes` none.
  - `totals.seconds` 13,457 (about 3 h 44 min, 13:39 to 17:24 UTC).
- **Phase table:**

| Phase | Result | Steps | Twists | Start | End |
|---|---|---|---|---|---|
| bulk | `budget` (`--bulk-steps 0`) | 0 | 0 | — | `015d839e…`, 100,584 off, (99,314, 1,085, 185) |
| height | `stuck` | 22 | 25 | `015d839e…`, 100,584 off, (99,314, 1,085, 185) | `2430f4cc…`, 103,484 off, (102,750, 734, 0) |
| · r22d75 | `stuck` (`no_step`, attempt 59.8 s) | 7 | 7 | `015d839e…` | `1ff0061f…`, 104,294 off, (103,103, 1,019, 172) |
| · r22d81 | `handoff` | 5 | 5 | `1ff0061f…` | `bbef1c8e…`, 103,896 off, (102,960, 936, 0) |
| · h4 | `stuck` (`pair_exhausted`) | 10 | 13 | `bbef1c8e…` | `2430f4cc…`, 103,484 off, (102,750, 734, 0) |
| high | not finished (`null`); the kernel call began at 17:24 | 0 | 0 | — | — |
| tail | not reached | 0 | 0 | — | — |
| lattice | not reached | 0 | — | — | — |

Notes on the table:
- (n2, n4, n6) are the counts of off pieces at left height 2, 4 and 6.
- h4's last call `00022` was stuck. Its `h4` attempt took 116.9 s, and its `two_letter_u` pass ran in three shards of
  1,704, 1,767 and 1,739 s. All ended `done` with no pair, hence `pair_exhausted`.

### Expected and actual first steps

| Expected (PLAN.md section 4, D2) | Actual |
|---|---|
| `r22d75` 7 letters → `1ff0061f…` | 7 one-letter steps (T0, T1, T1, T1, T2, T2, T2) → `1ff0061f…`, then `stuck` |
| `r22d81` (124,2) U, (274,13) H6, (46,1) U, (46,3) U, (104,14) H6 → hand-over `bbef1c8e…`, (n2, n4, n6) = (102,960, 936, 0) | U, H6, U, U, H6 → `bbef1c8e…`, `handoff`, (102,960, 936, 0) |
| `h4` (new), then the high phase with the kernel; `stuck` expected at `bbef1c8e…` | `h4` played 10 steps (13 twists) to `2430f4cc…` with n4 936 → 734, then `pair_exhausted`. The high phase started at `2430f4cc…`, not at `bbef1c8e…`, and did not finish |

The r22d81 letters themselves were not compared letter by letter with the plan's list; only the rules and the end
digest were.

### Steps

| # | Sub-phase | Rule | Letters | Before → after | Off after | (n2, n4, n6) after | Step s | Search attempts (s) |
|---|---|---|---|---|---|---|---|---|
| 0 | r22d75 | T0 | 1 | `015d839e` → `446f4e00` | 101,205 | (99,935, 1,098, 172) | 2.2 | |
| 1 | r22d75 | T1 | 1 | → `cc5fa964` | 102,062 | (100,797, 1,093, 172) | 2.2 | |
| 2 | r22d75 | T1 | 1 | → `3a4c0a56` | 102,999 | (101,801, 1,026, 172) | 2.1 | |
| 3 | r22d75 | T1 | 1 | → `0251a333` | 104,716 | (103,525, 1,019, 172) | 1.9 | |
| 4 | r22d75 | T2 | 1 | → `fd7a61a6` | 104,495 | (103,304, 1,019, 172) | 2.1 | |
| 5 | r22d75 | T2 | 1 | → `4f58b063` | 104,337 | (103,146, 1,019, 172) | 2.0 | |
| 6 | r22d75 | T2 | 1 | → `1ff0061f` | 104,294 | (103,103, 1,019, 172) | 1.9 | |
| 7 | r22d81 | U | 1 | → `9a227291` | 104,294 | (103,103, 1,019, 172) | 2.2 | |
| 8 | r22d81 | H6 | 1 | → `c9cefc31` | 103,532 | (102,593, 926, 13) | 2.2 | |
| 9 | r22d81 | U | 1 | → `7465613d` | 103,532 | (102,593, 926, 13) | 1.9 | |
| 10 | r22d81 | U | 1 | → `198e936f` | 103,532 | (102,593, 926, 13) | 1.7 | |
| 11 | r22d81 | H6 | 1 | → `bbef1c8e` | 103,896 | (102,960, 936, 0) | 1.8 | |
| 12 | h4 | U | 1 | → `9d647ad8` | 103,896 | (102,960, 936, 0) | 1.9 | h4 106 |
| 13 | h4 | U | 1 | → `2e1678eb` | 103,896 | (102,960, 936, 0) | 2.4 | h4 111 |
| 14 | h4 | U | 1 | → `5bae83cb` | 103,896 | (102,960, 936, 0) | 2.0 | h4 135 |
| 15 | h4 | U | 1 | → `e9065103` | 104,566 | (103,630, 936, 0) | 1.9 | h4 142 |
| 16 | h4 | L2 | 1 | → `6999b921` | 104,507 | (103,571, 936, 0) | 1.9 | h4 139 |
| 17 | h4 | pair | 2 | → `391490f7` | 104,218 | (103,282, 936, 0) | 2.7 | h4 168; pair shards 5,918 / 6,015 / 5,929 |
| 18 | h4 | L4 | 1 | → `c312a2fc` | 103,484 | (102,750, 734, 0) | 2.8 | h4 71 |
| 19 | h4 | pair | 2 | → `6c7a79d9` | 103,484 | (102,750, 734, 0) | 2.3 | h4 74; pair shards 1,482 / 1,495 / 1,458 |
| 20 | h4 | U | 1 | → `b2651c24` | 103,484 | (102,750, 734, 0) | 2.2 | h4 108 |
| 21 | h4 | pair | 2 | → `2430f4cc` | 103,484 | (102,750, 734, 0) | 2.6 | h4 110; pair shards 1,846 / 1,944 / 1,936 |

The rows of steps 12 to 21 are the step attempts of the h4 plugin call that proposed each step (`attempts` in
`run.json`). "Step s" is the step's own recorded time.

**The h4 pair steps**, from the step rows:
- **Step 17:** word (541, 2)(593, 54), class A, chosen `first_success` at order index 119 of 2,065 first letters,
  three shards, nothing freed. τ 482 → 482, u 1 → 1 → 0.
- **Step 19:** word (553, 5)(535, 3), class A, chosen `freeing` (frees cap 535) at order index 299 of 1,947 first
  letters. τ 597 → 597, u 41 → 41 → 21.
- **Step 21 (the last step):** word (363, 10)(535, 8), class B, chosen `first_success` at order index 1,543 of 1,947
  first letters, `freed` null. τ 597 → 597, u 10 → 10 → 8.
- After step 21, call `00022` found no single-letter step and no pair: `pair_exhausted`.

## Checks not run and open items

- `check_run.py` was not run. The run never ended, and the checkpoint is not a terminal record.
- The high phase at `2430f4cc…` (734 height-4 pieces) did not finish. The kernel's result there (step, `stuck`,
  `budget` or `timeout`) is unknown. The tail and the lattice phase were not reached.
- Step 3 (full run, six-hour bulk) and step 4 (reproduction of the census bulk from `--scramble 3000 7302`) were not
  run; the integrator dropped them.
- No Windows, DirectX, native host, input, long-session or performance check. The headless Linux runs above are the
  only evidence.

## Files

- `component-checkpoint/`: the checkpoint of attempt 3 at step 22, written by `checkpoint.py`. It holds `run.json`,
  `start-table.json`, `words/`, `step-trees/` and `tables/`. It is kept unchanged as the partial record. The checkpoint
  helper omits logs and attempt work directories.
