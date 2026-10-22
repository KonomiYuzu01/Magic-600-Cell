# Development line d2-s7302: D2, the 3000-twist stop of seed 7302

Status: in progress (progress note, 22 October 2026).

## Question

PLAN.md section 4, D2, seed 7302: what does the tutorial runner (`runner-run/3`) do from the census stop of seed 7302
(3000 twists, start `015d839e…`, 100,584 off), first as a component run with the bulk skipped, then as a full run
with the bulk on for at most six hours? Does the component run follow the first steps that stage 12 found with its
separate scripts?

All commands run from the repository root, on commit `35c3d4f`, with
`OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1`. `S=research/jumbling/restoration/13-tutorial-runner/scripts`,
`L=research/jumbling/restoration/12-low-height/scripts`. `<dir>` stands for a scratch directory outside the
repository. Times are cloud timings of research code on a shared headless Linux machine (4 cores), not performance
evidence.

## Step 1: start table (done)

    python3 -I -B -u $L/export_table.py research/jumbling/restoration/11-scramble-census/runs/heights/ia-k3000-s7302.json <dir>/tables/s7302-stop.json

Output: `{'record': 'ia-k3000-s7302.json', 'run': 'ia-k3000-s7302', 'moves': 313}
015d839ecbb37bbdde74ae9391d5d7f274dea7fee4b4fdf0092d224eef907a1f 100584`. Digest and off count match the expected
values. Wall time about 95 minutes (04:03 to 05:38 UTC), against the plan's estimate of about 1,650 s. The table is
not committed (it is rebuilt by this command).

## Step 2: component run (running, started 05:39 UTC)

    python3 -I -B -u $S/runner.py --table <dir>/tables/s7302-stop.json --out <dir>/out-c --cache <dir>/cache --bulk-steps 0 --height-seconds 12000 --height-cpu 12000 --pair-shards 3 --kernel-seconds 3600

## Next steps

1. When the component run ends: copy its output to `out-c/`, run `check_run.py`, record the steps.
2. Full run (step 3), then the reproduction (step 4) if time allows.
