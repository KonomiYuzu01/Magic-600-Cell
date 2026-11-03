# A computable conditional bound with height-dependent macro lengths

## Premises and the reason for this extension

The fixed-length theorem assumes a single integer L bounds every chosen macro. Geometric unlocking may instead require a macro whose length grows with the current state. The following result permits that growth to be linear in the total typed height burden, with fixed coefficients.

There are D=36 immutable physical types. Let N_t be their positive piece counts and A=max_t N_t=7200. As in the independently checked fixed-length theorem, set

    q_t(Z) = sum_{p of type t} max(h_p(Z)-2,0)/2.

All heights are even in the generated menu-pose universe. Each literal letter changes each individual summand by at most1. Hence a legal macro of literal length ell has

    |q_t(Z')-q_t(Z)| <= N_t*ell.

These statements include every collateral piece. The source premises and their hashes are bound by the fixed-length theorem and its independent review `type-lex-independent-weight-proof-review.md` (397700905baa165350f3ce1def4ec4774ff394ff7bc6440180d4739d404f5dd1).

Assume a domain of exact full states and a terminating state-only selector. At every admitted state with q nonzero, the selector returns a legal macro whose endpoint stays in the domain, whose q-vector strictly decreases in the fixed ascending-type lexicographic order, and whose actual literal length satisfies

    ell <= a*sum_t q_t(Z) + b,

for fixed integers a>=0 and b>=1. Availability, closure and this length estimate are hypotheses, not conclusions. A decidable domain and a proved existence theorem would allow bounded finite word enumeration to implement such a selector. Neither premise is established here.

## Explicit recursive functions

The norm below always includes ALL D coordinates, even when an induction considers only an active suffix. Define, for integers n>=0,

    c(n) = a*D*n+b,
    g(n) = n+A*c(n).

At a macro boundary with max_t q_t<=n, the cost is at most c(n), and the next boundary norm is at most g(n).

Define pairs (M_d(n),C_d(n)) recursively for 0<=d<=D. The base is

    M_0(n)=n, C_0(n)=0.

For d>=1 use this finite recurrence:

    m = n
    total = 0
    repeat n+1 times:
        total = total + C_{d-1}(m)
        m = M_{d-1}(m)
        total = total + c(m)
        m = g(m)
    M_d(n) = m
    C_d(n) = total.

The subscript decreases on recursive calls; every repeat count is the finite input n+1 to that invocation. Thus these are explicitly defined total computable integer functions. They are extremely large bounds; no feasible evaluation or practical complexity claim is made. They are not defined by waiting for a restoration search to succeed.

Both functions are nondecreasing in n, and M_d(n)>=n. Prove this jointly by induction on d: the base is immediate; c and g are nondecreasing with g(n)>=n; every update of m is nondecreasing, and accumulated costs are nonnegative. Increasing n increases the starting bound and never reduces the number of iterations. The same observations show each intermediate m is at most the final M_d(n).

## Controlled lexicographic sequence lemma

For any finite sequence of the stated legal macros, suppose its first D-d coordinates are fixed at all boundaries and its remaining d coordinates strictly decrease lexicographically at every transition. If the initial FULL norm is at most n, then:

1. The full norm at EVERY macro boundary is at most M_d(n).
2. The total literal cost is at most C_d(n).

For d=0 no strict transition is possible. For d>=1, the first active coordinate is nonincreasing, starts at most n, and strictly drops at most n times. Partition the sequence into at most n+1 maximal blocks in which that coordinate stays constant, separated by at most n dropping transitions.

In each block, all earlier coordinates including that first active coordinate are fixed. Apply the d-1 induction with the current full norm bound m. The block costs at most C_{d-1}(m) and all its boundary norms are at most M_{d-1}(m). Charge the following dropping transition at most c(M_{d-1}(m)); its endpoint norm is bounded by g(M_{d-1}(m)). This is exactly one iteration of the recurrence. Fewer than n+1 blocks and a missing final dropping transition only reduce actual cost; the recurrence deliberately charges one fictitious final drop. Monotonicity allows its worst-case bound to be used at each next block. This proves both assertions, including n=0.

The lemma bounds every finite prefix. Each strict macro has at least one literal letter, so an infinite selected sequence would have finite prefixes of arbitrarily large cost, contradicting C_D(n). Therefore the selector reaches q=0 in finitely many macros. By availability it cannot stop while q is nonzero.

## Conditional high-phase bounds and endpoint

For a state with maximum initial height H, let

    n_H = A*max(H-2,0)/2.

Each q_t is at most N_t*max(H-2,0)/2, so the initial full norm is at most n_H. The high phase has explicit conditional literal bound

    U_high(H) = C_D(n_H).

A valid peak-height bound is

    E_high(H) = H+2*C_D(n_H),

because every literal letter raises any individual height by at most2. This estimate covers prefixes inside macros, where lexicographic monotonicity is not assumed. A sharper optional bound is 2*M_D(n_H)+2+2*c(M_D(n_H)), using the boundary-norm lemma and the cost of a single macro. The crude bound alone suffices for the theorem.

At termination every piece has height at most2. This does not imply all-lattice placement or solved stickers. A separate theorem must cover every terminal tail actually produced, and its literal cost and peak must be composed at the matching endpoint. The currently verified restricted tail domains are not silently treated as universal.

When a=0, the earlier fixed-L theorem with L=b is much sharper and should be used. This new recurrence is useful only if a proved height-dependent macro construction removes the fixed-L hypothesis. It does not rescue a failed strict one-letter policy by itself, nor turn the observed282:13 step into an inductively closed domain. Its remaining obligations are a legal cost-controlled macro selector, domain coverage and preservation, and complete terminal-tail coverage.
