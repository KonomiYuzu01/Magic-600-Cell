# Review packet: Windows verification batch (L2)

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: one Sol review (fast tier) of the Windows verification batch: `tools/windows_batch/`, its two test files and the CI list entry. Under the risk tiers of 2 October 2026 this is non-critical tooling.
- The owner's task, verbatim in substance:
  - Make Windows-only verification a one-command batch, so that the owner's Windows time is spent only on manual steps.
  - `tools/windows_batch/run_all.py` runs, in order, whichever of these exist:
    - the native host regression (`tests/native/NativeHostRegression.cs`);
    - the migration probes in `tools/migration_probes/`;
    - the S-B renderer gate captures from `work/experiments/renderer-sb/`.
  - Before each manual step, print one clear line and wait for Enter.
  - Stop heavy work before captures.
  - Write sanitised results to `work/windows-batch/<date>/`:
    - placeholders for paths and the user name;
    - only pass/fail, counts, digests and the minimal error text;
    - a one-page summary that a cloud session can read once it is committed.
  - Fresh synthetic data only. Modify no critical path. Install nothing.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. Report only `blocker` or `major` findings, each with a concrete counterexample: inputs or state, then the wrong outcome. Verdict `pass` if there is none.
- Wrong outcomes that count:
  - private text in a public file: a private path, the user, computer or domain name, a SID, or raw machine diagnostics can reach `work/windows-batch/<date>/batch.json` or `summary.md`. Those two files are the ones meant to be committed.
  - a false pass:
    - a step, or the batch's outcome or exit code, says `pass` although its check failed, did not run, was incomplete or could not be read;
    - a failure is reported as `skipped`.
  - wrong attribution: a step's result is taken from the wrong run, for example:
    - an older probe run directory;
    - another scene's summary;
    - another invocation's capture.
  - a manual-step violation:
    - a manual action is reached without a `>>>` line and an Enter wait;
    - a capture (scene or fault run) starts without the heavy-work guard;
    - a capture starts while the guard lists work and the owner did not type `skip`.
  - data scope:
    - a step reads or writes a personal session, or data other than fresh synthetic data and its own private and scratch directories;
    - anything is installed.
  - process and result handling:
    - an unbounded hang, except a wait for the owner's input;
    - a child process tree left running after a timeout or Ctrl+C;
    - completed results not written after an interruption.
  - a README statement that the code contradicts.

## 2. Actual problem and reproduction
- Flow of `run_all.main`:
  1. Parse arguments; exit 2 off Windows unless `--list`.
  2. Load the step modules named in `STEP_MODULES`; a module that fails to import gives an `error` step.
  3. Create the private root `work/loop-memory/windows-batch/<stamp>/` and the scratch root `<temp>/m6wb-<stamp>`.
  4. Print the plan, then one `>>>` start line.
  5. Run each step: an unavailable step is `skipped`; an exception gives `error`; Ctrl+C or EOF gives `interrupted` and skips the rest.
  6. Sanitise and publish `batch.json` and `summary.md`. A detected leak writes the raw record privately and exits 2.
  7. Remove the scratch root unless `--keep-scratch`. Exit 0 pass, 1 fail or error, 130 interrupted.
- Steps:
  - `native`: `native/bootstrap.py --self-test-only --data <scratch>/data`, with the engine Python if present. It judges `winforms-self-test.json`.
  - `migration`: `tools/.venv/engine/Scripts/python.exe tools/migration_probes/run_probes.py p3 pipeline p1 p2`. It finds the one new run directory under `work/migration-probes/runs/` and judges its `public.json`.
  - `renderer`:
    - It needs an elevated batch, PresentMon 2.6.0 at the pinned path, and `run_scene.ps1` plus `tools/perf/renderer_gate.py`.
    - It builds `sb_probe.exe` if it is missing. Then come a manual preparation line, the guard, and two questions (overlays, vendor mode).
    - Per scene: a manual line, the guard, then an interactive `run_scene.ps1 -Scene <s> -Runs <n>`. It judges the new `<stamp>-<scene>-summary/summary.json`.
    - Then a manual line, the guard, and four non-interactive 10-second W3 fault runs. Each is judged by `label_check.status` in its new `run.json`.
- Defaults: scenes W1, W2 and W4 (attribution only), 3 runs each, the four fault runs.
- Origin:
  - Interface `batch_interface.py` (Claude, commit fff8edd).
  - Orchestrator, guard, report, README, `tests/test_windows_batch.py` and the CI entry: Codex implement call `20261003T072342Z-746b2120` (valid; acceptance passed).
  - Step modules and `tests/test_windows_batch_steps.py`: Codex call `20261003T072402Z-94ad7918` (valid; acceptance passed).
  - Integrator changes after the calls:
    - `summary()` adds a status tally to the title line, and its test checks the exact title;
    - README wording on manual steps and the renderer defaults.

## 3. Environment and versions
- Windows 11 (10.0.26200), CPython 3.14.7 64-bit, Windows PowerShell 5.1, ANSI code page 1252, OEM code page 850.
- PowerShell execution policy RemoteSigned (CurrentUser), so `powershell -File` runs the checkout's scripts without `-ExecutionPolicy`.
- The review sandbox is read-only, with no GPU and no administrator rights. Evidence kind there: source and fixtures.

## 4. Necessary source and evidence
- Files under review:
  - `tools/windows_batch/run_all.py`, `batch_guard.py`, `batch_report.py`, `batch_interface.py`, `README.md`;
  - `tools/windows_batch/step_native.py`, `step_migration.py`, `step_renderer.py`;
  - `tests/test_windows_batch.py`, `tests/test_windows_batch_steps.py`;
  - the two added lines in `tools/ci/run_headless.py`.
- Contracts the steps rely on. Read them as contracts; they are not under review:
  - `native/bootstrap.py` lines 110 to 155: self-test mode, `build-info.json`, `NativeHostRegression.exe`, `winforms-self-test.json`;
  - `tools/migration_probes/run_probes.py`:
    - exactly one run root per invocation;
    - `public.json` is written before its leak check;
    - exit 2 for a leak or bad arguments; exit 0 even when probes fail;
  - `tools/migration_probes/sanitize.py` (`sanitize`, `leaks`, `leaks_in`; private words from `USERNAME`, `COMPUTERNAME`, `USERDOMAIN` and the home folder name) and `tools/repo_digest.py`.
- Renderer contracts on branch `claude/renderer-sb`, which is not merged into this checkout. Read them with `git show claude/renderer-sb:<path>` if needed:
  - `work/experiments/renderer-sb/probe/run_scene.ps1`:
    - the scene name is used as passed;
    - run directories are `work/loop-memory/perf/renderer/sb/<yyyyMMddTHHmmssfffZ>-<scene>-<i>`, and the summary is `<stamp>-<scene>-summary/summary.json`;
    - after each non-injection run it asks `Read-Host` for `yes`; injection runs ask nothing;
    - any failed run throws, which ends the invocation without a `summary.json`;
    - the probe exits 2 for an intentional label-check failure;
    - a global mutex refuses a second concurrent invocation.
  - `tools/perf/renderer_gate.py` `summarize()`:
    - `scenes` entries have `scene`, `gate_scene`, `runs` (valid runs only, each with `run_id` and `build_identity`), `pooled` (`n`, `fps`, `p99_ms`), `verdict` and `vram_peak_mb`;
    - `invalid_runs` entries have `scene` and `reasons`;
    - `unreadable` entries have only `reason`.
- Integrator's checks on the candidate:
  - `python tests/test_windows_batch.py`: 37 tests OK.
  - `python tests/test_windows_batch_steps.py`: 71 tests OK.
  - `python tools/ci/run_headless.py --list` lists both files after `tests/test_migration_p2.py`.
  - `run_all.py --list` on the owner's machine, non-elevated:
    - native: run;
    - migration: skipped (no engine environment in that worktree);
    - renderer: skipped (S-B files not in this checkout).
  - A real `run_all.py --only native`, non-elevated, on that machine: `pass`, 19 of 19 checks, exit 0. The published `batch.json` and `summary.md` contain no user name and no absolute path, and the scratch root was removed.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | a fixed interface lets the orchestrator and the steps be written in parallel | `batch_interface.py` (fff8edd) | both implement calls built against it | both valid |
| 2 | the WB-1 packet as written | Codex call `20261003T072342Z-746b2120` | its acceptance check | 37 tests pass |
| 3 | the WB-2 packet as written | Codex call `20261003T072402Z-94ad7918` | its acceptance check | 71 tests pass |
| 4 | the two parts integrate | patches applied unchanged, plus the integrator changes in section 2 | section 4 checks | all pass |

## 6. Constraints and owned files
- Read-only review.
- Out of scope:
  - the contract files in section 4, except where the batch misreads them;
  - design choices, unless they cause a wrong outcome from section 1:
    - flat step modules;
    - the process-list guard through `Get-CimInstance`;
    - the private and public split;
    - the default scenes and timeouts;
    - no automatic elevation;
  - decoding of non-UTF-8 child output, beyond correctness of the judgement;
  - style, `minor` and `nit` findings.
- Questions:
  1. Can private text reach `batch.json` or `summary.md`? Cases:
     - reasons clipped before sanitising;
     - `details` copied from `public.json` or `summary.json`;
     - the owner's free-text answers;
     - exception texts;
     - the source-identity error.
  2. Can any step report `pass` without a complete, readable result of its own invocation? Cases:
     - run-directory and summary identification;
     - an operator who answers anything but `yes`;
     - a script that throws midway;
     - a guard skip after some captures;
     - the status precedence in `step_renderer`.
  3. Is every manual action preceded by a `>>>` line and an Enter wait, and every capture by the guard? Can the guard miss any heavy work it lists, or be bypassed?
  4. On a timeout, Ctrl+C or EOF:
     - are child trees ended;
     - are the results still published;
     - is the scratch root removed;
     - are the exit codes right?
  5. Does the README contradict the code?

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
