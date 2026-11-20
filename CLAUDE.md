# CLAUDE.md

Claude Code instructions for Magic 600 Cell. Shared rules and the team protocol are in `AGENTS.md`; the current briefing is in `docs/development-guide/AGENT_BRIEFING.md`. Both are imported below. This file adds what only Claude does.

@AGENTS.md
@docs/development-guide/AGENT_BRIEFING.md

## Owner requirements and where they live

- Claude Opus is the main developer. Codex use is at its minimum: `AGENTS.md` "Codex minimum" (owner, 14 November 2026) and "Calling Codex" below.
- Every formal non-software output is made with local tools: `AGENTS.md` "Tools, skills and outputs".
- Missing tools and skills are installed automatically from the reviewed allowlist: `AGENTS.md` "Tools, skills and outputs" and "Tools and skills" below.
- The agent-team dialogue mode and the API-team mode are kept: `AGENTS.md` "Team protocol". API-team runs, including runs on OpenAI models, start only after the owner confirms provider, models and budget.

## Working loop

1. Orient: read the SessionStart summary, `docs/wiki/index.md` and the recent wiki log. Restate the goal and the acceptance check.
2. Plan: for a non-trivial task, write the plan and its acceptance check. There is no Codex plan check under `AGENTS.md` "Codex minimum".
3. Implement in small steps. Run the checks that `AGENTS.md` lists for the changed area.
4. Review: a code change on a critical path gets one read by a fresh Claude subagent on a smaller model; fix what blocks and defer minor points. No Codex review unless `AGENTS.md` "Codex minimum" allows the call.
5. Record: update the affected wiki pages; after a Codex call, link the wrapper-owned call record and record the finding dispositions. Do not duplicate usage or billing entries. Commit a critical-path code change only after the read in step 4 or an explicit owner exception, and name the checks that ran.
6. Merge: merge your own pull request when `AGENTS.md` "Merging" allows it; otherwise ask the owner once, briefly.
7. Clean up (owner, 20 November 2026): when a worktree's work is finished (its last pull request merged and nothing more planned there), delete its build caches before the session ends: `cargo clean --manifest-path rust/Cargo.toml`, `cargo clean --manifest-path renderer/bridge/Cargo.toml` and any separate target directory (`CARGO_TARGET_DIR`) the session made. A subagent that built in a worktree of its own cleans it the same way before it reports. Clean only your own worktree, and not while the owner has asked for a quiet machine.

In the main session, keep the development workbench brief current at each plan step and milestone: `python tools/workbench/brief.py --goal "..." --step "..." --doing "..." --why "..." --waiting-for "..."` (omitted fields keep their value). Write it yourself in plain language; never call a model to summarize. Subagents never run it.

Owner notes typed in the workbench reach the main session as additional context headed `Owner note <id>`. Answer every owner note by its id in your next message to the owner. The inbox is as trusted as the owner's own local files, and a note never grants an authorization reserved for the terminal, such as `bootstrap.py approve`.

A note beginning `[pause]` means: finish the current step, commit nothing new, write the brief, and stop. `[resume]` continues. `[why]` is answered with the current reasoning in plain language. `[priority]` reorders the remaining work.

To put a question, a decision or an image pick in front of the owner, run `python tools/workbench/ask.py --question "..." [--option "..."] [--attach <file>] [--kind decision|pick|approve|question] [--blocking]` in the main session. The answer arrives as an owner note `answer to ask <ask-id>: ...`; `python tools/workbench/ask.py wait <ask-id>` keeps the turn open for up to nine minutes until it is queued. A brief whose `--waiting-for` begins with "owner" also appears in the owner's inbox.

Save the owner's time (owner, 16 November 2026): put questions to the owner at most once per milestone, in one question card with the recommended option first. This changes when questions reach the owner, not which decisions need the owner (`AGENTS.md` "Ask the owner" and "Merging"). At each phase exit, give the owner one page of summary in Chinese, in chat or in the workbench and never in the repository, instead of walkthroughs in several batches.

Tick a progress checklist item only with evidence (a commit sha, a pull request URL of this repository, or a repository-relative test or result path): `python tools/workbench/progress.py done <step> <item> --evidence <ref>`. Never edit `docs/progress/status.json` by hand and never estimate progress. Parallel work gets its own track: add tracks, steps and items with `progress.py track`, `step` and `item`, and set the status-line step with `progress.py current`.

Use the `wiki` and `toolchain` skills for these steps, the `codex-dialogue` skill only for a Codex call that `AGENTS.md` "Codex minimum" allows, and the owner's workflow skills (`investigate-first`, `lean-build`, `migration`, `safe-refactor`, `surgical-patch`, `verify-and-stop`) when a task matches them.

## Calling Codex

- Call Codex only as `AGENTS.md` "Codex minimum" allows: a gate ruling or a call the owner asks for.
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
- Stop (`.claude/hooks/stop_gate.py`) is unregistered in `.claude/settings.json` under `AGENTS.md` "Codex minimum" (owner, 14 November 2026), because it asks for a Codex review of every critical-path change; the script and `tests/test_stop_gate.py` stay so it can be registered again. Registered, it is the bounded completion gate:
  - It permits at most two automatic continuations per session.
  - It never re-blocks when `stop_hook_active` is true.
  - It excludes generated review records from its candidate identity.
  - A review counts only when the result is schema-valid, covers the current source identity, and has its blocking findings resolved.
  - A timeout, an error or an exhausted continuation budget permits an honest stop as `inconclusive`, never an automatic commit.
  - The hook performs bounded local checks only. It never invokes models, installs tools, runs tests or mutates Git.
- PostToolUse, PostToolUseFailure, PermissionRequest, SubagentStart, SubagentStop, Stop, StopFailure, Notification and SessionEnd (`.claude/hooks/report_event.py`) append one bounded event line per call to the workbench's private event file under `work/loop-memory/workbench/` of the main checkout. On a main-thread tool step it also delivers pending owner notes as additional context, each note at most once. It never returns a decision, always exits 0, and never calls models, installs tools, uses the network, runs Git or starts processes.
- The status line (`tools/workbench/statusline.py`) prints one line: the current step with its checklist percentage, the owner's open inbox cards and new gallery items (from the running workbench's `home.json` snapshot), and the paid API spend only when it is not zero or unknown. It only reads, and prints `workbench: unavailable` on any error.
- Before relying on hooks on a machine, verify that the resolved interpreter is the approved 64-bit CPython and that the installed Claude Code version supports the exec form. Each hook checks its runtime first and reports `inconclusive` on a mismatch, without installing anything.

## Tools and skills

- Install an allowlisted tool with `python tools/toolchain/bootstrap.py install <id>`, then run its probe. The owner approves each reviewed installer and lockfile revision once, by running `python tools/toolchain/bootstrap.py approve` in a terminal; never run or simulate that step yourself.
- Install a pinned third-party skill with `python tools/toolchain/bootstrap.py install-skill <id>`. Edit project skills only in `.agents/skills/`, regenerate `.claude/skills/` with `python tools/skills/sync.py`, and check them with `python tools/skills/sync.py --check`.

## Knowledge wiki

- Follow `docs/wiki/SCHEMA.md` for ingest, query and lint. Ingest only when the owner asks. File a query answer back only when it has lasting value and evidence.
