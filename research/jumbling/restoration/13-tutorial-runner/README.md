# 13. The tutorial runner: a machine that follows the rules (19 and 20 October 2026)

On 18 October 2026 the owner set the tutorial's acceptance test: a machine that follows it step by step, using only
its rules, takes a scramble to the solved state, playing only literal words, twist by twist; a step known only by a
certificate counts as stuck, and every step's length is recorded as it is
([decision](../../../../docs/wiki/decisions/owner-decisions-2026-10-18-machine-tutorial.md)). This stage holds that
machine with its phase plugins, its checker, and two programs for parts the machine does not play yet:

- `scripts/runner.py`: one state-only program that applies the rules the tutorial teaches, in a fixed order of
  phases, and writes one record of every word it plays (format `runner-run/3` since 20 October; section 2);
- `scripts/runner_api.py`: the interface between the runner and its phase plugins, and the writers and readers of step
  trees and `ia16` words (section 4);
- `scripts/rules_height.py`, `scripts/rules_kernel.py` and `scripts/lattice_phase.py`: the plugins of the height,
  high and lattice phases. They are stubs until packets P1 to P3 replace them (section 1, "Stubs and component
  labels");
- `scripts/check_run.py`: an independent checker that replays a record of any of the three formats twist by twist on
  the exact simulator, without the runner or its search code; a lattice phase taken by composition it checks through
  `verify_late.py`, which composes the permutations of the lattice word's retained part instead of playing it
  (section 6), and a played lattice phase (format 3) it replays through `check_lattice.py`. Its two
  helpers `scripts/check_contracts.py` (height and kernel contracts) and `scripts/check_lattice.py` (literal lattice
  steps) are stubs until packets P4 and P5 (section 3);
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
patch and fixed the findings that block (section 7). Later on 20 October the integrator wrote part I0 of the plan for
record format `runner-run/3` (revision 3d, which passed the senior plan check): the fixed phase order and its plugin
interface, acceptance and development modes, the clocks, and a tree of named parts for every step, on the runner's
side and on the checker's, with stub plugins. A fast review of I0 found four major gaps, which the integrator fixed
the same day (section 7). Research code, headless Linux; nothing here is Windows, DirectX or performance evidence.

Labels: **certificate** (an exact computation with a replayable record), **measured** (a result on finite inputs).

## 1. The rules the runner follows

The runner reads only the current configuration. It uses the scramble's history only to build the start state, and
it breaks every tie by fixed keys, with no randomness. Since format 3 this holds for the bulk phase too: before it,
the bulk also ran reopen, which reads the solver's own earlier twists (rule 2).

**Phases** (format 3; the owner's second reading of 20 October 2026,
`docs/wiki/decisions/owner-decisions-2026-10-20-tutorial-acceptance.md:77-80`):

- The order is bulk → height → high → tail → lattice (`scripts/runner_api.py:21`;
  `scripts/runner.py:1379-1388`).
  - Height has three sub-phases, in the order `r22d75` → `r22d81` → `h4` (`scripts/runner_api.py:22`,
    `scripts/runner.py:659-670`).
  - High is the R22D-25 kernel with D19 (stage 12).
  - In development mode the tail includes the fallback (rule 4).
- The runner goes through the phases in this order and never goes back.
- A phase starts at the first state where the phase before it finds no step. Inside a phase, the first rule that fires
  takes the step, and every rule reads only the current state.
- Every step records its phase and, in the height phase, its sub-phase (`phase`, `subphase`).
- So the rule that applies can depend on the path. At `bbef1c8e`, R22D-75 fires with (23, 14)
  (`12-low-height/runs/round22/cloud/height4-descent-bbef1c8e/r22d75_from_snapshot.txt:3`), but a run that reaches
  `bbef1c8e` inside `r22d81` does not go back to `r22d75`. Format 2's bulk → tail order already worked this way
  (decision page, :80).

**Entry tests.** A phase or sub-phase runs only when its entry test holds at the state where it starts; otherwise its
result is `not_applicable`. The tests need no rule code (`scripts/runner.py:584-590`). With n4 and n6 the numbers of
off pieces at height 4 and 6 and max the largest height:

- `r22d75`: max ≤ 6 and n4 + n6 > 0;
- `r22d81`: max ≤ 6 and n6 > 0;
- `h4`: max ≤ 4 and n4 > 0, so n6 = 0 (`h4_unlock.py` asserts max ≤ 4:
  `12-low-height/scripts/round22/cloud/height4-descent-bbef1c8e/h4_unlock.py:105-107`);
- `high`: max ≤ 6 and n4 + n6 > 0;
- `tail`: off > 0;
- `lattice`: off = 0.

After each step the runner tests again (`scripts/runner.py:611-626`):

- when the entry test fails, the sub-phase or phase ends `handoff`;
- when no piece is off, it ends `lattice`, and the phases in between are `not_applicable`;
- a plugin may not claim `handoff` or `lattice` against the state, and only a stub may answer `not_applicable` while
  its entry test holds; either is an error (`scripts/runner.py:632-641`).

**Phase results.**

| Result | When | What the run does |
| --- | --- | --- |
| `lattice` | no piece is off after a step | goes to the lattice phase; the phases in between are `not_applicable` |
| `handoff` | after a step, the entry test fails | goes to the next sub-phase or phase |
| `not_applicable` | the entry test fails where the phase starts | goes on |
| `stuck` | no rule of the phase gives a step and no attempt at this state timed out | goes on; after the tail the run ends `stuck` |
| `budget` | a step budget ends the phase (`--bulk-steps`, `--high-steps`, `--tail-steps`; after P1 and P2 a rule's BUDGET or the kernel's own budget) | after the bulk the run goes on as a component; after any other phase the run ends `budget` |
| `timeout` | a runner clock cuts the phase (`--bulk-seconds`, `--bulk-step-seconds`; after P1 and P2 the kill of a height or kernel call); in the tail: no tail rule gives a step and an attempt at this state timed out | as `budget` |
| `error` | an exception: a crash, truncated or malformed child output or a digest mismatch (`RunError`, below), a false claim by a plugin, or a lattice `End` other than the stub's (until I1); after P2 a kernel `error` or `unsupported` | the run ends `error`; no later rule, unit or phase runs, and the steps before it stand |

Sources: the bulk `scripts/runner.py:530-578` (component `bulk_budget` at :574-576); height and high :659-670 (a
`budget` or `timeout` ends the run `budget`, :1382-1384) and :652-656 (`error`); the tail :1211-1253; `--no-lattice`
ends the run `budget` (:1260-1263); an exception ends it `error` (:1390-1397). Under `PROOFS-B.md` R22D-55 a stop is
not exhaustion, so a `budget` or `timeout` in any phase after the bulk ends the run `budget`, never `stuck`.

**Errors and failed attempts** (review finding I0-01). A rule that finds no step records a failed attempt, and the
next rule or unit is tried. A child that crashes, writes truncated or malformed output, or disagrees with a digest the
runner holds raises `RunError` (`scripts/runner.py:317-319`); the unit census's digest check and the runner's own
replay of a word raise `ValueError` (:727-728, :263-266, :1139-1154). No rule handler catches either: the run ends
`error` at once, no later rule, unit or phase runs, and the steps before it stand.

- Failed attempts: `no word`; `not applicable` (an applicability screen, below); `no literal closure`;
  `certificate_needed`; `over literal limit`; `not strict_prime`; `no landing word`; the fallback's `bound
  exhausted`; and `timeout` from any clock.
- `require` reads every child's exit (`scripts/runner.py:515-528`). A clock or a cancellation fails the attempt. Any
  exit code the call does not declare is a crash. The one exception: `cellpure5.py` and `transport_census.py` write
  their applicability tests as asserts (`12-low-height/scripts/cellpure5.py:191`, :207, :212;
  `12-low-height/scripts/round22/transport_census.py:110`, :165). A failed assert at one of these lines, read from the
  last frame of the child's last traceback (`SCREENS` and `failed_assert`, `scripts/runner.py:55-59`, :108-121), means
  the rule does not apply to the unit. Any other failed assert of theirs is a crash. `config.scripts` pins both files.
- Integrity failures, which end the run `error` (all in `scripts/runner.py`): `stepchk.py` with no end table or a
  digest mismatch (:740-745); `tclose2.py` with no readable `pre.json` after exit 0, a word that is not V·middle·V⁻¹,
  or other unreadable evidence (:774-786, :801-803); a Bad(W), certificate state or certificate length mismatch
  (:808-810, :845-849); a `check_r22b10.py` rejection that names a digest, N1, N2 or an incomplete certificate, all of
  which the runner has already established (`CHECK_INTEGRITY`, :60-62, :850-857), and an exit 0 without PASS
  (:859-861); an expansion of the wrong length or hash (:876-877); every `slp_apply.py` mismatch, and a missing
  summary line (:919-952); `prep_t1prime.py` inputs that are missing or do not match (:974-980); a fallback word that
  is missing or does not land the unit without collateral, which `fallback.py` requires before it writes its word
  (:1122-1129; `09-general-method/scripts/gm/fallback.py:92`, :102).
- `jordan_target.py`'s fixed CPU cap is a clock, so it gives a timed-out attempt; an exit 0 without a `CONDITION`
  line, or a SIGKILL before the cap's 840 s, is an error ("Timed-out attempts", below).
- Under `--par K` with K > 1, an error in a later unit's trial waits like a later unit's step: it is raised only when
  no earlier unit's serial retry gives a step, so `--par 1` and `--par 2` give the same run
  (`scripts/runner.py:1081-1086`, :1111-1112; rule 3).

The rules of each phase:

1. **Start**: `--scramble <k> <seed>` (the census scramble of stage 11) or `--table <pose table>`.
2. **Bulk phase**: the census loop of stage 11 (`theory/seams.py`, `SeamRestorer`), in the order seam, cover, close,
   conj, lock, improve, shift: at each step the first rule that returns twists; each step is one word. The bulk child
   passes this order to `SeamRestorer` and asserts it (`scripts/runner.py:45-46`, :1443-1444), and the record keeps it
   in `config.bulk_order`. The phase ends on the lattice, when no rule fires (`stuck`), or at a step or time budget;
   a step cut off by the clock is not recorded. The height and high phases follow it, before the tail.
   - Format 3 leaves out reopen, the last rule of the default order (`theory/seams.py:95`). Reopen inverts one of the
     solver's own earlier twists, most recent first (`theory/seams.py:285-308`). It reads `Restorer.moves`, the
     twists the solver has played (`theory/restore.py:104`, :135), so it does not read only the current state.
   - This is the integrator default on the decision page (:82-85). It costs nothing measured: reopen never found the
     next step first in any census run (`11-scramble-census/README.md:154-155`).
3. **Tail phase**, at each state: the units of `mkblocks2.py`, largest first, then by key; for each unit the tail
   rules in order: `zero_bad` (a transport without endpoint collateral), `pure` (the class-cell pure twist of
   `cellpure5.py --search`), `close` (V·W·V⁻¹) and `t1prime` (W·V′·W⁻¹·V′⁻¹, whose closure is taken at Z1, the state
   after W, with the core W⁻¹). V comes from one of two sources:
   - (a) `tclose2.py` returns the literal word V·middle·V⁻¹. The runner splits it by the core's length and checks
     that the middle is the core and that the last part is V inverted letter by letter. `tclose2.py` always writes
     that form (`09-general-method/scripts/gm/tclose2.py:203`, :240-242), so any other word is malformed output and
     the run ends `error` (I0-01).
   - (b) When `tclose2.py` returns no literal word but its premises hold (its target, the unit for `close` and for
     `t1prime` the lattice pieces W took off, on the lattice at the end of the core's ghost play; the protected
     pieces at their exact poses; N1 and N2 not reported false) and `--literal-limit` is positive, the runner builds
     V at the state where it acts (the current state for `close`, Z1 for `t1prime`) as an R22B-10 certificate
     (RUN-3): it takes the target T, Bad of the core, and checks its digest, requires N1 and N2 from
     `jordan_target.py`, runs `r22b10_cert.py` and `check_r22b10.py` (PASS), checks the whole step's exact length
     against `--literal-limit` before it expands anything, expands V into a literal word with `expand_slp.py
     --limit`, and checks the expansion's length and sha256. A digest, length or hash that does not match ends the
     run `error`.

   A closure step's word is W (for `t1prime`), V, the middle and V⁻¹, kept as literal parts. A rule gives a step
   only when its whole literal word passes the gate at the state. The gate is `stepchk.py` (strict′) for every step
   except a certificate step, whose gate is `slp_apply.py --unit` (stage 12): it applies W and the middle twist by
   twist, and V and V⁻¹ as SAFE-ALL words whose letters' exact cap actions it composes into one action, applied
   once, not twist by twist. For a certificate step the runner requires strict′, the start digest, one segment line
   per part with that part's length, a `word_sha256` on the V and V⁻¹ lines equal to the runner's own parts, the W
   part ending at Z1's digest, and the end table's digest. It checks the evidence before it reads strict′: any
   mismatch, or no summary line, ends the run `error`, and only a false strict′ with otherwise matching evidence fails
   the attempt (`scripts/runner.py:928-948`). Then, in development mode, it loads `slp_apply.py`'s end table and asserts
   its digest, off count and off-set contract; in acceptance mode it plays the word twist by twist instead (A2,
   below). For every other step it replays the word on its own copy of the state and asserts the same
   end digest. The first unit in order with a step takes it. With `--par K` several units are tried at once and
   their results are taken in unit order; before a later unit's step is taken, every earlier unit with an attempt
   that timed out is tried again alone, as `--par 1` would, and the record keeps both attempts (review finding
   RV1-03). A later unit's error waits in the same way (I0-01).
4. **Fallback**: when no unit has a step, the exhaustive search `fallback.py` for the first unit within
   `--fallback-max-length` letters; otherwise the run ends **stuck**, with every unit's attempts in the record. An
   attempt that timed out also counts as no step here, so a stuck run would not show that the rules are exhausted
   (`PROOFS-B.md` R22D-55). Since format 3 such a stop is recorded as it is: the tail ends `timeout`, `stuck.reason`
   is `timeout` and the run ends `budget` (`scripts/runner.py:1238-1250`). In acceptance mode the fallback runs only
   as a probe (below).
5. **Lattice phase**: when every piece is on the lattice and the state is not solved, the runner asks the lattice
   plugin to play the phase (`scripts/runner.py:1255-1265`). The plugin returns a `LatticeResult` or an `End`.
   - Until packet P3 the plugin is a stub. Its `End('not_applicable')` keeps the path of format 2: the lattice
     restorer `restore_lattice_late.py` and then its independent checker `verify_late.py`, both run in the run
     directory with the run-relative input `table:<last table>`; neither has a time limit. The run is **solved** only
     when the checker passes. The lattice word is literal, but `verify_late.py` evaluates its retained part as an
     exact permutation program on the sticker labels and plays only the corrections, so this phase is checked exactly
     but not played twist by twist (stage 12, `PROOFS-B.md` R22D-57). Such a run is a component run,
     `lattice_composition` (`scripts/runner.py:1274-1277`). The player of section 4 plays it; the runner does not
     call the player yet.
   - After P3 the plugin plays the phase in its child and returns `LatticeResult(steps, end_table, report)`
     (`scripts/runner_api.py:61-87`; review finding I0-02). Before anything changes, the runner re-reads every step's
     `ia16` word and tree from disk and checks their sha256, length and size, the step's labels, the digest chain
     from the current state through every step (each step starts where the one before ended, and every step ends on
     the lattice), identity labels on the last step, and that the end table has the last step's digest and loads to
     its snapshot (`scripts/runner.py:1319-1349`). The runner does not replay the words; the checker does (section 3).
     Then it moves the files into the record, one row per step, loads the end table and ends the run `solved`
     (:1350-1372). Any mismatch ends the run `error` before a row is written or a file moved.
   - Any other `End` (`stuck`, `budget`, `timeout`, `error`, or `not_applicable` from a plugin that is not a stub)
     ends the run `error` and leaves `phases.lattice` as it was (`scripts/runner.py:1269-1275`). Recording such an
     `End` needs the checker's rules for it, which come with P3 (I1; deferred, section 7).
6. The runner records only literal words. Two parts are not played twist by twist inside the runner: the retained
   part of the lattice word, which `verify_late.py` checks by composition (rule 5), and, in development mode only, V
   and V⁻¹ of a certificate step, which the gate `slp_apply.py` applies as one composed exact action, after which the
   runner loads its end table (rule 3). After P3 the lattice words are played twist by twist in the plugin's child;
   the runner re-reads them, and the checker replays them (rule 5). `check_run.py` plays a certificate step's whole
   word twist by twist (section 3). A certificate never enters the record: the runner builds V as a certificate only
   to expand it into a literal word within `--literal-limit`, and only the whole literal word, passed by its gate, is
   recorded. `tclose2.py` may build V from its peeling certificate (Theorem B), but it returns V as a literal word. A
   closure or T1′ attempt that fails records N1, N2 and the deficits it read, and its outcome (`no landing word`, for
   T1′ only; `no literal closure`; `over literal limit` with V's length as a decimal string; `certificate_needed`, in
   acceptance mode only; `not strict_prime`; or `timeout`). Since the I0 fixes a digest mismatch or a malformed word
   is not an outcome: it ends the run `error` (above).

**Acceptance and development mode** (`--mode acceptance|development`, default `development`;
`scripts/runner.py:1512-1513`):

| | Development | Acceptance |
| --- | --- | --- |
| The runner's own clocks | as given | off |
| Child clock arguments | as given | the sentinel 10⁹ s |
| `config.budgets` for those clocks | their values | null; `config.clock_sentinel` 10⁹ |
| A closure whose V is a certificate, expanded within `--literal-limit` | the gate's end table is loaded; `standing: certificate` | played twist by twist; `standing: rule` |
| A closure whose V is a certificate that is incomplete (not on a clock), rejected by its checker or over `--literal-limit` | `no literal closure` or `over literal limit` | `certificate_needed` |
| A closure whose premises fail (`tclose2.py`'s premises, N1 or N2, or `--literal-limit 0`) | `no literal closure` | `no literal closure` |
| Fallback | a step, `standing: fallback` | a probe; its word is never played |
| Step budgets (`--bulk-steps`, `--tail-steps`, `--high-steps`) | as given | as given |

- **A2** (the integrator's amendment of the plan). A closure whose V comes from a certificate is known by its literal
  word once V is expanded, as stage 12 records E2's steps; it is not a step known only by a certificate. So in
  acceptance mode the runner plays it twist by twist (`scripts/runner.py:1142-1149`), requires the gate's end digest
  and off-set contract (:1150-1154), exports the table from the played state (:1160-1162) and never loads the gate's
  end table. The `slp_apply` gate still checks the step's contract. The row gets `standing: rule`, `construction:
  certificate` and `certificate_sha256` (:1167-1168, :1181-1182).
- **`certificate_needed`.** Where a certificate is built but no literal word comes out of it, an acceptance attempt
  records `certificate_needed`, with the ghost's T size and digest (`scripts/runner.py:377-379`, :812, :841-844,
  :854-858, :862-871): the certificate is incomplete and did not end on a clock (below), `check_r22b10.py` rejects it
  (exit 1 with a `FAIL:` line, `12-low-height/scripts/check_r22b10.py:483-485`, that names nothing the runner has
  already established), or the whole step is over `--literal-limit` (default 10⁷). That is the owner's "step known
  only by a certificate". A closure whose premises fail (`tclose2.py`'s premises, N1 or N2) builds no certificate and
  records `no literal closure` in both modes (`scripts/runner.py:791-796`, :831-833). A complete, checked
  certificate whose whole step is within the limit is never `certificate_needed`: it is played.
- **Development mode** keeps format 2's behaviour: a certificate step takes the gate's end table and has `standing:
  certificate` (`scripts/runner.py:1143-1146`, :1167-1168), and the fallback takes steps.
- **The probe.** In acceptance mode, when no unit has a step, the fallback runs once, for the first unit, as a probe
  (`scripts/runner.py:1227-1237`). A word it finds is written to `probe/word.json` and `probe/unit.json`, named in
  `stuck.probe` (`unit`, `unit_key`, `word`, `letters`, `input_digest`, `output_digest`; :1200-1209) and never
  played. `acceptance.probe` is true when it found a word, false when it ran and found none, and null when it did not
  run (:1243-1246). A probe whose check fails on a digest ends the run `error`, with `acceptance.probe` null.

**Clocks.** The owner set no time limit (decision page, :68). There are two kinds of clock: the runner's own wall
clocks, and the clocks inside the child programs, which the runner does not edit.

- **The runner's own clocks**: `--bulk-seconds`, `--bulk-step-seconds`, `--rule-seconds` and, after P1 and P2, the
  wall-clock kill of a height or kernel call. In acceptance mode they are off: a child runs with `seconds=None`,
  which means no clock (`scripts/runner.py:466-506`, :512, :550-551; `Ctx.wall`, `scripts/runner_api.py:107-109`).
- **Child clocks: one adapter.** The child programs take their clocks as command-line numbers: null cannot be passed,
  and most of them reject infinity (list below). So every child clock argument goes through `runner_api.child_clock`:
  its value in development mode, and the sentinel `CLOCK_SENTINEL` = 10⁹ s (about 31.7 years) in acceptance mode
  (`scripts/runner_api.py:28`, :115-121). `config.budgets` records null for each lifted clock and
  `config.clock_sentinel` records 10⁹ (`scripts/runner_api.py:124-127`; `scripts/runner.py:48-50`, :427-443).
- **The child clocks**, and what a null or infinite value would do:
  - `tclose2.py --greedy-seconds` (`scripts/runner.py:755-756`): a deadline in its greedy vacancy search; at the
    deadline it stops the search and writes `pre.greedy.timed_out` (`09-general-method/scripts/gm/tclose2.py:171-186`).
    It reads the value with `float`, so infinity would pass here and null would not (:171).
  - `r22b10_cert.py --seconds` (`scripts/runner.py:835-837`): finite and positive required
    (`12-low-height/scripts/r22b10_cert.py:857-858`).
  - After P1, `--cpu` of `r22d75_from_snapshot.py`, `r22d81_rule.py`, `h4_unlock.py`, `two_letter_u.py` and
    `r22d81_pairs.py`: each sets `RLIMIT_CPU` from `int(cpu)`, which fails on infinity (stage 12,
    `round22/r22d81_rule.py:49`, `round22/r22d81_pairs.py:71-72`; in `round22/cloud/height4-descent-bbef1c8e/`,
    `h4_unlock.py:48`, `two_letter_u.py:44`, `r22d75_from_snapshot.py:24`).
  - After P2, `r22d25_search.py --seconds`: its `Budget` rejects a non-finite value (`round22/r22d25_search.py:63-65`).
    The kernel passes its remaining time to D19, which requires a finite, positive value
    (`round22/r22d19_rule.py:684-685`), through `subprocess.run` without pipes (`round22/r22d25_search.py:345-348`).
  - After P3, `literal_lattice.play`: `seconds=None` means no deadline (`scripts/literal_lattice.py:402`, :480).
  - `jordan_target.py` takes no clock argument: it sets a fixed CPU cap of 840 s (`round22/jordan_target.py:10-14`).
    This is the one clock acceptance mode cannot lift.
- **A limit on the sentinel** (measured, 20 October 2026, CPython 3.13 on this Linux machine): `subprocess.run(...,
  capture_output=True, timeout=t)` raises `OverflowError: timeout is too large` for t above 2³¹ − 1 milliseconds,
  about 24.9 days (2,147,483.6 s passed, 2,147,483.7 s failed); without pipes it accepts 10⁹. No call passes the
  sentinel to a capturing `subprocess.run`.

**Timed-out attempts.** In both modes an attempt that ends on any clock records `timed_out: true`
(`scripts/runner.py:144-148`):

- the runner's kill under `--rule-seconds` (development mode; `scripts/runner.py:512`, :520-521);
- `tclose2.py`'s `pre.greedy.timed_out` (`scripts/runner.py:769-772`);
- `jordan_target.py` printing `CUT_SHORT_CPU_840_SECONDS`, exiting 124, or killed by its hard limit (SIGKILL after at
  least `JORDAN_CPU_SECONDS` = 840 s; the child is single-threaded, so its CPU cap cannot fire earlier), before any
  `CONDITION` line (`scripts/runner.py:63-64`, :818-826). An exit 0 without a `CONDITION` line, or an earlier SIGKILL,
  is an error (:827-828; I0-01). After a `CONDITION` line the attempt goes on: the certificate builder and its checker
  supply the witness (:816-817);
- an incomplete `r22b10_cert.py` result for which one of three tests holds (`scripts/runner.py:151-163`, :841-843):
  - (a) the child's log has a row with `timeout: true` (`12-low-height/scripts/r22b10_cert.py:889`);
  - (b) a `missing[].reason` contains one of the builder's deadline phrases `wall-clock budget exhausted`,
    `serialization budget exhausted` or `exceeded budget` (`r22b10_cert.py:67`, :72, :741, :860, :888, :909, :912).
    This covers the per-orbit deadline (:779-783, caught at :824-825);
  - (c) the result's `seconds` is at least s − min(10, s/10), where s is the `--seconds` value passed; when `seconds`
    is missing or not a finite number, the runner's own wall time of the child, which is never smaller, is used.
    s − min(10, s/10) is the builder's construction deadline (`r22b10_cert.py:866`). Its serialization deadline
    (:901) is later, and its serialization fallback records `seconds` after that deadline fired (:913-915), so (c)
    holds for every serialization expiry, also when no orbit was built and the copied reasons are not deadline
    phrases (:909-912). In the normal path `seconds` is taken before the serialization check passes (:896, :904).
  - One of (a), (b) or (c) marks the whole attempt `timeout`, even when another orbit failed for another reason,
    because the certificate's completeness is then unknown. (c) can also mark a failure that ended within the last
    min(10, s/10) seconds of the budget; that errs toward `budget`, never toward `stuck`.
  - The builder's two fixed size caps, `straight-line program node memory budget reached` (`r22b10_cert.py:123`) and
    `word-table generator memory budget reached before connectivity` (:316), are not clocks. They give the same result
    at the same state with any amount of time, so they give `certificate_needed` (acceptance) or `no literal closure`
    (development), with the reason `incomplete certificate within search budget` (`scripts/runner.py:844`). The
    builder's own reason stays in the attempt's `certificate.json.gz` under `attempts/`;
- after P1 and P2: a child's own BUDGET end, the kernel's `budget` with a clock cursor, and the runner's kill of a
  height or kernel call.

In acceptance mode only `jordan_target.py`'s fixed cap can still cut an attempt. Such cuts are counted in
`acceptance.timed_out_attempts`. A cut at the final state of a run that stops in the tail ends the tail `timeout` and
the run `budget`, so the verdict is `budget` (or an earlier match below: `component`, or `coverage_failure` when the
probe found a word), never `stuck`. A cut at a state where another rule or unit took the step does not change the
verdict.

**The verdict** (`acceptance.verdict`; `scripts/runner.py:166-179`, :1405-1422). The runner computes it at the end
of every run, component runs included, and the checker computes it again. It is the first match in this order:

1. `error` if the result is `error`;
2. `component` if `config.component` is set;
3. `stuck` if any step has `standing: certificate`;
4. `coverage_failure` if any step has `standing: fallback` or the probe found a word;
5. `budget` if the result is `budget`, or if the result is `stuck` and an attempt at the final state timed out;
6. `stuck` if the result is `stuck`;
7. otherwise the result (`solved`).

The attempts at the final state are those of the stuck tail's units and those of every height sub-phase or high
phase that ended at the final digest (`scripts/runner.py:1413-1420`).

**Stubs and component labels.**

- `scripts/rules_height.py` and `scripts/rules_kernel.py` set `STUB = True` and answer `End('not_applicable',
  'stub')` (`scripts/rules_height.py:10-15`, `scripts/rules_kernel.py:10-15`); `scripts/lattice_phase.py` does the
  same from `play` (:11-16).
- When the entry test holds, the runner reads a stub's answer as no step: the sub-phase or phase ends `stuck` with
  reason `stub`, and the run becomes a component run (`scripts/runner.py:636-641`). A plugin that is not a stub and
  answers `not_applicable` against its entry test is an error. The lattice stub's `not_applicable` keeps the
  composition path instead (rule 5).
- `config.component` keeps the first reason only (`scripts/runner.py:381-383`):
  - `bulk_budget`: a step or clock budget ended the bulk (:574-576);
  - `height_stub`, `high_stub`: a stub answered where its entry test held (:641);
  - `lattice_composition`: the lattice phase took the `verify_late.py` path (:1277).
- A component run keeps its counts, and its verdict is `component` unless the run ends `error`. Until P1 to P3 are
  integrated, any run whose state after the bulk has no piece above height 6 and some at height 4 or 6, or that needs
  the lattice phase, is a component run. Only a run whose bulk or tail ends at the solved digest can have the verdict
  `solved`.

Options added by RUN-3: `--literal-limit N` (default 10,000,000 letters; 0 turns rule 3 (b) off), `--seeds
atlas|default` (default `atlas`) and `--cert-seconds S` (default 3,600). In development mode every rule child keeps
the `--rule-seconds` limit (default 3,600); only the two lattice programs have none. An `--out` path that contains
`.json` anywhere once resolved is a usage error, reported before any file is written (RV1-07).

Options added by I0 (`scripts/runner.py:1512-1520`): `--mode` (above), `--lattice-restorer late|schedule11` (default
`late`), `--height-seconds` and `--height-cpu` (default 3,600 each), `--pair-shards` (default 4), `--kernel-seconds`
(default 3,600), `--kernel-wall-margin` (default 600) and `--high-steps` (default none; a step budget that ends the
high phase `budget`). Until P1 to P3 only `--mode` and `--high-steps` change what a run does. The height, kernel and
lattice options are recorded and handed to the stub plugins in `ctx.budgets` (`scripts/runner.py:592-599`), which
ignore them; the lattice path always runs `restore_lattice_late.py` (:1290-1292).

## 2. The record

A run directory holds `run.json` (rewritten atomically after every step), one word file per step (`{"word": [[grip,
item], ...], "input_digest", "output_digest"}`, gzipped above 100,000 letters), one unit file per tail step, the
child logs and the lattice phase output. Each step records its phase, rule, unit, word file, length in letters,
contract (`strict_prime` for tail steps; `fallback` for the fallback: the unit lands, no lattice piece goes off and
the off set shrinks), the digest and off count before and after, the left heights of the off pieces (bulk steps
record N instead) and its time. A stuck run records the state and every unit's attempts. The result is `solved`,
`stuck`, `budget` or `error`, and the totals give the twists of the bulk, the tail, the fallback and the lattice
phase. A `--table` start is stored as `start-table.json`, exported again from the loaded state (the same digest, not
necessarily the same bytes).

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

Since I0 the format is `runner-run/3` (`scripts/runner_api.py:20`). Every key of format 2 keeps its meaning, and
format 3 adds (`scripts/runner.py:427-463` unless noted):

- in `config`:
  - `mode` and `component` (null, or the first component reason of section 1);
  - `bulk_order` (the seven bulk rules, without reopen), `phase_order` and `subphase_order`;
  - in `budgets`, also `high_steps`, `height_seconds`, `height_cpu`, `kernel_seconds` and `pair_shards`; every clock
    that acceptance mode lifts is null there (`scripts/runner_api.py:124-127`; `scripts/runner.py:48-50`);
  - `clock_sentinel`: 10⁹ in acceptance mode, null in development mode;
  - `kernel {seconds, wall_margin, d19, max_steps: 1}` (`seconds` null in acceptance mode; `d19` is null in every
    record so far) and `lattice {restorer}`;
  - `scripts` also hashes `runner_api.py`, so 22 files before the plugins, and every file each plugin lists in its
    `SCRIPTS`: 25 files with today's stubs (`scripts/runner.py:68-75`, :434-435);
- in `phases`:
  - `height {result, steps, twists, start, end, sub: {r22d75, r22d81, h4}}`, each sub-phase with `{result, steps,
    start, end}`;
  - `high {result, steps, twists, start, end}`, and `lattice {result, steps, twists}`: after a played lattice phase
    `solved` with its steps and twists (:1368-1369); on the `verify_late.py` path the run's result, `solved` or
    `error`, with the restorer's twists and no steps (:1314-1316);
  - a sub-phase or the high phase that a plugin's `End` closed also has `reason`, `details` and `attempts`
    (:644-645);
  - `tail {result, steps}` keeps its shape, starts with `result: null` and can also end `not_applicable` or
    `timeout`; any phase that the run never reached keeps `result: null`, and so does a phase that an exception cut
    off after some of its steps, in an error run (the steps stand; section 3);
- in `totals`: `height_twists` and `high_twists`;
- on every step (bulk steps at :1480-1483, tail steps at :1169-1182, plugin steps at :713-718, played lattice steps
  at :1357-1361):
  - `subphase` (null outside the height phase);
  - `encoding`: `json`, or `ia16` for the lattice steps of P3 (below);
  - `word_sha256`: for a `json` word the sha256 of `json.dumps` of its letters; for an `ia16` word the sha256 of the
    uncompressed bytes;
  - `tree {file, sha256, size}` (below);
  - `standing`: `rule`, `certificate` (development mode: the gate's end table was loaded) or `fallback`;
  - outside the bulk also `contract`, `gate`, `attempts` and `table` (`tables/NNNNN.json`, the state after the step;
    :711-712, :1160-1164). A played lattice step has `attempts: []`, and only the last one has `table`, the end table
    (:1363-1365). Bulk steps keep `N` in `before` and `after`, and have no contract, gate or attempts;
  - on a closure whose V is a certificate: `construction: certificate` and `certificate_sha256`, in both modes;
  - a plugin step's own fields, which may not reuse a core key or name `segments`, `slp`, `certificate` or `node`
    (`ROW_KEYS`, :65-67; checked at :694-697 and, for lattice steps, :1334);
- every attempt, in a step, a stuck unit or a plugin's `End`, carries a boolean `timed_out` (:144-148, :629-630);
- `stuck` (the tail's only): the state, `phase`, `reason` (`no_rule` or `timeout`), `units` with their attempts and,
  in acceptance mode, `probe` when the probe found a word (:1241-1246);
- `acceptance {verdict, certificate_steps, fallback_steps, probe, timed_out_attempts}`, computed at the end of every
  run (:1405-1422).

Run results stay `solved`, `stuck`, `budget` and `error`. For example, `--scramble 10 7001 --mode acceptance` gives
nine bulk steps of one letter each; every later phase, the tail included, is `not_applicable`; the lattice phase is
not entered; the result and the verdict are `solved` (measured; a test of section 5).

**Step trees** (the owner's reading of the tutorial's content,
`docs/wiki/decisions/owner-decisions-2026-10-20-tutorial-acceptance.md:87-96`: every step is a tree of named parts that
expands to the twists played).

- `step-trees/NNNNN.json.gz` is gzip with mtime 0 of the compact JSON of the root part; `sha256` is taken over the
  uncompressed bytes and `size` is the number of parts (`scripts/runner_api.py:210-214`).
- A part is `{name, letters, parts: [part, ...]}` or `{name, letters, span: [from, to]}`. A span is a letter range of
  the step's word, end excluded, and the spans in depth-first order cover the word from 0 to its length. The depth is
  at most 64.
- A part may carry `inverse_of`: the index path from the root to an earlier part with the same number of letters;
  its letters are that part's letters inverted, in reverse order (for example V⁻¹ of a closure, which names V).
- A part may instead carry `same_as`: the index path to an earlier part with the same name and the same letters. This
  marks a macro the step uses again, so that the tutorial can name it once.
- No key is named `slp`, `node`, `segments` or `certificate`.
- The runner checks a tree's structure before it writes it, and a lattice step's tree after it reads it
  (`scripts/runner_api.py:179-207`, :217-223; `scripts/runner.py:706`, :1339-1341).
- Trees of today's steps: a bulk step is one span named by its rule (`scripts/runner.py:1478-1479`); a fallback step
  and a tail step without parts are one span; a closure or T1′ step has one span per part, W, V, the middle and V⁻¹,
  with V⁻¹ marked `inverse_of` V and, in a T1′ step, the middle W⁻¹ marked `inverse_of` W (:1187-1198); a plugin
  step has the proposal's tree, or one span named by its rule (:705); a played lattice step has the tree its plugin
  wrote.

**`ia16` words** (`scripts/runner_api.py:226-290`): `words/NNNNN.ia16.gz`, gzip with mtime 0 of little-endian
uint16 codes 60·grip + item, all below 36,000. Only the lattice plugin of P3 writes them, with `Ia16Writer`; the
runner reads each one back with `ia16_digest`, which recomputes its sha256 and length (`scripts/runner.py:1337-1338`).

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

`check_run.py` reads a record of format 1, 2 or 3 and the files it names, rebuilds the start (the census scramble, or
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

For a format 3 record (I0) it also checks:

- **Configuration**: `mode`, `component`, `literal_limit`, `seeds`, `kernel.max_steps` 1 and the lattice restorer; in
  acceptance mode every lifted clock null in `budgets`, `clock_sentinel` 10⁹ and `kernel.seconds` null; the phase and
  sub-phase orders; and a `bulk_order` within the seven bulk rules, so never reopen.
- **Step labels**: the step keys; the phase and sub-phase; the encoding (`ia16` for lattice steps); `word_sha256`;
  the standing (`fallback` exactly on fallback steps and only in development mode; `certificate` exactly on
  `slp_apply` steps in development mode); `construction: certificate` exactly on `slp_apply` steps, each with a
  `certificate_sha256`;
  every bulk rule in `config.bulk_order`; a boolean `timed_out` on every attempt; contract and gate `kernel_landing`
  or `kernel_lowering` and `r22d25` on high steps, and `lattice_literal` and `player` on lattice steps.
- **Step trees**: the file (gzip with mtime 0), its sha256 and its size; the structure of section 2; the root's
  length equal to the step's; and that the tree expands to exactly the replayed word. Every `inverse_of` range must
  equal the inverse of the range it names, letter by letter, through the menu's inverse table, and every `same_as`
  range the range it names, with the same name. A reference to a part that is not wholly before it, or a part with
  both keys, fails as `tree_malformed`. For a lattice step the checker checks the structure; packet P5 checks its
  ranges on the `ia16` word.
- **Phases in the fixed order**: a phase or sub-phase entered again fails. Every recorded start and end is compared
  with the checker's own snapshots, and every `not_applicable`, `handoff` and `lattice` claim is recomputed from them
  with the six entry tests of section 1, `h4`'s max ≤ 4 included. A `stuck` claim is not recomputed: that would need
  rule code. The results of the height and high phases and of the tail, and the lattice result of the composition
  path, must agree with the run's result; the result of a played lattice phase is not checked yet (the runner writes
  `solved`, the checker's fixture `lattice`).
  - A phase left out (a null result and no steps) must follow a result that ends the run. The checker tracks
    `ended`: it starts true only in an error run, and it becomes true after a `budget`, `timeout` or `error` at a
    position after the bulk, or after a stuck tail. A phase left out before that fails `phase_result`. The one other
    null position is the lattice phase of a run that ends `solved` (a state already solved) or `budget`
    (`--no-lattice`) (review finding I0-03; `scripts/check_run.py:730-745`).
  - A null result with steps passes only in an error run: an exception after a tail step, or after a step of a height
    sub-phase or the high phase, leaves that phase's result null with its steps standing. Such a sub-phase or high
    phase keeps the start it was entered at, which is compared with the checker's snapshot, and with steps its entry
    test must hold there; the tail must have had a piece off where it started. Steps without a start fail
    `phase_result`, and a failed entry test fails `entry_test` (scoped verification finding I0-V1-01;
    `scripts/check_run.py:617-629`, :692-694, :707-713).
- **The probe**: its word must not be empty, and every piece of its unit must be off at the final state. The word is
  replayed on a copy of the end state under the fallback's contract: afterwards no piece may be off unless it was off
  before and is outside the unit, so the unit lands, no lattice piece goes off and the off set strictly shrinks
  (review finding I0-04; `scripts/check_run.py:821-856` and :1288-1292, `fallback_kept` :597-600). A probe it cannot
  confirm, or a probe in development mode, fails as `probe`.
- **The acceptance block**: the counts of certificate and fallback steps and of timed-out attempts, the probe flag,
  and the verdict, recomputed in the order of section 1.
- **Hooks**: a height or high step goes through `check_contracts.py`: `prepare(row, st)` before the replay (the
  replay changes the live state), `on_letter(ctx, k, st)` after each letter and `finish(ctx, row, st)`, which returns
  the step's failures. A lattice step goes to `check_lattice.replay_lattice_step`, and an `ia16` word is read through
  `check_lattice.read_ia16`.
- **End digest**: a run with a played lattice phase is solved when the last lattice row has `labels_identity` true
  and no piece is off; its `end_digest` is the replayed digest, which may differ from the solved digest by rotations
  the labels do not see (the w9 case, `scripts/test_literal_lattice.py:637-638`). A run with component
  `lattice_composition` keeps the `verify_late.py` path below, and its `end_digest` is null until the runner plays
  the lattice (I1). Every other run ends at the replayed digest, and a solved one at the solved digest.

**`unchecked_contract`.** Until packets P4 and P5, `check_contracts.py` and `check_lattice.py` are stubs: every
height, high and lattice step reports `unchecked_contract`, and the stub of `check_lattice.py` plays a lattice word
on the exact simulator, which is right but far too slow for a real lattice word. An unchecked contract fails the
record unless `--allow-unchecked-contracts` is given. With it, the record can pass, each such step row says
`unchecked: true`, and the final row counts them in `unchecked_contracts`. The final row of a format 3 record also
gives the `verdict`.

**Independence** (the owner's first reading of 20 October 2026, decision page :74-76). The checker shares only the
definition of the puzzle: the model assets; the simulator (`sim/`); the I_a menu, `element` and `scramble` of
`theory/restore.py`; the trees; and the cut geometry that counts N (`rim.Domains(...).total_misalignment()`, the same N
as `theory/restore.py:175`). It shares no runner, rule, gate, player or verifier code. An AST scan in
`test_check_run.py` enforces this for `check_run.py`, `check_contracts.py` and `check_lattice.py`. It allows the
attributes `menu`, `element` and `scramble` of `restore`, and three names that `restore` re-exports from the
simulator: `ctx`, which is `rim.ctx = sim.get_context()` (`theory/restore.py:53`, `theory/rim.py:29`); `to_tuple`,
which is `sim.kplus.to_tuple` (`theory/restore.py:50`); and `transpose`, which is `exact.transpose`
(`theory/restore.py:49`), the exact arithmetic that `sim/` itself imports. `verify_late.py` runs only as a child
process.

For a solved run it runs `verify_late.py` on the lattice phase output, which checks the lattice word by composing
permutations, not by playing it (rule 5). The verifier now runs in its own process group with a time limit
(`--verify-seconds`, default 7,200), is stopped when the limit passes, and its report and output are saved in a new
directory `check/run-*/` inside the run directory, or next to `--out` (RV2-06). Those files can name absolute paths;
they are working files, not evidence to publish. The checker checks the words played, not the runner's choice of
rule or its budgets. A fallback step whose word is empty passes when its contract holds, as the record format allows
(RV2-05).

## 4. The literal lattice player and the runner's plugins

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

**The plugin interface** (`scripts/runner_api.py`, I0). The runner owns the state, the record and the phase order; a
plugin reads the current state and proposes one step, or ends its phase.

- A plugin is a module with `next_step(ctx)` (height and high) or `play(ctx)` (lattice), a list `SCRIPTS` of the
  repository-relative files it depends on, which `config.scripts` hashes, and, while it is a stub, `STUB = True`
  (`scripts/runner.py:51`, :385-389, :434-435).
- `Ctx` (`scripts/runner_api.py:90-112`; built at `scripts/runner.py:592-599`) holds:
  - `table`, `state`, `snapshot()` and `mechanics`: the current state and its table. A plugin reads the state; only
    the runner changes it;
  - `work_dir`, the call's directory `attempts/NNNNN-<sub-phase or phase>/`, and `cache`;
  - `execute(argv, tag, seconds, *, cwd=None, stop_when=None)`: the runner's child executor; `cwd` and `stop_when`
    are passed by keyword. Each child owns a process group; `seconds=None` means no clock; when `stop_when(line)` is
    true for a complete log line, the runner kills the group at once and returns `stopped`, so a plugin can take the
    first STEP line of a rule program that would go on searching (`scripts/runner.py:466-506`);
  - `budgets`, `mode` and `subphase`, with `wall(name)` (the runner's own clock for a call, None in acceptance mode)
    and `child_clock(name)` (section 1, "Clocks").
- `next_step(ctx)` returns one `Proposal` or one `End`. The runner calls it again after every step, until the entry
  test fails (`handoff`), no piece is off (`lattice`) or the plugin returns an `End` (`scripts/runner.py:611-641`).
- `Proposal` (`scripts/runner_api.py:34-46`): `phase`, `subphase`, `rule`, `word` (`{word, input_digest,
  output_digest}`, the claimed end digest included), `contract`, `gate`, an optional `tree`, `fields`, `standing`
  (default `rule`) and `attempts`.
- `End` (`scripts/runner_api.py:49-58`): `result` (one of the seven phase results), `reason`, `details` and
  `attempts`. Every attempt carries a boolean `timed_out`.
- `record_step` (`scripts/runner.py:691-720`) checks the proposal against its phase and the record's keys, takes a
  snapshot, plays the word twist by twist (`apply_word`), requires the claimed end digest, writes the word file and
  the step tree (one span named by the rule when the proposal has none), exports `tables/NNNNN.json`, appends the row
  and saves.
- The lattice plugin's `play(ctx)` returns a `LatticeResult` or an `End`. Today it is a stub that answers
  `not_applicable`; the runner then takes the `verify_late.py` path of rule 5 (`scripts/runner.py:1264-1277`). Any
  other `End` is an error until I1.
- `LatticeResult` (`scripts/runner_api.py:82-87`): `steps`, `end_table` and `report`. Each step is a `LatticeStep`
  (`scripts/runner_api.py:61-79`): `rule` (`lattice/...`), `word_file`, `word_sha256`, `letters`, `tree_file`,
  `tree_sha256`, `tree_size`, `before` and `after` (lattice snapshots), `fields` and `seconds`, with `contract`
  `lattice_literal`, `gate` `player`, `standing` `rule` and `encoding` `ia16`. The runner checks and records it as rule
  5 says (`played_lattice`, `scripts/runner.py:1319-1372`; review finding I0-02).
- The tree helpers `span`, `group`, `flat`, `single`, `check_tree`, `write_tree` and `read_tree`, the `ia16` writer
  `Ia16Writer` and its reader `ia16_digest` are in `scripts/runner_api.py:130-290`.

**What packets P1 to P3 add** (runner side, each in its own files; P4 and P5 are the checker's, section 3). Each
packet replaces a stub, runs the existing rule programs unchanged as child processes through `ctx.execute`, and maps
every way they can end to the phase results of section 1. None of them has been implemented yet.

- **P1, the height phase** (`rules_height.py`): `r22d75_from_snapshot.py`; `r22d81_rule.py`, then `r22d81_pairs.py`
  with the target cap from R22D-81's STUCK line (`--target` is required,
  `12-low-height/scripts/round22/r22d81_pairs.py:45`); `h4_unlock.py`, then the `two_letter_u.py` shards. Each call
  stops at the first STEP line.
  - HEIGHT2 and HANDOFF end the sub-phase `handoff`; STUCK `stuck`; BUDGET `budget`; the runner's kill `timeout`.
  - STUCK with no target cap, UNLOCKED before any step, and every pair shard done with no success end it `stuck`, with
    the reasons `no_target`, `unlocked_without_step` and `pair_exhausted`.
  - A crash, a missing STEP, a digest mismatch or truncated output is an error. Truncated output is never read as
    exhaustion (R22D-55).
  - A height step records τ before and after, and u; a pair step's tree has two parts.
- **P2, the high phase** (`rules_kernel.py`): `r22d25_search.py run ... --d19 round22/r22d19_rule.py --max-steps 1`,
  one kernel step per call.
  - One step with result `lattice`, or with result `budget` and a `max_steps` cursor, is a step.
  - With no step: the kernel's own `budget` is `budget`; `stuck` is `stuck` with reason `kernel_exhausted`, which
    covers only the catalogue `r22d25-cat/1`; `unsupported` is an error (the kernel's `d19_missing`, or its
    `height_above_6`, recorded as `invariant_height_above_6` because the high entry test excludes it;
    `round22/r22d25_search.py:329`, :395).
  - Contract `kernel_landing` or `kernel_lowering`, gate `r22d25`, and the kernel row's own fields; an ST step's tree
    has the parts S and T.
- **P3, the lattice phase** (`lattice_phase.py`, and one keyword in `literal_lattice.py`): `play(..., observer=None)`.
  With `None` the player behaves byte for byte as today; with an observer, poses move twist by twist and the observer
  receives every twist.
  - The plugin runs the restorer (`late`, or `schedule11` with its realignment data), plays the word in a child
    process and streams it as `ia16`.
  - It makes one step per cascade orbit and one per correction macro, with exact digests at every boundary, contract
    `lattice_literal`, gate `player` and `labels_identity` on the last row.
  - Each step's tree follows the program's nodes. In a `conj` node u^(−us)·x^(sign)·u^(us), the trailing u^(us)
    names the leading u^(−us) by `inverse_of`. A subtree that would take a step's tree over 10⁶ parts is folded into
    one span.
  - It requires the result `solved`, every scheduled twist played and identity labels, exports the end table and
    returns the steps as a `LatticeResult`.

## 5. Tests

- `scripts/test_runner.py` (RUN-1's acceptance check, extended by RUN-3 and by I0). Before I0 it had 34 tests, one of
  them skipped (the depth 20 run to the solved state, which needs `RUNNER_SLOW=1`). Before the integrator's fixes they
  passed in 3,142 s in Codex's run and in 1,526 s in the acceptance run. They include two exact certificate steps at
  states of E2's literal replay (stage 12): the closure for b20 at the state of 507 off (`T_size` 111, V of 20,232
  letters, 40,500 letters in the step, 507 → 447 off; 742 s in Codex's run, 395 s in the acceptance run) and the T1′
  step for b07 at the state of 324 off (W of 48 letters, V of 139,774, 279,644 letters in the step, 324 → 213 off; 962
  s and 538 s), each with the lengths, hashes and end digest of the packet. Both run in development mode. In both
  tests the core or W is E2's, given to `close` and `t1prime` directly; whether the runner's own rules, in unit order,
  find these steps at those states is not tested. The suite also has RUN-1's stored T1′ step for b04 (1,254 letters),
  now through `advance()` with the real mechanics; the small lattice configuration, now with a one-macro library; and
  stub tests for the certificate path, signals, the `--par` retry, the source identity and `--out`. On 20 October
  2026, with the fixes and before I0, in this checkout: OK, 34 tests, 1 skipped (the depth 20 test, which needs
  `RUNNER_SLOW=1`), 1,222 s, worktree unchanged (headless Linux). I0 adds 41 tests, 11 of them with the fixes of
  its review (75 in all, one skipped):
  - one per row of the phase result table, `timeout` of the tail included, with scripted plugins and fake geometry;
    a component run that ends stuck has the verdict `component`; a stuck run with a timed-out attempt at its end has
    `budget`; the verdict order; a probe word gives `coverage_failure`; `record_step` exports the table and writes a
    tree; the bulk order has no reopen;
  - step trees: a closure's tree names V and its inverse; malformed trees are refused before they are written; the
    written file and its sha256;
  - `execute`: `stop_when` kills the group after the first STEP line; a cancel still works with `stop_when`; with no
    clock a child runs to its end;
  - A2 on the certificate stub: in acceptance mode a complete certificate closure is played through `apply_word` and
    the gate's end table is never loaded (`standing: rule`, `construction: certificate`); with `--literal-limit 4`
    (the five-letter closure is over it, and nothing is expanded), with an incomplete certificate whose only reason is
    not a deadline, and with a checker rejection (exit 1 and a `FAIL:` line), the attempt is `certificate_needed`;
    development mode keeps the gated end table, and its certificate steps make the verdict `stuck`;
  - the clock tests: the adapter (the sentinel and null budgets in acceptance mode, the values in development mode);
    `tclose2.py`'s greedy clock (a timed-out attempt in development mode, none in acceptance mode); `jordan_target.py`'s
    cap before `CONDITION` (its line, exit 124, or a SIGKILL after 850 s: a timed-out attempt; an exit 0 without
    `CONDITION`, or a SIGKILL after 5 s: an error); the certificate deadlines (per orbit,
    serialization, mixed and an explicit `timeout` row give `timeout` and the verdict `budget`; a reason that is not
    a deadline, and a size cap, give `certificate_needed` and `stuck`); and two certificates emitted by the unchanged
    `r22b10_cert.main` with a fake clock, one past its serialization deadline (`timeout`, `budget`) and one not
    (`certificate_needed`, `stuck`);
  - with real geometry: a `--table` start at the R22D-36 state with scripted height plugins (run A: `r22d75` stuck,
    then an `r22d81` step to `7cd868f6…`, where `r22d75` would fire but is not entered again; run B starts there, and
    its first height step is an `r22d75` step); both records pass the format 3 checker with
    `--allow-unchecked-contracts`. A tampered tree of run B fails it: a wrong part sum, a gap, an overlap, a root
    shorter than the word, a wrong `inverse_of` range, an `inverse_of` that names a later part, a wrong sha256;
  - the fixes of the I0 review (section 7). `IntegrityTests` (I0-01), six tests: a crashing `stepchk.py` ends the run
    `error` before `close` or the fallback runs, in development mode with `--par 1` and `--par 2` and in acceptance
    mode, and so does a failed assert of `cellpure5.py` that is not a screen; a mismatched check digest ends it `error`
    at the pure rule, the certificate gate (also when the step is not strict′), the fallback and the probe; no word
    and an applicability screen stay attempts, and every rule is tried at every unit; an error leaves `choose` and its
    serial retry; a later unit's error waits for the earlier unit's serial retry, so `--par 1` and `--par 2` give the
    same record; an error after a tail step keeps the step. `LatticeTests` (I0-02), three tests with fake geometry: a
    played lattice is recorded with its end table; six mismatches (the end table, a malformed tree, a word sha256, the digest chain, the labels,
    no step) end the run `error` before anything changes; every lattice `End` but the stub's is an error until I1.
    With real geometry, `ExactTests.test_09`: a scripted plugin in P3's interface plays the inverse of the small
    lattice configuration's word in two steps, and the record passes the format 3 checker with
    `--allow-unchecked-contracts` (end digest solved, 10 lattice twists, 2 unchecked contracts); `test_10`: four
    lattice `End`s give error records that the checker passes. Three tests changed: the certificate gate (the
    checker's rejection, an incomplete certificate and a false strict′ are attempts; digest, hash, length and segment
    mismatches, a rejection that names a digest, truncated checker output and a missing summary line are errors),
    `tclose2.py`'s parts (a word of another form is an error) and `jordan_target.py`'s cap (above).

  On 20 October 2026, with I0 and its review fixes, in this checkout: OK, 75 tests, 1 skipped (`RUNNER_SLOW=1`), 1,140 s, worktree unchanged (headless Linux).
- `scripts/test_check_run.py` (RUN-2's acceptance check, extended by RUN-4 and by I0). Before I0 it had 66 tests.
  Before the integrator's fix they passed in 1,604 s in Codex's run and in 1,632 s in the acceptance run. RUN-2's
  tests are kept (the format 1 lattice fixture is now built from a format 2 run); new positive and negative tests
  cover the format 2 checks and the verifier's time limit, with the gaps listed in section 7. On 20 October 2026, with
  the fixes and before I0, in this checkout: OK, 66 tests, 482 s (headless Linux). I0 adds 31 tests, one of them with
  the fixes of its review and two with the fix of its scoped verification (97 in all):
  - format 3 records: a bulk record; the configuration in both modes; the bulk order (reopen fails); the step labels;
    a closure tree whose V⁻¹ names V and whose middle names W; tree failures; `same_as`; height steps with unchecked
    contracts; the hooks (`prepare` before a step, `on_letter` after each letter, `finish` after the last); entry
    tests recomputed from the checker's snapshots; the phase order; phases left out (I0-03: the review's
    counterexample and every prefix that does not end the run fail; prefixes that end it, an error record cut short,
    an error after a tail step and `--no-lattice` pass); a phase cut off by an exception after its steps (I0-V1-01:
    it keeps its start and its entry test; a tail step recorded with no piece off fails), also on stand-in snapshots;
    `h4` needs max ≤ 4; the six entry tests at their boundaries; the verdict with a timed-out attempt at the final
    state (`budget`) and only before it (`stuck`); the verdict order; a development fallback step
    (`coverage_failure`); the probe (I0-04: a unit on the lattice, with the empty word or with the probe's word,
    fails); a played lattice; a played lattice
    solved by identity labels with a digest that is not the solved one; the composition path (a component, end digest
    null);
  - records written by the real `runner.py` with the stub plugins: `--scramble 10 7001` in acceptance and in
    development mode (bulk only, `solved`, verdict `solved`, end digest the solved one, no unchecked contract), also
    through the command line; the R22D-36 state with no bulk and no tail rule (a component, with `h4` not applicable at
    max 6) and mutations of it; reopen in `config.bulk_order` or in a step;
  - the independence scan of section 3, and a test that the scan finds forbidden imports and attributes.

  On 20 October 2026, with I0 and its review fixes, in this checkout: OK, 97 tests, 818 s, worktree unchanged (headless Linux).
- No I0 test runs a real height, kernel or lattice plugin: those are packets P1 to P3. The played lattice is tested
  with scripted plugins in P3's interface only. Fixture times that the packets
  rely on, measured by I0 on 20 October 2026 (headless Linux, four cores, two other single-threaded jobs running):
  the kernel with `--seconds 1` at the R22D-36 state, 18.6 s (result `budget`, no step); `eb6.straddler_set` on all
  600 caps at the R22D-36 state, 2.5 s (6,489 straddlers), and at the seed 7102 stop, 18.9 s (29,212 straddlers);
  `r22d75_from_snapshot.py` at the R22D-36 state, from (12, 14) to HEIGHT2 in two steps, 13.6 s.
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

- The stubs: the height, high and lattice plugins are stubs until packets P1 to P3, and the checker's height, kernel
  and lattice contracts until P4 and P5. With them, a run whose state after the bulk passes a height or high entry
  test (no piece above height 6, and some at height 4 or 6), or that reaches the lattice phase, is a component run
  (`scripts/runner.py:636-641`, :1274-1277), and every height, high or lattice step that a test scripts is
  `unchecked_contract` in the checker (sections 1 and 3).
- The played lattice phase, until P3 is integrated (I1). Until then the lattice phase is not played twist by twist in
  a run: `verify_late.py` evaluates the retained factors of the lattice word through their net sticker permutations
  (lines 344 to 348) and plays only the corrections on the simulator (lines 351 to 374), and `runner.py` takes solved
  from its report (`scripts/runner.py:1309-1314`; lines 908 to 913 at commit `31b78a5`; R22D-57 cites lines 645 to
  650 of RUN-1's `runner.py`). Such a run is a component (`lattice_composition`), and the checker gives it no end
  digest. The player of section 4 plays the whole word literally, but the runner does not call it, and it has not
  been run on a runner record (its test on the committed depth 20 record needs `LATTICE_SLOW=1` and was not run).
  The runner's side of a played lattice phase is in place (rule 5, I0-02), but only scripted plugins have used it.
- A lattice phase that ends `stuck`, `budget` or `timeout`. Until I1 every lattice `End` but the stub's ends the run
  `error`, and the checker has no rules for such an `End` (deferred, section 7).
- `--lattice-restorer schedule11` is recorded in `config.lattice.restorer` but not used yet: the `verify_late.py` path
  always runs `restore_lattice_late.py` (`scripts/runner.py:1290-1292`).
- A finer tree for bulk `conj` steps. The bulk child reports only words, so every bulk step's tree is one span named
  by its rule (`scripts/runner.py:1478-1479`); a `conj` step's seed and conjugators are not named.
- `ia16` words in `record_step`: it writes every word as JSON, gzipped above 100,000 letters
  (`scripts/runner.py:680-684`, :709). Only the lattice plugin of P3 will write `ia16` words, which the runner moves
  into the record (rule 5).
- E2's certificate steps: rule 3 (b) builds and expands a certificate V at the state where it acts, but it is tested
  only at two states of E2's literal replay, not on a run from a scramble, and no runner record with a certificate
  step exists outside the tests. In both tests the core or W is E2's, given to `close` and `t1prime` directly; whether
  the runner's own rules, in unit order, find these steps at those states is not tested. E2's b06 step also needed a
  three-letter SAFE-ALL preparation, (178, 7)(170, 1)(75, 1), before its T1′ step (stage 12, `round22/e2_b06_*.py`);
  the runner has no rule that makes such a preparation. In acceptance mode such steps are played twist by twist (A2):
  E2's 13 literal steps from 737 to 0 total 2,012,155 twists (`12-low-height/README.md:936`).
- Height 4: the R22D-25 kernel of stage 12 plays R22D-25 with R22D-37's height-6 tier over a whole stop, under the
  practical catalogue `r22d25-cat/1` (S and T of one letter; R22D-60's pass left out). By `PROOFS-B.md` R22D-55 this is
  a revision of the rule, so its stuck outcome covers only that catalogue. `check_r22d25.py` checks its records. The
  runner's high phase is where the kernel will run, one step per call, but until packet P2 its plugin is a stub. The
  kernel's direct family R22D-19 is an external program (`round22/r22d19_rule.py`, D19 of stage 12), which was not in
  the repository when the kernel was written and tested; the kernel calls it by default but has not been run with it
  at a stop (its tests use stub programs), and without it, at a state with height-4 units where the height-6 tier has
  no step, the kernel reports `unsupported` before it tries S·T.
- The round 11 lattice schedule has a restorer for real lattice configurations (stage 09,
  `schedule11/restore_schedule11_late.py`, with its test outcome recorded there); the runner does not call it.
- It decides nothing about coverage: a solved run shows that the rules reached the solved state from that start.
- The acceptance test is fixed (owner, 20 October 2026,
  [decision](../../../../docs/wiki/decisions/owner-decisions-2026-10-20-tutorial-acceptance.md)): 21 scrambles that
  development never used, three fresh seeds at each of the depths 10, 20, 40, 100, 300, 1000 and 3000 (seeds 9011 to
  9073), run in batches from shallow to deep, every word played twist by twist, the lattice phase included, and every
  record replayed by the checker twist by twist. None of them has been run.

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
  (RUN3-false-pass-02; since format 3 an incomplete certificate that ended on its clock is a `timeout` attempt, and
  such an attempt is retried, section 1); a run killed in a way the runner cannot catch keeps the placeholder `budget`
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

**The fast review of I0** (call 20261020T143305Z-cbd105e8) found four findings, all `major`. The integrator adopted
all four. Each fix was drafted on its own and checked by two independent adversarial verifiers, with experiments on
real geometry, before it was integrated:

- I0-01, a crash or a digest failure in the tail could give a `stuck` verdict: `RunError` for a crash, truncated or
  malformed child output and every digest mismatch, which ends the run `error` at once; applicability screens, no
  word and every clock stay attempts; `jordan_target.py`'s cap is exit 124 or a SIGKILL after 840 s; under `--par` a
  later unit's error waits for the earlier serial retries (section 1, "Errors and failed attempts"). Tests:
  `IntegrityTests`, and the changed certificate gate, `tclose2.py` parts and `jordan_target.py` tests (section 5).
- I0-02, the runner could not take a played lattice result: `LatticeStep` and `LatticeResult` in `runner_api.py`, and
  `played_lattice`, which re-reads and checks every word, tree and digest and the end table before it records
  anything; every other lattice `End` is an error until I1 (rule 5). Tests: `LatticeTests`,
  `ExactTests.test_09_played_lattice_passes_the_checker` and `test_10_lattice_end_error_passes_the_checker`.
- I0-03, the checker accepted phases left out without a result that ends the run: the `ended` rule (section 3).
  Test: `test_check_run.py` `test_v3_omitted_phases`.
- I0-04, an empty probe at a unit on the lattice could give `coverage_failure`: the probe needs a non-empty word for
  a unit that is off, and the off set must strictly shrink (section 3). Tests: two new cases of `test_v3_probe`.
- One more change came from the adversarial check of the drafts (V1-B2): an exception after a tail step, or after a
  step of a height sub-phase or the high phase, leaves that phase's result null with its steps standing, and the
  checker accepts that only in an error run (section 3). Tests:
  `IntegrityTests.test_an_error_after_a_tail_step_keeps_the_step` and `test_v3_omitted_phases`.

The first scoped verification round of these fixes (call 20261020T164710Z-c78e765d) found I0-02, I0-03 and I0-04
fixed, I0-01 not yet fixed, and one new `major`, I0-V1-01; the integrator adopted both. I0-V1-01:
the checker accepted a phase that an exception cut off without checking where it started; such a phase now keeps its
start, which must match the checker's snapshot, and its entry test must hold there (section 3). Tests:
`test_v3_interrupted_phase_entry` and `test_interrupted_plugin_phase_rules`. At the certificate gate, a false strict′
hid a digest mismatch; the runner now checks the gate's evidence first, and only a well-formed false strict′ is an
attempt (section 1, rule 3). Test: a new case of `IntegrityTests.test_b_mismatched_check_digest_ends_the_run_error`.
The second and last scoped verification round (call 20261020T171924Z-d02fa182) found I0-01 and I0-V1-01 fixed and no
new `blocker` or `major`: verdict pass.

Deferred by decision:

- a lattice phase that ends `stuck`, `budget` or `timeout`, and the checker's rules for it (with packet P3 and stage
  I1; until then such an `End` gives an error record);
- recording `--no-lattice` in `config`;
- evidence for a claimed `budget` or `timeout` (a timed-out attempt or a reached step budget);
- the development-mode fallback branch of the checker's `check_unit`, which does not require an off unit and a
  non-empty word;
- `failed_assert` on coloured tracebacks (`FORCE_COLOR`);
- the result of a played lattice phase, which the checker does not yet compare (the runner writes `solved`); with
  packets P3 and P5.

## Replay

From the repository root, with Python 3 and NumPy, `S=research/jumbling/restoration/13-tutorial-runner/scripts`:

    python3 -I -B $S/test_runner.py                       # times in section 5
    python3 -I -B $S/test_check_run.py                    # times in section 5
    python3 -I -B $S/test_literal_lattice.py              # times in section 5
    python3 -I -B $S/test_check_r22d25.py
    python3 -I -B $S/runner.py --scramble 10 7001 --out <new dir> --cache <new dir> [--mode acceptance]
    python3 -I -B $S/runner.py --scramble 20 7106 --out <new dir> --cache <new dir>   # about 11 minutes
    python3 -I -B $S/check_run.py <run dir> [--verify-seconds S] [--allow-unchecked-contracts]
    python3 -I -B $S/literal_lattice.py <run dir>/lattice <new file>.json --start-table <run dir>/<lattice input table>
    python3 -I -B $S/check_r22d25.py <kernel run dir> --cache <new dir>

The runner now writes format 3 records. `<lattice input table>` is, for a format 2 or 3 record, the table that
`lattice.input` names in `run.json`; a format 1 record has no `lattice.input`, and its lattice phase's
`restore-summary.json` names the table (for the committed depth 20 record `tables/bulk-end.json`). The committed
records leave their tables out, so run the runner again first, as `LATTICE_SLOW=1` does: before
`test_literal_lattice.py` it runs the runner on the depth 20 scramble again and plays the committed record's lattice
word (not run). The `--out` path of the runner must not contain `.json`.

## Files

| Path | What it is |
| --- | --- |
| `scripts/runner.py` | The tutorial runner (RUN-1; RUN-3 on 20 October: the certificate path of rule 3, record format 2, signals, run-relative lattice paths and the source identity), with the integrator's fixes after review: the lattice programs run without a time limit again (RUN3-contract-01, RUN3-robust-01), the bulk child watches the runner's process id (RUN3-robust-02), and the source identity is taken before the run writes, with git run without optional locks (RUN3-contract-04, RUN3-robust-03, RUN3-robust-06). I0 on 20 October: record format 3, the fixed phase order with its entry tests and phase results, the bulk without reopen, acceptance and development modes, the clocks and timed-out attempts, the probe, the verdict, step trees and the plugin calls; the fixes of the I0 review: `RunError` for crashes, malformed output and digest mismatches, and the runner's side of a played lattice phase (I0-01, I0-02); and of its scoped verification: the certificate gate checks its evidence before strict′ |
| `scripts/runner_api.py` | The plugin interface (I0): `Proposal`, `End`, `LatticeStep`, `LatticeResult`, `Ctx`, the clock adapter, the step-tree helpers and the `ia16` writer and reader (section 4) |
| `scripts/rules_height.py`, `scripts/rules_kernel.py`, `scripts/lattice_phase.py` | The plugins of the height, high and lattice phases: stubs (I0) until packets P1 to P3 |
| `scripts/test_runner.py` | Its acceptance tests (RUN-1, extended by RUN-3 and by I0); the integrator made the worktree check also skip the private work trees, tolerate files that vanish and run git without optional locks, and made a run over 60 minutes a note, not a failure (RUN3-robust-05) |
| `scripts/check_run.py` | The independent record checker (RUN-2; RUN-4 on 20 October: both formats, the format 2 checks, a time limit and saved output for `verify_late.py`; I0 on 20 October: format 3, with the fixes of its review for phases left out and for the probe (I0-03, I0-04) and for a phase that an exception cut off after its steps (V1-B2), and of its scoped verification for that phase's start and entry test (I0-V1-01)), with two fixes by the integrator after RUN-2's review: every string in the record is scanned for an absolute, home-relative, Windows or parent path (RV2-01), and a corrupt gzip body fails as the file it is in (RV2-04) |
| `scripts/check_contracts.py`, `scripts/check_lattice.py` | The checker's height and kernel contracts and its literal lattice steps: stubs (I0) until packets P4 and P5; `check_lattice.read_ia16` is complete |
| `scripts/test_check_run.py` | Its acceptance tests (RUN-2, extended by RUN-4 and by I0, with the independence scan of section 3); the timeout of the lattice fixture was raised from 1,100 to 3,600 seconds by the integrator on 19 October after the first acceptance run timed out on a loaded machine, and on 20 October its worktree check was made to leave out the private work trees and to run git without optional locks (RUN4-robust-05) |
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
- A runner run with a real height, high or played lattice phase: packets P1 to P3 are not implemented.
- The fast review of the fixes covered the fixes only, not the whole patches. The fixes for IF-01 to IF-03 (in
  `test_literal_lattice.py` and stage 09's `test_sched11_late.py`) and the fix to `test_check_run.py`
  (RUN4-robust-05) came after it and have had no review.
