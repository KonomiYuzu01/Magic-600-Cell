"""Extra: which of the 3,091 lattice pieces of E2 that change exact pose were off at some prefix of W3."""
import json
import sys
from pathlib import Path
sys.dont_write_bytecode = True
AUD = Path('<work>/v26/B4/audit/replay')
sys.path.insert(0, str(AUD))
import numpy as np  # noqa: E402
import replay_audit as RA  # noqa: E402

R, M = RA.O.start(str(RA.E2_TABLE))
st = R.st
lat0 = st._kpose()[st.pose_id] >= 0
p0 = st.pose_id.copy()
keys = RA.Keys()
c0 = keys.codes(st)
ever_off = np.zeros(len(lat0), bool)
for n in ('V', 'P', 'Q', 'Pinv', 'Vinv'):
    for c, i in json.loads((RA.B4 / 'c3/parts' / f'{n}.json').read_text())['word']:
        R.apply(c, i)
        ever_off |= st._kpose()[st.pose_id] < 0
c3 = keys.codes(st)
moved = lat0 & (c3 != c0)
res = {'moved_lattice': int(moved.sum()), 'moved_and_ever_off': int((moved & ever_off).sum()),
       'lattice_ever_off': int((lat0 & ever_off).sum()), 'lattice_ever_off_back_at_exact_pose': int((lat0 & ever_off & (c3 == c0)).sum())}
(AUD / 'offunion-extra.json').write_text(json.dumps(res) + '\n')
print(json.dumps(res))
