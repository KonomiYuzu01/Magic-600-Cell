# 3E stage result: a block-shape family of restoration words (menu I_a)

Results of 14 October 2026. Scope: restore glued blocks of depth-one off pieces for the menu I_a, from the current
state only, with one word family that is parametrized by the block's own geometry and carried between blocks of the
same shape by the symmetry group K+. This stage follows `PROOFS-3E.md` of the seed-table package
(`Claude-3E-seed-table-20261014`), whose Section 2.6 left the Block lemma open. The lattice phase (Card 5) is out of
scope. Nothing here claims that E, or 3E in general, is solved.

**Evidence kind.** Exact computations in synthetic geometry on the jumbling simulator of the research tree
(`research/jumbling`, branch `claude/jumbling-e3`, commit `27a001c`), run headless on Linux. Nothing here is
Windows, DirectX, native-host or performance evidence. The new proofs have not been independently reviewed.

**Labels.** *proved*; *certificate* (a finite exact computation; its file and script are named in Section 9, and
this document claims no more than the file records); *float evidence*; *conditional* (the condition is named);
*open*.

## 0. Answers and status

**Question 1: can D17- and D87-type constructions become a restoration family parametrized by block shape?**
Yes for one template and with per-instance hypotheses; not as a general theorem.

- The template is W = P Q P^-1 with Q = [A, [B, X]] and an optional parking word P = [[X, Y], Z] (the shape of D17).
  Its letters are chosen from the block's own geometry by exact cut tests (conditions Q1 to Q4; G2 in the chain
  packages). Lemma Q (proved) shows that these conditions alone bring the block onto the lattice under Q, with no
  piece of the block straddling a cut.
- Lemma TT (proved) carries a word found for one block to every block of the same K+ shape: the transported word
  m W m^-1 acts on the image block exactly as W acts on the source block, and Bad(m W m^-1) = m Bad(W).
- Theorem CB (proved) turns block words into a restoration of a whole state: restore blocks in the order of a
  block conflict digraph, each step by V W V^-1, given a vacating word V at each turn (a per-instance hypothesis).
- Measured (certificates, Section 5): the family restored 171 of 860 and 110 of 690 off pieces (3 blocks each) at
  the two mid-depth samples, and 1,101 of 94,147 (7 blocks) at the I_a@40 fixture, every step with zero collateral and
  every other off piece at its exact pose, all replayed independently. At the 689-piece state of stop62 every
  remaining block has a word and the conflict digraph is acyclic; what stops the run there is the vacating word.

**Question 2: when can general 3E be delivered?** Not on a date. Section 7 gives the go/no-go reading: the family
works where every block has at least two free grips (the 690- and 860-piece samples on E3's history) up to the
vacating word, and reaches 1.2% of the deep fixture, where 78% of the off pieces lie in blocks with one free grip or
none, blocks for which no construction exists yet and which also lock most of the template's own blocks there.

| Item | Status |
|---|---|
| Template conditions put the block on the lattice (Lemma Q) | **proved** |
| K+ transport of block words between blocks of one shape (Lemma TT) | **proved** |
| Closure V W V^-1 with a SAFE vacating word (Lemma V) | **proved** (restated from the chain packages with a full proof) |
| Closure with K+ letters that may carry off pieces (Lemma V') | **proved**; no search yet |
| Block-by-block restoration by the conflict digraph (Theorem CB) and monotonicity (Proposition M) | **proved**, with the vacancy hypothesis per instance |
| Block census: 174 blocks, 71 shapes over X, E1, E2, two mid-depth samples and the fixture | **certificate** |
| Template search on the current state only (validated on X: reproduces D17's Bad sizes 198 and 87) | **certificate** |
| D17 transported to the two mid-depth samples, 17-block restored with zero collateral | **certificate** |
| Block-by-block runs on the two mid-depth samples | **certificate**, partial coverage (Section 5.1) |
| stop62 at 689 off: a word for every block, conflict digraph acyclic with three levels | **certificate**; restoring all of it is *conditional* on vacating words |
| Words carried by K+ to the I_a@40 fixture | **certificate**, partial coverage (Sections 5.2, 5.3) |
| Independent replay of every restoration chain | **certificate** (`certs/verify-*.jsonl`) |
| One-mover, several-frame and frozen blocks (78% of the fixture's off pieces) | **open**; no construction (Section 6) |
| Block lemma (PROOFS-3E Section 2.6) | **open** |
| General 3E | **open**; no date (Section 7) |

## 1. Setting and notation

Notation follows `PROOFS-3E.md` Section 1 and the closure report of the chain package X1
(`Claude-E-closure-X1-20261012/report/A-B-closure.md`).

- A *state* gives every piece p a pose g_p; its *region* is g_p A_p, with A_p the home chamber. A piece is *on the
  lattice* (on K) when g_p lies in K+, the rotation group of order 7,200 that preserves the cut arrangement; it then
  fills one chamber. Every observed off piece is depth one: g_p = K_k^-1 gamma K_r.
- A letter (c, i) at grip c is the menu element E(c, i). It is admissible when no region straddles cap c; it then
  multiplies the pose of every piece whose region is in cap c by E(c, i) on the left. *Lattice regions never
  straddle*: cuts are unions of chamber faces.
- A letter is *SAFE* at a state when its cap holds no off piece and is straddled by none (SAFE-ALL in the chain
  packages). A *mover* of a block T is a grip that no piece straddles and whose cap holds every piece of T.
- *Ghost.* For a word R and a piece, the ghost is the trajectory of the piece's content (frame domain and chamber)
  under the letters of R, computed exactly from the content alone (`ghostlib.py`, `fastghost.py` of the chain
  packages). The ghost is *alive* when the content never straddles the cap of the letter applied, and it *ends on K*
  when the final frame domain is K+. A word is admissible from a state exactly when the ghost of every piece is
  alive, and each piece ends where its ghost ends (the letters act piece by piece).
- *Bad(R)* is the set of chambers whose lattice content would straddle a cut or end off the lattice under R. It
  depends on R only (Proposition S of the restoration report). A lattice piece in a chamber outside Bad(R) stays on
  the lattice under R.
- *Blocks* are classes of off pieces with equal Mov (PROOFS-3E Section 2.6). Lemma G there shows that every block of
  two or more pieces is glued: any word that keeps the other off pieces fixed moves all of the block at its first move.
  A block is named by its least piece (for example "the 87-block @64213").
- *Shape.* The K+-orbit of a block's set of regions. The census key of a block is computed exactly from its frames
  and positions (Section 3).

## 2. Theorems

### 2.1 Lemma Q (the D-type word puts the block on the lattice; proved)

Let T be a set of off pieces with poses g_p, and let A at grip a, B at grip e and X at grip c be letters with A and B
in K+. Suppose, by exact cut tests on the regions of T:

- (Q1) caps a and e each hold every region of T;
- (Q2) A(T) is outside caps c and e;
- (Q3) T is outside cap c, and B(T) is inside cap c;
- (Q4) X^-1 B g_p lies in K+ for every p in T.

Then along Q = A, B, X, B^-1, X^-1, A^-1, X, B, X^-1, B^-1 no piece of T straddles the cap of the letter applied,
and every piece of T ends on K.

*Proof.* Follow one piece p of T, pose g. (1) A: the region is in cap a (Q1), the pose becomes A g. (2)-(5) B, X,
B^-1, X^-1: the region A g A_p is outside caps e and c (Q2), so nothing changes. (6) A^-1: A maps cap a onto itself, so
A g A_p is in cap a and the pose returns to g. (7) X: g A_p is outside cap c (Q3). (8) B: g A_p is in cap e (Q1), the
pose becomes B g. (9) X^-1: B g A_p is in cap c (Q3), the pose becomes X^-1 B g, which is in K+ (Q4). (10) B^-1: the
region is now a chamber, so it does not straddle cap e; the final pose is X^-1 B g or B^-1 X^-1 B g, both in K+. No
step met a straddling region. QED

Remarks.
- Q4 forces every pose of T into one coset B^-1 X K+, so a block whose pieces lie in several frames needs several
  such words (Section 6).
- Lemma Q says nothing about the other pieces. The protected off pieces and Bad(Q) are read from the ghost per
  instance (Section 4). The *pole condition* of the chain packages (G1: X^-1 n_e is a pole) is not needed for Lemma Q;
  it keeps Bad(Q) to the 198-chamber triple lens. Dropping it gives more candidates with larger Bad(Q) (Section 4).
- (Q1) and (Q2) need a != e, so a block needs at least two movers for a D-type word.
- **Lemma Q' (C-type; proved).** If cap e holds T, (Q3) and (Q4) hold, then along [X, B] = X, B, X^-1, B^-1 no piece
  of T straddles and T ends on K: these are steps (7) to (10) of the proof. One mover suffices. The outer conjugation
  by A in Q only serves the other pieces (the two halves cancel on what A keeps away from caps c and e), so a C-type
  word usually disturbs more of the state and has a larger Bad set; the ghost decides per instance.
- With a parking word, W = P Q P^-1, Lemma Q applies to P(T) at the state after P. Whether T stays on K under P^-1 is
  read from the ghost per instance (P contains letters outside K+).

### 2.2 Lemma V (closure; proved)

Let S be a state with off set O, T a subset of O and W a word such that, in S, the ghost of every piece of T is alive
and ends on K, and the ghost of every piece of O \ T is alive and ends at the piece's exact pose. Let V be a word of
letters SAFE at S such that, in the state V(S), no lattice piece lies in a chamber of Bad(W). Then V W V^-1 is
admissible from S, brings every piece of T onto K, leaves every piece of O \ T at its exact pose in S, and leaves
every lattice piece of S on the lattice.

*Proof.* A SAFE letter moves no off piece and maps the chambers of its cap onto chambers of its cap, so V is
admissible, the off pieces keep their poses, and lattice pieces stay on the lattice; by induction the letters of V
stay SAFE along V. The ghost of an off piece depends only on its content, which V does not change, so the premises on
T and O \ T hold in V(S). Every lattice piece of V(S) lies outside Bad(W), so its ghost is alive and ends on K. Hence
W is admissible from V(S) with the stated ends. After W the off set is O \ T at the poses of S, so every letter that
was SAFE at S is SAFE again (its cap holds a subset of the old off pieces at the same regions, that is none, and
nothing straddles it); V^-1 consists of such letters and is admissible, moves no off piece and keeps the lattice on
the lattice. QED

**Lemma V' (closure by K+ letters that may carry off pieces; proved; not yet searched).** Let S, O, T and W be as in
Lemma V. Let V = v_1 ... v_n be a word of letters whose elements lie in K+ and whose grip no region straddles when
the letter is applied (V admissible from S); off pieces may lie in their caps. Suppose that in S' = V(S): (a) the
ghost of every piece of T under W is alive and ends on K; (b) the ghost of every piece of O \ T is alive and ends at
the piece's exact content in S'; (c) no lattice piece lies in a chamber of Bad(W). Then V W V^-1 is admissible from S,
brings every piece of T onto K, returns every piece of O \ T to its exact pose in S, and leaves every lattice piece of
S on the lattice.

*Proof.* A letter with element k in K+ maps cap c onto itself and its chambers onto chambers, so it creates no
straddle at c, keeps lattice pieces on the lattice and off pieces off (k g is in K+ exactly when g is). By (a) to (c),
W is admissible from S' and ends with T on K, O \ T at its poses in S', and the lattice on the lattice. In W(S') the
off pieces are O \ T at their poses in S' = v_n(...), so the grip of v_n is unstraddled there (it was after v_n, and T
and the lattice never straddle); v_n^-1 returns the pieces of O \ T in its cap to their poses before v_n. Inductively
V^-1 is admissible and returns O \ T to its poses in S, while T and the lattice stay on the lattice. QED

Lemma V is the case where no off piece lies in the caps of V; then (a) and (b) in S' are the premises in S. Lemma V'
replaces "SAFE" by "unstraddled with a K+ element", at the price of rechecking (a) and (b) after V. At the fixture this
is 86 grips instead of 13 (Section 5.2). No search for such V has been written yet.

### 2.3 Lemma TT (K+ transport; proved)

For m in K+ (matrix K_m) and a letter (c, i), let m(c, i) = (m(c), i') with E(m(c), i') = K_m E(c, i) K_m^-1 (the
menu is K+-equivariant; `e3lib.Lib.conj_letter` computes i' by an exact product and an exact menu lookup). Write W^m
for the letterwise image of a word W. Then for every region R:

- R is in, out of, or straddles cap c exactly when K_m R is in, out of, or straddles cap m(c) (cap m(c) = K_m cap c);
- along W^m the region K_m R follows K_m applied to the trajectory of R along W, letter by letter; the accumulated
  element is conjugated by K_m, so it lies in K+ exactly when the original does.

Hence Bad(W^m) = K_m Bad(W) as chamber sets (a lattice piece has the identity frame up to K+, so its fate depends on
its chamber only). If m maps the regions of a block T0 in a state Y0 onto the regions of a block T in a state Y,
piece p0 to piece p, then W^m is admissible on the pieces of T exactly when W is on T0, and the final pose of p under
W^m is K_m h K_m^-1 g_p, where h is the accumulated element of p0 under W. It lies in K+ when h g_p0 does and
g_p0^-1 K_m^-1 g_p lies in K+.

*Proof.* The first item is the K+-invariance of the cut arrangement. The second follows by induction over the
letters: if the current region is K_m R_j, the letter m(c_j, i_j) applies K_m E(c_j, i_j) K_m^-1 to it exactly when
R_j is in cap c_j, giving K_m E(c_j, i_j) R_j; the accumulated element is K_m h K_m^-1. QED

Use. Lemma TT is a search device: a word found once for a shape is carried to every block of that shape without a
new search. Equal regions fix a pose only up to the elements that map a chamber onto a chamber, so the end of T on K
is recomputed from the exact poses of the target state (it held in every transport recorded here). The rest of each
step rests on the ghost premises *in the target state* (the other off pieces there are different) and on vacating
K_m Bad(W) there; both are checked per instance before replay.

### 2.4 Theorem CB (block-by-block restoration; proved) and Proposition M (monotonicity; proved)

Let S have off set O, partitioned into target sets T_1, ..., T_r (for example its blocks). For each i choose a word
W_i whose ghost in S brings every piece of T_i onto K, and let Conf(i) be the set of j != i such that the ghost of
W_i in S kills a piece of T_j or ends it away from its exact pose. Draw i -> j for j in Conf(i).

**Theorem CB.** Restore sets one at a time, each time a set i with no remaining j in Conf(i), by V_i W_i V_i^-1 with a
SAFE word V_i that vacates Bad(W_i) in the current state. Every step brings T_i onto K, keeps every remaining off piece
at its exact pose in S and keeps the lattice on the lattice. The conflict sets of the remaining sets only lose
members. If the digraph is acyclic and a vacating word exists at each turn, every off piece of S is brought onto K.

*Proof.* At its turn, i has no remaining conflict, so the ghost of W_i keeps every remaining off piece exact; the
pieces of already restored sets are on the lattice and are covered by V_i like every lattice piece. Lemma V gives the
step. The remaining off pieces keep their exact poses, so their ghosts under every W_j are unchanged: Conf(j) can only
lose the restored set. An acyclic digraph keeps a sink after each removal. QED

**Proposition M.** After a step of Theorem CB: (i) the off set is a subset of the old one at the same poses; (ii) every
grip that was unblocked stays unblocked, every grip that was SAFE stays SAFE, and every mover of a remaining block stays
a mover; (iii) blocks only split, never merge.

*Proof.* (i) is Theorem CB. (ii) A grip is blocked only by off pieces that straddle it, and the remaining off pieces
are a subset at the same regions. (iii) Mov(p) can only gain grips. If Mov(p) = Mov(p') after the step but not before,
some grip h was in exactly one of them before; h was unblocked then, stays unblocked, and its cap still holds the same
one of p and p'. QED

Consequences. A word found for a block at one state stays valid for that block at every later state of the run, up to
the per-instance vacancy, which can change in both directions: the SAFE alphabet grows, but the holes move. If a block
splits, the old word still treats the union and the new parts become conflicts of it; the run checks this by the ghost.

## 3. Block census (CERT-CENSUS)

`census.py` reads the pose table and the block list of a state and records, per block, its size, movers, distinct
exact poses, frames K_k^-1 gamma and the positions of its pieces. A single-frame block has region K_k^-1 gamma A_S, and
two such blocks lie in one K+-orbit exactly when their position sets lie in one S'-orbit (the 360-element group of
PROOFS-3E Section 1), so the shape key is the least sorted image of S under S'. A block with several frames gets one key
per frame and a key for the tuple.

| State | Off | Blocks | Shapes | One frame, >= 2 movers | One mover | Frozen (no mover) | Several frames |
|---|---:|---:|---:|---|---|---|---|
| X | 107 | 3 | 3 | 3 blocks (all) | - | - | - |
| E1 | 90 | 2 | 2 | 2 (all) | - | - | - |
| E2 | 3 | 1 | 1 | 1 (all) | - | - | - |
| e2run24 | 690 | 13 | 12 | 13 (all) | - | - | - |
| stop62 | 860 | 15 | 12 | 15 (all) | - | - | - |
| I_a@40 fixture | 94,147 | 140 | 57 | 88 blocks, 18,476 pieces | 32 blocks, 38,576 pieces (one frame); 11 blocks, 15,023 pieces (several frames) | 1 block, 19,720 pieces (8 frames) | 8 blocks with >= 2 movers, 2,352 pieces |

stop62 and e2run24 are the two deep samples of the seed-table stage (called mid-depth samples here, to set them apart
from the 94,147-piece fixture): states on E3's own history, 62 twists after the fixture and 24 twists after that
(CERT-HIST of the seed-table package), with 860 and 690 off pieces. Across the six states there are 174 blocks and 71
shapes. Nine shapes occur in more than one state:

| Shape key | Size | Occurrences |
|---|---:|---|
| da4b45e3 | 111 | e2run24 2, stop62 3, fixture 8 |
| fca5f066 | 86 | e2run24 1, stop62 2, fixture 5 |
| abd79147 | 87 | e2run24 1, stop62 1, fixture 4 |
| f96a9658 | 3 | X, E1, E2, e2run24, stop62 |
| 16121635 | 17 | X, e2run24, stop62 |
| 6bd3c2d1 | 87 | X, E1 |
| fa45d23d | 43 | e2run24, stop62 |
| 7cbcee17 | 29 | e2run24, stop62 |
| 11d19793 | 12 | e2run24, stop62 |

Inside the fixture, 140 blocks fall into 57 shapes; the most frequent shapes occur 16, 15 and 12 times. So a word found
once per shape covers several blocks, which is what Lemma TT is for. The table also shows the limit of the D-type
template: Lemma Q needs two movers (a != e), and 78% of the fixture's off pieces lie in blocks with one mover or none.
C-type words (Lemma Q') need one mover. A block with no mover can only gain one when the off pieces that straddle its
caps are restored (Proposition M: blocked grips only become unblocked).

## 4. The template search (CERT-DFAM)

`dfam.py` searches the D-type family for one block at one state, from the pose table only:

1. Q candidates by Q1 to Q4 (Lemma Q): e a mover and B a non-identity K+ letter at e; c an unstraddled grip with T
   outside cap c and B(T) inside cap c; X a letter at c outside K+ with X^-1 B g in K+ for every pose g of T; a a
   mover with a != e and A a non-identity K+ letter at a with A(T) outside caps c and e. By default e must be a pole
   of X's frame (G1 of the chain packages); `--no-pole` drops that condition and scores those candidates last.
2. Each Q is scored by the exact ghost: T ends on K, every other off piece is alive and returns to its exact content,
   Bad(Q) and its occupancy, and the blocks whose pieces the ghost kills or moves (its conflict set).
3. Parking for the best Q's: P = [[X, Y], Z] with Y a K+ letter at a cap holding all of Bad(Q) (a pole of X's frame,
   not c) and Z outside K+ with Z n_e = n_c; W = P Q P^-1 is scored the same way.

Validation on X (17-block): 60,588 candidates; of 1,500 scored, 1,493 put T on K with every other off piece exact,
all with |Bad(Q)| = 198 (the triple lens of the chain packages); parking gave 810 such W, the best with |Bad(W)| = 87,
the sizes of D17 (`runs/dfam/X-b01.summary.json`).

`--blocks <file>` (added for the fixture) reads the block list from a file instead of computing it, and computes the
contents of the off pieces once; the decisions are the same. On the fixture a search loads in about 25 s and scores
a word in about 0.2 s.

**A simpler family that does not scale (Type S).** `tshrink.py` generalizes the B' construction of E2 (one letter R
that puts T on K, then iterated conjugation by SAFE letters until a finishing letter clears Bad(R^-1)). On E2's
3-block it reproduces B' (finishing letter at level 4). On e2run24 every one of the 13 blocks has such letters R (12
to 48 each), but for the first four tried per block, R also put another off piece on K (9 blocks) or the conjugation
stopped without a step letter or at the level limit (4 blocks) (`runs/typeS/`). The D-type family replaced it.

## 5. Measured coverage (CERT-TRANSPORT, CERT-BLOCKRUN, CERT-FXRUN)

Every step below was replayed on the simulator from the pose table of its input state, every twist checked
admissible, and compared with the input: the target block on K, *collateral* (lattice pieces of the input that end
off) 0, every other off piece at its exact pose, every lattice piece of the input on K at the end. Some lattice pieces
end at another K pose (counted apart in each `result.json`); the lattice phase handles those. Each step records its
input and output digests, so the steps of a run chain.

### 5.1 The two mid-depth samples

| State | Step | Block | W | \|Bad(W)\| | V (certificate) | Length | Off after |
|---|---:|---|---|---:|---:|---:|---:|
| stop62 (860 off, 15 blocks) | 1 | 17-block | D17 carried from X by K+ (P Q P^-1, 30 twists) | 87 | 404 | 838 | 843 |
| | 2 | 111@3644 | Q (10 twists) | 111 | 37 | 84 | 732 |
| | 3 | 43@9104 | Q (10) | 111 | 407 | 824 | 689 |
| e2run24 (690 off, 13 blocks) | 1 | 17-block | D17 carried from X | 87 | 242 | 514 | 673 |
| | 2 | 43@9104 | Q (10) | 111 | 274 | 558 | 630 |
| | 3 | 50@16589 | Q (10) | 111 | 413 | 836 | 580 |

V is built from the peeling certificate of Theorem B (chain packages) at the step's own state; the length is
|V| + |W| + |V^-1|. Lattice pieces of the input that end at another K pose: 0 for the two D17 steps, 547, 581, 571 and
575 for the four Q steps. Coverage: stop62 171 of 860 off pieces (19.9%), 3 of 15 blocks; e2run24 110 of 690 (15.9%),
3 of 13 blocks. Each closure took 140 to 270 s.

**Why the runs stopped.** The SAFE alphabet is large at these states (552 and 555 of 600 grips SAFE) and there are as
many holes as off pieces (689 and 580). The template is not what stops them:

- *stop62 at 689 off (12 blocks).* Five blocks have clean words (T on K, every other off piece exact): 86@1040 (599
  of 601 scored), 3@16245 (601), 12@18445 (564), 15@18517 (559) and 87@64213 (131). None of the other seven has a
  clean word among its 1,500 scored words; its words with the smallest conflict set (708 to 1,439 of the 1,500)
  disturb only the blocks named here:

  | Block | Its words disturb |
  |---|---|
  | 78@18512, 10@48298 | 15@18517 |
  | 61@23955 | 78@18512, 15@18517, 10@48298 |
  | 29@21607, 111@64191, 111@68424 | 87@64213 |
  | 86@64184 | 111@64191, 87@64213, 111@68424 |

  With these words the conflict digraph is acyclic with three levels (`certs/conflict-stop62-689.json`, written by
  `mkconflict.py` from the search summaries `runs/dfam/stop62-689-*`), so Theorem CB orders the whole state: 15@18517,
  then 78@18512 and 10@48298, then 61@23955; 87@64213, then 29@21607, 111@64191 and 111@68424, then 86@64184; and
  86@1040, 3@16245, 12@18445. *Conditional* on a vacating word at each turn, the family restores all 689 off pieces of
  this state.
- *The vacating word is what fails.* Every recorded try with |Bad(W)| >= 333 failed the necessary condition N2 (some
  SAFE orbit has fewer holes than Bad(W) chambers to vacate, so no SAFE word can vacate it): 86@1040 (eight words,
  |Bad| 333 to 592, deficits in two orbits), 48@8888 (|Bad| 394 and 481, two orbits), 12@18445 (|Bad| 333, twelve
  orbits). The tries with |Bad(W)| = 198 met N2, but the peeling certificate failed and the greedy search stopped with
  58 to 64 occupied chambers left after 600 s (15@18517, 48@18491). Every success had |Bad(W)| = 87 or 111.
- *e2run24 at 580 off (10 blocks).* The same picture: 48@18491, 48@8888 and 12@18445 fail on vacancy as above; the
  86-, 111- and 87-blocks at 64184, 64191, 68424 and 64213 are the same cluster as in stop62 (111@68424's words
  disturb 87@64213); 45@35795 has only two movers, and all its 360 D-type and 180 C-type words disturb 48@18491
  (measured at 630 off, `runs/dfam/e2run24-630-45at35795-*`).

Both runs were stopped by hand on 14 October 2026 after their last restoration, while they cycled through words of
the vacancy-limited blocks (`runs/blockrun-*.jsonl` hold every try).

### 5.2 The deep fixture: carried words

The template library holds the words found at the mid-depth samples for three shapes: 111 (five source blocks), 86
(three) and 87 (two), at most 10 to 20 words each (`data/templates/templates-fxrun1.json`, with the source tables,
blocks and words beside it). Of the fixture's 140 blocks, 17 have one of
these shapes (1,666 pieces). `fxrun.py` finds every exact m in K+ with m(source regions) = target regions, carries
the words letter by letter (Lemma TT), 1,145 words in all, scores each by the exact ghost of all 94,147 off pieces
and of the chambers of its caps, and replays a clean word (T on K, no other off piece killed or moved, Bad(W) vacant,
so V is empty):

| Step | Block (index, first piece) | Template | m | W | \|Bad(W)\| | Off after |
|---:|---|---|---:|---|---:|---:|
| 0 | 113, 111@81217 | 111@64191 of stop62 | 732 | Q (10 twists) | 198 | 94,036 |
| 1 | 120, 111@171114 | 111@3644 of stop62 | 6,750 | Q (10) | 198 | 93,925 |
| 2 | 122, 87@89647 | 87@64213 of stop62 | 3,460 | Q (10) | 198 | 93,838 |

Separately, block 115 (111@95605) was restored from the fixture itself by the word of 111@3644 carried by m = 6,269
(10 twists, Bad(W) vacant; `runs/chains/fixture-115/`). In every case Bad(W) was already vacant: at 94,147 off pieces
there are as many holes, and the chambers around a block are often holes.

After step 2 no carried word was clean (`runs/fxrun1.jsonl`, last line). For the 14 remaining blocks with templates:

| Reason | Blocks |
|---|---|
| Every carried word uses a grip that another off piece straddles | 4 (109, 111, 121, 126) |
| Unblocked words exist, but each disturbs other blocks | 6 (118, 119, 124, 130, 131, 135) |
| Clean ghost premises, but Bad(W) holds lattice pieces (63 to 198 chambers) | 4 (115, 116, 123, 136) |

Block 115 moved to the last row: the three restorations filled chambers of its Bad(W) with lattice pieces. Restoring
blocks uses up holes, which is the mid-depth regime starting again.

**SAFE letters at depth.** After step 2 only 13 of the 600 grips are SAFE; 514 are straddled by an off piece and 73
are unstraddled with off pieces inside (`certs/safe-count.jsonl`). So Lemma V with a SAFE vacating word is almost
never available at depth, and the last row of the table cannot be finished by the closure used at mid-depth.

### 5.3 The deep fixture: families of new shapes

The three mid-depth shapes cover 17 of the fixture's 140 blocks. To reach more, the template search ran on the fixture
itself (lite mode, Section 4) at step 3 (93,838 off), on one block of each of the four most frequent shapes among the
one-frame blocks with two or more movers (`runs/dfam/fixture-D-*`):

| Shape | Blocks (pieces) | Searched block | Q candidates (scored) | Clean | \|Bad(Q)\| of clean words | Reading |
|---|---|---|---:|---:|---|---|
| cdd6ff93 | 6 (1,188) | 198@13039 | 2,496 (301) | 121 | 309 to 1,186; 34 with Bad(Q) vacant | restorable now |
| 74fb97fc | 12 (4,740) | 395@7279 | 324 (301) | 201 | 592 or 790; 37 to 395 occupied, none vacant | needs a vacating word |
| 3b2cd040 | 16 (3,152) | 197@13010 | 792 (600) | 0 | - | every recorded conflict set (589 of the 600 words) contains 198@13039 |
| 9162556f | 15 (5,925) | 395@1158 | 108 (108) | 0 | - | every word moves the one-mover, two-frame 2,702-block |

The second transport run (`runs/fxrun2.jsonl`) starts from step 3 with all 14 templates
(`data/templates/templates-fxrun2.json`: the ten mid-depth templates and the 20 best words of each of the four searches
above) and carries 2,135 words to the 63 blocks of these seven shapes that are still off:

| Step | Block (index, first piece) | Template | m | W | \|Bad(W)\| | Off after |
|---:|---|---|---:|---|---:|---:|
| 3 | 65, 198@13039 | its own search (cdd6ff93) | 0 (identity) | Q (10 twists) | 309 | 93,640 |
| 4 | 71, 198@103934 | the same | 1,343 | Q (10) | 309 | 93,442 |
| 5 | 69, 198@48781 | the same | 2,900 | Q (10) | 396 | 93,244 |
| 6 | 73, 198@122617 | the same | 2,670 | Q (10) | 396 | 93,046 |

Then a full scoring pass found no clean word, and the run stopped (`certs/fixture-stop-run2.json`, written by
`fxstop.py` from the run's last line). The whole fixture chain, steps 0 to 6, replays independently with zero
collateral, every other off piece at its exact pose and every lattice piece on K (`certs/verify-fixture.jsonl`; at step
6, 198 lattice pieces end at another K pose).

**Coverage at the fixture: 7 blocks, 1,101 of 94,147 off pieces (1.2%).** The seven template shapes cover 66 blocks
(16,671 pieces, 17.7%). The 59 that remain, classified by their best carried word:

| Reason | Blocks | Pieces |
|---|---:|---:|
| Every carried word uses a grip that another off piece straddles | 26 | 8,096 |
| Unblocked words exist, but each moves other off pieces | 21 | 4,512 |
| Clean ghost premises, but Bad(W) holds lattice pieces (37 to 198 chambers) | 12 | 2,962 |

For 39 of the 47 blocks in the first two rows (11,598 pieces), the best word's conflict set contains a block of none of
the seven shapes: one of the one-mover, several-frame or frozen blocks of Section 6 (the frozen 19,720-block alone
locks 12 of them). Only 8 blocks (1,010 pieces) are locked by template blocks alone and would follow in the order of
Theorem CB. Two effects of the restorations show in the run. Conflicts resolve as Theorem CB says: 197@13010, whose
words all moved 198@13039 at step 3, has clean ghost premises after step 6. Holes fill: 198@95589 had clean words with
Bad(W) vacant at step 3, and after step 6 its Bad(W) holds 87 lattice pieces. With 13 SAFE grips, the 12 blocks of
the last row need the vacating search of Lemma V'; the other 47 wait mostly on blocks outside the template.

## 6. What the family does not reach (CERT-CENSUS, CERT-BLOCKRUN, CERT-FXRUN)

Three limits are structural (they follow from Lemma Q and Proposition M), two are measured.

1. **Fewer than two movers** (structural). Lemma Q needs a != e. A one-mover block has only C-type words (Lemma Q'),
   which act on everything in caps c and e, so every other off piece there is moved unless it happens to return.
   At the fixture, one-mover blocks hold 53,599 off pieces (57%). Measured on three of them at step 3 of the
   fixture (`runs/dfam/fixture-C-*`): 2,505@4484 and 395@22327 have no C-type candidate at all; 197@21328 has 96,
   each of which puts the block on K but moves 261 to 1,246 other off pieces. C-type words also fail where D-type
   words exist: the two-mover 45@35795 of e2run24 has 360 D-type and 180 C-type words, and all of them disturb
   48@18491.
2. **Poses in several frames** (structural). Q4 puts every pose of T in one coset B^-1 X K+, and a glued block moves
   as one at its first move, so a block whose poses lie in several frames needs a word whose trajectory separates
   the parts. The template does not provide one (`dfam.py` stops with "poses in several domains"). At the fixture: 8
   two-mover blocks (2,352 pieces), 11 one-mover blocks (15,023) and the frozen block.
3. **No mover** (structural). The frozen block of the fixture (19,720 pieces, 8 frames) has every cap that holds it
   straddled. By Proposition M a blocked grip becomes free only when the off pieces that straddle it are restored, so
   it can only come last, after its straddlers.
4. **Vacancy at mid-depth** (measured, Section 5.1). Necessary condition N2 fails for every recorded word with
   |Bad(W)| >= 333; for |Bad(W)| = 198 neither the certificate nor the 600-second greedy search found V. Whether
   these are limits of the search or of the state is open for |Bad(W)| = 198 and settled (no SAFE V exists) for the
   N2 failures.
5. **No SAFE letters at depth** (measured, Section 5.2). 13 SAFE grips of 600 at the fixture, so the closure of
   Lemma V reduces to "Bad(W) already vacant", which the restorations themselves erode.

Together, at the fixture: the D-type template applies to the 88 one-frame blocks with two or more movers (18,476
pieces, 20% of the off pieces), and within those only where a carried or searched word is clean with Bad(W) vacant.

## 7. Go/no-go for general 3E

**Go for the family as a component.** The template, its transport and the block-by-block order are proved (Lemmas Q,
Q', V, V', TT, Theorem CB, Proposition M). Every measured restoration replays independently with zero collateral and
every other off piece at its exact pose (`certs/verify-*.jsonl`), and words found at the mid-depth samples were
carried by K+ to the 94,147-piece fixture and restored blocks there. A word is found once per shape and reused.

**No date for general 3E.** The measurements locate two different obstacles, and only the first has a route:

- *Mid-depth: vacancy.* The template is not the limit there: at stop62 @689 every block has a word and the conflict
  digraph is acyclic. What fails is the vacating word for |Bad(W)| >= 198 under Lemma V (SAFE letters only).
- *Depth: blocks outside the template.* 78% of the fixture's off pieces lie in blocks with one mover (C-type only,
  and the three measured one-mover blocks have no clean C-type word), in blocks whose poses span several frames, or
  in the frozen block. For these I have no construction. They also hold up the template's own blocks: at the end of
  the fixture run, 39 of the 59 remaining template blocks (11,598 pieces) are locked by one of them, 8 only by other
  template blocks, and 12 need only a vacating word (Section 5.3).

**Next stage, in order, each with its exit test.**

1. *Vacating search under Lemma V'.* Search V among unstraddled K+ letters that may carry off pieces, with the
   premises (a) to (c) checked after V. Exit: the 689-piece state of stop62 restored completely in the order of
   Section 5.1 (12 blocks), and the 12 fixture blocks whose only defect is an occupied Bad(W) (2,962 pieces) restored.
   This is the only step with a route; it decides whether the mid-depth regime closes.
2. *Parking at mid-depth.* Parking words that bring |Bad(W)| down to at most 111 (every success so far had 87 or
   111), as the fallback for step 1.
3. *A one-mover family.* 57% of the fixture's off pieces. Exit: a clean word for one of the three measured one-mover
   blocks. No route yet; Lemma Q' alone does not give one.
4. *Several-frame blocks.* A word whose trajectory separates the frame parts of a glued block. No route yet.
5. *The frozen block* comes last by Proposition M, after the blocks that straddle its caps.

Decide again after step 1. While steps 3 and 4 have no route, any date for general 3E would be a guess.

## 8. Open items

- The Block lemma (PROOFS-3E Section 2.6) and general 3E: **open**.
- Lemma V' is proved but has no search. Whether the |Bad(W)| = 198 vacancy failures are limits of the search or of
  the state is open; the N2 failures (|Bad(W)| >= 333) are settled for SAFE words only.
- One-mover, several-frame and frozen blocks: no construction (Section 6).
- C-type words were tried on four blocks only (45@35795 of e2run24 and three one-mover blocks of the fixture).
- The two mid-depth runs were stopped by hand after their last restoration while they cycled through
  vacancy-limited words; the fixture transport runs stopped at their own stop condition (Section 5.3).
- The new proofs have had no independent mathematical review.
- Lattice pieces that end at another K pose are left to the lattice phase (Card 5); this stage does not count them as
  restored or as damaged.
- Not run: anything on Windows, DirectX, the native host, input or performance. No repository file was changed, so
  no repository check applies; every script here only reads the research tree.

## 9. Certificates, data and scripts

All scripts run with `python3 -I` from `scripts/`, with the research tree `research/jumbling` (commit `27a001c`) and
the five chain-package scripts in `scripts/echain/` (`ghostlib.py`, `fastghost.py`, `conjprep.py`, `conjclose.py`,
`vacate.py`). `e3lib.py` also needs `classes.json` and `arrays.npz` from the seed-table package (`data/`). Paths
inside the JSON logs are written as `<scratch>`, `<research/jumbling>` and `<echain>` in place of local directories.

| Certificate | Files | Script |
|---|---|---|
| CERT-CENSUS | `data/census-all.json`; fixture block list `data/blocks-fixture40.json` | `census.py`, `mkblocks.py` |
| CERT-DFAM | `runs/dfam/*.summary.json` (X, stop62 at 843 and 689, e2run24 at 673 and 630, fixture) | `dfam.py`, `currows.py`, `bestq.py`, `run_dfam.sh`, `batch_dfam.sh` |
| CERT-TRANSPORT | `runs/chains/stop62/s1-*`, `runs/chains/e2run24/s1-*` (D17 carried by K+) | `transport.py`, `tghost.py`, `mghost.py`, `wreplay.py` |
| CERT-BLOCKRUN | `runs/blockrun-*.jsonl`; every step's word, target, result, vacating certificate and premises in `runs/chains/` | `blockrun.py`, `tclose.py`, `vacate2.py`, `echain/conjclose.py`, `echain/vacate.py` |
| CERT-FXRUN | `runs/fxrun1.jsonl`, `runs/fxrun2.jsonl`; templates in `data/templates/` | `fxrun.py`, `e3lib.py` |
| CERT-REPLAY | `certs/verify-*.jsonl`: every chain replayed from its start table, twist by twist | `verify_chain.py` (simulator, tables and words only) |
| CERT-SAFE | `certs/safe-count.jsonl` | `safecount.py` |
| CERT-CB | `certs/conflict-stop62-689.json` | `mkconflict.py` |
| Type S | `runs/typeS/*.summary.json` | `tshrink.py`, `run_ts.sh` |

Start tables: `data/states/` (stop62, e2run24 and the I_a@40 fixture; the same files as `data/deep/` of the
seed-table package). `SHA256SUMS.txt` lists every file of this package.
