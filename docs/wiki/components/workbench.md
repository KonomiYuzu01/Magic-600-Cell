---
id: workbench
type: component
status: draft
visibility: public
summary: The 0.4.1 development workbench, phase 1: sessions board with briefs, live view with tool switching, owner notes, attention flags, registered runs, owner-click launches, progress board, compact view and status line.
related: [development-workbench, development-loop]
supersedes: []
claims:
  - {id: scope, evidence_kind: decision, path: docs/wiki/decisions/development-workbench.md, checked_at: 2026-09-30}
  - {id: readers, evidence_kind: source, path: tools/workbench/sources.py, checked_at: 2026-09-30}
  - {id: reporting-hook, evidence_kind: source, path: .claude/hooks/report_event.py, checked_at: 2026-09-30}
  - {id: registration, evidence_kind: source, path: .claude/settings.json, checked_at: 2026-09-30}
  - {id: headless-tests, evidence_kind: fixture, path: tests/test_workbench.py, checked_at: 2026-09-30}
  - {id: watch-rules, evidence_kind: source, path: tools/workbench/watch.py, checked_at: 2026-09-30}
  - {id: run-registry, evidence_kind: source, path: tools/workbench/runs.json, checked_at: 2026-09-30}
  - {id: runner, evidence_kind: source, path: tools/workbench/runner.py, checked_at: 2026-09-30}
  - {id: runner-tests, evidence_kind: fixture, path: tests/test_workbench_runs.py, checked_at: 2026-09-30}
  - {id: watch-tests, evidence_kind: fixture, path: tests/test_workbench_watch.py, checked_at: 2026-09-30}
  - {id: progress-state, evidence_kind: source, path: docs/progress/status.json, checked_at: 2026-09-30}
---

# Development workbench

A local Qt Quick desktop app (PySide6) that shows the owner what every Claude Code session and Codex call of this repository is doing, flags what needs attention, lets the owner send notes to a running session, and starts registered runs, Codex calls and new sessions on the owner's click. Scope and boundaries: [development-workbench](../decisions/development-workbench.md). It is a development tool, not part of the public package, and not evidence for the stage 2.4 renderer selection.

## Launch

```text
tools\.venv\workbench\Scripts\python.exe tools\workbench\app.py            (full window)
tools\.venv\workbench\Scripts\python.exe tools\workbench\app.py --compact  (always-on-top progress only)
```

The environment comes from the `workbench` toolchain profile (`python tools/toolchain/bootstrap.py install pyside6`).

## What it shows

The window opens on three owner tabs: For you, Gallery and Progress. Everything else sits in the Machine details tab, collapsed by default.

- **For you.** Cards, newest first, with a count: questions, decisions and image picks a main session asked with `tools/workbench/ask.py`, permission prompts (with an open-session button, never an approve button), failures that need a decision after their session stopped or stalled, and briefs whose `waiting_for` names the owner. An answer goes back as an owner note `answer to <ref>: <option or text>`; a card counts as answered when Claude's reply names that note, and then moves to a collapsed Answered list. A new card raises one desktop notification.
- **Gallery.** Images and other visual outputs, newest first, filtered by type and session, from the files a session referenced, run outputs, ask attachments, `work/gallery/`, `work/loop-memory/ui-vision/` and `docs/**/figures/`. Raster images and SVG show in the app; PDFs and videos open externally; Mermaid, draw.io, Typst and Marp sources show their rendered sibling. Comment, star and "wrong direction" go to the producing session as notes.
- **Machine details.** Sessions, Codex calls, Runs, Launch, budget and toolchain as before. Its badge counts only failures and stalls.

Machine details:

- **Sessions.** Every local Claude Code session whose working directory is inside the main checkout or a registered worktree, with its subagents and linked Codex calls. Status is running, waiting for the owner, finished, failed or unknown.
- **Briefs.** Goal, current step, what Claude is doing and why, and what it waits for, written by Claude with `tools/workbench/brief.py` at each plan step and milestone. Without a brief, the latest task list in the transcript is shown and labelled.
- **Live view.** The selected transcript and its event file, followed as they grow. Tool calls, command output and test runs appear inline; full text opens in the app.
- **Codex calls.** Status, packet, findings and their answers. No live output: the review wrapper writes its result only when the call returns (its event stream is phase 2).
- **Progress.** `docs/progress/status.json` (schema `schemas/progress-status.schema.json`): 0.4.1 steps 1 to 7 and stage 2.0 to 2.5. Acceptance text appears only where a repository document records it. Under schema 2 each step has a checklist; a step's percentage is its done weight over its total weight, counting only items with evidence (a commit, a pull request of this repository or a repository file), and a track's is the weighted mean of its steps. Sessions tick items with `tools/workbench/progress.py done`; nothing is estimated. A schema-1 file shows "no checklist".
- **Runs.** The registered runs with their exact steps, a checkout chooser, Run and Stop, and the latest run records with status, exit codes and duration. A selected run's log is followed live.
- **Launch.** A checkout and packet chooser, "New Claude session" and "Start Codex call". See [Runs and launches](#runs-and-launches).
- **Compact view and status line.** The current step and its percentage, the For you count and the new gallery items, for example `Step 0.4.1-1 42% · for you 2 · gallery +3`. The status line reads the counts from the snapshot the running workbench writes (`home.json`) and says "workbench closed" without a fresh one. Cumulative paid API spend over the complete ledgers (billed, estimated and reserved kept apart, unknown shown as unknown, and "incomplete" instead of a total when a ledger is too large to read) is appended only when it is not zero or unknown.

## How status is decided

Each session's whole history (its transcript, its subagent transcripts and the hook events) is read incrementally from the start and reduced in global time order, kept per agent. While a source is still catching up, newer signals from the others wait, so an older record read later never overtakes a newer one. The newest signal decides: activity means running, a turn end means waiting for the owner, a failed turn means failed, and a session end means finished. A resumed or retried session becomes running again. Owner waits (a permission prompt, a question, including a subagent's) stay open until their own resolution arrives, however much later: a child's activity never clears a parent's prompt. Age is shown as staleness ("idle 7 h"), never as completion. Every read has a byte budget; malformed lines are skipped and counted, and when the newest record of a session or subagent is too large to read the session is listed as unknown and never resumed from the workbench.

## What is flagged

Flags come from `tools/workbench/watch.py`, a pure function over the session, call and run boards, and feed the For you cards and the Machine details badge. Anything whose newest time is more than 24 hours old is history and is not flagged, except an open owner wait and a Codex launch that is still running. Runs are judged on every record written in the last 24 hours (a running record is rewritten at each heartbeat), not only the 50 the Runs board lists, and a record whose fields have the wrong type is skipped and counted.

- **Waiting for you** (first): a permission prompt, a question or an input request, including a subagent's, however old, until its own resolution or the session's end. A session that ended its turn and waits for the next prompt is normal and not flagged.
- **Failures**: a failed turn (kept after the session ends, cleared when the session runs again); a Codex call that was invalid, refused, timed out, stale or whose acceptance check failed; and a registered run or Codex launch that failed, timed out or was interrupted. A failed launch and the call it names give one flag, not two.
- **Stalls and overdue calls** (last): a running session with no activity from any of its agents for 15 minutes. A session waiting in the foreground on its own review-wrapper call is exempt, but only for a plain `python tools/agents/codex_review.py ...` command of a live agent, and only up to the wrapper's expected maximum (2 h 10 min). A Codex launch still running past its expected duration (1 h 30 min for plan and review, 2 h for implement) is flagged as overdue, never ended.

## Runs and launches

- **Registry.** `tools/workbench/runs.json` (schema `schemas/workbench-runs.schema.json`), always read from the main checkout, lists named runs: fixed steps, a working directory, declared inputs and outputs, a timeout and a Windows-only flag. A step starts with `python` or `git`, taken as the first native executable on PATH (the current directory is never searched, and Windows App Execution Aliases are skipped because Windows starts them outside the runner's job), or a venv interpreter under `tools/.venv/`; Python runs a repository script with no options in front, and Git only `status`, `diff`, `log` or `show`. Anything else makes the registry invalid, and an invalid registry runs nothing. The seed holds the `AGENTS.md` check suites and `bootstrap.py doctor`.
- **Runner.** Run starts `tools/workbench/runner.py`, detached, which reloads the registry and refuses if the entry changed after it was shown. It runs the steps with no shell and no extra arguments, writes the log and a record under `work/loop-memory/workbench/runs/`, and enforces the run's timeout. On Windows every step and its descendants live in a kill-on-close job, so Stop, a timeout or a crashed runner ends the whole process tree. Each step sees the directories of the vetted native `python` and `git` first on PATH, so a descendant that starts bare `python` or `git` (cmd.exe, a Git hook, the wrapper's acceptance check) stays in the job; a descendant that starts another App Execution Alias by name would still escape it. A per-run lock shows whether the runner is alive, and a per-entry lock keeps one entry from running twice in the same checkout. Closing the app never stops a run.
- **Codex calls.** "Start Codex call" runs `python tools/agents/codex_review.py` through the same runner, with the kind, packet, model, effort and speed chosen from the wrapper's allowed values; `--gate` and `--timeout` are never offered, so gate rulings stay in the terminal. The runner never ends a wrapper call on time, because the wrapper bounds each of its own steps; Stop still ends it. The record keeps the exit code and the call id.
- **New sessions.** "New Claude session" starts `claude` in the chosen checkout with one fixed first prompt naming the packet (`Work on the problem packet work/reviews/packets/<name>. Read it first, then follow CLAUDE.md.`), optionally with Remote Control. Packet names are limited to letters, digits, `.`, `_` and `-`, because the `claude` shim passes the prompt through cmd.exe. For sessions and Codex calls alike, a packet must resolve inside the chosen checkout's `work/reviews/packets`; a junction or link at any level that leads elsewhere is refused.
- **Checkout choice.** The Runs and Launch choosers keep the owner's choice by path when worktrees are added or removed, and the packet list follows it; the packet choice is kept by name when the list reloads (the newest packet is preselected only until the owner chooses). If the chosen checkout or packet disappears, the chooser says so and nothing can start until the owner chooses again; it never falls back to another checkout.

## Owner notes

The note box appends to a private per-session inbox. The reporting hook claims pending notes under a lock and hands them to the main thread (never a subagent) as additional context at its next tool step, each note at most once and each batch under 9,000 characters. The workbench marks a note received when its id appears in the transcript and answered when Claude's reply names it; otherwise it shows "sent, not confirmed" so the owner can send it again. A note cannot grant authorizations reserved for the terminal.

## Opening tools

Tools open in their own applications. Private files (transcripts, reviews, ledgers, diagram and notebook sources) open as plain text in Notepad; inert private images and videos open in their default application and private PDFs in the browser. Private HTML and SVG never open outside the app. Project documents open in their default application; HTML and SVG outputs open in the browser, only from project folders. Commands run in their own console: git diff, `bootstrap.py doctor`, marimo notebooks, and `claude --resume` only for a session whose client has ended it; otherwise the workbench brings Claude Code forward and shows the resume command.

## Private data

Everything the workbench reads stays where it is. Its own private data lives under `work/loop-memory/workbench/` of the main checkout (events, briefs, inbox, asks, run records, logs, locks and stop requests), which Git ignores. The app's own writes are owner notes, launch records, logs, locks and stop requests, and `home.json`, a snapshot of its For you and gallery counts for the status line; asks are written by `ask.py` in a session. Nothing is uploaded, and the app opens no network listener.

## Rollout

- Stage A: the app, the brief CLI, the status line script, the progress file and the reporting hook, which landed unregistered.
- Stage B, after stage A reached the main checkout (a session in a worktree runs hook scripts from the main checkout): the reporting hook is registered in `.claude/settings.json` for its nine events, in exec form with a 5 s timeout, and so is the status line. Each registration runs the script through a short `python -I -c` guard that exits 0 without output when the script is missing, so a session whose `${CLAUDE_PROJECT_DIR}` names an older checkout is never affected. The status line is a shell command anchored to `${CLAUDE_PROJECT_DIR}`, which Claude Code exports to hooks and the status line, so it works from subdirectories and worktrees; a checkout without the script only leaves the status line blank.
- Stage C: attention flags, the run registry and runner, and owner-click launches of runs, Codex calls and new sessions. The runner and the flag rules were written by Codex in two implement calls and integrated after Claude's review.

## Live checks

On 2026-09-30, on the owner's Windows machine with Claude Code 2.1.285, throwaway headless sessions ran with exactly these registrations:

- PostToolUse, PostToolUseFailure (a failed Read), PermissionRequest, SubagentStart, SubagentStop, Stop and SessionEnd (reason `other`) were recorded. The permission request was still refused: the hook returns no decision.
- A subagent's PostToolUse carried `agent_id` and `agent_type`, and the subagent received no note.
- Two pending notes were delivered once, in one batch, to the main thread only. Both ids appeared in the parent transcript and the final reply answered both; the workbench showed them as answered and the sessions as finished.
- Not triggered: Notification and StopFailure, which need an interactive prompt or an API failure. The synthetic tests cover their handling.
- The status line command, run the way Claude Code runs it (Git Bash, `CLAUDE_PROJECT_DIR` exported), printed the expected line from the repository root, a subdirectory, a worktree and a path with spaces.
- In an interactive Claude Code session started in a worktree, the status line rendered the expected progress line under the prompt. A session started in a subdirectory first shows Claude Code's consent prompt for the external `CLAUDE.md` imports; it was not accepted, so rendering there was not observed.
- The guarded registrations were checked again live with the same results, and the configured command exits 0 without output against a checkout that lacks the script (tested).

Stage C, on 2026-09-30 on the owner's Windows machine, with the real app and runner on a throwaway checkout and registry:

- A two-step run whose second step exits 3 showed its log lines while it ran, ended as failed with exit codes `[0, 3]`, and raised one "run failed" flag, also counted in the compact view.
- Stop on a run whose step had started a long-running child ended the run as cancelled, and the child was gone afterwards.
- A run started by a process that exited at once still finished and passed.
- The QML loaded without warnings. New Claude sessions and Codex calls were not launched live: the headless tests cover their exact command lines.

Deferred: cloud sessions as links; reviews, budget, tools, decisions and Git boards; performance boards.
