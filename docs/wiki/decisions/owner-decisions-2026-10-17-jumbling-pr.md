---
id: owner-decisions-2026-10-17-jumbling-pr
type: decision
status: verified
visibility: public
summary: Owner decision of 17 October 2026 - the jumbling work collects in one draft pull request, #112, instead of a new pull request per task; it merges only after the A and B theory is complete, the tutorial is written and accepted, the Archon formal verification passes, and the handoff and observation logs for Taste Lab, Look Lab, the renderer line and the engineering line are done.
related: [owner-decisions-2026-10-17-repair-pairs, owner-decisions-2026-10-17-jumbling-tutorial, owner-decisions-2026-10-17-jumbling-m2]
supersedes: []
claims:
  - {id: one-draft-pr, evidence_kind: decision, checked_at: 2026-10-17}
  - {id: merge-conditions, evidence_kind: decision, checked_at: 2026-10-17}
---

# Owner decision, 17 October 2026: one draft pull request for the jumbling work

The owner gave this decision in a chat message in Chinese; this page records it in English.

## Decision

1. **No new pull request per task.** The integrator stops opening a pull request for each finished piece of jumbling
   work. The work collects on the branch of pull request #112, which stays a draft.
2. **#112 merges only when all of these are done:**
   - the theory of options A and B ([record](owner-decisions-2026-10-17-jumbling-tutorial.md)) is complete;
   - the tutorial is written and has passed the owner's acceptance;
   - the formal verification by the owner's Lean agent, Archon ([record](owner-decisions-2026-10-17-jumbling-m2.md)),
     has passed;
   - the handoff logs and observation logs for Taste Lab, Look Lab, the renderer line, the engineering line and the
     other lines the owner names are written.

## Effect

- Pull request #113 (the repair list at every repair step) was closed, and its commit merged into the branch of #112.
- The merge rule of `AGENTS.md` ("Merging") does not apply to #112: the integrator keeps its checks green and its
  branch mergeable, and does not mark it ready or merge it before the conditions above hold.
- Until #112 merges, its pages, including the [repair-list decision](owner-decisions-2026-10-17-repair-pairs.md) and
  the briefing's jumbling line, are on that branch only and not on `main`.
