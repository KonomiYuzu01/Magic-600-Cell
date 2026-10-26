# Stage 17: routing releases in the driver (25 October 2026)

Status: **research; P\*\*\* has been run from the nine stops of P\*\* and stops again (section 3).** Stage 16 found tame routing releases
at the jams of card 8 (O-SJ-D7 to O-SJ-D9 and I-7304), but its driver P** did not search them: the runs that used them
were started with them by hand ([stage 16, section 2.4](../16-six-jams/README.md#24-the-procedure-p-as-run)). On 25
October 2026 the owner asked to put routing releases into the procedure and run it from the nine configurations where
P** stopped. This stage declares the family RR (Lemma RR of stage 16 with the cover test of Lemma OT), adds it to the
stop search (the procedure P***), checks it against the known descents, and runs P*** from the nine stops.

**Findings (computed).** From the nine stops of P**, P*** continues at eight; at 7303 (Φ 536) it does not move. The
lowest Φ per seed, 7302 to 7307, is now 388, 536, 438, 386, 1158 and 1100, against 494, 536, 554, 508, 1260 and 1114
with P** (section 3.2). RR finds the release of O-SJ-D7 by itself, makes the two releases of O-SJ-D8's first macro one
at a time, and with three helpers finds a drop like O-SJ-D9's (section 2). At every stop where P*** ends, RR finds
thousands of releases and no tail of its forms descends after any of them (section 3.3): the bound to widen next is
the tails, not the releases.

Labels: **proved** (with the proof named), **computed** (exact, by the program named), **lead**, **open**. Everything is
headless Linux research code on exact geometry; nothing here is Windows, DirectX, input or performance evidence.

## 1. Family RR

The terms are those of stages 14 to 16 (labels, clean and blocked grips, Φ, F_x and D_x(h), partners, carriers,
positions, obligated sets, the cover test). At a configuration Z:

- **Pairs.** Every blocked grip r and every grip c clean at Z that holds every straddler of r (the setting of Lemma OT).
- **Routing words.** For the pair (c, r): every carrier position sequence of n ≤ n_max actions at c that passes the
  cover test, with at most H helper letters at cover opportunities (a helper at position j acts at a grip that is clean
  in the carrier-only state (c, p_j)Z and holds there a piece of the obligated set B_{p_n}), whose helpers together
  touch every piece of B_{p_n}, with every retained item at each helper; every letter must be admissible when it acts,
  and r must have no straddler after the word (a **release**). This is the word language of stage 16 `routing.py`.
- **Tails.** After a release, every word of at most two retained letters, at most one of them at a grip other than r,
  followed by a lowering (x, j) at a grip x that is clean and holds a high piece of its own branch (Lemma RR with at
  most one letter u besides the releaser). The reductions leave five forms: (a) (x, j) right after the routing word;
  (b) u (x, j), u at a clean grip y other than r and x a partner of y; (c) (r, m) (x, j), x a partner of r;
  (d) (r, m) u (x, j), u at a clean grip y, x a partner of y, and r a partner of y or of x; (e) u (r, m) (x, j), u at a
  clean partner of r and x a partner of r.
- **Candidates.** Every routing word and tail whose lowering has F_x < D_x(j). The word is tame: its letters are
  retained except the last, so its excursion is 0 and it lowers Φ by 2(D_x − F_x) (Lemma RR, stage 16).

Bounds of this stage: n ≤ 2, and n = 3 only where n ≤ 2 gave no candidate; H ≤ 2.

**Reductions (each exact for the best candidate by terminal Φ and length; computed checks in section 2).**
1. *Canonical words.* Two carrier letters with no helper between them compose to one carrier letter (Lemma RA, stage
   15), so a word whose middle position has no helper reaches the configuration of the word with the two letters
   composed. That word passes the cover test with the same helper positions, so it is enumerated (or it has no carrier
   letter, and then it is not a routing word). Only words with a helper at every middle position are built.
2. *Equal configurations.* Words that reach the same configuration of the search rows (stage 15 `RetainedSearch`) have
   the same tails, so the tails are searched once per configuration and the shortest word (then the least) is kept.
3. *Tails.* A tail letter that is not a partner of a later letter or of the target commutes past them and can be
   removed without changing F and D at the target (the minimality reduction R3, stage 16), which gives a shorter word of
   the same language with the same terminal Φ; in (e) a letter u that is not a partner of r commutes with the releaser
   (form (d)), and two letters at one grip in a row are one letter (Lemma RA). So the five forms give the best
   candidate per target of the whole tail language. The lowering needs x clean after the last letter and a high piece
   of the branch of x inside x (otherwise D_x = 0); both are read before any metric.

**What RR does not search** (a stop of P*** is a stop of these bounds, not a negative certificate): routing words with
more than two helper letters (O-SJ-D9 has three and I-7304 four) or more than three carrier actions (I-7304 has four);
helpers that are not cover opportunities (for example a helper made clean only by an earlier helper); two releases
whose words are concatenated (the first macro of O-SJ-D8); carriers enabled by a retained letter, or several carriers
for one blocked grip; tails with two letters besides the releaser; and jumble letters before the final lowering (T1,
T2).

## 2. Checks

**The gate** (`scripts/test_rr.py`; computed, one thread, at the 7306 jam with Φ 1490; 1,339 assertions, all passed,
667 wall seconds on the integrator's Linux run):

| Row | What it checks | Result |
| --- | --- | --- |
| H0 | The cached metric of `rr.py` equals stage 15 `RetainedSearch.metric`: 41 grips after each of six random retained letters | equal on all 246 reads |
| H1 | Canonical words, pair 598 → 536, n ≤ 2, one helper: every word of the unreduced language (`routing_words(..., canonical=False)`, the language of stage 16 `routing.py`) that is not canonical reduces, by composing its carrier letters, to an enumerated canonical word or to a word without a carrier letter, and reaches the configuration of its reduction | 418 words, 198 canonical; all 220 others reduced and compared |
| H2 | The tails against a brute force over the whole tail language (every word of at most two retained letters at clean grips, at most one at a grip other than r, then every lowering), as the best (terminal Φ, length) per target: after `561:1,598:4` (r = 536), and (H2b) after `561:1,598:4,536:11,233:6,240:3`, where the routing word 240:9 releases 90 and the lowering 370:15 follows at once (the configuration of review finding RR-001) | equal at both; at H2b the lowering at 370 has (1268, 1) |
| H3 | RR at the jam with n = 1 and two helpers, pair 598 → 536 | 22 words, 4 releases; best `561:1,598:4,536:11,233:6,370:15` to Φ 1268, O-SJ-D7 with its commuting letters 233:6 and 536:11 exchanged; certified by stage 15 `macro.execute` with excursion 0 |
| H4 | `pxrr.py` refuses an existing output before any load | refused |

**O-SJ-D9 with three helpers** (computed, `results/rr-7304-23-41.json`, 2,871 CPU s). At the 7304 jam (Φ 1172), RR
for the pair 23 → 41 with n ≤ 3 and three helpers (one more than P*** uses): no carrier position sequence with one or
two actions passes the cover test and 22 with three do; they give 63,888 words, among them the routing word
`23:10,20:2,112:2,23:5,20:1,23:7` of O-SJ-D9 (enumerated by `rr.routing_words`); 39,732 are admissible, 7,129 release
41, in as many configurations, and 6,839 of those have a descending tail, all at target 2. The best is
`23:2,20:2,23:6,20:1,112:6,23:11,41:1,2:14` to Φ 554 (F 0, D 309 at 2), the drop and the length of O-SJ-D9 with other
carrier items and helpers and another end (`73178765…`, O-SJ-D9 ends at `7ef6e3a2…`); it was certified from the jam
with excursion 0 and replayed (`results/certificates/RR-7304.json` and `-check.json`). I-7304 (four carrier actions and four helpers) and O-SJ-D8
at 7305 (two releases) are outside RR's bounds (section 1).

## 3. P*** and its runs

### 3.1 The procedure P***

P*** is P** ([stage 16, section 2.4](../16-six-jams/README.md#24-the-procedure-p-as-run)) with family RR in its stop
search:

- A segment is P* with the declared options (families T, R, M, E and L; E₀ = 20; R to three retained letters with a
  beam of 64) until it stops.
- At a stop it searches family R without a beam and family X with K = 1 and 2, and with K = 3 and the minimality
  reduction when K ≤ 2 found nothing, as P** does. When R and X found nothing, it searches RR with n ≤ 2 and two
  helpers, and with n = 3 when n ≤ 2 found nothing. The best candidate by P*'s key (terminal Φ, excursion, length,
  word) is the first macro of the next segment, certified again before it is committed.
- It stops when R, X (K ≤ 3) and RR (n ≤ 3, two helpers) have no candidate at a stop: a stop of the declared search,
  not a negative certificate. A run that reaches its CPU limit has the outcome unknown.

It is state-only and commits only certified macros that lower Φ with excursion at most 20 (RR and X words are tame),
so it ends after at most Φ/2 macros (Proposition P, stage 16). The runs start at the nine stops of P**: each applies
the start macros and the committed macros of its P** run as start macros (`--start`), so a run of P*** is P** followed
by P*** from the stop.

### 3.2 Runs

All runs are headless on Linux, one thread per process, with `scripts/pxrr.py` (`--nmax 3 --helpers 2`, CPU limit
28,800 s; section 5). `results/runs/pxrr-<run>.json` (`<run>` is `7302p`, `7303`, `7304`, `7305`, `7305-D8`,
`7305-OD3`, `7306`, `7306-D7` and `7307p` in the order of the table below) keeps the record of each run: its options
(the label cache of the stop replaced by a note), its start, the macros of each segment with Φ after them, and per
stop the counts of the stop search (R, X per K, RR per bound with its pairs, words, releases and hits), the best RR
candidates and the choice. The label caches and the per-step records stay outside the repository.

| Run | Start (stop of P**) | Φ after each committed macro (family) | Outcome | CPU s |
| --- | --- | --- | --- | --- |
| 7302 | 7302′, Φ 494 | 398 (RR2), 388 (RR3) | stop at 388 | 18,484 |
| 7303 | 536 | (none) | stop at 536 | 19,584 |
| 7304 | after I-7304, 554 | 442 (RR2), 438 (X1) | stop at 438 (stop search by separate runs, below) | 11,098 + «CPU438» |
| 7305 | from the jam, 1538 | 1476 (RR2), 1422 (X1), 1182 (RR2), 508 (R), 392 (X3), 386 (RR2) | stop at 386 | 12,405 |
| 7305 | after O-SJ-D8, 730 | 508 (RR2) | stop at 508 | 8,187 |
| 7305 | after O-D3, 508 | 412 (RR2), 406 (R), 386 (X1) | stop at 386 | 10,919 |
| 7306 | from the jam, 1482 | 1260 (RR2), 1202 (RR2), 1162 (T), 1158 (R) | stop at 1158 | 5,629 |
| 7306 | after O-SJ-D7, 1260 | 1202 (RR2), 1162 (T), 1158 (R) | stop at 1158 | 4,413 |
| 7307 | 7307′, 1114 | 1100 (RR2) | stop at 1100 | 15,731 |

RR2 and RR3 are RR with n ≤ 2 and with n = 3. The RR macros (computed; each certified by the driver before it was
committed and again in the certificates below):

| Run | Stop Φ → Φ | Word | Carrier c → released r, target x | F, D at x |
| --- | --- | --- | --- | --- |
| 7302 | 494 → 398 | `274:7,357:2,274:10,430:5,523:15` | 274 → 430, 523 | 0, 48 |
| 7302 | 398 → 388 | `59:5,64:4,59:3,2:1,59:11,50:9,136:13` (n = 3) | 59 → 50, 136 | 8, 13 |
| 7304 | 554 → 442 | `581:8,540:7,508:2,582:4,540:15` | 540 → 582, 540 | 2, 58 |
| 7305 | 1538 → 1476 | `421:11,574:8,520:10,421:11,389:4,421:3,389:12` | 421 → 389, 389 | 27, 58 |
| 7305 | 1422 → 1182 | `94:1,134:1,305:2,389:5,504:12` | 134 → 305, 504 | 0, 120 |
| 7305 | 392 → 386 | `577:2,520:1,577:10,484:1,538:12` | 577 → 538, 538 | 0, 3 |
| 7305 after O-SJ-D8 | 730 → 508 | `549:6,576:7,549:7,552:1,576:7,549:13` | 549 → 552, 549 | 0, 111 |
| 7305 after O-D3 | 508 → 412 | `421:9,574:10,520:10,421:7,389:4,421:9,389:12` | 421 → 389, 389 | 10, 58 |
| 7306 | 1482 → 1260 | `561:1,598:4,536:11,233:6,370:15` | 598 → 536, 370 | 0, 111 |
| 7306, both runs | 1260 → 1202 | `198:1,336:2,208:10,336:10,188:14` | 336 → 208, 188 | 0, 29 |
| 7307 | 1114 → 1100 | `543:1,494:1,543:8,597:8,546:14` | 543 → 597, 546 | 0, 7 |

At 7306 RR finds the release of O-SJ-D7 by itself (with its commuting letters 233:6 and 536:11 exchanged). From the
7305 jam's own stop it makes the two releases of O-SJ-D8's first macro one at a time, each with a tail that descends:
389 by the carrier 421 (with other helpers, 1538 → 1476), and later 305 by the carrier 134 with the router 94
(1422 → 1182), where O-SJ-D8 needs both before its lowering. After an RR macro, P*'s own families and X descend again:
at 7305 family R lowers 1182 to 508 at once.

**The 7304 run** was stopped by the integrator in its stop search at Φ 438, where RR has 778,272 routing words with
n = 3 (a count by `rr.py --count-only`). The run had used 11,098 CPU s when that stop search began, so it would have
passed its CPU limit inside it, which the limit does not bound (section 4); its record ends with segment 2 and the
status `unknown`, as saved when the stop search began. The same stop search was computed by separate runs with the
same inputs (the word of the record and the label cache the run wrote at 438): stage 15 `pstar2.py --stop-only` for
family R, stage 16 `transport.py` for X with K = 1, 2 and 3 (minimal), and `rr.py` for RR in three groups that together
hold the 19 pairs, with n ≤ 2 and then n = 3 (`results/stop-7304-438/`, «CPU438» CPU s in all). None has a candidate,
so the run stops at 438.

### 3.3 The stops

At every stop where P*** ended, family R had no candidate, X had none with K = 1, 2 and 3, and RR had none with n ≤ 3
and two helpers (computed, the stop records in `results/runs/` and, for 7304, `results/stop-7304-438/`):

| Stop | Φ | Digest | X leaf tests, K = 1 / 2 / 3 | RR, n ≤ 2: pairs, words, releases | RR, n = 3: words, releases | CPU s |
| --- | --- | --- | --- | --- | --- | --- |
| 7302 | 388 | `266ec63e…` | 1,632 / 369,340 / 924,725 | 14, 8,206, 1,062 | 59,895, 8,548 | 6,259 |
| 7303 | 536 | `f3fc8837…` | 312 / 53,053 / 148,505 | 25, 36,258, 8,003 | 205,700, 53,446 | 19,483 |
| 7304 (separate runs) | 438 | `eacffbb3…` | 463 / 118,556 / «X3LEAF» | 19, 167,464, 21,246 | 778,272, «N3REL» | «CPU438» |
| 7305 from the jam | 386 | `550ff26a…` | 0 / 33 / 1,369 | 14, 9,874, 2,099 | 66,066, 13,006 | 4,949 |
| 7305 after O-SJ-D8 | 508 | `f88cbcd4…` | 5 / 1,416 / 8,906 | 13, 10,200, 1,947 | 90,871, 17,141 | 6,559 |
| 7305 after O-D3 | 386 | `ae0353b6…` | 0 / 4 / 365 | 16, 17,586, 2,657 | 135,036, 23,691 | 8,540 |
| 7306 from the jam | 1158 | `fc1fcd0a…` | 1,142 / 177,920 / 82,697 | 20, 2,689, 679 | 19,481, 3,300 | 2,072 |
| 7306 after O-SJ-D7 | 1158 | `78377052…` | 1,142 / 177,920 / 82,697 | 20, 2,689, 679 | 19,481, 3,300 | 2,133 |
| 7307 | 1100 | `7d6b6613…` | 1,219 / 271,778 / 305,023 | 21, 20,990, 2,945 | 113,982, 21,193 | 11,125 |

Each is a stop of the declared search, not a negative certificate (section 3.1). At every one of them RR finds
releases, 3,979 to «MAXREL» routing words per stop with n ≤ 3 after which the blocked grip has no straddler, and no
tail of the declared forms descends after any of them: the stops of P*** are stops of the tails, not of the releases.
The two runs at 7306 end at Φ 1158 in different configurations with the same counts. As in P**, the CPU limit bounds
the segments but not the stop search: the longest stop search in a run took 19,483 CPU s (7303), and the one at 7304's
stop at 438 took «CPU438» CPU s in its separate runs.

### 3.4 Certificates

The whole descent of each run, its start macros followed by its committed macros, was certified from its jam with
stage 15 `macro.py cert` (excursion allowance 20, and 838 for the run after O-D3) and checked with `macro.py check
--replay` (computed, `results/certificates/PX3-<run>.json` and `-check.json`). All checks passed, and each ends at the
digest of its last stop above.

## 4. Tools (`scripts/`)

| Script | What it does |
| --- | --- |
| `rr.py` | Family RR at one configuration (the jam, or the jam followed by `--word`): `--nmin`, `--nmax` carrier actions, `--helpers`, optional `--pairs c:r,...`; `--count-only` counts the routing words. Writes the counts per pair and the best candidate per target. |
| `pxrr.py` | P***: stage 16 `pxrun.py` with RR in the stop search where R and X found nothing (n ≤ 2, then n = 3 when `--nmax 3`). Optional start macros (`--start`). |
| `test_rr.py` | The gate (section 2). |

They import the stage 14 to 16 scripts and change none of them, and they write only outside the repository. The
records carry the format names of the stage 16 record helper (`s16-rr/1`, `s16-pxrr/1`). `results/stop-7304-438/`
holds the separate stop search at 7304's stop at 438 (section 3.2): `rr-n2-*.json` and `rr-n3-*.json` for the three
groups of pairs (`134-172`, `463-458` and `other`; `s16-rr/1`), `x1.json`, `x2.json` and `x3-minimal.json`
(`s16-transport/1`) and `r.json` (the record of stage 15 `pstar2.py --stop-only`). The runs of section 3 were
given the label caches of the P** stops (`--labels`, outside the repository), which only spares the label computation:
the stage 16 loader reads a cache only when its digest equals the state's, and `pxrr.py` passes stage 15 only the copy
it writes into its own output directory. In `pxrr.py`, as in stage 16 `pxrun.py`, the CPU limit bounds the segments
but not the stop search. The integrator's helpers that turned the run records into start words and certificate
commands are not committed; section 5 gives the same commands.

## 5. Reproduction

From the repository root, with one thread per process (`OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1`),
`S=research/jumbling/restoration/17-routing-in-driver/scripts`, `S15=research/jumbling/restoration/15-lowering-machine/scripts`,
`R16=research/jumbling/restoration/16-six-jams/results/runs` and `D` a directory outside the repository:

    python3 -I -B $S/test_rr.py
    python3 -I -B $S/rr.py 7304 $D/rr-7304-23-41.json --nmax 3 --helpers 3 --pairs 23:41
    python3 -I -B $S/pxrr.py 7306 $D/pxrr-7306-D7 28800 --start "$START" --nmax 3 --helpers 2

`START` is the descent of the P** run whose stop the run starts from, its start macros followed by its committed
macros, separated by `;`; it is read from that run's record (`px-7302`, `px-7303`, `px-7304-I7304`, `px-7305`,
`px-7305-D8`, `px-7305-OD3`, `px-7306`, `px-7306-D7` and `px-7307` in `$R16`):

    START=$(python3 -I -c "import json, sys; s = json.load(open(sys.argv[1])); print(';'.join(','.join(f'{g}:{i}' for g, i in m) for m in s.get('start_macros', []) + [w for seg in s['segments'] for w in seg['words']]))" $R16/px-7306-D7.json)

`--labels` may add a label cache of the stop. The separate stop search at 7304's stop at 438 is, with `W` the word of
the 7304 run's record up to that stop (its start macros and the words of its segments, joined by `,`), `T` and `P`
the table and the word of the 7304 jam (stage 16 `jams.json`; `P` is its chain followed by its macros) and
`S16=research/jumbling/restoration/16-six-jams/scripts`:

    python3 -I -B $S/rr.py 7304 $D/rr-n3-134-172.json --word "$W" --nmin 3 --nmax 3 --helpers 2 --pairs 134:172
    python3 -I -B $S16/transport.py 7304 3 $D/x3-minimal.json --minimal --word "$W"
    python3 -I -B $S15/pstar2.py $T $D/r --prefix "$P,$W" --stop-only

and likewise `--nmin 1 --nmax 2` and the other groups of pairs (the pairs of each group are in its record), and
`transport.py` with K = 1 and 2 without `--minimal`. The certificates of section 3.4 and of O-SJ-D9's drop (section 2)
are

    python3 -I -B $S15/macro.py cert $T $D/ROW.json --prefix $W --labels $D/jam.npz --excursion 20 --macro M1 [--macro M2 ...]
    python3 -I -B $S15/macro.py check $T $D/ROW.json $D/ROW-check.json --labels $D/jam.npz --replay

with `T` and `W` the table and the word of the jam (stage 16 `jams.json`), the macros of the descent in order, the
allowance 838 for the run after O-D3 and 0 for `RR-7304`. `macro.py` loads with the stage 14 loader, which rewrites a
label cache whose digest differs from the state (stage 16, section 2.6): give it a cache of the jam itself, or a new
path.
