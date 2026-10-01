---
id: no-0-4-1-release-open-items
type: question
status: draft
visibility: public
summary: Contradiction - earlier decisions and rule text still assume a 0.4.1 release that the owner cancelled on 1 October 2026; each item needs an owner decision.
tags: [contradiction]
related: [owner-decisions-2026-10-01, owner-decisions-2026-09-29, development-workbench]
supersedes: []
claims:
  - {id: cancellation, evidence_kind: decision, checked_at: 2026-10-01}
  - {id: briefing-still-0-4-1, evidence_kind: source, path: docs/development-guide/AGENT_BRIEFING.md, checked_at: 2026-10-01}
  - {id: progress-schema-statuses, evidence_kind: source, path: schemas/progress-status.schema.json, checked_at: 2026-10-01}
---

# Open items after the cancelled 0.4.1 release

On [1 October 2026](../decisions/owner-decisions-2026-10-01.md) the owner ended the 0.4 line without a 0.4.1 release. Several earlier decisions and rule texts still depend on that release. This page records the conflicts; it changes no rule and accepts no decision.

1. **Migration exporter.** The [29 September decisions](../decisions/owner-decisions-2026-09-29.md) say that a 0.4.1 exporter writes the versioned `.c600migrate` package that 1.0 imports, and the agent briefing lists the minimal exporter, the migration schema and its validator among the items that must not be deferred out of 0.4.1. Without a 0.4.1 release the exporter has no release vehicle. The step 3 migration work exists on an unmerged branch. Open: how 0.4 users' data reaches 1.0.
2. **B4-12 closeout.** The same decisions close B4-12 in 0.4.1 and no longer defer it to 1.0. With no 0.4 measurement, B4-12 is neither met nor waived. Open: whether B4-12 is closed as not measured, carried into the stage 2.4 renderer gate, or dropped.
3. **1.0 engine selection input.** The decisions say that only 0.4.1 optimization records inform 1.0 engine selection. No 0.4.1 optimization will happen; the available records are the step 4 screening findings and the turn probe's attribution of a headless turn. Open: whether those records are enough.
4. **Rule text and progress file.** The agent briefing still describes 0.4.1 as the next release and the current phase as 0.4.1 step 1. The public progress file lists 0.4.1 steps 1 to 7, and its schema allows only `not_started`, `in_progress`, `blocked` and `done`, so a cancelled step can only be shown as blocked with a reason. Open: the owner's wording for the briefing, and whether the schema gets a cancelled status (a critical-path change).
5. **Development workbench scope.** The [workbench decision](../decisions/development-workbench.md) plans performance boards filled in steps 2 and 6, which no longer run. Open: whether the boards move to the stage 2.4 experiments.
