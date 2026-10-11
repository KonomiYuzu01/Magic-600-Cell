"""E3: return the two class-1 fragments at the E2 end with class-1 twists, keeping the lattice on the lattice.

At the E2 end of I_a@40 (the stop plus the E2 twists, joint-attack-I_a-40.json) six domains are off the
lattice; two of them, F50 (50 pieces) and F48 (48 pieces), are frames of class 1 (classes.py --states).
This script builds the class-1 cell model at that state, reads its structure, searches a word of class-1
twists that puts both fragments on the lattice, and verifies the word on the exact simulator.

Cell model. P = class 1 (120 poles). Own pieces are the lattice and the frames of P. A content is the set
of own pieces inside the same caps of P (Theorem B: none straddles a cut of P); its label is its domain's
left coset of 2I (five labels, label 0 = the lattice; card 3, F3). A free grip is one that is not blocked
and whose cap holds no piece of another class's frame. A twist (c, i) at a free grip moves the contents
in cap c by the exact permutation of the poles of P under its menu element, and changes their labels by
the element's left factor acting on the five cosets. Other-class pieces lie in no free cap, so they never
move. The points of the model are the pairs (cell, label) over every cell reachable from the occupied
ones; each twist is a permutation of the points.

Checks (each printed, and recorded in the results file):
- caps: every cap of P holds 63 reachable cells: the star of its pole (1 vertex, 12 edge, 30 triangle and
  20 tetrahedron cells of the 600-cell on the poles of P);
- orbits: the orbits of points under the free twists, with how they split the five labels of a cell;
- F50: its cell X = {38, 74, 170} has no label-0 point in the orbit of F50's point, so F50 cannot be
  put on the lattice in X; the only label-0 point of that orbit not held by a lattice content is in the
  empty cell Y = {5, 17, 38} (the hole); the single twists that put F50 there on label 0, and how many
  contents each leaves off label 0;
- F48: all five labels of its cell E = {5, 38} lie in one orbit;
- F50 search: commutator cubes [x, y]^3 of free twists at adjacent grips; P puts F50 on label 0 in Y,
  then Q puts every other content back on label 0 (both in fixed search order);
- F48 search: commutators [x, y] whose edge part twists two edge cells in place; the power n that clears
  every other point gives a pure twist of the two cells; against one fixed 5-cycle per cell the twist is
  an exponent pair, and per pair of cells all such twists have one exponent ratio, so two of them never
  untwist one cell alone; the Z5 rank of the pair vectors shows that the unit vector at E is in their span;
  the shortest combination along a cycle of three or four edge cells through E is searched, with each twist
  raised to its cheapest power that maps the goal set of non-edge points onto itself;
- model: the word puts every own content on label 0;
- exact: replayed on the exact simulator, the lattice domain after the word is the lattice before plus
  F50 and F48, and every other domain keeps exactly its pieces.

Run from the repository root:
    python research/jumbling/theory/e3_class.py            build, search, check and verify (about 6 min)
    python research/jumbling/theory/e3_class.py --verify   replay the recorded word on the exact simulator
Writes results/e3-class1-I_a-40.json by default; progress goes to stderr.

Evidence kind: source and exact synthetic geometry (exact pole permutations and cosets over Q(sqrt5), the
exact simulator), plus integer bookkeeping in the cell model. Timings are cloud timings of research code,
not performance evidence.
"""
import argparse
import itertools
import json
import math
import sys
import time
from collections import Counter, defaultdict
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
sys.path.insert(0, str(HERE.parent))
sys.path.insert(0, str(HERE.parent / 'fixtures'))

import classes as CL  # noqa: E402
import restore  # noqa: E402
import seams  # noqa: E402
import trees  # noqa: E402
from exact import matvec  # noqa: E402

P = 1
X_POLES, Y_POLES, E_POLES = (38, 74, 170), (5, 17, 38), (5, 38)
OUT = HERE / 'results' / 'e3-class1-I_a-40.json'
T0 = time.time()
ID5 = (0, 1, 2, 3, 4)


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def e2_end():
    R0 = HERE / 'results'
    stop = json.loads((R0 / 'seams-I_a-40-stop.json').read_text())['moves']
    e2 = json.loads((R0 / 'joint-attack-I_a-40.json').read_text())['e2_stop_no_history']['moves']
    st, M = seams._fixture_state('I_a', 40)
    R = seams.SeamRestorer(st, M)
    for c, i in stop + e2:
        R.apply(c, i)
    return R, M


def prop_unit(q):
    for u in trees.UNITS:
        k = next(i for i in range(4) if not u[i].is_zero())
        lam = q[k] / u[k]
        if all(q[i] == lam * u[i] for i in range(4)):
            return True
    return False


class Model:
    """The class-1 cell model at a state (see the module docstring)."""

    def __init__(self, R, M, members, cls):
        self.poles = poles = members[P]
        self.kof = {c: k for k, c in enumerate(poles)}
        D = R.domains()
        dom = np.asarray(D.dom)
        dcls = {}
        for d in range(1, len(D.reps)):
            h = CL.hub(D.reps[d], members, cls)
            dcls[d] = h[0][1] if len(h) == 1 else None
        frag = [d for d, v in dcls.items() if v == P]
        self.frag = {d: len(D.members[d]) for d in frag}
        own = (dom == 0) | np.isin(dom, frag)
        sig = np.zeros((len(dom), 120), bool)
        A, blocked = [], []
        for k, c in enumerate(poles):
            cl = R.st.classify(int(c), need_inside=True, stop_at_straddle=False)
            ins = np.asarray(cl.inside, np.int64)
            sig[ins, k] = True
            if cl.straddle is not None:
                blocked.append(int(c))
            elif not (~own)[ins].any():
                A.append(int(c))
        self.A, self.blocked = A, blocked
        contents = {}
        for p in np.flatnonzero(own).tolist():
            contents.setdefault(int.from_bytes(np.packbits(sig[p]).tobytes(), 'big'), []).append(p)
        domain = {}
        for key, ps in contents.items():
            ds = set(dom[ps].tolist())
            assert len(ds) == 1, ds
            domain[key] = ds.pop()
        self.size = {key: len(ps) for key, ps in contents.items()}
        log(own_contents=len(contents), free=len(A), blocked=blocked, fragments=self.frag)
        # the five left cosets of class P, label 0 = the lattice
        c0 = poles[0]
        inv0 = CL.ninv(c0)
        reps = []
        for u in trees.UNITS:
            a = trees.qmul(trees.qmul(CL.N[c0], u), inv0)
            if not any(prop_unit(trees.qmul(trees.qconj(r), a)) for r in reps):
                reps.append(a)
        assert len(reps) == 5
        reps.sort(key=lambda r: not prop_unit(r))
        assert prop_unit(reps[0]) and not any(prop_unit(r) for r in reps[1:])

        def label_of_left(l):
            out = [j for j, r in enumerate(reps) if prop_unit(trees.qmul(trees.qconj(r), l))]
            assert len(out) == 1, out
            return out[0]
        dl = {0: 0}
        for d in frag:
            l, r = trees.factors(D.reps[d])
            assert prop_unit(r)
            dl[d] = label_of_left(l)
        self.frag_label = {d: dl[d] for d in frag}
        # exact pole permutations and label actions of every free twist
        NI = len(M.items)
        perm, act = {}, {}
        for c in A:
            for i in range(NI):
                g = restore.element(M, c, i)
                img = [self.kof.get(CL.pole(matvec(g, CL.N[d])), -1) for d in poles]
                assert min(img) >= 0
                perm[(c, i)] = img
                l, _ = trees.factors(g)
                act[(c, i)] = tuple(label_of_left(trees.qmul(l, reps[k])) for k in range(5))
        log(model='pole permutations and label actions')
        self.gens = gens = [(c, i) for c in A for i in range(NI) if act[(c, i)] != ID5 or perm[(c, i)] != list(range(120))]
        cells = list(contents)
        idx = {s: n for n, s in enumerate(cells)}
        todo = list(cells)
        while todo:
            s = todo.pop()
            for c in A:
                if s >> (119 - self.kof[c]) & 1:
                    for i in range(NI):
                        t = self.move(s, perm[(c, i)])
                        if t not in idx:
                            idx[t] = len(cells)
                            cells.append(t)
                            todo.append(t)
        self.cells, self.cid = cells, idx
        self.NC = NC = len(cells)
        self.ncaps = np.array([bin(s).count('1') for s in cells])
        base = np.arange(NC * 5, dtype=np.int32)
        G = np.empty((len(gens), NC * 5), np.int32)
        for n, (c, i) in enumerate(gens):
            a, row, bit = act[(c, i)], base.copy(), 119 - self.kof[c]
            for s in cells:
                if s >> bit & 1:
                    f, t = idx[s], idx[self.move(s, perm[(c, i)])]
                    row[f * 5:f * 5 + 5] = [t * 5 + a[l] for l in range(5)]
            G[n] = row
        self.G = G
        key = {G[n].tobytes(): n for n in range(len(gens))}
        self.inv = np.empty(len(gens), np.int64)
        for n in range(len(gens)):
            r = np.empty(NC * 5, np.int32)
            r[G[n]] = base
            self.inv[n] = key[r.tobytes()]
        self.state = np.array([idx[s] * 5 + dl[domain[s]] for s in contents], np.int64)
        self.byc = defaultdict(list)
        for n, (c, i) in enumerate(gens):
            self.byc[c].append(n)
        self.ar = np.arange(NC * 5)
        log(cells=NC, generators=len(gens), bad_at_start=int((self.state % 5 != 0).sum()))

    def move(self, s, img):
        out, t = 0, s
        while t:
            b = t.bit_length() - 1
            t ^= 1 << b
            out |= 1 << (119 - img[119 - b])
        return out

    def cell(self, grips):
        return self.cid[sum(1 << (119 - self.kof[c]) for c in grips)]

    def caps(self, s):
        return [c for c in self.poles if s >> (119 - self.kof[c]) & 1]

    def comm(self, x, y, k=1):
        return [x, y, int(self.inv[x]), int(self.inv[y])] * k

    def run(self, word, pts):
        for g in word:
            pts = self.G[g][pts]
        return pts

    def array(self, word):
        return self.run(word, self.ar)

    def adjacent(self, near):
        """Ordered pairs of free grips sharing a cell, with at least one grip in near."""
        free = set(self.A)
        out = set()
        for s in self.cells:
            cs = [c for c in self.caps(s) if c in free]
            out |= {(a, b) for a in cs for b in cs if a != b and (a in near or b in near)}
        return sorted(out)


def cycles(w, pts):
    out, seen = [], set()
    for p in pts:
        if p in seen:
            continue
        n, q = 0, p
        while True:
            seen.add(q)
            q = int(w[q])
            n += 1
            if q == p:
                break
        out.append(n)
    return out


def lcm(xs):
    L = 1
    for v in xs:
        L = L * v // math.gcd(L, v)
    return L


def orbits(m):
    comp = m.ar.copy()
    while True:
        old = comp.copy()
        for g in m.G:
            np.minimum.at(comp, g, comp)
            comp = np.minimum(comp, comp[g])
        comp = comp[comp]
        if (comp == old).all():
            return comp


def structure(m, comp):
    X, Y, E = m.cell(X_POLES), m.cell(Y_POLES), m.cell(E_POLES)
    occ = {int(p) // 5: int(p) % 5 for p in m.state}
    per_cap = Counter(sum(1 for s in m.cells if s >> (119 - k) & 1) for k in range(120))
    star = Counter()
    for n, s in enumerate(m.cells):
        if s >> (119 - m.kof[38]) & 1:
            star[int(m.ncaps[n])] += 1
    names = {1: 'vertex', 2: 'edge', 3: 'triangle', 4: 'tetrahedron'}
    table = Counter()
    for o in set(comp.tolist()):
        pts = np.flatnonzero(comp == o)
        cs = Counter((pts // 5).tolist())
        typ = {int(m.ncaps[c]) for c in cs}
        k = set(cs.values())
        assert len(typ) == 1 and len(k) == 1
        table[(names.get(typ.pop(), 'outside'), len(cs), k.pop())] += 1
    f50, f48 = X * 5 + 1, E * 5 + 3
    assert occ[X] == 1 and occ[E] == 3
    o50 = np.flatnonzero(comp == comp[f50])
    zero50 = sorted({int(p) // 5 for p in o50 if p % 5 == 0})
    free_lattice = [c for c in zero50 if c not in occ]
    i50 = int(np.flatnonzero(m.state == f50)[0])
    one = []
    for n, g in enumerate(m.G):
        cur = g[m.state]
        if cur[i50] == Y * 5:
            one.append({'twist': list(m.gens[n]), 'contents_off_label_0_after': int((cur % 5 != 0).sum())})
    out = {
        'cells_per_cap': {str(k): v for k, v in per_cap.items()},
        'star_of_grip_38_by_cell_type': {names[k]: v for k, v in sorted(star.items())},
        'orbits': [{'cell_type': t, 'cells': n, 'labels_per_cell': k, 'orbits': v}
                   for (t, n, k), v in sorted(table.items(), key=lambda kv: (-kv[0][1] * kv[0][2], kv[0][0]))],
        'F50': {'cell': list(X_POLES), 'label': 1, 'pieces': int(m.size[m.cells[X]]),
                'labels_of_X_in_its_orbit': [l for l in range(5) if comp[X * 5 + l] == comp[f50]],
                'label_0_at_X_in_its_orbit': bool(comp[X * 5] == comp[f50]),
                'label_0_cells_in_its_orbit': len(zero50),
                'held_by_lattice_contents': sum(1 for c in zero50 if occ.get(c) == 0),
                'free_label_0_cells': [m.caps(m.cells[c]) for c in free_lattice],
                'single_twists_putting_it_on_label_0_in_the_hole': one},
        'F48': {'cell': list(E_POLES), 'label': 3, 'pieces': int(m.size[m.cells[E]]),
                'labels_of_E_in_its_orbit': [l for l in range(5) if comp[E * 5 + l] == comp[f48]]},
    }
    assert free_lattice == [Y] and not out['F50']['label_0_at_X_in_its_orbit']
    return out


def search_f50(m):
    """P then Q, both commutator cubes [x, y]^3, in fixed order."""
    X, Y, E = m.cell(X_POLES), m.cell(Y_POLES), m.cell(E_POLES)
    st = m.state
    i50 = int(np.flatnonzero(st == X * 5 + 1)[0])
    i48 = int(np.flatnonzero(st == E * 5 + 3)[0])
    near = {c for c in X_POLES + Y_POLES if c in m.byc}
    fam = [(x, y) for c, d in m.adjacent(near) for x in m.byc[c] for y in m.byc[d]]
    cands = []
    for x, y in fam:
        cur = m.run(m.comm(x, y, 3), st)
        if cur[i50] == Y * 5:
            cands.append((int((cur % 5 != 0).sum()), x, y, cur))
    cands.sort(key=lambda t: t[0])
    log(f50_first_cubes=len(cands), fewest_bad=cands[0][0] if cands else None)
    for bad, x, y, cur in cands:
        for qx, qy in fam:
            c2 = m.run(m.comm(qx, qy, 3), cur)
            badi = np.flatnonzero(c2 % 5 != 0)
            if c2[i50] == Y * 5 and len(badi) == 1 and badi[0] == i48:
                return m.comm(x, y, 3) + m.comm(qx, qy, 3), {'P': [m.gens[x], m.gens[y], 3], 'Q': [m.gens[qx], m.gens[qy], 3],
                                                             'bad_after_P': bad, 'first_cubes': len(cands)}
    raise RuntimeError('no F50 word in the searched family')


def rank5(rows):
    M = np.array(rows, np.int64) % 5
    rk = 0
    for col in range(M.shape[1]):
        piv = [i for i in range(rk, len(M)) if M[i, col]]
        if not piv:
            continue
        M[[rk, piv[0]]] = M[[piv[0], rk]]
        M[rk] = M[rk] * pow(int(M[rk, col]), -1, 5) % 5
        for i in range(len(M)):
            if i != rk and M[i, col]:
                M[i] = (M[i] - M[i, col] * M[rk]) % 5
        rk += 1
    return rk


def search_f48(m, after50):
    """A cycle of pure two-edge twists through E whose exponents close on E alone."""
    X, Y, E = m.cell(X_POLES), m.cell(Y_POLES), m.cell(E_POLES)
    ptype = np.repeat(m.ncaps, 5)
    occ = {int(p) // 5: int(p) % 5 for p in m.state}
    goal = np.zeros(len(m.ar), bool)
    for c in occ:
        goal[c * 5] = c != X
    goal[Y * 5] = True
    gnon = np.flatnonzero(goal & (ptype != 2))
    rho = {}

    def rpow(s, e):
        q = ID5
        for _ in range(e % 5):
            q = tuple(rho[s][j] for j in q)
        return q

    def expo(s, t):
        """e with rho_s^e = t (t a label permutation of cell s)."""
        for e in range(5):
            if rpow(s, e) == tuple(t):
                return e
        raise AssertionError('twist is not a power of the cell 5-cycle')
    near = set(m.caps(m.cells[E])) | {17}
    near = {b for a, b in m.adjacent(near)} | near
    pairs = [(a, b) for a, b in m.adjacent(near) if a in near and b in near]
    ratio, cheap, n_tools = defaultdict(set), {}, 0
    for c, d in pairs:
        for x in m.byc[c]:
            for y in m.byc[d]:
                w = m.array(m.comm(x, y))
                mv = np.flatnonzero(w != m.ar)
                em = mv[ptype[mv] == 2]
                if not len(em) or (w[em] // 5 != em // 5).any():
                    continue
                n = lcm(cycles(w, mv[ptype[mv] != 2].tolist()))
                if n % 5 == 0 or n > 100:
                    continue
                v = {}   # exponents of the commutator's own edge twists; C^n twists the same cells by n v
                for s in sorted(set((em // 5).tolist())):
                    t = tuple(int(w[s * 5 + l]) - s * 5 for l in range(5))
                    rho.setdefault(s, t)
                    v[s] = expo(s, t)
                if len(v) != 2 or not all(v.values()):
                    continue
                n_tools += 1
                u, z = sorted(v)
                ratio[(u, z)].add(v[z] * pow(v[u], -1, 5) % 5)
                cur = gnon.copy()
                for k in range(1, n + 1):
                    cur = w[cur]
                    if k % 5 and goal[cur].all():
                        lam = v[u] * k % 5
                        for sign, word in ((1, m.comm(x, y, k)), (-1, m.comm(y, x, k))):
                            key = ((u, z), sign * lam % 5)
                            if key not in cheap or len(word) < len(cheap[key]):
                                cheap[key] = word
    assert all(len(r) == 1 for r in ratio.values())
    nodes = sorted({c for pr in ratio for c in pr})
    ix = {c: k for k, c in enumerate(nodes)}
    rows = []
    for (u, z), r in ratio.items():
        row = [0] * len(nodes)
        row[ix[u]], row[ix[z]] = 1, next(iter(r))
        rows.append(row)
    unit = [0] * len(nodes)
    unit[ix[E]] = 1
    rk, rk_e = rank5(rows), rank5(rows + [unit])
    log(f48_pair_tools=n_tools, cell_pairs=len(ratio), cells=len(nodes), rank=rk, rank_with_unit_at_E=rk_e)
    lab = {}
    for (pr, lam), word in cheap.items():
        lab.setdefault(pr, {})[lam] = word
    for pr, d in lab.items():
        base = dict(d)
        for (l1, w1), (l2, w2) in itertools.product(base.items(), base.items()):
            l = (l1 + l2) % 5
            if l and (l not in d or len(w1) + len(w2) < len(d[l])):
                d[l] = w1 + w2
    tgt = next(e for e in range(5) if rpow(E, e)[3] == 0)   # rho_E^tgt maps F48's label 3 to label 0
    nb = defaultdict(set)
    for (u, z) in lab:
        nb[u].add(z)
        nb[z].add(u)

    def solve(cyc):
        k = len(cyc)
        es = [tuple(sorted((cyc[j], cyc[(j + 1) % k]))) for j in range(k)]
        A = np.zeros((k, k), np.int64)
        for j, (u, z) in enumerate(es):
            A[cyc.index(u), j] = 1
            A[cyc.index(z), j] = next(iter(ratio[(u, z)]))
        best = None
        for t in itertools.product(range(5), repeat=k):
            if (A @ np.array(t)) [0] % 5 != tgt or any((A[r] @ np.array(t)) % 5 for r in range(1, k)):
                continue
            if any(tt and tt not in lab[es[j]] for j, tt in enumerate(t)):
                continue
            word = [g for j, tt in enumerate(t) if tt for g in lab[es[j]][tt]]
            if best is None or len(word) < len(best[0]):
                best = (word, [m.caps(m.cells[c]) for c in cyc], list(t))
        return best
    sols = []
    for a in sorted(nb[E]):
        for b in sorted(nb[a]):
            if b != E and E in nb[b]:
                s = solve([E, a, b])
                if s:
                    sols.append(s)
            for c in sorted(nb[b]) if b != E else []:
                if c not in (E, a) and E in nb[c]:
                    s = solve([E, a, b, c])
                    if s:
                        sols.append(s)
    sols.sort(key=lambda s: len(s[0]))
    word, cyc, coef = sols[0]
    return word, {'pair_tools': n_tools, 'cell_pairs': len(ratio), 'one_ratio_per_pair': True,
                  'edge_cells': len(nodes), 'z5_rank': rk, 'z5_rank_with_unit_vector_at_E': rk_e,
                  'cycles_found': len(sols), 'cycle': cyc, 'multiples': coef, 'target_exponent': tgt}


def verify(word, members, cls):
    R, M = e2_end()
    D = R.domains()
    before = [frozenset(v.tolist()) for v in D.members]
    frag = [d for d in range(1, len(before)) if [q for _, q in CL.hub(D.reps[d], members, cls)] == [P]]
    for c, i in word:
        R.apply(c, i)
    D2 = R.domains()
    after = [frozenset(v.tolist()) for v in D2.members]
    want = before[0].union(*[before[d] for d in frag])
    rest = {v for d, v in enumerate(before) if d and d not in frag}
    out = {'twists': len(word), 'all_admissible': True,
           'domain_sizes_before': [len(v) for v in before], 'domain_sizes_after': [len(v) for v in after],
           'class_1_fragments': [len(before[d]) for d in frag],
           'lattice_after_is_lattice_before_plus_F50_F48': after[0] == want,
           'lattice_pieces_stay': before[0] <= after[0],
           'other_domains_unchanged': set(after[1:]) == rest and len(after) - 1 == len(rest),
           'off_lattice_before': sum(len(v) for v in before[1:]), 'off_lattice_after': sum(len(v) for v in after[1:])}
    log(exact=out)
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--verify', action='store_true')
    ap.add_argument('--out', default=str(OUT))
    a = ap.parse_args()
    if a.verify:
        rec = json.loads(Path(a.out).read_text())
        ex = verify(rec['word'], *CL.classes())
        ok = ex['lattice_after_is_lattice_before_plus_F50_F48'] and ex['other_domains_unchanged']
        print('exact replay', 'passes' if ok else 'FAILS')
        sys.exit(0 if ok else 1)
    members, cls = CL.classes()
    R, M = e2_end()
    m = Model(R, M, members, cls)
    out = {'state': 'E2 end of I_a@40 (seams-I_a-40-stop.json moves, then the E2 twists)', 'class': P,
           'free_grips': len(m.A), 'blocked_grips': m.blocked, 'own_contents': len(m.size),
           'fragments': {str(d): {'pieces': n, 'label': m.frag_label[d]} for d, n in m.frag.items()},
           'reachable_cells': m.NC, 'generators': len(m.gens)}
    comp = orbits(m)
    out['structure'] = structure(m, comp)
    log(structure=out['structure']['F50'])
    w50, info50 = search_f50(m)
    out['F50_search'] = {k: (list(map(list, v[:2])) + [v[2]] if k in 'PQ' else v) for k, v in info50.items()}
    log(f50=out['F50_search'])
    after50 = m.run(w50, m.state)
    w48, info48 = search_f48(m, after50)
    out['F48_search'] = info48
    log(f48=info48)
    word = w50 + w48
    cur = m.run(word, m.state)
    out['model_check'] = {'contents_off_label_0': int((cur % 5 != 0).sum())}
    assert out['model_check']['contents_off_label_0'] == 0
    out['word'] = [list(m.gens[g]) for g in word]
    out['parts'] = {'F50': len(w50), 'F48': len(w48)}
    out['exact'] = verify(out['word'], members, cls)
    out['seconds'] = round(time.time() - T0)
    Path(a.out).write_text(json.dumps(out, indent=1) + '\n')
    print('wrote', Path(a.out).resolve().relative_to(HERE.parent.parent.parent))


if __name__ == '__main__':
    main()
