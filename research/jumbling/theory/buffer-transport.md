# Buffer vacancy read from the current state (R1, question E, items 1 and 2)

Status: **12 October 2026.** Labels: **Proved** (by the integrator), **Computed** (exact, by the script named), **Open**, **Conjecture**. One independent Codex review (11 October 2026, proofs only, computed values out of its scope) found Proposition N, Theorem A, Corollary A1, the lock lemma and Corollary L sound under the stated model, and Theorem B sound once its words are required to be SAFE-ALL words; its five findings on the criteria built on them are answered in the text (section 9), and a scoped verification found all five fixed. Computed values are outside that review; the two scripts the integrator wrote (`setup_flow.py`, `lockcheck.py`) had a separate code review, whose four blocking findings were fixed and verified. Section 6.1 (Lemma M, 12 October) had its own full review and two scoped verifications, and the integrator's `monocheck.py` a code review and two scoped verifications; each review's scope is the section or script named, not the whole note. Section 6.2 reads the owner's v17 states. Section 8.5 takes apart the six unlock windows of the owner's construction: exact computation by `unlock.py`, which had one code review and a scoped verification; the section states no new proof, so it had no math review. Lemma numbers refer to the owner's manuscript (card 3, revision 11), which is not in this repository; its Lemmas 41, 44, 46, 48 to 51 are used as stated there.

The owner's two questions (11 October 2026): (1) turn BUFFER-PROGRESS into a condition read from the exact current geometry, in particular relax "83 specified vacant chambers" of Lemma 49, with explicit quantifiers, scope and simultaneity; (2) construct legal reachable depth-one states outside that condition and see whether blocks that cannot move alone ("frozen", Lemma 51) gain a way to recovery when a few blocks move together, keeping three outcomes apart: a finite search found nothing, a restricted strategy is infeasible, or the state is not recoverable at all.

## 1. Setting

- Y: a valid depth-one state. Ξ(Y): which pieces are off the lattice and their exact poses. Ω: the 177,120 home chambers. V(Y) ⊆ Ω: the chambers no lattice piece occupies (holes).
- A(Ξ), the SAFE-ALL alphabet: grips whose cut no piece straddles and whose cap holds no off-lattice piece, each with its 11 non-identity menu elements that lie in K. Lattice pieces never straddle a cut, so A(Ξ) depends only on Ξ. G(Ξ): the group these letters generate on Ω. By Lemma 41 a word w of A(Ξ) is admissible at every prefix, keeps every off pose and every lattice flag, and maps the holes to π_w(V).
- Orbits of G on Ω; Fix(G): the orbits of size one.
- F(Ξ): the valid states with off geometry Ξ and every other piece on the lattice, with any hole set (Lemma 48's family). A(Ξ), G and the certificates below are the same for every state of F(Ξ); only V changes.
- T ⊆ Ω: a target. BUFFER-VACANCY(Y, T) (Lemma 44's certificate): some word w of A(Ξ) has π_w(V(Y)) ⊇ T.

## 2. A necessary condition

**Proposition N (Proved).** If BUFFER-VACANCY(Y, T), then
- (N1) every chamber of T ∩ Fix(G) is a hole, and
- (N2) |V(Y) ∩ Δ| ≥ |T ∩ Δ| for every orbit Δ.

Proof. π_w is a bijection of Ω preserving every orbit, so |π_w(V) ∩ Δ| = |V ∩ Δ|, and a fixed chamber is in π_w(V) exactly when it is in V.

## 3. Jordan certificates: the counts suffice

A **Jordan certificate** for an orbit Δ of size n ≥ 8 is a word w of A(Ξ) and a prime p with n/2 < p ≤ n − 3 such that π_w has exactly one cycle of length divisible by p among all its cycles on Ω, and that cycle lies in Δ.

**Theorem A (Proved).** If orbits Δ_1, ..., Δ_r have Jordan certificates, then G contains Alt(Δ_1) × ... × Alt(Δ_r), each factor acting on its orbit and fixing every other chamber.

Proof.
1. The cycle has length p: its length is a multiple of p and at most n < 2p.
2. Let L be the order of π_w. Then p divides L and p² does not, since every other cycle length is prime to p. Put c = π_w^{L/p}. On the p-cycle, L/p is prime to p, so c is a p-cycle there. On a cycle of length ℓ with p ∤ ℓ, ℓ divides L/p, so c is the identity there. So c ∈ G is one p-cycle with support in Δ.
3. G acts on Δ transitively, and primitively: let B be a block of size b with 1 < b < n containing a point moved by c. If the ⟨c⟩-orbit of B had p blocks, then n ≥ pb ≥ 2p > n. So c(B) = B, B contains the whole support of c, and b ≥ p > n/2; but b divides n and b < n, so b ≤ n/2.
4. Jordan's theorem: a primitive group of degree n containing a cycle of prime length p ≤ n − 3 contains the alternating group. So the group G induces on Δ contains Alt(Δ).
5. Let N be the normal closure of c in G. Each conjugate g c g⁻¹ is a p-cycle supported in g(Δ) = Δ, so N fixes Ω ∖ Δ pointwise, and restricting to Δ is injective on N. The image is a non-trivial normal subgroup of the group induced on Δ, which is Alt(Δ) or Sym(Δ) with n ≥ 8; so it contains Alt(Δ). Hence G contains Alt(Δ) acting on Δ alone.
6. These subgroups for different orbits have disjoint supports and commute, so G contains their product.

**Corollary A1 (Proved).** If every orbit meeting T ∖ Fix(G) has a Jordan certificate, then for every Y ∈ F(Ξ): BUFFER-VACANCY(Y, T) holds if and only if (N1) and (N2) hold.

Proof. "Only if" is Proposition N. "If": in each orbit Δ meeting T ∖ Fix choose H_Δ ⊆ V ∩ Δ with |H_Δ| = |T ∩ Δ| and a permutation σ_Δ of Δ with σ_Δ(H_Δ) = T ∩ Δ. If σ_Δ is odd, compose it first with a transposition of two points of H_Δ (when |H_Δ| ≥ 2) or of Δ ∖ H_Δ (otherwise, since |Δ| ≥ 8); either keeps σ_Δ(H_Δ) = T ∩ Δ. The product of the σ_Δ lies in G by Theorem A, fixes every chamber outside these orbits, and takes V onto a superset of T ∖ Fix; (N1) covers T ∩ Fix.

- Simultaneity. Holes counted orbit by orbit, or orbits reachable one at a time, do not show that the holes can be placed at once (Lemma 46 gives the cyclic group on four points as an abstract counterexample). The product subgroup of Theorem A is what makes the orbits independent. The cyclic group on four points is not primitive and has no Jordan certificate.
- Quantifiers. Fix Ξ and T. The certificate words depend only on Ξ, so the equivalence holds for every state of F(Ξ), every hole set. It gives no bound on the length of the clearing word (section 4 does).
- Orbits of size 2 to 7 have no Jordan certificate (no prime in (n/2, n − 3]); for such an orbit, the group induced on it is small and can be computed exactly instead. None meets T ∖ Fix at the states of section 6.

## 4. A peeling certificate: the same with a bound

**Theorem B (Proved).** Let T′ = T ∖ Fix(G) be ordered t_1, ..., t_m, P_j = {t_1, ..., t_{j−1}}, Δ_j the orbit of t_j. Every word below is a word in the letters of A(Ξ) (a certificate is checked letter by letter against A(Ξ), not only by its permutation). Suppose that for every j:
- (a) a word E_j has π_{E_j} fixing P_j pointwise, and y_j = π_{E_j}⁻¹(t_j) is not in T;
- (b) a set R_j ⊆ Δ_j contains y_j, and for every z ∈ R_j a route word W_z has π_{W_z}(z) = y_j, π_{W_z} fixing P_j pointwise, and |W_z| ≤ e_j.

Let u_j = |Δ_j ∖ (P_j ∪ {t_j} ∪ R_j)| and h_Δ = max over j with Δ_j = Δ of |P_j ∩ Δ| + 1 + u_j. Then for every Y ∈ F(Ξ) with (N1) and |V(Y) ∩ Δ| ≥ h_Δ for every orbit Δ meeting T′, this procedure, which reads only the current holes, clears T with at most Σ_j (|E_j| + e_j) letters: for j = 1, ..., m, if t_j is occupied, take a hole z ∈ R_j ∖ P_j and apply W_z, then E_j.

Proof. By induction P_j consists of holes before step j; the words used fix P_j pointwise, so P_j stays vacant. If t_j is occupied, the holes of Δ_j outside P_j number at least h_Δ − |P_j ∩ Δ_j| ≥ 1 + u_j, none is t_j, and at most u_j lie outside R_j, so a hole z ∈ R_j ∖ P_j exists. W_z takes it to y_j and E_j to t_j. Hole counts per orbit never change, so the threshold is checked once.

- If u_j = 0 for every j, then h_Δ = |T ∩ Δ|: the necessary counts are sufficient, with an explicit bound.
- If some u_j > 0, the certificate proves only the higher threshold h_Δ; the counts alone are then not sufficient in general. Example (abstract, from the independent review): G = Sym(4) wr Sym(2) on {1, ..., 8} with blocks {1, 2, 3, 4} and {5, 6, 7, 8}, T = {1, 2}, V = {1, 5}. The counts hold, every image of V has one hole in each block, so T cannot be cleared; a valid certificate exists with E_1 = (1 3), E_2 = (2 3), R_1 = R_2 = {3}, u_1 = 6, u_2 = 5 and h_Δ = 7.
- Route words in `buffer.py`: letters whose caps avoid P_j; the words E_k for k ≥ j (they fix P_k ⊇ P_j), both ways; conjugates g⁻¹ l g with g one or two letters and l a letter whose cap avoids g(P_j), each checked on the full permutation.

## 5. Chambers locked to a rigid block

**Lock lemma (Proved).** Let B be a rigid off component of a depth-one state (Lemma 50), I_B the grips whose caps contain every member of B, and S_B the grips whose cut some member straddles. Let x be a home chamber with I_B ⊆ caps(x) ⊆ I_B ∪ S_B, where caps(x) is the set of caps containing x. Under every admissible retained word, whatever the other off pieces do, the lattice piece that was in x (or the hole) is afterwards in the chamber k_B(x), where k_B ∈ K is the multiplier B has then.

Proof. Induction on the letters. Before a letter (g, m) with m ∈ K, let k = k_B and suppose the content of x is in k(x). Now B lies inside exactly the caps k(I_B), its members straddle exactly the cuts k(S_B), and caps(k(x)) = k(caps(x)), so k(I_B) ⊆ caps(k(x)) ⊆ k(I_B) ∪ k(S_B). No piece straddles g, so g ∉ k(S_B), and B lies wholly inside or wholly outside g (Lemma 50). If inside, g ∈ k(I_B) ⊆ caps(k(x)): the content moves to m k(x) and B's multiplier becomes m k. If outside, g is in neither k(I_B) nor k(S_B), so not in caps(k(x)): the content stays, and so does k.

Consequences.
- Whether a locked chamber holds a lattice piece, relative to its block, is the same in every state reached by retained words, whatever other blocks move together. When B is back at its pose, x holds exactly what it held.
- A locked chamber is fixed by G(Ξ): SAFE-ALL letters never move B.
- If a locked chamber x lies in a target T, no retained preparation that ends with B at its starting pose can change whether x is vacant. A preparation for a fixed restoration word must end so (the word expects the off geometry it starts from); SAFE-ALL words never move B, and the setups w·v·w⁻¹ of section 8 return it. A retained word that leaves B moved by k_B ≠ 1 has carried x's content to k_B(x), and x may then hold what another chamber held. Only a twist outside K that moves B, or that moves the chamber's content while B stays, changes the content relative to B.
- Computed (`blocks.py`): at X0 the five chambers of T ∩ Fix(G), 119124, 119141, 119151, 128877 and 128888, are exactly the target chambers locked to the 86-piece block; at the last buffer step of the owner's construction (prefix 39,988, 86 off pieces, one rigid block, T the 86 chambers of the class-2 cell {162, 237}), the 13 chambers of T ∩ Fix(G), 42203, 42204, 42206, 42211, 42212, 42213, 47671, 47672, 47686, 47687, 47697, 47702 and 57072, are exactly the target chambers locked to that block; all are vacant at both states.

## 6. BUFFER-PROGRESS with a checkable inner test

- Lemma 46 decides at a state X whether an own-class action u (Lemma 44) followed by a BUFFER-VACANCY word and u⁻¹ restores a nonempty set. Its step (ii) enumerates the orbit of the hole set at Y = uX under the SAFE-ALL letters (up to C(N, h) sets).
- **Checkable replacement for step (ii).** BV(Y, T): from Y compute A(Ξ(Y)), G, its orbits and Fix(G); check (N1); then either (J) every orbit meeting T ∖ Fix(G) has a Jordan certificate and (N2) holds, or (P) a complete peeling certificate (Theorem B, every chamber of T ∖ Fix(G) peeled once) is valid and |V(Y) ∩ Δ| ≥ h_Δ for every orbit Δ meeting T ∖ Fix(G). BV(Y, T) implies BUFFER-VACANCY(Y, T) in both forms. Under (J) the two are equivalent (Corollary A1). Under (P) they are equivalent when every u_j is 0 (then h_Δ = |T ∩ Δ|), and the clearing word is then found greedily and bounded; when some u_j > 0, (P) is sufficient only (section 4). When an orbit has neither certificate, the counts are only necessary (Proposition N) and the finite search of Lemma 46 still decides.
- Everything is read from Y: the alphabet from the exact off geometry, the holes from the lattice pieces. A certificate is a list of native words, checked by recomputing their permutations; one found for Ξ holds on all of F(Ξ).
- **BUFFER-PROGRESS, certified form.** From the start of a run, at every state X_i with pieces off the lattice that the run reaches, some own-class action u_i with its bad-return target T(u_i) (Lemma 44) satisfies BV(u_i X_i, T(u_i)). Because BV implies BUFFER-VACANCY, the certified form implies BUFFER-PROGRESS. It is a strengthening, not an equivalent restatement: BUFFER-VACANCY can hold where no certificate exists (abstract example from the independent review: the regular action of the cyclic group of order four with V = T = {1, 2}; the empty word clears T, and there is neither a Jordan certificate nor a complete peeling certificate). There Lemma 46's finite search still decides. **Open (general E):** whether BUFFER-PROGRESS, or its certified form, holds at every reachable depth-one state. Nothing here implies it; section 7 records instances and section 8 a state where the SAFE-ALL form fails and a setup is needed.
- **Computed** (`buffer.py`): at X0 and at the five buffer steps of the owner's 41,457-twist construction (Lemma 47), every orbit meeting T ∖ Fix(G) has a Jordan certificate, and (N1) and (N2) hold.

| State (prefix of the construction) | Digest (first 16) | Off | T: class, cell | \|T\| | Vacant in T | SAFE-ALL grips | Orbits meeting T | Fixed (all locked, all vacant) | Smallest other orbit |
| --- | --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| X0 (17,670) | 3c086301091a2dad | 277 | 0, {369, 507} | 86 | 83 | 570 | 24 | 5 | 1,439 |
| v14 (22,318) | 5689234045dad477 | 186 | 0, {449, 452, 507} | 111 | 82 | 570 | 19 | 0 | 1,440 |
| v15 (26,443) | 88d079678c92fb99 | 149 | 0, {154, 369} | 86 | 74 | 580 | 24 | 5 | 1,440 |
| v16 (27,426) | b9cce35024af3f7b | 131 | 2, {140, 283, 331} | 111 | 68 | 579 | 19 | 0 | 1,440 |
| v17 (36,636) | 6a92a40834fb3f46 | 90 | 2, {162, 237, 239, 392} | 87 | 70 | 586 | 7 | 0 | 120 |
| v18 (39,988) | dd5ca52b66accc6d | 86 | 2, {162, 237} | 86 | 83 | 590 | 32 | 13 | 1,440 |

### 6.1 Restoring in sequence: what a restoration keeps

Setting of this section.
- Ω is the fixed set of 177,120 home chambers; it does not depend on the state. A letter (g, m) acts on Ω by a fixed permutation: the chambers of the cap of g move by m, every other chamber is fixed. The support of a word w (the chambers π_w moves) lies in the union of the caps of its letters.
- At a state Y, the content of a chamber is the lattice piece in it, or nothing (a hole; V(Y) is the set of holes). Off pieces have no chamber.
- Write Ξ′ ≤ Ξ when every off piece of Ξ′ is off in Ξ with the same exact pose matrix: some pieces of Ξ came back to the lattice (call them released) and the others kept their exact poses. Nothing is assumed about how the later state was reached or about its holes. This is the end-state comparison the owner asks of every new construction.

**Lemma M (Proved).** Let Ξ′ ≤ Ξ.
- (M1) Alphabet. A(Ξ) ⊆ A(Ξ′), every letter with the same permutation of Ω. So G(Ξ) ≤ G(Ξ′), every orbit of G(Ξ′) is a union of orbits of G(Ξ), and Fix(G(Ξ′)) ⊆ Fix(G(Ξ)).
- (M2) Every step legal. Let Y ∈ F(Ξ′) (off geometry exactly Ξ′, any holes) and w a word of A(Ξ). Every letter of w is admissible at the state where it is applied. Every state along w is in F(Ξ′): the same pieces off, each at its exact pose at Y, every lattice piece on the lattice. After a prefix u the holes are π_u(V(Y)).
- (M3) Released points. A released piece is now the content of a chamber, like any other lattice piece: a word of A(Ξ) whose support contains that chamber moves it, keeping it on the lattice but not in its chamber. The letters that the release made SAFE-ALL, A(Ξ′) ∖ A(Ξ), are not words of A(Ξ); a chamber that only these letters move is fixed by G(Ξ), and the certificates of Ξ say nothing about moving it.
- (M4) Peeling certificates. A peeling certificate made for Ξ and a target T stays valid at every Y ∈ F(Ξ′), with the same words, routes, thresholds h_Δ (Δ an orbit of G(Ξ)) and bound. If T ∩ Fix(G(Ξ)) ⊆ V(Y) and |V(Y) ∩ Δ| ≥ h_Δ for every orbit Δ of G(Ξ) meeting T ∖ Fix(G(Ξ)), the greedy procedure of Theorem B clears T at Y with words of A(Ξ).
- (M5) Jordan certificates. A Jordan certificate (w, p) for an orbit Δ of G(Ξ) stays one for Δ and G(Ξ) at every state of F(Ξ′): G(Ξ) still contains Alt(Δ) acting on Δ and fixing every other chamber, realized by words that (M2) makes legal there. It is also one for the orbit Δ′ ⊇ Δ of G(Ξ′) when p > |Δ′|/2, in particular when Δ′ = Δ.
- (M6) Necessary counts. If the holes satisfy (N1) and (N2) for T with respect to G(Ξ), they satisfy them with respect to G(Ξ′).

Proof. (M1) Let (g, m) be a letter of A(Ξ). No off piece of Ξ straddles the cut of g or lies in its cap, so no off piece of Ξ′ does (the same pieces at the same poses, or fewer), and no released piece straddles it, because lattice pieces never straddle a cut. So g qualifies at Ξ′, with the same menu elements in K, and the permutation, defined by the cap and the menu element alone, is unchanged. The statements on orbits and fixed chambers follow from G(Ξ) ≤ G(Ξ′). (M2) By (M1) every letter of w is in A(Ξ′). Lemma 41 at Ξ′, applied letter by letter: a letter of A(Ξ′) is admissible at every state of F(Ξ′) (no piece straddles its cut), moves only the lattice pieces in its cap (no off piece is there), by an element of K, and so leads to a state of F(Ξ′) whose holes are its permutation applied to the holes before. (M3) The first sentence is (M2) for one letter. The rest holds because π_w, for a word w of A(Ξ), is a product of permutations of letters of A(Ξ). (M4) Conditions (a) and (b) of Theorem B concern only the permutations of words of A(Ξ) and the set T; none mentions holes or off pieces, so by (M1) they still hold. The proof of Theorem B uses only these conditions, that its words are SAFE-ALL at the current state (M2), that words of A(Ξ) keep the number of holes in every orbit of G(Ξ) (their permutations preserve each orbit), and that they fix T ∩ Fix(G(Ξ)). (M5) The hypotheses of Theorem A for Δ and G(Ξ) concern only words of A(Ξ) and their permutations, which do not change. For Δ′: w is a word of A(Ξ′); π_w has exactly one cycle of length divisible by p on Ω, and it lies in Δ ⊆ Δ′; p ≤ |Δ| − 3 ≤ |Δ′| − 3, and p > |Δ′|/2 by assumption. (M6) An orbit Δ′ of G(Ξ′) is a disjoint union of orbits Δ_i of G(Ξ), so |V ∩ Δ′| = Σ_i |V ∩ Δ_i| ≥ Σ_i |T ∩ Δ_i| = |T ∩ Δ′|; and T ∩ Fix(G(Ξ′)) ⊆ T ∩ Fix(G(Ξ)).

What Lemma M does not give.
- Merging is not a certificate. When orbits Δ_1, ..., Δ_k of G(Ξ) merge into one orbit Δ′ of G(Ξ′), G(Ξ′) need not contain Alt(Δ′), even when every Δ_i has a Jordan certificate. For example, if the new letters only exchange the Δ_i as wholes, G(Ξ′) preserves the partition into the Δ_i and is not primitive on Δ′ (an abstract possibility, not checked in the retained geometry). Alt(Δ′) needs its own certificate: (M5) with p > |Δ′|/2, or one computed at Ξ′. The independent alternating action that Theorem A gives is on the old orbits Δ_i, and the counts it needs are on them.
- Legal old words do not supply holes. A restoration fills chambers: the number of holes drops by the number of released pieces, and which holes remain depends on the restoration. The conditions the next target needs, (N1) with respect to G(Ξ) and the thresholds on the orbits of G(Ξ), must be checked at the new state; they can fail, as at X5 (section 8.2), where only a setup outside A(Ξ) re-established them.
- Recomputing at the later state guarantees less than it seems. (M6) transfers only the necessary counts. A certificate for a coarser orbit of G(Ξ′) must be found again, and none need exist. Abstract example from the independent review (finding MATH-M-01; a permutation-level example, not realized in the retained geometry): when the trivial group grows to the regular action of the cyclic group of order four with V = T = {1, 2}, (N1) and (N2) hold and there is no complete peeling certificate.

**Corollary S (Proved from Lemma M).** Let a restoration pass through depth-one states X_0, X_1, ..., X_r with Ξ(X_(i+1)) ≤ Ξ(X_i) for every i. Only these states are compared: between them a step may take lattice pieces off or move off pieces, as long as it ends so. The relation is transitive, so Ξ(X_i) ≤ Ξ(X_0) for every i. Hence every word of A(Ξ(X_0)) is legal at every X_i as in (M2), and every peeling or Jordan certificate computed once at X_0 stays valid at every X_i on the orbits of G(Ξ(X_0)), with the same words, thresholds and bounds. What has to be re-established at X_i is the hole set alone: (N1) for the target with respect to G(Ξ(X_0)), and the thresholds of X_0's certificate (or (N2), for orbits with Jordan certificates) on the orbits of G(Ξ(X_0)).

- Not covered: a step whose end state leaves a remaining off piece at another exact pose, for instance a block moved by a retained multiplier as between the buffer steps of the owner's construction. Then Ξ(X_(i+1)) ≤ Ξ(X_i) fails, and certificates must be recomputed at the new state.
- Still open for sequential restoration: whether the hole counts can always be re-established (general E). When the SAFE-ALL counts fail, as at X5, a one-letter setup w v w⁻¹ moves holes across orbits; the flow bound of section 8.2 is a necessary test and a certificate at Y a sufficient one, both read from the current geometry.
- **Computed** (`monocheck.py`; exact): three restorations that keep every remaining off pose, each checked for Ξ_end ≤ Ξ_start, for every SAFE-ALL letter of the start being one at the end with the same element and cap permutation, for coarsening orbits and for a shrinking fixed set; all hold.

| Restoration | Start → end (digests, first 16) | Off | SAFE-ALL letters | Orbits | Fixed chambers | Result |
| --- | --- | --- | --- | --- | --- | --- |
| X5 recovery (2,564 twists, section 8.2) | f7506db887e80200 → 72d09dec34ec5d38 | 277 → 191 | 6,270 → 6,281 | 1,110 → 1,085 | 1,075 → 1,050 | `results/outside/x5-recovery-mono.json` |
| Owner's v17 return 1 (82 twists) | b013c489f70c18c4 → a70175cb55ae4a03 | 499 → 388 | 6,270 → 6,270 | 1,110 → 1,110 | 1,075 → 1,075 | `results/v17/mono-4214-return-1.json` |
| Owner's v17 return 2 (88 twists) | 8eac8877d3373961 → ef33f3c0e6b2fdb5 | 388 → 277 | 6,270 → 6,270 | 1,110 → 1,110 | 1,075 → 1,075 | `results/v17/mono-4214-return-2.json` |

- In the owner's two returns the alphabet, the group and its orbits do not change at all: the released pieces opened no new SAFE-ALL grip. The X5 recovery opened one grip (11 letters); the orbits fell from 1,110 to 1,085 and the fixed chambers from 1,075 to 1,050. These computations certify nothing on a merged orbit.

### 6.2 The owner's v17 states (words of 4,214 and 8,372 twists)

- Source: the owner's package v17 (12 October 2026). Word 4214: an 82-twist return, a 4,044-twist preparation and an 88-twist return, 499 → 388 → 277 off. Word 8372, from another 499 state: a 3,938-twist preparation, the 82-twist return, a 4,264-twist preparation and the 88-twist return. Every boundary state comes as a current-only pose table; every computation below starts from these tables, never from older states with the same off count, and uses no generation history.
- Targets: the package lists, for each return word, the 111 home chambers that must be vacant before it. **Computed** (from the chamber signatures): they are exactly the class-0 cells T1 = {369, 507, 543} (return 1) and T2 = {369, 448, 507} (return 2).
- Same family. **Computed** (`buffer.py`): the two 499 states have the same off geometry (fingerprint 3bd9d53a…), and so do the two 388 states (b68681e5…). Each pair lies in one family F(Ξ) and shares A(Ξ), G, the orbits and every certificate; only the holes differ. All four have 570 SAFE-ALL grips (6,270 letters), 1,110 orbits and 1,075 fixed chambers, and no target chamber is fixed.

| State (package file) | Digest (first 16) | Off | Target | Vacant in target | Orbits meeting target | Jordan certificates | Peeling certificate | Preparation from the current holes | Owner's preparation |
| --- | --- | ---: | --- | ---: | ---: | --- | --- | --- | ---: |
| 4214 start | b013c489f70c18c4 | 499 | T1 | 111 | 19 | 19 of 19 | not found (search stops at 99 of 111) | none needed | none |
| 4214 after 82 | a70175cb55ae4a03 | 388 | T2 | 30 | 19 | 19 of 19 | complete, every u_j = 0, bound 1,443 | 535 letters | 4,044 |
| 8372 start | 23f812afb3d09c52 | 499 | T1 | 7 (T2: 3) | 19 | 19 of 19 | not found (search stops at 99 of 111) | no bound | 3,938 |
| 8372 after 4,020 | 967053b1f9058ab5 | 388 | T2 | 1 | 19 | 19 of 19 | the 4214 certificate, reused (same family) | 649 letters | 4,264 |

- **Proved** at the 8372 start (Corollary A1): (N1) and (N2) hold for T1 and every orbit meeting T1 has a Jordan certificate, so some SAFE-ALL word clears T1. This agrees with the owner's 3,938-twist preparation; it gives no length bound. Our peeling search (single letters and conjugated commutators of depth two, chosen greedily) stops at 99 of the 111 chambers of T1 at both 499 states. That is a limit of the search, not an obstruction.
- **Computed** (`buffer.py --certificate --demo 0 1 --suffix <return 2>`; `results/v17/4214-after-82-388.json`, `results/v17/8372-after-4020-388.json`): for T2 the greedy preparation reads only the current holes. At the 4214 state it takes 535 letters from 30 vacant target chambers (76 greedy steps); at the 8372 state it takes 649 letters from 1 (90 greedy steps), with the certificate computed at the 4214 state and reused after its geometry and witness checks. Both keep every off pose at every prefix, and their predicted holes equal the actual ones. The owner's 88-twist return word follows: depth one at all 88 prefixes, 111 pieces restored, no lattice piece of the source off at the end, and the 277 remaining off pieces at their exact source poses. End digests: 4f7f0f679890f404ea154aa9e98c8eb921a1a9c958408b8833e47f1d30887db8 and af6162c242a542a8bf22a545a7a6ca11a70b91e7b386173872318cfcdd4bc162. These differ from the owner's end digests: the preparations differ, so the lattice pieces end in other chambers. The protection checked is the owner's: lattice membership and the exact poses of the remaining off pieces.
- With Lemma M (the return-step rows above): the certificates of the 499 states remain valid at the 388 and 277 states of word 4214, on unchanged orbits.


## 7. Relaxing Lemma 49

- Family F(Ξ_X0): X0's 277 off pieces at their exact poses, every other piece on the lattice, any hole set. T: the 86 chambers of the class-0 cell {369, 507}.
- **Computed** (`buffer.py` at X0): 24 orbits meet T: the five fixed chambers above and 19 orbits of sizes 1,439 to 7,200 (table below). The peeling certificate, with its route gaps closed by conjugates, reaches every point of every row (all u_j = 0), with Σ_j (|E_j| + e_j) = 899 (computed twice: by the integrator's script and by `buffer.py`).
- **Consequence (Proved from Theorem B and Proposition N).** For every Y ∈ F(Ξ_X0), a SAFE-ALL word clears T if and only if the five locked chambers are vacant and every one of the 19 orbits has at least as many holes as target chambers. Then the greedy procedure, reading only Y, clears T in at most 899 letters, and word 11 (2,225 twists) restores the 86 pieces as in Lemma 49: total at most 3,124.
- Against Lemma 49 ((49.1): 83 named vacant chambers, thresholds 4, 8 and 6 on three orbits, bound 1,030): the only named chambers left are the five that no retained word can change (section 5), and the condition is necessary as well as sufficient for SAFE-ALL preparations.

| Orbit (smallest chamber order) | Size | Target chambers | Holes at X0 | Jordan prime |
| ---: | ---: | ---: | ---: | ---: |
| 13 | 1,439 | 2 | 3 | 877 |
| 19 | 2,396 | 2 | 3 | 1,759 |
| 14 | 3,590 | 3 | 4 | 1,951 |
| 22 | 3,598 | 3 | 6 | 1,913 |
| 27 | 3,600 | 1 | 5 | 2,837 |
| 8 | 7,158 | 1 | 1 | 6,761 |
| 10 | 7,174 | 3 | 3 | 3,727 |
| 11 | 7,184 | 6 | 7 | 3,889 |
| 15 | 7,185 | 8 | 11 | 3,877 |
| 18 | 7,187 | 4 | 6 | 4,217 |
| 12 | 7,191 | 8 | 11 | 3,929 |
| 16 | 7,192 | 6 | 10 | 4,253 |
| 21 | 7,193 | 8 | 13 | 4,363 |
| 20 | 7,195 | 4 | 8 | 4,253 |
| 17 | 7,196 | 8 | 14 | 4,703 |
| 23 | 7,197 | 6 | 12 | 5,923 |
| 24 | 7,198 | 4 | 11 | 5,077 |
| 25 | 7,199 | 2 | 11 | 5,503 |
| 28 | 7,200 | 2 | 22 | 6,421 |

- **Computed** (`buffer.py --demo 300 1 --suffix`, `results/buffer/x0-demo-300-1.json`): a 300-letter SAFE-ALL scramble of X0 (seed 1; 26 of the 86 target chambers vacant afterwards). A fresh constructor that sees only the exported pose table and the saved certificate prepared in 415 letters (64 greedy steps, one for each target chamber found occupied at its turn), kept all 277 off poses at every prefix and cleared T, with predicted holes equal to the actual ones. Word 11 then ran with depth one at all 2,225 prefixes: 86 pieces restored, no lattice piece taken off, the 191 remaining off pieces at their exact poses (end digest d8aa72820bc09696a5f9a3f9994306e6966789857f3aac113ba865196f1661e6).

## 8. Item 2: outside the condition

The owner's question: build legal, reachable depth-one states outside the condition of section 7, and see whether blocks that cannot move alone (Lemma 51, "frozen") gain a way to recovery when a few blocks move together. Generation histories below are external reachability evidence only; every recovery reads the current state alone.

### 8.1 Frozen blocks move with one partner

- Isolated transport (Lemma 51): one block moves by retained letters, every other off piece keeps its exact pose at every prefix. At the E2 end five blocks move and eight are frozen.
- Coupled transport of a set S of blocks: the blocks of S may move together (each by its own left multiplier in K, all blocks inside the cap of a letter moving by its matrix), every block outside S keeps its exact pose at every prefix. A letter is admissible exactly when every block outside S is strictly outside its cap and every block of S lies wholly inside or wholly outside it (Lemmas 50 and 51); breadth-first search over the multiplier tuples decides reachability exactly.
- A frozen block B is blocked at every grip containing it by other blocks inside the cap or straddling the cut. The fewest such blocks at one grip ("partners") at the E2 end: block 1 (43 pieces): {2}; block 3 (3): {1, 2}; block 7 (29): {10, 11} or {11, 12}; block 8 (45): {6}; block 9 (86): four blocks; block 10 (111): {7, 11}; block 11 (87): {7, 10} or {7, 12}; block 12 (111): {7, 11}. At X0, after blocks 0, 4, 6, 8, 10 and 12 have been restored, blocks 7 and 11 are each the other's only partner.
- **Computed** (`blocks.py --coupled`, exact):

| State | S (owner numbering) | Pieces | Reachable tuples | Complete | Multipliers of each block with the other at its pose |
| --- | --- | --- | ---: | --- | --- |
| E2 end | {1, 2} | 43 + 17 | at least 51,494,844 | no (search stopped at depth 9; depth 9 still had 76,043 new tuples) | block 1 (frozen alone): at least 7,166 non-identity; block 2: at least 7,184 |
| X0 | {7, 11} | 29 + 87 | 51,732,048 | yes | block 7: 7,190; block 11: 7,193 (both frozen alone) |

- So "frozen" describes the isolated family, not the block: with one partner moving along and returning, these frozen blocks reach almost every multiplier in K. Blocks whose fewest partners are two or more (3, 7, 9, 10, 11 and 12 at the E2 end) were not searched exhaustively (the state space is 7,200^3 or larger).

### 8.2 X5: a reachable state where no SAFE-ALL preparation exists

- Generator (external evidence only): from X0, the retained letter w = (319, 7) moves blocks 7 and 11 together (116 pieces; both frozen alone). At Y = wX0 the SAFE-ALL group changes, and the orbit of chamber 119126 (at X0 its orbit 8 of size 7,158 holds one hole and one target chamber) joins an orbit containing chamber 119050, which no target orbit meets at X0. A 4-letter SAFE-ALL route at Y moves that hole, then w⁻¹ = (319, 6). Word: (319, 7) (528, 2) (505, 10) (410, 1) (294, 1) (319, 6). Every prefix legal and depth one; at the end every off piece has its exact X0 pose. Independent replay (`replay_check.py`, `results/outside/x5-generator-replay.json`): 6 twists admissible, depth one at every prefix, end digest X5, no source lattice piece off (collateral 0), no off pose changed.
- X5: digest f7506db887e80200202b6c29a43a2068cbc808d43b604345e345505029cdcd4f, 277 off pieces, the off geometry Ξ_X0. The five locked target chambers are vacant. Orbit counts against targets: orbit 8 (size 7,158): 0 holes, 1 target chamber; orbit 10 (size 7,174): 2 holes, 3 target chambers; the other 17 orbits as at X0.
- **Proved** (Proposition N): no SAFE-ALL word clears T at X5, of any length. X5 lies outside the condition of section 7, and outside Lemma 49's family.
- Recovery, reading only X5 (`setup_flow.py`): compute A(Ξ), the orbits and the certificate; the counts fail, so search setups. A setup is an admissible retained letter w that moves off blocks (its cut not straddled, off pieces inside its cap), a SAFE-ALL word v at Y = wX5, and w⁻¹; it returns every off piece to its exact pose and changes the holes by a chamber permutation. For each letter passing the flow test below, the script takes a maximum flow, chooses for every unit of flow O → D a chamber of O whose preimage under w lies in D (chambers of w(T) first), builds a peeling certificate for these chambers at Y and its greedy word v (reading only the holes of Y), applies w⁻¹ and checks the condition of section 7. **Constructed** (exact; `results/outside/x5-setup-recovery.json`): the first letter tried, w = (315, 1), moves blocks 7 and 11 (116 pieces) together; its element has order two, so w⁻¹ = (315, 1). The flow reaches the demand, 81; the 81 chambers chosen at Y all lie in w(T); 99 SAFE-ALL letters at Y touch them; their peeling certificate at Y has bound 942, and its greedy word v, read from the holes of Y, has 337 letters. After w⁻¹ the condition of section 7 holds with no deficit, and T is already vacant (v emptied w(T ∖ Fix) at Y; the five fixed chambers are locked to block 9, which w does not move), so the source certificate's preparation is empty. Word 11 (2,225 twists, `e-chain-104/11-word-restore-86.json`) follows. The script replays all 2,564 twists from X5 with every check of its docstring: every twist admissible, depth one at every prefix, lattice flags unchanged and the off pieces outside blocks 7 and 11 at their exact poses during the setup, every off pose equal to X5's after w⁻¹, predicted holes equal to the actual ones, T vacant. End: 86 pieces restored, 191 off, every lattice piece of X5 on the lattice, the 191 remaining off pieces at their exact X5 poses. Digests: after the setup 21ae105844d43ba28089fa4a0a7027fd3a6d52f11fbd534130d27718cf9202c5, end 72d09dec34ec5d388ebbfdaa880b432ccc34efcdbdbef072741d9a1d5413fcab. The chronological word (2,564 (grip, menu_index) pairs) is `results/outside/x5-recovery-word.json`. Independent replay (`replay_check.py`, `results/outside/x5-recovery-replay.json`): all 2,564 twists admissible, depth one at all 2,564 prefixes, input and output digests as recorded, 86 pieces restored, collateral 0, no remaining off pose changed, 191 off at the end. Inside word 11 the off count rises to 8,440 and comes back.
- Which setup letters can work, exactly (`setup_flow.py`): w maps the holes of X onto the holes of Y = wX, V(Y) = π_w(V(X)). A SAFE-ALL word v at Y keeps the number of holes in every orbit of G(Y) (Proposition N at Y), and w⁻¹ maps a hole in the Y-orbit O to a chamber x with w(x) ∈ O. The X-orbit counts after w·v·w⁻¹ can be met only if a flow exists with supply s_O = |V(Y) ∩ O| = |{x ∈ V(X) : w(x) ∈ O}| at each Y-orbit O (the holes of Y, not of X), capacity O → D = |{x ∈ D : w(x) ∈ O}|, and demand |T ∩ D| at every X-orbit D meeting T ∖ Fix. **Proved:** if the maximum flow is below the total demand, no setup with that letter works, whatever v is. If it is reached and every Y-orbit involved carries a Jordan certificate (Theorem A at Y), some v achieves the counts; the fixed target chambers stay vacant, because they are locked to the 86-piece block, which w either leaves alone or moves and w⁻¹ returns (lock lemma).
- **Computed** (reading only X5; first by propagating block statuses, which is exact because blocks are rigid: a block inside the cap moves by the letter's element and has at grip g the status it had at that element's inverse image of g; then directly, by `setup_flow.py --classify`, which applies every letter on the simulator, recomputes G(Y) and undoes it (`results/outside/x5-setup-classify.json`); the two agree letter by letter in the flow, the SAFE-ALL grips at Y and the moved pieces): X5 has 187 admissible retained letters that move off blocks (17 grips, 11 letters each). Total demand 81; the maximum flow is 81 for 22 letters, 80 for the 11 letters at grip 507 and 79 for the other 154; the 165 letters below 81 are therefore **proved** useless as one-letter setups.

| Grip | Blocks inside (owner numbering) | Pieces | Letters passing the flow test |
| ---: | --- | ---: | ---: |
| 315 | 7, 11 | 116 | 9 of 11 |
| 319 | 7, 11 | 116 | 9 of 11 |
| 369 | 5, 9, 11 | 185 | 4 of 11 |
| 393, 427, 507 | 7, 9, 11 | 202 | 0 of 33 |
| 154, 188, 268, 277 | 5 | 12 | 0 of 44 |
| 162, 183, 237, 290, 301 | 1, 2, 3 | 63 | 0 of 55 |
| 283 | 1, 2 | 60 | 0 of 11 |
| 392 | 2 | 17 | 0 of 11 |

- Every useful setup moves block 11 together with block 7, or with blocks 5 and 9; no letter moving one block alone helps, and blocks 7 and 11 are each frozen alone. The recovery above uses the first passing letter, which moves blocks 7 and 11 together and returns them.

### 8.3 What coupled motion cannot change: chambers locked to a block

**Corollary L (Proved from the lock lemma).** Let B be a rigid off block of a depth-one state and q a lattice piece in a chamber x locked to B. In any sequence of admissible twists, let τ be the first twist that moves B by an element outside K. Before τ, q stays on the lattice, in the chamber k_B(x). The twist τ takes q off the lattice.

Proof. Before τ, every twist that moves B moves it by an element of K, so the lock lemma's induction applies to every twist, retained or not: a twist whose cap holds k_B(x) holds B (its grip lies in caps(k_B(x)) ⊆ k_B(I_B) ∪ k_B(S_B), and a grip in k_B(S_B) is straddled, so not admissible); a twist that does not move B does not move q. At τ, q is in the cap of τ with B, and τ moves it by an element m ∉ K; q's pose was in K, so m times it is not.

- So any word that brings a member of B back to the lattice (that needs a move of B outside K) takes every lattice piece in B's locked chambers off the lattice on the way. A restoration of B that keeps the source lattice at every prefix is impossible; one that protects the source lattice at its endpoint must bring these pieces back afterwards. No retained motion, coupled or isolated, changes this: the content of a locked chamber moves with B.
- A target chamber locked to a block and occupied is out of reach for every retained preparation (coupled ones included): when B is back at its pose the chamber holds the same piece.
- **Computed** (`blocks.py`): at X0 the 3-piece block 3 has three locked chambers, 57073, 57074 and 57075, all occupied, and the 12-piece block 5 has two, 58358 and 58361, both occupied; the 86-piece block 9 has the five vacant locked target chambers of section 5.
- **Computed** (`lockcheck.py`, along the owner's 41,457-twist construction from the E2 end, `results/outside/lockcheck-e2-to-lattice.json`): blocks and locked chambers registered at the prefixes 0, 17,670, 22,318, 26,443, 27,426, 36,636 and 39,988 (35 registrations), then checked at every twist: 0 violations of the lock lemma or of Corollary L. Two blocks have occupied locked chambers: the 3-piece block (three chambers, holding the lattice pieces 57220, 47840 and 47855) and the 12-piece block (two chambers, pieces 53589 and 31985). There are 15 forcing events, three for each of these five pieces: at each first move of one of the two blocks outside K after a checkpoint (prefixes 9,654, 17,884 and 24,219 for the 12-piece block; 27,388, 36,568 and 38,553 for the 3-piece block) exactly these pieces went off the lattice, as Corollary L says, and the construction brought them back 2 to 16 twists later (prefixes 9,659, 17,886 and 24,221; 27,393, 36,573 and 38,569). The end digest matches the owner's.

### 8.4 Failure classes at X5

| Action family (from X5) | Outcome | Kind |
| --- | --- | --- |
| SAFE-ALL words of any length (Lemma 41), then word 11 | impossible | proved (Proposition N) |
| One block-moving retained letter w, any SAFE-ALL word at wX5, then w⁻¹ | impossible for 165 of the 187 letters; the flow bound is met by 22 | proved per letter (flow bound); for the first of the 22, (315, 1), constructed (next row) |
| The same family, flow-guided construction (`setup_flow.py`) | constructed with the first letter tried, (315, 1): 86 pieces restored, collateral 0 | construction; exact replay by the script and an independent replay (`replay_check.py`) |
| Any legal word | X5 is recoverable: it is reachable from the E2 end, and the owner's construction restores the E2 end to the lattice | not an obstruction; the generation history is used only as this evidence |
| Restoring a block with an occupied locked chamber without taking a lattice piece off at some prefix | impossible | proved (Corollary L) |

### 8.5 The six unlock windows of the owner's construction

The owner asked (v17) for the six unlock mechanisms of section 8.3 in full: exact start tables or reproducible prefixes, the complete short words, the endpoint digests, the lattice pieces each step takes off and where each is first back, the remaining off poses, and the geometric hypotheses that might generalize.

Convention. Every number in this section is an executed-prefix count of the owner's complete 41,457-twist word from the E2 end (input digest bb72a668da3387c62eda6e079a9b251a2909b9abea0b0079bcda24d180f85686, output digest be0553b5c0bc3af1520eb470f6bf132d375930d9b2472ba58c0b431d90f9d289): prefix p is the state after the first p twists, and twist p (counted from 1) is word[p − 1]. Section 8.3 names two kinds of numbers.
- Forcing twists n = 9,654, 17,884, 24,219 (12-piece block) and 27,388, 36,568, 38,553 (3-piece block): twist n is the first twist after a checkpoint that moves the block by an element outside K. Prefix n − 1 is the last state with the forced pieces (the lattice pieces in the block's occupied locked chambers) on the lattice, prefix n the first without them.
- First returns b = 9,659, 17,886, 24,221, 27,393, 36,573 and 38,569: the first prefix ≥ n at which all forced pieces are on the lattice together.
- b − n (2 to 16 twists) is not the length of a witness. When the forced pieces are back, other lattice pieces that the same excursion took off can still be off, and the excursion can begin before n.

Definitions (`unlock.py`).
- Anchor s < n: the prefix just before the first letter of the commutator word whose letter n is the forcing twist; the integrator chose it from the word's structure (below). For window 17,884 it is the start of the owner's word 11.
- Closing prefix e: the first prefix ≥ n at which every lattice piece of prefix s is on the lattice again.
- Witness: word[s:e], the twists s + 1 to e (e − s twists). It contains the forcing twist n, the step at which the forced pieces first leave the lattice, and the n − s − 1 twists before it, which already take other lattice pieces off.
- Start table: the current-only pose table of prefix s (`buffer.export_pose_table`). Loaded with `orient.start(table)`, it reproduces the digest at s, and replaying the witness from it reproduces the window. The six tables are in the handoff package; the prefix s of the owner's word reproduces the same states.

Table 8.5a (**Computed**, `unlock.py`; report `results/outside/unlock-e2-to-lattice.json`). "Off" counts the pieces off the lattice; "restored" counts the pieces off at s and on at e; the last column compares the off pieces at e with those at s: off at both at the same exact pose, off at both at another pose, off at e only.

| Forcing twist n (block) | Anchor s (off) | n − s | First return b | b − n | Closing prefix e (off) | Witness e − s | Peak off (prefix) | Restored at e | Off at e: at the pose of s / moved / newly off |
| --- | --- | ---: | ---: | ---: | --- | ---: | --- | --- | --- |
| 9,654 (12-piece) | 9,649 (499) | 5 | 9,659 | 5 | 9,688 (388) | 39 | 6,093 (9,656) | 111 (block of 111) | 388 / 0 / 0 |
| 17,884 (12-piece) | 17,882 (277) | 2 | 17,886 | 2 | 20,107 (191) | 2,225 | 8,462 (17,895) | 86 (block of 86) | 191 / 0 / 0 |
| 24,219 (12-piece) | 24,218 (75) | 1 | 24,221 | 2 | 27,379 (63) | 3,161 | 8,373 (25,780) | 12 (block of 12) | 63 / 0 / 0 |
| 27,388 (3-piece) | 27,383 (63) | 5 | 27,393 | 5 | 36,593 (20) | 9,210 | 6,059 (27,390) | 43 (block of 43) | 20 / 0 / 0 |
| 36,568 (3-piece) | 36,563 (107) | 5 | 36,573 | 5 | 36,593 (20) | 30 | 6,014 (36,570) | 87 (block of 87) | 20 / 0 / 0 |
| 38,553 (3-piece) | 38,550 (3) | 3 | 38,569 | 16 | 41,456 (0) | 2,906 | 3,495 (38,555) | 3 (block of 3) | 0 / 0 / 0 |

Digests of the full exact state (`R.st.digest()`) at the prefixes s, n − 1, b and e:

| n | Prefix s | Prefix n − 1 | Prefix b | Prefix e |
| --- | --- | --- | --- | --- |
| 9,654 | `5b8e7a1f8e1bc28d25cd8e350a38e90ae6dd4ba1274ac331cd7d9e3a2e9ca60f` | `dbb934a1098ec60a550a94ba1ba51b33560f8d97b5ffd32459c692fa3084bbf0` | `1bc39b63e1b71c23be4c8dd06c85d7604cb2e8dcf0860fe58fc9ef4c389eda1a` | `a145da70782db64d18efaa464ff81856ed4a7413e64d4706252c1595b8f1d0cf` |
| 17,884 | `796499769d81572b3f342f6e1cdfd8f33c3408e54d3e3249ad6cb3923be30764` | `2a1ce11f4fa083b7906be86a018250093436fd76f6e354da79d9d8a279aefab9` | `9b80f98b50b136ade537ad0a9d221d1f0882bed24cb272ff233166e8ed88efcc` | `7244ef1c11f4c02c07fc672f7c70b548abf09181d794ae6e93b12b48191172a7` |
| 24,219 | `3d4cf5e05a0756e16f33b9224e92b382baa6d40038381eb9956e2c79960bfa65` | `3d4cf5e05a0756e16f33b9224e92b382baa6d40038381eb9956e2c79960bfa65` | `387635118e476627aef05d8d5616d6483f0abe8e7838a9c381b2525773d17263` | `bdee76cdc13f028a6440a7072691d1411f85232a9e86f7eed1d25d00db1d9e92` |
| 27,388 | `cdea8b7293303545ebebbacae4d678a83b06b0b08e217594158641d82d187f11` | `6ff41f298c94698a78bc07fc74b1f9ce36d46dc3f2cbdd1d2cb2237a712eb564` | `affdafc0e2dc08491d73d8b6e972f545658fc8df561e1c92c9aba4133e91af06` | `a3f3a71d332c2d609107cae1f2a2861e53257938de625a1fde9de93337d4190f` |
| 36,568 | `b318e80e110f408873bbaa5742d988c8cb91525888da24788ed5a7030c7e08e1` | `09006c62fae5b289a46c78e7bbfaa296de0eb0f4e88c78fa69ca1cc06d1c05f7` | `dedc1635a5106848bddaaa55cb14d5a2d8258ee47de6fb55affb737cdc4cdaca` | `a3f3a71d332c2d609107cae1f2a2861e53257938de625a1fde9de93337d4190f` |
| 38,553 | `c280ae5f1e379ae92f300abeff7b81d1de7cf318160d989de239c04483097ff0` | `618d52c6e1552a26d0f4d20edf73d65bd70cf25e637af23d49c65a7a5f998754` | `381aaab66661fc4d1392cef93797dde279c8c28df66d84fb57e8001ada9fbc41` | `c7376ad8c9726d88c49e338cf7f59b8ca0fbdfd94775a033220d40ff9a04807a` |

Structure of the witnesses (**Computed**, from every letter's exact element). Letters are named by grip; x′ is the exact inverse of x at the same grip (their exact matrix product is the identity), and x[i] is the menu entry i at the grip of x when it is neither x nor x′.

- **Window 9,654** (12-piece block B; s = 9,649, e = 9,688, 39 twists). Grips a = 452, b = 507, c = 369, d = 578, e = 592; the letters are a = (452, 26), a′ = (452, 58), b = (507, 11), c = (369, 15), c′ = (369, 44), d = (578, 1), e = (592, 11); b, d and e are involutions in K, a and c lie outside K. The witness is
  a b a′ b c b a b a′ c′ · d a d a′ e a d a′ d e · c a b a′ b c′ b a b a′ · e d a d a′ e a d a′,
  that is [[a, b], c] · [[d, a], e] · [[a, b], c]⁻¹ · [[d, a], e]⁻¹ without its last letter: the commutator of commutators [[[a, b], c], [[d, a], e]] less its final d. Twist 9,689 is that d; it lies in K, and a letter in K takes no piece on or off the lattice (K is a group, so it keeps every pose in K or out of K), so the window closes one letter before the commutator ends.
- **Window 17,884** (12-piece block B; s = 17,882, e = 20,107, 2,225 twists: the owner's word 11). Grips a = 543, b = 369, c = 507, d = 592, e = 449, f = 494; a = (543, 44), a′ = (543, 15), b = (369, 15), b′ = (369, 44), c = (507, 28), c′ = (507, 40), and x[i] is the letter (grip of x, menu i) when it is neither x nor x′; A = a[10] = (543, 10) and B = b[10] = (369, 10) are involutions in K, the other 14 letters lie outside K. The witness, compressed (exact):
  (a b c b′ c′ a′ c b c′ b′)^72 (d B A B A d′ A B A B)^35 d (B A)^2 d′ A B a[39] (e a′ e′ B e a e′ a′ B a)^35 e a′ e′ B e a e′ a[33] (e A e′ f e A e′ A f′ A)^35 e A e′ f e A e′ a[39] (f′ a′ f a)^35 f′ a′ (b c b′ c′)^72,
  so it opens with [a, [b, c]]^72 and ends with [b, c]^72. The forcing twist is its second letter b = (369, 15).
- **Window 24,219** (12-piece block B; s = 24,218 = n − 1, e = 27,379, 3,161 twists). 95 grips and 158 letters (140 of them in K); a = (369, 56), a′ = (369, 34), b = (154, 45), b′ = (154, 12). The witness opens with [a, b]^72, the forcing twist being its first letter a = (369, 56). Its first 2,225 letters are six periodic stretches with the repeat counts of word 11 (72, 35, 35, 35, 35, 72); then come 217 letters through 90 grips and three more periodic stretches of 288, 288 and 143 letters, the last (b a[3] b′ a[9])^35 b a[3] b′ (exact form in the package). Twist 27,380 is a[9] = (369, 9), in K: again the window closes one letter before the last period ends.
- **Window 27,388** (3-piece block B′; s = 27,383, e = 36,593, 9,210 twists). Grips a = 491, b = 283, c = 237, d = 407, e = 275; the letters are a = (491, 56), a′ = (491, 34), b = (283, 11), c = (237, 28), c′ = (237, 40), d = (407, 10), e = (275, 10); b, d and e are involutions in K, a and c lie outside K. The witness is exactly P · Q · P⁻¹: each of its last 43 letters is the exact inverse of the mirror letter among its first 43. P is the commutator of commutators [X, Y] = [[[a, b], c], [[d, a], e]] (40 letters, the shape of window 9,654 with its last letter d; the forcing twist is its fifth letter c), followed by three letters in K: (237, 2), (392, 2) and (283, 7). Q has 9,124 twists, with 924 distinct letters at 424 grips, every one in K, so Q takes no piece on or off the lattice. P⁻¹ ends with [X, Y]⁻¹ = Y X Y⁻¹ X⁻¹, and its last 30 letters, X Y⁻¹ X⁻¹, are the witness of window 36,568.
- **Window 36,568** (B′; s = 36,563, e = 36,593, 30 twists). The witness is [[a, b], c] · [[d, a], e]⁻¹ · [[a, b], c]⁻¹ with the letters of window 27,388, the conjugate of [[d, a], e]⁻¹ by [[a, b], c]; the forcing twist is again the fifth letter c = (237, 28), and c′ = (237, 40) is the tenth. Twist 36,594 is (237, 2), where the word leaves this pattern. Windows 27,388 and 36,568 close at the same prefix: the conjugate both closes window 36,568 and returns the last lattice pieces of prefix 27,383 that were still off.
- **Window 38,553** (B′; s = 38,550, e = 41,456, 2,906 twists). Grips a = 237, b = 162, c = 283 and 18 more; a = (237, 44), a′ = (237, 15), b = (162, 8), which lies in K and has order 3, b′ = (162, 4). The witness is exactly P · Q · P⁻¹: each of its last 1,438 letters is the exact inverse of the mirror letter among its first 1,438, with
  P = (a b a′ b′)^35 a b a′ (c b[51] c′ b[16])^72 (a[11] b[35] a[11] b[27])^71 a[11] b[35] a[11] b[29] (a b a′ b′)^35 a b a′ (c b[51] c′ b[16])^72 (a[11] b[35] a[11] b[27])^72 (1,438 letters),
  Q = d e f g h i h′ g f j k f[2] l m n k[5] n′ m l f[5] o p q r s t u t′ s′ r (30 twists, with 25 distinct letters at 18 grips, every one in K).
  The forcing twist is the third letter a′. Twist 41,457, the last of the owner's word, is b, in K; at e no piece is off the lattice.

What each forcing twist moves and what comes back (**Computed**, from the same report; "B" is the 12-piece block, "B′" the 3-piece block):
- **9,654.** x = (369, 15) moves two whole blocks of prefix 9,653: B (its locked chambers 114153 and 118328 hold the lattice pieces 31985 and 53589), which stays off at a new pose, and the anchor's 111-piece block whose smallest piece is 64191, which x carries onto the lattice. x takes 2,860 lattice pieces off, the two forced pieces among them, and x′ = (369, 44) puts all 2,860 back at twist 9,659. B is moved only by x and x′ (twists 9,654, 9,659, 9,670 and 9,675), so the forced pieces leave and return twice. Before x, at least 790 lattice pieces of s are already off (1,289 off at n − 1 against 499 at s). Restored at e: that 111-piece block.
- **17,884.** x = (369, 15) moves seven whole blocks of prefix 17,883: it carries B itself onto the lattice (its locked chambers 58358 and 58361 hold 53589 and 31985) and moves six blocks of 86 to 111 pieces that stay off. x takes 2,607 lattice pieces off; x′ at twist 17,886 puts 2,505 back, the other 102 come back at 17,896. In the window B is moved 646 times: 216 times each by x and x′ and 214 times by (369, 10), which lies in K. At least 3,085 lattice pieces of s are off at n − 1. Restored at e: the anchor's 86-piece block (smallest piece 64184).
- **24,219.** x = (369, 56), the window's first letter, moves one whole block, B, and carries it onto the lattice. x takes 3,097 lattice pieces off; x′ = (369, 34) at twist 24,221 puts 2,702 back and takes B off again, the other 395 come back at 24,224. B is moved 890 times in the window, by x, x′, three letters at grip 154 and 24 letters in K at other grips. Restored at e: B itself.
- **27,388.** x = (237, 28) moves two whole blocks of prefix 27,387: B′ (its locked chambers hold 57220, 47840 and 47855), which stays off, and the anchor's 43-piece block, which x carries onto the lattice. x takes 3,010 lattice pieces off and x′ = (237, 40) puts all of them back at twist 27,393. B′ is moved only in P and P⁻¹: by x and x′ at twists 27,388, 27,393, 27,404 and 27,409, by (237, 2), in K, at twist 27,424 and by its inverse (237, 5) at twist 36,553, and by x and x′ of window 36,568 at twists 36,568, 36,573, 36,584 and 36,589; no letter of Q moves it. At least 790 lattice pieces of s are off at n − 1 (853 against 63). After the 40-letter commutator, at prefix 27,423, 131 pieces are off: the 43-piece block is on the lattice, and 111 lattice pieces of s are off, all taken off at twist 27,416 (the a of Y⁻¹). They stay off through Q (131 pieces are off at every prefix of Q) and come back together at twist 36,561, the a′ of P⁻¹ that is the mirror inverse of twist 27,416. The a at twist 36,555 has meanwhile taken 87 other lattice pieces of s off: they form the 87-piece block of the anchor of window 36,568 and are back by e (first at twist 36,582). Restored at e: the 43-piece block; the other 20 off pieces of s, B′ among them, are at their exact poses of s.
- **36,568.** x = (237, 28) moves only B′, which stays off. x takes 3,009 lattice pieces off and x′ puts all of them back at twist 36,573; B′ is moved only by x and x′ (twists 36,568, 36,573, 36,584 and 36,589). At least 703 lattice pieces of s are off at n − 1. Restored at e: the anchor's 87-piece block (smallest piece 3569), lattice pieces of prefix 27,383 that were off at 36,563 (taken off at twist 36,555, in window 27,388).
- **38,553.** x = (237, 15) is a′, the exact inverse of the window's first letter a = (237, 44). It moves 26 whole blocks of prefix 38,552: B′, which stays off, and 25 blocks (2,702 pieces) that a took off at twist 38,551, which x puts back on. x takes 395 lattice pieces off, the three forced pieces among them; they come back at 38,563 (222), 38,565 (87) and 38,569 (86). Between n and b, B′ is moved only by x (at 38,553, 38,557, 38,561, 38,565 and 38,569) and by the pairs b′ = (162, 4), b = (162, 8), which cancel because a, between them, does not move B′. x has order 5 and no power x^j (1 ≤ j ≤ 4) lies in K (the same holds for the forcing letters of all six windows), so B′ is back at its pose of n − 1 exactly at the fifth x, which is b; that is why b − n = 16 here. Restored at e: B′ itself; nothing is off at e.

At b the forced pieces are back, but 198 (windows 9,654, 27,388 and 36,568), 396 (38,553), 3,095 (24,219) and 5,692 (17,884) lattice pieces of s are still off (`at_b` in the report): b − n (2 to 16 twists) is not the length of a witness.

Window 9,654 step by step (`windows["9654"].twists` in the report, which has the same rows for every twist of every witness). "Taken off" counts the lattice pieces the twist takes off and "First back" groups them by the first later prefix at which each is on the lattice again; a group can come back after e when its pieces were off at s (U4). "Put on" counts the pieces the twist puts on the lattice.

| Prefix | Letter (grip, menu) | Symbol | In K | Taken off | First back (prefix: pieces) | Put on | Off after |
| ---: | --- | --- | --- | ---: | --- | ---: | ---: |
| 9,650 | (452, 26) | a | no | 2,931 | 9,652: 2,536, 9,658: 395 | 197 | 3,233 |
| 9,651 | (507, 11) | b | yes | 0 | – | 0 | 3,233 |
| 9,652 | (452, 58) | a' | no | 592 | 9,656: 592 | 2,536 | 1,289 |
| 9,653 | (507, 11) | b | yes | 0 | – | 0 | 1,289 |
| 9,654 | (369, 15) | c | no | 2,860 | 9,659: 2,860 | 111 | 4,038 |
| 9,655 | (507, 11) | b | yes | 0 | – | 0 | 4,038 |
| 9,656 | (452, 26) | a | no | 2,647 | 9,658: 2,647 | 592 | 6,093 |
| 9,657 | (507, 11) | b | yes | 0 | – | 0 | 6,093 |
| 9,658 | (452, 58) | a' | no | 197 | 9,661: 197 | 3,042 | 3,248 |
| 9,659 | (369, 44) | c' | no | 198 | 9,670: 198 | 2,860 | 586 |
| 9,660 | (578, 1) | d | yes | 0 | – | 0 | 586 |
| 9,661 | (452, 26) | a | no | 3,042 | 9,663: 2,647, 9,667: 197, 9,688: 198 | 197 | 3,431 |
| 9,662 | (578, 1) | d | yes | 0 | – | 0 | 3,431 |
| 9,663 | (452, 58) | a' | no | 592 | 9,665: 592 | 2,647 | 1,376 |
| 9,664 | (592, 11) | e | yes | 0 | – | 0 | 1,376 |
| 9,665 | (452, 26) | a | no | 2,647 | 9,667: 2,647 | 592 | 3,431 |
| 9,666 | (578, 1) | d | yes | 0 | – | 0 | 3,431 |
| 9,667 | (452, 58) | a' | no | 197 | 9,671: 197 | 2,844 | 784 |
| 9,668 | (578, 1) | d | yes | 0 | – | 0 | 784 |
| 9,669 | (592, 11) | e | yes | 0 | – | 0 | 784 |
| 9,670 | (369, 15) | c | no | 2,860 | 9,675: 2,860 | 198 | 3,446 |
| 9,671 | (452, 26) | a | no | 2,844 | 9,673: 2,536, 9,679: 308 | 197 | 6,093 |
| 9,672 | (507, 11) | b | yes | 0 | – | 0 | 6,093 |
| 9,673 | (452, 58) | a' | no | 592 | 9,677: 592 | 2,536 | 4,149 |
| 9,674 | (507, 11) | b | yes | 0 | – | 0 | 4,149 |
| 9,675 | (369, 44) | c' | no | 0 | – | 2,860 | 1,289 |
| 9,676 | (507, 11) | b | yes | 0 | – | 0 | 1,289 |
| 9,677 | (452, 26) | a | no | 2,536 | 9,679: 2,536 | 592 | 3,233 |
| 9,678 | (507, 11) | b | yes | 0 | – | 0 | 3,233 |
| 9,679 | (452, 58) | a' | no | 197 | 9,682: 197 | 2,844 | 586 |
| 9,680 | (592, 11) | e | yes | 0 | – | 0 | 586 |
| 9,681 | (578, 1) | d | yes | 0 | – | 0 | 586 |
| 9,682 | (452, 26) | a | no | 2,844 | 9,684: 2,647, 9,688: 197 | 197 | 3,233 |
| 9,683 | (578, 1) | d | yes | 0 | – | 0 | 3,233 |
| 9,684 | (452, 58) | a' | no | 592 | 9,686: 592 | 2,647 | 1,178 |
| 9,685 | (592, 11) | e | yes | 0 | – | 0 | 1,178 |
| 9,686 | (452, 26) | a | no | 2,647 | 9,688: 2,647 | 592 | 3,233 |
| 9,687 | (578, 1) | d | yes | 0 | – | 0 | 3,233 |
| 9,688 | (452, 58) | a' | no | 197 | 13,923: 111, 19,824: 86 | 3,042 | 388 |

What the six windows share (**Computed** on these six windows; that the same pattern restores a block at other states is a **Conjecture**, untested):
- (U1) One twist, whole blocks. The forcing twist x is admissible and moves whole rigid blocks only (no off piece straddles its cut). One of them, B, has occupied locked chambers, and x moves it by an element outside K, so it takes their lattice pieces off (Corollary L, proved). x takes off every lattice piece in its cap, not only the locked ones, since m k ∉ K for m ∉ K and k ∈ K (395 to 3,097 pieces here); the locked ones are those that cannot come back unless B moves outside K again. In five windows the same twist carries a whole off block onto the lattice: the block the window restores (9,654 and 27,388), B itself (17,884 and 24,219), or the 25 blocks that a took off two twists before (38,553).
- (U2) The forced pieces come back with B. Between n and b, B is moved only by x, by its exact inverse x′ at the same grip, and by letters that cancel on it; b is the first prefix at which B is back at its pose of n − 1. In windows 9,654, 27,388 and 36,568, x is the c of [[a, b], c]: c′ comes five letters later, and [a, b]⁻¹ between them does not move B (b − n = 5). In windows 17,884 and 24,219, x is the first letter of a commutator x y x′ y′, and y does not move B: x′ comes two letters later (b − n = 2). In window 38,553, x = a′ has order 5 and no power x^j (1 ≤ j ≤ 4) lies in K; a does not move B′, which the K letter b′ = (162, 4) has moved out of a's cap and b = (162, 8) moves back, so B′ is back at its pose at the fifth x (b − n = 16).
- (U3) The window closes when the word around x is complete. e is the last prefix of the word that contains x: the commutator of commutators [X, Y] (window 9,654), the owner's word 11 (17,884), the last periodic stretch (24,219), or a conjugate P Q P⁻¹ with x in P (27,388, 36,568 and 38,553); in windows 9,654, 24,219 and 38,553 one letter early, because that last letter lies in K and a letter in K takes no piece on or off. In windows 27,388 and 38,553 every letter of Q lies in K, so through Q every piece stays on or off the lattice as P left it, and a lattice piece of s that is off after P can come back only in P⁻¹. In window 27,388, [X, Y] restores the 43-piece block but leaves 111 lattice pieces of s off; they stay off through Q and come back together at twist 36,561 of P⁻¹, the mirror inverse of twist 27,416, which took them off; 87 other lattice pieces of s, taken off at twist 36,555, come back in the last 30 letters of P⁻¹, the witness of window 36,568.
- (U4) Nothing else changes. At e every off piece of s that is still off has its exact pose of s, B included, and the pieces restored form whole blocks of s (111, 86, 12, 43, 87 and 3 pieces; in windows 24,219 and 38,553 the forcing block itself). So Ξ(e) ≤ Ξ(s) in the sense of section 6.1: by Lemma M every SAFE-ALL letter of s is one at e, with the same element and cap permutation, and section 6.1 states what Corollary S transfers to e and what must be computed again there (the holes).

So b − n measures (U2) only. The witness length e − s, from 30 to 9,210 twists, is set by (U3), and the witness begins before n whenever the word around x begins before it.

Status of the six windows against the owner's v17 request:
- Start tables or reproducible prefixes: prefix s of the owner's word (from the E2 end) and the pose table at s; both reproduce the digest at s (checked for all six windows: loading the table gives the digest at s, and replaying the witness from it gives the digest at e; the replay helper is in the owner's handoff package and was not reviewed).
- Complete short words: word[s:e], in `results/outside/unlock-e2-to-lattice.json` (`windows[n].witness`), with the twists before s that the report lists for context.
- Endpoint digests: the digests at s, n − 1, b and e above, and the digest after every twist of every witness in the report.
- The lattice pieces each step takes off and where each is first back: per twist, the count taken off and put on and, for the pieces taken off, how many are first back at each later prefix (`back`); for the twists s + 1 to n + 15 also the piece numbers (in the full report, which is in the handoff package; the repository copy drops the per-piece and per-twist block lists). A piece taken off inside a window and not back by e was off at s and is back at its pose of s at e (U4).
- Remaining off poses: at e every off piece is at its exact pose of s, so the pose table at s gives them.
- Geometric hypotheses: (U1) to (U4), computed on these six windows; that they suffice to build such a window at another state is a **Conjecture**.

## 9. Status

| Claim | Status |
| --- | --- |
| Proposition N | Proved; independent review: sound |
| Theorem A, Corollary A1 (Jordan's theorem as in Jones, Theorem 1.1) | Proved; independent review: sound |
| Theorem B | Proved; independent review: sound with the SAFE-ALL word hypothesis now stated (finding MATH-03) |
| Lock lemma, Corollary L | Proved; independent review: sound. The vacancy consequence now carries the return condition (MATH-04) |
| BV(Y, T) | Sufficient for BUFFER-VACANCY; equivalent under Jordan certificates or a peeling certificate with every u_j = 0 (MATH-02); the certified BUFFER-PROGRESS is a strengthening (MATH-05) |
| Flow bound for one-letter setups | Proved; supply stated as the holes of Y (MATH-01: the text said "holes of X"; the classification script already counted the holes of Y) |
| Certificates at X0 and v14 to v18; locked chambers; the 899 bound; the demo; X5 and its classification | Computed (exact), scripts named; not part of the proof review |
| Recovery of X5 by the setup (315, 1) | Constructed (exact); replayed with every check by the constructing script and independently by `replay_check.py` |
| Lemma M, Corollary S (a restoration that keeps the remaining off poses keeps SAFE-ALL words and certificates) | Proved; independent review: one full review (finding MATH-M-01, adopted: recomputing at a later state transfers only the necessary counts), then two scoped verifications, both with no finding (the second after the rewrite with explicit hypotheses) |
| Lemma M on three restorations (X5 recovery, the owner's two v17 returns) | Computed (exact), `monocheck.py` |
| The six unlock windows of the owner's construction (section 8.5): anchors, closing prefixes, witnesses, what each twist takes off and when it comes back, the comparison at b and at e | Computed (exact), `unlock.py`; code review: one full review (six findings, all adopted), then one scoped verification: pass; every witness replayed from its anchor table (helper in the handoff package, not reviewed) |
| U1 to U4 (what the six windows share) at other states | Conjecture, untested |
| v17 states: same families, Jordan certificates 19 of 19, T2 peeling certificate and the 535- and 649-letter preparations followed by the owner's return 2 | Computed (exact), `buffer.py`; BUFFER-VACANCY for T1 at the 8372 start proved from the computed certificates (Corollary A1) |
| BUFFER-PROGRESS at every reachable depth-one state (general E) | Open |
