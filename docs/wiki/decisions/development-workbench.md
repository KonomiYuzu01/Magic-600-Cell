---
id: development-workbench
type: decision
status: verified
visibility: public
summary: Owner decision of 30 September 2026 defining the 0.4.1 local development workbench as a local project monitor connected live to Claude.
related: [owner-decisions-2026-09-29, development-loop]
supersedes: []
claims:
  - {id: workbench-scope, evidence_kind: decision, checked_at: 2026-09-30}
  - {id: watch-and-launch, evidence_kind: decision, checked_at: 2026-09-30}
---

# 0.4.1 development workbench

On 30 September 2026 the owner defined the "dedicated local development workbench" in the 0.4.1 scope ([owner decisions of 29 September](owner-decisions-2026-09-29.md)). This page records that decision; the implementation plan still needs a Codex plan check.

## What it is

A local desktop application that lets the owner inspect and take part in the work at any time. It is a development tool, not part of the public application package.

## Required functions

- **All sessions.** List every local Claude Code session of this project, its subagents, and every Codex call made through the review wrapper, with status (running, waiting for the owner, finished, failed). Cloud sessions appear as links.
- **Session summaries.** Under each session, a short plain-language explanation: goal, current step, what Claude is doing now and why, and what it is waiting for.
- **Live connection to Claude.** Claude sessions report to the workbench as they work: bounded local hooks append structured events (tool use, subagent start and stop, stop state), and Claude writes the plain-language summaries at each plan step and milestone. The workbench never generates summaries with a model.
- **Live view with access to every tool.** Open any listed session or Codex call and follow its activity as it happens, and switch from there to every tool the work uses: subagents, Codex calls, terminal commands and their output, tests, diffs, marimo notebooks, diagrams and documents (Mermaid, draw.io, Typst, Marp), wiki pages and toolchain status. Tools open in their own applications.
- **Owner participation.** The owner can send a note to a running Claude session from the workbench; Claude receives it at its next tool step and answers it. Full interactive control stays in Claude Code (desktop app or Remote Control), which the workbench opens for the selected session.
- **Work boards.** Progress (0.4.1 steps 1 to 7, then stage 2.0 to 2.5, with the acceptance check and blocker of the current step), reviews (open findings, dispositions, Stop gate state), budget (subscription usage and paid API charges kept apart, the cumulative USD 100/150/300 ceilings, unknown cost shown as unknown), tools (`bootstrap.py doctor`, Codex login), decisions and wiki (recent log, open contradictions), Git and pull requests.
- **Continuous watch and launch.** The workbench watches all work continuously and flags stalled sessions, failed runs, failed Codex calls and items waiting for the owner. From it the owner can launch, at any time: a new Claude Code session with a chosen packet, a resumed session, a Codex call through the review wrapper, and a registered experiment or measurement run. A registered run is a named command in a checked-in registry with fixed arguments and declared inputs and outputs; the workbench shows its live output and result.
- **Progress always visible.** A compact always-on-top progress view and a matching Claude Code status line.
- **Performance boards.** Added during 0.4.1 steps 2 and 6: baseline and formal 3 x 100 measurements, PresentMon frame times and B4-12 closeout state.

## Boundaries

- The workbench never installs, approves, commits, pushes, edits project files or calls models by itself. Its only writes are owner notes to a private per-session inbox, the records, logs, locks and stop requests of the work it launches, and a snapshot of its inbox and gallery counts for the status line, all under `work/loop-memory/`. On the owner's click it may open a tool in its own application or launch a session, a wrapper call or a registered run; launched work runs under the normal rules of `AGENTS.md`, and project changes still happen only in sessions, runs and the terminal. It never launches a command outside the registry, passes arbitrary arguments, or launches anything on its own.
- The reporting hooks only append bounded local records. They never call models, install tools, use the network or block Claude, and a hook failure never stops a session. They are critical-path changes and need a Codex review.
- An owner note is owner input, but it cannot grant an authorization reserved for the terminal (for example `bootstrap.py approve`).
- Private data (session transcripts, ledgers, review outputs, machine diagnostics) is shown only on the owner's machine. It is never published, uploaded, committed or exposed on a network listener.
- Progress state that the boards read from the repository is sanitized and public; everything else is read in place from private locations.
- Using a toolkit for the workbench is not evidence for the stage 2.4 renderer selection.

## Phases

1. Sessions board with summaries and live view, continuous watch, launching sessions, wrapper calls and registered runs, progress board, always-on-top progress view, status line, Claude reporting hooks, owner notes, and switching to every tool.
2. Reviews, budget, tools, decisions and Git boards; live event stream from the review wrapper (a critical-path change).
3. Performance boards during 0.4.1 steps 2 and 6.
