"""Premise certificate for W = P Q P^-1 at a state, read from the current state only (no replay of W).

Checks, with exact arithmetic on the words and the block's own pieces:
- letters: K+ membership and involution of every letter; P^-1 is the letter-by-letter exact inverse of P;
- G1 pole compatibility: for X (Q's third letter, outside K+) the pole X^-1 n_p of every other grip p of W;
- G2 block conditions of Q = A, B, X, B^-1, X^-1, A^-1, X, B, X^-1, B^-1 on the block T: T inside cap e and outside
  cap c; B(T) inside cap c; X^-1 B m in K+ for every pose m of T; T inside cap a; A(T) outside caps c and e;
- G3 lens: Bad(Q) by the chamber ghost, the caps that hold all of it, whether it equals their intersection, and the
  pole X^-1 n_e;
- G4 parking: P = [[X, Y], Z] with Y a K+ letter at a lens cap p (a pole of X's frame), Z outside K+, Z n_e = n_c and
  Z n_p = n_p;
- B2 ghost: T ends on the lattice, the other off pieces return exactly, Bad(W), T's landing chambers, and how many
  chambers of Bad(W) hold lattice pieces at the state.

Usage: python -I premx.py <theory dir> <pose table> <block size> <W json {'P','Q'}> <out json>
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(__file__).resolve().parent))
import ghostlib as GL  # noqa: E402
from exact import matmul, matvec, transpose  # noqa: E402
from sim.kernel import Evaluator, pullback_form  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

data, kp, reg = GL.data, GL.BL.CL.ctx.kplus, GL.BL.CL.ctx.regions


def main():
    a = sys.argv[1:]
    C = GL.Ctx(a[1])
    size, W = int(a[2]), json.loads(Path(a[3]).read_text())
    P = [tuple(x) for x in W['P']]
    Q = [tuple(x) for x in W['Q']]
    E, st = C.E, C.st
    n_items = len(C.M.items)
    inK = lambda g: kp.index_of_matrix(to_tuple(g)) >= 0  # noqa: E731
    pole_of = lambda g, e: data.POLE.get(tuple(matvec(transpose(g), data.N[e])), -1)  # noqa: E731
    image = lambda g, e: data.POLE.get(tuple(matvec(g, data.N[e])), -1)  # noqa: E731

    def inv(c, i):
        t = to_tuple(transpose(E(c, i)))
        return next(j for j in range(n_items) if to_tuple(E(c, j)) == t)
    Pinv = [(c, inv(c, i)) for c, i in reversed(P)]
    T = C.block(size)
    prot = np.setdiff1d(C.off, T)
    groups = {}
    for p in T.tolist():
        g = st.pose(int(p))
        groups.setdefault(to_tuple(g), (g, []))[1].append(int(p))
    groups = list(groups.values())

    def status(h, cut):
        seen = set()
        for g, pcs in groups:
            ev = Evaluator(pullback_form(matmul(h, g), data.N[cut]))
            for p in pcs:
                seen.add(ev.classify(reg.vform(int(p)))[0])
        return 'straddle' if 'straddle' in seen else ('both' if len(seen) > 1 else seen.pop())
    ident = E(0, 0)
    (ga, ia), (ge, ib), (gc, ix) = Q[0], Q[1], Q[2]
    A, B, X = E(ga, ia), E(ge, ib), E(gc, ix)
    letters = sorted(set(P) | set(Q) | set(Pinv))
    res = {'state_digest': st.digest(), 'block_size': len(T), 'block_smallest_piece': int(T.min()),
           'block_poses': len(groups), 'P': [list(x) for x in P], 'Q': [list(x) for x in Q],
           'P_inverse': [list(x) for x in Pinv],
           'letters': {f'{c},{i}': {'in_K+': inK(E(c, i)), 'involution': inv(c, i) == i} for c, i in letters}}
    res['Q_template'] = Q == [(ga, ia), (ge, ib), (gc, ix), (ge, inv(ge, ib)), (gc, inv(gc, ix)), (ga, inv(ga, ia)),
                              (gc, ix), (ge, ib), (gc, inv(gc, ix)), (ge, inv(ge, ib))]
    grips = sorted({c for c, _ in P + Q} - {gc})
    res['G1_poles_of_X_frame'] = {str(p): pole_of(X, p) for p in grips}
    Xinv = E(gc, inv(gc, ix))
    res['G2'] = {'a': ga, 'e': ge, 'c': gc,
                 'T_in_cap_e': status(ident, ge), 'T_in_cap_c': status(ident, gc),
                 'B(T)_in_cap_c': status(B, gc),
                 'X^-1 B m in K+': all(inK(matmul(Xinv, matmul(B, g))) for g, _ in groups),
                 'T_in_cap_a': status(ident, ga), 'A(T)_in_cap_c': status(A, gc), 'A(T)_in_cap_e': status(A, ge)}
    badQ, _, _ = C.bad(Q)
    sig = data.sig_matrix(badQ)
    caps = np.flatnonzero(sig.sum(axis=0) == len(badQ)).tolist()
    lens = np.logical_and.reduce([GL.cap(c) for c in caps])
    res['G3'] = {'badQ': int(len(badQ)), 'badQ_occupied': int(C.occ[badQ].sum()), 'caps_holding_badQ': caps,
                 'lens_size': int(lens.sum()), 'badQ_equals_lens': bool(lens.sum() == len(badQ) and lens[badQ].all()),
                 'pole_X^-1_n_e': pole_of(X, ge)}
    (_, _), (py, iy) = P[0], P[1]
    gz, iz = P[4]
    Z = E(gz, iz)
    res['G4'] = {'P_shape': P == [(gc, ix), (py, iy), (gc, inv(gc, ix)), (py, inv(py, iy)), (gz, iz), (py, iy),
                                  (gc, ix), (py, inv(py, iy)), (gc, inv(gc, ix)), (gz, inv(gz, iz))],
                 'X_shared_with_Q': P[0] == (gc, ix), 'Y': [py, iy], 'Y_in_K+': inK(E(py, iy)),
                 'Y_grip_in_lens_caps': py in caps, 'Y_grip_pole_of_X_frame': pole_of(X, py),
                 'Z': [gz, iz], 'Z_in_K+': inK(Z), 'Z n_e': image(Z, ge), 'Z n_p': image(Z, py)}
    word = P + Q + Pinv
    g, n, U = C.ghost(word, np.concatenate([T, prot]))
    nT = len(T)
    d0, q0 = C.contents(prot)
    lat = slice(n, None)
    badW = np.sort(U[~g.alive[lat] | (g.dom[lat] != 0)])
    landing = g.q[:nT]
    res['B2'] = {'T_on_lattice': int((g.alive[:nT] & (g.dom[:nT] == 0)).sum()),
                 'prot_exact': bool(g.alive[nT:n].all() and np.array_equal(g.dom[nT:n], d0)
                                    and np.array_equal(g.q[nT:n], q0)),
                 'badW': int(len(badW)), 'badW_occupied': int(C.occ[badW].sum()),
                 'landing_in_badW': int(np.isin(landing, badW).sum()), 'landing': sorted(landing.tolist()),
                 'badW_in_badQ_lens': int(np.isin(badW, badQ).sum()),
                 'holes': int((~C.occ).sum()), 'badW_chambers': badW.tolist()}
    Path(a[4]).write_text(json.dumps(res, indent=1) + '\n')
    print(json.dumps({k: v for k, v in res.items() if k not in ('P', 'Q', 'P_inverse', 'B2')}, indent=1))
    print(json.dumps({k: v for k, v in res['B2'].items() if k not in ('landing', 'badW_chambers')}))


if __name__ == '__main__':
    main()
