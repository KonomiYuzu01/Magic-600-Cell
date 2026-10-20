# Cloud line height4-descent-bbef1c8e: a literal descent of the 936 height-4 pieces from bbef1c8e

Theory round 22, line (d), round 6. Research only, headless Linux, exact geometry (every letter admissible and
certain on the exact simulator `research/jumbling/sim/`, no float decision). Times are cloud timings of research code,
not performance evidence. No Windows, DirectX, input, long-session or performance check was run or is claimed.

## Question

At the 3000-twist stop of seed 7302 (`015d839e…`), R22D-75's seven letters and R22D-81's five letters reach
`bbef1c8ec0de78dead1473322a8c1236e48f9ccfe0017ce724461bb7f37ae299` with (n2, n4, n6) = (102,960, 936, 0), 103,896 off,
N = 132,930. Can literal single letters (and, where they stop, two-letter U steps) bring the 936 height-4 pieces down
to height 2, so that the height-2 phase can take over?

## Scripts (sha256)

| Script | sha256 |
| --- | --- |
| `scripts/round22/r22d31_snapshots.py` (existing, run unchanged) | `1e18b548b181dff29a1fee4c914acc785706887773b7cc75cbb62f8c25fb6256` |
| `scripts/round22/drafts/r22d80_check.py` (draft, run unchanged) | `39dda535a4a47b9c66c93537c15d504b01ffda4af42ca51e125beaf5ea6f8803` |
| `scripts/round22/r22d75_census.py` (source of the step 2 copy, not run) | `c642ecdd7e591dec23d879483c437e4867c32d3a3bf9c6a1f1a34b4dedbc79e2` |
| `scripts/round22/drafts/r22d81_rule.py` (source of the step 3 copy, not run) | `a63a177ba0c3ef652a2e6be3c55835c46c0eeb9dd0c1feb0a0e64ecaddcb8455` |
| `cloud/height4-descent-bbef1c8e/r22d75_from_snapshot.py` | `15a6b764b12c493f5f90dd3fb643e79182ddf1775d50f76d53dfa75540584aac` |
| `cloud/height4-descent-bbef1c8e/h4_unlock.py` | `12078831f1b41c46b9a9f50a1fa88ddabdcafdbb9b766c1a278a662c0ffc44a7` |
| `cloud/height4-descent-bbef1c8e/two_letter_u.py` | `65f8a0e2ce659e3d7b1eae0674ceca9b2af595d0903f72dad5a7e58d14cbc6d8` |

Paths are relative to `research/jumbling/restoration/12-low-height/`; `cloud/…` is `scripts/round22/cloud/…`. Every
run used `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 JUMBLING_HOME_CACHE=<a file outside the
repository> python3 -I -B -u` from the repository root. `<dir>` is a scratch directory outside the repository; no
snapshot is committed.

What was changed in the copies:

- `r22d75_from_snapshot.py` against `r22d75_census.py`: argument parsing (`--snapshot`, `--prefix`, `--expect`,
  `--cpu`), the snapshot load (`from r22d31_snapshots import load`) and the prefix (each letter admissible and certain,
  digest checked by `--expect`) in place of the record replay, the CPU budget as a parameter (3,000 s) and the round22
  directory on `sys.path`. The three families, the exact count prediction and its check, the cap bound and the end
  conditions are unchanged.
- `h4_unlock.py` against `r22d81_rule.py`: heights 6/4 → 4/2 in the target, the families, the guards and the handoff;
  the families L4, U, L2 (H6 dropped, since no piece is above height 4, which `counts()` asserts); R22D-75's count
  prediction kept for every item of a clean cap and checked against the simulator after every letter, U letters
  included (a retained item, one in K+, is predicted to keep every count; see run 0 below); the target table printed
  again at STUCK; `checktime()` inside `target()`; `--save-end <pkl>` (load() format, with the letters played since
  015d839e). Containment tests are the draft's.

## Step 1: the snapshot and the R22D-80 check

- `r22d31_snapshots.py <dir>` wrote `s7103-before74.pkl` (`48ef2f68…`, 94,981 off) and `s7302-stop.pkl`
  (`015d839ecbb37bbdde74ae9391d5d7f274dea7fee4b4fdf0092d224eef907a1f`, 100,584 off); 2,211 s wall for both. Output:
  `r22d31_snapshots.txt`.
- `drafts/r22d80_check.py <dir>/s7302-stop.pkl` reproduced R22D-80's five letters with every digest and ended at
  `bbef1c8e…` with `TARGET_HEIGHTS {4: 172} LATTICE_COLLATERAL 1052`. Output: `r22d80_check.txt`.

## Step 2: R22D-75 from bbef1c8e

Command: `r22d75_from_snapshot.py --snapshot <dir>/s7302-stop.pkl --prefix
'[[64,15],[185,14],[357,12],[471,12],[263,14],[407,13],[523,12],[124,2],[274,13],[46,1],[46,3],[104,14]]' --expect
bbef1c8e…` (output `r22d75_from_snapshot.txt`).

| step | letter | family | (n2, n4, n6) | off | N | end digest | cpu s |
| --- | --- | --- | --- | --- | --- | --- | --- |
| start | – | – | (102,960, 936, 0) | 103,896 | 132,930 | `bbef1c8e…` | 16.9 |
| 1 | (23, 14) | keeps n6, n4; lowers n2 | (102,901, 936, 0) | 103,837 | 133,134 | `0ad859c24e540a55de416e8a901e4da7b3ddbccc3d6e19a50629ac7b35a87e05` | 65.4 |

End: **STUCK** after one letter (every single letter exhausted; 600 caps classified, 562 blocked, 38 clean, 11 cap
bound prunes), cpu 113.0 s after the prefix. R22D-75 moves once at bbef1c8e and lowers no height-4 piece.

## Step 3: H4-unlock

### Termination argument (not coverage)

Φ = n4·B² + u·B + n2 with B = 177,121. Every coordinate is a non-negative integer below B: n4 and n2 count pieces and
u counts off pieces at one cut, and there are 177,120 pieces. An L4 letter lowers n4 by at least 1, so Φ falls by at
least B² − (B − 1)·B − (B − 1) = 1 whatever it does to u (τ may change) and n2. A U letter keeps the height-4 set and
every height-4 pose; the target τ depends only on that set and those poses (which caps hold which height-4 pieces
whole, and the exact products of the items with their left factors), so τ is unchanged and u is measured at the same
cap; n4 is kept and u falls by at least 1, so Φ falls by at least B − (B − 1) = 1 whatever n2 does. An L2 letter keeps
the height-4 set and poses (hence τ), does not raise u and lowers n2. So Φ falls strictly at every accepted letter and
the rule stops after at most Φ(start) letters. This is R22D-81's proof with one coordinate fewer. Termination is not
coverage: the rule may stop STUCK with height-4 pieces left, and it did.

### Run 0 (discarded: a check of my own copy failed)

From `bbef1c8e…` (started in parallel with step 2 before step 2 had ended): the U family chose (357, 2), a retained
item, and my added after-letter check asserted R22D-75's jumble-letter formula for it (prediction (n6, n4, n2) =
(0, 1,003, 103,070) against the simulator's (0, 936, 102,960)). The draft never applies that formula to retained
items (it skips them as `retained_prunes`); a retained item keeps every left height. The copy now predicts unchanged
counts for retained items, and the check still runs against the simulator. Output (path-sanitized):
`h4_unlock_run0_prediction_assert.txt`. This run started from bbef1c8e, not from the end of step 2, so it is not
part of the descent.

### Run 1: from the end of step 2

Command: `h4_unlock.py --snapshot <dir>/s7302-stop.pkl --prefix '<the twelve letters>,[23,14]]' --expect 0ad859c2…
--cpu 14400 --save-end <dir>/h4-end-1.pkl` (output `h4_unlock_run1.txt`).

Start: τ = 482 (k = 284), u = 80. Target table (cap, k): (12, 121), (34, 2), (65, 118), (71, 13), (94, 13), (140, 13),
(142, 13), (171, 67), (180, 13), (181, 13), (206, 48), (299, 49), (388, 45), (482, 284), (500, 77), (517, 178),
(551, 52), (597, 205).

| step | family | letter | τ | u | (n2, n4, n6) | off | N | end digest | cpu s |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | U | (357, 1) | 482 | 11 | (102,901, 936, 0) | 103,837 | 133,084 | `52c9a7154dc67a7de81b09b11fff2cd20f45c2eb0a3f0bbe1655fcf47a51f8e9` | 116.2 |
| 2 | U | (357, 3) | 482 | 8 | (102,901, 936, 0) | 103,837 | 133,044 | `e13f5264ae0471fad49390737d82aa7ffc175c693360141bda267afc6ed81ac0` | 206.1 |
| 3 | U | (593, 3) | 482 | 2 | (102,901, 936, 0) | 103,837 | 133,238 | `c7d5be2271b315ecaee7713a54ff2fff439c9261e2ee1b179ddde04761e2ae06` | 340.3 |
| 4 | U | (593, 43) | 482 | 1 | (103,571, 936, 0) | 104,507 | 133,668 | `6999b9214d6ea25486daf67e650773babff08f1a41742acfddd6eec2f88787ab` | 452.9 |

U_ITEMS vectors (u after items 1 to 59; null = not admissible and certain, or a height above 4, or a height-4
piece or pose changed) are in `h4_unlock_run1.txt` for cap 357 at u = 80 and u = 11 and cap 593 at u = 8, 2 and 1.
Cap 357's items 12 to 59 are all null at both states; its items 1 to 11 give u = 11, 8, 41, 34, 8, 57, 45, 8, 8, 108,
13 at u = 80.

End: **STUCK** after 4 letters (all U), cpu 607.5 s: τ = 482, u = 1, (n2, n4, n6) = (103,571, 936, 0), 104,507 off,
N = 133,668, the same target table. No L4 or L2 letter exists at that state (562 of the 600 caps blocked, 11 cap bound
prunes).

## Step 4: two-letter U steps at 6999b921

In progress (see the next update).

## Open questions

To be filled in at the end.
