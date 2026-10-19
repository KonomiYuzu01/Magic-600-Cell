# 13. The tutorial runner: a machine that follows the rules (19 October 2026)

On 18 October 2026 the owner set the tutorial's acceptance test: a machine that follows it step by step, using only
its rules, takes a scramble to the solved state, playing only literal words, twist by twist; a step known only by a
certificate counts as stuck, and every step's length is recorded as it is
([decision](../../../../docs/wiki/decisions/owner-decisions-2026-10-18-machine-tutorial.md)). This stage holds that
machine and its checker:

- `scripts/runner.py`: one state-only program that applies the rules the tutorial teaches, in a fixed order, and
  writes one record of every word it plays;
- `scripts/check_run.py`: an independent checker that replays a record twist by twist on the exact simulator, without
  the runner or its search code; the lattice phase it checks through `verify_late.py`, which composes the
  permutations of the lattice word's retained part instead of playing it (section 5).

Both were written by Codex under two packets (RUN-1, the runner; RUN-2, the checker, written in parallel from the
shared record format) and reviewed by the integrator. Research code, headless Linux; nothing here is Windows, DirectX
or performance evidence.

Labels: **certificate** (an exact computation with a replayable record), **measured** (a result on finite inputs).

## 1. The rules the runner follows

The runner reads only the current configuration. It uses the scramble's history only to build the start state, and
it breaks every tie by fixed keys, with no randomness.

1. **Start**: `--scramble <k> <seed>` (the census scramble of stage 11) or `--table <pose table>`.
2. **Bulk phase**: the census loop of stage 11 (`theory/seams.py`, rule order seam, cover, close, conj, lock,
   improve, shift, reopen): at each step the first rule that returns twists; each step is one word. The phase ends
   on the lattice, when no rule fires, or at a step or time budget; a step cut off by the clock is not recorded.
3. **Tail phase**, at each state: the units of `mkblocks2.py`, largest first, then by key; for each unit the tail
   rules in order: `zero_bad` (a transport without endpoint collateral), `pure` (the class-cell pure twist of
   `cellpure5.py --search`), `close` (V·W·V⁻¹ with V from `tclose2.py`), `t1prime` (W·V′·W⁻¹·V′⁻¹ with V′ literal
   from `tclose2.py`). A rule gives a step only when `stepchk.py` reports the whole literal word strict′ at the
   state; the first unit in order with a step takes it, and the runner replays the word on its own copy of the state
   and asserts the same end digest. With `--par K` several units are tried at once and their results are taken in
   unit order, so the step taken is the one a sequential run takes as long as no attempt is cut off by a time limit:
   on a loaded machine an earlier unit's search can time out with `--par K` and succeed with `--par 1`, and the two
   runs then take different steps, both checked (review finding RV1-03).
4. **Fallback**: when no unit has a step, the exhaustive search `fallback.py` for the first unit within
   `--fallback-max-length` letters; otherwise the run ends **stuck**, with every unit's attempts in the record.
5. **Lattice phase**: when every piece is on the lattice and the state is not solved, the lattice restorer
   `restore_lattice_late.py` and then its independent checker `verify_late.py`; the run is **solved** only when the
   checker passes. The lattice word is literal, but `verify_late.py` evaluates its retained part as an exact
   permutation program on the sticker labels and plays only the corrections, so this phase is checked exactly but
   not played twist by twist (stage 12, `PROOFS-B.md` R22D-57).
6. The runner plays only literal words, except that the retained part of the lattice word is checked by composition
   (rule 5). It never calls the certificate builder of round 22 (`r22b10_cert.py`) or
   the straight-line program applier (`slp_apply.py`); `tclose2.py` may build V from its peeling certificate
   (Theorem B), but it returns V as a literal word, and only the whole literal word, checked by `stepchk.py`, is
   played. A closure or T1′ attempt that fails records N1, N2 and the deficits it read, and its outcome.

## 2. The record

A run directory holds `run.json` (format `runner-run/1`, rewritten atomically after every step), one word file per
step (`{"word": [[grip, item], ...], "input_digest", "output_digest"}`), one unit file per tail step, the child logs
and the lattice phase output. Each step records its phase, rule, unit, word file, length in letters, contract
(`strict_prime` for tail steps; `fallback` for the fallback: the unit lands, no lattice piece goes off and the off set
shrinks), the digest and off count before and after, the left heights of the off pieces and its time. A stuck run
records the state and every unit's attempts. The result is `solved`, `stuck`, `budget` or `error`, and the totals give
the twists of the bulk, the tail, the fallback and the lattice phase. A `--table` start is stored as
`start-table.json`, exported again from the loaded state (the same digest, not necessarily the same bytes).

Every path in `run.json` and the word files is relative to the run directory. The lattice phase's own outputs
(`lattice/restore-summary.json`, `lattice/report.json` and the solutions file) and the child logs name the run
directory by its absolute path, so a run is checked where it was made, and those files are scrubbed before they are
published (review findings RV1-02 and RV2-02).

## 3. The checker

`check_run.py` reads a record and the files it names, rebuilds the start (the census scramble, or the stored table),
plays every word twist by twist on the exact simulator, and checks for each step the digests before and after, the
off counts, the heights, the length and the step's contract (strict′: the unit lands, no lattice piece goes off and
every other off piece keeps its exact pose or lands; Lemma U for the fallback), that every word is literal, that the
record has no absolute path, and the totals. For a solved run it runs `verify_late.py` on the lattice phase output,
which checks the lattice word by composing permutations, not by playing it (rule 5).
It checks the words played, not the runner's choice of rule or its budgets. A fallback step whose word is empty
passes when its contract holds, as the record format allows (review finding RV2-05).

## 4. Tests

- `scripts/test_runner.py` (RUN-1's acceptance check): 16 tests, one of them skipped (the depth 20 run to the solved
  state, which needs `RUNNER_SLOW=1`), passed in 885 s on 19 October 2026 with other long jobs on the machine
  (898 s with the worktree check): the depth 10 scramble of seed 7001 solved by the bulk alone in the census's
  9 twists (36 s); a small lattice configuration solved by the lattice phase (579 s); the stored T1′ step of E2
  for b04 at the state of 437, played literally with 1,254 letters to the recorded end digest (262 s); and 12
  record tests with stubbed rules
- `scripts/test_check_run.py` (RUN-2's acceptance check): 27 tests passed in 700 s on 19 October 2026: replays of a
  bulk run, a strict′ tail step (74 s), a lattice run and a fallback with its lattice chain (15 s each), and negative
  tests that change one letter, path, step, length, height, total, landed count, digest or lattice report of a valid
  record and require the failure that names it
- Two records made with these scripts on 19 October 2026 and passed by `check_run.py` (`runs/`, measured): the depth
  10 scramble of seed 7001, solved by the bulk phase in 9 twists (38 s); and the depth 20 scramble of seed 7106, which
  the bulk phase brings onto the lattice in 19 twists, all of rule `seam`, and the lattice phase restores in 8,701,310
  twists, passed by `verify_late.py` in the runner and again in the checker (654 s in all), which checks the
  lattice word by composition and does not play it twist by twist (rule 5). Neither needed a tail step. The start and end tables, the copy of the lattice library (the same sha256 as
  `09-general-method/data/library-merged.json.gz`) and the restorer's working files are left out: `runner.py
  --scramble` makes them again, and `run.json` holds their digests. In the lattice outputs and logs the run directory
  is written `<run>`.

## 5. What it does not do yet

- The nine steps of experiment E2 known only by certificates (stage 12, section 11) are not in its rules, so on the
  depth 40 stop of seed 7101 it would stop where E2 needed them. Round 6 of line (b) gave each of them a literal word
  of fewer than 10⁷ twists (stage 12, `PROOFS-B.md` R22B-31), and the literal builder of round 6 expands such a
  step into one literal word (`round22/replay_slp.py --seeds atlas --literal-limit N`, R22B-34); a rule that does
  this at the current state is the next addition.
- Height 4: the runner has only the rules above. Round 2 of line (d) gave a state-only rule with words of at most 244
  letters (`PROOFS-B.md` R22D-19), round 3 a rule for a whole stop at heights 2 and 4 (R22D-25), and round 4 a
  height-6 tier for it (R22D-37) and an implementation contract (R22D-50 to R22D-56); none of them is in the runner
  yet.
- The lattice phase is not played twist by twist: `verify_late.py` evaluates the retained factors of the lattice word
  through their net sticker permutations (lines 344 to 348) and plays only the corrections on the simulator (lines
  351 to 374), and `runner.py` takes solved from its report (lines 645 to 650). The owner's acceptance test asks for
  literal play, so a literal lattice player is needed: literal expansion and play of the retained factors and the
  corrections, with played lengths, input and output digests and the final solved digest (`PROOFS-B.md` R22D-56,
  R22D-57). In the sticker-permutation model one twist takes about 21 µs on the cloud machine, so the 166,777,936
  retained twists of E2's lattice word would take about one CPU hour (an estimate, not run).
- It decides nothing about coverage: a solved run shows that the rules reached the solved state from that start.
- Review findings left for the next runner packet: a runner stopped by a signal can leave its children running
  (RV1-01); the lattice outputs name the run directory by its absolute path (RV1-02, RV2-02); `advance()` and the bulk
  time-out path have no test with real mechanics, and several checker failures have no negative test (RV1-04,
  RV2-03); `config.scripts` hashes 16 of the files the steps depend on (RV1-06); an `--out` path containing `.json`
  breaks the pure rule's word file (RV1-07); `verify_late.py` runs in the checker without a time limit or a saved
  output (RV2-06).

## Replay

From the repository root, with Python 3 and NumPy, `S=research/jumbling/restoration/13-tutorial-runner/scripts`:

    python3 -I -B $S/test_runner.py                       # about 15 minutes
    python3 -I -B $S/test_check_run.py                    # about 12 minutes
    python3 -I -B $S/runner.py --scramble 10 7001 --out <new dir> --cache <new dir>
    python3 -I -B $S/runner.py --scramble 20 7106 --out <new dir> --cache <new dir>   # about 11 minutes
    python3 -I -B $S/check_run.py <run dir>

## Files

| Path | What it is |
| --- | --- |
| `scripts/runner.py` | The tutorial runner (RUN-1) |
| `scripts/test_runner.py` | Its acceptance tests |
| `scripts/check_run.py` | The independent record checker (RUN-2), with two fixes by the integrator after review: every string in the record is scanned for an absolute, home-relative, Windows or parent path (RV2-01), and a corrupt gzip body fails as the file it is in (RV2-04) |
| `scripts/test_check_run.py` | Its acceptance tests; the timeout of the lattice fixture was raised from 1,100 to 3,600 seconds by the integrator after the first acceptance run timed out on a loaded machine |
| `runs/depth10-seed7001/`, `runs/depth20-seed7106/` | The two records of section 4: `run.json`, the words, the lattice phase's summary, report and solutions (gzipped), the logs, the runner's last line (`runner.json`) and the checker's output (`check.jsonl`) |

## Checks not run

- Windows, DirectX, the native host, input, long-session and performance checks: none; timings are cloud timings of
  research code with other jobs on the machine.
