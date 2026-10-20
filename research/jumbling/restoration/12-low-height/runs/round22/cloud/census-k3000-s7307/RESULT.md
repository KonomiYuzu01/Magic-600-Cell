# Cloud line census-k3000-s7307 (round 22, line (d), round 6)

Status: finished. Research only, headless Linux; exact geometry.

## Question

What happens on a fresh 3000-twist scramble (seed 7307)? Where does the census descent stop, how far does R22D-75's
height-first rule get from there, and does R22D-81's unlock rule then reach n6 = 0? Before that, does the draft
R22D-81's new `--record` path reproduce the integrator's result at the 3000-twist stop of seed 7302?

## Summary

| Item | Value |
| --- | --- |
| Seed | 7307 (I_a menu, 3000 scramble twists) |
| Scramble | 119,518 off, N = 136,918, left heights {2: 117,245, 4: 2,263, 6: 10} |
| Census stop reason | `time-limit` (18,900 s wall-clock budget) |
| Census descent twists | 734 (252 steps) |
| At the census stop | 96,236 off, N = 112,776, digest `ba5371aebe00ac9411567545fe79e90fa3d2487d000d2c26d45613924d4e33ff` |
| Left heights at the stop | {2: 94,691, 4: 1,535, 6: 10} |
| R22D-75 letters | (275, 14), (516, 13), (50, 15), (474, 13), (494, 15), (147, 14) |
| R22D-75 end | STUCK (all single letters exhausted) after 6 letters; n6 = 0 already after the second letter |
| R22D-81 letters | none: START with τ = none, u = 0, empty target table, then HANDOFF at once (0 steps) |
| R22D-81 end | HANDOFF |
| Final digest | `895b45de8a9a83604ca8bf988a3c9a287e404e59bd22a5004703d0852b252507` |
| Final (n2, n4, n6) | (98,984, 1,585, 0) |
| Final off, N | 100,569 off, N = 115,782 |
| Gate (seed 7302, `--record` path) | passed: every expected line matched |

At this stop the height-6 layer was small (10 pieces), and R22D-75 alone removed it with two letters. R22D-81 had
nothing to unlock, so it handed off at once. R22D-75 then continued on the height-4 layer for four more letters and
stopped STUCK with n6 = 0. Off and N rose from the census stop (96,236 → 100,569 off, N 112,776 → 115,782), as the
rule lowers heights rather than off or N.

A state-only rule that reaches n6 = 0 at one scramble is not coverage. This line adds one scramble (seed 7307) to the
one before it (seed 7302), and at this one the unlock rule was not exercised at all.

## Commands and scripts

Every command ran from the repository root with
`OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 JUMBLING_HOME_CACHE=<a file outside the repository>`.

| Script | sha256 |
| --- | --- |
| `research/jumbling/restoration/11-scramble-census/scripts/census_seams.py` (unchanged) | d8ee4bc209ea8056951e0fea913a587eb6d1f24aa9385c455582ffea00d4be54 |
| `research/jumbling/restoration/12-low-height/scripts/round22/r22d75_census.py` (source of the copy) | c642ecdd7e591dec23d879483c437e4867c32d3a3bf9c6a1f1a34b4dedbc79e2 |
| `scripts/round22/cloud/census-k3000-s7307/r22d75_census_k3000.py` (this line's copy) | ccd314bad0cb0dbc7bcdd277734a1340fcf4fb77627f79b5d49802e5ccb28de7 |
| draft `scripts/round22/drafts/r22d81_rule.py` (commit 7fbbd31e, run unchanged) | a63a177ba0c3ef652a2e6be3c55835c46c0eeb9dd0c1feb0a0e64ecaddcb8455 |

The other draft sums were checked against the hand-off list and matched. The copy `r22d75_census_k3000.py` differs from
`r22d75_census.py` only in its header and its CPU limit, `(3590,3600)` → `(14390,14400)`. The rule and its
1,000 CPU-second budget after the replay are unchanged. The second run with a 6,000 CPU-second budget was not needed:
the run ended STUCK, not BUDGET, at 292 CPU s.

1. Census: `python3 -I -B -u research/jumbling/restoration/11-scramble-census/scripts/census_seams.py 3000 7307 18900 <dir>/census`
   → `ia-k3000-s7307.json` (the record, uncompressed, 67 kB) and `census.txt` (stdout).
2. Gate: `python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/drafts/r22d81_rule.py --record research/jumbling/restoration/11-scramble-census/runs/heights/ia-k3000-s7302.json --save <dir>/s7302-stop.pkl --prefix '[[64,15],[185,14],[357,12],[471,12],[263,14],[407,13],[523,12]]' --expect 1ff0061fc5863c8ccbc1d734b003ff49afa5b58969b0031e51fd1e5cf1a07d81`
   → `gate_r22d81_s7302.txt`.
3. R22D-75: `python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/cloud/census-k3000-s7307/r22d75_census_k3000.py research/jumbling/restoration/12-low-height/runs/round22/cloud/census-k3000-s7307/ia-k3000-s7307.json`
   → `r22d75_census_k3000.txt`.
4. R22D-81: `python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/drafts/r22d81_rule.py --record research/jumbling/restoration/12-low-height/runs/round22/cloud/census-k3000-s7307/ia-k3000-s7307.json --save <dir>/stop.pkl --prefix '[[275,14],[516,13],[50,15],[474,13],[494,15],[147,14]]' --cpu 7200`
   → `r22d81_after_r22d75.txt`.

The snapshot files (`.pkl`) were not committed.

## Validation gate (passed)

This was the first run of the draft's `--record` path. Every expected line matched:
- LOADED `015d839ecbb37bbdde74ae9391d5d7f274dea7fee4b4fdf0092d224eef907a1f` with 100,584 off.
- START τ = 274, u = 3, targets [[104,13],[150,84],[274,159],[290,13],[311,13]].
- Five STEP lines: (124,2) U, (274,13) H6, (46,1) U, (46,3) U, (104,14) H6.
- HANDOFF `bbef1c8ec0de78dead1473322a8c1236e48f9ccfe0017ce724461bb7f37ae299` with (n2, n4, n6) = (102,960, 936, 0),
  103,896 off, N = 132,930.

## R22D-75 at the seed 7307 stop

The replay of the 3,734 twists (3000 scramble and 734 descent) checked every twist as admissible and certain, and the
off count against the record.

| Step | Letter | Digest | (n6, n4, n2) | Off | N |
| --- | --- | --- | --- | --- | --- |
| start | – | `ba5371aebe00ac9411567545fe79e90fa3d2487d000d2c26d45613924d4e33ff` | (10, 1,535, 94,691) | 96,236 | 112,776 |
| 1 | (275, 14) | `a342049d030b4ed548866cd132bfaa46bc1f9cdddccbea46cc08e94ea590dae4` | (7, 1,538, 95,034) | 96,579 | 113,088 |
| 2 | (516, 13) | `b39821556608886f6f8112bc22b7a9bbae5a696d14c235c525326661aa85f6f5` | (0, 1,940, 95,398) | 97,338 | 113,566 |
| 3 | (50, 15) | `3b6790a1ecc893a7610000d13f18a8f570c9f6ab7195129303b5f61184aa6ba6` | (0, 1,742, 96,353) | 98,095 | 114,000 |
| 4 | (474, 13) | `7cc2722e38f136be454375dae385374e09d56a00748fc15518c17bdf10f0b7db` | (0, 1,696, 98,635) | 100,331 | 114,684 |
| 5 | (494, 15) | `f5a85c133450a23fd90811e63dd0f0f32b4a242f934c6a4461906d69440e77da` | (0, 1,585, 99,175) | 100,760 | 115,160 |
| 6 | (147, 14) | `895b45de8a9a83604ca8bf988a3c9a287e404e59bd22a5004703d0852b252507` | (0, 1,585, 98,984) | 100,569 | 115,782 |
| end | STUCK: all single letters exhausted | `895b45de…` | (0, 1,585, 98,984) | 100,569 | 115,782 |

The pruning counts for each step are in `r22d75_census_k3000.txt`.

## R22D-81 after R22D-75

LOADED `ba5371ae…` with 96,236 off, matching the census stop. START at `895b45de…` with τ = none, u = 0, an empty target
table and (n2, n4, n6) = (98,984, 1,585, 0). HANDOFF at once with 0 steps, so no U_ITEMS vectors were printed.
R22D-75 had already reached n6 = 0.

## Not run, not finished

- Only headless Linux checks ran. No Windows, DirectX, native host or performance check was run, and nothing here is
  evidence for any of them.
- The census stopped on its time limit, not on `stuck`, so the descent might go further with more time.
- The height-4 layer (1,585 pieces) at the final state was not attacked: R22D-81 hands off at n6 = 0, and lowering
  height 4 is outside this line.

## Timings (cloud timings of research code, not performance evidence)

| Run | Time |
| --- | --- |
| Census | 23,280 s wall clock in total (the `seconds` field of the record), scramble build included |
| Gate | 1,419 CPU s replay and 376 CPU s for the rule |
| R22D-75 | 1,591 CPU s replay and 292 CPU s for the rule |
| R22D-81 | 1,608 CPU s replay and 14 CPU s for the rule |
