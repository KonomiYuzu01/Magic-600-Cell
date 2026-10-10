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
- L2-V-002 is open (Astra escalation `20261009T155501Z-a2cb4438`). The finalizer did not bind the recorded framework files, or the shaders, to the bytes the process loaded, so these identity assignments rest on the preparation records and on the operator's account. The gate figures below remain attended observations with that qualification. The fix, a guard taken before launch, was built on 10 October 2026. It has passed its source checks and the synthetic experiments below, and its Astra review is pending. No gate run has used it yet, and it does not change these 4 October records.

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

## L2-V-002 binding: synthetic experiments, 10 October 2026

Claude ran `guard_experiments.py` (plan `PLAN-L2-V-002.md` section 7, item 1) on the owner's machine against Qt build `20261010T024533Z`. There was no window, GPU work or PresentMon. These are actual Windows file-system and loader results for this machine and this build, not gate evidence. Result: 8 passed, 1 not run, 0 failed.

| Experiment | Result |
|---|---|
| `subst` drive alias | refused at the spelling check |
| Symbolic link as the leaf | not run: creating a link needs a privilege this account lacks (Windows error 1314). `check_guard.py` tests the reparse refusal with a junction. |
| `\\localhost\` administrative share | the share was readable; the guard refused the UNC form at the path-form check |
| Hard link to a held file | the link could be created. A write through it failed with a sharing violation. The guarded name could not be removed (error 32); the second link could. The release check passed. A load through the second link has another path, and a replacement at the guarded name has another file ID; the finalizer refuses both (G4 checks 2 and 4). |
| Renaming an ancestor folder of a held deployment copy | 33 files and 20 folders held; renaming the deployment folder and `platforms/` both failed (error 32) |
| Guard timing, 33 files | start to `ready` 222 ms; `release` to exit 7 ms, including the write probe |
| Qt usage path under the guard | with all 33 files of a deployment copy held, `sd_smoke.exe` loaded its imports and exited 1 (`usage`, no window or GPU work). It recorded 11 in-scope modules, all of them guarded. Every file was still held afterwards, and the release check passed. |
| DLL redirection through a `.local` folder | none: with a `.local` copy present, a load by absolute path reported the requested path |
| Qt module observer test program | 7 of 7 cases, 207 child processes; in case 6, 27 of 200 children exited 3 at the seal and the rest recorded the probe |

Not run yet:
- the attended experiments of plan section 7, items 2 and 3 (a replacement helper during a W3 run, and the Qt `module-*` injections), which need a window, the GPU and the owner;
- any gate run under the guard.

## Not claimed

- That either candidate is selected. The selection gate needs the day-7 go/no-go ruling (`docs/wiki/decisions/renderer-candidates.md`).
- Any uncapped Qt figure, or any GPU-headroom comparison between Qt and Godot.
- Loaded-build binding for these identities (L2-V-002).
- Results on other hardware, at other resolutions or refresh rates, with frame generation, or over long sessions.
