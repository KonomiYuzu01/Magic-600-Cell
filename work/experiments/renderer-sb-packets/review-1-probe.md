# Review packet: S-B probe, geometry reference and readiness program (shard 1 of 2)

Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: find defects that would make the S-B gate evidence wrong or the probe unusable for it: the W3 workload not matching the SPEC, a label check that could pass with a real fault, a trace that misrepresents the frames, a D3D12 synchronisation or resource-state error, or a run script that produces invalid or misleading captures.
- Acceptance: one JSON result matching `schemas/review-result.schema.json`. Severity `blocker` or `major` only for defects with a concrete counterexample (inputs or sequence of events, and the wrong result).
- Non-goals: performance numbers; code style; the handoff test (`work/experiments/renderer-sb/handoff/`, shard 2, not yet integrated); changes to the gate (`tools/perf/renderer_gate.py`) or to the plan.

## 2. Actual problem and reproduction
- The probe is new code for stage 2.4 packet E-2.4-01 (bare Direct3D 12). Codex wrote SB-A (reference) and SB-B (probe) through the wrapper; the integrator changed `probe/run_scene.ps1` (probe no longer started hidden; PresentMon `--terminate_on_proc_exit` instead of a fixed `--timed`; swap chain filled by text replacement; process handles cached right after `Start-Process` so `ExitCode` is available after exit), `probe/src/gpu.cpp` (window `WS_EX_TOPMOST`, `SetForegroundWindow`, `run.json` `window` record with `foreground_at_trace_start`, `presents`, `presents_occluded`) and `probe/build.cmd` (CRLF line endings).
- Reproduce: `python work/experiments/renderer-sb/check_assets.py`; `python work/experiments/renderer-sb/probe/check_probe.py` (builds out of tree and runs `sb_probe.exe --selftest`).

## 3. Environment and versions
- Base: branch `claude/renderer-sb` at its current HEAD (diff against `main` at `64d72b3`; the probe, reference and readiness files came in `2eab553` and `3b08c1e`, the `run_scene.ps1` handle change after them).
- Windows 11, Visual Studio 2026 (MSVC 14.51), Windows SDK 10.0.26100.0, DXC 1.8.2502.11, CMake 4.4.3, Ninja 1.13.2, CPython 3.14. Owner GPU: NVIDIA GeForce RTX 4070 Laptop GPU, hybrid laptop now set by the owner to discrete-GPU mode (panel on the NVIDIA GPU, which is the only display adapter listed), 2560 x 1600 at 60 Hz, mains power, Windows power mode best performance.
- Evidence kind in the review sandbox: source/fixture only.

## 4. Necessary source and evidence
- Contract: `work/experiments/renderer-sb/SPEC.md` (sections 2 to 7), `cameras.json`, `workload/turn.json` and `workload/export_workload.py`.
- Gate reader: `tools/perf/renderer_gate.py` (run format `magic600-renderer-run-v1`, trace checks, conditions) and `tools/perf/b412_summary.py`.
- Authority: `docs/progress/1.0/renderer-experiment-plan.md` sections 1, 2 and 5; `docs/wiki/decisions/renderer-candidates.md`.
- Files under review:
  - `work/experiments/renderer-sb/reference_geometry.py`, `check_assets.py`, `reference/index.json`
  - `work/experiments/renderer-sb/probe/src/probe.cpp`, `src/gpu.cpp`, `src/probe.h`, `src/json.h`
  - `work/experiments/renderer-sb/probe/shaders/geometry.hlsl`, `draw.hlsl`, `check.hlsl`
  - `work/experiments/renderer-sb/probe/run_scene.ps1`, `build.cmd`, `check_probe.py`, `CMakeLists.txt`, `README.md`
  - `work/experiments/renderer-readiness/minimal_d3d12.cpp`, `capture.ps1`, `build.cmd`
- Integrator's local results on the owner's machine (actual Windows/DirectX, not performance evidence):
  - `check_assets.py`: three summary lines ok; `check_probe.py`: build ok, `selftest: ok`.
  - `sb_probe.exe --geometry-check`: pass; 9 of 9 camera/pose comparisons, max abs error 1.4e-6; 30,480 vertex-shader invocations for each of the 600 cells.
  - `sb_probe.exe --scene w3 --duration 12 --preroll 2` (launched from a background shell): label check pass (63 copies, 63 revisions), `presents_occluded` 0 of 7,925, `foreground_at_trace_start` false (topmost window visible on screen), probe self-timing about 577 fps mean and 3.3 ms p99 (not gate evidence), VRAM peak about 134 MB.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | a background-launched window is visible | none | screen capture during a W3 run | it was not: the window stayed behind other windows; fixed with `WS_EX_TOPMOST` and recorded with the `window` fields |
| 2 | `build.cmd` works from cmd | none | local build | `goto build_ok` failed ("cannot find the batch label") with LF line endings; fixed with CRLF |

## 6. Constraints and owned files
- Read-only review. Invariants: the full `600-cell-Full` model (259,800 slots, all 600 cells drawn every frame, no filtering or LOD); labels synthetic; no personal session; no NVIDIA-specific features; nothing in the probe may alter mechanical state.
- Questions the integrator wants answered:
  1. Can `checkLabels` (`probe.cpp`) report `pass` when a frame drew with labels that differ from the oracle of its revision, or when a revision was adopted late, skipped or bound from the wrong buffer? Consider frames slower than one turn, the three-buffer label ring with two frames in flight, and the upload ring.
  2. Does the trace (`trace.jsonl`) satisfy `renderer_gate.py`'s trace checks for a valid run, and could it pass them for a run that did not render the declared workload?
  3. Are there D3D12 hazards: buffer reuse while in flight, missing barriers, wrong states (for example the label buffer read by the vertex shader in `NON_PIXEL_SHADER_RESOURCE`), readback mapping before completion?
  4. Does `run_scene.ps1` produce a run directory the gate reads correctly (PresentMon CSV columns, process id, swap-chain address, interval coverage), and does it refuse or flag a run that is not representative?
  5. Does the reference (`reference_geometry.py`) match SPEC sections 3 and 7 independently of the shader, so that agreement is evidence?

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`: findings with ID, severity, evidence (`path:line` with file digest), counterexample, suggested experiment, verification status.
- Review only; do not perform follow-up work.
