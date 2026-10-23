# Integrator review of packets P1 to P5 (20 October 2026)

The integrator's review of the five Codex implementations, made before integration. Each patch was read in three
lenses (contract, false pass, robustness and independence); every finding above `nit` was then given to two
independent refuters, and one agent applied all five patches to a scratch copy of the tree at `97815b1` and checked
how they fit together. Evidence kind: source and fixture only, headless Linux. Paths are relative to the repository
root; `<scratch>` marks an experiment script outside the repository, not kept.

Result: 22 findings confirmed (1 `major`, 21 `minor`), 2 refuted, 23 `nit`.
Disposition: the `major` finding P3-falsepass-01 is adopted and open (fix below; not applied, since the patches are
not integrated); the `minor` findings and the nits are deferred to the integration (I1).

## Confirmed

### P1-contract-01 (minor, P1)

- Where: research/jumbling/restoration/13-tutorial-runner/scripts/rules_height.py:410-413 and :491-492 (worktree 20261020T201237Z-97d0c960)
- Claim: When the runner's wall clock kills a child, any parse failure in its output (malformed, mismatch or missing_step) is reported as `timeout` instead of `error`. In the shard path, `_shards` also overwrites a shard's parsed status `malformed` with its attempt outcome before calling `_pair_choice`. As a result the live adapter can propose an `h4/pair` step from a shard whose output `choose_pair` itself rejects as an error. Packet 4.1 (lines 175-177) says malformed output is never turned into stuck, budget or timeout. Packet 6.5, choose_pair step 2 (lines 810-811), says an output that fails these checks is malformed or inconsistent, which is an error.
- Counterexample: Only a complete line that is not valid JSON, followed by a wall-clock kill, triggers this. The real scripts print every line with json.dumps, so this is unlikely in practice. The masked parse stops at the bad line, so it can make the result undecided but cannot change which pair is chosen.
- Fix: In `_call`, check `status in ('malformed', 'mismatch', 'missing_step')` (and `crash`, if the packet's intent is that only the runner's kill can be mapped to timeout) before `child.timeout`. In `_shards`, keep the parsed status and pass it to `_pair_choice`, or return `End('error', ...)` when a killed shard's parse status is `malformed`. Add a stub test for each case.

### P1-contract-02 (minor, P1)

- Where: rules_height.py:138-140, :204-206, :421; test_rules_height.py:565-569
- Claim: Packet 6.3 (lines 668-689) gives the order for ends without a deciding line: timeout, then a known end line with rc == 0 (mapped), then SIGXCPU, then 'a Traceback, or rc != 0 -> crash'. The code lets any `Traceback` line, even one after a valid end line, override that end line, so an end line + Traceback with rc 0 maps to `error`/`crash` instead of the end line's result. The test asserts the code's order, which is the reverse of the packet's.
- Counterexample: Stub: START, then `STUCK {"steps": 0, ...}`, then `Traceback after end`, exit 0. Packet: `stuck`/`no_step`. Code: `error`/`crash`.
- Fix: Either follow the packet's order (a known end line with rc 0 is mapped before a later Traceback is considered) and change the test, or ask the integrator to record the stricter rule as a packet amendment. The current behaviour is conservative and cannot produce a false pass.

### P1-contract-03 (minor, P1)

- Where: test_rules_height.py:308-315 (CommittedTests.test_05) and :658-693 (test_13); code at rules_height.py:330-337
- Claim: Packet 6.5 rule 2 (lines 816-819) says the choice is decided only when every index whose letter precedes it, in both classes, is complete. No test covers a missing class-B index before a rule-2 choice. The negative cases in test 1 drop only class-A letters ((357,1) in round 1, (407,1) in round 2), so `undecided` there comes from the 'all class A complete' guard at :330 and never reaches the `needed` check at :336. A mutant that drops the rule-2 `needed` check would pass every test.
- Counterexample: Delete line 336's check, that is, decide rule 2 as soon as all class-A indices are complete. The suite still passes, but round 1 with (2,1) missing would wrongly decide (541,2)/(593,54).
- Fix: In test 1, also drop the FIRST line of a class-B letter before (541, 2) in round 1, for example (2, 1), and assert `undecided`.

### P1-falsepass-01 (minor, P1)

- Where: research/jumbling/restoration/13-tutorial-runner/scripts/rules_height.py:498-503 (also :415-420)
- Claim: If every two_letter_u shard ends DONE but the pair is still undecided, the plugin returns End('budget', 'cpu_budget'). No attempt is timed out and no clock ended anything. The output is inconsistent, so this should be an error.
- Counterexample: Experiment E1 in <scratch> two stub shards. Each prints CONTEXT, its FIRST lines with index 5 left out, and a DONE whose first_letters_done equals its printed count. No line has a success. Output: `End budget cpu_budget {}` with attempts [('h4','stuck',False), ('two_letter_u',0,'done',False), ('two_letter_u',1,'done',False)]. The run would end `budget`, not `error`. The real script only skips indices with --from-index, so this needs faulty output.
- Fix: For a DONE shard with CONTEXT, require its set of FIRST indices to equal {k < first_letters : k % nshards == shard}; otherwise return 'inconsistent_output'. In `_shards`, also return End('error', 'inconsistent_output') when every call's outcome is 'done' and the choice is neither 'step' nor 'exhausted'.

### P1-falsepass-02 (minor, P1)

- Where: research/jumbling/restoration/13-tutorial-runner/scripts/rules_height.py:410-413 and :491 (defeats :280-282)
- Claim: A wall-clock kill hides malformed or mismatched child output. A single-call script then ends `timeout` instead of `error`. For the pair shards, a malformed shard that was later killed counts as a plain timed-out shard, and the pair step is still proposed.
- Counterexample: exp1.py E2: shard 1 prints CONTEXT, then `FIRST {broken json`, then sleeps; `--height-seconds 2`. Shard 0 holds the decided freeing success at index 0. Result: `Proposal h4/pair [[1,1],[3,1]]`, with shard 1's attempt ('timeout','wall_clock',True). Control E2b, the same output without the kill: `End error malformed_output`. E3: r22d81 prints `START {broken json`, then hangs, and is killed: `End timeout wall_clock`. Control E3b without the kill: `End error malformed_output`.
- Fix: In `_call`, test `status in ('malformed', 'mismatch', 'missing_step')` and the 'crash' status before `child.timeout`. In `_shards`, do not overwrite a 'malformed' or 'crash' parse status with the attempt outcome, or return End('error', ...) for any call whose parse status is 'malformed'.

### P1-falsepass-03 (minor, P1)

- Where: research/jumbling/restoration/13-tutorial-runner/scripts/rules_height.py:401-406, :421-446, :530-539
- Claim: End lines (STUCK, UNLOCKED, HANDOFF, HEIGHT2, and the BUDGET exceptions) are accepted without a START line, and an end line's tau and u are never compared with START or with --target. So the 6.2 check (pairs START tau = t, u = r22d81 STUCK u) can be skipped, and inconsistent output becomes `stuck`. The checker does not recompute `stuck`.
- Counterexample: exp1.py E4b: the pairs stub, called with --target 311 after r22d81 STUCK with tau 311 and u 2, prints only `STUCK {steps:0, digest:<current>, tau:5, u:99}`, with no START. Result: `End stuck no_step`. exp3.py E7: r22d81 prints START with tau null and u 0, then STUCK with steps 0, tau 311, u 2. The plugin runs r22d81_pairs --target 311 and returns `stuck unlocked_without_step`. The consistent output (control E7b) gives `stuck no_target`. E6: an r22d81 BUDGET with steps 1 and played_unreported but no START gives `budget`. None of these comes from the real scripts, all of which print START before every end except BUDGET.
- Fix: For HEIGHT2, HANDOFF, STUCK and UNLOCKED, and for the two steps-1 BUDGET exceptions, require a START line; otherwise return 'truncated' or 'inconsistent_output'. When steps == 0, require the end line's tau and u to equal START's (r22d81, h4, pairs), and for pairs require the end line's tau to equal --target.

### P1-robust-01 (minor, P1)

- Where: research/jumbling/restoration/13-tutorial-runner/scripts/rules_height.py:480-485 (_shards), in worktree 20261020T201237Z-97d0c960 (sha256 a9cefa6c...)
- Claim: If SIGINT (Ctrl-C) or any other exception reaches the main thread while the two_letter_u shards run, the shard children are not stopped. The plugin waits until every shard ends by itself. The shards run in their own sessions, so a terminal Ctrl-C never reaches them, and a SIGKILL of the runner leaves them running as orphans. Separately, a shard that crashes at once does not stop its siblings: the call reports the error only after all of them finish, which in acceptance mode (no wall clock) can be hours.
- Counterexample: Experiment <scratch> with drive_sigint.sh, using the worktree's rules_height and runner, a real Runner.execute, acceptance clocks, two stub shards that print CONTEXT and sleep 40 s, and the default SIGINT handler. SIGINT was sent at t=4 s. next_step did not raise; both shard children were still alive 15.5 s later. A second SIGINT made KeyboardInterrupt surface at 19.3 s, but both children kept running. The process exited (status 130) only 36.3 s after the first SIGINT, when the stub shards finished on their own.
- Fix: Wrap the futures in try/except BaseException (or try/finally). On failure, set a plugin-local threading.Event, and give each shard `stop_when=lambda line: event.is_set()`. two_letter_u prints one FIRST line per first letter, so this predicate is checked every few seconds. Also set the event as soon as one shard returns `error`, so a crash stops its siblings. A complete fix needs an I0 hook so the plugin can cancel its own children, for example a cancel predicate in Ctx or a stop_when that execute calls on every poll; record that as an interface item for the integrator.

### P1-robust-02 (minor, P1)

- Where: rules_height.py:447-448 (SIGXCPU -> budget/cpu_rlimit) together with runner.py:489 (children run with cwd=ROOT)
- Claim: When a real rule child stops on its own RLIMIT_CPU soft limit, the plugin records an ordinary `budget`/`cpu_rlimit` end. On that same path the child is killed by SIGXCPU, whose default action dumps core into the child's working directory, which is the repository root. The core files are not git-ignored. A development run on a machine with a nonzero soft core limit and a plain `core` pattern would leave multi-gigabyte untracked `core` files in the checkout. Acceptance passed only because this container's soft core limit is 0. The packet handled this for the stub tests only (6.6).
- Counterexample: Experiment in <scratch> a parent raised RLIMIT_CORE to unlimited (as after `ulimit -c unlimited`) and started `python3 -I -B -u -c 'import os, signal, numpy; os.kill(os.getpid(), signal.SIGXCPU)'` with start_new_session=True. The child ended with rc -24, which the plugin maps to budget when seconds >= cpu, and left a 41.5 MB `core` file in its cwd. A rule script holding a full 600-cell state would leave a much larger one, in ROOT.
- Fix: Runner side (I0), because the packet fixes the argv form: have execute start children with RLIMIT_CORE (0, 0) via preexec_fn or a small `-c` launcher. Alternatively, add `core` and `core.*` to .gitignore and note in the README that height runs need `ulimit -c 0`. The plugin itself cannot fix this without changing the argv form the packet requires.

### P1-robust-03 (minor, P1)

- Where: rules_height.py:498-503 (_shards fallback mapping); DONE checks at :298-299 and :415-418
- Claim: When every shard ends DONE (rc 0, first_letters_done equal to its FIRST count) but the FIRST lines do not cover all indices, the result is undecided and the plugin returns `End('budget', 'cpu_budget')`. No attempt is timed_out and no clock fired. This shard output is inconsistent, because the real two_letter_u.py prints DONE only after visiting every index k with k mod n = shard (two_letter_u.py:180-218). It should end `error`/`inconsistent_output`, not a budget end that ends the run `budget`. The packet's budget row assumes "some shard BUDGET or SIGXCPU"; the code takes that row for every undecided case without a timeout.
- Counterexample: Seen in the first run of shard_sigint.py, where SIGINT was ignored so both shards ran to completion. Two stub shards each printed CONTEXT (first_letters 59, nshards 2) and then `DONE {"first_letters_done": 0}` with rc 0. next_step returned `End(result='budget', reason='cpu_budget', details={}, attempts=[h4 stuck; shard0 done timed_out False; shard1 done timed_out False])`.
- Fix: In _shards, return `budget` only if some shard attempt has outcome `budget` (BUDGET line or the SIGXCPU budget), and `timeout` only if some shard was killed by the clock. Otherwise return `End('error', 'inconsistent_output')`. Optionally, also require for each DONE shard that its FIRST indices are exactly {k < first_letters : k mod n = shard}. Add a stub test with all shards DONE and a missing index.

### P1-robust-04 (minor, P1)

- Where: rules_height.py:391-392 (`if child.cancelled: return finish('cancelled', 'cancelled', 'error')`) with runner.py:494, :656 and :1396
- Claim: A SIGTERM or SIGHUP to the runner while a height child runs is recorded as a failure of the height rules: sub-phase result `error` with reason `cancelled`, height result `error`, and run error 'r22d75 phase error: cancelled'. The signal number is lost. The runner's own phases record the same event as RunSignal ('signal 15') and leave the phase results alone. Inside a plugin call, `child.cancelled` can only come from a signal, because choose() clears `cancel` when no signal arrived (runner.py:1099-1100). Two encodings of an operator interrupt now exist, and one of them looks like a rule error. This follows packet decision 6 (cancelled -> error), so the change belongs to the runner.
- Counterexample: Experiment <scratch> Runner.run() at the R22D-36 table, with an r22d75 stub that prints START and sends SIGTERM to the runner. Record: result 'error', error 'r22d75 phase error: cancelled', phases.height.result 'error', sub.r22d75 {result 'error', reason 'cancelled', attempts [outcome 'cancelled', rc -9]}. No 'signal 15' appears anywhere.
- Fix: Runner side: in plugin_phase, call `self.check_signal()` right after `next_step` returns and before storing the End, so a signal ends the run as RunSignal, as it does for the runner's own rules. Keep the attempts, for example by storing them on the sub-phase before raising. No plugin change is needed, or the plugin could carry a `signal` hint in the attempt.

### P2-robust-01 (minor, P2)

- Where: research/jumbling/restoration/13-tutorial-runner/scripts/rules_kernel.py:162 (worktree 20261020T201240Z-67ca5cac)
- Claim: The plugin passes the runner's cache as the kernel's `--cache`. The kernel puts D19's scratch directory, which holds a full pose table, inside that cache. The runner stops a call by sending SIGKILL to the whole process group, on a cancel and in development mode on the wall clock. When that kill lands while D19 runs, `tempfile.TemporaryDirectory` never cleans up, so `r22d25-d19-*` stays in the cache. That cache is outside the run directory whenever `--cache` is given, as in every test here and in shared-cache census runs.
- Counterexample: `runner.py --mode acceptance --cache <shared dir> ...` reaches the high phase at a height-4 state, so the kernel calls D19 first. The operator sends SIGTERM to the runner. The run ends `error/cancelled`, but `<shared dir>/r22d25-d19-XXXX/table.json` (a pose table of the run's state) stays outside the run directory, and each such cancel adds another one. In development mode the same happens when the wall clock fires during D19.
- Fix: Keep all kernel scratch under the call's own directory. Pass `--cache directory / 'cache'`, after copying or hard-linking `home_centroids.npy` from `ctx.cache` when it exists (5.7 MB), so the cache is not recomputed. Alternatively record this as a known kernel limitation, and have I1 note that runs which can be cancelled should use the default in-run cache.

### P2-robust-02 (minor, P2)

- Where: research/jumbling/restoration/13-tutorial-runner/scripts/test_rules_kernel.py:15, :878-895
- Claim: The test suite creates one module-level temporary tree and never removes it. Each acceptance run leaves 8.6 to 14 MB in the system temp directory: about 100 run directories, the R22D-36 and END36 tables, and the cache. The model `test_runner.py` cleans up its temporary directories.
- Counterexample: Repeated acceptance or CI runs pile up roughly 14 MB per run in /tmp, and nothing ever removes them.
- Fix: In `main()`, remove TEMP in a `try/finally` after the after-inventory, using `shutil.rmtree(TEMP, ignore_errors=True)`. Keep it only when an environment variable such as `KEEP_TEMP=1` asks for it, for debugging.

### P3-contract-01 (minor, P3)

- Where: research/jumbling/restoration/13-tutorial-runner/scripts/lattice_phase.py:527-536 (worktree 20261020T201243Z-4185e2ec)
- Claim: A `failure certificate` status is turned into `End('stuck', 'restorer_no_solution')` before any of the step-5 identity checks run (input, input_digest, library_sha256, orbit partition). So a certificate for another input, or one made with a different library, is reported as `stuck` and not as `error`. Packet section 4.5 step 5 says these checks run on the restorer's output first. Its I0-01 rule (packet :431-436, :656-658) says only "no rule gives a step" is `stuck` and every digest mismatch is `error`.
- Counterexample: A restorer bug, or a stale `restore/`, leaves a certificate for a different table or library. play(ctx) returns stuck/restorer_no_solution, which I1 would record as a failed attempt, when it should be error/restorer_output_invalid with check input_digest, input or library_sha256.
- Fix: Move the identity checks (exactly one run, input, input_digest, library.sha256) ahead of the status branch, so that only a certificate for this input and library maps to `stuck`. Add a test G case: a certificate status with a wrong input_digest gives restorer_output_invalid/input_digest.

### P3-falsepass-01 (major, P3)

- Where: research/jumbling/restoration/13-tutorial-runner/scripts/lattice_phase.py:527-536 (worktree 20261020T201243Z-4185e2ec); test gap at test_lattice_phase.py:775-781
- Claim: The plugin decides that a restorer run is a failure certificate before it checks that the run belongs to this phase. It returns End('stuck', 'restorer_no_solution') with a failed attempt even when the run's input, input_digest or library sha256 differ from what the plugin gave the restorer. These are integrity failures, and I0-01 and the packet's result table (section 4.5) make them errors: only 'no rule gives a step' is stuck, and every digest mismatch is an error. So a mismatched run is reported as a no-solution certificate, a weaker outcome than error.
- Counterexample: Experiment <scratch>, using the worktree's lattice_phase with a fake execute. The restorer writes runs=[{input: 'table:OTHER.json', input_digest: 'b'*64, status: 'failure certificate: nu outside the span of the correction library'}] and library.sha256 '0'*64, while the snapshot digest is 'a'*64. Output: `End(result='stuck', reason='restorer_no_solution', details={'status': ...}, attempts=[{'rule': 'lattice/late', 'outcome': 'stuck', ..., 'timed_out': False, ...}])`. The same mismatches with status 'restored' give `End(result='error', reason='restorer_output_invalid', details={'check': 'input'})`.
- Fix: Run the input, input_digest, library_sha256 (and, for schedule11, realign_sha256) checks before the status branch, and map a failure to End('error', 'restorer_output_invalid', {'check': ...}). Only a run that matches this table and library may become stuck. Add a test G case: a failure-certificate stub with a wrong input_digest or library sha must return restorer_output_invalid.

### P3-robust-01 (minor, P3)

- Where: research/jumbling/restoration/13-tutorial-runner/scripts/lattice_phase.py:470 (with runner.py:1265, :1275, :1396 at 97815b1, unchanged by P3)
- Claim: Already tracked as integrator decision 6. If SIGTERM or SIGHUP hits during the lattice phase, the run record loses the signal. P3 itself switches runner.lattice() to the real plugin (change-log conflict 2), so applying P3 without I1 makes a signal during the longest phase record a misleading error. This is a reminder, not a new design issue.
- Counterexample: SIGTERM to the runner while the late restorer (545 s on this fixture) or the play child (440 s; hours for E2's 166M-twist word) is running. Record: result 'error', error 'lattice End error is integrated after packet P3' instead of 'signal 15'. phases.lattice keeps its null result.
- Fix: As decision 6 says: I1 adds `self.check_signal()` right after `plugin.play(...)` returns in runner.lattice(), before the End/LatticeResult dispatch. Keep decision 4: do not commit P3 without I1's runner and test edits.

### P4-contract-01 (minor, P4)

- Where: research/jumbling/restoration/13-tutorial-runner/scripts/test_check_contracts.py:431 (code at check_contracts.py:227-228)
- Claim: No test covers the packet's end-state part of 'Max <= h' (P4.md section 4.3: 'holds at the step's start and end state'). The clause `after.maximum > spec.maximum` in check_contracts.py:227 can be deleted and all 11 tests still pass. For rules with no pose guard (r22d81/H6, h4/L4, r22d75/T*), that clause is the only thing that catches a step which raises a piece above the sub-phase maximum.
- Counterexample: A forged h4/L4 row A -> Z36 by (12, inverse of 14), with tau_before/tau_after null and u 0/0. 'n4 falls' holds because the height-4 piece was raised to 6. Under the mutant the row passes, and the suite does not detect the regression.
- Fix: In test 9, add this row (h4/L4, start A, word [[12, INVERSE[14]]], null taus) and assert exactly ['height_max'], or at least that 'height_max' is in the result. Optionally add an H6 or r22d75 case whose end state exceeds the maximum.

### P4-falsepass-01 (minor, P4)

- Where: research/jumbling/restoration/13-tutorial-runner/scripts/test_check_contracts.py:398-406 (test_06_u_progress); check_contracts.py:254
- Claim: No test ever triggers a 'u does not rise' failure for r22d81/H4, r22d81/H2 or h4/L2. Test 6 looks for a rising-u tau only on reverse clean-add rows. On those rows u can never rise, so the test always prints its skip note. If the 'keep' clause were deleted or inverted, all 11 tests would still pass.
- Counterexample: Experiment <scratch> uses forward rows. Z36 → Z36·(2,12) labelled r22d81/H2 at tau 1 (u 139 → 284) gives ['height_counts', 'height_u_progress']. A → A·(2,12) labelled h4/L2 at tau 1 (u 90 → 235) gives the same. So the branch works today, but changing line 254 to drop the 'keep' clause would leave the acceptance suite green.
- Fix: In test 6, add a row whose u rises, asserting contains='height_u_progress'. Use the forward clean add X → X·L as r22d81/H2 and as h4/L2 at a tau where oracle u(X·L) > u(X); tau 1 works at both Z36 and A. Then remove the skip, or keep it only for the original NEAR search.

### P4-falsepass-02 (minor, P4)

- Where: research/jumbling/restoration/13-tutorial-runner/scripts/check_contracts.py:299-314 (check_high_phase); check_run.py in the worktree (identical to 97815b1) has no call to it
- Claim: The checker computes the kernel's phase-level letter bound 6·n6₀ + 964·off₀ but never enforces it. check_run.py does not call check_high_phase. A runner-run/3 record whose high rows exceed the bound gets no kernel_letter_bound failure and can pass the checker. P4's function itself is correct and follows the packet: wiring the call is integrator decision 1. Until I1 wires it, a passing record does not show that the kernel contract holds.
- Counterexample: Take a high phase starting at Z36 (off 5800, heights {'2': 5799, '6': 1}) whose high rows sum to 5,591,207 letters, one over the bound. C.check_high_phase(start, rows) returns ['kernel_letter_bound'] (test 11). check_run.check_run on the same record never calls it, so the record shows no bound failure.
- Fix: In I1, call check_high_phase from check_v3_phases with the checker's own starts(4) snapshot ({'off': snap.count, 'heights': snap.heights()}, decision 1 option a). Call it whenever the high phase has steps, including error runs whose result is null, and add the failure to the record. Add a check_run-level test in which a high phase one letter over the bound fails. Do not report the kernel contract as checked end to end until that test exists.

### P5-contract-01 (minor, P5)

- Where: research/jumbling/restoration/13-tutorial-runner/scripts/check_lattice.py:394-399, :449-467, :472-473 (with to_state :310-316)
- Claim: When the replay ends on the model and the rebuilt State fails the frame or label cross-check, the code drops the end State. `state` goes back to where the model segment started, which is neither the step end nor the last configuration reached. The step-5 end checks (after_off, after_digest, labels_identity) are skipped, and stats report labels_identity and digest as None although the end was reached. On the State.apply path the same D7 case writes the end in place and runs every end check, so the two paths behave differently. No test exercises the model path.
- Counterexample: Experiment rev/p5-contract/exp3.py: 20 retained letters from solved, a valid row with labels_identity true, and sim.State.lattice_stickers patched so that frame_agrees[0] is False from the second call on (the rebuilt end State). Result: ['model_labels']; stats letters=20, fast=20, labels_identity=None, digest=None; the state equals the step start (True), not the step end (False). check_run.py then takes Snapshot(st) of the start configuration and replays the following rows from it. This is not a false pass, because model_labels already fails the step.
- Fix: Have to_state return the rebuilt State together with its verdict, or raise an error that carries it. flush() then writes the rebuilt exact State in place, records model_labels and lets the step-5 end checks and the stats run. Add a test through the model path, for example the exp3 patch on a retained row.

### P5-contract-02 (minor, P5)

- Where: research/jumbling/restoration/13-tutorial-runner/scripts/test_check_lattice.py:422; work/reviews/20261020T201249Z-99b4dfec/report.md
- Claim: Test D prints, and the report gives, the throughput of the whole step rather than the model's rate, which the packet asks for. The printed figure understates the model by about 8x.
- Counterexample: Experiments rev/p5-contract/exp1.py and exp2.py, with warm caches and the same dispatcher (fast_letter plus LatticeModel.play): 15,111 letters/s on orbit-style letters (items 1, 2 and 5, one generator each) and 8,816 letters/s on random retained items. At the reported rate, D0's 8,701,310 letters would take about 80 min. At the measured model rate they take about 10 min, against the plan's estimate of about 7.
- Fix: Time the model loop separately: for example a stats field for time spent on the model, or a timed loop over W in test C. Print and report that rate next to the whole-step figure.

### P5-falsepass-01 (minor, P5)

- Where: research/jumbling/restoration/13-tutorial-runner/scripts/check_lattice.py:411-417, :425, :449-467 (also :170-174 for the range pass)
- Claim: The bytes that are replayed are never tied to the bytes that were verified. Step 2 checks the sha256 and letter count in one read (`info = ia16_info(path)`, :411). The tree ranges come from more reads (`ia16_info` and `ia16_ranges`, :170-174). The replay is yet another read, `for codes in ia16_chunks(path):` (:425), and that read passes no `sha=`. After the loop nothing compares `counts['letters']` or the replayed sha256 with `row['letters']` and `row['word_sha256']`. If the file changes between reads, the step passes for a word that is not the recorded one.
- Counterexample: Run directory with a valid W file whose row passes. The file is rewritten with W + (5,2)(5,5) after step 2's `ia16_info` and before the replay pass. `replay_lattice_step` returns [] after playing 22 letters for a row that records 20 letters and W's sha256.
- Fix: Make the replay pass verify itself: call `ia16_chunks(path, sha=hashlib.sha256())` in the replay loop. After the loop, require that `counts['letters'] == row['letters']` (add `letters` otherwise) and that the hexdigest equals `row['word_sha256']` (add `word_sha256` otherwise). This costs no extra pass. Optionally do the same for the range pass in `check_tree_ranges`, or compare ranges from the replay pass's own buffer.

### P5-robust-01 (minor, P5)

- Where: research/jumbling/restoration/13-tutorial-runner/scripts/check_lattice.py:411 (verify pass), :170 and :174 (range passes), :425 (replay pass)
- Claim: The word file is opened four times: `ia16_info` at :411, `ia16_info` again at :170, `ia16_ranges` at :174 and `ia16_chunks` at :425. Only the first pass is checked against `row['word_sha256']` and `row['letters']`. The replay pass at :425 (`for codes in ia16_chunks(path):`) plays whatever bytes it reads, and the code never compares the played count (`counts['letters']`) or a hash of those bytes with the row. If the file is replaced between passes, P5 plays letters it never verified and can still pass. The range comparison at :174 has the same gap.
- Counterexample: A run directory that changes during a long check, for example a second runner process writing into the same output directory or a re-run that renames new words into place. The replacement must end at the row's `after.digest` to pass silently; otherwise `after_digest` catches it. Normal use does not modify the run directory, which is why this is minor and not major.
- Fix: Bind the replayed bytes to the verified ones. Hash and count inside the replay pass (`sha = hashlib.sha256(); for codes in ia16_chunks(path, sha=sha): ...`). Once the loop finishes, report `word_sha256` when `sha.hexdigest() != row['word_sha256']` and `letters` when `counts['letters'] != row['letters']`. Alternatively, open the file once and pass that file object through every pass. The three extra passes would then also read verified bytes.

## Refuted

- P2-contract-01 (minor): If the D19 path exists but is not a readable regular file (a directory, or a permission error), next_step raises instead of returning an End. Packet section 6 says 'Do not raise for them: an End keeps the reason, details and attempt row', and section 4.4 maps a missing dependency to End('error', 'd1
- P3-falsepass-02 (minor): The plugin maps a child report with result 'budget' to End('budget', 'player_clock') with a timed_out attempt, even when it passed the child no clock. The player can report budget only when it gets `--seconds` (literal_lattice.py:402-404 and 521: `deadline = None if seconds is None else ...`). With 

## Nits

- P1-contract-04: For the r22d81_pairs START, packet 6.2 (lines 634-635) requires reason `inconsistent_output` when the tau, the digest or the u differs. The code uses `digest_mismatch` for a wrong digest, following 6.3's general START check (lines 720, 725), and the test accepts either reason. The packet contradicts itself here, and the test does not settle which reason is recorded.
- P1-contract-05: `_details` also copies `n6_n4_n2` (the r22d75 dialect) into End.details. That key is not in the packet's allowed list of details (6.3 lines 746-747: tau, u, steps, n2_n4_n6, digest, targets, best_pair, limit_cpu, max_height).
- P1-contract-06: Test 1 must check every expected value of 4.3, chained digests included. For `h4_unlock_run1.txt` it does not assert the first STEP's end digest `52c9a715…` (packet line 416). Only the chain and the final STUCK digest are checked.
- P1-falsepass-04: The wall-clock subtest of test_07 does not check which script timed out. For r22d81_pairs and two_letter_u, a slow first call (r22d81 or h4) would also pass, without the second script's timeout path ever running.
- P2-contract-02: Four small behaviours go beyond the packet's mapping and record checks (section 4.4) and are not recorded as decisions: (a) the record must hold a finite nonnegative `seconds`, which is not in the packet's list of record checks; (b) `_ledger_status` turns down a ledger with a row named after a path key or forbidden key, so the whole record becomes kernel_record_invalid, although D3 says extra ledger rows are accepted; (c) an `unsupported` record whose `reason` is not hashable gives kernel_record_invalid, while the table says 'any other reason' gives kernel_unsupported; (d) an OSError from ctx.execute maps to kernel_exit, a row the table does not have. The CLI kernel can produce none of these: it always writes `seconds`, registers no providers and writes string reasons. So none of them changes a real result.
- P2-contract-03: Two section 4.6 record fields are written by the code but no test asserts them: `End.details.kernel_result`, and `kernel.seconds` on a proposal row in acceptance mode, which should be the sentinel 1e9 (sections 4.5 and 4.6). B12 checks only the argv `--seconds`. B1 exercises only the landing contract. The lowering contract is covered separately, in B15.
- P2-contract-04: The durations in Codex's report (packet section 7 return format) come from a different run than the wrapper's acceptance log. If the integrator copies them into the README or wiki, they will not match the recorded acceptance evidence.
- P2-robust-03: Hashing D19 before the call catches only `FileNotFoundError`. A D19 path that exists but is not a readable regular file (a directory, or EACCES) raises out of `next_step`. The run then ends `error` with `phases.high.result` null and no reason or attempt row. That breaks the packet invariant "Do not raise for them: an End keeps the reason, details and attempt row". The kernel itself would have reported `unsupported`/`d19_missing`, since `path.is_file()` is false.
- P2-robust-04: The kernel runs with the runner's default cwd, the repository root. Every path in its argv is absolute, and the kernel pins D19's cwd itself (`cwd=K.ROOT`), so it does not need that cwd. But anything cwd-relative lands in the repository. The realistic case is a core file from a NumPy or interpreter crash when core dumps are enabled: this machine has `core_pattern` = `core` (relative) and only `ulimit -c 0` prevents it. The runner already confines its other long children (`cwd=self.out` for the lattice restorer and verifier).
- P3-contract-02: Every observer-side failure surfaces as `player_failed` with `details.failure = 'input_format'`, which hides the cause. This covers its named checks (`step_lattice`, `factor_boundary`, `correction_boundary`, `final_boundary`, `orbit_partition`, `step_boundary`) and an OSError from Ia16Writer or write_tree. It is never `solved`, so the packet's rule holds, but the named check that the plugin's design relies on is lost.
- P3-contract-03: Some failures take reasons that the section 4.5 table reserves for other outcomes. A failure to read or gunzip `LIBRARY`, to parse it, or to create the work files and directories maps to `restorer_output_missing`, although the restorer has not run yet. An OSError raised by `ctx.execute` maps to `restorer_exit` or `player_exit`. The table has no row for these, so the reason names are misleading.
- P3-contract-04: Section 4.6 says part names never contain `/`, `\` or `~`. The correction tree names spans `macro <m> <family>` straight from the library, and nothing enforces the rule. The step-7 parent check uses only `private_path`, which accepts an inner `/` (for example `a/b`), and check_run's tree check (:503) does not test names. The rule holds today only because the current library has no such family (change log section 5).
- P3-contract-05: Test D's independent assertions do not cover everything the packet lists. Packet test D asks for "the fields of section 4.6" for every step, but exact fields are asserted only for the first step of D1 and the last step of D2. The fields of the other steps (orbit-34 with factor_first 1 and factor_count 2, and its tree_folded and tree_folded_depth) are checked only through the plugin's own `expected_steps` and `orbit_partition`, so the check is self-referential. "Files and report hold no absolute path" is checked only with the plugin's own `L.safe_record` and `L.private_path` (these do match check_run.py:135-138).
- P3-contract-06: Section 7 asks for each I0 interface change "stated as the change, not made". The report only names areas: "I1 still needs zero-step recording, lattice End recording, report persistence, and composition-path integration." It does not state the change, for example "played_lattice must accept steps == [] when the end table's digest equals the snapshot digest and record phases.lattice with no row". The report's timings (suite 180.415 s, observer 0.018810 s for 24 twists) also come from a different run than acceptance.log (suite 156.982 s, observer 0.009056 s).
- P3-robust-02: The test's temporary directory is created at import and removed only at the normal end of main(), not in a finally block, a tearDownModule or an atexit hook. Any other way of running or ending the suite leaves it in the system temp dir. It holds the home-centroid cache, restorer stub outputs, real late-restorer output from test H and copied play outputs. test_literal_lattice.py cleans up with setUpModule/tearDownModule and addCleanup.
- P4-contract-02: Several 'decided here' type rules are implemented correctly but no test exercises them: non-null u_before/u_after/u_mid on an r22d75 row (height_fields), and kernel_fields for a non-int off_before/off_after or a non-int value in heights_after. For the kernel cases, kernel_counts cannot stand in, because True == 1 and 3098.0 == 3098 compare equal.
- P4-robust-01: The test harness creates its temporary directory when the module is imported, but only main() removes it. Any other import path leaks it, for example `python -m unittest test_check_contracts`, an importlib load, or a status()/inventory() failure before the try. The git child is also run without a timeout.
- P4-robust-02: The D4 scan in test 11 bans `subprocess`, `multiprocessing` and `ctypes` to stop the checker from starting a child process. It still allows `import os` with `os.system`, `os.posix_spawn` and `os.exec*`, and it does not catch dynamic lookups such as `sys.modules.get('e'+'b6')` or `getattr(__builtins__, '__imp'+'ort__')`. A later edit could run or reach rule code without failing the guard. The delivered check_contracts.py does none of this.
- P5-contract-03: The dispatcher predicate takes its retained-item table from restore.menu('I_a'), not from the `menu` that replay_lattice_step receives. The model's guard and the rest of the replay use the passed menu.
- P5-contract-04: The rewritten read_ia16 changes the stub's check order. A code of 36,000 or more now raises before sha256 is compared, so a non-lattice ia16 word with both defects gets `word_file` from check_run.load_word_v3 where the stub gave `word_sha256`. A missing file now raises a ValueError instead of an OSError. Section 4.1's contract still holds, but the report does not list these behaviour changes, which section 7 asks for.
- P5-contract-05: Two tree_malformed rules of packet 4.5 have code but no direct test. Separately, check_tree_ranges makes an extra full pass over the word, so a replay decompresses the word four times.
- P5-falsepass-02: Test C's check that `from_state` refuses an off-lattice state passes even without the module's own refusal. The test asserts the base class `ValueError`, and the simulator raises `ValueError` itself on an off-lattice state, so the explicit guard at check_lattice.py:262-263 is not exercised.
- P5-robust-02: P5 finds the run root by counting parents of the already resolved word path: `n = len(PurePosixPath(word).parts)`, then `base = Path(path).parents[n - 1].resolve()`. `check_run.inside` (check_run.py:130, `path = (base / name).resolve()`) resolves symlinks before the path reaches P5. A word file that is a symlink to a deeper file inside the run is therefore accepted by check_run, but P5 derives the wrong root and reports `path`. This is a false failure on a layout that check_run allows. It never produces a false pass.

## Integration check

- L1a-height-none (nit): None found. The height rows P1 records match what P4 checks: rule and contract names, contract = rule, subphases, letter counts, tau/u/u_mid field presence, types and nullability, and u semantics (each u is a straddler count at the recorded tau). The gate is the script stem, which P4 leaves to the runner side by design.
- L1b-kernel-none (nit): None found apart from the known params.unit item (K3). The high rows P2 records match P4's kernel_fields and kernel_counts. The rule is r22d25/<family>, the contract is kernel_<kind>, the gate is r22d25, and target, params, high_exact, off_before, off_after and heights_after (str to int, equal to the checker's histogram) all have the expected types.
- L2-lattice-none (nit): None found. P5 replays exactly what P3 records for lattice steps: ia16 words, tree spans with inverse_of and same_as (including the trailing inverse_of path+(0,) of conj nodes whose u is a 'path' leaf, nested conj/prod, folded group spans, empty word and prod as zero-length spans, correction repetitions), labels_identity on the last step only, and invisible-rotation end digests.
- K1-played-lattice-result-unchecked (minor): Known, deferred to I1 (README.md:966-967; P5.md:60-63). For a record with played lattice rows, check_run does not compare phases.lattice.result, so a P3-produced record still passes with any value there.
- K2-stub-dependent-tests (minor): Known (integrator decision 2; P3.md:548-555; P4.md:54-61; P5.md:181-187). With all five overlays, the I0 tests that assert stub behaviour fail exactly as the packets predict. No unexpected failure appeared.
  test_check_run.py -k test_v3 -k IndependenceTests: 26 tests in 245 s, 5 FAIL. All five are predicted:
  - test_v3_height_unchecked_contracts, test_v3_interrupted_phase_entry and test_v3_verdict_earlier_timeout (P4 rejects the 'height_order' rows);
  - test_v3_played_lattice (expects unchecked_contract; now 0);
  - test_v3_lattice_labels_not_digest (P5 correctly fails labels_identity on the two-letter prefix).
  IndependenceTests pass with the new check_contracts.py and check_lattice.py.
  
  test_runner.py, selected tests:
  - PhaseTests: 12 ok, 1 FAIL. test_not_applicable_row expects rules_height/rules_kernel to be stubs.
  - LatticeTests.test_lattice_ends_are_errors_until_p3 imports the real lattice_phase and starts the real restore_lattice_late.py child. I killed it after about 2 minutes.
  - Second batch, 9 tests in 10 s: 4 FAIL, all predicted:
    - test_07 subtests a and b and test_08 'intact': the scripted Plugin rows fail ['height_rule','height_fields','height_tau'(,'height_length')];
    - test_09: unchecked_contracts 0 != 2.
  - test_10, the two other LatticeTests and TreeTests pass.
  
  Logs: <scratch>, test_runner.sel.log and test_runner.sel2.log.
- K3-known-I1-items-reconfirmed (minor): Known items already recorded by the integrator, reconfirmed and not new.
1. An integer params.unit in d19 high rows fails check_run 'path' (decision 3).
2. check_contracts.check_high_phase is never called by check_run (decision 1).
3. config.kernel.d19 is null.
4. The runner refuses a zero-step LatticeResult.
5. A lattice End is not recorded.
6. LatticeResult.report is not recorded.
