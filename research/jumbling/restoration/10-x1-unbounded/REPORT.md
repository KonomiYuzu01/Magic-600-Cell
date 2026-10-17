Verdict: SETTLED-UNBOUNDED

# X1 run report

> Imported from the X1 run of 16 to 17 October 2026 ([README.md](README.md) gives the provenance and the
> integrator's independent check). Only paths were changed: the run's `out/` is this directory, and its commands,
> which ran from an export named `repo/`, run from the repository root with `S=research/jumbling/restoration/10-x1-unbounded`.
> Run times in section 3 are the run's; the outputs stored in `programs/` are the integrator's re-runs.

X1 asks whether R(I_a), the set of configurations reachable from the solved state under menu I_a, is finite;
equivalently, whether the left heights of reachable poses are bounded. **Answer: no.** The left heights are
unbounded and R(I_a) is infinite (Theorem, section 1). The proof combines three proved lemmas (isolation, tree,
perturbation) with an exact finite certificate (`programs/CERT.py`). Floating point was used only to find
words and boxes; every claim rests on exact arithmetic (the simulator, its exact Evaluator, Q(√5)) or a written
proof.

Notation. K = (118527, 126380, 126382, 126384, 126385, 149644, 149650), seven pieces that share one pose
throughout, with 16 vertices v (the simulator's rows for the 7 pieces). A pose is an orthogonal 4×4 matrix Q,
and a letter (x, j) multiplies the pose of every moved piece by E(x, j) on the left. A piece is whole in cap x
when n_x·(Q v) − κ ≥ 0 for all its vertices (the exact Evaluator's 'in'; pieces are convex). For pieces at
lattice poses the simulator decides the same relation by a signature certificate; `CERT.py` A1b checks that the
Good sets and property (i) below come out the same with the simulator's own classification. c = 11 (branch 3),
d = 80 (branch 2). For a quaternion q = a + bi + cj + dk, L(q) is the matrix of x ↦ qx (Hamilton product) in the
coordinates of the poses: L(q) = [[a,−b,−c,−d],[b,a,−d,c],[c,d,a,−b],[d,−c,b,a]]. For a word, P_k is the product
of its first k letters with the latest letter on the left, P_k = E(x_k, a_k) ⋯ E(x_1, a_1), and P_W = P_L.

## 1. Main result

**Theorem (proved; its computations are exact).** For every t ≥ 0 the word F_t defined below is admissible from
the solved state and takes K to left height 3888 + 2(|F_t| − 2085) ≥ 3888 + 132 t. So the left heights under
I_a are unbounded and R(I_a) is infinite.

*The family.* The certificate `witnesses/CERT.json` lists η = 1/200, seven words W_0, …, W_6 of 66, 302,
302, 268, 256, 254, 298 letters at grips 11 and 80, and 113 closed dyadic boxes in [−η, η]³, each labelled with
one of the words. Let σ be the state after the first 2085 letters of `witnesses/H6030.json` (the first 156
letters of H38, then 1929 pump letters); let Q* be K's pose there. By computation B below, P_i = L(λ_i) for the
matrix product P_i of W_i, with λ_i an exact unit quaternion. Put u_0 = 1 and, for t ≥ 0, let i_t be the word of
the first box (in the listed order) that contains the vector part of u_t, and u_{t+1} = λ_{i_t} u_t. Then
F_t = (first 2085 letters of H6030) W_{i_0} W_{i_1} ⋯ W_{i_{t−1}}. The rule is exact and deterministic;
`programs/FAMILY.py N` computes F_N in Q(√5) arithmetic. Which word comes next depends on the state
(through u_t), and the lengths grow without bound, as T9-02 and T16-03 require.

*Proof.* Let Z(t) be this exact property of the configuration Y_t reached by F_t:
> every piece of D_c has its pose at X multiplied on the left by E(c, 23), every piece of D_d its pose at X
> multiplied by E(d, 33), every other piece outside K has its pose at X, and K has pose L(u_t) Q* with
> |u_t − 1| ≤ η and left height 3888 + 2(|F_t| − 2085) in branch 2.

Here X, D_c, D_d and the Good sets are as in section 2.1. Items 23 and 33 are in Good_c and Good_d. When K is
whole in both caps (which section 2.5 gives), Y_t is of kind S(h, c) (section 2.3) with labels g_c = E(c, 23),
g_d = E(d, 33). The last letter of F_t is at grip 80, and the family continues at grip 11.

- *Z(0).* The first 156 letters are admissible (A1, simulator). X is of kind S(30, d): both labels are the
  identity (item 0, in both Good sets), and K is whole in both caps at height 30 in branch 3 (A1). The next
  1929 letters satisfy the hypotheses of the step lemma (A2). By the step lemma (2.3), applied 1929 times, F_0 is
  admissible and Y_0 = σ is of kind S(3888, c), with labels (23, 33) and K at Q* = L(1) Q*. The simulator
  replays these letters and reaches K at exactly Q* (A2).
- *Z(t) ⇒ Z(t+1).* By the covering lemma (2.6), some listed box contains the vector part of u_t, so i_t is
  defined, and |u_{t+1} − 1| ≤ η. Apply W = W_{i_t} at Y_t. The labels of Y_t are those of σ and the letters
  are those checked in computation B from σ. So at every letter the grips alternate starting with 11, the
  item is a jumble, and the running product at the moving grip stays in Good. After k letters K has pose
  P_k L(u_t) Q*, where P_k is the product of the first k letters. By the perturbation lemma (2.5), with (*)
  checked in B and |u_t − 1| ≤ η, K is whole in caps c and d at every state of W (k = 0 included, so Y_t is of
  kind S(h, c)). So every letter satisfies the hypotheses of the step lemma, applied letter by letter: W is
  admissible at Y_t, every intermediate configuration is of kind S(·, ·), and every letter raises K by 2 into
  the branch of its grip. At the end of W the labels are back to (23, 33) (B), so the debris are as in Z. K's
  pose is P_W L(u_t) Q* = L(λ_{i_t}) L(u_t) Q* = L(u_{t+1}) Q*, and the last letter is at 80. That is Z(t+1).

Every word has at least 66 letters, so |F_t| − 2085 ≥ 66 t. ∎

*Required evidence (computed).* `FAMILY.py 40` builds F_40 (13,177 letters) exactly and replays it with the
repository simulator. Every letter is applied and not uncertain. At each of the 41 member boundaries K's pose
equals L(u_t) Q* exactly, and its height is 3888 + 2(letters after σ), from 3888 to 26072
(`programs/FAMILY_40.out`; witness `witnesses/F40.json`). F_40 uses words 2, 3, 5 and 6. All seven words
are also replayed with the simulator from σ, each ending with K at exactly L(λ_i) Q* (`CERT.py` D). F_3 is also
checked by the generic checker (section 3).

## 2. Proofs and the exact facts they use

### 2.1 The state X and the Good sets (computed)
X is the state after the first 156 letters of H38 (equal to the first 156 letters of H6030). `CERT.py` A1 (and
`PUMP.py` P1–P3) checks exactly:
- (I1) cuts c and d are clean at X (no piece straddles);
- (I2) K is whole in both caps, and no other piece is whole in both;
- 80 is not in NB(11);
- the grip normals are at tree distance 1, in branches 3 and 2, and K is at height 30 in branch 3.

D_c (2505 pieces) and D_d (2702 pieces) are the pieces other than K that are whole in cap c, resp. cap d, at X.
Good_c is the set of items g at c such that every piece of D_c, at its X-pose multiplied by E(c, g), is 'out' of
cap d. It has 26 items, including the identity item 0. Good_d (c and d exchanged) has 25. Both are decided by
the exact Evaluator for every piece and every item; the lists are printed in `CERT.out`. A1b repeats this with
the simulator's own `classify`, which uses signature certificates at lattice poses (62,484 piece-cases). For
every item g at x it puts D_x at E(x, g)·(X-pose) and every other piece at its X-pose. It finds the same Good
sets (good iff cut y has no straddler and no piece of D_x is in cap y). For every good g it also finds cut x
clean with D_x in cap x.

### 2.2 Tree lemma (proved)
Let K be at left vertex z of height h ≥ 2 in branch β, and let (x, a) be a jumble at a grip x whose cap holds K,
where x has branch γ ≠ β. Then K moves to height h + 2 in branch γ.

*Proof.* The left part of every letter at x fixes w_x = n_x v0, and a jumble moves v0 (packet, section 2). The
geodesic from v0 to z starts with the edge v0 → w_β, and w_β ≠ w_x. Apply a. The path w_x, a·v0, a·w_β, …, a·z
has no backtracking, because a·w_β = w_x would give w_β = a⁻¹·w_x = w_x; so it is a geodesic of length h + 1.
Prefixing v0 → w_x gives a path without backtracking (a·v0 ≠ v0), hence a geodesic of length h + 2 that starts
towards w_x. ∎

### 2.3 Isolation lemma and step lemma (proved)
*Isolation lemma.* Let w = (x_1, a_1) … (x_m, a_m) be a word with every x_i in {c, d}. Let g_c^(i) and g_d^(i)
be the products, in order, of the letters at c, resp. d, among the first i. Let Q^(0) be K's pose at X, and
Q^(i) = E(x_i, a_i) Q^(i−1) if K at Q^(i−1) lies whole in cap x_i, else Q^(i) = Q^(i−1). Assume that for every
i, g_c^(i) is in Good_c, g_d^(i) is in Good_d, and K at Q^(i−1) lies whole in cap x_i or wholly outside its
interior. Then w is admissible from X. After i letters the pieces of D_c have their X-poses multiplied by
g_c^(i), those of D_d by g_d^(i), K has pose Q^(i), and all other pieces have their X-poses.

*Proof.* Induction on i; i = 0 is clear. Let letter i + 1 be (c, a); the case d is symmetric. A letter is
admissible exactly when its item is a menu item and no piece straddles its cut (`sim/state.py`, `apply`), and it
moves exactly the pieces whole in its cap. By the induction hypothesis:
- (i) a piece of D_c lies at g·(X-pose) with g in Λ_c; g fixes n_c, so the piece is still whole in cap c;
- (ii) a piece of D_d lies at g_d^(i)·(X-pose) with g_d^(i) in Good_d, so it lies out of cap c;
- (iii) K lies whole in cap c, or out of it, by assumption;
- (iv) any other piece has its X-pose. At X it was not whole in cap c (it is not in D_c ∪ K) and did not
  straddle cut c (I1), so it lies out of cap c. A piece of D_d is not whole in cap c, by (I2).

So nothing straddles cut c. The letter moves exactly D_c and, if K lies whole in cap c, K, and the poses become
the stated ones, since a·g_c^(i) = g_c^(i+1). ∎

*Kind and step.* A configuration is of kind S(h, x), x in {c, d}, if for some g_c in Good_c and g_d in Good_d:
- the pieces of D_c and D_d have their X-poses multiplied by g_c, resp. g_d, and the other pieces outside K
  have their X-poses;
- K lies whole in caps c and d at one pose of left height h ≥ 2, in the branch of the grip other than x.

*Step lemma.* Let Y be of kind S(h, x), y the other grip, and (x, a) a letter with a a jumble, a·g_x in Good_x,
and K, turned by a, whole in cap y. Then (x, a) is admissible at Y and the result is of kind S(h + 2, y).

*Proof.* Points (i)–(iv) of the proof of the isolation lemma, applied at Y with g_c^(i) = g_c and g_d^(i) = g_d,
use only the description of Y in S(h, x), together with (I1) and (I2). They show that no piece straddles cut x,
so (x, a) is admissible. They also show that it moves exactly D_x and K. Afterwards D_x is at
(a·g_x)·(X-pose) with a·g_x in Good_x, and the other pieces outside K are unchanged. K is whole in cap y by
hypothesis, and still whole in cap x because a fixes n_x. By the tree lemma (K is in the branch of y, and x
has the other branch) K's height is h + 2, in the branch of x. ∎

Grips other than c and d keep the products they have at X. So at most 18 + 2 = 20 grips are out at every state
of this kind: the out grips stay bounded, answering the owner's question of 17 October.

### 2.4 Exact facts at the centre and along the certificate words (computed, `CERT.py` A2 and B)
- A2: the 1929 pump letters after X alternate between 80 and 11, are jumbles, and keep the running products in
  Good. After every letter all 16 vertex margins m = n·(Q v) − κ of K for both caps are > 0 (61,760 exact
  margins). At σ, K has left height 3888, the labels are items (23, 33), and the next grip is 11 (all asserted
  against the certificate). The simulator replays the same letters and reaches K at exactly Q*. Q* is
  orthogonal.
- B: for each W_i, started at σ: the grips alternate from 11, the items are jumbles, and the running products
  stay in Good. At every state along W_i (the start included), for every vertex v and both caps n, the exact
  margin m satisfies

      (*)   m − (m + κ) η²/2  >  η · sqrt(|n|² |v|² − (m + κ)²),

  checked exactly in squared form, with 2144 to 9696 checks per word. At the end the labels are (23, 33), the
  length is even, and P_i = Q_end Q*ᵀ equals L(λ_i) entry by entry, where λ_i is its first column. All 120
  letters at the two grips are orthogonal, and |λ_i|² = 1 exactly.
- D: the simulator replays each W_i from σ, every letter applied and not uncertain, and K ends at exactly
  L(λ_i) Q*.

### 2.5 Perturbation lemma (proved)
Let W be a word with partial products P_k (P_0 = I), Q a pose of K, and η in (0, 1). Assume (*) holds for
m = n·(P_k Q v) − κ, for every k, every vertex v of K and both caps n. Then for every unit quaternion u with
|u − 1| ≤ η, and every k, K at pose P_k L(u) Q lies whole in caps c and d, and all its margins are > 0.

*Proof.* Fix k, v and n, and put w = Q v and n' = P_kᵀ n. Then n·(P_k L(u) Q v) = ⟨u w, n'⟩ = ⟨u, n' w̄⟩, by
⟨xy, z⟩ = ⟨x, z ȳ⟩ for quaternions. Let G = n' w̄. Then |G| = |n'| |w| = |n| |v|, because P_k and Q are
orthogonal. Also G_0 = ⟨1, n' w̄⟩ = ⟨w, n'⟩ = n·(P_k Q v) = m + κ. Here m > 0: the right side of (*) is ≥ 0,
so m > (m + κ)η²/2, which gives m(1 − η²/2) > κη²/2 > 0 (κ > 0, η < 1). Hence G_0 > 0. The square root is
real because m + κ = n·(P_k Q v) ≤ |n||v|. From |u − 1| ≤ η:
|u − 1|² = 2 − 2u_0, so u_0 ≥ 1 − η²/2 ≥ 0, and |u_vec|² = 1 − u_0² ≤ η². Hence
⟨u, G⟩ = u_0 G_0 + u_vec·G_vec ≥ (1 − η²/2)(m + κ) − η sqrt(|n|²|v|² − (m + κ)²). So
n·(P_k L(u) Q v) − κ ≥ m − (m + κ)η²/2 − η sqrt(|n|²|v|² − (m + κ)²) > 0. ∎

### 2.6 Covering lemma (proved from computation C)
For every unit quaternion u with |u − 1| ≤ η, some listed box b contains the vector part of u, and the word i of
any such box satisfies |λ_i u − 1| ≤ η.

*Proof.* u_0 ≥ 1 − η²/2 > 0, and |u_vec|² = 1 − u_0² ≤ R² := η² − η⁴/4 ≤ η². So u_vec lies in
[−η, η]³ ∩ {|x|² ≤ R²}. Computation C (`CERT.py`) subdivides [−η, η]³ into closed dyadic boxes, discarding a box
only when the minimum of |x|² on it exceeds R². It checks that every remaining leaf is a listed box (113 boxes,
16 split boxes), so the set is covered. For a listed box b with word i, C checks exactly that λ_{i,0} > 0 and
λ_{i,0} s_b − max_{x∈b} λ_{i,vec}·x ≥ 1 − η²/2, where s_b is a rational lower bound of
sqrt(1 − min(r2max(b), R²)). For u_vec in b: u_0 = sqrt(1 − |u_vec|²) ≥ s_b, so
Re(λ_i u) = λ_{i,0} u_0 − λ_{i,vec}·u_vec ≥ 1 − η²/2, i.e. |λ_i u − 1|² = 2 − 2 Re(λ_i u) ≤ η². ∎

### 2.7 Steps of the proof and refutation attempts
| step | justification | refutation attempt |
|---|---|---|
| X, (I1), (I2), Good sets | computed: `CERT.py` A1, `PUMP.py` P1–P3 | exact Evaluator for every piece and item; NB(11) check; both programs agree |
| 'whole in cap' = all 16 vertex margins ≥ 0 | simulator semantics (`sim/kernel.py` Evaluator, `classify_signs`) | `CERT.py` asserts the simulator rows `RG.vform(p)` are the points `RG.vertices(p)` for each piece of K; A1b recomputes the Good sets and (i) with the simulator's own `classify` (signature certificates at lattice poses): same result |
| admissible ⟺ menu item and no straddler | `sim/state.py` apply; packet (NB blocking is a consequence) | the 13,177 letters of F_40, all 3156 of H6030 and 3000 pump letters of `PUMP.py` replay with the simulator as the lemma predicts |
| isolation, tree, step lemmas | proved (2.2, 2.3) | `PUMP.py` checks the predicted pose of every piece every 100 letters and the +2 height at every letter |
| σ of kind S(3888, c) | computed A2 (exact margins), and `PUMP.py` P4 (simulator) | the two programs check independently |
| (*) and P_i = L(λ_i) | computed B; D replays every word with the simulator | margins are exact Q(√5) numbers; (*) squared with the sign of the left side checked first; a referee recomputed (*) with a differently arranged exact formula (56,096 checks) and replayed every word exactly from L(u) Q* with \|u − 1\| ≈ η |
| perturbation lemma | proved (2.5) | needs only that L(u) is left quaternion multiplication, P_k orthogonal, and G_0 > 0 |
| covering lemma | proved from computed C | boxes are closed; s_b is a lower bound (integer sqrt); λ_{i,0} > 0 and \|λ_i\| = 1 checked; duplicate boxes rejected; a referee removed one box, changed one word, or doubled η, and each time `CERT.py` failed |
| induction Z(t) | proved (section 1) | F_40 replayed with K at L(u_t) Q* at every boundary; float run of the rule for 100,000 words stays at \|u − 1\| ≤ 0.0039 (not used in the proof) |

## 3. Computations (all exact; run from the repository root with `S=research/jumbling/restoration/10-x1-unbounded`)
| program | command | output | time |
|---|---|---|---|
| ball certificate | `python3 -I -B $S/programs/CERT.py` | `programs/CERT.out` (PASS) | 161 s |
| family members 0..40 | `python3 -I -B $S/programs/FAMILY.py 40` | `programs/FAMILY_40.out` (PASS) | 836 s |
| family members 0..3 | `python3 -I -B $S/programs/FAMILY.py 3` | `programs/FAMILY_3.out` (PASS) | 80 s |
| F_3 by the generic checker | `python3 -I -B $S/programs/replay.py $S/witnesses/F3.json 2890` | `programs/F3.out` (PASS) | about 80 s |
| pump, 3000 letters | `python3 -I -B $S/programs/PUMP.py $S/witnesses/H6030.json 156` | `programs/PUMP_H6030.out` (PASS) | 224 s |
| pump, 600 letters | `python3 -I -B $S/programs/PUMP.py $S/witnesses/H1230.json 156` | `programs/PUMP.out` (PASS) | about 50 s |
| H6030 | `python3 -I -B $S/programs/replay.py $S/witnesses/H6030.json 3150` | `programs/H6030.out` (PASS) | see file |
| earlier records | `replay.py` on H22, H34, H38, H52, H122, H1230; `H20.py` | `programs/*.out` (PASS) | see files |

## 4. How the certificate was found (not part of the proof)
The pump of 2.3 alternates jumbles at caps 11 and 80, which lie 36° apart and both hold K. Its state is K's pose
(a left factor on S³ and a right factor in 2I) plus finite labels. Along a 3000-letter pump (H6030) there are
near-periodic stretches: from pump state 1929 on, a 22-letter pattern repeats, and its cube returns every label
while moving K's pose by a left multiplication of size 0.00116. Searches from that state (exhaustive forward
search with a lens-margin threshold, and meet-in-the-middle between forward and backward trees,
`work/cert5.py`, `work/mitm2.py`) found about 200 label-returning words whose displacements λ are within 0.006
of 1 and point in all directions. A greedy cover (`work/cert7.py`) picked 7 of them that cover the ball of radius
η = 1/200 with 113 boxes; `work/cert_export.py` wrote the certificate. All of this used floating point; the claims
rest on `CERT.py` and `FAMILY.py`. (The search programs in `work/` are not in this repository; they stay
in the run's output archive.)

## 5. Earlier results (computed; all replayed by `programs/replay.py`)
- **R1.** Left height 6030 is reachable with the 3156-letter word `witnesses/H6030.json`: the first 156
  letters of H38, then 3000 pump letters (`H6030.out`, `PUMP_H6030.out`). H1230 (756 letters) is the same
  construction with 600 pump letters (`H1230.out`, `PUMP.out`). H52 (169 letters) and H122 (206) are its first
  members.
- **Records before the pump.** All begin with the first 141 letters of the word of E17-4.
  - H20 (150 letters): tail (592, 22), (593, 30), (582, 30), (576, 51), (525, 21), (410, 15), (393, 20),
    (410, 14), (361, 12); `programs/H20.py`.
  - H22 (152 letters), H34 (160) and H38 (160) are in `witnesses/`, with outputs `programs/H22.out`,
    `H34.out`, `H38.out`.
  - H38's tail is (410, 15), (393, 20), (410, 14), (389, 36), (165, 2), (238, 50), (164, 57), (80, 32),
    (123, 51), (11, 26), (80, 12), (33, 43), (140, 54), (175, 12), with heights 14 to 38. Along it the out
    grips grow from 14 to 22, since every raise but two is a fresh departure.
  - What repeats in letters 147–160 is a walk of K across caps 36° apart. At letter 156 two out grips at 36°
    (11 and 80) hold K, and from there the pump replaces the walk.

## 6. What stays open, and the next step
X1 is settled. Open side questions:
- the growth rate of the number of reachable configurations by height;
- whether the same two-cap mechanism works for other menus or other pieces;
- a description of the closure of the set of K's poses reachable by the pump (the float runs suggest the
  survivor set has positive measure).

Two independent referees (subagents with fresh context) checked the result before `DONE`.
- One checked the mathematics of sections 1 and 2 against the simulator's code. It found no fatal issue. Its
  one fixable point was that the step lemma must use only the description of the state; that has been fixed. It
  also verified the premises of the tree lemma exactly for all 120 letters at grips 11 and 80.
- The other audited `CERT.py` and `FAMILY.py` and found no way for them to pass falsely. Its fixable points
  (duplicate boxes, |λ_i| = 1, orthogonality of the letters, the simulator pose at σ, asserted centre claims)
  are now checked in `CERT.py`.

The most useful next step is the integrator's own line-by-line check of sections 1 and 2 and a re-run of
`CERT.py` and `FAMILY.py`.
