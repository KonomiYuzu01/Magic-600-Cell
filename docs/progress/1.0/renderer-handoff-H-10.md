# H-10: the selected renderer

Status: **handed to the design track** by the owner's signature of 16 November 2026, which accepted
[ADR-R01](../../architecture/1.0/renderer/ADR-R01-renderer-selection.md) and recorded the
[renderer freeze](../../wiki/decisions/renderer-freeze.md) with W-J option A. It is handoff row H-10 of the
[renderer experiment plan](renderer-experiment-plan.md) (section 6) and of the
[protocol](stage-2-experiment-protocol.md) (section 4.6): the selected renderer, for "the vertical slice and the
framework-neutral rebuild if not Godot".

## The renderer

S-A2: the S-B Direct3D 12 drawing core, built as a shared native library, with Godot 4.7.2 .NET as its host
(ADR-R01.1, ADR-R01.2). The core draws all 259,800 sticker slots. Godot shows the core's texture and hosts the window,
the UI and the input. Because the host is Godot, no framework-neutral rebuild is needed: the Look Lab grows into the
product shell (protocol, section 4.2).

## What the design track can rely on

- The gate passes on the opaque baseline: pooled 670.69 fps, p99 1.907 ms and peak VRAM 217.2 MB on build
  `06d14019…` ([selection record](renderer-selection.md), G1). No look and no H-06 feature were in those runs.
- The S-A2 renderer constraints of H-09 ([S-A2 card](../../../work/experiments/renderer-sa2/RESULT.md), H-09 table):
  - The 3D view reaches Godot as one flat texture. Godot never sees its geometry, depth buffer or shaders, so Godot's
    UI composites over the view, not into it.
  - Text tied to a sticker needs that sticker's screen position every frame, and no ABI 2 function returns one yet.
  - Godot 4.7.2 is system DPI aware only (FRAMEWORK-FACTS G10).
  - A resize with a scene loaded needs a drain, a scene reload and a ring rebuild.
  - In the editor build, an imported texture reaches Godot only through a copy (R6).

## What changes in the Look Lab

- Its 3D view, drawn today by Godot's own renderer from Godot shading language, moves to the core. The look's features
  move into the core's HLSL, each with its H-06 cost measured. The Look Lab's present costs are not measured
  ([Look Lab README](../../../tools/looklab/README.md)), so none of them carries over as gate evidence.
- Its .NET core moves from .NET 8, out of support since 10 November 2026, to a supported .NET release.
- Its use stays outside the renderer evidence (protocol, section 4.2).

## Open items the design track meets

| Item | Taken up by | Where recorded |
|---|---|---|
| E1: the gate in the shipped look | the vertical slice, then LP3 | charter; chapter 10, RPI.79 |
| H-06: per-feature costs and the feature list | the design line | renderer experiment plan, section 6 |
| W-J item 7: overlay costs | the design line | packet E-2.4-0J |
| H-05 pointing, with picking in the core (F18) | the design line and LP3 | protocol, section 4.6; renderer feature matrix |
| Screen positions for UI anchored to the 3D view | LP3 and the vertical slice | S-A2 card, H-09 |
| Per-monitor DPI | the vertical slice | FRAMEWORK-FACTS G10 |
| The presented-frame report (RM.26) | LP3 | [renderer freeze](../../wiki/decisions/renderer-freeze.md), risks |

## What stays with the design line

The vertical slice of stage 2.4 (one complete core flow in the chosen layout and style, with a gate measurement) and
the living test stay open. They are held with the design line until LP2 to LP5 are done (owner, 6 November 2026), and
they now run on S-A2.
