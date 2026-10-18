# 12. Low heights: completeness of the off-lattice phase at left heights 2 and 4 (option B, 17 October 2026)

Status: in progress. The statement and the plan passed a plan check and a scoped re-check; experiments E-B1 to E-B4
and E-B6 are done, E-B5 has two bounded runs (no step, then twelve steps, 1,193 to 377), and the transport of LC-13 has its
first words and two closures (section 10). Theory round 22 (18 October 2026) proved the first lemmas of four research
lines toward B′(h) (section 11; the proofs are in [`PROOFS-B.md`](PROOFS-B.md)).

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
- **The goal** (owner, 18 October 2026: [decision](../../../../docs/wiki/decisions/owner-decisions-2026-10-18-bprime-goal.md)):
  a single step of a few hundred twists is executable by a person, so B′(h) is to be proved as an existence statement
  with explicit L_h and S_h that need not be small. The shortest words are not sought. The research starts from the
  units that the zero-Bad census (section 10) and E-B5 run 2 leave stuck, on four lines: (a) a zero-Bad transport
  lemma, (b) the vacating word V with an explicit bound, (c) the T2 catalogue and T3 unlocking by a joint attack,
  (d) T4 at height 4.

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
   made no step; run 2 made twelve, from 1,193 to 377 off pieces (section 8).
6. **E-B6**, height 4: the two blocks of the depth 100 residue that hold height-4 pieces, with X as the positive
   control and the fixture end as a negative control (done, section 9).

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
off only the lattice pieces of the cell. That filter decided 3 of the 20 closures (`runs/eb4-exact-tries-k60.json`,
copied from the records of `cellpure4.py` by `scripts/eb4_tries.py`): at the units 2, 105 and 122 the first two,
three and two solutions tried left 14, 15 and 87 pieces of other contents at their points but at other poses, and
the third, fourth and third kept them all; at the other 17 the first solution tried passed.

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

## 8. E-B5: a bounded driver run from the depth 40 stop (run 1: no step; run 2: twelve steps; measured)

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

Run 2 (17 to 18 October) used `cellpure5.py` (section 10) as the pure-twist script, through the driver's new option
`--pure-script`, every unit tried (`--pure-max 22`), the searches of run 1 as seeds, and one hour for each heuristic
subprocess (`--par 4 --heuristic-seconds 3600`, the other bounds as in run 1). It made twelve steps in about 14
hours, from 1,193 to 377 off pieces in 8,702 twists (`runs/eb5-k40-run2.jsonl`): six driver steps and six
pure-twist closures (units and lengths in section 11). The stage 09 verifier replayed all twelve from the start
table: each lands its unit, with no collateral and no lattice piece taken off; three driver steps also moved other
units inside the driver's free set (steps 0, 2 and 9), so they are clean steps but not strict′. At 377, with nine
units left, no pure-twist step was found, and the fallback found no single letter for b16, the 111-piece triangle
content (34,414 letters tried in 35 minutes); the length bound 1 ended the run, as in run 1.

What run 2 shows: with `cellpure5.py` the driver takes the depth 40 stop to 377 on its own. Its nine remaining units
(b03, b04, b05, b06, b12, b15, b16, b18 and b19) are among those that line (a) and E2 landed by closures and T1′
steps on their own trajectory, which did not pass through 377 (section 11). It does not say whether the driver alone
could finish.

## 9. E-B6: four-letter commutators that lower the high units (measured, exact replay)

T4 asks for an itinerary that lowers a unit of left height 4 or more and leaves every other piece where it is: the
lens words P1 and P2 of R21F-02, or two protected lowering segments. E-B6 tests the first form at the depth 100
residue, with X (the prefix 156 of the stage 10 witness H6030, E21-3) as the positive control and the end of the
fixture as the negative control.

`scripts/eb6.py` reads one state. For every high unit (the off pieces of left height at least 4 with one pose and one
inside vector over the 600 caps, E21-3) it lists the caps that hold the unit whole, which of them are clean, the caps
of its own branch with their 12 lowering items (R21F-03) and their other contents, and the lens pairs. With `--comm4`
it then finds every *strict four-letter commutator* for the unit: a word (x, a)(y, b)(x, a⁻¹)(y, b⁻¹) of non-identity
items at two grips after which every twist was admissible, every piece outside the unit is at its exact pose (lattice
pieces included) and every piece of the unit has moved, to one common pose. When the unit is the whole common content
of the two caps, P1 is such a word, and P2 at (x, y) is the commutator at (y, x). The search is complete by the
following lemma, and `scripts/eb6_replay.py` replays every word it keeps on a freshly built state.

The pieces of a jumbled state do not tile the body (the regions of off pieces stick out of it, section 4), so the
lemma uses only the ball that holds every piece: its radius R is the largest distance from the centre to a vertex of a
home cell (R² = 96 − 32√5, R ≈ 4.944), and twists are rotations. Two caps *meet* when they share a point of this
ball.

**Lemma E6 (proved here).** Let w = (x, a)(y, b)(x, a⁻¹)(y, b⁻¹), with x ≠ y, be admissible at Z, move every piece of a
unit U and return every other piece to its pose at Z. Then:
1. the cut of x is clean at Z, and every piece that straddles the cut of y at Z belongs to U;
2. U lies whole in x or whole in y at Z, and the two caps meet;
3. if a piece outside U lies whole in both caps at Z, the rotations of a and b commute;
4. a piece outside U that lies whole in x and outside y at Z lies outside y after the first letter;
5. if U is not whole in y at Z, the first letter carries U whole into y; if U is not whole in x at Z, the second
   letter carries U whole into x.

*Proof.* A twist rotates the pieces inside its cap, maps the cap onto itself and is admissible only when no piece
straddles its cut. For a piece p let m₁, …, m₄ ∈ {0, 1} record whether p lies in the twisted cap at each twist; p ends
at b^(−m₄)·a^(−m₃)·b^(m₂)·a^(m₁) applied to its pose. A twist keeps its cap, so (m₁, m₂) = (0, 0) forces m₃ = m₄ = 0;
(1, 0) forces m₃ = 1 and puts p back in place, so m₄ says whether p lies in y at Z; and (m₂, m₃) = (1, 0) forces m₄ = 1.
The patterns left are 0000, 1010, 0101 and 1111, which return p when 1111 has ab = ba, and 1011, 0110, 0111, 1101 and
1110, with products b⁻¹, a⁻¹b, b⁻¹a⁻¹b, a and a⁻¹ba. Only a⁻¹b can be the identity, when a = b, and then p is back in
place inside y, so m₄ = 1. So a piece outside U follows 0000, 1010, 0101 or 1111, and pieces of U, which share one
inside vector, start alike. (1) The first twist acts on Z, so the cut of x is clean there. A returned piece lies at its
place at Z after the last twist and before it in the patterns 0000 and 1010, which end outside y, and the last twist
keeps y in 0101 and 1111, which end inside y; so every piece outside U lies whole inside or whole outside y at Z. (2) If
U is not whole in x at Z, it lies outside x, which is clean; its pieces start with m₁ = 0, and 0000 would return them,
so they lie in y at Z. If no point of the ball lies in both caps, a piece in one cap lies outside the other, so every
piece follows 0000, 1010 or 0101, and w moves nothing. (3) A piece in both caps cannot follow 1010, which ends outside
y, so it follows 1111, and ab = ba. (4) A piece in x and outside y cannot follow 1111, which ends inside y, so it
follows 1010, and m₂ = 0. (5) If U is not whole in y at Z, it is whole in x by (2), and a piece of U with m₂ = 0 is back
in place before the last twist, which needs it outside y there, and then it follows 1010 and does not move; so m₂ = 1
for every piece of U. If U is not whole in x at Z, it lies outside x and in y, and a piece of U with m₃ = 0 follows 0101
and does not move; so m₃ = 1, and the second letter, which acts on U at its place at Z, carries U into x. ∎

Two caps meet exactly when R cos(φ/2) ≥ α|n|, with φ the angle between their poles and α|n| ≈ 4.430 the distance of
a cut from the centre (2θ = 52.73°). `scripts/eb6_meet.py` checks this in exact arithmetic for all 359,400 ordered
pairs (`runs/eb6-meet.json`): each cap meets 80 others, at 15.52°, 25.24°, 36°, 41.41°, 44.48° and 49.12°, the next
angle is 56.01°, and the caps within 55.7° (`restore.NEAR`) are exactly the caps that meet it.

The search takes as first cap every clean cap that holds U whole, with as last cap every other cap within 55.7° of it,
and as last cap every clean cap that holds U whole, with as first cap every clean cap within 55.7° of it that does
not hold U. Pairs that fail item 1 are dropped. When U is not whole in the last cap, only the first letters that carry
it whole into that cap are kept, and when U is not whole in the first cap, only the second letters that carry it whole
into the first cap (item 5, an exact test at Z); a pair with no such letter is dropped. Where every piece whole in
both caps belongs to U, every remaining item pair is replayed, skipping a first letter after which the last cap is
blocked or holds a piece of the first cap outside U (item 4); elsewhere only the remaining item pairs whose rotations
commute are replayed (item 3; a float screen, confirmed exactly).

| State | Off pieces | High units (pieces) | Held whole by a clean cap | Cap pairs | Fail item 1 | Fail item 5 | Searched: all items / commuting | Words replayed | Strict words | Lowering |
|---|---|---|---|---|---|---|---|---|---|---|
| X, unit K only | 52,883 | 47 (710); K: 7 pieces at height 30 | K: caps 11 and 80 | 208 | 110 | 72 | 10 / 16 | 3,428 | 1,434 | 232 |
| Depth 100 stop | 31,479 | 9 (124) | all 9: cap 61 only | 981 | 459 | 342 | 0 / 180 | 0 | 0 | 0 |
| Fixture end | 121,184 | 305 (4,639) | 119: one cap each; 186: none | 10,015 | 9,025 | 814 | 0 / 176 | 0 | 0 | 0 |

At the pairs searched by commuting items, no commuting item pair passed the item 5 tests, at any of the three
states, so every word replayed comes from an all-items pair. Records: `runs/eb6-X.json`, `runs/eb6-k100.json`,
`runs/eb6-fx841.json`.

Readings:

1. **Positive control.** At X the search finds 1,434 strict four-letter commutators for K, and 232 of them lower it: 196
   to height 28 and 36 to height 26, a fall of 4 in four letters. 1,075 of them (88 lowering) use the caps 11 and 80,
   which both hold K. The other 359 (144 lowering) use a pair of caps 36° apart (11 with 0, 4 or 25; 80 with 180) in
   which no piece lies whole in both caps at X: one letter carries K into an overlap that the jumbled pieces leave
   empty, which is why the search must not assume that the pieces tile the body. `eb6_replay.py` replays the 232
   lowering words and the 20 other strict words that the record keeps on a freshly built X: all 252 move exactly the 7
   pieces of K, to one pose, with the recorded heights (`runs/eb6-replay-X.json`).
2. **Depth 100 stop.** The 124 height-4 pieces are the two blocks of stage 11: the 81-piece block falls into five
   units and the 43-piece height-4 frame into four (one pose each, split by inside vector), all in branch 3. Each unit
   lies whole in the clean cap 61 and in no other clean cap; cap 61 holds 3,018 pieces, 2,361 of them on the lattice.
   Of the 109 cap pairs of each unit, 51 fail item 1, 36 to 46 fail item 5, and in the other 12 to 22 a piece outside
   the unit lies whole in both caps while no commuting item pair passes the item 5 tests. So no unit at the depth 100
   stop has a strict four-letter commutator, and the search replays no word.
3. **Fixture end.** 186 of the 305 units lie whole in no clean cap, so items 1 and 2 exclude a strict four-letter
   commutator outright. Each of the other 119 lies whole in exactly one clean cap; of their 10,015 cap pairs, 9,025
   fail item 1, 814 fail item 5 and in the other 176 no commuting item pair passes the item 5 tests. No unit at the
   fixture end has a strict four-letter commutator.

What E-B6 settles: at X a strict four-letter commutator lowers K, and at the depth 100 stop and the fixture end no
unit has one. There each unit shares its only clean cap with thousands of other pieces, and most cuts near that cap
are blocked. The search covers the commutator form only: it does not exclude other words of four letters or fewer,
or T4's second form, two protected lowering segments, so it sets no lower bound on the length of T4 at those states
(section 12). The non-existence is exact for these two states and says nothing about other states of the same depth.

## 10. LC-13: transport words for triangle and tetrahedron contents (measured, exact replay)

A triangle or tetrahedron content cannot land in its own cell (LC-12); it lands only by a transport to label 0 of a
home cell of its kind (LC-13, `09-general-method/SENIOR-REVIEW.md` section 6). The seeds of LC-13 (48 and 24 letters)
are not in the repository, and its bounds (240 and 192 letters) assume every class-P grip free, so
`scripts/cellpure5.py` looks for a transport at the state instead. It is `cellpure4.py` with a second branch, marked
in the file, for a unit whose content is not in an edge cell:

1. **Search, in the class-cell model.** For each ordered pair of free class-P grips that share a cell, one of them a
   cap of the unit's cell X, and each item x at the first grip and y at the second, it follows the powers [x, y]^n of
   the single-letter commutator up to n = 60 (`--max-power`). A power is a candidate when it moves the unit's point
   to label 0 of a home cell (a cell that holds lattice chambers) and fixes the point of every other off content of
   the class.
2. **Order.** Candidates are sorted by the number of lattice pieces they take off label 0, which are the pieces V
   must vacate, then by whether another off content of the class lies in one of the two caps (its exact pose can
   change), then by length.
3. **Exact filter.** W is the first candidate whose exact replay from the pose table lands the unit, keeps every other
   off piece at its exact pose and takes off exactly the predicted lattice pieces (at most 12 tries, as in
   `cellpure4.py`).

W then goes to the closure V·W·V⁻¹ of stage 09 (`tclose2.py`, and `tclose2m3.py` when the first finds no V), as an
edge-cell word does. The driver runs `cellpure5.py` through its new option `--pure-script` (section 8).

At the depth 40 stop, for its first three units in block order (`runs/lc13-k40/`; 3 to 4 minutes per word and 4 to
12 minutes per closure):

| Unit | Content | Grip pairs | Landing powers | Candidates | W | Exact tries | Lattice pieces off | Bad(W) occupied | Closure |
|---|---|---|---|---|---|---|---|---|---|
| b00 | triangle, 33 pieces | 66 | 125,751 | 5,798 | [x, y]³, 12 letters | 3 | 111 | 111 of 111 | no V (greedy bounded at 3,600 seconds: 12 left) |
| b01 | tetrahedron, 72 pieces | 84 | 221,937 | 16,052 | [x, y]⁶, 24 letters | 1 | 15 | 15 of 87 | V of 300 letters; step of 624 letters |
| b02 | triangle, 111 pieces | 66 | 125,751 | 4,632 | [x, y]³, 12 letters | 2 | 111 | 111 of 111 | V of 18 letters (greedy, 3,600-second bound); step of 48 letters |

Each grip pair gives 59 × 59 commutators (229,746 for the triangle units, 292,404 for the tetrahedron unit). A landing
power moves the unit's point to label 0 of a home cell; a candidate also fixes the other 12 off contents of the class.

- **Every unit has a transport word.** Each W is one commutator power of 12 or 24 letters, far below the bounds of
  LC-13, and lands the unit whole at its exact replay. Every candidate also takes lattice pieces off label 0: at
  least 111 for the triangle units (W takes the lattice content of one triangle home cell) and at least 15 for the
  tetrahedron unit, so all three need a V. The exact filter rejected the first two candidates of b00 and the first of
  b02: each left 72 other off pieces at their points but at other exact poses.
- **b01 closes.** The peeling certificate (Theorem B) gives a V of 300 letters that vacates the 15 occupied chambers
  of Bad(W), one tetrahedron home cell. The step V·W·V⁻¹ has 624 letters and takes the off count from 1,193 to
  1,121: the unit lands, collateral is 0, every other off piece keeps its exact pose, and all 175,927 pieces on the
  lattice stay on it (175,841 at the same exact pose). `scripts/eb4_replay.py` replays the step independently from
  the exported table and finds it strict′ (`runs/lc13-k40/replay-b01.json`).
- **The two triangle units: b02 closes with a longer bound, b00 lands another way.** Bad(W) is a whole triangle home
  cell of 111 occupied chambers. The peeling certificate stops after 35 (b00) and 34 (b02) of them, also with the
  depth-2 search of `tclose2m3.py` (144,292 and 100,435 letter pairs scanned), and the greedy fallback, bounded at 300
  seconds, leaves 35 and 3 occupied (`runs/lc13-k40/close-no-v.json`). With the bound raised to 3,600 seconds the
  greedy search finds an 18-letter V for b02 after 504 seconds. The step V·W·V⁻¹ has 48 letters and takes the off
  count from 1,193 to 1,082: the unit lands, collateral is 0, every other off piece keeps its exact pose, and all
  175,927 lattice pieces stay on the lattice (`close-b02.json`, `steps/u2.json`; `eb4_replay.py` finds it strict′,
  `replay-b02.json`). For b00 the greedy search still stops, with 12 chambers occupied after 3,600 seconds, although
  the target can be vacated (R22B-04, section 11). b00 lands instead without any V: two clean powers of single-letter
  commutators, [x, y]³ and [x′, y′]¹², 60 letters, carry it into a vacant triangle home cell of its class, and the
  independent replay finds the step strict′, 1,193 to 1,160 off pieces with collateral 0 (`steps/u0.json`,
  `replay-b00.json`; section 11).

What this shows, at one state: LC-13's transport exists here as single commutator powers of 12 and 24 letters, and
the closure works for b01, whose Bad(W) holds 15 lattice pieces, and for b02, whose Bad(W) holds 111. For a triangle
content every candidate takes at least 111 lattice pieces off, so the closure must vacate at least 111 chambers; where
a vacant home cell of the unit's kind can be reached by clean powers, no V is needed at all (section 11).

## 11. Theory round 22: the stuck units and four lines toward B′(h) (18 October 2026)

The owner's decision of 18 October set the goal of section 1 and four lines of research that start from the units the
depth 40 stop leaves stuck. The senior reviewer's results are in `09-general-method/SENIOR-REVIEW.md` section 27, and
their proofs, written hypothesis by hypothesis for a later Lean formalization, in [`PROOFS-B.md`](PROOFS-B.md).

**The 22 units of the depth 40 stop.** All are at height 2, in classes 2, 3 and 4. The census of line (a)
(`scripts/round22/transport_census.py`) searches, in the class-cell model with whole cells as owners (R22A-02), for
paths of clean commutator powers that carry each unit into a vacant home cell of its kind, and for entry powers into
an occupied one, and replays every word it keeps exactly:

| Group | Units (kind, pieces) | What holds them |
|---|---|---|
| A step without endpoint collateral | class 3: b00 (triangle, 33), b01 (tetrahedron, 72), b02 (triangle, 111), b07 (triangle, 111), b11 (tetrahedron, 29), b13 (triangle, 101), b14 (triangle, 78); class 2: b16 (triangle, 111), b20 (triangle, 60), b21 (triangle, 10) | strict′ steps of 60, 84, 60, 48, 60, 60 and 60 letters in class 3 and 72, 36 and 84 in class 2 (1 to 3 powers); no lattice piece goes off, so no V is needed. The targets are shared: class 3 has four vacant targets for its seven units, class 2 two for its three, so from this state at most four and two of them land this way (R22A-05). A first census with points as owners had failed to replay for b02, b07, b13, b14 and b21: it turned another off content of a moved cell to another pose at the same point (R22A-02). b02 also closes by its LC-13 transport (section 10) |
| Entry transport and closure | b19 (tetrahedron, 16, class 2), b08, b09, b10 (triangle 33, tetrahedron 58, triangle 34, class 4) | no vacant home cell of the unit's kind in its class; an entry power carries each into an occupied home cell of its kind and takes that cell's lattice content off (15, 20, 24 and 20 pieces). The closure V·W·V⁻¹ of R22A-06 gives strict′ steps for b08, b10 and b09 (222, 270 and 704 letters, V of 63, 63 and 268 letters), each replayed independently. For b19 N1 and N2 hold and every orbit that meets its Bad(W) has a Jordan certificate, so V exists (Corollary A1) and b19 has a strict′ step V·W·V⁻¹; it is not built yet, since neither the peeling certificate (68 of the 198 chambers of Bad(W)) nor a 300-second greedy search (13 chambers left) found V. b08 and b10 enter the same home cell, so only one of them lands there first |
| Glued | b03, b18 (triangle, 63 and 5), b04, b05, b12, b15 (edge, 45, 61, 41, 25), b06 (vertex, 10) | the unit shares its cell with lattice pieces, which every class-P landing takes off (R22A-07), and the cell holds occupied chambers that no SAFE word moves (N1 fails), so no closure V·W·V⁻¹ exists for them at this state |
| In place | b17 (edge, 86) | an edge content lands in its own cell (Theorem C), not by transport; no clean power moves it. At this state its pole 542 holds pieces of b08, b09 and b10; once they land, Theorem C applies and gives a step of 432 letters (line (a), round 3, below) |

E-B5 run 2 (the driver from this state) made twelve steps, from 1,193 to 377 off pieces:
driver steps of 10, 10 and 848 letters for b07, b13 and b21, pure-twist closures of 576, 24, 762, 928, 1,130 and
1,094 letters for b17, b20, b01, b09, b10 and b08, then driver steps of 904, 1,128 and 1,288 letters for b00, b14 and
b02. Three
of the driver steps changed the poses of other units (b11; b18 and b19; b03), which the driver's free set allows, so
they are clean steps but not strict′. b09 is in class 4, which has no transport target:
the pure twist lands a content in its own cell and its closure vacates what the twist takes off, so "no target"
rules out transport only. Then it stopped at the fallback's length bound (section 8).

**The four lines after round 22.**
- (a) Transport without endpoint collateral. A finite test at the state, with whole cells as owners, decides whether
  clean commutator powers carry a unit into a vacant home cell of its kind, and gives the step and an explicit
  (existence) bound (R22A-01 to R22A-04). Its limits are proved: contents never merge, so one class lands at most as
  many contents of a kind as it has vacant home cells of that kind (R22A-05); a glued content takes its cell's lattice
  pieces off whenever it lands (R22A-07). At this state class 3 has five triangle and two tetrahedron contents that
  clean powers move but three and one vacant targets, and class 4 has none. An entry power into an occupied home cell
  followed by a closure with V (R22A-06) lands three of the four units without a target (b08, b09, b10); for the
  fourth, b19, V exists by Corollary A1 but is not built yet. Round 2: b19's V can be built, |V| < 10^539
  (R22A2-01); a step computed at one state stays strict′ after another exactly when the other's landing leaves its
  complete Bad set vacant (R22A2-02), and the computed steps, used unchanged, compose at most six deep (R22A2-03);
  with a fresh V for each step, fourteen units land in a checked order, from 1,193 to 336 off pieces (R22A2-04).
  The other eight are b17 and the seven glued units, which no SAFE word can free at this state (R22A2-06).
  Round 3 replayed that order at the actual states: eight units land, b17 among them, from 1,193 to 737 off pieces,
  and four more have proved steps at 737. E2 continued the replay with every step built at the state in front of it:
  all 22 units land, 1,193 to 0 in 21 strict′ steps, b06 with b05 by a whole-cap T1′ after a three-letter SAFE-ALL
  preparation.
- (b) The vacating word V. Whenever V exists, one of at most C(177,120, n) − 1 letters exists (R22B-02); with a
  certificate of three-cycle seeds and conjugator tables, |V| ≤ 10·|T|·L (R22B-01). b00's transport target can be
  vacated: its 19 target orbits have Jordan certificates (R22B-04). A greedy V counts net gains only and must be
  checked as a whole (R22B-03). Round 2 closed the certificate in theory: where an orbit carries Alt(Δ), tables of
  Schreier words always exist, with e ≤ (d − 2)(2d − 1) and f ≤ (d − 3)(2d − 3)(2d − 1) (R22B-05); a seed that fixes
  the earlier orbits exists exactly when their pointwise stabilizer acts on the orbit, with a procedure that always
  finds one (R22B-07); and a Jordan word with any letter of small support gives a seed for every order of the orbits
  (R22B-08). Moving the anchors cannot be relied on: a nine-point alphabet has a seed and a Jordan word but no letter
  that fixes any point (R22B-06). At b00 every hypothesis holds, so the construction terminates with |V| < 10^539
  (R22B-09, a construction bound); no V has been built yet, and the checker for word-valued tables is specified
  (R22B-10). Round 4: Alt does not hold on every large orbit at every state of 𝒞₂ (R22B-21), so vacancy needs N1
  and N2 together with independent alternating factors (R22B-19) and, on orbits of 2 to 7 chambers, a joint finite
  test (R22B-22). Round 5 settles vacancy at a given middle: V exists exactly when N1, N2 and a joint test of the
  target's images pass, a finite test that needs no Alt (R22B-29); twenty twists from solved reach a state where an
  orbit without Alt meets a real target with one hole, and one letter vacates it (R22B-24).
- (c) The T2 catalogue and T3 unlocking: a joint attack. After the two sealed analyses the solver stopped (owner, 18
  October 2026); the senior reviewer cross-examined the solver's analysis against its own, and the integrator runs
  the adjudicating experiments. Glue is a theorem: every landing word of the unit's class takes the lattice content
  of its cell off (Lemma G). An occupied home chamber locked to the unit's own block rules out every step that turns
  the cell once between retained words (Lemma T: T1, M1-18 for every r, partner templates with retained phases);
  turning it twice, W·V′·W⁻¹·V′⁻¹ (T1′), escapes Lemma T. Partners never restore N1. The catalogue
  {T1, T1′, T2′} × {r ≤ 1} gives conditional explicit bounds on a class of states defined by finite checks, but a
  state's membership does not carry over to its successor, so coverage needs its own argument
  (`09-general-method/SENIOR-REVIEW.md` section 27). Round 3: an order of steps needs resource contracts as well as
  a blocker order (R22C3-02), and at the depth 60 stop 142 then 151 is proved, from 14,610 to 14,524 off pieces
  (R22C3-04).
- (d) Height 4. A retained letter before a word and its inverse after transfers the landing problem exactly
  (R22D-01), and a lowering conjugate has an exact endpoint contract (R22D-03). At the depth 100 stop 93 of the 108
  direct free-class cases (R22D-04) and both whole-cap middles (R22D-06) are impossible; SAFE preparations cannot
  unlock a cut (R22D-07).
- What all four leave open is coverage: that every state of 𝒞_2 (𝒞_4) has a step in a stated catalogue (first attempt:
  `PROOFS-B.md` section 8, COV-01 to COV-07). The explicit
  bounds proved so far are either conditional (R22B-01) or existence bounds that show a finite search decides a
  question; the measured steps (10 to 1,158 letters at this state) are of human scale.

**Experiments of the round (integrator).**
- E22-1 (measured, scratch probe): a vacating word for b00's target built from the greedy moves and from powers of
  commutators whose exponent, chosen by the Chinese remainder theorem, keeps every vacated target vacant and carries a
  hole onto the next target, each word checked on the full chamber permutation. The greedy phase (16 letters) vacates
  77 of the 111 target chambers; for the next one (chamber 3392) no commutator around it has such a power, so this
  construction stops with 34 occupied. A probe for seeds of the form [[x, y], z] found none. Neither result bears on
  existence (R22B-04).
- E22-1, the certificate of R22B-01 (`scripts/r22b01_cert.py`, written by Codex and reviewed by the integrator;
  `scripts/test_r22b01.py` checks every ordered three-cycle, the commutator support rule and the seed rule on small
  exact alphabets; measured, exact). For each of b00's 19 target orbits, largest first, the builder searches two- and
  three-letter SAFE commutators for a power that acts on the orbit as one three-cycle and fixes every earlier orbit
  (seeds of at most 20,000 letters), builds the two tables of R22B-01 by breadth-first search over single SAFE
  letters, and vacates the orbit's targets by conjugated three-cycles, each checked on the full chamber permutation.
  Within 900 seconds it completes two orbits per run: orbits 41 and 64 in one run (V of 318 letters, bound 376), 69
  and 67 in the acceptance run (1,008 letters, bound 1,440); the acceptance check fails because the certificate is
  incomplete. The other orbits fail inside the construction. Either no seed turned up in the orbit's time share (the
  search is random and timed, so which orbits get a seed differs between runs), or a table is disconnected: a few
  chambers (1 to 22) cannot be reached along letters that fix the anchors, and in four orbits the second table
  reaches nothing, because every letter that moves the third anchor moves one of the first two. This rejects single-letter
  tables, not V (R22B-04): the tables may use words, and conjugating the seed moves its anchors. `PROOFS-B.md`
  section 4 gives the result per orbit.
- E22-1b, the certificate of R22B-10 (`scripts/r22b10_cert.py` builds it and `scripts/check_r22b10.py` checks it, both
  written by Codex and reviewed by the integrator; `scripts/test_r22b10.py` holds the controls R22B-10 lists;
  measured, exact). For each target orbit, largest first, the builder tries short SAFE commutator seeds for a few
  seconds (8 here) and otherwise takes the Jordan route of R22B-08; it grows both tables as Schreier trees whose edges
  are words (R22B-05), and it keeps every word as an acyclic straight-line program with exact integer lengths, never
  expanded. The checker, which imports nothing from the builder, recomputes the orbits, N1 and N2 from the state,
  checks the Jordan certificates on it and every seed, table edge and three-cycle, evaluates V on every target orbit
  (every SAFE letter keeps each orbit, so this decides the targets' occupancy exactly) and replays the steps in order.
  b00 (111 targets in 19 orbits) completes in 254 seconds: V has fewer than 10^166 letters, within its bound (also 166
  digits), stored as 14,018 program nodes. b19 (198 targets in 20 orbits, 15 occupied) completes in 244 seconds: V has
  fewer than 10^141 letters (bound below 10^166), 12,688 nodes. The checker passes both. So b19's V, until now only
  shown to exist (R22A2-01), is built, and the construction of R22B-09 ends far below its 10^539. Every seed came from
  the Jordan route (121 to 165 digits), while the tables are short (e, f ≤ 22), so the length of V sits in the seeds
  (`runs/round22/e22-1b/`). E22-1's commutator seeds of 12 to 1,320 letters were each checked only against the orbits
  completed before it, so they do not form a certificate in the full order (R22B-11).
- Line (b), round 3 (the senior reviewer; R22B-11 to R22B-18 in `PROOFS-B.md` section 3; its computation
  `scripts/round22/r22b13_seeds.py`, re-run by the integrator with the same output, `runs/round22/r22b13_seeds.txt`,
  about 2 minutes):
  - A power of a SAFE word q is a seed exactly when, on the current and the earlier target orbits together, q has one
    cycle of length divisible by 3, of length 3 and in the current orbit; the least such power is q^M with M the least
    common multiple of the other cycle lengths (R22B-12). Two eight-letter commutators pass this test in the full
    order at b00: q⁴ on orbit 34 (32 letters) and q⁵ on orbit 66 (40 letters, fixing all of orbit 34) (R22B-13).
  - Every SAFE letter at the depth 40 stop has order 2 or 3, so no power of a single letter has a smaller support
    (R22B-14). A local Jordan witness already gives Alt on its orbit, but only the global certificate gives seeds that
    fix the earlier orbits; that every orbit carries Alt at every state of 𝒞₂ stays open (R22B-15).
  - The checker of E22-1b is sound and the recorded numbers hold (R22B-16). Seeds of at most S letters for every
    orbit would give |V| ≤ 2n·[S + 2,984,118,076,786] at this geometry, linear in n (R22B-17); the search that would
    look for them is specified, not built (R22B-18).
- Line (b), round 4 (the senior reviewer; R22B-19 to R22B-23 in `PROOFS-B.md` section 3; its computations
  `scripts/round22/r22b20_separation.py`, `r22b21_regular12.py` and `r22b22_small_orbits.py`, re-run by the
  integrator with the same outputs, `runs/round22/`, 2 to 5 minutes each):
  - Local Alt on each target orbit of at least 8 chambers, and for each pair of equal size a SAFE word with
    different cycle types on the two, give the whole product of the alternating groups, so N1 and N2 then decide
    vacancy (R22B-19). At b00 two of the 40 equal-size pairs are separated by no single letter, but the first
    120-letter random word separates all 40 (R22B-20).
  - Alt does not hold on every large orbit at every state of 𝒞₂: eight twists from the solved state reach a
    height-2 state with an orbit of 12 chambers whose group has order 12 (R22B-21). That orbit has no holes, so the
    universal claim falls, not B′(2).
  - Orbits of 2 to 7 chambers need a joint breadth-first test (R22B-22). The depth 60 stop has 12 orbits of 4
    chambers and 6 of 6 with groups of order 12, and one of 7 with Alt(7). Without Alt, a vacating word, when one
    exists, has at most ∏ C(d, k) − 1 ≤ 7,200^|T| − 1 letters; this bounds a word, it does not show one exists
    (R22B-23).
- Line (b), round 5 (the senior reviewer; R22B-24 to R22B-29 in `PROOFS-B.md` section 3; its computations
  `scripts/round22/r22b24_regular_hole.py`, `r22b26_small60.py` and `r22b27_small_t1prime.py`, re-run by the
  integrator with the same outputs, `runs/round22/`, 2 to 4 minutes each):
  - The vacancy theorem (R22B-29): at a given middle, a SAFE-ALL word that clears a complete target exists if and
    only if N1 and N2 hold and the joint orbit of the target's traces on its nontrivial orbits, under the SAFE letters
    acting on all of them at once, contains a tuple that lies in the holes. It is a finite test, uses no group
    classification and no Alt, and bounds the shortest such word; with independent alternating factors on the large
    orbits it reduces to the small ones. So the case of an orbit without Alt that meets a target with holes in it is
    decided by a finite test at each middle.
  - Such a case occurs: twenty letters from the solved state reach a height-2 state where a regular orbit of 12
    chambers with one hole meets a real catalogue target (the 111 home chambers of a class-3 triangle transport), and
    the single letter (6, 1) still clears it (R22B-24). So "every target orbit with enough holes carries Alt" is
    false; N1 and N2 still sufficed there.
  - A finite classifier of the induced action (exact order, block systems, the regular case) leaves a primitive
    residual case that only the joint test decides (R22B-25). At the depth 60 stop no home-cell target meets an orbit
    of 2 to 7 chambers, and neither do the T1′ middles of 151 and 180 (R22B-26, R22B-27). A strict′ step keeps the
    old subgroup's certificates, but not the hole inventories, which must be read again (R22B-28).
- Coverage of B′(2) (the senior reviewer; COV-01 to COV-07 in `PROOFS-B.md` section 8; its computations
  `scripts/round22/cov01_isolated_triangle.py`, `cov02_point_orbits.py` and `cov04_middles737.py` re-run by the
  integrator with the same outputs, and its proposed experiment run as `cov07_word_order.py`; `runs/round22/`):
  - A state with one off piece, a type-18 triangle content of class 0 with one hole, has an intrinsic Q-shadow: two
    occupied type-24 chambers block every retained letter that moves its landed chamber, which defeats T1, every T1′,
    every retained preparation and T2′ (COV-01). So a lemma that covers isolated units by geometry alone is false;
    whether this state is reachable is open.
  - Up to K⁺ there are seven geometric off-point cases, 5,764 slots in a class (COV-02); a point enumeration, not a
    classification of contents or states. What a local certificate contains, two deductions for isolated contents,
    and a symbolic Q-test; the missing part is coverage at every state, not closure (COV-03). Coverage checks need the
    holes and the off pieces' poses, not the lattice orientations, with explicit successor formulas for the holes
    after T1 and T1′ (COV-06).
  - At 737, b05 and b15 are blocked by other units, {b03, b04, b06} and {b07, b12, b13, b14}, not by Q alone, and
    b06 is short of two type-6 and one type-9 pieces (COV-04); unit 130 at depth 60 is blocked by seven units
    (COV-05).
  - COV-07: the four stored T1′ words of b03, b04, b12 and b18 form no order at 737. b12's word is strict′ there; the
    other three are not, at 737 or after b12. So the words must be rebuilt at the current state, which the replay of
    line (a) continued (E2, below) does.
- Coverage, round 2 (the senior reviewer, after E2; COV-08 to COV-16 in `PROOFS-B.md` section 8; its computations
  `scripts/round22/cov09_resource_persistence.py`, `cov13_mixed_frames.py` and `cov14_isolated_wholecap.py` re-run
  by the integrator with the same digests and verdicts; `runs/round22/`):
  - Resources. A T1′ step's N1 and N2 are exact formulas in the holes of Z pulled back through the good-ghost map
    (COV-08). They do not survive other steps: before b15's step the whole-cap middle of b06 at grip 91 passes N2;
    after it, with the old middle alphabet kept, N2 fails by 20 holes on eight orbits (COV-09). So each step needs its
    middle recomputed, as E2 did.
  - Preparation has an exact finite test (some image of the hole set under the SAFE group gives a passing middle) and
    a flow criterion when the groups are alternating (COV-10); the greedy `e2_b06_prep2.py` is a heuristic whose
    failure proves nothing (COV-11).
  - Whole caps. A whole-cap letter is eligible exactly when every off pose in the cap lies in one right coset, and the
    three-segment form X·W·V′·W⁻¹ lets other off pieces of the cap move for a while (COV-12). A reachable state of 120
    letters has no cap whose whole off content lands (four frame domains), so that universal lemma is false; the state
    still has a step, the inverse of its history (COV-13). COV-01's isolated triangle has 36 whole-cap landing
    letters, and N1 fails at every one of them under every preparation (COV-14); its reachability is still open.
  - Coverage reduces to one finite per-state predicate Cover(Z): if every state of 𝒞₂ with off pieces satisfies it,
    every such state is restored in at most n strict′ steps within the bounds of R22C3-06 (COV-15). Neither a proof
    of B′(2) nor a reachable deadlock exists.
  - The two adjudicating experiments are done, and a bounded replay survey cannot decide Cover(Z) (COV-16). The
    packet's E5 inputs were wrong (the integrator's error): the census round 3 records are deeper scrambles with
    height-4 pieces, not depth 40 stops of 𝒞₂; the 𝒞₂ census stops on record are `ia-k40-s7101` and `ia-k60-s7104`.
- Coverage, round 3 (the senior reviewer; COV-17 to COV-22 in `PROOFS-B.md` section 8; its computations
  `scripts/round22/cov17_single_off.py`, `cov18_singleton_kinds.py` and `cov19_perfect_menu.py` re-run by the
  integrator with the same digests, certificates and counts; `runs/round22/`):
  - One off piece is reachable. The 24-letter word J·[[[A,B],B],D]·J⁻¹ (J = (0,12), A = (74,8), B = (188,4),
    D = (178,1)) reaches a state whose only off piece is 58388, of type 17 and height 2, in a triangle cell, and its
    inverse restores the solved state (COV-17); the smallest off count the integrator had found in the stored records
    was 3 (stage 01's E2). By global Jordan certificates at solved·J, words J·V·J⁻¹ with V SAFE leave exactly one off
    piece in a vertex, an edge, a triangle and a tetrahedron cell (COV-18).
  - These singletons are covered: the inverse J·V⁻¹·J⁻¹ is a three-segment whole-cap step (COV-12) within the bounds
    of R22C3-06, and the catalogue needs no new core for them (COV-21).
  - COV-01's state X stays open. No homomorphism to an abelian group can exclude it, since every grip's menu is a
    perfect group (COV-19), and no word J·V·J⁻¹ with V SAFE reaches its family, since the chamber its piece must come
    from is a SAFE-fixed hole (COV-20). The experiment COV-22 samples 24-letter mixed-class commutators for a reaching
    word (eight seeds, two at a time, 1,700 s of CPU each); only a hit decides.
  - The integrator's checks after COV-20 (`scripts/round22/cov20_all_landers.py`, output
    `runs/round22/cov20_all_landers.txt`): at every one of the 36 landing letters L of COV-14, the chamber that X's
    piece must come from is a hole fixed by the SAFE group of solved·L, so no word L·V·L⁻¹ with V SAFE reaches X's
    family. A second level, conjugates g·V·g⁻¹ through the legal lattice letters g of the other caps that hold off
    pieces, is running for two of them (`cov20_two_level.py`).
- The census with whole cells as owners (R22A-02), class 3 (13 units; measured, scratch census, each kept word
  replayed exactly): seven units have strict′ steps without endpoint collateral, b00 60, b01 84, b02 60, b07 48,
  b11 60, b13 60 and b14 60 letters (one to three clean powers), and the other six (b03, b04, b05, b06, b12, b15)
  are glued. The targets are shared: b00 and b02 go to one vacant triangle home cell, b07 and b13 to another, b14
  to a third, and b01 and b11 to the one vacant tetrahedron home cell, so at most four of the seven land this way
  from this state (R22A-05).
- The census, classes 2 and 4 (9 units; measured, each kept word replayed exactly): b16, b20 and b21 have strict′
  steps of 72, 36 and 84 letters (b16 and b21 to one vacant triangle home cell, b20 to the other), b17 is moved by no
  clean power (it lands in its own cell, Theorem C), b18 is glued, and b19, b08, b09 and b10 have no vacant target
  but entry transports of 84, 96, 168 and 144 letters into occupied home cells holding 15, 20, 24 and 20 lattice
  pieces. With the five disputed units of R22A-09 the whole-cell rule gives strict′ steps for all five (b02, b07,
  b13, b14 in class 3, b21 in class 2). Script `scripts/round22/transport_census.py`; records, unit files and
  closures in `runs/round22/line-a/`.
- Line (a), the closures V·W·V⁻¹ of the four entry transports (R22A-06; measured, exact): `tclose2.py` at the depth
  40 stop, each step replayed again from the exported table (`stepchk.py`). N1 and N2 hold for all four Bad sets.
  b08, b10 and b09 close with V from the peeling certificate (63, 63 and 268 letters): strict′ steps of 222, 270 and
  704 letters that land 33, 34 and 58 pieces, take no lattice piece off and keep every other off piece at its exact
  pose. For b19 the peeling certificate stops at 68 of the 198 chambers of Bad(W) and a 300-second greedy search
  leaves 13 occupied, so V is not built. But N1 and N2 hold and all 20 orbits of the SAFE-ALL group that meet this
  Bad(W) have Jordan certificates (`scripts/round22/jordan_target.py`, 37 seconds of CPU), so by Corollary A1 of
  `research/jumbling/theory/buffer-transport.md` a vacating word exists, and with it a strict′ step for b19.
- Line (a), round 2 (the senior reviewer; R22A2-01 to R22A2-06 in `PROOFS-B.md` section 2; its computation
  `scripts/round22/r22a2_04_sequence.py`, re-run by the integrator with the same output,
  `runs/round22/r22a2_04_sequence.txt`, about 75 CPU seconds):
  - b19's V can be built: with its Jordan certificates and one SAFE letter of small support on each of its 20 target
    orbits, the seed and table construction of line (b) terminates with |V| < 10^539, and b19's closure is a strict
    step of fewer than 10^540 letters (R22A2-01; a construction bound, nothing built).
  - A step computed at one state stays strict′ after another step exactly when the other's landing leaves the first's
    complete Bad set vacant, a finite test at the first state (R22A2-02). Used unchanged, the ten census steps and
    the closures of b08, b09 and b10 compose at most six deep: 52 of the 156 ordered pairs fail, and the longest
    order is b21, b00, b14, b07, b11, b20 (R22A2-03, a complete search).
  - With a fresh V for each step, fourteen units land in the order b00, b01, b02, b07, b08, b11, b13, b19, b09, b10,
    b14, b16, b21, b20, from 1,193 to 336 off pieces; the fourteen closures have fewer than 10^541 letters together
    (R22A2-04). The proof is an induction on the hole count of each orbit of the SAFE group, whose hypotheses are
    finite checks at the stop; the order matters, since b01 and b09 each use one of the two holes of a 120-chamber
    orbit that b07, b11, b13 and b19 also need. b08 and b10, which enter the same home cell, both land this way, but
    not with the old shared 63-letter preparation (R22A2-05). No V of the sequence is built and nothing is replayed.
  - The eight units left are b17, which no clean power moves, and the seven glued units: their own cells hold occupied
    chambers that every SAFE letter fixes (N1 fails), and for b05, b06 and b15 orbits of size 7,125 and more with too
    few holes (N2 fails), so no SAFE word of any length frees them at this state (R22A2-06).
- Line (a), round 3 (the integrator; measured, exact):
  - A fresh closure of b10 after b08 (R22A2-05; `tclose2.py` at the state after b08's 222-letter step,
    `runs/round22/line-a/b10-after-b08/`): N1 and N2 hold for b10's Bad(W) (111 chambers, 26 occupied), the peeling
    certificate is complete (V of 396 letters, its bound 1,371), and V·W·V⁻¹ is a strict′ step of 936 letters. Both
    steps replay strict′ from the exported tables (`stepchk.py`): b08 lands 33 pieces (1,193 to 1,160 off), b10 34
    (1,160 to 1,126), no lattice piece goes off and every other off piece keeps its exact pose. The old 63-letter
    preparation, used unchanged, had met 33 occupied chambers.
  - The order of R22A2-04 at the actual states (`scripts/round22/replay_a3.sh`, `runs/round22/line-a/replay/`):
    each step built at the state in front of it, the unit's census word when it is strict′ there, otherwise a fresh
    closure (`tclose2.py`), and replayed by `stepchk.py` before the next. Seven units land in 4,084 letters, from
    1,193 to 823 off pieces: b00 and b01 by their census words (60 and 84 letters), b02, b08, b11, b09 and b10 by
    closures whose V came from the peeling certificate (158, 762, 878, 970 and 1,172 letters). For b07, b13, b19,
    b14, b16, b21 and b20 the census word is not strict′ where it is tried and the 300-second greedy search does not
    finish V, but N1 and N2 hold and every orbit that meets the closure's Bad(W) has a Jordan certificate, so each
    has a strict′ step there (Corollary A1): proved, not built.
  - b17 after the order: at the state of 823 b08, b09 and b10 have left its pole, and the premises of Theorem C hold
    (`scripts/round22/thmc_premises.py`: the cell's 86 home chambers vacant, both poles and five common neighbours
    free, one landing rotation). `cellpure4.py --search` builds a pure-cell step of 432 letters that replays strict′,
    823 to 737. An order turned a unit that nothing moved at the stop into one with a short step.
  - At the state of 737 b14, b16, b21 and b20 still have proved steps (the same Bad sets, N1, N2 and Jordan
    certificates read at 737). Without a preparation b15 and b05 have landing words of 432 letters, but after them
    17 and 48 occupied chambers of Bad(W⁻¹) are fixed by every SAFE letter, so N1 fails; b06, a vertex content, has
    no landing word of that family. They need the orders that R22C3-05 proposes.
- Line (a), E2: the replay continued from 737 (the integrator; measured, exact; `scripts/round22/replay_slp.py` and
  the b06 scripts, `runs/round22/line-a/replay2/`, `PROOFS-B.md` section 2). Each step is built at the state in front
  of it and applied exactly by `scripts/slp_apply.py`, which evaluates a certificate as its straight-line program
  without expanding it and checks the strict′ contract:
  - Run 1, 737 to 437: b12 by its T1′ word (1,158 letters), then b14, b16, b20 and b21 by closures whose V is a
    checked R22B-10 certificate (steps of 151 to 161 digits). The closures of b07, b13 and b19 then fail N2: there
    are only as many holes as off pieces, and the landings since 977 and 915, where round 3 proved these closures,
    used up the holes of four SAFE orbits.
  - Run 2, 437 to 71: T1′ steps instead. b04, b03 and b18 with literal V′ (1,254, 1,106 and 770 letters), then b07,
    b13 and b19 with V′ a checked certificate at the middle (147 to 163 digits), and b15 once its COV-04 blockers
    b07, b12, b13 and b14 had landed. A closure needs a hole outside Bad(W) for each lattice piece that W takes off, a
    T1′ step one outside its target for each landing piece only (COV-06), so the T1′ steps passed where the closures
    had run out of holes.
  - b06 with b05: one grip-91 letter whose element is off K⁺ lands both; its Bad set is all of cap 91, so N2 at the
    middle asks for holes outside the cap, orbit by orbit, as many as the 71 landing chambers. A SAFE-ALL
    preparation of three letters moves the holes there; then N1, N2 and Jordan certificates hold, V′ is built and
    checked, and the step lands b06 and b05 together: 324 to 253 on a branch, and at 71, where they are the last two
    units off, 71 to 0 after the preparation (178, 7)(170, 1)(75, 1). So b06's type deficit (R22C2-05, COV-04) is a
    matter of where the holes are, and b06 needs no partner.
  - Totals: with round 3, all 22 units land, 1,193 to 0 in 21 strict′ steps. The literal steps have 8,804 letters;
    the certificate steps have explicit lengths of 147 to 164 digits, under 10¹⁶⁴ letters in all. These are
    existence bounds, not words to play, and this is one trajectory: it says nothing about other states (section 12).
- Line (c), the integrator's re-check of the solver's computed steps (measured): the solver's four T1′ steps at the
  depth 40 stop, each its landing word W followed by V′·W⁻¹·V′⁻¹, replay strict with the units recomputed from the
  state: b03 160 letters, b04 942, b12 1,158 and b18 160; no lattice piece leaves the lattice and every other off
  piece ends at its exact pose (`runs/round22/t1prime-k40/`). All four units are glued, and every occupied chamber of
  their home cells that no SAFE letter moves is shadowed by the unit's own block; for b03, b04 and b18 some of these
  are locked to that block, for b12 none, so b12's step does not show T1′ escaping a lock (X22C-02).
- Line (c), the adjudicating experiments (measured, exact; every step checked from an exported pose table):
  - A, an order at the depth 40 stop (X22C-06): b12's T1′ step, then b14, then b15. The b12 step replays strict′.
    b14's census word (60 letters), computed at the stop, is admissible after it but takes 17 lattice pieces off, so
    it is not strict′ there: words computed independently at one state need not compose. At that state b15 has a
    landing word (432 letters; all 25 pieces land, the 86 lattice pieces of its cell go off), but 25 chambers of
    Bad(W⁻¹) are occupied and fixed by every SAFE letter, so this W has no closure there. The order is not decided;
    it needs a strict b14 step computed at the state after b12.
  - B, the blocking cycle {130, 180} at the depth 60 stop (R22C-03, X22C-07): T1′ after each unit's one-letter
    readiness letter, (481, 5) for 130 and (481, 1) for 180. Landing words exist (432 letters each; 61 and 56 pieces
    land), but after W 48 and 32 chambers of Bad(W⁻¹) are occupied and fixed by every SAFE letter, so T1′ fails its
    vacancy premise for these W. This agrees with R22C-03 and says nothing about other W.
  - C, two-letter readiness (r = 2) at the depth 60 stop for units 27, 36, 44 and 59 (X22C-09;
    `scripts/round22/x22c09_r2_ready.py`, outputs `runs/round22/line-c/r2-<unit>.txt`): the first letter at a cap that
    holds a straddler of the unit's inside caps, the second any retained letter. Unit 36 is ready after (255, 1) then
    (459, 1), found after 34 of its 66 first letters; units 44 and 59 have no ready pair (all 66 and 33 first letters
    tried), and unit 27 has no first letter of this shape. Readiness is not vacancy: a ready pair is a candidate for a
    closure, not a step.
- Line (c), round 2 (the senior reviewer alone, after the solver stopped; R22C2-01 to R22C2-06; the integrator re-ran
  its two computations and every computation of the sealed analyses and the cross-examination, with the same
  outputs; `scripts/round22/`, `runs/round22/`):
  - None of the three failed T1′ closures (b15 in experiment A, 130 and 180 in experiment B) has a landing chamber
    fixed by the displaced lattice content alone; each fixed chamber also has a cap blocked by other units: b07, b11
    and b13 at cap 286 for b15, 131, 132, 133, 142, 150, 151 and 163 at cap 429 for 130, and 174, 175 and 176 at cap
    555 for 180 (R22C2-02). So these failures depend on the order of the steps, not on the unit alone. When the
    displaced content alone fixes a landing chamber, no landing word of the unit's class has a T1′ closure, at any
    length (R22C2-01, the intrinsic Q-shadow; a finite test).
  - Every exact candidate of the edge-cycle family acts on the cell by one and the same rotation, so no other
    candidate of that family closes 130 or 180 after the same readiness letter (R22C2-03). Of b06's twelve landing
    rotations eleven leave no landing chamber fixed, but every T1′ for b06 keeps its type-6 deficit and the
    commutator transports cannot move a vertex content; b06 needs a partner and a new middle (R22C2-05). Coverage is
    stated as backward certification: starting from the lattice states, add every state with an exactly replayed
    catalogue step into a certified state; a state with n off pieces that is ever certified is certified within n
    stages, and what is missing is that every reachable height-2 state is certified (R22C2-06).
  - Experiment A continued (R22C2-04; `runs/round22/line-c/b14-after-b12/`): b14's census word, closed at the state
    after b12's step by V·W·V⁻¹. N1 and N2 hold for its Bad(W) (222 chambers, 17 occupied); the peeling certificate
    stops at 36 of them and a 1,800-second greedy search leaves 5 occupied, so V is not built; all 19 orbits that
    meet Bad(W) have Jordan certificates (`scripts/round22/jordan_target.py` on that state's pose table), so V exists
    by Corollary A1 and b14 has a strict′ step after b12. The builder of E22-1b then built that V at the same state
    (`b14-cert.json.gz`, about 2 minutes): 222 targets in 19 orbits, 51 steps, a commutator seed of 420 letters on
    orbit 34 and Jordan-route seeds of 117 to 157 digits on the other 18, tables with e ≤ 15 and f ≤ 23, and
    |V| < 10^158 within its bound. The checker passes it (`b14-check.txt`), so b14's strict′ step after b12 is
    constructed, as a checked program. b15 needs the state after that step, which needs V on all chambers, not only
    on the target orbits; that is not computed.
  - B continued (R22C2-03): T1′ for 130 after (496, 1) and for 180 after (481, 2), the preparations R22C2-03 does
    not cover, fails the same way. The landing words (432 letters each) land all 61 and 56 pieces, but after W 61 and
    22 chambers of Bad(W⁻¹) (86 chambers; 61 and 56 occupied) are occupied and fixed by every SAFE letter, so N1
    fails (`runs/round22/line-c/exp-b2.json`). Both preparations tried for each unit fail.
  - D, an order at the depth 60 stop (R22C2-02; `scripts/round22/exp_d.sh`, `runs/round22/line-c/exp-d/`): first
    174, 175 and 176, the units that block 180's cap 555, each by a strict′ step without endpoint collateral that the
    transport census finds at the current state (24, 24 and 132 letters; 14,610 to 14,515 off pieces; each replayed
    by `stepchk.py`). Then 180: without a preparation its landing word (360 letters) has no T1′ closure (N1 and N2
    fail). After the readiness letter (481, 1) the landing word (288 letters) lands all 56 pieces; N1 and N2 hold for
    T = Bad(W⁻¹) (86 chambers, 56 occupied), a 300-second greedy search leaves 8 occupied, and all 19 orbits that
    meet T have Jordan certificates, so V′ exists by Corollary A1. The builder of E22-1b built V′ there (56 steps,
    Jordan-route seeds of 117 to 162 digits, e ≤ 63, f ≤ 170, |V′| < 10^162, about 5 minutes) and the checker passes
    it (about 20 minutes). So after 174, 175 and 176, 180 has a strict step (T1′ under the retained conjugation of
    R22D-01): its blocking came from the order of the steps. 130 was not tried; the state after 180's step needs V′
    on all chambers.
- Line (c), round 3 (the senior reviewer alone; R22C3-01 to R22C3-06 in `PROOFS-B.md` section 7; its computations
  `scripts/round22/r22c3_audit_d.py`, `r22c3_u151.py` and `r22c3_geometry.py`, re-run by the integrator with the same
  outputs, `runs/round22/`):
  - Strict′ steps only enlarge the SAFE group, but an orbit's hole count can fall, by an exact inventory identity
    (R22C3-01). So an order needs, besides the blocker order, a resource contract at each stage, all finite checks
    (R22C3-02, the resource-certified order).
  - Experiment D passes an independent audit; 180's step is T1′ applied to an exactly evaluated straight-line
    program, not a literal replay, and its end pose table is not computed (R22C3-03).
  - At the depth 60 stop 142 lands by a pure-cell step (Theorem C) and then 151 by T1′, whose vacancy survives 142's
    landing with a slack of at least 24 holes per orbit: 14,610 to 14,524 off pieces in at most
    2,160 + 2(7,200^75 − 1) letters, proved from computed finite hypotheses, no word built (R22C3-04). 130 still
    waits for 131, 132, 133, 150 and 163.
  - The unresolved units and the routes proposed for them: b05, b06 and b15 after certified orders, b17 after b08,
    b09 and b10 (since done, line (a) round 3), and the 32 E-B4 leftovers at the depth 60 stop; run 2 is not a
    strict′ order, since steps of it move other off blocks (steps 0 and 2, R22C3-05; step 9 did so later). Every
    native free-class core has at
    most 13,205! − 1 letters, which makes the bounds of the catalogue explicit; coverage of 𝒞₂ stays open
    (R22C3-06).
- E22-2 (`scripts/eb6r.py`, written by Codex and reviewed by the integrator; its acceptance check
  `scripts/check_eb6r.py` passes): the complete search of R22D-01 with one retained letter r at the depth 100 stop.
  Of the 2,574 retained letters at the 234 clean grips, an exact rule prunes 1,309 (no piece of a high unit moves,
  and no inside or straddler set changes at a cap the search reads), so 1,265 prepared states are searched, at about
  a minute each; every kept word is replayed as r·C·r⁻¹. The 22 letters of grips 1 and 61 (the acceptance run) give
  no strict word. The full search ran in eight shards on two cores and is complete: shards 0 to 7 (322 letters each
  for 0 to 5, 321 for 6 and 7; 160, 163, 163, 165, 158, 152, 149 and 155 searched after pruning; 71 to 87 minutes
  each) give no strict word and no strict lowering word (`runs/round22/e22-2/`). So at the depth 100 stop one
  retained letter before an E-B6 candidate never gives a strict step.

## 12. Next

- Coverage of B′(2), the one step the proof still lacks: every reachable height-2 state has a step in the catalogue.
  Vacancy at a given middle is settled (R22B-29: N1, N2 and the joint target-image test, finite and without Alt), so
  coverage is what remains (`PROOFS-B.md` section 8). E2 took the depth 40 stop to the lattice in 21 strict′ steps,
  each found at the state in front of it, so no state along that trajectory was a dead end; it is one trajectory.
  Round 2 reduced coverage to one finite per-state predicate, Cover(Z) (COV-15), and refuted two shortcuts: resources
  do not persist across steps (COV-09), and not every reachable state has a cap whose whole off content lands
  (COV-13). Round 3 showed that reachable states with one off piece exist in every kind of cell and that the
  catalogue covers them (COV-17, COV-18, COV-21). Three parts remain. (1) Whether the isolated triangle of COV-01 is
  reachable, or an invariant that excludes it: no abelian invariant can (COV-19), and one family of words misses it
  (COV-20); the experiment COV-22 is running, and only a hit decides. If it is reachable, the geometric templates and
  the whole-cap route all fail there, and the inverse of its reaching word is the one native core it needs (COV-01,
  COV-14, COV-21). (2) The lemma that every state of 𝒞₂ with off pieces satisfies Cover(Z), with the resource contract
  of COV-08 and the preparation test of COV-10 as its tools. (3) Unit 130 at the depth 60 stop after 131, 132, 133,
  150 and 163.
- Line (a): the replay of the depth 40 stop is finished (E2, 1,193 to 0). The other 𝒞₂ census stop on record is the
  depth 60 stop `ia-k60-s7104`; the census round 3 records are deeper and have height-4 pieces, so they are not 𝒞₂
  inputs (COV-16). A further replay adds a trajectory and decides nothing universal.
- Line (b): whether N1 and N2 alone suffice at every target of 𝒞₂ (no step needs it once it carries the joint test
  of R22B-29), and the residual primitive case of R22B-25. The commutator-power search of R22B-18 stays deferred:
  short seeds shrink V's bound but do not decide existence.
- Existence before construction: a stuck step is settled by N1, N2 and the Jordan certificates of its target
  (`jordan_target.py`); V is built with the builder of E22-1b only when a later state is needed.
- Height 4 (lowest priority): E22-2 is complete and gives no strict step; next, six-letter commutators (R22D-02) and
  protected two-level steps with a smaller Bad set than a whole cap (R22D-05).
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
    python3 -I -B $S/eb4_tries.py <work dir> <out>

`eb4_close.py` writes `close.json` and, per unit k, a directory `u<k>/` with the tables, logs and words of its stages;
`step-word.json` there is the step at Z.

E-B5 run 1, with `C` as above and `G=research/jumbling/restoration/09-general-method/scripts` (`k40.json` is the
depth 40 stop as a pose table, digest `7e128820…`):

    python3 -I -B $S/export_table.py $C/ia-k40-s7101.json k40.json                # about 20 seconds
    python3 -I $G/gm/gdrive.py research/jumbling $G/echain k40.json eb5-k40 <work dir> \
        --par 2 --heuristic-seconds 1800 --max-steps 30 --fallback-max-length 1     # about 1 hour

The driver writes its record to `<work dir>/gdrive.jsonl`; `runs/eb5-k40-run1.jsonl` is that record with the private
directory of the start table written as `<working tree>`, as in the stage 09 run records.

E-B5 run 2, with the same `k40.json`, `cellpure5.py` as the pure-twist script and run 1's work directory as seeds:

    python3 -I $G/gm/gdrive.py research/jumbling $G/echain k40.json eb5-k40 <work dir 2> \
        --par 4 --heuristic-seconds 3600 --max-steps 30 --fallback-max-length 1 --pure-max 22 \
        --pure-script research/jumbling/restoration/12-low-height/scripts/cellpure5.py \
        --seed <work dir>/drv                                                        # about 16 hours

`runs/eb5-k40-run2.jsonl` is its record, with the private directories written as `<working tree>`.

E-B6, with `C` and `R` as above and `X=research/jumbling/restoration/10-x1-unbounded/witnesses/H6030.json`:

    python3 -I -B $S/eb6_meet.py <out>                                              # about 10 seconds
    python3 -I -B $S/eb6.py witness $X 156 <out> --comm4 --units 118527             # about 20 minutes
    python3 -I -B $S/eb6.py record $C/ia-k100-s7102.json <out> --comm4              # about 3 minutes
    python3 -I -B $S/eb6.py fixture 841 <out> --comm4 --comm4-seconds 5000          # about 16 minutes
    python3 -I -B $S/eb6_replay.py $R/eb6-X.json witness $X 156 <out>               # about 2.5 minutes

LC-13 (section 10), with `G` and `k40.json` as for E-B5, `L=research/jumbling/restoration/12-low-height/runs/lc13-k40`
and `u` one of `b00`, `b01`, `b02`:

    python3 -I -B $S/cellpure5.py research/jumbling/theory k40.json $L/units/eb5-k40-k00-$u.json t-$u.json --search   # 3 to 4 minutes
    python3 -I $G/gm/tclose2.py research/jumbling/theory $G/echain k40.json $L/units/eb5-k40-k00-$u.json w-$u.json \
        c2-$u --greedy-seconds 300                                                  # 4 to 7 minutes
    python3 -I $G/gm/tclose2m3.py research/jumbling/theory $G/echain k40.json $L/units/eb5-k40-k00-$u.json w-$u.json \
        c3-$u --greedy-seconds 300                                                  # after 'no V'; 10 to 12 minutes
    python3 -I -B $S/eb4_replay.py k40.json $L/steps <out>                         # 14 to 34 seconds per step

`cellpure5.py` writes `t-$u.json` and the word `t-$u-word.json`; `w-$u.json` is `{"P": [], "Q": <that word>}`, as
the driver writes it. The unit files are those the driver writes for the start state (`<work dir>/tg/eb5-k40-k00/`).
`steps/u1.json` is `word-full.json` of the closure `c2-b01`, and `steps/u2.json` that of the closure of b02 run with
`--greedy-seconds 3600` (about 10 minutes). `steps/u0.json` is b00's step without endpoint collateral (section 11),
found by the scratch census. `eb4_replay.py` replays each step from the start state on a fresh simulator;
`replay-b00.json`, `replay-b01.json` and `replay-b02.json` are its records for one step each.

Theory round 22 (section 11): the exact computations of the senior review are listed in [`PROOFS-B.md`](PROOFS-B.md)
section 6 (`scripts/round22/`, outputs in `runs/round22/`), with the commands of the line (a) census and of the four
entry-transport closures (`runs/round22/line-a/`), the computation of line (a)'s second round
(`scripts/round22/r22a2_04_sequence.py`), the computations of line (c) and the continuation of experiment A
(`runs/round22/line-c/b14-after-b12/`), and those of the fourth round of line (b) (`r22b20_separation.py`,
`r22b21_regular12.py`, `r22b22_small_orbits.py`) and the third of line (c) (`r22c3_audit_d.py`, `r22c3_u151.py`,
`r22c3_geometry.py`), each kept as written apart from a header comment.

Line (c), the integrator's re-check of the solver's four T1′ steps (section 11), with `k40.json` as for E-B5 and
`Q=research/jumbling/restoration/12-low-height/runs/round22/t1prime-k40`:

    python3 -I -B $S/eb4_replay.py k40.json $Q/steps <out>                         # about 3 minutes

Each `steps/u<k>.json` is the solver's step for unit k: its landing word W (`cellpure5.py`) followed by the closure
word V′·W⁻¹·V′⁻¹ (`tclose2.py` at W(Z), with W⁻¹ as the transport to close); `replay.json` is the replay record.

Line (c), the adjudicating experiments A and B (section 11), with `k40.json` as for E-B5 and `k60.json` as for E-B4:

    sh $S/round22/exp_a.sh k40.json <work dir>                                     # about 10 minutes
    python3 -I -B $S/round22/exp_b.py k60.json 130 481 5 <work dir>                # about 8 minutes
    python3 -I -B $S/round22/exp_b.py k60.json 180 481 1 <work dir>                # about 8 minutes
    python3 -I -B $S/round22/exp_b.py k60.json 130 496 1 <work dir>                # about 12 minutes
    python3 -I -B $S/round22/exp_b.py k60.json 180 481 2 <work dir>                # about 10 minutes

`runs/round22/line-c/exp-a.json`, `exp-b.json` and `exp-b2.json` (the other preparations) keep the outcome of each
step: the strict′ rows, the landing words' exact checks, and the vacancy premises of each closure with the occupied
chambers that every SAFE letter fixes. The landing words are `b15-W.json`, `u130-W.json`, `u180-W.json`,
`u130-496-1-W.json` and `u180-481-2-W.json`.

Line (c), experiment D (section 11), with `k60.json` as for E-B4; the Jordan check, the certificate of V′ and its
check, and the names under which `runs/round22/line-c/exp-d/` keeps the driver's outputs, are in
[`PROOFS-B.md`](PROOFS-B.md) section 6:

    sh $S/round22/exp_d.sh k60.json <new work dir>                                 # about 2 hours

E22-2 (section 11), shard i of 8 (i = 0 to 7; 75 to 87 minutes each on one core):

    python3 -I -B $S/eb6r.py record $C/ia-k100-s7102.json <out>/shard-i.json --shard i/8 --seconds 86400 > <out>/shard-i.out 2> <out>/shard-i.log

`runs/round22/e22-2/` keeps, for each finished shard, its final line (`shard-<i>-result.json`: letters, searched,
pruned, strict words, replayed words, whether the shard completed) and its per-letter log (`shard-<i>.jsonl.gz`).

E22-1 (section 11), with `T=research/jumbling/restoration/12-low-height/runs/lc13-k40/target-b00.json`:

    python3 -I -B $S/test_r22b01.py                                                # seconds
    python3 -I -B $S/r22b01_cert.py $C/ia-k40-s7101.json $T <out> --seconds 900   # 15 minutes
    python3 -I -B $S/check_r22b01.py                                               # about 17 minutes

The builder writes `complete: false` with the reason for each missing orbit when it does not finish; the check then
fails.

E22-1b (section 11), with `T=research/jumbling/restoration/12-low-height/runs/lc13-k40/target-b00.json` for b00 and
`T=research/jumbling/restoration/12-low-height/runs/round22/line-a/closures/b19-target.json` for b19:

    python3 -I -B $S/test_r22b10.py                                                 # about a minute
    python3 -I -B $S/r22b10_cert.py record $C/ia-k40-s7101.json $T <out>.json.gz --seconds 7200   # about 5 minutes
    python3 -I -B $S/check_r22b10.py record $C/ia-k40-s7101.json $T <out>.json.gz   # 9 to 15 minutes

`runs/round22/e22-1b/` keeps both certificates (`b00-cert.json.gz`, `b19-cert.json.gz`), the builder's logs
(`-build-log.txt`: one line per orbit with its route, seed length s and table depths e and f) and the checker's
outputs (`-check.txt`, ending in PASS). With the fixed seed 2210 a rerun gives the same certificate except the timing
fields.

b14's certificate after b12 (line (c), experiment A continued, section 11), with `za.json` the state after b12's step
as a pose table (written by `stepchk.py`; the commands are in `PROOFS-B.md` section 6) and
`L=research/jumbling/restoration/12-low-height/runs/round22/line-c/b14-after-b12`:

    python3 -I -B $S/r22b10_cert.py table za.json $L/b14-target.json <out>.json.gz --seconds 3600   # about 2 minutes
    python3 -I -B $S/check_r22b10.py table za.json $L/b14-target.json <out>.json.gz                 # about 7 minutes

`$L` keeps the certificate (`b14-cert.json.gz`), the builder's log (`b14-build-log.txt`) and the checker's output
(`b14-check.txt`, ending in PASS).

Line (a), round 3, the closure of b10 after b08 (section 11), with `k40.json` as for E-B5,
`A=research/jumbling/restoration/12-low-height/runs/round22/line-a` and `G` as for LC-13:

    python3 -I -B $S/round22/stepchk.py k40.json $A/units/eb5-k40-k00-b08.json $A/closures/b08-step.json <out>/b08.json <out>/z-b08.json
    python3 -I $G/gm/tclose2.py research/jumbling/theory $G/echain <out>/z-b08.json $A/units/eb5-k40-k00-b10.json $A/closures/b10-core.json <out>/c-b10 --greedy-seconds 300   # about 6 minutes
    python3 -I -B $S/round22/stepchk.py <out>/z-b08.json $A/units/eb5-k40-k00-b10.json <out>/c-b10/word-full.json <out>/b10.json

`runs/round22/line-a/b10-after-b08/` keeps the two replay records (`b08-replay.json`, `b10-replay.json`), b10's step
(`b10-step.json`), the closure's result and log (`b10-result.json`, `b10-log.txt`).

Line (a), round 3, the order of R22A2-04 at the actual states and b17 (section 11), with `k40.json` as for E-B5; the
Jordan checks, the premises of Theorem C, b17's search and replay and the T1′ tests at 737 are in
[`PROOFS-B.md`](PROOFS-B.md) section 6:

    bash $S/round22/replay_a3.sh k40.json <new work dir>                           # about 2 hours

`runs/round22/line-a/replay/` keeps the sequence (`seq.jsonl`), the words of the eight built steps (`words/`), their
replay records and those of the census words that were not strict′ (`steps/`), the closures' results (`closures/`),
the pose tables after b02, b11, b10 and b17 (`tables/977.json.gz`, `915`, `823` and `737`), the Jordan targets and
checks (`jordan/`), the premises of Theorem C for b17 (`b17/`) and the T1′ tests at 737 (`t1prime-737.jsonl`).

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
| `scripts/eb4.py`, `scripts/eb4_close.py`, `scripts/cellpure4.py`, `scripts/eb4_replay.py`, `scripts/eb4_tries.py` | E-B4: the structural stage and N1 and N2 at Y; the closures and the steps at Z; the class-cell pure twist with the exact-pose filter; the independent replay; the filter's tries at each closure |
| `runs/eb4-k60.json`, `runs/eb4-close-k60.json`, `runs/eb4-k60-steps/u<k>.json`, `runs/eb4-replay-k60.json`, `runs/eb4-exact-tries-k60.json` | E-B4 records: stages 1 and 2 (`eb4.py`), the closures and the checked steps (`close.json` of `eb4_close.py`), the 20 step words with input and output digests, the replay record, and the exact-pose filter's tries with the digest of each state Y |
| `runs/eb5-k40-run1.jsonl` | E-B5 run 1: the driver's event record (no step) |
| `runs/eb5-k40-run2.jsonl` | E-B5 run 2: the driver's event record (twelve steps, 1,193 to 377 off pieces) |
| `scripts/eb6.py`, `scripts/eb6_replay.py`, `scripts/eb6_meet.py` | E-B6: the high units of a state, their caps and lens pairs, and the exhaustive search for strict four-letter commutators (Lemma E6); the independent replay of the words it keeps; the exact check of which caps can meet |
| `runs/eb6-X.json`, `runs/eb6-k100.json`, `runs/eb6-fx841.json`, `runs/eb6-replay-X.json`, `runs/eb6-meet.json` | E-B6 records at X (the commutator search for K only), the depth 100 stop and the fixture end; the replay record at X; the angles at which caps can meet. A rerun reproduces them except the timing fields, as long as `--comm4-seconds` lets the search reach every unit |
| `scripts/cellpure5.py` | LC-13: `cellpure4.py` with a transport branch for a content that is not in an edge cell (commutator powers into a home cell of its kind, exact-pose filter) |
| `runs/lc13-k40/` | LC-13 at the depth 40 stop: the three unit files; the transport records and words with input and output digests (`transport-<u>.json`, `transport-<u>-word.json`); the closure records of b01 and b02 (`close-b01.json`, `close-b02.json`) and their step words (`steps/u1.json`, `steps/u2.json`); b00's step without endpoint collateral (`steps/u0.json`); the replay record of each step (`replay-b00.json`, `replay-b01.json`, `replay-b02.json`); and, for b00 and b02, the 300-second closures' Bad(W), vacancy, certificate and greedy fields (`close-no-v.json`, taken from their `pre.json` and `result.json`) |
| `PROOFS-B.md` | Theory round 22: the lemmas of lines (a), (b) and (d) toward B′(h), each with its hypotheses as finite checks at the state, its proof, its bound and its instances |
| `ARCHON-HANDOFF.md` | Draft handoff to the Lean formalization (not started; the owner decides when): the objects, the 81 results of round 22 in dependency order, the hypotheses that are computations with what decides them, and the results of earlier stages and of the literature that the proofs use |
| `scripts/round22/`, `runs/round22/` | The senior reviewer's exact computations of round 22, re-run by the integrator, and their outputs |
| `scripts/round22/transport_census.py`, `runs/round22/line-a/` | Line (a): the transport census with whole cells as owners (vacant targets, entry transports, exact replays); its records for class 3 and for classes 2 and 4 (`census-class3.json`, `census-class24.json`, words included); the 22 unit files of the depth 40 stop; and the closures of the four entry transports (`closures/<u>-core.json`, `-step.json`, `-result.json`, `-replay.json`, and `-log.txt`, the closure's log with the certificate bounds and, for b19, where the peeling and the greedy search stopped); b19's Bad(W) as a target file and its Jordan certificates (`closures/b19-target.json`, `b19-jordan.txt`, from `scripts/round22/jordan_target.py`) |
| `scripts/round22/r22a2_04_sequence.py`, `runs/round22/r22a2_04_sequence.txt` | Line (a), round 2: the senior reviewer's computation (kept as written apart from a header) and its output: the fourteen middles with their Bad sets and ghost maps, the composition matrix of the thirteen supplied steps, the hole-count search for fresh closures with its order, the construction bounds, and N1 and N2 for the glued units' own cells |
| `runs/round22/t1prime-k40/` | Line (c): the solver's four T1′ steps at the depth 40 stop (`steps/u3.json`, `u4.json`, `u12.json`, `u18.json`, words with input and output digests) and the integrator's replay record (`replay.json`) |
| `scripts/eb6r.py`, `scripts/test_eb6r.py`, `scripts/check_eb6r.py` | E22-2: the E-B6 search after one retained letter, with the exact classification update of R22D-01, exact pruning and the replay of every kept word r·C·r⁻¹; its unit test; its acceptance check at the depth 100 stop |
| `scripts/round22/stepchk.py`, `scripts/round22/prep_t1prime.py`, `scripts/round22/exp_a.sh`, `scripts/round22/exp_b.py` | Line (c), the adjudicating experiments A and B: one step replayed from a pose table against the strict′ contract, the preparation of a T1′ closure (the solver's computation, given arguments), and the two experiments (`exp_b.py` also takes the unit as a unit file and `-` for no preparation, as experiment D uses it) |
| `runs/round22/line-c/` | Their records: the unit files of b12, b14 and b15, b14's census word, the five landing words, and the outcomes (`exp-a.json`, `exp-b.json`, and `exp-b2.json` for the other preparations of 130 and 180) |
| `runs/round22/e22-2/` | E22-2: for each finished shard, its final line (`shard-<i>-result.json`) and its per-letter log (`shard-<i>.jsonl.gz`) |
| `scripts/round22/r22c02_locks.py`, `r22c03_locks60.py`, `r22c04_vectors.py`, `r22c04_orbits.py`, `r22c04_b06.py`, `x22c04_s61c.py`, `x22c07_movers.py`, `r22c2_02_blockers.py`, `r22c2_05_vertex.py`, `r22cf02_shadow.py` | Line (c): the computations of the senior reviewer's sealed analysis, cross-examination and second round, and of the solver's sealed analysis (`r22cf02_shadow.py`), each kept as written apart from a header; their outputs, re-run by the integrator, are `runs/round22/<name>.txt` and `runs/round22/line-c/shadow/` |
| `scripts/round22/x22c09_r2_ready.py`, `runs/round22/line-c/r2-<unit>.txt` | Line (c), experiment C: two-letter readiness at the depth 60 stop for units 27, 36, 44 and 59 |
| `runs/round22/line-c/b14-after-b12/` | Line (c), experiment A continued: b12's replay record, b14's core, the closure's log and result at the state after b12, the target file and Jordan check of its Bad(W) (`jordan_target.py` with that state's pose table), and b14's V as a certificate of R22B-10 with the builder's log and the checker's output |
| `scripts/round22/exp_d.sh`, `scripts/round22/exp_d_units.py`, `scripts/round22/exp_d_pick.py` | Line (c), experiment D: the driver (three steps found and checked at the current state, then T1′ for 180 with `exp_b.py`), the unit files at a state with the search for a unit by its pieces, and the choice of a census step or a core to close |
| `runs/round22/line-c/exp-d/` | Line (c), experiment D: the unit files, the census picks, the three step words and their replay records, 180's outcome without a preparation and after (481, 1), and `e180-cert/`: the target file, the Jordan check, V′ as a certificate of R22B-10, the builder's log and the checker's output |
| `scripts/r22b01_cert.py`, `scripts/test_r22b01.py`, `scripts/check_r22b01.py`, `runs/lc13-k40/target-b00.json` | E22-1: the bounded builder of the R22B-01 certificate (seeds, letter tables and V; it records what it completed and why each other orbit is missing), its unit test on small exact alphabets, its acceptance check at the depth 40 stop, and b00's transport target T (111 chambers, with its digest) |
| `scripts/r22b10_cert.py`, `scripts/check_r22b10.py`, `scripts/test_r22b10.py` | E22-1b: the builder of the R22B-10 certificate (seeds by the commutator or Jordan route, Schreier tables with word edges, every word an acyclic straight-line program with exact lengths), its independent checker, and its unit test with the controls of R22B-10 |
| `runs/round22/e22-1b/` | E22-1b: the certificates of b00 and b19 at the depth 40 stop, the builder's logs and the checker's outputs |
| `scripts/round22/r22b13_seeds.py`, `runs/round22/r22b13_seeds.txt` | Line (b), round 3: the senior reviewer's computation (kept as written apart from a header) and its output: two short seeds at b00 checked in the full order, the orders of all SAFE letters, an audit of the E22-1b certificates' lengths and bounds, and E(7,200), F(7,200) |
| `runs/round22/line-a/b10-after-b08/` | Line (a), round 3: the closure of b10 at the state after b08's step and the replay records of both steps |
| `scripts/round22/replay_a3.sh`, `runs/round22/line-a/replay/` | Line (a), round 3: the order of R22A2-04 replayed at the actual states (census word or fresh closure at each state, every built step replayed); its sequence, words, step records, closure results, pose tables, Jordan targets and checks, b17's records and the T1′ tests at 737 |
| `scripts/round22/thmc_premises.py` | Line (a), round 3: the premises of Theorem C for an edge unit at a pose table (whole cell content, vacant home chambers, free poles and a free common neighbour, one landing rotation) |
| `scripts/round22/r22b20_separation.py`, `r22b21_regular12.py`, `r22b22_small_orbits.py`, `runs/round22/r22b20_separation.txt`, `r22b21_regular12.txt`, `r22b22_small_orbits.txt` | Line (b), round 4: the senior reviewer's computations (kept as written apart from a header) and their outputs: separating words at b00, the regular orbit of 12 chambers, the small orbits of the depth 60 stop |
| `scripts/round22/r22c3_audit_d.py`, `r22c3_u151.py`, `r22c3_geometry.py`, `runs/round22/r22c3_audit_d.txt`, `r22c3_u151.txt`, `r22c3_geometry.txt` | Line (c), round 3: the senior reviewer's computations (kept as written apart from a header) and their outputs: the audit of experiment D, 151's pure endpoint and vacancy, the geometry of 142 and b17 |
| `scripts/round22/r22b24_regular_hole.py`, `r22b26_small60.py`, `r22b27_small_t1prime.py`, `runs/round22/r22b24_regular_hole.txt`, `r22b26_small60.txt`, `r22b27_small_t1prime.txt` | Line (b), round 5: the senior reviewer's computations (kept as written apart from a header) and their outputs: an orbit without Alt that meets a target with one hole, the small orbits against the depth 60 home targets, and the same at the T1′ middles of 151 and 180 |
| `scripts/round22/cov01_isolated_triangle.py`, `cov02_point_orbits.py`, `cov04_middles737.py`, `cov07_word_order.py`, `runs/round22/cov01_isolated_triangle.txt`, `cov02_point_orbits.txt`, `cov04_middles737.txt`, `cov07_word_order.txt` | Coverage, round 1: the senior reviewer's computations (kept as written apart from a header) and the integrator's run of its proposed experiment, with their outputs |
| `scripts/round22/cov09_resource_persistence.py`, `cov13_mixed_frames.py`, `cov14_isolated_wholecap.py`, `runs/round22/cov09_resource_persistence.txt`, `cov13_mixed_frames.txt`, `cov14_isolated_wholecap.txt` | Coverage, round 2: the senior reviewer's computations (kept as written apart from a header), with the integrator's re-run outputs |
| `scripts/round22/cov17_single_off.py`, `cov18_singleton_kinds.py`, `cov19_perfect_menu.py`, `cov22_mixed_search.py`, `runs/round22/cov17_single_off.txt`, `cov18_singleton_kinds.txt`, `cov19_perfect_menu.txt` | Coverage, round 3: the senior reviewer's computations and its experiment (kept as written apart from a header), with the integrator's re-run outputs |
| `scripts/round22/cov20_all_landers.py`, `cov20_two_level.py`, `runs/round22/cov20_all_landers.txt` | Coverage, round 3: the integrator's checks after COV-20, every one-letter SAFE sandwich over the 36 landing letters and the second level for two of them |
| `scripts/slp_apply.py`, `scripts/test_slp_apply.py` | The exact applier of a program of literal words, `tclose2.py` cores and R22B-10 certificates (straight-line programs of SAFE-ALL letters, applied to the labelled poses without expanding powers): the end pose table and the strict′ contract against the start state; its unit test |
| `scripts/round22/replay_slp.py`, `runs/round22/line-a/replay2/` | E2, the replay continued from 737: the driver (literal words, closures with certificate V, T1′ steps with a literal or certificate V′, each applied by `slp_apply.py`); the plans, the stored landing words and cores (`inputs/`), and runs 1 and 2 with every step's files and end tables |
| `scripts/round22/e2_b06_prep1.py`, `e2_b06_prep2.py`, `e2_b06_t1.py` | E2, b06 with b05: the first SAFE-ALL preparation, the second counted against the reference middle's orbits, and the prepared T1′ attempt applied by `slp_apply.py`; records in `runs/round22/line-a/replay2/b06-<n>/` |
