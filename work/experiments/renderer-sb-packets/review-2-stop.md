# Review packet: explicit PresentMon stop in the S-B capture script

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: one Sol review (fast tier, non-critical tooling) of a fix to `work/experiments/renderer-sb/probe/run_scene.ps1`. The fix replaces the PresentMon self-termination, which never fired on the owner's machine, with an explicit stop of a named trace session, and it guards against a leftover trace session.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. Report only `blocker` or `major` findings, each with a concrete counterexample: inputs or state, then the wrong outcome.
  - Wrong outcomes that count:
    - a capture admitted with incomplete or altered gate-interval rows;
    - a trace session left running after any exit path;
    - a hang without a bound;
    - a false refusal of a normal elevated run;
    - a broken existing check (exit 3 handling, the displayed-present bins, the operator confirmation, the placeholder fill).
  - Verdict `pass` if there is none.

## 2. Actual problem and reproduction
- Observed on the owner's machine with the committed script (`4d5da1a`): the first preliminary W3 capture (PresentMon 2.6.0.0, elevated, `--v1_metrics --qpc_time --process_id <pid> --output_file <csv> --terminate_on_proc_exit`).
  - The probe ran normally and exited 0. Its label check passed, and all 1,908 visibility samples were fine.
  - PresentMon was still running 60 s after the probe exited. The script threw "PresentMon did not stop within 60 s", and its `finally` block killed PresentMon.
  - The CSV's last row was cut mid-field: 154,730 rows, the last without `QPCTime`, against the probe's 154,731 presents. All gate-interval rows were present.
  - Afterwards, an unelevated `logman query PresentMon -ets` returned access denied (HRESULT 0x80070005), while a nonexistent name returned not found (0x80300002). This showed that the killed capture had left its default-named ETW session running. The owner later stopped it.
- Cause, from the PresentMon source (GameTechDev/PresentMon, console application):
  - `OutputThread.cpp`: the output loop calls `ProcessEvents` only `if (!presentEvents.empty())`. Inside it, a target's termination event is handled only when a later present's time passes the termination QPC. With `--process_id`, no present arrives after the probe's last one, so `HandleTerminatedProcess` never runs and `--terminate_on_proc_exit` never fires.
  - `ConsumerThread.cpp`: after `ProcessTrace` returns, which it also does when an external controller stops the session, the consumer calls `ExitMainThread()`.
  - `MainThread.cpp`: on `--terminate_existing_session`, it calls `StopNamedTraceSession(args.mSessionName)` and returns 0 on success or 7 on failure. After the message loop ends, a capture runs `pmSession.Stop()`, waits for the consumer thread, then stops the output thread. That thread reads its quit flag before one last processing pass, then closes the CSV. The capture returns 0.
- The fix (uncommitted working tree on top of `4d5da1a`; `git diff -- work/experiments/renderer-sb/probe/run_scene.ps1 work/experiments/renderer-sb/probe/README.md`):
  - `Trace-Session` maps the exit code of `cmd /d /c "logman query <name> -ets >nul 2>&1"`: 0 to `running`, -2144337918 (0x80300002, PLA_E_DCS_NOT_FOUND) to `missing`, anything else to `unknown`.
  - Before any run, the script refuses to start if a session named `PresentMon` is running, or if `logman` cannot answer.
  - Each run captures with `--session_name magic600-sb-<run id>` instead of `--terminate_on_proc_exit`. After the probe exits and its exit code is checked:
    1. a 2 s pause for the trace buffers to flush;
    2. a refusal if PresentMon already exited;
    3. `Stop-Capture`: `PresentMon --session_name <name> --terminate_existing_session`, bounded by 30 s, which must exit 0;
    4. PresentMon must exit within 60 s, with code 0;
    5. the session must be `missing`;
    6. the CSV's last byte must be LF.
  - `finally`: if PresentMon is still running, it calls `Stop-Capture`, waits 30 s, and only then kills it. If the session still runs afterwards, it stops the session.
- Gate-side checks that stay unchanged (`tools/perf/renderer_gate.py` `read_frames`): `capture-short`, `capture-not-covered` (first and last kept frames within `EDGE_S` of the interval bounds), `presentmon-rows-missing` (a step between kept presents longer than the later frame's own interval), and a strict CSV reader.

## 3. Environment and versions
- Windows 11 (10.0.26200), Windows PowerShell 5.1, PresentMon 2.6.0.0 at the pinned path, run elevated by the owner.
- The review sandbox cannot run PresentMon, an elevated shell or `logman` with session access. Evidence kind there: source/fixture only.

## 4. Necessary source and evidence
- Changed files:
  - `work/experiments/renderer-sb/probe/run_scene.ps1`: `Trace-Session`, `Stop-Capture`, the leftover-session refusal, the per-run `$session`, `$captureArguments`, the stop block after the probe exit checks, and `finally`;
  - `work/experiments/renderer-sb/probe/README.md`: the capture-script paragraph in "Owner measurements and negative tests".
- Reader: `tools/perf/renderer_gate.py` (`interval_ticks`, `read_frames`).
- Integrator's unelevated checks (not PresentMon evidence):
  - `run_scene.ps1` parses with 0 errors.
  - `Trace-Session` on a nonexistent name returns `missing`. On `PresentMon` while that session existed, it returned `unknown (logman exit -2147024891)`, the access-denied HRESULT, so an unelevated script refuses rather than proceeds.
  - `Stop-Capture` on a nonexistent name returns 7 at once.
  - The CSV check accepts a file ending in CRLF and refuses a truncated last row and an empty file.
- Private preliminary result of that capture: the gate interval complete, 142,212 frames, every present displayed. Not gate evidence.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | `--terminate_on_proc_exit` ends the capture after the probe exits | committed script | owner's preliminary capture | failed: PresentMon still running at 60 s; killed; last row truncated; session left running |
| 2 | an explicit named-session stop ends it cleanly | section 2 | unelevated helper checks only | pending the owner's next elevated capture |

## 6. Constraints and owned files
- Read-only review.
- Out of scope:
  - the probe C++ sources (unchanged since `4d5da1a`);
  - the gate tools;
  - performance numbers;
  - the Astra ruling items already adopted;
  - style;
  - `minor` and `nit` findings.
- Questions:
  1. Can the new sequence lose or reorder gate-interval rows? Can it admit a capture that the gate would not catch as `capture-not-covered` or `presentmon-rows-missing`?
  2. Can any exit path leave a trace session running, including:
     - a probe exit 3;
     - a probe start failure;
     - a PresentMon start failure or name clash;
     - a stop timeout;
     - a throw inside `finally`?
  3. Are the `logman` exit-code mapping and `$LASTEXITCODE` handling after `& cmd.exe /d /c ...` correct in Windows PowerShell 5.1 with `$ErrorActionPreference = 'Stop'`?
  4. Is `Stop-Capture`'s `ExitCode` reliable with the cached handle and `WaitForExit(30000)`?
  5. Is the refusal on a session named `PresentMon` a false-refusal risk for a normal run (for example from the Intel PresentMon service)? Is a bound or wait missing anywhere?

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
