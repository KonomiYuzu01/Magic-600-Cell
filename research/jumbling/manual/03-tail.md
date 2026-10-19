# 3. The tail (outline)

Status: **outline.** This chapter will be written when stage 12 (option B) has settled which of these steps a person
can carry out. Until then it names the techniques and points to the research records. Nothing here is a
recommendation to the solver yet.

## 3.1 What the tail is

After the descent, the pieces still off the lattice sit in **blocks**: pieces that no cut separates, so they move
together. At the stop of the census run of depth 60, 75% of the 14,610 off pieces lay in blocks of 111, 86 and 87
pieces, and 15% lay in blocks that no cap can move, because every cap that holds them whole is blocked (measured,
[census](../restoration/11-scramble-census/README.md), "The residue at the end of round 2"). A block that can move
often still cannot go home: the chambers it needs are held by other blocks, which are held up in turn.

## 3.2 Clean steps: vacate, act, put back

A **clean step** lands one unit on the lattice. A unit is a block, or one frame part of a block that lies in several
frames. After the step, the unit and every piece that was on the lattice are on it, and every other off piece has
its exact pose, except the pieces of the step's declared **free set**, which may move or land too. The usual form is
V·W·V⁻¹: V empties the chambers W needs, W puts the unit on the lattice, and V⁻¹ undoes V.
- A clean step lowers the number of off pieces by at least the unit's size (proved, stage 08 Lemma U), so clean
  steps never go round in circles.
- Off the lattice every unit has a clean step whose free set is all the other off pieces, and an exhaustive search
  that reads only the configuration finds one. Nothing bounds its length (proved, Theorem M\*,
  [senior review](../restoration/09-general-method/SENIOR-REVIEW.md)).
- Whether a **strict** clean step, with an empty free set, always exists is open
  ([general method](../restoration/09-general-method/METHOD.md), gap M1).
- **Steps found at one configuration need not combine.** A strict step found earlier stays strict after another step
  exactly when the other step's landing leaves the chambers the first step needs empty (proved, R22A2-02 in the
  [stage 12 proofs](../restoration/12-low-height/PROOFS-B.md)). At the depth 40 stop, 52 of the 156 ordered pairs of
  thirteen steps computed there fail, and at most six of them combine unchanged (computed, R22A2-03). Rebuilt at each
  new configuration, in a checked order, fourteen units land (proved, with hypotheses computed at the stop,
  R22A2-04). Build each step at the configuration in front of you.

## 3.3 The shape catalogue

Three block shapes have stored template words (stage 06), which the rotation group K⁺ carries to any position of the
shape. On the census stops the stored words alone cleared 1.5% (depth 60) and 0.4% (depth 100) of the off pieces.
Most words need chambers that other pieces hold, or move other blocks (measured,
[census](../restoration/11-scramble-census/README.md), "Template test"). A catalogue is not enough without
vacating.

## 3.4 Vacating

Finding V is the hard part. A peeling certificate gives an order in which the chambers that W needs can be emptied
([general method](../restoration/09-general-method/METHOD.md)). At the depth 40 stop of the census, a tetrahedron
unit closed this way, in a step of 624 twists. For the two triangle units W needs a whole triangle home cell of 111
chambers, and the certificate stops after about a third of them; a longer greedy search found an 18-twist V for one
of them, which then closed in a step of 48 twists (measured, [stage 12](../restoration/12-low-height/README.md),
section 10). The other one needed no V (section 3.5).

- **When V exists.** Suppose every orbit of the safe moves that meets the chambers W needs carries the full
  alternating group, which a finite Jordan certificate shows, and has at least as many empty chambers as W needs
  there (N1 and N2). Then V exists (proved, Corollary A1 of [buffer transport](../theory/buffer-transport.md)),
  with an explicit bound on its length (proved, R22B-01 and R22B-05 to R22B-08 in the
  [stage 12 proofs](../restoration/12-low-height/PROOFS-B.md)). At the depth 40 stop a builder now constructs such a
  V and a separate checker verifies it: for two units, words of fewer than 10^166 and 10^141 twists, kept as
  compressed programs (measured, [stage 12](../restoration/12-low-height/README.md), E22-1b). They settle existence
  and are far beyond what a person can carry out.
- **V as a word to play.** Built from short seeds, every V that the depth 40 stop needed became a literal word: its
  tail went from 737 off pieces to 0 in 13 steps of 692 to 672,417 twists, 2,012,155 in all, each played as one word
  (measured, R22B-31 and R22B-34 in the [stage 12 proofs](../restoration/12-low-height/PROOFS-B.md)). A machine plays
  these words; a person cannot.
- **The V a person uses** comes from a peeling certificate or a short search. After one triangle unit had landed in
  a home cell, a second triangle unit whose transport enters the same cell got a V of 396 twists, in a step of 936
  twists (measured, R22A2-05 in the [stage 12 proofs](../restoration/12-low-height/PROOFS-B.md)).

## 3.5 Turning one cell alone, and transport

- **The pure twist** turns the cell of one carrier piece and nothing else: at most 720 twists when the two poles of
  the carrier's edge cell and one common neighbour are free, at most 104 for a vertex cell. Pieces in triangle and
  tetrahedron cells cannot be turned in place. Transport words land them in another cell of the same kind instead,
  with at most 240 and 192 twists (proved, Theorem C and its extension,
  [senior review](../restoration/09-general-method/SENIOR-REVIEW.md); the bounds assume that the grips of the
  cell's class are free).
- At the depth 40 stop the transport words found were single commutator powers of 12 and 24 twists (measured,
  [stage 12](../restoration/12-low-height/README.md), section 10).
- **Into an empty home cell, without V.** When a chain of commutator powers leads the unit to an empty home cell of
  its kind and each power fixes every cell position that other pieces hold, the chain alone is a strict clean step
  (proved, R22A-03 in the [stage 12 proofs](../restoration/12-low-height/PROOFS-B.md), which list the exact
  conditions). A home cell that other pieces hold in part cannot take the unit (R22A-05). At the depth 40 stop one
  triangle unit landed this way, with two powers in a step of 60 twists (measured,
  [stage 12](../restoration/12-low-height/README.md), section 11). Look for such a cell before vacating one.

## 3.6 Unlocking

Blocks that no cap can move need another block moved first. At the depth 60 stop, one retained twist at a blocked
unit gave 20 checked steps (measured, [stage 12](../restoration/12-low-height/README.md), E-B4), and a partner
template was built from the configuration alone at another state (E-B3).

A **glued** unit, whose own cells hold pieces that no safe move can shift, cannot be freed by any safe word at that
configuration (proved, with the instances computed at the depth 40 stop, R22A2-06). Four of the seven glued units
there have steps of another form, the landing word followed by V′·W⁻¹·V′⁻¹, of 160, 942, 1,158 and 160 twists,
each replayed strict (measured, [stage 12](../restoration/12-low-height/README.md), section 11, line (c)).

## 3.7 What is open

Whether the tail can always be finished with steps of a length a person can carry out is the open question of
option B: the completeness conjecture of [stage 12](../restoration/12-low-height/README.md) for left heights 2
and 4. The owner counts one step of a few hundred twists as executable by a person and set the goal on 18 October
2026: prove that the steps exist, with explicit bounds that need not be small
([decision](../../../docs/wiki/decisions/owner-decisions-2026-10-18-bprime-goal.md)). Four research lines work on
it ([stage 12](../restoration/12-low-height/README.md), section 11): transport into empty home cells, the vacating
word V, unlocking glued units, and height 4.
