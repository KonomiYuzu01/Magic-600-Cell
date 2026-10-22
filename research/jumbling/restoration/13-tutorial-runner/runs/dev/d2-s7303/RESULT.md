# D2, seed 7303 from its 3000-twist scramble (development line d2-s7303)

Status: **in progress** (progress note, 22 October 2026, 06:50 UTC). The run has not ended; no check has run yet.

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

## Progress at 06:50 UTC

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

## Next step

Wait for the runner to exit (`runner.rc`), then copy the output directory here, run `check_run.py`, and complete this
file with the full phase and step tables.

## Checks not run

Windows, DirectX, the native host, input, long-session and performance checks: none (headless Linux only).
