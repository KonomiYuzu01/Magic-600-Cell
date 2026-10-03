# Review packet: add the H-06 cost-table test to the Windows CI list

Review only; do not perform follow-up work.

`python tools/agents/codex_review.py --kind review --model gpt-6.1-sol --effort max --speed fast --packet work/experiments/renderer-sb-packets/review-ci-feature-costs.md`

## 1. Goal and acceptance
- Goal: the renderer branch `claude/renderer-sb` added `tests/test_feature_costs.py` (the H-06 feature cost table, reviewed and committed earlier). Main has since added a Windows CI job that runs a fixed list of headless checks (`tools/ci/run_headless.py`, `.github/workflows/windows-headless.yml`). After merging main into the branch, Claude added the new test to that list. This is non-critical tooling, so it gets this one Sol review.
- Acceptance: one JSON result matching `schemas/review-result.schema.json` that answers:
  1. Does `tests/test_feature_costs.py` fit the list's stated rule (`tools/ci/run_headless.py:22-23`: needs nothing beyond CPython and NumPy; not a harness script that takes a work directory, a URL or a native profile)?
  2. Would it pass on the CI runner as the workflow runs it: a fresh checkout with `fetch-depth: 0` and no ignored files (no `work/` build output, no `tools/.venv`, no `tools/qt`), CPython 3.14.7 x64 with only the wheels of `tools/python/engine.txt`, `PYTHONDONTWRITEBYTECODE=1`, `PYTHONUTF8=1`, the runner's 8.3 short temp path (`RUNNER~1`), run from the repository root? Name any file, environment variable, GPU, network or machine state it depends on that the runner lacks, with `path:line`.
  3. Could it leave files in the checkout or interfere with the checks that run after it?
- Verdict `pass` when nothing reaches `major`.
- Out of scope: the cost-table design and the test's assertions (reviewed in calls `20261003T112324Z-39182ced` and `20261003T131015Z-1f68c2db`), the rest of the CI list, and the workflow file.

## 2. Actual problem and reproduction
- Change (one line, `tools/ci/run_headless.py:59`):

```diff
     ["tests/test_renderer_tools.py"],
+    ["tests/test_feature_costs.py"],
     ["tests/test_migration_probes.py"],
```

- Without it the CI job does not run the H-06 cost-table test, although the list already runs `tests/test_renderer_gate.py`, which imports the same `tools/perf/renderer_gate.py`.

## 3. Environment and versions
- Base: the merge of `origin/main` (`4458b6c`) into `claude/renderer-sb`, commit `08dda06`, plus this one line.
- Local machine: Windows 11, CPython 3.14.7 64-bit, NumPy 2.3.5 (the pinned engine environment, the same wheels as `tools/python/engine.txt`).
- The review sandbox is read-only, without network.

## 4. Necessary source and evidence
- `tools/ci/run_headless.py` (the list, the environment it sets, the 1,200 s limit per check).
- `.github/workflows/windows-headless.yml` (runner, Python setup, checkout options).
- `tests/test_feature_costs.py`: `ROOT` at line 16, `sys.path` at line 18, the temporary directories under `ROOT` at lines 78-87 (created there, not in the system temp folder, with a Windows `os.mkdir` mode patch).
- `tools/perf/feature_costs.py`, `tools/perf/renderer_gate.py`.
- Local results (before the merge, at `f3c367d`): `tests/test_feature_costs.py` passed with the engine environment and `TEMP`/`TMP` set to an 8.3 short path (`RUNNER~1`), 17 tests in about 25 s. A run of the whole CI list on the merged head is in progress; Claude checks its result before commit.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | The test needs nothing the runner lacks | add it to the list | local run, engine environment, short temp path | pass (before the merge) |

## 6. Constraints and owned files
- Only `tools/ci/run_headless.py` changes. The workflow file and the other tests do not change.
- A CI pass is not Windows/DirectX, input, long-session or performance evidence.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
