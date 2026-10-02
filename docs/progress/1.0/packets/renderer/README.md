# Stage 2.4 renderer packets

Status: **packets for the local Windows session, not results.** They were written in a Linux cloud session on 2 October 2026 from [renderer-experiment-plan](../../renderer-experiment-plan.md) (Astra-checked on 1 October 2026) and the gate in [renderer-candidates](../../../../wiki/decisions/renderer-candidates.md). No packet here has been run. Nothing in this folder is Windows, Direct3D 12 or performance evidence.

The plan is the authority for the workload (W1 to W5), the measurement rules and the decision points. These packets turn its sections 3 and 5 into wrapper-ready seven-part packets (`templates/problem-packet.md`), one per experiment, in the order the window runs them. Where a packet and the plan disagree, the plan wins and the packet is fixed.

## Sequence (stage days count the owner's working days)

| Stage day | Packet | Who runs it | Output |
|---|---|---|---|
| by 3 | [E-2.4-00 readiness](E-2.4-00-readiness.md) | owner and local session | tools at approved exact versions; a minimal D3D12 program presents; `renderer_gate.py` parses a short capture (and refuses it for length) |
| 3–4 | [E-2.4-01 S-B bare Direct3D 12 probe](E-2.4-01-sb-probe.md) | local session (Codex implements, Claude integrates) | probe, reference geometry, W1–W4 runs, H-06 cost table |
| 5–8 | [E-2.4-02 S-A2 Godot 4.7 .NET](E-2.4-02-sa2-godot.md) and [E-2.4-03 S-D Qt Quick RHI](E-2.4-03-sd-qt.md), in parallel | local session | interop smoke test first; geometry port only after an S-B W3 run meets the gate; H-09 by day 8 |
| 9 | [E-2.4-04 day-7 go/no-go](E-2.4-04-day7-gate.md) | Astra gate ruling | go or no-go per candidate; H-07 to the design track |
| 10–13 | remaining candidates: W5 with the design handoffs | local session | three formal runs each of W3 and W5 |
| 14 | [E-2.4-05 selection](E-2.4-05-selection.md) | local session, Astra review | selected renderer (H-10) or the recorded failure |

The first experiment is the bare Direct3D 12 probe, and its item 4 (a resource handoff with fence synchronisation, same device and second device) is the interop probe the framework smoke tests build on. The day-7 go/no-go uses the gate exactly; a near pass is a fail unless the owner names an exception.

## Fixed inputs every packet repeats

- **Gate:** all 259,800 sticker slots at full detail with the most complex animation (W3 trace, and W5 once the finalist preset exists); average at least 30 fps **and** 99th-percentile frame time at most 33.3 ms; PresentMon 2.6.0.0 frame times; native resolution; no DLSS, no frame generation, no upscaling; three cold runs of 180 s after a 10 s warmup, every run and the pooled frames passing; RTX 4070 Laptop GPU 8 GB on mains power, high-performance mode, discrete GPU confirmed; peak VRAM at most about 7 GB (above that is a recorded risk, not a pass criterion).
- **Judge:** `python tools/perf/renderer_gate.py <run-dir> [...] --out <summary.json>`. Its run format is `magic600-renderer-run-v1` (`run.json`, `presentmon.csv`, `trace.jsonl` per run directory).
- **NVIDIA features** (Reflex, DLSS, SER, NVAPI, Nsight Aftermath hooks) are optional, off by default, never on the correctness path and never on the day-7 critical path.
- **Correctness:** rendering never changes mechanical state or relabels pieces; labels come from the engine side; colour is a function of the label only.
- **Data:** synthetic geometry and labels from `assets/` only. No personal session, no user data directory.
- **Private captures** stay under `work/loop-memory/perf/renderer/`. Only the `renderer_gate.py` summary is published.
- **Licences:** no Managed DirectX DLLs; Streamline, DLSS and NVAPI SDK terms need the owner before adoption.

## Check scripts in this repository

| Script | What it checks | Runs on |
|---|---|---|
| `python tools/perf/check_renderer_packets.py` | every packet here has the seven parts, a parseable contract where one is needed, the gate numbers above, and the NVIDIA rule | any OS |
| `python tools/perf/check_renderer_assets.py` | the five workload assets match `assets/manifest.json` digests and the expected counts (600 cells, 433 stickers per cell, 259,800 slots, 30,480 base vertices) | any OS |
| `python tools/perf/renderer_gate.py` | the gate on real run directories | any OS; real runs only from the owner's machine |

## Checks not run in the cloud session

- Every Windows, Direct3D 12, PresentMon, VRAM and frame-time measurement.
- Building or running any probe, Godot project or Qt project.
- `tests/native/NativeHostRegression.cs` (Windows only; this folder does not change native code).
- `bootstrap.py pin` and `bootstrap.py approve` (owner, on Windows; see [E-2.4-00](E-2.4-00-readiness.md)).
