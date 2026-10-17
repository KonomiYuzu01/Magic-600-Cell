# E-B3, independent replay: from the s61c pose table, replay the whole word of the template (V0 D from eb3_prep.py,
# then P_S V1 W_S V1^-1 V2 W_R V2^-1 from eb3_finish.py) on a fresh simulator, every twist checked admissible, the
# digests checked at both ends of each part; at the end no piece may be off the lattice. Also compares the length
# with the 4,402-letter history certificate of M1-05 and reports the parts.
# Usage (from the repository root):
#   python3 -I -B eb3_replay.py research/jumbling/theory <s61c pose table json> <prep word json> <finish word json>
#       <out json>
import json
import sys
import time
from pathlib import Path

import numpy as np

sys.dont_write_bytecode = True
TH = Path(sys.argv[1]).resolve()
sys.path[:0] = [str(TH), str(TH.parent), str(TH.parent / 'fixtures')]
import buffer as B  # noqa: E402

O = B.O
S61C = '218f823f3b269c2c43146086601f9110274c2ccd98363276d0ea09466b30e669'
T0 = time.time()


def main():
    table, wprep, wfin, outp = sys.argv[2], sys.argv[3], sys.argv[4], Path(sys.argv[5])
    R, M = O.start(table)
    st = R.st
    assert st.digest() == S61C, 'not the s61c state'
    off0 = B.off_pieces(st)
    res = {'start_digest': st.digest(), 'off_start': int(len(off0)), 'parts': []}
    total = 0
    for name in (wprep, wfin):
        doc = json.loads(Path(name).read_text())
        assert st.digest() == doc['input_digest'], f'{name}: not at its input state'
        for c, i in doc['word']:
            R.apply(int(c), int(i))            # raises when a twist is not admissible
        assert st.digest() == doc['output_digest'], f'{name}: not at its output state'
        total += len(doc['word'])
        res['parts'].append({'file': Path(name).name, 'letters': len(doc['word']), 'digest_after': st.digest(),
                             'off_after': int(len(B.off_pieces(st)))})
    off = B.off_pieces(st)
    res.update({'letters': total, 'off_end': int(len(off)), 'end_digest': st.digest(),
                'history_certificate_letters': 4402, 'seconds': round(time.time() - T0, 1)})
    outp.write_text(json.dumps(res, indent=1) + '\n')
    print(json.dumps(res))
    assert len(off) == 0, 'pieces are still off the lattice'


if __name__ == '__main__':
    main()
