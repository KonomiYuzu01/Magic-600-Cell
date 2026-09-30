---
id: workbench
type: component
status: draft
visibility: public
summary: The 0.4.1 development workbench, phase 1: sessions board with briefs, live view with tool switching, owner notes, progress board, compact view and status line.
related: [development-workbench, development-loop]
supersedes: []
claims:
  - {id: scope, evidence_kind: decision, path: docs/wiki/decisions/development-workbench.md, checked_at: 2026-09-30}
  - {id: readers, evidence_kind: source, path: tools/workbench/sources.py, checked_at: 2026-09-30}
  - {id: reporting-hook, evidence_kind: source, path: .claude/hooks/report_event.py, checked_at: 2026-09-30}
  - {id: headless-tests, evidence_kind: fixture, path: tests/test_workbench.py, checked_at: 2026-09-30}
  - {id: progress-state, evidence_kind: source, path: docs/progress/status.json, checked_at: 2026-09-30}
---

# Development workbench

A local Qt Quick desktop app (PySide6) that shows the owner what every Claude Code session and Codex call of this repository is doing, and lets the owner send notes to a running session. Scope and boundaries: [development-workbench](../decisions/development-workbench.md). It is a development tool, not part of the public package, and not evidence for the stage 2.4 renderer selection.

## Launch

```text
tools\.venv\workbench\Scripts\python.exe tools\workbench\app.py            (full window)
tools\.venv\workbench\Scripts\python.exe tools\workbench\app.py --compact  (always-on-top progress only)
```

The environment comes from the `workbench` toolchain profile (`python tools/toolchain/bootstrap.py install pyside6`).

## What it shows

- **Sessions.** Every local Claude Code session whose working directory is inside the main checkout or a registered worktree, with its subagents and linked Codex calls. Status is running, waiting for the owner, finished, failed or unknown.
- **Briefs.** Goal, current step, what Claude is doing and why, and what it waits for, written by Claude with `tools/workbench/brief.py` at each plan step and milestone. Without a brief, the latest task list in the transcript is shown and labelled.
- **Live view.** The selected transcript and its event file, followed as they grow. Tool calls, command output and test runs appear inline; full text opens in the app.
- **Codex calls.** Status, packet, findings and their answers. No live output: the review wrapper writes its result only when the call returns (its event stream is phase 2).
- **Progress.** `docs/progress/status.json` (schema `schemas/progress-status.schema.json`): 0.4.1 steps 1 to 7 and stage 2.0 to 2.5. Acceptance text appears only where a repository document records it.
- **Compact view and status line.** Current step, open findings, Codex login, and cumulative paid API spend over the complete ledgers (billed, estimated and reserved kept apart, unknown shown as unknown, and "incomplete" instead of a total when a ledger is too large to read) against the current ceiling.

## How status is decided

Each session's whole history (its transcript, its subagent transcripts and the hook events) is read incrementally from the start and reduced in global time order, kept per agent. While a source is still catching up, newer signals from the others wait, so an older record read later never overtakes a newer one. The newest signal decides: activity means running, a turn end means waiting for the owner, a failed turn means failed, and a session end means finished. A resumed or retried session becomes running again. Owner waits (a permission prompt, a question, including a subagent's) stay open until their own resolution arrives, however much later: a child's activity never clears a parent's prompt. Age is shown as staleness ("idle 7 h"), never as completion. Every read has a byte budget; malformed lines are skipped and counted, and when the newest record of a session or subagent is too large to read the session is listed as unknown and never resumed from the workbench.

## Owner notes

The note box appends to a private per-session inbox, the workbench's only write. The reporting hook claims pending notes under a lock and hands them to the main thread (never a subagent) as additional context at its next tool step, each note at most once and each batch under 9,000 characters. The workbench marks a note received when its id appears in the transcript and answered when Claude's reply names it; otherwise it shows "sent, not confirmed" so the owner can send it again. A note cannot grant authorizations reserved for the terminal.

## Opening tools

Tools open in their own applications. Private files (transcripts, reviews, ledgers) open only as plain text in Notepad. Project documents open in their default application; HTML and SVG outputs open in the browser, only from project folders. Commands run in their own console: git diff, `bootstrap.py doctor`, marimo notebooks, and `claude --resume` only for a session whose client has ended it; otherwise the workbench brings Claude Code forward and shows the resume command.

## Private data

Everything the workbench reads stays where it is. Its own private data lives under `work/loop-memory/workbench/` of the main checkout (events, briefs, inbox), which Git ignores. Nothing is uploaded, and the app opens no network listener.

## Rollout

- Stage A: the app, the brief CLI, the status line script, the progress file and the reporting hook, which lands unregistered.
- Stage B, after stage A is merged into the main checkout: the hooks and the status line are registered in `.claude/settings.json`, because a session in a worktree runs hook scripts from the main checkout. Live checks then confirm subagent routing, note receipt and the status-line command on the installed Claude Code.

Deferred: cloud sessions as links; reviews, budget, tools, decisions and Git boards; performance boards.
