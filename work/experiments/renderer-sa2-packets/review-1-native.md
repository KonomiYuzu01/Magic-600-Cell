# Review packet: SA2 native producer DLL (SA2-N), integrated candidate

Review only; do not perform follow-up work.

Run from the `claude/renderer-sb` checkout:
`python tools/agents/codex_review.py --kind review --model gpt-6.1-sol --effort max --speed fast --packet work/experiments/renderer-sa2-packets/review-1-native.md`

## 1. Goal and acceptance
- Goal: one Sol review (fast tier, non-critical probe code) of the integrated SA2 native producer: `sa2_interop.dll`, `sa2_selftest.exe`, their build and their check. The Godot smoke test (SA2-G) loads this DLL into Godot's process; its answers go to PR #43 section 7.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. Report only `blocker` or `major` findings, each with a concrete counterexample (the call sequence or input, then the wrong outcome). Wrong outcomes that count:
  - **A protocol error.** A texture written while Godot may still read it; a Godot read that can start before the write finished; a fence value that can decrease or repeat; a release or detach that can run while GPU work still references a resource.
  - **A state error.** A barrier whose before-state differs from the state the slot is in at that point, or a handover that leaves the slot in a state other than `state_after_write`. With enhanced barriers, an access or sync pair the D3D12 Enhanced Barriers specification rejects.
  - **A false pass.** A self-test check that passes although the property it names is broken (for example a readback that does not decode both corners, or a verification that compares fewer texels than it claims).
  - **A contract break.** Behaviour that contradicts a comment in the committed header `work/experiments/renderer-sa2/native/include/sa2_interop.h`, or an exported function missing from it.
  - **A process hazard.** A crash, hang or unbounded wait reachable through the public functions with valid arguments; a drain failure that returns instead of ending the process as the header specifies.
  - **A build hazard.** A build or check that writes outside its own build directory, deletes anything it did not create, or places executables in the system temp folder.
- Verdict `pass` if there is none.

## 2. Actual problem and reproduction
- Specification: packet `work/experiments/renderer-sa2-packets/SA2-N-native.md` (read sections 1 and 6) and the header.
- Codex implement call `20261003T143409Z-24c43648` implemented the packet: valid run, acceptance check passed (`report.md` and `changes.patch` in `work/reviews/20261003T143409Z-24c43648/`). Claude reviewed and applied the patch, then ran the GPU modes on the owner's machine, which found three defects. Claude fixed them; the fixes are the diff `work/experiments/renderer-sa2-packets/review-1-claude-fixes.diff` (Codex's version on the left, the candidate on the right). This review covers the whole candidate, with particular attention to these fixes:
  1. **Hardware crash.** On the owner's NVIDIA adapter, `ClearRenderTargetView` with more than 32 rectangles in one call ended the process with `0xC0000409` inside the call (bisected: 33 fails at the first call, 32 runs; WARP accepts 100). `clear_code` now issues the code-block clears in batches of at most 32 rectangles.
  2. **Zero-value wait.** With its own queue, `sa2_produce` enqueued `Wait(free, 0)` for a slot never shown, which the debug layer reports as a warning ("waiting for a fence value of zero will always be satisfied"), so the WARP `--debug` own-queue runs failed their empty-warning check. The wait is now skipped when `slot.last_shown` is 0.
  3. **Blocked temp executables.** Application Control on the owner's machine refuses unsigned executables in the system temp folder (WinError 4551). `check_native.py` now builds in a fresh `build-check-<pid>` directory beside itself and removes only that directory; `build.cmd`'s default build directory moved from `%TEMP%` to `native\build`; the README examples follow. All of these are under the repository's ignored `work/` tree.

## 3. Environment and versions
- Windows 11 (build 26200); NVIDIA GeForce RTX 4070 Laptop GPU; MSVC 19.51.36260 (Visual Studio Community 2026), Windows SDK 10.0.26100.0; CMake 4.4.3, Ninja 1.13.2.
- The review sandbox is read-only and CPU only.

## 4. Necessary source and evidence
- Files under review (all untracked under the ignored `work/` tree, so read them directly; `git diff` does not show them): `work/experiments/renderer-sa2/native/` — `CMakeLists.txt`, `build.cmd`, `check_native.py`, `README.md`, `src/interop.cpp`, `src/pure.cpp`, `src/pure.h`, `src/selftest.cpp`, `src/gpu_test.cpp`, `src/test.h`. The header and `work/experiments/renderer-sa2/code_layout.json` are committed and unchanged.
- D3D12 Enhanced Barriers rules relied on: `SyncBefore = NONE` with `AccessBefore = NO_ACCESS` is valid for the first access of a resource in an ExecuteCommandLists scope; `SyncAfter = NONE` with `AccessAfter = NO_ACCESS` is valid as the final barrier; an ExecuteCommandLists completion is a full finish and flush.
- Integrator's checks on the candidate (owner's machine, this source):
  - `python work/experiments/renderer-sa2/native/check_native.py --gpu`: `check_native: ok (CPU, WARP and hardware)`; no leftover build directory.
  - Explicit build, `sa2_selftest.exe --cpu`, `--warp --debug`, `--hardware --debug`: exit 0 each; 11 CPU checks and 10 checks per GPU mode, all `pass` (8 configurations of same/own queue, legacy/enhanced barriers and created/external textures, each 300 frames with a resize, readback decode of both corners and full-texel verification; the `inject-drain` and `device-loss` child probes).

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | the packet is implementable as written | implement call `20261003T143409Z-24c43648` | acceptance check; Claude's patch review | CPU pass; GPU modes not run in the sandbox |
| 2 | the GPU modes pass on the owner's machine | none | `--warp --debug`, `--hardware` | own-queue WARP runs failed (zero-value wait); hardware ended with `0xC0000409` |
| 3 | the driver fails on large rectangle counts | instrumented copy, batch sizes 1 to 64 | hardware runs | 32 or fewer per call pass, 33 or more fail |
| 4 | fixes 1 to 3 in section 2 | the diff above | all checks in section 4 | all pass |

## 6. Constraints and owned files
- Read-only review.
- Out of scope:
  - the Godot project and run script (SA2-G), and the Qt smoke test;
  - the committed header's design (except where the implementation contradicts it);
  - the deferred header nit that the typedef `sa2_debug_counts` and the function `sa2_debug_counts` share a name;
  - performance and timing; style, `minor` and `nit` findings.
- Questions:
  1. **Batching.** Does the batched `clear_code` still produce exactly the image `code_layout.json` specifies (every 8 by 8 block of both corners set to the bit's colour, the fill everywhere else), for any slot size the DLL accepts?
  2. **Skipped wait.** With its own queue, can a slot be written while Godot may still read it now that the wait is skipped for `last_shown == 0`? Consider unregister and re-register, resize and a slot that was shown at an earlier generation.
  3. **Fences and drain.** Can any fence value repeat or decrease, can a CPU wait be unbounded, and can a drain failure without device removal return instead of ending the process?
  4. **Barriers.** For every combination of queue mode, barrier API, `state_before_write` and `state_after_write` the DLL accepts, is each barrier's before-state the slot's actual state, and is each enhanced barrier valid under the rules in section 4?
  5. **Self-test strength.** Could the GPU self-test pass with a protocol error, a state error or a corrupted image? Is the deliberate verification failure detected through the same path as a real one?
  6. **Build and check.** Can `check_native.py` or `build.cmd` write or delete outside their own build directory?

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
