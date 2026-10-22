# D1, seed 7101 at depth 40, acceptance mode (development line d1-acc-s7101)

Status: **in progress** (progress note, 22 October 2026, 12:13 UTC; bulk stuck at 97 steps, tail running; run 1 was interrupted at 97 steps, run 2 restarted at 08:12 UTC). The run has not ended; the check has not run.

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

## Not run

- `check_run.py` (it runs after the runner ends).
- Windows, DirectX, the native host, input, long-session and performance checks: none.
