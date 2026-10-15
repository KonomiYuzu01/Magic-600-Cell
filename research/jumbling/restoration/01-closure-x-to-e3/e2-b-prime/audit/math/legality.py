"""Audit (math): the five legality points of W3 = V3 P4 F P4^-1 V3^-1 at E2, recomputed.

- Bad(W), W = P4 F P4^-1, by the simulator-kernel ghost over all 177,120 chambers and by fastghost; dying chambers.
- B''s exact content ghost through W from its E2 pose, by fastghost and by the simulator ghost (alive, on K, landing).
- On the simulator: V3 at E2 (off set and exact poses, holes against Bad(W)); P4 at E2.V3 (parked set against the
  parked set of P4 at E2, exact poses of the parked pieces, B' chambers); F's grip classified directly at E2.V3.P4
  (SAFE-ALL?) and at E2.P4; F; P4^-1; V3^-1 (off counts, every piece on K, lattice pieces of E2 on K).
- Support of V3 against the caps of P4 and F; Bad(U) of the whole word by the simulator ghost; pi_V3^-1(Bad(W)).

Usage: python3 -I legality.py <theory dir> <v26 dir> <E2 table> <parts dir> <step-L4 json> <out json>
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
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import blocks as BL  # noqa: E402
import conjclose as CC  # noqa: E402
import conjprep as CP  # noqa: E402
import ghostlib as GL  # noqa: E402
import restore  # noqa: E402

CL = BL.CL
reg, kp, data = CL.ctx.regions, CL.ctx.kplus, CL.ctx.data
T0 = time.time()
BPRIME = np.array([16245, 16249, 32088], np.int64)


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def chambers(st):
    K = st._kpose()[st.pose_id]
    ch = np.full(len(K), -1, np.int64)
    lat = np.flatnonzero(K >= 0)
    ch[lat] = reg.piece_of[reg.orbit_of[lat], kp.compose_vec(K[lat], reg.transport[lat])]
    return ch, K


def holes_of(st):
    ch, K = chambers(st)
    occ = np.zeros(len(K), bool)
    occ[ch[K >= 0]] = True
    return np.flatnonzero(~occ)


def safe_all(st, c):
    K = st._kpose()[st.pose_id]
    cl = st.classify(c, need_inside=True, stop_at_straddle=False)
    inside = np.asarray(cl.inside, np.int64)
    return {'uncertain': bool(cl.uncertain), 'straddled': cl.straddle is not None,
            'off_inside': int((K[inside] < 0).sum()) if len(inside) else 0,
            'safe': (not cl.uncertain) and cl.straddle is None and not (len(inside) and (K[inside] < 0).any())}


def word_perm(M, word, NP):
    """Chamber permutation (source -> destination) of a word of K+ letters."""
    p = np.arange(NP)
    for c, i in word:
        k = kp.index_of_matrix(restore.element(M, c, i))
        if k < 0:
            raise ValueError('not a K+ letter')
        xs = np.flatnonzero(GL.cap(c))
        ys = reg.piece_of[reg.orbit_of[xs], kp.compose_vec(np.full(len(xs), k), reg.transport[xs])]
        lp = np.arange(NP)
        lp[xs] = ys
        p = lp[p]
    return p


def main():
    a = sys.argv[1:]
    table, parts, l4file, outp = a[2], Path(a[3]), a[4], Path(a[5])
    W = {n: [tuple(x) for x in json.loads((parts / f'{n}.json').read_text())['word']]
         for n in ('V', 'P', 'Q', 'Pinv', 'Vinv')}
    C = GL.Ctx(table)
    R, M, st = C.R, C.M, C.st
    NP = len(st.pose_id)
    d0 = st.digest()
    codes = CC.Codes()
    code0 = codes.of(st)
    ch0, K0 = chambers(st)
    lat0 = K0 >= 0
    holes0 = holes_of(st)
    res = {'E2_digest': d0, 'E2_holes': holes0.tolist()}
    # inverse parts are exact mirror inverses
    res['Pinv_is_mirror_inverse'] = CP.inverse_word(M, W['P']) == W['Pinv']
    res['Vinv_is_mirror_inverse'] = CP.inverse_word(M, W['V']) == W['Vinv']
    Wc = W['P'] + W['Q'] + W['Pinv']
    U = W['V'] + Wc + W['Vinv']
    # Bad(W) two ways
    badm, alive = CP.bad_chambers(M, Wc, NP)
    badW = np.flatnonzero(badm)
    bw1, g1, chs1 = C.bad(Wc)
    res['badW_simghost'] = badW.tolist()
    res['badW_fastghost'] = np.sort(bw1).tolist()
    res['badW_kill_simghost'] = int((~alive).sum())
    res['badW_kill_fastghost'] = int((~g1.alive).sum())
    log(badW=res['badW_simghost'], fast=res['badW_fastghost'])
    # B' content ghost through W from E2, two ways
    gB, n, _ = C.ghost(Wc, BPRIME, np.zeros(0, np.int64))
    res['Bprime_fastghost'] = {'alive': gB.alive.tolist(), 'on_K': (gB.dom == 0).tolist(), 'land': gB.q.tolist()}
    a2, fb2, gs = CC.ghost_run(M, Wc, None, BPRIME, st)
    Kg = gs._kpose()[gs.pose_id]
    land2 = reg.piece_of[reg.orbit_of[BPRIME], kp.compose_vec(np.maximum(Kg[BPRIME], 0), reg.transport[BPRIME])]
    res['Bprime_simghost'] = {'alive': a2[BPRIME].tolist(), 'on_K': (Kg[BPRIME] >= 0).tolist(), 'land': land2.tolist()}
    log(bprime=res['Bprime_simghost'])
    # simulator replay with stage checks
    applied = 0
    for c, i in W['V']:
        R.apply(c, i)
        applied += 1
    cv = codes.of(st)
    hv = holes_of(st)
    res['after_V'] = {'off_set_is_Bprime': bool(np.array_equal(np.flatnonzero(cv < 0), BPRIME)),
                      'Bprime_exact_E2': bool(np.array_equal(cv[BPRIME], code0[BPRIME])),
                      'holes': hv.tolist(), 'holes_equal_badW': bool(np.array_equal(hv, badW))}
    piV = word_perm(M, W['V'], NP)
    res['piV_holes_E2_equal_badW'] = bool(np.array_equal(np.sort(piV[holes0]), badW))
    res['piV_inverse_badW_equal_E2_holes'] = bool(np.array_equal(np.sort(np.argsort(piV)[badW]), holes0))
    suppV = np.flatnonzero(piV != np.arange(NP))
    capsPF = np.zeros(NP, bool)
    for c in sorted({c for c, _ in Wc}):
        capsPF |= GL.cap(c)
    res['V_support_size'] = int(len(suppV))
    res['V_support_meets_caps_of_P4_F'] = int(capsPF[suppV].sum())
    res['V_grips'] = sorted({c for c, _ in W['V']})
    for c, i in W['P']:
        R.apply(c, i)
        applied += 1
    cp = codes.of(st)
    chp, Kp = chambers(st)
    parkVP = np.flatnonzero(lat0 & (cp < 0))
    l4 = json.loads(Path(l4file).read_text())['levels']
    l4 = [r for r in l4 if r['level'] == 4][0]
    parkP = np.array(l4['parked_ids'], np.int64)
    res['after_V_P4'] = {'off': int((cp < 0).sum()), 'parked': parkVP.tolist(),
                         'parked_equals_parked_at_E2_P4': bool(np.array_equal(np.sort(parkVP), np.sort(parkP))),
                         'Bprime_on_K': int((Kp[BPRIME] >= 0).sum()), 'L': chp[BPRIME].tolist(),
                         'L_equals_L4': chp[BPRIME].tolist() == l4['L'],
                         'F_grip_status': safe_all(st, W['Q'][0][0])}
    for c, i in W['Q']:
        R.apply(c, i)
        applied += 1
    cq = codes.of(st)
    res['after_F'] = {'off': int((cq < 0).sum()),
                      'off_set_and_poses_unchanged': bool(np.array_equal(cq[cp < 0], cp[cp < 0])
                                                          and np.array_equal(cq < 0, cp < 0))}
    for c, i in W['Pinv']:
        R.apply(c, i)
        applied += 1
    ci, Ki = chambers(st)
    res['after_Pinv'] = {'off': int((Ki < 0).sum()), 'all_on_K': bool((Ki >= 0).all())}
    # point 5: every grip SAFE-ALL at this state
    grip_status = [safe_all(st, c)['safe'] for c in sorted({c for c, _ in W['Vinv']})]
    res['after_Pinv']['Vinv_grips_safe_all'] = all(grip_status)
    for c, i in W['Vinv']:
        R.apply(c, i)
        applied += 1
    ce, Ke = chambers(st)
    res['end'] = {'digest': st.digest(), 'off': int((Ke < 0).sum()), 'E2_lattice_on_K': int((lat0 & (Ke >= 0)).sum()),
                  'E2_lattice': int(lat0.sum()), 'holes': int(len(holes_of(st)))}
    for _ in range(applied):
        R.undo()
    if st.digest() != d0:
        raise AssertionError('undo did not return to E2')
    # F at E2.P4 (Y_4) for comparison
    n4 = 0
    for c, i in W['P']:
        R.apply(c, i)
        n4 += 1
    res['F_grip_status_at_Y4'] = safe_all(st, W['Q'][0][0])
    for _ in range(n4):
        R.undo()
    # Bad(U), whole word, simulator ghost over all chambers
    t = time.time()
    badU, aliveU = CP.bad_chambers(M, U, NP)
    res['badU_simghost'] = np.flatnonzero(badU).tolist()
    res['badU_equals_E2_holes'] = bool(np.array_equal(np.flatnonzero(badU), holes0))
    res['badU_kill'] = int((~aliveU).sum())
    res['badU_sec'] = round(time.time() - t, 1)
    res['seconds'] = round(time.time() - T0, 1)
    outp.write_text(json.dumps(res, indent=1) + '\n')
    log(done=True)


if __name__ == '__main__':
    main()
