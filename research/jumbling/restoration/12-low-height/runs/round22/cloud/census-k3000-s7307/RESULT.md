# Cloud line census-k3000-s7307 (round 22, line (d), round 6)

Status: in progress. The validation gate passed; the census descent of seed 7307 is running.

## Question

What happens on a fresh 3000-twist scramble (seed 7307): where does the census descent stop, how far does R22D-75's
height-first rule get from there, and does R22D-81's unlock rule then reach n6 = 0?

## Validation gate (passed)

Command (from the repository root, with `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 JUMBLING_HOME_CACHE=<file>`):
`python3 -I -B -u research/jumbling/restoration/12-low-height/scripts/round22/drafts/r22d81_rule.py --record research/jumbling/restoration/11-scramble-census/runs/heights/ia-k3000-s7302.json --save <dir>/s7302-stop.pkl --prefix '[[64,15],[185,14],[357,12],[471,12],[263,14],[407,13],[523,12]]' --expect 1ff0061fc5863c8ccbc1d734b003ff49afa5b58969b0031e51fd1e5cf1a07d81`
(draft `r22d81_rule.py` sha256 a63a177ba0c3ef652a2e6be3c55835c46c0eeb9dd0c1feb0a0e64ecaddcb8455, run unchanged from
commit 7fbbd31e). This was the first run of the draft's `--record` path. Output: `gate_r22d81_s7302.txt`.

Every expected line matched: LOADED 015d839e… with 100,584 off; START τ = 274, u = 3, targets
[[104,13],[150,84],[274,159],[290,13],[311,13]]; five STEP lines (124,2) U, (274,13) H6, (46,1) U, (46,3) U,
(104,14) H6; HANDOFF bbef1c8ec0de78dead1473322a8c1236e48f9ccfe0017ce724461bb7f37ae299 with (n2, n4, n6) =
(102,960, 936, 0), 103,896 off, N = 132,930. The replay took 1,419 CPU s and the rule 376 CPU s (cloud timings of
research code, not performance evidence).

## Census progress (partial)

Scramble of seed 7307 (3000 twists): 119,518 off, N = 136,918, left heights {2: 117,245, 4: 2,263, 6: 10}.
Latest descent step seen while running: N = 113,494, 96,807 off, 12,904 s into the
18,900 s wall-clock budget (cloud timings of research code, not performance evidence).

## Next step

Wait for the gate (R22D-81's `--record` path at the 3000-twist stop of seed 7302) and the census of seed 7307; then
run R22D-75 (`r22d75_census_k3000.py`) and R22D-81 on the seed 7307 record.
