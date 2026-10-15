"""report-vs-data audit: which of the 3,091 lattice pieces (exact pose changed E2 -> E3) were off K at some prefix.

Usage: python3 -I offunion.py <theory dir> <v26 dir> <E2 table> <parts dir> <chain-check json> <out json>
"""
import json
import sys
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
sys.path.insert(0, str(Path(sys.argv[2]).resolve()))
import ghostlib as GL  # noqa: E402


def main():
    a = sys.argv[1:]
    C = GL.Ctx(a[2])
    R, st = C.R, C.st
    word = []
    for n in ['V', 'P', 'Q', 'Pinv', 'Vinv']:
        word += [tuple(x) for x in json.loads((Path(a[3]) / f'{n}.json').read_text())['word']]
    lat0 = st._kpose()[st.pose_id] >= 0
    ever = np.zeros(len(lat0), bool)
    for c, i in word:
        R.apply(c, i)
        ever |= lat0 & (st._kpose()[st.pose_id] < 0)
    ids = np.array(json.loads(Path(a[4]).read_text())['E2_lattice_pieces_with_changed_exact_pose_ids'], np.int64)
    out = {'E2_lattice_ever_off': int(ever.sum()), 'changed_3091': int(len(ids)),
           'changed_and_ever_off': int(ever[ids].sum()), 'ever_off_not_changed': int((ever & ~np.isin(np.arange(len(ever)), ids)).sum())}
    Path(a[5]).write_text(json.dumps(out) + '\n')
    print(json.dumps(out))


if __name__ == '__main__':
    main()
