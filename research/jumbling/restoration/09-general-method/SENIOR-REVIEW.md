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

Let C be the s62m3 step (233 to 61 off pieces, 1,749 letters), S chain step 13 (233 to 86, 1,822 letters) and R chain step 14 (86 to 0, 1,374 letters). Then C⁻¹·S·R[0:831] takes s61c to the lattice in 1,749 + 1,822 + 831 = 4,402 letters:
- C⁻¹ returns s61c to the 233 state, and S and R take that state to the lattice;
- R's letters after letter 831 are all K⁺ letters, which keep every piece's lattice membership, so every piece is already on the lattice after letter 831.

The end is on the lattice but not solved: the letters after 831 are retained and keep ν, and the end of R has ν ≠ 0 (the input record of the stop62 lattice restore). So the lattice phase must still restore the stickers. (Correction, round 8, R8-03: this paragraph said "restores".)

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

## 12. Replacing the seeds (round 7)

Round 6 found short pure three-cycles for 33 orbits but used none of them: it assumed that a replacement seed must be aligned with the cascade's buffers and fibres (M2-01), and no certificate covered that. Round 7 states what the committed cascade needs from a seed. The integrator proposed four claims (H1 to H4). A Codex senior review proved them under sharper conditions (R7-01) and set out the certificates (R7-02), the bounds (R7-03) and the case of O15 and O17 (R7-04). Its four findings are `major` and verified, and the integrator adopted all four. The integrator then relocated the shorter candidates onto the buffers, certified them and re-ran the cascade with them. The runs and certificates are the integrator's; the review ran no restorer.

### R7-01: what the cascade needs from a seed (proved, model review)

Notation (chronological products, in the convention of the review):
- For an element h and a position P, h_P is the fibre map that takes the labels of P to the labels of h(P).
- A *candidate* c' is a legal retained word with three properties:
  - on orbit o it moves exactly the stickers of the three positions A, B and C, frame changes included;
  - it acts on them as the position cycle A → B → C → A, with fibre maps α = c'_A, β = c'_B and γ = c'_C;
  - every other sticker it moves lies on an orbit solved after o.
- The committed seed c_o is a candidate with α = β = γ = 1 (it is *aligned*). `restore_lattice.py` asserts this (lines 67 to 69).

The four claims hold for every candidate:
- **Positions (H1).** The position loop of `OrbitKit.solve` (stage 03 `kit.py`, line 236) reads only which piece sits at each position. Each tree path u_X fixes A and B and takes C to X. So the star u_X⁻¹ c' u_X is the position cycle (A B X), whatever the fibre maps. For the same entering positions, the loop makes the same choices with c' as with c_o. Earlier orbits leave different collateral, though, so a later orbit can enter differently.
- **Frames (H2).**
  - The setup and return paths of the committed trees fix every sticker of A and B, not only their positions.
  - Let a return word g bring C back with the value x. Then Q_x = g⁻¹ c' g c'⁻¹ has the value βxβ⁻¹ at B and x⁻¹ at C, and fixes the rest of orbit o. The return values cover H_o, so the values at B cover H_o as well.
  - A pair with the value v at B, carried by a star, has the value β⁻¹vβ at a position X outside {A, B, C}, and αvα⁻¹ at A.
  - So the committed `pair_to` works unchanged exactly when α and β lie in the centre of H_o. This always holds on the trivial, C2 and C5 groups. D5 and A5 have a trivial centre.
  - On D5 and A5 the pairs are re-indexed instead. For the value v, take the pair with the B value v at B, βvβ⁻¹ at X and α⁻¹vα at A. This adds no letters, so the pair bounds of M2-03 still hold. `ReKit.pair_to` searches for the pair whose carried value is v and checks the value exactly.
  - The single S_C(x, y) = Q_x Q_y Q_xy⁻¹ of M2-03 has the value 1 at B and x⁻¹y⁻¹xy at C, whatever α, β and γ are. The kit's own single, a commutator of two pairs, also covers H_o′.
  - A cube identity is not needed. After the pairs, the residual at C lies in H_o′, and a single removes it.
- **Collateral (H3).** Every retained word preserves each orbit. So every completed star, pair and single built from c' moves only orbits that c' moves, and the order condition is the committed one: the collateral orbits of c' must be solved after o. This holds for the completed factors, not for their prefixes. The kit's `allowed` set must also contain the certified collateral of c'.
- **Relocation (H4).** Suppose P acts on orbit o as a position three-cycle (p_i p_j p_k), and g takes p_i, p_j and p_k to A, B and C.
  - Then g⁻¹ P g acts on orbit o as a three-cycle on {A, B, C}, and its collateral lies on the same orbits as that of P.
  - It has at most |P| + 2|g| letters, and its inverse reverses the direction.
  - On these assets such a g always exists; the review's exact graph checks show this for every orbit.

The committed driver's alignment assertion is therefore stronger than needed. The round 7 runner replaces it by the conditions above and keeps the support, legality and collateral checks.

### R7-02: certificates, and what two runs show (proved, model review)

- **The certificate of a replacement** has these parts. Each is checked by composing the primitive words independently.
  - The asset and source digests.
  - The candidate word and its provenance.
  - The order of its three pieces, the relocation g and the direction.
  - The final word and its length s'.
  - The buffers and their labels, and its action on all slots.
  - α, β and γ.
  - Its collateral orbits, from every moved sticker.
- **The cascade.** Freeze the committed order, or validate a new dependency order. Take each kit's `allowed` set from the certified collateral.
- **What two runs show.** A run plus the stage 03 replay checker proves one statement: "these replacement words restore these two inputs, in these lengths". It needs the inputs to be bound to their digests. It shows nothing about every input, the bounds, shortest words, uniform sampling, performance or human replicability.
- **A gap in the checker.** For a synthetic input, `verify_restore.py` (lines 244 to 248) replays the stored start array without comparing its hash with the input digest of the summary. So the stored arrays must be hashed independently.
- **Shortest.** A greedy relocation is not provably shortest. The result is the shortest among the certified candidates.

What the integrator did:
- `realign.py` checks each replacement's action on all slots and its collateral against the loaded assets. It records the final word, the source of the candidate, the lengths of the candidate, the relocation and the final word, the flags (legal, pure, aligned fibres, cube identity on the orbit) and the collateral orbits. It does not archive the asset and source digests, the relocation word, the ordered triple or the buffer labels, and of α, β and γ it records only whether they are aligned (corrected in round 12, R12-05).
- The runner asserts the committed order and passes the certified collateral as `allowed`.
- Both stored start arrays, hashed independently as little-endian int64 bytes, equal the committed digests (`runs/realigned-cascade/commands.txt`).
- The checker's report says `pass`, with both runs restored to the identity and their lengths matching.

Not done:
- The exhaustive frame check of each replaced orbit: every value at B, at A and at other positions, and every single in H_o′. R7-01 proves both coverage statements.
- Separate counts of stars, pairs and singles in the run records.
- Archiving the asset and source digests, the relocation words, the ordered triples and the buffer labels with each replacement (R12-05).

### R7-03: the bounds with the replaced seeds (proved, model review; computed by the integrator)

- **The formula.** The M2-04 formula holds with the certified lengths s', relocation included. It bounds every input, and its expectation bounds a uniform input on G. Tree depths and return lengths do not change.
- **The kit as it runs.** The kit does not build the shorter single of M2-03. Its single at C is a commutator of a pair at B and a carried pair, of at most K_o = 2P_B + 2P = 12s + 8h + 8d twists on O06 and O34. With K_o in place of the M2-03 constants, the formula bounds the cascade as it runs.
- **With the committed seeds** (the review): the cascade as it runs needs at most 356,666,844 twists for every input (expected length at most 231,433,387). The construction needs at most 356,565,166 (expected length at most 231,334,003). The earlier bound of 771,844,512 is safe but loose.
- **With the replaced seeds** (the integrator, from the certified lengths): the construction needs at most 71,514,544 twists (expected length at most 46,189,354). The cascade as it runs needs at most 71,516,526 (expected length at most 46,191,246).
- **Actions.** With a library of the 35 seeds and their inverses, the number of actions does not depend on the seed lengths. The construction needs at most 4,840,993 actions (expected number at most 3,098,745), and the cascade as it runs at most 4,841,151 (expected number at most 3,098,883). Creating the 70 macros costs 2Σs' = 14,048 twists (128,076 with the committed seeds).
- **Scope.** The expected values are upper bounds for a uniform input on G, not exact means (corrected in round 12, R12-07; T11-07). The committed synthetic inputs are not uniform (M2-05).

The bounds with the replaced seeds (from the repository root). Output:

```text
construction twists worst 71514544 expected_upper_ceiling 46189354
construction actions worst 4840993 expected_upper_ceiling 3098745
kit twists worst 71516526 expected_upper_ceiling 46191246
kit actions worst 4841151 expected_upper_ceiling 3098883
creation70 14048
```

```python
import gzip,json
from fractions import Fraction as F
T=json.load(gzip.open('assets/execution_trees.json.gz','rt'))
RJ=json.load(gzip.open('research/jumbling/restoration/09-general-method/data/realign.json.gz','rt'))
SP={d['orbit']:d['s'] for d in RJ['orbits']}
H={0:2,3:2,6:10,8:2,11:2,17:5,22:2,25:2,28:5,32:2,34:60}
h={0:5,3:3,6:4,8:3,11:3,17:2,22:3,25:2,28:2,32:2,34:3}
for single in ('construction','kit'):
 for unit in ('twists','actions'):
  total=0;expect=F(0)
  for t in T:
   o=t['orbit'];N=len(set(t['positions']))+2;s=1 if unit=='actions' else SP[o];d=max(t['depth']);w=s+2*d
   ES=F(N-5)+sum((F(1,j) for j in range(1,N+1)),F(0))+F(3,N)+F(1,N*(N-1))
   P=4*s+2*h.get(o,0)+4*d;PB=2*s+2*h.get(o,0)
   SC=(4*s+(16 if o==6 else 10) if single=='construction' else 2*(PB+P)) if o in (6,34) else 0
   total+=(3*N//2-3)*w;expect+=ES*w
   if o in H:
    total+=(N-2)*P+PB+SC;expect+=F(H[o]-1,H[o])*((N-2)*P+PB)
    if o in (6,34):expect+=(F(4,5) if o==6 else F(59,60))*SC
  print(single,unit,'worst',total,'expected_upper_ceiling',(expect.numerator+expect.denominator-1)//expect.denominator)
print('creation70',sum(2*s for s in SP.values()))
```

### R7-04: O15 and O17 (proved, model review; computed by the review)

- S1 and S2 found no pure three-cycle for O15 or O17. That does not prove that none exists.
- **The two prefixes that fit.** The spine prefixes O17(10) and O15(94) each move exactly three pieces of their orbit, as a three-cycle with a cube identity on the orbit. Their collateral fits the committed order, in which O17 comes before O15. On both orbits a relocation needs at most 18 letters.
- **The shorter prefixes conflict.** Every listed prefix of O15 shorter than 94 letters (10, 22 and 46) moves O17, and every listed prefix of O17 shorter than 94 letters (10, 22 and 46) moves O15. So no one-pass order can take a prefix shorter than 94 letters on both orbits.
- **The swap does not pay.** Taking O15(10) before O17(94) would raise the bound by 362,376 twists at equal relocation costs, because the seed coefficient of O17 (7,911) exceeds that of O15 (3,597).
- The certificates below select exactly O17(10) and O15(94).

### Certificates for the replacements (computed by the integrator)

- **Candidates for orbit o.**
  - The pure three-cycles of S1 and S2.
  - The spine nodes of the orbit's own seed that are shorter than the seed and act on orbit o as a three-cycle of positions. Their collateral must lie on orbits solved after o (M2-06).
- **Relocation.** g = w1 w2 w3.
  - w1 is a shortest word moving one piece to A.
  - w2 is a shortest word of letters that fix position A, moving the second piece to B.
  - w3 is the inverse of the tree path to the third piece. That path fixes A and B; this was checked as positions for every node of every tree.
  - All six orders of the three pieces are tried. The shortest g is kept, and g⁻¹ P g is freely reduced.
- **Checks.** Each replacement is checked on all 259,800 slots:
  - on orbit o it moves exactly the stickers of A, B and C, as A → B → C → A;
  - its collateral orbits are exactly those recorded, and they are solved after o.
- **Result** (`scripts/realign/realign.py`, about 50 s; `data/realign.json.gz`).
  - Each orbit keeps the shortest of its certified candidates if that is shorter than its seed with its relocation.
  - 31 of the 35 seeds are replaced: 17 by pure three-cycles and 14 by spine prefixes. No replaced seed is now longer than 1,546 primitives with its relocation (O28).
  - O10, O24, O25 and O26 keep their seeds: no candidate is shorter.
  - The relocations have 4 to 19 letters.
  - Every replacement has a cube identity on its own orbit.
  - The fibres are aligned on the 21 replaced orbits with a trivial H_o. They are not aligned on the 10 replaced orbits with a nontrivial H_o: O00, O03, O08, O11, O22, O32 (C2), O17, O28 (C5), O06 (D5) and O34 (A5). Only O06 and O34 need the re-indexed pairs.
  - The three largest seeds all fall:
    - O15: from 11,574 to 106, a spine prefix of 94 letters;
    - O34: from 12,294 to 202;
    - O02: from 9,230 to 314.

### The cascade with the replaced seeds (computed; replayed independently)

- **The runner** (`scripts/realign/restore_realigned.py`, about 35 s). It runs the stage 03 cascade with the replaced seeds:
  - the committed order, which it asserts, and the committed kits on the other orbits;
  - `ReKit` on the replaced orbits, with the certified collateral as `allowed`;
  - the same synthetic inputs: the input label digests equal the committed ones.
- **Both inputs are restored, 5.1 times shorter:**
  - uniform:1: 45,009,802 twists (committed: 229,763,734), 191,143 factors;
  - uniform:2: 44,940,982 twists (committed: 230,484,810), 191,089 factors.
- **Control.** The same runner with no seed replaced reproduces the committed run exactly: 229,763,734 and 230,484,810 twists, 190,925 and 191,093 factors.
- **Independent check.** The stage 03 checker (`checker/verify_restore.py`, which shares no code with the restorer) evaluates the solutions with its own code. It passes: both inputs are restored to the identity, and the lengths agree. The stored start arrays, hashed independently, equal the committed inputs (R7-02).
- **What the inputs are.** They are those of M2-05: valid examples, not uniform samples of G.

Per orbit, on uniform:1 (sorted by the committed length; "s_o" is the seed with its relocation, in primitives):

| Orbit | H_o | s_o before | replacement | s_o after | relocation | fibres aligned | collateral orbits | uniform:1 before | uniform:1 after |
|---|---|---:|---|---:|---:|---|---:|---:|---:|
| O15 | 1 | 11,574 | spine prefix of 94 letters | 106 | 6 | yes | 23 | 27,878,156 | 274,448 |
| O02 | 1 | 9,230 | spine prefix of 286 letters | 314 | 14 | yes | 19 | 22,190,288 | 782,316 |
| O29 | 1 | 9,220 | pure three-cycle | 158 | 9 | yes | 0 | 22,171,180 | 395,522 |
| O28 | C5 | 3,080 | spine prefix of 1,534 letters | 1,546 | 6 | no | 4 | 18,579,754 | 9,374,454 |
| O08 | C2 | 1,544 | spine prefix of 94 letters | 114 | 10 | no | 18 | 16,468,066 | 1,287,212 |
| O03 | C2 | 1,110 | spine prefix of 562 letters | 566 | 8 | no | 3 | 11,771,346 | 6,058,270 |
| O18 | 1 | 1,546 | pure three-cycle | 248 | 8 | yes | 0 | 11,189,962 | 1,840,468 |
| O14 | 1 | 1,544 | pure three-cycle | 162 | 9 | yes | 0 | 11,182,258 | 1,224,608 |
| O17 | C5 | 1,546 | spine prefix of 10 letters | 24 | 8 | no | 24 | 9,364,276 | 176,060 |
| O11 | C2 | 776 | pure three-cycle | 68 | 10 | no | 0 | 8,463,566 | 807,286 |
| O22 | C2 | 776 | spine prefix of 22 letters | 38 | 9 | no | 13 | 8,440,728 | 482,496 |
| O00 | C2 | 2,218 | spine prefix of 560 letters | 584 | 19 | no | 32 | 8,176,278 | 2,195,432 |
| O01 | 1 | 2,212 | pure three-cycle | 608 | 14 | yes | 0 | 8,012,734 | 2,239,936 |
| O34 | A5 | 12,294 | spine prefix of 190 letters | 202 | 6 | no | 4 | 7,317,366 | 123,420 |
| O10 | 1 | 776 | seed kept | 776 | - | - | - | 5,650,306 | 5,653,440 |
| O12 | 1 | 776 | pure three-cycle | 72 | 10 | yes | 0 | 5,648,160 | 577,250 |
| O04 | 1 | 562 | pure three-cycle | 74 | 11 | yes | 0 | 4,124,174 | 609,294 |
| O05 | 1 | 562 | pure three-cycle | 166 | 9 | yes | 0 | 4,118,526 | 1,269,574 |
| O09 | 1 | 394 | spine prefix of 190 letters | 202 | 6 | yes | 8 | 2,902,848 | 1,520,288 |
| O07 | 1 | 298 | pure three-cycle | 82 | 11 | yes | 0 | 2,217,248 | 661,580 |
| O16 | 1 | 296 | spine prefix of 70 letters | 86 | 8 | yes | 3 | 2,186,072 | 674,652 |
| O06 | D5 | 398 | spine prefix of 4 letters | 28 | 12 | no | 33 | 1,350,986 | 117,034 |
| O32 | C2 | 106 | pure three-cycle | 74 | 9 | no | 0 | 1,195,612 | 874,152 |
| O13 | 1 | 152 | pure three-cycle | 68 | 10 | yes | 0 | 1,153,266 | 548,214 |
| O19 | 1 | 152 | pure three-cycle | 106 | 5 | yes | 0 | 1,150,222 | 819,106 |
| O20 | 1 | 152 | pure three-cycle | 46 | 10 | yes | 0 | 1,148,246 | 385,584 |
| O31 | 1 | 152 | pure three-cycle | 56 | 8 | yes | 0 | 1,146,608 | 455,380 |
| O21 | 1 | 104 | spine prefix of 46 letters | 56 | 5 | yes | 8 | 802,608 | 456,754 |
| O27 | 1 | 104 | pure three-cycle | 66 | 5 | yes | 0 | 800,174 | 526,458 |
| O30 | 1 | 104 | pure three-cycle | 56 | 9 | yes | 0 | 798,916 | 452,922 |
| O25 | C2 | 56 | seed kept | 56 | - | - | - | 646,818 | 658,140 |
| O23 | 1 | 56 | spine prefix of 46 letters | 54 | 4 | yes | 4 | 458,228 | 443,810 |
| O24 | 1 | 56 | seed kept | 56 | - | - | - | 454,880 | 455,014 |
| O26 | 1 | 56 | seed kept | 56 | - | - | - | 453,726 | 453,848 |
| O33 | 1 | 56 | pure three-cycle | 50 | 7 | yes | 0 | 150,152 | 135,380 |
| total | | | | | | | | 229,763,734 | 45,009,802 |

The lengths of O10, O24, O25 and O26 change slightly although their seeds do not. Earlier orbits now leave different collateral, so these orbits start from different states.

### Where M2 stands after round 7

- **Measured.** 4.5 × 10^7 twists on the two synthetic inputs, down from 2.3 × 10^8: about 1.4 years at one twist per second.
- **Proved (model review).**
  - For every input, the cascade as it runs needs at most 71,516,526 twists, about 2.3 years at one per second; the construction needs at most 71,514,544.
  - In actions, at most 4,841,151, about 56 days.
  - The lower bound stays at 224,601 twists. The gap between the bounds is now a factor of about 318 (it was about 1,600).
- **The largest terms on uniform:1.**
  - O28: 9.4 × 10^6. Its best replacement is a spine prefix of 1,534 letters (1,546 with its relocation); its pure three-cycle gives 8,404.
  - O03: 6.1 × 10^6. A prefix of 562 letters (566); its pure three-cycle gives 706.
  - O10: 5.7 × 10^6. It keeps its seed (776 with its relocation); its pure three-cycle gives 1,616.
- **Next (not run).**
  - Shorter candidates for O28, O03 and O10.
  - A cascade order chosen with the new collateral (M2-06).
  - The exhaustive frame check and the separate factor counts of R7-02.
  - A uniform sampler for the synthetic inputs (M2-05).
- **For the owner.** Whether a lattice phase of about 4.8 million actions, or 7 × 10^7 twists, counts as replicable.

## 13. Closing audit (round 8)

Round 8 audited METHOD.md and the public summaries against their sources before the repository is shown to other people. It asked three questions:
- **Q1.** Does every claim labelled proved, certificate, computed or measured have a source that supports it at that strength?
- **Q2.** Can the general method be stated as one theorem with explicit hypotheses?
- **Q3.** Does another choice among the recorded candidates of round 7, in any cascade order, give a smaller bound than 71,516,526 twists?

A Codex senior review returned nine findings, all verified:
- two statements, R8-01 for Q2 and R8-02 for Q3, which change no published claim;
- seven defects for Q1, R8-03 to R8-09: two `blocker`, four `major` and one `minor`.

The integrator checked each finding against the records and adopted all nine. The corrections are made in METHOD.md (the last section lists the changes), in the summaries and in M1-05 (section 2). The review's numbers are exact and were computed in memory; it re-ran no simulator replay.

### R8-01: the general method as one theorem (proved, model review)

**Theorem.** Assume:
- Z is a configuration reachable from the solved state by a finite admissible word in the menu I_a: at each of the 600 caps, 12 retained rotations (the identity among them) and 48 jumble rotations;
- the tests for legality and for the state are exact;
- while a piece is off the lattice, there is a nonempty unit;
- every heuristic batch is finite and accepts only verified clean steps, and the exhaustive fallback is fair;
- the retained group equals the kernel of the invariants (G = 𝒦, a certificate), and the correction library spans A_J, as Theorem R requires.

Then M\* reads only Z and gives a finite admissible word that restores all 259,800 sticker labels:
- **Phase 1** accepts clean steps for a nonempty unit U, with the free set off(Z) \ U for the fallback, and reaches the lattice in at most |off(Z)| steps.
- **Phase 2.** At the lattice endpoint Y, choose a correction C with ν(C) = −ν(Y) and put T = solved·C⁻¹. A retained word R takes Y to T in sticker labels, so Y·R·C is restored.
- A rotation of a piece that no sticker shows is allowed, by the owner's standard of "restored" (15 October 2026).
- No history is an input, and the reachable set need not be finite.

The summary of M\* in METHOD.md matches this statement.

What rests on what:
- **Proofs.** Round 1 proves that the fallback exists, by the inverse history and exact enumeration (M1-01). Round 4 composes the steps into M\* (AU-01, AU-06). Sections 7 and 8 review the progress of a clean step and Theorem R.
- **Conditional or partial results.** Rounds 2 and 3 give constructions for single cells under conditions. Round 5 adds bounded constructions for special cases without proving that they cover every unit. Rounds 6 and 7 bound the retained cascade and state when a seed can be replaced.
- **Finite certificates.** G = 𝒦, the lower bound on A_J, the validity and ranks of the library, the replacement lengths and the example restorations. Two runs with the replaced seeds are examples, not a proof of completeness.

What stays open:
- **M1.** No bound on the length of a fallback word computed from the state, and no proof that the bounded constructions cover every unit. A uniform bound exists if and only if R(I_a) is finite, which is open (M1-09).
- **E1.** Few runs, and no deep autonomous run.
- **E2.** No human has reviewed the proofs.
- **M2.** Whether the lattice phase counts as replicable is for the owner to judge; no threshold in twists or in actions is approved.

### R8-02: the round 7 selection is optimal over the recorded candidates (proved, model review; re-computed by the integrator)

The problem:
- For each orbit, choose a recorded candidate of round 7 (`data/realign.json.gz`) or the committed seed.
- Choose a cascade order in which the collateral orbits of every chosen element come after its own orbit.
- Minimise the R7-03 bound for the kit as it runs.

The proof:
- Write q_o = ⌊3N_o/2⌋ − 3, F_o = 1 on the 11 orbits with a nontrivial frame group (0 otherwise), and D_o = 1 on O06 and O34 (0 otherwise). With the tree depth d_o and the return length h_o of M2-03, the bound is

  B = Σ_o [q_o(s_o + 2d_o) + F_o((N_o − 2)(4s_o + 2h_o + 4d_o) + 2s_o + 2h_o) + D_o(12s_o + 8h_o + 8d_o)].

- Hence B = 4,470,438 + Σ_o a_o s_o, with a_o = q_o + F_o(4N_o − 6) + 12D_o. Every a_o is positive; the smallest is 663.
- Without the order condition, B is smallest when every s_o is smallest: Σ_o a_o min s_o = 67,046,088, so B ≥ 71,516,526.
- The round 7 selection takes the shortest element on every orbit, and it satisfies the committed order. So it attains the bound: the optimum is 71,516,526, a gain of zero.

The integrator re-computed the constant, the sum and the bound from `data/realign.json.gz` and `assets/execution_trees.json.gz`. All 111 recorded candidates satisfy the committed order, and the round 7 choice is the shortest on every orbit.

Scope:
- `realign.py` drops the spine prefixes whose collateral conflicts with the committed order before it writes `data/realign.json.gz`. The result says nothing about those prefixes (R7-04 treats O15 and O17) or about candidates not yet found.
- It optimises the bound, not the measured length.

### Corrections (R8-03 to R8-09)

| Finding | Severity | What was wrong | Correction |
| --- | --- | --- | --- |
| R8-03 | `blocker` | METHOD.md (gap M1) and M1-05 said that the 4,402-letter word restores s61c. Its end is on the lattice, but its ν is not zero (the input record of the stop62 lattice restore), so the stickers are not solved. | It takes s61c to the lattice, and the lattice phase then restores the stickers (METHOD.md section 5; M1-05 carries a correction note). |
| R8-04 | `major` | The root README and METHOD.md said that a pure transport word keeps every other piece at its exact pose. Each transport seed cycles three whole cells of its kind, so it moves the contents of the other two. | A transport word moves several whole cells of one kind and fixes every other cell, and a clean step needs the other chambers of those cells vacant. The turn of an edge or vertex cell keeps every piece outside the cell (root README, METHOD.md section 3, restoration README). |
| R8-05 | `major` | The toolkit row of METHOD.md bounded a clean step by 2(Q_safe − 1) + b. The tests of AU-03 do not bound \|W\|, and b bounds W only when all of the class's grips are free. | The bound is 2(Q_safe − 1) + \|W\|, with \|W\| ≤ b when all of the grips are free; otherwise the length of W is checked (METHOD.md section 3). |
| R8-06 | `blocker` | The restoration overview said that in a clean step every other off piece keeps its exact pose. A clean step may also restore pieces of its free set: stop62 step 13 restored its 61-piece unit and the 86 pieces of its free set (233 to 86 off pieces). | The definition of METHOD.md: the unit and every lattice piece end on the lattice, and every off piece outside the unit and the declared free set keeps its exact pose. V·W·V⁻¹ and the pure twist are the fast steps, and the exhaustive search of Theorem M\* is the fallback (restoration README). |
| R8-07 | `major` | The summaries gave the lower bound of 224,601 twists without its condition: M2-02 counts retained words only. | "With retained words only" in the root README and the two jumbling READMEs. METHOD.md says that a solution through jumble twists is not covered. |
| R8-08 | `minor` | Bounds for the retained cascade R were given as the cost of the whole lattice phase, which also has the corrections C, at most 25,852 twists (AU-05). | METHOD.md section 3 names R and C: R needs at most 71,516,526 twists as it runs, and R and C together at most 71,542,378. The summaries say "its cascade" and add the corrections. |
| R8-09 | `major` | The 09 README and METHOD.md section 6.2 cite the driver logs and the replay reports (`gdrive.jsonl`, `verify_u.jsonl`) of `runs/stop62-v4/` and `runs/e2run24-v4/`, but they were not in the repository: its `*.jsonl` ignore rule had dropped them. | The four files are now committed. Private paths in them are replaced by `<working tree>`. |

## 14. Is the reachable set finite? (round 9)

Gap M1 asks for a bound on the length of a fallback step. M1-09 shows that a uniform bound exists if and only if R(I_a), the set of configurations reachable from solved, is finite. That is open (Card 5 Q1.1, open problem X1). Round 9 asked the senior review to decide X1, or else to prove the strongest partial results and say what they give for M1.

X1 stays open. X2 is closed for I_a in its stated form, a repeatable word, by T9-02, and Proposition HB still covers only the model orbits 0 to 9 (corrected in round 12, R12-06). The review returned five findings, all verified:
- one correction to stage 04 (T9-01, `major`);
- four results (T9-02 to T9-05, `minor`).

The integrator re-ran the three exact computations (T9-01, T9-02 and T9-04), got the reported outputs, and adopted all five findings. Their code was added to this section in round 12 (R12-04; "Reproduction (round 9)" below).

### T9-01: the extended closure of stage 04 drops a reachable transition (proved, model review; the witness re-run by the integrator)

- **The filter.** `refexact2.py` (stage 04) extends the closure of Lemma R for model orbit 10 with the blocking by the inner cohorts of the out grips. When a twist carries the piece into the cap of an out grip g, the script admits g as out only if the other out grips do not block g after the twist (line 161).
- **Why it is wrong.** An out grip records an anchored inner cohort, and another cohort can straddle its cap. Being out does not mean being admissible now.
- **The witness.** From solved, the word [(0, 12), (11, 41)] is admissible.
  - Piece 12443 (model orbit 10) is not moved by the first letter; the second carries it wholly into cap 0.
  - Grip 0 stays out, and the inner piece 3468 (model orbit 6), jumbled by the second letter, straddles cap 0.
  - So the transition must add grip 0 as out, and the filter drops it. The integrator re-ran the witness: the letters move 3,097 and 2,702 pieces, and every assertion holds.
- **What changes.**
  - A closure of `refexact2.py` would not have proved a bound.
  - The stage 04 conclusion stands. The filter only removes transitions, so the plain system R_o still reaches everything the recorded run reached (600,000 states, height 14), and the method gives no bound from orbit 10 on.
  - Proposition HB uses `refexact.py`, which has no such filter, and stands.
- **Next.** Repair the filter: an entering out grip must be compatible with the admissibility of the letter just applied, not with the blocking after it. Then prove that the extended system simulates the puzzle, and use the witness as its first acceptance test.

### T9-02: a word repeated without end is periodic, with a uniform period (proved, model review; the constants re-run by the integrator)

**Theorem.** Put K = 4097⁴ = 281,749,955,297,281 and B = 60·lcm(1, 2, …, K). Suppose a nonempty I_a word W can be applied to Z again and again without end. Then W^B Z = Z: every piece returns to its exact pose. B depends on neither W nor Z.

*Proof.*
1. Every home piece contains a ball of radius r = 1/512 whose centre has norm less than R = 8. This is an exact check over the 36 chamber representatives (the smallest barycentre slack is 8(√5 − 1)/625), and K⁺ preserves it.
2. Fix a piece. At the start of each repetition of W, its pose determines its *itinerary*: whether it is inside or outside the cap of each later letter.
3. Suppose the itineraries at two starts differ. At the first letter where they differ, that letter's cut separates the two images of the ball, and both images have had the same rotations until then. So the two starting balls are disjoint.
4. All the centres lie in the ball of radius R. Disjoint balls of radius r inside the ball of radius R + r number at most ((R + r)/r)⁴ = K, so a piece has at most K different itineraries.
5. The itinerary at the next repetition is the shift of the current one. So the itineraries are eventually periodic, with a period k ≤ K.
6. Over one period the piece's rotation is A = [l, u] with u in 2I.
   - If no letter of the period moves the piece, A is the identity.
   - Otherwise the piece's centre lies inside the cap of some letter in every period, and averaging over the periods gives a nonzero vector v fixed by A. Then l = v u v⁻¹, and A⁶⁰ = 1 because 2I has exponent 60.
7. So each pose is eventually periodic, with a period dividing 60k, and the whole configuration repeats after B copies of W. Admissible letters are invertible, so W^B Z = Z. **QED**

What it gives:
- **No repeated word raises heights without bound.** X2 in the form "a repeatable word" cannot succeed. An unbounded family cannot be ultimately periodic, but it need not adapt to the state; stage 04 said that it "would have to be adaptive" (corrected there).
- **A bound for one family.** Along W, W², … from solved, every state has a restoring word of at most B·|W| − 1 letters. This covers only that family, so X1 stays open.

The integrator re-ran the exact check of the constants and got r = 1/512, R = 8 and K = 281,749,955,297,281.

### T9-03: component size bounds relative heights, not the common pose (proved, model review)

- **Relative heights.** In a connected component of n pieces of the 3-face graph, Lemma L gives d(x_p, x_q) ≤ 2 d_C(p, q) ≤ 2(n − 1). So an anchor q with a proved height bound H gives h(p) ≤ H + 2 d_C(p, q). An anchored piece has H = 2, and Proposition HB gives other anchors.
- **Frozen neighbourhoods, stronger than Lemma FN.** Suppose a piece has height at least 4 before or after an admissible twist. Then the twist changes neither its set of 3-face neighbours nor its relative poses to them.
- **Finitely many shapes.** Along a contact edge the relative pose has right height 0 and left height at most 2: at most 21 · 7,200 = 151,200 choices. So a fixed labelled component has finitely many internal geometries.
- **The obstruction.** The common pose of a component is not controlled.
  - A whole high component could rise with the same members and the same internal geometry. Corollary FN1 (d) recruits the height-2 neighbours that exist, but does not make them exist.
  - In the tree, alternating jumble factors from two branches raise a vertex by 2 each time, and the same factors carry a fixed patch rigidly.
  - This is a countermodel to any bound that uses only tree distances and co-motion. It is not an admissible history: whole-cap containment and blocking are not used.
- **What would settle M1.** Suppose an invariant gave every component an anchor of bounded height at every reachable configuration. Then all heights would be bounded, and M1-09 would give a uniform bound. No such invariant is known.

### T9-04: no single letter takes H6 to height 8 (computed by the review; re-run by the integrator)

- **The caps.** H6 is the deep state of stage 04 (digest fe917f9a…). It has 78 pieces at height 6, all in branch 4. Exactly six caps of another branch contain a height-6 piece:

  | Grip | 274 | 326 | 340 | 430 | 434 | 463 |
  |---|---:|---:|---:|---:|---:|---:|
  | Height-6 pieces held | 78 | 78 | 78 | 78 | 59 | 70 |
  | Straddling piece (witness) | 79414 | 84169 | 71323 | 106423 | 13070 | 165151 |

  Every one of these caps is straddled.
- **The argument.** Retained letters keep heights, and a jumble changes a height by at most 2 (the proof of Corollary FN1 (c)). So reaching height 8 in one letter would need one of these caps, and no single letter reaches height 8 from H6.
- **What it adds.** The check is exact and uses neither the floating-point 3-face graph nor component membership. It makes that part of the H6 evidence of stage 04 an exact computation that can be replayed (the code is below). It was called a certificate before its code was preserved (corrected in round 12, R12-04).
- **Scope.** It says nothing about longer words; a longer construction would first have to change the blocking.

### T9-05: bounded starting heights (proved, model review)

- **A finite maximum.** For each m, the reachable states whose pieces all have left height at most 2m have a finite maximum distance b(m) to the lattice. There are at most S_m of them (M1-09), and each has a finite restoration. This holds even if R(I_a) is infinite.
- **Not an explicit bound.** It does not give b(m) ≤ S_m − 1: that would need restorations that stay within the height restriction, and M1-09 shows that starting heights do not ensure this. Nor does it give an algorithm, since no procedure is known that recognizes when all reachable states of the restriction have been found.
- **M1-09 in both directions.**
  - A uniform clean-step length C restores any state to the lattice in at most 177,120·C letters. Each letter changes a height by at most 2, so every reachable height is at most 354,240·C.
  - A finite R(I_a) gives a path to solved of at most |R(I_a)| − 1 letters, which is a clean step for any unit.

### Where X1 stands after round 9

- X1 stays open. X2 is closed for I_a in its stated form by T9-02 (corrected in round 12, R12-06). Proposition HB still covers only the model orbits 0 to 9.
- **New results:**
  - a uniform period for words repeated without end (T9-02);
  - an exact computation, replayable from the code below, that one letter cannot reach height 8 from H6 (T9-04);
  - the obstruction for arguments from components alone (T9-03);
  - a finite but unknown maximum b(m) (T9-05).
- **Next (not run):**
  - repair the closure of T9-01 and prove that it simulates the puzzle;
  - build an exact 3-face graph of H6, to test whether its 78-piece component can gain new members (T9-03).

### Reproduction (round 9)

Added in round 12 (R12-04). From the repository root, with Python 3.13 and NumPy 2.5; each script runs in under ten seconds.

T9-01, the witness. Output:

```text
0 3097
11 2702
reachable newly holding out grip is blocked: confirmed
```

```python
import sys
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling']
import sim, restore as RS
from sim.kernel import Evaluator, pullback_form
C = sim.get_context()
M = RS.menu('I_a')
s = sim.State(C, filtered=False, menu=M)
p = 12443
assert int(C.data.orbit_id[p]) == 10
assert 0 not in C.data.signature(p) and 11 in C.data.signature(p)
def inner(c):
    return [int(q) for q in C.data.cap_members(c)
            if int(C.data.orbit_id[q]) in {0,1,2,3,6}]
a, b = inner(0), inner(11)
E0, E1 = RS.element(M,0,12), RS.element(M,11,41)
for c, E in [(0,E0),(11,E1)]:
    assert E in [x.matrix for x in M.for_cap(c)]
    out = s.apply(sim.Twist(C,c,E,'menu',{'menu':M.name}), journal=False)
    assert out.applied
    print(c, out.moved)
    if c == 0:
        assert int(s.pose_id[p]) == 0
assert len(a) == len(b) == 68
assert all(s.pose(q) == E0 for q in a)
assert all(s.pose(q) == E1 for q in b)
def classify(q,c):
    return Evaluator(pullback_form(s.pose(q),C.data.N[c])).classify(C.regions.vform(q))[0]
assert classify(p,0) == 'in'
assert 3468 in b and int(C.data.orbit_id[3468]) == 6
assert classify(3468,0) == 'straddle'
print('reachable newly holding out grip is blocked: confirmed')
```

T9-02, the constants. Output:

```text
r=1/512, R=8, K= 281749955297281
```

```python
import sys
sys.path.insert(0, 'research/jumbling')
import sim
from exact import Q5 as Q
C=sim.get_context(); d=C.data; zero=Q(0); slack=None
for V in C.regions.rep_vertices:
    c=[sum((v[i] for v in V),zero)/Q(len(V)) for i in range(4)]
    for v in V:
        assert (Q(64)-sum((x*x for x in v),zero)).sign()>0
    for n in d.N:
        t=sum((n[i]*c[i] for i in range(4)),zero)
        a=t-d.KAPPA
        assert a.sign()!=0
        if a.sign()<0: a=-a
        b=d.NN-t
        assert b.sign()>0
        for z in (a,b):
            if slack is None or (z-slack).sign()<0: slack=z
assert len(C.regions.rep_vertices)==36
assert slack==Q(-8,8,625)
assert (Q(25)-d.NN).sign()>0
assert (slack-Q(5,0,512)).sign()>0
print('r=1/512, R=8, K=',4097**4)
```

T9-04, the caps of H6. It asserts the digest of H6 and its height counts, classifies every height-6 piece exactly against every cap of another branch, and asserts the six containment counts and the six straddling witnesses. Output:

```text
{274: 79414, 326: 84169, 340: 71323, 430: 106423, 434: 13070, 463: 165151}
```

```python
import gzip,json,sys
from pathlib import Path
from collections import Counter
import numpy as np
sys.path[:0]=['research/jumbling/theory','research/jumbling']
import sim,trees as T
from sim.kplus import matrix_from_json
from sim.kernel import Evaluator,pullback_form
p=Path('research/jumbling/restoration/04-card5-lattice-invariants/data/t-T723x433-12.json.gz')
tab=json.loads(gzip.decompress(p.read_bytes()))
C=sim.get_context(); s=sim.State(C,filtered=False)
mats=[matrix_from_json(m) for m in tab['poses']]
s.pose_id[:]=np.array([s._intern_matrix(m) for m in mats],np.int32)[tab['pose_id']]
assert s.digest()==tab['digest']=='fe917f9a77c69cf037ce5c894debdec2448b73575ed776d5c529ccfaf5bc7c39'
lf={i:T.factors(s._pose_matrix(i))[0] for i in set(s.pose_id.tolist())}
h={i:T.dist(l) for i,l in lf.items()}
assert Counter(h[int(i)] for i in s.pose_id)=={0:57042,2:115850,4:4150,6:78}
top=[q for q,i in enumerate(s.pose_id) if h[int(i)]==6]
br={q:T.branch(lf[int(s.pose_id[q])]) for q in top}
assert set(br.values())=={4}
forms={q:C.regions.vform(q) for q in top}; caps={}
for c in range(600):
    cb=T.branch(tuple(C.data.N[c])); ev={}
    for q in top:
        if cb==br[q]: continue
        i=int(s.pose_id[q])
        if i not in ev: ev[i]=Evaluator(pullback_form(s._pose_matrix(i),C.data.N[c]))
        if ev[i].classify(forms[q])[0]=='in': caps.setdefault(c,[]).append(q)
assert {c:len(q) for c,q in caps.items()}=={274:78,326:78,340:78,430:78,434:59,463:70}
w={}
for c in caps:
    cl=s.classify(c,need_inside=False,stop_at_straddle=True)
    assert cl.straddle is not None and not cl.uncertain
    w[c]=int(cl.straddle['piece'])
assert w=={274:79414,326:84169,340:71323,430:106423,434:13070,463:165151}
print(w)
```

## 15. A shorter construction for the lattice phase (round 10)

After round 8 the construction of the retained cascade R needs at most 71,514,544 twists for every input: 45,376,326 for the position passes and 26,138,218 for the frame passes. The position loop performs each three-cycle of positions with up to two stars t_X = u_X⁻¹ c u_X, so with up to two applications of the seed. Round 10 asked whether another construction from the same seeds, trees and return words has a smaller proved bound:
- **Q1.** Perform each three-cycle with one application of the seed, conjugated by a short word (a *direct three-cycle*).
- **Q2.** Set the frame of each piece when the position loop places it (a *twisted cycle*), so that only the buffers need fixing at the end.
- **Q3.** The bound of each, and of both together.

A Codex senior review returned four findings, all verified:
- one result that lowers a published bound (T10-02, `major`): with direct three-cycles the construction needs at most 50,560,184 twists for every input;
- the direct three-cycle itself (T10-01), an obstruction to the proposed twisted cycles (T10-03) and the cost of the buffer cleanup that would follow them (T10-04), all `minor`.

The integrator re-ran both inline computations (T10-02 and T10-04) and got the reported outputs; the T10-02 computation also checks the witness of T10-01. All four findings are adopted. The new construction is not implemented.

### T10-01: a direct three-cycle (proved, model review; the witness re-run by the integrator)

- **The direction.** Words are applied left to right, so u⁻¹ c u carries the support of c forward along u. Let c' be the seed of orbit o, which cycles the buffers A, B and C. If a retained word g fixes position A and takes X to B and Y to C, then g c' g⁻¹, not g⁻¹ c' g, cycles the positions A, X and Y.
- **The conjugating word.** Let w2 be a shortest word of letters that fix position A and take X to B, and put Z = w2(Y). Z is neither A nor B, so the committed tree gives a word u_Z that fixes every sticker of A and B and takes C to Z. Then g = w2 u_Z⁻¹ works and has at most e_o + d_o letters, where d_o is the depth of the tree and e_o the largest length of w2 over the positions of the orbit. One cycle costs at most s_o + 2(e_o + d_o) twists: one application of the seed instead of up to two.
- **e_o.** An exact breadth-first search over the letters that fix A reaches all N_o − 1 other positions on every orbit. For O00 to O09, e_o = 18, 11, 9, 9, 8, 8, 8, 7, 7 and 7; it is 6 on O10 to O27 and on O30 to O33, 5 on O28 and O29, and 4 on O34.
- **The number of cycles.** The loop still performs (m + k − 2i)/2 cycles, as in M2-04: each cycle lowers m + k − 2i by two. Every N_o is divisible by 4, so there are at most 3N_o/4 − 1 cycles, attained by N_o/2 disjoint transpositions. For uniform even positions the expected number is (N_o − 4 + H_N + 2/N_o + 1/(N_o(N_o − 1)))/2, where H_N is the harmonic number of N_o.
- **Collateral.** A conjugate by a retained word moves pieces of exactly the orbits that the seed moves, so every completed direct three-cycle respects the cascade order. Its prefix g may move pieces of orbits already solved; g⁻¹ puts them back.
- **The witness.** On O00 (A = 117, B = 175946, C = 6290), take X = B and Y = 4939. The tree path u = [38] takes C to Y, so g = u⁻¹ = [−38] takes Y to C. The cycle g c' g⁻¹ has its third position at 4939, as required. The form g⁻¹ c' g, which the packet proposed, puts it at 437.

### T10-02: with direct three-cycles the construction needs at most 50,560,184 twists (proved, model review; re-computed by the integrator)

- **The construction.** The position loop of M2-04 with a direct three-cycle for every cycle, then the frame construction of M2-03 unchanged: the pairs, and the shorter single on O06 and O34.
- **The bound.** For each orbit, (3N_o/4 − 1)(s_o + 2(e_o + d_o)) for the positions, plus the frame term of M2-03 on the 11 orbits with a nontrivial frame group. For actions, s_o is replaced by 1. With the certified lengths of round 7:

| | Construction of round 7 | With direct three-cycles |
| --- | ---: | ---: |
| Position passes, twists | 45,376,326 | 24,421,966 |
| Frame passes, twists | 26,138,218 | 26,138,218 |
| Every input, twists | 71,514,544 | 50,560,184 |
| On average over G, at most, twists | 46,189,354 | 32,201,480 |
| Every input, actions | 4,840,993 | 4,675,165 |
| On average over G, at most, actions | 3,098,745 | 2,988,213 |

- **The averages.** They are upper bounds on the expected length for a uniform input on G, not exact expected lengths. The position word depends only on positions, so the frames of a uniform input stay uniform given its positions. Earlier passes of the cascade translate independent coordinates of later orbits and keep them uniform. So the frame factors of M2-04 still apply.
- **What it gives.**
  - A gain of 20,954,360 twists (29%) over 71,514,544. The bound is about 225 times the lower bound of 224,601 retained twists (M2-02), and about 1.6 years at one twist per second (4,675,165 actions: about 54 days).
  - The library is unchanged: the 35 replaced seeds and their inverses, created in 14,048 twists. A person replays one stored macro per cycle instead of up to two; each letter of a conjugating word counts as one action.
  - The frame passes are now the larger part. The largest orbit terms are O28 (10.6 million twists, of which 8.9 million frames), O03 (9.9 million, 8.3 million frames) and O10 (4.3 million, positions only).
  - The bound is for R; the corrections C add at most 25,852 twists (AU-05). The kit as it runs keeps the bound of round 7, 71,516,526 twists, until the direct three-cycle is implemented.

The bound (from the repository root, `python -B`; 31 seconds). Output:

```text
e [18, 11, 9, 9, 8, 8, 8, 7, 7, 7, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 5, 5, 6, 6, 6, 6, 4]
twists 50560184 32201480
actions 4675165 2988213
```

```python
import gzip, json, sys
import numpy as np
from fractions import Fraction as F
sys.path.insert(0, 'research/jumbling/restoration/03-retained-group-and-lattice-restorer/scripts')
from rlib import Retained, invword
R = Retained('.')
first = R.psorted[R.poff[:-1]]
E = []
for o, t in enumerate(R.trees):
    pcs = R.orbit_pieces(o)
    n = len(pcs)
    ix = np.full(R.np, -1, dtype=np.int32)
    ix[pcs] = np.arange(n)
    fs = first[pcs]
    A, B = map(int, t['buffers'])
    ai, bi = int(ix[A]), int(ix[B])
    pm = []
    for m in range(1, 1201):
        s, d = R.move(m)
        j = np.minimum(np.searchsorted(s, fs), len(s)-1)
        hit = s[j] == fs
        img = fs.copy()
        img[hit] = d[j[hit]]
        p = ix[R.sp[img]]
        if p[ai] == ai:
            pm.extend((p, np.argsort(p)))
    dist = np.full(n, -1, dtype=np.int32)
    dist[bi] = 0
    front = np.array([bi])
    depth = 0
    while len(front):
        new = np.unique(np.concatenate([p[front] for p in pm]))
        new = new[dist[new] < 0]
        depth += 1
        dist[new] = depth
        front = new
    assert dist[ai] == -1 and np.count_nonzero(dist < 0) == 1
    E.append(int(max(dist)))
print('e', E)
t = R.trees[0]
C, Y = t['third'], t['positions'][2]
u = [int(t['move'][2])]
assert t['parent'][2] == 0 and u == [38]
assert int(R.sp[R.map_through([first[C]], u)][0]) == Y == 4939
assert int(R.sp[R.map_through([first[C]], invword(u))][0]) == 437
RJ = json.load(gzip.open('research/jumbling/restoration/09-general-method/data/realign.json.gz', 'rt'))
SP = {r['orbit']: r['s'] for r in RJ['orbits']}
H = {0:2,3:2,6:10,8:2,11:2,17:5,22:2,25:2,28:5,32:2,34:60}
h = {0:5,3:3,6:4,8:3,11:3,17:2,22:3,25:2,28:2,32:2,34:3}
for unit in ('twists', 'actions'):
    total, expect = 0, F(0)
    for t in R.trees:
        o = t['orbit']
        N = len(set(t['positions'])) + 2
        assert N % 4 == 0
        s = SP[o] if unit == 'twists' else 1
        d = max(t['depth'])
        W = s + 2*(E[o]+d)
        EC = (F(N-4) + sum((F(1,j) for j in range(1,N+1)), F(0))
              + F(2,N) + F(1,N*(N-1))) / 2
        total += (3*N//4-1)*W
        expect += EC*W
        if o in H:
            P, PB = 4*s+2*h[o]+4*d, 2*s+2*h[o]
            SC = 4*s+(16 if o==6 else 10) if o in (6,34) else 0
            total += (N-2)*P + PB + SC
            expect += F(H[o]-1,H[o])*((N-2)*P+PB)
            if o in (6,34):
                expect += (F(4,5) if o==6 else F(59,60))*SC
    print(unit, total, -(-expect.numerator//expect.denominator))
```

### T10-03: the proposed twisted cycles do not work (proved, model review)

The proposal fails for two reasons. Neither uses commutativity, so both hold on D5 (O06) and A5 (O34).
- **Pieces that start in place are never visited.** The committed loop takes its work from the position errors only. On O03, leave all 3,600 positions correct, choose a position Z outside A, B and C, and give each of the other 3,596 positions the nonidentity frame. The number of flipped frames is even, so the frame invariant is zero and the input lies in G. The loop performs no cycle, and 3,596 frame errors remain outside the buffers, which no cleanup on the buffers can remove.
- **One fibre map does not depend on the return.** Let r_x fix the stickers of A and B and return C with frame value x. Then r_x⁻¹ c' r_x has the fibre maps (α, βx, x⁻¹γ): its map from A to B does not depend on x. After any fixed relocation one incoming map still does not depend on x. A step that closes the loop can place two pieces outside the buffers whose frames are prescribed independently. Give the piece on that fixed edge a frame that this map does not restore; then no choice of x works.

This refutes the proposed drop-in, not every construction with twisted cycles. A repaired schedule must visit the misframed pieces that start in place and control every frame that a step finalizes. Until such a schedule and its cost are proved, there is no bound for Q2 alone, or for Q1 and Q2 together; T10-02 keeps the frame passes.

### T10-04: the buffer cleanup, if the rest is solved (proved, model review; re-computed by the integrator)

- **The condition.** Every position, and every frame outside A, B and C, is already solved. T10-03 shows that the proposed twisted cycles do not reach this state.
- **The pairs.** Put Q_x = r_x⁻¹ c' r_x c'⁻¹. Its value at B is βxβ⁻¹. Its conjugate c'⁻¹ Q_x c' = c'⁻¹ r_x⁻¹ c' r_x is supported on A and C, has the value γ⁻¹x⁻¹γ at A, and costs at most 2s + 2h twists after cancellation. Both value maps cover the frame group, the non-abelian ones included. Clear A with the conjugate, then B with Q_x.
- **The single.** What remains at C lies in the commutator subgroup H′. The shorter single Q_x Q_y Q_xy⁻¹ gives the commutator x⁻¹y⁻¹xy there.
- **The cost.** At most 4s + 4h twists on the abelian orbits, 256 twists or 40 actions on O06, and 1,638 twists or 30 actions on O34, with no change of library. The commutators cover all 5 values of D5′ and all 60 of A5.

The computation (from the repository root). Output:

```text
6 10 5 8 256 40
34 60 60 5 1638 30
```

```python
import gzip, json
T = json.load(gzip.open('assets/execution_trees.json.gz', 'rt'))
R = json.load(gzip.open('research/jumbling/restoration/09-general-method/data/realign.json.gz', 'rt'))
S = {r['orbit']: r['s'] for r in R['orbits']}
def mul(x, y):
    return tuple(y[a] for a in x)
def inv(x):
    return tuple(x.index(j) for j in range(len(x)))
for o in (6, 34):
    t = T[o]
    base, h = t['frames'][0], {}
    for p, f, d in zip(t['positions'], t['frames'], t['depth']):
        if p == t['third']:
            x = tuple(base.index(j) for j in f)
            for z in (x, inv(x)):
                h[z] = min(h.get(z, d), d)
    assert all(mul(x,y) in h for x in h for y in h)
    best = {}
    for x in h:
        for y in h:
            z = mul(x,y)
            c = mul(mul(mul(inv(x),inv(y)),x),y)
            best[c] = min(best.get(c,10**9), h[x]+h[y]+h[z])
    q = max(best.values())
    print(o, len(h), len(best), q,
          8*S[o]+4*max(h.values())+2*q,
          8+4*max(h.values())+2*q)
```

The columns are the orbit, the size of the frame group, the number of commutator values, the longest single, and the cleanup cost in twists and in actions.

### Where M2 stands after round 10

- **Proved (model review).** A construction of R with direct three-cycles needs at most 50,560,184 twists for every input (at most 32,201,480 on average over G), or 4,675,165 actions: about 225 times the lower bound for retained words. The corrections add at most 25,852 twists.
- **Implemented.** The kit as it runs is unchanged: its bound stays 71,516,526 twists, and the measured runs 4.5 × 10^7 (section 12).
- **Next (not run).**
  - Implement the direct three-cycle in the kit, with the O00 witness of T10-01 as the first test of its orientation, and replay the two synthetic inputs with the stage 03 checker.
  - A twisted-cycle schedule that visits the misframed pieces that start in place and controls every frame a step finalizes (T10-03), followed by the cleanup of T10-04.
  - Shorter seeds for O28 and O03, whose frame passes are now the largest terms, and for O10.
- **For the owner.** Whether a lattice phase of about 4.7 million actions, or 5 × 10^7 twists, counts as replicable.

## 16. Fewer seed applications (round 11)

After round 10 a construction of the retained cascade R needs at most 50,560,184 twists for every input. Almost all of that is seed applications times seed length. Round 11 asked how far the number of seed applications can be cut with the same seeds, trees and return words:
- **Q1.** Move a frame pair to its target with a short word instead of a star (a *direct pair*).
- **Q2.** Fix two frames with one pair.
- **Q3.** Use the fewest three-cycles, without always passing through the buffer A.
- **Q4.** Repair the twisted cycles that T10-03 refuted.
- **Q5.** The new bound, and a floor: the fewest seed applications that any construction from conjugates of the seeds needs.

A Codex senior review returned seven findings:
- one result that lowers a published bound (T11-05, `major`): a construction of R needs at most 20,866,844 twists for every input;
- five results (T11-01 to T11-04 and T11-06, `minor`): direct pairs, two frames per pair, the fewest three-cycles, twisted cycles with two returns, and the floor;
- one open question (T11-07, `minor`): exact average lengths.

The integrator re-ran the five inline computations (T11-01, T11-02, T11-04, T11-05 and T11-06) and got the reported outputs. Two parts were checked independently:
- the distance arrays of T11-01, with a separate breadth-first search;
- the count of T11-03, by a breadth-first search over the alternating group A_8.

All seven findings are adopted. The construction is not implemented.

### T11-01: direct pairs (proved, model review; re-computed by the integrator)

- **The pair.** Let g be a word of letters that fixes position C and takes X to B. Write a for the fibre transport of g from X to B, δ for its transport at C, and β = c'_B. Then g Q_x g⁻¹ has the value aβxβ⁻¹a⁻¹ at X and δx⁻¹δ⁻¹ at C, and fixes the rest of the orbit.
  - Choosing x = (aβ)⁻¹v(aβ) puts any value v at X.
  - g must fix the position C, not its stickers.
  - The pair costs at most 2s_o + 2h_o + 2f_o twists: two applications of the seed instead of four (the committed pair conjugates by a star, which contains the seed).
- **f_o.** f_o is the largest distance to B with the letters that fix C. For O00 to O34 it is 18, 11, 9, 9, 8, 8, 8, 7, 7, 7, 7, then 6 on O11 to O21, and on O22 to O34 5, 6, 5, 5, 6, 5, 5, 5, 6, 6, 5, 6, 5. It differs from e_o of T10-01: on O34, f = 5 and e = 4.
- **The non-abelian groups.** An exact enumeration covers all 719 × 10 = 7,190 transported values on O06 and all 119 × 60 = 7,140 on O34. At every target the values fill the frame group.
- **The transport matters.**
  - On O06, take X = 10 and g = [1, 15, 57, 211, 441, 731, 1025]. The return [360, 432] gives the base pair the value (0, 3, 4, 1, 2) at B, but the transported value at X is (1, 0, 3, 2, 4).
  - On O34, g = [−510, −698, 921, −982] for X = 72146 fixes the position C but rotates its fibre.

### T11-02: two frames per pair (proved, model review; re-computed by the integrator)

- **C2 (seven orbits).** After the position pass the number m of flipped frames is even (the invariant), and a transported pair flips both of its endpoints. So m/2 pairs suffice, and no schedule of operations on at most two frame positions does better: at most N_o/2 pairs, N_o/4 on average.
  - A pair between two arbitrary positions first sends Y to C with at most b_o letters, then the image of X to B with letters that fix C. It costs 2s_o + 2h_o + 2(b_o + f_o).
  - b_o is the largest distance to C with all letters: 19, 9, 8, 7, 6, 6, 6, 5, 5, 5 and 5 on O00, O03, O06, O08, O11, O17, O22, O25, O28, O32 and O34. The distance to A does not replace it.
- **C5 (O17, O28).** A pair puts inverse values at its ends, so two errors a, a cannot both be removed by one pair; the errors a, a, a³ have zero invariant and show it.
  - Split the errors into blocks whose values sum to zero, of at most five errors each; any five residues contain such a block (prefix sums). A block of k errors takes k − 1 pairs.
  - At most 4N/5 = 1,152 pairs on either C5 orbit, attained when all 1,440 values equal one generator. On average about 677.42; the exact value is a rational number computed by a 25-state recurrence.
- **D5 and A5 with ordinary pairs.** A chain into one reservoir: at most N − 1 pairs, (N − 1)(1 − 1/|H|) on average.
- **A better gadget (T11-04).** A frame triple clears two arbitrary errors with two seed applications. With it, the worst counts are 720 on C5, 360 on D5 and 60 on A5, plus at most one single on D5 and A5.

### T11-03: the fewest three-cycles (proved, model review; checked independently by the integrator)

- **The count.** Let c_odd(σ) be the number of cycles of odd length of σ, fixed points included. A three-cycle changes it by at most two, and the identity has N, so σ needs at least (N − c_odd(σ))/2 three-cycles. That many suffice:
  - an odd cycle of length 2r + 1 takes r three-cycles: (a1 … a2r+1) = (a1 a2 a3)(a1 a4 a5)…(a1 a2r a2r+1), chronologically;
  - even cycles come in pairs in an even permutation; reduce them to transpositions and use (a b)(c d) = (a c d)(a c b).
- **Worst case and average.** At most N/2 when 4 divides N; the committed loop needs up to 3N/4 − 1. For a uniform even σ the average is (N − H_N + H_{N/2}/2 − 1/(N − 1))/2.
- **The transport.** g = w1 w2 u⁻¹ takes X, Y and Z to A, B and C:
  - w1 takes X to A;
  - w2 consists of letters that fix A and takes w1(Y) to B;
  - u is a tree path that takes C to w2(w1(Z)) and fixes A and B.

  A cycle costs s_o + 2(e1_o + e_o + d_o), where e1_o is the largest distance to A with all letters: 18, 11, 9, 9, 7, 8, 8, 7, 7, 7, then 6 on O10 to O17, and on O18 to O34 5, 6, 6, 6, 5, 5, 5, 5, 6, 5, 5, 5, 6, 5, 5, 5, 5.
- **Fewer cycles are not always cheaper.** w1 lengthens every cycle. With the frame passes unchanged, always using this count gives at most 43,522,258 twists for every input but an average ceiling of 33,260,800, above the 32,201,480 of round 10. T11-05 uses it only where the frame group is trivial, and only when it is cheaper.
- **The integrator's check.** A breadth-first search over A_8 with all 112 three-cycles reaches 20,160 states, at most 4 apart, at the mean distance 649/210. Every distance equals (8 − c_odd)/2, and the mean equals the formula.

### T11-04: twisted cycles with two returns (proved, model review; re-computed by the integrator)

- **The twisted seed.** Let r be a retained word that fixes the positions A, B and C, with fibre values 1, y and x there. Then r⁻¹c'r has the fibre maps (αy, y⁻¹βx, x⁻¹γ). Choosing y = α⁻¹u and x = β⁻¹yv prescribes any first two maps u and v; neither alignment nor αβγ = 1 is needed.
- **A return at B, without another seed.** A word t that takes (A, B, C) to (A, C, B) is built from the round 10 setup and a tree path. Then t r_z t⁻¹ fixes every sticker of A and C and gives every value of the frame group at B. Combined with a return at C it gives r.
  - The lengths τ_o = |t| are 34, 15, 13, 9, 7, 9, 9, 7, 8, 7 and 8 on O00, O03, O06, O08, O11, O17, O22, O25, O28, O32 and O34.
  - These lengths are not claimed to be the shortest.
- **The costs.**
  - A twisted direct three-cycle costs W = s_o + 4h_o + 4τ_o + 2(e_o + d_o).
  - The frame triple r⁻¹c'rc'⁻¹ has the values (αyα⁻¹, y⁻¹βxβ⁻¹, x⁻¹) at A, B and C, so it sets two frames freely. Moved to two targets, with C as the reservoir, it costs 2s_o + 4h_o + 4τ_o + 2(e1_o + e_o + d_o).
- **A complete schedule** on an orbit with a nontrivial frame group:
  1. Run the position loop through A with twisted direct cycles.
  2. Whenever X or Y becomes correct, choose its incoming map (the edge A → X or X → Y) to restore its frame.
  3. Clear the misframed positions outside A, B and C that started in place, with C2 pairs, frame triples or ordinary pairs; an odd one left over takes an ordinary pair.
  4. Clean the buffers as in T10-04.
- **Why nothing is missed.** Each step finalizes at most two positions outside A, and a finalized position never re-enters the loop. Positions that start in place never enter it, and completed factors do not touch them, so step 3 handles exactly the errors that T10-03 found unvisited. Every completed factor keeps the seed's collateral. If F counts the initial frame errors outside A, B and C, then E[F] = (N − 3)(1 − 1/|H|)/N for a uniform input on G.
- **Counterexample.** Without step 3, the second return does not help: on the O03 input of T10-03 the loop runs no cycle and 3,596 frame errors remain.

### T11-05: the combined construction needs at most 20,866,844 twists (proved, model review; re-computed by the integrator)

- **The construction.**
  - On the 11 orbits with a nontrivial frame group: the schedule of T11-04. The misframed positions that start in place are cleared with ordinary pairs on O06 and O17, with C2 pairs on the seven C2 orbits and with frame triples on O28 and O34, whichever the twist count favours.
  - On the 24 orbits with a trivial frame group: for each input, the cheaper of the round 10 loop and the fewest three-cycles of T11-03.
- **The worst case.** The bound maximises the cost over the number m of moved positions, the number k of nontrivial cycles (k ≤ m/2, of the parity of m) and whether A moves, with the frame errors F bounded by the positions that do not move. This bounds every input; it does not claim that one input attains every maximum at once.
- **The bounds** (with the certified lengths of round 7):

| | Round 10 | Round 11 |
| --- | ---: | ---: |
| Every input, twists | 50,560,184 | 20,866,844 |
| On average over G, at most, twists | 32,201,480 | 17,010,356 |
| Every input, actions | 4,675,165 | 4,655,642 |
| On average over G, at most, actions | 2,988,213 | 3,126,229 |

- **What it gives.**
  - A gain of 29,693,340 twists (59%) over round 10, and of 50,647,700 (71%) over the construction of round 7. The bound is about 93 times the lower bound of 224,601 retained twists, and about 8 months at one twist per second (4,655,642 actions: about 54 days).
  - The construction was chosen for twists. Its average in actions is higher than that of round 10, whose construction keeps its own bounds.
  - The library is unchanged: the 35 replaced seeds and their inverses, created in 14,048 twists.
  - The largest orbit terms are now O10 (2.9 million twists, positions only), O28 (2.3 million) and O03 (2.1 million). The 11 orbits with frames need 7.2 million in all and the 24 without 13.7 million.
  - The bound is for R; the corrections C add at most 25,852 twists (AU-05).
- **The averages.** They are upper bounds on the expected length, as in round 10. They use the exact averages of the loop count and of F, charge the cleanup at its worst, and use E[min(U, V)] ≤ min(E[U], E[V]) on the orbits without frames.
- **Not implemented.** An implementation must carry out the whole schedule, including the positions that start in place and the collateral; replacing only the cycle constructor does not give this bound. Its acceptance tests should include:
  - an input whose positions are all in place but whose frames are wrong;
  - a closing three-cycle;
  - paired transpositions;
  - the non-abelian cleanup.

### T11-06: a floor for constructions from seed conjugates (proved, model review; re-computed by the integrator)

- **Positions.** Suppose orbit o changes only through conjugates of c' and c'⁻¹. Each application is then one three-cycle of positions, so an input permutation σ needs at least (N − c_odd(σ))/2 applications. Disjoint transpositions need N/2. Summed over the orbits: 88,260 applications, or 13,956,600 twists charged for the seeds.
- **Frames.** Take the positions in place and a nonidentity frame at every position.
  - The supports of the seed applications form a hypergraph. A connected component with v positions needs at least v − 1 applications, because returning to the identity needs as many splits of cycles as merges.
  - On C2 and D5, choose every frame in the nonzero class of the abelianization. Each component then has even size, and with at least three positions it has at least four, so at least 3v/4 applications.
  - On C5, choose every frame equal to one generator. Component sizes are then multiples of 5, which gives at least 4v/5 applications.
  - On A5, a component has at least three positions, which gives at least 2v/3.
  - These inputs lie in G.
- **The floor.** Summed over the orbits, with N/2 on the orbits without frames: 95,024 applications, or 15,643,520 twists charged for the seeds. The construction of T11-05 is within a factor 5,216,711/3,910,880, about 1.33, of this floor.
- **Scope.**
  - The floor counts seed applications times s_o, not the length of a freely reduced word: if c' = u⁻¹pu, conjugating it by u leaves p, which may be shorter.
  - Adding the orbits' worst cases is justified for cascades that solve one orbit at a time. It is not established for an algorithm that uses the collateral of earlier seeds on purpose.
  - The lower bound for every retained word stays 224,601 (M2-02).

### T11-07: exact average lengths (open)

The averages above are upper bounds on the expected length, not exact averages:
- they charge the longest setup and return words;
- they bound the average of a minimum by the minimum of the averages;
- they charge the cleanup at its worst.

An exact average needs fixed tie-breaking for transports, factorizations, returns and cancellation, and then the joint distribution. The synthetic inputs are not uniform samples of G (M2-05).

### Where M2 stands after round 11

- **Proved (model review).**
  - A construction of R needs at most 20,866,844 twists for every input (at most 17,010,356 on average over G), or 4,655,642 actions. That is about 93 times the lower bound for retained words, and about 1.334 times the floor of T11-06: the twists charged for the seeds by any cascade that solves one orbit at a time with conjugates of these seeds. The comparison does not cover a construction that uses the collateral of earlier seeds on purpose, or free cancellation (corrected in round 12, R12-01).
  - The corrections add at most 25,852 twists.
- **What this means.** With these seeds and a cascade that solves one orbit at a time, a better schedule can save at most about a quarter of the bound, counted in seed charges. A larger gain needs shorter seeds, above all for O10, O28, O03, O01 and O18, or a construction outside that class: one not built from conjugates of the seeds, one that uses their collateral on purpose, or one whose conjugators cancel against the seeds.
- **Implemented.** The kit as it runs is unchanged: its bound stays 71,516,526 twists, and the measured runs 4.5 × 10^7 (section 12).
- **Next (not run).**
  - Implement the schedule of T11-04 and T11-05 with the acceptance tests above, and replay the two synthetic inputs with the stage 03 checker.
  - Shorter seeds for the largest terms.
- **For the owner.** Whether a lattice phase of about 4.7 million actions, or 2 × 10^7 twists, counts as replicable.

### Reproduction (round 11)

From the repository root, `python -B`.

T11-01: the distance arrays and the coverage of the transported values on O06 and O34 (31 seconds). Output:

```text
coverage 6 7190
coverage 34 7140
e1A [18, 11, 9, 9, 7, 8, 8, 7, 7, 7, 6, 6, 6, 6, 6, 6, 6, 6, 5, 6, 6, 6, 5, 5, 5, 5, 6, 5, 5, 5, 6, 5, 5, 5, 5]
e1C [19, 11, 9, 9, 7, 7, 8, 7, 7, 7, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 5, 5, 5, 5, 5, 5, 5, 6, 5, 5, 5]
e [18, 11, 9, 9, 8, 8, 8, 7, 7, 7, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 5, 5, 6, 6, 6, 6, 4]
f [18, 11, 9, 9, 8, 8, 8, 7, 7, 7, 7, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 5, 6, 5, 5, 6, 5, 5, 5, 6, 6, 5, 6, 5]
```

```python
import sys, gzip, json
import numpy as np
sys.path.insert(0, 'research/jumbling/restoration/03-retained-group-and-lattice-restorer/scripts')
from rlib import Retained
R = Retained('.')
J = json.load(gzip.open('research/jumbling/restoration/09-general-method/data/realign.json.gz', 'rt'))
first = R.psorted[R.poff[:-1]]
def mul(*xs):
    p = tuple(range(len(xs[0])))
    for x in xs: p = tuple(x[i] for i in p)
    return p
def inv(x): return tuple(x.index(i) for i in range(len(x)))
rows = []
for o, t in enumerate(R.trees):
    pcs = R.orbit_pieces(o); n = len(pcs)
    ix = np.full(R.np, -1, dtype=np.int32); ix[pcs] = np.arange(n)
    fs = first[pcs]
    A, B = map(int, t['buffers']); C = int(t['third'])
    ai, bi, ci = [int(ix[p]) for p in (A, B, C)]
    pm = {}
    for m in range(1, 1201):
        s, d = R.move(m)
        j = np.minimum(np.searchsorted(s, fs), len(s)-1)
        hit = s[j] == fs; img = fs.copy(); img[hit] = d[j[hit]]
        p = ix[R.sp[img]]
        pm[m] = p; pm[-m] = np.argsort(p)
    def bfs(root, gens):
        dist = np.full(n, -1, dtype=np.int32)
        nxt = np.zeros(n, dtype=np.int32)
        dist[root] = 0; front = np.array([root]); depth = 0
        while len(front):
            depth += 1; new = []
            for m in gens:
                y = pm[-m][front]; y = y[dist[y] < 0]
                dist[y] = depth; nxt[y] = m; new.append(y)
            front = np.unique(np.concatenate(new))
        return dist, nxt
    da, _ = bfs(ai, pm); dc, _ = bfs(ci, pm)
    de, _ = bfs(bi, [m for m,p in pm.items() if p[ai] == ai])
    df, nf = bfs(bi, [m for m,p in pm.items() if p[ci] == ci])
    assert min(da) >= 0 and min(dc) >= 0
    assert np.count_nonzero(de < 0) == np.count_nonzero(df < 0) == 1
    assert de[ai] == df[ci] == -1
    rows.append([int(max(x)) for x in (da, dc, de, df)])
    if o not in (6, 34): continue
    lab = {A:list(R.atlas[o]['frames'][0]), B:list(R.atlas[o]['frames'][1])}
    for p, fr in zip(t['positions'], t['frames']): lab.setdefault(p, list(fr))
    H = {tuple(lab[C].index(x) for x in fr)
         for p,fr in zip(t['positions'], t['frames']) if p == C}
    beta = tuple(lab[C].index(int(x)) for x in
                 R.map_through(lab[B], J['orbits'][o]['choice']['word']))
    for X in map(int, pcs):
        if X == C: continue
        y = int(ix[X]); g = []
        while y != bi:
            m = int(nf[y]); g.append(m); y = int(pm[m][y])
        a = tuple(lab[B].index(int(x)) for x in R.map_through(lab[X], g))
        delta = tuple(lab[C].index(int(x)) for x in R.map_through(lab[C], g))
        lam = mul(a, beta)
        assert {mul(lam,x,inv(lam)) for x in H} == H
        assert {mul(delta,inv(x),inv(delta)) for x in H} == H
    print('coverage', o, (n-1)*len(H))
for j, name in enumerate(('e1A', 'e1C', 'e', 'f')):
    print(name, [r[j] for r in rows])
```

T11-02: the C5 schedule, its worst count, the ceiling of its average and the average times 10^6, rounded down. Output:

```text
1152 678 677418870
```

```python
from itertools import combinations
from collections import deque
from fractions import Fraction

def step(st, a):
    v = tuple(sorted(st + (a,)))
    for k in range(1, len(v)+1):
        for idx in combinations(range(len(v)), k):
            if sum(v[i] for i in idx) % 5 == 0:
                return tuple(x for i,x in enumerate(v) if i not in idx), k-1
    assert len(v) <= 4
    return v, 0

seen = {()}; queue = deque([()]); trans = {}
while queue:
    st = queue.popleft()
    trans[st] = [step(st,a) for a in range(5)]
    for v,c in trans[st]:
        if v not in seen: seen.add(v); queue.append(v)
assert len(seen) == 25
count, cost, worst = {():1}, {():0}, {():0}
for j in range(1440):
    nc, nw, nm = {}, {}, {}
    for st,n in count.items():
        for v,c in trans[st]:
            nc[v] = nc.get(v,0) + n
            nw[v] = nw.get(v,0) + cost[st] + c*n
            nm[v] = max(nm.get(v,-1), worst[st]+c)
    count, cost, worst = nc, nw, nm
assert count[()] == 5**1439
E = Fraction(cost[()], count[()])
print(worst[()], -(-E.numerator//E.denominator),
      E.numerator*10**6//E.denominator)
# 1152 678 677418870
```

T11-04: the swap lengths τ_o and the number of checked pairs of prescribed values (|H|²) per orbit. Output:

```text
0 34 4
3 15 4
6 13 100
8 9 4
11 7 4
17 9 25
22 9 4
25 7 4
28 8 25
32 7 4
34 8 3600
```

```python
import sys, gzip, json
import numpy as np
sys.path.insert(0, 'research/jumbling/restoration/03-retained-group-and-lattice-restorer/scripts')
from rlib import Retained, invword
R = Retained('.')
J = json.load(gzip.open('research/jumbling/restoration/09-general-method/data/realign.json.gz', 'rt'))
first = R.psorted[R.poff[:-1]]
def path(t,n):
    w = []
    while t['parent'][n] >= 0:
        w.append(int(t['move'][n])); n = t['parent'][n]
    return w[::-1]
def mul(*xs):
    p = tuple(range(len(xs[0])))
    for x in xs: p = tuple(x[i] for i in p)
    return p
def inv(x): return tuple(x.index(i) for i in range(len(x)))
for rec in J['orbits']:
    if rec['H'] == '1': continue
    o = rec['orbit']; t = R.trees[o]; pcs = R.orbit_pieces(o); n = len(pcs)
    ix = np.full(R.np,-1,dtype=np.int32); ix[pcs] = np.arange(n)
    A,B = map(int,t['buffers']); C = int(t['third'])
    ai,bi,ci = [int(ix[p]) for p in (A,B,C)]
    fs = first[pcs]; pm = {}
    for m in range(1,1201):
        s,d = R.move(m); j = np.minimum(np.searchsorted(s,fs),len(s)-1)
        hit = s[j] == fs; img = fs.copy(); img[hit] = d[j[hit]]
        p = ix[R.sp[img]]
        if p[ai] == ai: pm[m] = p; pm[-m] = np.argsort(p)
    dist = np.full(n,-1,dtype=np.int32); nxt = np.zeros(n,dtype=np.int32)
    dist[bi] = 0; front = np.array([bi]); depth = 0
    while dist[ci] < 0:
        new = []; depth += 1
        for m,p in pm.items():
            y = p[front]; y = y[dist[y] < 0]
            dist[y] = depth; nxt[y] = -m; new.append(y)
        front = np.concatenate(new); assert len(front)
    a,w = ci,[]
    while a != bi:
        m = int(nxt[a]); w.append(m); a = int(pm[m][a])
    z = bi
    for m in w: z = int(pm[m][z])
    swap = w + invword(path(t,t['positions'].index(int(pcs[z]))))
    labs = [np.array(f,dtype=np.int64) for f in R.atlas[o]['frames']]
    assert [int(R.sp[R.map_through(f,swap)[0]]) for f in labs] == [A,C,B]
    H = {}; HB = set()
    for node,pos in enumerate(t['positions']):
        if pos != C: continue
        rw = path(t,node); cf = R.map_through(labs[2],rw)
        assert all(np.array_equal(R.map_through(labs[i],rw),labs[i]) for i in (0,1))
        x = tuple(labs[2].tolist().index(int(j)) for j in cf); H[x] = rw
        bw = swap + rw + invword(swap)
        assert all(np.array_equal(R.map_through(labs[i],bw),labs[i]) for i in (0,2))
        HB.add(tuple(labs[1].tolist().index(int(j)) for j in R.map_through(labs[1],bw)))
    assert HB == set(H)
    sw = rec['choice']['word'] if rec['choice'] else (
        invword(t['relocation']) + t['seed'] + t['relocation'])
    P = R.word_perm(sw)
    al,be,ga = [tuple(labs[(i+1)%3].tolist().index(int(P[s])) for s in labs[i])
                for i in range(3)]
    for va in H:
        for vb in H:
            y = mul(inv(al),va,al); x = mul(inv(be),y,vb,be)
            assert y in H and x in H
            assert mul(al,y,inv(al)) == va
            assert mul(inv(y),be,x,inv(be)) == vb
    print(o,len(swap),len(H)**2)
```

T11-05: the bounds. Output:

```text
twists 20866844 17010356
actions 4655642 3126229
gain 29693340
factor 20866844/224601
creation 14048
```

```python
import gzip,json
from fractions import Fraction as F
T = json.load(gzip.open('assets/execution_trees.json.gz','rt'))
J = json.load(gzip.open('research/jumbling/restoration/09-general-method/data/realign.json.gz','rt'))
S = {r['orbit']:r['s'] for r in J['orbits']}
e = [18,11,9,9,8,8,8,7,7,7]+[6]*18+[5,5]+[6]*4+[4]
a = [18,11,9,9,7,8,8,7,7,7,6,6,6,6,6,6,6,6,5,6,6,6,5,5,5,5,6,5,5,5,6,5,5,5,5]
b = {0:19,3:9,6:8,8:7,11:6,17:6,22:6,25:5,28:5,32:5,34:5}
f = {0:18,3:9,6:8,8:7,11:6,17:6,22:5,25:5,28:5,32:5,34:5}
H = {0:2,3:2,6:10,8:2,11:2,17:5,22:2,25:2,28:5,32:2,34:60}
h = {0:5,3:3,6:4,8:3,11:3,17:2,22:3,25:2,28:2,32:2,34:3}
tau = {0:34,3:15,6:13,8:9,11:7,17:9,22:9,25:7,28:8,32:7,34:8}
ans = {'twists':[0,F()], 'actions':[0,F()]}
for o,t in enumerate(T):
    n = len(set(t['positions']))+2
    D = e[o]+max(t['depth']); K = a[o]+D
    EC = (F(n)-sum((F(1,j) for j in range(1,n,2)),F())-F(1,n-1))/2
    EB = (F(n-4)+sum((F(1,j) for j in range(1,n+1)),F())
          +F(2,n)+F(1,n*(n-1)))/2
    if o not in H:
        mu = min(EB*(S[o]+2*D), EC*(S[o]+2*K))
        ans['twists'][0] += n//2*(S[o]+2*K)
        ans['twists'][1] += mu
        ans['actions'][0] += max((3*n//4-1)*(1+2*D), n//2*(1+2*K))
        ans['actions'][1] += mu-(S[o]-1)*EC
        continue
    def tariffs(z):
        P = 2*z+2*h[o]+2*f[o]
        Q = (2*z+2*h[o]+2*(b[o]+f[o]) if H[o] == 2 else
             2*z+4*h[o]+4*tau[o]+2*K)
        W = z+4*h[o]+4*tau[o]+2*D
        cleanup = 4*z+4*h[o]
        if o in (6,34): cleanup += 4*z+(16 if o == 6 else 10)
        return P,Q,W,cleanup
    ps,qs,_,_ = tariffs(S[o])
    paired = qs < 2*ps  # Choice is fixed by twist tariffs for both measures.
    for mode,z in (('twists',S[o]),('actions',1)):
        P,Q,W,C = tariffs(z)
        def fc(j): return j//2*Q+(j%2)*P if paired else j*P
        worst = fc(n-3)
        for m in range(3,n+1):
            k = m//2; k -= (k-m)%2
            for i in (0,1):
                if i == 0 and m == n: continue
                worst = max(worst, (m+k-2*i)//2*W+fc(n-m-(1-i)))
        EF = F(H[o]-1,H[o])*F(n-3,n)
        per_error = max(F(P),F(Q,2)) if paired else F(P)
        ans[mode][0] += worst+C
        ans[mode][1] += EB*W+EF*per_error+C
for mode,(worst,mean_bound) in ans.items():
    print(mode,worst,-(-mean_bound.numerator//mean_bound.denominator))
print('gain',50560184-ans['twists'][0])
print('factor',F(ans['twists'][0],224601))
print('creation',2*sum(S.values()))
# twists 20866844 17010356
# actions 4655642 3126229
# gain 29693340
# factor 20866844/224601
# creation 14048
```

T11-06: the floor, with frames and for positions alone. Output:

```text
95024 15643520
88260 13956600
```

```python
import gzip,json
J = json.load(gzip.open('research/jumbling/restoration/09-general-method/data/realign.json.gz','rt'))
q = []
for r in J['orbits']:
    n,H = r['N'],r['H']
    q.append(3*n//4 if H in ('C2','D5') else
             4*n//5 if H == 'C5' else
             2*n//3 if H == 'A5' else n//2)
print(sum(q),sum(x*r['s'] for x,r in zip(q,J['orbits'])))
print(sum(r['N']//2 for r in J['orbits']),
      sum(r['N']//2*r['s'] for r in J['orbits']))
# 95024 15643520
# 88260 13956600
```

The integrator's check of T11-03 on A_8: the number of states, the largest distance, the mean distance and the formula. Output:

```text
20160 4 649/210 649/210 True
```

```python
# Integrator's independent check of T11-03 on A_8: BFS over all 3-cycles from the identity.
from itertools import permutations, combinations
from fractions import Fraction as F
from collections import deque
n = 8
def compose(p, q):  # apply p then q (chronological)
    return tuple(q[p[i]] for i in range(n))
gens = []
for a, b, c in combinations(range(n), 3):
    for x, y, z in ((a, b, c), (a, c, b)):
        g = list(range(n)); g[x], g[y], g[z] = y, z, x
        gens.append(tuple(g))
assert len(gens) == 112
idp = tuple(range(n))
dist = {idp: 0}; q = deque([idp])
while q:
    p = q.popleft()
    for g in gens:
        r = compose(p, g)
        if r not in dist:
            dist[r] = dist[p] + 1; q.append(r)
def codd(p):
    seen, c = set(), 0
    for i in range(n):
        if i in seen: continue
        L, j = 0, i
        while j not in seen:
            seen.add(j); j = p[j]; L += 1
        c += L % 2
    return c
assert all(dist[p] == (n - codd(p)) // 2 for p in dist)
H = lambda m: sum(F(1, j) for j in range(1, m + 1))
mean = F(sum(dist.values()), len(dist))
formula = (n - H(n) + H(n // 2) / 2 - F(1, n - 1)) / 2
print(len(dist), max(dist.values()), mean, formula, mean == formula)
```

## 17. Closing audit (round 12)

Rounds 9 to 11 changed METHOD.md and the summaries again on the same day, and twice lowered the bound for a construction of the lattice cascade. Round 12 audited them before the repository is shown to other people. It asked three questions:
- **Q1.** Does every claim labelled proved, certificate, computed or measured, in METHOD.md, the four READMEs, the open problems X1 and X2 and the wiki log, have a source that supports it at that strength, and is every number attached to the right object?
- **Q2.** Can the results on the lattice phase be stated as one proposition?
- **Q3.** Which sentences of the root README and of the jumbling overview could a careful reader with a mathematical background take as stronger than the evidence?

A Codex senior review returned ten findings, all verified:
- one statement for Q2, R12-01 (`major`, because it changes a published comparison);
- nine defects for Q1 and Q3, R12-02 to R12-10: five `major` and four `minor`.

It found that every published bound of the lattice phase reproduces exactly. The integrator checked each finding against the cited sources and adopted all ten. It re-ran the three computations of round 9, whose code is now in section 14, and the split of the action bound below. The corrections are made in METHOD.md (the last section lists the changes), in the summaries, and in sections 12, 14 and 16. The wiki log is append-only, so its entry for this round corrects the earlier entries.

### R12-01: the lattice phase in one statement (proved, model review; re-computed by the integrator)

**Proposition.** Fix the execution trees, the return words and the round 7 selection of the 35 seeds, 31 of them replaced. Each seed cycles its three buffers on its own orbit, with collateral only on orbits that come later in the validated cascade order; on D5 and A5 the pairs are re-indexed. Then for every retained configuration in G, the retained cascade R alone needs at most:

| R built as | Every input, twists | Uniform input on G, expected twists at most | Every input, actions | Implemented |
| --- | ---: | ---: | ---: | --- |
| the kit as it runs (round 7) | 71,516,526 | 46,191,246 | 4,841,151 | yes, as a separate runner |
| the construction with the shorter single (round 7) | 71,514,544 | 46,189,354 | 4,840,993 | no |
| direct three-cycles (round 10) | 50,560,184 | 32,201,480 | 4,675,165 | no |
| the schedule of round 11 | 20,866,844 | 17,010,356 | 4,655,642 | no |

- **Actions** count single twists and applications of stored macros. The library is the 35 seeds and their inverses, and creating its 70 macros costs another 14,048 twists. For the kit, the 4,841,151 actions are 370,713 macro applications and 4,470,438 single twists.
- **The corrections.** For a reachable lattice input the spanning correction library adds at most 25,852 twists (AU-05): 71,542,378 in all with the kit, and 20,892,696 with the schedule of round 11.
- **The schedule of round 11** needs all of its parts, including the positions that start in place with wrong frames and the buffer cleanup. A new constructor of cycles alone does not give its bound.
- **What runs.** The kit with the replaced seeds is `restore_realigned.py`, a runner separate from the driver: the lattice phase of `gdrive.py` (`restore_lattice_late.py`) runs the committed cascade, which needs at most 356,666,844 twists by the exact count of R7-03. The kit was measured on the two committed synthetic inputs, which are not uniform on G: 45,009,802 and 44,940,982 twists.
- **The floor of T11-06** concerns cascades that solve one orbit at a time with conjugates of the seeds and do not use the collateral of earlier seeds on purpose. For them some input needs at least 95,024 seed applications, or 15,643,520 twists charged for the seeds. This is not a lower bound on the length of a freely reduced word. The bound of round 11 is 5,216,711/3,910,880, about 1.334, times this floor.
- **The general lower bound.** Some input needs at least 224,601 retained twists (M2-02). It counts retained words only; a solution through jumble twists is not covered.

What changed: the summaries said "within a factor of about 1.33 of the fewest seed applications possible with these seeds", and the wiki said "any construction from conjugates of the same seeds". The summaries now name the class of cascades, and the wiki entry of this round corrects the earlier one. METHOD.md says that, within that class, a larger gain needs shorter seeds; a construction outside the class is not covered by the floor.

### Corrections (R12-02 to R12-10)

| Finding | Severity | What was wrong | Correction |
| --- | --- | --- | --- |
| R12-02 | `major` | The root README and the jumbling overview said that the puzzle's group becomes infinite. The study of 9 October proves that a jumble twist and K⁺ generate an infinite group of rotations, a statement about grip closure, not about reachable configurations; whether R(I_a) is finite is open (X1). | Both say that the rotations of the caps now generate an infinite group, and that whether the menu I_a reaches infinitely many configurations is open. |
| R12-03 | `major` | The root README said that every twist was replayed on the exact simulator by an independent checker, and the restoration overview that every result was checked by an independent replay on the exact simulator. `verify_late.py` replays the off-lattice words and the corrections on the exact simulator, but evaluates the retained cascade as a permutation program (lines 344 to 348). | The root README, both jumbling READMEs and METHOD.md section 6.1 now say: checked independently; the off-lattice words and the corrections were replayed on the exact simulator, and the retained cascade was evaluated as an exact permutation program on all 259,800 sticker labels. "Replayed by the stage 03 checker" becomes "checked by". |
| R12-04 | `major` | Section 14 said that the integrator re-ran three inline computations, but it held no code, and it called T9-04 a certificate. The older `flt.py` of stage 04 selects caps by whole components of a floating-point 3-face graph, so it does not reproduce T9-04, which also finds caps 434 and 463 that hold only 59 and 70 of the 78 pieces. | Section 14 now holds the code of T9-01, T9-02 and T9-04 with their outputs, re-run from the repository root; T9-04 now also asserts the six witnesses. T9-04 is called an exact computation that can be replayed, not a certificate. |
| R12-05 | `major` | R7-02 said that `realign.py` records and checks every part of the certificate except α, β and γ, and the 09 README that `data/realign.json.gz` holds the relocation. The file has no asset or source digests, relocation words, ordered triples or buffer labels; of the relocation it has only the length. | R7-02 and the 09 README list what is checked and recorded, and what is not archived. The action and length results are not affected. |
| R12-06 | `major` | Section 14 said twice that X1 and X2 stay open, while the open problems close X2 for I_a by T9-02. | X1 stays open; X2 is closed for I_a in its stated form (a repeatable word) by T9-02; Proposition HB still covers only the model orbits 0 to 9. |
| R12-07 | `minor` | Several averages in METHOD.md and R7-03 read as exact means. They are upper bounds on the expected length (T11-07). | "Expected length at most …, for a uniform input on G". |
| R12-08 | `minor` | The root README said that no step knocks a lattice piece off, and the overview said "no collateral". By M1-03 the first jumble letter of each step at stop62 takes at least 3,097 − 860 = 2,237 lattice pieces off for a while. | The root README: each step ends with every piece that was on the lattice back on it. The overview: no new off-lattice pieces at the end of any step. |
| R12-09 | `minor` | The overview and the wiki called the action counts "macro actions". They count single twists too: the kit's 4,841,151 actions are 370,713 macro applications and 4,470,438 single twists. | "Actions (single twists and stored-macro applications)"; the creation of the library is charged apart. |
| R12-10 | `minor` | METHOD.md and the wiki called `test_fallback.py` two end-to-end runs. Both tests run without `--lattice`: they reach the lattice and pass `verify_u`, with the heuristic steps skipped or timed out, and do not run the lattice phase. | METHOD.md section 3 says so. |

### Reproduction (round 12)

The split of the action bound of R7-03 into macro applications and single twists, and the ratio of R12-01 (from the repository root). Output:

```text
construction seed calls 370697 other twists 4470296 actions 4840993
kit seed calls 370713 other twists 4470438 actions 4841151
ratio 5216711/3910880 1.3338969745939533
with C 71542378 20892696
```

```python
import gzip,json
from fractions import Fraction as F
T=json.load(gzip.open('assets/execution_trees.json.gz','rt'))
H={0:2,3:2,6:10,8:2,11:2,17:5,22:2,25:2,28:5,32:2,34:60}
h={0:5,3:3,6:4,8:3,11:3,17:2,22:3,25:2,28:2,32:2,34:3}
for single in ('construction','kit'):
  seed=0; other=0
  for t in T:
    o=t['orbit'];N=len(set(t['positions']))+2;d=max(t['depth'])
    seed+=3*N//2-3; other+=(3*N//2-3)*2*d
    if o in H:
      seed+=(N-2)*4+2; other+=(N-2)*(2*h[o]+4*d)+2*h[o]
      if o in (6,34):
        if single=='construction': seed+=4; other+=(16 if o==6 else 10)
        else:
          PB=(2,2*h[o]); P=(4,2*h[o]+4*d); seed+=2*(PB[0]+P[0]); other+=2*(PB[1]+P[1])
  print(single,'seed calls',seed,'other twists',other,'actions',seed+other)
print('ratio',F(20866844,15643520),float(F(20866844,15643520)))
print('with C',71516526+25852,20866844+25852)
```

## 18. Is the reachable set finite? (rounds 13 and 13b)

Round 9 left X1 open (section 14). Round 13 asked the senior review for two things: a sound closure for model orbit 10, which repairs the filter of T9-01, and anchors that control the common pose of a high component. The review returned four requirements for such work instead of results (T13-01 to T13-04, all `minor`). The integrator adopted them and met the first for the certificate of Proposition HB.

Round 13b put the same questions as a research request, with the four requirements as acceptance rules. It returned eight results (T13b-01 to T13b-08, all `minor`): proofs, exact computations and exact counterexamples. The integrator re-ran every computation from the repository root, got the reported outputs, and adopted all eight. One program needed a fix, noted under "Reproduction (round 13b)" below.

X1, P6 (below) and the uniform bound of M1 stay open. Proposition HB is unchanged, and all its cap decisions are now exact.

### Round 13: four requirements (proved, model review)

- **T13-01: exact pruning.** A closure is a height certificate only if every cap decision that it prunes with is exact.
  - `refexact.py` decides in floating point whether a cap holds or excludes a posed region, with margin 10⁻⁷, and exactly only near ties. `innerstraddle.py` records straddlers from floating-point signs.
  - **Met for REFX by the integrator.** The certificate REFX-EXACT (`refx_exact.py`, stage 04) recomputes over Q(√5) all 861,600 cap decisions of the ten closures: "holds" and "excludes" for each of the 718 classes and each of the 600 grips. It finds no difference. The smallest floating-point distance from a tie was 8.1 × 10⁻⁴. So every decision behind Proposition HB is exact.
  - `innerstraddle.py` is used only by the stopped run REFX2-11 for model orbit 10, which proves nothing. It is not re-certified.
- **T13-02: realized paths.** Lemma R follows only the letters that move the tracked piece.
  - So an abstract path may need letters that it does not show, like the first letter of the T9-01 witness. Failing to replay its visible edges does not make it spurious.
  - An abstract path counts as reachable only with a complete admissible realization. Without one, abstract growth gives no unbounded reachable family.
- **T13-03: P6.** Call a maximal connected set of pieces of height at least 4 in the 3-face graph a *high component*, and its largest member height its height.
  - "No unstraddled cap of another branch holds a component of height at least 6 whole" is then equivalent to the bound 6 on every left height. Just before the first piece reaches height 8, Corollary FN1 makes the raising letter a jumble at an unstraddled cap of another branch that holds the piece's component whole. Conversely, such a cap admits a jumble that raises the component.
  - The same holds piece by piece:

    > **P6.** At every reachable configuration, every cap of a grip of another branch that wholly contains a piece of left height at least 6 is straddled.

  - P6 holds if and only if every reachable left height is at most 6. So P6 is the main question, not a smaller one. T9-04 checks it at H6.
- **T13-04: potentials.** A potential that the moves of the blockers never increase bounds nothing unless its level sets bound the absolute height; the zero function is such a potential.
  - A potential that no admissible letter increases is an invariant, because every letter has an admissible inverse.
  - So a proposed potential must come with the inequality that ties it to the height.

### Round 13b: eight results

Notation:
- **B** is the open inscribed ball, of radius |n|.
- **INV** (stage 04, `PROOFS.md` section 2.3): every inner piece is on the lattice or belongs to the complete cohort of an out grip c, inside cap c. Theorem A of stage 04 proves that INV holds at every reachable configuration.
- **NB(c)** = {d ≠ c : 2κ² < NN(NN + n_c·n_d)}: the 16 grips at 15.52 and 25.24 degrees from c, which is N15(c) ∪ N25(c). The blocking lemma of stage 04 says that while c is out, no twist at a grip of NB(c) is admissible.

| Finding | Status | Result |
| --- | --- | --- |
| T13b-01 | proved; computed | The pieces that meet B are exactly the 9,720 pieces of the model orbits −1 (the 600 centre pieces), 0, 1, 3 and 6. With the fixed core they tile B at every reachable configuration: a twist moves whole pieces on one side of its cut, and its rotation maps B onto itself. Every cut meets B in a 3-disc of squared radius (11808 + 3936√5)/15625. A piece on the lattice, or in a frame of a grip's class, never straddles that grip's cut. |
| T13b-02 | proved; computed | Caps c and d can meet inside B only if d ∈ NB(c) (Cauchy–Schwarz). At every reachable configuration, a piece crosses the cut of d inside B if and only if d ∈ NB(c) for some out grip c. For each of the four jumble cosets at grip 0, the 53 pieces of cap 0 that meet B straddle 22 cuts, 16 of them inside B; the other 6 crossings lie outside B. So the gate inside B gives no blocking beyond the blocking lemma. |
| T13b-03 | computed | The admissible word [(0, 12), (2, 1), (0, 45), (2, 1)] from solved returns all 12,120 pieces of the model orbits −1, 0, 1, 2, 3 and 6 to their identity poses, and the piece 275 of model orbit 10 also has the identity pose. Yet cap 3, admissible at solved, is now blocked by the piece 2877 of model orbit 5 (digest `d58ca6a9…`, 790 pieces off). So no predicate of the inner cohorts and the tracked pose alone decides admissibility exactly: an abstraction on those data can only over-approximate. |
| T13b-04 | proved (soundness); closure not computed | A sound repair of the T9-01 filter. Record for each out grip c the coset of its cohort, one of four. A letter at d needs d outside the cuts that every recorded cohort straddles. A grip that newly holds the tracked piece branches into "not out" and its four cosets, constrained by the admissibility of the letter before it, not after. Every admissible history maps to a path. Each coset's cohort, the 68 inner pieces of cap 0, straddles 22 cuts: the 16 of NB and 6 more. The closure for model orbit 10 was not computed, so Proposition HB is not extended. |
| T13b-05 | computed; proved | The 78 height-6 pieces of H6 share one pose (pose 1138 of the H6 table) and form exactly one 3-face component, with 161 edges. No other piece is a 3-face neighbour: the only candidates, 71 pieces at height 4 in poses 313 and 556, are separated from it by exact bounding boxes in all 5,538 pairs. While the island has height at least 4, its members and relative poses stay fixed (AL). A cap that holds some but not all members of a connected high component is straddled by a member. |
| T13b-06 | proved | Let U6 say that every 3-face component with a piece of height at least 6 has one height, and B42 that every cap of another branch that holds a height-4 piece with a height-2 3-face neighbour is straddled. Then U6 holds if and only if B42 does. Proposition HB settles B42 for an edge whose height-4 end lies in an orbit bounded by 4, or whose height-2 end lies in an orbit bounded by 2; whether every 4–2 edge has such an end is open. U6 alone would not settle P6. |
| T13b-07 | computed | The exact straddler sets of the six caps of T9-04 at H6, fixed by SHA-256 digests. Cap 274: 339 pieces, all at height 2. Cap 326: 89, at height 2. Cap 340: 142 at height 2 and 16 at height 4. Cap 430: 114, at height 2. Cap 434: 226 at height 2, 18 at height 4 and 19 at height 6. Cap 463: 333 at height 2 and 8 at height 6. |
| T13b-08 | computed | From H6, the letter (433, 45), the inverse of the jumble that made H6, keeps all 78 island pieces inside caps 274, 326, 340 and 430. It clears cap 326 completely: all 89 straddlers leave its cut and none replaces them. But the island falls to height 4 in branch 0, the branch of 326 (digest `bd0cbd33…`). So keeping the island inside a cap does not anchor that cap's straddlers; an anchor must also keep the height. |

### What it gives

- **Where blocking comes from.**
  - Inside B the only blocking is that of the blocking lemma (T13b-02).
  - A cap that cuts a high island is blocked by the island itself (T13b-05). This covers caps 434 and 463 of T9-04, which hold 59 and 70 of the 78 pieces.
  - So the open case of P6 is a cap of another branch that holds a whole high island and lies outside NB(c) for every out grip c. Blocking such a cap needs a crossing outside B.
- **A new fact (integrator, exact; code below).**
  - H6 is reached by the fixture records 1 to 723 and the jumble (433, 12). That jumble raised the island from height 4 in branch 0 to height 6 in branch 4, the branch of grip 433, and grip 433 is still out.
  - All six caps of T9-04 lie in NB(433): 340 and 430 at 15.52 degrees from grip 433, and 274, 326, 434 and 463 at 25.24 degrees. Their branches are 3, 0, 1, 2, 3 and 1.
  - So at H6 every cap of another branch that holds a piece of the island is blocked by the cohort of the grip that raised it.
  - If at every reachable configuration every cap of another branch that holds a high island lay in NB(c) for some out grip c, P6 would follow. Round 14 (section 19) asked whether the raising grip gives this at the moment of a raise, and whether it lasts while the island stays high.
- **Status.** X1, P6 and the uniform bound of M1 stay open. Proposition HB still covers the model orbits 0 to 9, now with exact cap decisions. T13b-04 gives a sound system for an exact closure experiment on model orbit 10 (not run).

### Reproduction (round 13b)

From the repository root, with Python 3.13 and NumPy 2.5. The certificate REFX-EXACT runs from stage 04 (`python -I scripts/refx_exact.py <J> certs/q1/refx-exact.json`, about 30 seconds; stage 04 README). The programs below are the review's, re-run by the integrator.
- The NB check and T13b-01 to T13b-05 run in under 15 seconds each.
- T13b-07 and T13b-08 form one program, which replays the 724 letters of H6: about four minutes.
- One fix: the review's T13b-01 squared a `Q5` number with `**`, which `exact.Q5` does not support. The line `k=len(S)*d.KAPPA; q=k*k/dot(v,v)/d.NN` multiplies instead and gives the reported output.

The new fact: NB(433) and the six caps of T9-04. Output:

```text
16 [274, 320, 326, 340, 373, 390, 430, 434, 448, 451, 458, 463, 493, 518, 523, 532]
raiser branch 4
274 25.24 3 True
326 25.24 0 True
340 15.52 1 True
430 15.52 2 True
434 25.24 3 True
463 25.24 1 True
```

```python
import sys, math
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling']
import sim, trees as T
from exact import dot
C = sim.get_context(); d = C.data; g = 433
NB = [c for c in range(600) if c != g and 2*d.KAPPA*d.KAPPA < d.NN*(d.NN + dot(d.N[g], d.N[c]))]
print(len(NB), NB)
print('raiser branch', T.branch(tuple(d.N[g])))
for c in [274, 326, 340, 430, 434, 463]:
    a = math.degrees(math.acos(float(dot(d.N[g], d.N[c])) / float(d.NN)))
    print(c, round(a, 2), T.branch(tuple(d.N[c])), c in NB)
```

T13b-01, the pieces that meet B and the disc of a cut. Output:

```text
0 -1 600 Q5(14641,0,15625) -1
1 0 1200 Q5(175692,-58564,46875) -1
2 1 3600 Q5(761332,-58564,640625) -1
3 3 3600 Q5(146410,-58564,15625) -1
4 6 720 Q5(146410,-58564,15625) -1
5 2 2400 Q5(395307,-43923,296875) 1
outside_remaining 30 minimum_signature_bound 6 Q5(746691,-278179,124375)
ball_pieces 9720
NN_minus_cutdistance2 Q5(11808,3936,15625)
```

```python
import sys
sys.path[:0]=['research/jumbling/theory','research/jumbling']
import sim
from exact import Q5,dot,solve
C=sim.get_context(); d=C.data; r=C.regions
active=[(0,),(0,13),(12,13),(0,13,17),(0,10,35),(10,12,13)]
for o,a in enumerate(active):
 p=r.reps[o]; S=set(d.signature_list(p)); V=r.rep_vertices[o]
 u=[x[0] for x in solve(
  [[dot(d.N[c],d.N[e]) for e in a] for c in a],
  [[d.KAPPA] for c in a])]
 assert all(x.sign()>=0 for x in u)
 z=[sum((x*d.N[c][j] for c,x in zip(a,u)),Q5(0)) for j in range(4)]
 assert all((dot(d.N[c],z)-d.KAPPA).is_zero() for c in a)
 assert all(dot(n,z)<=d.NN and
  ((dot(n,z)-d.KAPPA).sign()>=0 if c in S
   else (dot(n,z)-d.KAPPA).sign()<=0)
  for c,n in enumerate(d.N))
 ratio=dot(z,z)/d.NN
 if o<5:
  b=[sum((v[j] for v in V),Q5(0))/len(V) for j in range(4)]
  w=[(999*zj+bj)/1000 for zj,bj in zip(z,b)]
  assert dot(w,w)<d.NN
  assert all(dot(n,w)<d.NN and
   ((dot(n,w)-d.KAPPA).sign()>0 if c in S
    else (dot(n,w)-d.KAPPA).sign()<0)
   for c,n in enumerate(d.N))
 print(o,int(d.orbit_id[p]),r.report['orbit_sizes'][o],
       ratio,(ratio-Q5(1)).sign())
lo=[]
for o,p in enumerate(r.reps[6:],6):
 S=d.signature_list(p)
 v=[sum((d.N[c][j] for c in S),Q5(0)) for j in range(4)]
 k=len(S)*d.KAPPA; q=k*k/dot(v,v)/d.NN
 assert Q5(1)<q
 lo.append((q,o))
q,o=min(lo)
print('outside_remaining',len(lo),'minimum_signature_bound',o,q)
print('ball_pieces',sum(r.report['orbit_sizes'][:5]))
print('NN_minus_cutdistance2',d.NN-d.KAPPA*d.KAPPA/d.NN)
```

T13b-02, the crossings inside B for the four cosets at grip 0. Output:

```text
NB [1, 3, 5, 6, 7, 8, 10, 12, 13, 14, 17, 18, 21, 27, 35, 38] core pieces in cap 53
12 global 22 inside_B 16 global_only [4, 15, 25, 33, 51, 65]
24 global 22 inside_B 16 global_only [11, 33, 39, 51, 67, 72]
25 global 22 inside_B 16 global_only [15, 26, 39, 41, 65, 72]
27 global 22 inside_B 16 global_only [4, 11, 25, 26, 41, 67]
```

```python
import sys
sys.path[:0]=['research/jumbling/theory','research/jumbling']
import sim,restore as R
from exact import Q5,dot,solve,matvec
from sim.kernel import Evaluator,pullback_form
C=sim.get_context(); d=C.data; rg=C.regions; M=R.menu('I_a')
NB=[c for c in range(1,600)
 if 2*d.KAPPA*d.KAPPA<d.NN*(d.NN+dot(d.N[0],d.N[c]))]
assert len(NB)==16
active=[(0,),(0,13),(12,13),(0,13,17),(0,10,35)]
zrep=[]
for a in active:
 u=[x[0] for x in solve(
  [[dot(d.N[c],d.N[e]) for e in a] for c in a],
  [[d.KAPPA] for c in a])]
 zrep.append([sum((x*d.N[c][j] for c,x in zip(a,u)),Q5(0))
              for j in range(4)])
P=[int(p) for p in d.cap_members(0) if int(rg.orbit_of[p])<5]
Z={p:matvec(C.kplus.matrix(int(rg.transport[p])),
            zrep[int(rg.orbit_of[p])]) for p in P}
print('NB',NB,'core pieces in cap',len(P))
for item in (12,24,25,27):
 G=R.element(M,0,item); whole=[]; inside=[]
 ZG={p:matvec(G,Z[p]) for p in P}
 VG={p:[matvec(G,v) for v in rg.vertices(p)] for p in P}
 for c in range(600):
  ev=Evaluator(pullback_form(G,d.N[c]))
  strad=[p for p in P if ev.classify(rg.vform(p))[0]=='straddle']
  if not strad: continue
  whole.append(c); found=False
  for p in strad:
   z=ZG[p]; hz=dot(d.N[c],z)-d.KAPPA
   if hz.is_zero() and dot(z,z)<d.NN:
    found=True; break
   for v in VG[p]:
    hv=dot(d.N[c],v)-d.KAPPA
    if hz.sign()*hv.sign()>=0: continue
    t=-hz/(hv-hz); x=[a+t*(b-a) for a,b in zip(z,v)]
    assert (dot(d.N[c],x)-d.KAPPA).is_zero()
    if dot(x,x)<d.NN:
     found=True; break
   if found: break
  if found: inside.append(c)
 assert inside==NB and len(whole)==22
 assert all(2*d.KAPPA*d.KAPPA>=
  d.NN*(d.NN+dot(d.N[0],d.N[c]))
  for c in range(1,600) if c not in NB)
 print(item,'global',len(whole),'inside_B',len(inside),
       'global_only',sorted(set(whole)-set(inside)))
```

T13b-03, the counterexample. Output:

```text
inner_identity 12120 tracked_piece 275 cap 3
solved admissible after_W blocked blocker 2877 signs [-1, 1]
off 790 digest d58ca6a97eb6c88b6c173a2bab511e233a7ef9c0ac5ceddce9ff38d8c8199b77
```

```python
import sys,numpy as np
sys.path[:0]=['research/jumbling/theory','research/jumbling']
import sim,restore as R
from exact import transpose
from sim.kplus import to_tuple
from sim.kernel import Evaluator,pullback_form
C=sim.get_context(); M=R.menu('I_a')
x=sim.State(C,filtered=False,menu=M)
z=sim.State(C,filtered=False,menu=M)
W=[(0,12),(2,1),(0,45),(2,1)]
assert R.element(M,0,45)==to_tuple(transpose(R.element(M,0,12)))
assert R.element(M,2,1)==to_tuple(transpose(R.element(M,2,1)))
for c,i in W:
 o=z.apply(sim.Twist(C,c,R.element(M,c,i),'menu',{'menu':M.name}))
 assert o.applied and not o.uncertain and o.moved==3097
inner=np.isin(C.data.orbit_id,[-1,0,1,2,3,6])
assert int(inner.sum())==12120 and np.all(z.pose_id[inner]==0)
p,c,q=275,3,2877
assert C.data.orbit_id[p]==10 and z.pose_id[p]==0
assert Evaluator(pullback_form(z.pose(p),C.data.N[c])).classify(
 C.regions.vform(p))[0]=='in'
a=x.classify(c,need_inside=False,stop_at_straddle=True)
b=z.classify(c,need_inside=False,stop_at_straddle=True)
assert a.status=='admissible' and b.status=='blocked'
assert not b.uncertain and b.straddle['piece']==q
v=C.regions.vform(q)
ev=Evaluator(pullback_form(z.pose(q),C.data.N[c]))
assert C.data.orbit_id[q]==5 and ev.classify(v)==('straddle',2,0,1)
assert [ev.sign_row(v[0],v[1][i]) for i in (0,1)]==[-1,1]
h=z.digest()
assert h=='d58ca6a97eb6c88b6c173a2bab511e233a7ef9c0ac5ceddce9ff38d8c8199b77'
assert z.off_lattice_count()==790
tw=sim.Twist(C,c,R.element(M,c,12),'menu',{'menu':M.name})
assert x.apply(tw).applied
assert not z.apply(tw).applied and z.digest()==h
print('inner_identity',int(inner.sum()),'tracked_piece',p,'cap',c)
print('solved',a.status,'after_W',b.status,'blocker',q,'signs',[-1,1])
print('off',790,'digest',h)
```

T13b-04, the cuts straddled by each coset's cohort. Output:

```text
inner 68 cosets [(0, 12), (12, 12), (24, 12), (25, 12), (27, 12)]
NB [1, 3, 5, 6, 7, 8, 10, 12, 13, 14, 17, 18, 21, 27, 35, 38]
shape 12 blocked 22 extra [4, 15, 25, 33, 51, 65]
shape 24 blocked 22 extra [11, 33, 39, 51, 67, 72]
shape 25 blocked 22 extra [15, 26, 39, 41, 65, 72]
shape 27 blocked 22 extra [4, 11, 25, 26, 41, 67]
```

```python
import sys
sys.path[:0]=['research/jumbling/theory','research/jumbling']
import sim,restore as R
from exact import matmul,transpose
from sim.kplus import to_tuple
from sim.kernel import Evaluator,pullback_form
C=sim.get_context(); M=R.menu('I_a')
S=[int(q) for q in C.data.cap_members(0)
   if int(C.data.orbit_id[q]) in {0,1,2,3,6}]
mat=[R.element(M,0,i) for i in range(60)]
reps=[]; cosets=[]
for i,g in enumerate(mat):
 for n,r in enumerate(reps):
  if C.kplus.index_of_matrix(to_tuple(matmul(transpose(mat[r]),g)))>=0:
   cosets[n].append(i); break
 else:
  reps.append(i); cosets.append([i])
print('inner',len(S),'cosets',[(r,len(x)) for r,x in zip(reps,cosets)])
NB=set().union(*(C.data.signature(q) for q in S))-{0}
print('NB',sorted(NB))
for r in reps[1:]:
 bs=[]
 for c in range(600):
  ev=Evaluator(pullback_form(mat[r],C.data.N[c]))
  if any(ev.classify(C.regions.vform(q))[0]=='straddle' for q in S):
   bs.append(c)
 assert NB<=set(bs)
 print('shape',r,'blocked',len(bs),'extra',sorted(set(bs)-NB))
```

T13b-05, the island of H6 (reads the H6 table of stage 04). Output:

```text
height6_pose_ids [1138]
signature_edges 161 connected 78
external_pose_ids [313, 556] piece_counts {313: 10, 556: 61}
candidate_pairs 5538 strict_separations 5538
```

```python
import sys,gzip,json,hashlib
from pathlib import Path
from collections import Counter
sys.path[:0]=['research/jumbling/theory','research/jumbling']
import sim,trees as T
from exact import matmul,matvec,transpose
from sim.kplus import matrix_from_json
p=Path('research/jumbling/restoration/04-card5-lattice-invariants/data/t-T723x433-12.json.gz')
raw=p.read_bytes()
assert hashlib.sha256(raw).hexdigest()=='a577af0c9722233741e08eb82f815cc74ab5b401a37918cc2f29014bd1140cdd'
a=json.loads(gzip.decompress(raw)); C=sim.get_context()
mat=[matrix_from_json(m) for m in a['poses']]; pid=a['pose_id']
left=[T.factors(m)[0] for m in mat]; h=[T.dist(l) for l in left]
assert Counter(h[i] for i in pid)=={0:57042,2:115850,4:4150,6:78}
high=[q for q,i in enumerate(pid) if h[i]==6]
assert sorted({pid[q] for q in high})==[1138]
sig={frozenset(C.data.signature(q)):q for q in high}; edges=set()
for S,q in sig.items():
 for c in S:
  r=sig.get(S-{c})
  if r is not None: edges.add(tuple(sorted((q,r))))
adj={q:[] for q in high}
for q,r in edges: adj[q].append(r); adj[r].append(q)
seen={min(high)}; queue=list(seen)
for q in queue:
 for r in adj[q]:
  if r not in seen: seen.add(r); queue.append(r)
assert len(edges)==161 and len(seen)==78
near=[i for i in range(len(mat)) if i!=1138 and
      T.dist(T.qmul(T.qconj(left[1138]),left[i]))<=2]
assert near==[313,556]
other=[q for q,i in enumerate(pid) if i in near]
assert Counter(pid[q] for q in other)=={313:10,556:61}
assert all(h[i]==4 for i in near)
rel={i:matmul(transpose(mat[1138]),mat[i]) for i in near}
vs={q:C.regions.vertices(q) for q in high}
vs.update({q:[matvec(rel[pid[q]],v) for v in C.regions.vertices(q)]
           for q in other})
def bounds(V):
 lo,hi=list(V[0]),list(V[0])
 for v in V[1:]:
  for j,x in enumerate(v):
   if (x-lo[j]).sign()<0: lo[j]=x
   if (x-hi[j]).sign()>0: hi[j]=x
 return lo,hi
boxes={q:bounds(V) for q,V in vs.items()}; count=0
for q in high:
 lo,hi=boxes[q]
 for r in other:
  lo2,hi2=boxes[r]
  assert any((hi[j]-lo2[j]).sign()<0 or
             (hi2[j]-lo[j]).sign()<0 for j in range(4))
  count+=1
assert count==5538
print('height6_pose_ids',[1138])
print('signature_edges',len(edges),'connected',len(seen))
print('external_pose_ids',near,'piece_counts',dict(sorted(Counter(pid[q] for q in other).items())))
print('candidate_pairs',count,'strict_separations',count)
```

T13b-07 and T13b-08, one program: the straddler sets at H6, then the letter (433, 45). Output:

```text
274 78 339 [(2, 339)] 41 d42a8bc768cc163613dda7c07b2c4a135d1c1766a50039a43acb542a5f146b58
326 78 89 [(2, 89)] 28 577d988fd54c61165eaf6c5076ed624040527f7f12d231d7d1e5199903e00b42
340 78 158 [(2, 142), (4, 16)] 32 b94ae50e5bff9e5e0ab4b456695bdb6e1b8bbd5f7d5f2999a34a8c33bc15dfbd
430 78 114 [(2, 114)] 28 a55096aed5d20796571266e89ea0a476e6ba5a0e6700dfe145d7fc99c77839af
434 59 263 [(2, 226), (4, 18), (6, 19)] 41 e2ff24ffc369a5c3098d1005fad17151a8fb592d2d0fda3295b899c7426a2ba8
463 70 341 [(2, 333), (6, 8)] 58 b9dc9b617050ff4ef5f0dbcbea3b207990f07a717d5cf4d64f449dfc7f038b94
clear 274 top_inside 78 removed 176 of 339 status blocked
clear 326 top_inside 78 removed 89 of 89 status admissible
clear 340 top_inside 78 removed 95 of 158 status blocked
clear 430 top_inside 78 removed 31 of 114 status blocked
height_branch [(4, 0)] digest bd0cbd3337ef16085cba3498916b3c683f04735b5cc3737012c6414af422f7d7
```

```python
import sys,json,hashlib
from pathlib import Path
from collections import Counter
import numpy as np
sys.path[:0]=['research/jumbling/theory','research/jumbling']
import sim,trees as T,restore as R
from sim.kernel import Evaluator,pullback_form
C=sim.get_context(); M=R.menu('I_a')
s=sim.State(C,filtered=False,menu=M)
records=json.loads(Path('research/jumbling/fixtures/wj-I_a.json').read_text())['journal']['records']
for rec in records[:723]:
 o=s.apply(sim.Twist.from_record(C,rec))
 assert o.applied and o.moved==rec.get('moved',o.moved)
assert s.apply(sim.Twist(C,433,R.element(M,433,12),'menu',{'menu':M.name})).applied
assert s.digest()=='fe917f9a77c69cf037ce5c894debdec2448b73575ed776d5c529ccfaf5bc7c39'
left={i:T.factors(s._pose_matrix(i))[0] for i in set(s.pose_id.tolist())}
h={i:T.dist(l) for i,l in left.items()}
b={i:T.branch(l) for i,l in left.items()}
assert Counter(h[int(i)] for i in s.pose_id)=={0:57042,2:115850,4:4150,6:78}
top=[q for q,i in enumerate(s.pose_id) if h[int(i)]==6]
off=np.flatnonzero(s._kpose()[s.pose_id]<0).tolist()
rows={}
for c in [274,326,340,430,434,463]:
 ev={i:Evaluator(pullback_form(s._pose_matrix(i),C.data.N[c]))
     for i in set(s.pose_id[off].tolist())}
 rows[c]=[]
 for q in off:
  i=int(s.pose_id[q])
  if ev[i].classify(C.regions.vform(q))[0]=='straddle':
   rows[c].append([q,int(C.data.orbit_id[q]),h[i],b[i]])
 held=sum(ev[int(s.pose_id[q])].classify(C.regions.vform(q))[0]=='in'
          for q in top)
 hist=sorted(Counter(r[2] for r in rows[c]).items())
 ball=sum(r[1] in {-1,0,1,3,6} for r in rows[c])
 digest=hashlib.sha256(json.dumps(rows[c],separators=(',',':')).encode()).hexdigest()
 print(c,held,len(rows[c]),hist,ball,digest)
# rows[c] contains every requested piece/orbit/height/branch record.
# For a literal listing, additionally print(json.dumps(rows)).
from exact import transpose
from sim.kplus import to_tuple
caps=[274,326,340,430]
def classify_piece(c,q):
 return Evaluator(pullback_form(s.pose(q),C.data.N[c])).classify(
  C.regions.vform(q))[0]
assert all(classify_piece(c,q)=='in' for c in caps for q in top)
assert R.element(M,433,45)==to_tuple(transpose(R.element(M,433,12)))
o=s.apply(sim.Twist(C,433,R.element(M,433,45),'menu',{'menu':M.name}))
assert o.applied and not o.uncertain
pairs=set()
for i in set(s.pose_id[top].tolist()):
 l=T.factors(s._pose_matrix(i))[0]
 pairs.add((T.dist(l),T.branch(l)))
assert pairs=={(4,0)} and T.branch(tuple(C.data.N[326]))==0
expected={274:(176,'blocked'),326:(89,'admissible'),
          340:(95,'blocked'),430:(31,'blocked')}
for c in caps:
 assert all(classify_piece(c,q)=='in' for q in top)
 removed=sum(classify_piece(c,r[0])!='straddle' for r in rows[c])
 cl=s.classify(c,need_inside=False,stop_at_straddle=True)
 assert not cl.uncertain and (removed,cl.status)==expected[c]
 print('clear',c,'top_inside',78,'removed',removed,
       'of',len(rows[c]),'status',cl.status)
assert s.digest()=='bd0cbd3337ef16085cba3498916b3c683f04735b5cc3737012c6414af422f7d7'
print('height_branch',sorted(pairs),'digest',s.digest())
```

## 19. Is a raised island trapped by the grip that raised it? (round 14)

Round 13b left one open case of P6: a cap of another branch that holds a whole high island and lies outside NB(c) for every out grip c (section 18). At H6 every cap of another branch that holds a piece of the island lies in NB(433), and grip 433 raised the island. Round 14 asked three things: whether the raising grip always gives this (RA, stated for the pieces of the island), whether it lasts while the island stays high, and for any progress on B42. The review returned four results (T14-01 to T14-04, all `minor`). The integrator re-ran the four programs from the repository root, got the reported outputs exactly, and adopted all four. It also checked that the 60 menu elements at a grip form a group, whose 12 retained elements lie in K⁺ and whose other 48 do not (grips 0, 326 and 433), which the trap lemma below uses.

| Finding | Status | Result |
| --- | --- | --- |
| T14-01 | computed | RA, stated for single pieces, is false at height 6. After the fixture records 1 to 723 and the jumble (433, 14), the 78 pieces of the island rise from height 4 in branch 0 to height 6 in branch 4. Piece 8412 (model orbit 25) lies wholly in cap 559, of branch 3, and 559 is outside NB(433): NN(NN + n₄₃₃·n₅₅₉) − 2κ² = (−309168 − 61072√5)/15625 < 0. But cap 559 holds only 38 of the 78 pieces and is straddled by the other 40 (for example by piece 6743). So P6 is not contradicted. |
| T14-02 | computed | A census of the 724 fixture records. 50 letters raise pieces, 11,724 piece raises in all, every one from height 2 to 4; no recorded letter reaches height 6. RA for raises to height 4 fails at 21 of the 50 letters, with 1,670 pairs of a raised piece and a cap of another branch outside NB of the raiser; the first is at record 182. After the prefixes 723 and 724, all 48 jumbles at grip 433 were applied. Twelve of them (items 14, 21, 23 and 43 to 51) violate RA at height 6, with 456 pairs per prefix, yet every cap of another branch that holds a raised piece is straddled. |
| T14-03 | proved; computed | The trap lemma (below). Its hypotheses hold exactly for H6, with g = 433 and h = 4. |
| T14-04 | computed | At T723 and at H6, nine exact 3-face contacts join a height-4 piece of model orbit 9 to a height-2 piece of orbit 4, 7 or 8. Proposition HB, applied to their ends, does not exclude a raise. But every cap of another branch that holds one of the three height-4 pieces lies in NB of an out grip (12, 517 or 525), so no letter raises these contacts there. This is partial progress on B42 (T13b-06), not an exhaustive check. |

### The trap lemma (T14-03; proved, model review)

Assume INV. Write Λg for the group of the 60 menu elements at grip g and A4g for its 12 retained elements.

> **Trap lemma.** Let X0 be a reachable configuration, g a grip that is not out at X0, and K a connected set of pieces, all at left height h ≥ 4 in a branch other than g's, wholly inside cap g. Suppose that a jumble at g is admissible at X0, and that for every A in Λg \ A4g, every cap other than cap g that wholly holds A(K) lies in NB(g). Then from that jumble until g first returns, only letters at g move K, and every member of K stays at height h + 2 in g's branch. When g first returns, every member is back at height h.

Proof (the review's). Let A be the product of the letters at g since its departure, in order. By INV, g is out exactly while A is not retained, and by induction K has pose A times its pose at X0. The tree facts then give height h + 2 in g's branch. By Lemma FN, an admissible letter that moves one member of K moves all of it. A cap that holds only part of K is straddled by a member (T13b-05). A cap other than g that holds all of K lies in NB(g) by the hypothesis, so it is blocked while g is out (blocking lemma). A letter at g keeps K inside cap g and updates A. At the first return A lies in A4g, which is contained in K⁺, so the height is h again. Letters elsewhere are not restricted.

The hypothesis needs a fresh departure of g. For a grip that is already out when K arrives in its cap, the product A and the island's own rotation since its arrival can differ, and the lemma says nothing.

At H6 (g = 433, h = 4, X0 = T723, where 433 is not out), the hypotheses hold exactly. The 48 non-retained orientations put the island at height 6 in branch 4, and the 12 retained ones at height 4. For every non-retained orientation, four or five caps (433 included) hold the whole island, and every one of them other than 433 lies in NB(433). The 68 inner pieces of cap 433 are all on the lattice at T723 and all off it at H6.

### What it gives

- RA, stated for single pieces, is false at heights 4 and 6 (T14-01, T14-02). The statement that holds at H6 is about whole islands, and the trap lemma uses it.
- P6 holds for the H6 island throughout its first height-6 excursion: a cap that holds part of the island is straddled by it, and a cap of another branch that holds all of it is blocked by the out raiser 433. So no continuation from H6 that keeps the island at height 6 the whole way reaches height 8.
- Still open after round 14 (taken up in section 20):
  - a continuation that first lowers the island to height 4 and then raises it again;
  - an island that enters the cap of a grip that is already out (an arrival), where the trap lemma does not apply;
  - whether the hypothesis of the trap lemma holds at every raise by a fresh departure;
  - B42 in general (T14-04 is partial).
- Status: X1, P6 and the uniform bound of M1 stay open. Proposition HB is unchanged.

### Reproduction (round 14)

From the repository root, with Python 3.13 and NumPy 2.5: `python3 -I -B -` with each program on standard input. On the integrator's machine T14-03 took 14 seconds, and T14-01, T14-04 and T14-02, which replay the fixture, took 193, 200 and 255 seconds.

T14-01: the piece 8412 and cap 559 after (433, 14).

```python
import sys
from collections import Counter
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling',
               'research/jumbling/fixtures']
import sim, trees as T, wj, restore as RS
from exact import matvec, dot, affine_rank
from sim.kernel import Evaluator, pullback_form

C = sim.get_context(); fixture, M = wj.load(C, 'I_a')
s = sim.State(C, filtered=False, menu=M)
for rec in fixture['journal']['records'][:723]:
    assert s.apply(sim.Twist.from_record(C, rec), journal=False).applied
assert s.digest() == 'bd0cbd3337ef16085cba3498916b3c683f04735b5cc3737012c6414af422f7d7'
before = T.factors(s.pose(8412))[0]
assert s.apply(sim.Twist(C, 433, RS.element(M,433,14),
    'menu', {'menu':M.name}), journal=False).applied
after = T.factors(s.pose(8412))[0]
assert (T.dist(before),T.branch(before),T.dist(after),T.branch(after)) == (4,0,6,4)
assert T.branch(tuple(C.data.N[559])) == 3

def classify(p):
    return Evaluator(pullback_form(s.pose(p), C.data.N[559])).classify(C.regions.vform(p))

d = C.data
gap = d.NN*(d.NN+dot(d.N[433],d.N[559]))-2*d.KAPPA*d.KAPPA
assert gap.sign() < 0 and classify(8412) == ('in',7)
hs = [0 if p.kidx >= 0 else T.dist(T.factors(p.matrix)[0]) for p in s._poses]
top = [p for p,i in enumerate(s.pose_id) if hs[int(i)] == 6]
assert len(top) == 78 and len(set(s.pose_id[top].tolist())) == 1
assert classify(6743) == ('straddle',3,2,0)
V = {p:{tuple(matvec(s.pose(p),v)) for v in C.regions.vertices(p)} for p in top}
edges = []
for i,p in enumerate(top):
    for q in top[i+1:]:
        common = list(V[p] & V[q])
        if len(common) >= 4 and affine_rank(common) == 3:
            edges.append((p,q))
seen = {top[0]}
while True:
    n = len(seen)
    for p,q in edges:
        if p in seen or q in seen: seen.update([p,q])
    if len(seen) == n: break
assert len(edges) == 161 and len(seen) == 78
print('heights/branches', (T.dist(before),T.branch(before),T.dist(after),T.branch(after)))
print('gap', gap)
print('cap559', dict(Counter(classify(p)[0] for p in top)))
print('internal straddler',6743,classify(6743))
print('connected',len(seen),'edges',len(edges))
print('digest',s.digest())
```

Output:

```text
heights/branches (4, 0, 6, 4)
gap Q5(-309168,-61072,15625)
cap559 {'straddle': 40, 'in': 38}
internal straddler 6743 ('straddle', 3, 2, 0)
connected 78 edges 161
digest fd67aa19c068d9b25d122729082ce8b8cb57d0ea50a83114250bd1322b0dcc59
```

T14-02: the census of the fixture records and of the 96 jumbles at 433 (about four minutes).

```python
import sys
from collections import Counter
from functools import lru_cache
sys.path[:0]=['research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import numpy as np
import sim, trees as T, wj, restore as RS
from exact import matvec
from sim.kernel import rows_form, sign5

C=sim.get_context(); fixture,M=wj.load(C,'I_a')
N1=C.data.N1.astype(object); N2=C.data.N2.astype(object)
B=[T.branch(tuple(n)) for n in C.data.N]
@lru_cache(None)
def height(k):
    return 0 if k[0]=='K' else T.dist(T.factors(k[1])[0])
@lru_cache(None)
def branch(k):
    return T.branch(T.factors(k[1])[0])
def heights(s):
    return np.array([height(p.key) for p in s._poses])[s.pose_id]
@lru_cache(None)
def nb(g):
    return {d for d in range(600) if d!=g and
        (C.data.NN*(C.data.NN+T.qdot(C.data.N[g],C.data.N[d]))
         -2*C.data.KAPPA*C.data.KAPPA).sign()>0}
@lru_cache(None)
def holding(k,p):
    E,rows=rows_form([matvec(k[1],v) for v in C.regions.vertices(p)])
    cand=np.arange(600)
    for row in rows:
        x=np.array(row[:4],object); z=np.array(row[4:],object)
        a=125*(N1[cand]@x+5*(N2[cand]@z))-2*E*1452
        b=125*(N1[cand]@z+N2[cand]@x)-2*E*484
        cand=cand[np.array([sign5(int(u),int(v))>=0 for u,v in zip(a,b)])]
        if not len(cand): break
    return tuple(map(int,cand))
def clone(s):
    u=sim.State(C,filtered=False,menu=M)
    u._poses=list(s._poses); u._lookup=dict(s._lookup); u.pose_id=s.pose_id.copy()
    return u
def audit(s,old,g,minimum):
    now=heights(s); raised=np.flatnonzero((now>old)&(now>=minimum))
    foreign=[]; bad=[]
    for p in raised.tolist():
        k=s._poses[int(s.pose_id[p])].key; b=branch(k)
        for d in holding(k,p):
            if B[d]==b: continue
            foreign.append((p,d))
            if d not in nb(g): bad.append((p,d))
    return raised,foreign,bad

s=sim.State(C,filtered=False,menu=M)
events=pieces=six=0; failures=[]; bases={}
for t,rec in enumerate(fixture['journal']['records'][:724],1):
    old=heights(s)
    assert s.apply(sim.Twist.from_record(C,rec),journal=False).applied
    raised,foreign,bad=audit(s,old,rec['grip'],4)
    if len(raised):
        events+=1; pieces+=len(raised)
        six+=sum(height(s._poses[int(s.pose_id[p])].key)>=6 for p in raised)
    if bad: failures.append((t,len(bad)))
    if t==182:
        assert old[75338]==2 and heights(s)[75338]==4
        assert (75338,75) in bad and B[75]==0
        assert branch(s._poses[int(s.pose_id[75338])].key)==1
    if t in (723,724): bases[t]=clone(s)
print('fixture',events,pieces,six,len(failures),sum(n for t,n in failures))
print('failures',failures)
print('fixture724 digest',s.digest())
for t,base in bases.items():
    old=heights(base); failed=[]; totalbad=pairs=0; capcounts=Counter()
    for i in RS.jumble_items(M):
        u=clone(base)
        assert u.apply(sim.Twist(C,433,RS.element(M,433,i),'menu',
                                {'menu':M.name}),journal=False).applied
        raised,foreign,bad=audit(u,old,433,6)
        assert len(raised)==78 and set(old[raised].tolist())=={4}
        pairs+=len(foreign); totalbad+=len(bad)
        if bad: failed.append(i)
        capcounts.update(d for p,d in bad)
        for d in {d for p,d in foreign}:
            cl=u.classify(d,need_inside=False,stop_at_straddle=True)
            assert cl.straddle is not None and not cl.uncertain
    print('extensions',t,failed,totalbad,pairs)
    print('badcaps',dict(sorted(capcounts.items())))
print('all extension foreign caps blocked')
```

Output:

```text
fixture 50 11724 0 21 1670
failures [(182, 127), (210, 78), (221, 60), (271, 127), (300, 61), (354, 124), (401, 28), (423, 242), (431, 137), (455, 19), (510, 21), (521, 66), (528, 124), (536, 14), (550, 124), (568, 34), (628, 19), (659, 63), (668, 43), (676, 7), (723, 152)]
fixture724 digest 99ebeeac74791692d73c0ad2dfa1281acab464345d3568d8dd9b6bd48a62e652
extensions 723 [14, 21, 23, 43, 44, 45, 46, 47, 48, 49, 50, 51] 456 17844
badcaps {221: 38, 258: 38, 271: 38, 344: 38, 388: 38, 398: 38, 436: 38, 454: 38, 467: 38, 534: 38, 559: 38, 564: 38}
extensions 724 [14, 21, 23, 43, 44, 45, 46, 47, 48, 49, 50, 51] 456 17844
badcaps {221: 38, 258: 38, 271: 38, 344: 38, 388: 38, 398: 38, 436: 38, 454: 38, 467: 38, 534: 38, 559: 38, 564: 38}
all extension foreign caps blocked
```

T14-03: the hypotheses of the trap lemma at H6, from the H6 table of stage 04.

```python
import sys, gzip, json
from pathlib import Path
from collections import Counter
import numpy as np
sys.path[:0]=['research/jumbling/theory','research/jumbling']
import sim, restore as RS, trees as T
from exact import matmul, matvec, transpose, dot, affine_rank
from sim.kplus import matrix_from_json, to_tuple
from sim.kernel import Evaluator, pullback_form, rows_form

C=sim.get_context(); M=RS.menu('I_a'); d=C.data; g=433
path=Path('research/jumbling/restoration/04-card5-lattice-invariants/data/t-T723x433-12.json.gz')
with gzip.open(path,'rt') as f: tab=json.load(f)
P=[matrix_from_json(m) for m in tab['poses']]
s=sim.State(C,filtered=False,menu=M)
ids=np.array([s._intern_matrix(m) for m in P],np.int32)
s.pose_id[:]=ids[np.array(tab['pose_id'],np.int32)]
assert s.digest()==tab['digest']=='fe917f9a77c69cf037ce5c894debdec2448b73575ed776d5c529ccfaf5bc7c39'
hb=[(T.dist(T.factors(m)[0]),T.branch(T.factors(m)[0])) for m in P]
K=[p for p,i in enumerate(tab['pose_id']) if hb[i][0]==6]
assert len(K)==78 and {hb[tab['pose_id'][p]] for p in K}=={(6,4)}
Ei=transpose(RS.element(M,g,12))
P0={p:to_tuple(matmul(Ei,P[tab['pose_id'][p]])) for p in K}
assert len(set(P0.values()))==1
Q=next(iter(P0.values()))
assert (T.dist(T.factors(Q)[0]),T.branch(T.factors(Q)[0]))==(4,0)
Vs={p:{tuple(matvec(P0[p],v)) for v in C.regions.vertices(p)} for p in K}
V=sorted(set.union(*Vs.values()),key=repr); vf=rows_form(V)
assert len(V)==74
adj={p:set() for p in K}
for z,p in enumerate(K):
    for q in K[z+1:]:
        common=Vs[p]&Vs[q]
        if len(common)>=4 and affine_rank(list(common))==3:
            adj[p].add(q); adj[q].add(p)
seen={K[0]}; todo=[K[0]]
while todo:
    for q in adj[todo.pop()]-seen:
        seen.add(q); todo.append(q)
assert seen==set(K)
print('island',len(K),'vertices',len(V),'edges',sum(map(len,adj.values()))//2,'connected',len(seen))
NB={c for c in range(600) if c!=g and
    (d.NN*(d.NN+dot(d.N[g],d.N[c]))-2*d.KAPPA*d.KAPPA).sign()>0}
inner=[int(p) for p in d.cap_members(g) if int(d.orbit_id[p]) in {0,1,2,3,6}]
assert NB==set.union(*(set(d.signature(p)) for p in inner))-{g}
print('NB',sorted(NB))
whole=Counter(); jumbles=set(RS.jumble_items(M)); outside=[]
for j in range(len(M.items)):
    A=RS.element(M,g,j); AP=to_tuple(matmul(A,Q))
    h=(T.dist(T.factors(AP)[0]),T.branch(T.factors(AP)[0]))
    if j not in jumbles:
        assert h[0]==4
        continue
    assert h==(6,4)
    holds=[]
    for c in range(600):
        ev=Evaluator(pullback_form(A,d.N[c]))
        if all(ev.sign_row(vf[0],row)>=0 for row in vf[1]): holds.append(c)
    assert g in holds
    whole[len(holds)]+=1
    outside.extend((j,c) for c in holds if c!=g and c not in NB)
assert not outside
print('jumble_height6',len(jumbles),'retained_height4',len(M.items)-len(jumbles))
print('whole_holding_sizes',dict(sorted(whole.items())),'outside_NB',outside)
for label in ['H6','T723']:
    cl=s.classify(g,need_inside=True,stop_at_straddle=False)
    assert cl.straddle is None and not cl.uncertain
    qi=[int(p) for p in cl.inside if int(d.orbit_id[p]) in {0,1,2,3,6}]
    flags=s.lattice_flags(); nlat=sum(bool(flags[p]) for p in qi)
    assert len(qi)==68 and nlat==(0 if label=='H6' else 68)
    print(label,'inner',len(qi),'inner_lattice',nlat,'digest',s.digest())
    if label=='H6':
        A=RS.element(M,g,45); assert A==to_tuple(Ei)
        out=s.apply(sim.Twist(C,g,A,'menu',{'menu':M.name}),journal=False)
        assert out.applied and out.moved==2071
R=[dot(v,v)-dot(d.N[g],v)*dot(d.N[g],v)/d.NN for v in V]
hi=max(R)
print('axis_radius_squared_max',(hi.a,hi.b,hi.d))
```

Output:

```text
island 78 vertices 74 edges 161 connected 78
NB [274, 320, 326, 340, 373, 390, 430, 434, 448, 451, 458, 463, 493, 518, 523, 532]
jumble_height6 48 retained_height4 12
whole_holding_sizes {4: 12, 5: 36} outside_NB []
H6 inner 68 inner_lattice 0 digest fe917f9a77c69cf037ce5c894debdec2448b73575ed776d5c529ccfaf5bc7c39
T723 inner 68 inner_lattice 68 digest bd0cbd3337ef16085cba3498916b3c683f04735b5cc3737012c6414af422f7d7
axis_radius_squared_max (457739, -199012, 6250)
```

T14-04: the nine 4–2 contacts at T723 and H6.

```python
import sys
sys.path[:0]=['research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import sim, trees as T, wj, restore as RS
from exact import Q5 as Q, dot, matvec, matmul, transpose
from sim.kplus import to_tuple
from sim.kernel import Evaluator, pullback_form

POINTS=[
 (13336,2431,((916165,-119737,165000),(432280,-157471,82500),(78655,47681,82500),(-87545,99351,165000))),
 (13336,2436,((37315,22332,22000),(21127,-549,22000),(18801,34963,44000),(1257,17393,44000))),
 (13336,9349,((41149,21027,22000),(5701,-945,4000),(21870,11357,22000),(18339,8915,44000))),
 (158513,162150,((-404945,-17187,165000),(123435,-74757,55000),(8575,-101394,82500),(-310665,238291,82500))),
 (158513,162158,((-17861,-2957,8800),(-25269,-5549,44000),(-82711,-14415,44000),(37357,35153,44000))),
 (158513,171358,((-34658,-11537,22000),(5673,-20759,44000),(-19501,-4065,11000),(31975,38033,44000))),
 (169566,156793,((-25977,-11774,22000),(23123,2439,11000),(-52241,-37603,44000),(-1819,2729,8800))),
 (169566,162806,((-176195,-11163,82500),(687905,-114053,165000),(-491815,109366,82500),(-406495,221257,165000))),
 (169566,173052,((-41739,-29047,44000),(18943,1647,8800),(-85091,-22807,44000),(5647,5851,44000))),
]
EXPECTED={13336:[0,1,21,27],158513:[451,520,559],169566:[389,447,452]}
ANCHOR={13336:12,158513:517,169566:525}
C=sim.get_context(); d,kp=C.data,C.kplus
fixture,M=wj.load(C,'I_a'); s=sim.State(C,filtered=False,menu=M)
G={c:kp.matrix(0) for c in ANCHOR.values()}
for rec in fixture['journal']['records'][:723]:
    tw=sim.Twist.from_record(C,rec)
    assert s.apply(tw,journal=False).applied
    if tw.grip in G: G[tw.grip]=to_tuple(matmul(tw.matrix,G[tw.grip]))
assert s.digest()=='bd0cbd3337ef16085cba3498916b3c683f04735b5cc3737012c6414af422f7d7'
# INV identifies out status with the cumulative local rotation outside K+.
assert all(kp.index_of_matrix(g)<0 for g in G.values())
before={p:s.pose(p) for a,b,_ in POINTS for p in (a,b)}
for a,b,_ in POINTS:
    assert int(d.orbit_id[a])==9 and int(d.orbit_id[b]) in {4,7,8}
    assert T.dist(T.factors(before[a])[0])==4
    assert T.dist(T.factors(before[b])[0])==2
assert s.apply(sim.Twist(C,433,RS.element(M,433,12),'menu',
                        {'menu':M.name}),journal=False).applied
assert s.digest()=='fe917f9a77c69cf037ce5c894debdec2448b73575ed776d5c529ccfaf5bc7c39'
assert all(s.pose(p)==g for p,g in before.items())
# The three witness grips stay out: the last letter was at 433.
for a,b,record in POINTS:
    x=tuple(Q(*v) for v in record); faces=[]
    for p in (a,b):
        v=matvec(transpose(s.pose(p)),x); sig=set(d.signature(p)); tight=[]
        for c in range(600):
            z=dot(d.N[c],v)-d.KAPPA; side=1 if c in sig else -1
            assert z.sign()*side>=0
            assert (d.NN-dot(d.N[c],v)).sign()>0
            if z.is_zero(): tight.append(c)
        assert len(tight)==1
        c=tight[0]
        faces.append((tuple(matvec(s.pose(p),d.N[c])),1 if c in sig else -1))
    assert faces[0][0]==faces[1][0] and faces[0][1]==-faces[1][1]
for p,want in EXPECTED.items():
    branch=T.branch(T.factors(s.pose(p))[0]); vf=C.regions.vform(p)
    held=[c for c in range(600) if T.branch(tuple(d.N[c]))!=branch
          and Evaluator(pullback_form(s.pose(p),d.N[c])).classify(vf)[0]=='in']
    assert held==want
    anchor=ANCHOR[p]
    for c in held:
        assert c!=anchor
        assert (d.NN*(d.NN+dot(d.N[anchor],d.N[c]))-Q(2)*d.KAPPA*d.KAPPA).sign()>0
    print(p,held,'NB witness',anchor)
print('T723 and H6: 9 exact 4-2 contacts; all 10 foreign-cap incidences NB-covered')
```

Output:

```text
13336 [0, 1, 21, 27] NB witness 12
158513 [451, 520, 559] NB witness 517
169566 [389, 447, 452] NB witness 525
T723 and H6: 9 exact 4-2 contacts; all 10 foreign-cap incidences NB-covered
```

## 20. Height 8 is reachable: P6 is false (round 15)

Round 14 left three cases of P6 open: an island that drops to height 4 and rises again, an island that enters the cap of a grip that is already out (an arrival), and the hypothesis of the trap lemma (TH) at every raise by a fresh departure (section 19). Round 15 asked for a reduction of P6 to TH, a no-arrival premise and U6, for a decision on arrivals and on TH, and for the positions that the H6 island reaches after its first return. The review returned five results: T15-01 (`major`) and T15-02 to T15-05 (`minor`). The integrator re-ran the two programs from the repository root (T15-01 and T15-02 form one program), got the reported outputs exactly, replayed the word of T15-01 with its own code, and adopted all five.

| Finding | Status | Result |
| --- | --- | --- |
| T15-01 | computed | P6 is false, and so is the bound 6 on left heights. After the fixture records 1 to 723, the four letters (433, 12), (593, 58), (501, 22) and (593, 12) are admissible; the first gives H6 and the last takes seven pieces to height 8. Grip 593 is out from record 85 on. After (593, 58) the seven pieces (118527, 126380, 126382, 126384, 126385, 149644 and 149650, of model orbits 19 to 24) form one 3-face component at height 4 in branch 2, inside cap 593. Grip 501 is not out, and the 68 inner pieces of its cap are on the lattice; its jumble (501, 22) is a fresh raise of the component to height 6 in branch 1. Cap 593 then holds the whole component, is unstraddled, and lies outside NB(501): NN(NN + n₅₀₁·n₅₉₃) − 2κ² = (−309168 − 61072√5)/15625 < 0; it is not in NB of any other out grip either. So TH fails at an admissible fresh raise, and the jumble (593, 12) at the out grip 593 raises the component to height 8 in branch 2. Heights at the end: 56,457 pieces at 0, 116,435 at 2, 4,143 at 4, 78 at 6 (the H6 island, branch 4) and 7 at 8. |
| T15-02 | computed | Arrivals happen at height 4. From H6, the letters (593, 58), (501, 22), (501, 12) and (501, 40) bring the component back wholly into cap 593, at height 4 in branch 2, while 593 stays out; before the last letter one member straddled cap 593 and the other six were outside it. The next letter (593, 12) keeps it at height 4, since the branches agree. Appending (593, 45), the inverse of (593, 12), to the word of T15-01 gives an arrival from height 8 to 6 into the cap of the out grip 501. |
| T15-03 | proved | A conditional reduction: INV, TH at every fresh raise, U6 and NA+ give P6. NA+ says that while a grip g stays out, no letter at another grip makes a high component wholly contained in cap g, whether the component was high before or is born there. T15-01 and T15-02 refute TH and the no-arrival premise in the puzzle, so the reduction gives no bound. |
| T15-04 | proved | R(I_a) is finite if and only if the clean steps of the general method have a uniform length bound; this equivalence does not need hypothesis H of Card 5. Under H, a bound 2m on left heights gives at most (7200·D_m)^177120 configurations, with D_m = (4·16^m − 1)/3 (right heights are always 0). Height 8 is compatible with a larger finite bound, so it does not decide X1. |
| T15-05 | computed | The H6 island after its first return, followed up to its first raise to height 6, with blocking by other pieces ignored: 4 K⁺-classes of 7,200 positions each (28,800 positions at height 4) and 1,380 menu edges from the four representatives (96 drop to height 2, 564 stay at height 4, 720 raise to height 6). TH fails geometrically on 96 of the raises: by the raiser 430 in the first class, 258 and 398 in the second, 299 in the third, and none in the fourth. Ignoring blocking, the path (274, 12), (326, 12), (281, 12) from the return position reaches height 10; it is not claimed reachable. |

### Integrator checks

- An independent replay of the word of T15-01 with the integrator's own code (the third program below) gives the same digest and the same height histogram. The seven pieces at height 8 belong to model orbits 19 to 24, so Proposition HB (orbits 0 to 9) is unchanged.
- No single letter takes that state to height 10. A letter raises a piece from 8 to 10 only by a jumble at an unstraddled grip of another branch whose cap holds the piece whole (Corollary FN1). A cap that holds part of the seven-piece component is straddled by a member (T13b-05), and the six caps of another branch that hold all of it (529, 547, 555, 562, 580 and 585) are blocked (exact classify). The 78 pieces at height 6 can reach at most 8.
- The seven-piece component is the one that TWOSTEP of stage 04 raised to height 6 (`certs/q1/twostep-H6.json`: a 7-piece cluster, 4 to 4 to 6, by the letters (593, 58) and (501, 12)). TWOSTEP stopped after two letters; a third letter, at the out grip 593, reaches height 8.

### What it gives

- P6 and the bound 6 on left heights are false (T15-01). The mechanism needs two grips of different branches, neither in the other's NB: one departs early and stays out (593, from record 85), the other raises by a fresh jumble a component that lies in the first one's cap (501), and the first one, which nothing blocks, raises it again. The trap lemma (T14-03) does not apply, because its hypothesis TH fails at 501.
- Arrivals happen (T15-02), so the no-arrival premise of the conditional reduction (T15-03) is false too; T15-03 records which hypotheses would have been enough.
- X1 and the uniform bound of M1 stay open: left heights may still be bounded by a larger constant, and a height-8 witness is not an unbounded family (T15-04). Proposition HB is unchanged.
- The searches of stage 04 that found nothing above height 6 were evidence only; TWOSTEP stopped one letter short of this word.
- Next (round 16, section 21): whether left heights are bounded at all.

### Reproduction (round 15)

From the repository root, with Python 3.13 and NumPy 2.5: `python3 -I -B -` with each program on standard input; the third takes `research/jumbling` as its argument (`python3 -I -B - research/jumbling`). On the integrator's machine they took 199, 20 and 201 seconds.

T15-01 and T15-02: the word to height 8 with the certificates of each raise, then the arrivals.

```python
import sys
from collections import Counter
sys.path[:0]=['research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import sim, wj, restore as RS, trees as T
from exact import matmul, matvec, transpose, dot, affine_rank
from sim.kplus import to_tuple
from sim.kernel import Evaluator, pullback_form, rows_form
C=sim.get_context(); d=C.data
F,M=wj.load(C,'I_a')
s=sim.State(C,filtered=False,menu=M)
I=C.kplus.matrix(0); G={g:I for g in range(600)}; O=set()
K=[118527,126380,126382,126384,126385,149644,149650]
def apply(tw):
    out=s.apply(tw,journal=False)
    assert out.applied and not out.uncertain
    G[tw.grip]=to_tuple(matmul(tw.matrix,G[tw.grip]))
    if C.kplus.index_of_matrix(G[tw.grip])<0: O.add(tw.grip)
    else: O.discard(tw.grip)
def put(g,j):
    apply(sim.Twist(C,g,RS.element(M,g,j),'menu',{'menu':M.name}))
def heights():
    return [0 if p.kidx>=0 else T.dist(T.factors(p.matrix)[0]) for p in s._poses]
def component_hb():
    Q=s.pose(K[0]); assert all(s.pose(p)==Q for p in K)
    l=T.factors(Q)[0]
    return T.dist(l),T.branch(l)
def classification(p,g):
    return Evaluator(pullback_form(s.pose(p),d.N[g])).classify(C.regions.vform(p))[0]
def holding_caps():
    V={tuple(matvec(s.pose(p),v)) for p in K for v in C.regions.vertices(p)}
    vf=rows_form(V)
    return [g for g in range(600) if Evaluator(pullback_form(I,d.N[g])).classify(vf)[0]=='in']
def gap(a,b):
    return d.NN*(d.NN+dot(d.N[a],d.N[b]))-2*d.KAPPA*d.KAPPA
for t,rec in enumerate(F['journal']['records'][:723],1):
    apply(sim.Twist.from_record(C,rec))
    if t>=85: assert 593 in O
assert s.digest()=='bd0cbd3337ef16085cba3498916b3c683f04735b5cc3737012c6414af422f7d7'
put(433,12)
assert s.digest()=='fe917f9a77c69cf037ce5c894debdec2448b73575ed776d5c529ccfaf5bc7c39'
put(593,58)
assert component_hb()==(4,2) and 593 in O and 501 not in O
assert s.digest()=='9e8efe5ec9a7fa035319baf1fa88aac49ef2cc7c54a0d52a3faa616c667bcf03'
cl=s.classify(501,need_inside=True)
assert cl.status=='admissible' and not cl.uncertain
inner=[int(p) for p in cl.inside if int(d.orbit_id[p]) in {0,1,2,3,6}]
flags=s.lattice_flags()
assert len(inner)==68 and all(flags[p] for p in inner)
h=heights()
assert sorted(int(p) for p in cl.inside if h[int(s.pose_id[p])]>=4)==K
put(501,22)
assert component_hb()==(6,1) and {501,593}<=O
assert s.digest()=='ede1ebc8e264cd95db54cc816e9fcb1c68f591491c2300d23eb99c71e61628fe'
assert holding_caps()==[496,501,548,551,555,580,593]
cl=s.classify(593,need_inside=False)
assert cl.status=='admissible' and not cl.uncertain
assert all(gap(593,c).sign()<=0 for c in O if c!=593)
assert gap(501,593).sign()<0
put(593,12)
assert component_hb()==(8,2) and {501,593}<=O
assert holding_caps()==[513,529,547,555,562,580,585,593]
assert all(classification(p,501)=='out' for p in K)
h=heights(); hist=Counter(h[int(i)] for i in s.pose_id)
assert hist=={0:56457,2:116435,4:4143,6:78,8:7}
hb={}
for i,p in enumerate(s._poses):
    if h[i]>=6:
        l=T.factors(s._pose_matrix(i))[0]
        hb[i]=(h[i],T.branch(l))
counts=Counter(hb[int(i)] for i in s.pose_id if int(i) in hb)
assert counts=={(6,4):78,(8,2):7}
V={p:{tuple(matvec(s.pose(p),v)) for v in C.regions.vertices(p)} for p in K}
edges=[]
for i,p in enumerate(K):
    for q in K[i+1:]:
        common=V[p]&V[q]
        if len(common)>=4 and affine_rank(list(common))==3: edges.append((p,q))
seen={K[0]}
while True:
    old=len(seen)
    for p,q in edges:
        if p in seen or q in seen: seen.update((p,q))
    if len(seen)==old: break
assert len(edges)==7 and seen==set(K)
assert s.digest()=='c4277e602ebcb90092e280382e3bebfba4c249bb2a03fc630cab9aa1bd493ff5'
z=gap(501,593)
print('heights',sorted(hist.items()))
print('high',sorted(counts.items()))
print('digest',s.digest())
print('gap',(z.a,z.b,z.d))
print('PASS',727)
assert RS.element(M,593,45)==to_tuple(transpose(RS.element(M,593,12)))
put(593,45)
assert component_hb()==(6,1) and 501 in O
assert all(classification(p,501)=='in' for p in K)
assert s.digest()=='ede1ebc8e264cd95db54cc816e9fcb1c68f591491c2300d23eb99c71e61628fe'
put(501,12)
assert component_hb()==(6,1) and 593 in O
assert holding_caps()==[501,542,548,551,580]
assert [classification(p,593) for p in K]==['straddle','out','out','out','out','out','out']
assert RS.element(M,501,40)==to_tuple(transpose(matmul(RS.element(M,501,12),RS.element(M,501,22))))
put(501,40)
assert component_hb()==(4,2) and 593 in O and 501 not in O
assert all(classification(p,593)=='in' for p in K)
assert holding_caps()==[496,501,551,555,580,593]
assert s.digest()=='9e8efe5ec9a7fa035319baf1fa88aac49ef2cc7c54a0d52a3faa616c667bcf03'
print('arrival',component_hb(),holding_caps(),s.digest())
put(593,12)
assert component_hb()==(4,2) and 593 in O
assert holding_caps()==[513,529,547,562,585,593]
assert s.digest()=='95f63be5675ac2868f4b2e53756981419f5c95908ab7ba6a73e320f16f898285'
print('next',component_hb(),holding_caps(),s.digest())
```

Output:

```text
heights [(0, 56457), (2, 116435), (4, 4143), (6, 78), (8, 7)]
high [((6, 4), 78), ((8, 2), 7)]
digest c4277e602ebcb90092e280382e3bebfba4c249bb2a03fc630cab9aa1bd493ff5
gap (-309168, -61072, 15625)
PASS 727
arrival (4, 2) [496, 501, 551, 555, 580, 593] 9e8efe5ec9a7fa035319baf1fa88aac49ef2cc7c54a0d52a3faa616c667bcf03
next (4, 2) [513, 529, 547, 562, 585, 593] 95f63be5675ac2868f4b2e53756981419f5c95908ab7ba6a73e320f16f898285
```

T15-05: the H6 island after its first return, up to its first raise to height 6.

```python
import sys,gzip,json
from functools import lru_cache
from collections import Counter
sys.path[:0]=['research/jumbling/theory','research/jumbling']
import sim,restore as RS,trees as T
from exact import Q5,matmul,transpose,dot
from sim.kplus import matrix_from_json,to_tuple
from sim.kernel import rows_form,Evaluator,pullback_form
C=sim.get_context();d=C.data;M=RS.menu('I_a')
with gzip.open('research/jumbling/restoration/04-card5-lattice-invariants/data/t-T723x433-12.json.gz','rt') as f:tab=json.load(f)
K=[p for p,i in enumerate(tab['pose_id']) if i==1138]
Q=to_tuple(matmul(transpose(RS.element(M,433,12)),matrix_from_json(tab['poses'][1138])))
V=sorted({tuple(v) for p in K for v in C.regions.vertices(p)},key=repr);vf=rows_form(V)
assert len(K)==78 and len(V)==74
@lru_cache(None)
def lhs(R):
    l=T.factors(R)[0]
    for a in [Q5(1),Q5(2),Q5(-1,1),Q5(1,1)]:
        q=tuple(a*x for x in l)
        if T.qnorm(q)==1:return T.qkey(q)
    raise AssertionError('normalization')
canon_cache={}
def key(R):
    l=lhs(R)
    if l not in canon_cache:
        orbit=[T.qkey(T.qmul(u,l)) for u in T.PUNITS]
        k=min(orbit,key=lambda q:tuple((x.a,x.b,x.d) for x in q))
        for x in orbit:canon_cache[x]=k
    return canon_cache[l]
@lru_cache(None)
def holding(R):
    out=[]
    for c in range(600):
        ev=Evaluator(pullback_form(R,d.N[c]))
        if all(ev.sign_row(vf[0],r)>=0 for r in vf[1]):out.append(c)
    return tuple(out)
@lru_cache(None)
def nb(g):
    return {c for c in range(600) if c!=g and (d.NN*(d.NN+dot(d.N[g],d.N[c]))-2*d.KAPPA*d.KAPPA).sign()>0}
@lru_cache(None)
def element(c,j):return RS.element(M,c,j)
reps=[Q];seen={key(Q)};parents=[None];counts=Counter();fails=[];i=0
while i<len(reps):
    R=reps[i]
    for c in holding(R):
        for j in range(60):
            S=to_tuple(matmul(element(c,j),R));h=T.dist(lhs(S));counts[h]+=1
            if h==4:
                k=key(S)
                if k not in seen:seen.add(k);reps.append(S);parents.append((i,c,j))
            elif h==6:
                bad=sorted(set(holding(S))-{c}-nb(c))
                if bad:fails.append((i,c,j,bad))
            else:assert h==2
    i+=1
assert len(reps)==4 and counts=={2:96,4:564,6:720} and len(fails)==96
print('classes',len(reps),'parents',parents,'edges',dict(sorted(counts.items())),'failures',len(fails))
for ic in sorted({(i,c) for i,c,j,b in fails}):
    print(ic,[(j,b) for i,c,j,b in fails if (i,c)==ic])
import numpy as np
orders=[]
for R in reps:
    gens=sorted({g for c in holding(R) for g in C.kplus.a4(c)})
    known={0};front=np.array([0],np.int64)
    while len(front):
        nxt=set()
        for g in gens:nxt.update(map(int,C.kplus.compose_vec(np.full(len(front),g,np.int64),front)))
        nxt-=known;known|=nxt;front=np.array(sorted(nxt),np.int64)
    orders.append(len(known))
assert orders==[7200]*4
print('orders',orders,'poses',sum(orders))
R=Q
for c,j in [(274,12),(326,12),(281,12)]:
    assert c in holding(R)
    R=to_tuple(matmul(element(c,j),R))
    print('abstract',c,j,'height',T.dist(lhs(R)),'holds',holding(R))
```

Output:

```text
classes 4 parents [None, (0, 326, 12), (0, 326, 13), (0, 326, 14)] edges {2: 96, 4: 564, 6: 720} failures 96
(0, 430) [(12, [281]), (14, [315]), (16, [488]), (20, [373]), (21, [406]), (23, [520]), (24, [390]), (25, [406]), (26, [206]), (27, [557]), (28, [532]), (29, [315]), (30, [231]), (31, [520]), (32, [381]), (43, [231]), (44, [488]), (45, [390]), (46, [381]), (47, [281]), (48, [206]), (49, [557]), (50, [373]), (51, [532])]
(1, 258) [(12, [433]), (13, [309]), (16, [79]), (17, [172]), (19, [398]), (20, [233]), (24, [102]), (25, [124]), (26, [398]), (27, [271]), (28, [309]), (29, [221]), (30, [454]), (31, [344]), (32, [172]), (34, [454]), (35, [433]), (36, [344]), (37, [102]), (38, [124]), (39, [233]), (40, [271]), (41, [79]), (42, [221])]
(1, 398) [(13, [271]), (15, [454]), (17, [172]), (18, [279]), (19, [258]), (22, [467]), (33, [564]), (34, [433]), (35, [454]), (36, [564]), (37, [298]), (38, [487]), (39, [467]), (40, [309]), (41, [279]), (42, [482]), (52, [271]), (53, [298]), (54, [487]), (55, [258]), (56, [309]), (57, [482]), (58, [433]), (59, [172])]
(2, 299) [(13, [425]), (14, [246]), (17, [493]), (19, [326]), (21, [97]), (23, [195]), (34, [196]), (35, [118]), (36, [97]), (37, [456]), (38, [246]), (39, [291]), (40, [320]), (41, [414]), (42, [195]), (43, [326]), (44, [320]), (45, [425]), (46, [414]), (47, [196]), (48, [118]), (49, [493]), (50, [456]), (51, [291])]
orders [7200, 7200, 7200, 7200] poses 28800
abstract 274 12 height 6 holds (274, 326, 340, 373, 430, 433)
abstract 326 12 height 8 holds (281, 313, 326, 342)
abstract 281 12 height 10 holds (281, 313, 342, 398, 425)
```

The integrator's replay of T15-01 and the one-letter check for height 10.

```python
# Independent replay of the T15-01 word (integrator's own code), then a one-letter search for height 10.
import sys, json
from collections import Counter
from functools import lru_cache
import numpy as np
J = sys.argv[1]
sys.path[:0] = [J + '/theory', J, J + '/fixtures']
import sim, trees as T, wj, restore as RS
C = sim.get_context(); d = C.data
F, M = wj.load(C, 'I_a')
s = sim.State(C, filtered=False, menu=M)
@lru_cache(None)
def hb(key):
    if key[0] == 'K':
        return (0, -1)
    f = T.factors(key[1])[0]
    return (T.dist(f), T.branch(f))
def piece_hb(st, p):
    return hb(st._poses[int(st.pose_id[p])].key)
def hist(st):
    ph = np.array([hb(p.key)[0] for p in st._poses])[st.pose_id]
    return dict(sorted(Counter(ph.tolist()).items()))
def letter(st, g, j):
    o = st.apply(sim.Twist(C, g, RS.element(M, g, j), 'menu', {'menu': M.name}), journal=False)
    assert o.applied and not o.uncertain, (g, j)
for rec in F['journal']['records'][:723]:
    o = s.apply(sim.Twist.from_record(C, rec), journal=False)
    assert o.applied and not o.uncertain
for g, j in [(433, 12), (593, 58), (501, 22), (593, 12)]:
    letter(s, g, j)
print('digest', s.digest())
print('hist', hist(s))
K = [118527, 126380, 126382, 126384, 126385, 149644, 149650]
print('K', [piece_hb(s, p) for p in K], 'orbits', sorted({int(d.orbit_id[p]) for p in K}))
top = [int(p) for p in np.flatnonzero(np.array([hb(p.key)[0] for p in s._poses])[s.pose_id] >= 8)]
print('pieces at height >= 8', top)
# one more letter: every grip whose cap wholly holds all of K, of a branch other than K's, admissible -> every jumble
def clone(st):
    u = sim.State(C, filtered=False, menu=M)
    u._poses = list(st._poses); u._lookup = dict(st._lookup); u.pose_id = st.pose_id.copy()
    return u
kb = piece_hb(s, K[0])[1]
cands = []
for c in range(600):
    if T.branch(tuple(d.N[c])) == kb:
        continue
    if not all(sim.kernel.Evaluator(sim.kernel.pullback_form(s.pose(p), d.N[c])).classify(C.regions.vform(p))[0] == 'in' for p in K):
        continue
    cl = s.classify(c, need_inside=False)
    cands.append((c, cl.status, bool(cl.uncertain)))
print('caps of another branch holding K whole', cands)
best = 8
for c, status, unc in cands:
    if status != 'admissible' or unc:
        continue
    for j in RS.jumble_items(M):
        u = clone(s)
        o = u.apply(sim.Twist(C, c, RS.element(M, c, j), 'menu', {'menu': M.name}), journal=False)
        if not (o.applied and not o.uncertain):
            continue
        h = piece_hb(u, K[0])[0]
        best = max(best, h)
        if h >= 10:
            print('height10', c, j, u.digest()); break
print('max height of K after one more letter', best)
```

Output:

```text
digest c4277e602ebcb90092e280382e3bebfba4c249bb2a03fc630cab9aa1bd493ff5
hist {0: 56457, 2: 116435, 4: 4143, 6: 78, 8: 7}
K [(8, 2), (8, 2), (8, 2), (8, 2), (8, 2), (8, 2), (8, 2)] orbits [19, 20, 21, 22, 23, 24]
pieces at height >= 8 [118527, 126380, 126382, 126384, 126385, 149644, 149650]
caps of another branch holding K whole [(529, 'blocked', False), (547, 'blocked', False), (555, 'blocked', False), (562, 'blocked', False), (580, 'blocked', False), (585, 'blocked', False)]
max height of K after one more letter 8
```

## 21. Relays, cohorts and pumping (round 16)

Round 15 refuted P6 (section 20). Round 16 asked whether left heights are bounded at all: whether the relay of T15-01 continues to height 10, for an explicit bound or an unbounded family, and for a sound abstraction of one high component. Before the round the integrator ran exact searches near T15-01 (E16-1 to E16-3 below). The review returned four results (T16-01 to T16-04, all `minor`). The integrator re-ran the three programs from the repository root, got the reported outputs exactly, checked the proof of T16-04, and adopted all four.

| Finding | Status | Result |
| --- | --- | --- |
| T16-01 | computed | Every raise-only relay from X4 stops at height 8. X4 is the state after the fixture records 1 to 723, (433, 12) and (593, 58), where the seven pieces of T15-01 form one component at height 4 in branch 2. Level 1: of the four caps of another branch that hold the component whole, three are NB-blocked and one (501) is admissible; 48 raises to height 6. Level 2: 252 candidate caps, of which 250 are NB-blocked, one is straddled by another piece and one is admissible (593, after (501, 22) only); 48 raises to height 8. Level 3: 228 candidates, 212 NB-blocked and 16 straddled; no raise. In all, 97 states and 58,200 exact cap decisions. |
| T16-02 | proved; computed | A sound abstraction for one high component K. It keeps the exact pose of K and, for each grip, the coset of the product of its letters modulo A4 (five cosets). A grip in a non-retained coset blocks the 22 cuts that its inner cohort straddles: the 16 of NB and six more. Under INV it over-approximates every continuation while K stays high. It admits the path (501, 22), (593, 12), (342, 15), (484, 13), (529, 12) from X4 to height 10: the returns of 342 and 484 do not move K, and after them cap 529 has no cohort blocker. In the puzzle the path fails at once: at the height-8 state cap 342 is straddled by piece 115294 (model orbit 9, height 2), cap 484 by piece 107148 (orbit 32) and cap 529 by piece 165151 (orbit 5). So this abstraction cannot prove a bound of 8. |
| T16-03 | proved | A fixed word transported by the powers of one global symmetry k ∈ K⁺, or any eventually periodic schedule of fixed words, cannot raise heights without bound. k has order dividing 60, so the schedule repeats one word, and T9-02 makes the trajectory periodic. An unbounded family would have to choose its words aperiodically or from the state. |
| T16-04 | proved | The uniform period of T9-02 does not by itself imply finiteness. An explicit infinite ray with three involutive letters has unbounded heights and unbounded return distances, yet every repeatable word returns after two repetitions. A finiteness proof needs the geometry of admissibility. |

### Integrator experiments (before the round)

- E16-1 (`e16c.py`), at the height-8 state of T15-01. Cap 501 is straddled (by piece 87889, model orbit 9, height 2), so 501 cannot return. The six caps of another branch that hold the component whole are in NB of the out grips 484, 501 and 593. Grip 593 can return by any of 12 items, and every return keeps the component at height 8: the component entered cap 593 after 593 departed, so the return does not undo the raise. After each return, four caps of another branch hold the component whole, and each is NB-blocked by an out grip or straddled by a height-2 piece. After three of the returns (items 52, 55 and 56) grip 501 can return as well, with the same outcome. No letter raises the component to 10.
- E16-2 (`e16d.py`, from X4): the same counts as T16-01, found with the integrator's own code. The 16 straddling pieces at level 3 have height 2 and belong to model orbits 7, 9, 22, 28 and 34.
- E16-3 (`e16d.py`, from H6): every cap of another branch that holds the component at H6 (four) is NB-blocked, so no raise-only relay starts there; T15-01 begins with the same-branch letter (593, 58).

### What it gives

- No height 10 has been found. Every continuation of the relay of T15-01 that was examined stops at a cap that is NB-blocked by an out grip or straddled by a height-2 piece outside the inscribed ball.
- A grip that was already out when a component arrived in its cap can return without lowering the component (E16-1), so a raise by such a grip is kept after the grip returns.
- The inner cohorts alone do not bound the height (T16-02). A bound would need the outer pieces, such as the height-2 straddlers above.
- Two routes are closed: a fixed pattern moved by a symmetry cannot pump heights (T16-03), and the periodicity of T9-02 cannot by itself prove finiteness (T16-04).
- X1 and the uniform bound of M1 stay open; Proposition HB is unchanged.
- Next (round 17, section 22): realize or refute the abstract path of T16-02 with preparation letters that leave the component at height 8, and look for an invariant of the outer off-lattice pieces that accounts for the blocking.

### Reproduction (round 16)

From the repository root, with Python 3.13 and NumPy 2.5: `python3 -I -B -` with each program on standard input; the integrator's programs take `research/jumbling` as their first argument (`python3 -I -B - research/jumbling`), and `e16d.py` takes `X4` or `H6` as its second. On the integrator's machine T16-01, T16-02 and T16-03 took 235, 9 and 2 seconds, and E16-1 and each run of E16-2 and E16-3 about 210 seconds.

T16-01: every raise-only relay from X4.

```python
import sys, signal, json, hashlib
from collections import Counter
from functools import lru_cache
sys.path[:0] = ['research/jumbling/theory','research/jumbling',
               'research/jumbling/fixtures']
import sim, wj, restore as RS, trees as T
from exact import matmul, matvec, dot, affine_rank
from sim.kplus import to_tuple
from sim.kernel import Evaluator, pullback_form, rows_form
signal.alarm(890)
C=sim.get_context(); d=C.data; F,M=wj.load(C,'I_a')
I=C.kplus.matrix(0)
K=(118527,126380,126382,126384,126385,149644,149650)
J=tuple(RS.jumble_items(M)); assert len(J)==48
E=[Evaluator(pullback_form(I,d.N[c])) for c in range(600)]
B=[T.branch(tuple(d.N[c])) for c in range(600)]
V={p:{tuple(v) for v in C.regions.vertices(p)} for p in K}
edges=[]
for i,p in enumerate(K):
    for q in K[i+1:]:
        common=V[p]&V[q]
        if len(common)>=4 and affine_rank(list(common))==3:
            edges.append((p,q))
seen={K[0]}
while True:
    old=len(seen)
    for p,q in edges:
        if p in seen or q in seen: seen.update((p,q))
    if len(seen)==old: break
assert len(edges)==7 and seen==set(K)
H=tuple(sorted(set().union(*V.values()),key=repr)); assert len(H)==16
@lru_cache(None)
def hb(Q):
    f=T.factors(Q)[0]
    return T.dist(f),T.branch(f)
@lru_cache(None)
def element(c,j): return RS.element(M,c,j)
@lru_cache(None)
def gap(a,b):
    return d.NN*(d.NN+dot(d.N[a],d.N[b]))-2*d.KAPPA*d.KAPPA
@lru_cache(None)
def holding(Q):
    vf=rows_form(sorted({tuple(matvec(Q,v)) for v in H},key=repr))
    labels=tuple(ev.classify(vf)[0] for ev in E)
    assert set(labels)<={'in','out','straddle'}
    return (tuple(c for c in range(600) if labels[c]=='in'),
            ''.join({'in':'I','out':'O','straddle':'S'}[x] for x in labels))
def clone(st):
    q=sim.State(C,filtered=False,menu=M)
    q._poses=list(st._poses); q._lookup=dict(st._lookup)
    q.pose_id=st.pose_id.copy()
    return q
def tracked(st):
    Q=st.pose(K[0]); assert all(st.pose(p)==Q for p in K)
    return Q,hb(Q)
def apply(st,G,O,tw):
    out=st.apply(tw,journal=False)
    assert out.applied and not out.uncertain,(tw.grip,out.status)
    G[tw.grip]=to_tuple(matmul(tw.matrix,G[tw.grip]))
    if C.kplus.index_of_matrix(G[tw.grip])<0: O.add(tw.grip)
    else: O.discard(tw.grip)
def put(st,G,O,c,j):
    apply(st,G,O,sim.Twist(C,c,element(c,j),'menu',{'menu':M.name}))
s=sim.State(C,filtered=False,menu=M)
G={g:I for g in range(600)}; O=set()
for n,rec in enumerate(F['journal']['records'][:723],1):
    apply(s,G,O,sim.Twist.from_record(C,rec))
    if n>=85: assert 593 in O
assert s.digest()=='bd0cbd3337ef16085cba3498916b3c683f04735b5cc3737012c6414af422f7d7'
put(s,G,O,433,12)
assert s.digest()=='fe917f9a77c69cf037ce5c894debdec2448b73575ed776d5c529ccfaf5bc7c39'
put(s,G,O,593,58)
assert s.digest()=='9e8efe5ec9a7fa035319baf1fa88aac49ef2cc7c54a0d52a3faa616c667bcf03'
assert tracked(s)[1]==(4,2)
print('X4',s.digest(),tracked(s)[1],holding(tracked(s)[0])[0],len(O))
stats=[Counter() for _ in range(3)]
cap_table=[]; leaf_rows=[]; continuing=[]; known=False
front=[(s,G,O,())]
for depth in range(3):
    nxt=[]
    for st,gprod,outs,path in front:
        Q,(h,b)=tracked(st); caps,labels=holding(Q)
        stats[depth]['nodes']+=1
        stats[depth]['cap_decisions']+=600
        cap_table.append([list(path),labels]); can=[]
        for c in caps:
            if B[c]==b: continue
            stats[depth]['candidates']+=1
            blockers=[a for a in sorted(outs) if a!=c and gap(a,c).sign()>0]
            cl=st.classify(c,need_inside=False); assert not cl.uncertain
            if blockers:
                stats[depth]['NB_blocked']+=1
                assert cl.status=='blocked'
                a=blockers[0]; z=gap(a,c)
                leaf_rows.append([list(path),c,'NB',a,[z.a,z.b,z.d]])
            elif cl.status=='blocked':
                stats[depth]['other_straddled']+=1
                leaf_rows.append([list(path),c,'straddle',cl.straddle['piece'],
                                  cl.straddle['h_below'],cl.straddle['h_above']])
            else:
                assert cl.status=='admissible'
                stats[depth]['admissible_grips']+=1; can.append(c)
                for j in J:
                    u=clone(st); gu=dict(gprod); ou=set(outs)
                    put(u,gu,ou,c,j); assert tracked(u)[1][0]==h+2
                    stats[depth]['raises']+=1; pp=path+((c,j),)
                    if pp==((501,22),(593,12)):
                        assert tracked(u)[1]==(8,2)
                        assert u.digest()=='c4277e602ebcb90092e280382e3bebfba4c249bb2a03fc630cab9aa1bd493ff5'
                        heights=[0 if p.kidx>=0 else hb(p.matrix)[0] for p in u._poses]
                        hist=Counter(heights[int(i)] for i in u.pose_id)
                        assert hist=={0:56457,2:116435,4:4143,6:78,8:7}
                        print('T15-01',u.digest(),'hist',sorted(hist.items()))
                        known=True
                    nxt.append((u,gu,ou,pp))
        if can: continuing.append([list(path),can])
    print('depth',depth,json.dumps(dict(sorted(stats[depth].items())),sort_keys=True))
    front=nxt
assert known and not front
print('continuing',continuing)
print('nodes',sum(x['nodes'] for x in stats),
      'caps_exact',sum(x['cap_decisions'] for x in stats))
for name,table in [('cap_table',cap_table),('blocking_table',leaf_rows)]:
    print(name+'_sha256',hashlib.sha256(
        json.dumps(table,separators=(',',':')).encode()).hexdigest())
print('raise10_paths',len(front))
```

Output:

```text
X4 9e8efe5ec9a7fa035319baf1fa88aac49ef2cc7c54a0d52a3faa616c667bcf03 (4, 2) (496, 501, 551, 555, 580, 593) 54
depth 0 {"NB_blocked": 3, "admissible_grips": 1, "candidates": 4, "cap_decisions": 600, "nodes": 1, "raises": 48}
T15-01 c4277e602ebcb90092e280382e3bebfba4c249bb2a03fc630cab9aa1bd493ff5 hist [(0, 56457), (2, 116435), (4, 4143), (6, 78), (8, 7)]
depth 1 {"NB_blocked": 250, "admissible_grips": 1, "candidates": 252, "cap_decisions": 28800, "nodes": 48, "other_straddled": 1, "raises": 48}
depth 2 {"NB_blocked": 212, "candidates": 228, "cap_decisions": 28800, "nodes": 48, "other_straddled": 16}
continuing [[[], [501]], [[(501, 22)], [593]]]
nodes 97 caps_exact 58200
cap_table_sha256 7af6a8afd8e5ef6ae7ef17ef359c417c30951a204d5c67f63997d21c5eecc956
blocking_table_sha256 33a6d26016138733ec5e9e2fcdcd4ca80b959321414504d991a9b16ed19ac33c
raise10_paths 0
```

T16-02: the cohort abstraction, its height-10 path and the blockers of its realization.

```python
import sys,json,gzip,hashlib
import numpy as np
from functools import lru_cache
sys.path[:0]=['research/jumbling/theory','research/jumbling',
              'research/jumbling/fixtures']
import sim,wj,restore as R,trees as T
from exact import matmul,matvec
from sim.kplus import matrix_from_json,matrix_json,to_tuple
from sim.kernel import rows_form,Evaluator,pullback_form
C=sim.get_context(); d=C.data; F,M=wj.load(C,'I_a'); I=C.kplus.matrix(0)
K=(118527,126380,126382,126384,126385,149644,149650)
base=[to_tuple(m) for _,m in M.items]; ix={m:j for j,m in enumerate(base)}
labels={name:j for j,(name,m) in enumerate(M.items)}
ret=[j for j,m in enumerate(base) if C.kplus.index_of_matrix(m)>=0]
reps=[]; cs={}
for j in range(60):
    if j not in cs:
        q=len(reps); reps.append(j)
        for a in ret: cs[ix[to_tuple(matmul(base[j],base[a]))]]=q
assert reps==[0,12,24,25,27]
act=[[cs[ix[to_tuple(matmul(base[j],base[r]))]] for r in reps]
     for j in range(60)]
inner=[int(p) for p in d.cap_members(0)
       if int(d.orbit_id[p]) in {0,1,2,3,6}]
assert len(inner)==68
forms=[C.regions.vform(p) for p in inner]; B0=[set()]
for r in reps[1:]:
    bs=set()
    for c in range(600):
        ev=Evaluator(pullback_form(base[r],d.N[c]))
        if any(ev.classify(v)[0]=='straddle' for v in forms): bs.add(c)
    B0.append(bs)
assert [len(b) for b in B0]==[0,22,22,22,22]
@lru_cache(None)
def block(c,q):
    return {int(C.kplus.perms[int(C.kplus.frame_idx[c]),e]) for e in B0[q]}
sigma=[0]*600
for rec in F['journal']['records'][:723]:
    c=rec['grip']; j=labels[rec['params']['label']]
    assert R.element(M,c,j)==matrix_from_json(rec['matrix'])
    sigma[c]=act[j][sigma[c]]
sigma[433]=act[12][sigma[433]]
p='research/jumbling/restoration/04-card5-lattice-invariants/data/t-T723x433-12.json.gz'
with gzip.open(p,'rt') as f: tab=json.load(f)
s=sim.State(C,filtered=False,menu=M)
ids=[s._intern_matrix(matrix_from_json(m)) for m in tab['poses']]
s.pose_id=np.array([ids[i] for i in tab['pose_id']],np.int32)
assert s.digest()==tab['digest']=='fe917f9a77c69cf037ce5c894debdec2448b73575ed776d5c529ccfaf5bc7c39'
for c,j in [(593,58),(501,22),(593,12)]:
    o=s.apply(sim.Twist(C,c,R.element(M,c,j),'menu',{'menu':M.name}),journal=False)
    assert o.applied and not o.uncertain
    sigma[c]=act[j][sigma[c]]
assert s.digest()=='c4277e602ebcb90092e280382e3bebfba4c249bb2a03fc630cab9aa1bd493ff5'
Q=s.pose(K[0]); assert all(s.pose(p)==Q for p in K)
V={tuple(v) for p in K for v in C.regions.vertices(p)}; assert len(V)==16
evs=[Evaluator(pullback_form(I,n)) for n in d.N]
def cats():
    vf=rows_form([tuple(matvec(Q,v)) for v in V])
    return [e.classify(vf)[0] for e in evs]
def blockers(c):
    return [e for e,q in enumerate(sigma) if q and c in block(e,q)]
def h(q): return T.dist(T.factors(q)[0])
print('cohort_blocker_counts',[len(b) for b in B0])
print('X8_out',sum(q!=0 for q in sigma))
print('X8_nonoutside_out',[(c,sigma[c],t) for c,t in enumerate(cats())
                           if sigma[c] and t!='out'])
print('X8_529_cohort_blockers',blockers(529))
for c in [342,484,529]:
    cl=s.classify(c,need_inside=False)
    assert cl.status=='blocked' and not cl.uncertain
    p=cl.straddle['piece']
    assert cl.straddle['points_checked_against_all_constraints']
    print('actual_blocker',c,p,int(d.orbit_id[p]),h(s.pose(p)))
for c,j in [(342,15),(484,13),(529,12)]:
    side=cats()[c]; assert side in {'in','out'} and not blockers(c)
    before=sigma[c]
    if side=='in': Q=to_tuple(matmul(R.element(M,c,j),Q))
    sigma[c]=act[j][sigma[c]]
    print('abstract',c,j,side,before,sigma[c],h(Q))
assert h(Q)==10
print('h10_holding',[c for c,t in enumerate(cats()) if t=='in'])
print('h10_pose_sha256',hashlib.sha256(
    json.dumps(matrix_json(Q),separators=(',',':')).encode()).hexdigest())
```

Output:

```text
cohort_blocker_counts [0, 22, 22, 22, 22]
X8_out 55
X8_nonoutside_out [(593, 2, 'in')]
X8_529_cohort_blockers [342, 484]
actual_blocker 342 115294 9 2
actual_blocker 484 107148 32 2
actual_blocker 529 165151 5 2
abstract 342 15 out 2 0 8
abstract 484 13 out 4 0 8
abstract 529 12 in 0 1 10
h10_holding [496, 529, 554, 555, 557, 562, 564]
h10_pose_sha256 7754cdd09aa6aa7e9e1749012541b89a8783bf04f1b34e208abb717c3f033492
```

T16-03: the orders of the units of 2I.

```python
import sys, math
from collections import Counter
sys.path[:0]=['research/jumbling/theory','research/jumbling']
import trees as T
D=Counter()
for u in T.UNITS:
    p=T.ONEQ
    for n in range(1,61):
        p=T.qmul(u,p)
        if p==T.ONEQ:
            D[n]+=1
            break
    else:
        raise AssertionError('order >60')
assert D=={1:1,2:1,3:20,4:30,5:24,6:20,10:24}
print(sorted(D.items()))
print('exponent',math.lcm(*D))
```

Output:

```text
[(1, 1), (2, 1), (3, 20), (4, 30), (5, 24), (6, 20), (10, 24)]
exponent 60
```

E16-1 (`e16c.py`): the returns of 593 and 501 at the height-8 state.

```python
# From the height-8 state of T15-01: return 593 by each of its 12 return items (K stays at 8), then
# (a) every foreign cap holding K whole, not NB-blocked: classify, raise -> 10?
# (b) can 501 return now (its cap was straddled at the height-8 state)? what straddles it? then (a) again.
import sys, time
from collections import Counter
J = sys.argv[1]
sys.path[:0] = [J + '/theory', J, J + '/fixtures']
import sim, wj, restore as RS, trees as T
from exact import matmul, matvec, dot
from sim.kplus import to_tuple
from sim.kernel import Evaluator, pullback_form, rows_form
t0 = time.time()
C = sim.get_context(); d = C.data; kp = C.kplus
F, M = wj.load(C, 'I_a')
JUMB = RS.jumble_items(M)
K = [118527, 126380, 126382, 126384, 126385, 149644, 149650]
I = kp.matrix(0)
BR = [T.branch(tuple(d.N[g])) for g in range(600)]
def gap(a, b): return d.NN * (d.NN + dot(d.N[a], d.N[b])) - 2 * d.KAPPA * d.KAPPA
NB = [set() for _ in range(600)]
for a in range(600):
    for b in range(a + 1, 600):
        if gap(a, b).sign() > 0: NB[a].add(b); NB[b].add(a)
def tw(g, j): return sim.Twist(C, g, RS.element(M, g, j), 'menu', {'menu': M.name})
def clone(st):
    u = sim.State(C, filtered=False, menu=M)
    u._poses = list(st._poses); u._lookup = dict(st._lookup); u.pose_id = st.pose_id.copy()
    return u
def apply(st, t, G):
    o = st.apply(t, journal=False)
    if not (o.applied and not o.uncertain): return False
    G[t.grip] = to_tuple(matmul(t.matrix, G[t.grip])); return True
def out(G): return {g for g in range(600) if kp.index_of_matrix(G[g]) < 0}
def khb(st):
    Q = st.pose(K[0]); assert all(st.pose(p) == Q for p in K)
    l = T.factors(Q)[0]; return T.dist(l), T.branch(l)
def holding(st):
    V = {tuple(matvec(st.pose(p), v)) for p in K for v in C.regions.vertices(p)}
    vf = rows_form(V)
    return [g for g in range(600) if Evaluator(pullback_form(I, d.N[g])).classify(vf)[0] == 'in']
def hof(st, p):
    P = st._poses[int(st.pose_id[p])]
    return 0 if P.kidx >= 0 else T.dist(T.factors(P.matrix)[0])
def try_raise(st, G, tag):
    h, b = khb(st); O = out(G); res = []
    for c in holding(st):
        if BR[c] == b: continue
        nbo = sorted(x for x in O if x != c and c in NB[x])
        if nbo:
            res.append((c, 'NB', nbo)); continue
        cl = st.classify(c, need_inside=False)
        if cl.status != 'admissible':
            stp = cl.straddle['piece']
            res.append((c, 'straddled', (stp, int(d.orbit_id[stp]), hof(st, stp)))); continue
        u = clone(st); G2 = dict(G)
        assert apply(u, tw(c, JUMB[0]), G2)
        h2 = khb(u)[0]
        res.append((c, 'RAISE', h2, u.digest()))
    print(tag, 'K', (h, b), res, flush=True)
    return res
s = sim.State(C, filtered=False, menu=M)
G = {g: I for g in range(600)}
for rec in F['journal']['records'][:723]:
    assert apply(s, sim.Twist.from_record(C, rec), G)
for g, j in [(433, 12), (593, 58), (501, 22), (593, 12)]:
    assert apply(s, tw(g, j), G)
assert s.digest() == 'c4277e602ebcb90092e280382e3bebfba4c249bb2a03fc630cab9aa1bd493ff5'
print('height-8 state', round(time.time() - t0), 's', flush=True)
cl = s.classify(501, need_inside=False)
stp = cl.straddle['piece'] if cl.straddle else None
print('cap 501 at the height-8 state:', cl.status, 'straddler', stp, None if stp is None else (int(d.orbit_id[stp]), hof(s, stp)), flush=True)
rets = [j for j in range(60) if kp.index_of_matrix(to_tuple(matmul(RS.element(M, 593, j), G[593]))) >= 0]
print('return items at 593', rets, flush=True)
for j in rets:
    u = clone(s); Gu = dict(G)
    assert apply(u, tw(593, j), Gu) and 593 not in out(Gu)
    r = try_raise(u, Gu, f'ret593:{j}')
    cl = u.classify(501, need_inside=False)
    if cl.status == 'admissible':
        rets5 = [i for i in range(60) if kp.index_of_matrix(to_tuple(matmul(RS.element(M, 501, i), Gu[501]))) >= 0]
        for i in rets5:
            w = clone(u); Gw = dict(Gu)
            assert apply(w, tw(501, i), Gw)
            try_raise(w, Gw, f'ret593:{j} ret501:{i}')
    else:
        p = cl.straddle['piece']
        print(f'ret593:{j} cap 501 blocked by piece', p, (int(d.orbit_id[p]), hof(u, p)), flush=True)
print('done', round(time.time() - t0), 's', flush=True)
```

Output:

```text
height-8 state 189 s
cap 501 at the height-8 state: blocked straddler 87889 (9, 2)
return items at 593 [15, 18, 22, 33, 52, 53, 54, 55, 56, 57, 58, 59]
ret593:15 K (8, 2) [(551, 'NB', [501, 537]), (555, 'NB', [501]), (580, 'NB', [501]), (582, 'NB', [521])]
ret593:15 cap 501 blocked by piece 126380 (19, 8)
ret593:18 K (8, 2) [(551, 'NB', [501, 537]), (555, 'NB', [501]), (580, 'NB', [501]), (582, 'NB', [521])]
ret593:18 cap 501 blocked by piece 126380 (19, 8)
ret593:22 K (8, 2) [(551, 'NB', [501, 537]), (555, 'NB', [501]), (580, 'NB', [501]), (582, 'NB', [521])]
ret593:22 cap 501 blocked by piece 112819 (4, 2)
ret593:33 K (8, 2) [(548, 'NB', [442, 501, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:33 cap 501 blocked by piece 153642 (23, 2)
ret593:52 K (8, 2) [(548, 'NB', [442, 501, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:52 ret501:14 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:52 ret501:21 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:52 ret501:23 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:52 ret501:43 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:52 ret501:44 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:52 ret501:45 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:52 ret501:46 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:52 ret501:47 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:52 ret501:48 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:52 ret501:49 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:52 ret501:50 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:52 ret501:51 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:53 K (8, 2) [(594, 'NB', [573]), (596, 'NB', [576]), (598, 'NB', [572, 588]), (599, 'straddled', (89318, 34, 2))]
ret593:53 cap 501 blocked by piece 138857 (9, 2)
ret593:54 K (8, 2) [(547, 'NB', [484]), (562, 'straddled', (161992, 28, 2)), (585, 'straddled', (162582, 22, 2)), (589, 'NB', [572])]
ret593:54 cap 501 blocked by piece 122213 (14, 2)
ret593:55 K (8, 2) [(594, 'NB', [573]), (596, 'NB', [576]), (598, 'NB', [572, 588]), (599, 'straddled', (89318, 34, 2))]
ret593:55 ret501:14 K (8, 2) [(594, 'NB', [573]), (596, 'NB', [576]), (598, 'NB', [572, 588]), (599, 'straddled', (89318, 34, 2))]
ret593:55 ret501:21 K (8, 2) [(594, 'NB', [573]), (596, 'NB', [576]), (598, 'NB', [572, 588]), (599, 'straddled', (89318, 34, 2))]
ret593:55 ret501:23 K (8, 2) [(594, 'NB', [573]), (596, 'NB', [576]), (598, 'NB', [572, 588]), (599, 'straddled', (89318, 34, 2))]
ret593:55 ret501:43 K (8, 2) [(594, 'NB', [573]), (596, 'NB', [576]), (598, 'NB', [572, 588]), (599, 'straddled', (89318, 34, 2))]
ret593:55 ret501:44 K (8, 2) [(594, 'NB', [573]), (596, 'NB', [576]), (598, 'NB', [572, 588]), (599, 'straddled', (89318, 34, 2))]
ret593:55 ret501:45 K (8, 2) [(594, 'NB', [573]), (596, 'NB', [576]), (598, 'NB', [572, 588]), (599, 'straddled', (89318, 34, 2))]
ret593:55 ret501:46 K (8, 2) [(594, 'NB', [573]), (596, 'NB', [576]), (598, 'NB', [572, 588]), (599, 'straddled', (89318, 34, 2))]
ret593:55 ret501:47 K (8, 2) [(594, 'NB', [573]), (596, 'NB', [576]), (598, 'NB', [572, 588]), (599, 'straddled', (89318, 34, 2))]
ret593:55 ret501:48 K (8, 2) [(594, 'NB', [573]), (596, 'NB', [576]), (598, 'NB', [572, 588]), (599, 'straddled', (89318, 34, 2))]
ret593:55 ret501:49 K (8, 2) [(594, 'NB', [573]), (596, 'NB', [576]), (598, 'NB', [572, 588]), (599, 'straddled', (89318, 34, 2))]
ret593:55 ret501:50 K (8, 2) [(594, 'NB', [573]), (596, 'NB', [576]), (598, 'NB', [572, 588]), (599, 'straddled', (89318, 34, 2))]
ret593:55 ret501:51 K (8, 2) [(594, 'NB', [573]), (596, 'NB', [576]), (598, 'NB', [572, 588]), (599, 'straddled', (89318, 34, 2))]
ret593:56 K (8, 2) [(548, 'NB', [442, 501, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:56 ret501:14 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:56 ret501:21 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:56 ret501:23 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:56 ret501:43 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:56 ret501:44 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:56 ret501:45 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:56 ret501:46 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:56 ret501:47 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:56 ret501:48 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:56 ret501:49 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:56 ret501:50 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:56 ret501:51 K (8, 2) [(548, 'NB', [442, 537]), (550, 'NB', [484]), (579, 'straddled', (80494, 22, 2)), (581, 'NB', [497])]
ret593:57 K (8, 2) [(594, 'NB', [573]), (596, 'NB', [576]), (598, 'NB', [572, 588]), (599, 'straddled', (89318, 34, 2))]
ret593:57 cap 501 blocked by piece 145398 (30, 2)
ret593:58 K (8, 2) [(547, 'NB', [484]), (562, 'straddled', (161992, 28, 2)), (585, 'straddled', (162582, 22, 2)), (589, 'NB', [572])]
ret593:58 cap 501 blocked by piece 146381 (9, 2)
ret593:59 K (8, 2) [(547, 'NB', [484]), (562, 'straddled', (161992, 28, 2)), (585, 'straddled', (162582, 22, 2)), (589, 'NB', [572])]
ret593:59 cap 501 blocked by piece 147851 (9, 2)
done 210 s
```

E16-2 and E16-3 (`e16d.py`): raise-only relays from X4 and from H6.

```python
# Raise-only relay search with details, from a given start. Level 1: every foreign cap holding K whole, outside
# NB of every out grip, exactly admissible; every jumble there. Levels 2 and 3 likewise. Reports every candidate
# cap outside NB(Out) that is straddled, with its straddler. Start A: X4 (records 1..723, (433,12), (593,58)).
# Start B: H6 (records 1..723, (433,12)), K at its H6 position.
import sys, time, math
from collections import Counter
import numpy as np
J = sys.argv[1]; START = sys.argv[2]
sys.path[:0] = [J + '/theory', J, J + '/fixtures']
import sim, wj, restore as RS, trees as T
from exact import matmul, matvec, dot
from sim.kplus import to_tuple
from sim.kernel import Evaluator, pullback_form, rows_form
t0 = time.time()
C = sim.get_context(); d = C.data; kp = C.kplus
F, M = wj.load(C, 'I_a')
JUMB = RS.jumble_items(M)
K = [118527, 126380, 126382, 126384, 126385, 149644, 149650]
s5 = math.sqrt(5)
def fl(q): return (q.a + q.b * s5) / q.d
NF = np.array([[fl(x) for x in d.N[g]] for g in range(600)]); KAP = fl(d.KAPPA)
BR = [T.branch(tuple(d.N[g])) for g in range(600)]
NB = [set() for _ in range(600)]
for a in range(600):
    for b in range(a + 1, 600):
        if (d.NN * (d.NN + dot(d.N[a], d.N[b])) - 2 * d.KAPPA * d.KAPPA).sign() > 0:
            NB[a].add(b); NB[b].add(a)
HOMEV = sorted({tuple(v) for p in K for v in C.regions.vertices(p)}, key=repr)
I = kp.matrix(0)
def holding_pose(Q):
    V = [tuple(matvec(Q, v)) for v in HOMEV]
    VF = np.array([[fl(x) for x in v] for v in V])
    m = (VF @ NF.T).min(axis=0) - KAP
    vf = rows_form(set(V))
    return tuple(g for g in range(600) if m[g] > -1e-7 and Evaluator(pullback_form(I, d.N[g])).classify(vf)[0] == 'in')
def lb(Q):
    l = T.factors(Q)[0]; return T.dist(l), T.branch(l)
def tw(g, j): return sim.Twist(C, g, RS.element(M, g, j), 'menu', {'menu': M.name})
def clone(st):
    u = sim.State(C, filtered=False, menu=M)
    u._poses = list(st._poses); u._lookup = dict(st._lookup); u.pose_id = st.pose_id.copy()
    return u
def hof(st, p):
    P = st._poses[int(st.pose_id[p])]
    return 0 if P.kidx >= 0 else T.dist(T.factors(P.matrix)[0])
def outset(G): return {g for g in range(600) if kp.index_of_matrix(G[g]) < 0}
s = sim.State(C, filtered=False, menu=M)
G = {g: I for g in range(600)}
def step(st, G, g, j):
    t = tw(g, j); o = st.apply(t, journal=False)
    if not (o.applied and not o.uncertain): return False
    G[g] = to_tuple(matmul(t.matrix, G[g])); return True
for rec in F['journal']['records'][:723]:
    t = sim.Twist.from_record(C, rec); o = s.apply(t, journal=False); assert o.applied and not o.uncertain
    G[t.grip] = to_tuple(matmul(t.matrix, G[t.grip]))
assert step(s, G, 433, 12)
if START == 'X4':
    assert step(s, G, 593, 58)
print('start', START, s.digest(), 'K', lb(s.pose(K[0])), round(time.time() - t0), 's', flush=True)
stats = Counter(); found = []; straddled = Counter()
def level(st, G, depth, path):
    Q = st.pose(K[0]); h, b = lb(Q); O = outset(G)
    for c in holding_pose(Q):
        if BR[c] == b: continue
        if any(c in NB[x] for x in O if x != c):
            stats[f'L{depth}_NBblocked'] += 1; continue
        cl = st.classify(c, need_inside=False)
        if cl.status != 'admissible':
            p = cl.straddle['piece']
            straddled[(depth, c, p, int(d.orbit_id[p]), hof(st, p))] += 1
            stats[f'L{depth}_straddled'] += 1; continue
        stats[f'L{depth}_admissible'] += 1
        for j in JUMB:
            u = clone(st); Gu = dict(G)
            assert step(u, Gu, c, j)
            h2 = lb(u.pose(K[0]))[0]; assert h2 == h + 2
            stats[f'reach{h2}'] += 1
            if h2 >= 10:
                found.append((path + [(c, j)], u.digest())); print('FOUND', found[-1], flush=True)
            if depth < 3:
                level(u, Gu, depth + 1, path + [(c, j)])
level(s, G, 1, [])
print('stats', dict(stats), flush=True)
for k, v in sorted(straddled.items()):
    print('straddled', k, v, flush=True)
print('found', len(found), 'seconds', round(time.time() - t0), flush=True)
```

Output from X4:

```text
start X4 9e8efe5ec9a7fa035319baf1fa88aac49ef2cc7c54a0d52a3faa616c667bcf03 K (4, 2) 186 s
stats {'L1_admissible': 1, 'reach6': 48, 'L2_NBblocked': 250, 'L2_admissible': 1, 'reach8': 48, 'L3_NBblocked': 212, 'L3_straddled': 16, 'L2_straddled': 1, 'L1_NBblocked': 3}
straddled (2, 593, 100537, 14, 2) 1
straddled (3, 501, 89318, 34, 2) 1
straddled (3, 501, 112820, 7, 2) 1
straddled (3, 562, 161992, 28, 2) 3
straddled (3, 577, 48812, 34, 2) 1
straddled (3, 577, 123569, 9, 2) 1
straddled (3, 579, 80494, 22, 2) 3
straddled (3, 585, 162582, 22, 2) 3
straddled (3, 599, 89318, 34, 2) 3
found 0 seconds 219
```

Output from H6:

```text
start H6 fe917f9a77c69cf037ce5c894debdec2448b73575ed776d5c529ccfaf5bc7c39 K (4, 2) 185 s
stats {'L1_NBblocked': 4}
found 0 seconds 185
```

## 22. Heights 10, 12 and 14 (round 17)

Round 16 found no height 10 near the relay of T15-01 (section 21). Round 17 asked three things: whether height 10 is reachable by preparation letters and a relay (Q1); for an invariant of the outer off-lattice pieces that straddle the caps (Q2); and what each result gives (Q3). The review returned four results (T17-01 to T17-04, all `minor`). The integrator re-ran the four programs from the repository root, got the reported outputs exactly, checked the proofs of T17-01 and T17-04, and adopted all four. While the review ran, the integrator's own experiments (E17-1 to E17-4 below) reached height 10 with a word of 147 letters, which answers Q1. At the height-10 state, returns of out grips alone do not lead to a raise (E17-5), but one more letter at the grip that raised K does: height 12 follows with a word of 149 letters (E17-6), and the same search reaches height 14 with 152 letters, where the hypothesis TH of the trap lemma holds and K is trapped until the raiser returns (E17-7).

| Finding | Status | Result |
| --- | --- | --- |
| T17-01 | computed; proved | At X8, no raise of the seven pieces K follows three or fewer preparation letters that leave every pose of K unchanged. The caps of another branch that hold K whole are 529, 547, 555, 562, 580 and 585. Piece 171842 (model orbit 0) straddles the last five, and every cap that holds it also holds K, so it cannot move while K stays put. Clearing 529 needs a first move of each of 89213 (orbit 6), 112424 (orbit 3) and 165151 (orbit 5). The caps that hold them and exclude K are {342}, {484} and {425, 488, 518, 564}: pairwise disjoint, and all blocked at X8. So at least four such letters must come first. |
| T17-02 | computed | The grip, the coset of its letters and the last letter that moved a piece do not determine which cuts an outer piece straddles. Piece 13 (model orbit 5, outside B, height 2) straddles cuts 6, 13, 27 and 33 after (0, 12), and cuts 4, 5 and 8 after (0, 1), (0, 12). Both words have the same last letter and the same coset at grip 0. |
| T17-03 | computed | INV does not extend directly to the outer pieces. At X8 the height-2 pieces 115294, 165151 and 87889 straddle cuts 342, 529 and 501. None of them has a grip whose cap holds it and whose current coset gives its pose. |
| T17-04 | proved; computed | A finite abstraction by domain and chamber. A pose J_z k with k ∈ K⁺ is coded by z and by the chamber k(A). There are 21 domains (one at height 0 and twenty at height 2; under H these are all of them), and an unknown symbol stands for every other pose. A known code gives the region exactly. Transitions are exact for known codes and over-approximate for unknown pieces, so the abstraction is sound, also together with the exact pose of K and the cohort state of T16-02. At X8 the three straddlers of the path of T16-02 have known codes whose regions straddle seven cuts. So every letter at 342, 484 and 529 is rejected until one of these pieces moves. No closure under the whole menu was computed. |

### Integrator experiments (during the round)

- E17-1, the history of K. Of the 727 letters of the word of T15-01, only 26 move K, all at grips 521, 501 and 593. Grips 521 and 501 each take K to height 2 and back to the lattice several times. Grip 593 takes it to 4, then 501 takes it to 6 and 593 to 8: the height grows only through alternating jumbles at 501 and 593. Merging consecutive letters at one grip gives (521, 8), (501, 39), (593, 46), (501, 22), (593, 12). Replayed from solved, this word stops at (593, 46). After (501, 39), cut 593 is straddled by 160 pieces of height 2 from cap 501, and no single letter at an admissible cap that excludes K clears it.
- E17-2, no second raise straight after a first one. From solved, K lies whole in nine caps. Each of the 432 jumbles there (9 caps × 48) takes K to height 2. Over the 432 resulting states, 2,160 caps of another branch hold K whole: 1,728 are in NB of the raiser, 432 are straddled by a piece of height 2, and none is admissible. So every raise after the first needs other letters in between.
- E17-3, the causal past of K. Keep the letters of the word that move K and replay them from solved. While a kept letter is blocked, add every earlier letter of the word that moved one of the exact straddlers of its cut, and replay again. After four replays, 145 of the 727 letters remain. They are admissible and take K to the same pose at height 8, in a state X8c with 33,434 off-lattice pieces and 13 out grips (X8: 120,663 and 55). The six caps of another branch that hold K whole are still blocked: each is in NB of two or three out grips, 593 among them for five of the six.
- E17-4, a return and a raise. At X8c, grip 593 returns by item 15. K moves with it and stays at height 8 in branch 2, as in E16-1. Then four caps of another branch hold K whole. Three (551, 555 and 580) are in NB of 501, and 582, of branch 1, is in NB of no out grip and is admissible. The jumble (582, 12) takes K to height 10 in branch 1. The whole word has 147 letters, and the simulator checks every letter from the solved state. The final heights are 143,805 pieces at 0, 32,605 at 2, 703 at 4 and the 7 pieces of K at 10.
- E17-5, the height-10 state X10. K lies whole in seven caps: 582, which raised it, and six caps of another branch (542, 548, 551, 560, 580 and 593). Five of the six are in NB of 582. The sixth, 548, is not, so TH fails at 582, but 548 is in NB of the out grips 442, 501 and 537, and 537 cannot return: its cap is straddled. Each of the six caps is also straddled by 95 to 444 pieces of height 2 or 4, and no cap holds all the straddlers of any of them, except 582 for those of 593. No other out grip holds K whole, and a cap that holds part of K is straddled by a member (T13b-05), so returns of other grips leave K where it is. While K stays put, the five caps stay blocked as long as 582 is out, and 548 needs the returns of 442, 501 and 537 first. Each of the 12 returns of 582 takes K back to height 8.
- E17-6, a turn at the raiser. At X10 grip 582 is out, its cap holds K, and it is admissible. The letter (582, 18) keeps 582 out and K at height 10 in branch 1, but turns K into a pose that the caps 560, 565, 576, 590 and 596 of other branches hold whole. Four of them are in NB of 582 (two also in NB of 537); 576, of branch 4, is in NB of no out grip and is admissible, and (576, 12) takes K to height 12 in branch 4. The letter (582, 59) frees cap 597 in the same way. The word has 149 letters, all applied by the exact simulator from the solved state; the final heights are 141,655 pieces at 0, 34,755 at 2, 703 at 4 and the 7 pieces of K at 12.
- E17-7, the relay repeated, and TH at 526. From the height-12 state, a search tries at most two letters before a raise: first letters at admissible caps that hold K whole, then returns of out grips, with K kept at its height. At the first state where a cap of another branch holds K whole, is in NB of no out grip and is admissible, it raises K there. It finds (576, 1), a turn at the raiser, and (573, 1), a letter at a cap of K's branch, after which (526, 12) takes K to height 14 in branch 0 (152 letters). No earlier letter is at 526, so this raise is a fresh departure, and TH holds there: in each of the 48 orientations of K at height 14 that letters at 526 give, every other cap that holds K whole is in NB of 526. By the trap lemma (T14-03), while 526 is out only letters at 526 move K and K stays at height 14, and each of the 12 returns of 526 takes K back to height 12.

### What it gives

- Left heights reach at least 14 (E17-4, E17-6, E17-7). Every word is in a program below, and the exact simulator applies every letter from the solved state.
- How the height grows here. A raise blocks the next raise at once: the raiser's NB and the pieces it moves straddle every candidate cap (E17-2). Three things clear the way.
  - A grip that departed before K entered its cap can raise K and then return without lowering it (E16-1, E17-4). Its return frees its NB.
  - Cutting the word down to the causal past of K drops 582 of its 727 letters, and with them nearly three quarters of the off-lattice pieces and 42 of the 55 out grips (E17-3).
  - A turn at the raiser. The raiser's letters that keep it out move K through 48 orientations at the same height. In some of them K may lie whole in a cap of another branch outside the raiser's NB, which may be free. This is where TH fails: at 582, cap 548 in the orientation of the raise and caps 576 and 597 in two others hold K whole and lie outside NB of 582 (E17-5, E17-6).
- TH holds at the raise to 14. At 526 every cap that holds K in any of the 48 orientations is in NB of 526 (E17-7), so by the trap lemma the height cannot grow until 526 returns, which takes K back to 12. TH depends only on the raiser and on the pose of K before the raise. Whether a raiser at which TH fails can always be found is open.
- E17-5 shows why the turn helps at X10: while K stays put and 582 is out, five of the six candidate caps stay blocked, and the sixth needs three returns, one of them at a grip whose cap is straddled.
- T17-01 shows why X8 itself is stuck while K stays put: an inner piece (model orbit 0) that cannot move without K straddles five of the six raisers, all of them in NB of 593. At X8c the return of 593 moves K, and one of the caps that then hold K (582) is admissible.
- The outer pieces are not attached to a grip by its coset or by the last letter (T17-02, T17-03). T17-04 gives a sound finite abstraction by domain and chamber, which could support a closure computation.
- X1 and the uniform bound of M1 stay open: the relay reached 14 and then met a raise where TH holds, which is evidence either way, not a proof. Proposition HB is unchanged: K belongs to model orbits 19 to 24.
- Next (round 18): characterize the raises where TH holds, and either build a relay that avoids them at every height, which would make R(I_a) infinite and rule out a uniform bound for M1, or prove a bound on left heights.

### Reproduction (round 17)

From the repository root, with Python 3.13 and NumPy 2.5: `python3 -I -B -` with each program on standard input. T17-01, T17-03 and T17-04 run the program of T16-02 (section 21) as a prefix. The review's versions checked the digest of this whole file, which this section changes; the versions below check the digest of that program instead and print the same output. On the integrator's machine T17-01 to T17-04 took 9, 5, 9 and 8 seconds, and E17-1, E17-2, E17-3 with E17-4, the letter-by-letter check of E17-4, E17-5, E17-6 and E17-7 took 12, 16, 252, 16, 34, 21 and 91 seconds.

T17-01: no raise after three or fewer preparation letters at X8.

```python
from pathlib import Path
import hashlib
path=Path('research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md')
code=path.read_text().split('T16-02: the cohort abstraction, its height-10 path and the blockers of its realization.')[1].split('```python\n')[1].split('```')[0]
assert hashlib.sha256(code.encode()).hexdigest()=='a35ed1ad07da956ccebc4a73baa59cf48f86e50d04f3dc92fb8399bc5737038c'
exec(code.split("print('cohort_blocker_counts'")[0])
assert (h(Q),T.branch(T.factors(Q)[0]))==(8,2)
kc=cats(); out={c for c,t in enumerate(kc) if t=='out'}
raisers=[c for c,t in enumerate(kc) if t=='in' and T.branch(tuple(d.N[c]))!=2]
assert raisers==[529,547,555,562,580,585]
print('K_out_caps',len(out),'raisers',raisers)
def pcats(p):
 return [Evaluator(pullback_form(s.pose(p),n)).classify(C.regions.vform(p))[0] for n in d.N]
expected={171842:[],89213:[342],112424:[484],165151:[425,488,518,564]}
for p,want in expected.items():
 pc=pcats(p); held=sorted(c for c in out if pc[c]=='in'); assert held==want
 blocked=[c for c in raisers if pc[c]=='straddle']
 assert blocked==([547,555,562,580,585] if p==171842 else [529,547] if p==112424 else [529])
 for c in blocked:
  r=Evaluator(pullback_form(s.pose(p),d.N[c])).classify(C.regions.vform(p))
  assert s._straddle_certificate(p,int(s.pose_id[p]),c,r[2],r[3])['points_checked_against_all_constraints']
 print('witness',p,'orbit',int(d.orbit_id[p]),'K_fixed_holders',held,'raiser_straddles',blocked)
for c in sorted(set().union(*(set(v) for v in expected.values()))):
 cl=s.classify(c,need_inside=False)
 assert cl.status=='blocked' and not cl.uncertain and cl.straddle['points_checked_against_all_constraints']
 print('initial_blocked',c,'by',cl.straddle['piece'])
H=[set(expected[p]) for p in [89213,112424,165151]]
assert all(H[i].isdisjoint(H[j]) for i in range(3) for j in range(i))
print('no raise of K after at most 3 pointwise-K-fixed preparation letters')
```

Output:

```text
K_out_caps 588 raisers [529, 547, 555, 562, 580, 585]
witness 171842 orbit 0 K_fixed_holders [] raiser_straddles [547, 555, 562, 580, 585]
witness 89213 orbit 6 K_fixed_holders [342] raiser_straddles [529]
witness 112424 orbit 3 K_fixed_holders [484] raiser_straddles [529, 547]
witness 165151 orbit 5 K_fixed_holders [425, 488, 518, 564] raiser_straddles [529]
initial_blocked 342 by 115294
initial_blocked 425 by 116921
initial_blocked 484 by 107148
initial_blocked 488 by 100041
initial_blocked 518 by 123569
initial_blocked 564 by 136721
no raise of K after at most 3 pointwise-K-fixed preparation letters
```

T17-02: piece 13 after two words with the same last letter and the same coset at grip 0.

```python
import sys
sys.path[:0]=['research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import sim,wj,restore as R,trees as T
from exact import Q5,matmul,transpose
from sim.kplus import to_tuple
from sim.kernel import Evaluator,pullback_form
C=sim.get_context(); _,M=wj.load(C,'I_a'); d=C.data; p=13
E1=R.element(M,0,1); E12=R.element(M,0,12)
assert C.kplus.index_of_matrix(E1)>=0
S=d.signature(p); n=[sum((d.N[c][i] for c in S),Q5(0)) for i in range(4)]
z=d.KAPPA*len(S); delta=z*z/sum((v*v for v in n),Q5(0))-d.NN
assert delta.sign()>0
print('outside_B_lower_bound',delta)
ss=[]
for word in ([(0,12)],[(0,1),(0,12)]):
 s=sim.State(C,filtered=False,menu=M)
 for c,j in word:
  o=s.apply(sim.Twist(C,c,R.element(M,c,j),'menu',{'menu':M.name}),journal=False)
  assert o.applied and not o.uncertain
 P=s.pose(p); assert T.dist(T.factors(P)[0])==2
 cuts=[]
 for c in range(600):
  r=Evaluator(pullback_form(P,d.N[c])).classify(C.regions.vform(p))
  if r[0]=='straddle':
   cert=s._straddle_certificate(p,int(s.pose_id[p]),c,r[2],r[3])
   assert cert['points_checked_against_all_constraints']; cuts.append(c)
 ss.append(s)
 print('word',word,'straddles',cuts)
assert ss[0].pose(p)==E12
assert ss[1].pose(p)==to_tuple(matmul(E12,E1))
assert C.kplus.index_of_matrix(matmul(transpose(E12),ss[1].pose(p)))>=0
print('same_piece_last_letter_and_cohort_coset',True)
```

Output:

```text
outside_B_lower_bound Q5(1904212,-848884,124375)
word [(0, 12)] straddles [6, 13, 27, 33]
word [(0, 1), (0, 12)] straddles [4, 5, 8]
same_piece_last_letter_and_cohort_coset True
```

T17-03: no anchor for the three height-2 straddlers of section 21.

```python
from pathlib import Path
import hashlib
path=Path('research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md')
src=path.read_text().split('T16-02: the cohort abstraction, its height-10 path and the blockers of its realization.')[1].split('```python\n')[1].split('```')[0]
assert hashlib.sha256(src.encode()).hexdigest()=='a35ed1ad07da956ccebc4a73baa59cf48f86e50d04f3dc92fb8399bc5737038c'
exec(src.split('Q=s.pose(K[0])')[0])
from exact import transpose
for p,target in [(115294,342),(165151,529),(87889,501)]:
 P=s.pose(p); assert T.dist(T.factors(P)[0])==2
 rr=Evaluator(pullback_form(P,d.N[target])).classify(C.regions.vform(p))
 assert rr[0]=='straddle'
 assert s._straddle_certificate(p,int(s.pose_id[p]),target,rr[2],rr[3])['points_checked_against_all_constraints']
 holding=[c for c in range(600) if Evaluator(pullback_form(P,d.N[c])).classify(C.regions.vform(p))[0]=='in']
 possible=[]; anchors=[]
 for c in holding:
  for q,j in enumerate(reps[1:],1):
   E=R.element(M,c,j)
   if C.kplus.index_of_matrix(matmul(transpose(E),P))>=0:
    possible.append((c,sigma[c],q))
    if sigma[c]==q: anchors.append(c)
 assert not anchors
 print(p,'holding',holding,'possible_(grip,current,required)',possible,'anchors',anchors)
```

Output:

```text
115294 holding [249, 281, 284, 313, 398] possible_(grip,current,required) [(281, 0, 4), (284, 0, 1)] anchors []
165151 holding [425, 488, 518, 564] possible_(grip,current,required) [(425, 0, 1), (564, 0, 1)] anchors []
87889 holding [495, 548, 551, 579, 593] possible_(grip,current,required) [(495, 0, 3), (593, 2, 3)] anchors []
```

T17-04: the abstraction by domain and chamber at X8.

```python
from pathlib import Path
import hashlib
path=Path('research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md')
code=path.read_text().split('T16-02: the cohort abstraction, its height-10 path and the blockers of its realization.')[1].split('```python\n')[1].split('```')[0]
assert hashlib.sha256(code.encode()).hexdigest()=='a35ed1ad07da956ccebc4a73baa59cf48f86e50d04f3dc92fb8399bc5737038c'
exec(code.split('Q=s.pose(K[0])')[0])
from exact import transpose
Js=[I]+[R.element(M,c,j) for c in (0,7,3,1,13) for j in (12,24,25,27)]
assert len(Js)==21
assert [T.dist(T.factors(g)[0]) for g in Js]==[0]+[2]*20
assert all(C.kplus.index_of_matrix(matmul(transpose(a),b))<0 for i,a in enumerate(Js) for b in Js[i+1:])
print('distinct_domains',len(Js),'region_catalogue_bound',len(Js)*len(d.orbit_id))
blocked=set()
for c,p in [(342,115294),(484,107148),(529,165151)]:
 P=s.pose(p); hits=[]
 for z,J in enumerate(Js):
  k=C.kplus.index_of_matrix(matmul(transpose(J),P))
  if k<0: continue
  o,t=C.regions.rep_of(p)
  slot=int(C.regions.piece_of[o,C.kplus.compose(k,t)])
  assert {tuple(matvec(P,v)) for v in C.regions.vertices(p)}=={tuple(matvec(J,v)) for v in C.regions.vertices(slot)}
  cuts=[e for e in range(600) if Evaluator(pullback_form(J,d.N[e])).classify(C.regions.vform(slot))[0]=='straddle']
  assert c in cuts; blocked.update(cuts); hits.append((z,slot))
  print('template',p,'code',(z,slot),'straddles',cuts)
 assert len(hits)==1
nonid=[j for j,(_,g) in enumerate(M.items) if to_tuple(g)!=I]
assert len(nonid)==59
rejected=[(c,j) for c in (342,484,529) for j in nonid if c in blocked]
assert len(rejected)==177
print('restricted_letters',3*len(nonid),'rejected',len(rejected),'reachable_states',1)
print('all_menu_letters_rejected',len(blocked)*len(nonid))
```

Output:

```text
distinct_domains 21 region_catalogue_bound 3719520
template 115294 code (7, 95579) straddles [342, 381, 425]
template 107148 code (14, 52033) straddles [484]
template 165151 code (11, 168042) straddles [463, 493, 529]
restricted_letters 177 rejected 177 reachable_states 1
all_menu_letters_rejected 413
```

E17-1: the history of K in the word of T15-01.

```python
# E17-1: the letters that move the seven pieces K in the word of T15-01, the merged word, and its replay from solved.
import sys
import numpy as np
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling',
                'research/jumbling/fixtures']
import sim, wj, restore as RS, trees as T
from exact import matmul, matvec, dot
from sim.kplus import to_tuple
from sim.kernel import Evaluator, pullback_form, rows_form
C = sim.get_context(); d = C.data; kp = C.kplus; F, M = wj.load(C, 'I_a')
I = kp.matrix(0)
K = (118527, 126380, 126382, 126384, 126385, 149644, 149650)
lab = {name: j for j, (name, m) in enumerate(M.items)}
W = [(r['grip'], lab[r['params']['label']]) for r in F['journal']['records'][:723]]
W += [(433, 12), (593, 58), (501, 22), (593, 12)]
E = lambda g, j: RS.element(M, g, j)
def hb(Q):
    l = T.factors(Q)[0]
    return T.dist(l), T.branch(l)
V = {p: C.regions.vertices(p) for p in K}
Q = I; kept = []
for n, (g, j) in enumerate(W, 1):
    ev = Evaluator(pullback_form(I, d.N[g]))
    sides = {ev.classify(rows_form({tuple(matvec(Q, v)) for v in V[p]}))[0] for p in K}
    assert sides in ({'in'}, {'out'})
    if sides == {'in'}:
        Q = to_tuple(matmul(E(g, j), Q)); kept.append((n, g, j, hb(Q)))
print('letters moving K:', len(kept), 'of', len(W), '; grips', sorted({g for _, g, _, _ in kept}))
print('heights of K after each:', [h for _, _, _, (h, b) in kept])
merged = []
for n, g, j, _ in kept:
    m = E(g, j)
    if merged and merged[-1][0] == g:
        m = to_tuple(matmul(m, merged[-1][1])); merged.pop()
    merged.append((g, to_tuple(m)))
MW = [(g, next(j for j in range(60) if to_tuple(E(g, j)) == m)) for g, m in merged]
print('merged word', MW)
s = sim.State(C, filtered=False, menu=M)
for g, j in MW:
    o = s.apply(sim.Twist(C, g, E(g, j), 'menu', {'menu': M.name}), journal=False)
    if not o.applied:
        print('blocked at', (g, j), 'by', o.straddle['piece']); break
    print('after', (g, j), 'K', hb(s.pose(K[0])), 'off-lattice', s.off_lattice_count())
c = 593
off = np.flatnonzero(~s.lattice_flags())
S = [int(p) for p in off if Evaluator(pullback_form(s.pose(int(p)), d.N[c])).classify(C.regions.vform(int(p)))[0] == 'straddle']
hs = {T.dist(T.factors(s.pose(p))[0]) for p in S}
print('cut 593: exact straddlers', len(S), 'heights', sorted(hs))
Qk = s.pose(K[0])
def kside(g):
    return Evaluator(pullback_form(I, d.N[g])).classify(rows_form({tuple(matvec(Qk, v)) for p in K for v in V[p]}))[0]
hold = {}
for p in S:
    for g in range(600):
        if Evaluator(pullback_form(s.pose(p), d.N[g])).classify(C.regions.vform(p))[0] == 'in':
            hold[g] = hold.get(g, 0) + 1
rows = []
for g, n in sorted(hold.items(), key=lambda x: (-x[1], x[0])):
    rows.append((g, n, s.classify(g, need_inside=False).status, kside(g)))
print('caps holding a straddler (cap, straddlers held, status, side of K):', rows)
clear = 0
for g, n, st, ks in rows:
    if g == c or st != 'admissible' or ks != 'out': continue
    for j in range(60):
        if to_tuple(E(g, j)) == to_tuple(I): continue
        u = sim.State(C, filtered=False, menu=M)
        u._poses = list(s._poses); u._lookup = dict(s._lookup); u.pose_id = s.pose_id.copy()
        assert u.apply(sim.Twist(C, g, E(g, j), 'menu', {'menu': M.name}), journal=False).applied
        clear += u.classify(c, need_inside=False).status == 'admissible'
print('single letters at an admissible holder that excludes K and clear cut 593:', clear)
```

Output:

```text
letters moving K: 26 of 727 ; grips [501, 521, 593]
heights of K after each: [2, 0, 2, 2, 2, 2, 0, 2, 2, 2, 2, 0, 2, 2, 2, 0, 2, 2, 2, 0, 2, 4, 4, 4, 6, 8]
merged word [(521, 8), (501, 39), (593, 46), (501, 22), (593, 12)]
after (521, 8) K (0, -1) off-lattice 0
after (501, 39) K (2, 1) off-lattice 3097
blocked at (593, 46) by 127839
cut 593: exact straddlers 160 heights [2]
caps holding a straddler (cap, straddlers held, status, side of K): [(501, 160, 'admissible', 'in'), (551, 158, 'blocked', 'in'), (580, 154, 'blocked', 'in'), (548, 145, 'blocked', 'in'), (542, 139, 'blocked', 'in'), (537, 88, 'admissible', 'out'), (582, 83, 'admissible', 'straddle'), (579, 80, 'admissible', 'in'), (560, 72, 'blocked', 'out'), (539, 68, 'blocked', 'straddle')]
single letters at an admissible holder that excludes K and clear cut 593: 0
```

E17-2: no second raise straight after a first one.

```python
# E17-2: no second raise straight after a first one. From solved, K is raised by every jumble at every cap that holds
# it whole; then every cap of another branch that holds K whole is decided exactly (NB of the out grip, or straddled).
import sys, math
from collections import Counter
import numpy as np
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling',
                'research/jumbling/fixtures']
import sim, wj, restore as RS, trees as T
from exact import matmul, matvec, dot
from sim.kplus import to_tuple
from sim.kernel import Evaluator, pullback_form, rows_form
C = sim.get_context(); d = C.data; kp = C.kplus; F, M = wj.load(C, 'I_a')
I = kp.matrix(0)
K = (118527, 126380, 126382, 126384, 126385, 149644, 149650)
J = RS.jumble_items(M); assert len(J) == 48
fl = lambda q: (q.a + q.b * math.sqrt(5)) / q.d
NF = np.array([[fl(x) for x in d.N[c]] for c in range(600)]); KAP = fl(d.KAPPA)
B = [T.branch(tuple(d.N[c])) for c in range(600)]
V = sorted({tuple(v) for p in K for v in C.regions.vertices(p)}, key=repr)
def holding(Q):
    X = [tuple(matvec(Q, v)) for v in V]
    m = (np.array([[fl(x) for x in v] for v in X]) @ NF.T - KAP).min(axis=0)
    vf = rows_form(set(X))
    return [c for c in range(600) if m[c] > -1e-6
            and Evaluator(pullback_form(I, d.N[c])).classify(vf)[0] == 'in']
def nb(a, b):
    return (d.NN * (d.NN + dot(d.N[a], d.N[b])) - 2 * d.KAPPA * d.KAPPA).sign() > 0
def hb(Q):
    l = T.factors(Q)[0]
    return T.dist(l), T.branch(l)
first = holding(I)
print('caps holding K at solved:', first)
cnt = Counter()
for c in first:
    for j in J:
        s = sim.State(C, filtered=False, menu=M)
        o = s.apply(sim.Twist(C, c, RS.element(M, c, j), 'menu', {'menu': M.name}), journal=False)
        assert o.applied and not o.uncertain
        Q = s.pose(K[0]); assert all(s.pose(p) == Q for p in K)
        h, b = hb(Q); assert h == 2 and b == B[c]
        cnt['raises to 2'] += 1
        for c2 in holding(Q):
            if B[c2] == b: continue
            cnt['candidates'] += 1
            if nb(c, c2):
                cnt['in NB of the raiser'] += 1; continue
            cl = s.classify(c2, need_inside=False)
            if cl.status == 'admissible':
                cnt['admissible'] += 1
            else:
                p = cl.straddle['piece']
                cnt[f'straddled by a piece of height {T.dist(T.factors(s.pose(p))[0])}'] += 1
print(dict(cnt))
```

Output:

```text
caps holding K at solved: [293, 334, 391, 418, 445, 498, 504, 515, 521]
{'raises to 2': 432, 'candidates': 2160, 'in NB of the raiser': 1728, 'straddled by a piece of height 2': 432}
```

E17-3 and E17-4: the causal past of K, a return of 593 and a raise to height 10.

```python
# E17-3 and E17-4: the causal past of K in the word of T15-01, then returns of out grips and a raise to height 10.
import sys, math, time
import numpy as np
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling',
                'research/jumbling/fixtures']
import sim, wj, restore as RS, trees as T
from exact import matmul, matvec, dot
from sim.kplus import to_tuple
from sim.kernel import Evaluator, pullback_form, rows_form
t0 = time.time()
C = sim.get_context(); d = C.data; kp = C.kplus; F, M = wj.load(C, 'I_a')
I = kp.matrix(0); RG = C.regions
K = (118527, 126380, 126382, 126384, 126385, 149644, 149650)
J = RS.jumble_items(M)
E = lambda g, j: RS.element(M, g, j)
ID = next(j for j in range(60) if to_tuple(E(0, j)) == to_tuple(I))
fl = lambda q: (q.a + q.b * math.sqrt(5)) / q.d
flm = lambda m: np.array([[fl(x) for x in r] for r in m])
NF = np.array([[fl(x) for x in d.N[c]] for c in range(600)]); KAP = fl(d.KAPPA)
B = [T.branch(tuple(d.N[c])) for c in range(600)]
NB = [{b for b in range(600) if b != a and
       (d.NN * (d.NN + dot(d.N[a], d.N[b])) - 2 * d.KAPPA * d.KAPPA).sign() > 0} for a in range(600)]
VK = sorted({tuple(v) for p in K for v in RG.vertices(p)}, key=repr)
lab = {name: j for j, (name, m) in enumerate(M.items)}
W = [(r['grip'], lab[r['params']['label']]) for r in F['journal']['records'][:723]]
W += [(433, 12), (593, 58), (501, 22), (593, 12)]
def hb(Q):
    l = T.factors(Q)[0]
    return T.dist(l), T.branch(l)
def isin(m): return kp.index_of_matrix(to_tuple(m)) >= 0
def replay(W, record=False):
    s = sim.State(C, filtered=False, menu=M); G = [I] * 600; moved = []
    for n, (g, j) in enumerate(W):
        cl = s.classify(g, need_inside=True, stop_at_straddle=True)
        if cl.straddle is not None or cl.uncertain:
            return s, G, moved, n
        s._commit(sim.Twist(C, g, E(g, j), 'menu', {'menu': M.name}), cl.inside, False)
        G[g] = to_tuple(matmul(E(g, j), G[g]))
        if record: moved.append(set(cl.inside.tolist()))
    return s, G, moved, None
def straddlers(s, c):
    kidx = s._kpose()[s.pose_id]; out = []
    for p in np.flatnonzero(kidx < 0).tolist():
        h = RG.frows(p)[0] @ flm(s.pose(p)).T @ NF[c] - KAP
        if h.min() < 1e-7 and h.max() > -1e-7 and \
           Evaluator(pullback_form(s.pose(p), d.N[c])).classify(RG.vform(p))[0] == 'straddle':
            out.append(p)
    return out
s, G, moved, bad = replay(W, record=True); assert bad is None
keep = {n for n, mv in enumerate(moved) if K[0] in mv}
print('X8: off-lattice', s.off_lattice_count(), '; out grips', sum(not isin(G[c]) for c in range(600)),
      '; letters moving K:', len(keep))
while True:
    sub = sorted(keep)
    s, G, _, bad = replay([W[n] for n in sub])
    if bad is None: break
    n0 = sub[bad]; S = set(straddlers(s, W[n0][0]))
    add = {n for n in range(n0) if moved[n] & S} - keep
    print('letter', n0 + 1, W[n0], 'blocked by', len(S), 'pieces; adding', len(add), 'earlier letters')
    assert add; keep |= add
Wc = [W[n] for n in sorted(keep)]
O = sorted(c for c in range(600) if not isin(G[c]))
print('causal past:', len(Wc), 'letters; K', hb(s.pose(K[0])), '; off-lattice', s.off_lattice_count(), '; out grips', O)
def holding(Q):
    X = [tuple(matvec(Q, v)) for v in VK]
    m = (np.array([[fl(x) for x in v] for v in X]) @ NF.T - KAP).min(axis=0)
    vf = rows_form(set(X))
    return [c for c in range(600) if m[c] > -1e-6 and Evaluator(pullback_form(I, d.N[c])).classify(vf)[0] == 'in']
def clone(s):
    u = sim.State(C, filtered=False, menu=M)
    u._poses = list(s._poses); u._lookup = dict(s._lookup); u.pose_id = s.pose_id.copy()
    return u
def raisers(s, G, report=False):
    Q = s.pose(K[0]); h, b = hb(Q); O = [c for c in range(600) if not isin(G[c])]; out = []
    for c in holding(Q):
        if B[c] == b: continue
        nbo = [x for x in O if c in NB[x]]
        st = 'NB' if nbo else s.classify(c, need_inside=False).status
        if report:
            print('  raiser', c, 'NB of out', nbo, st, *([] if nbo or st == 'admissible' else [len(straddlers(s, c))]))
        if st == 'admissible': out.append(c)
    return out
print('at the causal past:'); raisers(s, G, report=True)
found = None
for x in O:
    if found or s.classify(x, need_inside=False).status != 'admissible': continue
    for j in range(60):
        if j == ID or not isin(matmul(E(x, j), G[x])): continue
        u = clone(s); Gu = list(G)
        assert u.apply(sim.Twist(C, x, E(x, j), 'menu', {'menu': M.name}), journal=False).applied
        Gu[x] = to_tuple(matmul(E(x, j), G[x]))
        if hb(u.pose(K[0]))[0] < 8: continue
        for c in raisers(u, Gu):
            v = clone(u)
            assert v.apply(sim.Twist(C, c, E(c, J[0]), 'menu', {'menu': M.name}), journal=False).applied
            found = [(x, j), (c, J[0])]
            print('return', (x, j), 'keeps K at', hb(u.pose(K[0])), '; raise', (c, J[0]), 'takes K to', hb(v.pose(K[0])))
            print('raisers after the return:'); raisers(u, Gu, report=True)
            break
        if found: break
print('word to height 10:', len(Wc) + 2, 'letters; seconds', round(time.time() - t0))
```

Output:

```text
X8: off-lattice 120663 ; out grips 55 ; letters moving K: 26
letter 676 (593, 23) blocked by 160 pieces; adding 96 earlier letters
letter 676 (593, 23) blocked by 34 pieces; adding 13 earlier letters
letter 568 (280, 49) blocked by 34 pieces; adding 10 earlier letters
causal past: 145 letters; K (8, 2) ; off-lattice 33434 ; out grips [217, 218, 280, 284, 324, 353, 442, 484, 501, 525, 537, 564, 593]
at the causal past:
  raiser 529 NB of out [484, 564] NB
  raiser 547 NB of out [484, 593] NB
  raiser 555 NB of out [501, 564, 593] NB
  raiser 562 NB of out [564, 593] NB
  raiser 580 NB of out [501, 593] NB
  raiser 585 NB of out [564, 593] NB
return (593, 15) keeps K at (8, 2) ; raise (582, 12) takes K to (10, 1)
raisers after the return:
  raiser 551 NB of out [501, 537] NB
  raiser 555 NB of out [501, 564] NB
  raiser 580 NB of out [501] NB
  raiser 582 NB of out [] admissible
word to height 10: 147 letters; seconds 246
```

E17-4, checked letter by letter: the word of 147 letters from the solved state.

```python
# E17-4 checked letter by letter: the word of 147 letters, applied from the solved state by the exact simulator.
# Prints every letter that moves the seven pieces K, the final digest and the heights of all pieces.
import sys
from collections import Counter
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling',
                'research/jumbling/fixtures']
import sim, wj, restore as RS, trees as T
from exact import matvec
from sim.kernel import Evaluator, pullback_form, rows_form
C = sim.get_context(); d = C.data; F, M = wj.load(C, 'I_a'); I = C.kplus.matrix(0)
K = (118527, 126380, 126382, 126384, 126385, 149644, 149650)
W = [(521, 35), (557, 40), (501, 29), (353, 37), (501, 51), (218, 32), (217, 24), (557, 23), (593, 48),
     (218, 47), (501, 13), (501, 15), (521, 27), (537, 33), (537, 25), (218, 40), (501, 27), (218, 58),
     (324, 59), (218, 34), (324, 54), (484, 26), (218, 18), (537, 43), (537, 25), (484, 53), (484, 41),
     (537, 16), (521, 17), (484, 46), (501, 50), (537, 28), (442, 20), (557, 13), (564, 58), (501, 32),
     (501, 57), (218, 34), (484, 18), (537, 31), (537, 43), (360, 23), (360, 36), (501, 59), (360, 51),
     (218, 34), (501, 51), (521, 57), (501, 25), (218, 39), (442, 52), (442, 24), (537, 59), (521, 39),
     (218, 36), (217, 23), (217, 19), (564, 51), (218, 50), (564, 49), (218, 40), (442, 55), (110, 20),
     (501, 59), (521, 52), (521, 33), (564, 40), (501, 21), (512, 38), (537, 53), (484, 53), (218, 49),
     (217, 21), (217, 38), (521, 38), (512, 40), (564, 12), (217, 12), (537, 43), (484, 43), (537, 32),
     (217, 37), (525, 16), (217, 49), (525, 15), (110, 51), (324, 29), (484, 17), (217, 24), (217, 43),
     (521, 48), (537, 23), (110, 24), (217, 26), (521, 38), (521, 44), (564, 34), (501, 58), (501, 35),
     (110, 43), (521, 42), (218, 31), (501, 59), (537, 54), (564, 15), (324, 52), (110, 16), (537, 50),
     (284, 24), (110, 58), (218, 44), (564, 41), (110, 24), (501, 23), (110, 54), (218, 59), (501, 26),
     (110, 51), (564, 39), (537, 51), (280, 49), (501, 20), (564, 55), (537, 53), (501, 49), (280, 25),
     (501, 50), (564, 44), (501, 35), (537, 22), (501, 45), (501, 39), (537, 38), (217, 19), (537, 29),
     (218, 37), (218, 28), (525, 47), (537, 34), (537, 12), (593, 23), (593, 36), (593, 58), (501, 22),
     (593, 12), (593, 15), (582, 12)]
def hb(Q):
    l = T.factors(Q)[0]
    return T.dist(l), T.branch(l)
V = [v for p in K for v in C.regions.vertices(p)]
s = sim.State(C, filtered=False, menu=M)
for n, (g, j) in enumerate(W, 1):
    Q0 = s.pose(K[0])
    vf = rows_form({tuple(matvec(Q0, v)) for v in V})
    side = Evaluator(pullback_form(I, d.N[g])).classify(vf)[0]
    assert side in ('in', 'out')
    o = s.apply(sim.Twist(C, g, RS.element(M, g, j), 'menu', {'menu': M.name}), journal=False)
    assert o.applied and not o.uncertain, (n, g, j, o.status)
    Q = s.pose(K[0]); assert all(s.pose(p) == Q for p in K)
    assert (Q != Q0) == (side == 'in')
    if side == 'in':
        print(n, (g, j), 'branch of grip', T.branch(tuple(d.N[g])), 'K', hb(Q0), '->', hb(Q))
print('letters', len(W), 'digest', s.digest())
hp = {}
for i, P in enumerate(s._poses):
    hp[i] = 0 if P.kidx >= 0 else T.dist(T.factors(P.matrix)[0])
print('heights', sorted(Counter(hp[int(i)] for i in s.pose_id).items()))
```

Output:

```text
1 (521, 35) branch of grip 3 K (0, -1) -> (2, 3)
13 (521, 27) branch of grip 3 K (2, 3) -> (0, -1)
29 (521, 17) branch of grip 3 K (0, -1) -> (2, 3)
48 (521, 57) branch of grip 3 K (2, 3) -> (2, 3)
54 (521, 39) branch of grip 3 K (2, 3) -> (2, 3)
65 (521, 52) branch of grip 3 K (2, 3) -> (2, 3)
66 (521, 33) branch of grip 3 K (2, 3) -> (0, -1)
75 (521, 38) branch of grip 3 K (0, -1) -> (2, 3)
91 (521, 48) branch of grip 3 K (2, 3) -> (2, 3)
95 (521, 38) branch of grip 3 K (2, 3) -> (2, 3)
96 (521, 44) branch of grip 3 K (2, 3) -> (2, 3)
101 (521, 42) branch of grip 3 K (2, 3) -> (0, -1)
103 (501, 59) branch of grip 1 K (0, -1) -> (2, 1)
114 (501, 23) branch of grip 1 K (2, 1) -> (2, 1)
117 (501, 26) branch of grip 1 K (2, 1) -> (2, 1)
122 (501, 20) branch of grip 1 K (2, 1) -> (0, -1)
125 (501, 49) branch of grip 1 K (0, -1) -> (2, 1)
127 (501, 50) branch of grip 1 K (2, 1) -> (2, 1)
129 (501, 35) branch of grip 1 K (2, 1) -> (2, 1)
131 (501, 45) branch of grip 1 K (2, 1) -> (0, -1)
132 (501, 39) branch of grip 1 K (0, -1) -> (2, 1)
141 (593, 23) branch of grip 2 K (2, 1) -> (4, 2)
142 (593, 36) branch of grip 2 K (4, 2) -> (4, 2)
143 (593, 58) branch of grip 2 K (4, 2) -> (4, 2)
144 (501, 22) branch of grip 1 K (4, 2) -> (6, 1)
145 (593, 12) branch of grip 2 K (6, 1) -> (8, 2)
146 (593, 15) branch of grip 2 K (8, 2) -> (8, 2)
147 (582, 12) branch of grip 1 K (8, 2) -> (10, 1)
letters 147 digest 31c90699294d97b702312c74643e5d7462f74612925642d60a8bc9ec29375849
heights [(0, 143805), (2, 32605), (4, 703), (10, 7)]
```

E17-5: the height-10 state X10 and the returns of 582.

```python
# E17-5: the height-10 state X10 reached by the word of E17-4. For every cap of another branch that holds K whole: the
# out grips whose 16 nearest grips contain it, its exact straddlers and the caps that hold all of them. Then every
# letter at 582 that returns it, with K's height after it.
import sys, math, time
from collections import Counter
import numpy as np
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling',
                'research/jumbling/fixtures']
import sim, wj, restore as RS, trees as T
from exact import matmul, matvec, dot
from sim.kplus import to_tuple
from sim.kernel import Evaluator, pullback_form, rows_form
t0 = time.time()
C = sim.get_context(); d = C.data; kp = C.kplus; F, M = wj.load(C, 'I_a')
I = kp.matrix(0); RG = C.regions
K = (118527, 126380, 126382, 126384, 126385, 149644, 149650)
E = lambda g, j: RS.element(M, g, j)
ID = next(j for j in range(60) if to_tuple(E(0, j)) == to_tuple(I))
fl = lambda q: (q.a + q.b * math.sqrt(5)) / q.d
flm = lambda m: np.array([[fl(x) for x in r] for r in m])
NF = np.array([[fl(x) for x in d.N[c]] for c in range(600)]); KAP = fl(d.KAPPA)
B = [T.branch(tuple(d.N[c])) for c in range(600)]
NB = [{b for b in range(600) if b != a and
       (d.NN * (d.NN + dot(d.N[a], d.N[b])) - 2 * d.KAPPA * d.KAPPA).sign() > 0} for a in range(600)]
VK = sorted({tuple(v) for p in K for v in RG.vertices(p)}, key=repr)
W = [(521, 35), (557, 40), (501, 29), (353, 37), (501, 51), (218, 32), (217, 24), (557, 23), (593, 48),
     (218, 47), (501, 13), (501, 15), (521, 27), (537, 33), (537, 25), (218, 40), (501, 27), (218, 58),
     (324, 59), (218, 34), (324, 54), (484, 26), (218, 18), (537, 43), (537, 25), (484, 53), (484, 41),
     (537, 16), (521, 17), (484, 46), (501, 50), (537, 28), (442, 20), (557, 13), (564, 58), (501, 32),
     (501, 57), (218, 34), (484, 18), (537, 31), (537, 43), (360, 23), (360, 36), (501, 59), (360, 51),
     (218, 34), (501, 51), (521, 57), (501, 25), (218, 39), (442, 52), (442, 24), (537, 59), (521, 39),
     (218, 36), (217, 23), (217, 19), (564, 51), (218, 50), (564, 49), (218, 40), (442, 55), (110, 20),
     (501, 59), (521, 52), (521, 33), (564, 40), (501, 21), (512, 38), (537, 53), (484, 53), (218, 49),
     (217, 21), (217, 38), (521, 38), (512, 40), (564, 12), (217, 12), (537, 43), (484, 43), (537, 32),
     (217, 37), (525, 16), (217, 49), (525, 15), (110, 51), (324, 29), (484, 17), (217, 24), (217, 43),
     (521, 48), (537, 23), (110, 24), (217, 26), (521, 38), (521, 44), (564, 34), (501, 58), (501, 35),
     (110, 43), (521, 42), (218, 31), (501, 59), (537, 54), (564, 15), (324, 52), (110, 16), (537, 50),
     (284, 24), (110, 58), (218, 44), (564, 41), (110, 24), (501, 23), (110, 54), (218, 59), (501, 26),
     (110, 51), (564, 39), (537, 51), (280, 49), (501, 20), (564, 55), (537, 53), (501, 49), (280, 25),
     (501, 50), (564, 44), (501, 35), (537, 22), (501, 45), (501, 39), (537, 38), (217, 19), (537, 29),
     (218, 37), (218, 28), (525, 47), (537, 34), (537, 12), (593, 23), (593, 36), (593, 58), (501, 22),
     (593, 12), (593, 15), (582, 12)]
def hb(Q):
    l = T.factors(Q)[0]
    return T.dist(l), T.branch(l)
def isin(m): return kp.index_of_matrix(to_tuple(m)) >= 0
def twist(g, j): return sim.Twist(C, g, E(g, j), 'menu', {'menu': M.name})
s = sim.State(C, filtered=False, menu=M); G = [I] * 600
for g, j in W:
    assert s.apply(twist(g, j), journal=False).applied
    G[g] = to_tuple(matmul(E(g, j), G[g]))
Q = s.pose(K[0]); assert all(s.pose(p) == Q for p in K) and hb(Q) == (10, 1)
O = [c for c in range(600) if not isin(G[c])]
print('X10', s.digest(), 'K', hb(Q), 'off-lattice', s.off_lattice_count())
print('out grips (grip, branch, status):', [(x, B[x], s.classify(x, need_inside=False).status) for x in O])
def holding(Q):
    X = [tuple(matvec(Q, v)) for v in VK]
    m = (np.array([[fl(x) for x in v] for v in X]) @ NF.T - KAP).min(axis=0)
    vf = rows_form(set(X))
    return [c for c in range(600) if m[c] > -1e-6 and Evaluator(pullback_form(I, d.N[c])).classify(vf)[0] == 'in']
def side(s, p, c): return Evaluator(pullback_form(s.pose(p), d.N[c])).classify(RG.vform(p))[0]
def hrange(s, p): return RG.frows(p)[0] @ flm(s.pose(p)).T @ NF.T - KAP
def straddlers(s, c):
    kidx = s._kpose()[s.pose_id]; out = []
    for p in np.flatnonzero(kidx < 0).tolist():
        h = RG.frows(p)[0] @ flm(s.pose(p)).T @ NF[c] - KAP
        if h.min() < 1e-7 and h.max() > -1e-7 and side(s, p, c) == 'straddle':
            out.append(p)
    return out
hold = holding(Q)
print('caps holding K whole (cap, branch):', [(c, B[c]) for c in hold])
for c in hold:
    if B[c] == 1: continue
    Sx = straddlers(s, c); common = None
    for p in Sx:
        cand = np.flatnonzero(hrange(s, p).min(axis=0) > -1e-7).tolist()
        cand = [x for x in cand if common is None or x in common]
        common = {x for x in cand if side(s, p, x) == 'in'}
        if not common: break
    print('cap', c, 'branch', B[c], '; NB of out', [x for x in O if c in NB[x]], ';', s.classify(c, need_inside=False).status,
          '; straddlers', len(Sx), 'heights', sorted(Counter(hb(s.pose(p))[0] for p in Sx).items()),
          '; caps holding all of them', sorted(common or []))
for j in range(60):
    if j == ID or not isin(matmul(E(582, j), G[582])): continue
    u = sim.State(C, filtered=False, menu=M)
    u._poses = list(s._poses); u._lookup = dict(s._lookup); u.pose_id = s.pose_id.copy()
    assert u.apply(twist(582, j), journal=False).applied
    Qu = u.pose(K[0]); assert all(u.pose(p) == Qu for p in K)
    print('return (582, %d): K' % j, hb(Qu))
print('seconds', round(time.time() - t0))
```

Output:

```text
X10 31c90699294d97b702312c74643e5d7462f74612925642d60a8bc9ec29375849 K (10, 1) off-lattice 33315
out grips (grip, branch, status): [(217, 1, 'admissible'), (218, 1, 'admissible'), (280, 3, 'admissible'), (284, 1, 'admissible'), (324, 1, 'admissible'), (353, 1, 'admissible'), (442, 1, 'admissible'), (484, 1, 'admissible'), (501, 1, 'admissible'), (525, 4, 'admissible'), (537, 1, 'blocked'), (564, 2, 'admissible'), (582, 1, 'admissible')]
caps holding K whole (cap, branch): [(542, 2), (548, 4), (551, 0), (560, 4), (580, 3), (582, 1), (593, 2)]
cap 542 branch 2 ; NB of out [501, 537, 582] ; blocked ; straddlers 333 heights [(2, 301), (4, 32)] ; caps holding all of them []
cap 548 branch 4 ; NB of out [442, 501, 537] ; blocked ; straddlers 240 heights [(2, 240)] ; caps holding all of them []
cap 551 branch 0 ; NB of out [501, 537, 582] ; blocked ; straddlers 444 heights [(2, 393), (4, 51)] ; caps holding all of them []
cap 560 branch 4 ; NB of out [537, 582] ; blocked ; straddlers 439 heights [(2, 321), (4, 118)] ; caps holding all of them []
cap 580 branch 3 ; NB of out [501, 582] ; blocked ; straddlers 291 heights [(2, 230), (4, 61)] ; caps holding all of them []
cap 593 branch 2 ; NB of out [582] ; blocked ; straddlers 95 heights [(2, 49), (4, 46)] ; caps holding all of them [582]
return (582, 14): K (8, 4)
return (582, 21): K (8, 0)
return (582, 23): K (8, 3)
return (582, 43): K (8, 2)
return (582, 44): K (8, 3)
return (582, 45): K (8, 2)
return (582, 46): K (8, 4)
return (582, 47): K (8, 0)
return (582, 48): K (8, 3)
return (582, 49): K (8, 2)
return (582, 50): K (8, 4)
return (582, 51): K (8, 0)
seconds 29
```

E17-6: a turn at the raiser, and height 12.

```python
# E17-6: a turn at the raiser. The word of E17-4 is replayed from solved (every letter applied by the exact simulator).
# At X10, for each of the letters (582, 18) and (582, 59) at the out grip 582: K's height and branch after it, and every
# cap of another branch that holds K whole, with the out grips whose 16 nearest grips contain it and its exact status.
# Then (582, 18) and (576, 12) take K to height 12.
import sys, math
from collections import Counter
import numpy as np
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling',
                'research/jumbling/fixtures']
import sim, wj, restore as RS, trees as T
from exact import matmul, matvec, dot
from sim.kplus import to_tuple
from sim.kernel import Evaluator, pullback_form, rows_form
C = sim.get_context(); d = C.data; kp = C.kplus; F, M = wj.load(C, 'I_a')
I = kp.matrix(0); RG = C.regions
K = (118527, 126380, 126382, 126384, 126385, 149644, 149650)
E = lambda g, j: RS.element(M, g, j)
fl = lambda q: (q.a + q.b * math.sqrt(5)) / q.d
NF = np.array([[fl(x) for x in d.N[c]] for c in range(600)]); KAP = fl(d.KAPPA)
B = [T.branch(tuple(d.N[c])) for c in range(600)]
NB = [{b for b in range(600) if b != a and
       (d.NN * (d.NN + dot(d.N[a], d.N[b])) - 2 * d.KAPPA * d.KAPPA).sign() > 0} for a in range(600)]
VK = sorted({tuple(v) for p in K for v in RG.vertices(p)}, key=repr)
W = [(521, 35), (557, 40), (501, 29), (353, 37), (501, 51), (218, 32), (217, 24), (557, 23), (593, 48),
     (218, 47), (501, 13), (501, 15), (521, 27), (537, 33), (537, 25), (218, 40), (501, 27), (218, 58),
     (324, 59), (218, 34), (324, 54), (484, 26), (218, 18), (537, 43), (537, 25), (484, 53), (484, 41),
     (537, 16), (521, 17), (484, 46), (501, 50), (537, 28), (442, 20), (557, 13), (564, 58), (501, 32),
     (501, 57), (218, 34), (484, 18), (537, 31), (537, 43), (360, 23), (360, 36), (501, 59), (360, 51),
     (218, 34), (501, 51), (521, 57), (501, 25), (218, 39), (442, 52), (442, 24), (537, 59), (521, 39),
     (218, 36), (217, 23), (217, 19), (564, 51), (218, 50), (564, 49), (218, 40), (442, 55), (110, 20),
     (501, 59), (521, 52), (521, 33), (564, 40), (501, 21), (512, 38), (537, 53), (484, 53), (218, 49),
     (217, 21), (217, 38), (521, 38), (512, 40), (564, 12), (217, 12), (537, 43), (484, 43), (537, 32),
     (217, 37), (525, 16), (217, 49), (525, 15), (110, 51), (324, 29), (484, 17), (217, 24), (217, 43),
     (521, 48), (537, 23), (110, 24), (217, 26), (521, 38), (521, 44), (564, 34), (501, 58), (501, 35),
     (110, 43), (521, 42), (218, 31), (501, 59), (537, 54), (564, 15), (324, 52), (110, 16), (537, 50),
     (284, 24), (110, 58), (218, 44), (564, 41), (110, 24), (501, 23), (110, 54), (218, 59), (501, 26),
     (110, 51), (564, 39), (537, 51), (280, 49), (501, 20), (564, 55), (537, 53), (501, 49), (280, 25),
     (501, 50), (564, 44), (501, 35), (537, 22), (501, 45), (501, 39), (537, 38), (217, 19), (537, 29),
     (218, 37), (218, 28), (525, 47), (537, 34), (537, 12), (593, 23), (593, 36), (593, 58), (501, 22),
     (593, 12), (593, 15), (582, 12)]
def hb(Q):
    l = T.factors(Q)[0]
    return T.dist(l), T.branch(l)
def isin(m): return kp.index_of_matrix(to_tuple(m)) >= 0
def holding(Q):
    X = [tuple(matvec(Q, v)) for v in VK]
    m = (np.array([[fl(x) for x in v] for v in X]) @ NF.T - KAP).min(axis=0)
    vf = rows_form(set(X))
    return [c for c in range(600) if m[c] > -1e-6 and Evaluator(pullback_form(I, d.N[c])).classify(vf)[0] == 'in']
def apply(s, G, g, j):
    o = s.apply(sim.Twist(C, g, E(g, j), 'menu', {'menu': M.name}), journal=False)
    assert o.applied and not o.uncertain, (g, j)
    G[g] = to_tuple(matmul(E(g, j), G[g]))
def clone(s):
    u = sim.State(C, filtered=False, menu=M)
    u._poses = list(s._poses); u._lookup = dict(s._lookup); u.pose_id = s.pose_id.copy()
    return u
s = sim.State(C, filtered=False, menu=M); G = [I] * 600
for g, j in W: apply(s, G, g, j)
print('X10', s.digest(), 'K', hb(s.pose(K[0])), '; out grips', [c for c in range(600) if not isin(G[c])])
for x in [(582, 18), (582, 59)]:
    u = clone(s); Gu = list(G); apply(u, Gu, *x)
    Q = u.pose(K[0]); assert all(u.pose(p) == Q for p in K)
    h, b = hb(Q); O = [c for c in range(600) if not isin(Gu[c])]
    rows = [(c, B[c], [y for y in O if y != c and c in NB[y]], u.classify(c, need_inside=False).status)
            for c in holding(Q) if B[c] != b]
    print('after', x, ': 582 out', not isin(Gu[582]), '; K', (h, b), '; caps of another branch holding K (cap, branch, NB of out, status):', rows)
for g, j in [(582, 18), (576, 12)]: apply(s, G, g, j)
Q = s.pose(K[0]); assert all(s.pose(p) == Q for p in K)
print('after (582, 18), (576, 12): K', hb(Q), '; letters', len(W) + 2, '; digest', s.digest())
hp = {}
for i, P in enumerate(s._poses):
    hp[i] = 0 if P.kidx >= 0 else T.dist(T.factors(P.matrix)[0])
print('heights', sorted(Counter(hp[int(i)] for i in s.pose_id).items()))
```

Output:

```text
X10 31c90699294d97b702312c74643e5d7462f74612925642d60a8bc9ec29375849 K (10, 1) ; out grips [217, 218, 280, 284, 324, 353, 442, 484, 501, 525, 537, 564, 582]
after (582, 18) : 582 out True ; K (10, 1) ; caps of another branch holding K (cap, branch, NB of out, status): [(560, 4, [537, 582], 'blocked'), (565, 3, [537, 582], 'blocked'), (576, 4, [], 'admissible'), (590, 2, [582], 'blocked'), (596, 0, [582], 'blocked')]
after (582, 59) : 582 out True ; K (10, 1) ; caps of another branch holding K (cap, branch, NB of out, status): [(570, 2, [582], 'blocked'), (574, 4, [582], 'blocked'), (595, 3, [582], 'blocked'), (596, 0, [582], 'blocked'), (597, 2, [], 'admissible')]
after (582, 18), (576, 12): K (12, 4) ; letters 149 ; digest 7bb0df3ebb0e55a89135921955ae592e24249da89c16e429be0c18487f6a9aed
heights [(0, 141655), (2, 34755), (4, 703), (12, 7)]
```

E17-7: the relay repeated to height 14, and TH at 526.

```python
# E17-7: the relay repeated, and TH at 526. From the word of E17-6 (height 12), search by iterative deepening, at
# most two letters before a raise, over letters at admissible caps that hold K whole (these come first) and returns of
# out grips; K must keep its height. At the first state where a cap of another branch holds K whole, is in NB of no out
# grip and is admissible, raise K there by the first jumble that raises it and replay the whole word from solved. This
# reaches height 14 at cap 526. Then the number of earlier letters at 526, and for every letter at 526: whether it
# returns 526 (K drops), and otherwise every cap other than 526 that holds K whole after it, and whether all of them are
# in NB of 526. Every letter is applied by the exact simulator; floating point only preselects caps for exact tests.
import sys, math, time
import numpy as np
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling',
                'research/jumbling/fixtures']
import sim, wj, restore as RS, trees as T
from exact import matmul, matvec, dot
from sim.kplus import to_tuple
from sim.kernel import Evaluator, pullback_form, rows_form
HMAX = 14
t0 = time.time()
C = sim.get_context(); d = C.data; kp = C.kplus; F, M = wj.load(C, 'I_a')
I = kp.matrix(0); RG = C.regions
K = (118527, 126380, 126382, 126384, 126385, 149644, 149650)
J = RS.jumble_items(M)
E = lambda g, j: RS.element(M, g, j)
ID = next(j for j in range(60) if to_tuple(E(0, j)) == to_tuple(I))
L = [j for j in range(60) if j != ID]
fl = lambda q: (q.a + q.b * math.sqrt(5)) / q.d
NF = np.array([[fl(x) for x in d.N[c]] for c in range(600)]); KAP = fl(d.KAPPA)
B = [T.branch(tuple(d.N[c])) for c in range(600)]
NB = [{b for b in range(600) if b != a and
       (d.NN * (d.NN + dot(d.N[a], d.N[b])) - 2 * d.KAPPA * d.KAPPA).sign() > 0} for a in range(600)]
VK = sorted({tuple(v) for p in K for v in RG.vertices(p)}, key=repr)
W = [(521, 35), (557, 40), (501, 29), (353, 37), (501, 51), (218, 32), (217, 24), (557, 23), (593, 48),
     (218, 47), (501, 13), (501, 15), (521, 27), (537, 33), (537, 25), (218, 40), (501, 27), (218, 58),
     (324, 59), (218, 34), (324, 54), (484, 26), (218, 18), (537, 43), (537, 25), (484, 53), (484, 41),
     (537, 16), (521, 17), (484, 46), (501, 50), (537, 28), (442, 20), (557, 13), (564, 58), (501, 32),
     (501, 57), (218, 34), (484, 18), (537, 31), (537, 43), (360, 23), (360, 36), (501, 59), (360, 51),
     (218, 34), (501, 51), (521, 57), (501, 25), (218, 39), (442, 52), (442, 24), (537, 59), (521, 39),
     (218, 36), (217, 23), (217, 19), (564, 51), (218, 50), (564, 49), (218, 40), (442, 55), (110, 20),
     (501, 59), (521, 52), (521, 33), (564, 40), (501, 21), (512, 38), (537, 53), (484, 53), (218, 49),
     (217, 21), (217, 38), (521, 38), (512, 40), (564, 12), (217, 12), (537, 43), (484, 43), (537, 32),
     (217, 37), (525, 16), (217, 49), (525, 15), (110, 51), (324, 29), (484, 17), (217, 24), (217, 43),
     (521, 48), (537, 23), (110, 24), (217, 26), (521, 38), (521, 44), (564, 34), (501, 58), (501, 35),
     (110, 43), (521, 42), (218, 31), (501, 59), (537, 54), (564, 15), (324, 52), (110, 16), (537, 50),
     (284, 24), (110, 58), (218, 44), (564, 41), (110, 24), (501, 23), (110, 54), (218, 59), (501, 26),
     (110, 51), (564, 39), (537, 51), (280, 49), (501, 20), (564, 55), (537, 53), (501, 49), (280, 25),
     (501, 50), (564, 44), (501, 35), (537, 22), (501, 45), (501, 39), (537, 38), (217, 19), (537, 29),
     (218, 37), (218, 28), (525, 47), (537, 34), (537, 12), (593, 23), (593, 36), (593, 58), (501, 22),
     (593, 12), (593, 15), (582, 12),
     (582, 18), (576, 12)]
def hb(Q):
    l = T.factors(Q)[0]
    return T.dist(l), T.branch(l)
def isin(m): return kp.index_of_matrix(to_tuple(m)) >= 0
def holding(Q):
    X = [tuple(matvec(Q, v)) for v in VK]
    m = (np.array([[fl(x) for x in v] for v in X]) @ NF.T - KAP).min(axis=0)
    vf = rows_form(set(X))
    return [c for c in range(600) if m[c] > -1e-6 and Evaluator(pullback_form(I, d.N[c])).classify(vf)[0] == 'in']
def step(s, G, g, j):
    o = s.apply(sim.Twist(C, g, E(g, j), 'menu', {'menu': M.name}), journal=False)
    if not (o.applied and not o.uncertain): return False
    G[g] = to_tuple(matmul(E(g, j), G[g])); return True
def clone(s):
    u = sim.State(C, filtered=False, menu=M)
    u._poses = list(s._poses); u._lookup = dict(s._lookup); u.pose_id = s.pose_id.copy()
    return u
def outs(G): return [c for c in range(600) if not isin(G[c])]
def replay(W):
    s = sim.State(C, filtered=False, menu=M); G = [I] * 600
    for g, j in W: assert step(s, G, g, j), (g, j)
    return s, G
def raisers(s, G):
    Q = s.pose(K[0]); h, b = hb(Q); O = outs(G)
    return [c for c in holding(Q) if B[c] != b and not any(c in NB[x] for x in O if x != c)
            and s.classify(c, need_inside=False).status == 'admissible']
def moves(s, G):
    Q = s.pose(K[0]); O = set(outs(G)); hold = set(holding(Q)); res = []
    for x in sorted(hold | O):
        if any(x in NB[y] for y in O if y != x): continue
        if s.classify(x, need_inside=False).status != 'admissible': continue
        for j in L:
            if x in hold or (x in O and isin(matmul(E(x, j), G[x]))): res.append((0 if x in hold else 1, x, j))
    return [(x, j) for _, x, j in sorted(res)]
def search(s, G, h, path, seen, depth):
    for c in raisers(s, G):
        for j in J:
            u = clone(s); Gu = list(G); assert step(u, Gu, c, j)
            Qu = u.pose(K[0])
            if all(u.pose(p) == Qu for p in K) and hb(Qu)[0] >= h + 2: return path + [(c, j)]
    if depth == 2: return None
    for x, j in moves(s, G):
        u = clone(s); Gu = list(G)
        if not step(u, Gu, x, j): continue
        Qu = u.pose(K[0])
        if not all(u.pose(p) == Qu for p in K) or hb(Qu)[0] < h: continue
        key = (u.digest(), tuple((g, Gu[g]) for g in outs(Gu)))
        if key in seen: continue
        seen.add(key)
        r = search(u, Gu, h, path + [(x, j)], seen, depth + 1)
        if r: return r
    return None
s, G = replay(W)
Q = s.pose(K[0]); assert all(s.pose(p) == Q for p in K); h = hb(Q)[0]
print('start: K', hb(Q), '; letters', len(W), '; digest', s.digest(), flush=True)
while h < HMAX:
    tail = None
    for D in range(3):
        tail = search(s, G, h, [], set(), 2 - D)
        if tail: break
    if not tail:
        print('no raise within two letters at height', h, flush=True); break
    W = W + tail
    s, G = replay(W[:-len(tail)])
    for g, j in tail:
        assert step(s, G, g, j)
        print('  letter', (g, j), '; branch of the cap', B[g], '; K after it', hb(s.pose(K[0])), flush=True)
    s, G = replay(W)
    Q = s.pose(K[0]); assert all(s.pose(p) == Q for p in K)
    h = hb(Q)[0]
    print('height', h, 'branch', hb(Q)[1], '; tail', tail, '; letters', len(W), '; out grips', len(outs(G)),
          '; off-lattice', s.off_lattice_count(), '; digest', s.digest(), flush=True)
print('word', W)
r = W[-1][0]; Q = s.pose(K[0]); assert hb(Q)[0] == 14
print('letters at', r, 'before the raise:', sum(g == r for g, _ in W[:-1]))
rows = [('none', sorted(set(holding(Q)) - {r}))]; drops = []
for j in L:
    u = clone(s); Gu = list(G); assert step(u, Gu, r, j)
    Qu = u.pose(K[0]); assert all(u.pose(p) == Qu for p in K)
    if isin(Gu[r]):
        assert hb(Qu)[0] == 12; drops.append(j); continue
    assert hb(Qu)[0] == 14
    rows.append((j, sorted(set(holding(Qu)) - {r})))
print('letters at', r, 'that return it (K drops to 12):', drops)
print('orientations of K at height 14 (letter at', r, ', caps other than', r, 'that hold K whole):', rows)
print('all these caps in NB of', r, ':', all(set(c) <= NB[r] for _, c in rows), '; number of orientations', len(rows))
print('seconds', round(time.time() - t0))
```

Output:

```text
start: K (12, 4) ; letters 149 ; digest 7bb0df3ebb0e55a89135921955ae592e24249da89c16e429be0c18487f6a9aed
  letter (576, 1) ; branch of the cap 4 ; K after it (12, 4)
  letter (573, 1) ; branch of the cap 4 ; K after it (12, 4)
  letter (526, 12) ; branch of the cap 0 ; K after it (14, 0)
height 14 branch 0 ; tail [(576, 1), (573, 1), (526, 12)] ; letters 152 ; out grips 15 ; off-lattice 38192 ; digest ea0bc4f31df0c88e4019d2cf505e05d426122721bcca399c5801f3fdc1cedb72
word [(521, 35), (557, 40), (501, 29), (353, 37), (501, 51), (218, 32), (217, 24), (557, 23), (593, 48), (218, 47), (501, 13), (501, 15), (521, 27), (537, 33), (537, 25), (218, 40), (501, 27), (218, 58), (324, 59), (218, 34), (324, 54), (484, 26), (218, 18), (537, 43), (537, 25), (484, 53), (484, 41), (537, 16), (521, 17), (484, 46), (501, 50), (537, 28), (442, 20), (557, 13), (564, 58), (501, 32), (501, 57), (218, 34), (484, 18), (537, 31), (537, 43), (360, 23), (360, 36), (501, 59), (360, 51), (218, 34), (501, 51), (521, 57), (501, 25), (218, 39), (442, 52), (442, 24), (537, 59), (521, 39), (218, 36), (217, 23), (217, 19), (564, 51), (218, 50), (564, 49), (218, 40), (442, 55), (110, 20), (501, 59), (521, 52), (521, 33), (564, 40), (501, 21), (512, 38), (537, 53), (484, 53), (218, 49), (217, 21), (217, 38), (521, 38), (512, 40), (564, 12), (217, 12), (537, 43), (484, 43), (537, 32), (217, 37), (525, 16), (217, 49), (525, 15), (110, 51), (324, 29), (484, 17), (217, 24), (217, 43), (521, 48), (537, 23), (110, 24), (217, 26), (521, 38), (521, 44), (564, 34), (501, 58), (501, 35), (110, 43), (521, 42), (218, 31), (501, 59), (537, 54), (564, 15), (324, 52), (110, 16), (537, 50), (284, 24), (110, 58), (218, 44), (564, 41), (110, 24), (501, 23), (110, 54), (218, 59), (501, 26), (110, 51), (564, 39), (537, 51), (280, 49), (501, 20), (564, 55), (537, 53), (501, 49), (280, 25), (501, 50), (564, 44), (501, 35), (537, 22), (501, 45), (501, 39), (537, 38), (217, 19), (537, 29), (218, 37), (218, 28), (525, 47), (537, 34), (537, 12), (593, 23), (593, 36), (593, 58), (501, 22), (593, 12), (593, 15), (582, 12), (582, 18), (576, 12), (576, 1), (573, 1), (526, 12)]
letters at 526 before the raise: 0
letters at 526 that return it (K drops to 12): [14, 21, 23, 43, 44, 45, 46, 47, 48, 49, 50, 51]
orientations of K at height 14 (letter at 526 , caps other than 526 that hold K whole): [('none', [530, 563, 573, 591]), (1, [396, 401, 483, 486]), (2, [483, 497, 556, 563]), (3, [478, 486, 491, 530]), (4, [478, 486, 491, 530]), (5, [486, 503, 558, 563]), (6, [480, 483, 510, 530]), (7, [483, 497, 556, 563]), (8, [480, 483, 510, 530]), (9, [486, 503, 558, 563]), (10, [530, 563, 573, 591]), (11, [396, 401, 483, 486]), (12, [486, 530, 556, 558, 563, 591]), (13, [478, 483, 486, 530]), (15, [480, 483, 510, 530, 563, 573]), (16, [483, 486, 491, 510, 530, 573]), (17, [486, 503, 530, 563]), (18, [480, 483, 497, 530, 556, 563]), (19, [401, 483, 486, 563]), (20, [396, 480, 483, 486, 497, 563]), (22, [483, 530, 556, 563, 573, 591]), (24, [401, 478, 483, 486, 503, 563]), (25, [483, 530, 556, 558, 563, 591]), (26, [486, 491, 510, 530, 563, 573]), (27, [401, 478, 483, 486, 503, 530]), (28, [483, 486, 556, 558, 563, 591]), (29, [396, 480, 483, 497, 530, 563]), (30, [401, 478, 486, 503, 530, 563]), (31, [483, 491, 510, 530, 563, 573]), (32, [396, 480, 483, 486, 497, 530]), (33, [486, 503, 530, 558, 563, 591]), (34, [483, 530, 556, 563]), (35, [483, 486, 510, 530]), (36, [483, 486, 558, 563]), (37, [480, 483, 530, 563]), (38, [396, 483, 486, 530]), (39, [486, 530, 563, 591]), (40, [483, 530, 563, 573]), (41, [483, 486, 497, 563]), (42, [486, 491, 530, 563]), (52, [478, 486, 491, 503, 530, 563]), (53, [396, 480, 483, 486, 510, 530]), (54, [396, 401, 483, 486, 497, 563]), (55, [396, 401, 478, 483, 486, 530]), (56, [486, 491, 530, 563, 573, 591]), (57, [478, 483, 486, 491, 510, 530]), (58, [483, 486, 497, 556, 558, 563]), (59, [401, 483, 486, 503, 558, 563])]
all these caps in NB of 526 : True ; number of orientations 48
seconds 86
```

## 23. Where TH holds, returns and the lens criterion (round 18)

Round 17 reached height 14 and met a raise where the hypothesis TH of the trap lemma holds (section 22). Round 18 asked three things: how to decide TH and what it depends on (Q1); whether a relay can avoid the raises where TH holds, or whether something bounds the relay (Q2); and what each result gives (Q3). The review returned four results (T18-01 to T18-04, all `minor`). The integrator re-ran the three programs from the repository root, got the reported outputs exactly, checked the proofs of T18-01, T18-03 and T18-04, and adopted all four. The integrator's own experiments and two lemmas of its own (below) explain why the relay of E17-7 stops at 14: each of the 12 returns of the raiser 526 is a retained letter at 526 applied before the raise, and TH holds at 526 because the seven pieces lie deep in its cap.

| Finding | Status | Result |
| --- | --- | --- |
| T18-01 | proved | TH at (c, Q) is decided by four orientations. The 48 jumbles at c split into the four cosets A4c·A(c,r), r = 12, 13, 14, 15. A retained element h fixes n_c, permutes the caps and keeps NB(c), and cap d holds h·A·Q(K) whole exactly when cap h⁻¹(d) holds A·Q(K) whole. So TH holds if and only if, for each of the four representatives A(c,r), every cap other than c that holds A(c,r)·Q(K) whole is in NB(c): 4 × 583 exact cap tests. TH is a finite Boolean combination of linear inequalities in Q; height does not enter it explicitly. |
| T18-02 | computed | TH along the word of 152 letters. Over the poses after letters 141 to 152 and every cap of another branch that holds K whole, 55 pairs (2,640 orientations, 1,584,000 exact cap decisions), TH holds at 29: at heights 4, 6, 8, 10, 12 and 14 in 9 of 12, 2 of 6, 3 of 10, 4 of 11, 9 of 12 and 2 of 4 pairs. Height does not decide TH: after letters 145 and 146 K is at height 8 both times, and TH fails at cap 555 after 145 (item 12 puts K whole in cap 542, outside NB(555)) and holds after 146; cap 580 behaves the same way. At (144, 555) TH fails only through caps of the raiser's own branch. |
| T18-03 | computed; proved | TH failing is not enough. At X12 (after (576, 12)) the raiser 576 is out and TH fails there. Over its 48 orientations, 60 pairs of an orientation and a cap outside NB(576) hold K whole: 48 in its own branch (24 admissible, 24 blocked) and 12 in other branches, all blocked, each straddled by a height-2 piece of model orbit 3, 5 or 9. Any sequence of letters at 576 that keeps it out acts as one of these 48 elements, caps in NB(576) stay blocked, and a raise needs a cap of another branch (Corollary FN1), so no sequence of letters at 576 alone leads to a raise. The letters (576, 1), (573, 1), (526, 12) do: the letter at 573 is what leaves the raiser's own family. |
| T18-04 | proved; computed | At most 120 grips are out at once. With A the adjacency of the 4 nearest grips (N15), the NB adjacency is A² + A − 4I, so the closed N15 neighbourhoods of two grips meet only if the grips are NB neighbours. Out grips are pairwise not NB neighbours (INV), so their closed neighbourhoods are disjoint sets of 5 grips: \|O\| ≤ 120. Each branch is an NB-independent set of 120 grips, so the bound is sharp for NB independence (not shown to be reachable). Consequence: along a continuation in which every rise of a piece happens at a grip that was in just before, with D departures and R returns, h_t ≤ h_0 + 2D_t ≤ h_0 + 2(120 − \|O_0\| + R_t); from X10 (h_0 = 10, \|O_0\| = 13) without returns this gives at most 224. So a relay without a height bound needs infinitely many returns or infinitely many raises at grips that are already out (as 593 raised K to 8). |

### Two integrator lemmas (not yet reviewed)

The 60 menu elements at a grip c form a group Λc; at every grip the 12 retained items are exactly the elements in K⁺, and every element fixes n_c (checked at all 600 grips in E18-1). Cap c is the half-space x·n_c ≥ κ, all normals have the same norm |n|, and a piece lies whole in a cap when all its vertices do.

*Excursion identity.* Let c be in at a state X, and let w be a word of letters at c only, admissible from X, after which c is in again. Then w has the same effect as the single letter at c whose element is the product g of the elements of w, and g is retained. Proof: an admissible letter at c moves exactly the pieces that lie whole in cap c, and each element of Λc fixes n_c, so it maps cap c onto itself. Hence after each letter the same pieces lie whole in cap c and none straddles its cut: every letter of w moves the same set of pieces, which is moved by g in total, and the letter g is admissible at X. Since G[c] is in K⁺ before and after, g = G_after·G_before⁻¹ lies in Λc ∩ K⁺, the retained items. So a fresh raise at which TH holds, followed by its return and no letter at another grip, changes the state exactly as one retained letter at the raiser before the raise (E18-1 checks this for the 12 returns of 526 at X14).

*Lens criterion (LC).* Let x be a vertex of K at pose Q, with norm ρ and depth δ = (x·n_c − κ)/|n| in cap c. Write κ' = κ/|n|, φ = arccos(κ'/ρ), and θ3 = 36°, the least angle between n_c and the normal of a cap outside NB(c) ∪ {c} (at every grip; the NB angles are 15.52° and 25.24°). If δ > ρ cos(max(0, θ3 − φ)) − κ' for some vertex of K, then TH holds at (c, Q). Proof: a letter at c is a rotation that fixes n_c, so it keeps ρ and δ of every vertex. If the image y of x lies in cap d at angle θ ≥ θ3 from c, then y·n_d ≥ κ gives ∠(y, n_d) ≤ φ, so ∠(y, n_c) ≥ θ − φ and δ = ρ cos ∠(y, n_c) − κ' ≤ ρ cos(max(0, θ − φ)) − κ' ≤ ρ cos(max(0, θ3 − φ)) − κ'. So no orientation that letters at c give puts K whole in a cap outside NB(c) ∪ {c}.

Corollary (rim raises). If K lies whole in the cap of an out grip d and is raised at a free cap e other than d, then e is not in NB(d), so n_d is at angle at least 36° from n_e. Every vertex of K lies in cap d, so by the inequality above, with e in the place of c, no vertex satisfies the condition of LC at e. A raise made while K is still inside the cap of another out grip is never certified as a trap by LC.

### Integrator experiments (during the round)

- E18-1, the returns of 526. Let X12 be the state before the last letter (526, 12) of the word of 152 letters; no earlier letter is at 526. At X12, K is at height 12 in branch 4 and four caps of another branch hold K whole: 526, which is free, and 530, 563 and 591, which are blocked. Their exact straddlers are 26, 3 and 6 pieces of height 2, all whole in cap 526 and all last moved by the letter (573, 1). At X14 each of the 12 letters at 526 that return it gives the same state (digest) and the same product of letters at every grip as one retained letter at 526 applied at X12 (the excursion identity); the inverse of item 12 gives X12 itself. K is back at height 12, three times in each of the branches 1 to 4, and in each case the caps of another branch that hold K whole are 526, which is free, and three caps blocked by the retained images of those straddlers (one of them, 396, is also in NB of the out grip 280).
- E18-2, TH, depth and LC. Over 435 pairs (the three fresh raises of the relay, and every cap of another branch that holds K whole in each of the 48 orientations after the raises at 582 and 576), TH holds at 229 and fails at 206. The depth of K in the cap ranges from 0.110 to 0.306 where TH holds and from 0.001 to 0.198 where it fails, so depth alone does not decide TH. LC holds at 217 pairs, all of them pairs where TH holds; the 12 other pairs where TH holds all have depth 0.11018 (after the raise at 576, caps 531, 569, 583 and 590). The floating-point margins are at least 0.0024 on either side of the LC threshold. Of the relay raises, LC and TH fail at 582 (depth 0.072) and 576 (0.008) and both hold at 526 (0.268).
- E18-3, clearing a raiser and a second trap. After the return (526, 23), cap 486 holds K whole and is blocked by 26 pieces of height 2. Exactly one cap, 386, holds all of them whole, holds no piece of K and is free; 42 of its 59 letters (the 11 retained ones among them) leave K in place and make cap 486 free. After (386, 1), the jumble (486, 12) takes K to height 14 again, in branch 4. LC holds at 486 (depth 0.198), so TH holds there, and in none of the 48 orientations does a cap outside NB(486) hold K whole.
- E18-4, a bounded relay search from X14. Moves between raises are turns at an out grip that holds K after which K lies whole in a cap outside its NB, returns of such a grip, letters at free caps of K's branch that hold K whole, and raises below the height of X14 after which K lies whole in a cap outside the raiser's NB; a candidate raiser may first be cleared by at most two letters. With up to three such moves the search visits 1,799 states, below its budget of 3,000, and finds no raise to height 16.

### What it gives

- TH is decided by four orientations (T18-01), and height does not decide it (T18-02). LC gives a geometric reason for most traps: TH holds when some vertex of K lies deeper in the raiser's cap than any point of its norm can lie while it is also in a cap at 36° or more from the raiser. In the census LC accounts for 217 of the 229 pairs where TH holds (E18-2).
- Why the relay of E17-7 stops at 14. The letter (573, 1) moved K so that it no longer lies whole in cap 576 but lies deep in cap 526, and it was the last letter to move each piece that straddles the three other caps that hold K (26, 3 and 6 pieces of height 2). The raise at 526 is then a trap (LC holds), and by the excursion identity its 12 returns are just the 12 retained letters at 526 at X12, after which the candidate caps are blocked by the images of the same pieces (E18-1). Clearing one of them works (a letter at 386 frees cap 486), but the raise there is again a trap by LC (E18-3). A bounded search with clearing finds nothing within three moves (E18-4).
- A trap followed only by its return gains nothing: by the excursion identity it is one retained letter at the raiser. So after a fresh raise at c where TH holds, any later rise needs letters at other grips while c is out (outside NB(c), and by TH none of them moves K), or must start from the state after the return.
- Rim raises. A raise made while K still lies whole in the cap of another out grip is never certified as a trap by LC (corollary). The raises at 582 and 576 had K near the rim of the raiser's cap (depths 0.072 and 0.008), and TH failed there; at 526, LC holds, so K no longer lay whole in the cap of any out grip.
- T18-03 and T18-04 close two simple relay schemas: turns at the raiser alone do not always lead to a raise, and fresh raises without returns are bounded, because at most 120 grips are out at once (from X10, at most height 224).
- X1 and the uniform bound of M1 stay open: the relay stopped at a trap that the integrator's lemmas explain, which is evidence, not a proof. Proposition HB is unchanged: K belongs to model orbits 19 to 24.
- Next (round 19): check the two integrator lemmas; extend the excursion identity to excursions during which letters at other grips are applied; decide whether a trap can contribute to a later rise; and decide whether a relay of rim raises, which returns its old raisers once K has left their caps, can go on without bound, or prove a bound on left heights.

### Reproduction (round 18)

From the repository root, with Python 3.13 and NumPy 2.5: `python3 -I -B -` with each program on standard input. T18-01 is a proof; its partition of the 48 jumbles into four cosets is checked in T18-02. T18-02, T18-03 and the E18 programs read the word of 147 letters from the program "E17-4, checked letter by letter" of section 22 and check the SHA-256 of that program. The review's versions of T18-02 and T18-03 checked the digest of this whole file, which this section changes; the versions below check the digest of that program instead and print the same output. On the integrator's machine T18-02 to T18-04 and E18-1 to E18-4 took 27, 27, 7, 39, 41, 33 and 616 seconds.

T18-02: TH at every pair along the word of 152 letters, and the partition into four cosets.

```python
import ast, sys, hashlib
from pathlib import Path
from collections import Counter, defaultdict
sys.path[:0]=['research/jumbling/theory','research/jumbling',
             'research/jumbling/fixtures']
import sim, restore as R, trees as T
from exact import matmul, matvec, dot
from sim.kplus import to_tuple
from sim.kernel import Evaluator,pullback_form,rows_form

C=sim.get_context();d=C.data;M=R.menu('I_a');I=C.kplus.matrix(0)
K=(118527,126380,126382,126384,126385,149644,149650)
V=sorted({tuple(v) for p in K for v in C.regions.vertices(p)},key=repr)
B=[T.branch(tuple(n)) for n in d.N]
J=set(R.jumble_items(M));H=set(range(60))-J
E0=[R.element(M,0,j) for j in range(60)]
index={A:j for j,A in enumerate(E0)}
reps=[];coset={};remaining=set(J)
while remaining:
 r=min(remaining)
 cs={index[to_tuple(matmul(E0[h],E0[r]))] for h in H}
 assert len(cs)==12 and cs<=remaining
 reps.append(r);coset[r]=cs;remaining-=cs
assert reps==[12,13,14,15]

EV=[Evaluator(pullback_form(I,n)) for n in d.N]
def holding(Q):
 vf=rows_form([tuple(matvec(Q,v)) for v in V])
 return {c for c,ev in enumerate(EV)
         if all(ev.sign_row(vf[0],r)>=0 for r in vf[1])}
def hb(Q):
 l=T.factors(Q)[0]
 return T.dist(l),T.branch(l)
NB={}
def nb(c):
 if c not in NB:
  NB[c]={a for a in range(600) if a!=c and
         (d.NN*(d.NN+dot(d.N[c],d.N[a]))
          -2*d.KAPPA*d.KAPPA).sign()>0}
  assert len(NB[c])==16
 return NB[c]

path=Path('research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md')
code=path.read_text().split(
 'E17-4, checked letter by letter: the word of 147 letters from the solved state.'
 )[1].split('```python\n')[1].split('```')[0]
assert hashlib.sha256(code.encode()).hexdigest()==(
 '897b98b4306a832f2f07e4cf570316bbe15c8cdf41dbd9bd3c408f80238ad911')
W=next(ast.literal_eval(n.value) for n in ast.parse(code).body
       if isinstance(n,ast.Assign)
       and any(isinstance(t,ast.Name) and t.id=='W' for t in n.targets))
W += [(582,18),(576,12),(576,1),(573,1),(526,12)]

Q=I;summary=defaultdict(Counter);result={};poses={}
tested=0;onlysame=[]
for n,(g,j) in enumerate(W,1):
 vf=rows_form([tuple(matvec(Q,v)) for v in V])
 side=EV[g].classify(vf)[0]
 assert side in ('in','out')
 if side=='in':
  Q=to_tuple(matmul(R.element(M,g,j),Q))
 if n<141:
  continue
 poses[n]=Q;h,b=hb(Q);yes=[];no=[]
 for c in sorted(holding(Q)):
  if B[c]==b:
   continue
  bad={};foreign={}
  for j in sorted(J):
   Qj=to_tuple(matmul(R.element(M,c,j),Q))
   assert hb(Qj)==(h+2,B[c])
   S=holding(Qj)
   assert c in S
   bad[j]=S-{c}-nb(c)
   foreign[j]={x for x in bad[j] if B[x]!=B[c]}
   tested+=600
  for r in reps:
   assert all(len(bad[j])==len(bad[r])
              and len(foreign[j])==len(foreign[r])
              for j in coset[r])
  th=not any(bad.values())
  summary[h]['TH' if th else 'notTH']+=1
  (yes if th else no).append(c)
  result[n,c]=(th,sum(bool(x) for x in bad.values()),
               sum(map(len,bad.values())),
               sum(map(len,foreign.values())))
  if not th and not any(foreign.values()):
   onlysame.append((n,c))
 print(n,h,'TH',yes,'notTH',no,flush=True)

print('summary',[(h,v['TH'],v['TH']+v['notTH'])
                 for h,v in sorted(summary.items())])
print('all_orientations_checked',48*len(result),
      'all_caps_checked',tested)
print('only_same_branch_failure',onlysame)
print('same_height_same_raiser',
      [(n,c,result[n,c]) for n in [145,146] for c in [555,580]])
print('raiser576_before_raise',result[148,576])
print('raiser526_before_raise',result[151,526])
assert len(result)==55 and sum(v['TH'] for v in summary.values())==29
print('PASS')
```

Output:

```text
141 4 TH [594, 598, 599] notTH [592]
142 4 TH [551, 580, 582] notTH [560]
143 4 TH [551, 555, 580] notTH [501]
144 6 TH [551, 580] notTH [496, 548, 555, 593]
145 8 TH [585] notTH [529, 547, 555, 562, 580]
146 8 TH [555, 580] notTH [551, 582]
147 10 TH [551, 580] notTH [542, 548, 560, 593]
148 10 TH [590, 596] notTH [560, 565, 576]
149 12 TH [565, 586, 590] notTH [540]
150 12 TH [566, 569, 575] notTH [567]
151 12 TH [526, 530, 563] notTH [591]
152 14 TH [530, 563] notTH [573, 591]
summary [(4, 9, 12), (6, 2, 6), (8, 3, 10), (10, 4, 11), (12, 9, 12), (14, 2, 4)]
all_orientations_checked 2640 all_caps_checked 1584000
only_same_branch_failure [(144, 555)]
same_height_same_raiser [(145, 555, (False, 48, 84, 36)), (145, 580, (False, 48, 96, 48)), (146, 555, (True, 0, 0, 0)), (146, 580, (True, 0, 0, 0))]
raiser576_before_raise (False, 48, 60, 12)
raiser526_before_raise (True, 0, 0, 0)
PASS
```

T18-03: the 48 orientations at the raiser 576 at X12, and the letters that lead on.

```python
import ast,hashlib,sys
from pathlib import Path
from collections import Counter
sys.path[:0]=['research/jumbling/theory','research/jumbling','research/jumbling/fixtures']
import sim,wj,restore as R,trees as T
from exact import matmul,matvec,dot
from sim.kplus import to_tuple
from sim.kernel import Evaluator,pullback_form,rows_form
p=Path('research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md')
code=p.read_text().split('E17-4, checked letter by letter: the word of 147 letters from the solved state.')[1].split('```python\n')[1].split('```')[0]
assert hashlib.sha256(code.encode()).hexdigest()=='897b98b4306a832f2f07e4cf570316bbe15c8cdf41dbd9bd3c408f80238ad911'
W=next(ast.literal_eval(n.value) for n in ast.parse(code).body if isinstance(n,ast.Assign) and any(isinstance(t,ast.Name) and t.id=='W' for t in n.targets))
C=sim.get_context();d=C.data;kp=C.kplus;_,M=wj.load(C,'I_a');I=kp.matrix(0)
K=(118527,126380,126382,126384,126385,149644,149650)
E=lambda c,j:R.element(M,c,j)
isin=lambda q:kp.index_of_matrix(to_tuple(q))>=0
hb=lambda q:(T.dist(T.factors(q)[0]),T.branch(T.factors(q)[0]))
B=[T.branch(tuple(n)) for n in d.N]
V=sorted({tuple(v) for q in K for v in C.regions.vertices(q)},key=repr)
EV=[Evaluator(pullback_form(I,n)) for n in d.N]
def nb(a):
 return {b for b in range(600) if b!=a and (d.NN*(d.NN+dot(d.N[a],d.N[b]))-2*d.KAPPA*d.KAPPA).sign()>0}
def holding(q):
 vf=rows_form([tuple(matvec(q,v)) for v in V])
 return [c for c in range(600) if EV[c].classify(vf)[0]=='in']
def step(s,G,c,j):
 o=s.apply(sim.Twist(C,c,E(c,j),'menu',{'menu':M.name}),journal=False)
 assert o.applied and not o.uncertain
 G[c]=to_tuple(matmul(E(c,j),G[c]))
def clone(s):
 u=sim.State(C,filtered=False,menu=M);u._poses=list(s._poses);u._lookup=dict(s._lookup);u.pose_id=s.pose_id.copy();return u
s=sim.State(C,filtered=False,menu=M);G=[I]*600
for c,j in W+[(582,18),(576,12)]:step(s,G,c,j)
assert s.digest()=='7bb0df3ebb0e55a89135921955ae592e24249da89c16e429be0c18487f6a9aed'
assert hb(s.pose(K[0]))==(12,4)
O=[c for c in range(600) if not isin(G[c])];NB={c:nb(c) for c in O}
assert 576 in O
orient=set();counts=Counter();foreign=[]
for j in range(60):
 if isin(matmul(E(576,j),G[576])):continue
 u=clone(s);Gu=list(G);step(u,Gu,576,j);q=u.pose(K[0])
 assert all(u.pose(p)==q for p in K) and hb(q)==(12,4)
 orient.add(to_tuple(q))
 for c in holding(q):
  if c==576 or c in NB[576]:continue
  cl=u.classify(c,need_inside=False);assert not cl.uncertain
  typ='same' if B[c]==4 else 'foreign';counts[(typ,cl.status)]+=1
  if typ=='foreign':
   assert cl.status=='blocked'
   p=cl.straddle['piece'];ev=Evaluator(pullback_form(u.pose(p),d.N[c]))
   assert ev.classify(C.regions.vform(p))[0]=='straddle'
   foreign.append((j,c,[x for x in O if c in NB[x]],p,int(d.orbit_id[p]),hb(u.pose(p))[0]))
assert len(orient)==48
print('orientations',len(orient),'cap_tests',48*600)
print('outside_NB',sorted(counts.items()))
print('foreign',foreign)
for c,j in [(576,1),(573,1),(526,12)]:step(s,G,c,j)
assert all(s.pose(p)==s.pose(K[0]) for p in K)
print('two_preparatory_letters',hb(s.pose(K[0])),s.digest())
```

Output:

```text
orientations 48 cap_tests 28800
outside_NB [(('foreign', 'blocked'), 12), (('same', 'admissible'), 24), (('same', 'blocked'), 24)]
foreign [(0, 540, [537], 112551, 9, 2), (1, 567, [], 110889, 3, 2), (2, 594, [], 110889, 3, 2), (3, 544, [525], 127693, 5, 2), (4, 533, [], 112551, 9, 2), (5, 582, [], 112551, 9, 2), (6, 597, [], 110889, 3, 2), (7, 447, [525], 127693, 5, 2), (8, 459, [], 112551, 9, 2), (9, 464, [], 112551, 9, 2), (10, 535, [525], 149399, 3, 2), (11, 498, [], 112551, 9, 2)]
two_preparatory_letters (14, 0) ea0bc4f31df0c88e4019d2cf505e05d426122721bcca399c5801f3fdc1cedb72
```

T18-04: the NB graph and at most 120 out grips.

```python
import sys,hashlib
from collections import Counter
import numpy as np
sys.path[:0]=['research/jumbling','research/jumbling/theory']
from sim.model import ModelData
from exact import dot
import trees as T
D=ModelData();N=D.N
v=max(q for q in (dot(N[0],n) for n in N) if q<D.NN)
A=np.zeros((600,600),dtype=np.int64);B=A.copy()
for i in range(600):
 for j in range(i):
  z=dot(N[i],N[j]);A[i,j]=A[j,i]=int(z==v)
  B[i,j]=B[j,i]=int((D.NN*(D.NN+z)-2*D.KAPPA*D.KAPPA).sign()>0)
assert set(A.sum(axis=1))=={4} and set(B.sum(axis=1))=={16}
assert np.array_equal(B,A@A+A-4*np.eye(600,dtype=np.int64))
b=[T.branch(tuple(n)) for n in N]
assert Counter(b)=={0:120,1:120,2:120,3:120,4:120}
assert not any(B[i,j] and b[i]==b[j] for i in range(600) for j in range(i))
print('degrees',4,16,'out_bound',120,'branch_sizes',dict(sorted(Counter(b).items())))
for name,G in [('N15',A),('NB',B)]:
 print(name,hashlib.sha256(G.astype(np.uint8).tobytes()).hexdigest())
```

Output:

```text
degrees 4 16 out_bound 120 branch_sizes {0: 120, 1: 120, 2: 120, 3: 120, 4: 120}
N15 1294e67f2a79abbb1923b5614dfa2735ea6fe7d91d2b89ce4f0d53ca91aeda08
NB 0ad18304958e076b7c371e1cc939c24ff7e0d05fc8ef8d00a172aa3ddc8797a0
```

E18-1: the 12 returns of 526 are retained letters at X12.

```python
# E18-1: the 12 returns of 526 at X14 are the 12 retained letters at 526 at X12. The word of E17-7 (152 letters) is
# replayed from solved by the exact simulator; X12 is the state before its last letter (526, 12), and 526 has no
# earlier letter. At X12: every cap of another branch that holds K whole, with the out grips whose NB contains it and
# its exact status, and for each blocked one its exact straddlers: heights, whether each lies whole in cap 526, and
# the grip of the letter that last moved it. Then for each letter (526, j) that returns 526 at X14: the retained
# letter k at 526 with E(526, k) = E(526, j) E(526, 12), equality of the two states, and the same cap table. First, at
# every grip: the menu elements in K+ are exactly the 12 retained items, and every menu element fixes the normal.
import sys, ast, hashlib, math
from pathlib import Path
from collections import Counter
import numpy as np
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling',
                'research/jumbling/fixtures']
import sim, wj, restore as RS, trees as T
from exact import matmul, matvec, dot
from sim.kplus import to_tuple
from sim.kernel import Evaluator, pullback_form, rows_form
C = sim.get_context(); d = C.data; kp = C.kplus; F, M = wj.load(C, 'I_a')
I = kp.matrix(0); RG = C.regions
K = (118527, 126380, 126382, 126384, 126385, 149644, 149650)
path = Path('research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md')
code = path.read_text().split(
    'E17-4, checked letter by letter: the word of 147 letters from the solved state.'
    )[1].split('```python\n')[1].split('```')[0]
assert hashlib.sha256(code.encode()).hexdigest() == '897b98b4306a832f2f07e4cf570316bbe15c8cdf41dbd9bd3c408f80238ad911'
W = next(ast.literal_eval(n.value) for n in ast.parse(code).body
         if isinstance(n, ast.Assign) and any(isinstance(t, ast.Name) and t.id == 'W' for t in n.targets))
W = W + [(582, 18), (576, 12), (576, 1), (573, 1), (526, 12)]
E = lambda g, j: RS.element(M, g, j)
isin = lambda q: kp.index_of_matrix(to_tuple(q)) >= 0
fl = lambda q: (q.a + q.b * math.sqrt(5)) / q.d
NF = np.array([[fl(x) for x in d.N[c]] for c in range(600)]); KAP = fl(d.KAPPA)
BR = [T.branch(tuple(n)) for n in d.N]
NB = [{b for b in range(600) if b != a and (d.NN * (d.NN + dot(d.N[a], d.N[b])) - 2 * d.KAPPA * d.KAPPA).sign() > 0}
      for a in range(600)]
V = sorted({tuple(v) for p in K for v in RG.vertices(p)}, key=repr)
VF = np.array([[fl(x) for x in v] for v in V])
def lb(Q):
    l = T.factors(Q)[0]; return T.dist(l), T.branch(l)
def holding(Q):
    # caps that hold K whole at pose Q; floating point only preselects, the test is exact
    h = (VF @ np.array([[fl(x) for x in r] for r in Q]).T) @ NF.T - KAP
    vf = rows_form([tuple(matvec(Q, v)) for v in V])
    return [c for c in range(600) if h[:, c].min() > -1e-6
            and Evaluator(pullback_form(I, d.N[c])).classify(vf)[0] == 'in']
def side(st, p, c): return Evaluator(pullback_form(st.pose(p), d.N[c])).classify(RG.vform(p))[0]
def hof(st, p):
    P = st._poses[int(st.pose_id[p])]
    return 0 if P.kidx >= 0 else T.dist(T.factors(P.matrix)[0])
def step(st, G, g, j):
    o = st.apply(sim.Twist(C, g, E(g, j), 'menu', {'menu': M.name}), journal=False)
    assert o.applied and not o.uncertain, (g, j)
    G[g] = to_tuple(matmul(E(g, j), G[g]))
def clone(st):
    u = sim.State(C, filtered=False, menu=M)
    u._poses = list(st._poses); u._lookup = dict(st._lookup); u.pose_id = st.pose_id.copy()
    return u
def kpose(st):
    Q = st.pose(K[0]); assert all(st.pose(p) == Q for p in K); return Q
PF = {}
def posef(st, pid):
    key = st._poses[pid].key
    if key not in PF: PF[key] = np.array([[fl(x) for x in r] for r in st._pose_matrix(pid)])
    return PF[key]
def straddlers(st, c):
    # exact straddlers of cut c among the off-lattice pieces (floating point preselects)
    kidx = st._kpose()[st.pose_id]; out = []
    for p in np.flatnonzero(kidx < 0).tolist():
        h = RG.frows(p)[0] @ (posef(st, int(st.pose_id[p])).T @ NF[c]) - KAP
        if h.min() < 1e-6 and h.max() > -1e-6 and side(st, p, c) == 'straddle': out.append(p)
    return out
def caps_table(st, G, Q, detail):
    O = {g for g in range(600) if not isin(G[g])}; rows = []
    for c in holding(Q):
        if BR[c] == lb(Q)[1]: continue
        status = st.classify(c, need_inside=False).status
        rows.append((c, BR[c], sorted(x for x in O if c in NB[x]), status))
        if detail and status != 'admissible':
            S = straddlers(st, c)
            print(f'  cap {c}: {len(S)} straddlers ; heights {sorted(Counter(hof(st, p) for p in S).items())} ; '
                  f'whole in cap 526: {sum(side(st, p, 526) == "in" for p in S)} ; last moved at grip '
                  f'{sorted(Counter(W[last[p]][0] for p in S).items())}')
    return rows
RET = sorted(set(range(60)) - set(RS.jumble_items(M)))
for c in range(600):
    assert [j for j in range(60) if isin(E(c, j))] == RET
    assert all(tuple(matvec(E(c, j), d.N[c])) == tuple(d.N[c]) for j in range(60))
print('at every grip the menu elements in K+ are the retained items', RET, 'and every menu element fixes n_c')
s = sim.State(C, filtered=False, menu=M); G = [I] * 600; last = {}
for i, (g, j) in enumerate(W[:-1]):
    before = s.pose_id.copy(); step(s, G, g, j)
    for p in np.flatnonzero(before != s.pose_id).tolist(): last[p] = i
assert not any(g == 526 for g, _ in W[:-1])
Q12 = kpose(s); print('X12', s.digest()[:16], 'K', lb(Q12))
print('caps of another branch holding K:', caps_table(s, G, Q12, True))
x14 = clone(s); G14 = list(G); step(x14, G14, 526, 12)
assert x14.digest() == 'ea0bc4f31df0c88e4019d2cf505e05d426122721bcca399c5801f3fdc1cedb72'
index = {to_tuple(E(526, j)): j for j in range(60)}
for j in range(60):
    if not isin(matmul(E(526, j), G14[526])): continue
    u = clone(x14); Gu = list(G14); step(u, Gu, 526, j)
    k = index[to_tuple(matmul(E(526, j), E(526, 12)))]
    assert isin(E(526, k))
    v = clone(s); Gv = list(G)
    if to_tuple(E(526, k)) != to_tuple(I): step(v, Gv, 526, k)
    assert u.digest() == v.digest() and all(to_tuple(a) == to_tuple(b) for a, b in zip(Gu, Gv))
    Q = kpose(u)
    print(f'return (526, {j}) = retained (526, {k}) at X12: K {lb(Q)} ; caps {caps_table(u, Gu, Q, False)}')
```

Output:

```text
at every grip the menu elements in K+ are the retained items [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11] and every menu element fixes n_c
X12 5561f12602667ceb K (12, 4)
  cap 530: 26 straddlers ; heights [(2, 26)] ; whole in cap 526: 26 ; last moved at grip [(573, 26)]
  cap 563: 3 straddlers ; heights [(2, 3)] ; whole in cap 526: 3 ; last moved at grip [(573, 3)]
  cap 591: 6 straddlers ; heights [(2, 6)] ; whole in cap 526: 6 ; last moved at grip [(573, 6)]
caps of another branch holding K: [(526, 0, [], 'admissible'), (530, 3, [], 'blocked'), (563, 1, [], 'blocked'), (591, 2, [], 'blocked')]
return (526, 14) = retained (526, 5) at X12: K (12, 2) ; caps [(486, 4, [], 'blocked'), (526, 0, [], 'admissible'), (558, 3, [], 'blocked'), (563, 1, [], 'blocked')]
return (526, 21) = retained (526, 3) at X12: K (12, 1) ; caps [(486, 4, [], 'blocked'), (491, 2, [], 'blocked'), (526, 0, [], 'admissible'), (530, 3, [], 'blocked')]
return (526, 23) = retained (526, 11) at X12: K (12, 3) ; caps [(396, 1, [280], 'blocked'), (483, 2, [], 'blocked'), (486, 4, [], 'blocked'), (526, 0, [], 'admissible')]
return (526, 43) = retained (526, 6) at X12: K (12, 4) ; caps [(483, 2, [], 'blocked'), (510, 1, [], 'blocked'), (526, 0, [], 'admissible'), (530, 3, [], 'blocked')]
return (526, 44) = retained (526, 9) at X12: K (12, 3) ; caps [(486, 4, [], 'blocked'), (503, 2, [], 'blocked'), (526, 0, [], 'admissible'), (563, 1, [], 'blocked')]
return (526, 45) = retained (526, 0) at X12: K (12, 4) ; caps [(526, 0, [], 'admissible'), (530, 3, [], 'blocked'), (563, 1, [], 'blocked'), (591, 2, [], 'blocked')]
return (526, 46) = retained (526, 10) at X12: K (12, 2) ; caps [(526, 0, [], 'admissible'), (530, 3, [], 'blocked'), (563, 1, [], 'blocked'), (573, 4, [], 'blocked')]
return (526, 47) = retained (526, 1) at X12: K (12, 1) ; caps [(401, 3, [], 'blocked'), (483, 2, [], 'blocked'), (486, 4, [], 'blocked'), (526, 0, [], 'admissible')]
return (526, 48) = retained (526, 2) at X12: K (12, 3) ; caps [(483, 2, [], 'blocked'), (526, 0, [], 'admissible'), (556, 4, [], 'blocked'), (563, 1, [], 'blocked')]
return (526, 49) = retained (526, 7) at X12: K (12, 4) ; caps [(483, 2, [], 'blocked'), (497, 3, [], 'blocked'), (526, 0, [], 'admissible'), (563, 1, [], 'blocked')]
return (526, 50) = retained (526, 4) at X12: K (12, 2) ; caps [(478, 1, [], 'blocked'), (486, 4, [], 'blocked'), (526, 0, [], 'admissible'), (530, 3, [], 'blocked')]
return (526, 51) = retained (526, 8) at X12: K (12, 1) ; caps [(480, 4, [], 'blocked'), (483, 2, [], 'blocked'), (526, 0, [], 'admissible'), (530, 3, [], 'blocked')]
```

E18-2: TH, depth and the lens criterion.

```python
# E18-2: TH, depth and the lens criterion. Pairs (c, Q) with K whole in cap c at pose Q: the three fresh raises of the
# relay (pose before the raise), and every cap of another branch that holds K whole in each of the 48 orientations of K
# after the raises at 582 and 576 (pose A Q0, A a jumble at the raiser). For each pair: TH exactly (all 48 jumbles at c,
# every cap holding K whole); the depth of K in cap c, min over K's vertices of (v.n_c - kappa)/|n_c|; and the lens
# criterion LC: some vertex v of K, of norm rho and depth delta in cap c, has delta > rho cos(max(0, theta3 - phi)) -
# kappa/|n|, phi = arccos(kappa/(|n| rho)), theta3 the least angle between n_c and a cap outside NB(c) and c. Depth
# and LC are evaluated in floating point; LC implies TH (section 23).
import sys, ast, hashlib, math
from pathlib import Path
from collections import Counter
import numpy as np
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling',
                'research/jumbling/fixtures']
import sim, wj, restore as RS, trees as T
from exact import matmul, matvec, dot
from sim.kplus import to_tuple
from sim.kernel import Evaluator, pullback_form, rows_form
C = sim.get_context(); d = C.data; kp = C.kplus; F, M = wj.load(C, 'I_a')
I = kp.matrix(0); RG = C.regions
K = (118527, 126380, 126382, 126384, 126385, 149644, 149650)
path = Path('research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md')
code = path.read_text().split(
    'E17-4, checked letter by letter: the word of 147 letters from the solved state.'
    )[1].split('```python\n')[1].split('```')[0]
assert hashlib.sha256(code.encode()).hexdigest() == '897b98b4306a832f2f07e4cf570316bbe15c8cdf41dbd9bd3c408f80238ad911'
W = next(ast.literal_eval(n.value) for n in ast.parse(code).body
         if isinstance(n, ast.Assign) and any(isinstance(t, ast.Name) and t.id == 'W' for t in n.targets))
W = W + [(582, 18), (576, 12), (576, 1), (573, 1), (526, 12)]
E = lambda g, j: RS.element(M, g, j)
isin = lambda q: kp.index_of_matrix(to_tuple(q)) >= 0
fl = lambda q: (q.a + q.b * math.sqrt(5)) / q.d
NF = np.array([[fl(x) for x in d.N[c]] for c in range(600)]); KAP = fl(d.KAPPA)
BR = [T.branch(tuple(n)) for n in d.N]
NB = [{b for b in range(600) if b != a and (d.NN * (d.NN + dot(d.N[a], d.N[b])) - 2 * d.KAPPA * d.KAPPA).sign() > 0}
      for a in range(600)]
V = sorted({tuple(v) for p in K for v in RG.vertices(p)}, key=repr)
VF = np.array([[fl(x) for x in v] for v in V])
def lb(Q):
    l = T.factors(Q)[0]; return T.dist(l), T.branch(l)
def holding(Q):
    # caps that hold K whole at pose Q; floating point only preselects, the test is exact
    h = (VF @ np.array([[fl(x) for x in r] for r in Q]).T) @ NF.T - KAP
    vf = rows_form([tuple(matvec(Q, v)) for v in V])
    return [c for c in range(600) if h[:, c].min() > -1e-6
            and Evaluator(pullback_form(I, d.N[c])).classify(vf)[0] == 'in']
def step(st, G, g, j):
    o = st.apply(sim.Twist(C, g, E(g, j), 'menu', {'menu': M.name}), journal=False)
    assert o.applied and not o.uncertain, (g, j)
    G[g] = to_tuple(matmul(E(g, j), G[g]))
U = NF / np.linalg.norm(NF, axis=1)[:, None]; kp1 = KAP / np.linalg.norm(NF[0])
ANG = np.degrees(np.arccos(np.clip(U @ U.T, -1, 1)))
far = sorted(set(range(600)) - NB[0] - {0})
TH3 = min(ANG[0, x] for x in far)
assert all(abs(min(ANG[c, x] for x in set(range(600)) - NB[c] - {c}) - TH3) < 1e-9 for c in range(600))
print('theta3', round(TH3, 6), 'NB angles', sorted({round(float(ANG[0, x]), 3) for x in NB[0]}))
def depth_lc(c, Q):
    X = VF @ np.array([[fl(x) for x in r] for r in Q]).T
    rho = np.linalg.norm(X, axis=1); dl = X @ U[c] - kp1
    phi = np.arccos(np.clip(kp1 / rho, -1, 1))
    gap = dl - (rho * np.cos(np.maximum(0, np.radians(TH3) - phi)) - kp1)
    return float(dl.min()), float(gap.max())
def th(c, Q):
    for j in range(60):
        A = E(c, j)
        if isin(A): continue
        if any(x != c and x not in NB[c] for x in holding(matmul(A, Q))): return False
    return True
s = sim.State(C, filtered=False, menu=M); G = [I] * 600; poses = []
for g, j in W:
    poses.append(s.pose(K[0])); step(s, G, g, j)
assert s.digest() == 'ea0bc4f31df0c88e4019d2cf505e05d426122721bcca399c5801f3fdc1cedb72'
pairs = [('raise', g, poses[i]) for i, (g, j) in enumerate(W) if g in (582, 576, 526) and j == 12]
for i, (g, j) in enumerate(W):
    if g not in (582, 576) or j != 12: continue
    for jj in range(60):
        A = E(g, jj)
        if isin(A): continue
        Qa = matmul(A, poses[i])
        pairs += [(f'{g}/{jj}', x, Qa) for x in holding(Qa) if x != g and BR[x] != lb(Qa)[1]]
rows = []
for tag, c, Q in pairs:
    dp, gap = depth_lc(c, Q); rows.append((tag, c, th(c, Q), gap > 0, dp, gap))
    assert not (gap > 0 and not rows[-1][2])
hold = [r for r in rows if r[2]]; fail = [r for r in rows if not r[2]]
print('pairs', len(rows), '; TH holds', len(hold), '; fails', len(fail))
print(f'depth where TH holds {min(r[4] for r in hold):.5f} .. {max(r[4] for r in hold):.5f} ; '
      f'where TH fails {min(r[4] for r in fail):.5f} .. {max(r[4] for r in fail):.5f}')
print('(TH, LC):', sorted(Counter((r[2], r[3]) for r in rows).items()))
print(f'LC margin: least positive {min(r[5] for r in rows if r[3]):.5f} ; largest non-positive '
      f'{max(r[5] for r in rows if not r[3]):.5f}')
print('TH without LC:', [(r[0], r[1], round(r[4], 5)) for r in rows if r[2] and not r[3]])
print('relay raises:', [(r[1], r[2], r[3], round(r[4], 5)) for r in rows if r[0] == 'raise'])
```

Output:

```text
theta3 36.0 NB angles [15.522, 25.243]
pairs 435 ; TH holds 229 ; fails 206
depth where TH holds 0.11018 .. 0.30564 ; where TH fails 0.00117 .. 0.19791
(TH, LC): [((False, False), 206), ((True, False), 12), ((True, True), 217)]
LC margin: least positive 0.00242 ; largest non-positive -0.00349
TH without LC: [('576/13', 583, 0.11018), ('576/17', 590, 0.11018), ('576/19', 531, 0.11018), ('576/34', 531, 0.11018), ('576/35', 531, 0.11018), ('576/36', 569, 0.11018), ('576/37', 583, 0.11018), ('576/38', 569, 0.11018), ('576/39', 583, 0.11018), ('576/40', 590, 0.11018), ('576/41', 590, 0.11018), ('576/42', 569, 0.11018)]
relay raises: [(582, False, False, 0.07211), (576, False, False, 0.00841), (526, True, True, 0.26847)]
```

E18-3: clearing cap 486, and a second raise to 14.

```python
# E18-3: clearing a raiser after a return, and a second raise to 14. From X14 the return (526, 23) (K at height 12 in
# branch 3). Cap 486 holds K whole and is blocked: its exact straddlers, and the caps that hold all of them whole, hold
# no piece of K, are admissible and are in NB of no out grip. For each letter at such a cap: whether it leaves K in
# place and cap 486 free. Then (386, 1) and the raise (486, 12): TH and the lens criterion at 486, and for each of the
# 48 jumbles A at 486 (by E18-1 these give every state reached by the raise and letters at 486 that keep it out),
# every cap of another branch than K's that holds K whole outside NB(486), with its exact status and the out grips
# whose NB contains it.
import sys, ast, hashlib, math
from pathlib import Path
from collections import Counter
import numpy as np
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling',
                'research/jumbling/fixtures']
import sim, wj, restore as RS, trees as T
from exact import matmul, matvec, dot
from sim.kplus import to_tuple
from sim.kernel import Evaluator, pullback_form, rows_form
C = sim.get_context(); d = C.data; kp = C.kplus; F, M = wj.load(C, 'I_a')
I = kp.matrix(0); RG = C.regions
K = (118527, 126380, 126382, 126384, 126385, 149644, 149650)
path = Path('research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md')
code = path.read_text().split(
    'E17-4, checked letter by letter: the word of 147 letters from the solved state.'
    )[1].split('```python\n')[1].split('```')[0]
assert hashlib.sha256(code.encode()).hexdigest() == '897b98b4306a832f2f07e4cf570316bbe15c8cdf41dbd9bd3c408f80238ad911'
W = next(ast.literal_eval(n.value) for n in ast.parse(code).body
         if isinstance(n, ast.Assign) and any(isinstance(t, ast.Name) and t.id == 'W' for t in n.targets))
W = W + [(582, 18), (576, 12), (576, 1), (573, 1), (526, 12)]
E = lambda g, j: RS.element(M, g, j)
isin = lambda q: kp.index_of_matrix(to_tuple(q)) >= 0
fl = lambda q: (q.a + q.b * math.sqrt(5)) / q.d
NF = np.array([[fl(x) for x in d.N[c]] for c in range(600)]); KAP = fl(d.KAPPA)
BR = [T.branch(tuple(n)) for n in d.N]
NB = [{b for b in range(600) if b != a and (d.NN * (d.NN + dot(d.N[a], d.N[b])) - 2 * d.KAPPA * d.KAPPA).sign() > 0}
      for a in range(600)]
V = sorted({tuple(v) for p in K for v in RG.vertices(p)}, key=repr)
VF = np.array([[fl(x) for x in v] for v in V])
def lb(Q):
    l = T.factors(Q)[0]; return T.dist(l), T.branch(l)
def holding(Q):
    # caps that hold K whole at pose Q; floating point only preselects, the test is exact
    h = (VF @ np.array([[fl(x) for x in r] for r in Q]).T) @ NF.T - KAP
    vf = rows_form([tuple(matvec(Q, v)) for v in V])
    return [c for c in range(600) if h[:, c].min() > -1e-6
            and Evaluator(pullback_form(I, d.N[c])).classify(vf)[0] == 'in']
def side(st, p, c): return Evaluator(pullback_form(st.pose(p), d.N[c])).classify(RG.vform(p))[0]
def hof(st, p):
    P = st._poses[int(st.pose_id[p])]
    return 0 if P.kidx >= 0 else T.dist(T.factors(P.matrix)[0])
def step(st, G, g, j):
    o = st.apply(sim.Twist(C, g, E(g, j), 'menu', {'menu': M.name}), journal=False)
    assert o.applied and not o.uncertain, (g, j)
    G[g] = to_tuple(matmul(E(g, j), G[g]))
def clone(st):
    u = sim.State(C, filtered=False, menu=M)
    u._poses = list(st._poses); u._lookup = dict(st._lookup); u.pose_id = st.pose_id.copy()
    return u
def kpose(st):
    Q = st.pose(K[0]); assert all(st.pose(p) == Q for p in K); return Q
PF = {}
def posef(st, pid):
    key = st._poses[pid].key
    if key not in PF: PF[key] = np.array([[fl(x) for x in r] for r in st._pose_matrix(pid)])
    return PF[key]
def straddlers(st, c):
    # exact straddlers of cut c among the off-lattice pieces (floating point preselects)
    kidx = st._kpose()[st.pose_id]; out = []
    for p in np.flatnonzero(kidx < 0).tolist():
        h = RG.frows(p)[0] @ (posef(st, int(st.pose_id[p])).T @ NF[c]) - KAP
        if h.min() < 1e-6 and h.max() > -1e-6 and side(st, p, c) == 'straddle': out.append(p)
    return out
U = NF / np.linalg.norm(NF, axis=1)[:, None]; kp1 = KAP / np.linalg.norm(NF[0])
TH3 = np.radians(36.0)
def depth_lc(c, Q):
    X = VF @ np.array([[fl(x) for x in r] for r in Q]).T
    rho = np.linalg.norm(X, axis=1); dl = X @ U[c] - kp1
    phi = np.arccos(np.clip(kp1 / rho, -1, 1))
    return float(dl.min()), bool((dl - (rho * np.cos(np.maximum(0, TH3 - phi)) - kp1)).max() > 0)
outset = lambda G: {g for g in range(600) if not isin(G[g])}
def free(st, G, c):
    return not any(c in NB[o] for o in outset(G) if o != c) and st.classify(c, need_inside=False).status == 'admissible'
s = sim.State(C, filtered=False, menu=M); G = [I] * 600
for g, j in W + [(526, 23)]: step(s, G, g, j)
Q = kpose(s); print('return (526, 23): K', lb(Q), s.digest()[:16])
S = straddlers(s, 486)
print('cap 486:', len(S), 'straddlers ; heights', sorted(Counter(hof(s, p) for p in S).items()))
held = None
for p in S:
    h = RG.frows(p)[0] @ (posef(s, int(s.pose_id[p])).T @ NF.T) - KAP
    hp = {y for y in np.flatnonzero(h.min(axis=0) > -1e-6).tolist() if side(s, p, y) == 'in'}
    held = hp if held is None else held & hp
Y = sorted(y for y in held - {486, 526} if all(side(s, p, y) == 'out' for p in K) and free(s, G, y))
print('caps that hold every straddler whole, no piece of K, and are free:', Y)
for y in Y:
    ok = []
    for j in range(60):
        if to_tuple(E(y, j)) == to_tuple(I): continue
        u = clone(s); Gu = list(G); step(u, Gu, y, j)
        if kpose(u) == Q and free(u, Gu, 486): ok.append(j)
    print(f'letters at {y} after which K is in place and cap 486 is free: {len(ok)} of 59 ; '
          f'retained {sum(isin(E(y, j)) for j in ok)} ; the others '
          f'{[j for j in range(60) if j not in ok and to_tuple(E(y, j)) != to_tuple(I)]}')
step(s, G, 386, 1); assert kpose(s) == Q and free(s, G, 486)
dp, lc = depth_lc(486, Q)
ths = all(x == 486 or x in NB[486] for j in range(60) if not isin(E(486, j)) for x in holding(matmul(E(486, j), Q)))
print(f'at 486: depth {dp:.5f} ; LC {lc} ; TH {ths}')
u = clone(s); Gu = list(G); step(u, Gu, 486, 12); Qr = kpose(u)
print('raise (486, 12): K', lb(Qr), 'out grips', len(outset(Gu)), u.digest()[:16])
tally = Counter(); freecaps = []
for j in range(60):
    if isin(E(486, j)): continue
    u = clone(s); Gu = list(G); step(u, Gu, 486, j); Qa = kpose(u); O = outset(Gu)
    for c in holding(Qa):
        if c == 486 or c in NB[486] or BR[c] == lb(Qa)[1]: continue
        nbo = sorted(o for o in O if c in NB[o] and o != c)
        st = u.classify(c, need_inside=False).status
        tally[('in NB of an out grip' if nbo else 'not in NB of an out grip', st)] += 1
        if not nbo and st == 'admissible': freecaps.append((j, c))
print('caps of another branch holding K outside NB(486), over the 48 orientations:', sorted(tally.items()))
print('free raisers:', freecaps)
```

Output:

```text
return (526, 23): K (12, 3) 47f256f2bf080f0c
cap 486: 26 straddlers ; heights [(2, 26)]
caps that hold every straddler whole, no piece of K, and are free: [386]
letters at 386 after which K is in place and cap 486 is free: 42 of 59 ; retained 11 ; the others [13, 15, 20, 25, 30, 32, 35, 36, 40, 42, 45, 50, 51, 52, 54, 58, 59]
at 486: depth 0.19825 ; LC True ; TH True
raise (486, 12): K (14, 4) out grips 15 6d6ebe5ef0fa4008
caps of another branch holding K outside NB(486), over the 48 orientations: []
free raisers: []
```

E18-4: a bounded relay search from X14.

```python
# E18-4: a bounded relay search from X14. Moves between raises, depth first, at most three: a turn at an out grip c
# that holds K (a letter that keeps c out) when K then lies whole in a cap outside NB(c); a return of such a grip
# (K drops by two), only at the height of X14; a letter at a free cap of K's branch that holds K whole; and, below the
# height of X14, a raise at a free cap of another branch after which K lies whole in a cap outside its NB. A candidate
# raiser that is not free may first be cleared by at most two letters: returns of out grips whose NB contains it and
# whose caps hold no piece of K, and letters at free caps that hold no piece of K and hold all its remaining
# straddlers whole. A final raise needs K whole in a free cap of another branch. Free: admissible and in NB of no
# other out grip. Every letter is applied by the exact simulator; floating point only orders and preselects moves.
import sys, ast, hashlib, math
from pathlib import Path
from collections import Counter
import numpy as np
sys.path[:0] = ['research/jumbling/theory', 'research/jumbling',
                'research/jumbling/fixtures']
import sim, wj, restore as RS, trees as T
from exact import matmul, matvec, dot
from sim.kplus import to_tuple
from sim.kernel import Evaluator, pullback_form, rows_form
C = sim.get_context(); d = C.data; kp = C.kplus; F, M = wj.load(C, 'I_a')
I = kp.matrix(0); RG = C.regions
K = (118527, 126380, 126382, 126384, 126385, 149644, 149650)
path = Path('research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md')
code = path.read_text().split(
    'E17-4, checked letter by letter: the word of 147 letters from the solved state.'
    )[1].split('```python\n')[1].split('```')[0]
assert hashlib.sha256(code.encode()).hexdigest() == '897b98b4306a832f2f07e4cf570316bbe15c8cdf41dbd9bd3c408f80238ad911'
W = next(ast.literal_eval(n.value) for n in ast.parse(code).body
         if isinstance(n, ast.Assign) and any(isinstance(t, ast.Name) and t.id == 'W' for t in n.targets))
W = W + [(582, 18), (576, 12), (576, 1), (573, 1), (526, 12)]
E = lambda g, j: RS.element(M, g, j)
isin = lambda q: kp.index_of_matrix(to_tuple(q)) >= 0
fl = lambda q: (q.a + q.b * math.sqrt(5)) / q.d
NF = np.array([[fl(x) for x in d.N[c]] for c in range(600)]); KAP = fl(d.KAPPA)
BR = [T.branch(tuple(n)) for n in d.N]
NB = [{b for b in range(600) if b != a and (d.NN * (d.NN + dot(d.N[a], d.N[b])) - 2 * d.KAPPA * d.KAPPA).sign() > 0}
      for a in range(600)]
V = sorted({tuple(v) for p in K for v in RG.vertices(p)}, key=repr)
VF = np.array([[fl(x) for x in v] for v in V])
IDENT = [j for j in range(60) if to_tuple(E(0, j)) == to_tuple(I)]
LETTERS = [j for j in range(60) if j != IDENT[0]]
EL = {}
def el(g, j):
    if (g, j) not in EL: EL[g, j] = E(g, j)
    return EL[g, j]
def lb(Q):
    l = T.factors(Q)[0]; return T.dist(l), T.branch(l)
def step(st, G, g, j):
    o = st.apply(sim.Twist(C, g, el(g, j), 'menu', {'menu': M.name}), journal=False)
    if not (o.applied and not o.uncertain): return False
    G[g] = to_tuple(matmul(el(g, j), G[g])); return True
def clone(st):
    u = sim.State(C, filtered=False, menu=M)
    u._poses = list(st._poses); u._lookup = dict(st._lookup); u.pose_id = st.pose_id.copy()
    return u
def outset(G): return {g for g in range(600) if not isin(G[g])}
EPS = 1e-7
PF = {}
def posef(st, pid):
    key = st._poses[pid].key
    if key not in PF: PF[key] = np.array([[fl(x) for x in r] for r in st._pose_matrix(pid)])
    return PF[key]
def k_caps(Q):
    h = (VF @ np.array([[fl(x) for x in r] for r in Q]).T) @ NF.T - KAP
    vf = rows_form([tuple(matvec(Q, v)) for v in V])
    hold = [g for g in range(600) if h[:, g].min() > -1e-6
            and Evaluator(pullback_form(I, d.N[g])).classify(vf)[0] == 'in']
    return hold, h.min(axis=0), h.max(axis=0)
def side(st, p, c): return Evaluator(pullback_form(st.pose(p), d.N[c])).classify(RG.vform(p))[0]
def kpose(st):
    Q = st.pose(K[0])
    return Q if all(st.pose(p) == Q for p in K) else None
def free(st, O, c):
    return not any(c in NB[o] for o in O if o != c) and st.classify(c, need_inside=False).status == 'admissible'
def straddlers(st, c):
    kidx = st._kpose()[st.pose_id]; out = []
    for p in np.flatnonzero(kidx < 0).tolist():
        h = RG.frows(p)[0] @ (posef(st, int(st.pose_id[p])).T @ NF[c]) - KAP
        if h.min() < -EPS and h.max() > EPS and side(st, p, c) == 'straddle': out.append(p)
    return out
def holders_of(st, p):
    h = (RG.frows(p)[0] @ posef(st, int(st.pose_id[p])).T) @ NF.T - KAP
    return set(np.flatnonzero(h.min(axis=0) >= -EPS).tolist())
def apply(st, G, g, j):
    u = clone(st); Gu = dict(G)
    return (u, Gu) if step(u, Gu, g, j) else None
def escapes_after(c, Qn):
    return [x for x in k_caps(Qn)[0] if x != c and x not in NB[c]]
def raise_with_escape(st, G, e, hh, need_escape):
    first = None
    for j in LETTERS:
        Qn = matmul(el(e, j), st.pose(K[0]))
        if lb(Qn)[0] != hh + 2: continue
        if need_escape and not escapes_after(e, Qn): continue
        r = apply(st, G, e, j)
        if r is None: continue
        Q = kpose(r[0])
        if Q is not None and lb(Q)[0] == hh + 2:
            if not need_escape or escapes_after(e, Q): return (e, j), r
            if first is None: first = ((e, j), r)
    return first
stats = {'nodes': 0, 'clear tries': 0}
def try_clear(st, G, Q, hmax, e, depth):
    O = outset(G)
    if free(st, O, e): return [], st, G
    if depth == 0: return None
    nbo = [o for o in O if o != e and e in NB[o]]
    if len(nbo) > depth: return None
    S = straddlers(st, e) if st.classify(e, need_inside=False).status != 'admissible' else []
    moves = []
    for o in nbo:
        if hmax[o] < -1e-9 and free(st, O, o):
            moves += [(o, j) for j in LETTERS if isin(matmul(el(o, j), G[o]))][:3]
    if S:
        held = {}
        for p in S:
            for y in holders_of(st, p): held.setdefault(y, set()).add(p)
        need = len(S) if (depth == 1 or nbo) else 1
        for y in sorted(held, key=lambda y: -len(held[y])):
            if len(held[y]) < need or y == e or hmax[y] >= -1e-9 or not free(st, O, y): continue
            keep = [j for j in LETTERS if isin(matmul(el(y, j), G[y])) == isin(G[y])][:2]
            other = [j for j in LETTERS if j not in keep and not (e in NB[y] and not isin(matmul(el(y, j), G[y])))][:2]
            moves += [(y, j) for j in keep + other]
    for g, j in moves:
        stats['clear tries'] += 1
        r = apply(st, G, g, j)
        if r is None: continue
        u, Gu = r
        if kpose(u) != Q: continue
        res = try_clear(u, Gu, Q, hmax, e, depth - 1)
        if res: return [(g, j)] + res[0], res[1], res[2]
    return None
def level(st, G, h, depth, path, seen):
    stats['nodes'] += 1
    if stats['nodes'] > BUDGET: return None
    Q = kpose(st); hh, b = lb(Q); hold, hmin, hmax = k_caps(Q); O = outset(G)
    cands = [e for e in hold if BR[e] != b]
    if hh + 2 > h:
        best = None
        for e in cands:
            res = try_clear(st, G, Q, hmax, e, DC)
            if not res: continue
            pre, u, Gu = res
            r = raise_with_escape(u, Gu, e, hh, True)
            if r and (best is None or (best[2] is False and escapes_after(e, kpose(r[1][0])))):
                best = (pre + [r[0]], r[1], bool(escapes_after(e, kpose(r[1][0]))))
                if best[2]: break
        if best: return path + best[0], best[1]
    if depth == 0: return None
    def children():
        if hh + 2 <= h:
            for e in cands:
                res = try_clear(st, G, Q, hmax, e, DC)
                if not res: continue
                pre, u, Gu = res
                r = raise_with_escape(u, Gu, e, hh, True)
                if r and escapes_after(e, kpose(r[1][0])): yield pre + [r[0]], r[1]
        for c in hold:
            if c not in O or not free(st, O, c): continue
            for j in LETTERS:
                Gc = matmul(el(c, j), G[c])
                if isin(Gc):
                    if hh >= h:
                        r = apply(st, G, c, j)
                        if r: yield [(c, j)], r
                elif escapes_after(c, matmul(el(c, j), Q)):
                    r = apply(st, G, c, j)
                    if r: yield [(c, j)], r
        for x in hold:
            if x in O or BR[x] != b or not free(st, O, x): continue
            for j in LETTERS:
                r = apply(st, G, x, j)
                if r: yield [(x, j)], r
    for letters, (u, Gu) in children():
        Qu = kpose(u)
        if Qu is None or lb(Qu)[0] < h - 2: continue
        key = u.digest()
        if key in seen: continue
        seen.add(key)
        res = level(u, Gu, h, depth - 1, path + letters, seen)
        if res: return res
        if stats['nodes'] > BUDGET: return None
    return None
DO, DC, BUDGET = 3, 2, 3000
s = sim.State(C, filtered=False, menu=M); G = {g: I for g in range(600)}
for g, j in W: assert step(s, G, g, j)
assert s.digest() == 'ea0bc4f31df0c88e4019d2cf505e05d426122721bcca399c5801f3fdc1cedb72'
Q = kpose(s); h = lb(Q)[0]
print('start: K', lb(Q), 'letters', len(W), 'out', len(outset(G)))
for D in range(DO + 1):
    stats['nodes'] = 0; stats['clear tries'] = 0
    res = level(s, G, h, D, [], set())
    print(f'height {h}, depth {D}: {"found " + str(res[0]) if res else "none"} ; {stats}')
    if res: break
```

Output:

```text
start: K (14, 0) letters 152 out 15
height 14, depth 0: none ; {'nodes': 1, 'clear tries': 0}
height 14, depth 1: none ; {'nodes': 13, 'clear tries': 0}
height 14, depth 2: none ; {'nodes': 1000, 'clear tries': 38}
height 14, depth 3: none ; {'nodes': 1799, 'clear tries': 123}
```

## 24. What changed in METHOD.md

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
- After round 7 (section 12):
  - Section 2, phase 2: the cascade with the replaced seeds, its measured lengths and the new bounds.
  - Section 3: a new row for replacing a seed; the rows for the lattice-phase length, the short pure three-cycles and the frame elements now give the round 7 results.
  - Section 5, gap M2: the measured and upper bounds with the replaced seeds, the tighter bound for the committed code (R7-03), the shorter seeds now in use, and the next experiments.
  - Section 6.3 gives the run with the replaced seeds, and section 7 its commands.
- After round 8 (section 13):
  - Section 3: the rows for Lemma C, the other cell kinds, the toolkit theorem and the lattice-phase length are corrected (R8-04, R8-05, R8-08).
  - Section 5: the s61c bound of gap M1 says "to the lattice" (R8-03); gap M2 names the cascade and the corrections, says that the lower bounds count retained words, and records that the round 7 selection is optimal over the recorded candidates (R8-02, R8-07, R8-08).
- After round 9 (section 14): section 5, gap M1, has a new item on the finiteness of R(I_a) (T9-02, T9-04, T9-05).
- After round 10 (section 15):
  - Section 2, phase 2, and the row for the lattice-phase length in section 3 give the bound of the construction with direct three-cycles (T10-02).
  - Section 5, gap M2: a new item on direct three-cycles and on the twisted cycles that fail as proposed, and the next experiments (T10-01 to T10-04).
- After round 11 (section 16):
  - Section 2, phase 2, and the rows for the lattice-phase length and its lower bounds in section 3 give the bound of the construction with fewer seed applications and the floor for constructions from seed conjugates (T11-05, T11-06).
  - Section 5, gap M2: a new item on fewer seed applications, and the next experiments (T11-01 to T11-07).
- After round 12 (section 17):
  - Section 2, phase 2: the expected lengths are upper bounds (R12-07), the comparison of round 11 with the floor of T11-06 names its class of cascades (R12-01), and the kit with the replaced seeds is a runner separate from the driver, whose lattice phase runs the committed cascade.
  - Section 3: the rows for the lattice-phase length and its lower bounds give the expected lengths as upper bounds and the scope of the floor (R12-01, R12-07); the row for replacing a seed says "checked by" the stage 03 checker, and the row for the completeness of the driver says what `test_fallback.py` covers (R12-03, R12-10).
  - Section 5, gap M2: the upper bounds give expected lengths as upper bounds and count actions as single twists and macro applications, and the item on fewer seed applications gives the scope of the floor and the ratio 1.334 (R12-01, R12-07, R12-09).
  - Sections 6.1 and 6.3 say how the runs were checked: the off-lattice words and the corrections were replayed on the exact simulator, and the retained cascade was evaluated as an exact permutation program (R12-03).
- After rounds 13 and 13b (section 18):
  - Section 5, gap M1: the finiteness item gives P6 and its equivalence with the bound 6 on left heights (T13-03), the exact cap decisions of REFX (T13-01), where the blocking comes from (T13b-02, T13b-05), the open case, and the fact about grip 433.
  - Section 6.2: the e2run24 run to its end, and the fix to `cellpure3.py`, which stopped with a `KeyError` when no pair tool moves the unit's cell and now reports that no cycle passes through it.
- After round 14 (section 19): section 5, gap M1, the finiteness item gives the trap lemma (T14-03), the failure of RA for single pieces (T14-01, T14-02) and the cases that stay open.
- After round 15 (section 20): section 5, gap M1, the finiteness item says that P6 and the bound 6 on left heights are false (T15-01), that a high island can enter the cap of a grip that is already out (T15-02), and that R(I_a) is finite if and only if the clean steps have a uniform length bound, without hypothesis H (T15-04).
- After round 16 (section 21): section 5, gap M1, the finiteness item says that no height 10 was found, that the inner cohorts alone do not bound the height (T16-02), and that two routes to a proof or a pump are closed (T16-03, T16-04).
- After round 17 (section 22): section 5, gap M1, the finiteness item says that heights 10, 12 and 14 are reachable by exact words of 147, 149 and 152 letters (E17-4, E17-6, E17-7), through turns at the raiser, and that TH holds at the raise to 14.
- After round 18 (section 23): section 5, gap M1, the finiteness item says that TH is decided by four orientations at the raiser and not by height (T18-01, T18-02), that at most 120 grips are out at once (T18-04), and that two integrator lemmas, not yet reviewed, explain the stop at height 14 (E18-1 to E18-3).
