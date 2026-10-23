Changed only:

- [rules_kernel.py](research/jumbling/restoration/13-tutorial-runner/scripts/rules_kernel.py): replaced the stub with one-step kernel calls, record validation, result mapping, trees and provenance.
- [test_rules_kernel.py](research/jumbling/restoration/13-tutorial-runner/scripts/test_rules_kernel.py): added the packet’s acceptance tests and worktree check.

Acceptance command exited **0**: **26 tests, 1 skipped**, **83.982 s total**. Worktree unchanged during testing, including ignored files.

| Test | Seconds |
|---|---:|
| A1 import isolation | 0.071 |
| A2 script provenance | 0.105 |
| B1 lattice proposals | 0.833 |
| B2 budget proposal/handoff | 0.141 |
| B3 ST tree/splits | 0.942 |
| B4 clock cursors | 1.024 |
| B5 inconsistent results | 1.484 |
| B6 exhaustion | 0.917 |
| B7 unsupported | 0.437 |
| B8 exit/missing records | 0.983 |
| B8 record integrity | 2.035 |
| B8 word integrity | 1.291 |
| B9 replay digest rejection | 0.189 |
| B10 stale files | 0.147 |
| B11 wall kills | 4.271 |
| B12 clock arguments | 0.360 |
| B13 exhaustion continues | 0.181 |
| B13 missing dependency stops | 0.170 |
| B14 cancellation | 0.087 |
| B15 proposal metadata | 2.738 |
| B16 reason sanitization | 0.647 |
| B17 absent D19 provenance | 0.157 |
| C1 finite sentinel | 6.046 |
| C2 nested D19 clock | 18.655 |
| D real F1 | 38.267 |
| E slow D19 | 0.000, skipped |

F1 chose **[12, 14]** at R22D-36, producing digest `7cd868f6466953eb6ea1e0d466ed66857d34edaea7dd7c554c1e74975005541b`. Kernel-call durations were **37.689 s** at R22D-36 and **17.192 s** at END36 with the D19 budget stub.

`KERNEL_SLOW=1` was not run. No section-4 disagreement or new I0 interface change was found. The packet’s existing integration points remain: checker handling of integer `params.unit` and filling `config.kernel.d19`. Evidence is headless only.