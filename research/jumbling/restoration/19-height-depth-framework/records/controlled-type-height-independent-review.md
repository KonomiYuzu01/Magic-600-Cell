# Independent proof review: controlled type-height bound

Archived mathematical record from 2 November 2026. See README.md for its current scope and the independent review record. Original-source digests below identify the captured research version; IMPORT-MANIFEST.json binds this sanitized edition.

Reviewed candidate: CONTROLLED_TYPE_HEIGHT_BOUND.md, SHA25681d4736cb493efa446480c1008a55e9de50bbe16f16341c938981594fd9dea2d, pinned source a2ebb4cccd6b60d7527c001ef3b2cbc56b179249. This review is symbolic; no numerical dimension-36 recurrence, geometry, replay, or search was run.

## Source premises and control estimates

The candidate correctly reuses the independently reviewed premises: generated menu-pose heights are even nonnegative integers; each letter changes an individual q_p=max(h_p-2,0)/2 by at most1; physical types are immutable. Thus a length-ell macro satisfies |Delta q_t|<=N_t*ell, including all collateral, with no height ceiling. The prior independent arithmetic/word receipt supplies36 positive type counts summing to177120 and maximum A=7200. These premises are not re-established by a new geometric experiment here; their prior independent receipt and actual source-file hashes are bound in the accompanying review receipt.

For a FULL boundary norm ||q||_infinity<=n and the assumed uniform integers a>=0,b>=1, the actual literal cost is at most a*sum(q)+b<=a*36*n+b=c(n). Each endpoint coordinate is at most n+A*c(n)=g(n). This only uses the macro's starting burden for its length estimate. Internal macro prefixes may increase heights and typed burden; none are assumed lexicographically decreasing.

## Recurrence and full-norm induction

The recurrence in lines34–48 is total: each recursive call has a smaller dimension index and each loop has the finite count n+1 specified at entry to that invocation. Extremely large integer values do not affect this mathematical totality. No successful restoration search is used as a definition.

Joint induction proves that M_d and C_d are nondecreasing, M_d(n)>=n, and the internal m values never decrease. For larger initial n, the monotone paired update starts higher, runs at least as often, and adds nonnegative costs. This justifies replacing actual norms by upper bounds at subsequent blocks.

The lemma in lines56–63 has the necessary strong invariant: M_d(n) bounds the FULL36-coordinate norm at EVERY macro boundary of any finite controlled sequence whose first36-d coordinates stay fixed; C_d(n) bounds all literal cost. Previously fixed coordinates are included in each recursive input m. The proof would fail with only the active suffix's norm, but the frozen candidate explicitly excludes that interpretation.

For d=0 a strict lexicographic transition is impossible. For d>0 the first active coordinate is nonincreasing and initially at most n. It can strictly drop at most n times, even if a drop skips values. This partitions every finite prefix into at most n+1 constant-first blocks and at most n intervening drops. Inside each block the d-1 induction applies to the actual full norm, bounded by the current m. It bounds the block cost by C_{d-1}(m) and every boundary norm by M_{d-1}(m). The following drop costs at most c(M_{d-1}(m)) and leaves norm at most g(M_{d-1}(m)). These are exactly the four recurrence updates in order.

The recurrence overcounts a final boundary drop, and may overcount empty or absent blocks. Those additional nonnegative charges and norm enlargements preserve upper bounds. This is also valid at n=0: any impossible real drop is merely a fictitious overestimate. Thus both finite-prefix conclusions follow without presupposing that a complete restoration path already exists.

## Termination, literal cost and peak

Every strict macro contains at least one literal letter; an empty word cannot change q. An infinite selected sequence would therefore have finite prefixes with arbitrarily large total literal cost, contradicting the common finite bound C_36(n). Because the selector terminates on each admitted input, returns a macro at every nonzero q, and preserves its domain, it cannot halt prematurely. Its only terminal case is q=0. This proves the finite-prefix contradiction in line65 without a circular termination assumption on the whole sequence.

For initial maximum height H, evenness gives the integer n_H=A*max(H-2,0)/2. Each full coordinate q_t is at most N_t*max(H-2,0)/2<=n_H, so C_36(n_H) bounds the high phase's total literal cost. At most2 growth of an individual height per literal letter proves H+2*C_36(n_H) at every literal prefix.

The optional bound in line81 is also valid. At a macro boundary, q_p<=q_t<=M=M_36(n_H) implies h_p<=2*M+2, including pieces with q_p=0. The next macro has length at most c(M), so every internal height is at most2*M+2+2*c(M). M bounds the initial boundary as well, so this estimate includes the initial height and does not require a monotone within-macro path. Either displayed peak bound can be used; their minimum is also a bound, though the candidate does not need that observation.

## Exact scope

The theorem removes the globally fixed-L premise only in exchange for a PROVED uniform cost contract ell<=a*sum(q)+b and strict endpoint lexicographic descent. It still assumes a terminating state-only selector, complete availability on its admitted nonzero domain, full literal legality, all collateral, and endpoint closure. The finite-enumeration observation is conditional on decidable admission and the stated existence/length contract, as in the earlier fixed-L proof.

The result ends at individual heights at most2. It neither lands all pieces nor solves stickers, and a matching tail theorem remains necessary. The frozen candidate states those limits explicitly. For a=0, the fixed-L=b theorem is a valid and sharper alternative. No old target overshoot certificate or single282:13 witness is used to infer a general selector, an L=1 obstruction, or successor invariance.

The accompanying receipt binds this review, the current candidate, the prior reviewed premises and the source files; it records that the astronomical recurrence was not evaluated. Stop condition met: review complete, no follow-up implementation or experiment.
