---
id: no-0-4-1-release-open-items
type: question
status: draft
visibility: public
summary: Contradiction - two earlier decisions still assume the 0.4.1 work that the owner cancelled on 1 October 2026 (the 1.0 engine selection input and the workbench performance boards); each needs an owner decision.
tags: [contradiction]
related: [owner-decisions-2026-10-01, owner-decisions-2026-09-29, development-workbench]
supersedes: []
claims:
  - {id: cancellation, evidence_kind: decision, checked_at: 2026-10-01}
  - {id: briefing-engine-selection, evidence_kind: source, path: docs/development-guide/AGENT_BRIEFING.md, checked_at: 2026-10-01}
  - {id: workbench-performance-boards, evidence_kind: source, path: docs/wiki/decisions/development-workbench.md, checked_at: 2026-10-01}
---

# Open items after the cancelled 0.4.1 release

On [1 October 2026](../decisions/owner-decisions-2026-10-01.md) the owner ended the 0.4 line without a 0.4.1 release. That record settles the migration path (re-planned in stage 2.0), the B4-12 closeout (the stage 2.4 renderer gate is the performance acceptance for 1.0) and the progress file. Two earlier decisions still depend on the cancelled work. This page records them; it changes no rule and accepts no decision.

1. **1.0 engine selection input.** The [29 September decisions](../decisions/owner-decisions-2026-09-29.md) and the agent briefing say that only 0.4.1 optimization records inform 1.0 engine selection. No 0.4.1 optimization will happen; the available records are the screening findings (now 1.0 requirements) and the turn probe's attribution of a headless turn. Open: whether those records are enough, and the briefing's wording.
2. **Workbench performance boards.** The [workbench decision](../decisions/development-workbench.md) plans performance boards filled in 0.4.1 steps 2 and 6, which no longer run. Open: whether the boards move to the stage 2.4 experiments.
