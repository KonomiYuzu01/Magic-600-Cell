# Review packet: the `archive-7z-hashed` installer method and the Qt 6.10.3 entry (QT-I), shard A: download, pins and lockfile

Review only; do not perform follow-up work.

Run from the `claude/renderer-sb` checkout, concurrently with shard B (`review-1-qt-installer-b.md`):
`python tools/agents/codex_review.py --kind review --model gpt-6-astra --effort max --packet work/experiments/renderer-sd-packets/review-1-qt-installer-a.md`

## 1. Goal and acceptance
- Goal: the full review, by the other Codex model, of a Sol-authored critical-path change (`tools/toolchain/*`, `tools/toolchain.lock.json`). Shard A covers the transport, the download policy, the pins and the lockfile; shard B covers listing, extraction, the tree checks, the move and the probe.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. Report only `blocker` or `major` findings, each with a concrete counterexample (the server responses, lockfile text or call sequence, then the wrong outcome). Wrong outcomes that count:
  - **An unverified byte parsed.** Any byte of an archive reaches py7zr, a file system path or any parser before its exact size and SHA-256 match the lockfile.
  - **A policy escape.** A request goes to a URL the policy refuses: a scheme other than `https`, userinfo, a port other than 443, an IP literal, `localhost` or `*.localhost`, a host without a dot, a query or fragment, a path that does not end with `"/" + path`, or more than `max_hops` redirects. Or a redirect is followed automatically.
  - **A partial or oversized download kept.** A short, long or interrupted body, a `Content-Length` mismatch or a non-200 final status ends with a file named `<name>` (not `.part`) or with anything left in staging after the refusal.
  - **A lockfile that loads but is unsafe.** An `archive-7z-hashed` entry that violates a rule of the packet's `load_lock` list but loads.
  - **Wrong pins.** The `qt-6-10-3`, `py7zr` or `aqtinstall` entries differ from the packet's section 6 items 1 to 3 in any field, value, archive order or hash.
  - **aqt launched.** Any code path of `doctor`, `check`, `install <id>` or `install --profile <p>` launches aqt (`aqt` executable, `-m aqt`, or code importing aqt).
  - **A regression.** Any other method's behaviour changes, or the existing approval inputs, install guard or requirements files change.
  - **A leak.** An output, ledger record or error message carries a URL query, credentials or a private path beyond the repository-relative ones the ledger already records.
  - **A hollow test.** A test passes although the rule it names is broken.
- Verdict `pass` if there is none.

## 2. Actual problem and reproduction
- Specification: packet `work/experiments/renderer-sd-packets/QT-I-installer.md` (sections 1 and 6 are binding). It implements plan r2 after Astra plan check `20261003T144341Z-c770b9b1` (aqt rejected) and Astra re-check `20261003T151320Z-b183ce70` (QT-P-02 adopted through lockfile item 2).
- Sol implement call `20261003T160429Z-0ff22f98` implemented it. Its patch is `work/reviews/20261003T160429Z-0ff22f98/changes.patch` and its report `report.md` in the same folder. It is a re-run: the first call, `20261003T152454Z-012350e6`, was invalid because an existing test wrote the real install ledger, and the packet was amended to isolate the ledger (its section 5, row 3). Claude reviewed and applied the patch (section 4 lists the integrator changes).

## 3. Environment and versions
- Windows 11, CPython 3.14.7 64-bit, standard library only in `bootstrap.py`; py7zr 1.1.3 in `tools/.venv/renderer-spike` as a child process only.
- The review sandbox is read-only, CPU only, without network.

## 4. Necessary source and evidence
- Files under review: `tools/toolchain/bootstrap.py`, `tools/toolchain.lock.json`, `tests/test_bootstrap.py`, `.gitignore` — uncommitted changes against `HEAD`; `git diff HEAD -- <file>` shows them.
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
- Pin provenance: the four SHA-256 values are the `<archive>.sha256` files download.qt.io served on 3 October 2026, and the byte counts are from HTTP `HEAD` responses the same day; they are reproduced in the packet's lockfile item 3. No archive has been downloaded and nothing has been installed.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | aqt-based install | plan only | Astra plan check | rejected (1 blocker, 3 major) |
| 2 | own downloader, pinned verification, py7zr child | plan r2 | Astra scoped re-check | QT-P-01, -03, -04 closed; QT-P-02 adopted |
| 3 | the packet is implementable as written | implement call `20261003T152454Z-012350e6` | the wrapper's integrity check | invalid: the existing test suite appended a fake record to the real install ledger |
| 4 | the amended packet (ledger isolated in every test) | implement call `20261003T160429Z-0ff22f98` | acceptance check; Claude's patch review; the checks in section 4 | valid; acceptance passed; no blocker or major in Claude's review |

## 6. Constraints and owned files
- Read-only review. Never run `bootstrap.py install`, `install-skill`, `pin` or `approve`; do not use the network.
- Out of scope: listing, `EXTRACT_CODE`, `member_problems`, `install_archives` from step 6 on, `run_probe` (shard B); the choice of Qt 6.10.3 and of these four modules; the licence; style, `minor` and `nit` findings.
- Questions:
  1. **Verification order.** Is there any path on which archive bytes are listed, extracted, renamed to `<name>` or opened by anything other than the hashing loop before the size and SHA-256 checks pass?
  2. **Redirects.** For every 3xx the code accepts, is each rule of the policy checked on the resolved URL before the next request, and are relative `Location` values resolved against the current URL? Can `urllib` follow a redirect on its own (for example through a proxy handler or an opener default)?
  3. **Body handling.** Can a server make the code keep more than `bytes`, accept a body without `Content-Length` that is short, or hang past the per-archive deadline?
  4. **`load_lock`.** Does each rule of the packet's list refuse its violation with `lockfile: <id> ...`, and does the real lockfile still load round-trip identical?
  5. **aqt.** Can `doctor`, `install --profile renderer-spike` or any probe launch aqt? Does the metadata probe import aqt?
  6. **Regression.** Do `uv-venv-hashed`, `pip-hashed`, `npm-ci`, `winget` and the other methods behave exactly as before?

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
