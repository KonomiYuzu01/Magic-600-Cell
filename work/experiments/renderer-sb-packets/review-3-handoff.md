# Review packet: S-B D3D12 resource handoff test (SB-C)

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: one Sol review (fast tier, non-critical experiment code) of the handoff test in `work/experiments/renderer-sb/handoff/`. It is E-2.4-01 item 4: a minimal D3D12 resource handoff with fence synchronisation, on the same device and to a second device. The Godot and Qt interop smoke tests will reuse its protocol.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. Report only `blocker` or `major` findings, each with a concrete counterexample: inputs or state, then the wrong outcome.
  - Wrong outcomes that count:
    - a false pass: a mode reports `pass`, or a frame counts as verified, although a texel, the fence order or the slot ownership was wrong;
    - a hazard under the protocol as coded: a slot texture, clear source, command allocator, readback buffer or D3D11 staging texture reused or read before the GPU work that used it has completed;
    - an unbounded hang or a deadlock on any path, including a child crash, a timeout, device removal or cancellation;
    - a teardown that releases a resource while a queue may still use it, or a live object or leaked handle that the debug-layer check would miss;
    - a child-process handle defect: an unrelated handle inherited, a handle closed twice or leaked, or a report from another adapter accepted;
    - exit 0 although a mode failed after submitting work;
    - a README statement that the code contradicts.
  - Verdict `pass` if there is none.

## 2. Actual problem and reproduction
- Protocol for frame `n` (sequences and fence values start at 1): slot `(n - 1) % 3`.
  - The CPU observes `free >= n - 3` and verifies that slot's old readback before reuse.
  - A compute queue waits for `free >= n - 3`, clears a private texture to the pattern of `n`, copies it into the ring texture and signals `ready = n`.
  - The consumer waits for `ready = n`, copies to a readback resource and signals `free = n`. The CPU checks every texel.
- Topologies:
  - `same-device`: compute queue to direct queue on one device;
  - `second-device`: a child process (`--child`) with its own device. The parent shares a heap and two fences as NT handles, passed through `PROC_THREAD_ATTRIBUTE_HANDLE_LIST`, plus a report pipe and a cancel event;
  - `d3d11-consumer`: a D3D11 device opens three shared committed textures and both fences;
  - `resize`: `same-device`, with both queues drained and the ring rebuilt every 100 frames at three sizes.
- History, from packet `work/experiments/renderer-sb-packets/SB-C-handoff.md`:
  - A draft passed `same-device`, `second-device` and `resize` on the owner's GPU.
  - `d3d11-consumer` failed with `E_INVALIDARG` from `OpenSharedResource1` until the committed shared textures got `D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET`.
  - Codex implement call `20261003T052042Z-c85f556f` turned the draft into the current files. It was valid, and its acceptance check passed.
  - Integrator change after that call: the parent passes the ring extent to the child as `--width` and `--height`, instead of relying on the child's default of 1024 x 1024. The README has the matching sentence.

## 3. Environment and versions
- Windows 11 (10.0.26200), MSVC 19.51.36260 x64, Windows SDK 10.0.26100.0, CMake 4.4.3, Ninja 1.13.2, C++20.
- RTX 4070 Laptop GPU, NVIDIA user-mode driver 32.0.16.1692. The D3D12 and D3D11 debug layers are installed.
- The review sandbox is read-only and has no GPU. Evidence kind there: source only.

## 4. Necessary source and evidence
- Files under review, all in `work/experiments/renderer-sb/handoff/`:
  - `src/handoff.cpp`: device, queues, ring, the three topologies, child process and teardown;
  - `src/protocol.cpp` and `src/protocol.h`: ring arithmetic, pattern, argument parsing, JSON and exit policy, CPU self-test;
  - `src/main.cpp`, `src/handoff.h`;
  - `build.cmd`, `CMakeLists.txt`, `check_handoff.py`, `README.md`.
- Integrator's checks on the final source (`sb_handoff.exe` SHA-256 `1812e12e5d41ce7bc278bfade11d2d8abbd369db663899813e539213f3240601`):
  - `python work/experiments/renderer-sb/handoff/check_handoff.py` exits 0. It builds with NMake and runs `--selftest`.
  - The Ninja build through `build.cmd` has no warnings at `/W4`.
  - `--mode all --iterations 1000` on the RTX 4070:
    - with `--debug-layer`, every mode passes, 1,000 of 1,000 frames verified;
    - teardown passes with 0 unexpected live objects, and the child reports its own check;
    - `resize` rebuilt the ring 9 times;
    - the same build also passes every mode without the debug layer, and on WARP with it.
  - Sanitised per-run summary: `work/experiments/renderer-sb/results/handoff-20261003T052805Z-summary.json`.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | the packet as written | draft by Codex call `20261003T002449Z-00da5591` | wrapper validity | run invalid for an external reason (a ref change); draft kept |
| 2 | the draft works on the GPU | none | `--mode all` on the RTX 4070 | three modes pass; `d3d11-consumer` fails with `E_INVALIDARG` |
| 3 | D3D11 needs a render-target-capable shared texture | `ALLOW_RENDER_TARGET` on committed shared textures | `--mode d3d11-consumer` | pass |
| 4 | the draft plus attempt 3 is the design | implement call `20261003T052042Z-c85f556f`, then the extent change | section 4 checks | all pass |

## 6. Constraints and owned files
- Read-only review.
- Out of scope:
  - `work/experiments/renderer-sb/RESULT.md`, the results folder and the wiki pages: recorded measured results;
  - the S-B probe, the capture script and the gate tools;
  - timing numbers, which are information only;
  - the chosen design (three slots, `COMMON` at every handoff, a shared heap for the second device, committed textures for D3D11), unless it causes a wrong outcome from section 1;
  - style, `minor` and `nit` findings.
- Questions:
  1. Can any resource in the protocol be reused or read before the GPU work that used it has completed, in any topology, at a resize boundary or in the final drain?
  2. In `second-device`, does the parent count only frames that the child verified? Can a child failure end as `pass` or exit 0?
  3. On every failure path (child crash before or after its setup report, a timeout, an exception in the parent's produce loop, cancellation):
     - Is every wait bounded?
     - Are the queues drained before their resources are released?
     - Can a GPU queue wait on a fence value that is never signalled?
  4. Handles:
     - Is inheritance limited to the five listed handles?
     - Is any handle leaked or closed twice, in the parent or in the child?
     - Is the fixed-size report read over the anonymous pipe robust to a partial read?
  5. D3D12 and D3D11 rules: is anything invalid that the debug layer might not report on this driver? Cases:
     - `ALLOW_SIMULTANEOUS_ACCESS` textures with explicit `COMMON` transitions on compute and direct queues;
     - placed textures in a shared heap with `ALLOW_ONLY_NON_RT_DS_TEXTURES`;
     - `OpenSharedResource1` on committed render-target-capable textures;
     - D3D11 `Wait` and `Signal` on shared D3D12 fences.
  6. Does the teardown check (`ReportLiveDeviceObjects` with `DETAIL | IGNORE_INTERNAL`, failing on an empty report) support what the README claims?

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
