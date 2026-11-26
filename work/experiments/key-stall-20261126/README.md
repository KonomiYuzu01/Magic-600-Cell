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

## On the real core: the product shell on screen

`ab_keys.py` runs one product build (`c9-product-795b9d7ad272d5e2`, made from `06e3c26d`, the commit before the
change) on screen with its own host library (`old`) and with the change's host library (`new`), interleaved, Jumble
off. It finds the shell's window, asks once for the foreground, and posts one turn key every 0.5 s, cycling J, K and L
as tools/perf/c9/lx4.py posts a turn. `ab_keys_report.py` reads the shell's own `events.jsonl`. The raw run folders
stay on the owner's machine; the records here hold run IDs and counts.

`results/real-core-old-new.json`: 90 s each, 180 keys each, every key posted while the shell had the foreground, the
owner idle throughout.

| | old-1 | old-2 | new-1 | new-2 |
| --- | --- | --- | --- | --- |
| Key call (`handled_qpc - qpc`) p50 ms | 57.5 | 57.2 | 18.5 | 17.6 |
| Frame intervals above 1.5 refreshes, per key | 1.0 | 1.0 | 1.9 | 2.0 |
| Frame interval p99 / max ms | 75.3 / 81.7 | 74.9 / 78.8 | 36.1 / 47.4 | 34.9 / 45.6 |
| Refreshes without a submitted frame, per key | 3.37 | 3.33 | 1.93 | 2.01 |
| Key-down to the confirmed present showing it, p50 / p90 / p99 ms | 82.3 / 98.9 / 115.8 | 82.4 / 84.1 / 115.7 | 99.1 / 99.4 / 115.6 | 99.1 / 99.4 / 99.5 |
| Frames from the key-down to the frame showing it, p50 | 1 | 1 | 3 | 3 |

The stall per key fell from one interval of about 75 ms to two of about 35 ms. But the press was shown one refresh
later than before at the median: the key call returns before the worker's build ends (10.7 ms), the next frame call
returns at once because the key call already passed a refresh, and the build was adopted only after the frame after
that. The change's last part (the frame call waits at most 12 ms for a pending build after its submission) adopts it
one frame earlier.

`results/real-core-wait.json`: 60 s each, 120 keys each, the change without that wait (`new-1`, `new-2`) and with it
(`new-w1`, `new-w2`), interleaved. The owner returned to the machine during `new-w1`: only `new-1` had the foreground,
and the other three runs confirmed few presents, so their shown times are partial.

| | new-1 | new-w1 | new-2 | new-w2 |
| --- | --- | --- | --- | --- |
| Frames from the key-down to the frame showing it, p50 / p90 | 3 / 3 | 2 / 3 | 3 / 3 | 2 / 2 |
| Key-down to the confirmed present, p50 ms (confirmed keys) | 99.1 (120) | none (0) | 99.1 (15) | 82.3 (25) |
| Refreshes without a submitted frame, per key | 2.01 | 2.00 | 2.00 | 1.97 |

The frame that shows the press comes from frame IDs, which do not depend on the foreground. With the wait it is the
second frame after the key, and the one run with confirmed presents shows it at 82.3 ms, as before the change.

### Where the frame thread's time per key goes now

Temporary timers (`timers-ab.patch`, `timers-prepare.patch`, `timers-host.patch`; never committed to the sources) in
the change's host, on screen as above, 30 to 45 s each (`results/real-core-timers-*.txt`, one line per build,
install, publish and preparation), p50 ms:

| Part | ms |
| --- | --- |
| The key call: the commit (unchanged) | 17.6 to 18.5 (from the runs above) |
| The worker's build and checks (off the frame thread) | 10.7 |
| The install on the frame thread: the core's adoption | 0.2 |
| The install on the frame thread: the publish | 14.3 to 15.3 |
| ... the request digest | 0.06 |
| ... admission (`describe_backend`, a core call) | 1.4 |
| ... the backend's slot rows | 0.2 |
| ... the core's `sa2_prepare` | 10.6 to 11.3 |
| ... the host's pump after the publish | about 2 |

`sa2_prepare` creates a new command allocator, a command list, two 1 MB default-heap buffers and a 2 MB upload buffer
for every preparation (`Scene::product_prepare` in `renderer/core/src/scene.cpp`), so every twist creates GPU
resources on the frame thread. That and the commit are the two parts left, each over a refresh on this machine.

## Phase 1b: the core's preparation and the menu's resolve

The owner chose to remove the rest before U7 (26 November 2026). Both parts were timed headless on this machine.

### The core's `sa2_prepare`

`sa2_selftest.exe --hardware --measure-prepare 200 --out FILE` (`run_measure_prepare` in `renderer/core/src/gpu_test.cpp`)
prepares 200 full sets on one view on the hardware adapter, without a window, each activated and its predecessor
retired as the shell does per twist, and writes the duration of each `sa2_prepare` call. `results/prepare-before.jsonl`
ran the core of this branch's base with only that option added; `results/prepare-after.jsonl` the change. Rounds 11 to
200, ms:

| | p50 | p90 | p99 | max |
| --- | --- | --- | --- | --- |
| before | 10.10 | 11.18 | 14.98 | 22.85 |
| after | 1.21 | 1.49 | 1.72 | 1.97 |

Temporary timers in the core (`timers-core.py` applies them; never committed; `results/prepare-split.txt`, with the
resources already reused) split the call: the conversion of the caller's arrays (`sa2::preparation`) 9.0 ms, the
resources 0.01 ms, the copy into the upload buffer 0.12 ms, the recording and submission 0.19 ms, the checks and the
release under 0.01 ms. The conversion checks each of the 259,800 slots with `argument`, which built a `std::string` of
its message on every call; it now builds it only for a failure. Reusing the resources, the change the integrator
asked for, saves about 0.7 ms of creating them (10.1 to 9.4 ms with the reuse alone).

### The menu's resolve

`renderer/bridge/host/tests/key_timing.rs` (fake core, modelled 60 Hz loop, 60 keys) on the memo's commit and on its
parent (`twist.rs` swapped, the rest of the branch the same), interleaved: without, with, without, with, each with
Jumble off and on (`results/bench-1b-without-memo-p*.txt`, `results/bench-1b-with-memo-p*.txt`, both rounds in each).
The owner was using the machine, so every run is slower than the same bench earlier in the day; the difference is what
counts:

| `m6h_key_down`, ms, round 1 / round 2 | p50 | p90 | p99 |
| --- | --- | --- | --- |
| Jumble off, without the memo | 12.76 / 12.79 | 15.22 / 16.27 | 19.07 / 20.07 |
| Jumble off, with it | 7.30 / 7.67 | 8.41 / 12.56 | 14.87 / 14.97 |
| Jumble on, without the memo | 10.18 / 10.01 | 10.91 / 16.02 | 11.70 / 17.20 |
| Jumble on, with it | 4.91 / 5.15 | 5.65 / 9.06 | 12.38 / 12.39 |

The memo takes about 5 ms off each key's call at p50 in both puzzles.

The bench's frame calls have no wait for the display (the fake core's frame returns at once), so its post-submit wait
for a pending build (up to 12 ms) shows there as missed refreshes, which it is not on the real core, where that wait
follows the display's; its refresh counts are not compared here.
