# Card 8: the six jams of two-level P*, for the menu I_a

Status: **answered in part** by the joint attack of 25 October 2026 ([stage 16, section 2](../../restoration/16-six-jams/README.md#2-joint-attack-25-october-2026)):
every jam has a certified tame descent, so the least excursion is 0 at all six (Question 3); Questions 1.2, 1.3 and 2
are answered by proved transport, routing and accounting results with certified descents, Question 1.1 for four of the
six holders (307 and 517 stay open), and Question 4 in part: with transports at its stops P* continues at all six
jams and stops again, and where it stops depends on the route. Section 2.6 there lists the corrections to the facts
below. With routing releases also in its stop search (P***, [stage 17](../../restoration/17-routing-in-driver/README.md),
26 October 2026), the procedure continues at eight of the nine stops of P** and stops again at Φ 386 to 1158; at every
stop where it ends it releases the blocked grips in many configurations, and no tail of its forms descends after any
release.

The open part of [card 7](card-7-lowering-machine.md), Question 4, which the owner put before the tail completion and
the bulk on 25 October 2026
([`owner-decisions-2026-10-25-jams-first`](../../../../docs/wiki/decisions/owner-decisions-2026-10-25-jams-first.md)).
Two-level P*, a certified greedy procedure over five families of macros, lowers the depth Φ at the six chain ends of the
D2 height phase by 13% to 64% and then stops for good at six reachable configurations, the **jams**, where no macro of
its families and no single letter lowers Φ
([stage 15, section 2.5](../../restoration/15-lowering-machine/README.md#25-runs)). Answer Questions 1 to 4. Each
answer may be a proof, a reduction to finite certificates that a computer can check at the state, or a precise
counterexample or counterexample shape. Partial results are useful. Label every statement proved, conditional (name the
condition) or conjectured. Name every machine check as a certificate with its exact statement, and do not claim that a
certificate holds.

The target is unchanged: a configuration with every left height at most 2 (Φ = 0, depth one), reached from a jam. The
standard is **tame** where that suffices; a **bounded excursion** is admitted: a word along which Φ ≤ Φ(start) + E and
which ends with Φ lower, E stated in the data of the hypotheses.

## Model

As in [card 7](card-7-lowering-machine.md) and [card 6](card-6-blocking-structure.md), whose model, terms and notation
this card uses: the puzzle K⁺, the 600 grips in five classes, caps, the menu Λ_c (60 items: 0 the identity, 1 to 11
retained, 12 to 59 jumbles; a group of order 60 whose items 0 to 11 form A4_c), heights, branches and second vertices,
the labels in, out and straddle, S_d and I_d, clean and blocked grips, holders, Φ = Σ max(0, h − 2), the foreign load
F_x and the lowering count D_x(h) (ΔΦ = 2(F_x − D_x(h)) for a jumble at a clean grip), units, high units, own holders,
partners, levels and tame words. From stage 15 ([README section 1](../../restoration/15-lowering-machine/README.md#1-terms),
[`PROOFS.md`](../../restoration/15-lowering-machine/PROOFS.md)): words, traces, excursion, macros, supports, re-aims,
enablers, the release structure of a target (an AND/OR graph over its straddler **cells**, the straddlers with
identical holder rows; each leaf carries **obligations**: an item whose image leaves no moved piece across the grips
still to act, and the cost of that image), and the five families of P* (README section 2.4): T (T3P rounds), R
(retained letters at grips that matter for the target, then a lowering at the target), M (a re-aim, one tame helper
letter, a lowering), E (T3P with helpers that raise Φ by at most E₀ in total), L (lens cells at clean pairs whose caps
share a high piece).

New here:
- A **positive-budget blocked holder** is an own holder x that is blocked and whose current inside set has
  max_h D_x(h) > F_x. These numbers are those of I_x at the jam; once straddlers of x move, they can end in or out of
  cap x and change F_x and D_x.
- A **load jam** is a jam with no positive-budget own holder, clean or blocked: every own holder x has F_x ≥ D_x(h) for
  every jumble h.

## Given (proved; use as facts)

From the joint attack on card 7 ([stage 15, `PROOFS.md`](../../restoration/15-lowering-machine/PROOFS.md)):
- **Lemma RS, Corollary CR, Lemma CL, Proposition SL.** A word of retained letters is decided on the label table at the
  start by row permutations (admissibility, moved sets, labels, heights, branches; S and F after one letter read in the
  column k⁻¹x); D needs the exact second vertices. Every item at a grip permutes the 120 poles of its class, so after a
  jumble the labels of the moved pieces at grips of that class are a column permutation; other classes need an exact
  image. A retained letter (c, k) changes |S_d| to |S_d| − M_c(d) + M_c(k⁻¹d).
- **Theorem MC.** Any admissible word is certified event by event on the exact poses (acting grip clean, support its
  whole inside set, δ_t telescoping), which gives the descent, the exact excursion and tameness. `macro.py cert` and
  `macro.py check --replay` implement it.
- **Corollaries SRR and T3PE, Lemmas RA and CJ.** Retained release: retained letters, each admissible when it acts, then
  a lowering with F < D is a tame macro; m ≥ lev(x) when x is blocked. T3P with an excursion: helpers may raise Φ when
  the round's net change is negative; the excursion depends on the executed order. Re-aim: a retained letter at the
  target places the lowered set in one of twelve ways, read from one table. Conjugated lowering u·(x, h)·u⁻¹: valid when
  every piece of I_x is out at every grip of u after the lowering; no instance was found.
- **Propositions FL and AO.** Every grip has a finite level at every reachable configuration (reverse the history), so
  finiteness decides nothing. The release structure is an AND/OR graph whose min–max value is the level; every word
  that cleans the target contains a schedule of it; its leaves carry obligations that no theorem guarantees.
- **Propositions 1C, BN, NJ and Corollary OH.** Words confined to one clean cap act as one of its 60 items. A negative
  certificate with excursion E at X is a finite set of exact configurations, containing X, with Φ(X) ≤ Φ ≤ Φ(X) + E,
  closed under every admissible letter whose result has Φ ≤ Φ(X) + E; it proves that no word with excursion at most E
  lowers Φ, and conversely one exists when no such word does (with a finite but astronomical bound on its size). A
  certificate restricted to the letters of a declared family proves only that the family has no macro. If no piece of
  height at least 4 is in at a grip of its own branch's class (N_own = 0 for every high piece), every descent first
  raises Φ. Φ-neutral jumbles do not keep straddle numbers.
- **Theorems GC, Lemmas LC1 and LC2, Propositions RP and V.** The lowering machine on two clean caps whose inside sets
  meet exactly in a high unit, with excursion at most 2(f_x + f_z) whatever the length; four-letter lens cells; the
  reverse pump and the limit on its return; whole pumps as a finite viability computation.
- **Proposition P.** P* commits only certified macros that lower Φ with excursion at most E₀, so it ends after at most
  Φ/2 macros; a stop records which families were searched exhaustively.

From the joint attack on card 6 ([stage 14, `PROOFS.md`](../../restoration/14-blocking-structure/PROOFS.md)), as listed
in card 7: Theorem T3P with Lemma A and Corollaries B and I; Lemma R with Corollaries U and E (Ψ_x = Φ + 2F_x is
unchanged by every letter at x; the descent made by letters at c is 2(F_c(end) − F_c(start)) + 2U_c); Lemma P (for a
class P, Φ + 2J_P is unchanged by letters at grips of class P, J_P the off-lattice pieces of branch other than P); Lemmas
L4, L4′, L5, L6 and Proposition N; safety S1 and S2; the bounded-excursion shapes L8 (lower, then relower) and L9
(lower, land, return); Proposition P12 and Conjecture C13. From card 5: T5 and Lemmas 5.2 and 5.3 (a jumble of class Q
lowers by 2 exactly the pieces of branch Q whose second vertex is h_L⁻¹v₀, sending them into the other four branches,
keeps the height of the other pieces of branch Q and raises every other piece by 2, into branch Q; it lands a height-2
piece of branch Q with that second vertex on the lattice and raises a lattice piece to height 2; retained twists change
no height; a word of one class changes a height by at most 2 in total).

## The jams (exact)

Each jam is the D2 end table (in `research/jumbling/restoration/13-tutorial-runner/runs/dev/`) followed by its word:
the chain word of stage 14, then the macros that the declared run and the two-level run of P* committed (stage 15,
section 2.5; the same macros are the `events` of `15-lowering-machine/results/runs/s730N-descent.json`, whose `prefix`
is the chain word). Every letter is `grip:item`.

| Jam | D2 end table | Digest | Φ | Height 4 (6) | Height 2 | High units | Own holders | Clean grips |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 7302 | `d2-s7302/component-checkpoint/tables/00021.json` | `661010e5…` | 498 | 249 | 106,293 | 11 | 11 | 38 |
| 7303 | `d2-s7303/run2/d2-s7303/tables/00138.json` | `61d4b85e…` | 936 | 468 | 103,728 | 9 | 8 | 29 |
| 7304 | `d2-s7304/component/tables/00005.json` | `e608db08…` | 1172 | 586 | 96,720 | 11 | 8 | 40 |
| 7305 | `d2-s7305/component/tables/00010.json` | `c0070b05…` | 1572 | 786 | 109,364 | 18 | 17 | 29 |
| 7306 | `d2-s7306/component-checkpoint/tables/00017.json` | `1a9bcbb5…` | 1490 | 745 | 108,786 | 28 | 19 | 28 |
| 7307 | `d2-s7307/component/tables/00005.json` | `60ac608e…` | 1220 | 596 (7) | 100,049 | 14 | 15 | 38 |

Words (chain word, then the macros, separated by `|`):

    7302: 277:59,171:15,39:1,171:14 | 104:8,46:12,104:15 | 20:1,104:4,46:14 | 39:1,46:3,20:7,12:12 |
          535:1,363:5,535:1,597:14 | 274:6,482:1,357:2,142:14 | 262:15
    7303: 364:5,282:13,236:12,24:7,69:14 | 282:5,364:6,95:4,140:13 | 69:50,24:13 | 69:12 | 140:8,364:6,140:14 |
          272:9,245:7,365:14 | 365:1,508:8,365:15 | 69:4,71:12
    7304: 344:9,451:55,522:14 | 277:1,384:7,375:1,162:13 | 162:6,384:1,375:15 | 375:5,384:2,375:13 |
          522:8,451:8,522:15 | 522:1,375:6,384:1,375:14 | 181:6,295:6,347:7,229:14
    7305: 421:4,577:1,520:14,233:9,281:12,6:45,44:15,208:13,168:15 | 499:1,421:6,574:1,499:15 |
          134:1,47:1,44:5,177:13 | 177:2,374:1,177:15 | 113:2,122:2,67:7,113:13 | 421:5,520:10,436:14 | 436:8,468:15 |
          10:6,102:2,10:14 | 281:5,94:2,168:13 | 67:1,113:14 | 468:6,436:7,468:14 | 113:1,67:2,113:12
    7306: 208:5,336:15,445:55,252:13,506:7,561:14 | 582:3,487:6,240:14 | 561:6,598:1,561:15 |
          233:3,373:5,233:5,173:13 | 561:4,598:2,561:12 | 90:6,240:1,90:15 | 90:9,240:1,90:12 | 200:7,202:12 |
          16:1,109:4,16:3,139:14
    7307: 516:14,597:10,543:13,10:8,147:2,102:15,216:2,266:14,275:1,435:12,435:7,319:12,94:12 |
          474:7,394:22,474:14 | 102:8,67:9,10:1,102:12 | 102:1,94:1,102:15 | 67:6,88:11,266:3,226:14

## Computed facts (exact; leads, not proofs)

Programs: stage 15 (`pstar.py`, `pstar2.py`, `macro.py`) and stage 14 (`t3p.py`, `levels.py`); the integrator's
tables below were computed with these on the exact poses (label tables, levels, budgets, exact images).

- **J1 (what was searched at the jams).** At every jam: the T3P scan (family T) has no round, families M and L are
  exhausted and empty (L lists no clean pair whose caps share a high piece), family E is empty with E₀ = 20 and with
  E₀ = 200, family R is empty through three retained letters at its declared helper sets without a beam (the pruned
  exact search of `pstar2.py`), and no single letter lowers Φ (every jumble at every clean grip has F ≥ D). So a
  descent from a jam, if there is one, lies outside these families as declared: for example a retained release of four
  or more letters or with letters outside R's helper sets, jumble helpers in shapes other than those of M and E, or an
  excursion above E's budget.
- **J2 (statuses at the jams, from the T3P scan of the stop records).**

  | Jam | Budget (F, D) | No feasible items | No disjoint cover | Straddler without a clean holder |
  | --- | --- | --- | --- | --- |
  | 7302 | 12 (121, 6), 171 (105, 21), 597 (205, 2) | 430 | | 18, 65, 94, 181, 271, 340, 517 |
  | 7303 | 282 (198, 87 after its best cover) | 262 | | 11, 36, 234, 238, 328, 464 |
  | 7304 | 162 (111, 12), 522 (22, 9), 540 (111, 58) | | | 2, 64, 251, 336, 596 |
  | 7305 | 44 (248, 22), 226 (289, 3), 368 (560, 65), 374 (236, 15), 421 (255, 45), 436 (25, 14) | 389 | | ten others |
  | 7306 | 90 (25, 6), 208 (29, 22), 240 (524, 39), 252 (276, 6), 351 (636, 4) | | 158 | thirteen others |
  | 7307 | 10 (216, 41), 102 (69, 68), 319 (111, 46), 516 (395, 7) | 597 | | ten others |

  The (F, D) of a budget holder are those of its best cover in the scan; 102 of 7307 is short by one (a lowering there
  changes Φ by +2).
- **J3 (levels).** The largest level is 6 at 7302, 7303, 7304, 7305 and 7307 and 7 at 7306; λ = 0 at every jam. Levels
  of the own holders range from 0 to 7 (`levels.py`).
- **J4 (positive-budget blocked holders).** Four jams have them; 7302 and 7303 are load jams. For each, the level, the
  straddler count S, F and max D of the current inside set, and its straddler cells (size; whether some clean grip holds
  the cell; the holders of least level):

  | Jam | x | Level | S | F | max D | Straddler cells |
  | --- | --- | --- | --- | --- | --- | --- |
  | 7304 | 2 | 2 | 101 | 168 | 309 | ten cells (22, 18, 18, 16, 12, 10, 2, 1, 1, 1), none with a clean holder; every one has the level-1 holder 41 |
  | 7305 | 504 | 2 | 6 | 66 | 120 | three cells of 2, none with a clean holder; least-level holders 305 (two cells) and 389 (one), both of level 1 |
  | 7305 | 576 | 3 | 10 | 113 | 395 | cells of 7 and 1 held by the clean grip 431; a cell of 2 height-4 pieces with no clean holder, least-level holder 504 (level 2) |
  | 7306 | 307 | 4 | 15 | 1 | 109 | cells of 11 and 4, no clean holder; least-level holder 431 (level 3) |
  | 7306 | 370 | 2 | 85 | 69 | 111 | three cells (22, 12, 3) held by the clean grip 233; four (27, 13, 5, 3) with no clean holder, least-level holder 536 (level 1) |
  | 7307 | 517 | 2 | 74 | 79 | 95 | five cells (17, 10, 6, 5, 4) held by the clean grip 448; one cell of 32 height-4 pieces with no clean holder, least-level holder 597 (level 1) |

  First steps (computed by the integrator; every item at the clean holder evaluated on the exact image of its cap, the
  other straddlers unchanged). Words confined to one clean cap act as one of its items (Proposition 1C), so each line
  covers every word at that one cap:
  - 7304, x = 2: the 132 straddlers of 41 all lie in the clean cap 23 (2,825 pieces). No item at 23 cleans 41; the
    best, the retained item 1, moves all 132 off the cut and puts 9 other pieces of cap 23 across it.
  - 7305, x = 504: the 45 straddlers of 305 lie in cap 134 and the 26 of 389 in cap 421. No retained item cleans
    either; the jumbles (134, 12) and (421, 51) do, at the least cost of a cleaning item there, ΔΦ = +418 and +420.
  - 7305, x = 576: the retained item 3 at 431 moves the 8 straddlers that 431 holds off the cut and puts no new piece
    across it; the other 2 (of height 4) wait for 504.
  - 7306, x = 307: the least-level holder 431 has level 3, and no straddler cell of 431 has a clean holder.
  - 7306, x = 370: the retained item 6 at 233 moves the 37 straddlers there off the cut with no new ones; the 3
    straddlers of 536 lie in cap 598, no retained item cleans 536, and the jumble (598, 20) does, at ΔΦ = +292.
  - 7307, x = 517: the retained item 3 at 448 moves the 42 straddlers there off the cut with no new ones; the 86
    straddlers of 597 lie in cap 543, no retained item cleans 597, and the jumble (543, 12) does, at ΔΦ = +222 at the
    jam (after the relay (448, 3), retained items at 543 clean 597 at ΔΦ = 0; stage 16, section 2.6).

  So at each of the six the release needs, at some grip of its structure, either a word at several caps or a jumble
  that raises Φ by 222 to 420 (or, for 307, a deeper structure). Words at several caps clean 41, 305, 389, 536 and 597
  tamely, and lowerings at 2, 504, 576 and 370 follow; after the releases of 597 no lowering at 517 was found within
  stated bounds, and 307 has level 4 (stage 16, sections 2.2, 2.3 and 2.6).
- **J5 (load jams).** At 7302 every own holder has F ≥ 3·max D; the clean ones are 12 (F 121, max D 6), 171 (105, 21)
  and 597 (205, 2), and the blocked ones have F of 654 to 1,513 against max D of 2 to 111. At 7303 the only clean own
  holder is 282 (F 274, max D 87), and the blocked ones have F of 257 to 1,022 against max D of 2 to 198.
- **J6 (cheap letters).** Every jumble at a clean grip has F ≥ D (no descending letter). Clean grips with F = 0, each
  with 48 Φ-neutral jumbles (F = D = 0) that land height-2 pieces of their class's branch with the right second vertex and
  raise lattice pieces to height 2: 23 at 7302, 14 at 7303, 26 at 7304, 7 at 7305, 3 at 7306, 20 at 7307. The least
  positive change of Φ by one jumble at a clean grip is 10 at 7302, 4 at 7303, 2 at 7304, 6 at 7305, 2 at 7306 and 2 at
  7307.
- **J7 (own-class holders of high pieces).** Every high piece is in at one or two grips of its own branch's class
  (N_own ∈ {1, 2}) at every jam, so Corollary OH certifies nothing there.
- **J8 (heights).** Every high piece has height 4, except seven pieces of height 6 at 7307 (units of 3 and 4 pieces,
  branch 0, own holders 516 and 526). Between 96,720 and 109,364 of the 177,120 pieces have height 2: a jumble at a
  grip of class Q raises every height-2 piece of another branch in its cap, which is why the foreign loads are in the
  hundreds.

## Questions

### 1. Releases at the positive-budget blocked holders

1. For each of the six holders of J4, construct a word after which x is clean and has F_x < D_x(h) for a lowering item
   h, tame or with a stated excursion, with hypotheses that are finite exact tests at the jam and explicit bounds on the
   length and the excursion; or prove that no word of a stated kind does it (for example, no tame word of letters at the
   grips of its release structure up to a stated length). Say why the exact three-letter search of family R found
   nothing there.
2. **Obligation repair.** When the image of a letter that moves a straddler cell puts other pieces across a grip that
   must still act (as at the 7304 chain end, where no item at 23 cleans 41: [stage 15, section 2.3](../../restoration/15-lowering-machine/README.md#23-facts-at-the-chain-ends)),
   give a construction that repairs it, with a proof and a bound in the size of the release structure: letters that
   first move the pieces that the image would put across; splitting a cell's move between several holders; a jumble at
   a stated cost; a conjugation u·v·u⁻¹; or show that no repair of a stated kind exists at a stated jam.
3. A release may also change F and D at x: straddlers that end inside cap x join its foreign load or its lowering
   supply. Give the accounting (in the manner of Lemma R and Corollary U) and a condition under which a release keeps
   F < D.

### 2. Load jams

At 7302 and 7303, and at the budget holders of every jam, each own holder has F ≥ D, mostly by a factor of 3 or more.
Corollaries U and E say that descent at a cap x needs letters at other grips that lower F_x or raise D_x.

1. Give a construction that lowers F_x or raises D_x by enough, with hypotheses and bounds: **relays** that move foreign
   pieces out of cap x (letters at clean grips that hold them, without bringing others in); **landings** that turn
   foreign height-2 pieces of x into lattice pieces by Φ-neutral jumbles at pure grips of their branch's class (J6);
   **merges** that bring high pieces of one branch and one second vertex together into one own-class cap (placement,
   card 7, Question 3.2).
2. Or give an obstruction: an invariant or monovariant of a stated alphabet (in the manner of Lemma P, Lemma R,
   Corollary OH) that rules descent out at a stated jam for that alphabet, with its proof.

### 3. The least excursion

For a jam X let E*(X) be the least E such that some word from X with excursion at most E lowers Φ. Give bounds: an
upper bound by an exact word, certified as in Theorem MC; a lower bound by a proof, or by a negative certificate in the
sense of Proposition BN restricted to a stated alphabet (for example, all words of length at most L at grips of a
stated set), with its size. In particular: is there a tame descent (E*(X) = 0) at each jam? The reversed history gives a
finite upper bound for every reachable configuration; say whether a useful one follows.

### 4. An order that does not jam (continued)

Give a state-only procedure with a progress measure (Φ at the end of each macro, the excursion inside a step bounded by
a stated E) whose families include the constructions of Questions 1 to 3, prove that it terminates, and say at which of
the six jams it continues and where it stops. Or show that every procedure of a stated kind jams at a reachable
configuration, given as a pose table or an exact word from solved, with a finite negative certificate in the manner of
Proposition BN. A choice rule that avoids the jams from the chain ends counts as an answer if it is stated as a rule
with a proof or an exact run: at 7304 the choice of the first macro decided how far P* got (stage 15, DIS-1).

## Return

A proof blueprint with numbered lemmas for each question attempted. State every certificate exactly (what is computed,
on which objects, and what it must show). Give every descent as an exact word from the jam (with the jam's digest), and
every counterexample as an exact word from solved or a pose table. Keep proved, conditional and conjectured statements
apart. For each construction, name an adjudicating experiment at one or more of the six jams (the input table and word,
the procedure or the word, and the outcome that confirms or refutes it), and say which own holders of J2 and J4 it is
expected to reach.

## Attached evidence

- [`../../restoration/15-lowering-machine/`](../../restoration/15-lowering-machine/): `README.md` (sections 1 to 2: the
  terms, the results, the descents, the facts at the chain ends, P* and its runs, the stops), `PROOFS.md` (every result
  listed under Given, with proofs), `scripts/` (`macro.py`, `pstar.py`, `pstar2.py`, `test_macro.py`) and `results/`
  (`runs/s730N-pstar2.json`: the stop records of the jams, with the T3P scan rows and the family records;
  `runs/s730N-descent.json` and `-check.json`: the whole descents from the chain ends to the jams, certified and
  replayed; `runs/jam-730N-E200.json`: family E with E₀ = 200 at the jams).
- [`../../restoration/14-blocking-structure/`](../../restoration/14-blocking-structure/): `README.md`, `PROOFS.md`,
  `scripts/` (`t3p.py`, `levels.py`, `blockmap.py`, `jamcheck.py`, `locality.py`, `replay_word.py`, `chain.py`) and
  `results/`.
- The scripts run on the exact jumbling simulator of the research tree (`research/jumbling`) with
  `python3 -I -B <script> ...` from the repository root. They are exact computations in synthetic geometry, not
  native-host evidence.
