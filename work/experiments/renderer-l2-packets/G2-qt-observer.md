# Packet G2: Qt module observer and loaded-DLL path (L2-V-002)

Run from the `claude/renderer-l2-followup` checkout, with no gate capture or other implement call active, other than the parallel G1 and G3 calls:
`python tools/agents/codex_review.py --kind implement --model gpt-6.1-sol --effort max --timeout 7200 --packet work/experiments/renderer-l2-packets/G2-qt-observer.md`

Plan: `work/experiments/renderer-l2-packets/PLAN-L2-V-002.md`, approved by Astra (plan check `20261009T161932Z-1f325e3c`, re-checks `20261009T163456Z-b3c5367e` and `20261009T164302Z-5337660e`, pass). This packet implements plan section 5, the sd part of the app-suite tests of section 7, and the app side of the Qt probe of section 7, experiment 3. G1 (guard) and G3 (Godot) run in parallel. G4 (finalizer) comes later and reads the `harness.json` fields defined in section 6 below, so those fields are part of the contract.

## 1. Goal and acceptance
- Goal: the Qt level 2 app records every module it loads from its executable's directory during the whole process lifetime, and fails closed when it cannot. It also records the DLL path it actually loaded.
- Acceptance check: `python -B work/experiments/renderer-sd/check_project.py` exits 0.
- Done when:
  - plan section 5 is implemented (three intervals, callback rules, seal order, record, `files.dll`, code placement);
  - `module_observer_test.cpp` implements the seven cases of plan section 7 as a separate program, built by `build.cmd` with a probe DLL;
  - the app-side probe injection of section 6 exists;
  - `check_project.py` has the sd static checks and planted defects of plan section 7, and every existing check still passes or is updated where plan section 5 replaces the behaviour it pinned (the path-only and `EnumProcessModules` assertions);
  - `README.md` describes the observer, the test program, the probe and the stop point below.
- Non-goals:
  - the guard, the finalizer, the runner and `HARNESS.md` (G1, G4, G5);
  - the DLL, its header and the Godot app;
  - building, compiling or starting Qt, a C++ compiler or a GPU process in the sandbox;
  - performance claims.

## 2. Actual problem and reproduction
- Astra ruling `20261009T155501Z-a2cb4438`:
  - L2-V-002-04: `L2::files()` (`app/src/l2.cpp`) enumerates loaded modules once at the end of the run, so a late module that is unloaded before then, a transient load-call-unload module, or an unload and reload at the same path are not in the identity;
  - L2-P-002-02 (plan check): `files.dll` is the `--l2-dll` argument, not the module that was loaded, so a redirected copy would be recorded under the requested path.
- Reproduction: read `L2::files()` and `L2::write()` in `app/src/l2.cpp`, and `int main` in `app/src/main.cpp` (the level 2 dispatch is the first statement; nothing observes loads).

## 3. Environment and versions
- Base: branch `claude/renderer-l2-followup` (this worktree's HEAD).
- Owner's machine: Windows 11 (build 26200); Qt 6.10.3 `msvc2022_64` at `tools/qt/6.10.3/msvc2022_64` (not in this worktree); Visual Studio Community 2026 (MSVC 14.51), Windows SDK 10.0.26100.0, CMake and Ninja from `tools/.venv/renderer-spike/Scripts/`.
- The sandbox has no Qt, no compiler run and no GPU; Python 3.14, standard library only. Evidence in the sandbox is source/fixture only. Claude builds and runs everything on the owner's machine after integration.

## 4. Necessary source and evidence
- `work/experiments/renderer-l2-packets/PLAN-L2-V-002.md`: binding. Read sections 1, 5, 7 ("App suites", the sd observer test program, experiment 3) and 9.
- `work/experiments/renderer-l2-packets/HARNESS.md`, read-only: sections 3, 4 and 6 (options, exit codes, `harness.json`).
- The app: `work/experiments/renderer-sd/app/` (`main.cpp`, `l2.cpp`, `l2.h`, `native_loader.cpp`, `native_loader.h`, `code_layout_test.cpp`, `CMakeLists.txt`), `work/experiments/renderer-sd/build.cmd`, `check_project.py`, `README.md`.
- `work/experiments/renderer-sd/run_smoke.py`, read-only (`deployment_manifest`: `<build>/app/` holds the build outputs, `<build>/deploy/` the deployed app; the new test program and probe stay in `app/` and are never deployed).
- Win32 and loader facts the code relies on; list each in the final message as assumed, with the runtime check that catches it if it is wrong:
  - `LdrRegisterDllNotification` and `LdrUnregisterDllNotification` from `ntdll.dll` via `GetProcAddress`; the callback runs under the loader lock for every load and unload, with `FullDllName` as a counted `UNICODE_STRING`;
  - `RtlGetUnloadEventTraceEx` from `ntdll.dll`: element size, element count and the trace pointer; an entry is empty when its base address is null; `ImageName` holds at most 32 characters;
  - `EnumProcessModules` and `GetModuleFileNameW`.

## 5. Attempts so far
None. This is the first call.

## 6. Constraints and owned files
Owned:
- `work/experiments/renderer-sd/app/src/main.cpp`, `l2.cpp`, `l2.h`, `native_loader.cpp`, `native_loader.h`;
- new: `work/experiments/renderer-sd/app/src/module_observer.cpp`, `module_observer.h`, `module_observer_test.cpp`, `module_probe.cpp`;
- `work/experiments/renderer-sd/app/CMakeLists.txt`, `work/experiments/renderer-sd/build.cmd`;
- `work/experiments/renderer-sd/check_project.py`, `work/experiments/renderer-sd/README.md`.

Change nothing else. `run_smoke.py`, `prepare_l2.py`, `smoke_summary.py`, `sd_reference.py`, `RESULT.md`, `results/`, everything under `work/experiments/renderer-sa2/`, `work/experiments/renderer-l2/`, `work/experiments/renderer-l2-packets/` and `tools/` stay byte-identical.

**Observer** (`module_observer.cpp/.h`, no Qt dependency so the test program can link it without Qt):
- Scope: a module is in scope when its full path is under the directory of the process executable (`GetModuleFileNameW(nullptr)`), compared case-insensitively on backslash-separated paths.
- Started by the first statement of `main` (before the level 2 dispatch and before `QGuiApplication`), in this order: register the callback, read the unload history, take the `EnumProcessModules` snapshot (in-scope entries become `snapshot` events). The observer runs in level 1 too; only level 2 records it.
- Before `main`: a nonempty unload history at registration is a refusal (plan section 5), whatever the names.
- Callback: plan section 5 steps 1 to 3, with a fixed preallocated buffer (no allocation, no file I/O, no Qt, no CRT locale functions under the loader lock). An in-scope path longer than a slot holds sets the overflow flag, as does a full buffer.
- Seal: plan section 5, "After the seal", with sequentially consistent atomics in the stated order. An in-scope load that sees `sealed` set calls `TerminateProcess(GetCurrentProcess(), 3)`.

**`harness.json` (sd, level 2):** a new top-level object `modules`, written by `write()` after sealing:
```json
"modules": {
  "observer": "sealed",
  "failure": null,
  "scope": "C:\\...\\deploy",
  "unload_history": [],
  "overflow": false,
  "events": [{"kind": "snapshot", "path": "C:\\...\\deploy\\Qt6Core.dll"},
             {"kind": "load", "path": "C:\\...\\deploy\\platforms\\qwindows.dll"},
             {"kind": "unload", "path": "..."}]
}
```
- `observer`: `"sealed"` when registration, the history read and the snapshot succeeded and the seal completed; otherwise `"failed"`, with `failure` naming the first failed step (`"register"`, `"unload-history"`, `"snapshot"` or `"seal"`). `failure` is `null` when `observer` is `"sealed"`.
- `unload_history`: the base names of every nonempty unload-history entry at registration, in trace order; `null` if the history could not be read. A nonempty list is recorded as it is; the app does not change its exit code or reason for it. The finalizer (G4) refuses it.
- `events`: in-scope events only: the snapshot entries first (kind `snapshot`), then loads and unloads in the order their slots were reserved, with the full path as reported.
- Every level 2 exit that writes `harness.json` writes `modules`, including the usage path (`--l2-out` valid, another argument invalid). That path is the stop point below.
- The `qt:` keys of `files` are built from the union of the in-scope paths of every `snapshot` and `load` event, except the executable itself and the DLL; two paths with the same base name stay the existing `framework-modules` failure. The end-of-run `EnumProcessModules` in `L2::files()` is removed.
- `files.dll` is `GetModuleFileNameW` on the module handle `Native::load` returned, not `--l2-dll`. It stays `null` until the DLL is loaded. A failure of `GetModuleFileNameW` is the `framework-modules` failure.
- The observer state never changes `exit_code` or `reason` except through these existing `framework-modules` cases. No other field of `harness.json` changes.

**Probe** (plan section 7, experiment 3):
- `module_probe.cpp` builds `sd_module_probe.dll` (CMake target, output in the build's `app/` folder, not deployed) with one export, `int sd_module_probe(void)`, that returns a fixed value.
- `--l2-inject` gains three app-side values for `sd`: `module-late` (load and keep), `module-transient` (load, call the export, `FreeLibrary`), `module-reload` (load, `FreeLibrary`, load again and keep). The probe is `<executable directory>\sd_module_probe.dll`, loaded by absolute path with `LoadLibraryW` on the GUI thread after trace begin. These values follow the existing injection rules for scenes (W3 and W4 only), are recorded in `options.inject` as given, and are never passed to the DLL (the DLL's scene config gets no injection for them). A missing probe or a failed load is the `framework-modules` failure.

**Observer test program** (`module_observer_test.cpp`, CMake target `sd_module_observer_test`, built by `build.cmd`, no Qt window and no GPU):
- Plan section 7, cases 1 to 7, each in its own child process of the program (the program starts itself with a case argument), with `sd_module_probe.dll` from the program's directory. Case 1 copies the probe to a fresh name in the program's directory, loads, calls and unloads it from a global constructor, and (in a second variant) deletes the copy before `main`.
- Output: one line per case and a final count; exit 0 only when every case gave its expected result. Case 6 runs at least 200 repetitions.
- It writes only its probe copies in its own directory and removes them.

**Stop point** (plan section 5): `README.md` gives Claude's command for the usage-path run on the owner's machine (`sd_smoke.exe --l2-out <empty folder> --l2-scene x`) and says that a nonempty `modules.unload_history` there stops the work and returns the plan to Astra.

**`check_project.py`:**
- Adds the sd static checks of plan section 7 ("App suites", sd) with the six planted defects, in the existing planted-defect style, plus: `module_observer.cpp` has no Qt include; the probe values are never passed to the DLL; `build.cmd` and `CMakeLists.txt` build `sd_module_observer_test` and `sd_module_probe`.
- Keeps every other existing check. Keep its rules: builds nothing, starts no process other than Python, writes nothing in the worktree, fixture folders under the system temp directory created as `Path(tempfile.gettempdir()) / <unique name>` with a plain `mkdir()` (the sandbox refuses `TemporaryDirectory()` with WinError 5), removed on every exit.
- Keep the existing line endings of every file you change (`build.cmd` is CRLF; the C++ sources, `CMakeLists.txt` and Python files are LF). Every Python entry point disables bytecode before local imports.

```implement-contract
{"allowed_files": ["work/experiments/renderer-sd/app/src/main.cpp", "work/experiments/renderer-sd/app/src/l2.cpp", "work/experiments/renderer-sd/app/src/l2.h", "work/experiments/renderer-sd/app/src/native_loader.cpp", "work/experiments/renderer-sd/app/src/native_loader.h", "work/experiments/renderer-sd/app/src/module_observer.cpp", "work/experiments/renderer-sd/app/src/module_observer.h", "work/experiments/renderer-sd/app/src/module_observer_test.cpp", "work/experiments/renderer-sd/app/src/module_probe.cpp", "work/experiments/renderer-sd/app/CMakeLists.txt", "work/experiments/renderer-sd/build.cmd", "work/experiments/renderer-sd/check_project.py", "work/experiments/renderer-sd/README.md"], "acceptance_check": ["python", "-B", "work/experiments/renderer-sd/check_project.py"], "stop_condition": "the Qt app implements plan section 5 with the harness.json modules object, files.dll and probe injection of packet section 6; the observer test program and probe DLL are CMake targets built by build.cmd; check_project.py has the sd static checks and planted defects of plan section 7 and passes"}
```

## 7. Required return format
- Implementation: changes only in the assigned worktree. The final message lists:
  - the changed files;
  - the acceptance result;
  - each Win32, loader and Qt fact the code relies on, as assumed, with the runtime check that catches it;
  - what was not verified in the sandbox (every compile, Qt, loader and GPU step);
  - each choice the plan or this packet left open (slot count and size, the atomics used, how a child case is started), and the choice made;
  - every place where the plan contradicts what the code base or Windows does, what you did, and how the binding rule still holds;
  - open points for G4 about the `modules` fields.
