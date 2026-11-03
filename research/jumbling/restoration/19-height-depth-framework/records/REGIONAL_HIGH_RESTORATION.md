# Candidate continuous high-to-sticker restoration on labelled regional fibres

Archived mathematical record from 2 November 2026. See README.md for its current scope and the independent review record. Original-source digests below identify the captured research version; IMPORT-MANIFEST.json binds this sanitized edition.

Let A be the already recognized guarded pump language including incomplete literal prefixes, from PREFIX_COMPLETE_OPTIMAL_RETURN.md. Its reference states have a shared height H on the seven distinguished target pieces. The verified state-only return first decodes to sigma, follows the fixed return through the seed, and reaches the exact solved reference table in at most H/2+68 literal I_a letters. The finite seed uses83letters and the sigma prefix uses2012letters in total. Source geometry/guard/tree-code proofs and their existing independent reviews are premises.

## A larger, precisely defined domain

For a valid reference table Y, define Gamma(Y) as follows. Choose a type-preserving bijection pi from actual physical pieces p to reference pieces q. For each p choose t_p in K+ such that t_p(lambda_p)=lambda_pi(p), and set

    Z_p = Y_pi(p) t_p.

All equality here is equality of exact pose matrices. This is more specific than merely matching visual shape. It allows arbitrary physical labels and hidden K+ orientations, including assignments to off-lattice regions, but never an unverified non-K+ right gauge. Distinct reference occupied regions and the bijection preserve geometric validity. Define Gamma(A) as the union of these fibres over Y in A.

Claimed solving domain: Gamma(A) intersected with the legal-reachable component of solved, with reachability supplied as an input promise. The recognizer for Gamma(A) itself does not require that promise. If an input lies in Gamma(A) but is unreachable, the geometric phase still lands it on the lattice; the reachable-lattice sticker theorem cannot then be silently invoked.

## Regional word-transfer lemma

For any legal word w at Y, the same chronological literal menu word is legal at every Z in Gamma(Y). At every prefix, piece p occupies exactly the region occupied by pi(p) in the reference trajectory. A cut therefore classifies the two corresponding regions identically. If it moves them, both receive the same left multiplier E, preserving

    Z_p(prefix) = Y_pi(p)(prefix) t_p.

This proves admissibility and the entire collateral effect by induction on the word. Each individual height is preserved under the right K+ factor, since K+ fixes the tree root. It follows that all per-prefix height bounds transfer, not only endpoint legality.

If Yw is the exact solved reference table, then Zw has poses t_p in K+, hence is all-lattice. Its labelled sticker assignment generally differs from solved. Because the input Z was reachable and w is legal, that lattice endpoint is reachable; the existing independently checked lattice sticker construction applies with at most20892696 additional literal letters. Its entire execution, including correction inverses, has height at most2.

Consequently every promised-reachable Z in Gamma(A) has a state-only sticker restoration of length at most

    H/2 + 20892764.

The first phase has a concrete decreasing height at each decoded target letter, followed by the fixed finite seed return; the last phase has the existing finite constructive lattice-group schedule. This is a complete chain on the declared domain. It does not assert shortest sticker distance, and it does not return an arbitrary Z to the exact canonical pose table. For the original reference-labelled family, retain the sharper H/2+68 exact-solved bound; do not add the lattice cost unnecessarily.

## Terminating state-only recognition without a supplied reference history

The finite-height pose universe is an existing source theorem: at most

    P_m = 7200(4*16^m-1)/3

reachable-universe poses have height at most2m, and they are finitely listable. Inputs here are promised to lie in that generated pose universe; right-height zero alone is insufficient. Let h be the maximum exact input height, which is even on this universe.

There is a deliberately inefficient but finite recognizer:

1. Enumerate the1348 distinct finite prefix product/background templates already verified for A. For each template enumerate every possible boundary target pose of height at most h using the finite pose list. The reference candidate has the fixed sigma background at a boundary; apply the template algebraically to form its candidate prefix table.
2. Run the existing terminating guarded-boundary recognizer on that boundary candidate. For each accepted candidate verify the template's strict target-height relation, as in the existing prefix recognizer, thereby obtaining a reference Y in A and its legal reference restoration word.
3. Check Gamma-equivalence between input Z and Y by a finite bipartite perfect-matching problem. Edge p->q is allowed precisely when the types match, t=Y_q^-1 Z_p lies in K+, and t(lambda_p)=lambda_q. All conditions are exact and decidable using finite K+ and complete region data. A perfect matching supplies pi and the right gauges, and hence the transferred word. If none exists for any candidate, reject.

Completeness: any witness Y in A has boundary target height at most its prefix target height H, and Gamma-transfer preserves all heights, so that boundary pose occurs in the finite list bounded by h. Its actual prefix template and matching therefore appear. Soundness follows from the original recognizer and the regional transfer lemma. No reaching history or unbounded search for a restoring word is an input to this recognition procedure.

This establishes decidability of Gamma(A), not practical recognition complexity. A direct region index could make it far cheaper, but is outside this proof. No arbitrary floating geometric equality or quotient by pose IDs is allowed.

## A conservative explicit height ceiling

The reference solved-to-sigma word has2012letters, so the existing per-letter height change bound gives a conservative all-piece ceiling4024 along that fixed seed/prefix path and its reverse. The83-letter low seed has every piece at height at most166. Along the two-cap suffix/pump, the isolation theorem makes each debris pose one cap-group element times its low-seed pose, so each debris height is at most168; other non-target pieces are fixed at the low-seed poses. The target height grows strictly by2per literal pump letter. Therefore the transferred restoration and subsequent height2 lattice phase have maximum height at most

    max(H, 4024).

This bound is intentionally conservative; a tighter fixed-prefix maximum is not needed for the theorem. The same inequality holds for incomplete admitted pump blocks, whose inverse prefixes restore their boundary and whose target heights decrease strictly. Every piece and all collateral moves are included.

## Scope and source boundary

The new contribution is the explicit Gamma-domain, terminating reference-free recognition argument, and composition with the literal lattice sticker cost/height certificate. Regional trajectory transfer, finite height fibres, the guarded decoder and the lattice group theorem are existing ingredients; none is relabelled as a new universal result. The remaining global obligations are unchanged: arbitrary reachable states are not proved to enter Gamma(A) or the independently covered D1 tail profile classes with bounded cost. The deep-singleton exclusions narrow a different tail obligation and do not supply that entrance theorem.

Dependencies: GUARDED_OPTIMAL_RETURN.md; PREFIX_COMPLETE_OPTIMAL_RETURN.md and its review; UPPER_BOUND_INDUCTION.md (finite pose enumeration and generated-universe restriction); CURRENT_RESULTS_20261102.md's verified83-letter seed improvement; lattice-sticker-cost-independent-review.md; lattice-library-height-independent-review.md; source10-x1-unbounded/REPORT.md isolation/step lemmas. A final frozen manifest and exact source-line/hash audit are required before claiming independent PASS.
