# Restoring a jumbled 600-cell: a manual for the solver (draft)

Status: **draft, started 17 October 2026.** This is option A of the owner's decision of 17 October 2026: a manual
of the techniques the restoration method uses, written for a person
([decision](../../../docs/wiki/decisions/owner-decisions-2026-10-17-jumbling-tutorial.md)). Chapters 1 and 2 cover
what the scramble census measured, chapter 3 is an outline of research still under way, and chapter 4 is a draft of
the lattice phase. The pictures will be added when the program can draw them; until then each technique says what
the solver looks at. The complete tutorial will have detailed illustrations and every macro and formula in full,
expandable down to single twists (owner, 20 October 2026,
[decision](../../../docs/wiki/decisions/owner-decisions-2026-10-20-tutorial-acceptance.md)).

The manual is for a person who solves the jumbled puzzle in the program. It assumes the retained puzzle (the
ordinary 600-cell, whose twists keep every piece in a slot) and no mathematics beyond it. Where a claim rests on a
proof or a measurement, a short note says which, with a link to the research record.

## What you need from the program

The method needs the program to show facts about the configuration and to check moves the solver proposes. It never
needs the program to choose a move: the tools never list the twists that would help, rank caps or mark a next move
(the human-solve boundary of 2 October 2026; the requirements are in
[jumbling-observation](../../../docs/progress/1.0/jumbling-observation.md)). The one exception is the repair list
of section 2.3: for a cap the solver names, the program lists the three-twist repairs that lower N, and the solver
picks one (owner decision, 17 October 2026:
[record](../../../docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md)). The 1.0 program is not built
yet; this is what it has to show for the manual to work:

- which caps are **blocked** (a piece sticks across their cut) and which piece blocks them;
- the **domain** of every piece, for example as a tint: pieces that were turned together and still fit together;
- the **seams**: where pieces of different domains meet, or a face sticks out with nothing against it;
- for every cap, its **misalignment** N_c (how many cells along its cut are wrong) and its **floor** (below which
  no twist of that cap alone can bring N_c), and the total N;
- for a selected cap, its **cut slice**: the faces on its cut from inside and from outside, and which of them meet a
  face of their own domain;
- a **check** of a twist or a short word that the solver proposes: what N_c and N would become, and which pieces it
  would move;
- for a cap the solver names, its **repair list**: the pairs (x, d) whose three-twist repair lowers N, each with
  its y and z (chapter 2, section 2.3);
- for two caps the solver names, whether one **covers** the other (chapter 2, section 2.2; not yet in the
  observation requirements);
- the solver's **own twist record**, so the solver can take back an earlier twist of their own;
- on the lattice, the invariant **ν** coordinate by coordinate, and the 36 **corrections** as stored macros the
  solver can make (chapter 4).

## The plan of a solve

| Part | Goal | What you do | Chapter |
| --- | --- | --- | --- |
| Descent | every piece back on the lattice | undo the scramble cap by cap, reading the seams; every kept step lowers N | [2](02-descent.md) |
| Tail | the last blocks that hold one another up | clean steps: vacate, act, put back | [3](03-tail.md) (outline) |
| Lattice | every label home | read ν, make the corrections it calls for, then the retained method | [4](04-lattice.md) |

Most of a random scramble goes in the descent: scrambles of 10 and 20 twists, and one of two scrambles of 40 twists,
were undone by its first technique alone, and the deeper ones lost 35% to 99% of their off-lattice pieces to its
three techniques in 75 to 150 minutes of machine search (section 2.5). What the descent leaves behind is
the tail, which chapter 3 is about; whether its techniques always finish is the open research question of
[stage 12](../restoration/12-low-height/README.md).

## How sure each part is

Labels used in this manual:
- **Proved**: a mathematical proof in the research records (reviewed by models, not yet by a person).
- **Measured**: what happened on the scrambles that were run; it shows what can happen, not what must.
- **Open**: not known.

| Claim | Label | Record |
| --- | --- | --- |
| N = 0 exactly when every piece is on the lattice | proved | [restoration](../theory/restoration.md) M3 |
| A step that lowers N can never lead back to an earlier configuration | proved | [restoration](../theory/restoration.md) M2, M4 |
| From every reachable configuration some word lowers N | proved; its length is not bounded | [restoration](../theory/restoration.md) Theorem U |
| The descent's three techniques take random scrambles of 10 to 40 twists to the lattice | measured: three of the four scrambles of 10 to 40 twists; the fourth was 98.7% back when the clock stopped it | [census](../restoration/11-scramble-census/README.md) |
| On deeper scrambles the descent slows and leaves a tail | measured | [census](../restoration/11-scramble-census/README.md) |
| With the facts the program shows, a face match took a median of 71 checks in a simple search order, and a three-twist repair in the tail a median of about 110 openings and 800 pairs of opening and neighbour | measured on one record | [census](../restoration/11-scramble-census/README.md) |
| With the repair list, a repair step took naming one or two caps at 41 of 63 steps (at most 28) and picking one of 1 to 61 listed pairs (median 5; from step 44 on, median 3) | measured on one record, at all 63 of its repair steps | [census](../restoration/11-scramble-census/README.md) |
| Off the lattice, every unit has a clean step: a word after which the unit and every piece that was on the lattice are on the lattice, while the other off pieces may move or land; an exhaustive search from the state alone finds one | proved; its length is not bounded | [senior review](../restoration/09-general-method/SENIOR-REVIEW.md) (Theorem M\*) |
| A strict clean step, which also keeps every other off piece at its exact pose, always exists | open | [general method](../restoration/09-general-method/METHOD.md), gap M1 |
| When every orbit of the safe moves that the chambers a step needs meet carries the alternating group and has enough empty chambers, a vacating word V exists, with an explicit bound on its length | proved; built and checked for two units at the depth 40 stop, as words of fewer than 10^166 twists | [stage 12](../restoration/12-low-height/README.md), lines (a) and (b) |
| The tail can always be finished by the techniques of chapter 3, with lengths a person can carry out | open | [stage 12](../restoration/12-low-height/README.md) |
| On the lattice, what a jumbled history leaves is the invariant ν, and the 36 corrections of chapter 4 remove every value it can take | proved; certificate | [chapter 4](04-lattice.md), section 4.3 |
| The lattice phase finishes every reachable configuration on the lattice: its cascade needs at most 20,866,844 twists, its corrections at most 25,852 more | proved | [overview](../README.md) |

## Chapters

1. [What you see](01-what-you-see.md): caps, jumble twists, blocking, domains, seams and misalignment.
2. [The descent](02-descent.md): face match, cover and the three-twist repair, with worked examples.
3. [The tail](03-tail.md) (outline): blocks that hold one another up, clean steps, vacating, unlocking, transport.
4. [The lattice](04-lattice.md): the invariant ν, the corrections and the retained method, with tables of the orbits
   and the corrections.
