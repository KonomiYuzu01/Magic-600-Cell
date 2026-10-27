# Stage 18: the tail completion (card 9)

Status: **research; the joint attack on card 9 is done (section 3).** The record of
[card 9](../../theory/cards/card-9-tail-completion.md), opened by the
[owner decision of 26 October 2026](../../../../docs/wiki/decisions/owner-decisions-2026-10-26-tail-then-freeze.md)
after stage 17; with this card the tutorial and the jumbling research are frozen. Everything here is an exact
computation on the research simulator, headless on Linux (synthetic geometry; no native-host, input or performance
evidence).

Labels: **proved** (with the proof in [`PROOFS.md`](PROOFS.md)), **computed** (exact, by the program named),
**reported** (computed during the attack by an analyst's own program, which is not committed; "both" when both
analysts computed it independently), **conditional** (the condition is named), **open**.

## 1. The question

The tutorial runner's tail ([chapter 5](../../tutorial/05-tail.md)) lands the pieces still off the lattice one unit per
step, each step one literal strict′ word. The depth 40 development run D1 (seed 7101, run 4, acceptance mode;
[`13-tutorial-runner/runs/dev/d1-acc-s7101/`](../13-tutorial-runner/runs/dev/d1-acc-s7101/)) entered the tail with
1,193 pieces off and stopped `stuck` after 17 tail steps with 158 off, all at height 2, in five units (card 9, "The
stop"). Card 9 asks what completes the tail there and what keeps a tail from sticking.

## 2. The computed facts of the card

### 2.1 The whole-cap test at the stop (card 9, T5)

`scripts/wholecap.py` is stage 12's COV-33 computation (`12-low-height/scripts/round22/cov33_whole_cap_737.py`) at any
pose table, with every cap that part A leaves open resolved by the inside-mask-empty flow of COV-10 and COV-32. At the
stop (`results/wholecap/d1-stop-158.txt`) nine caps are eligible: 353 passes unprepared (item 22, for 81223); 91, 148,
217, 548 and 560 pass prepared; 12, 324 and 439 fail under every preparation. Every unit has a passing instance. The
program reproduces COV-33's record at E2's 737 state row for row (`results/wholecap/e2-737.txt`) and finds E2's last
step, cap 91 at item 14 with a flow of 71, at E2's 71 state (`results/wholecap/e2-71.txt`).

### 2.2 The tail completed from the stop (card 9, T6)

Four whole-cap steps X·(c, i)·V′·(c, i)⁻¹·V′⁻¹, each one literal word built at its state, put every piece of the stop
on the lattice. The order was chosen by hand: a passing unprepared instance if `wholecap.py` found one, else the
passing prepared instance that moves the fewest holes.

| Step | Off | Cap, item | Lands | X (holes moved out of the cap) | V′ | Letters | End digest |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 158 → 129 | 353, 22 | 81223 | none | 43,432 | 86,866 | `1007a601…` |
| 2 | 129 → 104 | 217, 15 | 112734 | none | 224,538 | 449,078 | `5e9f262c…` |
| 3 | 104 → 71 | 560, 13 | 60085 | 9,748 (23) | 46,740 | 103,230 | `b28448f3…` |
| 4 | 71 → 0 | 91, 14 | 9013, 16314 | 21,592 (19) | 424,994 | 871,582 | `c75c8dde…` |

1,510,756 letters in all, the longest step 871,582 (the literal limit is 10⁷). `scripts/wc_prep.py` builds X:
the prepared hole set J from the flow (outside the cap, the same number of holes in every source orbit of the SAFE-ALL
group, current holes kept where the flow allows), an R22B-10 certificate for J at the state (`r22b10_cert.py --seeds
atlas`, checked by `check_r22b10.py`, expanded by `expand_slp.py`), played from the state; after X the holes are
exactly J and every off piece keeps its pose. `scripts/wc_t1.py` builds the step (stage 12's `e2_b06_t1.py` with the
cap and the unit as arguments); `tclose2.py`'s peeling and greedy searches found no literal V′ at any of the four
middles, so every V′ is an expanded R22B-10 certificate. Each step took 13 to 38 minutes of one core. The whole-cap
test at 129 and 104 off is in `results/wholecap/d1-t6-129.txt` and `d1-t6-104.txt`: after step 1, cap 217 holds
112734 alone and passes unprepared.

`scripts/replay_tail.py` replays `results/d1-tail/` from the stop step by step with `slp_apply.py --unit`, which
checks the strict′ contract of each step (the unit on the lattice, no collateral, every other off piece at its pose):
X, W and W⁻¹ are played twist by twist, V′ and V′⁻¹ are checked letter by letter as SAFE-ALL letters at the span's
start and applied as one composed action. The replay also checks every word's SHA-256 and length, every digest and
count, and the off count dropping by exactly the unit's size against `results/d1-tail/steps.json`: passed. Its gate
`scripts/test_replay_tail.py` plays twelve mutations of the record (among them an empty unit, a lattice piece in a
unit, wrong counts, a removed preparation, two adjacent letters of V′ swapped and the other step form) and a wrong
start table, each rejected, and the unmutated record, accepted: passed. Steps 1 to 3 were also played twist by twist,
one twist per letter, from the end of the step before; the replay of step 4 was stopped at the owner's freeze
(section 3.3).

### 2.3 The lattice phase from the tail's end

From T6's end table (`c75c8dde…`, every piece on the lattice), the runner in development mode (`runner.py --table`,
restorer `schedule11`, [chapter 6](../../tutorial/06-lattice.md)) went straight to the lattice phase and ended
`solved`: 53 lattice steps, 13,231,266 twists, 2,844 s (`results/lattice-d1/`). `check_run.py` passes the record
(verdict `solved`, every step passed; the same summary form as the committed D0 runs).

So D1's scramble (depth 40, seed 7101) is restored to identity labels by the runner's 114 steps to the stop, T6's four
steps and the runner's lattice phase. This is not a runner run: T6's steps are not runner rules, and their order is the
integrator's. T6's step 4 passed the first two gates of section 3.3 only.

## 3. Joint attack (26 October 2026)

Card 9 was attacked as the owner decided on 24 and 26 October 2026: two sealed analyses of the same packet, by Astra
(call `20261026T130034Z-623a2c5e`) and by Claude Opus (a subagent of the integrator's session); the integrator's checks
of the T6 replay claim (I1), of T6's shortened steps as one chain (I3), and a literal build of the first two stages
common to both answers' orders (I4); a cross-examination, in which each analyst answered every finding of the other
and the integrator's points (Astra's: call `20261026T172611Z-e152c6ef`, after a first call that ran out of time,
`20261026T153330Z-cc62c46c`); then the experiment for the point still in dispute, run by the integrator. The analyses
are private review records; this section records what they established. The proofs are in [`PROOFS.md`](PROOFS.md);
each was checked line by line by the analyst who did not write it and by the integrator.

### 3.1 The results (proved; `PROOFS.md`)

- **Profile identity (Lemma PI).** A whole-cap step lowers the hole count of every orbit of the next state's SAFE-ALL
  group by exactly its landings there, whatever the preparation, the vacating word, the form and the holes it leaves.
  So the hole profile after any sequence of whole-cap steps is computed at the start, without words, and an occupied
  chamber that is a one-chamber orbit stays occupied.
- **Static plans (Theorem SP).** A per-stage test P on the virtual states (Claude Opus) and a layered integral flow
  with prescribed endpoint holes (Astra) certify a complete whole-cap plan before any word is built, and they are
  equivalent in their resource conditions. Checked global Jordan witnesses on the partially filled source orbits and
  the middle demand orbits suffice: each gives an alternating group acting on its orbit alone, so the factors are
  independent (not diagonal), and a middle orbit that meets the cap but not the landing chambers is left as it is.
  Every tested order passes at the stop (reported, both): all twelve orders of {(91,14), (217,15), (560,13)} and of
  {(91,14), (217,15), (548,12)}, T6's order and (91,14), (148,15), (217,15), (548,12).
- **Prescribed holes and chains (Lemma PK, Corollary CH).** The three-segment step X·W·V′·W⁻¹ can leave any hole set
  with the right count in every middle orbit. Dropping V′⁻¹ changes the end, so a stored later word does not chain
  after it (I3, section 3.4); a chain of three-segment steps is rebuilt at each state reached, prescribed by a layered
  flow, or connected by a SAFE word.
- **The rule (WC-PLAN, Theorem R).** Fewest single-cap stages first, then lexicographic in (cap, item), with P at every
  stage, a residue gate, only the first stage built at the state reached, and admission by exact length before
  expansion and the three acceptance gates. After its first stage it finds the rest of the same plan (persistence), so
  it never sticks on its plan class when the admitted words exist. From the stop it gives (91,14), (217,15), (548,12)
  after 15 tests (both; at the 33-off state cap 439 fails at all 12 items before (548,12) passes); section 3.2 builds
  it.
- **Whole caps alone stick at D1's entry (Theorem N).** From D1's entry `7e128820…`, E2's 737 state `2e3b8f99…` and
  D1's table 00097, no sequence of whole-cap steps that land the whole content of their caps lands any piece of the
  residue R = {b07, b12, b13, b14, b15} (356 pieces), with any preparation, in either form, in any order, joint steps
  included: only caps 148, 217, 324 and 353 can land R, their mandatory chambers stay occupied by the profile identity,
  and an optimistic middle group fixes at least 5, 5, 22 and 7 landing chambers at every item (Claude Opus at all three
  states; Astra at the entry and at 737). Corollaries: Cover is not inductive (N2), and at a reachable state the glued
  unit 112734 has no whole-cap step at its cell's caps (N1). D1's recorded runner steps followed by whole caps complete
  from the entry (Proposition RP: 21 steps, 1,483,042 letters).
- **Lengths (Lemma L, Proposition T6-4).** With sealed seeds a whole-cap step has at most
  2Σ_A m_A c_A + 2 + 2Σ_Δ |D ∩ Δ| c_Δ letters (c = s + 2e + 2f), linear in the holes moved and the pieces landed. The
  tables make e and f polynomial in the orbit size, but a Jordan witness does not bound the seed length s, so no
  unconditional short bound follows (scope by Astra). T6's step 4 has a word of at most 7,001,082 letters (3,511,338 in
  the three-segment form), by R22B-31's row on an identical middle alphabet.
- **Joint steps (Lemma J, Proposition JT).** Separated caps, also of different classes, combine into one step. The
  triple (91,14) + (217,15) + (560,13) passes the joint test at the stop (flow 158, 490 middle grips, witnesses 28/28;
  both), so one strict′ step lands all 158 pieces; it was not built, and no saving over three steps is established.
- **Closures and fixed chambers (Proposition CL, Lemma FO, Proposition CB).** No closure with T1's cores lands a unit
  at the stop under any preparation; for glued units a finite bracket (sufficient: P passes; necessary: an optimistic
  middle group fixes no landing chamber) decides every candidate at the stop.

### 3.2 The rule's order from the stop, built (computed)

The rule's three stages were built one by one at the states actually reached, with this stage's builders
(`wc_prep.py`: J from the flow of P, current holes kept where the flow allows, X an expanded R22B-10 certificate;
`wc_t1.py`: V′ an expanded R22B-10 certificate with atlas seeds at the actual middle, `--literal-limit 10000000`, so
every length was computed exactly before expansion and checked against 10⁷), and each was played in the three-segment
form X·W·V′·W⁻¹ by `slp_apply.py --unit` from the end of the stage before:

| Stage | Start | Cap, item | Lands | X (holes moved out of the cap; kept) | V′ | Letters | Off | End | Profile after |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | the stop `966dc8a9…` | 91, 14 | 9013, 16314 | 34,006 (25; 133) | 340,006 | 374,014 | 158 → 87 | `d9edc19c…` | `0d92becac0cd4779` |
| 2 | `d9edc19c…` | 217, 15 | 81223, 112734 | 714 (3; 84) | 223,796 | 224,512 | 87 → 33 | `ec6785c3…` | `788c1e45cbeed5ad` |
| 3 | `ec6785c3…` | 548, 12 | 60085 | 43,528 (22; 11) | 46,630 | 90,160 | 33 → 0 | `eea9b972…` | `4f53cda18c2baa0c` (no hole) |

688,686 letters in three steps, the longest 374,014, against T6's 1,510,756 in four. Every stage is strict′ with no
collateral and every other off piece exact. The profiles after stages 1 and 2, computed on the end tables with the
second analyst's functions, equal the static predictions of section 3.1; after stage 3 no hole is left. Each V′ is an
expanded certificate (as in T6, `tclose2.py`'s peeling and greedy searches found no literal V′); the builds took 41,
20 and 53 CPU minutes, and the composed gates 560, 56 and 1,025 CPU seconds. The builder's own check of the
four-segment form of each step passed as well (714,020, 448,308 and 136,790 letters). The words, units and
`steps.json` (form `three-segment`) are in `results/rule-stop/`; `replay_tail.py` replays them, each stage strict′ to
its recorded end, and its gate passes on them: the twelve mutations and a wrong start table are rejected and the
record is accepted (14 cases). The third gate, a twist-by-twist replay of each stage's full word, was stopped at the
owner's freeze before any stage was completed (section 3.3). So the rule's order completes the stop in literal steps
that pass the first two gates, and the disputed point DIS-1 is decided for those gates (section 3.7). The lattice
phase from `eea9b972…` was not run: no new restoration is claimed (section 2.3 restores D1 from T6's end, and
Proposition B says why an end reached by other words needs its own).

### 3.3 Acceptance: three gates, and the replays

The analysts agreed that the replay of section 2.2 as first recorded overstated what it did: it said every step was
played twist by twist, but `slp_apply.py` (lines 370 to 393) applies a `safe_word_file` span, here V′ and V′⁻¹, as one
composed action after checking every letter as a SAFE-ALL letter at the span's start (X, W and W⁻¹ are played twist by
twist, lines 412 to 427). SAFE-ALL letters keep the off pieces and the SAFE-ALL alphabet, so the composed action
equals the product of the twists; that is a fact of the model, not a play of the word. A step of a record, and a step
a rule plays, is accepted only through three separate gates:

1. its exact length before expansion is at most 10⁷, and the expanded word has that length and its recorded hash;
2. the composed gate, `slp_apply.py --unit`: strict′ with no collateral and every other off piece exact;
3. a twist-by-twist replay, `stepchk.py --require-strict` (`12-low-height/scripts/round22/stepchk.py`, one twist per
   letter), from the actual previous end to the same end digest.

The twist-by-twist replays (`stepchk.py`, one twist per letter, every twist checked admissible; each word X, W, V′,
W⁻¹ as P·Q·P⁻¹ and, in the four-segment form, V′⁻¹; CPU seconds of one core on a shared machine, 17 to 32 ms per
letter):

| Word | Letters | From | End | strict′ | Runs | CPU s |
| --- | --- | --- | --- | --- | --- | --- |
| T6 step 1 | 86,866 | the stop | `1007a601…` | true | 1 | 1,447 |
| T6 step 2 | 449,078 | `1007a601…` | `5e9f262c…` | true | 1 | 9,911 |
| T6 step 3 | 103,230 | `5e9f262c…` | `b28448f3…` | true | 1 | 3,266 |
| T6 step 4 | 871,582 | `b28448f3…` | `c75c8dde…` | not replayed | 0 of 45 chunks | |
| rule stage 1 | 374,014 | the stop | `d9edc19c…` | not replayed | 0 of 19 chunks | |
| rule stage 2 | 224,512 | `d9edc19c…` | `ec6785c3…` | not replayed | 1 of 12 chunks | |
| rule stage 3 | 90,160 | `ec6785c3…` | `eea9b972…` | not replayed | 4 of 5 chunks | |

T6's steps 1 to 3 were each played in one run of `stepchk.py --require-strict` from the end table of the step before;
each run ends at the end of the composed gate. Machine restarts cut off single runs of the longer words, so the other
four were split into chunks of at most 20,000 letters, played several at once (`scripts/chunk_prep.py`,
`chunk_run.py` and `chunk_verify.py`, reviewed): the pose table at each chunk boundary is computed by exact
composition, each chunk is played twist by twist from its table, and a word counts as played only if its chunks
concatenate to the recorded step's full word, every chunk's twist-by-twist end digest equals the next chunk's start
digest and the last one's the recorded end; strict′ is then computed between the start table and the last chunk's end
table. The owner froze the research on 27 October 2026 while the chunks ran, and they were stopped after 5 of the 81
(each with exit 0, ending at its boundary digest). No word was completed, so these four words pass the first two gates
only. T6's step 4 was also to be cut at the end of W⁻¹ (446,588 letters, end `4d51ea67…`) to play the bridge of
Proposition B twist by twist; the bridge rests on Astra's exact composition.

### 3.4 Chains of shortened steps

Omitting the last span V′⁻¹ (Lemma TS) shortens a single step but changes its end (Corollary CH). The integrator played
T6's stored steps without V′⁻¹ as one chain (I3; X and W twist by twist, the stored V′ composed with its input digest
removed, W⁻¹ twist by twist): step 1 from the stop, 43,434 letters, strict′, end `170be703…`; then step 2 from there,
224,540 letters, lands its unit and keeps the other 104 off poses but takes one lattice piece off (129 → 105, strict′
false, end `ed53fbb5…`). Astra reproduced this by exact composition. The established totals are:

| Chain | Steps | Letters | Status |
| --- | --- | --- | --- |
| T6 as recorded | 4 | 1,510,756 | composed gate (section 2.2); steps 1 to 3 also twist by twist (section 3.3) |
| T6's steps 1 to 3, then step 4 without V′⁻¹ | 4 | 1,085,762 | composed (both analysts); steps 1 to 3 also twist by twist (section 3.3) |
| D1's 17 tail steps, then T6 | 21 | 1,908,036 | records |
| D1's 17 tail steps, then T6 with the last step shortened | 21 | 1,483,042 | records and the above (Proposition RP) |
| the rule's order (section 3.2) | 3 | 688,686 | built at the states reached; composed (the twist-by-twist replays stopped at the freeze) |

The sealed totals 771,052 (T6 with every step shortened) and 1,168,332 (D1 with that T6) were sums of steps evaluated
at the recorded starts and are withdrawn (Claude Opus).

### 3.5 The entry (computed and reported)

Whole caps alone stick at D1's entry, at E2's 737 state and at D1's table 00097 (Theorem N, section 3.1). Along D1's
recorded path the rule finds a complete static plan at every table from 00101 to 00113 (reported, Claude Opus):

| Table | Off | Plan stages | Tests |
| --- | --- | --- | --- |
| 00113 (the stop) | 158 | 3: (91,14), (217,15), (548,12) | 15 |
| 00112 to 00108 | 163 to 256 | 4: adds (416,14) | 4 to 28 |
| 00107 to 00105 | 297 to 400 | 5: adds (353,15) | 29 |
| 00104 to 00102 | 460 to 595 | 6: adds (570,15) | 42 |
| 00101 | 673 | 6: (91,14), (353,22), (217,15), (416,14), (548,12), (570,15) | 56 |
| 00098 to 00100 | 971, 860, 759 | not run | |
| 00097 | 1,082 | none (residue certificate) | |
| the entry | 1,193 | none (Theorem N) | |

So the rule "plan first, else the runner's rules" (H1, Claude Opus) completes from the entry statically if the runner
reproduces D1's first two to five tail steps (exact enumerations of at most 804 letters in all) and if the plan from
that table builds within 10⁷ at the states reached. Neither condition was tested: running the rule from the entry and
building every step at the state reached (DIS-3; Astra estimates 2 to 4 CPU hours of building, plus the replays) is
outside the freeze, so the entry membership of this rule stays **conditional** (Astra's ruling). What holds at the
entry is the recorded-path completion (Proposition RP) and the whole-cap negative (Theorem N).

### 3.6 Corrections

Applied to [card 9](../../theory/cards/card-9-tail-completion.md), which now reads as attacked with these corrections:

- **T6's replay (both; major).** T6 said every step was played twist by twist by `slp_apply.py --unit`; V′ and V′⁻¹
  were composed (section 3.3). Card 9's T6 and the docstring of `replay_tail.py` now say what was composed, and
  section 3.3 records the twist-by-twist replays (steps 1 to 3 played, step 4 stopped at the freeze). The second
  analyst first rated this a nit and said the composition also covered X; both points were corrected in the
  cross-examination.
- **T5's citation (Claude Opus; adopted by Astra).** T5 concluded that X and V′ exist "by COV-10, COV-12, COV-32 and
  Corollary A1", but `wholecap.py` certifies only the middle demand orbits, and Corollary A1 needs witnesses on every
  orbit that meets the target outside the fixed chambers. The conclusion holds by independently supported actions on
  the demand orbits (sealed seeds, R22B-08), which leave the orbits that meet the cap but not the landing chambers as
  they are, since their chambers in the cap are holes at the middle (Theorem SP(c)).

The analysts' own sealed claims were corrected as well:

- **Shortened totals (Claude Opus).** 771,052 and 1,168,332 are withdrawn (section 3.4).
- **Seed length (Astra).** The second analyst's claim that the construction bound is polynomial in the orbit size and
  that a slow preparation needs uncertified orbits is withdrawn: R22B-08's seed needs a small-support word and its
  bound has a binomial factor, so the bound is O(s + d³) given the seed length s (Lemma L). The anchor cost of R22B-31
  belongs to atlas seeds, with the target count as multiplier.
- **The rule (both).** The second analyst's sealed rule searched caps in index order and gave four stages from the
  stop; Astra's first experiment took the first order of a fixed triple with (560,13) third. The agreed enumeration
  (fewest stages, then lexicographic, over all eligible caps) gives (548,12) third.
- **Scopes (Astra).** Theorem N, Corollary N1 and Proposition CL are stated for their catalogues only; COV-14 stays a
  counterexample over valid states of open reachability; the extension of the static theory to mixed catalogues needs
  orbit-clean cores (Lemma OC); the shadow explanation of T1's fixed chambers at the cell-cycle middles stays an
  explanation (its finite check was not run).

### 3.7 The disputed points and how they were decided

After the cross-examination three points were open; they are Astra's DIS-1 to DIS-3.

| Point | Question | Experiment | Outcome |
| --- | --- | --- | --- |
| DIS-1 | Is the rule's static plan from the stop realized by bounded literal words, and do they pass the three gates? | Build (548,12) from the actual end of the first two stages, with exact length before expansion; then play all three words twist by twist from their actual inputs | **Decided for the first two gates** (section 3.2). Three literal steps, 688,686 letters, strict′ at every stage, profiles as predicted; the twist-by-twist replays were stopped at the owner's freeze (section 3.3), so the third gate is open |
| DIS-2 | Does the joint triple meet the literal limit? | Seed and table preflight at the joint middle, then the joint word | **Not run (optional).** DIS-1 completes the stop, and no saving of the joint step is established (Proposition JT); its existence stays conditional and its length conjectured |
| DIS-3 | Does a deterministic mixed rule (plan first, else the runner) complete from D1's entry? | Run the rule from the entry with a deterministic builder, building every step at the state reached | **Not run (outside the freeze).** By Astra's ruling the entry membership of that rule is recorded as unverified; the recorded-path theorem (Proposition RP) and the whole-cap negative (Theorem N) stand |

### 3.8 Answers to the card's questions

- **1.1 A construction with a length bound: answered.** Proved: X and V′ are products of sealed three-cycles, with
  |X| + 2 + |V′| ≤ 2Σ m_A c_A + 2 + 2Σ |D ∩ Δ| c_Δ in the three-segment form, linear in the holes moved and the pieces
  landed and polynomial in the orbit size given the seed lengths (Lemma L), in place of COV-10's product of binomials;
  the generic numbers do not decide 10⁷, the measured tables do where they apply (T6's step 4: at most 7,001,082, or
  3,511,338 three-segment). Computed: the three steps of section 3.2 within 10⁷ (the longest 374,014). Open: a bound on
  the seed length from the state alone; with certified orbits no counterexample shape of the kind the card asked for is
  known.
- **1.2 Order, contract and a joint step: answered.** Proved: the profile identity makes every later instance a finite
  test at the stop (Theorem SP), so no first step can destroy a later instance of a passing plan; every tested order
  passes, 91 first and 560 first included. A stored word does not survive a changed earlier step (Corollary CH), which
  is a matter of words, not resources. A joint step exists for separated caps of different classes: the triple lands
  all 158 pieces at once (Proposition JT); it was not built.
- **1.3 Other catalogue instances: answered in part.** (353,22) passes unprepared; (148,15), (217,15), (548,12),
  (560,13) and (91,14) pass prepared; caps 12, 324 and 439 are excluded under every preparation; no closure with T1's
  cores lands a unit (Proposition CL). Open: prepared T1′, partner T2′ and 112734's own T1′ after 81223.
- **2.1 Fixed chambers and a criterion: answered in part.** The fixed chambers of the glued cells' targets lie only in
  caps that the glued contents and their partners hold or straddle (Lemma FO, reported); the whole-cap step removes
  both shadow sources at once, and its landing chambers lie outside the cells and in caps that are SAFE at the middle.
  The criterion is a finite bracket (Proposition CB). Open: an exact test cheaper than COV-10's exhaustive one; the
  cell-cycle middle explanation stays unchecked.
- **2.2 Is every glued unit landed by turning a cap of its cell? No.** At D1's entry, a reachable state of 𝒞₂, 112734
  has no whole-cap step at caps 217 or 324 under any preparation while R is off (Corollary N1). Whole caps do not
  vacate a shared cell; they move the content off its cell, which needs the whole cap vacant.
- **3.1 An order that does not stick: answered in part.** Proved: the rule WC-PLAN never sticks on its plan class when
  the admitted words exist (Theorem R), and at the stop it completes in three literal steps that pass the first two
  gates (section 3.2; the twist-by-twist replays were stopped at the freeze). Proved
  negative: no order of whole caps alone completes from D1's entry (Theorem N), so T6's rule sticks there; D1's recorded
  runner steps followed by whole caps complete it (Proposition RP). Conditional: the mixed rule H1 from the entry
  (section 3.5). Open: a rule with a proof on a class of states that contains D1's entry.
- **3.2 The rule stated for a program: answered** (PROOFS.md section 3: what it reads, the gate, the candidates in
  their order, the test, the word and its tree, the admission and the contract).
- **3.3 Cover along a run: answered.** Cover is not inductive (Corollary N2); COV-15's theorem applies along a run only
  if Cover holds at every state reached; a complete static plan is a checkable inductive replacement.
- **4.1 Bounds per kind of step: answered.** Lemma L and Lemma TS give the bound of each kind (closure, T1′, prepared
  T1′, whole cap, joint whole cap), with the trailing inverse unnecessary for a single step. Every recorded step is
  within 10⁷; bounds below 10⁷ are proved for E2's nine certificate steps (R22B-31) and T6's step 4 only.
- **4.2 A deterministic construction: answered in part.** Breadth-first Schreier tables (R22B-05), sealed seeds from
  Jordan words found with a fixed seed and no clock (R22B-08), fixed orders and letter budgets in place of time budgets
  give a construction that depends on the state only; R22B-07 decides whether a seed exists. Its practical lengths were
  not measured, and its bound depends on the seed lengths.

## 4. Files

- `PROOFS.md`: the results of the joint attack with proofs.
- `scripts/wholecap.py`, `wc_prep.py`, `wc_t1.py`, `replay_tail.py` and its gate `test_replay_tail.py` (research
  scripts; each docstring gives its use). `replay_tail.py` reads a record in the four-segment form (T6) or the
  three-segment form (the rule's chain). `chunk_prep.py`, `chunk_run.py` and `chunk_verify.py` play a recorded step
  twist by twist in chunks whose digests chain (section 3.3).
- `results/stop-units/`: the five units of the stop (`mkblocks2.py` on the stop table), named by key.
- `results/wholecap/`: the whole-cap outputs at the stop, at 129 and 104 off, and at E2's 737 and 71 states.
- `results/d1-tail/`: T6's four steps (`x.json.gz`, `w.json`, `v.json.gz`, `winv.json`, `unit.json` each) and
  `steps.json`.
- `results/rule-stop/`: the rule's three stages from the stop in the same files, and `steps.json` with
  `"form": "three-segment"`.
- `results/lattice-d1/`: `run/`, the runner record of the lattice phase from T6's end (without the restorer's scratch
  `attempts/`, as in the committed development runs), `runner.out` and `check.jsonl` (`check_run.py`: verdict
  `solved`, no fail).

## 5. Reproduction

From the repository root, single-threaded (`OPENBLAS_NUM_THREADS=1 OMP_NUM_THREADS=1 MKL_NUM_THREADS=1`), with
`Z=research/jumbling/restoration/13-tutorial-runner/runs/dev/d1-acc-s7101/out4/tables/00113.json` and
`S=research/jumbling/restoration/18-tail-completion`, every output outside the repository:

- T5: `python3 -I -B -u $S/scripts/wholecap.py $Z $S/results/stop-units/*.json` (about 2.5 CPU minutes);
- T6: `python3 -I -B $S/scripts/replay_tail.py $Z $S/results/d1-tail <new dir>` (about 10 CPU minutes), and its
  gate `python3 -I -B $S/scripts/test_replay_tail.py $Z $S/results/d1-tail <new dir> --full` (about 19 CPU
  minutes);
- the rule's chain: `python3 -I -B $S/scripts/replay_tail.py $Z $S/results/rule-stop <new dir>` (about 27 CPU
  minutes, the composed gates of section 3.2), and the same gate on `$S/results/rule-stop` (about 55 CPU minutes);
- a twist-by-twist replay of a step in chunks: `python3 -I -B $S/scripts/chunk_prep.py <start table> <step dir> <4|3>
  20000 <new dir> <recorded end digest>` (4 for T6's steps, 3 for the rule's; the start table is `$Z` for a first step
  and otherwise the previous step's end table, `<new dir>/<step>.json` of `replay_tail.py`), then
  `python3 -I -B $S/scripts/chunk_run.py <jobs> <new dir> ...` (rerun it after an interruption; it skips finished
  chunks) and `python3 -I -B $S/scripts/chunk_verify.py <new dir> ...`, which prints `"all_ok": true` when every
  chunk chains to the recorded end and the step is strict′ (17 to 32 ms per letter of one core); a short step can
  also be played in one run of `12-low-height/scripts/round22/stepchk.py <start table> <unit.json> <word file> <new
  report> <new end table> --require-strict` on its full word in one file `{"word": [...]}`;
- the lattice phase: `python3 -I -B research/jumbling/restoration/13-tutorial-runner/scripts/runner.py --table <new
  dir>/04-91-14.json --out <new run dir>` (about 50 minutes), then `check_run.py <new run dir>`.
