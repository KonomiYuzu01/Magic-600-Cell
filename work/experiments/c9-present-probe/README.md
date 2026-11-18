# C9 present probe (route B)

Probe evidence for a PHS.52 design proposal only; never product or gate evidence. Experiment code, not product code.

Question: can a core layer that owns its own D3D12 device, a composition swap chain and a DirectComposition target
beneath a transparent Godot 4.7.2 window confirm each presented frame (RM.26: `IDXGISwapChain::GetFrameStatistics`
reports its present count), and does what the screen shows agree with those confirmations, through resizes and a
removal of the core's own device?

## Layout

| Path | What it is |
| --- | --- |
| `native/c9pp.h` | The C ABI (version 1); every struct and rule is documented there |
| `native/src/c9pp.cpp` | `c9pp.dll`: device, swap chain, DComp target, frames, statistics, screen sampler, recovery |
| `native/src/code.{h,cpp}` | The frame code (8 x 8 cells: frame id, epoch, CRC-16/CCITT-FALSE, sync), shared by all three users |
| `native/src/control.cpp` | `c9pp_control.exe`: a Godot-free baseline window, and the `--cpu` check |
| `native/CMakeLists.txt`, `native/build.cmd` | MSVC build (C++20, `/MT`, `/W4 /permissive- /utf-8 /EHsc`; d3d12, dxgi, dcomp, user32, gdi32; no shader compiler) |
| `project/` | Godot 4.7.2 .NET project (net10.0, D3D12, transparent window, vsync on) |
| `run_probe.py` | Quiet-machine gate, build, identity, both harnesses with timeouts, analysis |
| `analyze.py` | Writes `summary.json` and `summary.md` for a run; `--selftest` checks it on synthetic logs |

## How the layers stack

- Godot's window is transparent (`window/size/transparent`, `per_pixel_transparency/allowed`, transparent viewport
  background). Its scene draws an opaque grey frame around the core rectangle and an opaque orange UI panel inside it.
- `c9pp_attach` creates its own device through `ID3D12DeviceFactory` with
  `DISALLOW_STORING_NEW_DEVICE_AS_SINGLETON` (falls back to `D3D12CreateDevice`, which may return Godot's singleton),
  checks by COM identity that it differs from Godot's device, creates a composition swap chain (BGRA, 3 buffers,
  FLIP_DISCARD by default, waitable, maximum latency 2), and shows it through `CreateTargetForHwnd(hwnd, FALSE)`.
- Every core frame clears the base colour and draws two copies of the frame code (top-left, bottom-right) with
  `ClearRenderTargetView` rectangles, presents, and records `GetLastPresentCount`. `c9pp_poll` records one
  `GetFrameStatistics` row. `c9pp_sample_screen` BitBlts the core rectangle from the screen (per-monitor-v2 thread
  context, two independent client-to-physical mappings) and decodes the code.
- `c9pp_remove_device` calls `ID3D12Device5::RemoveDevice` on the core's own device only, and refuses unless the device
  is proven independent of the host's. `c9pp_recover` rebuilds the device and swap chain (epoch + 1) and keeps or
  recreates the DComp target.

The control harness uses its own topmost=TRUE premultiplied swap chain as the stand-in for Godot's UI layer.

## Timeline (both harnesses; seconds x `--scale`)

warmup 2, steady 10, hold 3 (one frame every 0.3 s, so samples can catch a confirmed frame on screen), burst 5
(sync interval 0, three frames per host frame), resize to 1024 x 640 and back (1 s to settle, 2 s of frames each),
loss (0.5 s, removal, one frame attempt, one sample, recovery), recovered 5. About 31 s per harness at scale 1.

## Build and checks (no GPU)

From the repository root:

```
cmd.exe /d /c call work\experiments\c9-present-probe\native\build.cmd      (build dir: work\build\c9-present-probe)
work\build\c9-present-probe\c9pp_control.exe --cpu
python work/experiments/c9-present-probe/run_probe.py --build-only          (native build plus offline dotnet build)
python work/experiments/c9-present-probe/analyze.py --selftest
python work/experiments/c9-present-probe/run_probe.py --check-only          (gate JSON; exit 0 quiet, 5 busy)
```

`--cpu` checks argument validation, NULL and invalid input to every ABI call (no device is created), the CRC check
value, and the frame-code round trip (offsets, scales 1.25, 1.5 and 2, colour shift, flipped data and sync cells,
covered corner, base only, mismatched corners, out of bounds, half-cell misregistration never decoding a wrong code).
The dotnet build restores only the four Godot packages from the installation's `GodotSharp/Tools/nupkgs` into a
private package folder under the build directory; there is no network source. `M600_C9PP_NMAKE=1` forces the NMake
generator.

## GPU run (owner's machine)

```
python work/experiments/c9-present-probe/run_probe.py --topmost --wait-quiet 1800
```

The gate requires: none of the listed build or capture processes (another Godot, cargo, rustc, cl, link, ninja,
cmake, MSBuild, Blender, FFmpeg, PresentMon, OBS), no busy Codex process (a ChatGPT-app helper counts only while it
uses CPU), system CPU at most 15 % over 3 s, no keyboard or mouse input for 300 s (`--min-idle`), mains power, and an
unlocked input desktop. Exit 5 when it is not quiet. `--topmost` keeps both windows above others; without it a run
started from a background process may be covered. Results: `work/loop-memory/c9-present-probe/<UTC>/` (private;
never publish), with `identity.json` (git HEAD, source digest, binary digests, Godot version), `gate.json`,
`runner.json`, `control/` and `godot/` (each `core.jsonl`, `meta.json`, `result.json`, console logs) and
`summary.json`/`summary.md`. The runner stops after the first harness that exits 3 or times out.

Exit codes: 0 completed, 1 build failure, 2 harness error, 3 suspected TDR (the host device reports removal) or
timeout, 5 not quiet.

## Reading the summary

- Confirmed exact: a statistics row of the same swap chain segment reported the frame's own present count.
  Superseded: only a later count. Unconfirmed: neither. Rows whose count exceeds their own `GetLastPresentCount`
  are never used.
- Contradiction: a sample taken 50 ms or more after an exact confirmation shows an older frame, an unknown frame, a
  frame presented after the sample, or no code while the host UI is visible. Any contradiction is a lead against the
  route; a covered window (UI colour not seen) is excluded.
- Seen never confirmed: on screen but never confirmed (a false negative of RM.26).

## Open points

- Whether `GetFrameStatistics` works on composition swap chains at all, and how its PresentCount numbers relate
  to `GetLastPresentCount` (possibly `DXGI_ERROR_FRAME_STATISTICS_DISJOINT` or `DXGI_ERROR_INVALID_CALL`
  throughout); the DComp device's own frame statistics are logged alongside.
- Whether the device factory exists on the installed runtime (`factory_hr`); without it the core shares Godot's
  device and removal is refused.
- Whether Godot's transparent D3D12 window uses a topmost=TRUE DComp target, leaving the FALSE target free, and
  whether that target shows beneath Godot's content.
- DPI: Godot is system DPI aware; the sampler records both mappings and a consistency flag.
- Colour management or HDR can shift colours; the decoder tolerates moderate shifts and reports ambiguous cells.
- The BitBlt is not synchronized with composition; a sample can see a frame before its statistics row (recorded
  as seen before confirmation).
- Keeping the DComp target across recovery versus recreating it (`--recreate-target` in the control harness).
