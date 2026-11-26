---
id: owner-decisions-2026-11-26-context-and-test-tiers
type: decision
status: verified
visibility: public
summary: Owner decisions of 26 November 2026 on context and tests - agents load only the hard rules, the current state and a map, and look up the rest (map and index first, then search, then line ranges; logs stay in files; fresh one-step subagents); the suspended Codex protocol, Claude's hooks and machine cleanup, and the briefing's dated history move unchanged into guides loaded on demand; tests run in tiers - per commit only the touched area's fast checks (about five minutes), per step the step's own end-to-end script once, per pull request CI's fast tier, with the full tier on every push to main, nightly and on request, and the full regression together in stage 3.
related: [owner-decisions-2026-11-26-release-order, owner-decisions-2026-11-16-time-saving]
supersedes: []
claims:
  - {id: context-and-retrieval, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: guides-on-demand, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: test-tiers, evidence_kind: decision, checked_at: 2026-11-26}
  - {id: ci-fast-and-full-tiers, evidence_kind: decision, checked_at: 2026-11-26}
---

# Owner decisions, 26 November 2026: context and test tiers

**How the decisions came about.** On the evening of 26 November 2026 the owner wrote three messages (in Chinese),
translated:
1. "On context: this project's scale is far beyond a typical vibe-coded project. Most agents do not need to read the
   whole project's context to do their work; there should be a mechanism that finds the useful things directly in the
   repository. See what you think. Change nothing yet; if it works, I will change AGENTS.md directly."
2. "Also, the project's tests take far too long. Most of them can be run together in stage 3, instead of every commit
   spending 40 to 60 minutes repeating them."
3. After the integrator proposed both changes: "Write the draft", and "once it is written, change it directly
   yourself."

The integrator's measurements that day, from its own session records (private; they motivate the change and are not
public evidence): about three quarters of the week's Claude quota went to re-reading context on each call, and a
build session averaged about 295,000 tokens of context per call. The always-loaded rule files (`AGENTS.md`,
`CLAUDE.md` and the briefing) held about 62,600 characters. A full CI run took 35 to 51 minutes, and a build session
spent over three hours of one afternoon in local test runs, product-window scripts and exports.

## The decisions

1. **Context and retrieval** (`AGENTS.md` "Context and retrieval"). The always-loaded files hold only the hard rules,
   the current state and a map. An agent starts from the briefing's map, `docs/wiki/index.md` and its line's own plan,
   then searches, then reads only the line ranges it needs; it keeps command and test output in files; it gives a wide
   search or one bounded step to a fresh subagent with a brief that names the files and what done looks like. Quality
   is not traded for context: the model, the effort and the checks stay.
2. **Guides loaded on demand.** Text that only some tasks need moved unchanged into guides that the rule files name,
   with their authority: `docs/development-guide/codex-protocol.md` (the Codex protocol suspended under "Codex minimum",
   Claude's commands for calling Codex, and the full team-protocol quick reference),
   `docs/development-guide/claude-session-operations.md` (machine cleanup and the hooks) and
   `docs/development-guide/briefing-history.md` (the briefing's dated items from 1 October to 26 November 2026, a record
   that links each owner decision). The two rule guides join the critical paths. The always-loaded files now hold about
   43,300 characters.
3. **Test tiers** (`AGENTS.md` "Verification"). Per commit, about five minutes: the checks the rules name for the
   changed area (the mechanics, persistence, process-ownership, agent-rules and workbench checks are unchanged) and
   otherwise only the tests of the touched code. Per step: the step's own end-to-end or product-window script, once,
   when the step is done. Stage 3, and a freeze or release gate that names a full run: the full regression, run
   together.
4. **CI tiers.** A pull request runs CI's fast tier: the path-selected checks within a time budget, and the tests of
   the changed Rust crates. The full tier runs on every push to main, nightly and on request (`workflow_dispatch` with
   `tier: full`); nobody waits for it, and a red full run on main is fixed first by the line that caused it. The merge
   rule names the per-commit checks and the fast tier.

## What this does not do

- It deletes no check: checks move between tiers, and every one still runs in CI's full tier and in stage 3.
- It moves no bar, edits no frozen text and changes no moved rule's wording. "Codex minimum" is unchanged and still
  overrides the guides.
- The governance share of the week's commits is over the 30% budget; the owner asked for this change directly.
