# D1, seed 7101 at depth 40, acceptance mode (development line d1-acc-s7101)

Status: **finished** (25 October 2026, 05:20 UTC). Run 4 ended `stuck` in the tail at 158 off; `check_run.py` passes its record with verdict `stuck`. Runs 1 to 3 are interrupted or stopped records, kept below as history.

## Final result (run 4)

**Question answered:** in acceptance mode, the tutorial runner does not solve seed 7101 at depth 40. It plays 97 bulk
steps and 17 tail steps, then stops `stuck` in the tail with 158 pieces off, all at height 2. No tail rule and no
fallback word of length at most 1 applies to any of the five units it tried. The lattice phase is not reached.

**Command** (from the repository root, on commit `d8578e2`, which merges `origin/main` with PR #128):

    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/runner.py --scramble 40 7101 \
        --out research/jumbling/restoration/13-tutorial-runner/runs/dev/d1-acc-s7101/out4 \
        --cache <dir>/cache4 --mode acceptance > <dir>/runner4.out 2>&1; echo $? > <dir>/runner4.rc
    python3 -I -B $S/checkpoint.py --run research/jumbling/restoration/13-tutorial-runner/runs/dev/d1-acc-s7101/out4 \
        --pid <runner pid> --trailer '<co-author trailer>' --trailer '<session trailer>'
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/check_run.py \
        research/jumbling/restoration/13-tutorial-runner/runs/dev/d1-acc-s7101/out4 > <dir>/check.out 2>&1

Run 4 ran from 14:47 UTC on 23 October to about 03:09 UTC on 25 October, without a restart (no `resumes` in the
record). The runner exited with code 0; the check ran from 03:09 to 05:13 UTC and exited with code 0.

**Record:**

- `result` `stuck`, `acceptance.verdict` `stuck`, `config.component` null.
  - `certificate_steps` 0, `fallback_steps` 0, `probe` false, `timed_out_attempts` 0.
- `config.source` commit `d8578e2`, `dirty` true. When the run started, the only uncommitted path in the worktree was
  this line's untracked `out3/` directory (`git status --porcelain`). `config.code` is the research-code fingerprint
  that `--resume` checks.
- Start: scramble k 40, seed 7101, digest `be08cfaa…`, 90,441 off, all at height 2.

| phase | result | steps | twists | runner seconds | end |
|---|---|---|---|---|---|
| bulk | `stuck` | 97 | 223 | 9,791 (sum of steps) | `7e128820…`, 1,193 off {2: 1,193}, N 1,734 |
| height (r22d75, r22d81, h4) | `not_applicable` | 0 | 0 | | unchanged |
| high | `not_applicable` | 0 | 0 | | unchanged |
| tail | `stuck` | 17 | 397,280 | 103,294 (sum of steps) | `966dc8a9…`, 158 off {2: 158} |
| lattice | not entered | 0 | | | |

- Tail rules used: `zero_bad` 4, `pure` 1, `close` 5, `t1prime` 7.
- Total runner time 130,870 s. The final, unsuccessful tail attempt is the remaining ≈17,800 s.
- Every step's rule, length, off count and time are in `run4-steps.md`.

**Where it stopped** (`stuck`, phase `tail`, reason `no_rule`): digest `966dc8a96f7bf40b4c0361d7a26d9a3efbf62e6c59c1a6677cf5de9f8aefe661`,
158 off, all at height 2. The tail has no N (N is the bulk's misalignment count, 1,734 at the bulk's end). The record's
stuck units and their attempts:

| unit | size | zero_bad | pure | close (pure) | close (transport) | t1prime | fallback |
|---|---|---|---|---|---|---|---|
| 9013 | 61 | no word | not strict_prime | no V | no W_ok core | no V | bound exhausted (length 1) |
| 60085 | 33 | no word | not strict_prime | no V | no V | no V | |
| 81223 | 29 | no word | not strict_prime | no V | no V | no V | |
| 112734 | 25 | no word | not strict_prime | no V | no W_ok core | no V | |
| 16314 | 10 | no word | no word | no word | no W_ok core | no word | |

In acceptance mode the fallback runs only as a probe, on the first unit (9013). It found no word within
`fallback_max_length` 1, so `probe` is false and the run is `stuck`, not `coverage_failure`.

**Checker** (`check.jsonl`, the same JSON lines as `check.out`): every one of the 114 steps passes, replayed twist by
twist. The final line reads `pass` true, `verdict` `stuck`, `result` `stuck`, end digest `966dc8a9…`,
`unchecked_contracts` 0 and `lattice_checked` false.

**Agreement with earlier runs:**

- Run 4's 97 bulk steps agree with the census record `ia-k40-s7101.json` (`compare.py`, as for run 1): the same rule,
  twists, N and off, and the same 223 twists.
- Run 4's first 102 steps agree with run 3's (`out3-checkpoint/run.json`) in rule, letters, word sha256, before and
  after digest, and off. Run 3 ran on the code before PR #128. Its slower close was the only change, and it did not
  change any decision up to step 101. Run 3 has no step 102.
- Runs 1 and 2 agree with these over the steps they reached (97 bulk steps; run 2's five tail steps).

**Files in this directory:**

- `out4/`: run 4's record (21 MB). Two things are left out because they contain absolute scratch paths, and the
  record refers to neither: the runner's `attempts/` scratch tree (121 MB, per-attempt programs and tables) and the 18
  child logs `logs/attempts-<index>-units-units.log`. No file is over 20 MB, so nothing is gzipped.
- `runner.out`: the runner's stdout (its final JSON line).
- `check.out` and `check.jsonl`: the checker's output.
- `run4-steps.md`: the full step table.
- `out3-checkpoint/`: run 3's last checkpoint (step 102), kept as the record of run 3.
- `interrupted-run1/`, `interrupted-run2/`: the partial records of runs 1 and 2.
- `bulk-vs-census.md`: run 1's step-by-step comparison with the census.
- `out4-checkpoint/` is removed in the same commit that adds `out4/`, as the checkpoint procedure says.



## Question

What does the tutorial runner do in acceptance mode from the depth 40 scramble of seed 7101 (`PLAN.md`, section 4,
D1)? Do its bulk steps agree with the census record `11-scramble-census/runs/round2/ia-k40-s7101.json` (97 steps,
223 twists)?

## Commands (from the repository root)

Commit `35c3d4f` (`config.source.dirty` false in the record). `S=research/jumbling/restoration/13-tutorial-runner/scripts`.

    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/runner.py --scramble 40 7101 \
        --out <dir>/out --cache <dir>/cache --mode acceptance > <dir>/runner.out 2>&1; echo $? > <dir>/runner.rc

Started detached at 04:03 UTC. When it ends, the next steps are:

    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/check_run.py <dir>/out > <dir>/check.out 2>&1

Then copy `<dir>/out` here as `run/`, with `runner.out`, `check.out` and `check.jsonl` beside it.

## Bulk comparison with the census record

The comparison reads the runner's `run.json` and its word files, and the census record's `steps` and `moves`. For each
index it compares the rule, the twist count, N before and after, the off count after the step, and the twists
themselves. The census record's twists come from slicing its `moves` list by each step's `twists`.

- **All 97 census steps agree** with the runner's first 97 bulk steps: the same rule, twist count, N before and
  after, off after, and the same twists (all 223). The 97 steps are 27 `seam`, 7 `cover` and 63 `conj` steps (no
  `close`), in the census order. Indices 0 to 15 are `seam` and the last 39 (58 to 96) are `conj`. After index 95 N is 1,736, after index 96 N is 1,734, and 1,193
  pieces are off in both. The runner's state after index 96 has digest `7e128820…` (the census record has no
  digest).
- This is agreement with one earlier run over 97 steps, nothing more. Both runs use the same restorer
  (`research/jumbling/theory/seams.py`), so it does not test the cover's float tie-break (R22D-90) on another machine
  or library build.
- The census bulk order has `reopen` at the end; the runner's does not. No census step used `reopen`.
- Step times: the runner's last census-range steps took 149, 294, 643 and 1,694 s (indices 93 to 96). The census
  record gives cumulative seconds (3,610 s at index 96). These are cloud timings of research code on a shared headless
  Linux machine, not performance evidence.

Full step-by-step table: `bulk-vs-census.md`.

## Progress

- At 06:50 UTC (2 h 47 min of wall time), the bulk phase has 97 steps and 223 twists, at N 1,734 and 1,193 off. The
  98th bulk step (index 97) is running. It started about 770 s before this note. `bulk-progress.json` holds only
  `step_started` (a monotonic time); `logs/bulk.log` and `runner.out` are still empty.
- The census run stopped here (killed 49 minutes into a search, `11-scramble-census/README.md:61`). From this point
  on the runner is beyond every earlier record of this seed.

## Interruption and restart (08:12 UTC)

- The session's container restarted between 06:50 and 08:11 UTC and killed the first run (run 1) without an exit
  code. Its record stops after 97 bulk steps and 223 twists, at digest `7e128820…`, with the 98th bulk step (index
  97) unfinished. Its `run.json` shows the bulk `result` as `budget`, because the bulk child had not finished
  writing the phase. That is an artefact of the kill, not a result. `totals.seconds` is 9,225 s at the last save.
  The runner has no resume option.
- Run 1's output directory, as the kill left it, is in `interrupted-run1/` (2.2 MB, no cache). The bulk comparison
  above is from run 1.
- Run 2 started at 08:12 UTC: the same command, on the same commit, in fresh `--out` and `--cache` directories. The
  rest of this report is about run 2. A later container restart would cost the same again.

## Progress of run 2

- 10:12 UTC (2 h 0 min after the restart): run 2 has the same 97 bulk steps (`compare.py` against the census finds no
  difference; 223 twists) and the same digest `7e128820…` after index 96. It reached that point at 7,152 s of runner
  time, against 9,225 s in run 1. Index 96 took 1,248 s, against 1,694 s in run 1. The 98th bulk step (index 97)
  started about 75 s before this note. `bulk-progress.json` holds only `step_started`; `logs/bulk.log` and
  `runner.out` are empty. These are cloud timings of research code on a shared machine, not performance evidence.

- 12:13 UTC: the **bulk phase ended `stuck`** after 97 steps and 223 twists, at `7e128820…` with 1,193 off (all at
  height 2) and N 1,734. Every bulk rule failed at the 98th step. The last bulk save came at about 7,152 s and the
  tail's first attempt started at 12:08 UTC, so that search took roughly 1 h 55 min. **Height** (all three
  sub-phases) and **high** are `not_applicable`, as the plan expected for a state at height 2. The **tail** phase is
  running. Its first attempt (index 97) found 22 straddling blocks and works on unit `u2198` (block 2, 111 pieces)
  with a transport census (P 3, about 609,000 commutators after 249 s at the last log line).

## Run 2 interrupted (container restart between 15:39 and 21:02 UTC)

Run 2 was alive at 15:39 UTC (102 steps saved). At 21:02 UTC the container had just restarted (uptime 17 s), and the
runner was gone without an exit code. Run 2 started before the runner had `--resume`, so it cannot be resumed. Its
output directory, as the kill left it, is in `interrupted-run2/` (11 MB, no cache). One exception: the six child logs
`logs/attempts-<index>-units-units.log` are left out, because they contain absolute scratch paths. The record itself
(`run.json`) has none. Its `result` field reads `budget` only because the run never finished; that is not a result.

Its last save, at 20,509 s of runner time:

| index | phase | rule | letters | off after | seconds | unit |
|---|---|---|---|---|---|---|
| 0-96 | bulk | 27 seam, 7 cover, 63 conj | 223 in all | 1,193 | 7,152 in all | |
| 97 | tail | zero_bad | 60 | 1,082 | 1,368 | u2198 |
| 98 | tail | zero_bad | 48 | 971 | 1,376 | u60011 |
| 99 | tail | zero_bad | 96 | 860 | 1,323 | u149477 |
| 100 | tail | zero_bad | 24 | 759 | 1,387 | u106783 |
| 101 | tail | pure | 576 | 673 | 866 | u160130 |

- Phases: bulk `stuck` (97 steps, 223 twists); height and high `not_applicable`; tail running (5 steps, 804
  twists); lattice not reached.
- Step 102 was unfinished, on unit `u107733` (78 pieces). Its `pure` attempt had found a 24-letter word: off 673 to
  650, with 55 lattice pieces of the carrier knocked off as collateral, and `pure-check` agreed. The word's start
  digest is `e48ad986…` and its end digest `2f3203e7…`. The attempt was then closing that collateral with
  `close-pure-close`. Its last log line reads step 2, 46 left, at 5,287 s.
- Times are cloud timings of research code on a shared headless Linux machine, not performance evidence.

## Run 3 (with checkpoints)

From now on the line follows the integrator's checkpoint procedure (22 October 2026). The branch merges commit
`04057a4` of `claude/repair-pair-list`, which adds `--resume` to `runner.py` and `scripts/checkpoint.py`. Run 3
starts from the scramble in `out3/`, with `checkpoint.py` keeping a verified copy in `out3-checkpoint/`.

- Run 3, first start (21:15 UTC): a container restart killed it after 46 bulk steps and 404 s of runner time,
  before the first checkpoint round at 900 s. No checkpoint existed, so its directory was not kept (46 bulk steps,
  reproducible) and run 3 started again from the scramble at 22:08 UTC, with `checkpoint.py` beside it.

- Run 3, second start (22:08 UTC): killed the same way, after 45 bulk steps and 308 s, again before its first
  checkpoint (container uptime 23 s at the 23:05 check-in). Started a third time at 23:06 UTC, not kept.
- Observation for the integrator: run 2 lived from 08:12 to after 15:39 UTC while this session kept a background
  wait open (`until [ -e <dir>/runner.rc ]; do sleep 60; done`, 2 h timeout). Both run 3 starts after 21:05, with
  no wait open, died within minutes of the session's turn ending. That suggests the container is reclaimed when the
  session is idle. From the third start on, the session keeps that one background wait open again (it wakes only
  when the run ends or after 2 h).

- Run 3, third start (23:06 UTC) is the live run. `checkpoint.py` pushed checkpoints from step 59 onwards. A
  container restart at about 02:59 UTC killed it during the search for the 98th bulk step. The step-97 checkpoint
  was restored into an empty `out3/`, and the identical command with `--resume` was accepted. `run.json` records a
  continuation at step 97, after 8,773 s of runner time.
- 06:12 UTC progress: run 3's bulk ended `stuck` after 97 steps at `7e128820…`, as in run 2. Height and high are
  `not_applicable`. The tail has two steps so far, the same as run 2's first two: index 97 `zero_bad` 60 letters to
  1,082 off, and index 98 `zero_bad` 48 letters to 971 off. Runner time is 20,010 s.

- 12:08 UTC progress: run 3 is still running, now past the point where run 2 was killed. Tail steps 97 to 101 are
  the same as run 2's (rule, letters and off: 60/1,082, 48/971, 96/860, 24/759, then `pure` 576/673). Step 102,
  on unit `u107733`, is still unfinished. Its `pure` word knocked 55 lattice pieces off, and `close-pure-close` is
  putting them back one at a time. Its third repair step took 7,343 s (14,348 s after the second's 7,005 s) and left
  43. If the rate holds, that close alone needs many more hours. Saved runner time is 24,490 s at step 101; the
  latest checkpoint is step 102.

- **Run 3 stopped** at 14:47 UTC on 23 October, on the integrator's instruction, during step 102's close (43 left
  after its third repair step). It was killed by process id (helper first, then runner and its child). The latest
  checkpoint is step 102, and `out3/` and `out3-checkpoint/` are kept unchanged. The reason is PR #128 (merged into
  main as `e6b8be1`): it changed the tail rule close's greedy vacancy search (`gm/vacate2.py`) to evaluate candidates
  exactly and faster, with the same decisions. Run 3 cannot resume across that change, because the runner refuses a
  different code fingerprint.

## Run 4 (on main with PR #128)

- The branch merged `origin/main` with a merge commit (`d8578e2`). Run 4 started at 14:47 UTC on 23 October on
  that commit, with the run 3 command and only the run number changed: `--out <line dir>/out4`, a fresh
  `--cache`, and no `--resume`. `checkpoint.py` runs beside it for `out4`.
- Comparison with run 3: acceptance mode has no clocks, so steps 0 to 101 must agree in rule, letters, word sha256,
  before and after digest, and off. The first difference, if any, will be recorded here.

- 18:49 UTC progress: run 4 has 97 bulk steps (checkpoint step 97, runner time 9,812 s at the last save) and is in
  the search for the 98th bulk step. Compared with run 3 (`out3-checkpoint/run.json` against
  `out4-checkpoint/run.json`): all 97 steps agree in rule, letters, word sha256, before and after digest, and off.
  There is no difference so far.

- 22:50 UTC progress: run 4 has saved 102 steps (indices 0 to 101, runner time 26,373 s), and the checkpoint has all
  102. Compared with run 3, all 102 agree in rule, letters, word sha256, before and after digest, and off. There is
  no difference. Step 102 (unit `u107733`) is in its `close-pure-close` repair, with the same targets, lengths and
  gains as run 3's so far: target 60370 length 6 gain 4, target 60413 length 4 gain 5, target 60404 length 6 gain 3,
  leaving 43. Run 4 took 684 s for these three repair steps; run 3 took 14,348 s for the same three. These are cloud
  timings of research code on a shared machine, not performance evidence.

- 02:15 UTC on 24 October: **run 5 not started yet; run 4 kept alive, pending the integrator's confirmation.** The
  integrator's message of 02:13 UTC (PR #132, the exact bulk speed-up, merged as `7dbc4ae`) asks for run 4 to be
  stopped and restarted as run 5, because "run 4 is still in the bulk phase". The record says otherwise. Run 4's
  bulk ended `stuck` at 97 steps, and the checkpoint's 102 steps are 97 bulk steps plus tail steps 97 to 101. Its
  phases read bulk `stuck`, height and high `not_applicable`, tail running. PR #132 changes only the bulk, so a
  restart would repeat about 26,400 s of runner time (about 9,800 s of it in the bulk) to get back to step 102.
- Step 102 (unit `u107733`) is about to finish. `close-pure-close` stopped at 35 left with "no V" (greedy search
  11,760 s), then `close-pure-expand` found a 55,646-letter closing word (sha256 `da9bdf34…`). `close-pure-whole`
  plays the 24-letter pure word and the closing word from `e48ad986…` to `c8170209…`, with 595 off and no
  collateral left. Run 3 never reached this point.
- The next session step: if the integrator confirms run 5 anyway, stop run 4 after its next checkpoint (step 103)
  and start run 5 as instructed. Otherwise run 4 continues under the check-in procedure.

- 02:19 UTC: the integrator's follow-up of 02:18 UTC gives the check-in text for run 5, to be set "once run 5 is
  running". It does not answer the note above, so run 5 stays unstarted and the check-in prompt stays on run 4 until
  the integrator confirms. When run 5 starts, the routine's prompt is to be replaced with that text verbatim.

- 02:50 UTC progress: run 4 finished step 102 and checkpointed it (`4ae8ffd`, 103 steps). Step 102 is tail rule
  `close` on unit `u107733`: 111,316 letters, 673 to 595 off, 15,800 s. Run 4's first 102 steps (indices 0 to 101)
  agree with run 3's in every compared field; run 3 has no step 102. Runner time is 42,173 s. Step 103 (unit `u106`)
  has started. Run 5 is still not started; it waits for the integrator's confirmation (above).

- 03:01 UTC: the integrator withdrew the run 5 restart, because run 4 had already left the bulk (PR #132 changes only
  the bulk). Run 4 continues, and the check-in routine stays on run 4.

- 06:51 UTC progress: run 4 has checkpointed 108 steps (runner time 54,927 s). Steps 0 to 101 still agree with run 3
  in every compared field; run 3 has nothing after that. The tail steps since the last note:

  | index | rule | letters | off after | seconds |
  |---|---|---|---|---|
  | 103 | close | 820 | 523 | 2,178 |
  | 104 | t1prime | 964 | 460 | 1,419 |
  | 105 | close | 460 | 400 | 3,076 |
  | 106 | close | 1,144 | 342 | 3,435 |
  | 107 | t1prime | 974 | 297 | 2,646 |

- 10:51 UTC progress: run 4 has checkpointed 109 steps (runner time 57,724 s at the last save). Steps 0 to 101 still
  agree with run 3; run 3 has none after. Since the last note: index 108 `t1prime`, 1,240 letters, to 256 off,
  2,797 s. Step 109 (unit `u60412`) is in `close-pure-close`: at its tenth repair step, after 7,653 s, 51 are left.

- 14:52 UTC progress: run 4 still has 109 checkpointed steps (no new step since 10:51 UTC), and steps 0 to 101 still
  agree with run 3. Step 109 (unit `u60412`, 34 pieces) is closing. `close-pure-close` was followed by
  `close-pure-expand` and `close-pure-whole`, whose last log line (13:47 UTC) reports a closing play from `eb13b265…`
  to `625dd359…` with 222 off and no collateral. Its segment 2 is a 114,392-letter word. The runner itself is now
  playing and saving the step; step 102 took the same path.

- 18:52 UTC progress: run 4 has checkpointed 111 steps (runner time 91,227 s). Steps 0 to 101 still agree with run
  3; run 3 has none after. New since the last note: index 109 `close` on `u60412`, 228,796 letters, to 222 off,
  29,882 s; index 110 `t1prime`, 792 letters, to 189 off, 3,622 s. Step 111 (unit `u161418`) is running.

- 22:53 UTC progress: run 4 has checkpointed 113 steps (runner time 113,386 s). Steps 0 to 101 still agree with run
  3; run 3 has none after. New since the last note: index 111 `t1prime` on `u161418`, 44,844 letters, to 173 off,
  11,797 s; index 112 `t1prime`, 4,732 letters, to 163 off, 10,361 s. Step 113 is running. A `t1prime` attempt on
  unit `u9013` came first; the log now shows a transport census on unit `u60085`.

## Not run

- Windows, DirectX, the native host, input, long-session and performance checks: none. Times are cloud timings of
  research code on a shared headless Linux machine, not performance evidence.
- The lattice phase (and so `schedule11`): not reached, because the run stopped in the tail.
- The development-mode D1 run (`PLAN.md`, section 4): not part of this line.
