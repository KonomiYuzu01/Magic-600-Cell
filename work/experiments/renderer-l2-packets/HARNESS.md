# Level 2 harness contract (packets L2-F, L2-G and L2-Q)

Status: binding for L2-F, L2-G and L2-Q (stage day 4, 4 October 2026). Claude owns this file; the packets read it and never change it. It turns plan section 3 (`PLAN.md`) into exact behaviour. "Verify at the pinned version" marks a framework fact (Godot 4.7.2 .NET, Qt 6.10.3) the code relies on. The Codex sandbox has no framework source, so a packet lists each such fact as assumed, names the runtime check of this file that catches it if it is wrong, and keeps the code fail-closed. The integrator verifies the assumed facts against the pinned source before the owner's runs and records the result in `FRAMEWORK-FACTS.md`. Where the pinned version contradicts this file, follow the facts, keep the check and say so.

## 1. Parts

| Part | Packet | Files | Writes |
|---|---|---|---|
| Producer DLL, ABI 2 | L2-N (integrated, read-only here) | `work/experiments/renderer-sa2/native/` | `trace.jsonl`, `native.json`, `geometry.json` |
| Godot app, candidate `sa2` | L2-G | `work/experiments/renderer-sa2/project/`, `prepare_l2.py`, `check_project.py`, `README.md` | `harness.json` |
| Qt app, candidate `sd` | L2-Q | `work/experiments/renderer-sd/`: `app/`, `build.cmd`, `prepare_l2.py`, `check_project.py`, `README.md` | `harness.json` |
| Finalizer | L2-F | `work/experiments/renderer-l2/finalize_run.py` | `run.json`, `short-check.json`, `geometry-record.json`, `validation-record.json` or `refusal.json` |
| Runner | L2-F | `work/experiments/renderer-l2/run_scene.ps1` | run directories, `presentmon.csv` |

- One finalizer and one runner serve both candidates, so both are judged by the same code.
- The app writes only `harness.json`, plus the files the DLL writes for it. It never writes `run.json`.
- Only the finalizer writes records. `tools/perf/renderer_gate.py` stays unchanged and reads `run.json` as it reads S-B's.
- One scene per app process.

## 2. Window, display and presentation (both apps)

- One window on the primary monitor. It is borderless, covers the whole monitor and is topmost. The slot texture fills its client area, with nothing drawn over it: no UI, text, cursor overlay or debug display.
  - Godot: use the window mode that covers the monitor exactly. At 4.7.2 on Windows, `WINDOW_MODE_FULLSCREEN` extends the window 2 pixels past the monitor and clips them with a window region, and `WINDOW_MODE_EXCLUSIVE_FULLSCREEN` uses the exact monitor rectangle (`FRAMEWORK-FACTS.md`, G1). Content scale mode is disabled and the content scale factor is 1.
  - Qt: `showFullScreen()` on the primary screen. Check at the pinned version that the Windows platform plugin adds no border to a Direct3D 12 window.
- The app is per-monitor DPI aware (version 2), as both frameworks are by default. `harness.json` records the awareness of the render thread.
- Five sizes, all in physical pixels:
  - `display`: the current mode of the window's monitor (`MonitorFromWindow`, `GetMonitorInfoW`, `EnumDisplaySettingsW(ENUM_CURRENT_SETTINGS)`), with `refresh_hz`;
  - `window`: the client rectangle (`GetClientRect`);
  - `backbuffer`: the swap chain's size as the framework reports it. Godot: `RenderingDevice.ScreenGetWidth/ScreenGetHeight` for the main window. Qt: `QQuickWindow::swapChain()->currentPixelSize()`. Verify both at the pinned version;
  - `displayed`: the rectangle the framework lays the texture out in, converted to physical pixels;
  - `target`: the slot size the app registered.
- A run is valid only if all five sizes are equal and the scaling is 1:1 (section 7: `size-mismatch`, `scaling`).
- No vsync:
  - Godot: `--disable-vsync`;
  - Qt: swap interval 0 (`QSurfaceFormat::setSwapInterval(0)`, which the D3D12 backend maps to its no-vsync swap chain; verify);
  - the finalizer checks PresentMon's `SyncInterval`.
- Keep the display and the system on during the run: `SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED)`, as S-B (`renderer-sb/probe/src/gpu.cpp`, `DisplayRequest`).
- Foreground:
  - The app calls `SetForegroundWindow` once after its window is shown.
  - It uses no workaround for the foreground lock: no `AttachThreadInput`, no synthetic input, no helper process.
  - The runner starts it from the operator's console, which may give it the foreground.
- Adapter: the framework's device uses the high-performance adapter. Use the framework's own setting where it has one (`FRAMEWORK-FACTS.md`, G6 and Q5). The finalizer refuses a run on any other adapter (section 7, `adapter`).

## 3. App command line

Godot takes these after `--`, as user arguments. Qt takes them as ordinary arguments. A missing required option, an unknown, repeated or malformed option, or an out-of-range value makes the app exit 1 before any window or device work, with `harness.json` reason `usage` when `--l2-out` is usable.

| Option | Values | Default |
|---|---|---|
| `--l2-mode` | `run`, `geometry` | required |
| `--l2-scene` | `w1`, `w2`, `w3`, `w4` | required in `run` mode |
| `--l2-out` | an existing directory that holds none of the app's or the DLL's output files (`harness.json`, its temporary name, `native.json`, `trace.jsonl`, `geometry.json`); other files, such as the framework's log or PresentMon's CSV, may already be there | required |
| `--l2-run-id` | `[A-Za-z0-9][A-Za-z0-9._-]{0,127}` | required |
| `--l2-dll` | absolute path of `sa2_interop.dll` | required |
| `--l2-trace-ms` | 1000 to 3600000 | 192000 |
| `--l2-preroll-ms` | 0 to 60000 | 4000 |
| `--l2-turn-ms` | W3 only: 0 < value <= 10000 | 190 |
| `--l2-inject` | `corrupt-label`, `swap-same-colour`, `delay-adoption`, `stale-binding`; W3 and W4 only | none |
| `--l2-declare` | `key=value`, repeatable. `true` and `false` become booleans (S-B's rule). The key `overlays` is refused. | none |
| `--l2-gpu-validation` | `0`, `1` | `0` |
| `--l2-conditions` | `enforce`, `record` | `enforce` |
| `--l2-no-vram` | flag: `SA2_SCENE_NO_VRAM` | off |
| `--l2-debug-half-target` | flag: register the slots at half the displayed width and height, rounded down | off |

Framework options keep their level 1 names and values. Their defaults are the level 1 configurations that passed:
- Godot: R2 (route `export`, queue `same`, handover `tracked`, barriers `match`, render thread `safe`, warm-up 3 frames).
- Qt: Q5 (device `qt`, route `import-direct`, queue `same`, handover `tracked`, barriers `legacy`, render loop `threaded`).

Level 2 is a new mode of each app, selected by `--l2-mode`; the level 1 smoke mode stays as it is, so `run_smoke.py` and the level 1 checks keep working with the ABI 2 DLL. In level 2 mode, the level 1 options with no meaning there are refused: frame counts, resize, verify, device loss, drain injection.

`--l2-gpu-validation 1` turns on the framework's GPU validation (Godot: also pass `--gpu-validation` to the engine; Qt: `QQuickGraphicsConfiguration::setDebugLayer(true)` before the window is exposed) and attaches with `debug_callback = 1`.

## 4. Call order and exit codes

Follow `work/experiments/renderer-sa2/native/README.md`, "Scene host call order", on the framework's render thread:
1. Probe and attach the framework's device and direct queue (`wait_timeout_ms` 5000).
2. Create or import three slots at the `displayed` size (half size with `--l2-debug-half-target`), with the route's warm-up. Register them, then call `sa2_scene_load` with `trace_ms` = `--l2-trace-ms`, `turn_ms`, `inject` and `flags`.
3. `geometry` mode: call `sa2_scene_geometry_check` into `--l2-out`, then drain, unload and tear down. No trace and no condition sampling.
4. `run` mode, per framework frame `f` from 1:
   - `sa2_signal_godot_free(f - 1)`, `sa2_scene_produce(f % 3, f)`, `sa2_godot_wait_ready(f)`;
   - the framework draws the slot and presents;
   - `sa2_mark_shown(f % 3, f)`.
   Exactly one produce per framework frame and per present. Never produce twice between two presents, and never present a frame without a produce.
5. Preroll for `--l2-preroll-ms`, measured on the QPC clock from the first produce. Then:
   - wait up to 5 s for the window to be in the foreground (enforce mode: exit 3 with reason `foreground` if it is not);
   - record the five sizes;
   - call `sa2_scene_trace_begin`.
6. Keep producing. Call `sa2_scene_trace_end` at the first frame boundary at or after `--l2-trace-ms` from trace begin (no postroll), then `sa2_drain`, then `sa2_scene_write_run` into `--l2-out`:
   - `SA2_E_CHECK_FAILED` means exit 2 (outputs written);
   - any other failure means exit 1.
7. Unload, unregister, release, detach, then write `harness.json` (section 6) and exit.

The trace must contain exactly one DLL record per present. The finalizer checks this against PresentMon (section 7, `trace-steps`).

| Exit | Meaning |
|---|---|
| 0 | done; DLL outputs written |
| 1 | usage, device, DLL, framework or I/O failure; no usable outputs |
| 2 | label check (`run`) or geometry check (`geometry`) failed; outputs written |
| 3 | enforce mode: the window was not in the foreground before trace begin, or a condition sample of section 5 failed. The app stops at that sample: trace end, drain, no `write_run`, teardown. |

On every exit after the options are parsed, `harness.json` is written, with `exit_code` and `reason` (`null` on exit 0). A size change during the trace does not stop the run. The samples record it and the finalizer refuses it. Never rebuild the ring during the trace.

## 5. Condition sampling (run mode, during the trace)

Every 100 ms on the QPC clock, as S-B's `sampleConditions` (`renderer-sb/probe/src/gpu.cpp`):
- visibility: `IsWindowVisible`, `IsIconic`, `DWMWA_CLOAKED`, and the five-point covered test (centre and four points 10% inside the corners, each hit-testing to this window's root);
- foreground: `GetForegroundWindow` equals the window;
- power: `GetSystemPowerStatus` (mains, battery), plus the effective power mode at the first sample and whether it changed (`PowerRegisterForEffectivePowerModeNotifications`, S-B's names);
- the five sizes: count the samples where any size differs from its value at trace begin.

In enforce mode the first sample that is not visible or not in the foreground ends the run with exit 3 (section 4). In record mode the counts are kept and the run goes on.

## 6. `harness.json`

Format `magic600-l2-harness-v1`. UTF-8, written to a temporary name in `--l2-out` and renamed, never over an existing file. Every key below is required. Leave values unknown at failure time `null`, and keep the object shapes. Example (`sa2`, run mode):

```json
{
  "format": "magic600-l2-harness-v1",
  "candidate": "sa2",
  "mode": "run",
  "run_id": "20261005T090000000Z-w3-1",
  "scene": "w3",
  "process_id": 4242,
  "exit_code": 0,
  "reason": null,
  "options": {"trace_ms": 192000, "preroll_ms": 4000, "turn_ms": 190, "inject": null,
              "declared": {"frame_generation": false, "upscaling": false},
              "gpu_validation": false, "conditions": "enforce", "no_vram": false, "debug_half_target": false},
  "configuration": {"framework": "godot", "framework_version": "4.7.2.stable.mono.official",
                    "rendering_driver": "d3d12", "window_mode": "exclusive_fullscreen", "vsync": "disabled",
                    "route": "export", "queue": "same", "handover": "tracked", "barriers": "match",
                    "render_thread": "safe", "warmup_frames": 3},
  "files": {"dll": "C:\\...\\sa2_interop.dll", "godot:exe": "C:\\...\\Godot_v4.7.2-stable_mono_win64.exe",
            "godot:assembly": "C:\\...\\SA2Smoke.dll", "godot:project.godot": "C:\\...\\project.godot",
            "godot:Main.tscn": "C:\\...\\Main.tscn"},
  "dll_identity": {"dll": {"file": "sa2_interop.dll", "sha256": "..."}, "shaders": [{"file": "...", "sha256": "..."}]},
  "dll_status": {"abi_version": 2, "last_status": 0, "last_error": ""},
  "qpc_frequency": 10000000,
  "sizes": {"display": {"width": 2560, "height": 1600}, "window": {"width": 2560, "height": 1600},
            "backbuffer": {"width": 2560, "height": 1600}, "displayed": {"width": 2560, "height": 1600},
            "target": {"width": 2560, "height": 1600}, "samples": 1920, "samples_changed": 0},
  "scaling": {"content_scale_mode": "disabled", "content_scale_factor": 1.0, "texture_stretch": "none"},
  "dpi_awareness": "per-monitor-v2",
  "environment": {"power_source": "mains", "power_mode": "max_performance",
                  "presenting_adapter": "NVIDIA GeForce RTX 4070 Laptop GPU, driver 32.0.16.1692, drives the window's display",
                  "presentation_interval": 0, "declared": {"frame_generation": false, "upscaling": false},
                  "display": {"width": 2560, "height": 1600, "refresh_hz": 240},
                  "backbuffer": {"width": 2560, "height": 1600},
                  "power_samples": {"samples": 1920, "mains": 1920, "battery": 0},
                  "vsync": false, "adapter": "NVIDIA GeForce RTX 4070 Laptop GPU", "driver": "32.0.16.1692",
                  "msaa": 1, "warp": false},
  "window": {"topmost": true, "display_required": true, "foreground_at_trace_start": true, "sample_period_ms": 100,
             "samples": 1920, "samples_not_visible": 0, "samples_covered": 0, "samples_not_foreground": 0,
             "visible_throughout": true, "foreground_throughout": true, "presents": 21000},
  "debug": {"enabled": false, "debug_layer": 0, "counts": null, "messages": null}
}
```

- `configuration` holds every setting that can change what is drawn or how it is presented, as flat strings, numbers or booleans. It never holds run options or paths (section 8). The validation switches are run options (`options.gpu_validation`), so they stay out of it: Godot's `--gpu-validation` and `--gpu-abort`, Qt's debug layer and `QSG_RHI_DEBUG_LAYER`.
  - Godot: the framework version string, the rendering driver, window mode, vsync mode, the render thread, the level 1 options, and the engine arguments. These come from the process command line (`GetCommandLineW`, split with `CommandLineToArgvW`), without the executable, `--path`, `--log-file`, their values, the validation switches and everything from `--` on. `OS.GetCmdlineArgs()` cannot serve: at 4.7.2 it leaves out every argument the engine consumes (`FRAMEWORK-FACTS.md`, G4).
  - Qt: `qVersion()`, the graphics API, the render loop, swap interval, the level 1 options, and every environment variable named `QSG_*` or `QT_*` that affects the scene graph or the RHI, by name and value, except the validation switch.
- `files` maps part names to absolute paths. `dll` is the DLL the app loaded.
  - Godot: also `godot:exe` (the running executable), `godot:assembly` (the project's C# assembly as loaded), and `godot:<name>` for every project file the run reads (`project.godot`, `Main.tscn`, every other resource it loads).
  - Qt: also `qt:exe` and `qt:<basename>` for every module loaded from the executable's directory or its subdirectories (`EnumProcessModules`), except the DLL.
- `dll_identity` is the parsed output of `sa2_identity`.
- `scaling` holds the raw facts:
  - Godot: `content_scale_mode`, `content_scale_factor` and `texture_stretch` (how the display node maps the texture to its rectangle);
  - Qt: `device_pixel_ratio`, `item_width` and `item_height` (the item's size in logical pixels, as numbers), and `texture_stretch`.
- `environment` uses S-B's keys and meanings (`gpu.cpp`, `environment()`), except `tearing`, which the finalizer fills from PresentMon:
  - `power_samples` counts the condition samples of section 5 and the mains and battery samples among them. `power_source` is derived from them as S-B does: `mains` when every sample was on mains, `battery` when every sample was on battery, `changed` when both occurred, and `unknown` otherwise. `power_mode` is `changed` when the effective power mode changed during the trace;
  - `adapter` is the framework's adapter name (Godot: `RenderingServer.GetVideoAdapterName()`; Qt: `QRhi::driverInfo().deviceName`);
  - `driver` is the UMD version from `sa2_device_info.umd_version`, as `a.b.c.d` (S-B's format);
  - `presenting_adapter` is S-B's text. "drives the window's display" holds when `EnumDisplayDevicesW` gives the window monitor's device (`szDevice`) a `DeviceString` equal to `adapter`; otherwise the text says "display driven by another adapter".
- `window.presents` is the number of frames produced in the trace.
- `debug.debug_layer` is `sa2_device_info.debug_layer` from the probe, in every mode: it shows whether the device has the debug layer, whatever the options say.
- `debug.counts` and `debug.messages`, when `--l2-gpu-validation 1`, hold `sa2_debug_counts` (all fields) and `sa2_debug_messages`, read after the drain.

## 7. Finalizer (`finalize_run.py`)

`python -B finalize_run.py <dir> --candidate sa2|sd --mode run|short|geometry|validation --launched-pid <pid> --app-exit <code> [--adapter <name>] [--overlays <text> | --fault-injection]`

- `--adapter` names the adapter the run must use. Its default is the gate's GPU, `NVIDIA GeForce RTX 4070 Laptop GPU`.

- Its mode selects the app mode it expects in `harness.json`: `run`, `short` and `validation` expect `run`, and `geometry` expects `geometry`.
- It reads `harness.json`, the DLL outputs of that mode and `presentmon.csv` (`run` and `short` modes) in `<dir>`:
  - `run`, `short` and `validation`: `native.json` and `trace.jsonl`;
  - `geometry`: `geometry.json`, and neither `native.json` nor `trace.jsonl`.
- It writes exactly one of the outputs below with exclusive create, then exits:
  - 0: record written, all checks pass;
  - 2: record written, with a failed label or geometry check;
  - 5: `refusal.json` written;
  - 1: usage error, nothing written.
- It never overwrites. If any output file already exists, it exits 5 with reason `output-exists` and writes nothing.
- `refusal.json`: format `magic600-l2-refusal-v1`, with `candidate`, `mode`, `run_id` (or `null`), `reasons` (sorted codes) and `details` (one short sentence per code).
- It imports `tools/perf/renderer_gate.py` read-only. Before writing `run.json` it runs `validate_run` on the record (refused: `gate-shape`). It reports `condition_reasons` in the record and never refuses on them; the gate judges conditions.

Checks and refusal codes. The interval `[a, b)` is on the QPC clock of `native.json`, with `T0` = `trace_start_qpc` and `stop` = `trace_stop_qpc`.

| Code | Mode | Refused when |
|---|---|---|
| `harness` | all | `harness.json` is missing or malformed; `candidate` or `process_id` differ from the arguments (`process_id` must equal `--launched-pid`); or `mode` is not the app mode the finalizer's mode expects |
| `app-exit` | all | `--app-exit` differs from `exit_code`, or is neither 0 nor 2 |
| `native` | all | The mode's DLL outputs are missing or malformed. `run`, `short`, `validation`: `scene` differs from `harness.json`; `frames` differs from the number of `trace.jsonl` lines or from `window.presents`, or trace frames are not consecutive from 0; W3 `turn_ms` differs from the option. `geometry`: `geometry.json` lacks S-B's format `magic600-sb-geometry-check-v1`, or `native.json` or `trace.jsonl` exists |
| `identity` | all | a part file is missing or unreadable; a recomputed DLL or shader digest differs from `dll_identity` or, in the modes that read it, from `native.json`'s `identity`; a required part is missing (`sa2`: `godot:exe`, `godot:assembly`, `godot:project.godot`, `godot:Main.tscn`; `sd`: `qt:exe` and at least one `qt:Qt6*` module) |
| `adapter` | all | `environment.adapter` is not exactly the `--adapter` name |
| `size-mismatch` | run, short, validation | the five sizes differ, `samples_changed` > 0, or `native.json` `target` or `viewport` differ from `sizes.target` |
| `scaling` | run, short, validation | `sa2`: content scale mode not disabled, factor not 1, or `texture_stretch` not `none`. `sd`: displayed width or height in physical pixels differs from `item_width` or `item_height` times `device_pixel_ratio` by more than 0.000001 pixel, or `texture_stretch` not `none` |
| `conditions-not-enforced` | run, short | `options.conditions` is not `enforce` |
| `conditions` | run, short | The condition record contradicts itself or the trace: `window.samples` is 0 or fewer than half the trace length in 100 ms steps; `environment.power_samples.samples` differs from `window.samples`, or `mains` plus `battery` exceeds it; `power_source` is not the value section 6 derives from `power_samples`; `samples_not_visible`, `samples_covered` or `samples_not_foreground` is not 0; or `visible_throughout`, `foreground_throughout` or `foreground_at_trace_start` is not true. Enforce mode ends a run at the first failed visibility or foreground sample, so a finished run that records one is inconsistent. A battery sample is no refusal: it makes `power_source` `battery` or `changed`, and the gate judges it |
| `debug-switch` | run | `gpu_validation`, `no_vram` or `debug_half_target` is set, or `debug.debug_layer` is not 0 |
| `vram-missing` | run, short | `vram_peak_mb` is not a positive finite number |
| `presentmon` | run, short | `presentmon.csv` is missing, its last row is incomplete, or it lacks `ProcessID`, `SwapChainAddress`, `QPCTime`, `msBetweenPresents`, `Dropped`, `SyncInterval`, `PresentMode` or `AllowsTearing`; or no row of the launched PID falls in `[T0, stop)` |
| `swap-chain` | run, short | the PID's rows in `[T0, stop)` have more than one `SwapChainAddress` |
| `sync-interval` | run, short | a row of the chain in `[T0, stop)` has `SyncInterval` other than 0 |
| `blind-seconds` | run, short | a second of the checked interval has no row of the chain with `Dropped` 0. Run: `[T0 + 10 s, min(T0 + 190 s, stop))`, as S-B's runner. Short: `[T0 + 1 s, stop - 1 s)` |
| `trace-steps` | run, short | some step between two consecutive presents of the chain (`QPCTime`) within `[T0 + 1 s, stop - 1 s]` does not hold exactly one trace entry |
| `operator` | run | neither `--overlays` nor `--fault-injection` was given |
| `validation` | validation | `options.gpu_validation` is not true, `debug.debug_layer` is not 1, `debug.counts` is missing, or `error`, `corruption` or `mentioning_sa2` is not 0 |
| `gate-shape` | run | `renderer_gate.validate_run` rejects the record |

Outputs:
- `run.json` (`run` mode), format `magic600-renderer-run-v1`, the gate's contract with S-B's fields:
  - `run_id`, `candidate`, `scene`, `qpc_frequency`, `markers`, `frames`, `injection_applied`, `vram_peak_mb`;
  - `turn_ms` (W3) and `label_check` (W3, W4), from `native.json`;
  - `camera`, as S-B writes it for the scene;
  - `injected_fault` when set;
  - `presentmon`: `{"process_id": <launched PID>, "swap_chain": <the chain>}`;
  - `build`: `{"build_identity", "parts": [{"name", "sha256"}] sorted by name, "dll_identity"}`;
  - `environment`: `harness.json`'s, with `tearing` (true only when every row of the chain in `[T0, stop)` has `AllowsTearing` 1) and `declared.overlays`. That is the `--overlays` text, or S-B's `fault-injection run; not gate evidence`;
  - `window`: `harness.json`'s;
  - `l2`: `sizes`, `scaling`, `options`, `configuration`, `native` (`target`, `viewport`, `queue_mode`, `barrier_api`, `vram_samples`), `presentmon` (`rows`, `present_modes`, `sync_intervals`, `allows_tearing`, all in `[T0, stop)`), `expected_adapter`, `condition_reasons`, and `checks` (every code above with `pass`).
- `short-check.json` (`short`): format `magic600-l2-short-check-v1`, with `run_id`, `candidate`, `scene`, `build`, `l2.presentmon`, `presentmon`, `sizes`, `vram_peak_mb`, `window` and `checks`. Debug switches are allowed in this mode, so their effect is refused by its own check: half target gives `size-mismatch` only, and no VRAM gives `vram-missing` only.
- `geometry-record.json` (`geometry`): format `magic600-l2-geometry-record-v1`, with `candidate`, `run_id`, `status` (from `geometry.json`), `build` and the geometry summary. A failed geometry check gives exit 2.
- `validation-record.json` (`validation`): format `magic600-l2-validation-record-v1`, with `candidate`, `run_id`, `scene`, `build`, `debug` and `label_check` (W3, W4). A failed label check gives exit 2.

## 8. Composite build identity

- Parts:
  - `dll:<file>` for the loaded DLL and `shader:<file>` for each shader in `dll_identity`: SHA-256 of the files, recomputed by the finalizer from `files.dll` and its directory;
  - every other `files` entry under its own name;
  - `settings`: SHA-256 of `configuration` as canonical JSON (`json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)` encoded as UTF-8).
- `build_identity` = SHA-256 of the UTF-8 text made of one line `<name>=<sha256>\n` per part, sorted by name.
- Run options never enter the identity: scene, run ID, paths, trace, preroll and turn lengths, injection, declared conditions, condition mode, GPU validation, no-VRAM and half target. So validation, geometry, short and gate runs of one build share one identity.
- Acceptance: a change to only the DLL bytes changes the identity; a change to only run options does not.

## 9. Runner (`run_scene.ps1`)

A port of `work/experiments/renderer-sb/probe/run_scene.ps1` that keeps all its rules:
- administrator check;
- PresentMon 2.6.0 at the pinned path, with `--v1_metrics --qpc_time --process_id <pid> --output_file <dir>\presentmon.csv --session_name <name>`;
- the ownership mutex, `Assert-NoSession`, and helpers bounded to 30 s;
- the stop through `--terminate_existing_session` after a 2 s pause, and the complete-last-row check;
- guarded cleanup;
- the operator question after each run (S-B's text, adapted to the window);
- 20 s between runs;
- `renderer_gate.py` over the series' run directories at the end.

Changes:
- Parameters:
  - `-Candidate sa2|sd`;
  - `-Build <dir>` (a prepared build, see below; the runner never builds);
  - `-Scene`, `-Runs`, `-Inject`, `-Declare`, `-Overlays`, `-TraceMs`, `-PrerollMs`, `-TurnMs`;
  - `-Short`: 30 s trace, no operator question, no gate summary;
  - `-Geometry`: one geometry run, no PresentMon, no administrator rights;
  - `-Validation`: W1 to W4, 20 s each, `--l2-gpu-validation 1 --l2-conditions record`, no PresentMon, no administrator rights;
  - `-NoVram` and `-DebugHalfTarget`: only with `-Short`;
  - `-Adapter <name>`: passed to the finalizer as `--adapter`; without it the finalizer's default applies.
- Launch:
  - Read `<Build>\launch.json`, format `magic600-l2-launch-v1`, written by the candidate's prepare step. It holds `candidate`, `executable`, `dll` (the absolute path for `--l2-dll`), `working_directory`, `arguments`, `run_arguments` (each `{out}` is replaced by the run directory; Godot: `--log-file {out}\godot.log`), `validation_arguments`, `separator` (`["--"]` for Godot, `[]` for Qt), `environment` and `validation_environment`. The command line is `arguments`, `run_arguments`, `validation_arguments` (validation runs only), `separator`, then the `--l2-` options.
  - `environment` and `validation_environment` map variable names to values; `null` removes the variable from the app's environment.
  - Start `executable` itself: the process that presents, never a console wrapper. That PID goes to PresentMon and to the finalizer.
- Sessions:
  - session and mutex name `magic600-<candidate>-capture`;
  - `Assert-NoSession` refuses while `PresentMon`, `magic600-sb-capture`, `magic600-sa2-capture` or `magic600-sd-capture` runs.
- Before each run, refuse while any of these runs: another runner, a `codex_review.py --kind implement` process, a build (`cl`, `link`, `ninja`, `cmake`, `msbuild`, `dotnet`, `VBCSCompiler`), or Godot, Blender or FFmpeg other than this run's app. Name the process.
- Run directories: `work/loop-memory/perf/renderer/<candidate>/<stamp>-<scene>-<i>`. The runner creates each one and checks that it is empty before it starts the app; the framework's log and PresentMon's CSV then land beside the app's outputs. The run ID is the directory name.
- After the app exits and the capture is stopped, call the finalizer with the app's PID and exit code. Pass `--overlays` (S-B's composed text) only after the operator typed `yes`, or `--fault-injection` with `-Inject`.
  - Finalizer exit 5: stop the series and print the reasons.
  - Exit 2 without `-Inject`: stop the series.
  - App exit 3: stop the series; this is no gate evidence.

## 10. Sandbox acceptance

Nothing here starts Godot, .NET, Qt, PresentMon or a GPU process. Fixture folders never come from `mkdtemp` or `TemporaryDirectory`, whose access list the Codex sandbox refuses, and are removed on every exit. L2-F builds its fixtures under `work/experiments/renderer-l2/check-<pid>/`; L2-G and L2-Q keep their level 1 checks' folder, made with a plain `mkdir` under the system temp directory.
- L2-F, `python work/experiments/renderer-l2/check_l2.py`:
  - A valid synthetic W3 run passes. That covers `harness.json`, the DLL outputs, a 60 fps `presentmon.csv` on the trace clock and part files. Three such runs of one build give the verdict `met` in `renderer_gate.summarize_private`.
  - For each refusal code, a fixture with exactly that defect gives exit 5 and exactly that reason.
  - Each finalizer mode accepts a valid fixture written with the app mode it expects (`run` for `run`, `short` and `validation`; `geometry` for `geometry`). A geometry fixture holds only `harness.json` and `geometry.json` from the run, and its identity is checked against `dll_identity` and the part files.
  - Changing only the condition samples of a valid run is caught: a battery sample with `power_source` `mains` gives `conditions`; the derived `battery` or `changed` passes the finalizer, and `renderer_gate.condition_reasons` reports `conditions-not-met`.
  - Short mode: half target gives exactly `size-mismatch`, and no VRAM gives exactly `vram-missing`.
  - A failed label check gives exit 2 with `label_check.status` `fail`.
  - The identity acceptance of section 8 holds.
  - `output-exists` holds.
  - Geometry and validation records pass and refuse.
  - `run_scene.ps1` is checked statically: it parses (PowerShell language parser, when `powershell.exe` is available), and it contains the pinned PresentMon path, the session names, the mutex, the operator question, the 20 s pause and the finalizer and gate calls.
- L2-G, `python work/experiments/renderer-sa2/check_project.py`, and L2-Q, `python work/experiments/renderer-sd/check_project.py`:
  - Keep the level 1 checks.
  - Pin ABI 2: `SA2_ABI_VERSION` 2; struct sizes 48 (`sa2_device_info`), 28 (`sa2_config`), 128 (`sa2_debug_counts`) and 32 (`sa2_scene_config`); all 27 exports bound by exact name and signature.
  - Check statically that the app implements sections 2 to 6:
    - every option of section 3 with its range, and an `--l2-out` check that refuses only the app's and the DLL's output files, so a directory that already holds a log file and `presentmon.csv` is accepted;
    - every key of section 6, and `power_source` derived from the samples as section 6 says;
    - the window mode, vsync and scaling settings;
    - the call order;
    - the exit codes;
    - the prepare step's `launch.json`.
  - Pure helpers (option parsing, sizes, `harness.json` writing) get unit tests where the level 1 check already compiles or runs code of that language without the framework.

## 11. Owner-machine order (per candidate)

1. Prepare the build (`prepare_l2.py` for each candidate): it writes `launch.json`.
2. `-Validation`: four validation records pass.
3. `-Geometry`: the geometry record passes.
4. `-Short -Scene w3`: the short check passes. This is the plan's short capture: PresentMon selects the process and its chain, and there is one trace entry per present interval.
5. `-Short -Scene w3 -DebugHalfTarget`: refused, `size-mismatch` only. `-Short -Scene w3 -NoVram`: refused, `vram-missing` only.
6. `-Scene w1`, `w2` and `w4` runs (preliminary unless owner-attended).
7. Three owner-attended cold W3 runs, judged by `renderer_gate.py`.

Steps 4 to 7 need an administrator PowerShell and an idle machine on mains power (`PLAN.md` section 6).
