"""Close a restoring word W at the state X: vacate Bad(W) by a SAFE-ALL word V, then replay V W V^-1 exactly.

Bad(W) is read by the exact frame ghost (chambers whose lattice content would straddle or end off the lattice under
W); the ghost also checks that W puts the block T on the lattice and returns every other off piece to its exact pose.
V is a SAFE-ALL word at X (K+ letters of caps with no off piece and no straddled cut) after which no lattice piece is
in Bad(W): the peeling certificate of buffer.py (conjprep.generic_certificate / generic_prepare), else the greedy of
vacate.py. Then V, W and V^-1 are applied to the simulator state, every twist checked admissible, and the end is
compared with X. Reads only the state.

Usage: python -I finishw.py <theory dir> <pose table> <block size> <W json file or list> <out prefix>
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
import conjclose as CC  # noqa: E402
import conjprep as CP  # noqa: E402
import ghostlib as GL  # noqa: E402
import vacate as VA  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0))), file=sys.stderr, flush=True)


def main():
    a = sys.argv[1:]
    table, size, warg, prefix = a[1], int(a[2]), a[3], a[4]
    W = json.loads(Path(warg).read_text()) if Path(warg).exists() else json.loads(warg)
    C = GL.Ctx(table)
    R, M, st = C.R, C.M, C.st
    parts = None
    if isinstance(W, dict) and 'P' in W:
        P = [(int(c), int(i)) for c, i in W['P']]
        Q = [(int(c), int(i)) for c, i in W['Q']]
        parts = {'P': len(P), 'Q': len(Q), 'P_inverse': len(P)}
        W = P + Q + CP.inverse_word(M, P)
    elif isinstance(W, dict):
        W = W['word']
    W = [(int(c), int(i)) for c, i in W]
    T = C.block(size)
    prot = np.setdiff1d(C.off, T)
    g, n, U = C.ghost(W, np.concatenate([T, prot]))
    nT = len(T)
    d0, q0 = C.contents(prot)
    lat = slice(n, None)
    badW = np.sort(U[~g.alive[lat] | (g.dom[lat] != 0)])
    pred = {'T_on': int((g.alive[:nT] & (g.dom[:nT] == 0)).sum()),
            'prot_exact': bool(g.alive[nT:n].all() and np.array_equal(g.dom[nT:n], d0) and np.array_equal(g.q[nT:n], q0)),
            'badW': int(len(badW)), 'badW_occupied_at_X': int(C.occ[badW].sum()), 'landing': g.q[:nT].tolist()}
    log(**{k: v for k, v in pred.items() if k != 'landing'})
    res = {'input_digest': st.digest(), 'W_length': len(W), 'W_parts': parts, 'W': [list(x) for x in W],
           'ghost_prediction': pred, 'badW': badW.tolist()}
    Gal = B.Alphabet(R, M)
    Gal.orbits()
    cond = B.condition(Gal, badW, None, None)
    res['condition'] = {'N1_fixed_vacant': cond['fixed_vacant'], 'fixed_occupied': cond['fixed_occupied'],
                        'N2_necessary': cond['necessary'], 'deficits': cond['necessary_deficits'],
                        'safe_grips': len(Gal.safe), 'holes': int(len(Gal.holes))}
    log(N1=cond['fixed_vacant'], N2=cond['necessary'], deficits=cond['necessary_deficits'][:10])
    V = None
    if cond['necessary']:
        try:
            cert = CP.generic_certificate(Gal, badW)
            prep, _, deficits = CP.generic_prepare(Gal, cert, badW)
            res['certificate'] = {'peeled': cert['peeled'], 'of': cert['of'], 'deficits': deficits}
            if prep is not None:
                V = [tuple(x) for x in prep['word']]
                res['V_source'] = 'certificate'
        except Exception as exc:  # noqa: BLE001
            res['certificate_error'] = str(exc)[:300]
            log(certificate_error=res['certificate_error'])
        if V is None:
            mask = np.zeros(Gal.NP, bool)
            mask[badW] = True
            va = VA.Vacate(Gal, mask, Gal.holes)
            word, steps, ok = va.run()
            res['greedy'] = {'ok': ok, 'steps': len(steps), 'length': len(word)}
            if ok:
                V = [tuple(x) for x in Gal.pairs(word)]
                res['V_source'] = 'greedy'
    if V is None:
        res['result'] = 'no V'
        Path(prefix + '.json').write_text(json.dumps(res, indent=1) + '\n')
        log(result='no V')
        return
    log(V=len(V), source=res['V_source'])
    codes = CC.Codes()
    c_x = codes.of(st)
    d_x = st.digest()
    pose_x = {int(p): to_tuple(st.pose(int(p))) for p in np.flatnonzero(c_x < 0)}
    for c, i in V:
        R.apply(c, i)
    occ_v = np.ones(Gal.NP, bool)
    occ_v[B.hole_set(st)] = False
    res['after_V'] = {'digest': st.digest(), 'occupied_badW': int(occ_v[badW].sum()),
                      'off_set_unchanged': bool(np.array_equal(codes.of(st) < 0, c_x < 0)),
                      'off_exact_poses_unchanged': all(to_tuple(st.pose(p)) == m for p, m in pose_x.items())}
    log(after_V=res['after_V'])
    for c, i in W:
        R.apply(c, i)
    Vinv = CP.inverse_word(M, V)
    for c, i in Vinv:
        R.apply(c, i)
    c_e = codes.of(st)
    inT = np.zeros(len(c_x), bool)
    inT[T] = True
    off_x, off_e = c_x < 0, c_e < 0
    res['end'] = {'digest': st.digest(), 'off': int(off_e.sum()), 'T_restored': int((inT & ~off_e).sum()),
                  'collateral': int((~off_x & off_e).sum()),
                  'prot_pose_changed': int((off_x & ~inT & (c_e != c_x)).sum()),
                  'lattice_of_X_moved_on_K': int((~off_x & ~off_e & (c_e != c_x)).sum())}
    full = [list(x) for x in V] + [list(x) for x in W] + [list(x) for x in Vinv]
    res['word'] = {'length': len(full), 'V': len(V), 'W': len(W), 'V_inverse': len(Vinv)}
    Path(prefix + '-word.json').write_text(json.dumps({'word': full, 'input_digest': d_x,
                                                       'output_digest': res['end']['digest']}) + '\n')
    res['seconds'] = round(time.time() - T0)
    Path(prefix + '.json').write_text(json.dumps(res, indent=1) + '\n')
    log(end=res['end'], word=res['word'])


if __name__ == '__main__':
    main()
