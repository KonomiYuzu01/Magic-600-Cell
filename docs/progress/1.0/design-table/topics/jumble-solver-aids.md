# DT-J1: Jumble solver aids and the human-solve boundary

Status: **settled; waiting for the owner's batch** (see the outcome at the end). Opened on 22 October 2026 by the main session under the [round protocol](../README.md), as the first design-table topic on Jumble ([re-plan](../../replan-2026-10-22.md), section 4). Two rounds were run as a trial at the owner's request, then the outcome and one owner batch.

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

Run on 22 October 2026. The four roles answered independently on the record above. Their cards are appended unchanged; only the layout differs, with one list item per field.

### Front-end designer

#### D1

- on: items A1, A2
- kind: support
- claim: Items A1 and A2 are allowed as stated. n_h is the preview count of the height filter that the requirements already define, and u is the size of the straddler set of a cap the solver names; all of that cap's straddlers should be shown, not only the certificate piece (u reached 227 at one recorded cap). The step check of item D2 should report Δn_h, and Δu at a named cap, next to ΔN, because these values pull apart: a retained letter that freed 23 of cap 436's 227 straddlers raised N by 428, and a cover twist that lowered N by 1,056 undid a height step.
- evidence: docs/progress/1.0/jumbling-observation.md:42, :68, :74, :80; docs/wiki/decisions/owner-decisions-2026-10-02.md:59; research/jumbling/restoration/12-low-height/README.md:1251, :1254-1255 (ec0f0ac); docs/progress/1.0/jumbling-handoff/design-table.md:81 (ec0f0ac) (DO-20). Sketch: the totals list only the heights present, in ascending order, because left heights are unbounded (docs/progress/1.0/jumbling-handoff/engineering.md:61 (ec0f0ac)).
- needs_owner: no

#### D2

- on: item A3
- kind: support
- claim: Item A3 is allowed as stated, and ν belongs in the per-orbit residual summary that bar D8 already embeds: one sign flag per orbit, plus a frame flag or C5 value where the orbit has one, with a designed "undefined" state while N > 0 instead of zeros. A lone swap, flip or C5 turn that its orbit's ν coordinate accounts for must read as that coordinate, not as the retained "impossible residual" diagnosis; that diagnosis still applies to the nine coordinates that are always 0, and to all coordinates once ν = 0.
- evidence: research/jumbling/manual/04-lattice.md:45-50, :61-72, :90-95 (ec0f0ac); docs/progress/1.0/solving-workflow.md:54; docs/wiki/decisions/owner-decisions-2026-10-02-design.md:68; docs/progress/1.0/charter-2.0-draft.md:106
- needs_owner: no

#### D3

- on: item A4
- kind: proposal
- claim: I support item A4 with EH-28's exact side test, but a check of two named caps alone makes the cover step's condition "no cap covers c" cost one request per neighbour of c, and on the measured record each repair cap had 2 to 27 misaligned neighbours. Owner: (a) two named caps, as stated; (b) one named cap with the caps it covers and those that cover it, which I recommend because it answers only about the cap the solver names, as the slice's counts do for the face match; (c) a cap-map mark on caps that cover some cap and that no cap covers, which I advise against because it marks the next cap for the cover step.
- evidence: research/jumbling/manual/02-descent.md:100-108 (ec0f0ac); research/jumbling/restoration/11-scramble-census/README.md:266-269, :278, :280-281 (ec0f0ac) (the relation does not single out the repair pair); research/jumbling/restoration/11-scramble-census/scripts/density.py:61-66 (ec0f0ac) (pairs at a cap = 59 openings × misaligned neighbours, so 118 to 1,593 pairs means 2 to 27 neighbours; arithmetic on the record); docs/progress/1.0/jumbling-handoff/engineering.md:60 (ec0f0ac); docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:57-59 (ec0f0ac)
- needs_owner: yes

#### D4

- on: item B
- kind: proposal
- claim: I object to option 4, because a default order puts the program's choice of first cap in front of the solver. The cheapest admissible aid for the manual's walk by decreasing N_c is a "name next cap" key, with no key repeat, that follows an order the solver chose by a shown fact and requests that cap's repair list; the cap map keeps its fixed layout, because position carries identity (on the record, the cap with a repair was named first or second at 41 of 63 repair steps, and never later than 28th). Owner: (a) option 2, a threshold the solver sets; (b) option 3, chosen again each time; (c) option 3 kept as the solver's own saved setting, off until the solver first chooses it, which I recommend; a repair step found at the k-th named cap then costs k + 2 actions.
- evidence: research/jumbling/manual/02-descent.md:165-172 (ec0f0ac); research/jumbling/restoration/11-scramble-census/README.md:327-333 (ec0f0ac); docs/progress/1.0/jumbling-observation.md:42, :128; docs/wiki/decisions/owner-decisions-2026-10-02-design.md:41; docs/progress/1.0/command-table.md:46. Sketch: k namings, one pick and one commit (card D11); a design count, not a measurement. Each naming starts a list job; the research script took 3 to 5 minutes per cap (docs/progress/1.0/jumbling-observation.md:176 (ec0f0ac); a research timing, not performance evidence). Experiment: before DT-J2, the engineer measures one product request at the record's 63 repair-step states.
- needs_owner: yes

#### D5

- on: items C1, C3
- kind: proposal
- claim: A target cap that the program picks (item C1) is out, because the program never picks a target and does not choose the cap. For unlock lists (C1) and existence tests (C3), owner: (a) out of 1.0, which I recommend because no manual chapter teaches a person a step that uses them and a certificate gives no word to play (the plan of a solve has no height part, and the tail chapter recommends nothing yet); this leaves the check of a letter the solver proposes at a named cap, with its Δu and ΔN; (b) allowed at a cap or target the solver names, under the repair list's conditions.
- evidence: docs/wiki/decisions/owner-decisions-2026-10-03.md:59; docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:51-52, :57-59 (ec0f0ac); research/jumbling/manual/README.md:44-50 (ec0f0ac); research/jumbling/manual/03-tail.md:3-5, :56-59 (ec0f0ac); docs/progress/1.0/jumbling-handoff/engineering.md:59 (ec0f0ac) (EH-27)
- needs_owner: yes

#### D6

- on: items C2, E2
- kind: proposal
- claim: Item C2 as stated is out without a new exception, because the program never picks a correction or outputs a solving macro. The cheapest admissible aid is to show the ν that a draft of corrections chosen by the solver would leave, which is analysis of the solver's own macro; typing the corrections by hand is impractical (5 to 5,629 twists each, at most 25,852 in all). Owner, for item E2: (a) the 36 corrections ship as known constructions that the solver selects by id, as the 3 October ruling does for stars and transfers, which I recommend; (b) the solver enters them as their own words; (c) a new exception that lets the program work out the combination from ν. In every case, a commutator power that the solver writes out as a formula is the solver's own macro under the 2 October boundary.
- evidence: docs/wiki/decisions/owner-decisions-2026-10-02.md:59; docs/wiki/decisions/owner-decisions-2026-10-03.md:57-59; research/jumbling/manual/04-lattice.md:83-88, :97-105, :111-113, :121-123, :245-282 (ec0f0ac); docs/progress/1.0/jumbling-handoff/engineering.md:38-39 (ec0f0ac) (EH-11, EH-12)
- needs_owner: yes

#### D7

- on: items C4, F1, F2
- kind: proposal
- claim: The lattice phase is orbit-first block building, so I support EN1's proposed checkpoint rule at ν = 0, subject to its contract review. One crossing then gives the lattice phase the unchanged retained toolset, including setup search (item C4); setup search keeps the buffers fixed, so it applies only on the lattice and never to a vacating word V off it. Jumble off (item F2) is then refused only at N > 0 or ν ≠ 0, each time with a message that shows N or the ν coordinates still set. Item F1, owner: (a) the label test, which I recommend because it is 600-cell-Full's own test after the crossing and the only test whose result the screen can show; (b) exact pose equality; (c) both, shown separately.
- evidence: research/jumbling/manual/04-lattice.md:90-95, :125-153, :185-186 (ec0f0ac); docs/wiki/decisions/owner-decisions-2026-10-02.md:59; research/jumbling/manual/03-tail.md:20, :45 (ec0f0ac); docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:20, :36-37; docs/progress/1.0/replan-2026-10-22.md:72; docs/progress/1.0/charter-2.0-draft.md:93
- needs_owner: yes

#### D8

- on: item D1
- kind: proposal
- claim: I support item D1, with the exact check run on the release of every previewed twist and shown as N_c after against the floor, together with ΔN. A check then costs no action beyond keying the twist, which matters because checking is the most repeated act of the descent (a median of 71 checks per face match on the measured record). Owner: (a) the view shows only the latest check; (b) it also keeps the solver's own checks at the named cap for the current revision, in the order tried and unranked, so that the manual's "take the largest drop" needs no memory; I recommend (b), because it holds only twists the solver proposed.
- evidence: docs/progress/1.0/jumbling-observation.md:35, :42-43, :118-119; research/jumbling/restoration/11-scramble-census/README.md:258-263 (ec0f0ac); research/jumbling/manual/02-descent.md:43-48 (ec0f0ac); docs/progress/1.0/jumbling-handoff/design-table.md:61 (ec0f0ac) (DO-03)
- needs_owner: yes

#### D9

- on: items D2, D3, D4
- kind: support
- claim: I support items D2 to D4 as the Jumble form of S4 and of design bar D2's states. The collapsed operation summary shows ΔN, the unit pieces landed out of the unit's size, the collateral, the off count before and after and the strict′ status; digests and letter counts appear only when it is expanded; and the commit is one action from a whole-word preview at the current revision. A completed empty repair list, a cut-off or cancelled job and a stale result get three distinct status patterns, and the empty answer names its cap and revision and never reads as "no step exists".
- evidence: docs/progress/1.0/charter-2.0-draft.md:83, :106; docs/wiki/decisions/owner-decisions-2026-10-02-design.md:41, :68; docs/progress/1.0/jumbling-observation.md:35; docs/progress/1.0/solving-workflow.md:83; docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:51-52 (ec0f0ac); docs/progress/1.0/jumbling-handoff/design-table.md:60, :80 (ec0f0ac) (DO-02, DO-19); docs/progress/1.0/jumbling-handoff/engineering.md:114 (ec0f0ac) (EO-23). Sketch: "cap c · 0 of 1,180 pairs lower N" (completed) against "cap c · 412 of 1,180 searched" (cut off); IDs and numbers only, within the text budget.
- needs_owner: no

#### D10

- on: items E1, E3, E4
- kind: proposal
- claim: Item E1 is allowed by the 2 October boundary (entering, recording and reusing the solver's own macros), and both the tail and the corrections depend on it. I propose one macro base and one set of word commands for both puzzles: each word is stored with its model and menu identity and is re-checked as a whole at the current revision, through whichever engine path (labels or poses) the session is on, so the solver learns one set of keys. I support items E3 and E4: a word is made of literal twists only, so no certificate is offered as a word and no research word ships, while a word the solver enters has no length limit (recorded steps reach 672,417 twists).
- evidence: docs/wiki/decisions/owner-decisions-2026-10-02.md:59; research/jumbling/manual/03-tail.md:20, :56-66 (ec0f0ac); docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:20, :37; docs/progress/1.0/jumbling-handoff/engineering.md:41, :100 (ec0f0ac) (EH-14, EO-16); docs/progress/1.0/jumbling-handoff/design-table.md:49 (ec0f0ac) (DH-25); docs/progress/1.0/solving-workflow.md:83
- needs_owner: no

#### D11

- on: items G1, G2, G3
- kind: proposal
- claim: Item G2: picking a row should copy (c, x)(d, y)(c, z) into the solver's draft, because rule 3 discards the list at the next commit, so y and z would be lost after x. Owner, for G2: (a) one commit per twist from that draft; (b) one commit of the whole draft, after the solver has been able to step through x, y and z, which I recommend (S4: the commit takes one action). Owner, for item G1: (a) the cursor starts on row 1; (b) no row has focus and a number key picks a row, which I recommend because it marks nothing and picks any of the first nine rows with one key (the median list on the record has 5 rows). Item G3 is settled by the decision's "this is how the measurement counted them": y leaves N_d lowest, z leaves N_c lowest, and ties go to the smaller menu index.
- evidence: docs/progress/1.0/jumbling-observation.md:35; docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:48-50, :53-56 (ec0f0ac); docs/progress/1.0/charter-2.0-draft.md:83; research/jumbling/restoration/11-scramble-census/scripts/density.py:29, :38 (ec0f0ac); research/jumbling/restoration/11-scramble-census/README.md:330-333 (ec0f0ac). Sketch: the picked row's c and d are marked in Global and Local together (bar D8).
- needs_owner: yes

#### D12

- on: DT-J2 to DT-J4; ENG-0J command rows (EN3)
- kind: proposal
- claim: Every admitted aid becomes one command row whose arguments name the solver's object (a cap, two caps, a twist, a word or a pair) and whose answer is about that object only. The only exceptions are the per-revision facts (cap map, totals, ν) and the repair list at its named cap. The boundary can then be checked row by row, and no row can search across caps for one that has a pair. Later topics depend on this round: DT-J2 on items A1 to A4, B (a fixed cap map with a traversal order), D4 (job states) and F2 (switch messages); DT-J3 on A1 (height counts) and D4 (status patterns); DT-J4 on D2, D3, E1, E2, G1 and G2; and DT-M1 binds keys to all of these rows.
- evidence: docs/progress/1.0/command-table.md:46-47, :67; docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:15, :74; docs/progress/1.0/replan-2026-10-22.md:58-62, :74; docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:51-52 (ec0f0ac)
- needs_owner: no

### Interaction artist

#### A1

- on: B
- kind: proposal
- claim: In the grammar, position carries identity, and the cap map has a fixed layout, so no option may re-order the map; I object to option 4 because a first place the program chooses is the strongest "start here" mark there is, and a default sort by decreasing N_c would run the manual's own cap order for the solver. Owner: (a) option 1; (b) option 2: a threshold the solver sets, drawn as a status pattern on the unchanged map, with no default value (preferred); (c) option 2 plus option 3 as a separate list that the solver opens and sorts each time, with its sort key shown in the header.
- evidence: docs/wiki/decisions/owner-decisions-2026-10-02-design.md:41; docs/progress/1.0/jumbling-observation.md:42, :128; research/jumbling/manual/02-descent.md:169-171 (ec0f0ac); docs/progress/1.0/jumbling-handoff/design-table.md:90 (ec0f0ac) (open item); docs/progress/1.0/jumbling-handoff/design-table.md:79 (ec0f0ac) (DO-18: 528 to 578 of 600 caps blocked, so a threshold pattern must still read next to the blocked mark)
- needs_owner: yes

#### A2

- on: A (rule 6 facts), A4, B
- kind: proposal
- claim: Give "blocked" and "at its floor" two distinct status patterns, make "at its floor" look neither blocked nor closed (the manual's tail repair works on two caps that are at their floors), and leave "above its floor" unmarked. No cap gets a mark for passing the face match or for covering another cap, because such a mark picks the cap that the face match or the cover step would twist next; "covers" stays a check of two caps the solver names (A4).
- evidence: docs/progress/1.0/jumbling-observation.md:118, :128; research/jumbling/manual/02-descent.md:38-41, :106-108, :197-202 (ec0f0ac); docs/progress/1.0/jumbling-handoff/design-table.md:62 (ec0f0ac) (DO-04); docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:57-59 (ec0f0ac)
- needs_owner: no

#### A3

- on: G1
- kind: proposal
- claim: In the grammar, focus decides which objects get every channel, so a default focus on a row would draw that pair's x, y and z in full on the geometry while the other rows stay plain, and that is what marking a pair looks like. Owner: (a) the list opens with focus on the cap the solver named and on no row, and the first arrow key focuses the first row in the neutral order (preferred); (b) focus starts on the first row, shown only by the generic focus ring, and nothing is drawn on the geometry until the solver moves; (c) no row focus, and the solver picks a row by typing its x and d.
- evidence: docs/wiki/decisions/owner-decisions-2026-10-02-design.md:42; docs/progress/1.0/design-options-2026-10-round2.md:49 ("focus changes visibility, never meaning"); docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:55-56 (ec0f0ac); docs/progress/1.0/jumbling-handoff/design-table.md:91 (ec0f0ac); docs/progress/1.0/dispositions/dispositions.md:250 (retained precedent: nothing happens "merely because a candidate is first"); docs/progress/1.0/jumbling-observation.md:44
- needs_owner: yes

#### A4

- on: G1, DH-01 (row content)
- kind: proposal
- claim: If a row shows what its repair does to N, it shows it as a numeral in a fixed column, in the neutral order, and never as bar length, a colour ramp, weight or size: a magnitude channel makes the largest drop pop out and so marks a best pair. Owner: (a) rows show x, d, y and z only, and the solver checks a chosen row with the ordinary word check; (b) rows also show the change in N as a numeral (preferred: the program has already computed it to list the pair, and hiding it costs the solver one check per row).
- evidence: docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:53-56 (ec0f0ac) (the decision names x, d, y and z, not the change in N); docs/wiki/decisions/owner-decisions-2026-10-02-design.md:41; docs/progress/1.0/jumbling-observation.md:132; docs/progress/1.0/jumbling-handoff/design-table.md:59 (ec0f0ac) (DO-01: 1 to 61 rows, median 5)
- needs_owner: yes

#### A5

- on: D4, F2
- kind: proposal
- claim: Give each typed result its own status pattern so that a cut-off job never looks like an empty list: answered with rows; answered and empty, stating its scope (cap, revision and pairs tried); cut off or cancelled ("stopped at k of n"); stale; and an input mismatch shown as a typed error. The empty answer is the usual case and should read calmly and quickly, never in an error style and never claiming more than "no pair at cap c lowers N". A refused Jumble off is different: it is an E6 error whose "what you can do" names the fact and the kind of action (ν is not 0: make corrections; N > 0: pieces are off the lattice) and focuses that fact in the view, but never names which corrections or which twist.
- evidence: docs/progress/1.0/jumbling-handoff/engineering.md:106, :114, :116 (ec0f0ac) (EO-19, EO-23, EO-24); docs/progress/1.0/jumbling-handoff/design-table.md:60, :80, :27 (ec0f0ac) (DO-02, DO-19, DH-08); docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:51-52 (ec0f0ac); docs/progress/1.0/charter-2.0-draft.md:93, :106; docs/progress/1.0/command-table.md:31; docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:20; research/jumbling/manual/04-lattice.md:121-123 (ec0f0ac)
- needs_owner: no

#### A6

- on: D1
- kind: proposal
- claim: While the solver drags a turn in the cut slice, nothing may respond to alignment: no snap, detent or sound that favours aligning rotations (equal detents at every menu rotation are input, not a hint), and no live update of the three cut-cell states from the float preview. Those states keep the committed revision's values until the exact check on release; otherwise the dial would play out the closing twists as a hot-cold signal. In the same way, no check, list or preview gets a reward motion or sound, or any motion that differs by its outcome; the structure's own change (a seam closing) follows only the exact check of a committed state.
- evidence: docs/progress/1.0/jumbling-observation.md:34, :42, :119; docs/wiki/decisions/owner-decisions-2026-10-02-design.md:32, :90; docs/progress/1.0/charter-2.0-draft.md:115 (A1), :120 (A6)
- needs_owner: no

#### A7

- on: D1, D2, D3
- kind: proposal
- claim: A check shows three things apart: its evidence state as a pattern (exact at this revision, stale, or unchecked prefix, and an earlier revision's result never shown as current); the facts about the solver's word as numerals (N before and after, unit landed, collateral, other off pieces, off counts, the strict′ flag); and permission, shown only on the commit control. It carries no merged "good step" mark, no score such as change in N per twist, and no good-or-bad colour (colour carries a class only). A word that raises N or is not strict′ is shown as a fact, not as an error, because permission is not progress, and "keep only steps that lower N" is the manual's rule for the solver, not the program's.
- evidence: docs/progress/1.0/command-table.md:32; docs/progress/1.0/charter-2.0-draft.md:99; docs/wiki/decisions/owner-decisions-2026-10-02-design.md:36, :41; docs/progress/1.0/solving-workflow.md:82; docs/progress/1.0/dispositions/dispositions.md:624; docs/progress/1.0/jumbling-handoff/engineering.md:57 (ec0f0ac) (EH-25: "no verdict on whether a better step exists"); docs/progress/1.0/jumbling-handoff/design-table.md:76 (ec0f0ac) (DO-15); research/jumbling/manual/02-descent.md:22-23 (ec0f0ac)
- needs_owner: no

#### A8

- on: A1, A2
- kind: support
- claim: Counts by height and the u of a named cap are facts, and they fit the grammar as plain numerals: n₂, n₄, n₆ and one count above 6 next to N in the totals, and u inside the focus set on the named cap's blocked mark, since u > 0 is what blocks the cap. None of them gets progress styling such as a gauge, a bar or a "lower is better" colour, because N and height pull apart and such styling would tell the solver which measure to follow. Owner (DH-28): (a) show both this way; (b) counts by height only; (c) neither.
- evidence: docs/progress/1.0/jumbling-handoff/design-table.md:81, :52 (ec0f0ac) (DO-20, DH-28); docs/progress/1.0/jumbling-handoff/engineering.md:58 (ec0f0ac) (EH-26: u counts the pieces that straddle the cut and so block the cap); docs/progress/1.0/jumbling-observation.md:42, :132; docs/wiki/decisions/owner-decisions-2026-10-02-design.md:42
- needs_owner: yes

#### A9

- on: A3, C2
- kind: proposal
- claim: Draw ν as 45 fixed status marks (35 sign bits and 8 frame bits as set or clear; C5_17 and C5_28 as numerals 0 to 4), with the nine coordinates that are always 0 drawn as fixed structure. Never group the marks by table 4.7.3 or light the corrections that the set bits call for: that would draw the combination, which C2 leaves to the owner. If the retained residual views run while ν ≠ 0 (EH-18), a lone buffer swap, flip or turn that ν explains shows its ν coordinate, not the retained "impossible residual" mark.
- evidence: research/jumbling/manual/04-lattice.md:45-46, :64-66, :68-72, :97-100, :121-123 (ec0f0ac); docs/progress/1.0/solving-workflow.md:54; docs/progress/1.0/jumbling-handoff/engineering.md:39, :45 (ec0f0ac) (EH-12, EH-18)
- needs_owner: no

#### A10

- on: F1
- kind: question
- claim: The solved moment (S5) must fire on exactly the test that ends the solve, and nothing later may contradict it; yet a piece turned in a way that no sticker shows is invisible on screen. Owner: (a) exact pose equality, with any such turn drawn by the grammar's orientation glyph and counted, so the last work stays visible; (b) the sticker definition, celebrated on the labels, provided the engineer shows that no later refusal, such as Jumble off, depends on the hidden turn.
- evidence: docs/progress/1.0/jumbling-handoff/engineering.md:88, :108 (ec0f0ac) (EO-11, EO-20); docs/progress/1.0/charter-2.0-draft.md:84, :120; docs/wiki/decisions/owner-decisions-2026-10-02-design.md:32, :36; docs/progress/1.0/design-options-2026-10-round2.md:50 (orientation glyph); docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:20
- needs_owner: yes

#### A11

- on: G2
- kind: proposal
- claim: Either way, the three twists must read as one repair, because the opening twist x raises N by design (in the manual's tail example, cap 552 goes from 36 to 94). The state after x shows as "repair, 1 of 3" next to the word's total change in N, never as a regression or in an error style, and undo can take back the whole repair. Owner (DH-20): (a) one action plays x, y and z as three separate sweeps with one commit; (b) one action per twist, inside one open repair.
- evidence: research/jumbling/manual/02-descent.md:197-202 (ec0f0ac); docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:53-54 (ec0f0ac); docs/progress/1.0/jumbling-handoff/design-table.md:39 (ec0f0ac) (DH-20); docs/progress/1.0/charter-2.0-draft.md:83 (S4: the commit takes one action), :115 (A1); docs/progress/1.0/jumbling-handoff/engineering.md:92 (ec0f0ac) (EO-13: undo is always legal)
- needs_owner: yes

#### A12

- on: C1, C3, E3
- kind: proposal
- claim: C1 as stated is out: a target cap τ or unlock letters that the program picks are both a next-move mark and a target the program chose. In phase U, the strip slot that holds explained target candidates in the retained workflow stays empty, so the shared grammar cannot bring such a mark back; if the owner extends the exception to unlock letters at a cap the solver names (EH-27), that list reuses the repair list's grammar. An existence result (C3), if the owner allows one at all (EH-17), is a status on the target the solver named, with no word glyph, no length and no run, store or expand control, so it cannot read as a step.
- evidence: docs/progress/1.0/jumbling-observation.md:21, :42; docs/wiki/decisions/owner-decisions-2026-10-03.md:59; docs/progress/1.0/dispositions/dispositions.md:250 (U2-068, retained candidates); docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:57-59 (ec0f0ac); docs/progress/1.0/jumbling-handoff/engineering.md:44, :59 (ec0f0ac) (EH-17, EH-27); docs/progress/1.0/jumbling-handoff/design-table.md:49 (ec0f0ac) (DH-25)
- needs_owner: no

### Mathematician

#### M1

- on: A, A1, C1 (rule 6)
- kind: proposal
- claim: Every function of the configuration counts as a "fact", so rule 6 needs a sharper test: a quantity may be shown when it is static (defined at one revision with no quantifier, minimum or argmax over twists or words, as N_c, the floor, blocked, u, the height counts n_h, ν, domains and an exact covers are) and is shown per cap or for what the solver names, never as the result of the method's own selection (largest, top, first); on that test A1 is allowed as stated. The records contain counterexamples: the repair-list length per cap and table 3.2's "Carrier of c" ("one twist of c puts it back on the lattice") quantify over twists and so mark caps that have a helpful twist; the cover top and the research target cap and unlock letters (C1) are selections; "Fragment" has no definition ("small", "far more"); the repair list itself is the one owner exception.
- evidence: docs/progress/1.0/jumbling-observation.md:42 (rule 6), :72 (Carrier), :73 (Fragment); docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:51-52 (ec0f0ac) (the program does not choose the cap); research/jumbling/restoration/12-low-height/README.md:1152-1156 (ec0f0ac) (target cap τ = the cap whose best item lowers the most, ties to the lower cap); research/jumbling/theory/seams.py:194-197 (cover top). Heights are static facts: research/jumbling/theory/restoration.md:20 (T2: d_L is a tree distance at the prime 2); :27 (K⁺ fixes v₀, so d_L is constant on a domain γK⁺ and invariant under K⁺); :32 (T4: each I_a letter displaces v₀ by 0 or 2, and tree distances add mod 2, so left heights are even); :43 (T7: left height h needs at least h/2 more jumble twists, which is a truthful reading for the UI); research/jumbling/restoration/10-x1-unbounded/README.md:8-9 (heights are unbounded, so a "higher" bucket needs a stated threshold).
- needs_owner: no

#### M2

- on: A2, D1
- kind: proposal
- claim: The straddler count u_c is exact and static: each straddler has a certificate of two exact points, the class depends only on c's cut and not on the twist, and u_c > 0 exactly when c is blocked, so A2 is allowed as stated. u_c also fixes a definite description that fails: rule 5, table 3.2 "Blocked by", the slice in section 5 and the check "by which piece" all speak of "the straddling piece", yet one recorded state has 44 straddlers at cap 150, 30 at cap 104 and 3 at cap 274, so "blocked by" must be the straddler set of size u_c, each with its certificate pair, not the one witness a search happened to find.
- evidence: research/jumbling/state-contract.md:30, :36, :43, :47-48; docs/progress/1.0/jumbling-observation.md:38, :74, :122, :163; research/jumbling/manual/01-what-you-see.md:24 (ec0f0ac); research/jumbling/restoration/12-low-height/README.md:1156, :1168-1173 (ec0f0ac) (at state 1ff0061f…: u = 44 at cap 150; u from 30 to 3 to 0 at cap 104; u from 3 to 0 at cap 274); research/jumbling/fixtures/wj-I_a.json:1 (end_survey: 573 blocked grips and exactly one certificate piece per grip, counted with python -I).
- needs_owner: no

#### M3

- on: A3, F2
- kind: proposal
- claim: ν is defined only at N = 0 and only through the labels π(X): its sign bits and abelianised frame sums do not depend on the choice of coherent frames, but each C5 value depends on the generator that nucoords.py picks, so table 4.7.2's "by 4" and "by 1" hold only under that stated convention, and a nonzero value in one of the nine always-zero coordinates is an engine defect, not a state. For F2, ν ≠ 0 proves that no retained word reaches the labels, while ν = 0 makes them reachable by the certificate G = 𝒦 but gives no witness word, so refusing Jumble off at ν = 0 is ENG-0J's witness policy, and the refusal message must not say the state is unreachable.
- evidence: research/jumbling/state-contract.md:144-146; research/jumbling/restoration/03-retained-group-and-lattice-restorer/scripts/nucoords.py:74 (nu reads a labelled state); nucoords.py:64 (r0 = min(...) picks the C5 generator; using r0^k instead multiplies every C5 value by k⁻¹ mod 5); research/Full_600cell_Puzzle_Theory.tex:997 (Lemma 7: changing coherent frames conjugates the coordinates); research/Full_600cell_Puzzle_Theory.tex:1028-1036, :1043 (Lemma 8: Φ_o and the sign are homomorphisms to abelian groups, so conjugation leaves them unchanged); research/jumbling/manual/04-lattice.md:47-50, :61-66, :101-104 (ec0f0ac); docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:36.
- needs_owner: no

#### M4

- on: A4
- kind: proposal
- claim: Covers can be defined exactly by side tests: for an admissible cap c and a cap d that meets it, let S be the face pieces of c's cut that are certified inside d (a straddler of d is not inside); then d covers c when S has equal counts, at least ten, inside and outside c, and every piece of S sits on an aligned cell, with ten as a stated parameter. Up to rounding, this agrees with the research float test on every piece that does not straddle d, so EH-28's on-plane case can arise only when d is blocked; only the answer for two caps the solver names is a fact, while marking the cover top (covers some cap, covered by none) applies the rule's own selection (M1).
- evidence: research/jumbling/theory/seams.py:35-36 (float centroids), :174 (centroid strictly beyond d's cut), :177 (i ≥ 10 and i = o = ai = ao), :194-197 (top); research/jumbling/manual/02-descent.md:100-104 (ec0f0ac); docs/progress/1.0/jumbling-handoff/engineering.md:60 (ec0f0ac) (EH-28); research/jumbling/state-contract.md:42-43. Sketch: a full-dimensional region inside {h ≥ 0} has a vertex with h > 0, so its centroid (vertex mean or volume centroid) has h > 0; h(centroid) = 0 therefore forces both signs, which makes the piece a straddler.
- needs_owner: no

#### M5

- on: B
- kind: proposal
- claim: Options 2 and 3 pass the filter: a threshold filter, or a sort the solver picks each time, by one static fact, with ties broken by the fixed cap index (a total order) and recomputed at each revision, shows nothing beyond the facts. In option 4 the program chooses the key and direction, and N_c descending is exactly the manual's naming order, which put the repair cap first or second at 41 of 63 steps, so it ranks caps by promise in effect; owner: (a) options 1 and 2 only, (b) also option 3, (c) also option 4.
- evidence: research/jumbling/manual/02-descent.md:152, :169-171 (ec0f0ac) (decreasing N_c, ties to the smaller cap number); docs/progress/1.0/jumbling-handoff/design-table.md:60 (ec0f0ac) (DO-02); research/jumbling/theory/seams.py:192 (research selections sort by drop, then cap); docs/progress/1.0/jumbling-observation.md:35 (rule 3). A sort without a tie rule is not a function of the facts, and equal N_c values are common (1,566 and 1,360 recur in research/jumbling/manual/02-descent.md:67-75 (ec0f0ac)).
- needs_owner: yes

#### M6

- on: C2, E2, E4
- kind: objection
- claim: Because the 36 corrections are a basis of A_J, the corrections that ν calls for are the coordinates of ν in that basis; that is a forced function of ν, but output as macros to make it is a solving macro the program picks (out under the 2 and 3 October rulings: "never picks a ... correction by itself"), and showing ν in the correction basis is the same output; owner question (EH-12): (a) out, with ν shown in orbit coordinates plus the static table 4.7.3, (b) a check of a set of corrections the solver proposes (ν after it, zero or not), (c) the program lists the combination. For E2, each correction acts the same way from every lattice configuration (proved), so selecting one by its number fits the 3 October notion of a known construction; the stage 12 steps of E4 do not, because their effect changes with the state.
- evidence: research/jumbling/manual/04-lattice.md:78-81 (basis), :84-88 (same action from every lattice configuration; ν adds), :97-105 (a C5 part can be made v times, or its inverse 5 − v times, and the order changes the labels but not ν, so even the combination is not one word), :121-123 (ec0f0ac); docs/wiki/decisions/owner-decisions-2026-10-02.md:59; docs/wiki/decisions/owner-decisions-2026-10-03.md:57, :59; research/jumbling/restoration/12-low-height/README.md:958 (ec0f0ac) (b14's word is admissible after b12's step but takes 17 lattice pieces off).
- needs_owner: yes

#### M7

- on: C3, E3, D4
- kind: proposal
- claim: Corollary A1 answers a narrower question than "is there a step": N1, N2 and Jordan certificates prove that some SAFE-ALL word (retained letters of unblocked caps that hold no off piece) vacates the chamber set T; a failed N1 or N2 proves only that no SAFE-ALL word does; other cases are unknown; and no case gives a word or a length, so a C3 answer must never read "a step exists" or "no step exists", and E3 holds. E3 stays true only if the word store keeps flat words or shows the exact expanded length before a run, because parts reused by name form a straight-line program, which can hold b19's built V (12,688 nodes, under 10^141 letters); owner question (EH-17): (a) C3 not shown in 1.0, (b) C3's three-valued answer shown for a T the solver names, (c) C3 only inside a V search the owner allows (M8).
- evidence: research/jumbling/theory/buffer-transport.md:10 (SAFE-ALL), :13 (vacating T), :17-19 (N1, N2 necessary), :37 (Corollary A1), :42 (no length bound), :43 (orbits of 2 to 7 chambers have no Jordan certificate); research/jumbling/restoration/12-low-height/README.md:597-599 (ec0f0ac); docs/progress/1.0/jumbling-handoff/design-table.md:47, :80 (ec0f0ac) (DH-23 same_as parts; DO-19); docs/progress/1.0/jumbling-handoff/engineering.md:54, :114 (ec0f0ac) (EH-22: the runner stores the flat word beside its tree; EO-23). The same logic applies to D4: only a completed search proves "none in this finite set", and a cut-off search proves nothing.
- needs_owner: yes

#### M8

- on: C4
- kind: proposal
- claim: With Jumble on, the 2 October setup search keeps its exact meaning at N = 0, whatever ν is: every position is filled, every retained word is admissible because every lattice configuration has the solved shape, and a setup moves positions as in 600-cell-Full, so a target is a position as in the glossary (ENG-0J's refusal of retained tools while Jumble is on is a separate design choice, EH-18). Off the lattice, a "target" would be a chamber set T and its "setup" a vacating word V, since a clean step V·W·V⁻¹ has the conjugate shape of a star, so a V search is a search for most of a step and needs an owner extension: (a) setup search at N = 0 only, (b) also a V search for a T the solver names, (c) no search while Jumble is on.
- evidence: docs/wiki/decisions/owner-decisions-2026-10-02.md:59; research/jumbling/manual/04-lattice.md:84-87 (ec0f0ac); docs/progress/1.0/glossary/glossary.md:194, :204, :214 (Setup, Star, Target); research/Full_600cell_Puzzle_Theory.tex:1471 (Lemma 15: guarded setup graphs fix the buffers); research/jumbling/manual/03-tail.md:20 (ec0f0ac) (V·W·V⁻¹); research/jumbling/theory/buffer-transport.md:13; docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:20, :37; docs/progress/1.0/jumbling-handoff/engineering.md:45 (ec0f0ac) (EH-18).
- needs_owner: yes

#### M9

- on: D2, D3
- kind: proposal
- claim: For a unit U the solver names and a word w at revision X, the D2 fields are exact end-state counts, and strict′(X, w, U) means exactly "w is admissible at every prefix, all of U is on the lattice, no lattice piece of X is off, and every other off piece is on the lattice or at its exact pose"; so the flag is the conjunction of the counts, a fact about the solver's word and never a verdict on other steps, and D3 is sound because every field is a function of (X, w, U) alone. The flag needs three truth conditions shown with it: it is end-state only (a lattice piece may leave the lattice partway through the word), lattice pieces may change slot (labels are not kept), and "exact pose" means pose equality, so a piece turned only by one of its own symmetries counts as moved (M10).
- evidence: research/jumbling/restoration/12-low-height/README.md:22 (unit), :26-28 (strict′, R21F-01) (ec0f0ac); research/jumbling/manual/03-tail.md:17-19, :26 (ec0f0ac); docs/progress/1.0/design-table/topics/jumble-solver-aids.md:39-44; docs/progress/1.0/jumbling-handoff/engineering.md:56-57 (ec0f0ac) (EH-24: b03's stored word is not strict′ at 737; EH-25: b20's fields); research/jumbling/state-contract.md:146.
- needs_owner: no

#### M10

- on: F1, F2
- kind: proposal
- claim: On the lattice, two states with equal labels differ only by rotations s in K⁺ that map a piece's home region and stickers onto themselves; since twists act on the left and s on the right, every later region, sticker, domain, admissibility and N agree, so pose equality is invisible to the solver (the runner's fixture ends with identity labels and another digest), and the sticker test, every one of the 259,800 slots carrying its home label (hence N = 0), is the exact end test the view can show. With F2 this gives the owner three options: (1) the sticker test, with Jumble off allowed on a label-level witness (the empty word at a solved end, since 600-cell-Full's state is its labels; contract section 5 amended); (2) pose equality with a visible mark on every piece that differs, although no record shows that such rotations can always be removed; (3) pose equality unmarked, which is out because it reports "not solved" with nothing to see.
- evidence: research/jumbling/state-contract.md:25, :142-146; research/jumbling/theory/restoration.md:39 (T6: a domain is a coset γK⁺, so γsK⁺ = γK⁺); research/jumbling/restoration/13-tutorial-runner/README.md:540-542, :594-598 (ec0f0ac); research/jumbling/manual/04-lattice.md:185-186 (ec0f0ac) ("which nothing on the screen can show"); docs/wiki/decisions/owner-decisions-2026-10-15-jumbling-restored.md:21, :37; docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:24, :33, :36; docs/progress/1.0/jumbling-handoff/engineering.md:45 (ec0f0ac) (a change to contract section 5 gets the senior reviewer's check). Sketch: if every sticker has the same slot and orientation in X and Y, then s_A = (k_A^Y)⁻¹k_A^X lies in K⁺ and fixes the home region and each sticker; after any word the poses are g·k_A^X = g·k_A^Y·s_A, so posed regions and inside sets coincide at every step.
- needs_owner: yes

#### M11

- on: G3, E1
- kind: objection
- claim: "Menu index" is not intrinsic: a cap's items are the base items conjugated by its frame F_c, and the menu identity hashes the sorted matrices, so two records with the same identity can number the items differently; that changes the repair list's x order, the y and z that ties select, and through them which pairs are listed (EO-04). G3 is therefore well defined only with the ordered I_a record pinned beside the identity (F_c is already fixed by the model); words stored as (grip, menu index) pairs need the same pin or exact matrices (E1, EH-14); and "best" then means lowest N_d (or N_c) with ties to the smaller pinned index, which reproduces the measured lists.
- evidence: research/jumbling/sim/twists.py:482-483 (identity = sha256 of the sorted matrices), :491-503 (items transported by F_c in record order), :551-556 (a record is rebuilt in its own order, and only the identity is compared); research/jumbling/restoration/11-scramble-census/scripts/density.py:29, :38; docs/progress/1.0/jumbling-handoff/engineering.md:41, :74 (ec0f0ac) (EH-14, EO-04); docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:48-50, :55 (ec0f0ac). The neighbour set for d is already exact: research/jumbling/theory/restore.py:57 takes pole angles below 55.7° in floats, and a synthetic computation (the 600-cell's 120 vertices, its 600 cells as 4-cliques at edge length 1/φ, normalised centres, pure Python with -I) finds no pole angle between 49.118° and 56.012°, so the table equals "angle below 2θ = 52.67°", with 80 meeting caps for every cap.
- needs_owner: no

#### M12

- on: DH-09
- kind: proposal
- claim: The rule of one term per concept fails at Block (the solver's declared achievement, versus the tail's group of off pieces that no cut separates) and at Twist (one of the 1,200 generators or an inverse, versus a turn (c, g) by any element of the cap's menu, where 48 of I_a's 60 items are jumble twists and at least 7 of a cap's 11 non-identity retained items are not a single generator), and it also fails at Frame, Collateral, Strict, Target and, in part, Cap (see evidence). Proposal, with the names as preference: keep every glossary meaning; make Twist the genus "a turn of one cap by an element of its menu", with retained twist and jumble twist as species, keep Generator for the 1,200, and have every length name its alphabet; rename the tail's block to cluster (a connected component, see evidence), with a unit being one domain part of a cluster; call rule 5's collateral the moving set and D2's collateral lifted; give strict′ a name without the word "strict"; and add the terms this topic shows (lattice and off the lattice, menu, domain, height, seam, misalignment N_c and N, floor, straddler and u, covers, repair list, step check, ν, correction, and solved as in M10), with their levels set after DH-27.
- evidence: docs/progress/1.0/glossary/glossary.md:90 (Twist), :226 (Block), :246 (Frame) against research/jumbling/manual/03-tail.md:17 (ec0f0ac) ("frame part" means a domain part); docs/progress/1.0/glossary/glossary.md:102 (Collateral) against docs/progress/1.0/jumbling-observation.md:39 (everything a preview moves) and docs/progress/1.0/design-table/topics/jumble-solver-aids.md:41 (lattice pieces taken off); docs/progress/1.0/glossary/glossary.md:112 (Strict: checked after every generator) against research/jumbling/restoration/12-low-height/README.md:26-28 (ec0f0ac) (an end-state property); docs/progress/1.0/glossary/glossary.md:214 (Target: a position) against research/jumbling/theory/buffer-transport.md:13 (a chamber set) and research/jumbling/restoration/12-low-height/README.md:1153-1155 (ec0f0ac) (target cap τ); docs/progress/1.0/glossary/glossary.md:20 (Cap: a fixed set of pieces) against docs/progress/1.0/jumbling-observation.md:58 (the inside set changes, and is undefined while the cap is blocked). Counterexample for Block: with P inside c, R straddling c, Q outside c and no other cut between them, no cut separates P from R or R from Q, yet c separates P from Q; so research/jumbling/manual/03-tail.md:9 (ec0f0ac) must read "connected through pieces that no cut separates", which is what research/jumbling/theory/blocks.py:59-77 computes. Twist: research/jumbling/state-contract.md:12; research/jumbling/theory/restoration.md:32; research/jumbling/manual/01-what-you-see.md:8-10 (ec0f0ac) (12 retained rotations per cap); research/jumbling/sim/twists.py:313 (two generators per grip, so four moves per cap against the 11 non-identity elements of A4).
- needs_owner: no

### Engineer's advisor

#### E1

- on: A1, A2, B
- kind: support
- claim: A1 is cheap. It is a per-revision histogram over the pose table, built from the per-pose heights the requirements already propose and updated from each twist's moving set; its keys are at most twice the journal length, so it is a sparse map with 64-bit keys, not fixed n₂, n₄, n₆ slots, and only the pose entries need big integers. A2 at a cap the solver names costs one full classification that counts and certifies every straddler instead of stopping at the first, and B by N_c or the floor is a view-model sort of the 600-row rim summary; but any order, filter or cap-map mark by u needs 600 such classifications per revision, which is unmeasured at states with 528 to 578 caps blocked, so offer u only on request until it is measured.
- evidence: docs/progress/1.0/jumbling-observation.md:171, :173; docs/progress/1.0/jumbling-handoff/engineering.md:21, :58, :61 (ec0f0ac) (EH-29: at most 2 per letter, d_R = 0, proved by model review only); research/jumbling/theory/restore.py:177-189 and research/jumbling/theory/trees.py:372-403 (research heights per pose key, fixture histograms); research/jumbling/sim/state.py:271-272, :409 and research/jumbling/theory/restore.py:114 (the survey and the research restorer stop at the first straddler); research/jumbling/state-contract.md:40-43 (one certificate per straddler); research/jumbling/theory/buffer-transport.md:10 (lattice pieces never straddle, so u counts only off pieces); docs/progress/1.0/jumbling-handoff/renderer.md:60 (ec0f0ac) (181 to 827 straddlers per blocked grip at S1); docs/progress/1.0/jumbling-handoff/design-table.md:79 (ec0f0ac). Experiment: in EN2, time a 600-cap survey with early exit against 600 certified straddler counts on one rebuilt height-4 state and on one 3000-twist scramble (renderer.md:48 (ec0f0ac)), as an ENG-0J item 5 cost lead.
- needs_owner: no

#### E2

- on: A3, C2
- kind: proposal
- claim: A3 is cheap and exact. ν depends on π(X) alone and takes one pass over the 259,800 slots, but it exists only on lattice configurations, so off the lattice its field reads "not on the lattice" and never shows 0 or a stale value. Cost does not decide C2 either, because the combination is a small row reduction over F2 and F5 that research code already does. Owner question (EH-12): (a) the solver works the combination out from table 4.7.3, and the program checks ν after each correction; (b) the program also checks a combination the solver proposes, before any twist, which is exact because ν adds; (c) the program shows the combination ν calls for.
- evidence: research/jumbling/restoration/03-retained-group-and-lattice-restorer/scripts/nucoords.py:74-105 (ν from labels), :170-196 (combination solver); research/jumbling/state-contract.md:142-146; research/jumbling/manual/04-lattice.md:88, :97-105, :121-123 (ec0f0ac); docs/progress/1.0/jumbling-observation.md:35, :42; docs/progress/1.0/jumbling-handoff/engineering.md:37, :39 (ec0f0ac). Check: EH-10's check (equal to nucoords.py on the research lattice states, and 0 after any retained word).
- needs_owner: yes

#### E3

- on: A4
- kind: proposal
- claim: The research covers test cannot become the product's. It compares float centroids with the cut by a strict float test, so for a centroid exactly on the plane an unpinned BLAS kernel decides, which breaks rule 2. A4 is feasible under an exact definition: the vertex-average centroid of a posed piece is exact in Q(√5), so each side test is an exact sign. The definition must state the on-plane case, the pieces that straddle d's cut, and the threshold of ten pieces per side.
- evidence: research/jumbling/theory/seams.py:10-11, :83-91, :174, :177; research/jumbling/fixtures/wj.py:83-86; research/jumbling/restoration/12-low-height/README.md:1224-1226 (ec0f0ac) (a finding from reading the source; no actual disagreement is recorded); docs/progress/1.0/jumbling-observation.md:34. Experiment: EH-28's check (agreement with cover_graph at the census cover steps, each disagreement traced; docs/progress/1.0/jumbling-handoff/engineering.md:60 (ec0f0ac)), plus the time of the exact test per pair of caps.
- needs_owner: no

#### E4

- on: D1, D2, D3
- kind: proposal
- claim: D1 and D3 are one replay of the solver's word at the current revision, checking admissibility and the twisted cap's N change at every prefix, so checking a long word is a cancellable job. If the revision and the word digest have not changed, the commit takes the end state from that fresh preview instead of replaying again, so operation.execute stays a commit without a job, as the table has it. D2's fields come from one pass over the 177,120 pieces that compares start and end poses by exact key, never by pose-table index (J1 renumbers poses after every twist); the unit is an input the solver names at that revision.
- evidence: docs/progress/1.0/jumbling-handoff/engineering.md:70 (EO-02: N changes only along the twisted cap's cut), :57 (EH-25), :110-112 (EO-21, EO-22: a composed fast path needs an agreement test against per-twist replay) (ec0f0ac); docs/progress/1.0/command-table.json:511-528; research/jumbling/sim/state.py:385-397; research/jumbling/theory/replay_check.py:67-69; research/jumbling/restoration/12-low-height/runs/round22/line-a/literal/run1/03-b20-stepchk.json:2-14 (ec0f0ac). The product replay rate is unmeasured. Experiment: in EN2, time the preview of the recorded words of 624 and 1,158 twists, correction 35 (5,629 twists) and b20 (40,500 twists) from their recorded start states with the candidate engine. For reference, research code replayed b20 in 470 s, a cloud timing and not performance evidence.
- needs_owner: no

#### E5

- on: D4, G, C3
- kind: proposal
- claim: Adopt this first, because the research tools have already failed exactly this way (P3-falsepass-01, a confirmed major finding). Every job result is typed as completed, cancelled, cut off at a resource limit, or error. It carries its input digest, its parameters (cap, order-pinned menu identity, tie rule, contract revision) and what it covered, and the view model reads it only after its digest matches the shown revision. An empty repair list comes only from a completed job. A proved negative, such as a failed N1 or N2 under C3 if that is allowed, is the one true "no such word" answer and must state its scope. The open unknown is how long one repair-list request takes in the product: research code took 3 to 5 minutes per cap (cloud timings), and the right cap was named as late as 28th. Experiment: time it per cap at the 63 recorded steps with the candidate engine, split over the (x, d) pairs, which are independent at one revision.
- evidence: docs/progress/1.0/command-table.md:31, :47; research/jumbling/restoration/13-tutorial-runner/README.md:985-989, :361 (ec0f0ac); docs/progress/1.0/jumbling-handoff/engineering.md:29, :72, :114, :116 (ec0f0ac); docs/progress/1.0/jumbling-handoff/design-table.md:60, :80 (ec0f0ac); research/jumbling/restoration/11-scramble-census/scripts/density.py:21-44 (each pair applies and undoes on one state); research/jumbling/theory/buffer-transport.md:17-19, :37 (N1 and N2 are necessary, and sufficient only with Jordan certificates).
- needs_owner: no

#### E6

- on: G3, D3, E1
- kind: objection
- claim: Demonstrated: J1's menu identity ignores item order, but "x in menu order", the tie rule "smaller menu index" and stored (grip, menu index) words all depend on it. Recomputing the identity from the wj-I_a record with its base list reversed gives the same fddc8532…, and 0 of 60 indices keep their rotation. Fix first: freeze an order digest next to the signed identity and cite it in every stored word and list result. G3 then needs no new rule, because the owner confirmed the pairs as the measurement counted them: lowest N_d, then lowest N_c, ties to the smaller index, with the identity at index 0.
- evidence: research/jumbling/sim/twists.py:445-474 (items kept in insertion order), :482-483 (identity hashed over sorted matrices), :511-514 (the record keeps the base order). Measured here, read-only: SHA-256 over the sorted JSON of each base matrix of the `menu` record in research/jumbling/fixtures/wj-I_a.json gives fddc8532… both in record order and reversed; 0 of 60 indices map to the same rotation; index 0 is the identity. Also docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:48-50, :55 (ec0f0ac); research/jumbling/restoration/11-scramble-census/scripts/density.py:29, :38; docs/progress/1.0/jumbling-handoff/engineering.md:41, :74 (ec0f0ac); research/jumbling/restoration/13-tutorial-runner/README.md:464-466 (ec0f0ac) (ia16 codes are 60·grip + item).
- needs_owner: no

#### E7

- on: G2
- kind: support
- claim: Both G2 flows are feasible. One action for all three twists is a short word on the D3 path. One action per twist needs the list result to carry the predicted digests after x and after y, for the listed pairs only, so that a later twist is refused as stale if anything else changed. In the measured lists, z is the identity item for 86 of the 679 listed pairs, and for every listed pair at 6 of the 63 steps. Such a pair is a two-twist repair, and the program must never ask the solver to make an identity twist.
- evidence: counted here, read-only, from research/jumbling/restoration/11-scramble-census/runs/findability/ia-k40-s7101-density.jsonl.gz and ia-k40-s7101-density-rest.jsonl.gz (ec0f0ac): 63 steps, 679 pairs with delta < 0, 86 of them with z = item 0, which is the identity in the I_a record. research/jumbling/restoration/11-scramble-census/scripts/density.py:37-38 (z ranges over every item) and :63-65 (x skips the identity); docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:53-54 (ec0f0ac); docs/progress/1.0/jumbling-handoff/design-table.md:39 (ec0f0ac). How such a pair is shown belongs to DT-J4.
- needs_owner: no

#### E8

- on: F2, C4
- kind: objection
- claim: Jumble off at ν ≠ 0 is out, and this is demonstrated. With ν ≠ 0, π(X) lies outside G = 𝒦, so no retained word reaches it, and 600-cell-Full would hold a state with no legal witness whose ν-residuals its endgame would read as provenance errors. Under ENG-0J as written, such a state (any correction made from solved is one) also refuses the retained tools, and contract section 5 limits even the setup tables (C4) to witnessed checkpoints, so the lattice phase has only single twists. Owner question: (a) the lattice phase stays in Jumble, with retained tools and setup search acting on π(X) at lattice states, and invariant-based certificates only at ν = 0; (b) after the corrections, Jumble off at ν = 0 through a label checkpoint certified by ν = 0, which stores the exit pose state so that Jumble on can resume, and 600-cell-Full's tools run unchanged; this is the cheapest; (c) both.
- evidence: research/jumbling/manual/04-lattice.md:45-50 (G = 𝒦, a computed certificate), :68-72, :84-88 (ec0f0ac); docs/progress/1.0/charter-2.0-draft.md:11; docs/progress/1.0/solving-workflow.md:54; research/jumbling/state-contract.md:74-77, :146; docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:20, :35-37, :74; docs/progress/1.0/jumbling-handoff/engineering.md:38 (EH-11's check: a correction made from solved changes ν by its table row), :45, :86 (ec0f0ac); research/jumbling/sim/README.md:140 (no witness search); docs/progress/1.0/replan-2026-10-22.md:72. Either option changes contract section 5 and ENG-0J acceptance item 7, so it goes through EN1's Astra plan check.
- needs_owner: yes

#### E9

- on: F1
- kind: question
- claim: Both end tests are cheap: pose equality is a digest comparison with solved, and the sticker test is one π pass over 259,800 stickers. But they give different answers on reachable states, nothing on screen shows the difference, and under the current contract a sticker-solved state whose poses differ has no witness the program can use for Jumble off. Owner question: (a) pose equality, plus a filter "home labels, other pose" so that the difference shows; (b) the sticker definition, with Jumble off at such a state through E8(b)'s ν = 0 label checkpoint; (c) both tests, shown as two separate facts.
- evidence: research/jumbling/state-contract.md:25, :146; research/jumbling/sim/state.py:473-476; research/jumbling/sim/README.md:140; docs/wiki/decisions/owner-decisions-2026-10-15-jumbling-restored.md:21, :37; docs/progress/1.0/jumbling-handoff/engineering.md:88, :108 (ec0f0ac) (EO-20: a research fixture ends with identity labels and another digest, measured); docs/progress/1.0/jumbling-handoff/design-table.md:97 (ec0f0ac).
- needs_owner: yes

#### E10

- on: E1, E3, E4
- kind: proposal
- claim: E1 is inside the boundary and needs no second macro path. macro.add and macro.save store a jumble word, operation.check and operation.preview (both jobs) check it, and operation.execute (fresh preview required) commits it, with the engine deciding availability per mode. Beyond ENG-0J's proposed rows, DT-J1 then adds only query rows: covers (A4), the repair list (a job) and, if allowed, the C2 combination check or C3. A stored word is a literal list of (grip, item) letters with the model identity, E6's order-pinned menu identity, the contract revision and the start digest. It has no length cap (672,417 letters on record, about 1.3 MB at two bytes a letter) and no certificate or SLP part (E3), and a packaging check keeps research word files out of the build (E4).
- evidence: docs/wiki/decisions/owner-decisions-2026-10-02.md:59; docs/progress/1.0/command-table.json:470, :490, :511, :1051, :1146; docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:20, :37, :74; docs/progress/1.0/jumbling-handoff/engineering.md:41, :54, :92, :100 (ec0f0ac); research/jumbling/restoration/13-tutorial-runner/README.md:455, :464-466 (ec0f0ac); docs/progress/1.0/jumbling-handoff/design-table.md:48-49 (ec0f0ac). Sketch: undoing a committed word can restore its start snapshot (177,120 int32 pose ids, about 0.7 MB, plus the pose table) instead of replaying the inverse; EO-13 makes the inverse replay legal, but it costs a full replay. Check: EH-14's store round trip (correction 35 and b12's step end at J1's digests).
- needs_owner: no

#### E11

- on: E2, E4
- kind: question
- claim: Cost does not decide E2. All 36 corrections together are under 25,852 twists (about 48 KB), and each is admissible from every reachable lattice configuration with a fixed ν change, so shipped words can be verified once in CI and again by the normal whole-word preview each time they are used. But they are words found by research, so shipping them needs a named exception to E4. Owner question: (a) ship them as constructions the solver selects, extending the 3 October ruling to them, as the one named exception to E4; (b) the solver enters them from the manual's table, and the program stores and checks them as the solver's own words.
- evidence: research/jumbling/manual/04-lattice.md:84-88, :111-113 (the 34 bit corrections hold 22,012 twists; the two C5 corrections hold 1,920 together, derived from 3,840 for two uses of each), :121-123 (ec0f0ac); docs/wiki/decisions/owner-decisions-2026-10-03.md:57-59; docs/progress/1.0/jumbling-handoff/engineering.md:38 (EH-11: 5 to 5,629 twists each, from the stage 09 research library; its check would be the CI check), :41 (ec0f0ac); docs/progress/1.0/jumbling-handoff/design-table.md:48 (ec0f0ac).
- needs_owner: yes

#### E12

- on: A1-A4, D2, G
- kind: proposal
- claim: Every DT-J1 aid sits in the portable layers (engine, session store, command layer, view model) and can be tested headless against J1. The renderer receives only counts and lists for panels, plus per-piece masks bound to one revision (177,120 bits, about 22 KB each): straddlers, covered pieces, collateral, and the moving sets of a repair pair's x, y and z; no new geometry is needed. Their frame cost is a highlight-and-dim overlay, which is unmeasured: W-J item 7 is still open, and the research viewer needed dimming before a straddler highlight could be read. Add it to the item 7 series on wj-I_a and on one deep export in the next attended session on the owner's machine, and claim no cost before then.
- evidence: docs/progress/1.0/jumbling-observation.md:35, :176; docs/progress/1.0/jumbling-handoff/renderer.md:21, :31, :48, :60 (ec0f0ac); docs/progress/1.0/stage-2-experiment-protocol.md:160, :163 (H-06 and H-09 still open); docs/progress/1.0/replan-2026-10-22.md:81 (R1); docs/development-guide/AGENT_BRIEFING.md:28 (portable layers).
- needs_owner: no

## Round 2

Run on 22 October 2026. Each role read the whole record, round 1 included, and answered the other roles' cards by ID. The integrator had re-checked E6 before the round and told the roles so. Cards are appended unchanged; only the layout differs, with one list item per field and nested lists kept.

### Front-end designer

#### D13

- on: E6, M11, A3, D11 (items G3 and G1)
- kind: proposal
- claim: I support E6 and M11, and I change D11 on G3: the tie rule and the neutral order are defined only once the I_a order is pinned beside the identity, and in that order row 1 is the pair the census rule itself took at all 63 repair steps of the record, so a focus that starts on row 1 marks the method's own pick (M1's "first"). Owner, G1: (a) the list opens with no row focused, and a number key or an arrow key focuses a row (D11(b), A3(a)), which I recommend because keys 1 to 9 reach every row of 43 of the 63 lists; (b) A3(b), a focus ring on row 1 with nothing drawn on the geometry; A3(c), typing x and d, costs up to five digits per pick and gains nothing over (a).
- evidence: new evidence: counted here, read-only, from research/jumbling/restoration/11-scramble-census/runs/findability/ia-k40-s7101-density.jsonl.gz and ia-k40-s7101-density-rest.jsonl.gz (ec0f0ac). Ordered by x, then d, the record's own pair (`chosen`) is the first row with delta < 0 at 63 of 63 steps. The lists hold 1 to 61 rows (679 in all), and 43 of the 63 lists have at most 9 rows, all of them from step 44 on. Also research/jumbling/restoration/11-scramble-census/scripts/density.py:61-70 (ec0f0ac) (x in menu order, d in cap order); research/jumbling/manual/02-descent.md:151-153 (ec0f0ac) (the census kept the first word that lowered N); docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:55 (ec0f0ac); research/jumbling/sim/twists.py:482-483.
- needs_owner: yes

#### D14

- on: A11, E7, D11 (item G2)
- kind: proposal
- claim: A11 and E7 answer D11's concern that y and z are lost after x: in either flow the picked pair stays one open repair until it is committed or cancelled, undo takes the whole repair back, and when z is the identity the repair has two twists, so the program never asks for an identity twist (86 of the 679 listed rows). Owner, G2: (a) one commit of the whole repair after a preview the solver can step through twist by twist (D11(b), A11(a)), which I recommend because the state after x, which raises N by design, never becomes a committed revision and S4's commit stays one action; (b) one action and one commit per twist inside the open repair, kept valid by the predicted digests after x and after y (A11(b), E7).
- evidence: no new evidence. docs/progress/1.0/charter-2.0-draft.md:83 (S4); docs/progress/1.0/jumbling-observation.md:35 (rule 3); research/jumbling/manual/02-descent.md:197-202 (ec0f0ac) (the opening raises N_552 from 36 to 94); docs/progress/1.0/jumbling-handoff/engineering.md:92 (ec0f0ac) (EO-13). E7's count of 86 is reproduced by the same density files.
- needs_owner: yes

#### D15

- on: A1, M5, E1, D4 (item B)
- kind: proposal
- claim: Option 4 is out (A1, M5, D4), and because the owner-confirmed scope of the exception says "no view … ranks caps" for the face match and the cover step, without rule 6's "by how promising", even a sort the solver chooses needs the owner's ruling; I drop D4's saved setting, and with E1 nothing orders, filters or marks caps by u. Owner: (a) option 1; (b) option 2, a threshold on N_c or the floor that the solver sets, drawn on the fixed map (A1), with a key that names the next lit cap in cap-index order; (c) (b) plus a sort by N_c or the floor, ties going to the cap index, that the solver chooses again at each descent step and that the key then follows (M5; A1(c) proposes it as a list), which I recommend if the owner admits it, because the manual's walk then costs one key per naming (201 namings over the record's 63 repair steps) where (b) needs threshold edits in place of the order.
- evidence: new evidence: docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:57-59 (ec0f0ac), read against docs/progress/1.0/jumbling-observation.md:42. Counted here, read-only, from research/jumbling/restoration/11-scramble-census/runs/findability/ia-k40-s7101-steps.jsonl (ec0f0ac): at the 63 repair steps the repair cap's rank by decreasing N_c (`rank_c`) was 1 at 15 steps, 2 at 26, 3 at 13, 4 to 13 at 8 and 28 at 1, 201 in all, among 25 to 54 misaligned caps that could turn (median 41). Also research/jumbling/manual/02-descent.md:48, :169-171 (ec0f0ac) (the face match takes the largest drop first; the repair walk names caps by decreasing N_c); docs/progress/1.0/jumbling-handoff/design-table.md:50 (ec0f0ac) (DH-26). The key counts are design counts, not measurements.
- needs_owner: yes

#### D16

- on: A2, M1, M4, E3, D3 (item A4)
- kind: proposal
- claim: D3's option (c) is out, as A2, M1, M4 and I agree, but I keep (b): the loop tests "covers some cap and no cap covers it" before each cover step and each repair step (70 of the record's 97 steps), and "covers" ranges over all 80 caps that meet a cap, so under (a) settling one cap can take up to 80 two-cap requests plus one per misaligned neighbour, against one request under (b). Owner: (a) two caps the solver names; (b) one cap the solver names, answered with the IDs of the caps that cover it and of the misaligned caps it covers, with no "top" verdict and no map mark; this is a static fact about the named cap under M1's test, and I recommend it.
- evidence: new evidence: research/jumbling/theory/seams.py:161-179 (the covering cap d ranges over NEAR[c], every cap that meets c, not only misaligned ones), :194-197. I recounted 80 meeting caps for every cap in memory from the 600 cells of the 600-cell (no pole angle between 49.118° and 56.012°), as M11 found. The loop is at research/jumbling/manual/02-descent.md:14-20 (ec0f0ac). Counted from research/jumbling/restoration/11-scramble-census/runs/findability/ia-k40-s7101-steps.jsonl (ec0f0ac): 27 face-match, 7 cover and 63 repair steps; 21 to 28 misaligned caps that could turn and 1 to 4 qualifying caps at the cover steps; 2 to 27 misaligned neighbours per repair cap. The time of one exact request is E3's measurement.
- needs_owner: yes

#### D17

- on: M7, A12, E5, M8, D5 (items C1, C3 and C4)
- kind: proposal
- claim: I keep D5(a) and add M8(b)'s V search to it: a "yes" from C3 gives no word or length (M7), the Vs built from such certificates at the depth 40 stop have fewer than 10^166 and 10^141 twists, which the manual itself calls far beyond what a person can carry out, and the tail chapter recommends no step yet, so each aid would add a chamber-set selection to H-05 for a step that no chapter teaches. Owner: (a) unlock lists, C3 and the V search all out of 1.0 (D5(a), M7(a)), which I recommend; (b) C3's three-valued answer for a chamber set the solver names, shown as a status with no word, length or run control (M7(b), A12, E5), the rest out; (c) also a V search or an unlock list at an object the solver names, which extends the 17 October exception (M7(c), M8(b)).
- evidence: new evidence: research/jumbling/manual/03-tail.md:64-66 (ec0f0ac) names a peeling certificate or a short search as the source of a person's V, so the V search is the item to reopen when that chapter is written. Also research/jumbling/manual/03-tail.md:3-5, :56-59 (ec0f0ac); research/jumbling/theory/buffer-transport.md:10, :37, :42; docs/progress/1.0/jumbling-handoff/design-table.md:49 (ec0f0ac) (DH-25); docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:57-59 (ec0f0ac).
- needs_owner: yes

#### D18

- on: E8, M3, M8, M10, E9, A10, D7 (items C4, F1 and F2)
- kind: proposal
- claim: I support E8 and M3 and recommend E8(b), subject to EN1's contract review: Jumble off at N = 0 and ν = 0 through a label checkpoint, after which 600-cell-Full's tools and setup search run unchanged (which also settles M8, except D17's V search); the manual has a person make every correction before solving any orbit, so E8(a) and (c) add, beyond one saved crossing, only retained work before ν = 0, which the manual advises against. For F1 I recommend the sticker test (M10(1), E9(b), A10(b)), which needs only M10's label-level witness at the solved end and so holds even without E8(b), over pose equality with a mark on each piece that differs (A10(a), M10(2), E9(a)) and over both shown as two facts (E9(c)), because under pose equality the measured runner fixture would end "not solved" with every sticker at home, and no record shows that those rotations can be removed.
- evidence: new evidence: research/jumbling/manual/04-lattice.md:115-119 (ec0f0ac) ("for a person, first is simpler"). Also research/jumbling/manual/04-lattice.md:90-95 (ec0f0ac) (a correction made later would undo solved work); docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:20, :35-37 (Jumble off only at a witnessed checkpoint; Jumble on replays a retained word); research/jumbling/restoration/13-tutorial-runner/README.md:594-598 (ec0f0ac); docs/progress/1.0/jumbling-handoff/engineering.md:108 (ec0f0ac) (EO-20); docs/progress/1.0/replan-2026-10-22.md:72 (EN1).
- needs_owner: yes

#### D19

- on: M6, E2, E11, A9, D6, D10 (items C2, E2 and E4)
- kind: proposal
- claim: M6, E2 and D6 give one option set for C2, and I keep D6's choice. Owner: (a) ν in A9's 45 fixed marks beside the static table 4.7.3, with a check of ν after each correction; (b) also a check of a draft set of corrections the solver proposes, showing before any twist the ν it would leave (exact because ν adds), which I recommend; (c) the program lists the combination, which needs a new exception. For E2 I accept E11 and change D10, which backed E4 with no exception. Owner: (a) the 36 corrections ship as constructions the solver selects by number, as the one named exception to E4, checked once in CI and again by each whole-word preview, which I recommend because each acts the same from every lattice configuration and the manual already puts them in the program as stored macros; (b) the solver enters them from the manual's table as their own words.
- evidence: no new evidence. research/jumbling/manual/04-lattice.md:78-88, :97-105, :121-123 (ec0f0ac); docs/wiki/decisions/owner-decisions-2026-10-03.md:57-59; docs/wiki/decisions/owner-decisions-2026-10-02.md:59; docs/progress/1.0/jumbling-handoff/engineering.md:38-39 (ec0f0ac) (EH-11, EH-12).
- needs_owner: yes

#### D20

- on: M7, E10, D10 (items E1 and E3)
- kind: support
- claim: M7 is right, and I withdraw D10's "no length limit": the retained operation already composes by name (Prepare, Macro and Cleanup, with Cleanup the inverse of Prepare and a saved macro added to a phase), and DH-23's folded tree adds same_as parts, so reuse by name builds straight-line programs such as b19's V, which has 12,688 nodes and fewer than 10^141 letters. A stored word therefore keeps its flat letters as the record (E10, EH-22) and treats its named parts only as a view of them; every composition shows its exact expanded length before it is stored or previewed; and an expansion beyond what the store and replay can hold, a bound set from E4's measurement, is refused with that length shown.
- evidence: new evidence: docs/progress/1.0/solving-workflow.md:71, :81; docs/progress/1.0/command-table.json:451 (operation.cleanup.from_prepare), :1051 (macro.add), :1146 (macro.save). Also docs/progress/1.0/jumbling-handoff/design-table.md:47 (ec0f0ac) (DH-23); docs/progress/1.0/jumbling-handoff/engineering.md:54 (ec0f0ac) (EH-22); research/jumbling/restoration/12-low-height/README.md:597-599 (ec0f0ac).
- needs_owner: no

#### D21

- on: A8, M1, E1, D1 (items A1 and A2)
- kind: proposal
- claim: I change D1's list of every height present to A8's fixed form: n₂, n₄ and n₆, plus one count above 6 that also shows its largest height (the stated threshold M1 asks for). Random scrambles of up to 3,000 twists reach height 6 at most, while constructed words reach at least 3,888, so a list of every height present has no fixed size for the totals panel. On A8's owner question (DH-28) I support (a), both shown, with u and its straddler set shown at a cap the solver names, and no order, filter or map mark by u until E1's 600-cap timing exists.
- evidence: new evidence: counted here, read-only, from research/jumbling/restoration/11-scramble-census/runs/heights/h-7301.jsonl, h-7302.jsonl and h-7303.jsonl (ec0f0ac): only left heights 2, 4 and 6, at every checkpoint up to k = 3,000 on all three seeds (script: research/jumbling/restoration/11-scramble-census/scripts/heights_k.py:1 (ec0f0ac)). Also research/jumbling/restoration/10-x1-unbounded/README.md:8-9; docs/progress/1.0/jumbling-handoff/design-table.md:81 (ec0f0ac) (DO-20).
- needs_owner: yes

#### D22

- on: A4, A7, M9, M12, D8, D9 (items D1, D2 and G)
- kind: proposal
- claim: I support A4(b): each listed pair shows its change in N as a numeral in a fixed column, in the neutral order, never as a bar length, a colour ramp or a weight, because the program has already computed it to list the pair and hiding it costs a check per row (median 5 rows); the same rule holds for D8(b)'s history of the solver's own checks, kept in the order tried, which I still recommend. M9's three truth conditions belong on the term card of the step-check flag (renamed without "strict", as M12 asks), one focus away rather than as sentences in every check, because while solving the whole screen has about 20 words of free text, and a glossary term with its card uses none of them.
- evidence: new evidence: docs/wiki/decisions/owner-decisions-2026-10-02-design.md:84-85 (the budget is per whole screen; labels, numbers and glossary terms are not free text; every term has a term card). Also docs/progress/1.0/jumbling-handoff/design-table.md:59 (ec0f0ac) (DO-01); docs/wiki/decisions/owner-decisions-2026-10-02-design.md:41; research/jumbling/manual/02-descent.md:43-48 (ec0f0ac).
- needs_owner: yes

### Interaction artist

#### A13

- on: D4, M5, E1, A1 (item B)
- kind: objection
- claim: I object to M5's reading that option 3 passes the filter, and to D4's "name next cap" key. A passing cap's face-match drop is N_c minus its floor, the face match takes the largest drop first, and the census models a person's face match as a walk by decreasing N_c, so an order by these facts also ranks caps for the face match, where the 17 October scope keeps ranking out; and the program cannot hold the order back for the repair step without first deciding that no cap passes, which is a fact quantified over twists. I withdraw my A1(c); owner set: (a) option 1; (b) option 2, preferred: a threshold the solver sets on N_c or the floor (u only after E1's measurement), drawn as a status pattern on the fixed map with no default value, which gives a set and not an order; (c) option 2 plus D4's key, as an explicit owner extension of that scope to every step; option 4 fails rule 6 (D4, M5).
- evidence: new evidence: research/jumbling/restoration/11-scramble-census/README.md:260-263 (ec0f0ac) and research/jumbling/restoration/11-scramble-census/scripts/findability.py:45, :53 (ec0f0ac) (a face match brings N_c to the floor; the person's search takes caps by decreasing N_c); research/jumbling/manual/02-descent.md:48, :78-79 (ec0f0ac) (which cap first: the largest drop, which is the whole N_c when the floor is 0); docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:57-59 (ec0f0ac). Sketch: limiting an order to the repair step needs "no cap passes the face match", a minimum over each cap's twists, which M1's test excludes (docs/progress/1.0/design-table/topics/jumble-solver-aids.md:303). M5's reason against option 4 (the repair cap named first or second at 41 of 63 steps, :335) holds whoever picks the key. Also docs/progress/1.0/jumbling-observation.md:42, :128; docs/progress/1.0/jumbling-handoff/design-table.md:79 (ec0f0ac) (DO-18: the pattern must read beside the blocked mark).
- needs_owner: yes

#### A14

- on: D11, A3, M1, E6 (item G1)
- kind: proposal
- claim: The owner's neutral order (x in menu order, then d) is also the census rule's search order, so row 1 was exactly the pair the rule took at all 63 recorded repair steps, while it held the largest drop at only 26 (29.2 expected at a random row); a cursor or focus on row 1 when the list opens would show the machine's pick every time (M1's "first"), so I withdraw A3(b). Owner set: (a) preferred: the list opens with focus on the named cap and on no row, arrow keys move a row focus that draws only that row's twists on the geometry, and a number key copies a row into the draft (D11b with A3a); (b) the cursor starts on row 1 (D11a), which marks the rule's pick; (c) no row focus, and the solver types x and d (A3c).
- evidence: new evidence: counted with python -I, read-only, on temporary copies (made outside the repository and since deleted) of research/jumbling/restoration/11-scramble-census/runs/findability/ia-k40-s7101-density.jsonl.gz and ia-k40-s7101-density-rest.jsonl.gz (ec0f0ac). The records hold 63 steps and 679 listed pairs (delta < 0), ordered by x, then d. Row 1 is the row marked chosen at 63 of 63 steps. The largest drop is in row 1 at 26 steps, and at 11 of the 48 lists with more than one row (14.2 expected at a random row). The largest drop is tied at 9 steps, and the first row with the largest drop has a median position of 3 (at most 43). It holds by construction: research/jumbling/manual/02-descent.md:151-153 (ec0f0ac); research/jumbling/restoration/11-scramble-census/scripts/density.py:58, :61, :63, :68 (ec0f0ac); docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:55 (ec0f0ac). Row numbers and number keys stay stable only under E6's order digest (research/jumbling/sim/twists.py:482-483).
- needs_owner: yes

#### A15

- on: D11, E7, A11 (item G2)
- kind: proposal
- claim: I fold A11 into D11 and prefer D11(b): a picked row becomes a draft that the solver may step through in preview, each twist with its own sweep, and commit with one action, so a rise in N after x shows only in the preview, labelled "repair, k of m", and never as a committed regression. I also correct A11: x raises N at 517 of the 679 listed pairs but lowers it at 52 and leaves it unchanged at 110, so "x raises N by design" was too strong; per E7, m is 2 when z is the identity (86 pairs), and an identity twist gets no sweep, pause or count in the motion table; owner set: (a) one action and one commit per twist (D11a, close to A11b); (b) D11(b), preferred.
- evidence: new evidence: counted the same way from the same two density records (ec0f0ac), taking x's part of the change in N as N_c after x minus N_c (research/jumbling/restoration/11-scramble-census/scripts/density.py:39 (ec0f0ac)); research/jumbling/manual/02-descent.md:187 (ec0f0ac) (x = (91, 1) lowers N by 206) against :200 (x raises N_552 from 36 to 94). Also docs/progress/1.0/charter-2.0-draft.md:83 (S4: the witness sequence before commit, the commit in one action), :115 (A1); docs/wiki/decisions/owner-decisions-2026-10-02-design.md:32, :41; docs/progress/1.0/jumbling-observation.md:35 (rule 3).
- needs_owner: yes

#### A16

- on: M10, E9, D7, E8, A10 (items F1, F2)
- kind: proposal
- claim: I now prefer the sticker test, with Jumble off through E8(b)'s label-level checkpoint at ν = 0 (M10's option 1, E9b, D7a). M10 shows that the hidden rotation changes nothing the solver can see or do later, which meets my A10(b) condition once the witness is label-level, and I withdraw A10(a) because an orientation glyph for a rotation that fixes every sticker draws a difference with nothing visible behind it. I object to D7(c) and E9(c), because a second "not solved" fact after the solved moment contradicts S5 and splits exact completion; owner set: (a) the sticker test with the label checkpoint, preferred; (b) pose equality with every differing piece marked (M10's option 2, E9a), under which a complete solve by the manual's lattice method may still read as unsolved.
- evidence: new evidence: the lattice phase uses retained twists only (research/jumbling/manual/04-lattice.md:95, :127-128 (ec0f0ac)), and a retained generator rotates centre piece 0 in place with equal labels (research/jumbling/state-contract.md:146), so the method the manual teaches works on labels and never steers these poses. Also docs/progress/1.0/design-table/topics/jumble-solver-aids.md:375-376 (M10's sketch); docs/progress/1.0/charter-2.0-draft.md:84 (S5); docs/wiki/decisions/owner-decisions-2026-10-02-design.md:35-36; docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:20, :36 (option (a) changes contract section 5 through EN1's senior check).
- needs_owner: yes

#### A17

- on: D2, E2, M3, A9 (item A3)
- kind: support
- claim: I support D2's placement, which is my A9's 45 fixed marks arranged by orbit: a sign mark in each of the 35 orbits, a frame mark in eight and the C5 numeral in O17 and O28, nine of them fixed structure. I also support D2's and E2's undefined state off the lattice, drawn as its own pattern and never as 45 clear marks, because all clear is the signal the manual tells the solver to wait for before using retained twists only; and with M3, the C5 numerals 0 to 4 follow one pinned generator, the convention of the manual's correction table, while a nonzero value in one of the nine always-zero coordinates shows as a typed engine error, not as a mark.
- evidence: no new evidence. research/jumbling/manual/04-lattice.md:45-46, :64-66, :90-95, :101-104 (ec0f0ac); research/jumbling/restoration/03-retained-group-and-lattice-restorer/scripts/nucoords.py:64; docs/wiki/decisions/owner-decisions-2026-10-02-design.md:41, :68; docs/progress/1.0/charter-2.0-draft.md:93 (E6), :99 (M3).
- needs_owner: no

#### A18

- on: M6, E2, D6, E11, D10, E10 (items C2, E2)
- kind: proposal
- claim: For C2, one owner set: (a) the ν marks beside the static table 4.7.3, where the solver works out the combination and the program checks ν after each correction (E2a, M6a); (b) also a check of a set of corrections the solver drafts, drawn as the ν it would leave, in the same fixed marks under a preview pattern (E2b, M6b, D6), preferred; (c) the program shows the combination, which needs a new exception (E2c, M6c). For E2, I support shipping the 36 corrections as known constructions the solver selects by id (D6a, E11a, M6), in the single macro base of D10 and E10 with the role glyph of the retained known constructions, listed in fixed id order and never reordered, filtered or lit up by the current ν, because that would deliver (c) by another route.
- evidence: no new evidence. research/jumbling/manual/04-lattice.md:84-88, :97-105, :121-123 (ec0f0ac); docs/wiki/decisions/owner-decisions-2026-10-03.md:57-59 (a correction the solver selects is the solver's own macro; the program never picks one); docs/wiki/decisions/owner-decisions-2026-10-02-design.md:41; docs/progress/1.0/jumbling-handoff/engineering.md:38 (ec0f0ac) (EH-11: 5 to 5,629 twists each).
- needs_owner: yes

#### A19

- on: D3, M4, E3, A2 (item A4)
- kind: proposal
- claim: I change A2 and support D3(b): for one cap the solver names, show the caps it covers and the caps that cover it, under M4's exact side test (E3), drawn on caps that keep their places, with the direction as a glyph role. It answers only about the named cap and saves up to 27 two-cap requests, so it stays legible without searching across caps, while D3(c), a map mark on caps that cover some cap and are covered by none, stays out as the cover rule's own selection (M1, M4) in a step where the 17 October scope forbids next-move marks; owner set: (a) two named caps; (b) D3(b), preferred.
- evidence: no new evidence. docs/progress/1.0/design-table/topics/jumble-solver-aids.md:123-124 (D3: 2 to 27 misaligned neighbours), :303 (M1), :327 (M4); research/jumbling/manual/02-descent.md:106-108 (ec0f0ac); docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:57-59 (ec0f0ac); docs/wiki/decisions/owner-decisions-2026-10-02-design.md:41.
- needs_owner: yes

#### A20

- on: D1, M1, M2, E1, E12, A8 (items A1, A2)
- kind: objection
- claim: I object to D1's sketch that lists only the heights present, because position carries identity: on the record the count at height 4 rose from 0 to 111 and a descent cleared all 135 pieces at height 6, so such a list inserts or drops a slot exactly when a height step happens. Fixed slots n₂, n₄ and n₆ with zeros shown, plus one "above 6" slot (M1's stated threshold), keep each count in place, and no random scramble on record, up to 3000 twists, went above 6; I support D1's Δn_h and Δu beside ΔN, and M2's full straddler set, with all u_c pieces in the named cap's focus set and the rest dimmed (E12), all as numerals of equal weight with no colour by sign; DH-28 stays the owner's open item with A8's options, now that all four roles support showing both counts.
- evidence: new evidence: research/jumbling/restoration/11-scramble-census/README.md:180-185 (ec0f0ac) (no piece above height 6 after 1000 random twists or in three 3000-twist scrambles; the depth 1000 descent cleared its 135 pieces at height 6; pieces at height 4 rose from 0 to 111 and from 81 to 124); docs/progress/1.0/jumbling-handoff/design-table.md:96 (ec0f0ac) (the height ramp's rule above 6 is still open, so the totals' slots should match the ramp's steps, whose colours stay provisional). Also docs/wiki/decisions/owner-decisions-2026-10-02-design.md:41-42; docs/progress/1.0/design-options-2026-10-round2.md:49; docs/progress/1.0/jumbling-handoff/design-table.md:93 (ec0f0ac) (open item).
- needs_owner: yes

#### A21

- on: D5, M7, M8, D7, E8, A12 (items C1, C3, C4)
- kind: support
- claim: For C1 and C3, I support D5(a) with M7(a): neither unlock lists nor existence answers in 1.0, because no manual chapter teaches a step that uses them, and an existence answer covers SAFE-ALL words only, gives no word or length and stays unknown where no certificate applies, so its status would promise a step nobody can see; if the owner allows either, A12's rendering holds. For C4, I support M8(a) over a V search, in the form D7 and E8(b) give it: setup search on the lattice after the single crossing at ν = 0, so the retained strip keeps its setup role unchanged, while a V search off the lattice (M8b) would read as the program building most of a step.
- evidence: no new evidence. research/jumbling/theory/buffer-transport.md:10, :37, :42-43; research/jumbling/manual/README.md:44-50 (ec0f0ac); research/jumbling/manual/03-tail.md:20 (ec0f0ac); research/jumbling/manual/04-lattice.md:90-95 (ec0f0ac) (corrections before any orbit); docs/progress/1.0/jumbling-handoff/design-table.md:49 (ec0f0ac) (DH-25); docs/wiki/decisions/owner-decisions-2026-10-02.md:59.
- needs_owner: yes

#### A22

- on: D8, A4, M9, A7 (items D1, D2; repair-row content)
- kind: support
- claim: I support D8(b) and keep A4(b) under one rule: any list the solver reads, whether its own tries at the named cap in the order tried or the repair rows in the owner's order, shows ΔN as a numeral in a fixed column, never sorted and never as bar length, colour or weight, and is dropped at the next commit. M9's three truth conditions of the strict′ flag belong in its term card and the expanded summary, not in the collapsed one, so the flag stays a fact about the solver's word within the 20-word budget.
- evidence: no new evidence. docs/progress/1.0/design-table/topics/jumble-solver-aids.md:163 (D8), :229 (A4), :367 (M9); docs/wiki/decisions/owner-decisions-2026-10-02-design.md:41, :84-85; docs/progress/1.0/charter-2.0-draft.md:99 (M3); docs/progress/1.0/jumbling-observation.md:35 (rule 3); docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:55 (ec0f0ac).
- needs_owner: yes

### Mathematician

#### M13

- on: D2, A9, E2, M3
- kind: objection
- claim: Off the lattice, D2's "undefined" and E2's "not on the lattice" are the only truthful states. ν is defined on lattice configurations only and jumble twists still change it, so no zero, stale or partial value may be shown as ν while N > 0. A9's numerals 0 to 4 are truthful only when the C5 generator is fixed by the rule at nucoords.py:64: under r0² every C5 value is multiplied by 3, so a true C5_17 = 1 reads 3, and the manual's "make correction 27 v times" then leaves 3 (shown as 4), not 0. This narrows M3: the rule's inputs are model assets, so one contract sentence pins it without a new identity.
- evidence: new evidence: a counterexample mod 5. With r0′ = r0², the element r0^j reads 3j (because 2⁻¹ = 3), and three corrections 27 (each −1) take the true value from 1 to 3, shown as 3·3 ≡ 4. Sources: research/jumbling/restoration/03-retained-group-and-lattice-restorer/scripts/nucoords.py:64, :74-76; research/jumbling/sim/state.py:559 (labels are refused off the lattice); research/jumbling/manual/04-lattice.md:60-61, :101-104, :175, :274, :277 (ec0f0ac); assets/manifest.json:11, :20 (the execution trees and the seed atlas, the rule's inputs, are under the model identity); docs/progress/1.0/jumbling-handoff/engineering.md:37 (ec0f0ac) (EH-10's check against nucoords.py enforces the rule). The sign and frame bits need no pin (Lemma 8, research/Full_600cell_Puzzle_Theory.tex:1028-1036). The nine fixed coordinates are proved to be 0 (04-lattice.md:61-66 (ec0f0ac); the upper bound is proved). A9's fixed structure is therefore truthful, provided a nonzero value there raises an engine defect.
- needs_owner: no

#### M14

- on: D6, E2, M6, A9 (item C2)
- kind: proposal
- claim: One option set for C2: (a) ν in orbit coordinates beside the static tables 4.7.2 and 4.7.3, read again after each correction (E2(a), M6(a)); (b) also the ν that a draft set of corrections proposed by the solver would leave, which is exact because ν adds (D6's aid, E2(b), M6(b)); (c) a new exception under which the program shows the combination (D6(c), E2(c), M6(c)). Both (b) and (c) are functions of ν and the static tables, so what separates them is not information but the boundary's "outputting a solving macro": (b) passes as a check of the solver's draft, (c) needs the owner, and under (a) or (b) no view may light the corrections that the set bits call for (A9), because those corrections are the combination.
- evidence: no new evidence. research/jumbling/manual/04-lattice.md:79-81 (a basis, so exactly one combination), :88 (ν adds), :98-100 (the rows of the set bits, counted odd), :121-123 (ec0f0ac); research/jumbling/restoration/03-retained-group-and-lattice-restorer/scripts/nucoords.py:170-196 (the combination is a row reduction); docs/wiki/decisions/owner-decisions-2026-10-02.md:59; docs/progress/1.0/jumbling-handoff/engineering.md:39 (ec0f0ac) (EH-12 is open).
- needs_owner: yes

#### M15

- on: D6, E11, M6 (item E2)
- kind: proposal
- claim: One option set for E2: (a) the 36 corrections ship as constructions that the solver selects by id, as one named exception to E4 and an explicit extension of the 3 October ruling (D6(a), E11(a)); (b) the solver enters them as their own words (D6(b), E11(b)). I change M6. The ruling's rationale fits the corrections, because each one has a single proved effect on labels and on ν from every reachable lattice configuration, but its text does not: it names four 600-cell-Full families selected by parameters, and its "correction" means the buffer-A correction. D6's commutator power, written as a formula, is the solver's own macro only when the program expands it to the flat word and shows its exact length, as the ruling and M7 require.
- evidence: new evidence: docs/wiki/decisions/owner-decisions-2026-10-03.md:57 (placement star, orientation transfer, buffer-A correction, final-buffer commutator; "expands it into its exact word"), :59. Also research/jumbling/manual/04-lattice.md:84-88, :111-113 (ec0f0ac); docs/progress/1.0/jumbling-handoff/engineering.md:38, :41 (ec0f0ac) (EH-11, EH-14). Term, as a preference: call the 36 "ν-corrections", so that "correction" keeps its 3 October meaning.
- needs_owner: yes

#### M16

- on: D7, E8, E9, A10, M8 (items F1, F2, C4)
- kind: proposal
- claim: One combined option set for F1, F2 and C4: 1. Sticker end. Jumble off at N = 0 and ν = 0 through a label checkpoint certified by G = 𝒦, which keeps the exit pose state; setup search and the retained tools run in 600-cell-Full after the crossing (D7, E8(b), E9(b)). 2. Sticker end. The lattice phase stays in Jumble, where the retained tools and setup search act on π(X) at N = 0 for any ν and protection is read on labels; Jumble off at the sticker-solved end, with the empty word as a label-level witness (E8(a), M8(a)). 3. Pose end. Every piece whose labels are home and whose pose is not carries a mark; the lattice phase stays in Jumble with protection on poses; Jumble off at the pose-solved end, which section 5 already allows (A10(a), E9(a)).

  The pose end fits only a lattice phase in Jumble, because after a crossing the solve ends in 600-cell-Full, whose state is its labels. Unmarked pose equality is out, and "both tests" reduces to one of these options because S5 fires on one test (A10).
- evidence: new evidence: docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:24 (600-cell-Full's state is its labels), :36-37; research/jumbling/state-contract.md:74-77, :81, :146 (checkpoints and protection use poses); research/jumbling/manual/04-lattice.md:48-50, :178 (ec0f0ac) (G = 𝒦 is a certificate); research/Full_600cell_Puzzle_Theory.tex:1470-1478 (Lemma 15: guarded setup tables are properties of words on sticker frames, so they hold at N = 0 for any ν). Options 1 and 2 can be offered together (E8(c)). Each of them changes contract section 5 and goes through EN1's Astra plan check (docs/progress/1.0/jumbling-handoff/engineering.md:45 (ec0f0ac)).
- needs_owner: yes

#### M17

- on: E8, A10, M3, M10
- kind: support
- claim: Three facts fix M16's set:
  - Jumble off at ν ≠ 0 is out by proof, not only by the certificate, because retained twists keep ν = 0.
  - No witnessed crossing exists in practice before the pose-solved end. J1's checkpoint needs a retained word that reaches the exact poses; the inverse of that word is a solving word, which the program may not compute and the solver does not have. So the "witness policy" of M3 blocks every earlier crossing.
  - A10(b)'s condition is a contract matter. By M10's sketch, only checks that read poses (the pose-digest checkpoint and pose-based protection) can depend on a hidden turn.

  I correct M10: undoing the history always removes a hidden turn (EO-13), so option 3 is never a dead end. But Theorem R reaches only the sticker end, and a single retained generator already turns centre piece 0 in place, so no record gives a way to the pose end that is shorter than undoing.
- evidence: new evidence: research/jumbling/manual/04-lattice.md:47 (card 1 F1, proved), :178, :181, :185-186 (ec0f0ac) (Theorem R holds in the sense of stickers); research/jumbling/sim/state.py:489-510 (a journal witness only merges twists of the same grip), :531-536 (the witness is replayed and compared by pose digest); research/jumbling/state-contract.md:146 (generator 1 turns centre piece 0 in place); docs/wiki/decisions/owner-decisions-2026-10-02.md:59 (the program never solves automatically); docs/progress/1.0/jumbling-handoff/engineering.md:92 (EO-13, proved), :108 (EO-20, measured) (ec0f0ac). M10's sketch also answers ENG-0J's open question about a lift from labels, in the observable sense (docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:35). Any lift of the labels behaves the same at every later revision, so keeping the exit pose state serves only the digest and the journal.
- needs_owner: no

#### M18

- on: D3, A2, E3, M4 (item A4)
- kind: proposal
- claim: D3(b) passes the filter. The covers relation restricted to a cap that the solver names is static and equals the answers for the 80 caps that meet it (M11). Three conditions apply: it lists every meeting cap with its answer, it comes only from a completed job, and it states its domain (the covered cap is admissible and misaligned). D3(c) is out: the caps that cover some cap and that no cap covers are exactly the cover rule's candidates, so marking them marks a next move, which the 17 October scope forbids for the cover step. The owner question is therefore (a) two named caps or (b) one named cap in both directions. E3's on-plane case needs no centroid rule: by M4's sketch, a centroid on d's cut forces a straddler of d, and the exact classes already exclude straddlers.
- evidence: no new evidence. research/jumbling/manual/02-descent.md:100-104 ("a misaligned cap c"), :106-108 (the rule) (ec0f0ac); docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:57-59 (ec0f0ac); research/jumbling/state-contract.md:34-38, :43; research/jumbling/theory/restore.py:57 with M11's synthetic count (80 meeting caps per cap).
- needs_owner: yes

#### M19

- on: D4, A1, M5 (item B)
- kind: proposal
- claim: I change M5: option 4 is out by the filter, because a default order by decreasing N_c is the manual's own search order for the repair cap. The batch therefore carries:
  - (a) options 1 and 2: a fixed map with a threshold that the solver sets and no default (A1(b), D4(a));
  - (b) option 3 as well: a sort by one shown fact that the solver chooses, with ties to the smaller cap number and recomputed at each revision. It can be A1's separate list or D4's walk key, including D4(c)'s saved setting, because the solver chose the key.

  (b) depends on the owner reading "ranks caps" in rule 6 and in the 17 October scope as an order that the program chooses. D4's key must also name exactly the next cap in the order and never skip a cap because of a job result. A key that skips empty lists returns the first cap that has a lowering pair, which is a cap the program chose by searching twists.
- evidence: new evidence: the skip sketch, set against docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:51-52 (ec0f0ac) ("does not choose the cap"). Also docs/progress/1.0/jumbling-observation.md:42 (rule 6); docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:57-59 (no view ranks caps for the face match and the cover step), :66-67 (first or second at 41 of 63 steps) (ec0f0ac); research/jumbling/manual/02-descent.md:169-171 (ec0f0ac).
- needs_owner: yes

#### M20

- on: D5, A12, M7, E5 (items C1, C3)
- kind: proposal
- claim: C3 is not a fact under rule 6, because it quantifies over words (some SAFE-ALL word vacates T). It also lies outside the 17 October exception, which covers only (x, d) pairs, so D5(b) would be a new exception rather than an application of the repair list's conditions. The batch carries:
  - (a) no C3 in 1.0 (D5(a), M7(a));
  - (b) a three-valued status for a chamber set T that the solver names, reading "a SAFE-ALL word exists", "no SAFE-ALL word exists" or "unknown", with no word glyph (A12, E5);
  - (c) C3 only inside a V search that the owner allows (M8(b)).

  For C1, a target cap that the program picks is out. Unlock letters at a cap the solver names are a list of twists found by search, so they need their own extension of the exception (A12).
- evidence: no new evidence. research/jumbling/theory/buffer-transport.md:10, :17-19, :37 (Corollary A1); docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:48-50, :57-59 (ec0f0ac); docs/progress/1.0/jumbling-observation.md:42; docs/progress/1.0/jumbling-handoff/engineering.md:44, :59 (ec0f0ac) (EH-17, EH-27).
- needs_owner: yes

#### M21

- on: D11, E6, E7, A11 (items G2, G3)
- kind: support
- claim: G3 is settled without the owner by density.py's rule under E6's pin, as D11 and E6 say. y gives the lowest N_d after x, and z gives the lowest N_c after x and y; each tie goes to the smaller pinned index, and a pair is formed only when y strictly lowers N_d. So E6's "lowest N_d, then lowest N_c" is two rules, not one lexicographic key. E7's two-twist repairs are two-twist only under the pin, because the identity wins a tie at z only when it sits at index 0. A11's premise that x raises N by design is false: in the manual's buried-twist example, x lowers N by 206. The state after x should therefore show x's own change in N as a fact, positive or negative.
- evidence: new evidence: research/jumbling/restoration/11-scramble-census/scripts/density.py:29 (y), :30-31 (no pair unless y lowers N_d), :38 (z over every item), :63-65 (x skips the identity) (ec0f0ac); research/jumbling/manual/02-descent.md:187 (ec0f0ac) (x = (91, 1), change of N −206); docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md:48-50 (ec0f0ac). Sketch: in E6's reversed record the identity sits at index 59. Wherever another item ties with it at z, that item is chosen, and the repair has three twists with the same change in N.
- needs_owner: no

#### M22

- on: D9, A7, E12, M12 (item D2, rule 5)
- kind: objection
- claim: "Collateral" names three sets in this topic:
  - what a word moves outside the orbit being worked on (the glossary);
  - every piece that a preview moves, which rule 5 never hides;
  - D2's lattice pieces taken off, the meaning that D9, A7 and E12's masks use.

  A retained letter takes no lattice piece off, yet it moves its whole inside set, including off pieces that get new poses. A never-hidden mask fed from D2's field would therefore let a filter hide what rule 5 protects. Rule 5 should stay bound to the full moving set, and D2's field needs its own name, for example "lifted" (the name is a preference).
- evidence: new evidence: the counterexample follows from research/jumbling/state-contract.md:12 (a retained twist has g ∈ A4_c), :26 (on the lattice, g_A ∈ K⁺) and :51 (every inside piece is turned), so a lattice piece stays on the lattice. docs/progress/1.0/jumbling-handoff/engineering.md:57 (ec0f0ac) (EH-25: R22D-80's retained unlock letters move off pieces to new poses, with ΔN of +174, +192 and −54). Definitions: docs/progress/1.0/glossary/glossary.md:102; docs/progress/1.0/jumbling-observation.md:37-39; docs/progress/1.0/design-table/topics/jumble-solver-aids.md:41, :489.
- needs_owner: no

### Engineer's advisor

#### E13

- on: D4, A1, M5, D12
- kind: objection
- claim: Orders and filters by N_c or the floor cost the same under every B option (a 600-row sort of the rim summary, with ties broken by cap index as M5 requires). The walk that all the options serve is the expensive part, though: on the seed 7101 record, naming caps by decreasing N_c takes 201 list jobs for the 63 repair steps (138 of which must run to completion before they can answer "empty"), 128,502 (x, d) pairs in all and 18,113 at the worst step, so the product's time per pair decides whether D4's walk is usable and must be measured before DT-J2 fixes the walk; the key itself is safe if presses never queue, naming the next cap cancels the running job (typed cancelled, never shown as empty), and no job starts for a cap the solver has not named, because a prefetch along the solver's order is the cross-cap search D12 excludes.
- evidence: new evidence: counted read-only (python -I, standard library only) from research/jumbling/restoration/11-scramble-census/runs/findability/ia-k40-s7101-steps.jsonl (ec0f0ac), using the fields rank_c, openings_before, pairs_before, x_rank_in_menu, index_d and neighbours_of_c defined at research/jumbling/restoration/11-scramble-census/scripts/findability.py:118-130 (ec0f0ac).
  - Jobs per repair step: minimum 1, median 2, maximum 28 (step 97), 201 in all. All 138 caps named before a repair cap had misaligned neighbours, so none of those lists is trivially empty.
  - Pairs per walk: median 1,062, maximum 18,113, 128,502 in all. The research rule's own median is 340 pairs before its first working pair; the walk costs more because a list must be complete (docs/progress/1.0/jumbling-handoff/engineering.md:114 (ec0f0ac), EO-23).
  - Per pair, research/jumbling/restoration/11-scramble-census/scripts/density.py:21-44, :63-67 (ec0f0ac) applies x again for every d. Each apply or undo drops the domains and the meeting caps' classifications and scores (research/jumbling/theory/restore.py:124-128, :144-159), so every pair rebuilds them for d and then for c. A product job can apply each opening once and split the pairs by x.
  - Orders or filters by u stay out until E1's 600-classification experiment.
  - Combined owner set:
    - (a) a fixed map with a threshold the solver sets (A1's (b), M5's (a), D4's (a));
    - (b) also a sort or traversal by one shown fact, chosen each time (M5's (b), A1's (c), D4's (b));
    - (c) that choice kept as a saved setting, with D4's key (D4's (c)).
  - Experiment (an EN2 cost lead, then the candidate engine on the owner's machine): time the list job at all 201 (step, cap) requests of this walk, from the record's rebuilt states, per pair and per step. The research's 3 to 5 minutes per cap (docs/progress/1.0/jumbling-observation.md:176 (ec0f0ac)) is a cloud timing, not performance evidence.
- needs_owner: yes

#### E14

- on: D8, A6, D12, M1
- kind: support
- claim: I support D8's option (b) and A6. Kept checks hold at most 59 entries per named cap and revision, and keeping the cut-cell states at the committed revision during a drag avoids a per-frame float classification that rule 2 forbids and that no cost table has budgeted. As D12 and M1 propose, the line belongs in the command layer: the research survey that the rim summary would be ported from also computes N_c after all 60 items at every cap, which the summary does not need and rule 6 forbids showing, so the port drops that loop, each check row returns only the released twist's values, and a headless test asserts that no view-model field holds a value for a twist the solver did not release.
- evidence: new evidence:
  - research/jumbling/theory/restore.py:155 computes N_c itself; :156-157 also compute N_c after all 60 items; :161-167 do this for every cap. See also docs/progress/1.0/jumbling-observation.md:34, :42, :119, :172 and docs/progress/1.0/stage-2-experiment-protocol.md:160 (H-06 still open).
  - J1 has no map from a drag to a menu item: its numerical input map refuses a menu (research/jumbling/sim/README.md:60). A6's equal detents therefore need a new map: the nearest menu element under one fixed metric, ties to E6's pinned index, and no input from the state.
  - The median of 71 checks per face match is at research/jumbling/restoration/11-scramble-census/README.md:258-263 (ec0f0ac).
  - Experiment (EN2 cost lead): time one exact release check (N_c after, floor, ΔN) at the 63 repair-step states and at the end state of wj-I_a. When releases arrive faster than checks finish, answer only the latest release.
- needs_owner: yes

#### E15

- on: D3, M4, A2, E3
- kind: support
- claim: I withdraw E3's exact-centroid test in favour of M4's definition. M4's test needs only the contract's certified classification of c's face pieces against d's cut and has no on-plane case. It can differ from the research float test only on pieces that straddle d (so only where d is blocked) or by rounding. On cost I support D3's option (b): "covered by" for a named c takes one face set of c and one exact side classification of that set per meeting cap, and "covers" takes one face set per admissible neighbour (2 to 27 misaligned ones at the repair caps), so (b) is one job that does at most the work (a) spreads over many requests, while (c) needs the whole graph at every revision and is out under M4 and A2.
- evidence: new evidence:
  - research/jumbling/theory/seams.py:130-159: a face set needs the cap's inside set. :161-179: cover_graph computes one face(c), then one vectorised side test per d in NEAR[c].
  - research/jumbling/restoration/11-scramble-census/scripts/findability.py:68-76 (ec0f0ac): one pair needs the face set of the covered cap.
  - research/jumbling/state-contract.md:30-43.
  - neighbours_of_c runs from 2 to 27, counted from research/jumbling/restoration/11-scramble-census/runs/findability/ia-k40-s7101-steps.jsonl (ec0f0ac).
  - Experiment (EH-28's check, extended): compare M4's exact test with the float test at the census cover steps and at the c_covers_d and d_covers_c flags that file records for all 63 repair steps. Trace each disagreement to a blocked d or to rounding, and time one (b) query.
- needs_owner: yes

#### E16

- on: M2, A8, D1, E1
- kind: support
- claim: I support M2. Even in a full classification, J1 keeps only the first straddle certificate in an order set by its pose grouping and a greedy anchor heuristic. The product therefore computes the full straddler set and u of a named cap and validates each certificate, the cap map keeps the early-exit blocked flag, and the differential harness compares the flag and the set, never J1's chosen witness or its points. For A8 I support option (a) with the same cost split: n_h is a per-revision histogram over the pose table, while u and D1's Δu cost one or two full classifications of the named cap, made when the solver names the cap (not on hover) and cached per revision.
- evidence: new evidence:
  - research/jumbling/sim/state.py:166-171: off pieces are grouped by pose id.
  - research/jumbling/sim/state.py:173-186: anchors are picked greedily, by the most common signature bit.
  - research/jumbling/sim/state.py:267-273: a full classification keeps going but keeps only the first certificate and counts no straddlers.
  - research/jumbling/sim/state.py:368 and :385-389: the pose table is put in canonical order after each twist.
  - research/jumbling/sim/state.py:409: the survey stops at the first straddler.
  - docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:11: the harness compares grip status after every step. If that status included the witness, a product engine would have to copy J1's scan order.
  - research/jumbling/state-contract.md:43: any two exact points on opposite sides certify.
  - docs/progress/1.0/jumbling-handoff/renderer.md:60 (ec0f0ac), RO-06: 181 to 827 straddlers per blocked grip, readable only with dimming and a focus piece with a pager, a W-J item 7 cost.
  - docs/progress/1.0/jumbling-observation.md:44.
  - Experiment: E1's comparison of 600 early-exit classifications against 600 full certified straddler sets, on a rebuilt height-4 state and on a 3000-twist scramble.
- needs_owner: yes

#### E17

- on: D10, M7, M11, E10
- kind: proposal
- claim: I support D10's one macro base and one key set, provided each word is typed by its alphabet:
  - a generator word runs on the label path, and on the pose path only by exact matrix lookup checked by the differential harness (π of the pose replay equals the label replay);
  - a word with a jumble letter is refused with a typed error while Jumble is off;
  - whether retained macros may run while Jumble is on stays E19's owner question, because ENG-0J refuses them today.

  I change E10's "no SLP part" to M7's second branch, because the retained workflow already composes operations from stored parts by reference, and saving w·w k times stores k + 1 nodes for 2^k·|w| letters. References may be stored, the exact expanded length is shown before any check or run, and replay goes letter by letter until a composed path passes EO-22's agreement test.
- evidence: new evidence:
  - docs/progress/1.0/solving-workflow.md:24 (a star is setup⁻¹ · relocation⁻¹ · seed · relocation · setup), :71 (phases are composed), :83 (a work sheet keeps its macro versions).
  - research/jumbling/sim/README.md:61: J1 journal records carry the exact matrix, which needs no order pin. The compact (grip, index) form of 2 bytes per letter (research/jumbling/restoration/13-tutorial-runner/README.md:464-466 (ec0f0ac)) needs the pin of E6 and M11.
  - docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:20, :37.
  - research/jumbling/state-contract.md:146: protection is pose-based, so the same macro's check reports pose effects on one path and label effects on the other.
  - docs/progress/1.0/jumbling-handoff/engineering.md:110-112 (ec0f0ac) (EO-21, EO-22).
  - Sketch: m₀ = w and m_{i+1} = m_i·m_i give k + 1 stored nodes and 2^k·|w| letters. The expanded length is a big-integer sum over the nodes.
  - Experiment: E4's replay timing gives the duration shown before a run.
- needs_owner: no

#### E18

- on: M10, A10, D7, E9
- kind: support
- claim: I change E9 from a question to support for the sticker test (D7's (a), M10's option 1, A10's (b)): both tests are cheap, and M10's sketch shows that no later region, sticker, admissibility or N depends on the hidden rotation. A10's proviso fails under ENG-0J as written, because J1 replays a witness word from solved, compares pose digests and searches for no witness, so a sticker-solved state with another digest (as the runner's fixture ends) cannot leave Jumble; the sticker test therefore needs M10's label-level witness at the solved end, with the exit poses stored, while pose equality adds the risk of a solve that never ends, since no record shows that such rotations can be removed.
- evidence: new evidence:
  - research/jumbling/sim/state.py:512-538: the witness is replayed from solved and the digests are compared.
  - research/jumbling/sim/README.md:66, :140: comparison is pose by pose, and nothing searches for a witness.
  - research/jumbling/restoration/13-tutorial-runner/README.md:540-542, :594-598 (ec0f0ac): identity labels with another digest, measured.
  - docs/wiki/decisions/owner-decisions-2026-10-15-jumbling-restored.md:21, :37; research/jumbling/state-contract.md:146.
  - Combined owner set for F1:
    - (a) the sticker test, with M10's label-level witness at the solved end;
    - (b) pose equality, with a mark on every differing piece (a 177,120-bit mask per revision);
    - (c) both, shown as separate facts.
  - Either change to contract section 5 goes through EN1's Astra plan check (docs/progress/1.0/replan-2026-10-22.md:72).
  - Check: extend ENG-0J acceptance item 7 with Jumble off at a state like the end of the runner's fixture.
- needs_owner: yes

#### E19

- on: E8, D7, M8, M10
- kind: proposal
- claim: I keep E8 and support D7, because the manual's rule for a person already puts the corrections first and then uses only retained twists. A crossing at N = 0 and ν = 0, through a label checkpoint certified by ν = 0 with the exit poses stored, then gives the lattice phase the unchanged retained toolset and setup search (C4) for one amendment to contract section 5, and it covers M10's solved end as a special case. Combined owner set:
  - (a) retained tools and setup search inside Jumble, acting on π(X) at N = 0 (M8's (a), E8's (a)); this needs a pose-path integration and differential tests for every retained tool;
  - (b) corrections in Jumble, then the ν = 0 crossing; this is the cheapest;
  - (c) both.
- evidence: new evidence:
  - research/jumbling/manual/04-lattice.md:91-95, :115-119 (ec0f0ac): corrections come before any orbit, then retained twists only; "for a person, first is simpler"; corrections come last only for the simulator's speed.
  - research/jumbling/restoration/13-tutorial-runner/README.md:590-593 (ec0f0ac): both orders are implemented.
  - docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md:16-20, :35-37, :75.
  - docs/progress/1.0/solving-workflow.md:116.
  - docs/progress/1.0/replan-2026-10-22.md:72: EN1's checkpoint rule through G = 𝒦.
  - docs/progress/1.0/jumbling-handoff/engineering.md:45 (ec0f0ac) (EH-18).
  - Under (b), Jumble on after a crossing replays, on the stored exit poses, the retained twists made since the crossing. This is ENG-0J's replay rule, started from a stored checkpoint instead of from solved.
  - Check: extend ENG-0J acceptance item 7 with the crossing, the exit at the solved end and the resume, on fresh synthetic sessions.
- needs_owner: yes

#### E20

- on: D6, M6, E2, E11, A9, M3
- kind: proposal
- claim: Cost does not separate the C2 options. The check that D6 and M6's (b) propose is exact without any replay, because ν after a set of corrections is ν plus their fixed rows, and even (c) is a small row reduction, so I support M6's (b) as the cheapest admissible aid and leave the line to the boundary. For E2 I support shipping the 36 corrections as constructions selected by id. M6's criterion (the same effect from every lattice configuration, proved) is the stated reason for the one exception to E4, EH-11's check runs in CI, and the whole-word preview runs at each use. A9's 45 marks then need M3's C5 convention written into the interface; nucoords.py takes it from manifest-hashed assets by a lexicographic minimum, so E6's order pin does not affect it.
- evidence: new evidence:
  - research/jumbling/restoration/03-retained-group-and-lattice-restorer/scripts/nucoords.py:29-31, :38-49, :63-69: r0 is the lexicographically smallest order-5 permutation built from assets/execution_trees.json.gz and assets/seed_atlas.json, both hashed at assets/manifest.json:11, :20. No menu record is involved.
  - nucoords.py:170-196: the combination is a row reduction over F2 and F5.
  - research/jumbling/manual/04-lattice.md:84-88, :97-105, :111-113, :121-123 (ec0f0ac).
  - docs/wiki/decisions/owner-decisions-2026-10-03.md:57-59.
  - docs/progress/1.0/jumbling-handoff/engineering.md:37-38 (ec0f0ac): the checks of EH-10 and EH-11.
  - Combined owner set for C2:
    - (a) ν in orbit coordinates, plus table 4.7.3;
    - (b) also a check of a set of corrections the solver proposes;
    - (c) the program shows the combination.
  - Combined owner set for E2:
    - (a) shipped constructions, selected by id;
    - (b) the solver enters them as their own words.
- needs_owner: yes

#### E21

- on: D5, A12, M7, M8
- kind: support
- claim: I support D5's (a) and M7's (a) for 1.0, with M8's V search left out alongside them. An unlock list, even at a named cap, ranges over the admissible letters of every grip: one recorded stall had 1,947 candidate first letters, each needing a full classification of the cap for u, and U2's two-letter family multiplies that. C3 has only research code and needs a three-valued result that must never read as a step, and none of the three has a product timing. Each can be added after 1.0 as a new query row, and a target cap that the program picks (C1 as stated) is out in every option.
- evidence: new evidence:
  - research/jumbling/restoration/12-low-height/README.md:1152-1158, :1289-1291, :1311-1314 (ec0f0ac): the unlock families U and U2 range over (grip, item) letters; "all 1,947 first letters tried" at one stall.
  - research/jumbling/theory/buffer-transport.md:17-19, :37, :42-43.
  - docs/progress/1.0/jumbling-handoff/engineering.md:114, :116 (ec0f0ac) (EO-23, EO-24).
  - research/jumbling/sim/state.py:267-273: u needs E16's full classification.
  - Combined owner set:
    - (a) none of these in 1.0;
    - (b) unlock lists and C3 at a cap or target the solver names, under the repair list's conditions;
    - (c) (b) plus a V search for a T the solver names.
- needs_owner: yes

#### E22

- on: D11, A3, A4, A11, E7
- kind: proposal
- claim: For G2, one commit of the whole repair (D11's (b), A11's (a)) is the cheaper option. It uses the existing commit from a fresh whole-word preview and makes one journal entry, which one undo takes back. A commit per twist (D11's (a), A11's (b)) needs predicted digests after x and after y in the list result, plus a journal group marker so that A11's whole-repair undo works. Either way, a pair whose z is the identity (86 of the 679 listed pairs) puts two twists in the draft and reads "1 of 2". G1's options cost the same, and A4's option (b) costs nothing, because the list job already computes each row's change in N.
- evidence: new evidence:
  - research/jumbling/restoration/11-scramble-census/scripts/density.py:39 (ec0f0ac): the change in N is computed for every listed pair.
  - docs/progress/1.0/command-table.json:511-528: operation.execute needs a fresh preview, and undo goes through the journal.
  - docs/progress/1.0/jumbling-handoff/engineering.md:92 (ec0f0ac) (EO-13).
  - Recounted read-only from research/jumbling/restoration/11-scramble-census/runs/findability/ia-k40-s7101-density.jsonl.gz and ia-k40-s7101-density-rest.jsonl.gz (ec0f0ac): 679 pairs lower N; 86 of them have z = item 0, the identity; at 6 of the 63 steps every pair does.
  - D11's number keys pick rows in the neutral order, which is fixed only with E6's pin.
- needs_owner: yes

## Outcome

Settled by the main session on 22 October 2026 from the 88 cards above, in the order of the [round protocol](../README.md): the filter, demonstrated failures, the 8B order among admissible options, and experiments for unmeasured objections. Nothing here is an owner decision until the owner answers the batch at the end.

Round 2 produced new evidence, so the protocol would allow a third round. The owner asked for a two-round trial, so the topic is settled here. Some round 2 claims have not yet been answered by the other roles (M13, M17, M21, M22, E14, E16 and E17). Each rests on a counterexample, a count or a reading of the code, and the outcome adopts them; a third round is where they would be contested.

### Out by the filter

- A default sort of the cap map (B, option 4): it runs the manual's own naming order for the solver, so it ranks caps (rule 6 and the 17 October scope; D4, A1, M5, M19, A13, D15).
- A target cap or unlock letters that the program picks (C1 as stated): the program would choose a target and mark a next move (D5, A12, M20, E21).
- A cap-map mark on the caps that cover some cap and that no cap covers (D3, option (c)): it marks the cover rule's next cap, which the 17 October scope forbids (A2, M4, M18, A19, D16, E15).
- The combination of corrections that ν calls for, shown by the program (C2, option (c)), and any view that lights, filters or reorders the corrections by the current ν: both output a solving macro under the 2 and 3 October rulings (D6, M6, A9, M14, A18). Only a new owner exception admits the first (O6).
- An existence answer worded as "a step exists" or "no step exists" (C3): Corollary A1 covers SAFE-ALL words only and gives no word or length (M7, M20, A21). An empty or cut-off job never reads as "no step exists" (D9, A5).
- Jumble off at ν ≠ 0: no retained word reaches such labels, so 600-cell-Full would hold a state with no legal witness (E8, M3, M17).
- Pose equality as the end test with no marks: it can report "not solved" when nothing on the screen differs (M10, M16). Two end tests shown as two facts reduce to one option, because the solved moment (S5) fires on one test (A16, M16).
- A "next cap" key that skips caps with an empty list: it returns a cap that the program chose by searching twists (M19). A job started for a cap the solver has not named, such as a prefetch along the solver's order: it is a search across caps (E13, D12).
- Any response to alignment during a drag (a snap, detent or sound that favours an aligning rotation), live float updates of the cut-cell states, and motion or sound that differs by a check's outcome: each is a hot-cold signal, and float checks break rule 2 (A6, E14).

### Fix first

These are demonstrated failures that affect the work or correctness. They need no owner answer.

1. **Pin the I_a order.** J1's menu identity hashes the sorted matrices, so it ignores item order. The wj-I_a record reversed gives the same fddc8532…, and 0 of 60 indices keep their rotation (E6; the main session re-checked this before round 2). The neutral order, the tie rule, row numbers and stored (grip, index) words all depend on the order (M11, D13, A14, M21, E17, E22). Fix: an order digest beside the identity, cited by every stored word and list result. EN1's Astra plan check rules whether this touches the model identity; if it does, it is an owner item.
2. **Type every job result.** The research tools already failed this way (P3-falsepass-01, a confirmed major finding; E5). Each result is completed, cancelled, cut off or an error. It carries its input digest, its parameters (cap, order-pinned menu identity, tie rule, contract revision) and its coverage, and the view model reads it only after the digest matches the shown revision. An empty list comes only from a completed job and names its cap and revision (D9, A5, M7).
3. **"Blocked by" is the straddler set.** One recorded state has 44 straddlers at cap 150, so "the straddling piece" defines nothing (M2). J1 keeps only the first certificate, in an order set by its pose grouping and a greedy anchor rule (E16). The product computes and validates the full set at a named cap. The harness compares the blocked flag and the set, never J1's chosen witness.
4. **Fix the requirements draft's terms.** In table 3.2 of `jumbling-observation.md`, "Carrier of c" quantifies over twists ("one twist of c puts it back on the lattice"), so as a filter it marks the caps that have a helpful twist, and "Fragment" has no stated threshold (M1). Keep only their static parts, with stated thresholds, or drop them. Rule 5 and item D2 use "collateral" for different sets (M22, M12): rule 5 stays bound to every piece that a preview moves, and D2's field gets its own name ("lifted" is the mathematician's preference).
5. **Pin the C5 convention.** Each C5 value depends on the generator that nucoords.py picks. Under r0², a true C5_17 = 1 reads 3, and the manual's "make correction 27 v times" then leaves 3 (shown as 4), not 0 (M13). One contract sentence pins the rule; its inputs are model assets, so no new identity is needed (M3, M13, E20). A nonzero value in one of the nine always-zero coordinates is a typed engine error (A17).
6. **Port the rim summary without the all-items loop.** The research survey also computes N_c after all 60 items at every cap, which the summary does not need and rule 6 forbids showing (E14). The port drops that loop, each check row returns only the released twist's values, and a headless test asserts that no view-model field holds a value for a twist the solver did not release.

### Settled without the owner

Each item is allowed as stated, or under the condition given.

- **A1, height counts:** fixed slots n₂, n₄ and n₆ with zeros shown, and one "above 6" slot that also shows its largest height, as plain numerals with no gauge, bar or "lower is better" colour (A8, A20, D21, M1). The engine keeps a sparse histogram per revision, and the slots read from it (E1, E16). Whether A1 and A2 are shown at all is O2.
- **A2, u:** the full straddler set and u of a cap the solver names, computed when the cap is named (not on hover) and cached per revision (M2, E16). No order, filter or map mark by u before X2 (E1, D21, A13).
- **Step check additions:** Δn_h, and Δu at a named cap, beside ΔN (D1, A20). x's own change in N is shown as a fact, positive or negative, because x lowers N at 52 of the 679 listed pairs and leaves it unchanged at 110 (A15, M21).
- **A3, ν:** only at N = 0, as 45 fixed marks arranged by orbit: 35 sign marks, 8 frame marks and the C5 numerals of O17 and O28, with the nine always-zero coordinates drawn as fixed structure (A9, A17, D2). Off the lattice, ν has its own "undefined" pattern and never shows zeros, stale values or partial values (M13, E2).
- **A4, covers:** M4's exact definition, which E15 adopts in place of E3's centroid test. An answer comes only from a completed job, states its domain and lists every meeting cap with its answer (M18). The form of the query is O3.
- **Cap map:** "blocked" and "at its floor" have distinct status patterns, and no cap is marked for passing the face match or for covering another cap (A2).
- **D1:** the exact check runs on the release of every previewed twist and shows N_c after against the floor, with ΔN (D8). When releases come faster than checks finish, only the latest release is answered (E14).
- **D2:** M9's fields as exact end-state numerals. The flag gets a name without "strict" (M12), and its three truth conditions go on its term card and in the expanded summary, not in the collapsed one (D22, A22). The evidence state, the facts and the permission stay apart, with no "good step" mark, score or good-or-bad colour (A7).
- **D3:** one whole-word replay at the current revision. The commit takes the end state from that fresh preview when the revision and the word digest are unchanged (E4).
- **E1:** one macro base and one key set. Each word is typed by its alphabet, and a word with a jumble letter is refused with a typed error while Jumble is off (D10, E10, E17). Whether retained macros run while Jumble is on follows O8.
- **E3** holds under M7's condition: a word built from named parts shows its exact expanded length before any check, preview or run, and an expansion beyond a bound set by X3 is refused with its length shown (D20, E17). Whether the store keeps flat letters (D20) or references (E17) is EN3's choice; both keep E3 true.
- **E4** holds, with a packaging check that keeps research word files out of the build (E10). The 36 corrections are O7.
- **G3:** density.py's rule under the pin, as two rules. y gives the lowest N_d after x, and z gives the lowest N_c after x and y; each tie goes to the smaller pinned index, and a pair exists only when y strictly lowers N_d (M21, D11, E6).
- **Identity twists:** the program never asks for one. A pair whose z is the identity is a two-twist repair (86 of the 679 listed pairs; E7, D14, A15, E22).
- **Command rows:** each admitted aid is one row whose arguments name the solver's object and whose answer is about that object only. The per-revision facts and the repair list at its named cap are the only exceptions (D12, E10).
- **Refusals:** a refused Jumble off names the fact and the kind of action, never which correction or twist (A5). At ν = 0 it does not say that the state is unreachable (M3).
- **Layers:** every aid sits in the portable layers and is tested headless. The renderer receives counts, lists and per-revision masks of 177,120 bits (E12).

### Experiments

X1 to X5 are headless cost leads for EN2. A cloud timing is not performance evidence; product timings come from the candidate engine on the owner's machine.

- **X1** (E13, E5, D4): the repair-list job at all 201 (step, cap) requests of the seed 7101 walk, per pair and per step. It decides whether a walk key (O1, option (c)) is usable, before DT-J2 fixes the walk.
- **X2** (E1, E16): 600 early-exit classifications against 600 full certified straddler sets, on a rebuilt height-4 state and on a 3000-twist scramble. It decides whether u may ever order, filter or mark caps.
- **X3** (E4, E17, D20): previews of the recorded words of 624 and 1,158 twists, of correction 35 (5,629 twists) and of b20 (40,500 twists), from their start states. It sets E3's bound and the duration shown before a run.
- **X4** (E3, E15): M4's exact covers against the research float test at the census cover steps and at the recorded flags of the 63 repair steps, with each disagreement traced to a blocked d or to rounding, plus the time of one query.
- **X5** (E14): one exact release check at the 63 repair-step states and at the end state of wj-I_a.
- **X6** (E12): the highlight-and-dim overlay (straddlers, covered pieces, lifted pieces and a repair pair's moving sets) joins R1's W-J item 7 series on the owner's machine. No cost is claimed before then.

### Owner batch

Each question offers two or three options. "Recommended" marks the option the roles converge on, or the 8B order's choice where they do not.

- **O1. Cap order and filter (B).** (a) The fixed map only. (b) A threshold that the solver sets on N_c or the floor, drawn as a set on the fixed map, with no default (A1, A13, M19). (c) As (b), plus a sort by one shown fact that the solver chooses at each step (ties to the cap index), and a key that names exactly the next cap in that order and never skips one (D15, M19). Option (c) needs the owner to read "ranks caps" in the 17 October scope as an order that the program chooses; A13 holds that any order by these facts ranks caps for the face match. Recommended: (b) under the current text; (c) if the owner admits that reading and X1 shows that the walk is usable, since the walk then costs one key per naming (D15).
- **O2. Height counts and u (DH-28).** (a) Both, as settled above (all four roles). (b) Height counts only. (c) Neither. Recommended: (a).
- **O3. Covers query (A4).** (a) Two caps that the solver names. (b) One cap that the solver names, answered with the caps that cover it and the misaligned caps that it covers, with no "top" verdict and no map mark (D16, A19, M18, E15). Recommended: (b).
- **O4. Kept checks (D1).** (a) Only the latest check. (b) Also the solver's own checks at the named cap for this revision, in the order tried, with ΔN as a numeral in a fixed column, never sorted and dropped at the next commit (D8, A22, E14). Recommended: (b).
- **O5. Unlock lists, existence tests and V search (C1, C3, and C4 off the lattice).** (a) None in 1.0 (all four roles). (b) C3's three-valued status for a chamber set that the solver names, with no word, length or run control, as a new exception (M20, A12). (c) Also unlock lists at a named cap or a V search, extending the 17 October exception (M7, M8, D17). Recommended: (a). The V search is the item to reopen when the tail chapter is written (D17).
- **O6. Which corrections ν calls for (C2).** (a) The ν marks beside the static table 4.7.3; the solver works out the combination, and the program checks ν after each correction. (b) Also a check of a set of corrections that the solver drafts, showing the ν it would leave before any twist (D19, A18, E20, M14). (c) The program shows the combination, as a new exception. Recommended: (b).
- **O7. The 36 corrections (E2).** (a) Ship them as constructions that the solver selects by id, as the one named exception to E4 and an explicit extension of the 3 October ruling. They are checked in CI and by each whole-word preview, and listed in a fixed id order that ν never reorders, filters or lights. (b) The solver enters them from the manual's table as their own words (5 to 5,629 twists each). M15 prefers the term "ν-corrections", so that "correction" keeps its 3 October meaning. Recommended: (a) (D19, A18, E20).
- **O8. End of a solve, Jumble off and the lattice phase (F1, F2, C4).** (1) Sticker end. After the corrections, Jumble off at N = 0 and ν = 0 through a label checkpoint certified by G = 𝒦, with the exit poses stored; 600-cell-Full's tools and setup search then run unchanged. (2) Sticker end. The lattice phase stays in Jumble, with the retained tools and setup search acting on π(X) at N = 0, and Jumble off at the solved end. (3) Pose end. The lattice phase stays in Jumble, and every piece whose labels are home but whose pose differs is marked. Options 1 and 2 can be combined, at the cost of a pose-path integration for every retained tool (E19). Each option changes contract section 5 and goes through EN1's Astra plan check. Recommended: (1), by the designer, the artist and the engineer (D18, A16, E18, E19); the mathematician sets out the three options without a preference (M16, M17).
- **O9. Row focus in the repair list (G1).** (a) The list opens with focus on the named cap and on no row; arrow keys move a row focus that draws only that row's twists, and a number key copies a row into the draft (D13, A14). (b) The cursor starts on row 1. (c) No row focus; the solver types x and d. Note: the neutral order is also the census rule's search order, so row 1 was the rule's own pair at 63 of the 63 recorded steps, while it held the largest drop at only 26 (A14, D13). Recommended: (a).
- **O10. Making a repair (G2).** (a) One action and one commit per twist, inside one open repair. (b) One commit of the whole repair, after a preview that the solver can step through twist by twist ("repair, k of m") (D14, A15, E22). Recommended: (b).
- **O11. Repair-row content (DH-01).** (a) x, d, y and z only, as the 17 October text names them. (b) Also ΔN as a numeral in a fixed column, in the neutral order, never as a bar, colour or weight; the job already computes it (A4, D22, A22, E22). Recommended: (b).

### After the owner's answers

- The answers are recorded here and in `docs/wiki/decisions/`.
- EN1 (ENG-0J revision 2, with its Astra plan check) takes fix-first items 1, 2, 3 and 5, the covers definition and O8's contract change. Its acceptance item 7 gains the crossing, the exit at the solved end and the resume, on fresh synthetic sessions (E18, E19).
- EN2 runs X1 to X5 and adds the harness rule of item 3 and the headless test of item 6.
- EN3 turns everything allowed here into command rows and view-model fields.
- The requirements draft takes item 4, and the glossary terms of M12 go to DH-09.
- DT-J2 to DT-J4 take the layout, look and motion of what is settled here.
- No handoff row (H-01 to H-10) changes; X6 joins R1.
