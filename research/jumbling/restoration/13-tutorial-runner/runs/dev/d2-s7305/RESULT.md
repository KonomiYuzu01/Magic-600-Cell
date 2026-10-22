# D2, seed 7305: the runner from the 3000-twist stop (development line d2-s7305)

Status: step 1 done, step 2 (component run) done and checked, step 3 (full run) running. Times are cloud timings of
research code on a shared headless Linux machine, not performance evidence.

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

## Step 3: full run (running)

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

## Not run

No Windows, DirectX, native or performance check was run; none is claimed. The component run's tail and lattice phases
did not run to an end (see "The error").
