---
id: codex-plan-review-2026-09-29
type: dialogue
status: verified
visibility: public
summary: Three Codex review rounds on the development system and 1.0 roadmap; outcomes adopted into the owner-approved plan.
related: [owner-decisions-2026-09-29]
supersedes: []
claims:
  - {id: rounds, evidence_kind: decision, checked_at: 2026-09-29}
---

# Codex plan review, 29 September 2026

Sanitized summary. The full prompts and answers are private (`private:P-0001`, `private:P-0002`, `private:P-0003`) and support no public technical claim by themselves.

| Round | Setup | Outcome |
| --- | --- | --- |
| 1 | Claude proposed the stage plan, stage 2 design, migration follow-up, Codex integration, wiki and auto-install; Codex reviewed read-only. | A detailed review with repository citations; inventory counts re-checked by Codex. |
| 2 | Claude answered each point and raised 12 disagreements. | Codex ruled on all 12; three agreed, nine agreed with changes (mandatory bare D3D12 probe, 15-day window, exporter design, migration defaults, one public wiki, bounded stop gate, semantic non-trivial rule, private call ledger, solver as a subagent). |
| 3 | Codex reviewed the draft `AGENTS.md`, `CLAUDE.md`, settings and file list like code. | "Request changes": 15 findings (1 blocker, 11 major, 3 minor); all adopted, including two-phase install authorization and a restricted review wrapper. |

Lessons recorded as rules:
- Pass the Codex model explicitly on every call, including resume: one resumed call without `-m` ran on a different model.
- Opening SQLite with `immutable=1` misses commits that are still only in the WAL; the exporter copies the database under the session lock instead.
