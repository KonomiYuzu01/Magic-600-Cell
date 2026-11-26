---
id: presented-gap-2026-11-26
type: evidence
status: verified
visibility: public
summary: Why far fewer frames were confirmed as presented at the owner-machine sitting of 25 November 2026, and the fix. The shell's one statistics sample per frame runs 0.2 to 0.6 ms after the vblank, and whether DXGI has posted that vblank's statistics by then changes from frame to frame; each change from "not yet" to "posted" passes a present count, and the exact rule drops a frame that was on screen. The product build is not the cause; the window and the desktop decide how often the race changes sides. The core now takes one more sample inside a product frame's frame-latency wait, 5 ms in. In interleaved runs that removed the short skips, which were the whole of the sitting's unattended loss; stalls on a desktop in use remain. Separately, the stutter in LX-2's four runtime-losing faults comes from each turn key, which blocks the frame thread for about 50 ms in the synchronous commit, not from the runtime's loss.
related: [owner-sitting-2026-11-25-results, c9-sitting-results, owner-decisions-2026-11-22-lx-bars-freeze]
supersedes: []
claims:
  - {id: pg1126-race-sittings, evidence_kind: actual_windows_directx, path: work/experiments/presented-gap-20261126/results/race-sittings.json, sha256: 531f4de81fc132f6777f778452dee02ab70d132a9d09f000164c1064c4e2e7f0, checked_at: 2026-11-26}
  - {id: pg1126-race-builds, evidence_kind: actual_windows_directx, path: work/experiments/presented-gap-20261126/results/race-builds-20261126.json, sha256: 59cd9986ae23d4503bd5216f5a549a072ab623e3f8f9655f600cc29074efdc0a, checked_at: 2026-11-26}
  - {id: pg1126-race-core-fix, evidence_kind: actual_windows_directx, path: work/experiments/presented-gap-20261126/results/race-core-fix-20261126.json, sha256: 2f1c8b1e499c1c86bd55cee40cd743508ddcc9f0e2b37bc241d4479de24f1916, checked_at: 2026-11-26}
  - {id: pg1126-share-today, evidence_kind: actual_windows_directx, path: work/experiments/presented-gap-20261126/results/presented-share-20261126.json, sha256: 1c0d93195d59682923d506c9b7c62d2db42480a94576a46cff81ee1758cb6f22, checked_at: 2026-11-26}
  - {id: pg1126-core-sample, evidence_kind: source, path: renderer/core/src/interop.cpp, line: 108, sha256: 523dcbbefe895520cfda6b6a1df690a795bcaf55915170537717012484ce7d65, checked_at: 2026-11-26}
  - {id: pg1126-latency-wait, evidence_kind: source, path: renderer/core/src/product.h, line: 250, sha256: e42ca0f26d5706dcebc10ac558c85e8f16edcae341cb87de9305f3c585f517c9, checked_at: 2026-11-26}
  - {id: pg1126-latency-wait-check, evidence_kind: fixture, path: renderer/core/src/selftest.cpp, line: 1398, sha256: 07ba424693726553d5dbbf2a42dd4041632fcdcd665efcc14f04cd834b9828af, checked_at: 2026-11-26}
  - {id: pg1126-keys, evidence_kind: actual_windows_directx, path: work/experiments/presented-gap-20261126/results/keys-sittings.json, sha256: ec047018b3a3c2ab610fca11293ec2ebc84a051b0646e64394f32e1f6392db50, checked_at: 2026-11-26}
  - {id: pg1126-gate-not-required, evidence_kind: performance, path: work/experiments/owner-sitting-20261125/results/u7-s1125-lx2.json, line: 81, sha256: 854f4c729a9d22e2c0fb394ea3a71fdc8adb7d2861c6f4499b44e0e61f689346, checked_at: 2026-11-26}
---

# Frames without a confirmed present, 26 November 2026

The [owner-machine sitting of 25 November](owner-sitting-2026-11-25-results.md) left two problems open: far fewer
frames confirmed as presented than on 21 November, and the stutter in LX-2's faults that lose the runtime. Both are
traced here from the shell's own logs. The records and the method are in `work/experiments/presented-gap-20261126/`.

## The confirmation gap: a sampling race

The shell's frame loop blocks in the core's frame call for most of a refresh, waiting for the core's frame-latency
waitable, which a vblank releases. The shell reads the presented rows right after the call. That read takes the core's
only statistics sample of the frame, and a frame is confirmed only by a sample whose `PresentCount` equals its own
count. The sample runs 0.2 to 0.6 ms after the vblank (84 to 96% of samples in every run). DXGI posts that vblank's
statistics sometimes before the sample and sometimes after. When the answer changes from "not yet" to "posted"
between two frames, the sample passes one count, and the exact rule drops a frame that was shown
(`pg1126-race-sittings`):

| Run | Statistics already posted at the sample | Frames without a row | Single frames between neighbours two refreshes apart |
| --- | --- | --- | --- |
| U8 Jumble off, 21 November | 0.3% | 8,525 of 363,302 | 7,884 |
| U8 Jumble off, 25 November | 26% | 60,230 of 363,177 | 59,641 |
| The owner's step 2, 25 November | 13% | 2,908 of 7,253 | 487 |
| The 2400 × 1400 run, 25 November | 0% | 23 of 39,706 | 6 |

With sync interval 1, a single missing frame between neighbours shown two refreshes apart was on screen for the
refresh between them.

## Not the build: the window and the desktop

The code that confirms a present did not change between the two product builds. Run back to back on the owner's
machine on 26 November, in the default window, 21 November's build lost 55% and 85% of its frames and 25 November's
lost 43% and 2.1% (`pg1126-race-builds`, `pg1126-share-today`). Those windows started from a background process
behind the owner's active windows, which also caused stalls: four or more frames in a row with no statistics row,
which the sitting's unattended runs did not have.

## The fix in the core

A product frame's frame-latency wait now takes one more statistics sample for the same ledger when it has not ended
after 5 ms (`pg1126-core-sample`, `pg1126-latency-wait`). The last vblank's statistics are posted by then, and the
next vblank is still ahead. The exact rule, the ABI and the scene path are unchanged. The CPU check `latency wait`
covers the two-part wait. It also simulates the race: one sample per frame drops every third frame, and the in-wait
sample confirms every frame (`pg1126-latency-wait-check`). On screen, interleaved 60-second runs of 25 November's
build with its own core and with the fixed core (`pg1126-race-core-fix`) gave:

| Run | Frames without a row | In short skips (1 to 3 frames) | In stalls (4 or more) |
| --- | --- | --- | --- |
| Own core, run 1 | 1,651 of 3,506 | 1,628 | 20 |
| Fixed core, run 1 | 4 of 3,502 (the last frames, not yet shown at exit) | 0 | 0 |
| Own core, run 2 | 2,818 of 3,502 | 175 | 2,523 |
| Fixed core, run 2 | 1,653 of 3,512 | 89 | 1,560 |

The fix removes the short skips, which were the whole of the sitting's unattended loss. It cannot confirm frames
during a stall, where no sample sees a new count for several refreshes. The frame call's duration did not change.

## The stutter in the runtime-losing faults

LX-2 measures the renderer gate over limit windows for the three metered faults and over turn windows for the four
faults that lose the runtime. In those four the gate is not required (`pg1126-gate-not-required`). Each turn key
blocks the frame thread inside `m6h_key_down` for about 50 ms (p50 50.9 ms, p99 60.9 ms in U7 on 25 November). U7
has exactly one frame interval above 40 ms per key: 1,661 keys, 1,661 intervals. 21 November's runs show the same
interval around each key, 65.8 ms at the median (`pg1126-keys`). So the stutter follows the turns, not the loss of
the runtime. The key call does the commit with its durable writes (the session store syncs its journal and its new
objects to disk), two digests of the head, the adoption of the full engine bundle and a publish, all on the frame
thread. Moving that work off the frame thread changes the commit boundary, so it is not a small fix and is left open.

## What it does not show

These records show what the shell's logs say. They do not show what the screen showed (PHS.64), and none is a phase
pass on its own (PHS.66). The fixed core has run only in two 60-second runs whose windows had no foreground. The
night's unattended re-test on a product build that contains this change is the measurement.
