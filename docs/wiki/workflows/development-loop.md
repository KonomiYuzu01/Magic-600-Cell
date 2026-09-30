---
id: development-loop
type: workflow
status: verified
visibility: public
summary: How a task moves through plan check, implementation, verification, Codex review and recording.
related: [owner-decisions-2026-09-29, owner-decisions-2026-09-30]
supersedes: []
claims:
  - {id: wrapper, evidence_kind: source, path: tools/agents/codex_review.py, checked_at: 2026-09-30}
  - {id: pair-implementation, evidence_kind: fixture, path: tests/test_codex_implement.py, checked_at: 2026-09-30}
  - {id: stop-gate, evidence_kind: source, path: .claude/hooks/stop_gate.py, checked_at: 2026-09-29}
---

# Development loop

1. **Orient.** Read the session summary, [the index](../index.md) and the recent [log](../log.md). Restate the goal and the acceptance check.
2. **Plan check.** For a non-trivial task, write a seven-part packet (`templates/problem-packet.md`) and run `python tools/agents/codex_review.py --kind plan --packet <file>`.
3. **Implement** in small steps and run the checks that `AGENTS.md` lists for the changed area. A separable part can go to Codex as a packet with an `implement-contract` block: `python tools/agents/codex_review.py --kind implement --packet <file>` runs it in its own sandboxed worktree; Claude reviews and applies `changes.patch`, then removes the worktree with `--cleanup <call-id>`.
4. **Review.** Run `python tools/agents/codex_review.py --kind review --packet <file>` on the current candidate. Answer every finding with `adopt`, `reject_with_evidence` or `needs_verification` in `work/reviews/<call-id>/dispositions.json`.
5. **Record.** Update the affected wiki pages and append a log entry.

The Stop hook (`.claude/hooks/stop_gate.py`) blocks a stop at most twice per session while critical paths have changed without a valid review of the current candidate. After that it allows an honest `inconclusive` stop; critical changes still wait for a valid review or an explicit owner exception.

Escalation: three effective iterations without success bring in Codex for an independent diagnosis; two more failures after its instructions bring in the Fable solver; the hardest problems get a joint attack decided by experiment.
