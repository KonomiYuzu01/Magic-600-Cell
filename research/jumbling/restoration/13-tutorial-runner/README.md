# 13. The tutorial runner: a machine that follows the rules (19 and 20 October 2026)

On 18 October 2026 the owner set the tutorial's acceptance test: a machine that follows it step by step, using only
its rules, takes a scramble to the solved state, playing only literal words, twist by twist; a step known only by a
certificate counts as stuck, and every step's length is recorded as it is
([decision](../../../../docs/wiki/decisions/owner-decisions-2026-10-18-machine-tutorial.md)). This stage holds that
machine, its checker, and two programs for parts the machine does not play yet:

- `scripts/runner.py`: one state-only program that applies the rules the tutorial teaches, in a fixed order, and
  writes one record of every word it plays (format `runner-run/2` since 20 October);
- `scripts/check_run.py`: an independent checker that replays a record of either format twist by twist on the exact
  simulator, without the runner or its search code; the lattice phase it checks through `verify_late.py`, which
  composes the permutations of the lattice word's retained part instead of playing it (section 6);
- `scripts/literal_lattice.py`: the literal lattice player that stage 12 asks for (`PROOFS-B.md` R22D-57). It plays
  a lattice word's retained part and its corrections twist by twist and records the played lengths and digests
  (section 4). The runner does not call it yet;
- `scripts/check_r22d25.py`: an independent checker of the records of the R22D-25 whole-stop kernel of stage 12
  (`r22d25-run/1`). It replays every accepted word twist by twist and imports no kernel module. The runner does not
  call the kernel yet.

The runner and the checker were written by Codex under two packets on 19 October (RUN-1, the runner; RUN-2, the
checker, written in parallel from the shared record format). On 20 October Codex wrote four more patches in
parallel: RUN-3 (the runner's certificate path and record format 2), RUN-4 (the checker for both formats), LLP (the
literal lattice player) and R22D-25 (the kernel, in stage 12, and its checker here). The integrator reviewed every
patch and fixed the findings that block (section 7). Research code, headless Linux; nothing here is Windows, DirectX
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
   `cellpure5.py --search`), `close` (V·W·V⁻¹) and `t1prime` (W·V′·W⁻¹·V′⁻¹, whose closure is taken at Z1, the state
   after W, with the core W⁻¹). V comes from one of two sources:
   - (a) `tclose2.py` returns the literal word V·middle·V⁻¹. The runner splits it by the core's length and checks
     that the middle is the core and that the last part is V inverted letter by letter; otherwise the attempt fails
     with outcome `contract failed`.
   - (b) When `tclose2.py` returns no literal word but its premises hold (its target, the unit for `close` and for
     `t1prime` the lattice pieces W took off, on the lattice at the end of the core's ghost play; the protected
     pieces at their exact poses; N1 and N2 not reported false) and `--literal-limit` is positive, the runner builds
     V at the state where it acts (the current state for `close`, Z1 for `t1prime`) as an R22B-10 certificate
     (RUN-3): it takes the target T, Bad of the core, and checks its digest, requires N1 and N2 from
     `jordan_target.py`, runs `r22b10_cert.py` and `check_r22b10.py` (PASS), checks the whole step's exact length
     against `--literal-limit` before it expands anything, expands V into a literal word with `expand_slp.py
     --limit`, and checks the expansion's length and sha256.

   A closure step's word is W (for `t1prime`), V, the middle and V⁻¹, kept as literal parts. A rule gives a step
   only when its whole literal word passes the gate at the state. The gate is `stepchk.py` (strict′) for every step
   except a certificate step, whose gate is `slp_apply.py --unit` (stage 12): it applies W and the middle twist by
   twist, and V and V⁻¹ as SAFE-ALL words whose letters' exact cap actions it composes into one action, applied
   once, not twist by twist. For a certificate step the runner requires strict′, the start digest, one segment line
   per part with that part's length, a `word_sha256` on the V and V⁻¹ lines equal to the runner's own parts, the W
   part ending at Z1's digest, and the end table's digest; it then loads `slp_apply.py`'s end table and asserts its
   digest, off count and off-set contract. For every other step it replays
   the word on its own copy of the state and asserts the same end digest. The first unit in order with a step takes
   it. With `--par K` several units are tried at once and their results are taken in unit order; before a later
   unit's step is taken, every earlier unit with an attempt that timed out is tried again alone, as `--par 1` would,
   and the record keeps both attempts (review finding RV1-03).
4. **Fallback**: when no unit has a step, the exhaustive search `fallback.py` for the first unit within
   `--fallback-max-length` letters; otherwise the run ends **stuck**, with every unit's attempts in the record. An
   attempt that timed out also counts as no step here, so a stuck run does not show that the rules are exhausted
   (`PROOFS-B.md` R22D-55).
5. **Lattice phase**: when every piece is on the lattice and the state is not solved, the lattice restorer
   `restore_lattice_late.py` and then its independent checker `verify_late.py`, both run in the run directory with
   the run-relative input `table:<last table>`; neither has a time limit. The run is **solved** only when the
   checker passes. The lattice word is literal, but `verify_late.py` evaluates its retained part as an exact
   permutation program on the sticker labels and plays only the corrections, so this phase is checked exactly but
   not played twist by twist (stage 12, `PROOFS-B.md` R22D-57). The player of section 4 plays it; the runner does
   not call the player yet.
6. The runner records only literal words. Two parts are not played twist by twist inside the runner: the retained
   part of the lattice word, which `verify_late.py` checks by composition (rule 5), and V and V⁻¹ of a certificate
   step, which the gate `slp_apply.py` applies as one composed exact action, after which the runner loads its end
   table (rule 3). `check_run.py` plays a certificate step's whole word twist by twist (section 3). A certificate
   never enters the record: the runner builds V as a certificate only to expand it into a literal word within
   `--literal-limit`, and only the whole literal word, passed by its gate, is recorded. `tclose2.py` may build V from
   its peeling certificate (Theorem B), but it returns V as a literal word. A closure or T1′ attempt that fails
   records N1, N2 and the deficits it read, and its outcome (`no landing word`, for T1′ only; `no literal closure`;
   `over literal limit` with V's length as a decimal string; `not strict_prime`; `digest mismatch`; `contract
   failed`; or `timeout`).

Options added by RUN-3: `--literal-limit N` (default 10,000,000 letters; 0 turns rule 3 (b) off), `--seeds
atlas|default` (default `atlas`) and `--cert-seconds S` (default 3,600). Every rule child keeps the `--rule-seconds`
limit (default 3,600); only the two lattice programs have none. An `--out` path that contains `.json` anywhere once
resolved is a usage error, reported before any file is written (RV1-07).

## 2. The record

A run directory holds `run.json` (rewritten atomically after every step), one word file per step (`{"word": [[grip,
item], ...], "input_digest", "output_digest"}`, gzipped above 100,000 letters), one unit file per tail step, the
child logs and the lattice phase output. Each step records its phase, rule, unit, word file, length in letters,
contract (`strict_prime` for tail steps; `fallback` for the fallback: the unit lands, no lattice piece goes off and
the off set shrinks), the digest and off count before and after, the left heights of the off pieces and its time. A
stuck run records the state and every unit's attempts. The result is `solved`, `stuck`, `budget` or `error`, and the
totals give the twists of the bulk, the tail, the fallback and the lattice phase. A `--table` start is stored as
`start-table.json`, exported again from the loaded state (the same digest, not necessarily the same bytes).

Since RUN-3 the format is `runner-run/2`. Every key of `runner-run/1` keeps its meaning, and format 2 adds:

- `gate` on every tail and fallback step: `stepchk` or `slp_apply`;
- on every tail closure step (`close`, and `t1prime` with a closure): `V_source` (`tclose2` or `certificate`),
  `parts` (the lengths of W, V, the middle and V⁻¹, in that order), `V_letters` and `V_sha256` (the sha256 of
  `json.dumps` of V); a T1′ step whose W alone is strict′ has only the part W. Bulk steps keep their format 1 keys,
  although the bulk phase's census rule is also named `close`;
- on a certificate step: `T_size`, `T_digest`, `N1` and `N2`;
- in `config`: `literal_limit`, `seeds`, `budgets.cert_seconds`, `source` (the commit, the sha256 of `git status
  --porcelain=v1 --untracked-files=all -- research/jumbling` and, as `dirty`, whether that output is non-empty,
  taken before the run writes anything) and `scripts` (the sha256 of 21 files the steps depend on; format 1 hashed 16);
- `lattice.input` and a top-level `error`: a reason with no path in it (a reason that would name a path reads `see
  child log`); `lattice.chain`, a boolean in format 1, is always false.

Every path in `run.json` and the word files is relative to the run directory. Since RUN-3 the lattice phase runs in
the run directory with a run-relative input, so `lattice/restore-summary.json` and `lattice/report.json` name no
absolute path either (tested; RV1-02, RV2-02). The child logs can still name absolute paths; they are
not checked, and they are scrubbed before they are published. The two committed records of section 5 are format 1
records made before this change: in their lattice outputs and logs the run directory is written `<run>`.

SIGTERM and SIGHUP stop every running child by its process group and wait for it. In the bulk phase the runner then
reloads the bulk child's last checkpoint from `run.json` and rebuilds the bulk end table from its steps; only then
does it record `error` `signal <n>` (RV1-01). The bulk child stops when the runner that started it is gone; it is
given the runner's process id, so a runner that is process 1 does not stop its own child.

## 3. The checker

`check_run.py` reads a record of format 1 or 2 and the files it names, rebuilds the start (the census scramble, or
the stored table), plays every word twist by twist on the exact simulator, and checks for each step the digests
before and after, the off counts, the heights, the length and the step's contract (strict′: the unit lands, no
lattice piece goes off and every other off piece keeps its exact pose or lands; Lemma U for the fallback), that every
word is literal, that the record has no absolute path, and the totals. A certificate step's word is replayed twist
by twist like any other, so the checker plays the V that the runner's gate applied as one composed exact action.

For a format 2 record it also checks, before the replay (RUN-4): the configuration keys and their types; the error
(a string without a path, only with result `error`); a gate on every tail and fallback step, `slp_apply` exactly
when V is a certificate; the certificate keys (`T_size` at least 1, `T_digest`, N1 and N2 true); on closure steps
the parts in their order, summing to the word's length, `V_letters` and `V_sha256` against V cut from the word, V⁻¹
equal to V inverted letter by letter and, for T1′, the middle equal to W inverted; and no closure keys on any other
step. Every failure is named in the checker's JSON lines.

For a solved run it runs `verify_late.py` on the lattice phase output, which checks the lattice word by composing
permutations, not by playing it (rule 5). The verifier now runs in its own process group with a time limit
(`--verify-seconds`, default 7,200), is stopped when the limit passes, and its report and output are saved in a new
directory `check/run-*/` inside the run directory, or next to `--out` (RV2-06). Those files can name absolute paths;
they are working files, not evidence to publish. The checker checks the words played, not the runner's choice of
rule or its budgets. A fallback step whose word is empty passes when its contract holds, as the record format allows
(RV2-05).

## 4. The literal lattice player

`literal_lattice.py` (packet LLP) reads a lattice restorer's output (`restore-summary.json` and `solutions.json` or
`solutions.json.gz`, format `lattice-restoration-v1`), the pose table the restorer started from and the correction
library, and plays the run's word literally (R22D-57).

- Before play it checks that the run is `restored`, that the model files' sha256 match the repository, that the
  solution's factors expand to the recorded retained length and factor count, that the library's sha256 matches the
  summary, that every correction names a macro of the library, and that the start table loads to the recorded input
  digest, is on the lattice and has the recorded label hash.
- Retained part: the program's nodes (`word`, `path`, `conj`, `prod`) are expanded lazily, leaf by leaf, and every
  twist is played on the 259,800 sticker labels as one gather of its primitive's sticker permutation. No net
  permutation advances the labels. The exact poses are carried along (for every `word` leaf, such as a seed word
  that recurs thousands of times, the pose change is composed once and reused, while its labels are still played
  twist by twist; `path` leaves carry poses twist by twist), so the digest at the end, or at a budget stop, is an
  exact simulator digest.
- Corrections: every correction twist is played on the exact simulator. Each must be in the I_a menu and
  admissible, and each macro must end on the lattice.
- Two orders, as the restorer wrote them. Corrections first: C is played on the start, then the retained part R; the
  digest after C is checked against the summary. Retained first: R is played on the start Y; the player builds T =
  solved·C⁻¹ on the simulator, checks its digest against the summary's `T_digest` and compares all 259,800 labels of
  Y·R with those of T; then it plays C on the played state Y·R, not on T (review finding LLP-false-pass-01).
- Solved means that the played state's labels are the identity, the owner's definition of restored
  ([decision](../../../../docs/wiki/decisions/owner-decisions-2026-10-15-jumbling-restored.md)). `final.digest` is
  the digest of the state actually played. It need not be the solved digest: labels do not fix exact poses, because
  a piece may carry a rotation that maps it and each of its stickers onto themselves. The plain test fixture ends
  with identity labels and a digest that is not the solved one (measured).
- The report (`literal-lattice/1`, written atomically) gives the input, the order, the scheduled lengths (retained,
  corrections) and the played lengths (retained, corrections, total), the junction (`T_digest`, `labels_equal`),
  `final` (`digest`, `labels_identity`), the result (`solved`; `budget` with the position: phase, factor and twist; or
  `failed` with the name of the check that failed, `input_format` for a file it cannot read and, for now, for other
  unexpected failures, LLP-contract-03), the time of each phase and the sha256 of the research scripts loaded. The
  program exits 0 for `solved` and `budget` and 1 for `failed`.
- The committed depth 20 record of section 5 writes the run directory `<run>` both in the summary's input and in the
  solutions key, and the restorer's tag rule then no longer finds the key. The player also tries the key that starts
  at `<run>`, and names a missing key `solution_tag` (LLP-contract-01); a test of section 5 reads that record's
  10,731 factors through it.
- Played speed (measured; cloud timings of research code with other jobs on the machine): two test fixtures of
  16,184,022 retained twists each, one plain and one with nine correction twists of the macro w9, played their
  retained parts in 560 s (about 28,900 twists per second) and 641 s (about 25,250 twists per second) in the
  acceptance run, and in 570 s and 561 s in Codex's own run. At these rates the 8,701,310 retained twists of the depth
  20 record would take about 5 to 6 minutes, and the 166,777,936 of E2's lattice word about 1.6 to 1.8 hours
  (estimates, not run). `PROOFS-B.md` R22D-57 estimated about one CPU hour for the same word from about 21 µs per
  twist for the label gather alone; the player's measured rates are about 35 to 40 µs per twist, on a machine
  shared with other jobs.

## 5. Tests

- `scripts/test_runner.py` (RUN-1's acceptance check, extended by RUN-3): 34 tests, one of them skipped (the depth 20
  run to the solved state, which needs `RUNNER_SLOW=1`). Before the integrator's fixes they passed in 3,142 s in
  Codex's run and in 1,526 s in the acceptance run. They include two exact certificate steps at states of E2's literal
  replay (stage 12): the closure for b20 at the state of 507 off (`T_size` 111, V of 20,232 letters, 40,500 letters in
  the step, 507 → 447 off; 742 s in Codex's run, 395 s in the acceptance run) and the T1′ step for b07 at the state of
  324 off (W of 48 letters, V of 139,774, 279,644 letters in the step, 324 → 213 off; 962 s and 538 s), each with the
  lengths, hashes and end digest of the packet. In both tests the core or W is E2's, given to `close` and `t1prime`
  directly; whether the runner's own rules, in unit order, find these steps at those states is not tested. The suite
  also has RUN-1's stored T1′ step for b04 (1,254 letters), now through `advance()` with the real mechanics; the small
  lattice configuration, now with a one-macro library; and stub tests for the certificate path, signals, the `--par`
  retry, the source identity and `--out`. On 20 October 2026, with the fixes, in this checkout: OK, 34 tests, 1 skipped (the depth 20 test, which needs `RUNNER_SLOW=1`), 1,222 s, worktree unchanged (headless Linux).
- `scripts/test_check_run.py` (RUN-2's acceptance check, extended by RUN-4): 66 tests. Before the integrator's fix
  they passed in 1,604 s in Codex's run and in 1,632 s in the acceptance run. RUN-2's tests are kept (the format 1
  lattice fixture is now built from a format 2 run); new positive and negative tests cover the format 2 checks and
  the verifier's time limit, with the gaps listed in section 7. On 20 October 2026, with the fixes, in this checkout: OK, 66 tests, 482 s (headless Linux).
- `scripts/test_literal_lattice.py` (LLP's acceptance check): 16 tests, one of them skipped (the play of the committed
  depth 20 record, which needs `LATTICE_SLOW=1`), passed in 1,525 s in Codex's run and in 1,578 s in the acceptance
  run: the expansion of every node kind, the two fixtures played to identity labels, the pose transport compared with
  the simulator on short words, tampered outputs, a wrong start or library, budget stops with an exact stop digest, a
  compressed library and several runs. The integrator added two tests: the committed record's solution key
  (LLP-contract-01), and a start that differs from T by a rotation the labels do not see, where `final.digest` must
  equal a simulator replay of C on that start and differ from the solved digest (IF-01; added after the fix review and
  not reviewed). That makes 18 tests. On 20 October 2026, with the fixes, in this checkout: OK, 18 tests, 1 skipped, 468 s; the two fixtures played their 16,184,022 retained twists at about 92,000 twists per second (cloud timing of research code, headless Linux).
- `scripts/test_check_r22d25.py` (part of the R22D-25 packet): format checks and record mutations for
  `check_r22d25.py`, run by stage 12's `test_r22d25.py` and also on their own. On 20 October 2026 they ran inside `test_r22d25.py` (stage 12), which passed; `test_check_r22d25.py` was not run on its own in this checkout.
- Two records made with RUN-1's runner on 19 October 2026 and passed by RUN-2's `check_run.py` (`runs/`, format 1,
  measured): the depth 10 scramble of seed 7001, solved by the bulk phase in 9 twists (38 s); and the depth 20 scramble
  of seed 7106, which the bulk phase brings onto the lattice in 19 twists, all of rule `seam`, and the lattice phase
  restores in 8,701,310 twists, passed by `verify_late.py` in the runner and again in the checker (654 s in all),
  which checks the lattice word by composition and does not play it twist by twist (rule 5). Neither needed a tail
  step. The start and end tables, the copy of the lattice library (the same sha256 as
  `09-general-method/data/library-merged.json.gz`) and the restorer's working files are left out: `runner.py
  --scramble` makes them again, and `run.json` holds their digests. In the lattice outputs and logs the run directory
  is written `<run>`.

## 6. What it does not do yet

- E2's certificate steps: rule 3 (b) builds and expands a certificate V at the state where it acts, but it is tested
  only at two states of E2's literal replay, not on a run from a scramble, and no runner record with a certificate
  step exists outside the tests. In both tests the core or W is E2's, given to `close` and `t1prime` directly; whether
  the runner's own rules, in unit order, find these steps at those states is not tested. E2's b06 step also needed a
  three-letter SAFE-ALL preparation, (178, 7)(170, 1)(75, 1), before its T1′ step (stage 12, `round22/e2_b06_*.py`);
  the runner has no rule that makes such a preparation.
- Height 4: the R22D-25 kernel of stage 12 plays R22D-25 with R22D-37's height-6 tier over a whole stop, under the
  practical catalogue `r22d25-cat/1` (S and T of one letter; R22D-60's pass left out). By `PROOFS-B.md` R22D-55 this
  is a revision of the rule, so its stuck outcome covers only that catalogue. `check_r22d25.py` checks its records,
  but the runner does not call the kernel. The kernel's direct family R22D-19 is an external program
  (`round22/r22d19_rule.py`) that is not in the repository, so at a state with height-4 units where the height-6 tier
  has no step the kernel reports `unsupported` before it tries S·T.
- The lattice phase is not played twist by twist in a run: `verify_late.py` evaluates the retained factors of the
  lattice word through their net sticker permutations (lines 344 to 348) and plays only the corrections on the
  simulator (lines 351 to 374), and `runner.py` takes solved from its report (lines 908 to 913 at commit `31b78a5`;
  R22D-57 cites lines 645 to 650 of RUN-1's `runner.py`). The player of section 4 plays the whole word literally, but
  the runner does not call it, and it has not been run on a runner record (its test on the committed depth 20 record
  needs `LATTICE_SLOW=1` and was not run).
- The round 11 lattice schedule has a restorer for real lattice configurations (stage 09,
  `schedule11/restore_schedule11_late.py`, with its test outcome recorded there); the runner does not call it.
- It decides nothing about coverage: a solved run shows that the rules reached the solved state from that start.

## 7. Review findings

The integrator reviewed each patch for its contract, for false passes and for robustness, and had every finding
checked twice, independently. A separate fast Codex review of the fixes alone found three minor gaps (IF-01 to
IF-03), which were fixed after it; those fixes have not been reviewed. Findings that block, and what the integrator
did:

- RUN3-contract-01, RUN3-robust-01: RUN-3 had put the two lattice programs under `--rule-seconds`, so a long lattice
  phase would end `error`; both run without a time limit again.
- RUN3-robust-02: the bulk child's watch on its parent killed its own group at once when the runner was process 1,
  so every such run ended `error`; the runner now passes its process id to the child (`RUNNER_PARENT_PID`), and the
  child watches that. No test runs the runner as process 1.
- LLP-contract-01, LLP-robust-02: the player could not find the solution of the committed depth 20 record; it now
  tries both keys (section 4), and a test reads the record's 10,731 factors.
- LLP-false-pass-01 (with LLP-contract-02): at a retained-first junction the player played C on T and reported the
  digest of T·C, the solved digest by construction, so its final digest check could not fail; it now plays C on the
  played state and reports that state's digest, the check that could not fail is gone, the fixture test requires
  identity labels, and the new test of section 5 (IF-01) checks the difference.
- LLP-robust-01: `test_literal_lattice.py` hashed the whole checkout, `.git` and `work/` included, so it would fail
  in the main checkout whenever other work wrote there during its run (shown by the review's probes on a scratch
  tree, not by a run in the main checkout); it now skips Git metadata and the private work trees, tolerates files
  that vanish, and keeps directory links (IF-02).
- R22D25-robust-02 (with R22D25-contract-04): `check_r22d25.py` failed every older record after any edit of the
  kernel files, which it never uses; now only `assets/manifest.json` must match, and kernel files that differ are
  listed as `kernel_differs`. R22D25-robust-01 (the kernel's test) is in stage 12, SCHL-robust-01 in stage 09.

Minor findings fixed in the same pass: RUN3-contract-04 and the first part of RUN3-robust-03 (the source identity is
taken before the run writes), RUN3-robust-05 (a test run over 60 minutes is a note, not a failure), the first part
of RUN3-robust-06 (git runs without optional locks), RUN4-robust-05 (`test_check_run.py`'s worktree check leaves
out the private work trees; fixed after the fix review), LLP-contract-03 in part (a missing solution is named),
R22D25-false-pass-05 in part (an `unsupported` record must show a piece above height 6 or the D19 program missing),
IF-01 and IF-02 (fixed after the fix review; not reviewed).

Minor findings left for later:

- RUN-3: `--cert-seconds` is not kept below `--rule-seconds`, so a certificate search that uses its whole budget is
  recorded as `timeout` (RUN3-contract-02, RUN3-robust-04); an incomplete certificate is not retried under `--par`
  (RUN3-false-pass-02); a run killed in a way the runner cannot catch keeps the placeholder `budget`
  (RUN3-false-pass-01); the lattice test uses a one-macro library, not the production library or the `--table`
  command line (RUN3-contract-03, RUN3-false-pass-07); test gaps for the V line's hash, the lone retry, the source
  command and the W end digest (RUN3-false-pass-03 to -06); nothing checks that the repository root is the Git
  work tree's top level (RUN3-robust-03, second part); the source test calls git unguarded (RUN3-robust-06, second
  part); word and temporary files left by a killed bulk child (RUN3-robust-07).
- RUN-4: a format 2 label defect stops the replay of the whole record (RUN4-contract-01); the suite runs close to the
  wrapper's 30-minute limit, with five replays of the b04 step (RUN4-contract-02, RUN4-robust-03); the time-limit test
  uses a stand-in that obeys SIGTERM and allows it 0.5 s (RUN4-contract-03, RUN4-false-pass-05, RUN4-robust-04);
  checks not made: `dirty` against `status_sha256`, T and N keys on other steps, a gate on bulk steps, `literal_limit`
  against the steps, a bare file name in `error` (RUN4-false-pass-01, -02, -06 and the second part of -03); test gaps:
  the error test never reaches the private-path check, and no test changes `V_letters` alone with a right hash
  (RUN4-false-pass-03, -04); the error check rejects any `/`, which is stricter than the format (RUN4-contract-05);
  RUN-2's lattice fixtures now derive from a format 2 run (RUN4-contract-04); the status guard cannot see new contents
  of ignored or modified files (RUN4-false-pass-07); a killed checker leaves its verifier running, and the verifier
  runner needs POSIX process groups (RUN4-robust-01, RUN4-robust-02); `check/` logs and plain copies of chain words
  stay in the run directory (RUN4-robust-06).
- LLP: the five-second command-line budget test depends on the machine's load (LLP-contract-04, LLP-robust-03); the
  expansion test checks `expand`, not the leaves the player plays (LLP-contract-05); no negative test reaches the
  junction or post-correction checks, and the target phase counts `correction_index` in reverse (LLP-contract-06,
  LLP-false-pass-02); other failures are still reported as `input_format` (LLP-contract-03); exit 0 for `budget`
  (LLP-false-pass-03); an old report stays at the output path after a crash (LLP-false-pass-04); errors outside the
  caught list and the library fallback (LLP-robust-04); the report file's newline, mode and directory fsync
  (LLP-robust-05); the slow test stops only `runner.py` (LLP-robust-06); script provenance taken from loaded modules
  after the run (LLP-robust-07); side effects on import (LLP-robust-08); temporary directories left by killed test
  runs (LLP-robust-09).
- R22D-25 checker: step 4's S is re-derived only as a count (R22D25-contract-08, R22D25-false-pass-01); the gate
  mutation is caught by the parameter check before the gate (R22D25-contract-02, R22D25-false-pass-02); the start
  file is named by its copy, and the census fallback by bare name is ambiguous (R22D25-contract-03,
  R22D25-robust-05); smaller limits are accepted under `r22d25-cat/1` (R22D25-contract-06); a record with result
  `error` passes with no check (R22D25-false-pass-05). The kernel's findings are listed in stage 12.

RUN-3 and RUN-4 took up the review findings that RUN-1 and RUN-2 had left: RV1-01, RV1-02, RV1-03, RV1-04, RV1-06,
RV1-07, RV2-02, RV2-03 and RV2-06.

## Replay

From the repository root, with Python 3 and NumPy, `S=research/jumbling/restoration/13-tutorial-runner/scripts`:

    python3 -I -B $S/test_runner.py                       # times in section 5
    python3 -I -B $S/test_check_run.py                    # times in section 5
    python3 -I -B $S/test_literal_lattice.py              # times in section 5
    python3 -I -B $S/test_check_r22d25.py
    python3 -I -B $S/runner.py --scramble 10 7001 --out <new dir> --cache <new dir>
    python3 -I -B $S/runner.py --scramble 20 7106 --out <new dir> --cache <new dir>   # about 11 minutes
    python3 -I -B $S/check_run.py <run dir> [--verify-seconds S]
    python3 -I -B $S/literal_lattice.py <run dir>/lattice <new file>.json --start-table <run dir>/<lattice input table>
    python3 -I -B $S/check_r22d25.py <kernel run dir> --cache <new dir>

`<lattice input table>` is, for a format 2 record, the table that `lattice.input` names in `run.json`; a format 1
record has no `lattice.input`, and its lattice phase's `restore-summary.json` names the table (for the committed
depth 20 record `tables/bulk-end.json`). The committed records leave their tables out, so run the runner again
first, as `LATTICE_SLOW=1` does: before `test_literal_lattice.py` it runs the runner on the depth 20 scramble again
and plays the committed record's lattice word (not run). The `--out` path of the runner must not contain `.json`.

## Files

| Path | What it is |
| --- | --- |
| `scripts/runner.py` | The tutorial runner (RUN-1; RUN-3 on 20 October: the certificate path of rule 3, record format 2, signals, run-relative lattice paths and the source identity), with the integrator's fixes after review: the lattice programs run without a time limit again (RUN3-contract-01, RUN3-robust-01), the bulk child watches the runner's process id (RUN3-robust-02), and the source identity is taken before the run writes, with git run without optional locks (RUN3-contract-04, RUN3-robust-03, RUN3-robust-06) |
| `scripts/test_runner.py` | Its acceptance tests (RUN-1, extended by RUN-3); the integrator made the worktree check also skip the private work trees, tolerate files that vanish and run git without optional locks, and made a run over 60 minutes a note, not a failure (RUN3-robust-05) |
| `scripts/check_run.py` | The independent record checker (RUN-2; RUN-4 on 20 October: both formats, the format 2 checks, a time limit and saved output for `verify_late.py`), with two fixes by the integrator after RUN-2's review: every string in the record is scanned for an absolute, home-relative, Windows or parent path (RV2-01), and a corrupt gzip body fails as the file it is in (RV2-04) |
| `scripts/test_check_run.py` | Its acceptance tests (RUN-2, extended by RUN-4); the timeout of the lattice fixture was raised from 1,100 to 3,600 seconds by the integrator on 19 October after the first acceptance run timed out on a loaded machine, and on 20 October its worktree check was made to leave out the private work trees and to run git without optional locks (RUN4-robust-05) |
| `scripts/literal_lattice.py` | The literal lattice player (LLP, R22D-57; section 4), with the integrator's fixes for the committed record's solution key (LLP-contract-01, LLP-robust-02) and for the played final digest at a retained-first junction (LLP-false-pass-01, LLP-contract-02) |
| `scripts/test_literal_lattice.py` | Its acceptance tests, with the integrator's narrower worktree snapshot (LLP-robust-01, IF-02) and two added tests (LLP-contract-01, IF-01) |
| `scripts/check_r22d25.py` | The independent checker of `r22d25-run/1` records (packet R22D-25, with the kernel of stage 12), with the integrator's fixes: only `assets/manifest.json` must match its recorded digest (R22D25-robust-02, R22D25-contract-04), and an `unsupported` record must show its cause (R22D25-false-pass-05) |
| `scripts/test_check_r22d25.py` | Its tests: format checks and record mutations, run by stage 12's `test_r22d25.py` or on their own |
| `runs/depth10-seed7001/`, `runs/depth20-seed7106/` | The two format 1 records of section 5: `run.json`, the words, the lattice phase's summary, report and solutions (gzipped), the logs, the runner's last line (`runner.json`) and the checker's output (`check.jsonl`) |

## Checks not run

- Windows, DirectX, the native host, input, long-session and performance checks: none; timings are cloud timings of
  research code with other jobs on the machine.
- The slow tests: `RUNNER_SLOW=1` (the depth 20 run to the solved state through RUN-3's runner), `LATTICE_SLOW=1`
  (the player on the committed depth 20 record) and stage 12's `R22D25_SLOW=1`.
- The player on any runner record, and RUN-4's `check_run.py` on the two committed records.
- A runner run from a scramble that takes a certificate step.
- The fast review of the fixes covered the fixes only, not the whole patches. The fixes for IF-01 to IF-03 (in
  `test_literal_lattice.py` and stage 09's `test_sched11_late.py`) and the fix to `test_check_run.py`
  (RUN4-robust-05) came after it and have had no review.
