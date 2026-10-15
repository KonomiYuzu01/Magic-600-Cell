# Three-block finishing family: iterated conjugation read from the current state

Status: 12 October 2026. Labels: **Proved** (by the integrator; no independent review of this note), **Computed** (exact, by the script named, headless Linux), **Open**. A single instance, a sample or this family does not make general E complete. Nothing here is a Windows, DirectX or performance result.

Scope (owner's assignment): the last three off pieces B′ = {16245, 16249, 32088} and the geometric mechanism of the iterated conjugation W = P_m F P_m⁻¹. The lattice phase and the old X → E3 instance are out of scope; the E2 certificate below only checks that the template reproduces the old word.

## 1. Setting

- Model facts (Lemma H′, segment 2, (M1) to (M4)): a piece's region is its exact pose applied to its home chamber; it is on K exactly when its pose is in K+ (order 7,200), and then its region is a chamber; chambers never straddle a cut; every menu element fixes its grip's pole, so a twist maps the closed half-space of its cap onto itself; a twist is admissible when no piece straddles its cut. Hence piece regions keep disjoint interiors at every reachable state.
- Proposition S (earlier packages): a word is admissible at a state exactly when no content present straddles a cut along it; contents move independently.
- Word-only chamber ghost of a word w (every chamber followed as a lattice content): KV(w) (straddles at some twist), Off(w) (ends off K), Good(w) (ends on K, at π_w(z)), Bad(w) = KV(w) ∪ Off(w). The frame ghost of B′ follows its three exact contents. Only the chambers of the template's caps can move; the others are fixed.
- F_h: the states whose off set is exactly B′, rigid at E2's exact pose h ∉ K+, with every other piece on K and any hole set. A(h), G(h): the SAFE-ALL alphabet of that off geometry and its group (592 grips, 6,512 letters, 686 orbits, 651 fixed chambers). They are the same for every state of F_h.

**Lemma T (type invariant; Proved).** Call the K+ orbit of a chamber its type (chamber s has the type of piece s). At every state, for every type t, the number of holes of type t equals the number of off pieces whose home chamber has type t.
Proof. A piece on K at pose k is in chamber k(home), of its own type, and no two pieces share a chamber (disjoint interiors). There are as many pieces of type t as chambers of type t. ∎
Computed (`types3.py`, `cert/types-E2.json`): holds at X, E1, E2, E3. In F_h the off homes have types {14, 15, 15}, so the holes are one chamber of type 14 and two of type 15. Under G(h), type 15 is one orbit (7,200 chambers, Jordan certificate p = 3,863); type 14 is orbit 14 (3,599 chambers, p = 2,027) plus the fixed chamber 84071.

## 2. Template and layer calculus

Template σ = (R; x_0, ..., x_{m−1}; F). P_0 = R, P_{k+1} = P_k x_k P_k⁻¹, W = P_m F P_m⁻¹. At level k: π_k = π_{P_k}; T_k = Bad(P_k⁻¹) (the bubble); L_k = the chambers of B′ after P_k; parked contents = the end regions of the chambers of Off(P_k) after P_k.

Conditions (each a finite exact test on the word, B′'s exact contents and chamber ghosts; none reads the occupancy):
- R0: R moves all of B′, and B′ lands on K (r h ∈ K+).
- S1: x_k ∈ K+ ∖ {1}. S2: no parked content lies in cap(x_k) or straddles its cut. S3: L_k ⊆ cap(x_k) and x_k(L_k) ∩ T_k = ∅.
- F1: F ∈ K+ ∖ {1}. F2: as S2. F3: F(L_m) ∩ T_m = ∅. F4: F(T_m ∖ L_m) ⊆ T_m.

**Lemma 0 (Proved).** Good(w⁻¹) = π_w(Good(w)) and π_{w⁻¹} = π_w⁻¹ there; |Bad(w⁻¹)| = |Bad(w)|.
Proof. A letter acts on a region according to the region alone, and the inverse letter has the same cap and the inverse element. The trajectory of a good chamber under w, read backwards, is the trajectory of its end chamber under w⁻¹, and conversely. ∎

**Theorem L (layer calculus; Proved).** Assume R0 and S1 to S3 at levels 0, ..., k−1. Then:
- (K0) KV(P_k) = KV(P_k⁻¹) = ∅, and B′ stays clean through P_k and ends on K at L_k ⊆ T_k.
- If S1 to S3 also hold at level k, with x the chamber map of x_k and π = π_k:
  - (L1) Off(P_{k+1}) = π⁻¹(x⁻¹(T_k) ∖ T_k);
  - (L2) T_{k+1} = π⁻¹(x(T_k) ∖ T_k), so |T_{k+1}| = |x(T_k) ∖ T_k| = 3 + β_k with β_k ≥ 0;
  - (L3) π_{k+1} is the identity on Off(P_k) and π⁻¹ x π on the rest of Good(P_{k+1});
  - (L4) L_{k+1} = π⁻¹(x(L_k)).
- Every P_k is admissible at every state of F_h.

Proof. Level 0: R is one letter and chambers never straddle, so KV(R) = KV(R⁻¹) = ∅. R moves B′ from h ∉ K+ onto K, so its element is not in K+ and every chamber of cap(R) goes off: T_0 = Off(R) = cap(R) ⊇ L_0 (the cap is invariant).
Step k → k+1. Follow a chamber z through P_k x_k P_k⁻¹.
- z ∈ Off(P_k): its content is parked after P_k. By S2 x_k does not touch it, and P_k⁻¹ retraces its path back to z. So z is good and π_{k+1}(z) = z.
- z ∈ Good(P_k): u = π(z) lies outside T_k (Lemma 0). x_k ∈ K+ moves the lattice content of u to the chamber x(u) and never cuts it.
  - If x(u) ∉ T_k, P_k⁻¹ takes it to π⁻¹(x(u)).
  - If x(u) ∈ T_k = Off(P_k⁻¹), it ends off.
This gives KV(P_{k+1}) = ∅, (L1) and (L3). The same argument for P_{k+1}⁻¹ = P_k x_k⁻¹ P_k⁻¹ gives KV(P_{k+1}⁻¹) = ∅ and (L2); S1 and S2 hold for x_k⁻¹ (same cap, element in K+).
B′ is at L_k after P_k. By S3, x_k puts it on the chambers x(L_k), outside T_k, and P_k⁻¹ takes these lattice contents to π⁻¹(x(L_k)) on K: (L4). Since L_k ⊆ T_k, x(L_k) ⊆ x(T_k) ∖ T_k, so L_{k+1} ⊆ T_{k+1} and β_k ≥ 0.
Admissibility: at a state of F_h the contents are lattice contents of chambers, clean since KV(P_k) = ∅, and B′, which is clean. Apply Proposition S. ∎

**Lemma D (true domain of the inverse segment; Proved).** Let KV(P) = KV(P⁻¹) = ∅. Let Y′ be a state each of whose off contents is a parked content of P at exactly the region where P leaves it. Then P⁻¹ is admissible at Y′, and:
- each parked content returns to its chamber;
- the lattice content of a chamber u ends on K at π_P⁻¹(u) when u ∉ Bad(P⁻¹), and off K when u ∈ Bad(P⁻¹).
Proof. Parked contents retrace their paths. Lattice contents are clean because KV(P⁻¹) = ∅. Apply Proposition S. ∎
Limits:
- The lemma needs the exact parked regions. A parked content moved by any twist, or a different piece on a clean single-piece ghost, is outside it.
- A single-piece ghost does not give the legality of the common word. Here legality comes from KV over every chamber of the template's caps (11,613 of them) together with the exact frame ghost of B′.

**Theorem F (finish; Proved).** Assume (K0) at level m and F1 to F4. Let D = π_m⁻¹(F⁻¹(T_m) ∖ T_m). Then:
- KV(W) = ∅, Bad(W) = Off(W) = D and |D| = 3.
- B′ stays clean and lands on K at π_m⁻¹(F(L_m)). The landing set is D when F is an involution on chambers.
- W is admissible at every state of F_h. At every Y ∈ F_h whose holes are D, WY has every piece on K, and every lattice piece of Y stays on K.

Proof. Follow a chamber z through P F P⁻¹.
- z ∈ Off(P): parked, untouched by F (F2), returned by P⁻¹ (Lemma D, since the state after F has exactly P's parked contents off). So z is good with π_W(z) = z.
- z ∈ Good(P): u = π(z) ∉ T. F ∈ K+ moves the content to F(u). P⁻¹ then keeps it on K if F(u) ∉ T and takes it off if F(u) ∈ T.
So KV(W) = ∅ and Off(W) = π⁻¹({u ∉ T : F(u) ∈ T}) = D.
Count: F(T) is the disjoint union of F(L) and F(T ∖ L), with F(T ∖ L) ⊆ T (F4) and F(L) ∩ T = ∅ (F3). So |F(T) ∩ T| = |T| − 3. Then |F⁻¹(T) ∖ T| = |T| − |T ∩ F⁻¹(T)| = |T| − |F(T) ∩ T| = 3.
B′: F3 forces L ⊆ cap(F), since a block chamber outside the cap would stay in T. So F puts B′ on F(L), outside T, and P⁻¹ takes it to π⁻¹(F(L)). If F² = 1 on chambers, F⁻¹(T) ∖ T = F(T) ∖ T = F(L).
The last claims follow from Proposition S: every occupied chamber of Y lies in Ω ∖ D = Good(W). ∎

What the counts do not give: |T_k| falling (3,097, 134, 80, 14, 10 for σ*) is not set descent. T_{k+1} ⊄ T_k at every level of σ* (computed). Termination comes from the explicit finite template, checked level by level, not from a potential.

## 3. The certificate V(X, σ)

V(X, σ) reads only the pose table X, the fixed model and σ (`constr3.py`, library `layers3.py`). It never reads a generation word or history, and it is evaluated before any new word is applied. It holds when:
- (i) X's off set is one rigid block of three pieces (same exact pose).
- (ii) Template, at X's off geometry. R0; for every level, (K0) computed directly from the ghosts of P_k and P_k⁻¹: KV empty, B′ alive and on K at L_k ⊆ T_k, π injective, and Lemma 0 consistency. Also S1 to S3; the update formulas (L1) to (L4) compared with the direct ghosts of the next level; F1 to F4; and W's predicted KV, Bad = D (|D| = 3) and landing compared with W's direct ghost.
- (iii) Vacancy, at X:
  - A(Ξ(X)), its orbits and fixed chambers;
  - N1: the fixed chambers of D are holes;
  - N2: every orbit has at least as many holes as targets;
  - Jordan certificates (Theorem A) for the orbits meeting D ∖ Fix, each rechecked;
  - the peeling certificate (Theorem B) and its greedy clearing word V, read from X's holes, whose predicted holes must equal D.

Output: U = V · W · V⁻¹, written in five parts (V, P, F, P⁻¹, V⁻¹), with the claims 3 restored, 0 collateral, 0 off at the end and no remaining off pose changed. Nothing is applied.

**Theorem V (soundness; Proved).** If V(X, σ) holds, U is admissible at every prefix at X, and UX has all 177,120 pieces on K: B′ restored, and every lattice piece of X still on K.
Proof.
- V is a SAFE-ALL word (Lemma 41, Theorem B): it keeps B′'s exact pose and every lattice flag and moves the holes onto D.
- W then restores B′ (Theorem F, whose hypotheses are part of (ii)).
- At an all-on-K state every twist is admissible: no off piece, and chambers never straddle. The letters of V⁻¹ are in K+, so every piece stays on K. ∎

Condition (ii) depends only on B′'s exact pose and contents, never on the occupancy. A template certificate computed at one state of F_h therefore holds at every state of F_h; the same is true of A(h), G(h) and its Jordan and peeling certificates.

Exact failure branches (each refutes this template with a SAFE-ALL preparation, not every construction):

| Branch | Meaning |
| --- | --- |
| input | the off set is not one rigid three-piece block: outside the family |
| template | a failed condition (R0, K0, S1–S3, update formula, F1–F4, prediction) names the level and the letter; it depends only on B′'s exact pose |
| N1 | a fixed chamber of D is occupied: no SAFE-ALL word of any length vacates D (Proposition N) |
| N2 | some orbit has fewer holes than targets: no SAFE-ALL word of any length vacates D (Proposition N); the certificate lists the deficits, the holes in fixed (singleton) orbits, and for each such hole the lock-lemma premises I_B ⊆ caps(x) ⊆ I_B ∪ S_B |
| peeling | N1, N2 hold but no complete peeling certificate: with Jordan certificates a clearing word still exists (Corollary A1) but is not issued (not met here) |

## 4. The fixed-pose family at h

σ* = (R = (237, 13); x = (91, 6), (154, 7), (133, 1), (183, 6); F = (162, 1)). It was fixed before any new input was generated (`inputs/sigma-star.json`).

Computed (`layers3.py`, `cert/sigma-star-E2.json`):
- All conditions hold, and every update formula agrees with the direct ghosts.
- Universe: 11,613 chambers (the six caps).

| k | \|P_k\| | \|T_k\| = \|Off(P_k)\| | β_{k−1} | L_k | B′ K+ index | x_k | out of T / into T | T_k ⊆ T_{k−1} |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 0 | 1 | 3,097 | – | 42275, 58266, 42278 | 5532 | (91, 6) | 134 / 134 | – |
| 1 | 3 | 134 | 131 | 48269, 48270, 58569 | 2257 | (154, 7) | 80 / 80 | no |
| 2 | 7 | 80 | 77 | 55583, 58275, 55584 | 1726 | (133, 1) | 14 / 14 | no |
| 3 | 15 | 14 | 11 | 47688, 47699, 57090 | 1889 | (183, 6) | 10 / 10 | no |
| 4 | 31 | 10 | 7 | 57075, 89027, 57076 | 5962 | F = (162, 1) | – | no |

- Finish: KV(W*) = ∅ and Bad(W*) = D = {23354, 23362, 38667} (types 14, 15, 15; orbits 14, 15, 15).
- B′ lands on D at K+ index 21: rigid, pose matching chambers. F is an involution.
- |W*| = 63.

**Theorem G (generic branch; Proved from the computed certificates).** Let Y be a state of F_h whose type-14 hole lies in orbit 14, that is, is not 84071. Then V(Y, σ*) holds, and the constructor's word U(Y) = V(Y) · W* · V(Y)⁻¹ restores Y with |V(Y)| ≤ 18 and |U(Y)| ≤ 99.
Proof.
- (ii) holds at E2 ∈ F_h and so on all of F_h (section 3).
- D ∩ Fix(G(h)) = ∅, so N1 holds.
- By Lemma T, Y has one hole of type 14, in orbit 14 by assumption, and two in type 15, which is one orbit. So N2 holds with equality.
- Orbits 14 and 15 have Jordan certificates.
- The peeling certificate peels 3 of 3 with thresholds equal to the targets (1 and 2) and bound 18. So the greedy procedure, reading only Y's holes, clears D in at most 18 letters (Theorem B).
- Apply Theorem V. ∎

Coverage: every hole set with the type-14 hole in orbit 14 (3,599 × C(7,200, 2) = 93,273,123,600 hole sets), and every arrangement of the lattice pieces. Reachability of these states is not claimed. Two reachable ones are tested in section 6.

### 4.1 The frozen branch: the type-14 hole at 84071

**Computed facts at h** (`lock3.py`, `frozen3.py`, `blockmap3.py`, `layers3.py --search`):
- 84071 is the only fixed chamber of type 14.
- It is locked to B′: I_B = {232, 237, 257, 273, 301, 356}, S_B = {250, 307}, caps(84071) = I_B ∪ S_B. The lock lemma applies, so under every word the content of 84071, or its hole, stays at k_B(84071) until B′'s first move by an element outside K (Corollary L).
- At E2 the locked chambers are 84069, 84070 and 84071 (types 8, 10, 14), all occupied.

**Proposition O (σ*-level obstruction; Computed).**
- At level 0, 84071 is parked (Corollary L for R).
- At levels 1 to 4, π_k(84071) = 84071 ∉ T_k. Each of the eight grips whose cap holds 84071 has a parked content inside its cap or cut by it; at levels 3 and 4, two of them also hold L_k.
- So no finishing letter at any level of σ* has 84071 ∈ D: F would have to hold 84071 in its cap, which violates F2.
- The search agrees: levels 0 to 3 have no finishing letter; level 4 has 55, 15 of them with landing equal to D, and none with 84071 ∈ D. No pure letter (Lemma Z) moves 84071's image at any level.
- This excludes the σ* prefix with any finishing letter. It says nothing about other prefixes.

**Exact failure certificate on a frozen table.**
- Input S3 (`synth3.py`, reachability **UNPROVED**): E2 with lattice piece 47855 moved from 84071 to the hole 89251, at the least K+ pose that puts it there.
- The constructor reports "failure: vacancy":
  - the template conditions hold;
  - N1 holds, and N2 fails: orbit 14 needs 1 hole and has 0;
  - the type-14 hole 84071 is in a singleton orbit;
  - the lock premises hold.
- By Proposition N no SAFE-ALL preparation of any length exists for σ*'s D. No word is issued.

**Theorem Q (frozen-branch duality; Proved).** Let Φ_h be the frozen states of F_h. The following are equivalent:
- (a) some state of Φ_h is reachable from a state with every piece on K;
- (b) some state of Φ_h has a restoring word, that is, an admissible word ending with every piece on K;
- (c) there is a word w that, at the off geometry h, keeps B′ clean, lands it on K, and has Bad(w) = {84071, y_1, y_2} with y_1, y_2 of type 15;
- (d) every state Y of Φ_h is restored by V_Y · w · V_Y⁻¹, for any w as in (c) and a SAFE-ALL word V_Y read from Y's holes;
- (e) every state of Φ_h is reachable from a state with every piece on K.
Every restoring word w of a frozen state Y has Bad(w) = V(Y), and so contains 84071.
Proof.
- (a) ⇔ (b) and (d) ⇒ (e): twists are reversible, so a word admissible at Z has its inverse admissible at the end state.
- (b) ⇒ (c): by Lemma H′, a clean word that restores every off piece of Y has Bad(w) = V(Y) = {84071, a, b}, with a and b of type 15 by Lemma T.
- (c) ⇒ (d):
  - Type 15 is one orbit with a Jordan certificate, and 84071 ∈ Fix(G(h)) is a hole. So by Corollary A1 a SAFE-ALL word V_Y moves Y's two type-15 holes onto {y_1, y_2}, keeping B′ at h and the hole at 84071.
  - At V_Y Y, KV(w) ⊆ Bad(w) is exactly the hole set. So every content is clean and w is admissible (Proposition S); every occupied chamber is good and B′ lands on K.
  - V_Y⁻¹ then acts on an all-on-K state. ∎

Consequence: exactly one of these holds.
- (I) No frozen state is reachable from any all-on-K state, the solved state included. Then every reachable state of F_h is generic and Theorem G restores it.
- (II) Every frozen state is reachable, and one word w, found once, restores all of them through V_Y w V_Y⁻¹.
Either way, history-free restoration of all of F_h reduces to deciding (c) once.

**Open obligation O1.** Decide whether a word w as in (c) exists. A finite search that finds none would not settle it. One reachable frozen state with its generating word would give one: w = its inverse. As a constructor template it must then be fixed before the test inputs, and the constructor still reads only the current table.

### 4.2 Transport to the K+ orbit of h

**Corollary C (Proved from the computed premises).** Model premises:
- (P1) K+ is transitive on the 600 grips: `stab3.py`.
- (P2) Every menu element fixes its pole: 36,000 of 36,000, `cap3.py`.
- (P3) k(cap(0)) = cap(k·0) for all 7,200 k (`cap3.py`). With (P1) this extends to every grip: cap(k j·0) = k j(cap(0)).
- (P4) The K+ part of every menu has 12 elements, which is the grip's stabilizer (`stab3.py`). So conjugation maps SAFE-ALL letters to SAFE-ALL letters.
- (P5) For every k ∈ K+ and every letter (c, E) of W* and W*⁻¹, k E k⁻¹ is in the menu of k·c: 72,000 of 72,000, `equiv3.py`.

For k ∈ K+:
- let kY multiply every pose by k on the left;
- let w^k replace each letter (c, E) by (k·c, k E k⁻¹).
Then w^k is admissible at kY exactly when w is admissible at Y, and (kY)·w^k = k(Y·w). Regions inside, straddling or outside cap(c) map to regions inside, straddling or outside cap(k·c), and (k E k⁻¹)(k g) = k(E g). So Theorems G and Q hold for F_{kh} = k F_h, the frozen chamber k(84071) and the template (W*)^k, with every orbit and certificate transported by k and |U| ≤ 99.
This transports the whole family, the complete pose kh and the common domain F_{kh}; it is not a copy of one solved instance.

## 5. Length and cost

Length:
- |P_k| = 2^{k+1} − 1, so |W| = 2^{m+2} − 1 and |U| = 2|V| + 2^{m+2} − 1.
- For σ*: |W*| = 63, |U| ≤ 99 on the generic branch. Observed: 79 at E2 (the old word), 89 at I1, 71 at I2.

Checking cost (single runs, this machine):
- (ii) costs Σ_{k≤m} 2(2^{k+1} − 1) + 2^{m+2} − 1 = 177 letter applications to the chamber ghosts over the universe (11,613 chambers here), plus the three-piece frame ghost and one chamber map per letter: about 10 s.
- (iii) costs one SAFE-ALL alphabet (6,512 letters), its orbits, two 120-letter Jordan rechecks, three peeling rows and the greedy word: 20 to 37 s.
- The search mode (`--search`) enumerates, per level, the K+ letters of the grips whose cap holds L_k: under 2 s per level.
- Replay with a depth test at every prefix: about 95 s for 71 to 89 twists.

Bounded-level statement (Theorems L and F): for any template whose conditions are verified through m levels, the conclusion holds with |W| = 2^{m+2} − 1. No template is known to exist for every B′ pose; that is a coverage obligation (section 7).

## 6. New inputs, frozen before any construction

Order: σ* was fixed first, then the inputs were generated (`gen3.py`), then certified (`constr3.py`), then the new words were replayed. Each generation record lies apart from its table; the constructor received only the table path.

| Input | Source (record, separate) | Digest | Holes (types) | Reachability |
| --- | --- | --- | --- | --- |
| I1 | E2 + 60 seeded SAFE-ALL letters (seed 20261012) | f27c345e591eecfa… | 24971 (15), 52576 (14), 65558 (15) | replayed from E2: 60 twists, depth at every prefix, digests as recorded |
| I2 | E2 + V(8) · W*(63) · κ(40 K+ letters at the all-on-K state, caps away from B′'s landing) · W*⁻¹(63) · r(120 SAFE-ALL, seed 11) | 989fd04cb2a409b6… | 70074 (14), 70085 (15), 73877 (15) | replayed from E2: 294 twists, depth at every prefix, digests as recorded; it passes through an all-on-K state |
| S3 | E2 with piece 47855 moved from 84071 to 89251 (synthetic) | dd6ea43082efb134… | 24905 (15), 84071 (14), 89252 (15) | **UNPROVED** (equivalent to O1 by Theorem Q) |

## 7. Results

Certificates (`runs/<input>/construct/certificate.json`):

| Input | Result | \|V\| | \|U\| | V steps |
| --- | --- | --- | --- | --- |
| I1 | certificate issued | 13 | 89 | 3 fills (routes 4, 3, 3) |
| I2 | certificate issued | 4 | 71 | 1 fill (route 3) |
| S3 | failure: vacancy (N2, orbit 14: 0 of 1; hole in fixed orbit {84071}; locked) | – | – | – |

New words (`replay_check.py`, every twist checked admissible, depth at every prefix; `compare3.py` recomputes both digests over all 177,120 exact matrices):

| | I1 | I2 |
| --- | --- | --- |
| twists / depth tested / depth failures | 89 / 89 / 0 | 71 / 71 / 0 |
| off: start → max → end | 3 → 3,097 → 0 | 3 → 3,097 → 0 |
| restored / collateral / off at end | 3 / 0 / 0 | 3 / 0 / 0 |
| lattice pieces of the start still on K at the end | 177,117 of 177,117 | 177,117 of 177,117 |
| exact pose unchanged / changed | 174,026 / 3,091 | 174,026 / 3,091 |
| of the changed: chamber changed / same chamber, other pose | 3,086 / 5 | 3,086 / 5 |
| B′ at the end: chambers (K+ index) | 52576, 24971, 65558 (1323, 4900, 3793) | 70074, 73877, 70085 (3904 each) |
| start digest | f27c345e591eecfa9d6807fbdbeb79326eb7b3987641524bc11e84a71fb44269 | 989fd04cb2a409b6679b2106a18295a36161f9e76d804612875798f55131dd9a |
| end digest | d4c74f8e1d4d2b056e6e4c1e9e6b796b93aa872f76c92956f98eccb068e251e4 | 67bafbc07bece94c3909c8996f6ac76494ea7258cd46c668d95c1ab643d1a1a8 |

Notes:
- In both words, B′ ends in the input's own holes, as U = V W V⁻¹ with an involutive F predicts.
- The pose-change count 3,091 is the same for both inputs and for the old E2 → E3 word: conjugation by V moves the support of W* but keeps its size.
- Lattice membership and exact poses are listed separately: every lattice piece stays on K, and 3,091 change their exact pose.

## 8. Limits and open obligations

- O1 (section 4.1): existence of one frozen-branch word, equivalently the reachability of one frozen state.
- Coverage: only the families F_{kh}, k ∈ K+ (B′ rigid at a pose in K+h). Not covered:
  - other poses of B′;
  - other three-piece blocks;
  - three off pieces that are not one rigid block;
  - more than three off pieces.
  For a new B′ pose, a template must be found and certified first. Its existence is not proved.
- Only σ* and its finishing-letter variants were analysed. Proposition O excludes only that prefix for the frozen branch.
- The proofs are the integrator's. The owner asked for no new verifiers, and this note has had no independent review. Lemma H′, Proposition N, Theorems A and B, Corollary A1, the lock lemma and Corollary L are cited from earlier packages with their review status as recorded there.
- All runs are headless Linux runs of the exact simulator. Not run: Windows, DirectX, the native host, and any performance measurement. Timings are single runs.
- General E is not claimed.
