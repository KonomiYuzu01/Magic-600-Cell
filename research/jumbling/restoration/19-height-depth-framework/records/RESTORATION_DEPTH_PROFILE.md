# A restoration-depth profile for the recognized pump language

Archived mathematical record from 2 November 2026. See README.md for its current scope and the independent review record. Original-source digests below identify the captured research version; IMPORT-MANIFEST.json binds this sanitized edition.

Let S belong to the prefix-complete recognized language in PREFIX_COMPLETE_OPTIMAL_RETURN.md, and let H be the common left height of its seven labelled K pieces. The state-only prefix/boundary decoder returns from S to exact sigma in (H-3888)/2 letters.

Combining this with independently replayed finite suffixes gives:

| Fixed target configuration | K height | Rigorous distance interval from S |
| --- | ---: | --- |
| tau240, exact sigma background | 240 | exactly (H-240)/2 |
| tau28, exact sigma background | 28 | [(H-28)/2 + 1, (H-28)/2 + 2] |
| tau16, exact sigma background | 16 | [(H-16)/2, (H-16)/2 + 24] |
| Fully solved configuration | 0 | [H/2, H/2 + 68] |

The upper endpoints come respectively from suffix lengths 1824, 1932, 1960, and 2012 from sigma. The solved suffix is the literal inverse of the verified2012-letter solved-to-sigma witness: the original156-letter seed was compressed first to101 by locality rewrites, then to86 and83 by exact state-specific window substitutions, followed by the unchanged1929-letter pump. Independent join checks and full seed/inverse replays support the final83 seed (evidence/seed-state-local-shortening-review.json and evidence/seed-single-turn-window-shortening.json). The original gap141, locality-only gap86, and intermediate gap71 are superseded by68. The height part of every lower endpoint follows from the individual-piece 2-per-letter bound; tau28 additionally uses the first-step obstruction below. All lower endpoints apply against every alternative legal path using all600 grips. The intervals do not assert that the upper endpoint is optimal unless the interval is a singleton.

The tau28 construction is a uniform one-letter additive approximation to the exact fixed-target distance on this infinite recognizable family. It is not a multiplicative approximation for arbitrary states. The inverse-path first-step obstruction at tau28 raises the lower endpoint by1: every legal first letter there is neutral or lowering on every K piece. The matching two-letter first-raising example does not establish the exact long distance; evidence/tau28-firststep-independent-verified.json verifies the narrower interval.

For intermediate-prefix inputs, the sigma-background targets do not generally preserve the input's outside-K matrices. They end at the stated fixed sigma background. Boundary inputs themselves have that background, so the corresponding endpoint-return statement applies there.

## Geometry can impose an additional preparation cost

The exact tau16 state has common K height16, but no legal first turn can lower any of the K pieces. Every legal first turn either raises all7 by2 or is neutral on all7. A complete first-two census excludes a neutral first turn followed by a simultaneous drop2 of all7.

For any even t with 0 <= t <=14, let m=(16-t)/2. A path of m+1 letters taking every K height to at most t cannot start by raising: even m subsequent drops would leave height above t. It must start neutral. Every remaining letter must then lower each K piece by2, which would require the excluded simultaneous second drop. Consequently

    d(tau16, {every K piece has height <= t}) >= (16-t)/2 + 2.

This first-two analysis gives lattice depth>=10 and threshold14 depth>=3. It is a global600-grip bound without any restriction on the endpoint background, later splitting, or the macro family. It is supported by evidence/sigma16-global-firsttwo-independent-verified.json: all600 initial individual-piece classifications, all60 first11 menu height effects,600 raw nonholder witnesses, and12 full retained-first-turn replays were independently recomputed.

The independently verified four-letter exclusion now strengthens the bound for even0<=t<=12 to

    d(tau16, {every K piece has height <= t}) >= (16-t)/2 + 3.

For a putative word of length (16-t)/2+2, each target has at most two slack units N+2R. The initial no-split geometry and first-two obstruction force an NNDD or RDDD prefix. The independent checker exhausts both families, including all468 neutral-family endpoints and1416 raise-first nodes, with physical labelled-piece blockers. Thus threshold12 depth>=5 and all7 lattice depth>=11. This stronger formula deliberately excludes t=14. Evidence: evidence/tau16-four-independent-verified.json; proof: tau16-four-independent-proof.md.

The word [(11,15),(17,12),(42,1),(104,3),(17,14),(7,15)] reaches the exact height12 endpoint e54adb0ae334b7951802eb06d6e19f308ca14b08f7be9e82fcc58bebdfdf01aa in six letters, with height trace16,18,16,16,16,14,12. It merges the adjacent same-cap42 letters in the earlier seven-letter concatenation. Independent forward/inverse replay preserves every exact endpoint matrix. Hence both threshold12 distance and distance to this full endpoint lie in[5,6]. No five-letter impossibility or minimal-peak claim follows. Evidence: evidence/tau16-sixletter-height12-independent-verified.json.

This local extra-cost theorem is not automatically additive to the distance from an earlier S: a competing global path from S could bypass tau16. It does not assert an upper bound or an exact lattice-landing depth for tau16.

The distinction now has concrete evidence: the recognized high branch has exact height-saturating return to tau240, while a particular reachable low-height configuration already requires geometric preparation beyond its raw height lower bound.

## Sharp three-letter descent from tau16

The independently replayed word [(11,15),(17,12),(42,15)] has target-height trace16,18,16,14 and reaches exact digest6bc3b29bc4aec1003188ddcc2573001c58da70ce5edfff10e3bf83bfc6aa2efe. Its inverse [(42,44),(17,45),(11,44)] returns every labelled piece exactly. The threshold lower bound above and this witness prove distance exactly3 both to the set where all7 K heights are at most14 and to this particular complete endpoint.

This shortest target descent changes6850 outside-K poses. Off-lattice count grows52883 to55113, and total left height grows107270 to111716; nevertheless the global excess-height potential Phi=sum(max(h_p-2,0)) decreases1504 to1490. Thus it is a verified Stage1 potential improvement with collateral effects, not exactsigma-background lowering or whole-puzzle completion. Evidence: evidence/sigma16-three-letter-height14-independent-verified.json.

A different shortest word [(11,1),(1,1),(71,15)] attains height14 without ever raising K:16,16,16,14. Its full state digest is4506f31d3d9d3953627f380004d75fe530a8ed418fba015859a38fa23c099af5. Independent full forward/inverse and table checks give Phi1490, off55163, totalheight111816,7366 changed outside-K poses. Its peak16 is also minimal because the initial height is16. Compared with the earlier word it uses the same optimal3letters and a smaller peak, but leaves50 more off-lattice pieces. These two exact endpoints must not be identified. Evidence: evidence/sigma16-no-rise-threeletter-independent-review.json.

## The next state requires another geometric preparation

At the original raising-word height14 endpoint6bc3b29b..., all-seven height<=t requires at least (14-t)/2+3 letters for every even t<=12. These bounds are not transferred to the distinct nonraising endpoints. All initial cuts, retained-first cases, outside-first preparations and raise/drop/drop cases were independently checked; the three-letter exclusion is global across600 grips.

Moreover, an8-piece cluster K union {11067} has a unique whole holder42,13 internally blocked cuts and586 all-outside cuts. Its retained motion stays in the12-element stabilizer of42. All720 possible next menu effects in those12 orientations were checked: before any target can fall below14, all seven must first rise to16. This remains true under arbitrarily long background preparation. See GEOMETRIC_CLUSTER_HEIGHT_BARRIER.md for the induction and finite certificate.

The height14 landing lower bound remains10. The cluster theorem constrains path shape and does not establish a greater depth bound by simple addition.

The four-letter word [(42,14),(104,3),(17,14),(7,15)] now realizes the threshold12 lower bound from the earlier height14 endpoint. Its trace14,16,16,14,12 also realizes the cluster's peak16 lower bound. Final complete digest e54adb0ae334b7951802eb06d6e19f308ca14b08f7be9e82fcc58bebdfdf01aa has Phi1476,off53581,totalheight108638;9738 outside-K poses change. The independent checker replayed all four letters and their inverse and matched every177120 endpoint matrix. Evidence: evidence/sigma14-fourletter-independent-verified.json. This is exact distance4 to both the threshold goal and this particular endpoint, not a statement that every height12 endpoint has distance4.

## Further verified finite improvements

All468 full endpoints in the72-preparation nonraising three-letter catalogue were independently replayed. The best off-count word is [(11,11),(53,8),(201,14)], with target trace16,16,16,14, Phi1490, off53941 and7233 changed non-K poses. Three metric triples are Pareto optimal within that stated catalogue: (1490,53941,7233),(1490,54943,7179),(1490,55003,6904). No comparison against all raising paths or arbitrary longer words is claimed. Evidence: evidence/sigma16-nonraising-catalogue-independent-verified.json.

The known height14-to12 four-letter word was conjugated by each endpoint's exact relative K+ frame. All468 transported words were mechanically blocked (197 after one letter,101 after two,170 after three). Every target-frame correspondence existed, but the distinct background prevented these particular transports. This is a complete finite negative for those468 words, not a four-letter impossibility theorem. Evidence: evidence/sigma14-catalogue-conjugated-descent.json.

At e54adb0a..., the preparation [(7,23),(4,6),(34,12)] returns K to height12 after a temporary rise to14. It reduces off53581to52466, while the final Phi remains1476. Its exact endpoint is3c8eac3b..., and the full forward/inverse checks pass. Evidence: evidence/sigma12-preparation-independent-verified.json.

Both height12 endpoints have an independently verified cohesive eight-piece cluster K union {242}. Its sole whole holder is7 in e54adb0a... and34 in3c8eac3b.... The other599 cuts comprise587 all-outside cuts and12 cuts straddling an actual cluster member. The full12-state retained orbit and720 subsequent menu heights per endpoint show that every eventual descent must first raise all7 K pieces from12to14, even after arbitrary background preparation. Thus all7 height<=t needs at least (12-t)/2+2 letters for even0<=t<=10, including at least8 for all7 lattice landing. No matching upper landing construction is claimed. Evidence: evidence/sigma12-cluster-independent-verified.json.

The subsequent independent third-letter analysis strengthens that depth bound to (12-t)/2+3, including threshold10 depth>=4 and all7 lattice depth>=9. It rechecked403200 individual raised-piece/cut classifications, all1152 exact same-cap raise/drop products, and22 full retained letters. Every raised pose has the same cap as its sole own whole holder; the subsequent drop returns to a certified retained state with all own cuts blocked. Evidence: evidence/sigma12-third-independent-verified.json. The mandatory peak14 remains; no sharpness is established.
