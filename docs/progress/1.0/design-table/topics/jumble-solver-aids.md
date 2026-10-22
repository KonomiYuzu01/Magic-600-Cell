# DT-J1: Jumble solver aids and the human-solve boundary

Status: **open.** Opened on 22 October 2026 by the main session under the [round protocol](../README.md), as the first design-table topic on Jumble ([re-plan](../../replan-2026-10-22.md), section 4). Two rounds, then the outcome and one owner batch.

## Question

While Jumble is on (the jumbling puzzle, menu I_a), what may the program show, list and check for a person who solves the jumbled puzzle? The answer must stay within the human-solve boundary of 2 October 2026 and its one exception, the repair list of 17 October 2026.

For each item below the round settles one of three answers:
- allowed as stated;
- allowed under a stated condition;
- out.

Anything the records do not settle becomes an owner question.

## Items

**A. Facts about the configuration that the requirements draft does not list.** Rule 6 allows facts such as N_c, the floor and blocked caps (`docs/progress/1.0/jumbling-observation.md:42`).
- A1. Counts of off pieces by left height (n₂, n₄, n₆, and higher) in the Global totals. Steering by N alone can undo a height step (DH-28, EH-26, DO-20).
- A2. The straddler count u of a cap the solver names (DH-28, EH-26).
- A3. ν, coordinate by coordinate, on a lattice configuration (EH-10, DH-10).
- A4. The covers relation for two caps the solver names, with an exact definition (EH-08, EH-28, DH-10).

**B. Orders and filters by a shown fact.** Sorting or filtering the cap map by N_c, the floor or u (DH-26, EH-27). Rule 6 forbids ranking caps "by how promising they are". The manual's repair step has the solver name caps by decreasing N_c (`research/jumbling/manual/02-descent.md`, DO-02). Options on the table:
1. no order and no filter, so the caps keep a fixed layout order;
2. a filter or highlight at a threshold the solver sets, with no order;
3. a sort that the solver chooses each time, by a shown fact;
4. a default sort.

**C. Lists beyond the repair list.** The exception covers only the (x, d) pairs at a cap the solver names (`owner-decisions-2026-10-17-repair-pairs.md:57-59`).
- C1. Unlock letters or pairs, or a target cap, as the research rules R22D-81, U2 and H4-unlock pick them (EH-27).
- C2. Which of the 36 corrections ν calls for. The manual says this is not decided (EH-12).
- C3. Existence tests by certificate (N1 and N2 with Jordan certificates) for a target the solver names (EH-17, EO-08, EO-18).
- C4. The allowed setup search for a target the solver names (`owner-decisions-2026-10-02.md:59`): what counts as a target with Jumble on, and in which phase it applies. The lattice-phase help plans to use it (EH-13).

**D. Checks of the solver's own twist or word.** These are macro analysis, which the boundary allows.
- D1. The checks of section 7 with Jumble on: whether a twist closes a cut, whether a word lowers N, the floor, and what blocks a cap (EH-06).
- D2. The field list of a step check (EH-25, DO-22):
  - start and end digests and the number of letters;
  - the unit's size and how many of its pieces land;
  - the collateral, that is, the lattice pieces taken off;
  - the other off pieces that keep their exact pose or land;
  - the off counts before and after;
  - the strict′ flag.
- D3. A whole-word re-check at the current revision before commit. It reuses no certificate, seed, hole count or index from an earlier revision (EH-15, EH-24, DO-07, DO-15).
- D4. Typed job results:
  - a cancelled or cut-off job is kept apart from a completed one, and a result is matched to its input digest before its outcome is read (EO-23, EO-24);
  - no message says that no step exists (DO-19).

**E. Words with Jumble on.**
- E1. Storing and running the solver's own words of jumble twists. ENG-0J refuses the retained workflow's macros while Jumble is on and has no jumbling macro row (EH-14, EH-22, EO-10, DH-11).
- E2. The 36 corrections: shipped as program content, or made by the solver. Does the 3 October ruling on selected constructions cover them, and words such as commutator powers? (`owner-decisions-2026-10-03.md:57-59`; EH-11; design-table page, Open items.)
- E3. A step known only by a certificate is not a word that anyone can enter, store or run (DH-25).
- E4. Research words never become program content (DH-24, EH-14). They include machine words of up to 672,417 twists in one step and a lattice word of 166,788,405 twists that was checked by composition.

**F. The end of a solve, and Jumble off.**
- F1. Which test ends a jumbled solve in the product: exact pose equality or the sticker definition. The 15 October definition is for the research only (EO-11, EO-20).
- F2. Jumble off at ν ≠ 0. ENG-0J needs a witnessed retained checkpoint, which a lattice state with ν ≠ 0 cannot have. Each refused case needs a message (EH-18, EO-10, DH-08).

**G. Repair-list details that touch the boundary.** The layout of the list belongs to DT-J4.
- G1. Whether a default focus on one row marks a pair (design-table page, Open items).
- G2. Whether one action of the solver may run a pair's three twists, or one action runs each twist (DH-20).
- G3. The tie rule for y and z (EO-04). The owner's text says "the best twist". The research takes the lowest N_d and N_c, with ties going to the smaller menu index.

## Owner input (binding)

On `main`:
- the human-solve boundary, with setup search allowed: [`owner-decisions-2026-10-02.md`](../../../../wiki/decisions/owner-decisions-2026-10-02.md), line 59;
- selected constructions as the solver's own macro: [`owner-decisions-2026-10-03.md`](../../../../wiki/decisions/owner-decisions-2026-10-03.md), lines 57-59;
- design: [`owner-decisions-2026-10-02-design.md`](../../../../wiki/decisions/owner-decisions-2026-10-02-design.md) (two linked views, text budgets, the 8B order);
- jumbling: the decisions of 9 October (jumbling), 13 October (menu I_a), 15 October (the research definition of restored), and 17 October (M2 and the tutorial).

On #112's branch only (read with `git show ec0f0ac:<path>`):
- `docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md`: the one exception, lines 47-59;
- `docs/wiki/decisions/owner-decisions-2026-10-17-jumbling-pr.md`;
- `docs/wiki/decisions/owner-decisions-2026-10-18-bprime-goal.md`;
- `docs/wiki/decisions/owner-decisions-2026-10-18-machine-tutorial.md`: lines 81-83 say that the research tools leave the product's boundary unchanged;
- `docs/wiki/decisions/owner-decisions-2026-10-20-research-stop.md`;
- `docs/wiki/decisions/owner-decisions-2026-10-20-tutorial-acceptance.md`.

## Handoff and evidence

- [`jumbling-observation.md`](../../jumbling-observation.md): rules 1 to 8 (section 2), checks (section 7), data (section 8) and questions (section 10). The branch version adds the exception to rule 6 and question 9.
- On #112's branch:
  - `docs/progress/1.0/jumbling-handoff/design-table.md` (DH, DO);
  - `docs/progress/1.0/jumbling-handoff/engineering.md` (EH, EO).
- The manual: `research/jumbling/manual/`. The branch version is newer.
- The packet: [`ENG-0J`](../../packets/engineering/ENG-0J-jumbling-backend.md).

## Out of scope

- Layout, look and motion of any view (DT-J2 to DT-J4), including the layout of the repair list.
- The research restart, the tutorial's content and its acceptance.
- Aids for 600-cell-Full.
- Performance claims. A cost is given as measured, with its evidence, or as unmeasured.

## Round 1
