# Card 5 for the menu I_a: Questions 1 and 3 (final, second edition)

Final results of 14 October 2026, second edition. This package supersedes the first edition
`Claude-Card5-Ia-final-20261014` and the interim package `card5-q1-q3-3e-20261014`. Scope: Questions 1 and 3 of
Card 5 for the menu I_a only. Question 2, the menus S4_0 and I_b, and formalization are out of scope. Section 5
keeps the short 3E interface note of the interim package; the 3E line itself is reported in the package
`Claude-3E-seed-table-20261014`.

**Evidence kind.** Exact computations in synthetic geometry on the jumbling simulator of the research tree
(`research/jumbling`, branch `claude/jumbling-e3`, commit `27a001c`), run headless on Linux. Nothing here is
Windows, DirectX, native-host or performance evidence. The new proofs in this document have not been
independently reviewed.

**Labels.** *proved*; *proved under H* (H is the tree model T3 to T7 given on the card); *certificate* (a finite
exact computation; its objects, script and output file are named in Section 6, and this document claims no more
than the output file records); *conditional* (the condition is named); *conjectured*; *open*.

**Notation.** Orbits are the `orbit_id` values 0..34 of `assets/model.npz`; orbit -1 is the centres. For a
reachable lattice configuration Y, nu(Y) = (nu_o)_o with nu_o = (sign, frame) in Z/2 x H_o^ab. Coordinates:

- b_o, o = 0..34: the position sign of orbit o;
- f_o, o in {0, 3, 6, 8, 11, 22, 25, 32}: the C2 frame (for orbit 6 the abelianized D5 frame);
- C5_17, C5_28: the C5 frames; orbit 34 has frame group A5, whose abelianization is trivial.

So A = C2^43 x C5^2. nu is computed with the coherent-frame coordinates of the item-2 package (`nucoords.py`).
A_J is the set of nu(Y) over reachable lattice configurations Y; by S it is also the set of nu(w) over words w
admissible from solved that end on the lattice. A slot is a chamber other than the core; a hole of a
configuration is a slot occupied by no lattice piece; Q denotes the off-lattice pieces.

## 0. Status of the seven sub-questions

| Item | Status after this work |
|---|---|
| 1.1 finiteness | **open; delivered as open, with proved partial bounds.** Proposition HB (proved; certificate REFX): a piece of model orbit 0, 1, 2, 3 or 6 never passes left height 2, a piece of orbit 4, 5, 7 or 8 never passes height 4, and a piece of orbit 9 never passes height 6. Height 6 is reachable (certificate H6; there only pieces of orbits 13 and higher reach it). Height 8 was never reached: not by the one-twist extensions of the fixture history (COVER6X, complete under H; H6-MORE) nor by the targeted searches of Section 4.1 (TWOSTEP, CLEAR78, PUMP, SINGLE, RWALK). All evidence points to bounded heights; no bound is proved for orbits 10 to 34. |
| 1.2 Safe | answered in the handoff, under H (600, 120 and 0 elements at heights 0, 2 and at least 4); not redone. |
| 1.3 normal form | **proved for heights 2 and 4** (certificate HEIGHTS4 with Lemma E); open from height 6 (240 of the 5,120 vertices reached, certificates COVER6 and COVER6X). If left heights are bounded, the normal form fails at every larger height; by Proposition HB it already fails for each of the model orbits 0 to 9 taken alone. |
| 3.1 A_J | **answered: A_J = C2^34 x C5^2 exactly** (Theorem LB with certificate CERT-LB-V2 for the lower bound; Theorem A and Theorem B5 with certificate POT5 for the upper bound). |
| 3.2 sign 8 = sign 25? | **answered: no.** The 9-twist word M8 has sign 8 odd and sign 25 even (Theorem 3.2). |
| 3.3 frames zero? | answered in the handoff (no: f32). Here f8, f11, f22, f25 and the C5 frames of orbits 17 and 28 are also nonzero on A_J. |
| 3.4 conjugated half-turns enough? | answered in the handoff, under H (no). Refined here: their nu values generate exactly <b4, b7, b8, ..., b34> = C2^29 (the lower bound by CERT-LB-V2; the upper bound under H, with Theorem B5 excluding b5). |

By sub-question count, 5 of 7 are answered (1.2, 3.1, 3.2, 3.3, 3.4); 1.1 and 1.3 have proved partial results and
are delivered as open.

Changes against the first edition: Theorem B5 (Section 2.4) proves b5 = 0 and closes 3.1. The first edition's b5
material (Lemma U, Proposition U', certificate UNITS, now also run on four deep fixture states) is kept in
Section 2.4.1 as an independent check. Section 1 adds the tree facts behind both new parts (certificate TREEFACTS).
Section 4.1 adds Proposition HB, the alignment law AL, Lemma L, Lemma FN and new negative evidence; Section 4.3
records the consequence for 1.3. Section 6 adds the new certificates.

Changes in the first edition against the interim package: f8 and C5_17 entered A_J (Section 2.1), which shrank the
gap between the bounds from index 20 to index 2; the condition R_E3 was removed, because the 3E package replays the
whole history of E3 from solved (its certificate CERT-HIST); Lemma V' and Proposition U' were added; Section 4
added certificate COVER6X.

## 1. Tools

**F1 (certificate, item-2 package).** nu of every one of the 1,200 retained primitive twists is zero on all 35
orbits (`check-report.json`, check `F1_nu_zero_on_generators`, 0 failures). Since nu is a homomorphism on the
labelled-state group and a retained twist acts on labelled lattice states by right multiplication by its own
relabelling, nu(Y R) = nu(Y) for every lattice configuration Y and every retained word R.

**S (given).** Admissibility and the action on posed regions depend only on the shape (the set of posed regions).

**Lemma V (vacancy formula; proved).** Let U be admissible from solved, with off set Q and hole set H at its end.
Let R be a retained word such that no cap of a letter of R contains or is straddled by a posed region of a piece
of Q at the moment the letter is applied, and such that the slot permutation rho_R of R (its action on the solved
configuration) maps H onto H. Write c_R(s) for the frame value of R at slot s (coherent frames, abelianized).
Then U R U^-1 is admissible from solved, ends on the lattice, and for every orbit o

    nu_o(U R U^-1) = ( sgn(rho_R restricted to H_o),  - sum over h in H_o of c_R(h) ).

*Proof.* (1) Pieces of Q are never inside a cap of R, so R never moves them. Lattice regions never straddle a cut
(cuts are unions of chamber faces) and Q-regions do not by hypothesis, so every letter of R is admissible; a
retained letter maps the chambers of its cap onto chambers of its cap, so after R the lattice pieces occupy the
slots rho_R(slots minus H) = slots minus H. The shape after U R is the shape after U. (2) By S, U^-1 is admissible
after U R and moves the same regions as after U. A piece of Q follows the same trajectory as in U U^-1 and ends
with the identity pose. A lattice piece p sitting after U R in the region that p' occupied after U ends with pose
k_{p'}^-1 k_p, a product of two K+ elements, so the end is on the lattice. (3) Let sigma map each lattice piece to
its slot after U. The end relabelling restricted to the lattice pieces of orbit o is sigma^-1 rho_R sigma on
slots_o minus H_o, and pieces of Q_o are fixed. Conjugation keeps the sign, and the frames telescope to the sum of
c_R over slots_o minus H_o. By F1 applied to R on solved, sgn(rho_R on slots_o) is even and the sum of c_R over
slots_o is zero; subtracting the part on H_o gives the formula. QED

**Corollary D (doubling; proved from Lemma V).** Let C be admissible from solved, with off set Q and hole set H.
Let d be a grip whose cap contains no piece of Q and is straddled by none, and a in A4_d a half-turn with
a(h) != h for every hole h of C in cap d. Let C^a be C conjugated letter by letter by a (letter (c, g) becomes
(a(c), a g a^-1), an I_a letter by menu equivariance). Suppose U = C C^a is admissible from solved with off set
Q union a(Q) and hole set H union a(H), H and a(H) disjoint. Then w = U (d, a) U^-1 ends on the lattice and

    nu(w) = sum of b_{orbit(h)} over the holes h of C in cap d   (all frames zero).

*Proof.* a fixes n_d, so a(Q) is also outside cap d, and (d, a) swaps h and a(h) for every hole h of C in cap d
and fixes every other hole of U. Lemma V gives one transposition per such h; the frame contributions pair to
c_a(a h) + c_a(h) = c_{a^2}(h) = 0. QED

**Corollary L (frame loop; proved from Lemma V).** Let C be admissible from solved, with off set Q and hole set H,
and h a hole of orbit o. Let R be a retained word whose caps never contain a piece of Q or a slot of H minus {h},
are never straddled, and with rho_R(h) = h. Then C R C^-1 ends on the lattice and nu = (0, -c_R(h)) on orbit o,
zero elsewhere. (Every hole other than h is fixed with trivial frame.) A nonzero value needs c_R(h) != 0: the
vacancy at h travels around a loop and comes back rotated by a nontrivial element of its slot stabilizer.

**Lemma V' (vacancy formula with moving off pieces; proved).** Let U be admissible from solved, with off set Q and
hole set H. Let R be a retained word admissible after U whose end has the shape of U (the same hole set, and the
pieces of Q occupy the same set of posed regions). Write rho_R for the slot permutation of R and kappa for the
permutation of Q given by: kappa(q) is the piece that occupies after U R the region that q occupies after U. If
U R U^-1 ends on the lattice, then for every orbit o

    b_o(U R U^-1) = sgn(rho_R restricted to H_o) * sgn(kappa restricted to Q_o).

*Proof.* As in Lemma V, with two changes. A retained letter maps the chambers of its cap onto chambers of its cap
and the posed regions in its cap onto posed regions, so after U R the lattice pieces occupy the slots
rho_R(slots minus H) = slots minus H and the pieces of Q occupy the regions of Q. By S, U^-1 is admissible after
U R and moves the same regions as after U, so a piece that sits after U R in the position (slot or region) of a
piece p' after U ends in the home of p'. Hence the end relabelling of orbit o is conjugate to rho_R on
slots_o minus H_o together with kappa on Q_o (regions of different orbits are not congruent, so kappa preserves
orbits), and its sign is sgn(rho_R on slots_o minus H_o) sgn(kappa_o) = sgn(rho_R on H_o) sgn(kappa_o), because
rho_R is even on slots_o by F1. QED

Lemma V is the case where R moves no piece of Q. Here off pieces inside the caps of R may move; Proposition U'
(Section 2.4.1) uses this.

**Lemma E (equivariance; proved).** For k in K+, conjugating every letter of a word by k (grip c to k(c), element
g to k g k^-1) gives an I_a word that is admissible exactly when the original is, and its end configuration is
the k-conjugate of the original end. A pose [l, r] becomes [u l u^-1, v r v^-1] for k = [u, v], so the left
position t_L moves to u t_L with u in 2I. Hence the set of reachable left positions at each height is a union of
2I-orbits. (On the lattice the conjugate relabelling is the original one conjugated inside the wreath product, so
nu, which takes values in an abelian group, is unchanged.) The menu equivariance a Lambda_c a^-1 = Lambda_{a(c)}
for a in K+ is used throughout.

**Tree facts (certificate TREEFACTS; exact).** A rotation of R^4 is written [l, r]: x -> l x r^-1, with quaternions l
and r over Q(sqrt 5) determined up to non-zero scalars. Quaternions act by left multiplication on the Bruhat-Tits
tree of the quaternion algebra over Q(sqrt 5) at the inert prime 2, a 5-regular tree; v0 is the vertex fixed by the
icosian units 2I, and the *left height* of a pose [l, r] is d(l v0, v0) (scalars act trivially). H, the tree model
T3 to T7 of the card, describes this action; the statements below are exact computations, and the results of this
document that use only them do not need H. Certificate TREEFACTS (`treefacts.py`):

- T1: the normal n_c of every grip c moves v0 to a neighbour w_c = n_c v0; the 600 grips fall into 5 *branches* of
  120 according to w_c.
- T2: the 120 unit icosians fix v0.
- T3: for every grip c and each of its 60 menu elements, the right factor is a scalar multiple of a unit icosian;
  for the 12 retained elements so is the left factor; for the 48 jumble elements the left factor l satisfies
  d(l v0, v0) = 2, lies in the branch of c and fixes w_c (36,000 elements, no failure).
- T4: the right factor of each of the 1,183 distinct poses of the H6 table is a scalar multiple of a unit icosian.
- T5: every element of Lambda_0 maps the normal of every grip of the branch of grip 0 onto the normal of a grip of
  that branch (7,200 exact pole images); by menu equivariance the same holds at every grip.

A pose is a product of menu elements, so by T3 its right factor is a scalar multiple of a unit icosian. Hence the
K+-class of a pose (its orbit under left multiplication by K+ = {[u, v] : u, v in 2I}) is determined by the coset
2I l of its left factor, and the left height is a function of the class. For a piece p this applies to its pose
relative to the representative chamber of its orbit (its pose composed with the K+ transport of its home chamber);
this is the class used in Lemma R (Section 2.4) and Proposition HB (Section 4.1).

## 2. Question 3

### 2.1 Theorem LB (lower bound; certificate CERT-LB-V2)

The 40 words of `words/generators.jsonl` are admissible from solved at every prefix, end on the lattice, and have
the nu values recorded there (computed with the coherent frames; the 35 signs are recomputed independently from
the final poses by pole permutations). Their nu values span, over F2, the subspace

    V34 = < b4, b7, b8, ..., b34, f8, f11, f22, f25, f32 >   (dimension 34),

and the words FR28-0 and FR17-0 have nu equal to a nonzero C5 frame of orbit 28 (value 1) and of orbit 17
(value 4), all else zero. Hence V34 x C5_17 x C5_28 <= A_J and |A_J| >= 2^34 x 25. With Theorem A:

    C2^34 x C5^2 <= A_J <= C2^35 x C5^2,

and the two bounds differ exactly in the coordinate b5. Theorem B5 (Section 2.4) closes the gap:
A_J = C2^34 x C5^2.

The words: the 13 words M0 to M12 of the lattice-correction library (M0 is w9; M1 to M10 are conjugated
half-turns of length 5 or 9; M11, M12 are difference words of length 228 and 332 with frame f11); the two words
of the accepted handoff fact d0c9bda0a984e204 (3,720 and 3,902 letters, pure f32), replayed here; the unreviewed
handoff candidate W* (1,447 letters, pure b32), replayed here and not needed for the rank; and the words of this
work:

- pure position signs by Corollary D from single-hole words: b33 (349 letters), b27 (1,405), b26 (157), b31 (317),
  b32 (701), b30 (1,277), b29 (2,557), b25 (5,629), b24 (317);
- pure frames by Corollary L from single-hole words: f25 (FR-0, 2,816 letters; a loop of two retained twists at
  grips 195 and 235), f22 (FR22-0, 2,816 letters; a loop at grips 27 and 104), the C5 frame of orbit 28 (FR28-0,
  1,408 letters; a loop at grips 449 and 468), and, new in this package:
  - **FR8-0** (128 letters; nu = f8): the orbit-8 single H32-21 (63 letters; its one off piece has model orbit 8,
    position class 452, hole 14265), the loop R = (27, item 3) (33, item 7), then the inverse of the single;
  - **FR17-0** (512 letters; nu = C5 frame of orbit 17 equal to 4, a generator of C5): the orbit-17 single H31-1
    (255 letters; position class 59, hole 100057), the loop R = (291, item 2) (366, item 4), then the inverse;
- eight doubling words of length 37 or 85 from multi-hole preparations (Corollary D), which close the remaining
  sign relations;
- two words of 9 letters with sign 4 odd (B4-0, B4-1; Corollary D with the two-letter preparation
  C = (0, x) (e, r): the jumble x = item 23 at grip 0, then a retained twist at a grip e at 36 degrees whose cap
  is clean and holds two vacated orbit-4 slots; the doubling grip g then holds exactly one orbit-4 hole of C).

The single-hole words were found by a greedy refill descent: for a preparation C and a retained twist (g, a)
whose cap is clean, C (g, a) C^-1 leaves off the lattice the pieces that (g, a) moves into holes; when exactly one
remains, the word is a single. Singles were found for hole orbits 19 and 22 to 33. The orbit-8 and orbit-17
singles were found later by the symbolic seed harvest of the 3E line (`scripts/harvest.py`: nested conjugates
U r U^-1 tracked by the vacancy formula, every single replayed exactly), restricted with
`--want data/want-8-17.json` to the six position classes whose off piece has model orbit 8 or 17, with seeds 31
and 32 (3,500 s each, `--per 4 --beam 32 --levels 8`; logs in `certs/f8-c517/`). `scripts/tomodel.py` rewrites
the orbit fields to model orbits (`words/singles-orbit8.jsonl`, `words/singles-orbit17.jsonl`), and
`scripts/frame_track.py` (depth 6) finds the loops (`words/frame-loops-8-17.jsonl`). The interim searches for f8
and C5_17 had started from preparations with several holes; a single-hole word was the missing ingredient, as
Corollary L needs. The searches are heuristics; only the replayed words are evidence.

### 2.2 E3 and the 3E restorations (unconditional)

**Proposition 2.2.** Let E3 be the end of the X to E3 chain of the earlier 3E bundles. Its exact pose table (digest
checked) is on the lattice and

    nu(E3) = b9 + b10 + b11 + b13 + b14 + b15 + b20 + b25 + b26 + b30 + b31 + b33 + f22 + f25 + f32 + 2 C5_17

(certificate TABLE-NU; the same certificate gives the I1 and I2 ends of the three-block family, which differ
from E3 by f11 or by nothing). The interim package needed the condition R_E3 (E3 reachable from solved). The 3E
package `Claude-3E-seed-table-20261014` removes it: its certificate CERT-HIST replays the whole history of E3 from
solved (39,392 letters, every letter admissible, every segment end digest equal to the recorded one). So nu(E3)
is in A_J unconditionally; its value lies in V34 + <C5_17>, consistent with Theorem LB (FR17-0 gives C5_17
directly).

The 3E restorations of E2, I1 and I2 (all off pieces returned to the lattice; runs E2, I1, I2 and I2-noself of
the 3E package) are further reachable lattice configurations. Certificate TABLE-NU-3E: the E2 and I2 ends have
nu(E3); the I1 end has nu(E3) + b12 + f11; the I2 end restored without its own inverse word has nu(E3) + b12.
All lie in V34 + <C5_17>; in particular b5 = 0 on all of them.

### 2.3 Theorem A (upper bound; proved, using certificates C2, C3, C4 and F1)

**Theorem A.** For every reachable lattice configuration Y and every orbit o in I'' = {0, 1, 2, 3, 6},
nu_o(Y) = 0 (sign and frame). Hence A_J <= A'' := {b0 = b1 = b2 = b3 = b6 = 0, f0 = f3 = f6 = 0} = C2^35 x C5^2.

Write N15(c) for the 4 grips at 15.52 degrees from c and N25(c) for the 12 grips at 25.24 degrees. Inner slots
and inner pieces are those of orbits in I''. Facts (stated for grip 0 and transported to every grip by
K+-transitivity and menu equivariance):

- **C2 (certificate G2).** For every jumble element x of Lambda_0 (48) and every inner slot s of cap 0, the grips
  d != 0 whose closed cap wholly contains x(A_s) are: none for orbit 0; only grips of N15(0) for orbits 1, 3, 6;
  only grips of N15(0) union N25(0) for orbit 2. (Exact Q(sqrt 5) evaluation at every near-tie vertex.)
- **C3 (certificate SHARE).** A grip d != 0 has an inner slot of cap 0 in its cap exactly when d is in
  N15(0) union N25(0). Each grip of N15(0) shares the inner slots {0: 1, 1: 6, 2: 6, 3: 9, 6: 3} with cap 0, each
  grip of N25(0) shares {1: 1, 2: 2, 3: 3, 6: 1}. A cap holds the inner slots {0: 4, 1: 18, 2: 16, 3: 24, 6: 6}.
- **C4 (conservation; certificate CONS).** If a twist at d is admissible from a configuration X, then for every
  r <= rho, sum over o of (m_o(X, d) - m_o(solved, d)) v_o(r) = 0, where m_o(X, d) counts the orbit-o pieces whose
  posed region lies in cap d and v_o(r) is the volume of an orbit-o chamber inside the ball of radius r. (Poses are
  rotations about the origin, so pieces keep their ball volumes; for r <= rho the ball lies in the body, the pieces
  tile it outside the core, and an unstraddled cap is tiled by the pieces inside it.) The certificate gives exact
  rational interior points z_-1, z_0, z_1 of a centre chamber, an orbit-0 chamber and an orbit-1 chamber with
  |z_-1|^2 < min over o >= 0 of lambda_o, |z_0|^2 < min over o >= 1, |z_1|^2 < min over o >= 2, all below rho^2,
  where lambda_o = (|S| kappa)^2 / |sum over c in S of n_c|^2 bounds |x|^2 from below on an orbit-o chamber with
  signature S. Taking r just above |z_-1|, |z_0|, |z_1| in turn: **m_-1, m_0 and m_1 are conserved by every
  admissible twist.**

**Invariant INV.** Every inner piece is either on the lattice or *anchored at an out grip c*: its pose is x_c k
with x_c in Lambda_c minus A4_c and k in K+, and k maps its home to an inner slot of cap c. For each out grip c the
pieces anchored at c (its *cohort*) are exactly the pieces that occupied the inner slots of cap c, one per slot
(complete), and those slots are holes. Two out grips share no inner slot.

**Blocking lemma.** Under INV, if c is out then no twist at d in N15(c) union N25(c) is admissible.
*Proof.* d is not out (it shares inner slots with c). If d is in N15(c): cap d has 4 orbit-0 slots, one shared
with c and vacant; anchored orbit-0 pieces lie wholly only in their own anchor cap (C2), so at most 3 orbit-0
pieces lie in cap d, and C4 (m_0 conserved, m_0(solved, d) = 4) forbids the twist. If d is in N25(c): one of its 18
orbit-1 slots is vacant; an anchored orbit-1 piece lies wholly in cap d only if d is in N15 of its anchor (C2), and
then the twist is forbidden by the first case; otherwise m_1 <= 17 < 18. QED

**Proof of Theorem A.** INV holds at solved (no out grips). Let (d, g) be admissible from X satisfying INV.
(i) d in N15(c) union N25(c) for an out c: impossible (blocking lemma). (ii) d far (not out, not in N15 or N25 of
an out grip): by C3 and INV every inner slot of cap d holds a lattice inner piece; by C2 no anchored piece lies
wholly in cap d. A retained g permutes these pieces within cap d; a jumble g anchors all of them at d, forming a
complete cohort whose slots become holes and which shares no inner slot with another out grip (C3). (iii) d out:
the inner pieces in cap d are exactly its cohort (another cohort inside cap d would put d in N15 or N25 of its
anchor, which shares inner slots with d). If g x_d is a jumble element the cohort stays anchored; if g x_d = a is
in A4_d the cohort returns to the lattice and refills its slots bijectively. INV holds after the twist.
Potential: for X satisfying INV let V(X) be the labelled inner state obtained by replacing every anchored pose
x_c k by k. Choosing x_c a with a in A4_c instead replaces k by a^-1 k (the actual pose x_c k is fixed), so it
changes V(X) by the retained twist (c, a^-1) on inner orbits, and psi(X) = nu restricted to I'' of V(X) is well
defined by F1. In case (ii) a retained twist changes psi by its own
nu (zero by F1) and a jumble leaves V unchanged; in case (iii) an anchored step leaves V unchanged and a return
changes V by the retained (d, a). So psi is invariant along every admissible word, psi(solved) = 0, and on a
lattice configuration no piece is anchored, so psi = nu restricted to I''. QED

### 2.4 The coordinate b5 (Theorem B5; proved, with certificate POT5)

**Theorem B5.** b5(Y) = 0 for every reachable lattice configuration Y. With Theorem LB and Theorem A:

    A_J = V34 x C5_17 x C5_28 = C2^34 x C5^2   exactly.

The proof builds a potential: a map pi from the posed orbit-5 regions that can occur to the orbit-5 slots such that
every admissible twist acts on the pi-images of the orbit-5 pieces by an even permutation of slots, and pi is the
identity on lattice regions. The sign of the pi-image permutation is then invariant, and on a lattice
configuration it is b5.

**Notation.** Orbit 5 has 7,200 chambers (K+-orbit index 7 of the region list); K+ acts on them simply
transitively. For a grip d, F5(d) is the set of the 12 grips at 36 degrees from d that share an orbit-5 chamber with
cap d (one of the two 36-degree families of SHARE36). A posed region of orbit 5 is written [l, r](A_rep) with
A_rep the representative chamber; right factors are units, so its K+-class is the coset 2I l (Section 1).

**Lemma R (refined dynamics; proved from Theorem A).** Fix a piece p of any orbit. Its *refined state* is
(j, O, cpl): j the K+-class of its pose relative to the representative chamber (Section 1; for orbit 5 this is
also the K+-class of its posed region, by S3 below); O the set of out grips (INV) among the grips whose closed caps hold
p; cpl = (c, j0) if c is in O, p was in cap c when the jumble that made c out was applied (p was then in class j0),
and since that jumble every twist that moved p was a twist at c; otherwise cpl is empty. Let R be the transition
system on refined states with the rules R1 to R5 below, started at the lattice class with O empty. Then along every
admissible word from solved, the refined states of p after the twists that move p form a path in R, and twists that
do not move p leave the refined state unchanged.

*Frame convention.* The refined state keeps only the class j, so grips and twists are written in the frame of the
canonical region R_j of the class. When p's actual region is k R_j with k in K+, a physical grip d and a physical
twist (d, h) are written as d' = k^-1(d) and h' = k^-1 h k, and O and the grip of cpl are transported in the same
way. The rules below are read in this frame. `scripts/refexact.py` uses this convention: it writes O in the frame
of the canonical region of the class and transports grips by exact K+ permutations after each transition. (Without
it, R1 would be ill defined: for k in A4_d and h in Lambda_d with h k h^-1 outside A4_d, the poses 1 and k have the
same class and empty O, but the physical twist h gives the different classes K+ h and K+ h k. Stated after the
senior review of 15 October 2026.)

- R1 (move). A twist (d, h) moves p iff p lies in cap d; it is admissible only if d is not in N15(e) u N25(e) for an
  e in O other than d (blocking lemma). The class changes from j to the class of h R_j.
- R2 (retained at d). If h is in A4_d, the status of d is unchanged, and cpl is kept iff it is (d, j0).
- R3 (jumble at a grip that is not out). d becomes out with cpl = (d, j) (j the class before the twist).
- R4 (jumble at an out grip). If cpl = (d, j0): d returns exactly when the new class is j0, otherwise it stays out
  with cpl = (d, j0). If cpl is not (d, .): both outcomes are possible (branch), cpl is emptied.
- R5 (grips newly holding p). A grip whose cap holds p after the twist but not before may be out or not (branch over
  the subsets compatible with INV: no out grip in N15 u N25 of another, and none whose N15 u N25 contains d). Grips
  that no longer hold p are dropped from O.

*Proof.* The status of a grip changes only by twists at that grip (Theorem A, cases ii and iii), and a twist at a
grip whose cap holds p moves p; so between two moves of p the statuses of the grips holding p are constant, and the
grips entering O at a move are exactly those of R5, whose statuses R5 lets range over everything INV allows. R1 is
the blocking lemma. R2 to R4 are Theorem A case iii: a retained letter at an out grip keeps its cohort anchored;
a jumble at a grip that is not out anchors its cohort (case ii); a jumble at an out grip d multiplies its anchoring
element G_d by h, and the cohort returns iff h G_d lies in A4_d. If cpl = (d, j0), p and the cohort have moved
together since the anchoring jumble, so p's pose is G_d P0, with P0 its pose before that jumble (class j0). The
class is j0 again exactly when the left factor of G_d is a scalar multiple of a unit icosian, that is (its right
factor is one by T3) exactly when G_d lies in K+, hence in Lambda_d intersected with K+, which is A4_d: exactly when
d returns. (For orbit 5 the same follows from regions: class j0 gives k G_d R_{j0} = R_{j0} for some k in K+, hence
k G_d = 1 by S3.) QED

**Lemma T (even local permutations; proved).** For a grip d let e_d assign to every orbit-5 slot t of cap d a grip
e_d(t) in F5(d), K+-equivariantly (e_{k(d)}(k t) = k e_d(t)). For h in Lambda_d let tau_d(h) be the permutation of
slots that sends t in cap d to a(t), where a is the unique element of A4_d with a(e_d(t)) = h(e_d(t)), and fixes
every other slot. If Lambda_d maps F5(d) onto itself by even permutations and A4_d acts simply transitively on
F5(d), then tau_d(h) is an even permutation of the orbit-5 slots, and tau_{k(d)}(k h k^-1) = k tau_d(h) k^-1 for k
in K+.

*Proof.* Equivariance gives e_d(a t) = a e_d(t) for a in A4_d, so A4_d acts freely on the orbit-5 slots of cap d
(a t = t forces a e_d(t) = e_d(t), hence a = 1). On one A4_d-orbit O of slots, e_d is an equivariant bijection
O -> F5(d) between free transitive A4_d-sets, and for t in O, e_d(tau_d(h) t) = a e_d(t) = h e_d(t). So
tau_d(h) restricted to O is e_d^-1 h e_d, conjugate to the permutation h induces on F5(d), which is even. The
second claim is the K+-equivariance of e, A4 and the menus. QED

**Certificate POT5** (`scripts/potcert5.py`, output `certs/b5/pot5-cert.json`, exact Q(sqrt5) arithmetic):

- R_5, the refined dynamics of orbit 5 (`scripts/refexact.py`), is closed: 144 refined states, 49 classes at
  heights 0, 2, 4 (1, 16, 32), class digest `0a39e349...`.
- S1: every one of the 60 elements of Lambda_0 maps F5(0) = {2, 9, 20, 23, 30, 32, 43, 45, 46, 54, 56, 75} onto
  itself (exact pole images), always by an even permutation. S2: A4_0 acts simply transitively on F5(0).
- S3: the orbit-5 representative chamber has a trivial isometry group (its vertices have pairwise distinct
  Gram profiles and span R^4), so a posed orbit-5 region determines its class and its K+ frame.
- An association e, given on the 5 caps of the representative chamber ({0: 2, 10: 3, 12: 7, 13: 35, 35: 13}; for
  the lens pair (13, 35) the partner is forced) and transported by K+.
- A potential pi: each class j gets a K+ element t_j, and pi(k R_j) = k t_j(A_rep) for k in K+ (well defined by
  S3). The lattice class gets the identity frame, so pi is the identity on lattice regions.
- P: for every class j and every grip d that is unblocked at j in some refined state of R_5: (i) if R_j lies in cap
  d then pi(R_j) lies in cap d (65 checks); (ii) for each of the 60 rows h of Lambda_d,
  pi(h R_j) = tau_d(h) pi(R_j) (3,900 checks); (iii) if R_j lies wholly outside cap d then pi(R_j) is not in cap d
  (28,585 checks). All pass (8.3 s).

**Lemma P (potential; proved).** Let X be reachable from solved and, for an orbit-5 piece q, V(X)(q) = pi(R_q(X)),
R_q(X) its posed region. Then V(X) is defined, it is a bijection from the orbit-5 pieces onto the orbit-5 slots,
V(solved) is the home map, and an admissible twist (d, h) from X gives V(X (d, h)) = tau_d(h) o V(X).

*Proof.* By Lemma R every orbit-5 piece is, at every moment, in a class of R_5 (in some K+ frame), so V(X) is
defined. Let (d, h) be admissible from X and q an orbit-5 piece in class j with frame k (R_q = k R_j). No piece
straddles d. If q lies in cap d, the refined state of q has d unblocked (Lemma R: the twist is admissible), so
k^-1(d) is a grip checked in P; by (ii) and the equivariance of Lemma T, pi(h R_q) = tau_d(h) pi(R_q). If q lies
outside cap d, d is again unblocked in q's refined state (blocking is a property of the configuration), so by (iii)
pi(R_q) is outside cap d and tau_d(h) fixes it; R_q does not change. Hence V changes by the slot permutation
tau_d(h). At solved every piece is in its home chamber and pi is the identity on lattice regions, so V(solved) is
the home map, a bijection; bijectivity is kept because tau_d(h) is a permutation. QED

**Proof of Theorem B5.** Identify orbit-5 pieces with their home slots. By Lemma P and Lemma T, sgn V(X) is
multiplied by sgn tau_d(h) = +1 at every admissible twist, so sgn V(X) = sgn V(solved) = +1 for every reachable X.
At a reachable lattice configuration Y every orbit-5 piece occupies a slot and pi is the identity on lattice
regions, so V(Y) is the position permutation of orbit 5 and b5(Y) = 0. QED

**Validation (exact replays; not part of the proof).** `scripts/potval.py` recomputes V from actual posed regions
(left factor, class, frame) and checks bijectivity and the per-twist rule V(X (d, h)) = tau_d(h) o V(X) on every
admissible twist of four random walks from solved (600 attempted twists each; 116 to 130 admissible, up to 2,402 off
pieces) and on the deep fixture tables T200, T300, T723 and T723 + (433, item 12) (every orbit-5 region lies in a
class of R_5; V bijective). All pass (`certs/b5/val/`, certificate POT5-VAL). Proposition U' and its certificate
UNITS (Section 2.4.1) are now consequences for b5 and remain as the independent check that first located the
obstruction.

**Why orbit 5 differs from orbit 4.** The same construction fails for orbit 4: no association satisfies (ii) at all
classes of R_4 (certificate POT-O4, `certs/b5/pot-o4.json`: tau fails for each of the 1,728 associations), in line
with the odd orbit-4 words B4-0, B4-1 and VERIFY-LOOP.

#### 2.4.1 The first edition's route (kept as an independent check)

The first edition did not prove b5 = 0. Its material on b5 is kept because it was obtained independently of
Theorem B5, it located the obstruction, and its certificates remain valid checks: why the constructions that produce
the other coordinates fail for b5, Lemma U and Proposition U' with certificate UNITS (now also on four deep fixture
states), and the search evidence. Theorem B5 implies the conclusion of Proposition U' for o = 5.

- **Why Theorem A stops at orbits 4 and 5.** Orbits 4 and 5 lie just outside the inscribed ball
  (lambda/rho^2 = 1.0023), so C4 says nothing about them, and their slots are also shared with grips at 36 degrees
  (certificates SHARE45 and SHARE36), which the blocking lemma does not cover. SHARE36: the 24 grips at 36 degrees
  from grip 0 form two families of 12; each grip of the first shares 2 orbit-4 slots and no orbit-5 slot with
  cap 0, each grip of the second 2 orbit-5 slots and no orbit-4 slot. (Their counts for orbits 12 and 13, 18 and
  19, 26 and 27 are also exchanged.)
- **b4 (now in V34).** Certificate B45-ACCESS: after each of the 48 jumble twists at grip 0, the grips of N15(0) and
  N25(0) are straddled, 6 grips of the orbit-4 family are clean and 6 are straddled, and all 12 grips of the
  orbit-5 family are admissible but hold an off piece wholly inside their caps. A clean orbit-4 grip e gives the
  preparation C = (0, x) (e, r), and Corollary D then gives the two B4 words of Section 2.1.
- **Doubling after one jumble.** By B45-ACCESS no cap holding a vacated orbit-5 slot is clean after a single
  jumble. A targeted search (`b45_double.py`, mode `admissible`) used preparations C = (0, x) (e, r) with e an
  admissible grip of the orbit-5 family; the caps holding an odd number of orbit-5 holes of C were never clean
  (`certs/b5-search.json`), so Corollary D never applied. A second variant (`b5_double2.py`) also counted the off
  pieces of C = (0, x) lying wholly inside the doubling cap, which a doubled half-turn swaps in pairs; after every
  single jumble, no admissible cap had an odd orbit-5 count. Proposition U' below explains this.

**Members and self-locked sets.** Fix an orbit o. The *members* of a configuration are its off pieces (of every
orbit) and its holes of orbit o. A member lies in cap c when its posed region (for a hole, its slot) lies in the
closed cap; an off piece straddles c when its region meets both sides of the cut. A set G of members is
*self-locked* if for every grip c one of the following holds: (i) every member of G lies in cap c; (ii) no member
of G lies in cap c and no off piece of G straddles c; (iii) some off piece of G straddles c.

**Lemma U (rigid sets; proved).** Let G be self-locked in a configuration X and (c, a) a retained letter admissible
from X. The letter moves G rigidly (every member by a, or none), and the image of G is self-locked afterwards.
*Proof.* Admissibility excludes (iii) for c. In case (i) every member moves by a, in case (ii) none moves. The
image satisfies, for each grip c', the condition that G satisfied for a^-1(c') in case (i) or for c' in case (ii),
since a in K+ permutes the grips and maps caps onto caps. QED (A single jumble letter also moves G rigidly, but a
jumble element does not permute the grips, so the image need not be self-locked; only retained letters are used
below.)

**The splitting closure P(X).** Start from the classes of members with equal free-cap signature (the set of free
grips, those straddled by no off piece, whose cap holds the member). While some part has a grip c whose cap holds
some but not all of the part and that no off piece of the part straddles, split the part into its members in cap c
and the rest. The final parts are self-locked. *Claim (proved):* every self-locked set S of members lies in a single
final part; hence P(X) does not depend on the order of the splits and depends only on the shape of X (and o).
*Proof.* S lies in one signature class: a free grip is straddled by no off piece, so S lies wholly inside or wholly
outside its cap. If S lies in a part that is split along c, no off piece of S straddles c (none of the part does),
so S lies on one side. By induction S lies in one final part. Two split orders give partitions each of whose parts
lies in a part of the other, so they coincide. QED

**Proposition U' (proved).** Let U be admissible from solved and o an orbit. Suppose that every part of P(U) holding
an orbit-o member (an orbit-o hole or an orbit-o off piece) satisfies

- (N) its number n of orbit-o members is even, and
- (S) every k in K+ that maps the part onto itself (its holes onto its holes, its off regions onto its off regions)
  has sgn(k on the part's orbit-o holes) * sgn(k on the part's orbit-o off pieces) = +1.

Then b_o(U R U^-1) = 0 for every retained word R admissible after U whose end has the shape of U and for which
U R U^-1 ends on the lattice.

*Proof.* Each part G is self-locked, so by Lemma U the loop R moves it rigidly, by some k_G in K+, onto a self-locked
set of U R. The members of U R occupy the positions of the members of U (same shape), so by the claim k_G(G) lies in
a part G' of P(U). The reverse word R^-1 is a retained loop at U R, which has the shape of U, so it carries G'
rigidly into one part; that part contains R^-1(k_G(G)) = G, so it is G, and |G'| <= |G|. Hence R maps every part
rigidly onto a part. By Lemma V', b_o(U R U^-1) is the sign of the permutation that R induces on the orbit-o
members (holes and off pieces). On a cycle G1 -> G2 -> ... -> Gl -> G1 of parts this permutation is a cyclic shift
of l blocks of n members followed by s = k_Gl ... k_G1 on G1, which maps G1 onto itself; its sign is
(-1)^(n(l-1)) sgn(s) = +1 by (N) and (S). QED

**Corollary U1 (proved).** (a) If the hypotheses hold at U, they hold at every shape reachable from U by retained
letters (parts map rigidly onto parts, by the argument of the proof applied to a retained word and its reverse, and
(N), (S) are K+-invariant), and at every K+-conjugate of U (Lemma E). (b) They then give b_o = 0 for the bases U W
and W U with W retained: U W R W^-1 U^-1 = U (W R W^-1) U^-1, and nu(W Y W^-1) = nu(Y) for lattice-returning Y,
since nu is a homomorphism and nu(W) = 0 (F1). (c) Every jumble element at a grip is r1 x r2 with r1, r2 in A4 and
x = item 12 (A4 \ Lambda / A4 has two double cosets), and K+ is transitive on grips; so the certificate for the
single base (0, item 12) covers every lasso U R U^-1 whose stem U contains exactly one jumble letter.

**Certificate UNITS (o = 5).** `scripts/units5.py` (signature classes), `units5b.py` (splitting closure) and
`units5c.py` (splitting closure with a vectorized stabilizer prefilter; on base (0,12)(26,12) it reproduces the parts,
stabilizer orders and signs of `units5b.py`) compute, from an exact pose table and its exact cap sets (`capsets.py`
of the 3E package), the parts, their self-locking, n, and every k in K+ that fixes a part, with the two signs (holes
by exact slot images; off regions by matching posed-region centroids to 1e-12, bijectively). In every row of the
table the hypotheses (N) and (S) hold for o = 5 (summary in `certs/b5/units-summary.json`):

| Base | Off pieces | Parts with an orbit-5 member | Splits | n | Largest stabilizer |
|---|---|---|---|---|---|
| (0,12) (covers every single jumble) | 3,097 | 13 | 0 | 4, 72 | 3 |
| (0,12)(56,12) | 5,799 | 25 | 0 | 4, 72 | 3 |
| (0,12)(56,20) | 5,799 | 25 | 0 | 4, 72 | 3 |
| (0,12)(56,40) | 5,404 | 24 | 0 | 4, 72 | 3 |
| (0,12)(0,13) | 3,097 | 13 | 0 | 4, 72 | 3 |
| (0,12)(26,12) | 5,799 | 26 | 23 | 4, 72 | 3 |
| (0,12)(26,30) | 5,799 | 26 | 24 | 4, 72 | 3 |
| (0,12)(100,12) | 6,194 | 26 | 12 | 4, 72 | 3 |
| 30 random second jumbles after (0,12) | 5,404 to 5,905 | 24 to 26 | 0 to 26 | 4, 72 | 3 |
| e2run24 | 690 | 3 | 0 | 4 | 1 |
| stop62 | 860 | 3 | 0 | 4 | 1 |
| I_a@40 fixture | 94,147 | 445 | 807 | 4, 72 | 3 |
| X, E1, E2, I1, I2 | 107, 90, 3, 3, 3 | 0 | 0 | none | none |
| T200 | 120,119 | 609 | 1,274 | 4, 72, 76, 80 | 3 |
| T300 | 120,343 | 607 | 1,322 | 4, 72, 76, 80, 84 | 3 |
| T723 | 118,629 | 596 | 1,453 | 4, 72, 76, 80, 84 | 3 |
| H6 = T723 + (433, 12) | 120,078 | 606 | 1,478 | 4, 72, 76, 80, 84 | 3 |

Words are I_a letters (grip, menu item); "(0,12)(26,12)" is the jumble item 12 at grip 0 followed by the jumble item
12 at grip 26. (0,12)(0,13) is one rotation of cap 0, so it is again a single jumble; the other 36 two-letter bases
jumble two different grips. The random sample (`data/sample2.json`, seed 20261014) draws the second jumble uniformly
among the 48 jumble items at the 31 unstraddled grips whose cap holds an off piece or a hole of (0, 12). e2run24 and
stop62 are states on the E3 history (replayed from solved by CERT-HIST of the 3E package); I_a@40 is the fixture
state of the 3E package (40 jumble steps, all off pieces at depth one). In X, E1, E2, I1 and I2 no orbit-5 hole or
orbit-5 off piece exists, so the statement is vacuous there. T200, T300 and T723 are the states after records 1 to
200, 300 and 723 of the I_a W-J fixture journal, and H6 is T723 followed by (433, item 12) (Section 4.1); their pose
tables come from `export_fixture.py` (digests in `certs/q1/export-deep.log`), and the four rows are summarized in
`certs/b5/units-deep-summary.json` (certificate UNITS-DEEP).

**Positive control: orbit 4.** At (0, 12) the same computation for o = 4 fails exactly at six parts, each a pair of
orbit-4 holes in the cap of a clean 36-degree grip, not self-locked, with a K+ element that swaps the two holes
(`certs/b5/units4-u12.json`). There the loop search below finds odd orbit-4 loops, and one of them, replayed exactly
(certificate VERIFY-LOOP), gives the 6-letter word (0,12) (11,3) (0,10) (15,9) (0,8) (0,45), admissible from solved,
ending on the lattice with odd signs exactly on orbits 4, 7, 10, 11, 12, 17, 18, 20, 21, 23, 24, 26, 28, 31, 32 and
33 (`words/b4-loop-word-6.json`; its nu lies in V34). The test therefore separates the two orbits that Theorem A
does not cover.

**Consequences.**

- At a certified base every unstraddled cap holds an even number of orbit-5 members, since each part lies wholly
  inside or outside it and n is even. Corollary D and its off-piece variant need such a cap with an odd count, so they
  cannot produce b5 from any certified preparation; this is why B5-SEARCH found nothing.
- Every generator of Theorem LB, nu(E3) and the four 3E restoration ends have b5 = 0 (CERT-LB-V2, TABLE-NU,
  TABLE-NU-3E).

**Search evidence (floating-point searches, not certificates; all hits replayed exactly).**
`scripts/loopsearch.py` searches breadth-first over retained letters from a base, hashes shapes, and computes for each
collision (two paths A, B to one shape, so A B^-1 is a retained loop) the orbit-o sign of Lemma V'. From (0,12) at
depth 2 (letters at grips whose cap holds a member of the wanted orbits): 99,307 shapes, 43,144 collisions; orbit 5
all even, orbit 4 odd in 84. A depth-3 run from (0,12), stopped early: no odd orbit-5 collision, 1,424 odd orbit-4
collisions. From e2run24 at depth 2: 169,909 shapes, 93,289 collisions, orbits 4, 5 and 8 all even. From
(0,12)(26,12), restricted to 20 grips (`loopsearch2.py`), depth 2: 11,577 shapes, 7,487 collisions, orbit 5 all
even. Each of these bases is now covered by UNITS for orbit 5.

**A natural invariant does not exist in its simple form (certificate ORIENT; geometry and K+ only).** The orbit-5
chambers (7,200, each in 5 caps) carry a pair graph: two chambers are joined when their cap signatures share 4 grips
and that sub-signature belongs to exactly these two chambers. Its 7,200 edges form 1,200 hexagons and split into two
K+-orbits M0 and M1, each a perfect matching of 3,600 edges (`edgeorbits.py`). Each cap holds 24 edges of each
matching wholly and 12 chambers whose partner lies outside. Under local retained moves (a letter (g, a) applied to
an edge lying wholly in cap g), the oriented edges of each matching form a single component that contains both
orientations of every edge (`orient.py`), so no orientation of M0 or M1 is preserved by retained moves; orbit 4
behaves the same way. A b5 invariant cannot be a parity of domino orientations of this kind.

**What this route left open (closed by Theorem B5).** Choose a spanning tree of the graph of reachable shapes
(vertices) and admissible letters (edges) that restricts to a spanning tree of every retained component (shapes
joined by retained letters) and, on the graph whose vertices are the retained components and whose edges are jumble
letters, to a spanning tree there. A closed path at the lattice shape is, after cancelling inverse pairs, a product
of the basic cycles of the non-tree edges; these are admissible words (S), and nu is additive on concatenated
lattice-returning words, so A_J is generated by nu of the basic cycles. The basic cycle of a retained non-tree edge
is a lasso P R P^-1 with R a retained loop at the entry shape of its component, and Proposition U' with Corollary U1
gives it b5 = 0 whenever that component is certified. Two things remained: (1) components that are not certified
(only the bases of UNITS and their retained and K+ images are); (2) the basic cycles of jumble non-tree edges, that
is, the cycles in the graph of retained components. Lemma U says nothing across a jumble (a jumble creates and
destroys parts). Theorem B5 needs neither: its potential V is defined at every reachable configuration and changes by
an even permutation at every admissible twist, jumbles included.

### 2.5 Theorem 3.2 (answer to 3.2: no; certificate CERT-LB-V2, word M8, and certificate M8-POLES)

M8 = u (16, a) u^-1 with u four jumble twists at grips 0 (120 degrees), 43 (144), 161 (120), 44 (144) and a the
half-turn of A4_16 (the exact records are in `words/generators.jsonl`). It is admissible from solved, ends on the
lattice, and nu(M8) is odd exactly on orbits 8, 16, 18, 19, 20, 22, 23, 26, 27, 28, 31, 32, 33, 34; in particular
sign 8 = 1 and sign 25 = 0. M0 = w9 has both odd; the reverse pattern occurs in M5 (sign 25 odd, sign 8 even). So
sign 8 and sign 25 are independent on G_J: b8 and b25 are both in V33.

### 2.6 Questions 3.3 and 3.4

- **3.3.** The handoff answered no with f32 (fact d0c9bda0a984e204). Theorem LB adds f11 (M11) and f8, f22, f25
  (the frame loop words), so five C2 frame coordinates are nonzero on A_J, and the C5 frames of orbits 17 and 28 are
  nonzero too. By Theorem A, f0, f3 and f6 vanish; so the frame part of A_J is exactly
  <f8, f11, f22, f25, f32> x C5_17 x C5_28.
- **3.4.** Every sign generator of this work and M0 to M10 are conjugated half-turns u (d, a) u^-1: Corollary D
  words have u = C C^a, mirror words u = v v^a. Their nu values span <b4, b7, ..., b34> (certificate CERT-LB-V2,
  rank 29 of that subfamily, recorded as `conj_half_turn_rank`; the checker recognizes the shape u (d, a) u^-1 letter by
  letter). Under H, the handoff's fact f33453c92cc07c36 gives zero frames for every conjugate of a single retained
  twist, and with Theorem A and Theorem B5 their nu values lie in <b4, b7, ..., b34>. So conjugated half-turns
  generate exactly C2^29 = <b4, b7, ..., b34> (the upper bound under H), and A_J is strictly larger (f8, f11, f22,
  f25, f32, C5_17, C5_28): the answer is no, and the frame part needs other words, such as the frame loops of
  Corollary L.

## 3. What Theorem A says about the mechanism (3.1, "explicit per-twist update")

The card asks for a quantity with a per-twist update whose restriction to the lattice is nu modulo A_J. For the
inner orbits, psi of Theorem A is such a quantity: it is defined on every reachable configuration through the
cohort description INV, its per-twist update is zero, and on the lattice it equals nu restricted to I''. The
precise form of Card 2's "cap count modulo 2" is C4: an admissible twist conserves m_-1, m_0, m_1 exactly, and the
vacancy of one inner slot forbids every twist of N15 or N25 of an out grip. For the other orbits no such quantity
exists in the same form, since b4 and b7 to b34 all occur. For orbit 5 the map V of Theorem B5 is a quantity of
this kind: it is defined at every reachable configuration, its per-twist update is the even slot permutation
tau_d(h), and on the lattice its sign is b5.

## 4. Question 1

### 4.1 Q1.1: finiteness (open; proved partial bounds)

**Proposition HB (height bounds for the model orbits 0 to 9; proved, with certificate REFX).** At every
configuration reachable from solved, every piece of model orbit o has left height at most 2 if o is 0, 1, 2, 3 or 6,
at most 4 if o is 4, 5, 7 or 8, and at most 6 if o is 9.

*Proof.* Lemma R (Section 2.4) is stated for a piece of any orbit, with the class taken as in Section 1 (the
K+-class of the pose relative to the representative chamber). Its rules R1 to R5 are over-approximations of what
an admissible word can do, so along every admissible word from solved the refined state of a piece of orbit o stays
in the part of the transition system R_o reachable from the lattice class with no out grip. Certificate REFX
(`refexact.py`, exact representatives in Q(sqrt 5), every transition target confirmed exactly) computes that part
for the K+ orbits 1 to 10 of the region list, which are the model orbits 0, 1, 3, 6, 2, 4, 5, 7, 8 and 9, and finds
it closed in every case, with classes only at the heights stated:

| Model orbit | 0 | 1 | 3 | 6 | 2 | 4 | 5 | 7 | 8 | 9 |
|---|---|---|---|---|---|---|---|---|---|---|
| Refined states | 9 | 13 | 17 | 21 | 17 | 156 | 144 | 234 | 287 | 558 |
| Classes | 9 | 13 | 17 | 21 | 17 | 97 | 49 | 105 | 133 | 257 |
| Largest left height | 2 | 2 | 2 | 2 | 2 | 4 | 4 | 4 | 4 | 6 |

The left height is a function of the class (Section 1). QED

(Addendum, 15 October 2026, senior review round 13, T13-01: `refexact.py` decides whether a cap holds or excludes a
posed region in floating point, with margin 1e-7, and exactly only near ties. Certificate REFX-EXACT
(`refx_exact.py`) recomputes all 861,600 of these decisions over Q(sqrt 5), in and out for each of the 718 classes and
600 grips of the ten closures, and finds no difference; the closures reproduce the recorded states, classes, heights
and class digests. Every decision of REFX is therefore exact, including those of R_5 used by POT5.)

The rule R4 needs one remark for orbits other than 5. If cpl = (d, j0), the pose of p is G_d P0, with P0 its pose
before the jumble that made d out and G_d the product of the twists at d since then. The class is j0 again exactly
when the left factor of G_d is a scalar multiple of a unit icosian, that is (its right factor is one by T3) exactly
when G_d lies in K+, hence in Lambda_d intersected with K+, which is A4_d. So R4 holds without S3. The proof in
Section 2.4 uses S3 only because there the class is defined from the posed region.

The method stops at model orbit 10 (K+ orbit 11): an exact run with additional blocking (`refexact2.py`, which also
uses the straddle sets of the anchored inner cohorts) passed 600,000 refined states with classes up to height 14 and
a growing queue (`certs/q1/refx2-orbit11.log`). Since R_o without that blocking reaches at least as much, this
over-approximation gives no bound from orbit 10 on. In the deep states the orbit-10 pieces reach height 4 only (ADJ-H6
below), so the over-approximation is coarse there, not necessarily the dynamics. (Correction, 15 October 2026, senior
review round 9, T9-01 in `../09-general-method/SENIOR-REVIEW.md` section 14: the extra filter of `refexact2.py` on
newly holding out grips, line 161, drops a reachable transition, with the witness [(0, 12), (11, 41)] from solved. A
closure of `refexact2.py` would therefore not prove a bound. The conclusion above stands, since the filter only
removes transitions; Proposition HB uses `refexact.py`, which has no such filter.)

**Structural facts (proved from TREEFACTS; H is not needed).** Every plane of the cut arrangement (cut planes, body
facets and core facets) has the form n_c . y = const for a grip c, and every facet of a home chamber lies in such a
plane. A posed piece p with pose [l, r] has its facets in the images of these planes; its *left vertex* is
x_p = l v0.

- **AL (alignment law).** If a posed piece p has a 3-face in a plane n_c . y = const, then d(x_p, w_c) = 1.
  Consequently (i) a piece at left height at least 4 has no 3-face in any plane of the arrangement; (ii) a piece at
  left height 2, whose left vertex lies beyond w_b, has 3-faces only in planes of grips of branch b.
  *Proof.* The posed facet comes from a home facet with normal n_a, and its normal is l n_a r^-1, so
  l n_a r^-1 = s n_c for a scalar s. By T1 to T3, w_c = n_c v0 = l n_a r^-1 v0 = l n_a v0 = l w_a, a neighbour of
  l v0. (i) w_c is adjacent to v0, so d(x_p, v0) <= 2. (ii) The only neighbour of x_p at distance 1 from v0 is w_b.
  QED
- **Lemma L (3-face neighbours).** If two posed pieces p and q share a 3-face, then x_p and x_q have a common
  neighbour u, so d(x_p, x_q) <= 2. *Proof.* The shared face lies in one plane, which carries a facet of each piece
  (the pieces are convex with disjoint interiors). As in AL, u = l_p w_a = l_q w_b. QED
- **Lemma FN (frozen neighbourhoods).** If p and q share a 3-face and p has left height at least 4, then every
  admissible twist moves both or neither, so they still share a 3-face afterwards. *Proof.* An admissible twist at c
  moves exactly the pieces in cap c, and none straddles the cut. If it moved exactly one of p and q, their shared
  face would lie in both closed half-spaces of the cut plane of c, hence in that plane, against AL (i). QED
- **Corollary FN1 (rigid high components).** Let K be a connected set of the 3-face graph whose pieces all have left
  height at least 4. (a) All left vertices of K lie beyond one vertex a3 at distance 3 from v0, so they share the tree
  path v0, a1, a2, a3. (b) Every admissible twist moves all of K or none of it. (c) A twist that moves K changes the
  left heights of all its pieces by the same amount: 0 for a retained letter; for a jumble letter at d, +2 if the
  branch of d differs from that of K, -2 if the left factor l of the letter has l^-1 v0 = a2, and 0 otherwise. (d) A
  piece of left height 2 that shares a 3-face with K has left vertex a2 and changes its height with K, so after a
  raising letter it belongs to K.
  *Proof.* (a) For adjacent pieces of K, the common neighbour u of Lemma L has height at least 3, so the ancestors at
  distance 3 of x_p, u and x_q coincide; K is connected. (b) Lemma FN along the edges of K. (c) Retained letters have
  left factors in 2I, which fix v0. For a jumble letter with left factor l, the new height of x is d(l x, v0) = d(x,
  y) with y = l^-1 v0, a vertex at distance 2 from v0 beyond w_d (T3: l fixes w_d). The geodesic from y to x leaves
  the path v0, a1, a2 at a vertex m determined by y alone, and d(x, y) - d(x, v0) = d(m, y) - d(m, v0) is +2 (m = v0,
  that is, y in another branch), 0 (m = a1) or -2 (m = a2 = y). (d) The common neighbour u of the two left vertices
  is at height 3 (adjacent to a vertex of height 2 and one of height at least 4), so x_q = a2, and the formula of (c)
  gives the same change for x_q. Lemma FN keeps q attached while K is at height at least 4. QED

So a high component rises only through a jumble at an unstraddled grip of another branch whose cap wholly holds
the component together with its attached height-2 pieces, and it is never separated while it stays high.

**The deep state H6 (certificate).** Replaying the I_a W-J fixture journal (exact records 1 to 723) and then the
jumble twist (433, item 12) is admissible at every step and leaves 78 pieces at left height 6 (histogram {0: 57,042,
2: 115,850, 4: 4,150, 6: 78}). So reachable heights are not bounded by 4. The height-6 pieces belong to the model
orbits 13, 15, 16 and 18 to 32; no piece of orbits 0 to 12 is above height 4 there.
(Addendum, 15 October 2026, senior review rounds 13 and 13b, `../09-general-method/SENIOR-REVIEW.md` section 18:
the bound 6 on left heights is equivalent to the statement that every cap of another branch that wholly holds a
piece of height at least 6 is straddled (P6, T13-03). The 78 height-6 pieces of H6 form exactly one 3-face component,
with no other 3-face neighbour (T13b-05, exact). Every cap of another branch that holds one of them lies in
N15(433) union N25(433), where 433 is the grip of the raising jumble, which is still out (exact).)

**Evidence on the H6 state (floating point unless stated; diagnostics, not certificates).**

- **ADJ-H6** (`adj3.py`). The 3-face graph of the 177,120 posed pieces has 452,807 edges and 289 components; the
  largest has 157,378 pieces. Every edge joins left vertices at distance 0 (408,966 edges) or 2 (43,841), as Lemma L
  says. A piece is *anchored* when one of its facets lies in a plane of the arrangement (by AL it has height 0 or 2).
  22 components contain no anchored piece; they consist of all 78 height-6 pieces and 2,542 of the 4,150 height-4
  pieces. In the components with an anchored piece the height never exceeds 2 + 2 x (chain length to an anchored
  piece). 15 pieces have no 3-face neighbour at all (*floating pieces*, of heights 0 and 2). So the 3-face graph of a
  reachable state is not connected, and heights are not controlled by chain distance to the lattice.
- **PLOC** (`ploc.py`). For 60 pieces with at most one detected neighbour (3 floating, 57 with one), a point 1e-6
  beyond the centroid of each posed facet was located among all posed regions. Beyond the facets of the floating
  pieces there is no piece; each of the 57 others has exactly one facet with a piece beyond it, its detected
  neighbour. So the floating pieces are real and not an artefact of the adjacency test.
- **FLT-H6** (`flt.py`, exact classify). Each of the 22 unanchored components has one left height and one branch.
  Of the 108 caps that wholly hold one of them, all 83 caps of another branch are straddled; of the 25 caps of the
  component's own branch, 15 are unstraddled, and a jumble there does not raise it (Corollary FN1 (c)). So no single
  letter raises any unanchored component of H6.

**Searches for height 8 (negative evidence; every applied letter is checked by the exact simulator, candidate
generation uses floating-point pre-tests).**

- **H6-MORE (certificate).** After record 723 and each of the 48 jumble twists at 433, every grip within 55.7
  degrees of 433 whose cap is unstraddled (exact classify) and every jumble element y of that grip were tried on
  every distinct left factor l in the cap (y_L l is then a reachable left position). Transitions counted over such
  (l, y) pairs: 2 to 0: 55,440; 2 to 2: 166,320; 2 to 4: 4,992; 4 to 2: 2,208; 4 to 4: 6,624; 6 to 4: 576;
  6 to 6: 1,728. No height 8 occurs, and the height-6 vertices reached are the 48 of COVER6.
- **COVER6X (certificate; complete under H).** After each of the 724 records of the fixture journal (exact replay),
  the grips whose closed cap might wholly hold an off piece of left height at least 4 were found by a floating-point
  vertex test with margin 1e-6, which cannot miss such a grip, and classified exactly. For every unstraddled one,
  every jumble element y of it and every distinct left factor l of height at least 4 wholly inside its cap, y_L l is
  the left position of a reachable pose. Retained letters keep left heights, and under H one jumble letter changes a
  left height by at most 2, so these are all the one-twist extensions of fixture prefixes that can reach height 6 or
  more. Pairs (l, y) counted over all records: 4 to 2: 107,580; 4 to 4: 322,740; 4 to 6: 96. Height 6 occurs only
  after records 723 and 724, at the 48 vertices of COVER6 (2I-closure 240); height 8 never occurs. Every off piece
  of every prefix has left height 2 or 4 (the output field `last_step_height_histogram` is these piece counts summed
  over all records: 80,681,654 and 1,757,542).
- **TWOSTEP** (`twostep.py`). For each of the 22 components of H6 without an anchored piece: every
  non-identity element of every unstraddled cap that wholly holds it, then a jumble at every cap of another branch
  that wholly holds the moved component. 885 first letters applied, 3,087 second-letter candidates, 3,082 of them
  straddled. The 5 two-letter raises found end at height 6 (the 78-piece cluster 6 to 4 to 6; three 21-piece
  clusters 4 to 2 to 4; a 7-piece cluster 4 to 4 to 6).
- **CLEAR78** (`clear.py`). For the 78-piece height-6 cluster and each of its four other-branch caps (grips 274,
  326, 340, 430): the pieces straddling that cut plane number 339, 89, 158 and 114 (all of height 2 except 16 of
  height 4), and no clean cap away from the cluster holds all of them, so no single twist clears the plane.
- **PUMP** (`pump2.py`, `pump3.py`). A return pump for one tracked piece p: (G, y^-1) (D, a) (G, y) with G and D of
  one branch other than p's, a in A4_D, p wholly in cap D and outside cap G, and a(p) wholly in cap G. The return (G,
  y) raises p by 2 (a jumble at a grip of another branch, as in the proof of Corollary FN1 (c)). Depth-first over
  raises from solved, with direct raises as a fallback: pieces 116, 22 and 65 (881 pump attempts, 774 blocked at D
  and 99 at G, 8 completed) and a small-lens variant for piece 22. Best height 4 in every run.
- **SINGLE** (`single.py`). The 15 floating pieces of H6, depth-first over raising jumbles (depth 4): 137 candidate
  caps straddled, 1 open; one piece rose from 0 to 2, none higher.
- **RWALK** (`rwalk.py`). Four return-biased random walks from solved (1,200 s each; 2,529 to 3,703 admissible
  letters per walk; returns by the inverse of a grip's accumulated element): the largest left height of any piece at
  any step is 4.
- **INNERX** (`innerx.py`; exact admissibility, floating-point attribution). After the jumble (0, item 12) from
  solved, by angle from grip 0 and branch: other-branch grips at 15.52 degrees (4 of 4 blocked), 25.24 (12 of 12),
  36 (6 of 12), 41.41 (3 of 12); unblocked: other-branch grips at 44.48 (4), 49.12 (24), 56.01 (24), 60 (12), and
  same-branch grips at 36 (12) and 60 (20). At 15.52, 25.24 and 36 degrees the number of blocked grips equals the
  number of grips straddled by a piece of the inner K+ orbits 0 to 4 (the pieces that meet the open inscribed ball).
  So right after a jumble, the other-branch grips nearest to it are blocked by the jumbled inner pieces themselves.
- **Earlier ladder searches (first edition).** Base: a single off piece p at height 2 (orbit 33, a 175-letter
  word). (a) Commutator ladder x k x^-1 k^-1, x a raising jumble at a grip whose cap holds p, k a twist carrying p
  out of that cap with no other off piece in its cap: of all candidate pairs, 1,092 had a straddled cap and in
  4,476 p was not wholly inside k's cap after x; none remained. (b) Raise and refill: after a raising jumble y, a
  retained twist refilling exactly one hole, so that C (g, a) C^-1 keeps only the refilling piece: 61,920
  candidates screened, none ended off the lattice at height 4 or more.

**Status.** Neither a uniform bound nor an unbounded admissible family is established. **Open; delivered as
open.** A fixed word that can be repeated is periodic in all poses (handoff fact f3ab02a86e740ff2), so an unbounded
family would have to be adaptive. (Correction, 15 October 2026, senior review round 9, T9-02: a word repeated without
end has a period that depends on neither the word nor the state; an unbounded family cannot be ultimately periodic,
but it need not adapt to the state.) Everything computed points to a bound: heights are proved bounded for the ten
innermost model orbits, the largest height observed anywhere is 6, a high component can only rise as a rigid
block through a clean cap of another branch, and in the deepest state every such cap is straddled. The step still
missing is an invariant that controls the outer orbits across jumbles, as Lemma R and REFX do for the inner ten.

### 4.2 Q1.2

Answered in the handoff under H (fact 484b9292b3bbcc71): Safe has 600, 120 and 0 elements at heights 0, 2 and at
least 4. Safe = empty at height at least 4 only removes the uniform safe cuts; it does not prevent a further rise,
and H6 shows that rises happen.

### 4.3 Q1.3: normal form

Under H the handoff showed (fact 484b9292b3bbcc71) that the algebraic normal-form set at height 2m has
20 x 16^(m-1) = 5 x 4^(2m-1) elements, that is, every vertex at distance 2m. So the normal form says: every vertex
at even distance 2m is the left position of a reachable pose, and no other vertex is.

- **Heights 2 and 4: proved (certificate HEIGHTS4 and Lemma E).** The 320 vertices at distance 4 form six
  2I-orbits (sizes 60, 60, 60, 60, 60, 20); the exactly replayed fixture reaches a vertex of each orbit (the record
  indices are listed in the certificate), so by Lemma E all 320 are reachable. Height 2 (all 20) is Card 3 A.
- **Height 6: partial (certificate COVER6).** From the state of H6, the 48 jumble elements at grip 433 give 48
  vertices at distance 6; their 2I-closure has 240 of the 5,120 vertices. Open whether all are reachable.
  COVER6X shows that no one-twist extension of a fixture prefix adds a height-6 vertex outside these 240, so the
  rest need longer or different histories.
- Odd distances never occur (T5), so "no other vertex" holds under H.
- **Link with 1.1.** If left heights are bounded (1.1), the normal form fails at every distance above the bound.
  By Proposition HB, pieces of the model orbits 0 to 9 never pass height 2, 4 or 6 (Section 4.1), so the vertices at
  distance 8 and more could only be left positions of pieces of the orbits 10 to 34. All evidence of Section 4.1
  points to a bound, hence to a negative answer, but neither is proved. **Open; delivered as open.**

## 5. 3E interface note (where the paused 3E line can use this)

- **Atlas recomputation (floating point, with exact tie handling where stated; certificate ATLAS).** For class 0,
  grip 0 and one jumble frame: the class cut arrangement has 1,441 sign cells holding posed regions; orbits under the
  frame-compatible group (order 360) are: 1 cell (5,280 sources / 5,280 capacity), 120 cells (385/385), 360 cells
  (86/86) twice, 360 cells (111/0), 120 cells (111/111), 120 cells (87/0). In the single-cell orbit, 2,400 of the
  5,280 source positions meet the core and 2,880 do not. These match the 3E registry (orbits 0 to 6). The
  Frank-Wolfe separations are floating-point and are corroboration, not exact certificates.
- **Seed census (certificate CENSUS).** Every single-hole word found here (27 distinct, `words/singles.jsonl`) is a
  reachable configuration with exactly one off piece; locating that piece in the atlas gives pure single-input
  instances, with explicit restoring words (the inverses of the single words), for the source orbits
  3 (cells 111/0; 16 instances, off pieces of orbits 22, 25 to 31, 33), 4 (87/0; 4 instances, orbits 29, 31, 33),
  6 (111/111; 3 instances, orbits 23, 32) and the 86/86 pair (registry orbits 2 and 5, which this census does not
  separate; 4 instances, orbits 19, 23, 24), in outer classes 0, 1 and 3. Source orbit 3 is the first open seed in
  the 3E table. No instance was found for registry orbits 0 (5,280/5,280) and 1 (385/385). Scope: one off piece at
  a time; no protection of other off pieces, no multi-source compatibility, no coverage claim.
- **Vacancy formula.** Lemma V and Corollaries D and L are exact hole-bookkeeping statements. They may serve 3E's
  hole preparation and return steps, since they predict the full relabelling of a conjugate from the hole set
  alone.

## 6. Certificates

All scripts run with `python -I <script> <research/jumbling> ...` from a checkout of `27a001c`; outputs are in
`certs/` and words in `words/`. Scripts that use the structure arrays take `data/classes.json` and
`data/arrays.npz` (the same files as in the 3E package) and import `scripts/e3lib.py`.

| Name | What is computed, on which objects | Must show | Script / output |
|---|---|---|---|
| CERT-LB-V2 | replay from solved of the 40 words of `words/generators.jsonl`: menu membership, admissibility at every prefix, lattice end; nu by coherent frames; 35 signs again from final poses by pole permutations; F2 rank; shape test for conjugated half-turns | all pass; rank 34 on {b4, b7..b34, f8, f11, f22, f25, f32}; C5 frames of orbits 17 and 28 nonzero; conjugated half-turn subfamily rank 29 | `verify_words.py` / `certs/cert-lb-v2.json`, `.log` |
| TABLE-NU | nu of the E3, I1-end, I2-end pose tables of the earlier bundles (digests and file hashes recorded) | the values of Proposition 2.2 | `table_nu.py` / `certs/table-nu.txt` |
| TABLE-NU-3E | nu of the four restored ends of the 3E package (E2, I1, I2, I2 without its own inverse word) | the values of Section 2.2; b5 = 0 | `table_nu.py` / `certs/table-nu-3e.jsonl` |
| CERT-HIST | (3E package) replay from solved of the whole E3 history, 39,392 letters | every letter admissible, segment digests equal | 3E package `replay_e3_history.py` / `certs/` there |
| F8-C517 | harvest of orbit-8 and orbit-17 singles, frame-loop search | the singles and loops behind FR8-0 and FR17-0 (the words themselves are checked by CERT-LB-V2) | `harvest.py`, `tomodel.py`, `frame_track.py` / `certs/f8-c517/`, `words/singles-orbit*.jsonl`, `words/frame-loops-8-17.jsonl` |
| UNITS | for each base of Section 2.4 (orbit 5): splitting-closure parts, self-locking, n, all k in K+ fixing each part with hole and off signs | (N) and (S) hold in every row | `export_word.py`, `capsets.py`, `units5.py`, `units5b.py`, `units5c.py`, `run_u2.sh`, `run_u3.sh`, `build_units.py` / `certs/b5/units-summary.json`, `certs/b5/units/`, `certs/b5/units5-u12.json`, `certs/b5/run_u*.log` |
| UNITS-O4 | the same for orbit 4 at (0, 12) | fails exactly at six two-hole parts with an odd swap | `units5.py` / `certs/b5/units4-u12.json` |
| VERIFY-LOOP | exact replay from solved of one odd orbit-4 loop-search hit | admissible, lattice end, odd signs as in Section 2.4 | `verify_loop.py` / `certs/b5/verify-loop-o4.json`, `words/b4-loop-word-6.json` |
| LOOPSEARCH | breadth-first retained-loop searches (floating point; statistics, not a certificate) | the counts of Section 2.4 | `loopsearch.py`, `loopsearch2.py` / `certs/b5/loopsearch/` |
| ORIENT | orbit-4 and orbit-5 pair graphs, their K+-orbits on edges, oriented-edge components under local retained moves | two perfect matchings per orbit; one component holding both orientations of every edge | `edgeorbits.py`, `orient.py` / `certs/b5/edges-*.json`, `certs/b5/orient-*.json` |
| G2 | for grip 0, all 48 jumble x and all inner slots s: grips whose closed cap contains x(A_s) | lists of C2 | `g_prime.py` / `certs/g_prime.json` |
| SHARE | grips sharing inner slots with cap 0, by orbit | C3 | `shared_homes.py` / `certs/shared-homes-cap0.txt` |
| SHARE45 | grips sharing slots of orbits 0..8 with cap 0, by angle | orbits 4, 5 shared at 36 degrees | `share45.py` / `certs/share45.txt` |
| SHARE36 | orbit composition of cap 0 intersect cap d for every grip d meeting cap 0, by angle | the two 36-degree families of 2.4 | `shared36.py` / `certs/shared36.txt` |
| B45-ACCESS | after each of the 48 jumble twists at grip 0, every grip whose cap holds a vacated orbit-4 or orbit-5 slot: exact classify (straddle, off pieces inside) | the pattern of 2.4, identical for all 48 | `b45_access.py` / `certs/b45-access.json` |
| B5-SEARCH | targeted doubling for orbit 5 (search statistics, not a certificate) | no clean cap with an odd number of orbit-5 holes | `b45_double.py`, `b5_double2.py` / `certs/b5-search.json` |
| CONS | exact lambda_o for all orbits and exact interior points z_-1, z_0, z_1 | the three inequalities of C4 | `cons_cert.py` / `certs/cons-cert.json` |
| M8-POLES | M0, M5, M8 replayed from exact records; signs from pole permutations only | sign 8, sign 25 as in 2.5 | `verify_m8.py` / `certs/m8-verify.json` |
| H6 | fixture records 1..723, then each jumble twist at 433 | max height 6, 78 pieces | `h6_verify.py` / `certs/h6-verify.json` |
| H6-MORE | one more jumble twist near 433 after H6, all unstraddled grips within 55.7 degrees | transitions of 4.1; no height 8 | `h6_more.py` / `certs/h6-more.json` |
| HEIGHTS4 | 2I-orbits of the 320 distance-4 vertices; fixture hits | every orbit hit | `heights4.py` / `certs/heights4.json` |
| COVER6 | distance-6 vertices from H6 and their 2I-closure | 240 | `cover6.py` / `certs/cover6.json` |
| COVER6X | along fixture records 1..724: after each record, every jumble element of every unstraddled grip whose cap wholly holds a piece of left height at least 4, applied to each distinct left factor | height 6 only after records 723 and 724, at the 48 vertices of COVER6 (2I-closure 240); no height 8 or more; pairs 4 to 2: 107,580, 4 to 4: 322,740, 4 to 6: 96 | `cover6x.py` / `certs/q1/cover6x-all.json`, `.log` |
| ATLAS | class-0 sign cells, sources, capacities, orbits, core test | the rows of Section 5 | `atlas.py`, `atlas2.py` / `certs/atlas*-c0.json` |
| CENSUS | the off piece of each single located in the atlas | rows of Section 5 | `census.py` / `certs/census.json` |
| F1, P, F | item-2 package: nu of retained primitives; position and frame certificates of Card 1 | 0 failures | earlier package; its report copied as `certs/check-report.json` |
| TREEFACTS | grip normals, unit icosians, the left and right factors of all 36,000 menu elements, the right factors of the 1,183 distinct H6 poses, pole images under Lambda_0 | T1 to T5 of Section 1 | `treefacts.py` / `certs/q1/treefacts-H6.json` |
| RIGHT2I | earlier, weaker form of T3 and T4 (right factors fix v0) and pole images for 40 grips x 6 jumble elements | 36,000 and 1,183 at tree distance 0; 28,800 images in the branch | `rightpart.py` / `certs/q1/rightpart-H6.json` |
| POT5 | refined dynamics R_5 (exact), the families F5, S1 to S3, the association e, the potential pi, the checks P (i) to (iii) | R_5 closed (144 states, 49 classes); S1 to S3 hold; P: 65, 3,900 and 28,585 checks pass | `potcert5.py` (imports `refexact.py`) / `certs/b5/pot5-cert.json` |
| POT5-VAL | V recomputed from actual posed regions along four random admissible walks and on the deep tables T200, T300, T723, H6 (validation, not part of the proof) | V bijective; per-twist rule; every region in a class of R_5 | `potval.py` (with `potsolve.py`, `refbfs2.py`), `run_val.sh` / `certs/b5/val/` |
| POT-O4 | the same construction for orbit 4 (K+ orbit 6) | no association satisfies (ii) (1,728 tried) | `potsolve.py` / `certs/b5/pot-o4.json` |
| UNITS-DEEP | UNITS for orbit 5 on T200, T300, T723 and H6 | (N) and (S) hold | `export_fixture.py`, `capsets.py`, `units5c.py`, `run_u_deep.sh` / `certs/b5/units-deep-summary.json` |
| REFX | the transition systems of Lemma R for K+ orbits 1 to 10 (model orbits 0, 1, 3, 6, 2, 4, 5, 7, 8, 9), exact | all closed; the heights of Proposition HB; class digests recorded | `refexact.py` / `certs/q1/refx-1-10.json` |
| REFX-EXACT | every cap decision of REFX (in and out, for each of the 718 classes and 600 grips) recomputed over Q(sqrt 5) | 861,600 decisions, no difference; the closures reproduce REFX; among the decisions REFX takes in floating point, the smallest distance from 0 is 0.000811 | `refx_exact.py` (imports `refexact.py`) / `certs/q1/refx-exact.json` |
| REFX2-11 | the same with inner-cohort blocking for K+ orbit 11 (model orbit 10), stopped at 600,000 states | not closed; classes up to height 14 | `refexact2.py`, `innerstraddle.py` / `certs/q1/refx2-orbit11.log`, `certs/q1/innerstraddle.json` |
| DEEP | exact pose tables of fixture prefixes T200, T300, T723 and of H6 | the digests of `export-deep.log`; H6 shipped as `data/t-T723x433-12.json` | `export_fixture.py` / `certs/q1/export-deep.log` |
| ADJ-H6 | 3-face graph of the H6 state (floating point) | the counts of Section 4.1; no edge beyond left distance 2 | `adj3.py` / `certs/q1/adj-H6.json`, `.log` (its `.npz` is the input of the next five rows) |
| PLOC | point location beyond the facets of 60 low-degree pieces (floating point) | as in Section 4.1 | `ploc.py` / `certs/q1/ploc-H6.json` |
| FLT-H6 | the 22 unanchored components of H6, the caps holding them, exact straddle status | other-branch caps all straddled | `flt.py` / `certs/q1/flt-H6.json` |
| TWOSTEP | two-letter raises of the 22 unanchored components | best height 6 | `twostep.py` / `certs/q1/twostep-H6.json`, `.log`, `grip-branch.json` |
| CLEAR78 | straddlers of the four other-branch planes of the 78-piece cluster, clean caps that hold them all | none | `clear.py` / `certs/q1/clear-78.json` |
| SINGLE | raise search for the 15 floating pieces of H6 | best height 2 | `single.py` / `certs/q1/single-H6.json`, `.log` |
| PUMP | return-pump searches from solved for pieces 116, 22, 65 (and a small-lens variant for 22) | best height 4 | `pump2.py`, `pump3.py` / `certs/q1/p2-*.json`, `p2-*.log`, `p3-22-1.json` |
| RWALK | four return-biased random walks from solved, 1,200 s each | best height 4 | `rwalk.py` / `certs/q1/rw-*.log`, `certs/q1/rwalk-summary.json` |
| INNERX | after the jumble (0, item 12): exact admissibility of every grip within 60 degrees, attribution to inner pieces (floating point) | the table of Section 4.1 | `innerx.py` / `certs/q1/innerx-0.json` |

Commands for the certificates new in the second edition (`<J>` is `research/jumbling` of a checkout of `27a001c`,
`<D>` a scratch directory; run from the package root):

    python -I scripts/treefacts.py <J> data/t-T723x433-12.json certs/q1/treefacts-H6.json
    python -I scripts/rightpart.py <J> data/t-T723x433-12.json certs/q1/rightpart-H6.json
    python -I scripts/potcert5.py <J> certs/b5/pot5-cert.json
    bash scripts/run_val.sh <J> scripts <D> certs/b5/val
    python -I scripts/potsolve.py <J> 6 certs/b5/pot-o4.json
    python -I scripts/export_fixture.py <J> 200,300,723 <D> 433:12
    bash scripts/run_u_deep.sh <J> . <D> T200 T300 T723 T723x433-12
    python -I scripts/refexact.py <J> 1,2,3,4,5,6,7,8,9,10 40 certs/q1/refx-1-10.json
    python -I scripts/refx_exact.py <J> certs/q1/refx-exact.json   (added 15 October 2026, round 13)
    python -I scripts/innerstraddle.py <J> certs/q1/innerstraddle.json
    python -I scripts/refexact2.py <J> 11 40 <D>/refx2-11.json certs/q1/innerstraddle.json --max-states 600000
    python -I scripts/adj3.py <J> data/t-T723x433-12.json certs/q1/adj-H6.json
    python -I scripts/ploc.py <J> data/t-T723x433-12.json certs/q1/adj-H6.npz certs/q1/ploc-H6.json
    python -I scripts/flt.py <J> data/t-T723x433-12.json certs/q1/adj-H6.npz certs/q1/flt-H6.json
    python -I scripts/twostep.py <J> data/t-T723x433-12.json certs/q1/adj-H6.npz 4 certs/q1/twostep-H6.json
    python -I scripts/clear.py <J> data/t-T723x433-12.json certs/q1/adj-H6.npz 78 certs/q1/clear-78.json
    python -I scripts/single.py <J> data/t-T723x433-12.json certs/q1/adj-H6.npz 4 certs/q1/single-H6.json
    python -I scripts/pump2.py <J> 116 6 1 certs/q1/p2-116-1.json --per-level 3 --ys 2   (also 22 6 2 and 65 6 3)
    python -I scripts/pump3.py <J> 22 3 1 certs/q1/p3-22-1.json --per-level 2 --ys 1
    python -I scripts/rwalk.py <J> 1 1200 <D>/rw-1.json 0.45 0.5   (seeds 2 and 3 alike; seed 4 with 0.6 0.7)
    python -I scripts/innerx.py <J> 0 certs/q1/innerx-0.json

The pump, walk and search runs are seeded but time-limited; a rerun on other hardware may visit a different part of
the search space.

**Checks not run.** No Windows, DirectX, native-host or performance check. No independent mathematical review of
Lemma V, Lemma V', Theorem A, Lemma U, Proposition U', Lemma R, Lemma T, Lemma P, Theorem B5, Proposition HB, AL,
Lemma L, Lemma FN, Corollary FN1 or the other new statements. The loop searches, the adjacency graph, the point
location, the pump, clearing and walk searches are floating-point heuristics or diagnostics and certify nothing;
only their exactly replayed letters are evidence. Bases outside UNITS and UNITS-DEEP were not tested for
Proposition U' (Theorem B5 makes this unnecessary for o = 5). The refined dynamics was not closed for the model
orbits 10 to 34. The deep tables T200, T300 and T723 are not shipped (each is regenerated exactly by
`export_fixture.py`; digests in `certs/q1/export-deep.log`), and neither is the 1.4 MB adjacency array
`adj-H6.npz` (regenerated by `adj3.py`).
