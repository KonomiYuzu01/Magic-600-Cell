---
id: s-b-probe-results
type: evidence
status: verified
visibility: public
summary: The bare Direct3D 12 probe (S-B) meets the selection gate in the W3 scene on the owner's RTX 4070 Laptop GPU, and its label check catches injected faults; capture lessons from PresentMon 2.6.
related: [renderer-candidates]
supersedes: []
claims:
  - {id: w3-gate-met, evidence_kind: performance, path: work/experiments/renderer-sb/results/w3-20261003T045605948Z-summary.json, sha256: 93bd53efcdd70f20ac26d5554bb8238aeb4a12fdbb10f99ef710177cf3331cf0, checked_at: 2026-10-03}
  - {id: w3-label-check, evidence_kind: actual_windows_directx, path: work/experiments/renderer-sb/RESULT.md, sha256: b0a12a61e233b7f3c2f8de0a2f8e8d6db215fa9dcecb8ea7cf172ed311550ea5, checked_at: 2026-10-03}
  - {id: label-faults-caught, evidence_kind: actual_windows_directx, path: work/experiments/renderer-sb/results/neg-20261003T060641Z-summary.json, sha256: f5332574599b1692cb6d6276ab272c9e452eb226eec537e61ab5a1ebe89bd4e3, checked_at: 2026-10-03}
  - {id: presentmon-named-stop, evidence_kind: source, path: work/experiments/renderer-sb/probe/run_scene.ps1, sha256: 14c30cd815a27ade80376051bba1472c30d5d284d6da03ebaaa8e74ab8069f77, checked_at: 2026-10-03}
  - {id: killed-stop-helper, evidence_kind: actual_windows_directx, path: work/experiments/renderer-sb-packets/review-2-stop-adjudication.md, sha256: e152368554e4e4471a0056fee45d430f1d5da1acb6ff803dff396a634a8732ff, checked_at: 2026-10-03}
---

# S-B probe results

These are the results of the bare Direct3D 12 probe, packet E-2.4-01; details are on its [result card](../../../work/experiments/renderer-sb/RESULT.md). They hold only for build identity `2b5bf5e6...` on the owner's machine under the recorded conditions.

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
  - the resource handoff (SB-C);
  - the feature cost table (H-06).

What it changes:
- The S-B condition for the S-A2 and S-D geometry ports is now met.
- Each port still waits for its framework's interop smoke test, and those tests wait for the S-B resource handoff.
- The day-7 go/no-go is not decided here: it is an Astra gate ruling across all candidates.

## Injected label faults, 3 October 2026

- Each of the four faults from the renderer plan was injected once into a 10 s W3 run of build `2b5bf5e6...` on the owner's GPU, and each failed the probe's label check:
  - a corrupted label and two swapped labels of the same colour gave integer mismatches;
  - an adoption delayed by one frame and a stale binding gave late adoptions and binding mismatches.
- The probe ran without PresentMon, so these are label-check runs, not gate captures. The gate refused them as unreadable.

## Capture lessons (PresentMon 2.6.0.0)

- `--terminate_on_proc_exit` did not end a capture after the target's last frame. PresentMon handles a target's exit only when a later present arrives (observed on the owner's machine).
  - Stop a capture explicitly with `--session_name <name> --terminate_existing_session`.
  - A capture stopped this way exits 0 after closing its CSV.
- A killed PresentMon leaves its trace session running, and a leftover session adds tracing work to every present.
  - The capture script refuses to start while a session named `PresentMon`, or one with its own session name, is running.
  - Its cleanup falls back to `logman stop`.
- Only one capture script runs at a time. A global mutex covers everything from the first session check to the last cleanup, so no invocation can stop another's session.
- A stop helper killed the way the script kills it does not stop a session started later under the same name. The same helper left alive does. An adjudicating experiment showed both.
