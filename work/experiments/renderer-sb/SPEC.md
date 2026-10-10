# S-B probe: shared specification (stage 2.4, packet E-2.4-01)

Interface contract for the S-B bare Direct3D 12 probe, its Python geometry reference and its handoff test. The plan ([renderer-experiment-plan](../../../docs/progress/1.0/renderer-experiment-plan.md) sections 1, 2 and 5) and the gate ([renderer-candidates](../../../docs/wiki/decisions/renderer-candidates.md)) are the authority; this file pins the details they leave open so that separately written parts agree. Written by the integrator on 3 October 2026.

## 1. Layout of `work/experiments/renderer-sb/`

| Path | Owner | What |
|---|---|---|
| `SPEC.md`, `cameras.json` | integrator | this contract; the fixed cameras of the geometry check |
| `workload/export_workload.py`, `workload/turn.json` | integrator | W3 turn data exported from `core.Model` with the engine Python (NumPy) |
| `reference_geometry.py`, `reference/`, `check_assets.py` | packet SB-A | stdlib-only Python reference of the projection and its committed outputs |
| `probe/` | packet SB-B | the D3D12 probe (C++20, CMake, Ninja, HLSL compiled with DXC), run scripts, README |
| `handoff/` | packet SB-C | the resource-handoff test with fence synchronisation |

## 2. Assets (read-only, synthetic)

All little-endian. Verify the SHA-256 of every file against `assets/manifest.json` (`files` maps `assets/<name>` to a hex digest) before use.

| File | Content |
|---|---|
| `assets/mesh.json` | `base_vertices` 30480, `base_stickers` 433, `cells` 600, `slots` 259800, `offsets` (434 ints, vertex range of each base sticker), `normal` N0 (4 floats), `normal_length` (float) |
| `assets/mesh_vertices.f32` | 30,480 x 4 f32: the 4D vertices of base cell 0, an unindexed triangle list (10,160 triangles) |
| `assets/mesh_sticker.u32` | 30,480 u32: base sticker index (`local`, 0..432) of each vertex |
| `assets/mesh_centers.f32` | 433 x 4 f32: the retained toolkit's sticker numbering centres (4D). Digest-checked with the others, but since 10 October 2026 no step of the pipeline uses them: they do not move with the sticker under the puzzle's symmetries |
| `assets/cell_frames.f32` | 600 x 16 f32: cell frame F[c]; the 16 floats of cell c are F[c] in column-major order (float `c*16 + col*4 + row` is F[c][row][col]) |

Slot of a vertex: `slot = cell * 433 + local`. A label is the original slot id of the sticker in that slot; its colour class is `label / 433` (integer division). Solved labels: `label[slot] = slot`.

## 3. Geometry pipeline (port of `web/renderer.js:21-34`, mesh mode, not wireframe, except the anchor of step 1)

Shrink anchors (owner decision, 10 October 2026: [owner-decisions-2026-10-10-anchor](../../../docs/wiki/decisions/owner-decisions-2026-10-10-anchor.md)). The anchor `A[local]` of base sticker `local` is the area-weighted centroid of its triangles, computed from `mesh_vertices.f32` and `offsets`. It replaces the sticker centre of `renderer.js`.
- **Triangles.** The triangles of sticker `local` are the consecutive vertex triples `(a, b, c)` in `[offsets[local], offsets[local+1])`, in file order. Each f32 component is widened to float64, and all arithmetic up to the final rounding is in float64.
- **Area.** For each triangle, `e1 = b - a` and `e2 = c - a`. Each dot product sums its four component products in the order 0, 1, 2, 3. Then `area = 0.5 * sqrt(max(dot(e1,e1)*dot(e2,e2) - dot(e1,e2)^2, 0))`.
- **Centroid.** `total` is the sum of `area`, and `weighted[i]` is the sum of `area * ((a[i] + b[i] + c[i]) / 3)`, both summed in triangle order. Then `A[local][i] = weighted[i] / total`, rounded once to f32 (round to nearest even).
- **Refusal.** A sticker whose `total` is not finite and positive, or whose anchor is not finite, refuses the assets.
- **Use.** Every consumer uses the f32 anchors: the GPU reads them, and the float64 references widen them again.
- **Pin.** For the current assets, the 433 x 4 anchors as f32 little-endian have SHA-256 `0b6ead284d3f62719e3e6ec893d6c199d49698d246a59164bc362446e39faca6`. `reference_geometry.py` (`sticker_anchors`) is the stdlib definition. An implementation that computes other bytes from these assets is wrong.

Per vertex `vi` (0..30479) of cell `c` (0..599):

1. `v` = vertex `vi`; `local` = sticker of `vi`; `center` = anchor `A[local]`; `slot = c*433 + local`.
2. `v = (N0 + cs*(center - N0) + cs*ss*(v - center)) / R`, with `R` = `normal_length`.
3. `world = F[c] * v` (column vector).
4. Turn: if `slot` is in the animated set and the turn angle `theta != 0`: `x = dot(world, u)`, `y = dot(world, v4)`, `world += ((cos(theta)-1)*x - sin(theta)*y) * u + (sin(theta)*x + (cos(theta)-1)*y) * v4`, where `u`, `v4` are `plane_u`, `plane_v` of `turn.json`.
5. `world = Q * world`, Q a 4x4 matrix kept as 16 floats in column-major order exactly like `renderer.js` `q` (identity at start). `rotate(a, b, t)` updates it as `renderer.js:120`: for j in 0..3: `x = q[a*4+j]`, `y = q[b*4+j]`, `q[a*4+j] = cos(t)*x - sin(t)*y`, `q[b*4+j] = sin(t)*x + cos(t)*y`.
6. `p = d4 * world.xyz / (d4 - world.w)`.
7. `z = 5 - p.z`; clip position `(p.x*zoom/aspect, p.y*zoom, depth(z), z)` with near 0.05 and far 100. Map depth to the D3D range [0, z] (`far/(far-near)*(z - near)`) with a standard less-equal depth test. The checked outputs are convention-free: `ndc_x = p.x*zoom/aspect/z`, `ndc_y = p.y*zoom/z`, and `w = z`.

Parameters (`renderer.js:8`): `cs = 0.76`, `ss = 0.82`, `d4 = 1.18`, `zoom = 1.15`; `aspect` = backbuffer width / height. The reference geometry uses `aspect = 1.6` (2560 x 1600).

Colour (fragment): `class = label / 433`; `rgb = hsv(fract(class * 0.61803398875), 0.61, 0.90)` with `hsv` as `renderer.js:41`; shade `0.5 + 0.5*abs(dot(n, normalize(0.3, 0.5, 1)))` with `n = normalize(cross(ddx(p), ddy(p)))`. Background (0.13, 0.145, 0.16). Depth test on, blending off, no culling.

The 0.4 native projection lives in the retained MPUlt assembly and is not in repository source. This probe uses the repository's web-renderer projection above instead; the plan's mapping is `radius` -> `R` and `d4`, `face_shrink` -> `cs`, `sticker_shrink` -> `ss`, `angle` -> `zoom`. The side-by-side image against the 0.4 host is for the owner and is not a gate.

## 4. Turn animation and the W3 trace

- Turn data: `workload/turn.json` (format `magic600-sb-turn-v1`): `generator` 1 (H of cell 0, the largest moved-slot count, 4,600 slots), `plane_u`, `plane_v`, `angle` (pi), `moving_slots` (4,605 animated slots, the cap of cell 0), `move_src`/`move_dst` (the generator: `labels[dst[i]] = labels_before[src[i]]`), `inverse_src`/`inverse_dst` (the inverse), and `labels.revision_even_sha256` / `revision_odd_sha256` (SHA-256 of the 259,800 u32 LE labels of the solved state and of the state after one generator).
- Turn duration: `turn_ms = 190` by default (`web/renderer.js:104`, the only turn duration in the 0.4-era source; the 0.4 native host turns instantly). Command-line option; the value is written to `run.json`.
- Turns run back to back from the trace start `T0`: turn `t` (0-based) spans `[T0 + t*D, T0 + (t+1)*D)` on the QPC clock, `D` = turn duration in ticks. Even turns apply the generator, odd turns its inverse; the labels alternate between the solved state (even revisions) and the turned state (odd revisions).
- In a frame at time `now`: `t = floor((now - T0) / D)`, `phase = (now - T0 - t*D) / D` in [0, 1); `theta = sign * angle * phase*phase*(3 - 2*phase)` with sign +1 for even turns and -1 for odd turns.
- Label revision: the revision bound in turn `t` is `t` (revision = number of completed turns). The first frame of turn `t` (t >= 1) is the frame that adopts revision `t`: in that frame the CPU side applies the move of turn `t-1` to its authoritative label array (moves of turn `t-1`: generator if `t-1` is even, inverse if odd), writes all 259,800 labels to an upload buffer, copies them to a GPU buffer and binds it for that frame's draws.
- Label check copy: in that same command list, after the draws, `CopyBufferRegion` copies the label buffer actually bound for the draws (all 259,800 u32) into a readback slot. Record per copy: frame index, revision, the bound resource identity (an id the probe assigns to each GPU label buffer).
- W2 camera rotation: `rotate(0, 3, 0.002)` once per frame before drawing (`renderer.js:72`). It runs in W2 and W3.

## 5. Run directory (`magic600-renderer-run-v1`, read by `tools/perf/renderer_gate.py`)

- `run.json`: `format`, `run_id` (unique), `scene` (`w1`..`w4`), `candidate` `"s-b"`, `qpc_frequency`, `markers` {`trace_start_qpc` = T0, `trace_stop_qpc`}, `presentmon` {`process_id`, `swap_chain`: filled by the run script from the capture}, `build` {`build_identity`: SHA-256 of the probe executable plus the compiled shader blobs}, `turn_ms` (W3), `label_check` {`status` `pass`|`fail`, `copies`, `revisions`, `mismatches`, `late_adoptions`, `missing`, `oracle_sha256` {even, odd}} (W3, W4), `vram_peak_mb` (process local-segment usage peak from `IDXGIAdapter3::QueryVideoMemoryInfo`), `environment` {`power_source` `mains`|`battery`, `declared` {`frame_generation` false, `upscaling` false, plus the owner-confirmed conditions passed on the command line}, `display` {width, height} (current mode of the window's monitor), `backbuffer` {width, height}, `refresh_hz`, `vsync`, `tearing`, `adapter`, `driver`}.
- `trace.jsonl` (W3; also written for the other scenes): one JSON object per frame, `{"frame": n, "qpc": q, "turn": t, "phase": f, "revision": r}`. `qpc` is read after the previous `Present` returned and before this frame's `Present`. In W1, W2 and W4 `turn` and `phase` are `null`.
- `presentmon.csv`: PresentMon 2.6.0.0 with `--v1_metrics --qpc_time --process_id <pid>`, written by the run script, never by the probe.

## 6. Label check (plan section 2)

After the capture (untimed): compare every readback copy with the oracle labels of its revision (even: solved, odd: after one generator; both rebuilt from `turn.json` and verified against its SHA-256 digests at startup). The status is `fail` on any mismatch, a revision adopted during the trace without exactly one copy, a copy whose recorded binding differs from the buffer the frame was meant to bind, or a first use of a revision later than the frame after its upload. Fault injection options (owner-run negative tests, each must end in `fail` or in a `renderer_gate.py` refusal): `--inject corrupt-label`, `--inject swap-same-colour`, `--inject delay-adoption`, `--inject stale-binding`, each at a fixed turn.

## 7. Geometry check outputs (shared by the reference and the probe)

- Cameras: `cameras.json` (three Q matrices as rotate sequences from identity).
- Poses: start (`theta = 0`), midpoint (`theta = angle * 0.5`, smoothstep(0.5) = 0.5) and end (`theta = angle`), all with solved labels and the turn of `turn.json`.
- Sample: global vertex index `g = c*30480 + vi`; the sample is every `g` with `g % 4099 == 0`, plus the first vertex (`offsets[local]`) of every slot in `moving_slots`, as one sorted list without duplicates.
- Output per camera and pose: for each sampled `g` in order, `ndc_x`, `ndc_y`, `w` as f32 LE. Files `reference/c<k>_<pose>.f32` (`pose` in `start`, `mid`, `end`), `reference/sample.u32` (the sampled indices) and `reference/index.json` (parameters, counts, SHA-256 of every file).
- Per-cell draw count: 30,480 vertices for each of the 600 cells (from `offsets`); the probe counts vertex-shader invocations per instance in its geometry-check mode.
- Tolerance (probe against reference): `|probe - ref| <= 1e-4 + 1e-4 * |ref|` per component.
