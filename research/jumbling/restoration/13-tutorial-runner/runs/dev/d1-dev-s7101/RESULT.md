# D1, seed 7101 at depth 40, development mode (line d1-dev-s7101)

Status: **closed, interrupted** (22 October 2026). Run 1 was killed by a container restart in the tail phase, after 97
bulk steps and 1 tail step. Its partial record is kept in `run1-interrupted/`. The integrator then closed D1 in
development mode, since the acceptance-mode line for seed 7101 answers depth 40. Run 2, started at the same time as the
closing notice, was stopped by PID after about one minute. Nothing of run 2 is kept. `check_run.py` was not run.

## Question

What does the tutorial runner do on the scramble of depth 40 with seed 7101 in development mode (`PLAN.md`, section 4,
D1)? Do its bulk steps agree with the census record `11-scramble-census/runs/round2/ia-k40-s7101.json`?

## Command

Run 1 ran from the repository root at commit `35c3d4f`, detached. The record gives `config.source.commit`
`35c3d4f6ef1693fa19f3d0f155cd1cba8c43870b` and `dirty` false.

    S=research/jumbling/restoration/13-tutorial-runner/scripts
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/runner.py --scramble 40 7101 \
      --out <dir>/run --cache <dir>/cache --bulk-step-seconds 21600 --height-seconds 12000 --height-cpu 12000 \
      --pair-shards 3 --kernel-seconds 3600 > <dir>/runner.out 2>&1; echo $? > <dir>/runner.rc

- It started at 04:03 UTC.
- Mode `development` (the default). Lattice restorer `schedule11` (the default). `literal_limit` 10,000,000, seeds
  `atlas`, `pair_shards` 3.
- Run 2 used the same command in fresh directories. It started at 13:37 UTC and was stopped at 13:39 UTC, still in its
  first bulk steps.

## Result of run 1 (interrupted)

- `result`: none final. The runner was killed while it was writing the tail. `run.json` as left on disk reads `result`
  `budget` and the tail `result` null; those are interim values, not a verdict. `acceptance.verdict` null.
  `config.component` null.
- No exit code. The container restarted some time after 09:03 UTC, the time of the last line in the run's logs, and
  the runner has no resume option.
- Recorded totals: 98 steps; `seconds` 16,992.4 at the last write; bulk 223 twists, tail 60 twists. `certificate_steps`
  0, `fallback_steps` 0, `timed_out_attempts` 0.

| Phase | Result | Steps | Twists | Start | End |
|---|---|---|---|---|---|
| bulk | `stuck` | 97 | 223 | `be08cfaa…`, 90,441 off, n2 = 90,441 | `7e128820…`, 1,193 off, n2 = 1,193 |
| height (`r22d75`, `r22d81`, `h4`) | `not_applicable` | 0 | 0 | `7e128820…` | `7e128820…` |
| high | `not_applicable` | 0 | 0 | `7e128820…` | `7e128820…` |
| tail | not finished | 1 recorded | 60 | `7e128820…`, 1,193 off | `cc41acca…`, 1,082 off (after step 97) |
| lattice | not reached | | | | |

Full digests:

- start `be08cfaa04612751a5852bc79aec6f20a8f86634f2279e9a6ba1147041bfbf0a`, 90,441 off, all at height 2
  (n2, n4, n6) = (90,441, 0, 0);
- bulk end `7e128820541abc82e45c683db92f38413acba6e282c00f3185585e507b4e6818`, 1,193 off, (1,193, 0, 0);
- after tail step 97 `cc41accae7cac3add5ab1c3b734b5ba7bee72c1008ec666caf27a1150fdb9938`, 1,082 off, (1,082, 0, 0).

Bulk:

- The search for a 98th bulk step began at 7,518 s and found none.
- The bulk ended `stuck`, with no timed-out attempt.
- Height and high were `not_applicable`, as the plan expected, since every off piece is at height 2.

Tail:

- Step index 97 is `zero_bad` on unit 2198 (111 pieces).
- It has 60 letters, `standing` `rule`, contract `strict_prime` and gate `stepchk`, and took 1,486.8 s.
- It lands all 111 pieces with 0 others landed.
- Step index 98 (unit 60011) was in its transport pair passes when the run was killed. The last log line is
  `{"P": 3, "entry_pair": [149, 234], "updates": 454, "sec": 980}`.

No certificate step was reached, so there is no closure length to report.

## Bulk comparison with the census record (first 97 steps)

The census record holds 97 steps and 223 twists (`moves`). Run 1's 97 bulk steps agree with it exactly, with 0
mismatches. For every step these are the same:

- the rule;
- the number of twists;
- the twists themselves (the step's word against the next slice of the census `moves`);
- N before and after;
- off after.

Further details:

- Rules: 27 `seam`, 7 `cover`, 63 `conj`; 223 twists in all.
- N is 1,736 after index 95 and 1,734 after index 96. 1,193 pieces are off after index 96, as in the census.
- The 97 steps took 7,395 s of step time. The census reached its 97th step at 3,610 s.
- The step-by-step table (rule, twists, N and off after, the runner's time) is in `bulk-comparison.md`.
- The runner's bulk then ends `stuck` where the census record stops (`running`, killed). The census never searched past
  this point for as long, so this is no comparison of the end.

This shows agreement with one earlier run over 97 steps, nothing more (`PLAN.md`, section 4). No difference was seen,
so there is no lead for R22D-90.

## Files

- `run1-interrupted/`: run 1's output directory as the restart left it (`run.json`, words, units, step trees, tables,
  attempts, logs, `bulk-progress.json`). No file is over 20 MB, and nothing is gzipped by this line. In
  `logs/attempts-00097-units-units.log` and `logs/attempts-00098-units-units.log` the absolute run-directory prefix was
  replaced by `<dir>`. No other file was changed.
- `run1-interrupted-runner.out`: run 1's stdout and stderr, which are empty.
- `bulk-comparison.md`: the 97-step comparison table.

## Not run or not finished

- Run 1 did not finish: the rest of the tail and the lattice phase were not reached.
- Run 2 was stopped after the integrator closed the line. It has no record.
- `check_run.py` was not run. There is no finished record, and the line was closed.
- Windows, DirectX, the native host, input, long-session and performance checks: none. Times are cloud timings of
  research code on a shared headless Linux machine, not performance evidence.
