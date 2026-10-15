"""Extra cross-checks on my own replayed states (exports of replay_audit.py), cheap and adjacent to the replay.

1. Q2 and the parked set C: from my after-V state apply P2, take C (E1-lattice pieces off at prefix 600), then for
   each Q2 letter count moved pieces of C; net-moved set of Q2 (exact pose differs between prefixes 600 and 610).
2. T's landing after W2 (prefix 620, my after-Pinv export) against pre.json ghost.badW / T_landing_chambers, and the
   holes after V2 (my after-V export): badW inside them, the rest, and badW occupancy at E1.
3. K+-orbit types of the E2 holes against those of B'.
4. X (segment-1 input table): B' exact pose at X, and the pieces whose exact pose differs between X and E2.
"""
import json
import sys
import time
from pathlib import Path

sys.dont_write_bytecode = True
S = Path('<work>')
TH = S / 'wt-e3/research/jumbling/theory'
C0 = S / 'v26/E2/c0'
OUT = S / 'v26/audit/replay'
PKG = S / 'pkg26/Claude-E-closure-X1-20261012'
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import numpy as np  # noqa: E402

import blocks as BL  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

O = BL.O
CTX = BL.CL.ctx
BP = [16245, 16249, 32088]
T0 = time.time()


def log(**kw):
    print(json.dumps(dict(kw, sec=round(time.time() - T0, 1))), file=sys.stderr, flush=True)


def kposes(st):
    return st._kpose()[st.pose_id]


def chambers(st, pieces):
    reg, kp = CTX.regions, CTX.kplus
    K = kposes(st)
    pieces = np.asarray(pieces, np.int64)
    assert (K[pieces] >= 0).all()
    return reg.piece_of[reg.orbit_of[pieces], kp.compose_vec(K[pieces], reg.transport[pieces])]


def holes(st):
    K = kposes(st)
    lat = np.flatnonzero(K >= 0)
    occ = np.zeros(len(K), bool)
    occ[chambers(st, lat)] = True
    return np.flatnonzero(~occ)


def exact_keys(st):
    """Per-piece exact pose as a hashable key (K index for K+ poses, exact matrix otherwise)."""
    return [st._poses[i].key for i in range(len(st._poses))], st.pose_id.copy()


def main():
    res = {}
    pre = json.loads((C0 / 'pre.json').read_text())
    badW = sorted(pre['ghost']['badW'])
    landing = sorted(pre['ghost']['T_landing_chambers'])
    T = sorted(pre['target_block'])
    # E1
    R1, _ = O.start(str(C0 / 'E1-table.json'))
    e1 = R1.st
    K1 = kposes(e1)
    off1 = K1 < 0
    occ1 = np.ones(len(K1), bool)
    occ1[holes(e1)] = False
    # 1. Q2 and C
    R, M = O.start(str(OUT / 'state-after-V.json'))
    st = R.st
    P = [tuple(x) for x in json.loads((C0 / 'parts/P.json').read_text())['word']]
    Q = [tuple(x) for x in json.loads((C0 / 'parts/Q.json').read_text())['word']]
    for c, i in P:
        R.apply(c, i)
    K = kposes(st)
    C = np.flatnonzero((K < 0) & ~off1)
    keys600 = [st._poses[i].key for i in st.pose_id.tolist()]
    prev = keys600
    per_letter = []
    for k, (c, i) in enumerate(Q):
        R.apply(c, i)
        now = [st._poses[j].key for j in st.pose_id.tolist()]
        moved = [p for p in range(len(now)) if now[p] != prev[p]]
        ms = set(moved)
        per_letter.append({'letter': [c, i], 'moved': len(moved), 'moved_C': sum(1 for p in C.tolist() if p in ms),
                           'moved_T': sum(1 for p in T if p in ms)})
        prev = now
    net = [p for p in range(len(prev)) if prev[p] != keys600[p]]
    res['Q2_and_C'] = {'C_size': int(len(C)), 'per_letter': per_letter,
                       'letters_selecting_C': sum(1 for r in per_letter if r['moved_C'] > 0),
                       'letters_selecting_all_of_C': sum(1 for r in per_letter if r['moved_C'] == len(C)),
                       'net_moved': len(net), 'net_moved_is_T': net == T,
                       'digest_610': st.digest()}
    log(q2=res['Q2_and_C'])
    # 2. landing of T after W2 and holes after V2
    Rv, _ = O.start(str(OUT / 'state-after-V.json'))
    hv = holes(Rv.st).tolist()
    Rw, _ = O.start(str(OUT / 'state-after-Pinv.json'))
    lt = sorted(chambers(Rw.st, T).tolist())
    res['landing'] = {'T_chambers_after_W2_equal_badW': lt == badW, 'T_chambers_equal_ghost_landing': lt == landing,
                      'holes_after_V2': len(hv), 'badW_inside_holes_after_V2': set(badW) <= set(hv),
                      'holes_after_V2_not_in_badW': sorted(set(hv) - set(badW)),
                      'badW_occupied_at_E1': int(occ1[badW].sum()), 'badW_size': len(badW)}
    log(landing=res['landing'])
    # 3. types of the E2 holes
    doc2 = C0 / 'E2-table.json'
    R2, _ = O.start(str(doc2))
    h2 = holes(R2.st).tolist()
    reg = CTX.regions
    res['E2_hole_types'] = {'holes': h2, 'hole_orbits': sorted(reg.orbit_of[h2].tolist()),
                            'bp_orbits': sorted(reg.orbit_of[BP].tolist()),
                            'equal_multisets': sorted(reg.orbit_of[h2].tolist()) == sorted(reg.orbit_of[BP].tolist())}
    log(types=res['E2_hole_types'])
    # 4. X
    RX, _ = O.start(str(PKG / 'input/X-table.json'))
    x = RX.st
    KX = kposes(x)
    offX = np.flatnonzero(KX < 0)
    K2 = kposes(R2.st)
    changed = []
    for p in range(len(KX)):
        a_off, b_off = KX[p] < 0, K2[p] < 0
        if not a_off and not b_off:
            if KX[p] != K2[p]:
                changed.append(p)
        elif a_off and b_off:
            if to_tuple(x.pose(p)) != to_tuple(R2.st.pose(p)):
                changed.append(p)
        else:
            changed.append(p)
    blk17 = sorted(set(offX.tolist()) - set(T) - set(BP))
    res['X'] = {'digest': x.digest(), 'off': int(len(offX)), 'T_subset_of_X_off': set(T) <= set(offX.tolist()),
                'bp_off_at_X': all(KX[p] < 0 for p in BP),
                'bp_pose_X_equals_E1': all(to_tuple(x.pose(p)) == to_tuple(e1.pose(p)) for p in BP),
                'block17_size': len(blk17), 'changed_X_E2': len(changed),
                'changed_is_T_plus_17': changed == sorted(set(T) | set(blk17)),
                'block17_same_exact_pose_E1_E2': all(K1[p] >= 0 and K1[p] == K2[p] for p in blk17)}
    log(X=res['X'])
    (OUT / 'extras.json').write_text(json.dumps(res, indent=1) + '\n')


if __name__ == '__main__':
    main()
