# Scoped verification packet: SB-C-001 fix in the S-B handoff test

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: scoped verification round 1 for review `20261003T053340Z-26123011` (packet `work/experiments/renderer-sb-packets/review-3-handoff.md`). Check only:
  - whether blocking finding SB-C-001 is fixed;
  - whether the fix introduces a new `blocker` or `major`.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`.
  - SB-C-001 counts as fixed if no path remains on which a drain that failed without device removal is followed by the release of a GPU object of that process, whether by our code or by a destructor.
  - Verdict `pass` if SB-C-001 is fixed and there is no new `blocker` or `major`.

## 2. Actual problem and reproduction
- SB-C-001 (major): `Queue::~Queue` and `D11Consumer::~D11Consumer` caught a drain failure and continued. The enclosing scope then destroyed the ring, the fences and the device, although a timeout or an event-registration failure does not establish GPU completion.
- Disposition: `adopt`.
- The fix, in `work/experiments/renderer-sb/handoff/`:
  - `src/handoff.cpp` `abandon()`:
    - marks the result failed, with teardown `fail`;
    - sets the cancel event of a running child, if any (`abandon_cancel`);
    - calls the registered report, inside `try`/`catch`;
    - then calls `TerminateProcess(GetCurrentProcess(), 3)`, with `ExitProcess(3)` as an unreachable fallback.
  - No destructor runs after a failed drain, so nothing is released. Windows reclaims the process's GPU objects only after the GPU stops using them.
  - `Queue::~Queue`: on a drain failure it calls `abandon()` unless the drain fence reports `UINT64_MAX`, which means device removal: all GPU work has ended, so teardown continues and the mode fails.
  - `D11Consumer::~D11Consumer`: the same, with `GetDeviceRemovedReason() == S_OK` as the test.
  - `ChildProcess::start` registers its cancel event; `~ChildProcess` unregisters it.
  - `run_child` registers a report that sends the final child report (phase 2) over the pipe before the child ends.
  - `src/main.cpp`:
    - registers a report that prints the failing mode and writes the JSON of every finished mode plus the failing one;
    - JSON writing moved into `write_output`, with unchanged behaviour.
  - `src/handoff.h`: `abandoned_exit = 3` and `set_abandon_report`.
  - Test hook `--inject-unconfirmed-drain` (`src/protocol.h`, `src/protocol.cpp`):
    - every D3D12 `Queue::drain` with work outstanding throws, as an unconfirmed wait would;
    - the child rejects the flag, and the self-test covers the parse;
    - `run_mode` copies the flag into a file-scope variable.
  - `README.md`: the paragraph after the exit-0 rules, and "released after their drains succeed".

## 3. Environment and versions
- As in the earlier packet: Windows 11 (10.0.26200), MSVC 19.51.36260 x64, Windows SDK 10.0.26100.0, RTX 4070 Laptop GPU, NVIDIA user-mode driver 32.0.16.1692.
- The review sandbox is read-only with no GPU: source only.

## 4. Necessary source and evidence
- Files: `src/handoff.cpp`, `src/handoff.h`, `src/main.cpp`, `src/protocol.cpp`, `src/protocol.h`, `README.md`, all under `work/experiments/renderer-sb/handoff/`.
- Integrator's runs on the fixed build (`sb_handoff.exe` SHA-256 `5257a350ad18e2245e81bb0f2a4034d529b7818d6309296d206a3f0b43905e83`):
  - `check_handoff.py` exits 0; the Ninja build has no warnings at `/W4`.
  - `--mode all --iterations 1000`, on the RTX 4070 with and without `--debug-layer` and on WARP with it: every mode passes, 1,000 of 1,000. With the debug layer, teardown passes with 0 unexpected live objects.
  - `--iterations 50 --debug-layer --inject-unconfirmed-drain`, once each with `same-device`, `second-device`, `d3d11-consumer` and `all`:
    - every run exits 3;
    - the JSON holds the failing mode with teardown `fail` and the error "injected unconfirmed drain (test); drain unconfirmed, so the process ended without releasing GPU objects";
    - `all` stops after `same-device`;
    - no `sb_handoff` process remained 2 s later.
  - Sanitised summary: `work/experiments/renderer-sb/results/handoff-20261003T052805Z-summary.json`.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | ending the process is a confirmed abort that never frees memory in use | section 2 | GPU runs and injection, section 4 | pass |

## 6. Constraints and owned files
- Read-only review.
- Out of scope:
  - everything outside SB-C-001 and its fix;
  - findings already settled in the first review;
  - `minor` and `nit` findings;
  - result pages and timing numbers.
- Questions:
  1. Can any path still release a GPU object after a failed drain? Consider:
     - a drain failure during exception unwinding;
     - the explicit drains at the end of each mode;
     - the resize path;
     - the child.
  2. Is the device-removal exception sound? Does `UINT64_MAX` from the drain fence, or `GetDeviceRemovedReason() != S_OK`, establish that all GPU work of that device has ended?
  3. Can `abandon()` hang or lose the report? Consider:
     - the report throws;
     - stdout buffering;
     - the pipe is full or closed;
     - `TerminateProcess` while another thread exists.
  4. Does the parent still record a child's abandonment as a failure, and does the cancel registration avoid setting a closed handle?

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
