# ADR-D02 plan: a lean A1 to A3 for the Jumble and research surfaces

Status: plan of 23 November 2026, written by the ADR line at the owner's request. The owner asked for a lean A1 to A3
plan, handed to a new session, with the work held for now. **Nothing in this plan starts until the owner says so in
the session that receives it.** The owner's hold on ADR-D02 of 23 November 2026 stands until then. The plan records the
ADR line's selections, and the owner may overrule any of them. It is no ADR, accepts nothing and freezes nothing. A4,
the ADR text, the shell build and the acceptance runs come after A3 and are only outlined here (section 8).

A1 to A3 status, 24 November 2026: done on the owner's go of 23 November 2026, with A3 allowed up to three rounds
instead of section 5's one (owner, 23 Nov 2026: up to 3 rounds; three ran); the 152 open rows sit in 28 question groups,
each with two or three options, the recommended one first, one question left split for A4; supplemented the same day
after LP4 and LP5 landed (owner: "fill it in once they merge"), which aligns A3's words with the landed glossary and
verdict types and adds two open rows; the outputs stay private in the main checkout's `work/adr-d02/`.

A4 status, 25 November 2026: the pick page went to the owner as one card on the owner's go of the same day ("resume
work; tell the ADR line that D02's A4 may be prepared", then "once A4 is prepared, start right away"). It carries the
data its three waiting picks needed: the colouring number of the domain contact graph (4, exact, at W-J record 300 and
both deep exports; 1 at the I_a@40 stop), the seam-face mesh size (0.59 to 0.66 million triangles at the deep exports,
drawn whole) and offline drawings of the deep exports in both Global layouts, stand-ins for L1 and not Look Lab output.
Following the owner's direction that open research is ADR-D02's core ("open research is D02's core"), it recommends the
freer option inside the frozen frame at items 1, 3 and 12. Item 13, who builds the ADR-D02 and ADR-D03 shells, was
settled by the owner the same day: Fable writes the build framework, Opus builds, on the subscription. The page, data
and scripts stay private in the main checkout's `work/adr-d02/a4/`.

A4 answered, 26 November 2026: every recommended option except item 9 (Q3.b, the owner's reading of the frozen J-7
and N-1) and item 12c (the base targets: at most 3,000 hours of real time for an expert full solve without jumbling;
substantive progress on restoration theory for the jumbled puzzle); recorded in
[the decision page](../../wiki/decisions/owner-decisions-2026-11-26-adr-d02-a4.md). The proposed ADR text, read once
by a fresh Claude subagent, is
[ADR-D02-jumble-research.md](../../architecture/1.0/design/adr/ADR-D02-jumble-research.md) (26 November 2026); Fable's
build framework and the shell build follow.

Build framework, 26 November 2026: on the owner's go after reading the ADR text, Fable wrote
[the build framework](adr-d02-build-framework.md) as a subscription subagent: 23 steps (B1 to B23) in Look Lab's chrome
library and the product shell, five Fable checkpoints, the reduced Track J run at the end, about 106 Opus agent hours.
The integrator checked it and fixed the ADR text for its findings F5 to F16. Opus builds it one step per commit.

Inputs:
- The owner decisions on ADR-D02:
  - [design and art path](../../wiki/decisions/owner-decisions-2026-11-22-design-art-path.md), items 3 and 4;
  - [evening decisions](../../wiki/decisions/owner-decisions-2026-11-22-adr-line-evening.md), items 2 and 3;
  - [A7 freeze](../../wiki/decisions/owner-decisions-2026-11-23-a7-freeze.md).
- ADR-D01 and its revisions 4 to 6 in `docs/architecture/1.0/design/adr/`. Revision 6 (`ADR-D01-rev6-a6-fixes.md`)
  holds the answers to the A6 review's item J.
- DT-J1's outcome: [jumble-solver-aids.md](design-table/topics/jumble-solver-aids.md), "Outcome" to "After the
  owner's answers", about lines 932 to 1034.
- The [re-plan of 22 October 2026](replan-2026-10-22.md), section 4. It covers DT-J2 to DT-J4, Look Lab's L1 and the
  render contract revision R5.
- The Jumble interfaces and handoff:
  - [jumbling-interfaces.md](jumbling-interfaces.md);
  - [jumbling-observation.md](jumbling-observation.md);
  - the handoff pages in [jumbling-handoff/](jumbling-handoff/): the design table (DH), Look Lab (LH), Taste Lab
    (TH), engineering (EH) and the research data.
- The [LP4 plan](language-lp4-plan.md). It covers the editor services, the palette, overlay rows, recording and the
  log file. LP4's editor look waits for freeze round 3.
- The LP5 plan, `docs/progress/1.0/language-lp5-plan.md`. It lands with LP5's first pull request. Its section 6 fixes
  the research output panel's data interface and leaves the panel's look, layout, text and motion to ADR-D02 and K2.
- The [command table](command-table.json) and the [design table protocol](design-table/README.md).
- Track J of the ADR-D03 solve-test plan, adopted by the owner on 23 November 2026 as the recommended values. It is
  restated in section 7.

Evidence kind: design records from source and documents only. Nothing here is Windows, DirectX, input, performance or
human evidence.

## 1. What ADR-D02 decides

ADR-D02 covers the Jumble and research surfaces. ADR-D01's frame hosts them:
- The A6 review's item J found three surfaces hosted (J-1, J-2, J-5).
- Revision 6, frozen at A7, closed the rest:
  - J-3: the undefined value pattern;
  - J-4: the release order;
  - J-6: a repair commits whole;
  - J-7: typed, matching job results;
  - J-8: research origin is checked at preview admission;
  - J-9: outcome-neutral motion and drags.

So ADR-D02 adds no host rule. It decides the layout, look, words and motion that DT-J2 to DT-J4 own, and the slot
names of these surfaces. Its framework items and slot names freeze in round 2 with K2.

| Surface | Topic and questions | Settled; not reopened | Open in ADR-D02 |
| --- | --- | --- | --- |
| Cap map, slice layout, raw view, entry defaults, Global layout at deep states | DT-J2: Q1, Q4, Q5, Q6, Q8; DH-02 to DH-04, DH-12, DH-15 to DH-17, DH-19, DH-26 | DT-J1's O1 (b): a threshold set on the fixed map, no default and no sort; distinct "blocked" and "at its floor" patterns | Layout and arrangement inside ADR-D01's frame; slot names |
| Domain palette (up to 37 domains), exposed-face material, height ramp, totals, ν | DT-J3: Q2, Q3; DH-05, DH-13, DH-14; TH-02 to TH-09, TH-12 | Totals as plain numerals with no gauge or "lower is better" colour (A1, O2 (a)); ν's undefined pattern (revision 6, J-3) | Encodings within one G4-checked sticker palette per family and ADR-D01's state roles |
| The folded tree of named parts, slice motion, the repair list, the step check and kept checks, words | DT-J4: Q7, Q9; DH-11, DH-18, DH-20, DH-23, DH-24 | The repair list in neutral order with no ranking (O9 (a), O11 (b), the 17 October exception); whole commit (O10 (b), J-6); outcome-neutral motion (J-9) | Presentation, motion and words within the text budgets |
| Research output panel | LP5 section 6 rows | A row reads and checks; none chooses, ranks or applies a solving word | Layout, look, text, motion, stale and pending states, slot names |
| Tuning panel: A/B slots, saved presets, advanced overrides | Evening decisions, item 2 (point 13) | Built at the end of ADR-D02 | Placement and slot names |

Out of scope:
- reopening DT-J1, the owner's answers of 22 October 2026 or the 17 October exception;
- new aids;
- the research method itself;
- any edit to ADR-D01's frozen text.

## 2. What may start and what waits

| Work | State | Waits for |
| --- | --- | --- |
| A1 to A3 | Held | The owner's word in the receiving session |
| A4, the owner's pick page | After A3 | A3's options |
| The ADR text and its read | After A4 | The owner's picks |
| The ADR-D02 shell build | After the ADR text | The owner's answer on who builds the ADR-D02 and ADR-D03 shells, and on what budget (open since the A7 freeze) |
| Jumble drawing in Look Lab | Waits | L1, the per-piece pose path (LH-14), and the owner's answer on when jumbling enters Look Lab |

Rules for A1 to A3:
- **No paid API.** The design and art line uses no API until the owner says so (22 November 2026). A1 to A3 run on
  subscription subagents only.
- **No owner rating data goes to a subagent or a model.** This covers Taste Lab exports and ratings, picks, and
  `work/adr-line/a4/`. TH candidates are cited by ID only. Comparisons that need the owner's taste go to A4.
- **Files.** A1 to A3 write only under `work/adr-d02/` in the main checkout. It is git-ignored and private until A4.
  At the end of A3, one status line is added to this plan.

## 3. A1: the gap list

Goal: list everything ADR-D02 must decide, against what is already settled.

1. For each surface of section 1, collect:
   - the settled items: DT-J1's outcome, the owner's answers of 17 and 22 October 2026, ADR-D01 revisions 4 to 6,
     ENG-0J's command rows, LP5's section 6 rows, and LP4's items that touch these surfaces (palette, overlay rows,
     recording, keys);
   - the open items: DT-J2 to DT-J4's questions Q1 to Q9, and the DH, LH, TH and EH items still open.
2. Give each open item one status:
   - **decided by ADR-D01**: cite the item;
   - **needs a decision in ADR-D02**;
   - **needs data first**: for example X1's repair-list job timing before DT-J2 fixes the walk, L1 (LH-14), or R5;
   - **out of scope** (section 1).
3. Check each item J finding against revision 6 as written. If one is not closed, record it as a gap for the owner.
   Do not patch ADR-D01.
4. List each conflict between a frozen language clause and these surfaces, as ADR-D01's overrules did. Record it;
   do not resolve it.

Output: `work/adr-d02/a1-gap-list.md`, one row per item: ID, surface, source `path:line`, status, DT topic and a note.
It ends with counts per status.

Acceptance:
- Every DH, LH, TH and EH item that names a Jumble or research surface appears in exactly one row.
- Every row has `path:line` evidence.
- No settled DT-J1 item is reopened.

## 4. A2: reuse of the tool analyses

Goal: give each open item precedents from the existing tool analyses, without a new collection.

Sources:
- ADR-D01's A1 collection of 48 professional tools and its A2 reverse-engineering records, in `work/adr-line/out/a1/`
  and `work/adr-line/out/a2/` of the main checkout;
- the A3 pairing records in `work/adr-line/out/a3/`.

Not `a4-figs/` or `work/adr-line/a4/`: those hold the owner's picks. The class rule of ADR-D01's evidence section
holds: source and public reference only.

The patterns to look for:
- a map of many items with a threshold the user sets;
- numeral-only status tables;
- a list in neutral order with a row-focus preview;
- a step-through preview that commits whole;
- a job panel with typed outcomes;
- run and record browsers with reproducibility keys;
- a tree of named parts;
- A/B tuning slots and presets.

For each open A1 row, cite up to three precedents and say what each does, or write "none found".

Output: `work/adr-d02/a2-precedents.md`.

Acceptance:
- Every open row has precedents or "none found".
- Every source is public.
- Nothing from a third-party tool is committed.

## 5. A3: one design-table round

Goal: two or three options per open question, recommended first, as DT-J2 to DT-J4 resume (22 November 2026,
item 4).

Who:
- The four roles of the design table protocol, as subagents: `front-end-designer`, `interaction-artist`,
  `mathematician` and `engineer-advisor`.
- One round. The roles exchange views, and no role waits for another to finish.

Packets: three, one per topic. The research output panel goes with DT-J2 (layout) and DT-J4 (words and motion), and
the tuning panel with DT-J2. Each packet holds its A1 rows and A2 precedents, and these limits:
- ADR-D01's frozen frame: slot names, the inspector tiers, D2 states, the undefined pattern (J-3) and outcome-neutral
  motion (J-9);
- the human-solve boundary and the 17 October exception;
- the text budgets of about 20, 50 and 80 words per screen by context;
- one G4-checked sticker palette per family;
- the look-cost envelope of the PHS.52 revision that follows A7.

What each role adds:
- **Engineer advisor:** cost and feasibility against the renderer gate and that envelope. It names what needs L1
  (LH-14) or R5.
- **Mathematician:** checks the terms (DH-09, through `docs/progress/1.0/glossary/build.py --check`) and checks
  that each encoding is true to the puzzle: domains, heights and ν.

The session merges the answers into options per question.

Output: `work/adr-d02/a3-round/`: the packets, each role's answers, and the merged options per question.

Acceptance:
- Every A1 row that is a DT-J2 to DT-J4 question has two or three options, with a recommendation, a cost note and a
  mathematician's check.
- No option reopens DT-J1 or breaks a frozen ADR-D01 item.
- Each option that needs the owner's taste, such as the palette or the material, is marked for A4.

## 6. Parallel work and boundaries

- **LP4 and LP5 run in parallel.** ADR-D02 edits neither their crates nor the command table. A row that ADR-D02 needs
  is named as a row request for the top integrator, with its source, as ADR-D01.84 did for `file.document.*`.
- **K2** opens after A7 and ADR-D02, beside LP4 and LP5. The front-end designer and the interaction artist return for
  it.
- **Look Lab:** A1 to A3 build nothing there.

## 7. The acceptance benchmark ADR-D02 carries (Track J)

The research module is part of the shell that ADR-D02 builds, so Track J of the ADR-D03 solve-test plan is ADR-D02's
acceptance. The owner adopted it on 23 November 2026 as the recommended values. The seed range, the compute budget per
depth and the base target hours are set when ADR-D02 starts.

The setup:
- A researcher agent works only through the module, the language and scripts it writes itself. It does not use the
  frozen research scripts.
- Seeds are fresh scrambles made as the census made them. They are disjoint from the research seeds (7001 to 7109 and
  7301 to 7307) and from the 9000 to 9099 block, which stays reserved for the shelved tutorial's acceptance test.
- The researcher agent runs on the owner's API only after the owner confirms provider, model and budget.

The bars:
- **J0 Validity** (gate). The run catches at least 80% of the defects planted in the module. It replays the recorded
  restoration of seed 7001 through the module, with every digest matching.
- **J1 Operability** (gate).
  - Every research operation is reachable from the keyboard.
  - Observation works as `jumbling-observation.md` requires.
  - The repair list follows the 17 October boundary.
  - Built-in tools never choose: the boundary count is zero.
- **J2 Reproduction** (gate). Three fresh scrambles at depth 10 and three at depth 20 reach the solved state. An
  independent checker replays every record twist by twist.
- **J3 Frontier** (scored, not a gate). Three fresh scrambles at each of depths 40, 100, 300, 1000 and 3000. Each is
  scored against the frozen record at its depth (research data, section 3).
- **J4 Output** (gate). Records in the research data's section 5 formats, with long steps as trees of named parts and
  a one-paragraph finding. The checker passes every record.

When it runs:
- **Reduced**, after ADR-D02's shell, on sample data: J0 and J1.
- **Full**, after K2's greybox wires the Jumble repair flow and the research panel to the engine: J2 to J4.

A3 checks that every surface the benchmark needs is in the gap list:
- keyboard reach for every research operation;
- observation;
- the repair list's boundary;
- research records with trees of named parts.

## 8. After A3 (outline only)

1. **A4.** One owner page of picks, with the recommended option first. It also carries the open question of who builds
   the ADR-D02 and ADR-D03 shells, and on what budget. Questions reach the owner at most once per milestone.
2. **The ADR text.** One read by a fresh Claude subagent under the Codex minimum. Its framework items and slot names
   freeze in round 2 with K2.
3. **The shell build** with the tuning panel at its end. Then the reduced Track J run.
4. **Two held tasks.** When ADR-D02 is finished, the owner is reminded once of the two design-line tasks held until
   then: the level-game learning sequence on paper, and a one-flow art-capacity probe with real values.

## 9. Estimates

These estimates bind nothing.

| Step | Agent days |
| --- | --- |
| A1, the gap list | about 1 |
| A2, the precedents | about 0.5 |
| A3, one round of three packets | about 1.5 |

Owner time is none until A4, and about 30 to 45 minutes at A4.
