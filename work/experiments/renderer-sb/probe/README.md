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
The sandbox check's actual tool versions are recorded in `report.md`; the build
uses the installed Visual Studio CMake fallback when the pinned venv is absent.

`check_probe.py` creates a disposable `build-check-<pid>` directory in this probe, builds
there, runs `sb_probe.exe --selftest` with a 60-second limit, and removes that
directory. The self-test creates no device or window. It verifies every asset
digest in `assets/manifest.json`, the full mesh ranges, the exported turn and
camera files, the even/odd label SHA-256 digests, the clean and four faulty copy
fixtures, turn boundary arithmetic, and an in-memory synthetic 200-second W3
run and trace. It also checks feature options, frame-driven W3f states across two
cycles, camera/display metadata and synthetic effect/sort comparisons. It prints
`selftest: ok` only when all checks pass. Keeping the build inside the packet's
directory permits its executable under Windows Application Control; system temp
was refused by that policy in the sandbox.

In the restricted Windows sandbox, Ninja's child-output pipes hang even for a
one-line command. The acceptance script sets a child-only build mode that asks
Ninja for the generated command list and executes it serially. CMake's compiler
smoke test is replaced by the actual C++ compilation/link; the same DXC
commands run. It does not change installed tools or sandbox permissions. Normal
owner builds use `cmake --build` and Ninja's usual scheduling.

## Drawing and scenes

The baseline method is `DrawInstanced(30480, 600, 0, 0)`, with full detail.
Transparency uses a sorted index list with the same vertices; no feature filters
geometry or adds LOD. The vertex shader fetches the vertex, shrink anchor, cell frame, animation
flag and integer label through structured-buffer root SRVs. Static inputs reside
in default heaps. All 259,800 labels are uploaded on each adopted revision into
one of three default-heap buffers from a two-entry upload ring. Two frame contexts
carry their own allocator, constants, upload memory and fence. Reusing them limits
submissions to two frames in flight; no per-frame GPU drain is used.

The shrink anchors are the area-weighted triangle centroids defined in
[SPEC section 3](../SPEC.md), computed from the unchanged mesh in double precision
and rounded once to float32. The retained `centers` buffer stores these 433
anchors, and the CPU self-test checks their pinned SHA-256 digest.

The borderless window uses the primary monitor's current display mode. It is
topmost and asks for the foreground. A flip-model `Present` never
returns `DXGI_STATUS_OCCLUDED`, so every 100 ms of the trace the probe samples the
window itself: visible, not minimised, not cloaked (`DWMWA_CLOAKED`), not covered
(the centre and four inner points hit-test to the window) and in the foreground.
`run.json` `window` records `foreground_at_trace_start`, `samples`,
`samples_not_visible`, `samples_covered`, `samples_not_foreground`,
`visible_throughout`, `foreground_throughout` and `presents`. A run that failed any
sample exits 3 and is no gate evidence. The capture script also refuses a run in
which some second of the gate interval [T0+10 s, T0+190 s) has no present that
PresentMon saw displayed (`Dropped` = 0).

These checks are safeguards, not proof of continuous full-area visibility.
`visible_throughout` means only that every collected sample passed. A disabled
window (which `WindowFromPoint` skips) or coverage between two samples can escape
them, and one displayed present per second does not show that the whole window was
visible. Formal gate captures are therefore owner-attended (Astra ruling
`20261003T033021Z-274f20af`): the owner watches each capture throughout, turns off
avoidable overlays, and discards any interrupted, visibly obstructed or
insufficiently observed run even if the script accepted it. An unattended run is no
gate evidence. The
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
- W3f: W3's animation and label path stepped by frame number, for paired feature
  costs. Preroll holds the identity camera and turn angle zero. For trace frame k,
  the camera resets when k modulo n is zero, then rotates once; `camera` is
  (k modulo n) + 1. Turn index is k / T (integer division), phase is (k modulo T)
  / T, and the alternating angle uses the same smoothstep. T defaults to 157
  (`--turn-frames`); n defaults to 3140 (`--cycle-frames`). T must be at least 2;
  n must be a positive multiple of 2T and at most 2^53, so that `run.json` records
  both counts exactly. These options require W3f; injection is
  refused there. W3f is cost evidence only and never gate evidence.

The projection and colour follow [SPEC sections 3–4](../SPEC.md). The shader
`geometry.hlsl` supplies the same projection function to the draw and compute
check. `--feature none` is the unchanged baseline. Features use separate shaders,
pipelines and resources, created only for the selected feature; every sticker's
colour still comes from the bound label buffer. See the H-06 procedure below.
No NVIDIA-specific API or feature is linked or enabled.

## H-06 visual features and measurement

`--feature` and the capture script's `-Feature` accept `none` (default),
`no-gaps`, `outlines`, `transparency`, `fog`, `dof`, `ao` and `msaa4`. Completion
and implementation details are in [report.md](report.md). An unfinished feature
is refused as `not implemented`; it has no estimated cost.

- `no-gaps`: sticker shrink 1.0 instead of 0.82, using the baseline shaders.
  The cost of gaps is T(none) minus T(no-gaps).
- `outlines`: dark anti-aliased feature edges about 1.5 px wide at every depth.
  Open, non-coplanar and three-or-more-triangle edges are included; coplanar
  triangulation diagonals are excluded. Geometry stays at full detail.
- `transparency`: alpha 0.6 on every sticker, standard alpha blending and depth
  writes off. All 259,800 centres are sorted back to front on the GPU every frame,
  including the turn and camera transforms. Variable vertex counts are preserved;
  every triangle, including degenerate ones, is drawn exactly once.
- `fog`: per-pixel exponential fog by view depth towards (0.13, 0.145, 0.16).
- `dof`: native-resolution depth-of-field gather from readable depth, focused at
  the model centre, with an 8 px maximum radius at 1600 px height, scaled by height.
- `ao`: native-resolution screen-space ambient occlusion, 16 view-space samples
  per pixel, two depth-aware blur passes, then colour darkening.
- `msaa4`: the existing multisampled colour/depth render and 4x resolve.

Each feature run renders the same pose twice during preroll, before the trace,
with the feature off and on. Readback counts pixels with a difference strictly
greater than 2/255 in any channel. `run.json.feature_effect` records
`{changed_pixels,pixels,status}` and requires more than 0.1% of pixels changed.
Transparency also reads its GPU keys/ids once; `sort_check` requires a permutation
of 0..259,799 and descending view depth. Either failure prints its reason, exits
**4**, and writes no `run.json` or trace. Exit 1 remains an error, 2 a label failure,
and 3 a visibility/foreground failure. The check frames are not presented.
`--snapshot <directory>` saves them as `feature-off.png` and `feature-on.png`
using Windows Imaging Component; these private inspection images must never be
committed. A snapshot error never changes a failed check's exit 4; after passing
checks it is an error (exit 1). The baseline runs no feature check and allocates no feature resources.
With `none`, `--snapshot` has no effect because there is no check pair to save.

This build has a new identity, which hashes every compiled shader as well as the
executable. Measure `none` again with this build; the previously gated W3 result
belongs only to build `2b5bf5e6...`.

W3's camera advances once per frame while its turns follow the clock. A slower
feature samples different states in the same interval, so W3 cannot give paired
costs. Use W3f for costs and W3 for gate verdicts:

```powershell
# Three cold runs for none and every implemented feature:
& .\work\experiments\renderer-sb\probe\run_scene.ps1 -Scene w3f -Feature <name> -Runs 3 -Overlays '<configuration>' -Declare 'frame_generation=false','upscaling=false','driver_vsync=false','vendor_mode=<text>'
# Three cold W3 runs for none; repeat for each feature as time allows:
& .\work\experiments\renderer-sb\probe\run_scene.ps1 -Scene w3 -Feature <name> -Runs 3 -Overlays '<configuration>' -Declare 'frame_generation=false','upscaling=false','driver_vsync=false','vendor_mode=<text>'
python tools/perf/renderer_gate.py <all run directories> --out <gate record>
python tools/perf/feature_costs.py <same run directories> --out <table JSON> --markdown <table Markdown>
```

Every H-06 capture must explicitly declare `frame_generation`, `upscaling` and
`driver_vsync` as true/false and `vendor_mode` as text with `-Declare`. The table
treats undeclared controls as unknown and gives no cost. A verdict appears only
where three valid W3 runs exist; W3f supplies costs only. The table tool is owned
by packet H6-T. The capture script uses the default W3f sequence, includes a
non-baseline feature in run and summary directory names, refuses injection with
features, and treats exit 4 as failed evidence. Administrator, session ownership,
cold-process and operator-observation rules below still apply.

Example short diagnostic, from the repository root:

```bat
work\experiments\renderer-sb\probe\build\sb_probe.exe --scene w3 --duration 10 --preroll 4 --out work\loop-memory\perf\renderer\sb\diagnostic-w3
```

Options: `--scene w1|w2|w3|w4|w3f`, `--duration` (192 s), `--preroll` (4 s), `--out`,
`--run-id`, `--turn-ms` (190), `--vsync`, `--msaa 1|4`, `--inject`, repeatable
`--declare key=value`, `--geometry-check`, `--selftest`, `--warp`, and
`--debug-layer`, `--feature`, `--snapshot <directory>`, and W3f's `--turn-frames`
and `--cycle-frames`. `--msaa 4` is a synonym for `--feature msaa4`.
Only one feature can run at once. MSAA 4 with any feature besides none/msaa4 is
refused, as is any feature besides none with geometry checking or injection.
Every scene accepts a feature. Declared `true`/`false` are booleans. Frame generation and
upscaling default to false because the probe has neither. Owner confirmations
such as `high_performance=true` and `discrete_gpu=true` can be added with
`--declare`; declarations are copied verbatim, never inferred from timing.
Output directories may be created, but an existing run is never overwritten.
Escape stops a diagnostic early; its short capture cannot satisfy the gate.

## Label and trace evidence

At the first frame of turn `t`, revision `t` is uploaded and bound before drawing.
The CPU applies the complete chronological source-to-destination permutations.
Phase is elapsed QPC time divided by the turn duration; theta uses signed
smoothstep. In W2 and W3, one fixed 0.002 rad camera step in plane (0, 3) is applied
before every rendered frame, including preroll. W3f preroll advances no state.
Labels stay solved until the trace starts. No preroll frame is logged.

After that frame's draw, the same command list copies the buffer actually bound
to the label SRV into its preserved readback slot. Each record carries frame,
revision, actual resource id, intended resource id and slot. The readback buffer
has `ceil(duration*1000/turn_ms)+2` slots of 259,800 u32, approximately 1.1 GB at
the defaults. Allocation failure aborts before timing. Slots are never recycled.
W3f uses the same copy operation and extends preserved storage in equal chunks
when needed: its frame-driven turn count has no clock-derived allocation bound.
After the trace ends and the GPU is idle, every integer is compared with the
digest-verified even or odd oracle. The check also rejects missing/duplicate
copies, mismatched bindings, skipped revisions and late first use. CPU comparison
does not enter the measured interval; uploads and GPU readback copies do.

The run directory follows [SPEC sections 5–6](../SPEC.md): `run.json` contains
markers, build identity (SHA-256 of executable plus all DXIL blobs in fixed
order), process id, environment and exact label-check counters; `trace.jsonl`
has one `{frame,qpc,turn,phase,revision,camera}` per trace frame, with null turn and phase
for W1, W2 and W4. The trace QPC is read after the previous Present returns and
the frame-context fence wait, before upload/draw/Present. The probe leaves
`presentmon.swap_chain` as `FILL-FROM-CSV`. Only the capture script writes
`presentmon.csv` and fills that address. The run format remains
`magic600-renderer-run-v1`; a missing `feature` means `none`. A top-level `camera`
records `{plane:[0,3],step_rad:0.002,per:"frame"}` in W2/W3/W3f, otherwise
`{plane:null,step_rad:0,per:"none"}`. W3f also records `turn_frames` and
`cycle_frames`. The display is kept on with `SetThreadExecutionState` during the
run and the request is reset at exit; `window.display_required` is true.

The environment uses the keys the gate summary publishes. `power_source` is
sampled with `GetSystemPowerStatus` every 100 ms of the trace: `mains` or `battery`
only when every sample agrees, otherwise `changed` or `unknown` (the gate accepts
only `mains`). `power_mode` is the Windows effective power mode from
`PowerRegisterForEffectivePowerModeNotifications` (for example `max_performance` for
Best performance), or `changed` if it changed during the trace. `presenting_adapter`
names the rendering adapter, its UMD driver version (`CheckInterfaceSupport`) and
whether that adapter drives the window's display (a discrete-only or MUX mode) or
another adapter does (a hybrid copy). `display` carries the window monitor's mode and
refresh rate (`EnumDisplaySettingsW`), `backbuffer` its dimensions and
`presentation_interval` the sync interval; V-Sync, tearing and the raw adapter and
driver fields are kept beside them. Local process
video-memory usage is sampled once per frame with `QueryVideoMemoryInfo`. An
unavailable UMD version is recorded explicitly. The final `probe self-timing
(not gate evidence)` line reports trace-QPC steps, using [T0+10 s,T0+190 s) for
long captures and the whole trace for shorter ones; it cannot replace PresentMon.

## Owner measurements and negative tests

On the owner's machine (RTX 4070 Laptop GPU 8 GB, driver 616.92, 2560×1600 at
60 Hz), first run the geometry check below. Then use an **administrator
PowerShell** for PresentMon 2.6.0.0 captures:

```powershell
& .\work\experiments\renderer-sb\probe\run_scene.ps1 -Scene w3 -Runs 3 -Overlays '<none running, or the overlays running>' -Declare 'vendor_mode=<GPU and performance mode set in the vendor software>'
```

Owner declarations go into `declared` with the keys the summary publishes:
`vendor_mode`, `frame_generation`, `driver_vsync` and `overlays`. Frame generation
and upscaling default to `false`; power source, power mode, adapter and display are
measured, not declared. `-Overlays` states the overlay configuration and is required
for every run except fault injections. After each run the script asks whether the
owner watched the whole run with nothing covering the probe window; only `yes` keeps
the run. `declared.overlays` then records the configuration, that the confirmation
is operator-declared and was given after the run, and the sampling limitation above;
the gate summary publishes it. A run without the confirmation is refused, so an
intention stated before the run is never recorded as an observation.

Repeat with W1, W2 and W4 for attribution. Each run is a new process, with a
20-second pause between runs. The script refuses an unelevated terminal and a
second invocation: it holds a global mutex from its first trace-session check
through its last cleanup, so no other invocation can start, stop or remove a
session under its name meanwhile. It refuses to start a run while a trace session
named `PresentMon` or `magic600-sb-capture` is running (one left by a killed
capture adds tracing work to every present). It attaches the pinned PresentMon
executable to the PID with `--v1_metrics --qpc_time` and the session name
`magic600-sb-capture`, and waits for the probe, at most its preroll and duration
plus 300 s. PresentMon 2.6 handles a target's exit only when a later present
arrives, so `--terminate_on_proc_exit` never fires after the probe's last frame;
the script instead stops the session with PresentMon's
`--terminate_existing_session`, waits at most 60 s for PresentMon to exit 0, and
requires the session to be gone and the CSV to end with a complete row. On any
failure its cleanup stops the session with PresentMon, then with `logman stop`;
each step is guarded, each helper is bounded to 30 s plus 5 s for a kill, and it
warns with the stop command if the session may still run. It then
picks the swap-chain address with the most PID rows, prints the gate interval's
present count, presents not displayed, seconds without a displayed present and the
present modes, and runs
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
