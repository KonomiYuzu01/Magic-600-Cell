"""report-vs-data audit: replay the type-S word U = P4 Q0 F P4^-1 from E2 and check the reported end numbers;
check that Q0's grips have caps avoiding T = Bad(P4^-1).

Usage: python3 -I typeS_audit.py <theory dir> <v26 dir> <E2 table> <typeS dir> <out json>
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
import ghostlib as GL  # noqa: E402
from sim.kplus import to_tuple  # noqa: E402

T0 = time.time()


def main():
    a = sys.argv[1:]
    C = GL.Ctx(a[2])
    R, M, st = C.R, C.M, C.st
    d = Path(a[3])
    parts = {n: [tuple(x) for x in json.loads((d / 'parts' / f'{n}.json').read_text())['word']]
             for n in ['P', 'Q0', 'F', 'Pinv']}
    U = parts['P'] + parts['Q0'] + parts['F'] + parts['Pinv']
    T, _, _ = C.bad(parts['Pinv'])
    out = {'length': len(U), 'T_size': int(len(T))}
    out['Q0_caps_avoid_T'] = {str(c): bool(not GL.cap(c)[T].any()) for c, _ in parts['Q0']}
    gkey = {}

    def keys():
        pid = st.pose_id
        arr = np.zeros(int(pid.max()) + 1, np.int64)
        for q in np.unique(pid).tolist():
            arr[q] = gkey.setdefault(to_tuple(st._pose_matrix(q)), len(gkey))
        return arr[pid]
    k0 = keys()
    K0 = st._kpose()[st.pose_id]
    off0 = K0 < 0
    lat0 = ~off0
    d0 = st.digest()
    for c, i in U:
        R.apply(c, i)
    K = st._kpose()[st.pose_id]
    off = K < 0
    k = keys()
    out.update({'end_digest': st.digest(), 'off_start': int(off0.sum()), 'off_end': int(off.sum()),
                'bprime_restored': int((off0 & ~off).sum()), 'collateral': int((lat0 & off).sum()),
                'lattice_on_K_at_end': int((lat0 & ~off).sum()), 'lattice_same_exact_pose': int((lat0 & (k == k0)).sum())})
    for _ in range(len(U)):
        R.undo()
    out['undo_ok'] = st.digest() == d0
    out['sec'] = round(time.time() - T0, 1)
    Path(a[4]).write_text(json.dumps(out) + '\n')
    print(json.dumps(out))


if __name__ == '__main__':
    main()
