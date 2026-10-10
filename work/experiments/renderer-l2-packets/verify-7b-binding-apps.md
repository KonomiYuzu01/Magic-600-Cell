# Scoped verification packet, shard B: L2-V-002-B01

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: scoped verification round 1 of the L2-V-002 candidate on branch `claude/renderer-l2-followup`, app side. Shard B's full review (`20261010T040536Z-3c690ebe`) found one major finding. It was adopted and fixed in commit `823b3ab`. The commit that adds this packet adds two finalizer fixtures that refuse the spellings the observer now records.
- Diff: `git diff 635db95 HEAD -- work/experiments/renderer-sd work/experiments/renderer-l2/check_l2.py work/experiments/renderer-l2-packets/HARNESS.md`. `635db95` is the reviewed candidate.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. Answer only two questions:
  1. Is L2-V-002-B01 fixed? Say so in the summary, with the code and the check that show it.
  2. Does the fix introduce a new `blocker` or `major` finding? Report one only with a concrete counterexample: a load sequence, a path spelling, a record or a run that gives the wrong outcome. A finding about loader-lock safety (allocation, file I/O, locks or CRT calls in the callback path) counts.
  - The verdict is `pass` if B01 is fixed and there is no new `blocker` or `major`.
- Out of scope:
  - code the fix does not touch, and earlier findings other than B01;
  - the harness side (shard A);
  - spellings the fix does not claim to cover: a subst drive, a junction or symbolic link outside scope, a UNC share and a hard link outside scope. `HARNESS.md` section 6 states them as a limit of the level 2 evidence. The guard holds every guarded file with read sharing only, so these spellings cannot change guarded bytes;
  - hostile-machine attestation (plan section 1), and performance claims;
  - findings below `major`.

## 2. Actual problem and reproduction
B01, as reported: `inScope` compared the loader's spelling directly with the executable directory prefix. A load of `\\?\C:\build\deploy\sd_module_probe.dll` was treated as out of scope: the observer did not record it before the seal, and a load after the seal did not end the process.

## 3. Environment and versions
- Windows 11 (build 26200), NTFS with 8.3 names on the system volume, MSVC 19.51, Qt 6.10.3 msvc2022_64.
- Evidence for the fixed candidate (owner's machine, no window, GPU or PresentMon):
  - Qt build `20261010T042228Z` from commit `823b3ab`'s sources: `sd_module_observer_test.exe` gives 10 of 10 cases with 210 children (case 6: 30 of 200 exited 3 at the seal). Case 10 ran, because this path has 8.3 names;
  - `guard_experiments.py` on that build: 9 passed, 1 not run, 0 failed. This includes the Qt usage path with all 33 deployment files held, where the app loaded its imports and recorded 11 guarded modules;
  - `python -B work/experiments/renderer-sd/check_project.py`: ok, with three new planted defects that it catches;
  - `python -B work/experiments/renderer-l2/check_l2.py`: 348 cases, including `binding-modules-events-extended` and `binding-modules-events-short-name`, both exactly `identity`.

## 4. Necessary source and evidence
- `work/experiments/renderer-sd/app/src/module_observer.cpp`:
  - `Component` and the `State` fields;
  - `scopeText`, `devicePrefix`, `scopeComponent`, `aliasInScope` and `inScope`;
  - `readComponents`, called in `start()` before registration;
  - the callback, unchanged, which calls `inScope`.
- `work/experiments/renderer-sd/app/src/module_observer_test.cpp`, cases 8 to 10 and the pass rule.
- `work/experiments/renderer-sd/check_project.py`: the alias pins, the no-allocation and no-I/O regex (now forbidding `Find*File*` and `Get*PathName*` in the alias helpers), the test tokens and the planted defects.
- `work/experiments/renderer-l2/finalize_run.py`, `path_key`: it requires the `X:\` form and a path under scope plus a backslash, so a recorded `\\?\` or 8.3 spelling refuses.
- `work/experiments/renderer-sd/README.md` (G2 section) and `work/experiments/renderer-l2-packets/HARNESS.md` section 6.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | The loader keeps `\\?\`, `\??\` and `\\.\` prefixes and 8.3 component names in `FullDllName`, so prefix matching misses them | before registration, read each scope directory's long and 8.3 names with `FindFirstFileW`; in the callback, compare a device-prefixed or component-wise spelling against these fixed copies, with no allocation or I/O | observer cases 8 (`\\?\` load in the union), 9 (exit 3 after the seal) and 10 (8.3 load in the union once) | pass |
| 2 | A recorded alias spelling must not enter an accepted identity | none: `path_key` already refuses it | the new fixtures `events-extended` and `events-short-name` | `identity` |

## 6. Constraints and owned files
- Read-only review. No file changes.
- Invariants: synthetic labels only; claims only for the measured build identity; nothing under the loader lock may allocate, perform file I/O or call Qt or CRT locale functions.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
