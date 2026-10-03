# Packet WB-2: Windows batch steps

## 1. Goal and acceptance
- Goal: the three step modules of the Windows batch, each exposing `STEP`:
  - `tools/windows_batch/step_native.py`: the native host self-test;
  - `tools/windows_batch/step_migration.py`: the migration probes;
  - `tools/windows_batch/step_renderer.py`: the S-B renderer captures.
- They are written against the committed contract `tools/windows_batch/batch_interface.py`.
- Packet WB-1 writes the orchestrator `run_all.py` in parallel: console, heavy-work guard, directories, sanitising, results.
- Acceptance check: `python tests/test_windows_batch_steps.py` exits 0.
- Non-goals:
  - `run_all.py`, the guard, sanitising and the written results;
  - any change to a check that a step starts;
  - elevation prompts.

## 2. Actual problem and reproduction
- Today the owner starts each Windows-only check by hand and waits through each one. The batch starts once and stops only where a person must act.
- Owner requirements (verbatim):
  - "Before each manual step (start 0.4, wait, close), print one clear line and wait for Enter."
  - "Stop heavy work before captures."
  - "keep only pass/fail, counts, digests and the minimal error text."
  - "Fresh synthetic data only. Modify no critical path. Install nothing outside the allowlist."

## 3. Environment and versions
- Base: branch `claude/windows-batch` (this worktree's HEAD), which contains `tools/windows_batch/batch_interface.py`.
- Owner's machine:
  - Windows 11 and an RTX 4070 Laptop GPU;
  - the engine environment `tools/.venv/engine` (CPython 3.14.7, NumPy 2.3.5);
  - Visual Studio 2026 with Windows SDK 10.0.26100.0;
  - PresentMon 2.6.0 at the pinned path.
- The S-B capture script `work/experiments/renderer-sb/probe/run_scene.ps1` lives on branch `claude/renderer-sb` and is not in this worktree.
  - The renderer step must report itself unavailable when the script is missing.
  - Its interface is quoted in section 4.
- Evidence kind in the sandbox: source and fixture only.

## 4. Necessary source and evidence

### Native host self-test
- Command: `native/bootstrap.py --self-test-only --data <dir>`.
  - It compiles the host and `tests/native/NativeHostRegression.cs` with the .NET Framework 4.x x86 compiler (`%WINDIR%\Microsoft.NET\Framework\v4.0.30319\csc.exe`).
  - It runs the regression, which shows and closes real WinForms windows. Bootstrap's own time limit for the regression is 120 s.
  - It writes, in `<data>/native-host-0.3/diagnostics/`: `winforms-self-test.json`, `winforms-self-test.log`, `build-info.json` (with `source_sha256` of the host sources) and `NativeHostRegression.exe`.
  - It exits 0 on pass. On failure it exits 1 with a message on stderr.
  - It needs 64-bit Python, and refuses to compile while the `LIB` environment variable is set.
  - Without `--data` it writes to the personal `%LOCALAPPDATA%\C600Studio`. Always pass a fresh `--data` below `ctx.scratch`.
  - Paths near 260 characters break the compile (CS1619).
- Report keys: `passed` (bool), `scope`, `os`, `clr`, `process_bits`, `original_splitter_exception`, `checks` (a list of `{name, passed}`), `error` (a stack trace on failure) and `utc`. Read the report as `utf-8-sig`.

### Migration probes
- Command, from the repository root: `tools/.venv/engine/Scripts/python.exe tools/migration_probes/run_probes.py p3 pipeline p1 p2`.
  - The probes start and stop 0.4 themselves (`EngineProcess`) on synthetic sessions.
  - Each run uses a fresh run root `work/migration-probes/runs/<UTC stamp YYYYMMDDTHHMMSSZ>/` holding `raw.json` (private) and `public.json` (already sanitised).
  - Never pass `--out`: it would rewrite committed evidence.
- Exit codes:
  - 0, even when a probe fails: the pass flags are only in the JSON;
  - 2: the sanitised result still held private text, so no public result was written, or the arguments were bad;
  - 1: an exception.
- `public.json` is `{"run", "environment": {...}, "probes": {...}, "commands": {...}}`. Judge each probe:
  - `p3` and `pipeline`: a mapping from case to result. A case passes when `result["passed"] is True`.
  - `p1`: `{"strategies", "verdicts": {strategy: {"verdict", "attempts", "passed_attempts", "inconclusive_attempts"}}, "chosen", "controls": {name: {..., "passed": True | False | None}}, "attempts": [...]}`.
    - P1 passes when `verdicts["C3"]["verdict"] == "pass"` and no control has `passed is False`.
    - C3 is the strategy the owner chose on 3 October 2026 (`docs/wiki/decisions/owner-decisions-2026-10-03-migration.md`).
    - `None` marks the recorded detection limit of the unguarded mapped-write control.
    - Report `chosen` as well.
  - `p2`: `{"passed": bool, "qualification": {...}, "interruptions": [{"passed": bool, ...}], "evidence"}`. P2 passes when `passed is True`.
- The committed reference result `docs/progress/1.0/migration-probe-results.json` has the same probe results under `probes.<name>.result`. Build the test fixtures from it.

### S-B renderer captures
`work/experiments/renderer-sb/probe/run_scene.ps1` (on `claude/renderer-sb`). Its parameters:
```
param(
    [Parameter(Mandatory=$true)][ValidateSet('w1','w2','w3','w4')][string]$Scene,
    [ValidateRange(1,100)][int]$Runs = 3,
    [ValidateSet('corrupt-label','swap-same-colour','delay-adoption','stale-binding')][string]$Inject,
    [string[]]$Declare = @(),
    [string]$Overlays,
    [ValidateRange(1,86400)][double]$Duration = 192,
    [ValidateRange(0,60)][double]$Preroll = 4
)
```
- Requirements:
  - it throws in an unelevated terminal;
  - it needs `probe/build/sb_probe.exe`, built by `probe/build.cmd` with no arguments (which needs Visual Studio);
  - it needs PresentMon at `%ProgramFiles%\Intel\PresentMon\PresentMonConsoleApplication\PresentMon-2.6.0-x64.exe`;
  - `-Overlays` is required unless `-Inject` is given.
- Outputs of each invocation, with `<stamp>` as `yyyyMMddTHHmmssfffZ` (UTC):
  - run directories `work/loop-memory/perf/renderer/sb/<stamp>-<scene>-<i>/` (with `run.json`, `presentmon.csv` and a trace);
  - `<stamp>-<scene>-summary/summary.json`.
- Behaviour:
  - After each run without `-Inject`, it asks on the console: "did you watch the whole run, with nothing covering any part of the probe window? Type yes to keep it". It throws unless the answer is yes.
  - It waits 20 s between runs.
  - At the end it runs `python tools/perf/renderer_gate.py <runs> --out <summary>`, so `python` must be on `PATH`. `renderer_gate.py` uses the standard library only.
  - It throws (exit 1) on any refusal.
- `summary.json` (from `renderer_gate.py`):
  ```
  {"scenes": [{"scene", "gate_scene", "verdict",
               "pooled": {"n", "fps", "p99_ms", "max_ms", "met"}, "vram_peak_mb",
               "runs": [{"run_id", "build_identity", "fps", "p99_ms", "vram_peak_mb", "met", ...}]}],
   "invalid_runs": [{"run_id", "scene", "reasons"}], "unreadable": [{"reason"}], ...}
  ```
- Verdicts:
  - W3 is the gate scene: `met`, `not-met`, `insufficient-runs` (fewer than 3 valid runs) or `mixed-builds`;
  - W1, W2 and W4 get `attribution` when they have valid runs;
  - a scene without valid runs stays `no-data`.
- Injected-fault runs use `-Scene w3 -Runs 1 -Duration 10 -Inject <fault>` for each of the four faults `corrupt-label`, `swap-same-colour`, `delay-adoption` and `stale-binding`.
  - The probe exits 2 with a failed label check. The gate then refuses the run, so the script may exit 0 or 1.
  - Judge a fault run only from its `run.json`: `label_check.status == "fail"` means the fault was caught.
  - `label_check` also has `mismatches`, `late_adoptions`, `binding_mismatches`, `missing`, `revisions` and `copies`; `injection_applied` is a top-level key.
- The W3 gate runs of 3 October 2026 used `-Overlays 'none running'` and `-Declare 'vendor_mode=NVIDIA discrete GPU (MUX), high performance'`.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| - | none yet | - | - | - |

## 6. Constraints and owned files

Owned files:
- `tools/windows_batch/step_native.py`
- `tools/windows_batch/step_migration.py`
- `tools/windows_batch/step_renderer.py`
- `tests/test_windows_batch_steps.py`

Change nothing else. Follow the design below. Where it is silent, choose the simplest option and name it in the final message.

### Common rules
- Each module defines a class and `STEP = <instance>`. The names are `native`, `migration` and `renderer`.
- Import the contract by putting the module's own directory on `sys.path` (`import batch_interface as wb`), as `run_all.py` loads the steps from that directory.
- Reach the console and processes only through the Context:
  - start processes only with `ctx.run`, never with `subprocess` directly;
  - read files directly.
- `unavailable()` only reads: no prompt, process or write.
- Reasons are the minimal error text, passed through `wb.clip`, for example the last non-empty output line. `run_all.py` sanitises paths and names, so a step need not.
- Never pass a personal data directory, never pass `--out` to `run_probes.py`, and never open a database.
- Interpreter:
  - use `<repo>/tools/.venv/engine/Scripts/python.exe` when it exists;
  - the native step falls back to `sys.executable`;
  - the migration step needs the engine environment (0.4 needs NumPy) and is unavailable without it.
- Read the platform through one module-level name per module, for example `IS_WINDOWS = os.name == "nt"`, so that tests can patch it and run on any OS.
- LF line endings, ASCII source, standard library only, no network, nothing installed.

### step_native (title "Native host self-test (tests/native/NativeHostRegression.cs)")
- `unavailable`:
  - not Windows: "Windows only";
  - `native/bootstrap.py` or `tests/native/NativeHostRegression.cs` missing: "native host regression not in this checkout";
  - `csc.exe` missing (below `WINDIR`): ".NET Framework 4.x compiler not found";
  - a 32-bit fallback interpreter: "needs 64-bit Python".
- `run`:
  1. Call `ctx.manual("Native self-test: it compiles the host, then opens and closes test windows by itself for up to two minutes; do not click or type until it reports.")`.
  2. Set `data = ctx.scratch / "data"`. It does not exist yet; bootstrap creates it.
  3. Build `env`: a copy of `os.environ` without `LIB` (in any case), with `PYTHONUTF8=1`.
  4. Run `ctx.run([python, str(repo / "native/bootstrap.py"), "--self-test-only", "--data", str(data)], timeout=900, env=env)`.
  5. Copy the diagnostics directory's `*.json` and `*.log` files to `ctx.private` (not the executables).
  6. The result:
     - timed out: ERROR, "native self-test timed out after 900 s";
     - no report: ERROR, with the last non-empty output line (for example the compile error) or "no self-test report";
     - the report's `passed` is true and the exit code is 0: PASS;
     - otherwise FAIL, with the first line of the report's `error`, or the failed check names, or the last output line.
- `counts`: `checks`, `checks_passed`, `checks_failed`.
- `digests`:
  - `NativeHostRegression.cs`;
  - `host_sources` (from `build-info.json` `source_sha256`, when it is 64 hex characters);
  - `NativeHostRegression.exe`, when present;
  - `report`.
- `details`: `scope`, `clr`, `process_bits`, `failed_checks` (names), `returncode`.
- `lines`, for example: `WinForms self-test: 23 of 23 checks passed (CLR 4.0.30319.42000, 32-bit host)`.

### step_migration (title "Migration probes P3, pipeline, P1 and P2 (tools/migration_probes)")
- `unavailable`:
  - not Windows: "Windows only";
  - `tools/migration_probes/run_probes.py` missing: "migration probes not in this checkout";
  - the engine interpreter missing: "engine environment tools/.venv/engine is missing (docs/DEVELOPMENT.md)".
- `run`:
  1. Call `ctx.say("Migration probes: they start and stop 0.4 by themselves on synthetic sessions; do not start or close C600 Studio until they finish.")`. There is no manual step, so no Enter.
  2. List the directory names in `<repo>/work/migration-probes/runs` (none when the directory is missing).
  3. Run `ctx.run([engine_python, str(repo / "tools/migration_probes/run_probes.py"), "p3", "pipeline", "p1", "p2"], timeout=7200, cwd=repo, env=<os.environ plus PYTHONUTF8=1>)`.
  4. The result:
     - timed out: ERROR;
     - exit 2: ERROR, "run_probes.py exited 2: no public result (private text after sanitising, or bad arguments)";
     - any other nonzero exit: ERROR, with the last output line.
  5. Exactly one new directory is the run. Otherwise: ERROR, "could not identify the probe run directory (<n> new)".
  6. A missing or unreadable `public.json`, or a probe missing from it: ERROR.
  7. Judge each probe as in section 4. All four pass: PASS. Otherwise FAIL, with the failing probes and cases as the clipped reason.
- `counts`: `p3_cases`, `p3_passed`, `pipeline_cases`, `pipeline_passed`, `p1_c3_attempts`, `p1_c3_passed_attempts`, `p1_controls`, `p1_controls_failed`, `p2_interruptions`, `p2_interruptions_passed`.
- `digests`:
  - `public_json`;
  - `probe_sources`: SHA-256 of the lines `<file name>\0<sha256>\n` over the sorted `tools/migration_probes/*.py`.
- `details`:
  - `run` (the directory name) and `environment` (from `public.json`);
  - `cases`: probe, then case, then bool;
  - `p1_verdicts`: strategy, then verdict;
  - `p1_chosen`, `failed_controls`, `returncode`, `seconds`.
- `lines`, for example:
  - `P3 and F20: 6 of 6 cases pass`
  - `Pipeline fixtures: 6 of 6 pass`
  - `P1: C3 passes 12 of 12 attempts; first passing strategy C3; no control failed`
  - `P2: pass; 36 of 36 interruptions pass`
  - `Run 20261003T005400Z took 14 min`

### step_renderer (title "S-B renderer captures (work/experiments/renderer-sb)")
- Paths:
  - `probe_dir = repo / "work/experiments/renderer-sb/probe"`;
  - `script = probe_dir / "run_scene.ps1"`;
  - `build_cmd = probe_dir / "build.cmd"`;
  - `exe = probe_dir / "build/sb_probe.exe"`;
  - `gate = repo / "tools/perf/renderer_gate.py"`;
  - `captures = repo / "work/loop-memory/perf/renderer/sb"`;
  - `presentmon = Path(os.environ.get("ProgramFiles", r"C:\Program Files")) / "Intel/PresentMon/PresentMonConsoleApplication/PresentMon-2.6.0-x64.exe"`;
  - `powershell = Path(os.environ.get("SystemRoot", r"C:\Windows")) / "System32/WindowsPowerShell/v1.0/powershell.exe"`.
- `unavailable`:
  - not Windows: "Windows only";
  - the script or the gate missing: "S-B capture script not in this checkout";
  - the exe and `build.cmd` both missing: "S-B probe build files missing";
  - not `ctx.elevated`: "needs an administrator PowerShell (PresentMon)";
  - PresentMon missing: "PresentMon 2.6.0 not found at the pinned location";
  - no scenes and no faults requested: "no captures requested".
- `run`:
  1. When the exe is missing or `options["rebuild_probe"]` is set:
     - `ctx.say(...)`;
     - `ctx.run(["cmd.exe", "/d", "/c", str(build_cmd)], cwd=probe_dir, timeout=1800)`;
     - a nonzero exit or a missing exe: ERROR, "probe build failed: <last output line>".
  2. Call `ctx.manual("Renderer captures: mains power plugged in, NVIDIA GPU in high-performance mode, display on with no sleep or screen saver, other applications and overlays closed, nothing over the probe window; each run needs your yes after you watched it.")`.
  3. Call `ctx.require_quiet()`. If it returns a reason: SKIPPED with that reason.
  4. Ask for the declarations:
     - `overlays = ctx.ask("Overlays running during the captures", "none running")`;
     - `vendor = ctx.ask("GPU and performance mode set in the vendor software", "NVIDIA discrete GPU (MUX), high performance")`.
  5. Build `env`: a copy of `os.environ` whose `PATH` starts with `str(Path(sys.executable).parent)`, so that `python` in the script runs this interpreter.
  6. For each scene in `options["scenes"]`:
     - call `ctx.manual(f"Next: {scene.upper()} captures, {runs} runs of about 3.5 minutes with 20 s pauses; watch each run with nothing covering the probe window and answer yes after it.")`;
     - call `ctx.require_quiet()`. A reason skips this scene and every later capture, faults included;
     - list the names in `captures`;
     - run `ctx.run([str(powershell), "-NoProfile", "-File", str(script), "-Scene", scene, "-Runs", str(runs), "-Overlays", overlays, "-Declare", f"vendor_mode={vendor}"], timeout=runs * 600 + 600, cwd=repo, env=env, interactive=True)`. Do not pass `-ExecutionPolicy`;
     - the new directories identify the summary `<stamp>-<scene>-summary` and the runs `<stamp>-<scene>-<i>`;
     - the capture is complete when all of these hold:
       - the exit code is 0;
       - `summary.json` has the scene;
       - its `runs` count equals `runs`;
       - no invalid or unreadable entry belongs to this scene;
       - the verdict is `met` or `not-met` for a gate scene, or `attribution` for any other scene.
  7. When `options["faults"]` is set:
     - call `ctx.manual("Next: four 10-second W3 fault runs; they need no answers.")`, then `ctx.require_quiet()` (a reason skips the faults);
     - for each fault, list the names in `captures`, then run `ctx.run([str(powershell), "-NoProfile", "-File", str(script), "-Scene", "w3", "-Runs", "1", "-Duration", "10", "-Inject", fault], timeout=900, cwd=repo, env=env)`. This run is not interactive;
     - the new `<stamp>-w3-1/run.json` decides: `label_check.status == "fail"` means the fault was caught.
  8. The status:
     - FAIL when a fault ran and its label check passed, or a gate scene's verdict is `not-met`. FAIL takes precedence over ERROR, and the reason names both;
     - otherwise ERROR when the probe build failed, a scene that ran has no complete capture, a fault run left no `run.json`, or the owner skipped some of the captures after others ran;
     - SKIPPED when the owner skipped before any capture ran;
     - otherwise PASS.
- `counts`: `scenes_requested`, `scenes_complete`, `valid_runs`, `faults_run`, `faults_caught`.
- `digests`:
  - `sb_probe_exe`, `run_scene_ps1` and `renderer_gate_py`;
  - `summary_<scene>` for each `summary.json`;
  - `build_identity`, when every valid run shares one 64-hex identity.
- `details`:
  - `declared`: `{overlays, vendor_mode}`;
  - per scene: exit code, verdict, valid runs, invalid run reasons, pooled `n`, `fps` and `p99_ms`, `vram_peak_mb`, build identities and run ids;
  - per fault: exit code, label-check status and counts, `injection_applied`.
- `lines`, for example:
  - `W1: attribution; 3 valid runs of build 2b5bf5e6; pooled 812.40 fps, p99 1.432 ms; peak VRAM 81.7 MB`
  - `W2: no complete capture (script exit 1; 2 of 3 runs valid)`
  - `Faults: 4 of 4 caught by the label check`

### Tests (`tests/test_windows_batch_steps.py`)
Use unittest and the standard library. The tests use no real check, console, PowerShell, elevation or network, and write nothing inside the repository.
- Each test builds a fake repository below `tempfile.gettempdir()` with plain `Path.mkdir()`, never `tempfile.mkdtemp` or `TemporaryDirectory` (they fail with WinError 5 in the Codex sandbox), and removes it afterwards.
- A fake context records `say`, `manual` and `ask` calls, answers from a script, and serves `run` from handlers. The handlers create the files the real check would create.

Cover at least:
- native:
  - each unavailable reason (patch the platform name and `WINDIR`);
  - argv with `--self-test-only` and a `--data` below `ctx.scratch`;
  - no `LIB` in `env`;
  - `manual` is called once, before `run`;
  - PASS, FAIL with failed checks, ERROR on a time-out, and ERROR on a missing report with the compile-error line as the reason.
- migration:
  - unavailable without the engine interpreter;
  - argv is exactly the four probes in order, without `--out`;
  - PASS from a `public.json` built from `docs/progress/1.0/migration-probe-results.json`;
  - FAIL when a P3 case fails, when C3 fails, and when a control is False; PASS when a control is None;
  - ERROR on exit 2, on no new run directory and on a missing probe;
  - `say` is called and `manual` is not.
- renderer:
  - unavailable without the script, when not elevated, and without PresentMon (patch `ProgramFiles`);
  - the build runs when the exe is missing, and a failed build gives ERROR;
  - SKIPPED when `require_quiet` returns a reason;
  - the scene argv, `interactive=True`, `PATH` starting with the directory of `sys.executable`, and no `-ExecutionPolicy`;
  - a complete attribution scene gives PASS; a gate scene `not-met` gives FAIL; an invalid run gives ERROR; a missing summary gives ERROR;
  - all faults caught gives PASS; one fault whose label check passed gives FAIL; a missing `run.json` gives ERROR;
  - `manual` comes before the preparation, before each scene and before the faults.

```implement-contract
{"allowed_files": ["tools/windows_batch/step_native.py", "tools/windows_batch/step_migration.py", "tools/windows_batch/step_renderer.py", "tests/test_windows_batch_steps.py"], "acceptance_check": ["python", "tests/test_windows_batch_steps.py"], "stop_condition": "the acceptance check passes and the three step modules implement unavailable() and run() as designed above, each exposing STEP"}
```

## 7. Required return format
- Implementation: change only the assigned worktree.
- The final message lists:
  - the changed files;
  - the acceptance result;
  - the choices you made where this packet is silent;
  - what the sandbox could not verify;
  - open points.
