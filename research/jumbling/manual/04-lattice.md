# 4. The lattice

Status: **draft, 17 October 2026.** This chapter takes over when the descent and the tail have put every piece back
in a slot. From then on the shape is solved and only the labels are wrong. Most of what is left is the retained
puzzle's own solve, which section 4.4 points to; section 4.3 is the one thing that a jumbled history adds. The
reference tables are in section 4.7.

## 4.1 Where you are

When N = 0, every piece sits in a slot of the retained puzzle (proved, [restoration](../theory/restoration.md) M3).
What can still be wrong is which piece sits in which slot and how it is turned in it. The 600 centres are home
already: a centre moves only with its own cap, and on the lattice it is back in its slot (proved,
[card 2](../theory/cards/card-2-jumbled-lattice-invariants.md)).

Two kinds of move are used from here on:
- **retained twists**, which keep every piece in a slot;
- the **corrections** of section 4.3: fixed words of jumble and retained twists that start and end on the lattice.

## 4.2 Orbits, positions and frames

The 176,520 moving pieces fall into 35 **orbits** (table 4.7.1). A retained twist moves a piece only to slots of its
own orbit, so each orbit is a smaller puzzle of its own:
- an orbit has N_o slots and N_o pieces, from 120 to 7,200;
- each of its pieces carries 1, 2, 5 or 20 stickers;
- a piece can sit in its own slot in one or more ways, its **frames**: one way in 24 orbits, two in seven orbits (the
  frame group C2), five in O17 and O28 (C5), ten in O06 (D5), and sixty in O34 (A5), the 120 corner pieces with 20
  stickers each;
- the **rank** says how deep a piece lies: a piece of rank r lies in r + 1 caps.

The 600 centres, one per cap, carry the other 600 stickers and never move.

*In the program:* the retained views and filters, unchanged
([observation requirements](../../../docs/progress/1.0/jumbling-observation.md), section 1).

## 4.3 What the jumbling can leave behind, and how to remove it

### The invariant ν

The retained puzzle obeys rules like the parity rules of a cube. For every orbit:
- the pieces' positions differ from home by an **even** permutation;
- the frames add up to nothing, counted as follows: in a C2 orbit the number of pieces in their second frame is
  even; in O06 the number of pieces sitting flipped (the five frames of order 2) is even; in O17 and O28 the pieces'
  turns, each counted 0 to 4, add up to a multiple of 5. O34 has no such rule.

These rules are the coordinates of the invariant **ν**: 35 sign bits b0 to b34 (b_o = 1 when the permutation of
orbit o is odd), 8 frame bits f0, f3, f6, f8, f11, f22, f25 and f32, and 2 values modulo 5, C5_17 and C5_28. Every
retained twist keeps ν = 0 (proved, [card 1](../theory/cards/card-1-retained-group.md) F1). And ν = 0 is enough:
retained twists solve every lattice configuration the puzzle can reach that has ν = 0 (certificate: the retained
group is exactly the kernel of ν, G = 𝒦;
[stage 03](../restoration/03-retained-group-and-lattice-restorer/README.md)).

In the retained puzzle these rules are why some residuals can never be left at the end of an orbit: a lone swap of
the two buffers, a lone flip of the last buffer in a C2 orbit or in O06, and a lone turn of the last buffer in O17 or
O28. Two residuals of the last buffer are allowed, because no rule counts them: a turn in O06 (one of its four frames
of order 5) and any frame in O34. Retained twists remove them: the commutator of two orientation transfers changes
only that buffer's frame ([solving workflow](../../../docs/progress/1.0/solving-workflow.md), section 3.4).

### What a jumbled history can leave

A jumble twist can break these rules, and a word of jumble twists that ends on the lattice can leave them broken.
When the descent ends, ν can be nonzero. Its possible values are exactly the group A_J = C2^34 × C5^2
([stage 04](../restoration/04-card5-lattice-invariants/README.md): the upper bound is proved, the lower bound is a
certificate of 40 words):
- 34 bits can each be 0 or 1: the signs b4 and b7 to b34, and the frame bits f8, f11, f22, f25 and f32;
- C5_17 and C5_28 can take any value modulo 5;
- the other nine coordinates are always 0: b0, b1, b2, b3, b5, b6, f0, f3 and f6.

So in the jumbled puzzle three of the forbidden residuals can be ν instead of a mistake: a lone swap of the two
buffers in an orbit whose sign bit can be 1, a lone flip of the last buffer in O08, O11, O22, O25 or O32, and a lone
turn of the last buffer in O17 or O28. No retained twist can remove them. A lone swap in O00 to O03, O05 or O06, and
a lone flip in O00, O03 or O06, still cannot be left, because their coordinates are always 0. Once the corrections
below have brought ν to 0, all of them are impossible again.

*In the program:* ν, coordinate by coordinate, on a lattice configuration. It is a fact about the configuration.

### The corrections

A **correction** is a fixed word of jumble and retained twists that starts and ends on the lattice and changes ν. The
library has 36 (table 4.7.2): 34 change only bits and two change only a C5 value. They are a basis: every value of
A_J is made by exactly one combination of them (certificate: rank 34 over F2 and 2 over F5;
[general method](../restoration/09-general-method/METHOD.md), phase 2).

Two facts make corrections easy to use:
- **A correction does the same thing from every lattice configuration.** Whether a twist is admissible depends only
  on the shape, and every lattice configuration has the solved shape. So a correction that is admissible from the
  solved state is admissible from every lattice configuration the puzzle can reach, and it moves the labels in the
  same way from every one of them (proved, stage 03 report, section 3.2, from card 2 Lemma 2).
- **ν adds up.** ν after a correction is ν before it plus the correction's own ν (ν is a homomorphism, card 1 F1).

The rule for a person:
1. When the last piece is on the lattice, read ν.
2. Make the corrections that bring it to 0, before you solve any orbit. A correction moves pieces of many orbits
   (correction 0, of nine twists, moves 1,634 pieces: stage 03 report, section 3.4), so made later it would
   undo solved work.
3. Check that the program shows ν = 0. From then on use only retained twists.

Which corrections to make (table 4.7.3):
- **The bits.** Look up the row of every bit that is 1. Make, once each, the corrections that appear in an odd
  number of these rows. Each row changes its own bit alone, so the rows add up like the bits, and a correction that
  appears in two rows cancels.
- **C5_17 = v.** Correction 27 changes C5_17 by 4, that is by −1. Make it v times, or its inverse 5 − v times,
  whichever is fewer.
- **C5_28 = v.** Correction 30 changes C5_28 by 1. Make its inverse v times, or the correction itself 5 − v times,
  whichever is fewer.
- The order does not matter: after each correction the configuration is again on the lattice, and ν only adds.

Two examples. If b26 = 1 and b33 = 1 and every other coordinate is 0, the rows are {21} and {26}: make
corrections 21 (157 twists) and 26 (349 twists). If b12 = 1 and f11 = 1, the rows are {22, 25} and {22}: correction 22
appears twice and cancels, so make correction 25 alone (332 twists), which changes exactly b12 and f11.

The corrections need at most 25,852 twists in all: every bit correction once (22,012 twists) and each C5 correction
at most twice (3,840) (proved, [senior review](../restoration/09-general-method/SENIOR-REVIEW.md), AU-05 in
section 9). On four research inputs they took 8,129 to 21,888 twists (general method, section 6.1).

The research proves both orders: the corrections first (the restorer of stage 03 report, section 3.1) or last, by
solving towards a target that the corrections then turn into the solved state (the scope note of the same report,
section 3.2). Stage 09 puts them last only because its simulator is faster near the solved state
(`../restoration/09-general-method/scripts/lattice/scripts/restore_lattice_late.py`). For a person, first is
simpler: the target is then the ordinary solved state.

*In the program:* the 36 corrections as stored macros that the solver can make, and a check of ν before and after
each. Working out the combination with table 4.7.3 is the solver's part; whether the program may do it too is not
decided (human-solve boundary).

## 4.4 The retained method

With ν = 0 the configuration is one that retained twists alone can solve. The method is the retained puzzle's:
orbit-first block building ([solving workflow](../../../docs/progress/1.0/solving-workflow.md)), in short:
- the orbits one at a time, in the certified order of table 4.7.1 (column Place), in which each orbit's seed disturbs
  only orbits that come later;
- in each orbit two buffer positions A and B, and **stars**: the orbit's seed, a word that cycles three pieces of
  the orbit, carried to A, B and a reference position and conjugated by a setup that brings the target in without
  moving the buffers;
- the placement rules for a wrong position, then the frames, then protection of the finished orbit.

The research cascade that proves the lattice phase is a reference restorer and a correctness check, not a method for
a person (stage 03 report, section 3.6). These of its results carry over to a person's solve (senior review,
sections 12, 15 and 16):
- **Shorter seeds** (round 7). 31 of the 35 seeds were replaced: 17 by pure three-cycles, commutators [X, Y] of two
  words whose supports meet in one piece, which move nothing outside their orbit, and 14 by shorter seeds that still
  disturb later orbits. The 35 seeds have 7,024 twists together; with their inverses they are 70 stored macros.
- **One seed per three-cycle** (T10-01). A cycle of the positions A, X and Y is g c′ g⁻¹, where c′ is the seed at
  A, B and C and the word g fixes A and takes X to B and Y to C. This applies the seed once instead of up to twice.
- **Any three positions, and the fewest cycles** (T11-03). A permutation σ of an orbit's N positions needs
  (N − c)/2 three-cycles, where c counts the cycles of σ of odd length, fixed positions included, and that many
  suffice; a short word carries any three positions to A, B and C. It saves seed applications where the frames are
  trivial, though the longer carrying words do not always pay.
- **Frames two at a time** (T11-01, T11-02, T11-04). A pair carried straight to two positions changes two frames
  with two applications of the seed; in a C2 orbit with m pieces in the wrong frame, m/2 pairs suffice. A frame
  triple sets two frames freely, also on O17, O28, O06 and O34.

Under the human-solve boundary the program may propose setups for a target the solver names; the solver chooses,
may edit and executes (solving workflow, section 7).

## 4.5 How long it is

- **Proved bounds** (model review; senior review, section 16, T11-05): the combined construction of round 11 needs at
  most 20,866,844 twists for every input, or 4,655,642 actions when one application of a stored macro counts as one
  action. At one per second that is about 8 months of twists, or about 54 days of actions without a pause. The
  corrections add at most 25,852 twists, at most 38 applications of the 36 corrections.
- **Lower bound:** some input needs at least 224,601 retained twists (senior review, section 11, M2-02).
- **Measured:** on two synthetic inputs the implemented schedule took 15,761,436 and 15,760,112 twists, about six
  months at one twist per second (certificate by the checker of stage 03; general method, section 6.4). These inputs
  are not uniform samples of all lattice configurations (stage 03 report, section 3.4).
- **Not measured:** a person's solve with stars, setups and the program's setup search.

The owner closed the question of this length for the theory on 17 October 2026: the bounds stand, and the help that
lets a person carry out the lattice phase moves to the engineering line
([decision](../../../docs/wiki/decisions/owner-decisions-2026-10-17-jumbling-m2.md)).

## 4.6 What is proved and what is not

| Claim | Label | Record |
| --- | --- | --- |
| N = 0 exactly when every piece is on the lattice | proved | restoration M3 |
| On the lattice every centre is home | proved | card 2 |
| Retained twists keep ν = 0 | proved | card 1 F1 |
| Retained twists solve every reachable lattice configuration with ν = 0 (G = 𝒦) | certificate | stage 03 |
| After a jumbled history ν takes exactly the values of A_J = C2^34 × C5^2 | proved; the lower bound a certificate | stage 04 |
| The 36 corrections are a basis of A_J | certificate | stage 09; recomputed for table 4.7.3 |
| Corrections, then retained twists, solve every reachable lattice configuration in the sense of stickers, and so do corrections last (Theorem R) | proved (model review) | stage 03 report, section 3.2 |
| At most 20,866,844 twists for the cascade and 25,852 for the corrections | proved (model review) | senior review, sections 9 and 16 |
| A person can carry out the lattice phase in a reasonable time | open; moved to the engineering line | owner decision of 17 October 2026 |

"In the sense of stickers": every sticker is home; a piece may still differ from home by a rotation that fixes all
of its stickers, which nothing on the screen can show.

## 4.7 Tables

Printed by `scripts/lattice_tables.py` from repository data, which also checks the counts, the basis and the bound
stated above: the counts of card 1, the ranks of the
[theory report](../../Full_600cell_Puzzle_Theory.tex), the seeds of round 7
(`../restoration/09-general-method/data/realign.json.gz`) and the correction library
(`../restoration/09-general-method/data/library-merged.json.gz`).

### 4.7.1 The 35 orbits

Frames: the frame group, with its number of frames. ν coordinates: "= 0" marks the nine that are always 0. Place:
the orbit's place in the certified order (1 first). Seed: the length of the round 7 seed with its relocation to the
buffers, and its kind: pure (moves only its own orbit), prefix (a shorter seed that still disturbs later orbits),
kept (the earlier seed, unchanged).

| Orbit | Rank | Stickers | Pieces | Frames | ν coordinates | Place | Seed (twists) |
| --- | ---: | ---: | ---: | --- | --- | ---: | --- |
| O00 | 1 | 2 | 1,200 | C2 (2) | b0 = 0, f0 = 0 | 2 | 584 prefix |
| O01 | 2 | 1 | 3,600 | 1 | b1 = 0 | 35 | 608 pure |
| O02 | 3 | 1 | 2,400 | 1 | b2 = 0 | 5 | 314 prefix |
| O03 | 3 | 2 | 3,600 | C2 (2) | b3 = 0, f3 = 0 | 23 | 566 prefix |
| O04 | 4 | 1 | 7,200 | 1 | b4 | 34 | 74 pure |
| O05 | 4 | 1 | 7,200 | 1 | b5 = 0 | 33 | 166 pure |
| O06 | 4 | 5 | 720 | D5 (10) | b6 = 0, f6 = 0 | 1 | 28 prefix |
| O07 | 5 | 1 | 7,200 | 1 | b7 | 32 | 82 pure |
| O08 | 5 | 2 | 3,600 | C2 (2) | b8, f8 | 8 | 114 prefix |
| O09 | 6 | 2 | 7,200 | 1 | b9 | 10 | 202 prefix |
| O10 | 7 | 1 | 7,200 | 1 | b10 | 15 | 776 kept |
| O11 | 7 | 2 | 3,600 | C2 (2) | b11, f11 | 14 | 68 pure |
| O12 | 8 | 1 | 7,200 | 1 | b12 | 22 | 72 pure |
| O13 | 8 | 1 | 7,200 | 1 | b13 | 13 | 68 pure |
| O14 | 8 | 2 | 7,200 | 1 | b14 | 21 | 162 pure |
| O15 | 9 | 1 | 2,400 | 1 | b15 | 4 | 106 prefix |
| O16 | 9 | 2 | 7,200 | 1 | b16 | 20 | 86 prefix |
| O17 | 9 | 5 | 1,440 | C5 (5) | b17, C5_17 | 3 | 24 prefix |
| O18 | 10 | 1 | 7,200 | 1 | b18 | 31 | 248 pure |
| O19 | 10 | 1 | 7,200 | 1 | b19 | 30 | 106 pure |
| O20 | 10 | 2 | 7,200 | 1 | b20 | 29 | 46 pure |
| O21 | 11 | 1 | 7,200 | 1 | b21 | 7 | 56 prefix |
| O22 | 11 | 2 | 3,600 | C2 (2) | b22, f22 | 6 | 38 prefix |
| O23 | 12 | 2 | 7,200 | 1 | b23 | 9 | 54 prefix |
| O24 | 13 | 1 | 7,200 | 1 | b24 | 19 | 56 kept |
| O25 | 13 | 2 | 3,600 | C2 (2) | b25, f25 | 12 | 56 kept |
| O26 | 14 | 1 | 7,200 | 1 | b26 | 18 | 56 kept |
| O27 | 14 | 1 | 7,200 | 1 | b27 | 28 | 66 pure |
| O28 | 14 | 5 | 1,440 | C5 (5) | b28, C5_28 | 17 | 1,546 prefix |
| O29 | 15 | 1 | 2,400 | 1 | b29 | 11 | 158 pure |
| O30 | 15 | 2 | 7,200 | 1 | b30 | 27 | 56 pure |
| O31 | 16 | 1 | 7,200 | 1 | b31 | 26 | 56 pure |
| O32 | 17 | 2 | 3,600 | C2 (2) | b32, f32 | 25 | 74 pure |
| O33 | 18 | 1 | 2,400 | 1 | b33 | 24 | 50 pure |
| O34 | 19 | 20 | 120 | A5 (60) | b34 | 16 | 202 prefix |

### 4.7.2 The correction library

Family: the name in the library file. Changes: the coordinates of ν that the correction changes; for C5, by how much.

| Id | Family | Twists | Changes |
| ---: | --- | ---: | --- |
| 0 | w9 | 9 | b8, b25 |
| 1 | mirror1 | 5 | b8, b11, b22, b25, b34 |
| 2 | mirror1 | 5 | b34 |
| 3 | card5:B4-0 | 9 | b4, b7 to b10, b12, b14 to b16, b18, b20, b23, b25, b26, b28 to b31, b34 |
| 4 | card5:B4-1 | 9 | b4, b7 to b10, b12, b14 to b16, b18, b21, b22, b24 to b26, b28, b31 to b34 |
| 5 | mirror2 | 9 | b13, b15, b16, b19, b20, b22, b23, b26 to b28, b31 to b34 |
| 6 | mirror2 | 9 | b8, b9, b11, b14, b16 to b21, b23 to b31 |
| 7 | mirror2 | 9 | b10, b12, b14, b17, b18, b21 to b23, b25 to b28, b31, b33 |
| 8 | mirror2 | 9 | b10, b12, b14, b15, b17, b19, b23, b24, b29, b30, b32 to b34 |
| 9 | mirror2 | 9 | b7, b8, b10, b15, b17 to b28, b31, b33, b34 |
| 10 | mirror2 | 9 | b8, b16, b18 to b20, b22, b23, b26 to b28, b31 to b34 |
| 11 | mirror2 | 9 | b12, b15, b16, b18, b20, b22, b23, b26 to b28, b31 to b34 |
| 12 | card5:DB-3 | 37 | b20, b22, b23, b28, b31 to b33 |
| 13 | card5:DB-4 | 37 | b24, b25, b27, b28, b31 to b33 |
| 14 | card5:DB-5 | 37 | b10, b12, b16, b17, b19, b21, b24, b27, b28, b30, b31, b33 |
| 15 | card5:DB-6 | 37 | b17, b18, b21 to b23, b28, b30, b31, b33 |
| 16 | card5:DBC-0 | 37 | b18, b20, b24, b26, b27, b31 to b33 |
| 17 | card5:DB-0 | 85 | b21, b22, b24, b25, b28 to b30, b33 |
| 18 | card5:DB-1 | 85 | b23, b24, b26, b28, b30, b31, b33 |
| 19 | card5:DB-2 | 85 | b19, b22 to b25, b27, b28, b31 to b33 |
| 20 | card5:FR8-0 | 128 | f8 |
| 21 | card5:MB2-0 | 157 | b26 |
| 22 | diff:W3->I1 | 228 | f11 |
| 23 | card5:MB2-1 | 317 | b31 |
| 24 | card5:MO24-0 | 317 | b24 |
| 25 | diff:W3->A5 | 332 | b12, f11 |
| 26 | card5:MB1-0 | 349 | b33 |
| 27 | card5:FR17-0 | 512 | C5_17 by 4 |
| 28 | card5:MB2-2 | 701 | b32 |
| 29 | card5:MB1-1 | 1,405 | b27 |
| 30 | card5:FR28-0 | 1,408 | C5_28 by 1 |
| 31 | card5:MB2-4 | 2,557 | b29 |
| 32 | card5:FR-0 | 2,816 | f25 |
| 33 | card5:FR22-0 | 2,816 | f22 |
| 34 | card5:WA0 | 3,720 | f32 |
| 35 | card5:MB2-5 | 5,629 | b25 |

### 4.7.3 One bit at a time

For each bit that can be 1, the corrections whose combination changes that bit alone, and their total length.
Fourteen bits have a correction of their own.

| Bit | Corrections | Twists |
| --- | --- | ---: |
| b4 | 0, 1, 4, 6, 7, 8, 9, 10, 11, 29 | 1,482 |
| b7 | 2, 3, 4, 7, 8, 9, 10, 13, 14, 16, 18, 21, 22, 23, 24, 25, 26, 28, 35 | 8,285 |
| b8 | 0, 35 | 5,638 |
| b9 | 1, 2, 3, 4, 6, 7, 12, 13, 14, 15, 16, 17, 18, 24, 26, 29, 31, 35 | 10,658 |
| b10 | 7, 8, 11, 13, 14, 15, 16, 17, 21, 24, 29, 35 | 7,768 |
| b11 | 0, 1, 2, 3, 4, 12, 17, 23, 26, 35 | 6,454 |
| b12 | 22, 25 | 560 |
| b13 | 5, 11, 12, 16, 19, 21, 22, 23, 25, 26, 28, 35 | 7,890 |
| b14 | 0, 2, 3, 4, 7, 10, 13, 14, 17, 18, 21, 23, 24, 26, 28 | 2,135 |
| b15 | 2, 7, 8, 12, 16, 17, 19, 29, 35 | 7,301 |
| b16 | 0, 2, 10, 16, 19, 23, 26, 28, 29 | 2,917 |
| b17 | 3, 4, 13, 15, 16, 21, 23, 24, 28, 31, 35 | 9,807 |
| b18 | 0, 2, 7, 8, 10, 11, 17, 21, 22, 23, 25, 26, 28, 29, 35 | 9,253 |
| b19 | 0, 2, 7, 8, 10, 11, 12, 16, 17, 19, 22, 25, 29 | 2,259 |
| b20 | 0, 2, 7, 8, 10, 11, 16, 17, 22, 24, 25, 35 | 6,678 |
| b21 | 0, 2, 7, 8, 10, 11, 12, 13, 16, 18, 21, 22, 23, 25, 29, 31, 35 | 10,871 |
| b22 | 3, 4, 12, 17, 23, 26, 35 | 6,435 |
| b23 | 0, 2, 3, 4, 7, 8, 10, 11, 13, 16, 22, 23, 25, 26, 29, 35 | 8,402 |
| b24 | 24 | 317 |
| b25 | 35 | 5,629 |
| b26 | 21 | 157 |
| b27 | 29 | 1,405 |
| b28 | 13, 23, 24, 26, 28, 29, 35 | 8,755 |
| b29 | 31 | 2,557 |
| b30 | 0, 2, 3, 4, 7, 8, 10, 11, 16, 18, 21, 22, 23, 25, 26, 28 | 2,274 |
| b31 | 23 | 317 |
| b32 | 28 | 701 |
| b33 | 26 | 349 |
| b34 | 2 | 5 |
| f8 | 20 | 128 |
| f11 | 22 | 228 |
| f22 | 33 | 2,816 |
| f25 | 32 | 2,816 |
| f32 | 34 | 3,720 |
