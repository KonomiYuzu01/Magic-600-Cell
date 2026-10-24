# Jumbling cost leads: results (EN2 slice 3)

Status: measured on 22 October 2026 for packet ENG-0J revision 2, acceptance
item 5 (`docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md`).
The scripts and their commands are in [`../README.md`](../README.md).

**Evidence kind.** Source and fixture replay with the J1 reference engine
(`research/jumbling/sim/`) and the research restorer
(`research/jumbling/theory/`), and exact geometry of the retained model. Every
run was headless, in a Linux cloud container with four CPUs, CPython 3.11.15
and NumPy 2.3.5 (`tools/python/engine.txt`). A time is a cloud timing of
single-threaded Python research code while other measurement processes shared
the four CPUs (the load averages are in each summary and in the run table). It
is a lead for the engine-language question, never performance evidence for a
product engine, a renderer or Windows/DirectX.

**Boundary.** These are measurements, not solver aids. Where a script walks
caps in an order (X1) or reproduces a research choice (X4), the order or the
choice is a measurement input from the census record.

## The leads at a glance

Each line is a lead about J1 and the retained model, for EN3 and EN4 to
confirm or overturn with a product engine; the sections below give the
numbers, the checks and the caveats.

- **X1.** A repair list is a job: in J1 one list takes 3.5 s to about
  3 minutes, and one step's walk down to the repaired cap a median of
  2.3 minutes. Every list matched the research records.
- **X2.** u at a cap the solver names costs one full straddler set, a median
  of 0.5 to 0.6 s in J1. u for all 600 caps costs ten to twelve times the
  early-exit classification of all caps (296 s and 370 s in J1).
- **X3.** A whole-word preview costs 7 to 41 ms a letter in J1, depending on
  the state rather than the word's length. Proposed for E3: refuse a word
  built from named parts whose expansion exceeds 2²¹ letters.
- **X4.** M4's exact covers differ from the research float test only where d
  is blocked: 1,506 of 514,800 pairs at the 144 cover steps, each traced to
  pieces that straddle d, none to rounding; all 126 recorded flags agree. One
  covers query takes 4.2 s (median) in J1.
- **X5.** One exact release check takes 0.09 s (median) at the census states
  and 1.4 s at the `wj-I_a` end. The classification of the named cap and its
  domains is about half of that at the census states and four fifths at the
  `wj-I_a` end; N_c after the one item costs milliseconds.
- **XM1.** The closing set of every face-match step lies in one seat, so
  DT-M1's Q7 answer (a), four seat keys, stands.
- **Heights.** Under the I_a menu an entry's height is unbounded: the X1
  witness H6030 needs 3,018 bits, and its sign tests 3,036-bit operands. The
  restoration's words (correction 35, u1, u12 and b20's closure) keep every
  entry at 8 or below.
- **Classification.** In J1 the filtered classification of all 600 caps takes
  1.5 to 2.2 times as long as the exact one at each of the three states, the
  3,018-bit entries included, and agrees with it at every cap.
- **Meshes.** Every exposed face at full detail is 5,760,000 triangles, 276 MB
  as a float32 triangle soup before any sharing or instancing. The retained
  sticker asset holds 4.2 times the exact sticker triangles.

## Results

### X1: repair lists along the seed 7101 walk

The repair-list job at each of the 201 (step, cap) requests of the seed 7101
record's measurement walk: at each of its 63 repair steps, the misaligned caps
by decreasing N_c, through the cap the record repaired (DT-J1 E13). The run
used two workers, each replaying its steps with `census.walk`.

| | Smallest | Median | Largest | Total |
|---|---|---|---|---|
| Requests per step | 1 | 2 | 28 | 201 |
| Pairs per step | 118 | 1,062 | 18,113 | 128,502 |
| One list (any request) | 3.5 s | 54.0 s | 188.9 s | 11,048 s |
| One list at the repaired cap | 14.2 s | 83.3 s | 188.9 s | 5,755 s |
| One list before the repaired cap | 3.5 s | 40.1 s | 118.2 s | 5,294 s |
| All lists of one step | 14.2 s | 135.4 s | 1,232.6 s | 11,048 s |
| Ordering a step's caps by N_c | 4.4 s | 6.2 s | 17.2 s | 435 s |
| Time per pair | 45.9 ms | 72.0 ms | 287.2 ms | 86.0 ms over all pairs |

Every list completed, and every check passed: the request and pair counts
equal the record's, the repaired cap sits at the recorded place, every list
before it is complete and empty, the record's own pair is row 1 at the
repaired cap, and every outcome at all 63 steps equals `density.py`'s
recorded rows (48 of the steps from the extracted density records). The
lists at the repaired caps have 1 to 61 rows, 5 at the median, 679 in all. A
list's time follows its number of pairs (correlation 0.78).

For DT-J1's question whether a walk key (O1, option (c)) is usable: in J1 one
list takes from 3.5 s to about 3 minutes, and walking one step's caps down to
the repaired cap takes a median of 2.3 minutes and at most 20.5 minutes. So in
J1 the list is a job, as E13 has it, and a walk that lists each cap in turn
waits about a minute per cap. A product engine's answer needs its own
measurement (EN3 and EN4); DT-J2, which fixes the walk, is on hold.

### X2: early exit against full straddler sets

At each of the 600 caps, J1's early-exit classification (it stops at the
first certified straddler) and then the full certified straddler set of the
cap (every straddler, with J1's certificate construction inside the time), on
the same state. Each certificate was then checked by independent point checks
outside the timing.

| State | Off pieces | Blocked caps | u at the blocked caps: median, quartiles, largest | Early exit: total, median per cap | Full set: total, median per cap | Full set / early exit: median per cap, totals |
|---|---|---|---|---|---|---|
| height-4 state (census `ia-k3000-s7302` end and 12 letters) | 103,896 | 562 | 186.5; 120 and 259; 592 | 29.8 s, 28.7 ms | 296 s, 481 ms | 17.2, 9.9 |
| 3000-twist scramble (seed 7302) | 118,821 | 563 | 251; 182 and 329; 734 | 30.5 s, 26.6 ms | 370 s, 607 ms | 23.4, 12.1 |

At every cap of every state the two agreed on the status, the certainty and
the first straddler, every certificate passed its point checks, and no piece
was left uncertain. Every admissible cap had u = 0 and every blocked cap
u ≥ 2. The gain of the early exit is at the blocked caps, where it stopped
after a median of 10,063 and 7,509 exact evaluations, against 123,565 and
129,898 for the full set (height-4 state and scramble). At an admissible cap
neither can stop early (medians of 211 and 241 ms for the early exit against
191 and 213 ms for the full set). The two calls share
J1's caches: the early exit runs first at each cap and builds the cap's anchor
groups and cap geometry, which the full set then reuses, and piece geometry
cached during one cap's full set can serve the next cap's early exit. Neither
time is a cold time, and the ratio is a lead, not a cost model.

For DT-J1's question whether u may order, filter or mark caps (E1, E16): an
order, filter or map mark by u needs the full sets of all 600 caps at every
revision, ten to twelve times the early-exit classification of all caps; in
J1 that is 296 s at the height-4 state and 370 s at the scramble. u at one cap
the solver names (A2, M2) costs one full set: medians of 0.48 and 0.61 s and
at most 1.27 and 1.72 s in J1 at the two states. This is a
cost lead about J1. The answer for a product engine needs that engine's
measurement (EN3 and EN4), and the question itself stays with DT-J1.

### X3: whole-word previews

Each word was applied once, letter by letter, from its recorded start state,
with each letter's classification and application timed; a refusal would have
ended the preview at that prefix. No letter was refused in any word.

| Word | Start | Letters | Total | Per letter: mean, median, largest | Slowest 100 letters, mean | Off before, after | Recorded result checked |
|---|---|---|---|---|---|---|---|
| u1 | census 7101's end | 624 | 12.0 s | 19.2 ms, 15.1 ms, 235 ms | 46.9 ms (letters 301 to 400) | 1,193, 1,121 (72 landed) | input and output digests |
| u12 | census 7101's end | 1,158 | 37.7 s | 32.5 ms, 18.9 ms, 201 ms | 70.3 ms (601 to 700) | 1,193, 1,152 (41 landed) | input and output digests |
| correction 35 | solved | 5,629 | 85.8 s | 15.2 ms, 12.9 ms, 123 ms | 19.4 ms (3,201 to 3,300) | 0, 0 (at most 3,121 at the sampled steps) | output digest |
| H6030 | solved | 3,156 | 130.0 s | 41.2 ms, 40.7 ms, 91 ms | 47.9 ms (its last 56 letters) | 0, 52,883 (all lifted off) | output digest |
| b20's closure | b16's end | 40,500 | 298.1 s | 7.4 ms, 7.3 ms, 58 ms | 10.9 ms (20,201 to 20,300) | 507, 447 (60 landed) | end digest, off counts, landed pieces and the exact poses of the others (`03-b20-apply.log`) |

In u1, u12 and b20's closure every piece that stayed off kept its exact pose.
The time per letter follows the state rather than the position in the word. It
stays between 6.0 and 10.9 ms in every 100 letters of b20's closure, grows
from 40 to 48 ms over H6030 while its entries grow to 3,018 bits, and rises
four to five times inside u1 and u12 where the pose table or the off count
grows: u1's letters 301 to 400 average 46.9 ms with up to 3,376 poses
(sampled), against 9.5 ms in its first 100 letters, and u12's letters 601 to
700 average 70.3 ms at 4,159 to 4,297 off pieces and about 2,740 poses (the
samples at letters 600 and 700), against 17.1 ms in its first 100. Within a
word it tracks the count of exact evaluations (correlation 0.92 in u1 and 0.87
in correction 35).

**A proposed bound for E3, for EN3 to adopt or change.** DT-J1 E3 and D20
refuse an expansion beyond what the store and the replay can hold, with its
length shown, and leave the bound to X3. The longest literal word played on
record has 672,417 letters, and E2's 13 literal steps hold 2,012,155 in all;
the lattice word after them (166,788,405 letters) was checked by composition,
not played, and a straight-line program such as b19's V expands to fewer than
10¹⁴¹ letters. Proposed: refuse a word built from named parts whose expansion
exceeds 2²¹ = 2,097,152 letters. The bound admits every word played on record,
and E2's 13 literal steps joined into one; it refuses the lattice word and
every larger expansion, with the length shown. A word at the bound takes about
4 MB in the compact form of two bytes a letter (the `ia16` words of the
tutorial runner on draft #112's branch,
`research/jumbling/restoration/13-tutorial-runner/README.md` at `ec0f0ac`). A
flat stored word keeps E1's rule of no length cap. At the whole-word rates
measured here (7.4 to 41.2 ms a letter), J1 previews 672,417 letters in 1.4 to
7.7 hours and 2,097,152 letters in 4.3 to 24 hours; the lattice word would
take 14 to 80 days. So a long preview is a job, and the duration shown before
a run must come from the product engine's own rate (EN3 and EN4), not J1's.
Because the rate changes more than tenfold with the state (6.0 ms in b20's
fastest 100 letters to 70.3 ms in u12's slowest), a duration shown from one
fixed rate can be wrong by that factor. Playback time is separate: W-J's 190
ms per turn (RO-24) is a renderer figure, not part of X3.

### X4: exact covers against the float test

M4's exact relation "d covers c" (`covers.exact_covers`) against the research
float test (`covers.float_covers`, the strict centroid test of
`seams.cover_graph`) on the replayed census records. Both select pieces of c's
face and apply the same rule: d covers c when the selected pieces lie as many
on each side of c's cut, at least 10 on each, and all of them are aligned. The
exact relation selects the pieces in d's certified inside set, which never
holds a piece that straddles d's cut; the float test selects the pieces whose
float centroid lies beyond d's cut depth.

- At each of the 144 cover steps of the seven seeds that have them, every
  admissible misaligned cap c against each of its 80 meeting caps d (21 to 85
  misaligned caps a step).
- At each of the 63 repair steps of seed 7101, the two flags recorded before
  the repair in `runs/findability/ia-k40-s7101-steps.jsonl` (c covers d at 30
  steps, d covers c at 26).

A disagreement is traced to the pieces of c's face that one test selects and
the other does not: to a blocked d when d is blocked and every such piece
straddles its cut, to rounding when d is not blocked and every such piece's
float margin is under 10⁻⁹, and otherwise left unexplained.

| Comparison | Steps | Pairs | Agree | Disagree | Blocked d | Rounding | Unexplained |
|---|---|---|---|---|---|---|---|
| cover steps, seven seeds | 144 | 514,800 | 513,294 | 1,506 | 1,506 | 0 | 0 |
| recorded flags, seed 7101 | 63 | 126 | 126 | 0 | – | – | – |

By seed, at the cover steps (the time is one whole cover step: every pair of
every misaligned cap):

| Seed | Cover steps | Misaligned caps a step, median | Pairs | Disagreements | Steps with a disagreement | Time a step, median |
|---|---|---|---|---|---|---|
| 7101 | 7 | 25 | 13,600 | 88 | 7 | 78 s |
| 7102 | 21 | 55 | 95,440 | 406 | 21 | 241 s |
| 7103 | 33 | 49 | 128,400 | 37 | 24 | 423 s |
| 7104 | 15 | 43 | 53,040 | 255 | 15 | 188 s |
| 7105 | 16 | 37.5 | 48,800 | 89 | 16 | 308 s |
| 7108 | 19 | 37 | 67,680 | 403 | 19 | 239 s |
| 7109 | 33 | 42 | 107,840 | 228 | 32 | 316 s |

Every disagreement is traced to a blocked d. Each of the 7,736 differing
pieces straddles d's cut: the float test selects it by its centroid and the
exact relation does not. The float margins of these pieces run from 0.0041 to
0.85, except for one piece: at 15 cover steps of seed 7109, piece 175746, in
the pair where c is cap 551 and d is cap 550, has a float margin of 3.6·10⁻¹⁵,
so the float test's choice for that piece rests on rounding; it straddles d's
cut, so the trace counts it with the blocked d. The answer flips both ways:
only the exact relation covers in 834 pairs, only the float test in 672. The
1,506 disagreements are 359 distinct (seed, c, d) pairs, and 241 of these
recur at two or more cover steps of their seed (one at 32); 134 of the 144
cover steps have at least one. The float test reproduces all 126 recorded
flags, and the exact relation agrees with each. This bears out E15's claim
that M4's test can differ from the float test only where d is blocked or by
rounding: every disagreement lies at a blocked d, and none was traced to
rounding at an unblocked d.

As a measurement of the research rule, not a rule offered to the solver: the
research choice (`cover_tops`, then the largest drop) made from the float
relation reproduces the cap that the record repaired at all 144 cover steps,
and made from the exact relation it picks the same cap at every step.

**Time in J1.** One covers query, option (b) of O3 (the inside sets of the
named cap and its 80 meeting caps, the faces, and both answers for each
meeting cap), timed at the repaired cap of each of seed 7101's 63 repair steps
before the repair: median 4.2 s (quartiles 4.0 and 4.5 s), largest 12.0 s. The
parts' medians are 1.5 s for the inside sets, 2.4 s for the faces and 0.36 s
for the answers. The four queries above 8 s are the run's first four (steps
30, 32, 33 and 35); after them the largest is 5.0 s. A whole cover step, every
pair of every misaligned cap, took 40 to 558 s (median 303 s), 84.1 ms a pair
over all 514,800 pairs.

### X5: release checks

One exact release check, timed per part, at the first twist of each of the 63
repair steps of seed 7101 and at the 27 misaligned admissible caps of the
`wj-I_a` end state (item 1): 90 checks, each confirmed afterwards by the
research `scores` oracle outside the timing.

| States | Checks | Total: median, largest | Parts, median: classification and domains | cut sets | N_c after one item | height counts |
|---|---|---|---|---|---|---|
| seed 7101, repair steps | 63 | 0.089 s, 0.119 s | 45.1 ms | 27.0 ms | 1.5 ms | 10.4 ms |
| `wj-I_a` end | 27 | 1.43 s, 1.58 s | 1.13 s | 15.3 ms | 4.0 ms | 244 ms |

Every check agreed with the oracle. The classification of the named cap and
its domains is the largest part: about half of a check at the census states
and four fifths at the `wj-I_a` end (121,184 pieces off the lattice), where it
is about 25 times dearer. N_c after the one released item takes a few
milliseconds. No check found a piece above left height 4, before or after the
item: n₄ reached 4,639 at the `wj-I_a` end and was 0 at the census states.

### XM1: face-match seats

All 124 face-match steps (`rule == 'seam'`) of the census records, over nine
seeds (seed 7109 has none). The right cosets A4₀·s computed by exact
composition equal the pinned seats.

| Case (lattice flags of the cut's two sides) | Steps | Closing set Z: 1 item | 3 items | 12 items | Z in one seat |
|---|---|---|---|---|---|
| outside on the lattice | 78 | 32 | 5 | 41 | 78 |
| inside on the lattice only | 0 | – | – | – | – |
| both | 4 | 3 | 1 | 0 | 4 |
| neither | 42 | 42 | 0 | 0 | 42 |

The record's own item is in Z at every step, in Z's seat. The case that would
split Z over the seats (only the inside on the lattice) never occurs in the
census, so DT-M1's Q7 recommendation (a), four seat keys, stands
(`docs/progress/1.0/design-table/topics/command-keyboard-model.md`, XM1).

### Entry heights and denominators

`wj.height`'s height and denominator (the largest |a|, |b| or d over the
non-K⁺ poses of the live pose table) after every step, with the first
crossing of each width found at every step. Every input replayed completely;
every recorded stage, digest and cap-0 check passed.

| Input | Steps | Largest height | Largest denominator | First step above 2³¹, 2⁵³, 2⁶³, 2⁶⁴ | Largest pose table | Largest off count |
|---|---|---|---|---|---|---|
| `wj-I_a` | 841 | 16 (step 116) | 16 | none | 1,335 | 124,762 |
| `wj-S4` | 640 | 32 (step 210) | 32 | none | 1,620 | 123,784 |
| `wj-I_b` | 885 | 32 (step 851) | 32 | none | 1,253 | 130,165 |
| J1 journal E2 | 1 | 860,378,847,541 (40 bits) | the same | 2³¹ at step 1 | 2 | 3,097 |
| J1 journal E3 | 2 | 3,441,515,390,164 (42 bits) | the same | 2³¹ at step 1 | 4 | 3,097 |
| J1 journal random_0 | 6 | 2,314,538,318,160 (42 bits) | the same | 2³¹ at step 4 | 5 | 9,291 |
| J1 journal random_1 | 5 | 550,844,356 (30 bits) | the same | none | 10 | 6,194 |
| negative control | 24 | 708,588 (20 bits) | the same | none | 2 | 3,097 |
| X1 witness H6030 | 3,156 | 3,018 bits (step 3,156) | 3,018 bits | 170, 192, 202, 203 | 177 | 52,883 |
| correction 35 | 5,629 | 8 | 8 | none | 446 | 3,121 |
| u1 (from census 7101's end) | 624 | 8 | 8 | none | 4,311 | 6,512 |
| u12 (from census 7101's end) | 1,158 | 8 | 8 | none | 2,794 | 4,685 |
| b20's closure (from b16's end) | 40,500 | 8 | 8 | none | 890 (sampled every 100 steps) | 507 (sampled) |

The W-J fixtures and the restoration's words (correction 35, u1, u12 and b20's
closure) keep every entry at 32 or below. Larger entries come from three
inputs: the J1 journals of free twists at arbitrary angles (no menu), three of
which pass 2³¹ within one to four steps; the negative control of J1 acceptance
item 9, whose heights grow by design; and the X1 witness H6030, a word of
3,156 I_a menu letters from solved, which shows that heights under the menu
are unbounded. H6030's entries need 65 bits from step 203 and 3,018 bits at
its end, and its exact sign test's operands reach 3,036 bits (`--widths`,
every 10th step recorded). No fixed-width integer holds them.

### Exact against filtered classification

At each of the 600 caps both modes ran once untimed, then the exact call and
the filtered call (amendment A2) were timed on the same state, with
`need_inside=True`. Status, inside set and blocked flag agreed at every cap of
every state, and no J1 cache grew during a timed call.

| State | Off pieces | Exact: total, median per cap | Filtered: total, median per cap | Exact evaluations (exact mode) | Filtered evaluations, exact fallbacks (filtered mode) |
|---|---|---|---|---|---|
| `wj-I_a` end | 121,184 | 16.7 s, 17.7 ms | 29.2 s, 26.4 ms | 8,911,799 | 8,894,680, 259,382 |
| H6030 end (entries of 3,018 bits) | 52,883 | 8.8 s, 12.6 ms | 19.5 s, 18.7 ms | 5,643,435 | 5,484,739, 236,303 |
| 3000-twist scramble (seed 7302) | 118,821 | 26.3 s, 23.1 ms | 40.6 s, 30.0 ms | 13,242,640 | 13,402,843, 351,198 |

In J1 the filtered classification takes 1.5 to 2.2 times as long as the exact
one (totals), also at H6030's 3,018-bit entries. J1's exact test is Python
integer arithmetic, one vertex row at a time
(`research/jumbling/sim/kernel.py`, `Evaluator.sign_row`). The filtered test
first builds NumPy float arrays for the vertex set and the pulled-back normal,
and falls back to the exact test where the error bound does not decide
(`filtered_signs`). This is a lead about J1, not about a product engine. The
first run of the fixture state, before the review fix that warms both modes
(B-01), gave 18.6 s exact and 31.1 s filtered; it is superseded and not kept
here.

### Exposed-face meshes

For each of the 36 K⁺-orbit representative regions (177,120 pieces in all),
the 3-faces on the cut hyperplanes and on the facet hyperplanes, found in exact
arithmetic, each with an Euler check, and scaled by the orbit sizes. Triangles
are counted by fan triangulation of each exact polygon; a triangle stores three
4D float32 points (48 bytes).

| Faces | 3-faces | Vertices (summed per face) | Triangles | Bytes as float32 |
|---|---|---|---|---|
| exposed cut faces, base (36 regions) | 235 | 1,060 | 1,180 | 56,640 |
| exposed cut faces, full puzzle | 939,000 | 4,039,200 | 4,322,400 | 207,475,200 |
| stickers (facet faces), full puzzle, exact | 259,800 | 1,238,400 | 1,437,600 | 69,004,800 |
| stickers, retained float32 asset (`assets/mesh.json`), full puzzle | 259,800 | – | 6,096,000 | 292,608,000 |

So a mesh of every exposed face, cuts and stickers together, has 5,760,000
triangles (276 MB as a float32 triangle soup) at full detail, before any
sharing of vertices or instancing by orbit. The retained sticker asset holds
4.2 times the exact sticker triangles. `build_assets.py` builds each sticker
with SciPy's ConvexHull of floating point region vertices (line 33), and the
stored triangles show four kinds of excess: stored points that are not
float32-rounded exact vertices (1,652 distinct points), near-coincident points
(distinct stored points within 10⁻⁹ of one exact vertex, an exact zero stored
as a tiny nonzero value; 255), collapsed triangles (3,948) and repeated
triangles (761). Every sticker with more triangles than the exact count shows
at least one of them (`unattributed_stickers` is empty). Stickers 268, 322 and
399 are tetrahedra whose only excess is 1, 2 and 3 near-coincident points;
their 6, 8 and 10 triangles are exactly 2V − 4 for V = 5, 6 and 7.

## Runs

Every run ran this package's code from the repository checkout, with
`python -B`, outputs outside the repository and NumPy 2.3.5 from
`tools/python/engine.txt` in a virtual environment outside it. At most four
measurement processes ran at once on the four CPUs (X1 and `x4-b` used two
workers); the one-minute load average at a run's start and end is in the
table.

| Output directory | Command (`L` = `research/jumbling/differential/leads`) | Started (UTC) | Code at start | Wall time | Load at start, end |
|---|---|---|---|---|---|
| `x5` | `L/x5_release.py` | 06:54:23 | `e95d00c` | 251 s | 0.58, 4.09 |
| `xm1` | `L/xm1_seats.py` | 06:54:27 | `e95d00c` | 1,438 s | 0.77, 5.01 |
| `x1` | `L/x1_walk.py --rest INPUTS/ia-k40-s7101-density-rest.jsonl.gz --jobs 2` | 06:54:23 | `e95d00c` | about 6,687 s | 0.58, 4.09 |
| `x2-h4` | `L/x2_straddlers.py --record INPUTS/ia-k3000-s7302.json --letters INPUTS/h4-bbef1c8e-letters.json --expect bbef1c8e…` | 07:01:58 | `08f7291` | about 3,255 s | 4.11, 4.04 |
| `heights-main` | `L/heights.py` | 07:18:25 | `8fa2e3d` | 814 s | 5.01, 4.16 |
| `heights-u12` | `L/heights.py --word INPUTS/u12.json census 7101` | 07:31:59 | `52e2dd8` | 58 s | 4.16, 4.63 |
| `heights-b20` | glue `heights` (below) | 07:32:57 | `52e2dd8` | 308 s | 4.63, 4.08 |
| `heights-h6030-widths` | `L/heights.py --h6030 --widths --every 10` | 07:38:05 | `13bec8a` | 139 s | 4.08, 4.15 |
| `meshes-2` | `L/meshes.py` | 07:40:31 | `13bec8a` | 5 s | 4.06, 4.06 |
| `classify-fixture-2` | `L/classify_cost.py --fixture I_a` | 07:40:36 | `13bec8a` | 254 s | 4.06, 4.09 |
| `classify-h6030` | glue `classify` (below) | 07:44:50 | `13bec8a` | 193 s | 4.09, 4.12 |
| `x3-main` | `L/x3_previews.py` | 07:51:13 | `13bec8a` | 250 s | 3.12, 4.00 |
| `x3-u12` | `L/x3_previews.py --word INPUTS/u12.json --start census 7101 --label u12` | 07:55:23 | `13bec8a` | 58 s | 4.00, 4.11 |
| `x3-b20` | glue `x3` (below) | 07:56:21 | `13bec8a` | 303 s | 4.11, 4.06 |
| `x2-scramble` | `L/x2_straddlers.py --scramble 3000 7302` | 07:56:13 | `13bec8a` | 3,036 s | 4.04, 3.40 |
| `x4-b` | `L/x4_covers.py --seeds 7103 7109 --jobs 2` | 08:24:38 | `13bec8a` | 15,271 s | 3.03, 1.00 |
| `classify-scramble` | `L/classify_cost.py --scramble 3000 7302` | 08:46:49 | `13bec8a` | 2,596 s | 3.40, 4.00 |
| `x4-a1` | `L/x4_covers.py --seeds 7102 --jobs 1` | 09:20:28 | `13bec8a` | 5,255 s | 3.06, 4.05 |
| `x4-a2` | `L/x4_covers.py --seeds 7101 7001 7106 7107 --jobs 1` | 09:30:14 | `13bec8a` | 946 s | 3.85, 4.15 |
| `x4-a3` | `L/x4_covers.py --seeds 7108 --jobs 1` | 09:46:01 | `13bec8a` | 4,568 s | 4.15, 4.00 |
| `x4-a4` | `L/x4_covers.py --seeds 7105 --jobs 1` | 10:48:08 | `13bec8a` | 4,928 s | 3.97, 2.07 |
| `x4-a5` | `L/x4_covers.py --seeds 7104 --jobs 1` | 11:02:09 | `13bec8a` | 2,693 s | 4.00, 3.00 |

"Code at start" is the commit checked out when the run started; the `commit`
field inside a run's summary is the commit when the summary was written, which
is later for a long run. The ends of `x2-h4` and `x1` were logged by watchers
polling every 15 s, so their wall times are good to 15 s. `heights-b20`
started 44 s before `13bec8a` was committed, so it may have read that commit's
`census.py` from the working tree; the two differ only in the centroid cache's
path check.

**Glue scripts.** Two inputs start from states that no command line of this
package builds, so short scripts outside the repository called the package's
functions:

- b20's closure (`x3`, `heights`): loads b16's end into a J1 state from the
  pose table `z-02-b16.json.gz` (digest `fde9e0fd…` checked), forms the
  closure V·Q·V⁻¹ from b20's V word and core Q (P is empty; inverse letters by
  `menu_order.inverse_index`; 40,500 letters), and calls
  `x3_previews.preview(state, letters, menu, label='b20-closure',
  sample_every=1000)` or `heights.run_heights(state, twists,
  label='b20-closure', every=100)`. The end digest `aab10f25…` is checked, and
  for X3 also the recorded effect in `03-b20-apply.log` (507 to 447 off, 60
  landed, the other 447 exact).
- H6030's end (`classify`): applies H6030 from solved through
  `heights.h6030()`, checks `heights.H6030_DIGEST`, and calls
  `classify_cost.run([('h6030:end', state)], out)`.

X4 ran in six parts: `x4-b` ran the two seeds with the most cover steps (7103
and 7109, 33 each) on two workers, and the other eight seeds ran as five
single-worker parts (`x4-a1` to `x4-a5`), each started when a CPU came free.
A first X4 run over all seeds on one worker (from 08:03:50) was stopped after
seed 7101 and is not kept. From 11:47 to 11:50 this package's five test files
ran on the two CPUs that `x4-a5` and one `x4-b` worker had freed, beside
`x4-a4` and the other `x4-b` worker.

Superseded runs, not kept: the first classification run at the fixture state
(before the warm-up fix B-01) and the first mesh run (before the sticker
attribution fix B-03).

## Provenance of the runs

The scripts changed while the runs went on: the full review's fixes (`8fa2e3d`)
and the two verification fixes to the centroid cache (`52e2dd8`, `13bec8a`)
landed after the first runs had started. A run's summary records the commit at
the moment it was written, which for a long run can be later than the code it
started on, so the run table names the commit each run started on (the HEAD at
its start; Python reads the scripts when a run starts).

Runs started on `e95d00c` (`08f7291` changed only two README files): X5, XM1
and X1, and X2 at the height-4 state. What the later commits change in the
code these runs use:

- `census.walk` and `census.scramble` also check the scramble's off count
  against `off_scramble` and (N, off) at a continued round's start, as the
  census replay check does. Every census record of `RECORDS` passes both
  checks (the X4 runs walked all ten with them on `13bec8a`), so these runs
  walked the same states the final code walks. X2's height-4 record
  (`ia-k3000-s7302`, outside `RECORDS`) passes them too: its scramble has
  118,821 pieces off, which is its `off_scramble` (X2 and the classification
  at the scramble both show this count), and its `N_start` and `off_start`
  equal step 1's `N_before` and that count.
- The centroid cache and every output file are refused when they resolve
  inside the repository. These runs wrote outside it.
- X1 reads a repair list's rows only when the list completed. Every list in
  this X1 run completed, so its output is the one the final code gives.

Runs started on `8fa2e3d`: the heights of the main inputs. On `52e2dd8`: the
heights of u12 and of b20's closure. Their code differs from the final commit
only in the centroid cache's path check. Every other run started on `13bec8a`.

## Inputs outside this repository

Read from draft #112's branch (commit `ec0f0ac`) with `git show`, outside the
repository. Each digest is that of the committed blob, and every extracted file
equals it byte for byte. The two round-22 logs were read for the 12 letters of
the height-4 state and not extracted.

| File at `ec0f0ac` | SHA-256 | Used by |
|---|---|---|
| `research/jumbling/restoration/11-scramble-census/runs/findability/ia-k40-s7101-density-rest.jsonl.gz` | `e795b9bc5612916f1f8f6820de4659236484cbbdbc32cba1d77c794129180052` | X1 (`--rest`) |
| `research/jumbling/restoration/11-scramble-census/runs/heights/ia-k3000-s7302.json` | `29b2c1b6f2e40a271a03c8eefa1ea3e3166c69d360c0ecf6683d096811d07c86` | X2 height-4 state (`--record`) |
| `research/jumbling/restoration/12-low-height/runs/round22/r22d75_height_first.txt` | `e546df6ff33ec1b12028f4e897695f20dd4bbeb19c5a6b49f8bf3dc6344a7219` | X2 height-4 state: letters 1 to 7 |
| `research/jumbling/restoration/12-low-height/runs/round22/r22d81_rule.txt` | `60050aa4dd4eff5646d4a06e546538fb2afac381791695f219c6d8966e246d59` | X2 height-4 state: letters 8 to 12 |
| `research/jumbling/restoration/12-low-height/runs/round22/t1prime-k40/steps/u12.json` | `66f735c65e1c6756b7b74ce666a74b3c9e7eb70c662843e44cb38b4c16909cc4` | X3 and heights: u12 |
| `research/jumbling/restoration/12-low-height/runs/round22/line-a/literal/run1/z-02-b16.json.gz` | `b659492c5495131cb7eff0073097677d3bf5763d3e66dc2f7fe58a9ac08941e5` | X3 and heights: b20's start (the b16 end) |
| `research/jumbling/restoration/12-low-height/runs/round22/line-a/literal/run1/03-b20-V-word.json.gz` | `1ae91e59bc4be23a02fa58fa4b4f24d165b47f188c8f63785d331775c9c8bb9a` | X3 and heights: b20's V |
| `research/jumbling/restoration/12-low-height/runs/round22/line-a/replay2/inputs/cores/b20-core.json` | `b5abdc4c23cef1969cd149a70bf284107ce570406b7604c8f1856b1282b5dcef` | X3 and heights: b20's core Q |
| `research/jumbling/restoration/12-low-height/runs/round22/line-a/literal/run1/03-b20-apply.log` | `1778b87c3916fe533cc70bccc3f408c4fbedc6577d9c6c2503e738de7b7c5f08` | b20's recorded effect |

The height-4 state of X2 is the end of `ia-k3000-s7302` (digest `015d839e…`)
after these 12 letters, as (grip, item) in the pinned I_a order: letters 1 to 7
from `r22d75_height_first.txt` (all single letters were exhausted after them)
and letters 8 to 12 from `r22d81_rule.txt`:

```text
[[64, 15], [185, 14], [357, 12], [471, 12], [263, 14], [407, 13], [523, 12],
 [124, 2], [274, 13], [46, 1], [46, 3], [104, 14]]
```

The run read them from `h4-bbef1c8e-letters.json` (SHA-256
`77e1de5ecb77b1db651c064b8f4791a2a345dc4d4cccd60e2569015a009213e4`) and
checked the end digest
`bbef1c8ec0de78dead1473322a8c1236e48f9ccfe0017ce724461bb7f37ae299`
(`--expect`), the digest that `r22d81_rule.txt` records at its hand-off:
103,896 pieces off, none at left height 6. W-J's deep export on `main`
(`work/experiments/renderer-wj/deep/bbef1c8e-letters.json`) derived the same
12 letters independently, from `r22d25_d19_stops.py` at `ec0f0ac`, with the
same start and end digests.

## Output files

Each run's directory holds the files its script writes (a summary JSON and a
JSON Lines file of rows), with private paths replaced: `OUT` for the scratch
results directory, `INPUTS` for the extracted inputs, `SCRATCH` for other
scratch paths and `.` for the checkout. A JSON Lines file over 200,000 bytes
is stored gzip-compressed (`.jsonl.gz`, no file name or time stamp in the
header). The `.out` files of the runs (their standard output and error) were
empty and are not kept.

| Directory | Files (bytes) |
|---|---|
| `classify-fixture-2` | `classify-caps.jsonl.gz` (210,241), `classify-summary.json` (1,901) |
| `classify-h6030` | `classify-caps.jsonl.gz` (1,793,955), `classify-summary.json` (1,821) |
| `classify-scramble` | `classify-caps.jsonl.gz` (286,464), `classify-summary.json` (1,836) |
| `heights-b20` | `heights-steps.jsonl` (74,568), `heights-summary.json` (1,974) |
| `heights-h6030-widths` | `heights-steps.jsonl.gz` (78,728), `heights-summary.json` (415,925) |
| `heights-main` | `heights-steps.jsonl.gz` (823,492), `heights-summary.json` (34,590) |
| `heights-u12` | `heights-steps.jsonl.gz` (7,877), `heights-summary.json` (2,366) |
| `meshes-2` | `meshes-orbits.jsonl` (117,627), `meshes-summary.json` (317,804) |
| `x1` | `x1-requests.jsonl` (84,899), `x1-summary.json` (36,086) |
| `x2-h4` | `x2-caps.jsonl.gz` (310,230), `x2-summary.json` (10,299) |
| `x2-scramble` | `x2-caps.jsonl.gz` (388,486), `x2-summary.json` (10,463) |
| `x3-b20` | `x3-letters.jsonl.gz` (689,934), `x3-summary.json` (7,264) |
| `x3-main` | `x3-letters.jsonl.gz` (192,636), `x3-summary.json` (17,574) |
| `x3-u12` | `x3-letters.jsonl.gz` (25,653), `x3-summary.json` (3,958) |
| `x4-a1` | `x4-pairs.jsonl.gz` (34,320), `x4-summary.json` (1,411) |
| `x4-a2` | `x4-pairs.jsonl` (188,629), `x4-summary.json` (16,559) |
| `x4-a3` | `x4-pairs.jsonl.gz` (37,502), `x4-summary.json` (1,405) |
| `x4-a4` | `x4-pairs.jsonl` (113,700), `x4-summary.json` (1,411) |
| `x4-a5` | `x4-pairs.jsonl.gz` (19,402), `x4-summary.json` (1,406) |
| `x4-b` | `x4-pairs.jsonl.gz` (13,512), `x4-summary.json` (1,583) |
| `x5` | `x5-checks.jsonl` (84,047), `x5-summary.json` (826) |
| `xm1` | `xm1-steps.jsonl.gz` (576,023), `xm1-summary.json` (4,364) |

In all, 7,044,751 bytes in 22 directories.

## Checks not run

- No Windows/DirectX, input, long-session or performance check: these are
  headless Linux runs of research code, and no time here is evidence for a
  product engine, a renderer or Windows.
- No time was measured on an idle machine: up to four measurement processes
  shared the four CPUs, so the times are comparable within a run but carry
  the load in the run table.
- No product engine was measured. Every conclusion about a product engine
  waits for EN3 and EN4.
