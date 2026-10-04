# Plan check QT-P: a reviewed install method and lockfile entry for Qt 6.10.3 (MSVC x64)

Review only; do not perform follow-up work. This is a plan check of a critical-path change (`tools/toolchain/*`, `tools/toolchain.lock.json`). Read the files named below in this checkout; nothing is implemented yet.

## 1. Goal and acceptance
- **Goal.** `python tools/toolchain/bootstrap.py install qt-6-10-3` installs Qt 6.10.3 for MSVC 2022 x64 into `tools/qt/` on the owner's Windows machine. The archives are qtbase, qtdeclarative and qtsvg, plus the qtshadertools module.
  - Every archive is checked against a SHA-256 pinned in the lockfile.
  - The installation becomes usable only after that check.
  - It follows the installer's existing rules: allowlist, approval, no links, refuse instead of replace, ledger record.
- **Why.** The S-D smoke test (packet E-2.4-03, level 1) needs Qt Quick with the QRhi Direct3D 12 backend. The owner decided on 3 October 2026 that the Qt smoke test starts after the Qt lockfile entry has been reviewed and approved.
- **Acceptance.**
  - The agent-rules checks pass, among them new `tests/test_bootstrap.py` cases for the method: the hash mismatch blocks, an extra or missing archive blocks, links are refused, a leftover staging directory is handled, the exact aqt command line is used, and the probe is exact.
  - After the owner's `bootstrap.py approve`, the install runs on the owner's machine and the probe prints `6.10.3`.
- **Non-goals.**
  - Other Qt versions.
  - Linux or macOS.
  - The Qt online installer.
  - Changing aqtinstall's version.
  - Building anything against Qt.

## 2. Actual problem and reproduction
- **No method installs Qt.** The installer has no method for Qt (`METHODS` in `tools/toolchain/bootstrap.py:62`). The `aqtinstall` entry (`tools/toolchain.lock.json`) notes: "Qt itself is downloaded by aqt from download.qt.io; each Qt version is added as its own entry before use."
- **aqtinstall 3.3.0 cannot install Qt 6.11.0 or later** (observed 3 October 2026).
  - From 6.11.0 the repository splits packages per architecture: `online/qtsdkrepository/windows_x86/desktop/qt6_6113/qt6_6113_msvc2022_64/Updates.xml`.
  - aqt 3.3.0 requests `.../qt6_6113/qt6_6113/Updates.xml` and stops with `ERROR : Failed to locate XML data for Qt version '6.11.3'`.
  - Qt 6.8.x to 6.10.x keep the layout aqt 3.3.0 reads (`qt6_6103/qt6_6103/`).
- **Chosen version: Qt 6.10.3, the newest the pinned aqt can install.** It meets the S-D requirements:
  - `QQuickRhiItem` (6.7 or later);
  - the QRhi Direct3D 12 backend, `QRhiD3D12NativeHandles` with device and command-queue import, and `QQuickGraphicsDevice::fromRhi` (6.6 or later).

  Moving to 6.11 or later would need a newer aqtinstall, which replaces an installed tool version and is an owner decision.
- **Dry run.** `aqt install-qt windows desktop 6.10.3 win64_msvc2022_64 --archives qtbase qtdeclarative qtsvg -m qtshadertools --dry-run` lists exactly four archives (prefix `online/qtsdkrepository/windows_x86/desktop/qt6_6103/qt6_6103/`):

  | Archive | Package directory | Bytes | Published SHA-256 (`<archive>.sha256` on download.qt.io) |
  |---|---|---|---|
  | `6.10.3-0-202603310407qtbase-Windows-Windows_11_24H2-MSVC2022-Windows-Windows_11_24H2-X86_64.7z` | `qt.qt6.6103.win64_msvc2022_64/` | 42748757 | `4db84dee7fe3c558f242bef0a88852613af76580dc6d2b24596479f47004dad7` |
  | `6.10.3-0-202603310407qtdeclarative-Windows-Windows_11_24H2-MSVC2022-Windows-Windows_11_24H2-X86_64.7z` | `qt.qt6.6103.win64_msvc2022_64/` | 157478290 | `7dad584e12dc7641d9940e0ab30fa86092eda3f1c59ee7b5e4a02550986be35c` |
  | `6.10.3-0-202603310407qtsvg-Windows-Windows_11_24H2-MSVC2022-Windows-Windows_11_24H2-X86_64.7z` | `qt.qt6.6103.win64_msvc2022_64/` | 668085 | `fa5bb334b6b95582f87481a9f54819dc066e8cefed7f4fe9a21a77719630ba04` |
  | `6.10.3-0-202603310407qtshadertools-Windows-Windows_11_24H2-MSVC2022-Windows-Windows_11_24H2-X86_64.7z` | `qt.qt6.6103.addons.qtshadertools.win64_msvc2022_64/` | 3074558 | `8ef195016e371e5f685a721d89956b04dbf2b78f1bc18d5bff6821c2a7f2c061` |

  The total download is about 204 MB.
- **Archive requests are redirected.** download.qt.io answers them with HTTP 302 to a mirror chosen by location (observed: `mirrors.ukfast.co.uk/sites/qt.io/...`). `Updates.xml` and the `.sha256` files come from download.qt.io itself; aqt fetches hashes only from its `trusted_mirrors`.

## 3. Environment and versions
- Base: branch `claude/renderer-sb` at `329ea3b`.
- Machine: Windows 11 (owner's machine), CPython 3.14.7 (the installer runs on the owner's Python).
- aqtinstall 3.3.0 in `tools/.venv/renderer-spike` (entry `aqtinstall`, method `uv-venv-hashed`). Its CLI (`aqt install-qt --help`) offers:
  - `-O OUTPUTDIR`, `-b BASE`, `--timeout`, `--internal` (py7zr extractor), `-k/--keep`, `-d ARCHIVE_DEST`;
  - `--dry-run`, `--UNSAFE-ignore-hash`, `-m MODULES`, `--archives ARCHIVES`, `--noarchives`, `--autodesktop`;
  - global `-c CONFIG` (settings.ini).

  Its default settings have `[mirrors] trusted_mirrors = https://download.qt.io`, a list of 15 `fallbacks` mirrors, `hash_algorithm = sha256` and `INSECURE_NOT_FOR_PRODUCTION_ignore_hash = False`.
- Evidence kind here: source and command output only.

## 4. Necessary source and evidence
- **`tools/toolchain/bootstrap.py`**:
  - `METHODS` and `load_lock` (lines 62, 114–128);
  - `safe_dest` (146), `resolve_executable` with `{prefix}/` (164–191), `present` (194), `run_probe` and `EXACT_VERSION_METHODS` (207–244);
  - `install_env` (321), `_run` (353), `install_entry` (452–510), `install` (584–626), `ledger` (577).
- **`tools/toolchain.lock.json`**: the `aqtinstall`, `cmake` and `godot` entries show the entry style.
- **`tools/toolchain/approval_inputs.json`**: bootstrap.py and the lockfile are approval inputs, so this change needs the owner's `bootstrap.py approve`.
- **`.claude/hooks/install_guard.py`**: allows an exact `bootstrap.py install <id>` only for the approved revision.
- **`tests/test_bootstrap.py`**: the existing method tests monkeypatch `PLATFORM`, `_run`, `shutil.which` and `winget_show`.
- **`.gitignore`**: has `tools/.venv/` but no `tools/qt/`.
- **`docs/progress/1.0/packets/renderer/E-2.4-00-readiness.md`**, line 40: the Qt row.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | aqt 3.3.0 installs the newest Qt | none (dry run) | `aqt install-qt ... 6.11.3 ... --dry-run` | refused: new per-architecture layout |
| 2 | aqt 3.3.0 installs 6.10.3 | none (dry run) | dry run with the archive selection above | four archives listed |

## 6. Proposed change (to be checked)

Owned files:
- `tools/toolchain/bootstrap.py`, `tools/toolchain.lock.json`, `tests/test_bootstrap.py`;
- `.gitignore` (add `tools/qt/`);
- the E-2.4-00 Qt row.

### Lockfile entry

```json
{
  "id": "qt-6-10-3",
  "purpose": "Qt 6.10.3 for MSVC 2022 x64 (Qt Quick, QRhi Direct3D 12, shader tools) for the S-D Qt Quick candidate",
  "profiles": ["renderer-spike"],
  "method": "aqt-qt",
  "status": "verified",
  "installable": true,
  "platforms": ["windows"],
  "version": "6.10.3",
  "requires": ["aqtinstall"],
  "prefix": "tools/qt",
  "qt": {"host": "windows", "target": "desktop", "arch": "win64_msvc2022_64", "archives": ["qtbase", "qtdeclarative", "qtsvg"], "modules": ["qtshadertools"]},
  "base_url": "https://download.qt.io/",
  "archive_sha256": {"<the four archive names>": "<the four hashes above>"},
  "probe": ["{prefix}/6.10.3/msvc2022_64/bin/qmake", "-query", "QT_VERSION"],
  "license": "LGPL-3.0-only (dynamic linking)",
  "network_hosts": ["download.qt.io"],
  "note": "aqtinstall 3.3.0 reads the repository layout up to Qt 6.10; archive downloads are redirected by download.qt.io to its mirror network and verified against archive_sha256"
}
```

### bootstrap.py changes
1. Add `aqt-qt` to `METHODS` and to `EXACT_VERSION_METHODS`. `qmake -query QT_VERSION` prints only the version.
2. `load_lock` validates `aqt-qt` entries strictly:
   - `platforms == ["windows"]`, and `requires` contains `aqtinstall`;
   - `prefix` is exactly `tools/qt`, and `base_url` is exactly `https://download.qt.io/`, whose host is in `network_hosts`;
   - `qt.host` is `windows` and `qt.target` is `desktop`;
   - `qt.arch` matches `^win64_msvc20\d\d_64$`;
   - `qt.archives` and `qt.modules` are lists of `^[a-z0-9_]+$`, and `qt.archives` is not empty;
   - `version` matches `^\d+\.\d+\.\d+$`;
   - `archive_sha256` is a non-empty map from a plain file name (no separators, ending in `.7z`) to 64 lower-case hex digits.
3. `install_entry` for `aqt-qt`:
   1. Windows only. Resolve aqt as `{venv}/aqt` of the `aqtinstall` entry through `resolve_executable`, which keeps it inside that venv.
   2. Staging and archive directories:
      - `staging = safe_dest("tools/qt/.staging")` and `archives = safe_dest("tools/qt/.archives")`;
      - a leftover of either is removed first, after checking that it and everything below it are no links;
      - `tools/qt/<version>` must not exist (`install()` already refuses a present but failing tool).
   3. Write a settings.ini into a fresh temporary directory. Built from constants in bootstrap.py, it holds:
      - `baseurl = https://download.qt.io`, `concurrency = 4`;
      - `[requests]` defaults, with `hash_algorithm = sha256` and `INSECURE_NOT_FOR_PRODUCTION_ignore_hash = False`;
      - `trusted_mirrors = https://download.qt.io`, and `fallbacks` and `blacklist` empty.
   4. Run, through `_run` with `install_env()`:
      `[aqt, "-c", settings, "install-qt", "windows", "desktop", version, arch, "--archives", *archives, "-m", *modules, "-O", staging, "-b", base_url, "--internal", "-k", "-d", archives_dir]`
   5. Verify before use:
      - the file names in `archives_dir` equal the keys of `archive_sha256` exactly;
      - each file's SHA-256 equals its pinned value;
      - `staging` holds exactly `<version>/<arch without "win64_">/` (plus any aqt metadata file listed in the implementation), and no link anywhere.

      On any failure, remove `staging` and `archives_dir` and refuse ("installation blocked until verified again").
   6. Move `staging/<version>` to `tools/qt/<version>`, remove `staging` and `archives_dir`, and let `install()` run the probe and write the ledger.
4. `.gitignore`: `tools/qt/`.
5. Tests, without network:
   - a fake `_run` that writes archives and a tree;
   - mismatch, extra file and missing file;
   - a link in staging and a leftover staging directory;
   - the exact argv, including that no `--UNSAFE-ignore-hash` or fallback can appear;
   - `load_lock` rejecting each malformed field;
   - the probe parsing `6.10.3`.

### Questions for the plan check
1. **Extract, then verify.** aqt extracts each archive as it downloads it, so the pinned hashes are checked after extraction, in a staging directory nothing runs from. Is staging plus verify plus move acceptable? Or must the installer download and verify the four archives itself before any extraction, and then extract or install them without aqt? Is relying on aqt's own trusted-mirror check acceptable?
2. **Redirects.** Accept download.qt.io's redirects to its mirror network, given the pinned hashes and `network_hosts: ["download.qt.io"]`? Or pin one base host, which the owner then approves as a new source? Name what the entry must state either way.
3. **aqt metadata.** Does aqt 3.3.0 write files outside `-O`/`-d`, for example a log file in the working directory or settings under the user profile? How should the installer contain them? The working directory would be a fresh temporary directory.
4. **Probe and readiness.** Is `qmake -query QT_VERSION` a sound probe and presence test for this layout, or is `qtpaths --qt-version` better?
5. **Anything else** under AGENTS.md "Tools, skills and outputs" or "Ask the owner" that this plan misses: disk and network budget (about 204 MB down, about 1 GB on disk), licence notice, the approval flow.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`. Answer each question in section 6 as a finding or in the summary, with severity and evidence (`path:line`).
- Review only; do not perform follow-up work.
