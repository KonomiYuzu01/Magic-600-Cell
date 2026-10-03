# Packet WB-1: Windows batch orchestrator

## 1. Goal and acceptance
- Goal: `tools/windows_batch/run_all.py` runs, in order, whichever Windows-only checks exist in this checkout, so that the owner's Windows time is spent only on manual steps. The checks are the native host self-test, the migration probes and the S-B renderer captures.
- Packet WB-2 writes the three step modules (`step_native.py`, `step_migration.py`, `step_renderer.py`) in parallel, against the committed contract `tools/windows_batch/batch_interface.py`.
- This packet owns the console, the heavy-work guard, the directories, the sanitised results and the one-page summary.
- Acceptance check: `python tests/test_windows_batch.py` exits 0.
- Non-goals:
  - the step modules;
  - running any real check;
  - PresentMon;
  - elevation (UAC) prompts;
  - committing results.

## 2. Actual problem and reproduction
- Today each Windows-only check is started by hand with its own command, and the owner waits through each one. The batch should start once and stop only where a person must act.
- Owner requirements (verbatim):
  - "Before each manual step (start 0.4, wait, close), print one clear line and wait for Enter."
  - "Stop heavy work before captures."
  - "Write sanitised results to work/windows-batch/<date>/: placeholders for paths and the user name; keep only pass/fail, counts, digests and the minimal error text."
  - "Write a one-page summary that a cloud session can read once it is committed."
  - "Fresh synthetic data only. Modify no critical path. Install nothing outside the allowlist."

## 3. Environment and versions
- Base: branch `claude/windows-batch` (this worktree's HEAD), which contains `tools/windows_batch/batch_interface.py`.
- Owner's machine: Windows 11 and 64-bit CPython 3.14. The batch uses the standard library only.
- CI runs `python tools/ci/run_headless.py` on GitHub `windows-2025`.
- Evidence kind in the sandbox: source and fixture only.

## 4. Necessary source and evidence
- `tools/windows_batch/batch_interface.py` is the contract: `Context`, `Step`, `StepResult`, `Completed`, statuses, exit codes, `DEFAULT_OPTIONS`, `STEP_MODULES`, `clip`, `sha256_file`. Do not change it. If it blocks you, say so in the final message.
- `tools/migration_probes/sanitize.py`: `sanitize(value, roots)`, `leaks(text)`, `leaks_in(value)`.
  - Reuse it by loading the file by path with `importlib.util.spec_from_file_location` under a private module name.
  - Do not copy it, and do not put `tools/migration_probes` on `sys.path`.
  - Its private words come from `USERNAME`, `COMPUTERNAME`, `USERDOMAIN` and the home directory name.
- `tools/repo_digest.py`: `source_identity(timeout)` returns `{"head", "paths", "digest"}` for the checkout it lives in.
  - Load it by path too.
  - It is a critical path: do not change it.
- `tools/ci/run_headless.py`: the `CHECKS` list.
- A known sandbox limit: in the Codex sandbox, `tempfile.mkdtemp` and `tempfile.TemporaryDirectory` fail with WinError 5. Python 3.13 and later give a mode-0o700 directory on Windows an ACL that the sandbox cannot use.
  - Create every temporary directory, in the batch and in the tests, with plain `Path.mkdir()` and the default mode, below `tempfile.gettempdir()`.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| - | none yet | - | - | - |

## 6. Constraints and owned files

Owned files:
- `tools/windows_batch/run_all.py`
- `tools/windows_batch/batch_guard.py`
- `tools/windows_batch/batch_report.py`
- `tools/windows_batch/README.md`
- `tests/test_windows_batch.py`
- `tools/ci/run_headless.py`, only to add `["tests/test_windows_batch.py"]` and `["tests/test_windows_batch_steps.py"]` to `CHECKS`, right after `["tests/test_migration_p2.py"]`.

Change nothing else. Follow the design below. Where it is silent, choose the simplest option and name it in the final message.

### Command line
`python tools/windows_batch/run_all.py [--only NAMES] [--scenes LIST] [--runs N] [--no-faults] [--rebuild-probe] [--keep-scratch] [--list]`

- `--only`: comma-separated step names (`native`, `migration`, `renderer`, the `Step.name` values).
  - Steps still run in `STEP_MODULES` order.
  - An unknown name exits 2.
- `--scenes`: comma-separated scene names from `w1` to `w4`, without repeats, or `none` for no scene capture.
- `--runs`: an integer from 1 to 10.
- `--no-faults` sets `options["faults"]` to False; `--rebuild-probe` sets `options["rebuild_probe"]` to True.
- `--keep-scratch`: keep the scratch root.
- The options are `DEFAULT_OPTIONS` overridden by these arguments; `options["scenes"]` is a tuple.
- `--list` prints the plan and exits 0. It works on any OS and writes, prompts and starts nothing.
- Make the entry testable, for example `main(argv=None, *, repo=None, steps=None, input_fn=input, out=sys.stdout, clock=None) -> int`, so that tests can pass a temporary repository root, fake steps, scripted input and a fixed clock.

### Run flow
1. Not Windows, without `--list`: print one line and exit 2.
2. Load the steps.
   - Import each `STEP_MODULES` name from the batch's own directory and take `STEP`.
   - A module that cannot be imported gives an ERROR result for that step, with the clipped import error as its reason. The other steps still run.
3. Set up the directories.
   - Stamp: `YYYYMMDDTHHMMSSZ` in UTC.
   - Private root: `<repo>/work/loop-memory/windows-batch/<stamp>/`.
   - Scratch root: `<tempdir>/m6wb-<stamp>` (plain mkdir; when it exists, add `-<pid>`).
   - Each step that runs gets `<private root>/<name>/` and `<scratch root>/<name>/`, created just before it runs.
4. Print the plan:
   - whether the batch runs elevated;
   - the commit (`head[:12]`) and the number of changed paths;
   - for each selected step, `run`, or `skip: <reason>` from `unavailable(ctx)`.

   Then call `manual(...)` once, telling the owner to keep the window open and that the batch asks before every manual step. Starting the batch is the first manual step.
5. Run each selected step in order.
   - An unavailable step is SKIPPED with its reason and gets no directories.
   - KeyboardInterrupt makes this step INTERRUPTED and every later step SKIPPED with the reason "not run: the batch was interrupted". The loop stops; the results are still written.
   - Any other exception makes the step ERROR, with `clip("<type>: <message>")` as the reason, and the loop continues.
   - Normalise each result:
     - an unknown status becomes ERROR;
     - the reason is clipped;
     - `lines` keeps at most `MAX_LINES` lines, each clipped to 200 characters;
     - digests that are not 64 lowercase hex characters are dropped.
   - When a step ends, print one line: `<name>: <status> - <reason or its first summary line>`.
6. Finish.
   - Write the results (below).
   - Remove the scratch root unless `--keep-scratch` (best effort; print if it remains).
   - Print the summary path and the hint `git add -f work/windows-batch/<dir>`.
   - Exit.

### Context, one per step (implements `batch_interface.Context`)
- `say(line)`: print and flush.
- `manual(line)`: print the console bell (`\a`) and then exactly one line, `>>> <line> Press Enter to continue.`, then read one line of input. End of input (EOFError) counts as Ctrl+C: raise KeyboardInterrupt.
- `ask(question, default)`: print `??? <question> [<default>]: `, read one line and return it stripped, or the default when it is empty. EOF is handled as above.
- `run(argv, *, timeout, cwd=None, env=None, interactive=False)`:
  - `Popen` without a shell. `cwd` defaults to `repo`; `env=None` inherits the environment.
  - Not interactive:
    - stdin is `DEVNULL`, stdout is `PIPE` and stderr is `STDOUT`;
    - a reader thread reads byte lines, decodes them as UTF-8 with `errors="replace"`, echoes and collects each line;
    - the output is also written to `<ctx.private>/<NN>-<stem of argv[0]>.log`, where NN is two digits counting this step's runs.
  - Interactive: stdio is inherited and `output` is `""`.
  - Time limit: `wait(timeout)`. When it expires:
    - end the process tree with `taskkill /PID <pid> /T /F` on Windows, falling back to `Popen.kill()`;
    - wait at most 10 s more;
    - return `returncode=None` and `timed_out=True`.
  - KeyboardInterrupt while waiting: end the tree the same way, then re-raise.
  - `seconds` is the monotonic elapsed time.
- `require_quiet()` uses the guard below.
  - While heavy processes run, print each one as `  <pid> <name>: <why>`, without command lines. Then ask "Stop them and press Enter to check again, or type skip to skip the captures".
  - `skip`, in any case, returns `"skipped by the owner: heavy work was running (<unique names, comma-separated>)"`.
  - Any other answer checks again.
  - When no heavy process runs, return None.
  - If the process list cannot be read, print why and ask the same question; Enter retries.
- `elevated`: on Windows, `ctypes.windll.shell32.IsUserAnAdmin() != 0`; False elsewhere.
- `repo`, `private`, `scratch`, `options`: as described above.

### Heavy-work guard (`batch_guard.py`)
- `list_processes(timeout=60) -> list[dict]`, with keys `pid`, `ppid`, `name` and `command` (`command` may be None).
  - Source: `powershell.exe -NoProfile -NonInteractive -Command "Get-CimInstance Win32_Process | Select-Object ProcessId,ParentProcessId,Name,CommandLine | ConvertTo-Json -Compress"`.
  - Accept a single object as well as a list.
  - Raise an exception with a short message on failure.
- `heavy(processes, own_pid) -> list[tuple[int, str, str]]` returns `(pid, name, why)`. It is a pure function.
  - It excludes `own_pid` and all its descendants, following the `ppid` links.
  - It flags by executable name, compared case-insensitively without `.exe`:
    - `build`: `cl`, `link`, `lib`, `ninja`, `cmake`, `msbuild`, `devenv`, `csc`, `vbcscompiler`, `dxc`, `fxc`, `cargo`, `rustc`;
    - `heavy tool`: `blender`, `ffmpeg`;
    - `Godot`: any name starting with `godot`;
    - `project GPU program`: `sb_probe`, `sb_handoff`, `nativehostregression`, `c600native`, `mpult`;
    - `PresentMon capture`: any name starting with `presentmon`, except `presentmonservice`.
  - `Codex implementation call`: a process whose command line contains both `codex_review.py` and `implement`.
  - It flags nothing else. In particular it flags neither `codex.exe app-server`, nor a plain `python.exe` or `node.exe`, nor the shell.

### Results (`batch_report.py`)
- The record, with these keys:
  ```
  {"format": FORMAT, "started_utc", "finished_utc", "outcome": "pass" | "fail" | "interrupted",
   "source": {"head", "changed_paths": <count>, "digest"} or {"error": <clipped text>},
   "machine": {"os", "os_release", "os_build", "python", "python_bits", "elevated"},
   "options": {"only": [...], "scenes": [...], "runs", "faults", "rebuild_probe"},
   "steps": [{"name", "title", "status", "reason", "counts", "digests", "details", "lines"}],
   "private_records": "work/loop-memory/windows-batch/<stamp>/ (not committed)"}
  ```
  - `outcome`: `interrupted` if a step was interrupted; otherwise `fail` if a step failed or ended in error; otherwise `pass`.
  - `source`: from `source_identity(timeout=30)`. Any exception is recorded as `{"error": ...}`.
- Sanitise the whole record with `sanitize(record, roots)`.
  - Roots: `{"<repo>": repo, "<scratch>": scratch root, "<temp>": tempfile.gettempdir(), "<home>": Path.home()}`.
  - Render the summary from the sanitised record.
  - Check `leaks()` of the JSON text and of the summary text, and `leaks_in()` of the record.
  - If anything remains:
    - write neither public file;
    - write the unsanitised record to `<private root>/batch-unpublished.json`;
    - print the leak kinds;
    - exit 2.
- Output directory: `<repo>/work/windows-batch/<UTC date YYYY-MM-DD>/`, or `-2`, `-3`, and so on when it exists. Never overwrite.
  - Write `batch.json`: UTF-8, LF, `indent=1`, `sort_keys=True`, trailing newline.
  - Write `summary.md`.
- `summary.md` is one page: at most 60 lines, LF.
  - Title: `# Windows batch <date>: <outcome>`.
  - A short list: the source commit and the changed-path count, the Windows build, Python, administrator yes or no, and the start and end times (UTC).
  - The evidence line: "Actual Windows on fresh synthetic data. Renderer numbers hold only for the probe build they name; the native self-test and the migration probes are not performance evidence."
  - A table `| Step | Status | Counts | Reason |`. Counts are `name=value`, joined by `, `. Escape `|` in every cell.
  - One `## <name>` section per step, with its `lines` as a list; a step without lines gets no section.
  - A last line that names the private records directory (repository-relative) and says that raw records are not committed.

### Exit codes
- As in `batch_interface`: `EXIT_PASS`, `EXIT_FAIL`, `EXIT_USAGE` and `EXIT_INTERRUPTED`.
- A leak takes precedence over the step outcome (exit 2).
- An interrupted batch exits 130, after its results are written.

### README.md
- What the batch runs, and in which order.
- The command, run from the repository root:
  - run it in an administrator PowerShell to include the renderer captures, which need PresentMon;
  - otherwise that step is skipped with its reason.
- The manual steps the owner meets.
- Where the results go, and how to commit them: `git add -f work/windows-batch/<dir>`.
- What stays private.

### Tests (`tests/test_windows_batch.py`)
Use unittest and the standard library. The tests use no real step, console, PowerShell, elevation or network, and write nothing inside the repository. Each test uses a temporary repository root, created with plain mkdir below `tempfile.gettempdir()` and removed afterwards. Cover at least:
- arguments:
  - `--only` keeps `STEP_MODULES` order;
  - unknown names exit 2;
  - `--scenes` validation, including `none`;
  - `--runs` bounds;
  - `--list` writes and prompts nothing.
- fake steps:
  - pass, fail, error and skipped results give exit codes 0, 1, 1 and 0;
  - an exception in one step gives ERROR, and the next step still runs;
  - KeyboardInterrupt gives INTERRUPTED, skips the later steps, writes the results and exits 130;
  - an unavailable step is skipped with its reason and gets no directories;
  - an unimportable step module gives ERROR.
- results:
  - a step whose reason, details and lines hold a drive path below the temporary root, the current user name (patch `USERNAME`) and a SID gives a `batch.json` and a `summary.md` with placeholders only;
  - a forced leak (patch the loaded sanitiser to return its input) writes no public file and exits 2;
  - an existing date directory gives `-2`;
  - `summary.md` has at most 60 lines and escapes `|`.
- console:
  - `manual` prints exactly one line starting with `>>> ` and reads one input line;
  - `ask` returns the default for an empty answer;
  - EOF behaves like Ctrl+C.
- `Context.run` with `sys.executable` children:
  - the output is captured and logged;
  - a child that sleeps 60 s, with timeout 2, returns `timed_out=True` within 20 s.
- `heavy()`:
  - flags `cl.exe`, `Godot_v4.7.2-stable_win64.exe`, `PresentMon-2.6.0-x64.exe`, `sb_probe.exe` and a python process running `codex_review.py --kind implement`;
  - does not flag `PresentMonService.exe`, `codex.exe app-server`, a python process running another script, or the batch's own pid and its children.

Rules:
- LF line endings and ASCII source.
- Standard library only.
- No network.
- Nothing installed.

```implement-contract
{"allowed_files": ["tools/windows_batch/run_all.py", "tools/windows_batch/batch_guard.py", "tools/windows_batch/batch_report.py", "tools/windows_batch/README.md", "tests/test_windows_batch.py", "tools/ci/run_headless.py"], "acceptance_check": ["python", "tests/test_windows_batch.py"], "stop_condition": "the acceptance check passes; run_all.py implements the command line, flow, context, guard and results above and loads the step modules named in STEP_MODULES; README.md gives the owner's command; run_headless.py lists both batch test files"}
```

## 7. Required return format
- Implementation: change only the assigned worktree.
- The final message lists:
  - the changed files;
  - the acceptance result;
  - the choices you made where this packet is silent;
  - what the sandbox could not verify;
  - open points.
