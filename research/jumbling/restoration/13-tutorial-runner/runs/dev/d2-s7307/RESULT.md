# Development line d2-s7307: D2, the 3000-twist stop of seed 7307

Status: finished (22 October 2026). Research code, headless Linux only. All times are cloud timings of research code on
a shared headless Linux machine, not performance evidence.

## Question

What does the tutorial runner (`runner-run/3`) do from the census stop of seed 7307 at depth 3000 (`ba5371ae…`,
96,236 off, n6 = 10)? Run without the bulk (`--bulk-steps 0`, a component run) and check the record with
`check_run.py` (`PLAN.md` section 4, D2). The full run with the bulk on was planned as well. The integrator dropped it
during the line (below).

## Summary

| Item | Value |
| --- | --- |
| Commit | `35c3d4f` (`config.source.dirty` false) |
| Start | `ba5371aebe00ac9411567545fe79e90fa3d2487d000d2c26d45613924d4e33ff`, 96,236 off, heights {2: 94,691, 4: 1,535, 6: 10} |
| `result` | `error`: `rule subprocess exceeded rule_seconds` (the tail's unit listing) |
| `acceptance.verdict` | `error` |
| `config.component` | `bulk_budget` |
| Steps, twists | 6 steps, 6 twists, all in `r22d75` |
| End | `895b45de8a9a83604ca8bf988a3c9a287e404e59bd22a5004703d0852b252507`, 100,569 off, (n2, n4, n6) = (98,984, 1,585, 0) |
| Checker | pass, verdict `error`, end digest `895b45de…`, 0 unchecked contracts, 6 steps passed |
| Runner time | 8,530.7 s in all (`totals.seconds`) |

The first steps match the plan's prediction and stage 12's R22D-75 run at this stop letter for letter. The run then
stopped in the same way as the integrator saw at the stops of 7304 and 7305. `r22d81` was not applicable (n6 = 0).
`h4` found no step, its pair search was exhausted and stayed within its clocks. The high kernel exhausted every family.
In the tail, the unit listing (`mkblocks2.py`) at 100,569 off ran past `--rule-seconds` (3,600 s by default). The
runner raises `RuleFailure('timeout', …)` there, and nothing catches it. So the run ended `error` and the tail and
lattice phases have no result.

## Expected and actual first steps

Expected (`PLAN.md` section 4, D2 table): `r22d75` 6 letters → `895b45de` (n4 = 1,585), stuck; n6 = 10 at the start.

| # | Sub-phase | Rule | Letter | Length | Before | After | Off after | Heights after | Time (s) |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | r22d75 | r22d75/T0 | (275, 14) | 1 | `ba5371ae` | `a342049d` | 96,579 | {2: 95,034, 4: 1,538, 6: 7} | 1.63 |
| 1 | r22d75 | r22d75/T0 | (516, 13) | 1 | `a342049d` | `b3982155` | 97,338 | {2: 95,398, 4: 1,940} | 1.70 |
| 2 | r22d75 | r22d75/T1 | (50, 15) | 1 | `b3982155` | `3b6790a1` | 98,095 | {2: 96,353, 4: 1,742} | 1.45 |
| 3 | r22d75 | r22d75/T1 | (474, 13) | 1 | `3b6790a1` | `7cc2722e` | 100,331 | {2: 98,635, 4: 1,696} | 1.50 |
| 4 | r22d75 | r22d75/T1 | (494, 15) | 1 | `7cc2722e` | `f5a85c13` | 100,760 | {2: 99,175, 4: 1,585} | 1.46 |
| 5 | r22d75 | r22d75/T2 | (147, 14) | 1 | `f5a85c13` | `895b45de` | 100,569 | {2: 98,984, 4: 1,585} | 1.67 |

The six letters and digests are the same as stage 12's
(`12-low-height/runs/round22/cloud/census-k3000-s7307/RESULT.md`, "R22D-75 at the seed 7307 stop"). Every step has
`standing: rule` and the contract named by its rule. Each step tree is one span named by its rule.

## Phases of the component run

| Phase | Result | Steps | Twists | Start → end | Reason, attempts |
| --- | --- | --- | --- | --- | --- |
| bulk | `budget` | 0 | 0 | `ba5371ae` | `--bulk-steps 0` |
| height / `r22d75` | `stuck` | 6 | 6 | `ba5371ae` (96,236 off; n4 1,535, n6 10) → `895b45de` (100,569 off; n4 1,585, n6 0) | `no_step`, final attempt 49.3 s |
| height / `r22d81` | `not_applicable` | 0 | – | `895b45de` | n6 = 0 |
| height / `h4` | `stuck` | 0 | – | `895b45de` | `pair_exhausted`: `h4` no step (59.7 s); `two_letter_u` shards 0, 1, 2 `done` (1,869.1, 1,820.3, 1,820.7 s), none timed out |
| height (whole) | `stuck` | 6 | 6 | `ba5371ae` → `895b45de` | |
| high | `stuck` | 0 | 0 | `895b45de` | `kernel_exhausted`: kernel `stuck`; ledger F1, F2, F3, d19, S, ST, T1 all `exhausted`; 2,643.2 s; kernel record sha256 `4c2281d0…` |
| tail | null | 0 | – | `895b45de` | the unit listing passed `rule_seconds` 3,600 s → run `error` |
| lattice | null | 0 | null | – | not reached |

Totals: bulk 0, height 6, high 0, tail 0, fallback 0, lattice null twists. `acceptance`: `certificate_steps` 0,
`fallback_steps` 0, `probe` null, `timed_out_attempts` 0.

The runner's traceback (`component/logs/runner.log`, paths written `<dir>`): `runner.py` line 1475 `self.tail()` →
line 1270 `self.units(...)` → line 771 `self.require(child)` → line 562 `raise RuleFailure('timeout', 'rule subprocess
exceeded rule_seconds', timed_out=True)`. The units log `component/logs/attempts-00006-units-units.log` is empty.
Under `README.md` section 1, a clock in a tail attempt should give a timed-out attempt (tail `timeout`, run `budget`).
Here the clock fired on the unit listing, before any unit was tried, and the run ended `error` instead. This is a
lead for the integrator. The runner was not changed.

## Commands (from the repository root, on commit `35c3d4f`)

Every command ran with `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1`, detached as the common rules say.
`S=research/jumbling/restoration/13-tutorial-runner/scripts`, `L=research/jumbling/restoration/12-low-height/scripts`.

1. Start table (04:04 to 04:59 UTC; it printed digest `ba5371ae…4e33ff` and 96,236 off as expected, after replaying 734 descent
   moves):

       python3 -I -B -u $L/export_table.py research/jumbling/restoration/12-low-height/runs/round22/cloud/census-k3000-s7307/ia-k3000-s7307.json <dir>/tables/s7307-stop.json

2. Component run (05:00 to 07:22 UTC, exit code 2):

       python3 -I -B -u $S/runner.py --table <dir>/tables/s7307-stop.json --out <dir>/out-c --cache <dir>/cache-c --bulk-steps 0 --height-seconds 12000 --height-cpu 12000 --pair-shards 3 --kernel-seconds 3600

3. Check (13:41 UTC, exit code 0, under a minute):

       python3 -I -B -u $S/check_run.py <dir>/out-c --out <dir>/check-c/check.jsonl

## The full run (step 3): started, then dropped

- **First attempt** (`full-interrupted/`). The command was step 2's without `--bulk-steps 0` and with
  `--bulk-seconds 21600 --bulk-step-seconds 21600`, into `<dir>/out-f`. It started at 07:23 UTC. A container restart
  killed it, and its last checkpoint is from 08:57 UTC. That checkpoint has 20 bulk steps, all `conj`, of 3 twists each
  (60 twists, 5,633.8 s). They take N from 112,776 to 111,888 and off from 96,236 to 95,431 (heights {2: 93,886,
  4: 1,535, 6: 10}). The bulk end so far is `21fa8560…`. The runner had not finished, so `run.json` holds its
  in-progress checkpoint, with `result` `budget` and verdict null. It is a partial record, not a result. It was not
  checked, and `runner.out` is empty.
- **Second attempt.** The same command was started again at 13:41 UTC in fresh directories. Then the integrator's
  update of 13:40 UTC dropped the full run for D2. Its reason: the component runs at the stops of 7304, 7305 and 7307
  show the high phase's kernel exhausted and the tail's unit listing past `--rule-seconds`, and a full run puts a
  six-hour bulk in front of the same phases. I stopped the runner by its PID with SIGTERM after about 40 s. It recorded
  `error` `signal 15` with 0 steps. That record was not kept.
- The check of the component run also died in the 07:23 restart, so it was run again at 13:41.

## Files

- `component/`: the component run's whole output directory, plus `runner.out` (stdout), `check.jsonl` (the checker's
  report) and `check.out` (the checker's stdout and stderr). In `logs/runner.log` the checkout path is replaced by
  `<dir>`. Nothing else was edited. No file is over 20 MB, so nothing was gzipped. The cache was not committed.
- `full-interrupted/`: the partial record of the first full run, as the restart left it, plus its empty `runner.out`.

## Not run or not finished

- The tail and lattice phases from `895b45de…`: the run ended `error` in the tail's unit listing.
- The full run (step 3): dropped by the integrator. Only the partial bulk record above exists.
- Only headless Linux checks ran. No Windows, DirectX, native host or performance check was run. Nothing here is
  evidence for any of them.
