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

## 10. What changed in METHOD.md

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
