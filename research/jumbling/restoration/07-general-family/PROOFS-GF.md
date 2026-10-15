# PROOFS-GF: one parametrized restoration family for every block class (menu I_a)

14 October 2026. Second package of the block-family stage (the first is `Claude-3E-block-family-20261014`). Linux
headless exact computation on synthetic geometry; research tree `research/jumbling` at commit `27a001c` (branch
`claude/jumbling-e3`). No Windows, DirectX, input, long-session or performance claim. The new proofs have not been
reviewed independently. **General E (restoration from every reachable state) is not claimed solved.**

Status labels: **proved** (proof in this file), **certificate** (exact computation with a record that
`scripts/` replays), **float evidence**, **conditional**, **open**.

## 0. Summary

- **One family, no templates.** Parameters: a grip c, a frame domain d inside cap c, and a letter X of the 12-letter
  coset that sends d to the lattice. One step (Lemma P, proved) covers every block class of the first package: one
  or several frames, two movers, one mover, frozen. Transport by shape is automatic (every quantity the searcher
  compares is K+-invariant). Section 2.
- **Block lemma (do parameters always exist?).** Along a history: yes, at every state (Theorem R, proved), but
  some steps of the reverse chain lose pieces (certificate: the inverse of fixture record 36 has gain -2,413).
  From the state alone, as a monotone step: no. Below 1,549 off pieces every letter raises the off count
  (Proposition N, proved), with reachable witnesses at 790 and 689 off. Sections 3, 4 and 7.
- **Searcher (heap peeling).** Picks the grip by gain and the letter by *sibling support*, a state-only rule that
  selects the exact inverse at all 17 steps of the fixture history where the letter matters (certificate, margin
  at least 186,978). Every written chain is correct (Theorem C, proved) and is checked by an independent replay.
  Section 5.
- **Measured.**
  - Four deep states (87,599 to 118,286 pieces off the lattice): the off count falls by 60.8% to 89.7%, and all
    157 steps pass the independent replay.
  - On the I_a@40 fixture, 84,947 of its 94,147 off pieces are on the lattice at the end. The template family of
    the first package reached 1,101.
  - Every block class gains: 73.8% to 93.4% of each class's pieces are on the lattice at the end.
  - S2 (5,404 off) is restored completely in two letters.
  - Every deep run ends at a stall where the best single-letter gain is negative. Section 6.
- **Go/no-go.** Go for the family and the searcher as a correct, replayable partial restorer. No-go for general E:
  two failure modes are documented (the low regime, proved; the trap, measured). Section 8.

## 1. Setting and notation

Notation follows `PROOFS-BF.md` Section 1 of the block-family package (`Claude-3E-block-family-20261014`).

- A *state* gives every piece p a pose g_p; its *region* is g_p A_p, with A_p the home chamber. A piece is *on the
  lattice* when g_p lies in K+ (order 7,200); it then fills one chamber. There are NP = 177,120 pieces and as many
  chambers; every cap is the union of 3,097 chambers.
- A letter (c, i) is the menu element E(c, i) of I_a at grip c (60 letters, 12 of them in K+, the *K+ letters*; the
  other 48 are *jumble letters*). It is admissible when no region straddles cap c, and then multiplies the pose of
  every piece whose region is inside cap c by E(c, i) on the left; nothing else moves (Proposition S of the
  restoration report).
- A *content* is the pair (frame domain, chamber) of a piece (`fastghost.Frames`): the region is reps[d](chamber q),
  domain 0 is the lattice. Contents are followed exactly through letters (signature bits when the pulled-back normal
  is a pole, the exact evaluator otherwise). Float arithmetic only selects which contents get an exact test.
- A *hole* is a chamber that holds no lattice piece. The *off count* off(Z) is the number of pieces off the lattice.
- A *history* of Z is an admissible word from the solved state to Z. The *gain* of a word applied at Z is
  off(Z) - off(Z after the word).

**Fact 1 (regions stay distinct; proved).** In every reachable state the regions of distinct pieces are distinct; so
lattice pieces fill distinct chambers and the number of holes equals off(Z).

*Proof.* At solved the regions are the distinct chambers. Let a letter at cap c be admissible. Inside regions are
moved by one rotation, which is injective; outside regions are fixed. An inside image lies in the closed half-space
of cap c and an outside region in the opposite closed half-space, so the two could only coincide inside the cut
hyperplane, which a region with interior cannot. Induction over a history. Lattice regions are chambers, so lattice
pieces fill distinct chambers, and holes = NP - lattice pieces = off(Z). QED

**Fact 2 (proved).** A K+ letter never changes whether a piece is on the lattice (E g in K+ iff g in K+ when E is in
K+). A jumble letter sends every lattice piece inside its cap off (E k in K+ with k in K+ would put E in K+), and puts
an off piece inside its cap on exactly when it sends that piece's frame domain to domain 0.

## 2. The step family: one letter at one cap

**Lemma P (single-letter cap step; proved).** Let (c, X) be an admissible jumble letter at Z. Let T be the off pieces
inside cap c whose frame X sends to domain 0, O the other off pieces inside cap c, and L the lattice pieces inside
cap c. After the letter: T is on the lattice; L is off (collateral); O stays off with new poses E(c, X) g_p; every
piece outside cap c keeps its pose. The gain is |T| - |L|.

*Proof.* Fact 2 and Proposition S. QED

Kinds: **PS** when L is empty (zero collateral; at a state with no straddler of cap c and no lattice piece in it), **PSC**
when L is not empty. A PS step is Lemma S of the S/SV runs generalized to caps that hold several frames: the
restored frame is one of them and the others are moved, not required to vanish.

**Transport by shape (proved; Lemma TT of the block-family package).** For m in K+, the letter (c, X) at Z and the
letter m(c, X) = (m(c), X') with E(m(c), X') = K_m E(c, X) K_m^-1 at the image state act alike: the same pieces up to
m are inside, straddle, restored, sent off and moved. Every quantity the solver of Section 5 compares (gains, rows of
caps, lattice-free overlapping caps) is K+-invariant, so its choices commute with K+ up to the order in which ties
are broken. One computation per K+ class of cap configuration serves all of its images; nothing is tabulated.

## 3. Existence along a history

**Theorem R (inverse of a jumble twist; proved).** Let Z = t(Z') with t = (c, g) a jumble twist admissible at Z'.
Then (c, g^-1) is admissible at Z and returns Z'. Its gain is |L_t| - |R_t|, where L_t are the lattice pieces of Z'
inside cap c and R_t the off pieces of Z' inside cap c that t put on the lattice.

*Proof.* E = E(c, g) fixes the normal of grip c, so it maps the closed half-spaces of cap c onto themselves and
preserves the side of every point exactly. At Z the regions inside cap c are the E-images of the regions inside cap c
at Z' (none of which straddled, t being admissible), and the regions outside are those of Z'. No region at Z
straddles cap c; (c, g^-1) moves exactly the inside regions back. The gain follows from Lemma P applied to t. QED

**Corollary R (proved).** For a history t_n ... t_1 of jumble twists, the reverse word t_1^-1 ... t_n^-1 is a chain of
single-letter steps from Z to solved, step j undoing t_j with gain |L_j| - |R_j|. A step of the reverse chain is PS
exactly when t_j put nothing on the lattice (R_j empty); a whole re-jumbled cap (L_j and R_j empty) gives a neutral
step, which merges with the next letter at the same grip into one letter.

So the parameters of the family (a grip, a frame, a letter of its coset) exist at every state of a history, and the
reverse chain's steps have positive gain whenever the twist sent more pieces off than it put on. The state alone
does not show which grip and which of the 12 letters undo the last twist; Section 5 chooses them from the state.

## 4. Non-existence: counting

**Proposition N (proved).** Let h = off(Z).
- (a) A cap holds no lattice piece only if its 3,097 chambers are all holes. If h < 3,097, every cap holds a lattice
  piece: no PS step, no S or SV step (Lemma S, Lemma V' with an S core), and no closure V W V^-1 whose core needs a
  lattice-free cap exists at Z.
- (b) Every admissible jumble letter at Z leads to a state with at least 3,097 - h off pieces. If h <= 1,548, every
  jumble letter raises the off count, and every word that brings Z to the lattice passes through a state with at
  least 3,097 - h > h off pieces (an *excursion*).

*Proof.* (a) Fact 1. (b) Let the letter act at cap c, with r pieces put on and l lattice pieces inside cap c. The cap
holds at most h holes, so l >= 3,097 - h; and r <= h. Off after = h - r + l >= 3,097 - r >= 3,097 - h. For
h <= 1,548 this exceeds h. K+ letters do not change the off count (Fact 2), so the first jumble letter of any word that
changes the off set reaches at least 3,097 - h. QED

**Witnesses (certificates).** The states below are reachable (each is the end of a chain replayed exactly from the
solved state through the W-J journal):
- S2: the first two twists of the W-J I_a walk (5,404 off). One SV step of the S/SV runner (V of one K+ letter, zero
  collateral, every other off piece unchanged) leads to Y (2,307 off): by (a), no lattice-free cap exists at Y; the
  S/SV/CX family has no parameter there.
- Y' (790 off): Y after its only positive-gain letter (a PSC step at grip 86, gain 1,517). By (b), no letter at Y'
  lowers the off count; the solver's scan confirms it (best gains -1,517 and -2,307).
- stop62 at 689 off (the E3 history state of the block-family package): by (b), every restoration from it first
  rises to at least 2,408 off pieces.
From S2 itself, the heap-peeling solver of Section 5 reaches the lattice in two letters (a PSC step with collateral
395, then a PS step of 3,097), avoiding Y. Counting is what separates the regimes: a state-only family that restores
across the heap can leave fewer holes than a cap needs, and from there no monotone continuation exists.

## 5. The searcher: heap peeling

`scripts/hp.py` with `scripts/hpcore.py`. Input: a pose table (any reachable state; no history). Output: a chain of
steps, each a word of one or two letters, recorded with its input and output digests and its exact restored,
collateral and moved sets.

**State and rows.** The state is held as contents (frame domain, chamber) of all 177,120 pieces. For each of the 600
caps the *row* gives the off contents inside the cap, the number of off contents straddling it, the number of lattice
pieces inside it, and the count of inside contents per frame. Inside, outside and straddle are decided exactly
(`fastghost.Frames`); float vertices only select which contents get the exact test (margin 1e-6: a content whose
every vertex lies more than 1e-6 outside the cut half-space is not tested).

**Candidates.** A row with no straddler and at least one inside content gives one candidate per frame d inside the
cap that some letter of grip c sends to domain 0: gain = (inside contents of frame d) - (lattice pieces inside the
cap) (Lemma P). The letters that send d to domain 0 form one coset of 12 letters (exact, `Geo.letters`).

**Sibling support (the letter rule).** The 12 letters of a coset restore the same pieces and send the same lattice
pieces off; they differ by a rotation k in the 12-element K+ stabilizer of the grip, applied to everything the letter
leaves off: the other frames inside the cap and the lattice pieces it sends off. The pieces one twist t' at cap c'
sends off from the lattice share one frame domain d, and their chambers in content coordinates fill one cap (t' =
reps[d] k'^-1 maps cap c' onto the cap k'(c')). Later twists move parts of such a group; the exact inverse of the
last of them returns those parts, and the right letter of an undo returns the pieces a twist had put on the lattice to
the group they came from. A wrong letter leaves them rotated about the grip. Sibling support measures this exactly:
for each content left off by the letter, the largest number of off contents of its new domain, outside cap c, whose
chambers lie in one cap together with its new chamber; summed over those contents (`hpcore.Siblings`). The letter
with the largest support is taken; ties (nothing is left off) go to hole alignment (restored pieces land where the
outside caps hold the fewest holes), then to the most lattice-free overlapping caps, then to the lowest index.

*Check against the history (certificate `certs/lettertest-fixture.jsonl`).* Along the reverse W-J history of the
I_a@40 fixture (Z_40, Z_39, ..., each state reached by the exact inverses), the rule selects the exact inverse,
uniquely, at all 17 steps where the choice matters (other frames inside the cap or lattice pieces sent off); the margin
to the next letter is at least 186,978. At 20 steps the letter leaves nothing off, so the choice changes no off
content. The inverses of 3 twists (records 12, 22 and 36) restore no frame present at that point: records 12 and 22
re-twist a grip twisted before (6 and 21), and record 36 put 2,413 pieces back on the lattice (its inverse has gain
-2,413; Corollary R).

**Screening (order).** A step that lowers the sibling support of the contents it moves and leaves off breaks up the
cap-shaped group of a later twist; such candidates are set aside while others exist (`screen`; at most 40 caps are
scanned, best gain first). Among the remaining candidates the LOOK = 3 of largest gain are evaluated one step ahead:
value = gain + max(0, best gain among the caps the step affects); the largest value is taken (ties: the candidate
order, i.e. larger gain, fewer lattice pieces, lower cap index, lower frame).

**Two-letter steps (only when no single letter has a positive gain).**
- *Carrier* (KPS, KPSC): a K+ letter (c1, k) at an unstraddled cap holding off contents, then the best single letter
  it makes available. A K+ letter changes no piece's on/off status (Fact 2); it rotates the contents of cap c1. The 11
  non-identity K+ letters of each cap are ranked by the gain in sibling support of the contents of cap c1, and the 8
  best are evaluated.
- *Excursion* (XPS, XPSC): a letter with gain <= 0 at one of the 12 best caps, then the best letter among the caps it
  affects, taken when the net gain is positive.

**Incremental rows.** After a letter at grip c only two kinds of rows can change: caps that share a chamber with cap c
(lattice occupancy changes only inside cap c) and caps that a moved content, before or after the letter, is not
clearly outside of. Only those rows are recomputed (`Table.affected`); every other row is unchanged because no
content and no lattice piece it depends on moved.

**Execution.** Each chosen word is applied on the simulator (the restoration-report simulator over the pose table).
The run requires that the simulator's off set after the step equals the exact virtual off set and that the off count
falls; it writes the step's word with its input and output digests and the exact sets (restored, collateral, moved)
recomputed from exact pose codes of the simulator. It stops when every piece is on the lattice or when no step of
the kinds above has a positive gain (`stuck`).

**Theorem C (correctness of a written chain; proved).** Every letter of every written step is admissible at the state
where it is applied; the effect of a step is the composition of Lemma P (jumble letters) and Fact 2 (K+ letters); the
off count falls strictly at every step; so a run writes at most off(start) steps and ends either with every piece on
the lattice or with a recorded `stuck` state.

*Proof.* A letter is applied only after the exact test found no off content straddling its cap; lattice pieces fill
chambers and caps are unions of chambers, so no lattice piece straddles. The simulator applies the same twists to the
same pose table and refuses a twist whose cap a region straddles; the run checks that the simulator's off set equals
the virtual one and that it shrank. The off count is a non-negative integer. QED

Completeness (that a run always reaches off 0) is **not** proved, and Proposition N shows that it fails for some
states. Section 6 measures it.

**Transport by shape.** Every quantity the searcher compares (rows, gains, sibling support, hole alignment, lattice-free
caps) is defined from exact contents and is invariant under K+ acting on the whole state (Lemma TT of the
block-family package); so the searcher's choices commute with K+ except where ties are broken by indices. No
templates or per-shape tables are needed.

**Independent replay (certificate).** `scripts/verify_hp.py` uses only the simulator, the start pose table and the
written words and sets: it checks every input and output digest, applies every twist (the simulator refuses a twist
whose cap a region straddles), recomputes restored, collateral and moved sets from exact pose codes and requires them
to equal the recorded sets, requires the off count to fall, and reports the final off count and whether every piece
is on the lattice. It imports none of the search scripts.

## 6. Measured results

### 6.1 Start states

Each is reachable by construction and replays from the solved state.

| Name | Construction | Off | Start digest |
|---|---|---:|---|
| fx | I_a@40 fixture: the first 40 records of the W-J I_a walk (`data/states/fixture40-table.json`) | 94,147 | `b8918ba6...` |
| r40s11 | 40 admissible jumble twists at uniformly random grips, seed 11 (`restore.scramble`; history `data/states/r40s11-hist.json`) | 87,599 | `7424775f...` |
| p60 | the first 60 records of the W-J walk | 105,676 | `ec3acb21...` |
| p100 | the first 100 records of the W-J walk | 118,286 | `6a1c666a...` |
| S2 | the first 2 records of the W-J walk | 5,404 | `63f73c8a...` |
| Y | S2 after one SV step (chain `runs/g2`) | 2,307 | `dffad170...` |

`scripts/hx/prefix.py` and `scripts/hx/scr.py` rebuild the tables.

### 6.2 The searcher of Section 5 (certificate: every chain replayed)

| Run | Start off | Steps | PS | PSC | Two-letter steps | Collateral (sum) | End off | Off count reduced | Replay |
|---|---:|---:|---:|---:|---|---:|---:|---:|---|
| fx | 94,147 | 36 | 13 | 23 | none | 9,397 | 10,415 | 88.9% | 36/36 ok |
| r40s11 | 87,599 | 34 | 13 | 20 | 1 (XPSC) | 11,784 | 9,033 | 89.7% | 34/34 ok |
| p60 | 105,676 | 44 | 13 | 29 | 2 (KPSC, XPSC) | 15,706 | 17,316 | 83.6% | 44/44 ok |
| p100 | 118,286 | 43 | 7 | 32 | 4 (2 KPSC, 2 XPSC) | 19,010 | 46,389 | 60.8% | 43/43 ok |
| S2 | 5,404 | 2 | 1 | 1 | none | 395 | 0 | 100% | 2/2 ok, every piece on the lattice |
| Y | 2,307 | 1 | 0 | 1 | none | 790 | 790 | 65.8% | 1/1 ok; then no letter lowers the off count (Proposition N(b)) |

Each deep run ends with `stuck`. The three best single-letter candidates at the end are listed as (gain, cap,
frame, restored |T|, lattice |L|):

| Run | Best | Second | Third |
|---|---|---|---|
| fx | -514, 497, 2, 1,154, 1,668 | -1,123, 117, 11, 790, 1,913 | -1,273, 497, 6, 395, 1,668 |
| r40s11 | -1,151, 218, 8, 819, 1,970 | -1,190, 119, 8, 1,340, 2,530 | -1,323, 110, 8, 592, 1,915 |
| p60 | -160, 497, 4, 1,271, 1,431 | -955, 117, 16, 790, 1,745 | -1,036, 497, 6, 395, 1,431 |
| p100 | -127, 497, 3, 1,179, 1,306 | -278, 117, 4, 994, 1,272 | -535, 110, 9, 819, 1,354 |

|T| + |L| can exceed 3,097: a cap is a half-space and pieces may protrude from the polytope (rigid jumbling), so off
regions inside a cap are not bounded by its chambers. The end tables are shipped (`runs/<run>/end-table.json`) as
test states for the next stage.

### 6.3 Earlier rule versions (run logs; chains not replayed and not shipped)

End off counts on the same start states, in the order the rules were tried (`scripts/hp/old/`, logs in
`runs/old/`):

| Version | Letter rule | Order | fx | r40s11 | p60 | p100 |
|---|---|---|---:|---:|---:|---:|
| 1 | most lattice-free overlapping caps (ties cut to 4) | gain, 1-step look-ahead | 13,821 | 13,793 | 21,340 | not finished |
| 2 | hole alignment | same | 16,822 | 17,815 | not run | not run |
| 3 | sibling support of moved contents; carriers, excursions | same | 15,750 | 10,681 | 20,633 | 49,971 |
| 4 | same | screening | 9,457 | 10,250 | 19,528 | 46,041 |
| final | sibling support of moved contents and collateral | screening | 10,415 | 9,033 | 17,316 | 46,389 |

The final rule was chosen for its history certificate (Section 5, all 17 relevant steps unique), not for its end
counts, which differ from version 4 by up to 10% in either direction. The stall depends on the order of the steps
more than on the letter rule (Section 8).

### 6.4 Comparison with the history (certificate: `certs/histmatch-*.json`)

| Run | Steps | Steps at a grip the history twisted | Letter = exact inverse of a history twist, or of a product of consecutive twists at that grip |
|---|---:|---:|---:|
| fx | 36 | 36 | 13 |
| r40s11 | 34 | 34 | 11 |
| p60 | 44 | 44 | 13 |
| p100 | 43 | 43 | 16 |

From the state alone, the searcher finds grips that the history twisted at every step. It takes them in gain order,
not in reverse history order, so at most steps the cap's contents differ from those right after the history twist,
and the letter differs from the exact inverse. At a PS step where all 12 letters leave nothing off, the letter only
decides where the restored pieces land on the lattice; that is a matter for the lattice phase.

### 6.5 Per-class coverage on the fixture (certificate: `certs/classcov-fx.json`)

Classes are those of the first package's block census at the start state. Movers are unstraddled grips whose cap
holds the whole block; frames are distinct frame domains among the block's pieces, computed exactly.

| Class | Blocks | Off pieces | On the lattice at the end | Blocks complete | Steps involved | Pieces sent off again by a later step |
|---|---:|---:|---:|---:|---:|---:|
| two or more movers, one frame | 88 | 18,476 | 16,299 (88.2%) | 75 | 29 | 1,316 |
| two or more movers, several frames | 8 | 2,352 | 1,735 (73.8%) | 4 | 7 | 0 |
| one mover, one frame | 32 | 38,576 | 35,413 (91.8%) | 19 | 28 | 3,965 |
| one mover, several frames | 11 | 15,023 | 13,073 (87.0%) | 6 | 15 | 486 |
| frozen (no mover), several frames | 1 | 19,720 | 18,427 (93.4%) | 0 | 11 | 1,501 |
| all | 140 | 94,147 | 84,947 (90.2%) | 104 | 36 | |

The end count 10,415 is 9,200 block pieces still off plus 1,215 pieces that were on the lattice at the start and are
collateral not yet returned. For comparison, the template family of the first package restored 7 of the 140 blocks
(1,101 pieces, 1.2%) and had no construction for several frames, one mover or frozen blocks.

### 6.6 The lattice-free-cap family (S, SV) on the fixture

The S/SV family needs a lattice-free cap: an S step is Lemma S, an SV step is Lemma V' with an S core. It was run
from the end of the first package's fixture run (93,046 off), with the run's own checks on the simulator and no
independent replay (`runs/sfx1`, `runs/gfx1`):
- 4 S steps restored 10,121 pieces;
- 3 SV steps (V of 3, 1 and 3 K+ letters) restored 7,602 more;
- every step had zero collateral and left every other off pose unchanged;
- then the clearing search (beam 4, depth 8) failed on every remaining cap, at 75,323 off.

From S2, one SV step reaches Y (2,307 off), where no cap is lattice-free (Proposition N(a)).

## 7. Answers to the two questions

**1. Do the parameters always exist (the Block lemma)?**

- *Along a history: yes (proved).* At every state of a history, the inverse of the last twist is a family step: a
  grip, a frame (the domain the twist sent its pieces to) and a letter of that frame's coset (Theorem R). The
  reverse chain reaches the solved state. Existence does not mean progress: a reverse step has gain
  |L_j| - |R_j|, which is negative when the twist put more pieces on the lattice than it sent off. Fixture record
  36's inverse has gain -2,413 (certificate `certs/lettertest-fixture.jsonl`, and `certs/history-fixture40.json`).
- *From the state alone, as a monotone step: no (proved).* By Proposition N(b), at h <= 1,548 off pieces every
  jumble letter raises the off count to at least 3,097 - h. The witnesses are Y' at 790 (best gain -1,517) and
  stop62 at 689 (from the first package). Lattice-free-cap families (S, SV, CX) have no parameter at all below
  3,097 off (Proposition N(a); witness Y at 2,307).
- *Measured on deep states.* A positive-gain parameter existed at every step of the four runs until the stall
  (9,033 to 46,389 off). At the stall none exists among single letters, carriers (8 best K+ letters per cap) or
  excursions (12 best caps).
- *Open.* Whether every reachable state admits a monotone two-letter or longer step above 1,548 off.

**2. Multi-frame and frozen blocks.**

- *No separate construction is needed (proved).* A cap that holds several frames is one row: one letter restores
  one frame and moves the others (Lemma P), and the others are restored by later steps. A frozen block has no
  unstraddled grip whose cap holds it. It becomes reachable when the steps that restore the pieces straddling its
  grips have run. Its pieces are then restored cap by cap, never as a whole block. Both are ordinary candidates of
  the searcher.
- *Measured (fixture).*
  - Several frames: 14,808 of 17,375 pieces (85.2%) are on the lattice at the end, and 10 of 19 blocks are
    complete.
  - The frozen block (19,720 pieces, 8 frames): 18,427 pieces (93.4%) are on the lattice, over 11 steps, but the
    block is not complete.
- *Not solved.* No class is complete. The remainder of every class sits in the trap of Section 8.

## 8. Failure modes and go/no-go

- **F1: Low regime (proved).** At h <= 1,548 off pieces, no monotone step exists, and any restoration first rises
  to at least 3,097 - h off. The searcher takes only monotone steps and two-letter steps with positive net gain,
  so it cannot pass this regime. Witnesses: Y' (790) and stop62@689.
- **F2: Trap (measured on all four deep runs; mechanism diagnosed on fx).**
  - *What the stall looks like.* A group T of one frame (395 to 1,340 pieces) lies inside a cap whose other
    content is mostly lattice (|L| = 1,272 to 2,530). The letter that restores T loses 127 to 1,323 pieces.
  - *How it forms.*
    1. A PSC step runs at a state that differs from the history state, because steps are taken in gain order.
       Its letter is not the history's exact inverse, so the lattice pieces it sends off land rotated about the
       grip.
    2. Later steps put pieces on the lattice in the chambers this group would return to.
    3. The group ends inside a cap that is mostly lattice.
  - *Recurrence.* The same caps (497, 117, 110) appear in the stall rows of the three W-J prefixes (fx, p60, p100).
- **F3: Histories that are not monotone (proved by example).** The reverse chain of a known history can contain
  negative steps (fixture record 36: -2,413). A searcher that only descends cannot follow such a history, even
  where one exists.
- **F4: Ties (by design).** When all 12 letters leave nothing off, the lowest index wins. This never changes the
  off count; it changes the lattice permutation that the lattice phase must undo.
- **F5: Completeness (open).** A run ends either on the lattice or at a recorded `stuck` state. Theorem C does not
  say which.

**Go/no-go.**
- **Go:** the family (Lemma P, Theorems R and C, transport by shape) with the searcher and the independent replay,
  as a correct partial restorer from any reachable state. Every written step is admissible and checked, and the
  off count falls at every step. It restores shallow states completely (S2) and lowers the off count by 61% to 90% on
  the deep states tested; on the fixture, 74% to 93% of every block class ends on the lattice.
- **No-go:** general E. It is not solved, and this package does not claim it.
  - The family has no monotone parameter in the low regime (proved).
  - The searcher stalls in the trap on every deep state tested.
- **What the next stage would need (not started):**
  - a non-monotone step with a certificate, measured against the shipped stall states (`runs/*/end-table.json`);
  - the lattice phase after off = 0.

## 9. Certificates and data

| File | What it certifies |
|---|---|
| `certs/verify-fx.jsonl`, `verify-r40s11.jsonl`, `verify-p60.jsonl`, `verify-p100.jsonl` | Independent replay of the four deep chains: every digest, every twist admissible on the simulator, restored, collateral and moved sets equal to the recorded sets, off count falling (all ok) |
| `certs/verify-t2.jsonl`, `verify-g2.jsonl`, `verify-y.jsonl` | The same for S2 to the lattice, S2 to Y, and Y to Y' |
| `certs/lettertest-fixture.jsonl` | The letter rule along the fixture's reverse history: rank of the exact inverse among its 12-letter coset at each of the 40 records (17 relevant, all unique first, minimum margin 186,978; 20 all-tie; records 12, 22, 36 restore no frame) |
| `certs/history-fixture40.json` | History anatomy of the fixture walk: for each record, the lattice pieces it sent off and the off pieces it put on |
| `certs/classcov-fx.json` | Per-class and per-block coverage of the fixture run |
| `certs/histmatch-fx.json`, `-r40s11`, `-p60`, `-p100` | Each step's grip and letter against the history |
| `certs/summary.json` | Per-run summary with the replay result |

End digests after the replay:

| Run | End digest |
|---|---|
| fx | `3e5edc6f...` |
| r40s11 | `8847aa81...` |
| p60 | `e831c99d...` |
| p100 | `8ce17605...` |
| S2 | `68360081...` |
| Y | `dffad170...` |
| Y' | `9fef7c1a...` |

The full digests are in the replay records, and `SHA256SUMS.txt` lists every file.

## 10. Checks not run

- Nothing on Windows, DirectX, input, long sessions or performance. All results are headless Linux exact
  computation on synthetic geometry; run times quoted in logs are not performance evidence.
- No independent review of the new proofs (Lemma P, Theorem R, Corollary R, Proposition N, Theorem C) or of the
  searcher: no Codex review and no proof-assistant check.
- No independent replay of the earlier rule versions (Section 6.3) or of the S/SV runs (Section 6.6); only their
  own simulator checks ran.
- The float preselection (margin 1e-6) has not been shown never to skip a content that the exact test would
  count. It cannot make a written chain wrong, because the simulator and the replay decide admissibility
  exactly. It can only hide candidates, so its effect is on completeness, not on correctness (float evidence).
- No test beyond these six states, no lattice phase, and no completeness argument (F5).
