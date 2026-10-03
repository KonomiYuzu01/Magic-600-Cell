# S-B bare Direct3D 12 probe

This is the packet SB-B experiment, using the full `600-cell-Full` assets. It is
not a product renderer. Andrey Astrelin's MPUlt is the upstream mathematical
source; the retained asset and upstream licence notices continue to apply.

From the repository root in a normal Windows terminal:

```bat
work\experiments\renderer-sb\probe\build.cmd
python work\experiments\renderer-sb\probe\check_probe.py
```

`build.cmd [<build directory>]` finds Visual Studio with `vswhere`, calls
`vcvars64.bat`, and prints the CMake, Ninja, MSVC and DXC versions. It uses the
repository's pinned `tools/.venv/renderer-spike/Scripts` tools when present,
otherwise Visual Studio's CMake and Ninja on the resulting PATH. DXC comes from
the selected Windows SDK. C++20 uses only the Windows SDK libraries; shaders are
compiled at build time to shader model 6.0. The owner environment for this packet
is Windows 11, Visual Studio Community 2026 18.10 / MSVC 14.51, SDK 10.0.26100.0,
CMake 4.4.3, Ninja 1.13.2 and Python 3.14. No downloads are performed.

`check_probe.py` creates a plain directory under the system temp directory, builds
there, runs `sb_probe.exe --selftest` with a 60-second limit, and removes that
directory. The self-test creates no device or window. It verifies every asset
digest in `assets/manifest.json`, the full mesh ranges, the exported turn and
camera files, the even/odd label SHA-256 digests, the clean and four faulty copy
fixtures, turn boundary arithmetic, and an in-memory synthetic 200-second W3
run and trace. It prints `selftest: ok` only when all checks pass.

In the restricted Windows sandbox, Ninja's child-output pipes hang even for a
one-line command. The acceptance script sets a child-only build mode that asks
Ninja for the generated command list and executes it serially. CMake's compiler
smoke test is replaced by the actual C++ compilation/link; the same four DXC
commands run. It does not change installed tools or sandbox permissions. Normal
owner builds use `cmake --build` and Ninja's usual scheduling.

## Drawing and scenes

The single method is `DrawInstanced(30480, 600, 0, 0)`: no indices, filtering or
LOD. The vertex shader fetches the vertex, sticker centre, cell frame, animation
flag and integer label through structured-buffer root SRVs. Static inputs reside
in default heaps. All 259,800 labels are uploaded on each adopted revision into
one of three default-heap buffers from a two-entry upload ring. Two frame contexts
carry their own allocator, constants, upload memory and fence. Reusing them limits
submissions to two frames in flight; no per-frame GPU drain is used.

The borderless window uses the primary monitor's current display mode. It is
topmost, so no other window covers it even when Windows refuses the foreground
request (a launcher that is not in the foreground); `run.json` `window` records
`foreground_at_trace_start`, `presents` and `presents_occluded` (presents that
returned `DXGI_STATUS_OCCLUDED`). A run with occluded presents is not representative. The
flip-discard swap chain has three R8G8B8A8_UNORM buffers. Depth is D32_FLOAT, with
less-equal comparison, no blending, no culling and no MSAA by default. Presentation
uses sync interval zero and tearing when supported. `--vsync` uses interval one.
`--msaa 4` draws into a multisampled target/depth buffer and resolves to the
single-sample backbuffer. The adapter is selected with the high-performance DXGI
preference and software is refused unless `--warp` is explicitly set.

- W1: solved labels, identity camera, no turns.
- W2: W1 with `rotate(0, 3, 0.002)` before every draw.
- W3: continuous alternating generator/inverse turns on the QPC clock, W2 camera
  rotation, full label uploads and preserved readbacks.
- W4: the same label uploads and readbacks without animation or camera rotation.

The projection and colour follow [SPEC sections 3–4](../SPEC.md). The shader
`geometry.hlsl` supplies the same projection function to the draw and compute
check. `--feature none` reserves the H-06 hook; other feature names are rejected.
No NVIDIA-specific API or feature is linked or enabled.

Example short diagnostic, from the repository root:

```bat
work\experiments\renderer-sb\probe\build\sb_probe.exe --scene w3 --duration 10 --preroll 4 --out work\loop-memory\perf\renderer\sb\diagnostic-w3
```

Options: `--scene w1|w2|w3|w4`, `--duration` (192 s), `--preroll` (4 s), `--out`,
`--run-id`, `--turn-ms` (190), `--vsync`, `--msaa 1|4`, `--inject`, repeatable
`--declare key=value`, `--geometry-check`, `--selftest`, `--warp`, and
`--debug-layer`. Declared `true`/`false` are booleans. Frame generation and
upscaling default to false because the probe has neither. Owner confirmations
such as `high_performance=true` and `discrete_gpu=true` can be added with
`--declare`; declarations are copied verbatim, never inferred from timing.
Output directories may be created, but an existing run is never overwritten.
Escape stops a diagnostic early; its short capture cannot satisfy the gate.

## Label and trace evidence

At the first frame of turn `t`, revision `t` is uploaded and bound before drawing.
The CPU applies the complete chronological source-to-destination permutations.
Phase is elapsed QPC time divided by the turn duration; theta uses signed
smoothstep. Camera rotation also runs during preroll, and labels stay solved
until the trace starts. No preroll frame is logged.

After that frame's draw, the same command list copies the buffer actually bound
to the label SRV into its preserved readback slot. Each record carries frame,
revision, actual resource id, intended resource id and slot. The readback buffer
has `ceil(duration*1000/turn_ms)+2` slots of 259,800 u32, approximately 1.1 GB at
the defaults. Allocation failure aborts before timing. Slots are never recycled.
After the trace ends and the GPU is idle, every integer is compared with the
digest-verified even or odd oracle. The check also rejects missing/duplicate
copies, mismatched bindings, skipped revisions and late first use. CPU comparison
does not enter the measured interval; uploads and GPU readback copies do.

The run directory follows [SPEC sections 5–6](../SPEC.md): `run.json` contains
markers, build identity (SHA-256 of executable plus the four DXIL blobs in fixed
order), process id, environment and exact label-check counters; `trace.jsonl`
has one `{frame,qpc,turn,phase,revision}` per trace frame, with null turn and phase
for W1, W2 and W4. The trace QPC is read after the previous Present returns and
the frame-context fence wait, before upload/draw/Present. The probe leaves
`presentmon.swap_chain` as `FILL-FROM-CSV`. Only the capture script writes
`presentmon.csv` and fills that address.

The environment records power via `GetSystemPowerStatus`, display and refresh
via the window monitor's `EnumDisplaySettingsW`, backbuffer dimensions, adapter,
UMD driver version via `CheckInterfaceSupport`, V-Sync and tearing. Local process
video-memory usage is sampled once per frame with `QueryVideoMemoryInfo`. An
unavailable UMD version is recorded explicitly. The final `probe self-timing
(not gate evidence)` line reports trace-QPC steps, using [T0+10 s,T0+190 s) for
long captures and the whole trace for shorter ones; it cannot replace PresentMon.

## Owner measurements and negative tests

On the owner's machine (RTX 4070 Laptop GPU 8 GB, driver 616.92, hybrid panel
driven by AMD Radeon 610M, 2560×1600 at 60 Hz), first run the geometry check below.
Then use an **administrator PowerShell** for PresentMon 2.6.0.0 captures:

```powershell
& .\work\experiments\renderer-sb\probe\run_scene.ps1 -Scene w3 -Runs 3 -Declare 'high_performance=true','discrete_gpu=true'
```

Repeat with W1, W2 and W4 for attribution. Each run is a new process, with a
20-second pause between runs. The script refuses an unelevated terminal, attaches
the pinned PresentMon executable to the PID with `--v1_metrics --qpc_time
--terminate_on_proc_exit`, waits for both processes, picks the swap-chain address with the most PID rows, and runs
`python tools/perf/renderer_gate.py <runs> --out <summary directory>/summary.json`.
It prints the verdict, fps, nearest-rank p99, VRAM peak and invalid reasons. Raw
captures stay private under `work/loop-memory/perf/renderer/sb/`.

One short W3 negative test per fault, each in a new run directory:

```powershell
foreach ($fault in 'corrupt-label','swap-same-colour','delay-adoption','stale-binding') {
    & .\work\experiments\renderer-sb\probe\run_scene.ps1 -Scene w3 -Runs 1 -Duration 10 -Inject $fault
}
```

The injection occurs once in turn 20 (3.8 s at the default duration).
`corrupt-label` changes one uploaded integer; `swap-same-colour` exchanges two
distinct labels of one colour class; both must produce integer mismatches.
`delay-adoption` binds the old revision for the upload frame and adopts on the
next frame; `stale-binding` also copies the stale buffer from the fault frame.
Both must produce a failed label check, with late or binding/copy violations.
The probe exits 2 for a failed check but still writes its run. A requested fault
that was never reached also fails the check. These short runs are additionally
refused by the gate as short captures. A passing negative test is a failure of
the probe; never publish it as correctness evidence.

## Geometry check

```bat
work\experiments\renderer-sb\probe\build\sb_probe.exe --geometry-check --out work\loop-memory\perf\renderer\sb\geometry
```

This creates no window or timing run. For three cameras and start/mid/end poses,
a compute shader projects the SPEC sample and compares NDC x/y and clip w to
`reference/<camera>_<pose>.f32` with `1e-4 + 1e-4*abs(reference)` tolerance. The
sample file must match the SPEC rule; without it the rule is generated. A separate
offscreen W1 instanced draw increments a vertex-shader UAV counter and requires
30,480 invocations for every one of the 600 cells. `geometry_check.json` records
all sample errors, failures and cell counts. Exit zero means overall `pass`.
Missing reference outputs produce `reference missing` / `reference-missing` and
a nonzero exit; they never fail the CPU self-test. Geometric failure takes
precedence over missing references.

The sandbox acceptance proves source/build and CPU fixtures only. GPU rendering,
GPU readback integrity, the nine geometry comparisons, invocation counts,
PresentMon captures, the owner-run negative tests, native-resolution presentation,
VRAM and W3 gate performance remain unverified until the owner's runs. No frame
rate or gate verdict is claimed here.
