# Developer guide

Shared rules for every agent and person working on Magic 600 Cell. Codex reads this file directly; Claude Code imports it from `CLAUDE.md`. Read `docs/development-guide/AGENT_BRIEFING.md` before a task unless its current contents are already loaded. Only explicitly owner-approved decisions supersede older architecture text, within their recorded scope; proposals do not. Higher-priority instructions and current task permissions always apply. Detail that only some tasks need sits in guides that this file or `CLAUDE.md` names; such a guide has the authority of the file that names it, and changing it follows the same rules.

## Context and retrieval

The project is far larger than one context, and every model call re-reads its whole context, so context costs quota (owner, 26 November 2026). Load little and look up the rest.
- Always loaded: `AGENTS.md`, `CLAUDE.md` and the briefing hold only the hard rules, the current state and a map. Add a rule there only if every session needs it; put detail in a guide, a skill, the wiki or the line's own documents, and point to it.
- Look up before reading: start from the briefing's map, `docs/wiki/index.md` and the line's own plan or framework; then search (Grep, Glob, `git log -S`); then read only the line ranges you need. Do not read a large file, a log or a transcript whole.
- Keep command, build and test output in files, and read its tail, its summary or a search of it.
- Give a wide search, a read across many files or one bounded step to a fresh subagent whose brief names the files, the lines and what done looks like; it returns a conclusion, not file contents. Give each agent one step and end it when the step ends. Use a fork only when the step needs the whole conversation.
- A main session plans, integrates and decides, and keeps wide reads and logs out of its own context.
- Never trade quality for context: keep the model, the effort and the checks. Cut what is loaded, not what is verified.

## Mechanics and model

- Work only on the full `600-cell-Full` profile. Keep all 259,800 labelled sticker slots and all 1,200 legal generators. Rendering filters, framework visibility and motion sampling must never change mechanical state or relabel pieces.
- Treat `assets/manifest.json` as an immutable model boundary. Geometry, cuts, IDs, seeds and frame changes require a new model identity and migration.
- Preserve finite legal witnesses and full collateral effects for every macro. Execute source-to-destination permutations in chronological order.
- Recheck preview revisions, full-state hashes and protected-orbit constraints before commit.

## Persistence and process ownership

- Keep reset/import transactional and recoverable. Validate complete input before database writes; retain current preferences and the recovery checkpoint.
- Use `EngineProcess` (`engine_process.py`) for owned local engine startup, authenticated health checks, graceful shutdown and parent-pipe recovery.

## Verification

Tests run in tiers (owner, 26 November 2026): a commit runs the fast checks of what it touched, a pull request runs CI's fast tier, and the slow suites run in CI's full tier and together in stage 3. No tier is skipped; a check moves to a later tier only through this section.
- Per commit, about five minutes in all: the checks below for the changed area, and otherwise only the tests of the code the commit touched (`cargo test -p` for the changed crates, the changed module's own Python tests, the language scans where the area has them).
- After mechanics or persistence changes, run `python tests/test_core.py`, `python tests/test_reference_maps.py` and `python tests/test_crash.py`; after process-ownership changes, also run `python tests/test_engine_lifecycle.py`.
- After changes to agent rules, hooks, the review wrapper, the installer or skills, run `python tests/test_agent_rules_sync.py`, `python tests/test_codex_review.py`, `python tests/test_stop_gate.py`, `python tests/test_bootstrap.py`, `python tests/test_wiki_lint.py` and `python tests/test_workbench.py`.
- After changes to the development workbench (`tools/workbench/`), run `python tests/test_workbench.py` and, with the workbench environment, `tools/.venv/workbench/Scripts/python.exe tests/test_workbench_ui.py`.
- Per step, for a build or plan step that ends in a usable change: that step's own end-to-end or product-window script, once, when the step is done, not after each commit. Rebuild an export only when that script needs it.
- Per pull request: CI's fast tier (the path-selected checks within a time budget, and the tests of the changed Rust crates) passes on the final head. CI's full tier runs on every push to main, nightly and on request (`workflow_dispatch` with `tier: full`), and nobody waits for it. A red full run on main is fixed first by the line that caused it, before more work in that area merges.
- Stage 3, and a freeze or release gate that names a full run: the full regression, run together: every product-window script, the gate rows, the slow and full-replay suites, and the owner-machine, performance and long-session runs.
- Do not rerun a passing suite on an unchanged area, run the whole Rust workspace for a change to one crate, or run a product-window script on every commit.
- Compile and run `tests/native/NativeHostRegression.cs` before actual native regressions. Never run regression fixtures during normal user startup.
- Native builds of the retained 0.4/0.4.1 host need Windows, 64-bit CPython, NumPy, the .NET Framework 4.x x86 compiler and Managed DirectX. On Linux or in a cloud session, run only the headless Python checks and say which native checks were not run.
- Use fresh isolated test data. Do not run destructive tests against a personal session.

## Evidence and publishing

- Distinguish source/fixture, synthetic geometry, actual Windows/DirectX, and performance evidence. Publish only verified results for the matching source/build. Headless results cannot establish Windows/DirectX, input, long-session or performance claims.
- Preserve Andrey Astrelin's primary MPUlt credit and all upstream license notices. Do not redistribute Microsoft Managed DirectX DLLs in the public package.
- Keep public UI and documentation in English. Never publish user databases, personal logs, credentials, private paths, screenshots or raw machine diagnostics. Stage only reviewed files.
- Permission patterns are convenience controls, not a data boundary for subprocesses. Review packets and child processes may access only explicitly scoped inputs. Publishing requires the applicable owner authorization.

## Team protocol

Codex minimum (owner decision, 14 November 2026; it overrides every other Codex rule in this file, `CLAUDE.md`, the briefing and the guides until the owner lifts it):
- The owner's ChatGPT subscription quota is down to one twentieth. Keep Codex use at its minimum in all work.
- No Codex call by default: no plan check, candidate review, verification round, review shard, screening or pair implementation, critical paths included.
- Codex runs only for a gate ruling (one call per gate, at most one scoped re-check, then the owner decides) or a call the owner asks for. Such a call follows the model, effort, speed and packet rules below.
- A code change on a critical path (not rule text, documentation or wiki pages) gets one read by a fresh Claude subagent on a smaller model before commit. Give it the diff and the acceptance check, not the author's conclusion. All other work relies on the checks this file lists for the changed area.
- Escalations and joint attacks use the Fable solver and a fresh Claude subagent instead of Codex.
- No commit, merge or pull request waits for a Codex review. The commit message or pull request body names the checks that ran.

Roles:
- Claude Opus is the main-worktree integrator unless the owner explicitly assigns another integrator. Only the integrator edits the main worktree, merges results and commits. Delegated implementation is confined to packet-owned worktrees.
- Codex is the independent reviewer and, under pair implementation, a co-implementer, both only within "Codex minimum" above: GPT-6.1 Sol (`gpt-6.1-sol`) by default, and Codex Astra (`gpt-6-astra`) as the senior reviewer that gives the final ruling at the three gates. Codex works read-only unless a packet assigns it a separate worktree, allowed files, an acceptance check and a stop condition, and it never commits to the main worktree. Every Codex review packet states "review only; do not perform follow-up work".
- Fable is the solver. It joins only on escalation, for a joint attack, or for one of three proactive read-only tasks in LP2 (owner, 15 November 2026): attack tests for the M1 checker (programs that slip past the exactness rules or capability separation), crash-consistency tests for the M6 session store, and the differential test design for the M4 kernel against O01 to O07. A proactive task runs through the `fable-solver` subagent on a seven-part packet at effort `high`, one task at a time; its findings are leads until a test confirms them, and the integrator writes the tests and the fixes. Other proactive use needs the owner. Fable works for analysis and review unless a packet assigns it an isolated worktree.
- The owner decides scope, taste, UX, releases and the items under "Ask the owner". Agreement between models never replaces an owner decision.

The rest of the Codex protocol is in `docs/development-guide/codex-protocol.md`; read it when a Codex call is allowed. It holds Model calls, the review replies of Packets and replies, When review is required, Review rounds, Risk tiers, Pair implementation and the call records.

Packets and replies:
- Every hand-off uses the seven-part packet in `templates/problem-packet.md`: goal and acceptance; actual problem and reproduction; environment and versions; necessary source and evidence; attempts so far; constraints and owned files; required return format.
- Include the minimal relevant error text, code and failed results after removing credentials, private paths and personal data. Never attach personal logs or transcripts by default.
- ChatGPT Space and other hosted workspaces are never an authority; the repository is.

Rule text:
- Owner-dictated rule text is exempt from Codex review: when the owner states a rule change in `AGENTS.md`, `CLAUDE.md`, the briefing or the guides, the integrator writes it, records the owner decision in the wiki, runs the agent-rules checks and merges under "Merging". A change that widens an agent's own authority is still written by the owner. Code, hooks, the wrapper, the installer and lockfiles keep the normal review rules.

Process budget: in each working week at least 70% of the commits are experiment code, measurements or product content, and at most 30% are governance or documentation. The integrator reports the split in the milestone summary. When governance exceeds 30%, new governance work stops until the split recovers, except fixes to a broken check, hook or wrapper.

Escalation (an effective iteration is hypothesis, change, verification and judgement; under "Codex minimum" a fresh Claude subagent takes the other model's place):
- Same problem after three effective iterations: independent diagnosis by the other model.
- The same fix idea twice, or a regression: escalate early.
- Two more failed iterations after the reviewer's instructions: escalate to the solver.
- Owner dissatisfaction: immediate independent review.
- Joint attack: two independent analyses of the same clean packet, sealed before exchange; then cross-examination; then an experiment decides; one integrator implements. Model agreement is never a pass criterion.

Records and budgets:
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

Merging (the owner is not a merge button):
- The integrator merges its own pull request, without asking, when all of these hold: the per-commit checks this file lists for the changed area and CI's fast tier pass on the final head, and the PR body lists them; the review rules above are met (a valid review of the current candidate with no open `blocker` or `major`, a recorded exemption, or the read that "Codex minimum" requires where it applies); the PR contains nothing under "Ask the owner"; and the branch merges cleanly into the current base.
- Batch work: one pull request per finished task or milestone, not per commit or per fix. Follow-up fixes to an open pull request go to the same branch.
- The owner is asked only for pull requests that contain an "Ask the owner" item, that need an owner exception to the review rule, or that the owner marked for their own sign-off. Ask once, in two or three lines, never for a routine merge.
- After merging, the integrator reports what landed in a short summary. When a merged change touches an approval input (`tools/toolchain/approval_inputs.json`), the summary ends with `python tools/toolchain/bootstrap.py approve`. Batch approval-input changes so the owner approves once per milestone.
- Never force-push or rewrite `main`, never merge a branch another session is still working on, and delete merged branches.
- Questions to the owner follow the same rule: decide anything that has a conventional default or a recorded decision, record the choice, and ask only for items under "Ask the owner".
