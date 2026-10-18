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

### The order of R22A2-04 at the actual states, and b17 (computed; the integrator's replay)

- What was run (`scripts/round22/replay_a3.sh`, section 6). R22A2-04's order at the depth 40 stop, each step built at
  the state the previous built step reached: the unit's census word alone when it is strict′ there, otherwise a fresh
  closure V·W·V⁻¹ with R22A2-04's middle W (`tclose2.py`: V from the peeling certificate of Theorem B, else a
  300-second greedy search). Every built step is replayed from the exported pose table (`stepchk.py`) before the next
  one starts. A unit with neither is skipped, and the replay goes on from the last built state.
- Built, each landing its unit, taking no lattice piece off and keeping every other off piece at its exact pose: b00
  (census word, 60 letters, 1,193 → 1,160 off pieces), b01 (census word, 84, → 1,088), b02 (closure, 158, → 977),
  b08 (closure, 762, → 944), b11 (closure, 878, → 915), b09 (closure, 970, → 857) and b10 (closure, 1,172, → 823);
  every V came from the peeling certificate. Seven units in 4,084 letters, 1,193 → 823 (digest `743992e8…`).
- Proved, not built: b07 (tried after b02), b13 and b19 (after b11), b14, b16, b21 and b20 (after b10). At each of
  these states the census word is not strict′ and the greedy search did not finish V, but N1 and N2 hold for the
  closure's complete Bad(W), and `jordan_target.py` finds a global Jordan certificate on every orbit of size at least
  2 that meets it (20 orbits for b07, b13 and b19, 19 for each of the four at 823; none has 2 to 7 chambers). So by
  Corollary A1 each has a strict′ closure step at the state where it was tried (R22A-06, R22A2-05), as R22A2-04
  predicts from the stop. Their V are not built, so the states after these steps are not known exactly, and the
  replay went on without them.
- b17. At the stop b17 (86 pieces of class 2, the whole content of the edge cell {542, 593}) is moved by no clean
  power, and its pole 542 holds pieces of b08, b09 and b10, so Theorem C does not apply there (R22C3-05). At the state
  of 823 those three have landed: the cell's 86 home chambers are vacant, both poles and the five common neighbours
  495, 496, 552, 570 and 590 are free, and exactly one landing rotation fixes the edge (menu item 26 at 542), so
  Theorem C gives a pure-cell step of at most 720 letters. `cellpure4.py --search` builds one of 432 letters (a cycle
  through the edge cells {542, 593}, {542, 570} and {570, 593}); its replay lands all 86 pieces, takes no lattice piece
  off and keeps the other 737 off pieces at their exact poses: 823 → 737 (digest `2e3b8f99…`). An order turned a unit
  that no step could move at the stop into one with a short pure step.
- At the state of 737. The closures of b14, b16, b21 and b20 still have their premises: their Bad(W) is that of the
  same words (a fixed word's Bad set does not depend on the state), N1 and N2 hold, and every orbit that meets it has
  a global Jordan certificate (19 each), so each still has a strict′ step there. b21's Bad(W) is b16's (digest
  `1b0e8081…`), so b16's certificates serve both. The other off pieces at 737 are those at 823 less b17's, at the same
  poses, so W's ghost premises carry over; b17's landed pieces enter only through the occupancy of Bad(W), which the
  test reads at 737. b15 and b05 without a preparation (`exp_b.py`): `cellpure4.py` finds a landing word W of 432
  letters for each (all 25 and 61 pieces land; the 86 lattice pieces of the cell go off), but at Z1 = Z·W 17 and 48
  occupied chambers of Bad(W⁻¹) (86 chambers; 25 and 61 occupied) are fixed by every SAFE letter, so N1 fails and
  this T1′ has no closure. 48 is also the count R22C3-05 records at the stop: landing b00, b01 and b02 freed none of
  b05's. b06, a vertex content, has no landing word of this family. None of the three was tried after the orders
  R22C3-05 proposes.
- Totals. Built: eight units in 4,516 letters, 1,193 → 737. Proved at 737 and not built: four more. Left at 737: b03,
  b04, b05, b06, b07, b12, b13, b14, b15, b16, b18, b19, b20 and b21.
- Command: section 6. Records: `runs/round22/line-a/replay/`.
- Uses: R22A2-04, R22A2-05, R22A-06, Corollary A1, Theorem B, Theorem C, R22C3-01.

### The replay continued with certificate steps (E2; computed; the integrator's replay)

- What was run (`scripts/round22/replay_slp.py` and, for b06, `scripts/round22/e2_b06_prep1.py`, `e2_b06_prep2.py`
  and `e2_b06_t1.py`; section 6). From the state of 737 a plan of steps, each built at the state that the previous
  successful step reached:
  - a literal word (b12's T1′ word of `runs/round22/t1prime-k40/`);
  - a closure V·W·V⁻¹ with R22A2-04's middle W: N1 and N2 of its complete Bad(W) are read at the current state
    (`jordan_target.py`) and, if both hold, V is built there as an R22B-10 certificate (`r22b10_cert.py`) and checked
    (`check_r22b10.py`);
  - a T1′ step W·V′·W⁻¹·V′⁻¹ (R22C-T1′): W lands the unit and takes the lattice pieces Q off; V′ vacates
    T = Bad(W⁻¹) at Z1 = Z·W, a literal word from `tclose2.py` or else a certificate built and checked at Z1. W is
    a stored word or the class-cell pure twist that `cellpure4.py --search` finds at the current state.
  Every program is applied to the exact state by `slp_apply.py`, which evaluates a certificate as its straight-line
  program without expanding it, and checks the strict′ contract against the state in front of the step: the unit
  lands, no lattice piece goes off, and every other off piece keeps its exact pose or lands. Only such a step
  advances the state; its exported end table starts the next.
- Run 1, from 737 (digest `2e3b8f99…`). b12 by its T1′ word (1,158 letters; 737 → 696); b14, b16, b20 and b21 by
  closures with certificate V (696 → 618 → 507 → 447 → 437; each certificate passes the checker; the step lengths
  are integers of 151 to 161 digits). The closures of b07, b13 and b19 fail N2 at 437: their targets (198 chambers
  each; b07 and b13 share one) lack holes on SAFE orbits of 7,200, 3,600, 2,400 and 120 chambers (30, 11, 4 and 0
  holes where 33, 21, 17 and 1 are needed). At 737 the target of b07 and b13 already failed on the last three (15 of
  21, 5 of 17 and 0 of 1); b19's was not tested there. Round 3 proved these closures at the states of 977 and 915,
  and there are only as many holes as off pieces (O10), so the landings in between used them up: the resources of
  R22C3-02 run out at actual states, not only in principle.
- Run 2, from 437 (digest `a3fef11c…`): T1′ steps. b04 (W of 288 letters from `cellpure4.py`, Q = 86; V′ literal;
  1,254 letters; 437 → 392), b03 (W 60, Q 111; V′ literal; 1,106 letters; → 329) and b18 (W 60, Q 111; 770 letters;
  → 324); then b07 (W 48, Q 186), b13 (W 60, Q 197) and b19 (W 84, Q 198), whose closures had failed, each with V′ a
  certificate at its middle (T of 198 chambers; N1, N2 and 20 Jordan certificates; 163, 155 and 147 digits;
  → 213 → 112 → 96); then b15, whose blockers in COV-04 (b07, b12, b13 and b14) had all landed (W of 432 letters
  from `cellpure4.py`, Q = 86, T of 86 chambers; N1, N2 and 19 Jordan certificates; V′ a certificate; 163 digits;
  → 71). N2 asks, orbit by orbit, for a hole outside the target for each occupied chamber of the target. At a
  closure's middle Z these are the chambers of the lattice pieces that W takes off; at a T1′ middle Z1 they are only
  the landing chambers of the unit, since every other chamber of T is a hole there (COV-06). So a closure needs one
  hole outside Bad(W) per lattice piece that W takes off, a T1′ step one outside T per landing piece: that is why
  the T1′ steps passed where the closures had run out of holes.
- b05 at 71. Its T1′ with the 432-letter word of `cellpure4.py` (Q = 86; T of 86 chambers, 61 occupied) fails N1:
  31 occupied chambers of T are fixed by every SAFE letter at Z1, where the only off pieces besides Q are b06's 10.
  COV-04 put 31 of b05's fixed chambers at 737 behind {b03, b04, b06}, and of these only b06 is still off.
- b06 with b05, a branch at the state of 324 (`runs/round22/line-a/replay2/b06-324/`). The grip-91 letter W = (91, 14)
  lands b06 (10 pieces) and b05 (61) together. Its element is off K⁺, so W takes all lattice pieces of cap 91 off and
  T = Bad(W⁻¹) is the whole chamber set of cap 91 (3,097 chambers), of which 71, the landing chambers L, are
  occupied at Z1. So N2 at Z1 reads, orbit by orbit, |H ∖ T| ≥ |L| for the holes H in front of W. A SAFE-ALL
  preparation X keeps every off piece at its exact pose (so L is fixed) and moves H. `e2_b06_prep1.py` gives
  X0 = (178, 7), counted for b06 against its cell; after it N2 still fails by 6 on five orbits (2, 1, 1, 1 and 1).
  `e2_b06_prep2.py`, counting against the orbits of that middle, adds (256, 1), which keeps the holes inside T, and
  (75, 1), which moves one hole out of T: the deficit is 0. With X = (178, 7)(256, 1)(75, 1), at Z1 = Z·X·W (3,343
  off pieces, Q = 3,090) the ghost premises of W⁻¹ hold, N1 and N2 hold, and all 29 orbits of size at least 2 that
  meet T have Jordan certificates, so V′ exists (Corollary A1). The builder of E22-1b builds V′ there (3,097 targets;
  `b06-14-v-cert.json.gz`), the checker passes it, and `slp_apply.py` applies X·W·V′·W⁻¹·V′⁻¹ from 324: b06 lands,
  b05 lands, no lattice piece goes off and the other 253 off pieces keep their exact poses, 324 → 253 (digest
  `8e1a57da…`; a program of 1.8·10¹⁸³ letters). This is the first strict′ step for b06, and it needs no partner: the
  type deficit of R22C2-05 and COV-04 is a matter of where the holes are, which a SAFE-ALL preparation changes, and
  the Bad set of a letter off K⁺ is its whole cap, whose type vector has type 6, unlike the edge middles of M1-13
  (R22C-V). The step is checked exactly; it has the form of R22C-T1′ with U = b06 ∪ b05, after a SAFE-ALL
  preparation, which keeps every lattice piece on the lattice and so needs no undoing.
- b06 with b05 at 71, the last two units (`runs/round22/line-a/replay2/b06-71/`). They are the only off pieces, so
  there are 71 holes, as many as landing chambers, and N2 at the whole-cap middle needs every hole outside cap 91
  (H ∩ Bad(W) = ∅, COV-03). `e2_b06_prep1.py` gives X0 = (178, 7) again; after it one hole is still in T and N2 fails
  by 6 on five orbits (2, 1, 1, 1 and 1). `e2_b06_prep2.py` adds (170, 1) and (75, 1): no hole is left in T and the
  deficit is 0. With X = (178, 7)(170, 1)(75, 1), at Z1 = Z·X·W the 3,097 off pieces are Q, the whole lattice
  content of cap 91. N1 holds: the 715 chambers of T that every SAFE letter fixes are holes. N2 holds with equality
  on each of the 29 orbits of size at least 2 that meet T (2,382 chambers of T), and each has a Jordan certificate.
  `tclose2.py` finds no literal V′; the builder builds V′ (3,097 targets; about 3.5·10¹⁶³ letters) and the checker
  passes it. `slp_apply.py` applies X·W·V′·W⁻¹·V′⁻¹ from 71: b06 (10 pieces) and b05 (61) land and no lattice piece
  goes off, 71 → 0 (digest `3f84154f…`; a program of 7.0·10¹⁶³ letters). No piece is off: the state is on the
  lattice.
- Totals. With round 3, all 22 units land, 1,193 → 0 in 21 strict′ steps (eight in round 3, thirteen in E2). The
  literal steps have 8,804 letters (4,516 in round 3, 4,288 in E2); the certificate steps have explicit lengths of
  147 to 164 digits, and the whole route is under 10¹⁶⁴ letters (about 8.1·10¹⁶³). These are existence bounds, not
  words to play.
- Command: section 6. Records: `runs/round22/line-a/replay2/`.
- Uses: R22C-T1′, R22C-V, R22B-10, Corollary A1, R22C3-02, COV-04, COV-06.

Open after round 2 and the replay: E2 built every step it used at the state in front of it and landed every unit,
so along this trajectory nothing is left (the seven steps proved but not built and the units left at 737 are settled
there). Open: coverage of B′(2) at every state of 𝒞_2 (section 8). Nothing here is formalized.

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

### Round 3 of line (b)

After E22-1b (section 4) the senior reviewer took line (b) a third round (findings R22B-11 to R22B-18, 18 October
2026): shorter seeds, Alt on every orbit, an audit of the two certificates and their checker, and the bound that short
seeds would give. Its exact computation is `scripts/round22/r22b13_seeds.py` (section 6); the integrator re-ran it and
got the output `runs/round22/r22b13_seeds.txt`. Z is the depth 40 stop (digest `7e128820…`), shared by b00 and b19.

### R22B-11. E22-1's seeds protected fewer orbits (checked in the source)

- Statement. `r22b01_cert.py` (E22-1) adds an orbit to the protected set only after its seed, both single-letter
  tables and its vacating steps succeed; `r22b10_cert.py` (E22-1b) protects every earlier target orbit, failed ones
  included. So the commutator seeds of 12 to 1,320 letters that E22-1 found were checked against fewer orbits than H2
  of R22B-01 asks in the full order of 19 orbits: in its first run every seed search up to orbit 41, the first to
  complete, protected no orbit, and later ones protected only 41, then 41 and 64. They do not form a compatible
  family, and E22-1 and E22-1b differ in both the search time and the protected set, so the time alone does not
  explain their different seed lengths. Section 4 is corrected accordingly.
- The historical seed words were not replayed against the full protected set; some may still pass it.

### R22B-12. When a power of a word is a seed (proved)

- Objects: Δ, the current target orbit; E, the union of the earlier target orbits; q, a SAFE word.
- Statement. Some positive power of q acts on Δ as a three-cycle and fixes E pointwise if and only if, on E ∪ Δ, q has
  exactly one cycle whose length is divisible by 3, and that cycle has length 3 and lies in Δ. Then M, the least common
  multiple of the lengths of the other cycles of q on E ∪ Δ (1 if there are none), is prime to 3, q^M is a seed of
  |q|·M letters, and every admissible exponent is a multiple of M.
- Proof. On a cycle of length λ, q^m fixes every point when λ divides m and moves every point otherwise. So q^m moves
  exactly three points of E ∪ Δ, as a three-cycle, only when one original cycle of length 3 survives and every other
  cycle dies; killing another cycle whose length is divisible by 3 needs 3 | m and kills the wanted one too.
  Conversely, under the condition 3 does not divide M, q^M kills every other cycle and keeps the 3-cycle as a
  three-cycle.
- Bound. If q is [x, y], [x, yz] or [x, yzu] for SAFE letters x, y, z, u (4, 6 or 8 letters), the other cycle lengths
  use at most c distinct primes, and every orbit in E ∪ Δ has at most D chambers, then every prime power dividing M is
  at most D, so M ≤ D^c and s ≤ 8·D^c. Every hypothesis is a finite check of one word. No uniform c and no short q for
  every target orbit is established.
- Counterexample. If q has one 3-cycle on Δ and another on an earlier orbit, every power that kills the earlier cycle
  kills the wanted one, so no exponent works; a larger exponent never repairs a failed criterion. A different order of
  the orbits changes E and can change the outcome; no better order was found.
- Instances: R22B-13.

### R22B-13. Two short seeds for b00 (computed)

- Statement. At Z, in the full decreasing-size order of b00's 19 target orbits, two eight-letter commutators [x, yzu]
  have powers that are seeds, each checked by the criterion of R22B-12 on the protected union and then as a power:
  - orbit 34: C = q⁴ with q = (118,11)(254,11)(106,7)(160,4)(118,11)(160,8)(106,6)(254,11), 32 letters, the
    three-cycle (12773, 72831, 72678); the other cycle lengths on the orbit are 2 and 4;
  - orbit 66: C = q⁵ with q = (254,4)(320,6)(340,9)(170,7)(254,8)(170,6)(340,3)(320,7), 40 letters, the three-cycle
    (78293, 78304, 135764); it fixes all 7,200 chambers of orbit 34, and the other cycle lengths on the protected union
    are 5.
- They are witnesses, not shortest seeds. The reviewer's bounded searches (6,000 candidates per orbit, then up to
  60,000 on five orbits, cut short on the sixth) found no short seed for the other 17 orbits; this shows nothing about
  their absence.
- Command: `scripts/round22/r22b13_seeds.py`; output `runs/round22/r22b13_seeds.txt`.

### R22B-14. Powers of one letter cannot shrink a support (proved; computed)

- Statement. At Z every one of the 6,039 SAFE letters has prime order: 1,647 have order 2 and 4,392 order 3. If x has
  prime order p, x^m is the identity when p | m and has the same support as x otherwise. So no power of a single SAFE
  letter isolates some of its cycles, and powers of letters supply no smaller support for H2 of R22B-08 or for a
  single-overlap commutator (R22B-03).
- Products and conjugates can have mixed cycle lengths, as the two seeds of R22B-13 show. Whether single-overlap pairs
  of short conjugates exist was not searched exhaustively.
- Command: `scripts/round22/r22b13_seeds.py` (the letter orders; the largest chamber type has 7,200 chambers).

### R22B-15. A local test for Alt, and what stays open (proved; the universal claim open)

- Statement. Let Δ be an orbit with d ≥ 8. If some SAFE word w acts on Δ with a p-cycle for a prime p with
  d/2 < p ≤ d − 3, then the group G|Δ that G induces on Δ contains Alt(Δ). Nothing is asked of the other orbits.
- Proof. The other cycles of w on Δ have at most d − p < p points, so none has length divisible by p, and the power of
  w by the least common multiple of their lengths acts on Δ as one p-cycle. G|Δ is transitive because Δ is an orbit. A
  block of a nontrivial block system that meets the p-cycle cannot have p distinct translates under it (their total
  size would exceed d), so it is invariant under the cycle, contains its whole support and has more than d/2 points,
  which no proper block has. So G|Δ is primitive, and Jordan's theorem, as used in Theorem A, gives Alt(Δ). For
  5 ≤ d ≤ 7 an exact closure of the permutation group with a membership test for all three-cycles decides it.
- Local and global. The global certificate of `buffer.check_jordan` asks more, because it supplies an alternating
  factor that acts on Δ alone; local Alt gives no seed that fixes the earlier orbits (the diagonal actions of R22B-01
  and R22B-07).
- Open: that the SAFE geometry forces either condition at every state of 𝒞₂. No counterexample appeared at the named
  states.
- Without Alt a V can still exist. The exact condition is π_V⁻¹(T) ⊆ H (R22B-03), and breadth-first search over the
  joint images of the target, with at most ∏_Δ C(|Δ|, |T ∩ Δ|) vertices, decides it: an image inside H gives V as the
  inverse of its path word, of at most that number minus one letters, and an exhausted component shows that no V
  exists. N1 and N2 alone do not decide it outside the hypotheses of Theorem A.
- Counterexample (abstract, not a state of the puzzle): with r = (0 1 2 3 4 5) and letters r², r⁴, r³, the group is
  the cyclic group of order 6, imprimitive and without A6. The holes {0, 3} cannot cover T = {0, 1} under any rotation,
  although N2 holds, yet they cover {1, 4}; so the lack of Alt does not by itself forbid V.

### R22B-16. The checker of E22-1b is sound, and the recorded numbers hold (checked in the source; computed)

- Statement. `check_r22b10.py` checks every leaf of the program against the SAFE alphabet rebuilt from the state, and
  every reference and expanded length. Every SAFE letter keeps each orbit, and so does every node, so evaluating V on
  the target orbits gives the same occupancy of the targets as evaluating it on all chambers, collateral of earlier
  steps included; targets in singleton orbits are holes by N1. The checker also evaluates every seed and step on all
  earlier target orbits and replays every step on the current one. So a pass proves that the encoded SAFE word clears
  T at the stated state, and its bound is R22B-01's 2·Σ k_i(s_i + 2e_i + 2f_i) with expanded word and path lengths.
- Instances. The reviewer re-ran the complete check of b19's certificate (PASS: 198 targets, 15 steps) and recomputed
  every node length and both bounds: |V| < 10^166 for b00 and < 10^141 for b19, both bounds below 10^166, seeds of
  121 to 165 digits over the two certificates (b00 from 122), and 14,018 and 12,688 nodes. In the reviewer's sandbox
  the command-line test of `test_r22b10.py` could not create its temporary directory; the integrator re-ran the whole
  file in this session and all 19 tests pass.
- Command: `scripts/round22/r22b13_seeds.py` (the audit of the node lengths and bounds).

### R22B-17. The bound that short seeds would give (proved; conditional)

- Statement. Assume the hypotheses of R22B-01 and Alt on each target orbit, so that R22B-05 gives the tables, and put
  E(d) = (d − 2)(2d − 1) and F(d) = (d − 3)(2d − 3)(2d − 1). If every seed has s_i ≤ S(d_i), then
  |V| ≤ 2·Σ_i k_i·[S(d_i) + 2E(d_i) + 2F(d_i)]. If every d_i ≤ D and S*(D) = max over 5 ≤ d ≤ D of S(d), then, since E
  and F increase there, |V| ≤ 2t·[S*(D) + 2E(D) + 2F(D)] with t = Σ_i k_i.
- Proof. Substitute the seed and table bounds into R22B-01; the number of targets occupied at the start is not used.
- Instances (Z, D = 7,200, E(7,200) = 103,644,002, F(7,200) = 1,491,955,394,391; the exact orbit sizes improve the
  constant):
  - b00 (t = 111): |V| ≤ 222·S*(7,200) + 547,390,496,139,580;
  - b19 (t = 198): |V| ≤ 396·S*(7,200) + 851,701,385,498,104.
  With the fixed-c hypothesis of R22B-12 the first terms become 1,776·7,200^c and 3,168·7,200^c. These are
  conditional bounds, not short words for either target. For b19, t is 198, not the 15 steps of its certificate: a
  seed can permute later orbits and fill targets that were holes. New seeds also need their own tables; the e, f ≤ 22
  of E22-1b belong to its anchors.
- For this geometry every SAFE letter keeps the K⁺ chamber type, of at most 7,200 chambers, and t ≤ n by N1 and N2;
  uniform seed bounds would give |V| ≤ 2n·[S*(7,200) + 2,984,118,076,786]. What is missing is the guarantee that the
  targets and compatible bounded seeds exist throughout the state class, with the coverage obligations of the
  catalogue (section 7).
- Command: `scripts/round22/r22b13_seeds.py` (E(7,200), F(7,200) and the constants).

### R22B-18. An extended commutator-power search (specification; not implemented)

- Inputs: the SAFE alphabet, the current orbit Δ in the existing order, E as the union of every earlier target orbit
  (failed ones included), the caller's deadline, a fixed random seed (suggested Random(221100 + least(Δ))), a limit on
  candidates (60,000) and on the expanded seed length (20,000). The phase takes at most the least of 120 seconds, half
  the orbit's remaining share and the deadline, and records interruptions and their reasons.
- Search. First validate cached candidates, among them the two words of R22B-13, against the current alphabet and the
  full E. Then sample: rank the grip pairs whose caps overlap on Δ by that overlap, then by their overlap on E, with
  ties shuffled; trial j takes every fourth time the next of the first 256 ranked pairs (a new pair every 12 trials) and
  otherwise a uniform overlapping pair; x is a SAFE letter at the first grip and y at the second; q = [x, yzu] when 3 |
  j and [x, yz] otherwise, the extra letters drawn from a random chamber of the two caps' intersection on odd trials
  and from a random SAFE grip on even ones. The search tests a bounded family; it never shows that no seed exists.
- Acceptance. Compute q's exact cycles on Δ, then on E, and reject q unless the criterion of R22B-12 holds; compute M
  by an exact least common multiple; reject when |q|·M exceeds the limit; keep the best by expanded length, then by the
  word and exponent. Evaluate an improving q^M independently on Δ and all of E before accepting it, store it as
  pow(word(q), M) with route `commutator`, take its anchors from its actual permutation, and run the existing tables and
  vacating steps. Without a candidate the Jordan route stays the fallback, and an exhausted budget is an incomplete
  result.
- Checker. `check_r22b10.py` already checks the leaves, exact lengths, the seed's three-cycle, the fixing of earlier
  orbits, the tables and V, so vacancy needs no new check. A claimed least exponent or spectrum bound would need q, M,
  the protected union and the primes recorded and recomputed independently; otherwise they are diagnostics.
- Controls: the two words of R22B-13 as positive controls, a duplicate 3-cycle on an earlier orbit as a negative
  control, and a forced deadline as an incomplete control; then one complete certificate built and checked before any
  conclusion about coverage.

### Round 4 of line (b)

After the decision to prove existence before building words, the senior reviewer took line (b) a fourth round
(findings R22B-19 to R22B-23, 18 October 2026): when local Alt and separating words give independent alternating
factors, whether Alt holds on every large orbit at every state of 𝒞₂, orbits of 2 to 7 chambers, and what the closure
bounds give without Alt. Its computations are `scripts/round22/r22b20_separation.py`, `r22b21_regular12.py` and
`r22b22_small_orbits.py` (section 6); the integrator re-ran them and got the outputs `runs/round22/r22b20_separation.txt`,
`r22b21_regular12.txt` and `r22b22_small_orbits.txt`, and re-ran the global check `jordan_target.py` of b00 (R22B-04).

### R22B-19. Local Alt and separating words give a product of alternating groups (proved)

- Objects: G = G(Z); orbits Δ_1, ..., Δ_r of G with d_i = |Δ_i| ≥ 8; U their union; H the group G induces on U.
- Hypotheses: H1, for each i the group G|Δ_i contains Alt(Δ_i) (R22B-15's local test or a global certificate); H2,
  for each pair i ≠ j with d_i = d_j, some SAFE word has different cycle types on Δ_i and Δ_j (the words may differ
  between pairs). Both are finite exact checks.
- Statement. H contains ∏_i Alt(Δ_i).
- Proof. Each coordinate image H_i is Alt(d_i) or Sym(d_i), so D = [H, H] lies in P = ∏_i Alt(Δ_i) and projects onto
  every factor: D is a subdirect product of nonabelian simple groups. Such a group is a product of diagonal subgroups
  (induct on r: the kernel of the projection onto the first r − 1 coordinates meets the last factor in the whole factor
  or trivially; in the second case the last coordinate is a simple quotient of a product of nonabelian simple groups,
  which factors through one of them). A diagonal between i and j needs d_i = d_j, since |Alt(d)| = d!/2 increases, and
  for d ≥ 8 its automorphism is induced by a bijection f: Δ_i → Δ_j. D is normal in H, and the centralizer of Alt(d) in
  Sym(d) is trivial, so h_j = f h_i f⁻¹ for every h ∈ H; then every element of H has equal cycle types on Δ_i and Δ_j,
  against H2. So D = P.
- Consequence for vacancy. If every orbit of size at least 2 that meets the target T has d ≥ 8 and H1 and H2 hold on
  them, N1 and N2 suffice for a vacating V: choose k_i = |T ∩ Δ_i| holes in each Δ_i, map them onto T ∩ Δ_i by an even
  permutation (correct the parity inside the chosen set or its complement), realize all of them at once in P and lift
  to a SAFE word. The word may move other orbits. If T meets an orbit of 2 to 7 chambers, H1 and H2 say nothing about
  it (R22B-22).
- This is containment in the induced action on U; the lifted word need not fix chambers outside U.
- Counterexample to dropping the size condition (abstract, not a state of the puzzle): the cyclic group generated by
  (0 1 2 3), holes {0, 2}, T = {0, 1}: N1 and N2 hold, and no element maps a subset of the holes onto T.
- Uses: R22B-15, Theorem A; the simplicity of Alt(d) and Aut(Alt(d)) = Sym(d) for d ≥ 7 (standard).

### R22B-20. At b00 no letter separates two pairs; one word separates all (computed)

- Statement. At Z the 6,039 SAFE letters have orders 2 (1,647 letters) and 3 (4,392). b00's 19 target orbits form 40
  pairs of equal size. Every letter has equal cycle types on two of them, the pairs with least chambers 34 and 66 and
  82 and 90, all four of 7,200 chambers, so single letters do not give H2 of R22B-19. The first 120-letter word of
  Random(1) separates all 40 pairs (pair-word digest `a41fd137…`), each of the 19 orbits gets a local Jordan witness
  within the first 80 words, and no target orbit has 2 to 7 chambers, so R22B-19 applies at b00. The global check
  (R22B-04) also passes, so the failed letter test contradicts nothing.
- The 10 non-target orbits of at least 8 chambers without a local witness in 200 words are failures of that search,
  not orbits without Alt.
- Command: `scripts/round22/r22b20_separation.py`; output `runs/round22/r22b20_separation.txt`.

### R22B-21. An orbit of twelve chambers with a group of order twelve (computed; counterexample)

- Statement. From the solved state, the eight menu twists (40, 12), (81, 12), (133, 12), (86, 12), (121, 12),
  (52, 12), (215, 12), (221, 12) (grip, item), in this order, are each admissible without uncertainty. They reach a
  state (digest `93d1dab0…`, 24,776 off pieces) whose off pieces all have left height 2, so it lies in 𝒞₂. Its SAFE
  alphabet has 356 grips and 3,916 letters. Grip 0 is SAFE, and every other SAFE cap misses cap 0 on the home
  chambers, so every other SAFE letter fixes cap 0. The 12 chambers 4, 8, 9, 18, 26, 35, 120, 121, 131, 132, 242, 271
  form one G-orbit, and the group G induces on it has exactly 12 elements (exhaustive closure). It is regular, so no
  element fixes a point; it contains no three-cycle and not Alt(12).
- So the claim that G|Δ contains Alt(Δ) for every orbit of at least 8 chambers at every state of 𝒞₂ (the second
  question of the round) is false, and neither the local nor the global Jordan route covers every state.
- Limit. The orbit has no holes, so any target that meets it already fails N2. B′(2) is not refuted, and the narrower
  claim, for target orbits with enough holes, stays open.
- How it was found: greedy packings of single-twist blocking sets that keep cap 0 SAFE and block every cap that meets
  it; then the word is replayed and the alphabet rebuilt at the reached state, so the conclusion does not rest on the
  packing.
- Command: `scripts/round22/r22b21_regular12.py`; output `runs/round22/r22b21_regular12.txt`.

### R22B-22. Small orbits need a joint test; they occur at the depth 60 stop (proved; computed)

- Statement. Let T meet orbits of 2 to 7 chambers and S be their union. Breadth-first search over the joint images of
  the tuple (T ∩ Δ)_{Δ ⊆ S} under the SAFE letters, with one common word for all coordinates, decides vacancy on S: a
  vertex whose sets all lie in the holes gives V on S as the inverse of its path word, and an exhausted component shows
  that none exists. It has at most ∏_Δ C(d_Δ, k_Δ) ≤ 35^s vertices for s such orbits, so a word of at most that number
  minus one letters exists when any does. Tests on each orbit alone (local Alt for 5 to 7 chambers by enumerating at
  most 5,040 elements, R22B-15) do not replace it: a diagonal A5 on two five-point copies passes every local test,
  and holes {1, 2′} cannot cover {1, 1′}.
- With large orbits. If R22B-19 gives P = ∏ Alt(Δ_i) on the large target orbits, project G onto the union of the large
  and small target orbits; the subgroup fixing S pointwise still projects onto P, since otherwise a nontrivial quotient
  of P would be a quotient of a subgroup of ∏ Sym(d), d ≤ 7, whose composition factors have order at most 5,040 <
  |Alt(8)| = 20,160. So clear S first with a passing word, then the large targets with that subgroup; singleton targets
  stay with N1. Without R22B-19 the general fallback is the joint search over the whole target (R22B-15, R22B-03).
- Counts. At the depth 40 stop no orbit has 2 to 7 chambers. At the depth 60 stop (14,610 off pieces, all of left
  height 2) there are 12 orbits of 4 chambers, 6 of 6 and one of 7; each of 4 or 6 chambers carries a group of order
  12 (generated by 11 letters), so the six-chamber orbits do not have Alt(6), and the one of 7 carries a group of order
  2,520, which is Alt(7). Eight orbits of 10 to 115 chambers passed the 100,000-element cap of the enumeration, which
  shows nothing about them.
- Command: `scripts/round22/r22b22_small_orbits.py`; output `runs/round22/r22b22_small_orbits.txt`.

### R22B-23. What the closure bounds give without Alt (proved; conditional)

- Objects: a preparation state Y with holes H; a target T; B(Y, T) = ∏_{Δ meets T} C(|Δ|, |T ∩ Δ|) − 1.
- Statement. If a SAFE vacating word for T exists at Y, one of at most B(Y, T) letters exists: breadth-first search on
  the joint target images reaches a subset of H, a shortest path repeats no vertex, and its inverse clears T (R22B-04's
  argument; no Alt needed). Every prefix is legal, because the SAFE alphabet depends only on the fixed off geometry.
  Each SAFE letter keeps a K⁺ chamber type of at most 7,200 chambers, so B(Y, T) ≤ 7,200^|T| − 1 ≤ 7,200^n(Y) − 1 when
  N1 and N2 hold; the hole-set bound C(177,120, n(Y)) − 1 also holds, and the smaller may be used.
- Closures. For R22A-06, |V·W·V⁻¹| ≤ |W| + 2B(Z, Bad(W)): V fixes every off pose but may rearrange lattice chambers
  of other orbits, which the proof does not use; after W the off geometry is a subset of the start's with the same
  poses, so V⁻¹ stays SAFE. For T1′, |W·V′·W⁻¹·V′⁻¹| ≤ 2|W| + 2B(Z1, Bad(W⁻¹)) with Z1 = Z·W; the hole count there is
  n(Z1), not n(Z), because W can create the collateral Q.
- Under R22B-19 with no target orbit of 2 to 7 chambers, vacancy is equivalent to N1 and N2; with such orbits add the
  passing joint test of R22B-22. Global Jordan certificates stay a sufficient alternative (Theorem A).
- What does not follow: a theorem that every closure at a state of 𝒞₂ exists exactly when N1 and N2 hold. R22B-21
  refutes Alt on every large orbit, and local Alt alone would not exclude diagonal coupling (a diagonal A8 on two
  eight-point copies, holes {1, 2′}, targets {1, 1′}). B′(2) still needs, at every required preparation state, either
  independent alternating factors or another exact certificate such as the joint search. N1 and N2 failures stay real
  obstructions for that target and send it to other preparations (section 7); not every remaining obligation moves to
  line (c).
- A later state needs the chosen word's full action: existence alone does not fix the lattice arrangement.
- Uses: R22B-04, R22B-19, R22B-22, Theorem A, R22A-06, R22C-T1′.

Open after round 4 of line (b): whether a non-Alt orbit can meet a target that has holes in it (R22B-21's orbit has
none), a target-specific theorem for orbits with enough holes, and the joint test wherever a target meets the small
orbits of the depth 60 stop.

### Round 5 of line (b)

The senior reviewer took line (b) a fifth round at the fast tier (findings R22B-24 to R22B-29, 18 October 2026): a
non-Alt orbit against a target with holes, a finite classifier of the induced actions, the small orbits at the depth 60
home targets and at the T1′ middles of 151 and 180, what a strict′ step keeps, and a vacancy theorem without Alt. Its
computations are `scripts/round22/r22b24_regular_hole.py`, `r22b26_small60.py` and `r22b27_small_t1prime.py`
(section 6); the integrator re-ran them and got the outputs `runs/round22/r22b24_regular_hole.txt`,
`r22b26_small60.txt` and `r22b27_small_t1prime.txt`.

### R22B-24. A non-Alt orbit meets a catalogue target that has a hole; vacancy still holds (computed)

- Statement. From the solved state, A = [(26, 45), (126, 2), (26, 12), (126, 5)]³ and then (15, 12), (100, 12),
  (105, 12), (116, 12), (84, 12), (132, 12), (238, 12) and (6, 1) (grip, item) are each admissible without
  uncertainty. They reach a state of 𝒞₂ (digest `3cb419d7…`, 21,790 off pieces, all of left height 2; 366 SAFE grips,
  4,026 letters). The twelve chambers 98, 2014, 2987, 3419, 3642, 3688, 4639, 6084, 6168, 8830, 9053 and 9099 form one
  orbit whose induced group has exactly 12 elements (exhaustive closure): it is regular and contains no Alt(12). Its
  only hole is 3642.
- The target. The 111 home chambers of the class-3 triangle cell {26, 99, 126} (digest `90c1dfc8…`, 63 occupied) are
  exactly Bad(W) of the transport W = A⁻¹ = [(126, 2), (26, 45), (126, 5), (26, 12)]³, a free class-3 word that lands
  the whole 111-piece content, keeps the other 21,679 off pieces at their exact poses and takes the 63 lattice pieces
  of the cell off. T meets the orbit at 8830 and passes N1 and N2.
- Vacancy. Grip 6 is the only SAFE grip whose cap meets cap 6, and its action there has 12 elements. Testing T under
  all of them at once finds one image inside the holes, that of the self-inverse letter (6, 1). So V = (6, 1) clears T
  and V·W·V⁻¹ is a closure.
- So the shortcut "every target orbit with enough holes contains Alt" is false at a reachable state with a real
  catalogue target. N1 and N2 still sufficed here: a reachable target that passes them without vacancy is not known.
- Command: `scripts/round22/r22b24_regular_hole.py`; output `runs/round22/r22b24_regular_hole.txt`.

### R22B-25. A finite classifier of the induced action, with a residual case (proved)

- Statement. Restrict the SAFE generators to a target orbit Δ of d points. A complete Schreier–Sims chain gives the
  exact order of the induced group K, and for d ≥ 8, K contains Alt(d) exactly when |K| ≥ d!/2 (every homomorphism
  Sym(d) → C₂ is trivial or the sign, so Alt(d) is the only subgroup of index 2). Blocks: fix a ∈ Δ and, for each b ≠
  a, close the equivalence generated by a ~ b under the generators; a partition into classes strictly between one
  point and Δ is a block system, and if none appears K is primitive. If |K| = d, K is regular. A chain or an
  enumeration cut off by a time limit decides nothing.
- Exact vacancy per case. Regular: test the d elements. Imprimitive, with blocks B_1, ..., B_m, h_i = |H ∩ B_i| and
  t_i = |T ∩ B_i|: if K contains independent Alt(B_i) factors (|B_i| ≥ 3; independent Sym(2) factors for blocks of
  two), vacancy on Δ is equivalent to some κ of the block action with h_i ≥ t_κ(i) for every i; without those factors
  the inequalities are only necessary. Several orbits: use their common projected group, not one element per orbit.
- Residual case. Primitive actions without Alt cannot be excluded on group-theoretic grounds: Sym(12) on the 66 pairs
  of 12 points is primitive, of order 479,001,600, and does not contain Alt(66). No proof excludes such an action
  from the SAFE geometry; there only the joint test of R22B-29 decides.
- Orbits that miss T impose nothing: the joint action on the union of the target orbits suffices, since every word
  in the restricted generators lifts to the same SAFE word. A global Jordan certificate can fail through cycles
  outside that union while vacancy holds; that is a failure of the stronger certificate.
- Uses: R22B-15, R22B-19, R22B-22.

### R22B-26. The depth 60 home targets miss every small orbit (computed)

- Statement. At the depth 60 stop (digest `ef45a117…`, 14,610 off pieces) the SAFE alphabet has 12 orbits of 4
  chambers, 6 of 6 and one of 7 (91 chambers), and none of them has a hole, so a target that meets one fails N2. None
  of the 184 units' home-cell targets meets them, and none of the 2,400 triangle and 600 tetrahedron home-cell
  targets does (480 and 120 in each of the five classes). So the joint test of R22B-22 has nothing to do at these
  targets; their large orbits and fixed chambers are separate questions.
- Scope: the home-cell targets of `screen.py`. A census or transport word needs its complete ghost Bad set, which can
  be larger than its landing cell.
- Command: `scripts/round22/r22b26_small60.py`; output `runs/round22/r22b26_small60.txt`.

### R22B-27. The T1′ middles of 151 and 180 miss the small orbits too (computed)

- Statement. With the alphabet rebuilt at each middle. 151: the endpoint of Theorem C's pure-cell action (R22C3-04;
  digest `d76309e0…`, 14,651 off pieces); its 86-chamber target has no chamber in an orbit of 2 to 7 chambers. 180:
  the recorded steps 174, 175 and 176, then (481, 1) and the 288-letter landing word of experiment D (digest
  `534aaa7a…`, 14,545 off pieces), with Bad(W⁻¹) rebuilt by ghost replay and matched to the stored target; it has
  none either, and N1 and N2 pass. Both middles still have 12 orbits of 4 chambers, 6 of 6 and one of 7. So the
  small-orbit joint tests at both are empty. This is about these two middles, not every T1′ word of the two units.
- Command: `scripts/round22/r22b27_small_t1prime.py`; output `runs/round22/r22b27_small_t1prime.txt`.

### R22B-28. What a strict′ step keeps: subgroup certificates, not inventories (proved)

- Statement. Let Y′ end a strict′ step from Y. By Lemma M, A(Y) ⊆ A(Y′) with the same chamber permutations for the
  old letters (R22C3-01), so every old local-Alt witness, every separating word (R22B-19) and the whole old joint
  target-image graph (R22B-22, R22B-29), which depend on the generators and T only, stay valid for the old subgroup.
  A cached accepting image S stays a certificate exactly when S ⊆ H(Y′). An exhausted old component proves failure
  only for the old subgroup, since new letters may help; if it is closed under the new generators, its acceptance test
  against H(Y′) decides the enlarged action.
- Merged orbits. Old local-Alt and separation hypotheses still certify the orbits that did not merge. On a merged
  orbit Δ′, an old Jordan word with a p-cycle certifies Alt(Δ′) when p > |Δ′|/2; otherwise check again.
- Inventories are not kept: recompute H(Y′) and the per-orbit counts. In the code, `Alphabet.holes` and
  `Alphabet.occupied` are snapshots taken when the alphabet is built, so `condition` on an old alphabet reads the old
  inventory.
- These are endpoint comparisons. They do not compare T1′ middles Y·W and Y′·W, or states after a retained
  preparation that moves off pieces; there check the off-geometry inclusion or rebuild the alphabet. A changed landing
  word needs its own complete Bad set.
- Uses: Lemma M, R22C3-01, R22B-19, R22B-22.

### R22B-29. The vacancy theorem: a finite equivalence without Alt (proved)

- Objects: a middle Y ∈ 𝒞₂ (where the vacating word acts: Z for a closure, Z1 for T1′, each T2′ premise at its own
  middle); H its hole set; T a complete target (Bad(W), or Bad(W⁻¹) for T1′); the nontrivial target orbits Δ, those
  of at least two chambers that meet T and are not all holes; Γ the component of the tuple (T ∩ Δ)_Δ under the SAFE
  letters acting on all coordinates at once.
- Statement. A SAFE-ALL word V with T ⊆ π_V(H) exists if and only if N1 holds (every fixed chamber of T is a hole), N2
  holds, and Γ contains a tuple whose union lies in H.
- Proof. If a path word u gives (T ∩ Δ)·u ⊆ H on every nontrivial target orbit, its inverse V = u⁻¹ maps H onto a
  superset of T there; singleton targets are holes by N1, and an orbit of holes stays one. Conversely a vacating V
  gives the accepting image T·V⁻¹ ⊆ H. Every prefix is admissible, because the SAFE alphabet depends only on the off
  geometry, which SAFE letters keep (O5). Γ has at most ∏_Δ C(|Δ|, |T ∩ Δ|) vertices, so its exhaustion is a finite
  negative certificate and a shortest accepting path gives a V of at most that number minus one letters (R22B-23).
  No group classification and no Alt is used.
- Reduction. Split the nontrivial target orbits into L (degree at least 8) and E, and let K be the subgroup of the
  joint action on L ∪ E that fixes E pointwise. If K induces a group containing ∏_{Δ ∈ L} Alt(Δ), vacancy is
  equivalent to N1, N2 and the joint test on E alone: clear E with an accepting word (hole counts on L do not
  change), then clear L through K with E fixed; necessity follows by projection. The kernel condition is a finite
  check (Schreier generators of the pointwise stabilizer). Global Jordan certificates give it, and so does R22B-19 when
  E has only orbits of 2 to 7 chambers (R22B-22's composition-factor argument); for large exceptional orbits it is not
  automatic. With L empty this is the equivalence above.
- T2′. The second middle depends on the full action of the first vacating word, so separate successes at unrelated
  middles do not certify one T2′ word: keep the first action's full effect, or quantify jointly over its possible
  actions and test the resulting second middles.
- This settles the vacancy question at a given middle, the case of a non-Alt orbit meeting a target with holes
  included (R22B-24 is an instance): it is decided by a finite test, and a step that carries the test needs nothing
  more. Not settled: whether N1 and N2 alone suffice at every target of 𝒞₂ (no reachable counterexample is known), and
  coverage, that every reachable height-2 state has a unit, a word and middles where the test accepts (section 8).
- Uses: O5, R22B-19, R22B-22, R22B-23, Theorem A.

Open after round 5 of line (b): the sufficiency of N1 and N2 alone, which no step needs once it carries the joint test
of R22B-29; the residual primitive case of R22B-25 where neither Alt nor a block system is found; and coverage of
B′(2), which line (b) hands to section 8.

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
- **E22-2, retained conjugation at the depth 100 stop (measured, exact; complete).** The complete r = 1 search of
  R22D-01: 2,574 retained letters at the 234 clean grips, the E-B6 search at each prepared state with exact
  classification updates, and an exact replay of every kept word r·C·r⁻¹. An exact rule prunes a letter when no
  piece of a high unit moves and no inside or straddler set changes at a cap the search reads (1,309 letters), so
  1,265 prepared states are searched. The 22 letters of grips 1 and 61 give no strict word (`eb6r.py`,
  `check_eb6r.py`), and neither does the full search: eight shards, all complete, 1,265 states searched, no strict
  word and no strict lowering word (`runs/round22/e22-2/`). So at the depth 100 stop no word r·C·r⁻¹ with one retained
  letter r and C in the E-B6 family is a strict step; longer preparations, six-letter commutators (R22D-02) and
  two-level steps (R22D-05) are not covered by this.
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
  more than 120 digits. No commutator seed turned up in the 8-second commutator phase. E22-1, with longer shares,
  found commutator seeds of 12 to 1,320 letters for at least 18 of b00's 19 orbits over its two runs, but checked
  each only against the orbits completed before it, not all earlier orbits (R22B-11), so they are not a certificate
  in the full order. Round 3 of line (b) (R22B-11 to R22B-18) found two seeds of 32 and 40 letters that pass in the
  full order (R22B-13).

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

The replay of R22A2-04 and b17 (section 2), with `k40.json` as above, `R=$A/replay`, and `<n>.json` the pose table
with n off pieces, gunzipped from `$R/tables/<n>.json.gz` (977, 915, 823 and 737) or rebuilt by replaying the words of
`$R/words/` in the order of `$R/seq.jsonl` with `stepchk.py`, whose records `$R/steps/` give each step's start and end
digests:

    bash $S/round22/replay_a3.sh k40.json <new dir>                                # about 2 hours
    python3 -I -B $S/round22/jordan_target.py $R/jordan/<u>-<n>-target.json <n>.json   # each about 1 minute
    python3 -I -B $S/round22/thmc_premises.py <n>.json research/jumbling/restoration/12-low-height/runs/screen-k40.json \
        $A/units/eb5-k40-k00-b17.json 17                                            # n = 1,193 (k40.json) and 823; about 1 minute
    python3 -I -B $S/cellpure4.py research/jumbling/theory 823.json $A/units/eb5-k40-k00-b17.json <out>.json --search
                                                                                    # writes <out>-word.json; about 4 minutes
    python3 -I -B $S/round22/stepchk.py 823.json $A/units/eb5-k40-k00-b17.json $R/words/b17.json <out> 737.json
    python3 -I -B $S/round22/exp_b.py 737.json $A/units/eb5-k40-k00-<u>.json - - <new dir>   # u = b15, b05, b06; about 5 minutes each

The run used a scratch copy of `replay_a3.sh` with fixed paths; the script here takes them as arguments, writes the
summary line of a closure step, which the scratch copy missed through a file-name slip (`seq.jsonl` is rebuilt from the
step records), and stops if a built closure is not strict′, which did not happen. `$R/jordan/` holds each target with
the test's output `<u>-<n>-jordan.txt` (b21 at 823 uses b16's, the same T), `$R/closures/` the closures' results,
`$R/b17/` the output of `thmc_premises.py` at 1,193 and 823, and `$R/t1prime-737.jsonl` the three `exp_b.py` results.

Experiment E2 (section 2: the replay continued), with `E=$A/replay2` and `<n>.json` the pose table with n off pieces,
gunzipped from `$R/tables/737.json.gz`, `$E/run1/z-b21.json.gz` (437) or `$E/run2/z-<kk>-<u>.json.gz`:

    python3 -I -B $S/round22/replay_slp.py 737.json $E/plan-run1.json <new dir>   # about 40 minutes
    python3 -I -B $S/round22/replay_slp.py 437.json $E/plan-run2.json <new dir>   # about 80 minutes
    python3 -I -B $S/round22/e2_b06_prep1.py <n>.json <p1>                         # 1 to 2 minutes
    python3 -I -B $S/round22/e2_b06_t1.py <n>.json <p1> <t0> 14                    # the reference middle; about 3 minutes
    python3 -I -B $S/round22/e2_b06_prep2.py <n>.json <p1>/x-word.json <t0>/b06-14-z1-table.json \
        <t0>/b06-14-v-target.json <p2>                                             # about 1 minute
    python3 -I -B $S/round22/e2_b06_t1.py <n>.json <p2> <new dir> 14               # about 35 minutes

`replay_slp.py` writes one line per step to `seq.jsonl` and, for step k of unit u, the files `<kk>-<u>-...` (run 1:
`<u>-...`): the landing word's check, the middle's pose table and Q, the closure's log and result, the target with
its Jordan test, the certificate with the builder's log and the checker's output, the program and its application
log, and the end table `z-<kk>-<u>.json`. The b06 branches `$E/b06-324/` and `$E/b06-71/` keep the first preparation
(`prep1/`), the X0 attempt's record with the reference middle and target of item 14 (`x0-attempt/`), the second
preparation with the script's output (`prep2/`) and the prepared attempt (`t1/`). The branch at 324 used scratch
copies of the three b06 scripts: `e2_b06_prep2.py` reproduces its preparation byte for byte, and `e2_b06_t1.py` adds
the free-list filter of review finding B06R-01, a path that run did not reach (`tclose2.py` found no literal V′); its
X0 attempt was stopped after items 14, 21, 23, 43 and 44. The branch at 71 ran the scripts as they are here.
Programs name their files by repository paths; pose tables, middles and closure premises are kept gzipped
(`gzip -n`).

Round 3 of line (b) (R22B-13, R22B-14, R22B-16, R22B-17), the senior reviewer's computation kept as written apart
from a header comment; it writes no file:

    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B $S/round22/r22b13_seeds.py   # about 2 minutes

Round 4 of line (b) (R22B-20 to R22B-22), the senior reviewer's computations kept as written apart from a header
comment; they write no file:

    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/round22/r22b20_separation.py    # about 1 minute
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/round22/r22b21_regular12.py     # about 5 minutes
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/round22/r22b22_small_orbits.py  # about 2 minutes

Round 5 of line (b) (R22B-24, R22B-26, R22B-27), the senior reviewer's computations kept as written apart from a
header comment; they write no file:

    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/round22/r22b24_regular_hole.py   # about 2 minutes
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/round22/r22b26_small60.py        # about 4 minutes
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/round22/r22b27_small_t1prime.py  # about 4 minutes

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
    python3 -I -B $S/r22b10_cert.py table za.json $L/b14-after-b12/b14-target.json <cert>.json.gz --seconds 3600   # about 2 minutes
    python3 -I -B $S/check_r22b10.py table za.json $L/b14-after-b12/b14-target.json <cert>.json.gz                 # about 7 minutes

`b14-core.json` is `{"P": [], "Q": W}` with W b14's census word (`$L/w14-census.json`); `b14-target.json` holds
`ghost.badW` of the closure (`pre.json`) with its digest; `b14-log.txt` and `b14-result.json` are the closure's log and
result, `b12-replay.json` the replay record of b12's step; `b14-cert.json.gz`, `b14-build-log.txt` and `b14-check.txt`
are the certificate of V, the builder's log and the checker's output.

Experiment D (R22C2-02), with `k60.json` the depth 60 stop as a pose table (`$S/export_table.py`), `E=$L/exp-d` and
`z1.json` the pose table at Z1 that `exp_b.py` writes as `u-z1-table.json` in the work directory `<dir>/e180-481-1`:

    sh $S/round22/exp_d.sh k60.json <new dir>                                      # about 2 hours
    python3 -I -B $S/round22/jordan_target.py $E/e180-cert/e180-target.json z1.json     # about 1 minute
    python3 -I -B $S/r22b10_cert.py table z1.json $E/e180-cert/e180-target.json <cert>.json.gz --seconds 3600   # about 5 minutes
    python3 -I -B $S/check_r22b10.py table z1.json $E/e180-cert/e180-target.json <cert>.json.gz                 # about 20 minutes

`exp_d.sh` needs a new or empty work directory and stops with a nonzero status when a producer or a check fails. It
writes the unit files (`units/`, `units.out`), the census records and picks (`zp-<u>.json`, `picks.out`), the step
words and their replay records (`w-<u>.json`, `s-<u>.json`), the pose tables after each step (`z1.json` to
`z3.json`), and for 180 the outcome without a preparation and after (481, 1) (`e180-none.out`, `e180-481-1.out` and
the work directories `e180-none/`, `e180-481-1/` of `exp_b.py`). The records in `$E` keep these files byte for byte
under other names: `run.out` is the driver's output, `units.jsonl` and `picks.jsonl` are `units.out` and
`picks.out`, `word-<u>.json` and `step-<u>.json` are `w-<u>.json` and `s-<u>.json`, `e180-<x>/result.jsonl` is
`e180-<x>.out` and `e180-<x>/closure-result.json` is `e180-<x>/c2/result.json`; the pose tables, the census records
and the closures' `pre.json` are not kept. `e180-cert/` keeps the target file (`ghost.badW` of the closure after
(481, 1), with its digest), the Jordan check, V′ as a certificate of R22B-10, the builder's log and the checker's
output.

Round 3 of line (c) (R22C3-03, R22C3-04, R22C3-05), the senior reviewer's computations kept as written apart from a
header comment; they read the records of experiment D and write no file:

    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 -I -B -u $S/round22/r22c3_audit_d.py   # about 6 minutes
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 -I -B -u $S/round22/r22c3_u151.py      # about 2 minutes
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 python3 -I -B -u $S/round22/r22c3_geometry.py  # a few minutes

Coverage (section 8: COV-01, COV-02, COV-04 and the experiment COV-07), the senior reviewer's computations kept as
written apart from a header comment; they write no file. COV-07 runs once for each first unit, at most two at a time:

    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/round22/cov01_isolated_triangle.py  # seconds
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/round22/cov02_point_orbits.py       # seconds
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/round22/cov04_middles737.py         # about 1 minute
    for u in 3 4 12 18; do COV_FIRST=$u OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 \
        python3 -I -B -u $S/round22/cov07_word_order.py; done                                     # up to 20 minutes each

Coverage round 2 (section 8: COV-09, COV-13 and COV-14), the senior reviewer's computations kept as written apart from
a header comment; they write no file:

    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/round22/cov09_resource_persistence.py  # about 3 minutes
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/round22/cov13_mixed_frames.py          # about 14 minutes
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/round22/cov14_isolated_wholecap.py     # under a minute

Coverage round 3 (section 8: COV-17, COV-18 with the checks of COV-20, and COV-19), the senior reviewer's computations
kept as written apart from a header comment; they write no file. Its experiment COV-22 runs once for each seed 0 to 7,
at most two at a time:

    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/round22/cov17_single_off.py       # seconds
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/round22/cov18_singleton_kinds.py  # under a minute
    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/round22/cov19_perfect_menu.py     # seconds
    for k in 0 1 2 3 4 5 6 7; do OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 \
        python3 -I -B -u $S/round22/cov22_mixed_search.py $k; done                                 # up to 29 minutes each

The integrator's checks after COV-20 (section 8, after COV-22); they write no file:

    OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 python3 -I -B -u $S/round22/cov20_all_landers.py     # about 14 minutes
    for L in 0:12 0:25; do OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1 \
        python3 -I -B -u $S/round22/cov20_two_level.py $L; done                                    # about 50 minutes each

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

All exact, at exported pose tables (`stepchk.py`, `exp_a.sh`, `exp_b.py`, `tclose2.py`, `jordan_target.py`,
`x22c09_r2_ready.py`; stage 12 section 11; commands in section 6). A step given as words is replayed letter by letter;
a step whose V is a checked straight-line program (b14's V in A, V′ in D) is Theorem T1′ or R22A-06 applied to that
exactly evaluated program, not a literal replay, and its end pose table is not computed (R22C3-03).
- A, an order at the depth 40 stop (X22C-06, R22C2-04): b12's T1′ step, then b14, then b15. b14's census word,
  computed at the stop, is admissible after b12's step but takes 17 lattice pieces off, so it is not strict′ there;
  b15's landing word at that state leaves 25 of 25 landing chambers occupied and fixed, each with the Q-free cap 286
  blocked by b07, b11 and b13; seven also have the Q-free cap 439, blocked by b08 and b09, and one the cap 378,
  blocked only by the 17 collateral pieces of b14's word. Closing b14's census word at the state after b12's step
  instead (R22C2-04; `runs/round22/line-c/b14-after-b12/`): N1 and N2 hold for its Bad(W), 222 chambers of which 17
  are occupied; the peeling certificate stops at 36 of them and a 1,800-second greedy search leaves 5 occupied, so V
  is not built, but all 19 orbits of the SAFE-ALL group that meet Bad(W) have verified Jordan certificates
  (`jordan_target.py` on the state's pose table). By Corollary A1 a vacating word V exists, and V·W·V⁻¹ is a strict′
  step for b14 after b12 (R22A-06). The builder of R22B-10 then built V there (51 steps, a 420-letter commutator seed
  on orbit 34 and Jordan-route seeds on the other 18 orbits, |V| < 10^158), and its checker passes it
  (`b14-cert.json.gz`, `b14-check.txt`), so the step is constructed. Whether b15 then lands is decided only at the
  state after that step, which needs V evaluated on all chambers.
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
- D, an order at the depth 60 stop (R22C2-02; `exp_d.sh`, `runs/round22/line-c/exp-d/`): 174, 175 and 176, which
  block 180's cap 555, land first, each by a strict′ step without endpoint collateral that the transport census finds
  at the current state (two clean powers each; 24, 24 and 132 letters; 14,610 to 14,515 off pieces), each replayed by
  `stepchk.py`. At that state 180's landing word without a preparation (360 letters) has no T1′ closure: N1 and N2
  fail for its T. After the readiness letter A = (481, 1) the landing word W (288 letters) lands all 56 pieces and
  takes the 86 lattice pieces of the cell off; at Z1 = W(Y), T = Bad(W⁻¹) has 86 chambers, 56 occupied, and N1 and N2
  hold. The peeling certificate stops early and a 300-second greedy search leaves 8 occupied, but all 19 orbits of
  the SAFE-ALL group that meet T have Jordan certificates (`jordan_target.py`), so V′ exists by Corollary A1. The
  builder of R22B-10 then built V′ (56 steps, Jordan-route seeds of 117 to 162 digits on all 19 orbits, tables with
  e ≤ 63 and f ≤ 170, |V′| < 10^162 within its bound; about 5 minutes), and its checker passes it
  (`e180-cert/check.txt`, about 20 minutes). So 180 has the strict step A·W·V′·W⁻¹·V′⁻¹·A⁻¹ at the state after 174,
  175 and 176 (T1′ with R22D-01): its blocking at cap 555 came from the order of the steps, as R22C2-02 says. 130,
  whose cap 429 seven other units block, was not tried, and the state after 180's step needs V′ on all chambers.

### Round 3 of line (c)

After experiment D the solver had stopped (owner, 18 October 2026), and the senior reviewer assisted the integrator
alone at the fast tier (findings R22C3-01 to R22C3-06, 18 October 2026): whether strict′ steps keep SAFE-ALL and the
hole inventories, a lemma for mixed orders, an audit of D, an order towards 130, the catalogue of the unresolved
units, and a uniform core bound. Its computations are `scripts/round22/r22c3_audit_d.py`, `r22c3_u151.py` and
`r22c3_geometry.py` (section 6); the integrator re-ran them and got the outputs `runs/round22/r22c3_audit_d.txt`,
`r22c3_u151.txt` and `r22c3_geometry.txt`.

### R22C3-01. Strict′ steps keep SAFE-ALL, not the hole inventories (proved)

- Statement. Let Z′ end a strict′ step from Z. Then A(Z) ⊆ A(Z′), G(Z) ≤ G(Z′), every orbit of G(Z) lies in one of
  G(Z′), and Fix(G(Z′)) ⊆ Fix(G(Z)). Proof: Z′ has a subset of Z's off pieces, each at its pose of Z; removing off
  pieces puts no off occupant and no straddler into a cap that was SAFE, and landed pieces are lattice pieces, which do
  not straddle (Lemma M). This compares endpoints, not the middle states of a macro.
- Certificates. An old global Jordan certificate with prime p still gives an alternating factor on its old orbit. It
  certifies a merged orbit Δ′ ⊇ Δ only if p > |Δ′|/2 (its other inequality survives); since orbits stay inside one K⁺
  type of at most 7,200 chambers, p > 3,600 suffices. Otherwise recompute on Δ′. Counterexample (abstract): Alt(8) ×
  Alt(8) on two blocks, with a 5-cycle certificate on each, enlarged by a swap of the blocks; the wreath product is
  transitive on 16 points and does not contain Alt(16); with two holes in one block and a two-point target split
  between the blocks, N1 and N2 hold and no V exists. Merging and counts do not replace a certificate.
- Inventory identity. With L the occupied chambers at Z, φ the step's good-ghost map on L, and D the chambers of all
  newly landed pieces (beneficial landings outside the named unit included): L′ = φ(L) ⊔ D, and for every chamber set Δ,
  h′_Δ = h_Δ + |L ∩ Δ| − |φ(L) ∩ Δ| − |D ∩ Δ|. Only when φ keeps Δ can the landings simply be subtracted; total holes
  fall, but the cores can move holes between old orbits.
- Finite tests for a later step: keep the old group and check its fixed target chambers and per-orbit holes at Z′, or
  recompute G(Z′), the counts and the certificates. Bad(W) of a fixed word does not depend on the state, its occupancy
  does; a newly chosen word needs its own ghost computation. For T1′ the tests belong at Z′·A·W.
- Uses: Lemma M, R22A-06, Theorem A.

### R22C3-02. Mixed orders need resource contracts as well as a blocker order (proved)

- Statement (resource-certified order). Name the units by their original piece sets, choose an order, and skip a unit
  that has already landed. At each stage choose a census word, a closure V·W·V⁻¹, a prepared T1′ word
  A·W·V·W⁻¹·V⁻¹·A⁻¹ or the complete T2′ word, and require: (i) the preparation is retained and admissible and the
  free-class hypotheses hold; (ii) the core keeps its whole-content and exact off-pose contracts; (iii) the Bad sets
  are complete ghost sets; (iv) vacancy holds at the right state, by N1, N2 and independent global Jordan factors or
  by an exact joint search; (v) the hole resources that later stages need are propagated. Then the whole order exists
  and is strict′, and every stage lowers n. All are finite checks, at the successive pose tables or uniformly over a
  stated finite family of predecessor endpoints. Proof: the local theorem of each stage, R22D-01 for the retained
  conjugation, the resource contract, induction.
- T1′ before V. At Y with B = Bad(W), T = Bad(W⁻¹), f: Ω∖B → Ω∖T the good-ghost bijection of W and D_U ⊆ T the landing
  chambers of U: H(Y·W) = (T∖D_U) ⊔ f(H(Y)∖B), so for every orbit Δ at Y·W,
  |H(Y·W) ∩ Δ| = |T ∩ Δ| − |D_U ∩ Δ| + |H(Y) ∩ f⁻¹(Δ)|, and N2 holds exactly when |H(Y) ∩ f⁻¹(Δ)| ≥ |D_U ∩ Δ|;
  singletons give the fixed-chamber obstruction. With a retained preparation A the resource set at Z is
  π_A⁻¹(f⁻¹(Δ)). This refines the type inequality of R22C-V to orbits.
- Static budget. If every allowed earlier step keeps each later resource set R under its good-ghost map and adds at
  most δ_{j,R} landed occupants to it, then h_initial(R) − Σ_j δ_{j,R} bounds the holes of R later; certify the SAFE
  subgroup needed there as well. R22A2-04 is the case where the resources are old SAFE orbits. Without these
  conditions use the identity of R22C3-01 or the actual endpoint.
- A blocker graph is only the schedule: all blockers of one cap are needed together, alternative caps are choices,
  and deficient orbits are shared resources. Acyclicity proves neither the predecessors' steps nor the budgets.
- Bounds, with F = ∏ C(d_i, k_i) − 1 for the vacancy target: |W| for a checked census word, |W| + 2F for a closure,
  2|A| + 2|W| + 2F for a prepared T1′. T2′ needs both its vacancy premises; replacing its first V by another vacating
  word does not keep the state of the second certificate.
- Counterexample to reusing independent successes (recorded, experiment A): after b12's step the census word of b14
  computed at the stop takes 17 lattice pieces off.
- Uses: R22C-T1′, R22C-T2′, R22D-01, R22A2-04, R22C3-01.

### R22C3-03. Experiment D passes an independent exact audit (computed)

- Statement. The reviewer replayed D: 174 (24 letters, 14,610 → 14,564, digest `d6653e1e…`), 175 (24 letters,
  → 14,532, `a4987caa…`) and 176 (132 letters, → 14,515, `9f24aa3e…`), each landing its whole piece set, taking no
  lattice piece off and keeping every other off pose. For 180 without a preparation the 360-letter W lands 56 pieces,
  keeps the other 14,459 off poses, displaces 86 lattice pieces, and N1 and N2 fail. With A = (481, 1), retained and
  admissible, U is the whole 56-piece content of its class cell and every grip of W is free in that class (H1); the
  288 letters give H2 and H3 (Q of 86 pieces; Z1 with 14,545 off pieces, digest `534aaa7a…`); a ghost replay rebuilds
  the stored 86-chamber T (56 occupied), and N1 and N2 pass. For H4 the reviewer checked the program's leaves against
  the SAFE alphabet, its references and lengths and the 19 Jordan certificates, and evaluated the actual V′ node on
  every nontrivial target orbit: every target is vacated; the 13 singleton targets were holes. So T1′ applies, and by
  R22D-01 A·W·V′·W⁻¹·V′⁻¹·A⁻¹ is a strict step at the state after 174, 175 and 176, of 578 + 2|V′| letters, ending
  with 14,459 off pieces.
- Evidence kind. This is a theorem applied to an exactly evaluated straight-line program, not a literal replay of
  about 10^162 letters, and the end pose table was not exported; the experiments paragraph of this section now says so.
- Command: `scripts/round22/r22c3_audit_d.py` (about 6 minutes).

### R22C3-04. 142, then 151: a proved prefix of 130's predecessors (proved from computed hypotheses)

- 142. At the depth 60 stop, 142 is the whole 41-piece class-0 content of the edge cell {412, 429}; its 86 home
  chambers are vacant, both poles and the common neighbours 224, 305, 394 and 515 are free, and one landing rotation
  fixes the edge (menu item 24 at 412). Theorem C gives a pure-cell strict step of at most 720 letters that fixes every
  original lattice piece.
- 151. 151 is the whole 45-piece class-1 content of {443, 481}; the poles and the common neighbour 286 are free; its
  cell holds 86 lattice pieces Q; the unique landing rotation is item 54 at 443. Theorem C gives a pure-cell W of at
  most 720 letters with that accumulated cell rotation, fixing everything outside the cell, so H1 to H3 of T1′ hold and
  Bad(W) = Bad(W⁻¹) is the cell's 86 home chambers. The reviewer computed W's exact endpoint without building W
  (digest `d76309e0…`, 14,651 off pieces: U landed, the 86 pieces of Q off): N1 and N2 pass, 11 target chambers are
  fixed holes, and the other 75 lie in 18 nontrivial orbits, each with a checked global Jordan certificate. So V′
  exists and T1′ lands 151, in at most 1,440 + 2·F151 letters with F151 ≤ 7,200^75 − 1.
- The order 142 then 151, without building V′. The two home cells are disjoint (the first all vacant, the second all
  occupied). The pure step of 142 changes only U142, so at 151's middle Q151 is unchanged and the off geometry has only
  lost U142. Keep the old SAFE subgroup and its 18 certificates (R22C3-01): the least slack on a nontrivial target orbit
  was 29, U142 has at most five pieces of any K⁺ type, and each old orbit lies in one type, so landing 142 uses at most
  five holes of any orbit; the singleton target holes are untouched. Every inequality keeps a slack of at least 24.
  The order lowers n from 14,610 to 14,524 in at most 2,160 + 2(7,200^75 − 1) letters. A proof from computed finite
  hypotheses, not a replayed word.
- Still open for 130: 131 and 163 (ready triangle contents) and 132 (a ready tetrahedron content) need a census
  transport or a fresh closure; 133 is a glued triangle content (a transport middle and T1′); 150 is a vertex content
  that needs a genuine vertex isolation middle (LC-12's all-free hypothesis fails at the stop). No order for these
  five is proved, so no step for 130 yet.
- Commands: `scripts/round22/r22c3_u151.py`, `scripts/round22/r22c3_geometry.py`.
- Uses: Theorem C, R22C-T1′, R22C3-01, Corollary A1.

### R22C3-05. The catalogue of unresolved units stays conditional (inventory)

- Depth 40. b05: an order followed by a fresh T1′ (48 occupied fixed landing chambers in the recorded middle; deleting
  b00 to b03 leaves 31; b04 and b06 are needed for the Q-free route); recompute H1 to H4 after a certified order.
  b06: a partner that supplies type 6, a genuine vertex middle and the complete T2′ cleanup (every own-class T1′
  without a partner is two type-6 pieces short at the stop, and edge partners cannot supply that type). b15: b12, then
  the built b14 closure, then b15's middle recomputed; the earlier diagnosis was at the end of b14's non-strict census
  word. b17: its home cell {542, 593} is vacant but pole 542 holds pieces of b08, b09 and b10, so Theorem C does not
  apply at the stop; after those three land, a vacant cell with free poles gives a step of at most 720 letters (done
  in the replay of section 2).
- Depth 60, the 32 E-B4 leftovers: 18 lack one-letter readiness (edge units 4, 36, 39, 44, 45, 59, 72, 74, 101, 115,
  121, 127, 144, 155, 167; vertex units 27 and 120; triangle unit 47); 11 fail N1 for their first preparation (50, 52,
  56, 60, 107, 112, 130, 138, 146, 162, 180), of which 56, 130, 138, 146 and 180 have occupied own locks; 3 triangle
  units become ready (102, 109, 114) but need actual transport words, Bad sets and certificates. D proves 180 after
  174, 175 and 176, not at the stop.
- Run 2 is not a strict′ order: two of its steps (0 and 2) move other off blocks (`moved_blocks` in its record), so
  Lemma M does not carry certificates through them, and its remaining units cannot be found by subtracting unit names
  from the census at the stop. The root-state E-B4 successes are independent instances, not a chain.
- The routes above are proposals, not results, except where R22C3-03, R22C3-04 or section 2 proves them.

### R22C3-06. A uniform core bound; coverage stays open (proved; conditional)

- Statement. Fix a state and its free class-P alphabet. Every letter permutes the complete five-label cell model of
  13,205 points; let H be the group generated. A realizable model action has a word of at most |H| − 1 ≤ 13,205! − 1
  letters (a shortest path in H repeats no element). Every word over a fixed free alphabet is admissible (freeness
  persists); equal action on all five labels of every nonempty cell gives the same accumulated rotation (O8), outer
  cells and foreign pieces stay fixed, so the shorter word has the same exact action and the same Bad set (free class
  words have no straddling ghost). Tracking only the occupied content points would not justify this.
- So b = 13,205! − 1 bounds every native core that satisfies the free-class hypotheses, those of T1′ and T2′
  included. It does not give such a core for any unresolved unit, nor cover mixed-class cores.
- With this b the supercatalogue of R22C-K (four cores, five retained segments) has explicit bounds without the open
  question whether b = 720 covers every cycle word: with N = 177,120, R = 7,200^N, m = 35,400, g = 6,600,
  C_b = (m^(b+1) − 1)/(m − 1) and T_R = (g^R − 1)/(g − 1), L = 4b + 5(R − 1) letters per step and S(n) ≤ n·C_b⁴·T_R⁵
  candidates. Shortening a retained segment must keep its complete labelled endpoint.
- Coverage. The strongest statement available is conditional backward certification (R22C2-06): A_0 the lattice
  tables of the finite height-2 universe, A_{j+1} adds the tables with a certified strict′ catalogue successor in A_j;
  the sets stabilize within N stages, and every state of their union has a restoration of at most n steps and n·L
  letters. Still needed: finite certificates for readiness and routing (legal vertex middles and partners included),
  for simultaneous vacancy at every middle (independent factors or joint subset reachability) and for the full T2′
  cleanup, and a successor invariant or backward-coverage certificate that contains all of 𝒞₂. A finite superset of
  𝒞₂ needs its own containment proof, and exploring histories of height at most 2 does not cover every reachable
  height-2 state.
- D and 142 → 151 support the order and resource lemmas; the occupied locks, b06's type deficit, the old b14 word after
  b12 and 180 without a preparation refute weaker criteria (readiness, total holes, deleting blockers, independent
  root-state successes). None is a counterexample to B′(2).
- Uses: O8, Theorem C, R22C-K, R22C2-06.

Open after round 3 of line (c): orders for 131, 132, 133, 150 and 163 with resource contracts, then 130; b05, b06 and
b15 after certified orders (section 2 gives the state after the replay); and coverage of 𝒞₂.

## 8. Coverage of B′(2)

The question left by lines (b) and (c): does every reachable height-2 state Z with n ≥ 1 off pieces have a catalogue
step, that is a unit, a word and middles at which the vacancy test of R22B-29 accepts? The coverage shard (the senior
reviewer, fast tier, 18 October 2026; findings COV-01 to COV-07, all `minor`, all adopted) attempted the proof and
designed the deadlock experiments. Its computations are `scripts/round22/cov01_isolated_triangle.py`,
`cov02_point_orbits.py` and `cov04_middles737.py`; the integrator re-ran them with the outputs
`runs/round22/cov01_isolated_triangle.txt`, `cov02_point_orbits.txt` and `cov04_middles737.txt`, and ran its proposed
experiment as `cov07_word_order.py` (output `cov07_word_order.txt`). Experiment E2 of this section is the replay of
section 2 continued with every step built at the state in front of it; it reached the lattice (the subsection after
COV-07). Round 2 (COV-08 to COV-16, after E2) refutes two shortcuts and reduces coverage to one finite per-state
predicate, Cover(Z) (COV-15). Round 3 (COV-17 to COV-22) constructs reachable states with one off piece in every kind
of cell and shows that the catalogue covers them; whether COV-01's state is reachable stays open, and COV-22 is its
experiment. Coverage stays open: no reachable state without a step was found, and nothing here proves that every
state has one.

### COV-01. An isolated triangle with an intrinsic Q-shadow (computed; proved; reachability open)

- The state X. Every piece is at the identity except piece 1474, at the pose R = element(I_a, 0, 25) (digest
  `6580ef02…`). Its one off piece is of type 18 and height 2, in the class-0 triangle cell C = {0, 2, 23}, and the only
  hole is 1474. Body facet 3 separates its rotated region from the interiors of the lattice pieces, so X passes the
  exact non-overlap, type and own-class freeness checks. Whether X is reachable is not established.
- The certificate. With g0 = R⁻¹, the chambers 404 and 762 are of type 24 and lie in Ω_C. Every retained letter that
  moves the landed chamber 1474 is blocked by g0(404) or g0(762): the moving grips are 0, 1, 2, 3, 7, 15, 18, 19 and
  23, and both witnesses are inside each of them, except that 762 straddles grip 18 and 404 straddles grip 19. This is
  an intrinsic Q-shadow in the sense of R22C2-01.
- What it defeats. (1) T1: landing a triangle content by a free-class word needs a whole triangle home-cell Bad set
  of 111 chambers, and X has one hole; R22C-05 is an edge template. (2) T1′ fails N1 for every own-class landing word,
  not only for one middle, since R22C2-01 identifies all landing rotations up to left multiplication by K⁺. (3) A
  retained preparation cannot remove the certificate: X has no type-24 hole, so every type-24 chamber stays occupied,
  and if the preparation gives U a multiplier k ∈ K⁺, the occupied chambers k(404) and k(762) give the same
  certificate in the moved cell (their occupants need not keep their identities). (4) T2′ cannot repair it: its
  creator must use U's free class and return U to its exact pose, so by Lemma G its accumulated rotation on U's cell
  is the identity, and the witness chambers are occupied again before W. (5) There is no other off unit to order
  first. The same holds for every otherwise-lattice assignment with this off geometry and its one type-18 hole,
  wherever the hole is.
- So a lemma that covers isolated units by non-overlap, the class-cell model, type conservation and the absence of
  other off pieces is false. X is not a counterexample to B′(2) without a reaching history, nor to the unrestricted
  supercatalogue, whose mixed-class native cores it does not obstruct. Open: a legal reaching history of X or of its
  family, or a reachability invariant that excludes it. A bounded reaching search that fails would prove nothing.
- Command: `scripts/round22/cov01_isolated_triangle.py`; output `runs/round22/cov01_isolated_triangle.txt`.
- Uses: R22C2-01, Lemma G, R22C-05.

### COV-02. Seven geometric off-point cases (computed)

- Fix class 0 and divide its off-point slots by H = K⁺ ∩ Sym⁺(class 0), of order 1,440 (generated by the K⁺ indices
  1, 2 and 45, found by exact closure; the frames are menu items 0, 12, 24, 25 and 27). Since K⁺ is transitive on the
  classes, these are all the cases. There are seven, with 5,764 slots in the class (slots, points in the class,
  stabilizer order): outer (5,280, 4, 360); vertex (385, 480, 3); edge I, in its home cell (86, 1,440, 1); edge II,
  the cell {0, 2} from the home cell {0, 54} (86, 1,440, 1); a triangle without home chambers (111, 1,440, 1); a
  tetrahedron without home chambers (87, 480, 3); a triangle with home chambers (111, 480, 3). COV-01 is in the last
  case.
- This enumerates geometric points, not contents, hole sets or reachable states: an edge case with a trivial
  stabilizer already has 2^86 − 1 nonempty masks, so an exhaustive classification needs proved restrictions or a
  symbolic reduction (COV-03). An outer content cannot move under its own-class words, and a retained conjugation
  keeps it outer.
- Command: `scripts/round22/cov02_point_orbits.py`; output `runs/round22/cov02_point_orbits.txt`.

### COV-03. A local certificate, and what coverage still needs (proved; conditional)

- A local certificate is a named unit, a retained preparation, the whole-content and free-class core checks, complete
  Bad sets, and an exact vacancy certificate at every middle it needs, the T2′ cleanup included. Off pieces outside
  the neighbourhood of its grips (inside or straddling one of their caps or cuts) cannot invalidate its letters; the
  hole resources are separate data.
- Isolated contents. Let U, with type vector u, be the only off content, in a cell of kind κ. If |U| is less than the
  home-cell size e_κ, no T1 closure works: a landing uses a label-0 point of that kind, so Bad(W⁻¹) contains a whole
  home cell, while the forward and inverse Bad sets have equal type counts and only |U| holes exist; a retained
  preparation keeps this. For T1′, R22C-V becomes u ≤ u − |H ∩ Bad(W)| type by type, so H ∩ Bad(W) = ∅ is necessary,
  and then the middle has exactly |Bad(W⁻¹)| holes: vacancy must carry the whole hole set of the middle onto that
  target.
- A symbolic Q-test. Fix a content and a reference landing rotation g0. For a landed chamber x let M(x) be the grips
  with a retained letter that moves x, and for c ∈ M(x) let B_c be the chambers q of the source cell whose regions
  g0(q) lie inside or straddle cap c. The intrinsic Q obstruction is that every B_c contains an occupied chamber. So the
  hole bits enter through finite Boolean clauses, and if for each c some type t has |B_{c,t}| > u_t, every
  type-compatible hole set leaves c blocked (COV-01 is the case u_24 = 0). The hole quantifier must keep
  non-overlap, all type counts and every proved reachability restriction. H ∩ Bad(W) fixes Q and with it the SAFE
  geometry of the middle; with that geometry fixed and independent alternating factors, vacancy reduces to the
  occupancy of fixed targets and the per-orbit inventories, and otherwise to the joint action (R22B-29).
- The global obligation is that every reachable Z has a passing local certificate. 𝒞₂ is closed under strict′
  endpoints and n decreases, so once this per-state statement is proved no new successor invariant is needed: what is
  missing is coverage, not closure. A proposed order needs a certified first step; blockers form AND/OR choices with
  shared hole resources; intrinsic middle shadows are self-obstructions; Theorem FC gives a mover, not a ready or
  strict′ unit (COV-01 has free own-class grips). No reachable state where every template fails was found.
- With the free-class cores, b = 13,205! − 1 and R = 7,200^177,120 keep L = 4b + 5(R − 1) and S(n) ≤ n·C_b⁴·T_R⁵
  (R22C3-06). The recorded reaching histories of the test states are much shorter than b, so their inverses already
  give a step in the unrestricted native-core supercatalogue, and failures of the four geometric templates at those
  states cannot refute it.
- Uses: R22C2-01, R22C-V, R22C-FC, R22C3-06, R22B-29.

### COV-04. At 737, b05 and b15 are blocked by other units, b06 by two types (computed)

- At the replay state of 737 (digest `2e3b8f99…`) the reviewer built the T1′ middles of b05 and b15 from the
  pure-cell accumulated rotations, without expanding the words, and checked the fixed-chamber lists against
  `buffer.condition`.
- b05: class-3 cell {12, 91}, U = 61, Q = 86, landing rotation item 17 at grip 12; middle `a4c50c4c…` with 762 off
  pieces. 48 landing chambers are fixed and occupied, none of them by Q alone. With Q's geometry fixed, the minimal
  blocker sets are {b03} for 17 chambers and {b03, b04, b06} for 31; grip 17 is Q-free, moves all 48 and is blocked by
  b03, b04 and b06.
- b15: class-3 cell {217, 324}, U = 25, Q = 86, item 58 at grip 217; middle `a5fb0b10…` with 798 off pieces. 17 fixed
  occupied landing chambers, none fixed by Q alone; the minimal blocker set is {b07, b12, b13, b14}, and grips 286 and
  371 are Q-free and move all 17. So landing b12 and b14 does not open these routes; b07 and b13 block them too.
  Earlier steps can change Q and open other routes, so this does not say that these four units must land first.
- b06: Ω_91 has 10 holes and 727 are elsewhere; the type-6 triple (pieces of U, holes, holes inside) is (2, 2, 2) and
  the type-9 triple (1, 1, 1). R22C-V fails by two type-6 pieces and one type-9 piece, and since every own-class
  vertex landing has Ω_91 ⊆ Bad(W), this excludes every unprepared own-class T1′ at 737, not only the pure-edge family
  of `exp_b.py`. Edge partners cannot supply type 6. The deficit depends on where the holes are, so after other steps
  the inventory needs a new check.
- So the recorded failures of b05 and b15 are interactions, not intrinsic Q obstructions; a certified order that
  removes the blockers, followed by fresh vacancy checks, is a route, not a result.
- Command: `scripts/round22/cov04_middles737.py`; output `runs/round22/cov04_middles737.txt`.

### COV-05. Unit 130 at depth 60 is blocked by seven units (from the record)

- From the stored record, which the reviewer did not re-run: after the preparation (481, 5) the middle `3747dce8…` has
  Q = 86, U = 61 and 48 fixed occupied landing chambers, none fixed by Q alone. The minimal Q-free blocker sets are
  {131, 132, 133, 163} for 17 chambers, that set with 150 and 151 for 15, and with 142 as well for 16; grip 429 is a
  Q-free route for all 48, blocked by {131, 132, 133, 142, 150, 151, 163}.
- R22C3-04 proves 142 then 151; the steps of 131, 132, 133, 150 and 163 are not proved, and 150 also needs a genuine
  vertex middle. So neither a strict′ step for 130 at the stop nor an order of its predecessors is established.

### COV-06. Coverage needs the holes and the off poses, not the lattice orientations (proved)

- Statement. At a strict′ boundary keep the identities and exact poses of the off pieces and the hole set H. Two
  valid states with the same such data have the same admissibility and strict′ outcome for every fixed word, although
  their labelled lattice endpoints may differ. Proof: match their lattice pieces by chamber. A matched pair has the
  same region and poses k1, k2 ∈ K⁺, and receives the same accumulated left multiplier a along a common trajectory;
  a·k1 ∈ K⁺ iff a ∈ K⁺ iff a·k2 ∈ K⁺. Regions and cut classifications agree at every prefix, and the off pieces follow
  the same exact trajectories.
- Successor formulas. Let a free-class W land exactly U and fix the other off pieces; B = Bad(W), T = Bad(W⁻¹),
  f: Ω∖B → Ω∖T its good-ghost bijection, and D ⊆ T the landing chambers of U. For T1, with π the chamber permutation of
  V and B ⊆ π(H): H_out = π⁻¹((T∖D) ⊔ f(π(H)∖B)). For T1′, with π the chamber permutation of its V′:
  H1 = (T∖D) ⊔ f(H∖B); H2 = π(H1) ⊇ T; H3 = (H ∩ B) ⊔ f⁻¹(H2∖T); H_out = π⁻¹(H3). The term H ∩ B records the source
  holes that are left when Q returns. Retained conjugations add their known chamber permutations before and after.
- So a coverage order can be continued without the accumulated rotations of the lattice pieces, but it needs V's
  action on all chambers, not only on the target orbits; the evaluator can compute it on all of Ω, or on each of the
  36 K⁺ types, with cycle powers. A labelled end pose table needs the rotations as well; `slp_apply.py` computes them
  (E2 below).
- Uses: Lemma M, R22C-T1′.

### COV-07. The stored T1′ words form no order at 737 (computed)

- The question: does some order of the four stored T1′ words of b03, b04, b12 and b18
  (`runs/round22/t1prime-k40/steps/u<n>.json`, of 160, 942, 1,158 and 160 letters, built at the depth 40 stop) give
  successive strict′ steps at 737, a unit that has landed being skipped? Each word is applied literally and checked
  against the strict′ contract; a CPU cutoff or an uncertain classification would be inconclusive.
- The integrator ran the four branches (by first unit, two at a time). b12's word is strict′ at 737; the words of b03,
  b04 and b18 are not, either at 737 or after b12. All four branches end without an order. So this unchanged-word
  hypothesis is refuted: the words must be rebuilt at the current state. This is not a deadlock: E2 rebuilds them.
- Command: `COV_FIRST=<3|4|12|18> scripts/round22/cov07_word_order.py`; output `runs/round22/cov07_word_order.txt`.

### E2 and coverage (computed; the integrator's replay)

- What E2 shows. With every step rebuilt at the state in front of it (COV-07), the replay of section 2 takes the
  depth 40 stop to the lattice: 1,193 → 0 off pieces in 21 strict′ steps (eight in round 3, thirteen in E2), each
  applied exactly. At every state it reached, a step of the catalogue {census word, closure V·W·V⁻¹, T1′ with a
  literal or certificate V′, Theorem C, T1′ after a SAFE-ALL preparation} was found for some unit; no state of the
  replay was a dead end.
- COV-04's blockers, both ways. b15 passes once b07, b12, b13 and b14 have landed; b05 alone still fails N1 at 71,
  where b06 is the only other off content of its middle. b06's type deficit goes once a SAFE-ALL preparation has moved
  the holes, as COV-04's remark that the deficit depends on where the holes are allows.
- COV-03's isolated contents. At 71 b05 and b06 are the only off contents, so H ∩ Bad(W) = ∅ is necessary for the
  whole-cap T1′, and then the middle has exactly |T| = 3,097 holes and V′ must carry the whole hole set onto T. The
  preparation moves all 71 holes out of cap 91, and N2 holds with equality on each of the 29 nontrivial target orbits.
- Resources (R22C3-02). Closures that round 3 proved at 977 and 915 fail at 437 for lack of holes, while T1′ steps
  for the same units pass: a closure needs a hole outside Bad(W) for each lattice piece that W takes off, a T1′ step
  one outside its target for each landing piece.
- What it does not show. E2 is one trajectory from one stop, along a fixed plan rather than a search; it says
  nothing about other states of 𝒞₂, and COV-01's state is untouched. Coverage stays open; round 2 (below) starts
  from these results.
- Uses: COV-03, COV-04, COV-06, COV-07, R22C3-02, section 2 (E2).

### Round 2 of coverage

After E2 the senior reviewer took up coverage again at the fast tier (findings COV-08 to COV-16, all `minor`, all
adopted, 18 October 2026): whether a step's hole resources survive the steps before it, whether every reachable state
has a cap whose whole off content lands, an exact preparation test, and the strongest coverage theorem the record
supports. Its computations are `scripts/round22/cov09_resource_persistence.py`, `cov13_mixed_frames.py` and
`cov14_isolated_wholecap.py`; the integrator re-ran them and got the same digests and verdicts, with the outputs
`runs/round22/cov09_resource_persistence.txt`, `cov13_mixed_frames.txt` and `cov14_isolated_wholecap.txt`. Two
proposed shortcuts are false (COV-09, COV-13), coverage reduces to one finite per-state predicate (COV-15), and
neither a proof of B′(2) nor a reachable deadlock is established.

### COV-08. T1′ resources pull back through the good-ghost map (proved)

- Statement. Let a free-class core W land a nonempty set U exactly and keep every other off piece at its pose; let
  B = Bad(W), T = Bad(W⁻¹), f: Ω∖B → Ω∖T its good-ghost bijection, H the holes of Z and D ⊆ T the landing chambers.
  The middle holes are H1 = (T∖D) ⊔ f(H∖B) (COV-06). For each orbit Δ of the actual middle group G1 = G(Z·W) put
  R_Δ = f⁻¹(Δ∖T). Then N1 is exactly D ∩ Fix(G1) = ∅, and N2 is exactly |H ∩ R_Δ| ≥ |D ∩ Δ| for every Δ: finite checks
  from Z, W and the middle geometry. Counting the holes of Z outside T is wrong in general, since the source sets R_Δ
  lie outside B. For a whole-cap letter B = T is the cap and f is the identity outside it.
- The resource contract. For a strict′ predecessor P with occupied chambers E before it, good-ghost map φ on E and
  landing chambers D_P of all the pieces it lands, every fixed chamber set R has h_after(R) = h_before(R) + |E ∩ R| −
  |φ(E) ∩ R| − |D_P ∩ R| (the identity of R22C3-01, applied to R = R_Δ). Keeping off poses or an orbit partition gives
  no bound on this. Sufficient for an old vacancy certificate to survive: the target off pieces and W's contract
  survive, the old middle SAFE alphabet stays available, and the old per-orbit inequalities hold with the new H. A
  stronger and simpler condition is that φ keeps each R_Δ and D_P avoids it.
- Middles are not monotone. Endpoint SAFE alphabets grow under strict′ steps (R22C3-01), but middles need not: moving
  holes out of B adds displaced lattice geometry Q, which can remove SAFE letters, split orbits or fix landing
  chambers; moving holes into B removes some Q but spends outside holes; landing other off pieces can merge orbits.
  Detection: recompute the middle off geometry, its SAFE alphabet and orbits, D ∩ Fix(G1) and the inequalities, with
  the joint test of R22B-29 where independent alternating factors are not certified. Merged-orbit counts alone certify
  nothing.
- Uses: COV-06, R22C3-01, R22B-29.

### COV-09. A strict′ predecessor turns N2 at a whole-cap middle from true to false (computed)

- The run. Start from the recorded 96 state of run 2 (`runs/round22/line-a/replay2/run2/z-05-b19.json.gz`) and prepare
  it by X = (178,7)(170,1)(75,1), the preparation that E2 later used for b06 at 71 (`replay2/b06-71/prep2/x-word.json`);
  every letter is in the SAFE alphabet and the off fingerprint is kept. Then apply X⁻¹ and the stored b15 program as
  an SLP. This predecessor is strict′: 96 → 71, b15's 25 pieces land, collateral 0, the other 71 off pieces exact; it
  ends at the recorded 71 state (`291ae718…`).
- The letter is W = (91,14), b06's whole-cap letter, whose landing set D is the 71 chambers of b05 and b06 at both
  states. Before the predecessor (source `5e077fb6…`, the prepared 96 state) cap 91 holds 3 holes and the middle 3,119
  off pieces; N1 and N2 hold. After it the cap holds 20 holes and the middle 3,077 off pieces; N1 holds and N2 fails,
  with the deficits (orbit, deficit) (6, 2), (8, 2), (9, 1), (10, 3), (11, 3), (12, 2), (14, 2), (15, 5), 20 in all.
  The holes outside the cap fell from 93 to 51. The old middle SAFE alphabet is a subset of the new one; the orbit
  partitions differ.
- So keeping the target off poses and the old middle alphabet does not keep the T1′ resources; COV-08's inventory
  contract is the missing hypothesis. E2 restored the resource at 71 by applying the same X again before b06's T1′
  (section 2).
- Command: `scripts/round22/cov09_resource_persistence.py`; output `runs/round22/cov09_resource_persistence.txt`
  (159 s of CPU in the integrator's run, 167 s in the reviewer's).
- Uses: COV-06, COV-08, section 2 (E2).

### COV-10. SAFE preparation: an exact finite test and a flow criterion (proved)

- Exact test. Fix Z and a core W with the contract of COV-08, put G0 = G(Z), and let Γ0 be the joint orbit of the
  hole set H under G0. A SAFE preparation exists exactly when some J ∈ Γ0 gives a middle at which the exact vacancy
  test of R22B-29 accepts T. For each J the middle holes are (T∖D) ⊔ f(J∖B), and the middle off geometry, hence its
  SAFE group, is fixed by the original off geometry and J ∩ B (the displaced lattice regions are W applied to B∖J).
  Every J ∈ Γ0 is reached by a legal SAFE word, on a shortest path of at most ∏_i binom(|A_i|, |H ∩ A_i|) − 1 letters,
  A_i the starting SAFE orbits; conversely every SAFE preparation gives such a J. No Alt hypothesis and no
  identification of starting with middle orbits is assumed.
- Flow criterion (sufficient). Choose an inside mask I ⊆ B; it fixes the middle geometry and G1. With A_i the G0
  orbits, h_i = |H ∩ A_i|, b_i = |I ∩ A_i|, Δ_j the G1 orbits, d_j = |D ∩ Δ_j| and R_j = f⁻¹(Δ_j∖T), find integers
  0 ≤ x_ij ≤ |A_i ∩ R_j| with Σ_j x_ij = h_i − b_i and Σ_i x_ij ≥ d_j; require also that I respects the fixed hole
  bits of Z and that D ∩ Fix(G1) = ∅. If the partially filled starting orbits have degree at least 8 and their joint
  action contains the product of their alternating groups (global Jordan words or R22B-19), one SAFE word X realizes
  the chosen J; if the nontrivial target orbits of the middle carry independent alternating actions, Corollary A1
  gives V′. Partially filled small or exceptional orbits need the joint-action test (with its kernel condition when
  they are combined with large alternating factors); separate local tests do not suffice.
- This is a finite sufficient condition, not a claim that its group hypotheses hold at every reachable state. It keeps
  each starting orbit's hole count, which is stronger than type conservation; the capacities |A_i ∩ R_j| record how
  starting and middle orbits meet, so no bijection between them is needed.
- The integrator's count. In the six certificate checks of V′ recorded for E2 (`*-v-check.txt` under
  `runs/round22/line-a/replay2/`) every target orbit had at least 120 chambers and passed by a Jordan certificate; no
  orbit of 2 to 7 chambers met a target there.
- Uses: COV-06, COV-08, R22B-19, R22B-29, Corollary A1.

### COV-11. prep2 is a heuristic, not a preparation test (audit)

- `scripts/round22/e2_b06_prep2.py` accepts only a single letter that strictly lowers its deficit and stops after 60
  added letters, so even the flow hypotheses of COV-10 do not make it complete. Abstract counterexample: Sym(8)
  generated by adjacent transpositions, one hole at point 1, and the requirement that the hole reach point 8. The
  group contains Alt(8) and the one-unit flow is feasible, but every one-letter move from {1} leaves deficit 1, so
  strict descent stops, while the sequence (1 2), (2 3), …, (7 8) succeeds.
- Its second phase scores against the stored middle partition and has no inside-mask constraint, so it can move holes
  into or out of T and change Q and the actual middle alphabet; a zero reference deficit certifies nothing about the
  resulting middle. The E2 driver rebuilt and checked the actual middle and applied the whole program, so the recorded
  steps stand. A greedy failure or timeout never shows that no preparation exists; the exact test is COV-10's.
- Uses: COV-10, section 2 (E2).

### COV-12. Whole-cap cores: exact eligibility and a three-segment form (proved)

- Eligibility. Let C be the chamber set of an unstraddled cap c and W = (c, i) with element g ∉ K⁺. Then B = T = C and
  the good-ghost map is the identity outside C. If W must land some off pieces and keep every other off piece at its
  exact pose, every off piece inside c must land, since a nonidentity left multiplication changes every pose. So W is
  eligible exactly when the cap holds a nonempty off set U_c and g·γ_p ∈ K⁺ for every p ∈ U_c: all these poses lie in
  the one right coset g⁻¹K⁺. Two frame domains in the cap rule it out; one domain still needs a matching menu element.
  A SAFE preparation fixes all off poses and cannot change eligibility.
- Vacancy. For a prepared hole set J and landing chambers D the middle holes are (C∖D) ⊔ (J∖C); N1 is
  D ∩ Fix(G1) = ∅, N2 is |(J∖C) ∩ Δ| ≥ |D ∩ Δ| for every actual middle orbit Δ, and R22B-29 decides vacancy exactly.
  With COV-10's preparation test this characterizes the whole-cap T1′ route that keeps H3.
- Three-segment form. Allow other off pieces inside c to move to other off poses under W, and require only that a
  nonempty set U lands. If a SAFE V′ at Z·X·W vacates C, then X·W·V′·W⁻¹ is strict for U. V′ fixes every middle off
  piece, the moved ones and Q included. W⁻¹ is admissible, since those pieces stay on their side of the cap and lattice
  pieces never straddle. C is vacant, so W⁻¹ moves no lattice piece: it returns Q to the lattice and the moved pieces to
  their poses, and U, which V′ took out of C, stays landed. X was SAFE and needs no undoing. The same hole formula and
  vacancy test apply. Appending V′⁻¹ then needs its own admissibility check, since after the unwinding the surviving
  off pieces need not have their middle poses. So a cap whose content does not all land does not rule out every
  whole-cap step.
- Uses: COV-06, COV-10, R22B-29, Lemma M.

### COV-13. A reachable height-2 state with no all-content-landing cap (computed)

- The state. From the solved state apply menu item 12 at every class-0 grip in increasing order: 120 legal letters,
  digest `afc43efd…`. It has 147,904 off pieces, all of left height 2, in four off frame domains.
- The audit. Of the 600 grips, 480 have straddled cuts and the other 120 are unstraddled caps holding at least two off
  frame domains; there are no other cases. Two domains γ1K⁺ ≠ γ2K⁺ cannot both land under one g, since g·γ1 and g·γ2 in
  K⁺ would give γ1⁻¹γ2 ∈ K⁺. So no admissible letter lands the whole off content of its cap, and SAFE preparations
  leave this unchanged.
- This answers the packet's question Q3 negatively inside 𝒞₂: the universal all-content-landing lemma is false.
  Theorem FC holds (the 120 unstraddled grips are movers), and by FC no reachable state with off pieces has every cut
  straddled. The state is no counterexample to coverage: its inverse history of 120 letters restores the solved state
  with one free class, a direct strict′ core (a closure with empty V whose Bad set is already vacant).
- Command: `scripts/round22/cov13_mixed_frames.py`; output `runs/round22/cov13_mixed_frames.txt`.
- Uses: R22C-FC, COV-12.

### COV-14. The isolated triangle has whole-cap landers but no prepared whole-cap vacancy (computed; proved)

- The state is COV-01's X (digest `6580ef02…`, reconstructed). The unstraddled caps that contain piece 1474 are those
  of grips 0, 1, 2, 3, 7, 13, 15, 19 and 23, and 36 whole-cap letters land it: grip 0 with items 12, 16, 20 and 24 to
  32; grip 2 with items 14, 21, 23 and 43 to 51; grip 23 with the items of grip 0.
- At W = (0, 12) the middle (digest `dad66d60…`) has 3,096 off pieces and a target of 3,097 chambers; 1474 lands in
  chamber 2149. N1 fails, since 2149 is SAFE-fixed and occupied, and N2 has the deficit orbit 564 (0 holes, 1 needed).
  The one source hole, 1474, is inside the cap.
- Every preparation fails (proved). All 36 landers use grips 0, 2 and 23, each containing the triangle cell, so the
  type-24 witness chambers 404 and 762 receive the same landing rotation as the piece. There is no type-24 hole and a
  SAFE preparation keeps the piece exact, so both witnesses stay occupied. Every landing rotation differs from the
  reference R⁻¹ by a left multiplication in K⁺, and the intrinsic Q-shadow certificate (R22C2-01) moves with it and
  fixes the occupied landing chamber under every SAFE letter of the middle. So N1 fails for every preparation and all
  36 letters; moving the type-18 hole out of the cap does not help.
- Reachability of X is still open, so this is no counterexample to B′(2). If X is reachable, the geometric templates
  of COV-01 and the whole-cap route all fail there, and only a native core, such as the inverse of a reaching
  history, is left.
- Command: `scripts/round22/cov14_isolated_wholecap.py`; output `runs/round22/cov14_isolated_wholecap.txt`.
- Uses: COV-01, COV-12, R22C2-01.

### COV-15. Coverage reduces to one finite per-state predicate (proved; conditional)

- The catalogue. Free-class cores with their complete off-pose contracts and complete ghost Bad sets; closures
  V·W·V⁻¹; T1′ steps with vacancy at their actual middles; SAFE-prepared T1′ steps; the single-letter whole-cap
  three-segment form of COV-12; Theorem C instances. Units that land on the way count in the landing set. Each
  instance checks its whole-content and free-class hypotheses where needed, the admissibility of every required
  prefix, the joint preparation and vacancy conditions and the strict′ endpoint; a final V′⁻¹ is included only when
  justified or checked. N1 and N2 alone are the acceptance test only where the simultaneous group-action hypotheses
  are certified.
- Cover(Z) means that some instance passes at Z. It is finite: enumerate the bounded cores, the joint SAFE hole images
  and preparations, and the exact joint vacancy components at the resulting middles (COV-10 gives the exact
  preparation test, its flow criterion a shortcut).
- Theorem (conditional). If every Z ∈ 𝒞₂ with n(Z) > 0 satisfies Cover(Z), then every such Z is restored in at most
  n(Z) strict′ steps: each accepted step lowers n by at least one, keeps the surviving off poses and ends in 𝒞₂, so no
  further successor invariant is needed. The bounds stay those of R22C3-06: b = 13,205! − 1, N = 177,120,
  R = 7,200^N, m = 35,400, g = 6,600, C_b = (m^(b+1) − 1)/(m − 1), T_R = (g^R − 1)/(g − 1), L = 4b + 5(R − 1) and
  S(n) ≤ n·C_b⁴·T_R⁵. The templates fit the envelope of four cores and five retained segments (a preparation uses a
  retained slot, and a whole-cap core has length one); shortening a core must keep its complete five-label action, and
  shortening a retained segment its complete labelled endpoint.
- What is established. E2 gives passing instances along one trajectory, not the lemma; the reviewer independently
  evaluated only the b15 predecessor of COV-09, not all 21 steps. FC gives a mover, not a core or the resources;
  COV-13 refutes the all-content shortcut but has an inverse-history step; COV-14's state is not known to be
  reachable. Neither a proof of B′(2) nor a reachable deadlock of the full catalogue is established. The open lemma
  is Cover(Z) on 𝒞₂.
- Uses: COV-03, COV-10, COV-12, R22C3-06, R22C-K, R22C-FC.

### COV-16. The adjudicating experiments are done; E5's inputs were wrong (experiment design)

- The two bridge lemmas of the packet are decided by COV-09 (resources) and COV-13 (mixed frames), both run to the end
  (167 s and 826 s of CPU on one thread each, in the reviewer's runs). The universal lemma Cover(Z) is not decided by a
  bounded replay survey: an E5 success adds a trajectory, an E6 success gives step counts (lower bounds when the search
  is incomplete), and a stall, a failed greedy preparation, a failed certificate builder or a timeout shows no
  deadlock. A negative certificate must exhaust the stated cores and joint-action alternatives, or prove an
  obstruction that covers them all.
- Input correction (the integrator's error). The round 2 packet named the census round 3 records as depth 40 stops of
  𝒞₂. Their recorded end heights are {2: 74,541, 4: 513} (`ia-k200-s7105`), {2: 27,088, 4: 111} (`ia-k100-s7108`) and
  {2: 72,396, 4: 607} (`ia-k400-s7109`): deeper scrambles with height-4 pieces, outside 𝒞₂. The integrator confirmed
  this from the records (`11-scramble-census/runs/round3/`, field `heights_end`). The 𝒞₂ census stops on record are
  `round2/ia-k40-s7101` (E2's input) and `round2/ia-k60-s7104` (heights {2: 14,610}).
- Next. Decide the reachability of COV-01's state (a reaching history or an invariant that excludes it), and attack
  the Cover(Z) lemma itself.
- Uses: COV-09, COV-13, COV-15.

### Round 3 of coverage

The senior reviewer took up the two items round 2 left open (findings COV-17 to COV-22, all `minor`, all adopted,
fast tier, 18 October 2026): whether COV-01's isolated triangle X is reachable, and what the answer does to Cover(Z).
Its computations are `scripts/round22/cov17_single_off.py`, `cov18_singleton_kinds.py` and `cov19_perfect_menu.py`;
the integrator re-ran them and got the same digests, certificates and counts, with the outputs
`runs/round22/cov17_single_off.txt`, `cov18_singleton_kinds.txt` and `cov19_perfect_menu.txt`. Its experiment is
`scripts/round22/cov22_mixed_search.py`. Reachable states with exactly one off piece exist in every kind of cell, and
the catalogue as it stands covers them (COV-17, COV-18, COV-21); no abelian invariant can exclude X (COV-19); one
family of words provably misses X (COV-20); X's reachability stays open.

### COV-17. A legal 24-letter word reaches exactly one off piece (computed)

- The word. With chronological commutators [A, B] = A·B·A⁻¹·B⁻¹, A = (74, 8), B = (188, 4), D = (178, 1) and
  J = (0, 12), the word J·[[[A, B], B], D]·J⁻¹ has 24 letters. Its 22 middle letters are SAFE at solved·J, every prefix
  is applied exactly, and the middle keeps the off fingerprint of solved·J.
- The endpoint (digest `16d61ac2…`) has exactly one off piece, 58388, of type 17 and left height 2, in the class-0
  triangle cell {0, 32, 45}; its only hole is 58388. Of the 2,976 pieces with nonidentity poses, all but 58388 are on
  the lattice. The inverse word, applied exactly, returns the solved digest.
- So the smallest positive off count at a reachable state is 1, inside 𝒞₂; the smallest the integrator had found in
  the stored records was 3, at stage 01's E2 (one block of three pieces; [stage 01](../01-closure-x-to-e3/README.md)).
  The witness does not settle X: its piece is of type 17, X's of type 18. It also shows that one member of a home-cell
  cohort can carry a nonzero frame label while every other member is on the lattice, so no unrestricted form of
  Lemma G forbids a singleton; Lemma G (R22C-G) holds in its free-class scope.
- Command: `scripts/round22/cov17_single_off.py`; output `runs/round22/cov17_single_off.txt` (9 s of CPU).
- Uses: R22C-G.

### COV-18. Singleton off contents exist in all four kinds of cell (proved from computed certificates)

- The construction. Let J = (0, 12), C the 3,097 chambers of cap 0 and Y = solved·J. At Y every piece of C is off,
  every other piece is on the lattice and the hole set is C; the SAFE group has 562 grips, 6,182 letters and 1,816
  chamber orbits, 1,781 of them singletons. Suppose G(Y) contains a three-cycle (h a b) that fixes every other chamber,
  with h ∈ C and a, b ∉ C, and let V be a SAFE word that realizes it. Then the holes of Y·V are (C∖{h}) ∪ {a}, so one
  lattice piece is inside cap 0, in chamber h. J⁻¹ lands all 3,097 original off pieces and takes that piece off:
  solved·J·V·J⁻¹ has one off piece, in the region J⁻¹(h), of left height 2, and one hole, a. Every prefix is legal: V
  uses Y's SAFE alphabet, which keeps the original off pieces, and J⁻¹ meets only those pieces, still inside cap 0, and
  lattice pieces.
- The certificates. For each row a global Jordan certificate (O5) was checked: its prime p satisfies d/2 < p ≤ d − 3
  and divides exactly one cycle length on all 177,120 chambers, so a power of the word isolates a p-cycle on that
  orbit. The orbit action is primitive and contains Alt(d), and the normal closure of the isolated p-cycle contains
  Alt(d) on that orbit alone, which gives the three-cycle.
  - Vertex: h = 2335, a, b = 809, 813; an orbit of 3,555 chambers with 6 holes; type 9; the cell {0}, which has home
    chambers; p = 3,083; a certificate word of 400 letters.
  - Edge: h = 25, a, b = 561, 571; 1,434 chambers, 18 holes; type 13; the cell {0, 54}, with home chambers; p = 797;
    120 letters.
  - Triangle: h = 75, a, b = 607, 623; 1,440 chambers, 36 holes; type 26; the cell {0, 45, 54}, without home
    chambers; p = 863; 120 letters.
  - Tetrahedron: h = 116, a, b = 644, 909; 120 chambers, 4 holes; type 35; the cell {0, 32, 45, 54}, without home
    chambers; p = 79; 120 letters.
- So each kind of cell has a reachable state whose one off content is one piece. X's case, a type-18 piece in a
  triangle cell with home chambers, is not among them. Bound: |V| ≤ 7,200^174,023 − 1, since the 3,097 off poses stay
  fixed through V and each of the other 174,023 pieces has at most 7,200 lattice poses; the reaching word has at most
  7,200^174,023 + 1 letters.
- Command: `scripts/round22/cov18_singleton_kinds.py`; output `runs/round22/cov18_singleton_kinds.txt` (35 s of CPU;
  it also prints the checks of COV-20).
- Uses: O5 (SAFE-ALL and Jordan certificates).

### COV-19. No homomorphism to an abelian group tells X apart (computed; proved)

- The check. The 60 I_a elements at grip 0 form a group that acts faithfully on the 120 class-0 poles (the poles 0, 2,
  9 and 20 span R⁴) and is perfect: all 60 elements are commutators. Every other grip's menu is its conjugate by the
  grip's frame, and all 600 frames lie in K⁺.
- So every homomorphism χ from the group Γ that the menus generate to an abelian group vanishes on each grip's menu,
  hence on every generator, hence on Γ. Lead (i) of the packet, an orientation sum of χ over the pieces weighted by
  type or class, cannot tell X from a reachable state, whatever the cap populations are; this is stronger than χ
  vanishing on K⁺. A position-dependent frame cocycle, an invariant built from stabilizers or a nonabelian invariant
  is not excluded; each needs its own definition and a proof across all legal mixed-class moves.
- Command: `scripts/round22/cov19_perfect_menu.py`; output `runs/round22/cov19_perfect_menu.txt` (9 s of CPU; it
  rebuilds X first).
- Uses: COV-01.

### COV-20. X is outside the SAFE sandwich family of J = (0, 12); its reachability stays open (computed)

- The obstruction. With J, Y and C as in COV-18, J·R lies in K⁺ and maps chamber 1474 to chamber 2149. A state of X's
  family at the end of J·V·J⁻¹ would need a lattice piece in chamber 2149 just before J⁻¹. At Y, 2149 is a hole in a
  singleton orbit of G(Y), so every SAFE V leaves it vacant: no word of this form reaches X's family, whatever V's
  length or the final hole.
- The type-18 holes of cap 0 at Y, by the cell of J⁻¹(h) and by whether h's SAFE orbit also holds occupied chambers
  (wholly vacant, mixed): vertex cells with home chambers 18 and 6; edge cells with home chambers 15 and 33; triangle
  cells without home chambers 3 and 24; triangle cells with home chambers 9 and 0. So the nine sources in X's case all
  lie in wholly vacant orbits and none can take a lattice piece in. These are finite checks at this Y, not a
  classification of reachability.
- What stays open. No invariant that excludes X's family under every legal history was found, and no reaching word.
  The orbit obstruction does not extend to an invariant, since further jumble letters change the off geometry and its
  SAFE group. COV-17 shows that the unrestricted singleton prohibition of the packet's lead (ii) is false; Lemma G
  constrains free words of one class and gives no law for mixed-class moves. An invariant special to type-18 pieces
  in triangle cells with home chambers is still possible and unproved.
- Command: the X lines of `scripts/round22/cov18_singleton_kinds.py` (`X_TARGET_FIXED`, `TYPE18_SINGLE_J`); output
  in `runs/round22/cov18_singleton_kinds.txt`.
- Uses: COV-01, COV-14, COV-17, COV-18, R22C-G.

### COV-21. The constructed singletons are covered by the catalogue as it stands (proved)

- The subclass. L_J is the set of states Z = solved·J·V·J⁻¹ with J = (0, 12), V SAFE at Y = solved·J and n(Z) > 0.
  It is defined by a history; it is not proposed as an invariant of reachable states.
- Every Z ∈ L_J satisfies Cover(Z) (COV-15). Apply J, then V⁻¹, then J⁻¹. J returns to Y·V and lands every off piece
  of Z; at that middle the off geometry is exactly Y's, so V⁻¹ is SAFE there and restores the hole set C, which
  vacates cap 0; J⁻¹ then returns to the solved state. This is COV-12's three-segment whole-cap form with an empty
  preparation, and a strict′ step, since its endpoint is solved. It covers the four singletons of COV-18, and COV-17's
  witness, whose 24-letter inverse word is the step.
- Bounds. Shortening V to its complete labelled endpoint gives |V| ≤ 7,200^174,023 − 1 ≤ R − 1. The step uses two
  one-letter cores and one retained segment, so it fits L = 4b + 5(R − 1) and the S(n) of R22C3-06; no native
  mixed-class core is needed.
- What it does not prove: Cover(Z) for every singleton state, the lemma on 𝒞₂, or X's case. If a reaching word H for
  X's family is found, the geometric catalogue fails there (COV-01, COV-14), and adjoining the one native core H⁻¹
  covers that instance; with |H| ≤ b the counting bound needs no change, otherwise b becomes max(b, |H|) for it. Neither
  gives a uniform bound over arbitrary mixed-class histories.
- Uses: COV-01, COV-12, COV-14, COV-15, COV-18, R22C3-06.

### COV-22. One bounded mixed-class experiment for X's family (experiment design)

- Design. COV-20 rules out a SAFE middle after one jumble letter, so the experiment samples 24-letter words
  J·[[[a, b], c], d]·J⁻¹ whose middle has an own-class jumble letter a and a foreign-class jumble letter c, with b and
  d retained letters, all at grips local to chambers 1474, 404 and 762. Every prefix is applied with the exact
  simulator, and only an endpoint whose sole off piece is 1474 at exactly R is accepted: a member of X's family,
  whatever its other lattice poses. Eight seeds, at most two at a time, each with a 1,700-second CPU cutoff (about 3.8
  CPU hours in all).
- What it decides. FOUND gives a legal reaching word and settles X's reachability positively; covering that state
  then needs the 24-letter inverse as a native core, within the present b. NO_HIT or CUT_SHORT decides nothing: the
  family is sampled, not exhausted, and no finite exhaustive reduction of unrestricted reachability is proved.
- Command: `scripts/round22/cov22_mixed_search.py <seed>` for the seeds 0 to 7. The integrator started the eight runs
  on 18 October; their outputs go to `runs/round22/cov22/` when they end.
- Uses: COV-01, COV-17, COV-20.

### The integrator's checks after COV-20: no one-letter SAFE sandwich, and the second level (computed)

- The claim. For no letter L and no word V over the SAFE alphabet of solved·L is solved·L·V·L⁻¹ a state of COV-01's
  family. COV-20 shows it for L = (0, 12); COV-22's design assumes it for every L.
- The proof. A SAFE word keeps every off piece and maps the lattice to the lattice, so at W = solved·L·V the off pieces
  are the pieces of L's cap, in their poses after L, and L⁻¹ lands them all; the off pieces of the endpoint are the
  lattice pieces that W has in the chambers of L's cap, turned by L⁻¹ (if L lies in K⁺ the endpoint is on the
  lattice). A state of the family has one off piece, in X's off region, so W has one lattice piece in L's cap, in a
  chamber x that L⁻¹ takes to that region: L lands X's piece. L is legal at the endpoint, whose only off piece is X's
  (lattice pieces straddle no cut), so its cap is unstraddled at X too, and L is one of the 36 whole-cap letters of
  COV-14, at grips 0, 2 and 23, with x = (L·R)(1474). At solved·L every one of these chambers x is a hole in an orbit
  of one chamber of the SAFE group, so every SAFE word leaves it vacant.
- The computation. For each of the 36 letters, solved·L has 3,097 off pieces and 3,097 holes (the chambers of L's
  cap), 562 SAFE grips with 6,182 letters and 1,816 orbits, and its x is a type-18 hole in a singleton orbit (33
  distinct chambers; x = 2149 at (0, 12), as in COV-20). It agrees with COV-14: N1 fails at the middle X·L of every
  one of the 36 letters, and the SAFE alphabet of solved·L is contained in that of X·L, whose off pieces are those of
  solved·L but one, so x is fixed at solved·L as well. Command: `scripts/round22/cov20_all_landers.py`; output
  `runs/round22/cov20_all_landers.txt` (814 s of CPU).
- The second level (running). For L = (0, 12) and (0, 25), `scripts/round22/cov20_two_level.py` applies each lattice
  letter g at the twelve caps other than L's that are unstraddled and hold off pieces at solved·L, and prints the SAFE
  orbit of the chamber x′ = g(x) (x itself when g's cap does not contain x) from which g⁻¹ brings content to x. The
  integrator started both runs on 18 October; their outputs go to `runs/round22/cov20_two_level-0-12.txt` and
  `cov20_two_level-0-25.txt` when they end.
- Uses: COV-01, COV-14, COV-20.
