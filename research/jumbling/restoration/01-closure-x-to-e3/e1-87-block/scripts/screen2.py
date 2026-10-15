"""Cheap exact screen of W = P Q P^-1 candidates for one block at a state (reads only the state's pose table).

For each candidate (P from a parksearch row, Q from a list): the frame ghost of W gives T on the lattice, the other off
pieces back at their exact contents, Bad(W), T's landing chambers and the occupied chambers of Bad(W); the ghost of
P^-1 from the identity gives Bad(P^-1), and T's landing after P Q is tested against Good(P^-1). The SAFE-ALL alphabet
of the state (built once) gives the orbits and the vacancy condition for the target Bad(W): N1 (fixed bad chambers are
holes) and N2 (every orbit has at least as many holes as bad chambers), with the per-orbit demand.

Usage: python -I screen2.py <theory dir> <pose table> <block size> <out jsonl> <candidates json>
The candidates file is a list of {'P': [...], 'Q': [...], 'source': ...}.
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
import buffer as B  # noqa: E402
import conjprep as CP  # noqa: E402
import ghostlib as GL  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def main():
    a = sys.argv[1:]
    C = GL.Ctx(a[1])
    size, outp, cands = int(a[2]), Path(a[3]), json.loads(Path(a[4]).read_text())
    T = C.block(size)
    prot = np.setdiff1d(C.off, T)
    d0, q0 = C.contents(prot)
    t_alpha = time.time()
    G = B.Alphabet(C.R, C.M)
    G.orbits()
    log(safe_grips=len(G.safe), letters=len(G.letters), holes=len(G.holes), orbits=len(G.orbit_size),
        alphabet_sec=round(time.time() - t_alpha, 1))
    with outp.open('w') as fh:
        for n, cand in enumerate(cands):
            t1 = time.time()
            P = [tuple(x) for x in cand['P']]
            Q = [tuple(x) for x in cand['Q']]
            Pinv = CP.inverse_word(C.M, P)
            W = P + Q + Pinv
            g, nP, U = C.ghost(W, np.concatenate([T, prot]))
            nT = len(T)
            lat = slice(nP, None)
            badW = np.sort(U[~g.alive[lat] | (g.dom[lat] != 0)])
            T_on = int((g.alive[:nT] & (g.dom[:nT] == 0)).sum())
            prot_exact = bool(g.alive[nT:nP].all() and np.array_equal(g.dom[nT:nP], d0)
                              and np.array_equal(g.q[nT:nP], q0))
            gpq, _, _ = C.ghost(P + Q, T, np.zeros(0, np.int64))
            land_dom, land_q = gpq.dom[:nT], gpq.q[:nT]
            badPinv, _, _ = C.bad(Pinv)
            cond = B.condition(G, badW)
            row = {'n': n, 'source': cand.get('source'), 'P': [list(x) for x in P], 'Q': [list(x) for x in Q],
                   'T_on': T_on, 'prot_exact': prot_exact, 'badW': int(len(badW)),
                   'badW_occupied': int(C.occ[badW].sum()), 'holes': int((~C.occ).sum()),
                   'landing_W': sorted(g.q[:nT].tolist()), 'landing_in_badW': int(np.isin(g.q[:nT], badW).sum()),
                   'T_after_PQ_on_lattice': int((gpq.alive[:nT] & (land_dom == 0)).sum()),
                   'badPinv': int(len(badPinv)),
                   'T_after_PQ_in_badPinv': int(np.isin(land_q[land_dom == 0], badPinv).sum()),
                   'N1': cond['fixed_vacant'], 'N2': cond['necessary'],
                   'fixed_occupied': cond['fixed_occupied'], 'deficits': cond['necessary_deficits'],
                   'orbits_hit': len(cond['by_orbit']),
                   'tight_orbits': sum(1 for r in cond['by_orbit'] if r['holes'] == r['target']),
                   'by_orbit': cond['by_orbit'], 'badW_chambers': badW.tolist(),
                   'sec': round(time.time() - t1, 2)}
            fh.write(json.dumps(row) + '\n')
            fh.flush()
            log(**{k: v for k, v in row.items() if k not in ('P', 'Q', 'by_orbit', 'badW_chambers', 'landing_W')})


if __name__ == '__main__':
    main()
