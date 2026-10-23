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
- 18:51 UTC: the runner was alive at 77 bulk steps and 183 twists, at N 34,622 and 34,258 off, after 7,476 s of runner
  time. All 77 steps agree with the census record (`compare.py`: no difference).
- 21:08 UTC: **third container restart** (uptime 0 min, no processes, no `runner.rc`). The local record's last save
  was at 18:53 UTC (78 steps, 7,908 s). The last pushed checkpoint was `09ece1f` (step 77). As in step 4, I moved the
  surviving `out/` out of the repository, restored the checkpoint into an empty `out/` (exit 0, 77 steps) and resumed
  with the identical command. The resume was accepted. The record's `resumes` now holds two entries, at steps 25 and
  77. The first resume (15:43 UTC, at step 25) is not in it, because it lived only in the local `out/` that the
  16:49 restore replaced. Step 77 (index 77) runs again. From now on the integrator's session sends the check-ins.
- 22:06 and 23:04 UTC: **fourth and fifth container restarts**, each found at a check-in with uptime 0 min, no
  processes and no `runner.rc`. Both times the runner had finished index 77 again (word file written at 22:11:55
  after the 22:06 resume; the 21:08 resume's copy was saved at 21:14). Both times the container died before the
  checkpoint helper's next 900 s round, so the branch still holds only the step 77 checkpoint `09ece1f`. Both
  surviving local copies (78 steps) agree with the census record at all 78 indices (186 twists; index 77 is `conj`,
  3 twists, N 34,622 to 34,588, 34,258 off). Each time I restored `09ece1f` into an empty `out/` and resumed (accepted,
  restore exit 0). A restore drops the resume entries made after its checkpoint, so `resumes` shows [25, 77].

## Problem: the run cannot get past index 77 under the current regime

Evidence: the container now lives only a few minutes after each check-in turn ends (the last saves came 5 to 6 min
after the 21:08 and 22:06 resumes, and nothing was saved after that). The checkpoint helper's default interval is
900 s. One bulk step at this point takes about 5 min (index 77: 282 s on its second run), and the census steps
after index 77 grow to several hundred seconds each (the census record's cumulative times rise from 3,816 s to
4,658 s over its last 30 steps). So every cycle restores step 77, recomputes index 77 and dies before a checkpoint
can save it. Following step 4 exactly, the run makes no progress. Its state stays at 77 steps, 183 twists, N 34,622,
34,258 off.

I have changed nothing to work around this. Options for the integrator, none taken: keep the container alive
between check-ins (an environment setting); a shorter `checkpoint.py --every` interval (a documented option of the
helper, but not the command this task gives); or resuming the surviving local `out/` instead of the older
checkpoint (the resume checks every step's digests, but that is not what step 4 says). I keep restoring and
resuming at each check-in as instructed.

## Restarts under the integrator's revised procedure (23 October 2026, 00:30 UTC)

The integrator changed the procedure: resume the local `out/` directly when it has more steps than the newest
checkpoint (restore only if that resume refuses), run `checkpoint.py --every 120`, and keep a harness-tracked
background waiter on the runner process.

- 00:06 UTC (sixth restart, old procedure): restored `09ece1f` (step 77) and resumed. The container died before
  index 77 was done again.
- 00:34 UTC (seventh restart): local `out/` had 78 steps (index 77 written at 00:12:09), the checkpoint had 77. I
  resumed `out/` directly (accepted, `resumes` [25, 77, 78]), started `checkpoint.py --every 120` and a tracked waiter.
- 02:06 UTC progress: the container stayed up while the waiter ran. **All 108 census steps agree** with the runner's
  first 108 bulk steps: rule, twist count, N before and after, off after, and the twists themselves (272 twists;
  `compare.py`, no difference). Index 107 ends at N 32,642 and 31,479 off, as in the census. The bulk phase went on:
  118 steps at N 32,136 and 30,671 off, after 13,160 s of runner time.
- 02:35 UTC: the waiter stopped at its two-hour limit. The runner was alive at 123 steps.
- 04:06 UTC (eighth restart): the local `out/` and the newest checkpoint `4de07e3` both held 123 steps (index 122
  written at 02:35:05). I restored the checkpoint into an empty `out/` (exit 0), resumed (accepted, `resumes` [25, 77,
  78, 123]), started `checkpoint.py --every 120` and a new waiter.
- 04:34 UTC (ninth restart, about 04:30, **while the waiter was running**, so idleness is not the only cause): the
  local `out/` and checkpoint `42d1d62` both held 127 steps (index 126 written at 04:28:21). I restored (exit 0),
  resumed (accepted, `resumes` [25, 77, 78, 123, 127]), started `checkpoint.py --every 120` and a new waiter.
