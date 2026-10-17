# 11. Scramble census: the state-only descent on unrestricted random scrambles (17 October 2026)

On 17 October 2026 the owner made a tutorial that people can read and carry out the most important output of the
restoration research, ruled out scrambles restricted to what the method covers, and asked for scramble experiments
before choosing a research option ([decision](../../../../docs/wiki/decisions/owner-decisions-2026-10-17-jumbling-tutorial.md)).
This stage holds those experiments, the integrator's assessment that the owner chose from, and the plan that
followed: a measured coverage census with a manual of the techniques the method uses (option A), and a completeness
theorem for low heights (option B), both provisional.

The question: from a fresh random scramble, how far does the existing state-only procedure get, where does it stop,
and what is left? This is the first part of gap E1 (coverage) of the [restoration research](../README.md).

Labels: **certificate** (an exact computation with a replayable record), **measured** (a result on finite inputs).

## Setup

- **Scramble.** Menu I_a. A scramble of depth k is k admissible jumble twists at uniformly random grips, each with a
  uniformly random jumble item of the grip's menu; a draw that is not admissible is skipped and drawn again
  (`theory/restore.py`, `scramble`, with no retained twists). One seed per depth at first, two at depth 40.
  Nothing restricts the scramble.
- **Descent.** Phase U of the earlier theory (`theory/seams.py`, `SeamRestorer`, default rule order seam, cover,
  close, conj, lock, improve, shift, reopen; [restoration](../../theory/restoration.md) section 3). Its rules read only
  the configuration; the run keeps the scramble's history only to label its record. Every kept step lowers the total
  misalignment N, the number of cut cells covered on one side of a cut and not on the other by the same domain, and
  N = 0 exactly on the lattice (M3). The three rules that did all the work here:
  - **seam** (face match): one twist of a cap that aligns it on every misaligned cut cell inside it;
  - **cover**: one twist of a cap that covers a misaligned cap and is covered by none;
  - **conj** (three-twist repair): open a cap c by any twist, make the best single twist of a misaligned cap d that
    meets c, then the best single twist of c; kept if N drops.
- **What is not run here.** The same procedure with the same rule order produced stop62 (stage 05): 62 twists from
  the I_a@40 fixture, a snapshot taken while it was still descending (`theory/results/seams-I_a-40-stop.json`). The
  general method of stage 09 (clean steps, with the exhaustive search as the fallback of Theorem M\*) and the lattice
  phase (Theorem R) were not run from the stops of this census.
- **Budget.** Each round gives a run 75 minutes of descent, checked between steps, so a step under way finishes; a
  process is killed after 110 minutes. A later round replays the moves of the earlier rounds and continues. A *time
  limit* is therefore not a stop of the method. Runs used `ulimit -v 7000000`; times are cloud timings of research
  code, not performance evidence.

## Results

### Round 1 (75 minutes each)

| Depth k | Seed | Off after the scramble | Left heights at the scramble | End | Twists | Steps: seam / cover / conj | Off at the end | N: start → end |
| ---: | ---: | ---: | --- | --- | ---: | --- | ---: | --- |
| 10 | 7001 | 26,519 | all 2 | **lattice, solved** | 9 | 9 / 0 / 0 | 0 | 13,270 → 0 |
| 20 | 7106 | 51,828 | all 2 | **lattice** | 19 | 19 / 0 / 0 | 0 | 26,942 → 0 |
| 40 | 7101 | 90,441 | all 2 | time limit | 199 | 27 / 7 / 55 | 1,315 | 50,344 → 1,866 |
| 40 | 7107 | 97,491 | all 2 | **lattice, solved** | 36 | 36 / 0 / 0 | 0 | 50,254 → 0 |
| 60 | 7104 | 106,075 | all 2 | time limit | 237 | 15 / 15 / 69 | 17,997 | 63,814 → 17,498 |
| 100 | 7102 | 115,566 | 2: 115,485; 4: 81 | time limit | 177 | 5 / 19 / 51 | 34,258 | 73,454 → 34,674 |
| 1000 | 7103 | 125,304 | 2: 123,685; 4: 1,484; 6: 135 | time limit | 491 | 1 / 28 / 154 | 87,067 | 123,654 → 91,682 |

The 10-twist run and the second 40-twist run end at the solved state itself, by face matches alone; seed 7107 took
152 seconds. The 20-twist run ends on the lattice, not solved; its invariant lies in v(G), as expected, and the
lattice phase restores it (Theorem R; not run here).

### Round 2 (the four deep runs, another 75 minutes each)

| Depth k | Off: start → end | N: start → end | Twists in all | New steps | Left heights at the end | End |
| ---: | --- | --- | ---: | --- | --- | --- |
| 40 | 1,315 → 1,193 | 1,866 → 1,734 | 223 | 8 conj | all 2 | killed at 110 minutes, 49 minutes into a search |
| 60 | 17,997 → 14,610 | 17,498 → 15,334 | 291 | 18 conj | all 2 | time limit |
| 100 | 34,258 → 31,479 | 34,674 → 32,642 | 272 | 2 cover, 31 conj | 2: 31,355; 4: 124 | time limit |
| 1000 | 87,067 → 81,309 | 91,682 → 87,820 | 625 | 5 cover, 43 conj | 2: 80,919; 4: 390 | time limit |

After two rounds the descent has put back 98.7%, 86.2%, 72.8% and 35.1% of the off pieces of the 40- (seed 7101),
60-, 100- and 1000-twist scrambles. **Certificate**: `scripts/check_records.py` rebuilds every scramble from its
seed and replays every recorded move on the exact simulator. Every twist is admissible and N drops at every step;
the off count and N after the scramble, at the start of each round (which must fall between two steps) and after
every step, the heights and off poses at the start and at the end, the twist count, the result and, for the runs
that reached the lattice, the solved flag and the invariant v agree with the records (`runs/check.out`, with the
end digests). Records changed on purpose (a move, an N, a start count, the twist count, the round boundary, the
result, the solved flag, the start heights or off poses) fail it (`scripts/test_check_records.py`).

### What the runs show (measured)

1. **No run got stuck.** Every run that did not reach the lattice was stopped by the clock; every step it took
   lowered N.
2. **Three techniques did all the work**: seam, cover and conj. Close, lock, improve, shift and reopen never found
   the next step first.
3. **A fast bulk, then a slow tail.**
   - The bulk is single-twist steps (seam and cover). At depths 10 to 60 the median such step lowers N by 1,105 to
     1,566; at depth 100 by about 550, at depth 1000 by about 55.
   - The deeper the scramble, the sooner the single-twist steps run out: the first conj step came after 29 steps
     (when 25,783 pieces were off) for depth 40, after 27 (53,647) for 60, after 9 (101,481) for 100 and after 3
     (121,636) for 1000.
   - In the tail almost every step is a conj step. In round 1 the median conj step lowered N by 50 to 146 after a
     search of 20 to 52 seconds; in round 2 by 2 to 50 after 91 to 269 seconds. One search took 25 minutes, and the
     last search of the depth 40 run had found nothing after 49 minutes when the process was killed.
   - The twists are few: 199 twists put back 98.5% of the 40-twist scramble. What is slow is finding the next step.
     For a person this is the work of seeing which cap matches and which three twists repair, which is what the
     tutorial and the program's help have to teach and show.
4. **One seed per depth is not enough.** The two 40-twist scrambles have about the same number of off pieces, but
   seed 7107 is solved by face matches in under three minutes while seed 7101 runs into the tail.
5. **Heights stay low.** After 1000 random twists no piece is above left height 6. The descent cleared all pieces at
   height 6, but heights do not only fall: in round 2 the pieces at height 4 rose from 81 to 124 (depth 100) and from
   264 to 390 (depth 1000). N is the potential, not the height.

## The residue at the end of round 2

`scripts/residue.py` rebuilds the end state from the seed and the moves (no journal), exports its pose table and
lists its restoration units with stage 09's `mkblocks2.py`. A *rigid block* is a component of "no cut separates";
its *movers* are the grips whose caps hold it whole and whose twists are admissible now
(`residue/*-summary.json`, `residue/*-blocks.json`). **Measured.**

| | Depth 60 | Depth 100 |
| --- | ---: | ---: |
| Off pieces | 14,610 | 31,479 |
| Rigid blocks (units) | 184 (184) | 437 (439) |
| Blocks of 111, 86 and 87 pieces | 53, 44 and 15 blocks: 10,972 pieces (75%) | 98, 91 and 29: 21,227 pieces (67%) |
| Blocks without a mover (every cap that holds them whole is blocked) | 20 blocks, 2,156 pieces (15%) | 88 blocks, 8,946 pieces (28%) |
| Blocks in several frames | 0 | 2 (121 pieces each) |
| Largest blocks | 385, 385 | 385 (three), 375, 365 |
| Left heights | all 2 | 2: 31,355; 4: 124, in two blocks: 81 pieces all at height 4, and one of the two-frame blocks (78 at height 2, 43 at height 4) |

111, 86 and 87 are the sizes of the three shapes for which stage 06 stored template words, carried to any position by
K⁺ (Lemma TT of stage 06). Equal size does not mean equal shape, and stage 06 works with a coarser grouping, so the
template test computes both:

- **Mov groups** (stage 05 `blocks.py`): the pieces with the same set of admissible movers, which move together under
  every admissible twist now. Rigid blocks whose separating cuts are all blocked fall into one group, and every piece
  without a mover falls into a single frozen group. Depth 60: 117 groups, 7 of them with more than one pose (3,358
  pieces, the frozen group of 2,156 included). Depth 100: 173 groups, 42 with more than one pose (18,932 pieces, the
  frozen group of 8,946 included).
- **Exact shape classes** (stage 06 `census.py`: the K⁺ class of a single-pose group; a group with several poses
  gets one key made from the classes of its parts): depth 60 has 57 distinct keys (51 among its single-pose groups),
  depth 100 has 123 (82). The groups of the three template classes (da4b45, 111 pieces; fca5f0, 86; abd791, 87), all
  single-pose, hold 3,909 off pieces at depth 60 (27%: 22, 12 and 5 groups) and 1,652 at depth 100 (5%: 11, 4 and 1
  groups) (`templates/groups.jsonl`, by `scripts/groups.py`).

So most of the residue lies in rigid blocks of the three catalogued sizes, but at depth 100 most of it moves in
larger groups or sits in frozen blocks, outside the catalogue as it stands.

### Template test

`scripts/run_fx.sh` runs stage 06's transport run `fxrun.py` on each residue with the ten stored templates
(`scripts/templates.py` unpacks them). For every group of a template shape it carries the template's words by the
exact K⁺ element, scores them on the current state by the exact ghost, and replays a clean word: the group on the
lattice, collateral 0, every other off piece at its exact pose, and Bad(W), the chambers the word needs, already
vacant. It stops when a full scoring pass finds no clean word (`templates/`; summary by `scripts/fxsummary.py`).

| | Depth 60 | Depth 100 |
| --- | --- | --- |
| Groups of a template shape, carried words | 39, 2,350 words | 16, 905 words |
| Restored, each by a clean 10-twist word | 2 groups of 111 pieces: 222 pieces, 1.5% of the off pieces | 1 group of 111: 0.4% |
| What stops the best word of each other group | chambers of Bad(W) occupied 14, other off pieces disturbed 16, grips blocked 7 | occupied 8, blocked 5, disturbed 2 |

This agrees with stage 06 at the I_a@40 fixture, where the same words restored 7 of 140 blocks (1.2% of the off
pieces). Without a vacating word the stored catalogue clears almost nothing of a residue: its words need chambers
that other pieces hold, or they move other groups. For the rest a clean step needs a vacating word V, as in
V·W·V⁻¹ (Lemma V of stage 06, the clean steps of stages 08 and 09), and finding V is the search whose length gap M1
asks to bound. **Certificate** for the three steps: `scripts/check_steps.py` rebuilds the residue from the census
record and replays them on the exact simulator, without the code that found them: every letter admissible, the group
on the lattice, no lattice piece taken off, every other off piece at its exact pose, and the recorded digests
(`templates/check.out`).

## Assessment and the owner's choice

The integrator proposed four options before the experiments (decision item 1) and assessed them against the results
after each round:
- **A. Measured coverage census and a manual of techniques.** The evidence supports it: three techniques, a few
  hundred twists, no run stuck. The manual would teach the bulk (face match, cover, three-twist repair) and, for the
  tail, a catalogue of block shapes with their template words and how to unlock frozen blocks; coverage is what the
  catalogue clears. The template test, run after the owner's choice, shows that the words alone clear little of a
  tail: the manual also has to teach how to vacate the chambers a word needs.
- **B. A completeness theorem for low heights.** Random scrambles stay at left heights 2 to 6, and the residue is at
  heights 2 and 4. After round 2 the integrator narrowed B to residues at heights 2 and 4: the difficulty there is
  blocks that hold one another up, not height.
- **C. Scrambles restricted to what the method covers.** Rejected by the owner (decision item 2).
- **D. The strong form of gap M1.** The least help to a tutorial.

The owner chose A and B, provisionally and open to change (decision item 4). D is not pursued for now.

One correction to the integrator's round 2 report to the owner: it said that about 70% of the off pieces belong to
the three catalogued shapes. That is the share by block size. By exact shape class, the groups that the stored
templates can address hold 27% (depth 60) and 5% (depth 100).

## Next

- **A.**
  1. The general method from a stop, with vacating words: the clean steps of stage 09. The depth 40 stop (1,193 off
     pieces) is the size of stop62, where the state-only driver v4 went from 860 to 269 off pieces in 8 steps
     (stage 09, section 6.2).
  2. More seeds and depths: second seeds at 100 (7108) and 1000 (7110), more at 40, and depths 200 (7105) and 400
     (7109). The runs at 200, 100 (7108) and 400 started on 17 October; their records come with the next update.
  3. A manual of the techniques, written for a person: the three descent techniques, the shape catalogue, vacating
     and unlocking, each with its picture in the program. Started on 17 October: [manual](../../manual/README.md),
     with chapters 1 and 2 (what the solver sees, the descent with worked examples from these records) drafted and
     chapters 3 and 4 as outlines.
- **B.** A statement: at every reachable state whose off pieces lie at left heights 2 and 4, a clean step V·W·V⁻¹
  exists whose W comes from a finite catalogue and whose vacating word V has an explicit length bound, or an
  unlocking step of explicit length; that is, gap M1 restricted to low heights. The template test puts the weight
  on V. A plan check comes before any joint attack.
- **The lattice phase**, written for a person (commutators, three-cycles, correction macros), separately.

## Replay

From the repository root, with Python 3 and NumPy, `S=research/jumbling/restoration/11-scramble-census/scripts`
(cloud times):

    # a fresh run, then a second round from its record (75-minute budget per round, 110-minute hard limit)
    $S/run_jobs.sh <out1> 40:7101 60:7104
    $S/run_jobs.sh <out2> 40:7101:<out1>/ia-k40-s7101.json 60:7104:<out1>/ia-k60-s7104.json
    # the tables of this page (one JSON line per record)
    python3 -I -B $S/table.py research/jumbling/restoration/11-scramble-census/runs/round1
    # replay check of every record from its seed (about 50 minutes)
    python3 -I -B $S/check_records.py research/jumbling/restoration/11-scramble-census/runs/round*/ia-k*.json
    # its mutation test: eight changed copies of the depth 10 record must fail, the record must pass (about 1 minute)
    python3 -I -B $S/test_check_records.py
    # the residue of a record: pose table, units, summary (about 10 minutes at depth 100)
    python3 -I -B $S/residue.py research/jumbling/restoration/11-scramble-census/runs/round2/ia-k60-s7104.json <out>
    # the template test on that residue (about 15 minutes), its summary, and the replay of its steps
    python3 -I -B $S/templates.py <tmpl>
    $S/run_fx.sh k60 <out>/ia-k60-s7104-stop-table.json <fx> <tmpl>/templates.json
    python3 -I -B $S/groups.py <fx>/k60-blocks.json <fx>/k60-census.json <tmpl>/templates.json
    python3 -I -B $S/fxsummary.py <fx>/k60-fx/run.jsonl
    python3 -I -B $S/check_steps.py research/jumbling/restoration/11-scramble-census/runs/round2/ia-k60-s7104.json \
        research/jumbling/restoration/11-scramble-census/templates/k60-step0*-word.json

A rerun stops wherever its clock runs out, so its record can differ from the stored one; `check_records.py` checks
the stored records exactly. Every script reads `research/jumbling` (the simulator `sim/`, the theory modules and the
scripts and data of stages 05, 06 and 09) without writing to it. The runs used that code as it is on main at commit
`5eed80e`, where it last changed on 15 October; this stage changes no file outside its own directory.

## Files

| Path | What it is |
| --- | --- |
| `scripts/census_seams.py` | One census run: scramble, phase U with a clock, a record after every step |
| `scripts/run_jobs.sh` | Runs census jobs in parallel, fresh or continued |
| `scripts/table.py` | The tables of this page from a directory of records |
| `scripts/check_records.py`, `scripts/test_check_records.py` | Replay check of records from their seeds, and its mutation test |
| `scripts/residue.py` | The residue of a record: pose table, units, block sizes, movers and heights |
| `scripts/templates.py`, `scripts/run_fx.sh` | The template test with the tools of stages 05 and 06 |
| `scripts/groups.py` | Mov groups and shape classes of a residue, from the stage 05 and 06 outputs of `run_fx.sh` |
| `scripts/fxsummary.py`, `scripts/check_steps.py` | Summary of a template test, and an independent replay of its steps |
| `runs/round1/`, `runs/round2/` | The run records (scramble history, moves, steps) and `table.jsonl` |
| `runs/check.out` | Output of `check_records.py` on all eleven records, in the order of the command above |
| `residue/` | Summaries and per-block rows of the depth 60 and 100 residues |
| `templates/` | The template test: `fxrun.py` logs, the step words, `groups.jsonl`, `summary.jsonl` and `check.out` |

## Limits

- One or two seeds per depth, and a time limit per round: the census measures how far phase U gets in a given time,
  not where it would stop. The two seeds at depth 40 already behave very differently.
- Phase U only. Whether the general method finishes from these stops, and at what cost, is not measured yet.
- The scramble has no retained twists. A product scramble may mix them in; they move pieces but do not change the
  heights.
- The block notions (rigid blocks, Mov groups) and the shape classes are those of stages 05, 06 and 09; the shares
  above are measured on two residues.
- Everything here is exact computation on synthetic geometry in headless Linux runs. Times are cloud timings, not
  performance evidence. Not run: Windows, DirectX, the native host regression, input, long sessions and
  performance measurements.
