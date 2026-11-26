# The twist key's stall on the frame thread, 26 November 2026

Each twist key held the product shell's frame thread inside `m6h_key_down` for about 50 ms on the owner's machine
(25 November U7: p50 50.9 ms, p99 60.9 ms; `docs/wiki/evidence/presented-gap-2026-11-26.md`). These records split
that time into its parts and compare the frame loop before and after the change that moves the view's work off the
frame thread.

Every number here is a headless run on this machine (Windows, release build) of the host library's C interface over
the fake core on route B, with the session's journal on the machine's disk. The fake core has no GPU: the real core's
adoption and preparation cost more, and the shell's own work is not included. Nothing here is Windows/DirectX, input
or performance evidence (PHS.64, PHS.66); LX-2's device run (U7) is the measurement.

## The parts (before the change)

`timers.patch` is the temporary timing code that produced `results/parts-jumble-off.txt` and
`results/parts-jumble-on.txt`; it was never committed to the product sources. It prints one line per part per key
from a bench that opens a session and presses 60 twist keys, one frame between keys. `aggregate.py <log> 5` folds the
lines into the per-key p50, p90 and maximum, skipping the first five keys. Jumble on, the menu resolve and the derive
inside the commit were not timed separately.

| Part (p50 ms) | Jumble off | Jumble on |
| --- | --- | --- |
| `m6h_key_down`, whole | 35.3 | 22.7 |
| The commit (`Turns::key_down`: router, command layer, session store) | 11.0 | 8.5 |
| ... twist menu resolve (`rotation::resolve`) | 4.8 | |
| ... kernel derive | 3.7 | |
| ... journal write and its `sync_all` | 2.2 (sync 1.8) | 2.1 (sync 1.8) |
| ... token, rechecks, head digests (cached) | about 0.3 | |
| The head's adoption build (bundle and primitive) | 9.5 | 2.3 |
| `Host::adopt`: bundle check, then the backend's adoption with a second check | 8.8 (4.3 + 4.4) | 8.7 (2.0 + 6.6) |
| The head digest | 0.0 | 0.1 |
| The publish: publication check, then the preparation | 5.7 (4.7 + 0.8) | 2.7 (2.2 + 0.3) |
| The preview sync | 0.0 | 0.0 |

A twist writes no object, only the journal frame and its sync. The two head digests read cached values. Most of the
view's share is SHA-256 over the 259,800-slot arrays (building them, then checking them twice).

## Before and after: the frame loop

`renderer/bridge/host/tests/key_timing.rs` (an ignored test) drives a frame loop on a modelled 60 Hz display with a
frame latency of 1: a twist key every 15 frames, a frame each iteration, each frame shown at the first refresh after
its submission, the loop waiting for that refresh as the core's frame-latency wait does, and the fake core completing
its work after each frame. `results/bench-before.txt` ran it on main at `06e3c26d`, `results/bench-after.txt` on the
change, 60 keys each:

```text
cargo test --release --manifest-path renderer/bridge/Cargo.toml -p c600-shell-host --test key_timing -- --ignored --nocapture
PUZZLE=1 cargo test ... (Jumble on)
```

| | Jumble off, before | Jumble off, after | Jumble on, before | Jumble on, after |
| --- | --- | --- | --- | --- |
| `m6h_key_down` p50 / p99 ms | 39.0 / 48.8 | 11.4 / 18.9 | 24.8 / 32.6 | 8.9 / 15.7 |
| `m6h_frame` p99 / max ms | 0.5 / 0.6 | 1.3 / 1.5 | 0.2 / 0.4 | 5.7 / 6.1 |
| Refreshes that showed no new frame | 121 | 7 | 61 | 1 |
| Key-down to the refresh that shows the press, p50 / p90 ms | 66.4 / 66.6 | 66.4 / 66.6 | 49.8 / 49.9 | 49.8 / 66.2 |

The press reaches the screen at the same refresh at the median. With Jumble on, the 90th percentile is one refresh
later than before: some presses are shown a frame later, when the worker's build ends just after a frame boundary. The remaining missed refreshes
come from key calls over one refresh: the commit itself, which this change leaves as it is.
