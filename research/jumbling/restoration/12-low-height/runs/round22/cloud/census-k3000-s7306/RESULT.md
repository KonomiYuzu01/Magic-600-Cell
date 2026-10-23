# Cloud line census-k3000-s7306 (round 22, line (d), round 6)

Status: finished. Every step of the line was run.

## Question

At the 3000-twist census stop of seed 7302, R22D-75's height-first rule played seven letters and stopped with 172
pieces at height 6, and R22D-81's unlock rule lowered all of them with five letters. What happens on a fresh
3000-twist scramble (seed 7306)?

## Summary

| Item | Value |
| --- | --- |
| Seed | 7306 (I_a menu, 3000 scramble twists) |
| Census stop reason | `time-limit` (18,900 s budget) |
| Census descent twists | 270 (92 steps: 89 `conj`, 3 `cover`) |
| Scramble: off, N | 123,181; 149,224 |
| Census stop: off, N | 108,823; 137,562 |
| Left heights at the scramble | {2: 118,426, 4: 4,726, 6: 29} |
| Left heights at the stop | {2: 106,032, 4: 2,762, 6: 29} |
| R22D-75 letters | (208,12) (76,13) (90,12) (109,12) (233,15) (387,15) (485,13) (403,13) (512,15) |
| R22D-75 end | STUCK (all single letters exhausted) after 9 letters; n6 = 0 after its first letter |
| R22D-81 letters | none |
| R22D-81 end | HANDOFF at once (0 steps): R22D-75 had already reached n6 = 0 |
| Final digest | 16c4aefe2f15a043693937cae1d320b8a820e2151a744cf9ad8df9d11e36d96b |
| Final n2, n4, n6 | 108,641; 2,591; 0 |
| Final off, N | 111,232; 140,018 |
| Validation gate (seed 7302 stop) | passed: every expected line matched |

At this scramble the census stop had only 29 pieces at height 6 (seed 7302's had 172 after R22D-75), and R22D-75's
first letter (208,12) lowered all 29 to height 4. R22D-75 then lowered n4 from 2,793 to 2,591 with eight more letters
and stopped stuck; off and N rose over its nine letters (off 108,823 -> 111,232, N 137,562 -> 140,018). The state
still has 2,591 pieces at height 4 and 108,641 at height 2.

A state-only rule reaching n6 = 0 at one scramble is not coverage: this is one more data point (two scrambles in
all, seeds 7302 and 7306), not evidence that the rules lower every height-6 piece at every 3000-twist stop.

## Scripts and commands

All runs from the repository root as `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1
JUMBLING_HOME_CACHE=<file outside the repository> python3 -I -B -u <script> ...`.

| Script | sha256 |
| --- | --- |
| `research/jumbling/restoration/11-scramble-census/scripts/census_seams.py` (unchanged) | d8ee4bc209ea8056951e0fea913a587eb6d1f24aa9385c455582ffea00d4be54 |
| draft `r22d81_rule.py` (commit 7fbbd31e, unchanged) | a63a177ba0c3ef652a2e6be3c55835c46c0eeb9dd0c1feb0a0e64ecaddcb8455 |
| `scripts/round22/r22d75_census.py` (source of the copy) | c642ecdd7e591dec23d879483c437e4867c32d3a3bf9c6a1f1a34b4dedbc79e2 |
| `scripts/round22/cloud/census-k3000-s7306/r22d75_census_k3000.py` | 977d88dfed22c56c0563f40e9a48a0526d00c7524dab3552d46539e7aece0a49 |

The drafts directory was checked out read-only from commit 7fbbd31e and all six sha256 sums matched the line's list.
The draft was not edited; it was run from a byte-identical copy (same sha256) kept outside the repository, with the
repository root as the working directory, so its relative imports resolved to the repository. The drafts are not
committed on this branch.

`r22d75_census_k3000.py` differs from `r22d75_census.py` only in its header and its CPU limit,
`(3590,3600)` -> `(14390,14400)`; the rule and its 1,000 CPU-second budget are unchanged. It ended STUCK, not
BUDGET, so the 6,000 CPU-second rerun was not needed.

1. Census: `census_seams.py 3000 7306 18900 <dir>/census` -> `census.txt`, `ia-k3000-s7306.json` (45 KB, not compressed).
2. Gate: `r22d81_rule.py --record research/jumbling/restoration/11-scramble-census/runs/heights/ia-k3000-s7302.json --save <dir>/s7302-stop.pkl --prefix '[[64,15],[185,14],[357,12],[471,12],[263,14],[407,13],[523,12]]' --expect 1ff0061fc5863c8ccbc1d734b003ff49afa5b58969b0031e51fd1e5cf1a07d81` -> `gate-s7302.txt`.
3. R22D-75: `r22d75_census_k3000.py research/jumbling/restoration/12-low-height/runs/round22/cloud/census-k3000-s7306/ia-k3000-s7306.json` -> `r22d75_census_k3000.txt`.
4. R22D-81: `r22d81_rule.py --record research/jumbling/restoration/12-low-height/runs/round22/cloud/census-k3000-s7306/ia-k3000-s7306.json --save <dir>/stop.pkl --prefix '[[208,12],[76,13],[90,12],[109,12],[233,15],[387,15],[485,13],[403,13],[512,15]]' --cpu 7200` -> `r22d81_rule.txt`.

Snapshot .pkl files were written outside the repository and are not committed.

## Validation gate (seed 7302 stop): passed

Output: `gate-s7302.txt`. This was the first run of the draft's `--record` path. Every expected line matched:

- LOADED digest 015d839ecbb37bbdde74ae9391d5d7f274dea7fee4b4fdf0092d224eef907a1f, off 100,584.
- START tau 274, u 3, targets [[104,13],[150,84],[274,159],[290,13],[311,13]]; n2_n4_n6 [103103, 1019, 172], off 104,294, N 132,702.
- STEP (124,2) U; (274,13) H6; (46,1) U; (46,3) U; (104,14) H6.
- HANDOFF digest bbef1c8ec0de78dead1473322a8c1236e48f9ccfe0017ce724461bb7f37ae299, n2_n4_n6 [102960, 936, 0], off 103,896, N 132,930.

## Census of seed 7306

Scramble (3000 twists): off 123,181, N 149,224, left heights {2: 118,426, 4: 4,726, 6: 29}, 2,735 off poses.
Stop: `time-limit` after 270 descent twists in 92 steps (89 `conj`, 3 `cover`). At the stop: off 108,823,
N 137,562, grip N 20,501, 28 misaligned grips, left heights {2: 106,032, 4: 2,762, 6: 29}, 2,753 off poses.

## R22D-75 on the stop

START digest 9ebdc4904438ffec5eb18ec35f71ee01d3e037cae70587a786335ce095b3c83e, n6_n4_n2 [29, 2762, 106032],
off 108,823, N 137,562 (replay of 3,270 twists checked against the record's off count).

| Step | Letter | Digest after | n6, n4, n2 | off | N |
| --- | --- | --- | --- | --- | --- |
| 1 | (208,12) | 25adf6cbc3d63b89214f7d3ee49f1d6cbc1645f0ba2503072fbd2b09d90091c1 | 0, 2793, 106277 | 109,070 | 137,838 |
| 2 | (76,13) | e9503aa9d5b5a3558ee073e0b811b9200280d9b6a71136ea420c1aa80478898c | 0, 2688, 106958 | 109,646 | 138,226 |
| 3 | (90,12) | 01754de30bd79b32cdaf38272852b6fc451e557eb5ae3d89b6a7dfb3fe1973b6 | 0, 2663, 108441 | 111,104 | 138,822 |
| 4 | (109,12) | 9548bea646b2e3418b2edf9458819d3fcbb2428f1674a7281180cda9d6857d1a | 0, 2648, 108028 | 110,676 | 138,674 |
| 5 | (233,15) | fd310293e2dcc909b2d352aae05aca26bbe58e2171276d63fbbc09a3ff91d448 | 0, 2640, 108344 | 110,984 | 138,952 |
| 6 | (387,15) | 1ebe4c0b49c9977e673d1d84e7d440709bc8d61d288709b0f0824a04a99c26e2 | 0, 2637, 108633 | 111,270 | 139,262 |
| 7 | (485,13) | 11ec47a44f0b6abbe98d142715894c1f3d32f5269be9196af507155811e6b333 | 0, 2591, 109269 | 111,860 | 139,644 |
| 8 | (403,13) | eeb93181a0521708fd79ff86b255cfe9c9e3ff7616dd36d432d19df0bf6658cd | 0, 2591, 108838 | 111,429 | 139,736 |
| 9 | (512,15) | 16c4aefe2f15a043693937cae1d320b8a820e2151a744cf9ad8df9d11e36d96b | 0, 2591, 108641 | 111,232 | 140,018 |

End: STUCK, all single letters exhausted, digest 16c4aefe2f15a043693937cae1d320b8a820e2151a744cf9ad8df9d11e36d96b,
n6_n4_n2 [0, 2591, 108641], off 111,232, N 140,018. Pruning counts per step are in `r22d75_census_k3000.txt`.

## R22D-81 after R22D-75

LOADED digest 9ebdc4904438ffec5eb18ec35f71ee01d3e037cae70587a786335ce095b3c83e, off 108,823 (the census stop).
After the nine-letter prefix: START tau null, u 0, empty target table, digest
16c4aefe2f15a043693937cae1d320b8a820e2151a744cf9ad8df9d11e36d96b, n2_n4_n6 [108641, 2591, 0]. HANDOFF at once with
0 steps (no STEP and no U_ITEMS line): R22D-75 had already reached n6 = 0, so R22D-81 had nothing to do.

## Times (cloud timings of research code, not performance evidence)

- Census: 23,223.5 s wall clock in all (scramble build plus the 18,900 s descent budget).
- Gate: 1,395.7 CPU s replay, 365.0 CPU s rule.
- R22D-75: 1,623.8 CPU s replay, 550.8 CPU s rule.
- R22D-81: 1,583.7 CPU s replay, 23.5 CPU s rule.

## Not run, not finished

- No Windows, DirectX, input, long-session or performance check; only headless Linux research scripts.
- The 6,000 CPU-second R22D-75 rerun was not needed (R22D-75 ended STUCK, not BUDGET).
- Nothing was run beyond this line's six steps. The 2,591 height-4 pieces at the stuck state are open; lowering them
  is not this line's question.
