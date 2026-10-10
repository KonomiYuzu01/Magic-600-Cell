# Packet: review of the J2 jumbling viewer prototype (routine reviewer, review kind)

Run read-only as a review with the default reviewer model, effort `max` and speed tier `fast`. Review only; do not perform follow-up work.

## 1. Goal and acceptance
- Goal: check that the J2 viewer (`research/jumbling/viewer/`) shows the exact witness faithfully:
  - every displayed legality status and certificate comes from the exact computation;
  - float data is labelled as such;
  - animation follows each twist's own one-parameter family;
  - the page keeps the private-page rules stated in its README;
  - the headless check tests what it claims.
- Acceptance: a schema-valid result. Every `blocker` or `major` finding names the line and gives a counterexample or a failing input.
- Out of scope:
  - look, colour and motion choices, which are provisional and belong to the design track;
  - the exact witness itself (`witness.py`, already reviewed);
  - `research/jumbling/sim/`;
  - any performance, Windows or Direct3D claim.

## 2. Actual problem and reproduction
- `python research/jumbling/viewer/export_scene.py` takes about 3 minutes. It regenerates `scene.json` and `scene.bin` from `research/jumbling/witness.py`.
- `node research/jumbling/viewer/check_viewer.mjs --vendor <dir>` runs the headless Playwright check. `<dir>` holds three.js 0.160.0 `three.module.js` and `OrbitControls.js`.
- Claims to check, in `research/jumbling/viewer/README.md`:
  - the state table (S0–S4 grip counts and off-lattice counts);
  - "what is exact and what is float";
  - the renderer observations.

## 3. Environment and versions
- Branch `claude/jumbling-1-0`, current head. Linux, CPython 3, NumPy, Node 22, Playwright with headless Chromium on SwiftShader. Evidence kind: source and synthetic geometry in a headless browser.

## 4. Necessary source and evidence
- `research/jumbling/viewer/export_scene.py`. Points to check:
  - sticker extraction (region vertices exactly on a host facet; face ordering);
  - the pose table;
  - the survey and its certificates;
  - the float conversion;
  - the claim that states 3 and 4 equal states 1 and 0 exactly.
- `research/jumbling/viewer/viewer.js`. Points to check:
  - loading, including the base64 path;
  - the projection;
  - the twist-plane animation;
  - the exact and preview badges;
  - grip status shown only at exact states;
  - picking and linking.
- `research/jumbling/viewer/index.html` and `check_viewer.mjs`.
- `research/jumbling/viewer/README.md`.
- `research/jumbling/state-contract.md` sections 2–3, for the definitions the viewer displays.

## 5. Attempts so far

| # | Step | Result |
|---|---|---|
| 1 | Build and headless check | 32 of 32 checks pass, stable on a second run |
| 2 | Base64 scene path added for publishing | Headless check passes again |

## 6. Constraints and owned files
- Read-only review; no files may change.

## 7. Required return format
- JSON matching `schemas/review-result.schema.json`, with finding IDs prefixed `V`.
- Review only; do not perform follow-up work.
