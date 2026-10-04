# Plan re-check QT-P r2: install Qt 6.10.3 from pinned archives without aqt

Review only; do not perform follow-up work. This is a scoped re-check of a changed plan for a critical-path change (`tools/toolchain/*`, `tools/toolchain.lock.json`). Check only two things:
- whether the revised design resolves QT-P-01 to QT-P-06 of plan check `20261003T144341Z-c770b9b1`;
- whether the changes introduce a new `blocker` or `major`.

Nothing is implemented yet. Read the files named below in this checkout.

## 1. Goal and acceptance (unchanged except the method)
- **Goal.** `python tools/toolchain/bootstrap.py install qt-6-10-3` installs Qt 6.10.3 for MSVC 2022 x64 into `tools/qt/6.10.3/msvc2022_64` on the owner's Windows machine.
  - The content is qtbase, qtdeclarative, qtsvg and qtshadertools.
  - Every archive is checked against a size and SHA-256 pinned in the lockfile before anything reads its structure.
  - No Qt binary runs before the tree is at its final location.
- **Acceptance.**
  - All six agent-rules checks pass: `python tests/test_agent_rules_sync.py`, `python tests/test_codex_review.py`, `python tests/test_stop_gate.py`, `python tests/test_bootstrap.py`, `python tests/test_wiki_lint.py` and `python tests/test_workbench.py`. The new offline cases are listed in section 6.
  - After the owner's `bootstrap.py approve`, the install runs on the owner's machine and the probe passes. Readiness stays provisional until that actual install passes.
- **Non-goals.**
  - Other Qt versions, Linux, macOS.
  - The Qt online installer.
  - Running or patching aqt.
  - Building anything against Qt.

## 2. What changed and why (dispositions in `work/reviews/20261003T144341Z-c770b9b1/dispositions.json`, all `adopt`)
- **QT-P-01 (blocker).** aqt is dropped. bootstrap.py downloads the four archives itself and verifies each one's exact size and pinned SHA-256 before any extraction. It never fetches a checksum at install time. Nothing from the Qt tree runs until the tree has been moved to its final location, and the first execution is the probe.
- **QT-P-02 and QT-P-03 (major).** Moot: aqt is never imported or run, so its `%APPDATA%` cleanup, its log file and its fallback selection never happen. The extractor is a child `python -I -c <constant code>` that imports only py7zr.
- **QT-P-04 (major).** The downloader saves each archive under a name pinned in the lockfile, so there is no remote-name or retained-name mapping.
- **QT-P-05 (minor).**
  - No `qtenv2.bat` and no qmake patching.
  - The installer writes `bin/qt.conf` (`[Paths]` / `Prefix=..`), the relocation mechanism aqt also writes (`aqt/updater.py`, `make_qtconf`).
  - The probe also checks that `qmake -query QT_INSTALL_PREFIX` names the final directory.
  - A short required-file list is checked before the move.
- **QT-P-06 (minor).** The entry states an explicit redirect policy (section 6), and tests exercise it with mocked redirect chains.

## 3. Environment and versions
- Base: branch `claude/renderer-sb` at `329ea3b`.
- Owner machine: Windows 11, CPython 3.14.7, 64-bit.
  - `HKLM\SYSTEM\CurrentControlSet\Control\FileSystem\LongPathsEnabled` = 1, read on 3 October 2026.
  - The checkout path is 104 characters long. Some Qt member paths may exceed 260 characters in total, for example `include/QtQuickControls2FluentWinUI3StyleImpl/6.10.3/...`.
- py7zr 1.1.3 is already pinned with hashes in `tools/python/renderer-spike.txt:703` and installed in `tools/.venv/renderer-spike`. Its licence is LGPL-2.1-or-later (`py7zr-1.1.3.dist-info/METADATA`).
- Licences, from the SPDX headers at tag v6.10.3:
  - Libraries (`qrhid3d12.cpp`, `qquickrhiitem.cpp`, `qshaderbaker.cpp`, `qsvgrenderer.cpp`): `LicenseRef-Qt-Commercial OR LGPL-3.0-only OR GPL-2.0-only OR GPL-3.0-only`.
  - Tools (`qsb.cpp`): `LicenseRef-Qt-Commercial OR GPL-3.0-only WITH Qt-GPL-exception-1.0`.

## 4. Necessary source and evidence
- **`tools/toolchain/bootstrap.py`**:
  - `METHODS` (62), `load_lock` (114–128), `safe_dest` (146–157), `resolve_executable` with `{prefix}/` (164–191), `present` (194–204), `run_probe` (207–238), `EXACT_VERSION_METHODS` (244);
  - `venv_contained` (251), `venv_conflicts` (285), `probe_env` (316), `install_env` (321), `_run` (353), `APPROVED_SOURCES` (388), `install_entry` (452–507), `ledger` (577), `install` (584–626).
- **py7zr 1.1.3** (`tools/.venv/renderer-spike/Lib/site-packages/py7zr/py7zr.py`): `ArchiveFile.uncompressed` (164), `is_directory` (191), `is_file`, `is_symlink` (225), `is_junction` (235), `is_socket` (244), `needs_password` (971), `extractall` (996), and its own `check_archive_path` (1091, 1134).
- **Install path per archive.**
  - aqt reads `@TargetDir@/<path>` from each package's Extract operation (`aqt/archives.py:189–212`) and extracts into `<output>/<path>` (`aqt/installer.py:1652`).
  - The 3 October 2026 dry run printed `-> 6.10.3/msvc2022_64` for all four archives, so the archive members are relative to that directory.
- **Hash provenance.** The four SHA-256 values are the `<archive>.sha256` files that download.qt.io served directly on 3 October 2026. The byte counts are from `curl -sIL` on the same day. Archive URLs, sizes and hashes are in the first plan's section 2 (`work/experiments/renderer-sd-packets/QT-plan.md:32–42`).
- **Observed redirect.** download.qt.io answers archive requests with HTTP 302 to `https://mirrors.ukfast.co.uk/sites/qt.io/online/qtsdkrepository/...`, the same repository path under a mirror prefix.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | aqt-based install (plan r1) | none | Astra plan check `20261003T144341Z-c770b9b1` | 1 blocker, 3 major, 2 minor; all adopted |

## 6. Revised proposed change (to be re-checked)

Owned files:
- `tools/toolchain/bootstrap.py`, `tools/toolchain.lock.json`, `tests/test_bootstrap.py`;
- `.gitignore` (add `tools/qt/`);
- the E-2.4-00 Qt row (after the install).

### Lockfile
1. **New `py7zr` entry.**
   - `uv-venv-hashed`, version `1.1.3`, `requires: ["uv"]`;
   - `venv: tools/.venv/renderer-spike` and `requirements: tools/python/renderer-spike.txt`, the same as `cmake`, `ninja` and `flip`;
   - probe `["{venv}/python", "-I", "-c", "import py7zr; print(py7zr.__version__)"]`, expect `^1\.1\.3\s*$`;
   - licence `LGPL-2.1-or-later`, network hosts `pypi.org` and `files.pythonhosted.org`.

   It installs nothing new: the environment already holds these pins.
2. **`aqtinstall` entry: note only.** "Not used to install Qt: aqt runs staged binaries and fetches checksums at install time (plan check 20261003T144341Z-c770b9b1). Qt entries use archive-7z-hashed." The environment and its requirements are unchanged.
3. **New `qt-6-10-3` entry:**
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
1. **Method registration.**
   - Add `archive-7z-hashed` to `METHODS` and `EXACT_VERSION_METHODS`.
   - `load_lock` validates its fields strictly:
     - `platforms == ["windows"]`;
     - `prefix` and `root` are plain relative names without links or `..`;
     - `base_url` is `https://<host>/`, and its host is in `network_hosts`;
     - `redirects` has exactly the three keys with these types;
     - `archives` is non-empty. Each archive has a unique plain `name` ending in `.7z`, a relative `path` without `..`, `//`, `\`, query or fragment, an integer `bytes` > 0, a 64-hex `sha256`, and an `install_path` under `root`;
     - `max_unpacked_bytes` is an integer > 0;
     - `required_files` are under `root`;
     - `prefix_check` has the form shown;
     - `extractor` names a `uv-venv-hashed` entry that is listed in `requires`.
2. **Download and verify** (`install_entry`, new branch). Windows only.
   1. `final = safe_dest(prefix/root)`. Refuse if it exists (`os.path.lexists`).
   2. Staging is `tools/qt/.st-<pid>/` with a marker file `.magic600-staging`, and subdirectories `dl/` and `x/`. The names are short because of the path length above.
      - A leftover `tools/qt/.st-*` that holds the marker and has no link anywhere below is removed first.
      - Any other leftover refuses.
   3. Free space at `tools/qt` must be at least the sum of `bytes` plus `max_unpacked_bytes`; otherwise refuse before any request.
   4. Fetch the archives one at a time, in lockfile order, with the standard library only (`urllib.request`, default TLS context with certificate and hostname checks). Automatic redirects are off, and every 3xx goes through the policy below.
      - Request headers: `Accept-Encoding: identity` and a fixed `User-Agent`.
      - Only a final 200 is accepted.
      - A `Content-Length` that differs from `bytes` refuses.
      - The body is streamed to `dl/<name>.part` while it is hashed. Reading stops and refuses at `bytes`+1.
      - The body must be exactly `bytes` long, and the SHA-256 must equal the pin. Only then is the file renamed to `dl/<name>`.
      - The socket timeout is 60 s, and each archive has an overall deadline of 1800 s. There are no retries.
   5. **Redirect policy, applied before each next request:**
      - `Location` is resolved against the current URL;
      - the scheme is `https`, with no userinfo, port absent or 443;
      - the host is a DNS name with a dot: not an IP literal, not `localhost`;
      - there is no query and no fragment;
      - the path ends with `/` + the pinned `path` (exact);
      - at most `max_hops` redirects.

      The host that served the 200 is recorded, hostnames only.
   6. Any failure removes the staging directory and refuses with the archive name ("installation blocked until verified again"). No byte of an archive is parsed before its size and hash match.
3. **Extract** (only after all four archives are verified).
   1. Resolve the extractor entry's environment through `venv_contained`, and refuse on `venv_conflicts`.
   2. Run `[<venv>/Scripts/python.exe, "-I", "-c", EXTRACT_CODE, "list", <archive>]` with `probe_env()`, cwd = staging and the install timeout.
      - `EXTRACT_CODE` is a constant in bootstrap.py that imports only `json`, `sys` and `py7zr`.
      - It prints a JSON listing: name, `is_directory`, `is_file`, `is_symlink`, `is_junction`, `is_socket` and uncompressed size per member.
      - It exits 3 when `needs_password()` is true.
   3. A pure function in bootstrap.py checks the listing. A member is refused when:
      - its name is empty, has NUL or control characters, or contains `:`;
      - it is absolute, has a drive or UNC prefix, or has a leading `/` or `\`;
      - it has an empty, `.` or `..` segment, or a segment ending in `.` or a space;
      - it has a Windows device name (CON, PRN, AUX, NUL, COM1–9, LPT1–9, with or without an extension);
      - it is not exactly one of file or directory, or is a symlink, junction or socket;
      - it duplicates another name in the same archive, case-insensitively.

      The total size of all archives must not exceed `max_unpacked_bytes`, and free space must cover that total.
   4. Run the same child with `"extract", <archive>, <x>/<install_path>`.
   5. Then check the extracted tree:
      - no link or junction anywhere;
      - only regular files and directories;
      - every file is a listed member at its listed size. The later archive wins on an overlap, and overlaps are counted.
      - Nothing unlisted.
   6. Write `x/<install_path>/bin/qt.conf` = `[Paths]\r\nPrefix=..\r\n`, and record whether it replaced an existing file. Check `required_files`.
   7. Rename `x/<root>` to `final` (same volume), then remove the staging directory.

   Read-only attributes are cleared only inside the staging directory, during that removal.
4. **Probe.**
   - `install()` then calls `run_probe`. That is the first execution of any Qt file.
   - For this method `run_probe` also runs `prefix_check.command` and requires its output, after `normpath`/`normcase`, to equal `safe_dest(prefix_check.path)`. `doctor` and the present-check path use the same check.
   - A failing probe leaves the tree in place, and `install()` then refuses replacement without the owner, as for every other method.
5. **Ledger.** The method appends one record: tool, archive names, bytes, serving hosts, seconds, the number of overlapping members and the longest final path length. `install()` writes its usual record.
6. **`.gitignore`.** Add `tools/qt/`.

### Tests (offline, `tests/test_bootstrap.py`)
- **`load_lock`:** each malformed field is refused.
- **Download, with a fake transport.**
  - Accepted: a direct 200, and a two-hop permitted redirect whose final host is recorded.
  - Refused: http, userinfo, port 8443, an IP literal, `localhost`, a query, a path mismatch, six hops, a non-200 final status, a `Content-Length` mismatch, a short body and a long body (reading stops at `bytes`+1).
  - **Hash mismatch.** The bytes match a simulated server-side checksum but not the lockfile pin. The install is refused, the staging directory is removed, and neither the extractor nor any probe is called.
- **Member checks:** each rule above, and the size cap.
- **Install flow, with fake transport and fake extractor:**
  - On success, `qt.conf` is written, the tree is moved, staging is removed and nothing runs from staging. Every `subprocess` call is asserted: only the extractor child before the move, and the probe after it.
  - Refused: a link in the extracted tree, an unlisted file, a size mismatch, an existing destination, too little free space (before any request), a leftover staging directory without the marker, and a missing required file.
  - A leftover staging directory with the marker is removed.
- **`run_probe`:**
  - The exact version is required (`6.10.3rc1` fails).
  - A `QT_INSTALL_PREFIX` that differs from the final directory fails.
- **Real extractor child.** Runs against a tiny archive created by the renderer-spike environment's py7zr, if that environment exists; otherwise the case is skipped.
- **`.gitignore`** lists `tools/qt/`.

### Questions for the re-check
1. Does the order of download and verification, listing and member checks, extraction, tree check, `qt.conf`, required files, move and probe resolve QT-P-01? Is anything executed or parsed before verification or before the move?
2. Is the redirect policy explicit and testable enough to resolve QT-P-06 for the owner's approval? Or should the entry pin `https://master.qt.io/` with redirects refused? That host's no-redirect behaviour has not been verified.
3. Is a py7zr child process (`python -I`, the renderer-spike environment, constant code) a sound extractor once the bytes match the pin? Do the member checks and the tree check cover what `extractall` can do?
4. Do the `qt.conf` write, the `QT_INSTALL_PREFIX` check and the required-file list resolve QT-P-05?
5. Is a new `blocker` or `major` introduced anywhere else? Areas to check: the budgets (exact download bytes, the unpacked cap, free-space checks), the licence statement, the `py7zr` entry sharing the environment, the long-path situation, and approval (bootstrap.py and the lockfile are already approval inputs, so `bootstrap.py approve` is needed again).

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`, with findings limited to the scope above, each with severity and evidence (`path:line`). Answer each question in the summary or as a finding.
- Review only; do not perform follow-up work.
