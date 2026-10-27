# Jumbling research data at the freeze (27 October 2026)

Research data handed to the engineering line under the owner's decisions of 27 October 2026
([handoff](../../../wiki/decisions/owner-decisions-2026-10-27-jumbling-handoff.md),
[freeze](../../../wiki/decisions/owner-decisions-2026-10-27-freeze.md)). The owner frames the jumbling line as a puzzle
to be solved: a methodology that restores the complete puzzle from any scramble of depth 3000 is open. This page gives
the research data only: the definitions the records use, what is proved, what was computed at each scramble depth and
where each method on record stopped, what is open, and where the records are and in which formats. It states no task,
proposal or ranking for the line. The jumbling research is frozen, so nothing here changes; every file it cites is on
`main`.

Labels are the research's: **proved** (with the review it had), **certificate** (an exact computation with a replayable
record), **computed** and **measured** (exact results on finite inputs), **open**. No proof here has had a human review.
A citation `path:line` refers to the file at the freeze. Evidence kind: source, fixture and exact synthetic geometry,
from headless research code on Linux; run times are cloud timings of research code, not performance evidence. Section 7
names the one measurement made on the owner's machine.

## 1. Definitions the records use

| Term | Definition | Source |
| --- | --- | --- |
| Model | The retained 600-cell-Full geometry: 177,120 pieces, 259,800 sticker slots and 600 grips; the rotation group K⁺ of the cut arrangement has order 7,200. | `research/jumbling/restoration/README.md:8-11` |
| Menu I_a | At each grip 60 items: the 12 rotations that keep the cut arrangement (retained) and 48 jumble twists of one icosahedral group that do not; 36,000 letters (grip, item) in all. Signed by the owner on 13 and 14 October 2026; J1 identity `fddc8532b9184bd1b413c1032b5a7a305dd9fd1f10385d87ba6ec4fd06eb958c`. | `docs/wiki/decisions/owner-decisions-2026-10-13-jumbling-menu.md:40`; `research/jumbling/fixtures/wj-I_a.json` |
| Admissible | A twist (c, g) is admissible when no piece straddles the cut of grip c. It is decided in exact arithmetic over Q(√5), with no tolerance. | `research/jumbling/state-contract.md` section 3 |
| On and off the lattice | A piece is on the lattice when its pose is in K⁺, so that it fills a chamber of the retained puzzle; otherwise it is off. An off piece can straddle other cuts and block their grips. | `research/jumbling/restoration/README.md:13-15` |
| Clean and blocked grips | A grip is clean when no piece straddles its cut (its twists are admissible), blocked otherwise. | `research/jumbling/restoration/14-blocking-structure/README.md:20` |
| Height | A piece's left height h is the left tree distance of its pose, an even number. Every reachable pose has right height 0 (proved, model review); one letter changes a left height by at most 2 (computed over the 36,000 letters). | `research/jumbling/restoration/12-low-height/PROOFS-B.md:6292-6302` |
| n₂, n₄, n₆ | The off pieces at left heights 2, 4 and 6. | `research/jumbling/restoration/14-blocking-structure/README.md:124` |
| N | Total misalignment: the cut cells covered on one side of a cut and not on the other by the same domain. N = 0 exactly on the lattice (M3), and a twist changes N only along the cut of its cap (M2); both proved. | `research/jumbling/restoration/11-scramble-census/README.md:29-31`; `research/jumbling/theory/restoration.md` M2, M3 |
| Φ | Φ = Σ_A max(0, h(A) − 2) over the pieces; Φ = 0 when no piece is above height 2. | `research/jumbling/restoration/14-blocking-structure/README.md:24` |
| Unit | A set of off pieces with one pose: a rigid block, or one frame part of a block in several frames. A clean step lands one unit and keeps every lattice piece on the lattice and every other off piece outside its free set at its exact pose. | `research/jumbling/restoration/README.md:77-80` |
| Restored | Every sticker slot carries its home label; a rotation of a piece that maps its stickers onto themselves does not matter (owner, 15 October 2026). Records say `solved` in this sense. | `research/jumbling/restoration/README.md:20-25` |
| Scramble of depth k | k admissible jumble twists, each at a uniformly random grip with a uniformly random jumble item of that grip; a draw that is not admissible is drawn again; no retained twists (`theory/restore.py`, `scramble`). The 21 acceptance scrambles (seeds 9011 to 9073, three at each of seven depths from 10 to 3000) are drawn the same way; none was run. | `research/jumbling/restoration/11-scramble-census/README.md:17-20` |

Chapter 1 of the tutorial ([`01-state.md`](../../../../research/jumbling/tutorial/01-state.md)) defines the other terms
the records use (domains, seams, the state digest, snapshots, entry tests, step trees), and section 1 of
[stage 14](../../../../research/jumbling/restoration/14-blocking-structure/README.md) those of the deep-state analysis
(labels, holders, branches, foreign load, own holders).

## 2. What is proved about restoring

| Result | Statement | Status | Source |
| --- | --- | --- | --- |
| Reversal | After an admissible twist (c, g), the twist (c, g⁻¹) is admissible and undoes it, and I_a contains every inverse; so every scramble is undone by its inverse twists in reverse order. | proved | `research/jumbling/state-contract.md` section 3; `docs/wiki/decisions/owner-decisions-2026-10-27-freeze.md:50-56` |
| A clean step exists | At every reachable state with an off piece, an exhaustive search over words that reads only the state finds a clean step. There is no bound on its length. | proved, senior model review of 15 October 2026 | `research/jumbling/restoration/README.md:58-60` |
| Lemma U | A clean step lowers the off count by at least its unit's size, so the off-lattice phase terminates. | proved | `research/jumbling/restoration/README.md:56-57` |
| Theorem M\* | Fast steps, the exhaustive search as a fallback, then the lattice phase with the corrections last, restore every reachable configuration from the current state alone, in at most as many clean steps as there are off pieces at the start. The fallback has no length bound. | proved, senior model review | `research/jumbling/restoration/README.md:61-67` |
| X1 | R(I_a) is infinite: an explicit family of words, admissible from the solved state, raises the left heights of seven pieces without bound. So no bound on the restoration length holds for every reachable configuration. | proved; checked by the integrator and two referee subagents | `research/jumbling/README.md:70-74`; `research/jumbling/restoration/10-x1-unbounded/README.md` |
| Theorem D | A computable bound in (n, h), the off count and the largest height, valid at every reachable state, or any computable bound on pose tables, exists exactly when reachability is decidable. M1 is neither settled nor refuted. | proved, model review (round 21) | `research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md:7203` |
| Theorem C | If the two poles of a carrier's edge cell and one common neighbour are free, a word of at most 720 twists turns that cell alone; vertex cells at most 104; triangle and tetrahedron contents move by transport words of at most 240 and 192 twists. The bounds assume the class's grips are free. | proved, senior model review | `research/jumbling/restoration/README.md:68-73` |
| G = 𝒦 | The retained puzzle's group is exactly the kernel of its invariants (35 position and 11 frame certificates); its order has 598,061 decimal digits, its index in the full wreath product is 2^43 · 5^2. | certificate | `research/jumbling/restoration/README.md:46-48` |
| A_J | The corrections a jumbled history can leave on the lattice form A_J = C2^34 × C5^2. | lower bound certificate, upper bound proved | `research/jumbling/restoration/README.md:49-50` |
| Theorem R | With a correction library that spans A_J (36 macros, rank 34 over F2 and 2 over F5), every reachable configuration with all pieces on the lattice is solved in the sticker sense. | proved; restricted to reachable configurations by the senior review | `research/jumbling/restoration/README.md:51-55` |
| Lattice bounds | The round 11 construction needs at most 20,866,844 twists for every input, or 4,655,642 actions (single twists and applications of 70 stored macros); the corrections at most 25,852 more. With retained words only, some input needs at least 224,601 twists. | proved, model review | `research/jumbling/README.md:57-69`; `docs/wiki/decisions/owner-decisions-2026-10-17-jumbling-m2.md:21` |

The theorems of the deep-state cards (cards 6 to 9: the precedence peel, the chronological certificate MC, the
lowering machine, transports and routing releases, the profile identity of whole-cap plans, Theorem N) and their
status are in the cards' index, [`theory/cards/README.md`](../../../../research/jumbling/theory/cards/README.md), lines
29 to 32, with proofs in the `PROOFS.md` of stages 14 to 18.

## 3. Scrambles by depth

### 3.1 The census: the state-only descent

Phase U of the theory (rules seam, cover, close, conj, lock, improve, shift, reopen; every kept step lowers N) from
unrestricted random scrambles, 75 minutes of descent per round, and 5.25 hours for seeds 7304 to 7307. Every record
replays on the exact simulator (`scripts/check_records.py`). Source:
[`11-scramble-census/README.md`](../../../../research/jumbling/restoration/11-scramble-census/README.md), lines 49 to 145.

| Depth | Seed | Off after the scramble | Left heights after the scramble | End | Off at the end |
| ---: | ---: | ---: | --- | --- | ---: |
| 10 | 7001 | 26,519 | all 2 | solved (9 twists) | 0 |
| 20 | 7106 | 51,828 | all 2 | lattice (19 twists) | 0 |
| 40 | 7101 | 90,441 | all 2 | time limit, two rounds | 1,193 |
| 40 | 7107 | 97,491 | all 2 | solved (36 twists) | 0 |
| 60 | 7104 | 106,075 | all 2 | time limit, two rounds | 14,610 |
| 100 | 7102 | 115,566 | 2: 115,485; 4: 81 | time limit, two rounds | 31,479 |
| 100 | 7108 | 117,876 | all 2 | time limit | 27,199 |
| 200 | 7105 | 119,469 | 2: 118,956; 4: 513 | time limit | 75,054 |
| 400 | 7109 | 118,493 | 2: 118,083; 4: 410 | time limit | 73,003 |
| 1000 | 7103 | 125,304 | 2: 123,685; 4: 1,484; 6: 135 | time limit, two rounds | 81,309 |
| 3000 | 7302 | 118,821 | 2: 116,665; 4: 1,971; 6: 185 | time limit | 100,584 |
| 3000 | 7304 | 121,001 | 2: 118,951; 4: 2,050 | time limit (5.25 h) | 95,569 |
| 3000 | 7305 | 116,870 | 2: 113,826; 4: 3,044 | time limit (5.25 h) | 105,437 |
| 3000 | 7306 | 123,181 | 2: 118,426; 4: 4,726; 6: 29 | time limit (5.25 h) | 108,823 |
| 3000 | 7307 | 119,518 | 2: 117,245; 4: 2,263; 6: 10 | time limit (5.25 h) | 96,236 |

- No census run got stuck: every run that did not reach the lattice was stopped by the clock, and every step it took
  lowered N. Seam, cover and conj (the three-twist repair) did all the work (lines 158 to 161).
- Along three 3000-twist scrambles (seeds 7301 to 7303, a checkpoint every 250 twists, no descent) the off count stays
  between 115,610 and 124,344 from 250 twists on, and no piece is above left height 6 at any checkpoint; pieces at
  height 6 come and go (0 to 287 at a checkpoint) (lines 97 to 110).
- From the 3000-twist state of seed 7302 the 185 pieces at height 6 stayed there through all 109 steps of the descent
  (lines 122 to 129).
- At the conj steps of the seed 7101 record from step 44 on, 0.08% to 6.67% of the three-twist repairs at the cap the
  rule chose lowered N (line 278).

### 3.2 The tutorial runner's development runs

The runner follows the tutorial's rules from the current state and plays literal words; an independent checker
(`check_run.py`) replays a record twist by twist. Sources: the
[tutorial README](../../../../research/jumbling/tutorial/README.md), lines 58 to 68, and each run's `RESULT.md` under
[`13-tutorial-runner/runs/dev/`](../../../../research/jumbling/restoration/13-tutorial-runner/runs/dev/).

| Run | Input | End | Checked |
| --- | --- | --- | --- |
| D0 | seed 7001 (depth 10) and seed 7106 (depth 20), acceptance mode | both solved; at depth 20, 19 one-letter bulk steps and a lattice phase of 35 steps, 8,701,310 twists | yes, both |
| D1, development mode | seed 7101 (depth 40) | bulk stuck at 1,193 off; one tail step to 1,082 off; killed by a container restart in the tail | no |
| D1, run 4 | seed 7101 (depth 40), acceptance mode | the tail entered at 1,193 off and stopped `stuck` after 17 tail steps at 158 off, all at height 2, in five units (114 steps in all) | yes |
| D2 | the 3000-twist census stops of seeds 7302 and 7304 to 7307, and seed 7303 from its scramble | none reached the lattice phase; the height phase stuck with n₄ between 734 and 1,955; the high phase stuck (`kernel_exhausted`) at 7304 to 7307, ran out of its clock at 7303 and was cut off by a lost container at 7302; the tail stopped while listing the units at 7304, 7305 and 7307 and was cut off by a lost container at 7306 | four of six |
| D3 | seed 7102 (depth 100), acceptance mode | stopped at the freeze at its last checkpoint: 530 bulk steps, 14,528 off, N 19,560, 245,275 s of runner time; N was falling by about 1 to 20 an hour | no (interrupted record) |

Sources for the D0 lattice phase and the D1 tail: `research/jumbling/restoration/13-tutorial-runner/README.md:594-601`;
`research/jumbling/restoration/18-tail-completion/README.md:17-20`. D3's rate:
`docs/wiki/decisions/owner-decisions-2026-10-27-freeze.md:30-32`.

### 3.3 Restorations to the solved state on record

| Input | Route | Lengths | Status |
| --- | --- | --- | --- |
| Seeds 7001 (depth 10) and 7106 (depth 20) | the runner (D0) | 9 letters; 19 letters and a lattice phase of 8,701,310 twists | checked records |
| Seed 7107 (depth 40) | the census descent, seam steps only | 36 twists | certificate |
| The I_a@40 fixture (40 twists, 94,147 off) | a 62-twist seam descent to 860 off (stop62); 14 clean steps whose order a researcher chose by hand; the lattice phase | 14,098 twists in the steps; corrections 8,129 and cascade 151,011,430 twists | certificate (`research/jumbling/restoration/README.md:33-43`) |
| Seed 7101 (depth 40), route A | the census descent (223 twists) to 1,193 off; round 3 of line (a) to 737 off (4,516 twists); E2's 13 literal strict′ steps to the lattice (2,012,155 twists, steps of 692 to 672,417); the lattice word | 168,805,299 twists after the scramble, of which 166,788,405 in the lattice word | one trajectory joined from several scripts' records, not a runner run; the lattice word checked by composition, not played (`research/jumbling/restoration/12-low-height/README.md:934-949`) |
| Seed 7101 (depth 40), route B | the runner's 114 steps (D1, run 4) to 158 off; four literal whole-cap steps of card 9 (T6) to the lattice; the runner's lattice phase | T6: 1,510,756 letters, the longest step 871,582; lattice phase: 53 steps, 13,231,266 twists | identity labels; not a runner run; T6's step 4 passed the length and composed gates, and its twist-by-twist replay was stopped at the freeze (`research/jumbling/restoration/18-tail-completion/README.md:41-46`, `:67-76`) |

On record, no scramble deeper than 40 twists reached the lattice. Card 9's state-only rule completes the depth 40 tail
stop in three stages of 688,686 letters (the longest 374,014), which pass the length and composed gates; their
twist-by-twist replays were stopped at the freeze (stage 18, lines 146 and 214).

### 3.4 The six deep stops (depth 3000)

The ends of the runner's height phase in D2: the states where the research on height 4 worked. Off counts, heights and
clean grips from `research/jumbling/restoration/14-blocking-structure/README.md:133-141`; Φ at the start from
`research/jumbling/restoration/14-blocking-structure/README.md:443-448`; the lowest Φ reached from
`research/jumbling/restoration/17-routing-in-driver/README.md:10-12`.

| Seed | Digest | Off | n₄ | n₆ | Clean grips (of 600) | Φ at the start | Lowest Φ reached (cards 6 to 8, stage 17) |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 7302 | `2430f4cc…` | 103,484 | 734 | 0 | 37 | 1,468 | 388 |
| 7303 | `d01dc91e…` | 100,650 | 929 | 87 | 26 | 2,206 | 536 |
| 7304 | `ac127dc3…` | 96,471 | 797 | 0 | 41 | 1,594 | 438 |
| 7305 | `cc5ef7ec…` | 109,872 | 1,419 | 0 | 22 | 2,838 | 386 |
| 7306 | `ac4be82c…` | 110,315 | 1,955 | 0 | 26 | 3,910 | 1,158 |
| 7307 | `895b45de…` | 100,569 | 1,585 | 0 | 38 | 3,170 | 1,100 |

- Φ fell by 65% to 86% and reached 0 at none of the six. Each procedure on record stopped: the precedence peel T3P
  chained greedily (−6% to −56%), the certified procedure P\* over five macro families, P\*\* with transports, and
  P\*\*\* with routing releases, at whose stops thousands of releases exist and no tail of its forms descends (stage 17,
  lines 10 to 15).
- Where such a procedure stops depends on the route: at 7305 the end of an excursion descent leads to Φ 508 and the end
  of a tame one at the same Φ to 730 (`docs/wiki/log.md`, entry of 25 October 2026; stage 16).
- At six height-4 states rebuilt from these lines (stage 12), D19 and the R22D-25 kernel ended stuck, not at a budget;
  559 to 578 of the 600 caps held a straddler. Stuck covers only D19's case set and the kernel's catalogue: it does not
  show that no step exists (`research/jumbling/restoration/12-low-height/README.md:1408-1488`).

### 3.5 Lengths on record

| What | Lengths | Source |
| --- | --- | --- |
| Strict′ and clean steps at the depth 40 stop | 10 to 1,158 letters for the first steps found; E2's literal steps 692 to 672,417 twists | `research/jumbling/restoration/12-low-height/README.md:934-938`; `docs/progress/1.0/jumbling-handoff/engineering.md`, EO-06 and EO-16 |
| Card 9's whole-cap steps at the tail stop | T6: 86,866 to 871,582 letters (four steps, 1,510,756); the rule's stages: 688,686 in three, the longest 374,014 | `research/jumbling/restoration/18-tail-completion/README.md:41-46`, `:146` |
| The 36 corrections | 5 to 5,629 twists (library `research/jumbling/restoration/09-general-method/data/library-merged.json.gz`) | `research/jumbling/manual/04-lattice.md` section 4.7.2 |
| Lattice phases from scrambles | 8,701,310 (seed 7106); 13,231,266 (seed 7101, route B); 166,788,405 (seed 7101, route A) | sections 3.2 and 3.3 |
| Lattice phases on two synthetic inputs | 15,761,436 and 15,760,112 twists (round 11 construction) | `research/jumbling/restoration/09-general-method/METHOD.md:147` |
| The X1 witness `H6030.json` | 3,156 letters from the solved state to left height 6,030 | `research/jumbling/restoration/10-x1-unbounded/README.md:23` |

## 4. What is open at the freeze

| Item | What is open | Source |
| --- | --- | --- |
| M1 | A construction from the state alone with a bound on its length, which must depend on the state (X1). By Theorem D a general computable bound in (n, h) would decide reachability. | `research/jumbling/restoration/README.md:95`; `research/jumbling/restoration/09-general-method/SENIOR-REVIEW.md:7203` |
| E1 | Coverage: no scramble deeper than 40 twists was restored, and the general method of Theorem M\* (with its fallback) was not run from the deep stops. | `research/jumbling/restoration/README.md:97`; section 3 |
| E2 | A human review of the proofs; a Lean formalization was planned for after M1. | `research/jumbling/restoration/README.md:98` |
| Bulk at depth 100 and more | A bulk rule that takes the off count down at depth 100: D3's bulk was still descending, slowly, when it was stopped. | section 3.2 |
| Height 4 to height 2 | No rule on record always takes height 4 to height 2; Φ reached 0 at none of the six deep stops; the coverage of B′(2) is open. | section 3.4; `research/jumbling/restoration/12-low-height/README.md:667` |
| The tail from its entry | From D1's tail entry whole caps alone stick (Theorem N, a certificate on a residue of 356 pieces); a mixed rule (whole-cap plan first, else the runner) is not shown to complete there (DIS-3, not run); a rule with a proof on a class that contains D1's entry is open. | `research/jumbling/restoration/18-tail-completion/README.md:112-119`, `:280`, `:304-312` |
| The third gate of four words | T6's step 4 and the rule's three stages were not replayed twist by twist (5 of 81 chunks played). | `research/jumbling/restoration/18-tail-completion/README.md` section 3.3 |
| Card questions | Card 5, question 1.3 (a normal form from height 6); card 6, question 3.3; card 7, question 3.2; card 8, question 1.1 for holders 307 and 517; and the parts of cards 6 to 9 answered only in part. | `research/jumbling/theory/cards/README.md:28-32` |
| X3 | The number of reachable configurations and the worst-case distance (deferred; not known even for the retained puzzle). | `research/jumbling/theory/open-problems.md:163` |
| Acceptance scrambles | None of the 21 (seeds 9011 to 9073) has run; the tutorial is shelved unaccepted. | `research/jumbling/tutorial/README.md:70-74` |

## 5. Data and formats

| What | Where | Format and check |
| --- | --- | --- |
| Exact reference simulator J1 | [`research/jumbling/sim/`](../../../../research/jumbling/sim/README.md) | exact poses over Q(√5); tests `tests/test_jumbling_sim.py` |
| Fixtures (S4, I_a and I_b menus) | [`research/jumbling/fixtures/`](../../../../research/jumbling/fixtures/README.md) | scripted scrambles with references; `python research/jumbling/fixtures/wj.py check` replays them |
| State and render contracts | [`state-contract.md`](../../../../research/jumbling/state-contract.md) (sections 1 to 7, amendments A1 to A5), [`render-contract.md`](../../../../research/jumbling/render-contract.md) | what a valid jumbled state is; the drawing data contract `magic600-jumbling-render/1` |
| Census records | `research/jumbling/restoration/11-scramble-census/runs/` (`round1/` to `round3/`, `heights/`) | scramble history, moves, steps and `table.jsonl`; `scripts/check_records.py` rebuilds each scramble from its seed and replays every move |
| Runner records | `research/jumbling/restoration/13-tutorial-runner/runs/dev/<run>/` with `RESULT.md` | format `runner-run/3`: `run.json`; one word file per step, `{word: [[grip, item], ...], input_digest, output_digest}`, gzipped above 100,000 letters, with its SHA-256; one step tree per step; checked by `scripts/check_run.py` ([chapter 7](../../../../research/jumbling/tutorial/07-records.md)) |
| Long words | `ia16` files | gzip of little-endian uint16 codes 60·grip + item, the SHA-256 over the uncompressed bytes ([chapter 7](../../../../research/jumbling/tutorial/07-records.md)) |
| Words known by a certificate | stage 12, `scripts/expand_slp.py` and `scripts/slp_apply.py` | a straight-line program; `expand_slp.py` expands it to a literal word after checking its exact length against a limit (10⁷ by default) |
| Deep-state pose tables | listed with SHA-256 and digest in `research/jumbling/restoration/14-blocking-structure/results/sources.json`; summaries in `results/states.json` | pose tables of the D0 to D2 states in section 3.4 and the table of stage 14 |
| Results of cards 6 to 9 | `results/` of stages 14 to 18 | certificates, stop states and words, each with its replay command in the stage README |
| Lattice phase | library `research/jumbling/restoration/09-general-method/data/library-merged.json.gz` (36 corrections); restorers `scripts/lattice/scripts/restore_lattice_late.py` and `scripts/schedule11/restore_schedule11_late.py` in stage 09 | checker `scripts/lattice/checker/verify_late.py` (identity labels on all 259,800 stickers) |
| Research viewer | [`research/jumbling/viewer/`](../../../../research/jumbling/viewer/README.md) (`full.html`, all 259,800 stickers of a state; shot mode) and the explorer [`research/jumbling/explorer/`](../../../../research/jumbling/explorer/README.md) | WebGL pages for research, not the Direct3D 12 renderer |
| Theory modules | [`research/jumbling/theory/`](../../../../research/jumbling/theory/README.md) (`rim.py`, `seams.py`, `trees.py`, `replay_check.py` and others) | rim summaries, seams, trees and independent replays |
| Written accounts | the tutorial ([`research/jumbling/tutorial/`](../../../../research/jumbling/tutorial/README.md), shelved unaccepted), the manual ([`research/jumbling/manual/`](../../../../research/jumbling/manual/README.md), a draft) and the restoration stages ([`restoration/README.md`](../../../../research/jumbling/restoration/README.md)) | rules, worked examples and records of the research |

## 6. Earlier handoff entries

The handoff and observation logs of 18 to 22 October 2026 on this directory's line pages hold earlier research data
with their sources (for the engineering line: [engineering.md](engineering.md), EH-01 to EH-34 and EO-01 to EO-24).
Their "what the line does with it" columns are the research's proposals of those dates.

## 7. Recorded measurement on the owner's machine

The jumbling pose probe ran the W-J I_a fixture at a pooled average of 613.28 fps, p99 1.911 ms, on the owner's RTX
4070 Laptop GPU; overlay costs were not measured
([evidence](../../../wiki/evidence/w-j-probe-results.md), line 41). It is the only Windows, Direct3D 12 or
performance evidence this page cites.

## Checks not run

- This page was written by reading the repository in a Linux session. No research script, test, run, build, browser or
  GPU run was made for it, and no number was recomputed; each is read from the record cited.
- No Windows, DirectX (Direct3D 12), input, long-session or performance check; the native host regression
  (`tests/native/NativeHostRegression.cs`) needs Windows and was not run.
