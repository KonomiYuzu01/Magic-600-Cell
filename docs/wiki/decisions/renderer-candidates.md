---
id: renderer-candidates
type: decision
status: draft
visibility: public
summary: Stage 2.4 renderer candidates, the owner's performance gate for choosing one, and optional NVIDIA enhancements.
related: [owner-decisions-2026-09-29, development-loop]
supersedes: []
claims:
  - {id: candidates, evidence_kind: decision, checked_at: 2026-09-29}
  - {id: selection-gate, evidence_kind: decision, checked_at: 2026-09-29}
  - {id: nvidia-optional, evidence_kind: decision, checked_at: 2026-09-29}
---

# Renderer candidates and selection gate

Owner decisions of 29 September 2026. Technical statements about vendor features come from model knowledge and are **not yet verified**; versions, hardware support and licence terms must be checked before use.

## Candidates (stage 2.4)

All candidates render through Direct3D 12, the vendor-neutral main path. Rendering never changes mechanical state or relabels pieces.

| ID | Candidate | Role |
| --- | --- | --- |
| S-B | Bare Direct3D 12 interop probe | Mandatory first experiment (1 to 1.5 days), before deeper Godot work. Establishes the baseline and the resource-sharing path. |
| S-A2 | Godot 4.7 (.NET) with a custom RenderingDevice renderer | Full candidate. |
| S-D | Qt Quick with `QQuickRhiItem` on the QRhi Direct3D 12 backend | Full candidate. |

The window was 15 working days; the owner decision of [1 October 2026](owner-decisions-2026-10-01.md) shortens it to 12 days (stage days 3 to 14) with the go/no-go on window day 7 (final ruling by Codex Astra). NVIDIA-specific work is at most an add-on experiment and is never on the day-7 critical path.

## Selection gate: full detail at a stable 30 fps

A renderer is selected only if it passes this gate on the owner's machine. The gate applies at the day-7 go/no-go and at final selection. It is separate from the 0.4.1 B4-12 formal measurement (3 x 100).

**Target hardware:** NVIDIA GeForce RTX 4070 Laptop GPU, 8 GB VRAM. Laptop power and clock limits apply, so tests use laptop conditions.

**Workload**
- Full detail: all 259,800 sticker slots visible, highest detail level, no filtering or level-of-detail reduction.
- Animation: the most complex turn animation, plus continuous operations and camera rotation.

**Pass criteria**
- Frame times recorded with PresentMon, not only average frame rate.
- Average frame rate at least 30 fps **and** the 99th-percentile frame time at most 33.3 ms.
- Each scene runs continuously for several minutes and is repeated three times.

**Conditions** (recorded with every run)
- On mains power, high-performance power mode.
- The program runs on the discrete GPU, confirmed in the NVIDIA Control Panel, not on integrated graphics.
- Driver version, resolution, display refresh rate and vertical-sync setting.

**Memory**
- Record peak VRAM. Keep the peak at or below about 7 GB; above that the system spills to shared memory and frame pacing degrades. A candidate above this budget is flagged as a risk even if it meets the frame-rate criteria.

**Does not count as a pass**
- 30 fps reached with DLSS frame generation: generated frames do not represent rendering capability.
- DLSS super resolution may run as a separate comparison set, but the pass decision uses native resolution.

## Optional NVIDIA enhancements

Layered on top of the vendor-neutral Direct3D 12 path, enabled by hardware detection, **off by default** and never on the correctness path.

| Area | Technology | Use for this project |
| --- | --- | --- |
| Debugging and crash diagnosis | Nsight Graphics, Nsight Aftermath | Most valuable: captures GPU crash and TDR state for 0.4.1 debugging and performance work. Nsight Graphics is owner-installed. |
| Input latency | Reflex (Streamline SDK) | Lower key-to-photon latency for a keyboard-driven program. |
| Upscaling and frame generation | DLSS super resolution, Ray Reconstruction, frame generation (2x on the 40 series) | Presentation or preview modes only. Temporal reconstruction can alter sticker colours, labels and edges, so these modes never produce evidence screenshots or state inspection. |
| Ray tracing and geometry | DXR, Shader Execution Reordering (via NVAPI), Opacity Micromaps | Possible help for many translucent stickers; high complexity and vendor lock-in. Candidate add-on experiment only. |
| General D3D12 Ultimate | Mesh shaders, DXR 1.1, variable-rate shading | Vendor-neutral; may be evaluated on the main path. |

Rules:
- Evidence captured with DLSS or frame generation enabled is neither actual Windows/DirectX evidence nor performance evidence unless the enabled state is stated explicitly.
- Godot has no built-in DLSS; using it would need a GDExtension or integration at the bare Direct3D 12 layer.
- Streamline/DLSS and NVAPI SDKs have their own redistribution terms; confirm with the owner before adoption (new commercial licence terms).
