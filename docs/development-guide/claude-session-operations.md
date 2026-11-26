# Claude session operations

Detail for Claude Code sessions that `CLAUDE.md` points to. This guide has the authority of `CLAUDE.md`, and changing it follows the rules for changing `CLAUDE.md` (owner, 26 November 2026: the text moved here unchanged so that the always-loaded files stay short).

## Machine cleanup

The main session also keeps the owner's SSD free (owner, 20 November 2026). It cleans this project's dormant worktrees and files on this machine once a day while sessions work, before each owner-machine sitting, and whenever the system disk has less than 100 GB free:
- A worktree is dormant when it is clean (no uncommitted or untracked change), every commit on it is on the remote, and either its last pull request is merged or closed with nothing more planned there, or neither a commit nor its session has been active there for 3 days. Remove it with `git worktree remove` (never `--force`) and delete its merged local branch; a Codex worktree under `work/worktrees/` goes through `codex_review.py --cleanup`. The main checkout is never removed.
- Before removing a worktree, check its ignored `work/` folder: records, measurement runs and other evidence that a record, a wiki claim or an open measurement needs are moved to the main checkout's `work/` first, or the worktree stays.
- Build caches go in every dormant worktree and in each worktree whose session has been idle for a day: `cargo clean` for `rust/Cargo.toml` and `renderer/bridge/Cargo.toml`, separate target directories, and Python and Node build output.
- Scratch folders and task outputs that this project's sessions left in the system temp folder go when they are older than 7 days and their session is not running.
- Never deleted: the owner's session directory and user databases, `work/loop-memory/`, review records under `work/reviews/`, evidence that is cited or not yet recorded, installed tools and their environments under `tools/`, stashes, and any commit that is not on the remote.
- Whatever these rules do not cover, or whose state is unclear, is listed once to the owner with its size instead of deleted.
- No cleanup runs during a quiet-machine hold or while an implementation call runs. The next summary to the owner names the space freed.

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
