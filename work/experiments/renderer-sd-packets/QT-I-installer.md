# Implementation QT-I: the `archive-7z-hashed` install method and the Qt 6.10.3 lockfile entry

You implement this packet in your assigned worktree only. Do not commit, push, tag, or change Git configuration, refs or hooks. Do not use the network. Never run `bootstrap.py install`, `install-skill`, `pin` or `approve`, and never download anything.

**Starting point.** A previous call (`20261003T152454Z-012350e6`) implemented this packet but was invalid because the test suite wrote the real install ledger (section 5, row 3). Its four files are a draft you may start from: `../../reviews/20261003T152454Z-012350e6/invalid-run-files/` relative to your worktree root (`tools/toolchain/bootstrap.py`, `tools/toolchain.lock.json`, `tests/test_bootstrap.py`, `.gitignore`). Read them; do not write there. Copy them into your worktree, then check every item of section 6 against them and fix what differs. They are unreviewed. If you cannot read them, implement from this packet.

## 1. Goal and acceptance
- **Goal.** `python tools/toolchain/bootstrap.py install qt-6-10-3` will install Qt 6.10.3 for MSVC 2022 x64 into `tools/qt/6.10.3/msvc2022_64` on the owner's Windows machine. The content is qtbase, qtdeclarative, qtsvg and qtshadertools.
  1. The installer downloads four pinned archives itself and checks each one's exact size and SHA-256 against the lockfile before any byte is parsed.
  2. py7zr (run as a child process) lists each archive's members, and bootstrap.py checks them.
  3. py7zr extracts each archive into a staging directory.
  4. bootstrap.py checks the extracted tree, writes `bin/qt.conf`, checks the required files and moves the tree to its final location.
  5. Only then does anything from the tree run: the probe.
- **Acceptance check (sandbox).** `python tests/test_bootstrap.py` passes. The integrator then runs all six agent-rules checks locally.
- **Non-goals.**
  - Running or importing aqt.
  - Other Qt versions, Linux or macOS.
  - Changing `approval_inputs.json`, `install_guard.py`, any other method's behaviour, or the requirements files.

## 2. Actual problem and reproduction
- The installer has no method for Qt (`METHODS`, `tools/toolchain/bootstrap.py:62`).
- An aqt-based plan was rejected by plan check `20261003T144341Z-c770b9b1` (Astra). aqt's updater runs the staged `qmake` before any pinned hash check (`aqt/updater.py:314`, `:99`), and it fetches checksums from the server.
- This packet implements the revised plan (`QT-plan-r2.md`, not in your worktree) as amended by its Astra re-check `20261003T151320Z-b183ce70`. Everything you need from that plan is in section 6 below.

## 3. Environment and versions
- Base: the committed HEAD of branch `claude/renderer-sb`.
- The owner machine: Windows 11, CPython 3.14.7 64-bit, and `LongPathsEnabled` = 1. Qt member paths can exceed 260 characters under the checkout.
- py7zr 1.1.3 is pinned in `tools/python/renderer-spike.txt:703` and installed in `tools/.venv/renderer-spike` on the owner machine. The environment is not in your worktree, so tests that need it must skip.
- Line endings are kept per file:
  - `tools/toolchain/bootstrap.py` and `tests/test_bootstrap.py` are CRLF;
  - `tools/toolchain.lock.json` and `.gitignore` are LF.

  The repository has `* -text`, so write bytes or use `newline=""`.
- The lockfile is exactly `json.dumps(lock, indent=2) + "\n"`. Keep it round-trip identical after your edit.

## 4. Necessary source and evidence
- **`tools/toolchain/bootstrap.py`**:
  - `METHODS` (62), `load_lock` (114–128), `safe_dest` (146–157), `resolve_executable` (164–191), `present` (194–204), `run_probe` (207–238), `EXACT_VERSION_METHODS` (244);
  - `venv_contained` (251), `venv_conflicts` (285), `probe_env` (316), `install_env` (321), `_run` (353), `install_entry` (452–507), `ledger` (577), `install` (584–626), `doctor` (714).
- **`tests/test_bootstrap.py`**: tests replace module attributes and restore them in `finally` (for example `test_winget_install_skips_dependencies_and_uses_the_hashed_selection`, line 150). `test_lockfile_loads_and_is_consistent` (33) must keep passing with the new entries.
- **py7zr 1.1.3** (`py7zr/py7zr.py`), `ArchiveFile` members:
  - `filename`, `uncompressed` (164), `is_directory` (191), `is_file`, `is_symlink` (225), `is_junction` (235), `is_socket` (244);
  - `SevenZipFile.files`, `needs_password()` (971), `extractall(path=...)` (996).
- **Archive layout.** Each Qt archive extracts into `6.10.3/msvc2022_64` (`@TargetDir@/6.10.3/msvc2022_64` in the repository's Extract operation; `aqt/archives.py:189–212`, `aqt/installer.py:1652`).

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | aqt-based install | plan only | Astra plan check | rejected (1 blocker, 3 major) |
| 2 | own downloader, pinned verification, py7zr child | plan r2 | Astra scoped re-check | QT-P-01, -03 and -04 closed; QT-P-02 (major) still partly open: the `aqtinstall` probe `aqt version` still runs from `doctor` and `install --profile renderer-spike`, and importing aqt cleans `%APPDATA%/aqt/tmp` and may write `aqtinstall.log`. Adopted: lockfile item 2 replaces that probe with a metadata query that never imports aqt. |
| 3 | this packet, as written | implement call `20261003T152454Z-012350e6` | the wrapper's integrity check | invalid: `work/loop-memory/ledgers/installs.jsonl` changed. Cause: the existing test `test_shared_environment_sync_never_replaces_another_tool` (`tests/test_bootstrap.py`, HEAD lines 201–226) ends with a successful `bootstrap.install(lock, "py-spy", set())` without replacing `bootstrap.ledger`, so every run of the suite appends a fake py-spy record to the real ledger. The call's own work was otherwise complete (69 passed, 12 skipped); its new tests already replaced `bootstrap.ledger`. |

## 6. Constraints and owned files

### Lockfile (`tools/toolchain.lock.json`)
1. **Insert a `py7zr` entry after `aqtinstall`:**
```json
{"id": "py7zr", "purpose": "7z extractor for pinned archive installs (archive-7z-hashed)", "profiles": ["renderer-spike"], "method": "uv-venv-hashed", "status": "verified", "installable": true, "platforms": ["windows", "linux", "macos"], "version": "1.1.3", "requires": ["uv"], "venv": "tools/.venv/renderer-spike", "requirements": "tools/python/renderer-spike.txt", "probe": ["{venv}/python", "-I", "-c", "import py7zr; print(py7zr.__version__)"], "expect": "^1\\.1\\.3\\s*$", "license": "LGPL-2.1-or-later", "network_hosts": ["pypi.org", "files.pythonhosted.org"]}
```
2. **Change the `aqtinstall` entry** in two fields only (closes QT-P-02):
   - `note`: "Not used to install Qt: aqt runs staged binaries and fetches checksums at install time (plan check 20261003T144341Z-c770b9b1). Qt entries use archive-7z-hashed; this environment provides their py7zr extractor. The probe reads package metadata and never imports aqt, because importing aqt cleans %APPDATA%/aqt/tmp and configures logging (re-check 20261003T151320Z-b183ce70)."
   - `probe`: `["{venv}/python", "-I", "-c", "import importlib.metadata as m; print(m.version('aqtinstall'))"]`, and `expect`: `"^3\\.3\\.0\\s*$"` (JSON-escaped as written).

   No other field changes, and no code path may launch aqt.
3. **Insert the `qt-6-10-3` entry after `py7zr`**, with exactly these fields, values, archive order and hashes:
```json
{
  "id": "qt-6-10-3",
  "purpose": "Qt 6.10.3 for MSVC 2022 x64 (qtbase, qtdeclarative, qtsvg, qtshadertools) for the S-D Qt Quick candidate",
  "profiles": ["renderer-spike"],
  "method": "archive-7z-hashed",
  "status": "verified",
  "installable": true,
  "platforms": ["windows"],
  "version": "6.10.3",
  "requires": ["py7zr"],
  "extractor": "py7zr",
  "prefix": "tools/qt",
  "root": "6.10.3",
  "base_url": "https://download.qt.io/",
  "redirects": {"max_hops": 5, "scheme": "https", "same_path": true},
  "archives": [
    {"name": "qtbase.7z", "path": "online/qtsdkrepository/windows_x86/desktop/qt6_6103/qt6_6103/qt.qt6.6103.win64_msvc2022_64/6.10.3-0-202603310407qtbase-Windows-Windows_11_24H2-MSVC2022-Windows-Windows_11_24H2-X86_64.7z", "bytes": 42748757, "sha256": "4db84dee7fe3c558f242bef0a88852613af76580dc6d2b24596479f47004dad7", "install_path": "6.10.3/msvc2022_64"},
    {"name": "qtshadertools.7z", "path": "online/qtsdkrepository/windows_x86/desktop/qt6_6103/qt6_6103/qt.qt6.6103.addons.qtshadertools.win64_msvc2022_64/6.10.3-0-202603310407qtshadertools-Windows-Windows_11_24H2-MSVC2022-Windows-Windows_11_24H2-X86_64.7z", "bytes": 3074558, "sha256": "8ef195016e371e5f685a721d89956b04dbf2b78f1bc18d5bff6821c2a7f2c061", "install_path": "6.10.3/msvc2022_64"},
    {"name": "qtsvg.7z", "path": "online/qtsdkrepository/windows_x86/desktop/qt6_6103/qt6_6103/qt.qt6.6103.win64_msvc2022_64/6.10.3-0-202603310407qtsvg-Windows-Windows_11_24H2-MSVC2022-Windows-Windows_11_24H2-X86_64.7z", "bytes": 668085, "sha256": "fa5bb334b6b95582f87481a9f54819dc066e8cefed7f4fe9a21a77719630ba04", "install_path": "6.10.3/msvc2022_64"},
    {"name": "qtdeclarative.7z", "path": "online/qtsdkrepository/windows_x86/desktop/qt6_6103/qt6_6103/qt.qt6.6103.win64_msvc2022_64/6.10.3-0-202603310407qtdeclarative-Windows-Windows_11_24H2-MSVC2022-Windows-Windows_11_24H2-X86_64.7z", "bytes": 157478290, "sha256": "7dad584e12dc7641d9940e0ab30fa86092eda3f1c59ee7b5e4a02550986be35c", "install_path": "6.10.3/msvc2022_64"}
  ],
  "max_unpacked_bytes": 4294967296,
  "required_files": ["6.10.3/msvc2022_64/bin/qmake.exe", "6.10.3/msvc2022_64/bin/Qt6Core.dll", "6.10.3/msvc2022_64/bin/Qt6Gui.dll", "6.10.3/msvc2022_64/bin/Qt6Quick.dll", "6.10.3/msvc2022_64/bin/Qt6Svg.dll", "6.10.3/msvc2022_64/bin/Qt6ShaderTools.dll", "6.10.3/msvc2022_64/plugins/platforms/qwindows.dll", "6.10.3/msvc2022_64/lib/cmake/Qt6/Qt6Config.cmake"],
  "probe": ["{prefix}/6.10.3/msvc2022_64/bin/qmake", "-query", "QT_VERSION"],
  "expect": "^6\\.10\\.3\\s*$",
  "prefix_check": {"command": ["{prefix}/6.10.3/msvc2022_64/bin/qmake", "-query", "QT_INSTALL_PREFIX"], "path": "{prefix}/6.10.3/msvc2022_64"},
  "license": "LGPL-3.0-only (libraries, dynamic linking); tools GPL-3.0-only WITH Qt-GPL-exception-1.0",
  "network_hosts": ["download.qt.io"],
  "note": "Archive requests to download.qt.io are redirected to the Qt mirror network; any HTTPS mirror serving the same repository path is accepted (redirects policy), and integrity rests on the pinned bytes and sha256 checked before extraction. Download 203,969,690 bytes. tools/qt is never committed or redistributed; a redistributed build must carry Qt's licence texts."
}
```

### bootstrap.py
1. **`METHODS` and `EXACT_VERSION_METHODS`** gain `archive-7z-hashed`.
2. **`load_lock`** validates every `archive-7z-hashed` entry and exits with `lockfile: <id> ...` on any violation:
   - `platforms == ["windows"]`;
   - `prefix` and `root` are relative, forward-slash, with no `..`, drive or empty segment;
   - `base_url` matches `https://<host>/`, and the host is in `network_hosts`;
   - `redirects` is exactly `{"max_hops": int 0..10, "scheme": "https", "same_path": true}`;
   - `archives` is a non-empty list. Each archive is exactly `{name, path, bytes, sha256, install_path}`:
     - `name` is unique and matches `^[a-z0-9][a-z0-9._-]*\.7z$`;
     - `path` is relative, made of `[A-Za-z0-9._-]` segments separated by `/`, with no `..`, query or fragment;
     - `bytes` is an int > 0;
     - `sha256` matches `^[0-9a-f]{64}$`;
     - `install_path` equals `root` or starts with `root + "/"`.
   - `max_unpacked_bytes` is an int > 0;
   - `required_files` is a non-empty list of paths under `root`;
   - `prefix_check` is exactly `{command: list[str] whose first item starts with "{prefix}/", path: str starting with "{prefix}/"}`;
   - `extractor` names a `uv-venv-hashed` entry that is listed in `requires`.
3. **Transport seam.** A module-level `_http_get(url: str, timeout: float)` returns `(status: int, headers: Mapping[str, str], stream)`, where `stream.read(n)` returns bytes and `stream.close()` exists.
   - It uses `urllib.request` with `ssl.create_default_context()`. Environment proxies are honoured, as transport only.
   - Automatic redirects are off: a 3xx is returned, not followed.
   - Request headers: `Accept-Encoding: identity` and `User-Agent: magic600-bootstrap`.
   - Tests replace `_http_get`.
4. **`fetch_archive(entry, archive, dest_dir) -> str`** (returns the serving host).
   - **Start** at `base_url + path`. Loop:
     - On 301, 302, 303, 307 or 308, resolve `Location` against the current URL and check the redirect policy before the next request:
       - the scheme is `https`;
       - no userinfo; the port is absent or 443;
       - the host is a DNS name containing a dot, and is not an IP literal (`ipaddress`), not `localhost`, not ending in `.localhost`;
       - no query and no fragment;
       - the path ends with `"/" + path`;
       - at most `max_hops` redirects.
     - Any other non-200 status refuses.
   - **On 200:**
     - a present `Content-Length` must equal `bytes`;
     - stream into `dest_dir/<name>.part` in chunks while hashing with SHA-256;
     - refuse as soon as more than `bytes` arrive;
     - require exactly `bytes`, then compare the SHA-256 with the pin and rename to `dest_dir/<name>`.
   - The socket timeout is 60 s, and the per-archive deadline is `INSTALL_TIMEOUT`. There are no retries.
   - Every refusal names the archive and says "installation blocked until verified again". Never print a URL query.
5. **`EXTRACT_CODE`** is a constant string run as `[<extractor venv>/Scripts/python.exe, "-I", "-c", EXTRACT_CODE, mode, archive, (target)]`, with `env=probe_env()`, `cwd=<staging>`, `timeout=INSTALL_TIMEOUT` and captured output.
   - It imports only `json`, `sys` and `py7zr`.
   - It exits 3 when `needs_password()` is true.
   - `list` prints a JSON array: `name`, `dir`, `file`, `symlink`, `junction`, `socket` and `size` (the uncompressed size, 0 when None).
   - `extract` runs `extractall(path=target)`.
   - Resolve the interpreter through the extractor entry: `safe_dest(venv)`, `venv_contained`, and refuse on `venv_conflicts`.
   - A non-zero exit or unparsable output refuses with the archive name.
6. **`member_problems(listing, install_path) -> list[str]`**, a pure function. `/` and `\` are both separators. A member is refused when:
   - its name is empty or not a str;
   - it contains NUL, a control character, or any of `<>:"|?*`;
   - it has a leading separator, a drive or a UNC prefix;
   - it has an empty, `.` or `..` segment, or a segment ending in `.` or a space;
   - it has a Windows device name segment (CON, PRN, AUX, NUL, COM1–9, LPT1–9; case-insensitive, with or without an extension);
   - it is not exactly one of `dir` or `file`, or `symlink`, `junction` or `socket` is true;
   - its name duplicates another in the listing, case-insensitively after normalising separators.

   Return all problems, capped at 20.
7. **`install_archives(entry)`**, the new `install_entry` branch (Windows only):
   1. `prefix = safe_dest(entry["prefix"])` and `final = safe_dest(f"{prefix}/{root}")`. Refuse if `os.path.lexists(final)`. Create `prefix` if needed.
   2. **Leftover staging.** For each `prefix/.st-*`:
      - if it is a real directory (not a link), holds the marker file `.magic600-staging`, and nothing below it is a link, remove it;
      - otherwise refuse ("unexpected leftover <name>; remove it or ask the owner").
   3. **Free space.** Refuse unless `shutil.disk_usage(prefix).free >= sum(bytes) + max_unpacked_bytes`. This happens before any request.
   4. **Staging.** Create `prefix/.st-<pid>`, write the marker, and create `dl/` and `x/`.
   5. **Download.** Fetch every archive in lockfile order into `dl/`.
   6. **List and check.** For every archive, run the `list` child and `member_problems`. The sum of file sizes over all archives must be ≤ `max_unpacked_bytes`, and free space must cover that sum.
   7. **Extract.** For every archive in order, run the `extract` child into `x/<install_path>`.
   8. **Tree check.** Walk `x/` without following links:
      - no symlink or junction (use `_is_link`), and only regular files and directories;
      - the file set equals the union of `install_path/member` for listed files, compared case-insensitively;
      - each size equals the last listing that contained the file. Count the overlaps.
      - `x/` contains only `root`.
   9. **qt.conf.** Write `x/<install_path of the first archive>/bin/qt.conf` as bytes `b"[Paths]\r\nPrefix=..\r\n"`, and record whether it replaced a file. Every `required_files` path must be a regular file under `x/`.
   10. **Move.** `os.rename(x/<root>, final)`.
   11. **Cleanup.** Remove the staging directory in `finally`, on success and on every failure. Read-only attributes are cleared only for paths inside the staging directory.
   12. **Ledger.** Append one record: `{"tool", "phase": "archives", "archives": [{"name", "bytes", "host"}], "overlaps", "longest_path", "qt_conf", "seconds"}`. `longest_path` is the length of the longest final absolute path.

   No file from the archives may execute before step 10. `install()` then runs `run_probe` as for every method.
8. **`run_probe`.** For `archive-7z-hashed`, after the existing checks pass:
   - run `prefix_check.command` (placeholders resolved as in the probe) with `probe_env()` and `PROBE_TIMEOUT`;
   - require `os.path.normcase(os.path.normpath(output.strip()))` to equal `os.path.normcase(str(safe_dest(path without "{prefix}/" joined to prefix)))`. On a mismatch, return `(False, "prefix check failed: ...")`.

   `doctor` and `install` use `run_probe` unchanged.
9. **`.gitignore`.** Add `tools/qt/` after `tools/.venv/`.
10. **Docstring.** Keep the module docstring accurate.

### Tests (`tests/test_bootstrap.py`, offline, no network, no real `tools/qt`)
Use synthetic entries with `prefix` under a unique repository directory, for example `work/test-bootstrap-<pid>/qt`, created and removed by the test. `safe_dest` requires paths inside the repository.

- **No test writes the real ledger** (row 3 of section 5). `setUpModule` points `bootstrap.LEDGER_PATH` at a file in a unique test directory and `tearDownModule` restores it and removes that directory. `test_shared_environment_sync_never_replaces_another_tool` also replaces `bootstrap.ledger` and restores it, like the new tests. A test asserts that `bootstrap.LEDGER_PATH` differs from `ROOT / "work" / "loop-memory" / "ledgers" / "installs.jsonl"` while the suite runs. After `python tests/test_bootstrap.py`, no file under `work/` may be new or changed: every test removes the files it creates, and a directory the sandbox refuses to remove must contain no files.

- **`load_lock` validation.** Each malformed field is refused (a lockfile copy written to a temporary path, with `bootstrap.LOCK_PATH` swapped and restored). The real lockfile loads, with `py7zr` and `qt-6-10-3` present.
- **Download policy, with a fake `_http_get`.**
  - Accepted, with the host recorded: a direct 200, and a two-hop redirect to `https://mirror.example.org/sites/qt.io/<path>`.
  - Refused:
    - redirects to `http://`, to a URL with userinfo, to port 8443, to `https://127.0.0.1/...`, to `https://localhost/...`, with a query, or with a different path;
    - `max_hops + 1` redirects;
    - a 404 or a 206;
    - a `Content-Length` mismatch;
    - a short body;
    - a long body (assert that reading stops after `bytes + 1`).
  - **Hash mismatch.** A body that matches a fake "server checksum" but not the pin is refused. Assert that the staging directory is gone and that neither the extractor nor `subprocess.run` was called.
- **`member_problems`.** One case per rule, plus a clean listing with nested paths.
- **`install_archives` flow,** with a fake `_http_get`, fake listing and extract children (`bootstrap.subprocess.run` replaced, writing files into the target), and a fake `shutil.disk_usage`.
  - **Success.**
    - `qt.conf` has the exact bytes;
    - the final tree exists and staging is removed;
    - every `subprocess.run` call before the move ran the extractor interpreter with `-I -c EXTRACT_CODE`;
    - no call ran a path under the staging directory or the final tree.
  - **Refused:**
    - a symlink in the extracted tree, if symlinks are available (otherwise skip);
    - an unlisted extracted file;
    - a size mismatch;
    - an existing destination;
    - too little free space, before any `_http_get` call;
    - a leftover `.st-*` without the marker;
    - a missing required file;
    - extractor exit 3.
  - **Leftover staging with the marker** is removed.
- **`run_probe`, with a fake `subprocess.run`.**
  - `6.10.3` passes; `6.10.3rc1` fails.
  - A `QT_INSTALL_PREFIX` naming another directory fails.
  - The probe and the prefix check run only from the final tree.
- **Real extractor child.** If `tools/.venv/renderer-spike/Scripts/python.exe` exists and imports py7zr:
  - build a tiny archive with it in the test directory;
  - check that `list` and `extract` work and that `member_problems` is empty.

  Otherwise skip with a reason.
- **aqt is never launched (QT-P-02).**
  - Lockfile: no entry's probe or `prefix_check` names an `aqt` executable, uses `-m aqt`, or passes code that imports aqt; the `aqtinstall` probe is exactly the metadata query of lockfile item 2.
  - Routing: with `resolve_executable` returning a fake path for every name and a recording `subprocess.run` (returning a failed `CompletedProcess` with string output), run `doctor` and `install --profile renderer-spike` (with `install_entry` and `venv_conflicts` replaced, and `install`'s refusals caught). No recorded command may launch aqt in any of those forms.
  - Real metadata probe: if `tools/.venv/renderer-spike/Scripts/python.exe` exists, run `run_probe` on the real `aqtinstall` entry with `APPDATA` set to a test directory holding `aqt/tmp/sentinel`, `LOG_CFG` set to a test file, and the working directory changed to a test directory (all restored afterwards). It must pass; the sentinel must remain unchanged, and no `aqtinstall.log` may appear. Otherwise skip with a reason.
- **`.gitignore`** contains the line `tools/qt/`.
- **Every test restores the module attributes it replaces** and removes its directories.

### Files that must not change
`tools/toolchain/approval_inputs.json`, `.claude/hooks/install_guard.py`, `tools/repo_digest.py`, `tools/python/*`, `tools/skills/*`, every file outside `allowed_files`.

```implement-contract
{"allowed_files": ["tools/toolchain/bootstrap.py", "tools/toolchain.lock.json", "tests/test_bootstrap.py", ".gitignore"], "acceptance_check": ["python", "tests/test_bootstrap.py"], "stop_condition": "the archive-7z-hashed method, the py7zr and qt-6-10-3 lockfile entries, the aqtinstall note and metadata probe, the .gitignore line and the offline tests listed in section 6 are implemented, the test suite cannot write the real install ledger, other methods behave as before, and tests/test_bootstrap.py passes"}
```

## 7. Required return format
- Changes only in the assigned worktree. The final message lists:
  - the changed files;
  - the acceptance result;
  - every test that skipped and why;
  - open points.
- Do not commit.
