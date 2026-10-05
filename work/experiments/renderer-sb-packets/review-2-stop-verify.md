# Scoped verification packet: S-B capture-script cleanup findings R1 and R2 (round 1 of 2)

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: check only two things:
  - whether the two blocking findings of review `20261003T040442Z-0b358eeb` are fixed;
  - whether the fix introduces a new `blocker` or `major`.
- Findings under verification:
  - R1 (major): an exception from the stop helper inside `finally` aborted the rest of the cleanup. That left the capture and its named ETW session running.
  - R2 (major): repeated stop-helper timeouts left the named session running. The script had no independent fallback, no final session check and no report.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`.
  - One finding per unresolved item, with the same ID.
  - Any new `blocker` or `major`, each with a concrete counterexample.
  - Verdict `pass` if both are fixed and nothing new blocks.

## 2. Actual problem and reproduction
- The fix is uncommitted, in the working tree on top of `4d5da1a`. See `git diff -- work/experiments/renderer-sb/probe/run_scene.ps1 work/experiments/renderer-sb/probe/README.md`; it includes the change that the first review covered.
- Helpers (all bounded, each run hidden through `Run-Helper`):
  - `Run-Helper` waits at most 30 s. It returns the exit code, or -1 after a kill.
  - `Trace-Session` runs `logman.exe query <name> -ets` through `Run-Helper`. It now reads `ExitCode` instead of `$LASTEXITCODE` after `cmd /c`.
  - `Stop-Capture` runs `PresentMon --session_name <name> --terminate_existing_session` through `Run-Helper`.
- Fixed session name `magic600-sb-capture` instead of one per run. `Assert-NoSession` refuses to start the script, and every later run, while a session named `PresentMon` or `magic600-sb-capture` is running or cannot be queried. This means a session left by any earlier run or script invocation is always found by its exact name.
- R1: `finally` now guards every step with its own `try`/`catch` and `Write-Warning`:
  1. the probe kill;
  2. PresentMon's stop with a 30 s wait, then a kill;
  3. a separate kill if PresentMon still runs;
  4. `Remove-Session`, which is called unconditionally.
- R2: `Remove-Session <name>`:
  - tries PresentMon's stop, then `logman stop <name> -ets` as an independent controller, each step guarded and stopping once the session is gone;
  - queries once more at the end;
  - if the session is still not `missing`, warns with the exact stop command. The next run, or the next invocation, then refuses through `Assert-NoSession`.
- The success path is unchanged:
  1. a 2 s pause, then a refusal if PresentMon already exited;
  2. `Stop-Capture`, which must exit 0;
  3. PresentMon must exit 0 within 60 s;
  4. the session must be `missing`;
  5. the CSV must end with LF.

## 3. Environment and versions
- As in `work/experiments/renderer-sb-packets/review-2-stop.md` section 3.
- Evidence kind in the review sandbox: source/fixture only.

## 4. Necessary source and evidence
- First-round result: `work/reviews/20261003T040442Z-0b358eeb/review.json`, if readable; otherwise section 1.
- Changed files: `work/experiments/renderer-sb/probe/run_scene.ps1` (helpers, `Assert-NoSession`, the per-run check, the stop block and `finally`) and `work/experiments/renderer-sb/probe/README.md` (the capture-script paragraph).
- Integrator's fixture: `work/loop-memory/perf/renderer/sb/cleanup_test.ps1`, a private scratch copy scoped to this review.
  - It extracts the script's function definitions, the post-probe stop block and the `finally` body verbatim.
  - It runs them in Windows PowerShell 5.1 against a simulated `Start-Process` (PresentMon, logman and ETW session state) and fake process objects. `WaitForExit()` returns nothing and `WaitForExit(ms)` returns a bool, as `System.Diagnostics.Process` does.
  - Results:

  | Case | Outcome | Session | Next run |
  |---|---|---|---|
  | `finally`, PresentMon stop launch throws (R1 counterexample) | completes; 2 warnings; capture killed | removed by logman | allowed |
  | `finally`, PresentMon stop hangs, logman works (R2) | completes | removed | allowed |
  | `finally`, both controllers fail | completes; warning with the stop command | running | refused |
  | `finally`, PresentMon stop works | completes | removed | allowed |
  | `finally` after a clean stop | completes | removed | allowed |
  | stop block, normal | completes | removed | allowed |
  | stop block, PresentMon stop hangs, then `finally` | throws "Stopping the PresentMon session failed (exit -1)" | removed | allowed |
  | stop block, PresentMon exited early with 5, then `finally` | throws "PresentMon exited 5 before its session was stopped" | removed | allowed |

- Real unelevated helpers on the owner's machine, not PresentMon evidence:
  - `Trace-Session` on a nonexistent name and on `PresentMon` (now stopped) both return `missing`;
  - `Stop-Capture` on a nonexistent name returns 7 at once;
  - the script parses with 0 errors.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | an explicit named-session stop ends the capture cleanly | first fix | review `20261003T040442Z-0b358eeb` | R1 and R2: cleanup could abort or leave the session |
| 2 | guarded cleanup steps, an independent logman stop, bounded helpers, a fixed name with a pre-run check | section 2 | fixture and helper checks in section 4 | as listed |

## 6. Constraints and owned files
- Read-only review.
- Out of scope:
  - the probe C++ sources;
  - the gate tools;
  - performance numbers;
  - the operator confirmation and display checks, unless this fix breaks them;
  - style;
  - `minor` and `nit` findings.
- Questions:
  1. Can any exit path still skip a cleanup step, or end with the session running without a warning and without a refused next run?
  2. Can a helper hang without a bound?
  3. Does the fixed session name create a new failure, for example a false refusal of a normal run, or a stop of a session the script did not start?
  4. Do the bounded `Trace-Session` and `Run-Helper` keep the exit-code mapping correct (0, and -2144337918 for 0x80300002)?

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
