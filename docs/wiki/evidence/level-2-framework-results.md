---
id: level-2-framework-results
type: evidence
status: verified
visibility: public
summary: Level 2 framework scenes on the owner's RTX 4070 Laptop GPU, 4 October 2026. Godot 4.7.2 (S-A2) and Qt 6.10.3 (S-D) both meet the W3 selection gate; Qt's 60 fps is a vblank cap in Qt, not GPU headroom; two finalizer defects were found and fixed; loaded-file binding of the build identities is still open (L2-V-002).
related: [renderer-candidates, s-b-probe-results, interop-smoke-results]
supersedes: []
claims:
  - {id: qt-w3-gate-met, evidence_kind: performance, path: work/experiments/renderer-l2/results/sd-w3-20261004T135157249Z-summary.json, sha256: 3eabbce21a5afc2c6fca3c18a5863adfe5996ee74e159b1b27eb61991406505b, checked_at: 2026-10-09}
  - {id: godot-w3-gate-met, evidence_kind: performance, path: work/experiments/renderer-l2/results/sa2-w3-20261004T150202105Z-summary.json, sha256: 6f1a7576cbe83da9e9f80e2d232a0b6d18fe42ffbaec1f0788f1a3477843ef34, checked_at: 2026-10-09}
  - {id: level-2-result-card, evidence_kind: actual_windows_directx, path: work/experiments/renderer-l2/RESULT.md, sha256: 3c52ded6f4665e0aeabce21a8756a63240d1e42fe1c964495e17f9b30b6d6299, checked_at: 2026-10-09}
  - {id: qt-vblank-cap, evidence_kind: source, path: work/experiments/renderer-l2-packets/FRAMEWORK-FACTS.md, sha256: a16abed2b6ad187adbf2e2942c94a281f853fb8ba7fdd0954f009c3aa27825cc, checked_at: 2026-10-09}
  - {id: finalizer-fixes, evidence_kind: fixture, path: work/experiments/renderer-l2/finalize_run.py, sha256: acd23d5707f0ad5eeebfa4a074a6ee8b8c6e9fe1299925cd4824ed0d60b72747, checked_at: 2026-10-09}
---

# Level 2 framework results

These are the level 2 results for the two framework candidates. Godot 4.7.2 .NET is S-A2 and Qt 6.10.3 is S-D. The owner ran attended steps 4 to 7 of the level 2 harness on 4 October 2026. Details are on the [result card](../../../work/experiments/renderer-l2/RESULT.md).

The results hold only on the owner's machine, under the recorded conditions, for these builds:
- Qt identity `64f08369...`, prepared from `a52dd1c`;
- Godot identity `709a24eb...`, prepared from `2e1868e`.

## Qualification: the build identities are not yet bound to the loaded files (L2-V-002)

An Astra escalation ruling (9 October) found that the finalizer hashes the recorded framework files and shaders after the run. It does not bind those hashes to the bytes the app loaded.
- Each build was prepared offline at a committed head with `matches_head` true, and the owner reports no rebuild during the runs.
- Complete loaded-file binding was not enforced, so these identity assignments rest on the preparation records and on the owner's account.
- The fix is planned, not built: a guard process holds every identity file before the app starts and keeps it until the finalizer is done.

## W3 gate scene

`tools/perf/renderer_gate.py` gave the verdict `met` for both candidates, over three attended cold runs each:

| | Pooled average | p99 | Max |
|---|---|---|---|
| Qt (S-D) | 60.00 fps | 17.380 ms | 18.005 ms |
| Godot (S-A2) | 670.71 fps | 1.887 ms | 4.029 ms |

- The gate needs at least 30 fps and a p99 of at most 33.3 ms, per run and pooled.
- W1, W2 and W4 ran three times each as attribution scenes (not gate scenes). Their figures are on the result card.
- Peak VRAM was 229.6 MB for Qt and 217.2 MB for Godot. The budget is about 7 GB.
- Conditions:
  - RTX 4070 Laptop GPU on mains power, NVIDIA driver 32.0.16.1692;
  - 2560 x 1600 at 60 Hz, exclusive full screen, swap interval 0;
  - no frame generation, upscaling, MSAA or overlays.

## What the runs taught

- **Qt is capped at the display refresh.** Qt 6.10.3 delivers each `requestUpdate()` only after the display's vertical blank, whatever the swap interval. This is source evidence, fact Q10 in the framework facts. The Qt figures therefore show the cap, not how fast Qt could render, and the Qt and Godot frame rates are not a like-for-like comparison. An uncapped Qt run needs two `QT_*` variables and gives a new build identity. It has not been run.
- **Two finalizer defects, now fixed:**
  - The `blind-seconds` check rounded the short interval up to a whole second. That made Godot's step 5 refuse a 0.5 ms sliver shorter than one frame. The check now counts whole seconds only (review `20261009T155227Z-a94ea84f`).
  - `tearing` was false in five Godot runs, because each had one dropped present that PresentMon reported without `AllowsTearing`. The rule now uses displayed presents only (review `20261009T160651Z-220450f6`, verified by `20261009T161532Z-7d576044`).
  - `tearing` is metadata and no verdict uses it.
  - The published summaries keep the values recorded on 4 October.

## What it does not show

- It does not show that either candidate is selected. Selection needs the day-7 go/no-go ruling ([renderer-candidates](../decisions/renderer-candidates.md)).
- It gives no uncapped Qt figure and no GPU-headroom comparison between Qt and Godot.
- It does not show loaded-file binding for these identities.
- It gives no results for other hardware, other resolutions or refresh rates, frame generation, or long sessions.
