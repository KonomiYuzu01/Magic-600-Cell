# SA2 Godot 4.7.2 .NET interop (levels 1 and 2)

E-2.4-02 level 1: a synthetic sequence-numbered RGBA8 texture on Godot's
own D3D12 device, displayed by Godot and decoded from its composited viewport.
There is no geometry port, personal session, product UI or performance claim.
The immutable interfaces are [sa2_interop.h](native/include/sa2_interop.h)
(ABI 2, retaining the level 1 exports) and [code_layout.json](code_layout.json).
The native producer DLL and self-test are built from [native/](native/README.md).

## Prerequisites and commands

Use Windows with the already installed D3D12 debug layer, the approved Visual
Studio C++ tools, and Godot **4.7.2 .NET**, exact version
`4.7.2.stable.mono.official.ed1daf0bf`. The owner's pinned .NET SDK is 10.0.401
with the .NET 8 targeting pack. No tool installation, administrator operation
or download occurs in this runner. Godot uses the OS D3D12 runtime; this test
does not add an Agility SDK.

Run only on an idle machine, with mains power and the intended discrete GPU.
Never run during PresentMon captures, builds or Codex implementation calls.
Do not minimise every Godot window during a run. The expected adapter is
`NVIDIA GeForce RTX 4070 Laptop GPU`; `--expect-adapter` changes that explicit
expectation for separately identified runs.

From the repository root in PowerShell:

```powershell
# Portable source/fixture acceptance: no Godot, .NET build or GPU.
python work/experiments/renderer-sa2/check_project.py

# Prints commands with placeholders; creates no files or processes.
python work/experiments/renderer-sa2/run_smoke.py --dry-run
python work/experiments/renderer-sa2/run_smoke.py --dry-run --device-loss

# Full R0-R12 matrix, including the intentional exit-3 drain probe.
python work/experiments/renderer-sa2/run_smoke.py

# Same matrix plus the explicit device-removal probe R13.
python work/experiments/renderer-sa2/run_smoke.py --device-loss

# A focused subset; --only selects exactly these runs in matrix order.
python work/experiments/renderer-sa2/run_smoke.py --only R2,R5

# Reuse the newest private build only if source and binary digests still match.
python work/experiments/renderer-sa2/run_smoke.py --skip-build --only R2,R5

# Build, run, and write the sanitized public summary for review.
python work/experiments/renderer-sa2/run_smoke.py --write-summary

# R10 control: a blank project with R10's engine flags, three runs per render-thread model.
python work/experiments/renderer-sa2/blank_control.py --write-summary
```

The runner locates exactly one console executable under the installed WinGet
Godot Mono package, or accepts `--godot <exe>`. It checks `--version` before
building. The local NuGet folder is `GodotSharp/Tools/nupkgs/` next to the exe;
the project's `nuget.config` clears inherited sources and uses only
`%M600_GODOT_NUPKGS%`. It never uses nuget.org. The private package cache must
contain exactly Godot.NET.Sdk, Godot.SourceGenerators, GodotSharp and
GodotSharpEditor, each version 4.7.2.

The build commands used by the runner are:

```text
cmd.exe /d /c call native\build.cmd <build>\native
dotnet build SA2Smoke.csproj -c Debug -nologo -nodeReuse:false -p:UseSharedCompilation=false
```

`<build>` is `work/sa2b/<run stamp>/` in the ignored work tree: cl.exe is not
long-path aware, and CMake's try-compile objects under the private output root
pass 260 characters in a long checkout path. The runner refuses a native build
path longer than 150 characters. Logs and results stay in the private output
root. The native build runs with this experiment as its working directory. The
.NET build runs in the project copy under `<build>`. `NUGET_PACKAGES`, `DOTNET_CLI_HOME` and
the HTTP cache are private; telemetry, first-time setup, workload notification
and shared compilation are disabled. The source project's `.godot/`, `bin/`
and `obj/` are neither copied nor used. There is no editor/import invocation
and no release template build.

## Level 2: S-B scenes on Godot's device

Packet L2-G implements [HARNESS.md](../renderer-l2-packets/HARNESS.md)
sections 2 to 6 for candidate `sa2`, and its Godot preparation step. The packet's
three amendments apply: exclusive fullscreen covers the exact monitor rectangle;
engine arguments come from the Windows process command line; the finalizer
checks the recorded adapter against its expected adapter. The level 1 smoke
mode and its project settings remain unchanged, with ABI 2 bindings for all
27 exports and runtime guards for the four structures (48, 28, 128 and 32 bytes).

[Level2.cs](project/Level2.cs) is a separate controller, selected by `--l2-mode`
among Godot's user arguments after `--`. It displays only the DLL's scene texture,
on the primary monitor, with exclusive fullscreen, topmost, content scaling
disabled, 3D scale 1, the cursor hidden and vsync disabled. It waits for two
consecutive frames with equal physical display, client, displayed and framework
surface sizes before creating the ring. Failure to settle within five seconds
exits 1. The ring is fixed for the rest of the process, including size changes
during the trace. Size samples record those changes for the finalizer.

Defaults are R2: `export`, `same`, `tracked`, `match`, render thread `safe`, three
warm-up frames. Warm-up establishes Godot's resource tracking before scene load.
Its shown-fence value is zero, so scene production can start at frame 1. The
warm-up ends with a Godot flush and native drain before scene load; this
prevents the own queue from treating fence zero as a completed warm-up read. Each
scene frame signals free `f-1`, produces slot `f%3` once, waits ready `f`, and
queues Godot's draw/present. The next render callback marks that preceding
frame shown, before signalling its free fence. `FramePreDraw` checks that the
previous iteration drew; the short PresentMon capture must confirm one trace
entry per present interval. Teardown clears wrappers on the main thread,
ends the trace, flushes Godot, drains, writes the DLL outputs on a completed
run, unloads and unregisters, frees RIDs, flushes/drains deferred destruction,
releases imported resources and detaches. All DLL and RD work uses
`CallOnRenderThread`. The display request is released on its requesting thread.

Run mode uses QPC for preroll (4,000 ms), the foreground wait (at most five
seconds), trace (192,000 ms) and 100 ms condition samples. Enforce mode stops at
the first invisible, covered or non-foreground sample, with exit 3 and no
`scene_write_run`. Record mode continues and preserves the counts. Power source
is derived from all samples, including unknown samples, and effective power
mode uses S-B's names and change rule. Geometry mode defaults to loading W1
when no scene is supplied; the DLL checks all reference cameras on its own
offscreen targets. It takes no condition samples and starts no trace.

The app writes only `harness.json` and requests the DLL's `native.json`,
`trace.jsonl` or `geometry.json`. The shared L2-F finalizer writes gate records.
Harness output uses exclusive `harness.json.tmp`, flush and rename without
overwrite. A directory containing `godot.log` or `presentmon.csv` is accepted;
the five reserved app/DLL names are refused. Usage failures write a null-filled
harness when a single usable `--l2-out` can be recovered. Esc and window close
follow the same teardown and write path. Exit codes are 0 (outputs written),
1 (usage/framework/device/DLL/I/O failure), 2 (written label/geometry check
failure), and 3 (enforce condition failure). Unknown failure-time facts stay
null, with the required object shapes intact.

On an unconfirmed drain the DLL ends the process with `TerminateProcess` and
exit code 3, as its header requires, so the managed exit writer never runs and
no `harness.json` is written; a framework process abort also leaves none. These
are the exceptions to the every-exit harness rule. The runner stops the series
on every exit 3 and names both causes, and the finalizer refuses a run without
`harness.json` (`harness`). The app refuses the intentional drain injection. A
pending harness written in advance is not an option, because the no-overwrite
rule would keep it from being replaced.

Level 2 options, all passed after `--`:

| Option | Values / default |
| --- | --- |
| `--l2-mode` | required: `run` or `geometry` |
| `--l2-scene` | `w1` through `w4`, required for `run` |
| `--l2-out` | required existing directory without the five reserved outputs |
| `--l2-run-id` | required `[A-Za-z0-9][A-Za-z0-9._-]{0,127}` |
| `--l2-dll` | required absolute path of `sa2_interop.dll` |
| `--l2-trace-ms` | 1,000 to 3,600,000; default 192,000 |
| `--l2-preroll-ms` | 0 to 60,000; default 4,000 |
| `--l2-turn-ms` | W3 only: finite, greater than 0 and at most 10,000; default 190 |
| `--l2-inject` | W3/W4 run only: `corrupt-label`, `swap-same-colour`, `delay-adoption`, `stale-binding` |
| `--l2-declare` | repeatable `key=value`; lowercase `true`/`false` are booleans; `overlays` and duplicate keys are refused |
| `--l2-gpu-validation` | `0` or `1`, default `0`; engine `--gpu-validation` must agree |
| `--l2-conditions` | `enforce` (default) or `record` |
| `--l2-no-vram` | flag, passes `SA2_SCENE_NO_VRAM` |
| `--l2-debug-half-target` | flag, halves only registered slot width/height, rounded down |

Meaningful level 1 options keep their `--sa2-` names: route, queue, handover,
barriers, warmup, render-thread and timeout-ms (fixed at 5,000 in level 2).
`--sa2-gpu-validation`, if supplied, must agree with `--l2-gpu-validation`.
The `rd-compute` baseline cannot draw DLL scenes and is refused in level 2.
Frame-count, resize, verify, device-loss, smoke output/DLL and drain-injection
options are refused. An inherited unconfirmed-drain injection is also refused
before attaching. Experimental import/handover/barrier values retain their
level 1 meanings; R2 is the prepared configuration. The imported shared-view
route still checks the editor binary's refusal.

[prepare_l2.py](prepare_l2.py) imports the level 1 offline DLL/.NET build recipe,
the 150-character native-build path limit and isolated NuGet/.NET environment.
It copies the project into a new `work/sa2b/<stamp>/`, builds Debug, and prints
that directory. Logs and caches use the same private root as level 1. It locates
the pinned WinGet package without executing either Godot executable, and the
app checks the exact engine version when it starts. It writes `launch.json`
with `magic600-l2-launch-v1`: the non-console executable, D3D12, no vsync,
render-thread `safe`, per-run `--log-file {out}\godot.log`, validation-only
`--gpu-validation`, the `--` separator, and a whitelist of isolated environment
values (the drain injection is removed). It passes no `--gpu-index`.

`--skip-build` reuses only the newest prepared level 2 build whose source,
Godot executable, DLL, shader, assembly and copied-project digests still match
`l2-build-identity.json`; otherwise it refuses. Unchanged `launch.json` is reused;
different launch settings require a new build. It launches no Godot, editor,
import or GPU process and changes no installed tool. No whole inherited
environment is copied into the launch record.

The configuration records raw framework facts and engine arguments as one
JSON-array string. The latter comes from `GetCommandLineW` split with
`CommandLineToArgvW`, dropping the executable, `--path`/`--log-file` and their
values, `--gpu-validation`, `--gpu-abort` and all user arguments from `--` onward.
Run options and paths therefore do not change the composite identity. `files`
records the loaded DLL, actual running exe and loaded assembly, `project.godot`,
`Main.tscn` and its external `Smoke.cs` resource. The DLL supplies shader digests.

`assembly_mvid` records
`typeof(Smoke).Assembly.ManifestModule.ModuleVersionId.ToString("D")` beside
`godot:assembly`, for the finalizer to compare with the metadata of the assembly
bytes it hashes. `files.dll` stays null until `new Native(...)` returns, then
records the full `GetModuleFileNameW` path for the `NativeLibrary.Load` handle.
A zero or truncated module path fails the run with `framework-modules`.
Both facts stay null until recorded, including in usage records.

The full-rectangle `TextureRect` maps the texture directly, with no fit/crop or
aspect-preservation transform (`texture_stretch: none`); the raw enum is also
recorded. Half-target injection keeps those framework settings and displayed
extent, so the intended refusal is `size-mismatch` alone.

Owner-machine order from HARNESS section 11, once L2-F is integrated:

```powershell
# Source/fixture check only. Preparation builds but starts no Godot process.
python work/experiments/renderer-sa2/check_project.py
$build = python work/experiments/renderer-sa2/prepare_l2.py
# Optional, after a successful preparation with unchanged source and binaries:
# $build = python work/experiments/renderer-sa2/prepare_l2.py --skip-build
$runner = 'work/experiments/renderer-l2/run_scene.ps1'

# Four validation records, then the geometry record; no administrator rights.
& $runner -Candidate sa2 -Build $build -Validation
& $runner -Candidate sa2 -Build $build -Geometry

# Steps below: administrator PowerShell, idle machine, mains power.
# These declarations require the operator to have disabled both features.
$declare = @('frame_generation=false', 'upscaling=false')
# State the overlays that are actually running, for example 'none running'.
$overlays = '<none running, or the overlays running>'
& $runner -Candidate sa2 -Build $build -Short -Scene w3 -Declare $declare
# Each must be refused for exactly its named reason.
& $runner -Candidate sa2 -Build $build -Short -Scene w3 -Declare $declare -DebugHalfTarget
& $runner -Candidate sa2 -Build $build -Short -Scene w3 -Declare $declare -NoVram

# Preliminary unless owner-attended. After each run, answer the runner's
# question whether you watched all of it.
& $runner -Candidate sa2 -Build $build -Scene w1 -Runs 1 -Overlays $overlays -Declare $declare
& $runner -Candidate sa2 -Build $build -Scene w2 -Runs 1 -Overlays $overlays -Declare $declare
& $runner -Candidate sa2 -Build $build -Scene w4 -Runs 1 -Overlays $overlays -Declare $declare

# Three owner-attended cold W3 processes; the runner judges the series.
& $runner -Candidate sa2 -Build $build -Scene w3 -Runs 3 -Overlays $overlays -Declare $declare
```

The sandbox check preserves every level 1 check and adds ABI 2 layout/export
checks, static checks of sections 2 to 6, planted defects, a real fixture write
of `launch.json`, and digest-change refusal fixtures. It starts only Python
processes and removes its plain-mkdir system-temp fixture directory on every
exit. It does not compile C#, start Godot or measure GPU/window behavior.

Godot 4.7.2 dependencies for level 2 and their runtime checks:

| Fact | Evidence / use | Owner-machine check in HARNESS |
| --- | --- | --- |
| G1 | Source-verified by the integrator: exclusive fullscreen is the exact monitor rectangle; ordinary fullscreen extends 2 pixels and clips them. Use exclusive fullscreen. This supersedes section 2's earlier 1-pixel explanation. | Section 7 `size-mismatch`, using section 2's five measured physical sizes. |
| G2 | Source-verified: `AlwaysOnTop` sets `HWND_TOPMOST`. | The app checks `WS_EX_TOPMOST` before building; failure is `app-exit`; section 5 covered samples feed `conditions`. |
| G3 | Source-verified: RD screen width/height report the window surface; resize reaches the swap chain at the next frame. | Two settled frames before the ring, section 7 `size-mismatch` including size-change samples, and the half-target short refusal. |
| G4 | Source-verified: `OS.GetCmdlineArgs()` omits engine-consumed arguments. The amended section 6 uses native process argv instead. | Inspect recorded `configuration.engine_arguments` against `launch.json`; section 8 checks settings identity and stability across run options. No dependency on `OS.GetCmdlineArgs()` remains. |
| G5 | Source-verified: `--disable-vsync` gives `VSYNC_DISABLED`, sync interval 0 and tearing when supported. | Actual `WindowGetVsyncMode()` is recorded and required disabled; section 7 `sync-interval` and the PresentMon tearing fact. |
| G6 | Source-verified: the D3D12 adapter order prefers discrete unless a GPU index or Windows preference overrides it. | Record `GetVideoAdapterName()` without judging; the amended section 7 `adapter` refusal checks the expected RTX 4070 name. |
| G7 | Source-verified at the pinned tag: the console executable starts a suspended non-console child in a kill-on-close job and returns its exit code. | Prepare launches the non-console exe itself; section 7 `harness` checks the app PID equals the launched PID, then `presentmon`/`swap-chain` select it. No wrapper was run in the sandbox. |
| S1/S2/S4 | Existing level 1 source facts: import ownership/initial tracking, barrier matching, and exported texture pointer stability. | Warm-up and pointer/wrap checks, section 7 `validation` on W1 to W4; rerun R2 with ABI 2 for preserved smoke behavior. |
| S3 | Existing source fact: editor builds reject shared views of foreign allocations. | Explicit imported-wrapper check exits 1; section 7 `app-exit`. |
| S5 | Existing source fact: GPU validation enables the debug layer/callback. | Section 7 `validation` requires actual debug layer 1 and the full native counters. |
| S6 | Existing source fact: safe callbacks run immediately; separate render callbacks and draws are FIFO, wrapping is deferred, pre-draw is synchronous. | Measured thread model and pre-draw guard, section 7 `trace-steps` in the short capture, then validation runs. |
| S7 | Existing source fact used by the preserved smoke path: RGBA8 viewport readback is tightly packed, captures the previous frame and changes RID on resize. | R2's two-corner decode, readback coverage and resize checks with ABI 2; level 2 uses the DLL's checks. |
| S8 | Existing source fact: drawn frames submit at swap-buffers; synchronous readback submits and disposes deferred freed textures. | Bounded drains and native Release counts; teardown failures give `app-exit`, and section 7 `validation` checks the resulting counters. |
| Additional assumptions | Window/scaling and TextureRect C# setters report effective values; disabled content scaling gives physical layout pixels; default Windows DPI awareness is per-monitor v2. Newly used C# API signatures have not been compiled here. | Raw scaling and physical rectangle measurements feed section 7 `scaling`/`size-mismatch`; render-thread DPI is required v2 before attach (`app-exit`). The owner build checks signatures; validation and short capture check behavior. |

Every level 2 Godot, .NET compilation, device, GPU, window, foreground,
validation, geometry, PresentMon, shutdown and performance check remains
unverified in the sandbox. Source/fixture success is the L2-G packet's
acceptance evidence, not a renderer gate or performance result.

## Run matrix

Each row is a fresh process, in this order. Default warm-up is three iterations
for tracked native routes, one use per ring slot; compute and render-target
probes use zero. Verification runs every 50 run frames. All native CPU fence
waits and drains use 5,000 ms.

| Run | Route | Queue | Handover | DLL barriers | Validation | Render thread | Frames | Resize every | Expected / purpose |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| R0 | native self-test, hardware/debug | — | — | both | on | — | — | — | Pass before framework runs; failure stops the matrix |
| R1 | rd-compute | same | — | match | on | safe | 1200 | 200 | Pass: Godot compute, display, viewport decode and resize baseline |
| R2 | export | same | tracked | match | on | safe | 1200 | 200 | Pass: zero copy on Godot's device and queue |
| R3 | export | own | tracked | match | on | safe | 1200 | 200 | Pass: zero copy and ready/free fences across queues |
| R4 | import-copy | same | tracked | match | on | safe | 1200 | 200 | Pass: imported native resource copied by Godot |
| R5 | import-copy | own | tracked | match | on | safe | 1200 | 200 | Pass: import/copy with both queues |
| R6 | import-texture2drd | same | tracked | match | on | safe | 300 | 0 | Unsupported `texture2drd-refused` in the editor binary; complete teardown |
| R7 | export | same | render-target | match | on | safe | 300 | 0 | Recorded: render-target handover without tracker warm-up |
| R8 | import-copy | same | render-target | match | on | safe | 300 | 0 | Recorded: PR #43 import-state probe without warm-up |
| R9 | export | own | tracked | match | off | safe | 3000 | 500 | Pass: longer run without validation |
| R10 | export | own | tracked | match | on | separate | 1200 | 200 | Pass: render-thread ordering and callback handling |
| R11 | export | own | tracked | legacy | on | safe | 1200 | 0 | Recorded: producer legacy barriers, Godot's chosen barrier API unchanged |
| R12 | export | own | tracked | match | on | safe | 120 | 0 | Exit 3, exact native drain diagnostic and pending result |
| R13 | export | own | tracked | match | on | safe | 400 | 0 | Only with `--device-loss`; removal at run frame 200, record engine/native outcome |

R12 alone gets `M600_SA2_INJECT_UNCONFIRMED_DRAIN=1`. The runner removes an
inherited injection variable, in any letter case, from all other runs. Its required stderr line is
`sa2: drain not confirmed; ending the process with exit code 3`.

Every run has a 300-second limit; R13 has 120 seconds. A timeout kills only
that run's process tree with `taskkill /PID <pid> /T /F`. R13 is as expected
when its process terminates within that limit, whatever its exit code and
whether the result exists: Godot may abort on device removal. If an adapter is
reported, a different adapter still makes the run unexpected. R0 and an
aborted R13 can lack adapter metadata; this is recorded as unknown, never
invented. Normal Godot runs require the exact expected adapter.

`--only` omits unselected rows, including R0. Use it for focused reruns after
the native self-test has passed.

Godot consumes `--gpu-validation` and `--render-thread` itself; they never
reach the harness. The runner therefore repeats both as the required user
arguments `--sa2-gpu-validation 0|1` and `--sa2-render-thread safe|separate`,
taken from the row. A missing or invalid value prints one `sa2:` line on stderr
and exits 2. The result's `config` echoes them as `gpu_validation` and
`render_thread`. `config.render_thread_separate_observed` is measured, not
declared: `_Ready` stores `!RenderingServer.IsOnRenderThread()` on the main
thread, which is Godot's render thread in the safe model.

Judging, in addition to each row's expected status:

- R0 passes only with exit 0, format `magic600-sa2-native-selftest-v1`, every
  check `pass` or `unsupported` with a reason, and at least one `pass`.
- Every Godot run except R13 must echo its row in `config` with the exact JSON
  type and value: route, queue, handover (not for R1, whose row has none),
  barriers, frames, resize_every, gpu_validation and render_thread; and
  `render_thread_separate_observed` must be true exactly when the row's render
  thread is `separate`. Otherwise the reason is `config-mismatch`.
- A validation row needs `device.debug_layer == 1` (`validation-not-active`):
  zero validation counts prove nothing without an active debug layer.
  Validation errors/corruption in the native debug counts prevent a normal pass.
- A pass row with `resize_every > 0` needs `resize.rebuilds ==
  (frames − 1) // resize_every`. Recorded rows and R12 need `frames.run` equal
  to the row's frames; R6 and R13 are exempt.
- The runner counts stdout/stderr lines with `ERROR:` and `WARNING:`. These
  counts are recorded data, never a pass condition: the packet SA2-G pass rule
  does not gate on Godot's own log. A run with an `ERROR:` line carries the
  non-gating judge note `godot-output-errors`, which never changes its
  judgement. R6 may log Godot errors describing the expected shared-view
  refusal. R10 logs Godot's shutdown line `This function (finalize) can only be
  called from the render thread.` [blank_control.py](blank_control.py) runs a
  blank project (no harness, no DLL) with R10's engine flags under both
  render-thread models, to show whether Godot prints that line without the
  interop. It keeps raw output private and writes counts only to
  `results/sa2-blank-control-summary.json`. Warnings, including mismatching
  optimized clear values, are recorded.

## Frame, ownership and shutdown protocol

The scene stays on the main thread. All RenderingDevice and DLL work is inside
`RenderingServer.CallOnRenderThread`, including initialization, wrap checks,
warm-up, production, readbacks, rebuild and teardown. The main thread clears
and disposes `Texture2Drd` resources before the release steps. A lock guards
the result, abort flags and mailbox. No Godot or DLL call holds it: synchronous
readback can invoke pending callbacks on its calling thread.

Iteration f is the `_Process` iteration, not the count of produced frames.
Slot `f % 3`, sequence f and a 12-bit ring generation form a 64-bit CRC-protected
code. Both 64×64 corner patterns must decode to that code. Native and RD
verification compare every texel, including the fill. The compute baseline
uses an inline GLSL compute shader and a 32-byte push constant; GLSL here is a
level-1 diagnostic exception, not a product shader-language decision.

Before a native run step, Godot's queue signals free=f−1. The producer writes
the slot and signals ready=f; Godot waits for ready=f and the slot is marked
shown. Own-queue reuse waits for the slot's previous shown frame; same-queue
reuse follows queue order. This carries the S-B ready/free protocol into the
framework, with the states Godot tracks rather than a COMMON-state assumption.
Warm-up marks each used slot shown without a ready wait. Rebuild/wrap steps
also signal free after attach.

`frame_pre_draw` associates a draw with iteration f. If iteration f had no
such signal, iteration f+1 stops production, free signals and readback requests
and records `not-drawn`. Teardown flushes pending work. The run cannot pass by
continuing after a minimised window left recorded work unsubmitted.

The viewport RD RID is re-read on each render step. A request at step f sees
the composite of f−1. That frame is eligible only if it was a run frame in
the current generation, the viewport RID has not changed and it can hold the
ring. Other frames are counted as `warmup` or `transition`. Readback callbacks
capture immutable dimensions/code expectations. Teardown's first flush
completes outstanding callbacks; the final normally drawn run frame is also
requested before that flush. A normal pass needs every eligible frame verified,
at least 90% coverage of run frames, no texel mismatch and all requests completed.

Resize cycles 1280×720, 1024×640, 1440×810 and 800×600. A ring rebuild clears
the scene's texture wrappers, flushes Godot through an initialized 4×4 texture,
frees ring/display/dependent RIDs, flushes again, drains the native queues,
unregisters slots and releases imported resources. It then creates and wraps
the new ring, warms the tracker again and resumes. Each transition records its
start/end iterations and length. Export wraps also recheck that the native
resource pointers did not change.

Final teardown follows the same order, frees the flush texture, then atomically
writes a result with `teardown.phase="pending"` **before** the native drain.
Only a confirmed drain permits unregister/release/detach; a removed device is
recorded separately and permits release under the header's rules. Debug counts,
distinct IDs and masked native messages are read before detach. Successful
teardown writes `complete`; unresolved disposal/drain/detach writes `incomplete`.
Imported Release counts must be zero. An unconfirmed native drain terminates
the process inside the DLL with exit 3, leaving the pending result. Main-thread
quit returns 0 for pass/recorded/expected unsupported, 1 for fail, 2 for malformed
arguments. Exit 3 is never synthesized by C#.

The device-loss probe records a pre-removal snapshot, removal status and
GetDeviceRemovedReason when the engine permits it, and observes subsequent
draw signals before teardown. An engine abort, missing result or timeout is
visible in the runner's private record.

DLL waits are bounded. Godot's synchronous `texture_get_data` has no timeout
parameter in the supplied API and can stall on the main thread in safe mode.
The runner bounds the process lifetime. This smoke test does not establish
full product-level R-14 UI responsiveness. R-04 ordered teardown and R-17 loss
reporting likewise need the actual owner-machine evidence.

## Outputs and source identity

Builds, logs, per-run JSON and `private-summary.json` go under
`work/loop-memory/perf/renderer/sa2-smoke/<UTC stamp>/`. They contain private
paths and diagnostics and must never be committed. No result is stored in
`user://`; Godot file logging, shader caching and pipeline caching are disabled.

Source identity records HEAD, whether the source matches HEAD, and SHA-256 over
sorted experiment-relative paths followed by NUL, the file's SHA-256 in ASCII,
and newline. It covers the experiment including native source and the immutable
interface. It excludes every `results/`, `.godot/`, `bin/`, `obj/`,
`__pycache__/` and `.sandbox-build/` directory at any depth, and the manual
native build folders `native/build*/`. Since `work/` is ignored, the walked
source file set is compared with `git ls-files` under the same exclusion rule,
and `git diff --quiet HEAD` runs over this directory with `:(exclude)` and
`:(exclude,glob)` pathspecs for the same directories, so a summary written to
`results/` or a build output does not count as a source change. The DLL and
managed assembly have separate digests. Reuse refuses
source/version/binary changes; a build whose source changed during compilation
is refused.

With `--write-summary`, [smoke_summary.py](smoke_summary.py) writes
`results/sa2-smoke-summary.json` (`magic600-sa2-smoke-summary-v1`). It copies only
allowlisted source/build identity, engine/editor flag, adapter/driver version,
enhanced-barrier flag, matrix row, status/judgement/reason codes, known judge
notes, counts (including the per-run `ERROR:`/`WARNING:` output line counts),
distinct debug IDs, teardown and exit code. R0's public status is derived from
its checks by the self-test rule above, never copied from the result. It strips
messages, last errors, configuration
paths, command lines, LUIDs and vendor/device IDs. A final scan refuses private
path/environment patterns and the current user's name even in an allowed field.
Only reviewed source and this sanitized, matching-build summary may be committed.
A summary for `--only` describes exactly the selected runs, not a full-matrix pass.

## Godot API facts and runtime confirmation

These source facts were supplied in packet SA2-G from tag `4.7.2-stable`.
**Every runtime confirmation below is pending the owner's runs.** Documentation
facts and source/fixture acceptance are never counted as GPU smoke-test results.

| Fact | Source evidence | Runtime confirmation pending |
| --- | --- | --- |
| S1 | **source-verified against 4.7.2-stable**: import keeps the raw resource pointer without AddRef/Release; legacy tracking begins at RENDER_TARGET; render-graph usage begins at NONE | R4/R5, release counts and teardown; R8 probes the initial-state assumption |
| S2 | **source-verified against 4.7.2-stable**: OPTIONS12 selects enhanced barriers; generic layouts, NONE→UNDEFINED first-use discard, and last usage retained across frames | R2–R5 after warm-up; R7/R8 without it; R11 changes only the producer API |
| S3 | **source-verified against 4.7.2-stable**: Texture2DRD creates a shared view; DEBUG_ENABLED rejects a view of a foreign allocation, leaving an invalid RD RID behind the server texture | R6 checks that RID after deferred wrapping and tears down fully |
| S4 | **source-verified against 4.7.2-stable**: Godot color-attachment textures are typeless, allow RT, have black optimized clear, and initially UNDEFINED/COPY_DEST; first-use clear; native texture pointer survives wrapping | R2/R3/R9/R10, pointer recheck and warm-up |
| S5 | **source-verified against 4.7.2-stable**: GPU validation enables the debug layer and Godot's IGNORE_FILTERS callback; errors/warnings reach output | R1–R8/R10–R12, native counters plus captured Godot output |
| S6 | **source-verified against 4.7.2-stable**: safe callbacks run immediately; separate callbacks and draw queue FIFO; pre-draw is synchronous on main, post-draw deferred; Texture2DRD wrapping is deferred | R10 compared with R3; draw/step and wrap ordering |
| S7 | **source-verified against 4.7.2-stable**: non-HDR root target is RGBA8 with copy-from; async readback/callback on render thread, tightly packed bytes; step f captures f−1; resize changes the RD RID | R1–R5/R9/R10, both-corner viewport decode and resize skips |
| S8 | **source-verified against 4.7.2-stable**: submission at swap_buffers only for drawn frames; synchronous readback records copy-from, submits/stalls, invokes pending async callbacks and disposes current-frame freed textures | R1–R5/R9/R10 teardown/resize; R12 pending-before-drain failure path |

Additional C# API uses beyond packet section 3 are ordinary Node/Control
`GetNode`, `GetViewport`, `GetWindow`, `_Ready`, `_Process`; Viewport
`GetVisibleRect`/`GetTexture`; Resource `GetRid`/`Dispose`; Rid `IsValid`/equality;
`OS.IsDebugBuild`; `RenderingServer.GetCurrentRenderingMethod`/`IsOnRenderThread`;
`Callable.From` (including the byte-array callback); RDTextureFormat/RDTextureView
properties and constructors; RDShaderSource `SourceCompute`;
RDShaderSpirV `GetStageCompileError`; and RDUniform `UniformType`, `Binding`,
`AddId`. Their names/signatures were checked in the locally installed
GodotSharp 4.7.2 package metadata. The optional offline compilation checks their
C# binding availability; it does not establish runtime behavior.

Not verified: release export templates, Godot's legacy-barrier path on a device
that supports enhanced barriers, rendering geometry or labels, input, performance,
long-session stability or any renderer selection gate. R11 forces legacy barriers
only in the DLL; it cannot force Godot onto its legacy path.

## Reading PR #43 and level-1 evidence

Compare R4/R5 (imported resource initially RENDER_TARGET, then warmed and handed
back in tracked COPY_SOURCE) with R8 (every handover RENDER_TARGET, no warm-up).
Use R2/R3 versus R7 to separate Godot-owned resource first-use behavior. The
packet's source facts predict that subsequent handovers must match the tracker,
not a universal RENDER_TARGET rule. A runtime answer must cite the actual rows,
barrier API, editor/debug build, device and driver; until those rows run, the
answer is pending. A `recorded` probe is an observation, never a correctness pass.

Device/queue identity from `sa2_probe` covers ownership; R2–R5 cover both queues
and synchronization; their resized runs cover recreation; completed drains,
unregistration, Release counts and detach cover teardown; both-corner viewport
decodes cover the sequence actually composited. R12 establishes the unconfirmed
drain stop path, and R13 records device loss separately. None of this substitutes
for E-2.4-02 level 2 or PresentMon evidence.
