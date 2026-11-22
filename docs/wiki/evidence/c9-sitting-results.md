---
id: c9-sitting-results
type: evidence
status: verified
visibility: public
summary: LP3's C9 sitting on the frozen candidate 013900ff, on the owner's RTX 4070 Laptop GPU, 21 and 22 November 2026. Charter E3 (1.89 to 1.91 s against 3 s), charter E2 in A3 (p95 82.7 ms) and in U8 (p95 82.6 ms, one failed key) and the renderer gate in W-J S4, I_a, I_b, W3 and W3 with user stages (60 fps at vsync, p99 about 17 ms) are met; LX-2 is not met (echo 76 to 93 ms against 50 ms in every fault, and the gate drops to 39 to 57 fps while the runtime is lost), and LX-4 is invalid (16 of 10,800 samples without a confirmed frame). Key to confirmed display takes 77 to 83 ms.
related: [owner-decisions-2026-11-21-ceilings-e3, owner-decisions-2026-11-20-lp3-exit, owner-decisions-2026-11-18-route-b, renderer-freeze, w-j-probe-results]
supersedes: []
claims:
  - {id: c9-a1-shell-launch, evidence_kind: actual_windows_directx, path: work/experiments/c9-sitting/results/final-a1-launch.json, sha256: 735f31009753eaa4c6fca4a7ce7d79e2099466beb669643057f078296d5ce2bc, checked_at: 2026-11-22}
  - {id: c9-a2-input-echo, evidence_kind: actual_windows_directx, path: work/experiments/c9-sitting/results/final-a2-launch.json, sha256: 1174594c59ca70cbabfd4f5c4b887efd0cbaa77d65a22c8956cb4c859c7ef336, checked_at: 2026-11-22}
  - {id: c9-a3-e2-met, evidence_kind: performance, path: work/experiments/c9-sitting/results/final-a3-e2.json, sha256: bd0f0cd90d2d6af26078f256496ad864a6c7fc144be0a998e7b7488b8b6de000, checked_at: 2026-11-22}
  - {id: c9-unattended-log, evidence_kind: actual_windows_directx, path: work/experiments/c9-sitting/results/unattended.log, sha256: 791d2324114716c574e11360c9cabe90f7d5dee59ab2426e2856f06f2734406f, checked_at: 2026-11-22}
  - {id: c9-u2-probe-1, evidence_kind: actual_windows_directx, path: work/experiments/c9-sitting/results/u2-1-probe-20261121T120338Z.md, sha256: 1aed00a0e046f51b510cfdba37f72ca7dad19233103adf60a0d1edebd5d671d9, checked_at: 2026-11-22}
  - {id: c9-u2-probe-2, evidence_kind: actual_windows_directx, path: work/experiments/c9-sitting/results/u2-2-probe-20261121T120451Z.md, sha256: 0614cf6025457955a5ff6241c1e4849d9ed50d5ecb333e8d23e9d22fca372c11, checked_at: 2026-11-22}
  - {id: c9-u2-probe-3, evidence_kind: actual_windows_directx, path: work/experiments/c9-sitting/results/u2-3-probe-20261121T120604Z.md, sha256: b684e9424698219cb6d7df9d244f4df7e79e688c48e1bcb3abcc0e790c21b660, checked_at: 2026-11-22}
  - {id: c9-u3-skew, evidence_kind: performance, path: work/experiments/c9-sitting/results/u3-final-skew.json, sha256: 4304b434b02db7152f5a90212b5133b37c0651352f08b80996882b732eaca7f2, checked_at: 2026-11-22}
  - {id: c9-u4-wj-s4-gate-met, evidence_kind: performance, path: work/experiments/c9-sitting/results/S4-summary-final.json, sha256: b09274f91e0ea06dc52a65c7487ed7f16f168e4184ffa9e3be11146d7c85ba47, checked_at: 2026-11-22}
  - {id: c9-u4-wj-i-a-gate-met, evidence_kind: performance, path: work/experiments/c9-sitting/results/I_a-summary-final.json, sha256: 36e0eed9602ce0bb1f76c32ec5f8a70764ef5ad40c066328112367ff72de8a3d, checked_at: 2026-11-22}
  - {id: c9-u4-wj-i-b-gate-met, evidence_kind: performance, path: work/experiments/c9-sitting/results/I_b-summary-final.json, sha256: 68181f470a8296ecee12cbadec30add14c48c360e2c2f0f81bf6bd1d6e10168b, checked_at: 2026-11-22}
  - {id: c9-u4-w3-gate-met, evidence_kind: performance, path: work/experiments/c9-sitting/results/w3-summary-final.json, sha256: c7a43ae68505f1c4df5d59400189070346765dad5a119333533e6a2ac968a0ca, checked_at: 2026-11-22}
  - {id: c9-u5-stages-gate-met, evidence_kind: performance, path: work/experiments/c9-sitting/results/w3-stages-summary-final.json, sha256: ebae348fa68537a7671cfe3317ae98f45f65a7907d4b1d6f95c1d89c8581cae8, checked_at: 2026-11-22}
  - {id: c9-u6-e3-warmup, evidence_kind: performance, path: work/experiments/c9-sitting/results/e3-final-warmup.json, sha256: bcde83fc721c4a1df752e7e13b10bb0d207cf290de79b0df314396b78b6a400b, checked_at: 2026-11-22}
  - {id: c9-u6-e3-met, evidence_kind: performance, path: work/experiments/c9-sitting/results/e3-final.json, sha256: 79227dd10dfdea107cfe80a6457fc2f11087d713fa5228f5853894b45dc9d161, checked_at: 2026-11-22}
  - {id: c9-u7-lx2-faults-agree, evidence_kind: actual_windows_directx, path: work/experiments/c9-sitting/results/u7-final-faults.json, sha256: 9d67b47e46b8b79b2668431473e0590227b8c2f5a4d53001d16a4db3d5cee4d8, checked_at: 2026-11-22}
  - {id: c9-u7-lx2-not-met, evidence_kind: performance, path: work/experiments/c9-sitting/results/u7-final-lx2.json, sha256: 670f65eca3cfed946d9f437acf6cb7f6390b6445675dc7de7cec99ac5f5c2725, checked_at: 2026-11-22}
  - {id: c9-u8-e2-met, evidence_kind: performance, path: work/experiments/c9-sitting/results/u8-final-e2.json, sha256: 92f7ee7877c719b2571c6d34ddb4adb322fc7ccaa6610331c3521e9499dd86f6, checked_at: 2026-11-22}
  - {id: c9-u8-lx4-invalid, evidence_kind: performance, path: work/experiments/c9-sitting/results/u8-final-lx4.json, sha256: 513c16d7c23afecb435f10b278ee1be3b32e400fd8a96149f80e1790761f6f20, checked_at: 2026-11-22}
---

# C9 sitting results

C9 is LP3's on-screen half on the selected renderer: route B, with the S-B Direct3D 12 core under Godot 4.7.2 .NET
([LP3 plan](../../progress/1.0/language-lp3-plan.md), section 8). Its owner-machine runs were made once, on the frozen
candidate, main `013900ffc013d4c63f0dd8c39d83836de2b7c443`, and on builds made from that commit
([owner decision](../decisions/owner-decisions-2026-11-21-ceilings-e3.md), item 3). The records are copied byte for
byte into `work/experiments/c9-sitting/results/` ([README](../../../work/experiments/c9-sitting/README.md)).

- **Unattended part (U1 to U8).** `tools/perf/c9/unattended.ps1 -Suffix final` ran from 11:39 to 16:57 UTC on 21
  November 2026. No run was aborted or retried, and no quiet-gate wait ran out (`unattended.log`).
- **Attended part (A1 to A3).** The owner ran A1 to A3 from 01:52 to 01:57 UTC on 22 November 2026, with the shell
  in the foreground throughout.

Builds:
- product build `c9-product-2efda50d54592f20` (shell, host and core), for A1 to A3, U3 and U6 to U8;
- route-B scene build `fb7dbd6e…`, for U4;
- stage build `f5b7531a…` (overdraw 108, DXC 1.9.2609), for U5.

Conditions, as the gate summaries record them:
- RTX 4070 Laptop GPU, driver 32.0.16.1692, on mains power at maximum performance;
- 2560 x 1600 at 60 Hz, vsync on, no tearing, no MSAA, frame generation, upscaling or overlays;
- unattended runs had no operator. Their automatic checks sample the window's visibility every 100 ms, which does not
  prove continuous full-area visibility.

The results hold for these builds on the owner's machine under these conditions only. Each record states what its
logs show (PHS.64), and none of them is a phase pass on its own (PHS.66).

## Verdicts per step

| Step | Check | Verdict | Numbers |
| --- | --- | --- | --- |
| A1 | ABI 5 GPU smoke, part 2: the shell's first launch | logs as required | Exit 0 after 38.5 s; 2,071 frames, 1,961 with a core-confirmed present, one device epoch and one surface; foreground throughout |
| A2 | Route B check B1: input through the transparent window | logs as required | Exit 0 after 61.7 s; 79 key presses and 30 clicks in `input.jsonl`; foreground throughout |
| A3 | Charter E2 | **met** | 222 keys, all confirmed; p95 82.7 ms, p99 82.8 ms, max 115.7 ms; PresentMon cross-check 0 disagreements |
| U1 | ABI 5 GPU smoke, part 1: the core self-test | ok | `check_core: ok (CPU, WARP and hardware)`, 35 CPU checks, identity recomputed |
| U2 | Route B check B3: three present-probe runs | exit 0 three times | Probe evidence, not a gate; see below |
| U3 | Route B check B2: UI frame to 3D frame skew | no bar | 32,177 matched pairs of 32,254 UI frames (77 with no confirmed 3D frame); absolute skew p50 0.017 ms, p95 0.049 ms, p99 0.284 ms, max 0.82 ms, all within one refresh |
| U4 | Renderer gate, W-J S4, I_a, I_b and W3 | **met** in each | Pooled 60.0 fps; p99 16.95, 16.95, 16.95 and 16.93 ms; VRAM peak 216, 216, 216 and 209 MB |
| U5 | RPI.79: W3 with user stages | **met** | Pooled 60.0 fps, p99 16.96 ms, max 17.99 ms, VRAM peak 488 MB |
| U6 | Charter E3 | **met** | 1.890 s, 1.895 s and 1.905 s against 3 s; the untimed warm-up took 1.899 s |
| U7 | LX-2 conditions 1 and 2 (faults record) | agree | Every trial's outcome is the expected one; each session stays open and replays equal to the control |
| U7 | LX-2 conditions 3 and 4 | **not met** | See below |
| U8 | Charter E2 | **met** | 602 keys, 601 confirmed, 1 failed key; pooled p95 82.63 ms; run p95 82.65 and 82.19 ms; PresentMon cross-check 0 disagreements |
| U8 | LX-4 | **invalid** | `one-valid-run-per-puzzle-needed`: 9 and 7 of 5,400 samples per puzzle without a confirmed frame |

## Attended steps

The shell's session control at close passed in A1, A2 and A3: the journal reopen and the kernel replay give the
shown state. A3's session holds 246 commits.

Some of A1's and A2's pass items are only visible on screen:
- the 3D view under the transparent window;
- two resizes without an error;
- each press and click on the echo panel.

The logs cannot show these: the shell writes no resize row, and `input.jsonl` records what Godot received, not what the
echo panel drew. They rest on the owner's observation.

## E3 and its cold definition

Each timed run was cold under the definition the owner accepted on 21 November 2026
([decision](../decisions/owner-decisions-2026-11-21-ceilings-e3.md), item 2):
- a new process, with no product process running at launch;
- a fresh session copy;
- 20 s since the previous exit;
- no reboot and no cache flush.

The warm-up launch came first and is recorded apart. Its record is `insufficient` by design: one run, never counted.
Both records were written before the decision page landed, so they still list the cold definition under `drafts`. The
C9 tools lift that mark in a later pull request.

## U2, the present probe

These are probe evidence for a PHS.52 design proposal, not product or gate evidence. In all six probe runs (three
control, three Godot-hosted), every steady frame was confirmed. Core removal left the host unaffected, and the
recovered frames were confirmed.

In the steady phase, the median from present to display is about 33 ms in the control runs and about 50 ms in the
Godot-hosted runs. That is two and three refreshes at 60 Hz. The second control run has one contradiction, in the
loss phase.

## LX-2 by fault

U7 ran the injection child `c600-lx2-child` in place of the build's runtime; launch.json and the faults record name
the same child. The candidate contains pull request 356, so the session job is the process host's job. Bars:
- echo, at most 50 ms at p95 and 67 ms at p99;
- the renderer gate, at least 30 fps and at most 33.3 ms at p99;
- charter E2, at most 100 ms at p95.

E2 was met in every fault. The run had 1,741 keys, 1,731 of them confirmed.

| Fault | Windows | E2 p95 (ms) | Echo p95 / p99 (ms) | Gate fps / p99 (ms) |
| --- | --- | --- | --- | --- |
| alloc_metered | 61 limit windows, none counted (see below) | 82.6 | insufficient, 0 timed | insufficient, 0 presents |
| recursion_metered | 2 limit windows, 123 s | 82.5 | 76.2 / 77.2: not met | 59.7 / 17.1: met |
| loop_metered | 2 limit windows, 120 s | 82.4 | 76.5 / 82.1: not met | 60.0 / 16.9: met |
| forced_kill | 148 turn windows | 82.6 | 90.0 / 92.8: not met | 57.2 / 52.2: not met |
| alloc_ceiling | 404 turn windows | 82.4 | 92.0 / 92.8, 84 timed: insufficient | 46.7 / 54.1: not met |
| recursion_native | 622 turn windows | 82.4 | 92.9 / 93.8: not met | 39.2 / 58.8: not met |
| loop_native | 2 turn windows, 120 s | 82.6 | 89.0 / 92.9: not met | 57.6 / 53.2: not met |

In the shell, alloc_metered's view job runs under the product's own view limits. All 61 of its limit windows ended at
`KN7:budget.kernel_cost`. `lx2.py` counts only `KN7:budget.memory` and the 60 s safety stop as this fault's limit ends,
which is the integrator's draft (`LIMIT_ENDS`). In the faults record, under the trial's limits, the same fault ends at
`KN7:budget.memory` as expected.

## LX-4

LX-4's draft rule makes a run invalid if any sample has no confirmed frame. Both puzzles' runs had such samples: 9 of
5,400 timed samples with Jumble off and 7 with Jumble on. So the record has no series table and no verdict on the bars.

The record keeps U4's W3 summary as the gate reference and does not count it as the gate. The owner rules on that
reference after the sitting ([decision](../decisions/owner-decisions-2026-11-21-ceilings-e3.md), "What follows").

Two limitations are in the record:
- the module set's views read no host rows, so both puzzles time the same rerun path;
- the rerun endpoint's frame is the first confirmed frame that drew the run's reference publication.

## Leads (not decisions)

1. **A latency floor.** A key press or a posted edit reaches a confirmed display in about 77 ms (echo, stale) to 83 ms
   (E2 turns): four to five refreshes. U2 shows about 50 ms from present to display in the Godot-hosted probe, so
   about three of those refreshes come after the present. That alone is above echo's 50 ms p95 bar.

   A read of the same LX-4 runs, outside the record with `lx4.py`'s own reader, puts the p95 values at:
   - echo, about 77 ms;
   - syntax at sizes S and M, about 94 ms against 67 ms;
   - every other metric within its bar.

   E2's 100 ms bar holds with about 17 ms to spare.
2. **Frames without a confirmation.** A small share of keys and endpoints never got a core-confirmed display:
   - 16 of 10,800 LX-4 samples;
   - 1 of 602 U8 keys, and none of 222 in A3;
   - 77 of 32,254 UI frames in U3.

   E2 counts such a key as a failed key in a valid run (owner, 20 November 2026, item 2). LX-4's draft rule invalidates
   the run instead.
3. **The gate under runtime loss.** The renderer gate drops to 39 to 57 fps, with p99 of 52 to 59 ms, while the runtime
   process dies and is restarted. It holds at 60 fps under the metered faults.
4. **alloc_metered's limit end.** Either `lx2.py`'s expected limit ends change (under PHS.52 where the text is frozen),
   or the fixture changes so that the memory limit comes first.

## What it does not show

- It is not LP3's exit. C10 needs the whole index on the frozen candidate and the owner's sign-off.
- It gives no LX-4 verdict on the bars, and no ruling on the gate reference.
- It gives no result for other hardware, resolutions or refresh rates, frame generation, or long sessions.
- It shows no A1 or A2 item that only the screen shows, beyond the owner's observation.
