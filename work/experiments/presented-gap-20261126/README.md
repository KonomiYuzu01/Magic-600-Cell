# Frames without a confirmed present: cause and fix, 26 November 2026

The owner-machine sitting of 25 November 2026 found that 15 to 18% of the measured runs' frames had no `presented`
row, against 2 to 6% on 21 November
([owner-sitting-2026-11-25-results](../../../docs/wiki/evidence/owner-sitting-2026-11-25-results.md), "Finding: far
fewer frames are confirmed as presented"). These records find the cause and test the fix in `renderer/core`.

## The cause

The shell's frame loop blocks in the core's frame call (`m6h_frame`, then `sa2_frame_submit`) for most of a refresh:
the call waits for the core's frame-latency waitable, which a vblank releases. The shell reads the presented rows
(`sa2_presented`) right after the call, about 0.2 to 0.6 ms after the vblank. That call takes the core's only
statistics sample of the frame. A frame is confirmed only by a sample whose `PresentCount` equals the frame's own
count. Whether DXGI has posted that vblank's statistics by the time of the sample changes from frame to frame. Each
time the answer changes from "not yet" to "posted" between two frames, the sample passes one present count, and the
exact rule drops a frame that was on screen for a whole refresh.

- **What the logs show.** In every run, 84 to 96% of the samples run 0.2 to 0.6 ms after the vblank (`race.py`'s
  `sample_offset_us`). On 21 November's build the statistics were posted by then for 0.3% of the frames, and in the
  2400 × 1400 window for none: the confirmations ran one frame later, and almost no frame was passed. In the default
  window on 25 November's build they were posted by then for 26% of the frames in U8 and 13% in the owner's step 2, so
  the sample kept changing sides (`results/race-sittings.json`).
- **The skipped frames were on screen.** Of the 60,230 frames without a row in U8 with Jumble off (25 November),
  59,641 lie between two confirmed neighbours that are two refreshes apart. With sync interval 1, the skipped frame's
  own refresh lies between them.
- **The build is not the cause.** The code that confirms a present did not change between the two builds. Run back to
  back on this machine today, 21 November's build lost 55% and 85% of its frames and 25 November's lost 43% and 2.1%
  (`results/race-builds-20261126.json`). Today's windows started from a background process behind the owner's active
  windows, which adds stalls: runs of 4 or more frames with no statistics row (`missed_in_steps_5_up`). The sitting's
  unattended U8 run had none of those, and only short skips.

## The fix

The core takes one more statistics sample during a product frame's frame-latency wait, when that wait has not ended
after 5 ms (`sa2::latency_wait` in `renderer/core/src/product.h`; `SurfaceSampleMs` and `product_statistics` in
`renderer/core/src/interop.cpp`). That sample runs well after the last vblank's statistics are posted and well before
the next vblank. A frame shown for a whole refresh therefore meets a sample of its own. The exact rule, the ABI and
the scene (traced) path are unchanged. The CPU self-test check `latency wait` covers the two-part wait. It also
simulates the race: one sample per frame drops every third frame, and the in-wait sample confirms every frame.

Interleaved 60-second runs of 25 November's product build, with its own core (`ctl`) and with the fixed core (`fix`),
on this machine today (`results/race-core-fix-20261126.json`):

| Run | Core | Frames | Without a row | In short skips (1 to 3 frames) | In stalls (4 or more) |
| --- | --- | --- | --- | --- | --- |
| ctl-1 | build | 3,506 | 1,651 | 1,628 | 20 |
| fix-1 | fixed | 3,502 | 4 (the last 4, not yet shown at exit) | 0 | 0 |
| ctl-2 | build | 3,502 | 2,818 | 175 | 2,523 |
| fix-2 | fixed | 3,512 | 1,653 | 89 | 1,560 |

The fix removes the short skips, which were the whole of the sitting's unattended loss. It does not remove stalls, in
which no sample sees a new count for several refreshes. Today's stalls came from a window without the foreground on a
desktop in use. The frame call's duration is unchanged (`frame_call_ms`).

## Files

| File | What it is |
| --- | --- |
| `race.py` | Per run: frames without a row, the neighbours' refresh spacing, short skips against stalls, and where the per-frame sample ran relative to the vblank |
| `ab_core.py` | Interleaved short runs of one product build with its own core and with another core DLL |
| `results/race-sittings.json` | `race.py` over 21 November's U8 (Jumble off), and over 25 November's U8 (Jumble off), the owner's step 2 and the integrator's 2400 × 1400 run |
| `results/race-builds-20261126.json` | `race.py` over today's back-to-back runs of both product builds (`old-1`, `new-1`, `old-2`, `new-2`) |
| `results/race-core-fix-20261126.json` | `race.py` over today's `ctl-1`, `fix-1`, `ctl-2` and `fix-2` |
| `results/presented-share-20261126.json` | `presented_share.py` (owner-sitting-20261125) over today's eight runs |

The production commands, from the repository root:

    python -B work/experiments/presented-gap-20261126/race.py <21 Nov u8-off-final> <25 Nov u8-off-s1125> <25 Nov greybox> <25 Nov greybox-2> --out work/experiments/presented-gap-20261126/results/race-sittings.json
    python -B work/experiments/presented-gap-20261126/race.py work/ab/run-old-1 work/ab/run-new-1 work/ab/run-old-2 work/ab/run-new-2 --out work/experiments/presented-gap-20261126/results/race-builds-20261126.json
    python -B work/experiments/presented-gap-20261126/race.py work/ab/run-ctl-1 work/ab/run-fix-1 work/ab/run-ctl-2 work/ab/run-fix-2 --out work/experiments/presented-gap-20261126/results/race-core-fix-20261126.json
    python -B work/experiments/owner-sitting-20261125/presented_share.py work/ab/run-old-1 work/ab/run-new-1 work/ab/run-old-2 work/ab/run-new-2 work/ab/run-ctl-1 work/ab/run-fix-1 work/ab/run-ctl-2 work/ab/run-fix-2 --out work/experiments/presented-gap-20261126/results/presented-share-20261126.json

Today's runs: each build's own `renderer/shell/launch.py` (21 November's through its worktree's launcher) on copies of
the two product builds, 60 seconds each in the default window, closed with WM_CLOSE. Then `ab_core.py` ran 25
November's copy with its own core and with a core built from this change by `renderer/core/build.cmd`. The raw run
folders stay on the owner's machine.

A record states what the shell's logs show. It does not establish what the screen showed, the machine or the
conditions (PHS.64), and it is never a phase pass on its own (PHS.66).
