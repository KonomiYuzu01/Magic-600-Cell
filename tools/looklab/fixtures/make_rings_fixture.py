"""Pin the Taste Lab ring search in engine cell and vertex order; stdlib only."""
import ast
import hashlib
from itertools import permutations
import json
from pathlib import Path
import struct
import zipfile

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]


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


def main():
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
    cells = [[] for _ in range(600)]
    positions = [p for p, orbit in enumerate(orbits) if orbit == 34]
    assert len(positions) == 120
    for v, p in enumerate(positions):
        owners = incidence[offsets[p]:offsets[p + 1]]
        assert len(set(owners)) == 20
        for c in owners:
            cells[c].append(v)
    assert all(len(c) == 4 for c in cells)
    faces = {}
    for n, cell in enumerate(cells):
        for omit in range(4):
            faces.setdefault(tuple(v for i, v in enumerate(cell) if i != omit), []).append(n)
    assert len(faces) == 1200 and all(len(x) == 2 for x in faces.values())
    for a, b in faces.values():
        dots = [sum(normals[a * 4 + i] * normals[n * 4 + i] for i in range(4))
                if n != a else -float("inf") for n in range(600)]
        top = max(dots)
        assert abs(dots[b] - top) < 1e-9
        assert sum(abs(x - top) < 1e-9 for x in dots) == 4
        assert top - sorted(dots, reverse=True)[4] > 1e-6
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
    candidates = sorted(unique)
    masks = [sum(1 << n for n in r) for r in candidates]
    by_cell = [[r for r, cells_in_ring in enumerate(candidates) if n in cells_in_ring] for n in range(600)]
    choice = []

    def graph(ring_of):
        return sorted({tuple(sorted((ring_of[a], ring_of[b]))) for a, b in faces.values() if ring_of[a] != ring_of[b]})

    def owner():
        ring_of = [-1] * 600
        for r, selected in enumerate(choice):
            for n in candidates[selected]:
                assert ring_of[n] == -1
                ring_of[n] = r
        return ring_of

    def search(used):
        if len(choice) == 20:
            pairs = graph(owner())
            return all(sum(r in p for p in pairs) == 7 for r in range(20))
        n = next(n for n in range(600) if not used & (1 << n))
        for r in by_cell[n]:
            if used & masks[r]:
                continue
            choice.append(r)
            if search(used | masks[r]):
                return True
            choice.pop()
        return False

    assert search(0)
    ring_of = owner()
    fixture = {"format": "magic600-look-rings", "version": 1, "modelDigest": digest,
               "vertexPositions": positions, "candidateCount": len(candidates),
               "ringOf": ring_of, "rings": [list(candidates[r]) for r in choice], "graph": graph(ring_of)}
    (HERE / "rings.json").write_text(json.dumps(fixture, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"Rings: first 7-regular cover from {len(candidates)} sorted helices")


if __name__ == "__main__":
    main()
