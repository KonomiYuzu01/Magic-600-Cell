# Stage 2.4 renderer experiment plan

Status: **revised after the Astra plan check of 1 October 2026** (call `20261001T020112Z-5200bbe5`; findings R1–R8 adopted). Two scoped re-checks followed: `20261001T021256Z-d5bfed6d` raised R5 again and a new R9, both adopted, and `20261001T021627Z-a255e883` passed. No `blocker` or `major` is open. The candidates and the selection gate are fixed in [renderer-candidates](../../wiki/decisions/renderer-candidates.md); this plan only says how to test them inside the 12-day window (stage days 3 to 14, 3 to 14 October 2026, owner decision of 1 October). Experiment cards follow [stage-2-experiment-protocol](stage-2-experiment-protocol.md) section 2.

## 1. Workload (same for every candidate)

Synthetic geometry from the model assets, never a personal session:

| Asset | Content |
|---|---|
| `assets/mesh_vertices.f32` | 30,480 base vertices, 4 x f32 each (4D), for one cell |
| `assets/mesh_sticker.u32` | sticker index per base vertex (433 stickers per cell) |
| `assets/mesh.json` | `offsets`: the vertex range of each base sticker |
| `assets/cell_frames.f32` | 600 frames, 16 x f32 each, placing the base cell |
| `assets/slot_piece.u32` | slot to piece, for colour by label |

Full detail is 600 x 433 = 259,800 stickers and about 18.3 million projected vertices per frame. The probe may restructure the work (instancing per cell frame, GPU projection, compute) but must draw every sticker of every cell.

Geometry reference: the 0.4 projection and sticker shrink live in the retained runtime assembly that `native/NativeFullRenderer.cs` adapts by reflection; they are not in repository source. S-B therefore first writes its projection, cell and sticker shrink and turn interpolation as a small Python reference (`reference_geometry.py`) and emits immutable reference outputs: projected vertex positions and per-cell draw counts for three fixed cameras and the start, midpoint and end poses of the W3 turn. The D3D12 output is checked against these outputs before any timing run, and the reference uses the 0.4 camera parameters (`radius`, `angle`, `face_shrink`, `sticker_shrink`, as captured by the 0.4 host's `CaptureCamera`). A side-by-side image against the 0.4 host at one fixed camera is recorded for the owner, but it is not a gate.

Scenes:
- **W1 static:** full detail, fixed camera.
- **W2 rotation:** full detail, continuous scripted camera rotation (the B4-12 M3 path: one fixed rotation step per frame).
- **W3 sustained turns (the gate trace):** full detail, one repeatable trace that runs for the whole capture.
  - Turns run back to back without idle frames: the generator with the largest moved-slot count, then its inverse, and so on. This keeps the state legal and the trace periodic.
  - Each turn is animated over its full duration: the 0.4 turn duration until the motion table (H-03) arrives, then the H-03 duration.
  - W2 camera rotation runs throughout.
  - After every turn the engine side uploads a fresh authoritative 259,800-entry label buffer, and the next frame adopts it. Upload and adoption costs are therefore inside the gated frames.
  - The probe writes a trace log with the QPC time, the animation phase and the adopted label revision of every frame. The gate summary uses this log to refuse a run whose measured interval has an idle gap or a missed adoption (section 2).
  - Trace contract:
    - One entry per frame.
    - Its QPC time is read after the previous `Present` returned and before this frame's `Present`.
    - Its phase is the elapsed fraction of the current turn on the QPC clock, not a per-frame step.
    - The adapter refuses the run when any of these fail:
      - each step between two presents holds exactly one entry;
      - every entry of a turn places the turn's start within 50 ms;
      - consecutive turn starts lie one declared turn duration apart, within 100 ms.
- **W4 labels (attribution):** the same uploads without animation, to attribute their cost. The gate does not use W4. Label correctness is checked on every W3 and W4 run (section 2).
- **W5 design:** W3 inside the design track's current layout (H-04) with its visual features (H-01), finalist preset (H-02) and motion table (H-03), using whatever version has been handed over (section 6).

The gate is measured on W3, and on W5 once the finalist preset has arrived; the selection uses W5 when it exists, because the product has to hold the gate with its look, not only bare. W1, W2 and W4 attribute cost. The experiment dates are the compatibility test of the design and engineering tracks: every candidate is tried with the design's features and layout as they arrive.

## 2. Measurement

The B4-12 M3 method (`docs/progress/0.4.1/b4-12-harness.md`) supplies the capture conditions and the PresentMon parsing. Its summary tool `tools/perf/b412_summary.py` cannot judge this gate. It counts only the first 100 frames after warmup, limits its context statistics to 60 s and treats p99 as report-only, so a run that slows down after 100 frames would pass.

The gate therefore uses a new adapter, `tools/perf/renderer_gate.py`. It reuses the PresentMon parsing and the nearest-rank percentile of `b412_summary.py`, and evaluates every frame of a fixed interval:
- PresentMon on the candidate process only, CSV with `msBetweenPresents` and `msBetweenDisplayChange`.
  - Use PresentMon 2.6.0.0, `PresentMon-2.6.0-x64.exe` from its `PresentMonConsoleApplication` folder.
  - Run it from an administrator terminal: without elevation it ends with exit code 6 and writes no CSV.
  - Pass `--v1_metrics --qpc_time --process_id <pid> --output_file <run>/presentmon.csv`. The default 2.x metrics have different columns.
  - `renderer_gate.py` reads `ProcessID`, `SwapChainAddress`, `QPCTime` and `msBetweenPresents`. These match a real 2.6 capture of 1 October 2026: integer QPC times at 10 MHz. The probe writes QPC markers for the start and stop of the trace.
- A 10 s warmup is excluded. The measured interval is exactly the next 180 s ("several minutes" in the gate, taken as at least three). Three cold runs per scene.
- A run whose PresentMon rows or trace log do not cover the whole interval is invalid. So is a run whose trace log shows an idle gap or a missed label adoption.
- Every frame of the interval counts: fps = 1000 / mean(`MsBetweenPresents`), and p99 frame time by nearest rank.
- The gate is met only if each of the three runs has fps >= 30 and p99 <= 33.3 ms, and the pooled frames of the three runs meet both thresholds as well.
- The adapter's tests include the late-slowdown counterexample from the plan check: 100 frames of 17 ms followed by 50 ms frames must fail. They also cover a short capture, a missing marker and a repeated run, each of which must be refused. The adapter and its tests are built and reviewed before the first GPU capture.
- Conditions recorded per run: mains power, high-performance mode, discrete GPU confirmed, driver, native resolution, refresh rate, V-Sync, no frame generation or DLSS. Some of these are checked by the adapter, and a run is invalid when a checked condition is violated or not declared:
  - battery power;
  - frame generation;
  - upscaling;
  - a backbuffer smaller than the display.
- Each run also declares:
  - the workload's swap chain, the only one counted; other chains are reported;
  - the turn duration, so a frozen or slowed animation is refused.
- Peak VRAM recorded (budget at most about 7 GB).
- **Label correctness, exact, in the timed run itself.** Labels are checked as integers, not colours: labels within one cell share a colour (1.0 architecture V10), so colour comparison cannot prove label equality.
  - In the first frame that uses a new label revision, the same command list copies the label buffer actually bound for that frame's draws into a readback ring slot, after the draws, with `CopyBufferRegion`. All 259,800 integer labels are copied, about 1 MB per turn.
  - The probe records the frame index, the label revision and the bound resource identity with the slot.
  - The CPU compares the preserved copies with the engine oracle's labels for that revision after the capture. The comparison is not timed, but the copies are part of every gated run, so their cost is inside the measured frames. W4 reports that cost.
  - Any mismatch, a missing copy, or a first use of a revision later than the frame after its upload invalidates the run. A later replay never substitutes for these copies.
  - Negative tests must be rejected: one corrupted label, two swapped labels of the same colour, an adoption delayed by one frame, and a stale binding injected only during the capture.

Raw captures stay under `work/loop-memory/perf/renderer/` (private). Only the summary table is published.

## 3. Sequence and decision points

| Window day (stage day) | Experiment | Kill or pass criteria |
|---|---|---|
| Before day 1 (by stage day 3) | **Readiness** | The `renderer-spike` and `native-performance` tools are installed at approved exact versions (PresentMon, RenderDoc, PIX, CMake, Ninja, the compiler). A minimal D3D12 program builds and presents. A short PresentMon capture of it is accepted by `renderer_gate.py` (as invalid for length, but parsed). Setup failures are recorded as setup failures, not as renderer results. |
| 1–2 (3–4) | **E-2.4-01 S-B** bare Direct3D 12 probe (packet in section 5), including the feature cost table (H-06) | Pass: W3 meets the gate. Kill the whole route early only if S-B cannot reach 30 fps at full detail; then the window goes to an escalation on the drawing method, not to the frameworks. |
| 3–6 (5–8) | **E-2.4-02 S-A2** Godot 4.7 .NET with a custom RenderingDevice renderer, and **E-2.4-03 S-D** Qt Quick with `QQuickRhiItem` on QRhi D3D12, in parallel | Each first passes an interop smoke test inside the actual framework, before any geometry port. A changing, sequence-numbered test texture produced by D3D12 work is displayed through the framework's own D3D12 backend. Device ownership (same device where the framework supports it, for example `QQuickRhiItem` on the window's QRhi), synchronisation, resize and teardown all work. Then, only after an S-B W3 run has met the gate, W1–W4 run inside the framework using the S-B drawing method. The first layout specification (H-04, day 6) is the window structure, and the first visual feature list (H-01, day 6) is prototyped. Interop design is checked against R-02, R-04, R-14 and R-17 ([requirements](requirements-from-screening.md)). Each reports its renderer constraints (H-09) by day 8. |
| 7 (9) | **Day-7 go/no-go**, Astra gate ruling (`--gate day7-go-no-go`); the result goes to the design track (H-07) | Go only for a candidate whose W3 runs meet the gate exactly (section 2). No-go drops a candidate that does not. A near-pass candidate is recorded as failing, and it continues only under an explicit owner exception that names the fix and its deadline. A predicted improvement is not passing evidence. If none passes: continue S-B as the dedicated renderer with a minimal shell, and record the risk. |
| 8–11 (10–13) | Remaining candidates: integrate the finalist presets (H-02) and motion table (H-03) as W5; three formal runs each of W3 and W5 | The gate on three runs; VRAM within budget. A feature that alone breaks the gate goes back to the design track with its measured cost. |
| 12 (14) | **Selection**, sent to the design track (H-10) | The candidate that passes W5 (or W3 if no preset arrived), preferring the one the design track (vertical slice, stage days 13–15) can use with the least extra work. If none passes, record the failure; the owner decides the next step (protocol section 5). |

NVIDIA-only work (Reflex, DLSS, SER) is never on this path.

## 4. Correctness boundary

The renderer never changes mechanical state or relabels pieces. Every candidate reads labels from the engine side; colour is a function of the label only. Picking (screen point to sticker) is a separate check: a fixed list of 100 screen points must resolve to the expected stickers in W1 (1.0 architecture V10, probe `pick`).

## 5. Packet: E-2.4-01 S-B bare Direct3D 12 probe

### 1. Goal and acceptance
- Goal: find out whether a bare Direct3D 12 renderer draws the full 600-cell at full detail with the most complex turn animation within the selection gate, and establish the drawing method and the resource-sharing path the framework candidates will reuse.
- Acceptance, in priority order:
  1. The geometry check against `reference_geometry.py` passes.
  2. W1–W4 run on the owner's RTX 4070 Laptop GPU.
  3. Three cold W3 runs are judged by `renderer_gate.py`, and the exact label check passes on every run (section 2).
  4. A minimal D3D12 resource handoff with fence synchronisation works, both on the same device and to a second device. This is only the S-B half of interop: each framework still runs its own smoke test with its actual API (section 3).
  5. A cost table (H-06) gives the frame time added by each candidate visual feature, switched on alone at full detail in W3: sticker gaps, outlines, per-sticker transparency with sorting, fog, depth of field, ambient occlusion and MSAA 4x. A feature not measured by the time box is listed as unmeasured, never estimated.
- Non-goals: UI, input beyond camera rotation, Look Lab presets, NVIDIA features, shipping code quality.

### 2. Actual problem and reproduction
- 0.4 draws with Managed DirectX 9 on .NET Framework (`native/NativeFullRenderer.cs`). 1.0 needs a Direct3D 12 base (owner decision, 29 September 2026), and no D3D12 drawing of this model exists yet.
- Expected: a number for each scene, plus a working method.

### 3. Environment and versions
- Base commit: `main` with `tools/perf/renderer_gate.py` merged.
- Windows 11, RTX 4070 Laptop GPU 8 GB, current NVIDIA driver; `renderer-spike` and `native-performance` toolchain profiles (PresentMon, RenderDoc, PIX) at the exact versions recorded by the readiness step (section 3).
- Language: C++20 with the Windows SDK, or C# with a D3D12 binding; the implementer picks one and states why. Build with CMake and Ninja.
- Evidence kind: actual Windows/DirectX and performance, valid only for the built probe identity.

### 4. Necessary source and evidence
- Assets in section 1; `core.py:34` (`Model`) for loading and the asset hashes.
- The geometry reference rule in section 1. `native/NativeFullRenderer.cs` only shows how 0.4 adapts the runtime renderer; it does not specify the projection.
- The 0.4 camera parameters as captured by `CaptureCamera` (`native/NativeHost.cs`).
- The gate: `docs/wiki/decisions/renderer-candidates.md`. Measurement: section 2 of this plan.

### 5. Attempts so far
None. The 0.4 renderer is the only prior art; its measurements are not 1.0 evidence.

### 6. Constraints and owned files
- Never change `assets/`, model identity, labels or mechanical state. Read assets through their manifest hashes.
- No personal session; synthetic labels only (solved, and the scripted turn).
- Owned files: `work/experiments/renderer-sb/**` only. No Managed DirectX DLLs; nothing under `native/` changes.
- Time box: 1.5 days. Stopping optimisation is separate from completing acceptance.
  - **Early pass:** when a drawing method passes W3, stop trying other methods. Still complete acceptance items 1–4, then measure as many H-06 features as the time box allows.
  - **Early failure:** when two drawing methods both stay below 50 % of the gate, stop optimising. Still complete items 1, 2 and 4, record the W3 figures, and escalate the drawing method.
  - **Timeout:** deliver what exists. Unfinished items are listed as not done, and unmeasured features as unmeasured.
  - Framework work has two levels, and S-B's result decides how far it goes:
    - The interop smoke test of S-A2 and S-D (section 3, a test texture, no geometry) may start once item 4 exists, whatever S-B's result.
    - The geometry port (W1–W4 inside a framework) starts only after an S-B W3 run on the owner's machine meets the gate.
  - On early failure or a timeout without a passing W3 run, no geometry port starts. The window goes to the drawing-method escalation in section 3.
  - The H-06 table is due on day 5 with whatever has been measured.

```implement-contract
{"allowed_files": ["work/experiments/renderer-sb/*"], "acceptance_check": ["python", "work/experiments/renderer-sb/check_assets.py"], "stop_condition": "the probe builds; check_assets.py confirms the asset digests, the vertex and sticker counts and the geometry reference outputs; the handoff test and the trace log are implemented; and the measurement and label-check steps are written down for the owner's machine"}
```

The acceptance check in the contract covers only what a sandbox can prove: the assets are read correctly, the counts are right and the reference geometry matches. Frame-time results, label readbacks and the handoff run come only from the owner's machine.

### 7. Required return format
- The probe source, a `README.md` with build and run commands, `check_assets.py`, `reference_geometry.py` with its reference outputs, and the trace-log format.
- A result card (protocol section 2) filled after the owner's runs. It gives:
  - per-scene fps and p99 from `renderer_gate.py`, the VRAM peak and the label-check result;
  - the drawing method and the handoff result;
  - which acceptance items are done, not done or unmeasured, and what failed.

## 6. Cross-track record (engineering side)

Mirror of the design side in [stage-2-experiment-protocol](stage-2-experiment-protocol.md) section 4.6, with the same IDs. The engineering track covers the renderer experiments and the backend boundaries (engine, session store, command layer, view model, shell, renderer; charter section 4). When a handoff happens, both copies are updated with the date and where the item lives. A late handoff is not waited for: the receiving track uses the last delivered version and records that.

**Engineering track gives**

| ID | Item | Due | From | Used by the design track for | Status |
|---|---|---|---|---|---|
| H-06 | Cost table: frame time per visual feature at full detail | day 5 | S-B | the Look Lab cost meter | open |
| H-07 | Day-7 go/no-go result and remaining candidates | day 9 | Astra ruling | Look Lab framework risk; where G6 is built | open |
| H-08 | Command table and layer boundaries | day 4 | 2.1 and 2.2 | greybox layouts, keyboard model, flow scripts | first version 1 October: [command-table](command-table.md); final after the 2.2 dispositions |
| H-09 | Renderer constraints (overlay layers, text in the 3D view, transparency and sorting limits) | day 8 | S-A2 and S-D | which features stay in the feature list | open |
| H-10 | Selected renderer | day 14 | selection | vertical slice; rebuild of framework-neutral outputs if not Godot | open |

**Engineering track needs**

| ID | Item | First version | Final | Used for | Status |
|---|---|---|---|---|---|
| H-01 | Visual feature list | day 6 | day 10 | W5 and the renderer feature list frozen in 2.5 | open |
| H-02 | Finalist presets (JSON tokens) | day 10 | day 12 | W5 formal runs, vertical slice | open |
| H-03 | Motion table | day 11 | day 12 | W5 turn duration and easing | open |
| H-04 | Layout specification | day 6 | day 12 | window structure, UI-over-3D compositing, input routing | open |
| H-05 | Pointing needs from the core flows | day 4 | day 8 | picking design and the `pick` probe | open |
