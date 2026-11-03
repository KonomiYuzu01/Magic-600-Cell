# Height, geodesic depth, and strict lowering

Archived mathematical record from 2 November 2026. See README.md for its current scope and the independent review record. Original-source digests below identify the captured research version; IMPORT-MANIFEST.json binds this sanitized edition.

Source: `source/research/jumbling/restoration/10-x1-unbounded/REPORT.md`, sections1,2.2–2.6, at repository commit a2ebb4cccd6b60d7527c001ef3b2cbc56b179249. New finite evidence: `evidence/sigma-strict-continuation-verified.json`. Independent mathematical review: height4_bridge accepted the metric argument, with the scope conditions below.

## Metric and premises

Vertices are full labelled mechanically reachable states. An edge is one legal, nonidentity, single-grip I_a menu letter, with cost1. Inverses are legal in reverse order. No render filtering, label quotient, retained-primitive expansion, or macro-as-one-step convention is used.

Fix one named piece p in K. Its left-tree height satisfies |h_p(Y)-h_p(X)|<=2 across any edge. Thus h_p/2 is1-Lipschitz on the entire state graph.

The X1 construction starts at sigma with h_p=3888 and supplies an infinite legal sequence of individual menu letters, each raising h_p by exactly2. This statement includes every literal prefix within each certificate word, not just word boundaries.

## Proposition1: an isometric ray

Let Z_k be the full state after k pump letters following sigma, so Z_0=sigma.

For any j<k, the recorded forward segment is a legal path of length k-j, hence d(Z_j,Z_k)<=k-j. On every competing legal path the height Lipschitz bound gives

d(Z_j,Z_k)>=(h_p(Z_k)-h_p(Z_j))/2=k-j.

Therefore d(Z_j,Z_k)=k-j for every pair. The map k↦Z_k is an isometric embedding of the nonnegative integers in the full puzzle state graph.

Competing paths may use any of the600 grips, alter other pieces, or make arbitrarily large excursions. None can shorten this distance. This is a consequence of the existing pump and height lemmas, not a new pumping construction.

Relative to the solved state O, the2085-letter seed gives

1944+k <= d(O,Z_k) <= 2085+k.

Thus the elementary height lower bound is within141 letters of the explicit construction throughout this unbounded family. This constant-gap observation does not apply to arbitrary reachable states with the same height.

## Proposition2: admissible guarded branches

The certificate contains seven words W_i of lengths66,302,302,268,256,254,298. Each returns the two debris labels to their sigma values and acts on K by L(lambda_i). The perturbation certificate proves each word legal, with height increasing by2 at every letter, from every certified guarded boundary with |u-1|<=1/200.

Consider any finite sequence of these words starting at sigma, requiring the guard again at every word boundary. The same induction as the published deterministic construction proves it admissible, with all outside-K poses equal to sigma at each boundary and

H=3888+2m,

where m is its total literal length. This includes all deterministic family boundaries and any other sequence satisfying these stated inductive premises. It does not include an arbitrary state merely because its height and geometric guard look similar.

Reversing the actual finite pump history returns that boundary exactly to sigma in m letters, restoring every outside-K pose at the endpoint. Hence its distance to sigma is exactly m by the height lower bound.

This is an existence and metric statement. It does not require the separate state-only decoder or a uniqueness claim about word histories.

## Proposition3: a uniform strict lowering anchor

Let tau be the verified endpoint of the53-letter word U from sigma:

- h_p(sigma)=3888; h_p(tau)=3856.
- All seven K pieces retain a common exact pose at tau.
- Every one of the177113 outside-K exact pose matrices equals its sigma value.
- tau digest: d111ec33e0146f1e28a219faaf2b352fb1123a64a2e24129655821d987e1b589.
- The entire53-letter word and its inverse were independently replayed.

For any admitted guarded boundary Z of Proposition2 at height H, reverse its m-letter pump history and then execute U. This reaches the same exact tau in

m+53=(H-3856)/2+37

letters. Every outside-K pose returns to its original boundary value at the endpoint. The general height lower bound gives

(H-3856)/2 <= d(Z,tau) <= (H-3856)/2+37.

Consequently the endpoint-preserving strict lowering construction is within37 letters of the unrestricted optimum to this fixed target, uniformly over the unbounded admitted boundary family. The same lower and upper bounds hold if competitors are required to restore outside-K poses at their endpoints; for these fixed exact endpoints that requirement is already automatic.

There is NO claim that outside-K pieces remain unchanged at every intermediate letter. The construction moves debris temporarily. There is also no claim of landing K, reaching the solved state, or recognizing every admissible boundary from height alone.

More generally, any legal sigma-to-tau' word of length L restoring outside K, with target height h', gives the additive gap

delta = L-(3888-h')/2 >=0.

Then d(Z,tau') lies between (H-h')/2 and (H-h')/2+delta on the same admitted family. New finite anchors can therefore be compared by a proved height/depth tradeoff, without asserting that their words can be repeated.

## Why the collective bounds are complementary

At the independently reconstructed7304 stop, maximum height is only4. Nevertheless every word bringing all pieces to height<=2 needs at least7 letters, and every word bringing all pieces to heightzero needs at least76 letters.

The first claim uses conditional first-touch times and cut-precondition regression. The second uses an exact static first-touch cover optimum, with75 disjoint witness domains plus a seven-constraint contradiction to a75-cap cover and an explicit76-cap static cover.

These results quantify constraints missed by the single-piece height lower bound. They do not say that no function of maximum height could bound restoration depth, and the76-cap static cover is not a legal restoration word.

Current evidence:
- `evidence/px3-7304-depth6-regression-verified.json`
- `evidence/px3-7304-lattice76-review.json`
- `evidence/px3-7304-lattice76-core.json`

## Open work

Later checked anchors: a different53-letter word reaches height3852, with additive gap35; a201-letter word reaches height3782, with additive gap148. The earlier17-letter anchor atheight3882 has gap14. These compare different fixed targets, so a smaller gap does not byitself mean a deeper target or a shorter restoration. New53/201 words are independently verified in evidence/sigma-transport-independent-verified.json.

The high-family strict anchors are finite verified descents. They have not been extended to heightzero or to arbitrary starting states. Bounded commutator searches may exclude only rigid, co-posed K paths within their specified height and expansion limits. A failure of one inverse preparation to transport an endpoint is a failed candidate, not a general obstruction.

Later verified improvement (2026-11-02): the seed's156letters reduce to101 by53exact locality/same-grip rewrites, independently replayed. The unchanged1929 pump suffix therefore reaches sigma in2030letters. Replace the older solved-distance upper gap141 by86 for every recognized guarded boundary/prefix state: H/2 <= d(solved,S) <= H/2+86. This is a shorter witness, not an optimality claim. See RESTORATION_DEPTH_PROFILE.md and evidence/seed-locality-reduction-review.json. The original2085letter history remains a valid witness and is preserved unchanged.
