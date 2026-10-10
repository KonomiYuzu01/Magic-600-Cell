# Level 2 result card: Godot (S-A2) and Qt (S-D) framework scenes

The owner ran the attended level 2 steps 4 to 7 of `HARNESS.md` section 11 on 4 October 2026. Claude read the records and wrote this card on 9 October 2026. These are actual Windows/Direct3D 12 measurements on one machine, under the conditions below, for the build identities named here. The public summaries are the files in [results/](results/), written by `tools/perf/renderer_gate.py` (`summarize`): eight from 4 October and two from the re-acceptance of 10 October (section below). The raw PresentMon output, traces and logs stay private.

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
- L2-V-002 is open (Astra escalation `20261009T155501Z-a2cb4438`). The finalizer did not bind the recorded framework files, or the shaders, to the bytes the process loaded, so these identity assignments rest on the preparation records and on the operator's account. The gate figures below remain attended observations with that qualification. The fix, a guard taken before launch, was built on 10 October 2026. Its two-shard Astra review found four blocking findings (L2-A-001 to L2-A-003, L2-V-002-B01); they were fixed the same day, and the fixed build passed its source checks and the synthetic experiments below. The scoped verification round passed on both shards with no finding (`20261010T044131Z-d960c484` and `20261010T044132Z-5d1126c3`, candidate `0be417e`). No gate run has used it yet, and it does not change these 4 October records.

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

Claude ran `guard_experiments.py` (plan `PLAN-L2-V-002.md` section 7, item 1) on the owner's machine against Qt build `20261010T042228Z`, which carries the review fixes. There was no window, GPU work or PresentMon. These are actual Windows file-system and loader results for this machine and this build, not gate evidence. Result: 9 passed, 1 not run, 0 failed. The first run, on build `20261010T024533Z` before the fixes, gave 8 passed, 1 not run, 0 failed, with the same results in the shared rows.

| Experiment | Result |
|---|---|
| `subst` drive alias | refused at the spelling check |
| Symbolic link as the leaf | not run: creating a link needs a privilege this account lacks (Windows error 1314). `check_guard.py` tests the reparse refusal with a junction. |
| `\\localhost\` administrative share | the share was readable; the guard refused the UNC form at the path-form check |
| Hard link to a held file | the link could be created. A write through it failed with a sharing violation. The guarded name could not be removed (error 32); the second link could. The release check passed. A load through the second link has another path, and a replacement at the guarded name has another file ID; the finalizer refuses both (G4 checks 2 and 4). |
| Renaming an ancestor folder of a held deployment copy | 33 files and 20 folders held; renaming the deployment folder and `platforms/` both failed (error 32) |
| Guard timing, 33 files | start to `ready` 247 ms; `release` to exit 9 ms, including the write probe |
| Qt usage path under the guard | with all 33 files of a deployment copy held, `sd_smoke.exe` loaded its imports and exited 1 (`usage`, no window or GPU work). It recorded 11 in-scope modules, all of them guarded. Every file was still held afterwards, and the release check passed. |
| DLL redirection through a `.local` folder | none: with a `.local` copy present, a load by absolute path reported the requested path |
| Qt module observer test program | 10 of 10 cases, 210 child processes; in case 6, 30 of 200 children exited 3 at the seal and the rest recorded the probe. Cases 8 to 10 (L2-V-002-B01): a `\\?\` load entered the union under that spelling, the same load after the seal exited 3, and a load by the directory's 8.3 spelling entered the union once (this path has 8.3 names, so case 10 ran). |
| A guard that never becomes ready (L2-A-001) | a test guard held a synthetic file and either gave no line or gave another line; `Start-Guard`, taken from `run_scene.ps1` with the runner's own `python` resolution, threw in both cases, and afterwards the guard process was gone and the file writable. The runner as committed before the fix failed this check (the guard outlived `Start-Guard`). |

Not run yet: the attended experiments of plan section 7, items 2 and 3 (a replacement helper during a W3 run, and the Qt `module-*` injections), which need a window, the GPU and the owner. The first gate runs under the guard are in the next section.

## Re-acceptance after the anchor change, 10 October 2026

The owner changed the sticker shrink anchor (`docs/wiki/decisions/owner-decisions-2026-10-10-anchor.md`). Both apps load the S-A2 native library, which compiles the S-B probe source, so both got new builds. The owner ran the attended level 2 steps again for each. Conditions were as above.

Builds, both prepared offline from commit `c4b31b0` with `matches_head` true:
- Qt 6.10.3 (S-D): build identity `befc441f2b5715c3f0c60d173700fc2eef22ad3eac18c6f91beac23f58f088c5`.
- Godot 4.7.2 .NET (S-A2): build identity `06d14019c937ed55e90ad3a78e28e9988139a3fdd946e3c9f20495e9ee034106`.

Every step ran under the guard (rule `guard-before-launch`). The guard held 38 files and 20 folders for Qt, and 12 files and 27 folders for Godot, from before launch until the finalizer was done. The L2-V-002 qualification of the 4 October records therefore does not apply to these runs. The guard's stated limits (`HARNESS.md` section 6) still apply.

| Step | Qt (S-D) | Godot (S-A2) |
|---|---|---|
| Validation, W1 to W4 | recorded; W3 and W4 label checks pass, 105 revisions | recorded; W3 and W4 label checks pass, 105 revisions |
| Geometry | pass | pass |
| Short W3 check | label check passes, 157 revisions | label check passes, 157 revisions |
| Deliberate refusals | exactly `size-mismatch`, then exactly `vram-missing` | exactly `size-mismatch`, then exactly `vram-missing` |
| W3, three cold runs | `met` | `met` |

W3 ([Qt summary](results/sd-w3-20261010T172910231Z-summary.json), [Godot summary](results/sa2-w3-20261010T175043422Z-summary.json)):
- Qt: 60.00 fps in each run; p99 17.412, 17.431 and 17.475 ms; pooled p99 17.440 ms, max 18.310 ms. This is the vblank cap of finding 1.
- Godot: 671.28, 671.05 and 669.74 fps; p99 1.895, 1.906 and 1.916 ms; pooled 670.69 fps, p99 1.907 ms.
  - The maximum frame time was 12.328 ms, in run 1; runs 2 and 3 peaked at 3.898 and 3.310 ms.
  - `tearing` was true in all three runs.
- The exact label check passed in every run, with 1,010 revisions each.
- Peak VRAM: Qt 229.6 MB, Godot 217.2 MB.
- Godot's refusals now give exactly the required reasons. This confirms on the GPU the `blind-seconds` fix of finding 2.

These figures hold for these two build identities only.

## Not claimed

- That either candidate is selected. The selection gate needs the day-7 go/no-go ruling (`docs/wiki/decisions/renderer-candidates.md`).
- Any uncapped Qt figure, or any GPU-headroom comparison between Qt and Godot.
- Loaded-build binding for the 4 October identities (L2-V-002). The 10 October identities ran under the guard, within its stated limits.
- That the Qt observer records a load through a subst drive, a junction or symbolic link outside the build directory, a UNC share or a hard link outside it. It does not; `HARNESS.md` section 6 states this limit.
- Results on other hardware, at other resolutions or refresh rates, with frame generation, or over long sessions.
