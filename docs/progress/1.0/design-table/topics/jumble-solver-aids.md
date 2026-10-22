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
