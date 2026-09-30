# 0.4.1 screening findings

Ranked result of the read-only 0.4.1 screening (step 4 preparation) at commit 9768913. Six Codex shards (packets in `packets/`) reported 25 leads. The integrator then answered every lead and ran a falsifying experiment for each adopted one. Two more findings came up during the experiments.

Evidence kinds:
- **Headless fixture**: the full 259,800-slot model in a fresh temporary session, or the production functions with injected faults.
- **Source fixture**: a static check of the named source.
- **Headless timing**: wall-clock timing of the Python engine on the development machine.

No Windows/DirectX, input-device or native frame-time results are claimed. Only confirmed findings become fix packets.

## Counts

| | Blocker | Major | Minor | Total |
|---|---|---|---|---|
| Leads reported by the shards | 0 | 13 | 12 | 25 |
| Confirmed (ranked below; includes 1 new) | 0 | 10 | 3 | 13 |
| Rejected with evidence | | | | 2 |
| Needing verification | | | | 11 |

Two leads were merged into one finding: D-01 and the new X-01 share one root cause. A-01 was lowered from major to minor because no current caller can trigger it.

## Ranked confirmed findings

Order: correctness and data safety first, then turn and navigation latency, then code optimization.

### Correctness and data safety

| # | ID | Sev. | Finding | Where | Experiment and result |
|---|---|---|---|---|---|
| 1 | A-02 | major | Save log writes a C600 proof log that the application's own import rejects. Preview allows 3,000,000 primitives per operation and has no transaction cap. Import caps a log at 2,000,000 primitives and 10,000 transactions. | `session.py:331`, `log_io.py:64`, `log_io.py:96` | Headless fixture. A 2.1M-primitive history and a 10,001-transaction history both exported. Import rejected both. |
| 2 | C-03 | major | A committed native command is reported as a failed job when the paired snapshot fails afterwards. The journal has the event, but the user sees a failure and may retry. | `work/experiments/magic600-04/adapter.py:2120` | Headless fixture (injected snapshot failure). Journal head advanced to 1; the job reported an error. |
| 3 | X-01 (with D-01) | major | A default Git for Windows clone (`core.autocrlf=true`, no `.gitattributes`) writes hashed text assets and proof sources with CRLF. The model refuses to load ("Asset hash mismatch"), and the bundled invariant proof reads as stale. | `assets/manifest.json`, `core.py:41`, `work/experiments/magic600-04/orbit_invariants.py:109` | Headless fixture. `Model()` fails in such a checkout and loads once the same files are LF. The step-1 lane already owns the fix (line-ending attributes and a checkout-bytes check; see [clean-checkout-audit](clean-checkout-audit.md)). |
| 4 | B-001 | major | One idle connection holds engine shutdown past the owner's 12-second grace: request sockets wait 20 s and the server joins handler threads. The engine is force-terminated instead of closing in order. | `server.py:138`, `server.py:359`, `engine_process.py:166` | Headless fixture, real owned engine. Shutdown took 13.3 s and ended forced. |
| 5 | F-01 | major | With a forecast token selected, the Solve window buttons Place only, Hold place, Free exact and Free place still change position protection. The declared restriction lists these commands, but the direct-button guard omits them. | `work/experiments/magic600-04/native/ExperimentShell.cs:113`, `ExperimentTools.cs:93`, `ExperimentSolveWindow.cs:56` | Source fixture. All four commands are unguarded, all four are Solve buttons, and no enablement check applies. |
| 6 | C-02 | major | A failed orbit change leaves the workspace orbit and the session orbit different. This happens whether the failure is in the orbit save or the workspace save. | `work/experiments/magic600-04/adapter.py:1871`, `:282` | Headless fixture (injected save failure). Both cases diverged (0 vs 33, 33 vs 0). |
| 7 | C-01 | major | The E1 practice fixture commits through the session directly. It moves a piece under a captured exact position lock and then discards the lock. | `work/experiments/magic600-04/adapter.py:1815` | Headless fixture. The fixture was accepted, the locked slots changed and no locks remained. |
| 8 | B-002 | major | A native build rejected because its inputs changed during compilation still records its identity and digest. After the source is restored, the next build reuses the rejected binary. | `work/experiments/magic600-04/native_launch.py:145` | Headless fixture (production `build()` in a temporary root). One compile ran; the second build returned the rejected binary. |
| 9 | B-003 | major | The declared harness input `tests/run_postapproval.py` is missing, so a clean checkout cannot build the native host. This gap is already known (clean-checkout audit CC-7) and is scheduled for 0.4.1 step 1. | `work/experiments/magic600-04/native_launch.py:33` | Source fixture. The file is absent. |
| 10 | A-01 | minor | A preview keeps the caller's move list. If the caller changes that list before commit, the journal stores a witness that does not replay to the committed state. No current caller does this. | `core.py:174`, `session.py:159` | Headless fixture (API level). The journal stored the changed witness, and replay did not match the post-state. |
| 11 | A-03 | minor | Inverse, conjugate and commutator composition raise `KeyError` for a star step without an explicit sign, although validation accepts it with sign +1. | `enhanced.py:65`, `core.py:24` | Headless fixture. All three operations raised. |
| 12 | C-04 | minor | Rejected draft input removes the recorded-scramble provenance, so the next preview is journalled as manual work. | `work/experiments/magic600-04/adapter.py:1346` | Headless fixture. Provenance was dropped; the preview was classified as manual. |

### Turn and navigation latency

| # | ID | Sev. | Finding | Where | Experiment and result |
|---|---|---|---|---|---|
| 13 | X-02 | major | `Session.status()`, returned by every commit, undo and redo, lists checkpoints by reading `created`, a column stored after each checkpoint's label blob. Its cost grows about 0.3 ms per checkpoint, and an automatic checkpoint is added every 50 turns. | `session.py:326` | Headless timing. `status()` took 0.3, 14.8, 61.3 and 126.1 ms at 0, 50, 200 and 400 checkpoints. The same rows without `created` and ordering took 0.04 ms. |

### Code optimization

None confirmed. The optimization leads are listed under "Needing verification".

## Rejected with evidence

| ID | Lead | Evidence |
|---|---|---|
| A-04 | Unindexed journal lookups slow each turn | Headless timing. The parent lookup took 0.07 ms in a 3,000-event journal, against a 26 ms turn. The measured growth comes from X-02. |
| D-02 | Local-centre navigation repeats global filter evaluation | Headless timing. A local-centre request took 1.7 ms at p95; filter evaluation is a large share of a negligible cost. |

## Needing verification

| ID | Sev. (reported) | Lead | Needed experiment |
|---|---|---|---|
| E-02 | major | The Center action loads grips synchronously on the UI thread, and a failed lookup reaches the application-exit handler | The lookup is synchronous (source), and the engine answers it after any running job: 6 ms idle, 64.8 s behind a 2.9M-primitive preview (headless timing). The host disables Center while its own job runs, so a reachable freeze is not shown. It needs a Windows fixture with a lookup that fails or is delayed. |
| B-005 | minor | GET responses hold the session lock while writing | Lock retention is visible in the source. A stalled reader of the 1 MB labels response did not delay `/api/status` (13 ms), because that response fits the loopback buffers. It needs a response larger than the socket buffers, such as a native snapshot or an export. |
| B-004 | minor | Concurrent job submissions can bypass the one-job limit | The race is visible in the source, but 0 of 40 synchronized real-HTTP pairs reproduced it. It needs an instrumented admission fixture. |
| E-01 | major | Device loss during a native click can leave publication held | Windows device-lost fixture |
| F-02 | major | Held Enter or Space can append a macro repeatedly | WinForms key-repeat message fixture |
| F-03 | minor | Enter is consumed before the macro list sees it | WinForms message-routing fixture |
| E-03 | minor | Auxiliary tooltips keep stale counts | CPU control fixture |
| D-03 | minor | After-graph paging rebuilds forecasts before checking the cache | Headless paging timing |
| E-04 | minor | Metadata-only snapshots still do full-slot UI work | Windows UI timing |
| F-04 | minor | A hidden keyboard window keeps its update work | Windows UI timing |
| F-05 | minor | Every macro-row paint revalidates the candidate batch | Windows UI timing |

## Next

Fix status, batch 1 (headless and source evidence only; the native host was compiled but not run):

| ID | Status | Regression |
|---|---|---|
| A-02 | Fixed. Save log and Export refuse a record that import would reject and suggest a session backup. | `tests/test_log_budget.py` |
| B-001 | Fixed. Request input stops waiting once the engine is stopping, including input that keeps trickling in. | `tests/test_engine_lifecycle.py` (stalled and trickling clients) |
| C-01 | Fixed. The E1 fixture refuses while any position lock exists or when its preview conflicts with a protected orbit. | `tests/test_fixture_respects_locks.py` |
| C-02 | Fixed. The orbit, the workspace and the caller's follow-up fields (Next activation, focus) are stored in one write. | `tests/test_orbit_switch_atomic.py` |
| C-03 | Engine side fixed: a committed command whose snapshot fails reports success with a refresh request. The native host's durable receipt is a separate packet gated on the native regression. | `tests/test_native_reply_after_commit.py` |
| F-01 | Fixed (Codex). The four protection commands join the forecast restriction list used by both guards. | `tests/test_forecast_guard.py` |
| B-002 | Fixed (Codex). A build is reused only when its inputs were unchanged after compilation. | `tests/test_native_launch_reuse.py` |

X-01 and B-003 belong to the step-1 lane. Codex implementation worktrees inherit the same line-ending conversion, so fix packets with model-dependent acceptance checks wait until X-01 lands. Each other confirmed finding becomes a fix packet whose acceptance check is its experiment, turned into a regression test.
