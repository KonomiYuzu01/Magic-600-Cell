"""Exact exposed-face mesh counts for every retained K+ orbit representative.

All 600 cut and 600 facet hyperplanes are scanned in exact integer form.
Floating point is used only for an in-memory triangulation and comparison
with the retained float32 sticker asset. This is a size lead, not a product
mesh asset or renderer performance evidence.
"""

import sys

sys.dont_write_bytecode = True

import argparse
import json
from pathlib import Path
from time import perf_counter

ROOT = Path(__file__).resolve().parents[4]
JUMBLING = ROOT / "research" / "jumbling"
if str(JUMBLING) not in sys.path:
    sys.path.insert(0, str(JUMBLING))

import numpy as np
import sim
from exact import Q5, affine_rank, dot
from differential.leads import heights

EVIDENCE_KIND = "exact geometry of the retained model and retained float32 asset comparison; source evidence, not renderer performance evidence"
BYTES_PER_TRIANGLE = 3 * 4 * 4  # Three corners, four float32 coordinates each.


def _tight_sets(ctx, orbit):
    """All tight vertex sets; object arrays ensure unbounded integer arithmetic.

    n.v = (p + q sqrt(5))/(2*denominator), from J1's exact pole/vertex
    forms. Equality requires both numerator coefficients to vanish.
    """
    denominator, rows = ctx.regions.rep_vform[orbit]
    a = np.array(rows, dtype=object)
    n1, n2 = ctx.data.N1.astype(object), ctx.data.N2.astype(object)
    p = a[:, :4] @ n1.T + 5 * (a[:, 4:] @ n2.T)
    q = a[:, :4] @ n2.T + a[:, 4:] @ n1.T
    result = []
    for kind, offset in (("cut", ctx.data.KAPPA), ("host", ctx.data.NN)):
        tight = ((p * offset.d == 2 * denominator * offset.a)
                 & (q * offset.d == 2 * denominator * offset.b))
        for pole in range(600):
            ids = frozenset(np.flatnonzero(tight[:, pole]).tolist())
            if ids:
                result.append((kind, pole, ids))
    return result


def orbit_faces(ctx):
    """Exact 3-faces, polygon vertex sets, and checked Euler counts per orbit.

    Vertex ids refer to the exact representative vertices in the same row.
    Polygon intersections are deduplicated by their complete vertex sets, so
    redundant tight hyperplanes cannot inflate the face or triangle counts.
    """
    result = []
    regions = ctx.regions
    for orbit, piece in enumerate(regions.reps):
        vertices = regions.rep_vertices[orbit]
        tight = _tight_sets(ctx, orbit)
        ranks = {}

        def dimension(ids):
            if ids not in ranks:
                ranks[ids] = affine_rank([vertices[i] for i in sorted(ids)])
            return ranks[ids]

        cuts, hosts = [], []
        signature = ctx.data.signature(piece)
        for kind, pole, ids in tight:
            if len(ids) < 4 or dimension(ids) != 3:
                continue
            polygons = set()
            for _, _, other in tight:
                intersection = ids & other
                if len(intersection) >= 3 and intersection != ids and dimension(intersection) == 2:
                    polygons.add(intersection)
            faces2 = [{"vertex_ids": sorted(s), "V": len(s)}
                      for s in sorted(polygons, key=lambda s: tuple(sorted(s)))]
            V, F = len(ids), len(faces2)
            edge_twice = sum(f["V"] for f in faces2)
            triangles = sum(f["V"] - 2 for f in faces2)
            if edge_twice % 2 or V - edge_twice // 2 + F != 2 or triangles != 2 * V - 4:
                raise AssertionError(f"orbit {orbit}, {kind} {pole}: Euler check failed")
            face = {"pole": pole, "side": ("in" if pole in signature else "out") if kind == "cut" else "host",
                    "vertex_ids": sorted(ids), "V": V, "E": edge_twice // 2,
                    "F": F, "faces2": faces2, "triangles": triangles,
                    "euler": V - edge_twice // 2 + F, "euler_ok": True}
            (cuts if kind == "cut" else hosts).append(face)
        got_hosts = sorted(f["pole"] for f in hosts)
        if got_hosts != sorted(regions.rep_hosts[orbit]):
            raise AssertionError(f"orbit {orbit}: exact host faces differ from rep_hosts")
        result.append({"orbit": orbit, "piece": int(piece), "vertices": vertices,
                       "cut_faces": cuts, "host_faces": hosts,
                       "hosts_match": True})
    return result


def _counts(faces):
    return {"faces3": len(faces), "vertices": sum(f["V"] for f in faces),
            "faces2": sum(f["F"] for f in faces),
            "triangles": sum(f["triangles"] for f in faces)}


def _add(total, counts, factor=1):
    for key, value in counts.items():
        total[key] = total.get(key, 0) + value * factor


def _mesh_size(cut, host):
    return {"cut_triangles": cut, "cut_bytes": cut * BYTES_PER_TRIANGLE,
            "host_triangles": host, "host_bytes": host * BYTES_PER_TRIANGLE,
            "all_triangles": cut + host, "all_bytes": (cut + host) * BYTES_PER_TRIANGLE}


def summary(faces, sizes):
    """One mesh per representative and the full size, weighted by orbit size.

    Vertices and 2-faces are counted per 3-face, as a triangle-soup mesh uses
    them. No cross-face welding or pose/state-dependent visibility is assumed.
    """
    sizes = [int(s) for s in sizes]
    if len(faces) != len(sizes) or sum(sizes) != 177120:
        raise ValueError("orbit sizes must cover all 177,120 pieces exactly")
    base = {k: {} for k in ("cut", "in", "out", "host")}
    full = {k: {} for k in base}
    orbits = []
    for row, size in zip(faces, sizes):
        counts = {"cut": _counts(row["cut_faces"]), "host": _counts(row["host_faces"])}
        for side in ("in", "out"):
            counts[side] = _counts([f for f in row["cut_faces"] if f["side"] == side])
        weighted = {}
        for key, value in counts.items():
            _add(base[key], value)
            _add(full[key], value, size)
            weighted[key] = {k: v * size for k, v in value.items()}
        orbits.append({"orbit": row["orbit"], "piece": row["piece"], "size": size,
                       "base": counts, "full": weighted,
                       "base_mesh": _mesh_size(counts["cut"]["triangles"], counts["host"]["triangles"]),
                       "full_mesh": _mesh_size(weighted["cut"]["triangles"], weighted["host"]["triangles"])})
    retained = json.loads((ROOT / "assets/mesh.json").read_text(encoding="utf-8"))
    retained_base = retained["base_vertices"] // 3
    return {"representatives": len(faces), "piece_count": sum(sizes), "orbit_sizes": sizes,
            "orbits": orbits, "base": base, "full": full,
            "base_mesh": _mesh_size(base["cut"]["triangles"], base["host"]["triangles"]),
            "full_mesh": _mesh_size(full["cut"]["triangles"], full["host"]["triangles"]),
            "retained_stickers": {"base_stickers": retained["base_stickers"],
                                  "base_triangles": retained_base,
                                  "base_bytes": retained_base * BYTES_PER_TRIANGLE,
                                  "full_triangles": retained["full_triangles"],
                                  "full_bytes": retained["full_triangles"] * BYTES_PER_TRIANGLE}}


def sticker_comparison(ctx):
    """Compare every cell-0 sticker against its exact host face.

    Report the requested 1e-9 distance to exact vertices literally, AND after
    rounding the exact vertices to the asset's float32 storage. The literal
    comparison also detects ordinary float32 conversion error (about 1e-7),
    which must not be mistaken for extra geometric points.
    """
    metadata = json.loads((ROOT / "assets/mesh.json").read_text(encoding="utf-8"))
    points = np.fromfile(ROOT / "assets/mesh_vertices.f32", dtype="<f4").reshape(-1, 4)
    slots = np.fromfile(ROOT / "assets/slot_piece.u32", dtype="<u4")
    if not np.array_equal(slots, ctx.data.slot_piece):
        raise AssertionError("slot_piece.u32 differs from the J1 model")
    if len(points) != metadata["base_vertices"] or metadata["base_stickers"] != 433:
        raise AssertionError("unexpected retained mesh dimensions")
    rows = []
    for sticker in range(433):
        piece = int(slots[sticker])
        vertices = [v for v in ctx.regions.vertices(piece)
                    if dot(ctx.data.N[0], v) == ctx.data.NN]
        if affine_rank(vertices) != 3:
            raise AssertionError(f"sticker {sticker}: host face is not three-dimensional")
        exact = np.array([[float(x) for x in v] for v in vertices])
        begin, end = metadata["offsets"][sticker:sticker + 2]
        retained = points[begin:end].astype(np.float64)
        unique = np.unique(retained, axis=0)

        def not_vertices(test_points, reference):
            distances = np.linalg.norm(test_points[:, None, :] - reference[None, :, :], axis=2).min(axis=1)
            return int((distances > 1e-9).sum())

        stored_exact = exact.astype("<f4").astype(np.float64)
        # Distinct stored points within 1e-9 of one float32-rounded exact vertex (an exact zero
        # coordinate stored as a tiny nonzero value): each one is an extra hull point.
        distances = np.linalg.norm(unique[:, None, :] - stored_exact[None, :, :], axis=2)
        close = distances.min(axis=1) <= 1e-9
        near_coincident = int(close.sum()) - len(set(distances.argmin(axis=1)[close].tolist()))
        triangles = retained.reshape(-1, 3, 4)
        collapsed = sum(len({tuple(p) for p in tri}) < 3 for tri in triangles)
        canonical_triangles = {tuple(sorted(tuple(p) for p in tri)) for tri in triangles}
        exact_triangles = 2 * len(vertices) - 4
        rows.append({"sticker": sticker, "piece": piece, "exact_vertices": len(vertices),
                     "exact_triangles": exact_triangles, "retained_triangles": len(triangles),
                     "ratio": len(triangles) / exact_triangles,
                     "retained_corners": len(retained), "retained_unique_points": len(unique),
                     "corners_not_exact_vertices_1e9": not_vertices(retained, exact),
                     "unique_points_not_exact_vertices_1e9": not_vertices(unique, exact),
                     "corners_not_float32_exact_vertices_1e9": not_vertices(retained, stored_exact),
                     "unique_points_not_float32_exact_vertices_1e9": not_vertices(unique, stored_exact),
                     "near_coincident_points_float32": near_coincident,
                     "collapsed_triangles_float32": collapsed,
                     "duplicate_triangles_float32": len(triangles) - len(canonical_triangles)})
    additive = ("exact_triangles", "retained_triangles", "retained_corners", "retained_unique_points",
                "corners_not_exact_vertices_1e9", "unique_points_not_exact_vertices_1e9",
                "corners_not_float32_exact_vertices_1e9", "unique_points_not_float32_exact_vertices_1e9",
                "near_coincident_points_float32", "collapsed_triangles_float32", "duplicate_triangles_float32")
    totals = {key: sum(row[key] for row in rows) for key in additive}
    # Extra triangles that none of the reported counts accounts for: every corner is a
    # float32-rounded exact vertex, no two stored points share one, and no triangle is
    # collapsed or repeated.
    unattributed = [row["sticker"] for row in rows
                    if row["retained_triangles"] > row["exact_triangles"]
                    and not (row["corners_not_float32_exact_vertices_1e9"] or row["near_coincident_points_float32"]
                             or row["collapsed_triangles_float32"] or row["duplicate_triangles_float32"])]
    return {"stickers": rows, "sticker_count": len(rows),
            "differing_stickers": sum(row["exact_triangles"] != row["retained_triangles"] for row in rows),
            "largest_ratio": max(row["ratio"] for row in rows), "totals": totals,
            "unattributed_stickers": unattributed,
            "explanation": "build_assets.py uses ConvexHull of floating region points. The retained triangle soup contains extra points besides the exact host vertices, repeated triangles and triangles whose corners collapse at float32 precision. It is not a minimal triangulation of the exact host polyhedron. Raw 1e-9 vertex matching also counts float32 rounding error; the separately reported float32-rounded reference separates that error from extra stored points. Counts do not reconstruct the unavailable pre-float32 hull inputs. Near-coincident points are distinct stored points within 1e-9 of one float32-rounded exact vertex (an exact zero coordinate stored as a tiny nonzero value); each adds hull points and triangles. unattributed_stickers lists the stickers with more triangles than the exact count that none of these counts explains."}


def build_base_mesh(faces):
    """Fan-triangulate each exact cut-face polygon, sorting by angle in its plane.

    The returned float32 4D triangle soup has per-3-face offsets in vertices,
    matching the retained base sticker storage convention. Hosts are counted
    separately and are not part of the exposed-face mesh.
    """
    triangles, offsets, entries = [], [0], []
    expected = 0
    for row in faces:
        vertices = np.array([[float(x) for x in v] for v in row["vertices"]])
        for face in row["cut_faces"]:
            before = len(triangles)
            for polygon in face["faces2"]:
                points = vertices[polygon["vertex_ids"]]
                centred = points - points.mean(axis=0)
                _, _, basis = np.linalg.svd(centred, full_matrices=False)
                angle = np.arctan2(centred @ basis[1], centred @ basis[0])
                ordered = points[np.argsort(angle)]
                triangles.extend([ordered[[0, i, i + 1]] for i in range(1, len(ordered) - 1)])
            if len(triangles) - before != face["triangles"]:
                raise AssertionError("float triangulation count differs from exact count")
            expected += face["triangles"]
            offsets.append(len(triangles) * 3)
            entries.append({"orbit": row["orbit"], "piece": row["piece"],
                            "pole": face["pole"], "side": face["side"]})
    mesh = np.asarray(triangles, dtype="<f4").reshape(-1, 4)
    if len(mesh) != expected * 3 or not np.isfinite(mesh).all():
        raise AssertionError("base mesh has an invalid count or coordinate")
    return {"vertices": mesh, "offsets": offsets, "faces": entries,
            "triangle_count": expected, "bytes": mesh.nbytes}


def _exact_json(value):
    if isinstance(value, Q5):
        return [value.a, value.b, value.d]
    raise TypeError(type(value).__name__)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", required=True)
    parser.add_argument("--write-mesh", action="store_true")
    args = parser.parse_args(argv)
    out = heights.output_directory(args.out, ("meshes-orbits.jsonl", "meshes-summary.json",
                                              "exposed-mesh-vertices.f32", "exposed-mesh-offsets.json"))
    started = perf_counter()
    ctx = sim.get_context()
    faces = orbit_faces(ctx)
    command = ["python", "-B", str(Path(__file__).resolve().relative_to(ROOT)),
               *(sys.argv[1:] if argv is None else argv)]
    report = dict(heights.environment(EVIDENCE_KIND, command=command),
                  **summary(faces, ctx.regions.report["orbit_sizes"]))
    report["sticker_comparison"] = sticker_comparison(ctx)
    mesh = build_base_mesh(faces)
    report["built_base_mesh"] = {"triangles": mesh["triangle_count"], "bytes": mesh["bytes"]}
    with (out / "meshes-orbits.jsonl").open("w", encoding="utf-8") as stream:
        for row in faces:
            stream.write(json.dumps(row, default=_exact_json) + "\n")
    if args.write_mesh:
        mesh["vertices"].tofile(out / "exposed-mesh-vertices.f32")
        (out / "exposed-mesh-offsets.json").write_text(json.dumps({
            "units": "float32 4D vertices", "offsets": mesh["offsets"],
            "faces": mesh["faces"]}, indent=2) + "\n", encoding="utf-8")
    report["sources"] = [heights.input_evidence(ROOT / "assets" / name)
                         for name in ("model.npz", "mesh.json", "mesh_vertices.f32", "slot_piece.u32")]
    report["seconds"] = perf_counter() - started
    (out / "meshes-summary.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    return report


if __name__ == "__main__":
    main()
