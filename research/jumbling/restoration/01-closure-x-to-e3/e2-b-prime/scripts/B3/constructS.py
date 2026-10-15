"""Construct and replay a type-S restoration of B' at the current state: U = P Q0 F P^-1 (reads only the pose table).

P puts B' on K and parks lattice pieces; T = Bad(P^-1) (word-only ghost). F is a short SAFE-ALL word at the state
after P with sigma_F(BH) inside T (BH: the holes in T); N = sigma_F^-1(T) minus BH are the chambers that must hold holes
before F. Q0 routes holes onto N with H0 letters only (SAFE-ALL grips whose caps avoid T, so Q0 fixes T pointwise): the
peeling certificate (Theorem B) and its greedy procedure on the alphabet restricted to H0, target N. Before any twist
of Q0 or F is applied, the script records the premises: F's condition, N outside T, N's H0 orbits against the holes
outside T, and the certificate with its thresholds. Then it replays P, Q0, F, P^-1 on the simulator (every twist
checked admissible) and compares the end with the start: B' restored, collateral, lattice kept, exact pose kept.

Usage: python -I constructS.py <theory dir> <pose table> <P json word> <F json word> <out dir>
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
from sim.kplus import to_tuple  # noqa: E402

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
    out = Path(a[4])
    out.mkdir(parents=True, exist_ok=True)
    times = {}
    t = time.time()
    C = GL.Ctx(a[1])
    R, M, st = C.R, C.M, C.st
    from blocks import CL
    reg, kp = CL.ctx.regions, CL.ctx.kplus
    codes = CC.Codes()
    c0 = codes.of(st)
    d0 = st.digest()
    off0 = np.flatnonzero(c0 < 0)
    lat0 = c0 >= 0
    poses0 = {int(p): to_tuple(st.pose(int(p))) for p in off0}
    P = [tuple(x) for x in json.loads(a[2])]
    Fw = [tuple(x) for x in json.loads(a[3])]
    Pinv = CP.inverse_word(M, P)
    badPinv, _, _ = C.bad(Pinv)
    for c, i in P:
        R.apply(c, i)
    d_p = st.digest()
    G = B.Alphabet(R, M)
    G.orbits()
    times['after_P_alphabet'] = round(time.time() - t, 1)
    t = time.time()
    holes = np.asarray(G.holes)
    T = np.zeros(G.NP, bool)
    T[badPinv] = True
    BH = holes[T[holes]]
    GH = holes[~T[holes]]
    isBH = np.zeros(G.NP, bool)
    isBH[BH] = True
    Kp = st._kpose()[st.pose_id]
    L = reg.piece_of[reg.orbit_of[off0], kp.compose_vec(Kp[off0], reg.transport[off0])]
    F = G.numbers([list(x) for x in Fw])
    caps = sorted({G.letters[n][0] for n in F})
    U = np.unique(np.concatenate([G.capx[c] for c in caps]))
    img = G.images(F, U)
    into_T = U[T[img]]
    N = np.sort(into_T[~isBH[into_T]])
    pre = {'state_digest': d0, 'P': [list(x) for x in P], 'after_P_digest': d_p, 'T': int(T.sum()),
           'BH': int(len(BH)), 'GH': GH.tolist(), 'L': L.tolist(), 'F': [list(x) for x in Fw],
           'F_bh_into_T': bool(T[img[isBH[U]]].all()), 'N': N.tolist(), 'N_outside_T': bool(not T[N].any()),
           'N_count_equals_GH': int(len(N)) == int(len(GH))}
    h0 = [c for c in G.safe if not T[G.capx[c]].any()]
    G0 = restrict(G, h0, kp)
    pre['h0_grips'] = len(h0)
    pre['h0_letters'] = len(G0.letters)
    pre['N_h0_orbits'] = [int(G0.orbit[x]) for x in N]
    pre['GH_h0_orbits'] = [int(G0.orbit[x]) for x in GH]
    times['premises'] = round(time.time() - t, 1)
    log(pre={k: v for k, v in pre.items() if k not in ('BH',)})
    ok = pre['F_bh_into_T'] and pre['N_outside_T'] and pre['N_count_equals_GH']
    Q0 = None
    if ok:
        t = time.time()
        try:
            cert = CP.generic_certificate(G0, N)
            counts = np.bincount(G0.orbit[holes], minlength=len(G0.orbit_size))
            suff = {o: {'threshold': int(v), 'holes': int(counts[int(o)])} for o, v in cert['thresholds'].items()}
            pre['certificate'] = {'peeled': cert['peeled'], 'of': cert['of'], 'fixed': cert['fixed'],
                                  'thresholds': suff, 'all_reached': cert['all_reached'],
                                  'sufficient': all(x['holes'] >= x['threshold'] for x in suff.values())}
            (out / 'certificate-Q0.json').write_text(json.dumps(cert) + '\n')
            times['certificate'] = round(time.time() - t, 1)
            log(certificate={k: v for k, v in pre['certificate'].items() if k != 'thresholds'})
            t = time.time()
            prep, _, deficits = CP.generic_prepare(G0, cert, N)
            pre['prepare_deficits'] = {str(k): v for k, v in deficits.items()}
            if prep is not None:
                Q0 = [tuple(x) for x in prep['word']]
                pre['Q0_predicted_holes_cover_N'] = bool(np.isin(N, prep['predicted_holes']).all())
            times['prepare'] = round(time.time() - t, 1)
        except Exception as exc:  # noqa: BLE001
            pre['certificate_error'] = str(exc)[:400]
            log(certificate_error=pre['certificate_error'])
    (out / 'pre.json').write_text(json.dumps(pre, indent=1) + '\n')
    for _ in P:
        R.undo()
    if st.digest() != d0:
        raise AssertionError('state not restored after undo')
    if Q0 is None:
        (out / 'result.json').write_text(json.dumps({'result': 'no Q0', 'times': times}, indent=1) + '\n')
        log(result='no Q0', times=times)
        return
    t = time.time()
    parts = [('P', P), ('Q0', Q0), ('F', Fw), ('Pinv', Pinv)]
    rows = []
    (out / 'parts').mkdir(exist_ok=True)
    for name, part in parts:
        d_in = st.digest()
        for c, i in part:
            R.apply(c, i)
        d_out = st.digest()
        cn = codes.of(st)
        (out / 'parts' / f'{name}.json').write_text(json.dumps({'word': [list(x) for x in part], 'input_digest': d_in,
                                                                'output_digest': d_out}) + '\n')
        rows.append({'part': name, 'twists': len(part), 'input_digest': d_in, 'output_digest': d_out,
                     'off_end': int((cn < 0).sum()), 'lattice_of_start_off': int((lat0 & (cn < 0)).sum())})
        if name == 'F':
            hl = B.hole_set(st)
            rows[-1]['holes_equal_T'] = bool(np.array_equal(np.sort(hl), np.sort(badPinv)))
        log(part=name, twists=len(part), off=rows[-1]['off_end'])
    times['replay'] = round(time.time() - t, 1)
    ce = codes.of(st)
    end = {'digest': st.digest(), 'off': int((ce < 0).sum()), 'bprime_restored': int((ce[off0] >= 0).sum()),
           'collateral': int((lat0 & (ce < 0)).sum()), 'lattice': int(lat0.sum()),
           'lattice_on_K_at_end': int((lat0 & (ce >= 0)).sum()),
           'lattice_same_exact_pose': int((lat0 & (ce == c0)).sum())}
    full = [list(x) for _, part in parts for x in part]
    (out / 'word-full.json').write_text(json.dumps({'word': full, 'input_digest': d0,
                                                    'output_digest': end['digest']}) + '\n')
    res = {'input_digest': d0, 'parts': rows, 'end': end, 'length': len(full), 'times': times,
           'seconds': round(time.time() - T0, 1)}
    (out / 'result.json').write_text(json.dumps(res, indent=1) + '\n')
    log(end=end, length=len(full), times=times)


if __name__ == '__main__':
    main()
