# Scoped verification packet 2: SB-C-001 and SB-C-002 in the S-B handoff test

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: scoped verification round 2, the last round allowed for this candidate. Check only:
  - whether blocking findings SB-C-001 and SB-C-002 of round 1 (review `20261003T060243Z-53cbaf18`, packet `work/experiments/renderer-sb-packets/review-3-handoff-verify.md`) are fixed;
  - whether the fix introduces a new `blocker` or `major`.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`.
  - SB-C-001 counts as fixed if two things hold:
    - after a drain that failed without device removal, no GPU object of that process is released, whether by our code or by a destructor;
    - a drain failure cannot turn into ordinary teardown through a later successful retry.
  - SB-C-002 counts as fixed if the abort path ends the process within a bounded time, whether or not the report can be delivered.
  - Verdict `pass` if both are fixed and there is no new `blocker` or `major`.

## 2. Actual problem and reproduction
- Round 1 findings:
  - **SB-C-001 (major).** `abandon()` was reached only from a destructor's second drain attempt. An explicit drain failure therefore unwound first. Unwinding could release an already-drained peer (the consumer queue in `same-device`, the producer in `d3d11-consumer`) before the failing queue's destructor abandoned. A transient failure that passed on the destructor's retry gave ordinary teardown.
  - **SB-C-002 (major, unverified).** `abandon()` called the report callback synchronously. A stdout pipe that nobody reads could block it before `TerminateProcess`.
- Both dispositions: `adopt`.
- The fix, in `work/experiments/renderer-sb/handoff/`:
  - **`Queue::drain()`** wraps the injection test, `Signal` and `wait_fence` in `try`.
    - On any exception it calls `abandon()` at once unless the drain fence reports `UINT64_MAX` (device removal). Device removal rethrows.
    - So `drain()` returns or throws only when release is safe. No retry or unwinding happens after a non-removal failure.
    - `~Queue()` now only records the exception, which can only be device removal.
  - **`D11Consumer::drain()`**: the same, using `GetDeviceRemovedReason() == S_OK` as the test for "not removed". `~D11Consumer()` likewise.
  - **`abandon()`**:
    - records the failure;
    - sets the running child's cancel event;
    - runs the registered report on a new thread (`CreateThread`) and waits at most `report_wait_ms` = 10,000 ms for it;
    - then calls `TerminateProcess(GetCurrentProcess(), 3)`, with `ExitProcess(3)` as an unreachable fallback.
    - If `CreateThread` fails, it terminates without a report.
  - **Test hook** `--inject-unconfirmed-drain producer|consumer` (`src/protocol.h`, `src/protocol.cpp`; it was a flag):
    - every drain of that role fails before its `Signal`, as an unconfirmed wait would;
    - `Queue` derives its role from its type: the producer is always the compute queue, consumers are direct queues;
    - `consumer` also covers the D3D11 consumer and, in `second-device`, the child, which receives `--inject-unconfirmed-drain consumer` on its command line;
    - the child accepts only `consumer`;
    - `run_child` copies the role into the file-scope variable;
    - self-test cases: a missing value, an unknown value and a child with `producer` are rejected; both valid forms are accepted.
  - **Ownership, unchanged.** In every mode, what queue work references is declared before the D3D12 queues: the shared fences, the ring, and the D3D11 consumer with its opened resources. The ring holds the textures, scratch textures, readbacks, command allocators and lists. So during unwinding each queue is destroyed, and drained, before anything its work references. A `Queue`'s own members (queue, drain fence, event) are used only by that queue.
  - **`src/main.cpp`**: help text only.
  - **`README.md`**: the abort paragraph and the injection paragraph.

## 3. Environment and versions
- Windows 11 (10.0.26200), MSVC 19.51.36260 x64, Windows SDK 10.0.26100.0, RTX 4070 Laptop GPU, NVIDIA user-mode driver 32.0.16.1692.
- The review sandbox is read-only with no GPU: source only.

## 4. Necessary source and evidence
- Files, all under `work/experiments/renderer-sb/handoff/`: `src/handoff.cpp`, `src/handoff.h`, `src/main.cpp`, `src/protocol.cpp`, `src/protocol.h`, `README.md`.
- Integrator's runs on the fixed build, `sb_handoff.exe` SHA-256 `598a3e28d8779136fbade2cf1d663f9af85104082fa8dbbeb1a8d6660b7ae113`:
  - `check_handoff.py` exits 0 (self-test included). The Ninja build has no warnings at `/W4`.
  - `--mode all --iterations 1000`: exit 0, every mode 1,000 of 1,000. Run on the RTX 4070 with and without `--debug-layer`, and on WARP with it. With the debug layer, teardown passes with 0 unexpected live objects.
  - `--debug-layer --inject-unconfirmed-drain <role>`, 50 iterations (150 for `resize`, so that the first drain is the resize-boundary drain), for each role and each of `same-device`, `second-device`, `d3d11-consumer`, `resize` and `all`:
    - **producer**: every run exits 3. The JSON holds the failing mode with teardown `fail` and the error "injected unconfirmed drain (test); drain unconfirmed, so the process ended without releasing GPU objects". `all` stops after `same-device`; `resize` stops at frame 100.
    - **consumer**: exits 3, except `second-device`. There the child exits 3 and the parent records "child: injected unconfirmed drain (test); …" with teardown `fail` and exits 1.
    - No `sb_handoff` process remained 2 s after any run.
  - Stalled-pipe experiment, the round 1 counterexample: `--mode same-device --iterations 10000 --inject-unconfirmed-drain producer`, with stdout redirected to an anonymous pipe that is never read:
    - exit 3 after 21.1 s, about 10.8 s of run plus the 10 s limit;
    - no process left.
  - The same command with a reader: exit 3 after 10.8 s, and 80,000 bytes of stdout that parse as complete JSON with all 10,000 timing samples.
  - Sanitised summary: `work/experiments/renderer-sb/results/handoff-20261003T061444Z-summary.json`.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | abandoning from the destructor's retry prevents releases | `abandon()` in `~Queue`/`~D11Consumer` | round 1 review | SB-C-001 still open; SB-C-002 new |
| 2 | abandoning inside the failing drain, before unwinding, with a bounded report | section 2 | GPU runs and pipe experiment, section 4 | pass |

## 6. Constraints and owned files
- Read-only review.
- Out of scope:
  - everything outside SB-C-001, SB-C-002 and this fix;
  - findings settled in earlier rounds;
  - `minor` and `nit` findings;
  - result pages and timing numbers.
- Questions:
  1. Is there still a path on which a GPU object is released after a drain that failed without device removal? Consider:
     - explicit drains;
     - destructor drains during unwinding from another exception;
     - the resize path;
     - the child;
     - the D3D11 consumer.
  2. Can a non-removal drain failure still be retried, or turn into ordinary teardown?
  3. Is the abort path bounded in every case? Consider:
     - the report blocks or throws;
     - `CreateThread` fails;
     - abort during unwinding;
     - the child's report pipe.
  4. Does the role-based injection exercise the drains the findings named, and does the child's acceptance of `consumer` alone keep the command line strict?

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
