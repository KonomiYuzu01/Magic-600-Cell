# Codex protocol

The rules for Codex calls and the review and implementation protocol that `AGENTS.md` "Team protocol" points to. This guide has the authority of `AGENTS.md` and `CLAUDE.md`, which name it, and changing it follows the rules for changing them (owner, 26 November 2026: the text moved here unchanged so that the always-loaded files stay short).

Under `AGENTS.md` "Codex minimum" (owner, 14 November 2026) most of it is suspended: Codex runs only for a gate ruling or a call the owner asks for, and such a call follows this guide. Where this guide and "Codex minimum" differ, "Codex minimum" wins.

## Team protocol detail

Roles (Codex):
- Codex is the independent reviewer and, under "Pair implementation", a co-implementer, both only within "Codex minimum" above. The default model is GPT-6.1 Sol (`gpt-6.1-sol`) for routine plan checks and reviews, scoped verification rounds and mechanical checks. Codex Astra (`gpt-6-astra`) is the senior reviewer: it gives the final ruling at three gates (the stage 2.4 day-7 go/no-go, the migration format freeze and the 1.0 architecture freeze), and it runs the plan check and the full review for critical-path changes, behaviour or contract changes and substantive design or ADR decisions, escalation diagnoses, joint attacks and milestone audits. Codex works read-only unless a packet assigns it a separate worktree, allowed files, an acceptance check and a stop condition, and it never commits to the main worktree.

Model calls:
- Pass the model and the effort explicitly on every call. Do not rely on inherited defaults.
- Codex calls default to `max`. Use `ultra` for joint attacks, escalation after repeated failure, coupled decision packages and the three Astra gates. If `ultra` is unavailable, say so and fall back to `max`. Use `high` only for mechanical checks that no script can do.
- Speed tier (owner decision, 4 October 2026): use `fast` for every plan check, review and scoped verification round, critical paths included, and for mechanical checks. Use the standard tier only for gate rulings, joint attacks and escalations.
- Sol works in a proactive persistent mode. Every review packet states "review only; do not perform follow-up work", and review calls stay read-only.

Packets and replies (review replies; the packet and privacy rules are in `AGENTS.md`):
- Review results follow `schemas/review-result.schema.json`: finding ID, severity, evidence (`path:line` with file digest), counterexample, suggested experiment, verification status.
- The integrator answers each finding with `adopt`, `reject_with_evidence` or `needs_verification`.
- After two proposer/reviewer rounds without new evidence, stop exchanging positions and design an adjudicating experiment.

When review is required (suspended under "Codex minimum"; it returns when the owner lifts the minimum):
- A non-trivial task gets a plan check before implementation and a review of the current candidate before commit. Both are mandatory for critical paths, behaviour or contract changes, and substantive design or ADR decisions. Pure spelling, formatting and mechanically provable generated-file syncs are exempt; record the reason.
- An unchanged approved plan may reuse its plan check. The final review must cover the current candidate, not an earlier diff.
- Critical paths are listed in `docs/development-guide/AGENT_BRIEFING.md`.
- Owner-dictated rule text: `AGENTS.md` "Rule text".

Review rounds (keep reviews few and decisive; under "Codex minimum" they apply only to gate rulings and calls the owner asks for):
- Review a finished candidate once: after the implementation is complete and its checks pass, not after each step. Batch related changes into one candidate.
- Only `blocker` and `major` findings block a commit. `minor` and `nit` findings never start another round: fix them in the same pass when the fix is trivial and local, otherwise record them as deferred.
- After fixing blocking findings, run a scoped verification review of the current candidate: it checks only whether each blocking finding is fixed and whether the fix introduces a new `blocker` or `major`. Do not request another full review.
- Each candidate gets at most one full review and two scoped verification rounds. If a blocking finding remains after that, stop iterating and give the owner a short decision: the remaining findings, the evidence and the options (an adjudicating experiment, escalation, or commit under an owner exception).
- A plan check runs once per task. A changed plan gets a scoped re-check of the changes only.
- Every packet states the acceptance check and what is out of scope, and the reviewer stays within it.
- Spend spare subscription quota on parallelism, not on more serial rounds: every full review of a critical-path change runs as at least two independent shards run concurrently (split by files or by concern), and the blocking findings of every shard are resolved before commit. Other full reviews may run as one shard. Paid API calls still reserve budget first.

Risk tiers (owner decision, 2 October 2026; their Codex calls are suspended under "Codex minimum"):
- Critical paths (briefing section 3) keep every rule above.
- Other code and tools: no plan check unless the change alters behaviour or a contract; one Sol review with `--speed fast` of the finished candidate; no verification round for findings below `major`.
- Documentation, wiki and progress pages that record owner decisions or measured results: no Codex review; run the wiki and agent-rules checks.
- Process budget: `AGENTS.md`.

Pair implementation (Claude and Codex write code together; off under "Codex minimum" unless the owner asks):
- Enabled: Codex writes code only through the wrapper's `--kind implement`, which creates the packet's worktree and branch itself, runs Codex there in the workspace-write sandbox, rejects any change outside the allowed files and any commit, ref, Git config, hook or link change, and runs the acceptance check in the same sandbox. The wrapper never merges.
- The Windows sandbox available without administrator rights enforces the filesystem boundary (so a push to a local repository fails) but not a network firewall. Git transports are off in the default configuration Codex and the acceptance check receive, but a process can override that, so pushes over the network and other network use are only instructed away. Firewall isolation needs the elevated backend, which is an owner decision.
- While an implementation call runs, nobody commits, tags or pushes in the repository: any ref change invalidates the run.
- The integrator splits a task with separable parts into disjoint packets. Each packet names its own worktree, allowed files, acceptance check and stop condition. Codex implements its packets in parallel while Claude implements the rest; the integrator merges and commits.
- Codex first: while Codex subscription quota allows, every separable part of a task goes to Codex. Claude keeps planning, interfaces and acceptance checks, the parts that cannot be separated, integration and review of Codex's changes. Read-only Codex screening of existing code (sharded, concurrent) is encouraged ahead of a fix phase; its findings are leads until an experiment confirms them.
- No one reviews their own code. Claude reviews Codex-authored changes, and a separate Codex call reviews Claude-authored ones. A Codex-authored critical-path change also gets a review from the other Codex model.
- Codex never commits to the main worktree or pushes. The integrator reviews `changes.patch`, applies it to the main worktree and removes the finished worktree with `--cleanup`. The review rules above apply to the merged candidate.

Records (the budgets are in `AGENTS.md` "Records and budgets"):
- Every model call gets a unique call ID and one private ledger record under `work/loop-memory/ledgers/`, written by the calling wrapper: packet digest, resolved model and effort, channel, source identity, outcome and cost status. Keep subscription usage apart from paid API charges, and reserved, estimated and billed amounts apart. Record an unknown cost as `null`, never as zero.
- The public wiki records closed problems, accepted decisions and sanitized evidence, not individual calls.
## Calling Codex from Claude Code

- Call Codex only as `AGENTS.md` "Codex minimum" allows: a gate ruling or a call the owner asks for.
- Use the wrapper for every automated call. It passes the model (`gpt-6.1-sol` by default; `gpt-6-astra` when `AGENTS.md` assigns Astra, and always with `--gate`; `gpt-6.2-astra` and `gpt-6.2-sol` when the owner asks for them, each with the owner's default effort, owner 19 November 2026), the effort and the speed tier explicitly, uses a fixed read-only policy for plan checks and reviews, ignores the user Codex config (its sandbox mode, MCP servers and plugins; sign-in must use the default file-backed store), accepts no arbitrary CLI passthrough, and rejects a run whose reported model, effort or sandbox differs from the request or whose speed tier was dropped.
- Plan check: `python tools/agents/codex_review.py --kind plan --packet <file>`
- Review: `python tools/agents/codex_review.py --kind review --packet <file>`
- Implementation: `python tools/agents/codex_review.py --kind implement --packet <file>`. The packet carries one `implement-contract` block (allowed files, acceptance check, stop condition; see `templates/problem-packet.md`). The wrapper creates `work/worktrees/<call-id>/` on branch `codex/<call-id>` from the committed HEAD, so commit what Codex needs to see before the call. It verifies the reported model, effort, sandbox and writable roots, rejects changes outside the allowed files and Git-state or link changes, and runs the acceptance check inside the Codex sandbox. Exit 0 means valid with a passing acceptance check, 4 a valid run whose check failed.
- Integrate a valid run by reading `work/reviews/<call-id>/changes.patch` and `report.md`, reviewing the change, applying it with `git apply`, and running the checks for the changed area. `meta.json` lists critical paths the change touches; those also get a review by the other Codex model. Then run `python tools/agents/codex_review.py --cleanup <call-id>`, also after a rejected or invalid run.
- Resume stays disabled in the wrapper until isolated tests show that it keeps the model, effort, sandbox and schema. Start a fresh read-only call instead. (Observed on 2026-09-29: a `resume` without `-m` ran on a different model.)
- `/codex:review` and `/codex:adversarial-review` are owner-invoked only. The Codex plugin is optional. Direct CLI automation through the wrapper is the supported path, and the plugin's stop-time review gate stays off.
## Quick reference

- Codex minimum (owner, 14 November 2026; `AGENTS.md`): no Codex call except a gate ruling or one the owner asks for; a critical-path code change gets one read by a fresh Claude subagent on a smaller model; escalations and joint attacks go to the Fable solver and a fresh Claude subagent. The points below apply only to the Codex calls that remain.
- Packet: `templates/problem-packet.md` (seven parts). Replies: `adopt`, `reject_with_evidence`, `needs_verification`, stored as `{"<finding id>": "<reply>"}` in `work/reviews/<call-id>/dispositions.json`.
- Plan check: `python tools/agents/codex_review.py --kind plan --packet <file>`
- Review: `python tools/agents/codex_review.py --kind review --packet <file>`
- Risk tiers (owner, 2 October 2026): critical paths keep every rule; other code and tools get one Sol `--speed fast` review and a plan check only for behaviour or contract changes; decision and result pages get no Codex review. Process budget: at most 30% governance or documentation commits per working week.
- Review rounds: one full review of the finished candidate, then at most two scoped verification rounds (fixed blocking findings plus new `blocker`/`major` only). `minor` and `nit` are deferred. Remaining blockers after that go to the owner as a decision.
- Escalation or joint attack: add `--effort ultra`.
- Astra (`--model gpt-6-astra`): plan checks and full reviews of critical-path, behaviour, contract and design/ADR changes; escalations, joint attacks, milestone audits. Sol: everything else.
- Gate rulings: `--model gpt-6-astra --effort ultra --gate day7-go-no-go|migration-format-freeze|architecture-freeze`.
- `--speed fast` (owner, 4 October 2026): every plan check, review and verification round, critical paths included, and mechanical checks. Standard tier: gates, joint attacks and escalations only.
- Codex first (off under the Codex minimum): give every separable part to Codex; Claude plans, integrates and reviews. Full reviews of critical-path changes run as at least two concurrent shards; other full reviews may run as one. Screening packets for 0.4.1 step 4: `docs/progress/0.4.1/packets/`.
- Pair implementation: `python tools/agents/codex_review.py --kind implement --packet <file>` (packet with an `implement-contract` block), then review and apply `changes.patch`, then `--cleanup <call-id>`. No commits, tags or pushes in the repository while a call runs. On Windows the sandbox enforces the filesystem boundary but not a network firewall.
- Escalate after three failed effective iterations (Codex diagnosis), then two more (Fable solver: the `fable-solver` subagent). Joint attack: same clean packet, sealed answers, an experiment decides.
- API team: role cards in `templates/roles/`. It may run on OpenAI models; the owner confirms provider, models and budget before each run.
- Claude subscription is Max 5x: send broad read-only searches and routine diff reading to subagents, keep Opus for planning and integration, and keep reviews small, since Codex is at its minimum. When usage runs low, record progress and stop; never switch models to keep writing code.
