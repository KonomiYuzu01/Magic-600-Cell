# Buffer vacancy read from the current state (R1, question E, items 1 and 2)

Status: **12 October 2026.** Labels: **Proved** (by the integrator), **Computed** (exact, by the script named), **Open**, **Conjecture**. One independent Codex review (11 October 2026, proofs only, computed values out of its scope) found Proposition N, Theorem A, Corollary A1, the lock lemma and Corollary L sound under the stated model, and Theorem B sound once its words are required to be SAFE-ALL words; its five findings on the criteria built on them are answered in the text (section 9), and a scoped verification found all five fixed. Computed values are outside that review; the two scripts the integrator wrote (`setup_flow.py`, `lockcheck.py`) had a separate code review, whose four blocking findings were fixed and verified. Lemma numbers refer to the owner's manuscript (card 3, revision 11), which is not in this repository; its Lemmas 41, 44, 46, 48 to 51 are used as stated there.

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
| BUFFER-PROGRESS at every reachable depth-one state (general E) | Open |
