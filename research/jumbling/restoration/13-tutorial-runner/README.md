# 13. The tutorial runner: a machine that follows the rules (19, 20 and 22 October 2026)

On 18 October 2026 the owner set the tutorial's acceptance test: a machine that follows it step by step, using only
its rules, takes a scramble to the solved state, playing only literal words, twist by twist; a step known only by a
certificate counts as stuck, and every step's length is recorded as it is
([decision](../../../../docs/wiki/decisions/owner-decisions-2026-10-18-machine-tutorial.md)). This stage holds that
machine, its phase plugins, its checker with two helpers, the literal lattice player that the lattice plugin runs, a
checker of the kernel's own records, the plan and the frozen acceptance options:

- `scripts/runner.py`: one state-only program that applies the rules the tutorial teaches, in a fixed order of
  phases, and writes one record of every word it plays (format `runner-run/3` since 20 October; section 2);
- `scripts/runner_api.py`: the interface between the runner and its phase plugins, and the writers and readers of step
  trees and `ia16` words (section 4);
- `scripts/rules_height.py`, `scripts/rules_kernel.py` and `scripts/lattice_phase.py`: the plugins of the height,
  high and lattice phases. Codex wrote them under packets P1 to P3. Applied at the start of I1, they replace the stubs
  of I0 (section 4). The height plugin runs the height rule programs of stage 12. The high plugin runs the R22D-25
  kernel with D19, one step per call. The lattice plugin runs the lattice restorer (`late` or `schedule11`) and then
  the literal player, each in a child process;
- `scripts/check_run.py`: an independent checker that replays a record of any of the three formats twist by twist on
  the exact simulator, without the runner or its search code (section 3). It replays a played lattice phase through
  `check_lattice.py`. Records of formats 1 and 2, and format 3 records made with a stub lattice plugin in tests, take
  the lattice phase by composition. The checker checks those through `verify_late.py`, which composes the permutations
  of the lattice word's retained part instead of playing it. Since I1 its two helpers are real, not stubs:
  `scripts/check_contracts.py` (the height and kernel contracts, packet P4) and `scripts/check_lattice.py` (literal
  lattice steps, packet P5);
- `scripts/literal_lattice.py`: the literal lattice player that stage 12 asks for (`PROOFS-B.md` R22D-57). It plays
  a lattice word's retained part and its corrections twist by twist and records the played lengths and digests
  (section 4). Since I1 the lattice plugin runs it in its child, with an observer that cuts the word into one step per
  orbit of the cascade and one per correction;
- `scripts/check_r22d25.py`: an independent checker of the records of the R22D-25 whole-stop kernel of stage 12
  (`r22d25-run/1`). It replays every accepted word twist by twist and imports no kernel module. Since I1 the high
  plugin runs the kernel one step per call, in the call's work directory. The kernel's own `r22d25-run/1` record stays
  there, and the step keeps that record's sha256 (`scripts/rules_kernel.py:137-143`, :171-173, :186-187, :208-214).
  `check_run.py` replays and checks those steps. It does not run this checker;
- `acceptance-config.json`: the options every acceptance run uses (format `acceptance-config/1`; plan revision 3e,
  E3). `--mode acceptance` refuses any other value (section 2);
- `PLAN.md`: the plan for record format `runner-run/3`: revision 3d of 20 October, then the revision 3e addendum of
  22 October;
- `packets/`: packets P1 to P5 as sent to Codex, their patches and reports, and the integrator's review of the five
  patches (`REVIEW.md`; section 8).

Codex wrote the runner and the checker under two packets on 19 October (RUN-1, the runner; RUN-2, the checker,
written in parallel from the shared record format). On 20 October Codex wrote four more patches in parallel: RUN-3
(the runner's certificate path and record format 2), RUN-4 (the checker for both formats), LLP (the literal lattice
player) and R22D-25 (the kernel, in stage 12, and its checker here). The integrator reviewed every patch and fixed
the findings that block (section 7). Later on 20 October the integrator wrote part I0 of the plan for record format
`runner-run/3` (revision 3d, which passed the senior plan check; `PLAN.md`). I0 added the fixed phase order and its
plugin interface, acceptance and development modes, the clocks, and a tree of named parts for every step. It covered
the runner's side and the checker's, with stub plugins. A fast review of I0 found four major gaps, which the
integrator fixed the same day (section 7). The same day Codex implemented packets P1 to P5, each in its own worktree.
P1 to P3 are the height, high and lattice plugins. P4 and P5 are the checker's contracts and its literal lattice
replay. The integrator's review of the five patches confirmed 22 findings, one of them `major`
(`packets/REVIEW.md`). The owner then stopped the jumbling research once the work in flight was done, with the five
patches unapplied ([decision](../../../../docs/wiki/decisions/owner-decisions-2026-10-20-research-stop.md)).

On 22 October 2026 the owner restarted the work for the tutorial and its acceptance test, to the standard of a
finished product ([decision](../../../../docs/wiki/decisions/owner-decisions-2026-10-22-tutorial-first.md)). The
five patches were applied as reviewed (commit `8ae8c4a`). The integrator then carried out part I1 under revision 3e
of the plan. Revision 3e is an addendum to 3d and passed a scoped senior plan check (call 20261022T015422Z-4acc2d97;
`PLAN.md`). Codex implemented one more packet, FIX (call 20261022T015515Z-4f47c9d6). FIX is disjoint from the runner
and `check_run.py`. It fixes the review findings that lie inside the plugins, `check_lattice.py` and the packet tests
(plan revision 3e, E1). Among them is the `major` finding P3-falsepass-01. Its acceptance check, the five packet test
files, passed in Codex's sandbox. The integrator applied it and changed two of its test files for I1. The integrator
did the runner's and `check_run.py`'s parts of the fixes itself.

After I1 the plugins and the checker's helpers are real. The runner records the lattice phase that the plugin plays,
with the plugin's report. It also records a lattice `End` and a lattice phase with no step. Acceptance mode reads the
frozen `acceptance-config.json`. `config.kernel.d19` is recorded, and a bulk `conj` step has a tree of three parts.
The checker applies the kernel's phase bound, the played lattice result and the new record fields (sections 2 and 3).
The development runs (D0 so far) are in section 5. Research code, headless Linux; nothing here is Windows, DirectX or
performance evidence.

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

A run directory holds:

- `run.json`, rewritten atomically after every step;
- the start table `start-table.json`;
- one word file per step (`{"word": [[grip, item], ...], "input_digest", "output_digest"}`, gzipped above 100,000
  letters). A played lattice step's word is an `ia16` file (below);
- one step tree per step (`step-trees/`);
- one unit file per tail or fallback step;
- the tables (`tables/`): the bulk's end table, the state after each tail, fallback, height or high step, and the
  lattice phase's end table;
- the child logs;
- the work directories (`attempts/`): `attempts/NNNNN/` for each round of the tail, and
  `attempts/NNNNN-<sub-phase or phase>/` for each plugin call (`scripts/runner.py:633-637`, :1269).

When the plugin plays the lattice phase, with steps or with none, the runner keeps the plugin's report as
`lattice/report.json`. The restorer's output, the player's result and copies of the start table and the lattice
library stay in `attempts/NNNNN-lattice/` (10.9 MB for the depth 20 run of D0, measured). Each step records:

- its phase, rule, unit, word file and length in letters;
- its contract: `strict_prime` for tail steps, or `fallback` for the fallback (the unit lands, no lattice piece goes
  off and the off set shrinks);
- the digest and off count before and after;
- the left heights of the off pieces (bulk steps record N instead);
- its time.

A run that stops stuck in the tail records the state and every unit's attempts. The result is `solved`, `stuck`,
`budget` or `error`. The totals give the twists of the bulk, the tail, the fallback and the lattice phase. A `--table`
start is stored as `start-table.json`, exported again from the loaded state (the same digest, not necessarily the same
bytes).

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

Since I0 the format is `runner-run/3` (`scripts/runner_api.py:20`). Every key of format 2 keeps its meaning. Format 3
adds the keys below (`scripts/runner.py:445-505` unless noted). Keys marked I1 were added in part I1:

- in `config`:
  - `mode` and `component` (null, or the first component reason of section 1);
  - `bulk_order` (the seven bulk rules, without reopen), `phase_order` and `subphase_order`;
  - in `budgets`, also `high_steps`, `height_seconds`, `height_cpu`, `kernel_seconds` and `pair_shards`. Every clock
    that acceptance mode lifts is null there (`scripts/runner_api.py:124-127`; `scripts/runner.py:51-52`);
  - `clock_sentinel`: 10⁹ in acceptance mode, null in development mode;
  - `kernel {seconds, wall_margin, d19, max_steps: 1}` (`seconds` null in acceptance mode);
  - `lattice {restorer}`: the restorer the lattice plugin runs, `late` or `schedule11`;
  - `kernel.d19` (I1; plan revision 3e, E2): `{file, sha256}` of the D19 rule that the high plugin passes to the
    kernel. That rule is `D19` in `scripts/rules_kernel.py:21`, the file `12-low-height/scripts/round22/r22d19_rule.py`.
    `file` is its repository-relative path, or null for a file outside the repository. `sha256` is its digest, or null
    when the file cannot be read (`scripts/runner.py:469-477`). `kernel.d19` is null only when the high plugin names
    no D19, as a scripted plugin in a test does. In acceptance mode, and for any non-null value, the checker requires
    this checkout's path and sha256 (`scripts/check_run.py:326-337`);
  - `acceptance_config` (I1; plan revision 3e, E3): null in development mode. In acceptance mode it is
    `{file, sha256, options}`: the repository-relative path of `acceptance-config.json`, the sha256 of its bytes and
    its `options` (`scripts/runner.py:145-158`, :447).
    - The file (format `acceptance-config/1`) fixes eleven options:
      - `literal_limit` 10⁷ and `seeds` `atlas`;
      - the four `tail_rules`;
      - `lattice_restorer` `late`;
      - `fallback_max_length` 1, `par` 1 and `pair_shards` 4;
      - `bulk_steps` null and `high_steps` null;
      - `tail_steps` 0, which means no tail budget and is recorded as null in `budgets`;
      - `no_lattice` false.
    - These are the runner's defaults. Plan revision 3e, E5 fixes the restorer from D0's measured twist counts and
      times before batch 1.
    - `--mode acceptance` reads the file before anything else in the run. It refuses to start when the file is missing
      or malformed, or when any of the eleven options differs from the run's (`scripts/runner.py:54-57`, :145-158).
      The run then stops with the error and writes no file. Only its empty directories exist.
    - The checker requires the recorded path, sha256 and options to match the file in its own checkout. It also
      compares the record's own `config` values with that file. A development record must have null here
      (`scripts/check_run.py:297-323`);
  - `scripts` hashes `runner_api.py` too, so 22 files before the plugins, and every file each plugin lists in its
    `SCRIPTS`. With the plugins of I1 that is 49 files, the D19 rule among them (`scripts/runner.py:74-81`, :466-467;
    measured in the D0 records);
- in `phases`:
  - `height {result, steps, twists, start, end, sub: {r22d75, r22d81, h4}}`, each sub-phase with `{result, steps,
    start, end}`;
  - `high {result, steps, twists, start, end}`;
  - a sub-phase or the high phase that a plugin's `End` closed also has `reason`, `details` and `attempts`
    (:688-691);
  - when a signal arrives during a plugin call, the runner keeps the attempts that the call returned on the
    sub-phase, the high phase or the lattice phase before the run ends `error` (I1; :661-665, :1313-1317);
  - `lattice {result, steps, twists}`, filled in one of four ways:
    - a played phase: `solved`, with its steps and twists and, since I1, `report {file, sha256}` of
      `lattice/report.json`, the plugin's report (`scripts/runner.py:1386-1390`, :1456-1457).
      - The report gives the restorer and its script, and the restore and play times.
      - It gives the sha256 of the restorer's summary and solutions, of the library and, for `schedule11`, of the
        realignment data.
      - It gives the cascade order, the skipped orbits, the numbers of orbit and correction steps and the folded tree
        parts.
      - It holds the player's own report (`literal-lattice/1`; `scripts/lattice_phase.py:569-576`).
      - The checker requires the file unchanged, `player.played.total` equal to the lattice twists it counts over the
        replayed rows, and `player.final.labels_identity` true (`scripts/check_run.py:949-960`);
    - a phase with no step (I1; plan revision 3e, E2): `solved` with `steps` 0, `twists` 0 and `report`.
      - The runner accepts it only when four conditions hold. The report must say that the restorer scheduled nothing
        (`player.played.total` 0) and that the player's final labels are the identity. The end table must have the
        current digest. The labels of the current state must be the identity on the simulator
        (`Mechanics.labels_identity`; `scripts/runner.py:277-284`, :1403-1417). Otherwise the run ends `error`.
      - The phase is entered only at a state that is not the solved digest (:1302-1305). So an accepted phase with no
        step ends at identity labels and a digest that is not the solved one.
      - The checker tests the labels again from its own state (`scripts/check_run.py:940-946`, :1412-1418);
    - a plugin's `End` (I1; plan revision 3e, E2): `stuck`, `budget`, `timeout` or `error`, with its `reason`,
      `details` and `attempts`. `steps` stays 0 and `twists` null, since nothing was played. The run ends `stuck` for
      `stuck`, `budget` for `budget` and `timeout`, and `error` for `error` (`scripts/runner.py:1369-1384`). The
      checker accepts these results only with no lattice rows (section 3);
    - the `verify_late.py` path, which only a stub lattice plugin takes, in tests: the run's result, `solved` or
      `error`, with the restorer's twists and no steps (:1363-1366). The record then has `config.component`
      `lattice_composition` and the top-level `lattice` of format 2;
  - `tail {result, steps}` keeps its shape. It starts with `result: null` and can also end `not_applicable` or
    `timeout`. Any phase that the run never reached keeps `result: null`. So does a phase that an exception cut off
    after some of its steps, in an error run (the steps stand; section 3);
- in `totals`: `height_twists` and `high_twists`. `lattice_twists` is the sum of the played lattice steps (:1458).
  It is 0 after a phase with no step (:1414), the restorer's twists on the `verify_late.py` path (:1363), and null
  otherwise;
- on every step (bulk steps at :1574-1577, tail and fallback steps at :1215-1228, plugin steps at :759-764, played
  lattice steps at :1445-1449):
  - `subphase` (null outside the height phase);
  - `encoding`: `json`, or `ia16` for lattice steps (below);
  - `word_sha256`: for a `json` word, the sha256 of `json.dumps` of its letters; for an `ia16` word, the sha256 of the
    uncompressed bytes;
  - `tree {file, sha256, size}` (below);
  - `standing`: `rule`, `certificate` (development mode: the gate's end table was loaded) or `fallback`;
  - outside the bulk, also `contract`, `gate`, `attempts` and `table` (`tables/NNNNN.json`, the state after the step;
    :757-758, :1206-1210). A played lattice step has `attempts: []`, and only the last one has `table`, the end table
    (:1449, :1451-1453). Bulk steps keep `N` in `before` and `after`, and have no contract, gate or attempts;
  - on a closure whose V is a certificate: `construction: certificate` and `certificate_sha256`, in both modes
    (:1227-1228);
  - a plugin step's own fields. They may not reuse a core key or name `segments`, `slp`, `certificate` or `node`
    (`ROW_KEYS`, :72-73; checked at :740-743 and, for lattice steps, :1422);
  - a played lattice step's own fields depend on its rule (`scripts/lattice_phase.py:229`, :248-273, :310):
    - a cascade orbit (rule `lattice/orbit-<o>`) has `orbit`, `factor_first`, `factor_count`, `tree_folded` and
      `tree_folded_depth`;
    - a correction (rule `lattice/correction-<k>-<macro>`) has `correction`, `macro`, `family`, `times`, `inverse`
      and `macro_letters`;
    - the last row also has `labels_identity`;
- every attempt carries a boolean `timed_out`: in a step, in a stuck unit, and in a plugin's `End`, the lattice
  plugin's included (:166-170, :675-676, :742, :1373-1374). The lattice plugin's attempt names the restorer as its
  rule (`lattice/<restorer>`). Its `timed_out` is true for `budget` and `timeout`
  (`scripts/lattice_phase.py:460-466`);
- `stuck`: the state, `phase` and `reason`.
  - For the tail: `reason` `no_rule` or `timeout`, `units` with their attempts and, in acceptance mode, `probe` when
    the probe found a word (:1286-1292).
  - For a lattice `End('stuck')` (I1): `phase` `lattice`, the plugin's reason and an empty `units` (:1382). The
    reason is `restorer_no_solution`: the restorer's failure certificate for this input and library
    (`scripts/lattice_phase.py:526-535`);
- `acceptance {verdict, certificate_steps, fallback_steps, probe, timed_out_attempts}`, computed at the end of every
  run (:1494-1513). Since I1 the attempts of a lattice `End` count too, as attempts at the final state (:1501,
  :1510-1511).

Run results stay `solved`, `stuck`, `budget` and `error`. Some examples:

- `--scramble 10 7001 --mode acceptance` (measured; a test of section 5, and development run D0):
  - nine bulk steps of one letter each;
  - every later phase, the tail included, is `not_applicable`;
  - the lattice phase is not entered;
  - the result and the verdict are `solved`.
- `--scramble 20 7106` (measured, D0 on 22 October 2026). D0 ran on the uncommitted I1 candidate, so
  `config.source.dirty` is true. It is to be repeated on the committed head (section 9).
  - The start has 51,828 pieces off, all at height 2.
  - 19 bulk steps of one letter, all of rule `seam`, end `lattice`.
  - Height, high and tail are `not_applicable`.
  - The lattice phase is played in 35 steps, one per orbit of the cascade and none for corrections, with
    `phases.lattice.report`: 8,701,310 twists with the late restorer (`--mode acceptance`, while
    `acceptance-config.json` still named `late`) and 1,516,410 with schedule11 (development mode).
  - The last row has `labels_identity` true. Neither end digest is the solved digest, and the two differ
    (`a71580bf…` and `71dbda5c…`): labels do not fix exact poses (section 4).
  - The result and the verdict are `solved`. `check_run.py` passes both records with 0 unchecked contracts.

**Step trees** (the owner's reading of the tutorial's content,
`docs/wiki/decisions/owner-decisions-2026-10-20-tutorial-acceptance.md:87-96`: every step is a tree of named parts that
expands to the twists played).

- `step-trees/NNNNN.json.gz` is gzip with mtime 0 of the compact JSON of the root part. `sha256` is taken over the
  uncompressed bytes and `size` is the number of parts (`scripts/runner_api.py:210-214`).
- A part is `{name, letters, parts: [part, ...]}` or `{name, letters, span: [from, to]}`. A span is a letter range of
  the step's word, end excluded, and the spans in depth-first order cover the word from 0 to its length. The depth is
  at most 64.
- A part may carry `inverse_of`: the index path from the root to an earlier part with the same number of letters.
  Its letters are that part's letters inverted, in reverse order (for example V⁻¹ of a closure, which names V).
- A part may instead carry `same_as`: the index path to an earlier part with the same name and the same letters. This
  marks a macro the step uses again, so that the tutorial can name it once.
- No key is named `slp`, `node`, `segments` or `certificate`.
- The runner checks a tree's structure before it writes it, and a lattice step's tree after it reads it
  (`scripts/runner_api.py:179-207`, :217-223; `scripts/runner.py:752`, :1427-1428).
- Trees of today's steps:
  - a bulk step is one span named by its rule, except a `conj` step (I1; plan revision 3e, E4). `conj` plays
    (c, x)(d, y)(c, z) at a cap c and a neighbouring cap d (`theory/seams.py:220`, :252). Its tree is a root `conj`
    with three one-letter spans named `x`, `y` and `z` (`scripts/runner.py:1569-1573`);
  - a fallback step and a tail step without parts are one span. A closure or T1′ step has one span per part: W, V,
    the middle and V⁻¹. V⁻¹ is marked `inverse_of` V and, in a T1′ step, the middle W⁻¹ is marked `inverse_of` W
    (:1233-1244);
  - a height or high step has the proposal's tree, or one span named by its rule (:751). The height plugin names the
    two letters of a two-letter step `first letter` and `second letter` (`scripts/rules_height.py:471`). The high
    plugin gives a step of family ST the parts S and T (`scripts/rules_kernel.py:123-128`);
  - a played lattice step has the tree its plugin wrote (`scripts/lattice_phase.py:136-201`; section 4):
    - for a cascade orbit, a root `orbit <o>` with one part per factor, expanded along the nodes of the restorer's
      program. The trailing u of a `conj` node names its leading u by `inverse_of`, and a node met again names its
      first occurrence by `same_as`. A node at depth 64, or past 10⁶ parts in the step, stays one span. The row counts
      these in `tree_folded_depth` and `tree_folded` (:143-145, :169-178);
    - for a correction, a root `correction <k>` with one span per application of its macro, the later ones `same_as`
      the first.

**`ia16` words** (`scripts/runner_api.py:226-290`): `words/NNNNN.ia16.gz`, gzip with mtime 0 of little-endian
uint16 codes 60·grip + item, all below 36,000. Only the lattice plugin writes them, with `Ia16Writer` in its child.
The runner reads each one back with `ia16_digest`, which recomputes its sha256 and length (`scripts/runner.py:1425`),
before it moves the file into the record (:1440-1444).

Every path in `run.json` and the word files is relative to the run directory. Since I1 the lattice plugin runs the
restorer and the player in its work directory `attempts/NNNNN-lattice/`. The restorer's input there is
`table:start-table.json`, a copy of the runner's table (`scripts/lattice_phase.py:489-509`, :542-547). The plugin ends
the phase `error` rather than return a report that names an absolute, home-relative, Windows or parent path (:81-95,
:577-578). D0's depth 20 record names no absolute path in `run.json` or `lattice/report.json` (measured). The
`verify_late.py` path, which only a stub plugin takes, runs as RUN-3 made it: in the run directory with a run-relative
input. There `lattice/report.json` is the verifier's report, and neither it nor `lattice/restore-summary.json` names
an absolute path (tested; RV1-02, RV2-02). The child logs can still name absolute paths. They are not checked, and
they are scrubbed before they are published. The two committed records of section 5 are format 1 records made before
RUN-3: in their lattice outputs and logs the run directory is written `<run>`.

SIGTERM and SIGHUP stop every running child by its process group and wait for it. In the bulk phase the runner then
reloads the bulk child's last checkpoint from `run.json` and rebuilds the bulk end table from its steps; only then
does it record `error` `signal <n>` (RV1-01). The bulk child stops when the runner that started it is gone. It is
given the runner's process id, so a runner that is process 1 does not stop its own child. A signal that arrives
during a plugin call takes effect as soon as the call returns, before its result is used (I1; P1-robust-04,
P3-robust-01). The attempts the call returned are kept, a step it proposed is not played, and the run ends `error`
`signal <n>` (`scripts/runner.py:661-665`, :1313-1317). `main()` sets `RLIMIT_CORE` to (0, 0) before any child
starts, and children inherit it, so no child writes a core file (:1652-1653; P1-robust-02, P2-robust-04).

## 3. The checker

`check_run.py` reads a record of format 1, 2 or 3 and the files it names, rebuilds the start (the census scramble, or
the stored table), plays every word twist by twist on the exact simulator (in a format 3 lattice word, the retained
letters on an exact model of the labels and poses; see "Hooks" below), and checks for each step the digests before
and after, the off counts, the heights, the length and the step's contract (strict′: the unit lands, no lattice
piece goes off and every other off piece keeps its exact pose or lands; Lemma U for the fallback), that every word
is literal, that the record has no absolute path, and the totals. A certificate step's word is replayed twist by
twist like any other, so the checker plays the V that the runner's gate applied as one composed exact action.

For a format 2 record it also checks, before the replay (RUN-4): the configuration keys and their types; the error
(a string without a path, only with result `error`); a gate on every tail and fallback step, `slp_apply` exactly
when V is a certificate; the certificate keys (`T_size` at least 1, `T_digest`, N1 and N2 true); on closure steps
the parts in their order, summing to the word's length, `V_letters` and `V_sha256` against V cut from the word, V⁻¹
equal to V inverted letter by letter and, for T1′, the middle equal to W inverted; and no closure keys on any other
step. Every failure is named in the checker's JSON lines.

For a format 3 record (I0, extended by I1) it also checks:

- **Configuration** (`scripts/check_run.py:340-370`): `mode`, `component`, `literal_limit`, `seeds`,
  `kernel.max_steps` 1 and the lattice restorer; in acceptance mode every lifted clock null in `budgets`,
  `clock_sentinel` 10⁹ and `kernel.seconds` null; the phase and sub-phase orders; and a `bulk_order` within the seven
  bulk rules, so never reopen.
- **The acceptance options** (I1; `PLAN.md` revision 3e, E3; `scripts/check_run.py:297-323`). An acceptance record
  must have `config.acceptance_config` with exactly the keys `file`, `sha256` and `options`. `file` must be
  `research/jumbling/restoration/13-tutorial-runner/acceptance-config.json`, `sha256` must be the sha256 of that file
  in the checker's checkout, and `options` must be its options. The file must hold exactly the eleven options of E3,
  with `no_lattice` false. The run's own options must equal them: `literal_limit`, `seeds`, `tail_rules` and
  `config.lattice.restorer`, plus `fallback_max_length`, `par`, `pair_shards`, `bulk_steps`, `tail_steps` and
  `high_steps` in `budgets`. The file's `tail_steps` 0 counts as null, because the runner records an unlimited tail
  as null (:316-317). The record does not keep `--no-lattice`, so the checker checks that option only in the file. A
  development record must have `acceptance_config` null or missing. Any other case fails `acceptance_config`.
- **The kernel's D19 rule** (I1; `PLAN.md` revision 3e, E2; `scripts/check_run.py:326-337`). In acceptance mode, and
  in development mode whenever it is not null, `config.kernel.d19` must be exactly the `file` and `sha256` of
  `research/jumbling/restoration/12-low-height/scripts/round22/r22d19_rule.py` as it is in the checker's checkout.
  Otherwise the record fails `kernel_d19`. The checker reads and hashes this file and `acceptance-config.json` but
  never runs them. It reads both from its own checkout, not from the commit in `config.source`, so a record fails in
  a checkout where either file has changed. Test: `test_v3_acceptance_config`. Seven changes to the acceptance
  options or their record fail, and so do two changes to the D19 record. A development record passes without a D19
  record and with this checkout's, and fails with acceptance options.
- **Step labels** (`scripts/check_run.py:451-489`): the step keys; the phase and sub-phase; the encoding (`ia16` for
  lattice steps); `word_sha256`; the standing (`fallback` exactly on fallback steps and only in development mode;
  `certificate` exactly on `slp_apply` steps in development mode); `construction: certificate` exactly on `slp_apply`
  steps, each with a `certificate_sha256`; every bulk rule in `config.bulk_order`; a boolean `timed_out` on every
  attempt; contract and gate `kernel_landing` or `kernel_lowering` and `r22d25` on high steps, and `lattice_literal`
  and `player` on lattice steps; and a boolean `labels_identity` only on lattice steps.
- **Step trees**: the file (gzip with mtime 0), its sha256 and its size; the structure of section 2; the root's
  length equal to the step's; and that the tree expands to exactly the replayed word. Every `inverse_of` range must
  equal the inverse of the range it names, letter by letter, through the menu's inverse table, and every `same_as`
  range the range it names, with the same name. A reference to a part that is not wholly before it, or a part with
  both keys, fails as `tree_malformed`. For a lattice step `check_run.py` checks the structure, and
  `check_lattice.check_tree_ranges` checks the `inverse_of` and `same_as` ranges on the `ia16` word
  (`scripts/check_lattice.py:130-205`).
- **Phases in the fixed order**: a phase or sub-phase entered again fails. Every recorded start and end is compared
  with the checker's own snapshots, and every `not_applicable`, `handoff` and `lattice` claim is recomputed from them
  with the six entry tests of section 1, `h4`'s max ≤ 4 included. A `stuck` claim is not recomputed: that would need
  rule code. The results of the height sub-phases, the high phase, the tail and the lattice phase must agree with the
  run's result (`scripts/check_run.py:816-830`). An `error` in a height sub-phase, the high phase or the lattice phase
  makes the run `error`. Unless the run ends `error`, a `budget` or `timeout` after the bulk makes it `budget`, a
  stuck tail or lattice phase makes it `stuck` (and nothing else does), and a solved lattice phase makes it `solved`.
  - A phase left out (a null result and no steps) must follow a result that ends the run. The checker tracks
    `ended`: it starts true only in an error run, and it becomes true after a `budget`, `timeout` or `error` at a
    position after the bulk, or after a stuck tail. A phase left out before that fails `phase_result`. The one other
    null position is the lattice phase of a run that ends `solved` (a state already solved) or `budget`
    (`--no-lattice`) (review finding I0-03; `scripts/check_run.py:800-815`).
  - A null result with steps passes only in an error run: an exception after a tail step, or after a step of a height
    sub-phase or the high phase, leaves that phase's result null with its steps standing. Such a sub-phase or high
    phase keeps the start it was entered at, which is compared with the checker's snapshot, and with steps its entry
    test must hold there; the tail must have had a piece off where it started. Steps without a start fail
    `phase_result`, and a failed entry test fails `entry_test` (scoped verification finding I0-V1-01;
    `scripts/check_run.py:669-681`, :744-746, :759-765).
- **The high phase's letter bound** (I1; review finding P4-falsepass-02; `scripts/check_run.py:775-787`). Whenever
  the high phase has a step, an error run included, the checker calls `check_contracts.check_high_phase` with all the
  high rows and its own snapshot at the phase's start (the off count and the height histogram). The kernel's bound is
  6·n6₀ + 964·off₀ letters over the whole phase (`scripts/check_contracts.py:16`, :299-314; the kernel's
  `12-low-height/scripts/round22/r22d25_search.py:584-585`). It is counted from the phase's start because each kernel
  call sees only one step (`PLAN.md` section 3, P4). A phase with more letters fails `kernel_letter_bound`, and a hook
  that crashes or returns something other than a list of names fails `contract`. Test: `test_v3_high_phase_bound`.
  With 964 replaced by 0, the bound at the fixture's start is 6 letters, so a seven-letter step fails
  `kernel_letter_bound`. With 1 the bound holds. A crashing hook fails `contract`.
- **The lattice phase** (I1; `PLAN.md` revision 3e, E2, and K1 in E1; `scripts/check_run.py:788-799`, :1397-1441). A
  phase with a result, a row or a composition block must start with no piece off. In every case its `steps` must
  equal the number of lattice rows.
  - Played rows (K1): `phases.lattice.result` must be `solved` and `twists` the rows' letters. The record must have no
    `lattice` block, and the last row must have a boolean `labels_identity`. Unless the run ends `error`, the run's
    result must be `solved`, the last row's `labels_identity` true and no piece off. `check_lattice.py` replays every
    row ("Hooks" below). The runner keeps the plugin's report as `lattice/report.json`. `phases.lattice.report` must
    have exactly the keys `file` and `sha256`, name a file inside the run directory and match that file's sha256. The
    report's `player.played.total` must equal the lattice rows' letters, and its `player.final.labels_identity` must
    be true (`check_lattice_report`, :949-960). Otherwise the record fails `lattice_report`.
  - No row and result `solved`: the restorer scheduled nothing (E2). The checker requires that no piece is off. It
    tests the labels of its own replayed state: every frame must agree and the labels must be the identity
    (`labels_identity`, :940-946, :1412-1418). It checks the report as above with 0 twists, and `twists` must be 0.
  - A recorded `End` (`stuck`, `budget`, `timeout` or `error`) passes only with no lattice row, and on the composition
    path only as `error`. Its `twists` stays null. An `error` End needs an `error` run. Unless the run ends `error`, a
    `stuck` End needs a `stuck` run and a `budget` or `timeout` End a `budget` run. A stuck run also has its `stuck`
    block at the final state. The lattice phase's attempts count in the verdict as attempts at the final state
    (:861-867). So a stuck lattice phase with a timed-out attempt has the verdict `budget`. Any other result
    (`lattice`, `handoff`, `not_applicable`) fails `phase_result`.
  - Tests:
    - `test_v3_played_lattice`.
    - `test_v3_lattice_labels_not_digest`: a word that leaves labels other than the identity, in a row that claims
      identity labels, fails.
    - `test_v3_lattice_with_no_step`: the record passes at the solved state. It fails when the report counts a
      played twist, when the report's labels are not the identity, and when the checker's own labels are not the
      identity.
    - `test_v3_lattice_ends`: five `End`s with their run results and verdicts, five changes to the record, and an
      `End` after a played row.
    - In `test_runner.py`, on records the runner wrote: `test_10_lattice_ends_pass_the_checker`, and, through P3's
      plugin, `LatticeEndToEndTests` (the synthetic program and w9).
- **Paths** (I1; integration check K3.1 of `packets/REVIEW.md`; `scripts/check_run.py:148-170`). No string in the
  record may name an absolute, home-relative, Windows or parent path. The non-null value of every key `file`, `word`,
  `unit`, `table`, `library` and `out` must be a relative path that stays inside the run directory. An integer `unit`
  is not checked as a path (:154). It is a piece number, such as `params.unit` of a d19 kernel row
  (`12-low-height/scripts/round22/r22d25_search.py:379`). No checker test has such a row yet. The repository-relative
  `file` of `config.acceptance_config` and of `config.kernel.d19` passes this spelling test. The checker reads those
  two files from its own checkout.
- **The probe**: its word must not be empty, and every piece of its unit must be off at the final state. The word is
  replayed on a copy of the end state under the fallback's contract. Afterwards a piece may be off only if it was
  off before and is outside the unit. So the unit lands, no lattice piece goes off and the off set strictly shrinks
  (review finding I0-04; `scripts/check_run.py:902-937` and :1392-1396, `fallback_kept` :649-652). A probe it cannot
  confirm, or a probe in development mode, fails as `probe`.
- **The acceptance block** (`scripts/check_run.py:838-893`): the counts of certificate and fallback steps and of
  timed-out attempts, the probe flag, and the verdict, recomputed in the order of section 1. The attempts are those
  of the steps, the stuck units, the height sub-phases and the high phase. Since I1 they also include any attempts
  recorded on the lattice phase, such as those of a lattice `End`.
- **Hooks** (packets P4 and P5, integrated by I1). A height or high step goes through `check_contracts.py`:
  `prepare(row, st)` before the replay (the replay changes the live state), `on_letter(ctx, k, st)` after each letter
  and `finish(ctx, row, st)`, which returns the step's failures (`scripts/check_run.py:1301-1335`).
  - Height steps (`scripts/check_contracts.py:103-165`, :207-255). The rule must be one in the table of `PLAN.md`
    section 1: r22d75 T0 to T2; r22d81 H6, U, H4 and H2; r22d88 U and U2; and h4 L4, U, L2 and pair. The step's
    contract, sub-phase, length and τ and u fields must match that rule. The checker counts every u itself, at the
    recorded τ, with its own straddle test (:21-41). It checks the maximum height, the counts that must fall or stay,
    the exact poses at the kept height, the guard after the first letter of a two-letter step, and the progress of u.
    An r22d75 step must make (n6, n4, n2) fall lexicographically, first at the count of its own tier.
  - High steps (:168-191, :258-286). The row's fields must be valid, and its target must be off before the step. No
    piece may go off. Every other piece that is off before and after the step keeps its exact pose. The pieces in
    `params.high_exact_pieces` other than the target keep their exact poses. With `high_exact` true, so does every
    other off piece at height 4 or more. A landing lands the whole target and lowers the off count. A lowering takes
    the whole target from height 6 to height 4 and lowers n6. The row's off counts and height histogram must match
    the replayed state.
  - A lattice step goes to `check_lattice.replay_lattice_step` (`scripts/check_run.py:1290-1299`;
    `scripts/check_lattice.py:392-504`). It first reads the `ia16` file once to compare its length and sha256 with
    the row (:414-430), and it checks the tree's ranges (:431-435). It then replays the word twist by twist. While the
    state is on the lattice, a retained letter is played on an exact model of the sticker labels and of the K+ poses
    by occupied place (`LatticeModel`, :268-327). Every other letter goes through `State.apply` with the admissible
    and certain checks (:447-461). The replay pass and the range pass each hash and count the bytes they read and
    compare them with the row (review findings P5-falsepass-01 and P5-robust-01; :180-185, :463-466). When the replay
    leaves the model, and at the end of the step, the exact state is rebuilt from the model. The rebuilt state's
    frames must agree, and its labels must equal the model's (`model_labels`, :303-327, :404-412). At the end no
    piece may be off, and the digest must be the row's `after` digest. The checker recomputes whether the labels are
    the identity and compares the result with the row's `labels_identity` (:467-485).
  - An `ia16` word of any other step is read through `check_lattice.read_ia16` (:89-96).
- **End digest** (`scripts/check_run.py:1397-1429`). After played lattice rows, or a solved lattice phase with no
  row, `end_digest` is the replayed digest. It may differ from the solved digest by rotations the labels do not see.
  This is the w9 case: `scripts/test_literal_lattice.py:637-638` for the player, and
  `LatticeEndToEndTests.test_w9_invisible_rotation` in `scripts/test_runner.py` through the runner and this checker.
  After a lattice `End` nothing was played, and `end_digest` is the replayed digest where the lattice phase started.
  A record of the composition path keeps the `verify_late.py` path below, and its `end_digest` stays null
  (`scripts/check_run.py:1428-1429`). Its component is `lattice_composition`, and the runner writes it only for a
  stub lattice plugin, in tests. Every other run ends at the replayed digest, and a solved one at the solved digest.
  In development run D0, the depth 20 scramble of seed 7106 ends with identity labels at `a71580bf…` (late
  restorer) and `71dbda5c…` (schedule11), neither of them the solved digest. The checker passes both records with
  the verdict `solved` and no unchecked contract, in about 270 s and 78 s (measured on the uncommitted I1
  candidate; section 9).

**`unchecked_contract`.** Since I1 the hooks are packets P4 and P5, not stubs. `check_contracts.finish` and
`check_lattice.replay_lattice_step` return only the names of failed checks and never `unchecked_contract`
(`scripts/check_contracts.py:289-296`; `scripts/check_lattice.py:392-504`), so no height, high or lattice step
reports it any more. The name remains for a hook that leaves a contract unchecked. Such a contract fails the record
unless `--allow-unchecked-contracts` is given. With the flag, the record can pass, each such step row says
`unchecked: true`, and the final row counts them in `unchecked_contracts` (`scripts/check_run.py:1137-1150`).
`test_v3_height_contracts` checks both cases with a stand-in `finish` that returns the name. The final row of a
format 3 record also gives the `verdict`.

**Independence** (the owner's first reading of 20 October 2026, decision page :74-76). The checker shares only the
definition of the puzzle: the model assets; the simulator (`sim/`); the I_a menu, `element` and `scramble` of
`theory/restore.py`; the trees; and the cut geometry that counts N (`rim.Domains(...).total_misalignment()`, the same N
as `theory/restore.py:175`). It shares no runner, rule, gate, player or verifier code. An AST scan in
`test_check_run.py` enforces this for `check_run.py`, `check_contracts.py` and `check_lattice.py`. It allows the
attributes `menu`, `element` and `scramble` of `restore`, and three names that `restore` re-exports from the
simulator: `ctx`, which is `rim.ctx = sim.get_context()` (`theory/restore.py:53`, `theory/rim.py:29`); `to_tuple`,
which is `sim.kplus.to_tuple` (`theory/restore.py:50`); and `transpose`, which is `exact.transpose`
(`theory/restore.py:49`), the exact arithmetic that `sim/` itself imports. `verify_late.py` runs only as a child
process. The checker reads and hashes `acceptance-config.json` and the D19 rule file but never imports or runs them.

For a solved run whose record has a `lattice` block, it runs `verify_late.py` on the lattice phase output
(`lattice_check`, `scripts/check_run.py:992-1121`). Only the composition path writes that block: format 1 and 2
records, and format 3 records made with a stub lattice plugin. `verify_late.py` checks the lattice word by composing
permutations, not by playing it (rule 5). The verifier now runs in its own process group with a time limit
(`--verify-seconds`, default 7,200), is stopped when the limit passes, and its report and output are saved in a new
directory `check/run-*/` inside the run directory, or next to `--out` (RV2-06). Those files can name absolute paths;
they are working files, not evidence to publish. Apart from the frozen acceptance options, the checker checks the
words played, not the runner's choice of rule or its budgets. A fallback step whose word is empty passes when its
contract holds, as the record format allows (RV2-05).

## 4. The literal lattice player and the runner's plugins

`literal_lattice.py` (packet LLP) reads a lattice restorer's output (`restore-summary.json` and `solutions.json` or `solutions.json.gz`, format `lattice-restoration-v1`), the pose table the restorer started from and the correction library. It then plays the run's word literally (R22D-57). Packet P3 added one keyword, `observer`, and the runner's lattice plugin plays every lattice word through it.

- Before play, it checks the following:
  - the run is `restored`;
  - the model files' sha256 match the repository;
  - the solution's factors expand to the recorded retained length and factor count;
  - the library's sha256 matches the summary;
  - every correction names a macro of the library;
  - the start table loads to the recorded input digest, is on the lattice and has the recorded label hash.
- Retained part: the program's nodes (`word`, `path`, `conj`, `prod`) are expanded lazily, leaf by leaf. Every twist is played on the 259,800 sticker labels as one gather of its primitive's sticker permutation. No net permutation advances the labels. The exact poses are carried along. For every `word` leaf, such as a seed word that recurs thousands of times, the pose change is composed once and reused, while its labels are still played twist by twist. `path` leaves carry poses twist by twist. So the digest at the end, or at a budget stop, is an exact simulator digest.
- Corrections: every correction twist is played on the exact simulator. Each twist must be in the I_a menu and admissible, and each macro must end on the lattice.
- There are two orders, as the restorer wrote them.
  - Corrections first: C is played on the start, then the retained part R. The digest after C is checked against the summary.
  - Retained first: R is played on the start Y. The player builds T = solved·C⁻¹ on the simulator, checks its digest against the summary's `T_digest` and compares all 259,800 labels of Y·R with those of T. It then plays C on the played state Y·R, not on T (review finding LLP-false-pass-01).
- Solved means that the played state's labels are the identity. This is the owner's definition of restored ([decision](../../../../docs/wiki/decisions/owner-decisions-2026-10-15-jumbling-restored.md)). `final.digest` is the digest of the state actually played, and it need not be the solved digest. Labels do not fix exact poses, because a piece may carry a rotation that maps it and each of its stickers onto themselves. The plain test fixture ends with identity labels and a digest that is not the solved one (measured (finite inputs)).
- The report (`literal-lattice/1`, written atomically) gives:
  - the input and the order;
  - the scheduled lengths (retained, corrections) and the played lengths (retained, corrections, total);
  - the junction (`T_digest`, `labels_equal`) and `final` (`digest`, `labels_identity`);
  - the result: `solved`; `budget` with the position (phase, factor and twist); or `failed` with the name of the check that failed. `input_format` names a file it cannot read and, for now, also other unexpected failures (LLP-contract-03);
  - the time of each phase and the sha256 of the research scripts loaded.

  The program exits 0 for `solved` and `budget` and 1 for `failed`.
- The committed depth 20 record of section 5 writes the run directory `<run>` both in the summary's input and in the solutions key, so the restorer's tag rule no longer finds the key. The player also tries the key that starts at `<run>`, and it names a missing key `solution_tag` (LLP-contract-01). A test of section 5 reads that record's 10,731 factors through it.
- The observer (P3): the signature is `play(restore_dir, start_table, run_index=0, library=None, seconds=None, observer=None)` (`scripts/literal_lattice.py:383`).
  - Every hook is guarded by `observer is not None` (:423-434, :519-523, :541-543, :555-565, :586-587). With no observer, the player takes its LLP path.
  - With an observer, `word` leaves no longer reuse their pose change, so poses move twist by twist (:540-542).
  - The observer receives the letter codes of every leaf, every correction twist, the state at the end of every factor and every correction, and the final state.
  - `ObserverLetters` (:224) gives each primitive and correction twist its exact I_a code from the player's menu.
- Played speed. All figures are measured (finite inputs), as cloud timings of research code with other jobs on the machine.
  - Two test fixtures of 16,184,022 retained twists each, one plain and one with nine correction twists of the macro w9, played their retained parts in 560 s (about 28,900 twists per second) and 641 s (about 25,250 twists per second) in the acceptance run. In Codex's own run they took 570 s and 561 s.
  - In D0 (section 5), the player with P3's observer played the 8,701,310 retained twists of the depth 20 scramble of seed 7106 in 236.7 s, about 36,800 twists per second. This measurement replaces LLP's estimate of 5 to 6 minutes for that word.
  - At the rates measured so far (about 25,250 to 92,000 twists per second; the higher rate is in section 5), the 166,777,936 retained twists of E2's lattice word would take about 0.5 to 1.8 hours (estimate, not run).
  - `PROOFS-B.md` R22D-57 estimated about one CPU hour for the same word, from about 21 µs per twist for the label gather alone. The player's measured rates are about 11 to 40 µs per twist, on machines shared with other jobs.

**The plugin interface** (`scripts/runner_api.py`, I0, unchanged in I1). The runner owns the state, the record and the phase order. A plugin reads the current state and either proposes one step or ends its phase.

- A plugin is a module with `next_step(ctx)` (height and high) or `play(ctx)` (lattice). It also has a list `SCRIPTS` of the repository-relative files it depends on, which `config.scripts` hashes (`scripts/runner.py:53`, :416-420, :466-467).
  - `PLUGINS` names the three packet modules: `rules_height`, `rules_kernel` and `lattice_phase`. None of them sets `STUB`.
  - `STUB = True` now marks only the stand-in plugins of the tests. A stub's `not_applicable` ends its sub-phase `stuck` and makes the run a component `<phase>_stub`.
  - A `not_applicable` from a plugin that is not a stub is an error, because the runner has already applied the entry test (`scripts/runner.py:683-687`). The runner raises, and `run()` records the result `error` (:1479-1486).
- `Ctx` (`scripts/runner_api.py:90-112`; built at `scripts/runner.py:633-640`) holds:
  - `table`, `state`, `snapshot()` and `mechanics`: the current state and its table. A plugin reads the state; only the runner changes it.
  - `work_dir`, the call's directory `attempts/NNNNN-<sub-phase or phase>/`, and `cache`.
  - `execute(argv, tag, seconds, *, cwd=None, stop_when=None)`: the runner's child executor (`scripts/runner.py:507-547`). `cwd` and `stop_when` are passed by keyword. Each child owns a process group, and `seconds=None` means no clock. When `stop_when(line)` is true for a complete log line, the runner kills the group at once and returns `stopped`. This lets a plugin take the first STEP line of a rule program that would go on searching.
  - `budgets`: the clocks, plus `pair_shards`, `kernel_wall_margin` and `lattice_restorer` (`scripts/runner.py:634-635`).
  - `mode` and `subphase`, with `wall(name)` (the runner's own clock for a call, None in acceptance mode) and `child_clock(name)` (section 1, "Clocks").
- `next_step(ctx)` returns one `Proposal` or one `End`. The runner calls it again after every step until one of three things happens: the entry test fails (`handoff`), no piece is off (`lattice`), or the plugin returns an `End` (`scripts/runner.py:652-687`). If a signal arrived during the call, the runner keeps the call's attempts on the sub-phase, saves the record and stops before it stores the result (:661-665; P1-robust-04).
- `Proposal` (`scripts/runner_api.py:34-46`) has these fields:
  - `phase`, `subphase` and `rule`;
  - `word` (`{word, input_digest, output_digest}`, which includes the claimed end digest);
  - `contract`, `gate` and an optional `tree`;
  - `fields`, `standing` (default `rule`) and `attempts`.
- `End` (`scripts/runner_api.py:49-58`) has `result` (one of the seven phase results), `reason`, `details` and `attempts`. Every attempt carries a boolean `timed_out`.
- `record_step` (`scripts/runner.py:737-766`) does the following, in order:
  - checks the proposal against its phase and the record's keys;
  - takes a snapshot and plays the word twist by twist (`apply_word`);
  - requires the claimed end digest;
  - writes the word file and the step tree (one span named by the rule when the proposal has none);
  - exports `tables/NNNNN.json`, appends the row and saves.
- The lattice plugin's `play(ctx)` returns a `LatticeResult` or an `End` (`lattice()`, `scripts/runner.py:1301-1367`).
  - A signal during the call is handled as for `next_step` (:1313-1317; P3-robust-01).
  - A `LatticeResult` goes to `played_lattice` (:1318-1320).
  - An `End` is recorded by `lattice_end` (:1323-1325, :1369-1384), unless it is a stub's `not_applicable`. Only `stuck`, `budget`, `timeout` and `error` are accepted. Any other result stops the run with the error "lattice claims … with every piece on the lattice".
  - For a recorded `End`, the phase keeps `result`, `reason`, `details` and `attempts`. `steps` stays 0 and `twists` stays null, because nothing was played.
  - `stuck` sets `record.stuck` with phase `lattice` and no units, and the run result is `stuck`. `budget` and `timeout` give the run result `budget`. `error` saves the record and stops the run with `lattice phase error: <reason>`.
  - The verdict counts the phase's attempts as attempts at the final state (:1510-1511). So a timed-out attempt turns `stuck` into the verdict `budget`.
  - The `verify_late.py` composition path of rule 5 (:1326-1367) is kept only for a stub plugin that answers `not_applicable`, which happens only in tests. Its record has `config.component = 'lattice_composition'` and cannot pass acceptance.
- `LatticeResult` (`scripts/runner_api.py:82-87`) has `steps`, `end_table` and `report`.
  - Each step is a `LatticeStep` (`scripts/runner_api.py:61-79`) with `rule` (`lattice/...`), `word_file`, `word_sha256`, `letters`, `tree_file`, `tree_sha256`, `tree_size`, `before` and `after` (lattice snapshots), `fields` and `seconds`. Its `contract` is `lattice_literal`, its `gate` is `player`, its `standing` is `rule` and its `encoding` is `ia16`.
  - The runner checks and records it as rule 5 says (`played_lattice`, `scripts/runner.py:1392-1461`; review finding I0-02).
  - It writes the plugin's report to `lattice/report.json` and keeps `phases.lattice.report = {file, sha256}` (`lattice_report`, :1386-1390; recorded at :1413 and :1456-1457).
  - A result with no step is accepted only when all four of these hold (:1403-1417):
    - the report says the restorer scheduled nothing (`player.played.total == 0`);
    - the end table's digest equals the current state's;
    - the player's final labels are the identity;
    - the runner's own labels test holds (`Mechanics.labels_identity`, :277-284).

    The phase is then `solved` with 0 steps and 0 twists. Otherwise the run ends `error`.
- The tree helpers are in `scripts/runner_api.py:130-290`: `span`, `group`, `flat`, `single`, `check_tree`, `write_tree` and `read_tree`, the `ia16` writer `Ia16Writer` and its reader `ia16_digest`.

**What packets P1 to P3 do** (runner side, each in its own file; P4 and P5 are the checker's, section 3). Each packet replaced a stub. It runs the existing rule programs unchanged as child processes through `ctx.execute` and maps every way they can end to the phase results of section 1. The packets were implemented and reviewed on 20 October 2026 (`packets/REVIEW.md`).

I1 integrated them with two sets of fixes (PLAN.md, revision 3d followed by the 3e addendum, E1 and E2):
- the FIX packet: Codex implement call 20261022T015515Z-4f47c9d6 on base 8ae8c4a, a valid run that touched no critical path;
- the runner's own fixes, below.

- **P1, the height phase** (`rules_height.py`). Each sub-phase runs its own programs (`RULE_SOURCES`, `scripts/rules_height.py:26-32`; `next_step`, :534-572):
  - `r22d75` runs `r22d75_from_snapshot.py`;
  - `r22d81` runs `r22d81_rule.py` and, after its STUCK line, `r22d81_pairs.py` with the target cap from that line (`--target` is required, `12-low-height/scripts/round22/r22d81_pairs.py:45`);
  - `h4` runs `h4_unlock.py` and, after its STUCK line, the `two_letter_u.py` shards.
  - Each call stops at the first STEP line. An `h4` call also stops at a START line with no target cap (:363-372).
  - Before any call, a piece above height 6 (above 4 in `h4`) ends the sub-phase `error` with `invariant_height_above_6` or `invariant_height_above_4` (:539-542).
  - HEIGHT2 and HANDOFF end the sub-phase `handoff`. STUCK ends it `stuck` and BUDGET ends it `budget`. The runner's kill gives `timeout`. A child stopped by its CPU limit (SIGXCPU) gives `budget` with `cpu_rlimit` (:459-460). R22D-81's NOT_APPLICABLE is an error, `invariant_height_above_6` (:428-431).
  - Three cases end it `stuck`:
    - STUCK with no target cap (reason `no_target`);
    - UNLOCKED before any step (reason `unlocked_without_step`);
    - every pair shard done with no success (reason `pair_exhausted`).
  - A crash, a missing STEP, a digest mismatch or truncated output is an error. Truncated output is never read as exhaustion (R22D-55).
  - FIX (P1-contract-01, P1-falsepass-02): a parse status `malformed`, `mismatch`, `missing_step` or `crash` is an error. It is decided before the clock kill is read, so a killed child keeps its parse status (:411-412, :419-420).
  - FIX (P1-falsepass-03): every end line needs a START line, except a plain BUDGET line. The packet's two one-step BUDGET exceptions need one too (:434-440). With zero steps, the end line's τ and u must equal START's (not for `r22d75`), and for pairs τ must equal `--target` (:445-450).
  - FIX (P1-falsepass-01, P1-robust-03): the FIRST indices of a DONE shard must be exactly {k < first_letters : k mod n = shard} (:413-418). If every shard is done and the pair is still undecided, the result is `error` with `inconsistent_output`. `timeout` needs a shard killed by the clock, and `budget` needs a shard whose outcome is `budget` (:521-528).
  - FIX (P1-robust-01): the shards share a plugin-local `threading.Event`, and each shard's `stop_when` returns true once it is set. It is set by the first shard error, by any exception, and in a `finally` block before the pool waits for its workers (:490-509). The runner tests `stop_when` only on complete log lines (`scripts/runner.py:520-527`), so a sibling stops only at its next line, and a silent shard runs on until it writes one.
  - A height step records the rule script and τ and u before and after. For `r22d75` these four fields are null (:154-196). A two-letter step also records the middle u (`u_mid`): `r22d88/U2` from `r22d81_pairs.py` (:176-179), and `h4/pair` from the shards, which also records `pair_choice` (:345-350). The contract is the rule name, such as `r22d75/T0`, and the gate is the script's stem (`_proposal`, :469-476). A two-letter step's tree has two parts, `first letter` and `second letter`.
- **P2, the high phase** (`rules_kernel.py`). It runs `r22d25_search.py run --table ... --d19 round22/r22d19_rule.py --max-steps 1`, one kernel step per call (`scripts/rules_kernel.py:171-173`).
  - The kernel's own `--seconds` comes from `ctx.child_clock('kernel_seconds')`. The runner's wall clock for the call is `ctx.wall('kernel_seconds')` plus `kernel_wall_margin`, which is None in acceptance mode (:155-158). The wall-clock kill gives `timeout` with `kernel_wall_clock`.
  - One step with result `lattice`, or with result `budget` and a `max_steps` cursor, is a step (:204-214).
  - With no step (:215-224):
    - the kernel's own `budget` is `budget` (`kernel_clock`);
    - `stuck` is `stuck` with reason `kernel_exhausted`, which covers only the catalogue `r22d25-cat/1`;
    - `unsupported` is an error. That is the kernel's `d19_missing`, or its `height_above_6`, which is recorded as `invariant_height_above_6` because the high entry test excludes it (`round22/r22d25_search.py:329`, :395). Any other reason is recorded as `kernel_unsupported`.
  - The contract is `kernel_landing` or `kernel_lowering` and the gate is `r22d25`. The row keeps the kernel row's own fields plus a `kernel` field. That field holds, among others, the catalogue, the limits, the D19 sha256 and the kernel record's sha256 (:208-214). An ST step's tree has the parts S and T (:123-126).
  - The module names its D19 rule (`D19`, :21). The runner records it as `config.kernel.d19 = {file, sha256}` (`scripts/runner.py:469-477`, :486). A rule outside the repository is recorded with `file` null, and a missing one with `sha256` null.
  - FIX (P2-robust-01): the kernel's `--cache` is a directory `cache` inside the call's work directory. When `home_centroids.npy` exists in `ctx.cache`, the directory is seeded with a hard link to it, or with a copy if the link fails (`scripts/rules_kernel.py:163-170`). P2-robust-02 is fixed in the tests (section 5).
- **P3, the lattice phase** (`lattice_phase.py`, and the `observer` keyword of `literal_lattice.py` above).
  - `play(ctx)` (`scripts/lattice_phase.py:456-579`) first checks that no piece is off and that the table's digest is the state's.
  - It then runs the restorer named by `lattice_restorer`: `late`, or `schedule11` with its realignment data (:28-30). The restorer runs in the call's directory with no clock, in either mode (:501-509).
  - FIX (P3-falsepass-01, P3-contract-01): before reading the run's status, the plugin checks exactly one run, the run's `input` and `input_digest`, the library's sha256 and, for `schedule11`, `realign_sha256` (:522-531). Only after these checks:
    - a failure certificate becomes `stuck` with `restorer_no_solution` (:533-535);
    - another status is `error` with `restorer_status` (:536-538);
    - a failed identity check is `error` with `restorer_output_invalid` and the name of the check (:540-541).
  - It plays the word in a child process (`play_child`, :296-316) with the observer `PlayerObserver` (:204-287). This child also has no clock (`PLAY_SECONDS` is None, :32; :542-547). The observer streams each step's word as `ia16`. For a `solved` play, the child exports the end table and sets `labels_identity` on the last step (:305-314). A player `failed` result is `error` with `player_failed`, and a player `budget` result is `budget` with `player_clock` (:561-564).
  - It makes one step per cascade orbit that has factors and one per correction macro, in the restorer's order (`expected_steps`, :374-389). Every boundary has an exact digest. The contract is `lattice_literal`, the gate is `player`, and the last row carries `labels_identity`.
  - Each step's tree follows the program's nodes (`orbit_tree`, :136-194).
    - In a `conj` node u^(−us)·x^(sign)·u^(us), the trailing u^(us) names the leading u^(−us) by `inverse_of` (:184-185).
    - A node already expanded in the same step names its first copy by `same_as` (:163-165).
    - A node at depth 64 is folded into one span (:169-171), and so is a subtree that would take the step's tree over 10⁶ parts (:176-178). The step's fields count both kinds of fold (`tree_folded`, `tree_folded_depth`).
    - A correction step repeats its macro `times` times, and every later copy names the first by `same_as` (:197-201).
  - It requires the result `solved`, every scheduled twist played and identity labels. It checks every step's word, tree and digest chain, and the end table the child exported, against the player's report (`checked_steps`, :392-445). It then returns the steps as a `LatticeResult` with a report (:569-579). The report gives the restorer, the restore and play times, the hashes, the cascade order, the skipped orbits, the step counts, the folds and the player's report.
  - `acceptance-config.json` names the restorer `schedule11`, and it is the runner's default. The integrator chose it from D0's measured twist counts and times under E5 of PLAN.md (3e addendum; section 9). `late` stays available in development mode.
- **The runner itself** (I1). `main()` sets `RLIMIT_CORE` to (0, 0) before any child starts. Children inherit the limit, so none of them writes a core file into the repository (`scripts/runner.py:1652-1653`; P1-robust-02, P2-robust-04).

## 5. Tests

Test counts in this section are counted with `grep "def test_"`, per file and per class.

- `scripts/test_runner.py` (RUN-1's acceptance check, extended by RUN-3, I0 and I1).
  - Before I0 it had 34 tests, one of them skipped (the depth 20 run to the solved state, which needs `RUNNER_SLOW=1`). Before the integrator's fixes they passed in 3,142 s in Codex's run and in 1,526 s in the acceptance run.
  - They include two exact certificate steps at states of E2's literal replay (stage 12), each with the lengths, hashes and end digest of the packet. Both run in development mode.
    - The closure for b20 at the state of 507 off: `T_size` 111, V of 20,232 letters, 40,500 letters in the step, 507 → 447 off. It took 742 s in Codex's run and 395 s in the acceptance run.
    - The T1′ step for b07 at the state of 324 off: W of 48 letters, V of 139,774, 279,644 letters in the step, 324 → 213 off. It took 962 s and 538 s.

    In both tests the core or W is E2's, given to `close` and `t1prime` directly. Whether the runner's own rules, in unit order, find these steps at those states is not tested.
  - The suite also has:
    - RUN-1's stored T1′ step for b04 (1,254 letters), now through `advance()` with the real mechanics;
    - the small lattice configuration, now with a one-macro library;
    - stub tests for the certificate path, signals, the `--par` retry, the source identity and `--out`.
  - On 20 October 2026, with the fixes and before I0, in this checkout: OK, 34 tests, 1 skipped (the depth 20 test, which needs `RUNNER_SLOW=1`), 1,222 s, worktree unchanged (headless Linux).

  I0 adds 41 tests, 11 of them with the fixes of its review (75 in all, one skipped):
  - one test per row of the phase result table, `timeout` of the tail included, with scripted plugins and fake geometry. The cases are:
    - a component run that ends stuck has the verdict `component`;
    - a stuck run with a timed-out attempt at its end has `budget`;
    - the verdict order;
    - a probe word gives `coverage_failure`;
    - `record_step` exports the table and writes a tree;
    - the bulk order has no reopen.
  - step trees: a closure's tree names V and its inverse; malformed trees are refused before they are written; the written file and its sha256.
  - `execute`: `stop_when` kills the group after the first STEP line; a cancel still works with `stop_when`; with no clock, a child runs to its end.
  - A2 on the certificate stub:
    - in acceptance mode, a complete certificate closure is played through `apply_word`, and the gate's end table is never loaded (`standing: rule`, `construction: certificate`);
    - the attempt is `certificate_needed` in three cases: with `--literal-limit 4` (the five-letter closure is over it, and nothing is expanded); with an incomplete certificate whose only reason is not a deadline; and with a checker rejection (exit 1 and a `FAIL:` line);
    - development mode keeps the gated end table, and its certificate steps make the verdict `stuck`.
  - the clock tests:
    - the adapter: the sentinel and null budgets in acceptance mode, the values in development mode;
    - `tclose2.py`'s greedy clock: a timed-out attempt in development mode, none in acceptance mode;
    - `jordan_target.py`'s cap before `CONDITION`. Its line, exit 124, or a SIGKILL after 850 s give a timed-out attempt. An exit 0 without `CONDITION`, or a SIGKILL after 5 s, give an error;
    - the certificate deadlines. Per orbit, serialization, mixed and an explicit `timeout` row give `timeout` and the verdict `budget`. A reason that is not a deadline, and a size cap, give `certificate_needed` and `stuck`;
    - two certificates emitted by the unchanged `r22b10_cert.main` with a fake clock: one past its serialization deadline (`timeout`, `budget`) and one not (`certificate_needed`, `stuck`).
  - with real geometry, a `--table` start at the R22D-36 state with scripted height plugins:
    - run A: `r22d75` is stuck, then an `r22d81` step goes to `7cd868f6…`, where `r22d75` would fire but is not entered again;
    - run B starts there, and its first height step is an `r22d75` step.

    Both records pass the format 3 checker with `--allow-unchecked-contracts`. A tampered tree of run B fails it: a wrong part sum, a gap, an overlap, a root shorter than the word, a wrong `inverse_of` range, an `inverse_of` that names a later part, a wrong sha256.
  - the fixes of the I0 review (section 7):
    - `IntegrityTests` (I0-01), six tests:
      - a crashing `stepchk.py` ends the run `error` before `close` or the fallback runs, in development mode with `--par 1` and `--par 2` and in acceptance mode. So does a failed assert of `cellpure5.py` that is not a screen;
      - a mismatched check digest ends it `error` at the pure rule, the certificate gate (also when the step is not strict′), the fallback and the probe;
      - no word and an applicability screen stay attempts, and every rule is tried at every unit;
      - an error leaves `choose` and its serial retry;
      - a later unit's error waits for the earlier unit's serial retry, so `--par 1` and `--par 2` give the same record;
      - an error after a tail step keeps the step.
    - `LatticeTests` (I0-02), three tests with fake geometry:
      - a played lattice is recorded with its end table;
      - six mismatches (the end table, a malformed tree, a word sha256, the digest chain, the labels, no step) end the run `error` before anything changes;
      - every lattice `End` except the stub's is an error until I1. I1 replaced this test (below).
    - With real geometry:
      - `ExactTests.test_09`: a scripted plugin in P3's interface plays the inverse of the small lattice configuration's word in two steps. The record passes the format 3 checker with `--allow-unchecked-contracts` (end digest solved, 10 lattice twists, 2 unchecked contracts).
      - `test_10`: four lattice `End`s give error records that the checker passes.

      I1 changed both tests (below).
    - Three tests changed:
      - the certificate gate: the checker's rejection, an incomplete certificate and a false strict′ are attempts. Digest, hash, length and segment mismatches, a rejection that names a digest, truncated checker output and a missing summary line are errors;
      - `tclose2.py`'s parts: a word of another form is an error;
      - `jordan_target.py`'s cap (above).

  On 20 October 2026, with I0 and its review fixes, in this checkout: OK, 75 tests, 1 skipped (`RUNNER_SLOW=1`), 1,140 s, worktree unchanged (headless Linux).

  I1 adds 7 tests and changes others. There are now 82 tests in all, one skipped (`test_04_slow_scramble_20`, `RUNNER_SLOW=1`). Per class: RecordTests 16, PhaseTests 13, LatticeTests 4, AcceptanceConfigTests 3, TreeTests 3, ExecuteTests 3, CertificateTests 13, ClockTests 5, IntegrityTests 6, BulkTests 3, ExactTests 11, LatticeEndToEndTests 2.
  - `LatticeTests` (`scripts/test_runner.py:1386`):
    - the played lattice is recorded with its end table and its report (:1401);
    - the mismatch test now has seven cases (:1426). `no report` is new. `no step` now means no step although the restorer scheduled three twists;
    - new `test_lattice_with_no_step` (:1458): a phase with nothing scheduled is `solved` with no row. Four failures end the run `error`: twists scheduled, final labels in the report that are not the identity, an end table that is not the state's, and state labels that are not the identity;
    - `test_lattice_ends_are_recorded` (:1500) replaces the I0 test that every lattice `End` but the stub's is an error. `stuck` gives result and verdict `stuck`. `stuck` with a timed-out attempt gives the verdict `budget`. `timeout` and `budget` give `budget`. `error` gives `error`. `not_applicable`, `handoff` and `lattice` from a plugin that is not a stub are errors ("lattice claims …"). A stub's `stuck` is recorded. The stub composition path is still covered.
  - `AcceptanceConfigTests` (new, :1557):
    - the committed options are the runner's defaults, and the record keeps the file, its sha256 and its options (:1563);
    - acceptance mode refuses each of the 11 options set to another value (:1576; `scripts/runner.py:145-159`). The runner has then made only its empty directories and written no file. Development mode records null;
    - `config.kernel.d19` is recorded, and a scripted high plugin records null (:1598).
  - `ExactTests`:
    - `test_02` (:2668) now uses a stub lattice plugin, so its record stays a `lattice_composition` component;
    - `test_09` (:2934) passes the checker without `--allow-unchecked-contracts`: 0 unchecked contracts, 10 lattice twists, verdict `solved`;
    - `test_10` (:2960) has five lattice `End`s: stuck; stuck with a timed-out attempt; timeout; budget; error. They give the verdicts stuck, budget, budget, budget and error, and the checker passes each;
    - new `test_11_kernel_end_to_end` (:2987) runs at the R22D-36 state with `--bulk-steps 0`, `--high-steps 1`, no tail rule and `--fallback-max-length 0`. A stub height plugin ends `r22d75` and `r22d81` `stuck`, and `h4` is not applicable. The real P2 kernel with the real D19 plays one `F1` high step whose first letter is at most (12, 45) and whose contract is `kernel_<kind>`. `config.kernel.d19` names the module's D19. The checker passes it with 0 unchecked contracts and the verdict `component` (the component is `bulk_budget`). The other kind fails with a `kernel_` failure.
  - `LatticeEndToEndTests` (new, :3039) runs the real P3 plugin through the runner with the stub restorers of `test_lattice_phase.py`:
    - the synthetic program plays `lattice/orbit-23` and `lattice/orbit-34` to the solved digest, and the checker passes with 0 unchecked contracts (:3062);
    - the w9 fixture plays 9 twists to identity labels at a digest that is not the solved one, and the checker passes. The same word without its last letter fails the checker (:3071).
  - `PhaseTests.test_not_applicable_row` (:1140) asserts that none of the three plugins of `PLUGINS` is a stub. It also asserts that a `not_applicable` from a plugin that is not a stub is an error.
    - `test_07` and `test_08` keep their scripted height plugins, whose steps are not rule steps: they have no τ or u fields, and run B's `r22d75` word has three letters. In the packets' integration check with the real P4 they failed on `height_rule`, `height_fields` and `height_tau` (`packets/REVIEW.md`, K2). `test_07` now asserts that the real contracts reject both records with `height_` failures only. Both tests then check the phase order and the step trees through `scripted_check`, which leaves the height contracts unchecked (one per record).
  - Not changed in I1:
    - the b20 acceptance-mode test of the plan (`--literal-limit 40499`) is not written, and a comment at :2072-2073 still names it;
    - `test_04_slow_scramble_20` (:3033) still asserts that `record['lattice']` is not None. Only the stub composition path sets it (`scripts/runner.py:1338`), so the real lattice plugin does not. This test was not run.

  TODO-T: result, test count, skips and time of this file on the merged I1 candidate (headless Linux, worktree unchanged).
- `scripts/test_check_run.py` (RUN-2's acceptance check, extended by RUN-4, I0 and I1).
  - Before I0 it had 66 tests. Before the integrator's fix they passed in 1,604 s in Codex's run and in 1,632 s in the acceptance run. RUN-2's tests are kept, and the format 1 lattice fixture is now built from a format 2 run. New positive and negative tests cover the format 2 checks and the verifier's time limit, with the gaps listed in section 7.
  - On 20 October 2026, with the fixes and before I0, in this checkout: OK, 66 tests, 482 s (headless Linux).

  I0 adds 31 tests, one of them with the fixes of its review and two with the fix of its scoped verification (97 in all):
  - format 3 records:
    - a bulk record; the configuration in both modes; the bulk order (reopen fails); the step labels;
    - a closure tree whose V⁻¹ names V and whose middle names W; tree failures; `same_as`;
    - height steps with unchecked contracts;
    - the hooks: `prepare` before a step, `on_letter` after each letter, `finish` after the last;
    - entry tests recomputed from the checker's snapshots; the phase order;
    - phases left out (I0-03). The review's counterexample and every prefix that does not end the run fail. Prefixes that end it, an error record cut short, an error after a tail step and `--no-lattice` pass;
    - a phase cut off by an exception after its steps (I0-V1-01). It keeps its start and its entry test; a tail step recorded with no piece off fails. This is also tested on stand-in snapshots;
    - `h4` needs max ≤ 4; the six entry tests at their boundaries;
    - the verdict with a timed-out attempt at the final state (`budget`) and only before it (`stuck`); the verdict order;
    - a development fallback step (`coverage_failure`);
    - the probe (I0-04): a unit on the lattice, with the empty word or with the probe's word, fails;
    - a played lattice, and a played lattice solved by identity labels with a digest that is not the solved one;
    - the composition path (a component, end digest null).
  - records written by the real `runner.py` with the stub plugins:
    - `--scramble 10 7001` in acceptance mode and in development mode (bulk only, `solved`, verdict `solved`, end digest the solved one, no unchecked contract), also through the command line;
    - the R22D-36 state with no bulk and no tail rule (a component, with `h4` not applicable at max 6), and mutations of it;
    - reopen in `config.bulk_order` or in a step.
  - the independence scan of section 3, and a test that the scan finds forbidden imports and attributes.

  On 20 October 2026, with I0 and its review fixes, in this checkout: OK, 97 tests, 818 s, worktree unchanged (headless Linux).

  I1 adds 3 tests and changes others. There are now 100 tests in all: CheckRunTests 94, RunnerRecordTests 4, IndependenceTests 2.
  - `test_v3_lattice_ends` (`scripts/test_check_run.py:1760`):
    - five recorded lattice `End`s (stuck; stuck with a timed-out attempt; timeout; budget; error) pass with the run result each implies, and the verdict counts their attempts;
    - these mutations fail: the run result `budget` for a `stuck` End; a timed-out attempt added while the verdict stays `stuck`; `twists` 0 instead of null; the phase result `lattice` or `handoff`;
    - an `End` after played rows fails `phase_result` (K1).
  - `test_v3_lattice_with_no_step` (:1790): a phase with nothing scheduled is `solved` only when the checker's own state has identity labels. A report that schedules a twist, or whose final labels are not the identity, fails `lattice_report`. A start whose labels are not the identity fails `labels_identity`.
  - `test_v3_high_phase_bound` (:1814): the checker applies the kernel's letter bound to the whole high phase, from its own snapshot at the phase start (P4-falsepass-02). With `ST_LIMIT` 0, a 7-letter phase is one letter over the bound and fails `kernel_letter_bound`. A crash of `check_high_phase` fails `contract`.
  - `test_v3_height_contracts` (:1390), renamed from `test_v3_height_unchecked_contracts`: height rows have 0 unchecked contracts. Mutations of `height_rule` and `height_tier` fail. A patched `UNCHECKED` hook still needs the flag.
  - `test_v3_acceptance_config` (:1218) has nine more cases:
    - these fail `acceptance_config`: no acceptance config; another option; a step budget; another restorer; other tail rules; another file sha256; other recorded options;
    - these fail `kernel_d19`: no D19, and another D19;
    - a development run passes with no acceptance options and with this checkout's D19, and fails with recorded acceptance options.
  - `test_v3_played_lattice` (:1716) now has 0 unchecked contracts. `test_v3_lattice_labels_not_digest` (:1736) now fails on `labels_identity`. Several `allow_unchecked` flags were removed.
  - `RunnerRecordTests.test_dispatcher_record` (:1950) replaces `test_stub_record`. The real P1 at the R22D-36 state plays `r22d75/T0` [[12, 14]] and then `r22d75/T1` [[35, 13]] to `3ab25d21…`, with 5,405 pieces off, all at height 2.
    - The height sub-phases give `handoff`, `not_applicable`, `not_applicable`. The high phase is `not_applicable` and the tail is `stuck`.
    - With `--bulk-steps 0` the bulk ends on its step budget, so the run is a component (`bulk_budget`) with result `stuck`. The checker gives the verdict `component`, with 0 unchecked contracts and 2 height twists.
    - These mutations fail: the verdict, `r22d81` entered at height 2, the phase order, and a tree sha256.

  TODO-T: result, test count and time of this file on the merged I1 candidate (headless Linux, worktree unchanged).
- Real plugins under test. I0's tests ran no real height, kernel or lattice plugin. In I1 the real plugins run in `test_runner.py` (`ExactTests.test_11` for P2, `LatticeEndToEndTests` for P3 with stub restorers) and in `test_check_run.py` (`RunnerRecordTests.test_dispatcher_record` for P1), as well as in the packets' own test files below. Fixture times that the packets rely on were measured by I0 on 20 October 2026 (headless Linux, four cores, two other single-threaded jobs running; measured (finite inputs)):
  - the kernel with `--seconds 1` at the R22D-36 state: 18.6 s (result `budget`, no step);
  - `eb6.straddler_set` on all 600 caps: 2.5 s at the R22D-36 state (6,489 straddlers), and 18.9 s at the seed 7102 stop (29,212 straddlers);
  - `r22d75_from_snapshot.py` at the R22D-36 state, from (12, 14) to HEIGHT2 in two steps: 13.6 s.
- The five packet test files (P1 to P5), with the I1 FIX changes:
  - `scripts/test_rules_height.py` (P1): 27 tests (21 before I1). ExactTests 3, StubTests 19, CommittedTests 5.
    - FIX adds tests 14 to 19: parse errors survive the wall kill; DONE coverage and an undecided pair with no clock; end lines need START; the zero-step end identity; a shard crash stops its sibling; an exception while waiting stops the workers.
    - Committed test 5 also drops the FIRST line of the class-B letter (2, 1) and asserts `undecided` (P1-contract-03).
    - One stub case changed: an `h4` STUCK line with no τ after a START line that has one is now `error` with `inconsistent_output`.
  - `scripts/test_rules_kernel.py` (P2): 28 tests (26 before), one skipped (`KERNEL_SLOW=1`). AModuleTests 2, BMappingTests 22, CRealKernelTests 4.
    - FIX adds `b18`: the cache and an interrupted scratch directory are local to the call (:655).
    - FIX adds `b19`: the module removes its temporary tree on success and on an exception, unless `KEEP_TEMP=1` (:678).
    - The integrator added a `setUpModule` (:53) that replaces the runner's acceptance-configuration read with None, because these acceptance-mode cases use test-only options. The integrator also added an assertion to `b17` that a missing D19 outside the repository is recorded as `{file: null, sha256: null}`.
  - `scripts/test_lattice_phase.py` (P3): 25 tests (23 before), one skipped (`LATTICE_PHASE_SLOW=1`). Classes A 1, B 3, C 4, D 3, E 1, F 1, G 8, H 1, I 1, J 1, K 1.
    - FIX adds G `test_07` (:799): for both restorers, a failure certificate with a wrong run count, input, input digest, library sha256 or (for `schedule11`) realignment sha256 is `error` with `restorer_output_invalid` and that check. Only a matching certificate is `stuck`.
    - FIX adds G `test_08` (:840): the temporary tree is removed unless `KEEP_TEMP=1`.
    - The integrator made `ResultChecks.check_runner` also check the recorded report.
    - A comment in `D_PluginTests.test_03_zero_steps` (:664) still says that I0 refuses a zero-step result. That is no longer true.
  - `scripts/test_check_contracts.py` (P4): 11 tests (11 before).
    - FIX changes test 6: forward rows labelled `r22d81/H2` and `h4/L2` at τ 1 whose u rises fail `height_u_progress`.
    - FIX changes test 9: an `h4/L4` row whose end state is over the maximum fails `height_max`.
  - `scripts/test_check_lattice.py` (P5): 21 tests (18 before). AIndependenceTests 1, BReaderTests 2, CLatticeTests 18.
    - FIX adds `f_replay_rechecks_replaced_word` (:526), `f_range_pass_rechecks_replaced_word` (:551) and `f_model_label_failure_keeps_exact_end` (:574).
    - Test D prints the model loop's rate next to the whole-step rate.

  The wrapper ran the five files as the FIX call's acceptance check, in Codex's sandbox, on 8ae8c4a with the FIX patch alone, before the integrator's three test edits (measured (finite inputs)). All exited 0 and left the worktree unchanged:
  - `test_rules_height.py`: 27 tests, 94.6 s;
  - `test_rules_kernel.py`: 28 tests, one skipped, 63.8 s;
  - `test_lattice_phase.py`: 25 tests, one skipped, 156.8 s;
  - `test_check_contracts.py`: 11 tests, 109.7 s;
  - `test_check_lattice.py`: 21 tests, 83.9 s.

  In that run, Test D printed 8,862.0 model letters per second against 1,541.1 whole-step letters per second. Codex's report gives 21 in-memory mutants caught, and Test D at 8,204.7 against 2,489.7 in its own run. TODO-T: results and times of the five files on the merged I1 candidate (headless Linux, worktree unchanged).
- `scripts/test_literal_lattice.py` (LLP's acceptance check).
  - Its 16 tests, one of them skipped (the play of the committed depth 20 record, which needs `LATTICE_SLOW=1`), passed in 1,525 s in Codex's run and in 1,578 s in the acceptance run. They cover:
    - the expansion of every node kind;
    - the two fixtures played to identity labels;
    - the pose transport compared with the simulator on short words;
    - tampered outputs, and a wrong start or library;
    - budget stops with an exact stop digest;
    - a compressed library and several runs.
  - The integrator added two tests, which makes 18:
    - the committed record's solution key (LLP-contract-01);
    - a start that differs from T by a rotation the labels do not see. Here `final.digest` must equal a simulator replay of C on that start and differ from the solved digest (IF-01). This test was added after the fix review and was not reviewed.
  - On 20 October 2026, with the fixes, in this checkout: OK, 18 tests, 1 skipped, 468 s. The two fixtures played their 16,184,022 retained twists at about 92,000 twists per second (cloud timing of research code, headless Linux; measured (finite inputs)).
  - I1 did not change `literal_lattice.py` or this file. The observer keyword came with P3, and its tests are in `test_lattice_phase.py`, class B.

  TODO-T: result and time of this file on the merged I1 candidate.
- `scripts/test_check_r22d25.py` (part of the R22D-25 packet): 7 tests, unchanged in I1. They are format checks and record mutations for `check_r22d25.py`, run by stage 12's `test_r22d25.py` and also on their own. On 20 October 2026 they ran inside `test_r22d25.py` (stage 12), which passed; `test_check_r22d25.py` was not run on its own in this checkout. TODO-T: result of this file run on its own on the merged I1 candidate.
- Two records made with RUN-1's runner on 19 October 2026 and passed by RUN-2's `check_run.py` (`runs/`, format 1, measured (finite inputs)):
  - the depth 10 scramble of seed 7001, solved by the bulk phase in 9 twists (38 s);
  - the depth 20 scramble of seed 7106. The bulk phase brings it onto the lattice in 19 twists, all of rule `seam`. The lattice phase restores it in 8,701,310 twists. `verify_late.py` passed it in the runner and again in the checker (654 s in all); it checks the lattice word by composition and does not play it twist by twist (rule 5).

  Neither run needed a tail step. Some files are left out because `runner.py --scramble` makes them again and `run.json` holds their digests: the start and end tables, the copy of the lattice library (the same sha256 as `09-general-method/data/library-merged.json.gz`) and the restorer's working files. In the lattice outputs and logs, the run directory is written `<run>`.
- D0, the first development runs of format 3 (22 October 2026), and the choice of the acceptance restorer: section 9.

## 6. What it does not do yet

- Height and high steps in a run. Since I1 the height, high and lattice plugins are packets P1 to P3, and the
  checker's contracts are P4 and P5 (section 8). None of these plugins is a stub, so none of them makes a run a
  component. No height, high or lattice step reports `unchecked_contract` any more. But no run from a scramble or a
  census stop has taken a height or high step yet. The D0 runs (section 9) have only height-2 pieces, so every phase
  between the bulk and the lattice is `not_applicable`. Through the runner, only two tests take the real height or high
  plugin, both at the R22D-36 state. `test_dispatcher_record` runs the height plugin and expects two `r22d75` steps to
  HEIGHT2 (`scripts/test_check_run.py:1950-1978`). `ExactTests.test_11` runs the kernel for one step after a stub
  height plugin (`scripts/test_runner.py:2987-3030`). The stub branches stay for test plugins that set `STUB`
  (`scripts/runner.py:682-687`, :1323-1367). They give the component labels `height_stub`, `high_stub` and
  `lattice_composition`.
- A lattice phase that ends with no played step. Since I1 a lattice `End` and a phase with nothing scheduled are
  recorded (section 8). Only the tests' scripted plugins produce them (`scripts/test_runner.py:1458-1555`,
  :2960-2985). In the D0 run with the late restorer, the real plugin solved the lattice phase (section 9).
- `ia16` words in `record_step`: it writes every word as JSON, gzipped above 100,000 letters
  (`scripts/runner.py:726-730`, :755). Only the lattice plugin writes `ia16` words, which the runner moves into the
  record (rule 5).
- A bulk `conj` step in a run. Since I1 its tree has three named parts `x`, `y` and `z` of one letter each
  (`scripts/runner.py:1569-1573`; section 8). No test and no D0 run has a bulk `conj` step, so no run has written
  such a tree yet. Every other bulk step is one span named by its rule.
- E2's certificate steps. Rule 3 (b) builds and expands a certificate V at the state where it acts. It is tested only
  at two states of E2's literal replay, in development mode, and not on a run from a scramble. No runner record with a
  certificate step exists outside the tests. In both tests the core or W is E2's, given to `close` and `t1prime`
  directly. Whether the runner's own rules, in unit order, find these steps at those states is not tested. E2's b06
  step also needed a three-letter SAFE-ALL preparation, (178, 7)(170, 1)(75, 1), before its T1′ step (stage 12,
  `round22/e2_b06_*.py`); the runner has no rule that makes such a preparation. In acceptance mode such steps are
  played twist by twist (A2): E2's 13 literal steps from 737 to 0 total 2,012,155 twists
  (`12-low-height/README.md:936`). The A2 tests play a five-letter stub closure (section 5). No test plays a
  certificate step of real size in acceptance mode. The plan's b20 case with `--literal-limit 40499` (`PLAN.md`,
  section 4) cannot run in acceptance mode, which now refuses any limit but the frozen one (section 8).
- Height 4. The R22D-25 kernel of stage 12 plays R22D-25 with R22D-37's height-6 tier over a whole stop, under the
  practical catalogue `r22d25-cat/1` (S and T of one letter; R22D-60's pass left out). By `PROOFS-B.md` R22D-55 this
  is a revision of the rule, so its stuck outcome covers only that catalogue. `check_r22d25.py` checks its records.
  Since I1 the runner's high phase runs the kernel, one step per call (P2, `scripts/rules_kernel.py`). The kernel uses
  the D19 rule (`12-low-height/scripts/round22/r22d19_rule.py`), and `config.kernel.d19` keeps that file's path and
  sha256 (`scripts/runner.py:469-477`). D19 was not in the repository when the kernel was written and tested, and the
  kernel's own tests use stub programs. The integrator's runs with D19 at six height-4 stops (stage 12, section 11)
  end stuck in `r22d25-cat/1` with no step, and each record passes `check_r22d25.py`. Without D19, at a state with
  height-4 units where the height-6 tier has no step, the kernel reports `unsupported` before it tries S·T. The plugin
  maps that to an error (section 4). Through the runner, only `ExactTests.test_11` (above) takes a kernel step. No run
  from a scramble or a census stop has taken one.
- `check_run.py` compares an acceptance record's options with `acceptance-config.json` in its own checkout. It
  compares the D19 record with `r22d19_rule.py` in the same checkout (`scripts/check_run.py:297-337`). It does not
  read either file at the record's `config.source.commit`, which is the file that `PLAN.md` E3 names. So a record made
  with other contents of either file fails there, even when its own commit is consistent.
- It decides nothing about coverage: a solved run shows only that the rules reached the solved state from that start.
- The acceptance test is fixed (owner, 20 October 2026,
  [decision](../../../../docs/wiki/decisions/owner-decisions-2026-10-20-tutorial-acceptance.md)). It is 21 scrambles
  that development never used: three fresh seeds at each of the depths 10, 20, 40, 100, 300, 1000 and 3000 (seeds 9011
  to 9073). They run in batches from shallow to deep. Every word is played twist by twist, the lattice phase included,
  and the checker replays every record twist by twist. None of them has been run. Every batch uses the options of
  `acceptance-config.json`. Its lattice restorer is `late` until D0's numbers decide it (`PLAN.md`, E5; section 9).

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

## 8. Packets P1 to P5: implemented, reviewed and integrated (I1)

On 20 October 2026 Codex implemented the five packets through the wrapper, each in its own worktree from commit
`97815b1`. All five runs were valid: the reported model, effort, sandbox and writable roots were as requested, and the
changes were only in the allowed files. Their acceptance checks passed in Codex's sandbox (headless Linux):

- P1 `test_rules_height.py`: 21 tests in 116 s;
- P2 `test_rules_kernel.py`: 26 tests, one skipped (`KERNEL_SLOW`), 102 s;
- P3 `test_lattice_phase.py`: 23 tests, one skipped, 157 s;
- P4 `test_check_contracts.py`: 11 tests in 127 s;
- P5 `test_check_lattice.py`: 18 tests in 94 s.

None of the patches touches a critical path. The owner stopped the work on 20 October 2026 when the review was done.
On 22 October the owner restarted it for the tutorial and its acceptance test
([decision](../../../../docs/wiki/decisions/owner-decisions-2026-10-22-tutorial-first.md)). The integrator applied the
five patches unchanged at commit `8ae8c4a` (22 October). Part I1 of the plan (`PLAN.md`, revision 3d section 4 and the
revision 3e addendum) then connected them to the runner and the checker. The height, high and lattice plugins and the
checker's contracts are now the packets' code, not the I0 stubs.

- `packets/P<n>.md` is the packet as sent (seven parts).
- `P<n>-changes.md` is its change log from the review rounds before the call. It ends with the integrator's decisions
  of 20 October 2026 that the packet carries.
- `P<n>.patch` is the change, made against `97815b1` and applied unchanged at `8ae8c4a`.
- `P<n>-report.md` is Codex's report.
- The integrator's review of the five patches is in `packets/REVIEW.md`. Each patch was read in three lenses, each
  finding above `nit` went to two refuters, and one check covered all five patches applied together (K1 to K3). The
  review confirmed 22 findings, one of them `major`, and refuted 2. It also lists 23 nits.

**What I1 integrated.** The integrator did the runner and checker sides:

- `lattice_phase.py` (P3) plays the lattice phase, with the restorer that `--lattice-restorer` names: `late` or
  `schedule11` (`scripts/lattice_phase.py:28-29`).
  - The runner records the plugin's `LatticeResult` as rule 5 says. It writes the plugin's report to
    `lattice/report.json` and keeps the file's path and sha256 in `phases.lattice.report`
    (`scripts/runner.py:1386-1390`, :1456-1457).
  - A result with no step is accepted only when four things hold. The plugin's report must say that nothing was
    played and that the final labels are the identity. The end table must keep the state's digest. The runner's own
    test must find identity labels. The phase is then `solved` with 0 steps and 0 twists. Otherwise the run ends
    `error` (:1403-1417; `PLAN.md`, E2).
  - A lattice `End`, other than a stub's `not_applicable`, is recorded with its reason, details and attempts. `stuck`
    ends the run `stuck`, `budget` and `timeout` end it `budget`, and `error` ends it `error` (:1323-1325,
    :1369-1384). The verdict counts its attempts as attempts at the final state (:1510-1511).
  - The composition path through `verify_late.py` stays only for a stub plugin that ends `not_applicable`, which only
    tests use. It keeps the component `lattice_composition` (:1326-1367).
- `--mode acceptance` reads `acceptance-config.json`. When any of its eleven options differs, the runner refuses to
  start, before it writes any file. Otherwise it records the file's path, sha256 and options in
  `config.acceptance_config`; a development run records null (`scripts/runner.py:55-57`, :145-158, :446-447;
  `PLAN.md`, E3). The file's values are the runner's defaults, so `--mode acceptance` needs no other option
  (`scripts/test_runner.py:1563-1574`).
- `config.kernel.d19` records the path and sha256 of the D19 rule that the kernel plugin passes
  (`scripts/runner.py:469-477`).
- A bulk `conj` step's tree has three named parts `x`, `y` and `z` of one letter each (`scripts/runner.py:1569-1573`;
  `PLAN.md`, E4).
- The runner sets `RLIMIT_CORE` to 0 before any child starts, so no child writes a core file into the repository
  (`scripts/runner.py:1652-1653`).
- After `next_step` or `play` returns, the runner checks for a signal before it stores the result. The call's attempts
  stay on the sub-phase or phase (`scripts/runner.py:660-665`, :1312-1317).
- The checker's new checks:
  - It calls `check_contracts.check_high_phase` with its own snapshot at the high phase's start whenever the phase has
    steps, an error run included (`scripts/check_run.py:775-787`).
  - With played lattice rows, `phases.lattice.result` must be `solved` and its steps and twists must be the rows'
    (:793-799, :1432-1433). The last row's `labels_identity` must equal the labels of the checker's own replay
    (`scripts/check_lattice.py:482-485`; `scripts/check_run.py:1405-1408`).
  - The path scan leaves an integer `unit` alone (`scripts/check_run.py:154`).
  - It checks the plugin's report against its sha256, the replayed length and the identity labels (:949-960).
  - For a zero-step lattice phase it tests the labels itself (:1412-1418).
  - It holds a lattice `End` against the run's result and accepts it only with no lattice row (:797-799, :819-830).
    It counts the `End`'s attempts in the verdict (:861-867).
  - It checks the acceptance configuration (:297-323) and the D19 record (:326-337).
- The I0 tests that asserted the stubs' behaviour (K2) now test the real plugins or use an explicit stub plugin
  (`stub_lattice`, `scripts/test_runner.py:707-710`).

**Review findings fixed in I1** (`PLAN.md`, revision 3e, E1). One Codex implement packet (FIX) took the fixes inside
the plugin and checker modules and their tests. It ran through the wrapper from `8ae8c4a` (call
20261022T015515Z-4f47c9d6). The run was valid: nine files changed, all of them allowed. Its acceptance check, the five
packet test files, passed in Codex's sandbox. The integrator applied its patch unchanged. Then the integrator adapted
`test_rules_kernel.py` and `test_lattice_phase.py` to the runner's changes (the acceptance configuration, the D19
record and the lattice report). The integrator took the runner and `check_run.py` sides (above).

- P3-falsepass-01 (`major`) and P3-contract-01. `lattice_phase.py` now checks the restorer run first: its input, its
  input digest, its library digest and, for schedule11, its realignment digest. Only then does it read the status
  (`scripts/lattice_phase.py:523-541`). So only a failure certificate for this input and library ends the phase
  `stuck` (`restorer_no_solution`); a mismatch is `error` (`restorer_output_invalid`). Test G gives a failure
  certificate with each of these checks broken in turn, for both restorers
  (`scripts/test_lattice_phase.py:799-838`).
- P1, `rules_height.py`:
  - a parse failure or a crash is decided before a wall-clock kill (P1-contract-01, P1-falsepass-02;
    `scripts/rules_height.py:411-420`);
  - a DONE shard must report exactly its own first-letter indices, and all shards DONE with the pair undecided is
    `error` (`inconsistent_output`). `budget` needs a shard `budget`, and `timeout` a shard killed by the clock
    (P1-falsepass-01, P1-robust-03; :413-418, :523-528);
  - an end line other than a BUDGET line needs a START line. With no step, the end line must repeat START's tau and u
    (`r22d75` excepted), and for pairs its tau must be the target (P1-falsepass-03; :439-450);
  - a shard error, or an exception while the shards run, stops the other shards (P1-robust-01; :490-512);
  - test 5 also drops a class-B FIRST line (P1-contract-03; `scripts/test_rules_height.py:308`), and tests 14 to 19
    cover the rest (:696-809).
- P2, `rules_kernel.py`: the kernel's `--cache` is a directory inside the call's work directory. When the runner's
  cache has `home_centroids.npy`, the directory gets a link or a copy of it (P2-robust-01;
  `scripts/rules_kernel.py:163-173`). The test module removes its temporary tree unless `KEEP_TEMP=1` (P2-robust-02;
  `scripts/test_rules_kernel.py:958`).
- P4, tests only: test 6 adds forward clean-add rows of `r22d81/H2` and `h4/L2` whose u rises
  (`scripts/test_check_contracts.py:407-410`). Test 9 adds an `h4/L4` row whose end state exceeds the maximum (:436).
  These are P4-contract-01 and P4-falsepass-01.
- P5, `check_lattice.py`:
  - the replay pass and the range pass hash and count the bytes they read. The checker compares them with the row's
    `word_sha256` and `letters` (P5-falsepass-01, P5-robust-01; `scripts/check_lattice.py:176-185`, :438-466);
  - after a model segment the rebuilt state is kept even when its labels or frames disagree, and the step fails
    `model_labels` (P5-contract-01; :303-327, :404-412);
  - test D prints the model loop's own rate next to the whole step's rate (P5-contract-02).
- Runner and checker: P1-robust-02 (`RLIMIT_CORE`), P1-robust-04 and P3-robust-01 (the signal check),
  P4-falsepass-02 (the high-phase bound), K1 and K3.1 (above).

That is the `major` finding and 20 of the 21 `minor` findings.

What stays deferred, as `PLAN.md` E1 says:

- P1-contract-02: a Traceback line after a valid end line still overrides it, so such a child ends `error`, not with
  the end line's result. This is conservative and cannot pass a false record.
- The nits, except two:
  - `RLIMIT_CORE` also covers P2-robust-04. The kernel and height children still run in the repository root
    (`scripts/runner.py:530`).
  - P3-robust-02 is fixed: P3's test module removes its temporary tree in a `finally` block unless `KEEP_TEMP=1`
    (`scripts/test_lattice_phase.py:1123`).

The other integration items, K2 and K3.2 to K3.6, are done by the changes listed under "What I1 integrated".

## 9. Development runs

The plan runs D0 first (`PLAN.md`, revision 3d section 4 and revision 3e, E5). D0 is seeds 7001 and 7106 with the late
lattice restorer, then seed 7106 with schedule11. The integrator chose the acceptance restorer from D0's measured
twist counts and times, before batch 1 (below). D1 (seed 7101, depth 40) and D2 follow.
D2 is depth 3000: the stops of seeds 7302 and 7304 to 7307, and seed 7303 from its scramble. All times are cloud
timings of research code on a headless Linux machine shared with other jobs.

**D0** (22 October 2026, measured):

- `--scramble 10 7001 --mode acceptance`:
  - nine bulk steps of one letter each, all `seam`; the bulk ends `lattice` at the solved digest;
  - height, high and tail are `not_applicable`, and the lattice phase is not entered;
  - result and verdict `solved`, 38.2 s.
- `--scramble 20 7106 --mode acceptance`, with the late restorer (the value `acceptance-config.json` held before E5):
  - start digest `9ad04d45…`, 51,828 pieces off, all at height 2;
  - 19 bulk steps of one letter each, all `seam`; the bulk ends `lattice` at `1b263d3e…`, and height, high and tail
    are `not_applicable`;
  - the lattice phase is `solved` in 35 steps, one per orbit of the cascade. There is no correction step: the
    restorer scheduled none. The phase is 8,701,310 twists, each played twist by twist in the plugin's child;
  - restorer 544.6 s. Player 244.8 s, of which the retained part took 236.7 s, about 36,800 twists per second, with
    poses moved twist by twist (section 4). Runner 857 s in all;
  - final digest `a71580bf…`, with identity labels. It is not the solved digest, because labels do not fix exact poses
    (section 4). It is the digest that a scratch play of this word gave before I1 (`PLAN.md`, section 4, "D0
    regression");
  - result and verdict `solved`. `check_run.py` replays all 54 words twist by twist. It passes the record with verdict
    `solved`, end digest `a71580bf…` and 0 unchecked contracts, in about 270 s.
  - The play and the check took less time than the plan's estimates (about 6 and 7 minutes, `PLAN.md`, section 4).
  - The run has the same start digest, the same 19 `seam` twists and the same lattice length as the committed format 1
    record of section 5. That record's lattice phase was checked by composition through `verify_late.py`, not played.
- `--scramble 20 7106 --lattice-restorer schedule11` (development mode):
  - the same start and the same 19 `seam` bulk steps to `1b263d3e…`; height, high and tail are `not_applicable`;
  - the lattice phase is `solved` in 35 steps, one per orbit of the cascade in the same order, and no correction
    step. The phase is 1,516,410 twists;
  - restorer 575.8 s. Player 65.2 s, of which the retained part took 57.2 s. Runner 720 s in all;
  - final digest `71dbda5c…`, with identity labels. It differs from the late run's end digest: both end on the
    lattice with identity labels, in different invisible rotations (section 4);
  - result and verdict `solved`. `check_run.py` replays all 54 words and passes the record with verdict `solved`,
    end digest `71dbda5c…` and 0 unchecked contracts, in 78 s.

**Bounds** (stage 09, `09-general-method/METHOD.md:175`, "Lattice-phase length"). On the committed cascade the late
restorer needs at most 1,324,660,182 retained twists. The schedule of round 11 (schedule11) needs at most 20,866,844.
The corrections add at most 25,852 twists to either. D0's late run used 8,701,310 retained twists and its schedule11
run 1,516,410, neither with a correction.

**The acceptance restorer (E5).** The integrator chose `schedule11` from D0, and `acceptance-config.json` and the
runner's default now name it. On seed 7106 it played 5.7 times fewer lattice twists than `late` (1,516,410 against
8,701,310). Its play took 65 s against 245 s and its check 78 s against 271 s, and its restore took about as long
(576 s against 545 s). Its proved bound is 63 times smaller. `late` stays available in development mode. Every
acceptance batch uses `schedule11`.

**Provenance.** The D0 records are not in the repository. They ran on uncommitted I1 candidates over `8ae8c4a`, so
`config.source.dirty` is true. Their `config.scripts` show `rules_height.py`, `rules_kernel.py` and `lattice_phase.py`
as committed at `8ae8c4a`, before the FIX patch of section 8, and a `runner.py` that has changed since. D0 is to be
repeated on the committed head before anything relies on these numbers. D1, D2 and the acceptance batches have not
been run.

## Replay

From the repository root, with Python 3 and NumPy, `S=research/jumbling/restoration/13-tutorial-runner/scripts`:

    for t in rules_height rules_kernel lattice_phase check_contracts check_lattice literal_lattice check_r22d25 \
             runner check_run; do python3 -I -B $S/test_$t.py; done   # times in section 5
    python3 -I -B $S/runner.py --scramble 10 7001 --out <new dir> --cache <new dir> [--mode acceptance]
    python3 -I -B $S/runner.py --scramble 20 7106 --out <new dir> --cache <new dir> --mode acceptance  # about 12 minutes
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
| `PLAN.md` | The plan for record format `runner-run/3`: revision 3d (20 October 2026; the senior plan check and two scoped verifications), followed by the revision 3e addendum of 22 October 2026 (E1 to E6). The addendum covers the review findings fixed in I1, the lattice phase after P3, the frozen acceptance configuration, `conj` trees, the order of the development runs and small items |
| `acceptance-config.json` | The options every acceptance run uses (format `acceptance-config/1`; `PLAN.md`, E3): `literal_limit` 10,000,000, `seeds` `atlas`, the four tail rules, `lattice_restorer` `schedule11` (chosen under E5, section 9), `fallback_max_length` 1, `par` 1 and `pair_shards` 4. It sets no bulk, tail or high step budget and sets `no_lattice` false. `tail_steps` 0 means no budget (`scripts/runner.py:1264`), and the record keeps it as null (:464-465). `--mode acceptance` refuses any other value (section 8) |
| `scripts/runner.py` | The tutorial runner. RUN-1. RUN-3 on 20 October: the certificate path of rule 3, record format 2, signals, run-relative lattice paths and the source identity. The integrator's fixes after review: the lattice programs run without a time limit again (RUN3-contract-01, RUN3-robust-01); the bulk child watches the runner's process id (RUN3-robust-02); the source identity is taken before the run writes, with git run without optional locks (RUN3-contract-04, RUN3-robust-03, RUN3-robust-06). I0 on 20 October: record format 3, the fixed phase order with its entry tests and phase results, the bulk without reopen, acceptance and development modes, the clocks and timed-out attempts, the probe, the verdict, step trees and the plugin calls. The fixes of the I0 review: `RunError` for crashes, malformed output and digest mismatches, and the runner's side of a played lattice phase (I0-01, I0-02). The fix of its scoped verification: the certificate gate checks its evidence before strict′. I1 on 22 October: the lattice phase played by P3's plugin and recorded with its report file, lattice `End`s and a zero-step lattice result (`PLAN.md`, E2); the frozen acceptance options (E3); `config.kernel.d19`; the tree of a bulk `conj` step (E4); the reworded reopen comment (E6); `RLIMIT_CORE` 0 and a signal check after every plugin call (P1-robust-02, P1-robust-04, P3-robust-01) |
| `scripts/runner_api.py` | The plugin interface (I0): `Proposal`, `End`, `LatticeStep`, `LatticeResult`, `Ctx`, the clock adapter, the step-tree helpers and the `ia16` writer and reader (section 4) |
| `scripts/rules_height.py`, `scripts/rules_kernel.py`, `scripts/lattice_phase.py` | The plugins of the height, high and lattice phases (section 4). They were stubs in I0. Packets P1 to P3 (Codex, 20 October) replaced them, applied unchanged at `8ae8c4a`. In I1, Codex's FIX patch fixed the findings of section 8: P1's output and shard rules, the kernel's own cache directory, and P3-falsepass-01 with P3-contract-01 |
| `scripts/test_rules_height.py`, `scripts/test_rules_kernel.py`, `scripts/test_lattice_phase.py` | The acceptance tests of P1 to P3 (`KERNEL_SLOW=1` and `LATTICE_PHASE_SLOW=1` add slow cases). The FIX patch added cases for its fixes and made the P2 and P3 modules remove their temporary trees unless `KEEP_TEMP=1`. The integrator adapted the P2 and P3 modules to the acceptance configuration, the D19 record and the lattice report |
| `packets/` | Packets P1 to P5 as sent to Codex (`P<n>.md`), their change logs (`P<n>-changes.md`), Codex's changes (`P<n>.patch`, made against `97815b1` and applied unchanged at `8ae8c4a`), Codex's reports (`P<n>-report.md`), and the integrator's review of the five patches with the integration checks K1 to K3 (`REVIEW.md`; section 8) |
| `scripts/test_runner.py` | Its acceptance tests (RUN-1, extended by RUN-3, by I0 and by I1). I1 adds tests for a lattice phase with no step, lattice `End`s, the acceptance configuration and `config.kernel.d19`, the real kernel end to end (`ExactTests.test_11`), and P3's synthetic and w9 fixtures through the runner and the checker (`LatticeEndToEndTests`). The integrator made the worktree check also skip the private work trees, tolerate files that vanish and run git without optional locks. A run over 60 minutes is a note, not a failure (RUN3-robust-05) |
| `scripts/check_run.py` | The independent record checker. RUN-2. RUN-4 on 20 October: both formats, the format 2 checks, and a time limit and saved output for `verify_late.py`. I0 on 20 October: format 3, with the fixes of its review for phases left out and for the probe (I0-03, I0-04) and for a phase that an exception cut off after its steps (V1-B2), and the fix of its scoped verification for that phase's start and entry test (I0-V1-01). I1 on 22 October: P4's high-phase bound (P4-falsepass-02), the result of a played lattice phase (K1), integer `unit` values in the path scan (K3.1), the plugin's lattice report, lattice `End`s and a zero-step lattice phase, the acceptance configuration and the D19 record. Two fixes by the integrator after RUN-2's review: every string in the record is scanned for an absolute, home-relative, Windows or parent path (RV2-01), and a corrupt gzip body fails as the file it is in (RV2-04) |
| `scripts/check_contracts.py`, `scripts/check_lattice.py` | The checker's height and kernel contracts (P4) and its literal lattice replay (P5). They were stubs in I0. Packets P4 and P5 replaced them, applied unchanged at `8ae8c4a`. `check_lattice.py` has the FIX patch of I1 (P5-falsepass-01, P5-robust-01, P5-contract-01) |
| `scripts/test_check_contracts.py`, `scripts/test_check_lattice.py` | The acceptance tests of P4 and P5, with the FIX patch's cases: P4-contract-01, P4-falsepass-01, P5-falsepass-01, P5-contract-01, and the model loop's own rate in test D (P5-contract-02) |
| `scripts/test_check_run.py` | Its acceptance tests (RUN-2, extended by RUN-4, by I0 and by I1, with the independence scan of section 3). I1 adds tests for the height contracts, the high-phase bound, lattice `End`s, a zero-step lattice phase, and a record of the real height plugin at the R22D-36 state. It also extends the acceptance-configuration test to the frozen options and the D19 record. On 19 October the integrator raised the timeout of the lattice fixture from 1,100 to 3,600 seconds, after the first acceptance run timed out on a loaded machine. On 20 October its worktree check was made to leave out the private work trees and to run git without optional locks (RUN4-robust-05) |
| `scripts/literal_lattice.py` | The literal lattice player (LLP, R22D-57; section 4), with P3's `observer` keyword (applied at `8ae8c4a`) and the integrator's fixes for the committed record's solution key (LLP-contract-01, LLP-robust-02) and for the played final digest at a retained-first junction (LLP-false-pass-01, LLP-contract-02) |
| `scripts/test_literal_lattice.py` | Its acceptance tests, with the integrator's narrower worktree snapshot (LLP-robust-01, IF-02) and two added tests (LLP-contract-01, IF-01) |
| `scripts/check_r22d25.py` | The independent checker of `r22d25-run/1` records (packet R22D-25, with the kernel of stage 12). The integrator's fixes: only `assets/manifest.json` must match its recorded digest (R22D25-robust-02, R22D25-contract-04), and an `unsupported` record must show its cause (R22D25-false-pass-05) |
| `scripts/test_check_r22d25.py` | Its tests: format checks and record mutations, run by stage 12's `test_r22d25.py` or on their own |
| `runs/depth10-seed7001/`, `runs/depth20-seed7106/` | The two format 1 records of section 5: `run.json`, the words, the lattice phase's summary, report and solutions (gzipped), the logs, the runner's last line (`runner.json`) and the checker's output (`check.jsonl`) |

## Checks not run

- Windows, DirectX, the native host, input, long-session and performance checks: none. Timings are cloud timings of
  research code with other jobs on the machine.
- The slow tests:
  - `RUNNER_SLOW=1`: the depth 20 run through the runner. The test still asserts the `lattice` block of the
    composition path (`scripts/test_runner.py:3033-3036`), which a run with the real lattice plugin does not write;
  - `LATTICE_SLOW=1`: the player on the committed depth 20 record;
  - `KERNEL_SLOW=1`: P2, the real D19 after the F1 step;
  - `LATTICE_PHASE_SLOW=1`: P3, schedule11 on the rebuilt seed 7106 hand-over;
  - stage 12's `R22D25_SLOW=1`.
- The current `check_run.py` on the two committed format 1 records.
- A runner run from a scramble or a census stop with any of these: a height, high, tail, fallback or certificate
  step, or a lattice phase that ends with an `End` or with nothing scheduled. The D0 runs have none of them
  (section 9).
- D0 on the committed head, D1, D2 and the acceptance batches (section 9).
- The fast review of the fixes covered the fixes only, not the whole patches. The fixes for IF-01 to IF-03 (in
  `test_literal_lattice.py` and stage 09's `test_sched11_late.py`) and the fix to `test_check_run.py`
  (RUN4-robust-05) came after it and have had no review.
