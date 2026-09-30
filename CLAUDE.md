# CLAUDE.md

Claude Code instructions for Magic 600 Cell. Shared rules and the team protocol are in `AGENTS.md`; the current briefing is in `docs/development-guide/AGENT_BRIEFING.md`. Both are imported below. This file adds what only Claude does.

@AGENTS.md
@docs/development-guide/AGENT_BRIEFING.md

## Owner requirements and where they live

- Claude Opus is the main developer and calls Codex often: `AGENTS.md` "Team protocol" and "Calling Codex" below.
- Every formal non-software output is made with local tools: `AGENTS.md` "Tools, skills and outputs".
- Missing tools and skills are installed automatically from the reviewed allowlist: `AGENTS.md` "Tools, skills and outputs" and "Tools and skills" below.
- The agent-team dialogue mode and the API-team mode are kept: `AGENTS.md` "Team protocol". API-team runs, including runs on OpenAI models, start only after the owner confirms provider, models and budget.

## Working loop

1. Orient: read the SessionStart summary, `docs/wiki/index.md` and the recent wiki log. Restate the goal and the acceptance check.
2. Plan: for a non-trivial task, run a Codex plan check.
3. Implement in small steps, splitting separable work with Codex under `AGENTS.md` "Pair implementation". Run the checks that `AGENTS.md` lists for the changed area.
4. Review: run one Codex review of the finished candidate (parallel shards for a large one), fix the blocking findings, then run scoped verification rounds within the limits of `AGENTS.md` "Review rounds". Answer every finding; defer minor ones.
5. Record: link the wrapper-owned call record, record the finding dispositions and update the affected wiki pages. Do not duplicate usage or billing entries. Commit a critical change only after a valid review or an explicit owner exception.

Use the `codex-dialogue`, `wiki` and `toolchain` skills for these steps, and the owner's workflow skills (`investigate-first`, `lean-build`, `migration`, `safe-refactor`, `surgical-patch`, `verify-and-stop`) when a task matches them.

## Calling Codex

- Use the wrapper for every automated call. It passes the model (`gpt-6.1-sol` by default; `gpt-6-astra` when `AGENTS.md` assigns Astra, and always with `--gate`), the effort and the speed tier explicitly, uses a fixed read-only policy for plan checks and reviews, ignores the user Codex config (its sandbox mode, MCP servers and plugins; sign-in must use the default file-backed store), accepts no arbitrary CLI passthrough, and rejects a run whose reported model, effort or sandbox differs from the request or whose speed tier was dropped.
- Plan check: `python tools/agents/codex_review.py --kind plan --packet <file>`
- Review: `python tools/agents/codex_review.py --kind review --packet <file>`
- Implementation: `python tools/agents/codex_review.py --kind implement --packet <file>`. The packet carries one `implement-contract` block (allowed files, acceptance check, stop condition; see `templates/problem-packet.md`). The wrapper creates `work/worktrees/<call-id>/` on branch `codex/<call-id>` from the committed HEAD, so commit what Codex needs to see before the call. It verifies the reported model, effort, sandbox and writable roots, rejects changes outside the allowed files and Git-state or link changes, and runs the acceptance check inside the Codex sandbox. Exit 0 means valid with a passing acceptance check, 4 a valid run whose check failed.
- Integrate a valid run by reading `work/reviews/<call-id>/changes.patch` and `report.md`, reviewing the change, applying it with `git apply`, and running the checks for the changed area. `meta.json` lists critical paths the change touches; those also get a review by the other Codex model. Then run `python tools/agents/codex_review.py --cleanup <call-id>`, also after a rejected or invalid run.
- Resume stays disabled in the wrapper until isolated tests show that it keeps the model, effort, sandbox and schema. Start a fresh read-only call instead. (Observed on 2026-09-29: a `resume` without `-m` ran on a different model.)
- `/codex:review` and `/codex:adversarial-review` are owner-invoked only. The Codex plugin is optional. Direct CLI automation through the wrapper is the supported path, and the plugin's stop-time review gate stays off.

## Hooks

- SessionStart (`.claude/hooks/session_start.py`) injects a bounded local summary. It never calls models, installs tools, uses the network or reads personal data.
- PreToolUse (`.claude/hooks/install_guard.py`) lets an exact `bootstrap.py install` or `install-skill` command, run from the project root, proceed without a prompt only when the installer, every script it runs and its lockfiles match the owner-approved revision; otherwise the command asks the owner. The hook never imports repository code.
- Stop (`.claude/hooks/stop_gate.py`) is the bounded completion gate:
  - It permits at most two automatic continuations per session.
  - It never re-blocks when `stop_hook_active` is true.
  - It excludes generated review records from its candidate identity.
  - A review counts only when the result is schema-valid, covers the current source identity, and has its blocking findings resolved.
  - A timeout, an error or an exhausted continuation budget permits an honest stop as `inconclusive`, never an automatic commit.
  - The hook performs bounded local checks only. It never invokes models, installs tools, runs tests or mutates Git.
- Before relying on hooks on a machine, verify that the resolved interpreter is the approved 64-bit CPython and that the installed Claude Code version supports the exec form. Each hook checks its runtime first and reports `inconclusive` on a mismatch, without installing anything.

## Tools and skills

- Install an allowlisted tool with `python tools/toolchain/bootstrap.py install <id>`, then run its probe. The owner approves each reviewed installer and lockfile revision once, by running `python tools/toolchain/bootstrap.py approve` in a terminal; never run or simulate that step yourself.
- Install a pinned third-party skill with `python tools/toolchain/bootstrap.py install-skill <id>`. Edit project skills only in `.agents/skills/`, regenerate `.claude/skills/` with `python tools/skills/sync.py`, and check them with `python tools/skills/sync.py --check`.

## Knowledge wiki

- Follow `docs/wiki/SCHEMA.md` for ingest, query and lint. Ingest only when the owner asks. File a query answer back only when it has lasting value and evidence.

6. Merge: merge your own pull request when `AGENTS.md` "Merging" allows it; otherwise ask the owner once, briefly.