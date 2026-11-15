---
id: owner-decisions-2026-11-15-fable
type: decision
status: verified
visibility: public
summary: Owner decision of 15 November 2026 - Fable 5.2 also runs three proactive read-only tasks in LP2 (attack tests for the M1 checker, crash-consistency tests for the M6 session store and the differential test design for the M4 kernel), one at a time at effort high; recorded in AGENTS.md with a pin refresh of the frozen format packet (FP.1).
related: [owner-decisions-2026-11-14, owner-decisions-2026-11-15-freeze]
supersedes: []
claims:
  - {id: fable-proactive-tasks, evidence_kind: decision, checked_at: 2026-11-15}
---

# Owner decision, 15 November 2026: Fable 5.2 for three proactive tasks

The owner gave this decision in chat in Chinese on 15 November 2026. This page records it in English.

## Decision

The owner asked which work in the coming implementation suits Fable 5.2. The integrator proposed three read-only tasks
and asked whether to use Fable for them; the owner chose to use it as proposed.

1. **M1 checker attack tests.** Fable looks for programs that slip past the exactness rules (LG2.19 to LG2.22) or
   capability separation (LG3.13, BND.39, BND.42) and returns them as failing test cases.
2. **M6 session-store crash consistency.** Fable checks every write point for data loss on a crash or a power loss and
   designs crash-injection tests.
3. **M4 kernel differential test design.** Fable designs the test matrix that shows the Rust kernel matches the
   reference exactly on O01 to O07 (all 1,200 generators, full collateral effects, chronological order) and checks the
   hot path for exactness traps.

Each task is read-only, runs at effort `high`, and only one runs at a time.

## Integrator readings (the owner may change them)

- Each task runs through the `fable-solver` subagent, with the model and the effort passed explicitly, on a seven-part
  packet (`templates/problem-packet.md`) that names the rules under attack and the return format.
- A task starts when its milestone has code to attack: the first once the M1 pull request is open, the others with M6
  and M4.
- Fable's findings are leads until a test confirms them. Fable changes no file; the integrator writes the tests and the
  fixes.
- Fable does no routine coding, CI, documentation, wiki, progress or pull-request work. The read of a critical-path code
  change before commit stays with a fresh Claude subagent on a smaller model (`AGENTS.md`, "Codex minimum"), and Claude
  Opus stays the main developer.
- Already allowed and kept in reserve: if an LP3 acceptance run of LX-1, LX-2 or LX-4 fails, a joint attack by Fable
  and a fresh Claude subagent diagnoses it. The failure still reopens LQ9, as the 14 November decisions set.
- Fable 5.2's list price per token is about twice Opus 5.5's, and its turns can be long, so it is expected to draw on
  the Claude subscription faster than Opus; this is an inference, not a measurement. Other proactive use goes to the
  owner first.

## What changed

- `AGENTS.md`, team protocol, Roles: the Fable line names the three tasks and their limits.
- `.claude/agents/fable-solver.md`: its description admits the three tasks, and a proactive task's packet sets the
  return format.
- `docs/development-guide/HUMAN_GUIDE.md`: the Fable line says the same in plain words.
- The briefing is unchanged. It leaves rules to `AGENTS.md`, and the format packet pins it.

## Format packet pin refresh (FP.1)

`AGENTS.md` is an input of the frozen migration format packet, so this governance-only change needs a recorded pin
refresh (FP.1).

| Pin in `candidate.json` | Old | New |
| --- | --- | --- |
| `AGENTS.md` SHA-256 | `9b21d4852b0a6094cfcc6e32752100c73d145cde50771b2b81edd5a0ce9ba65e` | `9c473c758e257b58a67d175382582d3d3879aff63d03f5d0e2c281ba5f2f5800` |
| `input_commit` | `a5b24fe96015e540006f62d8cbd56a3db6d6e1d1` | the commit of this change that edits `AGENTS.md` |

- The `AGENTS.md` change is confined to the Fable bullet under Roles. The packet and the chapters cite `AGENTS.md` for
  its mechanics, persistence, evidence and authority rules and its "Ask the owner" list, and none of those changed.
- No frozen clause, vector or dependency meaning changed: the candidate files and `candidate_sha256` are unchanged, and
  `check_vectors.py` verifies every other candidate and input pin against the same bytes as before.
