# 4. The lattice (outline)

Status: **outline.** The chapter for a person (commutators, three-cycles and the correction macros, with what the
program shows) is still to be written. This outline states what the research has established.

## 4.1 Where the descent leaves you

When N = 0, every piece is in a slot of the retained puzzle (section 1.5). The labels can still be out of place,
and some of what is wrong cannot be fixed by retained twists alone: a jumbled history can leave corrections behind.

## 4.2 What can be wrong

The invariant ν has 43 bits over F2 and 2 values over F5. The corrections that a jumbled history can leave on the
lattice form the group A_J = C2^34 × C5^2 (stage 04: the lower bound is a certificate of 40 generator words, the
upper bound is proved).

## 4.3 The method

1. Compute ν and choose the correction from the library: 36 macros, of rank 34 over F2 and 2 over F5.
2. Solve the rest with the retained puzzle's cascade.
3. Apply the correction last.

Every reachable configuration with all pieces on the lattice is solved this way, in the sense of the stickers
(proved, Theorem R of stage 09, restricted to reachable configurations by the senior review;
[overview](../restoration/README.md)).

## 4.4 How long it is

The cascade needs at most 20,866,844 twists for every input and the corrections at most 25,852 more (proved, model
review; [overview](../README.md)). On each of two synthetic test inputs, the implemented schedule took about
1.58 × 10^7 twists (15,761,436 and 15,760,112; certificate by the checker of stage 03), about six months at one
twist per second ([general method](../restoration/09-general-method/METHOD.md), section 6.4). The owner closed the
question of this length for the theory on 17 October 2026: the bounds stand, and help for a person moves to the
engineering line ([decision](../../../docs/wiki/decisions/owner-decisions-2026-10-17-jumbling-m2.md)). How the
program helps a person carry out the lattice phase is the subject of the full chapter.
