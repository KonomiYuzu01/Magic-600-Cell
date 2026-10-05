# Escalation packet: Astra ruling on the remaining part of S-B probe finding SB1-R1

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: a senior ruling, ordered by the owner, on the one finding still blocking after the probe candidate used its full review and both scoped verification rounds (AGENTS.md "Review rounds"). The ruling decides:
  - whether the remaining part of SB1-R1 must block S-B gate captures;
  - if it must, the smallest fix that closes it without creating a new false-refusal or evidence defect.
- The finding, as last stated by verification round 2 (`20261003T031824Z-3366dd48`):
  - `covered()` relies only on `WindowFromPoint`, which skips disabled windows.
  - An opaque, disabled, topmost popup shown with `SWP_NOACTIVATE`, covering all but a narrow edge strip, keeps the probe:
    - visible, uncloaked and not minimised;
    - in the foreground;
    - uncovered at the five hit-test points.
  - The strip still yields a displayed present every second, so `run_scene.ps1` admits the run.
  - This sequence is inferred from the source and API documentation, not reproduced.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`.
  - Keep the ID `SB1-R1` for the ruling on this finding, with the severity you rule (`major` keeps it blocking; `minor` or `nit` releases it). Give the reasoning, and if it stays blocking, the concrete fix and the experiment that would verify it.
  - Verdict `pass` if SB1-R1 no longer blocks and you find no other `blocker` or `major` in the changed files.
  - Report other issues only if they are a `blocker` or `major` in the files of section 4.

## 2. Actual problem and reproduction
- Gate context:
  - S-B is the bare Direct3D 12 probe for the stage 2.4 renderer gate (`docs/wiki/decisions/renderer-candidates.md`): all 259,800 sticker slots at full detail, average at least 30 fps and p99 at most 33.3 ms over [T0+10 s, T0+190 s), on each of three cold runs and on the pooled frames, on the owner's RTX 4070 Laptop GPU.
  - The committed evidence is only the sanitised summary of `tools/perf/renderer_gate.py`.
  - The captures run on the owner's own machine, started by the owner in an administrator PowerShell while the owner watches the screen. Other agent sessions are asked to stop builds and Codex implement calls during captures.
  - The question is whether an accidental non-representative run can become gate evidence, versus a deliberate evasion by a local program.
- History of SB1-R1:
  1. Full review `20261003T025224Z-7c1d70c4`: the occlusion counter was useless for a flip-model swap chain, which never returns `DXGI_STATUS_OCCLUDED`, so a hidden window could pass. Adopted.
  2. Round 1 fix:
     - per-100-ms samples of `IsWindowVisible`, `IsIconic` and `DWMWA_CLOAKED`;
     - probe exit 3 when any sample is not visible;
     - the capture script refused a swap chain with no displayed present.
     
     Verification round 1 (`20261003T030940Z-7fcac351`) found two gaps: the `Dropped` check spanned the whole capture, so displayed preroll frames could mask an all-dropped interval; and an opaque topmost window covering the probe was not detected.
  3. Round 2 fix:
     - `WindowFromPoint` hit tests at the centre and four inner points (10% inset), with `GetAncestor(hit, GA_ROOT)` required to be the probe;
     - foreground loss also exits 3;
     - the capture script refuses any one-second bin of the gate interval [T0+10 s, min(T0+190 s, trace stop)) without a `Dropped` = 0 present.
     
     Verification round 2 found the disabled-overlay bypass above.
- Alternatives the integrator considered, not implemented:
  - (a) Accept it as a documented limitation; the owner attends every capture.
  - (b) Walk the z-order above the probe (`GetWindow(hwnd, GW_HWNDPREV)` repeatedly) and treat any visible, uncloaked window whose rectangle intersects the probe as covering, enabled or not. Concern: always-present transparent overlays (layered, click-through, full-screen, for example a vendor overlay) would then refuse every normal run. Layered windows' visual opacity cannot be read reliably (per-pixel alpha).
  - (c) Something else you rule better, for example DWM or PresentMon evidence of the window's composition (present mode `Hardware: Independent Flip` throughout the interval), with its own false-refusal risks.

## 3. Environment and versions
- Base: branch `claude/renderer-sb` HEAD plus the uncommitted working-tree fixes, which are the current candidate.
- Windows 11 (10.0.26200).
- Owner GPU: NVIDIA GeForce RTX 4070 Laptop GPU, driver 32.0.16.1692. The owner set discrete-only (MUX) mode, and the probe measured that the NVIDIA adapter drives the display. Mains power; Windows power mode measured `max_performance`; 2560 x 1600 at 60 Hz.
- PresentMon 2.6.0.0, run with `--v1_metrics --qpc_time --terminate_on_proc_exit`.
- Evidence kind in the review sandbox: source/fixture only.

## 4. Necessary source and evidence
- Changed files: `work/experiments/renderer-sb/probe/src/gpu.cpp` (`covered`, `sampleConditions`, `environment`, the exit-code logic at the end of `runGpu`), `run_scene.ps1` (the displayed-present block), `README.md`, `src/probe.cpp`, `CMakeLists.txt`.
- Readers: `tools/perf/renderer_gate.py` (`interval_ticks`, `read_frames`, `condition_reasons`), `tools/perf/b412_summary.py` (`public_environment`).
- Authority: `docs/progress/1.0/renderer-experiment-plan.md` sections 1, 2 and 5.
- Prior results: `work/reviews/20261003T025224Z-7c1d70c4/review.json`, `work/reviews/20261003T030940Z-7fcac351/review.json`, `work/reviews/20261003T031824Z-3366dd48/review.json`, if readable.
- Integrator's local results on the owner's machine. These are actual Windows/DirectX functional results, not performance evidence; the probe was launched without PresentMon.

  | Run (W3) | Exit | Samples | Not visible | Covered | Not foreground | Label check |
  |---|---|---|---|---|---|---|
  | 8 s, normal | 0 | 80 | 0 | 0 | 0 | pass |
  | 10 s, `ShowWindow(SW_MINIMIZE)` after 7 s | 3 | 100 | 53 | — | 53 | — |
  | 10 s, opaque topmost enabled WinForms form over the primary screen for 2 s | 3 | 100 | 20 | 20 | 0 | — |
  | 8 s, each of four label-fault injections | 2 | — | — | — | — | fail |

  - The displayed-present block of `run_scene.ps1`, extracted verbatim, on synthetic CSVs:

  | Case | Result |
  |---|---|
  | all displayed | accepted |
  | preroll displayed, interval all dropped | refused (180 of 180 seconds blind) |
  | one second dropped | refused |
  | one in ten displayed | accepted |

  - No PresentMon capture has been run with this build yet.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | per-sample visibility and a capture-wide displayed check close SB1-R1 | round 1 fixes | verification round 1 | partly; capture-wide window and coverage gaps |
| 2 | interval bins, hit-test coverage and a foreground requirement close it | round 2 fixes | verification round 2 | partly; disabled non-activating overlay bypass (inferred) |

## 6. Constraints and owned files
- Read-only ruling.
- Out of scope:
  - changes to `renderer_gate.py` or `b412_summary.py`;
  - performance numbers;
  - the handoff test;
  - style;
  - minor findings.
- Invariants:
  - full model;
  - synthetic labels;
  - no NVIDIA-specific features;
  - the probe may not alter mechanical state.
- Questions:
  1. Under the stated threat model (owner-attended captures; accidental versus deliberate), should the remaining part of SB1-R1 block gate captures?
  2. If yes:
     - What is the smallest sound fix?
     - Is (b) acceptable given transparent overlays?
     - Would (c) or another signal be better?
     - What experiment should verify it on the owner's machine?
  3. If no, what must the README state as the limitation, and must the gate summary or run record carry anything for it?

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
