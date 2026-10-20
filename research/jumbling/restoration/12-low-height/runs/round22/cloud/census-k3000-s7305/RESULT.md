# Cloud line census-k3000-s7305 (round 22, line (d), round 6)

Status: in progress. The gate has passed; the census descent of seed 7305 is running.

## Question

At the 3000-twist stop of seed 7302, R22D-75's height-first rule played seven letters and stopped with 172 pieces at
height 6, and R22D-81's unlock rule lowered all of them with five letters. What happens on a fresh scramble: the
3000-twist census scramble of seed 7305, taken to its bulk stop, then R22D-75, then R22D-81?

## Scripts (sha256)

| Script | sha256 |
| --- | --- |
| `research/jumbling/restoration/11-scramble-census/scripts/census_seams.py` (unchanged) | `d8ee4bc209ea8056951e0fea913a587eb6d1f24aa9385c455582ffea00d4be54` |
| `research/jumbling/restoration/12-low-height/scripts/round22/r22d75_census.py` (source of the copy) | `c642ecdd7e591dec23d879483c437e4867c32d3a3bf9c6a1f1a34b4dedbc79e2` |
| `scripts/round22/cloud/census-k3000-s7305/r22d75_census_k3000.py` (copy: header and CPU limit (3590,3600) → (14390,14400) only) | `085d6cf497f7e792d9f62ec1fe04bfb3be58d8a1cfc1620c7fba05b8e0de0c0e` |
| draft `r22d81_rule.py` (commit 7fbbd31e, used unchanged from `round22/drafts/`, not committed here) | `a63a177ba0c3ef652a2e6be3c55835c46c0eeb9dd0c1feb0a0e64ecaddcb8455` |

Every command ran from the repository root with `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1
JUMBLING_HOME_CACHE=<a file outside the repository>`.

## Validation gate (passed)

Command:
`python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/drafts/r22d81_rule.py --record research/jumbling/restoration/11-scramble-census/runs/heights/ia-k3000-s7302.json --save <dir>/s7302-stop.pkl --prefix '[[64,15],[185,14],[357,12],[471,12],[263,14],[407,13],[523,12]]' --expect 1ff0061fc5863c8ccbc1d734b003ff49afa5b58969b0031e51fd1e5cf1a07d81`

Output: `gate_r22d81_s7302.txt`, exit code 0. Every expected line matched:

- LOADED: digest `015d839ecbb37bbdde74ae9391d5d7f274dea7fee4b4fdf0092d224eef907a1f`, 100,584 off (replay 1,335.8 CPU s).
- START: τ = 274, u = 3, targets [[104,13],[150,84],[274,159],[290,13],[311,13]], digest `1ff0061f…`,
  (n2, n4, n6) = (103,103, 1,019, 172), 104,294 off, N = 132,702.
- STEP: (124, 2) U → `9a227291…`; (274, 13) H6 → `c9cefc31…`; (46, 1) U → `7465613d…`; (46, 3) U → `198e936f…`;
  (104, 14) H6 → `bbef1c8e…`.
- HANDOFF: digest `bbef1c8ec0de78dead1473322a8c1236e48f9ccfe0017ce724461bb7f37ae299`, (n2, n4, n6) = (102,960, 936, 0),
  103,896 off, N = 132,930 (340.7 CPU s after the replay).

This is the first run of the draft's `--record` path; it reproduces the snapshot path's result.

## Census, R22D-75, R22D-81

Census part 1: `census_seams.py 3000 7305 18900 <dir>/census`. Scramble: 116,870 off, N = 141,328, left heights
{2: 113,826, 4: 3,044} over 1,849 off poses. The session's container restarted about 46 minutes into the descent and
killed the process. Its last saved record (`ia-k3000-s7305-part1.json`, result `running`) holds 17 steps, 49 twists,
107,565 off, N = 133,140, last step at 2,852 s; stdout in `census_part1.txt`.

Census part 2 (running): the script's own continuation option, unchanged script, with the remaining budget
18,900 − 2,852 = 16,048 s: `census_seams.py 3000 7305 16048 <dir>/census2 --start <dir>/part1/ia-k3000-s7305.json`.
It rebuilds the same scramble, replays the 49 twists and continues with the same rules. The descent rules do not read
the clock, so the restart can change only where the time limit falls (by the work lost after 2,852 s), not the
twists chosen. The part 2 record's `heights_start`, `N_start` and `off_start` describe the part 1 stop, not the
scramble; the scramble values are the ones above. Partial (committed while running): `census_part2.txt` and
`ia-k3000-s7305.json` (result `running`), 7 more conj steps by 7,330 s, N = 132,718, 107,565 off.

Next step: when `census_seams.py 3000 7305 18900 <dir>/census` ends, copy its record and stdout here, then run
`r22d75_census_k3000.py` on the record, then the draft `r22d81_rule.py --record <record> --prefix <R22D-75 letters> --cpu 7200`.

## Not run

No Windows, DirectX, native or performance checks; headless Linux research code only. Times are cloud timings of
research code, not performance evidence.
