"""Per-prefix record of native word files replayed from a pose table (every twist checked admissible).

For the empty prefix and after every twist: the digest, the off count, and against the start (exact matrices, as in
replay_check.endpoint): restored pieces, collateral (lattice pieces of the start now off), still-off pieces whose exact
pose changed, and the depth test of replay_check (every off domain's hub has one member).

Usage: python -I prefixrec.py <theory dir> <pose table> <out jsonl> <word json> ...
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
sys.path.insert(0, str(TH.parent / 'fixtures'))
import replay_check as RC  # noqa: E402

O, CL = RC.O, RC.CL
T0 = time.time()


def main():
    a = sys.argv[1:]
    R, M = O.start(a[1])
    st = R.st
    off0 = st._kpose()[st.pose_id] < 0
    off_pieces = np.flatnonzero(off0).tolist()
    mats = {int(pid): RC.matrix_json(st._pose_matrix(int(pid))) for pid in np.unique(st.pose_id[off_pieces]).tolist()}
    poses0 = {p: mats[int(st.pose_id[p])] for p in off_pieces}
    members, cls = CL.classes()
    hubs = {}
    n = 0
    with open(a[2], 'w') as fh:
        def record(name, index, letter):
            e = RC.endpoint(st, off0, poses0)
            bad = RC.depth_failure(R, st._kpose()[st.pose_id] < 0, members, cls, hubs)
            fh.write(json.dumps({'prefix': n, 'file': name, 'index_in_file': index, 'letter': letter,
                                 'digest': e['end_digest'], 'off': e['off_end'], 'restored': e['restored'],
                                 'collateral': e['collateral'], 'pose_changed': e['pose_changed'],
                                 'depth_ok': not bad, 'depth_failed_domains': bad}) + '\n')
        record(None, None, None)
        for name in a[3:]:
            doc = json.loads(Path(name).read_text())
            if doc.get('input_digest') not in (None, st.digest()):
                raise AssertionError(f'{name}: input digest mismatch')
            for index, (c, i) in enumerate(doc['word']):
                R.apply(int(c), int(i))
                n += 1
                record(Path(name).stem, index, [int(c), int(i)])
            if doc.get('output_digest') not in (None, st.digest()):
                raise AssertionError(f'{name}: output digest mismatch')
            print(json.dumps({'file': name, 'prefix': n, 'sec': round(time.time() - T0)}), file=sys.stderr, flush=True)


if __name__ == '__main__':
    main()
