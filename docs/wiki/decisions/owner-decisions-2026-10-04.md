---
id: owner-decisions-2026-10-04
type: decision
status: verified
visibility: public
summary: Owner decision of 4 October 2026 - every Codex plan check, review and scoped verification round runs on the fast tier, critical paths included; only gate rulings, joint attacks and escalations use the standard tier.
related: [owner-decisions-2026-10-02-scope]
supersedes: []
claims:
  - {id: fast-tier-reviews, evidence_kind: decision, checked_at: 2026-10-04}
---

# Owner decision, 4 October 2026: fast tier for every review

The owner found that verification still takes too much time, mainly in the development process (waiting for reviews and CI) and less in the owner's own Windows checks. Of three proposed options the owner chose option A, in a chat message in Chinese, and authorized the integrator to edit the four rule files. This page records the decision in English.

## Decision

- Every Codex plan check, review and scoped verification round uses `--speed fast`, critical-path changes included. Mechanical checks keep the fast tier.
- The standard tier is used only for the three gate rulings (stage 2.4 day-7 go/no-go, migration format freeze, 1.0 architecture freeze), joint attacks and escalations.
- Nothing else changes: the model assignment (Sol or Astra), the effort, the risk tiers, the number of review rounds and the two concurrent shards for critical-path full reviews stay as they were.

## Reason

The private call ledger showed these median durations: Astra fast review about 2 minutes, Astra standard about 9, Sol fast about 8, Sol standard about 15. Critical-path full reviews were the slowest step a candidate waited on.

## Where it is applied

- `AGENTS.md` "Model calls", speed tier.
- `docs/development-guide/AGENT_BRIEFING.md` section 6.
- `.agents/skills/codex-dialogue/SKILL.md`, with `.claude/skills/` regenerated and `tools/skills.lock.json` updated.
- `docs/development-guide/HUMAN_GUIDE.md`.

The review wrapper is unchanged; its default stays the standard tier, so each call passes `--speed fast` explicitly. `tools/skills.lock.json` is an approval input, so the owner runs `python tools/toolchain/bootstrap.py approve` on Windows after the merge.

Options B (one shard for critical-path full reviews) and C (no verification round for non-critical code) were not chosen.
