---
id: owner-decisions-2026-10-01
type: decision
status: verified
visibility: public
summary: Owner decisions of 1 October 2026: the 0.4 line ends without a 0.4.1 release, step 1 closes at its acceptance, no 0.4 baseline, formal measurement or 0.4 performance fixes, and the B4-12 harness contract, fixture builder and turn probe are kept for the stage 2.4 renderer gate.
related: [owner-decisions-2026-09-29, owner-decisions-2026-09-30, b4-12-measurement-decisions, renderer-candidates, build-identity-v2, no-0-4-1-release-open-items]
supersedes: []
claims:
  - {id: no-0-4-1-release, evidence_kind: decision, checked_at: 2026-10-01}
  - {id: step-1-closes, evidence_kind: decision, checked_at: 2026-10-01}
  - {id: no-0-4-measurement, evidence_kind: decision, checked_at: 2026-10-01}
  - {id: harness-kept, evidence_kind: source, path: docs/progress/0.4.1/b4-12-harness.md, checked_at: 2026-10-01}
---

# Owner decisions, 1 October 2026

These decisions change the 0.4.1 plan of [29 September](owner-decisions-2026-09-29.md).

- **No 0.4.1 release.** The 0.4 line ends with 0.4, released on 19 September 2026. No new 0.4.1 work starts.
- **Step 1 closes at its current acceptance.** The reproducible identity and harness work ([build-identity-v2](../concepts/build-identity-v2.md)) runs its listed checks and is merged under `AGENTS.md` "Merging". Its acceptance is not widened.
- **No 0.4 measurement and no 0.4 performance fixes.** There is no B4-12 baseline, no formal 3 x 100 measurement and no performance fix for the 0.4 host. The B4-12 work stops at a clean point: the native harness, the runner and the planned engine fixes are not built.
- **Harness kept for stage 2.4.** The [harness contract](../../progress/0.4.1/b4-12-harness.md), the fixture builder (`tools/perf/b412_fixture.py`) and the headless turn probe (`tools/perf/turn_probe.py`) stay in the repository, with their tests, because the stage 2.4 renderer gate ([renderer-candidates](renderer-candidates.md)) will reuse them. The summary tool (`tools/perf/b412_summary.py`) stays with them.

Earlier decisions that assumed a 0.4.1 release, such as the 0.4.1 migration exporter and the B4-12 closeout, are listed in [no-0-4-1-release-open-items](../questions/no-0-4-1-release-open-items.md) for an owner decision. This page does not change them.
