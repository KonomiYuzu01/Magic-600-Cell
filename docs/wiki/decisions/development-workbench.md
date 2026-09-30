---
id: development-workbench
type: decision
status: verified
visibility: public
summary: Owner decision of 30 September 2026 defining the 0.4.1 local development workbench as a read-only project monitor.
related: [owner-decisions-2026-09-29, development-loop]
supersedes: []
claims:
  - {id: workbench-scope, evidence_kind: decision, checked_at: 2026-09-30}
---

# 0.4.1 development workbench

On 30 September 2026 the owner defined the "dedicated local development workbench" in the 0.4.1 scope ([owner decisions of 29 September](owner-decisions-2026-09-29.md)). This page records that decision; the implementation plan still needs a Codex plan check.

## What it is

A local desktop application that lets the owner watch the whole project at any time. It is a development tool, not part of the public application package.

## Required functions

- **All sessions.** List every local Claude Code session of this project, its subagents, and every Codex call made through the review wrapper, with status (running, waiting for the owner, finished, failed). Cloud sessions appear as links.
- **Live view.** Open any listed session or Codex call and follow its activity as it happens. Interactive control of a Claude session stays in Claude Code (desktop app or Remote Control); the workbench links to it and does not replace it.
- **Work boards.** Progress (0.4.1 steps 1 to 7, then stage 2.0 to 2.5, with the acceptance check and blocker of the current step), reviews (open findings, dispositions, Stop gate state), budget (subscription usage and paid API charges kept apart, the cumulative USD 100/150/300 ceilings, unknown cost shown as unknown), tools (`bootstrap.py doctor`, Codex login), decisions and wiki (recent log, open contradictions), Git and pull requests.
- **Progress always visible.** A compact always-on-top progress view and a matching Claude Code status line.
- **Performance boards.** Added during 0.4.1 steps 2 and 6: baseline and formal 3 x 100 measurements, PresentMon frame times and B4-12 closeout state.

## Boundaries

- Read-only. The workbench never installs, approves, commits, pushes, edits project files or calls models. Every action stays in the terminal or in a session.
- Private data (session transcripts, ledgers, review outputs, machine diagnostics) is shown only on the owner's machine. It is never published, uploaded, committed or exposed on a network listener.
- Progress state that the boards read from the repository is sanitized and public; everything else is read in place from private locations.
- Using a toolkit for the workbench is not evidence for the stage 2.4 renderer selection.

## Phases

1. Sessions board with live view, progress board, always-on-top progress view, status line.
2. Reviews, budget, tools, decisions and Git boards; live event stream from the review wrapper (a critical-path change).
3. Performance boards during 0.4.1 steps 2 and 6.
