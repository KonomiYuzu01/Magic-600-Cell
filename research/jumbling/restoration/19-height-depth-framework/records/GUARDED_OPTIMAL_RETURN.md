# Optimal return on the recognized guarded pump language

Archived mathematical record from 2 November 2026. See README.md for its current scope and the independent review record. Original-source digests below identify the captured research version; IMPORT-MANIFEST.json binds this sanitized edition.

Historical proof draft, 2026-11-02. This is a derived consequence of the published X1 pump certificate and the verified return through the seed history. It is not a new pump mechanism, a general history decoder, or a general restoration theorem.

The exact numerical certificate is `evidence/guarded-pump-prefix-code.json`, reproduced by `guarded-pump-prefix-code-command.ps1.txt`. The source is pinned at `a2ebb4cccd6b60d7527c001ef3b2cbc56b179249`; the certificate records input digests. The geometric admissibility input is the isolation and perturbation proof in `source/research/jumbling/restoration/10-x1-unbounded/REPORT.md`, using its `witnesses/CERT.json`. Those existing geometric results are premises here, not replaced by the small numerical checks below.

Attribution: the core state-only inverse-pump return and unique inverse-certificate-block argument were already accepted in source/research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md, R21F-06 and R21F-07/X21A-07 (lines7137-7138). This note revalidates and states that machinery as an exact recognizer contract, then appends the newly checked strict tau240 history return. The core inverse-block theorem is not counted as a new discovery of this run.

Let sigma be the exact state after 2085 letters of the published H6030 word. Its seven labelled K pieces share pose Qstar and height 3888, in left-tree branch 2. Every reference to equality of other pieces means equality of their exact canonical pose matrices, not equality of local pose IDs.

The seven certificate words W_i have lengths

```
66, 302, 302, 268, 256, 254, 298.
```

Write P_i = L(lambda_i) for their chronological target products. Define the guarded boundary condition G on a valid full configuration by:

1. Every one of the 177113 pieces outside K has exactly its sigma pose.
2. All seven K pieces share an exact pose Q = L(u) Qstar, where u is an exact unit quaternion and ||u-1|| <= 1/200.
3. The left height H is at least 3888 and the left-tree branch is 2.

The admitted language consists of finite concatenations of W_i starting at sigma for which every boundary satisfies G. The source's deterministic first-containing-box family is an unbounded subset. The present decoder does not require or certify that deterministic choice rule; it recognizes the larger admitted guarded language.

## Exact tree code

Let o be the left-tree root and v_i = lambda_i o. The certificate checks, in exact quaternion arithmetic,

```
dist(o,v_i) = 2|W_i| = 132,604,604,536,512,508,596,
branch(lambda_i o) = 2,
branch(lambda_i^-1 o) = 3.
```

For every distinct i,j it also checks

```
dist(v_i,v_j) > |dist(o,v_i)-dist(o,v_j)|.
```

In a rooted tree this says that neither of the two vertices is a rooted prefix of the other. The common-prefix length is exactly

```
(dist(o,v_i)+dist(o,v_j)-dist(v_i,v_j))/2.
```

This is a prefix code of tree vertices; no assertion about syntactic prefixes of the printed menu words is needed.

Suppose the current left-tree vertex is x and a proposed predecessor is y_i = lambda_i^-1 x in branch 2. Since lambda_i^-1 o is in branch 3, the geodesic from lambda_i^-1 o to y_i passes through o. Applying lambda_i gives the geodesic

```
o ---- v_i ---- x.
```

Consequently v_i is a prefix of x and

```
h(x)-h(y_i) = dist(o,v_i) = 2|W_i|.
```

Two eligible predecessors would place two distinct v_i on the same rooted path, contradicting prefix incomparability. Thus at most one predecessor can meet the branch condition. The inverse-branch check is important: prefix incomparability alone does not imply uniqueness among every inverse that merely decreases height by some amount.

## State-only recognition and decoding

Check G using the full outside-K matrices and the common exact K pose. Reject on failure. Then perform the following exact procedure, storing the proposed inverse blocks rather than consulting an input history:

```
if the full state is exactly sigma:
    accept

if H <= 3888 but the state is not exactly sigma:
    reject

for i = 0,...,6:
    Qprev = P_i^-1 Q
    retain i only if Qprev satisfies the exact Euclidean guard,
    has branch 2, has height at least 3888, and lowers height

if the number retained is not exactly one:
    reject

replace Q by that predecessor and append W_i^-1 to the output
repeat
```

The outside background is the fixed sigma background at every proposed boundary. The tree-code result forces every retained transition to lower height by exactly 2|W_i|, not merely by a positive amount. Each iteration therefore lowers height by at least 132, so there are at most floor((H-3888)/132) iterations. A different pose at height 3888 is not an accepting terminal state.

For an admitted guarded concatenation, its actual last word supplies a candidate: its preceding boundary satisfies G. Prefix incomparability makes that candidate unique. Induction strips the whole concatenation and reaches exact sigma. This proves completeness on the stated language.

Conversely, if the decoder reaches sigma, read its proposed predecessor chain forward. At each guarded boundary, the published isolation and perturbation results make the corresponding W_i admissible, return all outside-K poses, and give precisely P_i Qprev as the next common K pose. Starting from the genuine sigma state, this reconstructs the accepted input as an admitted concatenation. Reversing this legal chain proves that the emitted inverse word is legal. This proves sound recognition and the full collateral-effect assertion.

The unique object proved here is the eligible last certificate block at a boundary. The proof does not say that all shortest paths in the full state graph are unique.

## The Euclidean guard is not sufficient

The separate finite experiment `evidence/sigma-inverse-pump-probe.json` checks every W_i^-1 directly at sigma. Each is legal and restores the outside background, but every endpoint raises height and lies in branch 3, despite satisfying the Euclidean ball guard. Therefore the branch test and exact terminal-state test cannot be replaced by ball membership alone. The decoder's recognition floor is a language boundary, not an impossibility theorem about further descent.

## Appending the optimal history return

The independently reviewed certificate `evidence/sigma-history-return-review.json`, together with the full-state replay in `evidence/sigma-history-return-verified.json`, gives a fixed legal 1824-letter word from sigma to tau240. It is the inverse of H6030[261:2085], uses actual menu inverses, restores every outside-K sigma pose, and takes all seven K pieces to height 240 in branch 2.

For an accepted input of height H, decoding to sigma uses

```
sum_i |W_i| = (H-3888)/2
```

letters. Appending that fixed history suffix produces a state-only return word of length

```
(H-3888)/2 + 1824 = (H-240)/2.
```

Every literal letter of the emitted word lowers the height of each K piece by exactly 2: this follows from the source's strict forward pump property for inverse blocks, and from saturation of the height bound for the fixed suffix.

Now fix any one of the seven labelled K pieces. Every legal I_a menu letter, at any grip, changes its height by at most 2. Hence any competing path from the accepted input to a state where that piece has height at most 240 has length at least (H-240)/2. The constructed word attains the bound.

Thus the exact full-state distance to the fixed tau240 endpoint is (H-240)/2. The same number is the minimum reach distance if any one, or all seven, of these K pieces must reach height at most 240. Adding exact outside-K endpoint restoration leaves these minima unchanged because tau240 meets it. This does not claim equality for every individual target state with low height.

These are endpoint restoration statements. Other pieces may move during individual letters. The result does not cover arbitrary guarded-looking states that the decoder rejects, intermediate pump prefixes, split-K inputs, arbitrary reachable states, or lattice landing. The fixed seed suffix is a constant witness; no history of the input state is needed.
