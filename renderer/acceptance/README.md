# C9 acceptance harness

The run harness for the renderer freeze's acceptance runs on the product core `renderer/core/` (W-J's pose path, ABI 3, kept unchanged in ABI 4): three cold runs each of W-J's S4, I_a and I_b, three cold W3 runs, and W-J's untimed geometry and lattice checks. The bars are unchanged: every run and the pooled frames at least 30 fps on average and a 99th-percentile frame time of at most 33.3 ms over [trace start + 10 s, + 190 s), peak VRAM at most about 7 GB (the gate flags peaks above 7168 MB), and a passing label check (W3) or label and pose check (W-J).

The host is S-A2 as tested: the official Godot 4.7.2 .NET editor binary running `project/` (net8.0) with the core's `sa2_interop.dll`. Where every file comes from: [PROVENANCE.md](PROVENANCE.md). The harness writes nothing under `work/experiments/`.

| File | Role |
| --- | --- |
| `project/` | the S-A2 Godot project with a W-J mode: `--l2-scene wj --l2-menu <S4, I_a, I_b> --l2-data <dir>` and the `lattice` check mode |
| `prepare_host.py` | builds the core and the project offline under `work/build/c9-acceptance/<UTC>/` and runs the core's CPU self-test; `--verify` checks a build; never starts Godot or a GPU device |
| `prepare_data.py` | runs W-J's `prepare_wj.py` in place with the engine Python (CPU only); `--check` verifies an export against the fixtures |
| `run_scene.ps1` | one cold run, or one geometry or lattice check, per invocation; `-DryRun` validates and prints the planned commands |
| `file_guard.py` | holds every loaded build file from before the launch until the finalizer has accepted the run |
| `finalize_run.py` | checks the app's, the DLL's and PresentMon's outputs and writes `run.json`, `fault-run.json` for an injected run, a check record or `refusal.json` |
| `route-b/override.cfg` | route B's window settings (the product shell's), copied into a `--route surface` build |
| `stages/` | the renderer gate's user stages (C9 U5): `vertex.6cs`, `fragment.6cs`, their `limits.json` and the array values `arrays.json`, built into a `--stages` build |

## What a run needs

- **The owner away, or watching.** A run with `-Unattended` records that nobody watched it (`environment.declared.watched` is `false` in `run.json`); without it, the runner asks after each run whether the whole run was watched with nothing covering the window.
- **A quiet machine**: mains power, no build (`cl`, `link`, `ninja`, `cmake`, `msbuild`, `dotnet`), no Godot, Blender or FFmpeg, no PresentMon overlay or other capture, no other runner, no Codex implement call, and no trace session named `PresentMon` or `magic600-*-capture`. The runner refuses to start otherwise. Close idle heavy applications as well; the runner cannot see them.
- **The foreground.** Start the runner from an interactive administrator PowerShell on the owner's desktop session, not from a background process: the Godot window must stay visible in the foreground for the whole trace, or the app exits 3 and the run is not evidence.
- **PresentMon 2.6.0.0** at `%ProgramFiles%\Intel\PresentMon\PresentMonConsoleApplication\PresentMon-2.6.0-x64.exe`.
- **Godot 4.7.2 .NET** (`Godot_v4.7.2-stable_mono_win64.exe`, the WinGet install `GodotEngine.GodotEngine.Mono`); `prepare_host.py` records its path in `launch.json`, and the app checks the version at start.
- The display at its native resolution, frame generation and upscaling off, and the RTX 4070 Laptop GPU presenting.

## Prepare (no GPU, once per source change)

From the checkout root. In a worktree without its own `tools/.venv/renderer-spike`, point `M600_RENDERER_SPIKE_SCRIPTS` at the main checkout's `tools\.venv\renderer-spike\Scripts` first.

```powershell
python -B renderer/acceptance/prepare_host.py      # prints the build directory work/build/c9-acceptance/<UTC>
python -B renderer/acceptance/prepare_data.py      # W-J exports in work/loop-memory/c9-acceptance/wj-data
$build = 'work/build/c9-acceptance/<UTC>'          # the directory prepare_host.py printed
$runner = 'renderer/acceptance/run_scene.ps1'
```

## Route B (C9 U4)

The same scenes and bars on route B, the product's presentation. `prepare_host.py --route surface` builds a host whose Godot window is transparent with vsync on (`route-b/override.cfg`, the product shell's settings). Its harness creates the core's own device on Godot's adapter (`sa2_create`), attaches the core's surface beneath the window (`sa2_surface_attach`), and draws and presents each frame on that surface (`sa2_scene_produce` with `SA2_TARGET_SURFACE`, sync interval 1). `run_scene.ps1` reads the route from the build and passes `--sa2-route surface`. The finalizer measures the core's swap chain, which the harness names in `harness.json`, and lists Godot's own chain as ignored.

```powershell
python -B renderer/acceptance/prepare_host.py --route surface   # prints work/build/c9-acceptance/<UTC>
$surface = 'work/build/c9-acceptance/<UTC>'
& $runner -DryRun -Scene w3 -Build $surface -Overlays 'none running' -Declare @('frame_generation=false','upscaling=false') -Unattended
```

The other commands below take `-Build $surface` unchanged. The app exits with `surface-transparency` when Godot does not give a transparent window. Whether the core's layer is what the screen shows in exclusive fullscreen is for the owner to see in the first route-B run.

## User stages (C9 U5)

W3 with the renderer gate's user stages: `stages/vertex.6cs` (the built-in projection, with `d4` as an array input) and `stages/fragment.6cs` (tier-1 operations only), drawn by the core's stage chain (RPI.72) in place of the built-in shaders. The bars are W3's, unchanged.

`prepare_host.py --stages renderer/acceptance/stages` adds, on either route:

- the generator `c600-stage-hlsl`, built offline under `work/build/c9-stage-tool`, which writes `vertex.hlsl`, `vertex.json`, `fragment.hlsl` and `fragment.json` into the build's `stages/` folder from the two `.6cs` files and `stages/limits.json`; `stages/arrays.json` is copied beside them (format `magic600-stage-arrays/1`: each array's values as binary32 bit patterns; the vertex `d4` must be the built-in 1.18f, `0x3f970a3d`);
- DXC's `dxcompiler.dll` and `dxil.dll` beside the core in `native/`, from `--dxc DIR` or, by default, the toolchain's `tools/.dxc/v<version>/bin/x64` (`tools/toolchain.lock.json`, id `dxc`);
- a `stages` record in the build identity: the stage digests, the HLSL, record and arrays SHA-256s, the overdraw the harness attaches (the smaller of the two declared values), `d4` against the built-in value, and the DLLs' SHA-256s and file versions with the toolchain's DXC version. `--verify` checks it against the files and the current stage sources.

`run_scene.ps1` reads the record, accepts only `-Scene w3 -Mode run` without `-Inject`, prints the stage plan in the dry run and passes `--sa2-stages <build>\stages`. After the scene loads, the app compiles both stages with the core's DXC (`sa2_stage_compile`), writes each compile record to `stage-compile-vertex.json` and `stage-compile-fragment.json`, attaches the pair before the first frame (`sa2_scene_stages`), and writes one `sa2_stage_report` row per chain frame to `stage-frames.jsonl`. A failed compile ends the run with reason `stage-compile`, its record kept. The finalizer refuses a run whose compile records are missing, failed or not the build's (`stage-compile`), a measured frame without a row (`stage-rows`), or any frame that stopped (`stage-stopped`); an accepted `run.json` carries a `stages` record with the compile passes, the row counts and the largest bound per pixel. The guard holds the stage files and the DXC libraries.

The overdraw allowance. The core stops a frame exactly when its raster bound U exceeds width * height * overdraw (RPI.72; U counts, per triangle, the pixels of its bounding box rounded outward, clipped and widened to 2x2 quads, and a near-plane crossing as the whole target; W3 draws one pass at one sample). Every small triangle costs at least one quad, so U per pixel grows as the target shrinks, and the owner's native resolution is not on record. The gate stages therefore declare 5/4 of the largest U per pixel at 1920x1080, 2560x1440 and 2560x1600, rounded up, and `limits.json` allows exactly that (`max_overdraw`). The tool in `overdraw/` (crate `c9-overdraw`, C9 U5c) computes U headless, with no GPU: every vertex of the 259,800 slots at full detail goes through the core's chain (home point, turn, view) and the gate's vertex stage in c600-render's binary32 CPU reference, then the core's projection and classification, and every triangle through c600-display's `raster::bound`; it covers camera frames 1 to 6284 (two periods of the 0.002 rad step) and, in each frame, the turn angle 0 and 32 more angles from -pi to pi on the 4,605 moving slots.

| Size | Largest U | Frame, turn angle | Per pixel | 5/4, rounded up | `frame_work` at 108 |
| --- | --- | --- | --- | --- | --- |
| 1920x1080 | 177,655,528 | 5803, -3pi/8 | 85.6749 | 108 | 1,508,284,800 |
| 2560x1440 | 271,603,388 | 5800, -3pi/8 | 73.6771 | 93 | 2,553,379,200 |
| 2560x1600 | 320,336,844 | 5803, -3pi/8 | 78.2072 | 98 | 2,818,800,000 |

Both stages declare overdraw 108. Under it no frame of the sweep stops at any of the three sizes; under U5a's draft of 8 every frame does. RM.38's `frame_work` (18,288,000 vertex evaluations times the vertex stage's 9 operations, plus pixels times 108 times the fragment stage's 6) stays far within the shell host's profile `shell-draft/1`, whose every ceiling is 2^40 (1,099,511,627,776). The tool's turn-angle-0 figures equal the WARP device's bound (`sa2_selftest --warp --measure-stages`) at 2560x1600 in all 1,991 frames measured there, and its evaluation of distinct vertices equals the full per-vertex evaluation in frames 1 to 12. The figures are the bound's value, not performance evidence; another target size needs a new sizing, and a smaller one needs a larger allowance.

```powershell
python -B renderer/acceptance/prepare_host.py --route surface --stages renderer/acceptance/stages   # prints work/build/c9-acceptance/<UTC>
# in a worktree without tools/.dxc, add: --dxc <main checkout>\tools\.dxc\v1.9.2609\bin\x64
$staged = 'work/build/c9-acceptance/<UTC>'
python -B renderer/acceptance/prepare_host.py --verify $staged
& $runner -DryRun -Scene w3 -Build $staged -Overlays 'none running' -Declare @('frame_generation=false','upscaling=false') -Unattended
& $runner -Scene w3 -Build $staged -Overlays 'none running' -Declare @('frame_generation=false','upscaling=false') -Unattended
```

Run the last command three times and judge the three runs with the gate as below. To size the overdraw again (headless, no GPU or window), build the tool with `Push-Location renderer/acceptance/overdraw; cargo build --release --offline --locked --target-dir ../../../work/build/c9-overdraw; Pop-Location` (Cargo reads the folder's `.cargo/config.toml`, which points at `rust/vendor`) and run `work/build/c9-overdraw/release/c9-overdraw.exe --root . --out work/build/c9-overdraw/w3.jsonl --first 1 --last 6284`; the sweep took about 70 minutes on 28 threads. It writes a header, one line per frame with U per size, and a summary with the largest U, the overdraw each size needs, `frame_work` and the proposal. Parts swept with `--first` and `--last` join with `c9-overdraw --out <file> --merge <part> --merge <part> ...`. The device cross-check runs on WARP only (no hardware adapter, no window): build the core with `python renderer/core/check_core.py --stages <DXC folder>`, generate the pair with `c600-stage-hlsl` into a folder with `arrays.json`, and run `sa2_selftest.exe --warp --measure-stages <folder> --size 2560x1600 --frames 3142 --traced 400 --out <file>.jsonl` from the build folder; it writes one line per frame with U, attaches the pair with an unlimited allowance, and its untraced frames must equal the tool's turn-angle-0 figures. A route-B surface cannot be checked without a window, so no headless check covers the stage chain on route B's surface; the first route-B stage run is that check.

## Dry run (no Godot, PresentMon or GPU device; creates nothing)

```powershell
foreach ($scene in 'w3','S4','I_a','I_b') {
    & $runner -DryRun -Scene $scene -Build $build -Overlays 'none running' -Declare @('frame_generation=false','upscaling=false') -Unattended
}
```

## Untimed checks (no administrator rights, no PresentMon)

```powershell
foreach ($scene in 'S4','I_a','I_b','w3') { & $runner -Scene $scene -Build $build -Mode geometry }
& $runner -Scene S4 -Build $build -Mode lattice
```

Each writes `geometry-record.json` or `lattice-record.json` with status `pass` in its run directory.

## One cold run (administrator PowerShell)

```powershell
& $runner -Scene S4  -Build $build -Overlays 'none running' -Declare @('frame_generation=false','upscaling=false') -Unattended
& $runner -Scene I_a -Build $build -Overlays 'none running' -Declare @('frame_generation=false','upscaling=false') -Unattended
& $runner -Scene I_b -Build $build -Overlays 'none running' -Declare @('frame_generation=false','upscaling=false') -Unattended
& $runner -Scene w3  -Build $build -Overlays 'none running' -Declare @('frame_generation=false','upscaling=false') -Unattended
```

Leave out `-Unattended` when the owner watches the run. Each invocation is one cold run: it pauses 20 s, checks that the machine is quiet, starts a new Godot process for a 4 s preroll and a 192 s trace (turns of 190 ms), captures it with PresentMon and finalizes it in `work/loop-memory/c9-acceptance/<UTC>/`. It prints fps, p99, peak VRAM and the label or pose check, and writes that run's gate summary `summary.json` (verdict `insufficient-runs` for one run by design). Run each command three times.

## The gate over three runs

```powershell
python -B tools/perf/renderer_gate.py work/loop-memory/c9-acceptance/<UTC 1> work/loop-memory/c9-acceptance/<UTC 2> work/loop-memory/c9-acceptance/<UTC 3> --out work/loop-memory/c9-acceptance/<scene>-summary.json
```

The summary has the evidence format `magic600-renderer-gate-v1`; the verdict is `met` only if all three runs and the pooled frames meet both frame bars with the label and pose checks passing.

## Negative tests (administrator PowerShell)

`-Inject` runs a fault at turn 20 and records the run as fault-injection, never as gate evidence: `corrupt-label`, `swap-same-colour`, `delay-adoption` or `stale-binding` for W3; `corrupt-index`, `swap-same-colour`, `stale-pose`, `delay-adoption` or `stale-binding` for W-J. A caught fault ends with finalizer exit 2 and a failing check. The record is `fault-run.json` with its own format, which the gate neither reads nor accepts, and no gate summary is made; an injected run never takes an operator declaration. If the file guard reports a changed build file after the finalizer wrote its record, the record is renamed `*.rejected`.

```powershell
& $runner -Scene S4 -Build $build -Inject stale-pose
```
