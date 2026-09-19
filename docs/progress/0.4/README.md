# Magic 600 Cell 0.4 — archived development progress

> **Historical snapshot (17 September 2026).** Version 0.4 was subsequently released on 19 September; see the [release guide](../../RELEASE_0_4.md) for the published package. Status, failures and remaining-work lists below describe this earlier snapshot only. The archived documents and their manifest remain unchanged.

Published **2026-09-18** from the existing **2026-09-17 stopped handoff**. This publication updates documentation only. It does not resume development, run application tests, promote candidate code, or release a 0.4 binary. The published 0.3 package remains unchanged.

## Status at the archived handoff

**G1 Grip/Twist and G2 workbench samples were approved. Final 0.4 acceptance is not complete.** The experimental work now covers linked real geometry and cycle views, mathematical names, keyboard interaction, macros, protection, references, scoring, endgame and cross-orbit work. Component evidence does not establish full native usability or a human full solve.

The authoritative record for this archived handoff is [Evidence closeout](snapshot/experiment/docs/reviews/evidence-closeout-20260917.md). Read it before older documents. [Handoff](snapshot/experiment/HANDOFF.md) and [development log](snapshot/experiment/DEVELOPMENT_LOG.md) preserve the history; historical Pending/Approved or next-step instructions refer to their original dates, not permission to resume now.

## Recorded verification, with limits

| Scope | Recorded result | Important boundary |
|---|---|---|
| Final keyboard batch | 104 native checks; 134 inputs unchanged | In experiment source; injected input, not physical-key latency qualification |
| Core/reference/crash checks | Three commands passed | Existing headless evidence; not rerun for this publication |
| Full headless baseline | 55 groups, 497 reported cases; 46 groups passed, 9 failed; overall exit 1 | Failures retained; no overall pass claim |
| Selected manual endgame paths | 35 moving orbits / 59 legal stages passed | Finite witnesses, not arbitrary protected-state reachability or a human full solve |
| Candidate backend/fixtures | 42/44 passed; overall exit 1 | Two test-copy assertions remain; later edits not all tested |
| Independent solver wave-d | Two actual E1 insertions, Next, undo/redo, protection rejection/cancel and exact-completion UI | Raw-ID workaround disclosed; independent endgame/Resume/Strict/cross-orbit checks remain |
| Frozen-package checks | Seven groups passed in the recorded package check | No frozen-package GUI acceptance; not a final release package |

Three fixes remain **unpromoted**: C1 post-primary cancellation reply, C2 missing-frame diagnosis, and C3 mathematical-name orbit-protection picker. Their candidate evidence must not be described as integrated product acceptance.

## Remaining work recorded at the archived handoff

1. Close the two identified test-copy assertion issues.
2. Integrate reviewed C1/C2/C3 candidates and verify affected behavior.
3. Close recorded frame/transport evidence gaps.
4. Complete final native continuous operations, worksheet reuse, focus/window/input-latency and independent solver scenarios.
5. Complete the final package, native launch acceptance, captioned recording and release preparation.

No percentage or completion-date estimate is asserted. Instant turn, framework visibility, low detail and retained display controls remain in scope. Existing valid evidence for unchanged behavior should be reused.

## Architecture and product scope

- [0.4 architecture](snapshot/design/03_ARCHITECTURE.md)
- [Mathematical naming and recommendation contract](snapshot/design/07_NAMING_AND_RECOMMENDATION.md)
- [Integration, residuals and endgame contract](snapshot/design/08_INTEGRATION_AND_ENDGAME.md)
- [Steering backlog](snapshot/design/09_STEERING_BACKLOG.md)
- [Independent solver review](snapshot/experiment/docs/reviews/native-solver-review-20260917.md)
- [Final solver acceptance](snapshot/experiment/docs/reviews/final-solver-acceptance.md)
- [Final acceptance plan](snapshot/experiment/docs/plans/final-acceptance-20260917.md)

## Snapshot scope and provenance

This archive includes all Markdown development records at the experiment root, all Markdown plans/specifications/reviews under its docs directory, all Markdown evidence summaries under its evidence directory, the existing 0.4 design pack, next-session handoff documents, and selected source-level development/roadmap records. Original languages and historical results are preserved. The older product brief and prompts remain historical, not current implementation authority.

Excluded: chat/shared personal memory, credentials, personal sessions, application code, binaries, screenshots/videos, raw JSON/log evidence and third-party runtimes. Bare paths in archived documents identify local evidence; they are not claims that those files were uploaded. Missing local Markdown links are rendered as explicit local-only references. This is a documentation archive, not a runnable or self-contained reproducibility package.

[Manifest](manifest.json) binds each archived source digest and its published, anonymized copy. Source documents and the active checkout were not changed by this export.

## Complete document inventory

- [snapshot/design/00_READ_FIRST_ZH.md](snapshot/design/00_READ_FIRST_ZH.md)
- [snapshot/design/01_PRODUCT_BRIEF_ZH.md](snapshot/design/01_PRODUCT_BRIEF_ZH.md)
- [snapshot/design/02_ABSTRACT_VIEW_SPEC_ZH.md](snapshot/design/02_ABSTRACT_VIEW_SPEC_ZH.md)
- [snapshot/design/03_ARCHITECTURE.md](snapshot/design/03_ARCHITECTURE.md)
- [snapshot/design/04_ULTRA_PROMPTS.md](snapshot/design/04_ULTRA_PROMPTS.md)
- [snapshot/design/05_ACCEPTANCE_AND_TEST_GUIDE.md](snapshot/design/05_ACCEPTANCE_AND_TEST_GUIDE.md)
- [snapshot/design/06_REFERENCE_AUDIT.md](snapshot/design/06_REFERENCE_AUDIT.md)
- [snapshot/design/07_NAMING_AND_RECOMMENDATION.md](snapshot/design/07_NAMING_AND_RECOMMENDATION.md)
- [snapshot/design/08_INTEGRATION_AND_ENDGAME.md](snapshot/design/08_INTEGRATION_AND_ENDGAME.md)
- [snapshot/design/09_STEERING_BACKLOG.md](snapshot/design/09_STEERING_BACKLOG.md)
- [snapshot/design/README.md](snapshot/design/README.md)
- [snapshot/experiment/DEVELOPMENT_LOG.md](snapshot/experiment/DEVELOPMENT_LOG.md)
- [snapshot/experiment/HANDOFF.md](snapshot/experiment/HANDOFF.md)
- [snapshot/experiment/ID_AUDIT.md](snapshot/experiment/ID_AUDIT.md)
- [snapshot/experiment/NATIVE_REVIEW_GUIDE.md](snapshot/experiment/NATIVE_REVIEW_GUIDE.md)
- [snapshot/experiment/docs/CONSOLIDATION.md](snapshot/experiment/docs/CONSOLIDATION.md)
- [snapshot/experiment/docs/final-package-notes.md](snapshot/experiment/docs/final-package-notes.md)
- [snapshot/experiment/docs/plans/current-score-integration-20260916.md](snapshot/experiment/docs/plans/current-score-integration-20260916.md)
- [snapshot/experiment/docs/plans/endgame-completion-20260916.md](snapshot/experiment/docs/plans/endgame-completion-20260916.md)
- [snapshot/experiment/docs/plans/final-acceptance-20260917.md](snapshot/experiment/docs/plans/final-acceptance-20260917.md)
- [snapshot/experiment/docs/reviews/MACRO_VARIANTS_REVIEW_20260916.md](snapshot/experiment/docs/reviews/MACRO_VARIANTS_REVIEW_20260916.md)
- [snapshot/experiment/docs/reviews/TOMORROW_COVERAGE_20260916.md](snapshot/experiment/docs/reviews/TOMORROW_COVERAGE_20260916.md)
- [snapshot/experiment/docs/reviews/capture-focus-20260916.md](snapshot/experiment/docs/reviews/capture-focus-20260916.md)
- [snapshot/experiment/docs/reviews/coherent-frame-plan-20260916.md](snapshot/experiment/docs/reviews/coherent-frame-plan-20260916.md)
- [snapshot/experiment/docs/reviews/current-score-integration-20260916.md](snapshot/experiment/docs/reviews/current-score-integration-20260916.md)
- [snapshot/experiment/docs/reviews/endgame-evidence-audit-20260916.md](snapshot/experiment/docs/reviews/endgame-evidence-audit-20260916.md)
- [snapshot/experiment/docs/reviews/endgame-integration-20260917.md](snapshot/experiment/docs/reviews/endgame-integration-20260917.md)
- [snapshot/experiment/docs/reviews/evidence-closeout-20260917.md](snapshot/experiment/docs/reviews/evidence-closeout-20260917.md)
- [snapshot/experiment/docs/reviews/final-compatibility-20260917.md](snapshot/experiment/docs/reviews/final-compatibility-20260917.md)
- [snapshot/experiment/docs/reviews/final-native-acceptance-map.md](snapshot/experiment/docs/reviews/final-native-acceptance-map.md)
- [snapshot/experiment/docs/reviews/final-native-session-20260917.md](snapshot/experiment/docs/reviews/final-native-session-20260917.md)
- [snapshot/experiment/docs/reviews/final-package-checker-review-20260917.md](snapshot/experiment/docs/reviews/final-package-checker-review-20260917.md)
- [snapshot/experiment/docs/reviews/final-solver-acceptance.md](snapshot/experiment/docs/reviews/final-solver-acceptance.md)
- [snapshot/experiment/docs/reviews/instrument-20260915.md](snapshot/experiment/docs/reviews/instrument-20260915.md)
- [snapshot/experiment/docs/reviews/keyboard-batch-independent-review-20260917.md](snapshot/experiment/docs/reviews/keyboard-batch-independent-review-20260917.md)
- [snapshot/experiment/docs/reviews/keyboard-dock-20260915.md](snapshot/experiment/docs/reviews/keyboard-dock-20260915.md)
- [snapshot/experiment/docs/reviews/mathematical-addresses-20260916.md](snapshot/experiment/docs/reviews/mathematical-addresses-20260916.md)
- [snapshot/experiment/docs/reviews/native-macro-discovery-plan-review-20260917.md](snapshot/experiment/docs/reviews/native-macro-discovery-plan-review-20260917.md)
- [snapshot/experiment/docs/reviews/native-solver-review-20260917.md](snapshot/experiment/docs/reviews/native-solver-review-20260917.md)
- [snapshot/experiment/docs/reviews/orbit_signature_feasibility_20260916.md](snapshot/experiment/docs/reviews/orbit_signature_feasibility_20260916.md)
- [snapshot/experiment/docs/reviews/reference-alignment-discovery-20260916.md](snapshot/experiment/docs/reviews/reference-alignment-discovery-20260916.md)
- [snapshot/experiment/docs/reviews/retained-display-controls-20260917.md](snapshot/experiment/docs/reviews/retained-display-controls-20260917.md)
- [snapshot/experiment/docs/reviews/use-score-plan-20260916.md](snapshot/experiment/docs/reviews/use-score-plan-20260916.md)
- [snapshot/experiment/docs/specs/endgame-intent-stage-20260916.md](snapshot/experiment/docs/specs/endgame-intent-stage-20260916.md)
- [snapshot/experiment/docs/specs/instrument-scene-20260915.md](snapshot/experiment/docs/specs/instrument-scene-20260915.md)
- [snapshot/experiment/docs/specs/keyboard-dock-lucid.mmd](snapshot/experiment/docs/specs/keyboard-dock-lucid.mmd)
- [snapshot/experiment/docs/specs/macro-variants-stage-20260916.md](snapshot/experiment/docs/specs/macro-variants-stage-20260916.md)
- [snapshot/experiment/docs/specs/phase-workspace-20260915.md](snapshot/experiment/docs/specs/phase-workspace-20260915.md)
- [snapshot/experiment/docs/specs/post-approval-workspace-20260916.md](snapshot/experiment/docs/specs/post-approval-workspace-20260916.md)
- [snapshot/experiment/docs/specs/solve-transition-stage-20260916.md](snapshot/experiment/docs/specs/solve-transition-stage-20260916.md)
- [snapshot/experiment/docs/specs/solve-use-stage-20260916.md](snapshot/experiment/docs/specs/solve-use-stage-20260916.md)
- [snapshot/experiment/evidence/browseract-setup-report.md](snapshot/experiment/evidence/browseract-setup-report.md)
- [snapshot/experiment/evidence/clipboard-restore-20260917/DIAGNOSIS.md](snapshot/experiment/evidence/clipboard-restore-20260917/DIAGNOSIS.md)
- [snapshot/experiment/evidence/compact-keyboard-20260916/REVIEW.md](snapshot/experiment/evidence/compact-keyboard-20260916/REVIEW.md)
- [snapshot/experiment/evidence/endgame-index-20260917.md](snapshot/experiment/evidence/endgame-index-20260917.md)
- [snapshot/experiment/evidence/endgame-library-matrix.md](snapshot/experiment/evidence/endgame-library-matrix.md)
- [snapshot/experiment/evidence/final-headless-resume-20260917/BASELINE_RESULT.md](snapshot/experiment/evidence/final-headless-resume-20260917/BASELINE_RESULT.md)
- [snapshot/experiment/evidence/final-headless-resume-20260917/closeout/HANDOFF-before.md](snapshot/experiment/evidence/final-headless-resume-20260917/closeout/HANDOFF-before.md)
- [snapshot/experiment/evidence/final-headless-resume-20260917/fixture-candidates/STATUS.md](snapshot/experiment/evidence/final-headless-resume-20260917/fixture-candidates/STATUS.md)
- [snapshot/experiment/evidence/final-headless-resume-20260917/fixture-review.md](snapshot/experiment/evidence/final-headless-resume-20260917/fixture-review.md)
- [snapshot/experiment/evidence/functions-20260916/FRONTEND.md](snapshot/experiment/evidence/functions-20260916/FRONTEND.md)
- [snapshot/experiment/evidence/keymap-files-20260917/README.md](snapshot/experiment/evidence/keymap-files-20260917/README.md)
- [snapshot/experiment/evidence/macro-variants-20260916/REVIEW.md](snapshot/experiment/evidence/macro-variants-20260916/REVIEW.md)
- [snapshot/experiment/evidence/math-glyph-20260916-210146/REVIEW.md](snapshot/experiment/evidence/math-glyph-20260916-210146/REVIEW.md)
- [snapshot/experiment/evidence/math-glyph-20260916-211106/REVIEW.md](snapshot/experiment/evidence/math-glyph-20260916-211106/REVIEW.md)
- [snapshot/experiment/evidence/native-solver-review-20260917/input-helper-a.md](snapshot/experiment/evidence/native-solver-review-20260917/input-helper-a.md)
- [snapshot/experiment/evidence/native-solver-review-20260917/wave-b-review-snapshot.md](snapshot/experiment/evidence/native-solver-review-20260917/wave-b-review-snapshot.md)
- [snapshot/experiment/evidence/native-solver-review-20260917/wave-d/input-helper.md](snapshot/experiment/evidence/native-solver-review-20260917/wave-d/input-helper.md)
- [snapshot/experiment/evidence/native-solver-review-20260917/wave-d/report-before-wave-d.md](snapshot/experiment/evidence/native-solver-review-20260917/wave-d/report-before-wave-d.md)
- [snapshot/experiment/evidence/native-solver-review-20260917/wave-d/review.md](snapshot/experiment/evidence/native-solver-review-20260917/wave-d/review.md)
- [snapshot/experiment/evidence/performance-resume-20260917/REVIEW.md](snapshot/experiment/evidence/performance-resume-20260917/REVIEW.md)
- [snapshot/experiment/evidence/performance-resume-20260917/backend-review/API_DIAGNOSTIC.md](snapshot/experiment/evidence/performance-resume-20260917/backend-review/API_DIAGNOSTIC.md)
- [snapshot/experiment/evidence/performance-resume-20260917/backend-review/FRESH_PROFILE.md](snapshot/experiment/evidence/performance-resume-20260917/backend-review/FRESH_PROFILE.md)
- [snapshot/experiment/evidence/performance-resume-20260917/backend-review/REVIEW.md](snapshot/experiment/evidence/performance-resume-20260917/backend-review/REVIEW.md)
- [snapshot/experiment/evidence/performance-resume-20260917/hub-review/FINE_TIMING_051917.md](snapshot/experiment/evidence/performance-resume-20260917/hub-review/FINE_TIMING_051917.md)
- [snapshot/experiment/evidence/performance-resume-20260917/hub-review/REVIEW.md](snapshot/experiment/evidence/performance-resume-20260917/hub-review/REVIEW.md)
- [snapshot/experiment/evidence/performance-resume-20260917/keyboard-review/PATCH_PROPOSAL.md](snapshot/experiment/evidence/performance-resume-20260917/keyboard-review/PATCH_PROPOSAL.md)
- [snapshot/experiment/evidence/performance-resume-20260917/keyboard-review/REVIEW.md](snapshot/experiment/evidence/performance-resume-20260917/keyboard-review/REVIEW.md)
- [snapshot/experiment/evidence/performance-resume-20260917/keyboard-review/candidate/README.md](snapshot/experiment/evidence/performance-resume-20260917/keyboard-review/candidate/README.md)
- [snapshot/experiment/evidence/reference-variant-integration-20260917/README.md](snapshot/experiment/evidence/reference-variant-integration-20260917/README.md)
- [snapshot/experiment/evidence/role-text-20260916-213632/REVIEW.md](snapshot/experiment/evidence/role-text-20260916-213632/REVIEW.md)
- [snapshot/experiment/evidence/session-log-native-20260917/README.md](snapshot/experiment/evidence/session-log-native-20260917/README.md)
- [snapshot/experiment/evidence/session-presentation-20260917/README.md](snapshot/experiment/evidence/session-presentation-20260917/README.md)
- [snapshot/experiment/evidence/solve-window-20260916/IMPLEMENTATION.md](snapshot/experiment/evidence/solve-window-20260916/IMPLEMENTATION.md)
- [snapshot/experiment/evidence/workflow-continuity-20260916/README.md](snapshot/experiment/evidence/workflow-continuity-20260916/README.md)
- [snapshot/experiment/evidence/workflow-ownership-20260916/README.md](snapshot/experiment/evidence/workflow-ownership-20260916/README.md)
- [snapshot/next-session/START_HERE.md](snapshot/next-session/START_HERE.md)
- [snapshot/source/docs/DEVELOPMENT_LOG.md](snapshot/source/docs/DEVELOPMENT_LOG.md)
- [snapshot/source/docs/DEVELOPMENT_PERFORMANCE.md](snapshot/source/docs/DEVELOPMENT_PERFORMANCE.md)
- [snapshot/source/docs/LIMITATIONS_AND_ROADMAP.md](snapshot/source/docs/LIMITATIONS_AND_ROADMAP.md)
- [snapshot/source/docs/NEXT_UPDATE.md](snapshot/source/docs/NEXT_UPDATE.md)
