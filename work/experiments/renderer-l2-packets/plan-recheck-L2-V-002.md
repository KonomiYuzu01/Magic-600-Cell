# Scoped plan re-check packet: L2-V-002 plan, revision 2

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: a scoped re-check of the changes to `work/experiments/renderer-l2-packets/PLAN-L2-V-002.md` after plan check `20261009T161932Z-1f325e3c`. Check only:
  1. whether each of its three findings (L2-P-002-01, -02, -03) is closed by the revised plan;
  2. whether the changes introduce a new `blocker` or `major`.
- The plan check's other remark is also adopted: it asked for live-guard digest-mismatch controls, and they are added in section 7.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. The verdict is `pass` if no `blocker` or `major` finding remains. Each finding needs a concrete counterexample.
- Questions:
  1. L2-P-002-01: does the three-interval observer of section 5 close the lifetime gap? The intervals are:
     - before `main`: the snapshot plus the `RtlGetUnloadEventTraceEx` history, refusing on any base-name match or a full history;
     - from registration to the seal: the notification callback;
     - after the seal: an in-scope load terminates with exit 3.

     Is the seal ordering argument correct?
  2. L2-P-002-02: `files.dll` now comes from `GetModuleFileNameW` on the loaded handle in both apps, and the finalizer reads the shaders next to it and requires it to be guarded. Does that close the redirection case?
  3. L2-P-002-03: does the component walk of section 2 close the same-case-fold junction case, and the reparse cases in general? The walk opens every directory and the leaf with `FILE_FLAG_OPEN_REPARSE_POINT`, refuses `FILE_ATTRIBUTE_REPARSE_POINT` and case-sensitive directories, and holds each component without `FILE_SHARE_DELETE`. Is the new residual limit in section 9 acceptable under the threat model?
- Out of scope:
  - every unchanged part of the plan, which plan check `20261009T161932Z-1f325e3c` already covered;
  - code (none is written);
  - findings below `major`.

## 2. Actual problem and reproduction
The findings, their counterexamples and the suggested experiments are in `work/reviews/20261009T161932Z-1f325e3c/review.json`. All three are adopted (`dispositions.json`).

## 3. Environment and versions
Branch `claude/renderer-l2-followup` from `main` at `7057cb8`. Windows 11, Python 3.14, Qt 6.10.3, Godot 4.7.2 .NET.

## 4. Necessary source and evidence
- The revised plan: `work/experiments/renderer-l2-packets/PLAN-L2-V-002.md`. Its changes are the unified diff below (revision 1 to revision 2).
- `work/experiments/renderer-sd/app/src/main.cpp` (`level2Main`, line 120; `main`, line 223), `native_loader.cpp` (line 6).
- `work/experiments/renderer-sa2/project/Native.cs` (line 104), `Level2Result.cs` (line 70).
- `work/experiments/renderer-sa2/native/src/scene_record.cpp` (`module_path`, line 70).

```diff
@@ -24,14 +24,19 @@
 - sd (Qt): every `*.dll` and `*.exe` under the executable's directory, recursively. That set is wider than the identity on purpose: the identity keeps the loaded subset (section 5), and any module that may load is already protected.
 - Every ancestor directory of every protected file, up to the volume root.
 
-**Acquisition.** For each file, in a fixed order:
-1. `CreateFileW` with `GENERIC_READ`, share `FILE_SHARE_READ` only, `OPEN_EXISTING`. Existing write handles or writable mappings make this fail. Once the handle is held, ordinary write, delete and rename (which needs `DELETE` access) are refused.
-2. Ancestor directories: `FILE_LIST_DIRECTORY` with `FILE_FLAG_BACKUP_SEMANTICS`, share `FILE_SHARE_READ | FILE_SHARE_WRITE` with no `FILE_SHARE_DELETE`, so a directory on the path cannot be renamed or removed while files inside it are still created and renamed normally.
-3. Path checks, all on the held handle, failing closed:
-   - the path is absolute, on a local fixed drive (`GetDriveTypeW` is `DRIVE_FIXED`), with no `\\?\` prefix and no UNC form;
-   - `GetFinalPathNameByHandleW` (normalized, DOS volume name), without its `\\?\` prefix, equals the requested path case-insensitively. This refuses 8.3 components, `subst` drives, junctions, symbolic links and mounted folders.
-4. Identity: `GetFileInformationByHandleEx(FileIdInfo)` gives the volume serial and the 128-bit file ID. The size comes from the same handle.
-5. Digest: SHA-256 read through the held handle (`ReadFile` on it or on a duplicate), never by opening the path again.
+**Acquisition.** The guard walks each path from the volume root down, one component at a time, and holds every component it passes. Every step fails closed.
+1. Path form: the path is absolute, on a local fixed drive (`GetDriveTypeW` is `DRIVE_FIXED`), with no `\\?\` prefix, no UNC form and no `.` or `..` component.
+2. Each directory below the volume root:
+   - Open it with `FILE_LIST_DIRECTORY`, `FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT`, and share `FILE_SHARE_READ | FILE_SHARE_WRITE` with no `FILE_SHARE_DELETE`. The directory then cannot be renamed or removed, while files inside it can still be created and renamed normally.
+   - `FILE_FLAG_OPEN_REPARSE_POINT` opens the component itself and never its target.
+   - On that handle, refuse if `FileAttributeTagInfo` shows `FILE_ATTRIBUTE_REPARSE_POINT`. This covers junctions, symbolic links and mounted folders.
+   - Also refuse if `FileCaseSensitiveInfo` shows `FILE_CS_FLAG_CASE_SENSITIVE_DIR`. A case-sensitive directory is unsupported, and refusing it closes the same-case-fold junction case (L2-P-002-03).
+3. The file itself:
+   - Open it with `GENERIC_READ`, `FILE_FLAG_OPEN_REPARSE_POINT`, share `FILE_SHARE_READ` only, and `OPEN_EXISTING`. Existing write handles or writable mappings make this fail. Once the handle is held, ordinary write, delete and rename (which needs `DELETE` access) are refused.
+   - The same reparse-point test applies to the file.
+4. Spelling: `GetFinalPathNameByHandleW` (normalized, DOS volume name) of the file handle, without its `\\?\` prefix, must equal the requested path case-insensitively. With no reparse component and no case-sensitive directory on the path, this refuses only the remaining aliases: 8.3 components and `subst` drives.
+5. Identity: `GetFileInformationByHandleEx(FileIdInfo)` gives the volume serial and the 128-bit file ID. The size comes from the same handle.
+6. Digest: SHA-256 read through the held handle (`ReadFile` on it or on a duplicate), never by opening the path again.
 
 **Ready and release.**
 - After every handle is held and hashed, the guard writes `<run directory>/guard.json` and prints one `ready` line on stdout. `guard.json` holds:
@@ -40,7 +45,7 @@
   - `ready` (`GetSystemTimePreciseAsFileTime`, taken after the last hash);
   - one entry per file: path, final path, volume serial, file ID, size, sha256;
   - one entry per directory: path, volume serial, file ID.
-- The guard then blocks on stdin. On `release` it checks again that every handle still has the recorded file ID, closes the handles and exits 0. Any other input, an end of file, or a failed check makes it exit non-zero. The handles also close when the guard dies, which the finalizer detects (section 3).
+- The guard then blocks on stdin. On `release` it checks again that every handle still has the recorded file ID and no reparse attribute, closes the handles and exits 0. Any other input, an end of file, or a failed check makes it exit non-zero. The handles also close when the guard dies, which the finalizer detects (section 3).
 - The guard only reads. Its only write is `guard.json`.
 
 ## 3. Finalizer (`finalize_run.py`)
@@ -58,6 +63,7 @@
 3. Order: the app's process creation time is later than the guard's `ready`. The runner passes it as a new argument, `--launched-created <FILETIME>`, read from the app's process handle while that handle is open.
 4. Every identity part is guarded:
    - the parts are `dll`, every shader next to it, and every `files` entry;
+   - `files.dll` is now the path of the module actually loaded (sections 4 and 5), and the shaders are read next to it, as the DLL itself does (`module_path()`). A redirected copy of the DLL is therefore an unguarded path and is refused (L2-P-002-02);
    - each recorded path, normalized and compared case-insensitively, equals one guarded path;
    - the finalizer opens the path once and checks that `FileIdInfo` on that handle matches the guard's entry;
    - it hashes through the same handle, and that digest must equal the guard's;
@@ -79,23 +85,46 @@
 
    `renderer_gate.py` ignores unknown keys (`validate_run`).
 
-## 4. Godot app (`renderer-sa2/project/Level2.cs`)
+## 4. Godot app (`renderer-sa2/project/Level2.cs`, `Level2Result.cs`, `Native.cs`)
 
 - Record `assembly_mvid` = `typeof(Smoke).Assembly.ManifestModule.ModuleVersionId` (string, `D` format) next to `godot:assembly`.
+- Record `files.dll` from `GetModuleFileNameW` on the handle that `NativeLibrary.Load` returned, not from the `--l2-dll` argument. A failure is refused (`framework-modules`).
 - No other change. The guard covers `project.godot` and `Main.tscn` before Godot reads them (L2-V-002-03). It also covers the shaders before scene loading (L2-V-002-02).
 
-## 5. Qt app (`renderer-sd/app/src/main.cpp`, `l2.cpp`, `l2.h`)
+## 5. Qt app (`renderer-sd/app/src/main.cpp`, `l2.cpp`, `l2.h`, `native_loader.cpp`, new `module_observer.cpp` and `.h`)
 
-- First statement of `main`, before `QGuiApplication`:
-  - register an `LdrRegisterDllNotification` callback (resolved from `ntdll` with `GetProcAddress`);
-  - then snapshot the loaded modules once with `EnumProcessModules`.
-  - A failure in either step marks the observer as failed, and `write()` records that (fail closed).
-- Callback: on load and on unload, copy `FullDllName` and the reason into a fixed preallocated buffer (append-only, with an atomic index and an overflow flag). Do no file I/O and no allocation under the loader lock.
-- `write()`:
-  - records `modules`: the observer state, overflow, and the event list (snapshot entries, then loads and unloads in order);
-  - builds the `qt:` keys from the union of every loaded path under the executable directory. It no longer uses the end-of-run `EnumProcessModules`.
-  - Two paths with the same basename are refused, as now.
-  - This covers late modules, transient load-call-unload modules, and unload/reload at the same path (L2-V-002-04).
+The observer covers the whole process lifetime in three intervals. Every in-scope module load is either in the record or makes the run fail (L2-P-002-01). A module is in scope when its path is under the executable's directory.
+
+**Before `main`** (process start until registration):
+- Modules loaded before `main` that are still loaded are in the snapshot below.
+- Modules loaded and unloaded before `main` are in the loader's unload history, `RtlGetUnloadEventTraceEx`. Its entries keep only a base name of up to 32 characters.
+- At registration, the observer refuses (fail closed) when:
+  - any history entry matches, case-insensitively, the base name (or its 31-character prefix) of any file under the executable's directory;
+  - or the history is full, so it may have wrapped.
+
+**From `main` until the record is sealed:**
+- First statement of `main`, before `QGuiApplication` and before the level 2 dispatch: register an `LdrRegisterDllNotification` callback (resolved from `ntdll` with `GetProcAddress`), check the unload history, then snapshot the loaded modules once with `EnumProcessModules`.
+- A failure in any step marks the observer as failed, and `write()` records that (fail closed).
+- Callback, on load and on unload:
+  1. reserve a slot with an atomic increment;
+  2. copy `FullDllName` and the reason into a fixed preallocated buffer, or set the overflow flag;
+  3. mark the slot complete.
+
+  It does no file I/O and no allocation under the loader lock.
+
+**After the seal** (until process exit):
+- `write()` first sets an atomic `sealed` flag, then reads the slot count, then waits for those slots to be complete.
+- Any in-scope load whose callback sees `sealed` set calls `TerminateProcess(GetCurrentProcess(), 3)`. Exit 3 is never gate evidence: the runner stops and the finalizer refuses `app-exit`.
+- With sequentially consistent atomics, every in-scope load is either counted before the seal or sees the seal. Unloads after the seal change nothing, because a loaded module is already in the union.
+
+**The record** (`write()`):
+- `modules`: the observer state, the unload-history result, overflow, and the event list (snapshot entries, then loads and unloads in order).
+- The `qt:` keys are built from the union of every loaded in-scope path. The end-of-run `EnumProcessModules` is no longer used.
+- Two paths with the same basename are refused, as now.
+- This covers late modules, transient load-call-unload modules, and unload/reload at the same path (L2-V-002-04).
+- `files.dll` comes from `GetModuleFileNameW` on the module handle `native_loader.cpp` loaded, not from `--l2-dll` (L2-P-002-02).
+
+**Code placement:** the observer is its own source file, so a small non-GPU test program can link it (section 7), as `code_layout_test.cpp` does now.
 
 ## 6. Runner (`run_scene.ps1`)
 
@@ -122,16 +151,23 @@
 - After release, each of them succeeds again.
 - `guard.json` digests equal an independent hash. A planted defect that hashes by reopening the path is caught: the test removes the path's read access after acquisition, and the guard must still hash.
 - Each of these alias forms is refused:
-  - a junction in the path;
+  - a junction as a directory component, and a symbolic link as the leaf (when the test may create one);
+  - a junction whose name differs from its target only in case;
+  - a case-sensitive directory on the path (when `fsutil file setCaseSensitiveInfo` works on the test volume; otherwise "not run", with the refusal still checked against a stubbed `FileCaseSensitiveInfo` result);
   - an 8.3 component (when the volume has 8.3 names);
-  - a `\\?\` prefix;
-  - a relative path.
+  - a `\\?\` prefix, a `..` component and a relative path.
 - The guard exits non-zero on end of file or unknown input. A killed guard releases the files.
 
 **Finalizer (`check_l2.py`):** every existing fixture runs with a real guard over its parts. New cases, all in the four modes:
 - for each part (`godot:exe`, `godot:assembly`, `godot:project.godot`, `godot:Main.tscn`, `godot:Smoke.cs`, `dll`, each shader, `qt:exe`, one `qt:Qt6*.dll`, `qt:qwindows.dll`): release the guard, replace the part, finalize, and require exactly `identity`;
 - `guard.json` missing, malformed, or with a wrong or dead PID → `identity`;
 - an edited file ID, or a listed file that is not held (the write probe succeeds) → `identity`;
+- live-guard digest mismatches, with the guard alive throughout:
+  - an edited `sha256` in `guard.json`;
+  - a DLL whose `dll_identity` digest differs from the guarded bytes;
+  - a shader that the DLL reports but the guard does not list;
+  - each → `identity`;
+- `files.dll` at a path other than the guarded DLL, with the same bytes (a redirected copy) → `identity`;
 - app created before `ready` → `identity`;
 - a recorded path outside the guarded set (a copy with the same bytes) → `identity`;
 - sa2: same `FullName`, different MVID (synthetic minimal .NET PE built by the test) → `identity`;
@@ -151,19 +187,40 @@
 - The path-only assertions are replaced.
 - sd:
   - registration is the first statement of `main`;
-  - the snapshot follows registration;
+  - the unload-history check and the snapshot follow registration;
   - the callback does no I/O;
+  - `write()` seals before it reads the count, and a post-seal in-scope load terminates with exit 3;
   - `write()` uses the event union;
+  - `files.dll` comes from `GetModuleFileNameW` on the loaded module;
   - failure is fail-closed.
-  - Planted defects: registration after `QGuiApplication`; `write()` back on `EnumProcessModules`; overflow ignored.
-- sa2: the MVID is recorded from `ManifestModule.ModuleVersionId`. Planted defects: missing or from another assembly.
+  - Planted defects:
+    - registration after `QGuiApplication`;
+    - the unload-history check removed;
+    - the seal after the count read;
+    - `write()` back on `EnumProcessModules`;
+    - overflow ignored;
+    - `files.dll` back on `--l2-dll`.
+- sd observer test program (`module_observer_test.cpp`, built by `build.cmd`, no GPU and no Qt window). Each case runs in its own process with a probe DLL in the program's directory:
+  1. the probe is loaded, called and unloaded by a global constructor before `main`: refusal;
+  2. a persistent load during `main`: in the union;
+  3. load, call, unload during `main`: in the union;
+  4. unload, then reload at the same path: in the union once;
+  5. a load after the seal: exit 3;
+  6. a load on a second thread racing the seal: either in the union or exit 3, over many repetitions;
+  7. the event buffer overflowing: refusal.
+- sa2:
+  - the MVID is recorded from `ManifestModule.ModuleVersionId`;
+  - `files.dll` is taken from `GetModuleFileNameW` on the `NativeLibrary.Load` handle.
+  - Planted defects: the MVID missing, or taken from another assembly; `files.dll` back on the argument.
 - Same-handle hashing, protection lifetime and fail-closed binding are properties of the guard and the finalizer, so `check_guard.py` and `check_l2.py` test them. The app suites test what the apps record.
 
-**Owner-machine experiments (actual Windows, after integration):**
-1. Synthetic, run by Claude without the GPU or PresentMon (`guard_experiments.py`):
+**Owner-machine experiments (actual Windows):**
+1. Synthetic, run by Claude without the GPU or PresentMon (`guard_experiments.py`), before the full candidate review:
    - the alias forms that `check_guard.py` cannot make portably: `subst`, a symbolic link (if allowed) and `\\localhost\` UNC;
    - an ancestor rename and a hard link;
-   - release timing.
+   - release timing;
+   - DLL redirection: a small loader program without a manifest, with a `.local` folder holding a copy of a probe DLL. Load the probe by its absolute path and compare the requested path with `GetModuleFileNameW` on the returned handle. The experiment shows whether redirection applies and whether the recorded actual path reveals it. Either way, the finalizer check of section 3 refuses a path that is not guarded.
+   - the observer test program of the app suites, run on this machine.
 2. Attended, with the owner: one short W3 run per app with a helper that tries to replace each part every second during the run, while the guard holds it. Every attempt must be refused, and the run must finalize with an identity equal to an independent pre-run hash.
 3. Qt: a synthetic probe DLL in `deploy/`, loaded in three ways by a new fault-injection mode:
    - a late, persistent load;
@@ -177,8 +234,8 @@
 | Packet | Owner | Files | Acceptance | After |
 |---|---|---|---|---|
 | G1 guard | Codex | `renderer-l2/file_guard.py`, `renderer-l2/check_guard.py` | `python -B work/experiments/renderer-l2/check_guard.py` | — |
-| G2 Qt observer | Codex | `renderer-sd/app/src/main.cpp`, `l2.cpp`, `l2.h`, `renderer-sd/check_project.py` | `python -B work/experiments/renderer-sd/check_project.py` | — |
-| G3 Godot MVID | Codex | `renderer-sa2/project/Level2.cs`, `Level2Result.cs`, `renderer-sa2/check_project.py` | `python -B work/experiments/renderer-sa2/check_project.py` | — |
+| G2 Qt observer | Codex | `renderer-sd/app/src/main.cpp`, `l2.cpp`, `l2.h`, `native_loader.cpp`, new `module_observer.cpp`, `module_observer.h`, `module_observer_test.cpp`, `renderer-sd/build.cmd`, `renderer-sd/check_project.py` | `python -B work/experiments/renderer-sd/check_project.py` | — |
+| G3 Godot MVID and DLL path | Codex | `renderer-sa2/project/Level2.cs`, `Level2Result.cs`, `Native.cs`, `renderer-sa2/check_project.py` | `python -B work/experiments/renderer-sa2/check_project.py` | — |
 | G4 finalizer | Codex | `renderer-l2/finalize_run.py`, `renderer-l2/check_l2.py`, `renderer-l2-packets/HARNESS.md` | `python -B work/experiments/renderer-l2/check_l2.py` | G1 |
 | G5 runner | Claude | `renderer-l2/run_scene.ps1` and its static check | `check_l2.py` | G4 |
 
@@ -197,5 +254,5 @@
   - the 4 October records, which keep Astra's wording (`RESULT.md`).
 - Residual limits:
   - Files that are not identity parts are not bound and were never claimed: Godot's own .NET assemblies, Qt files that are not DLLs, and system DLLs.
-  - A Qt module that a static import's `DllMain` loads and unloads before `main` would not enter the identity. Its bytes are still protected.
   - The guard's directory handles stop renames of the folders on the path for the length of a run. That is intended.
+  - Setting a reparse point on a held directory or file and removing it again within one run is outside the accidental-replacement threat model. A junction or mount point needs an empty directory, and every held directory contains a held file. A reparse point left in place makes the finalizer open a different file (file-ID mismatch) and fails the guard's release check.
```

## 5. Attempts so far
Plan revision 1 was checked by `20261009T161932Z-1f325e3c` (three majors). This is revision 2, the first re-check.

## 6. Constraints and owned files
- Owned: the plan and this packet. No code changes.
- Not a critical path. Contract change, so Astra on the fast tier.

## 7. Required return format
One JSON object matching `schemas/review-result.schema.json`, with ID, severity, evidence (`path:line` with file digest), counterexample, suggested experiment and verification status for each finding.
