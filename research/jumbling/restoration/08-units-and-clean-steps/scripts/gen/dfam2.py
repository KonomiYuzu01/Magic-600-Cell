"""D-type block word search for one target block of a state (menu I_a), read from the pose table only.

Template (A-B-closure.md, D-X17.md): W = P Q P^-1 with Q = [A, [B, X]] and P = [[X, Y], Z].
1. Q candidates (block conditions G2, exact cut tests on T's own pieces): e an unstraddled grip whose cap holds all of
   T, B a non-identity K+ letter at e; c an unstraddled grip with T outside cap c and B(T) inside cap c; X a letter at
   c outside K+ with X^-1 B m in K+ for every pose m of T (all poses of T must lie in one domain m K+); the grip e a
   pole of X's frame; a an unstraddled grip whose cap holds T, A a non-identity K+ letter at a with A(T) outside caps
   c and e. Q = A, B, X, B^-1, X^-1, A^-1, X, B, X^-1, B^-1.
2. Each Q is scored by the exact frame ghost: T ends on the lattice, every other off piece (protected) returns to its
   exact content and never straddles, and Bad(Q) with its occupancy at the state.
3. Parking for the best Q's (smallest Bad(Q) among those with T on K and the protected pieces exact): Y a non-identity
   K+ letter at a cap p that holds all of Bad(Q) (p != c, p a pole of X's frame), Z a letter outside K+ with
   Z n_e = n_c (and Z n_p = n_p unless --relaxed); P = X, Y, X^-1, Y^-1, Z, Y, X, Y^-1, X^-1, Z^-1 = [[X, Y], Z].
   W = P Q P^-1 is scored by the ghost in the same way.
Every decision is exact (Q(sqrt5) products, exact cut classification, the exact frame ghost).

--free <json {pieces}> (dfam2): pieces of the target block outside T (its other frames) that the word may move: they
are not protected, only required to stay admissible (their ghost alive); rows record free_dead and free_on.

usage: dfam2.py <theory dir> <E-chain scripts dir> <pose table> <target json {pieces}> <out prefix>
       [--max-q N] [--good-q K] [--park-q K] [--relaxed] [--max-park N] [--no-pole]
--no-pole drops the condition that e is a pole of X's frame (Lemma Q does not use it; it keeps Bad(Q) small). Such
candidates are scored after the others. --commutator adds the C-type words [X, B] = X, B, X^-1, B^-1 (the second half
of Q; T reaches K under the same conditions without A, so one mover suffices), scored after the D-type words; their
rows carry a = A = -1.
--c-only keeps only the C-type words (implies nothing else).
--adm-T counts a grip as unstraddled unless T itself straddles it (other off pieces ignored): the family then holds
every word that the same rule gives at any later state where T is unchanged (Bad(W), the landing of T and the cut
statuses of T do not depend on the other off pieces). --only-new keeps only the candidates with a grip e, c or a that
is straddled at the given state (the words added by --adm-T).
--blocks <json> skips the block computation of ghostlib.Ctx (slow on very large off sets): the block list is read from
the file ({'blocks': [{'pieces': [...]}, ...]}, as written by mkblocks.py); blocks with a piece no longer off are
dropped. Block indices in conflict sets then refer to that file.
"""
import json
import sys
import time
from collections import defaultdict
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import ghostlib as GL  # noqa: E402
from exact import matmul, matvec, transpose  # noqa: E402
from sim.kernel import Evaluator, pullback_form  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()
CTX = GL.BL.CL.ctx
data, kp, reg = CTX.data, CTX.kplus, CTX.regions


class _Blocks:
    def __init__(self, blocks):
        self.blocks = blocks


class Lite(GL.Ctx):
    """ghostlib.Ctx with the block list read from a file instead of computed (same other fields)."""

    def __init__(self, table, bfile):  # noqa: D401
        self.R, self.M = GL.O.start(table)
        self.st = self.R.st
        self.D = GL.rim.Domains(self.st)
        self.F = GL.FG.Frames()
        self.dmap = {j: self.F.domain_of(rep) for j, rep in enumerate(self.D.reps)}
        lat = np.flatnonzero(self.D.dom == 0)
        self.occ = np.zeros(GL.NP, bool)
        self.occ[self.D.cham[lat]] = True
        self.off = np.flatnonzero(self.st._kpose()[self.st.pose_id] < 0)
        self._el = {}
        offset = set(self.off.tolist())
        bl = json.loads(Path(bfile).read_text())['blocks']
        self.bl = _Blocks([np.asarray(sorted(b['pieces']), np.int64) if all(int(p) in offset for p in b['pieces'])
                           else np.zeros(0, np.int64) for b in bl])
        self._cdom = None

    def contents(self, pieces):
        """Same values as ghostlib.Ctx.contents, computed once for all off pieces (it is called with off pieces only)."""
        if self._cdom is None:
            dom = np.full(GL.NP, -1, np.int64)
            q = np.full(GL.NP, -1, np.int64)
            for p in self.off.tolist():
                d2, k = self.dmap[int(self.D.dom[p])]
                dom[p] = d2
                q[p] = int(GL.FG.chamber_image(k, np.array([self.D.cham[p]]))[0])
            self._cdom, self._cq = dom, q
        pieces = np.asarray(pieces, np.int64)
        assert (self._cdom[pieces] >= 0).all(), 'contents of a lattice piece requested'
        return self._cdom[pieces].copy(), self._cq[pieces].copy()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def arg(a, name, default, typ=int):
    return typ(a[a.index(name) + 1]) if name in a else default


def main():
    a = sys.argv[1:]
    table, tfile, prefix = a[2], a[3], a[4]
    MAXQ, GOODQ, PARKQ = arg(a, '--max-q', 4000), arg(a, '--good-q', 40), arg(a, '--park-q', 6)
    MAXP = arg(a, '--max-park', 3000)
    MINQ = arg(a, '--min-q', 400)
    relaxed = '--relaxed' in a
    if '--blocks' in a:
        C = Lite(table, a[a.index('--blocks') + 1])
    else:
        C = GL.Ctx(table)
    R, M, st, E = C.R, C.M, C.st, C.E
    n_items = len(M.items)
    T = np.asarray(sorted(json.loads(Path(tfile).read_text())['pieces']), np.int64)
    assert np.isin(T, C.off).all()
    FREE = np.asarray(sorted(json.loads(Path(a[a.index('--free') + 1]).read_text())['pieces']), np.int64)         if '--free' in a else np.zeros(0, np.int64)
    prot = np.setdiff1d(np.setdiff1d(C.off, T), FREE)
    d0, q0 = C.contents(prot)
    blk_of = {}
    for j_, b_ in enumerate(C.bl.blocks):
        for p_ in b_:
            blk_of[int(p_)] = j_
    prot_blk = np.array([blk_of.get(int(p_), -1) for p_ in prot.tolist()], np.int64)
    groups = {}
    for p in T.tolist():
        g = st.pose(int(p))
        groups.setdefault(to_tuple(g), (g, []))[1].append(int(p))
    groups = list(groups.values())
    m = groups[0][0]
    one_domain = all(kp.index_of_matrix(to_tuple(matmul(transpose(m), g))) >= 0 for g, _ in groups)
    inK = lambda g: kp.index_of_matrix(to_tuple(g)) >= 0  # noqa: E731
    ident = to_tuple(E(0, 0))
    inv_cache = {}

    def inv(c, i):
        if (c, i) not in inv_cache:
            t = to_tuple(transpose(E(c, i)))
            inv_cache[(c, i)] = next(j for j in range(n_items) if to_tuple(E(c, j)) == t)
        return inv_cache[(c, i)]

    def pole(g, e):
        return data.POLE.get(tuple(matvec(transpose(g), data.N[e])), -1)

    def img(g, e):
        return data.POLE.get(tuple(matvec(g, data.N[e])), -1)

    kitems = {}

    def kit(c):
        if c not in kitems:
            kitems[c] = [i for i in range(n_items) if inK(E(c, i)) and to_tuple(E(c, i)) != ident]
        return kitems[c]

    st_cache = {}

    def status(h, cuts):
        """cut -> 0 outside, 1 inside, 2 straddle, 3 both sides, for all of T moved by the left factor h."""
        key = to_tuple(h)
        res = st_cache.setdefault(key, {})
        todo = [e for e in cuts if e not in res]
        if todo:
            mats = [(matmul(h, g), pcs) for g, pcs in groups]
            for e in todo:
                seen = set()
                for G, pcs in mats:
                    ev = Evaluator(pullback_form(G, data.N[e]))
                    for p in pcs:
                        r = ev.classify(reg.vform(int(p)))[0]
                        seen.add(r)
                        if r == 'straddle':
                            break
                    if 'straddle' in seen:
                        break
                res[e] = 2 if 'straddle' in seen else (3 if len(seen) > 1 else (1 if 'in' in seen else 0))
        return res

    adm = np.array([R.inside(c) is not None for c in range(600)])
    # float pre-filter of caps that might hold all of T (exact status decides)
    NF = np.array([[float(x) for x in data.N[c]] for c in range(600)])
    KAP = float(data.KAPPA)
    fm = lambda mm: np.array([[float(v) for v in row] for row in mm])  # noqa: E731
    Vs = []
    for g, pcs in groups:
        for p in pcs:
            V = np.array([[float(x) for x in v] for v in reg.rep_vertices[int(reg.orbit_of[p])]])
            V = V @ fm(kp.matrix(int(reg.transport[p]))).T @ fm(g).T
            Vs.append(V)
    Vall = np.concatenate(Vs)

    def maybe_inside(h):
        W_ = Vall @ fm(h).T
        lo = (W_ @ NF.T - KAP).min(0)
        return np.flatnonzero(lo > -1e-6).tolist()

    s0 = status(E(0, 0), list(range(600)))
    adm_state = adm.copy()
    if '--adm-T' in a:
        adm = np.array([s0[c] != 2 for c in range(600)])
    inside0 = [c for c in range(600) if s0[c] == 1 and adm[c]]
    straddled = [c for c in range(600) if s0[c] == 2]
    log(T=len(T), poses=len(groups), one_domain=one_domain, prot=len(prot), holes=int((~C.occ).sum()),
        inside_admissible=inside0, straddled_by_T=straddled)
    summary = {'table': table, 'target': T.tolist(), 'poses': len(groups), 'one_domain': one_domain,
               'inside_admissible': inside0, 'straddled_by_T': straddled}
    if not one_domain or not inside0:
        summary['stop'] = 'poses in several domains' if not one_domain else 'no unstraddled cap holds T'
        Path(prefix + '.summary.json').write_text(json.dumps(summary, indent=1) + '\n')
        log(stop=summary['stop'])
        return
    # Q candidates
    cands = []
    incompatible = 0
    for e in inside0:
        for Bi in kit(e):
            Bm = E(e, Bi)
            gB = matmul(Bm, m)
            near = maybe_inside(Bm)
            sB = status(Bm, near)
            for c in [cc for cc in near if sB.get(cc) == 1 and s0[cc] == 0 and adm[cc]]:
                Xs = [i for i in range(n_items) if not inK(E(c, i)) and inK(matmul(transpose(E(c, i)), gB))]
                if not Xs:
                    continue
                nopole = pole(E(c, Xs[0]), e) < 0
                if nopole:
                    incompatible += 1
                    if '--no-pole' not in a:
                        continue
                if '--commutator' in a and ('--only-new' not in a or not (adm_state[e] and adm_state[c])):
                    for Xi in Xs:
                        cands.append((e, Bi, c, Xi, -1, -1, nopole))
                for a_ in ([] if '--c-only' in a else inside0):
                    if '--only-new' in a and adm_state[e] and adm_state[c] and adm_state[a_]:
                        continue
                    for Ai in kit(a_):
                        sA = status(E(a_, Ai), [c, e])
                        if sA[c] != 0 or sA[e] != 0:
                            continue
                        for Xi in Xs:
                            cands.append((e, Bi, c, Xi, a_, Ai, nopole))
    log(q_candidates=len(cands), incompatible=incompatible)
    summary['q_candidates'] = len(cands)
    summary['c_type_candidates'] = sum(1 for cd in cands if cd[4] < 0)
    # involutions A and B first (the shape of the D17 and D87 words), spread over (e, c, a) groups
    def is_inv(c, i):
        return inv(c, i) == i
    by = defaultdict(list)
    for cd in cands:
        e, Bi, c, Xi, a_, Ai, nopole = cd
        inv_ok = is_inv(e, Bi) and (a_ < 0 or is_inv(a_, Ai))
        pri = (0 if inv_ok else 1) + (2 if nopole else 0) + (4 if a_ < 0 else 0)
        by[(pri, e, c, a_)].append(cd[:6])
    order = []
    for pri in range(8):
        keys = [k_ for k_ in by if k_[0] == pri]
        k = 0
        while any(k < len(by[key]) for key in keys) and len(order) < MAXQ:
            for key in keys:
                if k < len(by[key]):
                    order.append(by[key][k])
            k += 1
    order = order[:MAXQ]
    summary['q_involution_candidates'] = sum(len(v) for k_, v in by.items() if k_[0] == 0)
    good, rows = [], []
    with open(prefix + '.q.jsonl', 'w') as fh:
        for n, (e, Bi, c, Xi, a_, Ai) in enumerate(order):
            J = [(e, Bi), (c, Xi), (e, inv(e, Bi)), (c, inv(c, Xi))]
            Jinv = [(c, Xi), (e, Bi), (c, inv(c, Xi)), (e, inv(e, Bi))]
            Q = Jinv if a_ < 0 else [(a_, Ai)] + J + [(a_, inv(a_, Ai))] + Jinv
            g, nP, U = C.ghost(Q, np.concatenate([T, prot, FREE]))
            nT = len(T)
            nF = len(FREE)
            fa = g.alive[nP - nF:nP]
            free_on = int((fa & (g.dom[nP - nF:nP] == 0)).sum())
            nP = nP - nF
            pa = g.alive[nT:nP]
            mv = pa & ((g.dom[nT:nP] != d0) | (g.q[nT:nP] != q0))
            row = {'e': e, 'B': Bi, 'c': c, 'X': Xi, 'a': a_, 'A': Ai, 'Q': [list(x) for x in Q],
                   'T_on': int((g.alive[:nT] & (g.dom[:nT] == 0)).sum()), 'T_dead': int((~g.alive[:nT]).sum()),
                   'prot_dead': int((~pa).sum()), 'prot_moved': int(mv.sum()),
                   'conflict_blocks': sorted(set(prot_blk[~pa | mv].tolist())),
                   'free_dead': int((~fa).sum()), 'free_on': free_on}
            lat = slice(nP + nF, None)
            bad = U[~g.alive[lat] | (g.dom[lat] != 0)]
            row['badQ'] = int(len(bad))
            row['badQ_occupied'] = int(C.occ[bad].sum())
            fh.write(json.dumps(row) + '\n')
            rows.append(row)
            if row['T_on'] == nT and row['prot_dead'] == 0 and row['prot_moved'] == 0 and row['free_dead'] == 0:
                good.append(row)
            if n % 100 == 0:
                log(n=n, of=len(order), good=len(good),
                    best=sorted((r['badQ'], r['badQ_occupied']) for r in good)[:3])
            if len(good) >= GOODQ and n >= MINQ:
                break
    good.sort(key=lambda r: (r['badQ'], r['badQ_occupied']))
    summary['q_scored'] = len(rows)
    summary['q_good'] = len(good)
    summary['q_best'] = [{k: v for k, v in r.items() if k != 'Q'} for r in good[:10]]
    fail = defaultdict(int)
    for r in rows:
        if r['T_on'] != len(T):
            fail['T_not_on'] += 1
        elif r['prot_dead']:
            fail['prot_dead'] += 1
        elif r['prot_moved']:
            fail['prot_moved'] += 1
    summary['q_fail'] = dict(fail)
    csets = defaultdict(int)
    for r in rows:
        if r['T_on'] == len(T) and r['T_dead'] == 0:
            csets[tuple(r['conflict_blocks'])] += 1
    summary['q_conflict_sets'] = sorted(([list(k), v] for k, v in csets.items()), key=lambda x: (len(x[0]), -x[1]))[:30]
    summary['blocks'] = [[int(j_), len(b_), int(b_[0]) if len(b_) else -1] for j_, b_ in enumerate(C.bl.blocks)]
    log(q_scored=len(rows), q_good=len(good), q_fail=dict(fail))
    if not good:
        summary['stop'] = 'no Q with T on K and the protected pieces exact'
        Path(prefix + '.summary.json').write_text(json.dumps(summary, indent=1) + '\n')
        return
    # parking for the best distinct (e, c) Q's
    parked = []
    seen_ec = set()
    picks = []
    for r in good:
        if (r['e'], r['c'], r['a']) in seen_ec or r['badQ'] > good[0]['badQ'] * 2:
            continue
        seen_ec.add((r['e'], r['c'], r['a']))
        picks.append(r)
        if len(picks) >= PARKQ:
            break
    nT = len(T)
    with open(prefix + '.park.jsonl', 'w') as fh:
        for r in picks:
            Q = [tuple(x) for x in r['Q']]
            badQ, _, _ = C.bad(Q)
            sig = data.sig_matrix(badQ)
            lens = np.flatnonzero(sig.sum(axis=0) == len(badQ)).tolist()
            e, c, Xi = r['e'], r['c'], r['X']
            Xm = E(c, Xi)
            Ys = [(p, y) for p in lens if p != c and pole(Xm, p) >= 0 and adm[p] for y in kit(p)]
            Zs = [(z, i) for z in range(600) if adm[z] for i in range(n_items)
                  if not inK(E(z, i)) and img(E(z, i), e) == c]
            log(Q=[e, c, r['a']], badQ=int(len(badQ)), lens=lens, Y=len(Ys), Z=len(Zs))
            nP_tried = 0
            for (p, y) in Ys:
                zs = Zs if relaxed else [(z, i) for (z, i) in Zs if img(E(z, i), p) == p]
                for (z, i) in zs:
                    if nP_tried >= MAXP:
                        break
                    nP_tried += 1
                    J1 = [(c, Xi), (p, y), (c, inv(c, Xi)), (p, inv(p, y))]
                    J1i = [(p, y), (c, Xi), (p, inv(p, y)), (c, inv(c, Xi))]
                    P = J1 + [(z, i)] + J1i + [(z, inv(z, i))]
                    Pinv = [(cc, inv(cc, ii)) for cc, ii in reversed(P)]
                    g, nP, U = C.ghost(P + Q + Pinv, np.concatenate([T, prot]))
                    pa = g.alive[nT:nP]
                    lat = slice(nP, None)
                    bad = U[~g.alive[lat] | (g.dom[lat] != 0)]
                    mv = pa & ((g.dom[nT:nP] != d0) | (g.q[nT:nP] != q0))
                    row = {'Q': r['Q'], 'P': [list(x) for x in P],
                           'T_on': int((g.alive[:nT] & (g.dom[:nT] == 0)).sum()),
                           'prot_dead': int((~pa).sum()), 'prot_moved': int(mv.sum()),
                           'conflict_blocks': sorted(set(prot_blk[~pa | mv].tolist())),
                           'badW': int(len(bad)), 'badW_occupied': int(C.occ[bad].sum()),
                           'admissible_grips': bool(all(adm[x] for x, _ in P))}
                    fh.write(json.dumps(row) + '\n')
                    if row['T_on'] == nT and row['prot_dead'] == 0 and row['prot_moved'] == 0:
                        parked.append(row)
            log(Q=[e, c, r['a']], parking_tried=nP_tried, parked_good=len(parked),
                best=sorted(x['badW'] for x in parked)[:5])
    parked.sort(key=lambda x: (x['badW'], x['badW_occupied']))
    summary['park_good'] = len(parked)
    summary['park_best'] = [{k: v for k, v in x.items()} for x in parked[:10]]
    summary['seconds'] = round(time.time() - T0, 1)
    best = parked[0] if parked else None
    if best is not None:
        Path(prefix + '.best.json').write_text(json.dumps({'P': best['P'], 'Q': best['Q'], 'badW': best['badW']}) + '\n')
    Path(prefix + '.summary.json').write_text(json.dumps(summary, indent=1) + '\n')
    log(done=True, park_good=len(parked), best=(best['badW'] if best else None))


if __name__ == '__main__':
    main()
