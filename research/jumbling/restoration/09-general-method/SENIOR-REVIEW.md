# Senior review of the general method (15 October 2026)

On 15 October 2026 the owner asked for continuous theoretical work by the project's senior reviewer, a Codex model called through the project's review wrapper with read-only access to the repository. This page records the results that the integrator adopted. The private call records are not in the repository.

Labels on this page:
- **proved (model review)**: the reviewer wrote the proof; the integrator read it against the sources and adopted it. No human has reviewed it.
- **computed**: an exact computation; the command is given, and the integrator re-ran it where this page says so.
- **open**: not settled.

Headless research only: no Windows, DirectX or performance evidence.

## 1. M1: does every reachable state have a clean step? (round 1)

### Theorem M1-01 (existence; proved, model review)

For every reachable state Z with off pieces and every unit U of Z, there is a clean step for U with free set F = off(Z) \ U. An exhaustive search over menu words, from the state alone, finds one and stops.

*Proof.*
1. **Every admissible letter has an admissible inverse at its image.** The letter's rotation fixes the cap's normal, so regions inside the cap stay inside and regions outside stay outside; no region starts to straddle the cut. The inverse moves the same pieces back to their exact poses. So if a_1, ..., a_m is a history from the solved state to Z, then a_m^-1, ..., a_1^-1 legally returns Z to the solved state.
2. **The search.** Enumerate all words in order of length, and lexicographically within a length. Apply each word to Z, reject it at its first inadmissible letter, and accept it when the off set of the end state lies inside off(Z) \ U.
3. **Every test is exact and finite.** Admissibility compares linear functionals at the finitely many vertices of each region, with values in Q(√5). The sign of a + b√5 is decided exactly. Lattice membership is a test against the finite group K⁺.
4. **The search stops.** Each length has finitely many words, and the inverse history of part 1 is accepted. So the search stops at some finite trial, although the history is not known to it.
5. **An accepted word is a clean step.** Every letter was admissible, U ends on the lattice, every lattice piece of Z ends on the lattice, and no off piece outside U and F exists.

Repeating this reaches the lattice in at most |off(Z)| steps. **QED**

The theorem settles bare existence. It gives no bound on the length of the word or on the run time.

### Observation M1-02 (the driver as written; source level)

The driver v4 is not complete as written:
- its pure-twist stage takes at most 8 units (`--pure-max`);
- it stops after 80 steps (`--max-steps`);
- it has no exhaustive fallback.

A complete method needs the search of M1-01 as a fair fallback, or a coverage theorem for v4's own families. Section 3 of `METHOD.md` now says so.

### Proposition M1-03 (small states need excursions; proved, model review)

If 0 < h = |off(Z)| < 3,097, then:
- every clean step at Z contains at least two jumble letters;
- its first jumble letter takes at least 3,097 - h lattice pieces off.

*Proof.*
1. K⁺ letters keep each piece's lattice membership. So before the first jumble letter exactly the original h pieces are off, and there are h holes.
2. The cap of the first jumble letter is the union of 3,097 chambers. So it holds at least 3,097 - h lattice pieces.
3. A jumble element g is not in K⁺, so g·k is not in K⁺ for any k in K⁺. Every such lattice piece goes off.
4. A suffix of K⁺ letters cannot bring them back. So a clean step needs a second jumble letter. **QED**

At s61c (h = 61) every clean step passes through at least 3,036 off pieces.

## 2. M1: the last carrier at s61c, and bounds (round 2)

### M1-04: one class is not enough at s61c (proved, model review; computed)

At s61c the off set is one carrier, 23955: 61 pieces in one frame domain of class 0. Its class-0 cell is the edge cell X = {30, 145}. Of X's 86 home chambers, 68 hold lattice pieces and 18 are holes; the other 43 holes lie outside X. No class-0 grip is straddled. The reviewer reproduced the state's digest `218f823f…` by an in-memory replay.

- **Comotion.** Every word of class-0 letters that lands the carrier takes every lattice ghost of X off.
  - Under the class-cell model, a carrier piece with pose p outside K⁺ and a lattice ghost in X receive the same accumulated left multiplier a.
  - Landing gives a·p in K⁺. If the ghost also stayed on the lattice, a·k would be in K⁺ for some k in K⁺, so a and then p would be in K⁺: a contradiction.
  - So X ⊆ Bad(W), and |Bad(W)| >= 86 > 61.
- **Retained letters around the core do not help.** The same holds for every word V·W·T in which V and T are retained (K⁺ letters) and W uses the grips of one class, even when W is chosen after V.
  - A retained prefix moves the block rigidly by one g in K⁺, keeps it off and keeps the 61 holes.
  - g maps the class, its cuts and its lattice chambers to those of the class gP0. So the carrier lies in the cell gX, which has 86 lattice chambers, and by Card 5 Lemma 5.3 W must be a word of class gP0.
  - At least 25 of those 86 chambers are occupied, since only 61 holes exist. W takes their pieces off, and T cannot bring them back.
- **Consequence.** Between its first and its last jumble letter, a restoring word uses grips of at least two classes. This does not say that the other-class letter is a jumble letter, or that it acts while the original carrier is still off.

### M1-05: a 4,402-letter certificate at s61c (proved, model review; re-run by the integrator)

Let C be the s62m3 step (233 to 61 off pieces, 1,749 letters), S chain step 13 (233 to 86, 1,822 letters) and R chain step 14 (86 to 0, 1,374 letters). Then C⁻¹·S·R[0:831] restores s61c in 1,749 + 1,822 + 831 = 4,402 letters:
- C⁻¹ returns s61c to the 233 state, and S and R restore that state;
- R's letters after letter 831 are all K⁺ letters, which keep every piece's lattice membership, so the state is already restored after letter 831.

So a search of all words in order of length, from s61c alone, stops by length 4,402. The bound holds for this state only, and it comes from a known history, not from features of the state.

The integrator re-ran both commands from the repository root at commit `1a261bb` (2 min 43 s and 6 s). Step 13 ends at digest `090b175c…` with 86 off pieces, step 14 at `981c5bb2…` with 0 off, and R's last jumble letter is letter 831, followed by 543 K⁺ letters.

How the certificate works (the integrator's reading of the stored words):
- The 233 state has three carriers: 1040 (86 pieces, class 1, edge cell {5, 11}), 64184 (86 pieces, class 0, edge cell {369, 507}) and 23955 (61 pieces, class 0, edge cell {30, 145}).
- C⁻¹ re-creates the carriers 1040 and 64184 from lattice pieces. S lands 23955 together with 64184 by a carrier move (Lemma CM). R pure-twists 1040, which fills its edge cell.
- The jumble letters of C lie at the class-0 grip 448 and those of S at the class-0 grip 64, 16 each. Those of R lie at the class-1 grips 5 and 11. The core of C also uses K⁺ letters at the class-1 grip 427.
- Along the whole certificate every piece has left height at most 2 (computed with `trees.pose_tree` after every letter).

### M1-06: what a whole restoring word must do (proved, model review)

- **Jumble prefixes.** Lemma D does not carry over to a prefix V that contains jumble letters. For the conjugate T = V·W·V⁻¹,

  Bad(T) = D(V) ⊔ {x not in D(V) : W(V(l(x))) is undefined or lies outside A_V},

  where D(V) is the set of chambers whose ghosts die during V, l(x) is the lattice ghost of chamber x, and A_V is the set of points that V⁻¹ takes to lattice contents. So the new holes that V makes are not free slack.
- **Single-unit states.** Let the off set of Z be one unit U. Every restoring word T, not only a conjugate, has Bad(T) = H(Z), type by type: Lemma K gives |Bad(T)_t| >= |U_t|, and Fact 1′ gives |H(Z)_t| = |U_t|. Together with the survival and landing of U, this is also sufficient. At s61c the whole word must make exactly the 61 initial holes bad.

### M1-07: retained preparation has a finite bound (proved, model review)

Let the off set be one rigid unit U. For each type t let N_t be the number of chambers of type t and h_t = |U_t|.
- For a given middle word W of length L, it is decidable from the state alone whether some retained prefix V makes V·W a restoring word.
- If one exists, there is one with |V| <= Q − 1, where Q = 7,200 · ∏_t binom(N_t, h_t).
- The reason: under retained letters the block moves by one g in K⁺ and the holes keep their types, so the records (g, holes) form an exact finite quotient, and a shortest successful path repeats no record.
- For SAFE-ALL preparation, the smaller count Q_safe = ∏_Δ binom(n_Δ, h_Δ), over the SAFE-ALL orbits Δ that meet Bad(W), gives the same result.

These bounds are finite but astronomically large. They give decidability, not practical lengths.

### M1-08: local steps (proved, model review)

A unit U of a block B has a clean step that keeps every other off block at its exact pose exactly when a target set T_{B,U}(Z) meets R(I_a). That is the strong Block lemma, and it is open. M1-01 does not settle it, because its end point, the solved state, breaks the other blocks' exact poses. The method does not need local steps: its free set is every other off piece.

### M1-09: bounds and finiteness (proved, model review)

- **Height-bounded search.** A search that keeps the left height of every piece at most 2m is finite. It has at most S_m = (7,200·D_m)^177,120 states, with D_m = (4·16^m − 1)/3 domains, so a target reachable within the restriction has a word of length at most S_m − 1.
- **Starting heights are not enough.** A bound on the heights of the start state does not keep a restoring word inside such a restriction.
- **Uniform bounds.** A uniform bound on the restoration length, or on one clean step at every reachable state, exists if and only if R(I_a) is finite (Card 5 Q1.1, open).

The s61c certificate stays at left height 2, so for that state the search with m = 1 (S_1 = 151,200^177,120 states) also succeeds. That is a statement about one state.

### Where M1 stands after round 2

- Bare existence: proved (M1-01).
- A bound for one state, from a known certificate: s61c, 4,402 letters (M1-05).
- A bound from the features of the state, for every state: open. A uniform bound is equivalent to the finiteness of R(I_a) (M1-09).

## 3. M1: partners created from the state (round 3)

### M1-10: a saturated landing gives a creator (proved, model review)

Call a word W that lands an off set F onto chambers L *saturated* when |Bad(W)_t| = |F_t| for every type t. Then W⁻¹ kills no lattice ghost, Bad(W⁻¹) = L, and the off pieces W⁻¹ creates are exactly the contents F.
- Proof: Lemma K gives L ⊆ Bad(W⁻¹), and the two sets have the same size type by type, so they are equal. Every ghost outside L is kept, and every ghost in L returns to its off content.
- So W⁻¹ creates the carriers F from lattice pieces at a state Z, without any history, if two finite checks pass: every off piece of Z survives W⁻¹ at its exact pose, and no chamber of L is a hole at Z.
- A program can list candidate creators by enumerating short words and keeping the saturated landings (the certificate's middles have 46 letters). This gives candidates, not a proof that every state has one.

### M1-11: the three-phase template is decidable from the state (proved, model review)

The template: a retained preparation; a creator D (M1-10); a retained preparation; a middle W that lands the original carrier together with the first created carrier; a retained preparation; the pure twist of Theorem C on the second created carrier.
- Each retained phase is a search over records (g_1, ..., g_k, H): one K⁺ multiplier for each designated carrier, and the hole set. Lattice pieces follow their chamber ghosts, so a record determines every later test, and there are at most Q_k(a) = 7,200^k · ∏_t binom(N_t, a_t) records.
- So whether the template applies at a state is decidable from the state alone. When it applies, the whole word has at most Q_1(h) + Q_3(h + e1 + e2) + Q_1(e2) + |D| + |W| + 717 letters, where h, e1 and e2 are the type vectors of the carrier and the two created carriers.
- At s61c the certificate shows that the template applies: retained segments of 849, 1,744 and 1,429 letters, and cores of 46, 46 and 288.
- Open: whether every state with one deficient carrier passes some template. A rejection decides only the template tried.

### M1-12: what landing two carriers at once needs (proved, model review; computed, re-run by the integrator)

A carrier move P0 (Lemma CM), a SAFE retained word V and a middle W land two carriers A and B and fix every other piece when:
- every ghost of A and of the moved B survives and lands;
- every other off piece keeps its exact pose;
- Bad(W) lies in the holes of V(P0(Z)).

Then P0·V·W·V⁻¹ has |P0| + 2|V| + |W| letters. Lying in the same class or cell is not enough.

The stored steps instantiate it:
- C = 5 + 849 + 46 + 849 letters and S = 4 + 886 + 46 + 886. The jumble letters span 41 letters of each 46-letter middle.
- C's prefix turns carrier 1040 from class 1 into class 0 at its second letter.
- Each middle has 172 Bad chambers (twice the edge-cell type vector e) and no dead ghost. 104 (C) or 98 (S) of them are occupied before V, and none after.

### M1-13: s61c needs two partners (proved, model review; computed, re-run by the integrator)

- All 3,600 edge cells of the five classes have 86 chambers and the same type vector e over 21 chamber types.
- The s61c carrier's type vector u ≤ e falls short of e by 25 pieces in 14 types. Types 27 and 28 have no hole at all at s61c.
- One pure-twist partner (one lattice-only edge cell taken off) leaves fewer than 172 holes. Then no middle of one class lands both carriers.
- The 233 state has u + 2e holes, enough for the middle's 2e Bad chambers. Landing u + e leaves one full e-carrier for Theorem C.

### M1-14: the creator needs a preparation (computed, re-run by the integrator)

At s61c, the 51-letter creator W_C⁻¹·P_C⁻¹ (the inverted middle and prefix of C) is admissible and keeps the carrier exactly. But six of its source chambers are holes, so it creates 86 + 80 pieces, not 86 + 86 (digest `d984cce9…`, 227 off). The stored S then blocks at its letter 961. A retained preparation from s61c that fills those six chambers must come first. That is the next experiment; it was not run.

### M1-15: later letters of one class do not help (proved, model review)

M1-04's obstruction holds even when the word continues after the carrier first lands. Whether retained letters of another class inside the core suffice on their own, as in C's core, is open.

The integrator re-ran the computations of M1-12, M1-13 and M1-14 from the repository root at commit `1a261bb` (13 s, 16 s and 1 min 52 s) and got the reported outputs.

## 4. Lemma C: the class-cell pure twist (round 1)

### Setting (LC-01)

- **Labels.** Fix a class of grips P = q·2I and put G = Sym⁺(P) and H = G ∩ K⁺. The five labels are the cosets G/H; label 0 is H.
- **Cells and contents.** A cell is a sign vector over the 120 caps of P. A content is the set of pieces with the same cell and the same domain. A point is a pair (cell, label).
- **The alphabet.** The letters are at class-P grips that are admissible and whose caps hold only the lattice and P's frames. A letter (c, g) sends (C, ℓ) to (gC, gℓ) when c is a cap of C, and fixes it otherwise.
- **Edge cells.** The pointwise stabilizer of an edge cell's two poles is C5, with generator ρ_e. Its action on the five labels is a 5-cycle.
- **Elementary commutators.** C = [x, y] is elementary when its action on edge positions is the identity and its label action is non-trivial at exactly two edge cells u and v, as (ρ_u^a, ρ_v^b). Its ratio is r_uv = b/a in F5.
- **Two kinds of powers.**
  - The power C^n is *genuinely pure* when n is the order of C on the non-edge points and 5 does not divide n; it then fixes every non-edge point.
  - The cheaper *goal-preserving* powers that `cellpure3.py` uses only map a goal set of non-edge points onto itself.

### LC-02: the ratio and the cycle criterion (proved, model review; computed)

- **The ratio (proved, model review).** For the elementary commutators, each pair of edge cells has a single exponent ratio.
  - Two grip stars meet on edge positions only at one edge cell e. If both twists move e, the commutator three-cycles the edge positions, so it is not elementary.
  - If exactly one twist fixes e, its rotation at e is paired with its inverse, transported to a second edge cell f.
  - Two such transports differ by an element that centralizes C5, so the ratio is unique.
- **Arbitrary pure words differ (computed; re-run by the integrator).** Take C1 = [(5,7),(38,33)], C2 = [(5,13),(79,9)] and C3 = [(38,10),(5,34)] at the E2 end of I_a@40, and put T_j = C_j^36. Then T1 has exponents (1, 1, 0) on the edge cells (5,38), (5,79) and (38,79), and T1^4 T2^2 T3^4 has (1, 2, 0). Both are pure words on the same pair, with ratios 1 and 2.
- **The cycle criterion (proved, model review).** Take a cycle of edge cells v_0 = X, v_1, ..., v_(k-1), with ratios r_i and R = r_0 ··· r_(k-1).
  - Normalise column i to the entries 1 at v_i and r_i at v_(i+1).
  - The rows other than X force t_i = (-1)^i (r_0 ··· r_(i-1)) t_0. The row at X reads [1 - (-1)^k R] t_0 = τ.
  - So the target τ·e_X with τ ≠ 0 has exactly one solution when R ≠ (-1)^k, and none otherwise. In words, the signed holonomy ∏(-r_i) must differ from 1.
  - R does not depend on the chosen generators ρ.
- **The recorded F48 word is not pointwise pure (computed; re-run by the integrator).** Its part C1^18 C2^18 C3^36 moves 55 non-edge points, among them three occupied contents. Every content still ends on label 0, and its ghost Bad set is the cell (5, 38).

The integrator re-ran the computation from the repository root at commit `1a261bb`, in 85 s. It built the class-1 model at the E2 end with `e3_class.Model`, read the recorded word, and computed the permutations and the F5 rank. The output was `nonedge 55 occupied 3`, `ghost_bad_cells [[5, 38]]`, `basis [[1, 0, 3], [1, 4, 0], [0, 1, 2]] rank 3`, and exponents `[1, 1, 0]` and `[1, 2, 0]`.

### LC-03: from the model to the puzzle (proved, model review)

- **Exact lifting.**
  - An own piece has pose h·k with h in G and k in K⁺, so it never straddles a cut of P, and foreign pieces stay outside the free caps. Every letter therefore stays admissible.
  - All pieces of a cell follow one rotation, so their labels move exactly as the model says.
  - A rotation that fixes a nonempty cell and all its labels is the identity: it is a right multiplication that fixes the nonzero sum of the cell's cap normals.
- **The corrected conclusion.**
  - For a genuinely pure word that takes the carrier's label to 0, Bad(W) is Ω_X, the home chambers of the carrier's cell X, and every piece outside X keeps its exact pose.
  - The 5-cycle also takes label 0 away from 0, so every lattice piece in X goes off. W alone is a clean step only when X holds no lattice piece.
- **The closure.**
  - Let V be a SAFE word of K⁺ letters that makes Ω_X vacant (Ω_X inside the holes; equality is not needed). Then V·W·V⁻¹ is a clean step.
  - A second protected content in X is incompatible with a nonzero 5-cycle; it must join the target or the free set.
- **What the cheaper powers need.** They need two extra checks: the protected pieces' exact poses, and Bad(W) on every home cell, vacant ones included. The implementation has both:
  - `tclose2.py` computes Bad(W) from the ghost of every chamber in the union of the word's caps (`scripts/echain/ghostlib.py`, `ghost`), occupied or vacant, and checks the protected pieces' exact contents;
  - `verify_u.py` replays every chain step from its start table.

  The steps of the chains therefore stand. What changed is the statement in `METHOD.md`: the cheaper powers may also permute lattice pieces outside X, which a clean step allows.

### LC-04 to LC-07: scope (proved, model review)

- **LC-04, multi-frame blocks.**
  - If the off contents of X all belong to class P, take one as the target and put the others in the free set; the closure then still gives a clean step.
  - One common rotation cannot put two different frame cosets on the lattice.
  - A rigid block with a part of a foreign frame cannot be moved by the fixed class-P alphabet: a free cap holding the target would also hold the foreign part.
- **LC-05, frozen blocks.**
  - The initial free alphabet never moves a frozen target, so no power or longer cycle helps.
  - The way out is Proposition F, once the straddlers of an inside grip are restored.
- **LC-06, several edge cells.** A target spread over several edge cells gives one F5 system, B·a = b.
  - In a connected pair graph the left null space has dimension at most one. It is zero when some cycle has signed holonomy different from 1.
  - Otherwise the target is reachable exactly when it satisfies the one null-vector condition.
- **LC-07, s61c.** At a state with 61 off pieces no whole-edge-cell pure closure can be clean.
  - Every edge cell has 86 home chambers, and all of them are Bad.
  - K⁺ letters keep the number of holes at 61, so they cannot vacate 86 chambers.
  - This obstructs the strategy only. It does not rule out other words, for example words that mix classes.

## 5. Lemma C as a theorem, with a bound (round 2)

### Theorem C (proved, model review; certificate re-run by the integrator)

Let P be a class of grips and X = {a, b} an edge cell of P. Suppose the grips a and b and one common neighbour c of a and b in P are free (LC-01). Then for every τ ≠ 0 in F5 there is a word of at most 720 menu letters, at the grips a, b and c only, with these properties:
- it acts on X by ρ_X^τ;
- it leaves every piece outside X at its exact pose;
- every letter is admissible when applied.

In particular this holds when every class-P grip is free.

*Proof.*
1. **Certificate (LC-08; computed).** In class 1 take the triangle of edge cells X = {5, 38}, Y = {5, 79}, Z = {38, 79}, and the commutators C1 = [(5,7),(38,33)], C2 = [(5,13),(79,9)] and C3 = [(38,10),(5,34)], each of four menu letters.
   - On the complete model of 2,641 cells and 13,205 points, each commutator's cycles on the non-edge points have lengths 1, 4, 9 and 12. Its order there is 36, prime to 5, so T_i = C_i^36 is genuinely pure.
   - The exponent columns are B = [[1, 0, 3], [1, 4, 0], [0, 1, 2]], with determinant 11 = 1 mod 5. So B·(3τ, 3τ, τ) = (τ, 0, 0).
   - The product of the powers was checked point by point on all 13,205 points for the four values of τ.
   - The ratios around X → Y → Z → X are 1, 4 and 4, so the signed holonomy is 4, not 1, as the cycle criterion requires.
2. **Symmetry.** G = Sym⁺(P), of order 7,200, acts on the cell-label model and conjugates a menu letter at c to a menu letter at g(c).
   - G is transitive on edge cells, and the pointwise stabilizer of an edge rotates its five triangles transitively. So G is transitive on triangles, and the certificate transports to any triangle {a, b, c}, with the same length and purity.
   - Under a change of generators the ratios change by factors that telescope around a cycle, so the signed holonomy is preserved.
   - K⁺ permutes the five classes and their menus, which extends the certificate to every class.
3. **Lifting (LC-03).** Freeness persists along the word, because foreign pieces lie outside the caps used and own pieces never straddle a cut of P. So the model word lifts to admissible letters with exact action.
4. **Length (LC-09).** Each T_i has 144 letters and order five. With signed coefficients in {−2, ..., 2} (inverse letters exist at the same grips), the solutions for τ = 1, 2, 3, 4 have signed coefficients (−2, −2, 1), (1, 1, 2), (−1, −1, −2) and (2, 2, −1). Their lengths are 720, 576, 576 and 720. **QED**

The integrator re-ran the certificate from the repository root at commit `1a261bb` (7 s, in memory). It printed `nonedge_order 36` for each commutator, `matrix [[1, 0, 3], [1, 4, 0], [0, 1, 2]]`, the four coefficient vectors with lengths 720, 576, 576 and 720, and `PASS full 2641-cell, 13205-point certificate`.

### The whole step

- **The closure (LC-09).** Let V come from a peeling certificate (Theorem B of `research/jumbling/theory/buffer-transport.md`) that vacates Ω_X; its thresholds must hold for the state's holes, orbit by orbit. Then V·W·V⁻¹ is a clean step with |V·W·V⁻¹| <= 2·L_cert + 720, where L_cert is the certificate's length bound.
- **Holes.** The total |holes| >= 86 is necessary, but it is not sufficient.
- **The span criterion (LC-10).** With only some grips free, keep the pure elementary tools whose letters are free. A twist vector b is reachable exactly when λ·b = 0 for every λ in the left null space. For one cell this means: X's connected component in the pair graph contains a cycle whose signed holonomy is not 1. X itself need not lie on that cycle.
- **What `cellpure3.py` does.** It searches cheaper goal-preserving powers and cycles of length three or four. Its per-pair costs are never worse than the pure baseline. But it is a search: Theorem C says what exists, not what the search returns.

### What the implementation certifies (LC-11)

- **Bad(W).** `tclose2.py` computes Bad(W) from the ghost of every home chamber in the union of the word's caps, occupied or vacant. That covers every chamber that can be bad. Nothing compares that set with Ω_X, so the integrator added the comparison and ran it on the stored application at s86a. Chain step 14 is V·W·V⁻¹ with |V| = 543 and W the 288-letter pure twist. `scripts/gm/omega_check.py` ghosts the 8,106 home chambers of W's caps and finds Bad(W) = Ω_{5,11} exactly, 86 chambers (`earlier/stop62-860-to-0/step014-pure-twist-omega.json`; 7 s). Bad(W) depends on the word alone, so no state is needed.
- **Exact poses.** `tclose2.py`'s `protected_exact_at_end` compares ghost domains and chambers, and these do not determine an exact pose. Counterexample (computed; re-run by the integrator): home chamber 1939 at grip 5, initial pose γ = E(5, 12), and h = γ·E(5, 1)·γ⁻¹ = E(5, 55). Applying h keeps the ghost at (domain 1, chamber 1939) but changes the pose from γ to γ·E(5, 1). The exact check is done by `cellpure3.py`'s pose comparison and by `verify_u.py`'s exact pose codes.
- **The replay is the certificate.** `verify_u.py`'s checks are exactly the clean-step conditions: every twist admissible, collateral 0, the unit restored, no protected off piece restored, every protected off piece at its exact pose. It reports `all_ok`, but its exit status does not reflect it, so acceptance must read `all_ok`. Every stored chain replay in `research/jumbling/restoration` has `all_ok: true` and no row with `ok: false` (the integrator checked all 10).

## 6. Lemma C for every cell kind (round 3)

### LC-12: what one cell can do in place (proved, model review; computed, re-run by the integrator)

Suppose every class-P grip is free. The table gives, for each cell kind, the group of label permutations that a word can realize on one cell while fixing every other point of the complete model:

| Cell kind | Home chambers per cell | Group in place | Real contents |
| --- | --- | --- | --- |
| vertex | 385 | A5, transitive on the five labels | landed in place, at most 104 letters |
| edge | 86 | C5 | landed in place by Theorem C, at most 720 letters |
| triangle | 111 | C3, fixing two labels | cannot be landed in place |
| tetrahedron | 87 | V4, fixing one label | cannot be landed in place |
| outer | 5,280 | identity | cannot be moved by its own class |

- **Why no more.** The setwise stabilizers in Sym⁺(P) are A5, D5, S3 and A4. Each grip's menu generates the perfect group A5, so every word has trivial abelianized orientation. A word supported on one cell therefore acts by an element of the derived subgroup: A5, C5, C3 or V4.
- **Attained.** Certificates of 52 (vertex), 56 (triangle) and 112 (tetrahedron) letters realize the groups.
- **Real contents.** Home chambers of a triangle or tetrahedron cell occur only in the labels these groups fix. So a real triangle or tetrahedron carrier must be moved to another cell to land.

### LC-13: pure transport (proved, model review; computed, re-run by the integrator)

- **Seeds.** A 48-letter seed for triangles and a 24-letter seed for tetrahedra each act as five parallel 3-cycles on three whole cells. They fix every other point, and every other cell's rotation, exactly.
- **The graph.** Conjugating the seeds by the 7,200 symmetries gives a graph on the points. It is connected, with diameter 5 on the 2,400 points of real triangle contents and 8 on the 600 points of real tetrahedron contents.
- **Bounds.** A real triangle carrier reaches label 0 in any triangle home cell in at most 5 × 48 = 240 letters, and a tetrahedron carrier in at most 8 × 24 = 192.
- **Bad set and closure.** Bad(W) is the union of the home cells whose label 0 the word moves; no ghost dies. The closure needs those chambers vacant.
- **Hypothesis.** The bounds assume every class-P grip is free. At a real state each word must be checked against the free grips and the protected pieces.

### LC-14: one cell per rigid block (proved, model review)

A rigid block whose pieces lie in one frame lies in exactly one class cell: two pieces with different class-P cap signatures are separated by a class-P cut. Landing several contents at once is a finite subgroup-membership problem, decidable by search.

### LC-15: what occurs in practice (computed, re-run by the integrator)

At the stop62 start (860 off) every unit is one content in one cell: 5 units in edge cells, 7 in triangle cells and 3 in tetrahedron cells. No class has all 120 grips free (115, 114, 113, 110 and 111 are free), but every pole of every unit's cell is free. So transport, not the edge-cell twist, is the common case. The driver v4 has a pure-twist stage for edge cells only.

The integrator re-ran both computations at commit `1a261bb` (7 s and 13 s) and got the reported outputs: the orbit table, the three in-place certificates, the two transport seeds and their graphs, the home-cell table and the 15-unit inventory.

## 7. Review of the clean-step proofs (stage 08, `PROOFS-U.md`)

The reviewer checked six result groups against their stated hypotheses. Computed certificates were taken as given, not replayed.

| Result | Verdict | Change made |
| --- | --- | --- |
| Fact 1′ and Lemma U | verified: holes of type t number the off pieces of type t, and the fall of the off count is exact | none |
| Lemma D, Corollaries D' and D'' | verified; the remark after D'' (Bad(W)_t = L_t) needs f_t = 0. At depth one, a coset letter's Bad set is the whole cap while the target lands in part of it | remark restricted to f_t = 0 |
| Proposition F (frozen blocks) | verified as a conditional unlocking result; it does not show that the straddlers can be restored | none |
| Proposition GH (glued holes) | verified | none |
| Lemma CM (carrier move) | the last implication was false as written | statement corrected (below) |
| Theorem D1 (depth one) | verified | none |

**Lemma CM.** Version 3.1 said: if V·W·V⁻¹ is a clean step at P0(Z) for A with B in its free set, then P0·V·W·V⁻¹ is a clean step for A at Z with free set B. That is false:
- At stop62@233, let A be the carrier 61@23955, B the carrier 86@1040 and C the carrier 86@64184.
- An inverse history to the solved state is a clean step for A with free set B ∪ C, but not with free set B, because it restores C.

The corrected lemma keeps the same free set: a clean step at P0(Z) for A with free set F ⊇ B gives a clean step for A at Z with the same free set F. The three carrier-move constructions use F = B, so their certificates stand.

## 8. Review of the lattice phase (stages 03, 04 and 09)

The reviewer found the lattice phase complete for reachable lattice configurations, given the certificates (G = 𝒦, CERT-LB-V2, POT5). It raised no blocking finding. It changed the following:
- **Theorem R holds for reachable configurations.**
  - The earlier statement said every configuration. One with every piece rotated by the same order-three element of K⁺ has ν = 0 but displaced centres, so the retained cascade cannot restore it.
  - "Solved" means every sticker at its home.
  - The stage 09 order, with the corrections last, follows from a short relative-state argument: choose C with ν(C) = −ν(Y), put T = solved·C⁻¹ and solve Y relative to T.
  - Stage 03 `report.md` section 3.2 and `METHOD.md` now say so.
- **Theorem A had an inverse typo.** Changing the anchor x_c to x_c·a replaces k by a⁻¹k, so V(X) changes by the retained twist (c, a⁻¹). F1 kills either one, so the conclusion stands. Fixed in stage 04 `PROOFS.md`.
- **Lemma R needs its frame convention.**
  - The refined state keeps only the class j, so grips and twists must be written in the frame of the class's canonical region: d' = k⁻¹(d) and h' = k⁻¹hk for the actual region k·R_j.
  - `scripts/refexact.py` already does this. The text now states it.
  - With it, rules R1 to R5, Lemma T, Lemma P and Theorem B5 check out.
- **The library spans A_J.**
  - The reviewer read `data/library-merged.json.gz` independently: 36 macros, binary rank 34 with row space exactly V34, rank 2 over F5, and every coordinate excluded by Theorem A zero.
  - The two ranks suffice by the Chinese remainder theorem; `METHOD.md` now says why.
  - The macros' legality and their recorded ν values rest on the certificates.

## 9. The method as a whole (round 4 audit)

A milestone audit asked whether the results above compose into one complete method, what the toolkit of sections 4 to 6 proves on its own, and how a state can fail. The integrator re-ran its three computations at commit `1a261bb` and got the reported outputs.

### AU-01 and AU-06: the general algorithm M\* (proved, model review)

**Theorem M\*.** For every reachable I_a configuration, the following algorithm reads only the current state and ends with all 259,800 sticker labels solved:
1. While a piece is off the lattice, list the units and pick one, U.
   - Try a finite batch of heuristics (the driver step, the pure twist, transport words). Accept only an exactly replayed clean step.
   - If none succeeds, enumerate all I_a menu words in order of length. Replay each from the state, and take the first clean step for U with free set off(Z) \ U.
2. When no piece is off the lattice, run the lattice restorer with a spanning library, corrections last.

It takes at most as many clean steps as there are off pieces at the start. It needs no history, no bound on heights and no finiteness of the reachable set.

*Proof.*
1. **The fallback stops.** The inverse of any history from the solved state is a clean step for U (M1-01). Each length has finitely many words, and every test is exact. The history appears only in the proof, never in the algorithm.
2. **Each step makes progress.** By Lemma U the next off set lies inside off(Z) \ U, so the off count falls by at least one. Every state stays reachable.
3. **The lattice phase finishes.** At the lattice endpoint Y, ν(Y) lies in A_J because Y is reachable. The library spans A_J, so a correction C with ν(C) = −ν(Y) exists. With T = solved·C⁻¹, the retained cascade takes Y to T in sticker labels, and C finishes (Theorem R, section 8). **QED**

The theorem gives no bound on the length of a fallback word. Lengths are bounded only for the toolkit, under the checks of AU-03, and for the lattice phase (AU-05).

**v4 is not yet M\*.** It has no fallback (M1-02). Its outer loop also has to change in three ways:
- **The step limit.** `--max-steps` must not end M\*.
- **The lattice phase can be skipped.** `gdrive.py` tests off = 0 only at the start of an iteration. A step that reaches the lattice on the last permitted iteration therefore ends the run without the lattice phase (the integrator checked this in the source).
- **Fairness.** A heuristic batch must end, or share time with the fallback, before the fallback counts as fair.

`--pure-max` may stay as a heuristic limit.

### AU-02: the toolkit does not reach every unit (proved, model review; computed, re-run by the integrator)

Under any word of class-P grips, the accumulated rotation of every piece lies in Sym⁺(P). So a piece with pose γ can land under such a word only if γ ∈ Sym⁺(P)K⁺: it must start on the lattice or in a frame of class P (left height two). Preparation keeps off poses, so V·W·V⁻¹ does not help.

Reachable states have deeper pieces. The integrator replayed the first 150 twists of the I_a fixture, with every twist admitted, and counted:

| Left height | Pieces |
| --- | ---: |
| (0, 0) | 56,906 |
| (2, 0) | 118,803 |
| (4, 0) | 1,411 |

The state digest is `2bfe305f…`; the re-run took 29 s. No unit with a height-four piece can be landed by the single-class toolkit. M\* is unaffected, because its fallback has no such restriction.

The repair is a check before any class-P word:
- the unit's domain is a frame of class P;
- the unit lies in one non-outer cell content with a definite label.

LC-14 holds for a block inside one class frame and does not extend to other domains. Containment suffices. `cellpure3.py` asks for more: the unit's whole domain in one cell.

### AU-03: the toolkit theorem with bounds (proved, model review, given the certificates of sections 5 and 6)

Let U pass the AU-02 check, and let W be a class-P word that lands U. The closure V·W·V⁻¹ is a clean step with free set off(Z) \ U when these finite checks pass at Z:
1. every grip W uses is free, and the exact class-cell action lands U at label 0;
2. every other off piece either lands or ends at its exact pose under W;
3. the exact lift gives Bad(W) = ⋃ Ω_f over the cells f with φ_f(0) ≠ 0;
4. vacancy holds for all targets at once (below).

Check 2 is what makes V⁻¹ admissible. Without it, a full replay of V·W·V⁻¹ must be the acceptance test.

The step has at most 2(Q_safe − 1) + b_κ letters, or 2·L_peel + b_κ with a complete peeling certificate, where:
- b_κ = 720 (edge), 104 (vertex), 240 (triangle) or 192 (tetrahedron);
- Q_safe = ∏_Δ binom(n_Δ, h_Δ) over the orbits Δ of the SAFE alphabet, with n_Δ chambers and h_Δ holes in Δ;
- L_peel = Σ_j (|E_j| + e_j).

The landing bounds assume that all of the class's grips are free. With some grips blocked, an actual word whose grips are free must be checked: the diameter of the unrestricted graph does not bound the restricted one.

**Vacancy must hold for all targets at once.**
- A peeling certificate proves vacancy at the thresholds β_Δ = max_j(|P_j ∩ Δ| + 1 + u_j). N2 alone meets them only when every u_j = 0.
- Separate certificates per orbit do not prove vacancy for all targets together.
  - Example: the diagonal Sym(3) acting on {1, 2, 3} and {1′, 2′, 3′}, generated by (1 3)(1′ 3′) and (2 3)(2′ 3′), with holes {1, 2′} and targets {1, 1′}.
  - N1 and N2 hold, and each orbit can be vacated alone. Doing both at once would need one σ with σ(1) = σ(2) = 1.
- Valid certificates are:
  - Jordan certificates on every non-fixed target orbit, with N1/N2;
  - one complete peeling certificate whose thresholds are met;
  - a peeling certificate for the non-Jordan targets first, then the Jordan factors.

**A bound without a certificate.** SAFE letters keep the number of holes in each orbit, so at most Q_safe hole sets are reachable. A shortest vacating path repeats none, so |V| ≤ Q_safe − 1. A breadth-first search over whole hole sets finds V from the state alone. Searching each orbit separately is not enough.

### AU-04: how a state can fail (partly proved; the stronger claim is open)

- **Proved: there is always an admissible mover.** Take the last letter of a reaching history that moves an off piece. Its cap holds that piece, and nothing straddles the cap at Z. So some cap is admissible and holds an off piece, and a scan of the 600 caps finds one without the history.
- **Open: an eligible unit whose grips are all free.** The mover may hold foreign-frame pieces or belong to another class, and the grips that different units need may block each other. No reachable counterexample is known.
  - The cheapest test runs at prefixes of the fixture. List the rigid components by domain and check their eligibility. Record the blocked grips and the foreign-frame occupancy for every grip a unit needs.
- **Failures are per candidate.** "No unit passes" means that every candidate (U, W) fails some check: eligibility, free grips and live ghosts, the counts N1/N2, a simultaneous certificate with its thresholds, or the end-pose check. A missing certificate shows the limit of a sufficient test, not that vacancy is impossible.
- **Hole counts are a weak guide.**
  - If type t is deficient, then h ≤ 177,120 − N_t + b_t − 1.
  - For a single home cell this gives h ≤ 176,520 (vertex), 175,681 (edge), 175,682 (triangle) and 177,000 (tetrahedron), far above any cell size. Every home cell of one kind has the same type vector in all five classes (computed, re-run by the integrator in 2 s).
  - A deficient SAFE orbit Δ likewise allows up to 177,120 − |Δ| + |T ∩ Δ| − 1 holes. The recorded X5 case shows the gap: it has 277 holes for an 86-chamber target and fails the finer orbit counts.
- **Support bounds for transport.** From the seeds and the graph diameters, |Bad| ≤ 1,665 for triangle transport (15 cells) and ≤ 2,088 for tetrahedron transport (24 cells). The bounds are 385 for vertex words and 86 for edge words.

### AU-05: a bound for the lattice phase (proved, model review; computed, re-run by the integrator)

The quoted 1.5 to 2.2 × 10^8 twists were measurements. The stage 03 report already lists retained solves of 229,763,734 and 230,484,810 twists.

A bound for every input follows from the code. For each moving orbit o, let:
- N_o be its number of positions, buffers included;
- s_o be the seed length plus twice the relocation length;
- d_o be the depth of its execution tree.

The bound has three parts:
- **The position loop** runs at most 2N_o cycles, each of at most 2(s_o + 2d_o) letters. A cycle that does not lower the number of wrong positions is always followed by one that does.
- **The frame pass** uses at most N_o − 1 pairs and one commutator, each pair of at most 4s_o + 6d_o letters.
- **The corrections** reduce each coefficient modulo the order (1, 2, 5 or 10) of the macro's ν and take the shorter of the word and its inverse.

| Part | Bound (twists) |
| --- | ---: |
| Retained cascade R: Σ_o [4N_o(s_o + 2d_o) + (N_o + 3)(4s_o + 6d_o)] over the committed trees | 1,324,660,182 |
| Corrections C, 36-macro library | 25,852 |
| Total | 1,324,686,034 |

This bounds the word, not the search time, and it says nothing about phase 1. At one twist per second the bound is about 42 years; the measured runs (1.5 to 2.3 × 10^8 twists) are 5 to 7 years. The integrator's re-run took under 1 s.

### Where the method stands after round 4

- **Proved (model review): a general method.** M\* restores every reachable configuration from the current state alone, in at most |off| clean steps followed by the lattice phase.
- **Bounded where the toolkit applies.**
  - Under the AU-03 checks, a clean step has at most 2(Q_safe − 1) + b_κ letters.
  - The lattice phase has at most 1,324,686,034 letters.
- **Open:**
  - a bound on the length of a fallback word;
  - whether the toolkit or the partner template covers every state;
  - whether some eligible unit always has its grips free;
  - bounded constructions for deeper domains.
- **The reviewer's next question.** Does every reachable depth-one state with no toolkit step pass the three-phase partner template of M1-11, or is there a reachable counterexample?
- **The implementation.** v4 is a partial implementation of M\*: it lacks the fallback, and its outer loop must change as described under AU-01.
  - *Integrator's note.* Since this audit the driver implements M\*: `mstar.py` (the outer loop and the exhaustive word search), `fallback.py` (the search on the simulator) and the main loop of `gdrive.py`; METHOD.md section 2, step 4. A Codex review of the code at the fast tier found two major defects, both fixed before commit: a heuristic closure killed by its timeout could be taken without its end table, and a recursive search capped the word length at the interpreter's recursion limit. The runs of METHOD.md section 6 used v4.

The commands for the three computations, as re-run by the integrator from the repository root (`python3 -I -B <file>`):

AU-02: heights after 150 twists of the I_a fixture:

```python
import sys
sys.path[:0] = ['research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import numpy as np
import sim, wj, trees
ctx=sim.get_context()
fixture, menu=wj.load(ctx,'I_a')
st=sim.State(ctx,menu=menu)
for i,rec in enumerate(fixture['journal']['records'][:150],1):
    assert st.apply(sim.Twist.from_record(ctx,rec)).applied, i
counts=np.bincount(st.pose_id,minlength=len(st._poses))
hist={}
for pid in np.flatnonzero(counts):
    h=(0,0) if st._poses[int(pid)].kidx>=0 else tuple(trees.pose_tree(st._pose_matrix(int(pid)))[:2])
    hist[h]=hist.get(h,0)+int(counts[pid])
print({'twists':150,'heights':hist,'digest':st.digest()},flush=True)
```

AU-04: one type vector per cell kind, and the deficit bounds:

```python
import json
from pathlib import Path
from collections import Counter,defaultdict
import numpy as np
with np.load('assets/model.npz',allow_pickle=False) as a:
    offsets,masks,types=a['mask_offsets'],a['mask_values'],a['orbit_id']
    members=json.loads(Path('research/jumbling/theory/results/classes-I_a.json').read_text())['classes']['members']
    N=Counter(map(int,types)); vectors=defaultdict(set); counts=Counter()
    for P in members:
        poles=set(P); cells=defaultdict(Counter)
        for p,t in enumerate(types):
            cell=tuple(sorted(poles.intersection(map(int,masks[offsets[p]:offsets[p+1]]))))
            cells[cell][int(t)]+=1
        for cell,v in cells.items():
            vectors[len(cell)].add(tuple(sorted(v.items()))); counts[len(cell)]+=1
    for k in range(1,5):
        assert len(vectors[k])==1
        e=dict(next(iter(vectors[k])))
        upper=max(len(types)-N[t]+b-1 for t,b in e.items())
        print({'kind':k,'cells_all_classes':counts[k],'cell_size':sum(e.values()),'type_deficit_h_upper':upper})
```

AU-05: the lattice-phase bound:

```python
import gzip,json
from pathlib import Path
trees=json.loads(gzip.decompress(Path('assets/execution_trees.json.gz').read_bytes()))
total=0
for t in trees:
    N=len(set(t['positions']))+2
    cache={}
    def depth(i):
        if i not in cache:
            cache[i]=0 if t['parent'][i]<0 else depth(t['parent'][i])+1
        return cache[i]
    d=max(map(depth,range(len(t['parent']))))
    s=len(t['seed'])+2*len(t['relocation'])
    total+=4*N*(s+2*d)+(N+3)*(4*s+6*d)
lib=json.loads(gzip.decompress(Path('research/jumbling/restoration/09-general-method/data/library-merged.json.gz').read_bytes()))['macros']
cbound=0
for m in lib:
    has2=any(v[0] or (int(o) in [0,3,6,8,11,22,25,32] and v[1]) for o,v in m['nu'].items())
    has5=any(int(o) in [17,28] and v[1] for o,v in m['nu'].items())
    order=(2 if has2 else 1)*(5 if has5 else 1)
    cbound+=(order//2)*len(m['records'])
print({'R_bound':total,'C_bound':cbound,'total':total+cbound,'macros':len(lib)})
```

## 10. Coverage of the bounded toolkit at depth one (round 5)

Round 5 asked the audit's next question: do the bounded constructions supply a clean step at every reachable depth-one state? The constructions are the toolkit of AU-03 and the partner template of M1-11. A *depth-one state* is a reachable state in which every off piece has left height two.

The question is not settled. The round narrows it, proves two bounded extensions and a sharper vacancy bound, and gives a finite test with a positive case. The integrator re-ran the two computations (M1-17 and M1-19) and got the reported outputs.

### M1-16: what the template decides (proved, model review; the coverage question stays open)

- **Open.** Does every reachable depth-one state without a toolkit step admit some three-phase partner template? There is no proof and no reachable counterexample.
- **The scope of M1-11 (proved).** M1-11 decides one template with given cores D and W, not every creator and middle.
  - For a fixed state and a finite catalogue of cores, the retained searches are finite.
  - With k rigid carriers and hole vector a, a record holds the carriers' exact K⁺ multipliers and the complete hole set. There are at most Q_k(a) = 7200^k · ∏_t binom(N_t, a_t) records.
  - If the cores keep the other original carriers, the three phases need at most Q_k(h) + Q_{k+2}(h + e_1 + e_2) + Q_k(h − u + e_2) + |D| + |W| + 717 letters. Here h is the whole initial off vector and u is the target carrier.
  - Where rigidity is not proved, the records track one multiplier per off piece instead.
  - A failed search rejects the catalogue, not every possible template.
- **An edge middle needs u ≤ e (proved).**
  - By Lemma K, |Bad(W)_t| ≥ |F_t| when W lands F.
  - A middle with Bad vector 2e that lands U together with one full e-partner therefore needs u + e ≤ 2e, so u ≤ e. More identical partners cannot repair a violation; a different middle is needed.
  - At s61c, u ≤ e holds, and types 27 and 28 have u_t = 0 < e_t. With r full partners there are u + r·e holes, so vacating the 2e Bad set needs r ≥ 2. This is the arithmetic of M1-13.
- **The s61c preparation exists (proved).**
  - Write the recorded C chronologically as P·V·W·V⁻¹, with |V| = 849. Its inverse begins V·W⁻¹.
  - Just before W⁻¹, the landing chambers of both partners are occupied.
  - So a retained preparation that fills the six source chambers of M1-14 exists. Finding it from the state is a separate experiment.
- **A caution.** The finite bound on depth-one poses does not justify searching only the solved component of the depth-one graph. No theorem says that every reachable depth-one state is reached by a path that stays at depth one.

### M1-17: vacancy by one search over target images (proved, model review; the counterexample re-run by the integrator)

- **An exact criterion.** Let G be the SAFE-ALL group, H the holes and T the target. For each orbit Δ_i that meets T, let n_i = |Δ_i|, h_i = |H ∩ Δ_i| and k_i = |T ∩ Δ_i|.
  - Vacancy holds exactly when the G-orbit of the tuple (T ∩ Δ_i)_i contains a tuple (S_i)_i with S_i ⊆ H ∩ Δ_i, because T ⊆ g(H) if and only if g⁻¹(T) ⊆ H.
  - A breadth-first search on target images decides it, and the inverse of its witness vacates T.
- **A sharper bound.** Put Q_T = ∏_i binom(n_i, k_i) and Q_H = ∏_i binom(n_i, h_i). When a vacating word exists, a shortest one has at most min(Q_T, Q_H) − 1 letters.
  - For a target of fixed size m, Q_T ≤ N^m, where N is the number of relevant chambers.
  - A non-outer single-cell target has m ≤ 385.
- **Counts suffice only with independent alternating factors.**
  - Suppose the joint restriction of G to the remaining target orbits contains the product of their alternating groups, each orbit of size at least three. Then the counts suffice.
  - Alternating projections alone do not suffice. Example: diagonal A5 on two five-point copies, with holes {1, 2′} and targets {1, 1′}. This is an abstract group, not a puzzle state.
- **Distinct degrees (proved).** When the non-fixed target orbits have pairwise distinct sizes n_i ≥ 5, alternating projections do give the full product.
- **Search per orbit, then a compatibility check.** This is exact only when every choice S_i and its whole transporter coset {g : g(S_i) = T ∩ Δ_i} are kept. One local witness per orbit is not enough.
- **Polynomial certificates.** Straight-line programs for the cycles (1 2 j), j = 3, …, n_i, on each orbit, fixing the other target orbits, certify the alternating factors. Their size and checking time are polynomial. They do not promise short words.

### M1-18: retained conjugation extends the toolkit (proved, model review)

- **The construction.** Let A be a legal retained word at Z, and put Y = A(Z). Suppose an AU-03 closure C = V·W·V⁻¹ at Y lands every piece of U as transported by A, and every other off piece of Y lands or keeps its exact pose. Then A·C·A⁻¹ is a clean step for U at Z.
- **Its length.** At most 2r + 2B + b letters, where:
  - r = |A| and b = |W|;
  - B is a vacancy bound: Q_safe − 1, the bound of M1-17, or a peeling bound.
- **The setup search is finite from the state.** With h off pieces and type counts h_t, there are at most R(Z) = 7200^h · ∏_t binom(N_t, h_t) records, so a shortest setup has r ≤ R(Z) − 1.
- **The first test (not run).** Try r = 1: one retained letter before the toolkit and its inverse after. Such a letter can change foreign-frame interference and the SAFE-ALL orbits without creating partners.

### M1-19: a finite test for free grips, and a positive case (proved, model review; computed, re-run by the integrator)

- **Open.** Does every reachable depth-one state with off pieces have a candidate that passes AU-03 apart from vacancy? The admissible-mover lemma does not settle this, because an admissible cap can hold foreign-frame pieces.
- **A finite exact test at a given state (proved).**
  - For each class P, use its free alphabet and track the accumulated rotation a_C ∈ Sym⁺(P) of every initial class cell C.
  - An action qualifies when it lands an eligible unit, and every other off content of the class either keeps its pose (a_C is the identity) or lands (a_C γ ∈ K⁺).
  - There are at most 7200^2640 lifted actions, so exhausting them decides the question at that state.
- **An obstruction (proved).** Suppose one class-P cell holds off pieces of two frame domains. Then every word that passes AU-03 check 2 acts trivially on that cell, so neither content can be the target.
- **A positive case (computed).**
  - The state: the first 115 twists of the I_a fixture. Its digest is `f639b10b…`, and it has 114,014 off pieces, all of height (2, 0).
  - Grip 194 is free for class 1. The letter (194, 12) lands 1,410 off pieces and leaves every other off piece at its exact pose. It also takes 395 lattice pieces off, so it is a witness for this question, not a clean step.
  - Grip 91 is admissible but holds off pieces of classes 0 and 3. So admissibility does not imply freeness.
- **The next test (not run).** The 11 non-identity retained letters at grip 91 of this state.

### Where the method stands after round 5

- The coverage question of round 4 stays open, with three bounded extensions to test before new creators: retained conjugation (M1-18, r = 1 first), the template with a stated catalogue (M1-16), and the free-grip test (M1-19).
- Vacancy has an exact search with the bound min(Q_T, Q_H) − 1 (M1-17).

The commands, as re-run by the integrator (`python3 -I -B <file>` from the repository root):

M1-17: the diagonal A5 counterexample (prints `"vacating_elements": 0`):

```python
"""M1-17 check: diagonal A5 on two copies of {0..4}; holes {0, 1'} and targets {0, 0'}. Per-orbit counts and both
projections hold, but no g has g^-1(T) inside H."""
import itertools, json

def parity(p):
    s, seen = 0, set()
    for i in range(len(p)):
        if i in seen:
            continue
        j, n = i, 0
        while j not in seen:
            seen.add(j); j = p[j]; n += 1
        s += n - 1
    return s % 2

A5 = [p for p in itertools.permutations(range(5)) if parity(p) == 0]
H = {('a', 0), ('b', 1)}
T = [('a', 0), ('b', 0)]
def inv(p):
    q = [0] * 5
    for i, x in enumerate(p):
        q[x] = i
    return q
ok = [p for p in A5 if all((s, inv(p)[x]) in H for s, x in T)]
proj = len({p for p in A5}) == 60
print(json.dumps({'group_order': len(A5), 'per_orbit_counts': [1 >= 1, 1 >= 1], 'projections_A5': proj,
                  'vacating_elements': len(ok)}))
```

M1-19: the positive case at I_a@115 (about 20 s):

```python
import sys
sys.path[:0]=['research/jumbling/theory','research/jumbling/fixtures','research/jumbling']
import numpy as np
import classes as CL, restore, sim, trees, wj
fixture,M=wj.load(restore.ctx,'I_a')
st=sim.State(restore.ctx,menu=M,filtered=False)
for rec in fixture['journal']['records'][:115]:
    assert st.apply(sim.Twist.from_record(restore.ctx,rec)).applied
R=restore.Restorer(st,M); D=R.domains(); dom=np.asarray(D.dom)
members,cls=CL.classes(); dc={}
for d in range(1,len(D.reps)):
    assert tuple(trees.dist(q) for q in trees.factors(D.reps[d]))==(2,0)
    hubs=CL.hub(D.reps[d],members,cls); assert len(hubs)==1
    dc[d]=hubs[0][1]
print('start',st.digest(),'off',int((dom!=0).sum()),flush=True)
for c in (194,91):
    q=st.classify(c,need_inside=True,stop_at_straddle=True)
    assert not q.uncertain and q.straddle is None
    ds=set(map(int,dom[np.asarray(q.inside,dtype=np.int64)]))-{0}
    print('grip',c,'class',int(cls[c]),'off_domain_classes',sorted({dc[d] for d in ds}),flush=True)
    if c==194:
        assert ds and all(dc[d]==cls[c] for d in ds)
old=st.pose_id.copy(); matrices=[st._pose_matrix(i) for i in range(len(st._poses))]
off=~st.lattice_flags(); R.apply(194,12)
nowoff=~st.lattice_flags(); remaining=off & nowoff
assert int((off & ~nowoff).sum())==1410
pairs=np.unique(np.stack((old[remaining],st.pose_id[remaining]),axis=1),axis=0)
assert all(matrices[int(a)]==st._pose_matrix(int(b)) for a,b in pairs)
print('witness',(194,12),'original_off_landed',int((off & ~nowoff).sum()),'remaining_original_off_exact',True,'source_lattice_off',int((~off & nowoff).sum()))
```

## 11. The length of the lattice phase (round 6)

Round 6 asked for a quantitative answer to gap M2: constructions with proved length bounds, lower bounds, and the cost to a person who replays macros with the program's help. The integrator proposed two measures; the owner has not fixed one.
- **Twists L**: the number of primitives in the word. Where it matters, also *cap twists*: a maximal run of letters at one cap counts once, as an element of the cap's group A4.
- **Actions A**: a person performs cap twists, one action each, and macro applications, one action each. A macro is a word the person defined once; the program stores it and replays it at any of the 7,200 placements of K⁺. Creating the library costs the sum of its lengths in twists, reported apart from A.

The round proves lower bounds (M2-02), a shorter frame construction (M2-03) and a cascade bound of 356,565,166 twists with the committed seeds (M2-04). It corrects the single-overlap lemma (M2-01), the reading of the measured runs (M2-05) and a setup remark of the packet (M2-06). It establishes no uniform bound of 200 twists per orbit and no better cascade order. The integrator re-ran every exact computation of the round and got the reported outputs. The integrator's own searches for short pure elements (S1 and S2) follow the findings.

### M2-01: the single-overlap lemma, corrected (proved, model review; computed, re-run by the integrator)

- **The lemma.** Let S(X) be the set of pieces with any sticker moved by X, frame changes at a fixed position included. Suppose S(X) ∩ S(Y) = {p} and X and Y both move p to another position. Put x = X⁻¹(p) and y = Y⁻¹(p).
  - Then [X, Y] = X·Y·X⁻¹·Y⁻¹ (chronological) moves the positions p → y → x → p and fixes every other piece. Its length is at most 2(|X| + |Y|).
  - With α the fibre map of X from x to p and β that of Y from y to p, its fibre maps are β⁻¹, α⁻¹β and α, whose product around the cycle is the identity. So the commutator is **pure**: its cube fixes every sticker.
- **Both positions must move.** Two C2 frame pairs on {p, a} and {p, b} meet in one piece and commute, so their commutator is the identity. Counting only displaced positions is unsafe too: it omits frame collateral.
- **Pure is not aligned.** The cube identity does not make the cycle's fibre maps agree with the ordered fibres of the cascade. Using a pure cycle in the committed kit needs a relocation to its buffers, charged in the length, and exact agreement with its fibres.
- **Why the spines shrink slowly.** For I = S(X) ∩ S(g), S([X, g]) ⊆ I ∪ X⁻¹(I) ∪ g⁻¹(I), so |S([X, g])| ≤ 3|I|. This gives no contraction when much of X's support meets the cap, while a commutator with a primitive takes the length from ℓ to 2ℓ + 2. Also S(X) ∩ g(S(X)) contains every piece of S(X) outside g's cap region, so conjugating by one cap twist cannot leave a single overlap.
- **Placement cost (computed).** In every cap, the 12 elements of A4 lie at primitive distances 0, 1, 2, 3 with multiplicities 1, 3, 4, 4. A K⁺ placement keeps the number of cap twists but can turn one primitive into three, so [X, σ(Y)] has at most 2(|X| + 3|Y|) primitives and 2(|X| + |Y|) cap twists.
- **The short small-support family is too small (computed).** Of the 119 distinct atlas subnodes of formula length at most 50, 20 move at most 600 pieces, and none of these 20 moves a piece of O00, O01, O02, O03, O04 or O06. Placements preserve orbits, so words built from them never reach those six orbits.
- **A finite search (proved; outcome not given).** Counting, per orbit and per placement, the pieces where S(X) and σ(S(Y)) meet is exact; a single overlap with both positions moved gives a pure three-cycle. One certificate per orbit proves a bound for that orbit; a failure only rules out the family. The integrator's searches S1 and S2 below are of this kind, over all 122 spine nodes.

### M2-02: lower bounds (proved, model review; the integer checks re-run by the integrator)

- **Caps that commute (computed).** Each cap region holds 3,097 pieces, and the stickers its two primitives move lie on 3,096 of them. Each region is disjoint from 543 others and meets 56. Caps with disjoint supports commute; there are 162,900 such pairs.
- **The counting group.** Let P be the group generated by the 600 cap groups A4 with only these relations: each cap's own, and commutation of caps with disjoint supports. P maps onto G, so a ball of radius r in G is no larger than the ball of radius r in P.
- **Its growth series.** Let Γ be the overlap graph of the caps (maximum degree 56), and f(z) = 3z + 4z² + 4z³ for primitives or f(z) = 11z for cap twists. Reduced words in P are unique up to exchanging commuting syllables, and their cost is the word length. Their generating series H satisfies H(z) · I_Γ(u) = 1 with u = f(z)/(1 + f(z)), where I_Γ(u) = Σ (−u)^|J| over the independent sets J of Γ.
- **A bound on it.** For 0 < x < 1 and u ≤ x(1 − x)^56, deleting the 600 vertices one at a time gives I_Γ(u) ≥ (1 − x)^600 > 0, so H converges there and H(z) ≤ (1 − x)^−600. With x = 1/57:
  - |Ball_L(r)| ≤ 40,936 · 460^r for primitives;
  - |Ball_cap(r)| ≤ 40,936 · 1,679^r for cap twists.
- **The bounds** (exact integer comparisons with |G|):

  | Measure | Some input needs at least | All but a fraction 10⁻⁶ of uniform inputs need at least | All but 10⁻¹² |
  | --- | ---: | ---: | ---: |
  | Primitives L | 224,601 | 224,599 | 224,596 |
  | Cap twists | 185,441 | 185,439 | 185,438 |
  | Actions A, fixed library of m = 1 macro | 145,909 | 145,908 | 145,906 |
  | Actions A, m = 35 | 110,495 | 110,494 | 110,493 |
  | Actions A, m = 70 | 104,774 | 104,773 | 104,772 |

  - These are lower bounds, not estimates of a typical length.
  - For actions, each macro has 7,200 placements, an inverse counts as a separate macro, and the cap blocks between macro applications are reduced as above. Creation costs are excluded.
  - The library must be fixed in advance. If it may depend on the input, no bound on A alone holds: one macro can be the whole solution.
- **Scope (integrator's note).** The bounds count words in the 1,200 retained primitives, which is what the retained phase uses. A lattice phase that leaves the lattice through jumble twists and returns is not covered.
- They supersede the integrator's rough count in the packet (free reduction only: about 1.8 × 10^5 primitives and 1.6 × 10^5 cap twists).

### M2-03: frame elements (proved, model review; computed, re-run by the integrator)

- **Which singles exist.** A pure element that fixes every position and changes only the frame at C, by a value v, has invariant equal to the class of v in H_o^ab. Since G = ∏ ker ν_o, v must lie in the derived group H_o′. So the seven C2 and the two C5 orbits have only the identity single, O06 has the five values of D5′ = C5, and O34 has all 60 values of A5.
- **Pairs.** Let c be a pure cycle of length ℓ aligned with the ordered fibres (f_A → f_B → f_C → f_A), and g_x a return word of length h_x that fixes both buffers and returns C with frame x. Then Q_x = g_x⁻¹ c g_x c⁻¹ has value x at B and x⁻¹ at C, fixes everything else and has length at most 2ℓ + 2h_x. Conjugated by a star it moves to any X ≠ C, at most 4ℓ + 2h_x + 4|u_X| letters.
- **A shorter single.** For z = xy, S_C(x, y) = Q_x Q_y Q_z⁻¹ has value 1 at B and x⁻¹y⁻¹xy at C. Two seed occurrences cancel, so its length is at most 4ℓ + 2(h_x + h_y + h_xy), shorter than a commutator of two transported pairs. Every admissible single is one such commutator.
- **The constants per orbit (computed from the execution trees; h_o is the longest chosen return, d_o the tree depth):**

  | Orbit | H_o | h_o | d_o | pair at B | pair at X ≠ C | single at C |
  | --- | --- | ---: | ---: | --- | --- | --- |
  | O00 | C2 | 5 | 20 | 2ℓ + 10 | 4ℓ + 90 | identity only |
  | O03 | C2 | 3 | 10 | 2ℓ + 6 | 4ℓ + 46 | identity only |
  | O06 | D5 | 4 | 9 | 2ℓ + 8 | 4ℓ + 44 | 4ℓ + 16 |
  | O08 | C2 | 3 | 7 | 2ℓ + 6 | 4ℓ + 34 | identity only |
  | O11 | C2 | 3 | 7 | 2ℓ + 6 | 4ℓ + 34 | identity only |
  | O17 | C5 | 2 | 6 | 2ℓ + 4 | 4ℓ + 28 | identity only |
  | O22 | C2 | 3 | 6 | 2ℓ + 6 | 4ℓ + 30 | identity only |
  | O25 | C2 | 2 | 6 | 2ℓ + 4 | 4ℓ + 28 | identity only |
  | O28 | C5 | 2 | 6 | 2ℓ + 4 | 4ℓ + 28 | identity only |
  | O32 | C2 | 2 | 6 | 2ℓ + 4 | 4ℓ + 28 | identity only |
  | O34 | A5 | 3 | 5 | 2ℓ + 6 | 4ℓ + 26 | 4ℓ + 10 |

- **These constants need c aligned with the existing fibres.** A short pure cycle on another triple, or one that only satisfies c³ = 1, needs an alignment certificate first.
- **A correction to stage 03.** Its report says that products of pairs on two positions always carry conjugate values of equal order (section 2.1, the A5 case). That is false: Q_x Q_y Q_xy⁻¹ has value 1 at B and a commutator at C. The certificates of stage 03 do not depend on the claim.

### M2-04: a cascade bound of 356,565,166 twists (proved, model review; computed, re-run by the integrator)

- **Trivial frame passes vanish.** When H_o is trivial, a piece at its home position carries the identity frame, so the orbit is solved when its position pass ends. AU-05's formula without the frame terms of the 24 such orbits gives 771,844,512 twists for the committed cascade.
- **The exact position loop.** For an orbit's position permutation, let m be the number of moved positions, k the number of nontrivial cycles, and i = 1 if A is moved, else 0.
  - The loop runs exactly (m + k − 2i)/2 three-cycles: each step lowers m + k − 2i by exactly two, whichever unsolved position it picks.
  - With b the number of steps that involve B, it uses exactly S = m + k − 2i − b stars, and S ≤ 3N_o/2 − 3. The bound is attained (N_o/2 disjoint transpositions, A paired with B).
- **The construction.** Use the committed seeds (s_o = seed length plus twice the relocation length), the exact position count, and the frame recipe of M2-03 (pairs at every X ≠ C, B apart, and the shorter single). Then:
  - every input needs at most **356,565,166** twists;
  - for an input uniform on G, the expected length is at most 231,334,003.

  This is a proved construction, not the length of the unchanged executable. It excludes the corrections (at most 25,852 twists).
- **Actions.**
  - A fixed library of the 35 relocated seeds and their inverses (m = 70), with identity placements only: A ≤ 4,840,993 and E[A] ≤ 3,098,745. Creating it costs 128,076 primitives.
  - A library of every star, frame pair and admissible single (m = 400,760): A ≤ 291,186 and E[A] ≤ 191,121, with a creation cost of at most 869,912,422 primitives.
- **Conditional.** If pure seeds aligned with the existing fibres have at most 200 primitives each, the same trees give L ≤ 78,609,696 and E[L] ≤ 49,764,387, with a creation cost of 14,000 for the 70 macros. Q1 has not established this.

### M2-05: what the measured runs show (proved, model review, from the source)

- **Factors, not stars.** The per-orbit counts of the stage 03 runs (190,925 and 191,093 in total) count every factor: stars, frame pairs and singles. A C2 orbit with 3,600 positions has about 5,400 factors, of which about 1,800 are frame pairs.
- **The `uniform` inputs are not uniform on G.** The generator draws positions uniformly, and frame values uniformly except the last one, which it sets to a fixed representative of the required class. On O06 and O34 a class has 5 and 60 elements, so the inputs cover 1/300 of G. The two restorations are valid examples, not uniform samples.

### M2-06: setups and the cascade order (proved, model review; computed, re-run by the integrator)

- **The packet's setup remark was wrong.** For u_X = pa and u_Y = pb, the middle word u_X u_Y⁻¹ is p a b⁻¹ p⁻¹: a common prefix does not cancel. In the tree of O00, nodes 5 and 6 have paths [37, 103] and [37, 104], at tree distance two, and the middle word is [37, 103, −104, −37]. For a sequence of stars, a valid setup bound is |u_X1| + |u_Xr| + Σ_j |red(u_Xj u_X(j+1)⁻¹)| plus the seed lengths; free cancellation removes common suffixes.
- **Placements do not supply the stars.** A master three-cycle gives at most 3 · 7200/N_o placed cycles of the form (A B X). On an orbit with 7,200 positions that is three of the 7,198 stars needed, so setups stay essential.
- **Shorter spine prefixes need alignment (computed).** The shortest spine prefix that the committed order allows is, on every orbit, a pure three-cycle of that orbit (cube identity, exactly three positions moved). On 13 orbits it is shorter than the seed but differs from the seed and from its inverse: (orbit, formula length) = (6, 4), (0, 560), (17, 10), (15, 94), (2, 286), (22, 22), (21, 46), (8, 94), (9, 190), (34, 190), (28, 1,534), (16, 70), (3, 562). None can replace its seed without a new alignment certificate.
- **A better order (open).** With certified candidates (own-orbit action, relocation, setups, collateral), choosing an order is a finite problem: a candidate for orbit o is legal after a set S of solved orbits when its collateral avoids S. One feasible order with verified costs would prove an improvement; none is established. Pure aligned elements remove the ordering constraint.

### S1: short pure elements by single overlap (computed by the integrator; words checked independently)

- **The family.**
  - X and Y run over the 122 distinct spine nodes of the atlas seeds (formula lengths 4 to 12,286).
  - Y is placed by every σ ∈ K⁺, 7,200 placements. Y^σ is written as the transported word: each letter becomes a shortest word in the A4 of its image cap.
  - A triple (X, Y, σ) qualifies when supp X and supp Y^σ meet in exactly one piece q. The support counts every piece with a sticker moved, frame changes in place included, as M2-01 requires. There are 530,387 such triples.
  - By the single-overlap lemma, [X, Y^σ] then moves pieces of q's orbit only:
    - a three-cycle of positions when X and Y^σ both move q to another position, pure by M2-01 (no recorded three-cycle is twisted);
    - a pair twist when exactly one of them keeps q at its position;
    - a single twist when both do.
- **The search** (`scripts/short-cycles/short_cycles.py`, about 6 minutes).
  - It ranks the triples by the exact length of the commutator.
  - It verifies the six shortest per orbit and kind exactly:
    - the transported word's permutation equals the conjugate;
    - the support and the kind are as predicted;
    - C³ = 1 on all 259,800 slots for a pure three-cycle.
  - `check_cycles.py` re-checks every recorded word from the primitives alone: 32 records, `all_ok`.
  - The exact action of K⁺ on caps, pieces and slots is `kplus.py`: K⁺ has order 7,200, and all 1,200 primitives are equivariant (0 mismatches).
  - One Codex review at the fast tier found no blocking issue.
- **The result** (`data/short-cycles.json.gz`):
  - *Coverage.* Pure three-cycles for 29 of the 35 orbits, from 28 to 18,496 primitives. Each moves three pieces of its orbit and nothing else.
  - *Shorter than the seed* for 21 orbits, by up to a factor of 16: O11 goes from 766 to 48, O12 from 766 to 52, O04 from 546 to 52, O14 from 1,534 to 144.
  - *Longer than the seed* for 8 orbits, among them O29 (18,496 against 9,214) and O34 (12,416 against 12,286).
  - *None* for O00, O02, O06, O15, O17 and O28. Their seeds carry the most collateral: O00 moves 1,272 slots in 32 orbits, O06 1,134 slots in 27, O17 720 in 19, O15 647 in 22 and O02 195 in 19.
  - *Frame elements:* for O06 (H = D5), a pair twist of 556 primitives and a single twist of 1,104 (order 5); for O32 (C2), a pair twist of 216.
- **What it changes.**
  - A seed with collateral can be used only in the cascade's order. A pure element can be used in any order.
  - To replace a seed in the cascade, a found element must also be aligned (M2-01, M2-03): relocated onto the buffers (A_o, B_o, C_o), with the relocation charged in the length, and agreeing with the ordered fibres there. No alignment is certified yet; on the 24 orbits with trivial H_o the fibres impose nothing, but the relocation is still needed.
  - Weight each orbit by its number of factors in the run uniform:1 of stage 03 (stars and frame factors, M2-05), and replace each seed by the shorter of the two at the seed's relocation and setup cost. Then the cascade would be about a third shorter: about 1.24 × 10^8 twists instead of 1.81 × 10^8. This is an estimate that assumes alignment at no extra cost, not a run.
  - The three largest terms are O15, O29 and O02, about 7.2 × 10^7 together, and none of them is improved.

### S2: two levels of overlap (computed by the integrator; words checked independently)

- **The idea.** If supp X and supp Y meet in a set I of pieces, [X, Y] moves only pieces of I, X⁻¹(I) and Y⁻¹(I), so at most 3|I| pieces. A commutator with a small overlap is therefore an element of small support, and a second commutator can isolate one piece of it.
- **The search** (`scripts/short-cycles/short_cycles2.py`, about 10 minutes per run). It is limited to the target orbits.
  - *Level 1.* Z = [X, Y^σ] for spine nodes X and Y and σ ∈ K⁺ whose supports meet in 2 to k_max pieces, at least one of them in a target orbit. For each target orbit, the 12 shortest such Z are computed exactly. Those that move a piece of a target orbit are kept.
  - *Level 2.* C = [Z, W^τ], where W is a spine node or a kept Z and τ ∈ K⁺, whose supports meet in exactly one piece of a target orbit. C is then a three-cycle, a pair twist or a single twist in that orbit. C is verified exactly as in S1 and re-checked by `check_cycles.py`.
  - One Codex review at the fast tier found no issue.
- **The run with k_max = 3** (targets O00, O02, O06, O15, O17, O28 and O29; `data/short-cycles-2.json.gz`, 7 records, `all_ok`).
  - Level 1 kept 50 elements Z, each moving 6 to 9 pieces, of 56 to 14,328 primitives.
  - Level 2 gave pure three-cycles for O00 (4,464 primitives), O02 (1,200), O06 (1,068), O28 (8,392) and O29 (140).
    - O29 drops from 9,214 (its seed) to 140.
    - O02 drops from 9,214 to 1,200.
  - It also gave a pair twist of 1,412 and a single twist of 2,428 on O06, both longer than those of S1.
  - Level 1 found no candidate for O15 or O17: no two spine nodes, in any placement, overlap in two or three pieces that include a piece of these orbits.
- **The run for O15 and O17 with k_max = 8** writes an empty result. Level 1 kept 12 elements Z for O15, each moving 21 or 24 pieces, of 184 to 592 primitives. It found none for O17: no two spine nodes, in any placement, overlap in two to eight pieces that include a piece of O17. Level 2 found no single overlap in O15 or O17. This is a bounded search (12 Z per orbit, spine nodes only), not an obstruction.
- **Together** (S1 and S2):
  - pure three-cycles for 33 of the 35 orbits, 23 of them shorter than the seed;
  - weighted as in S1, with the shorter of seed and found element per orbit, the cascade estimate drops from about 1.81 × 10^8 to about 8.3 × 10^7 twists (82,998,956), 54% less. Like the S1 figure, it assumes alignment at no extra cost; it is an estimate, not a run.
  - The largest remaining term is O15, about 2.8 × 10^7 (a third of the total): its seed of 11,564 primitives has no pure replacement. O17 (seed 1,534) adds about 4 × 10^6.

Per orbit, in primitives (S1 and S2 together; "-" means none found; the ratio uses the shorter pure three-cycle; frame elements are the shortest of either search):

| Orbit | H_o | seed | pure three-cycle, S1 | pure three-cycle, S2 | seed / shortest | pair twist | single twist |
|---|---|---:|---:|---:|---:|---:|---:|
| O34 | A5 | 12,286 | 12,416 | - | 0.99 | - | - |
| O15 | 1 | 11,564 | - | - | - | - | - |
| O02 | 1 | 9,214 | - | 1,200 | 7.7 | - | - |
| O29 | 1 | 9,214 | 18,496 | 140 | 66 | - | - |
| O28 | C5 | 3,070 | - | 8,392 | 0.37 | - | - |
| O01 | 1 | 2,190 | 592 | - | 3.7 | - | - |
| O00 | C2 | 2,182 | - | 4,464 | 0.49 | - | - |
| O08 | C2 | 1,534 | 336 | - | 4.6 | - | - |
| O14 | 1 | 1,534 | 144 | - | 11 | - | - |
| O17 | C5 | 1,534 | - | - | - | - | - |
| O18 | 1 | 1,534 | 232 | - | 6.6 | - | - |
| O03 | C2 | 1,094 | 700 | - | 1.6 | - | - |
| O10 | 1 | 766 | 1,600 | - | 0.48 | - | - |
| O11 | C2 | 766 | 48 | - | 16 | - | - |
| O12 | 1 | 766 | 52 | - | 15 | - | - |
| O22 | C2 | 766 | 472 | - | 1.6 | - | - |
| O04 | 1 | 546 | 52 | - | 10 | - | - |
| O05 | 1 | 546 | 148 | - | 3.7 | - | - |
| O06 | D5 | 382 | - | 1,068 | 0.36 | 556 | 1,104 |
| O09 | 1 | 382 | 496 | - | 0.77 | - | - |
| O07 | 1 | 286 | 60 | - | 4.8 | - | - |
| O16 | 1 | 286 | 88 | - | 3.2 | - | - |
| O13 | 1 | 142 | 48 | - | 3 | - | - |
| O19 | 1 | 142 | 96 | - | 1.5 | - | - |
| O20 | 1 | 142 | 28 | - | 5.1 | - | - |
| O31 | 1 | 142 | 40 | - | 3.5 | - | - |
| O21 | 1 | 94 | 108 | - | 0.87 | - | - |
| O27 | 1 | 94 | 56 | - | 1.7 | - | - |
| O30 | 1 | 94 | 40 | - | 2.4 | - | - |
| O32 | C2 | 94 | 56 | - | 1.7 | 216 | - |
| O23 | 1 | 46 | 68 | - | 0.68 | - | - |
| O24 | 1 | 46 | 88 | - | 0.52 | - | - |
| O25 | C2 | 46 | 136 | - | 0.34 | - | - |
| O26 | 1 | 46 | 40 | - | 1.1 | - | - |
| O33 | 1 | 46 | 36 | - | 1.3 | - | - |

### Where M2 stands after round 6

- **Lower bounds (proved).** Some input needs at least 224,601 retained twists, about 2.6 days at one twist per second. With a fixed library of 70 macros it needs at least 104,774 actions (M2-02).
- **Upper bounds (proved).** The construction of M2-04 needs at most 356,565,166 twists for every input, about 11 years at one per second. A person who replays the 35 relocated seeds and their inverses from the program needs at most 4,840,993 actions, about 56 days at one action per second; the expectation for an input uniform on G is at most 3,098,745 actions, about 36 days. The bound for the committed executable is 771,844,512 twists (AU-05 without the trivial frame terms).
- **The gap** between them is a factor of about 1,600 in twists and about 46 in actions.
- **Short pure elements (computed, S1 and S2).** Pure three-cycles exist for 33 of the 35 orbits, 21 of them within 200 primitives. None is aligned with the cascade yet, and O15 and O17 have none.
- **Next experiments (not run).**
  - Alignment certificates for the found three-cycles: a relocation onto the buffers and agreement with the ordered fibres. On the aligned orbits, the conditional bound of M2-04 then applies.
  - A search for O15 and O17 beyond the spine-node families.
  - The sampler fix of M2-05, with stars, frame pairs and singles counted apart.
  - One feasible cascade order with certified shorter prefixes (M2-06).

The commands of the round, as re-run by the integrator (`python3 -I -B <file>` from the repository root, a few seconds each). They are the reviewer's, unchanged.

M2-01: the rotation group, the local distance profiles, the short subnodes and the spine prefixes the committed order allows (about 17 s). Output:

```text
K 7200 all rp/fp in K True True fp0 True
local distance profiles {((0, 1), (1, 3), (2, 4), (3, 4)): 600}
short 119 {1: 81, 4: 5, 10: 7, 16: 2, 22: 9, 34: 2, 46: 13} small 20 small absent orbits [0, 1, 2, 3, 4, 6]
all35 allowed prefixes: cube identity and exactly3 positions moved; differ from seed and inverse: [(6, 4), (0, 560), (17, 10), (15, 94), (2, 286), (22, 22), (21, 46), (8, 94), (9, 190), (34, 190), (28, 1534), (16, 70), (3, 562)]
```

```python
import json,collections,numpy as np
from pathlib import Path
z=np.load('assets/model.npz');pr=np.load('assets/primitives.npz');rp=z['rotperms'];fp=z['frameperms'];sp=z['slot_piece'];oid=z['orbit_id'];so=oid[sp];I=np.arange(len(sp),dtype=np.int32);E=np.arange(600,dtype=np.int16)
G={E.tobytes():E};Q=[E]
for x in Q:
 for g in rp[:4]:
  y=g[x];k=y.tobytes()
  if k not in G:G[k]=y;Q.append(y)
print('K',len(G),'all rp/fp in K',all(x.tobytes() in G for x in rp),all(x.tobytes() in G for x in fp),'fp0',np.array_equal(fp[:,0],np.arange(600)))
profiles=collections.Counter()
for c in range(600):
 H,T=rp[2*c:2*c+2];D={E.tobytes():0};Q=[E]
 for x in Q:
  for g in (H,T,T[T]):
   y=g[x];k=y.tobytes()
   if k not in D:D[k]=D[x.tobytes()]+1;Q.append(y)
 profiles[tuple(sorted(collections.Counter(D.values()).items()))]+=1
print('local distance profiles',dict(profiles))
cache={}
def ev(t):
 k=json.dumps(t,separators=(',',':'))
 if k in cache:return cache[k]
 if t[0]=='g':
  a=t[1];i=abs(a)-1;u,v=pr['offsets'][i:i+2];s=pr['src'][u:v];d=pr['dst'][u:v];P=I.copy();P[s if a>0 else d]=d if a>0 else s;L=1
 elif t[0]=='comm':
  X,lx=ev(t[1]);Y,ly=ev(t[2]);P=np.argsort(Y).astype(np.int32)[np.argsort(X).astype(np.int32)[Y[X]]];L=2*(lx+ly)
 else:
  X,lx=ev(t[1]);P=I.copy();xx=X if t[2]>=0 else np.argsort(X)
  for _ in range(abs(t[2])):P=xx[P]
  L=abs(t[2])*lx
 cache[k]=(P,L);return P,L
at=json.loads(Path('assets/seed_atlas.json').read_text())
for a in at:ev(a['ast'])
short=[(P,L) for P,L in cache.values() if L<=50];small=[(P,L) for P,L in short if len(set(sp[P!=I]))<=600]
print('short',len(short),dict(sorted(collections.Counter(L for P,L in short).items())),'small',len(small),'small absent orbits',sorted(set(range(35))-set(int(o) for P,L in small for o in so[P!=I])))
order=[6,0,17,15,2,22,21,8,23,9,29,25,13,11,10,34,28,26,24,16,14,12,3,33,32,31,30,27,20,19,18,7,5,4,1];earlier=set();changed=[]
for o in order:
 final,_=ev(at[o]['ast']);t=at[o]['ast'];cands=[];ix=np.flatnonzero(so==o)
 while True:
  P,L=ev(t);S=np.unique(sp[P!=I]);own=S[oid[S]==o]
  if len(own)==3 and not set(map(int,oid[S])).intersection(earlier):cands.append((L,P))
  if t[0]=='g':break
  t=t[1]
 L,P=min(cands,key=lambda c:c[0]);assert np.array_equal(P[P[P[ix]]],ix);assert len(np.unique(sp[ix[sp[P[ix]]!=sp[ix]]]))==3
 if not np.array_equal(P[ix],final[ix]):
  assert not np.array_equal(P[ix],np.argsort(final)[ix]);changed.append((o,L))
 earlier.add(o)
print('all35 allowed prefixes: cube identity and exactly3 positions moved; differ from seed and inverse:',changed)
```

M2-02: cap supports and regions, and their disjointness graph. Output:

```text
support_piece_sizes Counter({3096: 600})
disjoint_piece_degrees Counter({543: 600}) edges 162900 examples [[0, 22], [0, 28], [0, 31]]
region_sizes Counter({3097: 600})
region_disjoint_degrees Counter({543: 600}) edges 162900
support_vs_region Counter({1: 600})
```

```python
import numpy as np,collections
m=np.load('assets/model.npz');p=np.load('assets/primitives.npz');sp=m['slot_piece'];src=p['src'];off=p['offsets']
def bits(indices):
 b=np.zeros(177120,dtype=np.uint8);b[indices]=1
 return int.from_bytes(np.packbits(b).tobytes(),'little')
ps=[];sizes=[]
for c in range(600):
 ix=np.unique(sp[src[off[2*c]:off[2*c+2]]]);ps.append(bits(ix));sizes.append(len(ix))
D=np.array([[a!=b and (ps[a]&ps[b])==0 for b in range(600)] for a in range(600)])
print('support_piece_sizes',collections.Counter(sizes))
print('disjoint_piece_degrees',collections.Counter(D.sum(1).tolist()),'edges',int(D.sum())//2,'examples',np.argwhere(np.triu(D,1))[:3].tolist())
maskoff=m['mask_offsets'];maskval=m['mask_values'];memberships=np.repeat(np.arange(177120),np.diff(maskoff));regions=[];rsize=[]
for c in range(600):
 ix=memberships[maskval==c];regions.append(bits(ix));rsize.append(len(ix))
R=np.array([[a!=b and (regions[a]&regions[b])==0 for b in range(600)] for a in range(600)])
print('region_sizes',collections.Counter(rsize))
print('region_disjoint_degrees',collections.Counter(R.sum(1).tolist()),'edges',int(R.sum())//2)
print('support_vs_region',collections.Counter((regions[c]&~ps[c]).bit_count() for c in range(600)))
```

M2-02: the local distance profiles and the twist and cap-twist bounds against the exact |G|. Output:

```text
all_cap_Cayley_distance_histograms Counter({((0, 1), (1, 3), (2, 4), (3, 4)): 600})
primitive lambda 460 tail10^-k 0 r_max 224600 distance_at_least 224601
primitive lambda 460 tail10^-k 6 r_max 224598 distance_at_least 224599
primitive lambda 460 tail10^-k 12 r_max 224595 distance_at_least 224596
cap lambda 1679 tail10^-k 0 r_max 185440 distance_at_least 185441
cap lambda 1679 tail10^-k 6 r_max 185438 distance_at_least 185439
cap lambda 1679 tail10^-k 12 r_max 185437 distance_at_least 185438
N_sum 176520 G_decimal_digits_exact 598061
```

```python
import numpy as np,collections,math
from fractions import Fraction
m=np.load('assets/model.npz');p=np.load('assets/primitives.npz');src=p['src'];dst=p['dst'];off=p['offsets'];hist=collections.Counter()
for c in range(600):
 support=np.unique(src[off[2*c]:off[2*c+2]])
 ident=np.arange(len(support),dtype=np.int16);gen=[]
 for a in [2*c,2*c+1]:
  v=ident.copy();v[np.searchsorted(support,src[off[a]:off[a+1]])]=np.searchsorted(support,dst[off[a]:off[a+1]])
  assert np.array_equal(np.sort(v),ident);gen.append(v)
 assert np.array_equal(gen[0][gen[0]],ident)
 assert np.array_equal(gen[1][gen[1][gen[1]]],ident)
 gen.append(np.argsort(gen[1]).astype(np.int16))
 seen={ident.tobytes():0};queue=[ident]
 for v in queue:
  d=seen[v.tobytes()]
  for g in gen:
   w=g[v];key=w.tobytes()
   if key not in seen:seen[key]=d+1;queue.append(w)
 hist[tuple(sorted(collections.Counter(seen.values()).items()))]+=1
print('all_cap_Cayley_distance_histograms',hist)
N=np.bincount(m['orbit_id'][m['orbit_id']>=0],minlength=35).tolist()
orders={0:(2,2),3:(2,2),8:(2,2),11:(2,2),22:(2,2),25:(2,2),32:(2,2),6:(10,2),17:(5,5),28:(5,5),34:(60,1)}
G=1
for o,n in enumerate(N):
 h,ab=orders.get(o,(1,1));G*=math.factorial(n)*h**n//(2*ab)
assert 10**598060<=G<10**598061
p0=Fraction(56**56,57**57)
assert Fraction(57,56)**600<40936
for label,lam,f in [('primitive',460,lambda z:3*z+4*z*z+4*z*z*z),('cap',1679,lambda z:11*z)]:
 z=Fraction(1,lam);assert f(z)/(1+f(z))<=p0
 for k in [0,6,12]:
  lo=0;hi=300000
  while hi-lo>1:
   mid=(lo+hi)//2
   if 10**k*40936*lam**mid<G:lo=mid
   else:hi=mid
  assert 10**k*40936*lam**lo<G<=10**k*40936*lam**hi
  print(label,'lambda',lam,'tail10^-k',k,'r_max',lo,'distance_at_least',lo+1)
print('N_sum',sum(N),'G_decimal_digits_exact',598061)
```

M2-02: the action bounds for m = 1, 35 and 70. Output:

```text
m 1 tail10^-k 0 max_r 145908 A_at_least 145909
m 1 tail10^-k 6 max_r 145907 A_at_least 145908
m 1 tail10^-k 12 max_r 145905 A_at_least 145906
m 35 tail10^-k 0 max_r 110494 A_at_least 110495
m 35 tail10^-k 6 max_r 110493 A_at_least 110494
m 35 tail10^-k 12 max_r 110492 A_at_least 110493
m 70 tail10^-k 0 max_r 104773 A_at_least 104774
m 70 tail10^-k 6 max_r 104772 A_at_least 104773
m 70 tail10^-k 12 max_r 104771 A_at_least 104772
```

```python
import numpy as np,math
from fractions import Fraction
m=np.load('assets/model.npz');N=np.bincount(m['orbit_id'][m['orbit_id']>=0],minlength=35).tolist()
orders={0:(2,2),3:(2,2),8:(2,2),11:(2,2),22:(2,2),25:(2,2),32:(2,2),6:(10,2),17:(5,5),28:(5,5),34:(60,1)}
G=1
for o,n in enumerate(N):
 h,ab=orders.get(o,(1,1));G*=math.factorial(n)*h**n//(2*ab)
for macros,lam,k,C in [(1,12556,1082,1308),(35,258543,23444,25981),(70,510576,46357,50492)]:
 M=7200*macros;x=Fraction(1,k);z=Fraction(1,lam)
 assert 11*z/(1+11*z)<=x*(1-x)**56
 assert (1-x)**600-M*z>=Fraction(1,C)
 for tail in [0,6,12]:
  r=int((598060-tail-math.log10(C))/math.log10(lam))
  while 10**tail*C*lam**r>=G:r-=1
  while 10**tail*C*lam**(r+1)<G:r+=1
  assert 10**tail*C*lam**r<G<=10**tail*C*lam**(r+1)
  print('m',macros,'tail10^-k',tail,'max_r',r,'A_at_least',r+1)
```

M2-03: return words, derived groups and the frame constants. Output:

```text
{'o': 0, 'H': 2, 'derived': 1, 'all_derived_single_commutator': True, 'return_depth_max': 5, 'tree_depth_max': 20, 'pair_B_bound': '2*l+10', 'pair_any_bound': '4*l+90', 'single_C_bound': '0', 'identity_only_single': True}
{'o': 3, 'H': 2, 'derived': 1, 'all_derived_single_commutator': True, 'return_depth_max': 3, 'tree_depth_max': 10, 'pair_B_bound': '2*l+6', 'pair_any_bound': '4*l+46', 'single_C_bound': '0', 'identity_only_single': True}
{'o': 6, 'H': 10, 'derived': 5, 'all_derived_single_commutator': True, 'return_depth_max': 4, 'tree_depth_max': 9, 'pair_B_bound': '2*l+8', 'pair_any_bound': '4*l+44', 'single_C_bound': '4*l+16', 'identity_only_single': False}
{'example_v': (3, 0, 1, 4, 2), 'example_x_y_xy': ((4, 2, 1, 3, 0), (0, 3, 4, 1, 2), (2, 4, 3, 1, 0)), 'loop_lengths': [2, 3, 3], 'additional_constant': 16}
{'o': 8, 'H': 2, 'derived': 1, 'all_derived_single_commutator': True, 'return_depth_max': 3, 'tree_depth_max': 7, 'pair_B_bound': '2*l+6', 'pair_any_bound': '4*l+34', 'single_C_bound': '0', 'identity_only_single': True}
{'o': 11, 'H': 2, 'derived': 1, 'all_derived_single_commutator': True, 'return_depth_max': 3, 'tree_depth_max': 7, 'pair_B_bound': '2*l+6', 'pair_any_bound': '4*l+34', 'single_C_bound': '0', 'identity_only_single': True}
{'o': 17, 'H': 5, 'derived': 1, 'all_derived_single_commutator': True, 'return_depth_max': 2, 'tree_depth_max': 6, 'pair_B_bound': '2*l+4', 'pair_any_bound': '4*l+28', 'single_C_bound': '0', 'identity_only_single': True}
{'o': 22, 'H': 2, 'derived': 1, 'all_derived_single_commutator': True, 'return_depth_max': 3, 'tree_depth_max': 6, 'pair_B_bound': '2*l+6', 'pair_any_bound': '4*l+30', 'single_C_bound': '0', 'identity_only_single': True}
{'o': 25, 'H': 2, 'derived': 1, 'all_derived_single_commutator': True, 'return_depth_max': 2, 'tree_depth_max': 6, 'pair_B_bound': '2*l+4', 'pair_any_bound': '4*l+28', 'single_C_bound': '0', 'identity_only_single': True}
{'o': 28, 'H': 5, 'derived': 1, 'all_derived_single_commutator': True, 'return_depth_max': 2, 'tree_depth_max': 6, 'pair_B_bound': '2*l+4', 'pair_any_bound': '4*l+28', 'single_C_bound': '0', 'identity_only_single': True}
{'o': 32, 'H': 2, 'derived': 1, 'all_derived_single_commutator': True, 'return_depth_max': 2, 'tree_depth_max': 6, 'pair_B_bound': '2*l+4', 'pair_any_bound': '4*l+28', 'single_C_bound': '0', 'identity_only_single': True}
{'o': 34, 'H': 60, 'derived': 60, 'all_derived_single_commutator': True, 'return_depth_max': 3, 'tree_depth_max': 5, 'pair_B_bound': '2*l+6', 'pair_any_bound': '4*l+26', 'single_C_bound': '4*l+10', 'identity_only_single': False}
{'example_v': (4, 7, 14, 9, 5, 1, 17, 0, 16, 10, 2, 8, 19, 18, 3, 12, 6, 11, 15, 13), 'example_x_y_xy': ((7, 14, 17, 4, 16, 5, 9, 10, 18, 19, 0, 1, 12, 8, 11, 2, 3, 15, 13, 6), (17, 9, 4, 14, 8, 18, 7, 12, 1, 2, 19, 16, 0, 5, 13, 10, 15, 6, 3, 11), (12, 13, 6, 8, 15, 18, 2, 19, 3, 11, 17, 9, 0, 1, 16, 4, 14, 10, 5, 7)), 'loop_lengths': [1, 2, 2], 'additional_constant': 10}
```

```python
import gzip,json
from pathlib import Path
import numpy as np
T=json.loads(gzip.decompress(Path('assets/execution_trees.json.gz').read_bytes()))
A=json.loads(Path('assets/seed_atlas.json').read_text())
with np.load('assets/primitives.npz',allow_pickle=False) as z:src,dst,off=z['src'],z['dst'],z['offsets']
def inv(p):
 r=[0]*len(p)
 for i,j in enumerate(p):r[j]=i
 return tuple(r)
def mul(p,q):return tuple(q[i] for i in p)
def path(t,n):
 w=[]
 while t['parent'][n]>=0:w.append(int(t['move'][n]));n=t['parent'][n]
 return w[::-1]
def follow(sl,w):
 out=np.array(sl,copy=True)
 for m in w:
  a,b=off[abs(m)-1:abs(m)+1];s,d=src[a:b],dst[a:b]
  if m<0:s,d=d,s
  order=np.argsort(s);s,d=s[order],d[order]
  idx=np.minimum(np.searchsorted(s,out),len(s)-1)
  hit=s[idx]==out;out[hit]=d[idx[hit]]
 return out.tolist()
for o in [0,3,6,8,11,17,22,25,28,32,34]:
 t,a=T[o],A[o];C=t['third'];fc=t['frames'][0];lab={v:i for i,v in enumerate(fc)};k=len(fc);e=tuple(range(k));H={};cn=[]
 for n,p in enumerate(t['positions']):
  if p!=C:continue
  w=path(t,n);assert follow(fc,w)==t['frames'][n]
  assert follow(a['frames'][0],w)==a['frames'][0]
  assert follow(a['frames'][1],w)==a['frames'][1]
  x=tuple(lab[v] for v in t['frames'][n]);assert x not in H;H[x]=len(w);cn.append(n)
 assert all(inv(x) in H for x in H)
 assert all(mul(x,y) in H for x in H for y in H)
 h={x:min(H[x],H[inv(x)]) for x in H};best={};wit={}
 for x in H:
  for y in H:
   xy=mul(x,y);v=mul(mul(inv(x),inv(y)),xy);cost=2*(h[x]+h[y]+h[xy])
   if v not in best or cost<best[v]:best[v]=cost;wit[v]=(x,y,xy)
 generated={e};change=True
 while change:
  grown=generated|{mul(a,b) for a in generated for b in best}
  change=grown!=generated;generated=grown
 assert set(best)==generated
 print({'o':o,'H':len(H),'derived':len(generated),'all_derived_single_commutator':len(best)==len(generated),'return_depth_max':max(h.values()),'tree_depth_max':max(t['depth']),'pair_B_bound':'2*l+'+str(2*max(h.values())),'pair_any_bound':'4*l+'+str(2*max(h.values())+4*max(t['depth'])),'single_C_bound':'0' if len(best)==1 else '4*l+'+str(max(best.values())),'identity_only_single':len(best)==1})
 if o in [6,34]:
  v=max(best,key=best.get)
  print({'example_v':v,'example_x_y_xy':wit[v],'loop_lengths':[h[x] for x in wit[v]],'additional_constant':best[v]})
```

M2-04: the cascade bounds, expectations, action counts and creation costs. Output:

```text
committed worst 356565166 expected_upper_ceiling 231334003 creation70 128076 full_m 400760 full_creation_bound 869912422 full_actions 291186 full_expected_ceiling 191121
compatible200 worst 78609696 expected_upper_ceiling 49764387 creation70 14000 full_m 400760 full_creation_bound 115154290 full_actions 291186 full_expected_ceiling 191121
actions70 worst 4840993 expected_upper_ceiling 3098745 creation70 70 full_m 400760 full_creation_bound 6863664 full_actions 291186 full_expected_ceiling 191121
seed_master_creation 128076
```

```python
import gzip,json
from fractions import Fraction as F
T=json.load(gzip.open('assets/execution_trees.json.gz','rt'))
H={0:2,3:2,6:10,8:2,11:2,17:5,22:2,25:2,28:5,32:2,34:60}
h={0:5,3:3,6:4,8:3,11:3,17:2,22:3,25:2,28:2,32:2,34:3}
for mode in ('committed','compatible200','actions70'):
 total=0;expect=F(0);create=0;mfull=0;createfull=0;Af=0;Ef=F(0)
 for t in T:
  o=t['orbit'];N=len(set(t['positions']))+2;s0=len(t['seed'])+2*len(t['relocation']);s=200 if mode=='compatible200' else 1 if mode=='actions70' else s0;d=max(t['depth']);w=s+2*d
  ES=F(N-5)+sum((F(1,j) for j in range(1,N+1)),F(0))+F(3,N)+F(1,N*(N-1))
  P=4*s+2*h.get(o,0)+4*d;PB=2*s+2*h.get(o,0);SC=4*s+(16 if o==6 else 10) if o in (6,34) else 0
  total+=(3*N//2-3)*w;expect+=ES*w;Af+=3*N//2-3;Ef+=ES
  if o in H:
   frame=(N-2)*P+PB+SC;total+=frame;expect+=F(H[o]-1,H[o])*((N-2)*P+PB)
   if o in (6,34):expect+=F(4,5)*SC if o==6 else F(59,60)*SC
   Af+=N-1+int(o in (6,34));Ef+=F(H[o]-1,H[o])*(N-1)+(F(4,5) if o==6 else F(59,60) if o==34 else 0)
   mfull+=(N-1)*(H[o]-1)+(4 if o==6 else 59 if o==34 else 0)
   createfull+=(H[o]-1)*((N-2)*P+PB)+(4 if o==6 else 59 if o==34 else 0)*SC
  mfull+=2*(N-2);createfull+=2*(N-2)*w;create+=2*s
 print(mode,'worst',total,'expected_upper_ceiling',(expect.numerator+expect.denominator-1)//expect.denominator,'creation70',create,'full_m',mfull,'full_creation_bound',createfull,'full_actions',Af,'full_expected_ceiling',(Ef.numerator+Ef.denominator-1)//Ef.denominator)
print('seed_master_creation',sum(2*(len(t['seed'])+2*len(t['relocation'])) for t in T))
```

M2-06: the O00 setup example and AU-05 without the trivial frame terms. Output:

```text
O00 nodes5,6 [37, 103] [37, 104] middle [37, 103, -104, -37]
AU05_without_trivial_frames 771844512
```

```python
import gzip,json
T=json.load(gzip.open('assets/execution_trees.json.gz','rt'));t=T[0]
def path(n):
 w=[]
 while t['parent'][n]>=0:w.append(t['move'][n]);n=t['parent'][n]
 return w[::-1]
u,v=path(5),path(6)
print('O00 nodes5,6',u,v,'middle',u+[-x for x in v[::-1]])
non={0,3,6,8,11,17,22,25,28,32,34}
print('AU05_without_trivial_frames',sum(4*(len(set(t['positions']))+2)*(len(t['seed'])+2*len(t['relocation'])+2*max(t['depth']))+((len(set(t['positions']))+5)*(4*(len(t['seed'])+2*len(t['relocation']))+6*max(t['depth'])) if t['orbit'] in non else 0) for t in T))
```

## 12. What changed in METHOD.md

- Section 2, step 3: the pure twist is tried on at most 8 units. Its word lands the carrier and takes the lattice pieces of X off. The cheaper powers may also permute lattice pieces outside X. Theorem C bounds the word by 720 letters; `cellpure3.py` searches and does not use that bound.
- Section 2, phase 2: Theorem R holds for reachable configurations, with the corrections applied last; the library spans A_J by the Chinese remainder theorem.
- Section 3: new rows for the existence of a clean step (M1-01), for small states (M1-03) and for the other cell kinds (LC-12, LC-13). The rows for Lemma C, Lemma U, Lemma D, Proposition GH and Lemma CM, A_J, Theorem R and the completeness of phase 1 now give the review status and the corrections.
- Section 5: gap M1 now records M1-01 to M1-15 and asks for a bounded construction from the state alone and for v4's coverage; its scope list names the transport words for triangle and tetrahedron carriers and the created partners that s61c needs. Gap E2 records the model review.
- The reproduction section says that acceptance reads `all_ok` and gives the `omega_check.py` command.
- After the round 4 audit (section 9):
  - Section 2 states Theorem M\* and says that v4 is a partial implementation of it. The lattice lengths now give the measured range, 1.5 to 2.3 × 10^8 twists, and the bound for every input, 1,324,686,034.
  - Section 3 has new rows for Theorem M\*, the toolkit theorem and the lattice-phase bound. The row for the completeness of phase 1 now concerns v4 only, and the vacancy row says that one certificate must cover all targets at once.
  - Section 5: gap M1 records the audit, the scope list names units with pieces of height four, and gap M2 uses the bound.
- After the driver change (AU-01):
  - Section 2: step 4 is the exhaustive fallback, and the outer loop checks the off count before any budget; the runs of section 6 are marked as v4 runs.
  - Section 3: the row for the completeness of the driver concerns the current driver. Section 5: gap M1 no longer asks for v4's coverage.
  - The reproduction section gives the new options and the two tests.
- After round 5 (section 10): gap M1 gains the coverage results (M1-16, M1-18, M1-19), and its V-existence item gives the bound min(Q_T, Q_H) − 1 of M1-17.
- After round 6 (section 11):
  - Section 2, phase 2: the lattice-phase lengths give the construction bound of M2-04, its expectation, the executable's bound without trivial frame terms, and the lower bounds of M2-02.
  - Section 3: new rows for the single-overlap lemma (M2-01), the short pure elements (S1, S2), the lower bounds (M2-02) and the frame elements (M2-03); the row for the lattice-phase length gives the new bounds.
  - Section 5: gap M2 states the two measures, the lower and upper bounds in twists and in actions, the short pure elements and what aligning them needs, and the next experiments.
- After the v4 runs ended or were cut off: section 6.2 gives their results and records (`runs/stop62-v4/`, `runs/e2run24-v4/`), and the reproduction section gives the commands of the short-cycle searches.
