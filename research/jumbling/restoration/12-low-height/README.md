# 12. Low heights: completeness of the off-lattice phase at left heights 2 and 4 (option B, 17 October 2026)

Status: in progress. The statement and the plan passed a plan check and a scoped re-check; experiments E-B1 to E-B4
are done, and E-B5 has a first bounded run (no step).

The owner chose options A and B on 17 October 2026, provisionally
([decision](../../../../docs/wiki/decisions/owner-decisions-2026-10-17-jumbling-tutorial.md), item 4). Option A is the
coverage census of [stage 11](../11-scramble-census/README.md). Option B, this stage, aims at what the census leaves
behind: pieces at left heights 2 and 4, in blocks that hold one another up. It should give the tutorial's tail a
finite, checkable set of techniques with explicit lengths.

Evidence kind throughout: exact synthetic geometry (Q(√5)) and source evidence; headless Linux. Nothing here is a
Windows, DirectX, input or performance result.

## 1. The statement (a conjecture; nothing in it is proved)

The setting and the terms are those of [`METHOD.md`](../09-general-method/METHOD.md) and
[`SENIOR-REVIEW.md`](../09-general-method/SENIOR-REVIEW.md): menu I_a; a piece's left height is the tree distance of
the left factor of its pose; 𝒞_h is the set of reachable configurations whose off pieces all have left height at most
h; a unit is a rigid block of off pieces, or one frame part of a multi-frame block.

**Conjecture B′(h)** (h = 2, then h = 4). There are an explicitly stated finite catalogue 𝒯 of step templates and
explicit functions L_h(n) and S_h(n) such that at every Z in 𝒞_h with n ≥ 1 off pieces, an explicit enumeration of at
most S_h(n) instances of 𝒯 at Z finds an admissible word of at most L_h(n) letters after which a nonempty unit and
every original lattice piece are on the lattice, and every other original off piece is either on the lattice or at its
exact original pose (a strict′ step, R21F-01). Strict′ steps keep 𝒞_h closed and may land several units at once, so
the off-lattice phase needs at most n·max_{1≤m≤n} L_h(m) letters and n·max_{1≤m≤n} S_h(m) instances.

- **Strict B(h)**, where every other off piece keeps its exact pose, is a separately tested strengthening; its first
  test is the falsifier E-B2. **Tame B(h)** is a further relaxation, not pursued unless B′ fails.
- 𝒞_h is finite, so some bound exists (T9-05); B′ asks for an explicit catalogue and bound. By Theorem D (round 21)
  a computable bound at every height exists exactly when reachability is decidable, so B′ is a statement about the
  low heights only.

## 2. The templates (obligations, not results)

- **T1, cell closure.** V·W·V⁻¹, with W from Theorem C (edge cell, at most 720 letters at the poles a, b and one
  common neighbour c), LC-12 (vertex cell in place, at most 104) or LC-13 (triangle or tetrahedron transport, at most
  240 or 192), carried by K⁺ where needed, and V a SAFE word of K⁺ letters that vacates Bad(W). LC-12 and LC-13 assume
  every class-P grip free; each instance is instead checked at Z for its actual free grips and live ghosts.
- **T2, partners.** A stated finite catalogue of creator/middle pairs (D, W) with their Bad vectors, each decided at Z
  by M1-16 and M1-17, with three explicit checks: the creator's source chambers are occupied, the protected off pieces
  are exact at the end, the partners are cleaned up inside the step. The known middle with Bad vector 2e covers edge
  contents only; triangle, tetrahedron and vertex contents need their own. That an adequate catalogue always exists is
  a joint-attack lemma, not an assumption. Positive control first: the s61c preparation of M1-14 from the s61c state.
- **T3, escape from blocking.** Three named obligations: (a) straddlers of a needed grip (Proposition F, conditional);
  (b) foreign-frame occupants of a needed cap, which the class-P alphabet and a SAFE preparation cannot move; (c) two
  off frame domains of one class in one cell, a proved obstruction to the fixed class alphabet (M1-19). Escape
  candidates: M1-18 retained conjugation (r = 1 first), LC-06 several edge cells, the M1-19 lifted-action test. A cycle
  among the supports of selected words is a catalogue obstruction only, not a counterexample.
- **T4, height 4.** An explicit itinerary with a stated endpoint contract: the lens conditions P1 and P2 (R21F-02), or
  two protected lowering segments 4 → 2 → 0 with a combined bound; every prefix admissible, every original lattice
  piece and every protected off piece checked at the end. Units with no admissible lowering cap (186 of 305 at the
  fixture end, E21-3) need a named bounded preparation, unlocking or joint move; AU-02's two-class necessity is not
  sufficiency.

The statement and this list passed a plan check and a scoped re-check (both by the senior reviewer, 17 October); the
re-check found the first check's five blocking findings resolved.

## 3. The experiments, in order

1. **E-B1, structural screen** (done at three states, section 4).
2. **E-B2, strict-step falsifier R21-02** (done at three states, section 5).
3. **E-B3**, the positive control for T2: the s61c preparation from the state alone (done, section 6).
4. **E-B4**, M1-18 with r = 1 at the units E-B1 finds blocked (done at the depth 60 stop, section 7).
5. **E-B5**, a bounded driver run from the depth 40 stop (explicit `--heuristic-seconds`, `--max-steps`,
   `--fallback-max-length`; no lattice phase; every accepted word replayed against the strict′ contract). Run 1
   made no step (section 8).
6. **E-B6**, height 4: the two blocks of the depth 100 residue that hold height-4 pieces, with the fixture end as a
   negative control.

## 4. E-B1: the structural screen (measured, exact geometry)

`scripts/screen.py` reads one state and, for every unit, finds:
- its frame class P and its class-P cell (the sign vector over the 120 caps of P, LC-01), the cell kind, whether the
  unit is the whole content of its cell (content: the off pieces with the same cell and the same domain, LC-01), and
  other frame domains of class P in the cell (M1-19);
- for every grip a T1 word needs, whether it is free (admissible, and its cap holds only lattice pieces and pieces of
  P's frames) or blocked by a straddler or a foreign-frame occupant, and by which units. The needed grips are the
  minimal supports: an edge cell {a, b} needs a, b and one of their five common neighbours (Theorem C); a vertex cell
  {a} needs a; a triangle or tetrahedron cell needs one of its poles to move at all. A unit whose needed grips are
  all free, in a cell without a second frame domain of its class, is called ready;
- the T1 checks. A T1 word that lands a unit turns its whole cell, so Bad(W) contains the cell's home chambers (the
  lattice chambers inside the cell: 86 for an edge cell, 385 for a vertex cell, 111 or 87 for the triangle and
  tetrahedron cells that have any). For those chambers the screen checks Lemma D's per-type counts, and N1 and N2 for
  vacating them with SAFE-ALL letters (the letters of the caps that hold no off piece and are not straddled;
  `buffer.condition`). A triangle or tetrahedron content lands only by a transport into a home cell of its kind
  (LC-13), whose chambers are then in Bad(W) too, so the screen also counts the home cells of its kind and class that
  pass the same checks. A unit passes the T1 checks when its own home chambers pass (if it has any) and, for a
  transport, at least one target passes. All of these are necessary conditions only, evaluated at the state.

`scripts/order.py` then lands units in rounds under a structural model: a unit is landable once the units blocking
its needed grips have been landed, and never when its cell holds a second frame domain of its class. No word is run,
and the T1 checks are not re-evaluated along the way: the order is a structural bound, not a result.

| State | Off | Units (edge, triangle, tetra, vertex) | Ready | In a home cell (Lemma D; N1 and N2) | Transports with a target | T1 checks pass | Ready and pass (pieces) | Landable in order (pieces) | Left |
|---|---|---|---|---|---|---|---|---|---|
| stop62@233 | 233 | 3 (3, 0, 0, 0) | 3 | 3 (3; 3) | none | 3 | 3 (233) | 3 (233), one round | 0 |
| depth 40 stop | 1,193 | 22 (5, 12, 4, 1) | 21 | 8 (7; 1) | 16 of 16 | 15 | 14 (857) | 22 (1,193), rounds 21, 1 | 0 |
| depth 60 stop | 14,610 | 184 (71, 81, 27, 5) | 118 | 88 (88; 62) | 108 of 108 | 158 | 107 (9,111) | 132 (10,274), rounds 118, 11, 3 | 52 (4,336) |

The depth 40 stop is the end of round 2 of the census run with seed 7101, the depth 60 stop the end of round 2 with
seed 7104 (stage 11; both all at height 2). stop62@233 is the stage 08 state with three carriers.

What the screen shows:
1. **Every unit is the whole content of one class cell.** At all 209 units of the three states the pieces of a unit
   share one sign vector over the caps of their class (LC-14), no piece straddles a cut of its own class, the
   positive caps form a face of the class's 600-cell, and no other piece of the same domain lies in that cell. Every
   unit is at height 2. Most of the residue is in triangle and edge cells (depth 60: 7,332 and 4,756 of the 14,610
   pieces; tetrahedron 1,725, vertex 797).
2. **Positive control.** At stop62@233 the three units come out as recorded in stage 08: 1040 (86 pieces, class 1,
   edge cell {5, 11}), 64184 (86, class 0, {369, 507}) and 23955 (61, class 0, {30, 145}), each ready and passing
   Lemma D, N1 and N2. That the stage 08 runs still needed a 1,749-letter step there shows what the screen does not
   decide: whether the vacating word V exists (N1 and N2 are necessary only) and what it costs.
3. **Lemma D rarely binds; N1 often does.** Lemma D fails at one unit only, the 10-piece vertex content at the depth
   40 stop: the 385 home chambers of a vertex cell include chambers of types 0 to 5, and no piece of those types is off
   at that state. N1 fails where a home chamber of the unit's cell is occupied and lies in no SAFE-ALL cap (every cap
   that contains it holds an off piece or is straddled), so no SAFE-ALL word can vacate it at the state. That happens
   at 7 of the 8 home-cell units at the depth 40 stop (four edge contents, two triangle contents and the vertex
   content; 250 pieces) and at 26 of the 88 at the depth 60 stop (20 edge, 3 triangle, 3 vertex; 1,023 pieces), but
   at none of the three at stop62@233. Landing other units frees caps, so N1 may hold later; the screen does not
   follow that.
4. **Vacant home cells and transport targets.** At the depth 60 stop 57 of the 88 home-cell units have no lattice
   piece in their cell's home chambers, so V is empty for their own cell (at the depth 40 stop 1 of 8, at stop62@233
   none). Every triangle and tetrahedron content has home cells of its kind and class that pass Lemma D, N1 and N2:
   478 of 480 triangle cells and all 120 tetrahedron cells at the depth 40 stop, 406 to 416 of 480 and 106 to 113 of
   120 at the depth 60 stop.
5. **Blocking.** At the depth 60 stop 66 units are not ready: 60 have a needed grip that is not free (the grip is
   straddled, or its cap holds pieces of another class's frame), and 6 lie in three cells that hold two frame domains
   of one class (M1-19), so no T1 word can target them. Landing in rounds clears 14 of the 60; 52 units (4,336
   pieces) are left:
   - the 6 units in M1-19 cells (320 pieces);
   - 40 units (3,541 pieces) in six groups that block one another (strongly connected under "A blocks a grip that B
     needs", among the units left): of 2, 3, 3, 4, 11 and 17 units; 36 of the 40 are edge contents, and 14 are frozen
     blocks;
   - 6 units (475 pieces) only behind one unit of the 11-unit group.

   Of the 132 units landable in order, 116 pass the T1 checks at the state; 10 of the 52 left fail them. By the plan
   the groups are catalogue obstructions for T1 with its minimal supports, not counterexamples: the T3 escapes
   (M1-18, LC-06, the M1-19 lifted-action test) are their next test. If the 132 units could be landed by strict
   steps, the 52 would keep their exact poses and their blocking would not change (landed pieces block no grip). That
   would give a reachable state at which every unit has a needed grip blocked by another unit or lies in an M1-19
   cell, and so answer the open part of AU-04 (`09-general-method/SENIOR-REVIEW.md`: is there always an eligible
   unit whose grips are all free?) in the negative for T1 with its minimal supports. That state has not been
   constructed, and strict steps for the 132 units are not known to exist.

## 5. E-B2: the strict-step falsifier R21-02 (measured, exact)

R21-02: for a unit U put P = off(Z) \ U, and let F_t(P) be the lattice chambers of type t whose interior meets the
interior of a piece of P; a strict step for U needs |F_t(P)| ≤ |P_t| for every t. `scripts/r2102.py` bounds F(P) from
both sides, exactly: a piece whose region lies beyond one facet plane of the body meets no chamber; for the other
pieces, sampled interior points (24 per piece) inside the body and on no cut give exact overlap certificates (a lower
bound), and the chambers whose signature agrees with the piece on every cut that does not straddle it give an upper
bound.

| State | Off pieces beyond one facet | Samples of the others | Largest F(P): certified, possible | Units where R21-02 holds for certain | Refuted |
|---|---|---|---|---|---|
| stop62@233 | 224 of 233 | 216, all outside the body | 0, 10 | 3 of 3 | 0 |
| depth 40 stop | 1,188 of 1,193 | 120: 117 outside, 3 in a chamber | 2, 8 | 22 of 22 | 0 |
| depth 60 stop | 14,083 of 14,610 | 12,648: 9,259 outside, 3,389 in a chamber | 185, 905 | 184 of 184 | 0 |

The rotated regions of the off pieces stick out of the body almost entirely, as Lemma O found at stop62@245
([`PROOFS-U.md`](../08-units-and-clean-steps/PROOFS-U.md)), so they meet few chambers: for every unit at the depth 60 stop at most 905
chambers against at least 14,225 other off pieces, type by type within |P_t|. R21-02 holds with room to spare, and
the falsifier cannot refute strict steps at these states; strict B stays open, neither refuted nor supported. The
three runs took about 6, 25 and 270 seconds.

## 6. E-B3: the positive control for T2 at s61c (measured, exact replay)

T2 asks for a stated catalogue of creator and middle words whose instances are decided at the state alone. Its
positive control is the s61c preparation of M1-14 (`09-general-method/SENIOR-REVIEW.md`), not run until now. s61c is
the stop62 state with one off unit, the carrier 23955: 61 pieces in one class-0 frame domain at the edge cell
{30, 145} (M1-04). The stage 08 step s62m3 reaches it from stop62@233. The template has three phases. Each core is a
word of the stop62 chain, so the catalogue has one entry per phase. Every vacating word is read from the holes of the
state it acts on.

| Phase | Word | Letters | Off after | Source |
|---|---|---|---|---|
| preparation | V0 | 1 | 61 | state: `vacate.Vacate` with the roles of holes and pieces exchanged moves the 6 holes that lie in Bad(D) out of it with one letter, (369, 1) |
| creator | D = W_C⁻¹·P_C⁻¹ | 51 | 233 | catalogue: the inverted middle (46 letters) and prefix (5) of the stage 08 step s62m3 = P_C·V_C·W_C·V_C⁻¹ |
| middle | P_S | 4 | 233 | catalogue: the prefix of the stage 08 step s62m = P_S·V_S·W_S·V_S⁻¹ |
| | V1 | 429 | 233 | state: the peeling certificate of Theorem B for Bad(W_S) (172 chambers, 68 occupied): 162 of 162 movable chambers peeled, family bound 1,955 |
| | W_S | 46 | 86 | catalogue: the middle of s62m |
| | V1⁻¹ | 429 | 86 | |
| last | V2 | 563 | 86 | state: the peeling certificate for Bad(W_R), the 86 home chambers of the carrier's cell (58 occupied): 81 of 81 peeled, family bound 974 |
| | W_R | 288 | 0 | catalogue: the class-cell pure twist inside step 14 of the stop62 chain (its letters 543 to 830) |
| | V2⁻¹ | 563 | 0 | |
| total | | 2,374 | 0 | |

`scripts/eb3_replay.py` replays the whole word from the s61c table on a fresh simulator. It checks every twist for
admissibility and the digests at the boundaries of the parts. The checks:
- the creator's source chambers are occupied when it runs: after V0 no hole lies in Bad(D) (172 chambers);
- V0·D keeps the carrier at its exact pose and takes exactly the 172 lattice pieces of Bad(D) off; the off regions
  after it equal those of stop62@233;
- N1 and N2 hold at both vacating states, and both certificates have no deficit, so the greedy fallback was not
  used;
- the partners are cleaned up inside the step: no piece is off the lattice at the end.

So s61c reaches the lattice from the state alone, with these catalogue cores, in 2,374 letters; the last piece lands
with W_R, after 1,811 letters. The history certificate M1-05 takes 4,402 letters, ending with W_R as well. Its
creator is the whole inverted step s62m3 (1,749 letters), whose vacating part V0 replaces with one letter; its two
vacating words are 886 and 543 letters long, against 429 and 563 here. As in M1-05, the end is on the lattice but the
stickers are not solved: the lattice phase follows.

The control passes for one state, with cores taken from that state's own history. It shows that the vacating phases
of the template can be read from the state with exact certificates once the cores are given. It does not show that a
finite catalogue covers every partner situation at low heights; that remains the joint-attack lemma of T2.

## 7. E-B4: one retained letter at the blocked units (M1-18 with r = 1; measured, exact replay)

E-B1 left 52 units (4,336 pieces) at the depth 60 stop Z that T1 with its minimal supports cannot land in any order:
a needed grip is blocked by another left unit, or the unit's cell holds a second frame domain of its class (M1-19).
M1-18 is the escape to try first: if C is a clean step at Y = A(Z) for the unit A(U), then A·C·A⁻¹ is a clean step at
Z for U. With r = 1, A is one retained letter, a non-identity K⁺ item at an admissible grip (11 per grip). It turns
the content of its cap by a symmetry E of the 600-cell that permutes the 600 caps, so the cut labels at Y follow from
those at Z and the cap permutation of E, with no new classification; a moved piece of frame domain R K⁺ lies in
E R K⁺ at Y, and E permutes the five classes as blocks (checked). `scripts/eb4.py` runs the first two stages,
`scripts/eb4_close.py` the third:

1. **Structural, at all 52 units.** The candidate grips are the admissible caps that wholly hold the unit, one of its
   blockers, or another frame domain of its cell. A letter passes when A(U) is ready at Y in the sense of E-B1: every
   grip it needs is free, and no second frame domain of its class shares its cell (the frame domains at Y identified
   exactly as cosets). Two variants: at Z every off piece counts; at Z′ only the 52 left units count, as they would
   after the 132 landable units had landed by strict steps (structural only, as in E-B1).
2. **N1 and N2 at Y**, for the first passing letter of each unit at Z, with the SAFE-ALL alphabet of Y, for A(U)'s home
   chambers and, for a triangle or tetrahedron content, for the home cells of its kind and class.
3. **Closures**, for the edge contents that pass N1 and N2: at Y the class-cell pure twist W of A(U)'s cell
   (`scripts/cellpure4.py`), then V·W·V⁻¹ by the driver's `tclose2.py` (V from the peeling certificate of Theorem B,
   greedy fallback), and by `tclose2m3.py` on 'no V'. The step A·C·A⁻¹ is replayed at Z on a fresh simulator, every
   twist checked admissible, against the strict′ contract: U lands, no lattice piece of Z leaves the lattice, and
   every other off piece of Z ends on the lattice or at its exact pose.

`cellpure4.py` is the driver's `cellpure3.py` with two changes, both marked in the file. At a deep state a frame
domain can have contents in several cells, so the unit's content is found by its pieces (cellpure3 asked the domain
to have one content). And the class-cell model compares content points (cell, label) only, so another off content
can come back to its point at another exact pose: cellpure4 replays the model-passing solutions exactly, shortest
first (at most 12), and takes the first that lands the unit, keeps the exact pose of every other off piece and takes
off only the lattice pieces of the cell. That filter decided 3 of the 20 closures: at the units 2, 105 and 122 the
shortest solutions left 14, 15 and 87 pieces of other contents at their points but at other poses, and the third or
fourth solution kept them all.

| Stage | Units (pieces) | Notes |
|---|---|---|
| left by the landing order | 52 (4,336) | 46 edge, 4 triangle, 2 vertex contents; six blocking groups of 2, 3, 3, 4, 11 and 17 units, 6 units in M1-19 cells, 6 behind one group |
| 1. ready after one letter, at Z | 34 (2,417) | 31 edge, 3 triangle; 322 of 2,310 letters tried pass; every blocking group and every M1-19 cell has a unit among them (groups: 2 of 2, 1 of 3, 2 of 3, 3 of 4, 3 of 11, 11 of 17; M1-19: 6 of 6; behind: 6 of 6) |
| 1. ready after one letter, at Z′ | 42 (3,185) | structural only |
| 2. N1 and N2 at Y (first letter) | 20 (1,593) | all edge; 11 edge contents fail N1 at their first letter (an occupied home chamber that no SAFE-ALL letter reaches at Y); the 3 triangle contents need a transport: no home chambers in their cell, and 402 to 413 of the 480 home cells of their kind pass Lemma D, N1 and N2 |
| 3. strict′ step A·C·A⁻¹ at Z | 20 (1,593) | 110 to 506 letters (110: 4 units, 326: 7, 362: 1, 434: 7, 506: 1); V empty in all 20, so C = W |

All 20 closures succeeded: in every case A(U)'s cell held no lattice piece at Y, so Bad(W) was vacant and C = W, a
class-cell pure twist of 108 to 504 letters (commutator powers around three edge cells, once four). Each step lands
its unit with collateral 0 and leaves all 14,524 (or 14,565 and 14,569) other off pieces at their exact poses. The run
took 77 minutes with two closures at a time. `scripts/eb4_replay.py` replays the 20 committed words independently
from the exported table, and all 20 meet the strict′ contract (`runs/eb4-replay-k60.json`).

What E-B4 shows, at one state:
- **One retained letter breaks every group.** The six blocking groups and the three M1-19 cells of the depth 60 stop
  each have a unit that one retained letter makes ready; in three of the six M1-19 units the letter moves the other
  frame domain out of the cell rather than the unit. For 20 units the escape is a checked strict′ step from the
  state alone, built with the driver's own tools: at this state M1-18 with r = 1 is a working template for T3, with
  at most 2 + 720 letters when V is empty (Theorem C) and 11 letters per candidate grip to enumerate. At Z′, the state
  that section 4 describes for AU-04 (not constructed), 42 of the 52 units become ready after one letter.
- **What it does not show.** 32 of the 52 units have no step yet: 18 have no single letter that makes them ready (15
  edge, 2 vertex, 1 triangle), 11 fail N1 at their first letter (their other passing letters were not tried), and 3
  triangle contents need a transport closure (LC-13), which the toolkit does not build. Each step starts at Z: that
  they compose is not tested (landing a unit can only remove blockers, since landed pieces block no grip, but it can
  change N1). Z′ is structural only.

## 8. E-B5: a bounded driver run from the depth 40 stop (run 1: no step; measured)

The driver of stage 09 (`09-general-method/scripts/gm/gdrive.py`, the method M* with experiment bounds) ran once from
the depth 40 stop (1,193 off pieces, 22 units; table digest `7e128820…`) with `--par 2 --heuristic-seconds 1800
--max-steps 30 --fallback-max-length 1`, no seed library and no lattice phase. It made no step in 59 minutes
(`runs/eb5-k40-run1.jsonl`):

1. **Driver step** (`cbrun3.py`, one step). With no seed library every unit needed a fresh search, and the step was
   killed at the 30-minute bound before it had tried a candidate.
2. **Pure-twist step** (`cellpure3.py` on the 8 largest units; `--pure-max 8` dropped the other 14). All 8 stopped
   at a precondition: their frame domains have 2, 4 or 13 contents in the class-cell model, and `cellpure3.py` asks
   for one, the restriction that `cellpure4.py` lifts (section 7). Six of the eight are triangle contents and one is
   a tetrahedron content, which land only by a transport (LC-13); the eighth, the 86-piece edge content, is not
   ready. The four ready edge contents (61, 45, 41 and 25 pieces) were among the 14 dropped.
3. **Fallback** for the largest unit (a 111-piece triangle content): no single letter of the I_a menu is a clean
   step for it (34,124 letters tried in 22 minutes), and the length bound 1, an experiment bound outside M*, ended
   the run.

What run 1 shows: as it stands, the driver cannot start its pure-twist step at a state as deep as this one, and its
other two steps found nothing within these bounds; the census driver of stage 11 had also found nothing after 49
minutes of search at this state. It says nothing about whether steps exist. The screen (section 4) explains why
`cellpure4.py` alone would not be enough here: no edge content at this state is both ready and passing N1 (the four
ready ones fail N1, the fifth is not ready), and the 14 units that are ready and pass the T1 checks are triangle and
tetrahedron contents, which need a transport.

## 9. Next

- An LC-13 transport closure, for the triangle and tetrahedron contents: it is what E-B5 needs first at the depth 40
  stop (16 of its 22 units) and what three of the E-B4 leftovers need. Then E-B5 run 2: the driver with
  `cellpure4.py` as its pure-twist script (a new option whose default stays `cellpure3.py`, so earlier runs replay
  unchanged), every unit tried, a seed library and a longer driver-step bound.
- E-B6, height 4.
- For the other E-B4 leftovers: the other passing letters of the 11 units that fail N1; r = 2 or several edge cells
  (LC-06) for the 18 units without a single letter.
- E-B2 at height-2 fixture prefixes is cheap but, where the off pieces stick out of the body, has no teeth.

## Replay

From the repository root, with Python 3 and NumPy, `S=research/jumbling/restoration/12-low-height/scripts`,
`C=research/jumbling/restoration/11-scramble-census/runs/round2`, `Z=research/jumbling/restoration/08-units-and-clean-steps/certs/stop62/z233/inputs/stop62-233-table.json.gz`:

    python3 -I -B $S/screen.py table $Z <out>                      # about 25 seconds
    python3 -I -B $S/screen.py record $C/ia-k40-s7101.json <out>   # about 45 seconds
    python3 -I -B $S/screen.py record $C/ia-k60-s7104.json <out>   # about 4 minutes
    python3 -I -B $S/order.py <screen out>
    python3 -I -B $S/r2102.py table $Z <out>                       # about 6 seconds
    python3 -I -B $S/r2102.py record $C/ia-k40-s7101.json <out>    # about 25 seconds
    python3 -I -B $S/r2102.py record $C/ia-k60-s7104.json <out>    # about 5 minutes

E-B3 reads two tables unpacked with `gunzip -c` into `s61c.json` and `s233.json`, from
`$U/certs/stop62/cm233/inputs/stop62-61-c-table.json.gz` and `$U/certs/stop62/z233/inputs/stop62-233-table.json.gz`,
with

    U=research/jumbling/restoration/08-units-and-clean-steps
    W=research/jumbling/restoration/09-general-method/earlier/stop62-860-to-0/chain/step014-word.json
    R=research/jumbling/restoration/12-low-height/runs
    T=research/jumbling/theory
    python3 -I -B $S/eb3_prep.py $T s61c.json $U/chains/s62m3/step001-word.json s233.json p    # about 30 seconds
    python3 -I -B $S/eb3_finish.py $T p-table.json $U/chains/s62m/step001-word.json $W f       # about 10 minutes
    python3 -I -B $S/eb3_replay.py $T s61c.json $R/eb3-prep-word.json $R/eb3-finish-word.json <out>   # about 2.5 minutes

The first two write `p.json`, `p-word.json`, `p-table.json` (the state Z1, not kept) and `f.json`, `f-word.json`.

E-B4, with `C` and `R` as above (`k60.json` is the depth 60 stop as a pose table, digest `ef45a117…`):

    python3 -I -B $S/export_table.py $C/ia-k60-s7104.json k60.json                # about 4 minutes
    python3 -I -B $S/eb4.py $C/ia-k60-s7104.json $R/screen-k60.json <out>         # about 31 minutes
    python3 -I -B $S/eb4_close.py k60.json $R/eb4-k60.json <work dir> --par 2     # about 77 minutes
    python3 -I -B $S/eb4_replay.py k60.json $R/eb4-k60-steps <out>               # about 10 minutes

`eb4_close.py` writes `close.json` and, per unit k, a directory `u<k>/` with the tables, logs and words of its stages;
`step-word.json` there is the step at Z.

E-B5 run 1, with `C` as above and `G=research/jumbling/restoration/09-general-method/scripts` (`k40.json` is the
depth 40 stop as a pose table, digest `7e128820…`):

    python3 -I -B $S/export_table.py $C/ia-k40-s7101.json k40.json                # about 20 seconds
    python3 -I $G/gm/gdrive.py research/jumbling $G/echain k40.json eb5-k40 <work dir> \
        --par 2 --heuristic-seconds 1800 --max-steps 30 --fallback-max-length 1     # about 1 hour

The driver writes its record to `<work dir>/gdrive.jsonl`; `runs/eb5-k40-run1.jsonl` is that record with the private
directory of the start table written as `<working tree>`, as in the stage 09 run records.

## Files

| File | What it is |
|---|---|
| `scripts/screen.py` | E-B1: units, classes, cells, needed grips, Lemma D, N1 and N2 at one state |
| `scripts/order.py` | The structural landing order and the units left over, from a screen output |
| `scripts/r2102.py` | E-B2: exact two-sided bounds for R21-02 at one state |
| `runs/screen-*.json`, `runs/order-*.json`, `runs/r2102-*.json` | The outputs at the three states (`233`, `k40`, `k60`); a rerun reproduces them except the timing fields |
| `scripts/eb3_prep.py`, `scripts/eb3_finish.py`, `scripts/eb3_replay.py` | E-B3: the preparation V0 and creator D; the middle and last phases with their vacating words; the independent replay |
| `runs/eb3-prep*.json`, `runs/eb3-finish*.json`, `runs/eb3-replay.json` | E-B3 records and words (52 and 2,322 letters) with input and output digests, and the replay record; a rerun reproduces them except the timing fields |
| `scripts/export_table.py` | The state at the end of a census record as a pose table |
| `scripts/eb4.py`, `scripts/eb4_close.py`, `scripts/cellpure4.py`, `scripts/eb4_replay.py` | E-B4: the structural stage and N1 and N2 at Y; the closures and the steps at Z; the class-cell pure twist with the exact-pose filter; the independent replay |
| `runs/eb4-k60.json`, `runs/eb4-close-k60.json`, `runs/eb4-k60-steps/u<k>.json`, `runs/eb4-replay-k60.json` | E-B4 records: stages 1 and 2 (`eb4.py`), the closures and the checked steps (`close.json` of `eb4_close.py`), the 20 step words with input and output digests, and the replay record |
| `runs/eb5-k40-run1.jsonl` | E-B5 run 1: the driver's event record (no step) |
