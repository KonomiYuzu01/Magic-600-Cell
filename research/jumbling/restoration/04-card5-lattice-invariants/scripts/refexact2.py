"""Refined single-piece dynamics with exact cohort elements (certificate version; menu I_a).

Extends refexact.py. For every out grip c whose cap holds p the state may carry the exact accumulated element
G_c in Lambda_c (anchoring element of Theorem A's cohort: the cohort of c is exactly G_c(S_c), S_c the inner slots of
cap c). It is known when the jumble that made c out was seen (p was in cap c then) and every later twist at c moved p,
which holds while p stays in cap c. With G_c known:
  - a twist (c, h) returns c (cohort back on the lattice) iff h G_c lies in A4_c (Theorem A, case iii), else
    G_c := h G_c;
  - twists at the grips straddled by the inner cohort G_c(S_c) are inadmissible (certificate INNER-STRADDLE: N15 u N25
    plus 6 grips at 36 degrees that depend on the coset G_c A4_c; transported from grip 0 by K+).
With G_c unknown (an out grip that newly holds p): only N15 u N25 block, a jumble at c may keep or return it.
G_c is stored by its right factor u (G_c = [n u n^-1, u]) in the frame of the canonical region of the class.
usage: refexact2.py <research/jumbling> <orbits|all> <max height> <out json> <innerstraddle json> [--max-states N]"""
import json, sys, time, hashlib
from collections import Counter, deque
from itertools import combinations
from pathlib import Path
import numpy as np
sys.path.insert(0, str(Path(__file__).resolve().parent))
import refexact as RX

class Shapes:
    """Exact data for cohort elements: returns, coset, inner straddle sets."""
    def __init__(self, C, istr):
        self.C = C
        T, kp = C.T, C.kp
        import restore as RS
        from exact import matmul, transpose
        from sim.kplus import to_tuple
        self.matmul, self.transpose, self.to_tuple = matmul, transpose, to_tuple
        M = RS.menu('I_a')
        self.reps = [int(x) for x in istr['reps']]
        self.repmat = {x: RS.element(M, 0, x) for x in self.reps}
        self.S36 = {x: set(int(e) for e in istr['by_coset'][str(x)]['inner_straddled']) - C.NB[0] for x in self.reps}
        self.pun = {T.qkey(u) for u in T.PUNITS}
        self._ret, self._blk = {}, {}
    def gmat(self, c, u):
        """exact matrix of [n u n^-1, u] (x -> n u n^-1 x u^-1)."""
        C, T = self.C, self.C.T
        n = C.N[c]; nn = C.dot(n, n)
        L = T.qmul(T.qmul(n, u), tuple(x / nn for x in T.qconj(n)))
        ui = T.qconj(u)
        cols = [T.qmul(T.qmul(L, e), ui) for e in C.E4]
        return tuple(tuple(cols[b][a] for b in range(4)) for a in range(4)), L
    def returns(self, c, u):
        """G = [n u n^-1, u] in K+ ?"""
        key = (c, u)
        r = self._ret.get(key)
        if r is None:
            C, T = self.C, self.C.T
            n = C.N[c]; nn = C.dot(n, n)
            L = T.qmul(T.qmul(n, u), tuple(x / nn for x in T.qconj(n)))
            r = T.qkey(L) in self.pun
            self._ret[key] = r
        return r
    def blocked(self, c, u):
        """grips blocked by the inner cohort of out grip c with element [n u n^-1, u]: N15 u N25 u S36."""
        key = (c, u)
        b = self._blk.get(key)
        if b is None:
            C = self.C
            G, _ = self.gmat(c, u)
            k = C.to_grip[c]
            K = C.kp.matrix(k)
            Gp = self.matmul(self.matmul(self.transpose(K), G), K)     # in Lambda_0
            sig = None
            for x in self.reps:
                if C.kp.index_of_matrix(self.to_tuple(self.matmul(self.transpose(self.repmat[x]), Gp))) >= 0:
                    sig = x
                    break
            assert sig is not None, 'cohort element not a jumble at its grip'
            b = frozenset(C.NB[c] | {int(C.perms[k][e]) for e in self.S36[sig]})
            self._blk[key] = b
        return b

def explore(C, SH, o, hmax, max_states, log=None, log_every=0):
    T, Q5 = C.T, C.Q5
    B = RX.Orbit(C, o)
    one = (Q5(1), Q5(0), Q5(0), Q5(0))
    k0, _ = B.klass(one)
    TR = {}
    def trans_of(j, dg):
        r = TR.get((j, dg))
        if r is None:
            r = []
            for ri, (L, Rt, ret) in enumerate(C.MENU[dg]):
                j2, i = B.klass(T.qmul(L, B.rep[j]))
                r.append((ri, j2, C.kidx_of(i, Rt)))
            TR[(j, dg)] = r
        return r
    qk = T.qkey
    def conj_by(u, Rt):          # Rt^-1 u Rt (frame change of right factors)
        return qk(T.qmul(T.qmul(T.qconj(Rt), u), Rt))
    s0 = (k0, frozenset())
    seen = {s0}; q = deque([s0]); hist = Counter(); ntrans = 0; t0 = time.time()
    allowed_in, allowed_out = {}, {}
    complete = True
    while q:
        j, O = q.popleft()
        Od = dict(O)
        h, D = B.height[j], B.caps[j]
        hist[h] += 1
        blk = set()
        for e, u in Od.items():
            blk |= (SH.blocked(e, u) if u is not None else C.NB[e])
        ai = allowed_in.setdefault(j, set()); ao = allowed_out.setdefault(j, set())
        for dg in B.outcaps[j]:
            if dg not in blk:
                ao.add(dg)
        if h >= hmax:
            complete = False
            continue
        for dg in D:
            # blocked by out grips other than dg (dg's own cohort lies in cap dg and does not straddle it)
            bo = set()
            for e, u in Od.items():
                if e != dg:
                    bo |= (SH.blocked(e, u) if u is not None else C.NB[e])
            if dg in bo:
                continue
            ai.add(dg)
            for ri, j2, kk in trans_of(j, dg):
                L, Rt, ret = C.MENU[dg][ri]
                D2 = B.caps[j2]
                pk = C.perms[kk]
                base = {}
                for e, u in Od.items():
                    if e == dg:
                        continue
                    e2 = int(pk[e])
                    if e2 in D2:
                        base[e2] = conj_by(u, Rt) if u is not None else None
                dk = int(pk[dg])
                outs = []
                if dg in Od:
                    u = Od[dg]
                    if u is None:
                        if ret:
                            outs.append({**base, dk: None})
                        else:
                            outs.append({**base, dk: None}); outs.append(dict(base))
                    else:
                        un = qk(T.qmul(Rt, u))                  # h G
                        if SH.returns(dg, un):
                            outs.append(dict(base))
                        else:
                            outs.append({**base, dk: conj_by(un, Rt)})
                else:
                    if ret:
                        outs.append(dict(base))
                    else:
                        outs.append({**base, dk: conj_by(qk(Rt), Rt)})
                img_D = {int(pk[e]) for e in D}
                newly = [g for g in D2 if g not in img_D]
                for Ob in outs:
                    bl = set()
                    for e, u in Ob.items():
                        bl |= (SH.blocked(e, u) if u is not None else C.NB[e])
                    # a newly holding grip g can be out only if the twist at dg was admissible with g out
                    # (dg not in NB[g]) and g is not blocked by, nor blocks, the out grips of the state
                    cand = [g for g in newly if dk not in C.NB[g] and g not in bl and all(g not in C.NB[e] and e not in C.NB[g] for e in Ob)]
                    subsets = [()]
                    for r in range(1, len(cand) + 1):
                        subsets += [s for s in combinations(cand, r)
                                    if all(b not in C.NB[a] for a, b in combinations(s, 2))]
                    for s in subsets:
                        On = dict(Ob)
                        for g in s:
                            On[g] = None
                        st = (j2, frozenset(On.items()))
                        ntrans += 1
                        if st not in seen:
                            seen.add(st); q.append(st)
                            if log and log_every and len(seen) % log_every == 0:
                                log({'orbit': o, 'states': len(seen), 'classes': len(B.rep), 'queue': len(q),
                                     'by_height': dict(sorted(Counter(B.height).items())),
                                     'sec': round(time.time() - t0, 1)})
                            if len(seen) >= max_states:
                                complete = False
                                q.clear()
                                break
    B.distinct_check()
    R = RX.Result()
    R.B, R.seen, R.hist, R.ntrans, R.complete, R.TR = B, seen, hist, ntrans, complete, TR
    R.allowed_in, R.allowed_out = allowed_in, allowed_out
    R.digest = hashlib.sha256(json.dumps(sorted([str(tuple(str(x) for x in T.qkey(r))) for r in B.rep])).encode()).hexdigest()
    return R

if __name__ == '__main__':
    jdir, which, hmax, out, istr = sys.argv[1], sys.argv[2], int(sys.argv[3]), sys.argv[4], sys.argv[5]
    maxs = int(sys.argv[sys.argv.index('--max-states') + 1]) if '--max-states' in sys.argv else 2000000
    C = RX.Ctx(jdir)
    SH = Shapes(C, json.load(open(istr)))
    g = C.reg.report['kplus_orbit_g_classes']
    orbs = [o for o in range(len(C.reg.reps)) if g[str(o)] != [-1]] if which == 'all' else [int(x) for x in which.split(',')]
    res = {}
    for o in orbs:
        t1 = time.time()
        R = explore(C, SH, o, hmax, maxs, log=lambda r: print(json.dumps(r), flush=True), log_every=50000)
        res[str(o)] = RX.summary(C, R, o)
        res[str(o)]['sec'] = round(time.time() - t1, 1)
        print(json.dumps(res[str(o)]), flush=True)
        Path(out).write_text(json.dumps(res, indent=1) + '\n')
