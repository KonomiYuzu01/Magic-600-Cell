# Agent briefing

Operational briefing for every agent session. Claude Code loads it through `CLAUDE.md`; Codex is pointed here by `AGENTS.md`; the review wrapper sends reviewers its digest. Rules live in `AGENTS.md`; this file says where the project stands and how to act on it. People should read `HUMAN_GUIDE.md` instead.

Current phase: 0.4.1 step 1 (reproducible identity and harness); development system deployed 2026-09-29; stage 2 not started

## 1. Where the project stands

- **0.4** is released (19 September 2026). Its package, provenance and hashes are frozen.
- **0.4.1** is the next release and the historical closing release of the 0.4 line: optimization, debugging, performance and bug fixes, code optimization, and a local development workbench. B4-12 closes here.
- 0.4.1 order: (1) reproducible identity and harness; (2) baseline; (3) exporter and fixtures; (4) screening and fixes; (5) freeze candidate; (6) formal measurement, 3 x 100; (7) release.
- The 0.4.1 development workbench is a local project monitor connected live to Claude: all sessions with summaries, a live view with access to every tool, owner notes to running sessions, work boards, always-visible progress, and performance boards added in steps 2 and 6. Scope and boundaries: `docs/wiki/decisions/development-workbench.md`.
- Must not be deferred out of 0.4.1: reproducible build, minimal exporter, migration schema and validator, core fixtures, B4-12 closeout.
- **1.0** is a new program. Stage 2 (inventory, teardown, redesign) starts after 0.4.1: 2.0 charter and authority, 2.1 two independent inventories, 2.2 dispositions, 2.3 redesign, 2.4 experiments (15 working days, day-7 go/no-go, mandatory bare Direct3D 12 interop probe first), 2.5 freeze.
- Stage 2.4 candidates and the renderer selection gate: `docs/wiki/decisions/renderer-candidates.md`. A renderer is selected only if it renders all 259,800 sticker slots at full detail with the most complex animation at an average of at least 30 fps and a 99th-percentile frame time of at most 33.3 ms (PresentMon, native resolution, no frame generation) on the owner's RTX 4070 Laptop GPU (8 GB, peak VRAM at most about 7 GB). NVIDIA-specific features are optional, off by default and never on the correctness path or the day-7 critical path.

## 2. Owner decisions in force (2026-09-29)

Full record: `docs/wiki/decisions/owner-decisions-2026-09-29.md`.

- 1.0 does not inherit the 0.4 runtime. Only 0.4.1 optimization records inform 1.0 engine selection. The mathematical contract, protection rules and model identity stay binding.
- The Rust-only boundary is dropped. Godot and Blender are in scope. Direct3D 12 is the base of the dedicated renderer and must be testable.
- User data migrates one way through a versioned `.c600migrate` package: the 0.4.1 exporter copies the database under the session lock and never modifies the original session directory. Never open a user database with `immutable=1` (it misses WAL-only commits).
- Themes and subtitles are undecided. Do not assume English-only subtitles.
- Superseded in `docs/architecture/1.0/10_V1_ARCHITECTURE.md`: line 13 (B4-12 deferred to 1.0), line 29 (English-only subtitles), line 45 (unspecified 1.0 behaviour inherits 0.4 contracts), lines 102 and 108 (reuse of the Python engine and the 0.4 process split as the 1.0 runtime), lines 115 and 746 (egui/wgpu as the chosen stack), line 786 (Qt only as a fallback). Line 806 still holds: its PERF packages are candidates, not authorizations. Mathematical and protection invariants in that document remain in force.

## 3. Critical paths

A change here needs a Codex plan check and a valid Codex review of the current candidate before commit. The machine-readable list used by the Stop hook is `tools/agents/critical_paths.json`.

- `core.py`, `session.py`, `session_lock.py`, `log_io.py`, `engine_process.py`, `server.py`
- `work/experiments/magic600-04/adapter.py`
- `assets/manifest.json`, model and protection logic, the migration schema, `schemas/*`
- `AGENTS.md`, `CLAUDE.md`, `.claude/settings.json`, `.claude/hooks/*`
- `tools/agents/*`, `tools/toolchain/*`, `tools/toolchain.lock.json`, `tools/skills.lock.json`, `tools/skills/*`, `tools/repo_digest.py`

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
| `templates/`, `schemas/` | Problem packet, API-team role cards, review and ledger schemas |
| `work/loop-memory/`, `work/reviews/` | Private ledgers, raw material, review outputs (ignored by Git) |

Known gap: `tests/run_postapproval.py` is hashed by `native_launch.py` but absent from the public checkout (`docs/DEVELOPMENT.md`), so the native build is not a clean-checkout recipe yet. 0.4.1 step 1 fixes this.

## 5. Change type and required checks

| Change | Run |
| --- | --- |
| Mechanics or persistence | `python tests/test_core.py`, `python tests/test_reference_maps.py`, `python tests/test_crash.py` |
| Process ownership | the above plus `python tests/test_engine_lifecycle.py` |
| Native host | compile and run `tests/native/NativeHostRegression.cs` (Windows only) |
| Agent rules, hooks, wrapper, installer, skills | `python tests/test_agent_rules_sync.py`, `python tests/test_codex_review.py`, `python tests/test_stop_gate.py`, `python tests/test_bootstrap.py` |
| Wiki | `python tools/wiki/lint.py` and `python tests/test_wiki_lint.py` |
| Skills | `python tools/skills/sync.py --check` |

In a Linux or cloud session, run only headless checks and list the Windows checks that were not run.

## 6. Team protocol quick reference

- Packet: `templates/problem-packet.md` (seven parts). Replies: `adopt`, `reject_with_evidence`, `needs_verification`, stored as `{"<finding id>": "<reply>"}` in `work/reviews/<call-id>/dispositions.json`.
- Plan check: `python tools/agents/codex_review.py --kind plan --packet <file>`
- Review: `python tools/agents/codex_review.py --kind review --packet <file>`
- Review rounds: one full review of the finished candidate, then at most two scoped verification rounds (fixed blocking findings plus new `blocker`/`major` only). `minor` and `nit` are deferred. Remaining blockers after that go to the owner as a decision.
- Escalation or joint attack: add `--effort ultra`.
- Astra (`--model gpt-6-astra`): plan checks and full reviews of critical-path, behaviour, contract and design/ADR changes; escalations, joint attacks, milestone audits. Sol: everything else.
- Gate rulings: `--model gpt-6-astra --effort ultra --gate day7-go-no-go|migration-format-freeze|architecture-freeze`.
- `--speed fast`: scoped verification rounds, plan re-checks, mechanical checks, non-critical reviews. Standard tier: critical full reviews and plan checks, joint attacks, escalations, gates.
- Pair implementation: `python tools/agents/codex_review.py --kind implement --packet <file>` (packet with an `implement-contract` block), then review and apply `changes.patch`, then `--cleanup <call-id>`. No commits, tags or pushes in the repository while a call runs. On Windows the sandbox enforces the filesystem boundary but not a network firewall.
- Escalate after three failed effective iterations (Codex diagnosis), then two more (Fable solver: the `fable-solver` subagent). Joint attack: same clean packet, sealed answers, an experiment decides.
- API team: role cards in `templates/roles/`. It may run on OpenAI models; the owner confirms provider, models and budget before each run.
- Claude subscription is Max 5x: send broad read-only searches and routine diff reading to subagents, keep Opus for planning and integration, and give heavy reviews to Codex. When usage runs low, record progress and stop; never switch models to keep writing code.

## 7. Tools quick reference

- `python tools/toolchain/bootstrap.py check | doctor | install <id> | install --profile <p> | install-skill <id>`
- Profiles: `planning` (now), `native-performance` (0.4.1), `renderer-spike` (stage 2.4), `media`, `release`, `optional`.
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
