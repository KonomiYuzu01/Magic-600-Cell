---
id: owner-decisions-2026-11-27-gate-reading
type: decision
status: verified
visibility: public
summary: Owner reading of 27 November 2026 - the renderer gate's 99th-percentile bar of 33.3 ms is 1/30 s, two refreshes of a 60 Hz display, so a p99 of two refreshes passes.
related: [renderer-candidates, owner-decisions-2026-11-26-adr-d02-build]
supersedes: []
claims:
  - {id: p99-bar-is-two-refreshes, evidence_kind: decision, checked_at: 2026-11-27}
---

# The gate's p99 bar is two refreshes

Owner answer of 27 November 2026, in the ADR-D02 build session.

## The question

The renderer selection gate ([renderer-candidates](renderer-candidates.md)) asks for a 99th-percentile frame time of at most
33.3 ms. On the owner's 60 Hz display a frame interval is a whole number of refreshes: one is 16.67 ms and two are 33.33 ms. The
display's own timestamps put two refreshes at 33.334 to 33.335 ms. After the twist-stall fixes (steps SF1 to SF6 under B19 in
[the build framework](../../progress/1.0/adr-d02-build-framework.md)), ADR-D02's gate dry runs G1 and G9 read p99 33.33 ms and
failed the literal bar by a few hundredths of a millisecond.

## The answer

The owner: it counts as a pass. The 33.3 ms bar is 1/30 s: a p99 of two refreshes of a 60 Hz display passes. A p99 of
three refreshes (50 ms) still fails.

## How it is applied

- `tools/perf/d02/gate_row.py` passes a p99 of at most 33.4 ms: 1/30 s plus room for the display timestamps' spread, below a third
  60 Hz refresh and below a fifth 120 Hz refresh (41.67 ms). Its record states the reading.
- The average frame rate (at least 30 fps), the VRAM ceiling and every other gate condition are unchanged.
