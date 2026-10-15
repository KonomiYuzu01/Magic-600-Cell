# The new input X (Claude, 12 October 2026)

Evidence kind: exact simulator replay on isolated public test data. The restoration constructor reads only X's pose table
(`X-table.json`); the reachability certificate (`X-reachability.json`) is a separate file that no constructor reads.

## Identity

| Field | Value |
|---|---|
| Name | X-20261012-1 |
| Digest | f13c168efc4e038e4774f81159d9e24443c5c32727e7cd379767d95dc44dc0d7 |
| Off pieces | 107, in three rigid blocks of 87, 17 and 3 pieces (the same piece sets as at prefix 36563) |
| Domains | 4 (the lattice and one per block); the 17-block has two poses in one domain, the 87-block 55 poses in one domain |
| Holes | 107 |
| Rim misalignment | N(X) = 512 over 136 misaligned caps (grip-cut part 475) |

## Reachability (certificate, stored apart)

X = u(prefix 36563), with prefix 36563 (digest b318e80e…e08e1) the owner's word replayed from the E2 end. u has 302
letters, each checked admissible, with the digest after every letter recorded:
- two K+ letters (116, 5) and (431, 9), which move the 17-block and the 87-block rigidly by K+ elements;
- then 300 SAFE-ALL letters (K+ letters of caps that hold no off piece and whose cut is not straddled), drawn with
  seed 20261012 from the 570 SAFE-ALL grips available after the first two letters.

The seed state was rechecked: replaying the owner's 41,457-letter word from the E2 end to prefix 36563 gives a pose
table identical to the handed-over one (same digest, same pose ids and the same 7,251 exact matrices).

SAFE-ALL letters never move an off piece, so the off pieces of X are those of prefix 36563 moved by the first two letters
only. Their poses changed (each block by a K+ element), and the holes and lattice labels were scattered by the 300
SAFE-ALL letters.

## Differences from the old success state (prefix 36563)

From `diff-36563-X.json` (exact matrices compared entry by entry; chambers and holes from the rim domains of each state):

| Measure | Value |
|---|---:|
| Pieces whose exact pose differs | 170,539 of 177,120 |
| Off pieces (the same 107 in both) with a different exact pose | 104 (the 87- and 17-blocks; B' is unchanged) |
| Lattice pieces (177,013 in both) in a different chamber | 169,932 |
| Holes in common | 0 of 107 |

So X shares the off piece set and the blocking mechanism with 36563, but its block poses, its holes and almost all
lattice labels differ. In particular the 87 landing chambers of the old window, which were holes at 36563, are not
holes at X.

## The same blocking mechanism

| Block | 36563: straddled grips | X: straddled grips | X: inside admissible grips | Locked chambers (occupant) |
|---|---|---|---|---|
| 87 (smallest piece 3569) | 314, 339, 519 | 272, 407, 490 | 314, 327, 346, 431, 432, 435, 464, 509 | none |
| 17 (smallest piece 9117) | 99, 190, 301 | 162, 190, 209 | 43, 104, 116, 122, 140, 176, 183, 219 | none |
| B' (16245, 16249, 32088) | 250, 307 | 250, 307 | unchanged | 84069-84071 held by 57220, 47840, 47855 at both |

At X, 8 grips are straddled. The B' lock (lock lemma, Corollary L) is unchanged: any move of B' outside K+ takes
57220, 47840 and 47855 off the lattice.

## Direct counterexamples on X

1. The owner's window 36568, replayed literally at X (30 letters), is admissible for its P (10 letters, 174 lattice
   pieces taken off) and is blocked at its twelfth twist, the second letter of Q, (407, 10): piece 90780 of the 87-block
   straddles cut 407 at X. At that point 281 pieces are off and none is restored. File: `w0-at-X.json`.
2. The single-letter conjugate R V R^-1 for B' (R = (237, 13), which puts B' on the lattice) has
   |Bad(R)| = 3,097 = C_R + h_R = 3,096 + 1. N1 and N2 hold, but after R the whole cap 237 is jumbled around B', and the
   greedy SAFE-ALL search finds no first step (computed at prefix 36563 in the sandbox; B' and its lock are identical at
   X). A finite search failure, not a proof that B' cannot be restored.
3. Q-type cores alone. The restoring double commutators found at X for the 17-block have |Bad(Q)| >= 198 > 107 holes,
   so no SAFE-ALL word can vacate Bad(Q) at X (counting identity): a parking word is needed. 60,588 candidates were
   scored; the minimum is 198.
