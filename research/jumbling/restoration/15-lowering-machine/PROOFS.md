# Stage 15: proofs of the joint attack on card 7 (24 October 2026)

The results of the joint attack on [card 7](../../theory/cards/card-7-lowering-machine.md), with proofs. The process,
the computed facts and the answers to the card's questions are in [README section 2](README.md#2-joint-attack-24-october-2026).
Each result below was proved in one of the two sealed analyses or in the cross-examination, and its proof was checked
line by line by the other analyst and by the integrator. The source of each is given in brackets: Astra, Claude Opus
(the second analyst), or both.

Notation is that of [stage 14](../14-blocking-structure/README.md) (section 1), its [`PROOFS.md`](../14-blocking-structure/PROOFS.md)
and card 7. Items at a grip are numbered 0 to 59 (0 the identity, 1 to 11 retained, 12 to 59 jumbles); a·g means g
first, then a; h_L is the left factor of an item h, and the left factor of a product is the product of the left
factors. A twist (c, h) maps the tree position of every piece it moves by h_L (card 5, T5). φ(A) = max(0, h(A) − 2), so
Φ = Σ_A φ(A). J_x is the foreign set of x (F_x = |J_x|), O_x = S_x ∪ J_x its obstruction, and N(x) its 80 partners
(stage 14, L1).

A **word** W = e_1 ⋯ e_N, e_t = (y_t, g_t), is read left to right. Its **trace** from Z is Z_0 = Z and Z_t = e_t Z_{t−1}
while e_t is admissible at Z_{t−1}; δ_t = Φ(Z_t) − Φ(Z_{t−1}). The **excursion** of W from Z is
E = max(0, max_{1≤k≤N} Σ_{t≤k} δ_t), the last letter included. W is **tame** when every δ_t ≤ 0 (a word with E = 0 need
not be tame: a profile 10, 8, 10 has E = 0). A **macro** is a word whose last letter is a jumble (its **lowering**) and
which lowers Φ.

Two facts about the menu are used throughout:
- **Same-cap letters compose** (theory draft, Proposition 2.2): if c is clean, (c, g₁)(c, g₂) is admissible and equals
  (c, g₂·g₁) on the same inside set; after a letter at c, c is clean.
- **The menu at a grip is a group of order 60** (it contains the twelve retained items as the subgroup A4_c). Checked
  with exact matrices at 522, 540 and 516 (all 3,600 products are menu items; Claude Opus); every grip follows, since
  Λ_c = F_c Λ₀ F_c⁻¹ with F_c ∈ K⁺ (`sim/twists.py`, `TwistMenu`).

## 1. Label tables along a word (Questions 1.1 and 3.1)

**Lemma RS (retained sequences) [Claude Opus].** Let w = (c_1, k_1) ⋯ (c_m, k_m) be a word of retained items. Put
ℓ_0 = ℓ_Z, and for t ≥ 1 let I_t be the rows in at c_t in ℓ_{t−1}; let ℓ_t(A, d) = ℓ_{t−1}(A, k_t⁻¹d) for A ∈ I_t and
every grip d, and ℓ_t(A, ·) = ℓ_{t−1}(A, ·) for the other rows; keep every height, and map the branch of each A ∈ I_t
by σ_{k_t}. Then w is admissible at Z exactly when no column c_t of ℓ_{t−1} has a straddle label. In that case its t-th
letter moves exactly I_t, the tables ℓ_t with these heights and branches are those of the configuration after t letters,
and Φ is constant along w.

*Proof.* Induction on t. A letter is admissible exactly when no piece straddles its grip, and it moves exactly the pieces
inside its cap (`sim/state.py`, `apply`). Lemma L (stage 14, section 2) gives the new rows, the heights and the branches;
retained items change no height (card 5, Lemma 5.2). ∎

Labels, heights and branches do not determine second vertices, so the D-values at the end need the exact poses (or the
left tree positions) carried along with the table [Astra].

**Corollary CR (column reading) [Claude Opus].** Let c be clean at Z, k a retained item at c, Z′ = (c, k)Z, x a grip and
h a jumble at x. A piece A ∈ I_c(Z) straddles x at Z′ exactly when it straddles k⁻¹x at Z; it is in at x at Z′ exactly
when it is in at k⁻¹x at Z; it is foreign at x at Z′ exactly when it is foreign at k⁻¹x at Z; and it counts in
D_x(Z′, h) exactly when it counts in D_{k⁻¹x}(Z, k⁻¹hk). The other pieces keep all of these. So S_x, F_x and D_x(h)
after one retained letter are read from the table at Z: in the column k⁻¹x for the moved rows, in the column x for the
others.

*Proof.* The labels are Lemma L. Heights are kept and the branch becomes σ_k(b(A)); since CLS(x) = σ_k(CLS(k⁻¹x)),
σ_k(b(A)) ≠ CLS(x) exactly when b(A) ≠ CLS(k⁻¹x), which gives the foreign test, and σ_k(b(A)) = CLS(x) exactly when
b(A) = CLS(k⁻¹x). The tree position of A at Z′ is k_L t(A), and k_L fixes v₀, so the second vertex of A at Z′ is
k_L s(A), where s(A) is its second vertex at Z. The condition k_L s(A) = h_L⁻¹v₀ is s(A) = k_L⁻¹h_L⁻¹k_L v₀ =
(k⁻¹hk)_L⁻¹v₀, and k⁻¹hk is a jumble of Λ_{k⁻¹x} = k⁻¹Λ_x k (stage 14 README, proof of Lemma L). ∎

**Lemma CL (class calculus) [Claude Opus; scope by Astra].** Every item g at a grip c permutes the 120 poles of the
grips of class CLS(c); a jumble maps no pole of another class to a pole. Hence, for a letter (c, g) at a clean c, a
piece A ∈ I_c and every grip d of class CLS(c), the label of A at d after the letter is its label at g⁻¹d before it,
where g⁻¹d is the grip with pole g⁻¹n_d. Unmoved pieces keep their labels. For a grip d of another class, the label
of a moved piece after a jumble needs an exact evaluation of its image.

*Proof.* The pole fact was checked with exact matrix-vector products for all 60 items at every grip (Claude Opus) and,
independently, at the grips 0, 229, 451 and 522, one of each of four classes (the integrator). It also follows from
grip 0 alone: Λ_c = F_c Λ₀ F_c⁻¹ with F_c ∈ K⁺ mapping grip 0 to c, and an element k of K⁺ maps the poles of class Q
onto those of class σ_k(Q) and non-poles to non-poles. For the labels: g is orthogonal, so n_d·(gy) = (g⁻¹n_d)·y,
and g⁻¹n_d is the pole of a grip of class CLS(c) with the same cut constant; the moved region gR therefore lies
inside, outside or across H_d exactly as R does for H_{g⁻¹d}. ∎

The lemma is about moved rows: the holder set of a set of pieces transforms this way only when the whole set receives
the same map [Astra].

**Proposition SL (single-letter clearing) [Claude Opus].** Let c be clean and M_c(e) the number of pieces of I_c that
straddle e. A retained letter (c, k) changes |S_d| to |S_d| − M_c(d) + M_c(k⁻¹d) for every grip d, and a letter (c, g)
does the same for every grip d of class CLS(c) with g⁻¹d in place of k⁻¹d. So the retained letter makes a blocked d
clean exactly when S_d ⊆ I_c and M_c(k⁻¹d) = 0; such a letter is admissible and Φ-neutral.

*Proof.* Unmoved pieces keep their labels; the moved pieces of I_c straddle d afterwards exactly when they straddled
k⁻¹d (Lemma L), respectively g⁻¹d (Lemma CL). ∎

**Corollary OH (no own holder) [Claude Opus; scope by Astra].** For a piece A of height at least 4, let N_own(A) be the
number of grips of class b(A) at which A is in. If N_own(A) = 0 for every piece of height at least 4 at Z, then along
any word from Z the first letter that changes Φ raises it. So every word from Z that lowers Φ has excursion at least 2;
in particular no tame word lowers Φ.

*Proof.* Consider the letters before the first that changes Φ. We show by induction that each of them moves a piece of
height at least 4 only if it is retained, that no piece reaches height 4 or more, and that every piece of height at
least 4 keeps N_own = 0. A retained letter (c, k) keeps every height and maps the holder grips of a moved piece by k and
its branch by σ_k; since CLS(k·e) = σ_k(CLS(e)), the grips of class σ_k(b(A)) that hold A afterwards are the images of
those of class b(A) before, so N_own is kept. A jumble (c, g) with ΔΦ = 0 has F_c = D_c(g) (Lemma Φ). Every piece
counted in D_c(g) has branch CLS(c), height at least 4 and is in at c, so it would have N_own ≥ 1; hence D_c(g) = 0
and F_c = 0. So no piece of height at least 2 of another branch lies in cap c, and no piece of height at least 4 at all
(one of branch CLS(c) would have N_own ≥ 1): the letter moves no high piece, raises lattice pieces to height 2, and keeps
or lowers to 0 the height-2 pieces of branch CLS(c). Now let e = (c, g) be the first letter that changes Φ. It is a jumble, and D_c(g) = 0 by the same
argument, so ΔΦ = 2F_c > 0. ∎

The hypothesis is per piece: a unit's own holders in the sense of stage 14 require every member. At the chain ends every
high piece has N_own of 1 or 2 (Claude Opus), so OH certifies nothing there.

**Proposition NJ (neutral jumbles move straddles) [Astra; replayed by the integrator].** Corollary L1 of stage 14
(straddle numbers and holder numbers are kept) does not extend to Φ-neutral jumbles. From solved, the letter (0, 12)
keeps Φ = 0 and makes piece 0 straddle grip 1: two of its exact vertices have signed margins (−847 + 363√5)/500 and
(847 − 363√5)/500 at H_1. The integrator's replay finds 31 straddlers at grip 1 after the letter, end digest
`b9f09a41…`. So the Φ-neutral alphabet of card 7, Question 3.1, is the retained letters at clean grips and the jumbles
at clean grips with F = D (Lemma Φ), which may be impure; the invariants of Lemma L and L5 hold for its retained part
only. ∎

## 2. Macros (Questions 1.1 and 1.3)

**Theorem MC (chronological macro certificate) [Astra; the table tests by Claude Opus].** A certificate of a word
W = e_1 ⋯ e_N at Z consists of the exact pose table of Z with its digest, the word, and for each event t a claimed
support S_t (a set of pieces) and a claimed δ_t. Reconstruct poses Q⁰ = the poses of Z and, for t = 1, …, N, require:
- no piece straddles y_t at Q^{t−1};
- S_t is exactly the set of pieces that are in at y_t at Q^{t−1};

then put Q^t = g_t Q^{t−1} on S_t and Q^t = Q^{t−1} elsewhere, and require δ_t = Σ_{A∈S_t} [φ(Q^t_A) − φ(Q^{t−1}_A)].
If every test passes, W is admissible at Z, its t-th letter moves exactly S_t, Z_t has the poses Q^t, and

    Φ(Z_k) = Φ(Z) + Σ_{t≤k} δ_t   for every k.

So W lowers Φ exactly when Σ_t δ_t < 0; its excursion is max(0, max_k Σ_{t≤k} δ_t); it is tame exactly when every
δ_t ≤ 0. Conversely every admissible word has exactly one such certificate. Letters may repeat grips, have overlapping
supports and act at the eventual target.

*Proof.* By induction, Z_{t−1} has the poses Q^{t−1}. The letter e_t is admissible exactly when no piece straddles y_t,
and it then moves exactly the pieces inside cap y_t, each by g_t; so Z_t has the poses Q^t. φ is a function of the pose
(tree arithmetic), Φ is the sum of φ over the pieces, and only the pieces of S_t change pose at event t. Summing the
δ_t telescopes. The supports of an admissible word are determined by it. ∎

The tests are finite and exact at Z [Claude Opus]:
- **T1 (table).** A piece that has not moved before event t has at y_t its label in the table of Z.
- **T2 (calculus).** For a moved piece, walk its history backwards: each retained letter (c, k) replaces the grip d by
  k⁻¹d (Lemma L), each jumble (c, g) with CLS(c) = CLS(d) replaces d by g⁻¹d (Lemma CL); at the start, read the table
  of Z.
- **T3 (exact image).** When the walk meets a jumble at a grip of another class than d, evaluate the current pose of the
  piece at y_t exactly.
- **T4 (tree arithmetic).** δ_t = 0 for a retained letter (Lemma 5.2); δ_t = 2(F − D) for a jumble at a clean grip
  (Lemma Φ), with D from the exact second vertices.

Bounds. The length is N. If the last letter is at x and x is blocked at Z, then N ≥ lev(x) + 1 (L4); there is no upper
bound in terms of the level. A certificate that also fixes a partial order on the events, with every topological order
admissible (Astra's occurrence DAG, the remark below), has N ≤ 600r for a DAG of height r: same-grip events are
comparable, so each of the r antichains has distinct grips.

All seventeen descents of README section 2.2 satisfy this certificate. The table tests T1, T2 and T4 suffice for R01,
R02, R05, R07 to R14, R17 and the first four letters of R04; R03, R04, R06, R15 and R16 need one to five exact image
evaluations of whole moved sets (Claude Opus).

**Corollary SRR (retained release) [Claude Opus; length bound by Astra].** Let w be a word of m retained letters that is
admissible at Z (Lemma RS), after which x is clean, and h a jumble at x with F_x(wZ) < D_x(wZ, h). Then W = w·(x, h) is
a tame macro and Φ(WZ) = Φ(Z) + 2(F − D). Admissibility, cleanliness and F are table tests at Z (Lemma RS, Corollary
CR); D needs the second vertices from the exact poses. With distinct grips, m ≤ 600 (letters at x itself, re-aims,
included); if x is blocked at Z, m ≥ lev(x).

*Proof.* Lemma RS, then Lemma Φ at the clean grip x; retained letters have δ = 0. ∎

The letters of w need not interact with x, need not be clean at Z (only when they act) and may overlap: an **enabler**
can free a partner of x without touching cap x (at 7304, 277 is not a partner of 162). Thirteen of the seventeen
descents are SRR words; a fourteenth, R06, is tame with a Φ-neutral jumble helper (README section 2.2).

**Corollary T3PE (precedence peel with an excursion) [Claude Opus; schedule by Astra].** In Theorem T3P (stage 14,
section 2) replace P2 and P4 by the single test Σ_i ΔΦ_i + 2(F − D) < 0. Then the word W = w·(x, h) of Lemma A,
with a fixed topological order π, is admissible, Φ(WZ) = Φ(Z) + Σ_i ΔΦ_i + 2(F − D) < Φ(Z), and its excursion is
max(0, max_k Σ_{j≤k} ΔΦ_{π(j)}) ≤ Σ_i max(0, ΔΦ_i). For a clean helper, ΔΦ_i = 2(F_{y_i} − D_{y_i}(g_i)) when g_i is a
jumble, 0 when it is retained.

*Proof.* The proof of Lemma A uses only P0 and P1 (stage 14, section 2; P2 enters only in the proof of T3P, for
tameness). With Lemma A, wZ = Z′, S_x(Z′) = 0 by P3, and Lemma Φ applies to the last letter at the clean grip x. ∎

The excursion depends on the order: store the order that is executed. The deletion argument of stage 14 (a minimal
certificate with clean helpers uses only partners of x, so at most 81 letters) needs P2: without it a descending
non-partner helper may pay for raising helpers, and deleting it can destroy the net descent [Astra]. The bound 81 holds
only for a family that is restricted to partner helpers. R03, R16 and the first round of R15 are T3P-E rounds with one
helper of ΔΦ = +20 each (README section 2.2).

**Lemma RA (re-aim) [both; historical substitution by Astra].** Let x be clean at Z, h an item at x, Y = (x, h)Z and a
an item at x. Then (x, a) is admissible at Y and (x, a)Y = (x, a·h)Z. For a retained a this keeps Φ and F_x, and the
twelve lowering items A4_x·h of a lowered set place it in twelve ways, decided by one table: after (x, a·h) a moved
piece has at a grip d the label, height and foreign status that it has at a⁻¹d after (x, h).

*Proof.* Same-cap letters compose; retained letters keep heights; the rest is Corollary CR with c = x (stage 14, S2(b)
with y = x). ∎

Replacing an earlier letter (x, h) of a history u·(x, h)·v by (x, a·h) gives the configuration (x, a)·(u·(x, h)·v)Z
when (x, a) commutes with every letter of v; this is sufficient, not necessary (letters of v can cancel). Letters at
grips that do not interact with x commute with it (L1, L10); letters at x itself in general do not (b·a·h ≠ a·b·h), so
otherwise the substituted history must be checked as a word of its own [Astra]. At the 7304 chain end the lowering (522, 14) is the last
letter, so (522, 8) gives exactly the configuration of W5 after (522, 49) (8·14 = 49 at 522; digest `d4851f18…`).

**Remark (occurrence DAG) [Astra].** A stronger certificate orders only the events that share a piece (and the events
at one grip). Each piece then has a fixed local history; testing in at the acting grip for its own events and out at
every compatible prefix of its history for the other events proves, by induction over the executed order ideals, that
every topological order is admissible and reaches the same end. Its worst excursion is the largest sum of δ over an
order ideal. No procedure of this stage needs order independence; the tools check the fixed chronological order.

**Lemma CJ (conjugated lowering) [Claude Opus; checked by Astra].** Let u = (c_1, g_1) ⋯ (c_r, g_r) be admissible at
Z (u need not be tame), Y = uZ, x clean at Y, M = I_x(Y), h a jumble at x and Y′ = (x, h)Y. Suppose every piece of M
is out at every c_i at its pose in Y′. Then u⁻¹ = (c_r, g_r⁻¹) ⋯ (c_1, g_1⁻¹) is admissible at Y′, the end
configuration equals Z off M and Y′ on M, and its Φ is Φ(Z) + Σ_{A∈M} [φ(hY_A) − φ(Z_A)].

*Proof.* Let Z_i be the trace of u. By induction along u⁻¹: before the letter (c_i, g_i⁻¹) every piece outside M has
its pose in Z_i, and every piece of M has its pose in Y′. At Z_i the grip c_i is clean (a letter leaves its grip
clean), so the pieces outside M do not straddle c_i, and those of M are out at c_i by hypothesis. So the letter is
admissible, moves exactly the pieces outside M that are in at c_i at Z_i, and takes them to their poses in Z_{i−1}. The
pieces of M never move. ∎

No instance has been found: with one-letter u the hypothesis failed at every pair tested (Claude Opus; for example at
240 of 7306, 206 pieces of M stay in or across 487).

## 3. The release structure (Question 1.2)

**Proposition FL (finite levels) [both].** At every reachable configuration Z every grip has a finite level, and
lev(d) ≤ |u| for every admissible word u after which d is clean.

*Proof.* The second statement is the induction in the proof of L4 (stage 14, section 4). For the first, let V be a
word from solved to Z. Its reverse, with each item inverted, is admissible at Z (a letter leaves its grip clean, and
its inverse at the same grip moves the same set back) and reaches solved, where no piece straddles any cut. ∎

So finite levels carry no information about a reachable configuration; only their values do, as lower bounds. The
reverse history unlocks every grip, but it neither keeps a target unit nor stays tame, and finiteness does not make a
release a matter of cost alone. Corollary L4′ with λ = ∞ never occurs at a reachable configuration.

**Proposition AO (release structure) [Astra; Claude Opus].** The release structure of a target x at Z is an AND/OR
graph: an AND over the straddler cells of x, where a cell is a set of straddlers of x with identical holder rows at Z;
for each cell, an OR over its holders other than x (a holder that holds part of the target unit only with a retained
letter); a clean holder is a leaf; a blocked holder expands into the AND over its own straddler cells. Its min–max
value is lev(x) (the defining equation of L4). Every word that makes x clean contains a schedule of this graph: the
first move of each straddler of x is a letter at one of its holders at Z, which is clean when it acts. Each leaf
carries two obligations that the graph does not decide: an item whose image leaves no moved piece across x or across a
grip that must still act, and the cost of that image (new straddlers, new foreign pieces, δ). The foreign pieces of x
are not part of the AND: a lowering needs F < D, not F = 0, so they enter only through that final budget.

*Proof.* A piece's labels change only when it moves, and a letter moves exactly the pieces that are in at its grip; so
a straddler of x stops straddling only by moving, and its first move is at a grip where it is in at Z, a holder at Z,
which must be clean at that moment. Applied to the straddlers of every blocked holder that a schedule uses, this gives
the expansion; L4's equation, lev(d) = 1 + the largest over the straddlers of d of the least level of their holders,
is its min–max value. ∎

The structure is a restatement of L4 and of stage 14, S1; no theorem says that a startable letter exists. At grip 2 of
7304 it collapses to the chain 23 → 41 → 2, and the obligation fails at the first step: no item at 23 cleans 41 (README
section 2.3). The obstruction is the new straddlers that the images create, not a cycle of the graph.

## 4. Negatives (Questions 2.2 and 4)

**Proposition 1C (one cap) [Astra; menu closure by Claude Opus].** If x is clean at Z, every word of letters at x alone
is admissible and acts as one of the 60 items at x on I_x(Z). So some word confined to x lowers Φ exactly when some
jumble h at x has D_x(h) > F_x, and the least change of Φ by such words is min(0, 2(F_x − max_h D_x(h))) over all
lengths and excursions.

*Proof.* Same-cap letters compose, and the menu at x is a group. ∎

At the 7304 chain end, 522 has F = 70 and D = 22 for twelve jumbles, 0 for 36; 540 has F = 111 and D = 58 for twelve,
0 for 36. At the 7307 chain end, 516 has F = 395 and D = 7 for twelve, 0 for 36. The least jumble changes are +96, +106
and +776 (both, computed). These are statements about one cap at the current configuration: letters elsewhere change
F and D (Lemma RA and the relays of README section 2.2).

**Proposition BN (bounded-excursion negatives) [Astra].** Fix an even E ≥ 0 and M = Φ(X) + E. A **negative certificate
with excursion E** at X is a finite set R of exact configurations containing X, each with Φ(X) ≤ Φ ≤ M, such that for
every member and every admissible letter whose result has Φ ≤ M, that result is in R. If one exists, no word from X with
excursion at most E lowers Φ. Conversely, if no such word exists, the set of configurations reached from X by words
whose prefixes all have Φ ≤ M is such a certificate, with at most [7200·(4·16^{q+1} − 1)/3]^177120 members, q = M/2.
A word of excursion E that lowers Φ together with a certificate with excursion E − 2 proves that E is the least
excursion.

*Proof.* Every prefix of a word with excursion at most E has Φ ≤ M, so by induction every prefix stays in R, where Φ is
at least Φ(X). Conversely, the set described is closed by construction, contains no configuration below Φ(X) (the word
reaching it would be a descent), and is finite: Φ ≤ M bounds every height by 2q + 2, and there are
(4·16^{q+1} − 1)/3 tree positions of height at most 2q + 2 (card 5), with 7200 poses each. Φ is even, so the least
excursion is even. ∎

Closure under neutral letters alone (stage 14, Proposition N) is not enough once E > 0: a word may raise, fall part of
the way and raise again. A certificate restricted to the letters of a declared family (the stop record of P*,
section 6) proves only that the family has no macro there. At the 7304 chain end the tame word R05 shows that no
certificate with excursion 0 exists, although the T3P scan is empty.

## 5. The lowering machine (Question 2.1)

**Theorem GC (Good-set controller) [Astra].** Let x and z be clean at Z, with I_x ∩ I_z = G exactly, where G is a unit
(one pose) of height H ≥ 4; put B_x = I_x ∖ G and B_z = I_z ∖ G. Good_x is the set of items a at x such that every
piece of B_x at its pose times a is out at z; Good_z likewise. A controller state is (Q, a_x, a_z): the pose of G and
the products of the letters so far at x and at z. A letter (c, g), c ∈ {x, z}, is allowed when g·a_c ∈ Good_c, G at Q
is wholly in or wholly out at c, and G's height after the letter is at most H. Then every word of allowed letters is
admissible at Z; after it, B_x has its Z-poses times a_x, B_z times a_z, G has pose Q (updated only by the letters
whose cap holds it) and every other piece its Z-pose. Along such a word

    Φ ≤ Φ(Z) + 2(f_x + f_z),

where f_c is the number of pieces of B_c of height at least 2 and branch other than CLS(c) at Z. If at the end a_x and
a_z are retained and G has height at most H − 2, then Φ has fallen by at least 2|G|. The states are finite (at most
|Good_x||Good_z|·7200·(4·16^{H/2} − 1)/3), so if an accepting state can be reached, a word of at most that length
minus one reaches it.

*Proof.* This is the isolation lemma of X1 (stage 10, REPORT section 2.3) with the target G: before a letter at x, the
pieces of B_x are in cap x (a_x ∈ Λ_x keeps cap x), those of B_z are out at x (a_z ∈ Good_z), G is wholly in or out by
the rule, and every other piece has its Z-pose, at which it is neither in nor across x (I_x ∩ I_z = G and x is clean
at Z). So x is clean and the letter moves B_x and, when it holds G, G. The identity is in both Good sets, since B_x is
out at z at Z. For the bound: the change of φ over B_c is that of the single item a_c applied at Z (Lemma 5.2), which
raises only the f_c pieces, by 2 each (lattice pieces end at height 0 or 2, own-branch pieces keep or lose 2), and G
never exceeds H. With retained products the debris keep their heights, and G falls from H to at most H − 2. The count
of states: (4·16^{H/2} − 1)/3 tree positions of height at most H (card 5), 7200 poses each. ∎

**Lemma LC1 (lens cell, P1 form) [Astra].** In the setting of GC, let a at x lower G by 2 and b be an item at z with:
aB_x out at z, aG in at z, bB_z out at x, and baG out at x. Then (x, a)(z, b)(x, a⁻¹)(z, b⁻¹) is admissible; it returns
every piece outside G to its pose and moves G by a; G passes through G, aG, baG, baG, aG; Φ falls by exactly 2|G| and
never exceeds Φ(Z) + 2(f_x + f_z).

*Proof.* Trace the inside sets. (x, a) moves B_x ∪ G. At z: B_z in, aB_x out, aG in; (z, b) moves B_z ∪ aG. At x: aB_x
in, bB_z out, baG out; (x, a⁻¹) moves aB_x back. At z: bB_z in, baG in (b keeps cap z), B_x out at its Z-pose; (z, b⁻¹)
moves bB_z back and baG to aG. G's height is H − 2 after a and at most H after b (a letter changes a height by at most
2); the debris change by at most 2f_x and 2f_z. ∎

A sequence of (H − 2)/2 such cells, each with an a that sends G into the class of the other grip and the roles of x and
z exchanged, takes G to height 2 in 2(H − 2) letters; the debris are back at every cell boundary, so the bound does not
accumulate. The existence of each next cell is a separate test.

**Lemma LC2 (lens cell, P2 form) [Claude Opus; scope by Astra].** Let x and y be clean at Z, K = I_x ∩ I_y,
D_x = I_x ∖ K and D_y = I_y ∖ K, a an item at x and b an item at y such that b(K ∪ D_y) is out at x and a⁻¹D_x is out
at y. Then (y, b)(x, a⁻¹)(y, b⁻¹)(x, a) is admissible, returns every piece outside K and moves K by a; its Φ profile is
Φ, Φ + β, Φ + β + α′, Φ + α′, Φ + Σ_{A∈K} [φ(aA) − φ(A)], where β is the change by b on K ∪ D_y and α′ that by a⁻¹ on
D_x.

*Proof.* (y, b) moves K ∪ D_y. At x: D_x in, b(K ∪ D_y) out; (x, a⁻¹) moves D_x. At y: b(K ∪ D_y) in, a⁻¹D_x out;
(y, b⁻¹) moves K ∪ D_y back. At x: a⁻¹D_x in, K in, D_y out (I_x ∩ I_y = K); (x, a) moves D_x back and K to aK. ∎

K need not be a whole unit or of one pose, and x and y need not have different classes. Its first letter can raise K,
so LC1's excursion bound does not carry over (β includes K).

**Proposition RP (reverse pump) [Claude Opus; return corrected by Astra].** In the setting of GC, let x have class Q,
z class P ≠ Q, and G branch Q and height H = 2m ≥ 4.
- (a) *Lowering phase.* A word of allowed letters that alternate between x and z, each in G's lowering coset at its
  grip and sending G into the class of the other grip, lowers G by 2 at each letter. After j letters
  Φ = Φ(Z) − 2|G|j + E_x(a_x) + E_z(a_z), where E_c(a) = Σ_{A∈B_c} [φ(aA) − φ(A)] ≤ 2f_c, and E_c(a) = 0 for a
  retained a.
- (b) *Return.* Let G end at height 2, let c be the grip that acted last on G and c′ the other. G has branch CLS(c′)
  and lies in cap c. A letter at c′ keeps G's height or lands it (a height-2 piece of branch CLS(c′)). If a_c is not
  retained, the return letter at c that makes its product retained is a jumble: if G is still in cap c, it raises G
  from 2 to 4, or from 0 to 2 if a letter at c′ has landed G. So, when a_c is not retained, both debris sets return to
  their heights with G at height at most 2 only if, before c's return, G has left cap c (by a landing letter at c′
  whose image of G is out at c, as in L9, or as in the lens cell LC1) or has been landed by a letter at c′ (it then
  ends at height 2, of branch CLS(c)). When a_c is retained, c has no return to make.
- (c) L9 (stage 14, section 6) is the case H = 4 with one lowering at x, a landing at z and a return at x only: I_x ∖ G
  returns exactly, and I_z ∖ G stays moved by g, with its Φ change at most 0 by the hypothesis F_z ≤ D_z(g).

The hypotheses are the GC tests at every letter, including the return letters: the exact pose and height of G must be
checked at each of them, since a return letter can move G and raise it again [Astra]. The length is at most the number
of lowering and landing letters plus two; the excursion is at most 2(f_x + f_z).

*Proof.* (a) GC and Lemma 5.2. (b) A letter at a grip maps its cap onto itself, so G is in cap c after c's last letter
on it. A jumble at a grip of class P raises by 2 every piece in its cap of height at least 2 and branch other than P,
and every lattice piece there, into branch P (Lemma 5.2); it keeps or lowers to 0 a height-2 piece of branch P. The
return item at c is r·a_c⁻¹ with r retained; A4_c is a subgroup, so it is retained exactly when a_c is. (c) is L9's
proof read in this setting. ∎

**Proposition V (whole pumps) [Astra; carry-over from X1 by both].** For a fixed pair x, z, a whole pump from a controller
state s exists exactly when s lies in the backward viability set of GC's controller restricted to the pump letters of
RP: V_0 the accepting states (both products retained, G at height at most 2, every return letter tested), and V_{j+1}
the states with an allowed pump letter into V_j. Membership is a finite exact computation, and a stored choice per state
constructs the word. Non-empty Good sets and three branch-compatible lowering items per rung do not imply membership.
From X1 the isolation lemma carries over; the step lemma's statement that every jumble raises K does not (a lowering
needs an item of the right coset), and neither do the X1 perturbation and covering lemmas, which control a pump that
raises K without bound.

*Proof.* The controller's states are finite and its transitions are tests at a state (GC); reachability of a finite set
in a finite graph is decided by the backward iteration. ∎

## 6. The procedure P* (Question 4)

The families of macros (T, R, M, E, L), the choice rule and the stop record are defined in README section 2.4.

**Proposition P (termination) [both].** P* commits a macro only when Theorem MC certifies it at the current
configuration with Φ lower at its end and an excursion of at most E₀ (`macro.py` rejects every other candidate, of
every family, before it is committed). Then every run of P* ends after at most Φ(Z₀)/2 committed macros, and along the
whole executed word Φ ≤ Φ(Z₀) + E₀. A run that stops with Φ > 0 records, for each family, whether its search at the
stop was exhaustive; an exhaustive empty family is a negative certificate restricted to that family (Proposition BN
with the family's letters), not for all words.

*Proof.* Φ is a non-negative even integer, so each committed macro lowers it by at least 2; its excursion is at most E₀
above its own start, and its start is at most Φ(Z₀). ∎

Nothing here proves that P* reaches Φ = 0, or that it jams, at any configuration other than those it was run on
(README section 2.5).
