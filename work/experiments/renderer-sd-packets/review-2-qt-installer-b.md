# Scoped verification packet: QT-I installer fixes, shard B (QT-B-01, QT-B-02, QT-B-03)

Review only; do not perform follow-up work.

Run concurrently with shard A (`review-2-qt-installer-a.md`):
`python tools/agents/codex_review.py --kind review --model gpt-6.1-sol --effort max --speed fast --packet work/experiments/renderer-sd-packets/review-2-qt-installer-b.md`

## 1. Goal and acceptance
- Goal: the first scoped verification round after the full review of the `archive-7z-hashed` installer (QT-I). Astra shard B, call `20261003T163024Z-1c9a1c6e`, reported three `major` findings: QT-B-01, QT-B-02 and QT-B-03. Claude adopted all three and fixed them. This is a critical path (`tools/toolchain/*`).
- Acceptance: one JSON result matching `schemas/review-result.schema.json` that answers only two things:
  1. Is each of QT-B-01, QT-B-02 and QT-B-03 fixed in the current files? If one is not, report it again under its own ID, with a counterexample against the current code.
  2. Does the fix introduce a new `blocker` or `major`? Report only those, each with a concrete counterexample (file-system state or call sequence, then the wrong outcome).
- Verdict `pass` when every finding is fixed and nothing new reaches `major`.

## 2. Actual problem and reproduction
The findings, in short. The full text is in `work/reviews/20261003T163024Z-1c9a1c6e/review.json`.
- **QT-B-01.** Archive probes accepted executable links into staging. The resolver confined `{prefix}/` executables to `tools/qt`, not to the final version tree, and a link on `qmake.exe` itself was not refused.
- **QT-B-02.** Failures after the move left the new final tree installed. Both an archives-ledger `PermissionError` and a failed prefix probe left `tools/qt/6.10.3` in place.
- **QT-B-03.** Cleanup did not reject hard-linked files. `_archive_tree` accepted regular files with more than one link, and the read-only retry in cleanup could change the attributes of a hard-linked file outside staging.

## 3. Environment and versions
- Windows 11, CPython 3.14.7 64-bit. `bootstrap.py` uses the standard library only.
- The review sandbox is read-only, CPU only, without network.

## 4. Necessary source and evidence
- Fix-only delta: `work/experiments/renderer-sd-packets/qt-i-fix-delta.patch`.
  - It runs from the reviewed files (`tools/toolchain/bootstrap.py` sha256 `d9d4fea8…14af`, `tests/test_bootstrap.py` sha256 `1930559e…5009`, the digests in the findings' evidence) to the current files (`754fa54e…eff58` and `a65face2…a22b`).
  - `install_archives` gained an outer `try`, so its body moved one level in; the statements did not change.
- Current files: `tools/toolchain/bootstrap.py` and `tests/test_bootstrap.py`. `git diff HEAD -- <file>` shows the whole uncommitted QT-I change.
- The fixes:
  - **QT-B-01.** For a `{prefix}/` executable of an `archive-7z-hashed` entry, `resolve_executable` makes two changes:
    - The containment root is now the final tree, `safe_dest(prefix + "/" + root)`. That call already refuses when the final root itself is a link or junction.
    - Every candidate that exists must pass `safe_dest` on its full repository-relative path, which refuses any link component, the executable included. The refusal reads "executable passes through a link".
    - `run_probe` turns that `Refused` into `(False, "refused: …")` before any subprocess runs, and `present` returns False. Other methods are unchanged.
  - **QT-B-02.**
    - `install_archives` sets `moved` after `os.rename(tree/root, final)`. If anything after the rename fails (the staging cleanup or the archives ledger append), its new outer `except BaseException` removes `final` and re-raises.
    - `install()` wraps `run_probe` and the install ledger. On any failure of an `archive-7z-hashed` entry it removes the final tree. Reaching that point means `install_entry` returned, and `install_archives` refuses an existing destination before changing anything, so this attempt created the tree.
    - A destination that existed before is refused with "destination already exists" and never removed.
  - **QT-B-03.**
    - `_archive_tree` refuses a regular file with `st_nlink > 1` ("extracted tree contains a hard-linked file"). It reads the count with `os.lstat`, because a `scandir` entry reports `st_nlink` 0 on Windows. Marked leftovers go through `_archive_tree` before removal, so a leftover holding a hard link is refused as "unexpected leftover …" and kept.
    - `_remove_staging` is renamed `_remove_tree` and is now also used for the rollback above. It refuses when the top directory is a link. Its read-only retry raises `Refused` instead of calling `chmod` on any non-directory with `st_nlink > 1`.
  - Native facts measured on this machine (Windows 11, NTFS, CPython 3.14.7): `os.unlink` of a read-only file fails with WinError 5; `shutil.rmtree` then calls `onexc(os.unlink, …)`; `DirEntry.stat(follow_symlinks=False).st_nlink` is 0 for a file whose `os.lstat().st_nlink` is 2.
- New tests in `tests/test_bootstrap.py`:
  - `test_probe_refuses_a_final_root_junction_into_staging`. A real junction (`_winapi.CreateJunction`) points from the final root to a copy under `.st-old/x/6.10.3`. The probe is refused, `present` is False, and no subprocess runs. This test ran here.
  - `test_probe_refuses_a_linked_qmake`. A file symlink on `qmake.exe` points into staging. It was skipped here because this account has no symlink privilege.
  - `test_failures_after_the_move_remove_the_new_tree`. It runs `install()` twice: once with an archives-ledger `PermissionError`, and once with a probe that reports another prefix. After each, the final tree and staging are absent, and the install record says `probe-failed`.
  - `test_existing_destination_survives_a_failed_install`. A pre-existing final directory with an owner file is refused, and the owner file is kept.
  - `test_marked_leftover_with_a_hard_link_is_preserved`. A marked `.st-old` holds a hard link to a read-only sentinel outside it. It is refused before any request; the sentinel's mode, link count and bytes are unchanged.
  - `test_cleanup_never_makes_a_hard_linked_file_writable`. `_remove_tree` meets a read-only hard link. On Windows it raises `Refused`, and the sentinel stays read-only and unchanged.
- Checks on the current files (owner's machine, Windows 11, CPython 3.14.7):
  - `python tests/test_bootstrap.py`: 92 OK, 11 skipped. Every skip says "symlinks not available".
  - `tests/test_agent_rules_sync.py`: 11 OK.
  - `tests/test_codex_review.py`: 14 OK.
  - `tests/test_stop_gate.py`: 22 OK, 2 skipped.
  - `tests/test_wiki_lint.py`: 6 OK, and `tools/wiki/lint.py`: ok.
  - `tests/test_workbench.py`: 247 OK, 5 skipped.
  - The lockfile is unchanged since the full review.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | QT-I as implemented by Sol (`20261003T160429Z-0ff22f98`) | full review, two Astra shards | `20261003T163023Z-e2d3e15b`, `20261003T163024Z-1c9a1c6e` | 6 major (QT-A-01 to 03, QT-B-01 to 03) |
| 2 | the six fixes in section 4 here and in shard A | Claude-authored fixes | the checks in section 4 | all pass; this scoped round |

## 6. Constraints and owned files
- Read-only review. Never run `bootstrap.py install`, `install-skill`, `pin` or `approve`, and do not use the network. Running `tests/test_bootstrap.py` is allowed, because its fixtures stay under `work/`.
- In scope: QT-B-01, QT-B-02 and QT-B-03, and any new `blocker` or `major` introduced by their fixes in `resolve_executable`, `_archive_tree`, `_remove_tree`, `install_archives`, `install` and the tests named above.
- Out of scope:
  - shard A's findings and fixes (`_DeadlineSocket`, `_http_get`, `_header_values`, `fetch_archive`, `validate_archive_entry`);
  - code the full review already covered and these fixes did not touch;
  - `minor` and `nit` findings, and style.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`. Use the finding IDs QT-B-01 to QT-B-03 for a finding that is not fixed, and new IDs from QT-B-04 for new findings.
- Review only; do not perform follow-up work.
