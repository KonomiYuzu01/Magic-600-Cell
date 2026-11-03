# No height overshoot on the recognized regional high domain

Archived mathematical record from 2 November 2026. See README.md for its current scope and the independent review record. Original-source digests below identify the captured research version; IMPORT-MANIFEST.json binds this sanitized edition.

## Exact peak H on Gamma(A)

Use A and Gamma(A) exactly as in REGIONAL_HIGH_RESTORATION.md. Every admitted reference has target height H>=3888, including incomplete admitted pump blocks. All heights here are individual left-tree distances; a full-state peak includes every physical piece and every literal prefix.

The independently checked83-letter solved-to-X seed has peak30 over all177120 pieces and all prefixes, and ends at the exact source X digest052d47cd58a668e18b7dc121d9ef642a408969db2c03f30707a12707bf99e3db. The source REPORT.md section1 and2.1 give target height30 at X, and the exact1929-letter X-to-sigma suffix raises the target by2 at each letter to3888. The source step lemma proves this for every one of those letters; it is also forced by the endpoint difference3888-30=2*1929 and the per-letter height-change bound. The83-letter replacement reaches precisely the same full table, so the original suffix remains applicable with all collateral intact.

By the source isolation lemma, every nontarget piece throughout that suffix and all subsequent admitted pump prefixes has one of the forms

    X_p,  g_c X_p,  g_d X_p,

where the applicable g is one element of its cap's finite60-element menu group. It is not a product of unrelated cap actions on that piece. Every such group element is itself a single menu element. Its left height is at most2 and its inverse has the same bound. Tree isometry and the triangle inequality therefore give

    height(g X_p) <= height(X_p)+2 <=32.

The same holds for retained/identity elements. The full seed inverse has peak30. The target decreases by2 at every literal inverse pump/suffix step. Thus from any reference input of height H>=3888 the complete inverse reference word has full-state peak exactly H: target<=H, nontarget<=32, fixed seed<=30, and the initial target already has height H.

The regional trajectory/right-K lemma transfers every one of these prefix bounds to Gamma(A). The actual lattice endpoint then uses the verified lattice sticker algorithm of peak2. Consequently every promised-reachable input Z in Gamma(A) has a state-only sticker restoration with

    length <= H/2+20892764,
    full-state peak = H.

No legal restoration can have a peak smaller than its initial maximum H. The construction therefore attains the optimal possible peak on this domain, although its sticker word need not have minimum length. For the original reference-labelled family, preserve the sharper exact-solved length H/2+68; its peak is H as well.

This is a global ceiling along the word, not a claim that maximum height falls at every literal seed/lattice step. Small increases below H can occur there. It also does not show a bound for an input outside Gamma(A).

## Sharpened finite-neighborhood framework ceiling

Use the same fixed-k domain D[k], reachability promise, admission procedure and length bound U(h,k) of CONTINUOUS_FRAMEWORK_CANDIDATE.md. Let an admitted input have maximum height h, and choose its admitted entrance word of length j<=k. Every entrance prefix has maximum height at most h+2j<=h+2k.

- If its endpoint is lattice, the remaining sticker construction has peak2.
- If its endpoint is in the admitted D1 profile, the complete tail/lattice chain has peak2.
- If its endpoint is in Gamma(A), the result above bounds the remaining construction by that endpoint's own maximum H<=h+2k.

Therefore the earlier uniform cost U is unchanged, while its full-state height ceiling improves to

    E_sharp(h,k) = max(h+2k, 2).

The additive2 only covers the height2 correction macros from an initially lattice input. For h>=2 this is h+2k. In particular k=0 admits restoration without exceeding the input height whenever that height is positive. This remains a theorem about D[k], not a proof that arbitrary reachable states have a bounded k.

## Evidence to bind in review

- regional-high-independent-seed-verified.json: fresh166 full forward/inverse letters and complete prefix maximum30, exact X digest, source and command hashes.
- source/research/jumbling/restoration/10-x1-unbounded/REPORT.md: X/sigma/1929 provenance, isolation and step lemmas, full nontarget pose formula, finite-menu closure premise.
- regional-high-independent-proof-review.md and regional-high-independent-premise-verified.json: generated-universe, Gamma recognizer, source metric and right-K trajectory premises.
- lattice-library-height-independent-review.md: actual-endpoint lattice transfer and complete correction height2.
- continuous-framework-independent-final-review.md: final promise quantifier and unchanged cost U.

No1929-letter replay is newly claimed. The fresh seed peak and exact endpoint, combined with the pinned source's already verified suffix/isolation proof, establish the sharper height bound. The original4024 ceiling was safe but unnecessarily loose.
