"""Score double commutators Q = A J A^-1 J^-1 for one rigid off block with the exact frame ghost (fastghost.py).

Candidates as in dcomm.py (J^-1 = (c, X)(e, B)(c, X^-1)(e, B^-1) puts the block T on the lattice; A is a K+ letter
moving T out of caps c and e). For each candidate the ghost follows T, the other off pieces (Prot) and every chamber
of the union of Q's caps as a lattice content, and reports: T alive and on the lattice at the end, Prot back at its
exact pose, and Bad(Q), the chambers whose lattice content would straddle or end off the lattice. Q leaves zero
collateral at a state S with the same off pieces exactly when Bad(Q) holds no lattice piece of S. Reads only the
state; every decision is exact.

Usage: python -I dcomm2.py <theory dir> <pose table> <block index> <out jsonl> [--max N] [--top K]
"""
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(__file__).resolve().parent))
import blocks as BL  # noqa: E402
import dcomm as DC  # noqa: E402
import fastghost as FG  # noqa: E402
import rim  # noqa: E402
from exact import matmul, transpose  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

CL, O, restore = BL.CL, BL.O, BL.restore
kp, data = CL.ctx.kplus, CL.ctx.data
NP = len(data.sigbits)
T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def enumerate_candidates(R, M, st, bl, b):
    T = bl.blocks[b]
    m = st.pose(int(T[0]))
    grp = DC.groups_of(st, T)
    n_items = len(M.items)
    el = {}

    def E(c, i):
        if (c, i) not in el:
            el[(c, i)] = restore.element(M, c, i)
        return el[(c, i)]

    def inK(g):
        return kp.index_of_matrix(to_tuple(g)) >= 0

    inv_cache = {}

    def inv(c, i):
        if (c, i) not in inv_cache:
            t = to_tuple(transpose(E(c, i)))
            inv_cache[(c, i)] = next(j for j in range(n_items) if to_tuple(E(c, j)) == t)
        return inv_cache[(c, i)]

    def pole_of(g, e):
        return FG.data.POLE.get(tuple(FG.matvec(transpose(g), data.N[e])), -1)

    def kitems(c):
        ident = to_tuple(matmul(E(c, 0), transpose(E(c, 0))))
        return [i for i in range(n_items) if inK(E(c, i)) and to_tuple(E(c, i)) != ident]

    adm = {c: R.inside(c) is not None for c in range(600)}
    s0 = bl.status[b]
    inside0 = [int(c) for c in np.flatnonzero(s0 == 1) if adm[int(c)]]
    cands, cache_A, incompatible = [], {}, 0
    for e in inside0:
        for Bi in kitems(e):
            gB = matmul(E(e, Bi), m)
            sB = DC.status(E(e, Bi), grp)
            for c in [cc for cc, v in sB.items() if v == 1 and s0[cc] == 0 and adm[cc]]:
                Xs = [i for i in range(n_items) if not inK(E(c, i)) and inK(matmul(transpose(E(c, i)), gB))]
                if not Xs:
                    continue
                if pole_of(E(c, Xs[0]), e) < 0:
                    incompatible += 1
                    continue
                for a_ in inside0:
                    for Ai in kitems(a_):
                        if (a_, Ai) not in cache_A:
                            cache_A[(a_, Ai)] = DC.status(E(a_, Ai), grp)
                        sA = cache_A[(a_, Ai)]
                        if sA[c] != 0 or sA[e] != 0:
                            continue
                        for Xi in Xs:
                            J = [(e, Bi), (c, Xi), (e, inv(e, Bi)), (c, inv(c, Xi))]
                            Jinv = [(c, Xi), (e, Bi), (c, inv(c, Xi)), (e, inv(e, Bi))]
                            cands.append({'e': e, 'B': Bi, 'c': c, 'X': Xi, 'a': a_, 'A': Ai,
                                          'Q': [(a_, Ai)] + J + [(a_, inv(a_, Ai))] + Jinv})
    log(candidates=len(cands), incompatible_pairs=incompatible, inside=inside0)
    return cands, E


def main():
    a = sys.argv[1:]
    table, b, outp = a[1], int(a[2]), Path(a[3])
    mx = int(a[a.index('--max') + 1]) if '--max' in a else 10 ** 9
    R, M = O.start(table)
    st = R.st
    bl = BL.Blocks(R, M)
    T = np.asarray(bl.blocks[b], np.int64)
    off = np.flatnonzero(st._kpose()[st.pose_id] < 0)
    prot = np.setdiff1d(off, T)
    D = rim.Domains(st)
    F = FG.Frames()
    # off contents in the ghost's frames
    dmap = {}
    for j, rep in enumerate(D.reps):
        dmap[j] = F.domain_of(rep)

    def contents(pieces):
        dom = np.empty(len(pieces), np.int64)
        q = np.empty(len(pieces), np.int64)
        for n, p in enumerate(pieces.tolist()):
            d2, k = dmap[int(D.dom[p])]
            dom[n] = d2
            q[n] = FG.chamber_image(k, np.array([D.cham[p]]))[0]
        return dom, q
    T_dom, T_q = contents(T)
    P_dom, P_q = contents(prot)
    lat = np.flatnonzero(D.dom == 0)
    occupied = np.zeros(NP, bool)
    occupied[D.cham[lat]] = True
    log(T=len(T), prot=len(prot), holes=int((~occupied).sum()), domains=len(F.reps))
    cands, E = enumerate_candidates(R, M, st, bl, b)
    caps = {}

    def cap(c):
        if c not in caps:
            caps[c] = data.in_sig(np.arange(NP), np.full(NP, c, np.int64))
        return caps[c]
    rows = []
    with outp.open('w') as fh:
        for n, cd in enumerate(cands[:mx]):
            grips = sorted({c for c, _ in cd['Q']})
            U = np.flatnonzero(np.logical_or.reduce([cap(c) for c in grips]))
            nT, nP = len(T), len(prot)
            dom = np.concatenate([T_dom, P_dom, np.zeros(len(U), np.int64)])
            q = np.concatenate([T_q, P_q, U])
            g = FG.Ghost(F, dom, q)
            for c, i in cd['Q']:
                g.apply(c, E(c, i), (c, i))
            tA, pA, lA = g.alive[:nT], g.alive[nT:nT + nP], g.alive[nT + nP:]
            row = {k: v for k, v in cd.items() if k != 'Q'}
            row['Q'] = [list(x) for x in cd['Q']]
            row['T_dead'] = int((~tA).sum())
            row['T_on'] = int((tA & (g.dom[:nT] == 0)).sum())
            row['prot_dead'] = int((~pA).sum())
            row['prot_moved'] = int((pA & ((g.dom[nT:nT + nP] != P_dom) | (g.q[nT:nT + nP] != P_q))).sum())
            bad = ~lA | (g.dom[nT + nP:] != 0)
            badz = U[bad]
            row['bad'] = int(len(badz))
            row['bad_unclean'] = int((~lA).sum())
            row['bad_occupied_at_X'] = int(occupied[badz].sum())
            if row['T_on'] == nT:
                land = g.q[:nT]
                row['landing_in_bad'] = int(np.isin(land, badz).sum())
            fh.write(json.dumps(row) + '\n')
            rows.append(row)
            if n % 200 == 0 or (row['T_on'] == nT and row['prot_moved'] == 0 and row['prot_dead'] == 0):
                log(n=n, of=len(cands), **{k: v for k, v in row.items() if k != 'Q'})
    good = [r for r in rows if r['T_on'] == len(T) and r['T_dead'] == 0 and r['prot_dead'] == 0 and r['prot_moved'] == 0]
    good.sort(key=lambda r: r['bad'])
    log(done=len(rows), restoring_clean_prot=len(good), best=[(r['bad'], r['bad_occupied_at_X']) for r in good[:10]])


if __name__ == '__main__':
    main()
