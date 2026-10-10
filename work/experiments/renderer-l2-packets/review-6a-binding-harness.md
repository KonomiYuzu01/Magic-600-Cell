# Review packet, shard A: the L2-V-002 guard, finalizer and runner

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: one full review of the harness side of the L2-V-002 candidate on branch `claude/renderer-l2-followup`. The plan is `PLAN-L2-V-002.md` (Astra plan check passed; sections 1 to 3, 6, 7 and 9 apply here). The parts, all in `work/experiments/`:
  1. `renderer-l2/file_guard.py` and `renderer-l2/check_guard.py`, new. Codex wrote them in implement call 20261009T165241Z-23345f6b (G1). Claude reviewed them and added the fixes listed in section 5.
  2. `renderer-l2/finalize_run.py`, `renderer-l2/check_l2.py` (outside `static_runner()`) and `renderer-l2-packets/HARNESS.md`. Codex wrote the changes in implement call 20261010T024831Z-0d4dc59d (G4), and Claude reviewed them with no change.
  3. `renderer-l2/run_scene.ps1` and `static_runner()` in `check_l2.py`, written by Claude (G5).
  4. `renderer-l2/guard_experiments.py`, written by Claude: the synthetic experiments of plan section 7, item 1, and their results in `renderer-l2/RESULT.md`.
  - Diff: `git diff 30b4fac HEAD -- work/experiments/renderer-l2 work/experiments/renderer-l2-packets/HARNESS.md`. `30b4fac` is the plan commit. The merge commit `d0e028c` brought unrelated main work and is out of scope.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`.
  - Report only `blocker` or `major` findings, each with a concrete counterexample: a file system state, a sequence of file operations, a record, or a run that gives the wrong outcome.
  - The verdict is `pass` if there is no such finding.
- Questions:
  1. Guard: can it print `ready` while an identity file or one of its ancestor folders can still be written, truncated, deleted, renamed or replaced? Can a `guard.json` digest differ from the bytes that the held handle reads? Consider each alias form: a junction, a symbolic link, a case-sensitive folder, an 8.3 name, `subst`, UNC, `\\?\`, `..`, a relative path, and two spellings with one case-fold key.
  2. Release: does release check every held file ID and the reparse attribute again? Does every input other than one `release` line, and every end of file, give a non-zero exit?
  3. Finalizer: can it write a record (exit 0 or 2) when any check of plan section 3 fails? The checks are:
     - a dead or foreign guard;
     - a write probe other than a sharing violation;
     - an app created before `ready`;
     - an identity part that is not guarded, or whose file ID or digest differs;
     - a wrong MVID;
     - a Qt `qt:` key set that differs from the observer union;
     - an unload with no earlier load.

     Can it refuse a run that meets the contract? Does the same set of bytes give the same identity as before this change?
  4. Runner: is the order guard `ready`, then `Start-App`, then `--launched-created` from the open process handle, then the finalizer, then release? Does every failure stop the series with no record? Does `finally` stop a guard that is still running? Can a coarse process creation time make a valid run fail the order check, or let an invalid run pass it?
  5. Does `HARNESS.md` agree with the code? Does it state the limit: modules outside `scope` are neither recorded nor bound?
  6. Do the experiment results in `RESULT.md` follow from `guard_experiments.py`'s checks? Does any sentence claim more than was measured?
- Out of scope:
  - the apps (`renderer-sd`, `renderer-sa2`; shard B reviews them);
  - `tools/perf/renderer_gate.py`, S-B's runner, the level 1 files and the 4 October records;
  - hostile-machine attestation, which the plan's threat model excludes (plan section 1);
  - performance claims;
  - findings below `major`.

## 2. Actual problem and reproduction
No failure is known. Astra's ruling `20261009T155501Z-a2cb4438` found that the level 2 identity hashed files after the run by path, so its digests were not bound to the bytes the app loaded (L2-V-002 and -02 to -05). This candidate implements the approved fix.

## 3. Environment and versions
- Branch `claude/renderer-l2-followup`, at the commit that adds this packet.
- Windows 11 (build 26200), NTFS system volume, CPython 3.14 (standard library only), Windows PowerShell 5.1.
- Evidence available:
  - Source: `python -B work/experiments/renderer-l2/check_guard.py` (33 passed; 1 not run: creating a symbolic link needs a privilege this account lacks) and `python -B work/experiments/renderer-l2/check_l2.py` (337 finalizer cases, 128 of them `identity` refusals; the runner static check and the PowerShell parser).
  - Actual Windows, no GPU and no PresentMon: `guard_experiments.py` on Qt build `20261010T024533Z` (results in `RESULT.md`).
  - No app run with a window, the GPU or PresentMon has used this candidate yet.

## 4. Necessary source and evidence
- Plan: `work/experiments/renderer-l2-packets/PLAN-L2-V-002.md`.
- Packets: `G1-guard.md`, `G4-finalizer.md`.
- Contract: `HARNESS.md` sections 6 to 10.
- Gate: `tools/perf/renderer_gate.py`, `validate_run` (ignores unknown keys).

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | G1 output as written | implement call 20261009T165241Z-23345f6b | wrapper acceptance check | valid |
| 2 | Python case folding is not the NTFS upcase table (`straße.dll` and `strasse.dll` fold equal and are two files), so one guard entry could stand for two files | the guard refuses two spellings with one case-fold key; exact duplicates are merged | `check_guard.py` `alias_spellings` | fixed |
| 3 | Nothing tested that acquisition hashes through its own handle | a test denies read access to the path after the open, and acquisition must still hash | `check_guard.py` `acquire_hash_after_denial` | fixed |
| 4 | Windows PowerShell 5.1 may send a UTF-8 byte-order mark before the first line, which the ANSI code page does not decode, so release could fail | the guard reads stdin as bytes and strips the mark; the runner writes raw ASCII bytes | `check_guard.py` `cli_bom_release`; `check_l2.py` pins the runner | fixed |
| 5 | G4 output as written | implement call 20261010T024831Z-0d4dc59d | wrapper acceptance check; Claude's review, including the Qt record keys against `l2.cpp` | valid, 337 cases, no change |
| 6 | The loader might refuse DLLs that the guard holds with read-only sharing, so every attended run would fail at launch | experiment: the Qt usage path with the deployment held | `guard_experiments.py` | the app loaded its imports and recorded 11 guarded modules |

## 6. Constraints and owned files
- Read-only review. No file changes.
- Invariants:
  - synthetic labels only;
  - claims only for the measured build identity;
  - the finalizer alone writes records;
  - never commit raw PresentMon output or machine diagnostics.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
