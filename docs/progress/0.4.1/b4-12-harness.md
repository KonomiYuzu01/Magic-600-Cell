# B4-12 measurement harness contract (0.4.1 steps 2 and 6)

This is the contract for the harness that measures the B4-12 targets of the [measurement plan](measurement-plan.md). The step 2 baseline and the step 6 formal runs use the same harness, unchanged. It fixes what each metric times, how input is delivered, which fixture every run starts from, the files a run writes, and when a run is valid. It contains no measurement.

**Status (1 October 2026).** The owner cancelled the 0.4.1 release and every 0.4 measurement ([owner-decisions-2026-10-01](../../wiki/decisions/owner-decisions-2026-10-01.md)). This contract, the fixture builder, the turn probe and the summary tool are kept because the stage 2.4 renderer gate will reuse them. The native harness (`B412Checks.cs`) and the runner (`run_b412.py`, `b412_environment.py`) were never built, so the `run_b412.py` commands in section 3 do not exist; the fixture builder, turn probe and summary commands do.

## 1. Metrics and series

| Series | Metric | Operation | Start | End |
|---|---|---|---|---|
| `m1-active` | M1, active orbit (2,400 stickers visible) | one click turn on the puzzle viewport | QPC immediately before the final committing button-up message is posted | CPU return of the first Present, after adoption, whose frame shows the adopted snapshot at full detail |
| `m1-all` | M1, all 259,800 stickers visible | the same | the same | the same |
| `m2-bank` | M2, bank navigation | one press of the unmodified key bound to `bank-previous` (F9 in every default key set), which switches between two fixed banks of the working orbit (`<orbit>-A` and `<orbit>-B`) | QPC immediately before the key-down message is posted | return of the first actual `WM_PAINT` of the bank caption that names the new bank, after the bank request completed, followed by `GdiFlush` |
| `m2-local` | M2, Local-centre navigation | one press of the unmodified key bound to `local-center-grip` in the Views key set (KeyN by default), which centres Local on the selected Grip's cell; two fixed Grips on different cells alternate, selected outside the timed interval | QPC immediately before the key-down message is posted | return of the actual `WM_PAINT` of the Local view that shows the new centre with its 433 actual labels, followed by `GdiFlush` |
| `m3` | M3 and M3b, full-detail rotation | continuous scripted camera rotation with all 259,800 stickers visible | rotation start marker | rotation stop marker; frames come from PresentMon |

Method decisions (integrator, 30 September 2026):
- **M2 operations.** M2 is bank navigation and Local-centre navigation, one row each. The 0.3 structure table's operations (Structure-tab colour selection, graph focus, graph activation) belong to the 0.3 host; the 0.4 product compiles those sources but never creates those controls, so they cannot be measured on a 0.4.1 candidate. Bank and Local-centre navigation are the navigation categories of the retained B4-12 contract (1.0 architecture V11.1).
- **Input.** Every timed input is an owned-HWND message posted to the window's queue (`PostMessage`), so it passes the production message filter and window procedures, as in 0.3. M1 uses `WM_MOUSEMOVE`, then `WM_LBUTTONDOWN` and `WM_LBUTTONUP` for a forward turn, or `WM_RBUTTONDOWN` and `WM_RBUTTONUP` for its inverse, at one fixed click point with no travel. M2 posts `WM_KEYDOWN` and `WM_KEYUP` with the scan code of the key that the active key set binds to the command; only unmodified keys are used, because a posted message does not change the keyboard state that the input filter reads for modifiers. Untimed setup (Grip selection, window activation) may use the same messages or the shell's command entry.
- **M1 end.** As in 0.3, automatic rendering is paused for the M1 series, so a timer frame is never counted and ordinary timer delay is excluded. When the turn's send completes (the viewport is re-enabled), the harness renders frames with the renderer's own synchronous draw until one is valid. A valid frame shows the adopted snapshot, draws every visible sticker (drawn = visible) and has a native colour field equal to the adopted colours.
- **M2 end.** Automatic rendering of the main viewport is paused for the M2 series, as in 0.3. The end is the return of the surface's actual `WM_PAINT` followed by `GdiFlush`, as in the 0.3 auxiliary timings.
- **M3 path.** The camera path is scripted, not dragged: before each frame of the production render timer, the harness applies one fixed rotation step (the same plane and angle every frame, every run) and marks the scene changed. The production timer keeps its own cadence. The fixture turns adaptive motion off, and every frame in the capture must draw all 259,800 stickers.
- **Protection.** In 0.4, the 0.4 engine adds every orbit that a commit brings from unsolved to solved to the protected set, and a later live turn that moves a protected orbit is rejected. The inverse turn of an M1 pair re-solves every orbit that the forward turn unsolved, so after every M1 pair (the click-point trial pair included) the harness releases the new protection with the untimed engine command `protect` with an empty orbit list, as the 0.4 latency harness does (`NativeLatencyChecks.ReleaseNewProtection`), and records each release. The release changes no labels and is never inside a timed interval. The headless turn probe (`tools/perf/turn_probe.py`) observed a release after every pair.
- **Warmups.** M2 has 5 warmup samples before the 100 measured ones. M1 has 6 (three inverse pairs), because every turn must be paired to return to the fixture state; this meets the plan's minimum of 5. M3 warmup is the first 5 s of rotation.
- **No early stop.** A slow series is measured in full. A single sample that takes more than 60 s makes the run invalid.
- **Observation.** No profiler or tracer runs during an M1 or M2 run. PresentMon runs only in M3 runs, where it is the measurement.

## 2. Fixtures

Every run starts the application cold with a fresh `--data` directory copied from a fixture template. The fixture builder creates each template once, headlessly, and records it in `fixture.json`.

| Profile | Contents | Use |
|---|---|---|
| `basic` | Home plus one fixed legal two-generator word, no protection, no pending preview | all gate series |
| `history-<n>` | `basic` plus `n` named checkpoints (default 400) | attribution only, for session-size costs such as screening finding X-02; never a gate row |

`fixture.json` (format `magic600-b412-fixture-v1`): `profile`, `word` (the primitive IDs), `checkpoints`, `state_hash` (the scripted expectation, computed independently of the session), `head`, `journal_depth`, `database_sha256`, `builder_sha256` (the builder file digest), `engine_sources` (digests of the engine sources and model manifest the builder imported), `created_utc`.

Per-series untimed setup, applied by the harness after start: filter `active` (`m1-active`, `m2-*`) or `everything` (`m1-all`, `m3`); view preference `native_adaptive_motion` false for `m3` only; main window maximized on the primary display. The harness checks the baseline hash against `fixture.json` before the first sample.

## 3. Commands

One command per metric, run with the engine environment from the repository root:

```
tools/.venv/engine/Scripts/python.exe work/experiments/magic600-04/tests/run_b412.py fixture --profile basic
tools/.venv/engine/Scripts/python.exe work/experiments/magic600-04/tests/run_b412.py run --metric m1-active
tools/.venv/engine/Scripts/python.exe work/experiments/magic600-04/tests/run_b412.py run --metric m1-all
tools/.venv/engine/Scripts/python.exe work/experiments/magic600-04/tests/run_b412.py run --metric m2-bank
tools/.venv/engine/Scripts/python.exe work/experiments/magic600-04/tests/run_b412.py run --metric m2-local
tools/.venv/engine/Scripts/python.exe work/experiments/magic600-04/tests/run_b412.py run --metric m3
tools/.venv/engine/Scripts/python.exe work/experiments/magic600-04/tests/run_b412.py series --runs 3
python tools/perf/b412_summary.py <run directories> --out <summary.json>
```

The runner uses the fixture builder, which can also be run on its own: `tools/perf/b412_fixture.py build --profile <profile> --output <directory>`, `copy --fixture <directory> --data <fresh directory>` and `verify --data <directory> --fixture <directory>`. For attribution only, `tools/perf/turn_probe.py --output <fresh directory>` times the engine side of the native turn route headlessly, from a fresh `basic` fixture. It never yields B4-12 results.

`run` copies the fixture into a fresh data directory and calls `run_postapproval.py --focus b412`, which compiles the harness with its inputs bound in `build.json`, starts the owned engine on that data directory and runs the harness. For `m3` the runner starts PresentMon before the harness and stops it after the application exits. The runner then merges the harness record, the build receipt, the fixture and the exit status into `run.json`.

Runner to harness, by environment variable: `MAGIC600_NATIVE_FOCUS=b412`, `MAGIC600_B412_SERIES` (the series), `MAGIC600_B412_EXPECTED_HASH` (the fixture's `state_hash`), and for `m3` `MAGIC600_B412_M3_READY` (a file the runner creates once PresentMon is capturing; the harness waits up to 60 s for it) and `MAGIC600_B412_M3_SECONDS` (rotation length, default 75). `series` runs every metric `--runs` times as separate cold runs, in a fixed order, adds one run for a metric whose run p95 values differ by more than 20 %, and then writes the summary. `--fixture history-400` selects an attribution fixture; its runs are labelled attribution.

## 4. Run directory

Private, under `work/loop-memory/perf/b412/<run id>/`, where the run id is `<UTC yyyymmddThhmmssZ>-<series>`:

| File | Content |
|---|---|
| `harness.json` | the native harness's own record (section 5) |
| `run.json` | the run record (section 5), written by the runner |
| `samples.jsonl` | one JSON line per sample, warmups included (section 5) |
| `environment.json` | the full environment record (section 6); private |
| `build.json` | the build receipt of the harness executable |
| `run.log` | the harness console output |
| `presentmon.csv` | M3 only: the PresentMon capture of the host process |

The directory is created with `exist_ok=False`. Nothing in it is published; the summary tool publishes a sanitized summary.

## 5. Records

`samples.jsonl`, one object per sample (format `magic600-b412-sample-v1`):

| Field | Type | Meaning |
|---|---|---|
| `format` | string | `magic600-b412-sample-v1` |
| `series` | string | `m1-active`, `m1-all`, `m2-bank` or `m2-local` |
| `operation` | string | the operation id, for example `turn P2 left`, `turn P2 right`, `bank Macro`, `local C7` |
| `index` | int | 0-based position in the series, warmups included |
| `warmup` | bool | true for warmup samples |
| `input` | string | the timed input: `post-message` for every gate sample |
| `input_ticks` | int | QPC value at the start boundary |
| `adopt_ticks` | int or null | QPC value when the send completed (M1: the viewport was re-enabled; M2: the send task completed) |
| `end_ticks` | int | QPC value at the end boundary |
| `ms` | number | `(end_ticks - input_ticks) * 1000 / qpc_frequency` |
| `end_kind` | string | `present-return` (M1) or `wm-paint-gdiflush` (M2) |
| `frames_rendered` | int or null | M1: frames the harness drew before a valid one |
| `visible_slots`, `drawn_slots` | int or null | M1: of the valid frame |
| `state_hash` | string | authoritative state hash after the sample |
| `head` | string | journal head after the sample |
| `snapshot_revision` | int | the adopted native snapshot revision |

`harness.json` (format `magic600-b412-harness-v1`): `series`, `pid`, `qpc_frequency`, `qpc_high_resolution`, `status`, `invalid_reasons`, `expected_hash`, `baseline_hash`, `final_hash`, `warmups`, `measured`, `window`, `click_points`, `keys`, `rotation` (with `start_qpc`, `stop_qpc`, `frames`, `min_drawn`, `max_drawn`), `protection_releases` (M1: `after_pair` and `orbits` per release; else empty), `foreground_losses`, `started_utc`, `finished_utc`.

`run.json` (format `magic600-b412-run-v1`), which carries the harness fields forward: `run_id`, `series`, `fixture` (profile, expected hash, baseline hash, final hash), `attribution` (bool), `status` (`valid` or `invalid`), `invalid_reasons` (list), `qpc_frequency`, `qpc_high_resolution`, `warmups`, `measured`, `window` (client and backbuffer size, DPI, maximized), `click_points` (M1), `keys` (M2: command, key, scan code), `rotation` (M3: plane, step angle, start and stop QPC markers, frames drawn, minimum drawn count), `protection_releases` (M1), `presentmon` (M3: version, arguments, process id, CSV sha256), `build` (`build_identity`, `executable_sha256`, harness file digests), `started_utc`, `finished_utc`, `exit` (`normal` and the exit code, written by the runner after the application closes).

A run is valid only when all of these hold; otherwise `status` is `invalid` and every reason is listed:
- the baseline hash equals the fixture's `state_hash`, and the final hash equals it again (M1 turns come in inverse pairs; M2 and M3 change no labels);
- every sample met its end condition within 60 s, and the full 259,800 labels were checked against the fixture after every M1 pair and at the end of every series;
- the measured window kept the Windows foreground for every sample;
- M3: every frame between the markers drew 259,800 stickers, and at least 100 frames fall after the 5 s warmup;
- the application exited normally with exit code 0.

An invalid run is kept and repeated as a whole, never trimmed.

## 6. Environment record

Taken once per run by the runner. The full record stays private; the summary publishes only the sanitized fields marked *public*.
- *public* build identity, executable sha256, package ZIP sha256 (`null` for a source build);
- *public* GPU names and driver versions, the presenting adapter if the device reports it (otherwise `unknown`), CPU name, RAM size;
- *public* display resolution, refresh rate, DPI, backbuffer size, the device's presentation interval (V-Sync);
- *public* power source, Windows power mode;
- *public* owner-declared conditions from `work/loop-memory/perf/conditions.json`: vendor performance mode, frame generation, driver V-Sync setting, overlays; a missing file is recorded as `undeclared`;
- *public* whether another process was using the GPU at start (a count, no process names);
- private: OS build string, machine-specific paths and names, raw tool output.

## 7. Summary

`tools/perf/b412_summary.py` reads run directories and writes one JSON summary and a Markdown table in the shape of the measurement plan's section 6.
- Only valid, non-attribution runs count. Invalid runs are listed with their reasons.
- Per run and series: count, mean, p50, p95, p99 and max of `ms` over non-warmup samples, nearest-rank (rank `ceil(p/100 * n)`).
- Pooled per series: the same statistics over all counted runs; with 3 runs p95 is rank 285. A target is met only if the pooled p95 and every run's p95 meet it.
- Spread: flag a series whose run p95 values differ by more than 20 % (max / min - 1).
- M3: per run, the 100 PresentMon frames of the host process that start 5 s after the rotation start marker; fps = 1000 / mean(`MsBetweenPresents`); M3b = nearest-rank p99 of `MsBetweenPresents` over the same frames; also the 60 s context statistics, and `MsBetweenDisplayChange` statistics when the column exists.
- The published summary carries no absolute path, machine name, user name or raw diagnostic.

## 8. Open items

- The PresentMon command line (process filter, output file, timestamp format, stop) is fixed only after the owner pins a PresentMon version and its `--help` output is recorded. Until then `m3` refuses to run.
- The attribution fixture size (400 checkpoints) is a starting point for screening finding X-02, not a claim about typical sessions.
