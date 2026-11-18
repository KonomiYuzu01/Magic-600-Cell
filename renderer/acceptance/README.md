# C9 acceptance harness

The run harness for the renderer freeze's acceptance runs on the product core `renderer/core/` (ABI 3 with W-J's pose path): three cold runs each of W-J's S4, I_a and I_b, three cold W3 runs, and W-J's untimed geometry and lattice checks. The bars are unchanged: every run and the pooled frames at least 30 fps on average and a 99th-percentile frame time of at most 33.3 ms over [trace start + 10 s, + 190 s), peak VRAM at most about 7 GB (the gate flags peaks above 7168 MB), and a passing label check (W3) or label and pose check (W-J).

The host is S-A2 as tested: the official Godot 4.7.2 .NET editor binary running `project/` (net8.0) with the core's `sa2_interop.dll`. Where every file comes from: [PROVENANCE.md](PROVENANCE.md). The harness writes nothing under `work/experiments/`.

| File | Role |
| --- | --- |
| `project/` | the S-A2 Godot project with a W-J mode: `--l2-scene wj --l2-menu <S4, I_a, I_b> --l2-data <dir>` and the `lattice` check mode |
| `prepare_host.py` | builds the core and the project offline under `work/build/c9-acceptance/<UTC>/` and runs the core's CPU self-test; `--verify` checks a build; never starts Godot or a GPU device |
| `prepare_data.py` | runs W-J's `prepare_wj.py` in place with the engine Python (CPU only); `--check` verifies an export against the fixtures |
| `run_scene.ps1` | one cold run, or one geometry or lattice check, per invocation; `-DryRun` validates and prints the planned commands |
| `file_guard.py` | holds every loaded build file from before the launch until the finalizer has accepted the run |
| `finalize_run.py` | checks the app's, the DLL's and PresentMon's outputs and writes `run.json`, a check record or `refusal.json` |

## What a run needs

- **The owner away, or watching.** A run with `-Unattended` records that nobody watched it; without it, the runner asks after each run whether the whole run was watched with nothing covering the window.
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

`-Inject` runs a fault at turn 20 and records the run as fault-injection, never as gate evidence: `corrupt-label`, `swap-same-colour`, `delay-adoption` or `stale-binding` for W3; `corrupt-index`, `swap-same-colour`, `stale-pose`, `delay-adoption` or `stale-binding` for W-J. A caught fault ends with finalizer exit 2 and a failing check.

```powershell
& $runner -Scene S4 -Build $build -Inject stale-pose
```
