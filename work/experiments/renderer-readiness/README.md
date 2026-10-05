# Renderer window readiness (packet E-2.4-00)

Checklist for [E-2.4-00](../../../docs/progress/1.0/packets/renderer/E-2.4-00-readiness.md), run on the owner's Windows machine on 3 October 2026. Evidence kind: setup only. Nothing here is a renderer result or a timing claim.

## Files

| File | What it is |
|---|---|
| `minimal_d3d12.cpp`, `CMakeLists.txt` | Clear-and-present D3D12 program: borderless window at the display's native size, high-performance adapter, flip model, two back buffers, V-Sync on. Writes a `run.json` skeleton (scene `w1`, format `magic600-renderer-run-v1`). |
| `build.cmd` | Builds it with the MSVC x64 toolset (found with `vswhere`), CMake and Ninja from `tools/.venv/renderer-spike`. Output in `build/` (ignored). |
| `capture.ps1` | Administrator PowerShell only. Runs the program, takes a 20 s PresentMon 2.6.0.0 capture (`--v1_metrics --qpc_time --process_id <pid>`), fills the swap chain from the capture into `run.json` and runs `tools/perf/renderer_gate.py`. The capture stays under `work/loop-memory/perf/renderer/readiness/` (private). |

## Tools (section 4 of the packet), observed 3 October 2026

| Tool | Expected | Observed | State |
|---|---|---|---|
| PresentMon | 2.6.0.0 | 2.6.0.0 (winget record; `PresentMon-2.6.0-x64.exe` present) | done |
| RenderDoc | 1.46.0 | 1.46.0 | done |
| PIX | 2603.25 | 2603.25 | done |
| CMake | 4.4.3 | 4.4.3 (`bootstrap.py install cmake`) | done |
| Ninja | 1.13.2 | 1.13.2 | done |
| aqtinstall | 3.3.0 | 3.3.0 | done |
| FLIP | 1.7 | 1.7 | done |
| py-spy, viztracer | pinned | 0.4.2, 1.1.1 | done |
| Engine Python | CPython 3.14.7, NumPy 2.3.5 | CPython 3.14.7, NumPy 2.3.5 (`tools/.venv/engine`) | done |
| Godot .NET | 4.7.2 intended, unpinned | missing | blocked on owner: `bootstrap.py pin godot`, then `bootstrap.py approve` (needed by S-A2 only) |
| Blender | unpinned | missing | not needed by the renderer window |
| Visual Studio (C++ x64 tools) | owner-recorded | Visual Studio Community 2026 18.10, MSVC 14.51 (cl 19.51) | done (owner-installed) |
| Windows SDK | owner-recorded | 10.0.26100.0 with `d3d12.h`, `dxc.exe`; `d3d12SDKLayers.dll` present (debug layer available) | done (owner-installed) |
| .NET SDK 8 | no lockfile entry | not checked | not needed until S-A2 |
| Qt 6 for MSVC x64 | no lockfile entry | not installed | not needed until S-D |
| DirectX Agility SDK | optional | not installed | not needed so far |
| NVIDIA driver | record | 616.92 | recorded |

## Machine conditions observed

- GPU: NVIDIA GeForce RTX 4070 Laptop GPU, 8188 MiB. The DXGI high-performance adapter is the RTX 4070 (the program reports it).
- Display: 2560 x 1600 at 60 Hz, driven by the integrated AMD Radeon 610M (hybrid graphics): frames rendered on the RTX 4070 are copied to the integrated GPU for display.
- Power: mains (battery 100 %). Power plan: **Balanced**. The gate needs high-performance mode; the owner switches it before gate runs.

## Acceptance items (section 1 of the packet)

1. Tools: done for every tool the S-B probe needs; Godot is blocked on the owner's pin and approve (S-A2 only).
2. Minimal D3D12 program builds with CMake and Ninja and presents on the RTX 4070: done (3 s smoke run, 180 frames at 2560 x 1600, adapter "NVIDIA GeForce RTX 4070 Laptop GPU"). NVIDIA Control Panel confirmation: owner.
3. 20 s PresentMon capture parsed by `renderer_gate.py`: pending the owner's run of `capture.ps1` in an administrator terminal.
4. `python tools/perf/check_renderer_assets.py`: done (`renderer assets: ok (600 cells x 433 stickers = 259,800 slots; 30,480 base vertices)`).
