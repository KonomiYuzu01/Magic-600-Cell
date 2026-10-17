# 1. What you see

This chapter names the things the solver looks at. Each section ends with what the program shows for it.

## 1.1 Caps and twists

The puzzle has 600 cells, and each cell has a **cap**: the part of the puzzle on that cell's side of its cut. In the
retained puzzle a cap turns by the 12 rotations of its tetrahedron, and every piece it moves lands in a slot again.
Jumbling adds 48 more rotations per cap: with them each cap's **menu** is the 60 rotations of an icosahedral group
(menu I_a, the owner's choice of 13 October 2026). These 48 are the **jumble twists**. A jumble twist leaves the
pieces of its cap out of the slots: they are **off the lattice**.

In the research records a twist is written (cap, item), with the item one of the 60 rotations of that cap's menu.
The 177,120 pieces are the same as in the retained puzzle; only where they sit changes.

*In the program:* the caps, and each cap's menu of 60 twists.

## 1.2 Blocking

A cap can turn only when its cut runs between pieces, not through one. A piece that has been turned off the lattice
often sticks across the cuts of other caps, and those caps are **blocked** until it moves away. Whether a twist is
allowed is decided exactly, with no tolerance.

*In the program:* which caps are blocked, and which piece blocks each one.

## 1.3 Domains

Pieces that one twist turned together still fit together afterwards: they lie in one rotated copy of the slot
pattern. Such a set is a **domain**. The pieces in their slots form one domain, the **lattice**. Two pieces belong to
the same domain exactly when they fit the same rotated copy of the slot pattern (proved,
[restoration](../theory/restoration.md) T6), so the domain of a piece is a fact the program can show.

*In the program:* the domain of every piece, for example as a tint.

## 1.4 Heights

Every piece has a **left height**: 0 in its slot, and 2, 4, 6 and so on off the lattice. A jumble twist changes the
height of each piece it moves by −2, 0 or +2, so a piece at height h needs at least h/2 more jumble twists of caps
that hold it before it is back on the lattice (proved, [restoration](../theory/restoration.md) T5 and T7).

In random scrambles the heights stay small: after 1,000 random twists every off piece was at height 2, 4 or 6, almost
all at 2 (measured, [census](../restoration/11-scramble-census/README.md)). Special sequences of twists can raise a
few pieces without bound (proved, [stage 10](../restoration/10-x1-unbounded/README.md)); random scrambles have not
shown this.

*In the program:* the height of a piece, on request.

## 1.5 Seams and misalignment

A **seam** is a place where two pieces of different domains touch, or where a face of a piece touches nothing. Along
the cut of a cap, every cell of the cut is either **aligned** (covered on both sides by pieces of one domain, or by a
piece and the fixed core) or **misaligned**. The **misalignment** N_c of cap c is the number of misaligned cells along
its cut. The total misalignment N counts every misaligned cell, also those on rotated cuts that are no cap's cut;
those are seams that a jumble twist carried inside its cap, and no single twist closes them.

Three facts make N the solver's guide:
- **N = 0 exactly when every piece is on the lattice** (proved, [restoration](../theory/restoration.md) M3).
- **A twist of cap c changes N only along c's cut** (proved, M2). So the change of N for a proposed twist is the
  change of N_c, which the program can show before the twist is made.
- **A closed cut** (N_c = 0) means that every contact across the cut joins one domain and nothing sticks out or is
  left open (proved, M1). On a drawn puzzle without tools: every cut line on the cap's boundary continues across it.

*In the program:* the seams; N_c for every cap and the total N; for a twist or short word the solver proposes, what
N_c and N would become.

## 1.6 The one rule of the descent

**Keep a step only if it lowers N.** A step can be one twist or a short word of two or three twists. If it does not
lower N, take it back and try another. Two things follow:
- you never come back to a configuration you have already left, because N only goes down (proved, M2 and M4);
- the descent is over when N = 0: every piece is then on the lattice, and only the labels are left (chapter 4).

From every configuration that a scramble can reach, some word lowers N (proved,
[restoration](../theory/restoration.md) Theorem U: undoing the scramble is one), but nothing bounds how long that
word must be. Chapter 2 gives the three short kinds of step that did almost all the work in practice.
