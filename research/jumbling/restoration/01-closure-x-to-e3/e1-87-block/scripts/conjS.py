"""Type-S restoration of B' with a conjugated vacancy word: U = P (x h x^-1) P^-1 (reads only the pose table).

After P (B' on K in chambers L, target T = Bad(P^-1), holes H = BH + GH), every SAFE-ALL letter that moves L may also
move bad holes. Take such a letter x. H0 = SAFE-ALL grips whose caps avoid T, so H0 words fix T pointwise. Then
x h x^-1 ends with holes exactly T if and only if h maps x(H) onto x(T); with x(L) and x(GH) outside T this is a
vacancy problem in H0: target Tx = x(T) minus T, holes x(H) minus T. Before any twist after P is applied, the script
records for each x: Tx, N1 and N2 for Tx in the H0 orbits, and, if they hold, the peeling certificate (Theorem B) and
the greedy procedure's word h. For the first x with a word it replays P x h x^-1 P^-1 on the simulator (every twist
checked admissible) and compares the end with the start.

Usage: python -I conjS.py <theory dir> <pose table> <P json word> <out dir> [--grips g1,g2] [--all]
"""
import copy
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import buffer as B  # noqa: E402
import conjclose as CC  # noqa: E402
import conjprep as CP  # noqa: E402
import ghostlib as GL  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def restrict(G, grips, kp):
    gs = set(grips)
    G0 = copy.copy(G)
    for attr in ('orbit', 'orbit_size', 'orbit_least'):
        G0.__dict__.pop(attr, None)
    G0.safe = [c for c in G.safe if c in gs]
    G0.capx = {c: G.capx[c] for c in G0.safe}
    G0.letters = [l for l in G.letters if l[0] in gs]
    G0.by_grip = {}
    for n, l in enumerate(G0.letters):
        G0.by_grip.setdefault(l[0], []).append(n)
    G0.index = {(l[0], l[1]): n for n, l in enumerate(G0.letters)}
    kindex = {(l[0], l[2]): n for n, l in enumerate(G0.letters)}
    G0.inv = np.array([kindex[(l[0], int(kp.inv[l[2]]))] for l in G0.letters], np.int64)
    G0.orbits()
    return G0


def main():
    a = sys.argv[1:]
    out = Path(a[3])
    out.mkdir(parents=True, exist_ok=True)
    C = GL.Ctx(a[1])
    R, M, st = C.R, C.M, C.st
    from blocks import CL
    reg, kp = CL.ctx.regions, CL.ctx.kplus
    codes = CC.Codes()
    c0 = codes.of(st)
    d0 = st.digest()
    off0 = np.flatnonzero(c0 < 0)
    lat0 = c0 >= 0
    P = [tuple(x) for x in json.loads(a[2])]
    Pinv = CP.inverse_word(M, P)
    badPinv, _, _ = C.bad(Pinv)
    t = time.time()
    for c, i in P:
        R.apply(c, i)
    d_p = st.digest()
    G = B.Alphabet(R, M)
    G.orbits()
    holes = np.asarray(G.holes)
    T = np.zeros(G.NP, bool)
    T[badPinv] = True
    Kp = st._kpose()[st.pose_id]
    L = reg.piece_of[reg.orbit_of[off0], kp.compose_vec(Kp[off0], reg.transport[off0])]
    GH = holes[~T[holes]]
    h0 = [c for c in G.safe if not T[G.capx[c]].any()]
    G0 = restrict(G, h0, kp)
    isL = np.zeros(G.NP, bool)
    isL[L] = True
    lgrips = [c for c in G.safe if isL[G.capx[c]].any()]
    if '--grips' in a:
        lgrips = [int(x) for x in a[a.index('--grips') + 1].split(',')]
    pre = {'state_digest': d0, 'P': [list(x) for x in P], 'after_P_digest': d_p, 'T': int(T.sum()),
           'holes': int(len(holes)), 'GH': GH.tolist(), 'L': L.tolist(), 'h0_grips': len(h0),
           'h0_orbits': int(len(G0.orbit_size)), 'L_grips': lgrips, 'setup_sec': round(time.time() - t, 1),
           'candidates': []}
    log(**{k: v for k, v in pre.items() if k != 'candidates'})
    found = None
    for c in lgrips:
        for x in G.by_grip[c]:
            t = time.time()
            px = G.perm([x])
            xH = px[holes]
            xT = px[badPinv]
            row = {'x': G.pairs([x])[0], 'xL_outside_T': bool(not T[px[L]].any()),
                   'xGH_outside_T': bool(not T[px[GH]].any())}
            inter_ok = np.array_equal(np.sort(xH[T[xH]]), np.sort(xT[T[xT]]))
            row['x_holes_in_T_match'] = bool(inter_ok)
            Tx = np.sort(xT[~T[xT]])
            hx = np.sort(xH[~T[xH]])
            row['Tx'] = int(len(Tx))
            row['xL_h0_orbit_sizes'] = [int(G0.orbit_size[G0.orbit[px[l]]]) for l in L.tolist()]
            row['GH_h0_orbit_sizes'] = [int(G0.orbit_size[G0.orbit[g]]) for g in GH.tolist()]
            if inter_ok and row['xL_outside_T']:
                hmask = np.zeros(G.NP, bool)
                hmask[xH] = True
                cond = B.condition(G0, Tx, None, np.flatnonzero(hmask))
                row.update({'N1': cond['fixed_vacant'], 'N2': cond['necessary'],
                            'n_fixed_occupied': len(cond['fixed_occupied']),
                            'fixed_occupied': cond['fixed_occupied'][:10],
                            'deficit_total': sum(d['deficit'] for d in cond['necessary_deficits']),
                            'orbits_hit': len(cond['by_orbit'])})
                if cond['necessary'] and (found is None or '--all' in a):
                    try:
                        cert = CP.generic_certificate(G0, Tx)
                        counts = np.bincount(G0.orbit[np.flatnonzero(hmask)], minlength=len(G0.orbit_size))
                        row['certificate'] = {'peeled': cert['peeled'], 'of': cert['of'],
                                              'all_reached': cert['all_reached'],
                                              'sufficient': all(counts[int(o)] >= v
                                                                for o, v in cert['thresholds'].items())}
                        prep, _, deficits = CP.generic_prepare(G0, cert, Tx, np.flatnonzero(hmask))
                        row['prepare_deficits'] = {str(k): v for k, v in deficits.items()}
                        if prep is not None:
                            h = [tuple(y) for y in prep['word']]
                            Qp = [tuple(G.pairs([x])[0])] + h + [tuple(G.pairs([int(G.inv[x])])[0])]
                            pq = G.permutation([list(y) for y in Qp])
                            row['Qprime_length'] = len(Qp)
                            row['predicted_holes_equal_T'] = bool(np.array_equal(np.sort(pq[holes]),
                                                                                 np.sort(badPinv)))
                            if row['predicted_holes_equal_T'] and found is None:
                                found = (row['x'], Qp, cert)
                    except Exception as exc:  # noqa: BLE001
                        row['construct_error'] = str(exc)[:300]
            row['sec'] = round(time.time() - t, 1)
            pre['candidates'].append(row)
            log(**row)
    (out / 'pre.json').write_text(json.dumps(pre, indent=1) + '\n')
    for _ in P:
        R.undo()
    if st.digest() != d0:
        raise AssertionError('state not restored after undo')
    if found is None:
        (out / 'result.json').write_text(json.dumps({'result': 'no word'}, indent=1) + '\n')
        log(result='no word')
        return
    xw, Qp, cert = found
    (out / 'certificate-h.json').write_text(json.dumps(cert) + '\n')
    t = time.time()
    parts = [('P', P), ('Qprime', Qp), ('Pinv', Pinv)]
    rows = []
    (out / 'parts').mkdir(exist_ok=True)
    for name, part in parts:
        d_in = st.digest()
        for c, i in part:
            R.apply(c, i)
        cn = codes.of(st)
        rows.append({'part': name, 'twists': len(part), 'input_digest': d_in, 'output_digest': st.digest(),
                     'off_end': int((cn < 0).sum()), 'lattice_of_start_off': int((lat0 & (cn < 0)).sum())})
        (out / 'parts' / f'{name}.json').write_text(json.dumps({'word': [list(y) for y in part], 'input_digest': d_in,
                                                                'output_digest': rows[-1]['output_digest']}) + '\n')
        if name == 'Qprime':
            rows[-1]['holes_equal_T'] = bool(np.array_equal(np.sort(B.hole_set(st)), np.sort(badPinv)))
        log(part=name, twists=len(part), off=rows[-1]['off_end'])
    ce = codes.of(st)
    end = {'digest': st.digest(), 'off': int((ce < 0).sum()), 'bprime_restored': int((ce[off0] >= 0).sum()),
           'collateral': int((lat0 & (ce < 0)).sum()), 'lattice': int(lat0.sum()),
           'lattice_on_K_at_end': int((lat0 & (ce >= 0)).sum()),
           'lattice_same_exact_pose': int((lat0 & (ce == c0)).sum())}
    full = [list(y) for _, part in parts for y in part]
    (out / 'word-full.json').write_text(json.dumps({'word': full, 'input_digest': d0,
                                                    'output_digest': end['digest']}) + '\n')
    res = {'input_digest': d0, 'x': xw, 'parts': rows, 'end': end, 'length': len(full),
           'replay_sec': round(time.time() - t, 1), 'seconds': round(time.time() - T0, 1)}
    (out / 'result.json').write_text(json.dumps(res, indent=1) + '\n')
    log(end=end, length=len(full))


if __name__ == '__main__':
    main()
