# Jumbling viewer (workstream J2)

Status: **research prototype, 9 October 2026.** It is the rendering and observation prototype of [the jumbling plan](../../../docs/progress/1.0/jumbling-plan.md) (section 1, J2). It is not a 1.0 renderer candidate. Look, colour and motion values are provisional and belong to the design track.

Evidence kind: source, fixture and synthetic geometry in a headless browser. Nothing here is Windows, Direct3D 12, input, long-session or performance evidence.

## What it shows

The scene selector offers **Witness E2–E4** and **J1: S4 sequence**. Both have five exact states and four applied twists, plus a certified rejected attempt.

The witness comes from [`witness.py`](../witness.py), as a local patch: the 4,375 pieces of the caps of grips c = 0 and d = 13, with their 6,499 stickers on 67 host cells:

| State | Reached by | Admissible / blocked grips (of 600) | Off-lattice pieces | Distinct poses |
| --- | --- | --- | --- | --- |
| S0 | solved | 600 / 0 | 0 | 1 |
| S1 | (c, g), jumble twist of 9.99987° | 546 / 54 | 3,097 | 2 |
| S2 | (d, T_d), retained third-turn | 535 / 65 | 3,097 | 3 |
| S3 | (d, T_d⁻¹) | 546 / 54 | 3,097 | 2 |
| S4 | (c, g⁻¹), solved again | 600 / 0 | 0 | 1 |

The second scene comes from the [J1 simulator](../sim/README.md), with its exact 24-element `TwistMenu.s4`. From solved, q is the Cayley quarter-turn with ω = (1, 0, 0) in pole 0's cap frame. Among grips whose caps meet cap 0 and whose poles q realigns, the exporter selects the largest exact n₀ · n_d, with ties to the smaller index. This gives d = 2, with q⁻¹n₂ = n₃₉. The first third-turn of A4₂ is element 2 (retained word [6]).

| State | Reached by | Admissible / blocked grips (of 600) | Off-lattice pieces | Distinct poses |
| --- | --- | --- | --- | --- |
| S0 | solved | 600 / 0 | 0 | 1 |
| S1 | (0, q), exact 90° quarter-turn | 584 / 16 | 3,097 | 2 |
| S2 | (2, a), retained 120° third-turn | 582 / 18 | 3,097 | 3 |
| S3 | (2, a⁻¹) | 584 / 16 | 3,097 | 2 |
| S4 | (0, q⁻¹), solved again by J1 digest | 600 / 0 | 0 | 1 |

At S2, the first blocked grip is 1. J1 rejects A4₁ element 1 (a retained half-turn, word [3]) and its full snapshot stays unchanged, including its journal. The recorded certificate uses piece 0, with h = −0.07061465 and +0.07061465. The exported patch is the union of the inside sets of all four applied twists: 5,799 pieces with 8,618 stickers on 87 host cells.

Every S4 pose, twist, lattice flag, grip status and straddle certificate comes from J1. Every exact state calls `State.survey()` for all 600 grips. J1 surveys return one certified straddling piece per blocked grip; the witness records all its straddling pieces. Highlights and the pager show the certificates the selected scene supplies.

The page has two views, visible together and linked, as the owner decided on 2 October 2026 (Global and Local only):

- **Global**: the patch projected from four dimensions. The 4D eye is outside the polytope along a chosen pole direction: between c and d, c, or d. You can set its distance. A stereographic option projects from the antipode of the same direction. Cell shrink pulls each sticker toward its host cell centre, and sticker shrink pulls it toward its own centroid; both are applied in the piece's home frame before the pose. Certificate markers use the posed region vertices before either shrink, in both projections. Grip markers sit at the projected poles: a ring means admissible, a diamond means blocked. A hatch pattern marks off-lattice pieces and moving preview pieces. A faint world-fixed cage of the host cells serves as the lattice reference.
- **Local**: the focus piece against one anchor hyperplane, in an orthographic projection along one direction inside that hyperplane. The vertical axis is therefore the exact offset from the hyperplane.
  - With a grip selected, the anchor is the grip's cut, and the vertical axis is h of the state contract. A straddling piece crosses the plane, and its two certificate points carry their h values.
  - Without a grip, the anchor is the facet of the piece's nearest lattice slot. The slot is drawn dashed as a realignment cue, and anything above the plane lies outside the 600-cell.
- **Linking**: clicking a grip marker or a piece in Global sets the Local focus. Clicking a neighbouring straddling piece in Local, or using the pager, moves the focus and its outline in Global. The selected grip's straddling pieces stay bright in Global while the others dim.
- **Timeline**: play or pause, step, and scrub along the sequence. Integer positions are the exact states. Between them, every moving piece follows the twist's own one-parameter family R(φ) = I + (cos φ − 1)(uuᵀ + vvᵀ) + sin φ (vuᵀ − uvᵀ), a rotation in the twist's plane {u, v} with the fixed plane unchanged. There is no matrix interpolation. The views are labelled "float preview, uncertified" there, and grip markers lose their status. A moving piece is described as "moving (float preview)", including retained turns with lattice poses at both endpoints. Lattice status is shown only at exact states.
- **Deep links**: all existing links, including `#s2-g0` and `#s1-g1-p7`, open the witness scene. The S4 scene adds the token `s4`: `#s4-s2-g1-p0` opens its rejected attempt and focuses its certificate piece. Links remain bare anchors of letters, digits and hyphens. A grip is accepted only if its pole is exported in the selected patch; other grip IDs are ignored.

## Regenerate and open

```text
python research/jumbling/viewer/export_scene.py          # about 3 minutes on four cores
python research/jumbling/viewer/export_scene.py --source sim-s4
node research/jumbling/viewer/check_viewer.mjs --serve   # then open http://127.0.0.1:8600/
```

- **Export.** `export_scene.py` needs the standard library and NumPy. Without arguments it writes the witness to `scene.json` and `scene.bin`. `--source sim-s4` writes `scene-s4.json` and `scene-s4.bin` beside them. Both sources read the retained model read-only; J1 also reads `assets/primitives.npz` to record its model identity.
  - `--workers N` sets the region pool.
  - `--cache FILE` (development only; keep the file outside the repository) pickles the exact source, so changes to the export step rerun in a second. Use a separate cache for each source; a source mismatch is refused.
- **Sources and checks.** `witness_source()` and `sim_s4_source()` return the same exact-data dictionary. S4 geometry uses the existing exact region/sticker builder. Every exported piece's vertex set must equal J1's `regions.vertices(p)` as an exact set, or export stops. Certificates are remapped from J1's vertex order to the builder's order and checked against their exact posed points and signed h. All 15 S4 header checks must be true: menu membership, exact realignment and cap intersection, certified surveys and applied moves, rejection without change, equality of the piece union and vertex sets, certificate agreement, reverse-state digests, journal identities and journal replay. The header records J1's full journal document, exact menu and deterministic script.
- **Serving.** The page needs HTTP, because it fetches its data; `file://` does not work. Use HTTPS or localhost HTTP so Web Crypto can verify the scene digest. `index.html` is written in the private-page format, without its own document skeleton. `--serve` wraps it in the same minimal skeleton that the host adds at publish time. A static server also works in that secure context, but then the page renders in quirks mode.
- **Libraries.** three.js 0.160.0 loads from cdn.jsdelivr.net through an import map. The fonts come from Google Fonts, with system fallbacks.

## Headless check

```text
PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers node research/jumbling/viewer/check_viewer.mjs --shots [--vendor DIR]
```

It uses the Node Playwright package and Chromium with SwiftShader WebGL (`--use-angle=swiftshader`). The integrator runs it outside the implementation sandbox. It runs the following assertions for **both scenes**:

- no console errors and no page errors, on desktop light, desktop dark and phone width;
- the model counts match the selected header, and its binary matches the byte count and digest;
- the S4 header checks pass, its journal names its exact menu and its state counts match J1's saved surveys;
- every animated twist family ends on the exported pose;
- for every exact state, the panel counts match the selected scene and the views are labelled exact; in between, they are labelled as an uncertified preview;
- a retained-turn piece is described as moving at S1.5, with lattice status restored at the exact endpoints (the witness keeps its piece-1035 regression);
- both views draw non-background pixels (read back with `readPixels`), in perspective and stereographic projection;
- all binary certificate rows have negative below h and positive above h;
- the Certificate row matches both vertex identities and signed h values of the selected source's rejected attempt, and reversing either sign fails the assertion;
- Global certificate markers keep their posed vertex positions and signed h at default shrink in both projections, and stay fixed when shrink changes (the witness keeps S1, grip 1, piece 7; S4 checks its rejected attempt);
- the two certificate points are labelled in Local and their marker heights keep the sign of h;
- clicking a grip marker in Global drives Local, and the pager cycles through the exported certificates;
- the scene selector loads the other scene's model and writes a bare anchor;
- play advances and pause holds;
- the dark theme applies;
- state/grip/piece deep links restore the correct scene, including the legacy `#s1-g1-p7` and the S4 token;
- deep links ignore grips without exported poles and load without errors;
- the binary and base64 paths verify the SHA-256 of the decoded scene bytes, and digest mismatches show a load error without building a model;
- there is no horizontal scroll at 390 px;
- no request goes to a host outside the allowlist.

`--vendor DIR` serves `three.module.js` and `OrbitControls.js` from a local directory when the browser cannot reach the CDN, as in a cloud session behind a TLS-inspecting proxy. The files must match the pinned SHA-256 digests in the script. Font requests get an empty stylesheet, so the screenshots use fallback fonts.

The S4 export acceptance and JavaScript syntax checks passed in the implementation sandbox. The integrator's run of the expanded browser assertions on 9 October 2026 (Linux cloud session, `--vendor` with the pinned three.js files, the committed scene files) passed 105 of 105 for both scenes. `--shots` adds S4 captures with the prefix `s4-`; that run took none.

Recorded run before the certificate, preview, digest and deep-link fixes: 32 of 32 checks passed in a Linux cloud session. That result does not cover the added regression assertions. Screenshots in `shots/` are from that run:

| File | Content |
| --- | --- |
| `1-solved.png` | S0, piece focus on the lattice |
| `2-after-g.png` | S1, the negative-control grip 1 with its certificate |
| `3-after-td-blocked-dark.png` | S2 in the dark theme: grip c blocked, 65 blocked grips, the panels |
| `4-local.png` | the Local view of S2, with piece 1 straddling the cut of c, at h = −0.13980 and +0.01344 |

## Data

`scene.json` (45 KB) is the header. `scene.bin` (1.66 MB) holds little-endian typed arrays at the offsets the header lists. With `index.html` and `viewer.js`, the page and its data come to about 1.8 MB.

`scene-s4.json` (about 523 KB) and `scene-s4.bin` (about 1.08 MB) use the same layout. The S4 header additionally keeps each J1 survey (certificate counts, exact evaluations and exact straddle fields), each state digest, the journal with model/menu/contract identities, the exact menu and the script. Mapped straddle records preserve J1's orbit, transport, original vertex indices, exact points and exact h triples alongside the display indices and float h values.

The loader checks the decoded byte count and SHA-256 against the header before constructing any arrays or building the model. A mismatch stops loading with an error.

- **Header** (`scene.json`):
  - counts, the layout of `scene.bin` and its SHA-256;
  - the patch grips with unit poles;
  - the host cells with their colour class, centre and tetrahedron;
  - the pose table, with each pose's float matrix, nearest K⁺ element and residual angles;
  - the twists, each with its label, grip, kind, angle, the plane {u, v} and the exact matrix as Q(√5) triples (a, b, d) for (a + b√5)/d;
  - for each state: the 600-character grip status string, the blocked grips, the off-lattice count, pieces per pose, the straddling-record count and the outer-shape statistics;
  - the rejected attempt (negative control);
  - consistency checks against `witness-results.json`.
- **Binary** (`scene.bin`):
  - piece ids and cap flags;
  - region vertices (float32, 4D, divided by the facet distance);
  - region edges;
  - stickers, with their piece, host and fan triangles of their ordered 2-faces;
  - per-state pose index and lattice flag for each piece;
  - every straddling certificate (state, grip, piece, the two vertex indices, and h below and above).

## Exact and float

| Exact (Q(√5), from `witness.py` or J1) | Float (display only) |
| --- | --- |
| piece regions, by double description | exported coordinates (float32 of exact vertices) |
| sticker vertex sets: region vertices exactly on a host facet | pose and twist matrices (float64 of exact matrices) |
| sticker 2-faces and region edges, from exact tight-constraint sets; every sticker passes V − E + F = 2 | twist plane {u, v} and angle; partial angles during animation |
| poses, twists and the pose of every piece at every state | nearest lattice pose, by a float search over the 7,200 elements of K⁺ |
| grip status and every straddling certificate (exact signs) | host-cell colouring, cell cage, projections and shrink |
| agreement with `witness-results.json` (blocked sets and negative control) | outer-shape statistics; h ranges in the focus panel |
| S4 vertex-set agreement with J1, signed certificate points, unchanged rejected attempt and state/journal digests | |

Legality is never decided in the browser. Certificates are displayed with float h values; their signs are exact.

## Limits

- **Patch only.** The witness draws 4,375 pieces of caps 0 and 13, and S4 draws the 5,799 pieces in its applied inside sets. Grip counts cover all 600 grips; markers cover the selected patch's 67 or 87 signature grips.
- **Two fixed sequences.** The witness and the certified J1 S4 script are available. There is no interactive jumble twist with a live preview.
- **Lossy projection.** The Local view drops one direction of the anchor hyperplane. Heights against the anchor are exact; the two other axes are a principal-axis projection of the focus piece.
- **Untested surfaces.** The page was exercised only in headless Chromium with software WebGL. Input on touch devices and timing were not assessed.

## Observations for the renderer packet (plan section 3)

1. **Per-piece transforms.**
   - The witness needs only a small pose table and a per-piece index: 1, 2, 3, 2 and 1 distinct poses at S0–S4.
   - A general jumbled state can give every piece its own pose, so the renderer needs a per-piece rigid 4D transform for up to 177,120 pieces. A float32 4 × 4 matrix is 64 bytes, about 11.3 MB in all; a unit-quaternion pair is 32 bytes, about 5.7 MB.
   - During a twist, only the moving set changes: 3,097 pieces for every twist here, one cap.
2. **Off-lattice counts.** S0 to S4 have 0, 3,097, 3,097, 3,097 and 0 off-lattice pieces.
   - The twist (d, T_d) moves 1,819 off-lattice pieces and 1,278 lattice pieces.
   - Those 1,278 lattice pieces still pass through off-lattice poses during the animation. Every animated twist therefore leaves the slot grid for its whole moving cap, retained twists included.
3. **Shrink and culling must travel with the piece.**
   - Cell shrink and sticker shrink are applied in the piece's home frame, before the pose. By linearity, this equals shrinking about the posed centres. Shrinking about world-fixed slot or cell centres would deform off-lattice stickers.
   - 4D back-face culling must use each sticker's posed facet normal, not its slot's cell normal.
4. **Outer shape.**
   - After the 10° twist, 3,789 of the 6,499 patch stickers have a vertex outside the 600-cell. The largest protrusion is 0.0225 facet distances, against a cap depth of 0.032. This is a float statistic; the same holds at S2 and S3.
   - Stickers of moved pieces leave their facet hyperplanes, so 3D projections of neighbouring cells can interpenetrate.
   - The world-fixed cage was needed to see the change at all.
5. **Certificates need more than sticker meshes.**
   - 16,938 of the 20,136 straddling certificates at S1 use at least one region vertex that lies on no sticker. Those are interior vertices on cut hyperplanes only, and 624 certificates use two of them.
   - Showing why a grip is blocked therefore needs the certificate points from the engine, or the full region vertices, not only the sticker geometry. The patch has 4,473 such interior vertices.
6. **Straddling sets are large.**
   - A blocked grip has 181 to 827 straddling pieces at S1 (median 345), and 37 to 690 at S2 (median 286).
   - Highlighting the set only reads when the other pieces dim. One focus piece with a pager was needed to make a single certificate legible.
7. **Status exists only at exact states.**
   - Grip status is undefined during motion, so the markers turn neutral.
   - Both views carry an exact or preview badge.
   - Any renderer preview of a twist angle needs the same separation from certified results.
8. **Overlays that were needed:**
   - grip markers with status by shape and colour, and a halo on the selected grip;
   - the straddling-set highlight with dimming of the rest;
   - the two certificate points with h labels;
   - the Local anchor plane with an exact vertical offset;
   - the dashed nearest-lattice ghost as the realignment cue;
   - the world-fixed cell cage;
   - a hatch pattern for off-lattice pieces;
   - the exact or preview badge.
9. **Projection.**
   - From an eye at 3 facet distances between c and d, all 67 patch cells face the eye, so culling removes nothing by default.
   - Global perspective cannot show on which side of a cut a piece lies. The Local orthographic frame, which keeps h exact on one axis, was the clearest view of a blocking reason that this prototype found.
10. **Picking** needs a triangle → sticker → piece map, must skip hidden or culled stickers, and gives grip markers priority within a pixel radius.

## Publishing on hosts without a binary type

Some page hosts serve no binary media type. For them, publish each scene's binary bytes as base64 text, and in the corresponding published header set `bin.file` to that text file and `bin.encoding` to `"base64"`. `viewer.js` decodes it, then checks the byte count and SHA-256 of the decoded bytes against that scene's exported header values.

## Review

| Call | Kind | Result |
| --- | --- | --- |
| `20261009T195031Z-2a42a035` | Routine review of the prototype | V1 and V2 major; V3, V4 and V5 minor. All adopted |
| `20261009T201016Z-d7b09cf4` | Implementation of the five fixes, integrated as `3487b02` after the integrator reviewed the patch | Headless check 43/43 |
| `20261009T202151Z-5320d9c6` | Scoped verification by the senior reviewer | Pass: V1 and V2 fixed, no new blocker or major |
