# Plan check: area-centroid shrink anchors (owner decision, 10 October 2026)

Review only; do not perform follow-up work. Read only.

## 1. Goal and acceptance
- **Goal.** Before two parallel implement calls start, check the plan that carries out the owner's decision `docs/wiki/decisions/owner-decisions-2026-10-10-anchor.md`. The sticker shrink anchor becomes the area-weighted centroid of each base sticker's triangles; no asset and no model identity changes. The plan consists of:
  - the new anchor rule in `work/experiments/renderer-sb/SPEC.md` section 3 ("Shrink anchors" and step 1) and section 2, from the diff against `HEAD`;
  - the matching text in `research/jumbling/render-contract.md` section 2 (diff against `HEAD`);
  - the stdlib definition `sticker_anchors` in `work/experiments/renderer-sb/reference_geometry.py` (diff against `HEAD`);
  - two implement packets, `work/experiments/renderer-wj-packets/anchor-sb.md` and `anchor-wj.md`;
  - the integrator's own part and the evidence plan in section 6 below.
- **Acceptance.** A result per `schemas/review-result.schema.json`. A finding is `blocker` or `major` only when one of these holds:
  - the rule is ambiguous enough that two correct implementations could produce different anchor bytes, or it can refuse valid assets;
  - the plan misses a consumer whose geometry would then disagree with SPEC;
  - a check in the plan cannot fail when the anchors are wrong;
  - the packets conflict, or one breaks an existing check;
  - the plan claims evidence it does not produce, especially Windows/DirectX or performance evidence;
  - the re-acceptance scope is smaller than E-2.4-0J item 6 requires.
- **Out of scope.** The owner's choice of option A itself, the W-J pose method, the jumbling theory, Look Lab (`tools/looklab/`, owned by another session, which will follow), the 0.4 code (`web/`, `server.py`, `build_assets.py`, `magic600-04/`), and wording.

## 2. Actual problem and reproduction
- The pipeline anchored each sticker's shrink at `assets/mesh_centers.f32`, the retained toolkit's numbering centres. They do not move with the sticker under the puzzle's symmetries. The raw sticker meshes do (bounds agree to 1.07e-7).
- Two consequences, both computed by the integrator from the unchanged assets in the engine environment on 10 October 2026:
  - **W-J:** J1 generator 1 gives a lattice disagreement of up to 0.0144 world units in 4,125 of 4,605 moved stickers. This fails E-2.4-0J acceptance item 2.
  - **S-B W3:** each turn ends with a jump of up to 0.0118 in 4,120 of 4,600 moved slots.
- With the area-centroid anchors, the maxima fall to 7.73e-8 and 6.61e-8 (7.09e-8 with the f32-rounded anchors), with no slot above 2e-6.
- `sticker_anchors` (stdlib) and an independent NumPy computation give byte-identical f32 anchors, SHA-256 `0b6ead28…faca6`, now pinned in SPEC. The largest anchor change against `mesh_centers.f32` is 0.239 asset units.

## 3. Environment and versions
- Windows 11 (build 26200). The engine Python `tools/.venv/engine` (CPython 3.14.7, NumPy 2.3.5) and the system CPython 3.14 without NumPy.
- MSVC, the Windows SDK with DXC, CMake and Ninja for the probes.
- Branch `claude/wj-area-centroid` from `main` `afe5f85`.

## 4. Necessary source and evidence
- The diffs against `HEAD` of `work/experiments/renderer-sb/SPEC.md`, `research/jumbling/render-contract.md` and `work/experiments/renderer-sb/reference_geometry.py`.
- The two implement packets, and `docs/progress/1.0/packets/renderer/E-2.4-0J-jumbling.md` (acceptance items, section 4 "Pipeline change", section 6).
- The anchor consumers:
  - **S-B:** `work/experiments/renderer-sb/probe/src/probe.cpp` (`Assets::Assets`), `gpu.cpp` (uploads `centers`), `probe/shaders/geometry.hlsl` and `sort.hlsl`, `check_assets.py`, `reference/index.json`;
  - **S-A2 (also used by the Qt S-D scene):** `work/experiments/renderer-sa2/native/CMakeLists.txt`, which compiles the S-B `probe.cpp`; `src/scene.cpp` (`put(assets.centers)`, `geometry_check` reads the S-B reference); `src/selftest.cpp:243`;
  - **W-J:** `work/experiments/renderer-wj/probe.cpp`, `gpu.cpp`, `geometry.hlsl`, `lattice.hlsl`, `reference_wj.py`, `lattice_wj.py`, `check_lattice.py`, `check_wj.py`;
  - **J2 viewer:** `research/jumbling/viewer/full.js` (uploads `mesh_centers.f32` as `uCenters`; the shrink is at lines 243–246), `export_full.py`, `README.md`.
- The integrator's scripts for the figures in section 2 are not in the repository. The packets' checks reproduce them.

## 5. Attempts so far
| # | Hypothesis | Change | Verification | Result |
|---|---|---|---|---|
| 1 | The W-J probe with the fixed home shrink agrees with S-B slot geometry | call `20261010T024836Z-a9b1fb0b` | `check_lattice.py` | fails, 0.0144; queued to the owner |
| 2 | The twist, not the anchor, is at fault | integrator check of the raw mesh bounds | NumPy | the raw meshes agree to 1.07e-7, so the anchor is at fault |
| 3 | A covariant anchor from the unchanged mesh removes both defects | area-weighted triangle centroid | NumPy, W-J lattice and S-B W3 turn end | 7.73e-8 and 6.61e-8; owner chose this |

## 6. The plan
1. **Interface commit (integrator).** The SPEC and render-contract text, `sticker_anchors` and the decision page, committed before the calls so both worktrees see them.
2. **Two parallel Codex implement calls**, standard tier, with disjoint allowed files:
   - `anchor-sb.md`: S-B reference, outputs, source check and probe;
   - `anchor-wj.md`: W-J probe, reference, outputs and lattice check.
   W-J imports only `sticker_anchors` and the existing helpers from `reference_geometry.py`. Its references pin no digest of that file.
3. **Integrator, in parallel:**
   - the J2 viewer computes the anchors on load from `mesh_vertices.f32` and `mesh.json` `offsets`, in JavaScript doubles in the SPEC order and rounded through a `Float32Array`. It refuses to draw unless their SHA-256 equals the SPEC pin, and uses them for `uCenters`. `mesh_centers.f32` stays among the checked geometry files but is no longer drawn with. `check_full.mjs` needs network access and Chromium, which this machine's checks do not have. Its check is `node --check` plus a Node script that runs the same function on the assets and compares the digest. The full headless check is left to a Linux session and recorded as not run.
   - Notes in E-2.4-0J (sections 4 and 6, dated), the S-B and W-J result cards, the wiki (`s-b-probe-results`, the decision page, the index and the log) and the brief.
4. **Integration.** Claude reviews and applies both patches and runs:
   - `check_assets.py`, `probe/check_probe.py`, `check_lattice.py` and `check_wj.py`;
   - the S-A2 and S-D `check_project.py`, `tests/test_renderer_gate.py`;
   - the wiki lint and its test.
   Then an Astra full review of the merged candidate in two concurrent shards (S-B and S-A2 consumers; W-J and the J2 viewer), and at most two scoped verification rounds.
5. **After merge.** The Qt and Godot level 2 builds are prepared again at the merged head, because the S-A2 library compiles the S-B `probe.cpp`.
6. **Owner-attended GPU session**, not in this candidate:
   - S-B: the GPU `--geometry-check` against the new reference, then three cold W3 runs of the new build (E-2.4-0J item 6, judged as before);
   - S-A2 and S-D: three cold W3 runs each under the guard, with the new library. These were planned anyway; their `geometry_check` uses the new reference;
   - W-J: the GPU lattice check, the W-J runs, and three cold W3 runs of the changed W-J executable (`run_scene.ps1 -Scene w3 -Runs 3`, unchanged judge, the same executable and shader identity as its W-J runs). Its label and pose paths share geometry, bindings and uploads, so the S-B runs cannot stand in for them (plan check `20261010T053301Z-f2cab142`, ANCHOR-PLAN-1).

   Until those runs, the claims are source and fixture evidence only. The 3 and 4 October W3 records keep their build identities and results; they describe the old anchor.

Questions for the reviewer:
- Q1: Is the f32-rounded anchor with a pinned digest the right normative form, or should the float64 references keep float64 anchors?
- Q2: Is the world-axis bound comparison within 2e-6 an adequate continuity criterion for differently triangulated congruent stickers? Is the `mesh_centers.f32` control strong enough to show the check can fail?
- Q3: Is any consumer, check or record missing from section 4 or section 6, in particular the S-A2/S-D level 2 checks and any pinned reference digests?
- Q4: Is the re-acceptance scope of step 6 sufficient for E-2.4-0J item 6?

## 7. Required return format
- Findings per `schemas/review-result.schema.json`, with an answer to each of Q1–Q4 in the summary. Do not propose new scope.
