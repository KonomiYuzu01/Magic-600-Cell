# Development line d2-s7306: D2, the 3000-twist stop of seed 7306

Status: in progress (progress note, 22 October 2026, 07:30 UTC). Step 1 is done; step 2 (component run) is running.

## Question

What does the tutorial runner (`runner-run/3`) do from the 3000-twist census stop of seed 7306 (`PLAN.md` section 4,
D2): first as a component run with the bulk off, then as a full development run with the bulk on?

## Step 1: start table (done)

    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/export_table.py research/jumbling/restoration/12-low-height/runs/round22/cloud/census-k3000-s7306/ia-k3000-s7306.json <dir>/tables/s7306-stop.json

Commit `35c3d4f`. Output: `{'record': 'ia-k3000-s7306.json', 'run': 'ia-k3000-s7306', 'moves': 270}
9ebdc4904438ffec5eb18ec35f71ee01d3e037cae70587a786335ce095b3c83e 108823`, exit 0. The digest and the off count match
the expected values. Wall time about 4,840 s (the plan estimated about 1,650 s). The table (1.6 MB) is outside the
repository; the runner keeps its own copy as `start-table.json` in each run.

## Step 2: component run (running; progress note)

    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/13-tutorial-runner/scripts/runner.py --table <dir>/tables/s7306-stop.json --out <dir>/out-c --cache <dir>/cache --bulk-steps 0 --height-seconds 12000 --height-cpu 12000 --pair-shards 3 --kernel-seconds 3600

Started at 05:25 UTC on commit `35c3d4f`. State at 07:26 UTC (from `run.json`; there is no `bulk-progress.json`,
since the bulk is off):

- bulk `budget`, 0 steps (`--bulk-steps 0`);
- `r22d75` `stuck` (`no_step`) after 9 one-letter steps, ending at `16c4aefe…` with n4 = 2,591 and n6 = 0, off
  111,232: the same nine end digests as the stage 12 line's table (`25adf6cb`, `e9503aa9`, `01754de3`, `9548bea6`,
  `fd310293`, `1ebe4c0b`, `11ec47a4`, `eeb93181`, `16c4aefe`);
- `r22d81` `not_applicable` (n6 = 0 where it starts);
- `h4` running: 9 steps so far (`h4/U` ×6, `h4/L4`, `h4/L2`, `h4/pair` of 2 letters), at `ac4be82c…` with
  {2: 108,360, 4: 1,955}, off 110,315. The `h4/L4` step lowered n4 from 2,591 to 1,955.
- Current call: `00018-h4`, three `two_letter_u.py` shards, each about 2 minutes of CPU so far; the last log line of
  each shard is its `CONTEXT` line at `ac4be82c…` (tau 240, u 1).

Times are cloud timings of research code on a shared headless Linux machine, not performance evidence.

## Next step

Wait for `<dir>/runner-c.rc`, run `check_run.py` on the component run, copy the run into this directory, then step 3
(the full run with `--bulk-seconds 21600 --bulk-step-seconds 21600`).

## Not run

No Windows, DirectX, native, input, long-session or performance check.
