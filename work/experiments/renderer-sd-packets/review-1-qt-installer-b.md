# Review packet: the `archive-7z-hashed` installer method and the Qt 6.10.3 entry (QT-I), shard B: listing, extraction, tree checks, move and probe

Review only; do not perform follow-up work.

Run from the `claude/renderer-sb` checkout, concurrently with shard A (`review-1-qt-installer-a.md`):
`python tools/agents/codex_review.py --kind review --model gpt-6-astra --effort max --packet work/experiments/renderer-sd-packets/review-1-qt-installer-b.md`

## 1. Goal and acceptance
- Goal: the full review, by the other Codex model, of a Sol-authored critical-path change (`tools/toolchain/*`, `tools/toolchain.lock.json`). Shard B covers the extractor child, the member checks, `install_archives` from staging to the ledger, and the probe with its prefix check; shard A covers the transport, the download policy, the pins and the lockfile.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. Report only `blocker` or `major` findings, each with a concrete counterexample (the archive listing, file tree or call sequence, then the wrong outcome). Wrong outcomes that count:
  - **Code from the archives runs early.** Any file from an archive is executed, loaded or imported before the move into the final location (step 10), or the probe or prefix check runs from anywhere but the final tree.
  - **A write outside the target.** Extraction or any step writes, renames or deletes outside the staging directory and the final directory, including through a member name, a link, a junction, a reparse point, case folding, an 8.3 short name or a Windows device name.
  - **A member check that misses.** A listing that violates a rule of the packet's `member_problems` list but passes, or a tree that differs from the listings (an extra, missing or resized file, a link, a non-regular file, or anything but `root` at the top of `x/`) but passes.
  - **A resource hazard.** Free space or `max_unpacked_bytes` checks that can be bypassed, or an extract child that can run without a timeout.
  - **A destructive cleanup.** Cleanup, the leftover-staging removal or the read-only clearing can remove or modify anything that is not a staging directory this method made (marker present, no links below it).
  - **A wrong install.** On success, `qt.conf` differs from `b"[Paths]\r\nPrefix=..\r\n"`, a required file is missing, the final tree is not at `tools/qt/6.10.3`, or staging remains. On any failure, the final directory exists.
  - **A probe that passes wrongly.** `run_probe` passes for a version other than 6.10.3 or for a `QT_INSTALL_PREFIX` naming another directory.
  - **A regression.** Any other method's `install_entry` or `run_probe` behaviour changes.
  - **A hollow test.** A test passes although the rule it names is broken.
- Verdict `pass` if there is none.

## 2. Actual problem and reproduction
- Specification: packet `work/experiments/renderer-sd-packets/QT-I-installer.md` (sections 1 and 6 are binding). It implements plan r2 after Astra plan check `20261003T144341Z-c770b9b1` and Astra re-check `20261003T151320Z-b183ce70`.
- Sol implement call `20261003T160429Z-0ff22f98` implemented it. Its patch is `work/reviews/20261003T160429Z-0ff22f98/changes.patch` and its report `report.md` in the same folder. It is a re-run: the first call, `20261003T152454Z-012350e6`, was invalid because an existing test wrote the real install ledger, and the packet was amended to isolate the ledger (its section 5, row 3). Claude reviewed and applied the patch (section 4 lists the integrator changes).

## 3. Environment and versions
- Windows 11 (`LongPathsEnabled` = 1), CPython 3.14.7 64-bit; py7zr 1.1.3 in `tools/.venv/renderer-spike`, run only as a child process with `-I -c EXTRACT_CODE`.
- The review sandbox is read-only, CPU only, without network.

## 4. Necessary source and evidence
- Files under review: `tools/toolchain/bootstrap.py`, `tests/test_bootstrap.py` — uncommitted changes against `HEAD`; `git diff HEAD -- <file>` shows them. The lockfile and `.gitignore` are shard A's, but read the `qt-6-10-3` entry for `root`, `install_path`, `required_files`, `probe` and `prefix_check`.
- py7zr 1.1.3 facts the packet relies on: `ArchiveFile.filename`, `uncompressed`, `is_directory`, `is_file`, `is_symlink`, `is_junction`, `is_socket`; `SevenZipFile.files`, `needs_password()`, `extractall(path=...)`.
- Integrator changes after applying the patch:
  1. **Line endings only.** The packet's section 3 wrongly said `tools/toolchain/bootstrap.py` and `tests/test_bootstrap.py` are CRLF. At `HEAD` both are LF, so the patch rewrote each whole file in CRLF. Claude converted every CRLF in those two files to LF and changed nothing else (neither file has a lone CR). The lockfile and `.gitignore` are byte-identical to the patch. `git diff HEAD` therefore shows only the real change: 1,221 insertions and 13 deletions over the four files.
  2. **Wiki evidence digest.** `docs/wiki/concepts/build-identity-v2.md` cites `tools/toolchain.lock.json` as evidence `engine-environment`. Its sha256 now matches the new lockfile, as the wiki lint requires. The engine entry itself is unchanged.
  3. **Outside the diff.** This checkout held CRLF copies of 377 LF-committed files from before `* -text`. `python tools/checkout_bytes.py --fix` restored the committed bytes before the checks below; `git status` is unchanged by it.
  4. **Base.** `HEAD` is now `d53f5d7`, one commit after the patch's base `329ea3b`. That commit adds the SA2 producer DLL under `work/experiments/renderer-sa2/` and touches none of the files under review.
  5. **Claude's review of the Codex change** found no blocker or major. These minors and nits are deferred; do not report them again:
     - `member_problems` ignores its `install_path` argument;
     - an exception raised by the `finally` cleanup in `install_archives` would replace the original refusal;
     - a `bin/qt.conf` directory inside an archive would raise `OSError` rather than `Refused`. The install still fails closed.
- Integrator's checks on the final source (owner's machine, Windows 11, CPython 3.14.7 64-bit):
  - `git apply --cached` of `changes.patch` onto the base in a scratch index reproduces the implement worktree's four files exactly.
  - `python tests/test_bootstrap.py`: 82 tests OK, 10 skipped. All 10 skips say "symlinks not available", because this account has no symlink privilege.
    - The real extractor child test and the real metadata-probe test ran against `tools/.venv/renderer-spike` (py7zr 1.1.3, aqtinstall 3.3.0) and passed.
    - The junction test ran and passed.
    - In the implement sandbox: 70 passed, 12 skipped. The two real-environment tests skipped there.
  - Three full runs of that suite left no file under `work/` new, removed or changed. This was checked against a manifest of every path, size and modification time, with `work/worktrees/` excluded.
  - The real lockfile loads and round-trips identically (`json.dumps(lock, indent=2) + "\n"`).
    - Its `py7zr` and `qt-6-10-3` entries equal the packet's JSON blocks (compared as parsed objects).
    - The `aqtinstall` entry differs from `HEAD` only in `probe`, `expect` and `note`.
  - Other agent-rules checks:
    - `python tests/test_agent_rules_sync.py`: 11 OK;
    - `python tests/test_codex_review.py`: 14 OK;
    - `python tests/test_stop_gate.py`: 22 OK, 2 skipped;
    - `python tests/test_wiki_lint.py`: 6 OK, and `python tools/wiki/lint.py`: ok;
    - `python tests/test_workbench.py`: 247 OK, 5 skipped.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | aqt-based install | plan only | Astra plan check | rejected (1 blocker, 3 major) |
| 2 | own downloader, pinned verification, py7zr child | plan r2 | Astra scoped re-check | QT-P-01, -03, -04 closed; QT-P-02 adopted |
| 3 | the packet is implementable as written | implement call `20261003T152454Z-012350e6` | the wrapper's integrity check | invalid: the existing test suite appended a fake record to the real install ledger |
| 4 | the amended packet (ledger isolated in every test) | implement call `20261003T160429Z-0ff22f98` | acceptance check; Claude's patch review; the checks in section 4 | valid; acceptance passed; no blocker or major in Claude's review |

## 6. Constraints and owned files
- Read-only review. Never run `bootstrap.py install`, `install-skill`, `pin` or `approve`; do not use the network.
- Out of scope: `_http_get`, `fetch_archive`, `load_lock` and the lockfile values (shard A); the choice of Qt 6.10.3 and of these four modules; the licence; style, `minor` and `nit` findings.
- Questions:
  1. **Extraction boundary.** Given that the listing checks pass, can py7zr's `extractall` still write outside `x/<install_path>` (for example through members py7zr normalises differently from `member_problems`, or through overlapping archives)? Does the tree check catch it after the fact if it does?
  2. **Tree check.** Is the walk free of link following, and are the file set and size comparisons exact (case-insensitive set, last-listing size, overlaps counted)?
  3. **Staging and cleanup.** Can the leftover removal, the `finally` cleanup or the read-only clearing touch anything outside a staging directory this method made?
  4. **Order.** Is it impossible for any archive file to run before step 10, and do the probe and the prefix check resolve only to the final tree?
  5. **Probe.** Do the version and prefix checks refuse every mismatch, including a prefix that differs only in case, separators or a trailing separator in a way that names another directory?
  6. **Regression.** Do the other methods' `install_entry` and `run_probe` paths behave exactly as before?

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
