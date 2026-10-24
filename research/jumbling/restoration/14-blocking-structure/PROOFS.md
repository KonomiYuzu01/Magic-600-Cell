# Stage 14: proofs of the joint attack on card 6 (24 October 2026)

The results of the joint attack on [card 6](../../theory/cards/card-6-blocking-structure.md), with proofs. The process,
the computed facts and the answers to the card's questions are in [README section 5](README.md#5-joint-attack-24-october-2026).
Each result below was proved in one of the two sealed analyses or in the cross-examination, and its proof was checked
line by line by the other analyst and by the integrator. The source of each is given in brackets: Astra, Claude Opus
(the second analyst), or merged.

Notation is that of README section 1 and card 6. Items at a grip are numbered 0 to 59 (0 the identity, 1 to 11
retained, 12 to 59 jumbles). Products of items are maps: a·g means g first, then a. h_L is the left factor of an item h;
a twist (c, h) maps the tree position of every piece it moves by h_L (card 5, T5). A jumble h at a grip of class Q
lowers by 2 exactly the pieces of branch Q whose second vertex is h_L⁻¹v₀ (card 5, Lemma 5.2). J_x is the set of
foreign pieces of x (F_x = |J_x|) and O_x = S_x ∪ J_x its obstruction. **Tame** means ΔΦ ≤ 0 at every letter.

## 1. Locality

**Lemma L1 (locality) [Claude Opus].** Let |n|² = 12 + 4√5 be the squared norm of a pole vector n_c, κ = α|n|² (so
H_c = {z : n_c·z = κ}), and R² = 96 − 32√5 the squared circumradius of the solved body: the largest squared norm of a
vertex of the polytope of cap 0, which contains a whole facet of the body, whose K⁺-images give every vertex of the
body. For grips c ≠ d put

    Δ(c, d) = R²(|n|² + n_c·n_d) − 2κ²  ∈ Q(√5).

If Δ(c, d) ≤ 0, then at every configuration every piece inside cap c has the label out at d, and every piece inside cap
d has the label out at c; in particular I_c ∩ I_d = ∅. Grips c and d **interact** when Δ(c, d) > 0.

*Proof.* Poses are orthogonal maps fixing the origin, and the home chambers lie in the solved body, so every posed region
lies in the closed ball of radius R and its interior in the open ball. Antipodal grips have disjoint caps. For other
grips the smallest value of |z|² on {n_c·z ≥ κ, n_d·z ≥ κ} is 2κ²/(|n|² + n_c·n_d), reached at z = t(n_c + n_d); both
constraints are active, since one alone would need n_c·n_d ≥ |n|². Δ(c, d) ≤ 0 makes this value at least R², so no
interior point of a posed region lies in both closed caps. A region inside H_d⁺ whose label at c is in or straddle has
an interior point with n_c·z > κ, a contradiction. ∎

K⁺ acts transitively on the grips and keeps n_c·n_d, so every grip has the same number of partners: 80 (computed,
README section 5.3).

**Lemma L10 (locality of change) [Claude Opus].** A letter at a grip d changes no label, no inside set and no foreign
load at a grip c that does not interact with d. So the clean and the pure status of c are decided by the last letter of
the history at c or at one of its partners. With Lemma T: if that letter was a jumble at c that lowered no high piece,
c is clean and pure.

*Proof.* The letter moves only the pieces inside d; every item at d maps cap d onto itself (it fixes the pole n̂_d), so
these pieces are inside d before and after, hence out at c (L1). The other pieces keep their poses. ∎

## 2. The precedence peel (Question 1.1)

**Data of a round** at a configuration Z:
- a target grip x of class Q, a high unit G (a pose group of pieces of branch Q and height at least 4) whose members
  all lie inside cap x, and a jumble h at x that lowers G;
- helper letters (y_i, g_i), i = 1, …, m, at distinct grips y_i ≠ x, with I_i = I_{y_i}(Z).

The **image** of a piece A ∈ I_i is the pose g_i·Z[A], and Z′ is the configuration with the images on ∪I_i and the
Z-poses elsewhere. The **contributions** at x are read on the images, with labels from the exact evaluator and heights
and branches from tree arithmetic:
- s_i and f_i: the pieces of I_i whose image straddles H_x, or lies inside H_x⁺ with height at least 2 and branch other
  than Q;
- d_i(h): the pieces of I_i whose image lies inside H_x⁺ with branch Q, height at least 4 and second vertex h_L⁻¹v₀;
- s₀, f₀ and d₀(h): the same counts over the pieces outside ∪I_i, at their Z-poses. So s₀ = |S_x(Z) ∖ ∪I_i|,
  f₀ = |J_x(Z) ∖ ∪I_i| and d₀(h) ≥ |G|;
- ΔΦ_i: the change of Φ over I_i from the Z-poses to the images.

**Hypotheses** (finite exact tests at Z: the label table and one evaluation of each helper item):
- **P0.** The I_i are pairwise disjoint and none meets G. Each y_i is clean at Z, or (**T3P+**) every straddler of y_i
  at Z lies in some I_l with an edge l → i.
- **P1.** For i ≠ j there is an edge i → j ("y_i acts before y_j") when the image of some piece of I_j is not out at
  y_i. For a helper y_i that is blocked at Z there is also an edge l → i for every l with I_l ∩ S_{y_i}(Z) ≠ ∅. This
  digraph is acyclic, and π is a topological order of it.
- **P2.** ΔΦ_i ≤ 0 for every i. For a clean helper this is Lemma Φ: g_i is retained, or F_{y_i}(Z) ≤ D_{y_i}(Z, g_i).
- **P3.** s₀ + Σ s_i = 0.
- **P4.** F < D, where F = f₀ + Σ f_i and D = d₀(h) + Σ d_i(h).

**Lemma A (precedence additivity) [Astra; T3P+ by Claude Opus].** Under P0 and P1 the word
w = (y_π(1), g_π(1)) ⋯ (y_π(m), g_π(m)) is admissible, its k-th letter moves exactly I_π(k) to the images, wZ = Z′,
and its k-th letter changes Φ by ΔΦ_π(k).

*Proof.* By induction on k we show that after k letters the pieces of I_π(1), …, I_π(k) have their images and every
other piece has its Z-pose. Let i = π(k + 1).
- A piece that has not moved has its Z-label at y_i. If it straddled y_i at Z, then by P0 it would lie in some I_l
  with an edge l → i, so l would come earlier in π and the piece would have moved. So it does not straddle y_i, and it
  is inside y_i exactly when it lies in I_i; the pieces of I_i have not moved, because the sets are disjoint.
- A piece that has moved lies in some I_l with l earlier in π. Since π is a topological order there is no edge i → l,
  so its image is out at y_i.

So y_i is clean, the letter (y_i, g_i) is admissible, and it moves exactly I_i, each piece to its image. Labels,
heights and branches are functions of the pose and Φ is a sum over pieces, so the letter changes Φ by ΔΦ_i. ∎

**Theorem T3P (precedence peel) [merged].** Under P0 to P4 the word W = w·(x, h) is admissible and tame, and

    Φ(WZ) = Φ(Z) + Σ_i ΔΦ_i + 2(F − D) < Φ(Z).

Before its last letter, x is clean, holds G whole, and has F_x < D_x(h) for an item h that lowers G: the conclusion that
card 6, Question 1.1 asks for.

*Proof.* Lemma A gives wZ = Z′, and each helper letter changes Φ by ΔΦ_i ≤ 0 (P2). At Z′ each label is a function of
the pose, so S_x(Z′) = s₀ + Σ s_i = 0 (P3), F_x(Z′) = F and D_x(Z′, h) = D. G is untouched (P0), so x holds it whole
and h lowers it. By Lemma Φ the letter (x, h) changes Φ by 2(F − D) < 0 (P4). ∎

The helpers need not land the obstruction of x, keep its identities, or make x pure; only S = 0 and F < D at Z′ count.
At the 7304 stop the helper letter (451, 55) lands none of the obstruction of 522 and leaves F_522 = 22 at the lowering
(README section 5.3).

**Length.** |W| = m + 1. When every helper is clean at Z, a helper that does not interact with x can be deleted: its
pieces are out at x before and after its letter (L1), so S, F and D do not change, and P0 to P2 hold for the rest (an
induced subgraph of an acyclic digraph is acyclic). A minimal certificate with clean helpers therefore uses only
partners of x: m ≤ 80 and |W| ≤ 81. With T3P+ the helpers are distinct grips other than x, so |W| ≤ 600.

**Corollary B (the budget) [Astra].** Fix x, G and helpers y_1, …, y_m that are clean at Z, with pairwise disjoint
inside sets that avoid G, such that no choice of items creates a precedence edge (for example pairwise non-interacting
helpers, by L1). For each i let A_i be the set of items g at y_i with s_i(g) = 0 and ΔΦ_i(g) ≤ 0; the identity, which
leaves I_i at its Z-poses, is one of the candidates. A round with these helpers exists exactly when s₀ = 0, every A_i is
non-empty, and

    min over h of [ f₀ − d₀(h) + Σ_i min over g ∈ A_i of ( f_i(g) − d_i(g, h) ) ] < 0,

the outer minimum taken over the jumbles h at x that lower G. The minimizing items give the word.

*Proof.* P0 and P1 hold for every choice, and P2 and P3 hold exactly when s₀ = 0 and every g_i ∈ A_i (each s_i ≥ 0).
For a fixed h, F − D = f₀ − d₀(h) + Σ_i (f_i(g_i) − d_i(g_i, h)) is a sum of terms that each depend on one helper's
item, so its minimum over the choices is the sum of the minima. ∎

Minimizing F alone is not enough when the choice of items changes D: with two choices of counts (F, D) = (1, 1) and
(2, 3), the smaller F fails and the other succeeds. At 522 the helpers hold no high piece, all d_i are 0, and the rule
reduces to the smallest F.

**Corollary I (rounds) [Claude Opus].** A sequence of rounds, each a T3P certificate at the configuration the previous
round reached, is a tame word, and each round lowers Φ by at least 2. So there are at most Φ(Z₀)/2 rounds, of at most 81
letters each when all helpers are clean. Each configuration of the sequence is computed piecewise from Z₀ (Lemma A, and
the last letter of a round moves exactly the inside set of its target), so one exact replay checks the whole
certificate.

## 3. Per-cap accounting (Question 3.2)

**Lemma R (relower supply) [Astra; parts 2 and 3 by Claude Opus].** Let x be clean of class Q at Z, h a jumble at x,
and Y = (x, h)Z. Then:
1. F_x(Y) = D_x(Z, h): the foreign pieces of x at Y are exactly the pieces that h lowered from a height of at least 4;
2. the pieces of J_x(Z) are, at Y, the only pieces in cap x of branch Q, height at least 4 and second vertex h_L v₀; so
   D_x(Y, k) = F_x(Z) for each of the twelve items k ∈ A4_x·h⁻¹, which lower exactly these pieces;
3. ΔΦ(x, h) = 2(F_x(Z) − F_x(Y)), and Ψ_x = Φ + 2F_x is unchanged by every admissible letter at x.

*Proof.* The letter moves exactly I_x, which stays inside cap x, and maps each tree position t to h_L t; h_L fixes w_Q
and sends v₀ to h_L v₀, a neighbour of w_Q other than v₀. Take A ∈ I_x with position t at Z.
- t = v₀: then h_L v₀ has height 2 and branch Q.
- Height at least 2 and branch P ≠ Q: the path v₀, w_Q, h_L v₀, h_L w_P, … to h_L t does not backtrack, because
  h_L v₀ ≠ v₀ and h_L w_P ≠ w_Q (as w_P ≠ w_Q). So h_L t has height h(A) + 2 ≥ 4, branch Q and second vertex h_L v₀.
- Branch Q and second vertex s ≠ h_L⁻¹v₀: the path v₀, w_Q, h_L s, … does not backtrack (h_L s ≠ v₀), so the height
  and the branch stay, and the second vertex becomes h_L s ≠ h_L v₀.
- Branch Q and second vertex h_L⁻¹v₀: the height falls by 2; from a height of at least 4 the new branch is not Q
  (Lemma T).

So the foreign pieces at Y are the pieces lowered from a height of at least 4 (part 1), and the pieces of branch Q,
height at least 4 and second vertex h_L v₀ at Y are exactly those of J_x(Z) (part 2). An item k lowers a piece of branch
Q exactly when its second vertex is k_L⁻¹v₀; for k = a·h⁻¹ with a ∈ A4_x, k_L⁻¹v₀ = h_L a_L⁻¹v₀ = h_L v₀, because a_L
fixes v₀. Part 3: Lemma Φ gives ΔΦ = 2(F_x(Z) − D_x(Z, h)) = 2(F_x(Z) − F_x(Y)) for a jumble, and a retained letter at x
keeps Φ and F_x (Lemma L, with σ_a(Q) = Q). ∎

**Corollary U (unloading identity) [merged].** For an admissible word from Z, the descent made by its letters at c is

    −Σ_{letters at c} ΔΦ = 2(F_c(end) − F_c(start)) + 2U_c,

where U_c is the net decrease of F_c caused by the letters at other grips. A letter at a grip that does not interact
with c changes Ψ_c by its ΔΦ only (L10). *Proof.* Ψ_c changes only at letters not at c, by ΔΦ + 2ΔF_c; add up. ∎

**Corollary E (local exhaustion) [Claude Opus].** If h has the largest D_x(Z, ·) among the jumbles at x and lowers Φ,
then no word of letters at x alone lowers Φ after (x, h).

*Proof.* The lowering items at x fall into four cosets, one for each neighbour s ≠ v₀ of w_Q (the second vertex they
lower). At Y the coset A4_x·h⁻¹ has D = F_x(Z) < D_x(Z, h) = F_x(Y) (Lemma R). The other three cosets lower the branch-Q
high pieces that h did not lower, whose second vertices moved by h_L, so their D-values are those of the other three
cosets at Z, at most D_x(Z, h) = F_x(Y). So every jumble at x has D ≤ F at Y, and only those with D = F are tame. Such a
letter keeps F and, by the same argument, keeps every D ≤ F; retained letters at x permute the cosets and keep F. ∎

**Lemma P (class-P invariant) [Astra].** For a class P let J_P be the number of off-lattice pieces whose branch is not
P. Then

    Φ + 2J_P = Σ_A max(0, d(w_P, t(A)) − 1),

and every letter at a grip of class P leaves it unchanged. So J_P cannot decrease along a tame word of letters at grips
of class P.

*Proof.* A lattice piece contributes 0 to both sides; a piece of branch P and height h ≥ 2 contributes h − 2 to both;
a piece of another branch and height h ≥ 2 contributes (h − 2) + 2 = h on the left and d(w_P, t) − 1 = h on the right.
The left factor of every item at a grip of class P fixes w_P (card 5, Lemma 5.1), so it keeps every distance from
w_P. ∎

## 4. Obstructions to tame descent (Question 1.3)

**Lemma L4 (unlock levels) [Claude Opus].** Let Hold_Z(p) = {c : p ∈ I_c(Z)}. The level lev(c) ∈ {0, 1, 2, …, ∞} is the
greatest solution of lev(c) = 0 for c clean at Z, and lev(d) = 1 + max over p ∈ S_d(Z) of min over c ∈ Hold_Z(p) of
lev(c) for d blocked at Z (computed by monotone iteration from ∞ on the blocked grips). If a word from Z makes d clean
after t letters, then t ≥ lev(d); if lev(d) = ∞, d is blocked at every configuration reachable from Z.

*Proof.* We show by induction on t: if d is clean after t letters, then lev(d) ≤ t. For t = 0, d is clean at Z. Let d
be blocked at Z and clean after t letters. Every straddler p of d at Z has moved by then, since a piece keeps its labels
until it moves. The letter that first moves p is at a grip c that is clean just before it and holds p; as p has not
moved before, c ∈ Hold_Z(p), and c is clean after at most t − 1 letters, so lev(c) ≤ t − 1. By the defining equation
lev(d) ≤ 1 + (t − 1) = t. ∎

Corollary (per piece): if every c ∈ Hold_Z(p) has lev(c) = ∞, then p never moves. (Stated per piece: members of one
unit can have different holder sets, and a grip that holds only part of a unit splits it.)

**Corollary L4′ (λ) [Claude Opus].** Let λ be the smallest level of a holder of a piece of height at least 4. Every
tame word from Z that lowers Φ has at least λ + 1 letters. If λ = ∞, no tame word from Z lowers Φ.

*Proof.* A tame word that lowers Φ has a letter with D > F ≥ 0. Take the first letter with D > 0. Before it every jumble
has D = 0, so tameness forces F = 0: no piece of height at least 2 rises and no piece of height at least 4 falls, so
the pieces of height at least 4 are those of Z. That letter lowers such a piece p. The first letter that moves p is at a
grip c ∈ Hold_Z(p) that is clean before it; by L4 it is letter number lev(c) + 1 ≥ λ + 1 or later. If λ = ∞, no piece
of height at least 4 ever moves, so no letter has D > 0. ∎

**Lemma L5 (orphan cells) [Claude Opus; part (b) merged].** A cell V is a set of pieces of height 2 and branch B with one
pose and one holder set. V is an orphan when none of its holders has class B.
- (a) Under retained letters at clean grips and jumbles at clean pure grips, V moves only by retained letters and stays
  an orphan.
- (b) Under any tame word, the first jumble that moves V is at a clean holder c with D_c ≥ F_c ≥ |V|. It raises V to
  height 4 in branch CLS(c) and leaves F_c(after) = D_c(before) ≥ |V|.

*Proof.* Only letters at holders of V move V, and V is foreign at each of them, so no holder of V is pure. A retained
letter (k at c) maps the holder set to k·Hold(V) and the branch to σ_k(B), and CLS(k·e) = σ_k(CLS(e)), so V stays an
orphan; this proves (a), and also that V is still an orphan before its first jumble in (b). There V ⊆ J_c, so
F_c ≥ |V|; tameness gives D_c ≥ F_c; foreign pieces rise by 2 into branch CLS(c); and F_c(after) = D_c(before) by
Lemma R. ∎

**Lemma L6 (one grip) [Claude Opus; scope by Astra].** Let y be clean with F_y > 0 and no piece of branch CLS(y) and
height at least 4 in its cap. Then no jumble at y is tame, and a tame word of letters at y alone is one retained letter:
it moves I_y to one of its twelve retained images, whose labels at a grip x are ℓ(A, k⁻¹x) (Lemma L). This is a
statement about words confined to y; letters elsewhere can change the contents of y.

*Proof.* D_y = 0 < F_y for every jumble; retained letters at y keep F_y (Lemma L) and keep D_y = 0 (σ_k fixes CLS(y)),
and consecutive letters at one grip compose (theory draft, Proposition 2.2). ∎

**Proposition N (finite plateau certificates) [Astra].** Let q = Φ(Z)/2. Along a tame word, every letter before the first
strict decrease is Φ-neutral, so every piece has height at most 2q + 2; the number of such configurations is at most
V = [7200·(4·16^(q+1) − 1)/3]^177120 (card 5: 20·16^(k−1) tree positions at height 2k, 7200 poses each). So if some tame
word lowers Φ, one of length at most V does. A **negative certificate** is a finite set of exact pose tables containing
Z, closed under every admissible Φ-neutral letter, with no admissible letter that lowers Φ at any of its tables; it
excludes every tame descent from Z. A search that stops on a time or size limit is not such a certificate, and the bound
V is far too large to enumerate. At the 7304 stop the tame descent W3 shows that no negative certificate exists.

*Proof.* Remove the repetitions of the neutral part of a descending word; the remaining configurations are distinct and
of bounded height. Closure and the absence of descending letters exclude, by induction on length, every tame word that
leaves the set or lowers Φ. ∎

## 5. Safety for a target (Question 1.2)

Let y be clean at Z, x a grip, (y, g) a letter and U a named set of obstruction pieces of x inside cap y. Primes denote
the configuration after the letter.
- (y, g) **lands** U when it moves every piece of U onto the lattice.
- It **adds nothing** to the obstruction of x when S_x′ ⊆ S_x and J_x′ ⊆ J_x, separately (a straddler that becomes a
  foreign piece is an addition). It **leaves no moved obstruction** when no moved piece ends in S_x′ ∪ J_x′.
- It is **safe for x** [Astra] when it lands U and adds nothing; **strictly safe for x** [Claude Opus] when it lands U
  and leaves no moved obstruction. Unmoved pieces keep their labels, heights and branches, so strictly safe implies safe.
  The converse fails when a moved piece stays in the obstruction.

**Proposition S1 (the table) [Astra].** Only the pieces of I_y change, so both verdicts are decided by a finite table
over I_y: for each moved piece the old and the new straddler flag at x and the old and the new foreign flag at x, the
new ones from the exact label of its image and its new height and branch (Lemma 5.2). The letter adds nothing exactly
when no flag turns from false to true, and leaves no moved obstruction exactly when no new flag is true. The table must
contain the off-lattice pieces of I_y, not only lattice chambers: at the 7304 stop (344, 16) lands all 14 pieces of
unit 2494, adds no straddler and nothing from a lattice piece, and adds five foreign pieces at 522 (IDs 117853, 164033,
171543, 171544, 173364), height-2 pieces of branch 3 before and after. ∎

**Proposition S2 (symmetry) [merged].**
- (a) The verdicts are K⁺-equivariant: transporting the configuration, the grips and the item together by k ∈ K⁺
  (y ↦ ky, x ↦ kx, g ↦ kgk⁻¹) keeps them.
- (b) Landing coset [Claude Opus]. For a ∈ A4_y the item a·g lands the same pieces as g, and after (y, a·g) a moved
  piece has at x the label, height and foreign status that it has at a⁻¹x after (y, g). So the new flags at x, the
  contributions (s, f) at x and the strict-safety verdicts of the twelve items of A4_y·g are those of g at the twelve
  grips a⁻¹x: one image computation decides the whole landing coset. Safety also needs the old flags at x, which the
  label table gives.
- (c) The other side [Astra]. The items g·a (a first) differ from g by a retained letter applied first, which moves the
  pieces of cap y among themselves before g acts. At the solved state the evacuation table of R21F-09 is a function of
  the coset g·A4_y. At deep states the table is not: at 344 the product 25·8 is item 30, and at the 7304 stop (344, 25)
  adds 49 straddlers and 37 foreign pieces at 522, all from lattice pieces, while (344, 30) adds 24 and 32. Both
  letters are unsafe for every named U, so this example separates the tables and the counts within one coset, not the
  two Boolean verdicts.
- (d) If x and y do not interact, every item at y leaves no moved obstruction at x (L1). So does every item at y when
  y is pure, holds no piece of height at least 4, and CLS(y) = CLS(x): the moved pieces end on the lattice or in frames
  of CLS(x), which never straddle x and are never foreign there (Frames by height).

*Proof of (b).* The image of a posed region R under a·g is a(gR), which lies inside, outside or across H_x exactly as gR
does for H_{a⁻¹x}, because a ∈ K⁺ permutes the poles. The retained a keeps heights and maps branches by σ_a, and
CLS(x) = σ_a(CLS(a⁻¹x)); so a moved piece is foreign at x after a·g exactly when it is foreign at a⁻¹x after g. A
retained letter keeps heights, so a·g lands what g lands. ∎

## 6. Bounded excursions (Question 2)

**Lemma L8 (lower, then relower) [Claude Opus; premise corrected by Astra].** Let x be clean of class Q at Z, h a jumble
at x and Y = (x, h)Z. By Lemma R the pieces of J_x(Z) are, at Y, of branch Q, height at least 4 and second vertex h_L v₀.
Let z ≠ x be a grip of class Q that is clean at Y and holds them, and g an item at z with g_L⁻¹v₀ = h_L v₀, so that g
lowers them. Put a = 2(F_x(Z) − D_x(Z, h)) and b = 2(F_z(Y) − D_z(Y, g)). If a + b < 0, then (x, h)(z, g) lowers Φ by
−(a + b), and along it Φ never exceeds Φ(Z) + max(0, a).

*Proof.* Lemma Φ at Z and at Y. ∎ (The sealed version also required D > F at the first letter, which makes the
excursion 0 and excludes the raising first letters it was meant for.)

**Lemma L9 (lower, land, return) [Claude Opus].** Let h lower a unit G at x from height 4 into branch P′, and
Y = (x, h)Z. Let z be a grip of class P′, clean at Y, with I_z(Y) ∩ I_x(Y) = G, and g an item at z that lands G, such
that every piece moved by (z, g) is out at x afterwards and F_z(Y) ≤ D_z(Y, g). Then W = (x, h)(z, g)(x, h⁻¹) is
admissible; WZ equals Z except on G, which is on the lattice, and on I_z(Y) ∖ G, which is moved by g;
Φ(WZ) ≤ Φ(Z) − 2|G|; and along W, Φ never exceeds Φ(Z) + max(0, 2(F_x(Z) − D_x(Z, h))).

*Proof.* Put A = I_x(Y) and B = I_z(Y). The pieces of A ∖ G receive h and then h⁻¹, those of B ∖ G only g, and G
receives h and then g. Before the last letter x is clean: the pieces moved by g are out at x by hypothesis, and every
other piece has its Y-label (A ∩ B = G). So h⁻¹ moves exactly A ∖ G, back to its Z-poses. In Φ, A ∖ G contributes 0,
G falls from height 4 to the lattice (−2|G|), and B ∖ G contributes the middle letter's change, which is at most 0 by
Lemma Φ (G contributes nothing to it: it goes from height 2 to 0). ∎

## 7. The top layer (Question 3.1)

**Proposition P12 (the inverse lands) [Claude Opus].** If the last letter of a history was a jumble (c, h) that lowered
no piece of height at least 4 and moved a lattice piece, then (c, h⁻¹) is admissible and tame and lands that piece.

*Proof.* c is clean after a letter at c (theory draft, Proposition 2.2) and pure by Lemma T, so every letter at c is tame
(Lemma Φ). The lattice piece went to h_L v₀, and h⁻¹ maps it back to v₀. ∎

**Conjecture C13 [Claude Opus].** Every reachable configuration off the lattice has an admissible tame letter that lands
at least one piece. The analogue for letters that lower Φ is false (README section 5.3, the empty lowering layer at the
7304 stop).
