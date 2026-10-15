"""report-vs-data audit: off-geometry fingerprint at Y_4 = E2.P4 and at E2.V3.P4; is F SAFE-ALL at E2.V3.P4?

Usage: python3 -I yfp.py <theory dir> <E2 table> <parts dir> <out json>
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path.insert(0, str(TH))
sys.path.insert(0, str(TH.parent))
import blocks as BL  # noqa: E402
import buffer as B  # noqa: E402


def main():
    a = sys.argv[1:]
    R, M = BL.O.start(a[1])
    st = R.st
    rd = lambda n: [tuple(x) for x in json.loads((Path(a[2]) / f'{n}.json').read_text())['word']]
    V, P = rd('V'), rd('P')
    for c, i in P:
        R.apply(c, i)
    fpY = B.off_fingerprint(st)
    for _ in P:
        R.undo()
    for c, i in V + P:
        R.apply(c, i)
    fpVP = B.off_fingerprint(st)
    cl = st.classify(162, need_inside=True, stop_at_straddle=False)
    lat = st.lattice_flags()
    import numpy as np
    off_inside = int((~lat[np.asarray(cl.inside, np.int64)]).sum())
    out = {'fp_Y4': fpY, 'fp_E2V3P4': fpVP, 'equal': fpY == fpVP,
           'grip162_straddled_at_E2V3P4': cl.straddle is not None, 'grip162_off_inside_at_E2V3P4': off_inside}
    Path(a[3]).write_text(json.dumps(out) + '\n')
    print(json.dumps(out))


if __name__ == '__main__':
    main()
