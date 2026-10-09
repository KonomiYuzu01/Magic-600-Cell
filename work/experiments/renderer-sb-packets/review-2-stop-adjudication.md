# Adjudication record: R3 of verification round 2 (`20261003T043410Z-9eeb7f53`)

## Finding and owner decision
- Finding R3 (major, unverified): "A surviving stop helper can stop a later invocation's capture."
- Counterexample:
  1. A's PresentMon stop helper stalls before issuing its named-session stop.
  2. Both waits time out and `Kill` fails.
  3. A's logman fallback removes A's session, and A releases the ownership mutex.
  4. B acquires the mutex and starts `magic600-sb-capture`.
  5. A's surviving helper then resumes and stops B's session.
- Round 2 was the last verification round allowed for this candidate.
- The owner chose an adjudicating experiment from three options:
  - an owner exception;
  - an adjudicating experiment;
  - a redesign with a new review.

## Experiment
- Owner's machine, administrator PowerShell, PresentMon 2.6.0.0 at the pinned path.
- The experiment script took the capture script's ownership mutex, used a session name of its own (`magic600-sb-adjudicate`), and loaded `run_scene.ps1`'s helper functions verbatim from the candidate source:
  - `Native-Arguments`, `Run-Helper`, `Trace-Session`, `Stop-Capture`, `Remove-Session` and `Assert-NoSession`;
  - `Trace-Session` maps `logman query <name> -ets`.
- The stop helper was the capture script's exact stop command, `PresentMon --session_name <name> --terminate_existing_session`. It was created suspended in a new hidden console, as `Start-Process -WindowStyle Hidden` gives the script's helpers, so it stalled before its first instruction.
- Control (the helper resumed while alive):
  1. A capture with the capture script's arguments (target process absent) started the session.
  2. The suspended helper was created.
  3. After 5 s it was resumed.
- Test (the helper killed as `Run-Helper` kills it):
  1. A suspended helper was created.
  2. It was killed with `Kill()`, then `WaitForExit(5000)`.
  3. A new capture started the session under the same name.
  4. The helper's thread was resumed.
  5. The session was queried 10 s and 30 s later.
  6. The capture was ended with the script's own `Stop-Capture`.

## Result (2026-10-03, private record `stop-helper-20261003T044640Z`)

| Step | Control | Test |
|---|---|---|
| session before the helper acts | running | running (started after the kill) |
| while the helper is suspended | running | — |
| kill | — | returned; the helper exited within 5 s with exit -1 |
| resume | previous suspend count 1; the helper exited 0 | previous suspend count 0 (nothing runs) |
| session afterwards | missing: the live helper stopped it | running at 10 s and at 30 s; the capture still running |
| end | the capture exited 0 | `Stop-Capture` exit 0, the capture exited 0, the session missing |

- Verdict: pass. A stalled stop helper that has been killed the way `Run-Helper` kills it does not stop a session started later under the same name. The control shows that the same helper, left alive, does stop the session, and that the check detects the stop.
- The counterexample's premise that `Kill` fails did not occur: `Kill` returned and the process exited within the 5 s bound. For the script's own child with a cached handle, `Kill` throws only when the process has already exited.
- If the corner did occur, the affected run would still be refused, never admitted. A capture whose session is stopped early exits before the script's own stop, and the script refuses that as "PresentMon exited before its session was stopped".
- Disposition: `{"R3": "reject_with_evidence"}`.
