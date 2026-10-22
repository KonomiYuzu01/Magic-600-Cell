# Development line d2-s7304: D2, the 3000-twist stop of seed 7304

Status: in progress (progress note, 22 October 2026).

## Question

How far does the tutorial runner (`runner-run/3`, development mode) get from the census stop of seed 7304 at 3000
scramble twists (`PLAN.md`, section 4, D2): first as a component run with the bulk left out (`--bulk-steps 0`), then
as a full run with the bulk on? Does the component run reproduce stage 12's separate scripts (`r22d75` 6 letters to
`ac127dc3…` with 797 pieces at height 4, then stuck; `r22d81` not applicable)? Does `check_run.py` pass both records?

All times are cloud timings of research code on a shared headless Linux machine, not performance evidence. Headless
Linux only: no Windows, DirectX, native host, input, long-session or performance check was run or is claimed.

## Commit

All commands ran on commit `35c3d4f` (branch `claude/dev-d2-s7304`), from the repository root, each prefixed with
`OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1`. `S=research/jumbling/restoration/13-tutorial-runner/scripts`,
`L=research/jumbling/restoration/12-low-height/scripts`; `<dir>` is a scratch directory outside the repository.

## Step 1: start table

    python3 -I -B -u $L/export_table.py research/jumbling/restoration/12-low-height/runs/round22/cloud/census-k3000-s7304/ia-k3000-s7304.json <dir>/tables/s7304-stop.json

`export_table.py` sha256 `e3299974cf48386256145bdae426534ed1003e8743549784308868eba547bd00`. Output:

    {'record': 'ia-k3000-s7304.json', 'run': 'ia-k3000-s7304', 'moves': 580} 7e2304b2a4a053b8714dd806e43ff64c2de9e11f5ef2258f3b8a65c9c1a936d4 95569

Digest `7e2304b2…` and 95,569 off, as required. A first attempt was killed by a container restart after about 25
minutes; the second took about 3,600 s (04:29 to 05:29 UTC), against the plan's estimate of about 1,650 s. The table
(1.4 MB) is not committed; it is rebuilt by this command.

## Step 2: component run

    python3 -I -B -u $S/runner.py --table <dir>/tables/s7304-stop.json --out <dir>/component --cache <dir>/cache-c --bulk-steps 0 --height-seconds 12000 --height-cpu 12000 --pair-shards 3 --kernel-seconds 3600

Ran 05:30 to 07:11 UTC; runner exit code 2. Output directory copied to `component/` (13 MB, no file over 20 MB),
runner stdout as `component/runner.out`. In the committed copy of `component/logs/runner.log` only, the checkout
prefix in the traceback's file paths is replaced by `<repo>/` (the checker does not read that file; `run.json` and
every other file are unchanged).

Runner's last line:

    {"result": "error", "steps": 6, "totals": {"bulk_twists": 0, "height_twists": 6, "high_twists": 0, "tail_twists": 0, "fallback_twists": 0, "lattice_twists": null, "seconds": 6035.392565}}

- `result` `error`, `error` "rule subprocess exceeded rule_seconds"; `acceptance.verdict` `error`
  (0 certificate steps, 0 fallback steps, 0 timed-out attempts); `config.component` `bulk_budget`.
- The failure: after the high phase, the tail phase computes the units of the state (`runner.py:1270`, `units`,
  :771) in a rule subprocess. It ran from 06:10 for the default `rule_seconds` of 3,600 s and was stopped
  (`RuleFailure: timeout`, `runner.py:562`); its log `logs/attempts-00006-units-units.log` is empty. The tail and
  lattice phases have result `null` and 0 steps. As the rules say, the runner was not changed and the run was not
  repeated with a larger `--rule-seconds`.

Phases (digests are the first 8 hex digits; (n2, n4, n6) from the record's `heights`):

| Phase | Result | Steps | Twists | Start | End |
| --- | --- | --- | --- | --- | --- |
| bulk | `budget` (`--bulk-steps 0`) | 0 | 0 | `7e2304b2`, 95,569 off, (94,384, 1,185, 0) | same |
| height | `stuck` | 6 | 6 | `7e2304b2`, 95,569 off, (94,384, 1,185, 0) | `ac127dc3`, 96,471 off, (95,674, 797, 0) |
| · `r22d75` | `stuck` (`no_step`; STUCK "all single letters", 58.4 s) | 6 | 6 | `7e2304b2` | `ac127dc3` |
| · `r22d81` | `not_applicable` | 0 | 0 | `ac127dc3` | `ac127dc3` |
| · `h4` | `stuck` (`pair_exhausted`; `h4` 64.7 s, then three `two_letter_u` shards of 1,123.6, 1,074.0 and 1,171.0 s, all `done`) | 0 | 0 | `ac127dc3` | `ac127dc3` |
| high | `stuck` (`kernel_exhausted`, catalogue `r22d25-cat/1`; F1, F2, F3, d19, S, ST, T1 all exhausted; 772.8 s) | 0 | 0 | `ac127dc3`, 96,471 off, (95,674, 797, 0) | same |
| tail | `null` (error: `units` timed out) | 0 | 0 | | |
| lattice | `null` (not entered) | 0 | | | |

Height steps (each one letter; seconds as recorded):

| Index | Rule | Letter (stage 12) | Seconds |
| --- | --- | --- | --- |
| 0 | `r22d75/T1` | (170, 12) | 1.87 |
| 1 | `r22d75/T1` | (395, 12) | 1.79 |
| 2 | `r22d75/T1` | (500, 12) | 1.72 |
| 3 | `r22d75/T1` | (540, 12) | 1.70 |
| 4 | `r22d75/T2` | (217, 15) | 1.77 |
| 5 | `r22d75/T2` | (148, 15) | 1.78 |

Expected against actual first steps:

| Item | Expected (stage 12, `census-k3000-s7304/RESULT.md`) | Actual (component run) |
| --- | --- | --- |
| Start | `7e2304b2…`, 95,569 off, n6 = 0 | `7e2304b2…`, 95,569 off, (94,384, 1,185, 0) |
| `r22d75` | 6 letters → `ac127dc3…`, 797 at height 4, then stuck | 6 steps of one letter → `ac127dc3…`, (95,674, 797, 0), 96,471 off, then stuck (`no_step`) |
| `r22d81` | not applicable (n6 = 0) | `not_applicable` |
| `h4` | new | `stuck`, 0 steps (`pair_exhausted`) |
| high | `stuck` expected (stage 12's kernel with D19 at this stop found no step) | `stuck`, 0 steps (`kernel_exhausted`) |
| tail, lattice | if reached | not reached: `units` timed out at 3,600 s |

The end digest `ac127dc3…` and the end counts match stage 12's R22D-75 table exactly. The letters column above is
stage 12's; that the runner's six words are these letters follows from the matching end digest and per-step counts,
which were not compared step by step here.

Checker:

    python3 -I -B -u $S/check_run.py <dir>/component --out <dir>/check-c/check.jsonl

Exit code 0, about a minute. Final row: `pass` true, `verdict` `error`, `result` `error`, end digest
`ac127dc3488832b1d1b75a043d63e84b75addd1f4e73fe7e961407aea2ff0980`, 6 steps, 6 height twists,
`unchecked_contracts` 0, `lattice_checked` false. All six step rows pass with no fails. Report in
`component/check.jsonl`; `component/check.out` is its stdout and stderr (identical to it: no stderr).

## Step 3: full run

    python3 -I -B -u $S/runner.py --table <dir>/tables/s7304-stop.json --out <dir>/full --cache <dir>/cache-f --height-seconds 12000 --height-cpu 12000 --pair-shards 3 --kernel-seconds 3600 --bulk-seconds 21600 --bulk-step-seconds 21600

Started about 07:13 UTC, after the component runner had finished. Running. Like the component run it uses the default
`rule_seconds` of 3,600 s, so its tail may meet the same `units` time-out.

## Next step

Wait for
the full run (`runner-f.rc`), copy it to `full/`, run `check_run.py` on it.
