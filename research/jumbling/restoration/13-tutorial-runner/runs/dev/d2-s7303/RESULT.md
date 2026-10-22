# D2, seed 7303 from its 3000-twist scramble (development line d2-s7303)

Status: **finished** (22 October 2026). Run 1 was killed by a container restart and kept as an interrupted record.
Run 2 of the same command ran to its end: `result` `budget`, `acceptance.verdict` `component`. `check_run.py` passes
it. The tail and lattice phases did not run.

Times are cloud timings of research code on a shared headless Linux machine, not performance evidence.

## Question

PLAN.md section 4, D2: seed 7303 has no committed stop, so the runner starts from the 3000-twist scramble itself.
How far does the bulk get in 18,900 s? Then what do height (`r22d75`, `r22d81`, `h4`), high, tail and lattice do?

## Commands (from the repository root)

    S=research/jumbling/restoration/13-tutorial-runner/scripts
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/runner.py --scramble 3000 7303 \
      --out <dir>/d2-s7303 --cache <dir>/cache --bulk-seconds 18900 --bulk-step-seconds 18900 \
      --height-seconds 12000 --height-cpu 12000 --pair-shards 3 --kernel-seconds 3600
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/check_run.py <dir>/d2-s7303

Each command ran detached, with stdout and stderr to a file. Development mode is the default. Lattice restorer:
`schedule11`, the default. Runs 1 and 2 used separate fresh `<dir>`s.

## Source

- Run 1: `config.source` commit `35c3d4f`, `dirty` false.
- Run 2: `config.source` commit `13dcbd2`, `dirty` true. `13dcbd2` adds only this line's first progress note to
  `35c3d4f`. `dirty` comes from this line's own new files under `runs/dev/d2-s7303/`: the updated note and run 1's
  copied record. No script differs from `35c3d4f`, and the record's `config.scripts` holds the digests of the 49
  files.

## Start (checked against stage 11)

Both runs start at digest `ee1a56a8bef6b56dbb3debc117b394c1d43260497c678c3a2838ae0b17a76822`. The start has
116,111 pieces off, N 129,978 and left heights {2: 114,470, 4: 1,552, 6: 89}. This matches the 3000-twist table of
`11-scramble-census/README.md` (seed 7303: 116,111 off; 2: 114,470, 4: 1,552, 6: 89).

## Run 1 (killed; partial record in `killed-1/`)

- The run started at 04:05 UTC. The scramble build took about 70 minutes.
- Between 06:50 and 07:08 UTC the session's container was restarted. The restart killed the runner during bulk
  step 82: no exit code, no `runner.rc`, and an empty `runner.out`. The runner has no resume option.
- Its last `run.json` was written after step 81. It is kept unchanged with its words, trees and
  `tables/bulk-end.json`:
  - 82 bulk steps: 77 `conj` and 5 `cover`, 236 twists, 6,614 s of step time;
  - after step 81: `b431232b…`, 98,912 off, N 115,540, heights {2: 97,647, 4: 1,176, 6: 89};
  - `bulk-progress.json` at the kill: `{"step_started": 11007.014704082}`.
- `phases.bulk.result` reads `budget`, the value the runner writes after each step; the bulk had not ended.
- This record was not checked. It is evidence of the interruption, not a result.

## Run 2 (complete; `run2/`)

The run started at 07:08 UTC and exited with code 0 at about 15:26 UTC. The scramble build took about 1 h 46 min,
and the start table was written at 08:54 UTC. `totals.seconds` is 29,862.

**Reproducibility.** Run 2's first 82 steps are identical to run 1's 82: the same rule, the same word sha256 and the
same digest after every step.

| Item | Value |
| --- | --- |
| `result` | `budget` |
| `acceptance.verdict` | `component` (`certificate_steps` 0, `fallback_steps` 0, `timed_out_attempts` 1) |
| `config.component` | `bulk_budget` |
| steps / twists | 139 / 387 (bulk 382, height 5, high 0, tail 0, fallback 0, lattice none) |

### Phases

| Phase | Result | Steps | Twists | Start → end | Off, (n2, n4, n6) at the end |
| --- | --- | --- | --- | --- | --- |
| bulk | `timeout` (the 18,900 s clock) | 134 (124 `conj` of 3 letters, 10 `cover` of 1) | 382 | `ee1a56a8…` → `516f427d…` | 96,817, (95,559, 1,169, 89) |
| height / `r22d75` | `stuck` (`no_step`) | 5 (T0 1, T1 2, T2 2; 1 letter each) | 5 | `516f427d…` → `d01dc91e…` | 100,650, (99,634, 929, 87) |
| height / `r22d81` | `stuck` (`no_step`; `tau` 214, `u` 89) | 0 | 0 | `d01dc91e…` | 100,650, (99,634, 929, 87) |
| height / `h4` | `not_applicable` (n6 = 87 > 0) | 0 | 0 | `d01dc91e…` | same |
| height (whole) | `stuck` | 5 | 5 | `516f427d…` → `d01dc91e…` | same |
| high (R22D-25 kernel with D19) | `budget` (`kernel_clock`) | 0 | 0 | `d01dc91e…` | same |
| tail | not run (`result` null) | 0 | 0 | | |
| lattice | not run (`result` null) | 0 | | | |

- **Bulk.** In the first step (`cover`) the off count drops from 116,111 to 115,382. The bulk lowers the off count
  by 19,294 in 134 steps and n4 from 1,552 to 1,169; n6 stays at 89. Step times range up to 374 s, and the bulk's
  steps took 18,643 s in all.
- **Height.** `r22d75` played five single twists in about 13 s of step time. Its last attempt, 78 s, found no step.
  These five twists lower n4 from 1,169 to 929 and n6 from 89 to 87, but they raise the off count from 96,817 to
  100,650 (n2 from 95,559 to 99,634). `r22d81` found no step: `r22d81` took 77 s and `r22d81_pairs` 179 s.
- **High.** One kernel call ran, 3,614 s in all with 3,601 s inside the kernel. The kernel's clock ended it
  (`timed_out` true; stop family F2), and it played no step. Kernel ledger at the stop:
  - F1 `exhausted`: 0 tried; pruned 33,866 `blocked` and 1,534 `target_not_whole`;
  - F2 `budget`: 2,036 tried, with cursor unit 3547, c 9, x 1, d 24, y 35, z 31; pruned 1,107,198,189
    `blocked_first`, 80,063 `blocked_second`, 121 `other_off_pose` and 1,915 `lattice_pieces_off`;
  - F3, d19, S, ST and T1 `budget`, not reached.

  The kernel record's sha256 is `078bbbbe…`, and it is under `run2/d2-s7303/attempts/00139-high/kernel/`. Under
  section 1 of the README, a `budget` after the bulk ends the run `budget`, so the tail and the lattice phase were
  not entered.
- **End.** The record ends at `d01dc91edd1c73da399b8acb13a57b9f526da64a5d754993f42ae02856d51889`: 100,650 off,
  heights {2: 99,634, 4: 929, 6: 87}.

Every step's rule, length, time and the state after it are listed in `steps.md`.

### Check

`check_run.py run2/d2-s7303` (run on the original output directory) exited 0 after about 1 h 48 min. The time
includes rebuilding the 3000-twist start.

- All 139 steps pass.
- Summary line:
  - `pass` true, `result` `budget`, `verdict` `component`;
  - `end_digest` `d01dc91edd1c73da399b8acb13a57b9f526da64a5d754993f42ae02856d51889`;
  - `unchecked_contracts` 0;
  - `lattice_checked` false;
  - twists: bulk 382, height 5, high 0.
- The report is `run2/check.jsonl`, the checker's stdout. Its stderr, `run2/check.out`, is empty.

## Files

- `killed-1/d2-s7303/`: run 1's partial record, unchanged. `killed-1/runner.out` is empty.
- `run2/d2-s7303/`: run 2's whole output directory. `run2/runner.out` holds the runner's last line, and
  `run2/check.jsonl` and `run2/check.out` hold the check.
  - Two things are left out: the kernel's own cache file `attempts/00139-high/kernel/cache/home_centroids.npy`
    (5.7 MB) and the empty attempt directories, which Git does not keep. The cache is a regenerable centroid table,
    and the rules forbid committing caches.
  - No file is over 20 MB, so nothing was gzipped by hand.
- `steps.md`: the table of run 2's 139 steps.

## What was not run or did not finish

- Run 1 did not finish (killed by a container restart) and was not checked.
- Run 2's tail and lattice phases were not run: the high phase ended `budget` on the kernel's 3,600 s clock.
- No acceptance-mode run. The integrator's update of 22 October 2026 asked for no restart: the component runs at
  the other D2 stops already answer D2.
- Checks not run: Windows, DirectX, the native host, input, long-session and performance checks (headless Linux
  only).
