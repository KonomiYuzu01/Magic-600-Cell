# LX-7 W3 plan

Status: **not run here**. Decision: **open**, decider: **owner**. This file supplies
no renderer integration, foreground-window approval or gate result.

Before measuring, the integrator needs a renderer candidate with an explicit stage
selection interface, the frozen generated HLSL and built-in perspective stage wired
at the same 4D-to-3D point in its vertex path, and correctness readbacks showing that
the intended stage runs. It must retain full shrink, pose, animation, revision and
picking semantics. The compute harness does not implement that renderer interface.
The comparison inputs, shader bytes, native build and stage choice must be recorded
by digest. An accepted RPI.36 precision/exclusion/crossing-triangle rule precedes
any judgement; the experiment's radius is not that acceptance.

Use the renderer experiment plan's W3 fixture, most complex continuous turn
animation and camera rotation. Freeze camera/turn schedule, full-detail scene and
build before the series. Hold the foreground window with the owner; do not launch
a capture over unrelated work. For built-in perspective and generated perspective
make **three separate cold launches each**, several minutes per run. If another
generated chart is proposed for the renderer, it needs its own three cold runs;
its compute comparison cannot stand in for them.

All three runs of each stage independently need the whole unrelaxed gate:

- All 259,800 sticker slots, highest detail, no filtering or LOD reduction;
  30,480 base vertices times 600 cells, or 18,288,000 evaluations per frame.
- Owner's NVIDIA GeForce RTX 4070 Laptop GPU, 8 GB, at native resolution;
  no frame generation or upscaling; no vendor feature on the correctness path.
- PresentMon average at least 30 fps and p99 frame time at most 33.3 ms;
  peak VRAM at most about 7 GB. Report each run and generated-minus-built-in deltas.
- Mains power, high-performance Windows mode, discrete GPU, vendor performance
  mode, driver, native display size, refresh, V-Sync and overlays recorded.
- Full correctness/adoption readbacks and the renderer plan's W3/W5 requirements;
  invalid correctness invalidates timing, and a missed cold run is no pass.

The existing S-B command shape, **for its existing built-in stage only**, is:

```powershell
powershell -NoProfile -File work/experiments/renderer-sb/probe/run_scene.ps1 -Scene w3 -Runs 3 -Duration 192 -Preroll 4 -Overlays 'none running'
```

Use that exact declaration only when no overlay is running. Its foreground-window,
administrator/PresentMon and condition prompts remain applicable; this packet
grants no new privilege. This command cannot measure the generated stage today.
There is **no executable generated-stage W3 command yet**: the candidate and stage
switch do not exist in this packet. The integrator must first implement and review
that handoff, then record its exact commands for both stages. Do not label a repeat
of the unchanged S-B command as a generated-stage result.

Archive per-run build/source/input identities, PresentMon captures and gate reports
under the renderer protocol. Keep public summaries small and sanitized; large raw
captures get a digest and regeneration command. These are performance evidence only
on the owner machine under those conditions. The named-device no-window comparison
is windows evidence; neither it nor CPU agreement proves a chart property, a
renderer selection, E2 input latency or a W3 performance pass.
