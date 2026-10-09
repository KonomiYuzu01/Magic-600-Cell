# Level 2 result card: Godot (S-A2) and Qt (S-D) framework scenes

The owner ran the attended level 2 steps 4 to 7 of `HARNESS.md` section 11 on 4 October 2026. Claude read the records and wrote this card on 9 October 2026. These are actual Windows/Direct3D 12 measurements on one machine, under the conditions below, for the build identities named here. The public summaries are the eight files in [results/](results/), written by `tools/perf/renderer_gate.py` (`summarize`). The raw PresentMon output, traces and logs stay private.

## Conditions (every run)

- RTX 4070 Laptop GPU, NVIDIA driver 32.0.16.1692, driving the window's display; mains power; Windows power mode `max_performance`.
- 2560 x 1600 at 60 Hz, exclusive full screen, swap interval 0, no MSAA, no WARP.
- Declared: no frame generation, no upscaling, no overlays.
- The owner watched every run throughout and confirmed it afterwards.
- Gate method: every frame of the declared swap chain in [trace start + 10 s, + 190 s); at least 30 fps and p99 at most 33.3 ms, per run and pooled, over three valid runs of one build.

## Builds

- Qt 6.10.3 (S-D): build identity `64f08369f2098418fa28d82abcf89a00fd6b08da6d1d2772e7428837e6dc4fc8`, prepared from commit `a52dd1c`.
- Godot 4.7.2 .NET (S-A2): build identity `709a24ebdf753535bda56e4c8f5744820ca85f444f2497cc9cf71be75627b93f`, prepared from commit `2e1868e`.
- Each identity covers the native DLL, its shaders, the framework files the app recorded, and the settings. Each build was prepared offline at a committed head with `matches_head` true, and the owner reports no rebuild during the runs.
- L2-V-002 is open (Astra escalation `20261009T155501Z-a2cb4438`). The finalizer did not bind the recorded framework files, or the shaders, to the bytes the process loaded, so these identity assignments rest on the preparation records and on the operator's account. The gate figures below remain attended observations with that qualification. The fix (a guard taken before launch) is planned, not built.

## Results

| Scene | Qt (S-D) pooled | Godot (S-A2) pooled | Verdict |
|---|---|---|---|
| W3, gate scene, three cold runs | 60.00 fps, p99 17.380 ms, max 18.005 ms | 670.71 fps, p99 1.887 ms, max 4.029 ms | `met` for both |
| W1, three runs | 60.00 fps, p99 17.338 ms | 692.15 fps, p99 1.855 ms | `attribution` (not a gate scene) |
| W2, three runs | 60.00 fps, p99 17.333 ms | 671.89 fps, p99 1.906 ms | `attribution` |
| W4, three runs | 60.00 fps, p99 17.398 ms | 692.95 fps, p99 1.859 ms | `attribution` |

- W3 per run:
  - Qt: p99 17.405, 17.374 and 17.368 ms.
  - Godot: 671.8, 667.8 and 672.6 fps, with p99 1.876, 1.890 and 1.895 ms.
- Peak VRAM: Qt 229.6 MB, Godot 217.2 MB. The budget is about 7 GB.
- Step 4, the 30 s short W3 check, passed for both apps.
- Step 5, the deliberate refusals:
  - Qt: exactly the required reasons.
  - Godot: also `blind-seconds` (see below).

## Findings from these runs

1. **Qt's 60 fps is a frame-rate cap, not GPU headroom.**
   - Qt 6.10.3 delivers each `requestUpdate()` only after the display's vertical blank, whatever the swap interval (`FRAMEWORK-FACTS.md` Q10, source evidence). The Qt app asks for every frame that way.
   - The Qt figures therefore show this cap: about 60 fps with p99 about 17.4 ms in every scene. They do not show how fast Qt could render.
   - An uncapped measurement needs `QT_D3D_NO_VBLANK_THREAD=1` and `QT_QPA_UPDATE_IDLE_TIME=0`, which give a new build identity. It has not been run.
   - The Qt and Godot frame rates are therefore not a like-for-like comparison.
2. **Godot's extra `blind-seconds` in step 5 was a finalizer defect.**
   - The short check's interval is [T0 + 1 s, stop - 1 s), about 28.0005 s long. The finalizer rounded it up to 29 seconds.
   - The last "second" was then a 0.5 ms sliver, shorter than one Godot frame (about 1.5 ms). Qt's runs passed only because a present happened to fall in the sliver.
   - The finalizer now counts whole seconds only (review `20261009T155227Z-a94ea84f`, pass).
   - Input-only copies of the two Godot step 5 records, finalized again, give exactly `size-mismatch` and `vram-missing`. The original refusal records are unchanged.
3. **`tearing: false` in five Godot runs comes from one dropped present each.**
   - Affected runs: W1 runs 1 and 2, W3 run 1, W4 runs 1 and 3.
   - In each, one present 6.8 to 6.9 s after trace start was dropped. That is in the warm-up, before the gate interval. It carried `DXGI_PRESENT_ALLOW_TEARING` in `PresentFlags`.
   - PresentMon reports `AllowsTearing` 0 for a present it drops before any flip or blit event, whatever its flags (`FRAMEWORK-FACTS.md` P1). Every displayed present in these runs allowed tearing.
   - `tearing` is metadata; the gate verdict does not use it. The finalizer's rule now uses displayed presents only (under review). The published summaries keep the values recorded on 4 October.

## Not claimed

- That either candidate is selected. The selection gate needs the day-7 go/no-go ruling (`docs/wiki/decisions/renderer-candidates.md`).
- Any uncapped Qt figure, or any GPU-headroom comparison between Qt and Godot.
- Loaded-build binding for these identities (L2-V-002).
- Results on other hardware, at other resolutions or refresh rates, with frame generation, or over long sessions.
