# Jumbling render data contract (`magic600-jumbling-render/1`)

Status: **framework contract of 9 October 2026, draft until the 2.5 interface freeze.** One data contract between the engine side and every drawing of the jumbling puzzle:
- the full-state research viewer (`research/jumbling/viewer/`, WebGL);
- the Direct3D 12 path of the renderer packet [E-2.4-0J](../../docs/progress/1.0/packets/renderer/E-2.4-0J-jumbling.md), which carries the rendering acceptance;
- the view model of the engineering packet [ENG-0J](../../docs/progress/1.0/packets/engineering/ENG-0J-jumbling-backend.md).

The interfaces do not fix the dimension; 1.0 implements and accepts d = 4 only (owner decision 6 of [9 October 2026](../../docs/wiki/decisions/owner-decisions-2026-10-09-jumbling.md)). The [state contract](state-contract.md) decides what a state is; this file only says how a state is handed to a drawing.

## 1. Rules

- **The engine is the source of truth.** Poses, lattice flags, grip status, certificates, digests and revisions come from the engine (J1 today). A drawing reads them; it never decides legality, never changes a state and never relabels a sticker.
- **Floats are for drawing only.** Every float in this contract is converted from an exact engine value. The exact value, or the digest that fixes it, travels with it.
- **One adoption per revision.** A drawing adopts the pose index, the pose table and the lattice data of one revision together, never a mix of two.
- **Dimension in every header.** Every header names `dimension` (d). A reader refuses a d it does not implement. In 1.0, every reader implements d = 4 only.

## 2. Static geometry (per model identity)

The sticker geometry of the retained model, unchanged for the jumbling puzzle (`assets/`, checked by `tools/perf/check_renderer_assets.py`):

| Item | d = 4 source | Content |
|---|---|---|
| facets F | 600 | boundary cells; a facet frame is a d × d matrix (`cell_frames.f32`, column-major) |
| base mesh | `mesh_vertices.f32`, `mesh.json` | the stickers of one facet as triangle lists, d-vectors in the base facet's frame |
| stickers per facet | 433 (`mesh_sticker.u32`, `mesh.json` `offsets`) | a sticker slot is `facet * 433 + local` |
| sticker to piece | `slot_piece.u32` | 259,800 slots onto 177,120 pieces; all stickers of a piece move together |
| facet normal and length | `mesh.json` (`normal`, `normal_length` R) | world coordinates are the asset frame divided by R |

Shrink (facet shrink `cs`, sticker shrink `ss`) is applied in the home frame of the sticker, before its piece's pose, so a piece stays rigid (`work/experiments/renderer-sb/SPEC.md` section 3, steps 1–3).

The shrink anchor of each base sticker is the area-weighted centroid of its triangles, computed from the mesh as SPEC.md section 3 defines it (owner decision, 10 October 2026: `docs/wiki/decisions/owner-decisions-2026-10-10-anchor.md`).
- This anchor moves with the sticker, so a piece's shrunk stickers land exactly on the shrunk stickers of the slots it reaches.
- `mesh_centers.f32` is not the anchor. Those numbering centres do not move with the sticker: under J1 generator 1 they put the shrunk geometry up to 0.0144 world units off the destination slot's.

## 3. State (per revision)

`header.json`:

| Field | Meaning |
|---|---|
| `format` | `magic600-jumbling-render/1` |
| `dimension` | d (4) |
| `pieces` | number of pieces (177,120) |
| `poses` | number of entries P in the pose table |
| `revision` | monotonic revision number of the engine state |
| `digest` | the engine's canonical state digest (J1 `State.digest`) |
| `model_identity`, `menu_identity`, `contract_revision` | as recorded in the engine journal |
| `files` | SHA-256, shape and type of each array below |

Arrays, little-endian:

| File | Type and shape | Meaning |
|---|---|---|
| `pose_index.i32` | int32 [pieces] | pose id of each piece; pose 0 is the identity |
| `poses.f32` | float32 [P][d][d] | pose matrices, row-major, acting on column vectors (x′ = M x) in the asset frame |
| `pose_lattice.i32` | int32 [P] | index of the pose in the retained symmetry group (K⁺ for d = 4), or −1 off the lattice |

A piece is on the lattice when `pose_lattice[pose_index[piece]] >= 0`. The pose table is the engine's canonical table, so equal states give equal arrays.

Drawing one sticker vertex of slot s with home position x (after shrink): `world = poses[pose_index[slot_piece[s]]] · x`, then the motion of section 4 if its piece moves, then the projection chain of section 6.

## 4. Motion

A twist (c, g) fixes the pole n_c, so it is a simple rotation: it fixes a (d − 2)-plane pointwise and turns the plane spanned by two orthonormal d-vectors u, v. The swept family is

R(φ) = I + (cos φ − 1)(uuᵀ + vvᵀ) + sin φ (vuᵀ − uvᵀ), with R(θ) = g and 0 < θ < π.

| Field | Meaning |
|---|---|
| `grip` | the pole c |
| `plane_u`, `plane_v` | u and v, float d-vectors |
| `angle` | θ in radians |
| `moving` | sorted piece ids of the certified inside set, with its SHA-256 |
| `from_revision`, `to_revision` | the revisions before and after |

During the motion, a moving piece is drawn at R(φ(t)) · poses[...] · x; the easing φ(t) belongs to the design track's motion table. A half-turn (θ = π) has no unique turning direction: the engine must name u and v for it, and until it does, a drawing shows a half-turn without a swept preview. Retained turns use the same family (SPEC.md section 3, step 4).

## 5. Overlays

| Field | Meaning |
|---|---|
| `grip_status` | one character per grip: `a` admissible, `b` blocked |
| `certificates` | for each blocked grip: the straddling piece and its two certificate points below and above the cut, as exact values and as float d-vectors in the asset frame |
| `cut` | for a grip c, the cut hyperplane n_c · x = α‖n‖², with α = 121/125 |

Certificate points can lie on no sticker, so they are drawn from these fields, never derived from meshes.

## 6. Projection chain

The view is an ordered list of stages, each with a `kind` and parameters. For d = 4 the chain is:

1. `rotate`: the d × d camera rotation Q (S-B: `rotate(a, b, t)` steps);
2. `perspective` (d → d − 1) with eye distance `d4` (S-B: 1.18), or `stereographic` (d → d − 1) from a named pole;
3. `camera3`: the three-dimensional camera of SPEC.md section 3, step 7.

A reader implements the stages it knows for its d and refuses a chain it cannot draw. A puzzle of higher dimension would add stages, one dimension each; nothing in 1.0 builds one.

## 7. Revisions and checks

- Revisions increase by one per committed engine change. A drawing that shows revision r has adopted all three arrays of r.
- The renderer packet's revision check (E-2.4-0J acceptance items 3 and 4) reads back the bound arrays in the first frame of each revision and compares them with the engine's by SHA-256 and element by element.
- `research/jumbling/fixtures/wj.py export` writes this format from the W-J fixtures; its headers carry the fixture's stage name and record count beside the fields above.

## 8. Scope

- Dimension-open: the headers, array shapes, motion fields and projection chain.
- Four-dimensional only in 1.0: shaders, meshes, the model, the tests and every acceptance.
- Nothing here is Windows, Direct3D 12 or performance evidence.
