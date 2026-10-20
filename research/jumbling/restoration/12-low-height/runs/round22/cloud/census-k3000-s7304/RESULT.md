# Cloud line census-k3000-s7304 (round 22, line (d), round 6): in progress

Question: on a fresh 3000-twist I_a scramble (seed 7304), where does the census descent stop, and how far do
R22D-75's height-first rule and then R22D-81's unlock rule get from that stop?

Status: the census descent (`census_seams.py 3000 7304 18900 <dir>/census`) and the validation gate (R22D-81 on the
seed 7302 record with R22D-75's seven letters) were started on 2026-10-20. Results follow when they finish.

Gate (step 2): passed. Every line agreed with the expected values: LOADED 015d839e… with 100,584 off; START with tau 274,
u 3 and targets [[104,13],[150,84],[274,159],[290,13],[311,13]]; STEP (124,2) U, (274,13) H6, (46,1) U, (46,3) U,
(104,14) H6; HANDOFF bbef1c8e… with (n2, n4, n6) = (102,960, 936, 0), 103,896 off, N = 132,930. Output:
`gate-r22d81-s7302.txt`.

Census (steps 1 and 3): stop reason `time-limit` after 580 descent twists (198 rule steps), 22,517 s in all. Start:
121,001 off, N = 139,584, heights {2: 118,951, 4: 2,050}. Stop: 95,569 off, N = 115,200, heights {2: 94,384,
4: 1,185}; no piece at height 6 at the scramble or at the stop. Record `ia-k3000-s7304.json`, stdout `census.txt`.
R22D-75 started on the record.
