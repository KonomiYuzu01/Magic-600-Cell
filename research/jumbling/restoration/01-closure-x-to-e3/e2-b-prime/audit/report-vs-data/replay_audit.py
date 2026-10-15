"""report-vs-data audit: my own replay of the 79-letter word from E2 with stage-end and per-prefix bookkeeping,
plus Bad(W), Bad(U) and Bad(P4^-1) by two ghost engines (ghostlib and conjprep).

Usage: python3 -I letters... replay_audit.py <theory dir> <v26 dir> <E2 table> <parts dir> <out json>
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
import buffer as B  # noqa: E402
import conjprep as CP  # noqa: E402
import ghostlib as GL  # noqa: E402
import rim  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()


def main():
    a = sys.argv[1:]
    C = GL.Ctx(a[2])
    R, M, st = C.R, C.M, C.st
    NP = len(st.pose_id)
    parts_dir = Path(a[3])
    names = ['V', 'P', 'Q', 'Pinv', 'Vinv']
    parts = {n: [tuple(x) for x in json.loads((parts_dir / f'{n}.json').read_text())['word']] for n in names}
    W = parts['P'] + parts['Q'] + parts['Pinv']
    U = parts['V'] + W + parts['Vinv']
    out = {'lengths': {n: len(parts[n]) for n in names}, 'U_length': len(U)}
    badW, _, _ = C.bad(W)
    badU, _, _ = C.bad(U)
    badPinv, _, _ = C.bad(parts['Pinv'])
    out['badW'] = sorted(badW.tolist())
    out['badU'] = sorted(badU.tolist())
    out['badPinv'] = sorted(badPinv.tolist())
    b2, _ = CP.bad_chambers(M, W, NP)
    out['badW_xcheck_equal'] = sorted(np.flatnonzero(b2).tolist()) == out['badW']
    b2, _ = CP.bad_chambers(M, U, NP)
    out['badU_xcheck_equal'] = sorted(np.flatnonzero(b2).tolist()) == out['badU']
    b2, _ = CP.bad_chambers(M, parts['Pinv'], NP)
    out['badPinv_xcheck_equal'] = sorted(np.flatnonzero(b2).tolist()) == out['badPinv']
    holes0 = sorted(int(h) for h in B.hole_set(st))
    out['E2_holes'] = holes0
    out['badU_equals_E2_holes'] = out['badU'] == holes0
    out['badW_occupied_at_E2'] = int(len(set(out['badW']) - set(holes0)))

    gkey = {}

    def keys():
        pid = st.pose_id
        u = np.unique(pid)
        lut = {}
        for q in u.tolist():
            lut[q] = gkey.setdefault(to_tuple(st._pose_matrix(q)), len(gkey))
        arr = np.zeros(int(pid.max()) + 1, np.int64)
        for q, v in lut.items():
            arr[q] = v
        return arr[pid]
    key0 = keys()
    K0 = st._kpose()[st.pose_id]
    off0 = K0 < 0
    bp = np.flatnonzero(off0)
    lat0 = ~off0
    d0 = st.digest()
    out['start_digest'] = d0
    prefix = []
    stages = []
    n = 0
    for name in names:
        for c, i in parts[name]:
            R.apply(c, i)
            n += 1
            K = st._kpose()[st.pose_id]
            off = K < 0
            prefix.append({'prefix': n, 'part': name, 'off': int(off.sum()),
                           'collateral': int((lat0 & off).sum()), 'bprime_on': int((~off[bp]).sum())})
        K = st._kpose()[st.pose_id]
        off = K < 0
        k = keys()
        D = rim.Domains(st)
        rec = {'stage_end': name, 'twists': len(parts[name]), 'digest': st.digest(), 'off': int(off.sum()),
               'bprime_on': int((~off[bp]).sum()), 'collateral': int((lat0 & off).sum()),
               'collateral_ids': np.flatnonzero(lat0 & off).tolist(),
               'E2_lattice_same_exact_pose': int((lat0 & (k == key0)).sum()),
               'bprime_exact_pose_unchanged': bool((k[bp] == key0[bp]).all()),
               'bprime_chambers': sorted(D.cham[bp].tolist()) if (D.dom[bp] == 0).all() else None}
        if name == 'V':
            hs = set(int(h) for h in B.hole_set(st))
            rec['badW_vacant_after_V'] = all(x in hs for x in out['badW'])
            rec['off_set_unchanged'] = bool(np.array_equal(off, off0))
        if name == 'Q':
            rec['bprime_in_badPinv'] = int(np.isin(D.cham[bp], badPinv).sum()) if (D.dom[bp] == 0).all() else None
        stages.append(rec)
    out['stages'] = stages
    out['end_digest'] = st.digest()
    coll = [p for p in prefix if p['collateral'] > 0]
    out['prefixes_with_E2_lattice_off'] = len(coll)
    out['first_last'] = [coll[0]['prefix'], coll[-1]['prefix']] if coll else None
    out['max_collateral'] = max(p['collateral'] for p in prefix)
    out['parts_with_collateral'] = sorted({p['part'] for p in coll})
    out['prefix_rows'] = prefix
    # compare with prefixes.jsonl
    pj = [json.loads(l) for l in open(parts_dir.parent / 'prefixes.jsonl')]
    out['prefixes_jsonl_match'] = all(pj[r['prefix']]['off'] == r['off'] and pj[r['prefix']]['collateral'] == r['collateral']
                                      for r in prefix)
    for _ in range(len(U)):
        R.undo()
    out['undo_back_to_E2'] = st.digest() == d0
    out['sec'] = round(time.time() - T0, 1)
    Path(a[4]).write_text(json.dumps(out) + '\n')
    print(json.dumps({x: y for x, y in out.items() if x not in ('prefix_rows',)})[:6000])


if __name__ == '__main__':
    main()
