# Cloud line census-k3000-s7304 (theory round 22, line (d), round 6)

Question: on a fresh 3000-twist I_a scramble (seed 7304), where does the census descent stop, and how far do R22D-75's
single-letter height-first rule and then R22D-81's unlock rule get from that stop? Before that, a gate: the first run
of the R22D-81 draft's `--record` path must reproduce the integrator's seed 7302 result.

Research only, headless Linux, exact geometry (every letter admissible and certain on `research/jumbling/sim/`). No
Windows, DirectX, input or performance check was run or is claimed.

## Summary

| Item | Value |
| --- | --- |
| Seed | 7304 (I_a menu, 3000 scramble twists) |
| Census stop reason | `time-limit` (budget 18,900 s) |
| Census descent twists | 580 (198 rule steps) |
| At the scramble | 121,001 off, N = 139,584, heights {2: 118,951, 4: 2,050} |
| At the census stop | 95,569 off, N = 115,200, heights {2: 94,384, 4: 1,185}; digest `7e2304b2a4a053b8714dd806e43ff64c2de9e11f5ef2258f3b8a65c9c1a936d4` |
| R22D-75 letters | (170, 12), (395, 12), (500, 12), (540, 12), (217, 15), (148, 15) |
| R22D-75 end | STUCK (all single letters exhausted) after 6 letters |
| R22D-81 letters | none: n6 was already 0, so it hands off at once (HANDOFF, 0 steps) |
| Final digest | `ac127dc3488832b1d1b75a043d63e84b75addd1f4e73fe7e961407aea2ff0980` |
| Final (n2, n4, n6) | (95,674, 797, 0) |
| Final off, N | 96,471, 117,736 |
| Gate (R22D-81 `--record` on seed 7302) | passed: every expected line reproduced exactly |

At this scramble no piece reached height 6, either at the scramble or at the census stop, so R22D-81 (whose goal is
n6 = 0) had nothing to do. The open problem here is height 4: R22D-75 lowered 1,185 to 797 height-4 pieces and then
found no single letter that kept to its rule. Lowering the remaining 797 height-4 pieces is outside both rules and is
another line's work. A state-only rule reaching n6 = 0 at one scramble is not coverage; here n6 = 0 was already true
at the stop, so this run says nothing about R22D-81's reach.

## Commands and scripts

All run from the repository root as `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1
JUMBLING_HOME_CACHE=<a file outside the repository> python3 -I -B -u ...`; `<dir>` is a scratch directory outside the
repository.

| Script | sha256 |
| --- | --- |
| `research/jumbling/restoration/11-scramble-census/scripts/census_seams.py` (unchanged) | `d8ee4bc209ea8056951e0fea913a587eb6d1f24aa9385c455582ffea00d4be54` |
| `research/jumbling/restoration/12-low-height/scripts/round22/r22d75_census.py` (source of the copy) | `c642ecdd7e591dec23d879483c437e4867c32d3a3bf9c6a1f1a34b4dedbc79e2` |
| `research/jumbling/restoration/12-low-height/scripts/round22/cloud/census-k3000-s7304/r22d75_census_k3000.py` | `9334eabcf02212aee5642238f9d9f23f23c774e4bb77130179c232da96c96541` |
| `research/jumbling/restoration/12-low-height/scripts/round22/drafts/r22d81_rule.py` (draft at `7fbbd31e`, unchanged, not committed here) | `a63a177ba0c3ef652a2e6be3c55835c46c0eeb9dd0c1feb0a0e64ecaddcb8455` |

`r22d75_census_k3000.py` differs from `r22d75_census.py` only in its header and in the CPU limit `(3590,3600)` →
`(14390,14400)`; the rule and its 1,000 CPU-second budget after the replay are unchanged. It did not reach the
budget, so no 6,000 CPU-second rerun was made.

1. Census: `census_seams.py 3000 7304 18900 <dir>/census` → `ia-k3000-s7304.json` (record), `census.txt` (stdout).
2. Gate: `drafts/r22d81_rule.py --record research/jumbling/restoration/11-scramble-census/runs/heights/ia-k3000-s7302.json
   --save <dir>/s7302-stop.pkl --prefix '[[64,15],[185,14],[357,12],[471,12],[263,14],[407,13],[523,12]]'
   --expect 1ff0061fc5863c8ccbc1d734b003ff49afa5b58969b0031e51fd1e5cf1a07d81` → `gate-r22d81-s7302.txt`.
3. R22D-75: `cloud/census-k3000-s7304/r22d75_census_k3000.py
   research/jumbling/restoration/12-low-height/runs/round22/cloud/census-k3000-s7304/ia-k3000-s7304.json` →
   `r22d75_census_k3000.txt`.
4. R22D-81: `drafts/r22d81_rule.py --record
   research/jumbling/restoration/12-low-height/runs/round22/cloud/census-k3000-s7304/ia-k3000-s7304.json
   --save <dir>/stop.pkl --prefix '[[170,12],[395,12],[500,12],[540,12],[217,15],[148,15]]' --cpu 7200` →
   `r22d81_rule_s7304.txt`. The snapshot `stop.pkl` is not committed.

## Gate (seed 7302)

Passed. LOADED `015d839e…` with 100,584 off; START tau 274, u 3, targets [[104,13],[150,84],[274,159],[290,13],
[311,13]] at `1ff0061f…`, (n2, n4, n6) = (103,103, 1,019, 172); STEP (124,2) U → `9a227291…`, (274,13) H6 →
`c9cefc31…`, (46,1) U → `7465613d…`, (46,3) U → `198e936f…`, (104,14) H6 → `bbef1c8e…`; HANDOFF
`bbef1c8ec0de78dead1473322a8c1236e48f9ccfe0017ce724461bb7f37ae299`, (102,960, 936, 0), 103,896 off, N = 132,930. The
U_ITEMS vectors for cap 124 and cap 46 (twice) are in the output file.

## R22D-75 on seed 7304

| Step | Letter | Digest | (n6, n4, n2) | Off | N |
| --- | --- | --- | --- | --- | --- |
| start | | `7e2304b2…` | (0, 1,185, 94,384) | 95,569 | 115,200 |
| 1 | (170, 12) | `d642f92d…` | (0, 1,184, 95,059) | 96,243 | 115,470 |
| 2 | (395, 12) | `d07f03b4…` | (0, 1,106, 96,222) | 97,328 | 116,272 |
| 3 | (500, 12) | `f977cf2e…` | (0, 908, 95,906) | 96,814 | 116,608 |
| 4 | (540, 12) | `37af5641…` | (0, 797, 96,134) | 96,931 | 117,070 |
| 5 | (217, 15) | `dcaa79b1…` | (0, 797, 95,813) | 96,610 | 117,412 |
| 6 | (148, 15) | `ac127dc3…` | (0, 797, 95,674) | 96,471 | 117,736 |
| STUCK | all single letters exhausted | `ac127dc3…` | (0, 797, 95,674) | 96,471 | 117,736 |

Full digests and pruning counts are in `r22d75_census_k3000.txt`. N rises with each letter (115,200 → 117,736).

## R22D-81 on seed 7304

LOADED `7e2304b2…` (95,569 off), the six-letter prefix applied, START with tau null, u 0, empty target table at
`ac127dc3…`, then HANDOFF with 0 steps: (95,674, 797, 0), 96,471 off, N = 117,736. No U_ITEMS vector was printed.

## Not run or not finished

- No Windows, DirectX or native check; no performance measurement.
- No rule for the 797 remaining height-4 pieces was designed or tried (out of scope for this line).

## Timings (cloud timings of research code, not performance evidence)

Census 22,517 s wall clock in all (scramble build and descent). Gate: 1,384 CPU s to load, 374 CPU s for the rule.
R22D-75: 1,652 CPU s replay, 383 CPU s rule. R22D-81: 1,650 CPU s replay, 18 CPU s rule.
