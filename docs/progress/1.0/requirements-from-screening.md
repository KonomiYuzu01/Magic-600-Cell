# 1.0 requirements from the 0.4 screening

Status: **draft input to stage 2.2 and 2.5**. Source: [0.4.1 screening findings](../0.4.1/screening-findings.md). With no 0.4.1 release (owner decision, 1 October 2026), the confirmed 0.4 defects stop being fix packets. Each one names a class of failure that 1.0 must rule out by design and prove with a test. The 0.4 regression named in the last column is the starting point for the 1.0 test.

1.0 does not inherit the 0.4 runtime. These are requirements on the new design, not on 0.4 code.

## From confirmed findings

| Req. | From | Requirement for 1.0 | Proof |
|---|---|---|---|
| R-01 | A-02 | Every limit on what the program writes (logs, exports, packages) is at most the limit of what it reads. One shared limits table is used by both directions. | A round-trip test at each limit and one step above it; port of `tests/test_log_budget.py` |
| R-02 | C-03 | A committed command is never reported as failed. Commit outcome and display refresh are reported separately; a refresh failure after a commit shows "applied, display out of date" and never invites a retry. | Injected refresh failure after commit; port of `tests/test_native_reply_after_commit.py` and `tests/test_experiment_send_outcome.py` |
| R-03 | X-01 | Hashed assets and proof sources load identically from any checkout: line endings are pinned in the repository and asset hashes are taken over the pinned bytes. | A CRLF checkout fixture that must load the model; the step-1 checkout-bytes check |
| R-04 | B-001 | Shutdown completes in order within its grace period whatever clients do; idle or slow connections never hold it. | Stalled and trickling client fixtures; port of the `tests/test_engine_lifecycle.py` cases |
| R-05 | F-01 | Every command that changes protection goes through one permission check; there is no second path (button, key, script) that skips it. The command table, not each view, declares which commands a restriction blocks. | A test that enumerates every command entry point against the restriction table; port of `tests/test_forecast_guard.py` |
| R-06 | C-02 | State that must agree (orbit, workspace, follow-up fields) is written in one transaction. A failed write leaves all of it as before. | Injected save failures at each write; port of `tests/test_orbit_switch_atomic.py` |
| R-07 | C-01 | No fixture, tutorial, script or tool commits outside the normal commit path; protection and locks apply to every commit. | Enumerate commit call sites; a locked-slot fixture per non-user path; port of `tests/test_fixture_respects_locks.py` |
| R-08 | B-002 | A build artefact is reused only when its recorded inputs equal the inputs it was built from, checked after the build. | Inputs changed during a build must not be reused; port of `tests/test_native_launch_reuse.py` |
| R-09 | B-003 | A clean checkout builds and tests without files outside the repository. | Clean-checkout CI job on a fresh clone |
| R-10 | A-01 | A preview owns an immutable copy of its witness; later changes by the caller cannot change what is committed or journalled. | Mutate the caller's list after preview; replay must equal the committed state |
| R-11 | A-03 | Every word that validation accepts is accepted by every operation on words (inverse, conjugate, commutator, composition). One normal form is used before any operation. | Property test over validated words, including unsigned star steps |
| R-12 | C-04 | Provenance (recorded scramble, manual, assisted) survives rejected input; only a commit changes it. | Rejected input followed by a preview keeps the provenance |
| R-13 | X-02 | The cost of status after commit, undo and redo does not grow with history length. Status reads only indexed summary data, never history blobs. | Timing fixture at 0, 50, 200 and 400 checkpoints with a flat bound |

## From leads that still need verification

These were not confirmed in 0.4. In 1.0 they are design checks, settled in 2.3 and tested once the component exists.

| Req. | From | Design check for 1.0 |
|---|---|---|
| R-14 | E-02 | No synchronous engine call on the UI thread; a failed or slow lookup shows an error in place and never reaches the application-exit path. |
| R-15 | B-005 | No response is written while the session lock is held. |
| R-16 | B-004 | Job admission is atomic; a one-job limit cannot be bypassed by concurrent submissions. |
| R-17 | E-01 | Device loss during any input leaves no publication or input state held; the renderer recovers or reports. |
| R-18 | F-02, F-03 | Key repeat never repeats a state-changing command; key routing is defined in the keyboard model (G6), not per control. |
| R-19 | E-03, E-04, F-04, F-05, D-03 | Views do work only when visible and only for changed data; caches are checked before rebuilds. Verified with the renderer gate captures, not by separate fixes. |

## Use

- 2.2: each requirement attaches to the inventory items it constrains.
- 2.4: R-02, R-04, R-14 and R-17 are checked in the renderer candidates' interop design.
- 2.5: the architecture freeze lists each requirement with its owning component and test.
