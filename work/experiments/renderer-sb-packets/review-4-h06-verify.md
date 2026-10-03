# Review packet: H-06 candidate, scoped verification round 1

Review only; do not perform follow-up work.

Run from the `claude/renderer-sb` checkout:
`python tools/agents/codex_review.py --kind review --model gpt-6.1-sol --effort max --speed fast --packet work/experiments/renderer-sb-packets/review-4-h06-verify.md`

## 1. Goal and acceptance
- Goal: scoped verification round 1 of the H-06 candidate. Its full review ran as two shards:
  - call `20261003T112324Z-ed906b35`, probe features (packet `review-4-h06-probe.md`): H6-A-001 and H6-A-002;
  - call `20261003T112324Z-39182ced`, cost-table tools (packet `review-4-h06-tools.md`): H6T-B-01.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. Check only:
  - whether each of H6-A-001, H6-A-002 and H6T-B-01 is fixed;
  - whether a fix introduces a new `blocker` or `major`, judged by the wrong-outcome lists of the two shard packets.
- Each finding needs a concrete counterexample. Verdict `pass` if all three are fixed and the fixes add nothing that blocks.

## 2. Actual problem and the fixes
- **H6-A-001 (major).** Accepted frame counts above 2^53 were rounded in `run.json`.
  - Fix, `src/probe.cpp`, `options()`: validation also requires `cycle_frames <= 2^53`.
  - Since n is a positive multiple of 2T, this gives T <= 2^52, so both counts are exact as JSON doubles. The existing `T <= UINT64_MAX/2` overflow guard stays.
  - Self-test refusals: the review's counterexample (T = 9007199254740993, n = 18014398509481986) and T = 2 with n = 2^53 + 4.
  - Self-test acceptance: T = 2^52 with n = 2^53. This pair round-trips exactly through `runJson`, `dump` and `parse`; the check compares with `number()`, because the reader's `integer()` stops at 9e15.
  - `README.md` states the bound.
- **H6-A-002 (major).** A snapshot error could turn a failed check's exit 4 into exit 1.
  - Fix, `src/gpu.cpp`, `checkEffect`: the effect result is printed before the snapshot (the sort result already was). The snapshot is then written inside `try`/`catch`.
  - After a failed effect or sort check, a snapshot error prints `snapshot not written` and `checkEffect` returns its result. `runGpu`'s existing exit-4 guards then run, so the run exits 4 with no `run.json` or trace.
  - After passing checks, the error propagates, giving exit 1 as before.
  - `README.md` states both cases.
- **H6T-B-01 (major).** Sanitized identifiers were used as join keys.
  - Fix, `tools/perf/renderer_gate.py`: `summarize` is now `sanitize(summarize_private(...))`. `summarize_private` is the former body without the final `sanitize`. The public result and the CLI are unchanged.
  - Fix, `tools/perf/feature_costs.py`: it calls `summarize_private` and joins on the raw candidate, scene, feature, run ID and build identity. As before, only the complete table is sanitized at the end. The gate's `invalid_runs` and `unreadable` now pass through that final sanitize unsanitized.
  - Test, `tests/test_feature_costs.py`: `test_joins_use_raw_identifiers_when_sanitized_ones_collide` reproduces the counterexample.
    - It sets `USERNAME=synthetic-owner` and adds a second candidate `synthetic-owner-z` with 70 ms W3 fog runs.
    - It asserts that the selected candidate's fog row is unchanged: cost 5 ms, p99 30 ms, `met`, `breaks_gate` false.
  - The CLI test now wraps `summarize_private`, which must still be called exactly once.

## 3. Environment and versions
- As in the shard packets: Windows 11, MSVC 19.51, Windows SDK 10.0.26100.0, CMake 4.4.3, Ninja 1.13.2, RTX 4070 Laptop GPU with driver 616.92.
- The review sandbox is read-only and has no GPU.

## 4. Necessary source and evidence
- Changed since the full review:
  - `work/experiments/renderer-sb/probe/src/probe.cpp`, `src/gpu.cpp`, `README.md`;
  - `tools/perf/renderer_gate.py`, `tools/perf/feature_costs.py`, `tests/test_feature_costs.py`.
- Everything else is as the full review saw it.
- Integrator checks on this source (rebuilt probe, build identity `c651312b…`):
  - build ok; `sb_probe.exe --selftest` ok;
  - `check_probe.py` exit 0. A first attempt stopped with WinError 4551: Windows Application Control blocked the freshly built sandbox executable before its self-test ran. The rerun passed unchanged.
  - `tests/test_feature_costs.py` 17 OK; `tests/test_renderer_gate.py` 47 OK; `tests/test_b412_summary.py` 40 OK.
  - GPU (RTX 4070, W3, `--feature outlines`, 5 s):
    - with a fresh `--snapshot` folder: exit 0, effect pass, label pass, both PNG files and the run record written;
    - again with the same folder (files exist, checks pass): exit 1 with `snapshot files already exist`, and no run record.
  - The failed-check path with a snapshot error was not run on the GPU, because no switch forces a failed effect check (features refuse `--inject`). Verify it by reading `checkEffect` and `runGpu`.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | the H-06 candidate is ready | none | full review, two shards | three majors: H6-A-001, H6-A-002, H6T-B-01 |
| 2 | the three fixes above close them | section 2 | section 4 checks | this round |

## 6. Constraints and owned files
- Read-only review.
- Out of scope:
  - every other part of the candidate;
  - timing numbers;
  - style, `minor` and `nit` findings.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
