# Packet G4: finalizer binding checks (L2-V-002)

Run from the `claude/renderer-l2-followup` checkout after G1, G2 and G3 are integrated, with no gate capture or other implement call active:
`python tools/agents/codex_review.py --kind implement --model gpt-6.1-sol --effort max --timeout 7200 --packet work/experiments/renderer-l2-packets/G4-finalizer.md`

Plan: `work/experiments/renderer-l2-packets/PLAN-L2-V-002.md`, approved by Astra (plan check `20261009T161932Z-1f325e3c`, re-checks `20261009T163456Z-b3c5367e` and `20261009T164302Z-5337660e`, pass). This packet implements plan section 3, the finalizer tests of section 7 and the matching `HARNESS.md` text. The guard (`file_guard.py`, G1), the Qt `modules` record (G2) and the Godot `assembly_mvid` and `files.dll` (G3) are integrated at this worktree's HEAD; read their packets (`G1-guard.md`, `G2-qt-observer.md`, `G3-godot-binding.md`) for the contracts. G5 (the runner, `run_scene.ps1`, and its static check in `check_l2.py`) is Claude's and comes after this packet.

## 1. Goal and acceptance
- Goal: the finalizer accepts a level 2 record only when every identity part was protected by a live guard from before the app's launch until the finalizer's check, and the identity uses the digests the guard took through its held handles.
- Acceptance check: `python -B work/experiments/renderer-l2/check_l2.py` exits 0.
- Done when:
  - plan section 3, checks 1 to 8, is implemented in `finalize_run.py` for all four modes, each failure the existing `identity` refusal;
  - `--launched-created <FILETIME>` is a required finalizer argument in every mode;
  - `check_l2.py` runs every existing fixture with a real guard over its parts and has every finalizer case and positive control of plan section 7;
  - `HARNESS.md` describes the guard, `guard.json`, the binding checks, `--launched-created`, the `binding` object, the Qt `modules` object, the Godot `assembly_mvid`, the loaded `files.dll` and the three `module-*` injection values (sd only), and states the binding's limit: modules outside `scope` (system DLLs, the GPU driver) are neither recorded nor bound, so the identity covers only the build directory.
- Non-goals:
  - `file_guard.py`, the apps, `run_scene.ps1` and `check_l2.py`'s `static_runner()` (G5 extends it);
  - `tools/perf/renderer_gate.py` and everything else under `tools/`;
  - the 4 October records and `RESULT.md`.

## 2. Actual problem and reproduction
- Astra ruling `20261009T155501Z-a2cb4438`: `build_identity()` in `finalize_run.py` hashes the recorded files after the run by opening their paths, so the digests are not bound to the bytes the app loaded (L2-V-002, -02 to -05).
- Reproduction: replace an identity part between the app's exit and the finalizer, and restore it; or record a path that the app did not load. The current finalizer cannot tell.

## 3. Environment and versions
- Base: this worktree's HEAD (G1 to G3 integrated).
- Windows 11 (build 26200), CPython 3.14, standard library only.
- The acceptance check runs inside the Codex Windows sandbox: create fixture folders as `Path(tempfile.gettempdir()) / <unique name>` with a plain `mkdir()` (the sandbox refuses `TemporaryDirectory()` with WinError 5), as `check_l2.py` does now. Starting `file_guard.py` with Python is allowed.

## 4. Necessary source and evidence
- `PLAN-L2-V-002.md`: binding. Read sections 1, 3, 7 ("Finalizer") and 9.
- `G1-guard.md` section 6 and `work/experiments/renderer-l2/file_guard.py`: the command line, `guard.json`, exit codes and the public API. Import the module from the same directory; do not copy its Win32 code.
- `G2-qt-observer.md` section 6 and `work/experiments/renderer-sd/app/src/l2.cpp`: the `modules` object, `files.dll` and the `module-*` injections.
- `G3-godot-binding.md` section 6 and `work/experiments/renderer-sa2/project/Level2.cs`: `assembly_mvid` and `files.dll`.
- `finalize_run.py` (`read_harness`, `build_identity`, `finalize`, `main`), `check_l2.py`, `HARNESS.md` sections 6 to 10.
- ECMA-335 partition II, sections 24.2 (metadata root and stream headers) and 22.30 (Module table), for the MVID parser: PE optional header data directory 14 (CLI header), the metadata root, the `#~` stream (`HeapSizes` bit 0x04 gives 4-byte `#GUID` indexes) and the `#GUID` heap (1-based 16-byte entries). The MVID is formatted as .NET's `Guid.ToString("D")`, which equals `str(uuid.UUID(bytes_le=...))`.

## 5. Attempts so far
None. This is the first call.

## 6. Constraints and owned files
Owned:
- `work/experiments/renderer-l2/finalize_run.py`;
- `work/experiments/renderer-l2/check_l2.py`, except `static_runner()`;
- `work/experiments/renderer-l2-packets/HARNESS.md`.

Change nothing else. `file_guard.py`, `check_guard.py`, `run_scene.ps1`, the apps, the plan and packets, and everything under `tools/` stay byte-identical.

**Finalizer:**
- `SHADERS` comes from `file_guard` (one definition).
- `guard.json` in the run directory is an input, never an output; the finalizer never writes or removes it.
- The identity encoding is unchanged: same part names and lines; the digests are the guard's. Unchanged bytes give the same identity as before this packet.
- sd: `modules` must be `observer` `"sealed"`, `failure` `null`, `unload_history` `[]`, `overflow` `false`; `scope` equals the directory of `files["qt:exe"]` (case-insensitive); every event path is under `scope`; the `qt:` keys equal the basenames of the union of `snapshot` and `load` paths, except `qt:exe` and `files.dll`, each mapped to its path (case-insensitive). Anything else is `identity`.
- sa2: `assembly_mvid` is a 36-character lowercase `D`-format string equal to the MVID parsed from the guarded bytes of `godot:assembly`; a parse failure is `identity`.
- `files.dll` must be one guarded path, and the shaders are read next to it.
- Every record that carries `build` also carries `binding`: `{"format": "magic600-l2-guard-v1", "files": <count>, "directories": <count>, "guard_sha256": "<sha256 of guard.json bytes>", "rule": "guard-before-launch"}`.
- A `module-*` injection value is accepted only for `sd`, and only with `--fault-injection` in run mode (otherwise `operator`); the record keeps `injected_fault` as now.
- The finalizer's own opens use `file_guard.open_identity`, and it hashes through that handle (plan section 3, check 4).
- Liveness (check 2) uses the `pid` and `created` of `guard.json`: `file_guard.process_created(pid)` must equal `created`. The write probe passes only when `file_guard.write_probe(path)` returns exactly `"sharing-violation"`; any other value (`"opened"`, an access-denied or a missing-file result) is `identity`.
- `guard.json` `directories` include the drive root, spelled with its trailing backslash (`C:\`); accept that entry and compare directory paths without stripping or adding separators.
- Case-insensitive comparison only finds the guard entry for a recorded path; the `FileIdInfo` match through the finalizer's own handle (check 4) is what proves it is the same file. (Python case folding is not the NTFS upcase table: `straße.dll` and `strasse.dll` fold equal and are two files. The guard refuses two such spellings in one protected set.)
- sd events: an `unload` event whose path has no earlier `snapshot` or `load` event of the same path (case-insensitive) is `identity`. "Under `scope`" means the path starts with `scope` followed by a backslash, compared case-insensitively, so `...\deploy2\x.dll` is not under `...\deploy`.

**`check_l2.py`:**
- `fixture()` starts a real guard (`file_guard.py --file ...`, or the API in a child process) over the fixture's parts before the fixture's app creation time, passes `--launched-created` from a time after `ready`, and stops the guard on every exit, including failures.
- Every case of plan section 7, "Finalizer", and every positive control there, in the existing refusal style (exactly one reason). The synthetic minimal .NET PE for the MVID case is built by the test in memory.
- Keep every existing case, the counts printout, and `static_runner()` unchanged (G5 extends it). Keep the rules: starts no process other than Python and the existing PowerShell parser call; writes nothing in the worktree; fixture folders removed on every exit.
- Python files keep LF; `HARNESS.md` keeps its line endings. Every Python entry point disables bytecode before local imports.

```implement-contract
{"allowed_files": ["work/experiments/renderer-l2/finalize_run.py", "work/experiments/renderer-l2/check_l2.py", "work/experiments/renderer-l2-packets/HARNESS.md"], "acceptance_check": ["python", "-B", "work/experiments/renderer-l2/check_l2.py"], "stop_condition": "finalize_run.py implements plan section 3 checks 1 to 8 and the rules of packet section 6; check_l2.py runs every fixture under a real guard and covers every finalizer case and positive control of plan section 7; HARNESS.md documents the binding; the acceptance check passes"}
```

## 7. Required return format
- Implementation: changes only in the assigned worktree. The final message lists:
  - the changed files;
  - the acceptance result, with the case counts;
  - each choice left open, and the choice made;
  - every place where the plan or the G1 to G3 contracts contradict the code, what you did, and how the binding rule still holds;
  - what G5 (the runner) must pass or do for these checks to hold on the owner's machine.
