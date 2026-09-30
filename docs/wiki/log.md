# Wiki log

Append-only. Format: `## [YYYY-MM-DD] op | title`, then one or two sentences.

## [2026-09-30] update | Build identity v2 and the 0.4 native harness
0.4.1 step 1 made clean checkouts byte-exact, published the 0.4 native harness with a sanitized continuity record, and replaced the build identity; see [build-identity-v2](concepts/build-identity-v2.md) and [build-reproducibility-decisions](decisions/build-reproducibility-decisions.md).

## [2026-09-30] decision | Workbench watches continuously and launches work
The owner extended the [workbench scope](decisions/development-workbench.md): continuous watch with stall and failure flags, and launching sessions, wrapper calls and registered experiment runs on the owner's click. Recorded in [owner-decisions-2026-09-30](decisions/owner-decisions-2026-09-30.md).

## [2026-09-30] review | Development workbench stage A reviewed
The stage A candidate of the [workbench](components/workbench.md) had one full Codex review in two parallel shards (Astra on the critical files, Sol on the app) and two scoped verification rounds; every blocking finding was fixed with a regression test. The last open ordering edge case was settled by an adjudicating experiment with mutation checks and committed under an owner exception.

## [2026-09-30] update | Development workbench phase 1, stage A
Added the [workbench](components/workbench.md) component page: the Qt Quick app, the brief CLI, the status line script, the public progress file and the unregistered reporting hook. Hook and status-line registration follow in stage B.

## [2026-09-30] update | Codex calls ignore the user config
Plan, review and implementation calls now pass `--ignore-user-config` (and name the unelevated Windows sandbox backend), so user-configured MCP servers, plugins and sandbox settings never load during a wrapper call; sign-in must use the default file-backed store.

## [2026-09-30] update | Codex pair implementation enabled
The review wrapper gained `--kind implement` and `--cleanup`, with isolation tests in `tests/test_codex_implement.py` and one real smoke run; the [development loop](workflows/development-loop.md) and the rules now describe it. On Windows without administrator setup the Codex sandbox enforces the filesystem boundary but not a network firewall.

## [2026-09-30] decision | Review rounds, Astra, fast tier and pair implementation
Recorded in [owner-decisions-2026-09-30](decisions/owner-decisions-2026-09-30.md); `AGENTS.md`, `CLAUDE.md`, the guides, the codex-dialogue skill and the review wrapper now match.

## [2026-09-30] decision | Development workbench scope
Recorded the owner's definition of the 0.4.1 development workbench in [development-workbench](decisions/development-workbench.md): all sessions with summaries, a live view with access to every tool, a live link to Claude with owner notes, work boards and always-visible progress, local only.

## [2026-09-29] decision | Renderer candidates and selection gate
Recorded the stage 2.4 candidates, the owner's full-detail stable 30 fps gate and the optional NVIDIA enhancements in [renderer-candidates](decisions/renderer-candidates.md). Vendor facts are unverified.

## [2026-09-29] decision | Development system and 1.0 roadmap approved
Recorded the owner decisions in [owner-decisions-2026-09-29](decisions/owner-decisions-2026-09-29.md) and the Codex review rounds in [codex-plan-review-2026-09-29](dialogues/codex-plan-review-2026-09-29.md).

## [2026-09-29] update | Wiki created
Created the schema, index, log and the [development-loop](workflows/development-loop.md) workflow page.
