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

Progress note at 07:14 UTC (from `run.json`; the run is not finished, no `bulk-progress.json` since the bulk is
skipped):

- bulk: `budget`, 0 steps (`--bulk-steps 0`);
- height: `stuck`, 11 steps, 11 twists, `273eab69…` (105,437 off; n2 103,140, n4 2,297) to `cc5ef7ec…` (109,872 off;
  n2 108,453, n4 1,419):
  - `r22d75`: `stuck` (`no_step`) after 11 steps, at `cc5ef7ec…`, as expected (11 letters, n4 = 1,419). The last
    `r22d75` attempt finished 05:25 UTC;
  - `r22d81`: `not_applicable` (n6 = 0), as expected;
  - `h4`: `stuck` (`pair_exhausted`), 0 steps; its pair shards finished about 05:37 UTC;
- high: `stuck` (`kernel_exhausted`), 0 steps; the kernel's ledger shows F1, F2, F3, d19, S, ST and T1 all
  `exhausted`; the kernel log finished 06:29 UTC;
- tail: running since 06:29 UTC (`attempts/00011`); its units log is still empty;
- lattice: not reached yet.

Next: wait for `runner.rc`, then run `check_run.py` on the record.

## Step 3: full run (not started)

## Not run

No Windows, DirectX, native or performance check was run; none is claimed.
