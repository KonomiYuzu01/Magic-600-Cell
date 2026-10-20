Implemented P3 in three files:

- [literal_lattice.py](research/jumbling/restoration/13-tutorial-runner/scripts/literal_lattice.py): added observer callbacks and exact I_a codes; observed poses now move twist by twist.
- [lattice_phase.py](research/jumbling/restoration/13-tutorial-runner/scripts/lattice_phase.py): replaced the stub with the plugin, child player, streamed steps, bounded trees, and output validation.
- [test_lattice_phase.py](research/jumbling/restoration/13-tutorial-runner/scripts/test_lattice_phase.py): added acceptance coverage. `test_literal_lattice.py` is unchanged.

The required acceptance command **exited 0**: 23 tests, one skipped, plus four passing legacy tests. Suite time: **180.415 s**; total acceptance: **182.219 s**. The worktree remained unchanged during the check, including ignored files.

| Test | Seconds |
|---|---:|
| A — module contract/startup | 6.874 |
| B1 — observer codes, reports, orders | 26.440 |
| B2 — interrupted prefix/exception | 3.915 |
| B3 — all signed primitives | 1.385 |
| C1 — four conjugations | 0.001 |
| C2 — backward references | 0.000 |
| C3 — empty nodes/depth/size folding | 0.002 |
| C4 — correction repetitions | 0.004 |
| D1 — synthetic plugin | 11.654 |
| D2 — corrected plugin/runner | 12.269 |
| D3 — zero steps | 11.240 |
| E — four conjugation children | 47.078 |
| F — invisible rotation/runner | 11.994 |
| G1 — player budget | 8.425 |
| G2 — word/model tampering | 9.632 |
| G3 — restorer output validation | 0.424 |
| G4 — restorer outcomes/entry | 0.373 |
| G5 — child outcomes | 0.276 |
| G6 — play-output tampering | 0.281 |
| H — real late restorer/budget | 22.273 |
| I — restorer argv | 0.033 |
| J — legacy subset, total | 5.839 |
| ↳ expansion | 0.002 |
| ↳ deep expansion | 0.004 |
| ↳ pose transport | 1.795 |
| ↳ invisible rotation | 2.297 |
| K — schedule11 slow run | skipped |

Cloud research timings: observer play covered 24 twists in 0.018810 s (1,275.9 twists/s); simulator startup was 6.746 s. The first `--_play` child took 10.179 s, including 9.865 s of startup, validation, and finalization overhead. H’s restorer took **13.021 s**. Acceptance started **12 `--_play` children**; neither E nor H was moved.

`LATTICE_PHASE_SLOW=1`, the full legacy suite, and Windows/DirectX checks were not run. No fixture disagreement occurred. I1 still needs zero-step recording, lattice `End` recording, report persistence, and composition-path integration. Existing `LatticeStep` and `LatticeResult` interfaces were reused unchanged.