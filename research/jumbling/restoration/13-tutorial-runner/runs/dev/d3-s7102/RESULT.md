# D3, seed 7102 at depth 100, acceptance mode (development line d3-s7102)

Status: **in progress** (22 October 2026, 13:50 UTC). The run has not ended; the check has not run.

## Question

The owner's decision of 22 October 2026 (`docs/wiki/decisions/owner-decisions-2026-10-22-tutorial-first.md`, item 3)
starts batch 1 of the acceptance test only once development runs at depths 10, 20, 40 and 100 reach the solved state.
What does the tutorial runner do in acceptance mode from the depth 100 scramble of seed 7102? Do its bulk steps agree
with the census record `11-scramble-census/runs/round2/ia-k100-s7102.json` (108 steps, 272 twists, ended
`time-limit`) as far as that record goes? Does the run reach the solved state; if not, in which phase and with which
result and reason does it end?

## Commands (from the repository root)

Commit `04057a4` of `claude/repair-pair-list`. `S=research/jumbling/restoration/13-tutorial-runner/scripts`,
`<dir>` = `research/jumbling/restoration/13-tutorial-runner/runs/dev/d3-s7102`.

    (OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/runner.py --scramble 100 7102 \
        --out <dir>/out --cache <cache dir> --mode acceptance > <dir>/runner.out 2>&1; echo $? > <dir>/runner.rc) &
    python3 -I -B $S/checkpoint.py --run <dir>/out --pid <pid of that subshell> --trailer ... --trailer ... &

The cache directory is outside the repository. Started at 13:40 UTC. Bulk comparison:
`python3 -I -B <dir>/compare.py <dir>/out 11-scramble-census/runs/round2/ia-k100-s7102.json <dir>/bulk-vs-census.md`.

## Progress

- 13:50 UTC: the start state matches the census record (115,566 off, N 73,454). The first two bulk steps agree with
  the census record (rule, twists, N, off and the twists themselves).
