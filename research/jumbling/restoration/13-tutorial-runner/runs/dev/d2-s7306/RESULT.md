# Development line d2-s7306: D2, the 3000-twist stop of seed 7306

Status: finished, partial. Step 1 is done. The component run (step 2) was stopped by the integrator after its fourth
container loss, at its last checkpoint: height and high are complete there, and the tail had started without a step.
The full run (step 3) was dropped by the integrator. The run was not checked: `check_run.py` was not run.

## Question

What does the tutorial runner (`runner-run/3`) do from the 3000-twist census stop of seed 7306 (`PLAN.md` section 4,
D2) as a component run with the bulk off?

## Answer (from the last checkpoint, `component-checkpoint/run.json`)

- The height phase ends `stuck` after 18 steps and 19 twists: `r22d75` plays the expected 9 letters to `16c4aefe…`
  and ends `stuck`; `r22d81` is `not_applicable` (n6 = 0); `h4` plays 9 steps (10 twists) to `ac4be82c…` and ends
  `stuck` with reason `pair_exhausted`. n4 falls from 2,762 to 1,955; off rises from 108,823 to 110,315.
- The high phase (the R22D-25 kernel with D19, catalogue `r22d25-cat/1`) ends `stuck` with no step, reason
  `kernel_exhausted`: every family of its ledger is `exhausted` (F1, F2, F3, d19, S, ST, T1).
- The tail started at `ac4be82c…` (its unit listing, `attempts/00018`) and had recorded no step when the container
  was lost. The lattice phase was not reached.

One run, one start state: this says nothing about coverage at other stops.

## Commands (from the repository root)

Every command ran with `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1`.

1. Start table, on commit `35c3d4f`:

       python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/export_table.py research/jumbling/restoration/12-low-height/runs/round22/cloud/census-k3000-s7306/ia-k3000-s7306.json <dir>/tables/s7306-stop.json

   Output `{'record': 'ia-k3000-s7306.json', 'run': 'ia-k3000-s7306', 'moves': 270}
   9ebdc4904438ffec5eb18ec35f71ee01d3e037cae70587a786335ce095b3c83e 108823`, exit 0: digest and off count as expected.
   About 4,840 s wall time (the plan estimated about 1,650 s).

2. Component run. Attempts 1 to 3, on `35c3d4f`/`f080344`/`9b284c5` (only this RESULT.md changed between them), used
   `--out <dir>/out-c<n> --cache <dir>/cache`:

       python3 -I -B -u research/jumbling/restoration/13-tutorial-runner/scripts/runner.py --table <dir>/tables/s7306-stop.json --out <dir>/out-c<n> --cache <dir>/cache --bulk-steps 0 --height-seconds 12000 --height-cpu 12000 --pair-shards 3 --kernel-seconds 3600

   Attempt 4, the recorded one, ran on the merge `0f22c82` (this branch merged with `04057a4` of
   `claude/repair-pair-list`, which adds `--resume` and `scripts/checkpoint.py`), with `config.source.dirty` false:

       python3 -I -B -u research/jumbling/restoration/13-tutorial-runner/scripts/runner.py --table <dir>/tables/s7306-stop.json --out research/jumbling/restoration/13-tutorial-runner/runs/dev/d2-s7306/component --cache <dir>/d2s7306-cache --bulk-steps 0 --height-seconds 12000 --height-cpu 12000 --pair-shards 3 --kernel-seconds 3600
       python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/checkpoint.py --run research/jumbling/restoration/13-tutorial-runner/runs/dev/d2-s7306/component --pid <runner pid> --trailer 'Claude-Session: <session URL>'

## What happened to each attempt

| Attempt | Started (UTC) | End | Reached |
| --- | --- | --- | --- |
| 1 | 05:25 | container restart, about 08:55 | height complete (18 steps, the same as attempt 4); killed in the first kernel call |
| 2 | 08:56 | container restart, about 09:03 | 4 `r22d75` steps |
| 3 | 13:40 | stopped by me by PID after 55 s, 0 steps (`error`, signal), to switch to the resumable procedure | nothing |
| 4 | 13:41 | container lost between 17:24 and 21:02; not resumed (integrator) | height and high complete; tail started at 17:20 |

Attempts 1 to 3 are incomplete records outside the repository and are not committed. Attempt 4's last checkpoint
(commit `7c44034`, 17:24 UTC, step 18) is kept unchanged in `component-checkpoint/` as the partial record. After the
container came back at 21:02, a restore of that checkpoint was begun and interrupted: the live run directory was moved
out of the repository and the restore never ran. Its `run.json` there was the same as the checkpoint's (18 steps, the
tail with no step); its last log was the tail's unit listing, started at 17:20:15, with no output. The integrator then
stopped the line (21:05 UTC), so the run was neither resumed nor checked.

## The record (attempt 4, last checkpoint)

- Format `runner-run/3`, mode `development`, `config.component` `bulk_budget`. The run is not terminal: the stored
  `result` is `budget` and `acceptance.verdict` is null (the verdict is computed only at the end of a run). Had it ended,
  the verdict would be `component`.
- Budgets: `bulk_steps` 0, `rule_seconds` 3,600, `height_seconds` and `height_cpu` 12,000, `kernel_seconds` 3,600,
  `pair_shards` 3, `greedy_seconds` 60, `cert_seconds` 3,600, `fallback_max_length` 1, `par` 1.
- Totals: bulk 0, height 19, high 0, tail 0 twists; lattice null; 13,262 s of runner time at step 18.

| Phase | Result | Steps | Twists | Start → end digest | Off | (n2, n4, n6) at the end | Reason |
| --- | --- | --- | --- | --- | --- | --- | --- |
| bulk | `budget` | 0 | 0 | `9ebdc490…` | 108,823 | (106,032, 2,762, 29) | `--bulk-steps 0` |
| height / `r22d75` | `stuck` | 9 | 9 | `9ebdc490…` → `16c4aefe…` | 111,232 | (108,641, 2,591, 0) | `no_step` |
| height / `r22d81` | `not_applicable` | 0 | 0 | `16c4aefe…` | 111,232 | (108,641, 2,591, 0) | n6 = 0 |
| height / `h4` | `stuck` | 9 | 10 | `16c4aefe…` → `ac4be82c…` | 110,315 | (108,360, 1,955, 0) | `pair_exhausted` |
| high | `stuck` | 0 | 0 | `ac4be82c…` | 110,315 | (108,360, 1,955, 0) | `kernel_exhausted` |
| tail | not finished | 0 | 0 | `ac4be82c…` | 110,315 | | the unit listing was running |
| lattice | not reached | | | | | | |

Last rules and their details:

- `r22d75`: attempt `00009-r22d75-r22d75`, outcome `stuck`, `no_step`, 68.9 s; details `{steps: 0, digest:
  16c4aefe…, n6_n4_n2: [0, 2591, 108641]}`.
- `h4`: attempt `00018-h4-h4` `stuck` (`no_step`, 140.8 s), then `two_letter_u` in three shards, each `done` (3,993.6,
  4,005.4 and 4,005.4 s); the sub-phase ends `pair_exhausted`.
- high: rule `r22d25`, catalogue `r22d25-cat/1`, outcome `stuck`, `kernel_exhausted`, 2,914.6 s (kernel 2,907.1 s);
  `kernel_result` `stuck`, every ledger family `exhausted`; the kernel's record has sha256
  `53548c332bf9401e530217a5dfd399b5359daf37f8387c68b564881df4d1fab1` (`attempts/00018-high/kernel`).

### Expected and actual first steps (`PLAN.md` section 4, D2)

| | Expected | Actual |
| --- | --- | --- |
| Start | `9ebdc490`, 108,823 off, n6 = 29 | `9ebdc490…`, 108,823 off, {2: 106,032, 4: 2,762, 6: 29} |
| `r22d75` | 9 letters → `16c4aefe` (n4 = 2,591), stuck | 9 letters → `16c4aefe…`, n4 = 2,591, n6 = 0, `stuck` |
| `r22d81` | its first run here through the runner | `not_applicable` (n6 = 0), as stage 12's line found (HANDOFF at once, 0 steps) |
| `h4` | new | 9 steps, 10 twists → `ac4be82c…`, n4 = 1,955, `stuck` (`pair_exhausted`) |
| high | unknown | `stuck`, `kernel_exhausted`, no step |
| tail | | started, no step recorded |

The nine `r22d75` end digests match the stage 12 line's table (`25adf6cb`, `e9503aa9`, `01754de3`, `9548bea6`,
`fd310293`, `1ebe4c0b`, `11ec47a4`, `eeb93181`, `16c4aefe`).

### Every step

| # | Sub-phase | Rule | Letters | Digest after | Off | n4 | n6 | Seconds |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | r22d75 | r22d75/T0 | 1 | 25adf6cb | 109,070 | 2,793 | 0 | 2.7 |
| 1 | r22d75 | r22d75/T1 | 1 | e9503aa9 | 109,646 | 2,688 | 0 | 3.6 |
| 2 | r22d75 | r22d75/T1 | 1 | 01754de3 | 111,104 | 2,663 | 0 | 2.7 |
| 3 | r22d75 | r22d75/T1 | 1 | 9548bea6 | 110,676 | 2,648 | 0 | 2.4 |
| 4 | r22d75 | r22d75/T1 | 1 | fd310293 | 110,984 | 2,640 | 0 | 3.1 |
| 5 | r22d75 | r22d75/T1 | 1 | 1ebe4c0b | 111,270 | 2,637 | 0 | 2.8 |
| 6 | r22d75 | r22d75/T1 | 1 | 11ec47a4 | 111,860 | 2,591 | 0 | 3.5 |
| 7 | r22d75 | r22d75/T2 | 1 | eeb93181 | 111,429 | 2,591 | 0 | 3.3 |
| 8 | r22d75 | r22d75/T2 | 1 | 16c4aefe | 111,232 | 2,591 | 0 | 2.6 |
| 9 | h4 | h4/U | 1 | 45ff6a61 | 111,232 | 2,591 | 0 | 2.8 |
| 10 | h4 | h4/U | 1 | 102cd868 | 111,727 | 2,591 | 0 | 2.6 |
| 11 | h4 | h4/L4 | 1 | 501bc41c | 110,342 | 1,955 | 0 | 2.8 |
| 12 | h4 | h4/U | 1 | 645a259d | 110,342 | 1,955 | 0 | 2.6 |
| 13 | h4 | h4/U | 1 | 8fef1f82 | 110,342 | 1,955 | 0 | 2.5 |
| 14 | h4 | h4/U | 1 | 6405edde | 110,342 | 1,955 | 0 | 2.5 |
| 15 | h4 | h4/U | 1 | c11648ba | 110,342 | 1,955 | 0 | 3.0 |
| 16 | h4 | h4/L2 | 1 | a2a43562 | 110,315 | 1,955 | 0 | 3.1 |
| 17 | h4 | h4/pair | 2 | ac4be82c | 110,315 | 1,955 | 0 | 2.8 |

A step's seconds are its recording time; the searches that found it are in its attempts (above). Times are cloud
timings of research code on a shared headless Linux machine, not performance evidence.

## Not run, not finished

- `check_run.py` on the record: not run (the run did not end; the integrator stopped the line). So no checker
  verdict, end digest or unchecked contract count exists.
- The tail and the lattice phase of the component run; the full run (step 3, dropped by the integrator).
- No Windows, DirectX, native, input, long-session or performance check; headless Linux only.
- No file over 20 MB; nothing gzipped by hand. No `runner.out` or `check.jsonl`: the runner's stdout of attempt 4 was
  empty when the container was lost (it prints its summary at the end), and the checker did not run.
