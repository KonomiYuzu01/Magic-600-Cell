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
- 05:39 UTC (tenth restart, waiter running): the local `out/` and checkpoint `39ec79b` both held 132 steps (index 131
  written at 05:39:26, seconds before the restart). Restored (exit 0), resumed (accepted, `resumes` [..., 132]),
  checkpoint helper and waiter started again. Runner time so far: 20,000 s.
- 06:08 UTC progress: runner alive at 137 steps. 07:41 UTC: the waiter stopped at its two-hour limit, runner alive at
  152 steps.
- 08:06 UTC (eleventh restart, after the waiter had stopped): the local `out/` held 153 steps (index 152 written at
  07:46:27), the checkpoint `7f6b467` held 152. I resumed `out/` directly (accepted, `resumes` [..., 153]) and started
  the checkpoint helper and a new waiter. State after index 152: N 30,092, 28,498 off, 27,527 s of runner time, still
  in the bulk phase.
- 10:06 UTC progress: runner alive at 164 steps (N 29,620, 27,830 off). The waiter stopped at its two-hour limit
  soon after.
- 12:13 UTC (twelfth restart; last save at 10:10, a few minutes after the waiter stopped): the local `out/` and
  checkpoint `1459963` both held 168 steps. Restored (exit 0), resumed (accepted, `resumes` [..., 168]), checkpoint
  helper and a new waiter started. State: N 29,608, 27,830 off, 34,874 s of runner time, still in the bulk phase. The
  last steps change N by only a few each (164 to 168: 29,620 to 29,608) and leave off unchanged.
- 14:08 UTC progress: runner alive at 188 steps (N 28,794, 26,814 off); a second waiter was started before the first
  reached its limit. 16:08 UTC: that waiter stopped at its limit, runner alive at 201 steps.
- 16:09 UTC (thirteenth restart, within a minute of the waiter stopping): the local `out/` and checkpoint `040f615`
  both held 201 steps. Restored (exit 0), resumed (accepted, `resumes` [..., 201]), checkpoint helper and a new waiter
  started. State: N 27,598, 24,789 off, 48,091 s of runner time, still in the bulk phase.
- 18:20 UTC (fourteenth restart, about 18:15, with a waiter running): the local `out/` and checkpoint `f56ffa9` both
  held 205 steps. Restored (exit 0), resumed (accepted, `resumes` [..., 205]), checkpoint helper and waiter started.
- 20:06 UTC progress: runner alive at 209 bulk steps, N 27,258, 24,603 off, 59,278 s of runner time. The bulk phase
  is slowing: 205 to 209 took about 4,800 s of runner time and left off unchanged.
- 24 October, 00:05 UTC progress: runner alive at 218 bulk steps, N 26,988, 24,340 off, 73,660 s of runner time,
  still in the bulk phase. No restart since 18:15 UTC (container up 5 h 45 min, overlapping waiters).
- 02:05 UTC progress: runner alive at 224 bulk steps, N 26,732, 23,958 off, 82,013 s of runner time.

## Run 1 stopped; run 2 on the bulk speed-up (24 October 2026)

- **Run 1 stopped at 225 bulk steps on the old code** (commit `04057a4`), at 02:26 UTC, on the integrator's
  instruction. It is superseded by run 2. Its record is the last checkpoint `04f3197` in `out-checkpoint/` (225
  steps, still in the bulk phase, N 26,730, 23,958 off, after-digest `aeecee11…`, 83,416 s of runner time). The
  runner's process groups and `checkpoint.py` were killed after that checkpoint was pushed; no `runner.rc` was
  written. The bulk phase had not ended, so run 1 has no phase result and no check.
- Run 2 runs from the start on merged `main` (PR #132, `7dbc4ae`: exact bulk speed-up, rimx cached domains and
  the seamsx restorer with parallel conj trials), with the identical runner command, `--out <dir>/out2` and
  `JUMBLING_BULK_WORKERS=3`, beside `checkpoint.py --run <dir>/out2 --every 120`. `--resume` cannot continue run 1,
  because its fingerprint pins the research code. `compare_runs.py` compares run 2 with run 1 step by step (rule,
  letters, word sha256, before and after digest, N).
- Run 2 started at 02:27 UTC on `2e2609a` (the merge). 03:29 UTC (container restart, the first of run 2): local
  `out2/` held 158 steps, checkpoint `f68c161` 156. I resumed `out2/` directly (accepted, `resumes` [158]) with
  `JUMBLING_BULK_WORKERS=3`, plus `checkpoint.py --every 120` and a waiter. **Run 2 agrees with run 1 at all 158
  steps** (rule, letters, word sha256, before and after digest, N; `compare_runs.py`, no difference). Those 158
  steps took 3,642 s in run 2 against 29,996 s in run 1 (cloud timings of research code on a shared headless
  Linux machine, not performance evidence).
- 06:06 UTC (second restart of run 2; last save at 05:38): local `out2/` and checkpoint `6600cca` both held 232
  steps. Restored (exit 0), resumed (accepted, `resumes` [158, 232]), checkpoint helper and waiter started. **Run 2
  agrees with run 1 at all 225 steps of run 1** (`compare_runs.py`, no difference; 9,945 s in run 2 against 82,990 s
  in run 1). Beyond run 1, run 2 is at 232 bulk steps, N 26,608, 23,958 off, 11,391 s of runner time, still in the
  bulk phase.
- 10:06 UTC progress: run 2 alive at 343 bulk steps, N 21,716, 16,248 off, 25,291 s of runner time (no restart since
  06:06). The comparison with run 1 (`out-checkpoint/run.json` against `out2-checkpoint/run.json`) still finds no
  difference in the 225 steps run 1 has.
- 14:06 UTC progress: run 2 alive at 388 bulk steps, N 20,624, 14,904 off, 39,788 s of runner time (no restart since
  06:06). Comparison with run 1: still no difference in run 1's 225 steps. The bulk is slowing again: steps 375 to
  388 left off unchanged at 14,904 and took about 7,100 s.
- 18:06 UTC progress: run 2 alive at 400 bulk steps, N 20,580, 14,904 off, 53,009 s of runner time (no restart since
  06:06). Comparison with run 1: still no difference in run 1's 225 steps. Since 14:06, 12 steps lowered N by 44 and
  left off unchanged, about 1,100 s per step.
- 22:05 UTC progress: run 2 alive at 414 bulk steps, N 20,430, 14,820 off, 68,849 s of runner time (no restart since
  06:06). Comparison with run 1: still no difference in run 1's 225 steps. Since 18:06, 14 steps at about 1,140 s
  each.
- 25 October, 02:06 UTC progress: run 2 alive at 427 bulk steps, N 20,292, 14,820 off, 83,140 s of runner time (no
  restart since 06:06 on 24 October). Comparison with run 1: still no difference in run 1's 225 steps. Since 22:05, 13
  steps lowered N by 138 and left off unchanged.
- 06:06 UTC progress: run 2 alive at 441 bulk steps, N 20,196, 14,820 off, 97,207 s of runner time. Comparison with
  run 1: still no difference in run 1's 225 steps. Since 02:06, 14 steps lowered N by 96, off unchanged since
  20:05 on 24 October (14,820).
- 10:06 UTC progress: run 2 alive at 463 bulk steps, N 19,962, 14,528 off, 111,628 s of runner time. Comparison with
  run 1: still no difference in run 1's 225 steps. Since 06:06, 22 steps lowered N by 234 and off by 292.
- 14:06 UTC progress: run 2 alive at 476 bulk steps, N 19,894, 14,528 off, 125,491 s of runner time. Comparison with
  run 1: still no difference in run 1's 225 steps. Since 10:06, 13 steps lowered N by 68; off unchanged. The last
  four steps took about 1,800 s each.
- 18:06 UTC progress: run 2 alive at 484 bulk steps, N 19,822, 14,528 off, 140,477 s of runner time. Comparison with
  run 1: still no difference in run 1's 225 steps. Since 14:06, 8 steps (about 1,900 s each) lowered N by 72; off
  unchanged since 08:06.
- 22:07 UTC progress: run 2 alive at 491 bulk steps, N 19,754, 14,528 off, 153,940 s of runner time. Comparison with
  run 1: still no difference in run 1's 225 steps. Since 18:06, 7 steps (about 1,920 s each) lowered N by 68; off
  unchanged since 08:06.
