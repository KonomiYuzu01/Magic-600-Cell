# PROOFS-B: lemmas toward B′(h) at low heights (theory round 22)

18 October 2026. **Model review only.** The senior reviewer (a Codex model called read-only through the project's
review wrapper) wrote these proofs in theory round 22. The integrator read each one against the sources, re-ran every
computation listed here, and wrote them in this form. No human has reviewed them and nothing here is formalized. The
file is written so that it can be handed to the owner's Lean formalization agent (Archon) once the owner decides; E2
and the Lean plan wait for that decision
([record](../../../../docs/wiki/decisions/owner-decisions-2026-10-17-jumbling-m2.md)).

The lemmas serve conjecture B′(h) of the [stage 12 README](README.md), section 1, which is to be proved as an
existence statement with explicit bounds that need not be small
([owner decision, 18 October 2026](../../../../docs/wiki/decisions/owner-decisions-2026-10-18-bprime-goal.md)). The
review record of the round, with its finding ids, is
[`SENIOR-REVIEW.md`](../09-general-method/SENIOR-REVIEW.md) section 27. Line (c) of the round (the T2 catalogue and T3
unlocking) was a joint attack of the senior reviewer and the solver; its lemmas, after the cross-examination and a
second round by the senior reviewer, are in section 7. The second round of line (a) (R22A2) closes section 2.

Evidence kind: exact synthetic geometry over Q(√5) and source evidence; headless Linux. Nothing here is a Windows,
DirectX, input or performance result.

## 0. How each lemma is written

- **Objects**: what the lemma is about; section 1 defines the shared ones.
- **Hypotheses** H1, H2, ...: each is a finite exact check at the current state Z, and the function or script that
  decides it is named. No hypothesis refers to the history of Z.
- **Statement** and **proof**.
- **Bound**: an explicit expression, where the lemma gives one. A bound containing a factor such as 7,200^n, lcm(1,
  ..., 13,205) or a binomial coefficient in 177,120 is an *existence bound*: it shows that a finite search decides the
  question and that a word of at most that length exists. It is not a length a person can execute; under the size
  criterion of 14 October only bounds of human scale count as lengths.
- **Instances**: computations at named states, with the command (section 6) and its output. An instance is evidence
  about one state. It is never part of a proof.
- **Uses**: results proved elsewhere and used here as given. Until they are formalized they are the axioms of this
  file.
- **Status**: *proved* (model review); *computed* (exact computation, re-run by the integrator); *conditional* (proved
  under hypotheses that are not shown to hold at every state of 𝒞_h); *open*.

Words are chronological: the first letter acts first. For a word w of retained letters, π_w is the permutation of the
home chambers that w induces, and the holes after w are π_w(holes). A product of words uv means u, then v.

## 1. Objects

The setting is that of [`METHOD.md`](../09-general-method/METHOD.md),
[`buffer-transport.md`](../../theory/buffer-transport.md) section 1 and `SENIOR-REVIEW.md` section 4.

- **O1, pieces and the lattice.** Menu I_a on the retained 600-cell-Full geometry: 177,120 pieces, each with an exact
  pose, a rotation of R⁴ with entries in Q(√5). K⁺, the rotation group of the 600-cell, has order 7,200. A piece is
  on the lattice when its pose lies in K⁺ and off otherwise; n(Z) is the number of off pieces. Ω is the set of the
  177,120 home chambers; a chamber is a hole of Z when no lattice piece occupies it, so |holes(Z)| = n(Z).
- **O2, letters.** There are 600 grips. A letter (c, j) applies the element of menu item j at grip c
  (`restore.element`) to every piece whose region lies inside the cap of c. It is admissible when no piece straddles
  the cut of c; a word is admissible when every prefix is. The items of a grip whose element lies in K⁺ are its
  *retained* letters (11 besides the identity, closed under inverses); the other 48 are jumble letters. A retained
  letter keeps every piece's lattice membership. A lattice piece never straddles a cut.
- **O3, heights.** The left height of a piece is the tree distance of the left factor of its pose (`trees.pose_tree`);
  off pieces have even height at least 2. 𝒞_h is the set of reachable states whose off pieces all have left height at
  most h.
- **O4, steps.** A unit is a rigid block of off pieces or one frame part of a multi-frame block. A word w admissible
  at Z is a *strict′ step* for a unit U when, after w, U and every lattice piece of Z are on the lattice and every
  other off piece of Z is on the lattice or at its exact pose of Z; it is *strict* when every other off piece keeps its
  exact pose. The *endpoint collateral* C_Z(w) is the set of lattice pieces of Z that are off after w.
- **O5, SAFE-ALL.** A(Z): the grips whose cut no piece straddles and whose cap holds no off piece, with their 11
  retained letters each. A word over A(Z) is admissible at every prefix, fixes every off piece and maps the holes by
  π_w. G(Z) is the group these letters generate on Ω, Fix(G) its fixed chambers. For a target T ⊆ Ω: N1, every chamber
  of T ∩ Fix(G) is a hole; N2, |holes ∩ Δ| ≥ |T ∩ Δ| for every orbit Δ. Proposition N (N1 and N2 are necessary),
  Jordan certificates, Theorem A and Corollary A1 are those of `buffer-transport.md` sections 2 and 3. Decided by
  `buffer.condition` and `buffer.check_jordan` (`research/jumbling/theory/buffer.py`).
- **O6, ghosts and Bad.** For a word W, the ghost of a home chamber is a virtual lattice piece there at the identity
  pose, moved by W as a piece there would be. Bad(W) is the set of chambers whose ghost straddles a cut at some prefix
  or ends off the lattice (`fastghost.Ghost`, `ghostlib.ghost`). If a SAFE-ALL word V has Bad(W) ⊆ holes(Z·V), then
  V·W·V⁻¹ takes no lattice piece off (AU-03, `SENIOR-REVIEW.md` section 9).
- **O7, the class-cell model (LC-01).** Fix a class of grips P (120 grips) and put G_P = Sym⁺(P), of order 7,200, and
  H_P = G_P ∩ K⁺. The labels are the five cosets G_P/H_P; label 0 is H_P. A cell is a sign vector over the 120 caps of
  P; there are 2,641 cells, one of them outer. A point is a pair (C, ℓ), so there are N_P = 13,205 points. A content
  is the set of pieces with the same cell and the same domain; a class-P off content has poses h·k with h ∈ G_P and
  k ∈ K⁺ and sits at the point (C, hH_P); the lattice pieces of a home cell C (a cell that holds home chambers) form
  its lattice content, at (C, 0). The *kind* of a cell is the number of caps of P that contain it (vertex, edge,
  triangle, tetrahedron). A grip of P is *free* when it is admissible and its cap holds only lattice pieces and P's
  frames. A letter (c, g) at a free grip sends (C, ℓ) to (gC, gℓ) when C lies in the cap of c and fixes it otherwise.
  Decided by `e3_class.Model` and `classes.py` (`research/jumbling/theory/`).
- **O8, LC-03 (used throughout).** Over free class-P letters: every letter stays admissible (own pieces never
  straddle a cut of P, foreign pieces stay outside free caps); all pieces of one cell C receive one accumulated
  rotation a_C ∈ G_P, so their points move exactly as the model says; and a rotation that fixes a nonempty cell and all
  its five labels is the identity.

## 2. Line (a): transport without endpoint collateral

### R22A-01. Zero Bad means zero endpoint collateral (proved)

- Objects: a state Z, a class P, a word W over the free class-P letters of Z.
- Hypotheses: H1, every letter of W is at a grip of P that is free at Z (O7).
- Statement.
  1. No ghost of a home chamber straddles a cut of P, and Bad(W) is the union of Ω_C over the home cells C whose
     label-0 point (C, 0) W sends to a point of nonzero label.
  2. C_Z(W) is the set of lattice pieces of Z at chambers of Bad(W). So C_Z(W) = ∅ exactly when
     Bad(W) ⊆ holes(Z).
  3. If W lands a nonempty off content of kind κ, Bad(W) contains Ω_C for at least one home cell C of kind κ: at
     least 111 chambers for a triangle cell and 87 for a tetrahedron cell, even when they are all holes.
- Proof.
  1. A ghost of a chamber of cell C has pose 1 ∈ H_P, so it never straddles a cut of P (O8). After W its pose is the
     accumulated rotation a_C ∈ G_P, which lies in K⁺ exactly when a_C ∈ H_P, that is, when a_C sends label 0 to
     label 0. All ghosts of Ω_C share a_C.
  2. A lattice piece of Z in cell C has a pose k ∈ K⁺ and ends at a_C·k, which lies in K⁺ exactly when a_C ∈ K⁺ ∩
     G_P = H_P, the condition of item 1 for the ghost of its chamber.
  3. Class-P words act on points as permutations and keep the kind of a cell. Let S_κ be the finite set of label-0
     points of home cells of kind κ. W sends the unit's point p ∉ S_κ into S_κ, so W cannot send all of S_κ into S_κ;
     some (C, 0) ∈ S_κ goes to a nonzero label (a label-0 point outside a home cell would put a lattice ghost off
     the body, which a pose in K⁺ cannot do), and Ω_C ⊆ Bad(W) by item 1.
- Remark. The guarantee concerns endpoints, not prefixes: at the depth 40 stop the first jumble letter of any landing
  word takes at least 3,097 − 1,193 = 1,904 lattice pieces off (M1-03), so no landing word there keeps every lattice
  piece on the lattice at every prefix.
- Uses: O8 (LC-03); M1-03 for the remark.

### R22A-02. Whole-cell ownership (proved)

- Objects: Z, P, a word W over the free class-P letters of Z, a nonempty class-P off content Q of cell C.
- Hypotheses: H1 as in R22A-01; H2, Q is a class-P off content (O7).
- Statement. W keeps the exact pose of every piece of Q if and only if W fixes all five points (C, 0), ..., (C, 4).
  Fixing only the point of Q is not enough. A lattice content of a home cell C stays on the lattice if and only if W
  sends (C, 0) to a point of label 0; its chamber and exact pose need not be fixed. Pieces of a foreign frame never
  move.
- Proof. All pieces of C receive a_C (O8). If W fixes the five points, a_C fixes C and its labels, so a_C = 1 (O8).
  Conversely, if one piece of Q with pose γ keeps it, a_C·γ = γ and γ is invertible, so a_C = 1 and all five points
  are fixed. A lattice piece of C with pose k ends at a_C·k, which lies in K⁺ exactly when a_C ∈ H_P. Foreign pieces
  lie outside every free cap.
- Counterexample to the point rule: the nonidentity C3 rotations of a triangle cell in LC-12 fix two labels; a content
  at one of them keeps its point and changes its exact pose.
- Uses: O8; LC-12 for the counterexample.

### R22A-03. The transport lemma (proved)

- Objects: Z, P, the point model of O7, a class-P off content U at p = (X, ℓ). The *owned set* O_U consists of the
  label-0 points of the home cells that hold lattice pieces, every point of the cell of every other class-P off
  content, and every point of every *excluded* cell (a cell with an off piece of another class or with two off
  contents). U's own ownership is removed; ownership of the same points by anyone else is kept.
- Hypotheses.
  - H1: U is a whole content at p with ℓ ≠ 0.
  - H2: every grip used is free at Z.
  - H3: no point of the fibre {(X, 0), ..., (X, 4)} lies in O_U (U is not glued to another content of its cell).
  - H4: a path p = q_0, q_1, ..., q_m with q_{i+1} = A_i^{L_i}(q_i), where A_i = [x_i, y_i] is the commutator of two
    letters at free grips and L_i is the least common multiple of the lengths of the cycles of A_i that meet O_U (1
    when none does); q_m = (H, 0) for a home cell H of U's kind with no point of its fibre in O_U.
  - All four are exact computations in the model (`zpath4.py`, section 6).
- Statement. W = A_0^{L_0} ··· A_{m−1}^{L_{m−1}}, a word of 4·Σ L_i letters, is admissible at Z and is a strict step
  for U with C_Z(W) = ∅.
- Proof.
  1. A point on a cycle of length λ is fixed by A^e exactly when λ divides e, so each A_i^{L_i} fixes every point of
     O_U, and so does W.
  2. Every other class-P off content and every excluded cell has its whole fibre in O_U, so it keeps its exact pose
     (R22A-02). Every home cell with lattice pieces keeps label 0 at label 0, so Bad(W) consists of chambers of
     vacant home cells only and C_Z(W) = ∅ (R22A-01).
  3. W sends p to (H, 0), so the pieces of U receive a rotation that takes label ℓ to label 0: they land.
  4. Admissibility at every prefix is O8.
  5. Free cells: a class-P word sends the fibre of each cell bijectively onto the fibre of a cell, so a power that
     fixes a point of a cell fixes that cell. The cells that meet O_U are therefore permuted among themselves by each
     clean power, and so are the other cells; by H3, U is in one of the other cells at every power boundary (not
     necessarily inside a power).
- A failed test refutes this construction only, not every word with C_Z = ∅.
- Uses: R22A-01, R22A-02, O8.

### R22A-04. Bounds and completeness for clean powers (proved)

- Statement. Put N_P = 13,205 and E = lcm(1, ..., N_P). For every commutator A, L_A divides the order of A on the
  points, which divides E. If the component of p in the graph of R22A-03 has f points, a reachable target has a path
  of at most f − 1 clean powers and a word of at most 4E(f − 1) ≤ 4E(N_P − 1) letters.
- Completeness. Every power of A that fixes O_U has an exponent divisible by L_A, so the powers A^{L_A}, one per
  commutator, generate every individually clean power. Each has finite order, so its inverse is a positive power of
  it and the undirected components of the graph are its reachability classes. The component of p, computed over every
  commutator of the alphabet with no pair filter, decides whether this family reaches a target. A search cut at a
  fixed number of powers finds witnesses but proves no absence. With m commutators (m ≤ (120 · 59)²) a breadth-first
  search examines at most m·N_P point moves.
- Remark (integrator, not reviewed): the order of a permutation of N_P points is at most Landau's function g(N_P), and
  ln g(N) ≤ 1.05313·√(N ln N) (Massias, Nicolas and Robin, 1989), so E can be replaced by g(N_P) < e^{373}.
- Existence bound only. The paths found at the depth 40 stop have 1 to 3 powers with exponents up to 6 (section 4 of
  this file and the stage 12 README).

### R22A-05. No merging; target capacity (proved)

- Statement.
  1. Under class-P words distinct points stay distinct and distinct cells stay distinct. Two contents that start at
     different points never end at one point.
  2. Capacity. Let M_κ be the number of home cells of kind κ in class P and v_κ the number of those that hold no
     lattice piece. Along any sequence of class-P words after which every original and every landed lattice piece is
     on the lattice, at most v_κ initially distinct off contents of kind κ land.
  3. Type counts. If each home cell of kind κ has b_{κ,t} chambers of type t, N_t is the number of chambers of type t
     and h_t the number of holes of type t, then max(0, M_κ − Σ_t min(M_κ·b_{κ,t}, N_t − h_t)) ≤ v_κ ≤
     min over t with b_{κ,t} > 0 of ⌊h_t / b_{κ,t}⌋.
- Proof.
  1. Each letter permutes the cells and, cell by cell, the five labels; so does every word.
  2. At the start M_κ − v_κ label-0 points of kind κ hold lattice contents. At the end these contents are still on the
     lattice, at distinct label-0 points of kind κ (kinds are preserved), and every landed content of kind κ is at a
     further distinct label-0 point of kind κ (item 1). There are M_κ such points, so at most v_κ contents land. Moving
     lattice contents between home cells does not change the count.
  3. Vacant cells have disjoint chambers, which gives the upper bound. Each nonvacant cell holds at least one lattice
     piece, and the lattice pieces of type t in cells of kind κ number at most min(M_κ·b_{κ,t}, N_t − h_t), which gives
     the lower bound.
- Consequence. A cell partly filled by an earlier landing (a 33-piece content in a 111-chamber triangle cell leaves 78
  holes there) cannot receive another content by class-P words while its content stays: its label-0 point is taken.
- Instance (R22A-08): at the depth 40 stop class 3 has capacity three triangle contents and one tetrahedron content,
  against five triangle contents (b00, b02, b07, b13, b14) and two tetrahedron contents (b01, b11) that clean powers
  move; class 4 has capacity 0 for every kind and class 2 has no tetrahedron capacity. These are obstructions for one
  fixed class, not after a SAFE preparation or words of another class.

### R22A-06. Entry and closure (proved; conditional on the hypotheses)

- Entry. Objects as in R22A-03; r = (H, 0) an occupied label-0 point of a home cell of U's kind, h its lattice
  content; A a commutator at free grips and L the least common multiple of the lengths of the cycles of A that meet
  O_U \ {r}.
  - Hypotheses: H1 to H3 of R22A-03; H4, a clean path T from p to a point q; H5, q and r lie on one cycle of A, of
    length λ, with r = A^d(q), and gcd(L, λ) divides d.
  - Statement. Choose e ≡ d (mod λ) with L | e and 0 ≤ e < ord(A) (Chinese remainder theorem). Then W = T·A^e lands U
    at (H, 0), keeps every other protected off content at its exact pose, and takes off no lattice piece outside h.
    It takes all of h off exactly when A^e(r) has a nonzero label; A^e(r) can also be a vacant label-0 point, and then
    W takes nothing off. |W| ≤ 4E·f with f as in R22A-04.
  - Proof. T and A^e fix every owner other than r (as in R22A-03); apply R22A-02 to each owner and read r's final
    label. Reducing e modulo ord(A) keeps the exact action: A^{ord(A)} fixes every point, hence every non-outer cell
    with its labels, hence (O8) every accumulated rotation is 1.
- Closure. For this W compute Bad(W) completely, at every chamber of the union of its caps.
  - Hypothesis: a SAFE-ALL word V at Z with Bad(W) ⊆ holes(Z·V).
  - Statement. V·W·V⁻¹ is a strict step of 2|V| + |W| letters.
  - Proof. V fixes every off piece, so W meets the same off geometry at Z·V and acts on the off pieces as at Z; it
    takes no lattice piece off (O6). After W the off pieces are those of Z without U, at their poses of Z, so every
    letter of V⁻¹ is still a SAFE letter: it is admissible, fixes the off pieces and keeps the landed pieces of U on
    the lattice.
  - V must vacate all of Bad(W): the part of Bad(W) in initially vacant cells (from T) counts too, because V can move
    lattice pieces into it. If h has 22 pieces in a 111-chamber triangle cell and W sends its label 0 off, Bad(W) holds
    all 111 chambers.
- Empty cell first. A SAFE word V′ with Ω_H ⊆ holes(Z·V′) exists exactly when N1 and the target-orbit test of M1-17
  hold for Ω_H. After V′ the owners are recomputed; H is then vacant, and the graph of R22A-03 at Z·V′ must still reach
  (H, 0). If it does, V′ followed by that path is a strict step.
- Other classes, or landing other units first, need fresh checks; nothing here proves that either always exists.
- Uses: R22A-02, R22A-03, R22A-04, O6, O8, M1-17.

### R22A-07. Glued contents (proved; instance computed)

- Statement. If a class-P off content U and a nonempty lattice content h lie in the same cell X at Z, every class-P
  word that lands U takes every piece of h off the lattice: Ω_X ⊆ Bad(W).
- Proof. All pieces of X receive one rotation a. Take a pose γ ∉ K⁺ of a piece of U and a pose k ∈ K⁺ of a piece of h.
  If a·γ and a·k both lay in K⁺, then a = (a·k)·k⁻¹ ∈ K⁺ and γ = a⁻¹·(a·γ) ∈ K⁺, a contradiction.
- The SAFE remedy and its limit. If a SAFE-ALL word V vacates Ω_X, then V·W·V⁻¹ with W from Theorem C (edge cell, at
  most 720 letters when the two poles and a common neighbour are free) or from LC-12 (vertex cell, at most 104 letters
  under its free-grip hypotheses) is a strict′ step of at most 2|V| + 720 or 2|V| + 104 letters. Triangle and
  tetrahedron contents cannot land in their own cell (LC-12); they need a transport.
- Instance (R22A-08): at the depth 40 stop Ω_X has occupied chambers fixed by the whole SAFE-ALL group for each of the
  seven glued units: 4 for b03, 4 for b18, 5 for b04, 6 for b05, 5 for b12, 6 for b15 and 27 for b06. N1 fails, so by
  Proposition N no SAFE-ALL word of any length vacates Ω_X, and no closure V·W·V⁻¹ with W a class-P landing word
  exists for these units at this state. A step that changes the off geometry has to come first.
- Uses: O8, Theorem C, LC-12, Proposition N.

### R22A-08. Vacant home cells at the depth 40 stop (computed)

- Statement. At the depth 40 stop (state digest `7e128820…`, 1,193 off pieces), counting home cells that hold no
  lattice piece: class 2 has two triangle cells and one edge cell (the edge cell holds the off content b17 itself, so
  it is no target for another unit); class 3 has three triangle cells and one tetrahedron cell; class 4 has none. N1
  fails for the own cell of every glued unit, with the fixed occupied counts of R22A-07. The free grips number 113,
  116 and 111 in classes 2, 3 and 4.
- Command: `scripts/round22/r22a08_vacancy.py` (section 6); output `runs/round22/r22a08_vacancy.txt`.

### R22A-09. The five disputed units (computed)

- b02, b07, b13, b14 and b21 had model paths in a census that protected points instead of cells; their exact replays
  turned another off content of a moved cell to another pose at the same point. R22A-02 explains the failure.
- Under the whole-cell rule all five have strict′ steps (section 4): b02 60, b07 48, b13 60 and b14 60 letters in
  class 3, b21 84 letters in class 2, each replayed exactly. Command: `scripts/round22/transport_census.py`
  (section 6); records `runs/round22/line-a/census-class3.json` and `census-class24.json`.
- A wider exact decision procedure exists for one fixed free class alphabet: record the accumulated rotation
  a_C ∈ G_P of each of the 2,640 non-outer cells; accept when U lands, every lattice content ends at label 0 and every
  other protected off cell has a_C = 1. There are at most 7,200^2,640 records, so breadth-first search terminates, and a
  reachable accepting record has a word of at most 7,200^2,640 − 1 letters (existence bound). It admits intermediate
  disturbance that cancels later and lattice contents that change cells. It was not run.

### Round 2 of line (a)

After the census and the closures of section 4 the senior reviewer took line (a) a second round (findings R22A2-01 to
R22A2-06, 18 October 2026): whether b19's V can be built, whether the census steps compose, and what bounds and
failure modes the closures have. Its exact computation is `scripts/round22/r22a2_04_sequence.py` (section 6); the
integrator re-ran it and got the output `runs/round22/r22a2_04_sequence.txt`. Throughout, Z is the depth 40 stop
(digest `7e128820…`, 1,193 off pieces), G = G(Z), and the *middles* are the words that land one unit each: the ten
census steps (b00, b01, b02, b07, b11, b13, b14, b16, b20, b21) and the entry transports of b08, b09, b10 and b19
(section 4).

### R22A2-01. b19's vacating word by construction (computed hypotheses; proved consequence)

- Objects: W, b19's 84-letter entry transport (`runs/round22/line-a/closures/b19-core.json`); T = Bad(W), 198
  chambers, 15 occupied (`closures/b19-target.json`); the 20 orbits Δ of G that meet T (orbits 16 to 35 of
  `buffer.Alphabet`, sizes 120 to 7,200), d = |Δ|.
- Hypotheses (finite checks at Z):
  - H1: N1 and N2 hold for T.
  - H2: every Δ has a global Jordan certificate (w, p) with |w| = 120 (H1 of R22B-08; `buffer.check_jordan`, on all
    177,120 chambers). The orbit of 120 chambers has p = 73.
  - H3: on every Δ the SAFE letter (0, 1) moves r chambers with 2 ≤ r and 2r − 1 ≤ d (H2 of R22B-08); r runs from 4
    to 204, and r = 4 on the orbit of 120 chambers.
- Statement. A SAFE-ALL word V with T ⊆ holes(Z·V) exists with |V| ≤ B19 = 2·Σ_Δ |T ∩ Δ|·A_Δ, where
  A_Δ = s_Δ + 2e_Δ + 2f_Δ, s_Δ = 4 + 4·[120·M_Δ + 2(d − p)]·[C(d, r) − 1] with M_Δ as in R22B-08, e_Δ = (d − 2)(2d − 1)
  and f_Δ = (d − 3)(2d − 3)(2d − 1). B19 is a 539-digit integer, so |V| < 10^539, and V·W·V⁻¹ is a strict step of at
  most 84 + 2·B19 < 10^540 letters (R22A-06).
- Proof. R22B-08 turns H2 and H3 into a seed on Δ that fixes every other chamber, of at most s_Δ letters; R22B-05
  gives the tables with e_Δ and f_Δ; R22B-01 clears T ∩ Δ orbit by orbit using only the current holes, with the
  bound above; R22A-06 closes.
- Construction bound, an existence bound in the sense of section 0: the seed search on the subset graphs, the
  complete Schreier tables and the expanded V were not run.
- Command: section 6. `r22a2_04_sequence.py` checks H1 to H3 (its CORE row for b19 and its assertions); b19's closure
  bound 84 + 2·B19 is the difference of its two ALL_RECLOSURES_BOUND rows. `jordan_target.py` on
  `closures/b19-target.json` rechecks H1 and H2 alone.
- Uses: R22A-06, R22B-01, R22B-05, R22B-08.

### R22A2-02. When a second strict′ step survives the first (proved)

- Objects: strict′ steps w1 and w2 at Z for disjoint units; Z1 = Z·w1. Bad(w2) is the complete ghost Bad set of O6,
  prefix straddling and endpoint failure both.
- Statement. w2 is a strict′ step for its unit at Z1 if and only if Bad(w2) ⊆ holes(Z1).
- Proof. Every off piece of Z1 was off at Z with the same pose, so its trajectory under w2 is the one the strict′
  check of w2 at Z accepted: admissible, and ending on the lattice or at its pose of Z. Every other piece of Z1 is a
  lattice piece; piece trajectories are independent, so each follows the ghost of its chamber. All of them survive
  every cut and end on the lattice exactly when none occupies Bad(w2). This covers the pieces of w2's unit that w1
  may already have landed.
- A test at Z alone. Let L = Ω \ holes(Z), φ1 the map of w1's good ghosts to their end chambers, and D1 the end
  chambers of the pieces w1 lands. The occupied chambers of Z1 are φ1(L) ∪ D1, so the criterion is
  Bad(w2) ∩ φ1(L) = ∅ and Bad(w2) ∩ D1 = ∅, two finite exact checks. Applied in both directions it decides mutual
  survival; it says nothing about equal final poses. Disjoint unions of the two words' caps are a stronger sufficient
  condition. Distinct destination cells alone are not: after b07, b14's word has 111 occupied Bad chambers although
  the two land in different triangle cells.
- Capacity case. If w1 is a class-P clean-power step landing only U1 in a previously vacant home cell H1 of kind κ,
  every occupied home label-0 point stays fixed and every other off content stays exact, so v_κ drops by exactly one
  and the other kinds' counts do not change (R22A-05). Another class-P census word w2 then survives exactly when it
  sends (H1, 0) to label 0.
- Uses: O4, O6.

### R22A2-03. The thirteen supplied steps compose at most six deep (computed)

- Objects: the thirteen supplied strict′ steps at Z: the ten census words and the complete closures V·W·V⁻¹ of b08,
  b09 and b10 (section 4).
- Statement. Write i: j when the word for j is not a strict′ step after the word for i (R22A2-02). The failing
  directions are exactly
  - b00: b02, b08, b10, b16, b21; b02: b00, b08, b10, b16, b21; b08: b00, b02, b10, b16, b21;
    b10: b00, b02, b08, b16, b21; b16: b00, b02, b08, b10, b21; b21: b08, b10, b16;
  - b01: b07, b09, b11, b13; b11: b01, b07, b09, b13; b07: b01, b09, b13, b14; b13: b01, b07, b09, b14;
    b09: b01, b07, b11, b13, b14, b20; b14: b09; b20: b09;

  52 failing and 104 passing directions. The largest set of words compatible in both directions has four members
  (twelve such sets). Used once each, in some order and with no further preparation, at most six of the words are
  strict′ in succession, for example b21, b00, b14, b07, b11, b20; 261 subsets are reachable. So no order of all
  thirteen unchanged words is strict′ throughout.
- Proof. Each word fixes the original occupied chambers and every passing transition keeps the earlier landing sets
  (both checked exactly), so after any successful prefix the occupied set is the original one plus the landing sets of
  the chosen units. The subset search over these sets is therefore complete for every length, not a bounded search.
- The counts are occupied Bad chambers (a blocked prefix or endpoint collateral). For instance b16's word after b00
  meets 19 occupied chambers and b00's after b16 meets 45; b10's closure after b08 meets 33 and b08's after b10 34.
- Command: section 6 (the MATRIX and FIXED_WORDS_MAX rows).
- Uses: R22A2-02.

### R22A2-04. Fresh closures land fourteen units in a checked order (proved; computed hypotheses)

- Objects: a SAFE subgroup G0 at Z (here all of G); pairwise disjoint off units U_i with middles W_i, each landing U_i
  and keeping every other off piece of Z at its exact pose; B_i = Bad(W_i), φ_i the map of W_i's good ghosts to their
  end chambers, D_i the landing chambers of U_i; h_Δ the holes of Z in the G0-orbit Δ.
- Hypotheses (finite checks at Z), for a proposed order of the units:
  - H1: every orbit of size at least 2 that meets some B_i has a global Jordan certificate.
  - H2: φ_i maps every chamber of its good domain into the same G0-orbit.
  - H3: before step i, h_Δ − Σ_{j earlier} |D_j ∩ Δ| ≥ |B_i ∩ Δ| on every orbit Δ, singletons included (so N1 is the
    singleton case).
- Statement. Then fresh SAFE words V_i exist such that V_1·W_1·V_1⁻¹, V_2·W_2·V_2⁻¹, ... in that order is a sequence
  of strict steps, each landing its unit.
- Proof (induction along the order). Earlier strict closures leave every remaining off piece at its pose of Z, so
  every letter of G0 is still SAFE, and the Jordan certificates still give independent alternating factors on the old
  orbits (Lemma M). H3 and Corollary A1, applied to G0, give V_i with B_i ⊆ holes after V_i. V_i·W_i·V_i⁻¹ is strict
  (R22A-06): the remaining off trajectories do not change, every occupied ghost is good, and V_i⁻¹ is SAFE after U_i
  lands. The old lattice pieces stay in their G0-orbits under all three parts (H2), and U_i adds exactly |D_i ∩ Δ|
  occupied chambers to Δ, so the hole count of Δ drops by exactly that much. No certificate on an orbit that merges
  later is used.
- Instance at Z. Every one of the fourteen middles has the stronger property φ_i(x) = x for every good chamber x; their
  Bad sets meet only the 20 orbits certified in R22A2-01; replay confirms each middle's off-piece contract. The order
  b00, b01, b02, b07, b08, b11, b13, b19, b09, b10, b14, b16, b21, b20 satisfies H3: the off counts before the steps
  are 1,193, 1,160, 1,088, 977, 866, 833, 804, 703, 687, 629, 595, 517, 406 and 396, and 336 after the last; the least
  slack over the target orbits before each step is 11, 1, 11, 0, 9, 0, 0, 0, 0, 2, 0, 1, 0, 0. All 2^14 subsets are
  reachable. Without b19 the thirteen land in a similar order and leave 352 (all 2^13 subsets reachable).
- Order matters. b01 and b09 each use one hole of the 120-chamber orbit 35, which has two; with both before b07, b11,
  b13 or b19, which need one there, H3 fails. This is about the certificate of G0, not about every later strategy.
- Bound. With A_Δ of R22A2-01 and k_iΔ = |B_i ∩ Δ|, closure i has at most |W_i| + 4·Σ_Δ k_iΔ·A_Δ letters. The
  fourteen middles have 1,116 letters together, and the sum of the fourteen closure bounds is a 541-digit integer
  (< 10^541); for the thirteen without b19, 1,032 letters and a 540-digit bound (< 10^540). Construction bounds only:
  no V_i and no multi-step sequence was built or replayed; the existence follows from the induction and the checked
  hypotheses.
- Command: section 6 (CORE, RECLOSURE, RESOURCE_STEP, OFF_AFTER and ALL_RECLOSURES_BOUND rows).
- Uses: R22A-06, R22A2-01, Corollary A1, Lemma M.

### R22A2-05. A shared target: a fresh closure, not the old preparation (proved; instance computed and built)

- Statement. After a strict step, take a remaining unit's middle W. Its off-piece contract survives, because every
  remaining off piece keeps its pose. If N1, N2 and the needed Jordan certificates hold for the complete Bad(W) at the
  new state, a new SAFE word V makes V·W·V⁻¹ a strict step, even when W enters a home cell an earlier landing now
  occupies; it has |W| + 2|V| letters. V⁻¹ can change the cell in which the unit ends, so the middle's entry cell is
  not the closure's final cell.
- Instance. b08 and b10 enter the same home cell {35, 178, 189}, and their recorded preparations are the same
  63-letter word; the supplied closures fail in both orders (R22A2-03). After either closure the other core's
  111-chamber Bad set still passes N1 and N2 for G, with the Jordan certificates of R22A2-01: the full closure's Bad
  set is the preimage of the core's under a permutation of G, so the orbit demands are the same. So a new V lands the
  second unit, with |b10's step| ≤ 144 + 2·B(T), B(T) = 2·Σ_Δ |T ∩ Δ|·A_Δ, or by R22B-02 ≤ 144 + 2·[C(177,120, 1,160)
  − 1]; for b08 after b10, ≤ 96 + 2·[C(177,120, 1,159) − 1]. The old 63-letter preparation does not do it: b10's
  270-letter closure after b08 meets 33 occupied chambers. The same argument covers b01 and b11 and b16 and b21; the
  order of R22A2-04 gives capacity for all fourteen at once.
- Built (line (a), round 3; measured, exact). At the state after b08's 222-letter step, N1 and N2 hold for b10's
  Bad(W) (111 chambers, 26 occupied), the peeling certificate of Theorem B is complete with a V of 396 letters
  (its bound 1,371), and V·W·V⁻¹ is a strict′ step of 936 = 144 + 2·396 letters. Its replay from the exported table
  (`stepchk.py`) lands all 34 pieces, takes no lattice piece off and keeps every other off piece at its exact pose
  (1,160 to 1,126 off; `runs/round22/line-a/b10-after-b08/`).
- No contradiction with R22A-05: its capacity bound is for words inside one class, while V may use retained letters
  of other classes and move lattice contents before the class-P middle. With all extra work restricted to class P the
  capacity obstruction stands.
- Uses: R22A-06, R22A2-02, R22A2-04, R22B-02.

### R22A2-06. Closure bounds and the two failure modes at the stop (proved; instances computed)

- Bound. With E = lcm(1, ..., 13,205) and f_path the size of the clean-power component used for the entry, R22A-06
  gives |W| ≤ 4E·f_path. If the seed-and-table certificate holds for T = Bad(W), then
  |V·W·V⁻¹| ≤ 4E·f_path + 4·Σ_Δ |T ∩ Δ|·(s_Δ + 2e_Δ + 2f_Δ); if V is known to exist only by Corollary A1 or the
  target-orbit test, |V·W·V⁻¹| ≤ 4E·f_path + 2·[C(177,120, n(Z)) − 1]. For a given W replace 4E·f_path by |W|. T is
  the complete Bad set, initially vacant cells that the transport path disturbs included.
- Failure criteria. N1 fails exactly when T holds an occupied G-fixed chamber; N2 fails exactly when some orbit has
  fewer holes than T has chambers there; over all orbits, N1 is the singleton case of N2. Passing both does not prove
  existence without the group certificate.
- Instances at Z. The four entry targets pass: b19 198 Bad chambers (15 occupied), b08 and b10 111 (20), b09 87
  (24). For the glued units every class-P landing W has the unit's own cell in Bad(W) (R22A-07), and that cell holds
  occupied SAFE-fixed chambers: b03 4, b04 5, b05 6, b06 27, b12 5, b15 6 and
  b18 4, an N1 obstruction to every such closure. There are also N2 deficits on orbits of size at least 2: for b05 and
  b15 the own cell meets orbit 7 (7,125 chambers, no hole) once; for b06 orbit 6 (7,126 chambers, 2 holes) 56 times,
  and eleven more orbits fall short. Since the own cell lies in every such Bad(W), no SAFE word of any length vacates
  it, whatever the search budget.
- Command: section 6 (GLUE rows).
- Uses: Proposition N, R22A-06, R22A-07, R22B-01, R22B-02.

Open after round 2: building practical V_i for the sequence of R22A2-04 and replaying it; the eight units it leaves
(b03, b04, b05, b06, b12, b15, b18, which the N1 and N2 failures above block for every class-P closure, and b17,
which no clean power moves); coverage of B′(2) at every state of 𝒞_2. Nothing here is formalized.

## 3. Line (b): the vacating word V

### R22B-01. Certified three-cycles give a polynomial bound (proved; conditional on the certificate)

- Objects: Z, its holes H, a target T, the SAFE-ALL group G(Z); the orbits Δ_1, ..., Δ_r of size at least 2 that meet
  T, in a fixed order, d_i = |Δ_i|, k_i = |T ∩ Δ_i|.
- Hypotheses (every word checked letter by letter against A(Z), every permutation evaluated exactly on Ω):
  - H1: N1 and N2 hold for T; every d_i ≥ 5.
  - H2 (seed): for each i, distinct a_i, b_i, c_i ∈ Δ_i and a word C_i of at most s_i letters that acts on Δ_i as the
    three-cycle (a_i b_i c_i) and fixes every Δ_j with j < i pointwise.
  - H3 (first table): for each x ∈ Δ_i \ {a_i}, a word P_x of at most e_i letters that fixes a_i and sends x to b_i.
  - H4 (second table): for each z ∈ Δ_i \ {a_i, b_i}, a word Q_z of at most f_i letters that fixes a_i and b_i and
    sends c_i to z.
- Statement. A procedure that reads only the current holes builds a SAFE-ALL word V with T ⊆ holes(Z·V) and
  |V| ≤ 2·Σ_i k_i·(s_i + 2e_i + 2f_i). If every s_i, e_i, f_i is at most L, then |V| ≤ 10·|T|·L, and |T| ≤ n(Z) by N1
  and N2.
- Proof (index i dropped).
  1. For distinct x, y ∈ Δ \ {a} put z = π_{P_x}(y); z ∉ {a, b} by injectivity. U = P_x·Q_z⁻¹ sends (a, x, y) to
     (a, b, c). The word U·C·U⁻¹ induces the three-cycle (a x y) on Δ, fixes every earlier orbit pointwise (C does,
     and U maps each orbit to itself) and has at most s + 2e + 2f letters.
  2. For a three-cycle (x y z) avoiding a: (a x y) followed by (a z x) is (x y z). So every three-cycle on Δ costs at
     most 2(s + 2e + 2f).
  3. Let t ∈ T ∩ Δ be occupied. By N2 and because t is not a hole, some hole z ∈ (H ∩ Δ) \ T exists. Choose r ∈ Δ \
     {z, t}. If r is a hole apply (z r t), otherwise (z t r). In both cases the holes of Δ change from H ∩ Δ to
     ((H ∩ Δ) \ {z}) ∪ {t} and every other chamber of Δ keeps its status. At most k such operations clear T ∩ Δ.
  4. Earlier orbits are fixed pointwise, so their cleared targets stay clear; later orbits can be permuted, but their
     hole counts do not change, so N2 still holds when their turn comes. Targets in singleton orbits are holes by N1
     and never move. Induction over the orbits gives the bound. Prefixes of the words may move earlier targets; the
     preservation holds at the boundaries of the completed three-cycles, and every prefix is admissible because every
     letter is a SAFE letter.
- Tables from short letters. If the graph on Δ \ {a} whose edges are the SAFE letters fixing a (with their inverses) is
  connected, a breadth-first tree gives e ≤ d − 2; likewise f ≤ d − 3 with letters fixing a and b. Then
  |V| ≤ 2·Σ_i k_i·(s_i + 4d_i − 10).
- Without tables. If every Δ_i has a Jordan certificate and a seed C_i, G(Z) contains Alt(Δ_i) acting on Δ_i alone
  (Theorem A), which is transitive on ordered triples when d_i ≥ 5. A breadth-first search over ordered triples finds a
  word u of at most d_i(d_i − 1)(d_i − 2) − 1 letters that carries the seed's triple to a wanted one, and u⁻¹·C_i·u is
  the wanted three-cycle. Then |V| ≤ Σ_i k_i·[s_i + 2(d_i(d_i − 1)(d_i − 2) − 1)].
- Counterexample (abstract): without the condition on earlier orbits the construction is unsound. The diagonal A5 on
  two copies of five points contains every three-cycle in each projection, but the holes {1, 2′} cannot cover the
  targets {1, 1′}: one permutation would have to send both 1 and 2 to 1.
- Uses: Proposition N, Theorem A.

### R22B-02. A bound in n alone whenever V exists (proved)

- Statement. Let N = 177,120. If some SAFE-ALL word vacates T at Z, one of at most C(N, n(Z)) − 1 ≤ N^{n(Z)} − 1
  letters does, and breadth-first search on hole sets (at most C(N, n) vertices, at most 6,600 letters per vertex)
  finds it from the state alone. Consequently no family of states with bounded n has shortest vacating words of
  unbounded length. With a certificate as in R22B-01 whose lengths are all at most L, |V| ≤ 10·n·L.
- Proof. SAFE-ALL words permute the n-element hole set and keep the off geometry; a shortest path in the graph of hole
  sets repeats no vertex.
- What it does not give: coverage, that is, a vacatable target for every state. A large number of SAFE grips does not
  supply it. Instance: at the depth 40 stop the 549 SAFE grips leave 1,237 singleton orbits and only 1,193 holes, so at
  least 44 SAFE-fixed chambers are occupied, and a target containing one of them has no V.
- Existence bound only.

### R22B-03. Cluster certificates and separated commutators (proved; instance computed)

- Cluster lemma. Let w be a SAFE-ALL word with permutation p, H the holes and T the target. If P ⊆ H ∩ T, Q ⊆ T \ P,
  p fixes P pointwise and p(X) = Q for a given X ⊆ H \ P, then every chamber of P ∪ Q is a hole after w, since the new
  holes p(H) contain p(P) ∪ p(X). The exact test for keeping P ∪ Q vacant is p⁻¹(P ∪ Q) ⊆ H.
- Separated overlap. Let a and b be permutations with moved supports A and B, I = A ∩ B, and c = a·b·a⁻¹·b⁻¹
  (chronological). The support of c lies in I ∪ a⁻¹(I) ∪ b⁻¹(I). If a(I) ∩ I = ∅ and b(I) ∩ I = ∅, the three sets are
  disjoint and c is the product over r ∈ I of the disjoint three-cycles (r, b⁻¹(r), a⁻¹(r)).
- Proof. A point outside A ∪ B is fixed. A point x ∈ A \ B is moved only if a(x) ∈ I, so x ∈ a⁻¹(I); a point of
  B \ A is moved only if b(x) ∈ I. For r ∈ I: a(r) ∈ A \ I lies outside B, so b fixes it and c(r) = b⁻¹(r); s = b⁻¹(r)
  lies in B \ I, outside A, so c(s) = a⁻¹(r); u = a⁻¹(r) lies in A \ I, outside B, and c(u) = r. Under separation
  a⁻¹(I) ⊆ A \ B and b⁻¹(I) ⊆ B \ A, so the three sets are disjoint.
- Instance at b00 (depth 40 stop, transport target of section 10 of the stage 12 README): the first commutator of the
  greedy search, [(113, 4), (178, 10), (113, 8), (178, 10)], is separated: 335 disjoint three-cycles, 65 target holes
  gained and none lost. The second, [(33, 8), (88, 5), (33, 4), (88, 2)], is not (overlap 1,810, 943 returns each way):
  it gains 16, loses 8 and moves 31 vacated target chambers. The greedy search counts net gains, so a greedy V must be
  checked as a whole. A successful greedy run with moves of 1, 4 or 6 letters has at most 6·|T \ H| letters, since
  every accepted move lowers the occupied target count; this bounds successful runs and decides nothing.
- Command: `scripts/round22/r22b03_clusters.py`.

### R22B-04. b00's original target can be vacated (computed hypotheses; proved consequence)

- Statement. At the depth 40 stop, Bad(W) of b00's transport word (`runs/lc13-k40/transport-b00-word.json`) has 111
  chambers, all occupied (digest `391e9473…`). N1 and N2 hold, and each of its 19 target orbits (sizes 1,440 to 7,200,
  holding 1 to 15 targets) has a Jordan certificate checked on all 177,120 chambers (words of 120 letters, primes 1,009
  to 5,783). By Corollary A1 a SAFE-ALL V with Bad(W) ⊆ holes(Z·V) exists. Breadth-first search on the joint images of
  the target finds one of at most ∏_i C(d_i, k_i) − 1 ≤ 7,200^111 − 1 letters (existence bound).
- Proof. Theorem A gives the independent alternating factors on the 19 orbits; Corollary A1 turns the counts into
  vacancy. A shortest path in the graph of target images is simple; its inverse vacates the target.
- The earlier failures for b00 (peeling stopped after 35 chambers, greedy bounded at 300 seconds) were not
  nonexistence. No V for b00 is constructed yet: the clean-power probe stops with 34 chambers occupied, and the
  bounded builder of the R22B-01 certificate completes two of the 19 orbits per run (E22-1, section 4). R22B-09 gives
  a construction of the certificate that terminates.
- Command: `scripts/round22/r22b04_jordan.py`; output `runs/round22/r22b04_jordan.txt`.
- Uses: Theorem A, Corollary A1.

### R22B-05. Tables of Schreier words always exist (proved)

- Objects: Z, the SAFE-ALL letters S = A(Z) (closed under inverses, O2), a target orbit Δ with d = |Δ| ≥ 5, distinct
  anchors a, b, c ∈ Δ. For a chamber x and a word w, x·w is the image of x under w.
- Hypotheses:
  - H1: G(Z) induces on Δ a group that contains Alt(Δ). A Jordan certificate of Δ decides it
    (`buffer.check_jordan`).
- Statement. For every x ∈ Δ \ {a} there is a SAFE word P_x of at most e letters that fixes a and sends x to b, and
  for every z ∈ Δ \ {a, b} a SAFE word Q_z of at most f letters that fixes a and b and sends c to z, with
  e ≤ (d − 2)(2d − 1) and f ≤ (d − 3)(2d − 3)(2d − 1). These are H3 and H4 of R22B-01, whose proof is unchanged; the
  table words need not fix the earlier orbits, since a conjugate of a seed fixes them whenever the seed does.
- Proof.
  1. A breadth-first tree on Δ from a over single SAFE letters gives words R_x with a·R_x = x, R_a empty and
     |R_x| ≤ d − 1 (Δ is an orbit, so its letter graph is connected).
  2. For x ∈ Δ and s ∈ S put h(x, s) = R_x·s·R_{x·s}⁻¹. It fixes a and has at most L1 = 2d − 1 letters. These words
     generate the stabilizer G_a: if s_1 ⋯ s_t fixes a and x_j = a·s_1 ⋯ s_j, the product of the h(x_{j−1}, s_j)
     telescopes to s_1 ⋯ s_t, because x_0 = x_t = a.
  3. By H1, G_a induces on Δ \ {a} a group containing Alt(d − 1), which is transitive. A breadth-first tree from b on
     Δ \ {a} over the words of step 2 gives B_x with b·B_x = x in at most d − 2 edges; P_x = B_x⁻¹, and
     e ≤ (d − 2)·L1.
  4. For x ∈ Δ \ {a} and h a word of step 2, j(x, h) = B_x·h·B_{x·h}⁻¹ fixes a and b, has at most L2 = 2e + L1
     letters, and these words generate G_{a,b} by the same telescoping. G_{a,b} induces a group containing
     Alt(d − 2), transitive on Δ \ {a, b} since d − 2 ≥ 3. A breadth-first tree from c gives Q_z in at most d − 3
     edges, so f ≤ (d − 3)·L2 = (d − 3)(2d − 3)(2d − 1).
- Sharper bounds at an instance use the expanded lengths: L1 ≤ 2·max|R_x| + 1 and L2 ≤ 2·max|B_x| + L1. Tree depth
  alone does not bound a path when its edges are words of different lengths.
- Without H1, building all these Schreier words and testing the two reachability graphs still decides H3 and H4 for
  the chosen anchors, exactly and in finitely many steps. A failure then concerns the full stabilizers, not, as in
  E22-1 (section 4), the graphs of single letters.
- Bound: with R22B-01, |V| ≤ 2·Σ_i k_i·(s_i + 2e_i + 2f_i) with e_i and f_i as above, polynomial in the d_i once the
  seeds are given.
- Uses: Schreier's lemma; the point stabilizer of Alt(d) is Alt(d − 1).

### R22B-06. Moving the anchors: a decision procedure, and no guarantee (proved; counterexample checked)

- Convention. With chronological words, g⁻¹·C·g acts on Δ as the three-cycle (a·g, b·g, c·g); g·C·g⁻¹ has the
  anchors a·g⁻¹, b·g⁻¹, c·g⁻¹. (The first version of section 4 gave the forward anchors for g·C·g⁻¹; corrected.)
- Selection test. A breadth-first search over the ordered pairs (a′, b′) in the orbit of (a, b) under single SAFE
  letters keeps a word g for each pair. At each pair it tests whether the graph on Δ \ {a′} of the letters that fix
  a′ is connected, and the graph on Δ \ {a′, b′} of the letters that fix a′ and b′; the second test does not depend on
  the third anchor. If both hold, g⁻¹·C·g with third anchor c·g repairs both single-letter tables. There are at most
  d(d − 1) pairs, so a passing pair has a word g of at most d(d − 1) − 1 letters, and exhausting the pair orbit decides
  whether any conjugate of C repairs the single-letter tables. A search bounded in radius decides that radius only.
- Statement (negative). Neither a seed nor a Jordan certificate guarantees that some anchor pair passes.
- Counterexample (checked by enumeration). On nine points let r = (0 1 2 3 4 5 6 7 8), t = (0 1 2), b = r·t, and take
  the letters S = {r, r⁻¹, b, b⁻¹}. Both r and b are nine-cycles, so every letter moves every point and every
  single-letter stabilizer graph has no edges, for every choice of anchors. Yet r⁻¹·b = t is a seed; the conjugates of
  t by powers of r are the three-cycles of consecutive points, which generate A9 (the group S generates has order
  181,440); and t·r⁻²·t·r² = (0 1 3 4 2) is a Jordan word, since 9/2 < 5 ≤ 9 − 3. Conjugates of single letters are
  again derangements, so longer conjugators around one letter do not help. The word tables of R22B-05 still exist.
- Instance: at b00, whether another anchor pair repairs a failed single-letter table was not computed; an isolated c
  or a few unreached chambers do not decide it.

### R22B-07. Seeds: an exact criterion and a fallback that always works (proved)

- Objects: G = G(Z) acting on Ω, N = |Ω| = 177,120; an orbit Δ with d ≥ 5; E the union of the orbits before Δ in the
  chosen order; K the subgroup of G that fixes E pointwise.
- Hypotheses:
  - H1: G induces on Δ a group that contains Alt(Δ).
- Statement. A seed for Δ (a SAFE word acting on Δ as a three-cycle and fixing E pointwise) exists if and only if K
  acts nontrivially on Δ. The following finite exact procedure decides it and, when it holds, gives a seed of at most
  s ≤ 2·(2N − 1)^{|E| + d − 3} letters (an existence bound).
- Proof. E is a union of orbits, so it is G-invariant and K is normal in G; hence the group K|Δ that K induces on Δ
  is normal in G|Δ, which is Alt(d) or Sym(d). For d ≥ 5 every nontrivial normal subgroup of either contains Alt(d)
  (the fact Theorem A uses), so a nontrivial K|Δ contains a three-cycle. Conversely a seed lies in K and acts
  nontrivially on Δ.
- Procedure. Take Schreier point stabilizers (R22B-05, step 2) successively for the points of E, keeping the words.
  If every resulting word acts trivially on Δ, no seed exists. Otherwise stabilize d − 3 further points of Δ; the
  remaining words act on the last three points and generate a group containing A3, so one of them acts there as a
  three-cycle, or two act as distinct transpositions and their product is a three-cycle. That word fixes E and the
  stabilized points of Δ, so it is a seed. Each stabilizer step multiplies the word length by at most 2N − 1, which
  gives the bound; the procedure needs no random search and no length cutoff.
- Global Jordan certificates (R22B-04) give Alt(Δ) acting on Δ alone (Theorem A), so K|Δ contains Alt(Δ) for every
  order of the orbits. At b00 a missing seed (orbits 67 and 105 in one E22-1 run) is therefore a failure of the
  bounded search, not of existence, and no reordering is needed.
- Counterexample to the global hypothesis: the diagonal A5 on two five-point orbits induces A5 on each orbit, but
  fixing either orbit pointwise forces the identity on the other, whichever orbit comes first.
- Uses: Theorem A; the normal subgroups of Alt(d) and Sym(d), d ≥ 5.

### R22B-08. A seed from a Jordan word and a letter of small support (proved)

- Objects: an orbit Δ with d chambers; the orbits before it are arbitrary.
- Hypotheses:
  - H1: a global Jordan certificate (w, p) for Δ: |w| = ℓ, p prime with d/2 < p ≤ d − 3, π_w has exactly one cycle of
    length divisible by p, a p-cycle inside Δ (checked on all of Ω, `buffer.check_jordan`).
  - H2: a SAFE word h of q letters whose moved support R on Δ has r chambers, with 2 ≤ r and 2r − 1 ≤ d. h may move
    the earlier orbits.
- Construction and proof.
  1. Let M be the least common multiple of the cycle lengths of π_w other than the p-cycle. p does not divide M, so
     A = w^M is one p-cycle inside Δ and fixes every other chamber; |A| = ℓM.
  2. A breadth-first search in the single-letter graph of Δ from the set supp(A) gives for each x ∈ Δ a word u_x
     that carries some point of supp(A) to x, of at most d − p letters (a shortest path from the set visits at most
     the d − p points outside it). The conjugates u_x⁻¹·A·u_x and their inverses are p-cycles inside Δ that fix every
     other chamber, of at most J = ℓM + 2(d − p) letters. Their supports cover Δ and meet pairwise (2p > d), so they
     generate a transitive group, primitive by the long-prime-cycle block argument of Theorem A, and containing
     Alt(Δ) by Jordan's theorem; the generators are even, so the group is Alt(Δ) on Δ and fixes every other chamber.
     At most d − p + 1 conjugates are needed.
  3. Choose R′ with |R′| = r and R ∩ R′ = {z}, possible since 2r − 1 ≤ d. Alt(Δ) is transitive on the r-subsets of
     Δ (extend a bijection between two subsets to a permutation and correct its parity inside the source subset), so a
     breadth-first search over r-subsets with the generators of step 2 finds U with R·U = R′ in at most C(d, r) − 1
     steps: |U| ≤ J·(C(d, r) − 1).
  4. Put h′ = U⁻¹·h·U and C = h·h′·h⁻¹·h′⁻¹. On Δ the supports of h and h′ meet exactly in z, so by R22B-03 C acts
     on Δ as the three-cycle (z, z·h′⁻¹, z·h⁻¹). U fixes every chamber outside Δ, and Δ is G-invariant, so h′ = h
     outside Δ and C is the identity there. C is a seed for every order of the orbits.
- Bound: s ≤ 4q + 4·[ℓM + 2(d − p)]·[C(d, r) − 1].
- The construction does not need h to be a seed or to respect earlier orbits, and it cancels h's action outside Δ.
  It relies on the Jordan theorem as used in Theorem A, not on a new formalization of it. No complete subset search
  was run at a large instance.
- Uses: Theorem A, Jordan's theorem, R22B-03.

### R22B-09. b00: all hypotheses hold; a construction bound (computed hypotheses; proved consequence)

- Statement. At the depth 40 stop (state `7e128820…`, target digest `391e9473…`, 111 chambers, all occupied, N1 and
  N2 hold) each of b00's 19 target orbits has the Jordan certificate of R22B-04 (H1 of R22B-08, rechecked on all
  177,120 chambers) and a single SAFE letter with support r on the orbit, 36 ≤ r ≤ 204 and 2r − 1 ≤ d (H2 of
  R22B-08). For example orbit 67 (d = 3,600) has p = 2,287 and r = 72 for letter (0, 1), and orbit 105 (d = 2,400)
  has p = 1,291 and r = 63 for letter (0, 2), so neither is an obstruction. With ℓ = 120, the computed M of each
  certificate, J = 120M + 2(d − p), s = 4 + 4J·[C(d, r) − 1], e = (d − 2)(2d − 1) and f = (d − 3)(2d − 3)(2d − 1),
  R22B-01, R22B-05 and R22B-08 give a certificate and a vacating word V with
  |V| ≤ B = 2·Σ k(s + 2e + 2f), a 539-digit integer printed exactly (B < 10^539).
- B is a construction bound, an existence bound in the sense of section 0: larger than the 7,200^111 − 1 of R22B-04,
  but it comes from a construction of the seed-and-table certificate that terminates. No complete certificate and no
  V for b00 was built.
- Command: `scripts/round22/r22b09_bound.py`; output `runs/round22/r22b09_bound.txt`.
- Uses: R22B-01, R22B-04, R22B-05, R22B-08.

### R22B-10. A checker for certificates with word-valued tables (specification; implemented, E22-1b)

- Inputs: the state, the target and their digests; the SAFE alphabet and the chamber orbits; for every target orbit
  of size at least 2 the Jordan word itself and its prime, not a trial number. Checks first: target chambers unique,
  N1, N2, each Jordan certificate on all of Ω.
- Order: decreasing orbit size, then least chamber. Per orbit a SAFE letter that meets H2 of R22B-08 and the seed of
  R22B-08, or R22B-07 when no letter meets H2; the completed seed is checked on its orbit and on every earlier orbit.
  Anchors a = the least chamber the seed moves, b = a·C, c = b·C; moving the anchors (R22B-06) is optional, since
  R22B-05 works for every triple.
- Tables: the Schreier transversals and word-valued trees of R22B-05. An edge names its chamber, its parent and its
  complete word. A P edge is checked as a completed word: it fixes a and sends child to parent; a Q edge fixes a and
  b and sends parent to child. Letters inside an edge word may move the anchors. Every required vertex has exactly one
  parent, there are no parent cycles, and every path ends at its root. Path cost is the sum of the expanded edge
  lengths, which gives e and f. A Q path is reversed by reversing the order of its edge words, not the letters inside
  them.
- Words may be stored as an acyclic straight-line program whose leaves are SAFE letters and whose nodes are
  concatenation, inverse and nonnegative integer powers. References are checked and acyclic, expanded lengths are
  computed with exact integers (never the node count), and u·v is evaluated as u then v.
- Final check: V evaluated on all 177,120 chambers, T ⊆ holes after V, and |V| ≤ 2·Σ k_i(s_i + 2e_i + 2f_i). An
  interrupted run reports an incomplete certificate, never nonexistence.
- Controls the checker must reject: an anchor moved by a completed edge word, the wrong conjugation direction, a
  reversed Q segment, a leaf that is not a SAFE letter, a cyclic reference, a false power length, a seed that moves an
  earlier orbit. The existing `check_r22b01.py` assumes single-letter edges and must be extended this way.
- Implementation (E22-1b, section 4): `scripts/r22b10_cert.py` builds the certificate and `scripts/check_r22b10.py`
  checks it without importing the builder; `scripts/test_r22b10.py` holds the controls above. One change from this
  specification: the checker evaluates V on every target orbit, not on all 177,120 chambers. Every SAFE letter keeps
  each orbit of the SAFE-ALL group, so every node of the program does too, and the occupancy of the targets after V
  is the same as on all chambers, collateral of earlier steps included.

## 4. The integrator's experiments in this round

- **Zero endpoint collateral at the depth 40 stop (measured, exact replay).** With points as owners, clean-power paths
  gave strict′ steps of 60, 84, 60, 72 and 36 letters for b00, b01, b11, b16 and b20 (1 to 3 powers), each replayed
  exactly: the unit lands, no lattice piece goes off, every other off piece keeps its exact pose. b01 and b11 use the
  same tetrahedron target, so only one of them can land this way (R22A-05).
- **The census with whole cells as owners, class 3 (measured, exact replay).** With the owners of R22A-02, seven of
  the 13 class-3 units have strict′ steps of one to three clean powers: b00 60, b01 84, b02 60, b07 48, b11 60, b13
  60 and b14 60 letters, each replayed exactly (the unit lands, no lattice piece goes off, every other off piece
  keeps its exact pose). b02, b07, b13 and b14 had failed with points as owners, as R22A-02 predicts. The other six
  are glued (R22A-07). The seven share four vacant targets (three triangle home cells, one tetrahedron home cell),
  so at most four land this way from this state (R22A-05).
- **The census, classes 2 and 4 (measured, exact replay).** b16, b20 and b21 (class 2) have strict′ steps of 72, 36
  and 84 letters; b16 and b21 go to one vacant triangle home cell and b20 to the other, so at most two of the three
  land this way. b17 is moved by no clean power (its cell is its home, Theorem C), and b18 is glued. b19 (class 2,
  tetrahedron) and b08, b09, b10 (class 4) have no vacant home cell of their kind; for each the census keeps an
  entry transport W into an occupied home cell (84, 96, 168 and 144 letters) whose exact replay lands the unit,
  takes exactly that cell's lattice content off (15, 20, 24 and 20 pieces) and keeps every other off piece at its
  exact pose. b08 and b10 enter the same home cell.
- **The closures of the four entry transports (R22A-06; measured, exact replay).** `tclose2.py` at the depth 40
  stop with W as the transport to close. N1 and N2 hold for all four Bad(W) (the 198, 111, 111 and 87 chambers
  that the words need). For b08, b10 and b09 the peeling certificate of Theorem B is complete (V of 63, 63 and 268
  letters, against its bounds 1,371, 1,371 and 1,186; `closures/<u>-log.txt`), and V·W·V⁻¹ is a strict′ step of
  222, 270 and 704 letters: an independent replay from the exported table (`stepchk.py`) lands 33, 34 and 58
  pieces, takes no lattice piece off and leaves every other off piece at its exact pose. For b19 the peeling stops at
  68 of 198 chambers and a 300-second greedy search leaves 13 occupied (`closures/b19-log.txt`); this rejects the
  two searches, not V.
- **b19's V exists (computed; Corollary A1).** b19's Bad(W) has 198 chambers, 15 of them occupied, and meets 20
  orbits of G(Z) of sizes 120 to 7,200. N1 and N2 hold, and a seeded random search finds a Jordan certificate on
  every one of the 20 (the orbit of 120 chambers with the prime 73), each checked on all chambers. By Corollary A1
  of `research/jumbling/theory/buffer-transport.md` a SAFE-ALL word V clears Bad(W), so V·W·V⁻¹ is a strict′ step
  for b19 (R22A-06), and by R22B-02 one of at most C(177,120, 1,193) − 1 letters exists (an existence bound). V has
  not been built. Command `scripts/round22/jordan_target.py` on `runs/round22/line-a/closures/b19-target.json`
  (section 6); output `closures/b19-jordan.txt`.
- **b02 closes (measured, exact replay).** The greedy search, bounded at 3,600 seconds, found an 18-letter V for b02's
  transport word; the step V·W·V⁻¹ has 48 letters and is strict′ by an independent replay from the exported table.
- **E22-1, a vacating word built from clean powers (measured, scratch probe; no V).** For b00's target: the greedy
  phase of `vacate2`, then, for each still occupied target t, a power σ^m of a commutator σ around t with m chosen by
  the Chinese remainder theorem so that σ^m fixes every cycle of σ through an already vacated target and carries a
  hole onto t; every word is checked on the full permutation. The greedy phase (16 letters) vacates 77 of the 111
  target chambers; for chamber 3392 no commutator around it has such a power, so the construction stops with 34
  occupied. A first probe for three-cycle seeds of the form [[x, y], z] found none. Neither result bears on R22B-04.
- **E22-1, the certificate of R22B-01 with single-letter tables (measured, exact; incomplete).** `r22b01_cert.py`
  (written by Codex, reviewed by the integrator) takes b00's 19 target orbits, largest first. For each it searches
  two- and three-letter SAFE commutators, random with a fixed seed and bounded by the orbit's share of 900 seconds,
  for a power that acts on the orbit as one three-cycle (a b c) and fixes every earlier orbit, of at most 20,000
  letters; it builds the tables H3 and H4 as breadth-first trees over single SAFE letters (fixing a; fixing a and b),
  and it clears the orbit's targets by the three-cycles of step 3 of the proof, each checked on the full chamber
  permutation. One run completed orbits 41 (k = 3, s = 30, e = 5, f = 6) and 64 (k = 1, s = 12, e = f = 5): V has
  318 letters against the bound 376. The acceptance run completed orbits 69 (k = 6, s = 30, e = f = 5) and 67
  (k = 3, s = 120, e = f = 5): 1,008 letters against 1,440. The first run's other orbits:

  | Least chamber | d | k | s | Missing |
  |---:|---:|---:|---:|:---|
  | 34 | 7,200 | 3 | 210 | Q: 1 chamber |
  | 66 | 7,200 | 3 | 60 | Q: 3 |
  | 69 | 7,200 | 6 | 30 | Q: 7 |
  | 70 | 7,200 | 9 | 294 | Q: 19 |
  | 74 | 7,200 | 12 | 210 | Q: 17 |
  | 82 | 7,200 | 9 | 1,140 | Q: 1 |
  | 83 | 7,200 | 12 | 840 | P: 11, Q: 22 |
  | 90 | 7,200 | 15 | 294 | Q: 7,197 (all) |
  | 106 | 7,200 | 9 | 210 | Q: 2 |
  | 65 | 7,198 | 6 | 60 | Q: 7 |
  | 33 | 7,197 | 3 | 150 | P: 4, Q: 7 |
  | 67 | 3,600 | 3 | none | no seed |
  | 81 | 3,600 | 6 | 60 | Q: 3,597 (all) |
  | 108 | 3,600 | 3 | 1,320 | Q: 3,597 (all) |
  | 105 | 2,400 | 4 | none | no seed |
  | 115 | 2,400 | 1 | 120 | Q: 18 |
  | 75 | 1,440 | 3 | 210 | Q: 1,437 (all) |

  "P: m" and "Q: m" count the chambers the tree of H3 or H4 does not reach: they lie in other components of the
  graph of the letters that fix a (P), or a and b (Q); "all" means that c is alone in its component, that is, every
  letter moving c moves a or b. Which orbits get a seed depends on the time share, so the two runs differ. The result rejects single-letter tables with these anchors,
  not V: H3 and H4 allow words (R22B-05), and a conjugate g⁻¹·C·g is again a seed, with anchors a·g, b·g, c·g
  (R22B-06).
- **E22-2, retained conjugation at the depth 100 stop (implemented; the full search is running).** The complete
  r = 1 search of R22D-01: 2,574 retained letters at the 234 clean grips, the E-B6 search at each prepared state with
  exact classification updates, and an exact replay of every kept word r·C·r⁻¹. An exact rule prunes a letter when no
  piece of a high unit moves and no inside or straddler set changes at a cap the search reads (1,309 letters), so
  1,265 prepared states are searched. The 22 letters of grips 1 and 61 give no strict word (`eb6r.py`,
  `check_eb6r.py`).
- **E22-1b, the certificate of R22B-10 (measured, exact; complete for b00 and b19).** `r22b10_cert.py` (written by
  Codex, reviewed by the integrator) takes the target orbits largest first. For each it tries two- and three-letter
  SAFE commutator seeds for min(8, 0.12·share) seconds (8 seconds here) and otherwise builds the seed by the Jordan
  route of R22B-08; the tables are Schreier trees whose edges are words (R22B-05), and every word is a node of one
  acyclic straight-line program with its exact integer length. `check_r22b10.py` recomputes the orbits, N1 and N2 from
  the state, checks every Jordan certificate on it, then the program, the seeds, both tables, every step as its stated
  three-cycle fixing the earlier orbits, the bound and the occupancy after V, and replays the steps in order; it
  passes both certificates (`runs/round22/e22-1b/`).

  | Unit | Targets | Orbits | Occupied (steps) | Seeds | s (digits) | e, f | Nodes | \|V\| | Bound | Build |
  |:---|---:|---:|---:|:---|:---|:---|---:|:---|:---|---:|
  | b00 | 111 | 19 | 111 | all Jordan | 122 to 165 | ≤ 22 | 14,018 | < 10^166 | < 10^166 | 254 s |
  | b19 | 198 | 20 | 15 | all Jordan | 121 to 165 | ≤ 22 | 12,688 | < 10^141 | < 10^166 | 244 s |

  So the construction of R22B-09 terminates in practice, far below its bound 10^539, and b19's V, until now an
  existence statement (R22A2-01), is built. The seeds carry the length: the tables have e, f ≤ 22 while every seed has
  more than 120 digits. No commutator seed turned up in the 8-second commutator phase, whereas E22-1, with longer
  shares, found commutator seeds of 12 to 1,320 letters for at least 18 of b00's 19 orbits over its two runs. Shorter
  seeds are the open question of line (b), round 3.

## 5. Line (d): height 4

### R22D-01. Retained conjugation (proved)

- Objects: a word A of retained letters admissible at Z, Y = Z·A, a word C admissible at Y, a unit U.
- Statement. C is a strict′ landing step at Y for the pieces of U (at their poses of Y) if and only if A·C·A⁻¹ is a
  strict′ landing step for U at Z; in the forward direction the suffix A⁻¹ is admissible automatically. A·C·A⁻¹
  returns every piece outside U exactly if and only if C does so at Y.
- Proof.
  1. A letter maps the poses inside its cap bijectively and keeps the cap, so its inverse letter undoes it pose by
     pose; the pose map of A⁻¹ is the inverse of that of A.
  2. Forward. After C every piece is landed (U, and possibly others), a lattice piece of Y (these are the lattice
     pieces of Z, since retained letters keep membership) or returned to its pose of Y. Apply A⁻¹ letter by letter.
     A returned piece retraces its trajectory under A, and A was admissible, so it does not straddle the cut of the
     next inverse letter; lattice pieces never straddle. So A⁻¹ is admissible, returned pieces end at their poses of
     Z, and landed pieces stay on the lattice.
  3. Backward. A·C is a prefix of an admissible word, so C is admissible at Y. A⁻¹ keeps lattice membership, so the
     lattice pieces after C are those at the end. By item 1 a piece ends at its pose of Z exactly when it had its pose
     of Y before A⁻¹.
- Caution. If C changes an off piece to another off pose, A⁻¹ need not be admissible; a lowering segment that ends at
  height 2 needs its own replay of A⁻¹.
- Exact update for one letter. For A = r = (c, e) with e ∈ K⁺: a piece inside cap c at Z is inside (straddles) cap x
  at Y exactly when it is inside (straddles) cap e⁻¹(x) at Z, since e permutes the caps; every other piece keeps its
  classification. So the tests of Lemma E6 at Y follow from the classifications at Z (`eb6r.py`, E22-2).
- Counts (instance). At the depth 100 stop the 234 clean grips give 2,574 retained letters. A prepared state has at
  most 600·80·59² four-letter commutators with distinct grips whose caps meet (600·81·59² with equal grips allowed), so
  at most 430,084,512,000 (435,460,568,400) candidates in all before pruning. A setup of unrestricted length can be
  shortened to at most 7,200^177,120 − 1 letters, since each piece stays in its K⁺ orbit (existence bound).
- E-B6's common-final-pose filter is a property of that search, not a hypothesis of B′.

### R22D-02. Six-letter commutators: the mask lemma (proved; computed)

- Statement. Write [g, W] chronologically as a, b, c, a⁻¹, c⁻¹, b⁻¹ (g = a, W = b·c). A piece of pose Q whose
  membership in the six caps, letter by letter, is m ∈ {0, 1}⁶ ends at F(m)·Q with
  F(m) = b^(−m6)·c^(−m5)·a^(−m4)·c^(m3)·b^(m2)·a^(m1). For [W, g] = a, b, c, b⁻¹, a⁻¹, c⁻¹ (W = a·b, g = c),
  F(m) = c^(−m6)·a^(−m5)·b^(−m4)·c^(m3)·b^(m2)·a^(m1). A vector m is feasible when at each of the six prefixes the
  transformed piece lies inside the next cap if m_i = 1 and outside if m_i = 0 (a straddle rejects it); for a feasible
  m the piece returns exactly when F(m) = I and lands when F(m)·Q ∈ K⁺.
- Proof. Induction over the six letters; Q is invertible, so F·Q = Q exactly when F = I.
- Computed classification (symbolic, letters generic). In each form 5 masks always return, 20 always move and 39 need
  the product test. First form, returning: 000000, 001010, 010001, 011011, 100100. Second form, returning: 000000,
  001001, 010100, 100010, 110110. Mask 111111 is not always moving: in the first form it returns exactly when a
  commutes with c·b.
- Pruning for exact return. The first cap is clean at Z; every straddler of the last cut belongs to U; every target
  piece lies whole in one of the three caps. If the graph of meeting caps is disconnected, the word reduces to the
  identity or a four-letter commutator, so new six-letter candidates have a connected cap graph: at most
  2·3·600·81²·59³ = 4,850,969,828,400 for both forms (2·35,400³ = 88,723,728,000,000 unpruned).
- No six-letter search was run.
- Command: `scripts/round22/r22d02_masks.py`.

### R22D-03. The endpoint contract of a lowering conjugate (proved)

- Objects: a = (c, D) admissible at Z with U whole in cap c at height 4 and D lowering every piece of U to height 2;
  Y = Z·a; C admissible at Y; T = Y·C; ε_p ∈ {0, 1} whether piece p lies inside c at T.
- Statement. a·C·a⁻¹ is admissible and strict′ for U if and only if (1) c is clean at T; (2) D^(−ε_p)·T_p ∈ K⁺ for every
  p in U or on the lattice at Z; (3) D^(−ε_p)·T_p is the pose of p at Z or lies in K⁺ for every other off piece of Z.
  These are finite exact checks by replay from Z.
- Proof. The prefix a·C is admissible by hypothesis; (1) is the admissibility of a⁻¹; a⁻¹ changes T_p to
  D^(−ε_p)·T_p, so (2) and (3) are the definition of strict′.
- Consequences.
  - If C lands U, U must end C outside c: D ∉ K⁺, so D⁻¹·k ∉ K⁺ for every k ∈ K⁺.
  - Sufficient: C lands U outside c and returns every other piece exactly to Y. Then (1) holds (returned pieces have
    their regions of Y, where c is clean, and lattice pieces never straddle) and every other piece retraces a.
  - Returning only the pieces that were inside c is not enough: a lattice piece from outside c can enter c during C
    and leave the lattice under a⁻¹.
  - For C = V·W·V⁻¹ meeting AU-03 and this contract, |a·C·a⁻¹| ≤ 2 + 2|V| + |W|; the toolkit bounds b_κ (720, 104,
    240, 192) apply only when their hypotheses hold for the actual free grips. A protected lowering segment S1 followed
    by a strict′ landing S2 of its height-2 target composes to a strict′ step of |S1| + |S2| letters.
  - A single lowering item of class P lands its target in a branch Q ≠ P (R21F-12), so the middle needs a second
    class.
- Uses: AU-03, R21F-12.

### R22D-04. Free grips after lowering at the depth 100 stop (computed)

- Statement (instance, state digest `4ca44008…`). All 124 high pieces lie in clean cap 61. Each of its 12 lowering
  items (15, 18, 22, 33, 52 to 59) takes all of them to height 2. After nine of them (all but 55, 57, 59) no piece lies
  in a free grip of its new class. After 55, 57 or 59 the 81 pieces of the first block have exactly one free grip
  (215, 110 or 21) and the 43 pieces of the second block none. So 93 of the 108 combinations of a unit and an item
  cannot land over the fixed free class alphabet, whatever SAFE preparation comes first. The lowering letter also
  splits units over class cells: 2, 1, 4, 3, 1, 2, 2, 1, 1 cells for the units 270, 391, 408, 409, 6780, 32115, 53620,
  53629, 61683, for every item.
- Proof of the obstruction. A piece outside every initially free grip of its class is fixed by every letter of that
  alphabet, so it stays outside forever; a SAFE preparation fixes every off piece. This excludes the AU-03 free-class
  construction, not words that move foreign off pieces.
- Command: `scripts/round22/r22d04_free_grips.py`; output `runs/round22/r22d04_free_grips.txt`.

### R22D-05. Nested-set vacancy for a two-level step (proved)

- Objects: a = (c, D) lowering a height-4 unit U to height 2; Y = Z·a with holes H; a clean grip d whose cap holds
  exactly U among the off pieces of Y; a jumble letter b at d that lands U; T the home chambers inside d; L ⊆ T the
  chambers where b lands U; Ω_c the home chambers inside c.
- Hypothesis: a SAFE-ALL word q at Y with π_q(T) ⊆ H and π_q(L) ∩ Ω_c = ∅.
- Statement. a·q⁻¹·b·q·a⁻¹ is admissible, lands U and returns every other piece exactly to its pose of Z.
- Proof. q⁻¹ vacates T and fixes the off pieces, so b moves exactly U and lands it on L. Then q returns every lattice
  piece of Y exactly and carries U, now on the lattice, to π_q(L), outside c. The off pieces other than U are at their
  poses of Y, so a⁻¹ is admissible and returns them to Z; it does not move U.
- Bound. Per SAFE orbit Δ put n_Δ = |Δ|, t_Δ = |T ∩ Δ|, l_Δ = |L ∩ Δ|, and Q = ∏_Δ C(n_Δ, t_Δ)·C(t_Δ, l_Δ). A shortest q
  has at most Q − 1 letters, the step at most 2Q + 1, and the search at most 6,600·Q letter tests (existence bound).
  The joint pair (T, L) must be searched; separate witnesses per orbit are not compatible in general. Necessary:
  |H ∩ Δ| ≥ t_Δ and |(H \ Ω_c) ∩ Δ| ≥ l_Δ for every Δ.
- Without control of π_q(L) the construction fails: a landed chamber in Ω_c leaves the lattice again under a⁻¹.

### R22D-06. Fixed occupied chambers rule out both whole-cap middles (computed; proved)

- Statement (instance at the depth 100 stop). After (61, 57), cap 110 holds exactly the 81-piece block among the off
  pieces; of its 3,097 home chambers 163 lie outside every SAFE cap and 102 of those are occupied (chamber 15142 is
  one). After (61, 59), cap 21 likewise; 896 chambers outside every SAFE cap, 421 occupied (chamber 121). So no SAFE-ALL
  word of any length vacates either cap, and no single-letter landing middle V·b·V⁻¹ closes at these two prepared
  states.
- Proof. A chamber outside every SAFE cap is fixed by every SAFE word, so an occupied one never becomes a hole. A
  jumble letter at d takes every lattice piece inside d off, and retained letters cannot put it back, so the middle
  needs all of Ω_d vacant.
- Not excluded: a smaller-Bad middle of several letters, a retained preparation that moves off pieces, or a repair
  outside AU-03.
- Command: `scripts/round22/r22d06_fixed_chambers.py`; output `runs/round22/r22d06_fixed_chambers.txt`.

### R22D-07. Unlocking (proved; Lemmas B and C conditional)

- Lemma A. A word that fixes every off piece at every prefix changes neither which cuts are clean nor which caps hold
  a given unit whole, since the off regions never change and lattice pieces never straddle. So SAFE-ALL preparation
  alone cannot unlock any of the 186 high units of the fixture end that have no clean containing cap.
- Lemma B. Whether some retained preparation makes a clean lowering cap for a named height-4 unit U is decided on at
  most 7,200^n records (n off pieces): a retained word keeps the identities of the off pieces and sends each off pose
  Q_p into {k·Q_p : k ∈ K⁺}; admissibility of the next retained letter and the goal (U whole in a clean cap of its
  current branch) depend only on these records. If a preparation exists, one of at most 7,200^n − 1 letters does
  (existence bound; at most 6,600·7,200^n letter tests). Conditions on holes downstream need the holes in the record.
- Lemma C. If an own-branch cap c holds U whole with straddler set S, and strict′ steps of at most b_S letters each
  land at least one remaining straddler and keep U exact, then c is a clean lowering cap after at most |S|·b_S
  letters: surviving off pieces keep their poses and landed pieces stay on the lattice under later strict′ steps, so
  no new straddler appears and S shrinks. The lowering coset (R21F-03) then gives twelve lowering items.
- Uses: R21F-03, Proposition F (`PROOFS-U.md`).

### R22D-08. A finite bound for a stated family (proved)

- Objects: N = 177,120, G = 7,200^N, F = 7,200^2,640; states of 𝒞_4 with n off pieces. The family: an admissible
  retained setup P, then a core, then P⁻¹. The core is a four-letter commutator, a six-letter commutator, or
  a·V·W·V⁻¹·a⁻¹ with a one lowering item, V over the fixed SAFE-ALL alphabet after a, W over the fixed free alphabet of
  the target's height-2 class meeting the off-piece and ghost checks of AU-03, and the final inverse passing R22D-03.
  The core must land the target and be strict′.
- Statement. Whether the family has a member at Z is a finite exact predicate; if it has one, it has one of at most
  L4(n) = 4G + F − 3 letters, and an enumeration of at most
  S4(n) = n·G·[6,600 + D4 + D6 + 7,200·(7,080·F + 6,600·G + F·G)] candidates and edges decides it, where
  D4 = 600²·59² and D6 = 2·35,400³ (existence bounds).
- Proof.
  1. Under retained setups each piece stays in its K⁺ orbit, so there are at most G complete states; replace P by a
     shortest path to its endpoint (at most G − 1 letters); R22D-01 keeps the inverse valid.
  2. A free-class word is determined, as an action, by its accumulated rotation on each of the 2,640 non-outer cells,
     so there are at most F actions and a shortest representative has at most F − 1 letters; equal actions give equal
     endpoints, and freeness keeps every prefix admissible.
  3. A SAFE word acts on each lattice chamber by a retained rotation; recording the pose of one ghost per chamber
     gives at most G actions, and a record determines the action on every lattice pose, newly landed ones included.
     So V can be replaced by a word of at most G − 1 letters with the same lattice action; after W the remaining off
     pieces are at their original poses (AU-03), so the replaced inverse is admissible with the same endpoint.
  4. 2(G − 1) + 2 + 2(G − 1) + (F − 1) = 4G + F − 3; the commutator cores are shorter.
- What stays open is coverage: that every state of 𝒞_4 has a member of the family (or of a larger catalogue). The
  computed obstructions R22D-04 and R22D-06 reject particular direct instances, not the family.
- Uses: R22D-01, R22D-03, AU-03.

## 6. Commands

From the repository root, with Python 3.13 and NumPy 2.5; `S=research/jumbling/restoration/12-low-height/scripts`.
Each script prints the output recorded under `runs/round22/` (the timing lines aside):

    python3 -I -B $S/round22/r22a08_vacancy.py          # R22A-08 and the instance of R22A-07; about 1.5 minutes
    python3 -I -B $S/round22/r22b03_clusters.py         # R22B-03; about 4 minutes
    python3 -I -B $S/round22/r22b04_jordan.py           # R22B-04: N1, N2 and the 19 Jordan certificates; about 1.5 minutes
    python3 -I -B $S/round22/r22b09_bound.py            # R22B-09: the hypotheses of R22B-08 and the bound; about 1 minute
    python3 -I -B $S/round22/r22d02_masks.py            # R22D-02: the 64 masks of both forms; under a second
    python3 -I -B $S/round22/r22d04_free_grips.py       # R22D-04; about 1.5 minutes
    python3 -I -B $S/round22/r22d06_fixed_chambers.py   # R22D-06; about 3 minutes

The scripts are the senior reviewer's commands, kept as written apart from a header comment; the integrator re-ran
each one and got the outputs the review reports.

The integrator's census and closures of section 4, with `k40.json` the depth 40 stop as a pose table (written by
`$S/export_table.py`, digest `7e128820…`), `A=research/jumbling/restoration/12-low-height/runs/round22/line-a`,
`G=research/jumbling/restoration/09-general-method/scripts` and `u` one of `b19`, `b08`, `b10`, `b09`:

    python3 -I -B $S/round22/transport_census.py k40.json <out> $A/units/eb5-k40-k00-b{00,01,02,03,04,05,06,07,11,12,13,14,15}.json
                                                        # class 3; about 47 minutes
    python3 -I -B $S/round22/transport_census.py k40.json <out> $A/units/eb5-k40-k00-b{16,17,18,19,20,21,08,09,10}.json
                                                        # classes 2 and 4; about 66 minutes
    python3 -I $G/gm/tclose2.py research/jumbling/theory $G/echain k40.json $A/units/eb5-k40-k00-$u.json \
        $A/closures/$u-core.json <dir> --greedy-seconds 300                       # 4 to 10 minutes
    python3 -I -B $S/round22/stepchk.py k40.json $A/units/eb5-k40-k00-$u.json $A/closures/$u-step.json <out>

`$u-core.json` is `{"P": [], "Q": W}` with W the census's entry transport word; `$u-step.json` is the closure's
`word-full.json`, `$u-result.json` its `result.json`, `$u-log.txt` its log (the certificate and its bound, or where
the peeling and the greedy search stopped) and `$u-replay.json` the record of the independent replay.
b19's target file holds `ghost.badW` of its closure (`pre.json`), with its digest; then

    python3 -I -B $S/round22/jordan_target.py $A/closures/b19-target.json     # N1, N2 and 20 Jordan certificates; about 2 minutes

Round 2 of line (a) (R22A2-01 to R22A2-06), the senior reviewer's computation kept as written apart from a header
comment; it replays the fourteen middles and writes no file:

    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 -I -B $S/round22/r22a2_04_sequence.py   # about 75 CPU seconds

Line (c) (section 7): the senior reviewer's computations and the solver's (`r22cf02_shadow.py`), each kept as written
apart from a header comment and re-run by the integrator with the outputs the analyses report;
`L=research/jumbling/restoration/12-low-height/runs/round22/line-c`:

    python3 -I -B $S/round22/r22c02_locks.py            # R22C-02: locked occupied chambers at the depth 40 stop; about 1 minute
    python3 -I -B $S/round22/r22c03_locks60.py          # R22C-03: the same at the depth 60 stop; about 8 minutes
    python3 -I -B $S/round22/r22c04_vectors.py          # R22C-04: type vectors of the home cells; about 20 seconds
    python3 -I -B $S/round22/r22c04_orbits.py           # R22C-04: K⁺ orbits of the home cells; about 15 seconds
    python3 -I -B $S/round22/r22c04_b06.py              # R22C-04: b06's pieces and type vector; about 1 minute
    python3 -I -B $S/round22/x22c04_s61c.py             # X22C-04: the s61c state; about 20 seconds
    python3 -I -B $S/round22/x22c07_movers.py           # X22C-07: a mover for each blocking group, from the records
    python3 -I -B $S/round22/r22cf02_shadow.py \
        research/jumbling/restoration/11-scramble-census/runs/round2/ia-k40-s7101.json \
        research/jumbling/restoration/12-low-height/runs/screen-k40.json $L/shadow/shadow.json   # R22CF-02; about 2 minutes
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 -I -B $S/round22/r22c2_02_blockers.py   # R22C2-02; about 20 minutes
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 -I -B $S/round22/r22c2_05_vertex.py     # R22C2-05; about 2 minutes
    python3 -I -B $S/round22/x22c09_r2_ready.py <unit>  # X22C-09, experiment C, units 27, 36, 44 and 59; about 8 minutes each

Experiment A continued (R22C2-04), with `k40.json` as above, `G` as above and `za.json` the state after b12's step:

    python3 -I -B $S/round22/stepchk.py k40.json $L/units/eb5-k40-k00-b12.json \
        research/jumbling/restoration/12-low-height/runs/round22/t1prime-k40/steps/u12.json <out> za.json
    python3 -I -B $G/gm/tclose2.py research/jumbling/theory $G/echain za.json $L/units/eb5-k40-k00-b14.json \
        $L/b14-after-b12/b14-core.json <dir> --greedy-seconds 1800                  # about 35 minutes
    python3 -I -B $S/round22/jordan_target.py $L/b14-after-b12/b14-target.json za.json   # about 1 minute

`b14-core.json` is `{"P": [], "Q": W}` with W b14's census word (`$L/w14-census.json`); `b14-target.json` holds
`ghost.badW` of the closure (`pre.json`) with its digest; `b14-log.txt` and `b14-result.json` are the closure's log and
result, `b12-replay.json` the replay record of b12's step.

## 7. Line (c): T2 and T3 (joint attack)

Line (c) was a joint attack. The senior reviewer and the solver each wrote a sealed analysis at the highest effort
(R22C-01 to R22C-07 and R22CF-01 to R22CF-07), the senior reviewer then cross-examined the solver's findings against
its own (X22C-01 to X22C-09), and after that the solver stopped (owner, 18 October 2026). A second round by the senior
reviewer alone (R22C2-01 to R22C2-06) followed the integrator's experiments. Each heading below names the findings a
lemma comes from and its status after the cross-examination; the integrator re-ran every computation cited here
(section 6). Lemma letters (G, T, ...) are those of the analyses.

Further objects for this section:
- **O9, blocks, locks and shadows.** A *rigid block* B is a set of off pieces that every admissible letter moves
  together (`blocks.Blocks`; Lemma 50 of `buffer-transport.md`). I_B is the set of grips whose cap holds every piece
  of B and S_B the set of grips whose cut a piece of B straddles; caps(x) is the set of grips whose cap contains the
  chamber x. A home chamber x is *locked* to B when I_B ⊆ caps(x) ⊆ I_B ∪ S_B (lock lemma,
  `buffer-transport.md` section 5), and *shadowed* by B when every cap containing x holds a piece of B or is
  straddled by a piece of B. Locked implies shadowed. Under an admissible retained word B moves by one multiplier k_B ∈ K⁺, and the lock lemma says the content
  (lattice piece or hole) that was in a locked chamber x is afterwards in k_B(x).
- **O10, types.** The 36 chamber types are the K⁺ orbits of chambers (`regions.orbit_of`). A lattice piece of home
  chamber h occupies a chamber of h's type, and SAFE-ALL words keep the number of holes of each type. off_t(Z) is the
  number of off pieces whose home chamber has type t; it equals the number of holes of type t.
- **O11, creators and partners.** A *creator* D for a cell C″ at Z is an admissible word that ends with every off piece
  of Z at its exact pose and with the lattice content of C″ off the lattice; that content is the *partner*.

### R22C-G. Lemma G: one rotation per cell (proved; R22CF-01, X22C-01)

- Objects: Z, a class P, a word W over the free class-P letters of Z (H1 of R22A-01).
- Statement.
  1. For every class-P cell C there is g_C ∈ G_P such that every piece at a point (C, m) of Z is at (g_C C, g_C m)
     after W, with pose g_C·γ, where γ is its pose at Z.
  2. If W lands a class-P off content U at (C, ℓ), ℓ ≠ 0, then g_C sends label 0 to a nonzero label, every lattice
     piece of Z in Ω_C is off after W, and these pieces form one class-P content at (g_C C, g_C 0). In particular
     Bad(W) contains every occupied chamber of Ω_C, whether W lands U in its own cell or in another one.
  3. A content in a vertex cell {v} is in {v} after every prefix of W.
- Proof.
  1. Induction over the letters (O8): a letter (c, g) at a free grip rotates every piece inside its cap by g, the cap
     of a grip of P contains exactly the cells that have c as a cap, and a class-P piece never straddles a cut of P.
  2. A pose h·k with h ∈ G_P and k ∈ K⁺ lies in K⁺ exactly when h ∈ H_P, that is, when its label is 0. U lands, so
     g_C·ℓ = 0; the label action of g_C is a permutation, so g_C·0 ≠ 0, and a lattice piece of C with pose k ends with
     pose g_C·k of nonzero label.
  3. A letter at a grip c ≠ v acts only on cells that have c as a cap, and v is the only cap of {v}; a letter at v
     fixes the pole v, so it maps {v} to itself.
- Consequences. R22A-07 is the case of one lattice content in the unit's own cell. The census rows "glued" (stage 12
  section 11) are instances of item 2, not search limits. A vertex content lands in place only (LC-12), so every
  landing word of b06 turns the cell {91}, and its 375 lattice pieces go off.
- Uses: O7, O8, LC-12.

### R22C-T. Locked chambers and Lemma T (proved as scoped; R22CF-02, R22C-01, X22C-02)

- Objects: Z, a rigid block B, a class-P unit U ⊆ B in the cell C, a chamber x ∈ Ω_C.
- Hypotheses: H1, x is occupied at Z; H2, x is locked to B (O9). Both are decided by `blocks.Blocks`.
- Statement.
  1. (Lemma T) Let w = A1·W·A2 with A1 and A2 admissible retained words and W a word over free letters of the class
     P′ of U after A1 that lands the pieces of U. Then the lattice piece q that occupied x at Z is off the lattice
     after w, so w is not a strict′ step for U.
  2. (Creators, R22C-01) The same holds when finitely many creators are inserted before W, with admissible retained
     words between them, each creator a word over free letters of a single class that ends with every off piece of Z
     at its exact pose.
  3. (Persistence) The content of x relative to B (the lattice piece or hole that the lock lemma carries to k_B(x)) is
     unchanged by every admissible retained word, every SAFE-ALL word and every word over free letters of one class
     that returns B to its exact pose.
- Proof.
  1. By the lock lemma, after A1 the piece q is in k(x) with k = k_B ∈ K⁺. The element k maps the class-P cell C to
     a class-P′ cell kC, Ω_C onto Ω_{kC}, and the pieces of U into kC; so k(x) is an occupied chamber of the home cell
     of the moved unit. By R22C-G item 2, W takes every lattice piece of Ω_{kC} off, q among them. A2 is retained and
     keeps lattice membership (O2).
  2. Induction over the inserted words. A creator of U's current class gives the pieces of U and the content of the
     locked chamber one rotation (R22C-G item 1); it keeps a piece of U at its exact pose, so that rotation is the
     identity and the content returns. A creator of another class uses free grips of that class, whose caps hold no
     piece of U (a free cap holds off pieces of its own class only) and hence, B being rigid and the cap admissible,
     no piece of B; an admissible cap containing the locked chamber k(x) is not in k(S_B), hence in k(I_B), and holds
     B; so no letter of the creator moves that content. Retained phases are covered by the lock lemma.
  3. Retained and SAFE-ALL words: lock lemma (a SAFE-ALL letter never moves B). A one-class word that returns B
     exactly: as in item 2.
- Not proved (X22C-02): that the content of a locked chamber persists along an arbitrary strict′ step of another unit.
  Such a step may move off pieces in between; strict′ fixes lattice membership at its end, not chamber occupancy.
- Instances (computed; `r22c02_locks.py`, `r22c03_locks60.py`). At the depth 40 stop the occupied chambers of the
  unit's own home cell locked to its block number 3 for b03, 5 for b04, 6 for b05, 7 for b06, 5 for b15 and 2 for b18;
  b12 has none, and b17's 13 locked chambers are all vacant. At the depth 60 stop 16 units have such chambers, among
  them 130 (6) and 180 (5), which form one blocking group of the structural order (`order-k60.json`). So no retained
  conjugation of any length followed by one own-class landing core lands b03, b04, b05, b06, b15 or b18 at the depth
  40 stop, or 130 or 180 at the depth 60 stop (R22C-02, R22C-03), although 130 and 180 each have a one-letter
  readiness witness; readiness and vacancy are different conditions.
- Uses: lock lemma, R22C-G, O2.

### R22C-S. The Shadow lemma (proved; R22CF-02, X22C-02)

- Objects: Z, a rigid block B, a home chamber x.
- Statement.
  1. If x is shadowed by B, then x lies in no SAFE-ALL cap at any state where B has its pose of Z. So N1 fails for
     every target that contains an occupied chamber shadowed by B, at every such state.
  2. Let E be a retained element at an admissible grip g whose cap holds B, and Y = Z·(g, E). Then E(x) is shadowed
     by E(B) at Y exactly when x is shadowed by B at Z; and E(x) is occupied at Y exactly when x is occupied at Z (x
     in the cap of g) or E(x) is occupied at Z (x outside it).
  3. A retained letter at an admissible grip whose cap does not hold B does not move any chamber shadowed by B.
- Proof.
  1. A SAFE-ALL cap holds no off piece and no piece straddles its cut; every cap of x holds or is straddled by a piece
     of B.
  2. E permutes the caps, cuts and chambers, so caps(E x) = E caps(x), and E(B) is inside or straddles E(c) at Y
     exactly when B is inside or straddles c at Z. The letter moves the content of x to E(x) when x is in the cap of g
     and leaves the content of E(x) in place otherwise.
  3. If the cap contains a shadowed chamber x, it is a cap of x, so it holds B or B straddles its cut. An admissible
     cap is straddled by nothing, and a cap that holds a piece of a rigid block holds the whole block.
- Consequence. Item 2 reads the number of occupied shadowed chambers in the target after one retained letter (r = 1)
  from Z; item 1 says that each of them violates N1 at the prepared state.
- Instances (computed; `r22cf02_shadow.py`). At the depth 40 stop, for the seven units whose own cell fails N1 (fixed
  occupied, shadowed occupied, locked occupied, and the least number of shadowed occupied chambers in the target after
  one retained letter at an admissible cap holding the block): b03 4/4/3/4, b04 5/5/5/5, b05 6/6/6/6, b06 27/27/7/27,
  b12 5/5/0/5, b15 6/6/5/6 and b18 4/4/2/4. Every one of the 57 fixed occupied chambers is shadowed by the unit's own
  block, so no retained letter (and, for the units with locks, no retained word of any length) makes their own cells
  vacatable. The 600 chambers of type 0 lie in one cap each and are fixed by its 11 retained letters.
- Uses: O5, O9.

### R22C-M. Partners do not repair N1 (proved; R22CF-03, X22C-03)

- Objects: Z, a target T ⊆ Ω, a word D admissible at Z.
- Hypotheses: H1, every off piece of Z is off at Z·D with its exact pose of Z; H2, a chamber x ∈ T is fixed by G(Z)
  and occupied at Z·D.
- Statement. x is fixed by G(Z·D), so N1 fails for T at Z·D and no SAFE-ALL word at Z·D vacates T (Proposition N).
- Proof. By H1, Ξ(Z) ≤ Ξ(Z·D) in the order of Lemma M, so A(Z·D) ⊆ A(Z) by (M1) and Fix(G(Z)) ⊆ Fix(G(Z·D)).
- H2 holds for every creator of U's class that keeps U at its exact pose (R22C-G item 1: the rotation of U's cell is
  the identity, so the lattice pieces of Ω_C return to their chambers); for a creator of another class it is one exact
  check at Z·D. So adding partners never repairs an N1 failure at an occupied fixed chamber; partners can repair the
  counts (R22C-V).
- Uses: Lemma M (M1), Proposition N, R22C-G.

### R22C-V. Type vectors and counts (proved; computed; R22C-04, R22CF-03, X22C-03, X22C-04)

- Statement.
  1. (Computed; `r22c04_vectors.py`, `r22c04_orbits.py`.) All home cells of one kind have the same type vector, in
     every class: 385 chambers for a vertex cell, 86 for an edge cell, 111 for a triangle cell and 87 for a
     tetrahedron cell, with the vectors printed by the script (types 6 and 0 to 5 occur in vertex cells only). K⁺ acts
     transitively on the home cells of each kind (600, 3,600, 2,400 and 600 cells over the five classes).
  2. If some class-P word lands a class-P content U of cell C, then |U_t| ≤ |Ω_C|_t for every type t.
  3. (Creators, R22C-04.) A creator whose ghosts send exactly the lattice contents of r fully occupied home cells of
     kind κ off, and no other ghost, adds r·e_κ to the hole vector, e_κ the type vector of item 1. For a middle W with
     Bad vector b that lands U and partners p, u + p ≤ b and b ≤ h + r·e_κ are necessary (u the type vector of U, h
     the holes); they do not give simultaneous vacancy.
  4. (T1′ counts, X22C-05.) Let W be a word over free class-P letters of Z that lands U. A SAFE-ALL word that vacates
     Bad(W⁻¹) at Z·W exists only if |U_t| ≤ off_t(Z) − |holes(Z) ∩ Bad(W)|_t for every type t. When Bad(W) = Ω_C this
     reads |U_t| ≤ off_t(Z) − h^in_t, h^in_t the holes of type t in Ω_C, and it holds automatically when Ω_C has no
     hole.
- Proof.
  2. The landed pieces occupy distinct chambers of a home cell C′ of the kind of C (class-P words keep kinds, R22A-05),
     each of the type of its home chamber (O10); C′ and C have the same type vector by item 1.
  3. Count the pieces taken off type by type.
  4. A SAFE-ALL word keeps the number of holes of each type (O10), so it vacates T = Bad(W⁻¹) only if the holes of
     type t at Z·W number at least |T|_t. At Z·W the off pieces are those of Z without U, together with the displaced
     lattice pieces Q = the lattice pieces of Z in Bad(W) (R22C-G), so holes_t(Z·W) = off_t(Z) − |U_t| + |Bad(W)|_t −
     |holes(Z) ∩ Bad(W)|_t. Over free class-P letters Bad(W) and Bad(W⁻¹) have equal type counts: W permutes the
     points kind by kind, and the label-0 home points that W sends to a nonzero label are as many, in each kind, as
     those that W⁻¹ sends to a nonzero label (R22A-01 item 1).
- Instances (computed). b06 at the depth 40 stop (`r22c04_b06.py`, `r22c2_05_vertex.py`): u = {6: 2, 8: 1, 9: 1, 10: 1,
  11: 1, 12: 1, 16: 1, 18: 1, 20: 1}; its cell {91} has 385 chambers, 375 lattice pieces and 10 holes, with 1,183 holes
  outside, and item 4 fails by 2, both in type 6. The edge vector has no type 6, so no middle with Bad vector 2e
  (M1-13) can land b06, however many edge partners are created. The s61c state (`x22c04_s61c.py`, X22C-04): the
  carrier's class-0 cell {30, 145} has 86 chambers, 68 occupied, 18 holes inside and 43 outside, so item 4 fails by 18
  without a partner and the necessary count 43 + 86 ≥ 61 holds with one edge partner.
- Uses: R22A-01, R22A-05, R22C-G, O10.

### R22C-T1′. Theorem T1′: land, vacate the landing chambers, unwind (proved; computed; R22CF-04, X22C-05)

- Objects: Z, a class P, a class-P unit U that is the whole content of its cell C at a label ℓ ≠ 0, a word W, Z1 = Z·W,
  T = Bad(W⁻¹) computed at Z1 (the home chambers whose lattice ghost W⁻¹ takes off; `tclose2.py`).
- Hypotheses: H1, every letter of W is at a grip of P that is free at Z; H2, every piece of U is on the lattice at
  Z1; H3, every off piece of Z outside U is at its exact pose at Z1; H4, a SAFE-ALL word V′ of Z1 with T ⊆
  holes(Z1·V′).
- Statement. w = W·V′·W⁻¹·V′⁻¹ is admissible at Z and is a strict step for U (U on the lattice, every lattice piece of
  Z on the lattice, every other off piece of Z at its exact pose); so is W·V′·W⁻¹. |w| = 2|W| + 2|V′|.
- Proof.
  1. After W: by H2 and H3, the off pieces of Z1 are those of Z without U, at their poses of Z, and the displaced
     lattice pieces Q (possibly several contents; R22C-G).
  2. V′ is SAFE-ALL at Z1, so it is admissible at every prefix, moves lattice pieces only, by elements of K⁺, and fixes
     every off piece (Lemma M (M2)). At Z2 = Z1·V′ the off pieces are those of Z1 at the same poses and T is vacant.
     The pieces of U occupy chambers of T at Z1 (W⁻¹ would take them off again), so after V′ they lie outside T.
  3. W⁻¹ at Z2. Its caps are those of W. They hold no foreign piece (H1, H3 and O8), the class-P off pieces they hold
     are those of Z1 at the same poses, and lattice pieces never straddle a cut; so every letter is admissible and
     every off piece of Z2 retraces, in reverse, the path W gave it: Q returns to its chambers of Z on the lattice, and
     every other off piece returns to its pose of Z (it had that pose at Z1 by H3). A lattice piece of Z2 outside T
     ends on the lattice by the definition of T, and T is vacant at Z2; so U and every other lattice piece of Z2 end
     on the lattice.
  4. V′⁻¹. The off pieces of Z3 = Z2·W⁻¹ are those of Z without U, at their poses of Z, a subset of the off pieces of
     Z1 at the same poses; by Lemma M (M1) every letter of V′ is SAFE-ALL at Z3, so V′⁻¹ is admissible, moves lattice
     pieces only and keeps them on the lattice.
- Why it escapes Lemma T. W turns the cell once; the occupants of locked chambers leave with Q and come back with the
  second turn W⁻¹. What V′ must vacate is T at Z1, not Ω_C at Z.
- Counts: R22C-V item 4 is the necessary per-type condition for H4.
- Bound. |w| ≤ 2·b_κ + 2|V′| with b_κ = 720, 104, 240 or 192 (Theorem C, LC-12, LC-13) when their free-grip hypotheses
  hold, and |V′| bounded by a peeling certificate at Z1 (Theorem B) or by M1-17.
- Instances (computed; `runs/round22/t1prime-k40/`). At the depth 40 stop the integrator's replay finds strict steps of
  160 letters for b03, 942 for b04, 1,158 for b12 and 160 for b18, the units recomputed from the state. b03, b04 and
  b18 have occupied locked chambers, so for them no single-turn template exists (R22C-T); b12 has none. Negative
  instances: for b05 and b15 at the depth 40 stop, and for 130 and 180 at the depth 60 stop after one readiness letter,
  N1 fails for T at Z1 (48 of 61, 24 of 25, 48 of 61 and 32 of 56 landing chambers occupied and fixed).
- Uses: R22C-G, R22C-V, Lemma M (M1), (M2), O6, O8.

### R22C-T2′. One partner and T1′ (proved; conditional on its certificates; R22CF-03, X22C-03)

- Objects: Z, U as in R22C-T1′, a lattice-only home cell C″ of U's kind with free grips, a creator D for C″ (O11) over
  free class-P letters, a word W over the free class-P letters of Z·D.
- Hypotheses: H1, D passes the exact filter at Z (every off piece of Z exact, the lattice content of C″ off); H2, W·V′
  ·W⁻¹·V′⁻¹ satisfies the hypotheses of R22C-T1′ at Z·D; H3, a SAFE-ALL word V2 at S = Z·D·W·V′·W⁻¹·V′⁻¹ vacates
  Bad(D⁻¹) at S.
- Statement. D·W·V′·W⁻¹·V′⁻¹·V2·D⁻¹·V2⁻¹ is a strict step for U at Z, of 2|D| + 2|W| + 2|V′| + 2|V2| letters.
- Proof. The T1′ segment is strict at Z·D: it lands U and keeps every off piece of Z·D, the partner included, at its
  exact pose. V2 fixes the off pieces and vacates Bad(D⁻¹), so D⁻¹, applied as the inverse of D (its caps hold the same
  off pieces at the same poses as at the end of D), returns the partner to the lattice and every other off piece of Z
  to its exact pose, and takes no lattice piece off. V2⁻¹ is SAFE-ALL by Lemma M (M1).
- The abbreviated form D·T1′·D⁻¹ lacks V2 and is not covered (X22C-08).
- Open: whether one partner suffices at b06 (its middle needs type-6 Bad chambers, R22C-V) or at s61c.
- Uses: R22C-T1′, Lemma M.

### R22C2-01. The intrinsic Q-shadow (proved)

- Objects: a state Y, a class-P off content U in the cell C, Q0 ⊆ the lattice content of C, a rotation g0 ∈ G_P that
  lands U (it need not come from a word: when U is one frame domain, the inverse of the pose of one piece of U will do).
- Hypothesis: H1, a chamber x occupied by a piece of g0·U such that every retained letter moving x is at a grip whose
  cap contains, or whose cut is straddled by, a piece of g0·Q0 (U and Q0 rotated by g0, all else as at Y).
- Statement. No word W that gives U and Q0 one common rotation (R22C-G) and lands U has a T1′ closure
  W·V′·W⁻¹·V′⁻¹ with V′ SAFE-ALL at Y·W, whatever its length.
- Proof. Let g be W's rotation and γ the pose of a piece of U. Both g·γ and g0·γ lie in K⁺, so k = g·g0⁻¹ =
  (g·γ)(g0·γ)⁻¹ ∈ K⁺ and (g·U, g·Q0) = k·(g0·U, g0·Q0). K⁺ permutes chambers, caps and retained letters and keeps the
  inside and straddle relations, so the chamber k(x) is occupied by a piece of U at Y·W and every retained letter that
  moves it is at a grip blocked by a piece of g·Q0. These pieces are off at Y·W (R22C-G), so no SAFE-ALL letter moves
  k(x). And k(x) ∈ Bad(W⁻¹), since W⁻¹ takes its occupant from U off again. N1 fails, so V′ does not exist.
- Extending this across retained preparations needs a persistent part of Q0 (such as chambers locked to U's block);
  across arbitrary strict′ predecessor steps it is not justified.
- Instances (computed; `r22c2_02_blockers.py`, `r22c2_05_vertex.py`). In the three failed T1′ closures (b15 at the
  depth 40 stop after b12's step and b14's census word; 130 and 180 at the depth 60 stop after one readiness letter)
  no fixed landing chamber is fixed by Q alone, and for b06 none of its twelve landing rotations has a chamber fixed by
  Q alone. These failures are therefore not intrinsic; they depend on other off pieces (R22C2-02, below).

### R22C2-03. One action for the edge-cycle family (proved for a stated family)

- Objects: a prepared state Y, an edge unit U in the cell X, the pair-tool catalogue of `cellpure4.py` at Y.
- Statement. Every word of the catalogue that passes the exact endpoint filter acts on X by the same accumulated
  rotation. So all of them have the same off geometry at their endpoint and the same landing chambers, and an occupied
  landing chamber fixed by every SAFE-ALL letter for one of them is one for all.
- Proof. Every retained pair tool fixes each edge cell setwise and acts on X by a power of one permutation ρ[X] (the
  function `expo` enforces this); inverses, powers and concatenations keep this. The stabilizer of an edge cell is
  dihedral of order 10, so a nonidentity cyclic subgroup has order 2 or 5, and within it at most one element sends
  U's label to 0. So the label action on X is the same, and by O8 the rotation is the same. The filter keeps every
  other off piece at its pose and takes exactly the lattice content of X off.
- Consequence. No candidate of this family gives a T1′ closure for 130 after (481, 5) or for 180 after (481, 1).
  `--max-exact` only bounds the number of rejected tries; the tools stop at the first exact success.
- Not covered: other preparations, other cyclic actions from other core families, transports, partners and words of
  several classes.

### R22C2-05. b06: the twelve landing rotations (computed; proved parts)

- Computed (`r22c2_05_vertex.py`), at the depth 40 stop. Exactly twelve menu elements at grip 91 land b06 as the
  rotation of its cell (items 14, 21, 23 and 43 to 51). With Q displaced by that rotation and every other off piece at
  its pose, eleven leave no landing chamber fixed by every SAFE-ALL letter; item 45 leaves all ten fixed, each blocked
  by the units b00 to b05 together (for chamber 18427 the cap of grip 17 holds no piece of Q and holds piece 83 of
  b00). Q alone fixes none in any of the twelve.
- Proved.
  1. Every T1′ for b06 without a partner fails R22C-V item 4: its landing word turns the cell {91} (R22C-G item 3), so
     Bad(W) ⊇ Ω_{91} and the type-6 deficit of 2 stays.
  2. A commutator of two letters at distinct grips acts trivially on a vertex content, since only letters at its own
     grip move it and such a letter is followed by its inverse; so do products and powers of such commutators. The
     transport family of `cellpure5.py` consists of such powers, so it cannot give b06's middle.
- Open: a one-partner T2′ for b06. The eleven passing rotations are the targets for its middle; N2, the creator and
  middle words and the cleanup V2 remain to be checked.

### R22C-FC. Theorem FC: no self-frozen set (proved; R22CF-05, X22C-07)

- Objects: a state Z reachable from the solved state, a nonempty set S of off pieces of Z.
- Statement. Some grip g has a cap that holds a piece of S, and no piece of S straddles the cut of g at Z.
- Proof. The inverse of a history from the solved state to Z is admissible from Z (Theorem D (F2)) and ends with
  every piece on the lattice, so it moves every piece of S. Let τ, at grip g, be its first letter that moves a piece p
  of S. A letter moves exactly the pieces inside its cap and is applied only when no piece straddles its cut, so p is
  inside the cap of g when τ is applied and no piece of S straddles it. No earlier letter moved a piece of S, so each
  piece of S has its region of Z then.
- Corollaries. No nonempty set of blocks is closed under "is frozen by". For a blocking group left by the structural
  order, some cap holds a piece of the group and is straddled only by units outside it. This gives a mover relative to
  S, not readiness.
- Instance (record check, X22C-07): each of the six blocking groups at the depth 60 stop has such a cap in
  `screen-k60.json`: units and grips (130, 481), (62, 328), (56, 364), (2, 26), (21, 98) and (65, 1).
- Uses: Theorem D.

### R22C-R1. Lemma R1: the one-letter readiness test (proved; R22CF-05, X22C-07)

- Statement. Let E be a retained element at an admissible grip g and Y = Z·(g, E). A rigid block inside the cap of g
  moves whole by E and every other block is fixed; a moved piece is inside, outside or across the cut of c at Y exactly
  as it was for the cut of E⁻¹(c) at Z; a moved frame domain R·K⁺ becomes E·R·K⁺, of class E(P). So whether A(U) is
  ready at Y (every needed grip admissible and holding only lattice pieces and frames of its class, and no second
  frame domain of that class in its cell) is a function of the classifications at Z and the permutations E induces on
  caps and classes; this is what `eb4.py` computes. The closure A·C·A⁻¹ has at most 2 + 2L_V + b_κ letters (M1-18).
- Proof. A K⁺ element permutes the 600 cuts and moves regions rigidly; the relations used are geometric relations
  between a region and a cut.

### R22C-CH. Lemma CH: co-held cycles (conditional; R22CF-05, X22C-07)

- Statement (as corrected by X22C-07). Let S be a set of units such that, at Z, every cap not straddled by a piece of S
  holds all of S or none of it (an intrinsic condition, K⁺-equivariant), and the needed grips of the units of S are
  blocked by units of S only. Then every retained word conjugates the configuration of S by one element of K⁺, and no
  retained word of any length makes a unit of S ready.
- Proof. Each admissible letter moves all of S or none, by one E; the intrinsic condition is preserved by E. The
  original form, which checked only the admissible caps at Z, fails when an outside straddler leaves and exposes a cap
  that splits S; no reachable instance of that gap was computed.

### R22C-05. One-letter unlocking with vacancy (proved)

- Objects: a whole edge-cell content U at Z.
- Hypotheses: H1, the identity or one of the 6,600 nonidentity retained letters A is admissible; at Y = Z·A, H2, A(U)
  is the only off domain in its class cell X = {a, b}; H3, a, b and a common neighbour c of the two are free; H4, every
  chamber of Ω_X is a hole.
- Statement. A·W·A⁻¹ is a strict step for U with at most 722 letters, where W is the pure word of Theorem C at a, b, c
  (|W| ≤ 720). A direct search examines at most (6,600 + 1)·5·4 = 132,020 instances per eligible unit, so on this
  class S(n) = 132,020·n and L(n) = 722.
- Proof. W lands A(U), fixes every piece outside X exactly and has Bad(W) = Ω_X (Theorem C); by H4 it takes no lattice
  piece off. The retained inverse A⁻¹ is admissible (R22D-01) and returns every other off piece to its pose of Z.
- With a vacating word V in place of H4, M1-17 gives 2|A| + 2|V| + 720 letters. N1 and N2 alone do not replace H4.

### R22C-06. A rigid two-domain cell cannot be unlocked by retained conjugation (proved)

- Hypotheses: two nonempty off contents U and V of different frame domains of one class lie in one class cell and in
  one rigid block.
- Statement. No retained setup followed by an own-class strict′ core lands either content.
- Proof. A retained setup gives the block one multiplier k ∈ K⁺, which keeps the two contents in one transformed cell
  with distinct domains. The core gives both one rotation g. If both landed, g·γ_U and g·γ_V would lie in K⁺, so
  γ_U⁻¹·γ_V ∈ K⁺, contrary to distinct domains. If one lands and the other keeps its exact pose, g = 1, which lands
  nothing.
- Instance: the three M1-19 pairs at the depth 60 stop, (105, 112), (109, 114) and (110, 162), lie in separate blocks,
  so the lemma does not apply to them; they are positive controls for separation.

### R22C-K. The catalogue, backward certification and coverage (conditional; R22C-07, R22CF-06, X22C-08, R22C2-06)

- Supercatalogue (R22C-07 as corrected by X22C-08 and R22C2-06). Put N = 177,120, R = 7,200^N, m = 35,400 (600 grips
  times the 59 nonidentity items), g = 6,600 (600 grips times 11 retained items), a core cap b, C_b = (m^(b+1) − 1)/
  (m − 1) and T = (g^R − 1)/(g − 1). Enumerate, for each starting unit, the chronological words A0·C1·A1·C2·A2·C3·A3·
  C4·A4 with native cores C_j of at most b letters and retained segments A_j of at most R − 1 letters (empty words
  allowed), and accept an instance only after exact replay proves every prefix admissible and the strict′ endpoint,
  with the creator checks of T2′ where it is so designated. On the states where some instance passes, L = 4b + 5(R − 1)
  and S(n) ≤ n·C_b^4·T^5.
  - Retained segments can be shortened to R − 1 letters: under retained letters each labelled piece keeps a pose
    k·γ with k ∈ K⁺, so at most 7,200^N labelled pose assignments occur, and a shortest retained path between two of
    them repeats none.
  - The family contains T1, retained conjugation, T1′ and the complete T2′ (its four cores D, W, W⁻¹, D⁻¹ and its
    retained segments V′, V′⁻¹ V2, V2⁻¹), once b covers the core lengths actually used; the bound b = 720 is not
    established for every cellpure4/5 cycle word (whose source bound is 3,200).
- Backward certification (R22C2-06). Let E be the finite set of height-2 pose tables and A_0 its lattice tables. Put
  A_(j+1) = A_j ∪ {Z ∈ E : an exactly replayed catalogue step from Z ends in A_j}. Every strict′ step lowers n by at
  least one, so the sets stabilize after at most N stages, and a state in A_j has a catalogue restoration of at most j
  steps, found by following its witnessed successors. On ∪ A_j the restoration length is at most n·max_(m≤n) L(m).
  This is the closure property that a one-step class lacks: a state's membership in a one-step class does not carry
  over to its successor (experiment A below shows a concrete failure of that inference).
- Open (coverage, the core of B′(2)): that every reachable height-2 state lies in ∪ A_j for some explicit catalogue,
  for example through a finite-checkable invariant that contains 𝒞_2 and guarantees a selected successor inside it.
  Neither count sufficiency nor the existing examples supply it.

### Experiments of line (c) (computed)

All exact, at exported pose tables, every step replayed (`stepchk.py`, `exp_a.sh`, `exp_b.py`, `tclose2.py`,
`jordan_target.py`, `x22c09_r2_ready.py`; stage 12 section 11; commands in section 6).
- A, an order at the depth 40 stop (X22C-06, R22C2-04): b12's T1′ step, then b14, then b15. b14's census word,
  computed at the stop, is admissible after b12's step but takes 17 lattice pieces off, so it is not strict′ there;
  b15's landing word at that state leaves 25 of 25 landing chambers occupied and fixed, each with the Q-free cap 286
  blocked by b07, b11 and b13; seven also have the Q-free cap 439, blocked by b08 and b09, and one the cap 378,
  blocked only by the 17 collateral pieces of b14's word. Closing b14's census word at the state after b12's step
  instead (R22C2-04; `runs/round22/line-c/b14-after-b12/`): N1 and N2 hold for its Bad(W), 222 chambers of which 17
  are occupied; the peeling certificate stops at 36 of them and a 1,800-second greedy search leaves 5 occupied, so V
  is not built, but all 19 orbits of the SAFE-ALL group that meet Bad(W) have verified Jordan certificates
  (`jordan_target.py` on the state's pose table). By Corollary A1 a vacating word V exists, and V·W·V⁻¹ is a strict′
  step for b14 after b12 (R22A-06). Whether b15 then lands is decided only at the state after a constructed b14 step.
- B, the blocking cycle {130, 180} at the depth 60 stop (R22C-03, R22C2-02): after the readiness letters (481, 5) and
  (481, 1) the landing words (432 letters each) leave 48 of 61 and 32 of 56 landing chambers fixed, none by Q alone.
  For 130 every fixed chamber has the Q-free cap 429, blocked by 131, 132, 133, 142, 150, 151 and 163; for 180 the
  Q-free cap 555, blocked by 174, 175 and 176. The other preparations, (496, 1) for 130 and (481, 2) for 180, which
  R22C2-03 does not cover, fail the same way: their landing words (432 letters each) land all 61 and 56 pieces, and
  after W the occupied chambers of Bad(W⁻¹) (86 chambers; 61 and 56 occupied) include 61 and 22 that every SAFE
  letter fixes, so N1 fails (`runs/round22/line-c/exp-b2.json`).
- C, two-letter readiness at the depth 60 stop (X22C-09; `x22c09_r2_ready.py`): the first letter at a cap that holds
  a straddler of the unit's inside caps, the second any retained letter. Unit 36 becomes ready after (255, 1) then
  (459, 1) (34 of its 66 first letters tried before the first success); units 44 and 59 have no ready pair (66 and 33
  first letters, exhaustive for this shape), and unit 27 has no first letter of this shape. Readiness is not vacancy:
  the pair gives a candidate for a closure, not a step.
- D, an order at the depth 60 stop (R22C2-02): strict steps for 174, 175 and 176 in turn, then 180: queued.
