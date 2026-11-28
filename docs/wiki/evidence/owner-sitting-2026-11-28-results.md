---
id: owner-sitting-2026-11-28-results
type: evidence
status: verified
visibility: public
summary: The owner-machine chain of 28 November 2026 (00:28 to 05:34 UTC) and the lattice replay, on the owner's RTX 4070 Laptop GPU with the GPU overclock off, and the owner's acceptance of all of it as passed. The product build c9-product-12d4a0923978eda1 and a scene build from the same commit 112a931a met every bar. U4's renderer gate is met in W-J S4, I_a, I_b and W3 (3 runs each, 60.0 fps, p99 16.9 ms). U7's LX-2 reads met in all seven faults, with no key or echo sample lacking a confirmed frame in the E2 runs. U8's LX-4 reads met with all 152 series inside their bars over 10,800 timed samples and none unconfirmed; the analysis reads all-bars once the owner accepted the s1128 W3 summary as the gate reference. GAP: frames without a presented row fall from 13.3% on the 25 November build to 0.06 and 0.07% on the current build. The lattice replay (CT-LB5-05, 13,231,266 letters) passed in 28,802.7 s. An earlier attempt on 27 November measured nothing because the shell was started minimised.
related: [owner-decisions-2026-11-28, owner-sitting-2026-11-25-results, presented-gap-2026-11-26, key-stall-2026-11-26, owner-decisions-2026-11-26-release-order, owner-decisions-2026-11-22-lx-bars-freeze]
supersedes: []
claims:
  - {id: os1128-owner-accepts-results, evidence_kind: decision, path: docs/wiki/decisions/owner-decisions-2026-11-28.md, line: 24, sha256: 18fd0ad0bf26d2d387e38f4a853e888bd83a66bcb124fb05fae3fc6f9e99259d, checked_at: 2026-11-28}
  - {id: os1128-no-rerun-rule, evidence_kind: decision, path: docs/wiki/decisions/owner-decisions-2026-11-28.md, line: 39, sha256: 18fd0ad0bf26d2d387e38f4a853e888bd83a66bcb124fb05fae3fc6f9e99259d, checked_at: 2026-11-28}
  - {id: os1128-lp6-parallel, evidence_kind: decision, path: docs/wiki/decisions/owner-decisions-2026-11-28.md, line: 47, sha256: 18fd0ad0bf26d2d387e38f4a853e888bd83a66bcb124fb05fae3fc6f9e99259d, checked_at: 2026-11-28}
  - {id: os1128-u4-s4, evidence_kind: performance, path: work/experiments/owner-sitting-20261128/results/S4-summary-s1128.json, sha256: a865d52f8aa6e5be27e08904ac3f06c95b144c0a4e4288a25a1db8eea732209e, checked_at: 2026-11-28}
  - {id: os1128-u4-i-a, evidence_kind: performance, path: work/experiments/owner-sitting-20261128/results/I_a-summary-s1128.json, sha256: e35df5baff79af7ffda1fcff6ae91a9bb3e0d89f008b6c081fb4fde0551d0365, checked_at: 2026-11-28}
  - {id: os1128-u4-i-b, evidence_kind: performance, path: work/experiments/owner-sitting-20261128/results/I_b-summary-s1128.json, sha256: a8c11a947f96565d0ec4ad3235b691b1cbbde2a5c260b6c7c5880911c6363db6, checked_at: 2026-11-28}
  - {id: os1128-u4-w3, evidence_kind: performance, path: work/experiments/owner-sitting-20261128/results/w3-summary-s1128.json, sha256: 2d43c45ea82fe42118e585cdd6acbb5568b759d3069c6b14f934bbf90b02f0fa, checked_at: 2026-11-28}
  - {id: os1128-u7-faults, evidence_kind: actual_windows_directx, path: work/experiments/owner-sitting-20261128/results/u7-s1128-faults.json, sha256: d22ccf55af11dcdbde872f94df82dabf72f838c8c1d61fc3907b9e98eaf60857, checked_at: 2026-11-28}
  - {id: os1128-u7-lx2, evidence_kind: performance, path: work/experiments/owner-sitting-20261128/results/u7-s1128-lx2.json, sha256: e47286663391c7844d7ce7cc617db81f48de6f8004a9e0508dc5366dd7ec6808, checked_at: 2026-11-28}
  - {id: os1128-u8-e2, evidence_kind: performance, path: work/experiments/owner-sitting-20261128/results/u8-s1128-e2.json, sha256: 83b68a52ffbbf8beae16f443d53d3cb1243e6a38acee52ef9c735982b0e80a14, checked_at: 2026-11-28}
  - {id: os1128-u8-lx4-original, evidence_kind: performance, path: work/experiments/owner-sitting-20261128/results/u8-s1128-lx4.json, sha256: 8da5af44c6ffde9c956a2568c01bbc3c14bd23a6cab91a3d18f2d0f24b77ee93, checked_at: 2026-11-28}
  - {id: os1128-u8-lx4-accepted, evidence_kind: performance, path: work/experiments/owner-sitting-20261128/results/u8-s1128-lx4-accepted.json, sha256: 484ff9b573f05779b6f8fb5f9939f8c79a36becaa356fe725178c2523d2df277, checked_at: 2026-11-28}
  - {id: os1128-gap-share, evidence_kind: actual_windows_directx, path: work/experiments/owner-sitting-20261128/results/gap-s1128-share.json, sha256: 99cefccb1127c3f4b8179d58e0da0be768243cbeee6a3d5aa6a2d46ddcff2f47, checked_at: 2026-11-28}
  - {id: os1128-chain-log, evidence_kind: actual_windows_directx, path: work/experiments/owner-sitting-20261128/results/unattended-s1128.log, sha256: f82ca1ea3a524bf65ed2092ddba98abaeced683176723cd6235fa10f5609e860, checked_at: 2026-11-28}
  - {id: os1128-clock-log, evidence_kind: actual_windows_directx, path: work/experiments/owner-sitting-20261128/results/chain-s1128-clocks.csv, sha256: 79b0ea21aa77942825cb375da643a44559097a87b29c6a258f4582a6f175a281, checked_at: 2026-11-28}
  - {id: os1128-lattice, evidence_kind: source, path: work/experiments/owner-sitting-20261128/results/lattice-replay-s1128.json, sha256: 8de8afbb1ac60162c3ea4c3d848b0a488f561a0fd9512ff3722efda37411e7c6, checked_at: 2026-11-28}
  - {id: os1128-gate-reference-entry, evidence_kind: source, path: tools/perf/c9/lx4.py, line: 162, sha256: dd4c07364ee4d90e38748ec01fc3a5ea69ef20398d51853af103cc6ac19e2154, checked_at: 2026-11-28}
---

# Owner-machine chain of 28 November 2026

The [25 November sitting](owner-sitting-2026-11-25-results.md) left three things open: LX-2 and LX-4 read not-met
because 15 to 18% of frames had no confirmed present, the gate reference for U8 did not count, and the lattice replay
had not run. This chain re-ran U4, U7, U8 and GAP on builds that carry the fixes of 26 and 27 November
([presented-gap-2026-11-26](presented-gap-2026-11-26.md), [key-stall-2026-11-26](key-stall-2026-11-26.md)), and the
lattice replay followed it. The records are in `work/experiments/owner-sitting-20261128/`
([README](../../../work/experiments/owner-sitting-20261128/README.md)).

The owner accepted every result as passed on 28 November 2026 ([decision](../decisions/owner-decisions-2026-11-28.md),
item 1). This page records what each result shows. Under PHS.66 the records are evidence for that reading.

## What ran

- **Builds.** Product build `c9-product-12d4a0923978eda1`; scene build from the same commit `112a931a`. GAP also ran the
  25 November product build `c9-product-a9d908ad262ee306` for comparison.
- **Command.** `unattended.ps1 -Steps U4,U7,U8,GAP -Suffix s1128`, started in an administrator PowerShell with a normal
  window. The log (`results/unattended-s1128.log`) runs from 00:28:48 to 05:34:04 UTC and ends `exit=0`. Every step
  exited 0 except U8's first analysis, which exited 3 (below).
- **Machine.** The owner's RTX 4070 Laptop GPU. The owner turned the Vantage overclock off on 27 November 2026 at about
  19:47 UTC. These are actual Windows/DirectX and performance results for this machine and these builds only.
- **Clocks.** `nvidia-smi` every 30 s (`results/chain-s1128-clocks.csv`, 610 rows, 00:28:48 to 05:33:47 UTC): the
  maximum-memory-clock column reads 8001 MHz in every row, and no memory clock reading exceeds it. Under load (31 rows at
  or above 50% utilisation) the memory clock reads 8001 MHz in 25 rows and 810 MHz in 6 rows at P0; the peak is 75 C and
  18.4 W. The log does not say why 6 loaded rows read 810 MHz, and it is not judged here.

## U4: the renderer gate on the scene build

| Scene | Valid runs | Verdict | Pooled fps / p99 (ms) | VRAM peak (MB) |
| --- | --- | --- | --- | --- |
| W-J S4 | 3 | **met** | 60.0 / 16.93 | 216 |
| W-J I_a | 3 | **met** | 60.0 / 16.94 | 216 |
| W-J I_b | 3 | **met** | 60.0 / 16.93 | 216 |
| W3 | 3 | **met** | 60.0 / 16.92 | 209 |

On 25 November W3 had only 2 valid runs and read `insufficient-runs`; now every scene has 3 runs and meets the gate
(fps at least 30, p99 at most 33.3 ms, VRAM far below 7 GB).

## U7: LX-2 fault injection

The faults record agrees (`results/u7-s1128-faults.json`) and the analysis (`results/u7-s1128-lx2.json`) reads
**met**: conditions 1 and 2 are met, all 1,544 E2 keys have a confirmed frame, and every fault's gate and echo bar hold.

| Fault | Gate fps / p99 (ms) | E2 keys unconfirmed | E2 p95 (ms) | Echo samples unconfirmed | Echo p95 (ms) |
| --- | --- | --- | --- | --- | --- |
| alloc_metered | 60.0 / 16.8 | 0 of 110 | 82.6 | 0 of 111 | 76.9 |
| recursion_metered | 60.0 / 16.9 | 0 of 110 | 82.5 | 0 of 115 | 77.4 |
| loop_metered | 60.0 / 16.8 | 0 of 110 | 82.6 | 0 of 112 | 77.5 |
| forced_kill | 60.0 / 16.9 | 0 of 142 | 82.6 | 1 of 109 | 77.3 |
| alloc_ceiling | 59.1 / 24.6 | 0 of 365 | 82.5 | 1 of 109 | 76.2 |
| recursion_native | 59.9 / 17.8 | 0 of 589 | 82.5 | 0 of 110 | 76.5 |
| loop_native | 60.0 / 16.8 | 0 of 118 | 98.8 | 0 of 114 | 77.7 |

The stutter under runtime loss that the 25 November sitting found (38 to 58 fps, p99 55 to 61 ms) does not appear: the
worst gate in the runtime-loss faults is 59.1 fps and p99 24.6 ms (alloc_ceiling). loop_native's E2 p95 is 98.8 ms, under
the 100 ms bar by little.

**Why the record's `decision` is null.** `lx2.py` writes no `decision` field at all; only `lx4.py` does (its `judge`
returns one, and `build` stores it in the result). LX-2's outcome is its `verdict`, here `met`, with `reasons` empty. A
null decision for U7 is the absence of the field, not an unresolved outcome.

## U8: LX-4's editor bars

U8 ran LX-4's fixtures for about 101 minutes with Jumble off and about 101 minutes with Jumble on. Both runs are valid.
- **E2** (`results/u8-s1128-e2.json`): 301 keys per run, all confirmed. p95 is 82.7 ms (Jumble off) and 99.0 ms (Jumble
  on); p99 is 99.2 and 149.1 ms. The E2 verdict is **met**. The Jumble-on p95 sits 1 ms under the 100 ms bar.
- **LX-4 as the chain wrote it** (`results/u8-s1128-lx4.json`): all 152 series meet their bars, over 10,800 timed S, M
  and L samples with none unconfirmed, but the verdict is `insufficient` with the reason `gate-summary-other-build` and
  the decision null, so the chain's analysis exited 3. The only reason is that U4's W3 summary comes from the scene build
  and was not yet accepted as the gate reference ([LX bars decision](../decisions/owner-decisions-2026-11-22-lx-bars-freeze.md),
  item 2).
- **LX-4 with the accepted reference** (`results/u8-s1128-lx4-accepted.json`): after the owner's acceptance, the entry
  for `w3-summary-s1128.json` (sha256 `2d43c45e...`) and the product build is in `ACCEPTED_GATE_REFERENCES`
  (`tools/perf/c9/lx4.py`). The same inputs now read verdict **met**, decision **all-bars**, exit 0, with
  `counted_as_gate` true and `accepted_by` naming the 28 November decision. The measurements equal the original
  record's; only the reference's status differs.

Worst series per kind and size, p95 / p99 in ms:

| Kind | S | M | L |
| --- | --- | --- | --- |
| Echo | 78.2 / 78.6 | 78.1 / 78.5 | 78.0 / 78.2 |
| Syntax | 94.8 / 95.0 | 94.9 / 95.4 | 94.8 / 94.9 |
| Stale | 78.0 / 82.0 | 77.9 / 78.2 | 77.7 / 77.8 |
| Completion | 94.4 / 98.4 | 94.4 / 96.1 | 111.0 / 131.4 |
| Diagnostics | 94.5 / 95.0 | 94.6 / 94.9 | 127.8 / 128.0 |
| Rerun | 126.3 / 127.3 | 125.3 / 126.9 | 159.3 / 160.2 |

W dependents read at most 210.4 ms p95 and 224.7 ms p99 against 1,000 and 1,500 ms.

**L completion and L diagnostics.** On 25 November they read worst p95 144.7 and 177.8 ms; now 111.0 and 127.8 ms,
nearer to 21 November's 93.8 and 110.6 ms. The headless fix of 26 November (a shared parse and an indexed name lookup in
the editor services) therefore shows on the device, and L completion's headroom against its 150 ms bar is about 39 ms
instead of 5 ms.

## GAP: frames without a presented row

GAP ran ten-minute runs with Jumble off in the default window (`results/gap-s1128-share.json`; no bar):

| Build | Run | Frames | Without a `presented` row | Share |
| --- | --- | --- | --- | --- |
| `c9-product-a9d908ad262ee306` (25 November) | 20261128T045742Z | 35,847 | 4,758 | 13.27% |
| `c9-product-12d4a0923978eda1` (current) | 20261128T050951Z | 35,893 | 23 | 0.06% |
| `c9-product-12d4a0923978eda1` (current) | 20261128T052200Z | 35,903 | 24 | 0.07% |

On the same machine, window and night, the old build still loses 13% of its frames and the current build loses almost
none. This agrees with [presented-gap-2026-11-26](presented-gap-2026-11-26.md), which traced the gap to a sampling race
and fixed it in the core. It also explains why U7 and U8 now read almost no unconfirmed keys or samples.

## The lattice replay

CT-LB5-05, `s18::long::ct_lb5_05_lattice_replay_in_place`, ran from branch `claude/determined-lehmann-e0c74b` at commit
`42a72cc23c1baee7f5453b1a896913ac4b1dff3f` (`cargo test --release --locked --offline -p c600-checkers --test s18 --
--ignored --exact ...`) from 2026-11-28T05:34:05Z to 13:34:08Z. Cargo exited 0 and the test printed
`lattice replay: Pass in 28802.7 s` and `test result: ok. 1 passed` (`results/lattice-replay-s1128.json`).

It replays the 13,231,266 letters of the archived run `results/lattice-d1/run/run.json`. By the checker's code, a Pass
means every step was decoded to its declared letter count, every twist was admitted and every step's after-digest
matched. This is a headless source result: it says nothing about the GPU, input or frames. An earlier attempt (27
November 23:39 to 28 November 00:28 UTC) was stopped so that the chain could run and is not a result.

## The failed attempt of 27 November

The chain first ran on 27 November 23:22 to 23:39 UTC with the suffix s1127. Every run aborted `not-taken` with exit 3
("the framework window left the foreground or a condition sample failed"): the elevated shell had been started
minimised and deferred, so no run took the foreground. It measured nothing and produced no evidence; its lines are left
out of the committed log. The 28 November chain used a normal window started right after the UAC answer.

## The no-rerun rule

The owner's rule of 27 November ([decision](../decisions/owner-decisions-2026-11-28.md), item 2) was not needed: no step
showed a problem that required a fix and a rerun.

## What it does not show

- No result for other hardware, resolutions or refresh rates, or for long solver sessions.
- The runs are the harness's, not a solver's; the human tests and the on-screen twist-visibility finding of 25 November
  are not re-tested here.
- The GAP runs cover Jumble off in the default window only.
- The six loaded clock rows at 810 MHz are not explained.
