# Implement: area-centroid shrink anchors in the S-B reference and probe

## 1. Goal and acceptance
- Goal: implement the anchor rule of `work/experiments/renderer-sb/SPEC.md` section 3 ("Shrink anchors" and step 1) in the S-B reference, its committed outputs, its source check and the S-B probe. The owner decided the rule on 10 October 2026 (`docs/wiki/decisions/owner-decisions-2026-10-10-anchor.md`).
- Acceptance check: `python -B work/experiments/renderer-sb/check_assets.py` exits 0 and prints the new anchor line.
- Also before finishing: `python work/experiments/renderer-sb/probe/check_probe.py` builds the probe and its `--selftest` passes, including the new anchor line. Leave no build directory behind.
- Out of scope: shaders, GPU code (`gpu.cpp`), the run scripts, `cameras.json`, `workload/`, `handoff/`, `SPEC.md`, results, and every file outside the allowed list.

## 2. Actual problem and reproduction
- The pipeline anchored the shrink at `assets/mesh_centers.f32`, the toolkit's numbering centres, which do not move with the sticker.
- Consequence for W3: after the full turn of `workload/turn.json`, the shrunk geometry of 4,120 of the 4,600 moved slots differs from the destination slot's by up to 0.0118 world units, compared by world-axis bounds.
- With the area-centroid anchors of SPEC section 3 the jump is at most 7.09e-8. That figure uses the f32 anchors; with float64 anchors it is 6.61e-8.
- `reference_geometry.py` already contains `sticker_anchors(offsets, vertices)`, the stdlib definition. Its output for the current assets has SHA-256 `0b6ead284d3f62719e3e6ec893d6c199d49698d246a59164bc362446e39faca6`, the pin in SPEC section 3. An independent NumPy computation gives the same bytes.

## 3. Environment and versions
- Windows 11, the system CPython 3.14 (stdlib only for these scripts).
- For the probe: MSVC, Windows SDK with DXC, CMake and Ninja, as recorded in `work/experiments/renderer-sb/probe/report.md`. `check_probe.py` finds them.

## 4. Necessary source and evidence
- `work/experiments/renderer-sb/SPEC.md` sections 2, 3 and 7.
- `work/experiments/renderer-sb/reference_geometry.py` (`sticker_anchors`, `build_reference`), `check_assets.py` and `reference/`.
- `work/experiments/renderer-sb/probe/src/probe.cpp` (`Assets::Assets`, `selftest`) and `probe.h`.
- Consumers that must keep working unchanged:
  - `probe/src/gpu.cpp` uploads `Assets::centers` as the anchor buffer;
  - `probe/shaders/geometry.hlsl` and `sort.hlsl` read it;
  - `work/experiments/renderer-sa2/native/` compiles `probe.cpp` and uses `sb::Assets` the same way;
  - `work/experiments/renderer-wj/reference_wj.py` imports `sticker_anchors`, `binary`, `matvec`, `camera_matrix`, `turn_vertex`, `project`, `PARAMETERS` and `ASPECT` from `reference_geometry.py`.

## 5. Attempts so far
- The integrator's independent NumPy check, with the same assets and on the same day, gave the figures in section 2. No code uses the new anchors yet.

## 6. Constraints and owned files
- Never change `assets/`, the model identity, labels, mechanical state or `SPEC.md`. Do not change the signature or the result of `sticker_anchors`, or the names that `reference_wj.py` imports.
- `reference_geometry.py`:
  - `build_reference` takes the anchors from `sticker_anchors(offsets, vertices)` and no longer reads `mesh_centers.f32`. Remove it from `ASSET_NAMES`, so `index.json` `inputs` no longer lists it.
  - Add a module constant `ANCHOR_SHA256` equal to the SPEC pin, and refuse (`ValueError`) when the anchors' f32 little-endian bytes have another digest.
  - `index.json`: `format` becomes `magic600-sb-reference-v2`. Add `"anchors": {"rule": "area-weighted triangle centroid, SPEC.md section 3", "sha256": <digest>}`. All other fields keep their meaning.
  - Then regenerate `reference/` with `python -B work/experiments/renderer-sb/reference_geometry.py`. `sample.u32` must stay byte-identical (the sample does not depend on the anchor).
- `check_assets.py`: add `check_anchors`, run after `check_turn` and before `check_reference`:
  - the anchors' digest equals `ANCHOR_SHA256`;
  - **W3 turn-end continuity**, in float64 with the stdlib, over all 4,605 animated slots. The pairs are every `(src, dst)` of `move_src`/`move_dst`, plus `(s, s)` for the five slots of `moving_slots` that are not in `move_src` (0, 6, 37, 119 and 122). Those five turn back onto themselves; with the old anchors they jump by up to 0.0144. Derive the five slots from `turn.json`, do not hard-code them. For each pair:
    - left: the shrunk world vertices of slot `src` (SPEC steps 1–3), turned by the full `angle` of `turn.json` (`turn_vertex`);
    - right: the shrunk world vertices of slot `dst`, not turned;
    - the error is the largest difference of per-axis minima and maxima over the sticker's vertices. Every error must be at most 2e-6. Triangulations of congruent stickers differ, so vertex-wise comparison is not valid.
  - a **control**: the same comparison with `mesh_centers.f32` as anchors must give at least one error above 2e-6. The check fails if it does not.
  - The line reports the maximum error with the anchors and with the control. Keep the whole script at most about 60 s on this machine.
- `probe.cpp` / `probe.h`:
  - `Assets::Assets` fills `centers` (keep the name and layout: 433 x 4 floats) from the SPEC rule instead of reading `mesh_centers.f32`. Use `double` arithmetic in exactly the SPEC order and one `float` rounding.
  - Refuse when a sticker has no finite positive area.
  - The manifest digest loop over all asset files stays.
  - Add a constant equal to the SPEC pin. `selftest` checks the SHA-256 of the 1,732 floats against it and prints `selftest: shrink anchors SHA-256: ok`.
  - Do not use fast-math options or change compiler flags.
- `probe/README.md`: one short paragraph on the anchor in place of "sticker centre".
- Generated output: only `reference/` changes, by the command above. Leave no `__pycache__`, build directory or other generated file in the worktree.

```implement-contract
{"allowed_files": ["work/experiments/renderer-sb/reference_geometry.py", "work/experiments/renderer-sb/check_assets.py", "work/experiments/renderer-sb/reference/*", "work/experiments/renderer-sb/probe/src/probe.cpp", "work/experiments/renderer-sb/probe/src/probe.h", "work/experiments/renderer-sb/probe/README.md"], "acceptance_check": ["python", "-B", "work/experiments/renderer-sb/check_assets.py"], "stop_condition": "check_assets.py passes with the anchor digest, the W3 turn-end continuity over all 4,605 animated slots within 2e-6 and the failing mesh_centers control; the regenerated reference matches build_reference byte for byte with sample.u32 unchanged; check_probe.py builds the probe and its selftest passes including the anchor digest line; no generated files remain outside reference/"}
```

## 7. Required return format
- A short report: the changed files, the new `check_assets.py` lines (continuity maximum and control maximum), the new `index.json` anchor digest, the `check_probe.py` output with tool versions, and anything not done.
