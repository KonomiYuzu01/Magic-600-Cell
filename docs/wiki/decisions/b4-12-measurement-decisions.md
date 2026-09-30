---
id: b4-12-measurement-decisions
type: decision
status: draft
visibility: public
summary: 0.4.1 step 2 decisions for the B4-12 baseline: sample plan, measurement conditions, tools, performance board, and the harness method choices.
related: [owner-decisions-2026-09-29, build-identity-v2]
supersedes: []
claims:
  - {id: d5-sample-plan, evidence_kind: decision, checked_at: 2026-09-30}
  - {id: d6-conditions, evidence_kind: decision, checked_at: 2026-09-30}
  - {id: d7-tools, evidence_kind: decision, checked_at: 2026-09-30}
  - {id: d8-board, evidence_kind: decision, checked_at: 2026-09-30}
  - {id: baseline-allowance, evidence_kind: decision, checked_at: 2026-09-30}
  - {id: harness-contract, evidence_kind: source, path: docs/progress/0.4.1/b4-12-harness.md, checked_at: 2026-09-30}
---

# B4-12 measurement decisions, 30 September 2026

These decisions govern the 0.4.1 step 2 baseline and the step 6 formal measurement of the B4-12 targets (the [measurement plan](../../progress/0.4.1/measurement-plan.md)). The harness that implements them is specified in the [harness contract](../../progress/0.4.1/b4-12-harness.md).

## Owner decisions

- **D5: sample plan.** One smoke run, then three full series of 100 measured samples per metric on the unmodified build: the same shape as step 6, which also shows the spread between runs. The owner's B4-12 instructions allow a baseline of one run of 100 samples per metric; a baseline that uses this allowance says so next to every number, and it is never presented as a formal 3 x 100 result.
- **D6: measurement conditions.** The owner's normal everyday settings, recorded exactly: power source, Windows power mode, vendor performance mode, driver V-Sync and overlays. Frame generation is off, as the renderer gate requires. A run under other settings is a separately labelled condition, never a replacement.
- **D7: tools.** The owner pins PresentMon and approves the toolchain revision. PerfView and administrator-level ETW are not used in step 2; step 4 revisits them only if py-spy, viztracer and PresentMon leave a hotspot unclear.
- **D8: performance board.** The development workbench shows a read-only board built from the summary file: the targets, the baseline per run and metric, the B4-12 status, and empty step 6 slots.

## Method choices (integrator, within the measurement plan)

These follow from the measurement plan and the 0.3 method. They are recorded here so that the baseline and step 6 use them unchanged.
- M2 measures bank navigation and Local-centre navigation. The 0.3 structure operations are not reachable in the 0.4 product.
- Timed input is an owned-window message, posted to the window's queue, so it passes the production message filter.
- During the M1 and M2 series, automatic rendering is paused, as in 0.3, so timer delay is excluded. The M1 end is the first correct full frame after adoption.
- M3 rotates the camera by a fixed scripted step per production frame, with adaptive motion off. Every frame must draw all 259,800 stickers.
- M1 turns come in inverse pairs from a fixture that is not Home. In 0.4 a commit that re-solves an orbit protects it, and a later turn that moves it is rejected, so after every pair the harness releases the new protection with an untimed engine command, as the 0.4 latency harness does. The headless turn probe (`tools/perf/turn_probe.py`) needed the same release after every pair.
- A series is never stopped early for being slow. A run that fails any correctness or state-hash check is invalid, is kept, and is repeated as a whole.
