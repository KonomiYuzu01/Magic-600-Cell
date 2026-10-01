# Stage 2.4 renderer experiment plan

Status: **draft for a Codex Astra plan check** (it decides an architecture direction). The candidates and the selection gate are fixed in [renderer-candidates](../../wiki/decisions/renderer-candidates.md); this plan only says how to test them inside the 12-day window (stage days 3 to 14, 3 to 14 October 2026, owner decision of 1 October). Experiment cards follow [stage-2-experiment-protocol](stage-2-experiment-protocol.md) section 2.

## 1. Workload (same for every candidate)

Synthetic geometry from the model assets, never a personal session:

| Asset | Content |
|---|---|
| `assets/mesh_vertices.f32` | 30,480 base vertices, 4 x f32 each (4D), for one cell |
| `assets/mesh_sticker.u32` | sticker index per base vertex (433 stickers per cell) |
| `assets/mesh.json` | `offsets`: the vertex range of each base sticker |
| `assets/cell_frames.f32` | 600 frames, 16 x f32 each, placing the base cell |
| `assets/slot_piece.u32` | slot to piece, for colour by label |

Full detail is 600 x 433 = 259,800 stickers and about 18.3 million projected vertices per frame. The projection and sticker shrink follow the retained 0.4 renderer (`native/NativeFullRenderer.cs`); the probe may restructure the work (instancing per cell frame, GPU projection, compute) but must draw every sticker of every cell.

Scenes:
- **W1 static:** full detail, fixed camera.
- **W2 rotation:** full detail, continuous scripted camera rotation (the B4-12 M3 path: one fixed rotation step per frame).
- **W3 turn:** full detail, the most complex turn animation (the generator with the largest moved-slot count, animated over its full duration) during W2 rotation.
- **W4 labels:** after each turn, colours come from a new 259,800-entry label buffer uploaded from the engine side; the frame after adoption must show exactly those labels.

The gate is measured on W3. W1, W2 and W4 attribute cost.

## 2. Measurement

Reuse the B4-12 M3 method (`docs/progress/0.4.1/b4-12-harness.md`, on branch `claude/0-4-1-b412-perf` until it merges) and its summary tool `tools/perf/b412_summary.py`:
- PresentMon on the candidate process only, CSV with `MsBetweenPresents` and `MsBetweenDisplayChange`.
- 5 s warmup excluded; 75 s capture; three cold runs.
- fps = 1000 / mean(`MsBetweenPresents`); p99 frame time nearest-rank; the gate needs both pooled and per-run values to pass.
- Conditions recorded per run: mains power, high-performance mode, discrete GPU confirmed, driver, native resolution, refresh rate, V-Sync, no frame generation or DLSS.
- Peak VRAM recorded (budget at most about 7 GB).
- A label-correctness check per run: read back the colour of a fixed sample of 1,000 stickers after a W4 turn and compare with the expected labels. A wrong colour invalidates the run.

Raw captures stay under `work/loop-memory/perf/renderer/` (private). Only the summary table is published.

## 3. Sequence and decision points

| Window day (stage day) | Experiment | Kill or pass criteria |
|---|---|---|
| 1–2 (3–4) | **E-2.4-01 S-B** bare Direct3D 12 probe (packet in section 5) | Pass: W3 meets the gate. Kill the whole route early only if S-B cannot reach 30 fps at full detail; then the window goes to an escalation on the drawing method, not to the frameworks. |
| 3–6 (5–8) | **E-2.4-02 S-A2** Godot 4.7 .NET with a custom RenderingDevice renderer, and **E-2.4-03 S-D** Qt Quick with `QQuickRhiItem` on QRhi D3D12, in parallel | Each: W1–W4 running inside the framework, using the S-B drawing method. Interop design checked against R-02, R-04, R-14 and R-17 ([requirements](requirements-from-screening.md)). |
| 7 (9) | **Day-7 go/no-go**, Astra gate ruling (`--gate day7-go-no-go`) | Go for each candidate whose W3 run meets the gate or is within 20 % with a named, testable fix. No-go drops it. If none: continue S-B as the dedicated renderer with a minimal shell and record the risk. |
| 8–11 (10–13) | Remaining candidates: three formal runs each, plus integration of the Look Lab finalist preset | The gate on three runs; VRAM within budget. |
| 12 (14) | **Selection** | The candidate that passes, preferring the one the design track (vertical slice, stage days 13–15) can use with the least extra work. If none passes, record the failure; the owner decides the next step (protocol section 5). |

NVIDIA-only work (Reflex, DLSS, SER) is never on this path.

## 4. Correctness boundary

The renderer never changes mechanical state or relabels pieces. Every candidate reads labels from the engine side; colour is a function of the label only. Picking (screen point to sticker) is a separate check: a fixed list of 100 screen points must resolve to the expected stickers in W1 (1.0 architecture V10, probe `pick`).

## 5. Packet: E-2.4-01 S-B bare Direct3D 12 probe

### 1. Goal and acceptance
- Goal: find out whether a bare Direct3D 12 renderer draws the full 600-cell at full detail with the most complex turn animation within the selection gate, and establish the drawing method and the resource-sharing path the framework candidates will reuse.
- Acceptance: W1–W4 run on the owner's RTX 4070 Laptop GPU; three cold W3 runs summarised by `b412_summary.py`; label check passes; a shared-texture plus fence handoff to a second D3D12 device or process works in a minimal test (the path S-A2 and S-D need).
- Non-goals: UI, input beyond camera rotation, Look Lab presets, NVIDIA features, shipping code quality.

### 2. Actual problem and reproduction
- 0.4 draws with Managed DirectX 9 on .NET Framework (`native/NativeFullRenderer.cs`). 1.0 needs a Direct3D 12 base (owner decision, 29 September 2026), and no D3D12 drawing of this model exists yet.
- Expected: a number for each scene, plus a working method.

### 3. Environment and versions
- Base commit: `main` after the perf branch merge.
- Windows 11, RTX 4070 Laptop GPU 8 GB, current NVIDIA driver; `renderer-spike` and `native-performance` toolchain profiles (PresentMon, RenderDoc, PIX).
- Language: C++20 with the Windows SDK, or C# with a D3D12 binding; the implementer picks one and states why. Build with CMake and Ninja.
- Evidence kind: actual Windows/DirectX and performance, valid only for the built probe identity.

### 4. Necessary source and evidence
- Assets in section 1; `core.py:34` (`Model`) for loading and the asset hashes; `native/NativeFullRenderer.cs` for the 0.4 projection and shrink.
- The gate: `docs/wiki/decisions/renderer-candidates.md`. Measurement: section 2 of this plan.

### 5. Attempts so far
None. The 0.4 renderer is the only prior art; its measurements are not 1.0 evidence.

### 6. Constraints and owned files
- Never change `assets/`, model identity, labels or mechanical state. Read assets through their manifest hashes.
- No personal session; synthetic labels only (solved, and the scripted turn).
- Owned files: `work/experiments/renderer-sb/**` only. No Managed DirectX DLLs; nothing under `native/` changes.
- Time box: 1.5 days. Stop early when W3 passes or when two drawing methods both stay below 50 % of the gate.

```implement-contract
{"allowed_files": ["work/experiments/renderer-sb/*"], "acceptance_check": ["python", "work/experiments/renderer-sb/check_assets.py"], "stop_condition": "the probe builds, check_assets.py confirms the asset digests and the vertex and sticker counts, and the measurement steps are written down for the owner's machine"}
```

The acceptance check in the contract covers only what a sandbox can prove (assets read correctly, counts right). Frame-time results come only from the owner's machine.

### 7. Required return format
- The probe source, a `README.md` with build and run commands, `check_assets.py`.
- A result card (protocol section 2) filled after the owner's runs: per-scene fps and p99, VRAM peak, the drawing method, the handoff result, and what failed.
