# Scoped verification packet, shard A: L2-A-001 to L2-A-003

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: scoped verification round 1 of the L2-V-002 candidate on branch `claude/renderer-l2-followup`, harness side. Shard A's full review (`20261010T040536Z-8b6a90b5`) found three major findings. All three were adopted and fixed in commit `823b3ab`; the commit that adds this packet adds two fixtures to `check_l2.py` for shard B.
- Diff: `git diff 635db95 HEAD -- work/experiments/renderer-l2 work/experiments/renderer-l2-packets/HARNESS.md`. `635db95` is the reviewed candidate.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. Answer only two questions:
  1. Is each of L2-A-001, L2-A-002 and L2-A-003 fixed? For each one, say so in the summary, with the code and the check that show it.
  2. Does the fix introduce a new `blocker` or `major` finding? Report one only with a concrete counterexample: a file system state, a sequence of operations, a record or a run that gives the wrong outcome.
  - The verdict is `pass` if all three are fixed and there is no new `blocker` or `major`.
- Out of scope:
  - code the fix does not touch, and earlier findings other than these three;
  - the apps (`renderer-sd`, `renderer-sa2`; shard B);
  - hostile-machine attestation (plan section 1), and performance claims;
  - findings below `major`.

## 2. Actual problem and reproduction
The three findings, as reported:
- **L2-A-001** (major): if the guard gave no `ready` line within 60 s, `Start-Guard` threw without stopping it. The caller never received the guard, so the `finally` in the runner could not stop it, and it could keep holding files.
- **L2-A-002** (major): the finalizer bound load events through the held handle but looked unloads up only by Python case folding. An unload of a distinct NTFS file whose case folding equals a loaded one (`straße.dll` and `strasse.dll`) could pass.
- **L2-A-003** (major): the guard liveness and write-probe check ran before the later checks. A guard that died during those later checks could still leave an accepted record.

## 3. Environment and versions
- Windows 11 (build 26200), NTFS, CPython 3.14 (standard library only), Windows PowerShell 5.1.
- Evidence for the fixed candidate (owner's machine, no window, GPU or PresentMon):
  - `python -B work/experiments/renderer-l2/check_l2.py`: 348 finalizer cases, 139 of them `identity` refusals, plus the runner static check and the PowerShell parser;
  - `python -B work/experiments/renderer-l2/check_guard.py`: 33 passed, 1 not run (a symbolic link needs a privilege this account lacks);
  - `guard_experiments.py` on Qt build `20261010T042228Z`: 9 passed, 1 not run, 0 failed (`RESULT.md`).

## 4. Necessary source and evidence
- `work/experiments/renderer-l2/run_scene.ps1`: `Start-Guard`, its `try`/`catch` and the kill and dispose.
- `work/experiments/renderer-l2/finalize_run.py`: the Qt event loop (`guarded_digest` on every event path), `final_guard_check`, and its call as the last check before any output.
- `work/experiments/renderer-l2/check_l2.py`:
  - the `dies-late` loop (sa2 and sd, all four modes): the guard is killed after `build_identity` returns, and exactly `identity` is expected;
  - `binding-casefold-unload-id`;
  - the `static_runner` pin that `Start-Guard` kills the guard in its `catch` before it throws.
- `work/experiments/renderer-l2/guard_experiments.py`, `guard_cleanup`: a test guard holds a synthetic file and gives either no line or another line. `Start-Guard`, together with `Native-Arguments` and the runner's own `$python` line, is taken from `run_scene.ps1` and run in `powershell -NoProfile -Command`. Afterwards the guard process must be gone and the file writable.
- `work/experiments/renderer-l2-packets/HARNESS.md`, sections 7 and 9: the contract text for the three fixes.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | L2-A-001: the guard must be stopped where it is started | `try`/`catch` in `Start-Guard`: kill if still running, wait 5 s, dispose, rethrow | `guard_cleanup`, both cases; the runner committed before the fix (`635db95`) fails it with "the guard process outlived Start-Guard" | fixed |
| 2 | L2-A-002: bind every event path, unloads included | `guarded_digest(event['path'], guarded)` before the unload-or-load branch | `binding-casefold-unload-id`; with the old branch restored, `check_l2.py` fails on that fixture | fixed |
| 3 | L2-A-003: check the guard again right before writing | `final_guard_check` as the last `check('identity', ...)` before the outputs | eight `dies-late` fixtures; with the call removed, `check_l2.py` fails at `binding-sa2-run-dies-late` | fixed |

## 6. Constraints and owned files
- Read-only review. No file changes.
- Invariants: synthetic labels only; claims only for the measured build identity; the finalizer alone writes records; never commit raw PresentMon output or machine diagnostics.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
