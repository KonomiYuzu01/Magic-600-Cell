---
id: owner-decisions-2026-11-18-route-b
type: decision
status: verified
visibility: public
summary: Owner decision of 18 November 2026 - route B for the confirmed present is accepted for now. The renderer core presents the gated workload in its own composition swap chain on its own Direct3D 12 device, in a DirectComposition layer beneath Godot's per-pixel transparent window, so the core can confirm per frame what was presented (RM.26, ONE.41). C9 may build on it now; it becomes final when three checks on the owner's machine pass (input through the transparent window, the skew between the UI and the 3D view, three cold runs of the probe), and a failing check returns it to the owner. No frozen text is edited until then.
related: [owner-decisions-2026-11-18, renderer-freeze, owner-decisions-2026-11-16-renderer-close, owner-decisions-2026-11-15-freeze]
supersedes: []
claims:
  - {id: owner-18nov-route-b-for-now, evidence_kind: decision, checked_at: 2026-11-18}
  - {id: adr-r01-2-godot-shows-texture, evidence_kind: source, path: docs/architecture/1.0/renderer/ADR-R01-renderer-selection.md, line: 39, checked_at: 2026-11-18}
  - {id: freeze-no-confirmed-present-route, evidence_kind: source, path: docs/wiki/decisions/renderer-freeze.md, line: 71, checked_at: 2026-11-18}
  - {id: one-41-on-screen-means-presented, evidence_kind: source, path: docs/architecture/1.0/language/architecture/08-one-state.md, line: 247, checked_at: 2026-11-18}
  - {id: rm-26-presented-frame, evidence_kind: source, path: docs/architecture/1.0/language/renderer-feature-matrix.md, line: 274, checked_at: 2026-11-18}
  - {id: rpi-46-device-loss, evidence_kind: source, path: docs/architecture/1.0/language/architecture/10-renderer-platform.md, line: 500, checked_at: 2026-11-18}
---

# Owner decision, 18 November 2026: route B for the confirmed present, accepted for now

## The open item

ONE.41 binds the solver's token to the preview of the newest frame the renderer reports as presented
([08-one-state.md](../../architecture/1.0/language/architecture/08-one-state.md), line 247), and RM.26 drafts that
report ([renderer-feature-matrix.md](../../architecture/1.0/language/renderer-feature-matrix.md), line 274). The
renderer freeze left this open: Godot 4.7.2's public API gives no swap chain, so the frozen structure, in which Godot
shows the core's texture ([ADR-R01.2](../../architecture/1.0/renderer/ADR-R01-renderer-selection.md), line 39), had no
confirmed-present route ([renderer freeze](renderer-freeze.md), line 71). Godot's Direct3D 12 driver also has no
device-loss recovery, so a core that draws on Godot's device can only report a loss (RPI.46,
[10-renderer-platform.md](../../architecture/1.0/language/architecture/10-renderer-platform.md), line 500). On
18 November the owner chose to try Godot's public API before any Godot source build
([record](owner-decisions-2026-11-18.md), section 4).

The C9 line then ran a probe of route B on the owner's machine while the owner was away: one run each under a Win32
control host and under Godot 4.7.2 .NET. It is probe evidence only, not gate evidence, and its record is private.

## The options

- **B (recommended).** The core presents the gated workload in its own composition swap chain, on its own Direct3D 12
  device, in a DirectComposition layer beneath Godot's window. Godot hosts the shell (window, UI and input) in a
  per-pixel transparent window over that layer. Godot's own renderer draws no part of the gated workload. The core
  reads its swap chain's frame statistics, so it can confirm a presented frame per frame. A loss of its own device
  stays inside the core.
- **A.** The program captures its own window and decodes a frame code. Windows draws a capture border around the
  window unless the program is packaged with a capability and the user grants a prompt.
- **C.** A Godot source build that reads Godot's own frame statistics. This is an owner item outside the allowlist.
- **No confirmed present.** RM.26's host clears its latch, and ONE.41's binding to a presented preview cannot be met.

## The answer

The owner answered "use B for now" in Chinese on 18 November 2026.

## What it means

**Accepted for now.** C9 may build route B now: the core's own device and composition swap chain, the DirectComposition
layer under Godot's transparent window, and the positive paths of the core's presented-frame and recovery calls. Code
and documentation for B say that it is accepted for now, pending the three checks below.

**Costs accepted with it.**

- The 3D frame and the UI frame are no longer one atomic frame. The token binds the core's confirmed 3D frame, not
  the UI around it. A rule that needs the UI and the 3D view on one frame needs its own design.
- Godot's per-pixel transparency is on the product path.
- Under load, presentation takes about one refresh longer. The gate's frame-time bars are unaffected, but the
  key-to-frame time (charter E2) includes it.
- Not every frame gets its own confirmation. A frame without one is treated as not confirmed, which is the safe side.
- There is a second Direct3D 12 device, with its memory. A real driver reset (TDR) still resets every device, and
  Godot's own recovery stays absent.

**Three checks make it final.** They run on the owner's machine, batched with LP3's other owner-machine runs:

1. input through Godot's transparent window over the core's layer, which needs the foreground;
2. a measurement of the skew between the UI frame and the 3D frame;
3. three cold runs of the probe.

If all three pass, the integrator writes the revision of ADR-R01.2 under PHS.52, and route B is final under this
decision without a new question. If one fails, route B returns to the owner with the result.

**Evidence.** The selection evidence of build `06d14019` was presented through Godot's texture path, so it is not
reused as evidence for route B. ADR-R01.3's W-J and W3 runs and RPI.79's gate run on the product build cover route B,
with unchanged bars.

## What this does not change

The decision keeps the drawing core, its shaders and method, all 259,800 sticker slots, the pose path, Godot as host,
the C# UI and every gate bar. No Godot source build is made and no dependency is added: DirectComposition and DXGI are
Windows components behind the platform layer (RPI.52). No frozen text is edited before the three checks pass, and no
ADR is accepted by this page.
