# Packet G3: Godot assembly MVID and loaded-DLL path (L2-V-002)

Run from the `claude/renderer-l2-followup` checkout, with no gate capture or other implement call active, other than the parallel G1 and G2 calls:
`python tools/agents/codex_review.py --kind implement --model gpt-6.1-sol --effort max --timeout 7200 --packet work/experiments/renderer-l2-packets/G3-godot-binding.md`

Plan: `work/experiments/renderer-l2-packets/PLAN-L2-V-002.md`, approved by Astra (plan check `20261009T161932Z-1f325e3c`, re-checks `20261009T163456Z-b3c5367e` and `20261009T164302Z-5337660e`, pass). This packet implements plan section 4 and the sa2 part of the app-suite tests of section 7. G1 (guard) and G2 (Qt) run in parallel. G4 (finalizer) comes later and reads the `harness.json` fields defined in section 6 below.

## 1. Goal and acceptance
- Goal: the Godot level 2 app records the MVID of the project assembly it actually loaded, and the path of the native DLL module it actually loaded.
- Acceptance check: `python -B work/experiments/renderer-sa2/check_project.py` exits 0.
- Done when:
  - plan section 4 is implemented;
  - `check_project.py` has the sa2 checks and planted defects of plan section 7, and every existing check still passes or is updated where plan section 4 replaces the behaviour it pinned;
  - `README.md` mentions the two new record facts.
- Non-goals:
  - the guard, the finalizer, the runner and `HARNESS.md` (G1, G4, G5);
  - the DLL, its header, the Qt app, `prepare_l2.py` and the level 1 smoke mode;
  - building or starting Godot, .NET or a GPU process in the sandbox.

## 2. Actual problem and reproduction
- Astra ruling `20261009T155501Z-a2cb4438`, L2-V-002: the finalizer hashes `godot:assembly` from the path the app reports (`ProjectAssemblyPath`, which checks `FullName` only), so it does not show that those bytes are the assembly Godot loaded. The MVID ties the loaded module to the hashed bytes: the finalizer (G4) will parse the MVID from the bytes it hashes and compare.
- Plan check finding L2-P-002-02: `files.dll` is the `--l2-dll` argument (`Level2Result` constructor), not the module `NativeLibrary.Load` loaded.
- Reproduction: read `Level2Result.cs` (`files["dll"] = a.dll_path`), `Level2.cs` (`_result.files["godot:assembly"] = ProjectAssemblyPath(...)`) and `Native.cs` (`_library = NativeLibrary.Load(absolutePath)`).

## 3. Environment and versions
- Base: branch `claude/renderer-l2-followup` (this worktree's HEAD).
- Owner's machine: Windows 11 (build 26200); Godot 4.7.2 .NET (`4.7.2.stable.mono.official.ed1daf0bf`); .NET SDK as recorded in `work/experiments/renderer-sa2/README.md`.
- The sandbox has no Godot, no .NET build and no GPU; Python 3.14, standard library only. Evidence in the sandbox is source/fixture only.

## 4. Necessary source and evidence
- `work/experiments/renderer-l2-packets/PLAN-L2-V-002.md`: binding. Read sections 1, 3 (checks 4 and 6), 4 and 7 ("App suites", sa2).
- `work/experiments/renderer-l2-packets/HARNESS.md`, read-only: section 6 (`harness.json`).
- The app: `work/experiments/renderer-sa2/project/Level2.cs`, `Level2Result.cs`, `Native.cs` (read the other `.cs` files as needed, read-only), `work/experiments/renderer-sa2/check_project.py`, `README.md`.
- .NET facts the code relies on; list each in the final message as assumed, with the runtime check that catches it:
  - `Module.ModuleVersionId` returns the MVID of the module's metadata (Module table row 1, `#GUID` heap);
  - `Guid.ToString("D")` gives 36 lowercase characters;
  - `NativeLibrary.Load` returns the `HMODULE` on Windows, usable with `GetModuleFileNameW`.

## 5. Attempts so far
None. This is the first call.

## 6. Constraints and owned files
Owned:
- `work/experiments/renderer-sa2/project/Level2.cs`, `Level2Result.cs`, `Native.cs`;
- `work/experiments/renderer-sa2/check_project.py`, `work/experiments/renderer-sa2/README.md`.

Change nothing else. All other files under `work/experiments/renderer-sa2/` (the DLL, its header, `Smoke.cs`, `project.godot`, `Main.tscn`, the other `.cs` files, `prepare_l2.py`, `run_smoke.py`, `RESULT.md`, `results/`), and everything under `work/experiments/renderer-sd/`, `work/experiments/renderer-l2/`, `work/experiments/renderer-l2-packets/` and `tools/` stay byte-identical.

**`harness.json` (sa2, level 2):**
- New top-level field `assembly_mvid`: `typeof(Smoke).Assembly.ManifestModule.ModuleVersionId.ToString("D")`, recorded where `godot:assembly` is recorded. `null` until then, and `null` in the usage record.
- `files.dll`: `null` until `new Native(...)` has returned; then the full path from `GetModuleFileNameW` on the handle `NativeLibrary.Load` returned (exposed by `Native`, for example a read-only property). A failure of `GetModuleFileNameW` (zero, or a truncated path) fails the run with reason `framework-modules`. The `--l2-dll` argument is never copied into `files`.
- No other field changes. `options`, `configuration` and every other key keep their names and values.

**`check_project.py`:**
- Replaces the path-only assertions that pin `files["dll"] = a.dll_path` and adds the sa2 checks of plan section 7: the MVID comes from `typeof(Smoke).Assembly.ManifestModule.ModuleVersionId` and is written as `assembly_mvid`; `files.dll` comes from `GetModuleFileNameW` on the `NativeLibrary.Load` handle, after the load, and a failure gives `framework-modules`.
- Planted defects, in the existing style: the MVID missing; the MVID taken from another assembly (for example `typeof(Native).Assembly` replaced by a type of another assembly such as `typeof(Godot.Node)`); `files.dll` back on the argument.
- Keeps every other existing check and its rules: builds nothing, starts no process other than Python, writes nothing in the worktree, fixture folders under the system temp directory created as `Path(tempfile.gettempdir()) / <unique name>` with a plain `mkdir()` (the sandbox refuses `TemporaryDirectory()` with WinError 5), removed on every exit.
- Keep the existing line endings of every file you change (the `.cs` files and `check_project.py` are LF). Every Python entry point disables bytecode before local imports.

```implement-contract
{"allowed_files": ["work/experiments/renderer-sa2/project/Level2.cs", "work/experiments/renderer-sa2/project/Level2Result.cs", "work/experiments/renderer-sa2/project/Native.cs", "work/experiments/renderer-sa2/check_project.py", "work/experiments/renderer-sa2/README.md"], "acceptance_check": ["python", "-B", "work/experiments/renderer-sa2/check_project.py"], "stop_condition": "the Godot app records assembly_mvid and files.dll as packet section 6 says; check_project.py has the sa2 checks and planted defects of plan section 7 and passes"}
```

## 7. Required return format
- Implementation: changes only in the assigned worktree. The final message lists:
  - the changed files;
  - the acceptance result;
  - each .NET and Win32 fact the code relies on, as assumed, with the runtime check that catches it;
  - what was not verified in the sandbox (every Godot, .NET and GPU step);
  - each choice left open, and the choice made;
  - open points for G4 about the two fields.
