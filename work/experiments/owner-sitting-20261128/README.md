# Owner-machine chain of 28 November 2026: records

These files record the unattended chain that ran on the owner's RTX 4070 Laptop GPU on 28 November 2026 (00:28:48 to
05:34:04 UTC), and the lattice replay that followed it. The results page is
[owner-sitting-2026-11-28-results](../../docs/wiki/evidence/owner-sitting-2026-11-28-results.md); the owner's decision
is [owner-decisions-2026-11-28](../../docs/wiki/decisions/owner-decisions-2026-11-28.md).

Builds: the product build is `c9-product-12d4a0923978eda1` and the scene build comes from the same commit `112a931a`.
The compare build for GAP is `c9-product-a9d908ad262ee306` (25 November 2026). The chain ran as
`unattended.ps1 -Steps U4,U7,U8,GAP -Suffix s1128`. No record holds a local path; run folders and raw logs stay on the
owner's machine. Nothing needed sanitizing.

| File | Role | sha256 |
| --- | --- | --- |
| `results/S4-summary-s1128.json` | U4, W-J S4: renderer gate summary, 3 runs | `a865d52f8aa6e5be27e08904ac3f06c95b144c0a4e4288a25a1db8eea732209e` |
| `results/I_a-summary-s1128.json` | U4, W-J I_a gate summary | `e35df5baff79af7ffda1fcff6ae91a9bb3e0d89f008b6c081fb4fde0551d0365` |
| `results/I_b-summary-s1128.json` | U4, W-J I_b gate summary | `a8c11a947f96565d0ec4ad3235b691b1cbbde2a5c260b6c7c5880911c6363db6` |
| `results/w3-summary-s1128.json` | U4, W3 gate summary, 3 runs; U8's accepted gate reference | `2d43c45ea82fe42118e585cdd6acbb5568b759d3069c6b14f934bbf90b02f0fa` |
| `results/u7-s1128-faults.json` | U7: LX-2 fault-injection record (conditions 1 and 2) | `d22ccf55af11dcdbde872f94df82dabf72f838c8c1d61fc3907b9e98eaf60857` |
| `results/u7-s1128-lx2.json` | U7: LX-2 analysis, verdict met | `e47286663391c7844d7ce7cc617db81f48de6f8004a9e0508dc5366dd7ec6808` |
| `results/u8-s1128-e2.json` | U8: key-to-frame (charter E2) record | `83b68a52ffbbf8beae16f443d53d3cb1243e6a38acee52ef9c735982b0e80a14` |
| `results/u8-s1128-lx4.json` | U8: LX-4 analysis as the chain wrote it; the gate reference was not yet accepted (verdict insufficient, decision null) | `8da5af44c6ffde9c956a2568c01bbc3c14bd23a6cab91a3d18f2d0f24b77ee93` |
| `results/u8-s1128-lx4-accepted.json` | U8: the same inputs re-analysed on 28 November with the gate reference entered as accepted (verdict met, decision all-bars) | `484ff9b573f05779b6f8fb5f9939f8c79a36becaa356fe725178c2523d2df277` |
| `results/gap-s1128-share.json` | GAP: share of frames without a presented row, per run | `99cefccb1127c3f4b8179d58e0da0be768243cbeee6a3d5aa6a2d46ddcff2f47` |
| `results/unattended-s1128.log` | The chain's log, the 28 November lines only (the 27 November attempt is left out) | `f82ca1ea3a524bf65ed2092ddba98abaeced683176723cd6235fa10f5609e860` |
| `results/chain-s1128-clocks.csv` | `nvidia-smi` every 30 s during the chain: time, P-state, graphics clock, memory clock, maximum memory clock, power, temperature, utilisation | `79b0ea21aa77942825cb375da643a44559097a87b29c6a258f4582a6f175a281` |
| `results/lattice-replay-s1128.json` | The lattice replay's facts (CT-LB5-05), written from the run's output and times | `8de8afbb1ac60162c3ea4c3d848b0a488f561a0fd9512ff3722efda37411e7c6` |

The re-analysis command (run from the retest checkout, with this repository's `lx4.py`):

    python -B tools/perf/c9/lx4.py analyze --fixtures <lx4-fixtures> <u8-off run> <u8-on run> --e2 <u8-e2 record> --gate-reference <w3 summary> --out results/u8-s1128-lx4-accepted.json
