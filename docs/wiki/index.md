# Wiki index

Start here. Each entry: link, one-line summary, status. Rules for pages and operations are in [SCHEMA.md](SCHEMA.md); history is in [log.md](log.md).

## Decisions
- [owner-decisions-2026-09-29](decisions/owner-decisions-2026-09-29.md) — 0.4.1, the 1.0 program, migration, stage 2 and the development system. `verified`
- [owner-decisions-2026-09-30](decisions/owner-decisions-2026-09-30.md) — Review rounds, wider Astra role, fast tier, pair implementation with Codex. `verified`
- [owner-decisions-2026-10-01](decisions/owner-decisions-2026-10-01.md) — No 0.4.1 release; step 1 closes at its acceptance; the B4-12 harness is kept for stage 2.4; preparation and all of stage 2 within 20 days. `verified`
- [owner-decisions-2026-10-02](decisions/owner-decisions-2026-10-02.md) — Stage 2 days count the owner's working days and exit days are targets; 1.0 keeps later macOS and Linux versions cheap; Taste Lab on a private Artifact page with a learning model; 1.0 product thesis, target users and human-solve boundary; several themes on one design language. `verified`
- [owner-decisions-2026-10-02-design](decisions/owner-decisions-2026-10-02-design.md) — Ten 1.0 design decisions: small in-place rewards, one encoding grammar with defined and proved mathematical language, two theme families with scene looks, annotated references, learning measures, two linked views, one entry, role priority, text budgets, sound audition. `verified`
- [owner-decisions-2026-10-02-migration](decisions/owner-decisions-2026-10-02-migration.md) — The 1.0 migration path is option B: a 1.0 importer reads a locked copy of the 0.4 database; Astra plan check and Windows probes P1 to P3 first. `verified`
- [owner-decisions-2026-10-02-scope](decisions/owner-decisions-2026-10-02-scope.md) — Only sound and the mathematical-language work are reduced in 1.0; review effort tiered by risk with a weekly process budget; G1 to G6 tester source open. `verified`
- [development-workbench](decisions/development-workbench.md) — Scope of the 0.4.1 local development workbench: sessions with summaries, live view with access to every tool, live link to Claude, owner notes, work boards, always-visible progress; local only. `verified`
- [build-reproducibility-decisions](decisions/build-reproducibility-decisions.md) — 0.4.1 step 1: published harness and continuity record, CPython 3.14.7 with NumPy 2.3.5, byte-exact checkouts, optional desktop recorder. `verified`
- [b4-12-measurement-decisions](decisions/b4-12-measurement-decisions.md) — 0.4.1 step 2: B4-12 sample plan, measurement conditions, tools, read-only performance board and harness method choices. `draft`
- [renderer-candidates](decisions/renderer-candidates.md) — Stage 2.4 renderer candidates, the full-detail 30 fps selection gate on an RTX 4070 Laptop GPU (8 GB), and optional NVIDIA enhancements. `draft`

## Components
- [workbench](components/workbench.md) — The 0.4.1 development workbench, phase 1: sessions board with briefs, live view with tool switching, owner notes, progress board, compact view and status line. `draft`

## Concepts
- [build-identity-v2](concepts/build-identity-v2.md) — What identifies a native build of the retained 0.4 host from 0.4.1 on, how receipts are checked and what the identity cannot promise. `verified`

## Workflows
- [development-loop](workflows/development-loop.md) — Plan check, implementation, verification, Codex review and recording. `verified`

## Evidence
- [s-b-probe-results](evidence/s-b-probe-results.md) — The bare Direct3D 12 probe meets the selection gate in the W3 scene (778 fps average, 1.55 ms p99, three attended runs) on the owner's RTX 4070 Laptop GPU; its label check catches injected faults; its fenced resource handoff works on the same device, to a second device and to D3D11; PresentMon 2.6 capture lessons. `verified`

## Questions
- [no-0-4-1-release-open-items](questions/no-0-4-1-release-open-items.md) — Contradiction: the 1.0 engine selection input and the workbench performance boards still assume the cancelled 0.4.1 work. `draft`

## Dialogues
- [codex-plan-review-2026-09-29](dialogues/codex-plan-review-2026-09-29.md) — Three Codex review rounds behind the approved plan. `verified`
