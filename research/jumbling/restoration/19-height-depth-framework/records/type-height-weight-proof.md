# Bounded-macro type-height descent and an explicit conditional upper bound

Archived mathematical record from 2 November 2026. See README.md for its current scope and the independent review record. Original-source digests below identify the captured research version; IMPORT-MANIFEST.json binds this sanitized edition.

Historical mathematical candidate, pending independent review. The fixed source is `a2ebb4cccd6b60d7527c001ef3b2cbc56b179249`. The proposed weights and corollary were supplied by the parent research thread; this note gives the proof and exact scope. It does not assert that the required macros exist at every reachable state.

## Definitions and source premises

Partition the N=177120 labelled physical pieces into fixed types t=0,...,35, with N_t pieces of type t. Types are immutable physical attributes, not assignments chosen after seeing a move. The source registry defines these K+ orbits in `research/jumbling/sim/regions.py:68`–`:109` (SHA256 `54ffdfff32310b88a03dbc1dbe335c2780312aa83b2e20026a3935fbdf68b709`). Choose a fixed positive integer L before constructing a route.

Work in the generated menu-pose universe with even nonnegative left heights h_p. A legal jumble changes a moved piece's height by -2,0 or2; other pieces stay fixed. These cases are explicit in Stage14 `PROOFS.md:141`–`:148` (SHA256 `ba7fbae49c48614cfdb3c4f8d9b18f578d620fd3ed842c3316876bab025d1eb7`). Retained moves preserve each height by Stage15 `PROOFS.md:31`–`:39` (SHA256 `319b9136e63524f5dafb19d76701331a1a3f037fa95e353401481b6633f7b152`). Starting at lattice height0, these facts also preserve evenness. No height ceiling is assumed.

Define the nonnegative integers

`q_p=max(h_p-2,0)/2`,  `q_t=sum_{p of type t} q_p`.

Thus `sum_t q_t=Phi/2`. A literal letter changes each q_p by at most1 in absolute value, so any legal macro of at most L literal letters satisfies

`|Delta q_t| <= L*N_t` for every t.                                      (1)

All collateral pieces are included. No common-pose, rigid-cohort, fixed-background or monotone-prefix assumption is used.

## Fixed weights and bounded-transition equivalence

For t=0,...,35, set

`w_t=product_{s>t}(1+L*N_s)`, so `w_35=1`,

and define `R_L=sum_t w_t*q_t`. These are fixed positive integers. Telescoping gives

`w_t = 1 + L*sum_{s>t} N_s*w_s`.                                      (2)

Indeed `w_{s-1}-w_s=L*N_s*w_s`; summing from s=t+1 through35 proves (2).

For a legal macro of at most L letters, suppose j is the first type with `Delta q_j!=0`. If `Delta q_j<0`, integrality gives `Delta q_j<=-1`. Earlier terms vanish, and (1)–(2) imply

`Delta R_L <= -w_j + L*sum_{s>j} N_s*w_s = -1`.

If instead `Delta q_j>0`, the symmetric inequality gives `Delta R_L>=1`. If every typed change is zero, so is `Delta R_L`. Therefore **on these bounded-length transitions**, the sign of the weighted-rank change equals the sign of the first nonzero typed height change in ascending type order.

This is not a positional encoding or an order embedding of all arbitrary height vectors: q_t is unbounded. The proof relies on the bounded difference (1), not bounded digits. For example, with two types a vector change `(-1,M)` has negative lexicographic sign but positive weighted sum for sufficiently large M; such an M exceeds the bounded-transition allowance. L must remain fixed; silently increasing it without changing and re-establishing the rank invalidates the estimate.

R_L is a nonnegative integer, is invariant under retained letters, satisfies `R_L>=Phi/2`, and is zero exactly when every piece has height at most2. Rank zero does not assert all pieces are on the lattice or restored.

## Conditional constructive high-phase theorem

Let D be a domain of exact full states. Assume a terminating state-only selector has this contract: for every Z in D with some h_p>2, it returns a legal macro of at most L letters whose typed q-vector strictly decreases lexicographically, and whose endpoint remains in D. Literal legality and all collateral are part of this contract. A decidable D plus a proved existence statement for such macros would also permit finite enumeration of words of length at most L as a selector; this note proves neither premise.

Starting from Z0 in D, each selected macro decreases R_L by at least1. Hence at most `R0=R_L(Z0)` macros occur before all heights are at most2, with at most `L*R0` literal letters. At every macro boundary, `Phi/2<=R0` gives each height at most `2R0+2`. An intermediate prefix of at most L letters can add at most2L to an individual height. The entire high phase therefore has peak height at most

`2R0+2+2L`.

To express this bound using the initial maximum height H, let `qmax=max(H-2,0)/2`. Then `q_t(Z0)<=N_t*qmax`. A second telescoping identity is

`sum_t N_t*w_t = (product_t(1+L*N_t)-1)/L`.                            (3)

Consequently the conditional high-phase literal cost is at most

`max(H-2,0)/2 * (product_t(1+L*N_t)-1)`.                              (4)

For fixed L and the fixed model, this is an explicit linear-in-H upper bound with a very large constant. It is not a practical complexity claim. It is **not an unconditional restoration bound**: macro availability, domain coverage and closure, and termination of the selector are open obligations. Completing a height-at-most2 endpoint further requires a tail theorem whose actual admission hypotheses match that endpoint, and any subsequent sticker-completion theorem retains its own reachability assumptions.

## Current finite evidence

The frozen four-coset packet `type-lex-282-results.json` (SHA256 `c0202ce27593e6c15af338a1a90e94df75ef633164122cf363b032cba8c7d68c`) supplies actual typed q changes at one exact source state. For282:13, all types below11 are unchanged and type11 has Delta q_11=-1; its total Phi/2 nevertheless rises by242. The other three representatives first increase type16. This supplies one candidate step for the L=1 type-height rank, not a selector on a domain. `type-height-weight-results.json` records the physical type counts, exact L=1 weights, telescoping checks and exact four weighted changes, with input provenance. Independent full forward/inverse verification of282:13 is a separate packet.
