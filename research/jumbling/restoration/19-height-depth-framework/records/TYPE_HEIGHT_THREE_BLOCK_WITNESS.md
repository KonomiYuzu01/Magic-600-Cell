# A shorter three-block descent, with an optional low-tail reduction

Archived mathematical record from 2 November 2026. See README.md for its current scope and the independent review record. Original-source digests below identify the captured research version; IMPORT-MANIFEST.json binds this sanitized edition.

Historical candidate awaiting independent review. Source a2ebb4cccd6b60d7527c001ef3b2cbc56b179249; full177120 physical pieces and259800 stickers. Counts are literal legal menu letters. This is a finite descent segment, not a complete restoration or a universal selector.

## Exact finite construction

Start at the already checked reachable state f3fc88372815b7297a6d62baf806a49265c3f0f439276e6b86844ef96c61cadd. Define chronological blocks

    A = (282,13)
    D = (59,7),(9,14),(543,10),(598,15)
    C = (371,2),(196,20),(262,21)
    E = (196,14).

The independently reviewed nine-letter word is A,D,C,E. D is the previously verified guarded low-tail cleanup. The root deletion test proves every one of the12 pairs between a cap of D and a distinct cap of C,E is globally noninteracting; cap282 is also globally noninteracting with every cap of D,C,E by the earlier transfer review. Each complete full-state partial action commutes across these disjoint supports, with legality preserved. Thus D can be moved to the end of this word or omitted. This statement concerns piecewise puzzle actions, not commutation of unrestricted4x4 matrices.

The root tested the shortened suffix C,E from the independently verified post-A table: execute four actual State.apply letters, then reverse all four. The checker validates every physical piece's exact chronological matrix contract at every letter, freshly obtains canonical per-pose heights, and verifies the exact inverse join. Receipt `type-lex-trimmed-macro.json` has SHA256c6afe349ef1de25a370ac0d83de6d137ccbf21d0c17c450f80aea8ba7fa34724; runtime9.734375CPU seconds including imports,8 actual letters. The shortened endpoint is

    a515a342adb34bbc90851769a1550f3428c0bed65f038e4548754303ca438b78

with table SHA2563ecd27648d1cf2f81240e418dbef55355a3ec6146c2dd6b75f78d6c8bea7d7eb. It differs from the nine-letter endpoint in its low pieces; no identical-full-endpoint claim is made.

All five forward prefixes have maximum height4. At the end of C, all q0 through q15 are zero and q16=7; E preserves those zero coordinates and lowers q16 to6. Thus A,C,E are THREE strict type-height lexicographic steps of lengths1,3,1. The first changed coordinate for A and C is type11; for E it is type16. The same fixed L=3 weights w_t=product_{s>t}(1+3*N_s) therefore make each block strictly decrease the nonnegative integer R=sum(w_t*q_t), by the independently checked bounded-macro theorem. The weights are not changed between blocks.

| Boundary | q11 | q12 | q13 | q14 | q15 | q16 | Phi | Off count |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| Old source | 2 | 6 | 2 | 1 | 3 | 6 | 536 | 103857 |
| After A | 1 | 3 | 1 | 0 | 1 | 7 | 1020 | 103388 |
| After A,C | 0 | 0 | 0 | 0 | 0 | 7 | 1486 | 105215 |
| After A,C,E | 0 | 0 | 0 | 0 | 0 | 6 | 1136 | 104444 |
| After A,D,C,E | 0 | 0 | 0 | 0 | 0 | 6 | 1136 | 102667 |

Types0 through10 were already zero at the input and stay zero at these boundaries. Five additional priority types11 through15 are cleared of height above2. This is not all-lattice placement, and it does not make their future preservation automatic. Total high burden rises; the progress claim is the fixed priority rank. The shorter route leaves1777 more off pieces than the longer route. No shortest full-restoration or shortest five-letter rank-descent claim is made.

## One common rank also accounts for the low-tail block

Let n be the number of off-lattice pieces and set S=(N+1)*R+n, N=177120, with the same L=3 height weights. Since0<=n<=N, a strict integer decrease of R dominates any change of n. The block D preserves the complete typed q-vector and decreases n by1777. Thus every block in the actually replayed order A,D,C,E strictly decreases the ONE fixed S: high blocks have length at most3; the low-tail block has length4. The nine-letter construction joins height descent, geometric unlocking, recovery of collateral and a net low-tail reduction in a finite verified segment.

For clarity, a general extension would still need legal selectors and a preserved domain. If every nonterminal admitted reachable state has either a high block of length at most3 lowering R, or a low block of length at most4 preserving R and lowering n, then at most R0 high blocks and n0+N*R0 low blocks occur before R=n=0. The conditional literal cost to lattice is at most

    (3+4*N)*R0 + 4*n0.

The peak is at most2*R0+10 by the source per-letter height bound. Appending the independently checked reachable-lattice sticker procedure adds20892696 letters and has height at most2. This is a direct specialization of the existing mixed-rank counting argument, not proof of the missing selector or terminal coverage. The actual finite segment above does not establish that global hypothesis.

## Lower-bound lead and remaining obligations

The producer priority diagnostic at the post-A state identifies sole type11 high piece114169. Its five whole holders are all blocked; the only lowering holders262 and328 have complete straddler sets with empty common-wholeholder intersection. If independently verified, this excludes lowering that piece in at most two letters. The actual C lowers it in exactly three, so the corresponding target/q11-elimination distance would be exactly3 at this input. This lower-bound claim is kept pending until its separate scoped check; it is not a full-restoration distance.

Further progress at the resulting type16 boundary, preservation of a general admitted class, eventual height-at-most2 completion, and complete tail/sticker closure remain open. The five- and nine-letter endpoints have different low geometry and must not be interchanged in later legality tests.
