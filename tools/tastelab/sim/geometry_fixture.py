"""Build the Taste Lab geometry fixture from the exact base-polytope construction.

Vertices come from research/audit/verify_regular_geometry.py (radius two,
coordinates a+b*phi as integer pairs, sorted as exact pair tuples). The
fixture lists the vertices, the 600 cells and a deterministic cover of the
cells by 20 rings of 30 face-connected cells. Standard library only.

Usage: python tools/tastelab/sim/geometry_fixture.py [--check]
"""
from itertools import combinations, permutations, product
from pathlib import Path
import importlib.util
import json
import sys

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / "tools/tastelab/page/tests/fixtures/600cell.json"

spec = importlib.util.spec_from_file_location("vrg", ROOT / "research/audit/verify_regular_geometry.py")
vrg = importlib.util.module_from_spec(spec)
spec.loader.exec_module(vrg)
ZERO, dot = vrg.ZERO, vrg.dot


def vertices():
    vs = set()
    for axis in range(4):
        for sign in (-1, 1):
            v = [ZERO] * 4
            v[axis] = (2 * sign, 0)
            vs.add(tuple(v))
    vs.update(tuple((s, 0) for s in signs) for signs in product((-1, 1), repeat=4))
    even = [p for p in permutations(range(4))
            if sum(p[i] > p[j] for i in range(4) for j in range(i + 1, 4)) % 2 == 0]
    for a, b, c in product((-1, 1), repeat=3):
        v = (ZERO, (a, 0), (0, b), (-c, c))
        vs.update(tuple(v[i] for i in order) for order in even)
    return sorted(vs)


def build():
    V = vertices()
    assert len(V) == 120
    adj = [set() for _ in V]
    for i, j in combinations(range(120), 2):
        if dot(V[i], V[j]) == (0, 2):
            adj[i].add(j)
            adj[j].add(i)
    cells = sorted({tuple(sorted((i, j, k, l)))
                    for i in range(120) for j in adj[i] for k in adj[i] & adj[j]
                    for l in adj[i] & adj[j] & adj[k]})
    assert len(cells) == 600
    index = {frozenset(c): n for n, c in enumerate(cells)}
    faces = {}
    for n, c in enumerate(cells):
        for f in combinations(c, 3):
            faces.setdefault(frozenset(f), []).append(n)
    assert len(faces) == 1200 and all(len(x) == 2 for x in faces.values())

    def step(a, b, c, d):
        face = frozenset((b, c, d))
        for n in faces[face]:
            (e,) = set(cells[n]) - face
            if e != a:
                return e

    # A ring is a closed Boerdijk-Coxeter helix: a cyclic vertex sequence of
    # period 30 in which every 4 consecutive vertices form a cell.
    rings = set()
    for c in cells:
        for seq in permutations(c):
            seq = list(seq)
            for _ in range(30):
                seq.append(step(*seq[-4:]))
            ring = frozenset(index[frozenset(seq[i:i + 4])] for i in range(30))
            if seq[30:34] == seq[:4] and len(ring) == 30:
                rings.add(tuple(sorted(ring)))
    rings = sorted(rings)
    sets = [frozenset(r) for r in rings]
    by_cell = {n: [k for k, r in enumerate(sets) if n in r] for n in range(600)}
    pairs = [tuple(x) for x in faces.values()]

    def seven_regular(choice):
        owner = {}
        for k, r in enumerate(choice):
            for n in sets[r]:
                owner[n] = k
        touch = {frozenset((owner[x], owner[y])) for x, y in pairs if owner[x] != owner[y]}
        return all(sum(1 for e in touch if k in e) == 7 for k in range(20))

    def search(used, choice):
        # Exact cover in a fixed order: lowest uncovered cell, rings in sorted order;
        # the first cover in which every ring touches exactly 7 others is taken.
        if len(choice) == 20:
            return choice if seven_regular(choice) else None
        n = next(n for n in range(600) if n not in used)
        for k in by_cell[n]:
            if not sets[k] & used:
                found = search(used | sets[k], choice + [k])
                if found:
                    return found
        return None

    cover = search(frozenset(), [])
    assert cover is not None
    return {
        "source": "research/audit/verify_regular_geometry.py construction; radius 2; coordinates [a, b] mean a + b*phi",
        "vertices": [[list(x) for x in v] for v in V],
        "counts": {"vertices": 120, "edges": 720, "triangles": 1200, "cells": 600, "helices": len(rings)},
        "cells": [list(c) for c in cells],
        "rings": [list(rings[k]) for k in cover],
    }


def main():
    text = json.dumps(build(), separators=(",", ":")) + "\n"
    if "--check" in sys.argv[1:]:
        ok = OUT.exists() and OUT.read_text(encoding="utf-8") == text
        print("fixture: ok" if ok else "fixture: stale")
        return 0 if ok else 1
    OUT.write_text(text, encoding="utf-8")
    print("wrote", OUT.relative_to(ROOT).as_posix())
    return 0


if __name__ == "__main__":
    sys.exit(main())
