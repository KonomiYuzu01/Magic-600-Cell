# Cloud line census-k3000-s7306 (round 22, line (d), round 6)

Status: in progress. The validation gate has passed; the census descent of seed 7306 is running.

## Question

At the 3000-twist census stop of seed 7302, R22D-75's height-first rule played seven letters and stopped with 172
pieces at height 6, and R22D-81's unlock rule lowered all of them with five letters. What happens on a fresh
3000-twist scramble (seed 7306)?

## Scripts and commands

All runs from the repository root as `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1
JUMBLING_HOME_CACHE=<file outside the repository> python3 -I -B -u <script> ...`.

| Script | sha256 |
| --- | --- |
| `research/jumbling/restoration/11-scramble-census/scripts/census_seams.py` (unchanged) | d8ee4bc209ea8056951e0fea913a587eb6d1f24aa9385c455582ffea00d4be54 |
| draft `r22d81_rule.py` (commit 7fbbd31e, unchanged; run from a copy outside the repository with the same digest) | a63a177ba0c3ef652a2e6be3c55835c46c0eeb9dd0c1feb0a0e64ecaddcb8455 |
| `scripts/round22/r22d75_census.py` (source of the copy) | c642ecdd7e591dec23d879483c437e4867c32d3a3bf9c6a1f1a34b4dedbc79e2 |
| `scripts/round22/cloud/census-k3000-s7306/r22d75_census_k3000.py` | 977d88dfed22c56c0563f40e9a48a0526d00c7524dab3552d46539e7aece0a49 |

`r22d75_census_k3000.py` differs from `r22d75_census.py` only in its header and its CPU limit,
`(3590,3600)` -> `(14390,14400)`; the rule and its 1,000 CPU-second budget are unchanged.

1. Census: `census_seams.py 3000 7306 18900 <dir>/census`
2. Gate: `r22d81_rule.py --record research/jumbling/restoration/11-scramble-census/runs/heights/ia-k3000-s7302.json --save <dir>/s7302-stop.pkl --prefix '[[64,15],[185,14],[357,12],[471,12],[263,14],[407,13],[523,12]]' --expect 1ff0061fc5863c8ccbc1d734b003ff49afa5b58969b0031e51fd1e5cf1a07d81`

## Validation gate (seed 7302 stop): passed

Output: `gate-s7302.txt`. This was the first run of the draft's `--record` path. Every expected line matched:

- LOADED digest 015d839ecbb37bbdde74ae9391d5d7f274dea7fee4b4fdf0092d224eef907a1f, off 100,584 (replay 1,395.7 CPU s).
- START tau 274, u 3, targets [[104,13],[150,84],[274,159],[290,13],[311,13]]; n2_n4_n6 [103103, 1019, 172], off 104,294, N 132,702.
- STEP (124,2) U; (274,13) H6; (46,1) U; (46,3) U; (104,14) H6.
- HANDOFF digest bbef1c8ec0de78dead1473322a8c1236e48f9ccfe0017ce724461bb7f37ae299, n2_n4_n6 [102960, 936, 0], off 103,896, N 132,930 (365.0 CPU s after the start).

## Census of seed 7306

Scramble (3000 twists): off 123,181, N 149,224, left heights {2: 118,426, 4: 4,726, 6: 29}, 2,735 off poses.
The descent is running (wall-clock budget 18,900 s).

## Next steps

Record the census stop; run `r22d75_census_k3000.py` on the record; run `r22d81_rule.py` after R22D-75's letters.

## Not run

No Windows, DirectX, input, long-session or performance check. Times are cloud timings of research code, not
performance evidence.
