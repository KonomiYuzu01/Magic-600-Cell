"""Refined single-piece dynamics with the out-grip blocking of Theorem A (an over-approximation of reachability).

State of a piece p: its K+-class of posed region (left coset 2I l) and the set O of out grips (Theorem A: grips whose
inner cohort is anchored) whose caps hold p, written relative to the canonical region of the class. Proved facts used:
(INV, blocking lemma) a twist at d is admissible only if d is not in N15(e) u N25(e) for an out grip e; out grips are
pairwise outside each other's N15 u N25; a jumble at a grip that is not out makes it out; a retained twist does not
change out status; a jumble at an out grip may keep it out or return it. A grip whose cap holds p can change out status
only by a twist that moves p. A grip that newly holds p after p moves may already be out (branch), unless the twist
that moved p was at a grip of its N15 u N25.
usage: refbfs.py <research/jumbling> <kplus orbits|all> <max height> <out json> [--max-states N]"""
import json, sys, time
from collections import Counter, deque
from itertools import combinations
from pathlib import Path
import numpy as np
J = Path(sys.argv[1]).resolve()
for p in (J / 'theory', J, J / 'fixtures'):
    sys.path.insert(0, str(p))
sys.dont_write_bytecode = True
import sim
import trees as T
from exact import Q5
t0 = time.time()
ctx = sim.get_context(); d = ctx.data; reg = ctx.regions
NP = 600
N_ex = [tuple(d.N[c]) for c in range(NP)]
NF = np.array([[float(x) for x in d.N[c]] for c in range(NP)])
NFu = NF / np.linalg.norm(NF, axis=1)[:, None]
KAP = d.KAPPA; KAPf = float(KAP); MARGIN = 1e-7
ang = np.degrees(np.arccos(np.clip(NFu @ NFu.T, -1, 1)))
NB = [set(np.nonzero((np.abs(ang[c] - 15.52) < 0.3) | (np.abs(ang[c] - 25.24) < 0.3))[0].tolist()) for c in range(NP)]
assert all(len(s) == 16 for s in NB), Counter(len(s) for s in NB)
pole_key = {tuple(np.round(NFu[c], 7) + 0.0): c for c in range(NP)}
def qf(q):
    return np.array([float(x) for x in q])
def qinv(q):
    n = T.qnorm(q)
    return tuple(x / n for x in T.qconj(q))
def Lmat(q):
    a0, a1, a2, a3 = q
    return np.array([[a0, -a1, -a2, -a3], [a1, a0, -a3, a2], [a2, a3, a0, -a1], [a3, -a2, a1, a0]])
def Rmat(q):
    b0, b1, b2, b3 = q
    return np.array([[b0, -b1, -b2, -b3], [b1, b0, b3, -b2], [b2, -b3, b0, b1], [b3, b2, -b1, b0]])
UN = T.UNITS
UNf = np.array([qf(u) for u in UN])
unit_keys = {tuple(np.round(qf(u), 9) + 0.0) for u in UN}
# menu of each grip: (left exact, left float, right float, retained flag) for the 60 rotations (u up to sign)
MENU = []
for c in range(NP):
    n = N_ex[c]; ni = qinv(n)
    seen = set(); rows = []
    for u in UN:
        k = tuple(np.round(qf(u), 9) + 0.0)
        if tuple(-x + 0.0 for x in k) in seen:
            continue
        seen.add(k)
        L = T.qmul(T.qmul(n, u), ni)
        Lf = qf(L)
        rows.append((L, Lf, qf(u), tuple(np.round(Lf, 9) + 0.0) in unit_keys))
    assert len(rows) == 60 and sum(r[3] for r in rows) == 12
    MENU.append(rows)
print('menus ready', round(time.time() - t0, 1), flush=True)
def grip_image(uf, vf, cs):
    """k = [u, v]: n -> u n v^-1, applied to grips cs"""
    vinv = vf * np.array([1, -1, -1, -1])
    out = []
    for c in cs:
        y = Lmat(uf) @ (Rmat(vinv) @ NFu[c])
        out.append(pole_key[tuple(np.round(y, 7) + 0.0)])
    return out
def canon(lf):
    P = UNf @ Rmat(lf).T
    Pr = np.round(P, 9) + 0.0
    i = int(np.lexsort(Pr.T[::-1])[0])
    return i, tuple(Pr[i].tolist())

class Orbit:
    def __init__(self, o):
        self.V_ex = reg.rep_vertices[o]
        self.Vf = np.array([[float(x) for x in v] for v in self.V_ex])
        self.cls = {}      # key -> (exact l, height, frozenset caps)
    def caps(self, l):
        lf = qf(l)
        vals = (self.Vf @ Lmat(lf).T) @ NF.T
        mins = vals.min(axis=0) - KAPf
        inn = set(np.nonzero(mins > MARGIN)[0].tolist())
        tie = np.nonzero(np.abs(mins) <= MARGIN)[0]
        if len(tie):
            ex = [T.qmul(l, v) for v in self.V_ex]
            for c in tie.tolist():
                n = N_ex[c]
                if all((sum((n[a] * y[a] for a in range(4)), Q5(0)) - KAP).sign() >= 0 for y in ex):
                    inn.add(c)
        return frozenset(inn)
    def klass(self, l):
        i, key = canon(qf(l))
        if key not in self.cls:
            rep = T.qmul(UN[i], l)
            self.cls[key] = (rep, T.dist(rep), self.caps(rep))
        return i, key

def explore(o, hmax, max_states):
    """state: (class key, O, cpl) with cpl = (grip, start key) when p moved with that grip's cohort since the jumble
    that made it out (then the grip returns exactly when p's class returns to the start class), else None."""
    B = Orbit(o)
    one = (Q5(1), Q5(0), Q5(0), Q5(0))
    _, k0 = B.klass(one)
    s0 = (k0, frozenset(), None)
    seen = {s0}; q = deque([s0]); hist = Counter(); trans = 0
    while q:
        key, O, cpl = q.popleft()
        l, h, D = B.cls[key]
        hist[h] += 1
        if h >= hmax:
            continue
        for dgrip in D:
            if any(dgrip in NB[e] for e in O if e != dgrip):
                continue
            for (HL, HLf, HRf, ret) in MENU[dgrip]:
                nl = T.qmul(HL, l)
                i, k2 = B.klass(nl)
                _, h2, D2 = B.cls[k2]
                uf = UNf[i]
                kv = HRf * np.array([1, -1, -1, -1])
                stay = [e for e in O if e != dgrip]
                img_stay = dict(zip(stay, grip_image(uf, kv, stay)))
                base = {g for g in img_stay.values() if g in D2}
                dk = grip_image(uf, kv, [dgrip])[0]
                outs = []   # (O set, cpl)
                if dgrip in O:
                    if ret:
                        c2 = (dk, cpl[1]) if (cpl is not None and cpl[0] == dgrip) else None
                        outs.append((base | {dk}, c2))
                    elif cpl is not None and cpl[0] == dgrip:
                        if k2 == cpl[1]:
                            outs.append((set(base), None))
                        else:
                            outs.append((base | {dk}, (dk, cpl[1])))
                    else:
                        outs.append((base | {dk}, None))
                        outs.append((set(base), None))
                else:
                    if ret:
                        outs.append((set(base), None))
                    else:
                        outs.append((base | {dk}, (dk, key)))
                Dabs_new = set(grip_image(uf, kv, list(D)))
                newly = [g for g in D2 if g not in Dabs_new]
                for Ob, c2 in outs:
                    cand = [g for g in newly if dk not in NB[g] and all(g not in NB[e] for e in Ob)]
                    subsets = [()]
                    for r in range(1, len(cand) + 1):
                        subsets += [s for s in combinations(cand, r)
                                    if all(b not in NB[a] for a, b in combinations(s, 2))]
                    for s in subsets:
                        st = (k2, frozenset(Ob | set(s)), c2)
                        trans += 1
                        if st not in seen:
                            seen.add(st); q.append(st)
                            if len(seen) >= max_states:
                                return B, seen, hist, trans, False
    return B, seen, hist, trans, True

def main():
    which = sys.argv[2]; hmax = int(sys.argv[3]); out = sys.argv[4]
    max_states = int(sys.argv[sys.argv.index('--max-states') + 1]) if '--max-states' in sys.argv else 300000
    g = reg.report['kplus_orbit_g_classes']
    orbs = range(len(reg.reps)) if which == 'all' else [int(x) for x in which.split(',')]
    res = {}
    for o in orbs:
        if g[str(o)] == [-1]:
            continue
        t1 = time.time()
        B, seen, hist, trans, complete = explore(o, hmax, max_states)
        hc = Counter(v[1] for v in B.cls.values())
        res[str(o)] = {'model_orbit': g[str(o)], 'states': len(seen), 'classes': len(B.cls),
                       'classes_by_height': dict(sorted(hc.items())), 'states_by_height': dict(sorted(hist.items())),
                       'transitions': trans, 'complete': complete, 'sec': round(time.time() - t1, 1)}
        print(o, res[str(o)], flush=True)
        Path(out).write_text(json.dumps(res, indent=1) + '\n')

if __name__ == '__main__':
    main()
