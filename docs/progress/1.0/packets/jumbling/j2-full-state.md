# Packet: J2 full-state viewer on the render data contract (implementation)

Run as an implementation with the default reviewer model, effort `max` and speed tier `fast`, in the worktree the wrapper creates. Change only the allowed files.

## 1. Goal and acceptance
- Goal: the rendering framework of owner decision 6 (9 October 2026; `docs/wiki/decisions/owner-decisions-2026-10-09-jumbling.md`). Today the J2 viewer exports only the 4,375–5,799 pieces a short sequence moves. A scrambled jumbling state has never been drawn whole. Build a full-state page that draws every one of the 259,800 stickers of all 177,120 pieces of a W-J fixture state, reading only the render data contract (`research/jumbling/render-contract.md`) and the unchanged sticker assets. It uses the same geometry and data as the Direct3D 12 path of `docs/progress/1.0/packets/renderer/E-2.4-0J-jumbling.md`, so it can serve that path as a reference picture. It is a research page, not a renderer candidate, and it carries no performance claim.
- **Exporter** `research/jumbling/viewer/export_full.py`:
  - Geometry, once: `assets/mesh_vertices.f32`, `mesh_sticker.u32`, `mesh_centers.f32`, `cell_frames.f32`, `mesh.json` and `slot_piece.u32`, checked against `assets/manifest.json` (as `tools/perf/check_renderer_assets.py` does) and written unchanged with a `geometry.json` header naming their SHA-256.
  - States, per menu of `research/jumbling/fixtures/` (S4, I_a, I_b): `end` and `sweep-before`, written by `research/jumbling/fixtures/wj.py export` in the render contract format; and one `solved` state (`start`, the same for every menu).
  - Overlay, per menu: from the fixture, `grip_status`, the certificates of the blocked grips (piece and the two float points), and the swept twist (grip, `plane_u`, `plane_v`, angle in radians). The moving piece ids come from J1: classify the swept grip on the replayed `sweep-before` state, and the SHA-256 of the sorted ids must equal the fixture's `moved_sha256`. The overlay header carries the menu identity and the contract revision.
  - Output in `research/jumbling/viewer/full/`. Generated data is not committed: `full/.gitignore` ignores everything except itself. `--b64 <dir>` also writes every binary file as base64 text with the same name plus `.b64`, for hosts that serve text only.
  - `--menus` and `--stages` limit the work. Every replay checks the digest and the array hashes recorded in the fixture; any difference stops the export.
- **Page** `research/jumbling/viewer/full.html` with `full.js`, WebGL2 with three.js 0.160.0 from the same import map and pinned digests as `index.html`:
  - **Drawing.** One instanced draw of the 30,480 base vertices × 600 facets, as in `work/experiments/renderer-sb/SPEC.md` section 3. Per vertex: shrink in the home frame (steps 1–2), facet frame (step 3), then `world = poses[pose_index[slot_piece[slot]]] · world` (render contract section 3), then the swept rotation for moving pieces (section 4), then the projection chain (section 6): camera rotation Q, then perspective (`d4` = 1.18) or stereographic from a pole, then the three-dimensional camera with orbit controls. Data reaches the shader through integer and float data textures, not through per-vertex copies of state.
  - **Colour.** The S-B colour of the sticker's home label (facet class, SPEC.md section 3), with flat shading. An off-lattice tint can be switched on. All look values are provisional and belong to the design track.
  - **Controls.** Menu (S4₀, I_a, I_b); state (solved, end, before the swept twist); play and scrub of the swept twist, alternating the twist and its exact inverse; projection (perspective, stereographic); facet and sticker shrink; off-lattice tint; filters (all pieces, off-lattice only, on-lattice only, and isolate one piece); a list of blocked grips. Filters change only what is drawn.
  - **Certificates.** Selecting a blocked grip highlights its straddling piece and draws its two certificate points as markers placed by the same projection chain on the CPU, labelled below and above, with an "isolate" view of that piece alone.
  - **Panel.** Menu identity (shortened), contract revision, revision and digest of the state, pieces off the lattice (counted from the arrays), blocked grips, and the label "uncertified float drawing of an exact J1 state". No frame rate is shown.
  - **Integrity.** Every array is checked against its header SHA-256 (Web Crypto) before any GPU upload; a mismatch shows a load error and draws nothing. A header whose `dimension` is not 4 is refused with a message. The page reads the binary files, or the `.b64` files when given `?b64` or when binary loading fails.
  - **Page rules.** Light and dark themes from colour tokens, no horizontal scroll at 390 px, bare `#` anchors only (for example `#S4-end`), and no request to a host outside the allowlist of `index.html`.
- **Headless check** `research/jumbling/viewer/check_full.mjs` (Playwright, Chromium with SwiftShader, `--vendor DIR` as in `check_viewer.mjs`; the integrator runs it outside the sandbox):
  - no console or page errors on desktop light, desktop dark and phone width;
  - for each menu's `end` state: header counts equal the drawn instance counts; pieces off the lattice in the panel equal the count from the arrays and the fixture; the canvas has non-background pixels (`readPixels`) in both projections;
  - blocked grips listed equal the fixture's count; selecting the first places two markers and highlights the piece;
  - the swept twist plays and scrubs, and at its end the moving pieces sit at the `sweep-before` poses composed with the twist (checked on 16 sampled moving pieces against the fixture's `sweep.samples`, to float tolerance, through the same chain);
  - a tampered array and a header with `dimension` 5 are both refused without drawing;
  - the deep link `#I_b-end` restores menu and state;
  - no request leaves the allowlist.
- **README.** `research/jumbling/viewer/README.md` gets a section on the full-state page: what it draws, the data it reads, the commands, and its limits (no legality, no performance claim, look provisional).
- **Acceptance.**
  - The acceptance check below exits 0 inside the sandbox: it exports the S4 `end` state and the S4 overlay, with every digest, hash and moving-set check passing.
  - `node --check research/jumbling/viewer/full.js` passes.
  - The integrator then exports all menus and runs `check_full.mjs` outside the sandbox.
- Out of scope:
  - `research/jumbling/sim/`, `research/jumbling/fixtures/` and `assets/` (read only);
  - the existing witness and S4 scene page (`index.html`, `viewer.js`, `export_scene.py`, `check_viewer.mjs`) — leave them working and unchanged;
  - final look, colour and motion;
  - performance.

## 2. Actual problem and reproduction
- `python research/jumbling/viewer/export_scene.py --source sim-s4` exports 5,799 pieces; `scene.json` exports 4,375. The W-J fixtures hold states with 117,498 to 128,013 pieces off the lattice (`research/jumbling/fixtures/README.md`), which no page can show yet.
- `python research/jumbling/fixtures/wj.py export S4 end <dir>` writes the three state arrays in the render contract format (one J1 replay, a few minutes).

## 3. Environment and versions
- Branch `claude/jumbling-1-0`, committed head. Linux, Python 3.11 or later with NumPy, Node 22. three.js 0.160.0 through the import map; Playwright with headless Chromium may not run inside the sandbox, so the integrator runs the browser check.

## 4. Necessary source and evidence
- `research/jumbling/render-contract.md` (the contract this page implements).
- `work/experiments/renderer-sb/SPEC.md` sections 2–3 and `reference_geometry.py` (shrink, frames, projection, colour); the mesh frame equals J1's pole frame to within 1e-8, with R = `normal_length` = 4.5765.
- `research/jumbling/fixtures/wj.py` (`export`, `load`, `arrays`), `README.md` and `wj-*.json` (`stages`, `sweep`, `end_survey`).
- `research/jumbling/sim/` (`get_context`, `State.replay`, `classify`) and `TwistMenu.from_record`.
- `research/jumbling/viewer/index.html`, `viewer.js` and `check_viewer.mjs` for the import map, pinned digests, base64 loading, themes and the check style.
- `tools/perf/check_renderer_assets.py` for the asset digests and counts.

## 5. Attempts so far
| # | Step | Result |
|---|---|---|
| 1 | J2 witness and S4 scenes (`3487b02`, `82a8ab2`) | 105 of 105 browser assertions; partial states only |
| 2 | W-J fixtures and the render contract | full states available as arrays; no drawing yet |

## 6. Constraints and owned files
- Change only the allowed files below. Do not commit; the wrapper collects the patch. The page decides no legality and changes no state; every number it shows comes from the arrays, the fixture or J1.

```implement-contract
{"allowed_files": ["research/jumbling/viewer/export_full.py", "research/jumbling/viewer/full.html", "research/jumbling/viewer/full.js", "research/jumbling/viewer/check_full.mjs", "research/jumbling/viewer/README.md", "research/jumbling/viewer/full/.gitignore"], "acceptance_check": ["python", "research/jumbling/viewer/export_full.py", "--menus", "S4", "--stages", "end"], "stop_condition": "the exporter writes geometry, states and overlays with every check passing, the full-state page draws every sticker of a fixture state through the render contract with the controls, certificates and integrity checks above, the browser check is written, the README section exists, and the acceptance check passes"}
```

## 7. Required return format
- Changes only in the assigned worktree. The final message lists:
  - the changed files;
  - the shader inputs (textures and their layouts);
  - the acceptance output;
  - what the browser check covers;
  - open points.
