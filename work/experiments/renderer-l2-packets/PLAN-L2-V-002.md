# Plan: bind the level 2 build identity to the loaded files (L2-V-002)

Claude, 9 October 2026. This plan follows the Astra ruling `20261009T155501Z-a2cb4438` (findings L2-V-002 and -02 to -05; all adopted). It changes the level 2 harness contract, so it gets an Astra plan check before any code.

## 1. Threat model and binding rule

- Threat: accidental replacement. A file can be replaced, replaced and then restored, or loaded late, anywhere in the interval from the app's first possible read until the finalizer has finished. The OS, compiler and harness are trusted. Hostile-machine attestation is out of scope.
- Binding rule (Astra): each recorded digest describes the exact bytes consumed, or bytes protected against replacement before their first possible consumption.
- Design: an outer guard process takes the protection before the app is launched and keeps it until the finalizer has finished. The existing runner, the presenting PID and S-B's runner stay as they are. `renderer_gate.py` does not change.
- The identity encoding does not change: same part names, canonical settings, sorted `name=sha256` lines. The binding evidence stays outside the hash. The same bytes therefore give the same identity as before.

## 2. The guard (`work/experiments/renderer-l2/file_guard.py`, new)

A Windows-only Python script (ctypes) that the runner starts once per run, before `Start-App`.

**Protected set.** The script derives it from `launch.json` and the candidate. It needs no change to the launch format.
- Both candidates:
  - `launch.executable`;
  - `launch.dll`;
  - the four shader files next to the DLL (`SHADERS`, shared with the finalizer).
- sa2 (Godot):
  - `project.godot`, `Main.tscn` and `Smoke.cs` in `launch.working_directory`;
  - every `*.dll` in `<working_directory>/.godot/mono/temp/bin/Debug/`.
- sd (Qt): every `*.dll` and `*.exe` under the executable's directory, recursively. That set is wider than the identity on purpose: the identity keeps the loaded subset (section 5), and any module that may load is already protected.
- Every ancestor directory of every protected file, up to the volume root.

**Acquisition.** The guard walks each path from the volume root down, one component at a time, and holds every component it passes. Every step fails closed.
1. Path form: the path is absolute, on a local fixed drive (`GetDriveTypeW` is `DRIVE_FIXED`), with no `\\?\` prefix, no UNC form and no `.` or `..` component.
2. Each directory below the volume root:
   - Open it with `FILE_LIST_DIRECTORY`, `FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT`, and share `FILE_SHARE_READ | FILE_SHARE_WRITE` with no `FILE_SHARE_DELETE`. The directory then cannot be renamed or removed, while files inside it can still be created and renamed normally.
   - `FILE_FLAG_OPEN_REPARSE_POINT` opens the component itself and never its target.
   - On that handle, refuse if `FileAttributeTagInfo` shows `FILE_ATTRIBUTE_REPARSE_POINT`. This covers junctions, symbolic links and mounted folders.
   - Also refuse if `FileCaseSensitiveInfo` shows `FILE_CS_FLAG_CASE_SENSITIVE_DIR`. A case-sensitive directory is unsupported, and refusing it closes the same-case-fold junction case (L2-P-002-03).
3. The file itself:
   - Open it with `GENERIC_READ`, `FILE_FLAG_OPEN_REPARSE_POINT`, share `FILE_SHARE_READ` only, and `OPEN_EXISTING`. Existing write handles or writable mappings make this fail. Once the handle is held, ordinary write, delete and rename (which needs `DELETE` access) are refused.
   - The same reparse-point test applies to the file.
4. Spelling: `GetFinalPathNameByHandleW` (normalized, DOS volume name) of the file handle, without its `\\?\` prefix, must equal the requested path case-insensitively. With no reparse component and no case-sensitive directory on the path, this refuses only the remaining aliases: 8.3 components and `subst` drives.
5. Identity: `GetFileInformationByHandleEx(FileIdInfo)` gives the volume serial and the 128-bit file ID. The size comes from the same handle.
6. Digest: SHA-256 read through the held handle (`ReadFile` on it or on a duplicate), never by opening the path again.

**Ready and release.**
- After every handle is held and hashed, the guard writes `<run directory>/guard.json` and prints one `ready` line on stdout. `guard.json` holds:
  - format `magic600-l2-guard-v1`;
  - the guard PID and its process creation time;
  - `ready` (`GetSystemTimePreciseAsFileTime`, taken after the last hash);
  - one entry per file: path, final path, volume serial, file ID, size, sha256;
  - one entry per directory: path, volume serial, file ID.
- The guard then blocks on stdin. On `release` it checks again that every handle still has the recorded file ID and no reparse attribute, closes the handles and exits 0. Any other input, an end of file, or a failed check makes it exit non-zero. The handles also close when the guard dies, which the finalizer detects (section 3).
- The guard only reads. Its only write is `guard.json`.

## 3. Finalizer (`finalize_run.py`)

New checks for every mode (run, short, geometry, validation). A failure is the existing `identity` refusal, so the refusal inventory and `renderer_gate.py` stay unchanged.

1. `guard.json` is present and well formed:
   - all fields have the right types;
   - every path is absolute;
   - paths are unique (compared case-insensitively);
   - the format is `magic600-l2-guard-v1`.
2. The guard is alive and still protecting:
   - `OpenProcess` on the guard PID succeeds, its creation time equals the recorded one, and it has not exited;
   - for every guarded file, `CreateFileW` with `GENERIC_WRITE` fails with `ERROR_SHARING_VIOLATION`. The probe opens and closes a handle and never writes.
3. Order: the app's process creation time is later than the guard's `ready`. The runner passes it as a new argument, `--launched-created <FILETIME>`, read from the app's process handle while that handle is open.
4. Every identity part is guarded:
   - the parts are `dll`, every shader next to it, and every `files` entry;
   - `files.dll` is now the path of the module actually loaded (sections 4 and 5), and the shaders are read next to it, as the DLL itself does (`module_path()`). A redirected copy of the DLL is therefore an unguarded path and is refused (L2-P-002-02);
   - each recorded path, normalized and compared case-insensitively, equals one guarded path;
   - the finalizer opens the path once and checks that `FileIdInfo` on that handle matches the guard's entry;
   - it hashes through the same handle, and that digest must equal the guard's;
   - the identity uses the guard's digest, so the same bytes give the same identity as before.
5. The DLL's own reported shader and DLL digests (`dll_identity`) must still equal these digests (an existing check).
6. sa2:
   - the app records the loaded assembly's MVID (section 4);
   - the finalizer parses the MVID from the same bytes it hashed (PE, CLI header, metadata, Module table row 1, `#GUID` heap);
   - the two must be equal. The existing `FullName` check stays.
7. sd:
   - the app records its module observer state and its load events (section 5);
   - the finalizer requires that the observer registered and had no overflow;
   - it requires that the `qt:` keys equal the union of every loaded module under the executable directory (except the exe and the DLL);
   - every such module must be guarded (check 4).
8. `run.json` gets a `binding` object outside `build`:
   - the guard format and the count of guarded files and directories;
   - the sha256 of `guard.json`;
   - `"rule": "guard-before-launch"`.

   `renderer_gate.py` ignores unknown keys (`validate_run`).

## 4. Godot app (`renderer-sa2/project/Level2.cs`, `Level2Result.cs`, `Native.cs`)

- Record `assembly_mvid` = `typeof(Smoke).Assembly.ManifestModule.ModuleVersionId` (string, `D` format) next to `godot:assembly`.
- Record `files.dll` from `GetModuleFileNameW` on the handle that `NativeLibrary.Load` returned, not from the `--l2-dll` argument. A failure is refused (`framework-modules`).
- No other change. The guard covers `project.godot` and `Main.tscn` before Godot reads them (L2-V-002-03). It also covers the shaders before scene loading (L2-V-002-02).

## 5. Qt app (`renderer-sd/app/src/main.cpp`, `l2.cpp`, `l2.h`, `native_loader.cpp`, new `module_observer.cpp` and `.h`)

The observer covers the whole process lifetime in three intervals. Every in-scope module load is either in the record or makes the run fail (L2-P-002-01). A module is in scope when its path is under the executable's directory.

**Before `main`** (process start until registration):
- Modules loaded before `main` that are still loaded are in the snapshot below.
- Modules loaded and unloaded before `main` are in the loader's unload history, `RtlGetUnloadEventTraceEx`. Its entries keep only a base name of up to 32 characters and no directory, so a name cannot show that a module was out of scope.
- At registration, the observer therefore refuses (fail closed) when the history holds any entry at all. A module that was created, loaded, unloaded and deleted before registration is refused this way too (L2-P-002-01, re-check).
- Stop point: after the first build of G2, Claude runs the real `sd_smoke.exe` once with an invalid level 2 argument. That is the usage path: every static import loads, but there is no window and no GPU work, and it writes a harness record. Claude then reads the unload-history result from that record. If a normal Qt startup unloads any module before `main`, every run would refuse. Implementation then stops and the plan returns to Astra; the rule is never weakened silently.

**From `main` until the record is sealed:**
- First statement of `main`, before `QGuiApplication` and before the level 2 dispatch: register an `LdrRegisterDllNotification` callback (resolved from `ntdll` with `GetProcAddress`), check the unload history, then snapshot the loaded modules once with `EnumProcessModules`.
- A failure in any step marks the observer as failed, and `write()` records that (fail closed).
- Callback, on load and on unload:
  1. reserve a slot with an atomic increment;
  2. copy `FullDllName` and the reason into a fixed preallocated buffer, or set the overflow flag;
  3. mark the slot complete.

  It does no file I/O and no allocation under the loader lock.

**After the seal** (until process exit):
- `write()` first sets an atomic `sealed` flag, then reads the slot count, then waits for those slots to be complete.
- Any in-scope load whose callback sees `sealed` set calls `TerminateProcess(GetCurrentProcess(), 3)`. Exit 3 is never gate evidence: the runner stops and the finalizer refuses `app-exit`.
- The callback reserves its slot (step 1) before it reads `sealed`, and `write()` sets `sealed` before it reads the count. With sequentially consistent atomics in that order, every in-scope load is either counted before the seal or sees the seal. Unloads after the seal change nothing, because a loaded module is already in the union.

**The record** (`write()`):
- `modules`: the observer state, the unload-history result, overflow, and the event list (snapshot entries, then loads and unloads in order).
- The `qt:` keys are built from the union of every loaded in-scope path. The end-of-run `EnumProcessModules` is no longer used.
- Two paths with the same basename are refused, as now.
- This covers late modules, transient load-call-unload modules, and unload/reload at the same path (L2-V-002-04).
- `files.dll` comes from `GetModuleFileNameW` on the module handle `native_loader.cpp` loaded, not from `--l2-dll` (L2-P-002-02).

**Code placement:** the observer is its own source file, so a small non-GPU test program can link it (section 7), as `code_layout_test.cpp` does now.

## 6. Runner (`run_scene.ps1`)

Per run, inside the existing `try`:
1. Start `python -B file_guard.py --launch <launch.json> --candidate <c> --out <run directory>`, with stdin and stdout redirected.
2. Wait at most 60 s for the `ready` line, else stop the series.
3. Then `Start-App`. Record `$process.StartTime.ToFileTimeUtc()` while the handle is open, and pass it to the finalizer as `--launched-created`.
4. After the finalizer has exited: write `release`, wait at most 30 s, require exit 0, else stop the series.
5. The existing `finally` kills a guard that is still running.

`Assert-Idle` is unchanged, because the guard is idle while it waits. The guard starts and stops outside the PresentMon capture.

## 7. Tests

**Guard (`check_guard.py`, new, Windows):**
- Acquisition fails while another handle has write access.
- While the guard holds a file, each of these fails:
  - writing;
  - truncating;
  - deleting;
  - renaming;
  - `MoveFileEx` with `REPLACE_EXISTING` over it;
  - renaming each ancestor directory.
- After release, each of them succeeds again.
- `guard.json` digests equal an independent hash. A planted defect that hashes by reopening the path is caught: the test removes the path's read access after acquisition, and the guard must still hash.
- Each of these alias forms is refused:
  - a junction as a directory component, and a symbolic link as the leaf (when the test may create one);
  - a junction whose name differs from its target only in case;
  - a case-sensitive directory on the path (when `fsutil file setCaseSensitiveInfo` works on the test volume; otherwise "not run", with the refusal still checked against a stubbed `FileCaseSensitiveInfo` result);
  - an 8.3 component (when the volume has 8.3 names);
  - a `\\?\` prefix, a `..` component and a relative path.
- The guard exits non-zero on end of file or unknown input. A killed guard releases the files.

**Finalizer (`check_l2.py`):** every existing fixture runs with a real guard over its parts. New cases, all in the four modes:
- for each part (`godot:exe`, `godot:assembly`, `godot:project.godot`, `godot:Main.tscn`, `godot:Smoke.cs`, `dll`, each shader, `qt:exe`, one `qt:Qt6*.dll`, `qt:qwindows.dll`): release the guard, replace the part, finalize, and require exactly `identity`;
- `guard.json` missing, malformed, or with a wrong or dead PID → `identity`;
- an edited file ID, or a listed file that is not held (the write probe succeeds) → `identity`;
- live-guard digest mismatches, with the guard alive throughout:
  - an edited `sha256` in `guard.json`;
  - a DLL whose `dll_identity` digest differs from the guarded bytes;
  - a shader that the DLL reports but the guard does not list;
  - each → `identity`;
- `files.dll` at a path other than the guarded DLL, with the same bytes (a redirected copy) → `identity`;
- app created before `ready` → `identity`;
- a recorded path outside the guarded set (a copy with the same bytes) → `identity`;
- sa2: same `FullName`, different MVID (synthetic minimal .NET PE built by the test) → `identity`;
- sd: observer failed, overflow, or `qt:` keys that differ from the event union, covering a late module, a transient module and unload/reload → `identity`.
- Positive controls:
  - unchanged parts give the same identity as the old encoding for the same bytes;
  - moving the whole fixture to another directory keeps the identity;
  - scene, preroll and other run options do not change it;
  - a transient module that is guarded enters the identity.
- The static runner check also requires:
  - guard start and `ready` before `Start-App`;
  - `--launched-created`;
  - release after the finalizer;
  - the guard killed in `finally`.

**App suites (`check_project.py` in renderer-sa2 and renderer-sd):**
- The path-only assertions are replaced.
- sd:
  - registration is the first statement of `main`;
  - the unload-history check and the snapshot follow registration;
  - the callback does no I/O;
  - `write()` seals before it reads the count, and a post-seal in-scope load terminates with exit 3;
  - `write()` uses the event union;
  - `files.dll` comes from `GetModuleFileNameW` on the loaded module;
  - failure is fail-closed.
  - Planted defects:
    - registration after `QGuiApplication`;
    - the unload-history check removed;
    - the seal after the count read;
    - `write()` back on `EnumProcessModules`;
    - overflow ignored;
    - `files.dll` back on `--l2-dll`.
- sd observer test program (`module_observer_test.cpp`, built by `build.cmd`, no GPU and no Qt window). Each case runs in its own process with a probe DLL in the program's directory:
  1. the probe is loaded, called and unloaded by a global constructor before `main`: refusal. This holds also when the probe file is deleted before `main`, so no current file matches its name;
  2. a persistent load during `main`: in the union;
  3. load, call, unload during `main`: in the union;
  4. unload, then reload at the same path: in the union once;
  5. a load after the seal: exit 3;
  6. a load on a second thread racing the seal: either in the union or exit 3, over many repetitions;
  7. the event buffer overflowing: refusal.
- sa2:
  - the MVID is recorded from `ManifestModule.ModuleVersionId`;
  - `files.dll` is taken from `GetModuleFileNameW` on the `NativeLibrary.Load` handle.
  - Planted defects: the MVID missing, or taken from another assembly; `files.dll` back on the argument.
- Same-handle hashing, protection lifetime and fail-closed binding are properties of the guard and the finalizer, so `check_guard.py` and `check_l2.py` test them. The app suites test what the apps record.

**Owner-machine experiments (actual Windows):**
1. Synthetic, run by Claude without the GPU or PresentMon (`guard_experiments.py`), before the full candidate review:
   - the alias forms that `check_guard.py` cannot make portably: `subst`, a symbolic link (if allowed) and `\\localhost\` UNC;
   - an ancestor rename and a hard link;
   - release timing;
   - DLL redirection: a small loader program without a manifest, with a `.local` folder holding a copy of a probe DLL. Load the probe by its absolute path and compare the requested path with `GetModuleFileNameW` on the returned handle. The experiment shows whether redirection applies and whether the recorded actual path reveals it. Either way, the finalizer check of section 3 refuses a path that is not guarded.
   - the observer test program of the app suites, run on this machine.
2. Attended, with the owner: one short W3 run per app with a helper that tries to replace each part every second during the run, while the guard holds it. Every attempt must be refused, and the run must finalize with an identity equal to an independent pre-run hash.
3. Qt: a synthetic probe DLL in `deploy/`, loaded in three ways by a new fault-injection mode:
   - a late, persistent load;
   - load, call, unload;
   - unload, then reload at the same path.

   Each must enter the identity. A probe DLL created after `ready` must be refused.

## 8. Packets (Codex first)

| Packet | Owner | Files | Acceptance | After |
|---|---|---|---|---|
| G1 guard | Codex | `renderer-l2/file_guard.py`, `renderer-l2/check_guard.py` | `python -B work/experiments/renderer-l2/check_guard.py` | — |
| G2 Qt observer | Codex | `renderer-sd/app/src/main.cpp`, `l2.cpp`, `l2.h`, `native_loader.cpp`, new `module_observer.cpp`, `module_observer.h`, `module_observer_test.cpp`, `renderer-sd/build.cmd`, `renderer-sd/check_project.py` | `python -B work/experiments/renderer-sd/check_project.py` | — |
| G3 Godot MVID and DLL path | Codex | `renderer-sa2/project/Level2.cs`, `Level2Result.cs`, `Native.cs`, `renderer-sa2/check_project.py` | `python -B work/experiments/renderer-sa2/check_project.py` | — |
| G4 finalizer | Codex | `renderer-l2/finalize_run.py`, `renderer-l2/check_l2.py`, `renderer-l2-packets/HARNESS.md` | `python -B work/experiments/renderer-l2/check_l2.py` | G1 |
| G5 runner | Claude | `renderer-l2/run_scene.ps1` and its static check | `check_l2.py` | G4 |

- G1, G2 and G3 run in parallel. G4 needs G1's guard and the harness fields that G2 and G3 define (named in this plan).
- After integration:
  - Claude builds both apps (`build.cmd`, Godot build), which gives new build identities;
  - one Astra full review of the whole candidate;
  - the synthetic experiments.
- The attended experiments and any new gate runs are for the owner to schedule.

## 9. Not changed, and residual limits

- Not changed:
  - S-B's runner and `renderer_gate.py`;
  - the identity encoding;
  - the 4 October records, which keep Astra's wording (`RESULT.md`).
- Residual limits:
  - Files that are not identity parts are not bound and were never claimed: Godot's own .NET assemblies, Qt files that are not DLLs, and system DLLs.
  - The guard's directory handles stop renames of the folders on the path for the length of a run. That is intended.
  - Setting a reparse point on a held directory or file and removing it again within one run is outside the accidental-replacement threat model. A junction or mount point needs an empty directory, and every held directory contains a held file. A reparse point left in place makes the finalizer open a different file (file-ID mismatch) and fails the guard's release check.
