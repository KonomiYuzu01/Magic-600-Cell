---
id: owner-decisions-2026-10-02
type: decision
status: verified
visibility: public
summary: Owner decisions of 2 October 2026 - stage 2 days count the owner's working days with exit days as targets, and 1.0 keeps later macOS and Linux versions cheap.
related: [owner-decisions-2026-10-01, owner-decisions-2026-09-29, renderer-candidates]
supersedes: []
claims:
  - {id: working-day-schedule, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: future-macos-linux, evidence_kind: decision, checked_at: 2026-10-02}
  - {id: windows-first-line, evidence_kind: source, path: docs/architecture/1.0/10_V1_ARCHITECTURE.md, line: 748, checked_at: 2026-10-02}
---

# Owner decisions, 2 October 2026

These decisions replace the calendar-day count and the no-extension rule of the [1 October](owner-decisions-2026-10-01.md) "Stage 2 within 20 days" decision, and add a 1.0 architecture constraint.

## Stage 2 days are working days, and exit days are targets

The owner has other commitments and asked that the schedule count working days and not be held too tight.

- **Day numbers count the owner's working days, not calendar days.** A day counts when the owner works on the project that day; a day the owner does not work does not count. The owner can correct the count at any time. Day 1 was 1 October 2026. The owner did not work on 2 October, so the next working day is day 2. Calendar dates in the stage 2 plans are no longer deadlines.
- **Exit days are targets, not hard limits.** When a task is likely to miss its target day, the integrator says so once, with a short re-plan (move the target, cut the task down, or drop it), and the owner chooses. Nothing is cut or extended automatically, and the owner is not chased for a date.
- **The plan keeps its size and order.** Stage 2 is still planned as about 20 working days with the same sub-stages and handoffs, the bare Direct3D 12 probe first, the go/no-go on window day 7 and the 12-day renderer window, all counted in working days.
- **Relaxing time never relaxes a gate.** The [renderer selection gate](renderer-candidates.md), the Astra gate rulings, the differential oracle, the review rules and every owner sign-off are unchanged. A late result is a late result, not a pass.
- Agent work that needs no owner input (for example the inventory shards and reviews) may run on any day. It does not advance the day count, and anything that needs the owner, the owner's computer or a sign-off waits for a working day.

## Future macOS and Linux versions

The owner plans macOS and Linux versions after 1.0 and asked for the cheapest path. 1.0 still ships on Windows first, and Direct3D 12 stays the base of the dedicated renderer ([29 September](owner-decisions-2026-09-29.md)). This restores the intent of the 1.0 architecture document's "Windows first; Linux/macOS later" (line 748), which survives the superseded egui/wgpu stack choice on line 746. The 1.0 architecture keeps a later port cheap:

- **Platform separation.** The engine, session store, command layer and view model contain no Windows-only code or APIs. Windows-specific code (windowing, file locking, process ownership, paths, Direct3D 12) sits behind narrow platform interfaces.
- **One shader source.** Shaders are written once, in HLSL, and compiled for Direct3D 12 now. The source stays portable so that it can later be compiled to SPIR-V for Vulkan (Linux) and translated to Metal (macOS) without a rewrite.
- **A small renderer backend interface.** The renderer keeps a thin internal backend interface, so that a Vulkan or Metal backend can be added later without changing the layers above it.
- **No new Windows lock-in.** A new Windows-only dependency outside the platform layer needs a recorded reason and a named replacement path for macOS and Linux.
- **Headless checks stay portable.** The platform-independent layers and the differential oracle keep running headless on Linux and macOS, as the cloud sessions already do on Linux. Whether to add macOS runs to continuous integration is decided when CI is set up, since it may carry a cost.

Out of scope for 1.0: building, testing or releasing a macOS or Linux version. This constraint takes no time from the stage 2 schedule and does not change the renderer selection gate; the stage 2.5 architecture freeze checks it. The 0.4 migration and the H6 clean start stay Windows-specific, because 0.4 ran only on Windows. Evidence rules are unchanged: a headless or Linux result is never Windows/DirectX, macOS or performance evidence.
