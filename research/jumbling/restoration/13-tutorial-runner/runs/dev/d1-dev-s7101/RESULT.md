# D1, seed 7101 at depth 40, development mode (line d1-dev-s7101)

Status: **in progress** (progress notes, 22 October 2026). Run 1 was killed by a container restart in the tail phase.
Run 2 (same command) is running; see "Progress". Nothing has been checked yet.

## Question

What does the tutorial runner do on the scramble of depth 40 with seed 7101 in development mode (`PLAN.md`, section 4,
D1)? Do its bulk steps agree with the census record `11-scramble-census/runs/round2/ia-k40-s7101.json`?

## Command

Run from the repository root at commit `35c3d4f` (record `config.source.commit`
`35c3d4f6ef1693fa19f3d0f155cd1cba8c43870b`, `dirty` false), detached:

    S=research/jumbling/restoration/13-tutorial-runner/scripts
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/runner.py --scramble 40 7101 \
      --out <dir>/run --cache <dir>/cache --bulk-step-seconds 21600 --height-seconds 12000 --height-cpu 12000 \
      --pair-shards 3 --kernel-seconds 3600 > <dir>/runner.out 2>&1; echo $? > <dir>/runner.rc

Started at 04:03 UTC. Mode `development` (the default). Lattice restorer `schedule11` (the default).

## Start

Digest `be08cfaa04612751a5852bc79aec6f20a8f86634f2279e9a6ba1147041bfbf0a`, 90,441 pieces off, all at height 2.

## Bulk comparison with the census record (first 97 steps)

The census record holds 97 steps and 223 twists (`moves`). The runner's first 97 bulk steps agree with it exactly. For
every step the rule, the number of twists, the twists themselves (the step's word against the next slice of the
census `moves`), N before and after, and off after are the same. There are 0 mismatches.

- Rules: 27 `seam`, 7 `cover`, 63 `conj`; 223 twists in all.
- N is 1,736 after index 95 and 1,734 after index 96. 1,193 pieces are off after index 96, all at height 2.
- The state after index 96 has digest `7e128820541abc82e45c683db92f38413acba6e282c00f3185585e507b4e6818`.
- The 97 steps took 7,395 s of step time (the census reached its 97th step at 3,610 s).
- The step-by-step table, with the runner's time for each step, is in `bulk-comparison.md`.

This shows agreement with one earlier run over 97 steps, nothing more (`PLAN.md`, section 4).

## Progress

Third note, 13:38 UTC: **run 1 was interrupted, and run 2 has started.**

- The cloud container restarted some time after 09:03 UTC, the time of the last line in run 1's logs. The restart
  killed run 1, so it left no exit code. The runner has no resume option.
- Run 1 had reached tail step index 98: unit 60011, transport pair passes, last log line
  `{"P": 3, "entry_pair": [149, 234], "updates": 454, "sec": 980}`. Its 98 recorded steps (97 bulk, 1 tail) are as
  described below. Its record as left on disk is in `run1-interrupted/`: `run.json` reads `result` `budget` with
  tail `result` null, because the runner never finished writing it. The runner's stdout, which was empty, is in
  `run1-interrupted-runner.out`.
- In run 1's copied logs `attempts-00097-units-units.log` and `attempts-00098-units-units.log`, the absolute run
  directory prefix was replaced by `<dir>`. No other file was changed.
- At 13:37 UTC run 2 started: the same command at the same commit, in fresh `--out` and `--cache` directories.

Second note, 08:50 UTC (about 17,200 s after the start):

- The bulk phase ended `stuck` after the 97 steps above (223 twists), at `7e128820…` with 1,193 off. No step index 97
  was found within the step clock.
- Height (all three subphases) and high are `not_applicable` (all 1,193 off pieces are at height 2), as the plan
  expected.
- The tail is running. Step 97 is `zero_bad` on unit 2198 (111 pieces): 60 letters, `standing` `rule`, contract
  `strict_prime`, gate `stepchk`, 1,486.8 s. It lands all 111 pieces with no collateral. Off goes from 1,193 to 1,082
  (all at height 2), and the digest becomes `cc41accae7cac3add5ab1c3b734b5ba7bee72c1008ec666caf27a1150fdb9938`.
- Step 98 is being searched (unit 60011, transport pair passes; `logs/attempts-00098-u60011-transport.log`).

First note, 06:50 UTC, about 9,980 s after the start, `bulk-progress.json` reads `{"step_started": 7517.779213569}`. Step index
97 has been searched for about 2,460 s of its 21,600 s step clock. `logs/bulk.log` is empty, and `runner.out` has no
output yet.

## Not yet run or finished

- The rest of the run (bulk after index 96, height, high, tail, lattice) and `check_run.py`.
- Windows, DirectX, the native host, input, long-session and performance checks: none. Times are cloud timings of
  research code on a shared headless Linux machine, not performance evidence.

Next step: wait for `<dir>/runner.rc`, copy the run directory here, write the results, then run `check_run.py`
detached.
