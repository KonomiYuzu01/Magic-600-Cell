"""Search double commutators Q = A J A^-1 J^-1 that restore one rigid off block, read from the current state only.

J^-1 = (c, X)(e, B)(c, X^-1)(e, B^-1): X in I_c outside K+ (the cap c is jumbled), B in A4_e (a K+ letter at a grip e
whose cut is a pole of the frame X: X^-1 n_e is a pole, so B is not blocked by the jumbled lattice of cap c), and the
block T is outside cap c, inside cap e, B(T) inside cap c, and X^-1 B m in K+ (m: T's pose). So J^-1 puts T on the
lattice and jumbles some lattice pieces. A is a K+ letter at a grip a with T inside cap a and A(T) outside caps c and e;
the commutator with A lets J act on the lattice before J^-1 does, so that only T's region differs (the pattern of
window 36568's Q). Candidates are listed with exact cut tests on T only; each is then applied to the full state
(every twist checked admissible) and scored: T restored, the other off pieces exact, the collateral, and undone.

Usage: python -I dcomm.py <theory dir> <pose table> <block index> <out jsonl> [--max N] [--only-compatible]
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
import conjclose as CC  # noqa: E402
from exact import matmul, matvec, transpose  # noqa: E402
from sim.kernel import Evaluator, pullback_form  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

CL, O, restore = BL.CL, BL.O, BL.restore
kp, data = CL.ctx.kplus, CL.ctx.data
T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def groups_of(st, pieces):
    """[(pose matrix, [pieces])] for the distinct poses of a block."""
    by = {}
    for q in pieces:
        g = st.pose(int(q))
        by.setdefault(to_tuple(g), (g, []))[1].append(int(q))
    return list(by.values())


def status(h, groups, cuts=range(600)):
    """For each cut: 0 outside, 1 inside, 2 some piece straddles, 3 pieces on both sides, for every piece of the
    block moved by the left factor h (piece p at pose h g_p)."""
    reg = CL.ctx.regions
    mats = [(matmul(h, g), pcs) for g, pcs in groups]
    out = {}
    for e in cuts:
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
        out[e] = 2 if 'straddle' in seen else (3 if len(seen) > 1 else (1 if 'in' in seen else 0))
    return out


def main():
    a = sys.argv[1:]
    table, b, outp = a[1], int(a[2]), Path(a[3])
    mx = int(a[a.index('--max') + 1]) if '--max' in a else 10 ** 9
    R, M = O.start(table)
    st = R.st
    bl = BL.Blocks(R, M)
    T = bl.blocks[b]
    gs = [st.pose(int(p)) for p in T]
    m = gs[0]
    grp = groups_of(st, T)
    if any(kp.index_of_matrix(to_tuple(matmul(transpose(m), g))) < 0 for g in gs):
        raise ValueError('block poses are not in one domain')
    n_items = len(M.items)
    el = {}

    def E(c, i):
        if (c, i) not in el:
            el[(c, i)] = restore.element(M, c, i)
        return el[(c, i)]

    def inK(g):
        return kp.index_of_matrix(to_tuple(g)) >= 0

    def inv(c, i):
        t = to_tuple(transpose(E(c, i)))
        return next(j for j in range(n_items) if to_tuple(E(c, j)) == t)

    def pole_of(g, e):
        return data.POLE.get(tuple(matvec(transpose(g), data.N[e])), -1)

    adm = {c: R.inside(c) is not None for c in range(600)}
    s0 = bl.status[b]
    inside0 = [int(c) for c in np.flatnonzero(s0 == 1) if adm[int(c)]]
    log(block=b, size=len(T), inside_admissible=inside0, straddled=np.flatnonzero(s0 == 2).tolist())
    def kitems(c):
        ident = to_tuple(matmul(E(c, 0), transpose(E(c, 0))))
        return [i for i in range(n_items) if inK(E(c, i)) and to_tuple(E(c, i)) != ident]

    cands = []
    cache_A = {}
    for e in inside0:
        for Bi in kitems(e):
            gB = matmul(E(e, Bi), m)
            sB = status(E(e, Bi), grp)
            for c in [cc for cc, v in sB.items() if v == 1 and s0[cc] == 0 and adm[cc]]:
                Xs = [i for i in range(n_items) if not inK(E(c, i)) and inK(matmul(transpose(E(c, i)), gB))]
                if not Xs:
                    continue
                compat = pole_of(E(c, Xs[0]), e) >= 0
                if not compat:
                    log(skip='incompatible', e=e, B=Bi, c=c)
                    continue
                for a_ in inside0:
                    for Ai in kitems(a_):
                        key = (a_, Ai)
                        if key not in cache_A:
                            cache_A[key] = status(E(a_, Ai), grp)
                        sA = cache_A[key]
                        if sA[c] != 0 or sA[e] != 0:
                            continue
                        for Xi in Xs:
                            J = [(e, Bi), (c, Xi), (e, inv(e, Bi)), (c, inv(c, Xi))]
                            Jinv = [(c, Xi), (e, Bi), (c, inv(c, Xi)), (e, inv(e, Bi))]
                            Q = [(a_, Ai)] + J + [(a_, inv(a_, Ai))] + Jinv
                            cands.append({'e': e, 'B': Bi, 'c': c, 'X': Xi, 'a': a_, 'A': Ai, 'Q': Q})
                log(e=e, B=Bi, c=c, X=len(Xs), candidates=len(cands))
    log(candidates=len(cands))
    codes = CC.Codes()
    c_x = codes.of(st)
    d_x = st.digest()
    off_x = c_x < 0
    inT = np.zeros(len(c_x), bool)
    inT[T] = True
    with outp.open('w') as fh:
        for n, cd in enumerate(cands[:mx]):
            row = {k: v for k, v in cd.items() if k != 'Q'}
            row['Q'] = [list(x) for x in cd['Q']]
            applied = 0
            try:
                for c, i in cd['Q']:
                    R.apply(c, i)
                    applied += 1
            except RuntimeError:
                row['inadmissible_at'] = applied
            if 'inadmissible_at' not in row:
                c_y = codes.of(st)
                row['T_restored'] = int((inT & (c_y >= 0)).sum())
                row['prot_changed'] = int((off_x & ~inT & (c_y != c_x)).sum())
                row['collateral'] = int((~off_x & (c_y < 0)).sum())
                row['off_end'] = int((c_y < 0).sum())
            for _ in range(applied):
                R.undo()
            if st.digest() != d_x:
                raise AssertionError('undo did not restore the state')
            fh.write(json.dumps(row) + '\n')
            fh.flush()
            log(n=n, of=len(cands), **{k: v for k, v in row.items() if k not in ('Q',)})


if __name__ == '__main__':
    main()
