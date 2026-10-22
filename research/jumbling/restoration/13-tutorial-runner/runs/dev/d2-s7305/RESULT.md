# D2, seed 7305: the runner from the 3000-twist stop (development line d2-s7305)

Status: finished. Step 1 done; step 2 (component run) done and checked, and it is this line's D2 result for seed 7305;
step 3 (full run) was stopped with SIGTERM on the integrator's instruction (13:37 UTC) and is kept as an interrupted
record. Times are cloud timings of research code on a shared headless Linux machine, not performance evidence.

**Summary.**

- Start `273eab69…`, 105,437 off, (n2, n4, n6) = (103,140, 2,297, 0), as expected.
- Component run: `r22d75` plays exactly the expected 11 letters to `cc5ef7ec…` (n4 = 1,419) and is stuck; `r22d81` is
  `not_applicable`; `h4` is `stuck` (`pair_exhausted`) with no step; the high phase is `stuck` (`kernel_exhausted`)
  with no step; the tail's unit listing exceeds `rule_seconds` (3,600 s) and the run ends `error`. Verdict `error`,
  `config.component` `bulk_budget`. `check_run.py` passes it (end `cc5ef7ec…`, 0 unchecked contracts).
- Full run (interrupted): the bulk ends `timeout` on its 21,600 s clock after 80 steps (79 `conj`, 1 `cover`) and 238
  twists, N 131,542 → 126,592, off 105,437 → 101,937, at `736ea9e7…` with n4 unchanged at 2,297. `r22d75` had played
  4 steps when the run was stopped. `check_run.py` fails this record with `phase_steps` and `phase_twists` only: the
  runner left `phases.height` at 0 steps and 0 twists while its sub-phase `r22d75` records 4 (see step 3). Every one of
  the 84 steps replays and passes.

## Question

Starting from the 3000-twist census stop of seed 7305 (`12-low-height/runs/round22/cloud/census-k3000-s7305/`), what
does the tutorial runner (`runner-run/3`, development mode) do: first as a component run without the bulk
(`--bulk-steps 0`), then as a full run with the bulk on (`PLAN.md`, section 4, "D2")?

## Commit

All commands ran on commit `35c3d4f` (branch `claude/repair-pair-list`, checked out as `claude/dev-d2-s7305`), from
the repository root, each with `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1` in front.
`S=research/jumbling/restoration/13-tutorial-runner/scripts`, `L=research/jumbling/restoration/12-low-height/scripts`.
The component record's `config.source` is commit `35c3d4f6…`, `dirty` false.

## Step 1: start table (done)

    python3 -I -B -u $L/export_table.py research/jumbling/restoration/12-low-height/runs/round22/cloud/census-k3000-s7305/ia-k3000-s7305.json <dir>/tables/s7305-stop.json

Output (`export.out`): 103 moves, digest `273eab69dee9e3a2f4a3997a75f6b8d9ad16814fa26b941bf7a219247f9bab6d`, 105,437
off. Both match the expected values. Wall time about 2,300 s (started 04:04 UTC, finished between 04:45 and 05:12 UTC).

## Step 2: component run (done, `component/`)

    python3 -I -B -u $S/runner.py --table <dir>/tables/s7305-stop.json --out <dir>/out-c --cache <dir>/cache --bulk-steps 0 --height-seconds 12000 --height-cpu 12000 --pair-shards 3 --kernel-seconds 3600
    python3 -I -B -u $S/check_run.py <dir>/out-c --out <dir>/chk-c/check.jsonl

The whole output directory is in `component/`, with `runner.out`, `check.jsonl` and `check.out`. No file is over
20 MB (the directory is 19 MB), so nothing is gzipped beyond what the runner wrote. In `component/logs/runner.log`
(a child log, not checked by the checker) the absolute repository path was replaced by `<repo>`, as README section 2
asks for child logs before publishing. `run.json` and the other records are unchanged.

**Outcome.**

- `result` `error`, `error` "rule subprocess exceeded rule_seconds"; `acceptance.verdict` `error`
  (certificate steps 0, fallback steps 0, probe null, timed-out attempts 0); `config.component` `bulk_budget`.
- 11 steps, 11 twists; runner time 8,206.8 s (05:13 to 07:30 UTC).
- Budgets as recorded: `rule_seconds` 3,600 (the default; the line's command does not set it), `height_seconds` and
  `height_cpu` 12,000, `kernel_seconds` 3,600, `pair_shards` 3.

**The error.** The tail started at 06:29:54 UTC. Its first action, listing the units (`runner.py:1270`, `units()`), runs
a child, and that child was still running when `rule_seconds` (3,600 s) ran out. The runner raised
`RuleFailure('timeout', 'rule subprocess exceeded rule_seconds')` (`runner.py:562`, traceback in
`component/logs/runner.log`) and ended the run `error` at 07:29:54 UTC. The units child wrote no log
(`logs/attempts-00011-units-units.log` is empty). The tail's `result` stays null and no tail step was played; the
lattice phase was not reached. As the line's rules require, the runner was not changed and the run was not repeated
with another budget.

**Phases** (n2, n4, n6 from the record's `heights`; n6 = 0 throughout):

| Phase | Result | Steps | Twists | Start | End | Wall time (log stamps) |
|---|---|---|---|---|---|---|
| bulk | `budget` (`--bulk-steps 0`) | 0 | 0 | `273eab69…`, 105,437 off | the same | — |
| height | `stuck` | 11 | 11 | `273eab69…`, 105,437 off, (103,140, 2,297, 0) | `cc5ef7ec…`, 109,872 off, (108,453, 1,419, 0) | 05:13-05:37 |
| — `r22d75` | `stuck` (`no_step`) | 11 | 11 | `273eab69…` | `cc5ef7ec…` | 05:13-05:25 |
| — `r22d81` | `not_applicable` | 0 | 0 | `cc5ef7ec…` | `cc5ef7ec…` | — |
| — `h4` | `stuck` (`pair_exhausted`) | 0 | 0 | `cc5ef7ec…` | `cc5ef7ec…` | 05:25-05:37 |
| high | `stuck` (`kernel_exhausted`) | 0 | 0 | `cc5ef7ec…` | `cc5ef7ec…` | 05:37-06:29 |
| tail | null (cut off by the error) | 0 | 0 | `cc5ef7ec…` | — | 06:29-07:29 |
| lattice | null (not reached) | 0 | — | — | — | — |

The high phase's `details`: kernel result `stuck`, catalogue `r22d25-cat/1`; the ledger shows F1, F2, F3, d19, S, ST
and T1 all `exhausted`.

**Expected and actual first steps** (`PLAN.md`, section 4, D2 table, seed 7305):

| | Expected | Actual |
|---|---|---|
| start | `273eab69`, 105,437 off, n6 = 0 | `273eab69…`, 105,437 off, (103,140, 2,297, 0) |
| `r22d75` | 11 letters → `cc5ef7ec` (n4 = 1,419), stuck | 11 one-letter steps → `cc5ef7ec…` (n4 = 1,419), `stuck` (`no_step`) |
| `r22d81` | not applicable (n6 = 0) | `not_applicable` |
| `h4` | new | `stuck` (`pair_exhausted`), no step |
| high | `stuck` expected (as stage 12's kernel run at this stop) | `stuck` (`kernel_exhausted`), no step |
| tail | follows | entered; unit listing exceeded `rule_seconds` (3,600 s); run `error` |
| lattice | if the tail reaches it | not reached |

**Steps** (all in the height phase, sub-phase `r22d75`, one letter each; time is the step's `seconds`):

| # | Rule | Letters | Time (s) | After | Off after | (n2, n4) after |
|---|---|---|---|---|---|---|
| 0 | `r22d75/T1` | 1 | 2.1 | `2e48ac8a…` | 106,061 | (103,776, 2,285) |
| 1 | `r22d75/T1` | 1 | 1.9 | `b5a40004…` | 106,187 | (104,001, 2,186) |
| 2 | `r22d75/T1` | 1 | 1.9 | `b55863d1…` | 106,839 | (104,709, 2,130) |
| 3 | `r22d75/T1` | 1 | 2.1 | `ae2a0109…` | 107,579 | (105,613, 1,966) |
| 4 | `r22d75/T1` | 1 | 1.9 | `b2b54ca1…` | 108,181 | (106,326, 1,855) |
| 5 | `r22d75/T1` | 1 | 1.9 | `d8a9f6da…` | 108,607 | (107,004, 1,603) |
| 6 | `r22d75/T1` | 1 | 2.2 | `7c44c0d8…` | 108,669 | (107,080, 1,589) |
| 7 | `r22d75/T1` | 1 | 1.8 | `662d793e…` | 109,267 | (107,815, 1,452) |
| 8 | `r22d75/T1` | 1 | 1.9 | `5c7f89ec…` | 110,259 | (108,840, 1,419) |
| 9 | `r22d75/T2` | 1 | 1.9 | `491b45c8…` | 109,881 | (108,462, 1,419) |
| 10 | `r22d75/T2` | 1 | 1.9 | `cc5ef7ec…` | 109,872 | (108,453, 1,419) |

The step times are the recorded play times; each `r22d75` attempt took about one minute of wall time (log stamps
05:14 to 05:25).

**Check.** `check_run.py` passes the record: `pass` true, verdict `error`, end digest
`cc5ef7ecf0510ea9a74fc52bafe324d6d8f58294c4b0e20b40b97b414dada062`, 0 unchecked contracts, 0 fails, all 11 steps pass,
`lattice_checked` false. About 25 s (07:30:51 to 07:31:16 UTC).

## Step 3: full run (stopped by SIGTERM, `full-interrupted/`)

    python3 -I -B -u $S/runner.py --table <dir>/tables/s7305-stop.json --out <dir>/out-f --cache <dir>/cache --height-seconds 12000 --height-cpu 12000 --pair-shards 3 --kernel-seconds 3600 --bulk-seconds 21600 --bulk-step-seconds 21600

Started 07:30:49 UTC, after the component runner had finished. It shares the cache directory with the component run.
At 07:33 UTC it is in its first bulk step (no step recorded yet).

Progress note at 09:32 UTC: 22 bulk steps recorded (21 `conj`, 1 `cover`), 64 twists, 7,202 s of step time. N went
from 131,542 at the start to 130,232; off from 105,437 to 105,144. The last five steps: `conj` 3 letters each, 438,
331, 313, 500 and 325 s, N 130,444 → 130,232. `bulk-progress.json`: `{"step_started": 19721.4}` (a step is running).
`logs/bulk.log` is empty so far.

Progress note at 11:33 UTC: 46 bulk steps (45 `conj`, 1 `cover`), 136 twists, 14,222 s of step time; N 128,758 and
103,780 off after the last step (the last three: `conj` 3 letters, 407, 346 and 332 s, N 128,924 → 128,758).
`bulk-progress.json`: `{"step_started": 26759.9}`. `logs/bulk.log` still empty. The bulk clock (21,600 s) ends about
13:31 UTC.

Progress note at 13:33 UTC: the bulk ended `timeout` on its clock after 80 steps (79 `conj`, 1 `cover`), 238 twists,
at `736ea9e7…`, N 126,592, 101,937 off, (n2, n4, n6) = (99,640, 2,297, 0). The last bulk step: `conj`, 3 letters,
620 s. The height phase started at 13:32 UTC (`logs/00080-r22d75-r22d75.log`). `bulk-progress.json` still reads
`{"step_started": 33760.8}`.

**Stopped.** At 13:37 UTC the integrator dropped the full run (the component record is the D2 result for this seed;
a full run only puts a six-hour bulk in front of the same phases). The runner (one process) was sent SIGTERM by its
process id (`kill -TERM <pid>`) at 13:37:37 UTC; it wrote `runner.rc` 2 within two seconds. Then:

    python3 -I -B -u $S/check_run.py <dir>/out-f --out <dir>/chk-f/check.jsonl

The whole output directory is in `full-interrupted/` (8.7 MB, no file over 20 MB), with `runner.out`, `check.jsonl` and
`check.out`. In `full-interrupted/logs/runner.log` the absolute paths were replaced by `<repo>` and `<dir>`; the
records are unchanged. Every step is listed in `full-steps.md`.

**Outcome.**

- `result` `error`, `error` "signal 15" (traceback in `logs/runner.log`: `run` → `phases_between` → `plugin_phase` →
  `check_signal`, `runner.py:428`); `acceptance.verdict` `error` (certificate steps 0, fallback steps 0, probe null,
  timed-out attempts 0); `config.component` `bulk_budget`; `config.source` commit `35c3d4f6…`, `dirty` false.
- 84 steps, 242 twists (bulk 238, height 4); runner time 22,008.5 s.
- The `r22d75` attempt that was running at the signal is recorded as `cancelled` (rc −9, 24.8 s).

**Phases:**

| Phase | Result | Steps | Twists | Start | End |
|---|---|---|---|---|---|
| bulk | `timeout` (clock 21,600 s) | 80 | 238 | `273eab69…`, 105,437 off, N 131,542, (103,140, 2,297, 0) | `736ea9e7…`, 101,937 off, N 126,592, (99,640, 2,297, 0) |
| height | null (cut off by the signal) | 0 recorded (4 played) | 0 recorded (4 played) | null recorded | null |
| — `r22d75` | null | 4 | 4 | `736ea9e7…` | null; the last step reached `b78611f6…`, 104,941 off, (103,003, 1,938) |
| — `r22d81`, `h4` | null (not reached) | 0 | 0 | — | — |
| high, tail, lattice | null (not reached) | 0 | — | — | — |

Bulk rules: 79 `conj` steps of 3 letters and one `cover` step of 1 letter (step 13, 22.5 s). The bulk's step times ran
from about 300 to 620 s for `conj`; 21,198 s of step time in all. The four height steps are `r22d75/T1`, one letter
each, 2.1 to 2.4 s.

**Expected and actual first steps** (`PLAN.md`, section 4, D2, full run):

| | Expected | Actual |
|---|---|---|
| bulk | on, `--bulk-seconds 21600`; "if the clock ends the bulk, the run is `component`" | the clock ended it: `timeout`, 80 steps, 238 twists; `config.component` `bulk_budget` |
| height | not predicted after a bulk | `r22d75` played 4 steps (n4 2,297 → 1,938) before the signal |
| later phases | not predicted | not reached |

**Check.** `check_run.py` exits 1: `pass` false, `fails` `["phase_steps", "phase_twists"]`, verdict `error`, end
digest `b78611f6ea470cdac4d2add15525f5268420d7d70478dc2a3b25b3491cf97933`, 0 unchecked contracts, `lattice_checked`
false. All 84 step rows pass (84 rows with `pass` true, no failing row), so the words replay to the recorded digests.
The failure comes from the phase totals: `check_run.py:739-745` requires `phases.height.steps` and `twists` to equal
the sum over its sub-phases (4 and 4), and the record has 0 and 0 for the height phase while `r22d75` has `steps` 4.
README section 2 says a phase that an error cut off after some of its steps keeps `result: null` and its steps stand;
here the runner did not fill in the parent height phase's counts. This is a lead for the integrator (the runner's
height totals after a signal inside a sub-phase, or the checker's rule for such a record), not a fix: as the line's
rules require, neither the runner nor the checker was changed and the record was not edited. The check took about
6 minutes (13:37:51 to 13:43:39 UTC).

## Not run

- No Windows, DirectX, native or performance check was run; none is claimed.
- The component run's tail and lattice phases did not run to an end (see "The error"): no tail step, no lattice phase.
- The full run was stopped in its height phase on the integrator's instruction; its height, high, tail and lattice
  phases did not run to an end, and its record fails the checker on the height phase's totals (above).
- No run with a larger `--rule-seconds` was made.
