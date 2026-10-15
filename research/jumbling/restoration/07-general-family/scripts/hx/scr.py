"""Export the pose table of a random I_a scramble: k admissible jumble twists at uniformly random grips
(restore.scramble, inadmissible draws skipped), and its history.

usage: scr.py <research/jumbling> <E-chain scripts> <k> <seed> <out table> <out history json>
"""
import json
import sys
from pathlib import Path

sys.dont_write_bytecode = True
J, SC, K, SEED, OUT, HOUT = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]), sys.argv[5], sys.argv[6]
for p in (J + '/theory', J, J + '/fixtures', SC):
    sys.path.insert(0, p)
import buffer as BU  # noqa: E402
import restore  # noqa: E402
import wj  # noqa: E402

fixture, M = wj.load(restore.ctx, 'I_a')
st, hist = restore.scramble(M, K, SEED)
BU.export_pose_table(st, OUT)
Path(HOUT).write_text(json.dumps({'menu': 'I_a', 'k': K, 'seed': SEED, 'history': hist, 'digest': st.digest()}) + '\n')
print(st.digest(), int((~st.is_lattice_mask()).sum()) if hasattr(st, 'is_lattice_mask') else '')
