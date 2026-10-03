# Scoped verification packet, round 2: QT-I installer fixes, shard A (QT-A-01)

Review only; do not perform follow-up work.

`python tools/agents/codex_review.py --kind review --model gpt-6.1-sol --effort max --speed fast --packet work/experiments/renderer-sd-packets/review-3-qt-installer-a.md`

## 1. Goal and acceptance
- Goal: the second and last scoped verification round of the `archive-7z-hashed` installer change (QT-I), a critical path (`tools/toolchain/*`).
  - Round 1, shard A (call `20261003T170945Z-ea9e17bd`): QT-A-02 and QT-A-03 fixed. QT-A-01 still open on one path: a proxy's `CONNECT` reply was read on the plain socket, which had only the per-receive timeout.
  - Round 1, shard B (call `20261003T170945Z-bd9dd96c`): pass.
  - Claude adopted the remaining finding and fixed it.
- Acceptance: one JSON result matching `schemas/review-result.schema.json` that answers only two things:
  1. Is QT-A-01 fixed in the current files, both on the direct path and through a proxy? If not, report it again under QT-A-01, with a counterexample against the current code.
  2. Does this fix introduce a new `blocker` or `major`? Report only those, each with a concrete counterexample (server or proxy bytes, environment, or call sequence, then the wrong outcome).
- Verdict `pass` when QT-A-01 is fixed and nothing new reaches `major`.

## 2. Actual problem and reproduction
Round 1's counterexample, in short. The full text is in `work/reviews/20261003T170945Z-ea9e17bd/review.json`.
- Set `HTTPS_PROXY=http://proxy.example.org:8080`.
- The proxy answers `CONNECT` with the 65-byte reply `HTTP/1.1 407 Proxy Authentication Required\r\nContent-Length: 0\r\n\r\n`, one byte every 59 seconds.
- Every receive stays inside the 60-second timeout, so `fetch_archive` refused only at 3,835 seconds, against its 1,800-second deadline.

## 3. Environment and versions
- Windows 11, CPython 3.14.7 64-bit. `bootstrap.py` uses the standard library only.
- The review sandbox is read-only, CPU only, without network.
- The owner's machine has no proxy configured, neither in the environment nor in the registry, so the proxy path is not its live path. It is fixed anyway.

## 4. Necessary source and evidence
- Round-2 delta: `work/experiments/renderer-sd-packets/qt-i-fix-delta-2.patch`.
  - From the round-1 files: `tools/toolchain/bootstrap.py` sha256 `754fa54e…eff58`, `tests/test_bootstrap.py` sha256 `a65face2…a22b`.
  - To the current files: `f793b4b0af28b20996eeb8dec41f734d499de73b35a0d89112ef22c8571e655d` and `883c53777c86f073219ea9b7ae2cb129f1e42a2492332e2095cf2e976d3666fd`.
  - `git diff HEAD -- <file>` shows the whole uncommitted QT-I change.
- The fix:
  - `_http_get` computes one deadline and gives it to both sockets. The TLS socket class `_DeadlineSSLSocket` is unchanged.
  - The opener's HTTPS handler is now `_DeadlineHTTPSHandler`, a subclass of `urllib.request.HTTPSHandler`. Its `https_open` opens `_DeadlineHTTPSConnection` with the same TLS context and the deadline.
    - `build_opener` leaves out its default `HTTPSHandler`, because the handler passed is an instance of it.
    - The default `ProxyHandler` stays, so an owner-configured proxy still works.
  - `_DeadlineHTTPSConnection`, a subclass of `http.client.HTTPSConnection`, replaces `_create_connection`, the hook that `HTTPConnection.connect` calls for the TCP connection.
    - The replacement calls `socket.create_connection` as before.
    - It then re-wraps the connected descriptor (`detach()`) in `_DeadlinePlainSocket`, a subclass of `_DeadlineSocket` and `socket.socket` that carries the same deadline.
    - If the re-wrap fails, it closes the descriptor.
  - Through a proxy, `HTTPConnection._tunnel` sends `CONNECT` with `sendall` and reads the reply through `makefile("rb")`, `SocketIO.readinto` and `recv_into` on that plain socket.
    - Every one of those calls waits at most min(60 s, time left) and is refused at or after the deadline.
    - `HTTPSConnection.connect` then wraps the same socket in TLS, as before.
  - urllib's `do_open` turns an `OSError` raised while connecting into `URLError(reason=…)`. `fetch_archive` catches it as `OSError` and reports "download deadline exceeded" when the deadline has passed, as before.
- Tests in `tests/test_bootstrap.py`:
  - New `test_real_opener_obeys_the_deadline_with_and_without_a_proxy`.
    - It runs the real opener, `ProxyHandler`, `CONNECT` tunnel and TLS client. Only `socket.create_connection` is replaced, by one end of a local `socket.socketpair()`. The budget is 0.6 s.
    - Through `HTTPS_PROXY=http://proxy.example.org:8080` (the other `*_proxy` variables are removed for the test):
      - the peer receives `CONNECT download.qt.io:443 HTTP/1.1`;
      - it answers with the 65-byte 407 reply, one byte every 0.2 s (13 s in all);
      - the call ends with a `TimeoutError` reason after 0.5 to 5 s;
      - the TCP connection went to `("proxy.example.org", 8080)` with timeout 0.6.
    - Without a proxy:
      - the peer receives a TLS handshake record and never answers;
      - the time bounds are the same;
      - the connection went to `("download.qt.io", 443)`.
    - Mutation check: the pre-fix handler (the default `HTTPSConnection`) fails the proxy case. The call ran 13.8 s and ended with `OSError('Tunnel connection failed: 407 Proxy Authentication Required')`.
  - `test_http_seam_uses_default_tls_fixed_headers_and_no_redirects` now requires a `_DeadlineHTTPSHandler` whose deadline equals the TLS socket class's: 2700.0 and 1010.0 for budgets 1700 and 10 at monotonic time 1000.
- Checks on the current files (owner's machine, Windows 11, CPython 3.14.7):
  - `python tests/test_bootstrap.py`: 93 OK, 11 skipped. Every skip says "symlinks not available".
  - `tests/test_agent_rules_sync.py`: 11 OK.
  - `tests/test_codex_review.py`: 14 OK.
  - `tests/test_stop_gate.py`: 22 OK, 2 skipped.
  - `tests/test_wiki_lint.py`: 6 OK, and `tools/wiki/lint.py`: ok.
  - `tests/test_workbench.py`: 247 OK, 5 skipped.
  - The lockfile is unchanged since the full review.
- Bounds this fix does not change. Round 1 did not report them; judge them only if the fix makes them worse.
  - Name resolution waits on the operating system's resolver.
  - `socket.create_connection` gives each resolved address min(60 s, time left at the request). With k addresses, connecting can overrun the deadline by at most (k − 1) × 60 s. The first bounded send or receive then refuses.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | QT-I as implemented by Sol (`20261003T160429Z-0ff22f98`) | full review, two Astra shards | `20261003T163023Z-e2d3e15b`, `20261003T163024Z-1c9a1c6e` | 6 major (QT-A-01 to 03, QT-B-01 to 03) |
| 2 | the six fixes | Claude-authored fixes | scoped round 1: `20261003T170945Z-ea9e17bd` (A), `20261003T170945Z-bd9dd96c` (B) | B pass; A: QT-A-02 and QT-A-03 fixed, QT-A-01 open through a proxy |
| 3 | bound the plain socket by the same deadline | Claude-authored fix, section 4 | the checks in section 4 | all pass; this round |

## 6. Constraints and owned files
- Read-only review. Never run `bootstrap.py install`, `install-skill`, `pin` or `approve`, and do not use the network.
- Running `tests/test_bootstrap.py` is allowed: its fixtures stay under `work/`, and the new test uses only a local socket pair. If the sandbox refuses local sockets, say so and judge from the source.
- In scope: QT-A-01 on both paths, and any new `blocker` or `major` introduced by `_DeadlineHTTPSConnection`, `_DeadlineHTTPSHandler`, the `_http_get` change and the two tests above.
- Out of scope:
  - what round 1 found fixed (QT-A-02, QT-A-03 and shard B);
  - code this delta did not touch;
  - `minor` and `nit` findings, and style.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`. Use QT-A-01 if it is not fixed, and new IDs from QT-A-04 for new findings.
- Review only; do not perform follow-up work.
