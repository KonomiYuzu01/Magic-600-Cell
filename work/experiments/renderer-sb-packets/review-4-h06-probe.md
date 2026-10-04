# Review packet: H-06 features in the S-B probe (H6-P), shard A

Review only; do not perform follow-up work.

Run from the `claude/renderer-sb` checkout:
`python tools/agents/codex_review.py --kind review --model gpt-6.1-sol --effort max --speed fast --packet work/experiments/renderer-sb-packets/review-4-h06-probe.md`

## 1. Goal and acceptance
- Goal: one Sol review (fast tier, non-critical experiment code) of the H-06 visual features and the cost scene `w3f` in the S-B probe. Shard B (`review-4-h06-tools.md`) reviews the cost-table tools of the same candidate at the same time.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. Report only `blocker` or `major` findings. Each finding needs a concrete counterexample: the inputs or state, then the wrong outcome.
  - Wrong outcomes that count:
    - **A changed baseline.** With `--feature none`, the GPU or CPU work per frame differs from the probe at `HEAD`: root signature, pipeline state, shaders, resources, clear values, command sequence, frames in flight, label path, trace or camera.
    - **A wrong feature frame.** A feature run's frame fails to draw all 600 × 433 stickers at full detail with colours from the bound label buffer. Or it does not do what the H6-P packet defines for that feature. Transparency adds two cases:
      - a sticker drawn out of back-to-front order;
      - a triangle drawn twice or not at all.
    - **A wrong `w3f` state.** For a trace entry k, the drawn camera, turn index, phase, angle or label revision differs from what the trace records. Or the preroll advances a state.
    - **A weakened label check.** A preserved label copy is skipped, overwritten or recycled. Or the oracle comparison misses a copy, in any scene.
    - **A hazard.** A buffer or texture is written while earlier GPU work may still read it, or read before the work that writes it completes. This covers the sort buffers, the index list, the post-process targets, the readbacks and the two frames in flight.
    - **A false check.**
      - The effect check or the sort check passes although the trace frames do not draw the feature, or do not draw it as checked.
      - A check failure ends with anything but exit 4 and no `run.json`.
    - **A wrong record.** `run.json` or the trace misstates any of these: `feature`, `camera`, `turn_frames`, `cycle_frames`, `window.display_required`, `environment.msaa`, or a trace entry's `camera`. Or a feature run can be recorded as `none`.
    - **A build identity gap.** The identity omits a compiled shader, or accepts a missing file.
    - **A capture script defect.** `run_scene.ps1` drops the feature, names a run without its feature, accepts injection with a feature, or treats exit 4 as evidence.
    - **A false document.** A README or `report.md` statement that the code contradicts.
  - Verdict `pass` if there is none.

## 2. Actual problem and reproduction
- H-06 is in packet `docs/progress/1.0/packets/renderer/E-2.4-01-sb-probe.md`, acceptance item 5. Each visual feature must be switchable on its own, so that its frame-time cost can be measured at full detail.
- The specification is packet `work/experiments/renderer-sb-packets/H6-P-probe-features.md`, section 1, items 1 to 11.
- Codex implement call `20261003T101043Z-d14eea43` implemented it.
  - The run was valid, and its acceptance check passed.
  - Its report is `work/experiments/renderer-sb/probe/report.md`. That file is part of the candidate.
- Claude reviewed and applied the patch, then found one defect on the GPU: `--snapshot` stopped with "WIC did not accept RGBA" (exit 1), because the WIC PNG encoder does not take 32bppRGBA.
  - Integrator change: `png()` in `src/gpu.cpp` now swaps red and blue in a copy and writes 32bppBGRA.
  - Integrator change: the README's `feature_costs.py` command gains the required `--out` and the optional `--markdown`.
  - These are the only changes to the Codex patch.

## 3. Environment and versions
- Windows 11 (10.0.26200), MSVC 19.51.36260 x64, Windows SDK 10.0.26100.0 with its DXC, CMake 4.4.3, Ninja 1.13.2, C++20 and HLSL shader model 6.0.
- RTX 4070 Laptop GPU with NVIDIA driver 616.92, at 2560 x 1600 and 60 Hz, on mains power.
- The review sandbox is read-only and has no GPU. Evidence kind there: source only.

## 4. Necessary source and evidence
- Files under review, all in `work/experiments/renderer-sb/probe/`.
  - Changed against `HEAD` (`db97d5d`); `git diff HEAD -- work/experiments/renderer-sb/probe/` shows them:
    - `src/gpu.cpp`, `src/probe.cpp`, `src/probe.h`;
    - `CMakeLists.txt`, `check_probe.py`, `run_scene.ps1`, `README.md`.
  - New. They are untracked because `work/` is ignored, so read them directly:
    - `src/edges.cpp`;
    - `shaders/fog.hlsl`, `shaders/outlines.hlsl`, `shaders/transparency.hlsl`, `shaders/sort.hlsl`, `shaders/post.hlsl`;
    - `report.md`.
  - Unchanged, for reference: `shaders/draw.hlsl`, `shaders/geometry.hlsl` and `shaders/check.hlsl`.
- Integrator's checks on the final source:
  - **Build and CPU acceptance.**
    - `build.cmd` builds the probe; the only warnings are the two existing C++20 `u8path` deprecations.
    - `sb_probe.exe --selftest` prints `selftest: ok`, with edge counts features=3277, diagonals=5546, open=819, multiple=31, degenerate=3948 and duplicates=761.
    - `python work/experiments/renderer-sb/probe/check_probe.py` exits 0.
  - **GPU functional check.** These are 20 s runs, not measurements. The synthetic labels are those the label check uses.
    - One `w3f` run per variant, on build `9be08baa…`. That build differs from the final one only in `png()`, which these runs did not call.

      | Variant | Exit | Effect check (changed of 4,096,000 px) | Sort check | Label check | `w3f` entries, mismatches | Self-timed fps (information only) |
      |---|---|---|---|---|---|---|
      | none | 0 | not run (baseline) | — | pass | 15,713, 0 | 785.6 |
      | no-gaps | 0 | pass, 176,817 | — | pass | 14,582, 0 | 729.1 |
      | outlines | 0 | pass, 174,270 | — | pass | 12,083, 0 | 604.1 |
      | transparency | 0 | pass, 477,752 | pass: permutation of 259,800, back to front | pass | 7,695, 0 | 384.7 |
      | fog | 0 | pass, 529,959 | — | pass | 15,694, 0 | 784.7 |
      | dof | 0 | pass, 422,226 | — | pass | 11,460, 0 | 573.0 |
      | ao | 0 | pass, 362,893 | — | pass | 10,010, 0 | 500.5 |
      | msaa4 | 0 | pass, 81,315 | — | pass | 10,782, 0 | 539.1 |

    - Every run in that table:
      - recorded `window.visible_throughout`, `foreground_throughout` and `display_required` as true;
      - recorded the `camera` object of the H6-P packet, item 6a.
    - `environment.msaa` is 4 for `msaa4` and 1 for every other variant.
    - One W3 run per variant with `--snapshot`, on the final build `a5bd2b53…`:
      - every run exits 0, and the label check passes;
      - each effect check changes the same pixel count as its `w3f` run, and the transparency sort check passes again;
      - the window and camera records are as above, and `msaa4` records `environment.msaa` 4;
      - the PNG pairs exist. Claude inspected the outlines, transparency and AO images: dark edge lines, blended layers, and darkened creases.
    - Self-timed fps of those W3 runs, for information only: none 785.1, no-gaps 728.5, outlines 604.3, transparency 371.8, fog 766.2, dof 556.0, ao 488.8, msaa4 545.7.
  - **Baseline speed, information only.** Three interleaved pairs of 40 s W3 `none` runs compared the gated build `2b5bf5e6…` (old) with the final build (new), in the order old-new, new-old, old-new.
    - Pooled self-timed fps: old 760.5, new 757.2.
    - p99 per run: old 1.529, 1.592 and 1.601 ms; new 1.561, 1.586 and 1.603 ms.
    - Both builds slowed by about 5 % over the session, whichever was running.
- The private run records stay under `work/loop-memory/`, outside the review scope.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | a bare D3D12 draw meets the gate in W3 | E-2.4-01 probe | three owner-attended cold runs | met on build `2b5bf5e6…` |
| 2 | the H6-P packet is implementable as written | implement call `20261003T101043Z-d14eea43` | CPU acceptance check | pass |
| 3 | the integrated probe works on the GPU | none | the section 4 runs | every `w3f` run passes; W3 runs with `--snapshot` stop with exit 1 in the PNG writer |
| 4 | the PNG encoder needs 32bppBGRA | `png()` swaps to BGRA | W3 runs with `--snapshot` on the final build | all eight pass, with PNG pairs written |

## 6. Constraints and owned files
- Read-only review.
- Out of scope:
  - the cost-table tools, which are shard B;
  - `work/experiments/renderer-sb/RESULT.md`, the results folder, the handoff test and the wiki pages;
  - timing numbers, which are information only;
  - design choices that the H6-P packet records as decided, unless they cause a wrong outcome from section 1. These include:
    - the feature definitions and parameters;
    - the frame-stepped `w3f` scene;
    - the 2/255 and 0.1 % effect thresholds;
    - the bitonic sort;
  - style, `minor` and `nit` findings.
- Questions:
  1. **Baseline.** With `--feature none`, is every per-frame GPU command, barrier, allocation and CPU step the same as at `HEAD`?
  2. **`w3f` state.**
     - Does the render loop draw, for trace entry k, exactly the state the trace records?
     - Is the label path the same as W3's for that turn index?
     - Can the chunked `extraReadbacks` growth skip, overwrite or misplace a preserved copy? Can it break the after-run comparison?
  3. **Transparency.**
     - Is the bitonic network correct for 2^18 padded keys? Can a padding key sort into the first 259,800 ranks?
     - Does the prefix-sum and index pass draw every triangle exactly once? Sticker vertex counts are 12 to 252, and `indicesMain` runs 256 threads per sticker.
     - Are the UAV, copy and index-buffer barriers enough within a frame and across frames that share the sort buffers?
  4. **DoF and AO.**
     - Are the offscreen colour, readable depth and AO targets in the right state for every pass, every frame?
     - Is any target reused while a previous pass may still read it?
  5. **Effect and sort checks.**
     - Do the checks render through the same code path as the trace frames?
     - Can either check pass while the feature is absent from the trace frames?
     - Is MSAA's one-sample comparison frame equivalent to the baseline's output?
  6. **Records and the capture script.** Are the run records, the build identity and `run_scene.ps1` right in every scene and feature?

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`.
- Review only; do not perform follow-up work.
