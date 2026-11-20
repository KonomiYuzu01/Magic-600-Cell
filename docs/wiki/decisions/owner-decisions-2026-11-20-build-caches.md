---
id: owner-decisions-2026-11-20-build-caches
type: decision
status: verified
visibility: public
summary: Owner decision of 20 November 2026 - Rust debug and test builds keep small debug info (line tables for the workspace crates, none for the external crates) in rust/Cargo.toml and renderer/bridge/Cargo.toml, and a session cleans its own worktree's build caches when its work there is finished (CLAUDE.md working loop step 7). Release builds and optimization levels are unchanged.
related: [owner-decisions-2026-11-16-lp2-questions, owner-decisions-2026-11-14]
supersedes: []
claims:
  - {id: build-cache-decision, evidence_kind: decision, checked_at: 2026-11-20}
  - {id: dev-profile-debug-info, evidence_kind: source, path: rust/Cargo.toml, checked_at: 2026-11-20}
  - {id: bridge-profile-debug-info, evidence_kind: source, path: renderer/bridge/Cargo.toml, checked_at: 2026-11-20}
  - {id: cleanup-step, evidence_kind: source, path: CLAUDE.md, checked_at: 2026-11-20}
---

# Owner decision, 20 November 2026: smaller build caches

The owner gave this decision in chat in Chinese on 20 November 2026. This page records it in English.

## Background

On 20 November 2026 the system disk of the owner's machine was nearly full. A scan of the worktrees showed that the
Rust build caches took most of the space: every worktree builds the language workspace (`rust/`) and often the
renderer bridge (`renderer/bridge/`) in its own `target` directory, and one such directory held about 28 GB, of which
about 14 GB were Windows program databases (`.pdb`) and about 12 GB incremental-compilation data. The cause is that
debug and test builds compile every external crate at `opt-level = 2` (kept since M8, because unoptimized tests ran
too long) with full debug info, and that finished worktrees kept their caches. After the owner approved it, the
integrator deleted the caches and worktrees of finished work, which freed about 300 GB.

## Decision

The integrator proposed two measures so that the caches do not grow back, and the owner told the integrator to carry
them out:

1. **Smaller debug info.** In `rust/Cargo.toml` and `renderer/bridge/Cargo.toml`, `[profile.dev]` sets
   `debug = "line-tables-only"`, so the crates of each workspace keep line tables (backtraces and panics still name
   files and lines), and `[profile.dev.package."*"]` sets `debug = false`, so the external crates carry no debug info.
   The test profile inherits these settings. Optimization levels, overflow checks and debug assertions are unchanged,
   and so is the release profile.
2. **Sessions clean up after themselves.** `CLAUDE.md` gains working-loop step 7: when a worktree's work is finished
   (its last pull request merged and nothing more planned there), the session deletes that worktree's build caches
   with `cargo clean` for both manifests and removes any separate target directory it made; a subagent that built in a
   worktree of its own does the same before it reports. A session cleans only its own worktree, and never while the
   owner has asked for a quiet machine.

## Consequences

- The first build in each worktree after this change recompiles everything once, because the profile changed.
- A debugger shows less in debug builds (no local variables, no debug info for external crates). Whoever needs full
  debug info for one investigation can set `CARGO_PROFILE_DEV_DEBUG=true` for that build without changing the files.
- The size reduction has not been measured yet; it is to be measured on the owner's machine after the current quiet
  period, not claimed from this page.
