# Card 7: multi-level unlocking and the lowering machine, for the menu I_a

Status: **answered in part** by the joint attack of 24 October 2026 ([stage 15, section 2](../../restoration/15-lowering-machine/README.md#2-joint-attack-24-october-2026)):
Question 1.3 by a chronological certificate for any word (Theorem MC), Question 2.1 by a proved lowering machine that
has no instance where it was tested, and Question 2.3 as a finite test (Lemma RA); Questions 1.1, 1.2, 2.2 and 3.1
have partial answers there, and 3.2 is open. Question 4 is answered negatively for the families of the certified
procedure P*: it lowers Φ at all six chain ends and then stops for good at every one of them, at configurations
where no macro of its families and no single letter lowers Φ (section 2.5). The text below is the card as attacked, with the
corrections of the attack applied.

Problem 2 of the theorem programme (owner decision of 24 October 2026,
[`owner-decisions-2026-10-24-theorem-program`](../../../../docs/wiki/decisions/owner-decisions-2026-10-24-theorem-program.md)),
together with the part of problem 1 that card 6 left open: an order of unlocking and lowering that does not jam
([card 6](card-6-blocking-structure.md), Question 3.3).

Open problem M1 of the restoration (a construction from the state with a bound), in the form that remains after card 6.
The precedence peel (Theorem T3P) lowers the depth Φ at the six ends of the D2 height phase by 6% to 56% and then has no
round: at the chain ends, 131 own holders of high units are left, and the scan of T3P's family has no round at any of
them, for one of the reasons of K2 below (statuses of that family, not obstacles to descent: 15 of the 131 descend by
short words outside it, stage 15). Answer Questions 1 to 4. Each answer may be a proof, a reduction to finite certificates that a computer can
check at the state, or a precise counterexample or counterexample shape. Partial results are useful. Label every
statement proved, conditional (name the condition) or conjectured. Name every machine check as a certificate with its
exact statement, and do not claim that a certificate holds.

The target is unchanged: a configuration with every left height at most 2 (depth one), reached from a deep reachable
configuration. The standard is **tame** where that suffices; this card also admits a **bounded excursion**: a word
along which Φ ≤ Φ(start) + E and which ends with Φ lower, E stated in the data of the hypotheses. A progress measure is
read at the ends of such words, not after every letter.

## Model

As in [card 6](card-6-blocking-structure.md), whose model, terms and notation this card uses: the puzzle K⁺, the 600
grips c in five classes, caps H_c⁺, the menu Λ_c (12 retained items A4_c and 48 jumbles), the tree positions, heights,
branches and second vertices of pieces (card 5), the labels in, out and straddle, the straddler sets S_d and inside sets
I_d, clean and blocked grips, holders, the depth Φ = Σ_A max(0, h(A) − 2), the foreign load F_x and the foreign set J_x,
pure grips, units, high units, own holders, and tame words. Items are numbered 0 to 59 (0 the identity, 1 to 11
retained, 12 to 59 jumbles); products of items are maps (a·g means g first).

Terms from the joint attack on card 6 ([stage 14, `PROOFS.md`](../../restoration/14-blocking-structure/PROOFS.md)):
- **partners**: grips c ≠ d interact when Δ(c, d) = R²(|n|² + n_c·n_d) − 2κ² > 0; every grip has 80 partners, and a
  letter changes labels, inside sets and foreign loads only at its own grip and its partners (L1, L10);
- **lowering items** of a high unit G at an own holder x: the twelve items of the coset A4_x·h₀ that lower G (h₀ first);
  three of them send G into each of the other four branches. D_x(h) is the number of pieces in I_x of branch CLS(x),
  height at least 4 and second vertex h_L⁻¹v₀, so ΔΦ(x, h) = 2(F_x − D_x(h)) for a jumble at a clean grip (Lemma Φ);
- **round**: a certificate of Theorem T3P at a configuration (target x, unit G, lowering item h, helper letters with
  pairwise disjoint inside sets, an acyclic precedence digraph, P0 to P4);
- **level** lev(c) of a grip: 0 for clean grips, and lev(d) = 1 + max over straddlers p of d of min over holders c of p
  of lev(c) (Lemma L4); λ is the smallest level of a holder of a piece of height at least 4;
- **status** of an own holder x at a configuration, from the exhaustive scan of `t3p.py` (clean helpers that interact
  with x, every maximal disjoint cover, every item and lowering item), best first: **round**; **budget** (a cover with
  feasible items exists, but F ≥ D for every choice); **no feasible items**; **no disjoint cover** (the candidate
  helpers' inside sets overlap); **straddler without a clean holder** (some straddler of x has no clean holder).

## Given (proved; use as facts)

From card 5, card 6 and stage 10:
- **T5, Lemmas 5.2 and 5.3** (card 5). A twist (c, h) maps the left positions of the pieces it moves by h_L. A jumble of
  class Q lowers by 2 exactly the pieces of branch Q whose second vertex is h_L⁻¹v₀, keeps the height of the other pieces
  of branch Q, and raises every other piece by 2, into branch Q. Retained twists change no height. **A word of one class
  changes a height by at most 2 in total.**
- **Frames by height** (card 6). A piece of height 2 and branch P never straddles a cut of class P; a lattice piece
  straddles nothing; a high piece can straddle any cut.
- **Lemma L and Corollary L1** (card 6). A retained letter (c, k) at a clean grip permutes the label rows of the moved
  pieces (ℓ′(A, d) = ℓ(A, k⁻¹d)) and maps branches by σ_k; it keeps every piece's straddle number and number of holders.
- **Lemma T** (card 6). After a jumble (c, h) the grip c is clean, and pure unless h lowered a piece from height 4 or more.
- **Isolation and step lemmas of the X1 pump** ([stage 10, REPORT.md section 2.3](../../restoration/10-x1-unbounded/REPORT.md)).
  If two clean grips c and d hold a unit K whole, no other piece lies whole in both caps, the running products of the
  letters at each grip stay in its Good set (the items whose action on the cap's other pieces D_c puts every one of
  them out of the other cap), and before each letter K lies whole in that letter's cap or wholly outside it, then a word
  of letters at c and d is admissible, D_c and D_d move only by the products at their own grips, and K moves by the
  letters whose cap holds it. In X1 each letter raised K by 2.

From the joint attack on card 6 (`PROOFS.md`; section numbers of that file):
- **Lemma A and Theorem T3P** (section 2). Under P0 to P4 the helpers in a topological order of the precedence digraph,
  followed by (x, h), form a tame word that lowers Φ by at least 2(D − F); every hypothesis is a finite exact test at the
  state; at most 81 letters per round with clean helpers, 600 with T3P+ (a blocked helper whose straddlers all lie in
  the inside sets of earlier helpers). **Corollary B**: with non-interacting clean helpers the best items are chosen
  separately (minimize f − d per helper and lowering item). **Corollary I**: rounds chain, at most Φ/2 of them, and one
  replay checks the chain.
- **Lemma R, Corollaries U and E** (section 3). For x clean of class Q and a jumble h, F_x after (x, h) equals D_x(h),
  and the old foreign set J_x becomes the only relowering supply at x: Ψ_x = Φ + 2F_x is unchanged by every letter at x.
  The descent made by the letters at c is 2(F_c(end) − F_c(start)) + 2U_c, with U_c the net decrease of F_c caused by
  letters at other grips. After a lowering with the largest D at x, no word of letters at x alone lowers Φ.
- **Lemma P** (section 3). For a class P, Φ + 2J_P (J_P the off-lattice pieces of branch other than P) is unchanged by
  letters at grips of class P.
- **Lemmas L4, L4′, L5, L6 and Proposition N** (section 4). A grip of level ℓ needs at least ℓ letters before it can be
  clean; every tame descent has at least λ + 1 letters; λ = ∞ excludes every tame descent (and never occurs at a
  reachable configuration: every grip has a finite level there, stage 15, Proposition FL). A height-2 cell none of whose
  holders has its branch's class (an orphan) moves only by retained letters until a jumble at a clean holder c with
  D_c ≥ F_c ≥ |V| raises it to height 4. A clean grip with F > 0 and no own high piece has no tame jumble. A finite set of
  states closed under Φ-neutral letters, with no descending letter, is a complete negative certificate for tame descent.
- **Safety S1, S2** (section 5). Whether a letter at y lands named pieces and adds nothing to the obstruction of x is
  decided by a table of old and new flags over I_y; one image computation decides a whole landing coset A4_y·g.
- **Lemmas L8 and L9** (section 6). L8 (lower, then relower): after (x, h), a clean grip z ≠ x of class CLS(x) that holds
  the raised pieces J_x(Z) relowers them with an item g with g_L⁻¹v₀ = h_L v₀; the pair lowers Φ when
  2(F_x − D_x(h)) + 2(F_z − D_z(g)) < 0, with excursion at most max(0, 2(F_x − D_x(h))). L9 (lower, land, return): if h
  lowers G from height 4 into branch P′, z of class P′ is clean afterwards with I_z ∩ I_x = G, g lands G, every piece
  moved by (z, g) ends out at x, and F_z ≤ D_z(g), then (x, h)(z, g)(x, h⁻¹) lowers Φ by at least 2|G| with excursion at
  most max(0, 2(F_x − D_x(h))), and x's other pieces return to their poses.
- **Proposition P12 and Conjecture C13** (section 7). The inverse of a last jumble that lowered no high piece and moved a
  lattice piece is a tame landing letter. Conjectured: a tame landing letter exists at every reachable configuration
  off the lattice.

## Computed facts (exact; leads, not proofs)

Programs and data: [stage 14](../../restoration/14-blocking-structure/README.md) (`scripts/`, `results/`, section 5.7
and section 7). The **chain ends** are the configurations where the greedy chains of T3P rounds stopped, one for each
of the six D2 height ends (`results/s730N-t3p-chain.json`, end digests there and in `results/s730N-t3p-chain-end-scan.json`).

- **K1 (the chain ends).** Φ, number of high units, clean grips:

  | Seed | Φ at the D2 end | Φ at the chain end | High units (heights) | Own holders | Clean grips |
  | --- | --- | --- | --- | --- | --- |
  | 7302 | 1468 | 1376 | 20 (all 4) | 17 | 37 |
  | 7303 | 2206 | 1680 | 24 (all 4) | 19 | 27 |
  | 7304 | 1594 | 1498 | 15 (all 4) | 15 | 41 |
  | 7305 | 2838 | 2474 | 39 (all 4) | 35 | 25 |
  | 7306 | 3910 | 2684 | 29 (all 4) | 25 | 26 |
  | 7307 | 3170 | 1396 | 17 (15 at 4, 2 at 6) | 20 | 40 |

  At every chain end the scan finds no round and no single letter lowers Φ; λ = 0 (some clean grip holds a high piece),
  and the largest level is 5 to 8 (every grip of a reachable configuration has a finite level, so only the values carry
  information, as lower bounds).
- **K2 (why the T3P scan has no round at the 131 own holders), by status and level of the own holder:**

  | Status | Number | Levels of these own holders |
  | --- | --- | --- |
  | straddler without a clean holder | 84 | 2 (28), 3 (16), 4 (16), 5 (15), 6 (7), 7 (1), 8 (1) |
  | budget (F ≥ D for every feasible cover) | 32 | 0 (22), 1 (10) |
  | no feasible items | 10 | 1 (10) |
  | no disjoint cover | 5 | 1 (5) |

  So, within T3P's family, every own holder of level 0 or 1 is stopped by the budget or by its helpers, and every own
  holder of level 2 or more by a straddler that no clean grip holds. Outside it, 15 of the 131 descend by short words,
  among them four of level 2 with a straddler without a clean holder (140 of 7303, 162 of 7304, 177 of 7305 and 202 of
  7306; stage 15, section 2.2). For the 84, the straddlers without a clean holder number 1 to 281 per
  own holder, in 1 to 13 pose groups.
- **K3 (the budget cases).** (F, D) with the least F − D over the units and feasible covers of each of the 32: 7302: 12 (148, 134), 104 (29, 13), 171 (59,
  21), 482 (284, 82); 7303: 69 (113, 103), 282 (198, 87), 364 (120, 111), 365 (133, 78); 7304: 375 (123, 42), 522 (70,
  22), 540 (111, 58); 7305: 10 (111, 12), 44 (122, 103), 67 (111, 55), 102 (172, 10), 113 (10, 3), 226 (289, 3), 281
  (24, 15), 368 (560, 65), 421 (196, 59), 499 (111, 65); 7306: 139 (126, 104), 208 (29, 22), 252 (276, 6), 351 (636, 4),
  561 (18, 17); 7307: 10 (157, 41), 102 (135, 131), 226 (32, 15), 319 (111, 46), 474 (56, 10), 516 (395, 7). Ten of
  them are targets of rounds of the chain, and at each F is exactly the number of pieces of its last lowering there
  (Lemma R): for example 171 of 7302, with (F, D) = (21, 59) before its second lowering and (59, 21) after. 516 of 7307
  is of the same kind through a helper letter: (516, 14), a helper of the chain's first round, took 395 pieces from
  height 4 to 2 and raised 7 to height 6.
- **K4 (overlapping and infeasible helpers).** At 229 of 7304 the only candidate helpers 295 and 347 share 186 pieces;
  over all 7,200 two-letter words at them (both orders, all items, the second letter evaluated after the first), 23
  leave 229 clean and the least F − D at 229 among those is 25 (stage 14, DIS-5; computed at the D2 end, digest
  `ac127dc3…`; at the chain end the same enumeration gives 19 clean words and again 25, stage 15, section 2.3). At 71 of
  7302 the candidate 20 shares
  166 pieces with the candidate 112 and 85 with the candidate 185.
- **K5 (the 7304 stop, before the chain).** Two tame descents are known there: W3 = (344, 9), (451, 55), (522, 14)
  (Φ 1594 → 1498) and W5 = W3 with item 49 at 522, then (451, 44), (522, 15) (Φ → 1472). After item 14 the 70 lowered
  pieces lie in no clean cap other than 522 (a self-lock for helpers at other grips only: a retained letter at 522
  itself re-aims them, and the chain end followed by (522, 8) is W5's configuration exactly, stage 15); after item 49
  they lie in cap 451, whose letter (451, 44) unloads them. At 540, reported in stage 14 section 5.3: the replay of (540, 12), (582, 29), (540, 12⁻¹), an instance
  of the L9 shape without its hypothesis F_z ≤ D_z (F_582 = 514), gives Φ 1700, 2728, 2754.
- **K6 (the measure).** At the D2 end of 7307 each of the twelve items of one lowering coset at 516 lowers 395 pieces
  and raises 7 to height 6, taking Φ from 3170 to 2394; the D2 rule's measure Q = 177121 n₆ + n₄ refused them. Φ is the
  measure of this card.

## Questions

### 1. Multi-level unlocking

1. Give a construction that, from a reachable X, a high unit G and an own holder x of G with lev(x) = ℓ ≥ 2, produces a
   word after which x is clean, holds G, and has F < D(h) for a lowering item h, tame or with a bounded excursion, with
   hypotheses that are finite exact tests at X and explicit bounds on the length (in ℓ and the data of the hypotheses)
   and on the excursion. Candidates: nested rounds (a release tree whose letters at grips of level k are enabled by
   letters at grips of level k − 1, extending T3P+ and Lemma A; an enabler need not be a partner of x, since it acts on
   x only through the helpers it frees: 277 is not a partner of 162 of 7304); and conjugation u·(x, h)·u⁻¹, where u frees x, with
   conditions under which u⁻¹ stays admissible after the lowering and restores what u moved.
2. Define the **release dependency structure** at X: for each blocked grip d, its straddler units without a clean
   holder, their holders and those holders' straddlers, down to clean grips. Prove that it has a startable part (a
   letter that is tame and lowers some level or some straddle count that matters), or characterize the cycles that
   prevent this and the smallest controlled excursion that breaks one. Every grip has a finite level at every reachable
   configuration (reverse the history), so finiteness decides nothing; reversing the history neither keeps G nor stays
   tame, and the question is a release that does both.
3. Extend T3P to overlapping helpers (K4): helpers whose inside sets meet, each evaluated at the configuration where it
   acts, with hypotheses that are still finite tests at X (or at a bounded number of intermediate configurations) and
   with the precedence argument of Lemma A.

### 2. The lowering machine

1. **Reverse pump.** State and prove a lowering theorem in the shape of the X1 pump: a high unit G whole in the caps of
   two clean grips x and z of different classes, the other pieces of each cap isolated from the other grip by Good sets,
   and letters that lower G alternately at x and z. Each other piece moves only by the letters at one grip, so its height
   changes by at most 2 in total (Lemma 5.3); bound the excursion by the foreign loads of the two caps, and give a return
   (running products ending in the retained items, or the inverses of the first letters, as in L9) after which the other
   pieces are back at their heights. For a unit at height 4, L9 is the one-sided case: it returns x's other pieces only,
   and when the product at the grip that acted last on G is not retained, that grip's return raises G again unless G
   has first left its cap or been landed by the other grip (stage 15, Proposition RP).
   Give the hypotheses as finite exact tests at X,
   the excursion, the length and the descent.
2. **Relays.** For an own holder x of status budget (K3), Corollaries U and E say that descent at x needs letters at
   other grips that lower F_x or raise D_x. Give a construction (a relay: letters at grips of the classes of the foreign
   pieces that land them or relower them, as in L8, possibly in a chain x → z₁ → z₂ → …) with finite hypotheses and
   bounds on length and excursion. Say which of the 32 budget cases it reaches, and how it treats a cap that holds only
   what it lowered (the ten of K3).
3. **Where the lowered pieces go.** A lowering chooses, through its item, the new branch and position of the pieces it
   lowers, and so which caps hold them afterwards (K5: item 49 against item 14 at 522). Give a rule for choosing the item
   that keeps the lowered pieces in a cap where a later letter can unload or land them, with a proof, or show that no
   item works at a stated configuration.

### 3. Placement

1. Characterize what Φ-neutral letters (retained letters at clean grips; jumbles at clean grips with F = D, by Lemma Φ) can do
   to a high unit G and its surroundings: which own holders and which pairs of grips holding G whole they can reach, and
   what happens to the foreign loads and straddlers of those grips (Lemma L permutes label rows; Corollary L1 and Lemma
   L5 limit what retained letters change).
2. **Placement theorem.** Under finite hypotheses at X, give a Φ-neutral word (or one with a bounded excursion) of
   bounded length after which the hypotheses of Question 1.1, 2.1 or 2.2 hold for G; or give an obstruction (an
   invariant of the allowed letters, in the manner of L1 and L5) that rules placement out at a stated configuration.

### 4. An order that does not jam

Give a state-only procedure that combines T3P rounds with the constructions of Questions 1 to 3 and has a progress
measure (Φ at the end of each macro step, with the excursion inside a step bounded), and prove that it reaches depth one
on a stated class of configurations that contains the six chain ends, or explain why they lie outside it. Or show that
every procedure of a stated kind jams at some reachable configuration, with the configuration as a pose table or an
exact word from solved; a finite negative certificate in the manner of Proposition N, extended to words with excursion at
most E, counts.

## Return

A proof blueprint with numbered lemmas for each question attempted. State every certificate exactly (what is computed,
on which objects, and what it must show). Give counterexamples as exact words from solved or as pose tables. Keep proved,
conditional and conjectured statements apart. For each construction, name an adjudicating experiment at one or more of
the six chain ends (the input table, the procedure or the word, and the outcome that confirms or refutes it), and say
which own holders of K2 and K3 it is expected to reach.

## Attached evidence

In [`../../restoration/14-blocking-structure/`](../../restoration/14-blocking-structure/):

- `README.md`: card 6's lemmas with proofs, the measured tables, the joint attack (section 5) and the six D2 ends
  (section 5.7); `PROOFS.md`: the proofs of the joint attack.
- `scripts/`: `blockmap.py` (exact label table and blocking map), `jamcheck.py` (anatomy of own holders, exact single
  letters), `retained_search.py`, `coheld.py`, `peel.py`, `replay_word.py` (independent replay of a word), and
  `locality.py`, `levels.py`, `t3p.py` (`check`, `scan`, `pairs`, `plus`), `test_t3p.py` (their gate) and `chain.py`
  (greedy chains of checked rounds).
- `results/`: `sources.json` (the states and their tables), the scans, levels, chains and replays at the six D2 ends and
  chain ends (`s730N-t3p-scan.json`, `s730N-levels.json` (7304: `s7304-levels.json`), `s730N-t3p-chain.json`,
  `s730N-t3p-chain-replay.json`, `s730N-t3p-chain-end-scan.json`, `s730N-chain-end-levels.json`).
- A chain end's pose table is the D2 end's table (`results/sources.json`) followed by the chain's word
  (`s730N-t3p-chain.json`, field `word`); `chain.py` and `replay_word.py` rebuild it exactly.
- In [`../../restoration/10-x1-unbounded/`](../../restoration/10-x1-unbounded/): the X1 pump, its Good sets and the
  isolation and step lemmas (`REPORT.md` section 2).
- The scripts run on the exact jumbling simulator of the research tree (`research/jumbling`) with
  `python3 -I -B <script> ...` from the repository root. They are exact computations in synthetic geometry, not
  native-host evidence.
