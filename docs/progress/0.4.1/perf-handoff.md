# B4-12 performance work: closing handoff

On 1 October 2026 the owner cancelled the 0.4.1 release, the B4-12 baseline, the formal measurement and every 0.4 performance fix ([owner-decisions-2026-10-01](../../wiki/decisions/owner-decisions-2026-10-01.md)). The work stopped at a clean point. This page records what exists, what was not built, and what the stage 2.4 renderer gate can reuse.

## Kept for reuse

| Part | Path | What stage 2.4 can reuse |
|---|---|---|
| Harness contract | [b4-12-harness.md](b4-12-harness.md) | Metric boundaries (input QPC to Present return or `WM_PAINT` plus `GdiFlush`), posted owned-window input, cold start from a copied fixture, private run directory, validity rules (state hash before and after, full-detail draw count, foreground, normal exit), and the public and private split of the environment record. |
| Fixture builder | `tools/perf/b412_fixture.py`, `tests/test_b412_fixture.py` | Builds, copies and verifies a fresh data directory with an independently scripted state hash and a `fixture.json` record. It drives the 0.4 engine, so a 1.0 renderer needs its own state source, but the record format and the copy and verify rules carry over. |
| Turn probe | `tools/perf/turn_probe.py`, `tests/test_turn_probe.py` | Headless attribution of the engine side of a native turn: request, polling, reply size and native snapshot cost. Attribution only; it never yields a gate result. |
| Summary tool | `tools/perf/b412_summary.py`, `tests/test_b412_summary.py` | Nearest-rank statistics, pooled and per-run p95, the 20 % spread flag, and the PresentMon frame statistics (fps from the mean `MsBetweenPresents`, p99 frame time), which are the measures of the renderer gate. |

## Not built

The native harness (`B412Checks.cs`), the runner (`run_b412.py` with `b412_environment.py`), the journal-equivalence recorder and the engine fixes (X-02 checkpoint list, live-turn review) were planned but not implemented. The PresentMon command line was never fixed because no PresentMon version was pinned.

## Findings worth keeping

- 0.4 protects every orbit that a commit re-solves and then rejects a live turn that moves it, so every M1 inverse pair needs an untimed protection release.
- A step 3 session database has a pinned `sqlite_master`, so a performance fix must not add an index or any other schema object.
- On the 0.4 live turn path, skipping the draft review is not equivalent: it also saves the session timer on the browser route, raises validation errors that stop a live turn, and leaves a different residual cache after a failed turn. Any later engine optimization needs failure-path equivalence, not only equal committed results.
- A Codex implementation call becomes invalid when any ref in the shared repository changes, including a fetch or a new session's branch. Parallel sessions must agree on a ref freeze before such a call.
