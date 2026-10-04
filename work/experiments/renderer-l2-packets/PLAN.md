# Level 2 plan: the S-B drawing method inside Godot and Qt (E-2.4-02 and E-2.4-03, H-09)

Status: plan, written on stage day 3 (3 October 2026). The Astra plan check (20261003T213525Z-b263caa3) found the native route and the shader exception admissible and raised two major gaps; this revision of stage day 4 answers them in section 3 and moves the ABI 2 header text into L2-N. It turns into packets after the scoped re-check. L2-N is integrated (stage day 4; see the attempts table of its packet). The section 3 checks are specified exactly in `HARNESS.md`. A shared finalizer and runner (L2-F) now judge both candidates, so section 4 has three harness packets.

Level 2 is unlocked. S-B met the W3 gate on three owner-attended cold runs of build `2b5bf5e6`. Both framework candidates passed level 1: Godot 4.7.2 .NET (`work/experiments/renderer-sa2/RESULT.md`) and Qt 6.10.3 (`work/experiments/renderer-sd/RESULT.md`).

## 1. What level 2 must show

The source for this section is packets E-2.4-02 and E-2.4-03, acceptance items 2, 3 and 5.

- W1 to W4 inside each framework, drawn with the S-B drawing method.
  - Both checks are the same as in S-B: the geometry check against the outputs of `reference_geometry.py`, and the exact label check.
- Three cold W3 runs per candidate, judged by `tools/perf/renderer_gate.py`. The candidate names are `sa2` and `sd`.
  - These runs are owner-attended, with the operator confirming overlays (Astra ruling 20261003T033021Z-274f20af).
- An H-09 constraint list per framework, due on day 8. It covers:
  - overlay layers;
  - text in the 3D view;
  - transparency and sorting limits;
  - how the shaders are written.
- Not in this plan: the H-04 layout and H-01 feature-list prototypes (acceptance item 4). They wait for the design track's day-6 outputs.

## 2. Route (proposed)

**Native route.** The S-B drawing method runs as our own Direct3D 12 code on the framework's device. It lives in the shared producer DLL (`work/experiments/renderer-sa2/native/`, which S-D already loads). It draws into the texture the framework displays. This is the path both level-1 smoke tests proved, with 0 validation errors:
- Godot: `texture_create_from_extension`, on Godot's queue or on the producer's queue;
- Qt: routes A, B and C, on Qt's queue or on the producer's queue.

Why this route:
- One port serves both candidates.
- S-B's shaders, turn logic and checks are reused with unchanged behaviour.
- It follows the owner's decision that Direct3D 12 is the base of the dedicated renderer.
- It is the shortest path to W3 runs before the day-7 ruling.

**Stated exception for H-09.** The shaders stay portable HLSL compiled to DXIL with DXC. They do not go to SPIR-V for Godot's RenderingDevice or through Qt's shader tools, because the framework never sees them.
- The cost: framework-native effects (Godot's compositor, Qt's scene graph) reach the 3D view only as layers above or below it.
- H-09 records this per framework.

**Alternative, not proposed now.** Port the drawing method into Godot's RenderingDevice (SPIR-V) and into QRhi (qsb).
- This would answer whether the framework's own renderer can carry 259,800 slots.
- It roughly doubles the work, so it is the fallback only if the native route fails the gate.

## 3. What each harness must prove before it writes a gate record

`HARNESS.md` gives the exact behaviour: options, call order, `harness.json`, refusal codes, identity and runner. `tools/perf/renderer_gate.py` stays unchanged: nothing under the root `tools/` changes. The gate trusts the declared backbuffer size and the declared build identity, and it accepts a run without `vram_peak_mb`. So the level-2 harness checks the following itself: each app (L2-G, L2-Q) records the facts, and the shared finalizer (L2-F) checks them and refuses to write a gate run record when any check fails. The refusal is an exit code and a reason; no partial record is written.

**Native resolution (answers L2-PLAN-001).**
- Each run records four sizes in physical pixels:
  - the producer's render target;
  - the viewport the DLL rasterises with;
  - the 3D area the framework displays;
  - the swap chain's backbuffer.
- It also records the framework's render scaling:
  - Godot: the 3D scaling mode and scale, and the window's content scale;
  - Qt: the device pixel ratio and the item's size in device pixels.
  These settings are verified against the pinned versions.
- A W1 to W4 run is valid only when all four sizes are equal and the render scaling is 1. In level 2 the 3D view fills the window.
- Acceptance test, before any formal capture: a debug switch that halves the producer target must make the harness refuse the run.

**Complete run record and composite build identity (answers L2-PLAN-002).**
- The level-2 harness owns the whole run record that `renderer_gate.py` reads, in S-B's formats. The apps and the DLL write their parts, and the shared finalizer writes `run.json`:
  - `run.json` and `trace.jsonl`;
  - the label check;
  - the PresentMon capture;
  - the declared controls.
- **Composite build identity.** The identity is SHA-256 over the sorted list of these part digests:
  - the harness build: for Godot, the project files, the C# assembly and the Godot executable; for Qt, the harness executable;
  - the producer DLL file and every shader blob it loads, which the DLL reports through an ABI 2 function;
  - the framework settings that affect rendering: for Godot, the rendering settings in `project.godot` and the command-line options; for Qt, the Qt version, the Qt DLLs the process loads and the graphics-API environment variables.

  The run record lists the parts next to the identity. Acceptance test: changing only the DLL changes the identity.
- **Peak VRAM** of the framework process. The DLL samples the process's local video memory use during the trace window: `QueryVideoMemoryInfo` on node 0, local segment, `CurrentUsage`. This is the same measurement as S-B, so it includes the framework's own allocations.
  - A gate run record without `vram_peak_mb` is refused.
  - Acceptance test: a run with VRAM sampling switched off is refused.
- Before the three formal runs per candidate, one short framework capture shows two things:
  - PresentMon selects the framework's process and its swap chain;
  - the trace has exactly one scene entry per present interval.

## 4. Work split (Codex first)

| # | Part | Who | Owned files | Acceptance in the sandbox | Owner-machine check |
|---|---|---|---|---|---|
| L2-H | ABI version 2 contract: the additive scene functions, their semantics and error rules, written into the L2-N packet | Claude | the L2-N packet | — | — |
| L2-N | Write `sa2_interop.h` ABI 2 to that contract, and implement it by porting S-B's drawing path into the DLL | Codex | `renderer-sa2/native/**`, including the header | `check_native.py` (build plus CPU self-test) | `check_native.py --gpu`: W1–W4 offscreen on WARP and the RTX 4070, geometry and label checks pass |
| L2-F | Shared finalizer and runner (`HARNESS.md` sections 7 to 9): the only writer of run records, with every section 3 refusal | Codex | `renderer-l2/**` | `check_l2.py`, including a fixture test of each refusal | used by every level-2 run of both candidates |
| L2-G | Godot app, level 2 (`HARNESS.md` sections 2 to 6), and its prepare step | Codex | `renderer-sa2/project/**`, `renderer-sa2/prepare_l2.py`, `renderer-sa2/check_project.py`, `renderer-sa2/README.md` | `check_project.py`: ABI 2 pins and static checks of the app | level-2 runs, the section 3 acceptance tests and short capture, then three owner-attended cold W3 runs |
| L2-Q | Qt app, level 2 (`HARNESS.md` sections 2 to 6), and its prepare step | Codex | `renderer-sd/app/**`, `renderer-sd/build.cmd`, `renderer-sd/prepare_l2.py`, `renderer-sd/check_project.py`, `renderer-sd/README.md` | the same, `renderer-sd/check_project.py` | the same, candidate `sd` |
| H-09 | Constraint lists | Claude | both `RESULT.md` files | — | from the level-2 runs, with framework source facts verified against the pinned versions |

The scene functions L2-H adds:
- load the S-B workload through S-B's own readers, without changing S-B files;
- draw scene Wn, frame k, into a registered slot, with level 1's queue modes and fence protocol;
- read back the geometry at the reference cameras;
- copy the label buffer and run the exact label check;
- return per-frame trace records in S-B's `trace.jsonl` format, with the framework's present as the frame boundary;
- report the producer target's size and the raster viewport (section 3);
- report the DLL's identity part: the digests of the DLL file and of each shader blob it loads (section 3);
- sample the process's local video memory during the trace window and report the peak (section 3).

The header moves from Claude to L2-N to save a round. The contract stays Claude's: Claude checks the header text against the contract when integrating L2-N, before L2-G and L2-Q are dispatched.

Sequencing:
- L2-F, L2-G and L2-Q start in parallel once L2-N is integrated and `HARNESS.md` is committed. `HARNESS.md` fixes their interfaces (`harness.json`, `launch.json` and the finalizer's command line), so none waits for another.
- One finalizer and one runner judge both candidates. A difference between the candidates then comes from the frameworks, not from two copies of the checks.
- Packet E-2.4-03 allows changes only under `renderer-sd/`, while S-D loads the DLL from `renderer-sa2/native/`. So L2-Q builds against the integrated L2-N header and DLL without changing them.
- Every Codex-authored change gets Claude's review. Non-critical code gets one Sol fast review; none of these files is on a critical path.

## 5. Order

| Stage day | Steps |
|---|---|
| 3 (night) | Astra plan check of this plan (`plan-check-1.md`): the route sets how both candidates are judged and adds a contract, so it is a design decision. The H-06 preliminary series runs once that check ends and the owner is away (section 6). |
| 4 | Answer the plan check (this revision, scoped re-check). Write the L2-N packet with the ABI 2 contract and dispatch it. |
| 5 | Integrate L2-N: check the header against the contract, then run the GPU self-test. Dispatch L2-F, L2-G and L2-Q in parallel. |
| 6 to 8 | Integrate. Level-2 runs, the section 3 acceptance tests and the short capture per candidate, then the owner-attended W3 runs per candidate, then the H-09 lists and the result cards. |
| 9 (window day 7) | Packet E-2.4-04: the day-7 go/no-go, an Astra gate ruling. |

Day numbers count the owner's working days. The owner may move them.

## 6. Machine rules

- Level-2 and GPU self-test runs without PresentMon may run unattended on an idle machine on mains power. They are labelled preliminary. W3 gate runs are owner-only.
- **H-06 preliminary cost series** (27 runs, about 90 minutes): started on the night of stage day 3 on the owner's go. It stopped after three attempts without a valid run. Every run's window was visible but never in the foreground, because Windows does not give the foreground to a window started from a background process. It runs again after a fix to the private helper.
  - Start it from PowerShell in the `claude/renderer-sb` checkout: `powershell -NoProfile -ExecutionPolicy Bypass -File work\loop-memory\perf\renderer\sb-h06-helpers\start_h06_watch.ps1`. The folder is private and ignored by Git.
  - It runs only after 5 minutes without owner input, and only while no build, Codex call, Godot, Blender or FFmpeg process runs.
  - It pauses when the owner returns. Esc closes a run's full-screen window.
  - Afterwards, `tools/perf/feature_costs.py <run dirs> --timing trace` builds the table. Only the sanitised preliminary table is committed.
- No Codex implement call and no build may run during H-06 runs or gate captures. No commit, tag or push may happen during an implement call.
