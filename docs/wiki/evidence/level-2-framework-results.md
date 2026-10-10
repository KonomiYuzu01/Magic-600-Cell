---
id: level-2-framework-results
type: evidence
status: verified
visibility: public
summary: Level 2 framework scenes on the owner's RTX 4070 Laptop GPU. Godot 4.7.2 (S-A2) and Qt 6.10.3 (S-D) both meet the W3 selection gate on 4 October and again on 10 October with the area-centroid anchor, the second time under the loaded-file guard; Qt's 60 fps is a vblank cap in Qt, not GPU headroom; two finalizer defects were found and fixed; the 4 October identities keep the L2-V-002 qualification.
related: [renderer-candidates, s-b-probe-results, interop-smoke-results]
supersedes: []
claims:
  - {id: qt-w3-gate-met, evidence_kind: performance, path: work/experiments/renderer-l2/results/sd-w3-20261004T135157249Z-summary.json, sha256: 3eabbce21a5afc2c6fca3c18a5863adfe5996ee74e159b1b27eb61991406505b, checked_at: 2026-10-09}
  - {id: godot-w3-gate-met, evidence_kind: performance, path: work/experiments/renderer-l2/results/sa2-w3-20261004T150202105Z-summary.json, sha256: 6f1a7576cbe83da9e9f80e2d232a0b6d18fe42ffbaec1f0788f1a3477843ef34, checked_at: 2026-10-09}
  - {id: qt-w3-reaccepted, evidence_kind: performance, path: work/experiments/renderer-l2/results/sd-w3-20261010T172910231Z-summary.json, sha256: e990333279f6fe70ecc122059d247036945933b5f3f8ce7b98c6c921909ce52e, checked_at: 2026-10-10}
  - {id: godot-w3-reaccepted, evidence_kind: performance, path: work/experiments/renderer-l2/results/sa2-w3-20261010T175043422Z-summary.json, sha256: 339f629cc1197f5cd35a17337a3166f69f6c5769523e073f8af718bec07d43ed, checked_at: 2026-10-10}
  - {id: level-2-result-card, evidence_kind: actual_windows_directx, path: work/experiments/renderer-l2/RESULT.md, sha256: 530df439e46a53a81455703e607a34149553e0cbbe9a8d11760cabdfa1cdfe2d, checked_at: 2026-10-10}
  - {id: qt-vblank-cap, evidence_kind: source, path: work/experiments/renderer-l2-packets/FRAMEWORK-FACTS.md, sha256: a16abed2b6ad187adbf2e2942c94a281f853fb8ba7fdd0954f009c3aa27825cc, checked_at: 2026-10-09}
  - {id: finalizer-fixes, evidence_kind: fixture, path: work/experiments/renderer-l2/finalize_run.py, sha256: 6dd01b63b586cd94a80107b310dd3190dfe29a6af2340c95a5b6eee8fb34e5f8, checked_at: 2026-10-10}
---

# Level 2 framework results

These are the level 2 results for the two framework candidates. Godot 4.7.2 .NET is S-A2 and Qt 6.10.3 is S-D. The owner ran attended steps 4 to 7 of the level 2 harness on 4 October 2026. Details are on the [result card](../../../work/experiments/renderer-l2/RESULT.md).

The results hold only on the owner's machine, under the recorded conditions, for these builds:
- Qt identity `64f08369...`, prepared from `a52dd1c`;
- Godot identity `709a24eb...`, prepared from `2e1868e`;
- for the re-acceptance of 10 October, Qt `befc441f...` and Godot `06d14019...`, both prepared from `c4b31b0`.

## Anchor change (10 October)

The owner changed the sticker shrink anchor to the area-weighted centroid of each sticker's triangles ([decision](../decisions/owner-decisions-2026-10-10-anchor.md)).
- The S-A2 native library compiles the S-B probe source, and the Qt app loads the same library, so both candidates get new build identities.
- The 4 October results describe the old anchor (the retained numbering centres). They hold for the two identities above and say nothing about the changed builds.
- Re-acceptance was three attended cold W3 runs per candidate under the guard, with builds prepared again from `c4b31b0`. The owner ran it on 10 October, and `renderer_gate.py` gave `met` for both:

| | Build identity | Pooled average | p99 | Max |
|---|---|---|---|---|
| Qt (S-D) | `befc441f...` | 60.00 fps | 17.440 ms | 18.310 ms |
| Godot (S-A2) | `06d14019...` | 670.69 fps | 1.907 ms | 12.328 ms |

- Every step ran under the guard: validation, geometry, the short check, the two deliberate refusals and the W3 runs.
- The label check passed in every W3 run, with 1,010 revisions each. Peak VRAM was unchanged: 229.6 MB for Qt and 217.2 MB for Godot.
- Godot's refusals now give exactly the required reasons, which confirms the `blind-seconds` fix on the GPU.
- Godot's 12.328 ms maximum was one frame in run 1. Its other two runs peaked below 3.9 ms.

## Qualification: the build identities are not yet bound to the loaded files (L2-V-002)

An Astra escalation ruling (9 October) found that the finalizer hashes the recorded framework files and shaders after the run. It does not bind those hashes to the bytes the app loaded.
- Each build was prepared offline at a committed head with `matches_head` true, and the owner reports no rebuild during the runs.
- Complete loaded-file binding was not enforced, so these identity assignments rest on the preparation records and on the owner's account.
- The fix was built on 10 October: a guard process holds every identity file before the app starts and keeps it until the finalizer is done. Its two-shard Astra review found four blocking findings, which were fixed the same day; the fixed build passed its source checks and synthetic experiments on the owner's machine (no window, GPU or PresentMon), and the scoped verification round passed on both shards with no finding. The 10 October re-acceptance runs used it, so the qualification does not apply to them, within the guard's stated limits (`HARNESS.md` section 6). The 4 October records keep it.

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
- It does not show loaded-file binding for the 4 October identities.
- It gives no results for other hardware, other resolutions or refresh rates, frame generation, or long sessions.
