# Scoped verification packet: Windows verification batch (L2), round 2

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: the second and last scoped verification round (Sol, fast tier) of the Windows verification batch. Round 1 (`20261003T080931Z-e5504727`) confirmed WB-L2-01, 02, 04 and 05 as fixed and kept WB-L2-03 open as `major`: "Interruption before add() discards an already completed result". Under the risk tiers of 2 October 2026 this is non-critical tooling.
- Check only two things:
  1. Is WB-L2-03 fixed in the current candidate? A Ctrl+C that arrives after a step has returned must not discard its result, and the completed results must still be published.
  2. Does the fix introduce a new `blocker` or `major` in the code it changed? The wrong outcomes that count are the full review's:
     - private text in `work/windows-batch/<date>/batch.json` or `summary.md`;
     - a false pass, or a failure reported as `skipped`;
     - a result taken from the wrong run;
     - a manual action without a `>>>` line and an Enter wait, or a capture without the heavy-work guard;
     - data outside fresh synthetic data and the batch's own directories, or an installation;
     - an unbounded hang, a child process tree left running, or completed results not written after an interruption;
     - a README statement that the code contradicts.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. Report only `blocker` or `major` findings, each with a concrete counterexample: inputs or state, then the wrong outcome. Verdict `pass` if there is none.
- Out of scope: WB-L2-01, 02, 04 and 05 (verified in round 1); code the fix did not change; `minor` and `nit` findings; the choices in section 6.

## 2. Actual problem and reproduction
- Round 1 finding: after `step.run()` returned, SIGINT stayed enabled until `add()` entered `_ignore_interrupts()`. A KeyboardInterrupt at `result = add(step, planned)` reached the outer handler, which discarded `planned`. Counterexample: native returned PASS 19/19 and was published as `interrupted` with empty counts.
- Root cause: entering the protection took calls (`add()`, the context manager, `signal.getsignal`, and `signal.signal`, which runs pending handlers before it changes the handler). Python can raise a pending KeyboardInterrupt at any of those calls. The same kind of gap also existed between the end of the step loop and the recovery's `with`.
- The fix replaces calls with a flag. In `tools/windows_batch/run_all.py`:
  - `_InterruptHold` is the SIGINT handler for the whole run. `main` installs it after `--list` and before any directory exists, and restores the previous handler in a `finally`, after it sets `held = True`.
  - The handler raises KeyboardInterrupt only when `held` is false and the current frame is not `_run_batch`'s own frame. Otherwise it sets `pending`.
  - `held` starts true. `release()` sets `held = False`, then raises KeyboardInterrupt if `pending` was set, clearing it.
  - Only two regions run with the hold released:
    - the start prompt: `release()`, then `context.manual(...)`, then `held = True`;
    - each step: `release()` before the inner `try`, then `ctx.private.mkdir()`, `ctx.scratch.mkdir()` and `step.run(ctx)`. The first statement of the `else`, `except KeyboardInterrupt` and `except Exception` branches is `interrupts.held = True`.
  - Everything else runs held: source identity, the plan, `add()`, the progress line, the recovery, publication and scratch removal. A Ctrl+C there sets `pending`, and the next `release()` raises it:
    - before the start prompt: the first step is `interrupted before the batch started`, the rest are `skipped`;
    - before the next step: that step is `interrupted between steps`, the rest are `skipped`.
  - A Ctrl+C held after the last step is dropped, and the batch ends with the outcome of its results.
  - `add()` no longer touches signals. `main`'s former body from the clock line to the final `return` moved, otherwise unchanged, into `_run_batch(...)`, so that the handler can recognise its frame.
- Why no gap remains between a step's return and the hold:
  - CPython handles a pending signal only at eval-breaker checks: function entry, backward jumps and after calls to callables that are not inlined Python functions. It also handles one inside C functions that call `PyErr_CheckSignals`.
  - None of these lies between the inlined return of a Python `step.run` and the next statement. That statement, `interrupts.held = True`, is a plain attribute store. In the `except` branches, it runs before any call.
  - Where a check does run in the batch frame (for example after a callable that is not a Python function), the frame rule holds the signal.
- `tools/windows_batch/README.md` now says that a Ctrl+C at a prompt or during a step interrupts the batch. A Ctrl+C pressed while the batch plans, records, publishes or removes scratch data is held until the next step would start.

## 3. Environment and versions
- Windows 11 (10.0.26200), CPython 3.14.7 64-bit, Windows PowerShell 5.1.
- The review sandbox is read-only, with no GPU and no administrator rights. Evidence kind there: source and fixtures.

## 4. Necessary source and evidence
- Changed in this round:
  - `tools/windows_batch/run_all.py`: `_InterruptHold`, `_run_batch`, `main`; `_ignore_interrupts` is removed;
  - `tools/windows_batch/README.md`: the Ctrl+C paragraph;
  - `tests/test_windows_batch.py`: `test_publication_and_cleanup_ignore_ctrl_c` is replaced by five tests.
- The five tests use real signals through `_thread.interrupt_main()`, which goes through the same Python-level handler path as a console Ctrl+C:
  - `test_ctrl_c_while_a_result_is_recorded_keeps_it` covers the round-1 counterexample:
    - the signal arrives inside `normalize()` while native's PASS result is recorded;
    - expected statuses: native `pass` with `counts == {"checks": 19}` and `lines == ["OK"]`, migration `interrupted` ("interrupted between steps"), renderer `skipped`;
    - expected exit 130, and the scratch directory is removed;
  - `test_ctrl_c_during_a_step_interrupts_it`: the step is `interrupted by the owner` and the next step is skipped;
  - `test_ctrl_c_while_planning_stops_before_the_start_prompt`: the source identity is kept, the first step is `interrupted before the batch started`, and the start prompt is never shown;
  - `test_publication_and_cleanup_hold_ctrl_c`:
    - during publication the handler is the hold and is held;
    - a Ctrl+C there is held, so the outcome is `pass` with exit 0;
    - the scratch directory is removed and the original handler is restored;
  - `test_hold_never_raises_in_the_batch_frame`: a unit test of the frame rule and of `release()`.
- Integrator's checks on the current candidate:
  - `python tests/test_windows_batch.py`: 46 tests OK;
  - `python tests/test_windows_batch_steps.py`: 80 tests OK;
  - `python tools/ci/run_headless.py --list` lists both files;
  - `python tools/windows_batch/run_all.py --list`: plan only, exit 0.
- Mutation check on a scratch copy. Each of these mutants either fails named tests or aborts the run with an escaped KeyboardInterrupt:
  - `held` ignored in the handler;
  - no frame rule;
  - the hold never installed;
  - `release()` ignoring `pending`;
  - no hold after a step.
- Two real non-elevated runs of `run_all.py --only native` after the change:
  - one Enter, so EOF at the step's own prompt: `interrupted by the owner`, exit 130, published;
  - two Enters: `pass`, 19 of 19 checks, exit 0.
  - In both runs, neither public file leaked and the scratch root was removed.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | the candidate meets the task | full Sol review `20261003T074248Z-22b99cad` | five `major` findings | all adopted |
| 2 | SIG_IGN around recording and publication closes WB-L2-03 | `_ignore_interrupts()` | scoped round 1 `20261003T080931Z-e5504727` | WB-L2-03 open: the protection is entered through calls |
| 3 | a flag-based handler leaves no gap | this candidate (section 2) | section 4 checks | all pass |

## 6. Constraints and owned files
- Read-only review.
- Out of scope, unless they cause a wrong outcome from section 1:
  - the full review's design choices and the round-1 constraints;
  - holding Ctrl+C during source identity and planning:
    - the availability checks only read files;
    - `git` has a 30-second timeout and receives the console's Ctrl+C itself;
    - the held Ctrl+C stops the batch at the start prompt;
  - a Ctrl+C held after the last step is dropped;
  - a signal handled in the batch frame while the hold is released is held, not raised; it stops the batch before the next step;
  - when the handler cannot be installed, the batch keeps Python's default behaviour. That happens outside the main thread, where SIGINT is never delivered, or under a handler not installed from Python;
  - `BatchContext.run`'s `stop_tree` path, which this round does not change;
  - `minor` and `nit` findings.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
