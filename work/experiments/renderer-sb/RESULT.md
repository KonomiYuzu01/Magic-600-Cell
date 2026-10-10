# E-2.4-01 S-B result card

This is the experiment card ([protocol section 2](../../../docs/progress/1.0/stage-2-experiment-protocol.md)) for packet [E-2.4-01](../../../docs/progress/1.0/packets/renderer/E-2.4-01-sb-probe.md). It is filled in as results arrive: an item that has not run is listed as open, never estimated. Raw captures, traces and run records stay private; only the gate summary is published.

| Field | Content |
|---|---|
| ID | E-2.4-01 |
| Question | Does a bare Direct3D 12 renderer draw all 259,800 sticker slots at full detail with the most complex turn animation within the selection gate? |
| Decision it feeds | The day-7 go/no-go (E-2.4-04), the start of the S-A2 and S-D geometry ports, and the selection (E-2.4-05). |
| Hypothesis | No pass or fail was predicted before the runs; the packet expected a number for each scene and a working drawing method. |
| Method | The probe in this directory, the capture script `probe/run_scene.ps1`, PresentMon 2.6.0.0 and `tools/perf/renderer_gate.py` (below). The handoff test `handoff/` for item 4. |
| Time box | 1.5 days |
| Kill criteria | Two drawing methods both below 50 % of the gate. |
| Evidence class | Actual Windows/DirectX and performance, valid only for build identity `2b5bf5e6…` under the conditions below. |
| Result | W3 meets the gate: three valid runs; pooled average 778.37 fps, pooled p99 1.546 ms, peak VRAM 81.7 MB. The fenced resource handoff works on the same device, to a second device in a child process and to a D3D11 consumer, with every frame verified. The label check caught each of the four injected faults on the GPU. |
| Decision | Open. The day-7 go/no-go is an Astra gate ruling, and it also needs the S-A2 and S-D results. |

## Shrink anchor change, 10 October 2026

- The owner changed the shrink anchor (`docs/wiki/decisions/owner-decisions-2026-10-10-anchor.md`). [SPEC.md](SPEC.md) section 3 now anchors each sticker's shrink at the area-weighted centroid of its triangles, computed from the unchanged mesh, instead of at `assets/mesh_centers.f32`. No asset and no model identity changed.
- **What was wrong.** The old anchors do not move with the stickers. In the measured W3 scene, the end of every turn moved 4,120 of the 4,600 moved slots by up to 0.0118 world units. `check_assets.py` now checks turn-end continuity over all 4,605 animated slots:
  - with the new anchors, the maximum is 7.09e-8;
  - with the old anchors, as a control, it is 0.0144.
- **The reference changed.** It was regenerated as format `magic600-sb-reference-v2`, with the anchor digest recorded. Sampled projections moved by up to 0.0074 in normalised device coordinates. A probe built before the change therefore fails the geometry check against the new reference.
- **The W3 result above still stands for its build.** It belongs to build identity `2b5bf5e6…` with the old anchors, and its figures are unchanged.
- **The changed probe is not yet measured.** It has a new build identity. Its re-acceptance has not run: the GPU geometry check against the new reference, then three cold W3 runs judged the same way (E-2.4-0J item 6). Loading the assets refuses anchors other than the pinned ones, so neither this probe nor S-A2 nor the S-D scene can upload others. The CPU self-test passes: it checks the anchor digest, and under upward rounding the recomputed anchors are refused (source and compilation evidence only).

## Build and tools

- Probe source: commit `4d5da1a`, unchanged through `c6f7730`.
- Build identity (SHA-256 of `sb_probe.exe` and the four DXIL blobs in fixed order): `2b5bf5e6c26c86eac97800fc9dbccbdb10c8648668c0cf306bac9023d2f04c14`. It was recomputed from the build directory after the runs.
- Toolchain: MSVC 14.51, Windows SDK 10.0.26100.0 with its DXC, shader model 6.0.
- Capture: `probe/run_scene.ps1` at commit `c6f7730`, with PresentMon 2.6.0.0 and `--v1_metrics --qpc_time`.
- Judge: `tools/perf/renderer_gate.py` (format `magic600-renderer-gate-v1`). It counts every present of the declared swap chain in [T0 + 10 s, T0 + 190 s), with nearest-rank p99 and no outlier removal.

## Drawing method

- One `DrawInstanced(30480, 600, 0, 0)` per frame.
- The vertex shader reads the vertex, sticker centre, cell frame, animation flag and integer label from structured buffers, then projects every vertex. There are no indices, culling, filtering or level of detail.
- Two frames are in flight.
- All 259,800 labels are uploaded at each adopted revision, through a two-entry upload ring, into one of three default-heap buffers.

Details: [probe/README.md](probe/README.md) and [SPEC.md](SPEC.md).

## W3 gate result (3 October 2026)

Scene W3:
- 190 ms turns run back to back from the trace start. Each turn applies generator 1 of cell 0 (4,605 animated slots) or, alternately, its inverse.
- The W2 camera rotation runs every frame.
- The exact label check runs inside every timed run.
- Three cold runs, each in a new process, 20 s apart.

| Run | Presents in interval | Average fps | p99 (ms) | Max (ms) | Peak VRAM (MB) | Label check |
|---|---|---|---|---|---|---|
| `20261003T045605948Z-w3-1` | 140,927 | 782.93 | 1.5625 | 3.2578 | 81.7 | pass |
| `20261003T045605948Z-w3-2` | 140,321 | 779.57 | 1.5374 | 2.3821 | 81.7 | pass |
| `20261003T045605948Z-w3-3` | 139,073 | 772.63 | 1.5321 | 2.3325 | 81.7 | pass |
| Pooled | 420,321 | 778.37 | 1.5461 | 3.2578 | | |

- Verdict `met`: each run and the pooled frames reach at least 30 fps with p99 at most 33.3 ms.
- Published summary: [results/w3-20261003T045605948Z-summary.json](results/w3-20261003T045605948Z-summary.json). It is the gate's own output, unchanged, and carries no paths or user names.
- Label check, every run: 1,010 revisions adopted and 1,010 copies read back. There was no mismatch, late adoption, missing copy or binding mismatch.
- Peak VRAM is the probe process's local video-memory usage (`QueryVideoMemoryInfo`). The budget is about 7 GB.

## Conditions

- GPU: RTX 4070 Laptop GPU 8 GB, NVIDIA driver 616.92 (user-mode driver 32.0.16.1692). It drives the display. The owner declared the NVIDIA discrete GPU (MUX) in high-performance mode.
- Power: mains in every sample (1,908 per run); Windows power mode best performance (`max_performance`).
- Display and output:
  - display 2560 × 1600 at 60 Hz, backbuffer 2560 × 1600;
  - sync interval 0 with tearing allowed (V-Sync off);
  - no MSAA, frame generation or upscaling.
- The average counts presents, not display refreshes. With V-Sync off on a 60 Hz display, each refresh shows parts of several presents.
- Window checks, every 100 ms (1,908 per run): the window was visible, not covered and in the foreground in every sample.
- Present modes:
  - In the gate interval every present was `Hardware: Independent Flip`, none was reported dropped, and every second had a displayed present.
  - The only composed presents, 5 to 10 per run, came about 4 s before the trace start, during the preroll.
- The owner attended every run (Astra ruling `20261003T033021Z-274f20af`). After each run the owner confirmed watching it throughout with no visible obstruction. The summary records this as operator-declared.

## Runs that are not gate evidence

- A preliminary single W3 run on the same build earlier that day (790.07 fps, p99 1.527 ms) gave `insufficient-runs`. It stays private and does not count.
- An earlier capture attempt stopped with "PresentMon did not stop within 60 s". PresentMon 2.6 handles a target's exit only when a later present arrives, so `--terminate_on_proc_exit` never fired after the probe's last frame.
  - The capture script now stops the session by name and runs one invocation at a time (`c6f7730`).
  - The last open review finding on that fix was settled by an [adjudicating experiment](../renderer-sb-packets/review-2-stop-adjudication.md).

## Injected label faults (3 October 2026)

Each fault from plan section 2 was injected once, in turn 20 of a 10 s W3 run of build `2b5bf5e6…`:
- the probe ran directly, without PresentMon, because no elevated terminal was available;
- each run was a new process on the owner's GPU, on mains power, with the window visible throughout.

| Fault | Exit | Label check | Mismatches | Late adoptions | Binding mismatches | Missing copies |
|---|---|---|---|---|---|---|
| `corrupt-label` | 2 | fail | 1 | 0 | 0 | 0 |
| `swap-same-colour` | 2 | fail | 2 | 0 | 0 | 0 |
| `delay-adoption` | 2 | fail | 1 | 2 | 1 | 0 |
| `stale-binding` | 2 | fail | 2 | 2 | 2 | 1 |

- Every fault failed the check in the expected way:
  - the two label faults with integer mismatches;
  - the two adoption faults with late or binding/copy violations.
- Each run had 52 revisions. `stale-binding` also recorded 53 copies.
- `renderer_gate.py` refused the four runs as unreadable, because they have no PresentMon capture. These are label-check runs, not gate captures.
- The gate's refusal of a failed label check in a complete capture is shown only by its self-test on fixtures.
- Summary: [results/neg-20261003T060641Z-summary.json](results/neg-20261003T060641Z-summary.json).

## D3D12 resource handoff (3 October 2026)

The handoff test is [handoff/](handoff/README.md) (packet [SB-C](../renderer-sb-packets/SB-C-handoff.md)). It has no window, model data or shaders. For frame `n`:
- a compute queue clears a private texture to a pattern that encodes `n` and copies it into one of three shared RGBA8 1024 × 1024 textures, then signals `ready = n`;
- the consumer waits for `ready = n`, copies the texture to a readback resource, and the CPU checks every texel against `n`;
- the consumer then signals `free = n`, and the producer reuses the slot only after `free` reaches `n - 3`.

| Mode | Consumer | Result on the RTX 4070 Laptop GPU, debug layer on |
|---|---|---|
| `same-device` | direct queue of the producer's device | pass, 1,000 of 1,000 frames verified |
| `second-device` | a child process with its own D3D12 device on the same adapter; one shared heap and two shared fences, opened from NT handles | pass, 1,000 of 1,000 |
| `d3d11-consumer` | a D3D11 device on the same adapter; three shared committed textures and the two fences | pass, 1,000 of 1,000 |
| `resize` | as `same-device`; at every 100th frame both queues drain and the ring is rebuilt at 640 × 480, 1280 × 720 or 1024 × 1024 | pass, 1,000 of 1,000, 9 rebuilds |

- With the debug layer, every mode ended with no unexpected live object and no debug error. The child process reported its own check.
- The same build also passed every mode without the debug layer, and on WARP with it.
- A second `D3D12CreateDevice` call on the same adapter in one process returned the same device pointer, both in the parent process and in the child.
- D3D11 can open a shared D3D12 texture only if the texture allows render-target use. Without that flag, `OpenSharedResource1` returned `E_INVALIDARG`. The shared-heap path keeps non-render-target textures.
- A drain may fail to confirm that the GPU has finished (review finding SB-C-001). The failing drain then ends its process at once with exit code 3, before anything is released. Windows reclaims the GPU objects only after the GPU stops using them.
- Recording that failure is best-effort. An error while recording it, such as an allocation failure, cannot unwind and release objects either. An adjudicating experiment on the owner's GPU showed the difference:
  - copies of the reviewed source and of the fix were instrumented the same way: the first failure message in the abort path threw `std::bad_alloc`, and a trace showed when a queue's destructor ran;
  - in the reviewed source, the consumer queue was released before the process ended; with a drain that failed only once, both queues were released in ordinary teardown and the run exited 1;
  - with the fix, every case exited 3 and no queue destructor ran.
- Injected failures of the producer's drains, and separately of the consumer's drains, ended every mode this way:
  - the failure is in the JSON;
  - no process was left behind;
  - for the consumer in the child process, the child exits 3 and the parent records its failure.
- The abort report is written on a separate thread with a 10 s limit (review finding SB-C-002). With its standard output going to a pipe that nobody read, an aborted 10,000-frame run still exited 3, 53 s after it started. The same run with a reader received the complete report and exited after 33 s. The machine was busier than in earlier runs, so these times are information only.
- Timings, from submission to the CPU's check of a frame, are information only. They include ring pacing and process effects, and they are not GPU time or frame time.
- Build: `sb_handoff.exe` SHA-256 `f8d252bb…`, MSVC 19.51.36260, Windows SDK 10.0.26100.0, Ninja. Source digests, counts, teardown checks per run and the experiment: [results/handoff-20261003T063841Z-summary.json](results/handoff-20261003T063841Z-summary.json). Adapter LUIDs and per-frame timings stay private.

## Acceptance items (packet section 1)

| # | Item | Status |
|---|---|---|
| 1 | Geometry check against `reference_geometry.py` | Done. It passed on the owner's GPU with this build: three cameras, three poses, 9,066 samples each, largest absolute error 1.4e-6 in the checked projected coordinates, and 30,480 vertices drawn for each of the 600 cells. |
| 2 | W1–W4 on the owner's machine | W3 done; W1, W2 and W4 not run yet. |
| 3 | Three cold W3 runs judged by `renderer_gate.py`, exact label check on every run | Done: `met`. Each of the four injected faults (plan section 2) failed the label check on the owner's GPU with this build (section above). Those runs had no PresentMon capture, so the gate's refusal of a failed label check in a complete capture is shown only by its self-test on fixtures. |
| 4 | D3D12 resource handoff with fences, on the same device and to a second device | Done on the owner's GPU: same device, second device in a child process, D3D11 consumer and resize all pass with every frame verified (section above). |
| 5 | Feature cost table (H-06) | Not measured yet; due stage day 5. The probe features with the `w3f` cost scene (`f554af5`) and the cost-table tool (`1c95357`) are committed. |

## Not claimed

- Nothing here holds for another build, driver, display mode or machine.
- W5 (W3 with the design track's look, H-01 to H-04) is not measured. The selection uses W5 when it exists.
- The probe is not a product renderer. Long sessions, input and V-Sync-on behaviour are not measured.
- The window checks are safeguards, not proof of continuous full-area visibility; the owner's attendance covers the rest.
- The handoff result is not a Godot or Qt result. Each framework still needs its own smoke test with its import or native-handle API, display, resize and teardown.
