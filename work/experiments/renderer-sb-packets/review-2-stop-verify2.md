# Scoped verification packet: S-B capture-script findings R2 and R3 (round 2 of 2)

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: check only two things:
  - whether the two blocking findings of verification round 1 (`20261003T041458Z-f77724d2`) are fixed;
  - whether the fix introduces a new `blocker` or `major`.
- Findings under verification:
  - R2 (major): a timed-out helper could block cleanup indefinitely. `Run-Helper` and `finally` called a parameterless `WaitForExit()` after `Kill()`.
  - R3 (major): an earlier invocation could stop a later invocation's capture. The cause was the shared session name `magic600-sb-capture` together with the unconditional `Remove-Session` in `finally`. Counterexample: A waits at the operator prompt, B passes its own session check and starts capturing, and A's cleanup then stops B's session.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`.
  - One finding per unresolved item, with the same ID.
  - Any new `blocker` or `major`, each with a concrete counterexample.
  - Verdict `pass` if both are fixed and nothing new blocks.
- This is the last verification round this candidate may get. A finding that still blocks goes to the owner as a decision.

## 2. Actual problem and reproduction
- The fix is uncommitted, in the working tree on top of `4d5da1a`. It includes the changes the full review and round 1 covered.
  - View it with `git diff -w -- work/experiments/renderer-sb/probe/run_scene.ps1 work/experiments/renderer-sb/probe/README.md`.
  - Use `-w`: the main body is re-indented by four spaces inside the new owned region.
- R2, every wait is now bounded:
  - `Run-Helper`: `WaitForExit(30000)`; on a timeout, a guarded `Kill()`, then `WaitForExit(5000)`, and it returns -1 whether or not the kill took effect.
  - `finally`: each `Kill()` of the probe or PresentMon is followed by `WaitForExit(5000)`.
  - The probe wait itself is bounded to its preroll and duration plus 300 s, and throws on a timeout.
  - The script contains no parameterless `WaitForExit()`.
  - `Remove-Session` now skips a controller only when the session is `missing`, no longer whenever it is not `running`. An `unknown` state, for example after a hung `logman query`, therefore still reaches both controllers and the final report.
- R3, exclusive ownership across invocations:
  - Before its first session check, the script creates `Global\magic600-sb-capture` and takes it with `WaitOne(0)`.
    - If the mutex is held, the script refuses: "Another run_scene.ps1 invocation is running".
    - An `AbandonedMutexException`, which means a killed invocation's mutex, counts as acquired.
  - Everything from the first `Assert-NoSession` through the last run's `finally` cleanup runs inside one `try` whose `finally` releases and disposes the mutex.
  - So no second invocation can start, stop or remove a session under the shared name between this invocation's preflight check and its last cleanup.
  - After a killed invocation, the next one owns the abandoned mutex, and its `Assert-NoSession` refuses any session the killed one left running.
  - The gate summary runs after the release. It only reads files.

## 3. Environment and versions
- As in `work/experiments/renderer-sb-packets/review-2-stop.md` section 3.
- Evidence kind in the review sandbox: source/fixture only.

## 4. Necessary source and evidence
- Round-1 result: `work/reviews/20261003T041458Z-f77724d2/review.json`, if readable; otherwise section 1. Its dispositions are `{"R2": "adopt", "R3": "adopt"}`.
- Changed files:
  - `work/experiments/renderer-sb/probe/run_scene.ps1`: `Run-Helper`, `Remove-Session`, the ownership block, the owned region, the probe wait and `finally`;
  - `work/experiments/renderer-sb/probe/README.md`: the capture-script paragraph.
- Integrator's fixtures: private scratch copies scoped to this review, in `work/loop-memory/perf/renderer/sb/`. Both run in Windows PowerShell 5.1, unelevated.
- `cleanup_test.ps1` (R2):
  - It extracts the function definitions, the post-probe stop block and the `finally` body verbatim. It runs them against a simulated `Start-Process` and fake processes.
  - A parameterless `WaitForExit()` throws. Every timed-out `WaitForExit(ms)` adds `ms` to a simulated clock.
  - Process modes:
    - `hang`: waits time out until a kill or a session stop;
    - `stuck`: waits time out and a kill has no effect.
  - Results:

  | Case | Outcome | Session | logman stop reached | Simulated waits | Next run |
  |---|---|---|---|---|---|
  | `finally`, PresentMon stop launch throws (R1) | completes, 2 warnings | removed | yes | 0 s | allowed |
  | `finally`, PresentMon stop hangs, logman works | completes | removed | yes | 90 s | allowed |
  | `finally`, both controllers fail | completes, warning with the stop command | running | yes | 90 s | refused |
  | `finally`, PresentMon stop works | completes | removed | no (not needed) | 0 s | allowed |
  | `finally` after a clean stop | completes | removed | no | 0 s | allowed |
  | `finally`, no kill takes effect (probe, capture, both stop helpers) | completes, warning | running | yes | 150 s | refused |
  | the same, and `logman query` hangs too | completes, warning "still unknown (logman exit -1)" | running | yes | 255 s (the analytic worst case) | refused |
  | stop block, normal | completes | removed | no | 0 s | allowed |
  | stop block, PresentMon stop hangs, then `finally` | throws "Stopping the PresentMon session failed (exit -1)" | removed | yes | 120 s | allowed |
  | stop block, PresentMon stop and capture stuck, then `finally` | throws the same | removed | yes | 145 s | allowed |
  | stop block, capture already exited 5, then `finally` | throws "PresentMon exited 5 before its session was stopped" | removed | no | 0 s | allowed |

- `ownership_test.ps1` (R3):
  - Structure, from the script's AST:
    - exactly one top-level `try` whose `finally` releases the mutex, spanning lines 83 to 200;
    - its first statement is `Assert-NoSession`;
    - the mutex is acquired before it;
    - none of `Assert-NoSession`, `Trace-Session`, `Stop-Capture`, `Remove-Session`, `Run-Helper` or `Start-Process` is called outside it, other than inside the helper definitions.
  - Real processes: separate `powershell.exe` processes run the ownership block verbatim. The only change is the mutex name, replaced by a test-only `Global\` name.

  | Case | Result |
  |---|---|
  | A holds ownership, as anywhere from its first session check to its last cleanup, for example at the operator prompt; B starts | B refuses before its session check ("Another run_scene.ps1 invocation is running"), so B never starts a session that A's cleanup could stop |
  | A releases, as its outer `finally` does; C starts | C acquires |
  | D acquires and is killed while this test keeps a non-owning handle open, so the mutex survives as abandoned; E starts | E acquires through the `AbandonedMutexException` branch |
  | control: the same as D/E with the `catch` removed | the start refuses with "The wait completed due to an abandoned mutex", which shows that E took that branch |

- Earlier fixtures re-run on the patched script with unchanged results:
  - the displayed-present block: 2 accepted, 2 refused;
  - the operator confirmation and placeholder fill, on a 3 s real probe run: `yes` and padded `yes` kept; `no` and empty refused; the injection run kept as not gate evidence;
  - real unelevated helpers: `Trace-Session` on a nonexistent name returns `missing`, `Stop-Capture` on one returns 7 at once, and the script parses with 0 errors.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | an explicit named-session stop ends the capture cleanly | first fix | review `20261003T040442Z-0b358eeb` | R1, R2 |
| 2 | guarded cleanup, an independent logman stop, bounded helpers, a fixed name with a pre-run check | second fix | round 1, `20261003T041458Z-f77724d2` | R1 fixed; R2 open (post-kill waits); new R3 (cross-invocation) |
| 3 | every wait bounded; one invocation at a time, from preflight through cleanup | section 2 | fixtures in section 4 | as listed |

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
  1. Does any wait in the script remain unbounded, other than the operator prompt (`Read-Host`), which is intentional?
  2. Can an invocation still start, stop or remove a trace session under the shared name while another invocation is between its first session check and its last cleanup?
  3. Does the ownership block create a new failure? For example:
     - a false refusal of a normal elevated run;
     - a mutex left owned after an exit path;
     - a release from the wrong thread in Windows PowerShell 5.1;
     - a broken abandoned case.
  4. Does the re-indentation change behaviour anywhere? The script has no here-strings.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
