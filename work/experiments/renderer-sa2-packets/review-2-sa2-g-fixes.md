# Review packet: Claude's fixes to the SA2-G Godot smoke-test harness

Review only; do not perform follow-up work.

`python tools/agents/codex_review.py --kind review --model gpt-6.1-sol --effort max --speed fast --packet work/experiments/renderer-sa2-packets/review-2-sa2-g-fixes.md`

## 1. Goal and acceptance
- Goal: review Claude-authored changes to the SA2 Godot smoke-test harness before it is committed and rerun on the owner's machine.
  - Codex wrote the harness under packet `SA2-G-godot.md` (implement call `20261003T152926Z-4116b612`).
  - Claude reviewed that output and found the defects F1 to F7 in section 2. A Claude subagent fixed them.
  - Claude also added `blank_control.py`, a control run for row R10.
  - This is non-critical tooling, so it gets this one Sol review.
- Acceptance: one JSON result matching `schemas/review-result.schema.json` that answers three questions:
  1. Does each fix do what section 2 asks, without departing from the packet's pass rule (`SA2-G-godot.md`, "A run passes when:", line 268)?
  2. Is `blank_control.py` correct? It must use R10's engine flags with only the render-thread model varied, and publish counts only, with no private data.
  3. Is there a defect in the changed code that would make a judgement or the public summary wrong? That means a false pass, a false failure, a privacy leak in `results/`, or a run that cannot start. Give a concrete counterexample: a result JSON, an output line or a file state, then the wrong outcome.
- Verdict `pass` when nothing reaches `major`.

## 2. Actual problem and reproduction
The defects Claude found in the implement output:
- **F1 (major).** For pass rows, `judge()` required zero Godot `ERROR:` lines, a condition the pass rule does not have. In the owner-machine run, R10 (separate render thread) failed only because Godot 4.7.2 prints `ERROR: This function (finalize) can only be called from the render thread.` at shutdown.
  - Fix: the line counts are data, never a gate. A run with `ERROR:` lines gets the non-gating note `godot-output-errors`. The summary carries known notes and the per-run counts.
- **F2.** Two checks were missing:
  - A validation row did not prove that the debug layer was active. Fix: such a row now needs `device.debug_layer == 1` (reason `validation-not-active`).
  - R0 passed even when no check had status `pass`. Fix: R0 needs at least one `pass`, through the shared `selftest_status`.
- **F3.** The judge did not compare the result's `config` echo with the row. It also did not check the rebuild count, nor `frames.run` for recorded rows and R12.
  - Fix: `config_mismatches()` compares route, queue, handover (not for R1), barriers, frames, resize_every, gpu_validation, render_thread and render_thread_separate_observed, by exact JSON type and value (reason `config-mismatch`).
  - A pass row with `resize_every > 0` needs `resize.rebuilds == (frames - 1) // resize_every`.
  - Recorded rows and R12 need `frames.run == frames`.
- **F4.** Godot consumes `--gpu-validation` and `--render-thread`, so they never reach `OS.GetCmdlineArgs()`. As a result, every `config` reported both as false, and the validation-error check in `Smoke.cs` never fired.
  - Fix: the runner passes the required user arguments `--sa2-gpu-validation 0|1` and `--sa2-render-thread safe|separate`.
  - `_Ready` records the measured `render_thread_separate_observed = !RenderingServer.IsOnRenderThread()`.
- **F5.** The source identity's `git diff --quiet HEAD` did not exclude the directories that the file walk excludes (`results/` among them). `native/build*/` was excluded from neither.
  - Fix: one `excluded()` rule drives the walk and `git ls-files -z`, and `diff_pathspecs()` adds the matching `:(exclude)` and `:(exclude,glob)` pathspecs.
- **F6.** R0's public status read `missing`, because the self-test JSON has no `status` field. Fix: the summary derives it from the checks.
- **F7.** Three test gaps in `check_project.py`, each now closed with a planted-defect self-test:
  - (a) the R12 drain-injection scoping is now a function, `run_environment()`, which strips an inherited variable in any letter case;
  - (b) each delegate field must be bound from the export of its own name, with its declared type;
  - (c) `WriteResult();` must come between the pending phase and `sa2_drain`.
- **Added: `blank_control.py`.** The README had cited private control counts. It now points to this script.
  - The script runs a blank project (no harness, no DLL) with R10's engine flags, `--quit-after 600`, three times per render-thread model.
  - It counts Godot's `ERROR:` and `WARNING:` lines and the exact `finalize` line.
  - Raw output stays under `work/loop-memory/`. `--write-summary` writes counts only to `results/sa2-blank-control-summary.json`, after `scan_public`.
  - A run that timed out or did not report the expected adapter refuses the summary.

## 3. Environment and versions
- Windows 11, CPython 3.14.7 64-bit.
- Godot `4.7.2.stable.mono.official.ed1daf0bf`.
- .NET SDK 10.0.401, restoring offline from Godot's bundled NuGet packages.
- RTX 4070 Laptop GPU, NVIDIA driver 616.92.
- The review sandbox is read-only, without GPU, Godot or network.

## 4. Necessary source and evidence
- Delta: `work/experiments/renderer-sa2-packets/sa2-g-fix-delta.patch`. It runs from the reviewed implement output to the current files:

  | File | Before | Now (sha256) |
  |---|---|---|
  | `run_smoke.py` | `6f304a1c…` | `67ecc346433189d71baa5edeb80d531b05baecdcf55aa83ffa5aa1a8362b1baa` |
  | `smoke_summary.py` | `28f04026…` | `49da1a212f8fddad0195c21b5069b0d148a921d88ce3ae72d5a867a1b78946e1` |
  | `check_project.py` | `59e90734…` | `770435f14261f0dc718e9805431b2184648c40ad6b0e887cbf5b2e97b0645b4c` |
  | `README.md` | `c704ddb0…` | `d5665db479d18f7f1cf64be6e6d913d48346cc4ff2e320f3758264f1934af87c` |
  | `project/Arguments.cs` | `54e4cd99…` | `df7df76a6d04364dbf5fa6b4ce715ebfd95705518dab416c5e7e21f3f5965c76` |
  | `project/Smoke.cs` | `745e61d8…` | `43ed15baca539c4a2b45ead8069b351bbb58475b3d9332a6610da5d1093e6b25` |
  | `blank_control.py` | new | `b4bbd92359e9280aba3616a0df604c345389e6d709aa97b47927be824e13181f` |

- The files are under `work/experiments/renderer-sa2/`. `work/` is ignored and these files are not committed yet, so read them directly; `git diff` does not show them. The packet is `work/experiments/renderer-sa2-packets/SA2-G-godot.md`.
- Godot API: `RenderingServer.IsOnRenderThread()` is listed as `M:Godot.RenderingServer.IsOnRenderThread` in the installed `GodotSharp.xml`, and the offline build compiles against it. That it returns true on the main thread in the safe model and false with a separate render thread comes from Godot's source. It is confirmed only when the matrix is rerun.
- Checks run on the owner's machine:
  - `python work/experiments/renderer-sa2/check_project.py` passes ("settings, ABI, reference images, run matrix, judging, summary privacy, R10 control").
  - `run_smoke.py --dry-run` lists R0 to R12, and `--dry-run --device-loss` adds R13. Every Godot row passes both new arguments, and only R12 gets the injection variable.
  - Offline re-judge of the last real run (private results):
    - with the new judge, R0 is as expected, and R1 to R12 fail only with `config-mismatch`;
    - the mismatched fields are exactly the three that the old harness did not or could not report (`render_thread`, `render_thread_separate_observed`, and `gpu_validation`, which it always reported as false);
    - with those three fields filled from the row, all 13 runs are as expected; the note appears on R10 (one line) and on R6 (the expected refusal).
  - Offline `dotnet build`: 0 warnings, 0 errors.
  - A mutation run planted 23 defects across F1 to F7, and the checks caught all of them.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | SA2-G as implemented by Codex | implement call `20261003T152926Z-4116b612` | Claude review; owner-machine run R0 to R12 | R10 failed on a Godot shutdown line; F1 to F7 found |
| 2 | the fixes in section 2 and the R10 control | Claude-authored | the checks in section 4 | all pass; this review |

## 6. Constraints and owned files
- Read-only review. Do not start Godot, `dotnet` or GPU work, and do not run `run_smoke.py` or `blank_control.py` except `run_smoke.py --dry-run`.
- `check_project.py` creates a fixture directory in the system temp folder. If the sandbox refuses that, say so and judge from the source.
- In scope: the delta above and its effect on judgements and on the public summaries.
- Out of scope:
  - the native DLL under `native/`, which was reviewed separately (SA2-N);
  - the items deferred earlier: `Update(r => …)` lambdas under `_gate`, the C# CRC substring check, R1's handover argument, and the `--version` pipe timeout;
  - `minor` and `nit` findings, and style.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`, with finding IDs `SA2-G-F-01` and up.
- Review only; do not perform follow-up work.
