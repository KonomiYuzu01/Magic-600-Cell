# Screening shard E: Retained native renderer and host (C#)

Run: `python tools/agents/codex_review.py --kind review --packet docs/progress/0.4.1/packets/screening-E-native-renderer.md --model gpt-6.1-sol --effort max --speed fast`

## 1. Goal and acceptance
- Goal: prepare 0.4.1 step 4 (screening and fixes) by finding bugs and performance hotspots in the files below before the fix phase starts.
- Acceptance check: a schema-valid result in which every finding cites `path:line`, gives a concrete counterexample or trigger, and proposes a falsifying experiment (a headless test, a fixture or a timing probe). Findings are leads, not evidence, until that experiment runs.
- Non-goals: rewrites, style, renaming, 1.0 redesign, anything that changes model identity, cuts, IDs, seeds, frames or `assets/manifest.json`, and anything already listed as a known limit in `docs/LIMITATIONS_AND_ROADMAP.md` unless you add new evidence.

## 2. Actual problem and reproduction
- 0.4 shipped with open performance gaps: the short sample missed the 100 ms instant-turn p95 target (accepted input to correct submitted frame) and continuous full-detail 30 fps; one structure-interaction sample measured 50.04 ms against a 50 ms gate (`docs/progress/0.4/snapshot/source/docs/DEVELOPMENT_PERFORMANCE.md`). B4-12 must close in 0.4.1.
- Focus for this shard: Per-frame allocations, redundant buffer uploads, full rebuilds where a partial update would do, snapshot and sticker-access cost, device-loss and lifecycle bugs, and picking/visibility errors. Rendering filters must never change mechanical state.

## 3. Environment and versions
- Base commit: current `main`.
- Evidence kind available to the reviewer: source/fixture only. Do not claim Windows/DirectX, input, long-session or performance results; mark performance findings as hypotheses with the measurement that would confirm them.

## 4. Necessary source and evidence
- Files in scope:
  - `native/*.cs`
  - `native/bootstrap.py`
  - `native/directx_runtime.py`
  - `native/inspect_runtime.py`
- Related tests (read them for intended behaviour; running them is optional): `python tests/test_native_snapshot_cache.py`, `python tests/test_native_bridge.py`.
- Invariants: `AGENTS.md` "Mechanics and model" and "Persistence and process ownership".

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| - | none | - | - | - |

## 6. Constraints and owned files
- Read-only. The reader changes no files.
- Stay inside the files in scope; cite a file outside it only as context.
- Rank findings: correctness and data-safety first, then latency on the turn and navigation paths, then code optimization.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`. Mark each finding `unverified` unless you ran a check that proves it.
- Review only; do not perform follow-up work.
