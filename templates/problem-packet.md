# Problem packet

<!-- Seven parts. Remove credentials, private paths and personal data. Keep raw error text, the relevant code and the failed results. Do not paste whole chat transcripts. -->

## 1. Goal and acceptance
- Goal:
- Acceptance check (command or observable result):
- Non-goals:

## 2. Actual problem and reproduction
- Observed behaviour (raw error text):
- Steps or command to reproduce:
- Expected behaviour:

## 3. Environment and versions
- Base commit / branch:
- OS, Python, relevant tools and versions:
- Evidence kind available here: source/fixture | synthetic geometry | actual Windows/DirectX | performance

## 4. Necessary source and evidence
- Files and symbols (`path:line`):
- Minimal excerpts or diffs:
- Test output:

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|

## 6. Constraints and owned files
- Invariants that must hold (see AGENTS.md):
- Files the reader may change (only with an assigned worktree):
- Files that must not change:
- Implementation packet (`--kind implement`) only: exactly one contract block. `allowed_files` are repository-relative globs (`*` also matches `/`); `acceptance_check` is an argv list run without a shell in the worktree; the run is refused without all three fields.

```implement-contract
{"allowed_files": ["path/to/file.py"], "acceptance_check": ["python", "tests/test_x.py"], "stop_condition": "the acceptance check passes"}
```

## 7. Required return format
- Plan check or review: JSON matching `schemas/review-result.schema.json`.
- Solver: diagnosis, alternatives, experiment, fix outline, confidence.
- Implementation: changes only in the assigned worktree; final message lists the changed files, the acceptance result and open points.
- Review only; do not perform follow-up work.
