# Wiki log

Append-only. Format: `## [YYYY-MM-DD] op | title`, then one or two sentences.

## [2026-10-01] decision | No 0.4.1 release; the B4-12 harness is kept for stage 2.4
The owner ended the 0.4 line without a 0.4.1 release, closed step 1 at its acceptance and stopped the B4-12 baseline, measurement and 0.4 performance fixes; the harness contract, fixture builder, turn probe and summary tool stay for the stage 2.4 renderer gate ([owner-decisions-2026-10-01](decisions/owner-decisions-2026-10-01.md)). Earlier decisions that assumed the release are listed for the owner in [no-0-4-1-release-open-items](questions/no-0-4-1-release-open-items.md).

## [2026-09-30] decision | B4-12 measurement decisions
The owner chose a smoke run plus three full series per metric, everyday settings recorded exactly, PresentMon without PerfView or administrator ETW in step 2, and a read-only performance board. The harness method choices for M1 to M3 are recorded with them in [b4-12-measurement-decisions](decisions/b4-12-measurement-decisions.md). The fixture builder, the headless turn probe and the summary tool are in `tools/perf/`; the probe showed that every M1 pair re-solves orbits that 0.4 then protects, so the harness releases that protection after each pair, untimed.

## [2026-09-30] decision | One overall review for the 0.4.1 code work
The owner waived plan re-checks, per-step Codex reviews and scoped verification rounds for the current 0.4.1 code work. Each step runs its checks, and one overall Codex review covers the finished candidate before merge; recorded in [owner-decisions-2026-09-30](decisions/owner-decisions-2026-09-30.md).

## [2026-09-30] review | Build identity v2 reviewed
The step 1 candidate had one full Codex review in two parallel shards. All nine findings (eight major, one minor) were adopted. Receipts must now bind the complete inventory that the code declares, conflicting bindings are refused, build reuse is validated, and recorder, DirectX and continuity inputs are rechecked by hash. Each fix has a regression test that fails when the fix is reverted; see [build-identity-v2](concepts/build-identity-v2.md).

## [2026-09-30] update | Build identity v2 and the 0.4 native harness
0.4.1 step 1 made clean checkouts byte-exact, published the 0.4 native harness with a sanitized continuity record, and replaced the build identity; see [build-identity-v2](concepts/build-identity-v2.md) and [build-reproducibility-decisions](decisions/build-reproducibility-decisions.md).

## [2026-09-30] update | Development workbench stage C: watch, runs and launches
The [workbench](components/workbench.md) now flags owner waits, failures, stalls and overdue calls, runs the entries of a checked-in run registry with live logs and whole-tree Stop, and launches Codex calls and new Claude sessions on the owner's click. Codex wrote the runner and the flag rules; a live check on Windows confirmed failure reporting, Stop and runs that outlive the app. One full Astra review in two shards found six major problems (five distinct), all fixed with regression tests; an experiment confirmed that a descendant started through the WindowsApps `python` alias escaped Stop, and the fix. Two scoped rounds closed them and one new finding.

## [2026-09-30] update | C-03 host receipt and the first experiment-host harness
The experiment host now keeps a receipt for a committed command when its own display refresh fails, and never reports it as rejected. The worker and finalization logic moved into `ExperimentSendOutcome.cs`, which `tests/test_experiment_send_outcome.py` compiles and exercises without DirectX. This is the first regression for the experiment host itself, because `NativeHostRegression.cs` builds only the retained root host.

## [2026-09-30] update | 0.4.1 screening fixes, batch 1
Seven confirmed findings are fixed with regression tests: A-02, B-001, C-01, C-02 and the C-03 engine side by Claude; F-01 and B-002 by Codex pair implementation. Batch 1 got a plan check, one full Astra review in two shards (two majors, both fixed) and one scoped verification round. The C-03 native receipt, X-02 and A-03 remain; see [screening-findings](../progress/0.4.1/screening-findings.md).

## [2026-09-30] decision | 0.4.1 keeps the 0.4 data directory
Recorded in [owner-decisions-2026-09-30](decisions/owner-decisions-2026-09-30.md). The version and the data directory are decoupled; the plan keeps the native cache name so user-owned MPUlt settings survive.

## [2026-09-30] update | 0.4.1 screening findings
The six screening shards reported 25 leads; falsifying experiments confirmed 13 findings (10 major, 3 minor, including two found during the experiments), rejected 2 and left 11 needing Windows fixtures or timing. The ranked, sanitized list is [screening-findings](../progress/0.4.1/screening-findings.md).

## [2026-09-30] update | Development workbench stage B: hooks and status line registered
The [workbench](components/workbench.md) reporting hook and status line are now registered in `.claude/settings.json`, and `CLAUDE.md` asks Claude to answer every owner note by its id. Each registration exits silently when the script is missing. Live checks on the installed Claude Code confirmed the events, subagent routing and note delivery, and an interactive session showed the status line. Reviewed by one full Codex review and one scoped verification round.

## [2026-09-30] decision | Codex first and 0.4.1 screening packets
Recorded in [owner-decisions-2026-09-30](decisions/owner-decisions-2026-09-30.md): separable work goes to Codex and full reviews run as at least two shards. Six read-only screening packets for 0.4.1 step 4 are in `docs/progress/0.4.1/packets/`.

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
