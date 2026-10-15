"""Parking search for a restoring double commutator Q (dcomm2.py row), read from the current state only.

Q = A J A^-1 J^-1 restores block T but its Bad(Q) (here a triple lens of three caps) holds lattice pieces. A parking
word P = [J1, Z] = J1 Z J1^-1 Z^-1, J1 = [X, Y] (X: Q's jumbling letter; Y: a K+ letter at another cap of the triple,
a pole of X's frame; Z: a jumbling letter with Z n_e = n_c, e = B's grip, c = X's grip; optionally Z n_p = n_p for Y's
grip p) is tried, and W = P Q P^-1 is scored by the exact frame ghost: T on the lattice, the other off pieces back at
their exact poses, Bad(W) and how many of its chambers hold lattice pieces of the state.

Usage: python -I parksearch.py <theory dir> <pose table> <block size> <Q json> <out jsonl> [--relaxed]
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
import ghostlib as GL  # noqa: E402
from exact import matvec, transpose  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

data, kp = GL.data, GL.BL.CL.ctx.kplus
T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def main():
    a = sys.argv[1:]
    C = GL.Ctx(a[1])
    size, Q, outp = int(a[2]), [tuple(x) for x in json.loads(a[3])], Path(a[4])
    relaxed = '--relaxed' in a
    M = C.M
    n_items = len(M.items)
    E = C.E
    inK = lambda g: kp.index_of_matrix(to_tuple(g)) >= 0  # noqa: E731
    pole = lambda g, e: data.POLE.get(tuple(matvec(transpose(g), data.N[e])), -1)  # noqa: E731
    img = lambda g, e: data.POLE.get(tuple(matvec(g, data.N[e])), -1)  # noqa: E731

    def inv(c, i):
        t = to_tuple(transpose(E(c, i)))
        return next(j for j in range(n_items) if to_tuple(E(c, j)) == t)
    T = C.block(size)
    prot = np.setdiff1d(C.off, T)
    badQ, _, _ = C.bad(Q)
    sig = data.sig_matrix(badQ)
    triple = np.flatnonzero(sig.sum(axis=0) == len(badQ)).tolist()
    # Q = A, B, X, B^-1, X^-1, A^-1, X, B, X^-1, B^-1
    (a_, Ai), (e, Bi), (c, Xi) = Q[0], Q[1], Q[2]
    log(badQ=int(len(badQ)), badQ_occupied=int(C.occ[badQ].sum()), triple=triple, a=a_, e=e, c=c)
    Xm = E(c, Xi)
    Ys = [(p, y) for p in triple if p != c and pole(Xm, p) >= 0
          for y in range(n_items) if inK(E(p, y)) and to_tuple(E(p, y)) != to_tuple(E(p, 0))]
    Ys = [(p, y) for (p, y) in Ys if y != 0]
    Zs = []
    for z in range(600):
        for i in range(n_items):
            g = E(z, i)
            if inK(g):
                continue
            if img(g, e) != c:
                continue
            Zs.append((z, i))
    log(Y=len(Ys), Z_maps_e_to_c=len(Zs))
    cands = []
    for (p, y) in Ys:
        fixp = [(z, i) for (z, i) in Zs if img(E(z, i), p) == p]
        for (z, i) in (Zs if relaxed else fixp):
            X1, Y1, Z1 = (c, Xi), (p, y), (z, i)
            J1 = [X1, Y1, (c, inv(c, Xi)), (p, inv(p, y))]
            J1i = [Y1, X1, (p, inv(p, y)), (c, inv(c, Xi))]
            P = J1 + [Z1] + J1i + [(z, inv(z, i))]
            cands.append(P)
    log(candidates=len(cands))
    with outp.open('w') as fh:
        for n, P in enumerate(cands):
            Pinv = [(cc, inv(cc, ii)) for cc, ii in reversed(P)]
            W = P + Q + Pinv
            g, nP_, U = C.ghost(W, np.concatenate([T, prot]))
            nT = len(T)
            tA = g.alive[:nT]
            T_on = int((tA & (g.dom[:nT] == 0)).sum())
            d0, q0 = C.contents(prot)
            pr = slice(nT, nP_)
            prot_ok = bool(g.alive[pr].all() and np.array_equal(g.dom[pr], d0) and np.array_equal(g.q[pr], q0))
            lat = slice(nP_, None)
            bad = U[~g.alive[lat] | (g.dom[lat] != 0)]
            row = {'P': [list(x) for x in P], 'T_on': T_on, 'T_dead': int((~tA).sum()), 'prot_exact': prot_ok,
                   'badW': int(len(bad)), 'badW_occupied': int(C.occ[bad].sum())}
            fh.write(json.dumps(row) + '\n')
            fh.flush()
            log(n=n, of=len(cands), **{k: v for k, v in row.items() if k != 'P'}, Y=P[1], Z=P[4])


if __name__ == '__main__':
    main()
