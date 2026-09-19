# Development performance observations

> **Historical 0.3 measurements.** These results are not a benchmark of the 0.4 package. See the [0.4 release guide](RELEASE_0_4.md); performance optimization is planned for the next iteration.

These measurements describe the final interaction and rendering implementation included in 0.3, not the frozen 0.2.4 download or its historical validation. They precede the subsequent version-banner and packaging updates. They establish substantial improvements and identify targets that remain unmet. They are short Windows samples, not a sustained performance certification of the packaged release.

The measured 13-file native source set has combined SHA-256 `6188e585d4bf77e7b2bcd55f9e81e026df6e65309e709a2349eeba245bcbdcd0`, in the build script's source order. Its later native change centralizes the displayed version as 0.3; the rendering and interaction implementation is unchanged. The source inventory and portable package manifest record the release source hashes. Diagnostic copies, instrumentation, screenshots, session databases, and raw reports remain private.

## Environment and method

- Actual Direct3D device: **Intel HD Graphics 620**, adapter 0, driver **31.0.101.2140**, reported by the device's creation parameters and the Managed DirectX adapter information.
- Original MPUlt renderer in the x86 .NET Framework native host; authoritative Python engine in a separate owned process.
- Normal measured viewport/backbuffer: **931 × 604**. The separate full-resolution run verified a real **1920 × 1080** Direct3D backbuffer. That child window extended beyond the available 1280 × 720 automation desktop; it was physically clipped. This establishes the render-target workload, not a fully visible 1080p desktop session.
- Native input tests use actual owned HWND messages and the production message filter. The drag test uses a scoped test-thread keyboard state. These are not measurements of physical mouse hardware or display scanout.
- Turn timing starts at the final committing MouseUp and ends after the committed native state is synchronized and one correct full frame is rendered/presented. Automatic rendering is paused for each turn sample to avoid counting an extra timer frame. Ordinary scheduler delay before the next timer tick is excluded. Complete 259,800-slot labels, native colors, and styles are checked outside the timed interval.
- Camera dragging uses the active production render timer. The test requests 60 moves at 16 ms intervals; Windows determines the actual cadence. Auxiliary timings include GDI painting and GdiFlush, with the main DirectX timer paused.
- p95 uses the nearest-rank observation. With only 3, 5, 6, or 10 samples, it is the maximum. This is useful for finding regressions, but cannot establish a stable long-run tail.

## Instant native turns

Each mode has one excluded warmup, followed by six active-orbit turns or three all-piece turns. The baseline and development runs have identical before/after full labelled-state hashes for all eleven transitions, including warmups. Current samples expand to two primitives per active-mode native turn and three per all-piece turn. A native click is therefore not assumed to equal one primitive.

| Visible detail | Baseline mean / p95 | Development mean / p95 |
| --- | --- | --- |
| Active orbit, 2,400 stickers | 478.03 / 879.94 ms | **193.27 / 230.78 ms** |
| All 259,800 stickers | 1,669.53 / 1,734.74 ms | **777.92 / 839.77 ms** |

The corresponding mean stages are:

| Stage | Active baseline → development | All-piece baseline → development |
| --- | --- | --- |
| Final native MouseUp handler | 18.61 → 11.29 ms | 768.00 → 33.93 ms |
| Durable commit and native UI synchronization | 453.37 → 175.73 ms | 257.21 → 139.35 ms |
| First correct full Render/Present | 6.02 → 6.22 ms | 644.29 → 604.62 ms |

The **100 ms p95 target is not achieved**. In all-piece mode, a single complete frame already exceeds that budget. The measured reduction is primarily removal of redundant pre-commit work and use of atomic native updates; it should not be represented as a comparable multiplier in full-detail GPU throughput.

An earlier candidate observation was faster (138.44 ms active mean; 663.45 ms all-piece mean), but preceded the final explorer and preview changes and varied with system scheduling. The table deliberately uses the later complete run rather than selecting the best observation.

The native-turn endpoint is synchronous, so its remaining latency is not caused by a completed-job polling delay. A separate instrumented diagnostic recorded zero polls. In two incomplete diagnostic samples, server work was 122–164 ms, native JSON parsing 8–10 ms, snapshot materialization 0.5–1.1 ms, and UI refresh about 5.5 ms. Control disable/enable and worker scheduling also varied substantially. That diagnostic stopped on a foreground-activation guard and is not acceptance evidence; its timings must not be added to or substituted for the table above.

The production protocol returns committed state and its native update together. Unchanged v2 updates contain no packed sticker arrays; sparse changes are validated against their exact base revision. Full snapshots remain the recovery path. Bounded immutable bridge caches avoid repeated mapping and palette reconstruction. Mechanical state, complete collateral, and journal durability remain authoritative.

## Rotation and full-detail restoration

At the actual 1920 × 1080 backbuffer, two warmups followed by five changed-camera full-detail frames produced:

| Observation | Result |
| --- | --- |
| All 259,800 stickers, mean frame | **622.25 ms**, approximately **1.61 render-only FPS** |
| Full-detail minimum / maximum frame | 570.21 / 690.72 ms |
| Adaptive production drag | 60 submitted frames in 1.931 s, approximately **31.07 FPS** |
| Adaptive draw count | 1,500 stickers during motion |
| MouseUp full-detail restoration | **600.58 ms**; all 259,800 stickers restored |

The complete label hash and camera/input cleanup checks passed. Full detail was present when release handling returned and remained present after the following idle interval. The short drag sample meets the numerical 30 FPS target in that interval; it is not a sustained-rate or display-scanout claim. Full-detail restoration still causes a noticeable pause.

At 931 × 604, the same production-timer smoke observed about 30.07 FPS in active-orbit mode and 30.89 FPS while sampling the all-piece mode. Render-only frame cost, submitted-frame cadence, and release latency are different quantities.

**Recommended minimum for continuous Full Detail Rotation:** hardware that sustains all 259,800 stickers at **30 FPS at 1920 × 1080**. The GPU class satisfying that target remains **unverified pending comparison hardware**. Intel HD 620 does not meet it in these measurements. A GPU model cannot be inferred reliably by multiplying its measured rate: CPU projection, the DirectX wrapper, driver, resolution, and visible geometry also affect the result.

## Structure and auxiliary interactions

Reusing four neighbor buttons, four vertex buttons, and unchanged layer options removed repeated HWND creation and autosize work. A fixed isolated comparison reduced Colors navigation mean from 119.19 to 20.21 ms; its scope excludes painting. The complete native application subsequently measured ten separate actions per operation, including synchronous paint:

| Structure operation | Mean | p95 | 50 ms gate |
| --- | --- | --- | --- |
| Select another color | 39.69 ms | 45.33 ms | Passed |
| Move keyboard graph focus | 4.48 ms | 6.18 ms | Passed |
| Activate a graph color with Enter | 38.80 ms | 50.04 ms | **Missed** |

All three operations requested **zero DirectX frames** and preserved the complete puzzle hash. The combined performance run failed its unchanged 50 ms activation gate; the small overshoot is retained rather than rounded into a pass.

A follow-up isolated candidate deferred hidden Layers-page presentation. Its fixed comparison worsened Colors mean from 23.39 to 26.82 ms and p95 from 29.38 to 49.84 ms. It was discarded. An earlier broad layout-suspension candidate also increased layout work and was discarded. Neither approach is present in production.

With both real auxiliary controls sharing the host's immutable geometry and committed status, five warmups and thirty paired actions at each size produced:

| Each view size | Paired camera/projection/paint p95 | Paired hover/hit/forced-paint p95 |
| --- | --- | --- |
| 330 × 420 | 49.79 ms | 48.79 ms |
| 480 × 480 | 46.04 ms | 45.56 ms |

These paired p95 gates passed. Individual maximum hover samples reached about 100 ms, so the tail is not uniformly below 50 ms. The test intentionally forces both paints even when a hover hit is unchanged. Normal unchanged hover can avoid that work.

Both views added **zero main DirectX frames**. Separate one-second observations with the pair open but idle, minimized, and closed each recorded **zero auxiliary paint events**. Opening a warmed additional pair used about 0.67 MiB more managed memory in the sampled process. Closing retains camera/control state; disposal releases owned resources. Process-wide GC/JIT/GDI caches prevented a reliable retained-memory or leak estimate from these short samples. These results are not sustained CPU or memory-soak measurements.

## Remaining performance work

Keep changes isolated until they demonstrate a benefit and preserve complete state/recovery. The next useful investigations are backend transaction stages on the actual expanded native word, control-state transition cost, and driver/runtime projection and upload stages. Prefer bounded caches and revision-based invalidation to another general compatibility framework. Any projection or geometry cache must retain exact full-detail picking and be invalidated by every relevant camera, size, and visibility change.

The current release target has not been fully met: 100 ms instant turns and continuous full-detail 30 FPS remain unresolved, and one short explorer activation sample narrowly missed 50 ms. The measurements establish a more responsive working interface, not a claim that all performance requirements passed. See the [development log](DEVELOPMENT_LOG.md), [structure guide](STRUCTURE_EXPLORER.md), and [limitations](LIMITATIONS_AND_ROADMAP.md).
