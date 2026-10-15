"""Prototype: closed content set Omega for one root and BFS over the restricted maps of its iterated refinements.

Omega starts with the rare lattice contents in the caps of W0's grips and of the grips G whose caps hold the rare
landing chambers, plus the rare pieces of T, and is closed under W0, W0^-1 and every K+ letter at G (each a partial
map on contents: inside the cap moved, outside fixed, straddling dead). On a closed Omega, every iterated refinement
W_n = [W_(n-1), k_n] restricts to a partial bijection of Omega determined by W_(n-1)|Omega and k_n.

usage: rclose.py <theory> <echain> <table> <target json> <rows.jsonl> --types 7,9 [--row N] [--max-omega N]
       [--max-states N]
"""
import hashlib
import json
import sys
import time
from collections import deque
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import conjprep as CP  # noqa: E402
import fastghost as FG  # noqa: E402
import ghostlib as GL  # noqa: E402
from exact import identity  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()
table, tfile, rf = sys.argv[3:6]
a = sys.argv[6:]
TYPES = [int(x) for x in a[a.index('--types') + 1].split(',')]
ROW = int(a[a.index('--row') + 1]) if '--row' in a else 0
MAXO = int(a[a.index('--max-omega') + 1]) if '--max-omega' in a else 400000
MAXS = int(a[a.index('--max-states') + 1]) if '--max-states' in a else 20000
C = GL.Ctx(table)
M, data = C.M, GL.data
kp = GL.BL.CL.ctx.kplus
KID = kp.index_of_matrix(to_tuple(identity(4)))
TYPE = np.asarray(GL.restore.ctx.regions.orbit_of, np.int64)
RARE = np.isin(TYPE, TYPES)
T = np.asarray(sorted(json.loads(Path(tfile).read_text())['pieces']), np.int64)
Tr = T[RARE[T]]
need = {t: int((TYPE[T] == t).sum()) for t in TYPES}
ud, uq = C.contents(Tr)
rows = [json.loads(x) for x in open(rf) if x.strip()]
r = rows[ROW]
W0 = [(int(c), int(i)) for c, i in (r['W'] if 'W' in r else r['Q'])]
W0i = CP.inverse_word(M, W0)


def kletters(c):
    res = []
    for i in range(len(M.items)):
        k = kp.index_of_matrix(GL.restore.element(M, c, i))
        if k >= 0 and k != KID:
            res.append(i)
    return res


def run(word, dom, q):
    g = FG.Ghost(C.F, dom, q)
    for c, i in word:
        g.apply(c, C.E(c, i), (c, i))
    return g


g0 = run(W0, ud, uq)
assert g0.alive.all() and (g0.dom == 0).all(), 'root does not land the rare pieces'
Lr = g0.q.copy()
G = np.flatnonzero(data.sig_matrix(Lr).sum(axis=0) == len(Lr)).tolist()
letters = [(f, i) for f in G for i in kletters(f)]
grips = sorted(set(G) | {c for c, _ in W0})
lat = np.flatnonzero(np.logical_or.reduce([GL.cap(c) for c in grips]) & RARE)
print(json.dumps({'W0': W0, 'L_rare': Lr.tolist(), 'G': G, 'letters': len(letters), 'grips': len(grips),
                  'rare_lattice_in_caps': len(lat)}), flush=True)

# closure
idx = {}
order = []


def add(d, q):
    key = (int(d), int(q))
    if key not in idx:
        idx[key] = len(order)
        order.append(key)
        return True
    return False


for q in lat.tolist():
    add(0, q)
for d, q in zip(ud.tolist(), uq.tolist()):
    add(d, q)
maps = {}
ops = [('W0', W0), ('W0i', W0i)] + [(('k', f, i), [(f, i)]) for f, i in letters]
done_upto = 0
it = 0
while done_upto < len(order):
    it += 1
    lo, hi = done_upto, len(order)
    dom = np.array([order[j][0] for j in range(lo, hi)], np.int64)
    q = np.array([order[j][1] for j in range(lo, hi)], np.int64)
    for name, word in ops:
        g = run(word, dom, q)
        res = maps.setdefault(name, {})
        for j in range(hi - lo):
            if g.alive[j]:
                add(g.dom[j], g.q[j])
                res[lo + j] = (int(g.dom[j]), int(g.q[j]))
            else:
                res[lo + j] = None
    done_upto = hi
    print(json.dumps({'iter': it, 'omega': len(order), 'frames': len(C.F.reps), 'sec': round(time.time() - T0)}),
          flush=True)
    if len(order) > MAXO:
        print(json.dumps({'stop': 'omega too large'}))
        sys.exit(0)
N = len(order)
arr = {}
for name, res in maps.items():
    v = np.full(N, -1, np.int64)
    for j, img in res.items():
        if img is not None:
            v[j] = idx[img]
    arr[name] = v
isl = np.array([d == 0 for d, _ in order])
cham = np.array([q for _, q in order], np.int64)
ctype = TYPE[cham]
Ui = np.array([idx[(d, q)] for d, q in zip(ud.tolist(), uq.tolist())], np.int64)
latidx = np.flatnonzero(isl)


def inv(F):
    G_ = np.full(len(F), -1, np.int64)
    ok = F >= 0
    G_[F[ok]] = np.flatnonzero(ok)
    return G_


def comp(*fs):
    """comp(a, b, c)(x) = c(b(a(x))): apply a first."""
    x = np.arange(N)
    for f in fs:
        y = np.full(N, -1, np.int64)
        ok = x >= 0
        y[ok] = f[x[ok]]
        x = y
    return x


# check: W0i is the partial inverse of W0 on Omega
chk = bool(np.array_equal(inv(arr['W0']), arr['W0i']))
print(json.dumps({'omega': N, 'W0i_is_inverse_of_W0_on_omega': chk}), flush=True)
kinv = {}
for f, i in letters:
    j = int(CP.inverse_word(M, [(f, i)])[0][1])
    kinv[(f, i)] = ('k', f, j)


def stats(F):
    img = F[latidx]
    dead = img < 0
    off = ~dead & ~isl[np.where(dead, 0, img)]
    bad = dead | off
    cnt = {t: int((ctype[latidx][bad] == t).sum()) for t in TYPES}
    dcnt = {t: int((ctype[latidx][dead] == t).sum()) for t in TYPES}
    lu = F[Ui]
    land = bool((lu >= 0).all() and isl[lu].all())
    return cnt, dcnt, land, (cham[lu].tolist() if land else None)


F0 = arr['W0']
seen = {hashlib.sha1(F0.tobytes()).hexdigest(): 0}
q_ = deque([(F0, 0, Lr.tolist())])
st = {'states': 1, 'transitions': 0, 'landing': 0, 'closed_by_dead': 0, 'new_grips': 0, 'swaps': 0, 'dup': 0}
hist = {}
depth_max = 0
while q_ and st['states'] < MAXS:
    F, dep, L = q_.popleft()
    Fi = inv(F)
    for f, i in letters:
        if not all(GL.cap(f)[x] for x in L):
            continue
        K = arr[('k', f, i)]
        Ki = arr[kinv[(f, i)]]
        W = comp(F, K, Fi, Ki)
        st['transitions'] += 1
        cnt, dcnt, land, L2 = stats(W)
        if not land:
            continue
        st['landing'] += 1
        ex = sum(max(0, cnt[t] - need[t]) for t in TYPES)
        hist[ex] = hist.get(ex, 0) + 1
        if ex == 0:
            st['swaps'] += 1
        if sum(dcnt.values()):
            st['closed_by_dead'] += 1
            continue
        G2 = np.flatnonzero(data.sig_matrix(np.array(L2)).sum(axis=0) == len(L2)).tolist()
        if not set(G2) <= set(G):
            st['new_grips'] += 1
            continue
        h = hashlib.sha1(W.tobytes()).hexdigest()
        if h in seen:
            st['dup'] += 1
            continue
        seen[h] = dep + 1
        depth_max = max(depth_max, dep + 1)
        st['states'] += 1
        q_.append((W, dep + 1, L2))
    if st['transitions'] % 2000 < len(letters):
        print(json.dumps({**st, 'queue': len(q_), 'depth': depth_max, 'hist': sorted(hist.items()),
                          'sec': round(time.time() - T0)}), flush=True)
print(json.dumps({'final': True, 'exhausted': not q_, **st, 'depth': depth_max, 'hist': sorted(hist.items()),
                  'omega': N, 'sec': round(time.time() - T0)}), flush=True)
