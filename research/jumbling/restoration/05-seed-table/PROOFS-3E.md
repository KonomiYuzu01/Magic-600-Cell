# 3E stage result: seed table and conflict graph (menu I_a)

Results of 14 October 2026. Scope: restoration of depth-one off pieces for the menu I_a, from the current state
only, by the seed-table and conflict-graph method proposed for this stage. The lattice phase (Card 5) and the
general E question are out of scope; nothing here claims that E is solved in general.

**Evidence kind.** Exact computations in synthetic geometry on the jumbling simulator of the research tree
(`research/jumbling`, branch `claude/jumbling-e3`, commit `27a001c`), run headless on Linux. Nothing here is
Windows, DirectX, native-host or performance evidence. The new proofs have not been independently reviewed.

**Labels.** *proved*; *certificate* (a finite exact computation; its file and script are named in Section 7,
and this document claims no more than the file records); *float evidence* (floating point without an exact
confirmation of the negative cases); *conditional* (the condition is named); *open*.

## 0. Status against the acceptance criteria of this stage

| Criterion | Status |
|---|---|
| 1. Seed table full (every class has a replayable seed and its restoration word) | **partial.** 252 seeds in 76 of the 500 position classes, all replayed (CERT-VERIFY). Every class that occurs in the frozen inputs X, E1, E2, I1, I2 (34 classes) has a seed. The deep samples use 231 classes, 64 of them seeded; the I_a@40 fixture uses 484 classes, 76 seeded. |
| 2. Conditional theorem (acyclic conflict graph gives sequential restoration with the other off poses fixed) | **proved** (Theorem C', with the needed-vacancy preparation as an explicit, per-instance hypothesis), and extended to jointly restored blocks (Theorem K). |
| 3. Measured coverage on frozen reachable inputs | **done** for X, E1, E2, E3, I1, I2 and two deep samples: each input is fully restored to the lattice or carries a blocking certificate (Section 4). The 94,147-piece fixture is classified only. |
| Gap lemma "a conflict cycle can always be broken" | **false for single seeds** (Lemma G): inside a glued block no choice of seed or buffer vacancy breaks the cycle. The gap is restated as the **Block lemma** (Section 2.6, open). |
| Condition R_E3 | **removed**: E3 is reached from the solved state by one continuous replay of 39,392 admissible twists (CERT-HIST). |

## 1. Setting and notation

- Pieces and slots. The model has 177,120 pieces; each piece p has a home chamber A_p, and the chambers are the
  slots. A piece is *on the lattice* when its pose is in K+ (the rotation group of order 7,200 that preserves the
  cut arrangement); it then fills one chamber. A *hole* is a slot filled by no lattice piece. Off pieces are the
  others. In every orbit the number of holes equals the number of off pieces.
- Caps and admissibility. Grip c has a cap (the closed half-space beyond its cut). A posed region is *in* cap c,
  *out* of it, or *straddles* it. A twist at c is admissible when no posed region straddles cap c; it rotates the
  regions in the cap. Lattice regions never straddle (cuts are unions of chamber faces), so only off pieces can
  block a grip. A grip is *blocked* in a state when some off piece straddles its cap, and *clean* when no off piece
  is in or straddles it.
- Frame. gamma is the first jumble element at grip 0 (menu item 12). A piece is *depth one* when its pose is
  X = K_k^-1 gamma K_r with K_k, K_r in K+; its region is then K_k^-1 gamma A_s with s = r(p), its *position*.
  T = K+ ∩ gamma K+ gamma^-1; S' = gamma^-1 T gamma has order 360 and acts on slots; for tau in T write
  sigma = gamma^-1 tau gamma. The *position class* of p is the S'-orbit of s. Two depth-one off regions lie in one
  K+-orbit exactly when their positions lie in one class. There are 500 classes: 488 of size 360 and 12 of size 120
  (CERT-STRUCT).
- Class names. Each class carries the model orbit `orbit_id` (0..34 of `assets/model.npz`, -1 for the centres) of
  its slots, and a coarser *registry orbit* (0, 1, 2|5a, 2|5b, 3, 4, 6), the name used by the earlier 3E registry,
  read from the caps of grip 0's block that contain gamma A_s. The registry orbit is a class label, not a model
  orbit. All orbit numbers in this document are model orbit ids.
- Seeds. A *seed* is a word W, admissible from solved, whose end state has exactly one off piece q (depth one) and
  one hole h. Its restoration word is W^-1. A *k-seed* is the same with k off pieces and k holes.

## 2. Restoration theorems

### 2.1 Lemma R (relative replay; proved)

Let Y be a state and w a word admissible from Y. Let Z be a state whose set of posed regions is contained in
Reg(Y) ∪ E, where E is the set of regions of some pieces of Z, and suppose every region of E is out of the cap of
every grip that occurs in w. Then w is admissible from Z; the pieces of E keep their exact poses; every other piece
of Z follows the trajectory of the piece of Y that occupies the same region.

*Proof.* Induction over the letters of w. Before a letter at grip c, the regions of Z outside E are regions of the
current Y-state, which does not straddle cap c because w is admissible from Y; the regions of E are out of cap c
by hypothesis. So the letter is admissible on Z, it rotates exactly the regions in cap c in both states, and E is
unchanged. The containment holds after the letter. QED

### 2.2 Theorem C (single seed; proved)

Let Z be a state with off set O, and p in O depth one with pose K_kp^-1 gamma K_rp and position s_p. Let a seed
(W; q, h) have q at pose K_kq^-1 gamma K_rq with position s_q, and let tau in T with sigma(s_q) = s_p. Put
m = K_kp^-1 tau K_kq (in K+) and w = m W^-1 m^-1 (letterwise conjugation, an I_a word by menu equivariance).
Suppose
(i) the slot m(h) is a hole of Z, and
(ii) every piece of O other than p is out of the cap of every grip of w.
Then w is admissible from Z, p ends on the lattice, every other off piece keeps its exact pose, and every lattice
piece of Z ends on the lattice.

*Proof.* Let Y' be the end of m W m^-1 from solved; by Lemma E (Card 5) it is the m-conjugate of the end of W:
one off piece m(q) at pose m X_q m^-1, whose region is m K_kq^-1 gamma A_sq = K_kp^-1 tau gamma A_sq =
K_kp^-1 gamma sigma A_sq = K_kp^-1 gamma A_sp, the region of p; and one hole m(h). By (i) every lattice region of
Z is a lattice region of Y', so Reg(Z) ⊆ Reg(Y') ∪ E with E the regions of O \ {p}; by (ii) Lemma R applies to
w = (m W m^-1)^-1, which is admissible from Y'. The pieces of E keep their poses. Piece p follows m(q), which ends
at home with the identity pose, so p ends with pose (m X_q m^-1)^-1 X_p = K_kp^-1 tau K_kq K_rq^-1 sigma^-1 K_rp,
a product of elements of K+. A lattice piece of Z with pose k_z follows a lattice piece of Y' with pose k_y, which
ends with the identity, so it ends with k_y^-1 k_z in K+. QED

The candidates of p are the pairs (seed of p's class, tau). The *needed vacancy* of a candidate is m(h). Condition
(ii) is decided per letter grip: a float test on exact vertices, with every status a decision depends on confirmed
by the exact evaluator (`sim.kernel.Evaluator`).

### 2.3 Lemma S (SAFE preparation; proved)

Call a retained letter SAFE in a state when the cap of its grip contains no off piece and is straddled by none. A
word of SAFE letters is admissible, keeps every off piece at its pose, keeps every lattice piece on the lattice,
and moves the holes by the slot permutation of its letters. (Lattice regions never straddle; a retained letter maps
the chambers of its cap onto chambers of its cap.) The pipeline prepares a needed vacancy by a breadth-first search
over SAFE words that brings a hole of the right orbit to it.

### 2.4 Theorem C' (sequential restoration; proved)

Let Z have off set O, all depth one, and choose one candidate for each p in O. Draw an edge p -> p' when p' is not
out of some cap of the word of p's candidate. If the digraph is acyclic and, whenever its turn comes, the needed
vacancy of a sink can be made a hole by a SAFE word (Lemma S), then restoring sinks one at a time brings every
piece of O to the lattice, each step keeping every remaining off piece at its exact pose.

*Proof.* A sink satisfies (ii) of Theorem C; after its SAFE preparation it also satisfies (i). The step changes no
remaining off pose, so the candidates and edges among the remaining pieces are unchanged: the digraph on them is the
induced subgraph, still acyclic, and it has a sink. Induction on |O|. QED

The preparation hypothesis is checked per instance (the pipeline records the preparation word). It is not proved in
general.

### 2.5 Theorem K (k-seed; proved) and Lemma G (glued blocks; proved)

**Theorem K.** Let a k-seed (W; q_1..q_k, h_1..h_k) and m in K+ be such that, for every i, K_kpi m K_kqi^-1 lies in
T and its sigma maps s_qi to s_pi, for distinct off pieces p_1..p_k of Z. If the k slots m(h_i) are holes of Z and
every other off piece is out of the cap of every grip of w = m W^-1 m^-1, then w brings p_1..p_k to the lattice,
keeps every other off pose and keeps the lattice on the lattice.

*Proof.* As for Theorem C: the test makes m map the region of q_i onto the region of p_i for every i, and Lemma R
applies with E the remaining off pieces. QED

**Lemma G.** Let Z have off set O. For p in O let Mov(p) be the set of grips that are not blocked in Z and whose
cap contains p. Let B ⊆ O with |B| >= 2 be such that every unblocked grip whose cap contains a piece of B contains
all of B (for example, B a set of pieces with equal Mov). Then for every word admissible from Z that keeps every
piece of O \ B at its pose, the first letter that moves a piece of B moves all of B.

*Proof.* Before that letter every piece of O is at its pose in Z (those of B have not moved, the others never
move). The letter is admissible, so no piece of O straddles its cap: its grip c is not blocked in Z. Its cap
contains the moving piece b at its pose in Z, so c is in Mov(b), and by hypothesis cap c contains all of B. QED

**Corollary G.** If B is glued (as in Lemma G), no piece of B satisfies (ii) of Theorem C for any seed, any tau
and any preparation, because the word must move that piece while the rest of B stays out of all its caps. In the
conflict digraph every candidate of a piece of B has edges to all other pieces of B, so B contains a cycle on
every choice of candidates. Changing seeds, adding seeds with other footprints, or borrowing a buffer vacancy cannot
break it. Pieces of B can only leave together at their first move, which is what a k-seed does.

### 2.6 The Block lemma (open) and what it would give

Partition the off pieces of Z into *blocks*: pieces with equal Mov. By Lemma G each block of size >= 2 is glued.

**Block lemma (open).** In every reachable state whose off pieces are all depth one, some block B admits a word,
admissible from the state, after which the pieces of B are on the lattice, every other off piece has its pose in
the state, and every lattice piece is on the lattice.

**Proposition B (proved).** The Block lemma implies that every reachable state whose off pieces are all depth one
can be brought to the lattice. (Induction on the number of off pieces: the step reaches a reachable state whose
off pieces are a subset of the old ones at their old poses, hence still depth one.) The lattice phase then
belongs to Card 5.

Evidence for the Block lemma, none of it general: along the E chain the history-specific words D17, D87 and D3
remove exactly the 17-block, then the 87-block, then the 3-block of X, each step leaving every other off piece at
its identical pose (CERT-CHAIN; CERT-HIST replays the words from solved); the three 3-seeds restore the 3-block of
E2, I1, I2 and E1 (Section 4). So X satisfies the Block lemma at every step of the chain.

## 3. The seed table

### 3.1 Structure (CERT-STRUCT)

The 600 grips form 5 grip classes of 120; every element of K+ maps classes onto classes (checked for all 7,200)
and K+ induces 60 permutations of them, transitively. Because the frame is fixed at grip 0 and every depth-one
pose is written in it, a seed made at any grip class serves all of them: the other four classes need no table of
their own. The natural index is the position class, not "grip class x registry orbit": one seed covers exactly
one position class (one K+-orbit of off regions), and a registry orbit splits into many classes.

| Registry orbit | Classes | Positions | Model orbits of its classes |
|---|---|---|---|
| 0 | 16 | 5,280 | -1 to 3, 5 |
| 1 | 131 | 46,200 | -1 to 16, 18 |
| 2\|5a | 86 | 30,960 | 5, 7 to 26 |
| 2\|5b | 86 | 30,960 | 5, 7 to 26 |
| 3 | 111 | 39,960 | 13, 15, 16, 18 to 33 |
| 4 | 31 | 10,440 | 26, 29 to 34 |
| 6 | 39 | 13,320 | 13, 15, 16, 18 to 33 |

In registry orbit 0, 2,400 positions meet the central core (float test with an exact-sign shortcut and a
separating-distance check, float evidence); 8 of its 16 classes are not core-forbidden. No piece of registry orbit
0 occurs in any input or sample.

### 3.2 Harvest by the vacancy formula (Lemma H, proved) and the reach bound (float evidence)

**Lemma H (single-letter vacancy transition).** Let U be admissible from solved, Q its off set and H its holes.
For h in H the *phantom* of h is the chamber region A_h carried through U^-1 (rotated with a cap that contains it,
left alone outside); h is *alive* if the phantom never straddles the cap of the letter being applied, and G(h) is
its end region. Let r = (g, a) be a retained letter whose cap neither contains nor is straddled by a piece of Q
after U, and put R = {h in H ∩ cap g : a^-1(h) not in H}, V = {h in H ∩ cap g : a(h) not in H}. If every h in R is
alive, then U r U^-1 is admissible from solved; the pieces that occupy the slots h in R after U r end at the
regions G(h); every other piece ends on the lattice (pieces of Q at home). If the pieces at the regions G(h) are off
the lattice, the holes at the end are the homes of the pieces that occupy the slots a(h), h in V, after U; then
|R| = |V|, and |R| = 1 gives a single whose off piece lies at G(h).

*Proof.* Track every slot through U^-1 from the state after U: an occupied slot along the trajectory of its piece,
a hole along its phantom; each trajectory is fixed letter by letter (rotate inside the cap, stay outside). After
U r, Q is where it was (outside cap g) and the slots in cap g hold the same chambers, now with the occupant of
a^-1(x) in slot x. As in Lemma R, a letter of U^-1 is admissible after U r provided no trajectory that carries a
real piece straddles its cap: these are the trajectories of real pieces of U U^-1 (admissible) and the phantoms of
the slots in R (alive). A real piece in slot x after U r follows the trajectory of slot x: if x is not a hole of U
it ends in the home chamber of the piece P(x) that sat in x after U, with a pose in K+ (a product of a, the pose of
P(x) after U and the pose of the moved piece); if x is in R it ends at G(x). The pieces of Q return home. When the
pieces at the G(x) are off the lattice, a chamber is therefore empty at the end exactly when it is the home of a
lattice piece whose slot after U is empty after U r, that is a slot a(h) with h in V. QED

Iterating Lemma H with labels (the hole P[a(h)] created from h keeps h's phantom, since its trajectory through
(U r U^-1)^-1 = U r^-1 U^-1 passes through slot h) gives nested conjugates whose single off piece lies at
G(label). This screens a candidate by its hole set alone, before any replay; the seeds of this stage were found
this way in minutes, and each is replayed exactly before use.

**Reach bound.** Labels at any level come from the level-0 sets R and V of clean letters, and R for (g, a) equals V
for (g, a^-1). So every single the harvest can produce from a base U0 has its class among the classes of G(h), h in
those V-sets. Over 323 bases (48 single jumbles at grip 0 and 31 commutator bases; 40 conjugated and paired bases;
48 single jumbles at a representative grip of each of the other four grip classes and 12 mixed bases) the union is
353 classes (registry 1: 20, 2|5a: 73, 2|5b: 81, 3: 111, 4: 31, 6: 37); each of the 204 bases of the last group
reaches exactly these 353, and no base reaches a class outside them. The phantom tracking is float
evidence (CERT-REACH). This bounds the harvest, not reachability: the fixture uses 131 classes outside it (111 of
registry 1, 13 of 2|5a, 5 of 2|5b, 2 of 6; 15,748 pieces).

### 3.3 Library and table (CERT-LIB, CERT-TABLE, CERT-KSEED)

- 252 seeds in 76 classes (registry 2|5a: 5, 2|5b: 4, 3: 26, 4: 31, 6: 10; model orbits 11, 12, 19, 22-34).
  The shortest seed per class has 19 to 1,407 letters (median 79). Seeds per class: 3 in 47 classes, 1 in 10,
  2 in 2, 4 in 7, 5 in 3, 6 in 1, 8 in 6. Distinct grip footprints per class: 1 in 54, 2 in 19, 3 in 2, 4 in 1.
  By Corollary G, footprint diversity does not help inside a glued block.
- Every seed was replayed from solved by an independent script: every twist admissible, exactly one off piece, the
  recorded class and the recorded hole (CERT-VERIFY: 252 checked, 0 failed).
- Three 3-seeds for the 3-block family (classes 468, 479, 562; model orbits 11, 12, 12; registry 2|5a): K3-I2inv
  (71 letters), K3-D3inv (79), K3-I1inv (89). They are the inverses of the earlier restorations of this block in I2,
  E2 and I1, replayed from solved (CERT-VERIFY: 3 checked, 0 failed). An input is never restored with its own
  inverse word (Section 4).
- `seed-table.json` has one row per class: size, registry orbit, model orbit, core status, membership in the reach
  bound, occurrence counts in the inputs and samples, number of seeds, shortest seed, footprints.

## 4. Measured coverage (CERT-RUN, CERT-CAPS, CERT-BLOCKS, CERT-VERIFY)

The pipeline `e3pipe.py` reads only the current pose table and the seed library. Every executed step asserts:
every twist admissible, the target pieces on the lattice, every other off piece at its exact pose, every lattice
piece still on the lattice. Blocks come from exact cap sets (`capsets.py`; float only to exclude a cap with margin
1e-6) and `blocks.py`.

| Input | Off | Result | Words | End digest | Blocking certificate |
|---|---|---|---|---|---|
| E3 | 0 | on the lattice | none | - | - |
| E2 | 3 | restored | prep 8 + K3-I2inv 71 | ad0cd5a8... | - |
| I1 | 3 | restored | prep 7 + K3-I2inv 71 | 87af376a... | - |
| I2 | 3 | restored without its own seed | prep 7 + K3-I1inv 89 | 19aa392a... | - |
| E1 | 90 | 3-block restored, 87 remain | prep 8 + K3-I2inv 71 | 0620e435... | 87-block glued (movers 314, 327, 346, 431, 432, 435, 464, 509); no 87-seed in the library |
| X | 107 | none restored | - | - | blocks 87, 17, 3, all glued; blocked grips 162, 190, 209, 250, 272, 307, 407, 490; all three 3-seeds meet the 17-block (17 pieces at grips 162, 183); no 17- or 87-seed |
| e2run24 | 690 | none restored | - | - | 13 glued blocks (sizes 3 to 111), no free piece; 497 pieces with no seed for their class; the 3-seed candidates rejected (conflict or preparation) |
| stop62 | 860 | none restored | - | - | 15 glued blocks (sizes 3 to 111), no free piece; 626 pieces with no seed for their class; the 3-seed candidates rejected (conflict or preparation) |
| I_a@40 fixture | 94,147 | classified | - | - | all depth one; 484 classes, 76 seeded (19,484 pieces); no conflict analysis (size) |

All off pieces of every input and sample are depth one. Every run that restored pieces was replayed again by the
independent verifier from the input table (digest checked), every recorded preparation and word applied, and the
end digest and off count compared (CERT-VERIFY). I2 was first restored with K3-I2inv, which is its own inverse
word (the earlier I2 restoration); that run is kept only as a record, and the result counted is the rerun without
that seed.

Reading of the table. Every input that the method does not finish is stopped by glued blocks, which Corollary G
shows to be structural, not a gap in the seed library. In X the block order is forced: the 3-block can leave only
after the 17-block, because each 3-seed word uses grips 162 and 183 whose caps hold 17-block pieces. In the two
deep samples (states on E3's own history, 690 and 860 off pieces) every off piece lies in a glued block of size at
least 3, so no single-seed step can apply to any piece, whatever seeds are added.

## 5. E3 full-history replay (CERT-HIST; condition R_E3 removed)

One continuous replay from the solved state, every twist checked admissible on the exact simulator, the digest
compared at the end of every segment:

| Segment | Twists | Off at end | End digest |
|---|---|---|---|
| I_a@40 fixture, first 40 records | 40 | 94,147 | b8918ba6... |
| seams stop | 62 | 860 | 9e70c4b9... |
| E2 run | 24 | 690 | bb72a668... |
| owner's word, prefix 36,563 | 36,563 | 107 | b318e80e... |
| u (X reachability) | 302 | 107 | f13c168e... (X) |
| D17 | 1,112 | 90 | 90399745... (E1) |
| D87 | 1,210 | 3 | 19e404d4... (E2) |
| D3 | 79 | 0 | fb199d99... (E3) |

Total 39,392 twists, all digests as recorded, 1,868 s. So X, E1, E2 and E3 are reachable from solved by one
replayed word, unconditionally, and statements that assumed R_E3 (for example nu(E3) in Card 5) hold without it.

## 6. Open items

- **Block lemma** (Section 2.6). This is the one gap between Theorems C', K and a restoration of every reachable
  depth-one state. It is not the statement "a conflict cycle can always be broken", which Lemma G refutes for single
  seeds.
- **Seed coverage.** 76 of 500 classes are seeded; 131 occurring classes lie outside the harvest's reach, so they
  need another seed source (other base words, or k-seeds). Whether every class admits a single at all is open.
- **Depth.** Every observed off piece is depth one (all inputs, both deep samples and the 94,147-piece fixture);
  deeper poses are not treated.
- **Preparation.** The needed-vacancy hypothesis of Theorem C' is checked per instance, not proved.
- **Fixture.** The conflict and block analysis of the 94,147-piece fixture was not run (the cap-set computation did
  not finish in the time given).
- **Review.** Lemmas R, S, H, G and Theorems C, C', K are new and have not been independently reviewed.

## 7. Certificates, data and scripts

All scripts run with `python3 -I` against the research tree at commit `27a001c` (`J` below), the grip classes
`data/classes.json` and the arrays `data/arrays.npz` (rebuilt by `e3lib.py` when absent). The frozen input tables
are not included; they are identified by SHA-256 (file) and simulator digest (state):

| Input | Table file SHA-256 | State digest |
|---|---|---|
| X | a802e5bb... (`Claude-E-closure-X1-20261012/input/X-table.json`) | f13c168e... |
| E1 | 18242d2c... (`Claude-E-closure-X1-20261012/D17/end-table.json`) | 90399745... |
| E2 | 649576e6... (`Claude-E-closure-E1E2-20261012/D87/E2-table.json`) | 19e404d4... |
| E3 | 53cec2e3... (`Claude-E-closure-E2E3-20261012/D3/E3-table.json`) | fb199d99... |
| I1 | c4b8a173... (`v27/inputs/I1-table.json`) | f27c345e... |
| I2 | d683496e... (`v27/inputs/I2-table.json`) | 989fd04c... |

Full digests are in `certs/inputs.json`. The deep-sample tables are included (`data/deep/`), exported from the
replay by `export_deep.py`.

| Certificate | File | Script |
|---|---|---|
| CERT-STRUCT | `certs/structure.json` | `structure.py` |
| CERT-REACH (float evidence) | `certs/base-reach.json`, `base-reach2.json`, `base-reach3.json` | `base_reach.py`, `base_reach2.py`, `base_reach3.py` (import `harvest.py`) |
| CERT-LIB | `data/seeds-lib.json` | `seeds.py`, `harvest.py` (merged, duplicates removed) |
| CERT-KSEED | `data/kseeds.json`, `kseeds-noI2.json` | `kseeds.py` |
| CERT-TABLE | `certs/seed-table.json` | `seed_table.py` |
| CERT-RUN | `runs/run-*.json`, `runs/*.log` | `e3pipe.py` (`classify_all.sh`, `run_deep.sh`) |
| CERT-CAPS | `certs/capsets-*.json` | `capsets.py` |
| CERT-BLOCKS | `certs/blocks-*.json` | `blocks.py` |
| CERT-CHAIN | `certs/chain-blocks.json` | `chain_blocks.py` |
| CERT-HIST | `certs/e3-history-replay.json` | `replay_e3_history.py` |
| CERT-VERIFY | `certs/verify-3e.json`, `verify-3e-I2noself.json` | `verify_3e.py` |

`README.zh-CN.md` gives the commands.
