# Agent briefing

Operational briefing for every agent session. Claude Code loads it through `CLAUDE.md`; Codex is pointed here by `AGENTS.md`; the review wrapper sends reviewers its digest. Rules live in `AGENTS.md`; this file says where the project stands and how to act on it. People should read `HUMAN_GUIDE.md` instead.

Current phase: stage 2; the engineering line develops the project's own language first and the earlier schedule is set aside (owner, 3 November 2026); LP1 signed and the language architecture and migration format frozen (owner, 15 November 2026); LP2 and LP3 exited (owner, 22 November 2026), the design system, ADR-D01, frozen at A7 (owner, 23 November 2026), and LP4 and LP5 exited (owner, 24 November 2026); no 0.4.1 release; 0.4.1 step 1 closing

## 1. Where the project stands

- **0.4** is released (19 September 2026). Its package, provenance and hashes are frozen.
- **No 0.4.1 release** (owner decision, 1 October 2026: `docs/wiki/decisions/owner-decisions-2026-10-01.md`). The 0.4 line ends with 0.4. Step 1 (reproducible identity and harness) and the B4-12 harness, fixture builder and probe are being merged for reuse; steps 2 to 7 are cancelled. Start no 0.4 performance or release work.
- The development workbench (built during 0.4.1) is the local project monitor for 1.0 work: all sessions with summaries, a live view with access to every tool, owner notes to running sessions, work boards and always-visible progress. Scope and boundaries: `docs/wiki/decisions/development-workbench.md`.
- The 0.4.1 screening findings (`docs/progress/0.4.1/screening-findings.md`) are 1.0 requirements and tests. The migration path is re-planned in 2.0; its invariants are in section 2.
- **Settled so far.** The language is 600 Cell Script (`.6cs`). The engine, the session store, the command layer and the interpreter's host are in Rust; the language runs on route RT1, its own interpreter over the typed IR; a program either reads state or prepares previews (LQ8, LQ9, LQ21). ADR-L01 to ADR-L12 are accepted, and the language architecture and the migration format are frozen (LP1, 15 November 2026). The syntax is candidate A, braces (LQ13). The renderer is selected (`docs/wiki/decisions/renderer-freeze.md`). LP2 to LP5 have exited. ADR-D01, the design system, is accepted at revision 6 as UI/UX freeze round 1, and where it conflicts with a frozen clause, ADR-D01 wins. LQ2's option set is BS2, with policy table version 1 as overlay OV-5, which LP6 implements; until then the interim reading stays in force in code.
- **Lines in force.** ADR-D02 builds the Jumble and research surfaces as a shell in the product shell; ADR-D03 builds the complete canvas and UI/UX shell on sample data, without the backend or servers, and is accepted by a learning-curve solve test. For each of them, Fable writes the build-direction framework and Opus builds, in the subscription with no API. K2's greybox stage wires stage 2's usable flows to the backend (the log and document flow became the sixth on 26 November 2026). The IR line builds the constructs the IR draft lacks; then a syntax and IR freeze packet goes to the owner. The design and art line uses no API until the owner says so. A5's API spend passed the USD 150 ceiling with the owner's approval, under a Console limit of USD 200.
- **History.** How each of these was decided, from the 1.0 program's start on 1 October 2026 to 26 November 2026, with a link to every owner decision, is in `docs/development-guide/briefing-history.md`. Read the item a task touches.
- **26 November 2026, the order to release** (owner: `docs/wiki/decisions/owner-decisions-2026-11-26-release-order.md`):
  - The target: 1.0 is released before 2027, preferably by 20 December 2026. Plan from the measured pace, not from the plans' agent-day estimates.
  - ADR-D02 is finished first, then the IR line and the remaining tests.
  - Then ADR-D03 and LP6 start together. ADR-D03 gets a full A1 to A4 and is finished after its reduced solver-agent run.
  - After LP6 the language product freezes, and K2 wires it into the program. K2 starts only when ADR-D03 and LP6 are both done.
  - The owner makes a reduced Taste Lab selection first. K2 then gives each of its lines a full A1 to A4.
  - Nexus cards enter the product in K2, and K2's art line does the fine work that freeze rounds 3 and 4 held for after stage 2.
  - Then the greybox wires the backend and brings the program online. A full-power solver-agent run gives constructive findings, and refinement continues.
  - Last, the promotion line is planned and its tools are installed. Stage 2 then freezes with the program's first version.
  - Stage 3 runs only the full-volume acceptance and bug fixes. Stage 4 makes the complete promotional film, then packages, overhauls the GitHub home page and releases.
  - December's API budget may be used. The cumulative ceiling's USD 300 step is still the owner's to set.
  - The research line's G0 stays an open question.
- Stage 2.4 candidates and the renderer selection gate: `docs/wiki/decisions/renderer-candidates.md`. A renderer is selected only if it renders all 259,800 sticker slots at full detail with the most complex animation at an average of at least 30 fps and a 99th-percentile frame time of at most 33.3 ms (PresentMon, native resolution, no frame generation) on the owner's RTX 4070 Laptop GPU (8 GB, peak VRAM at most about 7 GB). NVIDIA-specific features are optional, off by default and never on the correctness path or the day-7 critical path.

## 2. Owner decisions in force (2026-09-29)

Full record: `docs/wiki/decisions/owner-decisions-2026-09-29.md`.

- 1.0 does not inherit the 0.4 runtime. Only 0.4.1 optimization records inform 1.0 engine selection. The mathematical contract, protection rules and model identity stay binding.
- The Rust-only boundary is dropped. Godot and Blender are in scope. Direct3D 12 is the base of the dedicated renderer and must be testable.
- User data migrates one way into 1.0. With no 0.4.1 release, the owner chose path B (2 October 2026: `docs/wiki/decisions/owner-decisions-2026-10-02-migration.md`): a 1.0 importer reads a locked copy of the 0.4 database; design in `docs/progress/1.0/migration-exporter-proposal.md`. The invariants stay: copy the database under the session lock, never modify the original session directory, never open a user database with `immutable=1` (it misses WAL-only commits).
- Several visual themes share one design language and users can tune them (owner decision, 2 October 2026: `docs/wiki/decisions/owner-decisions-2026-10-02.md`). Subtitles are undecided; do not assume English-only subtitles.
- Design decisions for 1.0 (owner, 2 October 2026: `docs/wiki/decisions/owner-decisions-2026-10-02-design.md`; details in charter section 5): small in-place content rewards at three occasions, never modal; one encoding grammar with a relationship strip, intuitive and with its mathematical grammar defined and proved in the theory book; two authored theme families with three scene looks each; annotated references and nexus cards; competence checks plus local in-situ metrics, soft key disclosure; Global and Local visible together and linked (the limit to these two views was lifted on 6 November 2026); one entry with a practice copy; an engineer's demonstrated finding affecting work or correctness goes first, otherwise the core workflow decides; trial text budgets about 20/50/80 words per screen by context; sound only after a G5 audition and an owner decision.
- Scope (owner, 2 October 2026: `docs/wiki/decisions/owner-decisions-2026-10-02-scope.md`): only sound (at most two cues, one short G5 comparison) and the mathematical-language work (theory book limited to what the 1.0 UI shows, proofs only where its correctness relies on them) are reduced; design stays the core of 1.0. The sound reduction was lifted on 22 November 2026: sound, motion and icons are not cut (`docs/wiki/decisions/owner-decisions-2026-11-22-design-art-path.md`). The language and what the research needs of it joined the scope on 3 November 2026; design follows the language. The owner alone takes G1 to G6 (owner, 3 October 2026: `docs/wiki/decisions/owner-decisions-2026-10-03.md`).
- Human-solve boundary (owner decision, 2 October 2026): tools may track, analyse, check, protect and reuse the solver's own macros and aid the view; setup search for a target the solver names is allowed; the program never chooses or outputs a solving macro by itself, never executes without the solver's action and never solves automatically. A tool a solver develops themselves, through the built-in base tools and while understanding its principle, that assists solving does not count as the program outputting a solution (owner, 3 November 2026); the owner and the integrator set the exact boundary together, and until then the built-in tools keep this boundary. 1.0 is built backward from orbit-first block building (`docs/progress/1.0/solving-workflow.md`).
- Stage 2.0 charter signed as written (owner, 3 October 2026); stage 2.0 is closed. A Windows CI job runs the headless checks; its pass is not Windows/DirectX evidence.
- Windows first; macOS and Linux versions come after 1.0 (owner decision, 2 October 2026). Keep the engine, session store, command layer and view model free of Windows-only code; put Windows code behind narrow platform interfaces; write shaders once in portable HLSL; keep a small renderer backend interface; add no Windows-only dependency outside the platform layer without a recorded reason and replacement path. No macOS or Linux product build, test or release in 1.0; the platform-independent layers and the differential oracle still run headless on Linux, as cloud sessions do.
- Jumbling (owner, 9 October 2026: `docs/wiki/decisions/owner-decisions-2026-10-09-jumbling.md`): no fudged variant. Rigid jumbling of the retained geometry enters 1.0 as an independent puzzle with its own model identity and as a Jumble feature, which reopens the charter's mathematical contract for that puzzle only. 600-cell-Full and its contract are unchanged. Plan: `docs/progress/1.0/jumbling-plan.md`. The twist menu is an owner sign-off at the 2.5 freeze. The human-solve boundary applies unchanged except at one point (owner, 17 October 2026: `docs/wiki/decisions/owner-decisions-2026-10-17-repair-pairs.md`): in the three-twist repair of the jumbled puzzle's descent, at a cap the solver names, the program may list the pairs (x, d) whose repair lowers N, each with its y and z, in a neutral order without ranking, and the solver picks one.
- Design table (owner, 22 October 2026: `docs/wiki/decisions/owner-decisions-2026-10-22-dt-m1.md`): DT-J1 and DT-M1 are settled for now, every recommended option, with the other options kept on record. The design line and the round table are on hold after their current stage: open no design-table topic (DT-J2 to DT-J4 included) until the owner resumes them. Engineering (EN2 to EN4) continues. Updated 3 November 2026: the round table resumes for the language only, with the mathematician and the engineer advisor; the front-end designer and the interaction artist, DT-J2 to DT-J4 and the design line's UI/UX work wait until the language's development is complete, which is when LP2 to LP5 are done apart from LP4's editor look (owner, 6 November 2026). Updated 22 November 2026: DT-J2 to DT-J4 resume while the Jumbling ADR (ADR-D02) is written after A6, and the design line's K2, with the front-end designer and the interaction artist, opens after A7 and ADR-D02, beside LP4 and LP5.
- Superseded in `docs/architecture/1.0/10_V1_ARCHITECTURE.md`: line 13 (B4-12 deferred to 1.0), line 29 (English-only subtitles), line 45 (unspecified 1.0 behaviour inherits 0.4 contracts), lines 102 and 108 (reuse of the Python engine and the 0.4 process split as the 1.0 runtime), lines 115 and 746 (egui/wgpu as the chosen stack), line 786 (Qt only as a fallback). Line 806 still holds: its PERF packages are candidates, not authorizations. Mathematical and protection invariants in that document remain in force.

## 3. Critical paths

Under `AGENTS.md` "Codex minimum" (owner, 14 November 2026) a code change here gets one read by a fresh Claude subagent on a smaller model before commit, and no Codex call. The machine-readable list is `tools/agents/critical_paths.json`; the Stop hook that reads it is unregistered until the owner lifts the minimum.

- `core.py`, `session.py`, `session_lock.py`, `log_io.py`, `engine_process.py`, `server.py`
- `work/experiments/magic600-04/adapter.py`
- `assets/manifest.json`, model and protection logic, the migration schema, `schemas/*`
- `AGENTS.md`, `CLAUDE.md`, `docs/development-guide/codex-protocol.md`, `docs/development-guide/claude-session-operations.md`, `.claude/settings.json`, `.claude/hooks/*`
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
| `docs/development-guide/` | This briefing, the human guide and the guides loaded on demand: the Codex protocol, Claude session operations (machine cleanup, hooks) and the briefing history |
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

These are the per-commit checks; CI's fast tier runs on each pull request, and the full regression runs in CI's full tier and in stage 3 (`AGENTS.md` "Verification"). In a Linux or cloud session, run only headless checks and list the Windows checks that were not run.

## 6. Team protocol quick reference

- Codex minimum (owner, 14 November 2026; `AGENTS.md`): no Codex call except a gate ruling or one the owner asks for; a critical-path code change gets one read by a fresh Claude subagent on a smaller model; escalations and joint attacks go to the Fable solver and a fresh Claude subagent.
- Gate rulings: `--model gpt-6-astra --effort ultra --gate day7-go-no-go|migration-format-freeze|architecture-freeze`. Every other Codex command, model, tier and round limit: `docs/development-guide/codex-protocol.md` "Quick reference".
- Claude subscription is Max 5x: send broad read-only searches and routine diff reading to subagents, keep Opus for planning and integration, and keep reviews small, since Codex is at its minimum. When usage runs low, record progress and stop; never switch models to keep writing code.

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
- Do not commit a code change on a critical path without the fresh Claude subagent read that the Codex minimum requires, or an explicit owner exception.
- Do not redistribute Microsoft Managed DirectX DLLs.
