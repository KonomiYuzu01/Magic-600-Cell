# Independent review: the correction library stays at height2

**PASS. No findings.** All36 fixed correction macros are legal from the full solved state, every physical piece has left height at most2 at every literal prefix, and every endpoint is all-lattice. The exact regional-path argument below transfers these statements to every valid all-lattice input and to each inverse word. Thus arbitrary signed concatenations of these correction macros also stay at height at most2.

This is a review-only result on pinned snapshot `a2ebb4cccd6b60d7527c001ef3b2cbc56b179249`. Source, main and all candidates were read-only. Only `lattice-library-height-independent-*` files were written. Paths below are relative to `<archive>`.

## Exact evidence and reproduction

| Frozen input | SHA256 |
|---|---|
| `lattice-library-height-audit.json` | `7b2616b9dbe99f64429da0f8a584b87778383fa9689a15f3d07250fdb4fd3081` |
| `lattice-library-height-audit.py` | `00ed516577dca059fa1b5858d25e33e406120f7d320aa16d6e2c81517e6d2650` |
| `tail-coverage-lattice-input-manifest.json` | `fceb21d2b3801a5dc833e2b7e2c7e8a5049a0dc82c28489545255fdcc1bcad42` |
| `tail-coverage-lattice-input/library-merged.json.gz` | `cfe46e135875b672975001e2e179970299ed538ef9442895580c951bede3db7e` |
| `tail-coverage-lattice-sticker-cost.json` | `ddb98e6b7e44a24b079f19cc750714381a39933bc3925516d82997c7718ecede` |
| `tail-coverage-lattice-sticker-cost.md` | `c59017a2ed68d14d7e65720e76a49be5c5c16b9b36c41ca06fe375462c9fb90d` |
| `evidence/lattice-sticker-cost-independent-verified.json` | `5e3e75fb92aba03dcf88573f5c54843057b5c027a4da9221421e22b0062cef0c` |

The input manifest's three compressed files were checked against byte counts, SHA256 and pinned Git blob SHA1. The library's blob identity is `7198a869151fc6cdf85df3a63c9c5efdc8751943`.

Production command: `lattice-library-height-independent-command.ps1.txt`, SHA256 `7fe8c64e3e68c21708d724db64c61377f68946f5e70baa98983dffac291a642f`. Script: `lattice-library-height-independent-audit.py`, SHA256 `3f91af374e2fb614692e12547e0d3482d5c74175b98a52295b1f70c6af8a4f1d`. Receipt: `lattice-library-height-independent-review.json`, SHA256 `fb400a2cd762962f9308f092d6157b84f822150673e1a9b9c51ddd76c1ad72fc`.

The check used the existing Python3.14.7/NumPy2.3.5 environment,417.109375 CPU seconds and23932 full simulator letters. It finished in one run below600 CPU seconds. Every completed macro was checkpointed; no completed macro was replayed. The command supports a partial-macro checkpoint if a future reproduction reaches its535 CPU-second guard. Timing changes may change receipt bytes. No inverse macro was physically replayed; its legality and bound follow from the exact proof below.

## Independent computation

For each macro a fresh `State(filtered=False, menu=I_a)` represented all177120 physical pieces at their home poses. The checker parsed and applied the original library records through `Twist.from_record`, not through the producer's `fastghost` engine. Before replay it matched every record's exact4x4 matrix and grip to the mapped I_a menu element and checked its retained flag by exact K+ membership. All23932 record occurrences and594 distinct grip/menu matrices passed. The exact inverse of each occurring matrix was independently found in the same grip's60-item menu.

Every literal application was certified legal by the full State classifier. At every prefix, every active non-K+ pose matrix was factored exactly; its right height was0 and left height at most2. K+ poses have both heights0. `State._compact` retains all used pose matrices, so this covers every physical piece, including full collateral. All23932 measured prefix maxima agree exactly with the frozen producer trace, and each of the36 macros attains peak2.

At every endpoint all177120 poses are in K+. Their exact chamber destinations were computed from their K+ matrices and region transport data. The result is a bijection of all177120 chambers. The complete source-to-destination array for macro i is saved in `lattice-library-height-independent-permutation-ii.npz`, for ii=00,...,35. The receipt binds every archive by SHA256, records the raw array digest, full endpoint state digest, moved-chamber count, and every prefix maximum. These are complete chamber permutations, not selected target projections.

The macro lengths, in library order, are:

`9,5,5,9,9,9,9,9,9,9,9,9,37,37,37,37,37,85,85,85,128,157,228,317,317,332,349,512,701,1405,1408,2557,2816,2816,3720,5629`.

Their sum is23932. The independently evaluated physical words supply the new height/legality evidence. The retained group/kernel theorem, the stored nu values and their spanning property, and the seed/cascade collateral proof remain the explicit previously accepted source premises in the frozen cost note; this audit does not recompute those algebraic certificates or the full259800-sticker action.

## Uniform transfer to lattice inputs and inverse words

Fix one verified word W of length L. For each home chamber q, let A_q(t) be the exact accumulated left multiplier received by its solved-state ghost after t letters, with A_q(0)=I. The check proves that every regional trajectory is legal, every A_q(t) has left height at most2, and G_q=A_q(L) belongs to K+. Let phi(q) be its final chamber. The saved full chamber array proves phi is bijective.

Consider any valid all-lattice input. Its chamber q contains a physical piece p with pose k_p in K+ and region k_p(lambda_p)=lambda_q. At prefix t that piece receives exactly A_q(t), because its region follows A_q(t)(lambda_q), the already verified ghost trajectory. Its full pose is A_q(t)k_p. Right multiplication by K+ does not change the lattice coset or its left tree height: the right factor fixes the root in the left-factor tree. Thus its prefix height is the ghost's height, all classifications remain legal, and its endpoint G_q k_p lies in K+. This covers every physical piece and arbitrary hidden lattice orientation. It is the pointwise content of the existing COV-06 chamber-matching principle, applied to a full lattice input.

For W^-1, start its ghost at chamber phi(q). After s inverse letters its regional multiplier is

`A_q(L-s) G_q^-1`.

Indeed the inverse letters reverse the already checked cut-membership trajectory, in reverse chronological order. This multiplier takes lambda_phi(q)=G_q(lambda_q) to A_q(L-s)(lambda_q). The path is therefore legal. Since G_q^-1 is in K+, its height equals the checked height of A_q(L-s), at most2. At s=L it is G_q^-1, hence lattice. Bijectivity of phi makes this cover every inverse starting chamber. An arbitrary lattice occupant adds another right K+ factor, so the same proof applies to every valid all-lattice input of the inverse word. Every inverse letter is an actual same-cap I_a menu item by the exact inverse mapping check.

Consequently each W and W^-1 is uniformly legal and lattice-to-lattice on valid lattice inputs, with height ceiling2 at every literal prefix. Repetition and concatenation preserve this ceiling because every macro boundary is lattice. This is a mathematical transfer of the checked full regional paths, not an assertion that hidden labelled endpoint poses are equal.

## Composition verdict

The frozen cost note, independently checked in receipt `5e3e75fb...`, reuses the source theorem giving, for every reachable all-lattice I_a state, a retained schedule followed by signed correction macros that solves its259800 sticker labels within

`20866844 + 25852 = 20892696`

literal I_a menu letters. The retained schedule stays entirely lattice. The measured and proved macro property above shows its entire correction phase stays at height at most2. The prior coarse ceiling2048 in cost-note line41 remains a valid upper bound but is sharpened here to2 for this fixed library and schedule.

Thus the existing reachable all-lattice sticker bound is realized inside C2. Appending this lattice phase to either accepted profile tail preserves the same height ceiling:

- For reachable members of the table00097 ten-stage family: sticker-solving depth at most `B10+20892696`, with every literal prefix at height at most2.
- For reachable members of the reconstructed D1 entry eleven-stage family: depth at most `B10+binom(177120,111)+59+20892696`, with every literal prefix at height at most2.

Here B10 is the frozen6775-digit tail bound; the eleven-stage extension is separately proved in `entry-profile-independent-review.md`, SHA256 `ed0011074c58ffd54ebb79a0c6d6fe2b4a57c2d4839dc8bc8559c7d4fb513df6`. Actual legal tail construction preserves reachability, so its all-lattice endpoint meets the reused lattice theorem's hypothesis. Preparations and SP middle words must still be rebuilt at their actual states, as required by those tail proofs.

The source attribution and metric distinctions in `tail-coverage-lattice-sticker-cost.md` lines7–19 and31 are retained: G=kernel, nu reachability and the correction/relative-target construction are reused pinned source results, and reducing signed coefficients modulo nu order is not an assertion about the full macro permutation. This review changes the verified height ceiling, not those premises or the existing cost arithmetic.

No general coverage of C2 or arbitrary-height states follows. These results solve sticker labels for the stated reachable domains; they do not prove an exact identity pose table or close the canonical-pose LRH gap. The current evidence is exact headless mechanics, not Windows/DirectX or performance evidence.

Review complete; no follow-up research or candidate/source changes. No active subprocess remains.
