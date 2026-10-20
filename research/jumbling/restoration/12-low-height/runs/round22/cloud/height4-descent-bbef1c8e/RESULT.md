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

## Step 4, round 1: two-letter U steps at 6999b921

Script `two_letter_u.py` (sha256 at the end of this file is for the final version; the runs below used it with the
changes stated). Commands: `two_letter_u.py --snapshot <dir>/h4-end-1.pkl --expect 6999b921… --shard i --nshards 3
--cpu 12000 [--from-index k]` for i = 0, 1, 2. Outputs `two_letter_u_shard{0,1,2}_part{1,2,3}.txt`.

What a pair is: first letter (c, j) at a cap c that is clean at the state and holds no height-4 piece, admissible
and certain, no height above 4, n4 and every height-4 pose kept; second letter (h, i) at the intermediate state t
under the same conditions; success when u after the pair is strictly below u before it. The pair's end therefore
satisfies every condition of a U letter.

Exact prune of second letters. Only the pieces inside a letter's cap move, so a second letter lowers u only if its
cap h holds a straddler of τ whole at t. If h is clean at both the state and t and holds the same pieces in the same
poses at both, the second letter moves the same pieces in the same way at both, so u(pair) = u(t) + u_s(h, i) − u,
where u_s(h, i) is u after (h, i) alone at the state; and u_s(h, i) ≥ u because H4-unlock's single-letter U family
is exhausted there (h was one of its candidates), while u(t) ≥ u for every valid first letter (a first letter that
moves a straddler of τ was a U candidate itself; one that moves none cannot remove one). So only caps h that hold a
straddler of τ whole at t, are clean at t, hold no height-4 piece at t, and were blocked at the state or changed
content are tried, with all 59 items each, every one applied on a copy.

Context at 6999b921: τ = 482, u = 1, the one straddler is piece 2492. It is held whole by caps 513, 529, 547, 555,
562, 564, 580, 585 (all blocked, with 226, 185, 329, 345, 160, 80, 366, 291 straddlers of their own, 1,550 distinct)
and 593 (clean). 38 caps are clean; 35 of them hold no height-4 piece and are the first-letter caps. Class A (caps
that hold whole a straddler of a blocked holder) = 357, 370, 541, 552, 590, 591, 593; class B = the other 28.

Enumerated: all 35 × 59 = 2,065 first letters, every one of them (indices 0 to 2,064, each exactly once across the
three shards and three parts), and for each valid one every item of every second cap the prune keeps.

- 1,825 first letters are valid (365 in class A, 1,460 in class B); 240 are not.
- No first letter frees a blocked holder (for every class A letter, every blocked holder still has a straddler).
- 250 pairs succeed, all with the second letter at cap 593 and all ending at u = 0; their first letters are 128
  letters at caps 541 (31), 552 (26), 590 (23) and 591 (48). No class B first letter gives a success.
- Chosen pair (no freeing letter exists, so the first success in (grip, item) order): **(541, 2), (593, 54)**, end
  digest `391490f78b2fe76ea07cede387bf335f76d34885c47a7aa0c09ebc8ad3d27da8`, (n2, n4, n6) = (103,282, 936, 0),
  104,218 off, N = 133,834, u = 0 at τ = 482.

Run history (cloud timings of research code, not performance evidence): part 1 (indices 0 to 138) ended when the
container restarted after about 33 CPU minutes per shard; part 2 resumed at 139 with `--from-index` and was stopped by
PID at about 90 CPU minutes per shard (indices up to 510) after I found that the unchanged-content prune compared
pose ids, which the simulator can renumber, so it never fired: every class B letter of part 2 tested cap 593 in full
(97 letters; all their vectors equal the single-letter vector at the state, so no result changes). The prune was
changed to compare pose keys and part 3 (indices 511 to 2,064) took about 770 CPU s per shard.

## Step 3 again: H4-unlock run 2 from the pair's end

Command: `h4_unlock.py --snapshot <dir>/h4-end-1.pkl --prefix '[[541,2],[593,54]]' --cpu 14400 --save-end
<dir>/h4-end-2.pkl` (output `h4_unlock_run2.txt`; a first attempt passed the snapshot's digest to `--expect`, which
the script checks after the prefix, and stopped at the assertion: `h4_unlock_run2_expect_misuse.txt`).

Start at `391490f7…`: τ = 482 (k = 284), u = 0, the same target table as in run 1.

| step | family | letter | τ | u | (n2, n4, n6) | off | N | end digest | cpu s |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | L4 | (482, 15) | 597 | 41 | (102,750, 734, 0) | 103,484 | 133,558 | `c312a2fcb7f2a502908f71b2d9acee8cb9868fb63930b4949483ee5061e830d1` | 56.7 |

(482, 15) lowers n4 by 202 (936 → 734) and n2 by 532 (103,282 → 102,750); the number of pieces on the lattice rises by
734 (off 104,218 → 103,484). New target table (cap, k): (12, 121), (34, 2), (65, 118), (71, 13), (94, 13), (140, 13),
(142, 13), (171, 67), (180, 13), (181, 13), (206, 48), (299, 49), (388, 45), (482, 82), (517, 178), (554, 82),
(597, 205); τ = 597, u = 41.

End: **STUCK** after 1 letter, cpu 99.2 s: no clean cap holds a straddler of 597 whole (no U_ITEMS line), and no L4
or L2 letter exists (563 caps blocked, 11 cap bound prunes).

## Step 4, round 2: two-letter U steps at c312a2fc

Command: `two_letter_u.py --snapshot <dir>/h4-end-2.pkl --expect c312a2fc… --shard i --nshards 3 --cpu 12000`.
Context: τ = 597, u = 41 (41 straddlers). Holders: caps 535, 566, 567, 569, 576, 578, 583, 587, 588, 595, all
blocked, with 2, 76, 321, 96, 462, 320, 416, 137, 369, 229 straddlers of their own (1,932 distinct). 37 caps are
clean; 33 are first-letter caps; class A = 407, 471, 492, 523, 533, 553, 590, 591; 1,947 first letters. Since no
clean cap holds a straddler of τ whole, no first letter moves one, and every holder of an original straddler is
blocked at the state, a success needs a first letter that frees a blocked holder (only class A letters can).

Enumerated (outputs `two_letter_u_round2_shard{0,1,2}.txt`): all 472 class A first letters (indices 0 to 471) and
the class B letters with indices 472 to 588 (contiguous; a few beyond), after which I stopped the shards by PID,
because by the argument above no class B letter can succeed and the freeing letters come first in the stated order.

- 10 first letters free a blocked holder, all at cap 553 and all freeing cap 535: items 5, 19, 27, 39, 43, 49, 53, 54,
  56, 58. They give 187 successful pairs, every second letter at cap 535; some reach u = 0 (for example (553, 27),
  (535, 21)).
- Chosen pair (the first freeing letter, then its first success in (grip, item) order): **(553, 5), (535, 3)**, u 41
  → 21, end digest `6c7a79d9aa68769261c3ba3d613bfaff5d2d539e57797664418f46e85f594744`, (n2, n4, n6) = (102,750, 734,
  0), 103,484 off, N = 133,678. The rule takes the first success, not the lowest u.

## Step 3 again: H4-unlock run 3 from 6c7a79d9

Command: `h4_unlock.py --snapshot <dir>/h4-end-2.pkl --prefix '[[553,5],[535,3]]' --cpu 14400 --save-end
<dir>/h4-end-3.pkl` (output `h4_unlock_run3.txt`). Start: τ = 597, u = 21.

| step | family | letter | τ | u | (n2, n4, n6) | off | N | end digest | cpu s |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | U | (535, 6) | 597 | 10 | (102,750, 734, 0) | 103,484 | 133,700 | `b2651c24f07172eb2cf18fa2cf5b61eba12fa2fb2271711feb7c249fb0e27b51` | 99.2 |

End: **STUCK** after 1 letter, cpu 167.4 s; cap 535 is the only clean holder tried (its U_ITEMS at u = 21 and 10 are
in the output).

## Step 4, round 3: two-letter U steps at b2651c24

Command: `two_letter_u.py --snapshot <dir>/h4-end-3.pkl --expect b2651c24… --shard i --nshards 3 --cpu 12000`
(outputs `two_letter_u_round3_shard{0,1,2}.txt`). Context: τ = 597, u = 10 (pieces 108404, 108405, 108407, 108420,
114001, 114010, 125565, 125574, 143345, 143346). Holders: 535 (clean) and 566, 567, 569, 576, 578, 583, 587, 588
(blocked, with 102, 393, 96, 389, 361, 396, 137, 340 straddlers of their own, 1,809 distinct). 37 clean caps; class
A = 407, 471, 492, 523, 533, 535, 590, 591; 1,947 first letters.

Enumerated: indices 0 to 1,587 contiguously (all of class A and the class B caps 2 to 358 and cap 363 items 1 to 54;
a few beyond in some shards), then stopped by PID, since every first letter before the chosen one in the stated
order had been tried. 1,383 valid first letters (328 A, 1,055 B), 240 not valid.

- No first letter frees a blocked holder. Second caps tried: only 535 (77 first letters).
- 14 first letters succeed, all at cap 363 (items 10, 21, 22, 32, 36, 37, 38, 41, 45, 46, 50, 51, 52, 57), each with
  the one second letter (535, 8), ending at u = 8 (12 of them) or 9 (2).
- Chosen pair: **(363, 10), (535, 8)**, u 10 → 8, end digest
  `2430f4cc99ea121a6d3476ec21b66fc401e277d13809ad5e1c979e6e0abe1472`, (n2, n4, n6) = (102,750, 734, 0), 103,484 off,
  N = 133,818.

## Step 3 again: H4-unlock run 4 from 2430f4cc

Command: `h4_unlock.py --snapshot <dir>/h4-end-3.pkl --prefix '[[363,10],[535,8]]' --cpu 14400 --save-end
<dir>/h4-end-4.pkl` (output `h4_unlock_run4.txt`). Start: τ = 597, u = 8. End: **STUCK** with no letter, cpu
102.0 s (cap 535's 59 items give u ≥ 10 or null).

## Step 4, round 4: two-letter U steps at 2430f4cc

In progress.

## Open questions

To be filled in at the end.
