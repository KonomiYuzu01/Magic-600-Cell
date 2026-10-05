# Review packet: second scoped verification of the level 2 review fixes

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: the second and last scoped verification round for the level 2 candidate on branch `claude/renderer-l2`. After round 1, Claude wrote four fixes, in four commits: `git diff fa28fd1 HEAD`.
  1. Qt app (`work/experiments/renderer-sd/app/src/l2.cpp`, `check_project.py`, `FRAMEWORK-FACTS.md` Q9): L2-A-002, second round.
  2. Finalizer (`work/experiments/renderer-l2/finalize_run.py`, `check_l2.py`): L2-V-001.
  3. Godot app (`work/experiments/renderer-sa2/project/Level2.cs`, `check_project.py`, `FRAMEWORK-FACTS.md` G9): L2-R-001, found by the first runtime run.
  4. Godot app and contract (`work/experiments/renderer-sa2/project/Level2.cs`, `Level2Windows.cs`, `check_project.py`, `HARNESS.md` section 2, `FRAMEWORK-FACTS.md` G10): L2-R-002, found by the second runtime run.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`.
  - For each finding of section 2, say whether it is fixed.
  - Report a new `blocker` or `major` only if one of these fixes introduces it, with a concrete counterexample: an input, a run or a sequence of calls and the wrong outcome.
  - The verdict is `pass` if every finding is fixed and nothing new blocks.
- Questions:
  1. L2-A-002: suppose logging rules from any source disable the default category's warnings. Sources: `QT_LOGGING_CONF`, `QT_LOGGING_RULES`, `qtlogging.ini` or `setFilterRules`. After a failed `endFrame` (Q9), can the Qt app still:
     - mark that frame shown;
     - count it as presented;
     - end the trace on it;
     - or exit 0?
     Further checks:
     - Are the filter and the handler installed before `QGuiApplication` reads the rules and before Qt starts a thread?
     - Does every other category keep the state its rules give it?
     - Does the previous handler still receive every message it received before?
     - Can the filter deadlock or recurse? Qt runs it under the registry mutex.
  2. L2-V-001:
     - Does the finalizer accept every consistent failed geometry record the DLL can write, up to three failed coordinates per sample, as failed evidence (exit 2)?
     - Does it still refuse a count above that?
     - Does the change alter any other case?
  3. L2-R-001:
     - Does `ProjectAssemblyPath` return the file that Godot loaded the running project assembly from? Can it return another file, such as a stale build or another assembly?
     - Does every failure exit 1 with reason `assembly-path` before any device work, and still write `harness.json`?
     - Does the finalizer's `identity` check then hash the file the run used?
  4. L2-R-002:
     - Suppose Windows scales the system-aware Godot window, because the window's monitor has a DPI other than the system DPI. Can that run still settle, pass the finalizer or become a record?
     - Do any of the five sizes come from a source that Windows scales along with the window, so that they would still agree?
     - Does accepting system awareness weaken any other check, or change the Qt app?
- Out of scope:
  - everything else in the level 2 candidate, including L2-A-001, L2-B-01 and L2-B-02 (fixed in round 1);
  - performance claims;
  - findings below `major`.

## 2. Actual problem and reproduction
Round 1 ran on the fast tier: 20261004T075147Z-a079eaf6. It confirmed L2-A-001 and L2-B-01 fixed and found two verified `major` findings:
- L2-A-002 (still open): disabled default warnings bypass the failed-frame watch. The app set `QT_LOGGING_RULES` for two named categories only, and `QT_LOGGING_CONF` stayed inherited. Counterexample: `QT_LOGGING_CONF` names an INI with `[Rules]` and `default.warning=false`. On the final trace frame, `Present` fails other than by device loss. Qt drops `Failed to end frame`, `frameFailed()` stays false, and the app exits 0.
- L2-V-001 (introduced by the L2-B-02 fix): the finalizer required `failures <= samples` for each geometry result. The DLL stores three coordinates per sample and counts each failed one (`renderer-sa2/native/src/scene.cpp` 190 and 409). A consistent failed record with 9066 samples and 9067 failures was refused (exit 5) instead of kept as failed evidence (exit 2).

L2-R-001 was found by Claude's first level 2 runs on 4 October:
- The Godot app's validation run and its geometry run (build from `6f83d0b`) both exited 1 with harness reason `usage`, before any device work.
- Cause: Godot 4.7.2 loads the project assembly from memory (G9), so `typeof(Smoke).Assembly.Location` is empty. `Path.GetFullPath("")` throws an `ArgumentException`, and `_Ready` reports every `ArgumentException` as `usage` (`Level2.cs` 135). `check_project.py` had pinned the faulty expression.

L2-R-002 was found by the second level 2 runs on 4 October (build from `a52dd1c`):
- The Godot app recorded the right assembly path. Its validation run and its geometry run then both exited 1 with reason `dpi-awareness`, and recorded `dpi_awareness` `unknown`.
- Cause: Godot 4.7.2 makes the process system DPI aware (G10). `HARNESS.md` section 2 assumed both frameworks are per-monitor version 2 aware by default, and the Godot app required that.
- Fix: the Godot app records `per-monitor-v2`, `system` or `unknown`, and accepts the first two. `HARNESS.md` section 2 now states the awareness per framework and why system awareness cannot hide scaling. The Qt app still requires per-monitor version 2.

## 3. Environment and versions
- Branch `claude/renderer-l2`, at the commit that adds this packet. Godot 4.7.2 .NET (`4.7.2.stable.mono.official.ed1daf0bf`); Qt 6.10.3 (`msvc2022_64`); Python 3.14.
- Evidence available (source, fixtures and offline builds):
  - `python -B work/experiments/renderer-l2/check_l2.py`: 170 finalizer cases. New: failed records with 9067 and 27198 failures give exit 2; the refusal limit is now 27199.
  - `python work/experiments/renderer-sa2/check_project.py`: PASS. It has four new planted defects:
    - `Assembly.Location` back;
    - the name check weakened;
    - the system-aware context number changed;
    - `unknown` accepted in place of `system`.
  - `python work/experiments/renderer-sd/check_project.py`: ok. It has four new planted defects: no filter, the warning level turned off, a `defaultCategory()` call, and the order of the previous filter and the override swapped.
  - Both apps' `prepare_l2.py` built offline from committed heads (Qt at `a52dd1c`, Godot at `2e1868e`; `matches_head` true). The app code compiles with no warnings. The shared native build repeats two existing C4996 deprecation warnings in S-B's `probe.cpp`.
- L2-A-002 experiment (private helper, QtCore 6.10.3):
  - A console program compiled from the watch code of `l2.cpp`, once as in `092463b` and once as fixed. It installs the watch, starts `QCoreApplication` and logs the warning text with a plain `qWarning`.
  - Rule setups: `default.warning=false` in `QT_LOGGING_RULES`; `*.warning=false` in `QT_LOGGING_RULES`; and a `QT_LOGGING_CONF` file with `default.warning=false`.
  - The `092463b` code missed the warning in all three. The fixed code saw it in all three, and Qt's default handler still wrote it to stderr (`QT_FORCE_STDERR_LOGGING=1`).
- Runtime: HARNESS section 11 steps 2 (`-Validation`, four records) and 3 (`-Geometry`), run unattended on the owner's RTX 4070 Laptop GPU with no PresentMon. The records are private and out of scope.
  - Qt app: all five records passed, at a build from `6f83d0b` and again at a build from `a52dd1c` (with fix 1).
  - Godot app: at `6f83d0b`, L2-R-001; at `a52dd1c`, L2-R-002 (section 2).
  - Godot app at a build from `2e1868e` (with fixes 3 and 4): the four validation records and the geometry record passed. Each run recorded `dpi_awareness` `system`, and all five sizes were 2560 x 1600, the display's mode.
  - No failed `Present` has been produced (it needs a fault inside Qt).

## 4. Necessary source and evidence
- `work/experiments/renderer-l2-packets/FRAMEWORK-FACTS.md` Q9, G9 and G10.
- Qt sources at tag `v6.10.3`:
  - `qtbase/src/corelib/global/qlogging.cpp` 2117-2121 (the category check before any handler) and 2374-2381;
  - `qtbase/src/corelib/io/qloggingregistry.cpp`:
    - 335-383: the rule sources and their order;
    - 391-400: the filter runs on registration;
    - 452-476: the filter runs again on every rule update, under the registry mutex;
  - `qtdeclarative/src/quick/scenegraph/qsgthreadedrenderloop.cpp` 788-799 and `qsgrenderloop.cpp` 709-722.
- Godot sources at tag `4.7.2-stable`: `modules/mono/glue/GodotSharp/GodotPlugins/PluginLoadContext.cs` 15, 27-28 and 53-69; `Main.cs` 139-151. `Load` replaces `AssemblyLoadedPath` for every assembly the context resolves itself. `work/experiments/renderer-sa2/project/SA2Smoke.csproj` has no package or project references.
- Godot DPI awareness (G10): `platform/windows/display_server_windows.cpp` 7927-7929 and `main/main.cpp` 2759 at tag `4.7.2-stable`. The manifest embedded in the pinned Godot executable declares only `longPathAware`.
- Microsoft's documentation of `EnumDisplaySettingsW` (Remarks): the function does not take part in DPI virtualization and always reports physical pixels. The Godot app's five sizes: `Level2Windows.Display` and `ClientSize`, `Level2.ReadSizes`, and `Level2Sizes.WindowSettled` (`Level2Result.cs` 19-20).
- `work/experiments/renderer-sa2/prepare_l2.py` 80-86: the build's part list expects `project/.godot/mono/temp/bin/Debug/SA2Smoke.dll`.
- The DLL's geometry check: `work/experiments/renderer-sa2/native/src/scene.cpp` 361-432.
- Contract: `work/experiments/renderer-l2-packets/HARNESS.md` sections 4 (exit codes), 7 (refusals) and 11 (validation sequence).

## 5. Attempts so far
| # | Finding | Change | Verification | Result |
|---|---|---|---|---|
| 1 | L2-A-002 | `watchFrameFailures` also installs `keepDefaultWarnings` before `QGuiApplication`. It applies the previous filter (Qt's rules), then keeps the `default` category's warnings on, matching by name. | experiment (section 3); `check_project.py`, four planted defects; offline build | fixed (experiment and source) |
| 2 | L2-V-001 | each result needs `0 <= failures <= 3 * samples` | three fixture cases (9067, 27198 accepted; 27199 refused) | fixed (fixtures) |
| 3 | L2-R-001 | `godot:assembly` is `ProjectAssemblyPath(typeof(Smoke).Assembly)`: the load context's `AssemblyLoadedPath`, after a check that the file's full assembly name is the running assembly's; otherwise `InvalidOperationException("assembly-path")` (exit 1) | `check_project.py`, two planted defects; offline build; the second runtime run recorded the build's assembly file | fixed (runtime) |
| 4 | L2-R-002 | `Level2Windows.DpiAwareness` returns `per-monitor-v2` (-4), `system` (-2) or `unknown`; `Settle` accepts the first two; `HARNESS.md` section 2 and G10 | `check_project.py`, two planted defects; offline build; five passing runs at `2e1868e` | fixed (runtime) |

## 6. Constraints and owned files
- Read-only review. No file changes.
- Invariants:
  - synthetic labels only;
  - claims only for the measured build identity;
  - the finalizer alone writes records;
  - never commit raw PresentMon output or machine diagnostics.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
