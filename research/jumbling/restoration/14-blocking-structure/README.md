# Stage 14: the blocking structure of deep states (24 October 2026)

Status: **research; the joint attack on card 6 is done (section 5).** The owner changed the method of the height 4
to height 2 research on 24 October 2026
([decision](../../../../docs/wiki/decisions/owner-decisions-2026-10-24-theorem-program.md)): each obstacle is stated
as a theorem with hypotheses that can be checked at a state, a construction with an explicit bound, and exact
certificates; computation tests hypotheses and checks certificates. This stage is the first problem: which grips are
blocked at deep states, by which pieces, and how a grip that a lowering needs can be freed without raising heights.
The problem card for the joint attack is [card 6](../../theory/cards/card-6-blocking-structure.md).

Labels: **proved** (with the proof here), **computed** (exact, by the script named), **lead**, **open**. Everything
is headless Linux research code on exact geometry; nothing here is Windows, DirectX, input or performance evidence.

## 1. Terms

Menu I_a; a piece's height h is the left tree distance of its pose (even; right distance 0), its branch (outer class)
the class whose centre is the first step of the geodesic from v₀ ([card 5](../../theory/cards/card-5-depth-and-lattice-group.md),
Lemma 5.1). Every grip c has a class CLS(c) (120 grips per class).

- A grip is **clean** when no piece straddles its cut (its twists are admissible), **blocked** otherwise; S_d is the set
  of straddlers of grip d, and I_c the set of pieces inside cap c.
- The **label** ℓ(A, d) ∈ {in, out, straddle} of piece A at grip d says where its posed region lies relative to H_d;
  the **holders** of a set of pieces are the grips whose caps hold every piece of the set whole.
- **Depth** Φ = Σ_A max(0, h(A) − 2). Φ = 0 is depth one.
- The **foreign load** F_x of grip x is the number of pieces inside cap x of height at least 2 whose branch is not
  CLS(x). A clean grip with F_x = 0 is **pure**.
- A **unit** is a set of pieces with one pose. A **high unit** has height at least 4. Its **own holders** are its
  holders of its own branch's class; only an own holder can lower it (card 5, Lemma 5.2).
- The **obstruction** of a grip x is S_x together with its foreign pieces.

## 2. Lemmas

**Lemma L (label calculus; proved).** Let c be a clean grip, k ∈ A4_c, and X′ = (c, k)X. Then:
1. a piece outside cap c keeps every label;
2. a piece inside cap c has ℓ_{X′}(A, d) = ℓ_X(A, k⁻¹d) for every grip d, where k⁻¹d is the grip with pole k⁻¹n̂_d
   (this is the retained conjugation of R22D-01);
3. heights do not change, and the branch of a moved off piece becomes σ_k(branch), where σ_k is the permutation of
   the five classes with CLS(k·e) = σ_k(CLS(e)).

*Proof.* (1) The piece does not move. (2) Its region R becomes kR, and n_d · ky = (k⁻¹n_d) · y = n_{k⁻¹d} · y because
k ∈ K⁺ permutes the poles; so kR lies inside, outside or across H_d exactly as R does for H_{k⁻¹d}. (3) The left
factor a of k is a unit icosian, which fixes v₀, so d(v₀, at) = d(v₀, t), and the first step of the geodesic to at is
a applied to the first step towards t. The menus satisfy Λ_{k·e} = kΛ_e k⁻¹ (for k = [a, b], n̂_{k·e} = a n̂_e b⁻¹), so
the left factor group of class CLS(k·e) is a L_{CLS(e)} a⁻¹, and the neighbour of v₀ it fixes is a·w_{CLS(e)}; hence
the class centre of the moved piece's branch becomes the centre of class σ_k(branch). ∎

So the dynamics of retained twists at clean grips is a permutation action on the label table, exact without geometry.
`retained_search.py` replays every word it reports on the exact simulator and rejects the word unless the replay
confirms the label calculus: every letter admissible, the target grip holding the unit at its height, and its straddler
count and foreign load as predicted. The ten words it reported at the seed 7304 stop and the three of the deeper search
pass (section 4).

**Corollary L1 (proved).** Under retained twists at clean grips:
1. every piece keeps its straddle number (how many cuts it straddles) and its number of holders;
2. the straddle incidence Σ_d |S_d| does not change; a retained twist only moves blocks from cut to cut:
   |S_d| after (c, k) = |S_d| − |I_c ∩ S_d| + |I_c ∩ S_{k⁻¹d}|;
3. two pieces with equal holder sets keep equal holder sets, and pieces that move together keep equal (or different)
   branches.

So retained twists alone never lower the straddle incidence. In the measured states it is between 849 (the D1 bulk
stop, with 1,193 pieces off) and 144,682, and at least 98,166 at every state of the 3000-twist lines (section 4).
`retained_search.py` checks point 2 at its target grips: after each replayed word the measured straddlers equal the
label calculus's prediction.

**Lemma Φ (one jumble; proved from card 5, Lemma 5.2).** Let x be a clean grip of class Q and h a jumble item of
Λ_x. The twist (x, h) changes heights as follows. Every lattice piece in the cap goes to height 2, so Φ does not
change. Every piece of another branch at height at least 2 rises by 2. A piece of branch Q falls by 2 if the second
vertex of its geodesic is h⁻¹v₀; otherwise its height does not change. Hence

  ΔΦ(x, h) = 2·(F_x − D_x(h)),

where D_x(h) counts the pieces in the cap of branch Q, height at least 4 and second vertex h⁻¹v₀. A pure grip's
twists never raise Φ, and a lowering twist at a pure grip lowers it.

**Lemma C (co-holding; proved).** A twist at grip c moves a piece exactly when c is one of its holders. So pieces with
equal holder sets are moved by the same twists. By Corollary L1 they keep equal holder sets under retained twists, so
only a jumble can separate them. If every member A of a high unit G has the same holder set as a piece p of another
branch at height at least 2, then p is in the foreign load of every own holder of G, and a lowering of G after retained
twists only raises p in the same twist (the two stay co-held, and their branches stay different). The hypothesis is per
member: the holder set of the unit (the intersection over its members) is not enough, because a twist at a grip that
holds only part of G splits it (finding of the joint attack, section 5.4).

**Lemma T (the last jumble; proved).** If X = (c, h)Y with h a jumble item and CLS(c) = Q, then c is clean at X
(Proposition 2.2 of the theory draft). The pieces in its cap are:
- pieces that were lattice pieces, now in a frame of Q;
- pieces of branch Q that h did not lower;
- pieces of another branch that h raised, now of branch Q;
- pieces that h lowered from height 2 to 0;
- pieces that h lowered from height h′ ≥ 4 to h′ − 2 ≥ 2. Their new branch differs from Q: the image of the old
  third vertex is a neighbour of v₀ other than w_Q.

So c is pure unless the last twist lowered a high piece. Undoing it with (c, h⁻¹) changes Φ by exactly minus what
the twist changed.

## 3. Tools (`scripts/`)

| Script | What it computes |
| --- | --- |
| `blockmap.py` | Exact label table of every piece at every grip (cached in a `.npz` file, about 1.2 MB), then per grip: class, straddlers by height and branch, clean, pure, foreign load; the matrix M[c, d] of pieces inside c that straddle d; the exact effect of every retained twist at every clean grip on every straddler set (Corollary L1); clean covers of blocked grips; for every high unit its holders and own holders with their state. Checks: no lattice piece straddles; no height-2 piece straddles a cut of its own class (card 3, Theorem B); the table reproduces its digest. |
| `jamcheck.py` | For the own holders of every high unit (or named grips): the obstruction in units (pose groups), each unit's cell and its clean holders (pure own class, impure own class, other class) and blocked holders, and the foreign load. With `--moves`, every non-identity menu twist at the clean holders of the obstruction units is applied exactly and undone, and the change of Φ, the largest height, the off count, the straddlers and foreign load of the target and the units that land are recorded. |
| `retained_search.py` | Lemma L as a search: a beam search over words of retained twists at clean grips for an own holder of a high unit that is clean and pure, with cost straddlers + foreign load; every reported word is replayed exactly. |
| `coheld.py` | Lemma C: for every high unit, the number of distinct holder sets of its members, the foreign pieces whose holder set equals that of every member (co-held; none when the members' sets differ), separately those whose holder set equals the unit's common holder set (not enough for Lemma C), and those whose holder set contains the unit's. |
| `peel.py` | Two-letter peeling of a target grip x through two clean grips A and B, then a lowering: every item at A, the best by straddlers + foreign load of x each followed by every item at B, and when x is clean every item at x, with the exact change of Φ and n4. Exact for the moved pieces; every twist undone and the digest checked. |
| `replay_word.py` | Replays a literal word from a stored table, recomputing every height from scratch after each letter: admissibility, n2, n4, n6, Φ, the largest height, the off count and the digest. |
| `locality.py` | Lemma L1: Δ(c, d) for every pair of grips, exact in Q(√5); the partners of every grip; the shells of grips around grip 0 by n_0·n_d. |
| `levels.py` | Lemma L4 and Corollary L4′: the unlock level of every grip (the greatest fixed point), their histogram, λ with a witness, and for every high unit its own holders with their levels and its members' holder rows (number of distinct rows, intersection, union, clean grips that hold a proper part of the unit). |
| `t3p.py` | Theorem T3P as finite tests. `check` verifies a certificate (rounds of a target, a unit, a lowering item and helpers): P0 to P4 from one image of each helper, then an exact replay that compares the moved sets, each ΔΦ and the predicted Φ. `scan` takes every own holder of every high unit and every maximal pairwise disjoint cover of its straddlers by clean helpers that interact with it, with all 60 items and all twelve lowering items (Corollary B, or every assignment when precedence edges can arise); it gives each target a status (round, budget, no-disjoint-cover, no-feasible-items, straddler-without-clean-holder), lists the single letters that lower Φ, and writes the best round as a certificate. `pairs x y z` evaluates every two-letter word at y and z in both orders, recomputing the second grip's legality and contents after the first letter, and records S, F and the largest D at x after each admissible tame word. `plus` lists the instances of T3P+ with one unlocking helper (a blocked grip y whose straddlers all lie in one clean grip z disjoint from it, and a tame letter at z that sends them out of y and nothing into or across it), each replayed. |
| `chain.py` | Corollary I as a greedy measurement: at each state the best round of `t3p.py scan`, executed by `t3p.py check` (P0 to P4, then the exact replay), until the scan finds no round; it records each round, the scan's statuses before it, the word and the reason for the stop. Its word is replayed independently with `replay_word.py`. |
| `test_t3p.py` | The gate of `locality.py`, `levels.py` and `t3p.py` (seven parts, 2,611 assertions, about 10 CPU minutes): locality, levels, the W3 and W5 certificates and tampered copies, a restricted scan, restricted pairs, `plus` with a limit, and the incremental label update against a full rebuild. |

The label table takes 2 to 4 CPU minutes per deep state (one process); the rest runs in seconds to minutes, except
`pairs` over all items (section 5.3). `locality.py`, `levels.py` and `t3p.py` write only outside the repository.
Commands are in section 7.

## 4. Measured facts (exact)

Inputs are the runner's pose tables in `13-tutorial-runner/runs/dev/` (`results/sources.json` lists each table, its
SHA-256 and its digest): the D0 and D1 scrambles of 20 and 40 twists and the D1 bulk stop; for seed 7303 the
3000-twist scramble, the state after its 18,900 s bulk, the states after the first and third steps of the height
rule `r22d75`, and the end of its height phase; for seeds 7305 and 7304 the census stop, for 7305 also the state after
21,600 s of bulk from it, and the end of the height phase; and the ends of the height phases of seeds 7306, 7307 and
7302. The six height ends are the inputs of the HD2P/2 campaign. Summaries: `results/states.json`.

| State | Digest | Off | n2 | n4 | n6 | Clean grips | Pure | Straddle pairs (height 2 / ≥ 4) | Median straddlers of a blocked grip | High units | … with a clean own holder | … with a clean pure own holder |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- | ---: | ---: | ---: | ---: |
| D0 7106 scramble (20 twists) | `9ad04d45…` | 51,828 | 51,828 | 0 | 0 | 255 | 255 | 58,140 (58,140 / 0) | 160 | 0 | 0 | 0 |
| D1 7101 scramble (40 twists) | `be08cfaa…` | 90,441 | 90,441 | 0 | 0 | 85 | 85 | 106,919 (106,919 / 0) | 200 | 0 | 0 | 0 |
| D1 7101 bulk stop | `7e128820…` | 1,193 | 1,193 | 0 | 0 | 578 | 565 | 849 (849 / 0) | 34.5 | 0 | 0 | 0 |
| 7303 scramble (3000 twists) | `ee1a56a8…` | 116,111 | 114,470 | 1,552 | 89 | 26 | 23 | 144,682 (140,437 / 4,245) | 248 | 42 | 14 | 9 |
| 7303 after 18,900 s of bulk | `516f427d…` | 96,817 | 95,559 | 1,169 | 89 | 25 | 17 | 105,673 (102,546 / 3,127) | 168 | 28 | 10 | 7 |
| 7303 after height step 1 | `6d72a29b…` | 98,096 | 96,838 | 1,171 | 87 | 25 | 15 | 107,756 (104,632 / 3,124) | 175 | 28 | 9 | 6 |
| 7303 after height step 3 | `65d33e72…` | 100,900 | 99,884 | 929 | 87 | 26 | 13 | 111,419 (108,924 / 2,495) | 185 | 22 | 3 | 0 |
| 7303 D2 height end | `d01dc91e…` | 100,650 | 99,634 | 929 | 87 | 26 | 13 | 112,321 (109,826 / 2,495) | 188.5 | 22 | 3 | 0 |
| 7305 census stop | `273eab69…` | 105,437 | 103,140 | 2,297 | 0 | 24 | 16 | 117,206 (111,880 / 5,326) | 198 | 58 | 27 | 17 |
| 7305 after 21,600 s of bulk | `736ea9e7…` | 101,937 | 99,640 | 2,297 | 0 | 25 | 18 | 112,162 (106,976 / 5,186) | 191 | 58 | 21 | 20 |
| 7305 D2 height end | `cc5ef7ec…` | 109,872 | 108,453 | 1,419 | 0 | 22 | 12 | 126,117 (122,909 / 3,208) | 214 | 42 | 6 | 0 |
| 7304 census stop | `7e2304b2…` | 95,569 | 94,384 | 1,185 | 0 | 41 | 34 | 98,166 (95,286 / 2,880) | 159 | 19 | 8 | 8 |
| 7304 D2 height end | `ac127dc3…` | 96,471 | 95,674 | 797 | 0 | 41 | 29 | 103,763 (102,003 / 1,760) | 169 | 14 | 3 | 0 |
| 7306 D2 height end | `ac4be82c…` | 110,315 | 108,360 | 1,955 | 0 | 26 | 8 | 128,899 (124,638 / 4,261) | 208.5 | 34 | 6 | 0 |
| 7307 D2 height end | `895b45de…` | 100,569 | 98,984 | 1,585 | 0 | 38 | 21 | 111,078 (107,147 / 3,931) | 185.5 | 28 | 5 | 0 |
| 7302 D2 height end | `2430f4cc…` | 103,484 | 102,750 | 734 | 0 | 37 | 28 | 110,769 (109,318 / 1,451) | 189 | 21 | 6 | 0 |

Checks held at every state: no lattice piece straddles a cut, and no height-2 piece straddles a cut of its own
branch's class. The blocked count at `ac127dc3…` (559) equals the HD2P/2 campaign's Bk there, and the best retained
twist found by `blockmap.py` is at grip 23, the grip of HD2P/2's first PEEL step.

**What the table shows.**
- Deep states are jammed: 22 to 41 of 600 grips are clean along the 3000-twist lines. Almost all blocking comes from
  height-2 pieces, which lie in other classes' frames (95.4% to 98.7% of the straddle pairs); the rest comes from
  high pieces.
- The height rule uses up the lowerings. At the 3000-twist scramble, the census stops and after the bulk, 6 to 20
  high units have a clean pure own holder; after the first steps of the height rule, and at every height end, none.
- After its bulk, the depth-40 line is almost free: 578 clean grips and no piece above height 2. At depth 3000 the
  bulk changes the picture little in 5 to 6 hours.

**Anatomy at the 7304 stop** (`results/s7304-jam.json`, `results/s7304-coheld.json`). The 14 high units have 13 own
holders between them, one or two per unit. One of them is clean (540, foreign load 111, all of branch 1). The others
have:
- 26 to 299 straddlers in 2 to 14 units, mostly cell contents of other classes' frames of 78 to 197 pieces, or parts
  of large frame domains of 993 to 2,762 pieces;
- foreign loads of 78 to 1,552 pieces.

Unit by unit, the obstruction of an own holder is of three kinds. Some of it is held by a clean pure grip of its own
class: at 522 all of it, 127 pieces of branches 1 and 3 held by the clean pure grips 451 (class 1) and 344 (class 3);
at 64 all 885 foreign pieces and 95 of the 136 straddlers. Some is held only by clean grips of other classes or by
impure ones. And some is held by no clean grip at all: at 2 and 183 the whole obstruction, at 596 almost all of it.
Some straddlers are high units of the own branch that straddle the own holder of another unit (1007 at 64; 1247 and
1378 at 596). No high unit has a co-held foreign piece (Lemma C); unit 2304 has 25 foreign pieces whose holder sets
contain its own.

**Retained preparations** (Lemma L; `results/s7304-retained.json`, `results/s7304-retained-deep.json`). The beam
search (depth 6, beam 6) lowers the cost straddlers + foreign load of the best own holder for 10 of the 14 units, from
111 to 559 to between 12 and 322, with words of 1 or 2 retained twists, and finds no improvement for the other 4.
Every word was replayed exactly, and the replayed straddler count and foreign load equal the label calculus's
(`results/s7304-retained-check.json`, `results/s7304-retained-deep-check.json`: `--check` of both outputs). For
unit 1557 the best own holder becomes pure, with 12 straddlers left; for units 1132, 1582 and 2559 at 522, 2
straddlers and 17 foreign pieces are left. A deeper search (depth 14, beam 30) finds nothing better for three units.
No unit reached a clean pure own holder.

**Single letters at the obstruction's holders** (`jamcheck.py --moves`; `results/s7304-moves-522.json`). At 522 all
118 non-identity letters at 451 and 344 leave Φ unchanged (both grips are pure). The obstruction of 522 lies in the
disjoint caps 451 and 344 (22 straddlers and 91 foreign pieces in cap 451, 4 and 10 in cap 344), so each count after
one letter is that letter's own contribution plus the untouched part in the other cap. The best letter leaves 6
straddlers and 27 foreign pieces (a retained letter at 451). The letters that land obstruction pieces each add a
straddler or a foreign piece, and not only from lattice pieces: (451, 43) lands three foreign units and adds 26
branch-1 pieces of height 2 that move from frame to frame (it leaves 4 straddlers and 36 foreign pieces, of which 4 and
10 are the untouched part), while the landings (344, 25) and (344, 30) add lattice pieces taken into frames. The
letters that land the 87-piece unit 1910 leave at least 28 straddlers.

**Two-letter peeling of 522, then a lowering** (`peel.py`; `results/s7304-peel-522.json`; replay in
`results/s7304-peel-522-replay.json`). All 59 non-identity items at 451 were applied; the 14 with the smallest
straddlers + foreign load of 522 were each followed by all 59 items at 344 (826 pairs, all admissible). 18 pairs make
522 clean: the first letter (451, 55) or (451, 43), each with the same nine items at 344 (retained items 1, 7 and 9 and
jumbles 13, 16, 19, 24, 43 and 56). After each of the 18, Φ is still 1594 and the foreign load of 522 is between 22
and 52, and the item 14 at 522 then lowers Φ by 36 to 96 with no height above 4. The best word,
(451, 55), (344, 9), (522, 14), replayed from the table with every height recomputed after each letter, takes n4 from
797 to 749, Φ from 1594 to 1498 and the off count from 96,471 to 95,934. No letter raises Φ, so the word is tame. Its
first two letters at two different helpers are what the single letters above and the words of HD2P/2 (at most two
letters) cannot express.

**HD2P/2** (pull request 131: literal words of at most two letters, SINGLE, CL2 and PEEL; Bk the global blocked count).
It ended STUCK at all six D2 height ends, and all six independent audits passed
([campaign record](../13-tutorial-runner/runs/hd2p2/README.md)).

## 5. Joint attack (24 October 2026)

Card 6 was attacked as the owner decided on 24 October: two sealed analyses of the same card, by Astra (call
`20261024T115347Z-88884066`) and by Claude Opus (a subagent of the integrator's session); an exchange and a
cross-examination, in which each analyst answered every finding of the other (Astra's: call
`20261024T131110Z-b454d61a`); then exact experiments for the points still in dispute, run by the integrator with tools
that Astra wrote from the merged statements (implementation call `20261024T141707Z-6cca9dcc`, reviewed by the
integrator). The analyses are private review records; this section records what they established. The proofs are in
[`PROOFS.md`](PROOFS.md); each was checked line by line by the analyst who did not write it and by the integrator, and
a Sol review of the proofs and their summaries (call `20261024T151953Z-cdb32589`) led to three corrections, verified
in two scoped rounds. Afterwards the same tools ran at all six D2 ends, with greedy chains of rounds (section 5.7).

Labels as at the top of this page, and one more: **reported**, computed during the attack by an analyst's own program,
which is not committed ("both" when both analysts computed it independently).

### 5.1 The construction (Question 1.1)

**Theorem T3P (precedence peel; proved, `PROOFS.md` section 2).** At a configuration Z, take a target grip x of class Q,
a high unit G whose members all lie inside cap x, a jumble h at x that lowers G, and helper letters (y_i, g_i) at other
grips, with inside sets I_i. Suppose that:
- P0: the I_i are pairwise disjoint and avoid G, and each y_i is clean (or, T3P+, its straddlers lie in the sets of
  helpers that act before it);
- P1: the precedence digraph (y_i acts before y_j when the image of a piece of I_j is not out at y_i) is acyclic;
- P2: no helper letter raises Φ;
- P3: no piece straddles x after the helpers, counting the images of the I_i and the other pieces where they are;
- P4: after the helpers, the foreign load F of x is smaller than the number D of pieces that h lowers from a height of
  at least 4.

Then the helpers in a topological order, followed by (x, h), form a tame word that lowers Φ by at least 2(D − F). Each
hypothesis is a finite exact test at Z: the label table and one evaluation of each helper item. With clean helpers, a
minimal certificate has at most 81 letters, since only the 80 grips that interact with x matter (Lemma L1). Rounds of
the theorem can be chained, at most Φ/2 of them, and one replay checks the whole chain. When no precedence edge can
arise, the best items are chosen separately for each helper (Corollary B: minimize f − d for each helper and lowering
item, not f alone).

The theorem merges the two sealed constructions: Astra's precedence graph with Claude Opus's end condition. Its
hypotheses are weaker than either. Astra's version needed pure helpers whose safe landings cover the obstruction, and
F = 0 at the end; Claude Opus's needed pairwise non-interacting helpers. The helpers need not land the obstruction of
x, keep its identities or make x pure.

### 5.2 The other results (proved; `PROOFS.md`)

- **Locality (L1, L10).** If Δ(c, d) = R²(|n|² + n_c·n_d) − 2κ² ≤ 0 (exact in Q(√5)), a piece inside cap c is out at
  d and a piece inside d is out at c, at every configuration. So a letter changes labels, inside sets and foreign loads
  only at its own grip and its partners, 80 per grip.
- **Per-cap accounting (Lemma R, Corollaries U and E, Lemma P; Question 3.2).** A lowering at x turns the pieces it
  lowers from a height of at least 4 into the new foreign load of x, and the old foreign load into the only relowering
  supply at x: Ψ_x = Φ + 2F_x is unchanged by every letter at x. So the descent made at a cap is 2(F_end − F_start)
  plus twice the unloading by letters at other grips, and after a best lowering no letter at that cap alone lowers Φ
  again. For a class P, Φ + 2J_P (J_P the off-lattice pieces of other branches) is unchanged by letters at grips of
  class P. This is the mechanism of C2: the height rule lowers at a cap until no lowering item there has D > F, and the
  cap descends again only after letters at other grips lower its foreign load or raise its lowering supply.
- **Unlock levels (L4, L4′; Question 1.3).** The level of a grip bounds from below the number of letters before it can
  be clean. With λ the smallest level of a holder of a high piece, every tame descent has at least λ + 1 letters, and
  λ = ∞ proves from one fixed-point computation that no tame descent exists.
- **Orphan cells, one grip, plateau certificates (L5, L6, N; Question 1.3).** A height-2 cell none of whose holders has
  its branch's class moves only by retained letters, until a jumble at a holder with at least as many own high pieces
  raises it. Letters at one cap with a foreign load and no own high piece reach only its twelve retained images. A
  finite set of states, closed under neutral letters and without a descending letter, is a complete negative
  certificate; a search that stops on a limit is not.
- **Safety (S1, S2; Question 1.2).** Two predicates: safe (the letter lands the named pieces and adds no straddler and
  no foreign piece, setwise) and strictly safe (no moved piece ends in the obstruction). Both are decided by a table of
  old and new flags over the moved pieces, which must include frame pieces as well as lattice pieces. For the twelve
  items of a landing coset A4_y·g, the new flags come from one image computation of g, read at the twelve grips a⁻¹x;
  the old flags come from the label table. At deep states the table is not a function of the other coset g·A4_y, as
  it is at the solved state (R21F-09).
- **Bounded excursions (L8, L9; Question 2).** Lower, then relower the raised pieces at a grip of the same class
  (length 2, excursion at most max(0, 2(F − D))); lower, land the lowered unit at a grip of its new branch, and undo
  the first letter (length 3, Φ lower by at least 2|G|).
- **Top layer (P12, C13; Question 3.1).** After a jumble that lowered no high piece and moved a lattice piece, its
  inverse is a tame letter that lands that piece. Conjecture C13: every reachable configuration off the lattice has a
  tame landing letter. The analogue for lowering letters is false (section 5.3).

### 5.3 Facts at the 7304 stop (exact)

All at `ac127dc3…` (Φ = 1594) unless stated otherwise; outputs in `results/` (commands in section 7).

- **Locality** (`locality.py`; `results/locality-summary.json`). R² = 96 − 32√5, and every grip has exactly 80 partners.
  Around grip 0, ordered by n_0·n_d, the shells of 4, 12, 24, 12, 4 and 24 grips interact with it and the next shell
  (24 grips) does not. Grips 451 and 344 do not interact (Δ = (1440832 − 811072√5)/15625 < 0); the helper pairs at 229
  (295 and 347) and at 336 (277 and 295) do.
- **W3 and W5 as certificates** (`t3p.py check`; `results/s7304-w3-certificate.json` and
  `results/s7304-w5-certificate.json`, checked in `results/s7304-t3p-check-w3.json` and `results/s7304-t3p-check-w5.json`).
  Both pass P0 to P4, and the replay reproduces every digest and Φ of section 4 and card 6 C9. W3 is one round at
  x = 522 with h = 14 and the helpers (344, 9) and (451, 55): inside sets of 2,334 and 2,989 pieces, disjoint, no
  precedence edge; contributions (s, f) = (0, 0) and (0, 22); s₀ = f₀ = 0; D = 70 (the units 1132, 1582 and 2559 of
  14, 15 and 41 pieces, all lowered by each of the twelve items); ΔΦ = 2(22 − 70) = −96. W5 is that round with h = 49,
  then a second round at `d4851f18…` (Φ = 1498): x = 522, helper (451, 44) with an inside set of 3,076 pieces and
  (s, f) = (0, 9), s₀ = f₀ = 0, D(15) = 22 (four units of 3, 5, 6 and 8 pieces), ΔΦ = −26, ending at `956d3d3e…` with
  Φ = 1472 and n4 = 736.
- **The pairs at 522** (from `results/s7304-moves-522.json` and `results/s7304-peel-522.json`). The items that leave no
  straddler at 522 are 38, 43, 44 and 55 at 451 (f = 114, 26, 207 and 22) and 1, 7, 9, 13, 16, 19, 24, 43 and 56 at 344
  (f = 5, 11, 0, 0, 5, 14, 26, 14 and 0). So exactly 36 pairs make 522 clean, and F < D = 70 for the 18 with item 43
  or 55 at 451, the 18 of C7; for all 826 pairs the counts after two letters are the sums of the one-letter
  contributions (C8). Every d_i is 0 here, so Corollary B reduces to the smallest F: 22, with (451, 55) and one of the
  items 9, 13 and 56, after which no piece of cap 344 straddles 522 or is foreign there.
- **Unlock levels** (`levels.py`; `results/s7304-levels.json`). The 600 grips have levels 0 (41, the clean grips),
  1 (102), 2 (114), 3 (150), 4 (142) and 5 (51); none is infinite. The own holders: 540 at 0; 229, 336 and 522 at 1; 2
  and 375 at 2; 64, 162, 239 and 251 at 3; 73, 183 and 596 at 4. λ = 0 (540 is clean and holds high units), so
  Corollary L4′ gives only the trivial bound here. Thirteen of the 14 high units have members with different holder
  rows (2 to 6 distinct rows); no clean grip holds a proper part of any unit, so no single letter splits one.
- **The scan** (`t3p.py scan`; `results/s7304-t3p-scan.json`), exhaustive in its family: all 19 pairs of a high unit
  and an own holder, clean helpers that interact with the target, every maximal disjoint cover, all items and every
  lowering item. Outcome per own holder:
  - 522 (units 1132, 1582 and 2559): a round; the best is W3 (ΔΦ = −96), from one cover, {344, 451};
  - 540 (units 1247, 1378 and 1557): clean, with no candidate helper, and F = 111 > D = 58 for every lowering item;
  - 229 (unit 2219) and 336 (unit 2304): no disjoint cover; their only candidate helpers overlap (295 and 347 share 186
    pieces, 277 and 295 share 59);
  - 2, 64, 73, 162, 183, 239, 251, 375 and 596: between 17 and 226 of the target's straddlers have no clean holder.

  No single letter at X lowers Φ: the lowering layer is empty.
- **E13** (`t3p.py plus`; `results/s7304-t3p-plus.json`). No blocked grip has all its straddlers inside one clean cap
  disjoint from its own, so T3P+ has no instance with one unlocking helper at X.
- **DIS-4** (`results/s7304-t3p-scan-w3.json`, `results/s7304-t3p-scan-w5.json`). At the end of W3 (`f93e461f…`,
  Φ = 1498) and at the end of W5 (`956d3d3e…`, Φ = 1472) the scan finds no round and no single letter that lowers Φ.
  - At the end of W3, 522 holds as its foreign load the 70 pieces that (522, 14) lowered (Lemma R) and own units of 22
    pieces (F − D = 48), and no clean grip other than 522 holds any of its obstruction: a self-lock (the only clean
    holder of the foreign load is the target), as at 540. With item 49 instead (W5), the same 70 pieces end in cap 451,
    which the second round unloads.
  - At the end of W5, 522 has F = 22 (the pieces that (522, 15) lowered) and D = 9 (three units of 3 pieces), again with
    no candidate helper. 375 can now be covered (helpers (277, 1) and (522, 1)), but F = 123 > D = 42.
  - The other own holders keep their status at X; 475, a new own holder at the end of W5, has straddlers without a
    clean holder. λ = 0 at both ends (`results/s7304-levels-w3.json`, `results/s7304-levels-w5.json`).
- **DIS-5** (`t3p.py pairs`; `results/s7304-t3p-pairs-229-a.json` and `-b.json`). All 7,200 two-letter words at 295
  and 347 (both orders, all 60 items at each, identities included; the second letter's legality, F and D recomputed
  after the first): 6,900 are admissible and 1,380 tame, every tame word keeps Φ = 1594, and 23 leave 229 clean, each
  of them with the letter (347, 7). Among those 23 the least F − D at 229 is 25, with (295, 6), (347, 7) (F = 34,
  D = 9); among all tame words it is 9, and D at 229 is at most 9 after every one. So no tame word of two letters at
  295 and 347 followed by a lowering at 229 lowers Φ: a negative for this family of two grips and two letters only.
- **Reported** (computed during the attack by the analysts' own programs):
  - Ψ_522 = Φ + 2F_522 along W5 is 1796, 1776, 1638, 1638, 1516, 1516, constant across both letters at 522 (Lemma R);
    the descent made at 522, 96 + 26 = 122, equals 2(22 − 101) + 2·140 with the unloading U = 10 + 69 + 61 = 140 by the
    three helper letters (Corollary U) (Claude Opus, checked by Astra).
  - (347, 12) keeps Ψ_347 = 1816; F_347 goes from 111 to 0; D_347 is 0 for every item before, and 111 afterwards for
    each of the twelve items of A4·45 (45 = 12⁻¹); Φ goes from 1594 to 1816; J_1 goes from 59,864 to 59,753, and
    Φ + 2J_1 = 121,322 before and after (Lemma P) (both).
  - At 540 (Question 2): its foreign load is one unit of 111 pieces of branch 1 and height 2, whose seven holders 457,
    502, 514, 537, 540, 549 and 552 have levels 2, 3, 2, 1, 0, 2 and 3. After each of the twelve lowering items Φ is
    1700; L8 never applies (540 is the only class-0 holder of the raised pieces), and L9 only with item 12 at 582,
    where F_582 = 514. The replay of (540, 12), (582, 29), (540, 12⁻¹) gives Φ 1700, 2728, 2754 (Claude Opus; the
    foreign unit, its holders and their levels checked by the integrator on the label table and
    `results/s7304-levels.json`).
  - Of the clean grips, 15 pairs interact and have disjoint inside sets; for five of them all 118 letters of each
    were applied, and no image touched the partner's cap, so no precedence edge arises there (Claude Opus; the 15
    pairs checked by the integrator on the label table and the locality graph).

### 5.4 Corrections

Applied to card 6 and to sections 2 and 4 of this page:
- tame is per letter (ΔΦ ≤ 0 at every letter), not per prefix (both analysts);
- the lowering set of a unit is the coset A4·h₀ (h₀ first), not h₀·A4: at 344, A4·12 = {12, 16, 20, 24, …, 32} and
  12·A4 = {12, …, 23} (both);
- Lemma C needs equal holder sets per member, not for the unit's intersection (Astra); 13 of the 14 high units at the
  7304 stop have members with different holder sets;
- the new obstruction of the landing letters at 451 is not made of lattice pieces: (451, 43) adds 26 branch-1 pieces of
  height 2 that move from frame to frame, and (451, 55) adds 22 such pieces (both);
- the descent at 522 clears the obstruction rather than landing it (Claude Opus);
- the label table takes 2 to 4 CPU minutes, not 15 (both; measured 120.6 s and about 125 s);
- the field `cell` of `jamcheck.py` lists the own-class grips that hold the selected obstruction pieces; it is not a
  set of pieces (both).

Found in the sealed analyses by the cross-examination:
- Claude Opus named the state after four letters of W5 (`32ae273f…`) as the stop of its order O2; (522, 15) lowers Φ
  there. The end of W5 is `956d3d3e…` (Astra).
- Claude Opus's counts at 336 were the contribution of the letter's own cap; after (277, 6) the whole counts are 37
  straddlers and 815 foreign pieces (Astra).
- Claude Opus's search rule (minimize F alone) is replaced by Corollary B (Astra).
- Claude Opus's L8 required D > F at the first letter, which excludes the raising first letters it was meant for
  (Astra).
- Astra's first answer to Question 1.4 (no own holder satisfies her peeling certificate) holds for that certificate
  only; W3 is a tame descent (Claude Opus).

### 5.5 The disputed points and how they were decided

The cross-examination left five disputed points (findings DIS-1 to DIS-5 of call `20261024T131110Z-b454d61a`) and a
proposed experiment, E13. The first three were settled in the exchange by changing the statements; the other three by
the exact runs of section 5.3, as stated in the implementation call's packet.

| Point | Question | Decided by | Outcome |
| --- | --- | --- | --- |
| DIS-1 | Is "no moved piece ends in the obstruction" the same test as "the letter adds nothing to the obstruction"? | the exchange | No. Two predicates, safe and strictly safe (S1); strictly safe implies safe, not conversely. |
| DIS-2 | Do helper letters certified at a common state compose in sequence? | the exchange | Not without more. The precedence digraph (P1) and Lemma A prove the sequential execution; chained rounds are checked by one replay (Corollary I). |
| DIS-3 | May a unit be treated through the holder set common to its members? | the exchange | No. Lemma C and the corollary of L4 are stated per member. Computed: 13 of the 14 high units have members with different holder rows; no clean grip holds a proper part of any unit. |
| DIS-4 | Is there a round of T3P with clean helpers at the end of W5 (`956d3d3e…`), and at the end of W3 (`f93e461f…`)? | `t3p.py scan` at both ends | No, at either end, and no single letter lowers Φ there. This is a negative for that family only. |
| DIS-5 | Over all two-letter words at 295 and 347 (both orders, identities included, prefix-aware), what is the least F − D at 229 among the words that leave it clean? | `t3p.py pairs` over all items | 25, with (295, 6), (347, 7): F = 34, D = 9. Of the 7,200 words 23 leave 229 clean, and D at 229 is at most 9 after every tame word, so none is followed by a lowering that lowers Φ. This is a negative for that family only. |
| E13 | Is there an instance of T3P+ at X? | `t3p.py plus` | None with one unlocking helper: no blocked grip has its straddlers inside one clean cap disjoint from its own. |

### 5.6 Answers to the card's questions

- **1.1 (construction): answered** by Theorem T3P (section 5.1): finite hypotheses at the state, at most 81 letters per
  round with clean helpers (600 with T3P+), rounds chained at most Φ/2 times and checked by one replay. At the 7304
  stop it gives W3, and W5 as two rounds.
- **1.2 (safety): answered.** Safety is decided by the table of S1 (old and new straddler and foreign flags of the moved
  pieces), which must include frame pieces as well as lattice pieces; strict safety is the stronger of the two
  predicates. By S2 the table is K⁺-equivariant, and one image computation gives the new flags of a whole landing
  coset A4_y·g; at deep states the table is not a function of the coset g·A4_y, as it is at the solved state
  (R21F-09). The question's premise was too narrow: by T3P the helpers need not land anything.
- **1.3 (dependency structure): in part.** Sufficient: T3P and T3P+. Obstructions: unlock levels (a lower bound on the
  letters before a grip can be clean, λ + 1 letters before any tame descent, and λ = ∞ a certificate that none exists),
  orphan cells, single grips, and finite plateau certificates as the only complete negative. What survives the tame
  jumbles: Ψ_x under letters at x and Φ + 2J_P under letters at grips of class P. No global invariant rules out
  unlocking.
- **1.4 (the 7304 stop): answered** (section 5.3). 522 satisfies T3P (W3). 540 is clean but self-locked (F = 111 > D =
  58, and no other clean grip holds its foreign load). At 229 and 336 the only candidate helpers overlap, so T3P does
  not apply, and at 229 no tame word of two letters at those two candidates is followed by a lowering that lowers Φ
  (DIS-5: of the 23 words that leave 229 clean the least F − D is 25). At 64 and eight other own holders some
  straddlers have no clean holder (at 64, 41 members of the high unit 1007), so they need T3P+ or a deeper chain.
- **2 (bounded excursions): in part.** L8 (lower, then relower: two letters, excursion at most max(0, 2(F − D))) and L9
  (lower, land, return: three letters, Φ lower by at least 2|G|) are proved as implications. At 540 neither applies
  (reported); no excursion was constructed at the hard own holders.
- **3.1 (top layer): in part.** Proved: L10 (the clean and the pure status of a grip are set by the last letter at it or
  at one of its 80 partners) and P12 (after a jumble that lowered no high piece and moved a lattice piece, its inverse
  is a tame letter that lands that piece). Conjectured:
  C13, a tame landing letter at every reachable configuration off the lattice. The analogue for lowering letters is
  false: no single letter lowers Φ at the 7304 stop.
- **3.2 (C2): the mechanism is answered; the choice of item in part.** A lowering at x turns the pieces it lowers into
  the new foreign load of x and the old foreign load into its only relowering supply, so Ψ_x is constant under letters
  at x and a best lowering exhausts x (Lemma R, Corollaries U and E). This is how the clean pure own holders run out:
  each lowering makes its own cap impure (Lemma T), with a foreign load equal to the number of high pieces it lowered,
  and after a best lowering the cap descends again only after letters at other grips lower that load or raise its
  lowering supply. At the six stops of section 5.7, each of the ten caps that the chain lowered and that now has
  F ≥ D carries as F exactly the pieces of its last lowering. The choice among the lowering items decides where the
  lowered pieces go: at 522, item 49 puts them in cap 451, whose letter (451, 44) unloads them for a second round,
  while after item 14 only 522 holds them. No rule for the choice is proved.
- **3.3 (an order that does not jam): open.** No order is proved not to jam. The greedy order of T3P rounds with clean
  helpers (the best round first) jams at all six D2 ends, after one to six rounds that lower Φ by 6.0% to 56.0%
  (section 5.7); at the 7304 stop it is W3, and W5 jams too (DIS-4).

### 5.7 The six D2 ends (exact)

The same scan and levels at the six height ends of the D2 research (`results/sources.json`: `s7302-last`,
`s7303-end`, `s7304-end`, `s7305-end`, `s7306-end` and `s7307-end`), then a greedy chain of rounds at each
(`chain.py`: at each state the scan's best round, executed by `t3p.py check`, until the scan finds no round), then the
scan and the levels at each chain's end. Each chain's word was replayed from its table by `replay_word.py`, which
reached the chain's end digest and Φ, with Φ never rising along the word.

| End | Φ at the D2 end (n4, n6) | Pairs with a round in the first scan | Chain: rounds (letters) | Φ after the chain | Own holders at the chain's end: straddler without a clean holder / F ≥ D / no cover or no items |
| --- | --- | --- | --- | --- | --- |
| 7302 | 1468 (734, 0) | 3 of 35, all at 171 | 2 (4) | 1376 (−6.3%) | 9 / 4 / 4 of 17 |
| 7303 | 2206 (929, 87) | 13 of 34, at 69, 140 and 282 | 3 (5) | 1680 (−23.8%) | 12 / 4 / 3 of 19 |
| 7304 | 1594 (797, 0) | 3 of 19, all at 522 | 1 (3): W3 | 1498 (−6.0%) | 10 / 3 / 2 of 15 |
| 7305 | 2838 (1419, 0) | 9 of 67, at 44, 281 and 520 | 5 (9) | 2474 (−12.8%) | 22 / 10 / 3 of 35 |
| 7306 | 3910 (1955, 0) | 11 of 52, at 252, 336 and 561 | 3 (6) | 2684 (−31.4%) | 18 / 5 / 2 of 25 |
| 7307 | 3170 (1585, 0) | 15 of 41, at 102, 266, 435, 516 and 543 | 6 (13) | 1396 (−56.0%) | 13 / 6 / 1 of 20 |

A pair is a high unit and one of its own holders. An own holder at a chain's end is counted once, under the best
status of its units in the order round, F ≥ D, no feasible items, no disjoint cover, straddler without a clean holder.

- T3P with clean helpers has rounds at every end. Chained greedily, it lowers Φ by 6.0% to 56.0% in one to six
  rounds; then, at every end, the scan finds no round and no single letter lowers Φ. λ = 0 at every end, before and
  after its chain, so Corollary L4′ gives only the trivial bound.
- At the six stops, of the 131 own holders of high units, 84 have straddlers that no clean grip holds, 32 have F ≥ D
  for every feasible cover, and 15 have no disjoint cover or no feasible items. Ten of the 32 are caps that the chain
  lowered, and at each of them F equals the number of pieces of the chain's last lowering there, as Lemma R says (the
  lowering turns its pieces into the cap's foreign load). At 171 of 7302, for example, (F, D) is (21, 59) before the
  second round and (59, 21) at the stop; at 69 of 7303 it is (103, 113) before the third round and (113, 103) at the
  stop.
- At 7307 the D2 rule stopped although single letters lower Φ. Each of the twelve items of one lowering coset at 516
  lowers 395 pieces and raises 7, which takes Φ from 3170 to 2394 (for item 14, `results/s7307-516-14-replay.json`:
  n4 1585 → 1183, n6 0 → 7). The rule's measure Q = 177121 n6 + n4 rises under such a letter, so the rule refuses it.
  The chain's first round uses (516, 14) as a helper, and seven pieces are still at height 6 at the chain's end.

## 6. Reading

- The jam is not a lack of moves in general (there are always clean grips: the last twisted grip is one, Lemma T). It
  is the state of the grips that a lowering needs. The high units still left are buried under layers of other classes'
  frames, in their own holders' caps (foreign load) and across their cuts (straddlers).
- Retained twists cannot remove those layers: they only move blocks and frames between cuts and caps (Corollary L1),
  and the measured preparations stop above zero.
- A layer of height-2 frames is invisible to Φ, and so are jumbles of that layer's class at pure grips (Lemma Φ). The
  joint attack showed that such letters do not have to land the layer: at 522 two of them, at two caps that do not
  interact, move it out of the way, and the lowering then needs only F < D (Theorem T3P).
- Where T3P does not reach (section 5.3), the obstacle is concrete: helpers whose caps overlap (229 and 336), a
  straddler that no clean grip holds (nine own holders), or a foreign load whose only clean holder is the target itself
  (540 at the 7304 stop, 522 after its lowering). Each lowering turns the pieces it lowers into the foreign load of its
  own cap (Lemma R); at 522 the lowering item decided whether a clean grip other than 522 holds them (item 49) or not
  (item 14).
- At the six D2 ends the construction does real work and then stops in the same way (section 5.7). About two thirds
  of the own holders left have straddlers that no clean grip holds, and most of the others lack supply (F ≥ D, a third
  of them because the cap holds what it lowered) or a disjoint cover with feasible items. What would continue lies
  outside the scanned family: blocked helpers freed first (T3P+ and longer chains), overlapping or repeated helpers,
  and words that unload a cap or raise its supply.
- The measure matters: the D2 rule weighs pieces of height 6 first and so refused, at 7307, single letters that lower
  Φ by a quarter. The theorems here use Φ, under which such letters are descents.

## 7. Reproduction

From the repository root, with one thread per process (`OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1`),
`S=research/jumbling/restoration/14-blocking-structure/scripts`, `T` a table from `results/sources.json` and `D` a
directory outside the repository:

    python3 -I -B $S/blockmap.py $T $D/state.json --labels $D/state.npz --name <name>
    python3 -I -B $S/jamcheck.py $T $D/state.npz $D/jam.json
    python3 -I -B $S/jamcheck.py $T $D/state.npz $D/moves-522.json --caps 522 --moves
    python3 -I -B $S/retained_search.py $T $D/state.npz $D/retained.json --depth 6 --beam 6 --seconds 150
    python3 -I -B $S/retained_search.py $T $D/state.npz $D/retained-deep.json --depth 14 --beam 30 --seconds 600 --groups 1557,1132,2234
    python3 -I -B $S/retained_search.py $T $D/state.npz $D/retained-check.json --check $D/retained.json
    python3 -I -B $S/coheld.py $T $D/state.npz $D/coheld.json
    python3 -I -B $S/peel.py $T $D/state.npz 522 451 344 $D/peel-522.json
    python3 -I -B $S/replay_word.py $T 451:55,344:9,522:14 $D/peel-522-replay.json

The first command writes the label cache that the others read. `results/states.json` holds the `summary` and `high`
blocks of the first command's output for each state, and `results/s7304-end-caps.json` its per-grip rows at the 7304
stop; the other files in `results/` are the outputs of the other commands at the 7304 stop
(`d2-s7304/component/tables/00005.json`). Label caches stay outside the repository.

The joint attack's runs (section 5.3), with the same `T`, `R=research/jumbling/restoration/14-blocking-structure/results`,
`W3=344:9,451:55,522:14` and `W5=344:9,451:55,522:49,451:44,522:15`:

    python3 -I -B $S/replay_word.py $T $W5 $D/w5-replay.json
    python3 -I -B $S/locality.py $D/locality.json
    python3 -I -B $S/levels.py $T $D/levels.json --labels $D/state.npz
    python3 -I -B $S/t3p.py check $T $R/s7304-w3-certificate.json $D/check-w3.json --labels $D/state.npz
    python3 -I -B $S/t3p.py check $T $R/s7304-w5-certificate.json $D/check-w5.json --labels $D/state.npz
    python3 -I -B $S/t3p.py scan $T $D/scan.json --labels $D/state.npz
    python3 -I -B $S/t3p.py plus $T $D/plus.json --labels $D/state.npz
    python3 -I -B $S/t3p.py pairs $T 229 295 347 $D/pairs-229-a.json --labels $D/state.npz --items-y $(seq -s, 0 29)
    python3 -I -B $S/t3p.py pairs $T 229 295 347 $D/pairs-229-b.json --labels $D/state.npz --items-y $(seq -s, 30 59)
    python3 -I -B $S/t3p.py scan $T $D/scan-w3.json --word $W3 --labels $D/w3.npz
    python3 -I -B $S/levels.py $T $D/levels-w3.json --word $W3 --labels $D/w3.npz
    python3 -I -B $S/t3p.py scan $T $D/scan-w5.json --word $W5 --labels $D/w5.npz
    python3 -I -B $S/levels.py $T $D/levels-w5.json --word $W5 --labels $D/w5.npz
    python3 -I -B $S/test_t3p.py

A `--word` is replayed before the label cache is read, and a cache whose digest differs is rebuilt, so each end state
gets its own cache. The two `pairs` commands split the first-letter items at 295 (and the second-letter items at 295 in
the other order) between two processes. The outputs are in `results/` under the names `s7304-w5-replay.json`,
`s7304-levels*.json`, `s7304-t3p-check-w3.json`, `s7304-t3p-check-w5.json`, `s7304-t3p-scan*.json` (the scan at X
also writes its best round as `s7304-t3p-scan.certificate.json`), `s7304-t3p-plus.json` and
`s7304-t3p-pairs-229-a.json` and `-b.json`. `results/locality-summary.json` is the output of `locality.py` without
its fields `delta_index`, `delta_values` and `partners` (2.8 MB, recomputed in about 1.5 CPU seconds). The two `pairs`
files keep only the rows of the 690 per-letter-tame words of each half (the other rows carry only admissibility and
the first letter's data); the full outputs (1 MB each) took 55 and 59 CPU minutes. The two certificates are
inputs, written by the integrator from W3 and W5; the unit piece of each round is a member of a unit that its lowering
item lowers.

The six D2 ends (section 5.7), with `T` the table of an end in `results/sources.json` (`s7302-last`, `s7303-end`,
`s7304-end`, `s7305-end`, `s7306-end` and `s7307-end`), `n` its name `s<seed>` and `w` the word that the chain
records:

    python3 -I -B $S/t3p.py scan $T $D/$n-scan.json --labels $D/$n.npz
    python3 -I -B $S/levels.py $T $D/$n-levels.json --labels $D/$n.npz
    python3 -I -B $S/chain.py $PWD $D $n:$T
    python3 -I -B $S/replay_word.py $T $w $D/$n-replay.json
    python3 -I -B $S/t3p.py scan $T $D/$n-end-scan.json --word $w --labels $D/$n-end.npz
    python3 -I -B $S/levels.py $T $D/$n-end-levels.json --word $w --labels $D/$n-end.npz

and at seed 7307 the single letter of section 5.7:

    python3 -I -B $S/replay_word.py $T 516:14 $D/s7307-516-14-replay.json

`chain.py` takes the best round of the scan at each state (by the scan's tie-break), executes it with `t3p.py check`,
which tests P0 to P4 and replays the round exactly, and repeats until the scan finds no round; it reads and writes the
label cache `$D/$n.npz` and writes `$D/$n-chain.json`. The outputs are in `results/` under the names
`s<seed>-t3p-scan.json` (with `.certificate.json`), `s<seed>-levels.json`, `s<seed>-t3p-chain.json`,
`s<seed>-t3p-chain-replay.json`, `s<seed>-t3p-chain-end-scan.json`, `s<seed>-chain-end-levels.json` and
`s7307-516-14-replay.json`; at seed 7304 the first scan and levels are those of section 5.3.
