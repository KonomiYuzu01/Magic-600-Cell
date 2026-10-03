# E-2.4-01 S-B result card

This is the experiment card ([protocol section 2](../../../docs/progress/1.0/stage-2-experiment-protocol.md)) for packet [E-2.4-01](../../../docs/progress/1.0/packets/renderer/E-2.4-01-sb-probe.md). It is filled in as results arrive: an item that has not run is listed as open, never estimated. Raw captures, traces and run records stay private; only the gate summary is published.

| Field | Content |
|---|---|
| ID | E-2.4-01 |
| Question | Does a bare Direct3D 12 renderer draw all 259,800 sticker slots at full detail with the most complex turn animation within the selection gate? |
| Decision it feeds | The day-7 go/no-go (E-2.4-04), the start of the S-A2 and S-D geometry ports, and the selection (E-2.4-05). |
| Hypothesis | No pass or fail was predicted before the runs; the packet expected a number for each scene and a working drawing method. |
| Method | The probe in this directory, the capture script `probe/run_scene.ps1`, PresentMon 2.6.0.0 and `tools/perf/renderer_gate.py` (below). |
| Time box | 1.5 days |
| Kill criteria | Two drawing methods both below 50 % of the gate. |
| Evidence class | Actual Windows/DirectX and performance, valid only for build identity `2b5bf5e6…` under the conditions below. |
| Result | W3 meets the gate: three valid runs; pooled average 778.37 fps, pooled p99 1.546 ms, peak VRAM 81.7 MB. |
| Decision | Open. The day-7 go/no-go is an Astra gate ruling, and it also needs the S-A2 and S-D results. |

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

## Acceptance items (packet section 1)

| # | Item | Status |
|---|---|---|
| 1 | Geometry check against `reference_geometry.py` | Done. It passed on the owner's GPU with this build: three cameras, three poses, 9,066 samples each, largest absolute error 1.4e-6 in the checked projected coordinates, and 30,480 vertices drawn for each of the 600 cells. |
| 2 | W1–W4 on the owner's machine | W3 done; W1, W2 and W4 not run yet. |
| 3 | Three cold W3 runs judged by `renderer_gate.py`, exact label check on every run | Done: `met`. The four injected-fault runs that must be refused (plan section 2) have not run on the GPU yet. The self-test covers the same faults on in-memory fixtures only. |
| 4 | D3D12 resource handoff with fences, on the same device and to a second device | Not done (SB-C, next). |
| 5 | Feature cost table (H-06) | Not measured yet; due stage day 5. |

## Not claimed

- Nothing here holds for another build, driver, display mode or machine.
- W5 (W3 with the design track's look, H-01 to H-04) is not measured. The selection uses W5 when it exists.
- The probe is not a product renderer. Long sessions, input and V-Sync-on behaviour are not measured.
- The window checks are safeguards, not proof of continuous full-area visibility; the owner's attendance covers the rest.
