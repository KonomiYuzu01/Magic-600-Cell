---
id: owner-decisions-2026-09-30
type: decision
status: verified
visibility: public
summary: Owner decisions of 30 September 2026 on review rounds, the wider Astra role, the fast tier and pair implementation with Codex.
related: [owner-decisions-2026-09-29, development-loop]
supersedes: []
claims:
  - {id: review-rounds, evidence_kind: decision, checked_at: 2026-09-30}
  - {id: astra-role, evidence_kind: decision, checked_at: 2026-09-30}
  - {id: fast-tier, evidence_kind: decision, checked_at: 2026-09-30}
  - {id: pair-implementation, evidence_kind: decision, checked_at: 2026-09-30}
---

# Owner decisions, 30 September 2026

The owner found long serial review loops inefficient and has subscription quota to spare. These decisions change the development system of [29 September](owner-decisions-2026-09-29.md); the rules themselves are in `AGENTS.md`.

- **Review rounds.** One full review of a finished candidate, then at most two scoped verification rounds that check only the fixed blocking findings and new `blocker` or `major` findings. `minor` and `nit` findings are deferred. Remaining blockers go to the owner as a decision. Spare quota goes to parallel review shards.
- **Astra role.** Codex Astra keeps the three final gate rulings and also runs the plan checks and full reviews of critical-path, behaviour, contract and design or ADR changes, escalation diagnoses, joint attacks and milestone audits. Sol handles the rest.
- **Fast tier.** `fast` is used for scoped verification rounds, plan re-checks, mechanical checks and non-critical reviews. Critical full reviews, joint attacks, escalations and gate rulings use the standard tier.
- **Pair implementation.** Claude and Codex write code together in separate packet-owned worktrees; Claude integrates and commits, and no one reviews their own code. The mode stays disabled until the wrapper's implementation mode passes its worktree ownership and isolation tests, which is the next development-system task.
- The owner authorized updating `AGENTS.md`, `CLAUDE.md` and the guides to match without a further question.
