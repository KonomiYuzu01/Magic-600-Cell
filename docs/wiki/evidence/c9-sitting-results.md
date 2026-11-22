---
id: c9-sitting-results
type: evidence
status: verified
visibility: public
summary: LP3's C9 sitting on the frozen candidate 013900ff, on the owner's RTX 4070 Laptop GPU, 21 and 22 November 2026. Charter E3 (1.89 to 1.91 s against 3 s), charter E2 in A3 (p95 82.7 ms) and in U8 (p95 82.6 ms, one failed key) and the renderer gate in W-J S4, I_a, I_b, W3 and W3 with user stages (60 fps at vsync, p99 about 17 ms) are met. Analysed again under the owner's 100 ms bars of 22 November 2026, LX-2 and LX-4 are still not met, only because of samples without a confirmed frame - LX-2's echo in four of seven faults (1 to 26 such samples each) and two syntax cells of LX-4 (two each) - while every timed latency is within its bar. Key to confirmed display takes 77 to 83 ms.
related: [owner-decisions-2026-11-22-lx-bars-freeze, owner-decisions-2026-11-21-ceilings-e3, owner-decisions-2026-11-20-lp3-exit, owner-decisions-2026-11-18-route-b, renderer-freeze, w-j-probe-results]
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
  - {id: c9-u7-lx2-not-met-100ms, evidence_kind: performance, path: work/experiments/c9-sitting/results/u7-final-lx2-20261122.json, sha256: 3fcca5790b3e61a67775d166d9209559c5e01594d1662216ea65a27163d6b667, checked_at: 2026-11-22}
  - {id: c9-u8-lx4-not-met-100ms, evidence_kind: performance, path: work/experiments/c9-sitting/results/u8-final-lx4-20261122.json, sha256: aa6026c1c41466bc83feea5b159296a917b29057e75fb152657203ee845d435a, checked_at: 2026-11-22}
---

# C9 sitting results

C9 is LP3's on-screen half on the selected renderer: route B, with the S-B Direct3D 12 core under Godot 4.7.2 .NET
([LP3 plan](../../progress/1.0/language-lp3-plan.md), section 8). Its owner-machine runs were made once, on the frozen
candidate, main `013900ffc013d4c63f0dd8c39d83836de2b7c443`, and on builds made from that commit
([owner decision](../decisions/owner-decisions-2026-11-21-ceilings-e3.md), item 3). The records are copied byte for
byte into `work/experiments/c9-sitting/results/` ([README](../../../work/experiments/c9-sitting/README.md)). The two
analyses of the recorded U7 and U8 data made again on 22 November 2026, with no new run, are written there beside them
([owner decision](../decisions/owner-decisions-2026-11-22-lx-bars-freeze.md), items 1 to 5).

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
| A1 | ABI 5 GPU smoke, part 2: the shell's first launch | **pass** (logs and the owner's report) | Exit 0 after 38.5 s; 2,071 frames, 1,961 with a core-confirmed present, one device epoch and one surface; foreground throughout |
| A2 | Route B check B1: input through the transparent window | **pass** (logs and the owner's report) | Exit 0 after 61.7 s; 79 key presses and 30 clicks in `input.jsonl`; foreground throughout |
| A3 | Charter E2 | **met** | 222 keys, all confirmed; p95 82.7 ms, p99 82.8 ms, max 115.7 ms; PresentMon cross-check 0 disagreements |
| U1 | ABI 5 GPU smoke, part 1: the core self-test | ok | `check_core: ok (CPU, WARP and hardware)`, 35 CPU checks, identity recomputed |
| U2 | Route B check B3: three present-probe runs | exit 0 three times | Probe evidence, not a gate; see below |
| U3 | Route B check B2: UI frame to 3D frame skew | no bar | 32,177 matched pairs of 32,254 UI frames (77 with no confirmed 3D frame); absolute skew p50 0.017 ms, p95 0.049 ms, p99 0.284 ms, max 0.82 ms, all within one refresh |
| U4 | Renderer gate, W-J S4, I_a, I_b and W3 | **met** in each | Pooled 60.0 fps; p99 16.95, 16.95, 16.95 and 16.93 ms; VRAM peak 216, 216, 216 and 209 MB |
| U5 | RPI.79: W3 with user stages | **met** | Pooled 60.0 fps, p99 16.96 ms, max 17.99 ms, VRAM peak 488 MB |
| U6 | Charter E3 | **met** | 1.890 s, 1.895 s and 1.905 s against 3 s; the untimed warm-up took 1.899 s |
| U7 | LX-2 conditions 1 and 2 (faults record) | agree | Every trial's outcome is the expected one; each session stays open and replays equal to the control |
| U7 | LX-2 conditions 3 and 4 | **not met** | Under the 100 ms bars of 22 November: the echo bar fails in alloc_ceiling, forced_kill, loop_native and recursion_metered, each on 1 to 26 samples without a confirmed frame; every timed echo p95 and p99 is at most 93.8 ms. See below |
| U8 | Charter E2 | **met** | 602 keys, 601 confirmed, 1 failed key; pooled p95 82.63 ms; run p95 82.65 and 82.19 ms; PresentMon cross-check 0 disagreements |
| U8 | LX-4 | **not met** | Under the 100 ms bars and E2's counting of 22 November: both runs valid; two syntax cells miss their p99 bar on two samples without a confirmed frame each (`fail-s-m`). The first analysis read `invalid` |

## Attended steps

The shell's session control at close passed in A1, A2 and A3: the journal reopen and the kernel replay give the
shown state. A3's session holds 246 commits.

Some of A1's and A2's pass items are only visible on screen:
- the 3D view under the transparent window;
- two resizes without an error;
- each press and click on the echo panel.

The logs cannot show these: the shell writes no resize row, and `input.jsonl` records what Godot received, not what the
echo panel drew. They rest on the owner's observation. After the sitting, on 22 November 2026, the owner reported in
chat that A1 and A2 were normal: the window resized normally, and the echo panel showed the presses.

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
the same child. The candidate contains pull request 356, so the session job is the process host's job.

On 22 November 2026 the recorded U7 data was analysed again under the owner's decision of that day
([decision](../decisions/owner-decisions-2026-11-22-lx-bars-freeze.md), items 1, 3, 4 and 5), with no new run. The
record is `u7-final-lx2-20261122.json`, made by `lx2.py` at sha256 `a96a1fe6…` with `lx4.py` at `33cfd55c…`. The
first analysis, `u7-final-lx2.json`, stays unchanged as evidence. Bars and rules:
- echo, at most 100 ms at p95 and at p99 (item 1). An echo sample in a fault window without a confirmed frame fails
  the bar; that is LX-2's draft rule, and the decision does not change it;
- the renderer gate, at least 30 fps and at most 33.3 ms at p99, in the metered faults. In forced_kill, alloc_ceiling,
  recursion_native and loop_native, where the runtime is lost, its figures are computed and recorded as a known limit,
  not judged (item 4);
- charter E2, at most 100 ms at p95;
- alloc_metered's limit windows may end at `KN7:budget.kernel_cost` (item 3);
- alloc_ceiling's 84 timed echo samples are accepted for that fault as the owner's exception, not as sufficient by
  rule (item 5).

E2 was met in every fault. The run had 1,741 keys, 1,731 of them confirmed. Conditions 1 and 2 agree (faults record).

| Fault | Windows | E2 p95 (ms) | Echo timed p95 / p99 (ms) | Echo samples without a confirmed frame | Echo | Gate fps / p99 (ms) |
| --- | --- | --- | --- | --- | --- | --- |
| alloc_metered | 61 limit windows, 60 s, all ended at kernel_cost | 82.6 | 77.1 / 77.4, 110 timed | 0 of 110 | **met** | 59.8 / 19.4: met |
| recursion_metered | 2 limit windows, 123 s | 82.5 | 76.2 / 77.2, 211 timed | 1 of 212 | **not met** | 59.7 / 17.1: met |
| loop_metered | 2 limit windows, 120 s | 82.4 | 76.5 / 82.1, 209 timed | 0 of 209 | **met** | 60.0 / 16.9: met |
| forced_kill | 148 turn windows | 82.6 | 90.0 / 92.8, 102 timed | 8 of 110 | **not met** | 57.2 / 52.2: known limit |
| alloc_ceiling | 404 turn windows | 82.4 | 92.0 / 92.8, 84 timed (accepted, item 5) | 26 of 110 | **not met** | 46.7 / 54.1: known limit |
| recursion_native | 622 turn windows | 82.4 | 92.9 / 93.8, 110 timed | 0 of 110 | **met** | 39.2 / 58.8: known limit |
| loop_native | 2 turn windows, 120 s | 82.6 | 89.0 / 92.9, 188 timed | 13 of 201 | **not met** | 57.6 / 53.2: known limit |

So LX-2 is **not met**: alloc_ceiling, forced_kill, loop_native and recursion_metered fail the echo bar. Every timed
echo p95 and p99 is within 100 ms. Each of the four fails only because some of its echo samples had no confirmed frame.
Item 5 accepts alloc_ceiling's number of timed samples; it does not cover that fault's 26 samples without a confirmed
frame.

The record also reports each fault's echo percentiles with those samples counted as failed, as charter E2 counts a
key. These are reported, not judged. Counted that way, recursion_metered reads p95 76.3 ms and p99 78.2 ms, within the
bar; forced_kill, alloc_ceiling and loop_native have no p95 or p99 within the bar.

In the shell, alloc_metered's view job runs under the product's own view limits, and all 61 of its limit windows ended
at `KN7:budget.kernel_cost`, which now counts (item 3). In the faults record, under the trial's limits, the same fault
ends at `KN7:budget.memory`.

Under RTE.21 a failing LX-2 run reopens LQ9, and the numbers go to the owner
([decision](../decisions/owner-decisions-2026-11-22-lx-bars-freeze.md), "What follows").

## LX-4

The first analysis, `u8-final-lx4.json`, read `invalid`: LX-4's draft rule made a run invalid if any sample had no
confirmed frame, and both puzzles' runs had such samples (9 of 5,400 with Jumble off and 7 with Jumble on). It stays
unchanged as evidence.

On 22 November 2026 the recorded U8 data was analysed again under the owner's decision of that day
([decision](../decisions/owner-decisions-2026-11-22-lx-bars-freeze.md), items 1 and 2), with no new run. The record is
`u8-final-lx4-20261122.json`, made by `lx4.py` at sha256 `33cfd55c…`. Rules:
- a sample without a confirmed frame counts as one failed sample, as charter E2 counts a key, and the run stays valid.
  A percentile then has a value only if it falls on a timed sample within its bar;
- typed echo is 100 / 100 ms at p95 / p99 at S, M and L, and syntax feedback is 100 / 100 ms at S and M. Every other
  RTE.49 bar is unchanged, syntax feedback at L (100 / 150 ms) among them;
- U4's W3 summary (`w3-summary-final.json`, met) counts as the renderer gate reference.

Both runs are valid. Of the S, M and L samples, 10,786 were timed and 14 failed; 2 W samples failed. U8's charter E2 is
met. LX-4 is **not met** (`bar-missed`, decision `fail-s-m`). Two cells miss their p99 bar, each because two of its 100
samples had no confirmed frame:
- Jumble on, series 1, syntax feedback at S (100 / 100 ms): 98 timed, p95 94.3 ms, no p99 within the bar;
- Jumble off, series 2, syntax feedback at L (100 / 150 ms): 98 timed, p95 94.1 ms, no p99 within the bar.

Every timed p95 and p99 is within its bar. Ranges over the series and both puzzles:

| Interval (`lx4.py` name) | Sizes | Bars p95 / p99 (ms) | Timed p95 (ms) | Timed p99 (ms) |
| --- | --- | --- | --- | --- |
| `echo` | S, M, L | 100 / 100 | 77.0 to 77.7 | 77.5 to 82.6 |
| `syntax` | S, M | 100 / 100 | 93.7 to 94.3 | 94.0 to 94.8 |
| `syntax` | L | 100 / 150 | 93.2 to 94.1 | 93.7 to 94.7 |
| `stale` | S, M, L | 100 / 150 | 76.7 to 77.3 | 77.0 to 82.1 |
| `completion` | S, M; L | 100 / 150; 150 / 250 | 93.1 to 93.8 | 93.5 to 98.5 |
| `diagnostics` | S; M; L | 150 / 250; 200 / 300; 500 / 750 | 93.3 to 110.6 | 93.9 to 115.5 |
| `rerun` | S, M; L | 250 / 400; 500 / 750 | 110.2 to 157.7 | 110.7 to 160.0 |
| `dependents` | W | 1000 / 1500 | 193.7 to 206.5 | 194.1 to 208.7 |

Under RTE.21 a failing LX-4 run reopens LQ9, and the numbers go to the owner.

Two limitations are in the record:
- the module set's views read no host rows, so both puzzles time the same rerun path;
- the rerun endpoint's frame is the first confirmed frame that drew the run's reference publication.

## Leads (not decisions)

1. **A latency floor.** A key press or a posted edit reaches a confirmed display in about 77 ms (echo, stale) to 83 ms
   (E2 turns): four to five refreshes. U2 shows about 50 ms from present to display in the Godot-hosted probe, so
   about three of those refreshes come after the present. Syntax feedback takes about 94 ms, and echo in the faults
   that lose the runtime 89 to 94 ms: within 100 ms, with 6 to 11 ms to spare. E2's 100 ms bar holds with about 17 ms
   to spare.
2. **Frames without a confirmation.** A small share of keys and endpoints never got a core-confirmed display:
   - 48 of 1,062 echo samples in LX-2's fault windows, 47 of them in the faults that lose the runtime;
   - 16 of 10,800 LX-4 samples;
   - 1 of 602 U8 keys, and none of 222 in A3;
   - 77 of 32,254 UI frames in U3.

   E2 counts such a key as a failed key in a valid run (owner, 20 November 2026, item 2), and LX-4 now counts a sample
   the same way (owner, 22 November 2026, item 2). LX-2's echo bar fails on one such sample in a fault window. These
   samples alone decide both failing verdicts. Whether such a frame was dropped, or its endpoint was shown by the next
   presented frame, is not checked.
3. **The gate under runtime loss.** The renderer gate drops to 39 to 57 fps, with p99 of 52 to 59 ms, while the runtime
   process dies and is restarted. It holds at 60 fps under the metered faults. It is now a known limit (owner, 22
   November 2026, item 4), and removing the stutter is product work after the freeze.

## What it does not show

- It is not LP3's exit. C10 needs the whole index on the frozen candidate and the owner's sign-off.
- It records no owner answer on LQ9. Under RTE.21 the failing LX-2 and LX-4 runs reopen it, and the numbers go to
  the owner.
- Chapter 11's RTE.42, RTE.49 and RTE.53 keep the draft bars until LP3's PHS.52 amendment cites the decision of 22
  November 2026.
- It gives no result for other hardware, resolutions or refresh rates, frame generation, or long sessions.
- For A1 and A2, the items that only the screen shows rest on the owner's report, not on a log or a capture.
