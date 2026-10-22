# D2, seed 7303 from its 3000-twist scramble (development line d2-s7303)

Status: **in progress** (progress note, 22 October 2026, 13:10 UTC). The first run was killed by a container restart;
a second run of the same command started at 07:08 UTC. No check has run yet.

## Question

PLAN.md section 4, D2: seed 7303 has no committed stop, so the runner starts from the 3000-twist scramble itself.
How far does the bulk get in 18,900 s, and what do height (`r22d75`, `r22d81`, `h4`), high, tail and lattice do after it?

## Command (from the repository root, commit `35c3d4f`)

    S=research/jumbling/restoration/13-tutorial-runner/scripts
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/runner.py --scramble 3000 7303 \
      --out <dir>/d2-s7303 --cache <dir>/cache --bulk-seconds 18900 --bulk-step-seconds 18900 \
      --height-seconds 12000 --height-cpu 12000 --pair-shards 3 --kernel-seconds 3600

The run was started detached at 04:05 UTC in development mode, the default.

## Start (checked against stage 11)

Start digest `ee1a56a8bef6b56dbb3debc117b394c1d43260497c678c3a2838ae0b17a76822`, 116,111 pieces off, left heights
{2: 114,470, 4: 1,552, 6: 89}. This matches the 3000-twist table of `11-scramble-census/README.md` (seed 7303).

## Run 1 (killed by a container restart; partial record in `killed-1/`)

The scramble build took about 70 minutes; the bulk began at about 05:15 UTC, so its 18,900 s clock ends at about
10:30 UTC.

- 74 bulk steps so far: 69 `conj`, 5 `cover`; 212 twists. The first step (`cover`, 1 letter) lowered the off count
  from 116,111 to 115,382 and N from 129,978 to 129,660.
- After step 73 (`conj`, 3 letters, 158 s): digest `923c9f89…`, 99,063 off, N 115,896, heights
  {2: 97,798, 4: 1,176, 6: 89}.
- The record's `phases.bulk.result` reads `budget` while the bulk runs. It is the value the runner writes after each
  step, not the bulk's final result.
- `bulk-progress.json`: `{"step_started": 9969.249017724}` (step 74 under way). `logs/bulk.log` is empty.

Times are cloud timings of research code on a shared headless Linux machine, not performance evidence.

Between 06:50 and 07:08 UTC the session's container was restarted. The restart killed the runner (no exit code,
no `runner.rc`; `runner.out` is empty) during bulk step 82. The runner has no resume option. Its last `run.json`,
written after step 81, is kept unchanged in `killed-1/d2-s7303/` (with `tables/bulk-end.json` of that state):

- 82 bulk steps: 77 `conj`, 5 `cover`; 236 twists; 6,614 s of step time;
- after step 81: digest `b431232b…`, 98,912 off, N 115,540, heights {2: 97,647, 4: 1,176, 6: 89};
- `bulk-progress.json` at the kill: `{"step_started": 11007.014704082}`;
- `phases.bulk.result` reads `budget`, the value written after each step; the bulk had not ended.

This partial record was not checked. It is evidence of the interruption, not a result.

## Run 2 (same command, fresh `--out` and `--cache`, started 07:08 UTC)

Running. Same start as run 1 (`ee1a56a8…`, 116,111 off, {2: 114,470, 4: 1,552, 6: 89}).

Progress at 09:10 UTC: 25 bulk steps (24 `conj`, 1 `cover`), 73 twists; after step 24 the state is `099feb02…`,
104,139 off, N 120,434, heights {2: 102,630, 4: 1,420, 6: 89}. `bulk-progress.json`:
`{"step_started": 7298.523741719}`; `logs/bulk.log` is empty. All 25 steps are identical to run 1's first 25 (rule,
word sha256 and digest after each step). Run 2 reaches each step later than run 1 did, so its bulk clock will end
after fewer steps.

Progress at 11:10 UTC: 75 bulk steps (70 `conj`, 5 `cover`), 215 twists; after step 74 the state is `89e3f27d…`,
99,063 off, N 115,820, heights {2: 97,798, 4: 1,176, 6: 89}. `bulk-progress.json`:
`{"step_started": 14364.328920746}`; `logs/bulk.log` is empty. All 75 steps are identical to run 1's first 75.

Progress at 13:10 UTC: 113 bulk steps (106 `conj`, 7 `cover`), 325 twists; state `414408cd…`, 97,709 off, heights
{2: 96,444, 4: 1,176, 6: 89}. `bulk-progress.json`: `{"step_started": 21577.624746481}`; `logs/bulk.log` is empty.
Run 2's scramble build took about 1 h 46 min (start table written 08:54 UTC), so its bulk clock of 18,900 s ends at
about 14:09 UTC.

## Next step

Wait for the runner to exit (`runner.rc`), then copy the output directory here, run `check_run.py`, and complete this
file with the full phase and step tables.

## Checks not run

Windows, DirectX, the native host, input, long-session and performance checks: none (headless Linux only).
