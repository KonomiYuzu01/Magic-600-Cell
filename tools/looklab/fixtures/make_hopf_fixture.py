"""Reproduce the view lab's Hopf ring facts from the verified model; stdlib only."""
import ast
from collections import Counter
import hashlib
from itertools import permutations
import json
import math
from pathlib import Path
import struct
import sys
import zipfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
TOL = 1e-9
if not __debug__:
    raise SystemExit("make_hopf_fixture.py: run without -O; its checks are assertions")


def array(z, name, dtype, code):
    raw = z.read(name)
    if raw[:8] != b"\x93NUMPY\x01\x00":
        raise ValueError(f"{name}: expected .npy v1")
    size, = struct.unpack_from("<H", raw, 8)
    header = ast.literal_eval(raw[10:10 + size].decode("ascii").strip())
    if header["descr"] != dtype or header["fortran_order"]:
        raise ValueError(f"{name}: wrong dtype or array order")
    count = 1
    for n in header["shape"]:
        count *= n
    return struct.unpack(f"<{count}{code}", raw[10 + size:])


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def norm(a):
    return math.sqrt(dot(a, a))


def unit(a):
    length = norm(a)
    return tuple(x / length for x in a)


def distance(a, b):
    return norm(tuple(x - y for x, y in zip(a, b)))


def conj(a):
    return (a[0], -a[1], -a[2], -a[3])


def qmul(a, b):
    w, x, y, z = a
    t, i, j, k = b
    return (w * t - x * i - y * j - z * k,
            w * i + x * t + y * k - z * j,
            w * j - x * k + y * t + z * i,
            w * k + x * j - y * i + z * t)


def project(x, u, v):
    a, b = dot(x, u), dot(x, v)
    return tuple(a * u[i] + b * v[i] for i in range(4))


def ring_graph(ring_of, faces):
    return sorted({tuple(sorted((ring_of[a], ring_of[b])))
                   for a, b in faces.values() if ring_of[a] != ring_of[b]})


def read_inputs():
    """2A: keep the existing fixture's cell, vertex, face and helix ordering."""
    path = ROOT / "assets/model.npz"
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    manifest = json.loads((ROOT / "assets/manifest.json").read_bytes())
    if digest != manifest["files"]["assets/model.npz"]:
        raise ValueError("model.npz: manifest digest mismatch")
    with zipfile.ZipFile(path) as z:
        normals = array(z, "normals.npy", "<f8", "d")
        orbits = array(z, "orbit_id.npy", "<i2", "h")
        offsets = array(z, "face_offsets.npy", "<i4", "i")
        incidence = array(z, "face_values.npy", "<i2", "h")
    assert len(normals) == 600 * 4, "normals: expected 600 four-vectors"
    centres = [unit(normals[c * 4:c * 4 + 4]) for c in range(600)]
    cells = [[] for _ in range(600)]
    positions = [p for p, orbit in enumerate(orbits) if orbit == 34]
    assert len(positions) == 120, "vertices: expected 120 orbit-34 positions"
    vertices = []
    for v, p in enumerate(positions):
        owners = incidence[offsets[p]:offsets[p + 1]]
        assert len(owners) == len(set(owners)) == 20, "vertex: expected 20 cells"
        vertices.append(unit(tuple(sum(centres[c][i] for c in owners) for i in range(4))))
        for c in owners:
            cells[c].append(v)
    assert all(len(c) == 4 for c in cells), "cells: expected four vertices"
    faces = {}
    for c, cell in enumerate(cells):
        for omit in range(4):
            faces.setdefault(tuple(v for i, v in enumerate(cell) if i != omit), []).append(c)
    assert len(faces) == 1200 and all(len(x) == 2 for x in faces.values()), "faces: expected 1,200 pairs"
    neighbors = [[] for _ in cells]
    for a, b in faces.values():
        neighbors[a].append(b)
        neighbors[b].append(a)
    for c in range(600):
        neighbors[c].sort()
        dots = [dot(normals[c * 4:c * 4 + 4], normals[n * 4:n * 4 + 4])
                if n != c else -float("inf") for n in range(600)]
        top = max(dots)
        assert all(abs(dots[n] - top) < TOL for n in neighbors[c]), "face: not a top-dot neighbor"
        assert sum(abs(x - top) < TOL for x in dots) == 4, "cell: expected four top-dot neighbors"
        assert top - sorted(dots, reverse=True)[4] > 1e-6, "cell: no gap to fifth neighbor"
    unique = set()
    for start, cell in enumerate(cells):
        for initial in permutations(cell):
            window, current, visited = initial, start, []
            for _ in range(30):
                visited.append(current)
                owners = faces[tuple(sorted(window[1:]))]
                current = owners[1] if owners[0] == current else owners[0]
                next_vertex = next(v for v in cells[current] if v not in window[1:])
                window = (*window[1:], next_vertex)
            if window == initial and len(set(visited)) == 30:
                unique.add(tuple(sorted(visited)))
    helices = sorted(unique)
    assert len(helices) == 240, "helices: expected 240"
    return {"digest": digest, "centres": centres, "vertices": vertices, "cells": cells,
            "faces": faces, "neighbors": neighbors, "helices": helices}


def helix_planes(model):
    """2B: canonical chains, Fourier planes and both quaternion invariants."""
    planes = []
    for index, helix in enumerate(model["helices"]):
        members = set(helix)
        neighbors = {c: [n for n in model["neighbors"][c] if n in members] for c in helix}
        assert all(len(n) == 2 for n in neighbors.values()), f"helix {index}: not a cycle"
        chain = [helix[0]]
        previous, current = -1, helix[0]
        for _ in range(30):
            following = next(n for n in neighbors[current] if n != previous)
            previous, current = current, following
            if current == helix[0]:
                break
            chain.append(current)
        assert current == helix[0] and len(chain) == len(set(chain)) == 30, f"helix {index}: bad closure"
        u, v = [0.0] * 4, [0.0] * 4
        for k, c in enumerate(chain):
            angle = math.radians(12 * k)
            for i in range(4):
                u[i] += math.cos(angle) * model["centres"][c][i]
                v[i] += math.sin(angle) * model["centres"][c][i]
        assert abs(dot(u, v)) / (norm(u) * norm(v)) < TOL, f"helix {index}: Fourier orthogonality"
        assert abs(norm(u) / norm(v) - 1) < TOL, f"helix {index}: Fourier lengths"
        u, v = unit(u), unit(v)
        right, left = qmul(conj(u), v), qmul(v, conj(u))
        for invariant in (right, left):
            assert abs(invariant[0]) < TOL and abs(norm(invariant) - 1) < TOL, f"helix {index}: invariant"
        planes.append({"chain": chain, "u": u, "v": v, "right": right, "left": left})
    return planes


def find_covers(model, planes):
    """2C: invariant classes, exact covers, graphs and the cover of helix zero."""
    covers, class_counts = [], {}
    membership = [None] * len(planes)
    for side in ("right", "left"):
        groups = []
        for h, plane in enumerate(planes):
            invariant = plane[side]
            for group in groups:
                reference = planes[group[0]][side]
                if min(distance(invariant, reference), distance(invariant, tuple(-x for x in reference))) < 1e-6:
                    group.append(h)
                    break
            else:
                groups.append([h])
        sizes = Counter(len(group) for group in groups)
        assert len(groups) == 16 and sizes == {12: 10, 20: 6}, f"{side}: invariant classes"
        class_counts[side] = {str(size): sizes[size] for size in sorted(sizes)}
        side_covers = sorted((group for group in groups if len(group) == 20), key=lambda group: group[0])
        for index, group in enumerate(side_covers):
            ring_of = [-1] * 600
            for ring, h in enumerate(group):
                assert membership[h] is None, f"helix {h}: multiple covers"
                membership[h] = (side, index)
                for c in model["helices"][h]:
                    assert ring_of[c] == -1, f"{side} cover {index}: overlap"
                    ring_of[c] = ring
            assert all(r >= 0 for r in ring_of), f"{side} cover {index}: incomplete"
            graph = ring_graph(ring_of, model["faces"])
            assert len(graph) == 30, f"{side} cover {index}: edge count"
            assert all(sum(r in edge for edge in graph) == 3 for r in range(20)), f"{side} cover {index}: degree"
            covers.append({"side": side, "cover": index, "helices": group, "graph": graph})
    assert all(item is not None for item in membership), "covers: do not partition all helices"
    chosen = [cover for cover in covers if 0 in cover["helices"]]
    assert len(chosen) == 1, "helix 0: expected one cover"
    chosen = chosen[0]
    if chosen["side"] != "right":
        raise ValueError("helix 0 belongs to a left cover; mirrored chart requires a new packet")
    return {"covers": covers, "classes": class_counts, "membership": membership,
            "chosen": chosen, "J": planes[0][chosen["side"]]}


def ring_phases(model, planes, cover_data):
    """2D: first Hamiltonian path, transported origins, half-steps and bricks."""
    cover, j = cover_data["chosen"], cover_data["J"]
    neighbors = [[] for _ in range(20)]
    for a, b in cover["graph"]:
        neighbors[a].append(b)
        neighbors[b].append(a)
    for adjacent in neighbors:
        adjacent.sort(key=lambda r: cover["helices"][r])
    path = [cover["helices"].index(0)]
    visited = set(path)

    def search():
        if len(path) == 20:
            return True
        for r in neighbors[path[-1]]:
            if r not in visited:
                visited.add(r)
                path.append(r)
                if search():
                    return True
                path.pop()
                visited.remove(r)
        return False

    assert search(), "ring graph: no Hamiltonian path"
    rings, parities = [], []
    ring_of, half_step = [-1] * 600, [-1] * 600
    directions = Counter()
    for r, cover_ring in enumerate(path):
        h = cover["helices"][cover_ring]
        plane = planes[h]
        u, v = plane["u"], plane["v"]
        for vector in (qmul(u, j), qmul(v, j)):
            assert distance(vector, project(vector, u, v)) < TOL, f"ring {r}: not a right fibre"
        if r == 0:
            assert plane["chain"][0] == 0, "path: first chain must start at cell 0"
            previous = model["centres"][plane["chain"][0]]
        else:
            previous = rings[-1]["origin"]
        origin = unit(project(previous, u, v))
        tangent = qmul(origin, j)
        for c in model["helices"][h]:
            angle = math.degrees(math.atan2(dot(model["centres"][c], tangent), dot(model["centres"][c], origin))) % 360
            steps = angle / 6
            assert abs(steps - round(steps)) < TOL, f"cell {c}: nonintegral half-step"
            half_step[c] = round(steps) % 60
            ring_of[c] = r
        cells = sorted(model["helices"][h], key=lambda c: half_step[c])
        steps = [half_step[c] for c in cells]
        assert len(set(steps)) == 30 and len({s % 2 for s in steps}) == 1, f"ring {r}: half-step parity"
        parities.append(steps[0] % 2)
        assert all(cells[(i + 1) % 30] in model["neighbors"][c] for i, c in enumerate(cells)), f"ring {r}: cell order"
        chain = plane["chain"]
        chain_steps = {(half_step[chain[(i + 1) % 30]] - half_step[c] + 30) % 60 - 30 for i, c in enumerate(chain)}
        assert chain_steps in ({2}, {-2}), f"ring {r}: chain direction"
        directions[next(iter(chain_steps))] += 1
        rings.append({"helix": h, "cells": cells, "halfSteps": steps, "origin": origin})
    assert all(a != b for a, b in zip(parities, parities[1:])), "path: parity does not alternate"
    brick_counts, seam_count = [], 0
    for r in range(19):
        offsets = []
        for a, b in model["faces"].values():
            if {ring_of[a], ring_of[b]} != {r, r + 1}:
                continue
            if ring_of[a] != r:
                a, b = b, a
            offsets.append((half_step[b] - half_step[a] + 30) % 60 - 30)
            if {half_step[a], half_step[b]} == {0, 59}:
                seam_count += 1
        assert set(offsets) == {-1, 1}, f"path rows {r}, {r + 1}: brick offsets"
        brick_counts.append(len(offsets))
    assert len(set(brick_counts)) == 1, "path rows: unequal face pair counts"
    return {"rings": rings, "ringOf": ring_of, "halfStep": half_step,
            "graph": ring_graph(ring_of, model["faces"]), "path": path,
            "parities": parities, "directions": directions,
            "brickCounts": brick_counts, "seamCount": seam_count}


def matmul(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(4)) for j in range(4)] for i in range(4)]


def inverse4(matrix):
    rows = [list(row) + [float(i == j) for j in range(4)] for i, row in enumerate(matrix)]
    for column in range(4):
        pivot = max(range(column, 4), key=lambda r: abs(rows[r][column]))
        rows[column], rows[pivot] = rows[pivot], rows[column]
        scale = rows[column][column]
        assert abs(scale) > 1e-12, "screw: singular normal equations"
        rows[column] = [x / scale for x in rows[column]]
        for r in range(4):
            if r != column:
                factor = rows[r][column]
                rows[r] = [x - factor * y for x, y in zip(rows[r], rows[column])]
    return [row[4:] for row in rows]


def angle_between(a, b):
    return math.degrees(math.acos(max(-1.0, min(1.0, dot(a, b)))))


def screws_and_base_points(model, planes, cover_data, phases):
    """2E: least-squares screw symmetries and the dodecahedral base."""
    centres = model["centres"]
    centre_keys = {tuple(round(x, 9) for x in centre): c for c, centre in enumerate(centres)}
    assert len(centre_keys) == 600, "centres: rounded keys collide"
    identity = [[float(i == j) for j in range(4)] for i in range(4)]
    orders, degrees, bases = [], [], []
    for r, ring in enumerate(phases["rings"]):
        plane = planes[ring["helix"]]
        xs = [centres[c] for c in plane["chain"]]
        ys = xs[1:] + xs[:1]
        gram = [[sum(x[i] * x[j] for x in xs) for j in range(4)] for i in range(4)]
        cross = [[sum(y[i] * x[j] for x, y in zip(xs, ys)) for j in range(4)] for i in range(4)]
        matrix = matmul(cross, inverse4(gram))
        assert all(distance(tuple(dot(row, x) for row in matrix), y) < TOL for x, y in zip(xs, ys)), f"ring {r}: screw residual"
        assert all(abs(dot(matrix[i], matrix[j]) - (i == j)) < TOL for i in range(4) for j in range(4)), f"ring {r}: screw orthogonality"
        images = set()
        for centre in centres:
            image = tuple(dot(row, centre) for row in matrix)
            key = tuple(round(x, 9) for x in image)
            assert key in centre_keys, f"ring {r}: screw does not preserve centres"
            assert distance(image, centres[centre_keys[key]]) < TOL, f"ring {r}: screw centre residual"
            images.add(centre_keys[key])
        assert len(images) == 600, f"ring {r}: screw is not a permutation"
        power, identity_powers = identity, []
        for exponent in range(1, 31):
            power = matmul(power, matrix)
            if max(abs(power[i][j] - identity[i][j]) for i in range(4) for j in range(4)) < TOL:
                identity_powers.append(exponent)
        assert identity_powers == [30], f"ring {r}: screw order {identity_powers}"
        orders.append(identity_powers[0])
        angles = [math.atan2(dot(x, plane["v"]), dot(x, plane["u"])) for x in xs[:2]]
        step_angle = math.degrees(angles[1] - angles[0]) % 360
        assert abs(step_angle - 12) < TOL, f"ring {r}: screw angle"
        degrees.append(round(step_angle))
        base = qmul(qmul(ring["origin"], cover_data["J"]), conj(ring["origin"]))
        assert abs(base[0]) < TOL and abs(norm(base[1:]) - 1) < TOL, f"ring {r}: base point"
        bases.append(base[1:])
    expected_angles = {0.0: 1, 41.81: 3, 70.53: 6, 109.47: 6, 138.19: 3, 180.0: 1}
    for r, base in enumerate(bases):
        base_angles = Counter(round(angle_between(base, other), 2) for other in bases)
        assert base_angles == expected_angles, f"ring {r}: dodecahedron angles {base_angles}"
    adjacent_angles = {round(angle_between(bases[a], bases[b]), 2) for a, b in phases["graph"]}
    assert adjacent_angles == {41.81}, "base graph: adjacent angle"
    assert len(set(orders)) == len(set(degrees)) == 1, "screws: inconsistent rings"
    return {"bases": bases, "order": orders[0], "degrees": degrees[0],
            "baseAngles": [[angle, count] for angle, count in sorted(base_angles.items())],
            "adjacentAngle": next(iter(adjacent_angles))}


def determinant4(matrix):
    rows, result = [list(row) for row in matrix], 1.0
    for column in range(4):
        pivot = max(range(column, 4), key=lambda r: abs(rows[r][column]))
        if pivot != column:
            rows[column], rows[pivot] = rows[pivot], rows[column]
            result = -result
        scale = rows[column][column]
        result *= scale
        for r in range(column + 1, 4):
            factor = rows[r][column] / scale
            for i in range(column + 1, 4):
                rows[r][i] -= factor * rows[column][i]
    return result


def charts_and_bead(model, cover_data, phases):
    """2F: deterministic K, positive orthonormal charts and one bead class."""
    j = cover_data["J"]
    for e in ((0.0, 1.0, 0.0, 0.0), (0.0, 0.0, 1.0, 0.0), (0.0, 0.0, 0.0, 1.0)):
        candidate = tuple(e[i] - dot(e, j) * j[i] for i in range(4))
        if norm(candidate) > 0.5:
            k = unit(candidate)
            break
    jk = qmul(j, k)
    reference, determinants = None, []
    for c, centre in enumerate(model["centres"]):
        angle = math.radians(18 - 72 * phases["halfStep"][c])
        exponential = (math.cos(angle), *(math.sin(angle) * x for x in j[1:]))
        e = qmul(centre, exponential)
        f, g, h = qmul(centre, j), qmul(e, k), qmul(e, jk)
        frame = (centre, f, g, h)
        assert all(abs(dot(frame[i], frame[d]) - (i == d)) < TOL for i in range(4) for d in range(4)), f"cell {c}: chart orthonormality"
        determinant = determinant4(frame)
        assert abs(determinant - 1) < TOL, f"cell {c}: chart determinant {determinant}"
        determinants.append(determinant)
        points = []
        for v in sorted(model["cells"][c]):
            vertex = model["vertices"][v]
            denominator = dot(vertex, centre)
            p = tuple(vertex[i] / denominator - centre[i] for i in range(4))
            points.append(tuple(dot(axis, p) for axis in (f, g, h)))
        if c == 0:
            reference = points
        matches = [[i for i, point in enumerate(reference) if distance(value, point) < TOL] for value in points]
        assert all(len(match) == 1 for match in matches), f"cell {c}: bead points differ"
        assert len({match[0] for match in matches}) == 4, f"cell {c}: bead is not a bijection"
    return {"K": k, "bead": reference, "beadClasses": 1, "determinant": round(determinants[0]),
            "determinantRange": (min(determinants), max(determinants))}


def rounded(value):
    if isinstance(value, float):
        return round(value, 12) + 0.0
    if isinstance(value, dict):
        return {key: rounded(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [rounded(item) for item in value]
    return value


def make_fixture(model, planes, cover_data, phases, screws, charts, comparison):
    """2G: ordered fixture, with floats rounded only after all geometry checks."""
    covers = cover_data["covers"]
    graph = phases["graph"]
    facts = {"rightClasses": cover_data["classes"]["right"], "leftClasses": cover_data["classes"]["left"],
             "rightCovers": sum(c["side"] == "right" for c in covers),
             "leftCovers": sum(c["side"] == "left" for c in covers),
             "coversPartitionHelices": all(c is not None for c in cover_data["membership"]),
             "coverGraphDegree": sum(0 in edge for edge in graph), "coverGraphEdges": len(graph),
             "screwOrder": screws["order"], "screwDegrees": screws["degrees"],
             "baseAngles": screws["baseAngles"], "adjacentBaseAngle": screws["adjacentAngle"],
             "halfStepsIntegral": True, "parityAlternates": True, "brick": True,
             "chainsAlongJ": phases["directions"][2], "chainsAgainstJ": phases["directions"][-2],
             "rowFacePairs": phases["brickCounts"][0], "seamFacePairs": phases["seamCount"],
             "beadClasses": charts["beadClasses"], "chartDeterminant": charts["determinant"]}
    rings = []
    for r, ring in enumerate(phases["rings"]):
        rings.append({"helix": ring["helix"], "cells": ring["cells"], "halfSteps": ring["halfSteps"],
                      "origin": ring["origin"], "basePoint": screws["bases"][r]})
    return rounded({"format": "magic600-look-hopf-rings", "version": 1, "modelDigest": model["digest"],
                    "helixCount": len(planes), "side": cover_data["chosen"]["side"],
                    "J": cover_data["J"], "K": charts["K"], "facts": facts, "rings": rings,
                    "ringOf": phases["ringOf"], "halfStep": phases["halfStep"], "graph": graph,
                    "bead": charts["bead"], "lookLabRings": comparison})


def look_lab_comparison(model, planes, cover_data):
    """2H: read the retained Look Lab rings only for the LH-05 comparison."""
    retained = json.loads((HERE / "rings.json").read_bytes())
    helix_indices = {helix: h for h, helix in enumerate(model["helices"])}
    assert len(retained["rings"]) == 20, "rings.json: expected 20 rings"
    indices = []
    for ring in retained["rings"]:
        key = tuple(sorted(ring))
        assert key in helix_indices, "rings.json: ring is not one of the 240 helices"
        indices.append(helix_indices[key])
    counts = Counter(cover_data["membership"][h] for h in indices)
    chosen = cover_data["chosen"]
    by_cover = [{"side": side, "cover": index, "chosen": (side, index) == (chosen["side"], chosen["cover"]), "rings": count}
                for (side, index), count in sorted(counts.items())]
    spreads = {}
    for side in ("right", "left"):
        reference = planes[indices[0]][side]
        spreads[side] = round(max(min(distance(planes[h][side], reference),
                                     distance(planes[h][side], tuple(-x for x in reference))) for h in indices), 6) + 0.0
    degrees = {sum(r in edge for edge in retained["graph"]) for r in range(20)}
    assert len(degrees) == 1, "rings.json: graph is not regular"
    return {"rings": len(indices), "byCover": by_cover, "jSpread": spreads,
            "graphDegree": next(iter(degrees)), "hopfCover": len(counts) == 1}


def compare(expected, actual, path="$"):
    if type(expected) is not type(actual):
        raise ValueError(f"{path}: different type")
    if isinstance(expected, dict):
        keys = list(actual)
        for i, (key, value) in enumerate(expected.items()):
            if i >= len(keys) or keys[i] != key:
                raise ValueError(f"{path}.{key}: missing or out-of-order key")
            compare(value, actual[key], f"{path}.{key}")
        if len(keys) > len(expected):
            raise ValueError(f"{path}.{keys[len(expected)]}: unexpected key")
    elif isinstance(expected, list):
        for i, value in enumerate(expected):
            if i >= len(actual):
                raise ValueError(f"{path}[{i}]: missing item")
            compare(value, actual[i], f"{path}[{i}]")
        if len(actual) > len(expected):
            raise ValueError(f"{path}[{len(expected)}]: unexpected item")
    elif isinstance(expected, float):
        if not math.isfinite(actual) or abs(expected - actual) > TOL:
            raise ValueError(f"{path}: different float")
    elif expected != actual:
        raise ValueError(f"{path}: different value")


def main():
    """2I: write once, or recompute and compare without writing anything."""
    if sys.argv[1:] not in ([], ["--check"]):
        print("usage: make_hopf_fixture.py [--check]", file=sys.stderr)
        return 2
    model = read_inputs()
    planes = helix_planes(model)
    covers = find_covers(model, planes)
    phases = ring_phases(model, planes, covers)
    screws = screws_and_base_points(model, planes, covers, phases)
    charts = charts_and_bead(model, covers, phases)
    comparison = look_lab_comparison(model, planes, covers)
    fixture = make_fixture(model, planes, covers, phases, screws, charts, comparison)
    path = HERE / "hopf-rings.json"
    if sys.argv[1:]:
        compare(fixture, json.loads(path.read_bytes()))
    else:
        path.write_text(json.dumps(fixture, indent=2) + "\n", encoding="utf-8", newline="\n")
    facts = fixture["facts"]
    print(f"HOPF_FIXTURE_PASS rings={len(fixture['rings'])} cells={len(fixture['ringOf'])} "
          f"covers={facts['rightCovers']}+{facts['leftCovers']} path={len(phases['path'])} beads={facts['beadClasses']}")
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (AssertionError, ValueError) as error:
        print(f"HOPF_FIXTURE_FAIL {error}", file=sys.stderr)
        sys.exit(1)
