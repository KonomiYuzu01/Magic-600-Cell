# Cloud line unlock-cap311-cap290 (round 22, line (d), round 6)

Research only, headless Linux; exact geometry. Nothing here is Windows, DirectX, input or performance evidence.

## Question

At the 3000-twist stop of seed 7302 (`015d839e…`) and after R22D-75's seven letters (`1ff0061f…`, 172 pieces at
height 6), can the targets cap 311 (u = 182 at `1ff0061f`, 107 at `015d839e`) and cap 290 (u = 38 at `9e6b991b…`,
where R22D-86 found the single-letter U family exhausted) be unlocked by literal letters, each admissible and certain
on the exact simulator, each keeping H6 (n6 unchanged and every height-6 piece at its exact pose) and leaving no piece
above height 6, and then used for an H6 letter? If not, where exactly does it stall?

## Answer in short

All three routes reach u = 0, and each ends with an H6 letter that lowers 13 height-6 pieces to height 4. So the
unlock rule has more than one way through: besides R22D-80's route through caps 274 and 104 there are routes through
cap 290 and cap 311.

| Target, start | Word after the start (letters) | End digest | n2 / n4 / n6 | off | N |
| --- | --- | --- | --- | --- | --- |
| 290 from `9e6b991b` (= `1ff0061f` + (136, 2), (136, 6)) | (301, 2) (136, 7) (301, 4) (136, 1) (136, 4) (136, 19) (301, 3) (136, 1) (301, 3) (136, 5) (290, 15) | `21db4f944c71f020f2380e434dc1b37618c9b1437960ef87365f3345aaa34928` | 104,526 / 1,391 / 159 | 106,076 | 134,146 |
| 311 from `1ff0061f` | (185, 1) (263, 1) (263, 3) (364, 3) (263, 6) (263, 40) (311, 14) | `5296c847f5a19cb1bc6e4cc531786c833dc681adbdc367763378f9325e740138` | 104,070 / 1,470 / 159 | 105,699 | 133,790 |
| 311 from `015d839e` (no seven letters) | 14 U letters, then (364, 1) (407, 2) (407, 4) (407, 10) (407, 15) (364, 2) (263, 2) (311, 14) | `2f88c1e617cc22c7f09eef19d6e7c530fdcb7a49c577305f1ca79196533726b1` | 100,879 / 1,372 / 172 | 102,423 | 131,998 |

None of these is protected or claimed as optimal. Each one is the first word the stated enumeration order finds.
Every letter is checked by the simulator (`o.applied and not o.uncertain`), its u by `E.straddler_set`, and the
pieces it moves against the label table. The H6 letter lowers 13 pieces in each case: of the start's height-6 pieces,
13 end at height 4 and the rest stay at height 6 with their poses (`start_height6_now` in the JSON records). Pieces
that were at the lattice at the route's start and are off at its end: 2,157 (cap 290 route, counted from `9e6b991b`),
1,902 (cap 311 from `1ff0061f`, counted from `fe273a8c`), 2,086 (cap 311 from the stop, counted from `efab42da`).

## Commands

All from the repository root as `OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1
JUMBLING_HOME_CACHE=<a file outside the repository> python3 -I -B -u <script> …`, with `<dir>` a scratch directory
outside the repository. Python 3.13.16 and NumPy 2.5.3 in this cloud session.

| Script (under `scripts/round22/`) | sha256 |
| --- | --- |
| `cloud/unlock-cap311-cap290/build_stop.py` | `52a721e6dc7c1628b604c2daffb7620e12aae872c556d791f082f4518dc94159` |
| `cloud/unlock-cap311-cap290/unlock_common.py` | `268edf01f9d02a8fffb2fc5471753f25803fc3af7c302301296076e926aad76d` |
| `cloud/unlock-cap311-cap290/cap311_unlock.py` | `15549ae4f1e871ee730ab8738deecb39559791ec7da3bf25b04f655bfa40cb0a` |
| `cloud/unlock-cap311-cap290/prep_two_letters.py` | `d8e9bc3dc25196f73aaf3f852e22a009429eff048334df5725dcaf1b56a4b03c` |
| `cloud/unlock-cap311-cap290/unlock_driver.py` | `accc7616ad09f0700b0c58df42f78ef88ca572f935938fffffd9e50952e797c4` |
| `r22d31_snapshots.py` (imported by `build_stop.py`, unchanged) | `1e18b548b181dff29a1fee4c914acc785706887773b7cc75cbb62f8c25fb6256` |
| `drafts/r22d80_check.py`, `r22d82_unlock_graph.py`, `r22d85_two_letters.py`, `r22d86_cap290.py` | as in the task (`39dda535…`, `377005e1…`, `190a7669…`, `92de16bf…`; checked) |

1. `cloud/unlock-cap311-cap290/build_stop.py <dir>` builds `<dir>/s7302-stop.pkl`: `015d839e…`, 100,584 off, 3000
   history twists and 313 descent twists. It does what `r22d31_snapshots.py` does for this one snapshot, using that
   script's own `SNAPSHOTS` entry and `load()`. I wrote it because a container restart killed the first full
   `r22d31_snapshots.py` run after it had written the 7103 snapshot. It was built a second time after a later restart,
   with the same digest and off count.
2. `drafts/r22d80_check.py <dir>/s7302-stop.pkl` → `r22d80_check.txt`.
   `drafts/r22d85_two_letters.py <pkl>` → `r22d85_two_letters.txt`. `drafts/r22d86_cap290.py <pkl>` →
   `r22d86_cap290.txt`. `drafts/r22d82_unlock_graph.py <pkl>` → `r22d82_unlock_graph.txt`. None was edited.
3. `cloud/unlock-cap311-cap290/cap311_unlock.py <pkl> after75 cap311_after75.json` and `… stop cap311_stop.json` →
   `cap311_after75.{txt,json}`, `cap311_stop.{txt,json}`.
4. `cloud/unlock-cap311-cap290/prep_two_letters.py <pkl> after75 136:2,136:6 9e6b991b… 290 k/3 10800 prep290_sk.json`
   for k = 0, 1, 2. Then `… after75 185:1,263:1,263:3 fe273a8c… 311 k/3 10800 prep311_sk.json` → `prep290_s*`,
   `prep311_s*`.
5. `cloud/unlock-cap311-cap290/unlock_driver.py <pkl> after75 136:2,136:6 9e6b991b… 290 21600 drive290.json`;
   `… after75 185:1,263:1,263:3 fe273a8c… 311 21600 drive311_after75.json`;
   `… stop 185:1,185:7,185:16,185:3,263:1,263:3,263:8,364:1,263:2,407:1,407:3,407:1,407:2,407:4 efab42da… 311 21600
   drive311_stop.json` → `drive*.{txt,json}`.

What the scripts take from the drafts: `unlock_common.py` copies `clone`, `put`, `heights`, `keys` and `inside` from
`r22d86_cap290.py`. `put` returns the admissible-and-certain flag instead of asserting it. `all_labels` comes from
`r22d82_unlock_graph.py`, changed only to take an optional list of caps. New helpers: `strad_among` (the
`E.straddler_set` loop restricted to the given pieces) and a label table that is updated only for the pieces a
letter moves. Before any run, a smoke test on the 7103 snapshot (not committed) checked the table against
`E.straddler_set` at 31 caps, before and after one letter. The table also matched every u reported here.

## Step 1: the drafts against the README

All four drafts printed exactly the README facts. I found no difference.

- `r22d80_check`: the five letters with digests `9a227291…`, `c9cefc31…`, `7465613d…`, `198e936f…`, `bbef1c8e…`; at
  the end (102,960, 936, 0), 103,896 off, N = 132,930; `TARGET_HEIGHTS {4: 172} LATTICE_COLLATERAL 1052`.
- `r22d85`: from the stop → `bd44f887…`, (98,713, 992, 26), 99,731 off, N = 130,250, 346 initially lattice pieces off.
  From `1ff0061f` → `c9cefc31…`, (102,593, 926, 13), 103,532 off, N = 132,572, 479 initially lattice pieces off; the start's height-6 pieces
  end {4: 159, 6: 13}. The README does not give these last numbers.
- `r22d86`: (136, 2) → u = 48 at `808aeac4…`; (136, 6) → u = 38 at `9e6b991b…`, N = 132,760. Holders [250, 260, 276,
  283, 301, 302, 307, 348, 355, 356, 362, 375, 392, 405, 453, 466, 472], clean [301]. The lowest u over the 59 items
  of cap 301 is 38, at items 3 and 9.
- `r22d82`: closure layers [38, 107, 130, 86, 93, 84, 15, 47] at `1ff0061f` and [39, 118, 125, 80, 92, 84, 15, 47] at
  `015d839e`, all 600 caps. Targets 274/104/290/311 have 3/30/70/182 straddlers at `1ff0061f` and 3/30/70/107 at the
  stop, all in layer 1. Cap 311's unit rows at `1ff0061f` are 1545: 19, 19065: 7, 38265: 99, 75665: 26, 81179: 23,
  96213: 8, all branch 2. At the stop the unit ids differ (422: 13, 1128: 3, 46248: 7, 61240: 19, 79627: 8,
  96213: 8, 107400: 26, 127471: 19, 158577: 4), still all branch 2. 18,730 pieces change between the two states.

## Step 2: monotone single-letter unlock of cap 311 (`cap311_unlock.py`)

Each round looks at the clean caps holding a remaining blocker whole, in cap order. Each item 1..59 is applied to a
copy. The first letter that keeps H6, leaves no height above 6 and lowers u311 is played. The full 59-item vector
(u, keeps H6, highest height) of every cap tried is in the .txt and .json files.

From `1ff0061f` (u = 182; holders 23 caps, clean [185, 263, 364]):

| # | letter | family | u311 | n2 / n4 / n6 | off | N | digest |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | (185, 1) | U | 152 | 103,103 / 1,019 / 172 | 104,294 | 132,720 | `02bb938f54773e28…` |
| 2 | (263, 1) | U | 38 | 103,103 / 1,019 / 172 | 104,294 | 132,942 | `e69a2823df1a58ac…` |
| 3 | (263, 3) | U | 8 | 103,103 / 1,019 / 172 | 104,294 | 132,952 | `fe273a8c88d8a8fe…` |

**Stall** at `fe273a8c88d8a8fe7f030f95e6130fc76a57da6d708dea110a1cb31125276122`, u311 = 8. Holders [338, 364, 420,
446, 491]; only 364 is clean (the others have 88, 264, 159 and 182 straddlers). Cap 364 holds all 8 blockers whole,
but none of its 59 items lowers u: the lowest is 8, at items 3 and 9. Every item keeps H6.

From `015d839e` (u = 107; clean holders [185, 263, 364, 407]): 14 U letters (185, 1), (185, 7), (185, 16), (185, 3),
(263, 1), (263, 3), (263, 8), (364, 1), (263, 2), (407, 1), (407, 3), (407, 1), (407, 2), (407, 4). The u values are
104, 102, 101, 100, 97, 77, 66, 61, 53, 40, 39, 33, 27, 23. (185, 16) takes 892 lattice pieces off: n2 goes from
99,314 to 100,206 and off from 100,584 to 101,476. The rule allows this: it keeps H6 and no height goes above 6.

**Stall** at `efab42dab21df190c53346f34b78f87a138e34be9e0314c92d78f5e04b666f43`, u311 = 23, (100,206, 1,085, 185),
101,476 off, N = 130,814. Holders [338, 364, 405, 420, 426, 446, 491]; only 364 is clean. It holds all 23 blockers
whole, and its lowest u is 23, at items 3 and 9.

Since both runs stall above 0, the H6 part of step 2 was not reached in `cap311_unlock.py`. It is reached by the
driver (below).

## Step 3: two-letter preparations (`prep_two_letters.py`)

Pair (a, b): a must be admissible and certain, keep H6 and leave no height above 6; u may stay or rise. b must also be
admissible and certain, keep H6 and leave no height above 6, and must leave u below u0, the value before a.

First letters are enumerated in this order:

- **Class (i).** First the clean caps that hold a remaining blocker whole, or that hold whole every straddler of some
  blocked holder, in cap order, items 1..59. An item counts as class (i) when, after it, a new (blocker, clean
  holder) pair exists or a blocked holder has become clean.
- **Class (ii).** Then every other clean cap's items, plus the class (i) caps' other items, in (cap, item) order, under
  3 CPU hours per shard.

Seconds b are taken over the clean caps that hold a blocker whole after a. A cap that holds at most u′ − u0 blockers
is skipped, since u after b is at least the number of blockers b does not move.

**Exact class (ii) bound.** A piece that a does not move keeps its pose, so b treats it exactly as b alone would. So
for any b at a cap that was clean at the start:

u(a, b) ≥ |Y_b| − |in(a) ∩ Y_b|

where Y_b is the set of straddlers of τ after b alone, and in(a) is the set of pieces that a's cap holds whole. This
bound is the same for all 59 items of a cap. A class (ii) letter creates no new pair and makes no blocked holder
clean, so its seconds lie at caps that were clean holders at the start. When the bound is at least u0 for every such
b, all 59 items of the cap are recorded as pruned with the bound, without trials. The work is split over 3 shards by
cap.

**Cap 290 at `9e6b991b` (u0 = 38).** The rebuilt digest was checked. There are 38 clean caps. The only clean holder
and the only class (i) cap is 301 (the 16 blocked holders have 67 to 259 straddlers).

- Class (i), complete: all 59 items of 301 are class (i). 3,717 pairs were tried; 4 first letters have accepted
  seconds, 68 pairs in all. The first accepted pair in order is (301, 2), (136, 7): u = 37, counts and off unchanged,
  `fa6ad5b2…`. The lowest u is 3, from (301, 9), (136, 24), with off rising by 477 (`e4a589a7…`).
- Class (ii), complete. There are 37 caps. The bound prunes 35 of them (bound 38 = u0), 2,065 first letters in all:
  [2, 20, 23, 39, 46, 59, 62, 64, 66, 70, 93, 112, 124, 167, 185, 262, 263, 357, 358, 363, 364, 370, 394, 407, 422,
  471, 492, 523, 533, 541, 552, 553, 590, 591, 593]. Two caps are left, 136 (bound 30) and 277 (bound 29), and all
  118 of their first letters were tested.
  - Cap 136: 3,363 pairs tried, 30 accepted pairs from 10 first letters. The first is (136, 1), (301, 1) with u = 37;
    the lowest is u = 30, from (136, 2), (301, 1).
  - Cap 277: 123 accepted pairs from 41 first letters. The first is (277, 1), (301, 4) with u = 29, which is also the
    lowest.
- In all, at `9e6b991b`: 118 + 59 = 177 first letters were tested, 2,065 were pruned and 10,561 pairs were tried.
  There are 221 accepted pairs. Every first letter at 301, 136 and 277 keeps H6.

**Cap 311 at `fe273a8c` (u0 = 8).** There are 38 clean caps. The only clean holder and class (i) cap is 364.

- Class (i), complete: 58 items of 364 are class (i). Item 9 is not class (i) by its effect, so it was deferred to
  class (ii). 2 first letters have accepted seconds, 5 pairs in all:
  - (364, 3) with (263, 6) → u = 2 (counts unchanged), with (263, 32) → u = 0 and with (263, 42) → u = 0. The last
    two take 462 and 841 more pieces off.
  - (364, 10) with (263, 32) and (263, 42) → u = 6.
- Class (ii), complete. There are 37 clean caps, and the bound prunes 36 of them (bound 8), 2,124 first letters. That
  leaves 407 (bound 6) and the deferred (364, 9), 60 first letters, all tested. (364, 9) has no accepted second. 9
  first letters at 407 have accepted seconds, 27 pairs in all, each with b at 364 (items 4, 5 and 10). The first is
  (407, 3), (364, 4) with u = 6; the lowest is 6.
- In all: 118 first letters tested, 2,124 pruned, 5,782 pairs tried, 32 accepted pairs.
- Neither target needed class (ii) or three letters to make progress: at each stall a class (i) pair was accepted.

The re-runs of `prep290_s0`, `prep311_s0` and `prep311_s1` after the container restart printed the interrupted runs'
lines again exactly, apart from CPU fields. The interrupted outputs are therefore not kept.

No triple (a, a′, b) was needed: two-letter preparations were found at every stall, so `--three` was not run.

## Step 4: the driver to u = 0 and the H6 letter (`unlock_driver.py`)

The driver repeats a round until u = 0, then plays the H6 letter. Each round:

1. It plays the first U letter.
2. If no U letter works, it plays the first accepted pair in the order of step 3, with the same bound.

At u = 0 it applies every item of τ to a copy and plays the one lowering n6 most (ties by lowest item) with no height
above 6.

Cap 290 from `9e6b991b`:

| # | letter | family | u290 | n2 / n4 / n6 | off | N | digest |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | (301, 2) | prep | 56 | 103,103 / 1,019 / 172 | 104,294 | 132,774 | `f0051e3844368f9b…` |
| 2 | (136, 7) | prep | 37 | 103,103 / 1,019 / 172 | 104,294 | 132,848 | `fa6ad5b26c600d92…` |
| 3 | (301, 4) | prep | 41 | 103,103 / 1,019 / 172 | 104,294 | 132,896 | `ba1e098737709cb5…` |
| 4 | (136, 1) | prep | 26 | 103,103 / 1,019 / 172 | 104,294 | 132,988 | `0533c38a5a529df9…` |
| 5 | (136, 4) | U | 9 | 103,103 / 1,019 / 172 | 104,294 | 132,964 | `bd31c9fb1becf6fd…` |
| 6 | (136, 19) | U | 6 | 103,642 / 1,019 / 172 | 104,833 | 133,296 | `176bb059a975a7b4…` |
| 7 | (301, 3) | prep | 6 | 103,642 / 1,019 / 172 | 104,833 | 133,264 | `3c60fd10573c58be…` |
| 8 | (136, 1) | prep | 3 | 103,642 / 1,019 / 172 | 104,833 | 133,210 | `eaf15426f3c1877f…` |
| 9 | (301, 3) | prep | 3 | 103,642 / 1,019 / 172 | 104,833 | 133,228 | `3aa0e23855e66b5e…` |
| 10 | (136, 5) | prep | 0 | 103,642 / 1,019 / 172 | 104,833 | 133,190 | `da130bb7b59907b4…` |
| 11 | (290, 15) | H6 | 0 | 104,526 / 1,391 / 159 | 106,076 | 134,146 | `21db4f944c71f020…` |

At u = 0, twelve items of cap 290 lower n6, each by 13: items 15, 18, 22, 33 and 52 to 59. They all give the same
counts and off, (104,526, 1,391, 159) and 106,076, and they take 1,243 lattice pieces off. Their N ranges from
133,928 to 134,146. Item 15 is the first.

Cap 311 from `fe273a8c` (after (185, 1), (263, 1), (263, 3)):

| # | letter | family | u311 | n2 / n4 / n6 | off | N | digest |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | (364, 3) | prep | 8 | 103,103 / 1,019 / 172 | 104,294 | 133,098 | `f89e1df01b3752ed…` |
| 2 | (263, 6) | prep | 2 | 103,103 / 1,019 / 172 | 104,294 | 133,138 | `be3ce23095a9bf44…` |
| 3 | (263, 40) | U | 0 | 103,565 / 1,019 / 172 | 104,756 | 133,040 | `b1cdbd09ab8aca71…` |
| 4 | (311, 14) | H6 | 0 | 104,070 / 1,470 / 159 | 105,699 | 133,790 | `5296c847f5a19cb1…` |

Twelve items of cap 311 lower n6 by 13: 14, 21, 23 and 43 to 51. All give 943 lattice pieces off, and their N ranges
from 133,670 to 133,790. Item 14 is the first.

Cap 311 from `efab42da` (the stop plus the 14 U letters of step 2):

| # | letter | family | u311 | n2 / n4 / n6 | off | N | digest |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | (364, 1) | prep | 40 | 100,206 / 1,085 / 185 | 101,476 | 130,838 | `cb0cf3a62d72bb8e…` |
| 2 | (407, 2) | prep | 19 | 100,206 / 1,085 / 185 | 101,476 | 130,892 | `6aebd6198a82acd4…` |
| 3 | (407, 4) | U | 17 | 100,206 / 1,085 / 185 | 101,476 | 130,850 | `c7938add27df566b…` |
| 4 | (407, 10) | U | 12 | 100,206 / 1,085 / 185 | 101,476 | 130,822 | `d86a42d2de13431e…` |
| 5 | (407, 15) | U | 8 | 99,940 / 1,085 / 185 | 101,210 | 130,846 | `341f9b5fdf80278e…` |
| 6 | (364, 2) | prep | 23 | 99,940 / 1,085 / 185 | 101,210 | 131,076 | `7672581d60813f5e…` |
| 7 | (263, 2) | prep | 0 | 99,940 / 1,085 / 185 | 101,210 | 131,158 | `352a06b12f314c15…` |
| 8 | (311, 14) | H6 | 0 | 100,879 / 1,372 / 172 | 102,423 | 131,998 | `2f88c1e617cc22c7…` |

The same twelve items (14, 21, 23, 43 to 51) lower n6 by 13, each taking 1,213 lattice pieces off.

## CPU times (cloud timings of research code, not performance evidence)

| Job | CPU s (user + sys) |
| --- | --- |
| `build_stop.py` (each build) | about 1,800 |
| `r22d80_check` / `r22d85` / `r22d86` / `r22d82` | 26 / 27 / 66 / 197 |
| `cap311_unlock` from `1ff0061f` / from the stop | 271 / 710 |
| `prep290` shards 0 / 1 / 2 | 6,503 / 1,615 / 192 |
| `prep311` shards 0 / 1 / 2 | 3,024 / 2,098 / 307 |
| `drive290` / `drive311` from `1ff0061f` / from the stop | 1,414 / 571 / 1,098 |

Building the full label table takes about 140 CPU s per state. Each trial letter costs about 0.4 s.

## Not run, not finished, open

- **Not run:** Windows, DirectX and native checks (headless Linux only); `--three`; no Codex or other model call (by
  the line's rules).
- **Not proved:** a stall proves only that the enumerated set fails, not that no unlocking word exists. The routes
  are the first ones found in the stated order, not shortest or best ones. N rises along every route; this line does
  not judge N.
- **Not done:** I did not continue the descent after the H6 letters, for example by applying R22D-81's rule to the
  end states. At the cap 290 and cap 311 end states, 159 pieces from `1ff0061f` are still at height 6; at the stop
  route's end state, 172 pieces are still at height 6.
