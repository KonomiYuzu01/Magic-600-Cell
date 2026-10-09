# Scoped verification packet: QT-I installer fixes, shard A (QT-A-01, QT-A-02, QT-A-03)

Review only; do not perform follow-up work.

Run concurrently with shard B (`review-2-qt-installer-b.md`):
`python tools/agents/codex_review.py --kind review --model gpt-6.1-sol --effort max --speed fast --packet work/experiments/renderer-sd-packets/review-2-qt-installer-a.md`

## 1. Goal and acceptance
- Goal: the first scoped verification round after the full review of the `archive-7z-hashed` installer (QT-I). Astra shard A, call `20261003T163023Z-e2d3e15b`, reported three `major` findings: QT-A-01, QT-A-02 and QT-A-03. Claude adopted all three and fixed them. This is a critical path (`tools/toolchain/*`).
- Acceptance: one JSON result matching `schemas/review-result.schema.json` that answers only two things:
  1. Is each of QT-A-01, QT-A-02 and QT-A-03 fixed in the current files? If one is not, report it again under its own ID, with a counterexample against the current code.
  2. Does the fix introduce a new `blocker` or `major`? Report only those, each with a concrete counterexample (server bytes, lockfile text or call sequence, then the wrong outcome).
- Verdict `pass` when every finding is fixed and nothing new reaches `major`.

## 2. Actual problem and reproduction
The findings, in short. The full text is in `work/reviews/20261003T163023Z-e2d3e15b/review.json`.
- **QT-A-01.** The download deadline could not interrupt a blocking buffered read. `HTTPResponse.read(n)` loops over many socket receives, so a server that sends one byte every 59 seconds passed the 1,800-second deadline. The reviewer reproduced a refusal at 3,776 seconds.
- **QT-A-02.** Repeated `Content-Length` fields bypassed the mismatch refusal. The headers were copied into a dict, which keeps the last value, while `http.client` frames the body by the first.
- **QT-A-03.** The first request bypassed the URL policy. `base_url` accepted IP literals, `localhost`, `*.localhost` and hosts without a dot when they were listed in `network_hosts`, and `_mirror_url` ran only on redirects.

## 3. Environment and versions
- Windows 11, CPython 3.14.7 64-bit. `bootstrap.py` uses the standard library only.
- The review sandbox is read-only, CPU only, without network.

## 4. Necessary source and evidence
- Fix-only delta: `work/experiments/renderer-sd-packets/qt-i-fix-delta.patch`.
  - It runs from the reviewed files (`tools/toolchain/bootstrap.py` sha256 `d9d4fea8…14af`, `tests/test_bootstrap.py` sha256 `1930559e…5009`, the digests in the findings' evidence) to the current files (`754fa54e…eff58` and `a65face2…a22b`).
  - Most of its length is shard B's change: `install_archives` gained an outer `try` and its body moved one level in.
- Current files: `tools/toolchain/bootstrap.py` and `tests/test_bootstrap.py`. `git diff HEAD -- <file>` shows the whole uncommitted QT-I change.
- The fixes:
  - **QT-A-01.** New `_DeadlineSocket` mixin. `_http_get(url, timeout)` now receives the time left before the download deadline. It sets `SSLContext.sslsocket_class`, the documented hook that `wrap_socket` uses, to a subclass of `_DeadlineSocket` and `ssl.SSLSocket` carrying that deadline.
    - Every `do_handshake`, `recv`, `recv_into`, `send` and `sendall` call first sets the socket timeout to min(`SOCKET_TIMEOUT` = 60 s, time left). At or past the deadline it raises `TimeoutError` instead.
    - `SocketIO.readinto` calls `recv_into`, so each receive inside a buffered `read` or `readline` is bounded. That covers the status line, the headers and the body.
    - `opener.open` receives min(60 s, time left) as the connect timeout.
    - `fetch_archive` passes `deadline - time.monotonic()` to `_http_get`. When any transport exception occurs at or after the deadline, it refuses with "download deadline exceeded". `check_deadline` now uses `>=`.
  - **QT-A-02.** New `_header_values(headers, name)`. It uses `HTTPMessage.get_all` and, for test doubles, keeps every matching dict item.
    - A repeated `Content-Length` is refused as "repeated Content-Length", before `.part` is opened.
    - A redirect needs exactly one `Location`; otherwise it is refused as "invalid redirect location".
  - **QT-A-03.** `validate_archive_entry` now requires `_mirror_url(base_url + path, path)` for every archive, so `load_lock` refuses such an entry. `fetch_archive` also refuses the initial URL before any request ("URL policy refused").
- New or changed tests in `tests/test_bootstrap.py`:
  - `test_trickling_server_is_cut_off_at_the_deadline`. It runs the real `HTTPResponse`, `BufferedReader` and `socket.SocketIO` over a socket double with a simulated clock. The double delivers one byte per 59 s: in the headers phase from the first byte, in the body phase after headers that arrive at once. Both phases are refused at exactly 1,800 s, and nothing is left in the destination.
  - `test_tls_socket_waits_no_longer_than_the_deadline`. It takes the context that `_http_get` built and wraps one end of a local `socket.socketpair()` whose peer never answers. With a 0.3 s budget, `recv` times out promptly. Once the deadline has passed, `do_handshake` raises "download deadline exceeded" at once.
  - `test_http_seam_uses_default_tls_fixed_headers_and_no_redirects` now also checks three things: the socket-class deadline is monotonic + time left; the opener timeout is min(60, time left); a budget of 0 raises before any opener is built.
  - `test_repeated_length_or_location_is_refused`. It uses a real `HTTPResponse` with `Content-Length: 4` followed by `Content-Length: 3`, and one with two `Location` fields.
  - `test_initial_url_policy_applies_before_any_request` covers 127.0.0.1, localhost, a.localhost and internal, with zero `_http_get` calls.
  - `test_malformed_archive_fields_are_rejected` has new lockfile cases for the same four hosts listed in `network_hosts`.
  - The deadline test now uses a clock, not a fixed tick count. The fakes accept 0 < budget ≤ `INSTALL_TIMEOUT` instead of exactly 60.
- Bounds the fix does not change; judge whether either reaches `major`:
  - Name resolution waits on the operating system's resolver.
  - `socket.create_connection` gives each resolved address min(60 s, time left at the request). With k addresses, connecting can overrun the deadline by at most (k − 1) × 60 s. The handshake check then refuses.
  - An HTTPS proxy that the owner configured has its `CONNECT` reply read on the plain socket, with the same per-receive timeout.
- Checks on the current files (owner's machine, Windows 11, CPython 3.14.7):
  - `python tests/test_bootstrap.py`: 92 OK, 11 skipped. Every skip says "symlinks not available"; this account has no symlink privilege.
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
| 2 | the six fixes in section 4 here and in shard B | Claude-authored fixes | the checks in section 4 | all pass; this scoped round |

## 6. Constraints and owned files
- Read-only review. Never run `bootstrap.py install`, `install-skill`, `pin` or `approve`, and do not use the network. Running `tests/test_bootstrap.py` is allowed, because its fixtures stay under `work/`.
- In scope: QT-A-01, QT-A-02 and QT-A-03, and any new `blocker` or `major` introduced by their fixes in `_DeadlineSocket`, `_http_get`, `_header_values`, `fetch_archive`, `validate_archive_entry` and the tests named above.
- Out of scope:
  - shard B's findings and fixes (`resolve_executable`, `_archive_tree`, `_remove_tree`, `install_archives`, `install`);
  - code the full review already covered and these fixes did not touch;
  - `minor` and `nit` findings, and style.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`. Use the finding IDs QT-A-01 to QT-A-03 for a finding that is not fixed, and new IDs from QT-A-04 for new findings.
- Review only; do not perform follow-up work.
