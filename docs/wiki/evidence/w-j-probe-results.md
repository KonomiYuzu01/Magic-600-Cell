---
id: w-j-probe-results
type: evidence
status: verified
visibility: public
summary: The W-J pose probe (E-2.4-0J) on the owner's RTX 4070 Laptop GPU, 10 October 2026. GPU geometry and lattice checks pass with the area-centroid anchor, the five negative tests fail as required, and three cold runs per jumbled fixture (S4, I_a, I_b) and three cold W3 runs meet the gate at about 610 to 615 fps; overlay costs are unmeasured, so W-J is not yet accepted as a whole.
related: [s-b-probe-results, owner-decisions-2026-10-10-anchor, owner-decisions-2026-10-09-jumbling, renderer-candidates]
supersedes: []
claims:
  - {id: wj-s4-gate-met, evidence_kind: performance, path: work/experiments/renderer-wj/results/wj-S4-20261010T184006692Z-summary.json, sha256: 54b4bf3b1ad1cabc6b6bd589288ddfbe6f2e2c7972c7b7d38cdfdf7eb105bff1, checked_at: 2026-10-10}
  - {id: wj-i-a-gate-met, evidence_kind: performance, path: work/experiments/renderer-wj/results/wj-I_a-20261010T191829654Z-summary.json, sha256: e622ec1a3b328074d1a584f7efbb359a786369d7f6207ca83ec61a4e53c1d3f7, checked_at: 2026-10-10}
  - {id: wj-i-b-gate-met, evidence_kind: performance, path: work/experiments/renderer-wj/results/wj-I_b-20261010T193029102Z-summary.json, sha256: 3ce21f3f2b5d70644bc5c013f5517e9c20fd904b3cce5f85ba6d4b34c6857330, checked_at: 2026-10-10}
  - {id: wj-w3-reaccepted, evidence_kind: performance, path: work/experiments/renderer-wj/results/w3-20261010T194430557Z-summary.json, sha256: 3cc627af916c9e564851d57b2c853ef389e24ddf5b634629908ce6e0cfa7c868, checked_at: 2026-10-10}
  - {id: wj-result-card, evidence_kind: actual_windows_directx, path: work/experiments/renderer-wj/RESULT.md, sha256: 56a0fd7db1279acf9882b2b42c1b15c7bf521c3f3537ba620faf1aae30191f13, checked_at: 2026-10-10}
---

# W-J probe results

W-J is the jumbling puzzle on the renderer path, packet E-2.4-0J. The probe is a copy of the S-B probe that adds a per-piece pose: an index into a table of 4 x 4 matrices, applied after the home shrink. The owner ran it in an attended GPU session on 10 October 2026. Details are on the [result card](../../../work/experiments/renderer-wj/RESULT.md).

The results hold only on the owner's machine, under the recorded conditions, for build identity `74b22fec...`. The build uses the area-centroid shrink anchor ([decision](../decisions/owner-decisions-2026-10-10-anchor.md)).

## Correctness on the GPU

- **Geometry.** The GPU output matched the standard-library reference in 18 of 18 comparisons for each fixture (S4, I_a and I_b: six states, three cameras) and in 9 of 9 for W3. The maximum error was 1.91e-6. Every one of the 600 cells ran 30,480 vertex invocations.
- **Lattice.** The lattice check passed for both control states, lattice-start and lattice-retained, with 259,800 slots each and no label, centre, bound or lattice failure. This item had failed with the old anchor.
- **Negative tests.** A clean control passed first. Each of the five injected faults then failed the probe's check with exit 2, with its own failure:
  - a corrupted index and two swapped pieces of the same colour gave element mismatches in the index;
  - a stale pose gave mismatches in the matrix table only;
  - a delayed adoption and a stale binding gave late adoptions and binding mismatches.

## Gate runs

`tools/perf/renderer_gate.py` gave the verdict `met` for every series, each of three attended cold runs:

| Scene | Pooled average | p99 | Max |
|---|---|---|---|
| W-J, S4 | 614.96 fps | 1.926 ms | 3.019 ms |
| W-J, I_a | 613.28 fps | 1.911 ms | 3.128 ms |
| W-J, I_b | 611.92 fps | 1.920 ms | 3.113 ms |
| W3 | 610.28 fps | 1.861 ms | 3.244 ms |

- The gate needs at least 30 fps and a p99 of at most 33.3 ms, per run and pooled.
- The exact pose check passed in every W-J run, for every drawn revision. The label check passed in every run.
- In the 180 s gate interval no present was dropped, and every second had a displayed frame.
- Peak VRAM was about 86.8 MB. The budget is about 7 GB.
- Conditions:
  - RTX 4070 Laptop GPU on mains power, NVIDIA driver 32.0.16.1692;
  - 2560 x 1600 at 60 Hz, swap interval 0 with tearing;
  - no frame generation, upscaling, MSAA or overlays.

## What the session taught

- **A runner defect voided the first attempt.** The runner assigned its confirmation text to `$overlay`. PowerShell names ignore case, so that is the parameter `-Overlay`, whose value set rejects the text, and every series stopped after run 1. The four runs of that attempt count for nothing. The fix renames the variable (review `20261010T200533Z-4f7766fb`, pass).
- **Stray console input can reach the confirmation prompt.** In one I_a series, text that was pasted or typed while the probe was on screen appeared at the prompt. The runner refused that run, as it should, and the series ran again from run 1. All three capture runners now flush the console input buffer before the prompt. That flush has not been tested in an attended run yet.

## What it does not show

- It does not show that W-J is accepted. The overlay costs (acceptance item 7) are unmeasured.
- It does not show that any renderer is selected. Selection needs the day-7 go/no-go ruling ([renderer-candidates](../decisions/renderer-candidates.md)).
- It gives no results for the overlays, other hardware, other resolutions or refresh rates, frame generation, or long sessions.
