# Conjugate closure: windows, geometric premises and conclusions (Claude, 12 October 2026)

Evidence kind: exact simulator replay and exact synthetic geometry (Q(sqrt5) arithmetic, signature bits) on isolated
public test data. No native-host, Windows/DirectX or performance evidence. Nothing here claims a general solution of E.

Notation. s is the anchor prefix, P, Q and P^-1 the three parts of a window, P^-1 the mirror inverse of P (checked letter
by letter with exact menu inverses). Classes at s: T (the target off block), Prot (the other off pieces at s), C (lattice
pieces of s that P takes off), L (other lattice pieces). M is the set of pieces that Q moves net (pose or chamber changed
between the end of P and the end of Q). For a word R, Bad(R) is the set of chambers whose lattice content would straddle
a cut or end off the lattice under R; it depends on R and on nothing else (Proposition S of restoration.md; computed by
a chamber ghost). Good(R) is its complement.

## A. The windows

The six old windows were re-read from the data of the previous round (no unchanged check was rerun); the new window
at X was computed fresh. F1 (pieces outside M end
at their exact pose at s), F2 (outside M, every P^-1 letter selects the pieces its mirror P letter selected) and F3 (the
ghost of P^-1 from the landing of M equals the real end) hold on every window with a conjugate structure.

| Window | s | P / Q / P^-1 | Type | T | Prot | C_P | Bad(P^-1) |
|---|---:|---|---|---:|---:|---:|---:|
| 36568 | 36563 | 10 / 10 / 10 | N (parking) | 87 | 20 | 198 | 198 |
| 9654 | 9649 | 10 / 10 / 10 + tail 9 | not closed as a triple; closed by the tail | 111 | 388 | 198 | 198 |
| 9654 shifted | 9660 | 9 / 10 / 9 | N, Prot not exact | 198 | 388 | 198 | 198 |
| 27388 | 27383 | 43 / 9,124 / 43 | S (SAFE-ALL Q) | 43 | 20 | 111 | 111 |
| 38553 | 38550 | 1,438 / 30 / 1,438 | S (SAFE-ALL Q) | 3 | 0 | 86 | 86 |
| 17884, 24219 | | mirror-inverse prefix 0 | no conjugate structure | | | | |
| new, at X (D-X17.md) | X after V | 10 / 10 / 10 | N (parking) | 17 | 90 | 198 | 198 |

Window 36568 (type N, parking). Words: P = [[X, Y], Z] with X = (491, 56) outside K+, Y = (283, 11) a K+ involution,
Z = (237, 28) outside K+; Q = [A, [B, X]] with A = (275, 10) and B = (407, 10) K+ involutions, the same X; P^-1 the mirror inverse of P.
- P selects L at 10 letters, Prot at 6 and C at 5; it never selects T. It parks C = 198 lattice pieces, the content of
  the triple lens 88 ∩ 237 ∩ 283, and moves the 17-piece Prot block (returned exactly by P^-1).
- Q selects T at 4 letters (A, A^-1, B, X^-1) and L at all 10; it never selects Prot or C. M = T exactly. T lands in 87
  chambers that were holes at s (L_T).
- P^-1 selects M (now lattice) at 6 of 10 letters where the mirror P letters selected none of M; outside M its
  selections equal the mirror. It ends M on K.
- Allowed landing for Q: Good(P^-1). Bad(P^-1) is the 198 holes P creates: 111 of them are Bad(Q) minus L_T, 87 lie in
  88 ∩ 237 ∩ 257 ∩ 283. Q lands T in L_T, which is in Good(P^-1), and moves no lattice piece into Bad(P^-1).
- Bad(Q) is exactly the triple lens 283 ∩ 453 ∩ 491 (198 chambers; 453 is the pole X^-1 n_407). At s it holds 111
  lattice pieces (the collateral of Q applied at s directly) and the 87 holes L_T. The whole window has
  Bad(P Q P^-1) = L_T: 87 chambers, all holes at s.

Window 9654. P = [[X, Y], Z] with X = (452, 26), Y = (507, 11), Z = (369, 15); Q = [[B, X], A] with B = (578, 1),
A = (592, 11). P puts T (111) on the lattice with C = 198 and moves 87 Prot pieces; Q moves T and 87 lattice pieces net,
all off after Q; P^-1 selects none of M; the triple is not closed and the 9-letter tail closes it. With the anchor
shifted by 11 letters (to 9660) the split is type N: P' = 9, Q' = the original P, P'^-1 = 9; Q' returns the 198 parked
pieces into chambers that were holes at 9660 and moves 87 Prot pieces to other off poses (Prot not exact).

Window 27388 (type S). P (43 letters, 20 outside K+; it starts with the same [X, Y] as window 36568) puts T (43) on the
lattice with C = 111. Q is 9,124 SAFE-ALL letters (no letter outside K+, no off piece moved; 131 off throughout).
Bad(P^-1) = 111 = C, and h = 0. After P the occupied bad chambers are exactly T's 43 chambers; after Q none. M = T and
137,021 lattice pieces, all on K after Q, so P^-1 selects M at every letter; outside M it equals the mirror.

Window 38553 (type S). P (1,438 letters, 720 outside K+) puts B' (3) on the lattice with C = 86; Q is 30 SAFE-ALL
letters; Bad(P^-1) = 86 = all holes after P (tight); Q moves B' out of Bad(P^-1) and leaves it vacant.

New window at X (type N, parking; D-X17.md and `window-X.json`). It is computed on the state after V, which is X
after 541 SAFE-ALL letters. P = [[X, Y], Z] with X = (95, 25) outside K+, Y = (56, 2) in K+ and Z = (187, 34) outside
K+; Q = [A, [B, X]] with A = (176, 1) and B = (43, 1) K+ involutions and the same X.
- P selects L at 10 letters and C at 7. It never selects T or Prot, and it parks C = 198.
- Q selects T at 4 letters and L at 10. It never selects Prot or C, and M = T exactly. T lands in 17 chambers that
  were holes after V.
- P^-1 selects M at 6 letters, where the mirror letters selected none of M. Outside M it equals the mirror.
- F1, F2 and F3 hold. Bad(P^-1) is the 198 holes P creates, and none of them is occupied after P or after Q.
This window has the shape of 36568 for another block and another lens, 56 ∩ 95 ∩ 107.

The same double commutators recur: [[X, Y], Z] is P in 36568 and 9654 and starts P in 27388; [A, [B, X]] (36568) and
[[B, X], A] (9654) are Q.

## B. Premises and conclusions

Conclusions are only ever read from replay: admissibility of every prefix, the end poses, collateral, protection and
digests. The premises below are checked before the replay.

B1. Geometric premises (closed form; exact cut tests and pole lookups, no simulation of the state):
- G1, pole compatibility (rim theory, restoration.md section 2). After a letter X outside K+ at grip c, the jumbled
  lattice content of cap c lies in the domain X K+. A cut H_e is a lattice cut in that domain's frame exactly when
  X^-1 n_e is a pole, and then no piece of the domain straddles H_e. For each jumble element, 120 of the 600 grips are
  poles. In window 36568 every grip used while cap 491 is jumbled is such a pole: 283 -> 392, 407 -> 453,
  275 -> 336, 237 -> 450 (and Z maps n_407 to n_491 and fixes n_283).
- G2, the block conditions for Q = [A, [B, X]] (letters A, B, X, B^-1, X^-1, A^-1, X, B, X^-1, B^-1) to put T on the
  lattice: T is inside cap e (B's grip) and outside cap c (X's grip); B(T) is inside cap c; X^-1 B m is in K+ (m: T's
  pose); A's cap holds T and A(T) is outside caps c and e. Every test is an exact cut test on T's own pieces. All of
  them hold for Q at 36563.
- G3, triple lens (computed, not proved in general). For Q of this form with A and B involutions, Bad(Q) is a triple
  lens of 198 chambers whose caps include c and the pole X^-1 n_e: 491 ∩ 453 ∩ 283 at 36563. At X, all 12,735
  minimal restoring candidates for the 17-block have |Bad(Q)| = 198. The 800 of the owner's shape (21 distinct
  (e, c)) each have Bad(Q) inside three caps that include c and the pole. For the Q used in D-X17.md, Bad(Q) equals
  the intersection 56 ∩ 95 ∩ 107 exactly.
- G4, parking (a lead from 36563, confirmed on X; D-X17.md). P = [[X, Y], Z] with Y a K+ letter at another cap of
  Bad(Q)'s lens (a pole of X's frame) and Z outside K+ with Z n_e = n_c; at 36563, also Z n_p = n_p for Y's grip p.
  At 36563 this rule yields 682 parking candidates; six make the whole window clean at s, the owner's P among them.
  At X it yields 4,378 candidates over the 21 owner-shaped Q's. Twenty-three give |Bad(W)| = 87 with T on the
  lattice and the other off pieces exact. One of them was carried to replay (D-X17.md).

B2. Ghost premises (exact chamber computation from the words and the off pieces only; independent of the labels):
- Bad(W) for the whole word W, together with the ghost of T (ends on K) and of Prot (returns exactly).
- W is clean at a state S exactly when Bad(W) holds no lattice piece of S (and the ghost of the off pieces is as
  stated). For V W V^-1 with V a SAFE-ALL word, it suffices that V vacates Bad(W). V never moves an off piece, so
  V^-1 is admissible after W.
- Counting identity (proved): |Bad(R)| = |C_R| + h_R, with h_R the holes of the start state whose R-trajectory leaves
  K. Vacating needs at least |Bad| holes in total (N2 per orbit) and every fixed bad chamber already vacant (N1).
- Ghost and simulator agree exactly on window 36568's Q at 36563: Bad 198, 111 occupied, which is the simulator's
  collateral of 111.
- They also agree on W at X: the ghost predicted T on the lattice (17), the other off pieces exact and Bad(W)
  vacant after V, and the replay of V W V^-1 ends with restored 17, pose_changed 0 and collateral 0 (D-X17.md).

B3. What cannot be certified before replay:
- The existence of P and Q: they are found by search over the template families above.
- Whether G3 holds for every Q of the family: computed only on the instances listed.
- The depth along the word: checked by replay.
- N1 and N2 are necessary; the vacating word V still has to be constructed (by the peeling certificate or the greedy)
  and replayed.

B4. Does X satisfy the premises? For the 17-block, yes:
- G1 to G4 and the ghost premises of B2 hold for W = P Q P^-1 at X (`premises.json`);
- |Bad(W)| = 87 is at most the 107 holes, and N1 and N2 hold;
- the peeling certificate gives a SAFE-ALL V after which Bad(W) is vacant.
The conclusions were then read from replay alone (D-X17.md). For the 87-block and B', the premises have not been
established at X.

B5. The direct counterexamples on X are in C-X.md: the literal window 36568 blocks at its twelfth twist, and the
single-letter conjugate R V R^-1 for B' has no first V step.
