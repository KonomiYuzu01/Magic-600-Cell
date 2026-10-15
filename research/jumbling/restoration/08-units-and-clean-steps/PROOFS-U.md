# PROOFS-U: units and clean steps, multi-frame and frozen blocks, and where the block family stops (menu I_a)

**Changes from v3 (this is v3.1):** a carrier-move step takes stop62 from 233 to 86 or 61 off pieces (Lemma CM,
proved; three steps replayed independently); the driver and the gluing check were run at the three end states; the
findings of an internal consistency audit were applied (where the driver stops, root sets, hypotheses, status labels,
references, reproduction commands).

15 October 2026. Package `Claude-3E-units-20261015`, version 3.1 of the third package of the block-family stage; it
supersedes `Claude-3E-units-20261014`. Earlier packages: `Claude-3E-block-family-20261014` and
`Claude-3E-general-family-20261014`. Linux headless exact computation on synthetic geometry; research tree
`research/jumbling` at commit `27a001c` (branch `claude/jumbling-e3`). No Windows, DirectX, input, long-session or
performance claim. The new proofs have not been reviewed independently. **General E (restoration from every reachable
state) is not claimed solved.**

Status labels: **proved** (proof in this file), **certificate** (exact computation with a record that `scripts/`
replays or recomputes), **measured**, **conditional**, **open**.

## 0. Summary

**Question 1: do the parameters always exist (the Block lemma)?**

- *Weak form: yes (proved).* At every reachable state every unit has a clean step whose free set is the rest of the
  off pieces; an inverse history is one. The state does not show it. Section 5.1.
- *Depth one: yes, from the state alone (Theorem D1, proved).* One jumble twist away from the lattice, each of the 12
  coset letters at the twist's grip is a clean step that puts all 3,097 pieces back. The driver's first try is such a
  letter (Corollary D1, from the driver's code; instance replayed). Section 6.4.
- *Family form: false for the family of roots and refinements of depth one, at an explicit reachable state
  (certificate); open for deeper refinements.*
  - The driver takes stop62 from 689 to 245 off pieces in 8 clean steps and stops there (run `s62g`: "no unit
    restored at this state"). Step 9 (245 to 233) is not a driver step: a refinement [Q, k] of depth one of the
    12-piece block (|Bad| = 41), found by the computation behind Theorem Z and closed by `tclose2.py`. All 9 steps
    have collateral 0 and every other off piece exact (certificate: independent replay). Section 6.1.
  - At 245 no word of the whole base family of the four remaining units (every single jumble letter, and the 37,380
    D-type and C-type words with every parameter) can be closed: each has a Bad set of at least 333 chambers, more than
    the 245 holes, and by Lemma D no word of K+ letters can vacate it, whatever the free set (Theorem S, Corollary S;
    certificate, with an independent recount of all 37,380 words).
  - At 233 only the three carriers of the rare types 7 and 9 are left, and 15 of the 36 types have no hole. Every
    root and every landing refinement of depth one of every carrier has a Bad chamber of the hole-free type 29
    (Theorem Z, certificate). There are 23,703 landing roots. The 677,372 landing refinements of the roots without a
    dead type-29 content have at least 8 such chambers each; every refinement of the 3,965 roots with a dead type-29
    content keeps that dead chamber (Lemma I). The roots taken from the earlier pools are the words usable at 245. The
    independent recount for Theorem Z covers only a sample: 483 refinements of depth one at 245, in types 29, 30 and
    32. So the driver started at 233 from the state alone has no step there, whatever its parameters and even with
    closure by any word of K+ letters (Corollary Z; certificate and the driver's code). Seeded with every stop62 pool
    it stops there too (measured; Theorem Z does not cover that run).
  - Depth two: refinements that meet Lemma D at 233 exist (42 found in a partial run), but for each the SAFE-ALL
    orbits of types 7 and 9 hold no hole, so the driver's closure cannot vacate their Bad sets (certificate per word,
    N2).
  - *Glued holes (Proposition GH, proved; certificate at 233).* Each of the 9 holes of types 7 and 9 at 233 is glued
    to one carrier, 3 per carrier: no word of K+ letters that returns the carriers to their poses moves it, Lemma V*
    included. A word of K+ letters that moves a whole carrier moves its glued holes with it (proof of Proposition
    GH).
  - *Carrier-move closure (Lemma CM, proved; three steps, certificate).* At 233 a prefix of 3 to 5 K+ letters moves a
    second carrier B so that its glued holes become the rare Bad chambers of a depth-two word W of carrier A outside
    A's own holes. Then V W V^-1, with B in the free set, restores A, and W lands B as well. Three such steps were
    replayed independently: 233 to 86 (twice: 86@1040 or 86@64184 left) and 233 to 61 (61@23955 left), collateral 0.
    They are outside the driver; Corollary Z is unchanged. Section 6.1.
  - *The last carrier.* At each of the three end states the last carrier's 3 holes of types 7 and 9 are still glued to
    it (certificate). So by Corollary GH a step V W V^-1 with V(U) = U needs Bad(W) in these types to be exactly these
    3 holes. No root and no refinement of depth one meets even the count (Certificate 5); a partial depth-two run and
    a closed-set search on sampled roots keep the excess at least 3 (measured). The driver started at each end state
    stops at once: no usable word meets the vacancy conditions, and every usable word has |Bad(W)| >= 333 (measured).
    With one carrier left there is no other carrier to move. In most words examined the cause is a copy of the
    carrier's rare pieces that the refinement letters move but do not remove (Section 5.3, measured); a proof for
    every depth is not done. Below a word with a dead ghost no refinement of any depth helps (Lemma I, Corollary I,
    proved). Parked words are not used by the driver and fail anyway (Certificate 6).
- *What is missing is construction, not existence.* The last carrier of a rare type needs a type-swap word: Bad(W)
  no larger than the carrier in that type (Corollary D''). Such words exist (an inverse history is one; proved), but
  neither the base family nor the searched pools contain one for the three carriers of types 7 and 9 at stop62
  (certificate; smallest excess 3: Certificate 2 of Section 6.1, summarised in Section 5.3).
- *New theory for refinements (proved, Section 3.5).* Kept chambers form a bijection (Lemma K); the refinement
  identity (Proposition J) gives |Bad([Q, k])| per type from Q's data alone, so one forward and one inverse ghost of Q
  decide all its refinements (the depth-two runs of Section 6.1 use this); dead lattice ghosts are inherited by every
  deeper refinement (Lemma I).

**Question 2: multi-frame blocks and frozen blocks?**

- *Multi-frame blocks need no separate construction (proved, Lemma U).* A frame is a unit; the other frames of its
  block go into the free set; a clean step lowers the off count by at least the unit's size whatever the free set.
  Instance (certificate): from the end of run r40s11, a 111-piece block and then the 86-piece frame of block 7127 (two
  frames) are restored with collateral 0; the other frame (45 pieces) becomes a single-frame block that stops on the
  vacancy conditions. On the end state of the 60-record run, the four frame units of the two multi-frame blocks (94
  and 147 pieces) stop at once on the vacancy conditions (smallest n2 = 2, deficit 0), not on a frame problem
  (measured: a search outcome). Section 6.2.
- *Frozen blocks: Proposition F (proved).* Once the pieces that straddle an inside grip of a frozen block are
  restored, that grip is a mover and the block is an ordinary unit. Instance (certificate, chains `frozen` and
  `frozen2`): from x16-1076 (21,113 off, 31 frozen blocks), six steps with collateral 0 reach 12,149. One letter
  unlocks five frozen blocks sharing grip 315, which are then restored in one step; at 15,590 one letter at grip 168
  leaves no block frozen at all (26 frozen before), and two more frozen blocks are restored. The run stops on the
  vacancy conditions for the last two (3009, 25409), both with a mover. Section 6.3.

**What completing 3E requires (for a restorer of this kind).**

1. A restorer that works from the state alone with clean steps finishes from every reachable state if and only if the
   family form of the Block lemma holds for its word family. By Lemma U every step lowers the off count, so a run
   that always has a step reaches 0; a reachable state without a step is where it stops.
2. Each step has two parts. The word part (a word that puts the unit on the lattice with the other off pieces exact
   or free) is not the bottleneck: in every state examined every unit with a mover had such words, and the tries
   of the driver that failed, failed at the closure (measured); frozen blocks reduce to units with movers (Proposition F). The
   closure part is the bottleneck: Bad(W) must be emptied by K+ letters, which needs |Bad(W)_t| <= off_t(Z) for every
   type t (Lemma D).
3. So a necessary condition at the end of every run is a type-swap word for the last carrier of each type
   (Corollary D''). The base family does not provide one at stop62 (Theorem S; Certificate 2 of Section 6.1), nor do
   its refinements of depth one (Certificate 5). The driver as built stops earlier: the actual run stops at 245, and
   started at the reachable state stop62@233 (three carriers left) it has no step (Corollary Z). Its words of depth
   one meet a hole-free type. The words of depth two that meet Lemma D have rare Bad chambers outside the unit's own
   holes, and the holes of those types are glued to the other carriers (Proposition GH), so a closure that returns
   every carrier to its pose cannot vacate them. A step outside the driver that first moves a second carrier with its
   glued holes restores two carriers at once (Lemma CM; 233 to 86 or 61, certificate). The last carrier then needs, for V
   with V(U) = U, a word whose Bad set in types 7 and 9 is exactly its own 3 glued holes (Corollary GH; its hypothesis is checked at
   the three end states). Proposition GH does not exclude such a word; none is known.
4. Two ways forward: (a) a word class built from the state whose Bad sets meet the per-type bound, with a vacancy
   step that carries an off block together with its glued holes (Lemma V* does not reach glued holes). At stop62 this
   is realised for all but the last carrier (Lemma CM, certificate). For the last carrier there is no other carrier
   to move: it needs a word whose rare Bad set is exactly its own glued holes, or a partner created first; open.
   (b) Steps that first take lattice pieces off and later return them, with a proof that the run still ends (a
   potential other than the off count); open.

**Errata** to the first two packages: Section 7.

**Contents.** 1 Setting and notation. 2 Units and clean steps. 3 Words, Lemma D and Lemma V*. 4 The driver and its
correctness. 5 The Block lemma. 6 Runs and certificates (6.1 includes the carrier-move step). 7 Errata.
8 Reproduction. 9 Files.

## 1. Setting and notation

Notation follows `PROOFS-BF.md` Section 1 (first package) and `PROOFS-GF.md` Section 1 (second package). NP = 177,120
pieces and as many chambers; every cap is the union of 3,097 chambers; K+ has order 7,200; a letter (c, i) of I_a is
admissible when no region straddles cap c, and multiplies the pose of every piece inside cap c by E(c, i).

- Fact 1 (`PROOFS-GF.md`): in a reachable state the regions are distinct, so the holes are exactly off(Z) many.
- Fact 2 (`PROOFS-GF.md`): a K+ letter never changes whether a piece is on the lattice.
- A *block* of Z is a rigid component (`theory/blocks.py`: no unstraddled cut separates its pieces). Its *inside grips*
  are the grips whose cap holds it; its *movers* are the inside grips that no region straddles. A block is *frozen*
  when it has no mover.
- The *type* of a chamber is its K+ orbit. There are 36 types. `regions.orbit_of` numbers them 0 to 35 and
  `data.orbit_id` numbers them -1 to 34 (the orbit of 600 chambers is -1); the two labellings give the same partition
  (checked over all 177,120 chambers). A lattice piece always sits in a chamber of its own type, because K+ maps
  chambers of a type onto chambers of the same type. The type of a piece is the type of its home chamber.
- **Fact 1' (holes per type; proved).** In a reachable state the holes of type t number exactly the off pieces of
  type t. *Proof.* Each chamber is the home of exactly one piece, so N_t chambers of type t are the homes of N_t
  pieces of type t. A lattice piece of type t occupies a chamber of type t, distinct pieces occupy distinct chambers
  (Fact 1), so the holes of type t number N_t minus the lattice pieces of type t, which is the off pieces of type t.
  QED

## 2. Units and clean steps

**Definition (unit).** Let B be a block of Z. If all pieces of B lie in one frame domain, B is one unit with an empty
free set. If they lie in the domains d_1, ..., d_m (m >= 2), B gives m units U_j = {p in B : dom(p) = d_j}, each with
the free set F_j = B \ U_j.

**Definition (clean step).** Let U be a unit of Z and F any set of off pieces of Z disjoint from U (the *free set*).
A word W applied at Z is a *clean step for U with free set F* when every letter is admissible when applied, every
piece of U ends on the lattice, every off piece of Z outside U and F ends at its exact pose, and every lattice piece
of Z ends on the lattice (possibly in another chamber). The free set of a frame unit contains the rest of its block;
the driver of Section 4 adds the other blocks that W moves but can carry (*free conflict blocks*).

**Lemma U (progress; proved).** A clean step lowers the off count by |U| plus the number of pieces of F that end on
the lattice, and leaves every off piece outside U and F at its exact pose.

*Proof.* The off pieces at the end are among off(Z) \ U; the lattice pieces of Z stay on. QED

The proof does not use the structure of F, so Lemma U holds for any free set. A free set costs exactness, not
progress: the free pieces may change their poses (and may straddle new grips afterwards), but the off count still
falls by at least |U|.

So a multi-frame block needs no separate construction: one clean step restores one of its frames and may move the
others, which stay off (or land on the lattice) and are listed again at the next state. Its progress is at least the
size of the restored frame. Monotonicity in the sense of Proposition M of `PROOFS-BF.md` (the other off pieces at
the same regions) holds for steps with an empty free set. A step with a free set keeps everything outside the block
exact, but the moved frames may straddle new grips; the driver lists the units again after every step, so this
affects the plan, not correctness.

## 3. Words

### 3.1 D-type and C-type words (`PROOFS-BF.md` Lemma Q and Lemma Q'; proved)

Q = A, B, X, B^-1, X^-1, A^-1, X, B, X^-1, B^-1 needs two movers a != e of the unit and a letter X at a grip c with
X^-1 B g_p in K+ for every pose g_p of the unit (Q1 to Q4). The C-type word [X, B] = X, B, X^-1, B^-1 needs one mover
(Lemma Q'). Q4 forces all poses of the target into one coset B^-1 X K+: this is why the target is a unit (one frame),
not a whole multi-frame block.

### 3.2 Commutator refinement (search device; the exact ghost decides)

For a word W whose ghost at Z puts the unit U on the lattice at the landing chambers L, keeps the protected pieces
exact and the free pieces admissible, and a K+ letter k at a grip whose cap holds L (exact chamber test), the
refinement is

    [W, k] = W, k, W^-1, k^-1.

*Why it often works.* After W, U lies in L on the lattice; k moves it to k(L), still on the lattice; if W^-1 does not
take it off there, k^-1 returns it to L. Everything that k leaves alone is undone by W^-1, so the pieces that W
knocked off the lattice come back unless k moved them in between. The Bad set of [W, k] is therefore what k moves
inside the support of W, usually far smaller than Bad(W).

*Filter (heuristic).* A lattice piece that k^-1 brings into L meets W^-1 on U's path and is knocked off; candidates
for which k^-1 maps L into chambers that are neither holes, nor in Bad(W), nor in L are dropped before scoring.

*Decision.* Every candidate is scored by the exact frame ghost of U, of every other off piece and of every chamber of
the word's caps (`comm3.py`): U on the lattice, protected pieces exact, free pieces admissible, Bad and its
occupancy, and the per-type deficit of Lemma D. A row may be refined again ([[W, k], k2]) by passing it to
`comm3.py` as input. One call of `comm3.py` refines each of its input rows once and does not read its own output
(its main loop runs over the input rows only), so one call adds one level of depth. No claim is made that a
refinement exists; the ghost and the replay decide each instance.

### 3.3 Closure (`PROOFS-BF.md` Lemma V and Lemma V'; proved)

V W V^-1 with V a word of SAFE letters (no off piece in the cap, nothing straddling) that empties Bad(W) of lattice
pieces. With a non-empty free set the free pieces move during W, so the admissibility of V^-1 after W is not given by
Lemma V; the exact replay of V W V^-1 decides it (`tclose2.py`). V comes from the peeling certificate (Theorem B of
the E-chain package) or from the two-pass greedy search.

### 3.4 Lemma D (per-type deficit; proved)

Let W be a word at Z with Bad set Bad(W). For a type t let occ_t be the number of chambers of type t in Bad(W) that
hold a lattice piece at Z, and hole_t the number of holes of type t at Z outside Bad(W). If a word V of K+ letters
(Lemma V or Lemma V') leaves no lattice piece in Bad(W), then occ_t <= hole_t for every type t.

*Proof.* K+ letters keep lattice pieces on the lattice and in chambers of their type (Section 1), and move them
bijectively. Let n_t be the number of lattice pieces of type t and N_t the number of chambers of type t. At V(Z) these
n_t pieces lie in distinct chambers of type t outside Bad(W), so n_t <= N_t - |Bad(W)_t|. At Z, n_t = N_t - (number of
holes of type t), and the holes of type t inside Bad(W) number |Bad(W)_t| - occ_t. Substituting gives occ_t <= hole_t.
QED

*Equivalent form.* The holes of type t inside Bad(W) number |Bad(W)_t| - occ_t, and all holes of type t number
off_t(Z) (Fact 1'). So occ_t <= hole_t reads

    |Bad(W)_t| <= off_t(Z)   for every type t.

**Corollary D' (the last unit; proved).** If off(Z) is a single unit U, a step V W V^-1 with V a word of K+ letters
that restores U needs |Bad(W)_t| <= |U_t| for every type t; in particular |Bad(W)| <= |U|.

*Proof.* Lemma D in the equivalent form with off_t(Z) = |U_t|. QED

So the last unit of a run can only be closed by a word whose Bad set is no larger than the unit, type by type: a
"swap" word, which exchanges the unit with lattice pieces of the same types. Some words of the runs are swap words:
the D-type words of steps 7 and 8 and the refinement [Q, k] of step 4 (Section 6.1, |Bad(W)| = |U|); for other units
the D-type and C-type words found have Bad sets of up to about 800 chambers.

**Remark L (the landing and Bad; measured, not a theorem).** In every try of the runs, the chambers L where W lands the
unit lie in Bad(W) (`pre.json`: T_landing_in_badW = |T| in every case). This is a property of the words used, not of
all words: for W = X, k with X a jumble letter at grip e that lands U and k a K+ letter at a grip f that carries part
of the landing out of cap e, the lattice ghost of such a landing chamber is not moved by X and is carried by k onto
the lattice, so that chamber is not in Bad(W). Nothing below depends on Remark L; it only explains why the bound
of Corollary D'' is tight for the words of the runs.

**Corollary D'' (the last carrier of a type; proved).** Let t be a type and Z a state at which every off piece of
type t belongs to the unit U or to free pieces that the step lands on the lattice; let f_t be the number of those
free pieces. A step V W V^-1 with V a word of K+ letters that restores U needs |Bad(W)_t| <= |U_t| + f_t. If no
other piece of type t is off (f_t = 0), this is |Bad(W)_t| <= |U_t|.

*Proof.* Lemma D in its equivalent form, with off_t(Z) = |U_t| + f_t. QED

For the words of the runs the landing lies in Bad(W) (Remark L), so the bound says Bad(W)_t = L_t: W may push no
lattice chamber of type t off the lattice except the ones where U lands. Bad(W) depends on the word alone (the lattice
ghosts do not depend on the state), so the condition can be checked once per word and holds at every later state.
Conjugation does not help: for V a word of K+ letters, Bad(V W V^-1) = V^-1(Bad(W)) as a set of chambers, and K+
letters map chambers of a type onto chambers of the same type, so the counts per type are the same. The word W itself
must meet the condition. Section 6.1 shows that at stop62 no word in the pools of the three carriers of two rare types
meets it with f_t = 0: whichever carrier is restored last cannot be restored alone by a word of its pool. The
certificate does not exclude a step that lands two carriers at once (f_t > 0). Section 6.1 gives three such steps at
233 (carrier-move closure, Lemma CM); the carrier left after them has f_t = 0 again.

**Lemma V* (conjugated vacancy letters; proved).** Let a be a K+ letter admissible at Z and s a SAFE-ALL letter of the
state a(Z). Then a s a^-1 is admissible at Z, keeps every lattice piece on the lattice and returns every off piece
to its exact pose.

*Proof.* a is admissible at Z. Cap a is unstraddled at a(Z): its inside regions are rotated inside it and the rest
does not move. s is admissible at a(Z), moves only lattice pieces (its cap holds no off piece) and keeps them on the
lattice. Lattice regions never straddle, so cap a is still unstraddled after s and a^-1 is admissible; it returns
the off pieces inside cap a to their poses at Z, which s did not change. K+ letters keep lattice pieces on the
lattice. QED

So the vacancy group can be enlarged by such conjugates: a hole that sits in a cap held by an off block (and so in a
singleton SAFE-ALL orbit) becomes movable when a K+ letter at one of the block's movers carries it into a SAFE cap.
The closure is still decided by the exact replay (Section 3.3: with moved free pieces, the admissibility of V^-1 after
W is not given in advance). The driver does not use these letters yet. A glued hole (Proposition GH, Section 6.1)
is not reached this way: a s a^-1 returns every off block to its pose, so it fixes every hole glued to an off block.
At stop62@233 all 9 holes of the rare types are glued. A word of K+ letters that moves a whole off block, and is not
undone, moves the holes glued to it (Lemma CM, Section 6.1).

**Vacancy conditions N1 and N2 (E-chain package, `buffer.condition`; necessary).** V is built from the SAFE-ALL
letters of Z (K+ letters at grips whose cap holds no off piece and no straddling region; the set does not change while
V runs, since V moves no off piece). Let the SAFE-ALL orbits be the orbits of the group they generate on chambers.
Then V can empty Bad(W) only if every occupied chamber of Bad(W) lies in an orbit of size at least 2 (N1) and every
orbit o has |Bad(W)_o| <= holes_o (N2). The SAFE-ALL orbits refine the types, so N2 implies the condition of Lemma D.
The driver computes N1 and N2 for every word (`score4.py`) and tries only words that meet both (*vac_ok*).

The *deficit* of W at Z is the sum over t of max(0, occ_t - hole_t). A word with a positive deficit cannot be closed
by any lattice-preserving V. The driver computes it for every word and tries only vac_ok words (N1 and N2, which imply
deficit 0). Since the holes of each type equal the off pieces of that type (Fact 1'), the endgame is hard in two
ways: the holes of a rare type become few (Lemma D, Corollary D''), and the holes that exist may sit in caps held by
off blocks, where no SAFE-ALL letter reaches them (N2 fails although the type count is fine; Section 6.1).

### 3.5 Refinements: kept chambers, the refinement identity and inherited deaths (proved)

*Contents.* A content is a region that a piece occupies, with its pose (Section 1). `fastghost.py` stores it as a
frame and a chamber of that frame. A letter (c, h) acts on contents: a content inside cap c is rotated by h, one
outside cap c is fixed, and one that straddles cap c dies. A content inside cap c is still inside after the rotation,
because h keeps the half-space of the cut H_c. So the inverse letter undoes the letter on every content that it does
not kill. For a word W the surviving contents are mapped injectively, and W^-1 is the exact inverse on the image.

Notation:
- W(y) is the image of a content y; it is undefined when y dies.
- l(x) is the lattice content of the chamber x: its region is the chamber and its pose lies in K+.
- Lambda is the set of all lattice contents.
- A chamber x is *kept* by W when W(l(x)) is a lattice content. Bad(W) is the set of chambers not kept.
- D(W), a subset of Bad(W), is the set of chambers whose lattice content dies under W.
- A K+ letter maps lattice contents to lattice contents and off contents to off contents (Fact 2).
- A content keeps its type along any word, because a K+ rotation maps a chamber to a chamber of the same type.
- For a K+ letter k at the grip f, k~ is the permutation of chambers that is k on cap f and the identity elsewhere.
  Lattice chambers never straddle a cut.

**Lemma K (kept chambers; proved).**

(a) Sending x to the chamber of W(l(x)) is a type-preserving bijection from the chambers kept by W onto the chambers
kept by W^-1. Hence |Bad(W)_t| = |Bad(W^-1)_t| for every type t.

(b) Suppose W lands a set U of off pieces: every piece u of U survives and ends on the lattice, at a chamber l_u, and
L is the set of these chambers. Then L is contained in Bad(W^-1), and |Bad(W)_t| >= |U_t| for every type t.

*Proof.*

(a) If W(l(x)) = l(y), then W^-1(l(y)) = l(x), so y is kept by W^-1. The map is injective because W is, and the
same map for W^-1 is its inverse. The map keeps types. Each type has a fixed number of chambers, so the complements,
Bad(W)_t and Bad(W^-1)_t, have the same size.

(b) W^-1(l(l_u)) = u, which is off the lattice, so l_u is in Bad(W^-1). Distinct pieces land at distinct chambers,
by injectivity. l_u has the type of u. Part (a) then gives the bound. QED

Lemma K(b) is the weakest form of the bound in Corollary D''. A word that restores the last carrier of a type t must
meet it with equality in type t.

**Lemma O (proved).** For a set U of off pieces, let H(U) be the set of chambers whose interior meets the region of a
piece of U. If W lands U, then H(U) is contained in Bad(W).

*Proof.*
1. Let x be in H(U), and let u in U have a region that meets the interior of x.
2. If the lattice content of x dies, x is in Bad(W).
3. Otherwise neither content straddles at any letter of W. Two non-straddling contents whose regions share an interior
   point lie on the same side of every cut. So, letter by letter, they are moved by the same rotations.
4. At the end the ghost's pose is g k and u's pose is g p. Here g is the product of the rotations, k in K+ is the
   ghost's pose at Z, and p, not in K+, is u's pose at Z.
5. u ends on the lattice, so g p is in K+.
6. If the ghost also ended on the lattice, g k and hence g would lie in K+. Then p = g^-1 (g p) would lie in K+, which
   it does not.
7. So the ghost ends off the lattice, and x is in Bad(W). QED

*Lemma O gives nothing at stop62@245 (measured).* `hloc.py` samples 24 interior points of every one of the 245 off
pieces, by exact arithmetic (`ep/lemo/hloc245c.json`). Of the 5,880 samples, 5,878 lie outside the 600-cell body
(exact facet test), 2 lie on a cut, and none lies in a chamber. The rotated regions there stick out of the body; the
off pieces do not tile it. As far as the samples show, H(U) is empty for every unit at 245.

**Proposition W (H(U) plus one chamber per piece; proved).** Let Q be a word that lands U at the chambers L, and let k
be a non-identity K+ letter at a grip f. If [Q, k] = Q, k, Q^-1, k^-1 also lands U, then Bad([Q, k]) contains H(U)
and a further set E, disjoint from H(U), with |E_t| = |U_t| for every type t. Hence |Bad([Q, k])_t| >= |H(U)_t| +
|U_t|.

*Proof.*

1. *L lies in Bad(Q^-1)* (Lemma K(b)).
2. *When [Q, k] keeps a chamber that Q keeps.* For x kept by Q, the ghost goes by Q to Q(x) and by k to k~(Q(x)).
   Q^-1 keeps it exactly when k~(Q(x)) is kept by Q^-1. k^-1 does not change lattice status (Fact 2). So x is in
   Bad([Q, k]) if and only if k~(Q(x)) is in Bad(Q^-1).
3. *What [Q, k] needs.* After Q and k, u sits exactly in k~(l_u) with its pose in K+. It ends on the lattice under
   Q^-1 only if k~(l_u) is kept by Q^-1. By step 1, k~(l_u) is then not in L. So l_u lies in cap f, and k(L) does not
   meet L. Let y_u be the chamber kept by Q with Q(y_u) = k(l_u) (Lemma K(a)).
4. *One extra Bad chamber per piece of U.* Fix u. Cap f is invariant under k, so k(l_u) and k^2(l_u) lie in cap f.
   (a) Suppose k^2(l_u) is kept by Q^-1, and let z_u be the chamber kept by Q with Q(z_u) = k^2(l_u). k^2(l_u) is not
   l_u (step 3), so k does not have order 2. The non-identity elements of A4 have order 2 or 3, so k^3 = 1 and
   k(Q(z_u)) = l_u, which is in Bad(Q^-1). So z_u is in Bad([Q, k]) by step 2. Put e_u = z_u.
   (b) Otherwise k(Q(y_u)) = k^2(l_u) is in Bad(Q^-1), so y_u is in Bad([Q, k]). Put e_u = y_u.
5. *Counting.* Each e_u is kept by Q, and Lemma O applied to Q puts H(U) in Bad(Q), so e_u is not in H(U). The map of
   Lemma K(a) is injective. Q(e_u) is k(l_u) or k^2(l_u), and k(l_u) = k^2(l_v) would give l_u = k(l_v), in both L and
   k(L), which step 3 excludes. So the e_u are |U| distinct chambers. e_u has the type of l_u, which is the type of u.
   With Lemma O applied to [Q, k], Bad([Q, k]) contains the disjoint sets H(U) and E = {e_u}. QED

Proposition W applies to any Q that lands U, so it covers refinements of every depth. When H(U) is empty, as at
stop62@245, it says no more than Lemma K(b).

**Corollary W (proved).** Let U be a unit of Z that carries every off piece of type t, with H(U)_t not empty. Then no
step V [Q, k] V^-1 restores U, for V a word of K+ letters, any free set, and Q any word that lands U.

*Proof.* Lemma D in its equivalent form needs |Bad([Q, k])_t| <= off_t(Z) = |U_t|. Proposition W gives at least
|H(U)_t| + |U_t| > |U_t|. QED

Corollary W does not apply at stop62@245, where the sampled H(U) is empty. There the next two statements carry the
argument (Section 6.1: "Depth two by the refinement identity" and the depth-two run behind "Depth two at 233").

**Proposition J (the refinement identity; proved).** Let Q be any word and k a K+ letter at a grip f. Put B' =
Bad(Q^-1), and let S(Q) be the set of off-lattice contents Q(l(x)) for the chambers x in Bad(Q) \ D(Q). Then for every
type t

    |Bad([Q, k])_t| = |D(Q)_t| + #{w in B'_t : k~(w) not in B'} + #{y in S(Q)_t : k(y) dies or k(y) not in S(Q)}.

*Proof.*
1. Run [Q, k] as Q, then k, then Q^-1, then k^-1. Let x be a chamber whose content y = Q(l(x)) survives Q.
2. k^-1 does not change lattice status, and lattice contents never straddle. So x is kept if and only if k(y) survives
   and Q^-1(k(y)) is a lattice content, that is, if and only if k(y) lies in Q(Lambda).
3. Q(Lambda) consists of S(Q) and of the lattice contents l(w) with w kept by Q^-1 (Lemma K(a)). k maps lattice
   contents to lattice contents and off contents to off contents, and on lattice contents it acts as k~.
4. So, in type t, the chambers kept by [Q, k] are of two kinds:
   - the x kept by Q with k~(Q(x)) kept by Q^-1. By Lemma K(a) there are #{w in K'_t : k~(w) in K'} of them, where K'
     is the complement of B'.
   - the x with y in S(Q) and k(y) in S(Q).
5. Hence |Bad([Q, k])_t| = N_t - #{w in K'_t : k~(w) in K'} - #{y in S(Q)_t : k(y) in S(Q)}, where N_t is the number
   of chambers of type t.
6. Three counts:
   - N_t = |K'_t| + |B'_t|.
   - |K'_t| - #{w in K'_t : k~(w) in K'} = #{w in K'_t : k~(w) in B'}. This equals #{w in B'_t : k~(w) not in B'},
     because k~ is a type-preserving permutation of the chambers.
   - |B'_t| = |Bad(Q)_t| = |D(Q)_t| + |S(Q)_t| (Lemma K(a); Q is injective on contents).
7. Substituting these into step 5 gives the identity. QED

The identity is checked exactly on 132 refinements of four family words of 86@1040 at stop62@245, landing or not
(`refid.py`).

**Corollary J (proved).** If Q lands U at L and [Q, k] lands U, the middle term is at least |U_t|. Hence

    |Bad([Q, k])_t| >= |D(Q)_t| + |U_t|.

*Proof.* L lies in B' (Lemma K(b)). k~(L) does not meet B' (Proposition W, step 3). So every chamber of L_t is counted
in the middle term. QED

So the excess of a landing refinement over the bound of Lemma K(b), in type t, is the sum of three terms: the dead
chambers of Q, the chambers of B' that k~ moves out of B' beyond L, and the off contents of Q that k does not map back
into S(Q).

**Corollary J' (deciding refinements from Q; proved).** Let Q land U at L. Then [Q, k] lands U if and only if k~(L)
does not meet Bad(Q^-1) (all types). With Proposition J, the data D(Q), Bad(Q^-1) and S(Q) of Q alone decide, for
every K+ letter k, whether [Q, k] lands U and its Bad count per type.

*Proof.* For u in U, [Q, k] sends u by Q to l(l_u), by k to l(k~(l_u)), then by Q^-1 to a lattice content exactly
when k~(l_u) is kept by Q^-1, and k^-1 keeps lattice status. QED

**Remark C (refinements as compositions; proved).** As partial maps of contents, [Q, k] = k^-1 o (Q^-1 o k o Q).
By induction, every iterated refinement W_n of W_0 (n >= 1) is a composition of K+ letters k at the grips used and of
the maps W_0^-1 o k o W_0 and their inverses. Any statement proved for all such compositions holds for every depth.
Section 5.3 uses this for a proof attempt.

**Lemma I (deaths are inherited; proved).** Let W_0 land U, and let W_j = [W_(j-1), k_j] for j = 1, ..., n, with k_j
K+ letters and every W_j landing U. Then D(W_j) contains D(W_0) for every j, and, for n >= 1,

    |Bad(W_n)_t| >= |D(W_0)_t| + |U_t|   for every type t.

*Proof.* W_j begins with W_(j-1), so a lattice content that dies in W_(j-1) also dies in W_j. By induction D(W_j)
contains D(W_0). Corollary J with Q = W_(n-1) gives |Bad(W_n)_t| >= |D(W_(n-1))_t| + |U_t| >= |D(W_0)_t| + |U_t|.
QED

**Corollary I (proved).** Let U carry every off piece of type t, and let W land U with a dead lattice content of type t
(D(W)_t not empty). Then no step V W_n V^-1 restores U, where V is a word of K+ letters and W_n is an iterated
refinement of W of depth n >= 1 that lands U at every depth.

*Proof.* Corollary D'' with f_t = 0 needs |Bad(W_n)_t| <= |U_t|. Lemma I gives more. QED

The driver refines only words that land the unit (`comm3.py`, both with and without `--rescore`, keeps only input rows
with T on K). So every refinement in its pool is an iterated refinement whose intermediate words all land the unit, as
Lemma I requires.

**Remark S (the copy; measured).** At stop62@245 `refdet.py` examined one D-type word and one C-type word of 86@1040
and their first refinements. In both:
- Bad(Q) in types 7 and 9 is L together with a set X of |U_t| chambers of each type. Q carries the lattice contents of
  X to a set Sigma of off contents in the unit's own frame.
- Bad(Q^-1) is L together with another set X', which Q^-1 carries to the same Sigma.
- So Q makes a copy Sigma of the carrier's rare pieces.
- In each refinement [Q, k] the copy either dies, and then Corollary I closes everything below, or k carries it where
  Q^-1 does not act, and it comes back to Sigma.
- In both cases the excess is 3 = |U| in types 7 and 9.

`refwho.py` ran over six family words of 86@1040 and their 240 landing refinements:
- The landing chambers of the refinement are the landing chambers of Q in 239 of the 240 cases.
- In none of the 240 do the rare landing chambers all lie in Bad of the refinement. So Remark L, measured on the tries
  of the runs, does not hold for refinements.

## 4. The general driver and its correctness

`gen/cbrun3.py` runs at each state:
1. **Units.** `mkblocks2.py` lists the blocks with their inside grips, movers, straddled grips and frames, and writes
   one target file per unit (and its free file).
2. **Pool.** Each unit keeps a pool of words: the D-type and C-type words of earlier searches (seeds), the single
   letters at every mover of its block (60 per mover; with occupied Bad = 0 such a letter is a step by itself, as at
   the fresh cap of Lemma BL5), and the words found by the steps below.
3. **Score.** `score4.py` scores every word of a unit's pool at the current state by the exact frame ghost of the
   unit, of its free set, of every other off piece and of every chamber of the word's caps: the unit on the lattice,
   the pieces the word cannot carry (the unit, its free set, other blocks: then the word is not usable here), the
   other blocks it moves but carries (they join the free set), Bad(W), its occupied chambers, the per-type deficit
   (Lemma D) and the vacancy conditions N1 and N2 for the SAFE-ALL alphabet of the state (*vac_ok*).
4. **Search.** A unit without a usable vac_ok word gets a fresh search at this state (`dfam2.py`, D-type and C-type
   words, with `--free` for frame units); the new words are scored.
5. **Order.** Only usable vac_ok words are tried. Default order: failed before; occupied Bad = 0 (larger units
   first); then smaller units first, occupied Bad, moved pieces, Bad. With `--hard-first`: units by the smallest Bad
   of their vac_ok words, largest first (Remark O below), then occupied Bad, moved pieces, Bad. The best word of
   each unit comes before the second word of any unit.
6. **Closure.** `tclose2.py` runs PAR words at a time. It checks the ghost premises, builds V (peeling certificate,
   else the two-pass greedy search with a time limit), and replays V W V^-1 letter by letter on the simulator. The
   free set of a try is the unit's own free set plus the pieces of the blocks the word moves. The first success in
   the order is taken and the run continues from its end table.
7. **Refinement.** If no try succeeds, `comm3.py` refines the usable words that move no other block ([W, k]), at
   most `--refine-q` of them per unit, best first by deficit, occupied Bad and Bad; the new words are scored and tried
   the same way. `refine()` keeps the set of (state, unit) pairs it has refined and skips them, so there is at most
   one refinement round per state.
8. The run stops when no unit is left (or no unit meets `--only`), or when no unit is restored at a state.
   `dfam2.py` writes parked words P Q P^-1 to a separate `.park.jsonl` file; `search()` and `seed()` read only
   `.q.jsonl` files and `comm3.py` output, so parked words are never read.

**Theorem G (correctness; proved).** Every step that `cbrun3.py` writes is a clean step for its unit, with the free
set of its try. The off count falls by at least the size of the unit at every step, so a run ends after at most
off(Z) steps, with all pieces (or all focus pieces) on the lattice or at a recorded stop.

*Proof.* A step is written only when the exact replay of V W V^-1 in `tclose2.py` admitted every letter, ended with
the unit on the lattice (T_restored = |U|), changed the exact pose of no protected piece (every off piece outside
the unit and the free set), and left no lattice piece of the start state off the lattice (collateral 0). That is the
definition of a clean step. Lemma U gives the fall of the off count. QED

The search, the scores, the order and the time limits decide only which steps are found, never whether a written
step is correct. `gen/verify_u.py` replays a written chain from its start table with the simulator alone (none of
the search scripts) and checks every condition of the definition again: input and output digests, every letter
admitted, the unit off before and on after, collateral 0, every moved or restored piece inside the unit or the
recorded free set, and the off count falling.

**Remark O (order; the inequality is proved, the order is a heuristic).** Every step is clean, so the off set only
shrinks along a run: off(Z_{i+1}) is a subset of off(Z_i), and off_t is non-increasing in every type t. Lemma D in
its equivalent form, |Bad(W)_t| <= off_t(Z), therefore gets harder to meet as the run proceeds. A unit whose words
all have a large Bad set has to be restored while the off count is still large; a unit with a swap word
(|Bad(W)_t| = |U_t|, Corollary D') can be restored last. The order is only a heuristic; no run of Section 6.1
isolates its effect, and nothing there depends on it. The remark only motivates the option `--hard-first`.

**Proposition F (frozen blocks; proved).** Let B be frozen at Z and h an inside grip of B. Let St(h) be the off pieces
whose regions straddle cap h. After any sequence of clean steps with empty free sets that restores every piece of
St(h), h is a mover of every block contained in B.

*Proof.* Clean steps with empty free sets keep every remaining off piece at its exact pose, so B's regions are
unchanged and cap h still holds them; the remaining straddlers of h are a subset of St(h) at the same regions, that
is none, and lattice regions never straddle. B can only split (Proposition M(iii) of `PROOFS-BF.md`); every part is
held by cap h. QED

So a frozen block needs no construction of its own: the straddlers of one of its grips are ordinary units, and when
they are restored the block becomes an ordinary unit. In a run the driver simply finds no word for the frozen unit
until then (`dfam2.py` stops with "no unstraddled cap holds T") and restores other units first; this is Theorem CB of
`PROOFS-BF.md` with the frozen block as a sink that is unlocked. Proposition F needs steps without free sets; a
partial step of another block can create a new straddler of h, and the run then lists the units again.

**Lemma H (the coset letters keep the depth; proved).** Let Z = t(Z') with t = (c, g) a jumble twist admissible at
Z', and let the frame d of Z be the domain of g (the frame of the lattice pieces of Z' that t sent off). For every
letter mu of the 12-letter coset that sends d to the lattice (mu = k0 g^-1 with k0 in the K+ stabilizer of c),
mu(Z) = kappa0(Z') with kappa0 the K+ letter k0 at c; so mu(Z) has a history with one jumble twist fewer than Z.

*Proof.* Theorem R of `PROOFS-GF.md`: cap c is unstraddled at Z and its inside regions are the g-images of those of
Z'. mu multiplies their poses by k0 g^-1, so they become k0 times their poses at Z'; outside nothing moves. kappa0 is
admissible at Z' because t was. QED

Consequence. To undo the last jumble twist only its grip and its frame have to be found; the choice among the 12
letters changes the lattice arrangement (the F4 ties of `PROOFS-GF.md`), not the depth. The state alone does not
show which (grip, frame) pair is the last twist, and a wrong pair does not lower the depth; this is the identification
problem behind the open items of Section 5.

## 5. The Block lemma

### 5.1 Statements

**Definition (removal).** Let Z be reachable and S a set of off pieces of Z. A *removal* of S is a reachable state D in
which every piece of S is on the lattice, every off piece of Z outside S has its pose at Z, and every lattice piece of
Z is on the lattice (in any chamber). S is *removable* at Z when a removal exists.

**Lemma BL1 (proved).** S is removable at Z exactly when some admissible word leads from Z to a removal of S.

*Proof.* Every letter has an inverse letter that is admissible at its image (Theorem R of `PROOFS-GF.md` for jumble
letters; the same argument for K+ letters), so two reachable states are joined by admissible words in both
directions. QED

**Lemma BL2 (proved).** The whole off set off(Z) is removable at every reachable Z.

*Proof.* The solved state is a removal: the condition on the other off pieces is empty. QED

So the induction of Proposition B (remove a removable non-empty set, repeat) is satisfied trivially by one removal of
everything. Three forms of the Block lemma follow, from weak to strong:

- **Weak form (proved).** At every reachable Z with off pieces, every unit U has a clean step with the free set
  off(Z) \ U. *Proof.* By BL1 and BL2 some admissible word leads from Z to the solved state; it ends with U on the
  lattice and every lattice piece of Z on the lattice, and the condition on the other off pieces is empty because they
  are all free. QED. The word is an inverse history; it exists, but the state does not show it.
- **Strong form (open).** At every reachable Z with at least two blocks, some single unit has a clean step whose free
  set is only the rest of its own block (every other off piece exact).
- **Family form (false for the base family and for the family of depth at most one; open for deeper refinements).**
  The driver of Section 4 finds a clean step (with free conflict blocks) at every reachable state with off pieces,
  from the state alone.

Lemma U accepts any free set, so a step of the weak form is a valid step of the method. Existence is therefore not
the open point; the open point is finding a step from the state alone. The strong form is neither proved nor refuted.
The family form depends on the family:
- for the base family (single letters, D-type and C-type words, closed by any word of K+ letters) it is false, with
  the reachable counterexample stop62@245 (Corollary S, Section 6.1);
- for roots and refinements [W, k] of depth one, which is everything the driver holds when it starts from a state
  alone, it is false, with the reachable counterexample stop62@233 (Theorem Z, Corollary Z): every such word of the
  three carriers left there meets the hole-free type 29 (23,703 landing roots; the 677,372 landing refinements of the
  roots without a dead type-29 content each have at least 8 type-29 Bad chambers, and the refinements of the 3,965
  roots with a dead type-29 content keep that dead chamber, Lemma I);
- with refinements of depth two the counting bound of Lemma D can be met at 233 (42 words found), but none of these
  can be vacated by SAFE-ALL letters (N2 fails in the orbits of types 7 and 9), nor by any word of K+ letters that
  returns every carrier to its pose (Proposition GH). A prefix of K+ letters that moves a second carrier with its glued
  holes does close some of them, outside the driver (Lemma CM): three steps restore two carriers each, 233 to 86 or
  61 (certificate). The last carrier still needs a swap word: no root and no refinement of depth one is one
  (Certificate 5), below every word with a dead rare ghost none is (Corollary I), and the driver's parked words fail
  as well (Certificate 6). In every deeper word examined the excess is at least 3 (Section 6.1, measured). At the
  three end states the driver stops at once (measured).

### 5.2 Transfer along a history (proved)

**Lemma BL3 (K+ letters).** Let Z = kappa(Z') with kappa a K+ letter admissible at Z'. If D' is a removal of S at Z',
then kappa is admissible at D' and kappa(D') is a removal of S at Z.

*Proof.* The off regions of D' are a subset of those of Z', so none straddles the cap of kappa; lattice regions never
straddle. kappa keeps lattice pieces on the lattice and off pieces off (Fact 2), and moves the pieces of off(Z') \ S
exactly as it does in Z'. So in kappa(D') S is on the lattice, every other off piece has its pose at Z, and every
lattice piece of Z (the lattice pieces of Z') is on the lattice. kappa(D') is reachable. QED

**Lemma BL4 (jumble letters, case of an unchanged cap).** Let Z = t(Z') with t a jumble twist at grip c admissible at
Z'. If D' is a removal of S at Z' whose cap c holds exactly the pieces that cap c holds in Z', at the same poses, then
t is admissible at D' and t(D') is a removal of S at Z.

*Proof.* Inside cap c, D' and Z' agree, so t is admissible at D' (no region straddles; S lies outside cap c, being on
the lattice in D' and off in Z') and acts identically there. Outside cap c nothing moves. QED

**Lemma BL5 (fresh cap).** If Z = t(Z') with t a jumble twist at grip c and cap c at Z' holds only lattice pieces, then
the set N of pieces that t sent off is removable at Z (by t^-1, which returns Z').

Lemmas BL3 to BL5 carry removability forward along a history, except at a jumble twist whose cap already held off
pieces while every available removal changes that cap's content. At such a twist the induction needs a new removable
set built from the state. BL5 covers the depth-one states (one jumble twist from solved; finitely many up to K+) only
for the whole off set N: t^-1 removes all of N at once. It does not give a clean step for a single unit of such a
state: N is cut into several blocks by the unstraddled grips that overlap cap c (Section 6.4), and a clean step for
one of them must leave the others at their exact poses. For the next depths no general argument is known. Lemma H shows that history reconstruction needs only the grip and
frame of the last twist, not its letter, but the state does not reveal them.

### 5.3 What blocks the family (certificates and measurements)

In the runs of Section 6 the steps that fail do so at the closure, not at the word: the word puts the unit on the
lattice with the protected pieces exact, but its Bad set cannot be emptied by a word of SAFE-ALL letters. There are
five causes, from strongest to weakest.

- **Hole-free types (Theorem Z; certificate).** At stop62@233, 15 of the 36 types have no hole, and every root and
  every landing refinement of depth one of the three carriers has a Bad chamber of type 29. A clean step only shrinks
  the off set, so these types stay hole-free at every later state; every closable word must miss them.
- **The base family is too coarse at the end (Theorem S, Corollary S; certificate).** At stop62@245 every word of the
  whole base family of every unit (single letters, D-type and C-type words, every parameter) has |Bad(W)| >= 333 > 245
  = off(Z), so no word of K+ letters can vacate it (Lemma D). The driver restricted to the base family has no step at
  this reachable state: the family form is false for the base family. Since Bad(W) depends on W alone, these words
  cannot be closed at any state with fewer than 333 off pieces either.
- **The last carrier of a rare type (Corollary D''; certificate).** At stop62 the pieces of types 7 and 9 (labels of
  `regions.orbit_of`) are carried by three blocks only, each with two pieces of type 7 and one of type 9. One of the
  three is restored last, and then |Bad(W)_7| <= 2 and |Bad(W)_9| <= 1 (if it is restored alone). Every usable word
  in the pools of the three carriers, and every word of their whole base family at 245, exceeds these bounds by at
  least 3 (Certificate 2 of Section 6.1): in these words the type-7 and type-9 part of Bad(W) holds at least twice
  the landing (measured). This is a property of the carriers' pools, not of the word shapes: words of other blocks
  are swap words (the D-type words of steps 7 and 8 and the refinement of step 4 in Section 6.1 have
  |Bad(W)| = |U|).
- **Holes held by off blocks (N1 and N2; certificate).** At 689 all nine holes of types 7 and 9 lie in chambers that
  no SAFE-ALL letter moves: every grip whose cap holds such a hole holds an off piece or is straddled, and for the
  holes near a carrier the off piece is the carrier itself (`release.py`). So even the first two carriers, whose words
  meet the counting bound of Lemma D when the other carriers are still off, fail N2 by 3. At 233 the same blocks the
  42 refinements of depth two that meet Lemma D: each has 2 Bad chambers in the SAFE-ALL orbit of type 7 and 1 in that
  of type 9, which hold no hole (Section 6.1). The holes are not merely out of reach of SAFE-ALL letters: at 233 each
  is glued to its carrier (Proposition GH, Section 6.1), so no word of K+ letters that returns the carriers to their
  poses moves it. In particular Lemma V*, which was meant to reach such holes, does not. A closure can use them by
  moving a carrier together with its glued holes; the carrier-move steps of Section 6.1 do this (Lemma CM,
  certificate). For the last carrier there is no other carrier to move. At the three end states its holes are still
  glued to it (certificate), so for V with V(U) = U its Bad set in types 7 and 9 must be exactly its own glued holes
  (Corollary GH).
- **Per-type deficit (Lemma D; certificate).** At the stop62 state with 506 off pieces, the D-type words of three
  blocks (29, 111, 111 pieces) have deficits of 45 in the rare types 31 to 34: their Bad set holds up to 32 occupied
  chambers of a type that has 16 to 18 holes outside it (`certs/ptype-stop62-506.jsonl`). These blocks were restored
  later in the run by other words.

**Why the refinements keep failing (measured; the proof attempt stops at the grips).** Section 3.5 turns the
refinement question into bookkeeping. By Proposition J, [Q, k] is a swap word in type t exactly when D(Q)_t is empty,
k~ moves no chamber of B'_t other than the landing out of B', and k maps the off contents S(Q)_t into S(Q). At
stop62@245 most open words of 86@1040 share one anatomy (`ep/lemo/deep/anat.py` and `geo2.py`, on open words of depth
one and two; their outputs are not in the package, so these counts have no record file):
- S(Q) in types 7 and 9 is the carrier's own rare pieces U (exactly their contents) and a copy Sigma of three more
  contents, and B'(Q) is the landing L and three chambers X' (the X' of Remark S): in 137 of 150 words of depth one
  and 141 of 150 of depth two;
- Sigma lies inside every cap that holds L in all 180 of the 200 words checked that have this anatomy; U and X' lie
  outside every such cap in 173 of them.
So in these words a refinement letter k fixes U and X' and moves Sigma, and it can only give a swap word if it
permutes Sigma. In no refinement examined does it: the smallest excess is 3, the size of Sigma (measured). The anatomy
is measured on these subsets only; it is not shown for every word.

A proof for every depth needs this anatomy to persist. The closed-set search of Section 6.1 shows that it does in
every map it reaches: the chambers that no lattice content enters are L and X', and X' stays in Bad. A natural route to
a proof: every iterated refinement of a root W_0 is, as a partial map, a composition of the letters k at the grips
used and of W_0^-1 k W_0 (by induction: [W, k] = k^-1 (W^-1 k W)). For a single piece u of type 9 and the copy
chamber x of type 9, it is then enough that no composition of these generators sends the pair (u, l(x)) to two
lattice contents, nor a pair (lam, l(x)) with lam on the lattice to (u, a lattice content): then x lies in Bad(W_n)
and in Bad(W_n^-1) for every landing W_n, and |Bad(W_n)_9| >= 2 > 1 (Lemma K). This is a finite computation over pairs
(`ep/lemo/deep/pairorb.py`) if the set of grips is fixed. It is not: the refinement letters sit at the grips that hold
the current landing, and over all compositions the landing of the rare pieces reaches caps far from the start (242
grips for the first root of 86@1040, against 4 that hold its landing). Bounding the grips needs the landing of the whole
unit along the actual refinements, not along all compositions. That is where the attempt stops.

**Type-swap words exist (proved).** At a state Z whose off set is a single unit U, the inverse of any history of Z is a
clean step for U with an empty free set and V empty: it ends at the solved state, so every lattice piece of Z ends on
the lattice. Its Bad set therefore holds no occupied chamber, that is, it lies in the holes, type by type. So the
bound of Corollary D'' can be met; what is missing is a way to build such a word from the state alone. A single
coset letter (Lemma H) meets it only when U and the free pieces fill a whole cap (Proposition N(b) of `PROOFS-GF.md`
rules this out below 1,549 off pieces); the commutator words of the pools and of the base family at 245 do not meet
it.

### 5.4 Status

- Proved: Lemmas U, D, H, V*, BL1 to BL5, K, O, I, CM; Corollaries D', D'', W, J, I; Propositions F, W, J; Theorem G;
  Theorem D1 (depth one, Section 6.4); the existence of type-swap words above; Proposition GH and Corollary GH (glued
  holes, Section 6.1).
- From the driver's code (read, not proved): Corollary D1 (the driver's first try at depth one is a coset letter) and
  the facts about the driver used in Corollary Z and Certificate 4 (one refinement round per state, one level of depth
  per round, parked words never read).
- Certificate: every written step of the runs in Section 6 and the steps outside the driver (step 9 and the three
  carrier-move steps), all replayed independently: stop62 from 689 to 245 (driver), 245 to 233 (step 9) and 233 to 86
  or 61 (carrier move); Theorem S and Corollary S (no word of the base family can be closed at stop62@245 or @233: the
  family form is false for the base family); Theorem Z and Corollary Z (no root and no refinement of depth one of a
  carrier can be closed at stop62@233, so the driver started there from the state alone has no step whatever its
  parameters: the family form is false for depth at most one); N2 fails for the 42 refinements of depth two found
  that meet Lemma D at 233; the 9 holes of types 7 and 9 at 233 are glued to the carriers (3 each), and at each of the
  three end states the last carrier's 3 holes are still glued to it; the stop62 obstruction for the word pools and the
  base family of the three carriers (Corollary D'') and for the SAFE-ALL alphabet (N1, N2); after 245 the driver uses
  only refinements and restores the carriers one at a time (Certificate 4); no swap word among the roots and the
  refinements of depth one (Certificate 5); parked words (Certificate 6).
- Measured: the seeded driver also stops at 233; the copy Sigma and the anatomy above (no record file); the
  identification of the glued holes of 86@1040 with the chambers X' of that anatomy (no record file); the partial
  depth-two runs of Section 6.1 ("Depth two by the refinement identity" and "Depth two at 233") and the closed-set
  search beyond (excess 3 in every map reached); the reachability search for the carrier moves and the batch tally
  (Section 6.1); the driver started at the three end states stops at once; Lemma O gives nothing at 245 (no sampled
  point of an off piece lies in a chamber).
- Open: the strong form; the family form for iterated refinements of depth two and more, with closure by any word of
  K+ letters and with prefixes that move other off blocks. At stop62 the open case is the last carrier: at each of the
  three end states it needs a word whose Bad set misses the hole-free types and whose Bad set in types 7 and 9 is
  exactly its own 3 glued holes (Corollary GH, for V with V(U) = U), and no other carrier is left to move. If no word
  of the driver's family, of any depth, restores the last carrier at one of these reachable states, that state refutes
  the family form for every depth. The open points beyond stop62 are unchanged: (a) a word class,
  built from the state, whose Bad set avoids the hole-free types and meets the per-type bound for the last carrier of
  a type; (b) a vacancy step that carries an off block together with its glued holes when no partner block is left
  (Lemma V* does not reach glued holes; Lemma CM needs a second off block).

## 6. Runs and certificates

Every chain below was written by `gen/cbrun3.py` or from `tclose2.py` results (Section 6.2, chain `s62z`, and the
carrier-move chains `s62m`, `s62m2` and `s62m3`), collected by `gen/mkchain_u.py` (`gen/mkchain1.py` for `s62z`; for the
carrier-move chains the single step word is the carrier-move path followed by the `tclose2.py` word, see Section 8)
and replayed from its start table by `gen/verify_u.py` alone (Section 4). Each replay checks the input and output
digests, admits every twist on the exact simulator, and checks that the unit is restored, the collateral is 0, every
moved or restored piece lies in the unit or the recorded free set, and the off count falls. All replays pass
(`chains/*/replay.jsonl`). All nine chains of v3.1 were run again from a fresh unzip of a v3.1 build
(`sha256sum -c` clean), with the research tree at `27a001c`: all nine all_ok, with end digests `0c945cbf...` (`s62`,
245 off), `46496bb1...` (`s62z`, 233 off), `090b175c...` (`s62m`, 86 off), `9963d65f...` (`s62m2`, 86 off),
`218f823f...` (`s62m3`, 61 off), `a5a5c8f6...` (`mf`), `e41cd3db...` (`frozen`), `1485262d...` (`frozen2`) and
`7d66e800...` (`d1`, every piece on the lattice). That build differs from this package only in the wording of
`PROOFS-U.md` and `REPORT.zh-CN.md` and in the later batch snapshot `certs/stop62/cm233/res.txt`: every chain,
script and other certificate file has the same SHA256.

| Chain | Start | Steps | Off pieces | Twists per step | Replay |
|---|---|---:|---|---|---|
| `s62` | stop62 @689 | 8 | 689 -> 245 | 580 to 1,432 | all_ok, collateral 0 at every step |
| `s62z` | end of `s62` @245 | 1 | 245 -> 233 | 328 | all_ok, collateral 0 |
| `s62m` | end of `s62z` @233 | 1 | 233 -> 86 | 1,822 | all_ok, collateral 0 |
| `s62m2` | end of `s62z` @233 | 1 | 233 -> 86 | 1,955 | all_ok, collateral 0 |
| `s62m3` | end of `s62z` @233 | 1 | 233 -> 61 | 1,749 | all_ok, collateral 0 |
| `mf` | end of the r40s11 run @9,033 | 2 | 9,033 -> 8,922 -> 8,836 | 10, 10 | all_ok |
| `frozen` | x16-1076 @21,113 | 3 | 21,113 -> 15,590 | 4, 1, 5 | all_ok |
| `frozen2` | end of `frozen` @15,590 | 3 | 15,590 -> 13,222 -> 12,741 -> 12,149 | 1, 10, 4 | all_ok |
| `d1` | depth one @3,097 | 1 | 3,097 -> 0 | 1 | all_ok, every piece on the lattice |

### 6.1 stop62: 689 to 233 and on to 86 or 61, and where the driver stops

**Start.** stop62 @689 is the state after step 3 of the stop62 run of the first package (`PROOFS-BF.md` Section 5.1,
which took stop62 from 860 to 689), digest `c75352a2...`. It has 12 blocks.

**Steps (certificate: replay).**

| Step | Unit | Size | W | \|Bad(W)\| (occupied) | Length of V W V^-1 | Off after |
|---:|---|---:|---|---:|---:|---:|
| 1 | 15@18517 | 15 | [Q, k], 22 letters | 87 (21) | 1,002 | 674 |
| 2 | 78@18512 | 78 | Q, 10 letters | 111 (38) | 924 | 596 |
| 3 | 3@16245 | 3 | [Q, k], 22 letters | 78 (78) | 580 | 593 |
| 4 | 87@64213 | 87 | [Q, k], 22 letters | 87 (85) | 832 | 506 |
| 5 | 10@48298 | 10 | [Q, k], 22 letters | 111 (76) | 1,368 | 496 |
| 6 | 29@21607 | 29 | Q, 10 letters | 111 (103) | 1,396 | 467 |
| 7 | 111@64191 | 111 | Q, 10 letters | 111 (101) | 1,294 | 356 |
| 8 | 111@68424 | 111 | Q, 10 letters | 111 (111) | 1,432 | 245 |
| 9 | 12@18445 | 12 | [Q, k], 22 letters | 41 (41) | 328 | 233 |

Q is a D-type word, [Q, k] its refinement (Section 3.2); V comes from the peeling certificate (Theorem B of the chain
packages) at every step; the Bad sizes are those `tclose2.py` computed for the word tried (`pre.json`). Steps 4, 7 and
8 use swap words in the sense of Corollary D' (|Bad(W)| = |U|). Every step has an empty free set: every off piece
outside the unit keeps its exact pose, and every lattice piece stays on the lattice. Steps 1 to 5 are the run
`runs/stop62` (written by an earlier version of the driver), steps 6 and 7 the run `s62d`, step 8 the run `s62f`.
The driver stops at 245: the run `runs/s62g` ends there with "no unit restored at this state" (Certificate 1).
Step 9 is not a driver step: the driver's single refinement round at 245 (`runs/s62g`) refines only its best-scored
words and missed it. The computation behind Theorem Z found it at 245 (a refinement of depth one with no chamber of
the hole-free types and Lemma D met in every type, `ep/lemo/hole/zf18445-p*.full.jsonl`); `tclose2.py` closed it
(V of 153 twists from the peeling certificate, N1 and N2 true) and chain `s62z` replays it.

**The state at 245.** Four blocks are left: 86@1040, 61@23955, 86@64184 and 12@18445.

**The state at 233.** Three blocks are left, the carriers 86@1040, 61@23955 and 86@64184 of the rare types 7 and 9,
at their poses of 245 (digest `46496bb1...`).

**Certificate 1: Lemma D fails for every usable word of the pools at 245.** The driver ran at 245 (`runs/s62g`, seeded with
every pool of the earlier stop62 runs, plus the single letters at the movers): it scored the pools, ran a fresh
D-type and C-type search for all four blocks (no new word), ran one refinement round (7 new words) and stopped with
"no unit restored at this state". Over all scored words (`certs/stop62/deficit-245.json`):

| Block | Words | Usable | Smallest \|Bad(W)\| | Smallest occupied Bad | Smallest deficit | Usable with \|Bad(W)\| <= 245 |
|---|---:|---:|---:|---:|---:|---:|
| 86@1040 | 995 | 779 | 333 | 308 | 151 | 0 |
| 12@18445 | 1,223 | 947 | 277 | 255 | 150 | 0 |
| 61@23955 | 1,762 | 1,546 | 333 | 299 | 151 | 0 |
| 86@64184 | 1,743 | 1,527 | 333 | 279 | 151 | 0 |

Every usable word has |Bad(W)| >= 277 > 245 = off(Z). By Lemma D summed over the types (|Bad(W)| <= off(Z)), no word
of K+ letters can vacate Bad(W): not the SAFE-ALL letters, not the conjugated letters of Lemma V*, not any other K+
word. So none of these words can be closed at 245. The bound is stated for the usable words only. The usable words
are the *245 pool* used as roots below: `ep/lemo/deep/pool/pool<block>.jsonl` holds one line per usable word (779,
947, 1,546 and 1,527 lines, the Usable column).

**Theorem S (the whole base family at 245; certificate).** Let Z be stop62@245 and U one of its four units (each block
lies in one frame domain with one pose, so each block is one unit). Let F0(U, Z) be every single jumble letter and
every D-type and C-type word of U at Z: Q = A, B, X, B^-1, X^-1, A^-1, X, B, X^-1, B^-1 and [X, B] = X, B, X^-1, B^-1
with
- e and a grips whose caps hold U and that no region straddles at Z, B and A non-identity K+ letters at e and a;
- c a grip that no region straddles at Z, with U outside cap c and B(U) inside cap c;
- X a jumble letter at c with X^-1 B m in K+ for the pose m of U;
- for Q, A(U) outside caps c and e

(Lemma Q and Lemma Q' without the pole condition; every cut test exact). Then every W in F0(U, Z) has
|Bad(W)| >= 333 > 245 = off(Z). So for every unit U, every W in F0(U, Z), every word V of K+ letters and every free
set, V W V^-1 is not a clean step at Z. Since Bad(W) depends on W alone, none of these words can be closed at any
state with fewer than 333 off pieces.

*Proof.* A jumble letter X at grip c sends every lattice piece of cap c off the lattice (X g is not in K+ for g in
K+), so Bad(X) is the 3,097 chambers of cap c. For the D-type and C-type words, `gen/dfam2.py` (`--no-pole
--commutator`, no limit on the number of candidates) enumerates every tuple (e, B, c, X, a, A) that meets the
conditions (a floating-point pre-filter only drops caps that cannot hold B(U); the exact cut tests decide) and scores
every word with the exact frame ghost: every word puts U on the lattice, and Bad(W) is as tabulated below. A clean
V W V^-1 needs every chamber of Bad(W) to be a hole at V(Z): a lattice piece there is sent off by W and kept off by
V^-1 (Fact 2). The holes of V(Z) number off(V(Z)) = off(Z) = 245 (Facts 1 and 2). QED

| Unit | D-type words | C-type words | Smallest \|Bad(W)\| (D-type, C-type) | Words with \|Bad(W)\| <= 245 |
|---|---:|---:|---|---:|
| 86@1040 | 7,368 | 456 | 333, 592 | 0 |
| 12@18445 | 14,172 | 612 | 333, 592 | 0 |
| 61@23955 | 6,684 | 420 | 333, 592 | 0 |
| 86@64184 | 7,224 | 444 | 333, 592 | 0 |

(`certs/stop62/family-245/u<block>.summary.json` and `u<block>.q.jsonl.gz`: every word with its parameters, the
pieces of U it puts on the lattice, |Bad(W)| and the occupied part.)

**Corollary S (the family form fails for the base family; certificate).** The driver of Section 4 restricted to F0
(single letters, D-type and C-type words; closure by any word of K+ letters; any free set) has no step at the
reachable state stop62@245. Nor has it at stop62@233: the words of F0(U, 245) have |Bad(W)| >= 333 > 233, and the
D-type and C-type words at 233 that use grips no longer straddled after step 9 are in the extended family of
Certificate 4, item 3(c), whose smallest |Bad(W)| for the carriers is 531 (61@23955) and 695 (86@64184), with no new
word for 86@1040. So the family form of the Block lemma is false for F0. The refinements [W, k] of the full driver are
the subject of Theorem Z.

**Theorem Z at 245 does not hold for every unit (certificate: step 9).** The zero-hole argument below was first run
at 245 with the three hole-free types 29, 30 and 32 (`ep/lemo/hole/zh*.out`, `zf18445-p*`). For 12@18445 it found
refinements of depth one with no chamber of these types and Lemma D met in every type (|Bad(W)| = 41, 95 and 97); one
was closed (step 9). For the three carriers the run, stopped after 800 to 1,400 roots each, found none (smallest
excess in these types 34 to 36). The certificate is therefore stated at 233, where only the carriers are left.

**Theorem Z (at stop62@233 no root and no refinement of depth one of a carrier can be closed; certificate).** Let Z =
stop62@233 and U one of the three carriers.

1. *Zero-hole types.* 15 of the 36 types have no off piece at Z, hence no hole (Fact 1'): types 0 to 6, 26 and 29 to
   35 (`ep/lemo/hole/t0.py`, `certs/stop62/z233/t0-233.json`). Type 29 has 7,200 chambers. By Lemma D a step
   V W V^-1 at Z, with V any word of K+ letters and any free set, needs Bad(W) to contain no chamber of type 29.
2. *Roots.* Every word of U's base family F0 (Theorem S) and of its extended family (Certificate 4, item 3(c): every
   D-type and C-type word that `dfam2.py` can produce for U at any state where U is unchanged); every word of the 245
   pool (the words usable at 245, Certificate 1) that lands U; every single letter at any grip that lands U. Words of
   the earlier pools that were not usable at 245 are not roots.
3. *Refinements.* For every root W_0 that lands U, every refinement W_1 = [W_0, k] that lands U: every grip f whose
   cap holds the landing and every non-identity K+ letter k at f with k(L) disjoint from L (by Proposition W, step 3,
   these are all the landing refinements).
4. *Computation.* `ep/lemo/hole/zhfull.py --types 29 --singles` at Z ghosts every root and every such W_1 exactly over
   the type-29 chambers in the caps of its grips (Bad(W) lies in those caps). A root with a dead lattice content of
   type 29 needs no refinement ghost: by Lemma I every landing refinement of it, of any depth, keeps that content
   dead. For any root or W_1 with no type-29 chamber in its Bad set, the script also computes Bad(W) over all
   chambers and checks Lemma D in every type against the holes at Z (`--full`).

Result at Z (`certs/stop62/z233/z1-<block>.json`, `.out`; the three carriers ran in parallel, about 30 minutes each):

| carrier | distinct roots (read from files) | roots that land U | landing roots by length | type-29-free roots | roots with a dead type-29 content | W_1 scored | W_1 that land U | type-29-free W_1 | fewest type-29 Bad chambers of a W_1 | rows for the full check |
| --- | ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 86@1040 | 43,958 (8,603) | 7,982 | 1: 24, 4: 456, 10: 7,368, 22: 56, 46: 74, 94: 4 | 0 | 1,357 | 238,307 | 225,748 | 0 | 8 | 0 |
| 61@23955 | 43,846 (9,370) | 7,870 | 1: 24, 4: 456, 10: 7,368, 22: 20, 46: 2 | 0 | 1,308 | 239,040 | 227,887 | 0 | 8 | 0 |
| 86@64184 | 43,827 (9,351) | 7,851 | 1: 24, 4: 456, 10: 7,368, 22: 3 | 0 | 1,300 | 235,347 | 223,737 | 0 | 8 | 0 |
| total | | 23,703 | | 0 | 3,965 | 712,694 | 677,372 | 0 | | 0 |

"Read from files" counts the words of the base family, the extended family and the 245 pool before duplicates are
removed: 8,603 = 7,824 + 0 + 779 for 86@1040, 9,370 = 7,104 + 720 + 1,546 for 61@23955 and 9,351 = 7,668 + 156 +
1,527 for 86@64184. The rest of the distinct roots are single letters (600 grips, every letter). The extended family
of 86@1040 is empty (`dfam2.py` finds no new word for it: `ep/lemo/ext/x0.summary.json`). The refinements of the
3,965 roots with a dead type-29 content were not ghosted: by Lemma I each of them keeps that dead content, so it has
at least 1 type-29 Bad chamber. The count 677,372 and the bound of 8 type-29 Bad chambers cover only the ghosted W_1,
the refinements of the other roots. The `.full.jsonl` files are empty because no root and no W_1 reached the full
check.

So for every carrier, no root and no landing refinement of depth one has a Bad set that misses type 29, and none of
them can be closed at Z, by any word of K+ letters and with any free set.

**Independent recount (certificate, on a sample only).** `ep/ghostsim.py` (the simulator's own classifier, no ghost
library, from the all-lattice end of chain `d1`) recounted |Bad(W)| and its parts in types 29, 30 and 32 for 483
landing refinements of depth one, sampled over the roots of the four units at 245 by `ep/lemo/hole/sampw1.py` (101,
178, 100 and 104 for 1040, 18445, 23955 and 64184; the roots of the sample are the base family and the 245 pool,
without the extended family). All 483 agree with the ghost library, in the total and in each type
(`ep/lemo/hole/gs<block>.jsonl` against `sw<block>.jsonl`). Bad(W) depends on W alone, so the check holds at 233. This
check is much narrower than the one for Theorem S, which recounted all 37,380 words: the 712,694 W_1 of Theorem Z
were scored by the ghost library alone.

**Corollary Z (the family form of depth one fails at a reachable state; certificate and the driver's code, read, not
proved).** The
driver of Section 4 (`cbrun3.py`), started at the reachable state stop62@233 from the state alone (no seed), restores
no unit there: with any values of its parameters (`--refine-q`, `--refine-k`, `--max-tries`, `--par`, `--dmax-q`,
`--hard-first`), and with closure by any word of K+ letters instead of the search of `tclose2.py`.

*Proof.*
1. At a state the driver holds, for a unit U, the seeded pool (empty here), the single letters at the movers of U's
   block, the words `dfam2.py` finds at this state (D-type and C-type words, in F0 or the extended family:
   Certificate 4, item 3(c)) and the refinements of one round (Certificate 4, item 2; parked words are never read:
   Section 4, step 8).
2. The round (`refine()`, then `comm3.py --rescore`) refines only usable words, which land U, with non-identity K+
   letters at grips whose caps hold the landing. A refinement that does not land U is not usable for U. One call of
   `comm3.py` refines its input rows once and does not read its own output (Section 3.2), so the round adds one level
   of depth.
3. The main loop runs at most one refinement round per state: `refine()` skips a (state, unit) pair it has refined
   (Section 4, step 7), and if the attempt after the round fails, the run stops with "no unit restored at this
   state".
4. So every word the driver can try at Z is a root or a landing refinement of depth one in the sense of Theorem Z
   (with no seed, the usable words at Z are single letters and fresh `dfam2.py` words, all roots), and Theorem Z says
   none can be closed. QED

Items 1 to 3 are properties of the code (`gen/cbrun3.py`, `gen/comm3.py`), read from it and not proved.

So the family form of the Block lemma is false for the family of roots and refinements of depth one: stop62@233 is a
reachable state where no word of it can be closed, whatever the closure. Started with the seeds of all stop62 runs
(`runs/s62h`, default parameters), the driver stops at 233 as well ("no unit restored at this state"; measured).
Theorem Z does not cover that run. Its pool holds all 15,885 seed words, while the roots of Theorem Z taken from the
pools are only the words usable at 245. A seed that was not usable at 245 (for example one that disturbed 12@18445,
which is restored at 233) may be usable at 233; such a word and its refinements are neither roots nor W_1 of
Theorem Z. The next paragraph does not cover the run either.

**Depth two at 233 (measured; certificate for the driver's closure on the words found).** Refinements of depth two
can meet Lemma D at 233. A partial depth-two run at 245 (`ep/lemo/hole/refjz.py --types 29`, Proposition J with every
prediction ghosted) found 42 landing W_2 with no type-29 chamber and Lemma D met in every type. It used the root set of
Theorem Z (base family, extended family, 245 pool and single letters), split into 4 parts by `--part`; part 0 holds
10,990, 10,962 and 10,957 of the 43,958, 43,846 and 43,827 distinct roots (before the landing test), and the run was
stopped within the first 200 roots of part 0 for each carrier. The 42 words: 20 of 61@23955, 22 of 86@64184, none of 86@1040 in that part. Rechecked at 233 (`rechk.py`), all 42 still land
their carrier and meet Lemma D in every type (|Bad(W_2)| = 172, of which 133 to 136 are occupied;
`certs/stop62/z233/rk<block>.jsonl`). The driver's closure cannot use them: for each, `tclose2.py` at 233 stops before
the construction because N2 fails. The SAFE-ALL orbit of type 7 (7,174 chambers) holds 2 chambers of Bad(W_2) and no
hole, and the orbit of type 9 (3,591 chambers) holds 1 and no hole: the 6 holes of type 7 and the 3 of type 9 lie
outside these orbits, glued to the carriers (Proposition GH below). A SAFE-ALL letter
permutes each orbit, so N2 is necessary for a V of SAFE-ALL letters (`certs/stop62/z233/n2/`). All 42 stop the same way: N1 holds, and N2 fails in exactly these two orbits (have 0, need 2 and 1).
By Proposition GH below, no V of K+ letters that returns every carrier to its pose can do it either (Lemma V*
included). A prefix that moves a second carrier with its glued holes, and is not undone, does close some of them
(Lemma CM below). The depth-two tree was not searched in full.

**Proposition GH (glued holes; proved).** Let Z be a state, U an off block and h a hole of Z (a lattice chamber with
no piece). Suppose:
1. every grip whose cap holds h has all of U inside its cap or a piece of U straddling its cut;
2. every grip whose cap holds all of U, with no piece of U straddling its cut, holds h;
3. no grip has pieces of U on both sides of its cut without a piece of U straddling that cut.
Let V be a word of K+ letters, each admissible where it is applied, with V(U) = U (every piece of U back at its
exact pose). Then V maps the vacancy at h to h: h is a hole of V(Z).

*Proof.* Induction over the prefixes of V. After a prefix there is g in K+ with every piece of U at g times its pose
at Z and the vacancy from h at g(h). Conditions 1 to 3 hold for g(U) and g(h), because g maps grips to grips, caps
onto caps and keeps every piece inside, outside or across a cut (the conditions are K+-equivariant). Take the next
letter, at grip c with element k (k in K+ acts on everything in cap c and fixes the rest; it maps cap c onto itself,
Proposition W, step 4). If a piece of g(U) straddles cut c, the letter is not admissible. If cap c holds all of
g(U), it holds g(h) by condition 2, and both move by k: replace g by kg. Otherwise cap c holds no piece of g(U)
(condition 3), so it does not hold g(h) (condition 1), and the letter fixes both. At the end g fixes the exact pose
of a piece of U, so g = 1. QED

The proof shows more: for any word V of admissible K+ letters (V(U) = U not assumed), U ends at g(U) for some g in
K+, moved rigidly, and the vacancy from h ends at g(h). The last step uses V(U) = U only to get g = 1.

**Certificate (the rare holes are glued, `ep/lemo/glue/glued.py`).** At stop62@233 each of the 9 holes of types 7
and 9 satisfies conditions 1 to 3 for exactly one carrier, 3 holes per carrier (2 of type 7, 1 of type 9): 2732,
2743, 4736 for 86@1040; 11335, 11338, 28650 for 61@23955; 119124, 119141, 128877 for 86@64184. Each lies in 5 or 6
caps: the 4 caps that hold its carrier unstraddled, and 1 or 2 that its carrier straddles
(`certs/stop62/z233/glued-233.json`). `glued.py` tests condition 3 with the pieces of the block only, as stated.
For 86@1040 these three holes are the chambers X' of the anatomy in Section 5.3 for the words examined there
(measured; that anatomy is word-dependent and no record file of the comparison is in the package).

Consequences (proved from Proposition GH; the statements at 233 use the certificate above):
- *Lemma V* cannot reach them.* A letter a s a^-1 of Lemma V* returns every off piece to its exact pose, so it fixes
  every glued hole. The same holds for every word of K+ letters that returns all three carriers to their poses: the
  holes of types 7 and 9 never move. This settles the vacancy question left open by the N2 failures above for every
  such V, not only for SAFE-ALL letters.
- *The only freedom is to move another carrier.* Let V be a word of K+ letters with V(U) = U for the unit U restored
  (this is assumed; it holds for the driver's V, which moves no off piece). Condition 3 holds for each carrier
  (`glued.py` reports no grip with pieces of a carrier on both sides of its cut), so by the remark after the proof
  each other carrier U' and U'' is moved rigidly, by some g' and g''. The holes of types 7 and 9 of V(Z) are then U's
  own glued holes, g'(holes of U') and g''(holes of U''). For each of the 42 words of depth two, the part of Bad(W)
  in types 7 and 9 outside U's own glued holes has exactly 3 chambers (`certs/stop62/cm233/reach233.json`, measured).
  A closure of such a word therefore needs the other carriers' glued holes to be carried onto these 3 chambers. When
  V itself moves a carrier, V^-1 must still be admissible after W; the next subsection moves the carrier by a prefix
  that is not undone (Lemma CM), and W lands the moved carrier.
- *The last carrier (Corollary GH).* At a state where U is the only off block with pieces of types 7 and 9 and its
  holes of these types are glued to it, every step V W V^-1 with V a word of K+ letters and V(U) = U needs Bad(W) in
  types 7 and 9 to be exactly U's glued holes: Bad(W)_t lies in the holes of type t of V(Z), which are U's own
  (Proposition GH), and |Bad(W)_t| = |Bad(W^-1)_t| >= |U_t| (Lemma K and the landing). The hypotheses must be
  checked at the state where the corollary is used: the gluing is a property of the state. They are checked at 233
  and at the three end states of the next subsection (certificate). In the anatomy of Section 5.3 the rare part of
  Bad(W^-1) is the landing L and the 3 chambers X', 6 chambers where 3 are allowed (excess 3; measured).

Two remarks:
- *Later states.* A clean step only removes pieces from the off set, so a type without holes keeps none: at every
  state reached from 233 by clean steps, every closable word must miss the 15 hole-free types as well. Certificates 2
  to 6 below concern the last carrier of types 7 and 9, which the carrier-move steps of the next subsection leave.
- *Deeper refinements.* Below a W_1 with a dead ghost of type 29, no refinement of any depth can be closed (Lemma I).
  The open W_1 (no dead type-29 ghost) have refinements of depth two and more; Theorem Z does not cover them.

#### Carrier-move closure at 233: 233 -> 86 and 233 -> 61 (certificate)

**Lemma CM (carrier move; proved).** Let Z be a state, B an off block, and c a grip whose cap holds every piece of B,
whose cut no piece straddles, and whose cap holds no piece of any other off block. Then every K+ letter (c, k) is
admissible at Z, keeps every lattice piece on the lattice, keeps every other off piece at its exact pose, and moves B
rigidly by k. Let P0 be a word of such letters, with the hypothesis checked again before each letter for the moved B.
Then P0 moves B rigidly by the product g of its elements and moves no other off piece; if a hole h is glued to B
(conditions 1 to 3 of Proposition GH), the vacancy from h ends at g(h). Hence, if V W V^-1 is a clean step at P0(Z)
for a unit A with B in its free set, then P0 V W V^-1 is a clean step for A at Z with free set B. The off count falls
by at least |A|, and by |A| + |B| if W also lands g(B) (Lemma U).

*Proof.*
1. No region straddles cut c: no off piece does by hypothesis, and lattice regions never straddle. So (c, k) is
   admissible.
2. Cap c holds only lattice pieces and the pieces of B. A K+ letter keeps lattice pieces on the lattice (Fact 2) and
   multiplies the pose of every piece of B by k. Every other off piece lies outside cap c (cap c holds none of its
   pieces and none straddles cut c), so it does not move.
3. By induction over P0, B is moved rigidly by g and nothing else off the lattice moves. B stays off the lattice:
   its poses are not in K+, and g is.
4. Each letter of P0 has a cap that holds all of the current image of B, unstraddled. By the remark after the proof
   of Proposition GH, the vacancy from a hole glued to B moves with B, so it ends at g(h).
5. P0 keeps every lattice piece of Z on the lattice and every off piece outside B at its exact pose, and no off
   piece lands. So the lattice pieces of P0(Z) are those of Z, and the off pieces of P0(Z) outside A and B are those
   of Z at their poses. A clean step at P0(Z) for A with free set B keeps all of them so (definition of a clean step),
   and Lemma U gives the fall of the off count. QED

**Construction (measured; the replay decides).** Start: stop62@233 (`certs/stop62/z233/inputs/stop62-233-table.json`,
digest `46496bb1...`). Off: the three carriers 86@1040, 61@23955 and 86@64184; each carries 2 pieces of type 7 and 1
of type 9 and has 3 glued holes of these types (certificate above).
- *Words.* The 42 refinements of depth two that meet Lemma D at 233 (`certs/stop62/z233/rk23955.jsonl`, 20 rows;
  `rk64184.jsonl`, 22 rows). For a word W of carrier A, the *need* is the part of Bad(W) in types 7 and 9 outside A's
  glued holes. It has 3 chambers for each of the 42 words.
- *Reachability* (`ep/lemo/glue/reach.py`; `certs/stop62/cm233/reach233.json`, `.out`). The script computes the
  exact cut status of every carrier at every grip (0 outside, 1 inside, 2 straddling, 3 on both sides). It checks the
  K+ equivariance of these vectors (the status of g(B) at grip c is the status of B at g^-1(c)) exactly on one letter
  per carrier: all true. It then searches breadth-first over the rotations g of a carrier B reachable by letters that
  meet the hypothesis of Lemma CM, the other carriers fixed. Rotations reachable, of the 7,200 in K+: 7,180 (A =
  23955, B = 1040), 7,174 (A = 23955, B = 64184), 7,180 (A = 64184, B = 1040) and 7,180 (A = 64184, B = 23955). For
  every one of the 42 words and each other carrier B there are exactly 2 rotations g in K+ with g(H_B) = need, and
  both are reachable: all 42 words have a reachable cover. The example paths have 2 to 5 letters (2 letters: 2,
  3: 32, 4: 36, 5: 14, over the 84 word-carrier pairs). This is measured: the search is exact on the status vectors,
  and the replay on the simulator decides. `ep/lemo/glue/cover.py` is the version over all of K+ without the
  reachability search (no record file in the package).
- *Batch* (`ep/lemo/glue/cm/one.sh`; `certs/stop62/cm233/jobs.txt`, `res.txt`). 168 jobs: 42 words x 2 other
  carriers x 2 rotations. Each job applies the path on the exact simulator (`ep/lemo/glue/apply.py`; the simulator
  refuses an inadmissible letter), then runs `tclose2.py` on the moved table with unit A, the moved carrier B passed
  as `--free`, the third carrier protected, and a greedy limit of 600 s. When this text was written (02:11 UTC,
  15 October 2026), 22 of the 168 jobs had finished:
  - 6 wrote a step that the exact replay inside `tclose2.py` admitted ("replayed"); 3 of them are collected as
    chains and replayed independently (table below); the other 3 (A = 64184, B = 1040, each ending at 61 off) are not
    collected and are not counted as certificates;
  - 9 stopped with "no V": in 6, N1 and N2 fail at the moved state; in 3, N1 and N2 hold but the greedy search found
    no V within 600 s;
  - 3 failed the ghost premises;
  - in 4 the exact replay inside `tclose2.py` refused a twist ("blocked"), so no step was written.
  The batch continues; `res.txt` in the package is a later snapshot (an empty result there means a refused twist).

**Results (certificate: independent replay with `verify_u.py`, all_ok).**

| Chain | Job | Moved carrier | Path (menu letters) | V / W / V^-1 | Twists in all | Restored | Off after | Left | Lattice pieces of 233 at the same exact pose |
|---|---|---|---|---|---:|---|---:|---|---|
| `s62m` | 23955-0-64184-1811 | 86@64184 | 4: [[369,3],[448,8],[326,10],[172,1]] | 886 / 46 / 886 | 1,822 | 61 + 86 = 147 | 86 | 86@1040 | 175,679 of 176,887 |
| `s62m2` | 23955-0-1040-722 | 86@1040 | 3: [[5,10],[79,2],[94,8]] | 953 / 46 / 953 | 1,955 | 61 + 86 = 147 | 86 | 86@64184 | 175,679 of 176,887 |
| `s62m3` | 64184-0-1040-4593 | 86@1040 | 5: [[5,10],[8,5],[45,1],[221,11],[448,7]] | 849 / 46 / 849 | 1,749 | 86 + 86 = 172 | 61 | 61@23955 | 176,185 of 176,887 |

In each step the collateral is 0, every lattice piece of 233 ends on the lattice, the carrier left keeps its exact
pose, and W (a 46-letter word of depth two of carrier A: row 0 of `rk23955.jsonl` or `rk64184.jsonl`) lands both A
and the moved carrier. V comes from a full peeling certificate (for `s62m`, 162 of 162 targets peeled, bound 1,955).
End digests: `090b175c...` (`s62m`), `9963d65f...` (`s62m2`), `218f823f...` (`s62m3`). Files: `chains/s62m`,
`chains/s62m2`, `chains/s62m3`; `certs/stop62/cm233/<job>/` (`path.json`, `apply.json`, `w.json`, `tclose2.log`,
`pre.json`, `result.json`, `certificate.json`, `word-full.json`). Each chain has one step whose word is the path
followed by V W V^-1, with the moved carrier as the free set. After the path the 9 rare holes are A's own 3, the
third carrier's 3, and the moved carrier's 3 at their new places, which are the need of W (`apply.json`: for `s62m` the need 21949, 21950,
30094 of row 0 of `rk23955.jsonl`).

So stop62 goes 860 -> 689 (first package) -> 245 (driver) -> 233 (step 9) -> 86 or 61 (one carrier-move step). The
carrier-move step is outside the driver: a prefix that moves another carrier, then a word of depth two with that
carrier free. Corollary Z (the driver started at 233 from the state alone has no step) is unchanged.

**The last carrier (certificate and measured).**
- *Still glued (certificate).* `glued.py` at the three end states (`certs/stop62/cm233/glued-s86a.json`,
  `glued-s86b.json`, `glued-s61c.json`): the last carrier's 3 holes of types 7 and 9 are still glued to it
  (all_glued true), and they are the same chambers as at 233: s86a (end of `s62m`, 86@1040 left) 2732, 2743, 4736;
  s86b (end of `s62m2`, 86@64184 left) 119124, 119141, 128877; s61c (end of `s62m3`, 61@23955 left) 11335, 11338,
  28650. So Corollary GH applies at these states with its hypotheses checked.
- *The driver stops at once (measured).* `cbrun3.py` started at each end state with `--only` the last carrier
  (`runs/s86a`, `runs/s86b`, `runs/s61c`; s86a and s86b were given the family-245 directory as seed, which added no
  word; s61c had no seed) stops at step 0 with "no unit restored at this state". It scores 24 usable single letters,
  the fresh search adds 601 D-type and C-type words, one refinement round adds 4, 5 and 8 words, and no usable word
  meets the vacancy conditions. Over the usable words the smallest deficit (Lemma D) is 247, 298 and 272, and the
  smallest |Bad(W)| is 333, 384 and 333, far above the 86, 86 and 61 holes, so no word of K+ letters can vacate them
  (Lemma D). (The stop event in `run.jsonl` lists the best words as [n2, n1, deficit, occupied Bad, |Bad(W)|, moved
  blocks]; the minima over all usable words come from the runs' score files, which are not in the package.)
- *What remains.* stop62 stops at one rare carrier, and what remains is exactly the last-carrier problem
  (Corollary D'', Corollary GH): for V with V(U) = U, a word whose Bad set in types 7 and 9 is the carrier's own 3
  glued holes. With one
  carrier left there is no other carrier to move. A partner would have to be created first (a step that is not
  monotone in the off count), or a new word class found. Both are open.

#### The last carrier at 233 and after: Certificates 2 to 6

**Certificate 2: the last carrier of types 7 and 9 (Corollary D'').** The pieces of types 7 and 9 (labels of
`regions.orbit_of`) are off only in the three blocks 1040, 23955 and 64184, two of type 7 and one of type 9 in each.
Whichever of the three is restored last, its step needs |Bad(W)_7| <= 2 and |Bad(W)_9| <= 1 if it lands no other
carrier. Bad(W) depends on the word only, so this is checked once per word (`ep/typeswap.py`):

| Carrier | Usable words checked | Excess histogram (excess: words) | Words that meet the bound |
|---|---:|---|---:|
| 86@1040 | 629 | 3: 569, 6: 9, 8: 11, 9: 16, 93: 24 | 0 |
| 61@23955 | 1,448 | 3: 1,341, 5: 6, 6: 21, 8: 20, 9: 36, 93: 24 | 0 |
| 86@64184 | 1,464 | 3: 1,374, 5: 4, 6: 10, 8: 18, 9: 34, 93: 24 | 0 |

(excess = sum over t in {7, 9} of max(0, |Bad(W)_t| - |U_t|), over the pools searched at 689.) The smallest excess, 3,
is the size of the landing in these two types: the words' Bad sets hold the landing twice (measured; Section 5.3).
Over the whole D-type and C-type family at 245 (Theorem S) the check gives the same answer: 7,824, 7,104 and 7,668
words for 1040, 23955 and 64184, none meets the bound, and the smallest excess is 3 (4,826, 4,106 and 4,826 words)
(`certs/stop62/family-245/ts<block>.json`).

**Certificate 3: the holes are held by the carriers (N1, N2 at 689).** At 689 all nine holes of types 7 and 9 lie in
chambers that no SAFE-ALL letter moves (singleton SAFE-ALL orbits). For the best words of each carrier the SAFE-ALL
orbit of type 7 has 2 occupied Bad chambers and no hole, and the orbit of type 9 has 1 and none (n2 = 3); the grips
whose caps hold those holes are unsafe because they hold the carrier itself or are straddled by it
(`certs/stop62/rel-*.json`). This is the case for Lemma V*; it does not remove Certificates 1 and 2.

**Independent recount for Theorem S (certificate).** `ep/ghostsim.py` recounted |Bad(W)| and its type-7 and type-9
parts for every one of the 37,380 words of the four families, with the simulator's own classifier and no ghost library
(from the all-lattice end of chain `d1`). There were 0 mismatches, and the smallest |Bad(W)| is 333 for every unit
(`certs/stop62/family-245/ghostsim-<block>.jsonl.gz`).

**Certificate 4: after 245 the driver can only use refinements, and the carriers keep their poses (from the driver's
code, read, not proved; certificate).**

1. Every step is clean (Theorem G) and lowers the off count (Lemma U). So every later state Z' of a run from 245 has
   off(Z') <= 245.
2. The words the driver can hold for a unit U at Z' (`cbrun3.py`):
   - (a) the pool at 245: the seeded words, the single letters at the movers, the search and the refinement round at
     245, including words that were not usable at 245;
   - (b) the single letters at the movers of U's block at Z';
   - (c) the D-type and C-type words that `dfam2.py` finds at Z';
   - (d) the refinements [W, k] that `comm3.py` makes of usable words W at Z' that move no other block, and
     refinements of those at later states.
   `dfam2.py` also writes parked words P Q P^-1, but to a separate file (`.park.jsonl`) that the driver never reads:
   `search()` reads only `.q.jsonl`, and `seed()` reads `.q.jsonl` and `comm3.py` rows.
3. Bad(W) depends on W alone. Whether W lands U depends only on W and on U's pieces, which keep their poses (item 4).
   - (a) Every usable word of the 245 pool has |Bad(W)| >= 277 (Certificate 1). For the three carriers the bound is
     333. Certificate 1 covers only the usable words. A pool word that was not usable at 245 lands U later only if it
     landed U at 245, so it was unusable because it disturbed another off block; such words are not covered here.
   - (b) A single jumble letter has its whole cap, 3,097 chambers, as its Bad set. A K+ letter lands nothing.
   - (c) At a later state `dfam2.py` can only produce D-type and C-type words of U whose grips have caps that hold U
     and are not straddled by U; other blocks can only remove grips. Theorem S covers the grips admissible at 245.
     `dfam2.py --adm-T --only-new` enumerates the rest (`ep/lemo/ext/x<B>.q.jsonl.gz`): 0, 12,684, 720 and 156 new
     words for 1040, 18445, 23955 and 64184, with smallest |Bad(W)| 349, 531 and 695 for the last three.
   By Lemma D summed over the types (|Bad(W)| <= off(Z') <= 245), none of the words covered by (a) to (c) can be
   closed at Z'. So every step of the driver after 245 uses a refinement made after 245, or a word of the earlier pools
   that was not usable at 245 (the gap of item (a)). The unseeded driver of Corollary Z has no such words.
4. `comm3.py` keeps only refinements whose ghost leaves every protected off piece at its exact pose. The free set of a
   single-frame block is empty, and V moves no off piece. So a refinement step changes only its own unit.
5. Hence, in the driver, the three carriers keep their poses from 245 until each is restored, and they are restored
   one at a time (the carrier-move steps above are outside the driver and restore two at once). The last one is
   restored with f_t = 0 in Corollary D''. Apart from the gap of item 3(a), its word is an iterated refinement W_n
   (n >= 1) of a *root* W_0 that lands the carrier at every depth (`comm3.py` refines only landing words). A root is a
   usable word of the 245 pool, a word of the base or extended family, or a single letter.

**Certificate 5: no refinement of depth one restores the last carrier, and dead ghosts close most of the tree
(certificate; Corollary I).** `ep/lemo/deep/refcert.py` takes as roots, for each carrier:
- every word of its base family (Theorem S) and of its extended family (Certificate 4);
- every word of the 245 pool (the words usable at 245, Certificate 1) that lands it;
- every single letter at every grip, of which those that land it are kept.

For each landing root W_0 it computes D(W_0) in types 7 and 9. If D(W_0) is empty, it scores every landing refinement
W_1 = [W_0, k]: every grip f whose cap holds the landing, and every non-identity K+ letter at f with k(L) disjoint from
L. By Proposition W, step 3, these are all the landing refinements. For each W_1 it computes the excess and D(W_1).

| | 86@1040 | 61@23955 | 86@64184 |
|---|---:|---:|---:|
| Landing roots (words of length 10 / 4 / 22 / 46 / 94; single letters) | 7,982 (7,368 / 456 / 56 / 74 / 4; 24) | 7,870 (7,368 / 456 / 20 / 2 / 0; 24) | 7,851 (7,368 / 456 / 3 / 0 / 0; 24) |
| Roots that are swap words | 0 | 0 | 0 |
| Roots with a dead rare ghost (closed by Corollary I) | 3,113 | 3,048 | 3,048 |
| Refinements scored / landing | 194,760 / 188,851 | 192,880 / 188,121 | 192,120 / 187,017 |
| Excess of the landing W_1: 3 / 5 / 6 / 8 / 9 / more (largest) | 172,030 / 992 / 1,722 / 1,741 / 10,791 / 1,575 (66) | 172,154 / 993 / 1,746 / 1,734 / 10,292 / 1,202 (66) | 171,485 / 967 / 1,686 / 1,627 / 10,224 / 1,028 (66) |
| W_1 that are swap words | 0 | 0 | 0 |
| W_1 with a dead rare ghost / open | 34,731 / 154,120 | 34,697 / 153,424 | 34,161 / 152,856 |

(`ep/lemo/deep/rc<block>.json`; 17 to 19 minutes per carrier on this container. The words of length 22, 46 and 94 are
the driver's own refinements in the 245 pool. Every 20th open W_1 is kept in `rc<block>.open.jsonl.gz`.)

So:
- No root and no refinement of depth one is a swap word for any carrier.
- Below every root and every W_1 with a dead chamber of type 7 or 9, no refinement of any depth is one (Corollary I).
- What remains are the refinements of depth >= 2 below the open W_1 (D(W_1) empty in types 7 and 9). The method below
  decides them depth by depth; the exhaustive run was stopped when the work moved to Theorem Z.

**Depth two by the refinement identity (method checked; the run is partial, measured).** Proposition J gives |Bad([Q, k])_t|
from three pieces of data of Q alone: D(Q)_t, B' = Bad(Q^-1) and the off contents S(Q). Landing is decided by the same
data: [Q, k] lands U if and only if k~(L) misses B' (all types). So one forward and one inverse ghost of Q decide
every refinement of Q, with no ghost of the refinement itself. `ep/lemo/deep/refj.py` uses this for depth two:
- for every root it predicts all W_1 = [W_0, k_1] from the data of W_0;
- it ghosts every predicted landing W_1 exactly and compares the prediction (landing and |Bad(W_1)_t| for t = 7, 9);
- for every open W_1 it predicts all W_2 = [W_1, k_2] from the data of W_1, and ghosts any W_2 predicted to be a swap
  word.

Check of the method: on the 40 roots of the sample below, the prediction agrees with the exact ghost on all 1,016
landing W_1, and the depth-two histogram it predicts equals the one that `refcert.py` computed by ghosting each of the
32,532 W_2 (`rj-t.json` against `s2.json`).

The exhaustive run (12 parts) was stopped when the work moved to Theorem Z. One part finished:
one quarter of the roots of 61@23955 (1,968 landing roots), with every one of its 45,639 predicted landing W_1 confirmed
by the exact ghost (no mismatch), 37,010 open W_1, 1,432,197 landing W_2, smallest excess 3 and no swap word
(`ep/lemo/deep/rj23955-p0.json`). The other parts stopped after about 1,000 roots each with no swap word and no
mismatch. This is a measurement on part of the tree, not a certificate for depth two.

**The sample used for the check (measured).** Every 200th word of the family of 86@1040 (40 roots) gives 1,016 landing
W_1, of which 840 are open, and 32,532 landing W_2, each ghosted exactly: smallest excess 3, no swap word, 8,751 with a
dead rare ghost (`ep/lemo/deep/s1.json`, `s2.json`).

**Deeper, sampled: a closed set of contents (measured).** For one root W_0, let Omega be the set of contents of types
7 and 9 obtained from the lattice contents in the caps of W_0's grips and of the grips G whose caps hold its rare
landing, and the carrier's rare pieces, closed under W_0, W_0^-1 and the K+ letters at G (each a partial map: inside the
cap moved, outside fixed, straddling dead). On a closed Omega every iterated refinement W_n with letters at G restricts
to a partial bijection of Omega, and W_n|Omega is determined by W_(n-1)|Omega and k_n. So a breadth-first search over
the distinct restricted maps follows the whole tree exactly in types 7 and 9, merging words with the same restricted
map. It allows every letter at G whose cap holds the rare landing, which is more than the driver allows; branches whose
rare landing leaves the caps of G are counted and not followed. `ep/lemo/deep/rclose.py`:
- the first root of 86@1040: Omega has 617 contents; 20,003 distinct restricted maps up to depth 4, 72,424 landing
  transitions, all with excess exactly 3; 1,880 branches leave the caps of G;
- the same root, type 9 alone (Omega 371): 200,015 maps up to depth 5, 1,412,136 landing transitions, all with type-9
  excess exactly 1; type 7 alone (Omega 399): 200,004 maps, 1,055,868 transitions, all with type-7 excess exactly 2;
- every 250th landing row of the family of 86@1040 (32 roots, at most 20,000 maps each): the search was stopped
  after 24 roots when the work moved to Theorem Z. Of those, 20 reach depth 4 without exhausting the tree (400,201
  maps in all, 1,201,044 landing transitions, 60,870 branches leaving the caps of G); 15 have every landing transition
  at excess exactly 3, the other 5 have excess 3 and more. For the other 4 roots every landing refinement has a dead
  rare ghost (smallest excess 14 to 23). No swap word (`ep/lemo/deep/bfs-sample.jsonl`).

The search does not end: the number of distinct restricted maps keeps growing with the depth. For the first root, in
each of the 6,352 open landing maps recorded (a run with 5,000 states), the chambers that no lattice content enters are
exactly L and the three chambers X' of the root's copy (the X' of Remark S), and X' lies in Bad (measured; this run
has no record file in the package). This is the invariant that a proof for every depth would have to establish;
Section 5.3 gives the attempt and where it stops.

**Certificate 6: parked words (for completeness).** The driver does not use parked words (Certificate 4, item 2). The
natural extension that would use them fails as well. `ep/lemo/parkall.py` covers every family word Q of a carrier
whose Bad set lies in one lens cap p (`ep/lemo/lens.py` with no bound on |Bad|: 261 words per carrier; the files
`ep/lemo/lens<block>.json` in the package are the run with `--max-bad 666`, which lists 93 of them for 86@1040). For
each such Q it scores every
parking word W = P Q P^-1 with
- P = [(c, X), (p, y), (c, X^-1), (p, y^-1), (z, i), (p, y), (c, X), (p, y^-1), (c, X^-1), (z, i^-1)],
- y every non-identity K+ letter at p, where p is a lens cap of Q that is a pole of X's frame;
- (z, i) every letter outside K+, at any grip, whose element maps the normal of grip e to that of grip c.
  There is no admissibility test, so this covers more words than `dfam2.py` makes in any state.

Results:
- 238,392 parking words per carrier, of which 234,186 land it.
- The excess histogram is 3: 234,016, 5: 4, 6: 156, 9: 10 for each of the three carriers. The per-word records are
  permutations of each other: the three carriers are congruent in this computation.
- No word meets the bound.

**Conclusion.** With the D-type and C-type families, their refinements and the single letters, the driver takes stop62
from 689 to 245 off pieces in 8 clean steps and stops there (run `s62g`). Step 9, a refinement of depth one closed
outside the driver, reaches 233. One carrier-move step, also outside the driver, reaches 86 or 61. Every step has
collateral 0 (certificate: replay).
- At 233 only the three carriers of types 7 and 9 are left, and 15 of the 36 types have no hole.
- No word of the base or extended family, no usable word of the pools and no single letter can be closed there
  (Theorem S, Corollary S, Certificates 1 and 4: |Bad(W)| >= 333 > 233).
- No landing refinement of depth one of any of these words can be closed either: its Bad set meets type 29, which
  has no hole (Theorem Z).
- So the driver started at 233 from the state alone has no step there, whatever its parameters and even with closure
  by any word of K+ letters (Corollary Z). Seeded with every stop62 pool it stops there too (measured).
- Refinements of depth two that meet Lemma D at 233 exist (42 found). The driver's closure cannot vacate their Bad
  sets (N2, certificate per word), and no closure that returns every carrier to its pose can (Proposition GH).
- A prefix that moves a second carrier with its glued holes, followed by such a word with that carrier free, does
  work (Lemma CM): three steps, each restoring two carriers (certificate).
- At the three end states the last carrier's holes are still glued to it (certificate), and the driver started
  there stops at once (measured).
The family form is therefore false for the family of depth at most one, at an explicit reachable state. For iterated
refinements of depth two and more, with closure by any K+ word and with prefixes that move other off blocks, it
stays open. At stop62 the open case is the last carrier. A step for it needs a word whose Bad set misses the hole-free
types and whose Bad set in types 7 and 9 is exactly the carrier's own 3 glued holes (Corollary GH, for V with
V(U) = U). Such words exist (an inverse history is one, Section 5.3), Proposition GH does not exclude them, and none
is known.

### 6.2 Multi-frame blocks

**The state.** The run r40s11 of the second package ends at 9,033 off pieces (digest `8847aa81...`, replayed there).
Step 1 restores a single-frame block of 111 pieces with a 10-letter D-type word (Bad(W) 198 chambers, none occupied,
V empty), giving tc35-0 (8,922 off, 134 blocks). There block 7127 has 131 pieces in two frame domains (86 in domain 7,
45 in domain 4), four inside grips, three of them movers (193, 201, 218) and one straddled (247)
(`certs/multiframe/bc-tc35-0.json`). It is the only multi-frame block of that state.

**The step (certificate: replay).** Unit = the 86 pieces of domain 7; free set = the 45 pieces of domain 4 plus the
pieces of the other blocks the word moves (242 in all). Word: a 10-letter D-type word with Bad(W) of 384 chambers,
none occupied, so V is empty. Result: 8,922 -> 8,836, the 86 pieces restored, collateral 0, the 242 free pieces moved
(still off), every other off piece exact.

**After the step.** The 45 pieces of domain 4 are one single-frame block. At 8,836 its usable words all fail the
vacancy conditions (n1 = 25, n2 = 25), so the run stops there; this is the vacancy obstruction of Section 5.3, not a
multi-frame obstruction.

**Second state (measured: a search outcome).** The end state of the W-J 60-record run of the second package (17,316
off, 259 blocks) has two
multi-frame blocks: 94 pieces in domains 16 (86) and 14 (8), and 147 pieces in domains 4 (86) and 8 (61)
(`certs/multiframe/bc-end-p60.json`). The run `runs/mf60` focuses on the four frame units and stops at its first state:
- Each unit has 24 usable single letters, and a fresh search adds 6,000 D-type and C-type words.
- No usable word meets the vacancy conditions: the smallest n2 over the usable words of every unit is 2 (with n1 = 0
  and deficit 0). The refinement round adds no word.
- This is again the vacancy obstruction of Section 5.3, not a multi-frame one: the type counts are fine (deficit 0),
  but two Bad chambers sit in SAFE-ALL orbits without a hole.

**What this shows.** Lemma U needs nothing about frames: a multi-frame block is restored frame by frame, each frame a
unit whose free set holds the other frames (proved). The instance confirms that the driver finds and writes such a
step (certificate).

### 6.3 Frozen blocks

**The state.** x16-1076 is the sub-history of the first 16 records of the W-J I_a walk that touches the caps around
block 1076 (`fz/extract.py`: 9 of the 16 records are kept, at grips 57, 86, 173, 315, 36, 36, 326, 168 and 181; every
twist admitted on the exact simulator). It has 21,113 off pieces, 212 blocks, 31 of them frozen (3,816 pieces;
`certs/frozen/bc-x16-1076.json`).

**Steps (certificate: replay).**

| Step | Unit | Word | Free set | Off pieces | Restored | Collateral |
|---:|---|---|---:|---|---:|---:|
| 1 | 86@1067 | [X, B], X at grip 111, B at its mover 36; V empty (Bad 592, none occupied) | 506 | 21,113 -> 20,719 | 394 | 0 |
| 2 | 78@45811 | one letter at grip 181; V empty (Bad = cap 181, all holes) | 2,349 | 20,719 -> 18,292 | 2,427 | 0 |
| 3 | 86@31895 | one letter at grip 315, V = two letters at grip 181 (greedy) | 3,011 | 18,292 -> 15,590 | 2,702 | 0 |

**Proposition F in the instance (certificate: `bclass.py` at each state).**
- At 20,719 five frozen blocks (86@31895, 86@45552, 86@48368, 111@28625, 385@53396; 754 pieces) share the inside grip
  315. Exactly four blocks straddle cap 315: 61@32359, 10@32385, 78@45811 and 15@45818 (164 pieces), all with the
  mover 181.
- Step 2 restores those four (and the rest of cap 181) with one letter. At 18,292 grip 315 is a mover of all five
  blocks (two of them also get the movers 243 and 145).
- Step 3 restores all five frozen blocks in one step.
- The other 26 frozen blocks are unchanged (the same piece sets at 18,292 and 15,590) and are not part of this run.

**One level more (run `runs/fz6`; certificate: replay and `bclass.py`).** At 15,590, 26 blocks are frozen
(`certs/frozen/bc-s3.json`). Four of them share the inside grip 86: 385@3009, 86@1729, 86@12875 and 86@25409. 25409 also has the
inside grip 79, which 3009 straddles: that looked like a second level. Cap 86 is straddled by six blocks with the
common mover 168. The run focuses on these ten blocks (1,235 pieces). Chain `frozen2` continues chain `frozen`:

| Step | Unit | Word | Free set | Off pieces | Restored | Collateral |
|---:|---|---|---:|---|---:|---:|
| 4 | 111@22652 | one letter at grip 168; V empty (Bad = cap 168, all holes) | 2,257 | 15,590 -> 13,222 | 2,368 | 0 |
| 5 | 86@1729 | D-type word, 10 letters; V empty (Bad 592, none occupied) | 395 | 13,222 -> 12,741 | 481 | 0 |
| 6 | 86@12875 | [X, B], X at grip 97, B at its mover 86; V empty (Bad 790, none occupied) | 506 | 12,741 -> 12,149 | 592 | 0 |

- Step 4 restores its unit and 28 other blocks with one letter: all 2,257 free pieces land, and every protected off
  piece keeps its pose. Cap 86 is unstraddled afterwards, so the six blocks that straddled it are among those
  restored.
- At 13,222 no block is frozen (125 blocks: 104 with two or more movers, 21 with one; `certs/frozen/bc-13222.json`). Each of the
  26 frozen blocks of 15,590 was either restored in step 4 or now has a mover. Grip 86 is a mover of 3009, 1729, 12875
  and 25409. So 25409 is unlocked at grip 86 directly; the second level through grip 79 is not needed (Proposition F).
- Steps 5 and 6 restore 1729 and 12875.
- The run stops at 12,149 with 3009 and 25409 still off, both with the mover 86 (`certs/frozen/bc-12149.json`). Neither has a
  usable word that meets the vacancy conditions:
  - For 3009 the only usable words are the 12 coset letters at grip 86. They restore the whole block, but their Bad
    set is cap 86, with 1,281 occupied chambers, 794 of them in singleton SAFE-ALL orbits.
  - For 25409 the best words leave 141 occupied Bad chambers in singleton SAFE-ALL orbits (n1 = n2 = 141).
  - The deficit is 0 for both.
  This is the vacancy obstruction again (Section 5.3), not a frozen-block one.

### 6.4 Depth one

**Theorem D1 (proved).** Let Z0 be a state with every piece on the lattice and Z = t(Z0) for a jumble letter t at grip
c. Then each of the 12 coset letters mu of Lemma H at c is a clean step at Z, with V empty, that puts all 3,097 off
pieces on the lattice with collateral 0. Every unit of Z has mu as a usable vac_ok word with occupied Bad 0, and every
other usable single letter has occupied Bad > 0.

*Proof.*
1. A jumble letter sends every lattice piece of its cap off the lattice (its element is not in K+), so off(Z) is the
   3,097 pieces of cap c, all with poses in one coset E K+ (E the letter's element): one frame domain. So every block
   is one unit with an empty free set.
2. Cap c is unstraddled at Z (Theorem R of `PROOFS-GF.md`) and holds every off piece, so c is a mover of every
   block. (That the driver puts the 60 letters at c into every unit's pool is a fact about the driver, used only in
   Corollary D1.)
3. For each of the 12 letters mu = k0 E^-1 of the coset that sends the frame back (Lemma H), mu is admissible (cap c
   unstraddled), puts every piece of cap c on the lattice and moves nothing else: the unit is on the lattice, no
   piece dies, the other blocks are moved and land (free). Bad(mu) is cap c (mu is not in K+), and every chamber of
   cap c is a hole at Z (the off pieces fill the cap), so the occupied Bad is 0, n1 = n2 = 0, and mu is a usable
   vac_ok word with occupied Bad 0. With V empty, mu is a clean step: every off piece lands and no lattice piece
   leaves the lattice.
4. Every other usable single letter has occupied Bad > 0: a letter at c outside the coset leaves the unit off; a
   jumble letter at another mover c' carries the lattice pieces of cap c' outside cap c off the lattice (cap c' is not
   inside cap c, since caps of different grips differ and have the same size), and a K+ letter restores nothing.
QED

**Corollary D1 (the driver at depth one; from the driver's code, read, not proved).** The driver tries words with
occupied Bad 0 first (Section 4, step 5), so by Theorem D1 its first try at Z is a coset letter. The current
`tclose2.py` takes V empty when Bad(W) holds no lattice piece, and its replay admits mu and ends with all 3,097
pieces on the lattice and collateral 0. So the first try restores everything.

*Instance (certificate).* Z = (0, 12) applied to the solved state: 29 blocks, 3,097 off. Every unit had exactly 12
vac_ok words, all coset letters at grip 0 with occupied Bad 0 (`runs/d1/run.jsonl`). The tries for units 80 and 93
(letter (0, 14)) restored all 3,097 pieces; the replay ends with every piece on the lattice. (The tries for units 0
and 70 come first in the log as failures: they ran with the earlier `tclose2.py`, which built a peeling certificate
for an empty V, and were stopped by hand; the current `tclose2.py` takes V empty there. So Corollary D1 depends on the
version of `tclose2.py`.)

So at depth one the family has a step from the state alone (Theorem D1), and the driver as shipped tries it first
(Corollary D1).

## 7. Errata to the earlier packages

Second package (`Claude-3E-general-family-20261014`):

**E1. Proposition N is about single letters.** `PROOFS-GF.md` Section 0 says "From the state alone, as a monotone
step: no", and `REPORT.zh-CN.md` Section 4 (F1) says that below 1,549 off pieces no monotone step exists. Proposition
N(b) proves less: every *jumble letter* raises the off count to at least 3,097 - h, so every restoring word passes
through such a state inside itself. A word of several letters can still end lower than it started. The clean steps
of Section 6 do exactly that: at stop62 every step lowers the off count (689 to 674 to 596 to 593 to 506 to 496 and
on), while its jumble letters raise it in between. Corrected statement: below 1,549 off pieces no *single letter*
lowers the off count; multi-letter clean steps can, and Lemma U counts their progress. Proposition N(b) itself is
unchanged.

**E2. The CX family is not covered by Proposition N(a).** `PROOFS-GF.md` Sections 3 and 7 say that the S, SV and CX
families have no parameter below 3,097 off pieces. Proposition N(a) proves this for families whose core needs a
lattice-free cap (S and SV). The CX core [X, V] (`gf/cx.py`) does not need one. At Y (2,307 off) the CX runner found
no parameter (measured), but that is not a proof. Corrected label: S and SV, proved; CX, measured at Y.

Nothing else in the second package is affected: Lemma P, Theorem R, Corollary R, Theorem C, the runs and their
replays stand.

First package (`Claude-3E-block-family-20261014`, `PROOFS-BF.md` Section 5.1 and the summary table of Section 0):

**E3. The conditional claim at stop62 is superseded: its hypothesis cannot be met with these word pools.** The first
package says that, conditional on a vacating word at each turn, the family restores all 689 off pieces of stop62 in
the order of the conflict digraph. The implication itself is not refuted: Theorem CB of the first package still
holds. What this package shows is that its hypothesis cannot be met with these word pools, in any order. The reason
lies in the words, not in the carriers themselves. The three blocks 61@23955, 86@64184 and 86@1040 are the only
carriers of the types 7 and 9, and every usable word in their pools exceeds the bound of Corollary D'' for the
carrier that is restored last, unless that step also lands another carrier (certificate, Certificate 2 of Section
6.1). In the order
of the runs of Section 6.1 the state at 245 off pieces fails Lemma D for every usable word of all four remaining
blocks (|Bad(W)| >= 277 > 245) and for every word of their whole D-type and C-type families (Theorem S,
|Bad(W)| >= 333); a refinement of depth one restores the 12-piece block (245 to 233, step 9, outside the driver), and
at 233 no root and no refinement of depth one of the three carriers can be closed (Theorem Z). A step that lands two
carriers at once does exist: the carrier-move steps of Section 6.1 go from 233 to 86 or 61 (certificate). The
conflict digraph, the words found and the restored blocks stand. The claim should read: the family and its
refinements of depth one restore 456 of the 689 off pieces (to 233, Section 6.1); one carrier-move step restores two
of the three carriers left (147 or 172 more pieces); the last carrier needs other words or another closure (open).

The first package (Section 7) also names the vacating search of its Lemma V' as the only step with a route, with the
test of restoring stop62@689 completely. By Lemma D no word of K+ letters vacates any word of the base family at 245
(Theorem S), so that route alone cannot finish stop62. The progress from 689 to 233 came from a new word class, the
refinements [Q, k], and the progress from 233 from a prefix that moves another carrier (Lemma CM).

**Status of the open items of the earlier packages.** General E stays open in all three packages. The Block lemma is
now split into several statements. The second package proved it along a history (Theorem R). This package proves the
weak form and the depth-one case from the state alone (Theorem D1, chain `d1`), shows the family form false for roots
and refinements of depth one at stop62@233 (Theorem Z, certificate), and leaves it open for deeper refinements and in
its strong form.

## 8. Reproduction (Linux, headless)

- Research tree `research/jumbling` at commit `27a001c` (branch `claude/jumbling-e3`), clean. The scripts import
  `buffer`, `rim` and `restore` from it, and the five E-chain scripts in `scripts/echain/` (byte-identical to those of
  the first and second packages).
- Set `J=<research/jumbling>` and `OUT=<an output directory>`, run from `scripts/`, with `ulimit -v 7000000` and
  `python3 -I`. `B` is a block number (0: 86@1040, 1: 12@18445, 2: 61@23955, 3: 86@64184) and `<block>` the block name.
- A path `ep/...` in this document names a script under `scripts/ep/` (`.py`) or a data file under `certs/ep/`
  (anything else).

```bash
cd scripts
IN=../certs/stop62/family-245/inputs; F=../certs/stop62/family-245; Z=../certs/stop62/z233
E=../certs/ep/lemo; CM=../certs/stop62/cm233

# 1. Independent replays (simulator, pose table and recorded sets only; 3 s to 4 min per chain)
for c in s62 s62z s62m s62m2 s62m3 mf frozen frozen2 d1; do
  python3 -I gen/verify_u.py $J echain ../chains/$c/chain.json $OUT/replay-$c.jsonl
done

# 2. The driver (each run writes run.jsonl and one directory per try; minutes to hours)
python3 -I gen/cbrun3.py $J echain <start table> <name> <work dir> [--par 2] [--only <focus json>] [--seed <dir>]...
#    the runs of this package (start tables and seed directories as recorded in runs/*/run.jsonl; the seed
#    directories of the stop62 runs are not shipped, the usable part of their words is ep/lemo/deep/pool/):
#    s62d, s62f, s62g: --par 2 --seed <earlier stop62 pools>; s62f with --only (block 68424)
#    s62h: default parameters, start table $Z/inputs/stop62-233-table.json, --seed for every stop62 pool directory
#    s86a, s86b, s61c: --par 1 --greedy-seconds 600, start tables $CM/inputs/stop62-86-a-table.json,
#      stop62-86-b-table.json and stop62-61-c-table.json, --only $IN/s62g-k00-b00.json, b03.json and b02.json;
#      s86a and s86b also --seed $F (it adds no word)
#    fz4, fz5, fz6: --par 2 --only <focus json>; mf60: --par 2 --only <focus json>; d1: no options
#    runs/stop62 (steps 1 to 5) was written by an earlier version of the driver that is not in this package; its
#    steps are replayed in chain s62 (step 1).

# 3. Collect a chain from run logs (the start table is copied into the chain directory)
python3 -I gen/mkchain_u.py $OUT/s62 stop62 <start table> <run.jsonl>...

# 4. Block classes (size, movers, inside and straddled grips, frame domains)
python3 -I mf/bclass.py $J/theory echain <pose table> $OUT/bc.json

# 5. Certificates at a state: per-type swap condition and where the holes are
python3 -I ep/typeswap.py $J/theory echain <table> <unit json> <scored jsonl> $OUT/swap.json --types 7,9
python3 -I ep/release.py $J/theory echain <table> <blocks json> <scored jsonl> $OUT/rel.json --top 2

# 6. The frozen start state x16-1076 (sub-history of the first 16 W-J records around block 1076)
python3 -I fz/extract.py $J echain 16 ../certs/frozen/bc-p16.json 1076 $OUT/x16-1076-table.json

# 7. Theorem S: the whole D-type and C-type family of each unit at stop62@245
python3 -I gen/dfam2.py $J/theory echain $IN/stop62-245-table.json $IN/s62g-k00-b0B.json $OUT/u<block> \
    --max-q 100000 --min-q 100000 --good-q 100000 --park-q 0 --no-pole --commutator --blocks $IN/blocks.json
python3 -I ep/q2w.py $OUT/u<block>.q.jsonl $OUT/u<block>.summary.json $OUT/w<block>.jsonl
python3 -I ep/typeswap.py $J/theory echain $IN/stop62-245-table.json $IN/s62g-k00-b0B.json $OUT/w<block>.jsonl \
    $OUT/ts<block>.json --types 7,9

# 8. Independent recount of |Bad(W)| and of its type-7 and type-9 part with the simulator's own classifier,
#    from the all-lattice end of chain d1 (no ghost library; about 0.2 to 0.5 s per word)
python3 -I ep/ghostsim.py $J echain ../chains/d1/start-table.json ../chains/d1/step001-word.json \
    $F/u<block>.q.jsonl.gz $OUT/g<block>.jsonl --all --types 7,9

# 9. The extended family (Certificate 4, item 3(c); a root input of Theorem Z and Certificate 5)
python3 -I gen/dfam2.py $J/theory echain $IN/stop62-245-table.json $IN/s62g-k00-b0B.json $OUT/x<B> \
    --max-q 100000 --min-q 100000 --good-q 100000 --park-q 0 --no-pole --commutator --adm-T --only-new \
    --blocks $IN/blocks.json

# 10. Certificate 5: roots and refinements of depth one in types 7 and 9 (B = 0, 2, 3; 17 to 19 minutes each);
#     for 86@1040 leave out the empty extended family
python3 -I ep/lemo/deep/refcert.py $J/theory echain $IN/stop62-245-table.json $IN/s62g-k00-b0B.json \
    $OUT/rc<block>.json $F/u<block>.q.jsonl.gz $E/ext/x<B>.q.jsonl.gz $E/deep/pool/pool<block>.jsonl \
    --types 7,9 --singles --open $OUT/rc<block>.open.jsonl --open-every 20

# 11. Depth two by the refinement identity (12 parts: 3 carriers x --part 0/4 to 3/4; only part 0 of 61@23955
#     finished), and the sample used for its check (every 200th family word of 86@1040, 40 roots)
python3 -I ep/lemo/deep/refj.py $J/theory echain $IN/stop62-245-table.json $IN/s62g-k00-b0B.json $OUT/rj<block>-p<p>.json \
    $F/u<block>.q.jsonl.gz $E/ext/x<B>.q.jsonl.gz $E/deep/pool/pool<block>.jsonl --types 7,9 --singles --part <p>/4
python3 -I ep/lemo/deep/refcert.py $J/theory echain $IN/stop62-245-table.json $IN/s62g-k00-b00.json $OUT/s1.json \
    <sample rows> --types 7,9 --open $OUT/s1.open.jsonl
python3 -I ep/lemo/deep/refcert.py $J/theory echain $IN/stop62-245-table.json $IN/s62g-k00-b00.json $OUT/s2.json \
    $OUT/s1.open.jsonl --types 7,9 --open $OUT/s2.open.jsonl

# 12. The closed-set search (one root per call; the sample used every 250th landing row, at most 20,000 maps each)
python3 -I ep/lemo/deep/rclose.py $J/theory echain $IN/stop62-245-table.json $IN/s62g-k00-b0B.json <roots jsonl> \
    --types 7,9 --row <r> --max-states 20000

# 13. Certificate 6: family words with a lens cap, then every parking word
python3 -I ep/lemo/lens.py $J/theory echain $IN/stop62-245-table.json $F/u<block>.q.jsonl.gz [$E/ext/x<B>.q.jsonl.gz] \
    --out $OUT/lens<block>.json --max-bad 100000
python3 -I ep/lemo/parkall.py $J/theory echain $IN/stop62-245-table.json $IN/s62g-k00-b0B.json $OUT/lens<block>.json \
    $OUT/park<block>.json --types 7,9

# 14. Lemma O at 245: sampled interior points of every off piece
python3 -I ep/lemo/hloc.py $J/theory echain $IN/stop62-245-table.json <blocks json {blocks: [[pieces]...]}> \
    $OUT/hloc245c.json

# 15. Check of the refinement identity (Proposition J) on the refinements of four family words of 86@1040
python3 -I ep/lemo/deep/refid.py $J/theory echain $IN/stop62-245-table.json $IN/s62g-k00-b00.json <q.jsonl> \
    $OUT/refid.json --types 7,9 --max-q <n>

# 16. Step 9: closure of the refinement found at 245 and its chain (W in $Z/inputs/step9-w.json)
python3 -I gen/tclose2.py $J/theory echain $IN/stop62-245-table.json $IN/s62g-k00-b01.json \
    $Z/inputs/step9-w.json $OUT/c9 --greedy-seconds 600
python3 -I gen/mkchain1.py $OUT/s62z stop62-245 $IN/stop62-245-table.json $OUT/c9

# 17. Theorem Z at 233 (B = 0, 2, 3; about 30 minutes per carrier on this container, run in parallel)
python3 -I ep/lemo/hole/t0.py $J/theory echain $Z/inputs/stop62-233-table.json > $OUT/t0-233.json
python3 -I ep/lemo/hole/zhfull.py $J/theory echain $Z/inputs/stop62-233-table.json $IN/s62g-k00-b0B.json \
    $OUT/z1-<block>.json $F/u<block>.q.jsonl.gz $E/ext/x<B>.q.jsonl.gz $E/deep/pool/pool<block>.jsonl \
    --types 29 --singles --full $OUT/z1-<block>.full.jsonl > $OUT/z1-<block>.out
python3 -I ep/lemo/hole/ztable.py $OUT > $OUT/ztable.json   # the table of Theorem Z ($Z/ztable.json)
#     the runs at 245: zf18445-p* (step 9) is zhfull.py with the 245 table, B = 1, --types 29,30,32 --part <p>/4;
#     zh* is ep/lemo/deep/refcert.py with the 245 table and --types 29,30,32 --singles

# 18. Depth two in the hole-free type at 245 (the partial run behind the 42 words; part 0 of 4 per carrier)
python3 -I ep/lemo/hole/refjz.py $J/theory echain $IN/stop62-245-table.json $IN/s62g-k00-b0B.json $OUT/rz<block>-p0.json \
    $F/u<block>.q.jsonl.gz $E/ext/x<B>.q.jsonl.gz $E/deep/pool/pool<block>.jsonl --types 29 --singles --part 0/4 \
    --full $OUT/rz<block>-p0.full.jsonl > $OUT/rz<block>-p0.out

# 19. Depth two at 233: recheck the words found at 245, then the closure premises of each
python3 -I ep/lemo/hole/rechk.py $J/theory echain $Z/inputs/stop62-233-table.json $IN/s62g-k00-b0B.json \
    $OUT/rk<block>.jsonl $E/hole/rz<block>-p0.full.jsonl
python3 -I gen/tclose2.py $J/theory echain $Z/inputs/stop62-233-table.json $IN/s62g-k00-b0B.json <one W_2 as
    {"P": [], "Q": W}> $OUT/n2-<i> --greedy-seconds 600        # stops with "no V": N2 fails (pre.json)

# 20. Glued holes of types 7 and 9 (Proposition GH, conditions 1 to 3; a few minutes per state)
python3 -I ep/lemo/glue/glued.py $J/theory echain $Z/inputs/stop62-233-table.json $OUT/glued-233.json --types 7,9

# 21. Independent recount of sampled refinements of depth one (types 29, 30, 32)
python3 -I ep/lemo/hole/sampw1.py $J/theory echain $IN/stop62-245-table.json $IN/s62g-k00-b0B.json \
    $OUT/sw<block>.jsonl $F/u<block>.q.jsonl.gz $E/deep/pool/pool<block>.jsonl --types 29,30,32 --every 160 --per-root 2
python3 -I ep/ghostsim.py $J echain ../chains/d1/start-table.json ../chains/d1/step001-word.json \
    $OUT/sw<block>.jsonl $OUT/gs<block>.jsonl --all --types 29,30,32

# 22. Carrier-move closure at 233 (Section 6.1)
#   reachable rotations and covers for the 42 words of depth two (seconds)
python3 -I ep/lemo/glue/reach.py $J/theory echain $Z/inputs/stop62-233-table.json $Z/glued-233.json \
    $OUT/reach233.json --types 7,9 --rows 23955:$Z/rk23955.jsonl 64184:$Z/rk64184.jsonl > $OUT/reach233.out
python3 -I ep/lemo/glue/cover.py $J/theory echain $Z/inputs/stop62-233-table.json $Z/glued-233.json \
    $OUT/cover233.json --types 7,9 --rows 23955:$Z/rk23955.jsonl 64184:$Z/rk64184.jsonl
#   one job of the batch (a line "<job> A n B <unitA> <unitB>" of $CM/jobs.txt, where <unitA> and <unitB> name the
#   block files b00 to b03 of $IN; ep/lemo/glue/cm/one.sh runs these two
#   commands, with paths written for the build tree that must be edited): the path, then the closure with the
#   moved carrier B free and the third carrier protected (about 5 to 12 minutes)
python3 -I ep/lemo/glue/apply.py $J/theory echain $Z/inputs/stop62-233-table.json $CM/<job>/path.json \
    $OUT/t-<job>.json $OUT/a-<job>.json --types 7,9
python3 -I gen/tclose2.py $J/theory echain $OUT/t-<job>.json $IN/s62g-k00-<unitA>.json $CM/<job>/w.json \
    $OUT/out-<job> --free $IN/s62g-k00-<unitB>.json --greedy-seconds 600
#   the chain has one step: the "word" of apply.json followed by the "word" of out-<job>/word-full.json, from the
#   233 table, unit A, free set B; assemble it and replay it with step 1 (chains s62m, s62m2, s62m3; for s62m the
#   job is 23955-0-64184-1811 with <unitA> b02 and <unitB> b03, and the result equals chains/s62m file for file)
python3 -I ep/lemo/glue/cm/assemble.py $Z/inputs/stop62-233-table.json $OUT/a-<job>.json \
    $OUT/out-<job>/word-full.json $IN/s62g-k00-<unitA>.json $IN/s62g-k00-<unitB>.json <job> $OUT/chain-<job>
#   the end states: gluing (glued-s86a, -s86b, -s61c) and the driver (runs s86a, s86b, s61c; see step 2)
python3 -I ep/lemo/glue/glued.py $J/theory echain $CM/inputs/stop62-86-a-table.json $OUT/glued-s86a.json --types 7,9
python3 -I gen/cbrun3.py $J echain $CM/inputs/stop62-86-a-table.json s86a $OUT/wa --par 1 \
    --only $IN/s62g-k00-b00.json --seed $F --greedy-seconds 600
```

Not run here: anything on Windows or DirectX, any rendering, any timing claim. The run times above are for this
container and are not performance evidence.

Outputs cited in the text but not in the package: the anatomy counts of Section 5.3 (`anat.py`, `geo2.py`), the run
with 6,352 open landing maps (Section 6.1), the check of the refinement identity (`refid.py`, 132 refinements), the
output of `cover.py`, the sample roots of steps 11 and 12, and the score files of the runs at the three end states.
These statements are labelled measured and can be recomputed with the commands above.

## 9. Files

| Path | Content |
|---|---|
| `PROOFS-U.md` | This document |
| `REPORT.zh-CN.md`, `README.zh-CN.md` | Chinese report and package guide |
| `scripts/gen/` | `cbrun3.py` (driver), `mkblocks2.py` (units), `score4.py` (scores), `dfam2.py` (D-type and C-type search), `comm3.py` (refinement), `tclose2.py` (closure and replay of V W V^-1), `vacate2.py`, `mkchain_u.py` and `mkchain1.py` (chains from driver logs or from `tclose2.py` results), `verify_u.py` (independent replay) |
| `scripts/ep/` | Certificates and diagnostics: `typeswap.py` (Corollary D''), `q2w.py` (family rows to a word file), `ghostsim.py` (independent recount of Bad(W) on the simulator), `release.py` (stuck holes), `landing.py`, `deadbad.py`, `n2diag.py` |
| `scripts/ep/lemo/` | `lens.py` and `parkall.py` (Certificate 6), `hloc.py` (Lemma O) |
| `scripts/ep/lemo/deep/` | `refcert.py` (Certificate 5), `refj.py` (depth two by Proposition J), `rclose.py` (closed-set search), `anat.py`, `geo2.py`, `pairorb.py` (Section 5.3), `refdet.py`, `refid.py`, `refwho.py` (checks of Section 3.5) |
| `scripts/ep/lemo/glue/` | `glued.py` (conditions 1 to 3 of Proposition GH for every hole of the given types and every off block), `reach.py` (reachable carrier rotations and covers), `cover.py` (covers over all of K+), `apply.py` (apply a path on the simulator), `cm/one.sh` (one job of the carrier-move batch) |
| `scripts/ep/lemo/hole/` | `t0.py` (hole-free types), `zhfull.py` (Theorem Z), `ztable.py` (the table of Theorem Z), `refjz.py` (depth two in the hole-free types), `rechk.py` (recheck at another state), `sampw1.py` (sample for the recount), `holes.py`, `defs.py` |
| `scripts/mf/bclass.py`, `scripts/fz/extract.py` | Block classes; the frozen start state |
| `scripts/echain/` | The five E-chain scripts (copies) |
| `chains/<name>/` | `chain.json`, `start-table.json`, per step `stepNNN-word.json` (word with input and output digests), `stepNNN-unit.json`, `stepNNN-free.json`; `replay.jsonl` (the independent replay). Chains: `s62`, `s62z`, `s62m`, `s62m2`, `s62m3`, `mf`, `frozen`, `frozen2`, `d1` |
| `runs/<name>/run.jsonl` | Driver logs (states, scores, searches, tries, stops); `runs/s86a`, `runs/s86b`, `runs/s61c` also hold `driver.log` |
| `certs/stop62/` | `swaps-*.json` (Corollary D'' over the searched pools), `rel-*.json` (holes at 689), `deficit-245.json` (Lemma D at 245) |
| `certs/ptype-stop62-506.jsonl` | Per-type deficits at the stop62 state with 506 off pieces (Section 5.3) |
| `certs/stop62/z233/` | Theorem Z: `z1-<block>.json`, `.out`, `.full.jsonl` (every root and refinement of depth one with no type-29 Bad chamber, with Lemma D in every type), `ztable.json`, `t0-233.json`; `glued-233.json` (Proposition GH); depth two: `rk<block>.jsonl` (the 42 words rechecked at 233), `n2/` (the closure premises of each, `tclose2.py` `pre.json` and log); `inputs/` (the pose table at 233 and the word of step 9) |
| `certs/stop62/cm233/` | Carrier-move closure: `reach233.json`, `.out`; `jobs.txt` and `res.txt` (the batch; `res.txt` is a snapshot); one directory per collected job (`path.json`, `apply.json`, `w.json`, `tclose2.log`, `pre.json`, `result.json`, `certificate.json`, `word-full.json`); `glued-s86a.json`, `glued-s86b.json`, `glued-s61c.json`; `inputs/` (the pose tables of the three end states) |
| `certs/ep/` | Outputs of the `ep/` scripts cited in Sections 5 and 6, at the paths the text gives (`ep/lemo/deep/rc<block>.json` is `certs/ep/lemo/deep/rc<block>.json`), the 245 pools `ep/lemo/deep/pool/pool<block>.jsonl` and the extended family `ep/lemo/ext/x<B>.q.jsonl.gz`, `.summary.json` |
| `certs/stop62/family-245/` | Theorem S: `u<block>.summary.json` and `u<block>.q.jsonl.gz` (every word of the base family with its Bad size), `ts<block>.json` (Corollary D'' over the whole family), `ghostsim-<block>.jsonl.gz` (the independent recount), `inputs/` (the pose table at 245, the four units, the block list) |
| `certs/frozen/` | Block classes at 21,113, 20,719, 18,292, 15,590, 13,222 (`bc-13222.json`) and 12,149 (`bc-12149.json`) off; the x16-1076 extraction record |
| `certs/multiframe/` | Block classes at tc35-0 and at the end of the 60-record run |
| `SHA256SUMS.txt` | Checksums of every file |

The research tree and the five E-chain scripts are the only code outside the package.
