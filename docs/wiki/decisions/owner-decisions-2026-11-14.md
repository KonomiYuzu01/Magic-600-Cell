---
id: owner-decisions-2026-11-14
type: decision
status: verified
visibility: public
summary: Owner decision of 14 November 2026 - the ChatGPT subscription quota is down to one twentieth, so Codex review is kept to its minimum in all work, and the owner authorized the integrator to change the project framework for it directly, without a pull request or approval steps.
related: [owner-decisions-2026-09-30, owner-decisions-2026-10-02, owner-decisions-2026-11-03]
supersedes: []
claims:
  - {id: codex-minimum, evidence_kind: decision, checked_at: 2026-11-14}
  - {id: framework-change-authorized, evidence_kind: decision, checked_at: 2026-11-14}
---

# Owner decision, 14 November 2026: Codex use at its minimum

The owner gave this decision in one chat message in Chinese on 14 November 2026. This page records it in English.

## Decision

1. **Codex at its minimum.** The owner's ChatGPT subscription quota is now one twentieth of what it was. From now on,
   whatever the work, Codex review is kept to the lowest level.
2. **Framework change authorized.** For this, the owner specially authorized the integrator to change the project's
   whole framework directly, without merging a pull request and without the approval steps.

## Integrator readings (the owner may change them)

- "Framework" is read as the working framework: the rules in `AGENTS.md`, `CLAUDE.md`, the briefing and the human
  guide, and the hooks and settings that enforce them. The product architecture is not touched.
- "The lowest level" is read as no Codex call by default, with two exceptions. The gate rulings stay, because the
  owner set those gates and no gate is relaxed without the owner; each is one call with at most one scoped re-check,
  then the owner decides. A call the owner asks for also runs.
- Independent review stays where it matters most: a code change on a critical path gets one read by a fresh Claude
  subagent on a smaller model before commit. That uses the Claude subscription, not ChatGPT.
- The authorization covers this change and the fixes it needs. Later framework changes follow the normal rules again.

## What changed

- `AGENTS.md` opens its team protocol with "Codex minimum", which overrides every other Codex rule until the owner lifts
  it. The earlier review, risk-tier, pair-implementation and escalation rules stay, marked as suspended or redirected,
  so lifting the minimum restores them.
- `CLAUDE.md`, the briefing and the human guide follow: the working loop has no Codex plan check or review,
  escalations go to the Fable solver and a fresh Claude subagent, and Codex runs only through the exceptions above.
- The Stop hook `.claude/hooks/stop_gate.py` is unregistered in `.claude/settings.json`, because it asks for a Codex
  review of every critical-path change. The script and its tests stay.
- The codex-dialogue skill is unchanged: changing it would change `tools/skills.lock.json`, an approval input, and the
  owner asked for no approval steps. `CLAUDE.md` now limits its use to the calls the minimum allows.
- The change went to `main` directly, without a pull request. The weekly process budget does not hold it back, since
  the owner ordered it.
- When the message arrived, a Sol review of experiment code had been running for a few minutes. It was stopped; its
  quota use is unknown.

## Codex calls still planned

The architecture-freeze and migration-format-freeze gate rulings on the language's freeze path. Replacing them too would
change gates the owner set, so that stays the owner's call.
