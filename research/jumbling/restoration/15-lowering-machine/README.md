# Stage 15: the lowering machine (24 October 2026)

Status: **research; the joint attack on card 7 is done (section 2).** Card 7 is problem 2 of the theorem programme
that the owner set on 24 October 2026
([decision](../../../../docs/wiki/decisions/owner-decisions-2026-10-24-theorem-program.md)), together with the part
of problem 1 that card 6 left open: an order of unlocking and lowering that does not jam. The precedence peel of
[stage 14](../14-blocking-structure/README.md) (Theorem T3P) lowers the depth Φ at the six ends of the D2 height phase
by 6% to 56% and then has no round; this stage asks what lowers Φ after that. The problem card is
[card 7](../../theory/cards/card-7-lowering-machine.md).

Labels: **proved** (with the proof in [`PROOFS.md`](PROOFS.md)), **computed** (exact, by the script named),
**reported** (computed during the attack by an analyst's own program, which is not committed; "both" when both
analysts computed it independently), **lead**, **open**. Everything is headless Linux research code on exact
geometry; nothing here is Windows, DirectX, input or performance evidence.

## 1. Terms

The terms of stage 14 (section 1) and card 6 hold: labels in, out and straddle; S_d and I_d; clean and blocked grips;
heights, branches and second vertices; Φ = Σ max(0, h − 2); the foreign load F_x and the lowering count D_x(h); units,
high units and own holders; partners (the 80 grips that interact with a grip); levels; rounds of T3P. New here:

- A **word** is a sequence of letters (grip, item), applied left to right. Its **excursion** from a configuration Z is
  the largest rise of Φ above Φ(Z) along it, the last letter included (0 if Φ never rises). A word is **tame** when no
  letter raises Φ; a word with excursion 0 need not be tame.
- A **macro** is a word that ends with a jumble (its **lowering**) and lowers Φ. The **support** of a letter is the set
  of pieces it moves: the whole inside set of its grip at the moment it acts.
- A **re-aim** is a retained letter at the target itself. Consecutive letters at one grip compose, so a lowering
  followed by a re-aim is the lowering with another item of the same coset (Lemma RA).
- An **enabler** is a letter that makes a helper clean without touching the target's cap.
- The **chain ends** are the six configurations where the greedy chains of T3P rounds stopped (stage 14, section 5.7):
  the D2 end tables followed by the chain words.

  | End | D2 end table (`13-tutorial-runner/runs/dev/`) | Chain word | Digest | Φ |
  | --- | --- | --- | --- | --- |
  | 7302 | `d2-s7302/component-checkpoint/tables/00021.json` | `277:59,171:15,39:1,171:14` | `b47d8474…` | 1376 |
  | 7303 | `d2-s7303/run2/d2-s7303/tables/00138.json` | `364:5,282:13,236:12,24:7,69:14` | `4c0234bf…` | 1680 |
  | 7304 | `d2-s7304/component/tables/00005.json` | `344:9,451:55,522:14` | `f93e461f…` | 1498 |
  | 7305 | `d2-s7305/component/tables/00010.json` | `421:4,577:1,520:14,233:9,281:12,6:45,44:15,208:13,168:15` | `e77981a8…` | 2474 |
  | 7306 | `d2-s7306/component-checkpoint/tables/00017.json` | `208:5,336:15,445:55,252:13,506:7,561:14` | `5e9f7e4f…` | 2684 |
  | 7307 | `d2-s7307/component/tables/00005.json` | `516:14,597:10,543:13,10:8,147:2,102:15,216:2,266:14,275:1,435:12,435:7,319:12,94:12` | `52aa8a30…` | 1396 |

## 2. Joint attack (24 October 2026)

Card 7 was attacked as the owner decided on 24 October: two sealed analyses of the same card, by Astra (call
`20261024T164544Z-108217b4`) and by Claude Opus (a subagent of the integrator's session); the integrator's exact replay
of every descent that either analysis reported; a cross-examination, in which each analyst answered every finding of
the other (Astra's: call `20261024T191955Z-365eff91`); then exact experiments for the points still in dispute, run by
the integrator with tools that Astra wrote from the merged statements (implementation call
`20261024T195417Z-064452c4`, reviewed by the integrator). The analyses are private review records; this section
records what they established. The proofs are in [`PROOFS.md`](PROOFS.md); each was checked line by line by the
analyst who did not write it and by the integrator.

### 2.1 The results (proved; `PROOFS.md`)

- **Label tables along a word (RS, CR, CL, SL).** A word of retained letters is decided by permuting rows of the label
  table at the start: admissibility, the moved sets, labels, heights and branches after every letter, and S and F at
  any grip x after one letter (read in the column k⁻¹x). D needs more than the table: the second vertices, from the
  exact poses, counted at k⁻¹x for the conjugated jumble k⁻¹hk. Every item at a grip permutes the 120 poles of its
  grip's class, so after a jumble the labels of the moved pieces at grips of that class are a column permutation too;
  at the other classes they need an exact image.
- **The chronological macro certificate (Theorem MC).** Any admissible word is certified event by event on the exact
  poses: the acting grip is clean, the support is its whole inside set, and the changes δ_t of Φ telescope, so the
  certificate gives the descent, the exact excursion and tameness. Letters may repeat grips, overlap and act at the
  target. Most tests are table tests at the start; a jumble followed by a letter at a grip of another class needs an
  exact image. It certifies all seventeen descents of section 2.2.
- **Constructions inside it.** Retained release (SRR): retained letters, admissible when they act, then a lowering
  with F < D, a tame macro; thirteen of the seventeen descents. The precedence peel with an excursion (T3PE): T3P with
  P2 dropped and a net-descent test, three descents with excursion 20. Re-aim (RA): a retained letter at the target
  chooses among the twelve placements of the lowered set, all read from one table. Conjugated lowering (CJ): u, a
  lowering, then u⁻¹, when the lowered cap's pieces stay out of u's grips; no instance found.
- **Release structure (FL, AO).** Every grip has a finite level at every reachable configuration (reverse the history),
  so finite levels carry no information. The release structure of a target is an AND/OR graph over the cells of its
  straddlers (pieces with identical holder rows), whose min–max value is the level; every word that cleans the target
  contains a schedule of it. Its leaves carry obligations (an item that leaves nothing across the grips still to act,
  and the cost of its image) that no theorem guarantees. Foreign pieces enter only through the final budget F < D.
- **Negatives (1C, BN, OH, NJ).** Words confined to one clean cap act as one of its 60 items, so the 60 items decide
  every one-cap word. A negative certificate with excursion E must be closed under every letter that stays below the
  ceiling, raising letters included. If no piece of height at least 4 lies in a cap of its own branch's class, every
  descent first raises Φ. Φ-neutral jumbles do not keep straddle numbers: from solved, (0, 12) keeps Φ = 0 and makes
  31 pieces straddle grip 1.
- **The lowering machine (GC, LC1, LC2, RP, V).** For two clean caps x and z whose inside sets meet exactly in a unit
  G: a finite controller (G's pose and the products at x and z, kept in the Good sets) makes every allowed word
  admissible, with excursion at most twice the foreign load of the two debris sets, independent of the length. Two
  four-letter lens cells return every other piece and move G by a lowering item. The reverse pump lowers G at every
  letter, but when the product at the grip that acted last on G is not retained, that grip's return raises G again
  unless G has first left its cap or been landed by the other grip; L9 returns one cap's debris only. Whether a whole
  pump exists is a finite viability computation; X1's isolation lemma carries over, its perturbation and covering
  lemmas do not.
- **The procedure P* (Proposition P).** Certified macros from five families (section 2.4), each committed only when
  `macro.py` certifies that it lowers Φ (so by at least 2) with excursion at most E₀; so at most Φ/2 macros, and Φ
  never exceeds its start by more than E₀. A stop records which families were searched exhaustively.

### 2.2 The descents (exact)

Every word below starts at a chain end. The integrator replayed each from the D2 end table followed by the chain word
with `14-blocking-structure/scripts/replay_word.py`: every letter admissible, every height recomputed from scratch.
Both analysts replayed them too. Each row was then certified with `macro.py cert` (Theorem MC) from its chain end, with
the excursion allowance 0 (20 for R03, R15 and R16), and every certificate passed `macro.py check --replay`, which
replays the word independently and recomputes every height after every letter (computed,
`results/certificates/RNN.json` and `RNN-check.json`).

| Row | End | Target: status, level | Word after the chain end | Family | Φ profile | End digest |
| --- | --- | --- | --- | --- | --- | --- |
| R01 | 7302 | 104: budget, 0 | `104:8,46:4,104:15` | SRR, re-aim | 1376, 1376, 1376, 1350 | `1db0dd54…` |
| R02 | 7303 | 140: no clean holder, 2 | `282:5,364:1,95:4,140:15` | SRR | 1680, 1680, 1680, 1680, 1660 | `699f3015…` |
| R03 | 7303 | 24: no feasible items, 1 | `69:50,24:13` | T3PE | 1680, 1700, 1604 | `f5d4c242…` |
| R04 | 7304 | 162: no clean holder, 2 | `277:1,375:1,384:7,162:13`, then `375:15` | SRR, then T3P | 1498, 1498, 1498, 1498, 1276, 1228 | `a7be8c10…` |
| R05 | 7304 | 522: budget, 0 | `522:8,451:8,522:15` | SRR, re-aim | 1498, 1498, 1498, 1472 | `6131c203…` |
| R06 | 7304 | 522: budget, 0 | `522:8,451:44,522:15` | re-aim, neutral jumble | 1498, 1498, 1498, 1472 | `956d3d3e…` |
| R07 | 7305 | 10: budget, 0 | `10:6,102:2,10:14` | SRR, re-aim | 2474, 2474, 2474, 2450 | `ec5c0592…` |
| R08 | 7305 | 113: budget, 0 | `250:4,208:9,113:15` | SRR | 2474, 2474, 2474, 2468 | `12e5f34a…` |
| R09 | 7305 | 177: no clean holder, 2 | `134:1,47:1,177:12` | SRR | 2474, 2474, 2474, 2360 | `5130ce11…` |
| R10 | 7305 | 281: budget, 0 | `281:4,168:1,281:12` | SRR, re-aim | 2474, 2474, 2474, 2450 | `2aab4185…` |
| R11 | 7305 | 499: budget, 0 | `499:1,574:1,499:15` | SRR, re-aim | 2474, 2474, 2474, 2344 | `6b87535a…` |
| R12 | 7305 | 538: no feasible items, 1 | `484:1,499:8,577:3,538:12` | SRR | 2474, 2474, 2474, 2474, 2468 | `850fe2af…` |
| R13 | 7306 | 202: no clean holder, 2 | `233:1,200:7,202:12` | SRR | 2684, 2684, 2684, 2682 | `47baf08f…` |
| R14 | 7306 | 561: budget, 0 | `561:6,598:1,561:15` | SRR, re-aim | 2684, 2684, 2684, 2650 | `4c348086…` |
| R15 | 7306 | 240: no feasible items, 1 | `487:12,240:14`, then `240:1,487:14` | T3PE, then T3P | 2684, 2704, 1852, 1852, 1832 | `eb25daf3…` |
| R16 | 7306 | 240: no feasible items, 1 | `487:33,240:14` | T3PE | 2684, 2704, 1850 | `348070c1…` |
| R17 | 7307 | 102: budget, 0 | `102:8,67:2,10:1,102:12` | SRR, re-aim | 1396, 1396, 1396, 1396, 1284 | `63e9f6c3…` |

Status and level of the target are those at the chain end (`14-blocking-structure/results/s730N-t3p-chain-end-scan.json`
and `s730N-chain-end-levels.json`; "no clean holder" is the status straddler without a clean holder). R04's second
macro and R15's second macro act at the state after the first. R16 ends with n6 = 10 and R17 with n6 = 7; all other
rows end with n6 = 0.

- 15 own holders descend, at all six chain ends: 8 of the 32 with status budget, 4 of the 84 with a straddler without
  a clean holder (all four at level 2), 3 of the 10 with no feasible items, none of the 5 with no disjoint cover. The
  K2 statuses are statuses of the T3P scan's family, not obstacles to descent.
- Thirteen rows are tame retained releases; seven of them start with a re-aim at their target. R06 is tame with a
  jumble helper of ΔΦ = 0 (F = D at 451). R03, R16 and the first macro of R15 have a helper that raises Φ by 20
  (excursion 20) and then lower by 96, 854 and 852.
- None of the fourteen tame rows is a T3P round: each has a piece moved twice among its helpers (R02, R04, R08, R09,
  R12, R13; overlaps of 70 to 473 pieces) or a letter at the target (R01, R05, R06, R07, R10, R11, R14, R17)
  (Claude Opus, reported).

### 2.3 Facts at the chain ends

- **One cap (Proposition 1C; both, computed).** At the 7304 chain end, 522 has F = 70 and D = 22 for twelve jumbles,
  0 for the other 36; 540 has F = 111 and D = 58 for twelve, 0 for 36. At the 7307 chain end, 516 has F = 395 and D = 7
  for twelve, 0 for 36. No word confined to one of these caps lowers Φ: the least changes are +96, +106 and +776, and
  49, 54 and 389 foreign pieces would have to leave first. The menu at each of the three grips is closed under
  composition (all 3,600 products are menu items, exact matrices).
- **540 (Claude Opus, reported).** After each of the twelve re-aims at 540, the T3P scan of 540 and of its partner own
  holders finds no round; for all three units at 540, F stays 111 and D 58 for every orientation and cover.
- **Grip 2 of 7304 (Claude Opus, reported).** The unit of piece 41541 (198 pieces, height 4, branch 0) has whole
  holders 2, 4, 31 and 36, none of them clean, and its members have six distinct holder rows. Grip 2 has 101 straddlers
  in 10 cells and 168 foreign pieces in 11 cells, and no cell has a clean holder. Every cell has the same single
  level-1 holder, 41, whose cap holds all 269 pieces and no member of the unit; all 132 straddlers of 41 lie in the
  clean cap 23. So the release structure (Proposition AO) is the chain 23 → 41 → 2. No item at 23 cleans 41: the best,
  the retained item 1, moves all 132 off cut 41 but puts 9 other pieces of cap 23 across it, and after it the only
  clean holder of those 9 (other than 2 and 41, and avoiding the unit) is 23 itself.
- **The two-letter family at 229 (DIS-4; computed, `results/s7304-chain-end-pairs-229-a.json` and `-b.json`).** At the
  7304 chain end, `t3p.py pairs 229 295 347` evaluated all 7,200 words (both orders, 60 × 60 items): 6,900 admissible,
  1,380 tame, and 19 of the tame ones leave 229 clean. The least F − D at 229 after a tame word is 25, from (295, 6),
  (347, 7) with F 34 and D 9, so no tame two-letter word at 295 and 347 followed by a jumble at 229 lowers Φ. At the D2
  end (stage 14, section 5.3) the same enumeration also gave 25, but with 23 clean words: the states differ (the chain
  letter (451, 55) moves pieces that are in or across the three grips, 243 piece–grip labels at the D2 end and 277 at
  the chain end), and only the completed enumeration at the chain end settles the minimum there.
- **Clean pairs (DIS-2 and DIS-3; computed, `results/census-7304.json`, `results/census-7307.json`,
  `results/census-7304-prep.json`).** At the 7304 and 7307 chain ends (41 and 40 clean grips), and at 7304 after the
  retained preparation 277:1, 375:1, 384:7 of R04 (42 clean grips), no piece of height at least 4 lies in two clean
  caps. So neither lens cell (LC1, LC2) nor a pump of Proposition RP has an instance at these three configurations:
  each needs two clean caps that share a high piece.

### 2.4 The procedure P*

P* repeats one step until Φ = 0, until no family offers a macro (a **stop**), or until a resource limit (an
**unknown** outcome). A step searches five declared families of macros at the current configuration, replays every
candidate exactly, takes the one with the least key (terminal Φ, excursion, length, word), certifies it again at the
current configuration (Theorem MC) and commits it. `macro.py` admits a candidate of any family only when it lowers Φ
and its excursion is at most E₀ = 20 (families T, R and M have excursion 0 by construction). The targets are the own
holders of the high units.

| Family | Words | Search |
| --- | --- | --- |
| T | T3P rounds (stage 14, Theorem T3P) | the `t3p.py` scan, exhaustive in its family; when rounds exist, the same covers are re-optimised with P*'s key |
| R | retained letters at grips that matter for the target, then a lowering at the target (Corollary SRR; re-aims included) | exhaustive through two retained letters, then a beam of 64 per target ranked by (straddlers, F − max D) to three letters; every letter read from the table (Lemma RS) |
| M | a re-aim at a clean target, one tame helper letter at a clean partner (retained, or a jumble with F ≤ D), then a lowering at the target | exhaustive: 12 re-aims × clean partners × tame items × 48 lowerings, one exact image per helper coset |
| E | T3P with the per-letter tameness P2 dropped (Lemma T3PE): helpers may raise Φ by at most E₀ = 20 in total, and the round must lower Φ | exhaustive over T3P's covers and items within the raising budget |
| L | the lens cells LC1 and LC2 at clean pairs whose caps share a high piece | exhaustive over both forms, both orientations and all items; every reported cell replayed with its collateral checked |

The helper sets of R are the clean target itself and the clean partners holding a piece that matters (an obstruction
piece or a high piece of the target's branch), plus the clean grips holding straddlers of the blocked partners that meet
the obstruction. "Exhaustive" means the family's whole declared set was evaluated; R is exhaustive only through two
letters, so a stop of P* is a stop of its declared search, not a negative certificate for SRR. A stop record lists,
for each family, what it searched and whether that search was exhaustive; an exhaustive empty family is a negative
certificate for that family only (Propositions BN and P). Every committed macro is certified by `macro.py`, and the
run's certificate is checked again by an independent replay that recomputes all heights after every letter.

### 2.5 Runs

All runs are headless on Linux, one thread per process, with the commands of section 5. `results/runs/` keeps a
trimmed record of each run (its options, Φ after each committed macro and, for every step, the chosen macro and per
family the number of candidates, whether the search was exhaustive and its CPU time) with the certificate of its
committed macros; the label caches and the per-candidate records stay outside the repository.

**Declared runs (DIS-5).** P* with the declared options (section 2.4) from each chain end. The runs at 7305 and 7306
were stopped by the integrator at 922 and 699 CPU seconds (they ran at the lowest priority) and continued from the
configuration they had reached as the first segment of their two-level runs below, which is the same procedure: a step
of P* depends only on the configuration.

| End | Φ after each committed macro | Families | Outcome |
| --- | --- | --- | --- |
| 7302 | 1376, 1338, 1324, 1094 | M, T, R | stop |
| 7303 | 1680, 1496, 1420, 1194, 1130, 1088, 974, 936 | R, E, T, R, R, R, T | stop |
| 7304 | 1498, 1300, 1252, 1236, 1210 | R, R, R, R | stop |
| 7305 | 2474, 2226, 2064, 1950, 1860, 1810, 1690, 1666, 1616, 1596, 1578, 1572 | R, R, R, R, R, T, R, R, T, R, R | stop |
| 7306 | 2684, 1814, 1780, 1764, 1750, 1738, 1700, 1698 | R, R, R, R, R, R, T | stop |
| 7307 | 1396, 1376 | M | stop |

At every stop T, M, E and L were searched exhaustively and were empty, and R was empty through two retained letters,
but its beam was truncated at three letters for every target (8 to 21 of them). So each stop is a stop of the declared
search, not a negative certificate for family R.

**The beam's stops.** R's beam keeps, per target, the 64 two-letter words with the least (straddlers, F − max D, word)
and extends only those. At each stop, family R was searched again to three retained letters without a beam (the stop
search of `pstar2.py` below, whose CPU time the table gives); every descending word it found starts with a two-letter
word that the beam had dropped:

| Stop | Φ | Best macro of the unbeamed search (target) | Φ after | Descending words | CPU s |
| --- | --- | --- | --- | --- | --- |
| 7302 | 1094 | `535:1,363:5,535:1,597:14` (597) | 688 | 228 | 545 |
| 7303 | 936 | none | | 0 | 232 |
| 7304 | 1210 | `522:1,375:6,384:1,375:14` (375) | 1190 | 1,056 | 722 |
| 7305 | 1572 | none | | 0 | 502 |
| 7306 | 1698 | `16:1,109:4,16:3,139:14` (139) | 1490 | 300 | 375 |
| 7307 | 1376 | `102:8,67:9,10:1,102:12` (102) | 1252 | 696 | 772 |
| 7304 after R05 (DIS-1) | 1472 | `181:6,295:6,347:7,229:14` (229) | 1454 | 96 | 820 |
| 7304 after R06 (DIS-1) | 1472 | `181:6,295:6,347:7,229:14` (229) | 1454 | 96 | 865 |

At the 7304 stop the search also finds `181:6,295:6,347:7,229:14` (Φ 1210 → 1192): a three-letter release at 229,
where the two-letter family at 295 and 347 has none (DIS-4). After R05 or R06 it finds only descents of 18 (at 229, and
`522:9,397:5,494:4,522:12` at 522) and none at 162: R04's 162 route is gone there.

**Two-level P\*.** `pstar2.py` runs P* in segments. When a segment stops, it searches family R again at the same depth
without the beam, and if that finds a macro, the next segment starts with the best one as its forced first macro,
certified again by `macro.py` before it is committed. It is a state-only procedure with the progress measure of
Proposition P, and its stop is a negative certificate for the five families as declared, with R searched to three
retained letters without a beam (restricted to the families, Proposition BN; not for all words). Its stop search
`family_r_full` gives every target the same best candidate as `family_r` without a beam (the two reductions and why
each is exact are in its docstring). Against the unbeamed `pstar.py step` it gave the same descending words and the same
best candidate per target at three configurations at two letters (7304 chain end: 12 words; 7302 stop: none; 7305 chain
end: 2,304 words), and the same candidates, choice and number of descending words at two stops at three letters: 7307
(696 words; 200,845 nodes and 772 CPU seconds against 220,766 nodes and 1,703 seconds) and 7304 (1,056 words; 158,774
nodes and 722 seconds against 172,018 and 1,419; computed, `results/runs/s730N-stop-pruned.json` and
`s730N-stop-reference.json`). Sol reviewed the driver (calls
`20261024T220941Z-36e6c67a`, `20261024T224727Z-b27849a2`; each blocking finding fixed and verified in
`20261024T221457Z-14facd76` and `20261024T225122Z-5d7bfe2c`).

| End | Φ at the declared stop | Φ after each macro of the two-level run | Outcome | CPU s |
| --- | --- | --- | --- | --- |
| 7302 | 1094 | 688, 662, 498 | stop | 3,022 |
| 7303 | 936 | (none) | stop | 333 |
| 7304 | 1210 | 1190, 1172 | stop | 1,326 |
| 7305 | 1572 | (none) | stop | 3,838 |
| 7306 | 1698 | 1490 | stop | 2,369 |
| 7307 | 1376 | 1252, 1250, 1220 | stop | 1,868 |

The two-level runs committed `535:1,363:5,535:1,597:14` (597; F 2 against D 205, so Φ falls by 406),
`274:6,482:1,357:2,142:14` (142) and the T3P round `262:15` at 7302; `522:1,375:6,384:1,375:14` (375) and
`181:6,295:6,347:7,229:14` (229) at 7304; `16:1,109:4,16:3,139:14` (139; F 0 against D 104) at 7306; and
`102:8,67:9,10:1,102:12` (102), `102:1,94:1,102:15` (102) and `67:6,88:11,266:3,226:14` (226) at 7307. The first
macro of each segment is the choice of the stop search before it. At 7305 and 7306 the first segment is the declared
run continued from where the integrator stopped it (its macros are in the table of declared runs), and the CPU time
includes it.

**The stops of the two-level procedure.** It stops for good at all six chain ends:

- 7302, Φ 498 (digest `661010e5…`), after the chain word, the three declared macros and the three two-level macros:
  38 clean grips and 11 high units with 11 own holders. Budget: 12 (F 121, D 6), 171 (F 105, D 21) and 597 (F 205,
  D 2); no feasible items: 430; a straddler without a clean holder: the other seven.
- 7303, Φ 936 (digest `61d4b85e…`), after the chain word and the seven declared macros: 29 clean grips and 9 high
  units with 8 own holders. In the T3P scan at the stop, 282 has status budget (its best cover, the helper (364, 5),
  leaves F 198 against D 87), 262 no feasible items, and 11, 36, 234, 238, 328 and 464 a straddler without a clean
  holder.
- 7304, Φ 1172 (digest `e608db08…`), after the chain word, the four declared macros and the two two-level macros: 40
  clean grips and 11 high units with 8 own holders. Budget: 162 (F 111, D 12), 522 (F 22, D 9) and 540 (F 111, D 58);
  a straddler without a clean holder: the other five.
- 7305, Φ 1572 (digest `c0070b05…`), after the chain word and the eleven macros of the declared run: 29 clean grips
  and 18 high units with 17 own holders. Budget: 44 (F 248, D 22), 226 (F 289, D 3), 368 (F 560, D 65), 374 (F 236,
  D 15), 421 (F 255, D 45) and 436 (F 25, D 14); no feasible items: 389; a straddler without a clean holder: the
  other ten.
- 7306, Φ 1490 (digest `1a9bcbb5…`), after the chain word, the seven macros of the declared run and the two-level
  macro: 28 clean grips and 28 high units with 19 own holders. Budget: 90 (F 25, D 6), 208 (F 29, D 22), 240 (F 524,
  D 39), 252 (F 276, D 6) and 351 (F 636, D 4); no disjoint cover: 158; a straddler without a clean holder: the other
  thirteen.
- 7307, Φ 1220 (digest `60ac608e…`), after the chain word, the declared macro and the three two-level macros: 38 clean
  grips and 14 high units with 15 own holders. Budget: 10 (F 216, D 41), 102 (F 69, D 68), 319 (F 111, D 46) and 516
  (F 395, D 7); no feasible items: 597; a straddler without a clean holder: the other ten.

At all six, no single letter lowers Φ (T3P's scan of every jumble at every clean grip), family L lists no clean pair
whose caps share a high piece, and family E with E₀ = 200 is empty: at four of them with the same covers and admitted
items as with E₀ = 20, at 7305 and 7306 with the same covers and 78 and 67 admitted helper items instead of 34 and 23
(computed: the stop records in `results/runs/s730N-pstar2.json` and `results/runs/jam-730N-E200.json`).

**Certificates.** The whole descent of each end, the declared run's macros followed by the two-level run's (4 to 11
macros, 42 in all), was certified from its chain end with `macro.py cert` (excursion allowance 20) and checked with
`macro.py check --replay` (computed, `results/runs/s730N-descent.json` and `-check.json`). All six checks passed, and
each ends at the digest of its stop above.

### 2.6 Corrections

Applied to [card 7](../../theory/cards/card-7-lowering-machine.md), which now reads as attacked with these corrections:

- **Finite levels (both).** The card read K1's finite levels as evidence that release is a question of cost. Every grip
  of every reachable configuration has a finite level (Proposition FL), so finiteness decides nothing, and L4′'s case
  λ = ∞ never occurs at a reachable configuration; the values remain lower bounds.
- **Statuses (Claude Opus; adopted by Astra).** K2's statuses are statuses of the T3P scan's family
  (`14-blocking-structure/scripts/t3p.py`, its declared family), not what stops the holders: 15 of the 131 descend by
  short words (section 2.2), four of them at level 2 with a straddler without a clean holder.
- **Self-lock (Claude Opus; adopted).** After (522, 14) the lowered pieces lie in no clean cap other than 522; this binds
  helpers at other grips only. A retained letter at 522 re-aims them: the chain end followed by (522, 8) is W5's
  configuration exactly, and R05 and R06 continue from there.
- **Enablers (Claude Opus; adopted).** A letter that frees a helper need not be a partner of the target: 277 is not a
  partner of 162 (computed with `locality.py`), and 277:1 enables 375:1 in R04.
- **Neutral letters (Astra; adopted).** Question 3.1's Φ-neutral alphabet is the retained letters at clean grips and the
  jumbles at clean grips with F = D. A jumble at a pure grip has F = 0 and lowers Φ when D > 0; an impure grip can have
  neutral jumbles.
- **K4's state (Claude Opus; adopted).** Stage 14's enumeration at 229 was made at the D2 end (`ac127dc3…`), not at the
  chain end; section 2.3 gives the chain-end result.
- **Two-sided return and L9 (both).** Question 2.1 asked for a return that brings both caps' other pieces back; L9 is
  the one-sided case (it returns x's other pieces only), and when the product at the grip that acted last on G is not
  retained, that grip's return raises G unless G has first left its cap or been landed by the other grip (Proposition
  RP).

The analysts' own sealed claims were corrected in the cross-examination as well: a conflict between the 162 word and
(522, 8) that holds only until the round (375, 15), and a claim that stage 14's minimum at 229 carries over to the chain
end, which the completed enumeration of section 2.3 now settles (both Claude Opus).

### 2.7 The disputed points and how they were decided

After the cross-examination five points were open; exact experiments decided them as follows.

| Point | Question | Experiment | Outcome |
| --- | --- | --- | --- |
| DIS-1 | At 7304, does the larger 162 descent (R04's first macro), the retained-helper 522 route (R05) or the jumble-helper 522 route (R06) continue better? | P* with the same families and choice rule from each of the three, committed first (`pstar.py run --first`) | The larger 162 descent: R04's first macro takes Φ to 1276, then the declared P* goes to 1228 and 1202 and stops; after R05 or R06 it stops at once at 1472, where the search without a beam finds only descents of 18 and none at 162 (section 2.5). P* without a forced first macro takes a smaller 162 descent (to 1300) and stops at 1210 |
| DIS-2 | Does a complete short pump exist at a chain end? | The clean-pair census at the 7304 chain end, after 277:1, 375:1, 384:7 there, and at the 7307 chain end | No: no high piece lies in two clean caps at any of the three, so no pump of Proposition RP has an instance |
| DIS-3 | Do partial lenses (LC2 with a mixed or partial intersection) exist where whole units have no clean co-holder? | The same census; family L tests both lens forms over all items at every pair it lists | No pair to test at the three configurations: LC2 needs a high piece in the intersection for a descent |
| DIS-4 | Is 25 the least F − D at 229 at the 7304 chain end? | `t3p.py pairs 229 295 347` at the chain end, all 7,200 words | Yes, for the two-letter family (section 2.3) |
| DIS-5 | Does P* reach Φ = 0 from the six chain ends, and if not, where does it stop? | P* from each chain end, every committed certificate replayed independently | No. The declared P* stops at all six ends with R's beam truncated for every target, and the search without a beam finds descents at four of these stops. Two-level P* then stops for good at all six: 7302 at Φ 498, 7303 at 936, 7304 at 1172, 7305 at 1572, 7306 at 1490 and 7307 at 1220 (36% to 87% of the chain-end Φ), where no macro of the five families and no single letter lowers Φ. Every whole descent is certified from its chain end and replayed independently (section 2.5) |

### 2.8 Answers to the card's questions

- **1.1 Multi-level unlocking: answered in part.** Proved: the retained release (Corollary SRR), retained letters that
  are each admissible when they act, after which x is clean, then a lowering with F < D, is a tame macro whose
  hypotheses are table tests at X (Lemma RS, Corollary CR) except D, which needs the exact second vertices. It has at
  least lev(x) + 1 letters (L4) and, with distinct grips, at most 601; there is no upper bound in ℓ alone. Words with
  jumble helpers are certified event by event (Theorem MC), and helpers that raise Φ give T3P-E, with excursion at most
  the sum of their rises. The conjugated form u·(x, h)·u⁻¹ is Lemma CJ, with no instance found. Computed: the four
  own holders of level 2 that descend at the chain ends do so by retained releases of two or three letters (R02, R04,
  R09, R13), R04 through an enabler that is not a partner of its target. Open: hypotheses under which a release exists
  for ℓ ≥ 2, and a length bound in ℓ.
- **1.2 Release dependency structure: answered in part.** Proved: the structure is an AND/OR graph over the straddler
  cells of x whose min–max value is lev(x), and every word that cleans x contains a schedule of it (Proposition AO);
  every grip of a reachable configuration has a finite level, so finiteness decides nothing (Proposition FL).
  Reported: at grip 2 of 7304 the graph is the chain 23 → 41 → 2, and no item at 23 cleans 41, because its image puts
  other pieces across 41; what fails is a leaf's obligation, not a cycle. Open: a startable part in general.
- **1.3 Overlapping helpers: answered.** Proved: Theorem MC certifies any admissible word in its executed order,
  overlapping supports, repeated grips and letters at the target included, with table tests at X and an exact image
  only after a jumble at a grip of another class; the occurrence DAG gives order independence where it is needed.
  Computed: at the 7304 chain end no tame two-letter word at 295 and 347 lets a jumble at 229 lower Φ (least F − D 25,
  DIS-4); at the stop of the declared run there, the three-letter word 181:6, 295:6, 347:7 does (Φ 1210 → 1192 with
  (229, 14)).
- **2.1 Reverse pump: answered; no instance where tested.** Proved: the Good-set controller (Theorem GC; excursion at
  most 2(f_x + f_z) whatever the length), the lens cells LC1 and LC2 (four letters that return every other piece), the
  pump and the limit on its return (Proposition RP), and whole pumps as a finite viability computation (Proposition
  V). Computed: no piece of height at least 4 lies in two clean caps at the 7304 and 7307 chain ends or after R04's
  preparation (DIS-2, DIS-3). Along the runs, family L found clean pairs whose caps share a high piece at ten steps
  (grips 9 and 59 and grips 140 and 364 at 7303, 90 and 240 at 7306, 262 and 482 at 7302): each pair is of one
  class and shares 193 to 309 pieces in 6 to 16 pose groups, so GC and LC1 do not apply, and no item pair passed the
  tests of LC1 or LC2 there.
- **2.2 Relays: answered in part.** Proved: words confined to one clean cap act as one of its 60 items (Proposition
  1C); a retained letter at the target re-aims the lowered set, in twelve placements read from one table (Lemma RA);
  L8 and L9 of stage 14, with the limit on L9's return in Proposition RP. Computed: 8 of the 32 budget own holders
  descend by short words (R01, R05 to R08, R10, R11, R14, R17), and all but R08 start with a re-aim at the target; a
  cap that holds only what it lowered is re-aimed and then unloaded by a helper (R05 and R06 at 522 of 7304). Family M
  (a re-aim, one tame helper letter and a lowering) was searched exhaustively at every step of P* and chosen twice, as
  the first macro at 7302 and at 7307. No word confined to 522 or 540 of 7304 or to 516 of 7307 lowers Φ. Open: relay
  chains with bounds; at the six stops of section 2.5, the budget own holders have no relay in the families searched.
- **2.3 Where the lowered pieces go: answered as a finite test.** Proved: the twelve lowering items of one coset place
  the lowered set in twelve ways, each read from the table at the lowering (Lemma RA, Corollary CR), and whether a
  later letter can unload or land them is a table test (stage 14, S1 and S2). Computed: at 522 of 7304, item 14 leaves
  the 70 lowered pieces in no clean cap but 522, and item 49 = 8·14 puts them in cap 451, whose letters (451, 8) and
  (451, 44) unload them (R05, R06). Open: a rule that guarantees a good item; P*'s choice rule does not look ahead.
- **3.1 Φ-neutral letters: answered for retained letters.** Proved: a retained word acts on the label table by row
  permutations (Lemma RS) read by columns (Corollary CR; Lemma CL for letters of the grip's class); one retained letter
  changes |S_d| by M_c(k⁻¹d) − M_c(d) (Proposition SL); retained letters keep straddle numbers, holder numbers (stage
  14, L1) and the number of own-class grips at which each high piece is in (Corollary OH). Neutral jumbles do not keep
  straddle numbers (Proposition NJ). Open: which pairs of grips holding G whole the neutral letters reach.
- **3.2 Placement theorem: open.** Proved obstruction: if no piece of height at least 4 is in at a grip of its own
  branch's class, every descent first raises Φ (Corollary OH); it certifies nothing at the chain ends, where every
  high piece has one or two such grips. The descents found are placements by retained letters, found by search; no
  placement for a lens cell or a pump exists in the families searched.
- **4. An order that does not jam: answered negatively for the families of P\*.** Proved: P* ends after at most Φ/2
  certified macros, with Φ never more than E₀ above its start (Proposition P); a negative certificate with excursion
  E must be closed under every letter that stays below the ceiling (Proposition BN). Computed: P* and two-level P*
  lower Φ at all six chain ends, and two-level P* stops for good at all six: 7302 (Φ 498), 7303 (936), 7304 (1172),
  7305 (1572), 7306 (1490) and 7307 (1220). At these six reachable configurations, given exactly by their tables and
  words, no macro of the five families exists (R to three retained letters without a beam; E also with E₀ = 200) and
  no single letter lowers Φ, so no procedure whose steps are macros of these families reaches depth one from them,
  and a class on which such a procedure provably works must exclude them. Open: whether words outside the families
  descend there (longer releases, jumble helpers, larger excursions), and a negative certificate in the sense of
  Proposition BN for a stated alphabet.

## 3. Tools (`scripts/`)

Astra wrote the tools from the merged statements of the cross-examination (implementation call
`20261024T195417Z-064452c4`); the integrator reviewed them line by line before applying them. They import the stage 14
scripts and change none of them.

| Script | What it computes |
| --- | --- |
| `macro.py` | Theorem MC as a checker. `cert` certifies one or more macros after a prefix: for every letter, admissibility on the exact poses (the acting grip clean, the moved set its whole inside set), the retained calculus of Lemma RS against stage 14's exact row update, the change of Φ, the heights and branches, the excursion against the declared allowance, and for the last letter of each macro a jumble with its F and D and δ = 2(F − D); the descent of each macro and the telescoped Φ. `check` re-verifies a certificate (format, source table digest, start, every claim) and with `--replay` replays the whole word independently, recomputing every height from scratch after each letter. |
| `pstar.py` | The procedure P* (section 2.4). `step` searches the families at one configuration and writes the stop or choice record with the certificate of the chosen macro; `run` repeats steps until Φ = 0, a stop, `--max-steps` or `--cpu-limit` (CPU seconds over all resumptions), checkpointing the label table, the certificate and the run record after each committed macro; `--resume` continues a run after replaying its certificate independently; `--first` commits a given macro before the procedure starts. `census` lists the clean pairs whose caps share a high piece, with their pose groups and the lens tests of family L. |
| `test_macro.py` | The gate (seven parts, 324 assertions, about 29 CPU minutes, peak memory 0.8 GB): the retained calculus and column reading at three clean grips of 7304 against stage 14's exact application; certificates and independent replays of R04, R05, R06, R16 and R17; seven tampered certificates, each rejected by the named test; a T, R, M step at 7304; an E step at 7306 with E₀ = 20 and 0; a run with checkpoint, resume, a CPU limit and a forced first macro; the 7304 census. |
| `pstar2.py` | Two-level P* (section 2.5), written by the integrator: segments of `pstar.py run` with the declared options; at each stop, family R searched again at the same depth without a beam, and the best candidate starts the next segment as its certified first macro. The stop search `family_r_full` walks the tree of `pstar.family_r` depth first, skips a letter at the grip of the letter before it (the shorter composed word is in the search with a smaller key) and tries a last letter at c only when every straddler of the target lies inside c and the foreign load outside c is below the largest D outside c plus the high pieces inside c; it gives every target the same best candidate as `family_r` with an unbounded beam (`--stop-search beam` runs that instead). `--stop-only` runs one stop search. |

All four write only outside the repository. Label tables are cached as in stage 14 (`.npz`, keyed by the state's
digest). Commands are in section 5.

## 4. Reading

The precedence peel stopped at the chain ends because of its family, not because of locks: short words with
overlapping retained helpers, enablers that are not partners of the target, re-aims and helpers that raise Φ by 20
release 15 of its 131 stopped own holders (section 2.2), and a certified greedy procedure over five such families
continues the descent at all six chain ends (section 2.5). The mechanism that works is the retained release: retained
letters keep Φ and act on one label table by row permutations, so a release is decided exactly before anything moves,
and one jumble then lowers. The lowering machine of Question 2.1, a pump on two clean caps that share a high unit, is
proved, but it has no instance where it was tested: no high piece lies in two clean caps there.

The procedure is not enough. Its declared search stopped at all six ends, at four of them early through its beam;
with that repaired it continues there, and then stops for good at all six ends, at 36% to 87% of their chain-end Φ.
There no macro of its families exists and no single letter descends; most own holders have a straddler without a
clean holder, and the others have no feasible helper items, no disjoint cover or a foreign load above their lowering
supply (102 of 7307 is short by one). The open part of card 7 is now concrete: at these six configurations, find a
descent outside the families (longer retained releases, jumble helpers, larger excursions), or prove a negative
certificate with excursion (Proposition BN) for a stated alphabet. Nothing here proves that P* or any order reaches
depth one, and these headless runs support no Windows, DirectX, input or performance claim.

## 5. Reproduction

From the repository root, with one thread per process (`OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1`),
`S=research/jumbling/restoration/15-lowering-machine/scripts`,
`S14=research/jumbling/restoration/14-blocking-structure/scripts`, `T` the D2 end table of a chain end and `W` its chain
word (section 1), `M` a comma-separated macro word, and `D` a directory outside the repository:

    python3 -I -B $S/test_macro.py
    python3 -I -B $S/macro.py cert $T $D/RNN.json --prefix $W --labels $D/end.npz --excursion 0 --macro $M
    python3 -I -B $S/macro.py check $T $D/RNN.json $D/RNN-check.json --labels $D/end.npz --replay
    python3 -I -B $S/pstar.py census $T $D/census.json --prefix $W --labels $D/end.npz
    python3 -I -B $S14/t3p.py pairs $T 229 295 347 $D/pairs-229-a.json --word $W --labels $D/end.npz --items-y $(seq -s, 0 29)
    python3 -I -B $S14/t3p.py pairs $T 229 295 347 $D/pairs-229-b.json --word $W --labels $D/end.npz --items-y $(seq -s, 30 59)
    python3 -I -B $S/pstar.py run $T $D/run --prefix $W --labels $D/end.npz --families T,R,M,E,L --e0 20 --depth 3 --beam 64 --choice best --max-steps 2000 --cpu-limit 3600
    python3 -I -B $S/pstar.py step $T $D/stop.json --prefix $W,<the run's macros> --labels $D/run/labels.npz --families R --depth 3 --beam 1000000
    python3 -I -B $S/pstar2.py $T $D/stop-pruned --prefix $W,<the run's macros> --labels $D/run/labels.npz --stop-only
    python3 -I -B $S/pstar2.py $T $D/two-level --prefix $W,<the run's macros> --labels $D/run/labels.npz [--first $M] --cpu-limit 5400
    python3 -I -B $S/pstar.py step $T $D/jam-E200.json --prefix <the stop's word> --labels <its labels> --families E --e0 200
    python3 -I -B $S/macro.py cert $T $D/descent.json --prefix $W --labels $D/end.npz --excursion 20 --macro <each committed macro, in order>

A row of two macros (R04, R15) passes `--macro` twice; R03, R15 and R16 use `--excursion 20`. The DIS-1 runs are the
`pstar.py run` line at 7304 with `--first` set to R04's first macro, R05 or R06. The two-level runs start at their
declared run's end (at 7305 and 7306, where the integrator stopped it) with the CPU limit 5400 (14,400 at 7302); at
7304 and 7307 they start with `--first` set to the choice of the stop search there.
The census at 7304 after the preparation of R04 adds `277:1,375:1,384:7` to the prefix. The two `pairs` commands
split the first-letter items at 295 between two processes; the files in `results/` keep only the rows of the 690
per-letter-tame words of each half (the full outputs are about 1 MB each and took 52 and 55 CPU minutes). A label
cache whose digest differs from the state is rebuilt, so each state gets its own cache.
