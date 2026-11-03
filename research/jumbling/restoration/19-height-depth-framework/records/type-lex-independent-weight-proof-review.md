# Independent review of the bounded-macro type-height theorem

Archived mathematical record from 2 November 2026. See README.md for its current scope and the independent review record. Original-source digests below identify the captured research version; IMPORT-MANIFEST.json binds this sanitized edition.

Verdict: **PASS, with the hypotheses and conditional scope already stated in the frozen candidate.** No blocker, major, minor or nit finding is raised.

Reviewed file: `type-height-weight-proof.md`, SHA256 `38900979b087f8d620843c5f73464a327dd373268bf78d9ae5dfc5186a4be2da`. The arithmetic artifact has SHA256 `3ce881efab9bec7c30a31c639c4a68f94a7c733af64fc316cbe57b8906acf336`; its manifest has SHA256 `930d23886205b2786456250212bafd40562a8ce4f0be5c195f9867fe9af61dd4`.

## Premises and proof

The source premises are correctly separated from the derived rank. The generated menu-pose universe has even nonnegative heights: a jumble changes a moved piece's height by -2, 0 or 2, and a retained letter preserves it. The cited Stage14 `PROOFS.md:141` through `:148` gives the complete jumble cases; Stage15 `PROOFS.md:31` through `:39` gives retained invariance. Their exact file hashes, and the physical-type registry hash, match the frozen artifact. Piece types are permanent attributes of the labelled pieces; neither their assignment nor their ordering changes during a route.

For `q_p=max(h_p-2,0)/2`, evenness gives nonnegative integrality. This function changes by at most 1 when h changes by at most 2. Therefore a macro of at most a fixed positive integer L letters satisfies `|Delta q_t|<=L*N_t`, including every collateral piece. This argument has no height ceiling and does not require monotone prefixes.

Define `w_t=product_{s>t}(1+L*N_s)`. The identity

`w_t = 1 + L*sum_{s>t} N_s*w_s`

follows by telescoping `w_{s-1}-w_s=L*N_s*w_s`. If j is the first changed type and `Delta q_j<=-1`, then

`Delta R_L <= -w_j + L*sum_{s>j}N_s*w_s = -1`.

The symmetric inequality for a positive first change is at least +1. Thus the claimed sign equivalence holds on the specified bounded transitions. It does not assert that finite weights embed the lexicographic order on all unbounded height vectors; the candidate explicitly and correctly excludes that stronger assertion. L must stay fixed.

All weights are positive integers, so `R_L>=sum_t q_t=Phi/2`, and `R_L=0` exactly when every height is at most 2. Retained letters fix every q_p and hence this rank. Rank zero is not a lattice or solved-state conclusion.

## Conditional construction and quantitative bounds

The selector/domain theorem is valid. Its hypotheses include: full literal legality, length at most L, a strict typed height-vector decrease, and return to the admitted domain at every nonterminal macro boundary. Under that contract, a nonnegative integer rank drops by at least one per macro, so there are at most R0 macros and at most `L*R0` letters before all heights are at most 2.

The stated peak bound is also correct and is sharper than bounding the entire total height increment. At a macro boundary every individual height is at most `2R0+2`; within a macro, an individual piece gains at most `2L`. Thus the complete high phase has peak at most `2R0+2+2L`.

For initial maximum height H, `q_t<=N_t*max(H-2,0)/2`. The second telescoping identity

`L*sum_t N_t*w_t = product_t(1+L*N_t)-1`

then yields the candidate's literal cost bound

`max(H-2,0)/2 * (product_t(1+L*N_t)-1)`.

It is linear in H for fixed model and L. This is a conditional constructive high-phase bound, not an arbitrary-input restoration bound. The candidate correctly leaves domain coverage, macro availability, endpoint closure and the selector contract unproved. Its conditional finite-enumeration alternative is sound when membership and endpoint checks are decidable. Height-at-most-2 completion still needs a tail theorem with matching admission hypotheses; any sticker or exact-pose restoration statement needs its separate endpoint contract.

## Actual witness and finite arithmetic binding

The independent actual `(282,13)` replay proves one valid L=1 transition. Its first changed typed Phi/2 component is type 11 with delta -1, even though total Phi/2 rises by 242. The independently recomputed exact rank decreases. The other three cap-282 representatives have positive first changed typed components; all four integer rank values agree with the frozen arithmetic packet.

`type-lex-independent-weight-review.json`, SHA256 `c10369b8b4b18c8f0cebbe4190bbe303d5484a6c5932fbd48d28d147bb3fe311`, binds all reviewed hashes, matches the type counts to the freshly loaded physical model, independently checks all 36 identities at L=1,2,4,17 and the global identity, and checks all four rank values against independently raw-geometry-verified typed vectors. The general theorem follows from the symbolic proof above, not from those four finite L checks. No geometry, move search or source edits occurred in this arithmetic check.

The separate I-lex construction is also valid in its stated one-letter scope: each I_t is bounded by N_t<177121, so the base-177121 integer compares the entire I-vector lexicographically. Multiplying it by 177121 dominates the possible one-letter total Phi/2 increase of at most177120. For a general length-L macro the corresponding domination coefficient would be `L*N+1`; this review does not silently extend the one-letter coefficient to longer macros.

The actual word, complete source-state single-coset classification and saved endpoint are covered by `type-lex-independent-word-review.md` (SHA256 `bd24438f2f5724c1cb828f7a311f1e9e878edc4585013c2fa963c6739a3c3ba0`). One positive step does not supply an invariant admitted domain or prove that the next high state has another lowering step. No successor census or iteration was performed.
