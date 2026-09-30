# Developer guide

Shared rules for every agent and person working on Magic 600 Cell. Codex reads this file directly; Claude Code imports it from `CLAUDE.md`. Read `docs/development-guide/AGENT_BRIEFING.md` before a task unless its current contents are already loaded. Only explicitly owner-approved decisions supersede older architecture text, within their recorded scope; proposals do not. Higher-priority instructions and current task permissions always apply.

## Mechanics and model

- Work only on the full `600-cell-Full` profile. Keep all 259,800 labelled sticker slots and all 1,200 legal generators. Rendering filters, framework visibility and motion sampling must never change mechanical state or relabel pieces.
- Treat `assets/manifest.json` as an immutable model boundary. Geometry, cuts, IDs, seeds and frame changes require a new model identity and migration.
- Preserve finite legal witnesses and full collateral effects for every macro. Execute source-to-destination permutations in chronological order.
- Recheck preview revisions, full-state hashes and protected-orbit constraints before commit.

## Persistence and process ownership

- Keep reset/import transactional and recoverable. Validate complete input before database writes; retain current preferences and the recovery checkpoint.
- Use `EngineProcess` (`engine_process.py`) for owned local engine startup, authenticated health checks, graceful shutdown and parent-pipe recovery.

## Verification

- After mechanics or persistence changes, run `python tests/test_core.py`, `python tests/test_reference_maps.py` and `python tests/test_crash.py`; after process-ownership changes, also run `python tests/test_engine_lifecycle.py`.
- After changes to agent rules, hooks, the review wrapper, the installer or skills, run `python tests/test_agent_rules_sync.py`, `python tests/test_codex_review.py`, `python tests/test_stop_gate.py`, `python tests/test_bootstrap.py` and `python tests/test_wiki_lint.py`.
- Compile and run `tests/native/NativeHostRegression.cs` before actual native regressions. Never run regression fixtures during normal user startup.
- Native builds of the retained 0.4/0.4.1 host need Windows, 64-bit CPython, NumPy, the .NET Framework 4.x x86 compiler and Managed DirectX. On Linux or in a cloud session, run only the headless Python checks and say which native checks were not run.
- Use fresh isolated test data. Do not run destructive tests against a personal session.

## Evidence and publishing

- Distinguish source/fixture, synthetic geometry, actual Windows/DirectX, and performance evidence. Publish only verified results for the matching source/build. Headless results cannot establish Windows/DirectX, input, long-session or performance claims.
- Preserve Andrey Astrelin's primary MPUlt credit and all upstream license notices. Do not redistribute Microsoft Managed DirectX DLLs in the public package.
- Keep public UI and documentation in English. Never publish user databases, personal logs, credentials, private paths, screenshots or raw machine diagnostics. Stage only reviewed files.
- Permission patterns are convenience controls, not a data boundary for subprocesses. Review packets and child processes may access only explicitly scoped inputs. Publishing requires the applicable owner authorization.

## Team protocol

Roles:
- Claude Opus is the main-worktree integrator unless the owner explicitly assigns another integrator. Only the integrator edits the main worktree, merges results and commits. Delegated implementation is confined to packet-owned worktrees.
- Codex is the independent reviewer and, under "Pair implementation", a co-implementer. The default model is GPT-6.1 Sol (`gpt-6.1-sol`) for routine plan checks and reviews, scoped verification rounds and mechanical checks. Codex Astra (`gpt-6-astra`) is the senior reviewer: it gives the final ruling at three gates (the stage 2.4 day-7 go/no-go, the migration format freeze and the 1.0 architecture freeze), and it runs the plan check and the full review for critical-path changes, behaviour or contract changes and substantive design or ADR decisions, escalation diagnoses, joint attacks and milestone audits. Codex works read-only unless a packet assigns it a separate worktree, allowed files, an acceptance check and a stop condition, and it never commits to the main worktree.
- Fable is the solver. It joins only on escalation or for a joint attack, for analysis and review unless a packet assigns it an isolated worktree.
- The owner decides scope, taste, UX, releases and the items under "Ask the owner". Agreement between models never replaces an owner decision.

Model calls:
- Pass the model and the effort explicitly on every call. Do not rely on inherited defaults.
- Codex calls default to `max`. Use `ultra` for joint attacks, escalation after repeated failure, coupled decision packages and the three Astra gates. If `ultra` is unavailable, say so and fall back to `max`. Use `high` only for mechanical checks that no script can do.
- Speed tier: use `fast` for scoped verification rounds, plan re-checks, mechanical checks and reviews of non-critical changes. Use the standard tier for full reviews and plan checks of critical-path changes, joint attacks and escalations. Gate rulings always use the standard tier.
- Sol works in a proactive persistent mode. Every review packet states "review only; do not perform follow-up work", and review calls stay read-only.
- ChatGPT Space and other hosted workspaces are never an authority; the repository is.

Packets and replies:
- Every hand-off uses the seven-part packet in `templates/problem-packet.md`: goal and acceptance; actual problem and reproduction; environment and versions; necessary source and evidence; attempts so far; constraints and owned files; required return format.
- Include the minimal relevant error text, code and failed results after removing credentials, private paths and personal data. Never attach personal logs or transcripts by default.
- Review results follow `schemas/review-result.schema.json`: finding ID, severity, evidence (`path:line` with file digest), counterexample, suggested experiment, verification status.
- The integrator answers each finding with `adopt`, `reject_with_evidence` or `needs_verification`.
- After two proposer/reviewer rounds without new evidence, stop exchanging positions and design an adjudicating experiment.

When review is required:
- A non-trivial task gets a plan check before implementation and a review of the current candidate before commit. Both are mandatory for critical paths, behaviour or contract changes, and substantive design or ADR decisions. Pure spelling, formatting and mechanically provable generated-file syncs are exempt; record the reason.
- An unchanged approved plan may reuse its plan check. The final review must cover the current candidate, not an earlier diff.
- Critical paths are listed in `docs/development-guide/AGENT_BRIEFING.md`.

Review rounds (keep reviews few and decisive):
- Review a finished candidate once: after the implementation is complete and its checks pass, not after each step. Batch related changes into one candidate.
- Only `blocker` and `major` findings block a commit. `minor` and `nit` findings never start another round: fix them in the same pass when the fix is trivial and local, otherwise record them as deferred.
- After fixing blocking findings, run a scoped verification review of the current candidate: it checks only whether each blocking finding is fixed and whether the fix introduces a new `blocker` or `major`. Do not request another full review.
- Each candidate gets at most one full review and two scoped verification rounds. If a blocking finding remains after that, stop iterating and give the owner a short decision: the remaining findings, the evidence and the options (an adjudicating experiment, escalation, or commit under an owner exception).
- A plan check runs once per task. A changed plan gets a scoped re-check of the changes only.
- Every packet states the acceptance check and what is out of scope, and the reviewer stays within it.
- Spend spare subscription quota on parallelism, not on more serial rounds: split a large candidate into independent review shards run concurrently, and resolve the blocking findings of every shard before commit. Paid API calls still reserve budget first.

Pair implementation (Claude and Codex write code together):
- Enabled once the wrapper's `--kind implement` passes its worktree ownership and isolation tests. Until then Codex does not write code; building and testing that mode is the next development-system task.
- The integrator splits a task with separable parts into disjoint packets. Each packet names its own worktree, allowed files, acceptance check and stop condition. Codex implements its packets in parallel while Claude implements the rest; the integrator merges and commits.
- No one reviews their own code. Claude reviews Codex-authored changes, and a separate Codex call reviews Claude-authored ones. A Codex-authored critical-path change also gets a review from the other Codex model.
- Codex never commits to the main worktree or pushes. The review rules above apply to the merged candidate.

Escalation (an effective iteration is hypothesis, change, verification and judgement):
- Same problem after three effective iterations: independent diagnosis by the other model.
- The same fix idea twice, or a regression: escalate early.
- Two more failed iterations after the reviewer's instructions: escalate to the solver.
- Owner dissatisfaction: immediate independent review.
- Joint attack: two independent analyses of the same clean packet, sealed before exchange; then cross-examination; then an experiment decides; one integrator implements. Model agreement is never a pass criterion.

Records and budgets:
- Every model call gets a unique call ID and one private ledger record under `work/loop-memory/ledgers/`, written by the calling wrapper: packet digest, resolved model and effort, channel, source identity, outcome and cost status. Keep subscription usage apart from paid API charges, and reserved, estimated and billed amounts apart. Record an unknown cost as `null`, never as zero.
- The public wiki records closed problems, accepted decisions and sanitized evidence, not individual calls.
- Paid API ceilings are cumulative: USD 100, then 150, then 300. Raising a ceiling needs owner approval. Exhausted subscription quota never switches to paid API automatically. Reserve budget before concurrent paid calls.
- The external API team controller (`agent-control/`) is not in this repository. The role cards in `templates/roles/` and the schemas here are its integration contract. By default its reviewer is Claude Opus and its solver is Fable, independently of the local roles. It may also run on OpenAI models; before each run the owner confirms the provider, the model for each role and the budget reservation.

## Tools, skills and outputs

- During development, automatically install missing tools and skills only from the approved, version-pinned, checksum-verified allowlist (`tools/toolchain.lock.json`) and within existing filesystem, network, privilege and spending permissions. Never install from startup or review hooks or from normal application startup. Skill scripts, MCP servers and live notebook kernels get the same access boundaries.
- Automatic installation is authorized only for reviewed installer and lockfile revisions. A change to either invalidates that authorization. The installer rejects unknown arguments, unverified entries and source overrides before any download or write.
- Produce every formal non-software artifact with approved locally installed tools. Keep its editable source or immutable captured evidence, input digests, tool versions and production command. Local execution does not authorize external services or data uploads. Report unavailable tooling; never replace it with fabricated artifacts or evidence.
- Canonical skills live in `.agents/skills/` and are pinned in `tools/skills.lock.json`. Generated copies in `.claude/skills/` must match their source digests.

## Knowledge wiki

- `docs/wiki/` is the public engineering memory. `docs/wiki/SCHEMA.md` defines pages and the ingest, query and lint operations.
- Authority order: owner decision, then `AGENTS.md` and `CLAUDE.md`, then accepted ADRs and contracts, then source and test evidence for the matching build, then the wiki. This order governs decisions and instructions, not facts: an owner decision or ADR cannot turn a failed or unrun check into passing evidence. Keep existing invariants until an explicit, versioned change is approved.
- A conflict creates a contradiction record. It never edits a rule or accepts an ADR.
- Private material stays under `work/loop-memory/`. A private reference cannot support a publicly verifiable claim.

## Ask the owner

Ask only when an action exceeds an existing recorded authorization; do not re-request an authorization already granted. Ask before:
- administrator rights, drivers, Windows optional features or services;
- sources outside the allowlist;
- new costs, subscriptions or commercial licences, any API-team run, or raising an API ceiling;
- uploading data, external MCP servers or cloud storage;
- large model downloads, or exceeding approved disk or network budgets;
- changing security settings, antivirus exclusions, execution policy or global TDR settings;
- overwriting an installed tool version;
- real code signing and key use;
- model identity changes, accepting ADRs, UX and release sign-off, and any DELETE or AUTO feature disposition.

A checksum or signature mismatch blocks the installation until it is investigated and verified again.

See `docs/DEVELOPMENT.md`, `docs/RUNTIME_PROVENANCE.md` and `DIRECTX.md` for build, provenance and dependency details.
