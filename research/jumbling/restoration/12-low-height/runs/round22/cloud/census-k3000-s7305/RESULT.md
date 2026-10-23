# Cloud line census-k3000-s7305 (round 22, line (d), round 6)

Status: finished. The gate passed. The census of seed 7305 stopped on its time limit with no piece at height 6;
R22D-75 played 11 letters and stopped stuck at n4 = 1,419; R22D-81 handed off at once (n6 = 0, no letter).

## Question

At the 3000-twist stop of seed 7302, R22D-75's height-first rule played seven letters and stopped with 172 pieces at
height 6, and R22D-81's unlock rule lowered all of them with five letters. What happens on a fresh scramble: the
3000-twist census scramble of seed 7305, taken to its bulk stop, then R22D-75, then R22D-81?

## Summary

| Item | Value |
| --- | --- |
| Seed, menu, scramble | 7305, I_a, 3000 twists |
| Scramble | 116,870 off, N = 141,328, left heights {2: 113,826, 4: 3,044} (1,849 off poses) |
| Census stop reason | `time-limit` (run in two parts after a container restart; see below) |
| Census descent | 103 twists in 35 steps (1 cover, 34 conj) |
| Census stop | 105,437 off, N = 131,542, left heights {2: 103,140, 4: 2,297} (1,805 off poses), digest `273eab69dee9e3a2f4a3997a75f6b8d9ad16814fa26b941bf7a219247f9bab6d` |
| R22D-75 letters | (10, 12), (10, 15), (67, 14), (134, 13), (195, 14), (233, 15), (421, 12), (421, 12), (484, 15), (122, 12), (175, 14) |
| R22D-75 end | STUCK (all single letters exhausted) after 11 letters, 814.8 CPU s after the replay; not BUDGET, so no 6,000 s re-run |
| R22D-81 letters | none |
| R22D-81 end | HANDOFF at once (steps 0): n6 = 0 already at the census stop |
| Final digest | `cc5ef7ecf0510ea9a74fc52bafe324d6d8f58294c4b0e20b40b97b414dada062` |
| Final (n6, n4, n2), off, N | (0, 1,419, 108,453), 109,872 off, N = 134,346 |
| Gate (draft `--record` path at seed 7302) | passed, every expected line matched |

At this scramble no piece is at height 6 at any measured point: the heights are at most 4 at the scramble, at the
census stop and after R22D-75. So R22D-81's unlock families had nothing to do; this line does not exercise them.
R22D-75 lowered n4 from 2,297 to 1,419 (and raised off by 4,435 and N by 2,804 on the way) and then found no single
letter in any of its three families. The stop at n4 = 1,419 is the open item of this line: it is a height-4 stop,
not a height-6 one, so it lies outside R22D-81 as written.

A state-only rule reaching n6 = 0 at one scramble is not coverage. Here n6 = 0 was given by the scramble and the
census descent, not reached by a rule.

## Scripts (sha256)

| Script | sha256 |
| --- | --- |
| `research/jumbling/restoration/11-scramble-census/scripts/census_seams.py` (unchanged) | `d8ee4bc209ea8056951e0fea913a587eb6d1f24aa9385c455582ffea00d4be54` |
| `research/jumbling/restoration/12-low-height/scripts/round22/r22d75_census.py` (source of the copy) | `c642ecdd7e591dec23d879483c437e4867c32d3a3bf9c6a1f1a34b4dedbc79e2` |
| `scripts/round22/cloud/census-k3000-s7305/r22d75_census_k3000.py` (copy: header, and CPU limit (3590,3600) → (14390,14400); nothing else) | `085d6cf497f7e792d9f62ec1fe04bfb3be58d8a1cfc1620c7fba05b8e0de0c0e` |
| draft `r22d81_rule.py` (commit 7fbbd31e, run unchanged from `scripts/round22/drafts/`, not committed here) | `a63a177ba0c3ef652a2e6be3c55835c46c0eeb9dd0c1feb0a0e64ecaddcb8455` |

Every command ran from the repository root as `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1
JUMBLING_HOME_CACHE=<a file outside the repository> python3 -I -B -u <script> ...`, one cache file per job.

## Validation gate (passed)

`r22d81_rule.py --record research/jumbling/restoration/11-scramble-census/runs/heights/ia-k3000-s7302.json --save <dir>/s7302-stop.pkl --prefix '[[64,15],[185,14],[357,12],[471,12],[263,14],[407,13],[523,12]]' --expect 1ff0061fc5863c8ccbc1d734b003ff49afa5b58969b0031e51fd1e5cf1a07d81`

Output `gate_r22d81_s7302.txt`, exit code 0. Every expected line matched:

- LOADED: digest `015d839ecbb37bbdde74ae9391d5d7f274dea7fee4b4fdf0092d224eef907a1f`, 100,584 off (replay 1,335.8 CPU s).
- START: τ = 274, u = 3, targets [[104,13],[150,84],[274,159],[290,13],[311,13]], digest `1ff0061f…`,
  (n2, n4, n6) = (103,103, 1,019, 172), 104,294 off, N = 132,702.
- STEP: (124, 2) U → `9a227291…`; (274, 13) H6 → `c9cefc31…`; (46, 1) U → `7465613d…`; (46, 3) U → `198e936f…`;
  (104, 14) H6 → `bbef1c8e…`.
- HANDOFF: digest `bbef1c8ec0de78dead1473322a8c1236e48f9ccfe0017ce724461bb7f37ae299`, (n2, n4, n6) = (102,960, 936, 0),
  103,896 off, N = 132,930 (340.7 CPU s after the replay).

This first run of the draft's `--record` path reproduces the snapshot path's result.

## Census (step 1 and 3)

Part 1: `census_seams.py 3000 7305 18900 <dir>/census`. The session's container restarted during the descent and
killed the process. Its last saved record (`ia-k3000-s7305-part1.json`, result `running`) holds 17 steps and 49
twists: 107,565 off, N = 133,140, last step finished at 2,852 s of descent. Stdout: `census_part1.txt`.

Part 2: the script's own continuation option, unchanged script, with the remaining budget 18,900 − 2,852 = 16,048 s:
`census_seams.py 3000 7305 16048 <dir>/census2 --start <dir>/part1/ia-k3000-s7305.json`. It rebuilt the same scramble,
replayed the 49 twists (its start, N = 133,140 and 107,565 off, equals the part 1 stop) and continued with the same
rule order. In `research/jumbling/theory/seams.py` the clock is read only by the job driver, not by the rules, so the
restart changes only where the time limit falls (the descent work lost between 2,852 s and the kill is re-done), not
the twists chosen. Part 2 played 18 more steps (54 twists) and stopped with `time-limit` after its last step at
16,298 s. Total descent time about 2,852 + 16,298 = 19,150 s of wall clock, slightly over 18,900 because the limit is
checked between steps (steps took 500 to 1,200 s near the end).

Record: `ia-k3000-s7305.json` (37 KB, not gzipped; sha256
`c8a27051f90c7596a6271ed10bfe50ead90c0d8066cbf88d43c054e9ef43caa2`), holding the history, all 103 descent twists and
all 35 steps. Its `N_start`, `off_start` and `heights_start` ({2: 105,268, 4: 2,297}) describe the part 1 stop, not the
scramble; the scramble values are in the part 1 record and in the summary above. `heights_end` = {2: 103,140,
4: 2,297}, `grip_N_now` = 18,227, 24 misaligned grips. Stdout: `census_part2.txt`; both parts together in
`census.txt`.

## R22D-75 (step 4)

`r22d75_census_k3000.py research/jumbling/restoration/12-low-height/runs/round22/cloud/census-k3000-s7305/ia-k3000-s7305.json`
Output `r22d75.txt` (exit code 0). Replay of 3,103 twists in 2,205.5 CPU s; START at digest `273eab69…`,
(n6, n4, n2) = (0, 2,297, 103,140), 105,437 off, N = 131,542.

| Step | Letter | Digest | (n6, n4, n2) | Off | N |
| --- | --- | --- | --- | --- | --- |
| 1 | (10, 12) | `2e48ac8a43a2f1e8881a43f15c066a78ef89931c427b2693e486e9de3f0917be` | (0, 2,285, 103,776) | 106,061 | 132,014 |
| 2 | (10, 15) | `b5a4000406f84f86ef150cdd6b3eac99e807fbdff4879f287ce47a58648b1202` | (0, 2,186, 104,001) | 106,187 | 131,954 |
| 3 | (67, 14) | `b55863d1a1d3b0aafe0c6306c35f7151c8398a04be8b93fbef82ee6b725b7734` | (0, 2,130, 104,709) | 106,839 | 132,310 |
| 4 | (134, 13) | `ae2a01095cc8cdba218497d282698aac1c64e5162239914a2cd419f92018adc7` | (0, 1,966, 105,613) | 107,579 | 132,894 |
| 5 | (195, 14) | `b2b54ca184a2d2c5646700a8b08eb747d555b8735247d33569e28eb1b56dd305` | (0, 1,855, 106,326) | 108,181 | 133,152 |
| 6 | (233, 15) | `d8a9f6da77d3d38737f507c422d65f467bdba58b5abcfcb7296ed19708d8e8e4` | (0, 1,603, 107,004) | 108,607 | 133,562 |
| 7 | (421, 12) | `7c44c0d8a88f64e2af43dc5db28b625da90f31835a5837ce2c14c0bd575e9a12` | (0, 1,589, 107,080) | 108,669 | 133,888 |
| 8 | (421, 12) | `662d793ee659eaa698f4d84e2aa61575a8600a8882de4b21a57784f34bcc4cfc` | (0, 1,452, 107,815) | 109,267 | 134,024 |
| 9 | (484, 15) | `5c7f89ecd059d55b581c2364ba5c5ad4770ec78ea9573d023e2506a956ec232b` | (0, 1,419, 108,840) | 110,259 | 134,540 |
| 10 | (122, 12) | `491b45c8720083a97b7b8b53225ce17757208130a5866172de70a5a9a6328868` | (0, 1,419, 108,462) | 109,881 | 134,282 |
| 11 | (175, 14) | `cc5ef7ecf0510ea9a74fc52bafe324d6d8f58294c4b0e20b40b97b414dada062` | (0, 1,419, 108,453) | 109,872 | 134,346 |

End: STUCK, all single letters exhausted, at `cc5ef7ec…`, 814.8 CPU s after the replay (inside the 1,000 s budget).
Steps 1 to 9 lower n4 (the height-4 family); steps 10 and 11 keep n4 and lower n2. Letter (421, 12) is played twice
in a row; each time it is the first lowering letter in the rule's order. The pruning counts per step are in
`r22d75.txt` (23 to 25 clean caps of 600 at every step).

## R22D-81 (step 5)

`r22d81_rule.py --record <runs dir>/ia-k3000-s7305.json --save <dir>/stop.pkl --prefix '[[10,12],[10,15],[67,14],[134,13],[195,14],[233,15],[421,12],[421,12],[484,15],[122,12],[175,14]]' --expect cc5ef7ecf0510ea9a74fc52bafe324d6d8f58294c4b0e20b40b97b414dada062 --cpu 7200`
(the `--expect` option, the draft's own digest check, is the only addition to the line's command). Output
`r22d81.txt` (exit code 0): LOADED `273eab69…`, 105,437 off (replay 2,363.4 CPU s); START with τ = none, u = 0,
empty target table, (n2, n4, n6) = (108,453, 1,419, 0), 109,872 off, N = 134,346; HANDOFF with 0 steps at
`cc5ef7ec…`. No U_ITEMS line: no cap was a target. It handed off at once because R22D-75 (indeed the census stop)
already had n6 = 0.

## Not run, not finished

- No Windows, DirectX, native or performance check; headless Linux research code only.
- No rule beyond R22D-75 and R22D-81 was applied at the height-4 stop `cc5ef7ec…`; lowering its 1,419 height-4 pieces
  is outside this line.
- The snapshot files written by `--save` were deleted and not committed.

Times are cloud timings of research code, not performance evidence.
