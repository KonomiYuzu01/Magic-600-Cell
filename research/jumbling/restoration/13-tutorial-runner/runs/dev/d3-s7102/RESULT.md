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
- 15:43 UTC check-in: **the container had restarted** (uptime 0 min, no runner or checkpoint process, no
  `runner.rc`). The record's last save was at 13:49 UTC: 25 bulk steps, 39 twists, 508 s of runner time. The
  checkpoint helper had committed **no checkpoint**: the runner stopped before the first 900 s round, and
  `checkpoint.out` is empty. So there was no `out-checkpoint` to restore. The local `out/` and the cache had survived on
  disk. I started the identical runner command with `--resume` on that `out/`, appending to `runner.out`, and
  started `checkpoint.py` beside it again. The resume was accepted: it replayed the 25 steps, checked each step's
  before and after digests and word hashes, and recorded `resumes: [{at_step: 25, seconds: 508.3, source: 4024027
  (dirty: the untracked run files)}]`. This differs from step 4 of the task (restore a checkpoint into an empty
  `out/`) only because no checkpoint existed. The resume's own checks are the same.
- 16:49 UTC: **the container had restarted again** (uptime 0 min, no processes, no `runner.rc`). The local record's
  last save was at 15:49 UTC: 29 bulk steps, 787 s of runner time. The branch held one checkpoint (`773deea`, step
  25). Following step 4, I moved the surviving `out/` out of the repository, restored the checkpoint into an empty
  `out/` (`checkpoint.py --restore`, exit 0, 25 steps) and started the identical runner with `--resume`, plus
  `checkpoint.py` beside it. Steps 25 to 28 are run again. Both restarts came soon after this session went idle (the
  last saves were at 13:49 and 15:49 UTC), so the container is probably reclaimed when the session is idle, not
  restarted at random. From now on this session keeps a monitor on the run so that it stays active.
