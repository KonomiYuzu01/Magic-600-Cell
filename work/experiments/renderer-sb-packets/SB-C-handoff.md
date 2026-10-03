# Packet SB-C: S-B D3D12 resource handoff with fence synchronisation (E-2.4-01 item 4)

## 1. Goal and acceptance
- Goal: a small standalone test that hands a sequence-numbered D3D12 texture from a producer to a consumer with fence synchronisation, (a) on the same device and (b) to a second device, and verifies every handed-over frame. It is the S-B half of interop that the framework smoke tests (Godot RenderingDevice, Qt QRhi D3D12) build on.
- Acceptance check: `python work/experiments/renderer-sb/handoff/check_handoff.py` exits 0. It builds the test out of tree and runs `sb_handoff.exe --selftest` (CPU only).
- The GPU runs happen on the owner's machine afterwards (no administrator rights needed); the sandbox result is source/fixture evidence only.
- Non-goals: drawing the model, PresentMon, any framework (Godot, Qt), NVIDIA features, the Agility SDK.

## 2. Actual problem and reproduction
- The framework candidates need a known-good D3D12 handoff path before their smoke tests: the plan (section 3) requires device ownership, synchronisation, resize and teardown to work inside each framework, and the frameworks may either share our device or create their own.
- Documented D3D12 behaviour to confirm, not assume: `D3D12CreateDevice` on an adapter that already has a device in the process returns the existing device (a per-adapter singleton). A "second device" on the same adapter therefore needs either a second process or another API's device.

## 3. Environment and versions
- Base: branch `claude/renderer-sb` (this worktree's HEAD).
- Owner's machine: Windows 11, NVIDIA GeForce RTX 4070 Laptop GPU (driver 616.92) plus an integrated AMD Radeon 610M, Visual Studio Community 2026 (MSVC 14.51), Windows SDK 10.0.26100.0, CMake 4.4.3 and Ninja 1.13.2 in `tools/.venv/renderer-spike/Scripts/` (not present in this worktree).
- C++20, Windows SDK only (D3D12, DXGI, D3D11 for mode c). HLSL compiled with the SDK's `dxc.exe` if shaders are needed.
- Evidence kind in the sandbox: source/fixture only.

## 4. Necessary source and evidence
- `docs/progress/1.0/packets/renderer/E-2.4-01-sb-probe.md` (item 4) and `docs/progress/1.0/renderer-experiment-plan.md` section 3 (interop smoke tests).
- `docs/progress/1.0/packets/renderer/E-2.4-02-sa2-godot.md` and `E-2.4-03-sd-qt.md`: what the framework smoke tests will need from this path.
- `work/experiments/renderer-readiness/minimal_d3d12.cpp` and `build.cmd`: working D3D12 setup and build script on this machine.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| — | none | — | — | — |

## 6. Constraints and owned files

Owned: `work/experiments/renderer-sb/handoff/**` only. Change nothing else.

Modes of `sb_handoff.exe` (all on the high-performance hardware adapter unless `--warp`):
- `--mode same-device`: one device, a producer queue (compute or copy) and the consumer direct queue. Each iteration `n`: the producer writes a texture (RGBA8, 1024 x 1024, a pattern that encodes `n` in every texel, written by a compute shader or a clear plus a copy) into one of 3 ring textures and signals a fence with `n`; the consumer queue waits on that fence value, copies or samples the texture into a readback buffer and signals back that the slot is free. The CPU verifies every iteration's pattern after the fact. Default 1,000 iterations.
- `--mode second-device`: the same exchange, with the textures in a shared heap (`D3D12_HEAP_FLAG_SHARED`) and a shared fence (`D3D12_FENCE_FLAG_SHARED`) exported as NT handles and opened by a second device in a child process (`sb_handoff.exe --child <handles>`, handles passed with `DuplicateHandle` or by name). The child creates its own device on the same adapter (LUID passed on the command line) and acts as the consumer.
- `--mode d3d11-consumer`: the same textures and fence opened in the same process by a D3D11 device (`ID3D11Device5::OpenSharedFence`, `ID3D11Device1::OpenSharedResource1`) as the consumer.
- `--mode resize`: same-device exchange while the ring textures are recreated at a new size every 100 iterations (all fences drained first), then teardown with outstanding work in flight drained cleanly.
- Every mode also records: whether a second `D3D12CreateDevice` on the same adapter in the same process returned the same `ID3D12Device` pointer (the singleton check), the adapter LUIDs involved, timings per iteration (mean and nearest-rank p99 in ms, for information only), errors, and teardown with no live objects reported by `ID3D12DebugDevice::ReportLiveDeviceObjects` when `--debug-layer` is given.
- Output: `--out <file.json>` with format `magic600-sb-handoff-v1`: per mode `status` `pass`|`fail`|`unsupported` with the reason, iterations, verified iterations, first failing iteration, singleton result, timings. Exit 0 only if every requested mode passed or was explicitly reported `unsupported` with a reason. `--mode all` runs the four modes.
- `--selftest` (CPU only, no device, under 30 s): pattern encode and verify for a range of `n` including wraparound, the ring-slot and fence-value arithmetic, the handle-argument parsing for `--child`, and the JSON writer; prints `selftest: ok`.

Files to write under `work/experiments/renderer-sb/handoff/`:
- `CMakeLists.txt`, `build.cmd` (as `work/experiments/renderer-readiness/build.cmd`: `vswhere` with `call "%VSWHERE%"` inside `for /f`, `vcvars64.bat`, the repository's pinned CMake and Ninja from `tools\.venv\renderer-spike\Scripts\` when present, else the Visual Studio copies on `PATH`; takes an optional build directory; prints the tool versions).
- `src/*.cpp`, `src/*.h`, and `shaders/*.hlsl` if used.
- `check_handoff.py`: builds into `Path(tempfile.gettempdir()) / "m600-sb-handoff-<pid>"` created with plain `mkdir` (never `mkdtemp` or `TemporaryDirectory`: the sandbox's temp ACL breaks them), runs `sb_handoff.exe --selftest`, exits non-zero on any failure, leaves nothing inside the repository.
- `README.md`: build and run commands, what each mode proves, and what the framework smoke tests can reuse (which object is shared, who owns the device, the fence protocol, resource states at handoff).

```implement-contract
{"allowed_files": ["work/experiments/renderer-sb/handoff/*"], "acceptance_check": ["python", "work/experiments/renderer-sb/handoff/check_handoff.py"], "stop_condition": "the handoff test builds with build.cmd, sb_handoff.exe --selftest passes, the four modes and the JSON result are implemented, and README.md gives the owner's run commands"}
```

## 7. Required return format
- Implementation: changes only in the assigned worktree; final message lists the changed files, the acceptance result, what was not verified in the sandbox, and open points.
