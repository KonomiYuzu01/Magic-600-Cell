# Level 2 plan: the S-B drawing method inside Godot and Qt (E-2.4-02 and E-2.4-03, H-09)

Status: plan, written on stage day 3 (3 October 2026), before its plan check. It turns into packets only after that check.

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

## 3. Work split (Codex first)

| # | Part | Who | Owned files | Acceptance in the sandbox | Owner-machine check |
|---|---|---|---|---|---|
| L2-H | ABI version 2 of `sa2_interop.h`, additive scene functions | Claude | `renderer-sa2/native/include/sa2_interop.h` | header compiles in the L2-N build | — |
| L2-N | Implement ABI 2 by porting S-B's drawing path into the DLL | Codex | `renderer-sa2/native/**` except the header | `check_native.py` (build plus CPU self-test) | `check_native.py --gpu`: W1–W4 offscreen on WARP and the RTX 4070, geometry and label checks pass |
| L2-G | Godot harness, level 2 | Codex | `renderer-sa2/project/**` and the run and check scripts in `renderer-sa2/` | `check_project.py` | level-2 runs, then three owner-attended cold W3 runs |
| L2-Q | Qt harness, level 2 | Codex | `renderer-sd/**` | `check_project.py` | the same, candidate `sd` |
| H-09 | Constraint lists | Claude | both `RESULT.md` files | — | from the level-2 runs, with framework source facts verified against the pinned versions |

The scene functions L2-H adds:
- load the S-B workload through S-B's own readers, without changing S-B files;
- draw scene Wn, frame k, into a registered slot, with level 1's queue modes and fence protocol;
- read back the geometry at the reference cameras;
- copy the label buffer and run the exact label check;
- return per-frame trace records in S-B's `trace.jsonl` format, with the framework's present as the frame boundary.

Sequencing:
- L2-G and L2-Q start in parallel once L2-N is integrated, because both depend on its ABI.
- Every Codex-authored change gets Claude's review. Non-critical code gets one Sol fast review; none of these files is on a critical path.

## 4. Order

| Stage day | Steps |
|---|---|
| 4 | Sol plan check of this plan. Write and commit L2-H, then dispatch L2-N. Run the H-06 preliminary series while the owner is away (section 5). |
| 5 | Integrate L2-N and run the GPU self-test. Dispatch L2-G and L2-Q in parallel. |
| 6 to 8 | Integrate. Level-2 runs, then the owner-attended W3 runs per candidate, then the H-09 lists and the result cards. |
| 9 (window day 7) | Packet E-2.4-04: the day-7 go/no-go, an Astra gate ruling. |

Day numbers count the owner's working days. The owner may move them.

## 5. Machine rules

- Level-2 and GPU self-test runs without PresentMon may run unattended on an idle machine on mains power. They are labelled preliminary. W3 gate runs are owner-only.
- **H-06 preliminary cost series** (27 runs, about 90 minutes): moved to stage day 4 on the owner's instruction.
  - Start it from PowerShell in the `claude/renderer-sb` checkout: `powershell -NoProfile -ExecutionPolicy Bypass -File work\loop-memory\perf\renderer\sb-h06-helpers\start_h06_watch.ps1`. The folder is private and ignored by Git.
  - It runs only after 5 minutes without owner input, and only while no build, Codex call, Godot, Blender or FFmpeg process runs.
  - It pauses when the owner returns. Esc closes a run's full-screen window.
  - Afterwards, `tools/perf/feature_costs.py <run dirs> --timing trace` builds the table. Only the sanitised preliminary table is committed.
- No Codex implement call and no build may run during H-06 runs or gate captures. No commit, tag or push may happen during an implement call.
