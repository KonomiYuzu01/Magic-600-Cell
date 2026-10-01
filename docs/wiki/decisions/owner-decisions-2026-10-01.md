---
id: owner-decisions-2026-10-01
type: decision
status: verified
visibility: public
summary: Owner decisions of 1 October 2026 - no 0.4.1 release, and preparation plus all of stage 2 within 20 days.
related: [owner-decisions-2026-09-29, owner-decisions-2026-09-30, renderer-candidates, b4-12-measurement-decisions, no-0-4-1-release-open-items]
supersedes: []
claims:
  - {id: no-0-4-1-release, evidence_kind: decision, checked_at: 2026-10-01}
  - {id: stage-2-twenty-days, evidence_kind: decision, checked_at: 2026-10-01}
  - {id: harness-kept, evidence_kind: source, path: docs/progress/0.4.1/perf-handoff.md, checked_at: 2026-10-01}
---

# Owner decisions, 1 October 2026

The owner judged the remaining 0.4.1 steps too costly in time and tokens for what they give 1.0. These decisions replace parts of the [29 September](owner-decisions-2026-09-29.md) and [30 September](owner-decisions-2026-09-30.md) records, as listed below.

- **No 0.4.1 release.** The 0.4 line ends with the released 0.4. 0.4.1 steps 2 to 7 (baseline, exporter, screening fixes, freeze candidate, formal 3 x 100 measurement, release) are cancelled. B4-12 does not close in the 0.4 line; the stage 2.4 renderer gate is the performance acceptance that matters for 1.0.
  - Step 1 (reproducible identity and harness) is finished at its current acceptance and merged. The B4-12 harness contract, fixture builder and probe are merged for reuse by the stage 2.4 renderer gate. No further 0.4 performance work starts. The summary tool (`tools/perf/b412_summary.py`) is kept with them; the native harness, the runner and the planned engine fixes were not built ([closing handoff](../../progress/0.4.1/perf-handoff.md)).
  - Work already done stays useful: the development system and the workbench continue as the development tools for 1.0; the [screening findings](../../progress/0.4.1/screening-findings.md) become 1.0 requirements and tests; the exporter draft becomes input to the 1.0 migration design; the measurement plan becomes input to the renderer gate method. The release preparation and data-directory plans are archived.
  - Migration: the decision that a 0.4.1 exporter ships the `.c600migrate` package no longer has a release to ship in. The migration path is re-planned in stage 2.0 (for example a 1.0 importer that reads a copy of the 0.4 database). Its invariants stay: copy under the session lock, never modify the original session directory, never open a user database with `immutable=1`.
- **Stage 2 within 20 days.** Preparation and all of stage 2 (2.0 to 2.5) finish within 20 days. Day 1 is 1 October 2026 and day 20 is 20 October 2026 (calendar days). The stage 2.4 renderer window shrinks from 15 working days to 12 days inside that limit, keeping the bare Direct3D 12 probe first, the day-7 go/no-go and the [selection gate](renderer-candidates.md) unchanged. The design-track living test shrinks from one week to five days. The day-by-day plan is in `docs/progress/1.0/stage-2-experiment-protocol.md` section 5. If no renderer candidate passes the gate by day 14, stage 2 still ends on day 20 with the failure recorded, and the owner decides the next step; there is no automatic extension.

Two items that this record does not settle, the 1.0 engine selection input and the workbench performance boards, are listed in [no-0-4-1-release-open-items](../questions/no-0-4-1-release-open-items.md).
