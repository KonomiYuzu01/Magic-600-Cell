# Packet H6-P: the H-06 visual features in the S-B probe

Run from the `claude/renderer-sb` checkout:
`python tools/agents/codex_review.py --kind implement --model gpt-6.1-sol --effort max --timeout 7200 --packet work/experiments/renderer-sb-packets/H6-P-probe-features.md`

The parallel packet H6-T (`H6-T-cost-table.md`) owns the gate and table tools in `tools/perf/` and their tests. This packet owns only the probe.

## 1. Goal and acceptance
- Goal: each H-06 visual feature can be switched on alone in the S-B probe. The owner's machine can then measure the frame time each feature adds in the W3 scene at full detail.
  - The features come from packet E-2.4-01, acceptance item 5: sticker gaps, outlines, per-sticker transparency with sorting, fog, depth of field, ambient occlusion and MSAA 4x.
  - A feature that is not measured is listed as unmeasured, never estimated. A feature that changes nothing on screen would give a false cost, so every run proves its effect.
- Acceptance:
  1. **Options.**
     - `--feature <name>` accepts these names:
       - `none` is the default baseline;
       - `no-gaps`, `outlines`, `transparency`, `fog`, `dof`, `ao` and `msaa4`.
     - `--msaa 4` stays accepted, as a synonym for `--feature msaa4`.
     - These combinations are refused with an error and exit 1:
       - two features at once, including `--msaa 4` with any feature other than `none` or `msaa4`;
       - any feature other than `none` with `--geometry-check` or `--inject`.
     - `--scene w3f` selects the cost scene (item 6b).
       - `--turn-frames <T>` (default 157) and `--cycle-frames <n>` (default 3140) set its sequence.
       - T must be at least 2, and n a positive multiple of 2T.
       - Both options are refused outside `w3f`, and `--inject` is refused in `w3f`.
     - Every scene accepts a feature. The H-06 table takes its costs from `w3f` and its gate verdicts from W3.
  2. **The baseline is unchanged.** With `--feature none`, the probe does the same GPU work as the current build:
     - the same root signature, pipeline state, shaders, resources, clear values and per-frame command sequence;
     - two frames in flight;
     - the same label path and trace.

     `draw.hlsl`, `geometry.hlsl` and `check.hlsl` stay byte-identical. A feature adds its own shader files, root signatures, pipeline states, resources and passes, and creates them only when that feature runs. The label upload, binding, preserved copies and label check work the same in every feature run, and the colour of every sticker still comes from the bound label buffer.
  3. **Feature definitions.** All features draw all 600 x 433 stickers at full detail, and none reduces geometry.
     - `no-gaps`: sticker shrink 1.0 instead of 0.82. Everything else stays as in the baseline, with the same shaders.

       The gated look has gaps, so the table derives the cost of gaps as T(none) minus T(no-gaps).
     - `fog`: per-pixel exponential fog by view depth, towards the clear colour (0.13, 0.145, 0.16).
       - Choose the parameters so that the fog is clearly visible over the W3 depth range.
       - Its cost does not depend on the parameter values.
     - `outlines`: a dark outline about 1.5 px wide, anti-aliased, of constant screen width at every depth, along every sticker's feature edges.
       - A feature edge is an edge between non-coplanar triangles of the sticker, an open edge, or an edge of three or more triangles. Diagonals between coplanar triangles get no outline.
       - Suggested method, one pass:
         - compute a per-triangle edge mask for the base cell on the CPU at start-up;
         - give each vertex a barycentric weight from its position in its triangle (`SV_VertexID % 3`; the draw is a non-indexed triangle list);
         - in the pixel shader, darken by the distance to the nearest masked edge, scaled with `fwidth`.
       - Another method is allowed if `report.md` says why and it draws the same edges.
     - `transparency`: every sticker is drawn with alpha 0.6.
       - The stickers are sorted back to front at sticker granularity on the GPU every frame, all 259,800 of them.
       - Blending is standard alpha blending, with depth writes off.
       - Sort key: the view depth of the sticker's centre. Apply the `projectVertex` math to the centre: cell and sticker shrink, the cell frame, the turn rotation of animated slots, and the camera.
       - Sort algorithm: implementer's choice, for example a bitonic or radix sort padded to 2^18 keys.
       - The draw processes the stickers in sorted order and draws every triangle of every sticker exactly once, the degenerate ones included, as the baseline does.
       - Sticker vertex counts range from 12 to 252, with a mean of 70.4. A fixed per-sticker vertex count would therefore multiply the vertex work.
         - Suggested: build each frame's order with a prefix sum over the sorted stickers' vertex counts.
         - A compute pass then writes an index list of 18,288,000 indices, about 73 MB, drawn with one `DrawIndexedInstanced`.
         - The vertex shader decodes an index g into cell g / 30480 and vertex g % 30480.
     - `dof`: depth of field as a post-process at native resolution.
       - Render the scene into an offscreen colour target. Depth must be readable as a texture: `R32_TYPELESS` with a `D32_FLOAT` view for depth and an `R32_FLOAT` shader view, only in this feature.
       - The circle of confusion comes from the view depth. The focus is at the depth of the model's centre. The maximum radius is 8 px at a 1600 px backbuffer height, scaled with the height.
       - Blur with a CoC-weighted gather at full resolution: two separable passes, or a disc of at least 24 taps.
       - Composite into the back buffer.
     - `ao`: screen-space ambient occlusion from the depth buffer.
       - Use 16 samples per pixel at full resolution, with view-space positions reconstructed from depth by the projection in `geometry.hlsl`.
       - Apply a depth-aware blur in two passes.
       - A composite pass darkens the scene colour into the back buffer.
     - `msaa4`: the existing 4x MSAA path.
  4. **Effect check.** For every feature other than `none`, the probe renders one frame twice during the preroll, before the trace starts:
     - both renders use the same camera, labels and turn angle, once with the feature off and once on;
     - it reads both back and counts the pixels whose colour differs by more than 2/255 in any channel;
     - `run.json` records `feature_effect`: `{"changed_pixels": n, "pixels": total, "status": "pass"|"fail"}`;
     - the check passes when more than 0.1 % of the pixels changed.

     For `transparency`, the probe also reads back that frame's sorted keys and ids once. `run.json` records `sort_check`: the ids must form a permutation of 0..259,799, and the keys must be ordered back to front.

     If either check fails, the probe prints the reason and exits 4 before the trace starts, without writing `run.json`. For `msaa4`, the feature-off frame needs a one-sample pipeline and depth buffer, created only for the check. The check frames need not be presented.
  5. **Snapshots.** `--snapshot <directory>` writes the two effect-check frames as `feature-off.png` and `feature-on.png`, using the Windows Imaging Component. These are private inspection images and are never committed.
  6. **`run.json`.**
     - Every run carries a top-level `"feature"` key, with `"none"` for the baseline.
     - `environment.msaa` stays as it is.
     - The format stays `magic600-renderer-run-v1`; a missing `feature` key means `none`.
     - Exit codes stay 0, 1, 2 and 3; 4 is new.
  6a. **Camera record.** The W2/W3 camera stays as it is: one fixed rotation of 0.002 rad in the (0, 3) plane per rendered frame, preroll included, applied before the draw. The renderer experiment plan defines it that way: "one fixed rotation step per frame". W3's turns follow the clock. A slower feature therefore samples other states in the same interval, so W3 runs cannot give paired costs; the cost scene `w3f` (6b) does. The probe records the pose in every scene.
     - Every trace entry carries `camera`, an integer: the number of rotation steps applied before this frame's draw. In `w3f` the count starts again at each cycle reset. It is 0 in scenes without rotation.
     - `run.json` carries a top-level `"camera"` object.
       - In W2, W3 and `w3f`: `{"plane": [0, 3], "step_rad": 0.002, "per": "frame"}`.
       - In other scenes: `{"plane": null, "step_rad": 0, "per": "none"}`.
     - The trace stays in memory during the run and is written after it, as now. In W2 and W3 the rotation and the GPU work are unchanged.
  6b. **Cost scene `w3f`.** It is W3 with every animation state stepped by the frame counter instead of the clock, so every variant draws the same states in the same order. It is never gate evidence, and W3 stays as it is.
     - **Preroll.** The preroll draws the start pose: the camera at its start pose and the turn angle 0. No state advances, and the camera does not rotate.
     - **Trace entry k** (0-based, its `frame`):
       - when k mod n = 0, the camera is reset to its start pose;
       - then one rotation step is applied before the draw, so `camera` = (k mod n) + 1;
       - the turn is `turnAt` with the frame counter in place of the clock: index k // T, phase (k mod T) / T, and angle (index odd ? -1 : 1) × angle × p²(3 - 2p);
       - the label path is W3's, driven by that turn index: one upload per turn, the same binding, preserved copies and label check, and `revision` = turn.
     - **Sequence.** T is `--turn-frames` and n is `--cycle-frames`.
       - n is a multiple of 2T, so each cycle holds whole pairs of turns.
       - The default n = 3140 is close to one camera rotation (3141.6 steps).
       - Any n consecutive entries show every state of the cycle once.
       - One pure function gives the camera step count, turn index and phase for k, T and n. The render loop and the self-test both use it.
     - **As in W3:** the duration, preroll length, trace, window record, environment, exit codes and the GPU work per frame.
     - **`run.json`:** `"scene": "w3f"`, `"turn_frames": T` and `"cycle_frames": n`, plus the camera object of 6a.
  7. **Display.** While the probe runs, it calls `SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED)`, so the display stays on during unattended runs. It resets the state at exit. `run.json` records `"display_required": true` in its `window` object.
  8. **Build identity.** `buildIdentity()` hashes `sb_probe.exe`, then every compiled shader file, in a fixed order listed in the source. The list includes the new shaders, and a missing file fails as now. CMake builds the new shaders with the same DXC flags.
  9. **`run_scene.ps1`.**
     - New parameter `-Feature`, with the eight names as a ValidateSet and `none` as the default, passed through as `--feature`.
     - Run ids and directory names include the feature when it is not `none`, for example `<stamp>-w3-fog-1`, and so does the summary directory.
     - It refuses a feature other than `none` together with `-Inject`.
     - `-Scene` also accepts `w3f`. The script passes `--scene w3f` and uses the probe's default sequence. `-Inject` stays limited to W3 and W4.
     - It treats exit 4 like a failed run: no gate evidence.
     - Everything else in the script is unchanged, including the administrator check, the session rules and the operator confirmation.
  10. **Self-test.** `--selftest` (CPU only) adds:
      - option parsing of every name and the refused combinations;
      - the `run.json` shape with `feature`, `camera` and `display_required`, and in `w3f` with `turn_frames` and `cycle_frames`;
      - the trace entry shape with `camera`;
      - the `w3f` state function for k across two cycles with T = 157 and n = 3140: camera step count, reset at each cycle start, turn index, phase and direction;
      - the refused `--turn-frames` and `--cycle-frames` values, and the refusal of both options outside `w3f`;
      - the edge mask on the real assets: counts of feature edges and diagonals, as in section 4, and at least one feature edge per sticker;
      - the sort-check and effect-check comparison helpers on synthetic data.
  11. **README.** `README.md` documents:
      - `--feature`, `--snapshot` and `-Feature`;
      - the feature definitions above;
      - the effect check and the new exit code;
      - that the new build has a new identity, so the baseline is measured again with the features' build;
      - the camera record, the cost scene `w3f`, and why costs come from `w3f`;
      - the H-06 procedure:
        - for the costs: three cold `w3f` runs for `none` and for each feature, with `run_scene.ps1 -Scene w3f -Feature <name>`;
        - for the gate verdicts: three cold W3 runs with `run_scene.ps1 -Scene w3 -Feature <name>`, always for `none`, and for each feature as time allows. The table shows a verdict only where they exist;
        - every H-06 run declares `frame_generation`, `upscaling` and `driver_vsync` as `true` or `false`, and `vendor_mode` as text, with `-Declare`. The table treats an undeclared control as unknown and gives no cost;
        - `tools/perf/renderer_gate.py` over all run directories, for the gate record;
        - `tools/perf/feature_costs.py` over the same directories, for the table (from packet H6-T).
  12. The acceptance check passes.

## 2. Actual problem and reproduction
- `--feature` accepts only `none`. Any other name stops with "H-06 features are not implemented" (`src/probe.cpp`, `options`).
- The gated W3 result for build `2b5bf5e6...` gives no feature costs, and the H-06 table is open.
- W3 cannot give paired costs. Its camera steps per frame and its turns follow the clock, so a slower variant samples other states. Packet H6-T section 2 shows the bias: no binning of a clock-driven state removes it.

## 3. Environment and versions
- Windows 11 and MSVC x64 with the Windows SDK and its DXC.
- CMake 4.4.3 and Ninja 1.13.2, from `tools/.venv/renderer-spike`.
- C++20 and HLSL shader model 6.0. A 6.x model is allowed if `report.md` says which and why.
- The owner's GPU: RTX 4070 Laptop GPU with 8 GB, driver 616.92, at 2560 x 1600 and 60 Hz.
- The Codex sandbox has no usable GPU, and the acceptance check is CPU only. Evidence kind there: source and fixtures. Claude runs the GPU checks after integration.

## 4. Necessary source and evidence
- Probe files in `work/experiments/renderer-sb/probe/`:
  - `src/probe.cpp`: options, assets, `run.json`, self-test;
  - `src/gpu.cpp`: device, pipelines, draw, label path, environment, exit codes;
  - `src/probe.h`;
  - `shaders/geometry.hlsl`, with `projectVertex`;
  - `shaders/draw.hlsl` and `shaders/check.hlsl`;
  - `CMakeLists.txt`, `build.cmd`, `check_probe.py`, `run_scene.ps1` and `README.md`.
- Facts from the current source:
  - The draw is one instanced, non-indexed triangle-list call, `DrawInstanced(30480, 600)`.
    - Vertices 3k to 3k+2 form triangle k of the base cell, and `SV_InstanceID` is the cell.
    - The vertex shader projects with the constants in `gpu.cpp` `constants()`: cell shrink 0.76, sticker shrink 0.82, d4 1.18 and zoom 1.15.
  - The root signature has one root CBV (b0), seven root SRVs (t0 to t6) and two root UAVs (u0, u1). There is no shader-visible descriptor heap and no sampler.
  - Depth is `D32_FLOAT` with `LESS_EQUAL`, and there is no culling.
  - Flat shading comes from `ddx`/`ddy` of the projected position. The colour comes from `labels[slot] / 433`.
  - Two frame contexts allow at most two frames in flight.
  - `--msaa 4` renders into a 4x target and resolves it into the back buffer.
  - Exit codes: 1 is an error, 2 a failed label check, and 3 a window not visible or not in the foreground.
  - The render loop in `runGpu`:
    - Each iteration reads the trace clock, then calls `rotate(q,0,3,0.002)` in W2 and W3, then draws.
    - After the trace starts, each iteration pushes one trace entry.
    - The turn follows the QPC clock (`turnAt`), not the frame count.
    - The label path advances with the turn index: one upload per new turn, with `revision` equal to the turn.
  - `buildIdentity()` hashes `sb_probe.exe`, `draw_vs.dxil`, `draw_ps.dxil`, `count_vs.dxil` and `geometry_cs.dxil`, in that order.
- Mesh facts, measured on the assets on 3 October 2026, for the base cell's 10,160 triangles:
  - 3,948 triangles have zero area, and 761 duplicate another triangle of their sticker.
  - Match edges by exact vertex positions, after dropping zero-area triangles and merging duplicates:
    - 7,973 edges have two triangles, 819 have one, and 31 have three or more;
    - 3,277 are feature edges and 5,546 are coplanar diagonals;
    - every sticker has 1 to 32 feature edges.
  - Coplanarity test: the Gram determinant of the shared edge and the two opposite-vertex offsets, divided by the product of their squared lengths.
    - Diagonals reach at most 2.1e-7, and feature edges are at least 2.6e-5.
    - A threshold of 1e-6 separates them.
  - Sticker vertex counts range from 12 to 252, with a mean of 70.4. That is 18,288,000 vertices per frame.
- Projection for depth reconstruction (`geometry.hlsl`):
  - clip = (p.x zoom / aspect, p.y zoom, 100/99.95 (z - 0.05), z), with z = 5 - p.z;
  - so the stored depth d = (100/99.95)(1 - 0.05/z).

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | a bare D3D12 draw meets the gate in W3 | E-2.4-01 probe | three owner-attended cold runs | met: 778.37 fps pooled, p99 1.546 ms, build `2b5bf5e6...` |
| 2 | features can be added later | `--feature none` hook only | — | the H-06 table is open |
| 3 | W3 runs can give feature costs by balancing states | H6-T plan, versions 2 and 3 | Astra re-check; CPU simulation | no: clock-driven turns bias any binning, so costs come from the paired scene `w3f` |

## 6. Constraints and owned files
- Owned files: `work/experiments/renderer-sb/probe/**` only. Do not change any of these:
  - `assets/`, `reference/`, `check_assets.py`, `reference_geometry.py`, `SPEC.md`, `RESULT.md`, `handoff/`;
  - anything outside the probe directory.
- Synthetic labels only, and no personal session.
- No NVIDIA-specific features.
- No new third-party dependency. Use only Windows SDK libraries; `windowscodecs` is allowed for PNG.
- Implement in this order, and finish each step before starting the next:
  1. the options, `run.json`, the camera record and the `w3f` scene, build identity, display request, `run_scene.ps1`, effect check and snapshots, with `no-gaps`, `fog` and `msaa4`;
  2. `outlines`;
  3. `transparency`;
  4. the post-process base, then `dof` and `ao`.

  If time runs out, a feature not finished is refused with "not implemented", and `report.md` lists it.
- The probe build is new, so its identity changes. The gated W3 result stays valid only for build `2b5bf5e6...`.
- Do not run the probe on a GPU, start PresentMon, or create files outside the build directory that `check_probe.py` uses.

## 7. Required return format
- The changed probe files.
- `report.md` with:
  - for each feature: the method, passes, resources and shader files, any deviation from section 1, and whether it is finished;
  - the new `buildIdentity` order;
  - how the effect check and the sort check work;
  - the self-test additions;
  - the acceptance check output.

```implement-contract
{"allowed_files": ["work/experiments/renderer-sb/probe/*"], "acceptance_check": ["python", "work/experiments/renderer-sb/probe/check_probe.py"], "stop_condition": "the acceptance check passes; --feature accepts the eight names with every feature either implemented as specified or refused as not implemented and listed in report.md; the baseline shaders and command sequence are unchanged; run.json, the camera record, the w3f cost scene, the effect check, run_scene.ps1 -Feature and -Scene w3f, and README.md are done as specified"}
```
