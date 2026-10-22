# D2, seed 7305: the runner from the 3000-twist stop (development line d2-s7305)

Status: in progress (progress note). Times are cloud timings of research code on a shared headless Linux machine, not
performance evidence.

## Question

Starting from the 3000-twist census stop of seed 7305 (`12-low-height/runs/round22/cloud/census-k3000-s7305/`), what
does the tutorial runner (`runner-run/3`, development mode) do: first as a component run without the bulk
(`--bulk-steps 0`), then as a full run with the bulk on (`PLAN.md`, section 4, "D2")?

## Commit

All commands ran on commit `35c3d4f` (branch `claude/repair-pair-list`, checked out as `claude/dev-d2-s7305`), from
the repository root, with `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1`.
`S=research/jumbling/restoration/13-tutorial-runner/scripts`, `L=research/jumbling/restoration/12-low-height/scripts`.

## Step 1: start table (done)

    python3 -I -B -u $L/export_table.py research/jumbling/restoration/12-low-height/runs/round22/cloud/census-k3000-s7305/ia-k3000-s7305.json <dir>/tables/s7305-stop.json

Output (`export.out`): 103 moves, digest `273eab69dee9e3a2f4a3997a75f6b8d9ad16814fa26b941bf7a219247f9bab6d`, 105,437
off. Both match the expected values. Wall time about 2,300 s (started 04:04 UTC, finished between 04:45 and 05:12 UTC).

## Step 2: component run (running)

    python3 -I -B -u $S/runner.py --table <dir>/tables/s7305-stop.json --out <dir>/out-c --cache <dir>/cache --bulk-steps 0 --height-seconds 12000 --height-cpu 12000 --pair-shards 3 --kernel-seconds 3600

Started 05:13 UTC on 22 October 2026.

## Step 3: full run (not started)

## Not run

No Windows, DirectX, native or performance check was run; none is claimed.
