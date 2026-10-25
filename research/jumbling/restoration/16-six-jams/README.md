# Stage 16: the six jams (25 October 2026)

Status: **research; the joint attack on card 8 is done (section 2).** Card 8 is the open part of card 7, Question 4:
the six configurations where two-level P* stopped for good ([stage 15, section 2.5](../15-lowering-machine/README.md#25-runs)),
which the owner put before the tail completion and the bulk on 25 October 2026
([decision](../../../../docs/wiki/decisions/owner-decisions-2026-10-25-jams-first.md)). This stage asks what lowers Φ
there, and how far a procedure that knows it gets. The problem card is [card 8](../../theory/cards/card-8-six-jams.md).

Labels: **proved** (with the proof in [`PROOFS.md`](PROOFS.md)), **computed** (exact, by the program named; section 3
says which runs used the integrator's reference programs), **reported** (computed during the attack by an analyst's or
the integrator's own program, which is not committed; "both" when both analysts computed it independently; the outputs
of the integrator's are in `results/` as captured records), **lead**, **open**. Everything
is headless Linux research code on exact geometry; nothing here is Windows, DirectX, input or performance evidence.

## 1. Terms

The terms of stages 14 and 15 and of card 8 hold: labels, clean and blocked grips, heights, branches and second
vertices, Φ, F_x and D_x(h), units, own holders, partners, levels, words, excursion, tame words, macros, supports,
re-aims, enablers, release structures, positive-budget blocked holders and load jams. The **jams** are the six
configurations given exactly in card 8 (its table "The jams": the D2 end table, then the chain word and the macros of
the declared and the two-level runs of P*); `jams.json` holds the same data for the scripts. New here:

- A **dynamic target** is the grip of a word's final lowering when it is not fixed in advance; it need not be an own
  holder, or hold any high piece, at the start of the word. Family R of P* fixes its targets to the own holders at the
  start of its search.
- A **transport word** (family X) is a word of K retained letters followed by a lowering at a dynamic target
  (Theorem X). Its letters take roles that are read off the path products: a **carrier** moves a high unit or a
  straddler cell, an **evacuator** moves the foreign load out of the landing cap, a **router** moves pieces that a
  carrier would otherwise put across a grip, and a **releaser** acts at a grip that has just become clean.
- A **routing word** releases a blocked grip r whose straddlers all lie in one clean cap c (the **carrier**) by letters
  at c and at **helpers**. The carrier's **positions** are the products p_j of its items so far, and c′ is its total
  item; the **obligated set** B_g of an item g at c is the set of pieces of I_c that g would put across r. A
  **bracket** (c, p) Y (c, p)⁻¹ is a helper word conjugated by a carrier position.
- A **T1 word** interleaves leaf jumbles (raises), cashing jumbles (lowerings), returns at the leaves and retained
  letters (Schema T1). A **T2 word** is a conjugate u v u⁻¹ (Lemma CT).
- **7302′** is the 7302 jam followed by `597:8,590:14` (Φ 494, digest `bc73c984…`), and **7307′** the 7307 jam followed
  by `382:1,516:9,494:13` and `237:3,319:4,369:10,387:14` (Φ 1114, digest `4f651cdd…`): the configurations where P*
  with family X stops after the 7302 and 7307 jams (section 2.5).

## 2. Joint attack (25 October 2026)

Card 8 was attacked as the owner decided on 24 and 25 October: two sealed analyses of the same card, by Astra (call
`20261025T051555Z-01b8d182`) and by Claude Opus (a subagent of the integrator's session); the integrator's certification
of every descent that either analysis reported, with the integrator's own exact searches; a cross-examination, in which
each analyst answered every finding of the other and the integrator's points (Astra's: call
`20261025T075401Z-fe5d4292`); then exact experiments for the points still in dispute, run by the integrator, and
repository versions of the integrator's search programs, written by Sol (implementation call
`20261025T100755Z-e32c3028`) and reviewed by the integrator. The analyses are private review records; this section records what
they established. The proofs are in [`PROOFS.md`](PROOFS.md); each was checked line by line by the analyst who did not
write it and by the integrator.

### 2.1 The results (proved; `PROOFS.md`)

- **Retained transport (Theorem X, Lemma Z).** A word of retained letters followed by a lowering is decided on the
  label table at the start: every piece carries a path product π_A, and admissibility, labels, heights, branches,
  second vertices and the final F and D at any grip are per-piece reads at the start (Lemma RS and Corollary CR
  composed). The final target is free. Four exact reductions (merging letters at one grip, commuting letters at grips
  that do not interact, minimality, dynamic targets) keep a representative of every shortest descent; the commuting
  rule keeps at least one representative of every trace but is not a normal form (Astra's bca/cab). A descent with
  excursion 0 exists exactly when a tame descent does (Lemma Z), so the transport family with Φ-neutral jumbles in its
  prefix (X⁺) is complete for E* = 0.
- **Release accounting (Lemma B, Lemma RF, consumption).** With B = D − F at the target, a release u followed by the
  lowering descends exactly when 2B(uX) > Δ_u, and B changes only through the pieces each letter moves. F < D after a
  raising release is not enough, and spatial conditions do not keep B, since a letter can change a branch or a second
  vertex in place: B never falls when no letter raises a foreign indicator or clears a lowering indicator. A return at
  a leaf refunds its raise exactly when the leaf's cap is back in the state the leaf left. A piece that a leaf raises
  and the target lowers nets 0; that a descent needs supply the target owned before the leaf is not proved.
- **Routing (Lemmas OT and RR, the conjugated-touch form).** If a clean carrier c holds every straddler of r, every
  piece of B_{c′} must be moved by a helper at some position of its orbit under c, at a grip clean at that moment
  (Lemma OT). The **cover test** turns this into a finite table test for the helpers that are clean in the carrier-only
  states; a helper made clean only by another helper is outside it. A retained routing word that releases r, followed
  by letters at the released grip and a lowering, is a tame macro whose hypotheses are table tests at the start
  (Lemma RR). Splitting each carrier letter writes a routing word as brackets (c, p_j) Y_j (c, p_j)⁻¹ followed by the
  carrier's total item.
- **Conjugated release (Lemma CT).** u v u⁻¹ returns every piece that v does not move to its pose at the start, when no
  piece moved by v straddles the grip of an inverse letter when it acts (a dynamic condition, checked before every
  inverse step). If moreover the contribution of the moved pieces to Φ is unchanged by every letter of u and of u⁻¹,
  then ΔΦ = ΔΦ(v) and E = max(E(u), d_u + E(v), d_v + E(u)); the third term is needed (at 7306 an exact conjugate has
  E 448 where the two-term formula gives 292; `results/certificates/T2-cx-trace.json`). Lemma CJ of stage 15 stays as
  a stronger special case.
- **Schema T1.** Words that raise at leaves, release, lower at one or more targets and optionally return: ΔΦ is the sum
  of the event changes 2(F − D), each read when it acts, and the excursion is the largest chronological prefix sum.
- **Negatives for stated alphabets (Lemma IC, N5, schema bounds).** Pairwise non-interacting clean caps whose items all
  have ΔΦ ≥ 0, and blocked grips that keep a straddler under every neutral product, give a negative certificate in the
  sense of Proposition BN with at most ∏|N_c| configurations (Lemma IC); its instances at four jams bound only words
  over their alphabets. At 7306 lev(307) = 4, so no word of at most three letters releases 307 and a lowering at 307
  needs at least five letters, whatever the alphabet or the excursion (N5). The schema bounds 748 (7305) and 256 (7306)
  hold for one leaf schema and one order of raises and refunds only.
- **Termination (Proposition P).** As in stage 15: a state-only procedure that commits certified macros that lower Φ,
  with excursion at most E₀, ends after at most Φ₀/2 macros. Its families, E₀ and choice key are declarations.

### 2.2 The descents (exact)

Every word below starts at a jam. Each was certified with `15-lowering-machine/scripts/macro.py cert` (Theorem MC)
from its D2 end table with the jam's word as the prefix and the excursion allowance set to its excursion, and every
certificate passed `macro.py check --replay`, which replays the whole word independently and recomputes every height
after every letter (computed, `results/certificates/<row>.json` and `<row>-check.json`).

Tame descents (excursion 0):

| Row | Jam | Word after the jam | Shape | Φ profile | F, D | End digest | Found by |
| --- | --- | --- | --- | --- | --- | --- | --- |
| SJ-D1 | 7302 | `597:8,590:14` | transport, K = 1 | 498, 498, 494 | 0, 2 | `bc73c984…` | Astra |
| SJ-D2 | 7302 | `597:9,523:14` | transport, K = 1 | 498, 498, 494 | 0, 2 | `8dd53a1b…` | Astra |
| SJ-D3 | 7302 | `597:11,591:14` | transport, K = 1 | 498, 498, 494 | 0, 2 | `5af2f798…` | Astra |
| I-7303 | 7303 | `245:2,272:2,245:1,365:14` | transport, K = 3: carrier, debris exchange, carrier | 936 (4×), 908 | 184, 198 | `42fdbf20…` | integrator |
| O-SJ-D9 | 7304 | `23:10,20:2,112:2,23:5,20:1,23:7,41:1,2:14` | routing release of 41 (carrier 23, helpers 20 and 112), releaser 41:1 | 1172 (8×), 554 | 0, 309 | `7ef6e3a2…` | Claude Opus |
| I-7304 | 7304 | `23:10,20:2,112:2,23:11,112:2,23:6,20:1,23:7,41:1,2:14` | O-SJ-D9 with one more bracket | 1172 (10×), 554 | 0, 309 | `0e245c04…` | integrator |
| SJ-D4 | 7305 | `226:2,175:12` | transport, K = 1 | 1572, 1572, 1566 | 0, 3 | `02522215…` | Astra |
| SJ-D5 | 7305 | `436:2,268:14` | transport, K = 1 | 1572, 1572, 1544 | 0, 14 | `08bf8d80…` | Astra |
| O-SJ-D8 | 7305 | `94:1,134:1,421:8,499:1,421:5,305:2,389:7,504:12`, then `504:1,431:3,576:13` | routing releases of 305 (carrier 134, router 94) and 389 (carrier 421, router 499), releasers 305:2 and 389:7; then a re-aim at 504 and the evacuator 431:3 | 1572 (8×), 1332, then 1332 (3×), 764 | 0, 120; 111, 395 | `e1e1d9a6…`, `8d0e1038…` | Claude Opus |
| I-7306 | 7306 | `351:3,384:1,485:1,526:14` | transport, K = 3: carrier, evacuator, carrier | 1490 (4×), 1482 | 0, 4 | `69c6f16b…` | integrator |
| O-SJ-D7 | 7306 | `561:1,598:4,233:6,536:11,370:15` | router 561, release of 536 by the carrier 598, evacuator 233:6, releaser 536:11 | 1490 (5×), 1268 | 0, 111 | `06ec0938…` | Claude Opus |
| I-7307 | 7307 | `382:1,516:9,494:13` | transport, K = 2: evacuator, carrier of the height-6 units | 1220 (3×), 1206 | 0, 7 | `280c80e2…` | integrator |

Descents with an excursion (T1 and T2 words):

| Row | Jam | Word after the jam | Shape | Φ profile | E | End digest |
| --- | --- | --- | --- | --- | --- | --- |
| O-D1 | 7305 | `134:12,305:2,421:51,389:6,504:16,431:3,576:13` | T1: leaves 134 and 421, cashings at 504 and 576, no return | 1572, 1990, 1990, 2410, 2410, 2136, 2136, 1568 | 838 | `c93f6c3a…` |
| O-D3 | 7305 | `134:12,305:2,421:51,389:6,504:28,389:7,421:12,305:4,134:14,431:3,576:13` | T1 with returns at both leaves | 1572, 1990, 1990, 2410, 2410, 2136, 2136, 1750, 1750, 1332, 1332, 764 | 838 | `e1dd076d…` |
| O-D4 | 7305 | `94:1`, then O-D3 | relay at 134, then O-D3 | 1572, 1572, 1900, 1900, 2320, 2320, 2046, 2046, 1660, 1660, 1332, 1332, 764 | 748 | `851f972f…` |
| O-D5 | 7306 | `598:20,536:11,233:6,370:15,370:4,536:11,598:14` | T1: leaf 598, release of 536, lowering and re-aim at 370, return | 1490, 1782, 1782, 1782, 1560, 1560, 1560, 1268 | 292 | `3c77cf21…` |
| O-D5c | 7306 | O-D5 with the last letter `598:50` | T2: u = (598, 20)(536, 11), v = (233, 6)(370, 15)(370, 4) | as O-D5 | 292 | `e93f1b85…` |
| O-D6 | 7306 | `582:1`, then O-D5 | relay at 598, then O-D5 | 1490, 1490, 1746, 1746, 1746, 1524, 1524, 1524, 1268 | 256 | `89826e46…` |

O-D1 to O-D6 are the second analyst's sealed descents; the integrator certified them with the reported allowances.
The traces `results/certificates/I-7304-trace.json` and `I-7306-trace.json` give S, F and max D at the grips of
I-7304 and I-7306 after every letter.

- **Every jam has a tame descent, so E\*(X) = 0 at all six.** The sealed answers found tame descents at 7302 and 7305
  (transport of a supply to a new target); the integrator's transport search found them at 7303, 7306 and 7307 and the
  routing release I-7304 at 7304; the cross-examination found the routing releases that lower the J4 holders 2, 504,
  576 and 370 tamely.
- **The tame releases reach the end of the excursion descents.** At 7305, O-SJ-D8 reaches Φ 764 in two tame macros,
  the end of O-D3 and O-D4 (excursions 838 and 748); at 7306, O-SJ-D7 reaches 1268, the end of O-D5, O-D5c and O-D6
  (292 and 256). At 7304, O-SJ-D9 lowers Φ by 618, the largest single descent found at the jams.
- **O-SJ-D9 is I-7304 reduced.** It is (23,10)[20:2, 112:2](23,10)⁻¹ · (23,9)[20:1](23,9)⁻¹ · (23,1) · 41:1 · 2:14 with
  the merged carrier letters 23:10, 23:5 (5 = 9·10⁻¹ at 23) and 23:7 (7 = 1·9⁻¹); I-7304 has one more bracket,
  (23,1)[112:2](23,1)⁻¹, whose helper touches no obligated piece. Each remaining helper is needed: without 20:2 the
  letter 23:7 is not admissible, and without the first 112:2 or 20:1, 3 straddlers stay at 41 (Claude Opus, reported).
  The helper 112 is blocked at the jam (S 19, F 178) and is made clean by the carrier's first letter 23:10 (computed,
  `I-7304-trace.json`).
- **A release need not restore the target's cap.** After I-7304's routing prefix, cap 2 has the counts of the jam
  (S 101, F 168, max D 309), but 916 labels in column 2 have changed and 712 pieces that were in or across cap 2 at the
  jam have other poses (both, reported); the prize is read after the release, not at the jam.
- **None of these words is in P*'s families.** R fixes its targets (the transports lower at 590, 523, 591, 365, 175,
  268, 526 and 494, none of them an own holder at the start) and stops at three retained letters (the routing releases
  have four to nine), and M, E, T and L have no such shapes.

### 2.3 Facts at the jams

- **Transport (computed, `scripts/transport.py`; `results/transport/`).** Family X with the reductions R1 to R4 at the
  jams (K = 3 with the minimality reduction, used only after K ≤ 2 found nothing):

  | Jam | K = 1 | K = 2 | K = 3 |
  | --- | --- | --- | --- |
  | 7302 | 36 descending words at 3 targets (590, 523, 591), best Φ 494 | | |
  | 7303 | none | none (53,584 leaf tests for K ≤ 2) | 204 descending words, one target (365), best 908 (179,411 leaf tests, 443 CPU s) |
  | 7304 | none | none (208,762 leaf tests for K ≤ 2) | none (274,381 leaf tests, 1,013 CPU s) |
  | 7305 | 24 descending words at 2 targets (268, 175), best 1544 | | |
  | 7306 | none | none (208,741 leaf tests for K ≤ 2) | 132 descending words, one target (526), best 1482 (126,594 leaf tests, 434 CPU s) |
  | 7307 | none | 348 descending words at 7 targets, best 1206 (369,040 leaf tests for K = 2) | |

  The reductions were checked against the unreduced enumeration at 7307 with K = 2 (678,231 leaf tests and 612
  descending words without R1 to R3, 369,040 and 348 with them, 7,085 and 348 with minimality as well): the best word
  per target is the same in all three (computed, `results/transport/k2raw-7307.json`, `k2-7307.json`, `k2m-7307.json`).
- **One Φ-neutral helper letter (reported, the integrator's scan; `results/transport/n2-730N.json`).** Every letter
  with ΔΦ = 0 at a clean grip (the eleven retained items, and every jumble with F = D, which includes the 48 jumbles at
  each pure clean grip of J6), followed by every lowering at a partner that is clean after it: descending words only at
  7302 (36) and 7305 (24), all of them transports of the K = 1 rows above. So one Φ-neutral jumble adds no two-letter
  descent at any jam. At 7304, helper letters that raise Φ by at most 800 give no two-letter word that lowers Φ.
- **The positive-budget blocked holders of J4.**
  - 2 at 7304: lowered tamely by O-SJ-D9 and I-7304 (Φ 1172 → 554).
  - 504 and 576 at 7305: lowered tamely by O-SJ-D8 (1572 → 1332 → 764), and with excursions by O-D1, O-D3 and O-D4.
  - 370 at 7306: lowered tamely by O-SJ-D7 (1490 → 1268), and with excursions by O-D5, O-D5c and O-D6.
  - 307 at 7306: not released. Its level is 4 (N5), and the six blocked grips whose prize letter would make 307 a
    target (407, 426, 431, 464, 466 and 509, each a lowering of 218 if it were clean) have straddlers without a clean
    holder (computed, `results/routing/rrpE-7306.json`).
  - 517 at 7307: not lowered (the bounded negative O-SJ-N2 below).
- **The obligation at 7304 (computed, `scripts/routing.py`; `results/routing/route-7304-23-41.json`).** At the carrier
  23 and r = 41: |I_23| = 2,825, all 132 straddlers of 41 lie in cap 23, and |B_g| for g = 0 to 11 is 132, 9, 67, 48,
  25, 49, 53, 25, 43, 14, 77, 17. One carrier letter makes 61, 112 and 184 clean. No sequence of one or two carrier
  positions passes the cover test (least uncovered counts 9 and 3), and 22 of the 1,331 sequences of three pass. So
  every routing release of 41 through 23 whose helpers are clean in the carrier-only states has at least three carrier
  actions; O-SJ-D9 has three. The nine pieces of B_1 (69621, 69622, 71166, 77850, 149023, 149026, 164990, 165621,
  177090) have height 2 and branch 0, and after 23:1 their only clean holder is 23 (both, reported).
- **Routing searches (Claude Opus, reported).** The second analyst's bounded routing search (carrier sequences that pass
  the cover test, helpers at cover opportunities, all retained items), exhaustive for n ≤ 2 carrier actions and at most
  two helpers:

  | Jam | Carrier → r | \|S_r\| | Passing sequences, n = 1 / 2 / 3 | Words, n ≤ 2 | Releases | CPU s |
  | --- | --- | --- | --- | --- | --- | --- |
  | 7305 | 134 → 305 | 45 | 6 / 80 / 1,006 | 13,618 | 3,410 | 66 |
  | 7305 | 421 → 389 | 26 | 0 / 7 / 142 | 528 | 28 | 4 |
  | 7306 | 598 → 536 | 3 | 2 / 42 / 632 | 3,080 | 227 | 31 |
  | 7307 | 543 → 597 | 86 | 0 / 17 / 314 | 3,245 | 461 | 22 |

  At 7306 every release gives the same prize at 370 (F − D = −111 after 233:6 and 536:11 in either order).
- **517 at 7307 (O-SJ-N2; Claude Opus, reproduced by the integrator with `scripts/routing.py`,
  `results/routing/route-7307-543-597.json`).** In the language above (carrier 543, n ≤ 2, at most two helpers) 3,245
  routing words release 597 in 461 ways. After each release the tails (448, 3)(597, m), (597, m)(448, 3) and (597, m)
  for m = 1 to 11 clean 517 after 249 of them, and every time with F_517 = max D_517 = 0: the release moves 517's
  supply out of cap 517 or off its second vertex. The second analyst also found no descending lowering at any of the 19
  class-2 grips that interact with 597 or 517 after each release and each item at 597. This is a bounded negative, not
  a certificate in the sense of Proposition BN; the prize (517, 1) → 428 (a lowering of 190 if 517 were clean with its
  present inside set) is a lead.
- **SJ-E1 (Astra; replayed exactly by the integrator, `results/certificates/SJ-E1-trace.json`).** `543:3,448:5,543:7,597:1` at 7307 keeps Φ 1220 and takes
  (S_597, S_517, F_517, max D_517) through (65, 74, 79, 95), (65, 32, 86, 95), (0, 32, 86, 95) and (0, 0, 89, 0)
  (end digest `80bc4ec5f36471fecf0bc6828f8f8f837caeeaed1e9be8579b90a7256c4e5434`): a tame release of 517 that removes
  all of its lowering supply.
- **597 is cleaned at no cost after a relay (Claude Opus; checked by Astra).** At 7307 the relay (448, 3) leaves cap
  543 with F 0 and no high piece, and then (543, k) for k = 29, 30, 31, 38, 45 and 47 keeps Φ at 1220 and leaves 597
  clean. Card 8's +222 for cleaning 597 is the cost at the jam only.
- **Routing releases with a prize (computed, `scripts/release.py`; `results/routing/rrp*.json`).** The integrator's
  search: prize letters (r, m, x) at every blocked grip r, read as if r were clean with its present inside set; for the
  best prize grips of level 1 (with carriers enabled by one retained letter), a beam of 2,000 routing words of up to
  ten letters at the carriers and helpers, and after every release every retained item at r with a lowering at every
  prize target. Hits: at 7304 only (r = 41: 110 releases, every hit to Φ 554, I-7304's family). At 7303 the prize grips
  153 and 307 were searched (30,880 and 42,835 releases, no hit); at 7307, 244 and 363 (no release); the other best
  prize grips at 7303 to 7307 have straddlers without a clean or enabled holder, and so do the eight best of the 13
  prize grips (30 prize letters) at the stop 7307′ (`rrp-7307p.json`, `rrpE-7307p.json`; the stop 7302′ is DIS-A,
  section 2.7). The beam is not a decision procedure:
  at 7304, without relevance pruning it ends with 3 straddlers at 41 and no release over its own alphabet (23 and the
  helpers 20, 61, 112 and 184), and finds 62 releases over the alphabet 23, 20, 112, 61 (the integrator's point I8;
  `results/routing/rrp-7304-*.json`), so its negatives decide nothing.
- **T1 with one leaf at the routing grips (computed, `scripts/t1.py`; `results/t1/t1-7304-23-20.json`,
  `t1-7307-543.json`).** Every leaf jumble with ΔΦ ≤ 900 at 23 and 20 (7304; the carrier and a helper of O-SJ-D9) and
  at 543 (7307; the carrier of the releases of 597), applied exactly, then every lowering at a grip clean after it
  whose cap held a moved piece, and the return at the leaf after the best lowering when that lowering descends: no
  pair and no return lowers Φ. At 7304 the best pair raises Φ by 394; at 7307 the best pair, (543, 16) then (448, 15),
  and the best of the 4 returns keep Φ (the leaf raises it by 222 and the lowering takes 222 back). A leaf jumble at
  these grips does not replace the routing releases.
- **Unit mobility (integrator, reported; `results/mobility/`).** A high unit is carried when some clean grip holds it
  whole, and frozen when no clean grip holds any of its pieces. Frozen high pieces: 48 of 249 at 7302, 149 of 468 at
  7303, 516 of 586 at 7304, 709 of 786 at 7305, 696 of 745 at 7306 and 438 of 603 at 7307. The transports move carried
  units; the routing releases make frozen ones lowerable where they stand.

### 2.4 The procedure P** (as run)

P** is two-level P* (stage 15) with family X in its stop search, started after the tame routing releases where the
cross-examination found them. It runs in segments:

- A segment is P* with the declared options (families T, R, M, E and L; E₀ = 20; R to three retained letters with a
  beam of 64) until it stops.
- At a stop it searches family R without a beam (`pstar2.stop_search`) and family X with K = 1 and 2, and with K = 3
  and the minimality reduction when K ≤ 2 found nothing. The best candidate by P*'s key (terminal Φ, excursion, length,
  word) is the first macro of the next segment, certified again before it is committed.
- Routing releases (family RR) and T1 and T2 words are not searched by the driver. Where a tame routing release was
  certified at a jam (I-7304 at 7304, O-SJ-D8 at 7305 and O-SJ-D7 at 7306), and for the excursion descent O-D3 at 7305
  as one macro, a run starts with it (`--start`): the driver applies the start macros as given, certifies the macros
  it commits, and the whole descent of each run is certified afterwards (section 2.5).
- It stops when R and X (K ≤ 3) have no candidate at a stop: a stop of the declared search, not a negative
  certificate. A run that reaches its CPU limit has the outcome unknown.

It is state-only and commits only certified macros that lower Φ with excursion at most 20 (X words are tame, and P*'s
families have E₀ = 20), so it ends after at most Φ/2 macros (Proposition P). A run with a start is P** after that
start; the run after O-D3 starts with an excursion descent.

### 2.5 Runs

All runs are headless on Linux, one thread per process, with the integrator's reference version of
`scripts/pxrun.py` (section 3; the commands are in section 5).
`results/runs/` keeps the record of each run (its start, the macros of each segment with Φ after them, and per stop
the counts of the stop search and its choice); the label caches and the per-step records stay outside the repository.

| Run | Start | Φ after each committed macro (family) | Outcome | CPU s |
| --- | --- | --- | --- | --- |
| 7302 | the jam, Φ 498 | 494 (X1) | stop at 494 (7302′) | 2,499 |
| 7303 | the jam, 936 | 908 (X3), 540 (M), 536 (X3) | stop at 536 | 2,477 |
| 7304 | I-7304, 1172 → 554 | (none) | stop at 554 | 1,744 |
| 7305 | the jam, 1572 | 1544 (X1), 1538 (X1) | stop at 1538 | 2,353 |
| 7305 | O-SJ-D8, 1572 → 1332 → 764 | 736 (X1), 730 (X1) | stop at 730 | 1,649 |
| 7305 | O-D3 (E 838), 1572 → 764 | 672 (X2), 542 (T), 514 (X1), 508 (X1) | stop at 508 | 2,435 |
| 7306 | the jam, 1490 | 1482 (X3) | stop at 1482 | 1,838 |
| 7306 | O-SJ-D7, 1490 → 1268 | 1260 (X3) | stop at 1260 | 1,824 |
| 7307 | the jam, 1220 | 1206 (X2), 1114 (X3) | stop at 1114 (7307′) | 4,183 |

A run from a start applies its start macros as given (each was certified at the jam, section 2.2); the driver
certifies only the macros it commits. From the 7304 jam P** would stop at once (P* stopped there, and X has no word
with K ≤ 3, section 2.3), so its run starts with I-7304. The X macros are the transports of section 2.2 where they
start at a jam (SJ-D1, SJ-D5 and SJ-D4, I-7303, I-7306, I-7307), and further transports after them:
`59:8,245:3,365:2,210:12` (to 210) at 7303, `237:3,319:4,369:10,387:14` (to 387) at 7307 and `576:1,574:1,499:12` (to
499) after O-D3. SJ-D5 and SJ-D4 recur after O-SJ-D8 and after O-D3, and I-7306 after O-SJ-D7, each with the drop it
has at the jam (28, 6 and 8). P*'s own families committed two macros: `365:4,508:14,365:13` (family M, 908 → 540)
after I-7303 at 7303, and `574:1,499:14` (family T, 672 → 542) after O-D3. Every other descent after a stop came from
family X.

**The stops.** At every stop, family R without a beam had no candidate, and family X had none with K = 1, 2 and 3
(leaf tests per K; computed, the stop records in `results/runs/`):

| Stop | Φ | Digest | K = 1 | K = 2 | K = 3 (minimal) | CPU s |
| --- | --- | --- | --- | --- | --- | --- |
| 7302′ | 494 | `bc73c984…` | 858 | 202,498 | 447,453 | 1,550 |
| 7303 | 536 | `f3fc8837…` | 312 | 53,053 | 148,505 | 636 |
| 7304 after I-7304 | 554 | `0e245c04…` | 913 | 230,465 | 280,441 | 1,498 |
| 7305 from the jam | 1538 | `6069d068…` | 626 | 104,161 | 107,646 | 898 |
| 7305 after O-SJ-D8 | 730 | `658a7655…` | 266 | 38,058 | 28,305 | 551 |
| 7305 after O-D3 | 508 | `71aa4fbb…` | 286 | 43,634 | 42,844 | 548 |
| 7306 from the jam | 1482 | `69c6f16b…` | 922 | 155,242 | 74,586 | 754 |
| 7306 after O-SJ-D7 | 1260 | `735d7597…` | 856 | 134,034 | 74,349 | 662 |
| 7307′ | 1114 | `4f651cdd…` | 797 | 174,524 | 202,677 | 1,489 |

Each is a stop of the declared search, not a negative certificate (section 2.4). The starts matter: at 7305 the jam's
own transports stop at 1538, the tame releases of O-SJ-D8 lead to 730, and the end of O-D3, which has the same Φ 764
as the end of O-SJ-D8 but other poses, leads to 508; at 7306 the jam's transport stops at 1482 and the release
O-SJ-D7 leads to 1260.

**Certificates.** The whole descent of each run, its start macros followed by its committed macros, was certified
from its jam with `macro.py cert` (excursion allowance 20, and 838 for the run after O-D3) and checked with
`macro.py check --replay` (computed, `results/runs/PX-<run>.json` and `-check.json`; at 7302, at 7304 and at 7306 from
the jam the descent is SJ-D1, I-7304 and I-7306 of section 2.2). All checks passed, and each ends at the digest of its
stop above.

### 2.6 Corrections

Applied to [card 8](../../theory/cards/card-8-six-jams.md), which now reads as attacked with these corrections:

- **The cost of cleaning 597 (Claude Opus; adopted by Astra).** J4's "the jumble (543, 12) does, at ΔΦ = +222" is the
  cost at the jam. After the relay (448, 3), retained items at 543 clean 597 with ΔΦ = 0 (section 2.3).
- **Words at several caps (both).** J4's closing question, whether words at several caps clean these grips tamely, is
  answered: yes for 41, 305, 389, 536 and 597 (O-SJ-D7 to D9, the 461 releases of 597), with lowerings at 2, 504, 576
  and 370; for 517 no lowering follows within the bounds of O-SJ-N2; 307 has level 4.
- **Why R found nothing (both).** The descents at the J4 holders need four to nine retained letters, and the transports
  lower at grips that are not own holders at the start; R stops at three letters and fixes its targets. The sealed
  reasons "305 and 389 can only be cleaned by jumbles at 134 and 421", "536 needs a jumble at 598" and "597 needs a
  jumble at 543" are wrong: tame routing releases exist for all four.

The analysts' own sealed claims and the integrator's cross-examination points were corrected as well:

- **Two released holders (Claude Opus).** The sealed claim that a repair at 7304 needs at least two released holders is
  refuted by O-SJ-D9 and I-7304, which release no grip but 41: their helpers are 20, clean at the jam, and 112, which
  the carrier's first letter makes clean. The sealed counts of bad images at 23 for the items 1, 4, 8, 9 and 11 are 9,
  25, 43, 14 and 17 (Astra), not 9, 14, 36, 14 and 17.
- **The least excursion (both).** The sealed statements "E\* = 0 is open at all six" (Claude Opus) and "E\* = 0 is
  proved only at 7302 and 7305" (Astra) are superseded: every jam has a certified tame descent (section 2.2).
- **The trace normal form (Astra).** The local commuting rule of the integrator's transport search keeps at least one
  representative of every trace but is not a normal form (bca and cab, PROOFS.md section 1).
- **The excursion of a conjugate (Astra).** The sealed formula max(E(u), d_u + E(v)) for u v u⁻¹ misses the term
  d_v + E(u) (section 2.1), and D5c satisfies the per-step Φ condition, not the stated exclusion condition (244 moved
  pieces lie in the forward support at 598 and 101 in the inverse one).
- **The integrator's points.** I6 called the nine pieces of B_1 high pieces of height 4: they have height 2 (both). I6
  said that I-7304 ends with cap 2 as it was at the jam: that holds for the counts only (section 2.2). I2-6 called 430
  a clean own holder at 7302′: it is blocked (S 23), as at the 7302 jam (Claude Opus). The first answer printed the end
  digest of SJ-E1 with two hex digits missing (the full digest is in section 2.3).
- **The stage 14 loader (Claude Opus; adopted).** `14-blocking-structure/scripts/t3p.py: load_input` treats a label
  cache whose digest does not match as a cache miss, recomputes the labels and writes them to the path it was given, so
  a writable input cache can be overwritten. The stage 16 scripts use a loader that never writes a cache it reads
  (section 3); stage 14 is unchanged.
- **Certificates of releases (Claude Opus; adopted as a contract).** `macro.py` certifies descending macros that end
  with a jumble, as Theorem MC's macros are defined; a release such as SJ-E1 is checked letter by letter with
  `macro.apply_checked` instead.

### 2.7 The disputed points and how they were decided

After the cross-examination three points were open. DIS-A and DIS-B are Astra's DIS-1 and DIS-2, DIS-C is the second
analyst's DIS-1.

| Point | Question | Experiment | Outcome |
| --- | --- | --- | --- |
| DIS-A | At 7302′, where P** stops, does a bounded routing, T1, T2 or landing family give a certified descent, or are those families empty there? | At 7302′: family X with K ≤ 3 (P**'s stop search); the prize scan and the release search with a prize, with level-1 and with enabled carriers (`release.py`); T1 with one leaf and E ≤ 900 (`t1.py`, every leaf with ΔΦ ≤ 900, four shards) | **Empty within their bounds.** X has no word with K ≤ 3 (section 2.5). The prize scan finds 12 prize letters at five blocked grips (388, 56, 67, 1 and 91), none of level 1; with carriers enabled by one retained letter, 388 and 56 stay above level 1, the beams at 67 and 1 end with 49 and 120 straddlers and no release, and at 91 the beam finds 24,840 releases and no hit (`results/routing/rrp-7302p.json`, `rrpE-7302p.json`). T1: every jumble at the 39 clean grips has ΔΦ ≤ 900, and the 1,872 leaves, the Φ-neutral landings among them, give 7,489 lowering targets (grips clean after the leaf whose caps held a moved piece) and 108 returns; no pair and no return lowers Φ, and the best of each keeps it (`results/t1/t1-7302p.json`; 10,262 CPU s in four shards). T2 was not searched. Bounded negatives, not a certificate in the sense of Proposition BN |
| DIS-B | At 7304, does no routing release of 41 have two carrier actions at 23, whatever its helpers, and is deciding routing manageable? | Every word with at most two carrier actions at 23 and two helper letters, the second at a grip that is not clean in the carrier-only state at its position (the words outside the cover test with two helpers: y1 y2 c:a, c:a y1 y2 c:b, y1 c:a y2 c:b and c:a y1 c:b y2, with c:a y1 y2 and y1 c:a y2 as prefixes), decided exactly by the label calculus | **Decided for two helper letters:** no word with at most two carrier actions at 23 and at most two helper letters releases 41. The cover test excludes the words whose helpers are clean in the carrier-only states (section 2.3), and the run the 178,002 others (least S_41 6; 179 CPU s; every 997th column reading checked against the letter applied; reported, the integrator's run, `results/dis/twocarrier-7304.json`). With three or more helper letters it is not decided: each helper letter multiplies the words by up to 440 (40 clean grips, eleven items), so enumeration does not decide routing in general, and O-SJ-D9 has three |
| DIS-C | Which choice key and E₀ should the procedure use: P*'s key (terminal Φ first) with E₀ = 20 and the transports (Astra), or a tame-first key with X and RR, and T1 and T2 with E₀ = 900 only where the tame families are empty (Claude Opus)? | (a) T1 with E ≤ 900 at 7302′, where the tame families are empty (as in DIS-A); (b) P** from both Φ-764 states at 7305, the end of O-D3 (`e1dd076d…`, E 838) and the end of O-SJ-D8 (`8d0e1038…`, tame) | **Neither key is shown to avoid jams.** (a) At 7302′, where the tame families are empty, T1 with E ≤ 900 adds nothing (DIS-A). (b) From the end of O-D3, P** reaches 508; from the end of O-SJ-D8, at the same Φ 764 with other poses, 730. The route that goes further starts with an excursion of 838, which neither proposal takes at 7305: E₀ = 20 excludes it, and the tame families are not empty there. Where the procedure ends depends on the route, and no choice rule with a proof is known |

### 2.8 Answers to the card's questions

- **1.1 Releases at the positive-budget blocked holders: answered for four of the six.** Computed: tame words, each
  certified and replayed, lower 2 at 7304 (O-SJ-D9, eight letters, and I-7304, ten; Φ 1172 → 554), 504 and 576 at 7305
  (O-SJ-D8, eight letters and then three; 1572 → 1332 → 764) and 370 at 7306 (O-SJ-D7, five letters; 1490 → 1268); their
  hypotheses are table tests at the jam (Lemma RR, Theorem X), with lengths 5 to 11 and excursion 0. Family R found
  nothing because these words have four to nine retained letters before the lowering, and R stops at three and fixes
  its targets at the start (section 2.6). Open: 307 at 7306 (level 4, so a lowering at 307 needs at least five letters,
  N5; its prize grips have straddlers without a clean holder) and 517 at 7307 (no lowering after any of the 461 releases
  of 597 within the bounds of O-SJ-N2, a bounded negative; SJ-E1 releases 517 tamely but removes all of its supply).
- **1.2 Obligation repair: answered.** Proved: an obligated piece must be touched by a helper at some position of its
  carrier orbit, at a grip clean at that moment (Lemma OT); for helpers clean in the carrier-only states this is a
  finite table test (the cover test); a routing word that passes Lemma RR's hypotheses is a tame macro; split at the
  carrier positions it is a product of brackets (c, p) Y (c, p)⁻¹, conjugates whose untouched pieces return (Lemma CT,
  with the dynamic condition and the three-term excursion). This is the repair the card asked for: helpers move the
  obligated pieces first, inside brackets. Computed at 7304: the 132 straddlers of 41 lie in cap 23; one or two carrier
  positions fail the cover test (at least 9 and 3 obligated pieces uncovered), three pass in 22 of 1,331 sequences, and
  O-SJ-D9 repairs the obligation with three carrier actions and the helpers 20 and 112 (112 made clean by the carrier's
  first letter). No routing release of 41 has at most two carrier actions at 23 and at most two helper letters (the cover
  test and an exact run of the words outside its scope, section 2.7). Open: a bound in the size of the release structure in general.
- **1.3 Accounting: answered.** Proved: the ledger B = D − F at the target, with the exact condition 2B(uX) > Δ_u for a
  release u followed by the lowering, and the monotone condition under which no letter lowers B (no letter raises a
  foreign indicator or clears a lowering indicator; Lemma B); the refund of a return (Lemma RF); a raised piece that the
  target lowers nets 0 (consumption). F < D after a raising release is not enough, and spatial conditions do not keep
  B. Computed: I-7304's routing prefix keeps cap 2's counts but changes 916 labels in its column, so the prize is read
  after the release.
- **2. Load jams: answered by construction.** At 7302 and 7303 the descents are transports: retained words that carry
  supply to a target that is not an own holder at the start (SJ-D1 to SJ-D3, Φ 498 → 494; I-7303, a carrier, a debris
  exchange and a carrier, 936 → 908), decided at the state by Theorem X and found by family X with dynamic targets.
  Relays appear in the excursion descents (94:1 in O-D4, 582:1 in O-D6, each lowering the excursion) and make cleaning
  597 at 7307 free after (448, 3). Landings: one Φ-neutral helper letter followed by a lowering adds no descent at any
  jam (section 2.3). Obstructions: the instances of Lemma IC bound words over their alphabets only, and each alphabet
  misses a helper that a certified descent uses.
- **3. The least excursion: answered.** E\* = 0 at all six jams: every jam has a certified tame descent (section 2.2),
  so the reversed history is not needed. Lower bounds for stated alphabets: no descent at all over {23, 41, 2} at 7304;
  E ≥ 418 over the seven grips of the 7305 instance, 156 at 7306 and 222 at 7307 (Lemma IC); E ≥ 748 at 7305 and 256 at
  7306 for one leaf schema and one order of raises and refunds (attained by O-D4 and O-D6).
- **4. An order that does not jam: answered in part.** Proved: P** is state-only and commits certified macros that lower
  Φ with excursion at most 20, so it ends after at most Φ/2 macros (Proposition P). Computed: with family X at its stops
  and the tame routing releases as starts, P** continues at all six jams and stops at 7302 (494), 7303 (536), 7304 (554,
  after I-7304), 7305 (1538 from the jam, 730 after O-SJ-D8, 508 after O-D3), 7306 (1482 from the jam, 1260 after
  O-SJ-D7) and 7307 (1114). At these stops R and X with K ≤ 3 are empty; these are stops of the declared search, not
  negative certificates. At 7302′ the bounded routing, T1 and landing families are empty as well (DIS-A). The route
  decides how far it gets: at 7305 the end of the excursion descent O-D3 leads to 508 and the end of the tame O-SJ-D8,
  at the same Φ 764, to 730, and neither proposed choice rule takes the route that goes further (DIS-C). Open: a
  negative certificate for all words at any stop, routing and T1 and T2 inside the driver, and a choice rule with a
  proof.

## 3. Tools (`scripts/`)

The searches of section 2 were run with the integrator's reference programs, written during the attack outside the
repository. Sol wrote the repository versions below from them (implementation call `20261025T100755Z-e32c3028`), with the same
algorithms, counters and outputs, and the integrator reviewed them line by line before applying them; the gate checks
them against the reference outputs on seven inputs. They import the stage 14 and stage 15 scripts and change none of
them, and they write only outside the repository.

| Script | What it computes |
| --- | --- |
| `common.py` | The loader: the jam of `jams.json` (table, chain word and macros, checked against its table digest, and against its end digest and Φ when no word follows) followed by an optional word, each letter checked for admissibility, with the stage 14 trees and labels. A label cache is used only when its stored digest equals the state's and its array is valid, and it is never written: the stage 14 loader's overwrite of a mismatched cache (section 2.6) cannot happen. New caches are created only at new paths. |
| `transport.py` | Family X (Theorem X): retained words of exactly K letters at grips clean when they act, in the trace normal form of the reductions R1 and R2, with minimality (R3) as an option, followed by a lowering at every grip clean after the word that holds a high row of its own class (R4, dynamic targets); per target the best word, and the counts of nodes, letters, leaf tests, descending words and skipped words. A function `search` serves `pxrun.py`. |
| `release.py` | Routing releases with a prize: the prize letters (r, m, x) at every blocked grip, read as if r were clean with its present inside set; for the best prize grips of level 1 (or with carriers enabled by one retained letter, `--enable`), a beam of routing words at the carriers and helpers (width 2,000, ten letters; relevance pruning with `--relevant`); after every release, every retained item at r with a lowering at every prize target. A beam: never complete. |
| `t1.py` | Schema T1 with one leaf: every leaf jumble with ΔΦ ≤ E at a clean grip, applied exactly; every lowering at a grip clean after it whose cap held a moved piece; for the best lowering of each leaf that descends after it, the return at the leaf; the pair and return totals, the hits and the excursions. Sharded over the leaves. |
| `routing.py` | The routing anatomy at a carrier c and a blocked grip r (I_c, S_r, the obligated sets B_g, the helpers made clean, the cover test for one, two and three carrier positions) and the bounded routing search (carrier sequences that pass the cover test, helpers at cover opportunities, n ≤ N carrier actions) with the prize test after each release. |
| `pxrun.py` | P** (section 2.4): segments of stage 15's `pstar.run_procedure` with the declared options and, at each stop, `pstar2.stop_search` and family X with K = 1, 2 and, when they find nothing, 3 with minimality; optional start macros. It passes stage 15 only label caches that it has written into its own output directory. |
| `test_six_jams.py` | The gate: the loader (a cache whose digest differs is read and never written, an existing new cache path is refused before any load, a matching cache is used); transport with K = 1 at 7302 and K = 2 at 7307, with and without the minimality reduction; the release search at 7304 (r = 41); T1 at 7304 and at 7307; the plumbing of `pxrun.py` (a start macro, an output-owned label cache, a refused existing output); the routing anatomy at 7304 and the routing search at 7307. The seven search rows are compared with the reference outputs on the same inputs (nine rows, 167 assertions, about 1,600 CPU s; passed on the applied candidate). |

Each record stores the options it was given, the `--labels` path among them, so a record meant for the repository is
made with a label cache outside private directories, or without one. The integrator's other programs (the one
Φ-neutral helper letter, unit mobility and the two-carrier run of DIS-B) are not committed; their outputs are in
`results/` and are labelled reported. Three limits of the tools affect no result above: `release.py --dedup` (which
merges beam words by their labels and branches, not their second vertices or poses) was used by no run and is
unverified; the counter `skipped_minimal` of `transport.py` stays 0, since the minimality reduction removes lowering
targets at the leaves (fewer leaf tests), not words; and in `pxrun.py` the CPU limit bounds the segments but not the
stop search, and a run of 1,000 segments would end without a terminal reason (the runs had at most four segments, a
stop search of at most 1,580 CPU s and at most 4,183 CPU s in all). Commands are in section 5.

## 4. Reading

The six jams were stops of P*'s families, not locks. Every jam has a certified tame descent, so the least excursion
is 0 at all six and the reversed history the card proposed is not needed there. Two mechanisms give them. A transport
carries supply to a target that is not an own holder at the start: retained letters keep Φ and act on one label table,
so the whole word and its final lowering, wherever it lands, are decided at the state (Theorem X). A routing release
takes the straddlers of a blocked grip through one clean cap: the pieces that the carrier would put across the grip
are moved first by helpers, each inside a bracket that returns everything else (Lemmas OT, RR and CT). The descents
with an excursion that the sealed analyses found end where the tame releases end.

The procedure is still not enough. With transports at its stops and the tame releases as starts, P** continues at all
six jams and stops again, at Φ 494 (7302′), 536 (7303), 554 (7304), 508 to 1538 (7305), 1260 and 1482 (7306) and 1114
(7307′): stops of its declared search, not negative certificates. At 7302′ the routing releases with a prize, T1 with
one leaf and E ≤ 900, and the landings among its leaves find nothing either. At 7304 no routing release of 41 has at
most two carrier actions and two helper letters, and each further helper letter multiplies the words to decide by up to
440, so routing is decided by enumeration only in small cases. Where the procedure ends depends on its route: at 7305
the end of the excursion descent O-D3 leads to 508 and the end of the tame O-SJ-D8, at the same Φ, to 730, and neither
choice rule that the analysts proposed takes the better route. The open part of card 8 is now a choice rule with a
proof, routing and conjugated releases inside the driver with a bound in the size of the release structure, the holders
307 and 517, and a negative certificate for all words at one stop. Nothing here proves that P** or any order reaches
depth one, and these headless runs support no Windows, DirectX, input or performance claim.

## 5. Reproduction

From the repository root, with one thread per process (`OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1`),
`S=research/jumbling/restoration/16-six-jams/scripts`, `S15=research/jumbling/restoration/15-lowering-machine/scripts`,
`D` a directory outside the repository, and for a jam `T` its table and `W` its word (the chain word and the macros,
`jams.json`). `--labels` takes a label cache of the state the script loads; the stage 16 scripts only read it, and
compute the labels when it does not match.

    python3 -I -B $S/test_six_jams.py
    python3 -I -B $S/transport.py 7303 1 $D/k1-7303.json
    python3 -I -B $S/transport.py 7303 2 $D/k2-7303.json
    python3 -I -B $S/transport.py 7303 3 $D/k3m-7303.json --minimal
    python3 -I -B $S/release.py 7304 $D/rrp-7304.json
    python3 -I -B $S/release.py 7303 $D/rrp-7303.json --relevant --prizes 8
    python3 -I -B $S/release.py 7303 $D/rrpE-7303.json --relevant --enable --prizes 6
    python3 -I -B $S/release.py 7304 $D/rrp-7304-41.json --only-r 41 --relevant
    python3 -I -B $S/release.py 7304 $D/rrp-7304-41-plain.json --only-r 41
    python3 -I -B $S/release.py 7304 $D/rrp-7304-41-alphabet.json --only-r 41 --alphabet 23,20,112,61
    python3 -I -B $S/release.py 7302 $D/rrp-7302p.json --word 597:8,590:14 --relevant --prizes 8
    python3 -I -B $S/release.py 7302 $D/rrpE-7302p.json --word 597:8,590:14 --relevant --enable --prizes 6
    python3 -I -B $S/t1.py 7302 $D/t1-7302p-0.json --word 597:8,590:14 --shard 0/4
    python3 -I -B $S/t1.py 7304 $D/t1-7304-23-20.json --grips 23,20
    python3 -I -B $S/routing.py 7304 $D/route-7304-23-41.json anatomy 23 41
    python3 -I -B $S/routing.py 7307 $D/route-7307-543-597.json search 543 597 --nmax 2 --fixed 448:3 --free 597 --target 517 --cap 300
    python3 -I -B $S/pxrun.py 7303 $D/px-7303 10800
    python3 -I -B $S/pxrun.py 7305 $D/px-7305-D8 14400 --start "94:1,134:1,421:8,499:1,421:5,305:2,389:7,504:12;504:1,431:3,576:13"
    python3 -I -B $S15/macro.py cert $T $D/ROW.json --prefix $W --labels $D/jam.npz --excursion 0 --macro M [--macro M2 ...]
    python3 -I -B $S15/macro.py check $T $D/ROW.json $D/ROW-check.json --labels $D/jam.npz --replay

The other jams and words take the same commands; the 7307′ searches add `--word 382:1,516:9,494:13,237:3,319:4,369:10,387:14`.
The four T1 shards at 7302′ are `--shard 0/4` to `3/4`, merged by summing their counts. The P** runs from the jams
used the CPU limit 10,800 and those with a start 14,400; the starts are the descents of section 2.2 (I-7304 at 7304,
O-SJ-D8 as two macros and O-D3 as one at 7305, O-SJ-D7 at 7306). The certificates of section 2.2 use the
excursion allowance of each row (0 for the tame rows, the row's E for O-D1 to O-D6); those of the runs use 20 (838 for
the run after O-D3). `macro.py` loads with the stage 14 loader, which rewrites a label cache whose digest differs from
the state (section 2.6): give it a cache of the jam itself, or a new path.
