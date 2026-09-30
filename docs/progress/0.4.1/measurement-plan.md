# Formal measurement plan, 3 x 100 (0.4.1 step 6)

This plan is for measuring the frozen step-5 candidate on the owner's Windows machine. It was written in a cloud session, so it contains no measurement. Evidence class: actual Windows/DirectX performance, valid only for the exact candidate build id.

## 1. Targets (B4-12)

| ID | Metric | Target | Timing interval |
|---|---|---|---|
| M1 | Instant turn latency | p95 at most 100 ms | from the final committing MouseUp until the committed native state is synchronized and one correct full frame is presented (the same interval as in `docs/DEVELOPMENT_PERFORMANCE.md`) |
| M2 | Structure and navigation interaction | p95 below 50 ms for each operation | from input message to completed synchronous paint (the same interval as in the 0.3 structure table) |
| M3 | Full-detail rotation frame rate | average of at least 30 fps | PresentMon frame times while the camera rotates continuously with all 259,800 stickers visible |
| M3b | Full-detail frame-time tail | 99th percentile reported | the same capture as M3; reported only, since B4-12 has no tail gate for M3 |

## 2. What "3 x 100" means

Three independent runs, with 100 measured samples per metric in each run:
- Each run starts from a cold application launch with a fresh isolated data directory created from the same fixture session. Never use the owner's session.
- In each run, 5 warmup samples come before the 100 measured samples and are excluded.
- For M3, one sample is one presented frame. A run is 100 consecutive frames taken after 5 s of warmup; also record a 60 s capture for context.
- The operation list, orbit, turn sequence and camera path are fixed in a script. Every run uses the same sequence.
- Before and after each run, record the full labelled-state hash. The hash after the run must equal the scripted expectation, otherwise the run is invalid.

## 3. Environment record, taken once per run

- Candidate build id and the package ZIP sha256.
- GPU model, driver version, CPU and RAM. Record only the component names; attach no raw machine diagnostic dump.
- Display resolution and refresh rate. The backbuffer uses native resolution with no scaling.
- Power: mains power, the Windows power mode and the vendor performance mode. Both must be the same for all three runs.
- Frame generation off, V-Sync off (or recorded if the driver forces it), no overlays, and no other GPU-heavy process running.

## 4. Capture steps

1. Install the tools with `python tools/toolchain/bootstrap.py install --profile native-performance`. This covers PresentMon, and pinning follows the lockfile.
2. Start the candidate with a fresh `--data` directory, then load the fixture session.
3. For M1 and M2, run the harness that sends real owned-HWND messages, the same method as 0.3. It writes one JSON line per sample, with the operation id and the time in ms taken from `QueryPerformanceCounter`.
4. For M3, start PresentMon aimed at the native host process only. Use CSV output with the `MsBetweenPresents` and `MsBetweenDisplayChange` columns. Then start the scripted rotation.
5. Stop the captures, then close the app normally. Normal exit is part of a valid run.
6. Repeat from step 2 twice more.

Store raw CSV and JSON files under `work/loop-memory/` (they are private). Publish only the summary table from section 6.

## 5. Statistics

- **Percentile:** nearest-rank. For n samples sorted ascending, p = sample at rank ceil(p/100 * n). With n = 100, p95 is the 95th value and p99 is the 99th.
- **Per run:** report the mean, p50, p95, p99 and max for each metric.
- **Gate decision:** use the pooled 300 samples, where p95 is rank 285. A target is met only when the pooled value meets it **and** every single run meets it as well.
- **Frame rate:** M3's fps is 1000 / mean(`MsBetweenPresents`) over the 100 frames. Do not average per-frame fps values.
- **Outliers:** do not remove any. A sample is excluded only when the run is invalid, for example on a state-hash mismatch or a crash. In that case the whole run is repeated.
- **Spread:** if the p95 values of the three runs differ by more than 20 %, record it and add one extra run. Do not pick the best three.

## 6. Result template

| Metric | Run 1 p95 | Run 2 p95 | Run 3 p95 | Pooled p95 (n=300) | p99 | Target | Met? |
|---|---|---|---|---|---|---|---|
| M1 instant turn (active orbit) | | | | | | at most 100 ms | |
| M1 instant turn (all pieces) | | | | | | at most 100 ms | |
| M2 &lt;operation&gt; (one row each) | | | | | | below 50 ms | |
| M3 full-detail fps (mean) | | | | | | at least 30 fps | |
| M3b frame time p99 | | | | | | report | |

State the build id, the environment record, the date and "3 independent cold-start runs, 100 samples each, nearest-rank percentiles". For each target that is not met, publish the measured value. B4-12 closes in 0.4.1 either way (owner decision, 2026-09-29).

## 7. Cost limits

- The harness scripts are written and tested once, in step 2 (baseline), and reused unchanged in step 6. The baseline uses the same plan on the 0.4 build.
- No model calls during measurement. A Sol `high` mechanical check of the summary table against the raw files is optional.
- Expected wall time on the owner's machine: about 30 to 45 minutes for three runs, with no owner input between runs.
