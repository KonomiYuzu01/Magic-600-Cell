# Jumbling: handoff and observation logs

Started 18 October 2026. One page per 1.0 line that the jumbling work feeds: what the research hands to the line, and
what it has observed that bears on the line's work. Writing these logs is one of the conditions under which draft pull
request #112 merges ([owner decision, 17 October 2026](../../../wiki/decisions/owner-decisions-2026-10-17-jumbling-pr.md)).
The logs decide nothing: decisions are the owner's and live in [`docs/wiki/decisions/`](../../../wiki/decisions/); the
jumbling plan is [`jumbling-plan.md`](../jumbling-plan.md) and the view requirements are
[`jumbling-observation.md`](../jumbling-observation.md).

## Pages

| Line | Page | Handoff items | Observations | What holds the line now |
| --- | --- | --- | --- | --- |
| Renderer (stage 2.4) | [renderer.md](renderer.md) | 17 (RH-01 to RH-17) | 22 (RO-01 to RO-22) | W-J item 7 (overlay costs) is unmeasured, so W-J is not accepted as a whole; tints, exposed faces, the cut slice, the cap map, the repair list and long-word playback are in no renderer packet |
| Engineering | [engineering.md](engineering.md) | 19 (EH-01 to EH-19) | 15 (EO-01 to EO-15) | ENG-0J's outputs are not in the repository; ENG-0J has no macro for the jumbling puzzle, and its Jumble off needs a retained checkpoint that a lattice state with ν ≠ 0 cannot have |
| Look Lab | [look-lab.md](look-lab.md) | 21 (LH-01 to LH-21) | 24 (LO-01 to LO-24) | the Look Lab draws slot labels in cell frames and has no per-piece pose path, and no jumbling command rows exist for its flows |
| Taste Lab | [taste-lab.md](taste-lab.md) | 9 (TH-01 to TH-09) | 14 (TO-01 to TO-14) | Taste Lab's files stay unchanged for the jumbling work; it draws no jumbled state, and in its colour sample no palette of 20 colours passed its colour-vision check |
| Design table | [design-table.md](design-table.md) | 20 (DH-01 to DH-20) | 12 (DO-01 to DO-12) | H-01, H-03, H-04, H-05 and the command table have no Jumble rows; nine questions wait for the owner |

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
- Evidence kinds are those of `AGENTS.md`: source and fixture, synthetic geometry, actual Windows/DirectX, and
  performance. The entries of 18 October are headless source and fixture evidence except where they say otherwise;
  W-J acceptance items 1 to 6 are recorded Windows/Direct3D 12 and performance evidence for one build on the owner's
  machine (renderer, RH-02).
- No log lifts a rule: the human-solve boundary (with its one exception, the repair list at a cap the solver names),
  taste and G1 to G6 stay with the owner, and Taste Lab's files stay unchanged.

## Questions that several lines share

- The nine questions of the view requirements (section 10 of `jumbling-observation.md`): slice layout, domain palette,
  exposed-face material, cap map, raw view, entry defaults, slice motion, Global layout and repair-list presentation
  (design table DH-12 to DH-20; Look Lab; renderer).
- How words of jumble twists are entered, checked, stored and run while Jumble is on: steps found at the depth 40 stop run
  from 10 to 1,158 letters, the 36 corrections from 5 to 5,629 twists, and the tutorial wants the corrections as stored
  macros (design table DH-11, renderer RH-14, engineering EH-11 and EH-14).
- Domain colours: about 20 are asked for, but the W-J I_a fixture path reaches 37 domains (Taste Lab TO-01; design
  table DH-13; Look Lab LH-09).
- Whether ordering or filtering the cap map by N_c or the floor counts as ranking caps (design table, Look Lab).
- Whether the final renderer selection needs W-J runs (renderer RH-17).

## Checks not run

None of these pages rests on a Windows, Direct3D 12, input, long-session or performance check made for it; each page
lists what it did not run. They were written in a Linux session by reading the repository.
