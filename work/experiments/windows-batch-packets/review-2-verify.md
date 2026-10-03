# Scoped verification packet: Windows verification batch (L2), round 1

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: a scoped verification round (Sol, fast tier) of the fixes for the five `major` findings of review `20261003T074248Z-22b99cad` on the Windows verification batch. Under the risk tiers of 2 October 2026 this is non-critical tooling.
- Check only two things:
  1. Is each finding, WB-L2-01 to WB-L2-05, fixed in the current candidate?
  2. Do the fixes introduce a new `blocker` or `major` in the code they changed? The wrong outcomes that count are the full review's:
     - private text in `work/windows-batch/<date>/batch.json` or `summary.md`;
     - a false pass, or a failure reported as `skipped`;
     - a result taken from the wrong run;
     - a manual action without a `>>>` line and an Enter wait, or a capture without the heavy-work guard;
     - data outside fresh synthetic data and the batch's own directories, or an installation;
     - an unbounded hang, a child process tree left running, or completed results not written after an interruption;
     - a README statement that the code contradicts.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. Report only `blocker` or `major` findings, each with a concrete counterexample: inputs or state, then the wrong outcome. Verdict `pass` if there is none.
- Out of scope: code the full review did not flag and the fixes did not change; `minor` and `nit` findings; the design choices in section 6.

## 2. Actual problem and reproduction
The five findings, each with its fix:
- WB-L2-01, failed or timed-out fault captures could pass.
  - Fix: `_judge_fault` in `tools/windows_batch/step_renderer.py`. It reads `run.json` of the one new `<stamp>-w3-1` directory:
    - `injection_applied` must be `true`; otherwise error;
    - label status `pass`: failure ("label check passed"), whatever happened to the capture afterwards;
    - any status other than `fail`: error;
    - script timed out or exited non-zero: error ("capture incomplete");
    - the same invocation's `<stamp>-w3-summary/summary.json` must exist; otherwise error;
    - the run among the summary's valid scene runs: failure ("the gate accepted the run");
    - the run in `invalid_runs` with `label-check-failed`: caught;
    - otherwise: error.
  - A new detail field records `gate_refused`. The summary line reads "Faults: N of M caught by the label check and refused by the gate".
- WB-L2-02, later fault captures started without a guard.
  - Fix: `ctx.require_quiet()` runs before each of the four fault runs. A skip stops the remaining runs: the step is an error if any capture started, otherwise skipped.
- WB-L2-03, Ctrl+C between steps lost completed results and the cleanup.
  - Fix in `run_all.main`:
    - `add()` normalises a result and appends it with SIGINT ignored.
    - One `try` covers source identity, the plan, the start prompt and the step loop, and catches `KeyboardInterrupt`.
    - Then, with SIGINT ignored until `main` returns:
      - the first step without a result becomes `interrupted` ("interrupted before the batch started" or "interrupted between steps");
      - the rest become `skipped` ("not run: the batch was interrupted");
      - the outcome is `interrupted` whenever an interrupt was caught;
      - publication and the scratch cleanup run;
      - the exit code is 130.
  - `_ignore_interrupts()` sets `SIG_IGN` only when the current handler is known to Python, restores the previous handler, and does nothing outside the main thread.
- WB-L2-04, clipping before sanitising left name fragments.
  - Fix: `batch_interface.clip()` sanitises before it normalises whitespace and cuts. Roots: `<repo>` (the checkout) and `<home>`, plus the sanitiser's drive-path, UNC, file-URI, SID and private-word rules.
  - It has no temporary-directory root: `tempfile.gettempdir()` can write a probe file, and `--list` must write nothing. Temp paths therefore become `<home>/...` or `<path>`.
  - `batch_report._sanitizer` is the same module object: `batch_interface.sanitizer()` loads it by path once, under `sys.modules["_windows_batch_sanitize"]`.
- WB-L2-05, free-text declarations could publish raw diagnostics.
  - Fix: both answers are written raw to `<private>/renderer/declared.json`.
  - The public `details.declared` keeps an answer only when it equals the documented default; otherwise it reads "custom answer (private record)".
  - The raw answers still go to `run_scene.ps1` (`-Overlays`, `-Declare vendor_mode=...`). Its run records stay under `work/loop-memory/`, and the batch does not copy declarations from them.

## 3. Environment and versions
- Windows 11 (10.0.26200), CPython 3.14.7 64-bit, Windows PowerShell 5.1, ANSI code page 1252, OEM code page 850.
- The review sandbox is read-only, with no GPU and no administrator rights. Evidence kind there: source and fixtures.

## 4. Necessary source and evidence
- Files changed by the fixes:
  - `tools/windows_batch/step_renderer.py`: `_judge_fault`, the fault loop, the declarations;
  - `tools/windows_batch/run_all.py`: `_ignore_interrupts`, `main`;
  - `tools/windows_batch/batch_interface.py`: `sanitizer`, `clip`;
  - `tools/windows_batch/batch_report.py`: the sanitiser import;
  - `tools/windows_batch/README.md`;
  - `tests/test_windows_batch.py`, `tests/test_windows_batch_steps.py`.
- Contracts on branch `claude/renderer-sb` (read with `git show claude/renderer-sb:<path>`):
  - `work/experiments/renderer-sb/probe/run_scene.ps1`:
    - it creates `<stamp>-<scene>-summary/` before its runs, and run IDs are `<stamp>-<scene>-<i>`;
    - the probe exits 2 for an intentional label-check failure, and the script accepts that;
    - any other failure throws before `summary.json` is written.
  - `tools/perf/renderer_gate.py` `summarize()`:
    - every readable run is listed either among `scenes[].runs` (valid) or in `invalid_runs` (`run_id`, `scene`, `reasons`);
    - W3 is a label scene, so a run whose label check is not `pass` gets `label-check-failed`;
    - the gate exits 0 unless every run is unreadable.
- Integrator's checks on the current candidate:
  - `python tests/test_windows_batch.py`: 42 tests OK. The five new tests:
    - Ctrl+C after a finished step;
    - Ctrl+C after the last step;
    - Ctrl+C during source identity;
    - SIGINT ignored during publication and restored afterwards;
    - user, computer and domain names across the 200- and 300-character cuts.
  - `python tests/test_windows_batch_steps.py`: 80 tests OK.
    - The test that asserted `pass` after a timeout is replaced.
    - New tests cover timeout, script exit 1, a missed fault with a failed script, injection not applied, no gate summary, refusal for other reasons only, gate acceptance, the guard before every fault run, published defaults and private free text.
  - `python tools/ci/run_headless.py --list` lists both files after `tests/test_migration_p2.py`.
  - `python tools/windows_batch/run_all.py --list` on the owner's machine: plan only, exit 0.
  - A real `run_all.py --only native` after the fixes, non-elevated:
    - `pass`, 19 of 19 checks, exit 0;
    - no leak in either public file;
    - the scratch root was removed.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | the candidate meets the task | full Sol review `20261003T074248Z-22b99cad` | five `major` findings | all adopted |
| 2 | the fixes in section 2 close them | this candidate | section 4 checks | all pass |

## 6. Constraints and owned files
- Read-only review.
- Out of scope, unless they cause a wrong outcome from section 1:
  - the full review's design choices: flat step modules, the `Get-CimInstance` guard, the private and public split, the default scenes and timeouts, no automatic elevation;
  - publishing only the documented default declarations;
  - no temporary-directory root in `clip()`;
  - one guard per scene invocation (a scene's runs run inside one `run_scene.ps1` call);
  - a Ctrl+C that arrives while SIGINT is ignored is dropped; the owner presses it again;
  - `minor` and `nit` findings.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
