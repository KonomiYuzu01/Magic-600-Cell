# Clean-checkout test audit (0.4.1 step 1 input)

Read-only audit of which checks in `tests/` run from a clean public checkout. Prepared on 2026-09-30 in a Linux cloud session (Python 3.11, no NumPy) for the step-1 session. It changes no test file; the step-1 lane owns the fixes. It does not overlap with the unmerged step-1 branch `claude/0-4-1-build-id-test-harness-e7ad4e` (`.gitattributes`, `tools/checkout_bytes.py`, `tests/test_checkout_bytes.py`).

Evidence class: source inspection plus headless Linux runs. Nothing here is Windows/DirectX evidence.

## Findings

| ID | Severity | Evidence | Problem | Suggested fix |
|---|---|---|---|---|
| CC-1 | major | `tests/test_native_startup_contract.py:36`, `native/NativeHost.cs:418` | The test asserts `'api.Get("native/snapshot")' in host`, but the host calls `native/snapshot?protocol=2`. The assertion is stale and fails on every platform. | Assert the current call (or a pattern that accepts the protocol query) and keep the protocol version pinned in one place. |
| CC-2 | major | `tests/test_debug_cleanup.py:16` | Writes evidence into `tests/v022/`. `.gitignore` ignores `tests/v0*/`, so the directory is absent from a clean checkout and the test fails with `FileNotFoundError`. | Create the directory first, or write to a temporary directory and copy evidence out on request. |
| CC-3 | minor | `tests/test_full_reference_replay.py:25` (`tests/v021`), `tests/test_native_snapshot.py:71` (`tests/v022`), `tests/test_browser_ui.py:37` (`tests/v02`) | Same pattern as CC-2; not reproduced here because these tests stop earlier on NumPy or Playwright. | Same fix as CC-2. `test_engine_lifecycle.py:14` and `test_native_startup_contract.py:48` already create their directories and can serve as the model. |
| CC-4 | minor | `tests/test_http_boundary.py`, `tests/test_native_auxiliary_controls.py`, `tests/test_portable_package.py` | These need command-line arguments (a running engine or a built package). They are harness tools, not standalone tests, but their names suggest otherwise. | Document them in `docs/DEVELOPMENT.md` as harness tools with their arguments, or move them under a harness directory; exclude them from any "run all tests" loop. |
| CC-5 | info | 18 test files import NumPy | Not runnable in this cloud session. Expected: the engine requires NumPy and the approved engine environment is Windows-only (`bootstrap.py install engine-python` refuses on Linux). | Run on the owner's Windows machine with the engine environment. |
| CC-6 | info | `tests/test_browser_ui.py` | Needs Playwright. | Run where the browser harness is installed; record it as optional. |
| CC-7 | major | `docs/DEVELOPMENT.md`, `native_launch.py` | `tests/run_postapproval.py` is hashed by `native_launch.py` but absent from the public checkout (known gap, briefing section 4). | Owner recovers the file from the retained 0.4 working folder, or step 1 removes the hash dependency with a recorded provenance note. |
| CC-8 | minor | `tests/test_codex_implement.py` | Uses `shutil.rmtree(onexc=...)`, which needs Python 3.12. Fails on 3.11 (also on `main`). The approved development interpreter is 3.12+, so this matters only for cloud sessions. | Either state the 3.12 minimum at the top of the test or fall back to `onerror` on 3.11. Tooling path: needs the agent-rules review. |

## Acceptance check for the step-1 fix

From a fresh clone on the owner's Windows machine, with only the approved engine environment:

1. `python tests/test_core.py`, `python tests/test_reference_maps.py`, `python tests/test_crash.py`, `python tests/test_engine_lifecycle.py` pass.
2. Every other file in `tests/` that is not a documented harness tool runs without a missing-directory error.
3. `git status --porcelain` is empty after the run (evidence goes to ignored paths only).
4. The native build either finds `tests/run_postapproval.py` or no longer hashes it.

## Out of scope

Test semantics, performance baselines and native regressions (`tests/native/NativeHostRegression.cs`).
