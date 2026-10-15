"""Certificate version of the refined single-piece dynamics (menu I_a): the transition system of refbfs2 with exact
class identification.

A state of a piece p is (class, O, cpl): the class is the K+-class of p's posed region, i.e. the coset 2I l of the left
factor l of [l, 1] with region [l, 1](A_rep) (right factors are units, Section 1.1 of PROOFS); O is the set of out
grips (Theorem A) whose caps hold p, written in the frame of the canonical region of the class; cpl = (grip, start
class) when p has moved only with that out grip's cohort since the jumble that made the grip out. Transitions use only
proved facts: INV and the blocking lemma (no twist at a grip of N15 u N25 of an out grip), a jumble at a grip that is
not out makes it out, a retained twist keeps the status, an out grip returns exactly when the product of the twists
at it since its jumble lies in K+, which for a coupled piece is exactly "the class is the start class again".

Exactness: every class has an exact unit-quaternion representative rep (Q(sqrt5)); each transition's target class is
confirmed exactly (u_i * h_L * rep_source equals +-rep_target), grip images come from exact K+ permutations, caps
from exact evaluation within a float margin, NB sets from exact inner products, and at the end no two classes lie
in one coset (exact). Floating point is used only as a hash key.
usage (library): explore(o, hmax, max_states) -> Result"""
import json, sys, time, hashlib
from collections import Counter, deque
from itertools import combinations
from pathlib import Path
import numpy as np

def setup(jdir):
    J = Path(jdir).resolve()
    for p in (J / 'theory', J, J / 'fixtures'):
        sys.path.insert(0, str(p))
    sys.dont_write_bytecode = True

class Ctx:
    def __init__(self, jdir):
        setup(jdir)
        import sim
        import trees as T
        from exact import Q5
        self.sim, self.T, self.Q5 = sim, T, Q5
        ctx = sim.get_context(); self.ctx = ctx
        self.d, self.reg, self.kp = ctx.data, ctx.regions, ctx.kplus
        d, kp, T = self.d, self.kp, self.T
        self.NP = 600
        self.N = [tuple(d.N[c]) for c in range(600)]
        self.KAP = d.KAPPA
        self.NF = np.array([[float(x) for x in n] for n in self.N])
        self.NFu = self.NF / np.linalg.norm(self.NF, axis=1)[:, None]
        self.KAPf = float(self.KAP)
        self.perms = kp.perms.astype(np.int64)
        Z = Q5(0)
        dot = lambda a, b: sum((a[i] * b[i] for i in range(4)), Z)
        self.dot = dot
        # exact NB sets: N15(0) u N25(0) from the exact inner products with grip 0, transported by K+
        ang0 = np.degrees(np.arccos(np.clip(self.NFu @ self.NFu[0], -1, 1)))
        vals = {}
        for c in range(600):
            vals.setdefault(dot(self.N[0], self.N[c]), []).append(c)
        near = [v for v, cs in vals.items() if any(abs(ang0[c] - 15.52) < 0.3 or abs(ang0[c] - 25.24) < 0.3 for c in cs)]
        assert len(near) == 2, 'two exact inner products at 15.52 and 25.24 degrees'
        nb0 = sorted(c for v in near for c in vals[v])
        assert len(nb0) == 16
        self.to_grip = {}
        for k in range(len(self.perms)):
            c = int(self.perms[k][0])
            if c not in self.to_grip:
                self.to_grip[c] = k
        assert len(self.to_grip) == 600
        self.NB = [set(int(x) for x in self.perms[self.to_grip[c]][nb0]) for c in range(600)]
        # menus: rows (L exact, R exact, retained) for the 60 units up to sign, in a fixed order
        self.PUN = T.PUNITS
        pun_keys = {T.qkey(u) for u in self.PUN}
        self.UN = list(T.UNITS)
        self.UNf = np.array([[float(x) for x in u] for u in self.UN])
        self.MENU = []
        for c in range(600):
            n = self.N[c]
            nn = dot(n, n)
            ni = tuple(x / nn for x in T.qconj(n))
            rows = []
            for u in self.PUN:
                L = T.qmul(T.qmul(n, u), ni)
                rows.append((L, u, T.qkey(L) in pun_keys))
            assert sum(r[2] for r in rows) == 12
            self.MENU.append(rows)
        self._kidx = {}
        self.E4 = [tuple(Q5(1 if k == b else 0) for k in range(4)) for b in range(4)]

    def kidx_of(self, ui, v):
        """K+ index of x -> UN[ui] x v (v a unit icosian), exact."""
        key = (ui, self.T.qkey(v), v[0] if not v[0].is_zero() else None)
        k = self._kidx.get((ui, v))
        if k is None:
            u = self.UN[ui]
            cols = [self.T.qmul(self.T.qmul(u, e), v) for e in self.E4]
            M = tuple(tuple(cols[b][a] for b in range(4)) for a in range(4))
            k = self.kp.index_of_matrix(M)
            assert k >= 0
            self._kidx[(ui, v)] = k
        return k

def lmat(q):
    a0, a1, a2, a3 = q
    return np.array([[a0, -a1, -a2, -a3], [a1, a0, -a3, a2], [a2, a3, a0, -a1], [a3, -a2, a1, a0]])

def rmat(q):
    b0, b1, b2, b3 = q
    return np.array([[b0, -b1, -b2, -b3], [b1, b0, b3, -b2], [b2, -b3, b0, b1], [b3, b2, -b1, b0]])

class Orbit:
    """Classes of posed regions of one K+-orbit; exact representatives."""
    MARGIN = 1e-7
    def __init__(self, C, o):
        self.C, self.o = C, o
        self.V = C.reg.rep_vertices[o]
        self.Vf = np.array([[float(x) for x in v] for v in self.V])
        self.cls = {}          # float key -> index
        self.rep, self.height, self.caps, self.outcaps = [], [], [], []
        self.checked = 0
    def _canon(self, lf):
        P = self.C.UNf @ rmat(lf).T
        Pr = np.round(P, 9) + 0.0
        i = int(np.lexsort(Pr.T[::-1])[0])
        return i, tuple(Pr[i].tolist())
    def _capsets(self, l):
        C, T, Q5 = self.C, self.C.T, self.C.Q5
        lf = np.array([float(x) for x in l])
        vals = (self.Vf @ lmat(lf).T) @ C.NF.T
        mins = vals.min(axis=0) - C.KAPf
        maxs = vals.max(axis=0) - C.KAPf
        inn = set(np.nonzero(mins > self.MARGIN)[0].tolist())
        out = set(np.nonzero(maxs < -self.MARGIN)[0].tolist())
        ex = None
        for c in np.nonzero(np.abs(mins) <= self.MARGIN)[0].tolist():
            ex = ex or [T.qmul(l, v) for v in self.V]
            if all((C.dot(C.N[c], y) - C.KAP).sign() >= 0 for y in ex):
                inn.add(c)
        for c in np.nonzero(np.abs(maxs) <= self.MARGIN)[0].tolist():
            ex = ex or [T.qmul(l, v) for v in self.V]
            if all((C.dot(C.N[c], y) - C.KAP).sign() <= 0 for y in ex):
                out.add(c)
        return frozenset(inn), frozenset(out)
    def klass(self, l):
        """(index of the class of l, i with UN[i] l = +-rep exactly)."""
        T = self.C.T
        lf = np.array([float(x) for x in l])
        i, key = self._canon(lf / np.linalg.norm(lf))
        P = T.qmul(self.C.UN[i], l)
        j = self.cls.get(key)
        if j is None:
            j = len(self.rep)
            self.cls[key] = j
            self.rep.append(P)
            self.height.append(T.dist(P))
            a, b = self._capsets(P)
            self.caps.append(a); self.outcaps.append(b)
        else:
            if T.qkey(P) != T.qkey(self.rep[j]):
                raise AssertionError('float class key collision')
            self.checked += 1
        return j, i
    def distinct_check(self):
        """No two classes in one coset 2I l (exact)."""
        T = self.C.T
        idx = {T.qkey(r): j for j, r in enumerate(self.rep)}
        assert len(idx) == len(self.rep)
        for j, r in enumerate(self.rep):
            for u in self.C.UN:
                k = idx.get(T.qkey(T.qmul(u, r)))
                if k is not None and k != j:
                    raise AssertionError(f'classes {j} and {k} coincide')
        return True

class Result:
    pass

def explore(C, o, hmax, max_states, log=None, log_every=0):
    B = Orbit(C, o)
    T, Q5 = C.T, C.Q5
    one = (Q5(1), Q5(0), Q5(0), Q5(0))
    k0, _ = B.klass(one)
    assert B.height[k0] == 0
    TR = {}        # (class, grip) -> list of (row, class2, frame K+ index)
    def trans_of(j, dg):
        r = TR.get((j, dg))
        if r is None:
            r = []
            for ri, (L, Rt, ret) in enumerate(C.MENU[dg]):
                j2, i = B.klass(T.qmul(L, B.rep[j]))
                r.append((ri, j2, C.kidx_of(i, Rt)))
            TR[(j, dg)] = r
        return r
    s0 = (k0, frozenset(), None)
    seen = {s0}; q = deque([s0]); hist = Counter(); ntrans = 0; t0 = time.time()
    allowed_in, allowed_out = {}, {}
    complete = True
    while q:
        j, O, cpl = q.popleft()
        h, D = B.height[j], B.caps[j]
        hist[h] += 1
        ai = allowed_in.setdefault(j, set())
        ao = allowed_out.setdefault(j, set())
        for dg in B.outcaps[j]:
            if not any(dg in C.NB[e] for e in O):
                ao.add(dg)
        if h >= hmax:
            complete = False
            continue
        for dg in D:
            if any(dg in C.NB[e] for e in O if e != dg):
                continue
            ai.add(dg)
            for ri, j2, kk in trans_of(j, dg):
                ret = C.MENU[dg][ri][2]
                D2 = B.caps[j2]
                pk = C.perms[kk]
                stay = [e for e in O if e != dg]
                base = {int(pk[e]) for e in stay} & D2
                dk = int(pk[dg])
                outs = []
                if dg in O:
                    if ret:
                        outs.append((base | {dk}, (dk, cpl[1]) if (cpl is not None and cpl[0] == dg) else None))
                    elif cpl is not None and cpl[0] == dg:
                        outs.append((set(base), None) if j2 == cpl[1] else (base | {dk}, (dk, cpl[1])))
                    else:
                        outs.append((base | {dk}, None))
                        outs.append((set(base), None))
                else:
                    outs.append((set(base), None) if ret else (base | {dk}, (dk, j)))
                img_D = {int(pk[e]) for e in D}
                newly = [g for g in D2 if g not in img_D]
                for Ob, c2 in outs:
                    cand = [g for g in newly if dk not in C.NB[g] and all(g not in C.NB[e] for e in Ob)]
                    subsets = [()]
                    for r in range(1, len(cand) + 1):
                        subsets += [s for s in combinations(cand, r)
                                    if all(b not in C.NB[a] for a, b in combinations(s, 2))]
                    for s in subsets:
                        st = (j2, frozenset(Ob | set(s)), c2)
                        ntrans += 1
                        if st not in seen:
                            seen.add(st); q.append(st)
                            if log and log_every and len(seen) % log_every == 0:
                                log({'orbit': o, 'states': len(seen), 'classes': len(B.rep), 'queue': len(q),
                                     'max_height': max(B.height), 'sec': round(time.time() - t0, 1)})
                            if len(seen) >= max_states:
                                complete = False
                                q.clear()
                                break
    B.distinct_check()
    R = Result()
    R.B, R.seen, R.hist, R.ntrans, R.complete, R.TR = B, seen, hist, ntrans, complete, TR
    R.allowed_in, R.allowed_out = allowed_in, allowed_out
    R.digest = hashlib.sha256(json.dumps(sorted([str(tuple(str(x) for x in T.qkey(r))) for r in B.rep])).encode()).hexdigest()
    return R

def summary(C, R, o):
    B = R.B
    return {'kplus_orbit': o, 'model_orbit': C.reg.report['kplus_orbit_g_classes'][str(o)], 'complete': R.complete,
            'states': len(R.seen), 'classes': len(B.rep), 'classes_by_height': dict(sorted(Counter(B.height).items())),
            'states_by_height': dict(sorted(R.hist.items())), 'transitions': R.ntrans,
            'class_transitions_exact': sum(len(v) for v in R.TR.values()), 'exact_key_confirmations': B.checked,
            'max_height': max(B.height), 'class_digest': R.digest}

if __name__ == '__main__':
    jdir, which, hmax, out = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4]
    maxs = int(sys.argv[sys.argv.index('--max-states') + 1]) if '--max-states' in sys.argv else 2000000
    C = Ctx(jdir)
    g = C.reg.report['kplus_orbit_g_classes']
    orbs = [o for o in range(len(C.reg.reps)) if g[str(o)] != [-1]] if which == 'all' else [int(x) for x in which.split(',')]
    res = {}
    for o in orbs:
        t1 = time.time()
        R = explore(C, o, hmax, maxs, log=lambda r: print(json.dumps(r), flush=True), log_every=100000)
        res[str(o)] = summary(C, R, o)
        res[str(o)]['sec'] = round(time.time() - t1, 1)
        print(json.dumps(res[str(o)]), flush=True)
        Path(out).write_text(json.dumps(res, indent=1) + '\n')
