---
id: s-b-probe-results
type: evidence
status: verified
visibility: public
summary: The bare Direct3D 12 probe (S-B) meets the selection gate in the W3 scene on the owner's RTX 4070 Laptop GPU, its label check catches injected faults, and its fenced resource handoff works on the same device, to a second device and to D3D11; capture lessons from PresentMon 2.6.
related: [renderer-candidates]
supersedes: []
claims:
  - {id: w3-gate-met, evidence_kind: performance, path: work/experiments/renderer-sb/results/w3-20261003T045605948Z-summary.json, sha256: 93bd53efcdd70f20ac26d5554bb8238aeb4a12fdbb10f99ef710177cf3331cf0, checked_at: 2026-10-03}
  - {id: w3-label-check, evidence_kind: actual_windows_directx, path: work/experiments/renderer-sb/RESULT.md, sha256: 715ba25001ba14f978ec3de8d89d67fbb9132c73053e97720db8dc1065df2429, checked_at: 2026-10-10}
  - {id: label-faults-caught, evidence_kind: actual_windows_directx, path: work/experiments/renderer-sb/results/neg-20261003T060641Z-summary.json, sha256: f5332574599b1692cb6d6276ab272c9e452eb226eec537e61ab5a1ebe89bd4e3, checked_at: 2026-10-03}
  - {id: presentmon-named-stop, evidence_kind: source, path: work/experiments/renderer-sb/probe/run_scene.ps1, sha256: bb65a943faec9856b55562278a811710caf8ef0a1d05074e8daff8c2bfa1b70a, checked_at: 2026-10-03}
  - {id: handoff-works, evidence_kind: actual_windows_directx, path: work/experiments/renderer-sb/results/handoff-20261003T063841Z-summary.json, sha256: f906f0686421f7bb0b193acfb4b3700e81e2a87eaf714183f66b37b9e50414ed, checked_at: 2026-10-03}
  - {id: killed-stop-helper, evidence_kind: actual_windows_directx, path: work/experiments/renderer-sb-packets/review-2-stop-adjudication.md, sha256: e152368554e4e4471a0056fee45d430f1d5da1acb6ff803dff396a634a8732ff, checked_at: 2026-10-03}
---

# S-B probe results

These are the results of the bare Direct3D 12 probe, packet E-2.4-01; details are on its [result card](../../../work/experiments/renderer-sb/RESULT.md). They hold only on the owner's machine under the recorded conditions: the gate results for build identity `2b5bf5e6...`, the handoff results for `sb_handoff.exe` `f8d252bb...`.

## W3 gate scene, 3 October 2026

- `tools/perf/renderer_gate.py` gave the verdict `met` on three owner-attended cold runs:
  - pooled average 778.37 fps and p99 1.546 ms over 420,321 presents;
  - the slowest run averaged 772.63 fps.
- The gate needs at least 30 fps and p99 at most 33.3 ms, per run and pooled.
- The exact label check passed in every timed run, with 1,010 label revisions each.
- Peak VRAM was 81.7 MB; the budget is about 7 GB.
- Conditions:
  - RTX 4070 Laptop GPU on mains power, NVIDIA driver 616.92;
  - 2560 x 1600 at 60 Hz, V-Sync off with tearing;
  - no frame generation, upscaling or MSAA.
- Drawing method: one instanced draw of 30,480 vertices x 600 cells, projected in the vertex shader, with two frames in flight.
- Not shown yet, with status on the result card:
  - W5 with the design track's look;
  - W1, W2 and W4;
  - the injected-fault runs through the capture script, with PresentMon;
  - the feature cost table (H-06).

What it changes:
- The S-B condition for the S-A2 and S-D geometry ports is now met.
- Each port still waits for its framework's interop smoke test. The S-B resource handoff now exists, so those tests may start; the owner decides when.
- The day-7 go/no-go is not decided here: it is an Astra gate ruling across all candidates.

## Shrink anchor changed, 10 October 2026

- The W3 result above is for build `2b5bf5e6...`. That build anchored the sticker shrink at `assets/mesh_centers.f32`.
- Those anchors do not move with the stickers, so the end of every turn moved 4,120 of the 4,600 moved slots by up to 0.0118 world units. This is source evidence from the unchanged assets; nobody had seen it on screen.
- The owner changed the anchor to the area-weighted centroid of each sticker's triangles ([owner-decisions-2026-10-10-anchor](../decisions/owner-decisions-2026-10-10-anchor.md)). The turn-end error is now at most 7.09e-8.
- The changed probe has a new build identity, and its GPU re-acceptance (geometry check and three cold W3 runs) has not run yet.

## Injected label faults, 3 October 2026

- Each of the four faults from the renderer plan was injected once into a 10 s W3 run of build `2b5bf5e6...` on the owner's GPU, and each failed the probe's label check:
  - a corrupted label and two swapped labels of the same colour gave integer mismatches;
  - an adoption delayed by one frame and a stale binding gave late adoptions and binding mismatches.
- The probe ran without PresentMon, so these are label-check runs, not gate captures. The gate refused them as unreadable.

## D3D12 resource handoff, 3 October 2026

- The handoff test passes three textures in a ring between a producer and a consumer, with a `ready` and a `free` fence. The CPU checks every texel of every frame.
- On the RTX 4070 Laptop GPU with the debug layer, all four modes verified 1,000 of 1,000 frames:
  - `same-device`: a compute queue to a direct queue on one device;
  - `second-device`: a child process with its own device, one shared heap and two shared fences opened from NT handles;
  - `d3d11-consumer`: a D3D11 device opening the shared textures and fences;
  - `resize`: the ring rebuilt nine times at three sizes, with fence numbering kept.
- No mode left an unexpected live object or a debug error. The same build passed without the debug layer and on WARP.
- D3D11 can open a shared D3D12 texture only if it allows render-target use; otherwise `OpenSharedResource1` returns `E_INVALIDARG`.
- A second `D3D12CreateDevice` on the same adapter returns the same device in that process, so a second device needs a second process.
- A drain that cannot confirm completion must not lead to releases.
  - The failing drain itself ends the process before anything is released. Windows reclaims the GPU objects only after the GPU stops using them.
  - Recording the failure is best-effort, so an allocation failure while recording cannot release objects either. An experiment with an injected `std::bad_alloc` showed the reviewed source releasing a queue and the fixed source releasing none.
  - Injected producer and consumer drain failures showed this path in every mode.
  - The abort report has a 10 s limit, so an output pipe that nobody reads cannot keep the process alive.
- Framework smoke tests can reuse the frame pattern, the three-slot ring and the two-fence protocol. Each still needs its own import API, display, resize and teardown test.

## Capture lessons (PresentMon 2.6.0.0)

- `--terminate_on_proc_exit` did not end a capture after the target's last frame. PresentMon handles a target's exit only when a later present arrives (observed on the owner's machine).
  - Stop a capture explicitly with `--session_name <name> --terminate_existing_session`.
  - A capture stopped this way exits 0 after closing its CSV.
- A killed PresentMon leaves its trace session running, and a leftover session adds tracing work to every present.
  - The capture script refuses to start while a session named `PresentMon`, or one with its own session name, is running.
  - Its cleanup falls back to `logman stop`.
- Only one capture script runs at a time. A global mutex covers everything from the first session check to the last cleanup, so no invocation can stop another's session.
- A stop helper killed the way the script kills it does not stop a session started later under the same name. The same helper left alive does. An adjudicating experiment showed both.
