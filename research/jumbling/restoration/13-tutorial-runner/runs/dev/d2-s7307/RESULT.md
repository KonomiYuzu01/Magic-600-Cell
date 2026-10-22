# Development line d2-s7307: D2, the 3000-twist stop of seed 7307

Status: in progress (progress note, 22 October 2026, 07:00 UTC). Research code, headless Linux only.

## Question

What does the tutorial runner (`runner-run/3`) do from the census stop of seed 7307 at depth 3000
(`ba5371ae…`, 96,236 off, n6 = 10)? First as a component run without the bulk (`--bulk-steps 0`), then as a full
development run with the bulk on (PLAN.md section 4, D2).

## Step 1: start table (done)

    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/export_table.py research/jumbling/restoration/12-low-height/runs/round22/cloud/census-k3000-s7307/ia-k3000-s7307.json <tables>/s7307-stop.json

Printed digest `ba5371aebe00ac9411567545fe79e90fa3d2487d000d2c26d45613924d4e33ff` and 96,236 off, as expected
(734 descent moves replayed; about 1,650 s).

## Step 2: component run (running)

Started 05:00 UTC on commit `35c3d4f`. Progress at 07:00 UTC from `run.json` (no `bulk-progress.json`: the bulk is
skipped):

- bulk `budget`, 0 steps (`config.component` `bulk_budget`);
- `r22d75`: 6 steps of one letter, `ba5371ae…` → `895b45de…` (n4 = 1,585, n6 = 0), then `stuck` (`no_step`), as
  expected;
- `r22d81`: `not_applicable` (n6 = 0);
- `h4`: `stuck` (`pair_exhausted`): `h4` found no step; the three `two_letter_u` shards ran about 1,820-1,870 s each
  and found none;
- high: `stuck` (`kernel_exhausted`, every family of the ledger exhausted), 0 steps, kernel 2,643 s;
- tail: running (units census) at 07:00 UTC.

Next: wait for `runner-c.rc`, run `check_run.py` on the component run, copy the run here, then start the full run
(step 3).
