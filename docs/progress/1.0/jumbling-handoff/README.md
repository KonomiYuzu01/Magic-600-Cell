# Jumbling: handoff and observation logs

Started 18 October 2026. One page per 1.0 line that the jumbling work feeds: what the research hands to the line, and
what it has observed that bears on the line's work. Writing these logs is one of the conditions under which draft pull
request #112 merges ([owner decision, 17 October 2026](../../../wiki/decisions/owner-decisions-2026-10-17-jumbling-pr.md)).
A second entry on each page, dated 21 October 2026, records the work of 18 to 20 October and the state after the owner
stopped the jumbling research on 20 October
([owner decision](../../../wiki/decisions/owner-decisions-2026-10-20-research-stop.md)).
The logs decide nothing: decisions are the owner's and live in [`docs/wiki/decisions/`](../../../wiki/decisions/); the
jumbling plan is [`jumbling-plan.md`](../jumbling-plan.md) and the view requirements are
[`jumbling-observation.md`](../jumbling-observation.md).

## Pages

| Line | Page | Handoff items | Observations | What holds the line now |
| --- | --- | --- | --- | --- |
| Renderer (stage 2.4) | [renderer.md](renderer.md) | 24 (RH-01 to RH-24) | 27 (RO-01 to RO-27) | W-J item 7 (overlay costs) is unmeasured, so W-J is not accepted as a whole, and no probe file, contract or packet has changed since 18 October; tints, exposed faces, the cut slice, the cap map, the repair list and long-word playback are in no renderer packet; no deep state (the six height-4 states, the 3000-twist scrambles) has a render-format export for a W-J run; nothing on the page waits on the stopped research |
| Engineering | [engineering.md](engineering.md) | 30 (EH-01 to EH-30) | 24 (EO-01 to EO-24) | ENG-0J has not started and its outputs are not in the repository; ENG-0J has no macro for the jumbling puzzle, and its Jumble off needs a retained checkpoint that a lattice state with ν ≠ 0 cannot have; stage 13's record formats and checker are reference designs only; no EH item waits on the stopped research |
| Look Lab | [look-lab.md](look-lab.md) | 31 (LH-01 to LH-31) | 31 (LO-01 to LO-31) | the Look Lab draws slot labels in cell frames and has no per-piece pose path, and no jumbling command rows exist for its flows; no record assigns or dates jumbling work for it; the deeper test states have no render-format export; none of this waits on the stopped research |
| Taste Lab | [taste-lab.md](taste-lab.md) | 13 (TH-01 to TH-13) | 18 (TO-01 to TO-18) | Taste Lab's files stay unchanged for the jumbling work (checked again on 21 October); it draws no jumbled state, and in its colour sample no palette of 20 colours passed its colour-vision check; no domain count exists at the deeper states |
| Design table | [design-table.md](design-table.md) | 29 (DH-01 to DH-29) | 23 (DO-01 to DO-23) | H-01, H-03, H-04, H-05 and the command table have no Jumble rows, and no design-table round on Jumble is recorded; the nine questions and ten further points (words, ν corrections, ranking, repair-list focus, a Jumble learning path, counts by height or u, the illustrations, the restart of the research, the height ramp, the end test of a solve) wait for the owner; DH-10's check waits for the tutorial's acceptance, which cannot run while the research is stopped |

## The two logs

- **Handoff log**: one table per dated entry. Each row names an item, its source, its status, what the line does with
  it and the check that shows it done.
- **Observation log**: numbered facts, each with its evidence and what it means for the line.
- **Status** of a handoff item:
  - *ready*: binding or recorded, usable now;
  - *draft (2.5 freeze)*: a proposal that the 2.5 freeze accepts or changes, not an authorized change;
  - *not started*;
  - *open (owner)*: waits for an owner decision;
  - *open (measurement)*: waits for a measurement, usually on the owner's machine.

## Rules for entries

- Append only. A new entry gets a dated heading; an item that a later entry replaces is marked "superseded by
  <entry>", not deleted.
- A citation `path:line` refers to the file as it was in the commit that added the entry; later edits can move lines.
- Current-state sections (integrator decision, 21 October 2026): the Summary, Open items and Checks not run sections
  of each page describe its current state and are updated in place; their citations refer to the commit that last
  changed them. The two logs stay append-only. A new row or observation that changes an older item starts with
  "Updates XH-NN:" (or "Updates XO-NN:"), and the older item keeps its text and gets a mark at its end: *Updated by XH-MM
  (date).* when it still holds but is extended, or *Superseded in part by XH-MM (date).* when a number, status or
  statement in it no longer holds.
- Evidence kinds are those of `AGENTS.md`: source and fixture, synthetic geometry, actual Windows/DirectX, and
  performance. The entries of 18 October are headless source and fixture evidence except where they say otherwise;
  W-J acceptance items 1 to 6 are recorded Windows/Direct3D 12 and performance evidence for one build on the owner's
  machine (renderer, RH-02). The entries of 21 October read headless research code run on Linux: its times are cloud
  timings of research code, not performance results, and its round 22 proofs are model review only.
- No log lifts a rule: the human-solve boundary (with its one exception, the repair list at a cap the solver names),
  taste and G1 to G6 stay with the owner, and Taste Lab's files stay unchanged. The tutorial runner, its checker and
  the research rules, players and restorers serve the owner's machine test of the tutorial; they are not product
  features, and nothing they choose is shown to a solver.

## Questions that several lines share

- The nine questions of the view requirements (section 10 of `jumbling-observation.md`): slice layout, domain palette,
  exposed-face material, cap map, raw view, entry defaults, slice motion, Global layout and repair-list presentation
  (design table DH-12 to DH-20; Look Lab; renderer). No owner record of 18 to 20 October answers any of them (design
  table DO-23). The cap map also has to work at deep states where 528 to 578 of the 600 caps are blocked (design table
  DH-26; Look Lab LH-25; renderer RH-21).
- How words of jumble twists are entered, checked, stored and run while Jumble is on. The shorter words on record, none
  tried by a person, run from 10 to 1,332 twists, and the 36 corrections, which the tutorial wants as stored macros,
  from 5 to 5,629. The machine words reach 672,417 twists in one literal step (E2's 13 steps, 2,012,155 in all) and
  166,788,405 in the lattice word after them, which was checked by composition, not played. The owner's content
  decision for the complete tutorial (every macro expandable to single twists; in the integrator's reading, a folded
  tree of named parts) is an input to this question, not an answer (design table DH-11, DH-23, DH-24; renderer RH-14,
  RH-19, RH-20; engineering EH-11, EH-14, EH-22; Look Lab LH-27, LH-31).
- Domain colours: about 20 are asked for, but the W-J I_a fixture path reaches 37 domains, and no domain count exists
  at the 3000-twist scrambles or the six height-4 states (Taste Lab TO-01, TO-17; design table DH-13; Look Lab LH-09,
  LH-26).
- The height ramp's range and its rule above height 6. Random play of up to 3000 twists shows left heights 2, 4 and 6
  only, and every reachable pose has right height 0 (proved, model review only); constructed states reach unbounded
  heights (Taste Lab TH-12; Look Lab LH-24; renderer RH-22; engineering EH-29; design table, Open items).
- Whether ordering or filtering the cap map by N_c, the floor or a cap's straddler count u counts as ranking caps
  (design table DH-26, DH-28; Look Lab LH-25, LH-28; renderer RH-21).
- Whether the program shows counts of off pieces by height (n₂, n₄, n₆) or the straddler count u of a cap the solver
  names, and whether a view may list unlock letters or pairs or pick a target cap. The repair-list exception covers
  only the (x, d) pairs at a cap the solver names (design table DH-28; engineering EH-26, EH-27; Look Lab LH-28;
  renderer RH-21).
- Which test ends a jumbled solve in the product, exact pose equality or the sticker definition; the 15 October
  definition is for the research (engineering EO-11, EO-20; design table, Open items).
- Who draws the complete tutorial's detailed illustrations, from which views and with which tool; whether their look
  is a Taste Lab item; and whether a product view shows a stored word as a folded tree of named parts (design table
  DH-29; Look Lab LH-31; Taste Lab TH-13; renderer RH-20).
- Whether and when the owner restarts the jumbling research
  ([owner decision, 20 October 2026](../../../wiki/decisions/owner-decisions-2026-10-20-research-stop.md)). Until then
  the tutorial is not accepted, none of the 21 acceptance scrambles has run, packets P1 to P5 stay unapplied, height 4
  and the coverage of B′(2) stay open, and no new research input arrives. No line's stage 2 work waits on it except the
  check of design table DH-10 (renderer RH-18; engineering EH-20; Look Lab LH-22; Taste Lab TH-10; design table DH-21,
  DH-22).
- Whether the final renderer selection needs W-J runs (renderer RH-17); not answered since 18 October.

## Checks not run

None of these pages rests on a Windows, Direct3D 12, input, long-session or performance check made for it; each page
lists what it did not run. Both entries were written in a Linux session by reading the repository; they ran no
research program, test, build, browser or GPU run (the Taste Lab entry of 21 October ran only `git diff` and a text
search, TH-11).
