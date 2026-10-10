# Packet G1: the level 2 file guard (L2-V-002)

Run from the `claude/renderer-l2-followup` checkout, with no gate capture or other implement call active, other than the parallel G2 and G3 calls:
`python tools/agents/codex_review.py --kind implement --model gpt-6.1-sol --effort max --timeout 7200 --packet work/experiments/renderer-l2-packets/G1-guard.md`

Plan: `work/experiments/renderer-l2-packets/PLAN-L2-V-002.md`, approved by Astra (plan check `20261009T161932Z-1f325e3c`, re-checks `20261009T163456Z-b3c5367e` and `20261009T164302Z-5337660e`, pass). This packet implements plan section 2 and the guard tests of section 7. G2 (Qt app) and G3 (Godot app) run in parallel. G4 (finalizer) comes after this packet and will import this module, so the public API below is part of the contract.

## 1. Goal and acceptance
- Goal: `work/experiments/renderer-l2/file_guard.py`, a process that the level 2 runner starts before the app. It holds every identity file and every ancestor directory against replacement, hashes the files through the held handles, writes `guard.json`, prints `ready`, and keeps the protection until it reads `release`.
- Acceptance check: `python -B work/experiments/renderer-l2/check_guard.py` exits 0.
- Done when:
  - `file_guard.py` implements plan section 2 exactly (protected set, acquisition steps 1 to 6, ready and release);
  - `check_guard.py` covers the "Guard" tests of plan section 7, each with the stated "not run" fallback where the machine cannot make the alias;
  - the public API of section 6 below exists and is used by `check_guard.py`.
- Non-goals:
  - the finalizer, the runner, `HARNESS.md` and the apps (G2 to G5);
  - the owner-machine experiments of plan section 7 (`guard_experiments.py` is written later);
  - any change under `tools/`.

## 2. Actual problem and reproduction
- Astra ruling `20261009T155501Z-a2cb4438` (L2-V-002): the finalizer hashes the recorded identity files after the run, so a file replaced and restored during the run, or loaded late, is not bound to the bytes the app consumed.
- Binding rule: each recorded digest describes the exact bytes consumed, or bytes protected against replacement before their first possible consumption. The guard provides the second form: it takes the protection before the app is launched and keeps it until the finalizer has finished.
- Reproduction: `work/experiments/renderer-l2/file_guard.py` does not exist.

## 3. Environment and versions
- Base: branch `claude/renderer-l2-followup` (this worktree's HEAD).
- Windows 11 (build 26200), NTFS system volume `C:`; CPython 3.14, standard library only (`ctypes`, `hashlib`, `json`, `subprocess`).
- The acceptance check runs inside the Codex Windows sandbox. In that sandbox `tempfile.TemporaryDirectory()` and `mkdtemp()` fail with WinError 5 (CPython's owner-only ACL on `os.mkdir(..., 0o700)` excludes the sandbox SID). Create fixture folders as `Path(tempfile.gettempdir()) / <unique name>` with a plain `mkdir()`, and remove them on every exit.
- Symbolic links may need a privilege the sandbox lacks; case-sensitive directories may need a Windows feature that is off. Each such case is "not run" with its reason, never a failure and never a pass.

## 4. Necessary source and evidence
- `work/experiments/renderer-l2-packets/PLAN-L2-V-002.md`: binding. Read sections 1, 2, 7 ("Guard") and 9.
- `work/experiments/renderer-l2-packets/HARNESS.md` section 9: the `launch.json` format (`magic600-l2-launch-v1`: `candidate`, `executable`, `dll`, `working_directory`, ...).
- `work/experiments/renderer-l2/finalize_run.py`, read-only: `SHADERS` (the four shader names next to the DLL) and `file_digest`.
- `work/experiments/renderer-l2/check_l2.py`, read-only: the fixture style (`sys.dont_write_bytecode = True` first, plain `mkdir` folders).
- Win32 facts the code relies on; name each in the final message as assumed, with the test that catches it if it is wrong:
  - `CreateFileW` with `FILE_FLAG_OPEN_REPARSE_POINT` opens a reparse point itself, not its target;
  - a directory held with share `FILE_SHARE_READ | FILE_SHARE_WRITE` (no `FILE_SHARE_DELETE`) cannot be renamed or removed, while files inside it can still be created and renamed;
  - a file held with `GENERIC_READ` and share `FILE_SHARE_READ` cannot be opened for write, truncate, delete or rename, and its acquisition fails while another handle has write access or a writable mapping;
  - `GetFileInformationByHandleEx` classes `FileAttributeTagInfo` (9), `FileIdInfo` (18) and `FileCaseSensitiveInfo` (23), with `FILE_CS_FLAG_CASE_SENSITIVE_DIR` = 1;
  - `GetFinalPathNameByHandleW` with `FILE_NAME_NORMALIZED | VOLUME_NAME_DOS`.

## 5. Attempts so far
None. This is the first call.

## 6. Constraints and owned files
Owned (new):
- `work/experiments/renderer-l2/file_guard.py`;
- `work/experiments/renderer-l2/check_guard.py`.

Change nothing else. In particular `finalize_run.py`, `check_l2.py`, `run_scene.ps1`, `HARNESS.md`, the plan, the apps and everything under `tools/` stay byte-identical.

**Command line** (the runner uses the first form; tests and finalizer fixtures may use the second):
- `python -B file_guard.py --launch <launch.json> --candidate sa2|sd --out <run directory>`
- `python -B file_guard.py --file <path> [--file <path> ...] --out <run directory>`

The guard protects exactly the files named by `--file` in the second form, plus their ancestor directories. Unknown or repeated options (other than `--file`), a missing value, or both forms at once: usage error.

**Protected set** (`--launch` form): plan section 2, "Protected set". `launch.json` must have format `magic600-l2-launch-v1` and its `candidate` must equal `--candidate`. A file the set names that does not exist refuses acquisition (the shader files and the Godot project files included); the recursive Qt `*.dll`/`*.exe` set and the Godot `.godot/mono/temp/bin/Debug/*.dll` set are whatever exists at acquisition. An empty Godot assembly set refuses.

**`guard.json`** (written atomically: a temporary name in the run directory, then a rename; refuse if `guard.json` exists already):
```json
{
  "format": "magic600-l2-guard-v1",
  "pid": 1234,
  "created": 134049612345678901,
  "ready": 134049612399999999,
  "files": [{"path": "C:\\...\\sa2_interop.dll", "final_path": "C:\\...\\sa2_interop.dll",
             "volume_serial": 1234567890, "file_id": "<32 lowercase hex>", "size": 123, "sha256": "<64 lowercase hex>"}],
  "directories": [{"path": "C:\\...", "volume_serial": 1234567890, "file_id": "<32 lowercase hex>"}]
}
```
- `created` is the guard's own process creation time and `ready` is `GetSystemTimePreciseAsFileTime`, both as FILETIME integers (100 ns units since 1601, UTC). `ready` is taken after the last hash and before `guard.json` is written.
- `files` and `directories` are sorted by `path` case-insensitively, with no duplicates (case-insensitive). Paths use backslashes, as given after the path-form check (no normalization that could hide an alias).
- `file_id` is the 128-bit `FileIdInfo.FileId` as 32 lowercase hex digits, byte order as in memory.

**Output and exit codes.**
- On success: stdout gets exactly one line, `ready`, flushed, after `guard.json` is in place. The guard then reads stdin line by line.
- Exactly `release` (after stripping the line ending): re-check that every held handle still gives the recorded `FileIdInfo` and no `FILE_ATTRIBUTE_REPARSE_POINT`, close every handle, print `released`, exit 0. A failed re-check closes the handles and exits 4.
- Any other line or end of file: close every handle, exit 5.
- Acquisition refused (any step of plan section 2 fails, or a protected file is missing): no `guard.json`, one `refused: <step>: <path>` line on stderr, exit 3. Close everything already held.
- Usage error: exit 2.
- No other output on stdout. Nothing written except `guard.json` and its temporary name.

**Public API** (G4 imports these; keep the names and meanings, choose the internals):
- `SHADERS`: the four shader file names, equal to `finalize_run.SHADERS` (do not import `finalize_run`; `check_guard.py` asserts the two are equal).
- `protected_set(launch: dict, candidate: str) -> list[str]`: the file paths of the protected set, sorted and unique as in `guard.json`.
- `acquire(paths: list[str]) -> Guard`: plan section 2 steps 1 to 6 for every path and every ancestor directory; raises `GuardRefused(step, path)` and releases everything on refusal. `Guard.record() -> dict` returns the `guard.json` object without `ready`; `Guard.check() -> bool` is the release re-check; `Guard.close()` closes every handle (idempotent).
- `open_identity(path: str)`: open one file as step 3 does (same flags, share mode and reparse refusal); returns a handle object usable with the next three helpers, and as a context manager that closes it.
- `file_id(handle) -> tuple[int, str]`: `(volume_serial, file_id)` as in `guard.json`.
- `sha256_handle(handle) -> tuple[int, str]`: `(size, sha256)` read through the handle from offset 0 (a duplicate handle or explicit offsets; never by opening the path again).
- `process_created(pid: int) -> int | None`: the creation FILETIME of a running process, `None` if it cannot be opened or has exited.
- `write_probe(path: str) -> str`: open the path with `GENERIC_WRITE` and close it at once without writing; returns `"sharing-violation"` for `ERROR_SHARING_VIOLATION`, `"opened"` if the open succeeded, otherwise `"error:<code>"`.

**`check_guard.py`:**
- Implements every "Guard (`check_guard.py`, new, Windows)" case of plan section 7, through the API and through the command line (at least one full ready, release and exit cycle, the exit codes above, and `guard.json`'s fields and types).
- The case "removes the path's read access after acquisition, and the guard must still hash": make the open and the hash separable through the API (for example, acquire with hashing deferred, remove read access with a DACL change, then hash). Include a planted defect that hashes by reopening the path and show that the test catches it.
- Junctions: create them with `_winapi.CreateJunction`. Symbolic links: `os.symlink`, "not run" when it raises for privilege. Case-sensitive directories: `fsutil file setCaseSensitiveInfo <dir> enable` or the equivalent `NtSetInformationFile` call; "not run" when it fails, and then check the refusal against a stubbed `FileCaseSensitiveInfo` result. 8.3: `GetShortPathNameW`; "not run" when the fixture has no short name.
- Prints one line per case (`pass`, or `not run: <reason>`) and a final count; exit 0 only when no case failed.
- Rules: `sys.dont_write_bytecode = True` before any local import; starts no process other than Python (`file_guard.py`) and the optional `fsutil`; writes nothing in the worktree; fixture folders under the system temp directory as in section 3, removed on every exit (restore any changed ACL or case-sensitivity first). Every guard process it starts is stopped on every exit.
- Keep the existing line endings of any file you read; new Python files use LF, like `finalize_run.py` and `check_l2.py`.

```implement-contract
{"allowed_files": ["work/experiments/renderer-l2/file_guard.py", "work/experiments/renderer-l2/check_guard.py"], "acceptance_check": ["python", "-B", "work/experiments/renderer-l2/check_guard.py"], "stop_condition": "file_guard.py implements plan section 2 and the command line, guard.json, exit codes and public API of packet section 6; check_guard.py covers every guard test of plan section 7 with the stated not-run fallbacks and passes"}
```

## 7. Required return format
- Implementation: changes only in the assigned worktree. The final message lists:
  - the changed files;
  - the acceptance result, with every "not run" case and its reason;
  - each Win32 fact the code relies on, as assumed, with the test that catches it;
  - each choice the plan or this packet left open, and the choice made;
  - every place where the plan contradicts what Windows does, what you did, and how the binding rule still holds;
  - open points for G4 (the finalizer) about the API.
