# Wiki index

Start here. Each entry: link, one-line summary, status. Rules for pages and operations are in [SCHEMA.md](SCHEMA.md); history is in [log.md](log.md).

## Decisions
- [owner-decisions-2026-09-29](decisions/owner-decisions-2026-09-29.md) — 0.4.1, the 1.0 program, migration, stage 2 and the development system. `verified`
- [owner-decisions-2026-09-30](decisions/owner-decisions-2026-09-30.md) — Review rounds, wider Astra role, fast tier, pair implementation with Codex. `verified`
- [owner-decisions-2026-10-01](decisions/owner-decisions-2026-10-01.md) — No 0.4.1 release; step 1 closes at its acceptance; the B4-12 harness is kept for stage 2.4; preparation and all of stage 2 within 20 days. `verified`
- [owner-decisions-2026-10-02](decisions/owner-decisions-2026-10-02.md) — Stage 2 days count the owner's working days and exit days are targets; 1.0 keeps later macOS and Linux versions cheap; Taste Lab on a private Artifact page with a learning model; 1.0 product thesis, target users and human-solve boundary; several themes on one design language. `verified`
- [owner-decisions-2026-10-02-design](decisions/owner-decisions-2026-10-02-design.md) — Ten 1.0 design decisions: small in-place rewards, one encoding grammar with defined and proved mathematical language, two theme families with scene looks, annotated references, learning measures, two linked views, one entry, role priority, text budgets, sound audition. `verified`
- [owner-decisions-2026-10-02-migration](decisions/owner-decisions-2026-10-02-migration.md) — The 1.0 migration path is option B: a 1.0 importer reads a locked copy of the 0.4 database; Astra plan check and Windows probes P1 to P3 first. `verified`
- [owner-decisions-2026-10-03-migration](decisions/owner-decisions-2026-10-03-migration.md) — After P1 found no read-only strategy, the importer copies the database and WAL bytes under the session lock and recovers only the copy; P1 is re-run for it first. `verified`
- [owner-decisions-2026-10-02-scope](decisions/owner-decisions-2026-10-02-scope.md) — Only sound and the mathematical-language work are reduced in 1.0; review effort tiered by risk with a weekly process budget; G1 to G6 tester source (decided 3 October: the owner only). `verified`
- [owner-decisions-2026-10-03](decisions/owner-decisions-2026-10-03.md) — The owner alone takes G1 to G6; the stage 2.0 charter is signed as written; a Windows CI job runs the headless checks. `verified`
- [owner-decisions-2026-10-03-taste-lab](decisions/owner-decisions-2026-10-03-taste-lab.md) — Taste Lab learner: thresholds unchanged, criteria checked at 90 comparisons per family and the relevance ranking at 160 (provisional before); settled rule recalibrated; current learner final after the cross diagnosis, acceptance run once; learner accepted with recorded limitations, two-family checkpoint at 150 comparisons per family. `verified`
- [owner-decisions-2026-10-03-image-library](decisions/owner-decisions-2026-10-03-image-library.md) — Taste Lab image library starts now, in parallel with the close of phase 1a: class A sources, embedding model and runtime (phase 1b), class B private references kept local (phase 2), annotated references and nexus cards (4A); review and installation gates unchanged. `verified`
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

## Questions
- [no-0-4-1-release-open-items](questions/no-0-4-1-release-open-items.md) — Contradiction: the 1.0 engine selection input and the workbench performance boards still assume the cancelled 0.4.1 work. `draft`

## Dialogues
- [codex-plan-review-2026-09-29](dialogues/codex-plan-review-2026-09-29.md) — Three Codex review rounds behind the approved plan. `verified`
