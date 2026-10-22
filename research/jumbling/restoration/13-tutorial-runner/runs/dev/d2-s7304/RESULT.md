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

Started 05:30 UTC. Running.

## Step 3: full run

Not started; it starts after the component run's runner has finished.

## Next step

Wait for the component run (`runner-c.rc`), copy its output directory to `component/`, run `check_run.py` on it, then
start the full run.
