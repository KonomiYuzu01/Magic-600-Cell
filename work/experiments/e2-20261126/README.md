# LX-2's E2 after the key-stall fix (26 November 2026)

The U7 rerun of 26 November 2026 (suffix s1126e, product `c9-product-8539f09f74b29597`, main d7833eb8's tree c7e922bf)
held 60 fps and confirmed every key, but E2's p95 stayed over 100 ms in three faults: alloc_metered 115.7 ms,
forced_kill 115.8 ms, recursion_metered 248.8 ms. The median key-to-display rose from 67.4 ms (25 November) to 82.3 ms.

`e2_faults.py RUN_DIR --out FILE` reads one run's shell log (`events.jsonl`) and writes the per-fault breakdown below and a
model of E2 after the two changes; `u7-s1126e-faults-e2.json` is its record of that run (input by SHA-256, no path).

## Why the three faults missed

Every one of the 50 keys over 100 ms (9 in alloc_metered, 8 in recursion_metered, 32 in forced_kill, 1 in
recursion_native) is drawn exactly when LX-2's control probe publishes (the window's `recovered` row, within 20 ms of
the drawing frame's call), never when the twist's own publication is ready:

- The twist adopts on time: its engine revision is in the frame after the key's frame call, as for every key.
- In a metered fault's turn window, and in forced_kill's, the twist's installation ends the fault's view job, and the
  module sends its control probe, a new view job, at once (`m6h_run_view`). The probe's `Host::schedule` accepts it as
  the view's newest command, which supersedes the view's pending command: the twist's reference publication whenever
  its preparation, GPU work behind the core's ready fence, had not installed yet. The view then showed the old revision
  until the probe's own publication of the new revision was installed. The sitting session found the same race
  independently (the retest notes, "U7 s1126e, the E2 tail's cause").
- The probe runs only after the runtime has ended what it was doing: unwinding the cancelled deep recursion (about 170
  ms), freeing the cancelled allocation (about 30 ms), or a fresh child after the kill (about 30 to 50 ms). loop_metered
  stops at once, so its probe publishes within a frame and its E2 passed.

The rest of the median is the present queue: a product frame reaches the display three refreshes after its frame call
returns (49.6 ms p50), the maximum frame latency of 2 of the composition swap chain plus the compositor's own frame.

## The change, and the follow-up

- (b), this change, `renderer/bridge/host/src/view.rs`: a view job's schedule first lets a reference publication still
  being prepared install, pumping the display host for at most `REFERENCE_WAIT` (20 ms). The twist is then drawn in
  its adoption frame, and the probe's publication replaces it later, as when the probe comes a moment after the
  upload. The display host's newest-command rule (RM.20) is unchanged; past the bound the schedule supersedes the
  publication as before.
- (a), a follow-up (dropped for 26 November by the integrator: it lowers only the median, and it carries risk to the
  renderer gate the night before a long run): `sa2_surface_latency`, added to ABI 5 without a version change, would set
  the maximum frame latency of the surface's swap chains before the surface is attached, and the product shell would
  ask for 1, the scene runs (renderer/acceptance) keeping 2. Written and checked headless (37 CPU checks, the core,
  shell and bridge tests) and set aside unmerged; it needs an on-screen check of 60 fps and the frame-time p99 first.

## Model (not a measurement)

From the same run: (b) draws each late key in its adoption frame, as the keys with lag 0 are drawn; (a) shows every
frame one refresh (16.67 ms) sooner. E2 p95, ms:

| Fault | measured | (b) | (a) | (a) and (b) |
| --- | --- | --- | --- | --- |
| alloc_metered | 115.69 | 82.59 | 99.02 | 65.92 |
| recursion_metered | 248.78 | 82.48 | 232.12 | 65.82 |
| forced_kill | 115.80 | 82.57 | 99.13 | 65.91 |
| loop_metered | 82.51 | 82.50 | 65.84 | 65.83 |
| alloc_ceiling | 82.55 | 82.52 | 65.88 | 65.85 |
| recursion_native | 82.55 | 82.51 | 65.88 | 65.84 |
| loop_native | 82.51 | 82.49 | 65.84 | 65.82 |

(b) alone brings every fault under 100 ms, to the on-time keys' 82.5 ms; (a) would take a further refresh off every
key. U7 is the measurement.
