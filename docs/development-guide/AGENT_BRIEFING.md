# Agent briefing

Operational briefing for every agent session. Claude Code loads it through `CLAUDE.md`; Codex is pointed here by `AGENTS.md`; the review wrapper sends reviewers its digest. Rules live in `AGENTS.md`; this file says where the project stands and how to act on it. People should read `HUMAN_GUIDE.md` instead.

Current phase: stage 2, working day 1 of about 20 (day 1 = 1 October 2026; days count only when the owner works); no 0.4.1 release; 0.4.1 step 1 closing

## 1. Where the project stands

- **0.4** is released (19 September 2026). Its package, provenance and hashes are frozen.
- **No 0.4.1 release** (owner decision, 1 October 2026: `docs/wiki/decisions/owner-decisions-2026-10-01.md`). The 0.4 line ends with 0.4. Step 1 (reproducible identity and harness) and the B4-12 harness, fixture builder and probe are being merged for reuse; steps 2 to 7 are cancelled. Start no 0.4 performance or release work.
- The development workbench (built during 0.4.1) is the local project monitor for 1.0 work: all sessions with summaries, a live view with access to every tool, owner notes to running sessions, work boards and always-visible progress. Scope and boundaries: `docs/wiki/decisions/development-workbench.md`.
- The 0.4.1 screening findings (`docs/progress/0.4.1/screening-findings.md`) are 1.0 requirements and tests. The migration path is re-planned in 2.0; its invariants are in section 2.
- **1.0** is a new program. Preparation and all of stage 2 are planned as about 20 working days (day 1 = 1 October 2026). Day numbers count the owner's working days, not calendar days, and exit days are targets the owner may move; nothing is cut or extended automatically, and no gate is relaxed (owner decision, 2 October 2026: `docs/wiki/decisions/owner-decisions-2026-10-02.md`). Sub-stages: 2.0 charter and authority, 2.1 two independent inventories, 2.2 dispositions, 2.3 redesign, 2.4 experiments (12-day renderer window on days 3 to 14, day-7 go/no-go, mandatory bare Direct3D 12 interop probe first), 2.5 freeze. Day-by-day plan and design track: `docs/progress/1.0/stage-2-experiment-protocol.md`.
- Stage 2.4 candidates and the renderer selection gate: `docs/wiki/decisions/renderer-candidates.md`. A renderer is selected only if it renders all 259,800 sticker slots at full detail with the most complex animation at an average of at least 30 fps and a 99th-percentile frame time of at most 33.3 ms (PresentMon, native resolution, no frame generation) on the owner's RTX 4070 Laptop GPU (8 GB, peak VRAM at most about 7 GB). NVIDIA-specific features are optional, off by default and never on the correctness path or the day-7 critical path.

## 2. Owner decisions in force (2026-09-29)

Full record: `docs/wiki/decisions/owner-decisions-2026-09-29.md`.

- 1.0 does not inherit the 0.4 runtime. Only 0.4.1 optimization records inform 1.0 engine selection. The mathematical contract, protection rules and model identity stay binding.
- The Rust-only boundary is dropped. Godot and Blender are in scope. Direct3D 12 is the base of the dedicated renderer and must be testable.
- User data migrates one way into 1.0. With no 0.4.1 release, the owner chose path B (2 October 2026: `docs/wiki/decisions/owner-decisions-2026-10-02-migration.md`): a 1.0 importer reads a locked copy of the 0.4 database; design in `docs/progress/1.0/migration-exporter-proposal.md`. The invariants stay: copy the database under the session lock, never modify the original session directory, never open a user database with `immutable=1` (it misses WAL-only commits).
- Several visual themes share one design language and users can tune them (owner decision, 2 October 2026: `docs/wiki/decisions/owner-decisions-2026-10-02.md`). Subtitles are undecided; do not assume English-only subtitles.
- Design decisions for 1.0 (owner, 2 October 2026: `docs/wiki/decisions/owner-decisions-2026-10-02-design.md`; details in charter section 5): small in-place content rewards at three occasions, never modal; one encoding grammar with a relationship strip, intuitive and with its mathematical grammar defined and proved in the theory book; two authored theme families with three scene looks each; annotated references and nexus cards; competence checks plus local in-situ metrics, soft key disclosure; Global and Local only, visible together and linked; one entry with a practice copy; an engineer's demonstrated finding affecting work or correctness goes first, otherwise the core workflow decides; trial text budgets about 20/50/80 words per screen by context; sound only after a G5 audition and an owner decision.
- Scope (owner, 2 October 2026: `docs/wiki/decisions/owner-decisions-2026-10-02-scope.md`): only sound (at most two cues, one short G5 comparison) and the mathematical-language work (theory book limited to what the 1.0 UI shows, proofs only where its correctness relies on them) are reduced; design stays the core of 1.0. The G1 to G6 tester source is open.
- Human-solve boundary (owner decision, 2 October 2026): tools may track, analyse, check, protect and reuse the solver's own macros and aid the view; setup search for a target the solver names is allowed; the program never chooses or outputs a solving macro by itself, never executes without the solver's action and never solves automatically. 1.0 is built backward from orbit-first block building (`docs/progress/1.0/solving-workflow.md`).
- Windows first; macOS and Linux versions come after 1.0 (owner decision, 2 October 2026). Keep the engine, session store, command layer and view model free of Windows-only code; put Windows code behind narrow platform interfaces; write shaders once in portable HLSL; keep a small renderer backend interface; add no Windows-only dependency outside the platform layer without a recorded reason and replacement path. No macOS or Linux product build, test or release in 1.0; the platform-independent layers and the differential oracle still run headless on Linux, as cloud sessions do.
- Superseded in `docs/architecture/1.0/10_V1_ARCHITECTURE.md`: line 13 (B4-12 deferred to 1.0), line 29 (English-only subtitles), line 45 (unspecified 1.0 behaviour inherits 0.4 contracts), lines 102 and 108 (reuse of the Python engine and the 0.4 process split as the 1.0 runtime), lines 115 and 746 (egui/wgpu as the chosen stack), line 786 (Qt only as a fallback). Line 806 still holds: its PERF packages are candidates, not authorizations. Mathematical and protection invariants in that document remain in force.

## 3. Critical paths

A change here needs a Codex plan check and a valid Codex review of the current candidate before commit. The machine-readable list used by the Stop hook is `tools/agents/critical_paths.json`.

- `core.py`, `session.py`, `session_lock.py`, `log_io.py`, `engine_process.py`, `server.py`
- `work/experiments/magic600-04/adapter.py`
- `assets/manifest.json`, model and protection logic, the migration schema, `schemas/*`
- `AGENTS.md`, `CLAUDE.md`, `.claude/settings.json`, `.claude/hooks/*`
- `tools/agents/*`, `tools/toolchain/*`, `tools/toolchain.lock.json`, `tools/skills.lock.json`, `tools/skills/*`, `tools/repo_digest.py`
- `tools/workbench/statusline.py` (Claude Code runs it in every session)

## 4. Repository map

| Path | What it is |
| --- | --- |
| root `core.py`, `session.py`, `log_io.py`, `server.py`, `engine_process.py`, `native/`, `assets/` | Shared 0.4 backend, retained renderer and full mechanical model |
| `work/experiments/magic600-04/` | The 0.4 workspace: engine entry, native launch, packaging, adapter (`Workbench`) |
| `research/` | Mathematics and engineering reports |
| `docs/architecture/1.0/` | 1.0-A3 proposal, partly superseded (section 2) |
| `docs/wiki/` | Public engineering memory (`SCHEMA.md`, `index.md`, `log.md`) |
| `docs/development-guide/` | This briefing and the human guide |
| `.agents/skills/` | Canonical skills; `.claude/skills/` holds generated copies |
| `tools/` | Review wrapper, toolchain installer, skills sync, wiki lint |
| `tools/workbench/` | 0.4.1 development workbench (Qt Quick app, brief CLI, status line); reporting hook in `.claude/hooks/report_event.py` |
| `templates/`, `schemas/` | Problem packet, API-team role cards, review and ledger schemas |
| `work/loop-memory/`, `work/reviews/` | Private ledgers, raw material, review outputs (ignored by Git) |

Native build: a clean checkout builds with the pinned engine environment (`tools/.venv/engine`, CPython 3.14.7 and NumPy 2.3.5); the recipe is in `docs/DEVELOPMENT.md`, and build identity v2 is described in `docs/wiki/concepts/build-identity-v2.md`. `work/experiments/magic600-04/print_identity.py` prints the identity without compiling.

## 5. Change type and required checks

| Change | Run |
| --- | --- |
| Mechanics or persistence | `python tests/test_core.py`, `python tests/test_reference_maps.py`, `python tests/test_crash.py` |
| Process ownership | the above plus `python tests/test_engine_lifecycle.py` |
| Native host | compile and run `tests/native/NativeHostRegression.cs` (Windows only) |
| Agent rules, hooks, wrapper, installer, skills | `python tests/test_agent_rules_sync.py`, `python tests/test_codex_review.py`, `python tests/test_stop_gate.py`, `python tests/test_bootstrap.py`, `python tests/test_workbench.py` |
| Development workbench | `python tests/test_workbench.py`; with the workbench environment `tools/.venv/workbench/Scripts/python.exe tests/test_workbench_ui.py` |
| Wiki | `python tools/wiki/lint.py` and `python tests/test_wiki_lint.py` |
| Skills | `python tools/skills/sync.py --check` |

In a Linux or cloud session, run only headless checks and list the Windows checks that were not run.

## 6. Team protocol quick reference

- Packet: `templates/problem-packet.md` (seven parts). Replies: `adopt`, `reject_with_evidence`, `needs_verification`, stored as `{"<finding id>": "<reply>"}` in `work/reviews/<call-id>/dispositions.json`.
- Plan check: `python tools/agents/codex_review.py --kind plan --packet <file>`
- Review: `python tools/agents/codex_review.py --kind review --packet <file>`
- Risk tiers (owner, 2 October 2026): critical paths keep every rule; other code and tools get one Sol `--speed fast` review and a plan check only for behaviour or contract changes; decision and result pages get no Codex review. Process budget: at most 30% governance or documentation commits per working week.
- Review rounds: one full review of the finished candidate, then at most two scoped verification rounds (fixed blocking findings plus new `blocker`/`major` only). `minor` and `nit` are deferred. Remaining blockers after that go to the owner as a decision.
- Escalation or joint attack: add `--effort ultra`.
- Astra (`--model gpt-6-astra`): plan checks and full reviews of critical-path, behaviour, contract and design/ADR changes; escalations, joint attacks, milestone audits. Sol: everything else.
- Gate rulings: `--model gpt-6-astra --effort ultra --gate day7-go-no-go|migration-format-freeze|architecture-freeze`.
- `--speed fast`: scoped verification rounds, plan re-checks, mechanical checks, non-critical reviews. Standard tier: critical full reviews and plan checks, joint attacks, escalations, gates.
- Codex first: give every separable part to Codex; Claude plans, integrates and reviews. Full reviews of critical-path changes run as at least two concurrent shards; other full reviews may run as one. Screening packets for 0.4.1 step 4: `docs/progress/0.4.1/packets/`.
- Pair implementation: `python tools/agents/codex_review.py --kind implement --packet <file>` (packet with an `implement-contract` block), then review and apply `changes.patch`, then `--cleanup <call-id>`. No commits, tags or pushes in the repository while a call runs. On Windows the sandbox enforces the filesystem boundary but not a network firewall.
- Escalate after three failed effective iterations (Codex diagnosis), then two more (Fable solver: the `fable-solver` subagent). Joint attack: same clean packet, sealed answers, an experiment decides.
- API team: role cards in `templates/roles/`. It may run on OpenAI models; the owner confirms provider, models and budget before each run.
- Claude subscription is Max 5x: send broad read-only searches and routine diff reading to subagents, keep Opus for planning and integration, and give heavy reviews to Codex. When usage runs low, record progress and stop; never switch models to keep writing code.

## 7. Tools quick reference

- `python tools/toolchain/bootstrap.py check | doctor | install <id> | install --profile <p> | install-skill <id>`
- Profiles: `planning` (now), `native-performance` (stage 2.4 measurement), `workbench` (0.4.1 development workbench), `renderer-spike` (stage 2.4), `media`, `release`, `optional`.
- winget entries are unverified until the owner runs `bootstrap.py pin <id>` on Windows; a pin changes the lockfile and returns installs to ask-first until the owner runs `bootstrap.py approve` again.
- Local output tools: Mermaid CLI (`tools/node/mermaid-cli/node_modules/.bin/mmdc`), Marp CLI (`tools/node/marp-cli/node_modules/.bin/marp`), marimo (`tools/.venv/planning`), draw.io, Typst, Blender, FFmpeg.
- Skills: `codex-dialogue`, `wiki`, `toolchain`, `marimo-pair`, and the owner's `investigate-first`, `lean-build`, `migration`, `safe-refactor`, `surgical-patch`, `verify-and-stop`.

## 8. Wiki quick reference

- Read `docs/wiki/index.md` first. Page front matter, claim format and log format: `docs/wiki/SCHEMA.md`.
- Log heading: `## [YYYY-MM-DD] op | title`.
- Authority: owner decision > `AGENTS.md`/`CLAUDE.md` > accepted ADRs and contracts > matching source and test evidence > wiki.

## 9. Hard don'ts

- Do not change model identity, cuts, IDs, seeds or frames without a new model identity and migration.
- Do not run destructive tests against a personal session, and never touch the owner's session directory.
- Do not publish user databases, personal logs, credentials, private paths, screenshots or raw diagnostics.
- Do not claim Windows/DirectX, input, long-session or performance results from headless runs.
- Do not install from hooks or at application startup; do not install anything outside the verified allowlist.
- Do not call `codex exec` directly for project work, rely on inherited model defaults, or use `resume`.
- Do not commit a critical change without a valid review of the current candidate or an explicit owner exception.
- Do not redistribute Microsoft Managed DirectX DLLs.
