# H6-P probe implementation report

Evidence kind: source, Windows compilation and CPU fixtures. All seven feature
implementations are finished. No feature cost, rendered effect, GPU sorting,
snapshot, Direct3D runtime or gate result has been measured in this packet.
Claude must perform the GPU checks after integration. The old gated W3 result
applies only to build `2b5bf5e6...`; measure the new build's baseline again.

Files changed, relative to this probe:

- `src/probe.h`, `src/probe.cpp`: options, metadata, shared W3f state function,
  comparison helpers, build identity and CPU self-tests.
- `src/gpu.cpp`: feature-specific pipelines/resources, sorting, post-processing,
  paired effect readback, WIC snapshots, display request and W3f render loop.
- New `src/edges.cpp`: base-cell feature-edge masks.
- New `shaders/fog.hlsl`, `shaders/outlines.hlsl`,
  `shaders/transparency.hlsl`, `shaders/sort.hlsl`, `shaders/post.hlsl`.
- `CMakeLists.txt`: those shaders, the edge source and Windows SDK WIC/COM links.
- `run_scene.ps1`: feature and W3f selection, feature run names and exit 4.
- `check_probe.py`: disposable build location permitted by Application Control.
- `README.md`, this `report.md`: feature definitions, evidence and H-06 procedure.

## Feature methods

All feature paths retain 600 cells, 433 stickers per cell and 18,288,000 scene
vertices. Every sticker colour originates from `labels[slot] / 433`, bound at the
same root SRV as before. The chronological label uploads, two-frame upload ring,
three default label buffers, first-use records, preserved copies and oracle
comparison are shared with the baseline. There is no geometry filter or LOD.

| Feature | Method and per-frame passes | Additional resources and shader sources | Finished; cost |
| --- | --- | --- | --- |
| `none` | Original one-sample instanced draw | Original root, PSO, shaders and resources | Yes; unmeasured for this build |
| `no-gaps` | Original draw with sticker shrink 1.0 instead of 0.82 | Original root, PSO and shaders; no extra steady-state resources | Yes; unmeasured |
| `fog` | One mesh draw; per-pixel exponential fog by view depth towards the clear colour; transmittance `exp(-0.23 * max(z - 2.2, 0))` | Own root/PSO; `fog.hlsl` includes unchanged baseline draw code | Yes; unmeasured |
| `outlines` | One mesh draw; masked, non-perspective barycentrics divided by the Euclidean screen derivative give pixel distance; darkening blends across 1–2 px, centred at 1.5 px | Own root/VS/PS PSO and 10,160 u32 edge masks; `outlines.hlsl` | Yes; unmeasured |
| `transparency` | Centre keys, 171 bitonic compare dispatches, block prefix scan, block-total scan, index generation; then one indexed draw with alpha 0.6, standard alpha blending and depth writes off | Own root/graphics PSO and five compute PSOs; 262,144 eight-byte keys, u32 prefix storage, 1,024 block offsets, sticker offsets, and 73,152,000-byte index buffer; `transparency.hlsl`, `sort.hlsl` | Yes; unmeasured |
| `dof` | One offscreen mesh draw, then one native-resolution 32-tap CoC-weighted disc gather composited directly into the back buffer | Own scene root/PSO, post root/PSO and SRV heap; native colour target and readable depth; `post.hlsl` | Yes; unmeasured |
| `ao` | One offscreen mesh draw; full-resolution 16-sample view-space SSAO, horizontal and vertical depth-aware blur, then colour composite | Own scene/post roots and scene/AO/blur/composite PSOs; native colour target, readable depth and two native R16_FLOAT AO targets; `post.hlsl` | Yes; unmeasured |
| `msaa4` | Existing 4x multisampled mesh draw and resolve | Existing MSAA target/depth/PSO path; no new steady-state shader | Yes; unmeasured |

Transparency sorts all 259,800 stickers every frame, padded to 2^18. The key is
the view depth of the transformed sticker centre, using the cell/sticker shrink
expression, cell frame, animated-slot turn and camera from `projectVertex`.
Padding has the lowest finite float key and an invalid id; ties use sticker ids.
An exclusive prefix sum over the actual 12–252 vertex counts places each sorted
sticker's original range in the index list. The vertex shader decodes a global
index into cell and base vertex. Every original triangle, including degenerate
and duplicate triangles, is drawn exactly once. UAV barriers order every dependent
sort/scan step and buffer reuse; no steady-state GPU drain is introduced.

DOF and AO alone use R32_TYPELESS depth with a D32_FLOAT DSV and R32_FLOAT SRV.
Depth reconstruction is `z = 0.05 / (1 - d / (100 / 99.95))`; view x/y follow the
projection in unchanged `geometry.hlsl`. The model centre stays at view depth 5,
so DOF focuses there. Maximum CoC is `8 * height / 1600` pixels; the gather weights
samples by the centre and neighbour CoC. AO samples a 0.22-unit hemisphere and
uses view-depth differences in both blur directions. Every pixel executes its
16 AO samples; the clear background retains AO 1.

## Edge classification and numerical detail

Edges match exact four-component asset positions within each sticker. Duplicate
triangles merge only for adjacency; masks propagate to the original triangles.
An edge is outlined if it has one or at least three incident triangles, or if
the normalized three-vector Gram determinant exceeds `1e-6`. Coplanar diagonals
remain unmasked. The width uses the length of `ddx`/`ddy`, rather than their sum,
to keep the perpendicular screen width independent of edge orientation.

The packet does not specify the zero-area tolerance used in its edge census.
Matching its measured adjacency totals requires ignoring triangles whose
two-vector Gram determinant is at most `1e-18`. The assets have 3,948 exact
repeated-vertex triangles and 562 additional numerically collapsed triangles
under this cutoff. Applying a relative area epsilon instead changed the census;
an independent CPU calculation reproduced the packet's totals with this absolute
cutoff. All these triangles still render; only adjacency and their masks ignore
them. This numerical detail is the sole clarification to the suggested method.

The real-asset self-test proves 3,277 feature edges, 5,546 coplanar diagonals,
819 open edges, 31 edges with at least three triangles, 761 duplicate triangles,
and 1–32 feature edges on every sticker. Thus 7,973 edges have two incident
triangles. No mesh, cut, frame, identity or label asset changed.

## Baseline, state and capture

With `none`, the original root signature, graphics PSO description, shaders,
resources, clear values and GPU command sequence remain unchanged. The optional
code branches emit no extra commands or resources in that path. W2/W3 retain the
original 0.002 rad rotation in plane (0, 3) before every ordinary rendered frame,
including preroll, and the original clock-driven turns. The added camera counter
records that pose without changing those operations. Two frames remain in flight.

`frameState(k, T, n, angle)` supplies the camera count, chronological turn index,
phase and signed smoothstep angle for W3f; both the render loop and self-test use
it. Preroll holds the start pose. The camera resets at cycle boundaries, then
rotates once. Label revision follows the unwrapped turn index. W3f preserves
copies in the original-size readback allocation and extends it in equal chunks
if its frame-driven turn count exceeds the clock-derived initial capacity.
W3/W4 retain their original allocation and copy commands. Allocation and storage
can grow for very fast or long W3f runs; no copies are recycled or omitted.

The run format stays `magic600-renderer-run-v1`. It adds `feature`, `camera`,
`window.display_required`, and W3f's `turn_frames`/`cycle_frames`; every trace entry
adds its integer `camera`. The trace remains in memory until after the run.
`SetThreadExecutionState` requests system/display availability through the run
and a scope guard resets it on normal or exceptional exit.

The capture script accepts all feature names and W3f, passes `--feature`, names
non-baseline run/summary directories with the feature, rejects feature injection,
and rejects exit 4 as evidence. Its administrator check, session ownership and
cleanup, cold-process pauses and operator confirmation are unchanged. No capture
or PresentMon process was started here. H6-T owns gate/table tooling.

## Effect and sort checks

Before ordinary preroll and trace timing, every feature renders the identity
camera, solved labels and angle zero twice without presenting: off, then on.
The frozen pair is extra diagnostic work and never advances scene state. Both
frames are copied with D3D12's row-pitched texture footprint into readback and
packed into RGBA. The helper counts each pixel once when any channel differs by
more than 2 byte values, and passes only above 0.1% of all pixels. `feature_effect`
records `changed_pixels`, `pixels` and `status`.

MSAA's off frame uses a one-sample PSO and one-sample depth/DSV allocated only
inside the check. Transparency additionally copies its first 259,800 sorted
keys/ids once. `sort_check` checks finite, descending keys and a complete unique
permutation of ids 0..259,799, recording `status`, `permutation`, `back_to_front`
and `stickers`. Failure of either check prints its reason and returns 4 before
any trace or run JSON is written. Other errors still return 1; label and window
failures still return 2 and 3. These GPU checks are implemented, not executed here.

With `--snapshot`, WIC writes `feature-off.png` and `feature-on.png` to a fresh
inspection location. Existing images are refused. WIC/COM come only from the
Windows SDK (`windowscodecs`, `ole32`). No images were created in this packet.

## Build identity order

SHA-256 concatenates the bytes of these files in this fixed order; any missing
file fails. The self-test now exercises the actual identity function as well.
All shaders use the existing DXC `-nologo -O3 -Ges` flags and shader model 6.0.

```text
sb_probe.exe
draw_vs.dxil
draw_ps.dxil
count_vs.dxil
geometry_cs.dxil
fog_ps.dxil
outline_vs.dxil
outline_ps.dxil
transparency_vs.dxil
transparency_ps.dxil
sort_keys_cs.dxil
sort_cs.dxil
sort_prefix_cs.dxil
sort_blocks_cs.dxil
sort_indices_cs.dxil
post_vs.dxil
dof_ps.dxil
ao_ps.dxil
ao_blur_ps.dxil
ao_composite_ps.dxil
```

## Self-test and acceptance

The CPU additions exercise all eight feature names in all five scenes; feature
pairs, geometry/injection exclusions and the MSAA alias; invalid T/n values and
sequence options outside W3f; every frame across two default cycles, boundary
resets, turn continuity, phase and direction, plus literal half-turn fixtures;
run/trace shapes for rotating and stationary cameras; real-asset edges; strict
image-difference/count thresholds; valid/tied sorted keys and duplicate, missing,
out-of-range, ascending and nonfinite key fixtures. Existing asset, turn, label
and 200-second synthetic trace checks remain.

`check_probe.py` initially built successfully in system temp, but Windows
Application Control blocked launching the unsigned executable with WinError 4551.
The disposable build is now `probe/build-check-<pid>`, inside this packet's owned
directory, and is removed after the check. That location runs the self-test under
the existing policy; no policy, tool version or privilege was changed.

Actual installed check tools: Python 3.14.7, CMake 4.3.1-msvc1 (Visual Studio
fallback), Ninja 1.13.2, MSVC x64 19.51.36260, DXC/dxil 1.8.2502.11. The pinned
CMake venv is absent in this isolated checkout. The existing `vswhere.exe` lookup
diagnostic and C++20 `u8path` deprecation warnings are non-blocking; the actual
compiler and shader commands complete. No new tool or dependency was downloaded.

Final acceptance command:

```text
python work/experiments/renderer-sb/probe/check_probe.py
```

Final acceptance exited 0. Sanitized output (build paths, repeated progress and
the deprecation warnings described above omitted):

```text
acceptance: serial Ninja command execution (Windows sandbox)
CMake: 4.3.1-msvc1
Ninja: 1.13.2
MSVC: 19.51.36260 for x64
DXC: dxcompiler.dll 1.8.2502.11; dxil.dll 1.8.2502.11
sandbox build command 23/23
build: ok
acceptance build: exit 0
selftest: edge counts: features=3277, diagonals=5546, open=819, multiple=31, degenerate=3948, duplicates=761
selftest: assets and oracle digests: ok
selftest: executable and 19 shaders in build identity: ok
selftest: clean labels and four injected faults: ok
selftest: turn arithmetic and synthetic 200 s run/trace shape: ok
selftest: feature options, w3f, camera/display metadata, effect/sort helpers: ok
selftest: ok
acceptance selftest: exit 0
```

PowerShell AST parsing of `run_scene.ps1` also passed; the script was not executed.
`git diff --check` passed. Direct byte comparison to `HEAD` proved all three
baseline shaders unchanged, recorded build-input digests matched current files,
the compiled shader list matched the identity order, and no acceptance build
directory remained. No commits, staging, refs, Git configuration or network
operations were performed. The packet stop condition holds; GPU validation and
measurement remain open for integration.

## Input identity

Baseline shader SHA-256 values match those read before implementation:

```text
shaders/draw.hlsl      e966831d900ee7bc35de114b48f6aca108fc9bb08b736a76ae7e4b2cd6c4890f
shaders/geometry.hlsl  1f9a18dc2c994b038963968ce79eddfb744209e9534f801b2ad190e0d7e36e7c
shaders/check.hlsl     d9e41fb05066acf170d35102f5ab13cbdf359ac5d294e7892ae3ec08996ae9ab
assets/manifest.json  b5db4469f3da9ba3cdbd93806a703f594153e61b08ce32d4fcce9de9c5cc60bc
cameras.json          6bb4288cc46e9ad4516b99d74322293dfe9eb076f12f58dc0a7d0e51b6403c50
workload/turn.json    13811e74385b7a07e2e66d7177178b41fdac7921a6e2fd1287e6eac9504445e1
```

Matching implementation build-input SHA-256 values (paths relative to probe):

```text
CMakeLists.txt             4fbc69b73ea695438d8d04f7de90bfd6aca5cce737ac6ed4fa9b22a7cb210733
src/probe.h                b30adf2dff1ea03642384cbdb92b9db69afd315f5ac4b2bd31260e2c7efb27ab
src/probe.cpp              c641526181b0ca808bb745f9ef087995f292629c1e4d3e3425a2ec8bea057d39
src/gpu.cpp                ea3379ff688eba4d25ccd350d35376cc46b1f7098f4b2402d4a7a47109758ec8
src/edges.cpp              836a45bed4c1847e7d642b723f40c2fe77711c4e4d3a106af77423e0f3c609bf
shaders/fog.hlsl           7a4e03fb97a4d128eab90d2a495da7ed63bb17de978b6cf00c4584869b2d50c1
shaders/outlines.hlsl      b84ba76993bd918fb8ebfb5129997b75a9c5e8cc16681b53c0fcc186e6c06d19
shaders/transparency.hlsl  374114b9404442f6b72225ae6d6b903e5708c965cb2e996ea63f65bfb25eb75a
shaders/sort.hlsl          4a1ae203eea48c1dfd1fb0dfaf33226bc14d0397fda294f85ba7157576cfe6d5
shaders/post.hlsl          2fb16a50f1a40cd2807e36008839d97853d45ea93aef8ad5003969168542f4bc
check_probe.py             58fa73ca7b87e18dd8a635a500ea577678162478dc569f365551c8879b8c7783
run_scene.ps1              bb65a943faec9856b55562278a811710caf8ef0a1d05074e8daff8c2bfa1b70a
```
