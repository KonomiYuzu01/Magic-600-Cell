# Stage 16: proofs of the joint attack on card 8 (25 October 2026)

The results of the joint attack on [card 8](../../theory/cards/card-8-six-jams.md), with proofs. The process, the
computed facts and the answers to the card's questions are in [README section 2](README.md#2-joint-attack-25-october-2026).
Each result below was proved in one of the two sealed analyses or in the cross-examination, and its proof was checked
line by line by the other analyst and by the integrator. The source of each is given in brackets: Astra, Claude Opus
(the second analyst), or both; "scope by" names the analyst whose objection fixed the statement as given here.

Notation is that of [stage 15](../15-lowering-machine/PROOFS.md) and its predecessors: items at a grip are numbered 0 to
59 (0 the identity, 1 to 11 retained, 12 to 59 jumbles); a·g means g first, then a; h_L is the left factor of an item h.
A twist (c, h) maps the tree position of every piece it moves by h_L, and a jumble changes the height of each moved
piece by −2, 0 or +2 (card 5, T5). φ(A) = max(0, h(A) − 2) and Φ = Σ_A φ(A). For a clean grip x and a jumble j at x,
F_x is the number of foreign pieces in at x (height at least 2, branch other than CLS(x)), D_x(j) the number of pieces
that j lowers, and (x, j) changes Φ by 2(F_x − D_x(j)) (Lemma Φ). S_x is the set of straddlers of x, I_x(Z) the inside
set of x at Z, ℓ the label table (out, in, across). Words, traces, excursions, tame words and macros are as in stage 15:
the excursion of a word from Z is the largest rise of Φ above Φ(Z) along it, the last letter included.

## 1. Transport (Questions 1.1 and 2)

**Theorem X (retained transport) [Claude Opus; the case K = 1 is Astra's Lemma 1].** Let Z be a configuration with
label table ℓ, heights h, branches b and, for each piece of height at least 4, its second vertex s, all read once from
the exact poses at Z. Let w = (c_1, k_1) ⋯ (c_K, k_K) be a word of retained letters. Give every piece A a path product
π_A ∈ K⁺: it starts at 1, and at letter t it becomes k_t·π_A exactly when ℓ(A, π_A⁻¹c_t) = in.

(a) w is admissible at Z exactly when, for every t, no piece has ℓ(A, π_A⁻¹c_t) = across at that point.

(b) After w, the label of A at a grip d is ℓ(A, π_A⁻¹d), its height is h(A), its branch is σ_{π_A}(b(A)) and its
second vertex is (π_A)_L s(A). Φ is constant along w.

(c) Let x be a grip with no straddler after w and j a jumble at x. A is foreign at x after w exactly when it is foreign
at π_A⁻¹x at Z, and A counts in D_x(j) after w exactly when it counts in D_{π_A⁻¹x}(Z, π_A⁻¹ j π_A). So w·(x, j)
changes Φ by 2(F′_x − D′_x(j)), where F′ and D′ are sums of these per-piece tests at Z.

Hence, if F′_x < D′_x(j), then w·(x, j) is a tame macro of length K + 1 with excursion 0 and ΔΦ = −2(D′_x − F′_x), and
every hypothesis is a finite exact read of (ℓ, h, b, s) at Z; no pose is evaluated.

*Proof.* Induction on t. Lemma RS (stage 15) gives (a) and the labels, heights and branches of (b): letter t moves
exactly the pieces in at c_t, and a moved row is read in the column k_t⁻¹d; composing, a row after t letters is the row
at Z read in the column π_A⁻¹d. Corollary CR gives the foreign test and the D test after one letter (CLS(x) =
σ_k(CLS(k⁻¹x)), the second vertex becomes k_L s, and the jumble is conjugated); composing along the letters that moved A
gives (c). Retained letters keep heights, and the last letter is Lemma Φ at the clean grip x. ∎

Rows with equal labels and tags stay equal under retained letters, so the computation can run on weighted classes of
rows (`pstar.RetainedSearch`, stage 15). Corollary SRR is Theorem X with the final target fixed in advance; family R of
P* fixes it to an own holder of the configuration where the search starts, so a descent at a grip that becomes a target
only after the word (a **dynamic target**) is outside R by declaration [Astra; Claude Opus].

The case K = 1 reads, for c clean, k retained at c, I = I_c(Z) and e = k⁻¹x: S′_x = |S_x \ I| + |I ∩ S_e|; F′_x is
read from the unchanged rows outside I and from the foreign flags of column e on I; D′_x(h) from the unchanged rows
outside I and the second-vertex test for k⁻¹hk on I [Astra]. In particular, after (y, a) at a clean y, a clean z ≠ y with
F′_z = 0 whose cap holds no high piece has all 48 jumbles Φ-neutral (the purification relay) [Claude Opus].

**Exact reductions of the search [Claude Opus; normal form corrected by Astra].** Each keeps at least one
representative of every shortest descent of family X:

- **R1 (merge).** Two adjacent letters at one grip c act as the single item k₂·k₁ (Proposition 1C; c is clean after its
  own letter); an identity product is dropped.
- **R2 (commute).** Letters at grips that do not interact commute, with the same admissibility and the same end (stage
  14, L1 and L10). So the admissible words up to these swaps form a trace monoid over the dependence relation "same grip
  or interacting grips". The lexicographically least linearization of a trace has no adjacent pair of non-interacting
  letters with the larger grip first, so the local rule "never put the larger of two adjacent non-interacting grips
  first" keeps at least one representative of every trace. It is not a normal form: for grips a < b < c with b
  independent of a and c and a dependent on c, both bca and cab pass the local rule and are the same trace [Astra].
- **R3 (minimality).** If a letter has no chain of interacting letters, in word order, to the final letter (x, j), then
  the trace equals w′·(x, j)·w″ with that letter in w″; since w″ is retained and Φ-neutral, w′·(x, j) is a shorter tame
  descent. So once every shorter length has been searched, it suffices to keep the words in which every letter has such
  a chain.
- **R4 (dynamic targets).** D′_x(j) > 0 needs a piece of height at least 4 in at x with branch CLS(x) after w, so the
  final targets are the clean grips that hold such a piece after w.

*Proof.* R1 is Proposition 1C. R2: a letter moves only pieces inside its cap and its admissibility depends only on its
column, and for non-interacting grips no piece is inside or across both (stage 14, L1), so the two letters act on
disjoint rows and neither changes the other's column. R3: by R2 the letter can be moved past every later letter it does
not interact with, and past (x, j); the word that ends at (x, j) is admissible, ends at the same Φ and is shorter. R4 is
the definition of D. ∎

So family X(K₀), all clean letters with R1 to R4 and every lowering at the dynamic targets, finds a descent of the
family whenever a tame descent whose prefix has at most K₀ retained letters exists, and an exhausted enumeration is a
bounded negative for that family (not a certificate in the sense of Proposition BN).

**Lemma Z (zero excursion) [Astra].** A descent with excursion 0 exists from Z exactly when a tame descent exists from
Z. A complete negative for excursion 0 is a set that contains Z, is closed under every admissible Φ-neutral letter
(retained letters at clean grips and jumbles with F = D), and whose members have no descending letter.

*Proof.* Cut a descent with excursion 0 at its first state below Φ(Z): every earlier state has Φ exactly Φ(Z), so the
prefix is neutral and its last letter descends. The converse is the definition. The certificate statement is
Proposition BN with E = 0. ∎

So the extension X⁺ of Theorem X, which also admits Φ-neutral jumbles in the prefix (their effect needs Lemma CL or one
exact image per moved cap, Theorem MC), is the complete family for E* = 0 [Claude Opus].

## 2. Release accounting (Question 1.3)

**Lemma B (budget ledger) [Astra; the U and V form by Claude Opus; monotone form corrected by Astra].** Fix a target x
and a jumble h at x. For each piece let f be its foreign-and-in indicator at x and d its lowering indicator for h
(in at x, branch CLS(x), height at least 4, second vertex h_L⁻¹v₀), and put B = D_x(h) − F_x = Σ(d − f). For an
admissible word u with chronological supports M_t,

B(uX) = B(X) + Σ_t Σ_{A∈M_t} [(d_after − d_before) − (f_after − f_before)].

If x is clean after u and Δ_u is the change of Φ along u, then u·(x, h) lowers Φ below Φ(X) exactly when
2B(uX) > Δ_u. Equivalently, with U the net decrease of F_x and V the net gain of the h-supply caused by u, the lowering
changes Φ by 2(F_x(X) − U − D_x(X, h) − V), and the word descends exactly when this is below −Δ_u.

*Proof.* A letter changes the indicators of the pieces it moves only, so the sum telescopes; the final jumble is Lemma
Φ, and the whole word changes Φ by Δ_u + 2(F − D) after u. ∎

- For a retained release Δ_u = 0, so the condition is B(uX) ≥ 1, the macro has excursion 0 and length |u| + 1. For a
  release that raises Φ, F < D after it is not enough.
- **Monotone release.** If no letter of u raises a foreign indicator at x or clears a lowering indicator for h (by
  moving the piece, changing its branch or moving its second vertex), then B never falls. Spatial conditions alone
  ("no foreign piece moves into x, no supply piece moves out") are not enough, because a letter can change a branch or a
  second vertex in place.
- Instance (Claude Opus, exact): at 7305, 504 goes from (S, F, D) = (6, 66, 120) to (2, 22, 120) after (305, 2), to
  (2, 5, 137) after the jumble (421, 51), which moves 17 foreign pieces of the class-4 target into its supply, and to
  (0, 0, 137) after (389, 6). Counter-instance (Astra, exact): the tame word 543:3, 448:5, 543:7, 597:1 at 7307 cleans
  517 and changes its (F, max D) from (79, 95) to (89, 0).

**Lemma RF (refund) [Claude Opus; scope by Astra].** Let ℓ be clean at Z and Y = (ℓ, h)Z. By Lemma R (stage 14),
F_ℓ(Y) = D_ℓ(Z, h) and D_ℓ(Y, k) = F_ℓ(Z) for k in the coset A4_ℓ·h⁻¹. If a word u with no letter at ℓ ends with ℓ clean
and with the same inside set of ℓ, at the same poses, as at Y, then the return (ℓ, h⁻¹) changes Φ by exactly
−2(F_ℓ(Z) − D_ℓ(Z, h)). Otherwise the return changes Φ by 2(F_ℓ − D_ℓ(k)) read when it acts.

*Proof.* Under the hypothesis the state of cap ℓ before the return is that of Y, and (ℓ, h⁻¹) undoes (ℓ, h) on it;
otherwise apply Lemma Φ at that moment. ∎

**Consumption [Claude Opus; scope by Astra].** A piece that a leaf jumble raises (+2) and the target later lowers (−2)
contributes 0 to the word, or +2 if a later return at the leaf finds it inside the leaf in another branch (card 5, T5:
each jumble changes a moved piece's height by −2, 0 or +2). This piece-level identity is proved. The reading "a descent
needs supply that the target owned before the leaf raised anything" is not: retained transport can bring pre-existing
supply to a new target, and later letters can change a return's branch and second-vertex tests. It needs an individual
ledger of the pieces and stated hypotheses on the later letters.

## 3. Routing (Question 1.2)

**Lemma OT (orbit touch) [Claude Opus].** Let c be clean at Z with every straddler of a grip r in I_c(Z). Let
w = Y_0 (c, a_1) Y_1 ⋯ (c, a_n) Y_n be admissible at Z, where no letter of the helper words Y_j acts at c. Put p_0 = 1,
p_j = a_j ⋯ a_1 (the carrier position after j actions) and c′ = p_n (the carrier's total item). For an item g at c let
the obligated set be B_g = {A ∈ I_c(Z) : ℓ(A, g⁻¹r) = across}. If r is clean after w, then every A ∈ B_{c′} is moved by
some helper letter; the first such letter lies in some Y_j, acts at a grip y with ℓ(A, p_j⁻¹y) = in, and y is clean at
that moment.

*Proof.* Until a helper moves it, A is moved only by the carrier. Every item at c maps the cap onto itself, so A stays
inside c and every carrier action moves it; its pose after j actions is p_j·A, and its label at y is ℓ(A, p_j⁻¹y)
(Corollary CR). A letter moves exactly the pieces in at its grip, and that grip is clean when it acts. If no helper ever
moves A, its final pose is c′·A, which is across r. ∎

**Cover test.** Let Clean(p) be the set of grips clean in the carrier-only state (c, p)Z: no piece outside I_c
straddles y, and no piece of I_c straddles p⁻¹y. A position sequence (p_1, …, p_n) **passes** when every A ∈ B_{p_n} has
some j ≤ n and some y ∈ Clean(p_j) with ℓ(A, p_j⁻¹y) = in. By Lemma OT, passing is necessary for every routing word
whose helpers act at grips of Clean(p_j) at position j, and the first helper of any word always does. A helper made
clean only by an earlier helper is outside the test: the stronger statement that no word with at most n carrier actions
releases r, whatever its helpers, is not proved by the test (DIS-B, README section 2.7) [scope by Astra].

**Lemma RR (routing release) [Claude Opus; Astra's form].** In the setting of Lemma OT let w be a routing word of
retained letters. Suppose (H1) every letter acts at a grip with no straddler at that point, (H2) r has no straddler after
w, and (H3) there are retained items m_i at released grips r_i (at most two releases, their words concatenated), a word
u of at most one retained letter, a grip x and a jumble j at x such that, after w·(r_i, m_i)·u, x is clean and
F_x < D_x(j). Then W = w·(r_i, m_i)·u·(x, j) is a tame macro of length n + H + k + |u| + 1 (H helper letters, k
releases) with excursion 0 and ΔΦ = −2(D_x − F_x). Every hypothesis is a finite exact read at Z.

*Proof.* W is a word of retained letters followed by a lowering; apply Theorem X. ∎

With jumble helpers or jumble carrier items the ledger of Lemma B applies, and the excursion is the largest prefix sum
of their changes 2(F − D), each read when it acts. A prize read at Z from the formal image of a blocked grip is a
priority, not a guarantee: the release changes the target's column (the routing prefix of I-7304 changes 916 labels in
column 2), so the prize must be recomputed after the release [Astra].

**Conjugated-touch form [Claude Opus].** Split each carrier letter as (c, a_j) = (c, p_{j−1}⁻¹)(c, p_j). Then
w = Y_0 · ∏_{j=1}^{n−1} [(c, p_j) Y_j (c, p_j)⁻¹] · (c, c′) · Y_n, which is admissible whenever w is (c is clean when the
merged letter acts and stays clean after (c, p_{j−1}⁻¹)). Each bracket is a conjugate u v u⁻¹ with u = (c, p_j) and
v = Y_j, and every piece that Y_j does not move leaves the bracket where it entered (Lemma CT below). So the untouched
pieces of the carrier cap move by c′ in total, and each piece of B_{c′} must be touched inside some bracket, at its
orbit position p_j·A. O-SJ-D9 at 7304 is (23,10)[20:2, 112:2](23,10)⁻¹ · (23,9)[20:1](23,9)⁻¹ · (23,1) · 41:1 · 2:14,
whose merged carrier letters are 23:10, 23:5 (5 = 9·10⁻¹ at 23) and 23:7 (7 = 1·9⁻¹).

## 4. Conjugated release (Question 1.2)

**Lemma CT (conjugated release) [Claude Opus; dynamic condition and excursion corrected by Astra].** Let
u = (c_1, g_1) ⋯ (c_r, g_r) be admissible at Z with trace Z_0 = Z, …, Z_r = Y. Let v be admissible at Y, Y′ = vY, and
P the union of the supports of v (the pieces v moves). Suppose that along u⁻¹ = (c_r, g_r⁻¹) ⋯ (c_1, g_1⁻¹), applied at
Y′, no piece of P straddles c_i at the moment (c_i, g_i⁻¹) acts. Then:

1. u⁻¹ is admissible at Y′;
2. every piece outside P ends at its pose in Z;
3. Φ(u v u⁻¹ Z) = Φ(Z) + Σ_{A∈P} [φ(end_A) − φ(Z_A)].

If moreover the contribution of P to Φ is unchanged by every letter of u and of u⁻¹ (in particular, if no piece of P
is inside the grip of a jumble letter of u when that letter or its inverse acts), then, with d_u = ΔΦ(u) and
d_v = ΔΦ(v), ΔΦ(u v u⁻¹) = d_v and E(u v u⁻¹) = max(E(u), d_u + E(v), d_v + E(u)). The last term can be dropped when
d_v ≤ 0.

*Proof.* Induction along u⁻¹. Before (c_i, g_i⁻¹) acts, the pieces outside P are at their Z_i poses, so none of them
straddles c_i (a twist leaves its own grip clean, so c_i is clean at Z_{i−1} for the pieces it then moved, and every
other piece outside P is where it was at Z_{i−1}); the pieces of P do not straddle it by hypothesis, so the letter is
admissible. It moves exactly (I_{c_i}(Z_i) \ P) together with the pieces of P inside c_i, and sends the first set to
their Z_{i−1} poses; pieces outside P and outside c_i were not moved by (c_i, g_i). This gives 1 and 2; 3 follows
because Φ is a sum over pieces. Under the extra condition the pieces outside P contribute d_u along u and −d_u along
u⁻¹, and those of P contribute d_v in total; the profile is that of u, then v shifted by d_u, then u⁻¹ shifted by d_v,
whose maximum is the stated E. ∎

- The condition is dynamic: it must hold before every inverse step, not only at the end of v. In D5c (7306,
  u = (598, 20)(536, 11), v = (233, 6)(370, 15)(370, 4)) no piece of P is inside 598 right after v, but 101 enter 598
  during the inverse action at 536; P meets the forward support at 598 in 244 pieces and the inverse support in 101, so
  the "no P inside a jumble grip" form does not hold, while the per-step Φ(P) form does (its contribution along the
  seven letters is 222, 222, 222, 222, 0, 0, 0, 0). Restoration was checked exactly: |P| = 5,428 and all 171,692 pieces
  outside P end at their jam poses (pose keys compared; both analysts). ΔΦ = −222 and E = 292.
- The third term is needed: at 7306, u = (233, 12)(233, 45) (45 is the inverse of 12 at 233) and v = (598, 12) satisfy
  the exclusion condition (the caps do not interact), and u v u⁻¹ has the profile 1490, 1646, 1490, 1782, 1938, 1782,
  so E = 448, where max(E(u), d_u + E(v)) = 292 (Astra, exact).
- Lemma CJ of stage 15 (a single lowering conjugated by u, with the lowered pieces out at every c_i) is a stronger
  sufficient condition with a stronger conclusion (the lowered pieces keep their poses) and stays as stated; Lemma CT is
  a separate generalization [Astra].

## 5. Raise, release, lower and return (Question 1.2)

**Schema T1 [Claude Opus; grammar by Astra].** A T1 word interleaves m leaf jumbles (ℓ_i, h_i), n cashing jumbles
(lowerings) (x_j, k_j), q returns at the leaves and R retained letters (releases and re-clearing chains); its length is
m + n + q + R. Suppose every letter is admissible when it acts (each a finite exact test: the leaf cleans its holder by
one exact image of cap ℓ_i; the retained letters by Lemma RS; each cashing and return at a clean grip). Then
ΔΦ = Σ of the event changes 2(F − D) of the jumbles, each read when it acts, and the excursion is the largest
chronological prefix sum; it equals the sum of the raises when every raise comes before every negative change. The word
descends exactly when the total is negative.

*Proof.* Lemma Φ at each jumble, retained letters keep heights, and Theorem MC certifies an instance event by event. ∎

The sealed instances O-D1 (which cashes at 504 and 576 and returns at neither leaf), O-D3, O-D4, O-D5 and O-D6 parse in
this grammar; a one-target, one-return-per-leaf grammar does not contain O-D1 [Astra].

## 6. Negatives for stated alphabets (Questions 1 and 3)

**Lemma IC (independent clean caps) [Astra].** Let the alphabet be all menu items at C ∪ B, where the grips of C are
clean at X and pairwise non-interacting and those of B are blocked. For c ∈ C and an item a at c let e_c(a) be the exact
change of Φ on I_c, and suppose e_c(a) ≥ 0 for all a. Let N_c = {a : e_c(a) = 0} and γ the least positive e_c(a). For
b ∈ B let μ_b = |S_b outside ∪_c I_c| + Σ_c min_{a∈N_c} |{A ∈ I_c : A straddles b after (c, a)}|. If every μ_b > 0, the
configurations obtained by applying independently one product a_c ∈ N_c at each c form a negative certificate in the
sense of Proposition BN for every even E < γ, with at most ∏_c |N_c| members.

*Proof.* The caps of C do not interact, so their supports are disjoint and each product acts on its own cap
(Proposition 1C). Every member keeps every b ∈ B blocked: its straddlers outside the caps never move, and inside the
caps at least μ_b of them remain. A letter at b is therefore never admissible, and a letter at c changes its product to
another item of N_c (Φ unchanged, a member) or raises Φ by at least γ > E. So the set is closed under every successor
below the ceiling Φ(X) + E, and no member has Φ below Φ(X). ∎

Instances [Astra; checked by Claude Opus]. Each bounds only words over its alphabet:

| Jam | Alphabet | C | Facts | Conclusion |
| --- | --- | --- | --- | --- |
| 7304 | {23, 41, 2} | {23} | all 60 images at 23 leave at least 9 straddlers at 41; the 101 straddlers of 2 lie outside I_23; 23 has F 197 and D 0 for every jumble (retained products Φ 1172, jumble products 1566) | only 23 can ever act; its orbit of at most 60 states has no descent: no descent at any length or excursion |
| 7305 | {134, 421, 431, 305, 389, 504, 576} | {134, 421, 431} | (F, max D) = (209, 0), (255, 45), (0, 0); \|N_c\| = 12, 12, 60; γ = 418; μ = 11, 15, 6, 2 at 305, 389, 504, 576 | a certificate of at most 8,640 states with E = 416: no descent with excursion below 418, no tame release of 504 or 576 |
| 7306 | {233, 598, 536, 370} | {233, 598} | (F, max D) = (78, 0), (146, 0); N_c the 12 retained items; γ = 156; μ_536 = 3, μ_370 = 48 | at most 144 states, E = 154: no descent with excursion below 156, no tame release of 370 |
| 7307 | {543, 597, 517} | {543} | F 111, max D 0; the 12 retained products leave at least 26 straddlers at 597 and 74 at 517 | at most 12 states, E = 220: a descent needs excursion at least 222 |

Each of these alphabets misses a helper that the certified tame descents use (94 and 499 at 7305, 561 at 7306, 448 at
7307, the routers 20 and 112 at 7304), so none bounds the unrestricted E*, which is 0 at all six jams (README section
2.2). At 7307 the bound fails as soon as 448 is added: 448 and 543 share 401 pieces, so Lemma IC does not apply.

**Proposition N5 (307 at 7306) [Astra; confirmed by both].** At the 7306 jam the fixed-point iteration of the levels
(stage 14, Lemma L4, `levels.py`) converges in eight steps with lev(307) = 4 and lev(431) = 3. By L4 no word of at most
three letters releases 307, whatever its alphabet or excursion, and a word whose lowering acts at 307 has at least five
letters. ∎

**Schema bounds [Claude Opus; scope by Astra].** At 7305, in the schema whose leaves are 134 and 421 (the only clean
holders of the straddlers of 305 and 389), with single relays at grips clean at the jam and all raises before any
refund: every jumble at 134 costs 2F_134 (D_134 = 0 for every jumble), at least 328 after its only relay (94, −45);
every jumble at 421 costs at least 420 (F 255, max D 45, no relay); the lowering at 504 needs both releases. So the
schema needs E ≥ 748, and O-D4 attains it. At 7306 the corresponding bound is 256, attained by O-D6. Both bounds hold
only for that schema and that order of raises and refunds; adding separate least costs does not bound words that
interleave refunds.

## 7. The procedure P** (Question 4)

**Proposition P (termination) [both].** Let a state-only procedure, at each step, search a finite set of declared
finite families at the current configuration, certify every candidate with Theorem MC, and commit one only when it
lowers Φ (so by at least 2, since Φ is even) with excursion at most E₀. Then it commits at most Φ₀/2 macros from a
configuration of depth Φ₀, every step ends, and Φ never exceeds Φ₀ + E₀; on a stretch of tame commits Φ never rises.

*Proof.* Φ is a non-negative even integer and each commit lowers it by at least 2. Each step is a finite union of finite
enumerations. Along a committed macro Φ stays below Φ(start of the macro) + E₀ ≤ Φ₀ + E₀. ∎

The choice rule, E₀ and the families are declarations, not theorems; neither key (terminal Φ first, or tame first) is
proved to avoid later jams (README section 2.7, DIS-C).
