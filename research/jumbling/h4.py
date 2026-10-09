"""H4 geometry helpers for the 4D jumbling study (scratch research code)."""
import itertools
import numpy as np

PHI = (1 + 5 ** 0.5) / 2
EPS = 1e-9


def qmul(a, b):
    a0, a1, a2, a3 = np.moveaxis(a, -1, 0)
    b0, b1, b2, b3 = np.moveaxis(b, -1, 0)
    return np.stack([
        a0 * b0 - a1 * b1 - a2 * b2 - a3 * b3,
        a0 * b1 + a1 * b0 + a2 * b3 - a3 * b2,
        a0 * b2 - a1 * b3 + a2 * b0 + a3 * b1,
        a0 * b3 + a1 * b2 - a2 * b1 + a3 * b0,
    ], -1)


def qconj(a):
    return a * np.array([1, -1, -1, -1])


def binary_icosahedral():
    """The 120 unit quaternions of 2I = vertices of the 600-cell."""
    pts = set()
    for s in itertools.product([1, -1], repeat=4):
        pts.add(tuple(0.5 * np.array(s)))
    for i in range(4):
        for s in (1, -1):
            v = [0.0] * 4
            v[i] = s
            pts.add(tuple(v))
    base = [0.0, 0.5, PHI / 2, 1 / (2 * PHI)]
    # even permutations of (0, 1/2, phi/2, 1/(2 phi)) with all signs
    even = [p for p in itertools.permutations(range(4))
            if sum(1 for i in range(4) for j in range(i + 1, 4) if p[i] > p[j]) % 2 == 0]
    for p in even:
        for s in itertools.product([1, -1], repeat=3):
            v = [base[p[k]] for k in range(4)]
            signs = iter(s)
            v = [x * next(signs) if abs(x) > 1e-12 else 0.0 for x in v]
            pts.add(tuple(np.round(v, 12)))
    arr = np.array(sorted(pts))
    assert len(arr) == 120, len(arr)
    return arr


def dedup(points, tol=1e-7):
    keys = {}
    out = []
    for p in points:
        k = tuple(np.round(p / tol).astype(np.int64))
        if k not in keys:
            # neighbour check for rounding boundaries
            keys[k] = len(out)
            out.append(p)
    out = np.array(out)
    # second pass with explicit distances (robust)
    keep = []
    for i, p in enumerate(out):
        if not keep or np.min(np.linalg.norm(out[keep] - p, axis=1)) > 1e-6:
            keep.append(i)
    return out[keep]


def h4_polytope():
    V = binary_icosahedral()
    G = V @ V.T
    edge_dot = PHI / 2  # cos 36 deg
    E = [(i, j) for i in range(120) for j in range(i + 1, 120) if abs(G[i, j] - edge_dot) < 1e-9]
    assert len(E) == 720
    adj = [set() for _ in range(120)]
    for i, j in E:
        adj[i].add(j); adj[j].add(i)
    T = sorted({tuple(sorted((i, j, k))) for i, j in E for k in adj[i] & adj[j]})
    assert len(T) == 1200
    C = sorted({tuple(sorted((i, j, k, l))) for (i, j, k) in T for l in adj[i] & adj[j] & adj[k]})
    assert len(C) == 600
    def centers(sets):
        c = np.array([V[list(s)].sum(0) for s in sets])
        return c / np.linalg.norm(c, axis=1)[:, None]
    return {
        'V': V,
        'E': centers(E), 'Eidx': E,
        'F': centers(T), 'Fidx': T,
        'C': centers(C), 'Cidx': C,
    }


def rotation_group(V):
    """All 7200 rotations x -> l x r of the 600-cell, as 4x4 matrices."""
    eye = np.eye(4)
    mats = []
    for l in V:
        L = qmul(np.broadcast_to(l, (4, 4)), eye)  # columns: l * e_k
        for r in V:
            M = qmul(L, np.broadcast_to(r, (4, 4)))  # rows k: l e_k r
            mats.append(M.T)
    mats = np.array(mats)
    # +-(l,r) give the same rotation: deduplicate
    flat = np.round(mats.reshape(len(mats), 16), 9)
    _, idx = np.unique(flat, axis=0, return_index=True)
    mats = mats[np.sort(idx)]
    assert len(mats) == 7200, len(mats)
    return mats


def frame_perp(v):
    """Orthonormal basis (3x4) of v-perp."""
    q, _ = np.linalg.qr(np.column_stack([v, np.eye(4)]))
    B = q[:, 1:4].T
    # fix orientation so that det([v; B]) = +1
    if np.linalg.det(np.vstack([v, B])) < 0:
        B[2] = -B[2]
    return B


def rot_about(v, R3):
    """Embed a 3D rotation R3 acting on v-perp (in frame_perp(v) coords) into SO(4)."""
    B = frame_perp(v)
    P = np.outer(v, v)
    return P + B.T @ R3 @ B


def axis_angle(axis, ang):
    a = np.asarray(axis, float); a = a / np.linalg.norm(a)
    K = np.array([[0, -a[2], a[1]], [a[2], 0, -a[0]], [-a[1], a[0], 0]])
    return np.eye(3) + np.sin(ang) * K + (1 - np.cos(ang)) * K @ K


def rot3_to_axis_angle(R):
    ang = np.arccos(np.clip((np.trace(R) - 1) / 2, -1, 1))
    if ang < 1e-9:
        return np.array([0, 0, 1.0]), 0.0
    if abs(ang - np.pi) < 1e-6:
        w, U = np.linalg.eigh((R + R.T) / 2)
        return U[:, np.argmax(w)], ang
    ax = np.array([R[2, 1] - R[1, 2], R[0, 2] - R[2, 0], R[1, 0] - R[0, 1]])
    return ax / np.linalg.norm(ax), ang
