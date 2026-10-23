# Plan: connect the existing rules to the tutorial runner (runner-run/3)

This is the plan the integrator follows for record format `runner-run/3`. Revision 3d passed a senior plan check and
two scoped verifications on 20 October 2026 (the change notes at the end of revision 3d list the findings and their
dispositions). Revision 3e, an addendum written when the owner restarted the work for the tutorial and its acceptance
on 22 October 2026, passed a senior scoped re-check of its changes with no findings. Section 4 of revision 3d (I1) with
the addendum's items E1 to E6 is what the integrator implemented on 22 October 2026 (README, sections 2 to 9).

# Revision 3d

Base: `S = research/jumbling/restoration/13-tutorial-runner/scripts`. Nothing in this plan edits a critical-path file or a rule program (stage 12 `round22/*.py` and `cloud/**`, the stage 09 restorers, `verify_late.py`, the census scripts). The plan adds new modules and the integrator's hooks. The only other edit is one keyword in `literal_lattice.py`, and its default keeps today's behaviour (P3).

Authority: the owner's decision `docs/wiki/decisions/owner-decisions-2026-10-20-tutorial-acceptance.md`. That page covers:
- the acceptance test;
- no time limit (:68);
- the two readings the owner accepted on 20 October 2026, on what the checker may share and on chapters in order (:70-81);
- the reopen default (:82-85);
- the step tree (:87-96).

## 0. Design decisions

**Phase order (D3; owner reading 2, decision page :77-80).** The order is bulk → height → high → tail → lattice:
- height has three sub-phases: `r22d75` → `r22d81` → `h4`;
- high is the kernel with D19;
- in development mode the tail includes the fallback.

How the phases work:
- The runner goes through the phases in this fixed order and never goes back.
- A phase starts at the first state where the previous phase finds no step.
- Inside a phase, the first rule that fires takes the step. Every rule reads only the current state.
- Each step records its phase and sub-phase. README §1 does not describe sub-phases today, so I0 adds this to README §1.
- So the rule that applies can depend on the path. At `bbef1c8e`, R22D-75 fires with (23,14) (`12-low-height/runs/round22/cloud/height4-descent-bbef1c8e/r22d75_from_snapshot.txt`). A run that reaches `bbef1c8e` inside `r22d81` does not go back to `r22d75`. Today's bulk → tail order works the same way, as the decision page says (:80).
- Entry tests, all computable without rule code:
  - `r22d75`: max height ≤ 6 and n4 + n6 > 0;
  - `r22d81`: max ≤ 6 and n6 > 0;
  - `h4`: max ≤ 4 and n4 > 0, so n6 = 0 follows (`h4_unlock.py:105-107` asserts max ≤ 4);
  - `high`: max ≤ 6 and n4 + n6 > 0;
  - `tail`: off > 0;
  - `lattice`: off = 0.
  - A failed entry test gives `not_applicable`.
- Termination: no phase is entered twice, and each phase lowers its own potential:
  - bulk: N;
  - `r22d75`: (n6, n4, n2), lexicographically;
  - `r22d81`: n6·B³ + u·B² + n4·B + n2;
  - `h4`: n4·B² + u·B + n2;
  - high: (off, n6), lexicographically. Under the kernel's contract (P4), a landing lowers off and moves no other off piece except to land it. A lowering takes no lattice piece off and lowers n6.
  - tail: off.
- U2 runs after R22D-81's H4 and H2. R22D-88 uses a different order (H6, U, U2, H4, H2), so U2 here is recorded as its own rule revision.

**Bulk order without reopen (D2; the integrator default on the decision page, :82-85).**
- `reopen` reads the solver's own earlier twists, `restorer.moves` (`theory/seams.py:285-308`), not just the current state.
- `SeamRestorer` takes an `order` argument (`seams.py:97`; `restore.py:95-97`). Today the bulk child uses the default order and asserts it (`runner.py:961-962`).
- v3 passes the order seam, cover, close, conj, lock, improve, shift and records it in `config.bulk_order` (`runner.py:317`).
- `reopen` never found the next step first in any census run (`11-scramble-census/README.md:150-151`).
- I0 corrects the stage 11 sentence "Its rules read only the configuration" (same README, :22-23).

**Clocks (D5, D1; R3-01).** The owner set no time limit (decision page :68). There are two kinds of clock: the runner's own wall clocks, and clocks inside the child programs, which this plan may not edit.
- **The runner's own clocks.** `--bulk-seconds`, `--bulk-step-seconds`, `--rule-seconds` and the wall-clock kill of a height or kernel call. In acceptance mode they are off: `execute(..., seconds=None)` (`runner.py:330-355` already treats None as no clock, and `script(..., unlimited=True)` at :357-361 passes None).
- **Child clocks: one adapter.** A child clock cannot be passed as null or infinity: the children reject both (table below). So `runner_api.child_clock(name, value)` gives every child clock argument its value in development mode and the sentinel `CLOCK_SENTINEL = 10⁹` s (about 31.7 years) in acceptance mode. Every call site goes through it. `config.budgets` records null for each unlimited clock, and `config.clock_sentinel` records 10⁹.
- **Inventory of child clocks** (integrator experiment, 20 October 2026, scratch probe; each child run with the argument and a missing input file, so it stops at its input after its clock setup):

| Child (who calls it) | Argument | What the child does with it | 10⁹ | inf |
|---|---|---|---|---|
| `r22d25_search.py` (P2) | `--seconds` | `Budget`: rejects a non-finite or negative value (:63-65); deadline = monotonic + s; `None` raises `TypeError` | accepted | `--seconds must be finite and nonnegative` |
| D19 inside the kernel | `--seconds str(remaining)`; `subprocess.run(..., stdout=DEVNULL, stderr=DEVNULL, timeout=remaining)` (`r22d25_search.py:345-348`) | finite and positive required (`r22d19_rule.py:684`) | accepted (`999999999.123`); `subprocess.run` without pipes accepts `timeout=1e9` | `--seconds must be finite and positive` |
| `r22d81_rule.py`, `h4_unlock.py` (P1) | `--cpu` | `setrlimit(RLIMIT_CPU, (int(cpu)+5390, int(cpu)+5400))` before input (`r22d81_rule.py:48`, `h4_unlock.py:48`), and the scripts' own budget test | accepted | `OverflowError` at `int(inf)` |
| `two_letter_u.py`, `r22d75_from_snapshot.py` (P1) | `--cpu` | the same with +1190/+1200 (`two_letter_u.py:44`, `r22d75_from_snapshot.py:24`) | accepted | `OverflowError` |
| `r22d81_pairs.py` (P1) | `--cpu` | `soft = int(T + cpu) + 600` after loading (:71-72) | accepted (expression run alone: limit (1,000,000,600, 1,000,000,610)) | `OverflowError` |
| `tclose2.py` (tail) | `--greedy-seconds` (`runner.py:451`) | deadline; `TimeoutError` in `gain`, caught; writes `pre.greedy.timed_out` (`tclose2.py:171-186`) | accepted (float) | accepted, never times out |
| `r22b10_cert.py` (tail) | `--seconds` (`runner.py:519`) | finite and positive required (:857-858); `setitimer(ITIMER_REAL, ...)` | accepted | `--seconds must be positive and finite` |
| `literal_lattice.play` (P3) | `seconds` | None means no deadline (:402, :480) | — | — (pass None) |
| `jordan_target.py` (tail) | none | hard-coded `RLIMIT_CPU (840, 850)`; on SIGXCPU prints `CUT_SHORT_CPU_840_SECONDS` and exits 124 (:10-14) | cannot be lifted | — |

- **A measured limit on the sentinel.** On this Linux CPython 3.13, `subprocess.run(..., capture_output=True, timeout=t)` raises `OverflowError: timeout is too large` for t ≥ 10⁷ (epoll's millisecond limit, about 24.8 days); without pipes it accepts 10⁹. No path in the table passes the sentinel to a capturing call. A new child path gets the sentinel only after a test like the ones below.
- **Timed-out attempts.** In both modes an attempt that ends on any clock records `timed_out: true`:
  - the runner's kill (`Child.timeout`);
  - a child's own budget end: `BUDGET` lines; the kernel's `budget` with a clock cursor;
  - an `r22b10_cert` result with `complete` false where any of these holds (R3B-01, R3C-01):
    - (a) the child's log has a row with `timeout: true` (`r22b10_cert.py:889`);
    - (b) a `missing[].reason` contains one of the builder's deadline phrases `wall-clock budget exhausted`, `serialization budget exhausted` or `exceeded budget` (the phrases at `r22b10_cert.py:67`, `:72`, `:741`, `:860`, `:888`, `:909`, `:912`);
    - (c) the result's `seconds` is at least `s − min(10, s/10)`, where `s` is the `--seconds` value the runner passed; when `seconds` is missing or not a finite number, the runner's own measured wall time of the child is used instead, which is never smaller. `s − min(10, s/10)` is the builder's construction deadline `end` (`:866`). Its serialization deadline `finish = T0 + s − min(0.05, s/100)` (`:901`) is later, and the serialization fallback records `seconds` measured after the deadline fired (`:913-915`), so (c) holds for every serialization expiry. That includes the case R3C-01 found, where no orbit was built, the copied missing reasons are not deadline phrases (`:909-912`) and no `timeout` row is written. In the normal path `seconds` is measured before `check_time(finish)` passes (`:896`, `:904`), so it is below `finish − T0`.

    The per-orbit deadline (`:779-783`, caught at `:824-825`) ends an orbit before `end` and is covered by (b): its reason is `<phase>: wall-clock budget exhausted`. One of (a), (b) or (c) marks the whole attempt `timed_out`, with outcome `timeout`, even when another orbit failed for another reason, because the certificate's completeness is then unknown. (c) can also mark a non-timeout failure that ended within the last `min(10, s/10)` seconds of the budget; that errs toward `budget`, never toward `stuck` (R22D-55). In acceptance mode the builder's clock is the sentinel, so each orbit's share is at least 10⁹ s divided by the number of target orbits, and these reasons are not expected; the rule holds in both modes. The builder's two fixed size caps (`straight-line program node memory budget reached`, `:123`, 10⁶ nodes; `word-table generator memory budget reached before connectivity`, `:316`) are not clocks: they are part of the builder's definition, give the same result at the same state with any amount of time, and so give `certificate_needed`, with the reason recorded;
  - `tclose2`'s `pre.greedy.timed_out` true (today the runner does not read it; I0 adds it);
  - `jordan_target` ending with `CUT_SHORT_CPU_840_SECONDS` before any `CONDITION` line.
  
  Under `PROOFS-B.md` R22D-55 a stop with a timed-out attempt is never read as stuck (verdict below). In acceptance mode only `jordan_target`'s fixed cap can still cut an attempt; such cuts are counted in `acceptance.timed_out_attempts` and make the verdict `budget`, never `stuck`.
- Development mode keeps explicit clocks.
- **Tests (mocked clocks; I0 for the runner side, P1 and P2 for their callers):**
  1. the adapter: acceptance gives 10⁹ for every child argument and null in `config.budgets`; development passes the values through;
  2. the real `r22d25_search.Budget(10⁹)`, and the kernel's D19 call with a stub D19 that records its `--seconds` and exits: the value is finite and above 10⁸, and the call returns (P2);
  3. each height script run with `--cpu 1e9` on a missing snapshot ends with the input error, not `OverflowError` or `ValueError` (P1);
  4. `tclose2`'s greedy clock forced with a stub `Vacate2` whose `run` sleeps past a mocked deadline: in development the attempt records `timed_out: true`; in acceptance, with the clock mocked forward by 10⁷ s, the timeout branch is not taken (I0);
  5. a stub `jordan_target` that prints `CUT_SHORT_CPU_840_SECONDS` and exits 124 before `CONDITION` gives a `timed_out` attempt, not `no literal closure` (I0);
  6. (R3B-01) `CertificateStub` variants of `r22b10_cert` that exit 0 with `complete` false and no `timeout` row: `missing = [{least: 0, reason: 'seed: wall-clock budget exhausted'}]` (per-orbit), `[{least: 0, reason: 'serialization budget exhausted; constructed program omitted'}]` (serialization), and a mix of one deadline reason and one `seed: no small-support word found in single letters and bounded short products` reason (`r22b10_cert.py:533`). Each gives an attempt with `timed_out: true` and outcome `timeout`; with no other rule, probe or fallback word at that state, the tail ends `timeout` and the acceptance verdict is `budget`. A fourth variant with only that `seed:` reason (no deadline phrase, no `timeout` row) and a fifth with only `word tables: word-table generator memory budget reached before connectivity` each give `certificate_needed`, `timed_out: false`, and the verdict `stuck`. A sixth with `complete` false, no deadline phrase, and a `{"timeout": true}` log row gives `timed_out: true`. (R3C-01) Two emitter-derived fixtures run the unchanged `r22b10_cert.main` in memory: its loaders (`modules`, `screen.load`, `buffer.Alphabet`, `target_check`) are stubbed, `build` returns `orbits = []` and the single missing reason `seed: no small-support word found in single letters and bounded short products`, and the module's clock is a fake that the test controls. In the seventh, the first `write_result` call moves the fake clock past `finish` and raises `Deadline`, so the builder writes its serialization fallback; the captured record and log, fed to the runner's classifier, give `timed_out: true` and outcome `timeout`, and with no other step the verdict `budget`. In the eighth, the clock does not move and the first write succeeds; the same reason gives `certificate_needed`, `timed_out: false`, and the verdict `stuck` (I0).

**Phase results (D5).**

| Result | When it happens | What the run does |
|---|---|---|
| `lattice` | off = 0 after a step, including a kernel call whose single step lands every remaining piece | goes to the lattice phase; the phases in between are `not_applicable` |
| `handoff` | a sub-phase's end test holds (`r22d75` HEIGHT2, `r22d81` HANDOFF, `h4` HANDOFF) | goes to the next sub-phase or phase |
| `not_applicable` | the entry test fails | continues |
| `stuck` | no rule of the phase gives a step and no attempt at this state timed out. This includes `certificate_needed`, UNLOCKED at the start of a pair call, no target for the pair call, every pair shard done with no success, and the kernel exhausted (`stuck`) | continues; after the tail the run ends `stuck` |
| `budget` | a step budget ends the phase (`--bulk-steps`, a rule's BUDGET, the kernel's own budget, `--tail-steps`, `--no-lattice`) | after bulk: continues, and the run is labelled `component`; after any other phase: the run ends `budget` (R22D-55) |
| `timeout` | the clock cuts a step (`--bulk-seconds`, `--bulk-step-seconds`, `runner.py:392-393`; the wall-clock kill of a height or kernel call) | same as `budget` |
| `timeout` (tail) | no tail rule gives a step and at least one attempt at this state timed out (README §1 rule 4, `README.md:67-70`) | in development mode the fallback still runs first; if it takes no step, the run ends `budget`, not `stuck` |
| `error` | exception, crash, truncated child output, digest mismatch, kernel `error`, kernel `unsupported` (R3-03: `d19_missing` is a missing dependency, and `height_above_6` after the high entry test is an invariant failure) | the run ends `error`; no later phase runs |

**Acceptance labelling (D1).** A new option `--mode acceptance|development`, default `development`.
- Acceptance mode:
  - A closure whose V comes from a certificate (`runner.py:485`, `certificate_word`, :494-563) is expanded to a literal word by `expand_slp.py`, as today. The `slp_apply` gate still checks its contract (`closure_checked`, :556-560; `slp_checked`, :576-623). The runner then plays the word twist by twist through `record_step` and requires the gate's end digest; it never loads the `slp_apply` end table (`advance`, :806-809). This is the only path whose gate is `slp_apply` today. It is a literal step: `standing: rule`, with `construction: certificate` and the certificate's digest (integrator amendment A2, below).
  - Where no literal word comes out (the certificate is incomplete and not timed out under §0 Clocks, the check does not end in PASS, or the whole step is over `--literal-limit`, default 10⁷, `runner.py:1021`), it records a failed attempt with outcome `certificate_needed`, plus the ghost's T size and digest. That is the owner's "step known only by a certificate". A complete, checked certificate whose whole step is within the limit is never `certificate_needed` in acceptance mode: it is played (A2).
  - The fallback (`runner.py:850-858`) runs only as a probe. A word it finds goes into `stuck.probe` and is never played.
  - The clocks are unlimited (above).
- Development mode keeps today's behaviour:
  - certificate steps take the gate's end table (`runner.py:806-809`), with `standing: certificate`;
  - fallback steps are taken.
  - Nothing new plays certificate steps, so `test_05` and `test_06` (`test_runner.py:1059-1102`) and the patch at :660 keep working.
- Every step gets `standing` ∈ `rule | certificate | fallback`.
- The record gets `acceptance {verdict, certificate_steps, fallback_steps, probe, timed_out_attempts}`. Component runs keep these counts too. The verdict is the first match in this order, and the checker recomputes it:
  1. `error` if the result is error;
  2. `component` if `config.component` is set;
  3. `stuck` if any certificate step exists;
  4. `coverage_failure` if any fallback step exists or the probe found a word;
  5. `budget` if the result is budget, or if the result is stuck and an attempt at the final state timed out;
  6. `stuck` if the result is stuck;
  7. otherwise the result.

**Checker independence (D4; owner reading 1, decision page :74-76).**
- In the owner's words, the checker shares only the definition of the puzzle:
  - the model assets;
  - the simulator (`sim/`);
  - the I_a menu, `element` and `scramble` of `theory/restore.py`;
  - the trees;
  - the cut geometry used to count N (`rim.Domains(...).total_misalignment()`, `check_run.py:170-174`; the same N as `restore.py:173-175`).
- It shares no runner, rule, gate, player or verifier code: no `runner*.py`, `rules_*.py`, `lattice_phase.py`, `literal_lattice.py`, `eb6`, `seams`, `orient`, `buffer`, `r22d25_*` or `verify_late.py`.
- An AST scan in the checker-side tests enforces this. It allows these `restore` attributes:
  - `menu`, `element` and `scramble`, from the owner's list;
  - three names that `restore` re-exports from the simulator, which `check_run.py` uses today (:133, :296-297, :621):
    - `ctx` is `rim.ctx = sim.get_context()` (`restore.py:53`, `rim.py:29`);
    - `to_tuple` is `sim.kplus.to_tuple` (`restore.py:50`);
    - `transpose` is `exact.transpose` (`restore.py:49`), the exact arithmetic that `sim/` itself imports.
  - The scan lists these three by name, with that reason. Any other `restore` attribute fails it.

## 1. Record format `runner-run/3`

The checker accepts /1, /2 and /3. The v1 and v2 paths do not change.

**Top level.**
- `config` adds:
  - `mode`, and `component` (null, or the reason);
  - `bulk_order` and `phase_order`;
  - `kernel {seconds, wall_margin, d19, max_steps: 1}` and `lattice {restorer}`;
  - `budgets.{height_seconds, height_cpu, pair_shards}`, with null for an unlimited clock.
  - `scripts` hashes every plugin's `SCRIPTS`, D19 included.
- `phases` adds:
  - `height {result, steps, twists, start, end, sub: {r22d75|r22d81|h4: {result, steps, start, end}}}`;
  - `high` and `lattice`.
- `totals` adds `height_twists` and `high_twists`. `lattice_twists` is the sum of the lattice steps.
- `stuck` holds:
  - the snapshot, plus `phase`, `reason` and the attempts (each with `timed_out`);
  - an optional `probe {unit, word, letters, input_digest, output_digest}`.
- Run results stay `solved | stuck | budget | error`.

**Every step.**
- Fields: `index, phase, subphase, rule, word, letters, encoding (json|ia16), word_sha256, tree, contract, gate, standing, before{digest, off, heights}, after{…}, seconds, attempts`. Bulk steps keep `N`.
- The tree follows from the owner's reading of the tutorial's content (decision page :87-96): "the runner records the tree of every step it takes, and the checker checks that the tree expands to exactly the word it replays".

**Step tree (`tree`).**
- The row holds `tree {file, sha256, size}`:
  - `file` is `step-trees/NNNNN.json.gz` (gzip with mtime 0), checked as a path like the other `file` keys;
  - `sha256` is taken over the uncompressed bytes;
  - `size` is the number of parts.
- The file holds one part, the root. A part is `{name, letters, parts: [part, …]}` or `{name, letters, span: [from, to]}`.
  - `span` is a letter range of the step's word, end excluded.
  - A part may also carry `inverse_of: [i, j, …]`: the index path from the root to an earlier part (earlier in depth-first order, so its span ends at or before this part's start) with the same number of letters. Its letters are that part's letters, inverted and in reverse order. Examples are V⁻¹ of a closure, which names V, and the trailing u^(us) factor of a `conj` node, which names the leading u^(−us) factor (R3-02). A reference to a later part fails as `tree_malformed`.
  - A part may instead carry `same_as: [i, j, …]`, the index path to an earlier part with the same name and number of letters. Its letters equal that part's letters. This marks a macro the step uses again, for example the same seed or correction macro, so that the tutorial can name it once (integrator amendment).
- No key is named `slp`, `node`, `segments` or `certificate` (`check_run.py:40`).
- Names say what the part is, for example:
  - `V`, `W`, `V⁻¹`;
  - `S`, `T`;
  - `seed`, `conjugator`;
  - `orbit 12`, `factor 3`, `conj 418`, `library word 77`;
  - `correction w9`.
- A short step can be one part with one span. A span still expands to the twists played.
- The checker checks that the tree expands to exactly the replayed word:
  - the root's `letters` equals the row's `letters`;
  - every part has a non-empty name and exactly one of `parts` (non-empty) or `span`;
  - a part's `letters` is the sum of its parts' letters, or `to − from` for a span;
  - the spans, in depth-first order, are contiguous from 0 to `letters`;
  - `size` matches, and the depth is at most 64;
  - every `inverse_of` range equals the inverse of the range it names, letter by letter, through the menu's inverse table (`check_run.py:296-297`);
  - every `same_as` range equals the range it names, letter by letter, and the two parts have the same name.

**Height fields and contracts (m1).**

| Rule | tau_before | tau_after | What the checker checks |
|---|---|---|---|
| `r22d75/T0..T2` | null | null | max ≤ 6; (n6, n4, n2) falls lexicographically, first at the tier's own count. No u or pose check: T1 keeps n6, but not the height-6 poses |
| `r22d81/H6` | target | recomputed (`r22d81_rule.py:300-301`) | max ≤ 6; n6 falls; u_before counted at tau_before, u_after at tau_after |
| `r22d81/U` | tau | same | max ≤ 6; n6 and every height-6 pose kept; u(tau) falls |
| `r22d81/H4`, `/H2` | tau | same | as U, except u must not rise; n4 falls (H4), or n4 kept and n2 falls (H2) |
| `r22d88/U`, `/U2` | R22D-81's tau at its stuck state (never null: a null tau ends the sub-phase `stuck`, reason `no_target`) | same | after every letter: max ≤ 6, n6 and height-6 poses kept; u(tau) falls; U2 records `u_mid` |
| `h4/L4` | target | recomputed (`h4_unlock.py:311-312`) | max ≤ 4; n4 falls |
| `h4/U`, `/L2` | tau | same | max ≤ 4; n4 and height-4 poses kept; U: u falls; L2: u does not rise and n2 falls |
| `h4/pair` | tau (recomputed by `two_letter_u.py`) | same | after every letter: max ≤ 4, n4 and height-4 poses kept; u falls; records `u_mid` |

Every u is the checker's own count at the recorded tau. Poses are compared as exact matrices, because pose ids are renumbered (`check_run.py:336-338`).

**Other step kinds.**
- **High** (`r22d25/<family>`):
  - contract `kernel_landing` or `kernel_lowering`, from the kernel row's `kind`; gate `r22d25`;
  - the kernel row's own fields `family, kind, target, params, high_exact, off_before, off_after, heights_after` (`r22d25_search.py:578-582`), plus `kernel`.
  - `high_exact` is a bool: True keeps every other piece at height 4 or more exact (`r22d25_state.py:305`).
  - `params.high_exact_pieces` is present only for d19, ST and the T blocker path (`r22d25_search.py:379`, `:465`, `:479`).
  - The kernel has no `height_from` field. The target's source height comes from the state before the step.
- **Lattice** (`lattice/orbit-<o>`, `lattice/correction-<k>-<macro>`): contract `lattice_literal`, gate `player`, and `labels_identity` on the last row.
- **`ia16` words** (`words/NNNNN.ia16.gz`):
  - gzip with mtime 0 of little-endian uint16 codes `60·grip + item`, all below 36,000;
  - `word_sha256` is taken over the uncompressed bytes;
  - no key may be named `slp`, `node`, `segments` or `certificate` (`check_run.py:40`).

## 2. Integrator part I0 (before any packet)

Files the integrator changes:
- `S/runner.py`, `S/check_run.py` and a new `S/runner_api.py`;
- both test suites and the stubs;
- the stage 13 README: §1 (phases, sub-phases, clocks, timed-out attempts), §2 (the record), §3 (the checker), §4 (the player and the runner), §5 (tests) and §6;
- `11-scramble-census/README.md`.

**runner_api.py.**
- `Proposal(phase, subphase, rule, word, tree, contract, gate, fields, claimed_output_digest)`, where `tree` is optional.
- `End(result, reason, details, attempts)`.
- `Ctx(table, snapshot(), mechanics, work_dir, cache, execute, budgets)`.
- Plugin protocol: `next_step(ctx) -> Proposal | End`. The lattice plugin: `play(ctx) -> LatticeResult(steps, end_table, report)`, where every step carries its tree.
- The `ia16` writer and the step-tree writer.

**runner.py.**
- `execute(argv, tag, seconds, cwd, stop_when=None)` (M7):
  - while it polls (`runner.py:330-355`), it reads new log lines;
  - when `stop_when(line)` is true, it kills the process group and returns `Child(stopped=True)`;
  - `seconds=None` means no clock;
  - the timeout and cancel handling stays as it is.
- The phase dispatcher: the fixed order, the entry tests, the result table and `config.component`. The bulk child uses the v3 order.
- `record_step(proposal)`:
  1. snapshot the state;
  2. play the word with `apply_word`, twist by twist;
  3. require the claimed digest;
  4. write the word file;
  5. write the step tree (one part with one span named by the rule when the proposal gives none);
  6. export `tables/NNNNN.json` (m4; today only `advance()` exports it, `runner.py:821-823`);
  7. snapshot with heights, append the row, save.
- Trees of today's steps:
  - closure steps from their existing `parts` (V, W and V⁻¹, with `inverse_of`);
  - T1′ steps from their parts in the same way;
  - bulk and fallback steps as one span each. The bulk child reports only words, so a finer tree for `conj` (seed and conjugators) is listed in README §6 as not done.
- New labels and blocks: `--mode`, `standing`, `certificate_needed`, `timed_out`, the probe and `acceptance`.
- Acceptance mode sets the attempt clocks to unlimited (§0 Clocks).
- New options (m3): `--lattice-restorer late|schedule11`, `--height-seconds`, `--height-cpu`, `--pair-shards`, `--kernel-seconds`, `--kernel-wall-margin`, `--high-steps` (a step budget for the high phase, default none; it ends the phase `budget`).
- `runner_api.child_clock` (§0 Clocks) at every child clock argument, `tclose2`'s `pre.greedy.timed_out` and `jordan_target`'s `CUT_SHORT_CPU_840_SECONDS` read into the attempt's `timed_out`.
- Lattice phase:
  - The runner calls the plugin and checks its end table against the last step's `after` digest.
  - Until P3 is integrated, the stub returns `not_applicable` and today's `verify_late` path runs (m3), so `test_02_small_lattice_configuration` and `test_runner.py:404-463` still pass.
  - A record made that way gets `config.component = 'lattice_composition'`.

**check_run.py.**
- Accepts v3 (`check_run.py:519-524`) and extends `PHASES`.
- Checks that phases and sub-phases come in the fixed order.
- Recomputes every entry test and every `handoff` or `not_applicable` claim from its own snapshot. This includes `h4`'s max ≤ 4.
- Checks that every bulk rule is in `config.bulk_order`, with no `reopen` in v3.
- Does not re-derive stuck claims, because that would need rule code.
- Checks the step tree of every step (§1): the structure for every encoding, and the `inverse_of` ranges for `json` words. P5 checks `inverse_of` ranges on `ia16` words.
- Hooks (M6):
  - `check_contracts.prepare(row, st)` runs before the replay. It has to: `Snapshot` keeps the live state (`check_run.py:157`), and the replay changes it (619-628). `before.n()` (607) is called before the replay for the same reason.
  - `on_letter(ctx, k, st)` runs after each letter of a height step.
  - `finish(ctx, row, st)` returns the failures.
- `load_word` reads `ia16` through `check_lattice.read_ia16`. Lattice steps go to `check_lattice.replay_lattice_step(state, row, path, menu)`.
- The probe word is replayed on a copy of the end state under the fallback contract (`check_unit`, 308-342). A probe it cannot confirm fails with `probe`.
- The verdict is recomputed (§0 order).
- End digest:
  - Today only the lattice-by-composition path takes `SOLVED` without a replay (`check_run.py:674`). Every other path already uses the replayed digest and fails a solved record whose digest is not `SOLVED` (:672-673).
  - In v3, records with `config.component = 'lattice_composition'` keep the `verify_late` path and get `end_digest = null`. This lasts until the switch in I1.
  - A played lattice gives the replayed digest. A v3 record with a played lattice is solved when the last lattice row has `labels_identity` true, as the checker recomputes it; its `end_digest` is the replayed digest, which may differ from `SOLVED` by rotations invisible to the labels (the w9 case, `test_literal_lattice.py:638`). The v3 path does not apply today's `solved_digest` test (`check_run.py:672-673`) to such records (R3-05).

**Stubs.** `rules_height.py`, `rules_kernel.py` and `lattice_phase.py` return `End('not_applicable')`. `check_contracts.py` and `check_lattice.py` return `['unchecked_contract']`.

**New I0 tests (with stubs).**
- One test for each row of the result table, the tail `timeout` row included.
- Verdict order:
  - a component run that ends stuck gets `component`;
  - a stuck run with a timed-out attempt gets `budget`.
- In acceptance mode the attempt clocks are null in `config.budgets`, and the clock tests 1, 4 and 5 of §0 pass.
- A2 and its negative cases on `CertificateStub` (`test_runner.py:540-634`; R3B-02):
  - positive: the default stub (complete `r22b10/2` certificate, check PASS, V of two letters, a five-letter closure V·middle·V⁻¹, below the default `--literal-limit`) in acceptance mode is played through `apply_word`, never `slp_apply`, gets `standing: rule` and `construction: certificate`, and the checker replays it;
  - over limit: the same stub with `--literal-limit 4` gives `certificate_needed` (5 > 4), with no expansion call;
  - incomplete: the stub with `fault = 'incomplete'` and only a non-deadline missing reason gives `certificate_needed` (§0 test 6, fourth variant);
  - check: the stub with `fault = 'check'` (the log does not end in PASS) gives `certificate_needed`;
  - development mode on the default stub keeps today's path (`slp_apply` end table, `standing: certificate`), so `test_complete_certificate_formats_dispatch_and_advance` (`test_runner.py:647-665`) keeps passing unchanged.
- `stop_when` kills the group after the first STEP line, and cancel still works.
- A probe that finds a word gives `coverage_failure`.
- `record_step` exports the table and writes a tree.
- Step-tree checks:
  - a closure tree with `inverse_of` passes;
  - each of these fails: a wrong part sum, a gap, an overlap, a root shorter than the word, a wrong `inverse_of` range, an `inverse_of` that names a later part, a wrong sha.
- The bulk order has no `reopen`.
- A `--table` start at an intermediate state (D3, M4):
  - run A: `r22d75` is stuck, then an `r22d81` step reaches S2;
  - run B starts at S2, and its first height step is an `r22d75` step;
  - both records pass the checker, each with the fixed phase order.

**Before commit.**
- Run `test_runner.py` (1,222 s, README §5, `README.md:212`) and `test_check_run.py` (482 s, `README.md:216`).
- One routine fast review of I0.
- Then commit, because the wrapper needs a committed HEAD.

## 3. Codex implement packets (parallel; disjoint files; seven-part template)

Rules for every packet:
- Acceptance check: `["python3","-I","-B","research/jumbling/restoration/13-tutorial-runner/scripts/<test>.py"]`.
  - It runs from the repository root, headless.
  - It takes about 20 minutes or less. The wrapper stops a check at 1,800 s (`tools/agents/codex_review.py:97`), and all five checks run at the same time.
  - It ends with a check that the worktree is unchanged.
- Stop condition: *"the acceptance check passes; stop. If the work needs a change outside allowed_files, stop and report the interface change needed."*
- Checker-side packets follow D4. Tests may use rule programs and the runner as oracles.
- The times cited for fixtures are private measurements from an earlier acceptance log. I0 measures them again and records them in README §5 before a packet relies on them.

### P1 R-H: height rules, runner side
- **Allowed files:** `S/rules_height.py`, `S/test_rules_height.py`.
- **Snapshots:** writes the state as an `r22d31_snapshots.load()` pickle (the format of `r22d81_rule.py --save`). It loads only its own pickles.
- **Calls,** through `ctx.execute(..., stop_when=<STEP line>)`:
  - `cloud/height4-descent-bbef1c8e/r22d75_from_snapshot.py`;
  - `round22/r22d81_rule.py`, then `round22/r22d81_pairs.py --target <tau from R22D-81's STUCK line>`. `--target` is required (`r22d81_pairs.py:45`).
  - `cloud/.../h4_unlock.py`, then the `two_letter_u.py` shards. The shards print no STEP or STUCK line. They print CONTEXT, one FIRST line per first letter, and then DONE or BUDGET (`two_letter_u.py:16-22`, `:218-220`).
- **Parsing:**
  - four STEP dialects;
  - check `input_digest`;
  - tau_before and u_before come from START (`r22d81_rule.py:244`, `h4_unlock.py:259`); tau_after and u come from STEP.
  - Output with no DONE or BUDGET line is truncated. It is never read as exhaustion; under R22D-55 a stop is not exhaustion (`PROOFS-B.md:2413`).
- **Ends (M1):**
  - `r22d75` HEIGHT2, `r22d81_rule` HANDOFF and `h4_unlock` HANDOFF → `handoff`.
  - `r22d81_rule` STUCK with tau null (no target cap; `r22d81_rule.py:6-7`) → `stuck`, reason `no_target`. The pair script is not called.
  - `r22d81_pairs` UNLOCKED before any STEP (u = 0 at tau; `r22d81_pairs.py:197, 293-298`) → `stuck`, reason `unlocked_without_step`.
  - Every `two_letter_u` shard ends DONE and no FIRST line has a success → `stuck`, reason `pair_exhausted`.
  - Other ends:
    - STUCK → `stuck`;
    - BUDGET → `budget`;
    - NOT_APPLICABLE → `not_applicable`.
  - The runner's own wall-clock kill → `timeout`, with the attempt marked `timed_out`.
  - Any of these → `error`:
    - a crash;
    - a missing STEP;
    - a digest mismatch;
    - truncated output that the runner did not cause.
- **State tests before each call:**
  - max ≤ 4 before `h4` (the entry test; `h4_unlock.py:105-107` asserts it);
  - tau present before an L2 letter (`h4_unlock.py:314`);
  - a non-empty target table before `two_letter_u` (`min()` at :140);
  - tau not null before `r22d81_pairs`.
- **Pair choice:**
  - Apply the rule in `two_letter_u.py:19-22` over all shards' FIRST lines.
  - Return `budget` (or `timeout` after a clock kill) if a shard stops before the chosen index is decided.
  - The pair step gets a two-part tree (first letter, second letter).
- **Acceptance:**
  1. Parse the committed outputs and reproduce the recorded chosen pairs. The outputs are `r22d81_from_stop.txt`, the census-k3000 `r22d75` files, the `h4_unlock` runs and the `two_letter_u` rounds 1-4. Several committed shard files end with `Terminated` (round 2 and 3 shards, the part 2 files) or in the middle of a FIRST line (the part 1 files). The parser reports these as truncated and never as exhausted.
  2. At the R22D-36 fixture (`test_r22d25.py:35-42`, WORD36 from solved): `r22d75`'s first step is (12, 14), to `7cd868f6…` (3,098 off; heights {2: 3,097, 4: 1}), and its second (35, 13) ends HEIGHT2 at `3ab25d21…` (5,405 off, all at height 2) (integrator measurement, 20 October 2026). One `h4` call at `7cd868f6…` (max 4, n4 = 1): its end is mapped, and a STEP's replay matches its digest.
  3. The clock test 3 of §0 for all five height scripts.
  4. Stub scripts for every end, crash and mismatch, including:
     - UNLOCKED at the start;
     - STUCK with tau null;
     - all shards DONE with no success;
     - a truncated shard output.

### P2 R-K: kernel and D19, runner side
- **Allowed files:** `S/rules_kernel.py`, `S/test_rules_kernel.py`.
- **Call:** `r22d25_search.py run --table T --out <fresh dir> --cache C --seconds S --d19 round22/r22d19_rule.py --max-steps 1`, killed at S plus a margin (no kill in acceptance mode, where S is unlimited).
  - The kernel's `--seconds` does not cover snapshots. A one-second run took 107 s (private measurement; I0 measures it again).
- **Result mapping:**
  - One step with result `lattice` is a step (`r22d25_search.py:553-555`: off = 0 is tested before max_steps). The runner then reports the phase result `lattice`.
  - One step with result `budget` and ledger cursor `max_steps` (`r22d25_search.py:557-561`) is a step, not a budget end.
  - With zero steps:
    - `budget` with any other ledger cursor is the kernel's own clock and maps to `budget`;
    - `stuck` → `stuck`, reason `kernel_exhausted` (r22d25-cat/1 only);
    - `unsupported` → `error` (R3-03): reason `d19_missing` when the D19 program is absent (`r22d25_search.py:326`), and `invariant_height_above_6` for `height_above_6` (:412), which the high entry test excludes.
  - `error`, a missing `run.json` or a nonzero exit → `error`.
- **Proposals:**
  - Contract `kernel_landing` or `kernel_lowering`, from the row's `kind`.
  - The kernel row's fields `family, kind, target, params, high_exact, off_before, off_after, heights_after` (`r22d25_search.py:578-582`).
  - No `height_from`: the source height is read from the state.
  - ST steps get the tree `S`, `T` from `params.S_length` and `params.T_length` (:462-465); other families get one span.
- **Acceptance:**
  1. At R22D-36, one call gives F1 with a first letter at or before (12, 45), and the replay matches.
  2. Stub kernels for:
     - every result, one step with `lattice` included;
     - `unsupported`/`d19_missing` with a tail stub that would succeed: the run ends `error` and the tail is never called; a genuine `stuck` still hands over to the tail (R3-03);
     - a `budget` with each cursor;
     - the kill;
     - stale files in the output directory;
     - a kill before `run.json` exists.
  3. The clock tests 2 of §0 (the real `Budget(10⁹)` and the nested D19 call with a stub D19).
  4. `KERNEL_SLOW=1` (not part of the check) runs the real D19 after that step.

### P3 R-L: literal lattice phase, runner side (D6, M3)
- **Allowed files:** `S/lattice_phase.py`, `S/test_lattice_phase.py`, `S/literal_lattice.py`, `S/test_literal_lattice.py`.
- **Edit or wrap — chosen: edit.**
  - `play()` sets `defer_poses` for every `word` leaf (`literal_lattice.py:497`) and then applies one composed pose field (510-512).
  - A wrapper would have to copy `play()`'s 130 lines of input checks (350-479).
  - So P3 adds `play(..., observer=None)`. With `None`, behaviour is byte-for-byte the same as today.
  - With an observer, `defer_poses` stays False, so poses move twist by twist (:281). The observer receives each leaf's codes, each factor boundary, each correction twist and the final state.
- **Step trees:**
  - Each orbit step's tree comes from the `Program` nodes (`literal_lattice.py:115`): the orbit, then its factors, then the nodes (`conj`, `prod`), then the leaves (`word` and `path`) as spans.
  - A `conj` node expands to u^(−us)·x^(sign)·u^(us) (`Program.leaves`, `literal_lattice.py:185`). The trailing u^(us) part carries `inverse_of` the leading u^(−us) part; the played order does not change (R3-02).
  - A node whose subtree would take the step's tree over 10⁶ parts is written as one span named by its node, and the record counts the folded nodes. Such a span still expands to the twists played.
  - Each correction step's tree is the macro, named.
- **`lattice_phase.py`:**
  - runs the restorer: `restore_lattice_late.py`, or `schedule11/restore_schedule11_late.py` with `realign.json.gz`;
  - runs the play in a child process through `ctx.execute`;
  - streams the `ia16` word;
  - makes one step per cascade orbit and one per correction macro, with exact digests at each boundary;
  - writes each step's tree;
  - exports the end table;
  - requires result `solved`, `played == scheduled` and identity labels.
- **Acceptance check:** `S/test_lattice_phase.py`, about 20 minutes or less. It does not play the two 16,184,022-twist fixtures. The full `test_literal_lattice.py` took 468 s in this checkout and 1,525-1,578 s in earlier runs (`README.md:221-229`), so it runs in I1 as an integrator check.
  1. A short synthetic program (a word W from solved, with the program W⁻¹):
     - with an observer, `transport_word` is never called;
     - the final digest equals the deferred play's;
     - the stream's sha equals the test's own expansion;
     - the boundary digests equal a `State.apply` replay;
     - every step's tree expands to the streamed word, `inverse_of` ranges included;
     - a three-letter `conj` (x = [1], u = [2], us = ±1, factor sign ±1; all four cases, in `json` and `ia16`) gives a tree whose references point backward and whose expansion equals `Program.expand` (R3-02);
     - the w9 fixture of `test_literal_lattice.py:638` (no retained factors, correction w9 only, identity labels, final digest not SOLVED) through the observer: the step's after digest is the played digest (R3-05).
  2. A named fast subset of `test_literal_lattice.py`, run through an explicit test list: the expansion of every node kind and the pose transport on short words.
  3. Budget and tamper cases.
  4. `LATTICE_PHASE_SLOW=1` (not part of the check): a schedule11 run.

### P4 C-C: height and kernel contracts, checker side
- **Allowed files:** `S/check_contracts.py`, `S/test_check_contracts.py`.
- **Height:** `prepare`, `on_letter` and `finish` follow the §1 table. u comes from its own per-piece straddle test (`sim` kernel `Evaluator`, `pullback_form`).
- **Kernel,** checked from the replayed states as the kernel's own gate and checks (`r22d25_state.py:272-329`; `r22d25_search.py:584-589`):
  - no lattice piece goes off;
  - every other off piece is exact or landed;
  - if `high_exact` is True, every other piece at height 4 or more is exact;
  - the pieces in `params.high_exact_pieces`, when present, are exact;
  - for kind `landing`: the whole target lands, and off falls;
  - for kind `lowering`: the whole target is at height 6 before and height 4 after (the kernel lowers only from 6: `r22d25_search.py:215, 261, 296`), and n6 falls;
  - the high phase's letters stay within the kernel's bound, 6·n6₀ + 964·off₀ (`r22d25_search.py:584-585`, `DEFAULT_LIMITS['ST']` at :34). The bound is counted from the phase's start state, because each kernel call sees only one step.
- **Acceptance:**
  1. Its u equals `eb6.straddler_set` (used as an oracle in the test only) on all 600 caps, at R22D-36 and at the seed 7102 stop (about 243 s, private measurement).
  2. A positive (12, 45) row.
  3. Mutations of:
     - tau_before, tau_after, u and `u_mid`;
     - the top-height poses;
     - the height guard after letter 1;
     - `high_exact` and `high_exact_pieces`;
     - the target height;
     - off and n6 progress;
     - the letter bound.

### P5 C-L: literal lattice replay, checker side
- **Allowed files:** `S/check_lattice.py`, `S/test_check_lattice.py`.
- **Decoder:** its own `read_ia16`. It also has a range reader for `inverse_of` checks on `ia16` words, which makes a second pass over the file.
- **Fast exact model:**
  - labels gathered from `assets/primitives.npz`; poses by K+ place from `model.npz`;
  - used only while the state is on the lattice and the letter is a retained item at its grip;
  - any other letter goes through `State.apply` with the admissible and certain checks.
- **At each step end:** rebuild the State, compare digest and labels, report `labels_identity`.
- **Acceptance:**
  1. About 1,000 retained letters and one correction macro, made with `State.apply`, pass. Their trees include an `inverse_of` part.
  2. Each of these fails:
     - a changed letter;
     - truncation;
     - a wrong sha;
     - a wrong after digest;
     - a non-retained letter on the fast path;
     - non-identity labels;
     - a wrong `inverse_of` range;
     - an `inverse_of` that names a later part.
  4. A lattice step that ends with identity labels and a digest other than SOLVED (the w9 case) passes with `labels_identity` true (R3-05).
  3. The fast model agrees with `State.apply` on three prefixes.

## 4. Integrator part I1, then the development runs

**I1.**
1. Review each `changes.patch`, apply it, run `--cleanup`.
2. Switch the v3 lattice phase to the plugin. After this, v3 records with a played lattice get the replayed `end_digest`.
3. Add end-to-end tests:
   - dispatcher at R22D-36 (R3-04): `r22d75` plays (12, 14) and (35, 13) and ends HEIGHT2 at `3ab25d21…`; `r22d81`, `h4` and high are `not_applicable` (n4 = n6 = 0), the tail follows; the checker recomputes every entry test and passes, trees included;
   - the kernel end to end (R3-04): a height stub that ends `stuck` at once at R22D-36 (n6 = 1, max 6), so `r22d81` is stuck too, `h4` is `not_applicable` and the real kernel runs, with D19, `--high-steps 1`; the record has a `high` row with contract `kernel_landing` or `kernel_lowering` (P2 acceptance 1 expects F1 with a first letter at or before (12, 45)); the checker's report shows P4's kernel contract applied to that row (a count of kernel contract checks of at least 1), and a mutated `high_exact` fails; `config.component = 'height_stub'`, verdict `component`;
   - the w9 fixture through the runner's lattice phase and the v3 checker (R3-05): result `solved`, `labels_identity` true, `end_digest` equal to the played, non-SOLVED digest; a mutation that leaves non-identity labels fails;
   - the synthetic lattice through the runner → the checker passes;
   - b20 in acceptance mode (R3B-02). Its whole closure is 40,500 letters (`test_runner.py:1075-1077`), below the default limit, so under A2 it is played, not `certificate_needed`:
     - with `--literal-limit 40499` the attempt is `certificate_needed` (over limit), and nothing is played;
     - with the default limit, under `RUNNER_SLOW=1` only (40,500 twists at 35-60 ms each, about 24-40 minutes), the step is played through `apply_word` with `standing: rule` and `construction: certificate`, and ends at `aab10f250f0aedbbcc0fbf0eb655cb0e4434a839c22fc7957559a27437f46e7c` with off 447, the end that `test_05` (`test_runner.py:1059-1081`) reaches through `slp_apply`;
   - a development fallback step → `coverage_failure`.
4. Run the four existing suites, including the full `test_literal_lattice.py`, and the five new ones.
5. Then:
   - one routine fast review of the merged candidate;
   - update README §2, §3, §4, §5 and §6;
   - open the PR.

**D0 regression.**
- `--scramble 10 7001`: 9 bulk twists, digest `e3b0c442…`.
- `--scramble 20 7106`:
  - 19 `seam` twists, to `1b263d3e…`;
  - 8,701,310 retained twists and 0 corrections, with poses now played twist by twist (about 6 minutes, estimated);
  - the final labels must be the identity;
  - the final digest is recomputed by the run and by the checker.
    - A scratch measurement of the deferred play gave `a71580bf…`, not the solved digest. It is not recorded in the repository.
    - I1 records the measured value in README §5 before anything relies on it.
  - solved under the 15 October definition;
  - the checker takes about 7 minutes (estimated).
- Repeat with schedule11. This is its first run on a real table.

**D1 seed 7101, depth 40** (acceptance mode, then development mode).
- **Clocks:**
  - Acceptance mode has no clock (§0).
  - The development run passes `--bulk-step-seconds 21600`, as in D2.
  - In the census, seed 7101's run was killed at 110 minutes, 49 minutes into a search (`11-scramble-census/README.md:61`).
- **Bulk (m2):**
  - `11-scramble-census/runs/round2/ia-k40-s7101.json` holds 97 steps (rounds 1 and 2, 223 twists).
  - N is 1,736 after index 95 and 1,734 after index 96, and 1,193 pieces are off.
  - Its `result` is `running`. The record does not say why it stopped: 3,636 s of a 4,500 s budget, last step at 3,610 s. The census README records the run as killed.
  - Compare the runner's first 97 steps with it: rule, twists, N, off, and the 223 twists. A match shows agreement with one earlier run over 97 steps, nothing more.
  - A difference is a lead for R22D-90 (cover's float tie-break), not yet a finding.
- **Height and high:** expected `not_applicable`, since the census states are at height 2.
- **Acceptance end:** not predicted. It could be `stuck` (with `certificate_needed`, or with no rule for b06's preparation), `coverage_failure`, or `budget` if an attempt timed out. Certificate-built closures are played twist by twist in this mode (A2): E2's 13 literal steps from 737 to 0 total 2,012,155 twists, about 20-33 hours in the runner and again in the checker.
- **Development:**
  - certificate steps use the gate's end table; the verdict is `stuck`;
  - the checker still replays them twist by twist. E2's 13 literal steps from 737 to 0 total 2,012,155 twists (`12-low-height/README.md:936`), about 20-33 hours at 35-60 ms per twist. Run that check in the background.
- If a run reaches the lattice, use schedule11. The late restorer needs 166.8 M twists at E2's literal end.

**D2 the 3000-twist stops (D5).**
- **Start tables:**
  - built from the census records with `12-low-height/scripts/export_table.py`, about 1,650 s each;
  - check the digest;
  - the records are `11-scramble-census/runs/heights/ia-k3000-s7302.json` and `12-low-height/runs/round22/cloud/census-k3000-s730{4,5,6,7}/`;
  - all five stops ended on the clock (`time-limit`).
- **Runner runs:** `--table <stop> --mode development`, bulk on, `--bulk-seconds 21600 --bulk-step-seconds 21600`.
  - If the bulk gets stuck, this tests the real hand-over.
  - If the clock ends the bulk, the run is `component`.
  - An unlimited bulk waits on the separate work on bulk speed at depth.
- **Component runs,** labelled as such (verdict `component`, with the step counts kept): `--bulk-steps 0`, compared with the recorded routes.

| Seed | Start | Expected first steps |
|---|---|---|
| 7302 | `015d839e`, 100,584 off, {2: 99,314, 4: 1,085, 6: 185} | `r22d75` 7 letters → `1ff0061f`; `r22d81` (124,2) U, (274,13) H6, (46,1) U, (46,3) U, (104,14) H6 → HANDOFF `bbef1c8e` |
| 7304 | `7e2304b2`, 95,569 off, n6 = 0 | `r22d75` 6 letters → `ac127dc3` (797 at height 4), stuck |
| 7306 | `9ebdc490`, 108,823 off, n6 = 29 | `r22d75` 9 letters → `16c4aefe` (n4 = 2,591), stuck |
| 7307 | `ba5371ae`, 96,236 off, n6 = 10 | `r22d75` 6 letters → `895b45de` (n4 = 1,585), stuck |
| 7305 | `273eab69`, 105,437 off, n6 = 0 | `r22d75` 11 letters → `cc5ef7ec` (n4 = 1,419), stuck (cloud line census-k3000-s7305, integrated with this plan's I0 or before) |

- **After that:**
  - `h4` follows. It is new at every seed (`r22d81` is not applicable at 7304-7307).
  - At 7302 the high phase follows, with about 700-900 height-4 pieces and an unknown cost. It ends `stuck` (the run goes on to the tail), `budget` or `timeout`.
- **7303** has no committed stop: run `--scramble 3000 7303 --bulk-seconds 18900`. Seed 7305's stop comes from its cloud line's record (`12-low-height/runs/round22/cloud/census-k3000-s7305/ia-k3000-s7305.json`).
- **Reproduction:** `--scramble 3000 7302 --bulk-steps 109` (about 2.3 hours).
  - Compare with the record's 109 steps, 313 twists, N 130,386 and 100,584 off.
  - Compare the end with `015d839e` from stage 12's rebuild; the record itself has no digest.
  - Claim nothing beyond these.

**Every run.**
- Run `check_run.py`.
- Record the `result`, `acceptance.verdict`, every step's length, tree and time, and the check output.
- Development budgets:
  - `--height-seconds` at least 12,000 (pair passes take 1,100-5,300 CPU s);
  - `--pair-shards 3`;
  - the kernel at 3,600 s plus a margin.
- Acceptance runs have no clock.

## 5. Checks, risks, owner questions

**Process (D8).**
- One senior plan check (fast tier) of this plan before I0.
- One routine fast review of I0 before its commit.
- The integrator reviews each Codex patch.
- One routine fast review of the merged candidate.
- The five implement calls run in parallel. Nothing is committed, pushed or fetched while one runs.

**Owner questions.** None.
- The phase order (D3) and what the checker may share (D4) follow the two readings the owner accepted on 20 October 2026 (decision page :70-81).
- The bulk order without reopen is the page's integrator default (:82-85).
- No acceptance batch waits on a question.
- Not planned:
  - a priority list checked again at every state, which would need a cycle stop;
  - a simulator written only for the checker.

**Risks.**
- The checker's height snapshot at about 100k off pieces, for every step and every height letter, has an unmeasured cost.
- The fast lattice model must be right without help from the runner. The simulator digests at every step end and the label cross-check anchor it.
- Acceptance runs have no clock, so a run can take very long. In the census, seed 7101's search was still running after 49 minutes. Every run's time is recorded.
- Acceptance mode plays certificate-built closures twist by twist (A2), at 35-60 ms per twist; a step near `--literal-limit` (10⁷) would take days. `test_05` and `test_06` stay in development mode.
- `jordan_target.py`'s fixed CPU cap of 840 s is the one clock acceptance mode cannot lift; its cuts are counted and make the verdict `budget`.
- In development runs, a timed-out attempt makes the verdict `budget`, not `stuck` (R22D-55).
- Playing poses twist by twist makes the lattice play about 3 times slower (estimate: 11.6 + 26.7 µs per twist), about 1.8 hours for 166.8 M twists.
- A lattice step's tree can be large. Folding into spans above 10⁶ parts per step bounds it.
- There is no resume: the output directory must be empty (`runner.py:1062-1063`).
- The fixture times cited in §3 are private measurements until I0 records them in README §5.
- **Not run:** none of this has been run. Windows and DirectX checks do not apply.

## Changes from the draft

| Item | Where it is handled |
|---|---|
| M1 UNLOCKED is not a handoff | P1 ends; §0 result table |
| M2 cost of playing certificate steps | §0 acceptance labelling (development never newly plays them; acceptance plays them since A2, revision 3b); D1 checker cost corrected |
| M3 composed lattice poses | P3 `observer` keyword |
| M4 sub-phases depend on the path | §0 phase order; I0 intermediate-state test |
| M5 hand-over and phase results | §0 result table; D2 runs with the bulk on; `component` label |
| M6 checker cannot see the state before a step | I0 `prepare`, `on_letter`, `finish` |
| M7 no early stop in `execute` | I0 `stop_when` |
| m1 field contracts | §1 height table |
| m2 D1 bulk count off by one | D1 bulk paragraph |
| m3 stub breaks lattice tests; missing options | I0 keeps `verify_late` until P3; options added in I0 |
| m4 table not exported | I0 `record_step` |
| m5 fallback label lost | §0 probe; `coverage_failure` |
| D1-D5 | §0; D5 also in the D2 runs |
| D6 | P3 |
| D7 | the M and m rows above |
| D8 | §5 process |

## Changes in revision 3

| Finding | Change |
|---|---|
| R2-01, F2 step tree | §1 `tree` field and checks; I0 writes trees and checks their structure and `inverse_of` on `json` words; P1, P2 and P3 build trees; P5 checks `inverse_of` on `ia16` words |
| R2-02, F1, F3 owner readings and sharing list | Q1 and Q2 removed and replaced by citations; no batch gate; sharing list in the owner's words; the AST scan allows `ctx`, `to_tuple` and `transpose` by name with the reason; alternatives kept as one "not planned" line |
| R2-03 timeouts and clocks | §0 Clocks: unlimited in acceptance mode; `timed_out` on attempts; tail `timeout` row; verdict rule 5; D1 clocks |
| R2-04 verdict order | `component` moved right after `error`; counts kept |
| R2-05 h4 entry test | `h4`: max ≤ 4 and n4 > 0, recomputed by the checker (the potential part is listed below) |
| R2-06, F12 unmapped ends and truncated output | P1 ends, parsing, state tests and stub tests |
| R2-07, F5 kernel `lattice` and budget kinds | P2 result mapping and tests |
| R2-08 P3 acceptance length | P3 acceptance file `test_lattice_phase.py`; full `test_literal_lattice.py` in I1; packet time 20 minutes |
| R2-09 v3 composition path and README lists | I0 end digest; README sections in I0 and I1 |
| F4 kernel row fields | §1 High; P2 proposals; P4 |
| F6 kernel contract | P4 states the kernel's own checks and the letter bound; §0 high potential (off, n6) |
| F7 digest `a71580bf…` | D0 marks it as an unrecorded scratch measurement; the check recomputes it |
| F8 seed 7101 stop | D1 bulk paragraph |
| F9 README §1 sub-phases | §0 phase order; I0 README list |
| F10 end digest claim | I0 check_run.py end digest |
| F11 private timings | §3 packet rule; P1, P2, P4 labels; §5 risks |

## Changes in revision 3b (senior plan check, 20 October 2026, findings R3-01 to R3-05, all adopted)

| Finding | Change |
|---|---|
| R3-01 unlimited clocks cannot reach the children | §0 Clocks: runner clocks off; one adapter with the sentinel 10⁹ s for every child clock argument; inventory with the integrator's probe; the capture-timeout limit; `tclose2` and `jordan_target` timeouts read into `timed_out`; mocked-clock tests in I0, P1 and P2 |
| R3-02 `conj` reference pointed forward | §1 `inverse_of` points backward; P3 puts it on the trailing u^(us); four-case `conj` test against `Program.expand`; a forward reference fails in I0 and P5 |
| R3-03 `unsupported` read as stuck | §0 result table and P2: `unsupported` → `error` (`d19_missing`, `invariant_height_above_6`); stub test with a tail that would succeed |
| R3-04 R22D-36 reaches HEIGHT2 | I1: R22D-36 as the dispatcher test (high `not_applicable`); a separate kernel end-to-end record with a height stub and `--high-steps 1`; P1's `h4` call moved to `7cd868f6…`, measured |
| R3-05 invisible rotation | I0 end digest: v3 solved by identity labels with the replayed digest; w9 in P3, P5 and I1 |
| A2 (integrator amendment) certificate-built closures | §0 acceptance labelling: in acceptance mode a closure whose V comes from a certificate and expands to a literal word is played twist by twist through `record_step` (`standing: rule`, `construction: certificate`); `certificate_needed` only where no literal word comes out. Reason: such a step is known by its literal word, as stage 12 records E2's steps ("played to the lattice in literal words"); revision 3 counted it as stuck. I0 stub test |

## Changes in revision 3c (senior scoped re-check, 20 October 2026, findings R3B-01 and R3B-02, both adopted)

| Finding | Change |
|---|---|
| R3B-01 certificate deadlines without a `timeout` row | §0 Clocks: an incomplete `r22b10_cert` result is timed out when its log has a `timeout: true` row or any missing reason contains a deadline phrase (`wall-clock budget exhausted`, `serialization budget exhausted`, `exceeded budget`); one such reason marks the whole attempt; the builder's fixed size caps are not clocks and stay `certificate_needed`. §0 acceptance labelling: `certificate_needed` only for a non-timed-out incomplete certificate. §0 test 6 (per-orbit, serialization, mixed, non-deadline, size cap, explicit row) |
| R3B-02 A2 against the retained rejection tests | I0: the default `CertificateStub` is the positive A2 case; over-limit (`--literal-limit 4`), incomplete (non-deadline reason) and failed-check variants give `certificate_needed`; the old "acceptance on `CertificateStub` gives `certificate_needed`" line is removed; development keeps `test_complete_certificate_formats_dispatch_and_advance`. I1: b20 in acceptance gives `certificate_needed` only with `--literal-limit 40499`; at the default limit it is played under `RUNNER_SLOW=1` and must end at test_05's end digest |

## Changes in revision 3d (senior scoped verification, 20 October 2026, finding R3C-01, adopted)

| Finding | Change |
|---|---|
| R3C-01 a serialization expiry can leave only non-deadline reasons and no `timeout` row | §0 Clocks: a third test (c), the result's `seconds` (or the runner's measured child wall time) at least `s − min(10, s/10)`, which every serialization expiry meets and the normal path cannot; §0 test 6: two emitter-derived fixtures of `r22b10_cert.main` with a fake clock, one with the serialization deadline (timed out, `budget`) and one without (`certificate_needed`, `stuck`) |

## Defects not applied

- **R2-05, first part (potential (off, n6, n4) for the high phase): not applied, because its premise is wrong.**
  - D19 does not lower a piece from height 4 to 2. Its word is gated by `strict_step` (`r22d25_search.py:375`), which accepts only a whole landing and gives kind `landing` (`r22d25_state.py:326-329`).
  - The kernel's only lowering rows come from `protected_lowering` at height 6 (`r22d25_search.py:215, 261, 296`).
  - The lowering to height 2 at :437 is only the middle of an ST step, which is gated as a whole by `strict_step` (:460).
  - The kernel also stops any lowering that does not lower n6 (:588-589).
  - The gap R2-05 found is real, but it was in the plan's loose checker contract ("height_from ≥ 4"). F6 closes it by stating the kernel's own contract. With that contract, (off, n6) lexicographically falls at every high step, and the plan keeps it.
- **R2-03, citations:** its content was applied, but its line numbers were wrong.
  - The clock defaults are at `runner.py:1015` and `:1019`, not 1029-1031.
  - README §1 rule 4 is at `README.md:67-70`, not 79-82.
- **F8, "the cause of the stop is not recorded":** this holds for the record, which the plan now says. The stage 11 README does record the run as killed at 110 minutes, 49 minutes into a search (`11-scramble-census/README.md:61`), and the plan cites it.
- **F3, choice of fix:** the plan allows `to_tuple` and `transpose` by name, with a reason, instead of rewriting `check_run.py`. It keeps `ctx`: it is `rim.ctx = sim.get_context()` (`rim.py:29`), the simulator's own context, which the owner's list covers as `sim/`. `transpose` comes from `exact.py`, which `sim/` imports itself (`sim/kplus.py`, `sim/state.py` and others). So the allowlist stays within the owner's list. The scan names the three re-exports explicitly.
- **F1 against R2-02 on the alternatives:** F1 asks to remove them, and R2-02 asks to keep them as notes. The "if the owner rejects" branches are removed, and one "not planned" line in §5 keeps them as notes.

# Revision 3e (addendum to 3d)

Revision 3d (senior plan check and two scoped verifications, 20 October 2026) is approved and unchanged except for the
items below. This addendum lists only the changes. Context: on 22 October 2026 the owner restarted the work for the
tutorial and its acceptance test, to the standard of a finished product
(`docs/wiki/decisions/owner-decisions-2026-10-22-tutorial-first.md`). Packets P1 to P5 were implemented and reviewed
on 20 October (`research/jumbling/restoration/13-tutorial-runner/packets/REVIEW.md`: 22 confirmed findings, one
`major`, P3-falsepass-01, open). I1 of revision 3d section 4 is what remains.

## E1. Review findings fixed in I1, not deferred

The `major` finding and every `minor` finding that can change a verdict, hide a failure or leave files behind are
fixed in I1. One Codex implement packet (FIX, disjoint from the runner and the checker) takes the fixes inside the
plugin and checker modules; the integrator takes the runner and `check_run.py` sides.

| Finding | Fix | Where |
|---|---|---|
| P3-falsepass-01 (major), P3-contract-01 | The identity checks (exactly one run, `input`, `input_digest`, `library.sha256`, and for schedule11 `realign_sha256`) run before the status branch; only a failure certificate for this input and library maps to `stuck/restorer_no_solution`; a mismatch is `error/restorer_output_invalid` with its check. Test G gets a certificate with a wrong input digest and one with a wrong library digest. | FIX: `lattice_phase.py`, `test_lattice_phase.py` |
| P1-contract-01, P1-falsepass-02 | A parse status `malformed`, `mismatch`, `missing_step` (and `crash`) is decided before `child.timeout`; a killed shard keeps its parse status; any shard with parse status `malformed` or `crash` gives `End('error', ...)`. Stub tests for each. | FIX: `rules_height.py`, tests |
| P1-falsepass-01, P1-robust-03 | All shards DONE and the pair undecided is `error/inconsistent_output`; `budget` only if some shard attempt is `budget`, `timeout` only if some shard was killed by the clock; a DONE shard's FIRST indices must be exactly {k < first_letters : k mod n = shard}. | FIX |
| P1-falsepass-03 | End lines need a START line (except the BUDGET lines the packet allows); with steps 0, the end line's tau and u equal START's, and for pairs tau equals `--target`. | FIX |
| P1-contract-03 | Test 1 also drops a class-B FIRST line before (541, 2) and asserts `undecided`. | FIX |
| P1-robust-01 | `_shards` wraps its futures in try/finally with a plugin-local event; each shard's `stop_when` returns true once the event is set; the event is set on any exception and as soon as one shard returns `error`. | FIX |
| P2-robust-01, P2-robust-02 | The kernel's `--cache` is a directory inside the call's work directory, seeded with a hard link or copy of `home_centroids.npy` from `ctx.cache` when it exists; the test module removes its temporary tree in a finally block unless `KEEP_TEMP=1`. | FIX: `rules_kernel.py`, tests |
| P4-contract-01, P4-falsepass-01 | Test 9 gets the h4/L4 row whose end state exceeds the maximum (`height_max`); test 6 gets forward clean-add rows labelled r22d81/H2 and h4/L2 at tau 1 whose u rises (`height_u_progress`). | FIX: `test_check_contracts.py` |
| P5-falsepass-01, P5-robust-01 | The replay pass hashes and counts the bytes it plays (`ia16_chunks(path, sha=...)`) and compares them with the row's `word_sha256` and `letters`; the range pass does the same. | FIX: `check_lattice.py`, tests |
| P5-contract-01 | `to_state` returns the rebuilt State with its verdict; `flush` writes the rebuilt State, records `model_labels` and runs the step-5 end checks. Test through the model path. | FIX |
| P5-contract-02 | Test D times the model loop separately and prints its rate next to the whole-step rate. | FIX |
| P1-robust-04, P3-robust-01 | `check_signal()` right after every `next_step` and `play` returns, before the result is stored; the attempts of a call cut off by a signal are kept on the sub-phase. | runner |
| P1-robust-02, P2-robust-04 | `main()` sets `RLIMIT_CORE` to (0, 0) for the runner process before any child starts, so no child writes a core file into the repository. | runner |
| P4-falsepass-02 | `check_v3_phases` calls `check_contracts.check_high_phase` with the checker's own snapshot at the high phase's start whenever the phase has steps (error runs included), and adds its failures to the record. A check_run-level test with a high phase one letter over the bound fails. | checker |
| K1 | For played lattice rows, `phases.lattice.result` must be `solved`, `steps` and `twists` must equal the rows', and the last row's replayed labels must be the identity. | checker |
| K3.1 | The path scan leaves integer values alone: an integer `params.unit` in a d19 high row is not a path. | checker |

Deferred, unchanged: P1-contract-02 (the stricter order is conservative and cannot pass a false record), the other
nits. They are listed in the README with their dispositions.

## E2. The runner's lattice phase after P3 (replaces the stub path for the real plugin)

- `lattice()` calls `plugin.play(ctx)`; `check_signal()` follows. A `LatticeResult` with steps is recorded as in
  revision 3d. Its `report` is written to `lattice/report.json` and the record keeps `phases.lattice.report =
  {'file': 'lattice/report.json', 'sha256': ...}`.
- A `LatticeResult` with no steps is accepted only when the plugin's report says the restorer scheduled nothing
  (`played.total == 0`) and the end table's digest equals the snapshot's. The runner records `phases.lattice =
  {result: 'solved', steps: 0, twists: 0}`, and the run result is `solved` only if the identity-labels test of the
  simulator holds at the current state; otherwise `error`. The checker recomputes the labels test from its own state.
- An `End` from the real plugin is recorded: `phases.lattice.result = end.result` with its `reason`, `details` and
  `attempts`; the run result is `stuck` for `stuck`, `budget` for `budget` and `timeout`, `error` for `error`. The
  checker accepts these results only with no lattice rows, checks the run result against them, and counts the
  attempts in the verdict as for the height and high phases.
- The composition path (`verify_late.py`) stays only for a stub plugin (tests); a record from it keeps
  `config.component = 'lattice_composition'` and cannot pass acceptance.
- `config.kernel.d19` records `{'file': <repository-relative path>, 'sha256': ...}` of the D19 rule the kernel plugin
  passes.

## E3. A frozen acceptance configuration

- New committed file `research/jumbling/restoration/13-tutorial-runner/acceptance-config.json`: the options an
  acceptance run must use (`literal_limit`, `seeds`, `tail_rules`, `lattice_restorer`, `fallback_max_length`, `par`,
  `pair_shards`, `bulk_steps` null, `tail_steps` null, `high_steps` null, `no_lattice` false).
- `--mode acceptance` reads it, refuses to start when an option differs, and records the file's sha256 and contents
  in `config.acceptance_config`. The checker compares the record's options with the recorded contents and with the
  committed file of the source identity it is given. A development run records null there.
- The lattice restorer in the file is chosen after D0 (E5) and is then fixed for every batch.

## E4. Step trees for the bulk rule conj

The bulk child records a conj step (`seams.SeamRestorer.conj` returns `[(c, i), (d, j), (c, k)]`) as three named
parts `x`, `y`, `z` of one letter each, through `api.flat`. The other bulk rules stay one span (each is one letter or
a fixed short word with no named parts in the theory). The checker already checks that a tree expands to the word.

## E5. Order of the development runs

D0 runs first with the late restorer and then with schedule11 (revision 3d). The integrator fixes the acceptance
restorer from D0's measured twist counts and times, and records the choice and the numbers in the README before
batch 1. The bound to cite for the late restorer on the committed cascade is 1,324,660,182 retained twists plus
25,852 for the corrections; for schedule11, 20,866,844 plus 25,852 (METHOD.md, "Lattice-phase length").

## E6. Small items

- `runner.py:45`: the comment says the owner left reopen out; it is the integrator's default under the owner's
  reading of 20 October 2026. Reword.
- The README's sections 2 to 6 and 8 and its file table are updated with I1.

## Checks

- The five packet test files, `test_runner.py`, `test_check_run.py`, `test_literal_lattice.py`,
  `test_check_r22d25.py`, all on headless Linux; the end-to-end tests of revision 3d section 4 plus the E2 and E3
  cases; then one routine fast review of the merged candidate.
- Not run here: Windows, DirectX and performance checks (not applicable).

## Out of scope

The batches themselves, the tutorial text and its illustrations (separate work, after D0 to D2), research on new
height rules, and any change to the mathematical contract or to critical paths.
