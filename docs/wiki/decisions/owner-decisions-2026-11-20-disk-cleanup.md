---
id: owner-decisions-2026-11-20-disk-cleanup
type: decision
status: verified
visibility: public
summary: Owner decision of 20 November 2026 - the main session regularly cleans this project's dormant worktrees and files on the owner's machine to free SSD space (once a day while sessions work, before each owner-machine sitting, and below 100 GB free), under rules in CLAUDE.md that define a dormant worktree, what may go and what is never deleted.
related: [owner-decisions-2026-11-20-build-caches]
supersedes: []
claims:
  - {id: disk-cleanup-decision, evidence_kind: decision, checked_at: 2026-11-20}
  - {id: disk-cleanup-rule, evidence_kind: source, path: CLAUDE.md, checked_at: 2026-11-20}
---

# Owner decision, 20 November 2026: regular cleanup of dormant worktrees and files

The owner gave this decision in chat in Chinese on 20 November 2026: dormant worktrees and other files of this project
on the owner's machine are to be cleaned up regularly, to free space on the SSD. This page records it in English.

## Background

Earlier the same day the [build-cache decision](owner-decisions-2026-11-20-build-caches.md) made each session clean
its own worktree's build caches when its work there is finished. That rule depends on each session ending its work
cleanly. Sessions that stop, lose their shell or wait for days leave worktrees and caches behind, and so do scratch
files in the system temp folder.

## Decision

The integrator wrote the rule into `CLAUDE.md`, after working-loop step 7. The thresholds are the integrator's
defaults, and the owner may change them:

- **Who and when.** The main session cleans once a day while sessions work, before each owner-machine sitting, and
  whenever the system disk has less than 100 GB free.
- **Dormant worktrees.** A worktree is dormant when it is clean, every commit on it is on the remote, and either its
  last pull request is merged or closed with nothing more planned there, or it has had no commit or session activity
  for 3 days. It is removed without `--force`, after any evidence in its ignored `work/` folder has been moved to the
  main checkout or kept. Codex worktrees go through the wrapper's cleanup.
- **Other files.** Build caches go in dormant worktrees and in worktrees whose session has been idle for a day.
  Sessions' scratch folders and task outputs in the system temp folder go after 7 days when their session is not
  running.
- **Never deleted.** The owner's session directory and user databases, private records, review records, evidence that
  is cited or not yet recorded, installed tools, stashes and unpushed commits. Anything unclear is listed to the owner
  with its size instead of deleted.
- **Not during measurements.** No cleanup runs during a quiet-machine hold or an implementation call.

## Consequences

- A worktree removed after 3 idle days is gone for its session; its branch stays on the remote.
- A session whose idle worktree lost its build caches rebuilds once when it resumes.
- The space freed is reported in the next summary to the owner; nothing on this page claims a measured saving.
