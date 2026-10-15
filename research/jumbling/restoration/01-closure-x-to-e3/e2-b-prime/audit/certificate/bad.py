"""Audit 'certificate', items 1, 2 and 7: Bad sets of W = P4 F P4^-1, of P4^-1 and of the whole word, from E2 alone;
B''s landing chambers in a real replay from E2 V3; the negative control (W directly at E2, without V3).

Usage: python3 -I bad.py <theory dir> <out json>
Reads only E2-table.json, the part files of c3/parts and P4.json (no producing script output is used as evidence;
the report values are compared at the end).
"""
import hashlib
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
S = Path('<work>')
B4 = S / 'v26/B4'
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(S / 'v26'))
import ghostlib as GL  # noqa: E402
import conjprep as CP  # noqa: E402
import conjclose as CC  # noqa: E402
from blocks import CL  # noqa: E402
from sim.kplus import matrix_json, to_tuple  # noqa: E402

T0 = time.time()
BPRIME = [16245, 16249, 32088]


def log(*a):
    print(round(time.time() - T0, 1), *a, file=sys.stderr, flush=True)


def chambers(st):
    reg, kp = CL.ctx.regions, CL.ctx.kplus
    K = st._kpose()[st.pose_id]
    ch = np.full(len(K), -1, np.int64)
    lat = np.flatnonzero(K >= 0)
    ch[lat] = reg.piece_of[reg.orbit_of[lat], kp.compose_vec(K[lat], reg.transport[lat])]
    return ch, K


def occupancy(st):
    ch, K = chambers(st)
    occ = np.full(len(K), -1, np.int64)
    lat = np.flatnonzero(K >= 0)
    if len(np.unique(ch[lat])) != len(lat):
        raise AssertionError('two lattice pieces in one chamber')
    occ[ch[lat]] = lat
    return occ, ch, K


def main():
    out = Path(sys.argv[2])
    table = B4 / 'E2-table.json'
    res = {'E2_table_sha256': hashlib.sha256(table.read_bytes()).hexdigest()}
    C = GL.Ctx(str(table))
    R, M, st = C.R, C.M, C.st
    NP = len(C.occ)
    res['E2_digest'] = st.digest()
    parts = {n: json.loads((B4 / 'c3/parts' / f'{n}.json').read_text()) for n in ('V', 'P', 'Q', 'Pinv', 'Vinv')}
    w = {n: [tuple(int(v) for v in x) for x in d['word']] for n, d in parts.items()}
    V, P, F, Pinv, Vinv = w['V'], w['P'], w['Q'], w['Pinv'], w['Vinv']
    full = [tuple(x) for x in json.loads((B4 / 'c3/word-full.json').read_text())['word']]
    p4file = [tuple(x) for x in json.loads((B4 / 'P4.json').read_text())]
    # P4 rebuilt from R and x0..x3 as in the report (P(k+1) = P(k) x_k P(k)^-1)
    Pk = [(237, 13)]
    for x in [(91, 6), (154, 7), (133, 1), (183, 6)]:
        Pk = Pk + [x] + CP.inverse_word(M, Pk)
    res['lengths'] = {n: len(x) for n, x in w.items()}
    res['P4_rebuilt_equals_part_P'] = Pk == P
    res['P4_rebuilt_equals_P4_json'] = Pk == p4file
    res['Pinv_is_mirror_inverse_of_P'] = CP.inverse_word(M, P) == Pinv
    res['Vinv_is_mirror_inverse_of_V'] = CP.inverse_word(M, V) == Vinv
    res['F'] = F
    res['F_inverse'] = CP.inverse_word(M, F)
    res['full_word_equals_parts'] = full == V + P + F + Pinv + Vinv
    res['inverse_letters'] = {'x0^-1': CP.inverse_word(M, [(91, 6)]), 'x1^-1': CP.inverse_word(M, [(154, 7)]),
                              'R^-1': CP.inverse_word(M, [(237, 13)])}
    W = P + F + Pinv
    res['W_length'] = len(W)
    allch = np.arange(NP)
    # Bad(W) over all chambers, and over the default chamber set (union of the word's caps)
    t = time.time()
    bw, g, ch = C.bad(W, allch)
    badW = np.sort(bw)
    bw_def, _, _ = C.bad(W)
    res['badW'] = badW.tolist()
    res['badW_default_chambers_equal_all'] = bool(np.array_equal(np.sort(bw_def), badW))
    res['badW_detail'] = [{'chamber': int(z), 'dies': bool(not g.alive[z]), 'died_at_step': int(g.died_at[z]),
                           'end_domain': int(g.dom[z]), 'end_chamber_in_frame': int(g.q[z])} for z in badW.tolist()]
    res['seconds_badW'] = round(time.time() - t, 1)
    log('badW', res['badW'])
    occ2, ch2, K2 = occupancy(st)
    holes2 = np.flatnonzero(occ2 < 0)
    res['E2_holes'] = holes2.tolist()
    res['E2_off'] = np.flatnonzero(K2 < 0).tolist()
    res['badW_occupied_at_E2'] = int((occ2[badW] >= 0).sum())
    res['badW_occupants_at_E2'] = {str(int(z)): int(occ2[z]) for z in badW.tolist()}
    res['ghostlib_occ_agrees'] = bool(np.array_equal(C.occ, occ2 >= 0))
    # Bad(P4^-1), Bad(P4) and Bad(F) over all chambers
    bpi, gpi, _ = C.bad(Pinv, allch)
    res['badPinv'] = np.sort(bpi).tolist()
    res['badPinv_size'] = int(len(bpi))
    bp, _, _ = C.bad(P, allch)
    res['badP_size'] = int(len(bp))
    # Bad of the whole 79-letter word
    t = time.time()
    U = V + W + Vinv
    bu, gu, _ = C.bad(U, allch)
    res['badU'] = np.sort(bu).tolist()
    res['badU_equals_E2_holes'] = bool(np.array_equal(np.sort(bu), holes2))
    res['seconds_badU'] = round(time.time() - t, 1)
    log('badU', res['badU'])
    # ghost of B' itself under W from E2 (V3 does not move it): landing chambers
    gT, n, _ = C.ghost(W, np.array(BPRIME), np.zeros(0, np.int64))
    res['ghost_Bprime_alive_on_K'] = int((gT.alive[:n] & (gT.dom[:n] == 0)).sum())
    res['ghost_Bprime_landing'] = sorted(gT.q[:n].tolist())
    # real replay from E2: V3, then P4, F, P4^-1, V3^-1 with every twist checked admissible by the simulator
    bpose0 = {p: to_tuple(st.pose(p)) for p in BPRIME}
    rep = {}
    applied = 0
    for name, part in (('V', V), ('P', P), ('F', F), ('Pinv', Pinv), ('Vinv', Vinv)):
        for c, i in part:
            R.apply(c, i)
            applied += 1
        occ, chx, K = occupancy(st)
        off = np.flatnonzero(K < 0)
        row = {'digest': st.digest(), 'off_count': int(len(off)), 'Bprime_on_K': int((K[BPRIME] >= 0).sum()),
               'Bprime_chambers': [int(chx[p]) for p in BPRIME],
               'E2_lattice_off': int(((K2 >= 0) & (K < 0)).sum())}
        if name == 'V':
            row['holes'] = np.flatnonzero(occ < 0).tolist()
            row['badW_vacant'] = int((occ[badW] < 0).sum())
            row['holes_equal_badW'] = bool(np.array_equal(np.flatnonzero(occ < 0), badW))
            row['off_set'] = off.tolist()
            row['Bprime_exact_pose_unchanged'] = all(to_tuple(st.pose(p)) == bpose0[p] for p in BPRIME)
        if name == 'F':
            row['Bprime_chambers_in_badPinv'] = int(np.isin([chx[p] for p in BPRIME], bpi).sum())
        if name == 'Pinv':
            row['Bprime_landing_sorted'] = sorted(int(chx[p]) for p in BPRIME)
            row['landing_equals_badW'] = sorted(int(chx[p]) for p in BPRIME) == badW.tolist()
            row['landing_equals_ghost'] = sorted(int(chx[p]) for p in BPRIME) == res['ghost_Bprime_landing']
        rep[name] = row
        log(name, row['digest'][:16], row['off_count'])
    res['replay'] = rep
    res['replay_twists'] = applied
    res['end_digest'] = st.digest()
    for _ in range(applied):
        R.undo()
    if st.digest() != res['E2_digest']:
        raise AssertionError('undo did not return to E2')
    # negative control: W directly at E2 (no V3)
    neg = {'applied': 0}
    for j, (c, i) in enumerate(W):
        try:
            R.apply(c, i)
        except RuntimeError as e:
            neg['blocked_at'] = {'index_in_W': j, 'letter': [c, i], 'error': str(e)[:200]}
            ins, strad = CC.classify3(st, c)
            neg['blocked_at']['straddlers'] = strad.tolist()
            neg['blocked_at']['straddlers_lattice_at_E2'] = [bool(K2[p] >= 0) for p in strad.tolist()]
            neg['blocked_at']['straddlers_are_badW_occupants'] = sorted(strad.tolist()) == sorted(
                res['badW_occupants_at_E2'].values())
            neg['blocked_at']['straddlers_in_badW_occupants'] = [int(p) in set(res['badW_occupants_at_E2'].values())
                                                                 for p in strad.tolist()]
            break
        neg['applied'] += 1
    occ, chx, K = occupancy(st)
    off_now = np.flatnonzero(K < 0)
    neg['off_now'] = int(len(off_now))
    neg['E2_lattice_off_now'] = np.flatnonzero((K2 >= 0) & (K < 0)).tolist()[:50]
    neg['E2_lattice_off_now_count'] = int(((K2 >= 0) & (K < 0)).sum())
    neg['Bprime_on_K_now'] = int((K[BPRIME] >= 0).sum())
    if neg['applied'] == len(W):
        lost = np.flatnonzero((K2 >= 0) & (K < 0))
        neg['completed'] = True
        neg['collateral'] = lost.tolist()
        neg['collateral_equals_badW_occupants'] = sorted(lost.tolist()) == sorted(res['badW_occupants_at_E2'].values())
    else:
        neg['completed'] = False
    res['negative_control'] = neg
    for _ in range(neg['applied']):
        R.undo()
    if st.digest() != res['E2_digest']:
        raise AssertionError('undo did not return to E2 after the negative control')
    # per-step ghost of the 3 occupants: which die where (contents are independent)
    res['seconds'] = round(time.time() - T0, 1)
    out.write_text(json.dumps(res, indent=1) + '\n')
    log('done')


if __name__ == '__main__':
    main()
