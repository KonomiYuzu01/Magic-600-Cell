# 2. The descent

The descent puts every piece back on the lattice. It follows the one rule of section 1.6, keep a step only if it
lowers N, and uses three kinds of step:
1. **face match** (2.1): one twist that puts every face inside a cap's cut against a matching face outside;
2. **cover** (2.2): when no cap passes the face match, undo a cap that lies over other caps and under none;
3. **three-twist repair** (2.3): open a cap, improve a neighbour, then twist the first cap again.

The first two read the order of the scramble from the seams and undo it one twist at a time. The third recovers
where that reading fails. Five more kinds of step exist (2.4); in the scramble census none of them ever found the
next step before these three. Every number in this chapter's examples is recomputed from the census records by
[`scripts/examples.py`](scripts/examples.py) (output in [`scripts/examples.out`](scripts/examples.out)).

**The loop.**
1. If N = 0, the descent is over: go to chapter 4.
2. If some cap passes the face match, make the face match with the largest drop of N.
3. Otherwise, if some cap covers another and no cap covers it, make its best twist.
4. Otherwise, try three-twist repairs until one lowers N.
5. If none does, try the steps of section 2.4. If they find nothing, or finding a step takes too long, the descent
   has reached the tail (chapter 3).

After each step, go back to 1. Before a step is kept the program's check says what N would become (section 1.5); a
step that does not lower N is taken back.

## 2.1 Face match

**What to look at.** Choose a misaligned cap that can turn and look at its cut from both sides. On the cut, each
face of a piece just inside the cap either lies against a face of a piece of the same domain just outside, or it
does not. The program counts three numbers for the cap:
- In_c, the cut cells that pieces inside the cap cover;
- Out_c, the cut cells that pieces outside cover;
- A_c, the **aligned** cut cells: covered on both sides by one domain.

The misalignment is N_c = In_c + Out_c − 2·A_c. A twist of the cap turns the inside faces about the cap's pole and
leaves the outside faces where they are, like the dial of a combination lock. It never changes In_c or Out_c, so no
twist of this cap alone can bring N_c below the **floor** |In_c − Out_c|.

**The test.** The cap passes the face match when some inside face is not aligned yet (In_c > A_c) and one twist of
it aligns every inside face: after the twist, A_c = In_c. N_c then drops by 2·(In_c − A_c), to Out_c − In_c, and
when In_c = Out_c the twist closes the cut. A cap with more faces inside than outside (In_c > Out_c) cannot pass,
and neither can a cap whose inside faces are all aligned already.

**Finding the twist.** Inside faces line up with outside faces only if the twist carries their domain onto the
domain outside. At most 12 twists of the menu do that, and they differ by the 12 retained twists of the cap.
Seams of other twists that reach the cut from inside the cap leave fewer of them. Try them with the check. Take the
one with the largest drop. Among equal drops, take the one that sends the most pieces home; the labels show it.

**Which cap first.** If several caps pass, take the one with the largest drop.

**Why it works.** If a cap's cut was closed before a twist of that cap and is misaligned after it, undoing the twist
closes the cut again (proved, [restoration](../theory/restoration.md) M5). So the last twist of a scramble can be
undone by a face match, unless its cut already carried a seam before it, or it left its own cut closed (a retained
twist, which chapter 4 handles). The face match finds such a twist from the configuration alone, without knowing
the scramble. It is rarely wrong, but it misses more as the scramble deepens. At the true
scramble states of the menu I_a fixtures of 20, 40 and 60 twists, 30 caps passed, and 29 of them were caps whose
last twist could be undone next. It missed 9 such caps (measured, [restoration](../theory/restoration.md)
section 3, U1a).

### Example: ten twists undone by nine face matches

Seed 7001 of the census: a random scramble of 10 jumble twists leaves 26,519 pieces off the lattice and N = 13,270.
Of the caps that can turn, 7 are misaligned, and 6 of them pass the face match. The descent makes nine face
matches and ends at the solved state:

| Step | Twist (cap, item) | N_c before | Twists that close the cut | N after | Off the lattice after |
| ---: | --- | ---: | ---: | ---: | ---: |
| 1 | (72, 18) | 1,566 | 12 | 11,704 | 23,422 |
| 2 | (380, 38) | 1,566 | 12 | 10,138 | 20,325 |
| 3 | (519, 26) | 1,566 | 12 | 8,572 | 17,228 |
| 4 | (11, 18) | 1,360 | 1 | 7,212 | 14,466 |
| 5 | (279, 12) | 1,360 | 1 | 5,852 | 11,764 |
| 6 | (120, 24) | 1,566 | 12 | 4,286 | 8,667 |
| 7 | (310, 57) | 1,360 | 3 | 2,926 | 5,859 |
| 8 | (151, 35) | 1,360 | 1 | 1,566 | 3,097 |
| 9 | (194, 58) | 1,566 | 12 | 0 | 0 |

What to notice:
- **Every step closes its cut.** On each of these caps no cut cell was aligned and In_c = Out_c, so the face match
  closes the cut and N drops by the whole N_c.
- **Where 12 twists close the cut,** nothing on the cut tells them apart, and the labels decide: the twist that
  sends the most pieces home was the undo of the scramble's twist every time. Where only one or three close it,
  other seams that reach the cut leave fewer choices.
- **The order is not the reverse of the scramble.** The first step undoes the scramble's ninth twist (cap 72). The
  last twist of the scramble (cap 279) comes off fifth, and the first (cap 120) sixth. Twists of caps that do not
  overlap can come off in either order, and the face match takes the largest drop first.
- **Nine twists undo ten.** The scramble twisted cap 380 twice in a row (its second and third twists). One twist of
  the cap's menu undoes both, because two twists of one cap in a row are one twist of its menu.
- Each of the nine twists undoes exactly the scramble's twists of its cap. The descent has read the whole scramble
  from the seams. (The scramble's own record is used here only to label the example.)

*In the program:* the cap map with N_c, the floor and whether the cap is blocked; for a selected cap, the cut slice
with its inside and outside faces, the counts In_c, Out_c and A_c, and a turn preview; for a twist the solver
proposes, what N_c would become ([observation requirements](../../../docs/progress/1.0/jumbling-observation.md),
sections 5 to 7).

## 2.2 Cover

When no cap passes the face match, the order of the twists can still be read from how their caps overlap.

**What to look at.** Take a misaligned cap c and a neighbouring cap d whose region c's cut runs into. Look at the
face pieces of c's cut (the pieces with a face on it) whose centres lie in d's region. Cap d **covers** c when all of
them sit on aligned cells, with the same number of them, at least ten, on each side of the cut. A later twist of d
has erased c's seam there. So d was twisted after c, and d comes
off first.

**The rule.** Find a cap that covers some cap and that no cap covers, and make its best twist: the one that lowers
its N_c the most, and among those, the one that sends the most pieces home. If several caps qualify, take the
largest drop first.

### Example: a cap that cannot pass but lies on top

Seed 7101, a scramble of 40 twists, after 16 face matches: N = 30,286, and 28 misaligned caps can turn. None of
them passes the face match.
- Cap 30 has 840 faces on its cut inside and 737 outside, and none is aligned: N_30 = 1,577. Its floor is
  840 − 737 = 103, so it cannot pass.
- Cap 30 covers caps 2 and 9, and no cap covers it. Four caps qualify, with best drops of 1,474 (cap 30), 1,154
  (cap 91), 856 (cap 210) and 438 (cap 495), so cap 30 comes first.
- Its best twist, (30, 49), brings N_30 down to its floor, 103: N goes from 30,286 to 28,812.

In the scramble, cap 30 was turned once, by the 19th twist, and (30, 49) undoes it. Caps 2 and 9 were turned by the
8th and 9th twists, before it.

**How often.** In the first round of the census, cover steps were 7 of the 89 steps at depth 40 (seed 7101), 15
of 99 at depth 60, 19 of 75 at depth 100 and 28 of 183 at depth 1000. The cover order can also be wrong. On a
60-twist fixture of the smaller menu S4 it twice picked a cap whose twist was not yet free to come off, because the
overlap that ordered the two twists had since been moved away by other twists (exploratory runs,
[restoration](../theory/restoration.md) section 3). The three-twist repair recovers from such a misread.

*In the program:* the cut slice shows which face pieces of c's cut sit on aligned cells, and where they lie. For two
caps the solver names, whether one covers the other would be a fact about the configuration like N_c, but it is not
yet in the observation requirements.

## 2.3 Three-twist repair

When neither the face match nor the cover order gives a step, repair.

**The word.**
1. Choose a misaligned cap c and twist it by any twist x of its menu other than the identity. This **opens** c.
2. Choose a misaligned cap d that meets c and make its best twist y: the one that lowers N_d the most.
3. Make the best twist z of c.
4. Keep the three twists if N is now lower than before. Otherwise take them back and try another c or x.

**The check.** N changes only along the cut of the cap being twisted (proved,
[restoration](../theory/restoration.md) M2). So the change of N is the sum of three numbers: the change of N_c by
x, of N_d by y and of N_c by z. The program checks the word before it is kept.

**Why it works.** When a later twist of c has covered most of an earlier seam on a neighbour's cut, neither cap
shows a clean seam. Opening c exposes the twist underneath on d, d's best twist undoes it, and c's best twist puts
c back as well as it can go. The word (c, x)(d, y)(c, x⁻¹) is the case z = x⁻¹.

**The search.** This is the slow part: c from the misaligned caps, x from 59 twists, d from c's misaligned
neighbours. The census tried c in order of decreasing N_c and x in menu order, and kept the first word that lowered
N.

### Example: a twist buried under a later one

Seed 7101 after 29 steps: N = 15,226, with 25 misaligned caps that can turn. None passes the face match, and no
cover step is available. Caps 91 and 268 meet; N_91 = 1,360 and N_268 = 1,062.

| Twist | What it does | Change of N |
| --- | --- | ---: |
| x = (91, 1) | opens cap 91: N_91 goes from 1,360 to 1,154. It also moves pieces on cap 268's cut: N_268 goes to 1,268 | −206 |
| y = (268, 50) | the best twist of cap 268: N_268 goes from 1,268 to 412 | −856 |
| z = (91, 15) | the best twist of cap 91: N_91 goes from 1,154 to 412 | −742 |

N goes from 15,226 to 13,422, a drop of 1,804. In the scramble, cap 268 was turned by the 28th twist and cap 91 by
the 32nd, over it: y undoes the 28th twist exactly. Cap 91 is not undone. It ends in a new position with
N_91 = 412.

### Example: the tail

The last step of the first round at seed 7101 (step 89): N = 1,868, with 39 misaligned caps that can turn. Caps 552
and 400 are both at their floor (N_552 = 36, N_400 = 12), so no twist of either alone lowers its own N_c. The
repair (552, 2)(400, 2)(552, 5):
- the opening raises N_552 from 36 to 94 and also moves N_400 from 12 to 14;
- (400, 2) brings N_400 back to 12;
- (552, 5) brings N_552 back to 36.

N drops by 2. Cap 400 was never turned in the scramble. Steps like this are the tail: each lowers N by little, and
this one took the machine search about nine minutes to find.

*In the program:* for a word the solver proposes, whether it lowers N and by how much (the sum of its own-cut
changes); the floor of each cap.

## 2.4 Rarer steps

The census never needed these first, but the procedure tries them, in this order, when the three steps above find
nothing ([restoration](../theory/restoration.md) section 3):
- **Close** (U2): a twist after which a cut closes. A closing twist aligns every inside face, so it is also a face
  match.
- **Lock** (U4): open a misaligned cap c by any twist, close a misaligned neighbour d that now closes, then close a
  further cut if one can close, typically c's. Keep the twists if N is lower.
- **Improve** (U5): the single twist that lowers some N_c the most without closing it. It comes late on purpose:
  on one mixed scramble of menu S4, taking it early led to a configuration where no short step lowered N.
- **Shift** (U5′): open a misaligned cap by any twist, even one that leaves its N_c unchanged, then make the best
  twist of a neighbour. Keep both if N is lower.
- **Reopen** (U7): take back one of your own earlier twists, the most recent first, then make up to six single-twist
  steps, the first on another cap. Keep them all if N ends lower than before the reopening; otherwise take them all
  back. This uses your own twist record, which the program keeps; it never uses the scramble's.

## 2.5 What to expect

The census of 17 October 2026 ran the descent, with its rules carried out by a machine, on unrestricted random
scrambles of menu I_a for one or two rounds of 75 minutes each (measured,
[census](../restoration/11-scramble-census/README.md)):

| Scramble (twists, seed) | Off the lattice after the scramble | Steps: face match / cover / repair | End |
| --- | ---: | --- | --- |
| 10 (7001) | 26,519 | 9 / 0 / 0 | solved |
| 20 (7106) | 51,828 | 19 / 0 / 0 | on the lattice |
| 40 (7107) | 97,491 | 36 / 0 / 0 | solved, in 152 seconds |
| 40 (7101) | 90,441 | 27 / 7 / 63 | 1,193 left off: 98.7% back, after two rounds |
| 60 (7104) | 106,075 | 15 / 15 / 87 | 14,610 left: 86.2% back, after two rounds |
| 100 (7102) | 115,566 | 5 / 21 / 82 | 31,479 left: 72.8% back, after two rounds |
| 100 (7108) | 117,876 | 10 / 19 / 47 | 27,199 left: 76.9% back, after one round |
| 200 (7105) | 119,469 | 2 / 16 / 42 | 75,054 left: 37.2% back, after 29 + 75 minutes |
| 400 (7109) | 118,493 | 0 / 33 / 74 | 73,003 left: 38.4% back, after one round |
| 1000 (7103) | 125,304 | 1 / 33 / 197 | 81,309 left: 35.1% back, after two rounds |

What the runs show:
- **A fast bulk, then a slow tail.** At depths 10 to 60 the median face match or cover step lowered N by 1,105 to
  1,566. The deeper the scramble, the sooner such steps run out: at depth 40 (seed 7101) the first repair came at
  step 30, at depth 1000 at step 4. In the tail almost every step is a repair, and each lowers N by little.
- **No run got stuck.** Every run that did not reach the lattice was stopped by the clock, and every step it took
  lowered N.
- **The twists are few.** 199 twists put back 98.5% of the 40-twist scramble at seed 7101. What takes time is
  finding the next step. For a person this is the work of seeing which cap matches and which three twists repair.
- **Equal depth, different difficulty.** The two 40-twist scrambles had about the same number of pieces off. One
  fell to face matches alone, while the other ran into the tail.
- **Past about 200 twists, depth matters little.** The scrambles of 200, 400 and 1000 twists each left 118,000 to
  125,000 pieces off, and the descent put back 30% to 40% of them in its first 75 to 104 minutes.

A scramble that ends on the lattice but not solved goes on to chapter 4. What the descent leaves off the lattice
goes on to chapter 3.

## 2.6 What is proved and what is not

- **The descent cannot go round in circles.** Every kept step lowers N, a whole number that cannot go below 0
  (proved, [restoration](../theory/restoration.md) M2 and M4).
- **N = 0 exactly when every piece is on the lattice** (proved, M3).
- **A step that lowers N always exists** from every configuration a scramble can reach: undoing the scramble is
  one (proved, Theorem U). Nothing bounds how long it must be. The three techniques are short steps that did almost
  all the work in practice, not a guarantee.
- **The rules read only the configuration.** They never need the scramble's record. This chapter uses that record
  only to label its examples.
